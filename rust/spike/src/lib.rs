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

/// Builds the spike's vertex buffer and returns it with the scene radius.
pub fn build_scene() -> (Vec<scene::Vertex>, f32) {
    let points = scene::lorenz_points(scene::SEGMENTS + 1);
    let radius = scene::bounding_radius(&points);
    let vertices = scene::build_ribbon(&points, radius * 0.004);
    (vertices, radius)
}
