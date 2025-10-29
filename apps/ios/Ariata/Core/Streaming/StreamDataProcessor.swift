//
//  StreamDataProcessor.swift
//  Ariata
//
//  Generic protocol for processing stream data before upload
//  Eliminates code duplication in BatchUploadCoordinator
//

import Foundation

/// Protocol for processing stream data of a specific type
protocol StreamDataProcessor {
    /// The type of individual data items in the stream
    associatedtype DataType: Codable

    /// The combined stream data type for upload
    associatedtype StreamDataType: Codable

    /// The stream name identifier (e.g., "ios_mic", "ios_location")
    var streamName: String { get }

    /// Decode a data blob into individual data items
    /// - Parameter data: The encoded data blob
    /// - Returns: Array of decoded data items
    /// - Throws: Decoding errors
    func decode(_ data: Data) throws -> [DataType]

    /// Combine multiple data items into a single stream data object
    /// - Parameters:
    ///   - items: The individual data items to combine
    ///   - deviceId: The device identifier
    /// - Returns: Combined stream data ready for upload
    func combine(_ items: [DataType], deviceId: String) -> StreamDataType
}

/// Generic processor implementation for batch uploads
struct GenericStreamProcessor<Data: Codable, StreamData: Codable>: StreamDataProcessor {
    typealias DataType = Data
    typealias StreamDataType = StreamData

    let streamName: String
    private let dataExtractor: (StreamData) -> [Data]
    private let dataCombiner: ([Data], String) -> StreamData

    init(streamName: String,
         dataExtractor: @escaping (StreamData) -> [Data],
         dataCombiner: @escaping ([Data], String) -> StreamData) {
        self.streamName = streamName
        self.dataExtractor = dataExtractor
        self.dataCombiner = dataCombiner
    }

    func decode(_ data: Foundation.Data) throws -> [Data] {
        let decoder = JSONDecoder()
        decoder.dateDecodingStrategy = .iso8601
        let streamData = try decoder.decode(StreamData.self, from: data)
        return dataExtractor(streamData)
    }

    func combine(_ items: [Data], deviceId: String) -> StreamData {
        return dataCombiner(items, deviceId)
    }
}
