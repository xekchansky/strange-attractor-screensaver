import AppKit
import Darwin
import AttractorCore
import AttractorRender

/// Current physical footprint (resident set) of this process, in bytes.
/// Diagnostic helper for the M6 memory-growth hunt: lets us watch whether the
/// demo process grows while one long show runs (vs between launches).
private func currentRSS() -> UInt64 {
    var info = task_vm_info_data_t()
    var count = mach_msg_type_number_t(
        MemoryLayout<task_vm_info_data_t>.size / MemoryLayout<integer_t>.size
    )
    let result = withUnsafeMutablePointer(to: &info) { pointer in
        pointer.withMemoryRebound(to: integer_t.self, capacity: Int(count)) { rebound in
            task_info(mach_task_self_, task_flavor_t(TASK_VM_INFO), rebound, &count)
        }
    }
    guard result == KERN_SUCCESS else { return 0 }
    return UInt64(info.phys_footprint)
}

/// Entry point of the demo app: parses arguments, builds a selection (manual
/// via `--attractor`/`--param`, or random by default), constructs the engine
/// and launches the AppKit application hosting the SceneKit view.
@MainActor
final class AppDelegate: NSObject, NSApplicationDelegate {
    private let selection: AttractorSelection
    private let engine: AttractorEngine
    private var scene: SimulationScene?
    private var window: NSWindow?

    init(selection: AttractorSelection, engine: AttractorEngine) {
        self.selection = selection
        self.engine = engine
    }

    func applicationDidFinishLaunching(_ notification: Notification) {
        let scene = SimulationScene(engine: engine, selection: selection)
        self.scene = scene

        let window = NSWindow(
            contentRect: NSRect(x: 0, y: 0, width: 900, height: 700),
            styleMask: [.titled, .closable, .miniaturizable, .resizable],
            backing: .buffered,
            defer: false
        )
        window.title = "Attractor Demo"
        window.contentView = scene.view
        scene.view.frame = window.contentView!.bounds
        window.center()
        window.makeKeyAndOrderFront(nil)
        NSApp.activate(ignoringOtherApps: true)

        self.window = window
        scene.start()

        startRSSLogger()
    }

    /// Print RSS every 5 s so a single long demo run can expose per-frame growth
    /// without needing Activity Monitor (temporary diagnostic, remove later).
    private func startRSSLogger() {
        let start = Date()
        let timer = Timer.scheduledTimer(withTimeInterval: 5, repeats: true) { _ in
            let elapsed = Int(Date().timeIntervalSince(start))
            let mb = Double(currentRSS()) / 1_048_576.0
            print(String(format: "[rss] t=%ds  %.1f MB", elapsed, mb))
        }
        RunLoop.main.add(timer, forMode: .common)
    }

    func applicationShouldTerminateAfterLastWindowClosed(_ sender: NSApplication) -> Bool {
        true
    }
}
