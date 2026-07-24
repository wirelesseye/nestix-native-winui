use std::{
    cell::RefCell,
    ffi::c_void,
    mem::size_of,
    ptr::null_mut,
    rc::{Rc, Weak},
    sync::{Once, OnceLock},
};

use nestix::{
    Element, PropValue, State, callback, closure, component, components::ContextProvider,
    create_state, layout, scoped_effect,
};
use nestix_native_core::{ImageSource, TrayIconError, TrayIconEvent, TrayIconProps};
use windows::Win32::{
    Foundation::{HWND, LPARAM, LRESULT, POINT, RECT, WPARAM},
    Graphics::GdiPlus::{
        GdipCreateHICONFromBitmap, GdipDisposeImage, GdipLoadImageFromFile,
        GdipLoadImageFromStream, GdiplusStartup, GdiplusStartupInput, GpImage, Ok as GDI_OK,
    },
    System::LibraryLoader::GetModuleHandleW,
    UI::{
        Shell::{
            NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE, NIM_MODIFY, NIM_SETVERSION,
            NIN_SELECT, NOTIFYICON_VERSION_4, NOTIFYICONDATAW, NOTIFYICONIDENTIFIER,
            SHCreateMemStream, Shell_NotifyIconGetRect, Shell_NotifyIconW,
        },
        WindowsAndMessaging::{
            CREATESTRUCTW, CreateWindowExW, DefWindowProcW, DestroyIcon, DestroyWindow,
            GWLP_USERDATA, GetCursorPos, GetWindowLongPtrW, HICON, RegisterClassW,
            RegisterWindowMessageW, SetWindowLongPtrW, WINDOW_EX_STYLE, WINDOW_STYLE, WM_APP,
            WM_CONTEXTMENU, WM_NCCREATE, WM_NCDESTROY, WNDCLASSW,
        },
    },
};
use windows_core::{HSTRING, PCWSTR, w};

use crate::menu::{MenuData, TrayMenuContext, show_tray_menu};

const CALLBACK_MESSAGE: u32 = WM_APP + 0x4e5;
const NIN_KEYSELECT: u32 = NIN_SELECT + 1;

fn process_name() -> String {
    std::env::current_exe()
        .ok()
        .and_then(|path| {
            path.file_stem()
                .map(|name| name.to_string_lossy().into_owned())
        })
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| "Application".to_string())
}

struct OwnedIcon(HICON);

impl Drop for OwnedIcon {
    fn drop(&mut self) {
        unsafe {
            let _ = DestroyIcon(self.0);
        }
    }
}

struct TrayIconState {
    hwnd: HWND,
    icon: Option<OwnedIcon>,
    registered: bool,
    desired_visible: bool,
    tooltip: Option<String>,
    menu: State<Option<Rc<MenuData>>>,
    on_activate: PropValue<Option<nestix::Shared<dyn Fn(TrayIconEvent)>>>,
    on_secondary: PropValue<Option<nestix::Shared<dyn Fn(TrayIconEvent)>>>,
}

impl TrayIconState {
    fn notify_data(&self) -> NOTIFYICONDATAW {
        let mut data = NOTIFYICONDATAW {
            cbSize: size_of::<NOTIFYICONDATAW>() as u32,
            hWnd: self.hwnd,
            uID: 1,
            uFlags: NIF_MESSAGE | NIF_ICON,
            uCallbackMessage: CALLBACK_MESSAGE,
            hIcon: self.icon.as_ref().map_or(HICON::default(), |icon| icon.0),
            ..Default::default()
        };
        if let Some(tooltip) = &self.tooltip {
            data.uFlags |= NIF_TIP;
            let encoded = tooltip.encode_utf16().take(data.szTip.len() - 1);
            for (target, value) in data.szTip.iter_mut().zip(encoded) {
                *target = value;
            }
        }
        data
    }

