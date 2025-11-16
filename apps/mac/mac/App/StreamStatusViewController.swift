import Cocoa

class StreamStatusViewController: NSViewController {

    private var daemonController: DaemonController
    private var refreshTimer: Timer?

    // UI components
    private var stackView: NSStackView!
    private var queueLabel: NSTextField!
    private var uploadLabel: NSTextField!
    private var loadingLabel: NSTextField?
    private var errorLabel: NSTextField?

    // Cached data for offline mode
    private var cachedStreams: [APIClient.StreamInfo] = []
    private var isOffline: Bool = false

    init(daemonController: DaemonController) {
        self.daemonController = daemonController
        super.init(nibName: nil, bundle: nil)
    }

    required init?(coder: NSCoder) {
        fatalError("init(coder:) has not been implemented")
    }

    override func loadView() {
        let containerView = NSView(frame: NSRect(x: 0, y: 0, width: 320, height: 280))
        self.view = containerView

        // Create main stack view
        stackView = NSStackView()
        stackView.orientation = .vertical
        stackView.spacing = 8
        stackView.edgeInsets = NSEdgeInsets(top: 16, left: 16, bottom: 16, right: 16)
        stackView.translatesAutoresizingMaskIntoConstraints = false

        // Title
        let titleLabel = NSTextField(labelWithString: "STREAM STATUS")
        titleLabel.font = NSFont.boldSystemFont(ofSize: 11)
        titleLabel.textColor = .secondaryLabelColor
        titleLabel.alignment = .center

        stackView.addArrangedSubview(titleLabel)
        stackView.addArrangedSubview(createSeparator())

        // Show loading state initially
        let loadingLabel = NSTextField(labelWithString: "Loading streams...")
        loadingLabel.font = NSFont.systemFont(ofSize: 11)
        loadingLabel.textColor = .secondaryLabelColor
        loadingLabel.alignment = .center
        self.loadingLabel = loadingLabel
        stackView.addArrangedSubview(loadingLabel)

        // Add stack view to container
        containerView.addSubview(stackView)

        // Layout constraints
        NSLayoutConstraint.activate([
            stackView.topAnchor.constraint(equalTo: containerView.topAnchor),
            stackView.leadingAnchor.constraint(equalTo: containerView.leadingAnchor),
            stackView.trailingAnchor.constraint(equalTo: containerView.trailingAnchor),
            stackView.bottomAnchor.constraint(equalTo: containerView.bottomAnchor)
        ])

        // Fetch streams asynchronously
        Task {
            await loadStreams()
        }
    }

    private func createStreamRow(name: String, status: StreamStatus, lastSync: String) -> NSView {
        let row = NSView()
        row.translatesAutoresizingMaskIntoConstraints = false

        let nameLabel = NSTextField(labelWithString: name)
        nameLabel.font = NSFont.systemFont(ofSize: 12)
        nameLabel.isBordered = false
        nameLabel.isEditable = false
        nameLabel.backgroundColor = .clear
        nameLabel.translatesAutoresizingMaskIntoConstraints = false

        let statusIcon = NSTextField(labelWithString: status.icon)
        statusIcon.font = NSFont.systemFont(ofSize: 14)
        statusIcon.isBordered = false
        statusIcon.isEditable = false
        statusIcon.backgroundColor = .clear
        statusIcon.translatesAutoresizingMaskIntoConstraints = false

        let timeLabel = NSTextField(labelWithString: lastSync)
        timeLabel.font = NSFont.systemFont(ofSize: 10)
        timeLabel.textColor = .secondaryLabelColor
        timeLabel.isBordered = false
        timeLabel.isEditable = false
        timeLabel.backgroundColor = .clear
        timeLabel.alignment = .right
        timeLabel.translatesAutoresizingMaskIntoConstraints = false

        row.addSubview(statusIcon)
        row.addSubview(nameLabel)
        row.addSubview(timeLabel)

        NSLayoutConstraint.activate([
            row.heightAnchor.constraint(equalToConstant: 20),

            statusIcon.leadingAnchor.constraint(equalTo: row.leadingAnchor),
            statusIcon.centerYAnchor.constraint(equalTo: row.centerYAnchor),

            nameLabel.leadingAnchor.constraint(equalTo: statusIcon.trailingAnchor, constant: 6),
            nameLabel.centerYAnchor.constraint(equalTo: row.centerYAnchor),

            timeLabel.trailingAnchor.constraint(equalTo: row.trailingAnchor),
            timeLabel.centerYAnchor.constraint(equalTo: row.centerYAnchor),
            timeLabel.leadingAnchor.constraint(greaterThanOrEqualTo: nameLabel.trailingAnchor, constant: 8)
        ])

        return row
    }

