# Changepoint Detection

## Overview

Changepoint detection is the process of identifying temporal boundaries in continuous ontology data to segment a person's life into discrete, meaningful events. This system uses a **hybrid approach** combining per-ontology signal detection with LLM-based narrative smoothing.

---

## 1. Event Definition

### What is an Event?

An **event** is a temporally bounded period of coherent activity.

**Coherent activity** means:

- **Single primary context**: Stable location, social setting, or activity type
- **Unified intention or state**: Working, commuting, socializing, resting, exercising
- **Temporal continuity**: No unexplained data gaps >15 minutes within the event

### Temporal Constraints

```
Minimum event duration: 15 minutes
Maximum event duration: 4 hours
Target event duration: 30 minutes - 2 hours
Expected events per day: 12-20 events (for 16-hour waking day)
```

**Rationale:**

- **Minimum (15 min)**: Anything shorter is likely noise, transition, or not semantically meaningful
  - *Exception*: Explicit calendar events <15 min (e.g., "Quick sync - 10 min") are still valid events

- **Maximum (4 hours)**: Human attention/activity rarely stays truly coherent beyond 4 hours
  - If an "event" exceeds 4 hours, it should be split at natural boundaries or every 2 hours
  - *Exception*: Sleep is always treated as a single event (see Special Cases)

- **Target (30 min - 2 hours)**: Most meaningful human activities fall in this range
  - Meetings: 30 min - 1 hour
  - Deep work: 1-2 hours
  - Meals: 30-45 min
  - Commute: 15-45 min
  - Exercise: 30-90 min

### Expected Distribution

For a typical 16-hour waking day:

```
8 events  = 2 hours/event     (too coarse - misses important detail)
16 events = 1 hour/event      (ideal - balanced granularity) ✅
24 events = 40 min/event      (acceptable - high detail)
32+ events = <30 min/event    (too granular - noisy)

Target: 12-20 events per waking day (45-80 min average duration)
```

---

## 2. Event Salience (Quality Hierarchy)

Not all events are equally important. **Salience** (from cognitive science) refers to the subjective importance or memorability of an event.

### Salience Tiers

**Tier 1: High Salience Events** (Major life moments)

- Duration: 90+ minutes
- High-fidelity boundaries (calendar, location)
- Semantically rich (meetings, workouts, social gatherings, focused work)
- Examples:
  - Deep work session (2 hours)
  - Important client meeting (1.5 hours)
  - Dinner with family (1.5 hours)
  - Workout at gym (1 hour)

**Tier 2: Medium Salience Events** (Standard activities)

- Duration: 30-90 minutes
- Clear boundaries (calendar, location, activity change)
- Routine but meaningful
- Examples:
  - Quick standup meeting (30 min)
  - Lunch break (45 min)
  - Email processing session (30 min)
  - Grocery shopping (1 hour)

**Tier 3: Low Salience Events** (Micro-events / Transitions)

- Duration: 15-30 minutes
- May have fuzzy boundaries
- Functional but less memorable
- Examples:
  - Coffee break (15 min)
  - Commute segment (20 min)
  - Bathroom/stretch break (10 min)
  - Quick phone call (15 min)

### LLM Guidance on Salience

The LLM should:

- **Prefer creating Tier 1 and Tier 2 events** (these form the narrative backbone)
- **Only create Tier 3 events if semantically meaningful** (e.g., commute is important context even if only 20 min)
- **Merge or absorb low-salience micro-events** into adjacent higher-salience events when appropriate
- **Use salience to resolve ambiguous boundaries**: Extend boundaries toward higher-salience activities

---

## 3. Boundary Types

Each event has a **start boundary** and **end boundary**. Boundaries differ in their certainty and detection method.

### Boundary Type Taxonomy

```rust
enum BoundaryType {
    /// Explicit start signal (e.g., meeting began, entered location)
    Begin {
        signal: String,      // "calendar_entry_start", "location_visit_begin"
        confidence: f64,     // 0.0-1.0
    },

    /// Explicit end signal (e.g., meeting ended, left location)
    End {
        signal: String,      // "calendar_entry_end", "location_visit_end"
        confidence: f64,
    },

    /// Inferred transition (signal change suggests boundary, but not explicit)
    Transition {
        from_state: String,  // "coding"
        to_state: String,    // "slack_communication"
        confidence: f64,
    },

    /// Unknown gap (no data, but previous event clearly ended)
    UnknownGap {
        last_known_state: String,
        gap_duration: Duration,
    },
}
```

