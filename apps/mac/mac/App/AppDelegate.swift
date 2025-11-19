import Cocoa

/// AppDelegate for GUI/daemon modes
/// Note: No @main here - main.swift handles routing and launches NSApplication
class AppDelegate: NSObject, NSApplicationDelegate {

    private var menuBarController: MenuBarController?
    private var daemonController: DaemonController?
    private var permissionsManager: PermissionsManager?
    private var isDaemonMode = false

    func applicationDidFinishLaunching(_ notification: Notification) {
        // Set up shared file logging
        Logger.setup()

        // Log app information at startup
        Logger.log("🚀 Ariata Mac starting up")
        Logger.log("   Bundle path: \(Bundle.main.bundlePath)")
        Logger.log("   Executable: \(Bundle.main.executablePath ?? "unknown")")
        Logger.log("   Bundle ID: \(Bundle.main.bundleIdentifier ?? "unknown")")

        // Check if launched in daemon mode (--daemon flag)
        isDaemonMode = CommandLine.arguments.contains("--daemon")

        // Initialize managers
        permissionsManager = PermissionsManager()
        daemonController = DaemonController()

        if isDaemonMode {
            // Headless daemon mode - no menu bar, no dock icon
            NSApp.setActivationPolicy(.prohibited)
            Logger.log("Starting in daemon mode (headless)...")

            // Start daemon directly
            daemonController?.start()
        } else {
            // Interactive mode - show menu bar
            NSApp.setActivationPolicy(.accessory) // No dock icon, but menu bar visible
            Logger.log("Starting in interactive mode (menu bar)...")

            // Check if app is paired (config exists)
            Logger.log("🔍 Checking if app is paired...")
            let isPaired = Config.load() != nil
            Logger.log("📱 Paired status: \(isPaired)")

            // Initialize menu bar
            Logger.log("🎨 Creating menu bar controller...")
            menuBarController = MenuBarController(
                daemonController: daemonController!,
                permissionsManager: permissionsManager!,
                needsPairing: !isPaired
            )
            Logger.log("🎨 Calling menuBarController.setup()...")
            menuBarController?.setup()
            Logger.log("✅ Menu bar setup complete", level: .success)

            // Only start daemon if paired AND has permissions
            if isPaired {
                // Delay permission check slightly to let menu bar appear first
                DispatchQueue.main.asyncAfter(deadline: .now() + 0.5) { [weak self] in
                    guard let self = self else { return }

                    Logger.log("🔐 Checking permissions...")
                    if let perms = self.permissionsManager, perms.allPermissionsGranted() {
                        Logger.log("✅ All permissions granted, starting daemon...", level: .success)
                        self.daemonController?.start()

                        // Trigger immediate menu update
                        self.menuBarController?.updateStatus()
                    } else {
                        Logger.log("⚠️ Missing permissions, showing alert...", level: .warning)
                        self.showPermissionsAlert()
                    }
                }
            }
            // If not paired, user will see "Pair This Mac" menu item
        }
    }

    func applicationWillTerminate(_ notification: Notification) {
        // Stop daemon unless we're in daemon mode and just closing menu bar
        if !isDaemonMode {
            Logger.log("Stopping daemon...")
            daemonController?.stop()
        }
        Logger.close()
    }

    func applicationShouldHandleReopen(_ sender: NSApplication, hasVisibleWindows flag: Bool) -> Bool {
        // This is called when user tries to relaunch the app while it's already running
        // (e.g., clicking app icon again or using `open` command)
        Logger.log("🔄 App reopen requested (hasVisibleWindows: \(flag))")

        // If we're in daemon mode, ignore reopen requests
        if isDaemonMode {
            Logger.log("   Ignoring reopen in daemon mode")
            return false
        }

        // Re-show the menu bar if it was hidden
        if let menuBar = menuBarController {
            menuBar.showMenuBar()
        }

        return true
    }

    private func showPermissionsAlert() {
        let alert = NSAlert()
        alert.messageText = "Ariata Needs Permissions"

        // Check which specific permissions are missing
        let needsAccessibility = !(permissionsManager?.checkAccessibility() ?? false)
        let needsFullDisk = !(permissionsManager?.checkFullDiskAccess() ?? false)

        var instructions = "To track your activity and messages, Ariata needs:\n\n"

        if needsAccessibility {
            instructions += "✓ Accessibility Access\n   (for monitoring app usage)\n\n"
        }

        if needsFullDisk {
            instructions += "✓ Full Disk Access\n   (for tracking iMessages)\n\n"
        }

        instructions += """
        Steps:
        1. Click 'Open System Settings' below
        2. Click the lock icon and enter your password
        3. Click the '+' button
        4. Select 'Ariata' from Applications folder
        5. Toggle the switch ON
        6. Return to this window

        Ariata will automatically detect when permissions are granted.
        """

        alert.informativeText = instructions
        alert.addButton(withTitle: "Open System Settings")
        alert.addButton(withTitle: "Quit")
        alert.alertStyle = .informational

        if alert.runModal() == .alertFirstButtonReturn {
            permissionsManager?.openSystemPreferences()

            // Start polling to detect when permissions are granted
            startPermissionsPolling()
        } else {
            NSApp.terminate(nil)
        }
    }

    private func startPermissionsPolling() {
        // Poll every 2 seconds to check if permissions were granted
        Timer.scheduledTimer(withTimeInterval: 2.0, repeats: true) { [weak self] timer in
            guard let self = self else {
                timer.invalidate()
                return
            }

            if let perms = self.permissionsManager, perms.allPermissionsGranted() {
                timer.invalidate()

                Logger.log("✅ All permissions granted! Starting daemon...", level: .success)

                // Start the daemon immediately
                DispatchQueue.main.async {
                    self.daemonController?.start()

                    // Show brief success notification using modern API
                    self.menuBarController?.showNotification(
                        title: "Ariata Ready",
                        message: "Now monitoring your activity and messages"
                    )

                    // Update menu bar status immediately
                    self.menuBarController?.updateStatus()
                }
            }
        }
    }
}
