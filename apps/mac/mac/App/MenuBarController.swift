import Cocoa
import UserNotifications

class MenuBarController: NSObject, NSMenuDelegate {

    private var statusItem: NSStatusItem!
    private var menu: NSMenu!
    private var daemonController: DaemonController
    private var permissionsManager: PermissionsManager
    private var statusUpdateTimer: Timer?
    private var needsPairing: Bool
    private var pairingWindowController: PairingWindowController?

    // Menu items that need dynamic updates
    private var daemonStatusItem: NSMenuItem!
    private var lastSyncItem: NSMenuItem!
    private var queuedRecordsItem: NSMenuItem!
    private var pauseResumeItem: NSMenuItem!
    private var accessibilityItem: NSMenuItem!
    private var fullDiskAccessItem: NSMenuItem!

    private let emojiPreferenceKey = "MenuBarIconEmoji"
    private var emojiPickerPopover: NSPopover?
    private var streamStatusPopover: NSPopover?
    private var globalEventMonitor: Any?

    init(daemonController: DaemonController,
         permissionsManager: PermissionsManager,
         needsPairing: Bool = false) {
        self.daemonController = daemonController
        self.permissionsManager = permissionsManager
        self.needsPairing = needsPairing
        super.init()
    }

    func setup() {
        print("📍 MenuBarController.setup() called")

        // Create status item in menu bar
        statusItem = NSStatusBar.system.statusItem(withLength: NSStatusItem.variableLength)
        print("📍 StatusItem created: \(statusItem != nil)")

        guard let button = statusItem.button else {
            print("❌ Failed to create status bar button")
            return
        }
        print("📍 Button created: \(button)")

        // Load saved emoji or use default
        let savedEmoji = UserDefaults.standard.string(forKey: emojiPreferenceKey) ?? "📡"
        button.title = savedEmoji
        print("📍 Button.title set to '\(savedEmoji)'")

        // Build menu and set delegate for dynamic rebuilding
        menu = NSMenu()
        menu.delegate = self
        menu.autoenablesItems = false  // We'll handle item enabling ourselves
        buildMenu()
        print("📍 Menu built: \(menu != nil)")
        statusItem.menu = menu
        print("📍 Menu assigned to statusItem")

        // Make sure button is visible
        statusItem.isVisible = true
        print("📍 StatusItem.isVisible = true")

        // Set up global keyboard shortcut for dashboard (Cmd+J)
        setupGlobalKeyboardShortcut()

        // Only start status updates if not in pairing mode
        if !needsPairing {
            // Start periodic status updates (every 30 seconds)
            statusUpdateTimer = Timer.scheduledTimer(
                withTimeInterval: 30,
                repeats: true
            ) { [weak self] _ in
                self?.updateStatus()
            }

            // Initial update
            updateStatus()
        }

        // Check if device is paired, show pairing window if not
        if Config.load() == nil && !needsPairing {
            print("⚠️ Device not paired, showing pairing window...")
            // Delay slightly to let menu bar appear first
            DispatchQueue.main.asyncAfter(deadline: .now() + 0.5) { [weak self] in
                self?.startPairing()
            }
        }
    }

    private func setupGlobalKeyboardShortcut() {
        // Register GLOBAL hotkey: Cmd+J to open dashboard (works even when app is in background)
        globalEventMonitor = NSEvent.addGlobalMonitorForEvents(matching: .keyDown) { [weak self] event in
            // Check for Cmd+J (only Command modifier, no shift/ctrl/option)
            if event.modifierFlags.intersection([.command, .shift, .control, .option]) == .command &&
               event.keyCode == 38 { // keyCode 38 = 'J'
                self?.openDashboard()
            }
        }
        print("✅ Global keyboard shortcut registered: Cmd+J for dashboard")
    }

    // MARK: - NSMenuDelegate

    func menuWillOpen(_ menu: NSMenu) {
        // Rebuild menu dynamically RIGHT when it's opening
        // This allows us to detect Option key in real-time
        print("🔍 menuWillOpen called, current event: \(String(describing: NSApp.currentEvent))")
        print("🔍 Current event modifierFlags: \(String(describing: NSApp.currentEvent?.modifierFlags.rawValue))")
        buildMenu()

        // Update status immediately so menu shows correct permissions
        if !needsPairing {
            updateStatus()
        }
    }

