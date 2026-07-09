use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    rc::Rc,
    sync::OnceLock,
};

use windows::Win32::{
    Foundation::{HWND, RPC_E_CHANGED_MODE},
    System::Com::{COINIT_APARTMENTTHREADED, CoInitializeEx},
    UI::HiDpi::{
        DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, GetDpiForWindow, SetProcessDpiAwarenessContext,
    },
};
use windows_core::{Error, EventRevoker, HRESULT, HSTRING, Interface, PCWSTR, Result};

use crate::bindings::{
    Microsoft::UI::Xaml::{
        Application, ApplicationInitializationCallback,
        Controls::{Button, StackPanel, TextBlock},
        FrameworkElement, UIElement, Window,
    },
    Windows::Graphics::SizeInt32,
};

const E_NOTIMPL: HRESULT = HRESULT(0x80004001u32 as i32);
const WINDOWS_APP_SDK_RELEASE_MAJORMINOR: u32 = 0x0001_0008;
const WINDOWS_APP_SDK_MIN_VERSION: u64 = 0;
const MDDBOOTSTRAP_INITIALIZE_OPTIONS_NONE: u32 = 0;

windows_core::imp::define_interface!(
    IWindowNative,
    IWindowNative_Vtbl,
    0xeecdbf0e_bae9_4cb6_a68e_9598e1cb57bb
);
windows_core::imp::interface_hierarchy!(IWindowNative, windows_core::IUnknown);

impl IWindowNative {
    #[allow(non_snake_case)]
    unsafe fn WindowHandle(&self, hwnd: *mut HWND) -> Result<()> {
        unsafe {
            (windows_core::Interface::vtable(self).WindowHandle)(
                windows_core::Interface::as_raw(self),
                hwnd.cast(),
            )
            .ok()
        }
    }
}

#[repr(C)]
#[allow(non_snake_case)]
pub struct IWindowNative_Vtbl {
    pub base__: windows_core::IUnknown_Vtbl,
    pub WindowHandle:
        unsafe extern "system" fn(*mut core::ffi::c_void, *mut *mut core::ffi::c_void) -> HRESULT,
}

thread_local! {
    static XAML_APPLICATION: RefCell<Option<crate::app_shim::CreatedXamlApplication>> = const { RefCell::new(None) };
    static PENDING_WINDOWS: RefCell<Vec<XamlElement>> = const { RefCell::new(Vec::new()) };
    static XAML_RUNNING: Cell<bool> = const { Cell::new(false) };
    static XAML_CONTROLS_RESOURCES_INSTALLED: Cell<bool> = const { Cell::new(false) };
    static NEXT_CALLBACK_ID: Cell<u64> = const { Cell::new(1) };
    static CLICK_CALLBACKS: RefCell<HashMap<u64, nestix::Shared<dyn Fn()>>> = RefCell::new(HashMap::new());
    static SCALE_FACTOR_CALLBACKS: RefCell<HashMap<u64, nestix::Shared<dyn Fn(f64)>>> = RefCell::new(HashMap::new());
}

#[link(name = "Microsoft.WindowsAppRuntime.Bootstrap")]
unsafe extern "system" {
    fn MddBootstrapInitialize2(
        major_minor_version: u32,
        version_tag: PCWSTR,
        min_version: u64,
        options: u32,
    ) -> HRESULT;
}

#[derive(Clone)]
pub(crate) struct XamlApp {
    is_running: Rc<Cell<bool>>,
}

impl XamlApp {
    pub fn initialize() -> Result<Self> {
        initialize_windows_app_runtime()?;

        unsafe {
            let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
            let hr = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
            if hr == RPC_E_CHANGED_MODE {
                return Err(Error::new(
                    RPC_E_CHANGED_MODE,
                    "WinUI requires an STA thread; the current thread is already initialized differently.",
                ));
            }
            hr.ok()?;
        }

        Ok(Self {
            is_running: Rc::new(Cell::new(false)),
        })
    }

