//! Cross-platform demo application with screenshot and recording capture.
//!
//! The crate resolves command-line flags against the catalog, runs the
//! deterministic headless simulation and renders the requested frames through
//! `attractor-gpu`. Shells (the macOS saver, the Windows `.scr`, the Linux
//! runner) will reuse `SimulationSession` through `attractor-player` in I7+.

mod arguments;
mod run;
mod session;

pub use arguments::{
    CaptureRequest, CommandLineArguments, CommandLineArgumentsError, SimulationOverrides,
};
pub use run::{
    CaptureReport, DEFAULT_HEIGHT, DEFAULT_SETTLE_FRAMES, DEFAULT_WIDTH, MAXIMUM_PIXEL_DIMENSION,
    PreparedRun, RunError, prepare, record_sequence, run,
};
pub use session::{SessionError, SimulationSession};

#[cfg(test)]
mod tests {
    #[test]
    fn crate_is_configured() {
        assert_eq!(env!("CARGO_PKG_NAME"), "attractor-cli");
    }
}
