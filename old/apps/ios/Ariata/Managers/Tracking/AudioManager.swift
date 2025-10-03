//
//  AudioManager.swift
//  Ariata
//
//  Handles audio recording and microphone permissions
//

import Foundation
import AVFoundation
import AVFAudio
import UIKit

class AudioManager: NSObject, ObservableObject {
    static let shared = AudioManager()

    // MARK: - Constants
    private let chunkDurationSeconds = 30.0
    private let healthCheckIntervalSeconds = 30.0
    private let interruptionRecoveryDelay = 0.5

    // MARK: - Published Properties
    @Published var microphoneAuthorizationStatus: AVAudioApplication.recordPermission = .undetermined
    @Published var isRecording = false
    @Published var lastSaveDate: Date?
    @Published var availableAudioInputs: [AVAudioSessionPortDescription] = []
    @Published var selectedAudioInput: AVAudioSessionPortDescription?
    
    private let audioSession = AVAudioSession.sharedInstance()
    private var audioRecorder: AVAudioRecorder?
    private var recordingTimer: DispatchSourceTimer?
    private var currentChunkStartTime: Date?
    private var backgroundTask: UIBackgroundTaskIdentifier = .invalid
    private let timerQueue = DispatchQueue(label: "com.ariata.audio.timer", qos: .userInitiated)
    private var healthCheckTimer: DispatchSourceTimer?
    
    override init() {
        super.init()
        checkAuthorizationStatus()
        setupAudioInputMonitoring()
        updateAvailableInputs()
        loadSelectedInput()
        startHealthCheckTimer()
    }
    
    deinit {
        NotificationCenter.default.removeObserver(self)
        healthCheckTimer?.cancel()
        healthCheckTimer = nil
    }
    
    // MARK: - Authorization
    
    func requestAuthorization() async -> Bool {
        let granted = await AVAudioApplication.requestRecordPermission()
        DispatchQueue.main.async {
            self.checkAuthorizationStatus()
        }
        return granted
    }
    
    func checkAuthorizationStatus() {
        microphoneAuthorizationStatus = AVAudioApplication.shared.recordPermission
    }

    var hasPermission: Bool {
        return microphoneAuthorizationStatus == .granted
    }
    
    // MARK: - Audio Input Management
    
    private func setupAudioInputMonitoring() {
        // Listen for audio route changes
        NotificationCenter.default.addObserver(
            self,
            selector: #selector(handleRouteChange),
            name: AVAudioSession.routeChangeNotification,
            object: audioSession
        )
        
        // Listen for available inputs changes
        NotificationCenter.default.addObserver(
            self,
            selector: #selector(handleInputsChange),
            name: AVAudioSession.mediaServicesWereResetNotification,
            object: audioSession
        )
        
        // Listen for audio interruptions (phone calls, etc.)
        NotificationCenter.default.addObserver(
            self,
            selector: #selector(handleInterruption),
            name: AVAudioSession.interruptionNotification,
            object: audioSession
        )
    }
    
    @objc private func handleRouteChange(notification: Notification) {
        updateAvailableInputs()
    }
    
    @objc private func handleInputsChange(notification: Notification) {
        updateAvailableInputs()
    }
    
    @objc private func handleInterruption(notification: Notification) {
        guard let info = notification.userInfo,
              let typeValue = info[AVAudioSessionInterruptionTypeKey] as? UInt,
              let type = AVAudioSession.InterruptionType(rawValue: typeValue) else {
            return
        }

        switch type {
        case .began:
            // Health check will handle recovery automatically
            break

        case .ended:
            // Trigger immediate health check to recover quickly
            DispatchQueue.main.asyncAfter(deadline: .now() + interruptionRecoveryDelay) { [weak self] in
                self?.performHealthCheck()
            }

        @unknown default:
            break
        }
    }

    // MARK: - Health Check

    private func startHealthCheckTimer() {
        // Run health check on main queue to avoid thread safety issues
        let timer = DispatchSource.makeTimerSource(queue: .main)
        timer.schedule(deadline: .now() + healthCheckIntervalSeconds, repeating: healthCheckIntervalSeconds)
        timer.setEventHandler { [weak self] in
            self?.performHealthCheck()
        }
        timer.resume()
        healthCheckTimer = timer
    }

