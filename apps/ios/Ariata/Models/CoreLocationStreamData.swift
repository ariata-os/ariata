//
//  CoreLocationStreamData.swift
//  Ariata
//
//  Data model for Core Location data to be uploaded
//

import Foundation
import CoreLocation

struct LocationData: Codable {
    let timestamp: String
    let latitude: Double
    let longitude: Double
    let altitude: Double
    let speed: Double
    let horizontalAccuracy: Double
    let verticalAccuracy: Double
    let course: Double?
    let floor: Int?

    private enum CodingKeys: String, CodingKey {
        case timestamp
        case latitude
        case longitude
        case altitude
        case speed
        case horizontalAccuracy = "horizontal_accuracy"
        case verticalAccuracy = "vertical_accuracy"
        case course
        case floor
    }

    init(location: CLLocation) {
        self.timestamp = ISO8601DateFormatter().string(from: location.timestamp)
        self.latitude = location.coordinate.latitude
        self.longitude = location.coordinate.longitude
        self.altitude = location.altitude
        self.speed = max(0, location.speed) // Negative values indicate invalid speed
        self.horizontalAccuracy = location.horizontalAccuracy
        self.verticalAccuracy = location.verticalAccuracy

        // Course is valid when >= 0
        self.course = location.course >= 0 ? location.course : nil

        // Floor is optional and may be nil
        self.floor = location.floor?.level
    }
}

struct CoreLocationStreamData: Codable {
    let source: String = "ios"
    let stream: String = "location"
    let deviceId: String
    let records: [LocationData]
    let timestamp: String
    let checkpoint: String?

    private enum CodingKeys: String, CodingKey {
        case source
        case stream
        case deviceId = "device_id"
        case records
        case timestamp
        case checkpoint
    }

    init(deviceId: String, locations: [LocationData], checkpoint: String? = nil) {
        self.deviceId = deviceId
        self.records = locations
        self.timestamp = ISO8601DateFormatter().string(from: Date())
        self.checkpoint = checkpoint
    }
}