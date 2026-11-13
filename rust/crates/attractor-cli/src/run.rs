//! From parsed flags to a rendered capture.

use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::fmt;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use attractor_core::catalog::{self, CatalogError};
use attractor_core::{
    AttractorSelection, DebugConfig, DebugConfigError, RandomizerError, SimulationSettings,
    SplitMix64, randomizer,
};
use attractor_gpu::{GpuContext, GpuError, lit_pixels, write_png};
use attractor_player::{Player, PlayerError, PlayerOptions};

use crate::arguments::{CaptureRequest, CommandLineArguments};

/// The fixed simulated interval of one capture frame, matching the Swift
/// `CaptureController`.
const FRAME_INTERVAL: f64 = 1.0 / 60.0;
/// Default number of integration steps fast-forwarded before a capture.
const DEFAULT_WARMUP_STEPS: usize = 60_000;
/// Default number of simulated frames before a screenshot: the Swift capture
/// lets the camera framing settle for 90 ticks first.
pub const DEFAULT_SETTLE_FRAMES: usize = 90;
/// Default logical capture size in points (the Swift demo's window).
pub const DEFAULT_WIDTH: u32 = 1_200;
/// Default logical capture height in points.
pub const DEFAULT_HEIGHT: u32 = 900;
/// Largest final pixel dimension, so a mistyped scale cannot request a
/// multi-gigabyte readback.
pub const MAXIMUM_PIXEL_DIMENSION: u32 = 8_192;

/// A run resolved against the catalog and ready to simulate.
#[derive(Debug)]
pub struct PreparedRun {
    /// The player, not yet warmed up.
    pub player: Player,
    /// What to capture.
    pub capture: CaptureRequest,
    /// Integration steps to fast-forward before the first frame.
    pub warmup_steps: usize,
    /// Logical capture width in points.
    pub width: u32,
    /// Logical capture height in points.
    pub height: u32,
    /// Resolution multiplier applied to both dimensions.
    pub capture_scale: u32,
    /// Frames to simulate before a screenshot, or to record.
    pub frames: Option<usize>,
}

impl PreparedRun {
    /// Final capture size in pixels.
    pub const fn pixel_size(&self) -> (u32, u32) {
        (
            self.width.saturating_mul(self.capture_scale),
            self.height.saturating_mul(self.capture_scale),
        )
    }
}

/// What a successful run wrote.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaptureReport {
    /// Rendered frames simulated (including the settle phase).
    pub frames_simulated: usize,
    /// PNG files written.
    pub frames_written: usize,
    /// Final frame width in pixels.
    pub width: u32,
    /// Final frame height in pixels.
    pub height: u32,
    /// Pixels that differ from the black background in the last written frame.
    pub lit_pixels: usize,
    /// Files written, in capture order.
    pub outputs: Vec<PathBuf>,
}

/// A failure while preparing or executing a capture.
#[derive(Debug)]
pub enum RunError {
    /// The bundled catalog does not decode.
    Catalog(CatalogError),
    /// Every catalog entry was dropped by validation.
    NoRunnableAttractor,
    /// `--attractor` names an id the catalog does not contain.
    UnknownAttractor(String),
    /// Parameters were supplied without `--attractor`.
    AttractorRequiredForParameters,
    /// `--param` names a parameter the attractor does not declare.
    UnknownParameter {
        /// The parameter name.
        name: String,
        /// The attractor id.
        attractor: String,
        /// Comma-separated declared names.
        available: String,
    },
    /// `--param` names a declared parameter with a non-finite value.
    InvalidParameterValue {
        /// The parameter name.
        name: String,
        /// The raw value.
        value: String,
        /// Comma-separated declared names.
        available: String,
    },
    /// The manual selection could not be built.
    DebugConfig(DebugConfigError),
    /// The random selection failed.
    Randomizer(RandomizerError),
    /// Hyperparameter overrides are nonsensical.
    Settings(Vec<String>),
    /// A simulation step, health check or model build failed.
    Player(PlayerError),
    /// No `--screenshot` or `--record` was requested.
    CaptureMissing,
    /// The requested pixel size is too large.
    InvalidCaptureSize {
        /// Final width in pixels.
        width: u32,
        /// Final height in pixels.
        height: u32,
    },
    /// The GPU device or renderer failed.
    Gpu(GpuError),
    /// A capture file could not be created.
    Io(std::io::Error),
}

