//! Windows `.scr` shell for the strange attractor screen saver.
//!
//! The crate is split so that everything testable lives outside the Win32
//! layer: [`args`] parses the `/s`, `/c` and `/p` protocol plus the saver's own
//! `--self-test` switches, [`settings`] persists the chosen attractor under
//! `%APPDATA%`, [`selection`] turns a settings file into a player selection,
//! [`input`] decides when a session ends, and [`self_test`] renders a frame
//! without a desktop. The [`windows`] module is the thin Win32 shell that owns
//! the windows, the message loop and the per-monitor players; it only builds
//! on Windows.
//!
//! The binary is the `.scr` executable itself: Windows runs it with `/s`,
//! `/c` and `/p`, and a release build is renamed to `StrangeAttractor.scr`.

pub mod args;
pub mod input;
pub mod selection;
pub mod self_test;
pub mod settings;
pub mod text;
#[cfg(windows)]
pub mod windows;

pub use args::{Command, CommandLine, CommandLineError, WindowHandle};
pub use selection::{SelectionError, choose, validated_catalog};
pub use self_test::{
    SelfTestBackend, SelfTestError, SelfTestOutcome, SelfTestReport, SelfTestRequest, run_self_test,
};
pub use settings::{Settings, SettingsError};
