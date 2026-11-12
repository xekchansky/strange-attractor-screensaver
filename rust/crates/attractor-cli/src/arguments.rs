//! Command-line parsing, mirroring the Swift demo's flags.

use std::collections::HashMap;
use std::error::Error;
use std::fmt;
use std::path::PathBuf;

use attractor_core::{State, TraceColorMode};

/// Errors produced while parsing command-line arguments.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandLineArgumentsError {
    /// A flag the CLI does not know.
    UnknownArgument(String),
    /// A flag that needs a value was the last token.
    MissingValue(String),
    /// A value that does not parse or is outside its accepted range.
    InvalidParameter(String),
    /// Two flags that cannot be combined were both supplied.
    ConflictingOptions(String),
    /// A modifier flag was supplied without the flag it modifies.
    OptionRequiresRecord(String),
}

impl fmt::Display for CommandLineArgumentsError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownArgument(value) => write!(formatter, "unknown argument: {value}"),
            Self::MissingValue(flag) => write!(formatter, "missing value for {flag}"),
            Self::InvalidParameter(value) => write!(formatter, "invalid value: {value}"),
            Self::ConflictingOptions(value) => write!(formatter, "conflicting options: {value}"),
            Self::OptionRequiresRecord(flag) => write!(formatter, "{flag} requires --record"),
        }
    }
}

impl Error for CommandLineArgumentsError {}

/// Optional overrides for the simulation hyperparameters, applied on top of
/// whatever the selection (manual or random) chose.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct SimulationOverrides {
    /// Override for the trail length (number of points kept).
    pub trail_length: Option<i64>,
    /// Override for the simulated-time multiplier per rendered frame.
    pub simulation_speed: Option<f64>,
    /// Override for the integration step size (accuracy).
    pub step_size: Option<f64>,
    /// Override for the camera orbit angular velocity (radians/second).
    pub camera_rotation_speed: Option<f64>,
    /// Override for the trail color gradient advance speed.
    pub trail_color_speed: Option<f64>,
    /// Override for the trail color mode.
    pub color_mode: Option<TraceColorMode>,
}

/// Capture request derived from the command line, if any.
#[derive(Debug, Clone, PartialEq)]
pub enum CaptureRequest {
    /// Save one PNG after warm-up (fast-forwarded trail and settled camera).
    Screenshot(PathBuf),
    /// Save a sequence of PNGs for GIF/APNG assembly.
    Record {
        /// Directory the frames are written to.
        output_directory: PathBuf,
        /// Duration of the recording in simulated seconds.
        seconds: f64,
        /// Capture every n-th rendered frame.
        frame_stride: usize,
    },
}

/// Parsed command-line options for the demo.
#[derive(Debug, Clone, PartialEq)]
pub struct CommandLineArguments {
    /// Fixed attractor id, when `--attractor` was supplied.
    pub attractor_id: Option<String>,
    /// Raw `name=value` parameters. Values stay strings: numeric conversion
    /// and name validation happen against the catalog later, which knows what
    /// each attractor actually declares.
    pub parameters: HashMap<String, String>,
    /// Optional simulation hyperparameter overrides.
    pub overrides: SimulationOverrides,
    /// Optional capture request (`--screenshot` / `--record`).
    pub capture: Option<CaptureRequest>,
    /// Number of fast-forward integration steps before the capture starts.
    /// `None` keeps the built-in default of 60 000.
    pub warmup_steps: Option<usize>,
    /// Resolution multiplier applied to the logical frame size.
    pub capture_scale: u32,
    /// Starting camera orbit azimuth in degrees. `None` picks a random angle.
    pub camera_azimuth_degrees: Option<f64>,
    /// Camera orbit elevation above the horizontal plane, in degrees.
    pub camera_elevation_degrees: Option<f64>,
    /// Explicit starting state for the integration.
    pub initial_state: Option<State>,
    /// Seed for the deterministic randomizer and camera azimuth.
    pub seed: Option<u64>,
    /// Frames to simulate before a screenshot (or to record after settle).
    pub frames: Option<usize>,
    /// Logical capture width in points; the default is 1200.
    pub width: Option<u32>,
    /// Logical capture height in points; the default is 900.
    pub height: Option<u32>,
    /// True when `--help` was requested.
    pub show_help: bool,
}