impl RunError {
    /// Process exit status this failure maps to.
    ///
    /// Usage errors use 2, like the Swift demo and the usual shell convention;
    /// runtime failures use 1 so `docs/capture_all.sh` retries a degenerate
    /// run.
    pub fn exit_code(&self) -> u8 {
        match self {
            Self::UnknownAttractor(_)
            | Self::AttractorRequiredForParameters
            | Self::UnknownParameter { .. }
            | Self::InvalidParameterValue { .. }
            | Self::Settings(_)
            | Self::CaptureMissing
            | Self::InvalidCaptureSize { .. } => 2,
            _ => 1,
        }
    }
}

impl fmt::Display for RunError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Catalog(error) => write!(formatter, "failed to load attractor catalog: {error}"),
            Self::NoRunnableAttractor => {
                write!(formatter, "the attractor catalog has no runnable entries")
            }
            Self::UnknownAttractor(id) => write!(formatter, "unknown attractor '{id}'"),
            Self::AttractorRequiredForParameters => {
                write!(
                    formatter,
                    "specify an attractor with -a <id> to pass parameters"
                )
            }
            Self::UnknownParameter {
                name,
                attractor,
                available,
            } => write!(
                formatter,
                "unknown parameter '{name}' for '{attractor}'. Available: {available}"
            ),
            Self::InvalidParameterValue {
                name,
                value,
                available,
            } => write!(
                formatter,
                "invalid value '{value}' for parameter '{name}'. Available: {available}"
            ),
            Self::DebugConfig(error) => write!(formatter, "failed to build simulation: {error}"),
            Self::Randomizer(error) => write!(formatter, "failed to build simulation: {error}"),
            Self::Settings(issues) => write!(formatter, "{}", issues.join("\n")),
            Self::Player(error) => write!(formatter, "{error}"),
            Self::CaptureMissing => write!(
                formatter,
                "no capture requested: pass --screenshot <path> or --record <dir>"
            ),
            Self::InvalidCaptureSize { width, height } => write!(
                formatter,
                "capture size {width}x{height} exceeds the {MAXIMUM_PIXEL_DIMENSION} pixel limit"
            ),
            Self::Gpu(error) => write!(formatter, "{error}"),
            Self::Io(error) => write!(formatter, "cannot write the capture: {error}"),
        }
    }
}

impl Error for RunError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Catalog(error) => Some(error),
            Self::DebugConfig(error) => Some(error),
            Self::Randomizer(error) => Some(error),
            Self::Player(error) => Some(error),
            Self::Gpu(error) => Some(error),
            Self::Io(error) => Some(error),
            _ => None,
        }
    }
}

