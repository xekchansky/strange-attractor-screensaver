//! The per-surface runtime: simulation stepping, rendering and healing.

use std::fmt;

use attractor_core::{
    AttractorDescriptor, AttractorEngine, AttractorSelection, IntegrationMethod, RestartLimiter,
    SimulationHealthIssue, SimulationHealthMonitor, SimulationStepScheduler, SplitMix64, State,
    randomizer,
};
use attractor_gpu::{FrameRenderer, GpuContext, SAMPLE_COUNT, SurfaceTarget};
use attractor_view::{CameraController, Mat4, OverlayModel, TrailBuilder, TrailGeometry};

use crate::error::PlayerError;

/// Construction settings of one player.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlayerOptions {
    /// Orbit azimuth in radians; `None` draws a seeded random one.
    pub camera_azimuth: Option<f64>,
    /// Orbit elevation in radians; `None` keeps the drawn or default one.
    pub camera_elevation: Option<f64>,
    /// Replace a degenerate run with a fresh random selection.
    ///
    /// Captures keep this off so a failed run exits non-zero instead of
    /// silently swapping the attractor.
    pub heal_automatically: bool,
    /// Initial frame width in pixels.
    pub width: u32,
    /// Initial frame height in pixels.
    pub height: u32,
    /// Overlay font and margin multiplier.
    pub overlay_scale: f32,
    /// Multisample count of the render target.
    pub sample_count: u32,
}

impl Default for PlayerOptions {
    fn default() -> Self {
        Self {
            camera_azimuth: None,
            camera_elevation: None,
            heal_automatically: true,
            width: 1_200,
            height: 900,
            overlay_scale: 1.0,
            sample_count: SAMPLE_COUNT,
        }
    }
}

/// What one [`Player::tick`] did.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TickEvent {
    /// The simulation advanced and the frame was refreshed.
    Advanced,
    /// The health monitor rejected the run and a new selection replaced it.
    Restarted,
    /// The run is degenerate, but [`RestartLimiter`] blocks another restart.
    ///
    /// The frame is kept as-is; tick again after `retry_after_seconds`.
    RestartDeferred {
        /// Seconds until the oldest restart leaves the limiter's window.
        retry_after_seconds: f64,
    },
}

/// What an explicit [`Player::restart`] did.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RestartOutcome {
    /// A fresh random selection replaced the current run.
    Restarted,
    /// The restart limiter blocks restarting right now.
    Blocked {
        /// Seconds until the oldest restart leaves the limiter's window.
        retry_after_seconds: f64,
    },
}

/// Drives one simulation and renders its frames.
///
/// A live shell calls [`Player::tick`] with the wall-clock interval of the
/// frame and [`Player::render`] into its surface; a capture calls `tick` with
/// a fixed 1/60 s interval, the same interval the Swift `CaptureController`
/// pins, so the recorded motion is uniform regardless of render time.
pub struct Player {
    selection: AttractorSelection,
    engine: AttractorEngine,
    camera: CameraController,
    scheduler: SimulationStepScheduler,
    monitor: SimulationHealthMonitor,
    builder: TrailBuilder,
    catalog: Vec<AttractorDescriptor>,
    rng: SplitMix64,
    options: PlayerOptions,
    limiter: RestartLimiter,
    pending_restart: bool,
    color_offset: f64,
    frames_simulated: usize,
    elapsed: f64,
    width: u32,
    height: u32,
    overlay_scale: f32,
    sample_count: u32,
    renderer: Option<FrameRenderer>,
    surface_renderer: Option<FrameRenderer>,
}

