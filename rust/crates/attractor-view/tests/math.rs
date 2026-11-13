//! Vector and view-matrix math.

use attractor_view::{Mat4, Vector3};

const TOLERANCE: f64 = 1e-6;
const MATRIX_TOLERANCE: f32 = 1e-6;

fn assert_close(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() <= TOLERANCE,
        "{actual} is not within {TOLERANCE} of {expected}"
    );
}

fn assert_vector_close(actual: Vector3, expected: Vector3) {
    assert_close(actual.x, expected.x);
    assert_close(actual.y, expected.y);
    assert_close(actual.z, expected.z);
}

#[test]
fn length_and_distance_follow_euclidean_geometry() {
    let vector = Vector3::new(3.0, 4.0, 12.0);
    assert_close(vector.length(), 13.0);
    assert_close(Vector3::ZERO.distance(vector), 13.0);
    assert_close(vector.distance(Vector3::ZERO), 13.0);
}

#[test]
fn normalized_divides_by_the_length() {
    let normalized = Vector3::new(0.0, 0.0, 2.0).normalized();
    assert_vector_close(normalized, Vector3::new(0.0, 0.0, 1.0));
    assert_close(normalized.length(), 1.0);
}

#[test]
fn normalized_falls_back_to_up_for_a_degenerate_vector() {
    // The Swift renderer treats a vector shorter than 1e-9 as degenerate and
    // returns the world up axis, so the ribbon still has a usable side.
    assert_eq!(Vector3::ZERO.normalized(), Vector3::UP);
    assert_eq!(Vector3::new(1e-10, 0.0, 0.0).normalized(), Vector3::UP);
}

#[test]
fn dot_product_multiplies_matching_components() {
    let left = Vector3::new(1.0, 2.0, 3.0);
    let right = Vector3::new(-4.0, 5.0, 0.5);
    assert_close(left.dot(right), -4.0 + 10.0 + 1.5);
}

#[test]
fn cross_product_is_right_handed() {
    let x = Vector3::new(1.0, 0.0, 0.0);
    let y = Vector3::new(0.0, 1.0, 0.0);
    assert_vector_close(x.cross(y), Vector3::new(0.0, 0.0, 1.0));
    assert_vector_close(y.cross(x), Vector3::new(0.0, 0.0, -1.0));
}

#[test]
fn lerp_moves_between_the_endpoints() {
    let start = Vector3::new(1.0, 2.0, 3.0);
    let end = Vector3::new(5.0, 6.0, 7.0);
    assert_vector_close(start.lerp(end, 0.0), start);
    assert_vector_close(start.lerp(end, 1.0), end);
    assert_vector_close(start.lerp(end, 0.25), Vector3::new(2.0, 3.0, 4.0));
}

#[test]
fn rounded_to_f32_matches_the_scenekit_storage_grid() {
    let rounded = Vector3::new(0.1, 1.0 / 3.0, 123_456_789.123).rounded_to_f32();
    assert_eq!(rounded.x, f64::from(0.1f32));
    assert_eq!(rounded.y, f64::from((1.0f64 / 3.0) as f32));
    assert_eq!(rounded.z, f64::from(123_456_789.123f32));
}

#[test]
fn look_at_is_orthonormal() {
    let eye = Vector3::new(3.0, 4.0, 5.0);
    let center = Vector3::new(0.5, -1.0, 2.0);
    let matrix = Mat4::look_at(eye, center, Vector3::UP);

    let columns: Vec<[f32; 4]> = (0..3).map(|index| matrix.column(index)).collect();
    for column in &columns {
        let length = (column[0] * column[0] + column[1] * column[1] + column[2] * column[2]).sqrt();
        assert!(
            (length - 1.0).abs() <= MATRIX_TOLERANCE,
            "column length {length}"
        );
        assert_eq!(column[3], 0.0);
    }
    for (left, right) in [(0, 1), (0, 2), (1, 2)] {
        let dot = columns[left][0] * columns[right][0]
            + columns[left][1] * columns[right][1]
            + columns[left][2] * columns[right][2];
        assert!(
            dot.abs() <= MATRIX_TOLERANCE,
            "columns {left}/{right} dot {dot}"
        );
    }
}

