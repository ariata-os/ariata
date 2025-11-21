//
//  NetworkType.swift
//  Ariata
//
//

/// Network connection type
enum NetworkType {
    case wifi
    case cellular
    case wired
    case unknown

    var description: String {
        switch self {
        case .wifi: return "WiFi"
        case .cellular: return "Cellular"
        case .wired: return "Wired"
        case .unknown: return "Unknown"
        }
    }
}