impl Player {
    /// Build a player for `selection`.
    ///
    /// `catalog` is the validated descriptor list restarts draw from. `rng`
    /// seeds the camera and every later restart; a pinned
    /// [`PlayerOptions::camera_azimuth`] consumes no draw.
    pub fn new(
        selection: AttractorSelection,
        catalog: Vec<AttractorDescriptor>,
        options: PlayerOptions,
        mut rng: SplitMix64,
    ) -> Result<Self, PlayerError> {
        if options.width == 0 || options.height == 0 {
            return Err(PlayerError::Gpu(
                attractor_gpu::GpuError::InvalidFrameSize {
                    width: options.width,
                    height: options.height,
                },
            ));
        }
        let engine = engine_for(&selection)?;
        let camera = camera_for(&options, &mut rng);
        let scheduler = SimulationStepScheduler::new(
            selection.settings.step_size,
            selection.settings.simulation_speed,
        );
        Ok(Self {
            selection,
            engine,
            camera,
            scheduler,
            monitor: SimulationHealthMonitor::default(),
            builder: TrailBuilder::new(),
            catalog,
            rng,
            options,
            limiter: RestartLimiter::default(),
            pending_restart: false,
            color_offset: 0.0,
            frames_simulated: 0,
            elapsed: 0.0,
            width: options.width,
            height: options.height,
            overlay_scale: options.overlay_scale,
            sample_count: options.sample_count.max(1),
            renderer: None,
            surface_renderer: None,
        })
    }

    /// Fast-forward the engine by `steps` integration steps.
    ///
    /// The health monitor watches the warm-up too. A degenerate start is
    /// reported as an error regardless of [`PlayerOptions::heal_automatically`]:
    /// warm-up is a capture concern, where a failed run must exit non-zero.
    pub fn warm_up(&mut self, steps: usize) -> Result<(), PlayerError> {
        for _ in 0..steps {
            let state = self.engine.advance()?;
            if let Some(issue) = self.monitor.observe(state) {
                return Err(PlayerError::Unhealthy(issue));
            }
        }
        Ok(())
    }

    /// Advance one rendered frame by `dt` seconds of wall-clock time.
    ///
    /// The simulation runs a whole number of fixed-size integration steps, the
    /// color gradient advances, the camera reframes and the ribbon is rebuilt.
    /// A non-finite or non-positive interval still refreshes the frame without
    /// stepping, like a repeated presentation callback.
    ///
    /// With [`PlayerOptions::heal_automatically`] a health-monitor verdict
    /// replaces the selection at once when the limiter allows it, or defers
    /// the restart until `retry_after_seconds` have passed. Without healing
    /// the verdict is returned as [`PlayerError::Unhealthy`].
    pub fn tick(&mut self, dt: f64) -> Result<TickEvent, PlayerError> {
        // Sanitize once: a non-finite or negative interval comes from a clock
        // epoch reset, and passing it to the camera would poison its angles.
        // A finite but huge interval is a sleep/suspend jump; the Swift scene
        // clamps it to the scheduler's bound before it reaches the camera and
        // the color gradient, so the same frame is drawn after a wake-up.
        let dt = if dt.is_finite() && dt > 0.0 {
            dt.min(self.scheduler.maximum_frame_interval())
        } else {
            0.0
        };
        self.elapsed += dt;
        if self.pending_restart {
            return self.retry_restart(dt);
        }

        let steps = self.scheduler.steps_for_frame_interval(dt);
        for _ in 0..steps {
            let state = self.engine.advance()?;
            if let Some(issue) = self.monitor.observe(state) {
                return self.handle_unhealthy(issue, dt);
            }
        }

        self.refresh_view(dt);
        self.frames_simulated += 1;
        Ok(TickEvent::Advanced)
    }

    /// Replace the current run with a fresh random selection.
    ///
    /// The restart limiter caps how often this may happen, so a run that
    /// degenerates immediately does not rebuild on every frame.
    pub fn restart(&mut self) -> Result<RestartOutcome, PlayerError> {
        if self.limiter.allow_restart(self.elapsed) {
            self.pending_restart = false;
            self.replace_selection()?;
            self.refresh_view(0.0);
            Ok(RestartOutcome::Restarted)
        } else {
            Ok(RestartOutcome::Blocked {
                retry_after_seconds: self.retry_after(),
            })
        }
    }

    /// Retarget the player at a new frame size.
    ///
    /// The renderer is recreated on the next [`Player::render`], so a resize
    /// costs nothing until something is actually drawn.
    pub fn resize(
        &mut self,
        width: u32,
        height: u32,
        overlay_scale: f32,
    ) -> Result<(), PlayerError> {
        if width == 0 || height == 0 {
            return Err(PlayerError::Gpu(
                attractor_gpu::GpuError::InvalidFrameSize { width, height },
            ));
        }
        self.width = width;
        self.height = height;
        self.overlay_scale = overlay_scale;
        Ok(())
    }

