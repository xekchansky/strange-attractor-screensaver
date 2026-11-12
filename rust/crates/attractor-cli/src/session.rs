//! The headless simulation loop: engine, camera, health monitor and ribbon.

use std::error::Error;
use std::fmt;

use attractor_core::formula::FormulaError;
use attractor_core::{
    AttractorEngine, AttractorModelError, AttractorSelection, IntegrationMethod, RandomSource,
    SimulationHealthIssue, SimulationHealthMonitor, SimulationStepScheduler, State,
};
use attractor_view::{CameraController, Mat4, OverlayModel, TrailBuilder, TrailGeometry};

/// A failure while building or advancing the headless simulation.
#[derive(Debug)]
pub enum SessionError {
    /// The selection's formulas cannot be bound to its parameters.
    Model(AttractorModelError),
    /// A formula failed to evaluate during a step.
    Integration(FormulaError),
    /// The health monitor rejected the run as degenerate.
    Unhealthy(SimulationHealthIssue),
}

impl fmt::Display for SessionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Model(error) => write!(formatter, "cannot build the attractor model: {error}"),
            Self::Integration(error) => write!(formatter, "simulation step failed: {error}"),
            Self::Unhealthy(issue) => {
                write!(formatter, "simulation health check failed: {issue:?}")
            }
        }
    }
}

impl Error for SessionError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Model(error) => Some(error),
            Self::Integration(error) => Some(error),
            Self::Unhealthy(_) => None,
        }
    }
}

/// Drives one simulation without a window: it steps the engine, orbits the
/// camera and rebuilds the ribbon.
///
/// A capture run advances with a fixed 1/60 s frame interval, the same
/// interval the Swift `CaptureController` pins, so captured motion is uniform
/// regardless of how long a frame takes to render.
#[derive(Debug)]
pub struct SimulationSession {
    selection: AttractorSelection,
    engine: AttractorEngine,
    camera: CameraController,
    scheduler: SimulationStepScheduler,
    monitor: SimulationHealthMonitor,
    builder: TrailBuilder,
    color_offset: f64,
    frames_simulated: usize,
}

impl SimulationSession {
    /// Build a session for `selection`.
    ///
    /// `camera_azimuth` and `camera_elevation` are in radians. `None` picks a
    /// seeded random azimuth, matching the screensaver; an explicit angle wins
    /// and consumes no draws from `rng`.
    pub fn new(
        selection: AttractorSelection,
        camera_azimuth: Option<f64>,
        camera_elevation: Option<f64>,
        rng: &mut impl RandomSource,
    ) -> Result<Self, SessionError> {
        let model = selection.build_model().map_err(SessionError::Model)?;
        let settings = selection.settings;
        let engine = AttractorEngine::new(
            model,
            selection.initial_state,
            IntegrationMethod::RungeKutta4,
            settings.step_size,
            usize::try_from(settings.trail_length.max(1)).unwrap_or(usize::MAX),
        );
        let camera = match camera_azimuth {
            Some(azimuth) => CameraController::new(
                azimuth,
                camera_elevation.unwrap_or(CameraController::DEFAULT_ELEVATION),
            ),
            None => {
                let random = CameraController::random(rng);
                match camera_elevation {
                    Some(elevation) => CameraController::new(random.azimuth(), elevation),
                    None => random,
                }
            }
        };
        Ok(Self {
            engine,
            camera,
            scheduler: SimulationStepScheduler::new(settings.step_size, settings.simulation_speed),
            monitor: SimulationHealthMonitor::default(),
            builder: TrailBuilder::new(),
            color_offset: 0.0,
            frames_simulated: 0,
            selection,
        })
    }

    /// Fast-forward the engine by `steps` integration steps.
    ///
    /// The health monitor watches the warm-up too: a degenerate start is
    /// usually periodic long before the first frame is rendered, and catching
    /// it here lets a capture script retry immediately.
    pub fn warm_up(&mut self, steps: usize) -> Result<(), SessionError> {
        for _ in 0..steps {
            let state = self.engine.advance().map_err(SessionError::Integration)?;
            if let Some(issue) = self.monitor.observe(state) {
                return Err(SessionError::Unhealthy(issue));
            }
        }
        Ok(())
    }

    /// Advance one rendered frame by `dt` seconds of simulated time.
    ///
    /// The simulation runs a whole number of fixed-size integration steps, the
    /// color gradient advances, the camera reframes and the ribbon is rebuilt.
    pub fn advance_frame(&mut self, dt: f64) -> Result<(), SessionError> {
        let steps = self.scheduler.steps_for_frame_interval(dt);
        for _ in 0..steps {
            let state = self.engine.advance().map_err(SessionError::Integration)?;
            if let Some(issue) = self.monitor.observe(state) {
                return Err(SessionError::Unhealthy(issue));
            }
        }

        self.color_offset =
            (self.color_offset + self.selection.settings.trail_color_speed * dt) % 1.0;
        self.camera.update(
            dt,
            self.selection.settings.camera_rotation_speed,
            self.engine.trace(),
        );
        self.builder.build(
            self.engine.trace(),
            self.color_offset,
            self.selection.settings.color_mode,
            self.camera.position(),
        );
        self.frames_simulated += 1;
        Ok(())
    }

    /// The selection this session runs.
    pub fn selection(&self) -> &AttractorSelection {
        &self.selection
    }

    /// The current state (the newest trace point).
    pub const fn state(&self) -> State {
        self.engine.state()
    }

    /// The ribbon built by the last [`Self::advance_frame`].
    pub fn geometry(&self) -> &TrailGeometry {
        self.builder.geometry()
    }

    /// Number of rendered frames advanced so far.
    pub const fn frames_simulated(&self) -> usize {
        self.frames_simulated
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
}
