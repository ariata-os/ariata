//
//  StreamProcessors.swift
//  Ariata
//
//  Concrete stream processor implementations for each data type
//

import Foundation

/// Factory for creating stream processors
enum StreamProcessorFactory {
    /// Create a processor for a given stream name
    static func processor(for streamName: String) -> (any StreamDataProcessor)? {
        switch streamName {
        case "ios_healthkit":
            return HealthKitStreamProcessor()

        case "ios_location":
            return LocationStreamProcessor()

        case "ios_mic":
            return AudioStreamProcessor()

        default:
            return nil
        }
    }
}

// MARK: - HealthKit Stream Processor

struct HealthKitStreamProcessor: StreamDataProcessor {
    typealias DataType = HealthKitMetric
    typealias StreamDataType = HealthKitStreamData

    let streamName = "ios_healthkit"

    func decode(_ data: Data) throws -> [HealthKitMetric] {
        let decoder = JSONDecoder()
        decoder.dateDecodingStrategy = .iso8601
        let streamData = try decoder.decode(HealthKitStreamData.self, from: data)
        return streamData.records
    }

    func combine(_ items: [HealthKitMetric], deviceId: String) -> HealthKitStreamData {
        return HealthKitStreamData(deviceId: deviceId, metrics: items)
    }
}

// MARK: - Location Stream Processor

struct LocationStreamProcessor: StreamDataProcessor {
    typealias DataType = LocationData
    typealias StreamDataType = CoreLocationStreamData

    let streamName = "ios_location"

    func decode(_ data: Data) throws -> [LocationData] {
        let decoder = JSONDecoder()
        decoder.dateDecodingStrategy = .iso8601
        let streamData = try decoder.decode(CoreLocationStreamData.self, from: data)
        return streamData.records
    }

    func combine(_ items: [LocationData], deviceId: String) -> CoreLocationStreamData {
        return CoreLocationStreamData(deviceId: deviceId, locations: items)
    }
}

// MARK: - Audio Stream Processor

struct AudioStreamProcessor: StreamDataProcessor {
    typealias DataType = AudioChunk
    typealias StreamDataType = AudioStreamData

    let streamName = "ios_mic"

    func decode(_ data: Data) throws -> [AudioChunk] {
        let decoder = JSONDecoder()
        decoder.dateDecodingStrategy = .iso8601
        let streamData = try decoder.decode(AudioStreamData.self, from: data)
        return streamData.records
    }

    func combine(_ items: [AudioChunk], deviceId: String) -> AudioStreamData {
        return AudioStreamData(deviceId: deviceId, chunks: items)
    }
}
