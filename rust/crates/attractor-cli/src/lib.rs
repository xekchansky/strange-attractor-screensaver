//! Cross-platform demo application with screenshot and recording capture.
//!
//! The crate resolves command-line flags against the catalog, runs the
//! deterministic headless simulation and renders the requested frames through
//! `attractor-gpu`. The simulation loop is `attractor_player::Player`, the
//! same runtime the macOS, Windows, Linux and Android shells drive.

mod arguments;
mod run;

pub use arguments::{
    CaptureRequest, CommandLineArguments, CommandLineArgumentsError, SimulationOverrides,
};
pub use run::{
    CaptureReport, DEFAULT_HEIGHT, DEFAULT_SETTLE_FRAMES, DEFAULT_WIDTH, MAXIMUM_PIXEL_DIMENSION,
    PreparedRun, RunError, prepare, record_sequence, run,
};

#[cfg(test)]
mod tests {
    #[test]
    fn crate_is_configured() {
        assert_eq!(env!("CARGO_PKG_NAME"), "attractor-cli");
    }
}
