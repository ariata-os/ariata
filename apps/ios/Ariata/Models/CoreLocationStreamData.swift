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
    
    private enum CodingKeys: String, CodingKey {
        case timestamp
        case latitude
        case longitude
        case altitude
        case speed
        case horizontalAccuracy = "horizontal_accuracy"
        case verticalAccuracy = "vertical_accuracy"
    }
    
    init(location: CLLocation) {
        self.timestamp = ISO8601DateFormatter().string(from: location.timestamp)
        self.latitude = location.coordinate.latitude
        self.longitude = location.coordinate.longitude
        self.altitude = location.altitude
        self.speed = max(0, location.speed) // Negative values indicate invalid speed
        self.horizontalAccuracy = location.horizontalAccuracy
        self.verticalAccuracy = location.verticalAccuracy
    }
}

struct CoreLocationStreamData: Codable {
    let streamName: String = "ios_location"
    let deviceId: String
    let data: [LocationData]
    let batchMetadata: BatchMetadata
    
    private enum CodingKeys: String, CodingKey {
        case streamName = "stream_name"
        case deviceId = "device_id"
        case data
        case batchMetadata = "batch_metadata"
    }
    
    struct BatchMetadata: Codable {
        let totalRecords: Int
        let appVersion: String
        
        private enum CodingKeys: String, CodingKey {
            case totalRecords = "total_records"
            case appVersion = "app_version"
        }
    }
    
    init(deviceId: String, locations: [LocationData]) {
        self.deviceId = deviceId
        self.data = locations
        self.batchMetadata = BatchMetadata(
            totalRecords: locations.count,
            appVersion: Bundle.main.infoDictionary?["CFBundleShortVersionString"] as? String ?? "1.0"
        )
    }
}