use std::{cell::RefCell, rc::Rc};

use windows_core::{Error, EventRevoker, HRESULT, HSTRING, Interface, Result};

use crate::{
    bindings::{
        Microsoft::UI::Xaml::{
            Controls::{Button, StackPanel, TextBlock},
            FrameworkElement, UIElement, Window,
        },
        Windows::Graphics::SizeInt32,
    },
    xaml_app::is_xaml_running,
    xaml_events::{RegisteredClickCallback, RegisteredScaleFactorCallback},
};

const E_NOTIMPL: HRESULT = HRESULT(0x80004001u32 as i32);

#[derive(Debug)]
pub(crate) struct XamlNode {
    kind: RefCell<XamlKind>,
    children: RefCell<Vec<XamlElement>>,
}

#[derive(Debug, Clone)]
pub(crate) enum XamlKind {
    Window(WindowElement),
    StackPanel(StackPanelElement),
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
}

#[derive(Debug, Clone)]
pub(crate) struct StackPanelElement {
    direction: nestix_native_core::FlexDirection,
    realized: Option<StackPanel>,
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

pub(crate) struct ClickHandlerState {
    callback: RegisteredClickCallback,
    _revoker: EventRevoker,
}

pub(crate) struct ScaleFactorHandlerState {
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
        })))
    }

    pub fn stack_panel() -> Result<Self> {
        Ok(Self::new(XamlKind::StackPanel(StackPanelElement {
            direction: nestix_native_core::FlexDirection::Column,
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
        if !self.0.children.borrow().contains(&child) {
            self.0.children.borrow_mut().push(child.clone());
        }

        if is_xaml_running() {
            self.realize()?;
            child.realize()?;
            self.append_realized_child(&child)?;
        }
        Ok(())
    }

    pub fn remove_child(&self, child: &XamlElement) -> Result<()> {
        self.0.children.borrow_mut().retain(|item| item != child);

        let panel = match &*self.0.kind.borrow() {
            XamlKind::StackPanel(element) => element.realized.clone(),
            _ => None,
        };
        let Some(panel) = panel else {
            return Ok(());
        };

        let child = child.as_ui_element()?;
        let children = panel.Children()?;
        let mut index = 0;
        if children.IndexOf(&child, &mut index)? {
            children.RemoveAt(index)?;
        }
        Ok(())
    }

    pub fn set_text(&self, text: String) -> Result<()> {
        let text_value = HSTRING::from(text.clone());
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
            XamlKind::StackPanel(_) => {}
        }
        Ok(())
    }

    pub fn set_size(&self, width: f64, height: f64) -> Result<()> {
        match &*self.0.kind.borrow() {
            XamlKind::Window(element) => {
                if let Some(window) = &element.realized
                    && let Ok(content) = window.Content()
                {
                    let content = content.cast::<FrameworkElement>()?;
                    content.SetWidth(width)?;
                    content.SetHeight(height)?;
                }
                Ok(())
            }
            XamlKind::StackPanel(element) => {
                if let Some(panel) = &element.realized {
                    panel.SetWidth(width)?;
                    panel.SetHeight(height)?;
                }
                Ok(())
            }
            XamlKind::Button(element) => {
                if let Some(realized) = &element.realized {
                    realized.control.SetWidth(width)?;
                    realized.control.SetHeight(height)?;
                }
                Ok(())
            }
            XamlKind::TextBlock(element) => {
                if let Some(block) = &element.realized {
                    block.SetWidth(width)?;
                    block.SetHeight(height)?;
                }
                Ok(())
            }
        }
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

    pub fn set_flex_direction(&self, direction: nestix_native_core::FlexDirection) -> Result<()> {
        let mut kind = self.0.kind.borrow_mut();
        let XamlKind::StackPanel(element) = &mut *kind else {
            return Ok(());
        };

        element.direction = direction;
        if let Some(panel) = &element.realized {
            panel.SetOrientation(orientation(direction))?;
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

    pub(crate) fn realize(&self) -> Result<()> {
        if self.is_realized() {
            return Ok(());
        }

        match &mut *self.0.kind.borrow_mut() {
            XamlKind::Window(element) => element.realize()?,
            XamlKind::StackPanel(element) => element.realize()?,
            XamlKind::Button(element) => element.realize()?,
            XamlKind::TextBlock(element) => element.realize()?,
        }

        let children = self.0.children.borrow().clone();
        for child in children {
            child.realize()?;
            self.append_realized_child(&child)?;
        }
        Ok(())
    }

    fn new(kind: XamlKind) -> Self {
        Self(Rc::new(XamlNode {
            kind: RefCell::new(kind),
            children: RefCell::new(Vec::new()),
        }))
    }

    fn is_realized(&self) -> bool {
        match &*self.0.kind.borrow() {
            XamlKind::Window(element) => element.realized.is_some(),
            XamlKind::StackPanel(element) => element.realized.is_some(),
            XamlKind::Button(element) => element.realized.is_some(),
            XamlKind::TextBlock(element) => element.realized.is_some(),
        }
    }

    fn append_realized_child(&self, child: &XamlElement) -> Result<()> {
        let child = child.as_ui_element()?;
        match &mut *self.0.kind.borrow_mut() {
            XamlKind::Window(element) => {
                if let Some(window) = element.realized.clone() {
                    window.SetContent(&child)?;
                    element.attach_scale_factor_handler(&window)?;
                }
            }
            XamlKind::StackPanel(element) => {
                if let Some(panel) = &element.realized {
                    let children = panel.Children()?;
                    let mut index = 0;
                    if !children.IndexOf(&child, &mut index)? {
                        children.Append(&child)?;
                    }
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
            XamlKind::StackPanel(element) => element.realized.as_ref().unwrap().cast(),
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
}

impl WindowElement {
    fn realize(&mut self) -> Result<()> {
        let window = Window::new()?;
        window.SetTitle(&HSTRING::from(&self.title))?;
        self.realized = Some(window);
        self.set_window_size()?;
        if let Some(window) = self.realized.clone() {
            self.attach_scale_factor_handler(&window)?;
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

        RegisteredScaleFactorCallback::invoke(
            callback_id,
            crate::window_native::window_scale_factor(window),
        );

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
}

impl StackPanelElement {
    fn realize(&mut self) -> Result<()> {
        let panel = StackPanel::new()?;
        panel.SetOrientation(orientation(self.direction))?;
        self.realized = Some(panel);
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

fn orientation(
    direction: nestix_native_core::FlexDirection,
) -> crate::bindings::Microsoft::UI::Xaml::Controls::Orientation {
    match direction {
        nestix_native_core::FlexDirection::Row | nestix_native_core::FlexDirection::RowReverse => {
            crate::bindings::Microsoft::UI::Xaml::Controls::Orientation::Horizontal
        }
        nestix_native_core::FlexDirection::Column
        | nestix_native_core::FlexDirection::ColumnReverse => {
            crate::bindings::Microsoft::UI::Xaml::Controls::Orientation::Vertical
        }
    }
}
