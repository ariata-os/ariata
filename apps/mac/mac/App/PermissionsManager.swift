import Cocoa
import ApplicationServices
import SQLite3

/// Manages macOS permissions required by Ariata
class PermissionsManager {

    private let appPathKey = "LastKnownAppPath"

    /// Check if Accessibility permission is granted
    func checkAccessibility() -> Bool {
        let currentPath = Bundle.main.bundlePath
        let executablePath = Bundle.main.executablePath ?? "unknown"

        Logger.log("🔍 Checking Accessibility permission", level: .debug)
        Logger.log("   App bundle path: \(currentPath)", level: .debug)
        Logger.log("   Executable path: \(executablePath)", level: .debug)

        // Check if app was moved
        checkForAppPathChange(currentPath: currentPath)

        let options: NSDictionary = [
            kAXTrustedCheckOptionPrompt.takeRetainedValue() as String: false
        ]
        let isGranted = AXIsProcessTrustedWithOptions(options)

        Logger.log("   Accessibility Result: \(isGranted ? "✅ GRANTED" : "❌ NOT GRANTED")", level: isGranted ? .success : .warning)

        return isGranted
    }

    /// Check if Full Disk Access permission is granted
    func checkFullDiskAccess() -> Bool {
        Logger.log("🔍 Checking Full Disk Access permission", level: .debug)

        let messagesDB = FileManager.default.homeDirectoryForCurrentUser
            .appendingPathComponent("Library/Messages/chat.db")

        Logger.log("   Testing access to: \(messagesDB.path)", level: .debug)

        // Check 1: File exists and is readable
        let canRead = FileManager.default.isReadableFile(atPath: messagesDB.path)
        Logger.log("   Can read file: \(canRead)", level: .debug)

        // Check 2: Try to actually open the database (more reliable)
        var db: OpaquePointer?
        let dbPath = messagesDB.path
        let openResult = sqlite3_open_v2(dbPath, &db, SQLITE_OPEN_READONLY, nil)
        let canOpen = openResult == SQLITE_OK

        if db != nil {
            sqlite3_close(db)
        }

        Logger.log("   Can open database: \(canOpen) (sqlite result: \(openResult))", level: .debug)

        let isGranted = canRead && canOpen
        Logger.log("   Full Disk Access Result: \(isGranted ? "✅ GRANTED" : "❌ NOT GRANTED")", level: isGranted ? .success : .warning)

        return isGranted
    }

    /// Check if all required permissions are granted
    func allPermissionsGranted() -> Bool {
        return checkAccessibility() && checkFullDiskAccess()
    }

    /// Request Accessibility permission (shows system prompt)
    func requestAccessibility() {
        let options: NSDictionary = [
            kAXTrustedCheckOptionPrompt.takeRetainedValue() as String: true
        ]
        _ = AXIsProcessTrustedWithOptions(options)
    }

    /// Check if app was moved to a different path
    private func checkForAppPathChange(currentPath: String) {
        let savedPath = UserDefaults.standard.string(forKey: appPathKey)

        if let savedPath = savedPath, savedPath != currentPath {
            Logger.log("⚠️ WARNING: App path changed!", level: .warning)
            Logger.log("   Previous path: \(savedPath)", level: .warning)
            Logger.log("   Current path:  \(currentPath)", level: .warning)
            Logger.log("   ⚠️ Permissions must be re-granted in System Settings", level: .warning)
        }

        // Save current path
        UserDefaults.standard.set(currentPath, forKey: appPathKey)
    }

    /// Open System Preferences to grant permissions
    func openSystemPreferences() {
        // First, request Accessibility which will show the system prompt
        requestAccessibility()

        // Then open Full Disk Access pane
        // Note: There's no direct way to programmatically open the Full Disk Access pane
        // We open the Security & Privacy preferences and the user must navigate to it
        if let url = URL(string: "x-apple.systempreferences:com.apple.preference.security?Privacy_AllFiles") {
            NSWorkspace.shared.open(url)
        }

        // Show an alert with instructions
        DispatchQueue.main.asyncAfter(deadline: .now() + 1.0) {
            let appPath = Bundle.main.bundlePath
            let appName = (appPath as NSString).lastPathComponent

            let alert = NSAlert()
            alert.messageText = "Grant Permissions"
            alert.informativeText = """
            Please grant the following permissions:

            1. Accessibility: Click the checkbox next to "\(appName)"
            2. Full Disk Access: Navigate to "Full Disk Access" in the left sidebar,
               then click the checkbox next to "\(appName)"

            App Location: \(appPath)

            Ariata will automatically detect when permissions are granted and start monitoring.
            """
            alert.addButton(withTitle: "OK")
            alert.addButton(withTitle: "Copy App Path")
            alert.alertStyle = .informational

            let response = alert.runModal()

            if response == .alertSecondButtonReturn {
                // User clicked "Copy App Path"
                let pasteboard = NSPasteboard.general
                pasteboard.clearContents()
                pasteboard.setString(appPath, forType: .string)
            }
        }
    }
}
