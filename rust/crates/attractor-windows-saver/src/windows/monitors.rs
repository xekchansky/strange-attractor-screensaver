//! Monitor enumeration for the multi-display session.
//!
//! One window and one player are created per monitor, the same convention the
//! macOS host follows with one `ScreensaverView` per screen. Rectangles are in
//! physical pixels because the shell is per-monitor DPI aware.

use std::mem::size_of;

use windows_sys::Win32::Foundation::{LPARAM, RECT, TRUE};
use windows_sys::Win32::Graphics::Gdi::{
    EnumDisplayMonitors, GetMonitorInfoW, HDC, HMONITOR, MONITORINFO,
};
use windows_sys::Win32::UI::WindowsAndMessaging::MONITORINFOF_PRIMARY;

/// One monitor's full rectangle, in physical pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MonitorRect {
    /// Left edge of the monitor in virtual-screen coordinates.
    pub left: i32,
    /// Top edge of the monitor in virtual-screen coordinates.
    pub top: i32,
    /// Width in pixels.
    pub width: i32,
    /// Height in pixels.
    pub height: i32,
    /// Whether Windows marks this monitor as the primary one.
    pub is_primary: bool,
}

/// Every monitor attached to the desktop, in the order Windows reports them.
pub fn all() -> Vec<MonitorRect> {
    let mut monitors: Vec<MonitorRect> = Vec::new();
    let data = &mut monitors as *mut Vec<MonitorRect> as LPARAM;
    unsafe {
        EnumDisplayMonitors(std::ptr::null_mut(), std::ptr::null(), Some(collect), data);
    }
    monitors
}

/// The `EnumDisplayMonitors` callback: record one monitor and keep going.
unsafe extern "system" fn collect(monitor: HMONITOR, _: HDC, _: *mut RECT, data: LPARAM) -> i32 {
    let mut info = MONITORINFO {
        cbSize: size_of::<MONITORINFO>() as u32,
        ..MONITORINFO::default()
    };
    if unsafe { GetMonitorInfoW(monitor, &mut info) } == 0 {
        return TRUE;
    }
    let rect = info.rcMonitor;
    // SAFETY: `all` passes a pointer to a live `Vec` and the enumeration is
    // synchronous.
    let monitors = unsafe { &mut *(data as *mut Vec<MonitorRect>) };
    monitors.push(MonitorRect {
        left: rect.left,
        top: rect.top,
        width: rect.right - rect.left,
        height: rect.bottom - rect.top,
        is_primary: info.dwFlags & MONITORINFOF_PRIMARY != 0,
    });
    TRUE
}
