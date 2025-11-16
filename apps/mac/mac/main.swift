import Cocoa
import ArgumentParser

/// Main entry point - routes to CLI or GUI based on arguments
/// This runs BEFORE NSApplication is created

print("🚀 main.swift starting...")
print("📋 Arguments: \(CommandLine.arguments)")

let args = CommandLine.arguments

// Filter out Xcode debug arguments (start with -NS or -XC) and their values
var filteredArgs: [String] = []
var skipNext = false
for arg in args {
    if skipNext {
        skipNext = false
        continue
    }
    if arg.hasPrefix("-NS") || arg.hasPrefix("-XC") {
        skipNext = true  // Skip the next argument (the value)
        continue
    }
    filteredArgs.append(arg)
}

print("🔍 Filtered arguments: \(filteredArgs)")

// Check for CLI commands (anything except --daemon or no args)
// Only enter CLI mode if we have actual user arguments (not just Xcode debug flags)
if filteredArgs.count > 1 && !args.contains("--daemon") {
    // CLI mode - run command and exit without launching NSApplication
    print("🔧 Running in CLI mode")
    let cliArgs = Array(filteredArgs.dropFirst())
    AriataMac.main(cliArgs)
    exit(0)
}

// GUI/daemon mode - launch NSApplication with AppDelegate
print("🖥️  Launching GUI mode...")
let app = NSApplication.shared
print("✅ NSApplication created")
let delegate = AppDelegate()
print("✅ AppDelegate created")
app.delegate = delegate
print("✅ Delegate assigned, calling app.run()...")
app.run()
