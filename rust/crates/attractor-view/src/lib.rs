//! GPU-free camera math, trail geometry, color ramps and overlay model.
//!
//! Everything here is deterministic and free of platform APIs: the wgpu
//! renderer in `attractor-gpu` only uploads the buffers this crate produces.
//! The Swift oracle (`CameraController` and `TraceRenderer`, archived at the
//! git tag `swift-oracle`) is
//! reproduced bit for bit, including its `f32` storage rounding; the golden
//! fixture in `tests/fixtures/view_fixtures.json` pins both.

mod camera;
mod math;
mod overlay;
mod trail;
mod vector;

pub use camera::CameraController;
pub use math::Mat4;
pub use overlay::OverlayModel;
pub use trail::{Rgb, TrailBuilder, TrailGeometry};
pub use vector::Vector3;
