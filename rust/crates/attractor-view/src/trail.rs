//! Trail geometry: a camera-facing ribbon colored by one of four modes.

use attractor_core::{TraceBuffer, TraceColorMode};

use crate::vector::Vector3;

/// A linear RGB color with the precision the GPU expects.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rgb {
    /// Red component in `[0, 1]`.
    pub r: f32,
    /// Green component in `[0, 1]`.
    pub g: f32,
    /// Blue component in `[0, 1]`.
    pub b: f32,
}

impl Rgb {
    /// Build a color from its components.
    pub const fn new(r: f32, g: f32, b: f32) -> Self {
        Self { r, g, b }
    }
}

/// A ribbon built from a trace: two vertices per sampled point, drawn as one
/// indexed triangle strip.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct TrailGeometry {
    /// Ribbon vertices, two per sampled point, in strip order.
    positions: Vec<Vector3>,
    /// One color per ribbon vertex.
    colors: Vec<Rgb>,
    /// Strip indices: `0, 1, 2, 3, …`, two per sampled point.
    indices: Vec<u32>,
    /// Half thickness of the ribbon in world units.
    ribbon_radius: f64,
}

impl TrailGeometry {
    /// Whether the trace was too short to build anything.
    pub fn is_empty(&self) -> bool {
        self.positions.is_empty()
    }

    /// Number of sampled trace points that produced a ribbon pair.
    pub fn point_count(&self) -> usize {
        self.positions.len() / 2
    }

    /// Number of ribbon vertices, two per sampled point.
    pub fn vertex_count(&self) -> usize {
        self.positions.len()
    }

    /// Number of triangles in the strip (`vertices - 2`).
    pub fn triangle_count(&self) -> usize {
        self.positions.len().saturating_sub(2)
    }

    /// Ribbon vertices, two per sampled point.
    pub fn positions(&self) -> &[Vector3] {
        &self.positions
    }

    /// One color per ribbon vertex.
    pub fn colors(&self) -> &[Rgb] {
        &self.colors
    }

    /// Triangle-strip indices.
    pub fn indices(&self) -> &[u32] {
        &self.indices
    }

    /// Half thickness of the ribbon in world units.
    pub fn ribbon_radius(&self) -> f64 {
        self.ribbon_radius
    }
}

/// Converts the engine's trace into a ribbon with a color gradient that shows
/// the direction of motion along the trail.
///
/// The builder retains its buffers across frames and refills them in place, so
/// a 60 Hz renderer does not allocate tens of kilobytes per frame for long
/// trails.
#[derive(Debug, Clone)]
pub struct TrailBuilder {
    budget: usize,
    sampled_indices: Vec<usize>,
    sampled_positions: Vec<Vector3>,
    point_colors: Vec<Rgb>,
    geometry: TrailGeometry,
}

impl TrailBuilder {
    /// Default budget for very long trails. Trails at or below this many
    /// points are drawn untouched; longer ones are uniformly subsampled.
    pub const DEFAULT_BUDGET: usize = 16_384;
    /// Brightness of the oldest point in the `age` mode.
    const AGE_FLOOR: f64 = 0.15;
    /// Brightness of the newest point in the `age` mode.
    const AGE_CEILING: f64 = 1.0;
    /// Saturation shared by the hue-based modes.
    const SATURATION: f64 = 0.9;
    /// Ribbon thickness as a fraction of the trail's largest coordinate.
    const RADIUS_FRACTION: f64 = 0.004;
    /// Smallest ribbon thickness, so a trail at the origin still has area.
    const MINIMUM_RADIUS: f64 = 1e-6;

    /// Create a builder with [`Self::DEFAULT_BUDGET`].
    pub fn new() -> Self {
        Self::with_budget(Self::DEFAULT_BUDGET)
    }

    /// Create a builder with an explicit point budget.
    ///
    /// A budget of zero is raised to one so the stride stays defined.
    pub fn with_budget(budget: usize) -> Self {
        Self {
            budget: budget.max(1),
            sampled_indices: Vec::new(),
            sampled_positions: Vec::new(),
            point_colors: Vec::new(),
            geometry: TrailGeometry::default(),
        }
    }

    /// Point budget of this builder.
    pub fn budget(&self) -> usize {
        self.budget
    }

