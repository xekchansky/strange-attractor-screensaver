//! The running saver: one borderless window and one player per monitor, plus
//! the preview session the control panel embeds.
//!
//! The shell is a thin translation layer on purpose. Windows events arrive in
//! [`window_procedure`], which renders on the process timer, ends the session
//! on input and resizes the preview; the simulation, camera, ribbon and
//! overlay stay inside `attractor-player`.

use std::cell::RefCell;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use attractor_core::{AttractorDescriptor, SplitMix64};
use attractor_gpu::{GpuContext, SurfaceTarget};
use attractor_player::{Player, PlayerOptions};
use windows_sys::Win32::Foundation::{FALSE, HWND, LPARAM, LRESULT, RECT, TRUE, WPARAM};
use windows_sys::Win32::UI::HiDpi::GetDpiForWindow;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::SetFocus;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, GetClientRect, IsWindow, KillTimer,
    PostQuitMessage, SetCursor, SetForegroundWindow, SetTimer, ShowCursor, WM_CLOSE, WM_DESTROY,
    WM_DISPLAYCHANGE, WM_KEYDOWN, WM_LBUTTONDOWN, WM_MBUTTONDOWN, WM_MOUSEMOVE, WM_MOUSEWHEEL,
    WM_RBUTTONDOWN, WM_SETCURSOR, WM_SIZE, WM_SYSKEYDOWN, WM_TIMER, WM_XBUTTONDOWN, WS_CHILD,
    WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_POPUP, WS_VISIBLE,
};

use super::{
    PRODUCT_NAME, SESSION_CLASS, WindowsError, enable_dpi_awareness, module_instance, monitors,
    register_window_class, run_message_loop,
};
use crate::args::WindowHandle;
use crate::input::{self, InputEvent};
use crate::selection;
use crate::settings::Settings;
use crate::text::wide;

/// Identifier of the one process timer that drives every display.
const TIMER_ID: usize = 1;
/// Frame interval in milliseconds, the 60 Hz cadence of the macOS host.
const FRAME_INTERVAL_MILLISECONDS: u32 = 16;

thread_local! {
    /// The running session. The window procedure and the entry point share it
    /// through a thread-local because Win32 callbacks cannot take a context
    /// pointer without keeping a raw pointer alive across the message loop.
    static SESSION: RefCell<Option<Session>> = const { RefCell::new(None) };
}

/// One display: its window, player and swap chain.
struct Display {
    hwnd: HWND,
    player: Player,
    target: SurfaceTarget,
    width: u32,
    height: u32,
    overlay_scale: f32,
    reporting_failure: bool,
}

/// Everything a running message loop owns.
struct Session {
    // Declared before the device so players and swap chains drop first.
    displays: Vec<Display>,
    context: GpuContext,
    started: Instant,
    last_frame: Instant,
    preview_parent: Option<HWND>,
}

/// Run the saver on every monitor until the user gives any input.
pub fn run_fullscreen(settings: &Settings) -> Result<(), WindowsError> {
    enable_dpi_awareness();
    let instance = module_instance()?;
    register_window_class(SESSION_CLASS, Some(window_procedure), std::ptr::null_mut())?;

    let monitors = monitors::all();
    if monitors.is_empty() {
        return Err(WindowsError::new("EnumDisplayMonitors found no monitor"));
    }

    let class = wide(SESSION_CLASS);
    let title = wide(PRODUCT_NAME);
    let mut windows: Vec<(HWND, u32, u32)> = Vec::with_capacity(monitors.len());
    for monitor in &monitors {
        let hwnd = unsafe {
            CreateWindowExW(
                WS_EX_TOPMOST | WS_EX_TOOLWINDOW,
                class.as_ptr(),
                title.as_ptr(),
                WS_POPUP | WS_VISIBLE,
                monitor.left,
                monitor.top,
                monitor.width,
                monitor.height,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                instance,
                std::ptr::null(),
            )
        };
        if hwnd.is_null() {
            destroy_windows(&windows);
            return Err(WindowsError::last("CreateWindowExW"));
        }
        windows.push((hwnd, monitor.width as u32, monitor.height as u32));
    }

    // The primary monitor's window takes the focus, so the first key press
    // reaches the saver instead of whatever had it before.
    if let Some(primary) = monitors.iter().position(|monitor| monitor.is_primary) {
        let hwnd = windows[primary].0;
        unsafe {
            SetForegroundWindow(hwnd);
            SetFocus(hwnd);
        }
    }

    unsafe {
        ShowCursor(FALSE);
    }

    let result = run_session(&windows, None, settings);

    // Tear down in the reverse order of creation: swap chains and players are
    // already gone with the session, the windows go last, the cursor returns
    // only after the saver is off the screen.
    destroy_windows(&windows);
    unsafe {
        ShowCursor(TRUE);
    }
    result
}

