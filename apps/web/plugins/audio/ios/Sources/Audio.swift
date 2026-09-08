import AVFoundation
import CallKit
import Foundation
import UIKit
import UserNotifications

// Shared outbox enqueue (defined in the reach plugin's ffi.rs). The whole app
// links one static lib, so this resolves by symbol name — no bridging header.
@_silgen_name("virtues_enqueue")
private func virtues_enqueue(_ stream: UnsafePointer<CChar>, _ json: UnsafePointer<CChar>) -> Int32

// Push recording health to the location plugin (its @_cdecl, same one-static-lib
// linkage). Location runs coarse (GNSS off) only while we record healthily; the
// moment we're down it escalates to precise so its callbacks become the fast
// re-arm heartbeat again. 0 = off, 1 = enabled but down, 2 = recording.
@_silgen_name("virtues_location_audio_state")
private func virtues_location_audio_state(_ state: Int32)

private let isoMillis: ISO8601DateFormatter = {
  let f = ISO8601DateFormatter()
  f.formatOptions = [.withInternetDateTime, .withFractionalSeconds]
  return f
}()

/// Continuous ambient microphone recorder — **never-stopped capture graph**.
///
/// iOS forbids *starting* the mic from the background, and an `AVAudioRecorder`
/// chunk boundary (`stop()` → new recorder) IS a background start — so it dies at
/// the first ~5-min background boundary. The fix (researched): one `AVAudioEngine`
/// **armed once in the foreground and never stopped**; a continuous input tap
/// downsamples to 16 kHz mono and writes to an `AVAudioFile` that we **rotate**
/// (swap the output file) at chunk boundaries — no stop, no gap, no background
/// start. Each rotated file is a standalone `.m4a` → box pipeline unchanged.
///
/// Session: `.playAndRecord` + [.mixWithOthers, .allowBluetoothA2DP], built-in mic
/// pinned. The WWDC-lab coexistence recipe: A2DP is output-only so our mic uses the
/// built-in mic while the user's music stays high-quality on their AirPods, and
/// mixing means we're never interrupted → the session stays live 24/7 (only phone
/// calls gap us). No `.defaultToSpeaker`/`.allowBluetooth` — those were the poison
/// pills that forced the speaker / tugged AirPods in our earlier tests.
///
/// Resurrection (safety net for the rare hard interruptions — calls / Siri / media
/// reset): re-arm on interruption-end / foreground / location wake / config-change,
/// each wrapped in a background-task assertion.
public final class AudioRecorder: NSObject {
  public static let shared = AudioRecorder()

  private let chunkSeconds: Double = 300  // rotation interval (gapless — pure cut)
  private let enabledKey = "virtues.audio.enabled"
  private let targetSampleRate = 16000.0
  private let targetBitRate = 24000  // ~24 kbps mono AAC
  // virtues_enqueue back-pressure code (reach ffi.rs): the outbox refused the
  // row for capacity — keep the file, and expect the same answer until acks
  // free space.
  private let rcOutboxCapacity: Int32 = -5

  // Gap-nudge: notify the user if recording is meant to be on but has been silently
  // down for a while (the cases the watchdog can't self-heal — app killed, exotic
  // takeovers). Default ON; user-toggleable. Suppressed during phone calls.
  private let notifyKey = "virtues.audio.notifyOnStop"        // user toggle (default true)
  private let silentDroppedKey = "virtues.audio.silentDropped" // metadata-only chunk count
  private let lastGoodKey = "virtues.audio.lastGoodCapture"   // persisted across launches
  private let gapThreshold: TimeInterval = 300                // 5 min sustained gap
  private let nudgeId = "virtues.audio.gap"                   // fixed id → dedupe + auto-clear
  private let callObserver = CXCallObserver()
  private var nudgeFired = false
  private var lastGoodPersistAt: Date?

  // In-memory mirrors of the UserDefaults flags the 5s watchdog tick consults.
  // Every mutation goes through this class, so the mirrors can't drift; without
  // them the tick did several cfprefsd lookups per beat, ~17k/day of waste.
  private var cachedEnabled = false
  private var cachedNotify = true
  private var cachedLastGood: TimeInterval = 0

  // Quiet hours — MUTE, DON'T RELEASE. Inside the window the engine, session,
  // and tap all stay exactly as they are (releasing the mic would hit the
  // background-restart wall at window end — an app nobody opens at 7am would
  // never resume); `process()` just stops writing chunks. The mic stays hot, so
  // this is a privacy/alarm-margin feature more than a battery one — but it
  // does eliminate the window's chunk encode + drain dials. The window is a
  // pair of minutes-since-midnight in LOCAL wall-clock time; start==end or
  // unset (-1) = off; start>end wraps midnight (22:00→07:00).
  private let quietStartKey = "virtues.audio.quietStart"
  private let quietEndKey = "virtues.audio.quietEnd"
  private var cachedQuietStart: Int = -1
  private var cachedQuietEnd: Int = -1

  private func quietHoursActive(_ now: Date) -> Bool {
    let start = cachedQuietStart, end = cachedQuietEnd
    if start < 0 || end < 0 || start == end { return false }
    let c = Calendar.current.dateComponents([.hour, .minute], from: now)
    let m = (c.hour ?? 0) * 60 + (c.minute ?? 0)
    return start < end ? (m >= start && m < end) : (m >= start || m < end)
  }