    /// The geometry produced by the most recent [`Self::build`].
    ///
    /// Empty before the first build. Exposed so a renderer can hold the
    /// builder across frames and read the buffers without rebuilding.
    pub fn geometry(&self) -> &TrailGeometry {
        &self.geometry
    }

    /// Build a ribbon from `trace`, colored by `mode`.
    ///
    /// `color_offset` is a value in `[0, 1)` shifting the color ramp; advance
    /// it by the trail color speed every frame to animate the gradient.
    /// `camera_position` is the current camera position, used by
    /// [`TraceColorMode::Depth`] and to orient the ribbon toward the viewer.
    ///
    /// A trace with fewer than two points produces an empty geometry.
    pub fn build(
        &mut self,
        trace: &TraceBuffer,
        color_offset: f64,
        mode: TraceColorMode,
        camera_position: Vector3,
    ) -> &TrailGeometry {
        self.geometry.positions.clear();
        self.geometry.colors.clear();
        self.geometry.indices.clear();
        self.geometry.ribbon_radius = 0.0;

        let count = trace.len();
        if count <= 1 {
            return &self.geometry;
        }

        // Uniformly subsample very long trails to keep the polyline within the
        // vertex budget. The trail is a dense continuous curve, so a constant
        // stride preserves its shape; the color ramp is derived from the
        // original index proportion, keeping the gradient unchanged.
        let stride = if count <= self.budget {
            1
        } else {
            count.div_ceil(self.budget)
        };
        self.sampled_indices.clear();
        let mut index = 0;
        while index < count {
            self.sampled_indices.push(index);
            index += stride;
        }
        // Always keep the newest point (the trail head) as the final vertex.
        if self
            .sampled_indices
            .last()
            .is_some_and(|&last| last != count - 1)
        {
            self.sampled_indices.push(count - 1);
        }

        // Build positions and, in the same pass, gather the per-mode
        // statistics needed to normalize the ramp (distance or axis bounds).
        let mut min_x = f64::INFINITY;
        let mut max_x = f64::NEG_INFINITY;
        let mut min_y = f64::INFINITY;
        let mut max_y = f64::NEG_INFINITY;
        let mut min_z = f64::INFINITY;
        let mut max_z = f64::NEG_INFINITY;
        let mut min_distance = f64::INFINITY;
        let mut max_distance: f64 = 0.0;

        self.sampled_positions.clear();
        for &index in &self.sampled_indices {
            let state = trace[index];
            let position = Vector3::new(state.x, state.y, state.z);
            self.sampled_positions.push(position);
            match mode {
                TraceColorMode::Trail | TraceColorMode::Age => {}
                TraceColorMode::Depth => {
                    let distance = position.distance(camera_position);
                    min_distance = min_distance.min(distance);
                    max_distance = max_distance.max(distance);
                }
                TraceColorMode::Spatial => {
                    min_x = min_x.min(position.x);
                    max_x = max_x.max(position.x);
                    min_y = min_y.min(position.y);
                    max_y = max_y.max(position.y);
                    min_z = min_z.min(position.z);
                    max_z = max_z.max(position.z);
                }
            }
        }

        let total = count as f64;
        self.point_colors.clear();
        self.point_colors.reserve(self.sampled_positions.len());
        for (offset, &index) in self.sampled_indices.iter().enumerate() {
            let position = self.sampled_positions[offset];
            let color = match mode {
                TraceColorMode::Trail => {
                    let hue = (index as f64 / total + color_offset) % 1.0;
                    Self::hsb_to_rgb(hue, Self::SATURATION, 1.0)
                }
                TraceColorMode::Age => {
                    // Older points (small index) are darker, newer ones
                    // brighter; the offset advances a brightness wave.
                    let ramp = (index as f64 / total + color_offset) % 1.0;
                    let value = Self::AGE_FLOOR + (Self::AGE_CEILING - Self::AGE_FLOOR) * ramp;
                    Rgb::new(value as f32, value as f32, value as f32)
                }
                TraceColorMode::Depth => {
                    let distance = position.distance(camera_position);
                    let t = Self::normalized(distance, min_distance, max_distance);
                    let hue = (t + color_offset) % 1.0;
                    Self::hsb_to_rgb(hue, Self::SATURATION, 1.0)
                }
                TraceColorMode::Spatial => {
                    // Static per-axis coordinate color (x→R, y→G, z→B),
                    // normalized per trail. Deliberately no color offset: a
                    // coordinate map is meant to be stable.
                    Rgb::new(
                        Self::normalized(position.x, min_x, max_x) as f32,
                        Self::normalized(position.y, min_y, max_y) as f32,
                        Self::normalized(position.z, min_z, max_z) as f32,
                    )
                }
            };
            self.point_colors.push(color);
        }

        // Camera-facing ribbon instead of a one-pixel polyline: each sampled
        // point becomes two vertices offset ±radius along a screen-facing
        // perpendicular, and the ribbon is one indexed triangle strip.
        let radius = Self::ribbon_radius(&self.sampled_positions);
        self.geometry.ribbon_radius = radius;
        let point_count = self.sampled_positions.len();
        self.geometry.positions.reserve(point_count * 2);
        self.geometry.colors.reserve(point_count * 2);

        for offset in 0..point_count {
            let position = self.sampled_positions[offset];
            // Tangent from the neighboring sampled point (clamped at the
            // ends); the screen-facing perpendicular is tangent × toCamera.
            let next = self.sampled_positions[(offset + 1).min(point_count - 1)];
            let previous = self.sampled_positions[offset.saturating_sub(1)];
            let tangent = next.subtract(previous).normalized();
            let to_camera = camera_position.subtract(position).normalized();
            let side = tangent.cross(to_camera).normalized();
            let offset_vector = side.scale(radius);
            // Round through `f32` like the geometry source SceneKit builds.
            self.geometry
                .positions
                .push((position + offset_vector).rounded_to_f32());
            self.geometry
                .positions
                .push(position.subtract(offset_vector).rounded_to_f32());
            let color = self.point_colors[offset];
            self.geometry.colors.push(color);
            self.geometry.colors.push(color);
        }

        // Triangle strip indices: two vertices per point, the strip zigzags
        // 0, 1, 2, 3, … connecting point i's pair with point i+1's pair.
        self.geometry.indices.reserve(point_count * 2);
        for offset in 0..point_count {
            let base = (offset * 2) as u32;
            self.geometry.indices.push(base);
            self.geometry.indices.push(base + 1);
        }

        &self.geometry
    }