impl CommandLineArguments {
    /// Largest accepted logical width or height.
    pub const MAXIMUM_DIMENSION: u32 = 16_384;

    /// Usage text shown by `--help` and after a parse error.
    pub const USAGE: &str = "\
Usage: attractor-cli [options]

Renders the attractor screensaver headlessly and captures PNG frames.

Options:
  -a, --attractor <id>        fix a specific attractor (lorenz, rossler, ...)
  -p, --param name=val        pin a simulation parameter value (repeatable)
      --trail-length <n>      override trail length (number of points)
      --speed <v>             override simulation speed multiplier
      --step <v>              override integration step size
      --cam-speed <v>         override camera orbit speed (rad/s)
      --trail-color-speed <v> override trail color gradient speed
      --color-mode <mode>     override color mode: trail|age|depth|spatial
      --screenshot <path>     save one PNG and exit
      --record <dir>          save a sequence of PNG frames and exit
      --record-seconds <v>    duration of the capture in seconds (default 4)
      --frame-stride <n>      capture every n-th rendered frame (default 2)
      --frames <n>            simulation frames before a screenshot (default
                              90), or the number of frames to record
      --warmup-steps <n>      integration steps fast-forwarded before capture
                              (default 60000; 0 records the trail from the start)
      --capture-scale <n>     resolution multiplier (default 2; 1-4)
      --width <n>             logical capture width in points (default 1200)
      --height <n>            logical capture height in points (default 900)
      --cam-azimuth <deg>     start the camera orbit at this azimuth
                              (default: random)
      --cam-elevation <deg>   camera height above the horizontal plane
                              (default: 36 degrees)
      --seed <n>              seed the randomizer and the camera azimuth
                              (default: a time-based seed)
      --initial-state x,y,z   start the integration at this point instead of
                              the selection's sampled start
  -h, --help                  show this help (including available attractors) and exit

Bare `name=value` tokens are also treated as pinned parameters, e.g.
  attractor-cli --attractor rossler a=0.2 b=0.2 c=5.7

Without options, a random attractor with random parameters is chosen.
This binary captures files only; at least one of --screenshot or --record is
required. The interactive surfaces are hosted by the platform shells.
";