/// Draw inside the control panel's preview control.
pub fn run_preview(parent: WindowHandle, settings: &Settings) -> Result<(), WindowsError> {
    enable_dpi_awareness();
    let parent = raw_handle(parent);
    if unsafe { IsWindow(parent) } == 0 {
        return Err(WindowsError::new(
            "the preview parent window does not exist",
        ));
    }
    let instance = module_instance()?;
    register_window_class(SESSION_CLASS, Some(window_procedure), std::ptr::null_mut())?;

    let (width, height) = client_size(parent);
    if width == 0 || height == 0 {
        return Err(WindowsError::new("the preview area has no pixels"));
    }

    let class = wide(SESSION_CLASS);
    let title = wide(PRODUCT_NAME);
    let hwnd = unsafe {
        CreateWindowExW(
            0,
            class.as_ptr(),
            title.as_ptr(),
            WS_CHILD | WS_VISIBLE,
            0,
            0,
            width as i32,
            height as i32,
            parent,
            std::ptr::null_mut(),
            instance,
            std::ptr::null(),
        )
    };
    if hwnd.is_null() {
        return Err(WindowsError::last("CreateWindowExW"));
    }
    let created = [(hwnd, width, height)];
    let result = run_session(&created, Some(parent), settings);
    destroy_windows(&created);
    result
}

/// Build the session and run the message loop until a quit message arrives.
fn run_session(
    windows: &[(HWND, u32, u32)],
    preview_parent: Option<HWND>,
    settings: &Settings,
) -> Result<(), WindowsError> {
    let Some(timer_source) = windows.first().map(|(hwnd, _, _)| *hwnd) else {
        return Err(WindowsError::new("the session has no window"));
    };
    let context = GpuContext::new().map_err(|error| WindowsError::new(error.to_string()))?;
    let catalog =
        selection::validated_catalog().map_err(|error| WindowsError::new(error.to_string()))?;
    let seed = time_seed();

    let mut displays = Vec::with_capacity(windows.len());
    for (index, (hwnd, width, height)) in windows.iter().enumerate() {
        let display_seed = seed ^ (index as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15);
        displays.push(Display::new(
            &context,
            *hwnd,
            *width,
            *height,
            settings,
            &catalog,
            display_seed,
        )?);
    }

    let now = Instant::now();
    SESSION.with(|cell| {
        *cell.borrow_mut() = Some(Session {
            displays,
            context,
            started: now,
            last_frame: now,
            preview_parent,
        });
    });

    if unsafe { SetTimer(timer_source, TIMER_ID, FRAME_INTERVAL_MILLISECONDS, None) } == 0 {
        take_session();
        return Err(WindowsError::last("SetTimer"));
    }
    run_message_loop();
    unsafe {
        KillTimer(timer_source, TIMER_ID);
    }
    take_session();
    Ok(())
}

