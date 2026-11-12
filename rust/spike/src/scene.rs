//! Throwaway geometry: a Lorenz trajectory turned into a colored ribbon.
//!
//! This is spike code. The real solver, trace buffer and ribbon builder land
//! in `attractor-core` / `attractor-view` (I3–I5); here it exists only to
//! give the renderer 10k segments of realistic, non-degenerate geometry.

use bytemuck::{Pod, Zeroable};

pub const SEGMENTS: usize = 10_000;
pub const STEP: f32 = 0.004;

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct Vertex {
    pub position: [f32; 3],
    pub color: [f32; 3],
}

impl Vertex {
    pub const ATTRIBUTES: [wgpu::VertexAttribute; 2] =
        wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3];

    pub fn layout() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBUTES,
        }
    }
}

fn lorenz_derivative(p: [f32; 3]) -> [f32; 3] {
    const SIGMA: f32 = 10.0;
    const RHO: f32 = 28.0;
    const BETA: f32 = 8.0 / 3.0;
    [
        SIGMA * (p[1] - p[0]),
        p[0] * (RHO - p[2]) - p[1],
        p[0] * p[1] - BETA * p[2],
    ]
}

fn rk4_step(p: [f32; 3], dt: f32) -> [f32; 3] {
    let add =
        |a: [f32; 3], b: [f32; 3], s: f32| [a[0] + b[0] * s, a[1] + b[1] * s, a[2] + b[2] * s];
    let k1 = lorenz_derivative(p);
    let k2 = lorenz_derivative(add(p, k1, dt * 0.5));
    let k3 = lorenz_derivative(add(p, k2, dt * 0.5));
    let k4 = lorenz_derivative(add(p, k3, dt));
    [
        p[0] + dt / 6.0 * (k1[0] + 2.0 * k2[0] + 2.0 * k3[0] + k4[0]),
        p[1] + dt / 6.0 * (k1[1] + 2.0 * k2[1] + 2.0 * k3[1] + k4[1]),
        p[2] + dt / 6.0 * (k1[2] + 2.0 * k2[2] + 2.0 * k3[2] + k4[2]),
    ]
}

/// `count` points of a Lorenz orbit, already centered on its bounding box.
pub fn lorenz_points(count: usize) -> Vec<[f32; 3]> {
    let mut points = Vec::with_capacity(count);
    let mut p = [0.1f32, 0.0, 0.0];
    // Discard the transient so the ribbon fills the frame immediately.
    for _ in 0..2000 {
        p = rk4_step(p, STEP);
    }
    let mut min = [f32::INFINITY; 3];
    let mut max = [f32::NEG_INFINITY; 3];
    for _ in 0..count {
        p = rk4_step(p, STEP);
        points.push(p);
        for axis in 0..3 {
            min[axis] = min[axis].min(p[axis]);
            max[axis] = max[axis].max(p[axis]);
        }
    }
    let center = [
        (min[0] + max[0]) * 0.5,
        (min[1] + max[1]) * 0.5,
        (min[2] + max[2]) * 0.5,
    ];
    for p in &mut points {
        for axis in 0..3 {
            p[axis] -= center[axis];
        }
    }
    points
}

/// Hue ramp matching the colorful look of the current saver.
fn ramp(t: f32) -> [f32; 3] {
    let h = (t * 0.85 + 0.55) % 1.0;
    let s = 1.0;
    let v = 1.0;
    let i = (h * 6.0).floor();
    let f = h * 6.0 - i;
    let p = v * (1.0 - s);
    let q = v * (1.0 - s * f);
    let t2 = v * (1.0 - s * (1.0 - f));
    match i as i32 % 6 {
        0 => [v, t2, p],
        1 => [q, v, p],
        2 => [p, v, t2],
        3 => [p, q, v],
        4 => [t2, p, v],
        _ => [v, p, q],
    }
}

/// Bounding radius of the orbit, used as the world-space ribbon scale.
pub fn bounding_radius(points: &[[f32; 3]]) -> f32 {
    points
        .iter()
        .map(|p| (p[0] * p[0] + p[1] * p[1] + p[2] * p[2]).sqrt())
        .fold(0.0, f32::max)
}

/// Two vertices per point, drawn as a triangle strip.
pub fn build_ribbon(points: &[[f32; 3]], radius: f32) -> Vec<Vertex> {
    let mut vertices = Vec::with_capacity(points.len() * 2);
    let last = points.len().saturating_sub(1);
    for (i, point) in points.iter().enumerate() {
        let prev = points[i.saturating_sub(1)];
        let next = points[(i + 1).min(last)];
        let tangent = [next[0] - prev[0], next[1] - prev[1], next[2] - prev[2]];
        let tangent_len =
            (tangent[0] * tangent[0] + tangent[1] * tangent[1] + tangent[2] * tangent[2]).sqrt();
        let tangent = if tangent_len > 0.0 {
            [
                tangent[0] / tangent_len,
                tangent[1] / tangent_len,
                tangent[2] / tangent_len,
            ]
        } else {
            [1.0, 0.0, 0.0]
        };
        // Ribbon plane: perpendicular to the tangent, near the world "up" axis.
        let mut side = [
            tangent[1] * 0.0 - tangent[2] * 1.0,
            tangent[2] * 0.0 - tangent[0] * 0.0,
            tangent[0] * 1.0 - tangent[1] * 0.0,
        ];
        let side_len = (side[0] * side[0] + side[1] * side[1] + side[2] * side[2]).sqrt();
        if side_len < 1e-4 {
            side = [1.0, 0.0, 0.0];
        } else {
            side = [side[0] / side_len, side[1] / side_len, side[2] / side_len];
        }
        let t = if points.len() > 1 {
            i as f32 / (points.len() - 1) as f32
        } else {
            0.0
        };
        let color = ramp(t);
        vertices.push(Vertex {
            position: [
                point[0] + side[0] * radius,
                point[1] + side[1] * radius,
                point[2] + side[2] * radius,
            ],
            color,
        });
        vertices.push(Vertex {
            position: [
                point[0] - side[0] * radius,
                point[1] - side[1] * radius,
                point[2] - side[2] * radius,
            ],
            color,
        });
    }
    vertices
}

#[cfg(test)]
mod tests {
    use super::*;

    fn orbit() -> Vec<[f32; 3]> {
        lorenz_points(SEGMENTS + 1)
    }

    #[test]
    fn every_orbit_point_is_finite() {
        let points = orbit();
        assert_eq!(points.len(), SEGMENTS + 1);
        assert!(points.iter().flatten().all(|c| c.is_finite()));
    }

    #[test]
    fn ribbon_has_two_vertices_per_point() {
        let points = orbit();
        let vertices = build_ribbon(&points, 0.05);
        assert_eq!(vertices.len(), (SEGMENTS + 1) * 2);
        assert!(
            vertices
                .iter()
                .flat_map(|v| v.position)
                .all(|c| c.is_finite())
        );
    }

    #[test]
    fn head_of_the_ribbon_is_present() {
        let points = orbit();
        let vertices = build_ribbon(&points, 0.05);
        let head = points.last().unwrap();
        let (left, right) = (vertices[vertices.len() - 2], vertices[vertices.len() - 1]);
        assert!((left.position[0] - head[0]).abs() < 0.1);
        assert!((right.position[0] - head[0]).abs() < 0.1);
    }

    #[test]
    fn colors_vary_along_the_trail() {
        let points = orbit();
        let vertices = build_ribbon(&points, 0.05);
        assert_ne!(vertices[0].color, vertices[vertices.len() / 2].color);
        assert_ne!(vertices[0].color, vertices[vertices.len() - 1].color);
    }
}
