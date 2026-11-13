//! The Linux command line: the xscreensaver hack protocol and the saver's own
//! switches.
//!
//! xscreensaver starts a hack with `-root` (draw on the screen) or
//! `-window-id N` (draw inside a window it created) and adds a handful of
//! settings flags (`-delay`, `-fps`, `-mono`, `-visual`, `-install`,
//! `-noinstall`, `-verbose`). The parser accepts all of them so the binary can
//! be dropped into `~/.xscreensaver` unchanged.
//!
//! Away from xscreensaver the same binary is the fullscreen runner that
//! `.desktop` launchers and the `swayidle`/`hypridle` recipes start with
//! `--fullscreen`. Its `--self-test` switches render one frame offscreen, which
//! is how CI checks the pipelines without a desktop.

use std::fmt;

use attractor_shell::self_test::{
    DEFAULT_HEIGHT, DEFAULT_WIDTH, MAXIMUM_SIZE, SelfTestBackend, SelfTestRequest,
};

/// Largest number of frames `--frames` accepts (one hour at 60 fps).
pub const MAXIMUM_FRAMES: u64 = 216_000;

/// Which window the xscreensaver hack draws into.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HackWindow {
    /// `-root`: create the saver's own windows on the screen and draw there.
    Root,
    /// `-window-id N`: draw inside the window xscreensaver mapped.
    Given(u32),
}

/// What this invocation of the executable should do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    /// `--fullscreen` (and the default): run the fullscreen runner.
    Fullscreen {
        /// Stop after this many frames; `None` runs until the user's input.
        frames: Option<u64>,
    },
    /// The xscreensaver hack protocol.
    Hack {
        /// Window the hack draws into.
        window: HackWindow,
        /// Stop after this many frames; `None` runs until the user's input.
        frames: Option<u64>,
    },
    /// `--self-test`: render one frame offscreen and report the result.
    SelfTest(SelfTestRequest),
    /// `--help`: print the usage text.
    Help,
}

/// A parsed command line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandLine {
    /// What the executable should do.
    pub command: Command,
}

impl CommandLine {
    /// Usage text printed by `--help` and after a usage error.
    pub const USAGE: &str = "\
Usage: attractor-linux-saver [command]

xscreensaver hack (passed by xscreensaver):
  -root           draw on the screen
  -window-id N    draw inside the window N

Fullscreen runner (desktop launchers and idle hooks):
  --fullscreen    cover the screen until the user gives any input
  --frames N      stop after N frames (a smoke test)

Self-test (no desktop needed):
  --self-test     render an offscreen frame and report the result
  --noop          use wgpu's noop backend (validates the pipelines only)
  --width N       self-test frame width (default 640)
  --height N      self-test frame height (default 480)

  -h, --help      show this text

xscreensaver's own flags (-delay, -fps, -mono, -visual, -install, -noinstall,
-verbose) are accepted and ignored.

Without arguments the fullscreen runner starts.
";

    /// Parse the arguments that follow the executable name.
    pub fn parse(arguments: &[String]) -> Result<Self, CommandLineError> {
        let mut root = false;
        let mut window_id: Option<u32> = None;
        let mut fullscreen = false;
        let mut frames: Option<u64> = None;
        let mut self_test = false;
        let mut help = false;
        let mut width: Option<u32> = None;
        let mut height: Option<u32> = None;
        let mut noop = false;
        let mut self_test_only: Option<String> = None;

        let mut index = 0;
        while index < arguments.len() {
            let raw = arguments[index].as_str();
            index += 1;
            match raw {
                "" => {}
                "-root" => root = true,
                "-window-id" => {
                    let value = next_value(arguments, &mut index, "-window-id")?;
                    window_id = Some(parse_window_id(value).ok_or_else(|| {
                        CommandLineError::InvalidWindowId {
                            value: value.to_owned(),
                        }
                    })?);
                }
                "-mono" | "-install" | "-noinstall" | "-verbose" => {}
                "-fps" => {
                    next_value(arguments, &mut index, "-fps")?;
                }
                "-delay" => {
                    next_value(arguments, &mut index, "-delay")?;
                }
                "-visual" => {
                    next_value(arguments, &mut index, "-visual")?;
                }
                "--fullscreen" => fullscreen = true,
                "--frames" => {
                    let value = next_value(arguments, &mut index, "--frames")?;
                    let count = value
                        .parse::<u64>()
                        .ok()
                        .filter(|count| (1..=MAXIMUM_FRAMES).contains(count))
                        .ok_or_else(|| CommandLineError::InvalidFrameCount {
                            value: value.to_owned(),
                        })?;
                    frames = Some(count);
                }
                "--self-test" => self_test = true,
                "--noop" => {
                    noop = true;
                    self_test_only.get_or_insert_with(|| raw.to_owned());
                }
                "--width" | "--height" => {
                    let flag: &'static str = if raw == "--width" {
                        "--width"
                    } else {
                        "--height"
                    };
                    let value = next_value(arguments, &mut index, flag)?;
                    let size = value
                        .parse::<u32>()
                        .ok()
                        .filter(|size| (1..=MAXIMUM_SIZE).contains(size))
                        .ok_or_else(|| CommandLineError::InvalidSize {
                            flag,
                            value: value.to_owned(),
                        })?;
                    if flag == "--width" {
                        width = Some(size);
                    } else {
                        height = Some(size);
                    }
                    self_test_only.get_or_insert_with(|| raw.to_owned());
                }
                "-h" | "--help" => help = true,
                _ => {
                    if let Some(value) = raw.strip_prefix("-window-id=") {
                        window_id = Some(parse_window_id(value).ok_or_else(|| {
                            CommandLineError::InvalidWindowId {
                                value: raw.to_owned(),
                            }
                        })?);
                    } else {
                        return Err(CommandLineError::UnknownArgument(raw.to_owned()));
                    }
                }
            }
        }

        if help {
            return Ok(Self {
                command: Command::Help,
            });
        }

        if root && window_id.is_some() {
            return Err(CommandLineError::ConflictingCommands);
        }
        let hack = root || window_id.is_some();
        let modes = usize::from(hack) + usize::from(fullscreen) + usize::from(self_test);
        if modes > 1 {
            return Err(CommandLineError::ConflictingCommands);
        }
        if !self_test && let Some(argument) = self_test_only {
            return Err(CommandLineError::SelfTestOnlyArgument(argument));
        }

        if self_test {
            return Ok(Self {
                command: Command::SelfTest(SelfTestRequest {
                    width: width.unwrap_or(DEFAULT_WIDTH),
                    height: height.unwrap_or(DEFAULT_HEIGHT),
                    backend: if noop {
                        SelfTestBackend::Noop
                    } else {
                        SelfTestBackend::Auto
                    },
                }),
            });
        }
        if hack {
            let window = match window_id {
                Some(id) => HackWindow::Given(id),
                None => HackWindow::Root,
            };
            return Ok(Self {
                command: Command::Hack { window, frames },
            });
        }
        Ok(Self {
            command: Command::Fullscreen { frames },
        })
    }
}

