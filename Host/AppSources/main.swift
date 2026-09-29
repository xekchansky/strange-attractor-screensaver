import AppKit

// Container app entry point. Launch policy is accessory so no Dock icon or
// menu bar extra is shown; the process just hosts the bundled extension.
let app = NSApplication.shared
app.setActivationPolicy(.accessory)
let delegate = AppDelegate()
app.delegate = delegate
app.run()