  public func quietHours() -> (start: Int, end: Int) {
    (cachedQuietStart, cachedQuietEnd)
  }

  public func setQuietHours(start: Int, end: Int) {
    cachedQuietStart = start
    cachedQuietEnd = end
    let d = UserDefaults.standard
    d.set(start, forKey: quietStartKey)
    d.set(end, forKey: quietEndKey)
    NSLog("[Audio] quiet hours set %d..%d (minutes, -1=off)", start, end)
  }

  private let session = AVAudioSession.sharedInstance()
  private let engine = AVAudioEngine()
  private var converter: AVAudioConverter?
  private var hwFormat: AVAudioFormat?
  private let targetFormat = AVAudioFormat(
    commonFormat: .pcmFormatFloat32, sampleRate: 16000, channels: 1, interleaved: false)!

  private var outFile: AVAudioFile?
  private var chunkStart: Date?
  private var framesInChunk: AVAudioFrameCount = 0
  private var sumSq: Double = 0
  private var sampleCount: Double = 0
  private var peak: Float = 0
  private var tapInstalled = false

  // Liveness: `recording` can be true while the tap silently stops firing (iOS
  // 18.4+ interruption bug — start() succeeds but no buffers arrive). We stamp each
  // tap buffer and a watchdog forces a rebuild if the stamp goes stale.
  private var lastBufferAt: Date?
  private var watchdog: DispatchSourceTimer?
  private let livenessTimeout: TimeInterval = 5

  // Notified-interruption hold: iOS gives us two distinct down-states and they need
  // different recovery cadences. A SILENT death (18.4 tap bug — no notification)
  // wants the fast 5s watchdog rebuild. A NOTIFIED interruption (`.began` — alarm,
  // call, Siri) means higher-priority audio owns the session BY DESIGN: re-arming
  // `setActive(true)` against it every 5s is a fight we lose, and mid-alarm it cut
  // the user's wake-up alarm to a snippet. While the hold is set, watchdog re-arms
  // slow to one per `interruptedRetryInterval` (a fallback for the documented
  // cases where `.ended` never arrives); the REAL resume is event-driven — `.ended`
  // / foreground / route change / media reset clear the hold, and a flowing buffer
  // clears a stale one.
  private var interruptionHoldUntil: Date?
  private let interruptedRetryInterval: TimeInterval = 60

  // Serialize engine lifecycle (start/stop/restart) off the realtime tap thread.
  private let q = DispatchQueue(label: "com.virtues.audio", qos: .userInitiated)

  private(set) var recording = false

  private override init() {
    super.init()
    let d = UserDefaults.standard
    cachedEnabled = d.bool(forKey: enabledKey)
    cachedNotify = d.object(forKey: notifyKey) == nil ? true : d.bool(forKey: notifyKey)
    cachedLastGood = d.double(forKey: lastGoodKey)
    cachedQuietStart = d.object(forKey: quietStartKey) == nil ? -1 : d.integer(forKey: quietStartKey)
    cachedQuietEnd = d.object(forKey: quietEndKey) == nil ? -1 : d.integer(forKey: quietEndKey)
    let nc = NotificationCenter.default
    nc.addObserver(self, selector: #selector(handleInterruption),
      name: AVAudioSession.interruptionNotification, object: session)
    nc.addObserver(self, selector: #selector(handleRouteChange),
      name: AVAudioSession.routeChangeNotification, object: session)
    nc.addObserver(self, selector: #selector(handleMediaReset),
      name: AVAudioSession.mediaServicesWereResetNotification, object: session)
    nc.addObserver(self, selector: #selector(handleConfigChange),
      name: Notification.Name.AVAudioEngineConfigurationChange, object: engine)
    nc.addObserver(self, selector: #selector(handleForeground),
      name: UIApplication.didBecomeActiveNotification, object: nil)
  }

  // MARK: - Authorization