/// The value following a switch, or a usage error naming that switch.
fn next_value<'a>(
    arguments: &'a [String],
    index: &mut usize,
    flag: &'static str,
) -> Result<&'a str, CommandLineError> {
    let value = arguments
        .get(*index)
        .ok_or(CommandLineError::MissingValue { flag })?;
    *index += 1;
    Ok(value)
}

/// Parse a decimal or `0x`-prefixed, non-zero X11 window id.
fn parse_window_id(text: &str) -> Option<u32> {
    let (digits, radix) = match text.strip_prefix("0x").or_else(|| text.strip_prefix("0X")) {
        Some(digits) => (digits, 16),
        None => (text, 10),
    };
    u32::from_str_radix(digits, radix)
        .ok()
        .filter(|value| *value != 0)
}

/// Why the command line could not be understood.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandLineError {
    /// A token matched no switch.
    UnknownArgument(String),
    /// A switch was not followed by a value.
    MissingValue {
        /// The switch that lacks a value.
        flag: &'static str,
    },
    /// `-window-id` was not a non-zero X11 window id.
    InvalidWindowId {
        /// The text that failed to parse.
        value: String,
    },
    /// `--frames` did not name a usable frame count.
    InvalidFrameCount {
        /// The text that failed to parse.
        value: String,
    },
    /// `--width` or `--height` did not name a usable size.
    InvalidSize {
        /// The switch that was given.
        flag: &'static str,
        /// The text that failed to parse.
        value: String,
    },
    /// More than one run mode was given.
    ConflictingCommands,
    /// A self-test switch was given without `--self-test`.
    SelfTestOnlyArgument(String),
}

impl fmt::Display for CommandLineError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownArgument(argument) => {
                write!(formatter, "unknown argument '{argument}'")
            }
            Self::MissingValue { flag } => write!(formatter, "'{flag}' needs a value"),
            Self::InvalidWindowId { value } => {
                write!(formatter, "'{value}' is not a window id")
            }
            Self::InvalidFrameCount { value } => {
                write!(
                    formatter,
                    "'{value}' is not a usable '--frames' (1 to {MAXIMUM_FRAMES})"
                )
            }
            Self::InvalidSize { flag, value } => {
                write!(
                    formatter,
                    "'{value}' is not a usable '{flag}' (1 to {MAXIMUM_SIZE})"
                )
            }
            Self::ConflictingCommands => {
                write!(
                    formatter,
                    "only one of -root, -window-id, --fullscreen and --self-test may be given"
                )
            }
            Self::SelfTestOnlyArgument(argument) => {
                write!(formatter, "'{argument}' only applies to --self-test")
            }
        }
    }
}

