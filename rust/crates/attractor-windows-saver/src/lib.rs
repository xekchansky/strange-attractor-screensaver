//! Windows `.scr` shell for the strange attractor screen saver.
//!
//! The crate is split so that everything testable lives outside the Win32
//! layer: [`args`] parses the `/s`, `/c` and `/p` protocol plus the saver's own
//! `--self-test` switches, and the [`windows`] module is the thin Win32 shell
//! that owns the windows, the message loop and the per-monitor players; it only
//! builds on Windows. The settings file, the attractor selection, the
//! input-policy and the offscreen self-test are shared with the other shells
//! through `attractor-shell`.
//!
//! The binary is the `.scr` executable itself: Windows runs it with `/s`,
//! `/c` and `/p`, and a release build is renamed to `StrangeAttractor.scr`.

pub mod args;
pub mod text;
#[cfg(windows)]
pub mod windows;

pub use args::{Command, CommandLine, CommandLineError, WindowHandle};
pub use attractor_shell::{
    SelectionError, SelfTestOutcome, SelfTestRequest, Settings, SettingsError, choose,
    run_self_test, validated_catalog,
};
