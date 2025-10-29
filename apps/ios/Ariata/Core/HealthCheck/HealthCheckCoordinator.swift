//
//  HealthCheckCoordinator.swift
//  Ariata
//
//  Centralized health monitoring for all data collection managers
//  Reduces main thread work and consolidates health check logic
//

import Foundation

/// Coordinates health checks across all managers
final class HealthCheckCoordinator {
    static let shared = HealthCheckCoordinator()

    // MARK: - Properties

    private var registeredManagers: [HealthCheckable] = []
    private var healthCheckTimer: ReliableTimer?
    private let healthCheckInterval: TimeInterval = 30.0
    private let lock = NSLock()

    // MARK: - Health Status Tracking

    private(set) var lastCheckDate: Date?
    private(set) var managerStatuses: [String: HealthStatus] = [:]

    private init() {}

    // MARK: - Registration

    /// Register a manager for health monitoring
    /// - Parameter manager: The manager to monitor
    func register(_ manager: HealthCheckable) {
        lock.lock()
        defer { lock.unlock() }

        // Avoid duplicate registrations
        if !registeredManagers.contains(where: { $0 === manager }) {
            registeredManagers.append(manager)
            print("🏥 Registered \(manager.healthCheckName) for health monitoring")
        }
    }

    /// Unregister a manager from health monitoring
    /// - Parameter manager: The manager to remove
    func unregister(_ manager: HealthCheckable) {
        lock.lock()
        defer { lock.unlock() }

        registeredManagers.removeAll { $0 === manager }
        managerStatuses.removeValue(forKey: manager.healthCheckName)
        print("🏥 Unregistered \(manager.healthCheckName) from health monitoring")
    }

    // MARK: - Health Check Coordination

    /// Start the coordinated health check timer
    func startMonitoring() {
        lock.lock()
        let managersCount = registeredManagers.count
        lock.unlock()

        print("🏥 Starting coordinated health monitoring for \(managersCount) managers")

        // Stop any existing timer
        stopMonitoring()

        // Create a single timer that checks all managers
        healthCheckTimer = ReliableTimer.builder()
            .interval(healthCheckInterval)
            .queue(.main)  // Run on main for thread safety with managers
            .handler { [weak self] in
                self?.performAllHealthChecks()
            }
            .build()

        // Perform initial health check
        performAllHealthChecks()
    }

    /// Stop health monitoring
    func stopMonitoring() {
        healthCheckTimer?.cancel()
        healthCheckTimer = nil
        print("🏥 Stopped coordinated health monitoring")
    }

    /// Perform health checks on all registered managers
    private func performAllHealthChecks() {
        lock.lock()
        let managers = registeredManagers
        lock.unlock()

        guard !managers.isEmpty else { return }

        lastCheckDate = Date()
        var healthyCount = 0
        var unhealthyCount = 0
        var disabledCount = 0

        for manager in managers {
            let status = manager.performHealthCheck()

            lock.lock()
            managerStatuses[manager.healthCheckName] = status
            lock.unlock()

            switch status {
            case .healthy:
                healthyCount += 1

            case .unhealthy(let reason):
                unhealthyCount += 1
                print("⚠️ [\(manager.healthCheckName)] Unhealthy: \(reason)")

            case .disabled:
                disabledCount += 1
            }
        }

        #if DEBUG
        if unhealthyCount > 0 {
            print("🏥 Health check complete: \(healthyCount) healthy, \(unhealthyCount) unhealthy, \(disabledCount) disabled")
        }
        #endif
    }

    /// Manually trigger a health check cycle
    func triggerHealthCheck() {
        performAllHealthChecks()
    }

    // MARK: - Status Reporting

    /// Get the current health status summary
    func getHealthSummary() -> HealthCheckSummary {
        lock.lock()
        defer { lock.unlock() }

        let healthy = managerStatuses.values.filter { $0.isHealthy }.count
        let unhealthy = managerStatuses.values.filter {
            if case .unhealthy = $0 { return true }
            return false
        }.count
        let disabled = managerStatuses.values.filter {
            if case .disabled = $0 { return true }
            return false
        }.count

        return HealthCheckSummary(
            totalManagers: registeredManagers.count,
            healthy: healthy,
            unhealthy: unhealthy,
            disabled: disabled,
            lastCheckDate: lastCheckDate
        )
    }

    /// Get detailed status for a specific manager
    func getStatus(for managerName: String) -> HealthStatus? {
        lock.lock()
        defer { lock.unlock() }
        return managerStatuses[managerName]
    }

    /// Get all manager statuses
    func getAllStatuses() -> [String: HealthStatus] {
        lock.lock()
        defer { lock.unlock() }
        return managerStatuses
    }
}

// MARK: - Health Check Summary

struct HealthCheckSummary {
    let totalManagers: Int
    let healthy: Int
    let unhealthy: Int
    let disabled: Int
    let lastCheckDate: Date?

    var isAllHealthy: Bool {
        unhealthy == 0 && healthy > 0
    }

    var description: String {
        """
        Health Summary:
          Total: \(totalManagers)
          Healthy: \(healthy)
          Unhealthy: \(unhealthy)
          Disabled: \(disabled)
          Last Check: \(lastCheckDate?.formatted() ?? "Never")
        """
    }
}
