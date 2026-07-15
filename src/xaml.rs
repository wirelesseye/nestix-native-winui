use std::{cell::RefCell, rc::Rc};

use nestix::Shared;
use nestix_native_core::{FontStyle, Rect, ResolvedFontProps};
use windows::Storage::Streams::{
    DataWriter, IRandomAccessStream as NativeRandomAccessStream, InMemoryRandomAccessStream,
};
use windows_core::{Error, EventRevoker, HRESULT, HSTRING, Interface, Result};

use crate::{
    bindings::{
        Microsoft::UI::Xaml::{
            Controls::{
                Button, Canvas, Control, Grid, Image, RowDefinition, ScrollView,
                ScrollingContentOrientation, ScrollingScrollBarVisibility, SelectorBar,
                SelectorBarItem, TextBlock, TextBox,
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
        RegisteredClickCallback, RegisteredContentSizeCallback, RegisteredResizeCallback,
        RegisteredScaleFactorCallback, RegisteredTabSelectionCallback,
        RegisteredTextChangedCallback,
    },
};

const E_NOTIMPL: HRESULT = HRESULT(0x80004001u32 as i32);
const E_FAIL: HRESULT = HRESULT(0x80004005u32 as i32);
const BUTTON_INTRINSIC_SLACK: f32 = 2.0;

pub(crate) struct XamlNode {
    kind: RefCell<XamlKind>,
    children: RefCell<Vec<XamlElement>>,
    layout: RefCell<Option<XamlLayout>>,
    measure_callback: RefCell<Option<Shared<dyn Fn(f32, f32)>>>,
    context_menu: RefCell<Option<Rc<crate::menu::MenuData>>>,
}

#[derive(Debug, Clone)]
enum XamlKind {
    Window(WindowState),
    Canvas(CanvasState),
    ScrollView(ScrollViewState),
    Button(ButtonState),
    TextBlock(TextBlockState),
    TextBox(TextBoxState),
    Image(ImageState),
    TabView(TabViewState),
    TabViewItem(TabViewItemState),
}

#[derive(Debug, Clone)]
struct WindowState {
    title: String,
    width: i32,
    height: i32,
    realized: Option<Window>,
    scale_factor_callback: Rc<RefCell<Option<RegisteredScaleFactorCallback>>>,
    scale_factor_handler: Rc<RefCell<Option<ScaleFactorHandlerState>>>,
    resize_callback: Rc<RefCell<Option<RegisteredResizeCallback>>>,
    resize_handler: Rc<RefCell<Option<ResizeHandlerState>>>,
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
    on_click: Option<nestix::Shared<dyn Fn()>>,
    realized: Option<RealizedButton>,
    click_handler: Rc<RefCell<Option<ClickHandlerState>>>,
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
typed_element!(TextBlockElement);
typed_element!(TextBoxElement);
typed_element!(ImageElement);
typed_element!(TabViewElement);
typed_element!(TabViewItemElement);

impl WindowElement {
    pub(crate) fn new(title: String) -> Result<Self> {
        XamlElement::window(title).map(Self)
    }

    pub(crate) fn activate(&self) -> Result<()> {
        self.0.activate()
    }
    pub(crate) fn set_title(&self, title: String) -> Result<()> {
        self.0.set_text(title)
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
    pub(crate) fn hwnd(&self) -> Result<windows::Win32::Foundation::HWND> {
        self.0.window_hwnd()
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
    fn window(title: String) -> Result<Self> {
        Ok(Self::new(XamlKind::Window(WindowState {
            title,
            width: 200,
            height: 200,
            realized: None,
            scale_factor_callback: Rc::new(RefCell::new(None)),
            scale_factor_handler: Rc::new(RefCell::new(None)),
            resize_callback: Rc::new(RefCell::new(None)),
            resize_handler: Rc::new(RefCell::new(None)),
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
            on_click: None,
            realized: None,
            click_handler: Rc::new(RefCell::new(None)),
        })))
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

    pub fn contains_child(&self, child: &XamlElement) -> bool {
        self.0.children.borrow().contains(child)
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
            | XamlKind::TextBlock(_)
            | XamlKind::TextBox(_)
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
            _ => {
                return Err(Error::new(
                    E_NOTIMPL,
                    "element does not support font styling",
                ));
            }
        }
        self.measure_intrinsic_recursive()
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
            XamlKind::TextBlock(element) => element.realize()?,
            XamlKind::TextBox(element) => element.realize()?,
            XamlKind::Image(element) => element.realize()?,
            XamlKind::TabView(element) => element.realize()?,
            XamlKind::TabViewItem(element) => element.realize()?,
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
        }))
    }

    fn is_realized(&self) -> bool {
        match &*self.0.kind.borrow() {
            XamlKind::Window(element) => element.realized.is_some(),
            XamlKind::Canvas(element) => element.realized.is_some(),
            XamlKind::ScrollView(element) => element.realized.is_some(),
            XamlKind::Button(element) => element.realized.is_some(),
            XamlKind::TextBlock(element) => element.realized.is_some(),
            XamlKind::TextBox(element) => element.realized.is_some(),
            XamlKind::Image(element) => element.realized.is_some(),
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
            | XamlKind::TextBlock(_)
            | XamlKind::TextBox(_)
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
            XamlKind::TextBlock(element) => element.realized.as_ref().unwrap().cast(),
            XamlKind::TextBox(element) => element.realized.as_ref().unwrap().cast(),
            XamlKind::Image(element) => element.realized.as_ref().unwrap().cast(),
            XamlKind::TabView(element) => element.realized.as_ref().unwrap().control.cast(),
            XamlKind::TabViewItem(element) => element.realized.as_ref().unwrap().content.cast(),
        }
    }

    pub(crate) fn as_framework_element(&self) -> Result<FrameworkElement> {
        self.as_ui_element()?.cast()
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
        self.realized = Some(window);
        self.set_window_size()?;
        if let Some(window) = self.realized.clone() {
            self.attach_scale_factor_handler(&window)?;
            self.attach_resize_handler(&window)?;
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
}

impl CanvasState {
    fn realize(&mut self) -> Result<()> {
        self.realized = Some(Canvas::new()?);
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
    use super::{CanvasElement, XamlElement, XamlKind};
    use nestix_native_core::TreeContext;

    #[test]
    fn typed_element_erases_without_changing_identity() {
        let canvas = CanvasElement::new().unwrap();
        assert_eq!(canvas.erased(), canvas.erased());
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
    fn native_child_index_excludes_missing_logical_siblings() {
        let canvas = XamlElement::canvas().unwrap();
        let button = XamlElement::button("button".into()).unwrap();
        canvas.insert_child(button.clone(), 1).unwrap();

        let tree = TreeContext::new();
        let parent_node = tree.create_node(false);
        let button_node = tree.create_node(true);
        tree.insert_child(
            parent_node,
            button_node,
            canvas.child_index(&button).unwrap(),
        );

        assert_eq!(canvas.child_index(&button), Some(0));
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
}
