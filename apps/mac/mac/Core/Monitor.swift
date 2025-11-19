import Foundation
import AppKit

class Monitor {
    private let queue: Queue
    private var frontmostApp: NSRunningApplication?
    private var timer: DispatchSourceTimer?

    init(queue: Queue) {
        self.queue = queue
    }
    
    func start() {
        print("Starting activity monitor...")
        
        // Track current frontmost app
        frontmostApp = NSWorkspace.shared.frontmostApplication
        if let app = frontmostApp {
            recordEvent(app: app, eventType: Event.EventType.focus)
        }
        
        // Set up workspace notifications
        let workspace = NSWorkspace.shared
        let center = workspace.notificationCenter
        
        center.addObserver(
            self,
            selector: #selector(appLaunched(_:)),
            name: NSWorkspace.didLaunchApplicationNotification,
            object: nil
        )
        
        center.addObserver(
            self,
            selector: #selector(appTerminated(_:)),
            name: NSWorkspace.didTerminateApplicationNotification,
            object: nil
        )
        
        center.addObserver(
            self,
            selector: #selector(appActivated(_:)),
            name: NSWorkspace.didActivateApplicationNotification,
            object: nil
        )
        
        // Poll for frontmost app changes (backup for notifications)
        // Use DispatchSourceTimer for reliability in menu bar apps
        let pollTimer = DispatchSource.makeTimerSource(queue: .main)
        pollTimer.schedule(deadline: .now() + 1.0, repeating: 1.0)
        pollTimer.setEventHandler { [weak self] in
            self?.checkFrontmostApp()
        }
        pollTimer.resume()
        self.timer = pollTimer

        print("Activity monitor started")
    }
    
    func stop() {
        timer?.cancel()
        timer = nil
        NSWorkspace.shared.notificationCenter.removeObserver(self)
        print("Activity monitor stopped")
    }
    
    @objc private func appLaunched(_ notification: Notification) {
        guard let app = notification.userInfo?[NSWorkspace.applicationUserInfoKey] as? NSRunningApplication else { 
            return 
        }
        recordEvent(app: app, eventType: Event.EventType.launch)
    }
    
    @objc private func appTerminated(_ notification: Notification) {
        guard let app = notification.userInfo?[NSWorkspace.applicationUserInfoKey] as? NSRunningApplication else { 
            return 
        }
        recordEvent(app: app, eventType: Event.EventType.quit)
    }
    
    @objc private func appActivated(_ notification: Notification) {
        guard let app = notification.userInfo?[NSWorkspace.applicationUserInfoKey] as? NSRunningApplication else { 
            return 
        }
        
        // Record unfocus for previous app
        if let previousApp = frontmostApp, previousApp != app {
            recordEvent(app: previousApp, eventType: Event.EventType.unfocus)
        }
        
        // Record focus for new app
        recordEvent(app: app, eventType: Event.EventType.focus)
        frontmostApp = app
    }
    
    private func checkFrontmostApp() {
        let currentFrontmost = NSWorkspace.shared.frontmostApplication
        
        if currentFrontmost != frontmostApp {
            // App focus changed
            if let previous = frontmostApp {
                recordEvent(app: previous, eventType: Event.EventType.unfocus)
            }
            
            if let current = currentFrontmost {
                recordEvent(app: current, eventType: Event.EventType.focus)
            }
            
            frontmostApp = currentFrontmost
        }
    }
    
    private func recordEvent(app: NSRunningApplication, eventType: String) {
        let appName = app.localizedName ?? "Unknown"
        let bundleId = app.bundleIdentifier
        
        // Skip our own app
        if bundleId == "com.ariata.mac" || appName == "ariata-mac" {
            return
        }
        
        let event = Event(eventType: eventType, appName: appName, bundleId: bundleId)

        // Add event asynchronously (non-blocking)
        queue.addEvent(event) { result in
            switch result {
            case .success:
                print("✓ [\(Date())] \(eventType): \(appName)")
            case .failure(let error):
                print("⚠️ Error recording event: \(error)")
            }
        }
    }
}