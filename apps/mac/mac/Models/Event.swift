import Foundation

struct Event: Codable {
    let id: Int64?
    let timestamp: Date
    let eventType: String
    let appName: String
    let bundleId: String?
    var uploaded: Bool = false
    
    init(eventType: String, appName: String, bundleId: String?) {
        self.id = nil
        self.timestamp = Date()
        self.eventType = eventType
        self.appName = appName
        self.bundleId = bundleId
        self.uploaded = false
    }
    
    enum EventType {
        static let focus = "focus_gained"
        static let unfocus = "focus_lost"
        static let launch = "launch"
        static let quit = "quit"
    }
    
    var toDictionary: [String: Any] {
        var dict: [String: Any] = [
            "timestamp": ISO8601DateFormatter().string(from: timestamp),
            "event_type": eventType,
            "app_name": appName
        ]
        if let bundleId = bundleId {
            dict["bundle_id"] = bundleId
        }
        return dict
    }
}