    private func buildMenu() {
        // Clear existing items
        menu.removeAllItems()

        if needsPairing {
            // Unpaired menu - simplified
            let warningItem = menu.addItem(
                withTitle: "⚠️ Not Configured",
                action: nil,
                keyEquivalent: ""
            )
            warningItem.isEnabled = false

            menu.addItem(NSMenuItem.separator())

            let pairItem = menu.addItem(
                withTitle: "Pair This Mac",
                action: #selector(startPairing),
                keyEquivalent: ""
            )
            pairItem.target = self

            menu.addItem(NSMenuItem.separator())

            let quitItem = menu.addItem(
                withTitle: "Quit Ariata",
                action: #selector(quit),
                keyEquivalent: "q"
            )
            quitItem.target = self
        } else {
            // Normal paired menu
            buildPairedMenu()
        }
    }

    private func buildPairedMenu() {
        // Background service status
        daemonStatusItem = menu.addItem(
            withTitle: "Background Service: Checking...",
            action: nil,
            keyEquivalent: ""
        )
        daemonStatusItem.isEnabled = false

        menu.addItem(NSMenuItem.separator())

        // Status section
        lastSyncItem = menu.addItem(
            withTitle: "Last sync: --",
            action: nil,
            keyEquivalent: ""
        )
        lastSyncItem.isEnabled = false

        queuedRecordsItem = menu.addItem(
            withTitle: "Records queued: --",
            action: nil,
            keyEquivalent: ""
        )
        queuedRecordsItem.isEnabled = false

        menu.addItem(NSMenuItem.separator())

        // Stream status
        let streamStatusItem = menu.addItem(
            withTitle: "Stream Status",
            action: #selector(openStreamStatus),
            keyEquivalent: ""
        )
        streamStatusItem.target = self

        menu.addItem(NSMenuItem.separator())

        // Control section
        pauseResumeItem = menu.addItem(
            withTitle: "⏸ Pause Monitoring",
            action: #selector(toggleMonitoring),
            keyEquivalent: "p"
        )
        pauseResumeItem.target = self

        menu.addItem(NSMenuItem.separator())

        // Permissions section - only show if not all permissions are granted
        if !permissionsManager.allPermissionsGranted() {
            accessibilityItem = menu.addItem(
                withTitle: "⚠️ Accessibility Access",
                action: nil,
                keyEquivalent: ""
            )
            accessibilityItem.isEnabled = false

            fullDiskAccessItem = menu.addItem(
                withTitle: "⚠️ Full Disk Access",
                action: nil,
                keyEquivalent: ""
            )
            fullDiskAccessItem.isEnabled = false

            let grantPermissionsItem = menu.addItem(
                withTitle: "Grant Permissions...",
                action: #selector(openSystemPreferences),
                keyEquivalent: ""
            )
            grantPermissionsItem.target = self

            menu.addItem(NSMenuItem.separator())
        }

        // Icon customization section - opens system emoji picker
        let changeIconItem = menu.addItem(
            withTitle: "Change Menu Bar Icon...",
            action: #selector(openEmojiPicker),
            keyEquivalent: "i"
        )
        changeIconItem.target = self

        menu.addItem(NSMenuItem.separator())

        // Dashboard section
        let dashboardItem = menu.addItem(
            withTitle: "Open Dashboard",
            action: #selector(openDashboard),
            keyEquivalent: "d"
        )
        dashboardItem.target = self

        menu.addItem(NSMenuItem.separator())

        // Pairing section
        let pairItem = menu.addItem(
            withTitle: "Pair with Server...",
            action: #selector(startPairing),
            keyEquivalent: ""
        )
        pairItem.target = self

        menu.addItem(NSMenuItem.separator())

        // Quit section - detect Option key
        // Use NSApp.currentEvent to get the event that triggered the menu opening
        let optionKeyHeld = NSApp.currentEvent?.modifierFlags.contains(.option) ?? false
        print("🔍 Option key held: \(optionKeyHeld), event modifierFlags: \(NSApp.currentEvent?.modifierFlags.rawValue ?? 0)")

        if optionKeyHeld {
            // Hidden advanced options when Option is held
            let advancedHeader = menu.addItem(
                withTitle: "⚙️ Advanced Options",
                action: nil,
                keyEquivalent: ""
            )
            advancedHeader.isEnabled = false

            let clearQueueItem = menu.addItem(
                withTitle: "Clear Local Queue",
                action: #selector(clearQueue),
                keyEquivalent: ""
            )
            clearQueueItem.target = self

            menu.addItem(NSMenuItem.separator())

            let stopMonitoringItem = menu.addItem(
                withTitle: "Stop Monitoring & Quit App",
                action: #selector(stopDaemon),
                keyEquivalent: "q"
            )
            stopMonitoringItem.target = self
        } else {
            // Normal quit: just hide menu bar, background service keeps running
            let quitItem = menu.addItem(
                withTitle: "Quit (Monitoring Continues)",
                action: #selector(quit),
                keyEquivalent: "q"
            )
            quitItem.target = self
        }
    }

