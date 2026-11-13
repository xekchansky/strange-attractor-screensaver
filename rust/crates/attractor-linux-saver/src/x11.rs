//! The Xlib backend: xscreensaver's `-root` and `-window-id` protocol.
//!
//! xscreensaver starts a hack with either `-root`, meaning "draw on the
//! screen", or `-window-id N`, meaning "draw inside the fullscreen window I
//! mapped". Drawing straight into the root window is not reliable through
//! Vulkan, so `-root` creates one override-redirect window that covers the
//! whole screen and presents there; the visible result is the same and the
//! window disappears with the process.
//!
//! For `-window-id` the swap chain is created on xscreensaver's window, and
//! the session ends when that window is destroyed or unmapped, which is how
//! xscreensaver stops a hack. Input is read from the same window, so a key
//! press or a mouse button ends the session within one frame.

use std::ffi::{CString, c_char};
use std::ptr;
use std::thread;
use std::time::{Duration, Instant};

use attractor_gpu::{GpuContext, SurfaceTarget};
use attractor_shell::Settings;
use attractor_shell::input::{self, InputEvent};
use attractor_shell::selection;
use raw_window_handle::{RawDisplayHandle, RawWindowHandle, XlibDisplayHandle, XlibWindowHandle};
use x11_dl::xlib;

use crate::args::HackWindow;
use crate::error::ShellError;
use crate::session::{Display, time_seed};

/// Frame interval, the 60 Hz cadence the other shells use.
const FRAME_INTERVAL: Duration = Duration::from_millis(16);
/// Window title, also the name xscreensaver shows in its list.
const PRODUCT_NAME: &str = "Strange Attractor";
/// Events the session reacts to.
const EVENT_MASK: i64 = xlib::KeyPressMask
    | xlib::ButtonPressMask
    | xlib::PointerMotionMask
    | xlib::StructureNotifyMask
    | xlib::ExposureMask;

/// Run the xscreensaver hack in the requested window.
pub fn run_hack(
    window: HackWindow,
    settings: &Settings,
    frames: Option<u64>,
) -> Result<(), ShellError> {
    let connection = Connection::open()?;
    let (window_id, owns_window) = match window {
        HackWindow::Given(id) => (u64::from(id), false),
        HackWindow::Root => (create_window(&connection)?, true),
    };
    let geometry = window_geometry(&connection, window_id)?;

    let context = GpuContext::new()?;
    let catalog =
        selection::validated_catalog().map_err(|error| ShellError::new(error.to_string()))?;

    let mut window_handle = XlibWindowHandle::new(window_id);
    window_handle.visual_id = geometry.visual_id;
    let display_handle = XlibDisplayHandle::new(
        std::ptr::NonNull::new(connection.display.cast()),
        connection.screen,
    );
    // SAFETY: the connection keeps the X display alive for the whole session
    // and the window is either xscreensaver's or ours; both outlive the target.
    let target = unsafe {
        SurfaceTarget::from_raw_handles(
            &context,
            RawWindowHandle::Xlib(window_handle),
            Some(RawDisplayHandle::Xlib(display_handle)),
            geometry.width,
            geometry.height,
        )
    }?;

    let mut display = Display::new(
        target,
        geometry.width,
        geometry.height,
        1.0,
        settings,
        &catalog,
        time_seed(),
    )?;

    // The input mask is set on xscreensaver's window after its own mask; our
    // events are added, not replacing what the caller selected.
    unsafe {
        (connection.xlib.XSelectInput)(connection.display, window_id, EVENT_MASK);
    }
    let cursor = hide_cursor(&connection, window_id);

    run_frames(
        &connection,
        window_id,
        owns_window,
        frames,
        &context,
        &mut display,
    );

    restore_cursor(&connection, window_id, cursor);
    if owns_window {
        unsafe {
            (connection.xlib.XDestroyWindow)(connection.display, window_id);
            (connection.xlib.XFlush)(connection.display);
        }
    }
    Ok(())
}

/// Present frames until input, a destroyed host window or the frame limit.
fn run_frames(
    connection: &Connection,
    window_id: u64,
    owns_window: bool,
    frames: Option<u64>,
    context: &GpuContext,
    display: &mut Display,
) {
    let started = Instant::now();
    let mut last_frame = started;
    let mut rendered = 0u64;

    loop {
        let events = drain_events(connection, window_id, owns_window, started);
        if events.quit {
            break;
        }
        if let Some((width, height)) = events.resized {
            display.resize(width, height, 1.0);
        }

        let now = Instant::now();
        let dt = (now - last_frame).as_secs_f64();
        last_frame = now;
        display.draw(context, dt);
        unsafe {
            (connection.xlib.XFlush)(connection.display);
        }

        rendered += 1;
        if frames.is_some_and(|limit| rendered >= limit) {
            break;
        }
        thread::sleep(FRAME_INTERVAL);
    }
}

/// What a batch of X events asked the session to do.
#[derive(Debug, Default, PartialEq, Eq)]
struct EventOutcome {
    quit: bool,
    resized: Option<(u32, u32)>,
}