    private func createSeparator() -> NSBox {
        let separator = NSBox()
        separator.boxType = .separator
        separator.translatesAutoresizingMaskIntoConstraints = false
        separator.heightAnchor.constraint(equalToConstant: 1).isActive = true
        return separator
    }

    private func formatRelativeTime(_ date: Date?) -> String {
        guard let date = date else {
            return "never"
        }

        let interval = Date().timeIntervalSince(date)

        if interval < 60 {
            return "just now"
        } else if interval < 3600 {
            let mins = Int(interval / 60)
            return "\(mins)m ago"
        } else if interval < 86400 {
            let hours = Int(interval / 3600)
            return "\(hours)h ago"
        } else {
            let days = Int(interval / 86400)
            return "\(days)d ago"
        }
    }

    @objc private func openLogs() {
        // Open Console.app
        NSWorkspace.shared.launchApplication("Console")
    }

    override func viewDidAppear() {
        super.viewDidAppear()

        // Refresh every 30 seconds while popover is open
        refreshTimer = Timer.scheduledTimer(withTimeInterval: 30.0, repeats: true) { [weak self] _ in
            self?.refreshStatus()
        }
    }

    override func viewDidDisappear() {
        super.viewDidDisappear()

        // Clean up timer
        refreshTimer?.invalidate()
        refreshTimer = nil
    }

    private func loadStreams() async {
        do {
            // Fetch streams from backend
            let streams = try await APIClient.shared.fetchStreams()

            // Cache successful result
            cachedStreams = streams
            isOffline = false

            // Update UI on main thread
            await MainActor.run {
                rebuildUI(with: streams)
            }
        } catch {
            print("⚠️ Failed to fetch streams: \(error)")

            // If we have cached data, use it with offline indicator
            if !cachedStreams.isEmpty {
                isOffline = true
                await MainActor.run {
                    rebuildUI(with: cachedStreams, showOffline: true)
                }
            } else {
                // No cached data, show error
                await MainActor.run {
                    showError(error)
                }
            }
        }
    }

