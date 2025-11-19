import Cocoa

/// Window controller for device pairing dialog
class PairingWindowController: NSWindowController {

    convenience init() {
        let window = NSWindow(
            contentRect: NSRect(x: 0, y: 0, width: 480, height: 340),
            styleMask: [.titled, .closable],
            backing: .buffered,
            defer: false
        )
        window.title = "Pair This Mac with Ariata"
        window.center()
        window.isMovableByWindowBackground = true

        self.init(window: window)

        let viewController = PairingViewController()
        window.contentViewController = viewController
    }
}

/// View controller for pairing form
class PairingViewController: NSViewController {

    // UI Components
    private var titleLabel: NSTextField!
    private var subtitleLabel: NSTextField!
    private var endpointField: NSTextField!
    private var codeField: NSTextField!
    private var connectButton: NSButton!
    private var statusLabel: NSTextField!
    private var progressIndicator: NSProgressIndicator!

    private var isConnecting = false

    var onPairingComplete: ((Config) -> Void)?

    override func loadView() {
        view = NSView(frame: NSRect(x: 0, y: 0, width: 480, height: 340))
    }

    override func viewDidLoad() {
        super.viewDidLoad()
        setupUI()
    }

    private func setupUI() {
        let containerView = view

        // Title
        titleLabel = NSTextField(labelWithString: "Connect to Ariata Server")
        titleLabel.font = NSFont.systemFont(ofSize: 20, weight: .bold)
        titleLabel.alignment = .center
        titleLabel.translatesAutoresizingMaskIntoConstraints = false
        containerView.addSubview(titleLabel)

        // Subtitle
        subtitleLabel = NSTextField(labelWithString: "Enter your server endpoint and the 6-digit pairing code from the web app.")
        subtitleLabel.font = NSFont.systemFont(ofSize: 13)
        subtitleLabel.textColor = .secondaryLabelColor
        subtitleLabel.alignment = .center
        subtitleLabel.lineBreakMode = .byWordWrapping
        subtitleLabel.maximumNumberOfLines = 2
        subtitleLabel.translatesAutoresizingMaskIntoConstraints = false
        containerView.addSubview(subtitleLabel)

        // Endpoint label
        let endpointLabel = NSTextField(labelWithString: "API Endpoint:")
        endpointLabel.font = NSFont.systemFont(ofSize: 13, weight: .medium)
        endpointLabel.translatesAutoresizingMaskIntoConstraints = false
        containerView.addSubview(endpointLabel)

        // Endpoint field
        endpointField = NSTextField()
        endpointField.placeholderString = "https://your-server.com"
        endpointField.font = NSFont.monospacedSystemFont(ofSize: 13, weight: .regular)
        endpointField.translatesAutoresizingMaskIntoConstraints = false
        endpointField.stringValue = ProcessInfo.processInfo.environment["ARIATA_API_URL"] ?? "http://localhost:3000"
        containerView.addSubview(endpointField)

        // Code label
        let codeLabel = NSTextField(labelWithString: "Pairing Code:")
        codeLabel.font = NSFont.systemFont(ofSize: 13, weight: .medium)
        codeLabel.translatesAutoresizingMaskIntoConstraints = false
        containerView.addSubview(codeLabel)

        // Code field
        codeField = NSTextField()
        codeField.placeholderString = "A7K2P9"
        codeField.font = NSFont.monospacedSystemFont(ofSize: 18, weight: .medium)
        codeField.translatesAutoresizingMaskIntoConstraints = false
        codeField.delegate = self
        containerView.addSubview(codeField)

        // Progress indicator
        progressIndicator = NSProgressIndicator()
        progressIndicator.style = .spinning
        progressIndicator.isHidden = true
        progressIndicator.translatesAutoresizingMaskIntoConstraints = false
        containerView.addSubview(progressIndicator)

        // Status label
        statusLabel = NSTextField(labelWithString: "")
        statusLabel.font = NSFont.systemFont(ofSize: 12)
        statusLabel.textColor = .systemRed
        statusLabel.alignment = .center
        statusLabel.lineBreakMode = .byWordWrapping
        statusLabel.maximumNumberOfLines = 3
        statusLabel.isHidden = true
        statusLabel.translatesAutoresizingMaskIntoConstraints = false
        containerView.addSubview(statusLabel)

        // Connect button
        connectButton = NSButton(title: "Connect", target: self, action: #selector(connectTapped))
        connectButton.keyEquivalent = "\r"  // Enter key
        connectButton.bezelStyle = .rounded
        connectButton.translatesAutoresizingMaskIntoConstraints = false
        containerView.addSubview(connectButton)

        // Layout constraints
        NSLayoutConstraint.activate([
            // Title
            titleLabel.topAnchor.constraint(equalTo: containerView.topAnchor, constant: 30),
            titleLabel.leadingAnchor.constraint(equalTo: containerView.leadingAnchor, constant: 40),
            titleLabel.trailingAnchor.constraint(equalTo: containerView.trailingAnchor, constant: -40),

            // Subtitle
            subtitleLabel.topAnchor.constraint(equalTo: titleLabel.bottomAnchor, constant: 8),
            subtitleLabel.leadingAnchor.constraint(equalTo: containerView.leadingAnchor, constant: 40),
            subtitleLabel.trailingAnchor.constraint(equalTo: containerView.trailingAnchor, constant: -40),

            // Endpoint label
            endpointLabel.topAnchor.constraint(equalTo: subtitleLabel.bottomAnchor, constant: 30),
            endpointLabel.leadingAnchor.constraint(equalTo: containerView.leadingAnchor, constant: 40),

            // Endpoint field
            endpointField.topAnchor.constraint(equalTo: endpointLabel.bottomAnchor, constant: 6),
            endpointField.leadingAnchor.constraint(equalTo: containerView.leadingAnchor, constant: 40),
            endpointField.trailingAnchor.constraint(equalTo: containerView.trailingAnchor, constant: -40),
            endpointField.heightAnchor.constraint(equalToConstant: 28),

            // Code label
            codeLabel.topAnchor.constraint(equalTo: endpointField.bottomAnchor, constant: 20),
            codeLabel.leadingAnchor.constraint(equalTo: containerView.leadingAnchor, constant: 40),

            // Code field
            codeField.topAnchor.constraint(equalTo: codeLabel.bottomAnchor, constant: 6),
            codeField.leadingAnchor.constraint(equalTo: containerView.leadingAnchor, constant: 40),
            codeField.trailingAnchor.constraint(equalTo: containerView.trailingAnchor, constant: -40),
            codeField.heightAnchor.constraint(equalToConstant: 32),

            // Status label
            statusLabel.topAnchor.constraint(equalTo: codeField.bottomAnchor, constant: 12),
            statusLabel.leadingAnchor.constraint(equalTo: containerView.leadingAnchor, constant: 40),
            statusLabel.trailingAnchor.constraint(equalTo: containerView.trailingAnchor, constant: -40),

            // Progress indicator
            progressIndicator.centerXAnchor.constraint(equalTo: containerView.centerXAnchor),
            progressIndicator.bottomAnchor.constraint(equalTo: connectButton.topAnchor, constant: -16),

            // Connect button
            connectButton.bottomAnchor.constraint(equalTo: containerView.bottomAnchor, constant: -24),
            connectButton.centerXAnchor.constraint(equalTo: containerView.centerXAnchor),
            connectButton.widthAnchor.constraint(equalToConstant: 120),
        ])
    }

    @objc private func connectTapped() {
        let endpoint = endpointField.stringValue.trimmingCharacters(in: .whitespacesAndNewlines)
        let code = codeField.stringValue.trimmingCharacters(in: .whitespacesAndNewlines).uppercased()

        // Validation
        guard !endpoint.isEmpty else {
            showError("Please enter an API endpoint")
            return
        }

        guard code.count == 6 else {
            showError("Pairing code must be 6 characters")
            return
        }

        // Check if code is alphanumeric
        let allowedChars = CharacterSet(charactersIn: "ABCDEFGHJKLMNPQRSTUVWXYZ23456789")
        guard code.unicodeScalars.allSatisfy({ allowedChars.contains($0) }) else {
            showError("Invalid pairing code format")
            return
        }

        // Start pairing
        startPairing(endpoint: endpoint, code: code)
    }

    private func startPairing(endpoint: String, code: String) {
        isConnecting = true
        updateUI()

        Task {
            do {
                let config = try await completePairing(endpoint: endpoint, code: code)

                await MainActor.run {
                    // Success!
                    self.isConnecting = false
                    self.showSuccessDialog(config: config)
                }
            } catch {
                await MainActor.run {
                    self.isConnecting = false
                    self.showError(error.localizedDescription)
                    self.updateUI()
                }
            }
        }
    }

    private func completePairing(endpoint: String, code: String) async throws -> Config {
        // Get device info
        let deviceId = DeviceIdentifier.getMachineUUID()
        let deviceName = Host.current().localizedName ?? "Mac"

        // Build request
        struct CompletePairingRequest: Codable {
            let code: String
            let device_info: DeviceInfo

            struct DeviceInfo: Codable {
                let device_id: String
                let device_name: String
                let device_model: String
                let os_version: String
                let app_version: String?
            }
        }

        struct CompletePairingResponse: Codable {
            let device_token: String
            let source_id: String
        }

        let request = CompletePairingRequest(
            code: code,
            device_info: CompletePairingRequest.DeviceInfo(
                device_id: deviceId,
                device_name: deviceName,
                device_model: "Mac",
                os_version: ProcessInfo.processInfo.operatingSystemVersionString,
                app_version: Version.full
            )
        )

        // Make API call
        guard let url = URL(string: "\(endpoint)/api/devices/pairing/complete") else {
            throw NSError(domain: "PairingError", code: 1, userInfo: [NSLocalizedDescriptionKey: "Invalid endpoint URL"])
        }

        var urlRequest = URLRequest(url: url)
        urlRequest.httpMethod = "POST"
        urlRequest.setValue("application/json", forHTTPHeaderField: "Content-Type")
        urlRequest.httpBody = try JSONEncoder().encode(request)

        let (data, response) = try await URLSession.shared.data(for: urlRequest)

        guard let httpResponse = response as? HTTPURLResponse else {
            throw NSError(domain: "PairingError", code: 2, userInfo: [NSLocalizedDescriptionKey: "Invalid response from server"])
        }

        guard httpResponse.statusCode == 200 else {
            // Try to parse error message from response
            if let errorJson = try? JSONSerialization.jsonObject(with: data) as? [String: Any],
               let errorMsg = errorJson["error"] as? String {
                throw NSError(domain: "PairingError", code: httpResponse.statusCode, userInfo: [NSLocalizedDescriptionKey: errorMsg])
            }
            throw NSError(domain: "PairingError", code: httpResponse.statusCode, userInfo: [NSLocalizedDescriptionKey: "Pairing failed (HTTP \(httpResponse.statusCode))"])
        }

        // Parse response
        let pairingResponse = try JSONDecoder().decode(CompletePairingResponse.self, from: data)

        // Create config
        return Config(
            deviceToken: pairingResponse.device_token,
            deviceId: pairingResponse.source_id,
            apiEndpoint: endpoint,
            createdAt: Date()
        )
    }

    private func showSuccessDialog(config: Config) {
        let alert = NSAlert()
        alert.messageText = "Successfully Paired!"
        alert.informativeText = """
        Your Mac is now connected to Ariata.

        Next Steps:
        • Open the dashboard to verify your streams are working
        • Check that data is being synced from this device
        • Configure which streams you want to enable
        """
        alert.alertStyle = .informational
        alert.addButton(withTitle: "Open Dashboard")
        alert.addButton(withTitle: "Done")

        let response = alert.runModal()

        // Call completion handler and close window
        self.onPairingComplete?(config)

        if response == .alertFirstButtonReturn {
            // User clicked "Open Dashboard"
            let dashboardURL = ProcessInfo.processInfo.environment["ARIATA_DASHBOARD_URL"] ?? "http://localhost:5173"
            if let url = URL(string: dashboardURL) {
                NSWorkspace.shared.open(url)
            }
        }

        self.view.window?.close()
    }

    private func showError(_ message: String) {
        statusLabel.stringValue = message
        statusLabel.isHidden = false
    }

    private func updateUI() {
        endpointField.isEnabled = !isConnecting
        codeField.isEnabled = !isConnecting
        connectButton.isEnabled = !isConnecting
        connectButton.title = isConnecting ? "Connecting..." : "Connect"

        if isConnecting {
            progressIndicator.isHidden = false
            progressIndicator.startAnimation(nil)
            statusLabel.isHidden = true
        } else {
            progressIndicator.isHidden = true
            progressIndicator.stopAnimation(nil)
        }
    }
}

// MARK: - Text Field Delegate

extension PairingViewController: NSTextFieldDelegate {
    func controlTextDidChange(_ obj: Notification) {
        guard let textField = obj.object as? NSTextField else { return }

        if textField == codeField {
            // Auto-uppercase and limit to 6 characters
            let text = textField.stringValue.uppercased()
            let limited = String(text.prefix(6))
            textField.stringValue = limited

            // Hide error when user starts typing
            if !statusLabel.isHidden {
                statusLabel.isHidden = true
            }
        }
    }
}
