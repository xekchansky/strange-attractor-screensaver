//! Linux shells for the strange attractor screen saver.
//!
//! The binary serves two audiences. Under xscreensaver it is a hack: the
//! screen saver starts it with `-root` or `-window-id N` and the process draws
//! until the user touches the keyboard or the mouse. On Wayland, which has no
//! screen saver API, the same binary is a fullscreen runner that idle hooks
//! (`swayidle`, `hypridle`) and a `.desktop` launcher start with
//! `--fullscreen`.
//!
//! Argument parsing and the per-display runtime are platform-neutral so they
//! stay testable on every development OS; the window backends — raw Xlib for
//! the xscreensaver protocol and winit for the fullscreen runner — only build
//! on Linux.

pub mod args;
pub mod error;
#[cfg(target_os = "linux")]
pub mod runner;
pub mod session;
#[cfg(target_os = "linux")]
pub mod x11;

pub use args::{Command, CommandLine, CommandLineError, HackWindow};
pub use attractor_shell::{
    SelfTestOutcome, SelfTestRequest, Settings, SettingsError, run_self_test,
};
pub use error::ShellError;
pub use session::{Display, SessionError};
