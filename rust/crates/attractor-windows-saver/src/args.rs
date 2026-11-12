//! The Windows screen saver command line.
//!
//! Windows, and the Screen Saver Settings page, start a `.scr` with one of
//! three switches: `/s` runs the saver, `/c` (optionally `/c:HWND`) opens its
//! settings and `/p HWND` asks it to draw inside the control panel's preview
//! control. The parser is platform-neutral: it keeps window handles as opaque
//! numbers so the same table of cases is testable on every development OS.
//!
//! The saver's own `--self-test` switches are accepted next to the protocol so
//! CI can render a frame without a desktop.

use std::fmt;

use crate::self_test::{
    DEFAULT_HEIGHT, DEFAULT_WIDTH, MAXIMUM_SIZE, SelfTestBackend, SelfTestRequest,
};

/// An opaque `HWND` received from the command line.
///
/// Windows passes the value as a decimal number; some launchers use a `0x`
/// prefix. The shell casts it back to a pointer when it talks to Win32.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WindowHandle(u64);

impl WindowHandle {
    /// A handle with the given protocol value.
    pub const fn from_value(value: u64) -> Self {
        Self(value)
    }

    /// The numeric value the command line carried.
    pub const fn value(self) -> u64 {
        self.0
    }

    /// Parse a decimal or `0x`-prefixed, non-zero handle.
    fn parse(text: &str) -> Option<Self> {
        let (digits, radix) = match text.strip_prefix("0x").or_else(|| text.strip_prefix("0X")) {
            Some(digits) => (digits, 16),
            None => (text, 10),
        };
        let value = u64::from_str_radix(digits, radix)
            .ok()
            .filter(|value| *value != 0)?;
        Some(Self(value))
    }
}

