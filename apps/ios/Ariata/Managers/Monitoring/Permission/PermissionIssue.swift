//
//  PermissionIssue.swift
//  Ariata
//
//

import Foundation

/// Represents a permission issue that needs user attention
struct PermissionIssue: Identifiable, Equatable {
    let id = UUID()
    let type: PermissionType
    let message: String
    let action: String

    enum PermissionType: String {
        case location = "Location"
        case microphone = "Microphone"
        case healthKit = "HealthKit"
    }
}
