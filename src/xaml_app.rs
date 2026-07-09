use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    sync::OnceLock,
};

use windows::Win32::{
    Foundation::RPC_E_CHANGED_MODE,
    System::Com::{COINIT_APARTMENTTHREADED, CoInitializeEx},
};
use windows_core::{Error, HRESULT, Interface, PCWSTR, Result};

use crate::{
    bindings::Microsoft::UI::Xaml::{
        Application, ApplicationInitializationCallback, Controls::XamlControlsResources,
    },
    xaml::XamlElement,
};

const WINDOWS_APP_SDK_RELEASE_MAJORMINOR: u32 = 0x0001_0008;
const WINDOWS_APP_SDK_MIN_VERSION: u64 = 0;
const MDDBOOTSTRAP_INITIALIZE_OPTIONS_NONE: u32 = 0;

thread_local! {
    static XAML_APPLICATION: RefCell<Option<crate::app_shim::CreatedXamlApplication>> = const { RefCell::new(None) };
    static PENDING_WINDOWS: RefCell<Vec<XamlElement>> = const { RefCell::new(Vec::new()) };
    static XAML_RUNNING: Cell<bool> = const { Cell::new(false) };
    static XAML_CONTROLS_RESOURCES_INSTALLED: Cell<bool> = const { Cell::new(false) };
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
    window_count: Rc<Cell<usize>>,
}

impl XamlApp {
    pub fn initialize() -> Result<Self> {
        initialize_windows_app_runtime()?;
        crate::window_native::set_process_dpi_awareness();

        unsafe {
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
            window_count: Rc::new(Cell::new(0)),
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
        self.window_count.set(0);
        clear_pending_windows();
        if let Ok(app) = Application::Current() {
            let _ = app.Exit();
        }
        XAML_APPLICATION.with_borrow_mut(|slot| {
            *slot = None;
        });
    }

    pub fn register_window(&self, window: XamlElement) -> XamlWindowRegistration {
        self.window_count.set(self.window_count.get() + 1);
        push_pending_window(window.clone());
        XamlWindowRegistration {
            inner: Rc::new(XamlWindowRegistrationInner {
                app: self.clone(),
                window,
                is_registered: Cell::new(true),
            }),
        }
    }

    fn unregister_window(&self, window: &XamlElement) {
        self.window_count
            .set(self.window_count.get().saturating_sub(1));
        remove_pending_window(window);
    }
}

#[derive(Clone)]
pub(crate) struct XamlWindowRegistration {
    inner: Rc<XamlWindowRegistrationInner>,
}

struct XamlWindowRegistrationInner {
    app: XamlApp,
    window: XamlElement,
    is_registered: Cell<bool>,
}

impl XamlWindowRegistration {
    pub fn unregister(&self) {
        if self.inner.is_registered.replace(false) {
            self.inner.app.unregister_window(&self.inner.window);
        }
    }
}

impl Drop for XamlWindowRegistrationInner {
    fn drop(&mut self) {
        if self.is_registered.replace(false) {
            self.app.unregister_window(&self.window);
        }
    }
}

pub(crate) fn is_xaml_running() -> bool {
    XAML_RUNNING.get()
}

fn push_pending_window(window: XamlElement) {
    PENDING_WINDOWS.with_borrow_mut(|windows| {
        if !windows.contains(&window) {
            windows.push(window);
        }
    });
}

fn remove_pending_window(window: &XamlElement) {
    PENDING_WINDOWS.with_borrow_mut(|windows| windows.retain(|item| item != window));
}

fn clear_pending_windows() {
    PENDING_WINDOWS.with_borrow_mut(Vec::clear);
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
            XamlControlsResources::new()?.cast()?;
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
