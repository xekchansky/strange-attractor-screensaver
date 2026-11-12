//! Throwaway wgpu spike for the Rust port (iteration I0.5).
//!
//! It answers one question: can a single wgpu pipeline draw the saver's
//! ribbon on native Metal, in the browser, and compile for Android GLES?
//! Nothing here is meant to survive I6.

pub mod gpu;
pub mod math;
pub mod scene;

#[cfg(not(target_arch = "wasm32"))]
pub mod native;

#[cfg(target_arch = "wasm32")]
mod web;

use math::Mat4;

/// Background color shared by the native and web paths.
pub const CLEAR: wgpu::Color = wgpu::Color {
    r: 0.015,
    g: 0.015,
    b: 0.030,
    a: 1.0,
};

/// Orbit camera identical in spirit to the saver's `CameraController`:
/// elevation is fixed at `pi/5`, the azimuth rotates with time, and the
/// distance is derived from the scene radius (framing factor 2.1).
pub fn camera_matrix(width: u32, height: u32, seconds: f32, scene_radius: f32) -> Mat4 {
    use std::f32::consts::PI;

    let elevation = PI / 5.0;
    let azimuth = seconds * 0.25;
    let distance = scene_radius * 2.1;
    let eye = [
        distance * elevation.cos() * azimuth.sin(),
        distance * elevation.sin(),
        distance * elevation.cos() * azimuth.cos(),
    ];
    let aspect = width as f32 / height.max(1) as f32;
    let projection = math::perspective(PI / 4.0, aspect, 0.01, distance * 10.0);
    let view = math::look_at(eye, [0.0, 0.0, 0.0], [0.0, 1.0, 0.0]);
    math::mul(projection, view)
}

/// Converts animation-frame timestamps into elapsed seconds.
///
/// The first timestamp becomes the origin, so callers do not need a second
/// clock. This matters: `requestAnimationFrame` passes `performance.now()`
/// milliseconds, and subtracting `Date.now()` (epoch milliseconds) from them
/// yields an offset so large that `f32` seconds lose all resolution and the
/// camera appears frozen.
#[derive(Debug, Default)]
pub struct FrameClock {
    origin: Option<f64>,
}

impl FrameClock {
    /// Returns seconds since the first frame, starting at zero.
    pub fn seconds(&mut self, frame_time_ms: f64) -> f32 {
        let origin = *self.origin.get_or_insert(frame_time_ms);
        ((frame_time_ms - origin) / 1000.0) as f32
    }
}

/// Builds the spike's vertex buffer and returns it with the scene radius.
pub fn build_scene() -> (Vec<scene::Vertex>, f32) {
    let points = scene::lorenz_points(scene::SEGMENTS + 1);
    let radius = scene::bounding_radius(&points);
    let vertices = scene::build_ribbon(&points, radius * 0.004);
    (vertices, radius)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_frame_starts_at_zero() {
        let mut clock = FrameClock::default();
        assert_eq!(clock.seconds(1234.5), 0.0);
    }

    #[test]
    fn elapsed_time_follows_the_frame_timestamps() {
        let mut clock = FrameClock::default();
        clock.seconds(1000.0);
        assert!((clock.seconds(1500.0) - 0.5).abs() < 1e-6);
    }

    #[test]
    fn epoch_scale_timestamps_still_advance() {
        // The bug this guards against: `Date.now()` (epoch milliseconds) and
        // the rAF timestamp (`performance.now()`) are different clocks.
        // Mixing them produced a huge negative offset that f32 could not
        // resolve, so the camera never moved.
        let mut clock = FrameClock::default();
        clock.seconds(1_700_000_000_000.0);
        let delta = clock.seconds(1_700_000_000_016.0);
        assert!((delta - 0.016).abs() < 1e-6);
    }
}
