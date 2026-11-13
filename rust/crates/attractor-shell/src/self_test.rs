//! One offscreen frame, for CI and for a quick install check.
//!
//! A runner without a desktop cannot compare pixels, but it can prove that the
//! same player, pipelines and catalog the shell uses render ink. The frame is
//! drawn through the real GPU backend when one is available (Metal on macOS,
//! D3D12/WARP on Windows, Vulkan/llvmpipe on CI Linux) and through wgpu's
//! `noop` backend when `--noop` is passed, which validates the pipeline on a
//! machine without a GPU.

use std::error::Error;
use std::fmt;

use attractor_core::SplitMix64;
use attractor_gpu::{GpuContext, GpuError, lit_pixels};
use attractor_player::{Player, PlayerError, PlayerOptions};

use crate::selection::{self, SelectionError};
use crate::settings::Settings;

/// Number of frames the self-test renders after the warm-up.
pub const FRAMES: usize = 60;
/// Integration steps before the first frame, the CLI snapshot's warm-up.
pub const WARM_UP_STEPS: usize = 5_000;
/// Seed of the self-test run, so the report is reproducible.
pub const SEED: u64 = 42;
/// Frame width when `--width` is not given.
pub const DEFAULT_WIDTH: u32 = 640;
/// Frame height when `--height` is not given.
pub const DEFAULT_HEIGHT: u32 = 480;
/// Largest side `--width` and `--height` accept.
pub const MAXIMUM_SIZE: u32 = 8_192;

/// Which wgpu backend the self-test asks for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelfTestBackend {
    /// The best native adapter; the test skips when there is none.
    Auto,
    /// wgpu's noop backend, which draws nothing but validates the pipeline.
    Noop,
}

/// What the self-test should render.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SelfTestRequest {
    /// Frame width in pixels.
    pub width: u32,
    /// Frame height in pixels.
    pub height: u32,
    /// Backend to render through.
    pub backend: SelfTestBackend,
}

impl Default for SelfTestRequest {
    fn default() -> Self {
        Self {
            width: DEFAULT_WIDTH,
            height: DEFAULT_HEIGHT,
            backend: SelfTestBackend::Auto,
        }
    }
}

/// A successful self-test run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelfTestReport {
    /// Adapter name as wgpu reports it.
    pub adapter: String,
    /// wgpu backend name (`Metal`, `Dx12`, `Vulkan`, …).
    pub backend: String,
    /// Frame width in pixels.
    pub width: u32,
    /// Frame height in pixels.
    pub height: u32,
    /// Frames rendered after the warm-up.
    pub frames: usize,
    /// Pixels that differ from the black background.
    pub lit_pixels: usize,
    /// Whether the frame was actually rasterized.
    ///
    /// The noop backend accepts every command and draws nothing, so its report
    /// has zero ink by construction and only proves the pipeline built.
    pub rasterized: bool,
}

/// Result of a self-test attempt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SelfTestOutcome {
    /// The frame was rendered and checked.
    Rendered(SelfTestReport),
    /// No GPU adapter was available; the caller should report a skip.
    Skipped {
        /// Why the test could not run.
        reason: String,
    },
}

/// A self-test failure.
#[derive(Debug)]
pub enum SelfTestError {
    /// The GPU device or renderer failed.
    Gpu(GpuError),
    /// The catalog could not produce a runnable attractor.
    Selection(SelectionError),
    /// The player failed to build or advance.
    Player(PlayerError),
    /// A real adapter drew a completely black frame.
    BlankFrame,
}

impl fmt::Display for SelfTestError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Gpu(error) => write!(formatter, "{error}"),
            Self::Selection(error) => write!(formatter, "{error}"),
            Self::Player(error) => write!(formatter, "{error}"),
            Self::BlankFrame => write!(formatter, "the self-test frame contains no ink"),
        }
    }
}

impl Error for SelfTestError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Gpu(error) => Some(error),
            Self::Selection(error) => Some(error),
            Self::Player(error) => Some(error),
            Self::BlankFrame => None,
        }
    }
}

/// Render a frame through the shared player and report what came out.
///
/// The run is seeded, so a failure is reproducible. The frame is rendered
/// offscreen through [`Player::render`], the same geometry a shell presents to
/// a window.
pub fn run_self_test(request: SelfTestRequest) -> Result<SelfTestOutcome, SelfTestError> {
    if request.width == 0 || request.height == 0 {
        return Err(SelfTestError::Gpu(GpuError::InvalidFrameSize {
            width: request.width,
            height: request.height,
        }));
    }

    let context = match request.backend {
        SelfTestBackend::Auto => match GpuContext::new() {
            Ok(context) => context,
            Err(GpuError::NoAdapter(error)) => {
                return Ok(SelfTestOutcome::Skipped {
                    reason: error.to_string(),
                });
            }
            Err(error) => return Err(SelfTestError::Gpu(error)),
        },
        SelfTestBackend::Noop => GpuContext::new_noop().map_err(SelfTestError::Gpu)?,
    };

    let catalog = selection::validated_catalog().map_err(SelfTestError::Selection)?;
    let mut rng = SplitMix64::new(SEED);
    let selection = selection::choose(&Settings::default(), &catalog, &mut rng)
        .map_err(SelfTestError::Selection)?;
    let options = PlayerOptions {
        width: request.width,
        height: request.height,
        heal_automatically: false,
        ..PlayerOptions::default()
    };
    let mut player =
        Player::new(selection, catalog, options, rng).map_err(SelfTestError::Player)?;
    player
        .warm_up(WARM_UP_STEPS)
        .map_err(SelfTestError::Player)?;
    for _ in 0..FRAMES {
        player.tick(1.0 / 60.0).map_err(SelfTestError::Player)?;
    }
    let pixels = player.render(&context).map_err(SelfTestError::Player)?;

    let info = context.adapter_info();
    let rasterized = request.backend == SelfTestBackend::Auto;
    let report = SelfTestReport {
        adapter: info.name.clone(),
        backend: format!("{:?}", info.backend),
        width: request.width,
        height: request.height,
        frames: FRAMES,
        lit_pixels: lit_pixels(&pixels),
        rasterized,
    };
    if report.rasterized && report.lit_pixels == 0 {
        return Err(SelfTestError::BlankFrame);
    }
    Ok(SelfTestOutcome::Rendered(report))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_noop_backend_validates_the_pipelines() {
        let outcome = run_self_test(SelfTestRequest {
            width: 320,
            height: 240,
            backend: SelfTestBackend::Noop,
        })
        .expect("the noop backend accepts the run");
        match outcome {
            SelfTestOutcome::Rendered(report) => {
                assert!(!report.rasterized);
                assert_eq!(report.lit_pixels, 0);
                assert_eq!(report.frames, FRAMES);
                assert_eq!(report.width, 320);
                assert_eq!(report.height, 240);
            }
            SelfTestOutcome::Skipped { reason } => {
                panic!("the noop backend never skips: {reason}")
            }
        }
    }

    #[test]
    fn a_zero_size_is_rejected_before_the_gpu_is_touched() {
        let error = run_self_test(SelfTestRequest {
            width: 0,
            height: 240,
            backend: SelfTestBackend::Noop,
        })
        .expect_err("a zero width is invalid");
        assert!(matches!(
            error,
            SelfTestError::Gpu(GpuError::InvalidFrameSize { width: 0, .. })
        ));
    }
}
