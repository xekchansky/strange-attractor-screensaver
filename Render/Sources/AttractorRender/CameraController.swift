import SceneKit
import AttractorCore

/// Positions a camera on a slowly rotating orbit, continuously re-framing the
/// attractor's current bounding box so it always stays in view.
///
/// Rotation speed is supplied externally (from `SimulationSettings`); the
/// controller owns the orbit state and the smoothing toward the target framing.
final class CameraController {
    /// Minimum camera distance, so the view never zooms inside the attractor.
    private static let minimumDistance: CGFloat = 2
    /// Multiplier of the bounding radius used to compute the target distance.
    /// Kept tight: a larger margin left the attractor small in the frame, which
    /// read poorly both on screen and in small README preview thumbnails.
    private static let framingFactor: CGFloat = 2.1

    /// Current orbit azimuth in radians. Kept unbounded; `cos`/`sin` are periodic.
    /// Starts at a random angle: a fixed 0 start produced the same view angle
    /// for every launch, and captures often landed on an unflattering side.
    /// A documentation capture passes an explicit angle instead, which also
    /// makes the resulting screenshot reproducible.
    private var azimuth: Double
    /// Polar angle of the orbit above the horizontal plane.
    private let elevation: Double
    /// Current smoothed orbit center (the point the camera looks at).
    private var center = SCNVector3Zero
    /// Current smoothed distance from the center to the camera.
    private var distance: CGFloat = 8
    /// How quickly the camera eases toward the target framing each frame.
    private let smoothing: CGFloat = 0.06
    /// Recompute the full framing target (bounds scan + radius) only every
    /// `framingStride` frames. The easing already converges toward the target,
    /// so a ~10 Hz refresh is visually indistinguishable from per-frame while
    /// skipping most of the O(n) bounds scan at large trail sizes (up to
    /// 20k points).
    private static let framingStride = 6
    /// Frame counter used to pace the framing refresh.
    private var frame = 0
    /// Cached framing target, recomputed at `framingStride` cadence.
    private var targetCenter = SCNVector3Zero
    private var targetDistance: CGFloat = 8

    /// Create a controller, optionally pinning the viewing angle.
    ///
    /// - Parameters:
    ///   - azimuth: Orbit azimuth in radians. `nil` starts at a random angle,
    ///     which is what the screensaver wants; a capture passes a fixed value.
    ///   - elevation: Polar angle above the horizontal plane, in radians.
    ///     `nil` keeps the default π/5.
    init(azimuth: Double? = nil, elevation: Double? = nil) {
        self.azimuth = azimuth ?? Double.random(in: 0..<(2 * .pi))
        self.elevation = elevation ?? Double.pi / 5
    }

    /// Advance the orbit and reposition `cameraNode` to frame `trace`.
    ///
    /// - Parameters:
    ///   - dt: Real elapsed seconds since the previous frame.
    ///   - rotationSpeed: Camera orbit angular velocity in radians/second.
    ///   - trace: The trace points to frame.
    ///   - cameraNode: The node to reposition.
    func update(
        dt: Double,
        rotationSpeed: Double,
        around trace: TraceBuffer,
        cameraNode: SCNNode
    ) {
        frame += 1
        azimuth += rotationSpeed * dt

        if frame % Self.framingStride == 1 {
            (targetCenter, targetDistance) = Self.framing(for: trace)
        }
        center = Self.lerp(center, targetCenter, smoothing)
        distance += (targetDistance - distance) * smoothing

        let cosElevation = CGFloat(cos(elevation))
        cameraNode.position = SCNVector3(
            center.x + distance * cosElevation * CGFloat(cos(azimuth)),
            center.y + distance * CGFloat(sin(elevation)),
            center.z + distance * cosElevation * CGFloat(sin(azimuth))
        )
        cameraNode.look(at: center)
    }

    /// Compute the bounding box of `trace` and a camera distance that frames it.
    private static func framing(for trace: TraceBuffer) -> (center: SCNVector3, distance: CGFloat) {
        guard let first = trace.first else { return (SCNVector3Zero, 8) }

        var minX = CGFloat(first.x), maxX = CGFloat(first.x)
        var minY = CGFloat(first.y), maxY = CGFloat(first.y)
        var minZ = CGFloat(first.z), maxZ = CGFloat(first.z)
        for state in trace {
            let x = CGFloat(state.x), y = CGFloat(state.y), z = CGFloat(state.z)
            minX = Swift.min(minX, x); maxX = Swift.max(maxX, x)
            minY = Swift.min(minY, y); maxY = Swift.max(maxY, y)
            minZ = Swift.min(minZ, z); maxZ = Swift.max(maxZ, z)
        }

        let center = SCNVector3(
            (minX + maxX) / 2,
            (minY + maxY) / 2,
            (minZ + maxZ) / 2
        )
        var radius: CGFloat = 0.1
        for state in trace {
            let x = CGFloat(state.x) - center.x
            let y = CGFloat(state.y) - center.y
            let z = CGFloat(state.z) - center.z
            radius = Swift.max(radius, (x * x + y * y + z * z).squareRoot())
        }
        return (center, Swift.max(radius * framingFactor, minimumDistance))
    }

    private static func lerp(_ a: SCNVector3, _ b: SCNVector3, _ t: CGFloat) -> SCNVector3 {
        SCNVector3(
            a.x + (b.x - a.x) * t,
            a.y + (b.y - a.y) * t,
            a.z + (b.z - a.z) * t
        )
    }
}
