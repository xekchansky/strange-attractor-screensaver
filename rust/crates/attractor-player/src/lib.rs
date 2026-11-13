//! One reusable runtime for every shell: engine, view and GPU surface.
//!
//! The player owns the simulation loop and the offscreen renderer, so the
//! macOS `.saver`, the Windows `.scr`, the Linux runner, the Android live
//! wallpaper and the web demo all drive the same code. It runs one instance
//! per surface: every display gets its own player, its own random selection
//! and its own camera.

mod error;
mod player;

pub use error::PlayerError;
pub use player::{Player, PlayerOptions, RestartOutcome, TickEvent};