### Boundary Confidence Levels

```rust
enum BoundaryConfidence {
    /// Calendar event, location visit start/end (fidelity: 1.0)
    Explicit,

    /// Signal change (app switch, speech start/stop, HR spike) (fidelity: 0.5-0.8)
    Inferred,

    /// No clear signal, assumed based on time or fallback (fidelity: 0.3)
    Assumed,
}
```

### Example Scenarios

#### Scenario A: Both Boundaries Explicit ✅

```
9:00 AM: calendar_entry_start("Standup") → BEGIN (Explicit)
9:30 AM: calendar_entry_end("Standup") → END (Explicit)

Event: 9:00-9:30 AM "Daily standup meeting"
- Start confidence: Explicit ✅
- End confidence: Explicit ✅
- Duration: 30 min (certain)
```

#### Scenario B: Begin Explicit, End Inferred ⚠️

```
9:30 AM: app_usage_start("VSCode") → BEGIN (Explicit)
[... 2 hours of VSCode activity ...]
11:35 AM: app_usage_end("VSCode"), app_usage_start("Slack") → END (Inferred)

Event: 9:30-11:35 AM "Deep work coding session"
- Start confidence: Explicit ✅
- End confidence: Inferred ⚠️
- Duration: ~2 hours (likely accurate, but end boundary fuzzy)

Narrative: "After standup, you dove into a deep work session in VSCode that lasted about 2 hours. Around 11:35, you switched to Slack."
```

#### Scenario C: Begin Explicit, End Unknown ❓

```
9:30 AM: app_usage_start("VSCode") → BEGIN (Explicit)
[... VSCode activity ...]
10:45 AM: last_ontology(app_usage "VSCode") → Last known signal
[... 30 minutes NO DATA ...]
11:15 AM: location_visit("Cafe") → BEGIN (Explicit, new event)

Event 1: 9:30-10:45+ AM "Deep work coding"
- Start confidence: Explicit ✅
- End confidence: Unknown ❓
- Duration: AT LEAST 1h 15min (could have continued during gap)

Gap: 10:45-11:15 AM (30 min unknown)

Event 2: 11:15 AM-... "Working at cafe"
- Start confidence: Explicit ✅

Narrative: "You worked in VSCode from 9:30 until at least 10:45 AM. After a gap in tracking, you arrived at a cafe around 11:15 AM."
```

#### Scenario D: No Clear Boundaries (Smooth Transition) 🌫️

```
10:00 AM: app_usage("Slack")
10:15 AM: app_usage("Chrome")
10:30 AM: app_usage("VSCode")
10:45 AM: app_usage("Slack")

No location change, no calendar, no major signal changes.

LLM decision: Don't create 4 separate events for app switches.
Instead: Single event with fuzzy boundaries.

Event: 10:00-11:00 AM "Mixed work activities"
- Start confidence: Assumed
- End confidence: Assumed
- Duration: ~1 hour (fuzzy boundaries)

Narrative: "During the late morning, you bounced between Slack conversations, web browsing, and coding—typical multitasking mode."
```

---

## 4. Detection Architecture

### Hybrid Approach

The system uses a **two-phase hybrid approach**:

1. **Phase 1: Per-Ontology Boundary Detection** (Mechanical)
   - Each ontology runs its own detection logic
   - Outputs: Boundary candidates with confidence scores

2. **Phase 2: LLM Smoothing & Narrative Generation** (Intelligent)
   - LLM reviews all boundary candidates + raw data
   - Accepts, rejects, merges, or adjusts boundaries
   - Generates coherent narrative for each event

---

## 5. Per-Ontology Detection Methods

### Type A: Binary/Explicit Boundaries (No Processing)

These ontologies have **explicit start/end timestamps** — they ARE the boundaries.

| Ontology | Fidelity | Boundary Signal | Detection Method |
|----------|----------|-----------------|------------------|
| `calendar_entry` | 1.0 | Event start/end times | Extract timestamps directly |
| `location_visit` | 0.95 | Visit start/end (derived from GPS clustering) | Extract timestamps directly |
| `activity_app_usage` | 0.75 | App open/close events | Extract timestamps directly |
| `speech_audio_classification` | 0.70 | Speech start/stop (VAD) | State change detection |

**Example: calendar_entry**

