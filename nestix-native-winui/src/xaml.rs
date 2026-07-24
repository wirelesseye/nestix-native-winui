use std::{
    cell::RefCell,
    collections::HashMap,
    rc::Rc,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
};

use nestix::Shared;
use nestix_native_core::{FontStyle, Rect, ResolvedFontProps, TitleBarMode};
use windows::Storage::Streams::{
    DataWriter, IRandomAccessStream as NativeRandomAccessStream, InMemoryRandomAccessStream,
};
use windows_core::{Error, EventRevoker, HRESULT, HSTRING, Interface, Result};

use crate::{
    bindings::{
        Microsoft::UI::Windowing::OverlappedPresenter,
        Microsoft::UI::Xaml::{
            Controls::Primitives::RangeBase,
            Controls::{
                Button, Canvas, CheckBox, ComboBox, ComboBoxItem, Control, Grid, Image,
                ItemsControl, MenuBar, RadioButton, RowDefinition, ScrollView,
                ScrollingContentOrientation, ScrollingScrollBarVisibility, SelectorBar,
                SelectorBarItem, Slider, TextBlock, TextBox, ToggleSwitch,
            },
            FrameworkElement, GridLength, GridUnitType, HorizontalAlignment,
            Media::{FontFamily, Imaging::BitmapImage, Stretch},
            UIElement, VerticalAlignment, Visibility, Window,
        },
        Windows::Foundation::Size,
        Windows::Graphics::SizeInt32,
        Windows::UI::Color as UiColor,
        Windows::UI::Text::{FontStyle as UiFontStyle, FontWeight as UiFontWeight},
    },
    xaml_app::is_xaml_running,
    xaml_events::{
        RegisteredBoolCallback, RegisteredClickCallback, RegisteredContentSizeCallback,
        RegisteredF64Callback, RegisteredResizeCallback, RegisteredScaleFactorCallback,
        RegisteredStringCallback, RegisteredTabSelectionCallback, RegisteredTextChangedCallback,
    },
};

const E_NOTIMPL: HRESULT = HRESULT(0x80004001u32 as i32);
const E_FAIL: HRESULT = HRESULT(0x80004005u32 as i32);
const BUTTON_INTRINSIC_SLACK: f32 = 2.0;
// The default WinUI templates do not expose these literal template
// dimensions as resources or control properties.
const RADIO_BUTTON_GLYPH_COLUMN_WIDTH: f32 = 20.0;
const RADIO_BUTTON_INTRINSIC_HEIGHT: f32 = 32.0;
const TOGGLE_SWITCH_TRACK_HEIGHT: f32 = 20.0;

pub(crate) struct XamlNode {
    kind: RefCell<XamlKind>,
    children: RefCell<Vec<XamlElement>>,
    layout: RefCell<Option<XamlLayout>>,
    measure_callback: RefCell<Option<Shared<dyn Fn(f32, f32)>>>,
    context_menu: RefCell<Option<Rc<crate::menu::MenuData>>>,
    realized_callbacks: RefCell<HashMap<u64, Shared<dyn Fn(UIElement)>>>,
}

static NEXT_REALIZED_CALLBACK_ID: std::sync::atomic::AtomicU64 =
    std::sync::atomic::AtomicU64::new(1);

pub(crate) struct XamlRealizedRegistration {
    node: std::rc::Weak<XamlNode>,
    id: u64,
}

impl Drop for XamlRealizedRegistration {
    fn drop(&mut self) {
        if let Some(node) = self.node.upgrade() {
            node.realized_callbacks.borrow_mut().remove(&self.id);
        }
    }
}

#[derive(Debug, Clone)]
enum XamlKind {
    Window(WindowState),
    Canvas(CanvasState),
    ScrollView(ScrollViewState),
    Button(ButtonState),
    CheckBox(CheckBoxState),
    RadioButton(RadioButtonState),
    Select(SelectState),
    Switch(SwitchState),
    Slider(SliderState),
    TextBlock(TextBlockState),
    TextBox(TextBoxState),
    Image(ImageState),
    MenuBar(MenuBarState),
    TabView(TabViewState),
    TabViewItem(TabViewItemState),
}

#[derive(Debug, Clone)]
struct WindowState {
    title: String,
    title_bar_mode: TitleBarMode,
    width: i32,
    height: i32,
    realized: Option<Window>,
    scale_factor_callback: Rc<RefCell<Option<RegisteredScaleFactorCallback>>>,
    scale_factor_handler: Rc<RefCell<Option<ScaleFactorHandlerState>>>,
    resize_callback: Rc<RefCell<Option<RegisteredResizeCallback>>>,
    resize_handler: Rc<RefCell<Option<ResizeHandlerState>>>,
    close_requested_callback: Rc<RefCell<Option<RegisteredClickCallback>>>,
    close_requested_handler: Rc<RefCell<Option<CloseRequestedHandlerState>>>,
}

#[derive(Debug, Clone)]
struct CanvasState {
    background_color: Option<nestix_native_core::Color>,
    realized: Option<Canvas>,
}

#[derive(Debug, Clone)]
struct ScrollViewState {
    scroll_x: bool,
    scroll_y: bool,
    realized: Option<ScrollView>,
}

#[derive(Debug, Clone)]
struct ButtonState {
    title: String,
    font: ResolvedFontProps,
    padding: Option<Rect<f64>>,
    enabled: bool,
    on_click: Option<nestix::Shared<dyn Fn()>>,
    realized: Option<RealizedButton>,
    click_handler: Rc<RefCell<Option<ClickHandlerState>>>,
}

#[derive(Debug, Clone)]
struct CheckBoxState {
    title: String,
    font: ResolvedFontProps,
    enabled: bool,
    checked: bool,
    on_change: Option<Shared<dyn Fn(bool)>>,
    realized: Option<(CheckBox, TextBlock)>,
    handler: Rc<RefCell<Option<ValueHandlerState<RegisteredBoolCallback>>>>,
    updating: Arc<AtomicBool>,
}

#[derive(Debug, Clone)]
struct RadioButtonState {
    title: String,
    font: ResolvedFontProps,
    enabled: bool,
    group: String,
    selected: bool,
    on_select: Option<Shared<dyn Fn()>>,
    realized: Option<(RadioButton, TextBlock)>,
    handler: Rc<RefCell<Option<ClickHandlerState>>>,
    updating: Arc<AtomicBool>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SelectOptionData {
    pub label: String,
    pub value: String,
    pub enabled: bool,
}

#[derive(Debug, Clone)]
struct SelectState {
    enabled: bool,
    value: Option<String>,
    options: Arc<Mutex<Vec<(u64, SelectOptionData)>>>,
    on_change: Option<Shared<dyn Fn(String)>>,
    realized: Option<ComboBox>,
    handler: Rc<RefCell<Option<ValueHandlerState<RegisteredStringCallback>>>>,
    updating: Arc<AtomicBool>,
}

#[derive(Debug, Clone)]
struct SwitchState {
    enabled: bool,
    checked: bool,
    on_change: Option<Shared<dyn Fn(bool)>>,
    realized: Option<ToggleSwitch>,
    handler: Rc<RefCell<Option<ValueHandlerState<RegisteredBoolCallback>>>>,
    updating: Arc<AtomicBool>,
}

#[derive(Debug, Clone)]
struct SliderState {
    enabled: bool,
    minimum: f64,
    maximum: f64,
    value: f64,
    on_change: Option<Shared<dyn Fn(f64)>>,
    realized: Option<Slider>,
    handler: Rc<RefCell<Option<ValueHandlerState<RegisteredF64Callback>>>>,
    updating: Arc<AtomicBool>,
}

struct ValueHandlerState<T> {
    _callback: T,
    _revoker: EventRevoker,
}

impl<T> std::fmt::Debug for ValueHandlerState<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ValueHandlerState").finish_non_exhaustive()
    }
}

#[derive(Debug, Clone)]
struct TextBlockState {
    text: String,
    font: ResolvedFontProps,
    realized: Option<TextBlock>,
}

#[derive(Debug, Clone)]
struct TextBoxState {
    text: String,
    on_text_change: Option<Shared<dyn Fn(String)>>,
    realized: Option<TextBox>,
    text_changed_handler: Rc<RefCell<Option<TextChangedHandlerState>>>,
}

#[derive(Debug, Clone)]
struct ImageState {
    source: Option<nestix_native_core::ImageSource>,
    content_fit: nestix_native_core::ContentFit,
    realized: Option<Image>,
    stream: Option<InMemoryRandomAccessStream>,
    opened_callback: Option<Shared<dyn Fn(f32, f32)>>,
    opened_handler: Rc<RefCell<Option<ImageOpenedHandlerState>>>,
}

#[derive(Clone)]
struct MenuBarState {
    menu: Option<Rc<crate::menu::MenuData>>,
    realized: Option<MenuBar>,
}

impl std::fmt::Debug for MenuBarState {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("MenuBarState")
            .field("has_menu", &self.menu.is_some())
            .field("realized", &self.realized)
            .finish()
    }
}

struct ImageOpenedHandlerState {
    _callback: RegisteredContentSizeCallback,
    _revoker: EventRevoker,
}

impl std::fmt::Debug for ImageOpenedHandlerState {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ImageOpenedHandlerState")
            .finish_non_exhaustive()
    }
}

#[derive(Clone)]
struct TabViewState {
    realized: Option<RealizedTabView>,
    selected_changed: Rc<RefCell<Option<Shared<dyn Fn(String)>>>>,
    content_resized: Rc<RefCell<Option<Shared<dyn Fn(f32, f32)>>>>,
    selection_handler: Rc<RefCell<Option<TabSelectionHandlerState>>>,
    content_resize_handler: Rc<RefCell<Option<TabContentResizeHandlerState>>>,
}

impl std::fmt::Debug for TabViewState {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("TabViewElement")
            .field("realized", &self.realized)
            .finish_non_exhaustive()
    }
}

#[derive(Debug, Clone)]
struct TabViewItemState {
    id: String,
    title: String,
    realized: Option<RealizedTabViewItem>,
}

#[derive(Debug, Clone)]
pub(crate) struct RealizedTabView {
    control: Grid,
    selector_bar: SelectorBar,
    content: Grid,
}

#[derive(Debug, Clone)]
pub(crate) struct RealizedTabViewItem {
    selector_item: SelectorBarItem,
    content: Grid,
}

#[derive(Debug, Clone)]
pub(crate) struct RealizedButton {
    control: Button,
    label: TextBlock,
}

#[derive(Debug, Clone, Copy)]
struct XamlLayout {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
}

impl std::fmt::Debug for XamlNode {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("XamlNode")
            .field("kind", &self.kind)
            .field("children", &self.children)
            .field("layout", &self.layout)
            .finish_non_exhaustive()
    }
}

pub(crate) struct ClickHandlerState {
    callback: RegisteredClickCallback,
    _revoker: EventRevoker,
}

pub(crate) struct ScaleFactorHandlerState {
    callback_id: u64,
    _revoker: EventRevoker,
}

pub(crate) struct ResizeHandlerState {
    callback_id: u64,
    _revoker: EventRevoker,
}

pub(crate) struct CloseRequestedHandlerState {
    callback_id: u64,
    _revoker: EventRevoker,
}

pub(crate) struct TextChangedHandlerState {
    callback: RegisteredTextChangedCallback,
    _revoker: EventRevoker,
}

impl std::fmt::Debug for ClickHandlerState {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ClickHandlerState")
            .field("callback_id", &self.callback.id())
            .finish_non_exhaustive()
    }
}

impl std::fmt::Debug for ScaleFactorHandlerState {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ScaleFactorHandlerState")
            .field("callback_id", &self.callback_id)
            .finish_non_exhaustive()
    }
}

impl std::fmt::Debug for ResizeHandlerState {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ResizeHandlerState")
            .field("callback_id", &self.callback_id)
            .finish_non_exhaustive()
    }
}

impl std::fmt::Debug for CloseRequestedHandlerState {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("CloseRequestedHandlerState")
            .field("callback_id", &self.callback_id)
            .finish_non_exhaustive()
    }
}

impl std::fmt::Debug for TextChangedHandlerState {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("TextChangedHandlerState")
            .field("callback_id", &self.callback.id())
            .finish_non_exhaustive()
    }
}

#[derive(Debug, Clone)]
pub(crate) struct XamlElement(Rc<XamlNode>);