impl std::error::Error for CommandLineError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(arguments: &[&str]) -> Result<Command, CommandLineError> {
        let owned: Vec<String> = arguments.iter().map(|value| (*value).to_owned()).collect();
        CommandLine::parse(&owned).map(|line| line.command)
    }

    #[test]
    fn no_arguments_run_the_fullscreen_runner() {
        assert_eq!(parse(&[]), Ok(Command::Fullscreen { frames: None }));
    }

    #[test]
    fn the_root_switch_runs_the_hack() {
        assert_eq!(
            parse(&["-root"]),
            Ok(Command::Hack {
                window: HackWindow::Root,
                frames: None
            })
        );
    }

    #[test]
    fn the_window_id_switch_attaches_to_the_given_window() {
        assert_eq!(
            parse(&["-window-id", "1234"]),
            Ok(Command::Hack {
                window: HackWindow::Given(1234),
                frames: None
            })
        );
        assert_eq!(
            parse(&["-window-id=0x4d2"]),
            Ok(Command::Hack {
                window: HackWindow::Given(1234),
                frames: None
            })
        );
    }

    #[test]
    fn a_bad_window_id_is_a_usage_error() {
        for value in ["0", "0x0", "left", "-5"] {
            assert_eq!(
                parse(&["-window-id", value]),
                Err(CommandLineError::InvalidWindowId {
                    value: value.to_owned()
                })
            );
        }
        assert_eq!(
            parse(&["-window-id"]),
            Err(CommandLineError::MissingValue { flag: "-window-id" })
        );
    }

    #[test]
    fn xscreensaver_settings_flags_are_accepted_and_ignored() {
        let command = parse(&[
            "-root",
            "-mono",
            "-install",
            "-noinstall",
            "-verbose",
            "-fps",
            "60",
            "-delay",
            "5",
            "-visual",
            "0x21",
        ]);
        assert_eq!(
            command,
            Ok(Command::Hack {
                window: HackWindow::Root,
                frames: None
            })
        );
    }

    #[test]
    fn frames_limit_the_run() {
        assert_eq!(
            parse(&["--fullscreen", "--frames", "30"]),
            Ok(Command::Fullscreen { frames: Some(30) })
        );
        assert_eq!(
            parse(&["-root", "--frames", "30"]),
            Ok(Command::Hack {
                window: HackWindow::Root,
                frames: Some(30)
            })
        );
        assert_eq!(
            parse(&["--frames", "30"]),
            Ok(Command::Fullscreen { frames: Some(30) })
        );
    }

    #[test]
    fn a_bad_frame_count_is_a_usage_error() {
        for value in ["0", "216001", "many", "-1"] {
            assert_eq!(
                parse(&["--frames", value]),
                Err(CommandLineError::InvalidFrameCount {
                    value: value.to_owned()
                })
            );
        }
        assert_eq!(
            parse(&["--frames"]),
            Err(CommandLineError::MissingValue { flag: "--frames" })
        );
    }

    #[test]
    fn the_self_test_defaults_are_used_without_size_flags() {
        assert_eq!(
            parse(&["--self-test"]),
            Ok(Command::SelfTest(SelfTestRequest::default()))
        );
    }

    #[test]
    fn the_self_test_accepts_a_size_and_the_noop_backend() {
        assert_eq!(
            parse(&["--self-test", "--width", "320", "--height", "240", "--noop"]),
            Ok(Command::SelfTest(SelfTestRequest {
                width: 320,
                height: 240,
                backend: SelfTestBackend::Noop,
            }))
        );
    }

    #[test]
    fn a_bad_self_test_size_is_a_usage_error() {
        for value in ["0", "8193", "-1", "wide"] {
            assert_eq!(
                parse(&["--self-test", "--width", value]),
                Err(CommandLineError::InvalidSize {
                    flag: "--width",
                    value: value.to_owned()
                })
            );
        }
        assert_eq!(
            parse(&["--self-test", "--height"]),
            Err(CommandLineError::MissingValue { flag: "--height" })
        );
    }

    #[test]
    fn self_test_switches_need_the_self_test_command() {
        assert_eq!(
            parse(&["--noop"]),
            Err(CommandLineError::SelfTestOnlyArgument("--noop".to_owned()))
        );
        assert_eq!(
            parse(&["-root", "--width", "320"]),
            Err(CommandLineError::SelfTestOnlyArgument("--width".to_owned()))
        );
    }

    #[test]
    fn run_modes_do_not_combine() {
        assert_eq!(
            parse(&["-root", "-window-id", "5"]),
            Err(CommandLineError::ConflictingCommands)
        );
        assert_eq!(
            parse(&["--fullscreen", "-root"]),
            Err(CommandLineError::ConflictingCommands)
        );
        assert_eq!(
            parse(&["--self-test", "-root"]),
            Err(CommandLineError::ConflictingCommands)
        );
    }

    #[test]
    fn help_wins_over_everything_else() {
        assert_eq!(parse(&["--help"]), Ok(Command::Help));
        assert_eq!(parse(&["-h"]), Ok(Command::Help));
        assert_eq!(parse(&["-root", "--help"]), Ok(Command::Help));
    }

    #[test]
    fn unknown_arguments_are_reported() {
        assert_eq!(
            parse(&["-bogus"]),
            Err(CommandLineError::UnknownArgument("-bogus".to_owned()))
        );
        assert_eq!(
            parse(&["--frame", "10"]),
            Err(CommandLineError::UnknownArgument("--frame".to_owned()))
        );
    }
}
