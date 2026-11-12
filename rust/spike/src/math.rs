//! Minimal column-major matrix helpers for the spike.
//!
//! `Mat4` is `[[f32; 4]; 4]` indexed as `m[column][row]`, the layout WGSL
//! expects for a `mat4x4<f32>` uniform.

pub type Mat4 = [[f32; 4]; 4];
pub type Vec3 = [f32; 3];

pub fn sub(a: Vec3, b: Vec3) -> Vec3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

pub fn dot(a: Vec3, b: Vec3) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

pub fn cross(a: Vec3, b: Vec3) -> Vec3 {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

pub fn normalize(v: Vec3) -> Vec3 {
    let len = dot(v, v).sqrt();
    if len == 0.0 {
        return [0.0, 0.0, 0.0];
    }
    [v[0] / len, v[1] / len, v[2] / len]
}

/// Right-handed perspective projection with a `0..1` depth range (wgpu).
pub fn perspective(fov_y_rad: f32, aspect: f32, z_near: f32, z_far: f32) -> Mat4 {
    let f = 1.0 / (fov_y_rad * 0.5).tan();
    let mut m = [[0.0; 4]; 4];
    m[0][0] = f / aspect;
    m[1][1] = f;
    m[2][2] = z_far / (z_near - z_far);
    m[2][3] = -1.0;
    m[3][2] = z_near * z_far / (z_near - z_far);
    m
}

pub fn look_at(eye: Vec3, center: Vec3, up: Vec3) -> Mat4 {
    let forward = normalize(sub(center, eye));
    let side = normalize(cross(forward, up));
    let up = cross(side, forward);
    [
        [side[0], up[0], -forward[0], 0.0],
        [side[1], up[1], -forward[1], 0.0],
        [side[2], up[2], -forward[2], 0.0],
        [-dot(side, eye), -dot(up, eye), dot(forward, eye), 1.0],
    ]
}

/// `a * b` in column-major order.
pub fn mul(a: Mat4, b: Mat4) -> Mat4 {
    let mut out = [[0.0; 4]; 4];
    for (col, out_col) in out.iter_mut().enumerate() {
        for (row, value) in out_col.iter_mut().enumerate() {
            *value = (0..4).map(|k| a[k][row] * b[col][k]).sum();
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity_look_at_keeps_origin_in_front() {
        let view = look_at([0.0, 0.0, 5.0], [0.0, 0.0, 0.0], [0.0, 1.0, 0.0]);
        let point = [0.0, 0.0, 0.0];
        let x = view[0][0] * point[0] + view[1][0] * point[1] + view[2][0] * point[2] + view[3][0];
        let z = view[0][2] * point[0] + view[1][2] * point[1] + view[2][2] * point[2] + view[3][2];
        assert!(x.abs() < 1e-6);
        assert!((z + 5.0).abs() < 1e-6, "camera sits 5 units behind: {z}");
    }

    #[test]
    fn perspective_maps_near_to_zero_and_far_to_one() {
        let p = perspective(std::f32::consts::FRAC_PI_3, 2.0, 0.1, 100.0);
        let project = |z: f32| {
            let clip_z = p[2][2] * z + p[3][2];
            let clip_w = p[2][3] * z;
            clip_z / clip_w
        };
        assert!((project(-0.1) - 0.0).abs() < 1e-5);
        assert!((project(-100.0) - 1.0).abs() < 1e-5);
    }
}