/// Resolve flags against the catalog: validate parameters, build the seeded
/// selection, apply overrides and construct the session.
pub fn prepare(arguments: &CommandLineArguments) -> Result<PreparedRun, RunError> {
    let validation = catalog::bundled_validation().map_err(RunError::Catalog)?;
    for issue in &validation.issues {
        log::warn!(
            "dropping attractor '{}' ({}): {}",
            issue.descriptor_id,
            issue.field,
            issue.message
        );
    }
    let valid = validation.valid;
    if valid.is_empty() {
        return Err(RunError::NoRunnableAttractor);
    }

    let descriptor = match &arguments.attractor_id {
        Some(id) => Some(
            valid
                .iter()
                .find(|descriptor| &descriptor.id == id)
                .cloned()
                .ok_or_else(|| RunError::UnknownAttractor(id.clone()))?,
        ),
        None => None,
    };

    let mut parameter_values = HashMap::new();
    if !arguments.parameters.is_empty() {
        let descriptor = descriptor
            .as_ref()
            .ok_or(RunError::AttractorRequiredForParameters)?;
        let declared: HashSet<&str> = descriptor
            .parameters
            .iter()
            .map(|parameter| parameter.name.as_str())
            .collect();
        let available = descriptor
            .parameters
            .iter()
            .map(|parameter| parameter.name.clone())
            .collect::<Vec<_>>()
            .join(", ");
        for (name, raw) in &arguments.parameters {
            if !declared.contains(name.as_str()) {
                return Err(RunError::UnknownParameter {
                    name: name.clone(),
                    attractor: descriptor.id.clone(),
                    available,
                });
            }
            let value = raw.parse::<f64>().ok().filter(|value| value.is_finite());
            let Some(value) = value else {
                return Err(RunError::InvalidParameterValue {
                    name: name.clone(),
                    value: raw.clone(),
                    available,
                });
            };
            parameter_values.insert(name.clone(), value);
        }
    }

    let seed = arguments.seed.unwrap_or_else(time_seed);
    let mut rng = SplitMix64::new(seed);
    let selection = match (&descriptor, parameter_values.is_empty()) {
        (Some(descriptor), false) => {
            DebugConfig::selection_with(&descriptor.id, &valid, parameter_values, None)
                .map_err(RunError::DebugConfig)?
        }
        (Some(descriptor), true) => {
            randomizer::select_attractor(descriptor, &mut rng).map_err(RunError::Randomizer)?
        }
        (None, _) => {
            randomizer::select_from_catalog(&valid, &mut rng).map_err(RunError::Randomizer)?
        }
    };

    let base = selection.settings;
    let overrides = arguments.overrides;
    let settings = SimulationSettings::new(
        overrides.step_size.unwrap_or(base.step_size),
        overrides.simulation_speed.unwrap_or(base.simulation_speed),
        overrides
            .camera_rotation_speed
            .unwrap_or(base.camera_rotation_speed),
        overrides
            .trail_color_speed
            .unwrap_or(base.trail_color_speed),
        overrides.trail_length.unwrap_or(base.trail_length),
        overrides.color_mode.unwrap_or(base.color_mode),
    );
    let issues = settings.validation_issues();
    if !issues.is_empty() {
        return Err(RunError::Settings(issues));
    }

    let effective_selection = AttractorSelection::new(
        selection.descriptor.clone(),
        selection.parameters.clone(),
        settings,
        arguments.initial_state.unwrap_or(selection.initial_state),
    );

    let capture = arguments.capture.clone().ok_or(RunError::CaptureMissing)?;

    let width = arguments.width.unwrap_or(DEFAULT_WIDTH);
    let height = arguments.height.unwrap_or(DEFAULT_HEIGHT);
    let capture_scale = arguments.capture_scale;
    let (pixel_width, pixel_height) = (
        width.saturating_mul(capture_scale),
        height.saturating_mul(capture_scale),
    );
    if pixel_width == 0
        || pixel_height == 0
        || pixel_width > MAXIMUM_PIXEL_DIMENSION
        || pixel_height > MAXIMUM_PIXEL_DIMENSION
    {
        return Err(RunError::InvalidCaptureSize {
            width: pixel_width,
            height: pixel_height,
        });
    }

    let mut player = Player::new(
        effective_selection,
        valid,
        PlayerOptions {
            camera_azimuth: arguments.camera_azimuth_degrees.map(f64::to_radians),
            camera_elevation: arguments.camera_elevation_degrees.map(f64::to_radians),
            heal_automatically: false,
            ..PlayerOptions::default()
        },
        rng,
    )
    .map_err(RunError::Player)?;
    player
        .resize(pixel_width, pixel_height, capture_scale as f32)
        .map_err(RunError::Player)?;

    Ok(PreparedRun {
        player,
        capture,
        warmup_steps: arguments.warmup_steps.unwrap_or(DEFAULT_WARMUP_STEPS),
        width,
        height,
        capture_scale,
        frames: arguments.frames,
    })
}

