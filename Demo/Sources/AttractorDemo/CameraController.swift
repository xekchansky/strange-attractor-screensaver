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
    private static let framingFactor: CGFloat = 3.4

    /// Current orbit azimuth in radians. Kept unbounded; `cos`/`sin` are periodic.
    private var azimuth: Double = 0
    /// Fixed polar angle of the orbit above the horizontal plane.
    private let elevation: Double = Double.pi / 5
    /// Current smoothed orbit center (the point the camera looks at).
    private var center = SCNVector3Zero
    /// Current smoothed distance from the center to the camera.
    private var distance: CGFloat = 8
    /// How quickly the camera eases toward the target framing each frame.
    private let smoothing: CGFloat = 0.06

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
        around trace: [State],
        cameraNode: SCNNode
    ) {
        azimuth += rotationSpeed * dt

        let (targetCenter, targetDistance) = Self.framing(for: trace)
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
    private static func framing(for trace: [State]) -> (center: SCNVector3, distance: CGFloat) {
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