  public func authorized() -> Bool {
    if #available(iOS 17.0, *) {
      return AVAudioApplication.shared.recordPermission == .granted
    } else {
      return session.recordPermission == .granted
    }
  }

  private func requestPermission(_ completion: @escaping (Bool) -> Void) {
    if #available(iOS 17.0, *) {
      AVAudioApplication.requestRecordPermission { granted in completion(granted) }
    } else {
      session.requestRecordPermission { granted in completion(granted) }
    }
  }

  // MARK: - Public control (plugin commands)

  /// Explicit opt-in: prompt, persist enabled, arm the engine (must be foreground).
  public func enable(_ completion: @escaping (Bool) -> Void) {
    requestPermission { [weak self] granted in
      guard let self = self else { completion(false); return }
      if granted {
        self.cachedEnabled = true
        UserDefaults.standard.set(true, forKey: self.enabledKey)
        // Ask for notification permission in context (they just opted into the
        // feature the gap-nudge protects). Denial just no-ops the nudge.
        UNUserNotificationCenter.current().requestAuthorization(options: [.alert, .sound]) { _, _ in }
        self.armEngine(reason: "enable")
      }
      completion(granted)
    }
  }

  /// Toggle off / pause: clear the enabled flag, finalize, and stop the engine.
  public func disable() {
    cachedEnabled = false
    UserDefaults.standard.set(false, forKey: enabledKey)
    virtues_location_audio_state(0)
    // stopWatchdog runs on q: `watchdog` is written by startWatchdog on q (via
    // startEngine), and stopping it from the caller's thread (Tauri's ipc
    // queue) is an unsynchronized ARC assignment race. A tick that fires
    // before this block lands no-ops — cachedEnabled is already false.
    q.async { [weak self] in
      guard let self = self else { return }
      self.stopWatchdog()
      self.stopEngine(finalize: true)
      // Re-assert state 0 now the engine is provably down: an arm block that
      // interleaved AHEAD of this one pushed state 2 after disable's
      // synchronous 0, and stopEngine pushes nothing — location would be left
      // believing audio is healthy while it is off.
      virtues_location_audio_state(0)
    }
  }

  public func resume() {
    interruptionHoldUntil = nil  // explicit user intent outranks the hold
    ensureRecording(reason: "resume")
  }

  /// Idempotent re-arm — start if we should be recording but the engine is down (or
  /// silently wedged). Safe to spam from interruption-end / foreground / location
  /// wake / config-change / watchdog.
  public func ensureRecording(reason: String = "ensure") {
    if !Thread.isMainThread {
      DispatchQueue.main.async { [weak self] in self?.ensureRecording(reason: reason) }
      return
    }
    guard authorized(), cachedEnabled else { return }
    // Liveness: if we think we're recording but no tap buffer has arrived within the
    // timeout, the tap silently died — drop the flag so armEngine does a full
    // rebuild instead of short-circuiting on `recording == true`.
    if recording, let last = lastBufferAt, Date().timeIntervalSince(last) > livenessTimeout {
      NSLog("[Audio] tap dead (no buffers %.1fs, reason=%@) — forcing rebuild",
            Date().timeIntervalSince(last), reason)
      recording = false
    }
    if recording {
      checkGapAndNudge()  // healthy, but still evaluate (clears a stale nudge fast)
      return  // already live — don't churn a bg-task assertion (watchdog)
    }
    // Notified interruption in progress: don't fight the interrupter (see the hold's
    // declaration). Event handlers clear the hold before calling us, so this gate
    // only ever slows the watchdog. On fallback expiry, try once and re-arm the
    // hold — a success clears it via the first buffer.
    if let hold = interruptionHoldUntil {
      if Date() < hold { return }
      interruptionHoldUntil = Date().addingTimeInterval(interruptedRetryInterval)
    }
    armEngine(reason: reason)
    // Evaluate the gap AFTER attempting recovery: if the arm just succeeded a buffer
    // will land and clear things; if it failed (killed / bg-start wall / exotic
    // takeover), this is where we decide to nudge.
    checkGapAndNudge()
  }

  // MARK: - Gap nudge

  /// Chunks shipped metadata-only because they measured silent (battery stats).
  public func silentDroppedCount() -> Int {
    UserDefaults.standard.integer(forKey: silentDroppedKey)
  }

  public func notifyEnabled() -> Bool {
    cachedNotify  // default-ON semantics live in the cache's init
  }

  public func setNotifyEnabled(_ on: Bool) {
    cachedNotify = on
    UserDefaults.standard.set(on, forKey: notifyKey)
    if !on { clearNudge(); nudgeFired = false }
  }

  private func callActive() -> Bool {
    callObserver.calls.contains { !$0.hasEnded }
  }

  /// Decide whether to surface "Recording paused — tap to resume". Fires once per
  /// gap episode (reset when a buffer flows again, in `process`). Suppressed while a
  /// call owns the mic (that gap is expected + self-heals on hang-up).
  private func checkGapAndNudge() {
    guard cachedEnabled, cachedNotify else { return }
    if nudgeFired { return }              // already showing — once-and-done per episode
    if callActive() { return }            // legit call gap — never nudge
    // Gap measured from the persisted last-good-capture (survives kill→relaunch), so
    // a nudge fires ~gapThreshold after recording ACTUALLY died, not after relaunch.
    let lastGood = cachedLastGood
    guard lastGood > 0 else { return }    // never captured yet → nothing to nudge about
    if Date().timeIntervalSince1970 - lastGood > gapThreshold {
      fireNudge()
    }
  }

  private func fireNudge() {
    nudgeFired = true
    let content = UNMutableNotificationContent()
    content.title = "Recording paused"
    content.body = "Tap to resume recording."
    // `.active` (polite, no Focus break-through) is already the default interruption
    // level, so we don't set it — avoids the iOS 15 availability gate.
    let req = UNNotificationRequest(identifier: nudgeId, content: content, trigger: nil)
    UNUserNotificationCenter.current().add(req)
    NSLog("[Audio] gap nudge fired (down >%.0fs)", gapThreshold)
  }

  private func clearNudge() {
    let c = UNUserNotificationCenter.current()
    c.removePendingNotificationRequests(withIdentifiers: [nudgeId])
    c.removeDeliveredNotifications(withIdentifiers: [nudgeId])
  }

  /// Repeating liveness probe: fires even when foregrounded + stationary (when
  /// location isn't waking us), so a silently-dead tap self-heals within ~5s.
  private func startWatchdog() {
    if watchdog != nil { return }
    let t = DispatchSource.makeTimerSource(queue: q)
    t.schedule(deadline: .now() + livenessTimeout, repeating: livenessTimeout)
    t.setEventHandler { [weak self] in self?.ensureRecording(reason: "watchdog") }
    watchdog = t
    t.resume()
  }

  private func stopWatchdog() {
    watchdog?.cancel()
    watchdog = nil
  }

  // MARK: - Engine lifecycle

  /// Arm/re-arm the capture graph, wrapped in a background-task assertion so a
  /// background re-arm has execution time (the piggyback test — does iOS allow it?).
  private func armEngine(reason: String) {
    var bg = UIBackgroundTaskIdentifier.invalid
    bg = UIApplication.shared.beginBackgroundTask(withName: "virtues-audio-arm") {
      UIApplication.shared.endBackgroundTask(bg); bg = .invalid
    }
    q.async { [weak self] in
      guard let self = self else { return }
      // Re-check the desire flag ON q: a tick can pass ensureRecording's guard
      // on main, then disable() clears the flag and enqueues its stop block
      // ahead of this one — without this check the stale arm restarts the
      // engine after the user said stop, and nothing ever stops it again
      // (ensureRecording bails on the flag without tearing down). The q block
      // that ran disable's stop has acquired the flag write, so this read is
      // ordered.
      if self.cachedEnabled, !self.recording {
        self.startEngine(reason: reason)
      }
      if bg != .invalid { UIApplication.shared.endBackgroundTask(bg); bg = .invalid }
    }
  }

  private func startEngine(reason: String) {
    do {
      try configureSession()
      let input = engine.inputNode
      // ALWAYS tear down and reinstall the tap. After an interruption iOS can leave
      // the engine "running" (start() succeeds) while the old tap silently stops
      // firing (iOS 18.4+ bug) — reusing it re-arms onto a corpse (recording=true,
      // no buffers, no dot). A fresh stop→removeTap→installTap→start is the
      // documented recovery (same as the manual disable/enable the user found).
      if engine.isRunning { engine.stop() }
      if tapInstalled { input.removeTap(onBus: 0); tapInstalled = false }
      let hw = input.inputFormat(forBus: 0)
      // A phone call (or any exclusive owner of the mic) can leave the input
      // node at 0 Hz/0 ch even after setActive(true) succeeds — .mixWithOthers
      // activates fine mid-call. installTap with that format throws an ObjC
      // exception no Swift catch can see → SIGABRT (the 1.2.14 field crash).
      // Guarded BEFORE hwFormat is stored so a straggler tap callback can
      // never divide by a 0 Hz rate in process().
      guard hw.sampleRate > 0, hw.channelCount > 0 else {
        NSLog("[Audio] arm failed (%@): unusable input format %.0fHz/%dch — will retry on next wake/foreground",
              reason, hw.sampleRate, hw.channelCount)
        bailArm()
        return
      }
      // A nil converter is the sibling wedge: buffers would flow and stamp the
      // health signals while zero bytes ever get written, invisible to the
      // watchdog. Logged apart from the format case — it arrives with a
      // healthy-looking format, and a shared line would read as a format
      // problem that does not exist.
      guard let conv = AVAudioConverter(from: hw, to: targetFormat) else {
        NSLog("[Audio] arm failed (%@): converter init failed for %.0fHz/%dch — will retry on next wake/foreground",
              reason, hw.sampleRate, hw.channelCount)
        bailArm()
        return
      }
      hwFormat = hw
      converter = conv
      input.installTap(onBus: 0, bufferSize: 4096, format: hw) { [weak self] buf, _ in
        self?.process(buf)
      }
      tapInstalled = true
      lastBufferAt = Date()  // reset heartbeat so the watchdog gives the new tap time
      try openChunk()
      engine.prepare()
      try engine.start()
      recording = true
      startWatchdog()
      virtues_location_audio_state(2)
      NSLog("[Audio] engine armed (%@) hw=%.0fHz/%dch", reason, hw.sampleRate, hw.channelCount)
      // After the live chunk is open, so the sweep can tell it apart from the
      // orphans it is retrying.
      sweepOrphanChunks()
    } catch {
      NSLog("[Audio] arm failed (%@): %@ — will retry on next wake/foreground",
            reason, error.localizedDescription)
      bailArm()
    }
  }

  /// Shared exit for an arm attempt that cannot proceed (unusable format,
  /// converter failure, configure/start throw). Mid-call a relaunched process
  /// has no .began hold (and foreground clears any hold), so without one the
  /// watchdog spins 5s bails against the call — match the .began cadence for
  /// the call's remainder instead.
  private func bailArm() {
    recording = false
    virtues_location_audio_state(1)
    if callActive() {
      interruptionHoldUntil = Date().addingTimeInterval(interruptedRetryInterval)
    }
  }

  private func stopEngine(finalize: Bool) {
    if finalize { rotate(restart: false) }
    if tapInstalled { engine.inputNode.removeTap(onBus: 0); tapInstalled = false }
    if engine.isRunning { engine.stop() }
    recording = false
    NSLog("[Audio] engine stopped")
  }

  private func configureSession() throws {
    // `.playAndRecord` + [.mixWithOthers, .allowBluetoothA2DP, .defaultToSpeaker],
    // mode `.default`. The coexistence recipe, tuned on-device. Why each option:
    //   • `.mixWithOthers` — never interrupt / never get interrupted → session stays
    //     live 24/7 → the "can't restart mic from background" wall is never hit
    //     (only phone calls gap us, and we auto-resume on interruption-end).
    //   • `.allowBluetoothA2DP` — A2DP is OUTPUT-ONLY, so iOS can't route our mic to
    //     the AirPods; it uses the built-in mic while the user's audio stays
    //     high-quality on their AirPods. (`.allowBluetooth`/HFP is the POISON PILL —
    //     it grabs the AirPods mic in mono call-mode; never add it.)
    // NOTE: we deliberately do NOT put `.defaultToSpeaker` in the category. It means
    // "force output to the phone speaker," so re-activating in the BACKGROUND while
    // another app owns the AirPods route would require seizing that route → iOS
    // rejects it with `.cannotInterruptOthers` (560557684) on EVERY retry, wedging
    // recording after a speaker→AirPods switch. The earpiece fix is instead done
    // dynamically below, only when it can't interrupt anyone.
    //
    // Residual risk: the AirPods TUG (Continuity moving them Mac→iPhone) has NO
    // programmatic opt-out — input activity alone can trigger it. This config
    // minimizes it (never requests an AirPods route); the hard fix is the user's
    // AirPods setting "Connect to This iPhone → When Last Connected to This iPhone".
    // We deliberately do NOT register MPNowPlayingInfoCenter (a second tug trigger).
    // Pin the built-in mic BEFORE activating (belt-and-suspenders).
    try session.setCategory(
      .playAndRecord, mode: .default, options: [.mixWithOthers, .allowBluetoothA2DP])
    // Largest IO buffer iOS grants (~93ms / 4096 frames vs the ~23ms default):
    // ~4× fewer audio-thread wakeups, which matters for a 24/7 capture graph.
    // Ambient recording has no latency requirement. Best-effort — iOS clamps.
    try? session.setPreferredIOBufferDuration(0.093)
    if let builtIn = session.availableInputs?.first(where: { $0.portType == .builtInMic }) {
      try? session.setPreferredInput(builtIn)
    }
    try session.setActive(true, options: .notifyOthersOnDeactivation)

    // Earpiece fix, done safely: `.playAndRecord` defaults output to the RECEIVER
    // (quiet top earpiece), and our always-on session owns the shared route — so
    // other apps' playback would come out the earpiece. Bump it to the bottom
    // speaker, but ONLY when there's no external output (built-in receiver/speaker).
    // When AirPods/headphones are active we override to `.none` (leave their route
    // untouched) — never seizing it, so we never hit `.cannotInterruptOthers`. This
    // only affects OTHER apps' output; our capture is unaffected, so `try?` is safe.
    let outs = session.currentRoute.outputs.map { $0.portType }
    let hasExternal = outs.contains { $0 != .builtInReceiver && $0 != .builtInSpeaker }
    try? session.overrideOutputAudioPort(hasExternal ? .none : .speaker)
  }

  // MARK: - Chunk file (rotate without stopping the engine)

  private func openChunk() throws {
    let dir = FileManager.default.urls(for: .documentDirectory, in: .userDomainMask)[0]
    let url = dir.appendingPathComponent("mic_\(UUID().uuidString).m4a")
    let settings: [String: Any] = [
      AVFormatIDKey: Int(kAudioFormatMPEG4AAC),
      AVSampleRateKey: targetSampleRate,
      AVNumberOfChannelsKey: 1,
      AVEncoderBitRateKey: targetBitRate,
    ]
    outFile = try AVAudioFile(forWriting: url, settings: settings,
                              commonFormat: .pcmFormatFloat32, interleaved: false)
    excludeFromBackup(url)
    chunkStart = Date()
    framesInChunk = 0
    sumSq = 0; sampleCount = 0; peak = 0
  }

  /// Chunks outlive their enqueue by days when the outbox is at capacity
  /// (rc -5 keeps them for the sweep), and Documents rides iCloud/device
  /// backups by default — raw room audio must not reach a backup. Same
  /// posture as the outbox dir's exclusion in ReachPlugin. Best-effort, but
  /// a failure is logged: a silent one would put audio back in backups.
  private func excludeFromBackup(_ url: URL) {
    var u = url
    var vals = URLResourceValues()
    vals.isExcludedFromBackup = true
    do {
      try u.setResourceValues(vals)
    } catch {
      NSLog("[Audio] backup exclusion failed for %@: %@",
            url.lastPathComponent, error.localizedDescription)
    }
  }

  /// Called on the realtime tap thread for every input buffer.
  private func process(_ input: AVAudioPCMBuffer) {
    let now = Date()
    lastBufferAt = now  // liveness heartbeat (watchdog reads this)
    // Persist "last good capture" (throttled) so a kill→relaunch measures the REAL
    // gap (from when recording actually died, not from relaunch). And if a nudge was
    // showing, we've recovered — clear it so no stale "paused" lie lingers.
    if lastGoodPersistAt == nil || now.timeIntervalSince(lastGoodPersistAt!) > 60 {
      lastGoodPersistAt = now
      cachedLastGood = now.timeIntervalSince1970
      UserDefaults.standard.set(now.timeIntervalSince1970, forKey: lastGoodKey)
    }
    if nudgeFired {
      nudgeFired = false
      DispatchQueue.main.async { [weak self] in self?.clearNudge() }
    }
    interruptionHoldUntil = nil  // audio is flowing — any interruption is over
    // Quiet hours: mute, don't release. Everything above still ran — the
    // heartbeat and lastGood stamps say "capture is HEALTHY, just muted", which
    // keeps the watchdog quiet, the gap nudge silent, and location in its cheap
    // mode. On window entry the partial chunk finalizes once (outFile goes nil);
    // on exit the next buffer reopens a chunk and capture resumes seamlessly.
    if quietHoursActive(now) {
      if let f = outFile {
        if sampleCount > 0 {
          rotate(restart: false)
        } else {
          // Chunk opened but zero samples converted (window entry within one
          // buffer of a rotation): rotate() would no-op on its sampleCount
          // guard and the writer would stay open across the whole window — the
          // chunk would then span the gap with lying timestamps. Discard the
          // empty writer (dealloc finalizes it) and delete the husk.
          let url = f.url
          outFile = nil
          try? FileManager.default.removeItem(at: url)
        }
      }
      return
    }
    if outFile == nil { try? openChunk() }
    guard let converter = converter else { return }
    let ratio = targetSampleRate / (hwFormat?.sampleRate ?? targetSampleRate)
    // A 0 Hz hwFormat would make this infinite and the AVAudioFrameCount cast
    // below traps on non-finite doubles. The arm guard keeps 0 Hz out of
    // hwFormat; this covers any straggler buffer racing a teardown.
    guard ratio.isFinite, ratio > 0 else { return }
    let cap = AVAudioFrameCount(Double(input.frameLength) * ratio) + 32
    guard let out = AVAudioPCMBuffer(pcmFormat: targetFormat, frameCapacity: cap) else { return }
    var err: NSError?
    var fed = false
    converter.convert(to: out, error: &err) { _, status in
      if fed { status.pointee = .noDataNow; return nil }
      fed = true; status.pointee = .haveData; return input
    }
    if err != nil || out.frameLength == 0 { return }

    // Meter (dBFS) from the 16 kHz mono float samples.
    if let ch = out.floatChannelData?[0] {
      let n = Int(out.frameLength)
      for i in 0..<n {
        let s = ch[i]
        sumSq += Double(s * s)
        let a = abs(s)
        if a > peak { peak = a }
      }
      sampleCount += Double(n)
    }

    try? outFile?.write(from: out)
    framesInChunk += out.frameLength

    if Double(framesInChunk) >= chunkSeconds * targetSampleRate {
      rotate(restart: true)
    }
  }

  /// Close the current file (→ finalized on dealloc), enqueue it, open a fresh one.
  /// Runs on the tap thread at a boundary, or on `q` for stop/teardown.
  private func rotate(restart: Bool) {
    guard outFile != nil, let start = chunkStart, sampleCount > 0 else {
      if restart { try? openChunk() }
      return
    }
    let url = outFile!.url
    let end = Date()
    let rms = sqrt(sumSq / sampleCount)
    let avgDb = rms > 0 ? 20 * log10(Float(rms)) : -160
    let peakDb = peak > 0 ? 20 * log10(peak) : -160
    // Drop the writer's ONLY strong reference so ARC deallocates it RIGHT NOW — its
    // destructor is what writes the `moov` atom (codec config + sample tables) that
    // makes the .m4a a valid, decodable file. CRITICAL: do NOT bind `let done =
    // outFile` and read via that — a lingering strong local keeps the file alive
    // past `outFile = nil`, so the destructor never runs before the async read
    // below, and we ship an unfinalized chunk (ftyp + raw AAC, no moov). That is
    // exactly the corruption that made every chunk unplayable + Gemini hallucinate.
    // Reading `outFile!.url` into a value first, then nil-ing, guarantees no
    // surviving reference → synchronous dealloc → finalized file → safe to read.
    outFile = nil
    if restart { try? openChunk() }

    let silent = avgDb < -60 && peakDb < -50
    q.async { [weak self] in
      self?.finalizeAndEnqueue(url: url, start: start, end: end, avgDb: avgDb, silent: silent)
    }
  }

  /// Returns the outbox rc (0 = taken, rcOutboxCapacity = refused for space,
  /// other negatives = broken) so the orphan sweep can stop on a deterministic
  /// refusal; 0 when the chunk was dropped as worthless, -1 when our own
  /// serialization failed (file kept). rotate() ignores the value.
  @discardableResult
  private func finalizeAndEnqueue(
    url: URL, start: Date, end: Date, avgDb: Float, silent: Bool
  ) -> Int32 {
    // NO blanket `defer { removeItem }` here. Every other stream this app
    // collects re-reads a source that persists — chat.db, HealthKit, Contacts —
    // so a failed handoff costs a retry. Audio's source IS this file. Deleting
    // it unconditionally, before the enqueue below has even run, made a full
    // disk or an uninitialized outbox destroy the recording permanently, with
    // one NSLog as the only trace. The file is now deleted at exactly two
    // points: when it is provably worthless, and when the outbox has taken it.
    guard let data = try? Data(contentsOf: url), data.count > 1000 else {
      // Worthless: nothing recoverable in a truncated or unreadable husk.
      NSLog("[Audio] chunk too small / unreadable, dropping")
      try? FileManager.default.removeItem(at: url)
      return 0
    }
    var rec: [String: Any] = [
      "id": UUID().uuidString,
      "audio_format": "m4a",
      "timestamp_start": isoMillis.string(from: start),
      "timestamp_end": isoMillis.string(from: end),
      "duration_seconds": end.timeIntervalSince(start),
      "is_silent": silent,
      "average_db_level": Double(avgDb),
    ]
    // Silent chunks (avg < -60 dBFS AND peak < -50 — an empty room) ship
    // metadata only: the timeline still shows the period was covered, but the
    // ~900 KB of measured silence never rides the radio. The box inserts a NULL
    // audio_url row and the transcriber skips it as before.
    if silent {
      let dropped = UserDefaults.standard.integer(forKey: silentDroppedKey) + 1
      UserDefaults.standard.set(dropped, forKey: silentDroppedKey)
    } else {
      rec["audio_data"] = data.base64EncodedString()
    }
    guard let json = try? JSONSerialization.data(withJSONObject: rec),
          let str = String(data: json, encoding: .utf8) else {
      // Our own encoding bug, not the file's fault — keep the audio. The sweep
      // will retry it, and its age cap eventually bounds the damage.
      NSLog("[Audio] chunk failed to serialize — KEPT at %@", url.lastPathComponent)
      return -1
    }
    let rc = "microphone".withCString { s in str.withCString { j in virtues_enqueue(s, j) } }
    NSLog("[Audio] enqueued chunk %d bytes, avg=%.0fdB silent=%@ rc=%d",
          data.count, avgDb, silent ? "y (metadata-only)" : "n", rc)
    guard rc == 0 else {
      // The outbox did NOT take it (capacity, uninitialized, disk full, SQLite
      // error). Keeping the file is the whole point: the outbox is durable once
      // a row lands, so the only unrecoverable window is right here.
      NSLog("[Audio] enqueue failed rc=%d — chunk KEPT at %@", rc, url.lastPathComponent)
      return rc
    }
    try? FileManager.default.removeItem(at: url)
    return 0
  }

  /// Re-offer chunks that a previous run wrote but could not hand to the outbox.
  ///
  /// This is the other half of not deleting on failure: without it, a retained
  /// chunk is never retried and never removed, and Documents grows without
  /// bound. Runs on arm, which is the same trigger that would have produced
  /// them, so recovery needs no new schedule.
  ///
  /// `maxAge` is a backstop, not a policy — a chunk nothing has accepted in a
  /// week is not going to be accepted, and unbounded growth on a phone is its
  /// own failure. It is deliberately far longer than any transient outbox
  /// problem.
  private func sweepOrphanChunks() {
    let fm = FileManager.default
    guard let dir = fm.urls(for: .documentDirectory, in: .userDomainMask).first,
          let files = try? fm.contentsOfDirectory(
            at: dir, includingPropertiesForKeys: [.contentModificationDateKey])
    else { return }
    let live = outFile?.url
    let maxAge: TimeInterval = 7 * 24 * 60 * 60
    for url in files where url.lastPathComponent.hasPrefix("mic_")
      && url.pathExtension == "m4a" && url != live {
      let modified = (try? url.resourceValues(forKeys: [.contentModificationDateKey]))?
        .contentModificationDate ?? Date()
      if Date().timeIntervalSince(modified) > maxAge {
        NSLog("[Audio] orphan chunk older than 7d, dropping %@", url.lastPathComponent)
        try? fm.removeItem(at: url)
        continue
      }
      // Timestamps are unknown for a chunk we did not just close, so bound it
      // by the file's own mtime rather than inventing a span. Better a slightly
      // imprecise window than a lost recording.
      guard let data = try? Data(contentsOf: url), data.count > 1000 else {
        try? fm.removeItem(at: url)
        continue
      }
      let end = modified
      let start = end.addingTimeInterval(-chunkSeconds)
      excludeFromBackup(url)  // legacy orphans predate creation-time flagging
      NSLog("[Audio] retrying orphan chunk %@", url.lastPathComponent)
      // Pool per chunk: this loop runs inside ONE dispatch block (startEngine
      // on q, under armEngine's bg-task assertion), and the ~2.4MB of
      // base64/JSON temporaries per chunk otherwise accumulate for the whole
      // sweep — a serial queue's default pool drains at thread idle, not per
      // block. After a long outbox outage that is jetsam bait at exactly the
      // moment the background memory limit is tightest.
      let rc = autoreleasepool {
        finalizeAndEnqueue(url: url, start: start, end: end, avgDb: -50, silent: false)
      }
      // Capacity is a property of the queue, not the chunk: every later orphan
      // in this pass gets the same refusal, so stop paying the read + base64
      // for guaranteed no's. The next arm retries; the age cap still bounds
      // the backlog.
      if rc == rcOutboxCapacity {
        NSLog("[Audio] outbox at capacity — deferring remaining orphan retries to next arm")
        break
      }
    }
  }

  // MARK: - Interruptions / route / config / foreground (resurrection vectors)

  @objc private func handleInterruption(_ note: Notification) {
    guard let info = note.userInfo,
      let raw = info[AVAudioSessionInterruptionTypeKey] as? UInt,
      let type = AVAudioSession.InterruptionType(rawValue: raw) else { return }
    switch type {
    case .began:
      // Alarm/call/Siri deactivated our session + stopped the engine. Finalize the
      // partial chunk. Do NOT setActive(false) — keep it as recoverable as possible.
      // Log the reason (iOS 14.5+): field debugging can't otherwise tell an alarm
      // from a call from an app-suspend, and the fix for each differs.
      var why = "n/a"
      if #available(iOS 14.5, *), let raw = info[AVAudioSessionInterruptionReasonKey] as? UInt {
        switch AVAudioSession.InterruptionReason(rawValue: raw) {
        case .default: why = "default"
        case .builtInMicMuted: why = "builtInMicMuted"
        case .some(let other): why = "raw=\(other.rawValue)"  // e.g. routeDisconnected (iOS 17)
        case .none: why = "raw=\(raw)"
        }
      }
      NSLog("[Audio] interruption began (reason=%@) — holding re-arms", why)
      interruptionHoldUntil = Date().addingTimeInterval(interruptedRetryInterval)
      virtues_location_audio_state(1)
      q.async { [weak self] in
        guard let self = self else { return }
        self.rotate(restart: false)
        self.recording = false
      }
    case .ended:
      // Always TRY to resume (drop the .shouldResume gate — an always-on recorder
      // always wants back). armEngine wraps it in a bg-task assertion so we probe
      // whether a background reactivation is allowed at all.
      NSLog("[Audio] interruption ended — attempting resume")
      interruptionHoldUntil = nil
      DispatchQueue.main.asyncAfter(deadline: .now() + 0.3) { [weak self] in
        self?.ensureRecording(reason: "interruption-end")
      }
    @unknown default: break
    }
  }

  @objc private func handleConfigChange(_ note: Notification) {
    // Route/format change stopped the engine. Finalize + re-arm (new hw format).
    // Deliberately does NOT clear interruptionHoldUntil: a config change can be
    // a SIDE EFFECT of the interruption that set the hold (the engine stops when
    // an alarm/call takes the session), and clearing here would reintroduce the
    // mid-ring re-arm fight. If a hold stands, the re-arm below gates and the
    // real resume arrives with `.ended`; with no hold, behavior is unchanged.
    NSLog("[Audio] engine config change — re-arming")
    virtues_location_audio_state(1)
    q.async { [weak self] in
      guard let self = self else { return }
      self.rotate(restart: false)
      if self.tapInstalled { self.engine.inputNode.removeTap(onBus: 0); self.tapInstalled = false }
      if self.engine.isRunning { self.engine.stop() }
      self.recording = false
    }
    DispatchQueue.main.asyncAfter(deadline: .now() + 0.25) { [weak self] in
      self?.ensureRecording(reason: "config-change")
    }
  }

  @objc private func handleRouteChange(_ note: Notification) {
    guard let raw = note.userInfo?[AVAudioSessionRouteChangeReasonKey] as? UInt,
      let reason = AVAudioSession.RouteChangeReason(rawValue: raw) else { return }
    switch reason {
    case .newDeviceAvailable, .oldDeviceUnavailable:
      // A device switch (e.g. plugging into / out of AirPods) deactivates our
      // session and stops the engine, but — unlike a phone call — delivers an
      // interruption `.began` with NO matching `.ended`. So we can't wait passively:
      // re-pin the mic, then SELF-RECOVER with staggered re-arms (the session may
      // still be settling at the first attempt). ensureRecording is idempotent, so
      // once one succeeds the rest are no-ops; the location wake is the final
      // backstop. This turns a ~10s recovery (old: wait for the slow location poll)
      // into <1s.
      interruptionHoldUntil = nil
      q.async { [weak self] in try? self?.configureSession() }
      for delay in [0.7, 2.5] {
        DispatchQueue.main.asyncAfter(deadline: .now() + delay) { [weak self] in
          self?.ensureRecording(reason: "route-change")
        }
      }
    default: break
    }
  }

  @objc private func handleMediaReset() {
    // Full audio-subsystem reset — rebuild everything.
    NSLog("[Audio] media services reset — rebuilding")
    interruptionHoldUntil = nil
    virtues_location_audio_state(1)
    q.async { [weak self] in
      guard let self = self else { return }
      if self.tapInstalled { self.engine.inputNode.removeTap(onBus: 0); self.tapInstalled = false }
      self.converter = nil
      self.recording = false
    }
    DispatchQueue.main.asyncAfter(deadline: .now() + 0.3) { [weak self] in
      self?.ensureRecording(reason: "media-reset")
    }
  }

  @objc private func handleForeground() {
    interruptionHoldUntil = nil
    ensureRecording(reason: "foreground")
  }
}
