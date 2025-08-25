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
    
    @Published var microphoneAuthorizationStatus: AVAudioSession.RecordPermission = .undetermined
    @Published var isRecording = false
    @Published var lastSaveDate: Date?
    @Published var availableAudioInputs: [AVAudioSessionPortDescription] = []
    @Published var selectedAudioInput: AVAudioSessionPortDescription?
    
    private let audioSession = AVAudioSession.sharedInstance()
    private var audioRecorder: AVAudioRecorder?
    private var recordingTimer: DispatchSourceTimer?
    private var currentChunkStartTime: Date?
    private var previousChunkData: Data? // For 2-second overlap
    private var previousChunkEndTime: Date?
    private var backgroundTask: UIBackgroundTaskIdentifier = .invalid
    private let timerQueue = DispatchQueue(label: "com.ariata.audio.timer", qos: .userInitiated)
    
    override init() {
        super.init()
        checkAuthorizationStatus()
        setupAudioInputMonitoring()
        updateAvailableInputs()
        loadSelectedInput()
    }
    
    deinit {
        NotificationCenter.default.removeObserver(self)
    }
    
    // MARK: - Authorization
    
    func requestAuthorization() async -> Bool {
        // Request microphone permission
        let micPermission = await requestMicrophonePermission()
        return micPermission
    }
    
    private func requestMicrophonePermission() async -> Bool {
        return await withCheckedContinuation { continuation in
            AVAudioSession.sharedInstance().requestRecordPermission { granted in
                DispatchQueue.main.async {
                    self.checkAuthorizationStatus()
                    continuation.resume(returning: granted)
                }
            }
        }
    }
    
    func checkAuthorizationStatus() {
        microphoneAuthorizationStatus = AVAudioSession.sharedInstance().recordPermission
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
            print("🔇 Audio interruption began (phone call, etc.)")
            // Pause recording if active
            if audioRecorder?.isRecording == true {
                audioRecorder?.pause()
            }
            
        case .ended:
            print("🔊 Audio interruption ended")
            // Check if we should resume
            if let options = info[AVAudioSessionInterruptionOptionKey] as? UInt {
                let shouldResume = AVAudioSession.InterruptionOptions(rawValue: options).contains(.shouldResume)
                
                if shouldResume && isRecording {
                    // Re-activate audio session and resume recording
                    do {
                        try audioSession.setActive(true)
                        audioRecorder?.record()
                        print("✅ Resumed audio recording after interruption")
                    } catch {
                        print("❌ Failed to resume recording after interruption: \(error)")
                        // Try to restart recording completely
                        stopRecording()
                        startRecording()
                    }
                }
            }
            
        @unknown default:
            break
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
            print("⏸️ Audio stream disabled in web app configuration")
            return
        }
        
        print("🎤 Starting audio recording")
        do {
            try setupAudioSession()
            print("✅ Audio session configured")
            startRecordingChunk()
            isRecording = true
            print("🎤 Started audio recording with 30-second chunks")
        } catch {
            print("❌ Failed to start recording: \(error)")
        }
    }
    
    func stopRecording() {
        recordingTimer?.cancel()
        recordingTimer = nil
        audioRecorder?.stop()
        audioRecorder = nil
        isRecording = false
        
        print("🛑 Stopped audio recording")
    }
}

// MARK: - Chunk Recording

extension AudioManager {
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
            print("🎵 Started recording new 30-second audio chunk")
            
            // Use DispatchSourceTimer for more reliable background execution
            let timer = DispatchSource.makeTimerSource(queue: timerQueue)
            timer.schedule(deadline: .now() + 30.0)
            timer.setEventHandler { [weak self] in
                print("⏰ Audio chunk timer fired - finishing chunk...")
                self?.finishCurrentChunk()
            }
            timer.resume()
            recordingTimer = timer
            print("⏱️ Audio chunk timer scheduled for 30 seconds")
        } catch {
            print("❌ Failed to start recording chunk: \(error)")
        }
    }
    
    private func finishCurrentChunk() {
        guard let recorder = audioRecorder,
              let startTime = currentChunkStartTime,
              recorder.isRecording else {
            return
        }
        
        recorder.stop()
        let endTime = Date()
        
        // Process the recorded audio
        if let audioData = try? Data(contentsOf: recorder.url) {
            // Create audio chunk with overlap handling
            var finalAudioData = audioData
            
            // If we have previous chunk data, prepend the last 2 seconds
            if let previousData = previousChunkData {
                finalAudioData = previousData + audioData
            }
            
            // Save last 2 seconds of current chunk for next overlap
            saveOverlapData(from: audioData, duration: 2.0)
            
            // Create chunk object
            let chunk = AudioChunk(
                startDate: startTime,
                endDate: endTime,
                audioData: finalAudioData,
                overlapDuration: previousChunkData != nil ? 2.0 : 0.0
            )
            
            let sizeKB = Double(finalAudioData.count) / 1024.0
            print("🎵 Recorded audio chunk: \(chunk.duration)s, size: \(String(format: "%.1f", sizeKB))KB")
            
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
    
    private func saveOverlapData(from audioData: Data, duration: Double) {
        // This is a simplified version - in production, you'd extract actual audio samples
        // For now, we'll save the last portion of the data
        let overlapRatio = duration / 30.0 // 2 seconds out of 30
        let overlapSize = Int(Double(audioData.count) * overlapRatio)
        let startIndex = audioData.count - overlapSize
        
        if startIndex >= 0 && startIndex < audioData.count {
            previousChunkData = audioData.subdata(in: startIndex..<audioData.count)
            previousChunkEndTime = Date()
        }
    }
    
    private func saveAudioChunk(_ chunk: AudioChunk) {
        // Begin background task
        beginBackgroundTask()
        
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
                print("✅ Saved audio chunk to SQLite queue")
                Task { @MainActor in
                    self.lastSaveDate = Date()
                }
                
                // Update stats in upload coordinator
                BatchUploadCoordinator.shared.updateUploadStats()
            } else {
                print("❌ Failed to save audio chunk to SQLite queue")
            }
        } catch {
            print("❌ Failed to encode audio chunk: \(error)")
        }
        
        endBackgroundTask()
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