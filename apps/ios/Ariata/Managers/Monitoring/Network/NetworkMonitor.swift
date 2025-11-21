//
//  NetworkMonitor.swift
//  Ariata
//
//  Monitors network conditions and adapts batch sizes for uploads
//

import Foundation
import Network
import Combine

/// Monitors network conditions and provides adaptive batch sizes
class NetworkMonitor: ObservableObject {
    static let shared = NetworkMonitor()

    // MARK: - Published Properties

    @Published var isConnected: Bool = false
    @Published var currentNetworkType: NetworkType = .unknown
    @Published var currentBatchSize: Int = 100 // Start conservative

    // MARK: - Constants

    private let minBatchSize = 20        // Minimum batch size (poor network)
    private let maxBatchSize = 500       // Maximum batch size (excellent network)
    private let defaultBatchSize = 100   // Default starting size

    // Batch size by network type
    private let wifiBatchSize = 500      // WiFi: large batches
    private let cellularBatchSize = 100  // Cellular: medium batches
    private let unknownBatchSize = 50    // Unknown: small batches

    // MARK: - Private Properties

    private let monitor = NWPathMonitor()
    private let queue = DispatchQueue(label: "com.ariata.network.monitor")

    // Success tracking for adaptive adjustment
    private var recentUploadResults: [Bool] = []  // true = success, false = failure
    private let maxResultHistory = 10  // Track last 10 uploads

    private init() {
        setupNetworkMonitoring()
    }

    // MARK: - Network Monitoring

    private func setupNetworkMonitoring() {
        monitor.pathUpdateHandler = { [weak self] path in
            guard let self = self else { return }

            DispatchQueue.main.async {
                self.isConnected = (path.status == .satisfied)
                self.currentNetworkType = self.determineNetworkType(from: path)
                self.updateBatchSize()

                #if DEBUG
                print("📡 Network status: \(self.isConnected ? "Connected" : "Disconnected"), Type: \(self.currentNetworkType.description)")
                #endif
            }
        }

        monitor.start(queue: queue)
    }

    private func determineNetworkType(from path: NWPath) -> NetworkType {
        if path.usesInterfaceType(.wifi) {
            return .wifi
        } else if path.usesInterfaceType(.cellular) {
            return .cellular
        } else if path.usesInterfaceType(.wiredEthernet) {
            return .wired
        } else {
            return .unknown
        }
    }

    // MARK: - Adaptive Batch Sizing

    /// Get recommended batch size based on current network conditions
    func getRecommendedBatchSize() -> Int {
        return currentBatchSize
    }

    /// Update batch size based on network type and recent success rate
    private func updateBatchSize() {
        guard isConnected else {
            currentBatchSize = minBatchSize
            return
        }

        // Start with base size for network type
        var baseSize: Int
        switch currentNetworkType {
        case .wifi, .wired:
            baseSize = wifiBatchSize
        case .cellular:
            baseSize = cellularBatchSize
        case .unknown:
            baseSize = unknownBatchSize
        }

        // Adjust based on recent success rate
        let successRate = calculateSuccessRate()

        // If we don't have enough data yet, be optimistic
        if recentUploadResults.count < 3 {
            // Not enough data - use full base size
            currentBatchSize = baseSize
        } else {
            if successRate >= 0.9 {
                // Excellent success rate - use full base size or increase
                currentBatchSize = min(baseSize, maxBatchSize)
            } else if successRate >= 0.7 {
                // Good success rate - use 75% of base size
                currentBatchSize = Int(Double(baseSize) * 0.75)
            } else if successRate >= 0.5 {
                // Moderate success rate - use 50% of base size
                currentBatchSize = Int(Double(baseSize) * 0.5)
            } else {
                // Poor success rate - use minimum
                currentBatchSize = minBatchSize
            }
        }

        // Ensure within bounds
        currentBatchSize = max(minBatchSize, min(maxBatchSize, currentBatchSize))

        #if DEBUG
        print("Batch size updated: \(currentBatchSize) (network: \(currentNetworkType.description), success rate: \(String(format: "%.1f%%", successRate * 100)))")
        #endif
    }

    // MARK: - Upload Result Tracking

    /// Record the result of an upload attempt
    func recordUploadResult(success: Bool) {
        DispatchQueue.main.async { [weak self] in
            guard let self = self else { return }

            // Add result to history
            self.recentUploadResults.append(success)

            // Keep only recent results
            if self.recentUploadResults.count > self.maxResultHistory {
                self.recentUploadResults.removeFirst()
            }

            // Update batch size based on new data
            self.updateBatchSize()
        }
    }

    /// Calculate success rate from recent uploads
    private func calculateSuccessRate() -> Double {
        guard !recentUploadResults.isEmpty else {
            return 1.0 // Assume good until proven otherwise
        }

        let successCount = recentUploadResults.filter { $0 }.count
        return Double(successCount) / Double(recentUploadResults.count)
    }

    /// Reset success tracking (useful after network type change)
    func resetSuccessTracking() {
        DispatchQueue.main.async { [weak self] in
            self?.recentUploadResults.removeAll()
            self?.updateBatchSize()
        }
    }

    // MARK: - Network Quality Assessment

    /// Get a human-readable description of current network quality
    var networkQualityDescription: String {
        guard isConnected else {
            return "No Connection"
        }

        // If we don't have enough data (< 3 uploads), show just network type
        guard recentUploadResults.count >= 3 else {
            return "\(currentNetworkType.description) (measuring...)"
        }

        let successRate = calculateSuccessRate()

        if successRate >= 0.9 {
            return "Excellent (\(currentNetworkType.description))"
        } else if successRate >= 0.7 {
            return "Good (\(currentNetworkType.description))"
        } else if successRate >= 0.5 {
            return "Fair (\(currentNetworkType.description))"
        } else {
            return "Poor (\(currentNetworkType.description))"
        }
    }

    deinit {
        monitor.cancel()
    }
}