#[test]
fn look_at_puts_the_camera_at_the_origin_and_the_center_in_front() {
    let eye = Vector3::new(3.0, 4.0, 5.0);
    let center = Vector3::new(0.5, -1.0, 2.0);
    let distance = eye.distance(center);
    let matrix = Mat4::look_at(eye, center, Vector3::UP);

    assert_vector_close(matrix.transform_point(eye), Vector3::ZERO);
    let transformed_center = matrix.transform_point(center);
    assert_close(transformed_center.x, 0.0);
    assert_close(transformed_center.y, 0.0);
    assert_close(transformed_center.z, -distance);
}

#[test]
fn look_at_keeps_the_world_up_axis_above_the_camera() {
    let matrix = Mat4::look_at(Vector3::new(0.0, 0.0, 5.0), Vector3::ZERO, Vector3::UP);
    // A point above the look target projects to positive view-space y.
    let above = matrix.transform_point(Vector3::new(0.0, 1.0, 0.0));
    assert!(above.y > 0.0);
}

/// Project a world point through `matrix` and divide by `w`, like the GPU.
fn project_ndc(matrix: &Mat4, point: Vector3) -> Vector3 {
    let clip = matrix.project(point);
    assert!(clip[3].abs() > f32::EPSILON, "point is at the camera plane");
    Vector3::new(
        f64::from(clip[0] / clip[3]),
        f64::from(clip[1] / clip[3]),
        f64::from(clip[2] / clip[3]),
    )
}

#[test]
fn perspective_maps_the_frustum_to_the_unit_cube() {
    // A 90-degree vertical field of view makes the projection constants 1.
    let projection = Mat4::perspective(std::f64::consts::FRAC_PI_2, 1.0, 1.0, 10.0);

    let near_center = project_ndc(&projection, Vector3::new(0.0, 0.0, -1.0));
    assert_vector_close(near_center, Vector3::new(0.0, 0.0, 0.0));

    let far_center = project_ndc(&projection, Vector3::new(0.0, 0.0, -10.0));
    assert_vector_close(far_center, Vector3::new(0.0, 0.0, 1.0));

    // The frustum edge at the near plane lands exactly on the NDC boundary.
    let edge = project_ndc(&projection, Vector3::new(1.0, 1.0, -1.0));
    assert_close(edge.x, 1.0);
    assert_close(edge.y, 1.0);
}

#[test]
fn perspective_divides_the_horizontal_axis_by_the_aspect_ratio() {
    let projection = Mat4::perspective(std::f64::consts::FRAC_PI_2, 2.0, 1.0, 10.0);
    // x_clip = x / aspect, so the same point moves halfway to the centre.
    let edge = project_ndc(&projection, Vector3::new(1.0, 0.0, -1.0));
    assert_close(edge.x, 0.5);
}

#[test]
fn perspective_puts_points_behind_the_camera_outside_the_frustum() {
    let projection = Mat4::perspective(std::f64::consts::FRAC_PI_2, 1.0, 1.0, 10.0);
    let behind = projection.project(Vector3::new(0.0, 0.0, 1.0));
    assert!(behind[3] < 0.0, "w must be negative behind the camera");
}

#[test]
fn perspective_falls_back_to_identity_for_degenerate_parameters() {
    let identity = Mat4::identity();
    for projection in [
        Mat4::perspective(0.0, 1.0, 1.0, 10.0),
        Mat4::perspective(std::f64::consts::FRAC_PI_2, 0.0, 1.0, 10.0),
        Mat4::perspective(std::f64::consts::FRAC_PI_2, 1.0, 0.0, 10.0),
        Mat4::perspective(std::f64::consts::FRAC_PI_2, 1.0, 1.0, 1.0),
    ] {
        assert_eq!(projection, identity);
    }
}

#[test]
fn multiplication_applies_the_right_hand_side_first() {
    let view = Mat4::look_at(Vector3::new(0.0, 0.0, 5.0), Vector3::ZERO, Vector3::UP);
    let projection = Mat4::perspective(std::f64::consts::FRAC_PI_2, 1.0, 1.0, 10.0);
    let view_projection = projection.multiply(&view);

    // The look target becomes the centre of the frame.
    let center = project_ndc(&view_projection, Vector3::ZERO);
    assert_close(center.x, 0.0);
    assert_close(center.y, 0.0);
    // The camera sits at its near plane only if it is 5 units away; with
    // near=1, far=10 the target's depth maps to a value strictly inside.
    assert!(center.z > 0.0 && center.z < 1.0, "z {}", center.z);
}

#[test]
fn identity_leaves_points_unchanged() {
    let point = Vector3::new(1.5, -2.5, 3.5);
    assert_vector_close(Mat4::identity().transform_point(point), point);
}
