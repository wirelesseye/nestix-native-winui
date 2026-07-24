use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    sync::OnceLock,
};

use windows::Win32::{
    Foundation::RPC_E_CHANGED_MODE,
    System::{
        Com::{COINIT_APARTMENTTHREADED, CoInitializeEx},
        LibraryLoader::{GetProcAddress, LoadLibraryW},
    },
};
use windows_core::{Error, HRESULT, HSTRING, Interface, PCWSTR, Result, s, w};

use crate::{
    bindings::Microsoft::UI::Xaml::{
        Application, ApplicationInitializationCallback, Controls::XamlControlsResources, Thickness,
    },
    xaml::XamlElement,
};

use crate::bindings::Windows::Foundation::PropertyValue;
use windows_reference::IReference;

const WINDOWS_APP_SDK_RELEASE_MAJORMINOR: u32 = 0x0001_0008;
const WINDOWS_APP_SDK_MIN_VERSION: u64 = 0;
const MDDBOOTSTRAP_INITIALIZE_OPTIONS_NONE: u32 = 0;

thread_local! {
    static XAML_APPLICATION: RefCell<Option<crate::app_shim::CreatedXamlApplication>> = const { RefCell::new(None) };
    static PENDING_WINDOWS: RefCell<Vec<XamlElement>> = const { RefCell::new(Vec::new()) };
    static XAML_RUNNING: Cell<bool> = const { Cell::new(false) };
    static XAML_CONTROLS_RESOURCES_INSTALLED: Cell<bool> = const { Cell::new(false) };
}

type MddBootstrapInitialize2 = unsafe extern "system" fn(u32, PCWSTR, u64, u32) -> HRESULT;

#[derive(Clone)]
pub(crate) struct XamlApp {
    is_running: Rc<Cell<bool>>,
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
        clear_pending_windows();
        if let Ok(app) = Application::Current() {
            let _ = app.Exit();
        }
        XAML_APPLICATION.with_borrow_mut(|slot| {
            *slot = None;
        });
    }

    pub fn register_window(&self, window: XamlElement) -> XamlWindowRegistration {
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

pub(crate) fn theme_thickness(key: &str) -> Result<Thickness> {
    let resources = Application::Current()?.Resources()?;
    let key = PropertyValue::CreateString(&windows_core::HSTRING::from(key))?;
    resources
        .Lookup(&key)?
        .cast::<IReference<Thickness>>()?
        .Value()
}

pub(crate) fn theme_f64(key: &str) -> Result<f64> {
    let resources = Application::Current()?.Resources()?;
    let key = PropertyValue::CreateString(&windows_core::HSTRING::from(key))?;
    resources.Lookup(&key)?.cast::<IReference<f64>>()?.Value()
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

    let hr = *BOOTSTRAP_RESULT.get_or_init(initialize_windows_app_runtime_once);

    if hr.is_ok() {
        Ok(())
    } else {
        Err(Error::new(
            hr,
            "failed to initialize Windows App SDK runtime. Add nestix-native-winui-build to the application's build-dependencies and call nestix_native_winui_build::configure() from build.rs.",
        ))
    }
}

fn initialize_windows_app_runtime_once() -> HRESULT {
    let local_runtime = std::env::current_exe().ok().and_then(|executable| {
        executable
            .parent()
            .map(|parent| parent.join("Microsoft.WindowsAppRuntime.dll"))
    });

    if let Some(local_runtime) = local_runtime.filter(|path| path.is_file()) {
        let path = HSTRING::from(local_runtime.as_os_str());
        return match unsafe { LoadLibraryW(&path) } {
            Ok(_) => HRESULT::default(),
            Err(error) => error.code(),
        };
    }

    unsafe {
        let bootstrap = match LoadLibraryW(w!("Microsoft.WindowsAppRuntime.Bootstrap.dll")) {
            Ok(module) => module,
            Err(error) => return error.code(),
        };
        let Some(initialize) = GetProcAddress(bootstrap, s!("MddBootstrapInitialize2")) else {
            return HRESULT(0x8007_007F_u32 as i32);
        };
        let initialize: MddBootstrapInitialize2 = std::mem::transmute(initialize);
        initialize(
            WINDOWS_APP_SDK_RELEASE_MAJORMINOR,
            PCWSTR::null(),
            WINDOWS_APP_SDK_MIN_VERSION,
            MDDBOOTSTRAP_INITIALIZE_OPTIONS_NONE,
        )
    }
}
