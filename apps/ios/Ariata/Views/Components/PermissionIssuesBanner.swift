//
//  PermissionIssuesBanner.swift
//  Ariata
//
//

import SwiftUI

struct PermissionIssuesBanner: View {
    let issue: PermissionIssue

    var body: some View {
        HStack(spacing: 12) {
            Image(systemName: "exclamationmark.triangle.fill")
                .font(.title2)
                .foregroundColor(.red)

            VStack(alignment: .leading, spacing: 4) {
                Text("\(issue.type.rawValue) Permission Issue")
                    .font(.headline)
                    .foregroundColor(.primary)

                Text(issue.message)
                    .font(.caption)
                    .foregroundColor(.secondary)
                    .fixedSize(horizontal: false, vertical: true)
            }

            Spacer()

            Button(action: {
                PermissionMonitor.shared.openSettings()
            }) {
                Text(issue.action)
                    .font(.caption)
                    .foregroundColor(.white)
                    .padding(.horizontal, 12)
                    .padding(.vertical, 6)
                    .background(Color.red)
                    .cornerRadius(8)
            }
        }
        .padding()
        .background(Color.red.opacity(0.15))
        .cornerRadius(12)
        .overlay(
            RoundedRectangle(cornerRadius: 12)
                .stroke(Color.red.opacity(0.3), lineWidth: 1)
        )
    }
}
