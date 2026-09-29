import AppKit

/// Minimal headless container app. It exists solely to bundle the screensaver
/// extension under `Contents/PlugIns/AttractorScreensaver.appex` — that is how
/// System Settings discovers and lists the screen saver. The app itself has no
/// UI and runs (accessory policy, no Dock icon) only if launched manually.
final class AppDelegate: NSObject, NSApplicationDelegate {}