impl PartialEq for XamlElement {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

impl Eq for XamlElement {}

impl XamlElement {
    pub(crate) fn on_realized(
        &self,
        callback: Shared<dyn Fn(UIElement)>,
    ) -> Result<XamlRealizedRegistration> {
        let id = NEXT_REALIZED_CALLBACK_ID.fetch_add(1, Ordering::Relaxed);
        self.0
            .realized_callbacks
            .borrow_mut()
            .insert(id, callback.clone());
        if self.is_realized() {
            callback(self.as_ui_element()?);
        }
        Ok(XamlRealizedRegistration {
            node: Rc::downgrade(&self.0),
            id,
        })
    }
}

/// Defines a typed façade over the erased element identity used by Nestix handles and
/// parent/child contexts. Component code keeps the façade, so control-specific APIs
/// cannot accidentally be invoked for the wrong XAML control.
macro_rules! typed_element {
    ($name:ident) => {
        #[derive(Debug, Clone, PartialEq, Eq)]
        pub(crate) struct $name(XamlElement);

        impl $name {
            pub(crate) fn erased(&self) -> XamlElement {
                self.0.clone()
            }
        }

        impl std::ops::Deref for $name {
            type Target = XamlElement;

            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }
    };
}

typed_element!(WindowElement);
typed_element!(CanvasElement);
typed_element!(ScrollViewElement);
typed_element!(ButtonElement);
typed_element!(CheckBoxElement);
typed_element!(RadioButtonElement);
typed_element!(SelectElement);
typed_element!(SwitchElement);
typed_element!(SliderElement);
typed_element!(TextBlockElement);
typed_element!(TextBoxElement);
typed_element!(ImageElement);
typed_element!(MenuBarElement);
typed_element!(TabViewElement);
typed_element!(TabViewItemElement);

impl WindowElement {
    pub(crate) fn new(title: String, title_bar_mode: TitleBarMode) -> Result<Self> {
        XamlElement::window(title, title_bar_mode).map(Self)
    }

    pub(crate) fn activate(&self) -> Result<()> {
        self.0.activate()
    }
    pub(crate) fn close(&self) -> Result<()> {
        self.0.close_window()
    }
    pub(crate) fn set_title(&self, title: String) -> Result<()> {
        self.0.set_text(title)
    }
    pub(crate) fn set_title_bar_mode(&self, mode: TitleBarMode) -> Result<()> {
        self.0.set_title_bar_mode(mode)
    }
    pub(crate) fn set_size(&self, width: i32, height: i32) -> Result<()> {
        self.0.set_window_size(width, height)
    }
    pub(crate) fn set_scale_factor_changed(
        &self,
        handler: Option<Shared<dyn Fn(f64)>>,
    ) -> Result<()> {
        self.0.set_scale_factor_changed(handler)
    }
    pub(crate) fn set_resized(
        &self,
        handler: Option<Shared<dyn Fn(nestix_native_core::dpi::Size)>>,
    ) -> Result<()> {
        self.0.set_resized(handler)
    }
    pub(crate) fn set_close_requested(&self, handler: Option<Shared<dyn Fn()>>) -> Result<()> {
        self.0.set_close_requested(handler)
    }
    pub(crate) fn hwnd(&self) -> Result<windows::Win32::Foundation::HWND> {
        self.0.window_hwnd()
    }

    pub(crate) fn window_id(&self) -> Result<crate::bindings::Microsoft::UI::WindowId> {
        let kind = self.0.0.kind.borrow();
        let XamlKind::Window(state) = &*kind else {
            return Err(Error::new(E_NOTIMPL, "element is not a window"));
        };
        let window = state
            .realized
            .as_ref()
            .ok_or_else(|| Error::new(E_FAIL, "WinUI window is not realized"))?;
        window.AppWindow()?.Id()
    }

    pub(crate) fn dispatcher_queue(
        &self,
    ) -> Result<crate::bindings::Microsoft::UI::Dispatching::DispatcherQueue> {
        let kind = self.0.0.kind.borrow();
        let XamlKind::Window(state) = &*kind else {
            return Err(Error::new(E_NOTIMPL, "element is not a window"));
        };
        let window = state
            .realized
            .as_ref()
            .ok_or_else(|| Error::new(E_FAIL, "WinUI window is not realized"))?;
        window.DispatcherQueue()
    }
}

impl CanvasElement {
    pub(crate) fn new() -> Result<Self> {
        XamlElement::canvas().map(Self)
    }
    pub(crate) fn set_background_color(
        &self,
        color: Option<nestix_native_core::Color>,
    ) -> Result<()> {
        self.0.set_background_color(color)
    }
}

impl ScrollViewElement {
    pub(crate) fn new() -> Result<Self> {
        XamlElement::scroll_view().map(Self)
    }
    pub(crate) fn set_scroll_enabled(&self, x: bool, y: bool) -> Result<()> {
        self.0.set_scroll_enabled(x, y)
    }
}

impl ButtonElement {
    pub(crate) fn new(title: String) -> Result<Self> {
        XamlElement::button(title).map(Self)
    }
    pub(crate) fn set_title(&self, title: String) -> Result<()> {
        self.0.set_text(title)
    }
    pub(crate) fn set_enabled(&self, value: bool) -> Result<()> {
        self.0.set_enabled(value)
    }
    pub(crate) fn set_on_click(&self, handler: Option<Shared<dyn Fn()>>) -> Result<()> {
        self.0.set_button_click(handler)
    }
    pub(crate) fn set_font(&self, font: ResolvedFontProps) -> Result<()> {
        self.0.set_font(font)
    }
    pub(crate) fn set_padding(&self, padding: Option<Rect<f64>>) -> Result<()> {
        self.0.set_button_padding(padding)
    }
}

impl CheckBoxElement {
    pub(crate) fn new(title: String) -> Result<Self> {
        Ok(Self(XamlElement::new_checkbox(title)))
    }
    pub(crate) fn set_title(&self, value: String) -> Result<()> {
        self.0.set_text(value)
    }
    pub(crate) fn set_enabled(&self, value: bool) -> Result<()> {
        self.0.set_enabled(value)
    }
    pub(crate) fn set_checked(&self, value: bool) -> Result<()> {
        self.0.set_checked(value)
    }
    pub(crate) fn set_on_checked_change(&self, value: Option<Shared<dyn Fn(bool)>>) -> Result<()> {
        self.0.set_bool_handler(value)
    }
    pub(crate) fn set_font(&self, value: ResolvedFontProps) -> Result<()> {
        self.0.set_font(value)
    }
}

impl RadioButtonElement {
    pub(crate) fn new(title: String) -> Result<Self> {
        Ok(Self(XamlElement::new_radio_button(title)))
    }
    pub(crate) fn set_title(&self, value: String) -> Result<()> {
        self.0.set_text(value)
    }
    pub(crate) fn set_enabled(&self, value: bool) -> Result<()> {
        self.0.set_enabled(value)
    }
    pub(crate) fn set_group(&self, value: String) -> Result<()> {
        self.0.set_radio_group(value)
    }
    pub(crate) fn set_selected(&self, value: bool) -> Result<()> {
        self.0.set_checked(value)
    }
    pub(crate) fn set_on_select(&self, value: Option<Shared<dyn Fn()>>) -> Result<()> {
        self.0.set_click_handler(value)
    }
    pub(crate) fn set_font(&self, value: ResolvedFontProps) -> Result<()> {
        self.0.set_font(value)
    }
}

impl SelectElement {
    pub(crate) fn new() -> Result<Self> {
        Ok(Self(XamlElement::new_select()))
    }
    pub(crate) fn set_enabled(&self, value: bool) -> Result<()> {
        self.0.set_enabled(value)
    }
    pub(crate) fn set_value(&self, value: Option<String>) -> Result<()> {
        self.0.set_select_value(value)
    }
    pub(crate) fn set_on_value_change(&self, value: Option<Shared<dyn Fn(String)>>) -> Result<()> {
        self.0.set_string_handler(value)
    }
    pub(crate) fn upsert_option(&self, id: u64, option: SelectOptionData) -> Result<()> {
        self.0.upsert_select_option(id, option)
    }
    pub(crate) fn remove_option(&self, id: u64) -> Result<()> {
        self.0.remove_select_option(id)
    }
    pub(crate) fn move_option(&self, id: u64, index: usize) -> Result<()> {
        self.0.move_select_option(id, index)
    }
}

impl SwitchElement {
    pub(crate) fn new() -> Result<Self> {
        Ok(Self(XamlElement::new_switch()))
    }
    pub(crate) fn set_enabled(&self, value: bool) -> Result<()> {
        self.0.set_enabled(value)
    }
    pub(crate) fn set_checked(&self, value: bool) -> Result<()> {
        self.0.set_checked(value)
    }
    pub(crate) fn set_on_checked_change(&self, value: Option<Shared<dyn Fn(bool)>>) -> Result<()> {
        self.0.set_bool_handler(value)
    }
}

impl SliderElement {
    pub(crate) fn new() -> Result<Self> {
        Ok(Self(XamlElement::new_slider()))
    }
    pub(crate) fn set_enabled(&self, value: bool) -> Result<()> {
        self.0.set_enabled(value)
    }
    pub(crate) fn set_range(&self, minimum: f64, maximum: f64, value: f64) -> Result<()> {
        self.0.set_slider_range(minimum, maximum, value)
    }
    pub(crate) fn set_on_value_change(&self, value: Option<Shared<dyn Fn(f64)>>) -> Result<()> {
        self.0.set_f64_handler(value)
    }
}

impl TextBlockElement {
    pub(crate) fn new(text: String) -> Result<Self> {
        XamlElement::text_block(text).map(Self)
    }
    pub(crate) fn set_text(&self, text: String) -> Result<()> {
        self.0.set_text(text)
    }
    pub(crate) fn set_font(&self, font: ResolvedFontProps) -> Result<()> {
        self.0.set_font(font)
    }
}

impl TextBoxElement {
    pub(crate) fn new(text: String) -> Result<Self> {
        XamlElement::text_box(text).map(Self)
    }
    pub(crate) fn set_text(&self, text: String) -> Result<()> {
        self.0.set_text(text)
    }
    pub(crate) fn set_on_text_changed(
        &self,
        handler: Option<Shared<dyn Fn(String)>>,
    ) -> Result<()> {
        self.0.set_text_changed(handler)
    }
}

impl ImageElement {
    pub(crate) fn new() -> Result<Self> {
        XamlElement::image().map(Self)
    }
    pub(crate) fn set_source(&self, source: nestix_native_core::ImageSource) -> Result<()> {
        self.0.set_image_source(source)
    }
    pub(crate) fn set_content_fit(&self, fit: nestix_native_core::ContentFit) -> Result<()> {
        self.0.set_image_content_fit(fit)
    }
    pub(crate) fn set_intrinsic_size_changed(
        &self,
        callback: Shared<dyn Fn(f32, f32)>,
    ) -> Result<()> {
        let mut kind = self.0.0.kind.borrow_mut();
        let XamlKind::Image(image) = &mut *kind else {
            return Ok(());
        };
        image.opened_callback = Some(callback);
        Ok(())
    }
}

impl MenuBarElement {
    pub(crate) fn new() -> Result<Self> {
        XamlElement::menu_bar().map(Self)
    }

    pub(crate) fn set_menu(&self, menu: Option<Rc<crate::menu::MenuData>>) -> Result<()> {
        self.0.set_menu_bar_menu(menu)
    }
}

impl TabViewElement {
    pub(crate) fn new() -> Result<Self> {
        XamlElement::tab_view().map(Self)
    }
    pub(crate) fn set_selected(&self, handler: Shared<dyn Fn(String)>) -> Result<()> {
        self.0.set_tab_selected(handler)
    }
    pub(crate) fn set_content_resized(&self, handler: Shared<dyn Fn(f32, f32)>) -> Result<()> {
        self.0.set_tab_content_resized(handler)
    }
}

impl TabViewItemElement {
    pub(crate) fn new(id: String, title: String) -> Result<Self> {
        XamlElement::tab_view_item(id, title).map(Self)
    }
    pub(crate) fn set_id(&self, id: String) -> Result<()> {
        self.0.set_tab_item_id(id)
    }
    pub(crate) fn set_title(&self, title: String) -> Result<()> {
        self.0.set_text(title)
    }
    pub(crate) fn set_visible(&self, visible: bool) -> Result<()> {
        self.0.set_visible(visible)
    }
}

impl XamlElement {
    fn window(title: String, title_bar_mode: TitleBarMode) -> Result<Self> {
        Ok(Self::new(XamlKind::Window(WindowState {
            title,
            title_bar_mode,
            width: 200,
            height: 200,
            realized: None,
            scale_factor_callback: Rc::new(RefCell::new(None)),
            scale_factor_handler: Rc::new(RefCell::new(None)),
            resize_callback: Rc::new(RefCell::new(None)),
            resize_handler: Rc::new(RefCell::new(None)),
            close_requested_callback: Rc::new(RefCell::new(None)),
            close_requested_handler: Rc::new(RefCell::new(None)),
        })))
    }