    func performHealthCheck() {
        let shouldBeRecording = hasPermission && DeviceManager.shared.configuration.isStreamEnabled("mic")
        let actuallyRecording = audioRecorder?.isRecording ?? false

        if shouldBeRecording && !actuallyRecording {
            stopRecording()  // Clean up any bad state
            startRecording() // Fresh start
        } else if !shouldBeRecording && actuallyRecording {
            stopRecording()
        }
    }

    
    private func updateAvailableInputs() {
        DispatchQueue.main.async { [weak self] in
            guard let self = self else { return }
            
            // Get all available inputs
            self.availableAudioInputs = self.audioSession.availableInputs ?? []
            
            // If selected input is no longer available, reset to default
            if let selectedInput = self.selectedAudioInput,
               !self.availableAudioInputs.contains(where: { $0.uid == selectedInput.uid }) {
                self.selectedAudioInput = nil
                self.saveSelectedInput()
            }
            
            // If no input selected, select the built-in mic
            if self.selectedAudioInput == nil {
                self.selectedAudioInput = self.availableAudioInputs.first(where: { 
                    $0.portType == .builtInMic
                })
            }
        }
    }
    
    func selectAudioInput(_ input: AVAudioSessionPortDescription?) {
        selectedAudioInput = input
        saveSelectedInput()
        
        // Apply the selection if currently recording
        if isRecording {
            do {
                try audioSession.setPreferredInput(input)
            } catch {
                print("❌ Failed to set preferred audio input: \(error)")
            }
        }
    }
    
    private func loadSelectedInput() {
        guard let savedInputUID = UserDefaults.standard.string(forKey: "selectedAudioInputUID") else {
            return
        }
        
        selectedAudioInput = availableAudioInputs.first(where: { $0.uid == savedInputUID })
    }
    
    private func saveSelectedInput() {
        UserDefaults.standard.set(selectedAudioInput?.uid, forKey: "selectedAudioInputUID")
    }
    
    func getDisplayName(for input: AVAudioSessionPortDescription) -> String {
        // Return user-friendly names for common port types
        switch input.portType {
        case .builtInMic:
            return "iPhone Microphone"
        case .bluetoothHFP, .bluetoothA2DP:
            return input.portName // Use the actual device name for Bluetooth
        case .headsetMic:
            return "Wired Headset"
        case .usbAudio:
            return "USB Microphone"
        case .carAudio:
            return "Car Audio"
        default:
            return input.portName
        }
    }
    
    // MARK: - Audio Session Setup
    
    func setupAudioSession() throws {
        // Configure audio session without .allowBluetooth if user selected built-in mic
        let shouldAllowBluetooth = selectedAudioInput?.portType != .builtInMic
        
        var options: AVAudioSession.CategoryOptions = [.defaultToSpeaker, .mixWithOthers]
        if shouldAllowBluetooth {
            options.insert(.allowBluetooth)
        }
        
        try audioSession.setCategory(.playAndRecord, mode: .default, options: options)
        
        // Set preferred input if one is selected
        if let selectedInput = selectedAudioInput {
            try audioSession.setPreferredInput(selectedInput)
        }
        
        try audioSession.setActive(true)
    }
    
    // MARK: - Recording Control
    
    func startRecording() {
        guard hasPermission else {
            print("❌ Microphone permission not granted")
            return
        }

        // Check if audio is enabled in configuration
        let isEnabled = DeviceManager.shared.configuration.isStreamEnabled("mic")
        guard isEnabled else {
            return
        }

        // Don't start if already recording
        guard !isRecording else {
            return
        }

        do {
            try setupAudioSession()
            startRecordingChunk()
            isRecording = true
        } catch {
            print("❌ Failed to start recording: \(error)")
            isRecording = false
        }
    }
    
    func stopRecording() {
        recordingTimer?.cancel()
        recordingTimer = nil

        // Save any partial chunk before stopping
        savePartialChunkIfNeeded()

        audioRecorder = nil
        currentChunkStartTime = nil
        isRecording = false
    }
}

// MARK: - Chunk Recording

extension AudioManager {
    private func savePartialChunkIfNeeded() {
        guard let recorder = audioRecorder,
              let startTime = currentChunkStartTime else {
            return
        }

        // Only stop if actually recording
        if recorder.isRecording {
            recorder.stop()
        }

        // Try to save the partial chunk
        if let audioData = try? Data(contentsOf: recorder.url) {
            let chunk = AudioChunk(
                startDate: startTime,
                endDate: Date(),
                audioData: audioData,
                overlapDuration: 0.0
            )
            saveAudioChunk(chunk)
            try? FileManager.default.removeItem(at: recorder.url)
        }
    }