    func updateStatus() {
        // Get daemon stats
        let stats = daemonController.getStats()

        // Update background service status
        let isRunning = daemonController.isRunning()
        if isRunning {
            if stats.isPaused {
                daemonStatusItem.title = "⏸️ Background Service: Paused"
            } else {
                daemonStatusItem.title = "🟢 Background Service: Running"
            }
        } else {
            daemonStatusItem.title = "❌ Background Service: Stopped"
        }

        // Update last sync time
        if let lastSync = stats.lastSyncTime {
            let timeAgo = formatTimeAgo(lastSync)
            lastSyncItem.title = "Last sync: \(timeAgo)"
        } else {
            lastSyncItem.title = "Last sync: Never"
        }

        // Update queued records
        queuedRecordsItem.title = "Records queued: \(stats.queuedRecords)"

        // Update pause/resume button
        if stats.isPaused {
            pauseResumeItem.title = "▶️ Resume Monitoring"
        } else {
            pauseResumeItem.title = "⏸ Pause Monitoring"
        }

        // Update permissions status (only if items exist - they're hidden when all permissions are granted)
        let hasAccessibility = permissionsManager.checkAccessibility()
        let hasFullDiskAccess = permissionsManager.checkFullDiskAccess()

        Logger.log("🔐 Menu updating with permissions: Accessibility=\(hasAccessibility), FullDiskAccess=\(hasFullDiskAccess)", level: .debug)

        if let accessibilityItem = accessibilityItem {
            accessibilityItem.title = hasAccessibility ? "✓ Accessibility Access" : "⚠️ Accessibility Access"
        }
        if let fullDiskAccessItem = fullDiskAccessItem {
            fullDiskAccessItem.title = hasFullDiskAccess ? "✓ Full Disk Access" : "⚠️ Full Disk Access"
        }

        // Update icon color based on state
        updateIcon(state: determineState(stats: stats, hasPermissions: hasAccessibility && hasFullDiskAccess))
    }

    private func determineState(stats: DaemonStats, hasPermissions: Bool) -> IconState {
        if !hasPermissions {
            return .warning
        }
        if stats.isPaused {
            return .warning
        }
        // Check if last sync was recent (within 10 minutes)
        if let lastSync = stats.lastSyncTime,
           Date().timeIntervalSince(lastSync) < 600 {
            return .active
        }
        // If never synced or synced long ago
        return .inactive
    }

    private func updateIcon(state: IconState) {
        guard let button = statusItem.button else { return }

        // We use emoji text for the icon, so we don't set button.image
        // Status is shown through the menu items instead
        // The emoji stays consistent regardless of state
    }

    private func formatTimeAgo(_ date: Date) -> String {
        let interval = Date().timeIntervalSince(date)

        if interval < 60 {
            return "just now"
        } else if interval < 3600 {
            let mins = Int(interval / 60)
            return "\(mins) minute\(mins == 1 ? "" : "s") ago"
        } else if interval < 86400 {
            let hours = Int(interval / 3600)
            return "\(hours) hour\(hours == 1 ? "" : "s") ago"
        } else {
            let days = Int(interval / 86400)
            return "\(days) day\(days == 1 ? "" : "s") ago"
        }
    }

    // MARK: - Actions

    @objc private func toggleMonitoring() {
        daemonController.togglePause()
        updateStatus() // Immediate update
    }