    fn canvas() -> Result<Self> {
        Ok(Self::new(XamlKind::Canvas(CanvasState {
            background_color: None,
            realized: None,
        })))
    }

    fn scroll_view() -> Result<Self> {
        Ok(Self::new(XamlKind::ScrollView(ScrollViewState {
            scroll_x: false,
            scroll_y: true,
            realized: None,
        })))
    }

    fn button(title: String) -> Result<Self> {
        Ok(Self::new(XamlKind::Button(ButtonState {
            title,
            font: ResolvedFontProps::default(),
            padding: None,
            enabled: true,
            on_click: None,
            realized: None,
            click_handler: Rc::new(RefCell::new(None)),
        })))
    }

    fn new_checkbox(title: String) -> Self {
        Self::new(XamlKind::CheckBox(CheckBoxState {
            title,
            font: ResolvedFontProps::default(),
            enabled: true,
            checked: false,
            on_change: None,
            realized: None,
            handler: Rc::new(RefCell::new(None)),
            updating: Arc::new(AtomicBool::new(false)),
        }))
    }

    fn new_radio_button(title: String) -> Self {
        Self::new(XamlKind::RadioButton(RadioButtonState {
            title,
            font: ResolvedFontProps::default(),
            enabled: true,
            group: String::new(),
            selected: false,
            on_select: None,
            realized: None,
            handler: Rc::new(RefCell::new(None)),
            updating: Arc::new(AtomicBool::new(false)),
        }))
    }

    fn new_select() -> Self {
        Self::new(XamlKind::Select(SelectState {
            enabled: true,
            value: None,
            options: Arc::new(Mutex::new(Vec::new())),
            on_change: None,
            realized: None,
            handler: Rc::new(RefCell::new(None)),
            updating: Arc::new(AtomicBool::new(false)),
        }))
    }

    fn new_switch() -> Self {
        Self::new(XamlKind::Switch(SwitchState {
            enabled: true,
            checked: false,
            on_change: None,
            realized: None,
            handler: Rc::new(RefCell::new(None)),
            updating: Arc::new(AtomicBool::new(false)),
        }))
    }

    fn new_slider() -> Self {
        Self::new(XamlKind::Slider(SliderState {
            enabled: true,
            minimum: 0.0,
            maximum: 100.0,
            value: 0.0,
            on_change: None,
            realized: None,
            handler: Rc::new(RefCell::new(None)),
            updating: Arc::new(AtomicBool::new(false)),
        }))
    }

    fn text_block(text: String) -> Result<Self> {
        Ok(Self::new(XamlKind::TextBlock(TextBlockState {
            text,
            font: ResolvedFontProps::default(),
            realized: None,
        })))
    }

    fn text_box(text: String) -> Result<Self> {
        Ok(Self::new(XamlKind::TextBox(TextBoxState {
            text,
            on_text_change: None,
            realized: None,
            text_changed_handler: Rc::new(RefCell::new(None)),
        })))
    }

    fn image() -> Result<Self> {
        Ok(Self::new(XamlKind::Image(ImageState {
            source: None,
            content_fit: nestix_native_core::ContentFit::Contain,
            realized: None,
            stream: None,
            opened_callback: None,
            opened_handler: Rc::new(RefCell::new(None)),
        })))
    }

    fn menu_bar() -> Result<Self> {
        Ok(Self::new(XamlKind::MenuBar(MenuBarState {
            menu: None,
            realized: None,
        })))
    }

    fn tab_view() -> Result<Self> {
        Ok(Self::new(XamlKind::TabView(TabViewState {
            realized: None,
            selected_changed: Rc::new(RefCell::new(None)),
            content_resized: Rc::new(RefCell::new(None)),
            selection_handler: Rc::new(RefCell::new(None)),
            content_resize_handler: Rc::new(RefCell::new(None)),
        })))
    }

    fn tab_view_item(id: String, title: String) -> Result<Self> {
        Ok(Self::new(XamlKind::TabViewItem(TabViewItemState {
            id,
            title,
            realized: None,
        })))
    }

    pub fn activate(&self) -> Result<()> {
        if !is_xaml_running() {
            return Ok(());
        }
        self.realize()?;
        self.with_window(|window| window.Activate())
    }

    pub fn append_child(&self, child: XamlElement) -> Result<()> {
        let index = self
            .0
            .children
            .borrow()
            .iter()
            .filter(|item| *item != &child)
            .count();
        self.insert_child(child, index)
    }

    pub fn insert_child(&self, child: XamlElement, index: usize) -> Result<()> {
        let index = {
            let mut children = self.0.children.borrow_mut();
            children.retain(|item| item != &child);
            let index = index.min(children.len());
            children.insert(index, child.clone());
            index
        };

        if is_xaml_running() {
            self.realize()?;
            child.realize()?;
            self.insert_realized_child(&child, index)?;
            child.measure_intrinsic_recursive()?;
        }
        Ok(())
    }

    pub fn insert_child_after(
        &self,
        child: XamlElement,
        predecessor: Option<&XamlElement>,
    ) -> Result<()> {
        let previous_index = self.child_index(&child);
        let index = predecessor
            .and_then(|predecessor| self.child_index(predecessor))
            .map(|index| index + 1)
            .unwrap_or_else(|| {
                if predecessor.is_none() {
                    0
                } else {
                    previous_index.unwrap_or(self.0.children.borrow().len())
                }
            });
        self.insert_child(child, index)
    }

    pub fn child_index(&self, child: &XamlElement) -> Option<usize> {
        self.0
            .children
            .borrow()
            .iter()
            .position(|item| item == child)
    }

    pub fn remove_child(&self, child: &XamlElement) -> Result<()> {
        self.0.children.borrow_mut().retain(|item| item != child);

        if !is_xaml_running() || !self.is_realized() {
            return Ok(());
        }

        match &*self.0.kind.borrow() {
            XamlKind::Window(element) => {
                if let Some(window) = &element.realized {
                    window.SetContent(None)?;
                }
            }
            XamlKind::Canvas(element) => {
                if let Some(canvas) = &element.realized {
                    let child = child.as_ui_element()?;
                    let children = canvas.Children()?;
                    let mut index = 0;
                    if children.IndexOf(&child, &mut index)? {
                        children.RemoveAt(index)?;
                    }
                }
            }
            XamlKind::ScrollView(element) => {
                if let Some(scroll_view) = &element.realized {
                    scroll_view.SetContent(None)?;
                }
            }
            XamlKind::TabView(element) => {
                if let Some(realized) = &element.realized {
                    let child_kind = child.0.kind.borrow();
                    if let XamlKind::TabViewItem(item) = &*child_kind
                        && let Some(item) = &item.realized
                    {
                        let items = realized.selector_bar.Items()?;
                        let mut index = 0;
                        if items.IndexOf(&item.selector_item, &mut index)? {
                            items.RemoveAt(index)?;
                        }
                        if realized.selector_bar.SelectedItem().is_err() && items.Size()? > 0 {
                            let first = items.GetAt(0)?;
                            realized.selector_bar.SetSelectedItem(&first)?;
                        }
                        let pages = realized.content.Children()?;
                        let page: UIElement = item.content.cast()?;
                        if pages.IndexOf(&page, &mut index)? {
                            pages.RemoveAt(index)?;
                        }
                    }
                }
            }
            XamlKind::TabViewItem(element) => {
                if let Some(realized) = &element.realized {
                    let child = child.as_ui_element()?;
                    let children = realized.content.Children()?;
                    let mut index = 0;
                    if children.IndexOf(&child, &mut index)? {
                        children.RemoveAt(index)?;
                    }
                }
            }
            XamlKind::Button(_)
            | XamlKind::CheckBox(_)
            | XamlKind::RadioButton(_)
            | XamlKind::Select(_)
            | XamlKind::Switch(_)
            | XamlKind::Slider(_)
            | XamlKind::TextBlock(_)
            | XamlKind::TextBox(_)
            | XamlKind::MenuBar(_)
            | XamlKind::Image(_) => {}
        }
        Ok(())
    }

    pub fn set_text(&self, text: String) -> Result<()> {
        let text_value = HSTRING::from(text.clone());
        let text_box_to_update = {
            match &mut *self.0.kind.borrow_mut() {
                XamlKind::Window(element) => {
                    element.title = text.clone();
                    if let Some(window) = &element.realized {
                        window.SetTitle(&text_value)?;
                    }
                    None
                }
                XamlKind::Button(element) => {
                    element.title = text.clone();
                    if let Some(realized) = &element.realized {
                        realized.label.SetText(&text_value)?;
                    }
                    None
                }
                XamlKind::CheckBox(element) => {
                    element.title = text.clone();
                    if let Some((_, label)) = &element.realized {
                        label.SetText(&text_value)?;
                    }
                    None
                }
                XamlKind::RadioButton(element) => {
                    element.title = text.clone();
                    if let Some((_, label)) = &element.realized {
                        label.SetText(&text_value)?;
                    }
                    None
                }
                XamlKind::TextBlock(element) => {
                    element.text = text.clone();
                    if let Some(block) = &element.realized {
                        block.SetText(&text_value)?;
                    }
                    None
                }
                XamlKind::TextBox(element) => {
                    element.text = text;
                    if let Some(text_box) = &element.realized
                        && text_box.Text()? != text_value
                    {
                        Some(text_box.clone())
                    } else {
                        None
                    }
                }
                XamlKind::TabViewItem(element) => {
                    element.title = text.clone();
                    if let Some(realized) = &element.realized {
                        realized.selector_item.SetText(&text_value)?;
                    }
                    None
                }
                XamlKind::Canvas(_)
                | XamlKind::ScrollView(_)
                | XamlKind::TabView(_)
                | XamlKind::MenuBar(_)
                | XamlKind::Select(_)
                | XamlKind::Switch(_)
                | XamlKind::Slider(_)
                | XamlKind::Image(_) => None,
            }
        };

        if let Some(text_box) = text_box_to_update {
            text_box.SetText(&text_value)?;
        }
        self.measure_intrinsic()?;
        Ok(())
    }

    pub fn set_font(&self, font: ResolvedFontProps) -> Result<()> {
        match &mut *self.0.kind.borrow_mut() {
            XamlKind::Button(element) => {
                element.font = font.clone();
                if let Some(realized) = &element.realized {
                    apply_font(&realized.label, &font)?;
                }
            }
            XamlKind::TextBlock(element) => {
                element.font = font.clone();
                if let Some(realized) = &element.realized {
                    apply_font(realized, &font)?;
                }
            }
            XamlKind::CheckBox(element) => {
                element.font = font.clone();
                if let Some((_, label)) = &element.realized {
                    apply_font(label, &font)?;
                }
            }
            XamlKind::RadioButton(element) => {
                element.font = font.clone();
                if let Some((_, label)) = &element.realized {
                    apply_font(label, &font)?;
                }
            }
            _ => {
                return Err(Error::new(
                    E_NOTIMPL,
                    "element does not support font styling",
                ));
            }
        }
        self.measure_intrinsic_recursive()
    }

