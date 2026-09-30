import AppKit
import SceneKit
import QuartzCore
import AttractorCore

/// Owns the SceneKit view and the running simulation: it steps the engine each
/// frame, repaints the trace as a gradient polyline, orbits the camera, and
/// shows the current formulas, parameters and the live position in a corner
/// overlay.
@MainActor
public final class SimulationScene: NSObject {
    /// The SceneKit view backing this simulation. Embed it in a window.
    public let view: SCNView
    /// The attractor selection being visualized (used for the overlay text).
    public let selection: AttractorSelection

    private var engine: AttractorEngine
    private let settings: SimulationSettings
    private let cameraNode: SCNNode
    private let traceNode: SCNNode
    private let cameraController = CameraController()
    private let traceRenderer = TraceRenderer()
    private let overlay = NSTextField(labelWithString: "")

    private var timer: Timer?
    private var lastTime: CFTimeInterval = 0
    private var accumulatedSimTime: Double = 0
    private var colorOffset: Double = 0
    private var lastOverlayUpdate: CFTimeInterval = 0

    /// Create the scene around a ready-to-run engine.
    public init(engine: AttractorEngine, selection: AttractorSelection) {
        self.engine = engine
        self.selection = selection
        self.settings = selection.settings

        let scene = SCNScene()
        scene.background.contents = NSColor.black

        let camera = SCNCamera()
        camera.automaticallyAdjustsZRange = true
        let cameraNode = SCNNode()
        cameraNode.camera = camera
        scene.rootNode.addChildNode(cameraNode)
        self.cameraNode = cameraNode

        scene.rootNode.addChildNode(Self.ambientLight())
        let traceNode = SCNNode()
        scene.rootNode.addChildNode(traceNode)
        self.traceNode = traceNode

        let scnView = SCNView(frame: .zero)
        scnView.scene = scene
        scnView.backgroundColor = .black
        scnView.autoresizingMask = [.width, .height]
        self.view = scnView

        super.init()

        setupOverlay()
    }

    /// Create the ambient light illuminating the scene.
    private static func ambientLight() -> SCNNode {
        let node = SCNNode()
        let light = SCNLight()
        light.type = .ambient
        light.color = NSColor.white
        node.light = light
        return node
    }

    /// Configure the corner overlay showing the attractor's formulas and params.
    private func setupOverlay() {
        overlay.textColor = .white
        overlay.font = .monospacedSystemFont(ofSize: 13, weight: .regular)
        overlay.maximumNumberOfLines = 0
        overlay.translatesAutoresizingMaskIntoConstraints = false
        view.addSubview(overlay)
        NSLayoutConstraint.activate([
            overlay.leadingAnchor.constraint(equalTo: view.leadingAnchor, constant: 16),
            overlay.bottomAnchor.constraint(equalTo: view.bottomAnchor, constant: -16)
        ])
        overlay.stringValue = Self.overlayText(for: selection, state: engine.state)
    }

    /// Begin the per-frame animation loop. Call from the main thread.
    public func start() {
        guard timer == nil else { return }
        lastTime = CACurrentMediaTime()
        let timer = Timer(
            timeInterval: 1.0 / 60.0,
            target: self,
            selector: #selector(tick(_:)),
            userInfo: nil,
            repeats: true
        )
        RunLoop.main.add(timer, forMode: .common)
        self.timer = timer
    }

    /// Stop the animation loop.
    public func stop() {
        timer?.invalidate()
        timer = nil
    }

    /// Advance one rendered frame. Invoked by the timer on the main run loop.
    @objc private func tick(_ timer: Timer) {
        let now = CACurrentMediaTime()
        let dt = lastTime == 0 ? 1.0 / 60.0 : now - lastTime
        lastTime = now

        // Integrate the system by a whole number of fixed-size steps.
        accumulatedSimTime += settings.simulationSpeed * dt
        let steps = Int(accumulatedSimTime / settings.stepSize)
        if steps > 0 {
            accumulatedSimTime -= Double(steps) * settings.stepSize
            for _ in 0..<steps {
                // A valid model never throws at evaluation; ignore the
                // pathological case rather than stopping the demo.
                _ = try? engine.advance()
            }
        }

        colorOffset = (colorOffset + settings.trailColorSpeed * dt)
            .truncatingRemainder(dividingBy: 1.0)
        traceNode.geometry = traceRenderer.geometry(
            for: engine.trace,
            colorOffset: colorOffset
        )

        cameraController.update(
            dt: dt,
            rotationSpeed: settings.cameraRotationSpeed,
            around: engine.trace,
            cameraNode: cameraNode
        )

        if now - lastOverlayUpdate > 0.5 {
            overlay.stringValue = Self.overlayText(for: selection, state: engine.state)
            lastOverlayUpdate = now
        }
    }

    /// Build the multi-line overlay text: attractor name, formulas, parameter
    /// values and the current state (the position of the newest point).
    private static func overlayText(for selection: AttractorSelection, state: State) -> String {
        let descriptor = selection.descriptor
        let parameters = selection
            .parameters
            .sorted { $0.key < $1.key }
            .map { "\($0.key) = \(Self.format($0.value))" }
            .joined(separator: "   ")

        let position = "(x, y, z) = (\(Self.format(state.x)), \(Self.format(state.y)), \(Self.format(state.z)))"

        let formulas = [
            "dX/dt = \(descriptor.dx)",
            "dY/dt = \(descriptor.dy)",
            "dZ/dt = \(descriptor.dz)"
        ]
        return ([descriptor.name] + formulas + [parameters, position]).joined(separator: "\n")
    }

    private static func format(_ value: Double) -> String {
        String(format: "%.3f", value)
    }
}