/// Read every queued X event and reduce it to the session's decisions.
fn drain_events(
    connection: &Connection,
    window_id: u64,
    owns_window: bool,
    started: Instant,
) -> EventOutcome {
    let mut outcome = EventOutcome::default();
    while unsafe { (connection.xlib.XPending)(connection.display) } > 0 {
        let mut event: xlib::XEvent = unsafe { std::mem::zeroed() };
        unsafe {
            (connection.xlib.XNextEvent)(connection.display, &mut event);
        }
        let kind = unsafe { event.type_ };
        match kind {
            xlib::KeyPress => outcome.quit = true,
            // Every mouse button, including the wheel's buttons 4–7.
            xlib::ButtonPress => outcome.quit = true,
            xlib::MotionNotify => {
                if input::ends_session(InputEvent::PointerMove, started.elapsed()) {
                    outcome.quit = true;
                }
            }
            xlib::ConfigureNotify => {
                let configure = unsafe { event.configure };
                if configure.window == window_id {
                    outcome.resized = Some((
                        configure.width.max(1) as u32,
                        configure.height.max(1) as u32,
                    ));
                }
            }
            xlib::DestroyNotify => {
                let destroy = unsafe { event.destroy_window };
                if destroy.window == window_id {
                    outcome.quit = true;
                }
            }
            xlib::UnmapNotify => {
                let unmap = unsafe { event.unmap };
                // xscreensaver unmaps its window to stop the hack; our own
                // window is only unmapped while it is being torn down.
                if !owns_window && unmap.window == window_id {
                    outcome.quit = true;
                }
            }
            _ => {}
        }
    }
    outcome
}

/// The size and visual id of an existing window.
struct WindowGeometry {
    width: u32,
    height: u32,
    visual_id: u64,
}

fn window_geometry(connection: &Connection, window: u64) -> Result<WindowGeometry, ShellError> {
    let mut attributes: xlib::XWindowAttributes = unsafe { std::mem::zeroed() };
    if unsafe {
        (connection.xlib.XGetWindowAttributes)(connection.display, window, &mut attributes)
    } == 0
    {
        return Err(ShellError::new("cannot read the X window attributes"));
    }
    let visual_id = if attributes.visual.is_null() {
        0
    } else {
        unsafe { (*attributes.visual).visualid }
    };
    Ok(WindowGeometry {
        width: attributes.width.max(1) as u32,
        height: attributes.height.max(1) as u32,
        visual_id,
    })
}

/// Create an override-redirect window that covers the whole screen.
fn create_window(connection: &Connection) -> Result<u64, ShellError> {
    let width = unsafe { (connection.xlib.XDisplayWidth)(connection.display, connection.screen) };
    let height = unsafe { (connection.xlib.XDisplayHeight)(connection.display, connection.screen) };
    let mut attributes: xlib::XSetWindowAttributes = unsafe { std::mem::zeroed() };
    attributes.override_redirect = 1;
    attributes.background_pixel = 0;
    attributes.event_mask = EVENT_MASK;
    let window = unsafe {
        (connection.xlib.XCreateWindow)(
            connection.display,
            connection.root(),
            0,
            0,
            width.max(1) as u32,
            height.max(1) as u32,
            0,
            xlib::CopyFromParent,
            xlib::InputOutput as u32,
            ptr::null_mut(),
            xlib::CWOverrideRedirect | xlib::CWBackPixel | xlib::CWEventMask,
            &mut attributes,
        )
    };
    if window == 0 {
        return Err(ShellError::new("cannot create the X11 saver window"));
    }
    let title = CString::new(PRODUCT_NAME).expect("the title has no interior NUL");
    unsafe {
        (connection.xlib.XStoreName)(connection.display, window, title.as_ptr());
        (connection.xlib.XMapWindow)(connection.display, window);
        (connection.xlib.XFlush)(connection.display);
    }
    Ok(window)
}

/// Install an invisible cursor; returns `None` when X11 refused.
fn hide_cursor(connection: &Connection, window: u64) -> Option<u64> {
    let data = [0u8; 1];
    let pixmap = unsafe {
        (connection.xlib.XCreateBitmapFromData)(
            connection.display,
            window,
            data.as_ptr().cast::<c_char>(),
            1,
            1,
        )
    };
    if pixmap == 0 {
        return None;
    }
    let mut black: xlib::XColor = unsafe { std::mem::zeroed() };
    let cursor = unsafe {
        (connection.xlib.XCreatePixmapCursor)(
            connection.display,
            pixmap,
            pixmap,
            &mut black,
            &mut black,
            0,
            0,
        )
    };
    unsafe {
        (connection.xlib.XFreePixmap)(connection.display, pixmap);
    }
    if cursor == 0 {
        return None;
    }
    unsafe {
        (connection.xlib.XDefineCursor)(connection.display, window, cursor);
    }
    Some(cursor)
}

/// Put the window's normal cursor back after the session.
fn restore_cursor(connection: &Connection, window: u64, cursor: Option<u64>) {
    if let Some(cursor) = cursor {
        unsafe {
            (connection.xlib.XUndefineCursor)(connection.display, window);
            (connection.xlib.XFreeCursor)(connection.display, cursor);
        }
    }
}

/// An open X display, closed when the session ends.
struct Connection {
    xlib: xlib::Xlib,
    display: *mut xlib::Display,
    screen: i32,
}

impl Connection {
    fn open() -> Result<Self, ShellError> {
        let xlib = xlib::Xlib::open()
            .map_err(|error| ShellError::new(format!("cannot load libX11: {error}")))?;
        let display = unsafe { (xlib.XOpenDisplay)(ptr::null()) };
        if display.is_null() {
            return Err(ShellError::new(
                "cannot open the X display; is DISPLAY set and is an X server running?",
            ));
        }
        let screen = unsafe { (xlib.XDefaultScreen)(display) };
        Ok(Self {
            xlib,
            display,
            screen,
        })
    }

    fn root(&self) -> u64 {
        unsafe { (self.xlib.XDefaultRootWindow)(self.display) }
    }
}

impl Drop for Connection {
    fn drop(&mut self) {
        unsafe {
            (self.xlib.XCloseDisplay)(self.display);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_quiet_batch_of_events_changes_nothing() {
        assert_eq!(
            EventOutcome::default(),
            EventOutcome {
                quit: false,
                resized: None
            }
        );
    }
}