    private func startRecordingChunk() {
        // Create audio file URL
        let documentsPath = FileManager.default.urls(for: .documentDirectory, in: .userDomainMask)[0]
        let audioFilename = documentsPath.appendingPathComponent("chunk_\(Date().timeIntervalSince1970).m4a")

        // Configure recording settings for 16kHz sample rate with optimized compression
        let settings: [String: Any] = [
            AVFormatIDKey: Int(kAudioFormatMPEG4AAC),
            AVSampleRateKey: 16000.0,
            AVNumberOfChannelsKey: 1,
            AVEncoderAudioQualityKey: AVAudioQuality.low.rawValue, // Low quality is fine for speech
            AVEncoderBitRateKey: 16000 // 16kbps - optimal for speech transcription
        ]

        do {
            audioRecorder = try AVAudioRecorder(url: audioFilename, settings: settings)
            audioRecorder?.delegate = self
            audioRecorder?.record()

            currentChunkStartTime = Date()

            // Use DispatchSourceTimer for more reliable background execution
            let timer = DispatchSource.makeTimerSource(queue: timerQueue)
            timer.schedule(deadline: .now() + chunkDurationSeconds)
            timer.setEventHandler { [weak self] in
                self?.finishCurrentChunk()
            }
            timer.resume()
            recordingTimer = timer
        } catch {
            print("❌ Failed to start recording chunk: \(error)")
        }
    }
    
    private func finishCurrentChunk() {
        guard let recorder = audioRecorder,
              let startTime = currentChunkStartTime else {
            print("⚠️ No active recorder or start time when finishing chunk")
            return
        }

        // Check recording state
        let wasRecording = recorder.isRecording
        recorder.stop()
        let endTime = Date()

        print("📊 Finishing chunk: wasRecording=\(wasRecording), duration=\(endTime.timeIntervalSince(startTime))s")

        // Process the recorded audio
        if let audioData = try? Data(contentsOf: recorder.url) {
            // Create chunk object
            let chunk = AudioChunk(
                startDate: startTime,
                endDate: endTime,
                audioData: audioData,
                overlapDuration: 0.0
            )

            // Save directly to SQLite
            saveAudioChunk(chunk)

            // Clean up temporary file
            try? FileManager.default.removeItem(at: recorder.url)
        }

        // Continue recording if still active
        if isRecording {
            startRecordingChunk()
        }
    }
    
    private func saveAudioChunk(_ chunk: AudioChunk) {
        // Begin background task
        beginBackgroundTask()

        // Ensure background task ends no matter what
        defer { endBackgroundTask() }

        // Create stream data with single chunk
        let streamData = AudioStreamData(
            deviceId: DeviceManager.shared.configuration.deviceId,
            chunks: [chunk]
        )

        // Encode and save to SQLite
        do {
            let encoder = JSONEncoder()
            encoder.dateEncodingStrategy = .iso8601
            let data = try encoder.encode(streamData)

            let success = SQLiteManager.shared.enqueue(streamName: "ios_mic", data: data)

            if success {
                Task { @MainActor in
                    self.lastSaveDate = Date()
                }

                // Update stats in upload coordinator
                BatchUploadCoordinator.shared.updateUploadStats()
            }
        } catch {
            print("❌ Failed to encode audio chunk: \(error)")
        }
    }
    
    private func beginBackgroundTask() {
        backgroundTask = UIApplication.shared.beginBackgroundTask { [weak self] in
            self?.endBackgroundTask()
        }
    }
    
    private func endBackgroundTask() {
        if backgroundTask != .invalid {
            UIApplication.shared.endBackgroundTask(backgroundTask)
            backgroundTask = .invalid
        }
    }
}

// MARK: - AVAudioRecorderDelegate

extension AudioManager: AVAudioRecorderDelegate {
    func audioRecorderDidFinishRecording(_ recorder: AVAudioRecorder, successfully flag: Bool) {
        if !flag {
            print("❌ Audio recording finished with error")
        }
    }
    
    func audioRecorderEncodeErrorDidOccur(_ recorder: AVAudioRecorder, error: Error?) {
        if let error = error {
            print("❌ Audio encoding error: \(error)")
        }
    }
}