import ArgumentParser
import Foundation

struct ResetCommand: ParsableCommand {
    static let configuration = CommandConfiguration(
        commandName: "reset",
        abstract: "Reset configuration and clear all data"
    )
    
    @Flag(name: .shortAndLong, help: "Skip confirmation prompt")
    var force = false
    
    func run() throws {
        if !force {
            print("This will delete all configuration and queued data.")
            print("Are you sure? (y/N): ", terminator: "")
            
            let response = readLine()?.lowercased()
            if response != "y" && response != "yes" {
                print("Cancelled")
                return
            }
        }
        
        print("Resetting...")
        
        // Delete config
        do {
            try Config.delete()
            print("✓ Configuration deleted")
        } catch {
            print("⚠️  Could not delete config: \(error)")
        }
        
        // Clear queue
        do {
            let queue = try Queue()
            try queue.reset()
            print("✓ Event queue cleared")
        } catch {
            print("⚠️  Could not clear queue: \(error)")
        }
        
        // Remove database file
        let dbPath = Config.configDir.appendingPathComponent("activity.db")
        if FileManager.default.fileExists(atPath: dbPath.path) {
            do {
                try FileManager.default.removeItem(at: dbPath)
                print("✓ Database deleted")
            } catch {
                print("⚠️  Could not delete database: \(error)")
            }
        }
        
        print("\nReset complete. Run 'ariata-mac init <token>' to reconfigure.")
    }
}