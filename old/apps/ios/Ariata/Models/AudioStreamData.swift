//
//  AudioStreamData.swift
//  Ariata
//
//  Data model for audio recordings to be uploaded
//

import Foundation

struct AudioChunk: Codable {
    let id: String
    let timestampStart: String
    let timestampEnd: String
    let duration: Double
    let audioData: String  // Base64 encoded Opus audio
    let overlapDuration: Double

    private enum CodingKeys: String, CodingKey {
        case id
        case timestampStart = "timestamp_start"
        case timestampEnd = "timestamp_end"
        case duration
        case audioData = "audio_data"
        case overlapDuration = "overlap_duration"
    }

    init(
        id: String = UUID().uuidString, startDate: Date, endDate: Date, audioData: Data,
        overlapDuration: Double = 2.0
    ) {
        self.id = id
        self.timestampStart = ISO8601DateFormatter().string(from: startDate)
        self.timestampEnd = ISO8601DateFormatter().string(from: endDate)
        self.duration = endDate.timeIntervalSince(startDate)
        self.audioData = audioData.base64EncodedString()
        self.overlapDuration = overlapDuration
    }
}

struct AudioStreamData: Codable {
    let streamName: String = "ios_mic"
    let deviceId: String
    let data: [AudioChunk]
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

    init(deviceId: String, chunks: [AudioChunk]) {
        self.deviceId = deviceId
        self.data = chunks
        self.batchMetadata = BatchMetadata(
            totalRecords: chunks.count,
            appVersion: Bundle.main.infoDictionary?["CFBundleShortVersionString"] as? String
                ?? "1.0"
        )
    }
}