impl Display {
    /// Create the swap chain and the player for one window.
    fn new(
        context: &GpuContext,
        hwnd: HWND,
        width: u32,
        height: u32,
        settings: &Settings,
        catalog: &[AttractorDescriptor],
        seed: u64,
    ) -> Result<Self, WindowsError> {
        if width == 0 || height == 0 {
            return Err(WindowsError::new("a display has no pixels"));
        }
        let dpi = unsafe { GetDpiForWindow(hwnd) };
        let overlay_scale = if dpi == 0 { 1.0 } else { dpi as f32 / 96.0 };
        let instance = module_instance()?;
        let target =
            unsafe { SurfaceTarget::from_window_handle(context, hwnd, instance, width, height) }
                .map_err(|error| WindowsError::new(error.to_string()))?;

        let mut rng = SplitMix64::new(seed);
        let selection = selection::choose(settings, catalog, &mut rng)
            .map_err(|error| WindowsError::new(error.to_string()))?;
        let options = PlayerOptions {
            width,
            height,
            overlay_scale,
            ..PlayerOptions::default()
        };
        let player = Player::new(selection, catalog.to_vec(), options, rng)
            .map_err(|error| WindowsError::new(error.to_string()))?;

        Ok(Self {
            hwnd,
            player,
            target,
            width,
            height,
            overlay_scale,
            reporting_failure: false,
        })
    }

    /// Advance this display by `dt` seconds and present the frame.
    fn draw(&mut self, context: &GpuContext, dt: f64) {
        if self.width == 0 || self.height == 0 {
            return;
        }
        let result = match self.player.tick(dt) {
            Ok(_) => self.player.render_to_surface(context, &mut self.target),
            Err(error) => Err(error),
        };
        match result {
            Ok(()) => self.reporting_failure = false,
            Err(error) => {
                if !self.reporting_failure {
                    log::warn!("cannot draw the attractor: {error}");
                    self.reporting_failure = true;
                }
            }
        }
    }

    /// Retarget the player after a window resize.
    fn resize(&mut self, width: u32, height: u32) {
        self.width = width;
        self.height = height;
        if width == 0 || height == 0 {
            return;
        }
        if let Err(error) = self.player.resize(width, height, self.overlay_scale) {
            log::warn!("cannot resize the attractor player: {error}");
        }
    }
}

impl Session {
    /// Draw one frame on every display.
    fn render(&mut self) {
        if let Some(parent) = self.preview_parent
            && unsafe { IsWindow(parent) } == 0
        {
            // The control panel closed its preview without telling us.
            unsafe { PostQuitMessage(0) };
            return;
        }
        let now = Instant::now();
        let dt = (now - self.last_frame).as_secs_f64();
        self.last_frame = now;
        for display in &mut self.displays {
            display.draw(&self.context, dt);
        }
    }

    /// Apply a `WM_SIZE` to the matching display.
    fn resize(&mut self, hwnd: HWND, width: u32, height: u32) {
        if let Some(display) = self
            .displays
            .iter_mut()
            .find(|display| display.hwnd == hwnd)
        {
            display.resize(width, height);
        }
    }
}

/// The window procedure shared by the fullscreen and preview windows.
unsafe extern "system" fn window_procedure(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match message {
        WM_TIMER if wparam == TIMER_ID => {
            with_session(|session| session.render());
            0
        }
        WM_TIMER => 0,
        WM_SETCURSOR => {
            // The pointer is hidden for the whole session; a stray cursor would
            // otherwise blink on the first mouse move.
            unsafe { SetCursor(std::ptr::null_mut()) };
            TRUE as LRESULT
        }
        WM_KEYDOWN | WM_SYSKEYDOWN => input_message(InputEvent::Key, hwnd, message, wparam, lparam),
        WM_LBUTTONDOWN | WM_RBUTTONDOWN | WM_MBUTTONDOWN | WM_XBUTTONDOWN => {
            input_message(InputEvent::PointerButton, hwnd, message, wparam, lparam)
        }
        WM_MOUSEWHEEL => input_message(InputEvent::Wheel, hwnd, message, wparam, lparam),
        WM_MOUSEMOVE => input_message(InputEvent::PointerMove, hwnd, message, wparam, lparam),
        WM_SIZE => {
            with_session(|session| {
                session.resize(hwnd, size_width(lparam), size_height(lparam));
            });
            0
        }
        WM_DISPLAYCHANGE => {
            // Monitor rectangles and swap chains are stale after a topology
            // change. Ending the session is safer than presenting into a
            // surface whose monitor no longer exists.
            unsafe { PostQuitMessage(0) };
            0
        }
        WM_CLOSE => {
            unsafe { DestroyWindow(hwnd) };
            0
        }
        WM_DESTROY => {
            unsafe { PostQuitMessage(0) };
            0
        }
        _ => unsafe { DefWindowProcW(hwnd, message, wparam, lparam) },
    }
}