```rust
fn detect_boundaries(calendar_events: Vec<CalendarEntry>) -> Vec<BoundaryCandidate> {
    let mut boundaries = vec![];
    for event in calendar_events {
        boundaries.push(BoundaryCandidate {
            timestamp: event.start_time,
            boundary_type: BoundaryType::Begin,
            signal: "calendar_entry_start",
            fidelity: 1.0,
        });
        boundaries.push(BoundaryCandidate {
            timestamp: event.end_time,
            boundary_type: BoundaryType::End,
            signal: "calendar_entry_end",
            fidelity: 1.0,
        });
    }
    boundaries
}
```

---

### Type B: Continuous Signals (PELT or Threshold Detection)

These ontologies are **continuous streams** requiring analysis to find "when did something change?"

| Ontology | Fidelity | Boundary Signal | Detection Method |
|----------|----------|-----------------|------------------|
| `health_heart_rate` | 0.50 | Sudden HR change (>20 bpm) | PELT or threshold detection |
| `health_hrv` | 0.45 | HRV shift (stress/recovery) | PELT |
| `location_point` | 0.60 | Significant location change (>500m) | Threshold detection |
| `ambient_noise_level` | 0.40 | Noise level change (quiet→loud) | Threshold detection |

**Example: health_heart_rate (Threshold Detection)**

```rust
fn detect_heart_rate_boundaries(hr_data: Vec<HeartRateSample>) -> Vec<BoundaryCandidate> {
    let mut boundaries = vec![];
    let window_size = Duration::minutes(5);

    for i in 1..hr_data.len() {
        let prev_window = avg_hr(&hr_data[i-5..i]);
        let curr_window = avg_hr(&hr_data[i..i+5]);

        // Threshold: >20 bpm change
        if (curr_window - prev_window).abs() > 20.0 {
            boundaries.push(BoundaryCandidate {
                timestamp: hr_data[i].timestamp,
                boundary_type: BoundaryType::Transition,
                signal: format!("heart_rate_change_{}bpm", (curr_window - prev_window) as i32),
                fidelity: 0.50,
            });
        }
    }

    boundaries
}
```

**Example: health_heart_rate (PELT)**

```python
# Pseudocode using ruptures library
import ruptures as rpt

def detect_hr_changepoints(hr_timeseries):
    # PELT (Pruned Exact Linear Time) algorithm
    algo = rpt.Pelt(model="rbf").fit(hr_timeseries)
    changepoints = algo.predict(pen=10)  # Penalty parameter (tune this)

    return [
        BoundaryCandidate(
            timestamp=timestamps[cp],
            boundary_type=BoundaryType.Transition,
            signal="heart_rate_pelt_changepoint",
            fidelity=0.50
        )
        for cp in changepoints
    ]
```

---

## 6. Ontology Fidelity Weights

Fidelity represents **how reliable an ontology is for detecting event boundaries**.

### Fidelity Scale (0.0 - 1.0)

```
1.0 = Perfect (explicit, ground truth)
0.8-0.9 = Very High (derived but reliable)
0.6-0.7 = High (strong signal for boundaries)
0.4-0.5 = Medium (useful but noisy)
0.2-0.3 = Low (weak signal, context only)
```

### Fidelity by Ontology Type

| Fidelity | Ontologies | Rationale |
|----------|-----------|-----------|
| **1.0** | `calendar_entry` | Explicit timestamps, human-curated |
| **0.95** | `location_visit` | Derived from GPS clustering, very reliable |
| **0.75** | `activity_app_usage` | App open/close is explicit, but may miss context |
| **0.70** | `speech_audio_classification` | VAD is reliable, but speech ≠ always an event boundary |
| **0.60** | `location_point` | Raw GPS, can be noisy |
| **0.50** | `health_heart_rate` | Physiological, but influenced by many factors |
| **0.45** | `health_hrv` | Similar to HR, but more subtle |
| **0.40** | `ambient_noise_level` | Environmental, very noisy |
| **0.30** | `social_email` | Async communication, weak temporal signal |
| **0.25** | `activity_web_browsing` | Tab switches are frequent, low signal |

---

## 7. Aggregation & Smoothing

### Phase 3: Boundary Aggregation

After per-ontology detection, the system has a list of **boundary candidates**. These need to be aggregated and smoothed.

#### Step 1: Sort by Timestamp

