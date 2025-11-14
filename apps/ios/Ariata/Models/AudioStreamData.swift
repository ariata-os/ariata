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
    let source: String = "ios"
    let stream: String = "microphone"
    let deviceId: String
    let records: [AudioChunk]
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

    init(deviceId: String, chunks: [AudioChunk], checkpoint: String? = nil) {
        self.deviceId = deviceId
        self.records = chunks
        self.timestamp = ISO8601DateFormatter().string(from: Date())
        self.checkpoint = checkpoint
    }
}