    private func rebuildUI(with streams: [APIClient.StreamInfo], showOffline: Bool = false) {
        // Remove loading/error labels if present
        if let loadingLabel = loadingLabel {
            stackView.removeArrangedSubview(loadingLabel)
            loadingLabel.removeFromSuperview()
            self.loadingLabel = nil
        }
        if let errorLabel = errorLabel {
            stackView.removeArrangedSubview(errorLabel)
            errorLabel.removeFromSuperview()
            self.errorLabel = nil
        }

        // Remove all stream rows (everything between separator and queue section)
        // We'll rebuild from scratch
        let viewsToRemove = stackView.arrangedSubviews.dropFirst(2) // Skip title and first separator
        for view in viewsToRemove {
            stackView.removeArrangedSubview(view)
            view.removeFromSuperview()
        }

        // Show offline indicator if needed
        if showOffline {
            let offlineLabel = NSTextField(labelWithString: "⚠️ Offline - Showing Cached Data")
            offlineLabel.font = NSFont.systemFont(ofSize: 10)
            offlineLabel.textColor = .systemOrange
            offlineLabel.alignment = .center
            stackView.addArrangedSubview(offlineLabel)
            stackView.addArrangedSubview(createSeparator())
        }

        // Group streams by source type
        let groupedStreams = Dictionary(grouping: streams) { stream -> String in
            // Infer source type from stream name patterns
            // This is a simple heuristic - ideally we'd get this from the backend
            if stream.name == "apps" || stream.name == "browser" || stream.name == "imessage" {
                return "Mac"
            } else if stream.name == "healthkit" || stream.name == "location" || stream.name == "microphone" {
                return "iOS"
            } else if stream.name == "gmail" || stream.name == "calendar" {
                return "Google"
            } else {
                return "Other"
            }
        }

        // Display streams grouped by source
        for (sourceType, sourceStreams) in groupedStreams.sorted(by: { $0.key < $1.key }) {
            let header = NSTextField(labelWithString: "\(sourceType):")
            header.font = NSFont.boldSystemFont(ofSize: 11)
            header.textColor = .secondaryLabelColor
            stackView.addArrangedSubview(header)

            for stream in sourceStreams.sorted(by: { $0.name < $1.name }) {
                let status = determineStatus(for: stream)
                let lastSync = formatRelativeTime(stream.lastSyncAt)

                let row = createStreamRow(
                    name: formatStreamName(stream.name),
                    status: status,
                    lastSync: lastSync
                )
                stackView.addArrangedSubview(row)
            }
        }

        stackView.addArrangedSubview(createSeparator())

        // Get daemon stats
        let stats = daemonController.getStats()

        // Queue status section
        queueLabel = NSTextField(labelWithString: "Queue: \(stats.queuedRecords) records pending")
        queueLabel.font = NSFont.systemFont(ofSize: 11)
        queueLabel.textColor = .secondaryLabelColor
        stackView.addArrangedSubview(queueLabel)

        uploadLabel = NSTextField(labelWithString: "Last upload: \(formatRelativeTime(stats.lastSyncTime))")
        uploadLabel.font = NSFont.systemFont(ofSize: 11)
        uploadLabel.textColor = .secondaryLabelColor
        stackView.addArrangedSubview(uploadLabel)

        stackView.addArrangedSubview(createSeparator())

        // View logs button
        let logsButton = NSButton(title: "View Full Logs →", target: self, action: #selector(openLogs))
        logsButton.bezelStyle = .rounded
        logsButton.font = NSFont.systemFont(ofSize: 12)
        stackView.addArrangedSubview(logsButton)

        print("✅ Stream status UI rebuilt with \(streams.count) streams")
    }

    private func showError(_ error: Error) {
        // Remove loading label
        if let loadingLabel = loadingLabel {
            stackView.removeArrangedSubview(loadingLabel)
            loadingLabel.removeFromSuperview()
            self.loadingLabel = nil
        }

        // Show error message
        let errorLabel = NSTextField(wrappingLabelWithString: "Failed to load streams.\n\(error.localizedDescription)")
        errorLabel.font = NSFont.systemFont(ofSize: 11)
        errorLabel.textColor = .systemRed
        errorLabel.alignment = .center
        self.errorLabel = errorLabel
        stackView.addArrangedSubview(errorLabel)

        // Add retry button
        let retryButton = NSButton(title: "Retry", target: self, action: #selector(retryLoad))
        retryButton.bezelStyle = .rounded
        retryButton.font = NSFont.systemFont(ofSize: 12)
        stackView.addArrangedSubview(retryButton)
    }

    @objc private func retryLoad() {
        // Remove error UI
        if let errorLabel = errorLabel {
            stackView.removeArrangedSubview(errorLabel)
            errorLabel.removeFromSuperview()
            self.errorLabel = nil
        }

        // Remove any retry button
        for view in stackView.arrangedSubviews {
            if let button = view as? NSButton, button.title == "Retry" {
                stackView.removeArrangedSubview(button)
                button.removeFromSuperview()
            }
        }

        // Show loading
        let loadingLabel = NSTextField(labelWithString: "Loading streams...")
        loadingLabel.font = NSFont.systemFont(ofSize: 11)
        loadingLabel.textColor = .secondaryLabelColor
        loadingLabel.alignment = .center
        self.loadingLabel = loadingLabel
        stackView.insertArrangedSubview(loadingLabel, at: 2)

        // Retry fetch
        Task {
            await loadStreams()
        }
    }

    private func determineStatus(for stream: APIClient.StreamInfo) -> StreamStatus {
        if !stream.isEnabled {
            return .paused
        }

        guard let lastSync = stream.lastSyncAt else {
            return .inactive
        }

        let interval = Date().timeIntervalSince(lastSync)

        // If last sync was more than 1 hour ago, consider it stale
        if interval > 3600 {
            return .stale
        }

        // Active if synced recently
        return .active
    }

    private func formatStreamName(_ name: String) -> String {
        // Convert snake_case or kebab-case to Title Case
        return name
            .split(separator: "_")
            .map { $0.capitalized }
            .joined(separator: " ")
    }

    private func refreshStatus() {
        // Reload streams from backend
        Task {
            await loadStreams()
        }
    }

    enum StreamStatus {
        case active
        case stale
        case error
        case paused
        case syncing
        case inactive

        var icon: String {
            switch self {
            case .active: return "✓"
            case .stale: return "⚠"
            case .error: return "❌"
            case .paused: return "⏸"
            case .syncing: return "⏳"
            case .inactive: return "⚪"
            }
        }

        var color: NSColor {
            switch self {
            case .active: return .systemGreen
            case .stale: return .systemYellow
            case .error: return .systemRed
            case .paused: return .systemGray
            case .syncing: return .systemBlue
            case .inactive: return .systemGray
            }
        }
    }
}