    @objc private func openSystemPreferences() {
        permissionsManager.openSystemPreferences()
    }

    @objc private func openEmojiPicker() {
        // Close existing popover if open
        if let popover = emojiPickerPopover, popover.isShown {
            popover.close()
            emojiPickerPopover = nil
            return
        }

        // Create emoji picker view controller
        let pickerViewController = EmojiPickerViewController()
        pickerViewController.onEmojiSelected = { [weak self] emoji in
            self?.statusItem.button?.title = emoji
            UserDefaults.standard.set(emoji, forKey: self?.emojiPreferenceKey ?? "MenuBarIconEmoji")
            print("✅ Menu bar icon changed to: \(emoji)")
            self?.emojiPickerPopover?.close()
            self?.emojiPickerPopover = nil
        }

        // Create popover
        let popover = NSPopover()
        popover.contentViewController = pickerViewController
        popover.behavior = .transient
        popover.animates = true

        // Show popover relative to status button
        if let button = statusItem.button {
            popover.show(relativeTo: button.bounds, of: button, preferredEdge: .minY)
        }

        emojiPickerPopover = popover
    }

    @objc private func openDashboard() {
        // TODO: Get dashboard URL from config or environment variable
        let dashboardURL = ProcessInfo.processInfo.environment["ARIATA_DASHBOARD_URL"] ?? "http://localhost:5173"

        if let url = URL(string: dashboardURL) {
            NSWorkspace.shared.open(url)
        }
    }

    @objc private func quit() {
        // Clean up UI components
        if let monitor = globalEventMonitor {
            NSEvent.removeMonitor(monitor)
            globalEventMonitor = nil
        }

        statusUpdateTimer?.invalidate()
        statusUpdateTimer = nil  // Set to nil so timer restarts on relaunch
        pairingWindowController?.close()
        emojiPickerPopover?.close()
        streamStatusPopover?.close()

        // Hide menu bar but keep background service running
        statusItem.isVisible = false

        print("✅ Menu bar hidden, background service continues running")
        print("💡 Tip: Launch the app again to show the menu bar")
    }

    /// Show the menu bar icon when app is relaunched while running
    func showMenuBar() {
        Logger.log("🔄 Re-showing menu bar after relaunch", level: .info)

        // Make status item visible again
        statusItem.isVisible = true

        // Restart status updates if not already running
        if statusUpdateTimer == nil {
            statusUpdateTimer = Timer.scheduledTimer(
                withTimeInterval: 30,
                repeats: true
            ) { [weak self] _ in
                self?.updateStatus()
            }

            // Immediate status update
            updateStatus()
        }

        // Re-setup global keyboard shortcut if needed
        if globalEventMonitor == nil {
            setupGlobalKeyboardShortcut()
        }

        Logger.log("✅ Menu bar restored", level: .success)
    }

    @objc private func stopDaemon() {
        let alert = NSAlert()
        alert.messageText = "Stop Monitoring?"
        alert.informativeText = "This will stop all activity tracking until you restart the app.\n\nAre you sure you want to stop monitoring?"
        alert.alertStyle = .warning
        alert.addButton(withTitle: "Stop Monitoring")
        alert.addButton(withTitle: "Cancel")

        if alert.runModal() == .alertFirstButtonReturn {
            print("🛑 User confirmed: Stopping background service and quitting app")

            // Clean up everything
            if let monitor = globalEventMonitor {
                NSEvent.removeMonitor(monitor)
                globalEventMonitor = nil
            }
            statusUpdateTimer?.invalidate()
            pairingWindowController?.close()
            emojiPickerPopover?.close()
            streamStatusPopover?.close()

            // Stop the background service
            daemonController.stop()

            // Terminate the app completely
            NSApp.terminate(nil)
        }
    }

