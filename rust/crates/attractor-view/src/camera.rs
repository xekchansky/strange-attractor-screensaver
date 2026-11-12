//! Orbit camera that keeps the attractor framed while it rotates.

use std::f64::consts::PI;

use attractor_core::{RandomSource, TraceBuffer};

use crate::math::Mat4;
use crate::vector::Vector3;

/// Positions a camera on a slowly rotating orbit, continuously re-framing the
/// attractor's current bounding box so it always stays in view.
///
/// Rotation speed is supplied by the caller (it comes from
/// [`SimulationSettings`](attractor_core::SimulationSettings)); the controller
/// owns the orbit state and the smoothing toward the target framing.
///
/// The controller stores its position rounded through `f32`, like SceneKit's
/// node storage, so the golden fixture captured from the Swift oracle matches
/// bit for bit.
#[derive(Debug, Clone, PartialEq)]
pub struct CameraController {
    /// Current orbit azimuth in radians. Kept unbounded; `cos`/`sin` are
    /// periodic.
    azimuth: f64,
    /// Polar angle of the orbit above the horizontal plane.
    elevation: f64,
    /// Current smoothed orbit center (the point the camera looks at).
    center: Vector3,
    /// Current smoothed distance from the center to the camera.
    distance: f64,
    /// Frame counter used to pace the framing refresh.
    frame: u64,
    /// Cached framing target, recomputed at [`Self::FRAMING_STRIDE`] cadence.
    target_center: Vector3,
    target_distance: f64,
    /// Last computed camera position, rounded like SceneKit's node storage.
    position: Vector3,
}

impl CameraController {
    /// Multiplier of the bounding radius used to compute the target distance.
    ///
    /// Kept tight: a larger margin left the attractor small in the frame,
    /// which read poorly both on screen and in small README preview
    /// thumbnails.
    pub const FRAMING_FACTOR: f64 = 2.1;
    /// Minimum camera distance, so the view never zooms inside the attractor.
    pub const MINIMUM_DISTANCE: f64 = 2.0;
    /// How quickly the camera eases toward the target framing each frame.
    pub const SMOOTHING: f64 = 0.06;
    /// Recompute the full framing target (bounds scan and radius) only every
    /// this many frames. The easing already converges toward the target, so a
    /// ~10 Hz refresh is visually indistinguishable from per-frame while
    /// skipping most of the O(n) bounds scan at large trail sizes.
    pub const FRAMING_STRIDE: u64 = 6;
    /// The orbit angle above the horizontal plane used when none is given.
    pub const DEFAULT_ELEVATION: f64 = PI / 5.0;
    /// Distance used until the first framing scan completes.
    const INITIAL_DISTANCE: f64 = 8.0;
    /// Radius floor so a single-point trace still has a finite target.
    const MINIMUM_RADIUS: f64 = 0.1;

    /// Create a controller pinned to an explicit viewing angle.
    ///
    /// Documentation captures pass a fixed azimuth so the resulting view is
    /// reproducible; the screensaver uses [`Self::random`] instead.
    pub fn new(azimuth: f64, elevation: f64) -> Self {
        Self {
            azimuth,
            elevation,
            center: Vector3::ZERO,
            distance: Self::INITIAL_DISTANCE,
            frame: 0,
            target_center: Vector3::ZERO,
            target_distance: Self::INITIAL_DISTANCE,
            position: Vector3::ZERO,
        }
    }

    /// Create a controller starting at a random orbit angle.
    ///
    /// Mirrors Swift's `Double.random(in: 0..<2 * .pi)`: 53 random bits are
    /// mapped into the half-open range, so the angle is drawn from exactly the
    /// same distribution. The elevation keeps [`Self::DEFAULT_ELEVATION`].
    pub fn random(source: &mut impl RandomSource) -> Self {
        const SIGNIFICAND_BITS: u64 = 53;
        let unit = source.next_below(1 << SIGNIFICAND_BITS) as f64 * (f64::EPSILON / 2.0);
        Self::new(unit * (2.0 * PI), Self::DEFAULT_ELEVATION)
    }

    /// Advance the orbit and reframe `trace`.
    ///
    /// `dt` is the elapsed time in seconds and `rotation_speed` the orbit's
    /// angular velocity in radians per second.
    pub fn update(&mut self, dt: f64, rotation_speed: f64, trace: &TraceBuffer) {
        self.frame += 1;
        self.azimuth += rotation_speed * dt;

        if self.frame % Self::FRAMING_STRIDE == 1 {
            (self.target_center, self.target_distance) = Self::framing(trace);
        }
        self.center = self.center.lerp(self.target_center, Self::SMOOTHING);
        self.distance += (self.target_distance - self.distance) * Self::SMOOTHING;

        let cos_elevation = self.elevation.cos();
        self.position = Vector3::new(
            self.center.x + self.distance * cos_elevation * self.azimuth.cos(),
            self.center.y + self.distance * self.elevation.sin(),
            self.center.z + self.distance * cos_elevation * self.azimuth.sin(),
        )
        .rounded_to_f32();
    }

    /// Current camera position in world space.
    ///
    /// Before the first [`Self::update`] this is the origin, matching a fresh
    /// SceneKit camera node.
    pub fn position(&self) -> Vector3 {
        self.position
    }

    /// Current smoothed orbit center.
    pub fn center(&self) -> Vector3 {
        self.center
    }

    /// Current smoothed distance from the center to the camera.
    pub fn distance(&self) -> f64 {
        self.distance
    }

    /// Current orbit azimuth in radians.
    pub fn azimuth(&self) -> f64 {
        self.azimuth
    }

    /// Orbit angle above the horizontal plane in radians.
    pub fn elevation(&self) -> f64 {
        self.elevation
    }

    /// View matrix for the current position and orbit center.
    pub fn view_matrix(&self) -> Mat4 {
        Mat4::look_at(self.position, self.center, Vector3::UP)
    }

    /// Compute the bounding box of `trace` and a camera distance that frames
    /// it.
    fn framing(trace: &TraceBuffer) -> (Vector3, f64) {
        let Some(first) = trace.first() else {
            return (Vector3::ZERO, Self::INITIAL_DISTANCE);
        };

        let (mut min_x, mut max_x) = (first.x, first.x);
        let (mut min_y, mut max_y) = (first.y, first.y);
        let (mut min_z, mut max_z) = (first.z, first.z);
        for state in trace.iter() {
            min_x = min_x.min(state.x);
            max_x = max_x.max(state.x);
            min_y = min_y.min(state.y);
            max_y = max_y.max(state.y);
            min_z = min_z.min(state.z);
            max_z = max_z.max(state.z);
        }

        let center = Vector3::new(
            (min_x + max_x) / 2.0,
            (min_y + max_y) / 2.0,
            (min_z + max_z) / 2.0,
        );
        let mut radius = Self::MINIMUM_RADIUS;
        for state in trace.iter() {
            radius = radius.max(center.distance(Vector3::new(state.x, state.y, state.z)));
        }
        (
            center,
            (radius * Self::FRAMING_FACTOR).max(Self::MINIMUM_DISTANCE),
        )
    }
}