/// End the session for input that the saver reacts to.
///
/// In preview mode the messages go back to `DefWindowProcW`, so the control
/// panel keeps its keyboard navigation.
fn input_message(
    event: InputEvent,
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    let ends = SESSION.with(|cell| {
        cell.borrow().as_ref().is_some_and(|session| {
            session.preview_parent.is_none()
                && input::ends_session(event, session.started.elapsed())
        })
    });
    if ends {
        unsafe { PostQuitMessage(0) };
        0
    } else {
        unsafe { DefWindowProcW(hwnd, message, wparam, lparam) }
    }
}

/// Run `action` on the live session, if there is one.
///
/// Messages arrive before the session exists (window creation runs the window
/// procedure) and after it is gone (the shutdown path), so the absence of a
/// session is normal, not an error.
fn with_session(action: impl FnOnce(&mut Session)) {
    SESSION.with(|cell| {
        if let Some(session) = cell.borrow_mut().as_mut() {
            action(session);
        }
    });
}

/// Drop the session, releasing every player, swap chain and the device.
fn take_session() {
    SESSION.with(|cell| {
        *cell.borrow_mut() = None;
    });
}

/// Destroy windows that may already be gone; the call is harmless then.
fn destroy_windows(windows: &[(HWND, u32, u32)]) {
    unsafe {
        for (hwnd, _, _) in windows {
            DestroyWindow(*hwnd);
        }
    }
}

/// The client area of `hwnd` in physical pixels.
fn client_size(hwnd: HWND) -> (u32, u32) {
    let mut rect = RECT::default();
    if unsafe { GetClientRect(hwnd, &mut rect) } == 0 {
        return (0, 0);
    }
    (
        (rect.right - rect.left).max(0) as u32,
        (rect.bottom - rect.top).max(0) as u32,
    )
}

/// Low word of a `WM_SIZE` parameter.
fn size_width(lparam: LPARAM) -> u32 {
    (lparam as usize & 0xFFFF) as u32
}

/// High word of a `WM_SIZE` parameter.
fn size_height(lparam: LPARAM) -> u32 {
    ((lparam as usize >> 16) & 0xFFFF) as u32
}

/// Turn the opaque command-line handle into a Win32 `HWND`.
fn raw_handle(handle: WindowHandle) -> HWND {
    handle.value() as isize as HWND
}

/// A seed that differs between launches, so every start picks a new attractor.
fn time_seed() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_nanos() as u64)
        .unwrap_or(0x5EED)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_size_message_carries_width_and_height() {
        let width = 1280u32;
        let height = 720u32;
        let lparam = (width | (height << 16)) as LPARAM;
        assert_eq!(size_width(lparam), width);
        assert_eq!(size_height(lparam), height);
    }

    #[test]
    fn a_zero_size_message_is_not_confused_with_a_switch() {
        assert_eq!(size_width(0), 0);
        assert_eq!(size_height(0), 0);
    }

    #[test]
    fn command_line_handles_become_win32_handles() {
        assert_eq!(
            raw_handle(WindowHandle::from_value(0x1234)),
            0x1234isize as HWND
        );
    }
}