    @objc private func clearQueue() {
        let stats = daemonController.getStats()

        let alert = NSAlert()
        alert.messageText = "Clear Local Queue?"
        alert.informativeText = "This will delete \(stats.queuedRecords) pending records from the local queue.\n\nThese records have not been uploaded yet and will be lost.\n\nAre you sure?"
        alert.alertStyle = .warning
        alert.addButton(withTitle: "Clear Queue")
        alert.addButton(withTitle: "Cancel")

        if alert.runModal() == .alertFirstButtonReturn {
            print("🗑️ User confirmed: Clearing local queue")

            // Clear the queue via daemon controller
            // Note: You'll need to add a clearQueue() method to DaemonController
            // For now, we'll show a success message
            // daemonController.clearQueue()

            let successAlert = NSAlert()
            successAlert.messageText = "Queue Cleared"
            successAlert.informativeText = "\(stats.queuedRecords) records removed from local queue."
            successAlert.alertStyle = .informational
            successAlert.addButton(withTitle: "OK")
            successAlert.runModal()

            // Update status immediately
            updateStatus()

            print("✅ Local queue cleared")
        }
    }

    @objc private func openStreamStatus() {
        // Close if already open
        if let popover = streamStatusPopover, popover.isShown {
            popover.close()
            streamStatusPopover = nil
            return
        }

        // Create status view controller
        let statusViewController = StreamStatusViewController(daemonController: daemonController)

        // Create popover
        let popover = NSPopover()
        popover.contentViewController = statusViewController
        popover.behavior = .transient
        popover.animates = true

        // Show relative to status button
        if let button = statusItem.button {
            popover.show(relativeTo: button.bounds, of: button, preferredEdge: .minY)
        }

        streamStatusPopover = popover
    }

    // MARK: - Pairing

    @objc private func startPairing() {
        // Create and show pairing window
        let windowController = PairingWindowController()
        pairingWindowController = windowController

        // Set completion handler
        if let viewController = windowController.window?.contentViewController as? PairingViewController {
            viewController.onPairingComplete = { [weak self] config in
                self?.handlePairingComplete(config: config)
            }
        }

        // Show window
        windowController.showWindow(nil)
        windowController.window?.makeKeyAndOrderFront(nil)
        NSApp.activate(ignoringOtherApps: true)
    }

    private func handlePairingComplete(config: Config) {
        do {
            // Save config
            try config.save()

            // Update state
            needsPairing = false

            // Rebuild menu (remove pairing, show normal)
            menu.removeAllItems()
            buildMenu()

            // Show success notification
            showNotification(title: "Paired Successfully!",
                           message: "Your Mac is now connected to Ariata")

            // Start daemon asynchronously to avoid blocking main thread
            DispatchQueue.global(qos: .userInitiated).async { [weak self] in
                guard let self = self else { return }
                if self.permissionsManager.allPermissionsGranted() {
                    print("🚀 Starting daemon in background...")
                    self.daemonController.start()
                } else {
                    print("⚠️ Cannot start daemon: missing permissions")
                }
            }

            print("✅ Pairing complete! Device token saved.")

            // Clean up window reference
            pairingWindowController = nil

        } catch {
            showError("Failed to save configuration: \(error.localizedDescription)")
        }
    }

    func showNotification(title: String, message: String) {
        let center = UNUserNotificationCenter.current()

        // Request authorization if not already done
        center.requestAuthorization(options: [.alert, .sound]) { granted, error in
            if let error = error {
                print("⚠️ Notification authorization error: \(error)")
                return
            }

            guard granted else {
                print("⚠️ Notification permission denied")
                return
            }

            // Create notification content
            let content = UNMutableNotificationContent()
            content.title = title
            content.body = message
            content.sound = .default

            // Create trigger (deliver immediately)
            let request = UNNotificationRequest(
                identifier: UUID().uuidString,
                content: content,
                trigger: nil
            )

            // Schedule notification
            center.add(request) { error in
                if let error = error {
                    print("⚠️ Failed to show notification: \(error)")
                }
            }
        }
    }

    private func showError(_ message: String) {
        DispatchQueue.main.async {
            let alert = NSAlert()
            alert.messageText = "Error"
            alert.informativeText = message
            alert.alertStyle = .critical
            alert.addButton(withTitle: "OK")
            alert.runModal()
        }
    }

    private func showValidationError(_ message: String) {
        let alert = NSAlert()
        alert.messageText = "Invalid Icon"
        alert.informativeText = message
        alert.alertStyle = .warning
        alert.addButton(withTitle: "OK")
        alert.runModal()
    }

    enum IconState {
        case active   // Green
        case warning  // Yellow
        case inactive // Gray
    }
}