    /// Parse command-line arguments, excluding the executable path.
    pub fn parse(arguments: &[String]) -> Result<Self, CommandLineArgumentsError> {
        let mut attractor_id = None;
        let mut parameters = HashMap::new();
        let mut overrides = SimulationOverrides::default();
        let mut screenshot_path: Option<PathBuf> = None;
        let mut record_directory: Option<PathBuf> = None;
        let mut record_seconds: Option<f64> = None;
        let mut record_stride: Option<usize> = None;
        let mut warmup = None;
        let mut capture_scale = 2;
        let mut camera_azimuth = None;
        let mut camera_elevation = None;
        let mut initial_state = None;
        let mut seed = None;
        let mut frames = None;
        let mut width = None;
        let mut height = None;
        let mut show_help = false;

        let mut index = 0;
        while index < arguments.len() {
            let argument = arguments[index].as_str();
            match argument {
                "-a" | "--attractor" => {
                    attractor_id = Some(take_value(arguments, &mut index, argument)?);
                }
                "-p" | "--param" => {
                    let raw = take_value(arguments, &mut index, argument)?;
                    add_parameter(&mut parameters, &raw)?;
                }
                "--trail-length" => {
                    let raw = take_value(arguments, &mut index, argument)?;
                    let count = raw
                        .parse::<i64>()
                        .ok()
                        .filter(|count| *count >= 1)
                        .ok_or_else(|| invalid(argument, &raw))?;
                    overrides.trail_length = Some(count);
                }
                "--speed" => {
                    let raw = take_value(arguments, &mut index, argument)?;
                    overrides.simulation_speed = Some(parse_double(argument, &raw)?);
                }
                "--step" => {
                    let raw = take_value(arguments, &mut index, argument)?;
                    overrides.step_size = Some(parse_double(argument, &raw)?);
                }
                "--cam-speed" => {
                    let raw = take_value(arguments, &mut index, argument)?;
                    overrides.camera_rotation_speed = Some(parse_double(argument, &raw)?);
                }
                "--trail-color-speed" => {
                    let raw = take_value(arguments, &mut index, argument)?;
                    overrides.trail_color_speed = Some(parse_double(argument, &raw)?);
                }
                "--color-mode" => {
                    let raw = take_value(arguments, &mut index, argument)?;
                    let mode = TraceColorMode::ALL
                        .into_iter()
                        .find(|mode| mode.as_str() == raw)
                        .ok_or_else(|| {
                            CommandLineArgumentsError::InvalidParameter(format!(
                                "\"{argument} {raw}\""
                            ))
                        })?;
                    overrides.color_mode = Some(mode);
                }
                "--screenshot" => {
                    if screenshot_path.is_some() || record_directory.is_some() {
                        return Err(mutually_exclusive());
                    }
                    screenshot_path =
                        Some(PathBuf::from(take_value(arguments, &mut index, argument)?));
                }
                "--record" => {
                    if screenshot_path.is_some() || record_directory.is_some() {
                        return Err(mutually_exclusive());
                    }
                    record_directory =
                        Some(PathBuf::from(take_value(arguments, &mut index, argument)?));
                }
                "--record-seconds" => {
                    let raw = take_value(arguments, &mut index, argument)?;
                    let seconds = parse_double(argument, &raw)?;
                    // An unbounded duration would overflow the frame count;
                    // an hour is far beyond any real recording.
                    if !(seconds > 0.0 && seconds <= 3600.0) {
                        return Err(invalid(argument, &raw));
                    }
                    record_seconds = Some(seconds);
                }
                "--frame-stride" => {
                    let raw = take_value(arguments, &mut index, argument)?;
                    let stride = raw
                        .parse::<usize>()
                        .ok()
                        .filter(|stride| *stride >= 1)
                        .ok_or_else(|| invalid(argument, &raw))?;
                    record_stride = Some(stride);
                }
                "-h" | "--help" => show_help = true,
                "--warmup-steps" => {
                    let raw = take_value(arguments, &mut index, argument)?;
                    warmup = Some(parse_usize(argument, &raw, usize::MAX)?);
                }
                "--capture-scale" => {
                    let raw = take_value(arguments, &mut index, argument)?;
                    let scale = raw
                        .parse::<u32>()
                        .ok()
                        .filter(|scale| (1..=4).contains(scale))
                        .ok_or_else(|| invalid(argument, &raw))?;
                    capture_scale = scale;
                }
                "--cam-azimuth" => {
                    let raw = take_value(arguments, &mut index, argument)?;
                    camera_azimuth = Some(parse_double(argument, &raw)?);
                }
                "--cam-elevation" => {
                    let raw = take_value(arguments, &mut index, argument)?;
                    let degrees = parse_double(argument, &raw)?;
                    if !(-89.0..=89.0).contains(&degrees) {
                        return Err(invalid(argument, &raw));
                    }
                    camera_elevation = Some(degrees);
                }
                "--seed" => {
                    let raw = take_value(arguments, &mut index, argument)?;
                    seed = Some(raw.parse::<u64>().map_err(|_| invalid(argument, &raw))?);
                }
                "--frames" => {
                    let raw = take_value(arguments, &mut index, argument)?;
                    frames = Some(parse_usize(argument, &raw, usize::MAX)?);
                }
                "--width" => {
                    let raw = take_value(arguments, &mut index, argument)?;
                    width = Some(parse_dimension(argument, &raw)?);
                }
                "--height" => {
                    let raw = take_value(arguments, &mut index, argument)?;
                    height = Some(parse_dimension(argument, &raw)?);
                }
                "--initial-state" => {
                    let raw = take_value(arguments, &mut index, argument)?;
                    initial_state = Some(parse_state(argument, &raw)?);
                }
                _ => {
                    // Any bare `name=value` token is treated as a parameter.
                    if argument.contains('=') {
                        add_parameter(&mut parameters, argument)?;
                    } else {
                        return Err(CommandLineArgumentsError::UnknownArgument(
                            argument.to_string(),
                        ));
                    }
                }
            }
            index += 1;
        }

        // Capture modifiers only make sense with a recording request, and the
        // order of the flags on the command line must not matter.
        if record_directory.is_none() {
            if record_seconds.is_some() {
                return Err(CommandLineArgumentsError::OptionRequiresRecord(
                    "--record-seconds".to_string(),
                ));
            }
            if record_stride.is_some() {
                return Err(CommandLineArgumentsError::OptionRequiresRecord(
                    "--frame-stride".to_string(),
                ));
            }
        }
        if frames.is_some() && record_seconds.is_some() {
            return Err(CommandLineArgumentsError::ConflictingOptions(
                "--frames and --record-seconds are mutually exclusive".to_string(),
            ));
        }

        let capture = if let Some(path) = screenshot_path {
            Some(CaptureRequest::Screenshot(path))
        } else {
            record_directory.map(|output_directory| CaptureRequest::Record {
                output_directory,
                seconds: record_seconds.unwrap_or(4.0),
                frame_stride: record_stride.unwrap_or(2),
            })
        };

        Ok(Self {
            attractor_id,
            parameters,
            overrides,
            capture,
            warmup_steps: warmup,
            capture_scale,
            camera_azimuth_degrees: camera_azimuth,
            camera_elevation_degrees: camera_elevation,
            initial_state,
            seed,
            frames,
            width,
            height,
            show_help,
        })
    }
}