    fn sync(&mut self) {
        if !self.desired_visible || self.icon.is_none() {
            self.remove();
            return;
        }
        let mut data = self.notify_data();
        unsafe {
            if self.registered && !Shell_NotifyIconW(NIM_MODIFY, &data).as_bool() {
                self.registered = false;
            }
            if !self.registered && Shell_NotifyIconW(NIM_ADD, &data).as_bool() {
                data.Anonymous.uVersion = NOTIFYICON_VERSION_4;
                let _ = Shell_NotifyIconW(NIM_SETVERSION, &data);
                self.registered = true;
            }
        }
    }

    fn remove(&mut self) {
        if self.registered {
            let data = self.notify_data();
            unsafe {
                let _ = Shell_NotifyIconW(NIM_DELETE, &data);
            }
            self.registered = false;
        }
    }

    fn menu_point(&self) -> Option<POINT> {
        let identifier = NOTIFYICONIDENTIFIER {
            cbSize: size_of::<NOTIFYICONIDENTIFIER>() as u32,
            hWnd: self.hwnd,
            uID: 1,
            ..Default::default()
        };
        unsafe {
            if let Ok(RECT {
                left,
                right,
                bottom,
                ..
            }) = Shell_NotifyIconGetRect(&identifier)
            {
                return Some(POINT {
                    x: left + (right - left) / 2,
                    y: bottom,
                });
            }
            let mut point = POINT::default();
            GetCursorPos(&mut point).ok().map(|_| point)
        }
    }
}

fn show_menu(state: &Weak<RefCell<TrayIconState>>) -> Result<(), TrayIconError> {
    let state = state.upgrade().ok_or(TrayIconError::NotMounted)?;
    let (menu, hwnd, point) = {
        let state = state.borrow();
        if !state.registered {
            return Err(TrayIconError::NotMounted);
        }
        let menu = state.menu.get().ok_or(TrayIconError::MenuUnavailable)?;
        let point = state
            .menu_point()
            .ok_or(TrayIconError::PresentationFailed)?;
        (menu, state.hwnd, point)
    };

    show_tray_menu(&menu, hwnd, point)
        .then_some(())
        .ok_or(TrayIconError::PresentationFailed)
}

fn event_for(state: &Weak<RefCell<TrayIconState>>) -> TrayIconEvent {
    let state = state.clone();
    TrayIconEvent::new(callback!(move || show_menu(&state)))
}

fn class_name(instance: windows::Win32::Foundation::HMODULE) -> PCWSTR {
    const NAME: PCWSTR = w!("NestixNativeWinUiTrayIcon");
    static INIT: Once = Once::new();
    INIT.call_once(|| unsafe {
        RegisterClassW(&WNDCLASSW {
            hInstance: instance.into(),
            lpszClassName: NAME,
            lpfnWndProc: Some(window_proc),
            ..Default::default()
        });
    });
    NAME
}

fn taskbar_created_message() -> u32 {
    static MESSAGE: OnceLock<u32> = OnceLock::new();
    *MESSAGE.get_or_init(|| unsafe { RegisterWindowMessageW(w!("TaskbarCreated")) })
}

unsafe fn window_state(hwnd: HWND) -> Option<Rc<RefCell<TrayIconState>>> {
    let pointer =
        unsafe { GetWindowLongPtrW(hwnd, GWLP_USERDATA) } as *const RefCell<TrayIconState>;
    if pointer.is_null() {
        return None;
    }
    unsafe {
        Rc::increment_strong_count(pointer);
        Some(Rc::from_raw(pointer))
    }
}

