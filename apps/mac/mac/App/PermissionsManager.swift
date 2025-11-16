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

        print("🔍 Checking Accessibility permission")
        print("   App bundle path: \(currentPath)")
        print("   Executable path: \(executablePath)")

        // Check if app was moved
        checkForAppPathChange(currentPath: currentPath)

        let options: NSDictionary = [
            kAXTrustedCheckOptionPrompt.takeRetainedValue() as String: false
        ]
        let isGranted = AXIsProcessTrustedWithOptions(options)

        print("   Result: \(isGranted ? "✅ GRANTED" : "❌ NOT GRANTED")")

        return isGranted
    }

    /// Check if Full Disk Access permission is granted
    func checkFullDiskAccess() -> Bool {
        print("🔍 Checking Full Disk Access permission")

        let messagesDB = FileManager.default.homeDirectoryForCurrentUser
            .appendingPathComponent("Library/Messages/chat.db")

        print("   Testing access to: \(messagesDB.path)")

        // Check 1: File exists and is readable
        let canRead = FileManager.default.isReadableFile(atPath: messagesDB.path)
        print("   Can read file: \(canRead)")

        // Check 2: Try to actually open the database (more reliable)
        var db: OpaquePointer?
        let dbPath = messagesDB.path
        let openResult = sqlite3_open_v2(dbPath, &db, SQLITE_OPEN_READONLY, nil)
        let canOpen = openResult == SQLITE_OK

        if db != nil {
            sqlite3_close(db)
        }

        print("   Can open database: \(canOpen) (sqlite result: \(openResult))")

        let isGranted = canRead && canOpen
        print("   Result: \(isGranted ? "✅ GRANTED" : "❌ NOT GRANTED")")

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
            print("⚠️ WARNING: App path changed!")
            print("   Previous path: \(savedPath)")
            print("   Current path:  \(currentPath)")
            print("   ⚠️ Permissions must be re-granted in System Settings")
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

            After granting permissions, you may need to restart Ariata.
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
