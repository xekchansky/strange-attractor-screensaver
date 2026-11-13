//! Platform-neutral pieces every screen saver shell shares.
//!
//! The Windows `.scr`, the macOS `.saver` and the Linux shells differ only in
//! how the platform hands them a window and a command line. Everything else —
//! which attractor to run, when a session ends, what the settings file means
//! and the offscreen self-test — is the same on every OS, so it lives here
//! instead of being copied into each shell.
//!
//! The modules are deliberately free of platform APIs: [`settings`] resolves
//! the configuration directory per OS, but takes the environment values as
//! arguments in its testable core; [`input`] is a pure decision function; and
//! [`self_test`] renders through `attractor-player` without a desktop.

pub mod input;
pub mod selection;
pub mod self_test;
pub mod settings;

pub use input::{InputEvent, POINTER_GRACE, ends_session};
pub use selection::{SelectionError, choose, validated_catalog};
pub use self_test::{
    SelfTestBackend, SelfTestError, SelfTestOutcome, SelfTestReport, SelfTestRequest, run_self_test,
};
pub use settings::{Settings, SettingsError};