    /// Render the current frame and return tightly packed RGBA8 rows.
    pub fn render(&mut self, context: &GpuContext) -> Result<Vec<u8>, PlayerError> {
        if self.renderer.is_none() {
            self.renderer = Some(FrameRenderer::with_sample_count(
                context,
                self.width,
                self.height,
                self.overlay_scale,
                self.sample_count,
            )?);
        }
        let renderer = self
            .renderer
            .as_mut()
            .expect("the renderer was just created");
        renderer.resize(context, self.width, self.height, self.overlay_scale)?;

        let aspect = f64::from(self.width) / f64::from(self.height);
        let view_projection = self.camera.view_projection(aspect);
        let overlay = OverlayModel::new(&self.selection, self.engine.state()).text();
        let pixels = renderer.render(
            context,
            self.builder.geometry(),
            &view_projection,
            Some(&overlay),
        )?;
        Ok(pixels)
    }

    /// Draw the current frame into a presentation surface and present it.
    ///
    /// The swap chain is retargeted to the player's frame size first, so a
    /// host may call [`Player::resize`] and this method on every display
    /// change. A frame the surface cannot hand out (a timeout or an occluded
    /// layer) is skipped and reported as success: there is nothing to draw.
    pub fn render_to_surface(
        &mut self,
        context: &GpuContext,
        target: &mut SurfaceTarget,
    ) -> Result<(), PlayerError> {
        target.resize(context, self.width, self.height)?;
        let Some(frame) = target.acquire(context)? else {
            return Ok(());
        };
        let format = target.format();
        if self
            .surface_renderer
            .as_ref()
            .is_none_or(|renderer| renderer.format() != format)
        {
            self.surface_renderer = Some(FrameRenderer::for_surface(
                context,
                format,
                self.width,
                self.height,
                self.overlay_scale,
                self.sample_count,
            )?);
        }
        let renderer = self
            .surface_renderer
            .as_mut()
            .expect("the surface renderer was just created");
        renderer.resize(context, self.width, self.height, self.overlay_scale)?;

        let aspect = f64::from(self.width) / f64::from(self.height);
        let view_projection = self.camera.view_projection(aspect);
        let overlay = OverlayModel::new(&self.selection, self.engine.state()).text();
        let view = frame.texture.create_view(&Default::default());
        renderer.render_to_view(
            context,
            &view,
            self.builder.geometry(),
            &view_projection,
            Some(&overlay),
        )?;
        context.queue().present(frame);
        Ok(())
    }

    /// The selection this player runs.
    pub const fn selection(&self) -> &AttractorSelection {
        &self.selection
    }

    /// The current state (the newest trace point).
    pub const fn state(&self) -> State {
        self.engine.state()
    }

    /// The ribbon built by the last tick or restart.
    pub fn geometry(&self) -> &TrailGeometry {
        self.builder.geometry()
    }

    /// Number of rendered frames advanced so far.
    pub const fn frames_simulated(&self) -> usize {
        self.frames_simulated
    }

    /// Wall-clock seconds the player has advanced through [`Player::tick`].
    pub const fn elapsed_seconds(&self) -> f64 {
        self.elapsed
    }

