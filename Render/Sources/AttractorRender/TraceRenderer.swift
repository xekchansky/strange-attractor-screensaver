import SceneKit
import AttractorCore

/// Converts the engine's trace into a SceneKit polyline with a color gradient
/// that shows the direction of motion along the trail.
///
/// This is a stateful helper: it reuses the Swift buffers backing the geometry
/// (positions/colors/indices) across frames, avoiding the tens of kilobytes of
/// per-frame heap churn a stateless builder would allocate for long trails.
/// The `SCNGeometry` is still recreated each frame, but its backing arrays are
/// retained and refilled in place. Shared by the demo and the screensaver host
/// (M5) without UI coupling.
final class TraceRenderer {
    /// Vertex budget for very long trails (matched by the `trailLength` ranges).
    /// Trails at or below this are drawn untouched.
    private let budget = 16_384

    // Reusable backing arrays, grown as needed and retained across frames.
    private var sampledIndices: [Int] = []
    private var positions: [SCNVector3] = []
    private var colors: [Float] = []
    private var indices: [UInt32] = []

    /// Build a polyline geometry from `trace`, colored by `mode`.
    ///
    /// - Parameters:
    ///   - trace: The ordered trace points, oldest first.
    ///   - colorOffset: A value in `[0, 1)` shifting the color ramp. Advance it
    ///     by the `trailColorSpeed` every frame to animate the gradient.
    ///   - mode: How to color the trail.
    ///   - cameraPosition: The current camera position, used by ``TraceColorMode/depth``.
    func geometry(
        for trace: [State],
        colorOffset: Double,
        mode: TraceColorMode,
        cameraPosition: SCNVector3
    ) -> SCNGeometry {
        let n = trace.count
        guard n > 1 else { return SCNGeometry() }

        // Uniformly subsample very long trails to keep the polyline within the
        // vertex budget. The trail is a dense continuous curve, so a constant
        // stride preserves its shape; the color ramp is derived from the
        // original index proportion, keeping the gradient unchanged.
        let stride: Int = n <= budget ? 1 : (n + budget - 1) / budget

        sampledIndices.removeAll(keepingCapacity: true)
        var index = 0
        while index < n {
            sampledIndices.append(index)
            index += stride
        }
        // Always keep the newest point (the trail head) as the final vertex.
        if let last = sampledIndices.last, last != n - 1 {
            sampledIndices.append(n - 1)
        }
        let sampledCount = sampledIndices.count

        // Build positions and, in the same pass, gather the per-mode statistics
        // needed to normalize the ramp (depth distances or per-axis bounds).
        var minX = CGFloat.greatestFiniteMagnitude
        var maxX = -CGFloat.greatestFiniteMagnitude
        var minY = CGFloat.greatestFiniteMagnitude
        var maxY = -CGFloat.greatestFiniteMagnitude
        var minZ = CGFloat.greatestFiniteMagnitude
        var maxZ = -CGFloat.greatestFiniteMagnitude
        var minDistance: CGFloat = .greatestFiniteMagnitude
        var maxDistance: CGFloat = 0

        positions.removeAll(keepingCapacity: true)
        positions.reserveCapacity(sampledCount)
        for index in sampledIndices {
            let state = trace[index]
            let position = SCNVector3(CGFloat(state.x), CGFloat(state.y), CGFloat(state.z))
            positions.append(position)
            switch mode {
            case .trail, .age:
                break
            case .depth:
                let distance = Self.distance(position, cameraPosition)
                minDistance = Swift.min(minDistance, distance)
                maxDistance = Swift.max(maxDistance, distance)
            case .spatial:
                minX = Swift.min(minX, position.x); maxX = Swift.max(maxX, position.x)
                minY = Swift.min(minY, position.y); maxY = Swift.max(maxY, position.y)
                minZ = Swift.min(minZ, position.z); maxZ = Swift.max(maxZ, position.z)
            }
        }

        let count = Double(n)
        colors.removeAll(keepingCapacity: true)
        colors.reserveCapacity(sampledCount * 3)
        for (offset, index) in sampledIndices.enumerated() {
            let position = positions[offset]
            let (r, g, b): (Double, Double, Double)
            switch mode {
            case .trail:
                let hue = (Double(index) / count + colorOffset).truncatingRemainder(dividingBy: 1.0)
                (r, g, b) = Self.hsbToRgb(hue: hue, saturation: 0.9, brightness: 1.0)
            case .age:
                // Grayscale fade: older points (small index) are darker, newer
                // ones brighter; the offset advances a brightness wave.
                let ramp = ((Double(index) / count) + colorOffset).truncatingRemainder(dividingBy: 1.0)
                let value = 0.15 + 0.85 * ramp
                (r, g, b) = (value, value, value)
            case .depth:
                let distance = Self.distance(position, cameraPosition)
                let t = Self.normalized(distance, min: minDistance, max: maxDistance)
                let hue = (t + colorOffset).truncatingRemainder(dividingBy: 1.0)
                (r, g, b) = Self.hsbToRgb(hue: hue, saturation: 0.9, brightness: 1.0)
            case .spatial:
                // Static per-axis coordinate color (x→R, y→G, z→B), normalized
                // per trail. Deliberately no colorOffset: cycling all three
                // channels through the moda offset made each point strobe as
                // its channels wrapped 1→0 at different times, which read as
                // harsh flashing. A coordinate map is meant to be stable.
                let red = Self.normalized(position.x, min: minX, max: maxX)
                let green = Self.normalized(position.y, min: minY, max: maxY)
                let blue = Self.normalized(position.z, min: minZ, max: maxZ)
                (r, g, b) = (red, green, blue)
            }
            colors.append(Float(r))
            colors.append(Float(g))
            colors.append(Float(b))
        }

        // Explicit paired line indices: SceneKit draws one segment per pair
        // (0,1), (1,2), ..., giving a single connected strip. An empty
        // index buffer (bytesPerIndex: 0) is not reliably handled by the
        // Metal renderer and can crash in drawIndexedPrimitives.
        let segmentCount = sampledCount - 1
        indices.removeAll(keepingCapacity: true)
        indices.reserveCapacity(segmentCount * 2)
        for i in 0..<segmentCount {
            indices.append(UInt32(i))
            indices.append(UInt32(i + 1))
        }
        let indexData = indices.withUnsafeBytes { Data($0) }

        let positionSource = SCNGeometrySource(vertices: positions)
        let colorSource = SCNGeometrySource(
            data: Data(bytes: colors, count: colors.count * MemoryLayout<Float>.size),
            semantic: .color,
            vectorCount: sampledCount,
            usesFloatComponents: true,
            componentsPerVector: 3,
            bytesPerComponent: MemoryLayout<Float>.size,
            dataOffset: 0,
            dataStride: MemoryLayout<Float>.size * 3
        )
        let element = SCNGeometryElement(
            data: indexData,
            primitiveType: .line,
            primitiveCount: segmentCount,
            bytesPerIndex: MemoryLayout<UInt32>.size
        )
        let geometry = SCNGeometry(sources: [positionSource, colorSource], elements: [element])
        // Vertex colors feed the diffuse channel; disable lighting so the
        // gradient is shown as-is on the unlit polyline.
        let material = SCNMaterial()
        material.lightingModel = .constant
        geometry.firstMaterial = material
        return geometry
    }

