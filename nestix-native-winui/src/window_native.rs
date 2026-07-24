use windows::Win32::{
    Foundation::HWND,
    UI::HiDpi::{
        DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, GetDpiForWindow, SetProcessDpiAwarenessContext,
    },
};
use windows_core::{Interface, Result};

use crate::bindings::Microsoft::UI::Xaml::Window;

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
    pub WindowHandle: unsafe extern "system" fn(
        *mut core::ffi::c_void,
        *mut *mut core::ffi::c_void,
    ) -> windows_core::HRESULT,
}

pub(crate) fn set_process_dpi_awareness() {
    unsafe {
        let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
    }
}

pub(crate) fn window_scale_factor(window: &Window) -> f64 {
    match window_hwnd(window) {
        Ok(hwnd) => hwnd_scale_factor(hwnd),
        Err(_) => 1.0,
    }
}

pub(crate) fn window_hwnd(window: &Window) -> Result<HWND> {
    let mut hwnd = HWND::default();
    let native = window.cast::<IWindowNative>()?;
    unsafe {
        native.WindowHandle(&mut hwnd)?;
    }
    Ok(hwnd)
}

pub(crate) fn hwnd_scale_factor(hwnd: HWND) -> f64 {
    if hwnd.is_invalid() {
        return 1.0;
    }
    let dpi = unsafe { GetDpiForWindow(hwnd) };
    if dpi == 0 { 1.0 } else { dpi as f64 / 96.0 }
}