    pub fn run(&self) {
        if self.is_running.replace(true) {
            return;
        }

        let result = Application::Start(&ApplicationInitializationCallback::new(|_| {
            let created_app = crate::app_shim::create_xaml_application(Box::new(|| {
                XAML_RUNNING.set(true);
                install_xaml_controls_resources()?;
                realize_pending_windows()
            }))?;

            XAML_APPLICATION.with_borrow_mut(|slot| {
                *slot = Some(created_app);
            });
            Ok(())
        }));

        if let Err(error) = result {
            panic!("failed to start WinUI application: {error:?}");
        }
    }

    pub fn quit(&self) {
        self.is_running.set(false);
        if let Ok(app) = Application::Current() {
            let _ = app.Exit();
        }
        XAML_APPLICATION.with_borrow_mut(|slot| {
            *slot = None;
        });
    }
}

fn realize_pending_windows() -> Result<()> {
    PENDING_WINDOWS.with_borrow(|windows| -> Result<()> {
        for window in windows {
            window.realize()?;
            window.activate()?;
        }
        Ok(())
    })
}

fn install_xaml_controls_resources() -> Result<()> {
    XAML_CONTROLS_RESOURCES_INSTALLED.with(|installed| {
        if installed.get() {
            return Ok(());
        }

        let controls_resources: crate::bindings::Microsoft::UI::Xaml::ResourceDictionary =
            crate::bindings::Microsoft::UI::Xaml::Controls::XamlControlsResources::new()?.cast()?;
        let app = Application::Current()?;

        match app.Resources() {
            Ok(resources) => {
                resources
                    .MergedDictionaries()?
                    .Append(&controls_resources)?;
            }
            Err(_) => {
                app.SetResources(&controls_resources)?;
            }
        }

        installed.set(true);
        Ok(())
    })
}

fn initialize_windows_app_runtime() -> Result<()> {
    static BOOTSTRAP_RESULT: OnceLock<HRESULT> = OnceLock::new();

    let hr = *BOOTSTRAP_RESULT.get_or_init(|| unsafe {
        MddBootstrapInitialize2(
            WINDOWS_APP_SDK_RELEASE_MAJORMINOR,
            PCWSTR::null(),
            WINDOWS_APP_SDK_MIN_VERSION,
            MDDBOOTSTRAP_INITIALIZE_OPTIONS_NONE,
        )
    });

    if hr.is_ok() {
        Ok(())
    } else {
        Err(Error::new(
            hr,
            "failed to initialize Windows App SDK runtime. Install the Windows App Runtime 1.8 framework package, or use a self-contained deployment before creating WinUI controls.",
        ))
    }
}

#[derive(Debug)]
pub(crate) struct XamlNode {
    kind: RefCell<XamlKind>,
    realized: RefCell<Option<RealizedXamlKind>>,
    children: RefCell<Vec<XamlElement>>,
    click_handler: RefCell<Option<ClickHandlerState>>,
    scale_factor_callback_id: Cell<Option<u64>>,
    scale_factor_handler: RefCell<Option<ScaleFactorHandlerState>>,
}

#[derive(Debug, Clone)]
pub(crate) enum XamlKind {
    Window {
        title: String,
        width: i32,
        height: i32,
    },
    StackPanel {
        direction: nestix_native_core::FlexDirection,
    },
    Button {
        title: String,
        on_click: Option<nestix::Shared<dyn Fn()>>,
    },
    TextBlock {
        text: String,
    },
}

#[derive(Debug)]
pub(crate) enum RealizedXamlKind {
    Window(Window),
    StackPanel(StackPanel),
    Button { control: Button, label: TextBlock },
    TextBlock(TextBlock),
}

pub(crate) struct ClickHandlerState {
    callback_id: u64,
    revoker: EventRevoker,
}

pub(crate) struct ScaleFactorHandlerState {
    callback_id: u64,
    revoker: EventRevoker,
}