    /// Half thickness of the ribbon in world units, derived from the trail
    /// bounding extent so the line reads equally well after auto-framing zooms.
    fn ribbon_radius(positions: &[Vector3]) -> f64 {
        let mut max_extent: f64 = 0.0;
        for position in positions {
            max_extent = max_extent
                .max(position.x.abs())
                .max(position.y.abs())
                .max(position.z.abs());
        }
        (max_extent * Self::RADIUS_FRACTION).max(Self::MINIMUM_RADIUS)
    }

    /// Normalize `value` in `[min, max]` to `[0, 1]`, guarding against a zero
    /// extent (all points at the same position).
    fn normalized(value: f64, min: f64, max: f64) -> f64 {
        let extent = max - min;
        if extent > 0.0 {
            (value - min) / extent
        } else {
            0.0
        }
    }

    /// Convert an HSB color (hue in `[0, 1)` modulo the circle) to RGB.
    ///
    /// The hue is reduced with a truncated integer segment, matching the Swift
    /// renderer including its behavior for negative hues.
    fn hsb_to_rgb(hue: f64, saturation: f64, brightness: f64) -> Rgb {
        let scaled = hue * 6.0;
        let segment = (scaled as i64) % 6;
        let fraction = scaled - segment as f64;
        let p = brightness * (1.0 - saturation);
        let q = brightness * (1.0 - fraction * saturation);
        let t = brightness * (1.0 - (1.0 - fraction) * saturation);
        let (r, g, b) = match segment {
            0 => (brightness, t, p),
            1 => (q, brightness, p),
            2 => (p, brightness, t),
            3 => (p, q, brightness),
            4 => (t, p, brightness),
            _ => (brightness, p, q),
        };
        Rgb::new(r as f32, g as f32, b as f32)
    }
}

impl Default for TrailBuilder {
    fn default() -> Self {
        Self::new()
    }
}
