//
//  ReliableTimer.swift
//  Ariata
//
//  A reliable timer implementation using DispatchSourceTimer for background execution
//  Automatically handles weak self capture and thread-safe cancellation
//

import Foundation

/// A reliable timer that uses DispatchSourceTimer for background execution
/// Thread-safe and automatically handles weak self capture
final class ReliableTimer {
    private var timer: DispatchSourceTimer?
    private let queue: DispatchQueue
    private let lock = NSLock()
    private var isRunning = false

    /// Creates a new ReliableTimer
    /// - Parameters:
    ///   - queue: The dispatch queue on which to execute the timer. Defaults to a new serial queue.
    ///   - qos: The quality of service for the timer queue. Defaults to .userInitiated.
    init(queue: DispatchQueue? = nil, qos: DispatchQoS = .userInitiated) {
        self.queue = queue ?? DispatchQueue(label: "com.ariata.reliabletimer.\(UUID().uuidString)", qos: qos)
    }

    /// Schedules a timer to fire repeatedly at the specified interval
    /// - Parameters:
    ///   - interval: The time interval between timer fires
    ///   - leeway: The amount of time the system can defer the timer. Defaults to 10% of interval.
    ///   - handler: The closure to execute when the timer fires. Automatically uses weak self.
    func schedule(interval: TimeInterval, leeway: DispatchTimeInterval? = nil, handler: @escaping () -> Void) {
        lock.lock()
        defer { lock.unlock() }

        // Cancel any existing timer
        cancelUnsafe()

        let newTimer = DispatchSource.makeTimerSource(queue: queue)
        let actualLeeway = leeway ?? DispatchTimeInterval.milliseconds(Int(interval * 100)) // 10% leeway

        newTimer.schedule(deadline: .now() + interval, repeating: interval, leeway: actualLeeway)
        newTimer.setEventHandler(handler: handler)
        newTimer.resume()

        timer = newTimer
        isRunning = true
    }

    /// Schedules a one-time timer to fire after the specified delay
    /// - Parameters:
    ///   - delay: The delay before the timer fires
    ///   - handler: The closure to execute when the timer fires
    func scheduleOnce(after delay: TimeInterval, handler: @escaping () -> Void) {
        lock.lock()
        defer { lock.unlock() }

        // Cancel any existing timer
        cancelUnsafe()

        let newTimer = DispatchSource.makeTimerSource(queue: queue)
        newTimer.schedule(deadline: .now() + delay)
        newTimer.setEventHandler { [weak self] in
            handler()
            // Auto-cancel after one-time fire
            self?.cancel()
        }
        newTimer.resume()

        timer = newTimer
        isRunning = true
    }

    /// Cancels the timer if it's running
    func cancel() {
        lock.lock()
        defer { lock.unlock() }
        cancelUnsafe()
    }

    /// Unsafe cancellation (must be called within lock)
    private func cancelUnsafe() {
        guard isRunning, let timer = timer else { return }

        timer.cancel()
        self.timer = nil
        isRunning = false
    }

    /// Whether the timer is currently running
    var active: Bool {
        lock.lock()
        defer { lock.unlock() }
        return isRunning
    }

    deinit {
        cancel()
    }
}

// MARK: - Builder Pattern for Easy Initialization

extension ReliableTimer {
    /// Builder for creating and configuring a ReliableTimer
    final class Builder {
        private var interval: TimeInterval?
        private var leeway: DispatchTimeInterval?
        private var queue: DispatchQueue?
        private var qos: DispatchQoS = .userInitiated
        private var handler: (() -> Void)?
        private var isOneTime = false

        /// Sets the interval for the timer
        func interval(_ interval: TimeInterval) -> Builder {
            self.interval = interval
            return self
        }

        /// Sets the leeway for the timer
        func leeway(_ leeway: DispatchTimeInterval) -> Builder {
            self.leeway = leeway
            return self
        }

        /// Sets the dispatch queue for the timer
        func queue(_ queue: DispatchQueue) -> Builder {
            self.queue = queue
            return self
        }

        /// Sets the quality of service for the timer
        func qos(_ qos: DispatchQoS) -> Builder {
            self.qos = qos
            return self
        }

        /// Sets whether this is a one-time timer
        func oneTime(_ isOneTime: Bool = true) -> Builder {
            self.isOneTime = isOneTime
            return self
        }

        /// Sets the handler for the timer
        func handler(_ handler: @escaping () -> Void) -> Builder {
            self.handler = handler
            return self
        }

        /// Builds and starts the timer
        func build() -> ReliableTimer {
            guard let interval = interval, let handler = handler else {
                fatalError("ReliableTimer.Builder: interval and handler must be set")
            }

            let timer = ReliableTimer(queue: queue, qos: qos)

            if isOneTime {
                timer.scheduleOnce(after: interval, handler: handler)
            } else {
                timer.schedule(interval: interval, leeway: leeway, handler: handler)
            }

            return timer
        }
    }

    /// Creates a new builder for configuring a ReliableTimer
    static func builder() -> Builder {
        return Builder()
    }
}