```
All boundary candidates (unsorted):
- 9:00 AM (calendar_entry_start, fidelity: 1.0)
- 9:05 AM (speech_detected, fidelity: 0.7)
- 9:30 AM (calendar_entry_end, fidelity: 1.0)
- 9:32 AM (speech_ended, fidelity: 0.7)
- 8:45 AM (location_visit_start, fidelity: 0.95)
- 8:30 AM (heart_rate_spike, fidelity: 0.5)
```

#### Step 2: Cluster Nearby Boundaries

```
Temporal clustering window: 5 minutes

Clusters:
- [8:30 AM (HR)] - singleton
- [8:45 AM (location)] - singleton
- [9:00 AM (calendar), 9:05 AM (speech)] - cluster (5 min apart)
- [9:30 AM (calendar), 9:32 AM (speech)] - cluster (2 min apart)
```

#### Step 3: Weight by Fidelity

```
Cluster: [9:00 AM (calendar, fidelity 1.0), 9:05 AM (speech, fidelity 0.7)]

Weighted average:
timestamp = (9:00 * 1.0 + 9:05 * 0.7) / (1.0 + 0.7) = 9:02 AM

But since calendar has fidelity 1.0 (ground truth), use 9:00 AM.
```

#### Step 4: Output Smoothed Boundaries

```
Final boundaries:
- 8:30 AM (heart_rate_spike, confidence: medium)
- 8:45 AM (location_change, confidence: high)
- 9:00 AM (calendar_start + speech_detected, confidence: very_high)
- 9:30 AM (calendar_end + speech_ended, confidence: very_high)
```

---

## 8. Gap Handling

Gaps in data are inevitable. The system must decide how to interpret them.

### Gap Duration Rules

| Gap Duration | Interpretation | Action |
|--------------|----------------|--------|
| **<10 minutes** | Same event (brief data loss or sensor gap) | Extend event boundary across gap |
| **10-30 minutes** | Ambiguous (could be transition or brief activity) | Create "transition" event OR extend boundary with note |
| **>30 minutes** | Separate events (clear unknown period) | Create explicit "unknown period" in narrative |

### Example: Short Gap (<10 min)

```
9:00-9:25 AM: Speech detected (meeting)
9:25-9:32 AM: NO DATA (7 min gap)
9:32-9:45 AM: Speech detected (same meeting continued)

Decision: Merge into single event
Event: 9:00-9:45 AM "Team meeting"
Narrative: "You participated in a team meeting from 9:00-9:45 AM."
```

### Example: Medium Gap (10-30 min)

```
10:00-10:30 AM: App usage (VSCode)
10:30-10:50 AM: NO DATA (20 min gap)
10:50-11:30 AM: App usage (VSCode)

Decision: Options
A) Merge: "10:00-11:30 AM Deep work (with brief break)"
B) Split: "10:00-10:30 AM Work", "10:50-11:30 AM Work (resumed)"

LLM chooses based on context (A if no other signals, B if location changed)
```

### Example: Long Gap (>30 min)

```
2:00-3:00 PM: Location (Office)
3:00-4:15 PM: NO DATA (75 min gap)
4:15-5:00 PM: Location (Home)

Decision: Explicit unknown period
Events:
- 2:00-3:00 PM "Working at office"
- 3:00-4:15 PM "Unknown (no data)"
- 4:15-5:00 PM "Arrived home"

Narrative: "You were working at the office until 3:00 PM. After a gap in tracking, you were back home by 4:15 PM."
```

---

## 9. LLM Context & Prompting

### Context Provided to LLM

For each 2-hour processing window, the LLM receives:

```rust
struct LLMContext {
    // Current window being processed
    current_window_start: DateTime,
    current_window_end: DateTime,
    proposed_boundaries: Vec<BoundaryCandidate>,
    ontology_data: HashMap<String, Vec<Ontology>>,

    // Context from earlier today
    prior_events_today: Vec<CompletedEvent>,
    last_known_state: PersonState,

    // Temporal metadata
    day_start_time: DateTime,
    time_since_wake: Duration,

    // Special states
    is_sleep_period: bool,  // 10 PM - 7 AM typically
}

struct CompletedEvent {
    time_start: DateTime,
    time_end: DateTime,
    narrative: String,
    primary_context: String,  // "office", "home", "commute", etc.
    salience: u8,  // 1=high, 2=medium, 3=low
}
```

### LLM Prompt Template