fn take_value(
    arguments: &[String],
    index: &mut usize,
    flag: &str,
) -> Result<String, CommandLineArgumentsError> {
    *index += 1;
    arguments
        .get(*index)
        .cloned()
        .ok_or_else(|| CommandLineArgumentsError::MissingValue(flag.to_string()))
}

fn add_parameter(
    parameters: &mut HashMap<String, String>,
    raw: &str,
) -> Result<(), CommandLineArgumentsError> {
    let Some(equals) = raw.find('=') else {
        return Err(CommandLineArgumentsError::InvalidParameter(raw.to_string()));
    };
    if equals == 0 {
        return Err(CommandLineArgumentsError::InvalidParameter(raw.to_string()));
    }
    parameters.insert(raw[..equals].to_string(), raw[equals + 1..].to_string());
    Ok(())
}

fn parse_double(flag: &str, raw: &str) -> Result<f64, CommandLineArgumentsError> {
    raw.parse::<f64>()
        .ok()
        .filter(|value| value.is_finite())
        .ok_or_else(|| invalid(flag, raw))
}

fn parse_usize(flag: &str, raw: &str, maximum: usize) -> Result<usize, CommandLineArgumentsError> {
    raw.parse::<usize>()
        .ok()
        .filter(|value| *value <= maximum)
        .ok_or_else(|| invalid(flag, raw))
}

/// Parse a capture dimension: at least one pixel, at most
/// [`CommandLineArguments::MAXIMUM_DIMENSION`].
fn parse_dimension(flag: &str, raw: &str) -> Result<u32, CommandLineArgumentsError> {
    raw.parse::<u32>()
        .ok()
        .filter(|value| (1..=CommandLineArguments::MAXIMUM_DIMENSION).contains(value))
        .ok_or_else(|| invalid(flag, raw))
}

fn parse_state(flag: &str, raw: &str) -> Result<State, CommandLineArgumentsError> {
    let parts: Vec<&str> = raw.split(',').map(str::trim).collect();
    if parts.len() != 3 {
        return Err(invalid(flag, raw));
    }
    let mut numbers = [0.0f64; 3];
    for (target, part) in numbers.iter_mut().zip(parts) {
        *target = part
            .parse::<f64>()
            .ok()
            .filter(|value| value.is_finite())
            .ok_or_else(|| invalid(flag, raw))?;
    }
    Ok(State::new(numbers[0], numbers[1], numbers[2]))
}

fn invalid(flag: &str, raw: &str) -> CommandLineArgumentsError {
    CommandLineArgumentsError::InvalidParameter(format!("{flag} {raw}"))
}

fn mutually_exclusive() -> CommandLineArgumentsError {
    CommandLineArgumentsError::ConflictingOptions(
        "--screenshot and --record are mutually exclusive".to_string(),
    )
}