extern "system" fn window_proc(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    unsafe {
        if message == WM_NCCREATE {
            let create = &*(lparam.0 as *const CREATESTRUCTW);
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, create.lpCreateParams as isize);
            return LRESULT(1);
        }
        if message == WM_NCDESTROY {
            let pointer = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const RefCell<TrayIconState>;
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
            if !pointer.is_null() {
                drop(Rc::from_raw(pointer));
            }
        } else if let Some(state) = window_state(hwnd) {
            if message == taskbar_created_message() {
                let mut state = state.borrow_mut();
                state.registered = false;
                state.sync();
                return LRESULT(0);
            }
            if message == CALLBACK_MESSAGE {
                let event_code = lparam.0 as u32 & 0xffff;
                let callback = {
                    let state = state.borrow();
                    if event_code == WM_CONTEXTMENU {
                        state.on_secondary.get()
                    } else if event_code == NIN_SELECT || event_code == NIN_KEYSELECT {
                        state.on_activate.get()
                    } else {
                        None
                    }
                };
                if let Some(callback) = callback {
                    callback(event_for(&Rc::downgrade(&state)));
                    return LRESULT(0);
                }
            }
        }
        DefWindowProcW(hwnd, message, wparam, lparam)
    }
}

fn ensure_gdiplus() {
    static TOKEN: OnceLock<usize> = OnceLock::new();
    TOKEN.get_or_init(|| unsafe {
        let mut token = 0;
        let input = GdiplusStartupInput {
            GdiplusVersion: 1,
            ..Default::default()
        };
        GdiplusStartup(&mut token, &input, null_mut());
        token
    });
}

fn load_icon(source: ImageSource) -> Option<HICON> {
    ensure_gdiplus();
    unsafe {
        let mut image: *mut GpImage = null_mut();
        let _stream = match source {
            ImageSource::File(path) => {
                GdipLoadImageFromFile(&HSTRING::from(path.as_os_str()), &mut image);
                None
            }
            ImageSource::Bytes(bytes) => {
                let stream = SHCreateMemStream(Some(&bytes));
                if let Some(stream) = &stream {
                    GdipLoadImageFromStream(stream, &mut image);
                }
                stream
            }
        };
        if image.is_null() {
            return None;
        }
        let mut icon = HICON::default();
        let status = GdipCreateHICONFromBitmap(image.cast(), &mut icon);
        GdipDisposeImage(image);
        (status == GDI_OK && !icon.is_invalid()).then_some(icon)
    }
}

#[component]
/// Adds an application icon to the Windows notification area.
pub fn TrayIcon(props: &TrayIconProps, element: &Element) -> Element {
    let menu = create_state(None::<Rc<MenuData>>);
    let state = Rc::new(RefCell::new(TrayIconState {
        hwnd: HWND::default(),
        icon: None,
        registered: false,
        desired_visible: false,
        tooltip: None,
        menu: menu.clone(),
        on_activate: props.on_activate.clone(),
        on_secondary: props.on_secondary.clone(),
    }));
    let instance = unsafe { GetModuleHandleW(None).unwrap() };
    let raw_state = Rc::into_raw(state.clone()) as *mut c_void;
    let hwnd = unsafe {
        CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            class_name(instance),
            w!(""),
            WINDOW_STYLE::default(),
            0,
            0,
            0,
            0,
            None,
            None,
            Some(instance.into()),
            Some(raw_state),
        )
        .unwrap()
    };
    state.borrow_mut().hwnd = hwnd;

    scoped_effect!(
        [state, props.icon, props.tooltip, props.visible] || {
            let decoded = load_icon(icon.get()).map(OwnedIcon);
            let mut state = state.borrow_mut();
            if let Some(icon) = decoded {
                state.icon = Some(icon);
            } else if state.icon.is_none() {
                eprintln!("nestix-native-winui: could not decode tray icon image");
            }
            state.tooltip = Some(tooltip.get().unwrap_or_else(process_name));
            state.desired_visible = visible.get();
            state.sync();
        }
    );

    element.on_unmount(closure!(
        [state] || {
            state.borrow_mut().remove();
            unsafe {
                let _ = DestroyWindow(state.borrow().hwnd);
            }
        }
    ));

    layout! {
        ContextProvider<TrayMenuContext>(TrayMenuContext { menu }) {
            $(props.menu.clone().map(|menu| nestix::Layout::from(menu.clone())))
        }
    }
}
