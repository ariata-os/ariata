import ArgumentParser
import Foundation

@main
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