impl std::fmt::Debug for ClickHandlerState {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ClickHandlerState")
            .field("callback_id", &self.callback_id)
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
        let element = Self::new(XamlKind::Window {
            title,
            width: 200,
            height: 200,
        });
        PENDING_WINDOWS.with_borrow_mut(|windows| windows.push(element.clone()));
        Ok(element)
    }

    pub fn stack_panel() -> Result<Self> {
        Ok(Self::new(XamlKind::StackPanel {
            direction: nestix_native_core::FlexDirection::Column,
        }))
    }

    pub fn button(title: String) -> Result<Self> {
        Ok(Self::new(XamlKind::Button {
            title,
            on_click: None,
        }))
    }

    pub fn text_block(text: String) -> Result<Self> {
        Ok(Self::new(XamlKind::TextBlock { text }))
    }

    pub fn activate(&self) -> Result<()> {
        if !XAML_RUNNING.get() {
            return Ok(());
        }
        self.realize()?;
        if let Some(RealizedXamlKind::Window(window)) = &*self.0.realized.borrow() {
            window.Activate()
        } else {
            Ok(())
        }
    }

    pub fn append_child(&self, child: XamlElement) -> Result<()> {
        if !self.0.children.borrow().contains(&child) {
            self.0.children.borrow_mut().push(child.clone());
        }

        if XAML_RUNNING.get() {
            self.realize()?;
            child.realize()?;
            self.append_realized_child(&child)?;
        }
        Ok(())
    }

    pub fn remove_child(&self, child: &XamlElement) -> Result<()> {
        self.0.children.borrow_mut().retain(|item| item != child);

        let Some(RealizedXamlKind::StackPanel(panel)) = &*self.0.realized.borrow() else {
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
        match &mut *self.0.kind.borrow_mut() {
            XamlKind::Window { title, .. } | XamlKind::Button { title, .. } => {
                *title = text.clone()
            }
            XamlKind::TextBlock { text: value } => *value = text.clone(),
            XamlKind::StackPanel { .. } => {}
        }

        let text = HSTRING::from(text);
        match &*self.0.realized.borrow() {
            Some(RealizedXamlKind::Window(window)) => window.SetTitle(&text),
            Some(RealizedXamlKind::Button { label, .. })
            | Some(RealizedXamlKind::TextBlock(label)) => label.SetText(&text),
            Some(RealizedXamlKind::StackPanel(_)) | None => Ok(()),
        }
    }

    pub fn set_size(&self, width: f64, height: f64) -> Result<()> {
        match &*self.0.realized.borrow() {
            Some(RealizedXamlKind::Window(window)) => {
                if let Ok(content) = window.Content() {
                    let content = content.cast::<FrameworkElement>()?;
                    content.SetWidth(width)?;
                    content.SetHeight(height)?;
                }
                Ok(())
            }
            Some(RealizedXamlKind::StackPanel(panel)) => {
                panel.SetWidth(width)?;
                panel.SetHeight(height)
            }
            Some(RealizedXamlKind::Button { control, .. }) => {
                control.SetWidth(width)?;
                control.SetHeight(height)
            }
            Some(RealizedXamlKind::TextBlock(block)) => {
                block.SetWidth(width)?;
                block.SetHeight(height)
            }
            None => Ok(()),
        }
    }

    pub fn set_window_size(&self, width: i32, height: i32) -> Result<()> {
        if let XamlKind::Window {
            width: stored_width,
            height: stored_height,
            ..
        } = &mut *self.0.kind.borrow_mut()
        {
            *stored_width = width;
            *stored_height = height;
        }

        match &*self.0.realized.borrow() {
            Some(RealizedXamlKind::Window(window)) => window.AppWindow()?.ResizeClient(SizeInt32 {
                Width: width,
                Height: height,
            }),
            None => Ok(()),
            other => panic!("XamlElement is not a window: {:?}", other),
        }
    }

    pub fn set_scale_factor_changed(
        &self,
        handler: Option<nestix::Shared<dyn Fn(f64)>>,
    ) -> Result<()> {
        self.detach_scale_factor_handler();
        self.0
            .scale_factor_callback_id
            .set(handler.map(register_scale_factor_callback));

        if let Some(RealizedXamlKind::Window(window)) = &*self.0.realized.borrow() {
            self.attach_window_scale_factor_handler(window)?;
        }
        Ok(())
    }

    pub fn set_flex_direction(&self, direction: nestix_native_core::FlexDirection) -> Result<()> {
        if let XamlKind::StackPanel {
            direction: stored_direction,
        } = &mut *self.0.kind.borrow_mut()
        {
            *stored_direction = direction;
        }

        let Some(RealizedXamlKind::StackPanel(panel)) = &*self.0.realized.borrow() else {
            return Ok(());
        };
        panel.SetOrientation(orientation(direction))
    }

    pub fn set_button_click(&self, handler: Option<nestix::Shared<dyn Fn()>>) -> Result<()> {
        if let XamlKind::Button { on_click, .. } = &mut *self.0.kind.borrow_mut() {
            *on_click = handler.clone();
        }

        let Some(RealizedXamlKind::Button { control, .. }) = &*self.0.realized.borrow() else {
            return Ok(());
        };

        self.detach_click_handler(control)?;

        let Some(handler) = handler else {
            return Ok(());
        };

        let callback_id = register_click_callback(handler);
        let revoker = control.Click(move |_, _| {
            CLICK_CALLBACKS.with_borrow(|callbacks| {
                if let Some(callback) = callbacks.get(&callback_id) {
                    callback();
                }
            });
        })?;
        self.0.click_handler.replace(Some(ClickHandlerState {
            callback_id,
            revoker,
        }));

        Ok(())
    }

    fn new(kind: XamlKind) -> Self {
        Self(Rc::new(XamlNode {
            kind: RefCell::new(kind),
            realized: RefCell::new(None),
            children: RefCell::new(Vec::new()),
            click_handler: RefCell::new(None),
            scale_factor_callback_id: Cell::new(None),
            scale_factor_handler: RefCell::new(None),
        }))
    }

    fn realize(&self) -> Result<()> {
        if self.0.realized.borrow().is_some() {
            return Ok(());
        }

        let kind = self.0.kind.borrow().clone();
        let realized = match kind {
            XamlKind::Window {
                title,
                width,
                height,
            } => {
                let window = Window::new()?;
                window.SetTitle(&HSTRING::from(title))?;
                let realized = RealizedXamlKind::Window(window);
                self.0.realized.replace(Some(realized));
                for child in self.0.children.borrow().iter() {
                    child.realize()?;
                    self.append_realized_child(child)?;
                }
                self.set_window_size(width, height)?;
                return Ok(());
            }
            XamlKind::StackPanel { direction } => {
                let panel = StackPanel::new()?;
                panel.SetOrientation(orientation(direction))?;
                RealizedXamlKind::StackPanel(panel)
            }
            XamlKind::Button { title, on_click } => {
                let control = Button::new()?;
                let label = TextBlock::new()?;
                label.SetText(&HSTRING::from(&title))?;
                control.SetContent(&label)?;
                let realized = RealizedXamlKind::Button { control, label };
                self.0.realized.replace(Some(realized));
                self.set_button_click(on_click)?;
                return Ok(());
            }
            XamlKind::TextBlock { text } => {
                let block = TextBlock::new()?;
                block.SetText(&HSTRING::from(&text))?;
                RealizedXamlKind::TextBlock(block)
            }
        };

        self.0.realized.replace(Some(realized));
        for child in self.0.children.borrow().iter() {
            child.realize()?;
            self.append_realized_child(child)?;
        }
        Ok(())
    }

    fn append_realized_child(&self, child: &XamlElement) -> Result<()> {
        let child = child.as_ui_element()?;
        match &*self.0.realized.borrow() {
            Some(RealizedXamlKind::Window(window)) => {
                window.SetContent(&child)?;
                self.attach_window_scale_factor_handler(window)
            }
            Some(RealizedXamlKind::StackPanel(panel)) => {
                let children = panel.Children()?;
                let mut index = 0;
                if !children.IndexOf(&child, &mut index)? {
                    children.Append(&child)?;
                }
                Ok(())
            }
            Some(RealizedXamlKind::Button { .. }) | Some(RealizedXamlKind::TextBlock(_)) | None => {
                Ok(())
            }
        }
    }

    fn as_ui_element(&self) -> Result<UIElement> {
        self.realize()?;
        match &*self.0.realized.borrow() {
            Some(RealizedXamlKind::Window(_)) => {
                Err(Error::new(E_NOTIMPL, "Window is not a UIElement."))
            }
            Some(RealizedXamlKind::StackPanel(panel)) => panel.cast(),
            Some(RealizedXamlKind::Button { control, .. }) => control.cast(),
            Some(RealizedXamlKind::TextBlock(block)) => block.cast(),
            None => Err(Error::new(E_NOTIMPL, "XAML element was not realized.")),
        }
    }

    fn detach_click_handler(&self, _control: &Button) -> Result<()> {
        let Some(handler) = self.0.click_handler.take() else {
            return Ok(());
        };

        drop(handler.revoker);
        CLICK_CALLBACKS.with_borrow_mut(|callbacks| {
            callbacks.remove(&handler.callback_id);
        });
        Ok(())
    }

    fn attach_window_scale_factor_handler(&self, window: &Window) -> Result<()> {
        let Some(callback_id) = self.0.scale_factor_callback_id.get() else {
            self.0.scale_factor_handler.take();
            return Ok(());
        };

        self.0.scale_factor_handler.take();
        invoke_scale_factor_callback(callback_id, window_scale_factor(window));

        let content = match window.Content() {
            Ok(content) => content.cast::<FrameworkElement>()?,
            Err(_) => return Ok(()),
        };
        let hwnd = window_hwnd(window)?;
        let hwnd_value = hwnd.0 as isize;
        let revoker = content.SizeChanged(move |_, _| {
            invoke_scale_factor_callback(callback_id, hwnd_scale_factor(HWND(hwnd_value as _)));
        })?;
        self.0
            .scale_factor_handler
            .replace(Some(ScaleFactorHandlerState {
                callback_id,
                revoker,
            }));
        Ok(())
    }

    fn detach_scale_factor_handler(&self) {
        if let Some(handler) = self.0.scale_factor_handler.take() {
            drop(handler.revoker);
        }
        if let Some(callback_id) = self.0.scale_factor_callback_id.take() {
            SCALE_FACTOR_CALLBACKS.with_borrow_mut(|callbacks| {
                callbacks.remove(&callback_id);
            });
        }
    }
}

