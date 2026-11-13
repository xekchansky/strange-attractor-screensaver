//! One display: a player and the swap chain it presents into.
//!
//! Both Linux backends — the xscreensaver hack and the winit fullscreen
//! runner — own one [`Display`] per surface and drive it with [`Display::draw`]
//! on a 60 Hz cadence. The simulation, camera, ribbon and overlay stay inside
//! `attractor-player`; this type only wires them to a window and a swap chain,
//! exactly as the Windows shell does.

use std::error::Error;
use std::fmt;
use std::time::{SystemTime, UNIX_EPOCH};

use attractor_core::{AttractorDescriptor, SplitMix64};
use attractor_gpu::{GpuContext, SurfaceTarget};
use attractor_player::{Player, PlayerOptions};
use attractor_shell::{Settings, selection};

/// A failure while creating or driving a display.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionError {
    message: String,
}

impl SessionError {
    /// Build an error with a human-readable message.
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for SessionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Error for SessionError {}

/// One display's player and swap chain.
pub struct Display {
    player: Player,
    target: SurfaceTarget,
    width: u32,
    height: u32,
    reporting_failure: bool,
}

impl Display {
    /// Build the player for an existing swap chain.
    ///
    /// `seed` seeds the attractor, its parameters and the camera, so two
    /// displays of one session draw different attractors.
    pub fn new(
        target: SurfaceTarget,
        width: u32,
        height: u32,
        overlay_scale: f32,
        settings: &Settings,
        catalog: &[AttractorDescriptor],
        seed: u64,
    ) -> Result<Self, SessionError> {
        if width == 0 || height == 0 {
            return Err(SessionError::new("a display has no pixels"));
        }
        let mut rng = SplitMix64::new(seed);
        let selection = selection::choose(settings, catalog, &mut rng)
            .map_err(|error| SessionError::new(error.to_string()))?;
        let options = PlayerOptions {
            width,
            height,
            overlay_scale,
            ..PlayerOptions::default()
        };
        let player = Player::new(selection, catalog.to_vec(), options, rng)
            .map_err(|error| SessionError::new(error.to_string()))?;
        Ok(Self {
            player,
            target,
            width,
            height,
            reporting_failure: false,
        })
    }

    /// Advance this display by `dt` seconds and present the frame.
    ///
    /// A surface that cannot hand out a frame (a timeout, an occluded window)
    /// is not an error; the next tick tries again.
    pub fn draw(&mut self, context: &GpuContext, dt: f64) {
        if self.width == 0 || self.height == 0 {
            return;
        }
        let result = match self.player.tick(dt) {
            Ok(_) => self.player.render_to_surface(context, &mut self.target),
            Err(error) => Err(error),
        };
        match result {
            Ok(()) => self.reporting_failure = false,
            Err(error) => {
                if !self.reporting_failure {
                    log::warn!("cannot draw the attractor: {error}");
                    self.reporting_failure = true;
                }
            }
        }
    }

    /// Retarget the player after a window resize.
    ///
    /// The swap chain catches up on the next [`Display::draw`], so a burst of
    /// configure events costs nothing until a frame is actually drawn.
    pub fn resize(&mut self, width: u32, height: u32, overlay_scale: f32) {
        if width == 0 || height == 0 {
            self.width = width;
            self.height = height;
            return;
        }
        self.width = width;
        self.height = height;
        if let Err(error) = self.player.resize(width, height, overlay_scale) {
            log::warn!("cannot resize the attractor player: {error}");
        }
    }
}

/// A seed that differs between launches, so every start picks a new attractor.
pub fn time_seed() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_nanos() as u64)
        .unwrap_or(0x5EED)
}

/// The seed of one display in a multi-monitor session.
///
/// Mixing the display index keeps every monitor on a different attractor
/// while one launch stays a single seed.
pub fn seed_for(seed: u64, index: usize) -> u64 {
    seed ^ (index as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_display_gets_a_different_seed() {
        let seed = 42;
        let seeds: Vec<u64> = (0..4).map(|index| seed_for(seed, index)).collect();
        assert_eq!(seeds[0], seed);
        for (index, first) in seeds.iter().enumerate() {
            for second in seeds.iter().skip(index + 1) {
                assert_ne!(first, second);
            }
        }
    }

    #[test]
    fn a_display_seed_is_stable() {
        assert_eq!(seed_for(42, 2), seed_for(42, 2));
        assert_eq!(seed_for(42, 1), 42 ^ 0x9E37_79B9_7F4A_7C15);
    }
}