```
System: You are analyzing life event boundaries for narrative generation.

Context - Events so far today (7:00 AM - 10:00 AM):
1. 7:00-8:00 AM: "Morning routine at home" (salience: medium)
   - Location: Home
   - Activity: Rest → Light activity

2. 8:00-8:30 AM: "Commute to office" (salience: low)
   - Location: Home → Office (in transit)

3. 8:30-9:00 AM: "Arrived at office, prepared for standup" (salience: low)
   - Location: Office

4. 9:00-9:30 AM: "Daily standup meeting" (salience: high)
   - Location: Office
   - Ended with: Meeting ended (calendar + speech stopped)

Current window (10:00 AM - 12:00 PM):
- Proposed boundaries:
  - 10:15 AM (email received, fidelity: 0.3)
  - 11:30 AM (app switch Slack→VSCode, fidelity: 0.75)

- Ontology data:
  - location_visit: Office (continuous)
  - activity_app_usage: VSCode (9:30-11:30), Slack (11:30-12:00)
  - social_email: Email from Sarah (10:15)
  - health_heart_rate: Stable 72-78 bpm

- Last known state: At office, coding in VSCode since 9:30 AM

Guidelines:
- Event duration: 15 min - 4 hours (target: 30 min - 2 hours)
- Prefer high-salience events (meetings, deep work, focused activities)
- Merge low-salience micro-events unless semantically important
- Use "salience" field: 1=high, 2=medium, 3=low
- Special case: Sleep is always 1 event (unless broken sleep with wake periods)

Questions:
1. Should 10:15 AM email create a boundary? (Low fidelity, in middle of work session)
2. Should 11:30 AM app switch be a boundary? (Higher fidelity, clear context change)
3. What is the narrative for 9:30 AM - 12:00 PM?

Respond in JSON:
{
  "accepted_boundaries": [
    {"timestamp": "11:30:00", "reasoning": "Clear app switch from coding to communication"}
  ],
  "rejected_boundaries": [
    {"timestamp": "10:15:00", "reasoning": "Email is async, doesn't interrupt deep work"}
  ],
  "events": [
    {
      "start": "09:30:00",
      "end": "11:30:00",
      "narrative": "After standup, you entered a 2-hour deep work session...",
      "salience": 1,
      "primary_context": "office_deep_work"
    },
    {
      "start": "11:30:00",
      "end": "12:00:00",
      "narrative": "You switched to Slack to catch up on messages...",
      "salience": 2,
      "primary_context": "office_communication"
    }
  ]
}
```

---

## 10. Special Cases

### Sleep

**Rule:** Sleep is always treated as a **single event**, regardless of duration.

**Exception:** Broken sleep (wake periods >15 minutes) creates separate events.

```
Example 1: Normal sleep
11:00 PM - 7:00 AM: "Slept at home" (8 hours, 1 event)

Example 2: Broken sleep
11:00 PM - 2:30 AM: "First sleep period" (3.5 hours)
2:30 AM - 3:15 AM: "Woke up, browsed phone" (45 min - separate event)
3:15 AM - 7:00 AM: "Returned to sleep" (3.75 hours)
```

**Detection:** Sleep periods typically have:

- `location_visit` = home
- No app usage, no speech
- Low heart rate (resting)
- Time: 10 PM - 8 AM

---

### Transitions vs Events

**Transition:** Movement between two events (commute, walking between rooms)

**Event:** Destination activity with duration

```
Example:
8:00-8:30 AM: Commute (transition, low salience)
8:30 AM-12:00 PM: Work at office (event, high salience)

Narrative preference: "After commuting to the office, you worked until noon."
(Transition absorbed into narrative flow, not standalone event unless significant)
```

---

### Unknown Periods

When data gaps >30 minutes exist, explicitly acknowledge uncertainty.

```
Narrative phrasing:
- "After a gap in tracking, you..."
- "You worked until at least 10:45 AM. Later, you were at..."
- "Between 3-4 PM, there's no tracking data. By 4:15 PM, you were..."
```

---

## 11. Smoothing Rules Summary

```
Rule 1: Events <15 min → Merge with adjacent event (unless explicit calendar)
Rule 2: Events >4 hours → Split at natural boundary or every 2 hours
Rule 3: Boundaries <5 min apart → Merge into single boundary
Rule 4: Gap <10 min → Extend event across gap
Rule 5: Gap 10-30 min → Create transition OR extend with note
Rule 6: Gap >30 min → Explicit unknown period in narrative
Rule 7: Sleep → Always single event (unless broken sleep)
Rule 8: Low-salience events → Merge into adjacent high-salience events when appropriate
```

