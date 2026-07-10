use std::{cell::RefCell, rc::Rc};

use nestix::Shared;
use windows_core::{Error, EventRevoker, HRESULT, HSTRING, Interface, Result};

use crate::{
    bindings::{
        Microsoft::UI::Xaml::{
            Controls::{Button, Canvas, TextBlock},
            FrameworkElement, HorizontalAlignment, UIElement, VerticalAlignment, Window,
        },
        Windows::Foundation::Size,
        Windows::Graphics::SizeInt32,
        Windows::UI::Color as UiColor,
    },
    xaml_app::is_xaml_running,
    xaml_events::{
        RegisteredClickCallback, RegisteredResizeCallback, RegisteredScaleFactorCallback,
    },
};

const E_NOTIMPL: HRESULT = HRESULT(0x80004001u32 as i32);

pub(crate) struct XamlNode {
    kind: RefCell<XamlKind>,
    children: RefCell<Vec<XamlElement>>,
    layout: RefCell<Option<XamlLayout>>,
    measure_callback: RefCell<Option<Shared<dyn Fn(f32, f32)>>>,
}

#[derive(Debug, Clone)]
pub(crate) enum XamlKind {
    Window(WindowElement),
    Canvas(CanvasElement),
    Button(ButtonElement),
    TextBlock(TextBlockElement),
}

#[derive(Debug, Clone)]
pub(crate) struct WindowElement {
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
pub(crate) struct CanvasElement {
    background_color: Option<nestix_native_core::Color>,
    realized: Option<Canvas>,
}

#[derive(Debug, Clone)]
pub(crate) struct ButtonElement {
    title: String,
    on_click: Option<nestix::Shared<dyn Fn()>>,
    realized: Option<RealizedButton>,
    click_handler: Rc<RefCell<Option<ClickHandlerState>>>,
}

#[derive(Debug, Clone)]
pub(crate) struct TextBlockElement {
    text: String,
    realized: Option<TextBlock>,
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

#[derive(Debug, Clone)]
pub(crate) struct XamlElement(Rc<XamlNode>);

impl PartialEq for XamlElement {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

impl Eq for XamlElement {}

impl XamlElement {
    pub fn window(title: String) -> Result<Self> {
        Ok(Self::new(XamlKind::Window(WindowElement {
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

    pub fn canvas() -> Result<Self> {
        Ok(Self::new(XamlKind::Canvas(CanvasElement {
            background_color: None,
            realized: None,
        })))
    }

    pub fn button(title: String) -> Result<Self> {
        Ok(Self::new(XamlKind::Button(ButtonElement {
            title,
            on_click: None,
            realized: None,
            click_handler: Rc::new(RefCell::new(None)),
        })))
    }

    pub fn text_block(text: String) -> Result<Self> {
        Ok(Self::new(XamlKind::TextBlock(TextBlockElement {
            text,
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
            XamlKind::Button(_) | XamlKind::TextBlock(_) => {}
        }
        Ok(())
    }

    pub fn set_text(&self, text: String) -> Result<()> {
        let text_value = HSTRING::from(text.clone());
        {
            match &mut *self.0.kind.borrow_mut() {
                XamlKind::Window(element) => {
                    element.title = text;
                    if let Some(window) = &element.realized {
                        window.SetTitle(&text_value)?;
                    }
                }
                XamlKind::Button(element) => {
                    element.title = text;
                    if let Some(realized) = &element.realized {
                        realized.label.SetText(&text_value)?;
                    }
                }
                XamlKind::TextBlock(element) => {
                    element.text = text;
                    if let Some(block) = &element.realized {
                        block.SetText(&text_value)?;
                    }
                }
                XamlKind::Canvas(_) => {}
            }
        }
        self.measure_intrinsic()?;
        Ok(())
    }

    pub fn set_window_size(&self, width: i32, height: i32) -> Result<()> {
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

    pub fn set_scale_factor_changed(
        &self,
        handler: Option<nestix::Shared<dyn Fn(f64)>>,
    ) -> Result<()> {
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

    pub fn set_resized(
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

    pub fn set_button_click(&self, handler: Option<nestix::Shared<dyn Fn()>>) -> Result<()> {
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

    pub fn set_background_color(&self, color: Option<nestix_native_core::Color>) -> Result<()> {
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
                    Width: (text_size.Width + padding.Left as f32 + padding.Right as f32)
                        .max(realized.control.MinWidth()? as f32),
                    Height: (text_size.Height + padding.Top as f32 + padding.Bottom as f32)
                        .max(realized.control.MinHeight()? as f32),
                }
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
            XamlKind::Button(element) => element.realize()?,
            XamlKind::TextBlock(element) => element.realize()?,
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
        Ok(())
    }

    fn new(kind: XamlKind) -> Self {
        Self(Rc::new(XamlNode {
            kind: RefCell::new(kind),
            children: RefCell::new(Vec::new()),
            layout: RefCell::new(None),
            measure_callback: RefCell::new(None),
        }))
    }

    fn is_realized(&self) -> bool {
        match &*self.0.kind.borrow() {
            XamlKind::Window(element) => element.realized.is_some(),
            XamlKind::Canvas(element) => element.realized.is_some(),
            XamlKind::Button(element) => element.realized.is_some(),
            XamlKind::TextBlock(element) => element.realized.is_some(),
        }
    }

    fn insert_realized_child(&self, child: &XamlElement, index: usize) -> Result<()> {
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
            XamlKind::Button(_) | XamlKind::TextBlock(_) => {}
        }
        Ok(())
    }

    fn as_ui_element(&self) -> Result<UIElement> {
        self.realize()?;
        match &*self.0.kind.borrow() {
            XamlKind::Window(_) => Err(Error::new(E_NOTIMPL, "Window is not a UIElement.")),
            XamlKind::Canvas(element) => element.realized.as_ref().unwrap().cast(),
            XamlKind::Button(element) => element.realized.as_ref().unwrap().control.cast(),
            XamlKind::TextBlock(element) => element.realized.as_ref().unwrap().cast(),
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

impl WindowElement {
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

impl CanvasElement {
    fn realize(&mut self) -> Result<()> {
        self.realized = Some(Canvas::new()?);
        Ok(())
    }
}

impl ButtonElement {
    fn realize(&mut self) -> Result<()> {
        let control = Button::new()?;
        let label = TextBlock::new()?;
        label.SetText(&HSTRING::from(&self.title))?;
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

impl TextBlockElement {
    fn realize(&mut self) -> Result<()> {
        let block = TextBlock::new()?;
        block.SetText(&HSTRING::from(&self.text))?;
        self.realized = Some(block);
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
    use super::{XamlElement, XamlKind};

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
    fn layout_is_cached_before_realization() {
        let element = XamlElement::canvas().unwrap();
        element.set_layout(1.0, 2.0, 30.0, 40.0).unwrap();

        let layout = element.0.layout.borrow().unwrap();
        assert_eq!((layout.x, layout.y), (1.0, 2.0));
        assert_eq!((layout.width, layout.height), (30.0, 40.0));
    }

    #[test]
    fn background_color_is_cached_before_realization() {
        let element = XamlElement::canvas().unwrap();
        element
            .set_background_color(Some(nestix_native_core::Color::RED))
            .unwrap();

        let kind = element.0.kind.borrow();
        let XamlKind::Canvas(canvas) = &*kind else {
            panic!("expected canvas");
        };
        assert_eq!(
            canvas.background_color,
            Some(nestix_native_core::Color::RED)
        );
    }
}
