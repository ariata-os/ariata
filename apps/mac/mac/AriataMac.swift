import ArgumentParser
import Foundation

/// CLI entry point for Ariata Mac
/// Note: No @main here - AppDelegate routes to this when CLI args detected
struct AriataMac: ParsableCommand {
    static let configuration = CommandConfiguration(
        commandName: "ariata-mac",
        abstract: "Ariata Mac activity monitor CLI",
        version: Version.full,
        subcommands: [
            InitCommand.self,
            StartCommand.self,
            DaemonCommand.self,
            StatusCommand.self,
            ResetCommand.self,
            StopCommand.self
        ],
        defaultSubcommand: StatusCommand.self
    )
}