impl Drop for XamlNode {
    fn drop(&mut self) {
        if let Some(handler) = self.scale_factor_handler.take() {
            drop(handler.revoker);
            SCALE_FACTOR_CALLBACKS.with_borrow_mut(|callbacks| {
                callbacks.remove(&handler.callback_id);
            });
        } else if let Some(callback_id) = self.scale_factor_callback_id.take() {
            SCALE_FACTOR_CALLBACKS.with_borrow_mut(|callbacks| {
                callbacks.remove(&callback_id);
            });
        }

        if let Some(handler) = self.click_handler.take() {
            drop(handler.revoker);
            CLICK_CALLBACKS.with_borrow_mut(|callbacks| {
                callbacks.remove(&handler.callback_id);
            });
        }
    }
}

fn register_click_callback(callback: nestix::Shared<dyn Fn()>) -> u64 {
    NEXT_CALLBACK_ID.with(|next_id| {
        let id = next_id.get();
        next_id.set(id.wrapping_add(1).max(1));
        CLICK_CALLBACKS.with_borrow_mut(|callbacks| {
            callbacks.insert(id, callback);
        });
        id
    })
}

fn register_scale_factor_callback(callback: nestix::Shared<dyn Fn(f64)>) -> u64 {
    NEXT_CALLBACK_ID.with(|next_id| {
        let id = next_id.get();
        next_id.set(id.wrapping_add(1).max(1));
        SCALE_FACTOR_CALLBACKS.with_borrow_mut(|callbacks| {
            callbacks.insert(id, callback);
        });
        id
    })
}

fn invoke_scale_factor_callback(callback_id: u64, scale_factor: f64) {
    SCALE_FACTOR_CALLBACKS.with_borrow(|callbacks| {
        if let Some(callback) = callbacks.get(&callback_id) {
            callback(scale_factor);
        }
    });
}

fn window_scale_factor(window: &Window) -> f64 {
    match window_hwnd(window) {
        Ok(hwnd) => hwnd_scale_factor(hwnd),
        Err(_) => 1.0,
    }
}

fn window_hwnd(window: &Window) -> Result<HWND> {
    let mut hwnd = HWND::default();
    let native = window.cast::<IWindowNative>()?;
    unsafe {
        native.WindowHandle(&mut hwnd)?;
    }
    Ok(hwnd)
}

fn hwnd_scale_factor(hwnd: HWND) -> f64 {
    if hwnd.is_invalid() {
        return 1.0;
    }
    let dpi = unsafe { GetDpiForWindow(hwnd) };
    if dpi == 0 { 1.0 } else { dpi as f64 / 96.0 }
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