    /// Frame size in pixels.
    pub const fn size(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    /// Whether a degenerate run is replaced automatically.
    pub const fn heals_automatically(&self) -> bool {
        self.options.heal_automatically
    }

    /// The corner overlay for the current state.
    pub fn overlay(&self) -> OverlayModel {
        OverlayModel::new(&self.selection, self.engine.state())
    }

    /// Combined perspective and view transform for a frame aspect ratio.
    pub fn view_projection(&self, aspect: f64) -> Mat4 {
        self.camera.view_projection(aspect)
    }

    /// Current orbit azimuth in radians.
    pub fn camera_azimuth(&self) -> f64 {
        self.camera.azimuth()
    }

    /// Current orbit elevation in radians.
    pub fn camera_elevation(&self) -> f64 {
        self.camera.elevation()
    }

    fn handle_unhealthy(
        &mut self,
        issue: SimulationHealthIssue,
        dt: f64,
    ) -> Result<TickEvent, PlayerError> {
        if !self.options.heal_automatically {
            return Err(PlayerError::Unhealthy(issue));
        }
        if self.limiter.allow_restart(self.elapsed) {
            self.pending_restart = false;
            self.replace_selection()?;
            self.refresh_view(dt);
            self.frames_simulated += 1;
            Ok(TickEvent::Restarted)
        } else {
            self.pending_restart = true;
            Ok(TickEvent::RestartDeferred {
                retry_after_seconds: self.retry_after(),
            })
        }
    }

    fn retry_restart(&mut self, dt: f64) -> Result<TickEvent, PlayerError> {
        if self.limiter.allow_restart(self.elapsed) {
            self.pending_restart = false;
            self.replace_selection()?;
            self.refresh_view(dt);
            self.frames_simulated += 1;
            Ok(TickEvent::Restarted)
        } else {
            Ok(TickEvent::RestartDeferred {
                retry_after_seconds: self.retry_after(),
            })
        }
    }

    fn replace_selection(&mut self) -> Result<(), PlayerError> {
        let selection = randomizer::select_from_catalog(&self.catalog, &mut self.rng)?;
        let engine = engine_for(&selection)?;
        let camera = camera_for(&self.options, &mut self.rng);
        let scheduler = SimulationStepScheduler::new(
            selection.settings.step_size,
            selection.settings.simulation_speed,
        );
        self.selection = selection;
        self.engine = engine;
        self.camera = camera;
        self.scheduler = scheduler;
        self.monitor = SimulationHealthMonitor::default();
        self.color_offset = 0.0;
        Ok(())
    }

    fn refresh_view(&mut self, dt: f64) {
        let settings = self.selection.settings;
        self.color_offset = (self.color_offset + settings.trail_color_speed * dt) % 1.0;
        self.camera
            .update(dt, settings.camera_rotation_speed, self.engine.trace());
        self.builder.build(
            self.engine.trace(),
            self.color_offset,
            settings.color_mode,
            self.camera.position(),
        );
    }

    fn retry_after(&self) -> f64 {
        self.limiter
            .next_retry_time(self.elapsed)
            .map(|time| (time - self.elapsed).max(0.0))
            .unwrap_or_else(|| self.limiter.window())
    }
}

impl fmt::Debug for Player {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Player")
            .field("attractor", &self.selection.descriptor.id)
            .field("size", &(self.width, self.height))
            .field("frames_simulated", &self.frames_simulated)
            .field("elapsed", &self.elapsed)
            .finish_non_exhaustive()
    }
}

fn engine_for(selection: &AttractorSelection) -> Result<AttractorEngine, PlayerError> {
    let model = selection.build_model()?;
    Ok(AttractorEngine::new(
        model,
        selection.initial_state,
        IntegrationMethod::RungeKutta4,
        selection.settings.step_size,
        usize::try_from(selection.settings.trail_length.max(1)).unwrap_or(usize::MAX),
    ))
}

fn camera_for(options: &PlayerOptions, rng: &mut SplitMix64) -> CameraController {
    match options.camera_azimuth {
        Some(azimuth) => CameraController::new(
            azimuth,
            options
                .camera_elevation
                .unwrap_or(CameraController::DEFAULT_ELEVATION),
        ),
        None => {
            let random = CameraController::random(rng);
            match options.camera_elevation {
                Some(elevation) => CameraController::new(random.azimuth(), elevation),
                None => random,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_options_match_the_capture_surface() {
        let options = PlayerOptions::default();
        assert_eq!(options.width, 1_200);
        assert_eq!(options.height, 900);
        assert_eq!(options.overlay_scale, 1.0);
        assert_eq!(options.sample_count, SAMPLE_COUNT);
        assert!(options.heal_automatically);
        assert_eq!(options.camera_azimuth, None);
    }
}