/// Prepare, simulate and write the requested capture.
pub fn run(arguments: &CommandLineArguments) -> Result<CaptureReport, RunError> {
    let mut prepared = prepare(arguments)?;
    prepared
        .player
        .warm_up(prepared.warmup_steps)
        .map_err(RunError::Player)?;

    let (pixel_width, pixel_height) = prepared.pixel_size();
    let context = GpuContext::new().map_err(RunError::Gpu)?;

    let mut report = CaptureReport {
        frames_simulated: prepared.player.frames_simulated(),
        frames_written: 0,
        width: pixel_width,
        height: pixel_height,
        lit_pixels: 0,
        outputs: Vec::new(),
    };

    match prepared.capture.clone() {
        CaptureRequest::Screenshot(path) => {
            let frames = prepared.frames.unwrap_or(DEFAULT_SETTLE_FRAMES);
            advance(&mut prepared.player, frames)?;
            render_into(&context, &mut prepared.player, &path, &mut report)?;
        }
        CaptureRequest::Record {
            output_directory,
            seconds,
            frame_stride,
        } => {
            fs::create_dir_all(&output_directory).map_err(RunError::Io)?;
            let frames_to_capture = prepared
                .frames
                .unwrap_or_else(|| ((seconds * 60.0) / frame_stride as f64).ceil() as usize)
                .max(1);
            record_sequence(
                &mut prepared.player,
                frames_to_capture,
                frame_stride,
                |player, index| {
                    let path = output_directory.join(format!("frame_{index:05}.png"));
                    render_into(&context, player, &path, &mut report)
                },
            )?;
        }
    }
    report.frames_simulated = prepared.player.frames_simulated();
    Ok(report)
}

fn advance(player: &mut Player, frames: usize) -> Result<(), RunError> {
    for _ in 0..frames {
        player.tick(FRAME_INTERVAL).map_err(RunError::Player)?;
    }
    Ok(())
}

/// Simulate a recording and hand every captured frame to `capture` together
/// with its one-based file index.
///
/// The sequence mirrors the Swift `CaptureController`: after the settle phase,
/// `frames_to_capture` frames are written every `frame_stride` simulated ticks.
pub fn record_sequence(
    player: &mut Player,
    frames_to_capture: usize,
    frame_stride: usize,
    mut capture: impl FnMut(&mut Player, usize) -> Result<(), RunError>,
) -> Result<(), RunError> {
    advance(player, DEFAULT_SETTLE_FRAMES)?;
    let total_frames = frames_to_capture.saturating_mul(frame_stride);
    for frame in 1..=total_frames {
        advance(player, 1)?;
        if frame % frame_stride != 0 {
            continue;
        }
        let index = frame / frame_stride;
        capture(player, index)?;
        if index >= frames_to_capture {
            break;
        }
    }
    Ok(())
}

fn render_into(
    context: &GpuContext,
    player: &mut Player,
    path: &std::path::Path,
    report: &mut CaptureReport,
) -> Result<(), RunError> {
    let pixels = player.render(context).map_err(RunError::Player)?;
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent).map_err(RunError::Io)?;
    }
    write_png(path, report.width, report.height, &pixels).map_err(RunError::Io)?;
    report.lit_pixels = lit_pixels(&pixels);
    report.frames_written += 1;
    report.outputs.push(path.to_path_buf());
    Ok(())
}

/// A seed that differs between runs when `--seed` is omitted.
fn time_seed() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos() as u64)
        .unwrap_or(0x9e37_79b9_7f4a_7c15)
        ^ u64::from(std::process::id())
}