    fn set_enabled(&self, enabled: bool) -> Result<()> {
        match &mut *self.0.kind.borrow_mut() {
            XamlKind::Button(s) => {
                s.enabled = enabled;
                if let Some(c) = &s.realized {
                    c.control.SetIsEnabled(enabled)?;
                }
            }
            XamlKind::CheckBox(s) => {
                s.enabled = enabled;
                if let Some((c, _)) = &s.realized {
                    c.SetIsEnabled(enabled)?;
                }
            }
            XamlKind::RadioButton(s) => {
                s.enabled = enabled;
                if let Some((c, _)) = &s.realized {
                    c.SetIsEnabled(enabled)?;
                }
            }
            XamlKind::Select(s) => {
                s.enabled = enabled;
                if let Some(c) = &s.realized {
                    c.SetIsEnabled(enabled)?;
                }
            }
            XamlKind::Switch(s) => {
                s.enabled = enabled;
                if let Some(c) = &s.realized {
                    c.SetIsEnabled(enabled)?;
                }
            }
            XamlKind::Slider(s) => {
                s.enabled = enabled;
                if let Some(c) = &s.realized {
                    c.SetIsEnabled(enabled)?;
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn set_checked(&self, checked: bool) -> Result<()> {
        match &mut *self.0.kind.borrow_mut() {
            XamlKind::CheckBox(s) => {
                s.checked = checked;
                if let Some((c, _)) = &s.realized {
                    s.updating.store(true, Ordering::SeqCst);
                    let result = c.SetIsChecked(Some(checked));
                    s.updating.store(false, Ordering::SeqCst);
                    result?;
                }
            }
            XamlKind::RadioButton(s) => {
                s.selected = checked;
                if let Some((c, _)) = &s.realized {
                    s.updating.store(true, Ordering::SeqCst);
                    let result = c.SetIsChecked(Some(checked));
                    s.updating.store(false, Ordering::SeqCst);
                    result?;
                }
            }
            XamlKind::Switch(s) => {
                s.checked = checked;
                if let Some(c) = &s.realized {
                    s.updating.store(true, Ordering::SeqCst);
                    let result = c.SetIsOn(checked);
                    s.updating.store(false, Ordering::SeqCst);
                    result?;
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn set_radio_group(&self, group: String) -> Result<()> {
        let mut kind = self.0.kind.borrow_mut();
        let XamlKind::RadioButton(s) = &mut *kind else {
            return Ok(());
        };
        s.group = group;
        if let Some((c, _)) = &s.realized {
            c.SetGroupName(&HSTRING::from(&s.group))?;
        }
        Ok(())
    }

    fn set_bool_handler(&self, handler: Option<Shared<dyn Fn(bool)>>) -> Result<()> {
        match &mut *self.0.kind.borrow_mut() {
            XamlKind::CheckBox(s) => {
                s.on_change = handler;
                s.attach_handler()?;
            }
            XamlKind::Switch(s) => {
                s.on_change = handler;
                s.attach_handler()?;
            }
            _ => {}
        }
        Ok(())
    }

    fn set_click_handler(&self, handler: Option<Shared<dyn Fn()>>) -> Result<()> {
        let mut kind = self.0.kind.borrow_mut();
        let XamlKind::RadioButton(s) = &mut *kind else {
            return Ok(());
        };
        s.on_select = handler;
        s.attach_handler()
    }

    fn set_select_value(&self, value: Option<String>) -> Result<()> {
        let mut kind = self.0.kind.borrow_mut();
        let XamlKind::Select(s) = &mut *kind else {
            return Ok(());
        };
        s.value = value;
        s.apply_selection()
    }

    fn set_string_handler(&self, handler: Option<Shared<dyn Fn(String)>>) -> Result<()> {
        let mut kind = self.0.kind.borrow_mut();
        let XamlKind::Select(s) = &mut *kind else {
            return Ok(());
        };
        s.on_change = handler;
        s.attach_handler()
    }

    fn upsert_select_option(&self, id: u64, option: SelectOptionData) -> Result<()> {
        let mut kind = self.0.kind.borrow_mut();
        let XamlKind::Select(s) = &mut *kind else {
            return Ok(());
        };
        let mut options = s.options.lock().unwrap();
        if let Some(current) = options.iter_mut().find(|(current, _)| *current == id) {
            current.1 = option;
        } else {
            options.push((id, option));
        }
        drop(options);
        s.apply_options()?;
        drop(kind);
        self.measure_intrinsic()
    }

    fn remove_select_option(&self, id: u64) -> Result<()> {
        let mut kind = self.0.kind.borrow_mut();
        let XamlKind::Select(s) = &mut *kind else {
            return Ok(());
        };
        s.options
            .lock()
            .unwrap()
            .retain(|(current, _)| *current != id);
        s.apply_options()?;
        drop(kind);
        self.measure_intrinsic()
    }

    fn move_select_option(&self, id: u64, index: usize) -> Result<()> {
        let mut kind = self.0.kind.borrow_mut();
        let XamlKind::Select(s) = &mut *kind else {
            return Ok(());
        };
        let mut options = s.options.lock().unwrap();
        if let Some(current) = options.iter().position(|(current, _)| *current == id) {
            let option = options.remove(current);
            let index = index.min(options.len());
            options.insert(index, option);
        }
        drop(options);
        s.apply_options()?;
        drop(kind);
        self.measure_intrinsic()
    }

    fn set_slider_range(&self, minimum: f64, maximum: f64, value: f64) -> Result<()> {
        let mut kind = self.0.kind.borrow_mut();
        let XamlKind::Slider(s) = &mut *kind else {
            return Ok(());
        };
        s.minimum = minimum;
        s.maximum = maximum;
        s.value = value;
        if let Some(c) = &s.realized {
            s.updating.store(true, Ordering::SeqCst);
            let value = if minimum <= maximum {
                value.clamp(minimum, maximum)
            } else {
                minimum
            };
            let result = (|| {
                c.SetMinimum(minimum)?;
                c.SetMaximum(maximum)?;
                c.SetValue2(value)
            })();
            s.updating.store(false, Ordering::SeqCst);
            result?;
        }
        Ok(())
    }

    fn set_f64_handler(&self, handler: Option<Shared<dyn Fn(f64)>>) -> Result<()> {
        let mut kind = self.0.kind.borrow_mut();
        let XamlKind::Slider(s) = &mut *kind else {
            return Ok(());
        };
        s.on_change = handler;
        s.attach_handler()
    }

    fn set_button_padding(&self, padding: Option<Rect<f64>>) -> Result<()> {
        let mut kind = self.0.kind.borrow_mut();
        let XamlKind::Button(element) = &mut *kind else {
            return Err(Error::new(E_NOTIMPL, "element does not support padding"));
        };
        element.padding = padding;
        if let Some(realized) = &element.realized {
            apply_button_padding(&realized.control, padding)?;
        }
        drop(kind);
        self.measure_intrinsic_recursive()
    }

    fn set_window_size(&self, width: i32, height: i32) -> Result<()> {
        match &mut *self.0.kind.borrow_mut() {
            XamlKind::Window(element) => {
                element.width = width;
                element.height = height;
                if let Some(window) = &element.realized {
                    window.AppWindow()?.ResizeClient(SizeInt32 {
                        Width: width,
                        Height: height,
                    })?;
                }
                Ok(())
            }
            other => panic!("XamlElement is not a window: {:?}", other),
        }
    }

    fn set_title_bar_mode(&self, mode: TitleBarMode) -> Result<()> {
        match &mut *self.0.kind.borrow_mut() {
            XamlKind::Window(element) => {
                element.title_bar_mode = mode;
                if let Some(window) = &element.realized {
                    apply_title_bar_mode(window, mode)?;
                }
                Ok(())
            }
            other => panic!("XamlElement is not a window: {:?}", other),
        }
    }

    fn close_window(&self) -> Result<()> {
        let kind = self.0.kind.borrow();
        let XamlKind::Window(element) = &*kind else {
            return Ok(());
        };
        if let Some(window) = &element.realized {
            window.Close()?;
        }
        Ok(())
    }

    fn set_scale_factor_changed(&self, handler: Option<nestix::Shared<dyn Fn(f64)>>) -> Result<()> {
        let mut kind = self.0.kind.borrow_mut();
        let XamlKind::Window(element) = &mut *kind else {
            return Ok(());
        };

        element.detach_scale_factor_handler();
        element
            .scale_factor_callback
            .replace(handler.map(RegisteredScaleFactorCallback::register));

        if let Some(window) = element.realized.clone() {
            element.attach_scale_factor_handler(&window)?;
        }
        Ok(())
    }

    fn set_resized(
        &self,
        handler: Option<nestix::Shared<dyn Fn(nestix_native_core::dpi::Size)>>,
    ) -> Result<()> {
        let mut kind = self.0.kind.borrow_mut();
        let XamlKind::Window(element) = &mut *kind else {
            return Ok(());
        };

        element.detach_resize_handler();
        element
            .resize_callback
            .replace(handler.map(RegisteredResizeCallback::register));

        if let Some(window) = element.realized.clone() {
            element.attach_resize_handler(&window)?;
        }
        Ok(())
    }

    fn set_close_requested(&self, handler: Option<Shared<dyn Fn()>>) -> Result<()> {
        let mut kind = self.0.kind.borrow_mut();
        let XamlKind::Window(element) = &mut *kind else {
            return Ok(());
        };

        element.detach_close_requested_handler();
        element
            .close_requested_callback
            .replace(handler.map(RegisteredClickCallback::register));

        if let Some(window) = element.realized.clone() {
            element.attach_close_requested_handler(&window)?;
        }
        Ok(())
    }

    fn set_button_click(&self, handler: Option<nestix::Shared<dyn Fn()>>) -> Result<()> {
        let mut kind = self.0.kind.borrow_mut();
        let XamlKind::Button(element) = &mut *kind else {
            return Ok(());
        };

        element.on_click = handler.clone();
        if let Some(realized) = &element.realized {
            element.attach_click_handler(&realized.control, handler)?;
        }
        Ok(())
    }

    fn set_text_changed(&self, handler: Option<Shared<dyn Fn(String)>>) -> Result<()> {
        let mut kind = self.0.kind.borrow_mut();
        let XamlKind::TextBox(element) = &mut *kind else {
            return Ok(());
        };

        element.on_text_change = handler.clone();
        if let Some(text_box) = &element.realized {
            element.attach_text_changed_handler(text_box, handler)?;
        }
        Ok(())
    }

    fn set_tab_selected(&self, handler: Shared<dyn Fn(String)>) -> Result<()> {
        let mut kind = self.0.kind.borrow_mut();
        let XamlKind::TabView(element) = &mut *kind else {
            return Ok(());
        };
        element.selected_changed.replace(Some(handler));
        if let Some(realized) = element.realized.clone() {
            element.attach_selection_handler(&realized)?;
        }
        Ok(())
    }

    fn set_tab_content_resized(&self, handler: Shared<dyn Fn(f32, f32)>) -> Result<()> {
        let mut kind = self.0.kind.borrow_mut();
        let XamlKind::TabView(element) = &mut *kind else {
            return Ok(());
        };
        element.content_resized.replace(Some(handler));
        if let Some(realized) = element.realized.clone() {
            element.attach_content_resize_handler(&realized)?;
        }
        Ok(())
    }

    fn set_visible(&self, visible: bool) -> Result<()> {
        let kind = self.0.kind.borrow();
        let XamlKind::TabViewItem(element) = &*kind else {
            return Ok(());
        };
        if let Some(realized) = &element.realized {
            realized.content.SetVisibility(if visible {
                Visibility::Visible
            } else {
                Visibility::Collapsed
            })?;
        }
        Ok(())
    }

    fn set_tab_item_id(&self, id: String) -> Result<()> {
        let mut kind = self.0.kind.borrow_mut();
        let XamlKind::TabViewItem(element) = &mut *kind else {
            return Ok(());
        };
        element.id = id.clone();
        if let Some(realized) = &element.realized {
            realized.selector_item.SetName(&HSTRING::from(id))?;
        }
        Ok(())
    }

    fn set_background_color(&self, color: Option<nestix_native_core::Color>) -> Result<()> {
        let mut kind = self.0.kind.borrow_mut();
        let XamlKind::Canvas(element) = &mut *kind else {
            return Ok(());
        };

        element.background_color = color;
        if let Some(canvas) = &element.realized {
            set_canvas_background(canvas, color)?;
        }
        Ok(())
    }

    fn set_scroll_enabled(&self, scroll_x: bool, scroll_y: bool) -> Result<()> {
        let mut kind = self.0.kind.borrow_mut();
        let XamlKind::ScrollView(element) = &mut *kind else {
            return Ok(());
        };
        element.scroll_x = scroll_x;
        element.scroll_y = scroll_y;
        if let Some(scroll_view) = &element.realized {
            configure_scroll_view(scroll_view, scroll_x, scroll_y)?;
        }
        Ok(())
    }

    fn set_image_source(&self, source: nestix_native_core::ImageSource) -> Result<()> {
        let mut kind = self.0.kind.borrow_mut();
        let XamlKind::Image(element) = &mut *kind else {
            return Ok(());
        };
        element.source = Some(source);
        if element.realized.is_some() {
            element.apply_source()?;
        }
        drop(kind);
        self.measure_intrinsic()
    }

    fn set_image_content_fit(&self, fit: nestix_native_core::ContentFit) -> Result<()> {
        let mut kind = self.0.kind.borrow_mut();
        let XamlKind::Image(element) = &mut *kind else {
            return Ok(());
        };
        element.content_fit = fit;
        if let Some(image) = &element.realized {
            image.SetStretch(stretch_for_fit(fit))?;
        }
        drop(kind);
        self.apply_layout()
    }

    fn apply_background_color(&self) -> Result<()> {
        let kind = self.0.kind.borrow();
        let XamlKind::Canvas(element) = &*kind else {
            return Ok(());
        };

        if let Some(canvas) = &element.realized {
            set_canvas_background(canvas, element.background_color)?;
        }
        Ok(())
    }

    pub fn set_layout(&self, x: f64, y: f64, width: f64, height: f64) -> Result<()> {
        self.0.layout.replace(Some(XamlLayout {
            x,
            y,
            width,
            height,
        }));
        self.apply_layout()
    }

    #[cfg(test)]
    pub(crate) fn cached_layout(&self) -> Option<(f64, f64, f64, f64)> {
        self.0
            .layout
            .borrow()
            .map(|layout| (layout.x, layout.y, layout.width, layout.height))
    }

    pub fn set_measure_callback(&self, callback: Shared<dyn Fn(f32, f32)>) -> Result<()> {
        self.0.measure_callback.replace(Some(callback));
        self.measure_intrinsic()
    }

    fn apply_layout(&self) -> Result<()> {
        let Some(layout) = *self.0.layout.borrow() else {
            return Ok(());
        };
        if !is_xaml_running() || !self.is_realized() {
            return Ok(());
        }

        let ui_element = self.as_ui_element()?;
        let framework_element = ui_element.cast::<FrameworkElement>()?;
        framework_element.SetWidth(layout.width)?;
        framework_element.SetHeight(layout.height)?;
        Canvas::SetLeft(&ui_element, layout.x)?;
        Canvas::SetTop(&ui_element, layout.y)?;

        if let XamlKind::Image(element) = &*self.0.kind.borrow()
            && element.content_fit == nestix_native_core::ContentFit::ScaleDown
            && let Some(image) = &element.realized
        {
            image.SetStretch(Stretch::None)?;
            image.Measure(Size {
                Width: f32::INFINITY,
                Height: f32::INFINITY,
            })?;
            let natural = image.DesiredSize()?;
            image.SetStretch(
                if layout.width >= natural.Width as f64 && layout.height >= natural.Height as f64 {
                    Stretch::None
                } else {
                    Stretch::Uniform
                },
            )?;
        }

        Ok(())
    }

    fn measure_intrinsic(&self) -> Result<()> {
        let Some(callback) = self.0.measure_callback.borrow().clone() else {
            return Ok(());
        };
        if !is_xaml_running() || !self.is_realized() {
            return Ok(());
        }

        let ui_element = self.as_ui_element()?;
        let framework_element = ui_element.cast::<FrameworkElement>()?;
        framework_element.SetWidth(f64::NAN)?;
        framework_element.SetHeight(f64::NAN)?;
        let available = Size {
            Width: f32::INFINITY,
            Height: f32::INFINITY,
        };
        let desired = match &*self.0.kind.borrow() {
            XamlKind::Button(element) => {
                let realized = element.realized.as_ref().unwrap();
                realized.control.ApplyTemplate()?;
                let control_padding = realized.control.Padding()?;
                let padding = if control_padding == Default::default() {
                    crate::xaml_app::theme_thickness("ButtonPadding").unwrap_or(control_padding)
                } else {
                    control_padding
                };
                realized.label.SetWidth(f64::NAN)?;
                realized.label.SetHeight(f64::NAN)?;
                realized.label.Measure(available)?;
                let text_size = realized.label.DesiredSize()?;
                Size {
                    Width: (text_size.Width
                        + padding.Left as f32
                        + padding.Right as f32
                        + BUTTON_INTRINSIC_SLACK)
                        .max(realized.control.MinWidth()? as f32),
                    Height: (text_size.Height
                        + padding.Top as f32
                        + padding.Bottom as f32
                        + BUTTON_INTRINSIC_SLACK)
                        .max(realized.control.MinHeight()? as f32),
                }
            }
            XamlKind::TextBox(element) => {
                let text_box = element.realized.as_ref().unwrap();
                text_box.ApplyTemplate()?;
                ui_element.Measure(available)?;
                let desired = ui_element.DesiredSize()?;
                let min_width = (text_box.MinWidth()? as f32).max(
                    crate::xaml_app::theme_f64("TextControlThemeMinWidth").unwrap_or(64.0) as f32,
                );
                let min_height = (text_box.MinHeight()? as f32).max(
                    crate::xaml_app::theme_f64("TextControlThemeMinHeight").unwrap_or(32.0) as f32,
                );
                Size {
                    Width: desired.Width.max(min_width),
                    Height: desired.Height.max(min_height),
                }
            }
            XamlKind::CheckBox(element) => {
                let (control, label) = element.realized.as_ref().unwrap();
                control.ApplyTemplate()?;
                label.SetWidth(f64::NAN)?;
                label.SetHeight(f64::NAN)?;
                label.Measure(available)?;
                let text_size = label.DesiredSize()?;
                let control_padding = control.Padding()?;
                let padding = if control_padding == Default::default() {
                    crate::xaml_app::theme_thickness("CheckBoxPadding").unwrap_or(control_padding)
                } else {
                    control_padding
                };
                let glyph_width = crate::xaml_app::theme_f64("CheckBoxSize").unwrap_or_default();
                let template_height =
                    crate::xaml_app::theme_f64("CheckBoxHeight").unwrap_or_default();
                Size {
                    Width: (glyph_width as f32
                        + padding.Left as f32
                        + padding.Right as f32
                        + text_size.Width)
                        .max(control.MinWidth()? as f32),
                    Height: (padding.Top as f32 + padding.Bottom as f32 + text_size.Height)
                        .max(template_height as f32)
                        .max(control.MinHeight()? as f32),
                }
            }
            XamlKind::RadioButton(element) => {
                let (control, label) = element.realized.as_ref().unwrap();
                control.ApplyTemplate()?;
                label.SetWidth(f64::NAN)?;
                label.SetHeight(f64::NAN)?;
                label.Measure(available)?;
                let text_size = label.DesiredSize()?;
                let padding = control.Padding()?;
                Size {
                    Width: (RADIO_BUTTON_GLYPH_COLUMN_WIDTH
                        + padding.Left as f32
                        + padding.Right as f32
                        + text_size.Width)
                        .max(control.MinWidth()? as f32),
                    Height: (padding.Top as f32 + padding.Bottom as f32 + text_size.Height)
                        .max(RADIO_BUTTON_INTRINSIC_HEIGHT)
                        .max(control.MinHeight()? as f32),
                }
            }
            XamlKind::Select(element) => {
                let control = element.realized.as_ref().unwrap();
                control.ApplyTemplate()?;
                ui_element.Measure(available)?;
                let desired = ui_element.DesiredSize()?;
                let template_height = crate::xaml_app::theme_f64("ComboBoxMinHeight")
                    .unwrap_or(desired.Height as f64) as f32;
                Size {
                    Width: desired.Width.max(control.MinWidth()? as f32),
                    Height: template_height.max(control.MinHeight()? as f32),
                }
            }
            XamlKind::Switch(element) => {
                let control = element.realized.as_ref().unwrap();
                control.ApplyTemplate()?;
                ui_element.Measure(available)?;
                let desired = ui_element.DesiredSize()?;
                let template_height = TOGGLE_SWITCH_TRACK_HEIGHT
                    + crate::xaml_app::theme_f64("ToggleSwitchPreContentMargin").unwrap_or_default()
                        as f32
                    + crate::xaml_app::theme_f64("ToggleSwitchPostContentMargin")
                        .unwrap_or_default() as f32;
                Size {
                    Width: desired.Width.max(control.MinWidth()? as f32),
                    Height: desired
                        .Height
                        .max(template_height)
                        .max(control.MinHeight()? as f32),
                }
            }
            XamlKind::Image(element) => {
                let image = element.realized.as_ref().unwrap();
                image.SetStretch(Stretch::None)?;
                ui_element.Measure(available)?;
                let desired = ui_element.DesiredSize()?;
                image.SetStretch(stretch_for_fit(element.content_fit))?;
                desired
            }
            _ => {
                ui_element.Measure(available)?;
                ui_element.DesiredSize()?
            }
        };
        callback(desired.Width, desired.Height);
        self.apply_layout()
    }

    fn measure_intrinsic_recursive(&self) -> Result<()> {
        self.measure_intrinsic()?;
        for child in self.0.children.borrow().clone() {
            child.measure_intrinsic_recursive()?;
        }
        Ok(())
    }

    pub(crate) fn realize(&self) -> Result<()> {
        if self.is_realized() {
            return Ok(());
        }

        match &mut *self.0.kind.borrow_mut() {
            XamlKind::Window(element) => element.realize()?,
            XamlKind::Canvas(element) => element.realize()?,
            XamlKind::ScrollView(element) => element.realize()?,
            XamlKind::Button(element) => element.realize()?,
            XamlKind::CheckBox(element) => element.realize()?,
            XamlKind::RadioButton(element) => element.realize()?,
            XamlKind::Select(element) => element.realize()?,
            XamlKind::Switch(element) => element.realize()?,
            XamlKind::Slider(element) => element.realize()?,
            XamlKind::TextBlock(element) => element.realize()?,
            XamlKind::TextBox(element) => element.realize()?,
            XamlKind::Image(element) => element.realize()?,
            XamlKind::MenuBar(element) => element.realize()?,
            XamlKind::TabView(element) => element.realize()?,
            XamlKind::TabViewItem(element) => element.realize()?,
        }

        let realized = self.as_ui_element();
        if let Ok(realized) = realized {
            for callback in self.0.realized_callbacks.borrow().values().cloned() {
                callback(realized.clone());
            }
        }

        let children = self.0.children.borrow().clone();
        for (index, child) in children.into_iter().enumerate() {
            child.realize()?;
            self.insert_realized_child(&child, index)?;
            child.measure_intrinsic_recursive()?;
        }
        self.measure_intrinsic()?;
        self.apply_layout()?;
        self.apply_background_color()?;
        self.notify_scale_factor_changed()?;
        if self.0.context_menu.borrow().is_some() {
            self.apply_context_menu()?;
        }
        Ok(())
    }

    fn new(kind: XamlKind) -> Self {
        Self(Rc::new(XamlNode {
            kind: RefCell::new(kind),
            children: RefCell::new(Vec::new()),
            layout: RefCell::new(None),
            measure_callback: RefCell::new(None),
            context_menu: RefCell::new(None),
            realized_callbacks: RefCell::new(HashMap::new()),
        }))
    }

    fn is_realized(&self) -> bool {
        match &*self.0.kind.borrow() {
            XamlKind::Window(element) => element.realized.is_some(),
            XamlKind::Canvas(element) => element.realized.is_some(),
            XamlKind::ScrollView(element) => element.realized.is_some(),
            XamlKind::Button(element) => element.realized.is_some(),
            XamlKind::CheckBox(element) => element.realized.is_some(),
            XamlKind::RadioButton(element) => element.realized.is_some(),
            XamlKind::Select(element) => element.realized.is_some(),
            XamlKind::Switch(element) => element.realized.is_some(),
            XamlKind::Slider(element) => element.realized.is_some(),
            XamlKind::TextBlock(element) => element.realized.is_some(),
            XamlKind::TextBox(element) => element.realized.is_some(),
            XamlKind::Image(element) => element.realized.is_some(),
            XamlKind::MenuBar(element) => element.realized.is_some(),
            XamlKind::TabView(element) => element.realized.is_some(),
            XamlKind::TabViewItem(element) => element.realized.is_some(),
        }
    }

    fn insert_realized_child(&self, child: &XamlElement, index: usize) -> Result<()> {
        let child_element = child;
        let child = child.as_ui_element()?;
        match &mut *self.0.kind.borrow_mut() {
            XamlKind::Window(element) => {
                if let Some(window) = element.realized.clone() {
                    let framework_element = child.cast::<FrameworkElement>()?;
                    framework_element.SetWidth(f64::NAN)?;
                    framework_element.SetHeight(f64::NAN)?;
                    framework_element.SetHorizontalAlignment(HorizontalAlignment::Stretch)?;
                    framework_element.SetVerticalAlignment(VerticalAlignment::Stretch)?;
                    window.SetContent(&child)?;
                    element.attach_scale_factor_handler(&window)?;
                    element.attach_resize_handler(&window)?;
                }
            }
            XamlKind::Canvas(element) => {
                if let Some(canvas) = &element.realized {
                    let children = canvas.Children()?;
                    let mut existing_index = 0;
                    if children.IndexOf(&child, &mut existing_index)? {
                        children.RemoveAt(existing_index)?;
                    }
                    children.InsertAt(index.min(children.Size()? as usize) as u32, &child)?;
                }
            }
            XamlKind::ScrollView(element) => {
                if let Some(scroll_view) = &element.realized {
                    scroll_view.SetContent(&child)?;
                }
            }
            XamlKind::TabView(element) => {
                if let Some(realized) = &element.realized {
                    let child_kind = child_element.0.kind.borrow();
                    let XamlKind::TabViewItem(item) = &*child_kind else {
                        return Ok(());
                    };
                    let item = item.realized.as_ref().unwrap();
                    let items = realized.selector_bar.Items()?;
                    let mut old_index = 0;
                    if items.IndexOf(&item.selector_item, &mut old_index)? {
                        items.RemoveAt(old_index)?;
                    }
                    let index = index.min(items.Size()? as usize) as u32;
                    items.InsertAt(index, &item.selector_item)?;

                    let pages = realized.content.Children()?;
                    let page: UIElement = item.content.cast()?;
                    if pages.IndexOf(&page, &mut old_index)? {
                        pages.RemoveAt(old_index)?;
                    }
                    pages.InsertAt(index.min(pages.Size()?), &page)?;
                    if realized.selector_bar.SelectedItem().is_err() {
                        realized.selector_bar.SetSelectedItem(&item.selector_item)?;
                    }
                }
            }
            XamlKind::TabViewItem(element) => {
                if let Some(realized) = &element.realized {
                    let framework_element = child.cast::<FrameworkElement>()?;
                    framework_element.SetWidth(f64::NAN)?;
                    framework_element.SetHeight(f64::NAN)?;
                    framework_element.SetHorizontalAlignment(HorizontalAlignment::Stretch)?;
                    framework_element.SetVerticalAlignment(VerticalAlignment::Stretch)?;
                    let children = realized.content.Children()?;
                    let mut old_index = 0;
                    if children.IndexOf(&child, &mut old_index)? {
                        children.RemoveAt(old_index)?;
                    }
                    children.InsertAt(index.min(children.Size()? as usize) as u32, &child)?;
                }
            }
            XamlKind::Button(_)
            | XamlKind::CheckBox(_)
            | XamlKind::RadioButton(_)
            | XamlKind::Select(_)
            | XamlKind::Switch(_)
            | XamlKind::Slider(_)
            | XamlKind::TextBlock(_)
            | XamlKind::TextBox(_)
            | XamlKind::MenuBar(_)
            | XamlKind::Image(_) => {}
        }
        Ok(())
    }

    fn as_ui_element(&self) -> Result<UIElement> {
        self.realize()?;
        match &*self.0.kind.borrow() {
            XamlKind::Window(_) => Err(Error::new(E_NOTIMPL, "Window is not a UIElement.")),
            XamlKind::Canvas(element) => element.realized.as_ref().unwrap().cast(),
            XamlKind::ScrollView(element) => element.realized.as_ref().unwrap().cast(),
            XamlKind::Button(element) => element.realized.as_ref().unwrap().control.cast(),
            XamlKind::CheckBox(element) => element.realized.as_ref().unwrap().0.cast(),
            XamlKind::RadioButton(element) => element.realized.as_ref().unwrap().0.cast(),
            XamlKind::Select(element) => element.realized.as_ref().unwrap().cast(),
            XamlKind::Switch(element) => element.realized.as_ref().unwrap().cast(),
            XamlKind::Slider(element) => element.realized.as_ref().unwrap().cast(),
            XamlKind::TextBlock(element) => element.realized.as_ref().unwrap().cast(),
            XamlKind::TextBox(element) => element.realized.as_ref().unwrap().cast(),
            XamlKind::Image(element) => element.realized.as_ref().unwrap().cast(),
            XamlKind::MenuBar(element) => element.realized.as_ref().unwrap().cast(),
            XamlKind::TabView(element) => element.realized.as_ref().unwrap().control.cast(),
            XamlKind::TabViewItem(element) => element.realized.as_ref().unwrap().content.cast(),
        }
    }

    pub(crate) fn as_framework_element(&self) -> Result<FrameworkElement> {
        self.as_ui_element()?.cast()
    }

    fn set_menu_bar_menu(&self, menu: Option<Rc<crate::menu::MenuData>>) -> Result<()> {
        let mut kind = self.0.kind.borrow_mut();
        let XamlKind::MenuBar(element) = &mut *kind else {
            return Err(Error::new(E_NOTIMPL, "Element is not a MenuBar."));
        };
        if let (Some(previous), Some(control)) = (&element.menu, &element.realized) {
            previous.detach_bar(control);
        }
        element.menu = menu;
        if let (Some(menu), Some(control)) = (&element.menu, &element.realized) {
            menu.attach_bar(control)?;
        } else if let Some(control) = &element.realized {
            let items = control.Items()?;
            while items.Size()? > 0 {
                items.RemoveAtEnd()?;
            }
        }
        drop(kind);
        self.measure_intrinsic()
    }

    pub(crate) fn set_context_menu(&self, menu: Option<Rc<crate::menu::MenuData>>) -> Result<()> {
        *self.0.context_menu.borrow_mut() = menu;
        if is_xaml_running() && self.is_realized() {
            self.apply_context_menu()?;
        }
        Ok(())
    }

    pub(crate) fn clear_context_menu_if(&self, menu: &Rc<crate::menu::MenuData>) -> Result<()> {
        let owns_menu = self
            .0
            .context_menu
            .borrow()
            .as_ref()
            .is_some_and(|current| Rc::ptr_eq(current, menu));
        if owns_menu {
            self.set_context_menu(None)?;
        }
        Ok(())
    }

    fn apply_context_menu(&self) -> Result<()> {
        let target = self.as_ui_element()?.cast::<FrameworkElement>()?;
        if let Some(menu) = self.0.context_menu.borrow().clone() {
            menu.attach(&target)
        } else {
            target.SetContextFlyout(
                None::<&crate::bindings::Microsoft::UI::Xaml::Controls::Primitives::FlyoutBase>,
            )
        }
    }

    fn window_hwnd(&self) -> Result<windows::Win32::Foundation::HWND> {
        self.realize()?;
        match &*self.0.kind.borrow() {
            XamlKind::Window(element) => crate::window_native::window_hwnd(
                element.realized.as_ref().expect("realized WinUI window"),
            ),
            _ => Err(Error::new(E_NOTIMPL, "Element is not a Window.")),
        }
    }

    fn with_window(&self, callback: impl FnOnce(&Window) -> Result<()>) -> Result<()> {
        match &*self.0.kind.borrow() {
            XamlKind::Window(element) => {
                if let Some(window) = &element.realized {
                    callback(window)
                } else {
                    Ok(())
                }
            }
            _ => Ok(()),
        }
    }

    fn notify_scale_factor_changed(&self) -> Result<()> {
        let Some((window, callback_id)) = (match &*self.0.kind.borrow() {
            XamlKind::Window(element) => element.realized.clone().and_then(|window| {
                element
                    .scale_factor_callback
                    .borrow()
                    .as_ref()
                    .map(RegisteredScaleFactorCallback::id)
                    .map(|callback_id| (window, callback_id))
            }),
            _ => None,
        }) else {
            return Ok(());
        };

        RegisteredScaleFactorCallback::invoke(
            callback_id,
            crate::window_native::window_scale_factor(&window),
        );
        Ok(())
    }
}

impl WindowState {
    fn realize(&mut self) -> Result<()> {
        let window = Window::new()?;
        window.SetTitle(&HSTRING::from(&self.title))?;
        apply_title_bar_mode(&window, self.title_bar_mode)?;
        self.realized = Some(window);
        self.set_window_size()?;
        if let Some(window) = self.realized.clone() {
            self.attach_scale_factor_handler(&window)?;
            self.attach_resize_handler(&window)?;
            self.attach_close_requested_handler(&window)?;
        }
        Ok(())
    }

    fn set_window_size(&self) -> Result<()> {
        if let Some(window) = &self.realized {
            window.AppWindow()?.ResizeClient(SizeInt32 {
                Width: self.width,
                Height: self.height,
            })?;
        }
        Ok(())
    }

    fn attach_scale_factor_handler(&mut self, window: &Window) -> Result<()> {
        self.scale_factor_handler.take();
        let Some(callback_id) = self
            .scale_factor_callback
            .borrow()
            .as_ref()
            .map(RegisteredScaleFactorCallback::id)
        else {
            return Ok(());
        };

        let content = match window.Content() {
            Ok(content) => content.cast::<FrameworkElement>()?,
            Err(_) => return Ok(()),
        };
        let hwnd = crate::window_native::window_hwnd(window)?;
        let hwnd_value = hwnd.0 as isize;
        let revoker = content.SizeChanged(move |_, _| {
            RegisteredScaleFactorCallback::invoke(
                callback_id,
                crate::window_native::hwnd_scale_factor(windows::Win32::Foundation::HWND(
                    hwnd_value as _,
                )),
            );
        })?;
        self.scale_factor_handler
            .replace(Some(ScaleFactorHandlerState {
                callback_id,
                _revoker: revoker,
            }));
        Ok(())
    }

    fn detach_scale_factor_handler(&mut self) {
        self.scale_factor_handler.take();
        self.scale_factor_callback.take();
    }

    fn attach_resize_handler(&mut self, window: &Window) -> Result<()> {
        self.resize_handler.take();
        let Some(callback_id) = self
            .resize_callback
            .borrow()
            .as_ref()
            .map(RegisteredResizeCallback::id)
        else {
            return Ok(());
        };

        let content = match window.Content() {
            Ok(content) => content.cast::<FrameworkElement>()?,
            Err(_) => return Ok(()),
        };
        let app_window = window.AppWindow()?;
        let revoker = content.SizeChanged(move |_, _| {
            if let Ok(size) = app_window.ClientSize() {
                RegisteredResizeCallback::invoke(
                    callback_id,
                    nestix_native_core::dpi::Size::Physical(
                        nestix_native_core::dpi::PhysicalSize::new(
                            size.Width as u32,
                            size.Height as u32,
                        ),
                    ),
                );
            }
        })?;
        self.resize_handler.replace(Some(ResizeHandlerState {
            callback_id,
            _revoker: revoker,
        }));
        Ok(())
    }

    fn detach_resize_handler(&mut self) {
        self.resize_handler.take();
        self.resize_callback.take();
    }

    fn attach_close_requested_handler(&mut self, window: &Window) -> Result<()> {
        self.close_requested_handler.take();
        let Some(callback_id) = self
            .close_requested_callback
            .borrow()
            .as_ref()
            .map(RegisteredClickCallback::id)
        else {
            return Ok(());
        };

        let revoker = window.AppWindow()?.Closing(move |_, args| {
            if let Some(args) = args.as_ref() {
                let _ = args.SetCancel(true);
            }
            RegisteredClickCallback::invoke(callback_id);
        })?;
        self.close_requested_handler
            .replace(Some(CloseRequestedHandlerState {
                callback_id,
                _revoker: revoker,
            }));
        Ok(())
    }

    fn detach_close_requested_handler(&mut self) {
        self.close_requested_handler.take();
        self.close_requested_callback.take();
    }
}

fn apply_title_bar_mode(window: &Window, mode: TitleBarMode) -> Result<()> {
    let presenter = window
        .AppWindow()?
        .Presenter()?
        .cast::<OverlappedPresenter>()?;
    presenter.SetBorderAndTitleBar(true, mode != TitleBarMode::Hidden)?;
    window.SetExtendsContentIntoTitleBar(mode == TitleBarMode::Overlay)
}

impl CanvasState {
    fn realize(&mut self) -> Result<()> {
        self.realized = Some(Canvas::new()?);
        Ok(())
    }
}

impl MenuBarState {
    fn realize(&mut self) -> Result<()> {
        let control = MenuBar::new()?;
        if let Some(menu) = &self.menu {
            menu.attach_bar(&control)?;
        }
        self.realized = Some(control);
        Ok(())
    }
}

impl ImageState {
    fn realize(&mut self) -> Result<()> {
        let image = Image::new()?;
        image.SetStretch(stretch_for_fit(self.content_fit))?;
        self.realized = Some(image);
        self.apply_source()
    }

    fn apply_source(&mut self) -> Result<()> {
        self.opened_handler.take();
        let Some(image) = &self.realized else {
            return Ok(());
        };
        let Some(source) = self.source.clone() else {
            image.SetSource(None)?;
            self.stream = None;
            return Ok(());
        };
        let bytes = match source {
            nestix_native_core::ImageSource::File(path) => {
                std::fs::read(&path).map_err(|error| {
                    Error::new(
                        E_FAIL,
                        format!("failed to read image {}: {error}", path.display()),
                    )
                })?
            }
            nestix_native_core::ImageSource::Bytes(bytes) => bytes.to_vec(),
        };
        let stream = InMemoryRandomAccessStream::new()?;
        let writer = DataWriter::CreateDataWriter(&stream)?;
        writer.WriteBytes(&bytes)?;
        writer.StoreAsync()?.join()?;
        writer.DetachStream()?;
        stream.Seek(0)?;

        let bitmap = BitmapImage::new()?;
        if let Some(callback) = self.opened_callback.clone() {
            callback(0.0, 0.0);
            let callback = RegisteredContentSizeCallback::register(callback);
            let callback_id = callback.id();
            let opened_bitmap = bitmap.clone();
            let revoker = bitmap.ImageOpened(move |_, _| {
                let width = opened_bitmap.PixelWidth().unwrap_or_default().max(0) as f32;
                let height = opened_bitmap.PixelHeight().unwrap_or_default().max(0) as f32;
                RegisteredContentSizeCallback::invoke(callback_id, width, height);
            })?;
            self.opened_handler.replace(Some(ImageOpenedHandlerState {
                _callback: callback,
                _revoker: revoker,
            }));
        }
        // WinUI metadata is generated locally while the stream implementation
        // comes from windows-rs. They describe the same WinRT interface but are
        // distinct Rust wrapper types, so transfer one owned interface reference.
        let native_stream: NativeRandomAccessStream = stream.cast()?;
        let raw_stream = native_stream.into_raw();
        let xaml_stream = unsafe {
            crate::bindings::Windows::Storage::Streams::IRandomAccessStream::from_raw(raw_stream)
        };
        bitmap.SetSource(&xaml_stream)?;
        image.SetSource(&bitmap)?;
        self.stream = Some(stream);
        Ok(())
    }
}

fn stretch_for_fit(fit: nestix_native_core::ContentFit) -> Stretch {
    match fit {
        nestix_native_core::ContentFit::Contain => Stretch::Uniform,
        nestix_native_core::ContentFit::Cover => Stretch::UniformToFill,
        nestix_native_core::ContentFit::Fill => Stretch::Fill,
        nestix_native_core::ContentFit::None => Stretch::None,
        nestix_native_core::ContentFit::ScaleDown => Stretch::Uniform,
    }
}

impl ScrollViewState {
    fn realize(&mut self) -> Result<()> {
        let scroll_view = ScrollView::new()?;
        configure_scroll_view(&scroll_view, self.scroll_x, self.scroll_y)?;
        self.realized = Some(scroll_view);
        Ok(())
    }
}

fn configure_scroll_view(scroll_view: &ScrollView, scroll_x: bool, scroll_y: bool) -> Result<()> {
    let orientation = match (scroll_x, scroll_y) {
        (true, true) => ScrollingContentOrientation::Both,
        (true, false) => ScrollingContentOrientation::Horizontal,
        (false, true) => ScrollingContentOrientation::Vertical,
        (false, false) => ScrollingContentOrientation::None,
    };
    scroll_view.SetContentOrientation(orientation)?;
    scroll_view.SetHorizontalScrollBarVisibility(if scroll_x {
        ScrollingScrollBarVisibility::Auto
    } else {
        ScrollingScrollBarVisibility::Hidden
    })?;
    scroll_view.SetVerticalScrollBarVisibility(if scroll_y {
        ScrollingScrollBarVisibility::Auto
    } else {
        ScrollingScrollBarVisibility::Hidden
    })
}

impl ButtonState {
    fn realize(&mut self) -> Result<()> {
        let control = Button::new()?;
        let label = TextBlock::new()?;
        label.SetText(&HSTRING::from(&self.title))?;
        apply_font(&label, &self.font)?;
        apply_button_padding(&control, self.padding)?;
        control.SetContent(&label)?;
        control.SetIsEnabled(self.enabled)?;
        self.attach_click_handler(&control, self.on_click.clone())?;
        self.realized = Some(RealizedButton { control, label });
        Ok(())
    }

    fn attach_click_handler(
        &self,
        control: &Button,
        handler: Option<nestix::Shared<dyn Fn()>>,
    ) -> Result<()> {
        self.click_handler.take();
        let Some(handler) = handler else {
            return Ok(());
        };

        let callback = RegisteredClickCallback::register(handler);
        let callback_id = callback.id();
        let revoker = control.Click(move |_, _| {
            RegisteredClickCallback::invoke(callback_id);
        })?;
        self.click_handler.replace(Some(ClickHandlerState {
            callback,
            _revoker: revoker,
        }));
        Ok(())
    }
}

impl CheckBoxState {
    fn realize(&mut self) -> Result<()> {
        let control = CheckBox::new()?;
        let label = TextBlock::new()?;
        label.SetText(&HSTRING::from(&self.title))?;
        apply_font(&label, &self.font)?;
        control.SetContent(&label)?;
        control.SetIsEnabled(self.enabled)?;
        control.SetIsChecked(Some(self.checked))?;
        self.realized = Some((control, label));
        self.attach_handler()
    }

    fn attach_handler(&self) -> Result<()> {
        self.handler.take();
        let (Some(handler), Some((control, _))) = (self.on_change.clone(), self.realized.as_ref())
        else {
            return Ok(());
        };
        let callback = RegisteredBoolCallback::register(handler);
        let id = callback.id();
        let updating = self.updating.clone();
        let control_for_event = control.clone();
        let revoker = control.Click(move |_, _| {
            if !updating.load(Ordering::SeqCst)
                && let Ok(value) = control_for_event.IsChecked()
            {
                RegisteredBoolCallback::invoke(id, value);
            }
        })?;
        self.handler.replace(Some(ValueHandlerState {
            _callback: callback,
            _revoker: revoker,
        }));
        Ok(())
    }
}

impl RadioButtonState {
    fn realize(&mut self) -> Result<()> {
        let control = RadioButton::new()?;
        let label = TextBlock::new()?;
        label.SetText(&HSTRING::from(&self.title))?;
        apply_font(&label, &self.font)?;
        control.SetContent(&label)?;
        control.SetIsEnabled(self.enabled)?;
        control.SetGroupName(&HSTRING::from(&self.group))?;
        control.SetIsChecked(Some(self.selected))?;
        self.realized = Some((control, label));
        self.attach_handler()
    }

    fn attach_handler(&self) -> Result<()> {
        self.handler.take();
        let (Some(handler), Some((control, _))) = (self.on_select.clone(), self.realized.as_ref())
        else {
            return Ok(());
        };
        let callback = RegisteredClickCallback::register(handler);
        let id = callback.id();
        let updating = self.updating.clone();
        let control_for_event = control.clone();
        let revoker = control.Click(move |_, _| {
            if !updating.load(Ordering::SeqCst) && control_for_event.IsChecked().unwrap_or(false) {
                RegisteredClickCallback::invoke(id);
            }
        })?;
        self.handler.replace(Some(ClickHandlerState {
            callback,
            _revoker: revoker,
        }));
        Ok(())
    }
}

impl SelectState {
    fn realize(&mut self) -> Result<()> {
        let control = ComboBox::new()?;
        control.SetIsEnabled(self.enabled)?;
        self.realized = Some(control);
        self.apply_options()?;
        self.attach_handler()
    }

    fn apply_options(&self) -> Result<()> {
        let Some(control) = &self.realized else {
            return Ok(());
        };
        self.updating.store(true, Ordering::SeqCst);
        let result = (|| {
            let items = control.cast::<ItemsControl>()?.Items()?;
            while items.Size()? > 0 {
                items.RemoveAtEnd()?;
            }
            for (_, option) in self.options.lock().unwrap().iter() {
                let item = ComboBoxItem::new()?;
                let label = TextBlock::new()?;
                label.SetText(&HSTRING::from(&option.label))?;
                item.SetContent(&label)?;
                item.SetIsEnabled(option.enabled)?;
                let item: windows_core::IInspectable = item.cast()?;
                items.Append(&item)?;
            }
            self.apply_selection()
        })();
        self.updating.store(false, Ordering::SeqCst);
        result
    }

    fn apply_selection(&self) -> Result<()> {
        let Some(control) = &self.realized else {
            return Ok(());
        };
        let index = self
            .value
            .as_ref()
            .and_then(|value| {
                self.options
                    .lock()
                    .unwrap()
                    .iter()
                    .position(|(_, option)| &option.value == value)
            })
            .map(|index| index as i32)
            .unwrap_or(-1);
        self.updating.store(true, Ordering::SeqCst);
        let result = control.SetSelectedIndex(index);
        self.updating.store(false, Ordering::SeqCst);
        result
    }

    fn attach_handler(&self) -> Result<()> {
        self.handler.take();
        let (Some(handler), Some(control)) = (self.on_change.clone(), self.realized.as_ref())
        else {
            return Ok(());
        };
        let callback = RegisteredStringCallback::register(handler);
        let id = callback.id();
        let updating = self.updating.clone();
        let options = self.options.clone();
        let control_for_event = control.clone();
        let revoker = control.SelectionChanged(move |_, _| {
            if updating.load(Ordering::SeqCst) {
                return;
            }
            let Ok(index) = control_for_event.SelectedIndex() else {
                return;
            };
            let value = (index >= 0)
                .then(|| {
                    options
                        .lock()
                        .unwrap()
                        .get(index as usize)
                        .map(|(_, option)| option.value.clone())
                })
                .flatten();
            // Do not invoke application code while holding the options lock:
            // controlled Select callbacks synchronously update `value`, which
            // needs to acquire the same lock to resolve the selected index.
            if let Some(value) = value {
                RegisteredStringCallback::invoke(id, value);
            }
        })?;
        self.handler.replace(Some(ValueHandlerState {
            _callback: callback,
            _revoker: revoker,
        }));
        Ok(())
    }
}

impl SwitchState {
    fn realize(&mut self) -> Result<()> {
        let control = ToggleSwitch::new()?;
        control.SetIsEnabled(self.enabled)?;
        control.SetIsOn(self.checked)?;
        self.realized = Some(control);
        self.attach_handler()
    }

    fn attach_handler(&self) -> Result<()> {
        self.handler.take();
        let (Some(handler), Some(control)) = (self.on_change.clone(), self.realized.as_ref())
        else {
            return Ok(());
        };
        let callback = RegisteredBoolCallback::register(handler);
        let id = callback.id();
        let updating = self.updating.clone();
        let control_for_event = control.clone();
        let revoker = control.Toggled(move |_, _| {
            if !updating.load(Ordering::SeqCst)
                && let Ok(value) = control_for_event.IsOn()
            {
                RegisteredBoolCallback::invoke(id, value);
            }
        })?;
        self.handler.replace(Some(ValueHandlerState {
            _callback: callback,
            _revoker: revoker,
        }));
        Ok(())
    }
}

impl SliderState {
    fn realize(&mut self) -> Result<()> {
        let control = Slider::new()?;
        control.SetIsEnabled(self.enabled)?;
        control.SetMinimum(self.minimum)?;
        control.SetMaximum(self.maximum)?;
        control.SetValue2(self.value)?;
        self.realized = Some(control);
        self.attach_handler()
    }

    fn attach_handler(&self) -> Result<()> {
        self.handler.take();
        let (Some(handler), Some(control)) = (self.on_change.clone(), self.realized.as_ref())
        else {
            return Ok(());
        };
        let callback = RegisteredF64Callback::register(handler);
        let id = callback.id();
        let updating = self.updating.clone();
        let control_for_event = control.clone();
        let revoker = control.cast::<RangeBase>()?.ValueChanged(move |_, _| {
            if !updating.load(Ordering::SeqCst)
                && let Ok(value) = control_for_event.Value()
            {
                RegisteredF64Callback::invoke(id, value);
            }
        })?;
        self.handler.replace(Some(ValueHandlerState {
            _callback: callback,
            _revoker: revoker,
        }));
        Ok(())
    }
}

fn apply_button_padding(button: &Button, padding: Option<Rect<f64>>) -> Result<()> {
    if let Some(padding) = padding {
        button.SetPadding(crate::bindings::Microsoft::UI::Xaml::Thickness {
            Left: padding.left,
            Top: padding.top,
            Right: padding.right,
            Bottom: padding.bottom,
        })
    } else {
        button.ClearValue(&Control::PaddingProperty()?)
    }
}

impl TextBlockState {
    fn realize(&mut self) -> Result<()> {
        let block = TextBlock::new()?;
        block.SetText(&HSTRING::from(&self.text))?;
        apply_font(&block, &self.font)?;
        self.realized = Some(block);
        Ok(())
    }
}

fn apply_font(block: &TextBlock, font: &ResolvedFontProps) -> Result<()> {
    if let Some(size) = font.font_size {
        block.SetFontSize(size)?;
    } else {
        block.ClearValue(&TextBlock::FontSizeProperty()?)?;
    }
    if let Some(family) = &font.font_family {
        let family = FontFamily::CreateInstanceWithName(&HSTRING::from(family))?;
        block.SetFontFamily(&family)?;
    } else {
        block.ClearValue(&TextBlock::FontFamilyProperty()?)?;
    }
    if let Some(weight) = font.font_weight {
        block.SetFontWeight(UiFontWeight {
            Weight: weight.value(),
        })?;
    } else {
        block.ClearValue(&TextBlock::FontWeightProperty()?)?;
    }
    if let Some(style) = font.font_style {
        block.SetFontStyle(match style {
            FontStyle::Normal => UiFontStyle::Normal,
            FontStyle::Italic => UiFontStyle::Italic,
        })?;
    } else {
        block.ClearValue(&TextBlock::FontStyleProperty()?)?;
    }
    if let Some(color) = font.text_color {
        let rgb = color.into_rgb();
        let brush =
            crate::bindings::Microsoft::UI::Xaml::Media::SolidColorBrush::CreateInstanceWithColor(
                UiColor {
                    A: rgb.alpha,
                    R: rgb.red,
                    G: rgb.green,
                    B: rgb.blue,
                },
            )?;
        block.SetForeground(&brush)?;
    } else {
        block.ClearValue(&TextBlock::ForegroundProperty()?)?;
    }
    Ok(())
}

impl TextBoxState {
    fn realize(&mut self) -> Result<()> {
        let text_box = TextBox::new()?;
        text_box.SetText(&HSTRING::from(&self.text))?;
        self.attach_text_changed_handler(&text_box, self.on_text_change.clone())?;
        self.realized = Some(text_box);
        Ok(())
    }

    fn attach_text_changed_handler(
        &self,
        text_box: &TextBox,
        handler: Option<Shared<dyn Fn(String)>>,
    ) -> Result<()> {
        self.text_changed_handler.take();
        let Some(handler) = handler else {
            return Ok(());
        };

        let callback = RegisteredTextChangedCallback::register(handler);
        let callback_id = callback.id();
        let revoker = text_box.TextChanged(move |sender, _| {
            if let Some(sender) = &*sender
                && let Ok(sender) = sender.cast::<TextBox>()
                && let Ok(text) = sender.Text()
            {
                RegisteredTextChangedCallback::invoke(callback_id, text.to_string_lossy());
            }
        })?;
        self.text_changed_handler
            .replace(Some(TextChangedHandlerState {
                callback,
                _revoker: revoker,
            }));
        Ok(())
    }
}

pub(crate) struct TabSelectionHandlerState {
    _callback: RegisteredTabSelectionCallback,
    _revoker: EventRevoker,
}

pub(crate) struct TabContentResizeHandlerState {
    _callback: RegisteredContentSizeCallback,
    _revoker: EventRevoker,
}

impl TabViewState {
    fn realize(&mut self) -> Result<()> {
        let control = Grid::new()?;
        let selector_bar = SelectorBar::new()?;
        let content = Grid::new()?;

        let rows = control.RowDefinitions()?;
        let header_row = RowDefinition::new()?;
        header_row.SetHeight(GridLength {
            Value: 1.0,
            GridUnitType: GridUnitType::Auto,
        })?;
        rows.Append(&header_row)?;
        let content_row = RowDefinition::new()?;
        content_row.SetHeight(GridLength {
            Value: 1.0,
            GridUnitType: GridUnitType::Star,
        })?;
        rows.Append(&content_row)?;
        Grid::SetRow(&content, 1)?;

        let control_children = control.Children()?;
        control_children.Append(&selector_bar.cast::<UIElement>()?)?;
        control_children.Append(&content.cast::<UIElement>()?)?;

        let realized = RealizedTabView {
            control,
            selector_bar,
            content,
        };
        self.attach_selection_handler(&realized)?;
        self.attach_content_resize_handler(&realized)?;
        self.realized = Some(realized);
        Ok(())
    }

    fn attach_selection_handler(&self, realized: &RealizedTabView) -> Result<()> {
        self.selection_handler.take();
        let Some(callback) = self.selected_changed.borrow().clone() else {
            return Ok(());
        };
        let callback = RegisteredTabSelectionCallback::register(callback);
        let callback_id = callback.id();
        let revoker = realized.selector_bar.SelectionChanged(move |sender, _| {
            if let Some(sender) = &*sender
                && let Ok(selected) = sender.SelectedItem()
                && let Ok(id) = selected.Name()
            {
                RegisteredTabSelectionCallback::invoke(callback_id, id.to_string_lossy());
            }
        })?;
        self.selection_handler
            .replace(Some(TabSelectionHandlerState {
                _callback: callback,
                _revoker: revoker,
            }));
        Ok(())
    }

    fn attach_content_resize_handler(&self, realized: &RealizedTabView) -> Result<()> {
        self.content_resize_handler.take();
        let Some(callback) = self.content_resized.borrow().clone() else {
            return Ok(());
        };
        let callback = RegisteredContentSizeCallback::register(callback);
        let callback_id = callback.id();
        let revoker = realized.content.SizeChanged(move |_, args| {
            if let Some(args) = &*args
                && let Ok(size) = args.NewSize()
            {
                RegisteredContentSizeCallback::invoke(callback_id, size.Width, size.Height);
            }
        })?;
        self.content_resize_handler
            .replace(Some(TabContentResizeHandlerState {
                _callback: callback,
                _revoker: revoker,
            }));
        Ok(())
    }
}

impl TabViewItemState {
    fn realize(&mut self) -> Result<()> {
        let selector_item = SelectorBarItem::new()?;
        selector_item.SetName(&HSTRING::from(&self.id))?;
        selector_item.SetText(&HSTRING::from(&self.title))?;
        let content = Grid::new()?;
        content.SetVisibility(Visibility::Collapsed)?;
        self.realized = Some(RealizedTabViewItem {
            selector_item,
            content,
        });
        Ok(())
    }
}

fn set_canvas_background(canvas: &Canvas, color: Option<nestix_native_core::Color>) -> Result<()> {
    let Some(color) = color else {
        return canvas.SetBackground(None);
    };

    let rgb = color.into_rgb();
    let brush =
        crate::bindings::Microsoft::UI::Xaml::Media::SolidColorBrush::CreateInstanceWithColor(
            UiColor {
                A: rgb.alpha,
                R: rgb.red,
                G: rgb.green,
                B: rgb.blue,
            },
        )?;
    canvas.SetBackground(&brush)
}

#[cfg(test)]
mod tests {
    use super::{CanvasElement, SelectOptionData, XamlElement, XamlKind};
    use nestix::Shared;
    use nestix_native_core::{TitleBarMode, TreeContext};
    use std::rc::Rc;

    #[test]
    fn typed_element_erases_without_changing_identity() {
        let canvas = CanvasElement::new().unwrap();
        assert_eq!(canvas.erased(), canvas.erased());
    }

    #[test]
    fn realized_callback_is_retained_until_registration_is_dropped() {
        let element = XamlElement::canvas().unwrap();
        let callback =
            Shared::from(Rc::new(|_: super::UIElement| {}) as Rc<dyn Fn(super::UIElement)>);
        let registration = element.on_realized(callback).unwrap();
        assert_eq!(element.0.realized_callbacks.borrow().len(), 1);
        drop(registration);
        assert!(element.0.realized_callbacks.borrow().is_empty());
    }

    #[test]
    fn select_options_update_remove_and_reorder_before_realization() {
        let select = XamlElement::new_select();
        let option = |label: &str, value: &str| SelectOptionData {
            label: label.into(),
            value: value.into(),
            enabled: true,
        };
        select
            .upsert_select_option(1, option("First", "first"))
            .unwrap();
        select
            .upsert_select_option(2, option("Second", "second"))
            .unwrap();
        select.move_select_option(2, 0).unwrap();
        select
            .upsert_select_option(1, option("Updated", "first"))
            .unwrap();

        let kind = select.0.kind.borrow();
        let XamlKind::Select(state) = &*kind else {
            panic!("expected select")
        };
        assert_eq!(
            state.options.lock().unwrap().as_slice(),
            &[
                (2, option("Second", "second")),
                (1, option("Updated", "first"))
            ]
        );
        drop(kind);

        select.remove_select_option(2).unwrap();
        let kind = select.0.kind.borrow();
        let XamlKind::Select(state) = &*kind else {
            panic!("expected select")
        };
        assert_eq!(
            state.options.lock().unwrap().as_slice(),
            &[(1, option("Updated", "first"))]
        );
    }

    #[test]
    fn child_operations_preserve_requested_order_before_realization() {
        let parent = XamlElement::canvas().unwrap();
        let first = XamlElement::text_block("first".into()).unwrap();
        let second = XamlElement::text_block("second".into()).unwrap();
        let third = XamlElement::text_block("third".into()).unwrap();

        parent.append_child(first.clone()).unwrap();
        parent.append_child(second.clone()).unwrap();
        parent.append_child(third.clone()).unwrap();
        assert_eq!(
            parent.0.children.borrow().as_slice(),
            &[first.clone(), second.clone(), third.clone()]
        );

        parent.insert_child(first.clone(), 2).unwrap();
        assert_eq!(
            parent.0.children.borrow().as_slice(),
            &[second.clone(), third.clone(), first.clone()]
        );

        parent.insert_child(third.clone(), 0).unwrap();
        assert_eq!(
            parent.0.children.borrow().as_slice(),
            &[third.clone(), second.clone(), first.clone()]
        );

        parent.remove_child(&second).unwrap();
        assert_eq!(parent.0.children.borrow().as_slice(), &[third, first]);
    }

    #[test]
    fn append_moves_an_existing_child_to_the_end() {
        let parent = XamlElement::canvas().unwrap();
        let first = XamlElement::text_block("first".into()).unwrap();
        let second = XamlElement::text_block("second".into()).unwrap();

        parent.append_child(first.clone()).unwrap();
        parent.append_child(second.clone()).unwrap();
        parent.append_child(first.clone()).unwrap();

        assert_eq!(parent.0.children.borrow().as_slice(), &[second, first]);
    }

    #[test]
    fn insert_child_after_uses_rendered_predecessor() {
        let canvas = XamlElement::canvas().unwrap();
        let first = XamlElement::text_block("first".into()).unwrap();
        let button = XamlElement::button("button".into()).unwrap();
        canvas.append_child(first.clone()).unwrap();
        canvas
            .insert_child_after(button.clone(), Some(&first))
            .unwrap();

        let tree = TreeContext::new();
        let parent_node = tree.create_node(false);
        let first_node = tree.create_node(true);
        let button_node = tree.create_node(true);
        tree.set_children(parent_node, &[first_node, button_node]);

        assert_eq!(canvas.child_index(&button), Some(1));
    }

    #[test]
    fn tab_items_preserve_requested_order_before_realization() {
        let tabs = XamlElement::tab_view().unwrap();
        let first = XamlElement::tab_view_item("first".into(), "First".into()).unwrap();
        let second = XamlElement::tab_view_item("second".into(), "Second".into()).unwrap();

        tabs.append_child(first.clone()).unwrap();
        tabs.insert_child(second.clone(), 0).unwrap();
        assert_eq!(
            tabs.0.children.borrow().as_slice(),
            &[second.clone(), first.clone()]
        );

        tabs.remove_child(&second).unwrap();
        assert_eq!(tabs.0.children.borrow().as_slice(), &[first]);
    }

    #[test]
    fn layout_is_cached_before_realization() {
        let element = XamlElement::canvas().unwrap();
        element.set_layout(1.0, 2.0, 30.0, 40.0).unwrap();

        let layout = element.0.layout.borrow().unwrap();
        assert_eq!((layout.x, layout.y), (1.0, 2.0));
        assert_eq!((layout.width, layout.height), (30.0, 40.0));
    }

    #[test]
    fn background_color_is_cached_before_realization() {
        let element = CanvasElement::new().unwrap();
        element
            .set_background_color(Some(nestix_native_core::Color::RED))
            .unwrap();

        let erased = element.erased();
        let kind = erased.0.kind.borrow();
        let XamlKind::Canvas(canvas) = &*kind else {
            panic!("expected canvas");
        };
        assert_eq!(
            canvas.background_color,
            Some(nestix_native_core::Color::RED)
        );
    }

    #[test]
    fn title_bar_mode_is_cached_before_realization() {
        let window = XamlElement::window("title".into(), TitleBarMode::System).unwrap();
        window.set_title_bar_mode(TitleBarMode::Overlay).unwrap();

        let kind = window.0.kind.borrow();
        let XamlKind::Window(window) = &*kind else {
            panic!("expected window");
        };
        assert_eq!(window.title_bar_mode, TitleBarMode::Overlay);
    }
}
