import Foundation
import IOKit

/// Helper for getting unique device identifier
class DeviceIdentifier {

    /// Get hardware UUID for this Mac
    /// Falls back to random UUID if hardware UUID unavailable
    static func getMachineUUID() -> String {
        // Get platform expert
        let platformExpert = IOServiceGetMatchingService(
            kIOMainPortDefault,
            IOServiceMatching("IOPlatformExpertDevice")
        )

        defer { IOObjectRelease(platformExpert) }

        // Get hardware UUID
        guard let uuid = IORegistryEntryCreateCFProperty(
            platformExpert,
            kIOPlatformUUIDKey as CFString,
            kCFAllocatorDefault,
            0
        )?.takeRetainedValue() as? String else {
            // Fallback to random UUID (save it for consistency)
            return getFallbackUUID()
        }

        return uuid
    }

    /// Get or create fallback UUID
    private static func getFallbackUUID() -> String {
        let key = "com.ariata.mac.device-uuid"

        // Check if we already generated one
        if let existing = UserDefaults.standard.string(forKey: key) {
            return existing
        }

        // Generate new one and save
        let newUUID = UUID().uuidString
        UserDefaults.standard.set(newUUID, forKey: key)
        return newUUID
    }
}
