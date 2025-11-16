import Cocoa

/// AppDelegate for GUI/daemon modes
/// Note: No @main here - main.swift handles routing and launches NSApplication
class AppDelegate: NSObject, NSApplicationDelegate {

    private var menuBarController: MenuBarController?
    private var daemonController: DaemonController?
    private var permissionsManager: PermissionsManager?
    private var isDaemonMode = false

    func applicationDidFinishLaunching(_ notification: Notification) {
        // Log app information at startup
        print("🚀 Ariata Mac starting up")
        print("   Bundle path: \(Bundle.main.bundlePath)")
        print("   Executable: \(Bundle.main.executablePath ?? "unknown")")
        print("   Bundle ID: \(Bundle.main.bundleIdentifier ?? "unknown")")

        // Check if launched in daemon mode (--daemon flag)
        isDaemonMode = CommandLine.arguments.contains("--daemon")

        // Initialize managers
        permissionsManager = PermissionsManager()
        daemonController = DaemonController()

        if isDaemonMode {
            // Headless daemon mode - no menu bar, no dock icon
            NSApp.setActivationPolicy(.prohibited)
            print("Starting in daemon mode (headless)...")

            // Start daemon directly
            daemonController?.start()
        } else {
            // Interactive mode - show menu bar
            NSApp.setActivationPolicy(.accessory) // No dock icon, but menu bar visible
            print("Starting in interactive mode (menu bar)...")

            // Check if app is paired (config exists)
            print("🔍 Checking if app is paired...")
            let isPaired = Config.load() != nil
            print("📱 Paired status: \(isPaired)")

            // Initialize menu bar
            print("🎨 Creating menu bar controller...")
            menuBarController = MenuBarController(
                daemonController: daemonController!,
                permissionsManager: permissionsManager!,
                needsPairing: !isPaired
            )
            print("🎨 Calling menuBarController.setup()...")
            menuBarController?.setup()
            print("✅ Menu bar setup complete")

            // Only start daemon if paired AND has permissions
            if isPaired {
                // Delay permission check slightly to let menu bar appear first
                DispatchQueue.main.asyncAfter(deadline: .now() + 0.5) { [weak self] in
                    guard let self = self else { return }

                    print("🔐 Checking permissions...")
                    if let perms = self.permissionsManager, perms.allPermissionsGranted() {
                        print("✅ All permissions granted, starting daemon...")
                        self.daemonController?.start()
                    } else {
                        print("⚠️ Missing permissions, showing alert...")
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
            print("Stopping daemon...")
            daemonController?.stop()
        }
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

                print("✅ All permissions granted! Starting daemon...")

                // Start the daemon immediately
                DispatchQueue.main.async {
                    self.daemonController?.start()

                    // Show brief success notification (non-blocking)
                    let notification = NSUserNotification()
                    notification.title = "Ariata Ready"
                    notification.informativeText = "Now monitoring your activity and messages"
                    notification.soundName = NSUserNotificationDefaultSoundName
                    NSUserNotificationCenter.default.deliver(notification)
                }
            }
        }
    }
}
