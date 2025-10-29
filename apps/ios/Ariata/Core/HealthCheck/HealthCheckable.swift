//
//  HealthCheckable.swift
//  Ariata
//
//  Protocol for managers that support health monitoring
//

import Foundation

/// Status of a health check
enum HealthStatus {
    case healthy
    case unhealthy(reason: String)
    case disabled

    var isHealthy: Bool {
        if case .healthy = self {
            return true
        }
        return false
    }
}

/// Protocol for managers that can be health-checked
protocol HealthCheckable: AnyObject {
    /// The name of the manager for logging
    var healthCheckName: String { get }

    /// Perform a health check
    /// - Returns: The current health status
    func performHealthCheck() -> HealthStatus
}
