//
//  EndpointEditView.swift
//  Ariata
//
//  View for editing the upload endpoint URL
//

import SwiftUI

struct EndpointEditView: View {
    @ObservedObject private var deviceManager = DeviceManager.shared
    @ObservedObject private var uploadCoordinator = BatchUploadCoordinator.shared
    
    @Environment(\.dismiss) var dismiss
    
    @State private var newEndpoint: String = ""
    @State private var isValidating = false
    @State private var isSaving = false
    @State private var errorMessage: String?
    @State private var showingUploadWarning = false
    @State private var validationPassed = false
    
    private var currentEndpoint: String {
        deviceManager.configuration.apiEndpoint
    }
    
    private var hasChanges: Bool {
        newEndpoint.trimmingCharacters(in: .whitespacesAndNewlines) != currentEndpoint
    }
    
    private var isValidURL: Bool {
        deviceManager.validateEndpoint(newEndpoint)
    }
    
    private var hasPendingUploads: Bool {
        uploadCoordinator.uploadStats.pending > 0
    }
    
    var body: some View {
        NavigationView {
            Form {
                Section(header: Text("Current Endpoint")) {
                    HStack {
                        Image(systemName: "network")
                            .foregroundColor(.secondary)
                        Text(currentEndpoint)
                            .font(.system(.body, design: .monospaced))
                            .foregroundColor(.secondary)
                            .lineLimit(2)
                            .fixedSize(horizontal: false, vertical: true)
                    }
                }
                
                Section(header: Text("New Endpoint")) {
                    VStack(alignment: .leading, spacing: 12) {
                        TextField("https://your-server.com", text: $newEndpoint)
                            .font(.system(size: 17))
                            .padding()
                            .frame(minHeight: 52)
                            .background(Color(.systemGray6))
                            .cornerRadius(10)
                            .overlay(
                                RoundedRectangle(cornerRadius: 10)
                                    .stroke(Color(.systemGray4), lineWidth: 0.5)
                            )
                            .autocapitalization(.none)
                            .disableAutocorrection(true)
                            .keyboardType(.URL)
                            .onChange(of: newEndpoint) { _, _ in
                                errorMessage = nil
                                validationPassed = false
                            }
                        
                        if !newEndpoint.isEmpty && !isValidURL {
                            Label("Invalid URL format", systemImage: "exclamationmark.triangle")
                                .font(.caption)
                                .foregroundColor(.orange)
                        }
                        
                        if validationPassed {
                            Label("Connection successful", systemImage: "checkmark.circle")
                                .font(.caption)
                                .foregroundColor(.green)
                        }
                    }
                }
                
                if let error = errorMessage {
                    Section {
                        HStack {
                            Image(systemName: "exclamationmark.triangle")
                                .foregroundColor(.red)
                            Text(error)
                                .font(.caption)
                                .foregroundColor(.red)
                        }
                    }
                }
                
                Section {
                    Button(action: validateConnection) {
                        HStack {
                            if isValidating {
                                ProgressView()
                                    .progressViewStyle(CircularProgressViewStyle())
                                    .scaleEffect(0.8)
                                Text("Testing Connection...")
                            } else {
                                Image(systemName: "network")
                                Text("Test Connection")
                            }
                        }
                    }
                    .disabled(!isValidURL || newEndpoint.isEmpty || isValidating)
                    
                    if hasPendingUploads {
                        HStack {
                            Image(systemName: "info.circle")
                                .foregroundColor(.blue)
                            VStack(alignment: .leading, spacing: 4) {
                                Text("Pending Uploads")
                                    .font(.footnote)
                                    .fontWeight(.medium)
                                Text("\(uploadCoordinator.uploadStats.pending) events will be uploaded to the current endpoint before switching")
                                    .font(.caption)
                                    .foregroundColor(.secondary)
                            }
                        }
                        .padding(.vertical, 4)
                    }
                }
                
                Section(footer: Text("The endpoint URL must be reachable and running a Ariata server")) {
                    Button(action: { showingUploadWarning = true }) {
                        HStack {
                            if isSaving {
                                ProgressView()
                                    .progressViewStyle(CircularProgressViewStyle())
                                    .scaleEffect(0.8)
                                Text("Saving...")
                            } else {
                                Text("Save Endpoint")
                            }
                            Spacer()
                            Image(systemName: "checkmark")
                        }
                    }
                    .disabled(!hasChanges || !validationPassed || isSaving)
                    .foregroundColor(hasChanges && validationPassed ? .accentColor : .gray)
                }
            }
            .navigationTitle("Edit Endpoint")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .navigationBarLeading) {
                    Button("Cancel") {
                        dismiss()
                    }
                }
            }
            .onAppear {
                newEndpoint = currentEndpoint
            }
            .alert("Change Endpoint?", isPresented: $showingUploadWarning) {
                Button("Cancel", role: .cancel) { }
                Button("Change", role: .destructive) {
                    saveEndpoint()
                }
            } message: {
                if hasPendingUploads {
                    Text("You have \(uploadCoordinator.uploadStats.pending) pending uploads. They will be sent to the current endpoint before the change takes effect.")
                } else {
                    Text("Are you sure you want to change the upload endpoint? Future data will be sent to the new server.")
                }
            }
        }
    }
    
    private func validateConnection() {
        Task {
            await MainActor.run {
                isValidating = true
                errorMessage = nil
                validationPassed = false
            }
            
            let trimmedEndpoint = newEndpoint.trimmingCharacters(in: .whitespacesAndNewlines)
            let isReachable = await NetworkManager.shared.testConnection(endpoint: trimmedEndpoint)
            
            await MainActor.run {
                isValidating = false
                
                if isReachable {
                    validationPassed = true
                    errorMessage = nil
                } else {
                    validationPassed = false
                    errorMessage = "Cannot reach the server. Please check the URL and your internet connection."
                }
            }
        }
    }
    
    private func saveEndpoint() {
        Task {
            await MainActor.run {
                isSaving = true
                errorMessage = nil
            }
            
            let success = await deviceManager.updateEndpoint(newEndpoint.trimmingCharacters(in: .whitespacesAndNewlines))
            
            await MainActor.run {
                isSaving = false
                
                if success {
                    dismiss()
                } else {
                    errorMessage = "Failed to update endpoint. Please try again."
                }
            }
        }
    }
}

struct EndpointEditView_Previews: PreviewProvider {
    static var previews: some View {
        EndpointEditView()
    }
}