---

## 12. Implementation Phases

### Week 1: Simplified MVP

**Goal:** Prove the concept with minimal complexity.

**Ontologies used:**

- `calendar_entry` (fidelity: 1.0)
- `location_visit` (fidelity: 0.95)

**Detection method:**

```rust
fn detect_boundaries_mvp(ontologies: Vec<Ontology>) -> Vec<EventBoundary> {
    let mut boundaries = vec![];

    // Strategy 1: Calendar events (ground truth)
    for cal in ontologies.iter().filter(|o| o.is_calendar_entry()) {
        boundaries.push(EventBoundary::from_calendar(cal));
    }

    // Strategy 2: Location visits
    for visit in ontologies.iter().filter(|o| o.is_location_visit()) {
        boundaries.push(EventBoundary::from_location(visit));
    }

    // Strategy 3: Fallback to 2-hour fixed windows if no boundaries
    if boundaries.len() < 2 {
        boundaries = fixed_2hour_windows(&ontologies);
    }

    // Smooth nearby boundaries (<5 min apart)
    smooth_boundaries(boundaries, Duration::minutes(5))
}
```

**LLM prompt:** Simplified (no prior context, just current window)

---

### Week 2-3: Full System

**Ontologies added:**

- `activity_app_usage` (fidelity: 0.75)
- `speech_audio_classification` (fidelity: 0.70)
- `health_heart_rate` (fidelity: 0.50)
- `social_email` (fidelity: 0.30)

**Detection methods:**

- Implement per-ontology trait system
- Add PELT for continuous signals
- Add fidelity weighting
- Add prior event context to LLM

**LLM prompt:** Full context (prior events, last known state, temporal metadata)

---

### Post-MVP: Advanced Features

- Community detection for recurring patterns (e.g., "Tuesday standup")
- Semantic similarity for event clustering (e.g., "all coding sessions")
- User feedback loop (correct boundaries manually → retrain weights)
- Predictive boundaries (e.g., "meeting about to start")

---

## 13. References

### Academic Background

- **PELT Algorithm**: Killick, R., Fearnhead, P., & Eckley, I. A. (2012). Optimal detection of changepoints with a linear computational cost. *Journal of the American Statistical Association*.
- **Activity Segmentation**: Ward, J. A., Lukowicz, P., & Gellersen, H. W. (2011). Performance metrics for activity recognition. *ACM Transactions on Intelligent Systems and Technology*.
- **Salience in Memory**: Kahneman, D., & Tversky, A. (1979). Prospect theory: An analysis of decision under risk. *Econometrica*.

### Implementation Libraries

- **Rust**: `ruptures` (Python, call via PyO3), `changepoint` (Rust crate)
- **Statistical tests**: `statrs` (Rust), `scipy.stats` (Python)

---

## 14. Example: Full Day Walkthrough

See [ARCHITECTURE.md](ARCHITECTURE.md) for the complete data pipeline architecture.

**Input:** All ontologies from 7 AM - 11 PM

**Output:** 16 events with narratives

```
1. 7:00-8:00 AM: Morning routine at home (salience: medium)
2. 8:00-8:30 AM: Commute to office (salience: low)
3. 8:30-9:00 AM: Settled at desk (salience: low)
4. 9:00-9:30 AM: Daily standup meeting (salience: high)
5. 9:30-11:30 AM: Deep work coding session (salience: high)
6. 11:30 AM-12:00 PM: Slack communication (salience: medium)
7. 12:00-1:00 PM: Lunch at cafe (salience: medium)
8. 1:00-1:30 PM: Return to office (salience: low)
9. 1:30-3:00 PM: Client meeting (salience: high)
10. 3:00-3:15 PM: Coffee break (salience: low)
11. 3:15-5:00 PM: Email processing and planning (salience: medium)
12. 5:00-5:30 PM: Commute home (salience: low)
13. 5:30-6:30 PM: Exercise at gym (salience: high)
14. 6:30-7:30 PM: Dinner with family (salience: high)
15. 7:30-10:00 PM: Leisure time at home (salience: medium)
16. 10:00 PM-7:00 AM: Sleep (salience: special)

Total: 16 events (average 1 hour each)
High salience: 6 events
Medium salience: 5 events
Low salience: 5 events
```

---

**Next Steps:** See implementation in `core/ontologies/changepoints/` for Rust code.
