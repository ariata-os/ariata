import Foundation
import AppKit

class Monitor {
    private let queue: Queue
    private var frontmostApp: NSRunningApplication?
    private var timer: Timer?
    private var messageMonitor: MessageMonitor?
    
    init(queue: Queue) {
        self.queue = queue
        self.messageMonitor = MessageMonitor(queue: queue)
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
        timer = Timer.scheduledTimer(withTimeInterval: 1.0, repeats: true) { [weak self] _ in
            self?.checkFrontmostApp()
        }
        
        // Start message monitoring
        messageMonitor?.start()
        
        print("Activity monitor started")
    }
    
    func stop() {
        timer?.invalidate()
        timer = nil
        NSWorkspace.shared.notificationCenter.removeObserver(self)
        messageMonitor?.stop()
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
        
        do {
            try queue.addEvent(event)
            print("[\(Date())] \(eventType): \(appName)")
        } catch {
            print("Error recording event: \(error)")
        }
    }
}