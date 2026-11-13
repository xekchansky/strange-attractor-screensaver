//! The fullscreen runner, windowed by winit so it works on X11 and Wayland.
//!
//! Wayland has no screen saver API, so an idle hook (`swayidle`, `hypridle`)
//! or a `.desktop` launcher starts this mode. winit creates one borderless
//! fullscreen window per monitor, the player presents into each of them, and
//! any key, mouse button, wheel notch or pointer motion — after the same short
//! grace period the other shells use — ends the session.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use attractor_core::AttractorDescriptor;
use attractor_gpu::{GpuContext, SurfaceTarget};
use attractor_shell::Settings;
use attractor_shell::input::{self, InputEvent};
use attractor_shell::selection;
use raw_window_handle::{HasDisplayHandle, HasWindowHandle};
use winit::application::ApplicationHandler;
use winit::event::{ElementState, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::monitor::MonitorHandle;
use winit::window::{Fullscreen, Window, WindowId};

use crate::error::ShellError;
use crate::session::{Display, seed_for, time_seed};

/// Frame interval, the 60 Hz cadence the other shells use.
const FRAME_INTERVAL: Duration = Duration::from_millis(16);
/// Window title.
const PRODUCT_NAME: &str = "Strange Attractor";

/// Run the fullscreen saver until the user gives any input.
pub fn run(settings: &Settings, frames: Option<u64>) -> Result<(), ShellError> {
    let event_loop = EventLoop::new()
        .map_err(|error| ShellError::new(format!("cannot start a window event loop: {error}")))?;
    let mut runner = Runner::new(settings.clone(), frames)?;
    event_loop
        .run_app(&mut runner)
        .map_err(|error| ShellError::new(format!("the window event loop failed: {error}")))?;
    match runner.failure {
        Some(error) => Err(error),
        None => Ok(()),
    }
}

/// The event loop state: one window and one player per monitor.
struct Runner {
    settings: Settings,
    catalog: Vec<AttractorDescriptor>,
    context: Option<GpuContext>,
    windows: HashMap<WindowId, WindowState>,
    frames_left: Option<u64>,
    started: Instant,
    last_frame: Instant,
    failure: Option<ShellError>,
}

struct WindowState {
    window: Window,
    display: Display,
}

impl Runner {
    fn new(settings: Settings, frames: Option<u64>) -> Result<Self, ShellError> {
        let catalog =
            selection::validated_catalog().map_err(|error| ShellError::new(error.to_string()))?;
        Ok(Self {
            settings,
            catalog,
            context: None,
            windows: HashMap::new(),
            frames_left: frames,
            started: Instant::now(),
            last_frame: Instant::now(),
            failure: None,
        })
    }

    /// Record a failure and leave the event loop.
    fn fail(&mut self, event_loop: &ActiveEventLoop, error: ShellError) {
        log::error!("{error}");
        self.failure = Some(error);
        event_loop.exit();
    }

    /// Create the GPU context and one window per monitor.
    fn start(&mut self, event_loop: &ActiveEventLoop) {
        if !self.windows.is_empty() {
            // Back-to-back `resumed` events are allowed; the session is up.
            return;
        }
        let context = match GpuContext::new() {
            Ok(context) => context,
            Err(error) => {
                self.fail(event_loop, error.into());
                return;
            }
        };
        let monitors: Vec<MonitorHandle> = event_loop.available_monitors().collect();
        let seed = time_seed();

        let mut windows = Vec::with_capacity(monitors.len().max(1));
        if monitors.is_empty() {
            match create_window(event_loop, &context, None, self, seed, 0) {
                Ok(window) => windows.push(window),
                Err(error) => {
                    self.fail(event_loop, error);
                    return;
                }
            }
        } else {
            for (index, monitor) in monitors.into_iter().enumerate() {
                match create_window(event_loop, &context, Some(monitor), self, seed, index) {
                    Ok(window) => windows.push(window),
                    Err(error) => {
                        self.fail(event_loop, error);
                        return;
                    }
                }
            }
        }

        self.context = Some(context);
        for (id, window) in windows {
            self.windows.insert(id, window);
        }
        self.started = Instant::now();
        self.last_frame = self.started;
    }
}

/// Create one fullscreen window and attach a player to it.
fn create_window(
    event_loop: &ActiveEventLoop,
    context: &GpuContext,
    monitor: Option<MonitorHandle>,
    runner: &Runner,
    seed: u64,
    index: usize,
) -> Result<(WindowId, WindowState), ShellError> {
    let attributes = Window::default_attributes()
        .with_title(PRODUCT_NAME)
        .with_fullscreen(Some(Fullscreen::Borderless(monitor)));
    let window = event_loop
        .create_window(attributes)
        .map_err(|error| ShellError::new(format!("cannot create the saver window: {error}")))?;
    window.set_cursor_visible(false);

    let size = window.inner_size();
    let width = size.width.max(1);
    let height = size.height.max(1);
    let overlay_scale = window.scale_factor() as f32;

    let window_handle = window
        .window_handle()
        .map_err(|error| ShellError::new(format!("cannot read the window handle: {error}")))?
        .as_raw();
    let display_handle = window
        .display_handle()
        .map_err(|error| ShellError::new(format!("cannot read the display handle: {error}")))?
        .as_raw();

    // SAFETY: the window owns the handles and is kept in `WindowState` for as
    // long as the swap chain exists.
    let target = unsafe {
        SurfaceTarget::from_raw_handles(context, window_handle, Some(display_handle), width, height)
    }?;
    let display = Display::new(
        target,
        width,
        height,
        overlay_scale,
        &runner.settings,
        &runner.catalog,
        seed_for(seed, index),
    )?;

    let id = window.id();
    Ok((id, WindowState { window, display }))
}

impl ApplicationHandler for Runner {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        self.start(event_loop);
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested | WindowEvent::Destroyed => {
                self.windows.remove(&window_id);
                if self.windows.is_empty() {
                    event_loop.exit();
                }
            }
            WindowEvent::Resized(size) => {
                if let Some(state) = self.windows.get_mut(&window_id) {
                    let overlay_scale = state.window.scale_factor() as f32;
                    state
                        .display
                        .resize(size.width.max(1), size.height.max(1), overlay_scale);
                }
            }
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                if let Some(state) = self.windows.get_mut(&window_id) {
                    let size = state.window.inner_size();
                    state.display.resize(
                        size.width.max(1),
                        size.height.max(1),
                        scale_factor as f32,
                    );
                }
            }
            WindowEvent::KeyboardInput { event, .. }
                if event.state == ElementState::Pressed && !event.repeat =>
            {
                self.end_on(event_loop, InputEvent::Key);
            }
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                ..
            } => self.end_on(event_loop, InputEvent::PointerButton),
            WindowEvent::MouseWheel { .. } => self.end_on(event_loop, InputEvent::Wheel),
            WindowEvent::CursorMoved { .. } => self.end_on(event_loop, InputEvent::PointerMove),
            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        let Some(context) = self.context.as_ref() else {
            // `resumed` has not run yet; nothing to draw into.
            return;
        };
        if self.windows.is_empty() {
            event_loop.exit();
            return;
        }

        let now = Instant::now();
        let dt = (now - self.last_frame).as_secs_f64();
        self.last_frame = now;
        for state in self.windows.values_mut() {
            state.display.draw(context, dt);
        }

        if let Some(left) = self.frames_left.as_mut() {
            *left = left.saturating_sub(1);
            if *left == 0 {
                event_loop.exit();
                return;
            }
        }
        event_loop.set_control_flow(ControlFlow::WaitUntil(Instant::now() + FRAME_INTERVAL));
    }
}

impl Runner {
    /// End the session when `event` calls for it.
    fn end_on(&mut self, event_loop: &ActiveEventLoop, event: InputEvent) {
        // The grace period is measured from the first frame, not from the
        // process start: creating a surface and a player takes a moment.
        if input::ends_session(event, self.started.elapsed()) {
            event_loop.exit();
        }
    }
}
