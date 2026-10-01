import AppKit
import SceneKit
import QuartzCore
import os
import AttractorCore

/// Lifecycle logger (temporary diagnostic, remove later): correlates scene
/// creation/start with deallocation while chasing the host memory growth.
private let simulationLog = Logger(subsystem: "com.example.AttractorSaver", category: "lifecycle")

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
    /// Monotonic scene id for correlating launches with deallocation in Console
    /// while chasing the host memory growth (temporary diagnostic, remove later).
    private static var nextSceneID = 0
    private let sceneID: Int
    /// Live scenes across the whole process (temporary diagnostic). The host
    /// keeps referencing old screensaver views, and scenes never deallocating
    /// is exactly the memory growth being chased. `nonisolated(unsafe)` so the
    /// counter is reachable from `deinit`; everything runs on the main thread.
    nonisolated(unsafe) private static var liveCount = 0
    private var lastTime: CFTimeInterval = 0
    private var accumulatedSimTime: Double = 0
    private var colorOffset: Double = 0
    private var lastOverlayUpdate: CFTimeInterval = 0
    /// Frame counter used to pace polyline rebuilds (every 2nd tick). Rebuilding
    /// the SCNGeometry on every tick made the saver preview redraw in ragged
    /// pieces at 60 fps; halving the rebuild rate is visually indistinguishable
    /// on a dense trail while removing most of that redraw pressure.
    private var renderFrame = 0
    /// Whether the animation loop is running. Used by the host to know whether
    /// `startAnimation` should reuse the scene or spin up a fresh one (after the
    /// scene was stopped because its window went away).
    public private(set) var isRunning = false

    /// Create the scene around a ready-to-run engine.
    public init(engine: AttractorEngine, selection: AttractorSelection) {
        self.engine = engine
        self.selection = selection
        self.settings = selection.settings
        self.sceneID = Self.nextSceneID
        Self.nextSceneID += 1
        Self.liveCount += 1

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

        simulationLog.info("SimulationScene #\(self.sceneID) created (live=\(Self.liveCount))")
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

    /// Begin the per-frame animation loop. A run-loop `Timer` drives the tick
    /// — the SceneKit view redraws whenever the node geometry is replaced, so
    /// no `isPlaying`/display-link is required (in the screensaver host the
    /// view's display link never starts, which produced a black screen).
    ///
    /// The timer uses a `[weak self]` block, never a strong `target:`: a
    /// strong target would hold the scene via `RunLoop → Timer → scene`, which
    /// is exactly what kept old scenes (and their large views) resident in the
    /// host process across preview launches. When the host releases the view,
    /// the scene deallocates and `deinit` invalidates the timer.
    public func start() {
        guard timer == nil else { return }
        simulationLog.info("SimulationScene #\(self.sceneID) start")
        lastTime = CACurrentMediaTime()
        let timer = Timer(
            timeInterval: 1.0 / 60.0,
            repeats: true
        ) { [weak self] _ in
            MainActor.assumeIsolated {
                self?.tick()
            }
        }
        RunLoop.main.add(timer, forMode: .common)
        self.timer = timer
        isRunning = true
    }

    /// Stop the animation loop.
    public func stop() {
        timer?.invalidate()
        timer = nil
        isRunning = false
    }

    /// Called when the scene goes away for good: stop the loop and release the
    /// view's scene so SceneKit frees the geometry even if the host forgets to
    /// call `stop()`. The scene lives on the main thread, so the isolation is
    /// safe to assume.
    deinit {
        Self.liveCount -= 1
        simulationLog.info("SimulationScene #\(self.sceneID) dealloc (live=\(Self.liveCount))")
        MainActor.assumeIsolated {
            self.timer?.invalidate()
            self.view.scene = nil
        }
    }

    /// Advance one rendered frame. Deposed by the run-loop timer (`start()`);
    /// the SceneKit view redraws once `traceNode.geometry` is replaced.
    private func tick() {
        autoreleasepool {
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

            cameraController.update(
                dt: dt,
                rotationSpeed: settings.cameraRotationSpeed,
                around: engine.trace,
                cameraNode: cameraNode
            )

            // Rebuild the polyline every other tick (plus the first tick so the
            // scene appears immediately). Rendering every tick produced ragged
            // chunks in the saver preview at 60 fps; a ~30 Hz rebuild of a dense
            // trail is visually smooth and roughly halves both the SCNGeometry
            // redraw pressure and the per-frame allocation behind it. Camera
            // orbit stays per-tick.
            renderFrame += 1
            if renderFrame <= 2 || renderFrame % 2 == 0 {
                traceNode.geometry = traceRenderer.geometry(
                    for: engine.trace,
                    colorOffset: colorOffset,
                    mode: settings.colorMode,
                    cameraPosition: cameraNode.position
                )
            }

            if now - lastOverlayUpdate > 0.5 {
                overlay.stringValue = Self.overlayText(for: selection, state: engine.state)
                lastOverlayUpdate = now
            }
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
