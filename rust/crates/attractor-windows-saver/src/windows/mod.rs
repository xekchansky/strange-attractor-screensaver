//! The Win32 shell: windows, the message loop and the per-monitor players.
//!
//! Everything in this module is a translation of Windows events into player
//! calls. The simulation, the camera, the ribbon and the overlay live in
//! `attractor-player`; the shell only creates a borderless window per monitor,
//! makes a swap chain from each `HWND`, ticks on a timer, and quits when the
//! user touches the keyboard or the mouse.

mod dialog;
mod monitors;
mod session;

use std::error::Error;
use std::fmt;

use windows_sys::Win32::Foundation::HINSTANCE;
use windows_sys::Win32::Graphics::Gdi::{COLOR_WINDOW, GetSysColorBrush, HBRUSH};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::HiDpi::{
    DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, SetProcessDpiAwarenessContext,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CS_HREDRAW, CS_VREDRAW, DispatchMessageW, GetMessageW, MB_ICONERROR, MB_OK, MSG, MessageBoxW,
    RegisterClassW, TranslateMessage, WNDCLASSW, WNDPROC,
};

use crate::text::wide;
use attractor_shell::Settings;

/// Window class of the fullscreen saver windows and the preview control.
const SESSION_CLASS: &str = "StrangeAttractorSaverWindow";
/// Window class of the `/c` settings window.
const DIALOG_CLASS: &str = "StrangeAttractorConfigWindow";
/// Title shown by the settings window and its message boxes.
const PRODUCT_NAME: &str = "Strange Attractor";

/// Drive the saver on every monitor until the user gives any input.
pub fn run_saver(settings: &Settings) -> Result<(), WindowsError> {
    session::run_fullscreen(settings)
}

/// Draw inside the control panel's preview control.
pub fn run_preview(
    parent: crate::args::WindowHandle,
    settings: &Settings,
) -> Result<(), WindowsError> {
    session::run_preview(parent, settings)
}

/// Open the settings window and save the chosen attractor.
pub fn run_configuration(owner: Option<crate::args::WindowHandle>) -> Result<(), WindowsError> {
    dialog::run(owner)
}

/// Show `message` in a modal error box.
///
/// A `.scr` started by Windows has no console, so a failure during startup is
/// reported this way; the caller also logs it for `--self-test` runs.
pub fn show_error(message: &str) {
    let text = wide(message);
    let title = wide(PRODUCT_NAME);
    unsafe {
        MessageBoxW(
            std::ptr::null_mut(),
            text.as_ptr(),
            title.as_ptr(),
            MB_OK | MB_ICONERROR,
        );
    }
}

/// A failure in the Win32 shell.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WindowsError {
    message: String,
}

impl WindowsError {
    /// Build an error with a human-readable message.
    pub(crate) fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }

    /// Build an error from the last Win32 error code.
    pub(crate) fn last(operation: &'static str) -> Self {
        let code = unsafe { windows_sys::Win32::Foundation::GetLastError() };
        Self::new(format!("{operation} failed (Win32 error {code})"))
    }
}

impl fmt::Display for WindowsError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Error for WindowsError {}

/// Make the process per-monitor DPI aware, so monitor rectangles and window
/// sizes are physical pixels.
///
/// The call fails on Windows versions before 10 1703 and when awareness was
/// already set by a manifest; both are harmless, so the result is ignored.
pub(crate) fn enable_dpi_awareness() {
    unsafe {
        SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
    }
}

/// The module handle used to register window classes and create windows.
pub(crate) fn module_instance() -> Result<HINSTANCE, WindowsError> {
    let instance = unsafe { GetModuleHandleW(std::ptr::null()) };
    if instance.is_null() {
        Err(WindowsError::last("GetModuleHandleW"))
    } else {
        Ok(instance)
    }
}

/// Register `name` with `procedure`, painting the background with
/// `background` (null leaves the window unpainted).
pub(crate) fn register_window_class(
    name: &str,
    procedure: WNDPROC,
    background: HBRUSH,
) -> Result<(), WindowsError> {
    let class_name = wide(name);
    let class = WNDCLASSW {
        style: CS_HREDRAW | CS_VREDRAW,
        lpfnWndProc: procedure,
        cbClsExtra: 0,
        cbWndExtra: 0,
        hInstance: module_instance()?,
        hIcon: std::ptr::null_mut(),
        hCursor: std::ptr::null_mut(),
        hbrBackground: background,
        lpszMenuName: std::ptr::null(),
        lpszClassName: class_name.as_ptr(),
    };
    let atom = unsafe { RegisterClassW(&class) };
    if atom == 0 {
        Err(WindowsError::last("RegisterClassW"))
    } else {
        Ok(())
    }
}

/// The dialog background brush.
pub(crate) fn dialog_background() -> HBRUSH {
    unsafe { GetSysColorBrush(COLOR_WINDOW) }
}

/// Run the message loop until a quit message arrives.
pub(crate) fn run_message_loop() {
    let mut message = MSG::default();
    loop {
        // A negative return value is an error; either way the loop is over.
        if unsafe { GetMessageW(&mut message, std::ptr::null_mut(), 0, 0) } <= 0 {
            break;
        }
        unsafe {
            TranslateMessage(&message);
            DispatchMessageW(&message);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_class_names_are_unique() {
        assert_ne!(SESSION_CLASS, DIALOG_CLASS);
    }

    #[test]
    fn the_module_instance_exists() {
        let instance = module_instance().expect("the executable module is loaded");
        assert!(!instance.is_null());
    }

    #[test]
    fn the_monitor_list_is_never_empty_on_a_desktop() {
        let monitors = monitors::all();
        assert!(
            !monitors.is_empty(),
            "a Windows session has at least one monitor"
        );
        for monitor in &monitors {
            assert!(monitor.width > 0);
            assert!(monitor.height > 0);
        }
        assert!(
            monitors.iter().any(|monitor| monitor.is_primary),
            "one monitor is the primary one"
        );
    }

    #[test]
    fn the_dialog_background_brush_comes_from_the_system_palette() {
        assert!(!dialog_background().is_null());
    }

    #[test]
    fn a_window_error_carries_its_message() {
        assert_eq!(
            WindowsError::new("something failed").to_string(),
            "something failed"
        );
        let last = WindowsError::last("Test");
        assert!(last.to_string().starts_with("Test failed (Win32 error "));
    }

    #[test]
    fn the_wide_constants_are_terminated() {
        assert_eq!(wide(PRODUCT_NAME).last(), Some(&0));
    }
}
