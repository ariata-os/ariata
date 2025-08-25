//
//  HealthKitStreamData.swift
//  Ariata
//
//  Data model for HealthKit metrics to be uploaded
//

import Foundation

struct HealthKitMetric: Codable {
    let timestamp: String
    let metricType: String
    let value: Double
    let unit: String
    let metadata: [String: AnyCodable]?
    
    private enum CodingKeys: String, CodingKey {
        case timestamp
        case metricType = "metric_type"
        case value
        case unit
        case metadata
    }
    
    init(timestamp: Date, metricType: String, value: Double, unit: String, metadata: [String: Any]? = nil) {
        self.timestamp = ISO8601DateFormatter().string(from: timestamp)
        self.metricType = metricType
        self.value = value
        self.unit = unit
        
        // Convert metadata to AnyCodable
        if let metadata = metadata {
            var codableMetadata: [String: AnyCodable] = [:]
            for (key, value) in metadata {
                codableMetadata[key] = AnyCodable(value)
            }
            self.metadata = codableMetadata
        } else {
            self.metadata = nil
        }
    }
}

// Helper struct to make Any codable
struct AnyCodable: Codable {
    let value: Any
    
    init(_ value: Any) {
        self.value = value
    }
    
    init(from decoder: Decoder) throws {
        let container = try decoder.singleValueContainer()
        
        if let value = try? container.decode(Bool.self) {
            self.value = value
        } else if let value = try? container.decode(Int.self) {
            self.value = value
        } else if let value = try? container.decode(Double.self) {
            self.value = value
        } else if let value = try? container.decode(String.self) {
            self.value = value
        } else if container.decodeNil() {
            self.value = NSNull()
        } else {
            throw DecodingError.dataCorruptedError(in: container, debugDescription: "AnyCodable value cannot be decoded")
        }
    }
    
    func encode(to encoder: Encoder) throws {
        var container = encoder.singleValueContainer()
        
        switch value {
        case let value as Bool:
            try container.encode(value)
        case let value as Int:
            try container.encode(value)
        case let value as Double:
            try container.encode(value)
        case let value as String:
            try container.encode(value)
        case is NSNull:
            try container.encodeNil()
        default:
            let context = EncodingError.Context(codingPath: container.codingPath, debugDescription: "AnyCodable value cannot be encoded")
            throw EncodingError.invalidValue(value, context)
        }
    }
}

struct HealthKitStreamData: Codable {
    let streamName: String = "ios_healthkit"
    let deviceId: String
    let data: [HealthKitMetric]
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
    
    init(deviceId: String, metrics: [HealthKitMetric]) {
        self.deviceId = deviceId
        self.data = metrics
        self.batchMetadata = BatchMetadata(
            totalRecords: metrics.count,
            appVersion: Bundle.main.infoDictionary?["CFBundleShortVersionString"] as? String ?? "1.0"
        )
    }
}

// Helper for normalized values
extension Double {
    func roundedForHealthKit(metricType: String) -> Double {
        switch metricType {
        case "heart_rate", "steps":
            return rounded()
        case "distance":
            return (self * 100).rounded() / 100
        case "active_energy", "heart_rate_variability":
            return (self * 10).rounded() / 10
        default:
            return self
        }
    }
}