/// What this invocation of the executable should do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    /// `/s`: draw on every monitor until any input arrives.
    Run,
    /// `/c` or `/c:HWND`: show the settings window.
    Configure {
        /// Owner window the settings dialog should center on, when given.
        owner: Option<WindowHandle>,
    },
    /// `/p HWND`: draw inside the control panel's preview control.
    Preview {
        /// The preview control the saver draws into.
        parent: WindowHandle,
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
Usage: attractor-windows-saver [command]

Screensaver commands (passed by Windows):
  /s              run the screen saver on every monitor
  /c              open the settings window
  /c:HWND         open the settings window owned by HWND
  /p HWND         draw inside the preview control HWND

Self-test (no desktop needed):
  --self-test     render an offscreen frame and report the result
  --noop          use wgpu's noop backend (validates the pipelines only)
  --width N       self-test frame width (default 640)
  --height N      self-test frame height (default 480)

  -h, --help      show this text

Without arguments the saver opens its settings, which is what a double-click
on the .scr file does.
";

    /// Parse the arguments that follow the executable name.
    ///
    /// An empty command line means "configure". Switches are matched without
    /// regard to case, because Windows passes `/S` in some paths.
    pub fn parse(arguments: &[String]) -> Result<Self, CommandLineError> {
        let mut run = false;
        let mut configure: Option<Option<WindowHandle>> = None;
        let mut preview: Option<WindowHandle> = None;
        let mut self_test = false;
        let mut help = false;
        let mut width: Option<u32> = None;
        let mut height: Option<u32> = None;
        let mut noop = false;
        let mut self_test_only: Option<String> = None;

        let mut index = 0;
        while index < arguments.len() {
            let raw = arguments[index].as_str();
            let lower = raw.to_ascii_lowercase();
            index += 1;
            match lower.as_str() {
                "" => {}
                "/s" => run = true,
                "/c" => {
                    let owner = arguments
                        .get(index)
                        .and_then(|value| WindowHandle::parse(value));
                    if owner.is_some() {
                        index += 1;
                    }
                    configure = Some(owner);
                }
                "/p" => {
                    let Some(value) = arguments.get(index) else {
                        return Err(CommandLineError::MissingWindowHandle);
                    };
                    let handle = WindowHandle::parse(value).ok_or_else(|| {
                        CommandLineError::InvalidWindowHandle {
                            value: value.clone(),
                        }
                    })?;
                    index += 1;
                    preview = Some(handle);
                }
                "--self-test" => self_test = true,
                "--noop" => {
                    noop = true;
                    self_test_only.get_or_insert_with(|| raw.to_owned());
                }
                "--width" | "--height" => {
                    let flag = if lower == "--width" {
                        "--width"
                    } else {
                        "--height"
                    };
                    let Some(value) = arguments.get(index) else {
                        return Err(CommandLineError::MissingValue { flag });
                    };
                    let size = value
                        .parse::<u32>()
                        .ok()
                        .filter(|size| (1..=MAXIMUM_SIZE).contains(size))
                        .ok_or_else(|| CommandLineError::InvalidSize {
                            flag,
                            value: value.clone(),
                        })?;
                    index += 1;
                    if flag == "--width" {
                        width = Some(size);
                    } else {
                        height = Some(size);
                    }
                    self_test_only.get_or_insert_with(|| raw.to_owned());
                }
                "-h" | "--help" | "/?" => help = true,
                _ => {
                    if let Some(handle) = lower.strip_prefix("/c:") {
                        let handle = WindowHandle::parse(handle).ok_or_else(|| {
                            CommandLineError::InvalidWindowHandle {
                                value: raw.to_owned(),
                            }
                        })?;
                        configure = Some(Some(handle));
                    } else if let Some(handle) = lower.strip_prefix("/p:") {
                        let handle = WindowHandle::parse(handle).ok_or_else(|| {
                            CommandLineError::InvalidWindowHandle {
                                value: raw.to_owned(),
                            }
                        })?;
                        preview = Some(handle);
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

        let modes = usize::from(run)
            + usize::from(configure.is_some())
            + usize::from(preview.is_some())
            + usize::from(self_test);
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
        if run {
            return Ok(Self {
                command: Command::Run,
            });
        }
        if let Some(owner) = configure {
            return Ok(Self {
                command: Command::Configure { owner },
            });
        }
        if let Some(parent) = preview {
            return Ok(Self {
                command: Command::Preview { parent },
            });
        }
        Ok(Self {
            command: Command::Configure { owner: None },
        })
    }
}

/// Why the command line could not be understood.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandLineError {
    /// A token matched no switch.
    UnknownArgument(String),
    /// `/p` was not followed by a window handle.
    MissingWindowHandle,
    /// The window handle was not a non-zero number.
    InvalidWindowHandle {
        /// The text that failed to parse.
        value: String,
    },
    /// `--width` or `--height` was not followed by a value.
    MissingValue {
        /// The switch that lacks a value.
        flag: &'static str,
    },
    /// `--width` or `--height` did not name a usable size.
    InvalidSize {
        /// The switch that was given.
        flag: &'static str,
        /// The text that failed to parse.
        value: String,
    },
    /// More than one command was given.
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
            Self::MissingWindowHandle => write!(formatter, "'/p' needs a window handle"),
            Self::InvalidWindowHandle { value } => {
                write!(formatter, "'{value}' is not a window handle")
            }
            Self::MissingValue { flag } => write!(formatter, "'{flag}' needs a value"),
            Self::InvalidSize { flag, value } => {
                write!(
                    formatter,
                    "'{value}' is not a usable '{flag}' (1 to {MAXIMUM_SIZE})"
                )
            }
            Self::ConflictingCommands => {
                write!(
                    formatter,
                    "only one of /s, /c, /p and --self-test may be given"
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
    fn no_arguments_opens_the_settings() {
        assert_eq!(parse(&[]), Ok(Command::Configure { owner: None }));
    }

    #[test]
    fn the_run_switch_is_case_insensitive() {
        assert_eq!(parse(&["/s"]), Ok(Command::Run));
        assert_eq!(parse(&["/S"]), Ok(Command::Run));
    }

    #[test]
    fn configure_accepts_the_owner_every_documented_way() {
        let owner = Some(WindowHandle(1234));
        assert_eq!(parse(&["/c"]), Ok(Command::Configure { owner: None }));
        assert_eq!(parse(&["/c:1234"]), Ok(Command::Configure { owner }));
        assert_eq!(parse(&["/C:1234"]), Ok(Command::Configure { owner }));
        assert_eq!(parse(&["/c", "1234"]), Ok(Command::Configure { owner }));
    }

    #[test]
    fn preview_accepts_the_parent_every_documented_way() {
        let parent = WindowHandle(4660);
        assert_eq!(parse(&["/p", "4660"]), Ok(Command::Preview { parent }));
        assert_eq!(parse(&["/p:4660"]), Ok(Command::Preview { parent }));
        assert_eq!(parse(&["/P", "0x1234"]), Ok(Command::Preview { parent }));
    }

    #[test]
    fn preview_requires_a_handle() {
        assert_eq!(parse(&["/p"]), Err(CommandLineError::MissingWindowHandle));
        assert_eq!(
            parse(&["/p", "not-a-handle"]),
            Err(CommandLineError::InvalidWindowHandle {
                value: "not-a-handle".to_owned()
            })
        );
        assert_eq!(
            parse(&["/p:0"]),
            Err(CommandLineError::InvalidWindowHandle {
                value: "/p:0".to_owned()
            })
        );
    }

    #[test]
    fn a_handle_never_swallows_a_following_switch() {
        // `/c` with no owner, then `/s`: the number parser must not treat the
        // switch as a handle, and the two commands still conflict.
        assert_eq!(
            parse(&["/c", "/s"]),
            Err(CommandLineError::ConflictingCommands)
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
            parse(&["/s", "--width", "320"]),
            Err(CommandLineError::SelfTestOnlyArgument("--width".to_owned()))
        );
    }

    #[test]
    fn commands_do_not_combine() {
        assert_eq!(
            parse(&["/s", "/c"]),
            Err(CommandLineError::ConflictingCommands)
        );
        assert_eq!(
            parse(&["/s", "--self-test"]),
            Err(CommandLineError::ConflictingCommands)
        );
    }

    #[test]
    fn help_wins_over_everything_else() {
        assert_eq!(parse(&["--help"]), Ok(Command::Help));
        assert_eq!(parse(&["-h"]), Ok(Command::Help));
        assert_eq!(parse(&["/?"]), Ok(Command::Help));
        assert_eq!(parse(&["/s", "--help"]), Ok(Command::Help));
    }

    #[test]
    fn unknown_arguments_are_reported() {
        assert_eq!(
            parse(&["/x"]),
            Err(CommandLineError::UnknownArgument("/x".to_owned()))
        );
        assert_eq!(
            parse(&["--frames", "10"]),
            Err(CommandLineError::UnknownArgument("--frames".to_owned()))
        );
    }

    #[test]
    fn handles_parse_as_decimal_or_hexadecimal() {
        assert_eq!(WindowHandle::parse("1234"), Some(WindowHandle(1234)));
        assert_eq!(WindowHandle::parse("0x4d2"), Some(WindowHandle(1234)));
        assert_eq!(WindowHandle::parse("0"), None);
        assert_eq!(WindowHandle::parse("0x"), None);
        assert_eq!(WindowHandle::parse(""), None);
    }
}