    /// Euclidean distance between two positions.
    private static func distance(_ a: SCNVector3, _ b: SCNVector3) -> CGFloat {
        let dx = a.x - b.x, dy = a.y - b.y, dz = a.z - b.z
        return (dx * dx + dy * dy + dz * dz).squareRoot()
    }

    /// Normalize `value` in `[min, max]` to `[0, 1]`, guarding against a zero
    /// extent (all points at the same position) edge case.
    private static func normalized(_ value: CGFloat, min: CGFloat, max: CGFloat) -> Double {
        let extent = max - min
        guard extent > 0 else { return 0 }
        return Double((value - min) / extent)
    }

    /// Convert an HSB color component (hue in `[0, 1)`) to linear RGB in `[0, 1]`.
    private static func hsbToRgb(
        hue: Double,
        saturation: Double,
        brightness: Double
    ) -> (Double, Double, Double) {
        let segment = Int(hue * 6) % 6
        let fraction = hue * 6 - Double(segment)
        let p = brightness * (1 - saturation)
        let q = brightness * (1 - fraction * saturation)
        let t = brightness * (1 - (1 - fraction) * saturation)
        switch segment {
        case 0: return (brightness, t, p)
        case 1: return (q, brightness, p)
        case 2: return (p, brightness, t)
        case 3: return (p, q, brightness)
        case 4: return (t, p, brightness)
        default: return (brightness, p, q)
        }
    }
}
