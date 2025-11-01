# Ariata Ontology Design

## Purpose

Normalized, queryable facts across sources for:

1. Deterministic event timeboxing (PELT/changepoint detection input)
2. Base elements for narrative event construction
3. Fact-based retrieval layer

**Structure**: Domain-based tables (`health_heart_rate`, `social_email`, etc.)
**Granularity**: Both infinitesimal signals (heart rate readings) AND discrete events (calendar meetings)
**Storage**: Postgres (relational, time-series optimized)

## Philosophy

- **Domain-Oriented**: 8 fundamental domains of human experience
- **Time is Orthogonal**: Time is not a domain - it's a universal field on all primitives (via `timestamp`, `start_time/end_time`)
- **Event-Oriented**: Optimized for narrative event synthesis and autobiography generation
- **Lossless Transformation**: No filtering during transformation - preserve all data
- **Naming Convention**: `domain_concept` pattern (e.g., `health_heart_rate`, `social_email`)
- **Temporal Types**: Primitives are either signals (infinitesimal) or temporal (bounded duration)

---

## 8 Ontological Domains

### 1. `health_*`

**Purpose**: Body signals and physiological states

**Signals (infinitesimal):**

- `health_heart_rate` - BPM measurements
- `health_hrv` - Heart rate variability
- `health_steps` - Step count increments
- `health_blood_oxygen` - SpO2 readings
- `health_blood_glucose` - Glucose levels
- `health_blood_pressure` - Systolic/diastolic readings
- `health_body_temperature` - Temperature readings
- `health_respiratory_rate` - Breaths per minute

**Temporal (bounded duration):**

- `health_sleep` - Sleep sessions with stages
- `health_workout` - Exercise sessions with activity type
- `health_meditation` - Meditation/mindfulness sessions
- `health_meal` - Eating events with nutrition data
- `health_symptom` - Illness/symptom occurrences
- `health_medication` - Medication intake events
- `health_mood` - Mood check-ins/assessments (?)

---

### 2. `location_*`

**Purpose**: Position and movement

**Signals (infinitesimal):**

- `location_point` - GPS coordinates (lat/lng/altitude)

**Temporal (bounded duration):**

- `location_visit` - Time spent at a place (clustered from points)
- `location_route` - Journey between places
- `location_place` - Named/identified locations (home, office, cafe) (?)

---

### 3. `social_*`

**Purpose**: Communication and relationships

**Temporal (bounded duration):**

- `social_email` ✅ **[IMPLEMENTED]**
- `social_interaction` - In-person meetings, calendar events with attendees
- `social_text_message` - SMS, iMessage
- `social_voice_call` - Phone calls
- `social_video_call` - Video conferencing
- `social_chat_message` - Slack, Discord, WhatsApp
- `social_post` - Social media posts
- `social_contact` - Contact/person record (?)

---

### 4. `activity_*`

**Purpose**: Time allocation, focus, planned commitments

**Temporal (bounded duration):**

- `activity_calendar_entry` - Scheduled calendar events (all events)
- `activity_app_usage` - App switches/usage sessions
- `activity_screen_time` - Device usage blocks
- `activity_web_browsing` - URL visits, page views
- `activity_focus_session` - Deep work blocks
- `activity_project_work` - Time on specific projects/tasks (?)

---

### 5. `finance_*`

**Purpose**: Transactions and resources

**Temporal (bounded duration):**

- `finance_transaction` - Purchases, payments, transfers
- `finance_subscription` - Recurring payments
- `finance_income` - Salary, payments received
- `finance_investment` - Stock trades, crypto transactions

**Signals (infinitesimal):**

- `finance_account_balance` - Account balance snapshots (?)

---

### 6. `ambient_*`

**Purpose**: External environmental conditions

**Signals (infinitesimal):**

- `ambient_weather` - Temperature, humidity, conditions
- `ambient_air_quality` - AQI, pollution levels
- `ambient_noise_level` - Decibel measurements
- `ambient_light_level` - Lux/brightness measurements

**Temporal (bounded duration):**

- `ambient_music` - What music was playing (Spotify, etc.) (?)
- `ambient_audio` - Environmental audio context (?)

---

### 7. `content_*`

**Purpose**: Information artifacts, media, knowledge objects

**Temporal (bounded duration):**

- `content_document` - Notion pages, Google Docs, files created/edited
- `content_bookmark` - Saved URLs, links
- `content_search` - Search queries (Google, ChatGPT prompts)
- `content_file` - Downloaded/saved files
- `content_book` - Books read/reading
- `content_article` - Articles read/saved (Pocket, Instapaper)
- `content_video` - Videos watched (YouTube)
- `content_podcast` - Podcast episodes listened to
- `content_music` - Songs/albums listened to (?)
- `content_annotation` - Highlights, notes, comments on content
- `content_course` - Online courses, tutorials

---

### 8. `introspection_*`

**Purpose**: Self-reflection, user-attested narratives

**Temporal (bounded duration):**

- `introspection_journal` - Journal entries (text, voice memos transcribed) (?)
- `introspection_mood` - Explicit mood logs (?)
- `introspection_goal` - Goals, intentions, aspirations
- `introspection_gratitude` - Gratitude logs
- `introspection_reflection` - Explicit reflections on events/experiences
- `introspection_dream` - Dream journals
- `introspection_value` - Axiom declarations (AD - Determined Axioms) (?)

---

## Source → Ontology Mappings

### Implemented ✅

**Google Gmail:**

- `stream_google_gmail` → `social_email` ✅

### Planned (Next Phase)

**Google Calendar:**

- `stream_google_calendar` → `activity_calendar_entry`
- `stream_google_calendar` → `social_interaction` (if attendees exist)

**iOS HealthKit:**

- `stream_ios_healthkit` → `health_heart_rate`
- `stream_ios_healthkit` → `health_hrv`
- `stream_ios_healthkit` → `health_sleep`
- `stream_ios_healthkit` → `health_workout`
- `stream_ios_healthkit` → `health_steps`
- `stream_ios_healthkit` → `health_body_composition`

**iOS Location:**

- `stream_ios_location` → `location_point`
- `stream_ios_location` → `location_visit`
- `stream_ios_location` → `location_route`

**iOS Microphone:**

- `stream_ios_microphone` → `ambient_noise_level`
- `stream_ios_microphone` → `content_annotation` (transcriptions)

**Mac iMessage:**

- `stream_mac_imessage` → `social_text_message`

**Mac Browser:**

- `stream_mac_browser` → `activity_web_browsing`

**Mac Apps:**

- `stream_mac_apps` → `activity_app_usage`

**Mac Screen Time:**

- `stream_mac_screentime` → `activity_screen_time`

**Notion Pages:**

- `stream_notion_pages` → `content_document`
- `stream_notion_pages` → `introspection_goal` (filtered by type)

---

## Design Principles

1. **Lossless**: Transform preserves all data from raw streams
2. **Traceable**: Every ontology record references its source stream record via `source_stream_id`
3. **Normalized**: Cross-source data (e.g., heart rate from multiple devices) shares same schema
4. **Semantic**: Table names are human-readable and self-documenting
5. **Event-first**: Optimized for temporal queries and narrative generation
6. **1:Many Transforms**: A single source stream can create records in multiple ontology tables

---

## Implementation Status

### Phase 1: Core Infrastructure ✅

**Completed:**

- ✅ Base transform trait (`sources/base/transform.rs`)
- ✅ Transform job executor (`jobs/transform_job.rs`)
- ✅ Job chaining (sync → transform)
- ✅ `social_email` ontology table and transform

**Architecture:**

- Transform logic lives alongside source streams (e.g., `sources/google/gmail/transform.rs`)
- Transforms run automatically after successful sync jobs
- All ontology tables include `source_stream_id`, `source_table`, `source_provider` for traceability
- Jobs table tracks both sync and transform operations independently
- **Multiple transform jobs can be created from a single sync** (1:Many pattern)

### Implemented Ontologies

#### `social_email` ✅

**Status:** Production-ready
**Source:** `stream_google_gmail`
**Transform:** `sources/google/gmail/transform.rs`
**Schema:** `elt.social_email`

**Fields:**

- Core: message_id, thread_id, subject, body_plain, body_html, snippet
- Participants: from_address, to_addresses, cc_addresses
- Metadata: direction (sent/received), labels, is_read, is_starred
- Threading: thread_position, thread_message_count
- Traceability: source_stream_id → `stream_google_gmail.id`

**Usage:**

```sql
-- Get all emails from a user's Gmail
SELECT * FROM elt.social_email
WHERE source_provider = 'google'
ORDER BY timestamp DESC;

-- Trace back to original raw data
SELECT e.*, g.raw_json
FROM elt.social_email e
JOIN elt.stream_google_gmail g ON e.source_stream_id = g.id
WHERE e.id = '...';
```

---

## Adding New Transforms

### Step 1: Create Migration

Create `core/migrations/XXX_<ontology>_table.sql`:

```sql
CREATE TABLE elt.<domain>_<concept> (
    id UUID PRIMARY KEY,
    timestamp TIMESTAMPTZ NOT NULL,
    -- domain-specific fields --
    source_stream_id UUID NOT NULL,
    source_table TEXT NOT NULL,
    source_provider TEXT NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);
```

### Step 2: Implement Transform

Create `core/src/sources/<provider>/<stream>/transform.rs`:

```rust
use crate::sources::base::{OntologyTransform, TransformResult};

pub struct MyTransform;

impl OntologyTransform for MyTransform {
    fn source_table(&self) -> &str { "stream_<source>" }
    fn target_table(&self) -> &str { "<domain>_<concept>" }
    fn domain(&self) -> &str { "<domain>" }

    async fn transform(&self, db: &Database, source_id: Uuid) -> Result<TransformResult> {
        // Query source, transform, write to ontology
    }
}
```

### Step 3: Register Transform (1:Many Support)

Add to `jobs/sync_job.rs` in `get_transform_configs()`:

```rust
fn get_transform_configs(stream_name: &str) -> Vec<(&str, &str, &str)> {
    match stream_name {
        "gmail" => vec![
            ("stream_google_gmail", "social_email", "social"),
        ],
        "calendar" => vec![
            ("stream_google_calendar", "activity_calendar_entry", "activity"),
            ("stream_google_calendar", "social_interaction", "social"),
        ],
        "healthkit" => vec![
            ("stream_ios_healthkit", "health_heart_rate", "health"),
            ("stream_ios_healthkit", "health_hrv", "health"),
            ("stream_ios_healthkit", "health_sleep", "health"),
            // ... more health primitives
        ],
        _ => vec![],
    }
}
```

### Step 4: Route Transform

Add to `jobs/transform_job.rs` in `execute_transform_job()`:

```rust
let transformer: Box<dyn OntologyTransform> = match (source_table, target_table) {
    ("stream_google_gmail", "social_email") => Box::new(GmailEmailTransform),
    ("stream_google_calendar", "activity_calendar_entry") => Box::new(CalendarActivityTransform),
    ("stream_google_calendar", "social_interaction") => Box::new(CalendarSocialTransform),
    _ => return Err(...),
};
```

---

## Traceability Pattern

All ontology records maintain bidirectional traceability:

**Forward (ontology → raw):**

```sql
SELECT * FROM elt.social_email WHERE source_stream_id = '<uuid>';
```

**Backward (raw → ontology):**

```sql
SELECT * FROM elt.stream_google_gmail g
JOIN elt.social_email e ON e.source_stream_id = g.id;
```

This enables:

- Debugging transformation issues
- Accessing raw provider-specific fields
- Auditing data lineage
- Reprocessing/backfilling ontologies

---

## Design Questions (To Be Resolved)

### Q1: Mood Placement

**Question:** Should mood be in `health_*` or `introspection_*`?

- `health_mood` - Physiological/psychological state (objective-ish, potentially HRV-derived)
- `introspection_mood` - Subjective self-assessment (user-logged)
- Both? (Different primitives for different sources)

### Q2: Music Placement

**Question:** Should music be in `ambient_*` or `content_*`?

- `ambient_music` - Background environmental audio (passive listening)
- `content_music` - Intentional listening/consumption (active engagement)
- Context-dependent? (Spotify explicit play vs background café music)

### Q3: Journal Entries

**Question:** Are journal entries primitives or Rich Signals?

- **Option A:** `introspection_journal` is a primitive table (stores the text)
- **Option B:** Journals are Rich Signals that get parsed into other primitives (no journal table in ontology layer)
- **Hybrid:** Store the journal text, but also extract primitives from it?

### Q4: Content Domain Scope

**Question:** Is `content_*` too broad?

- **Current:** Everything from docs to music to videos
- **Alternative:** Split into `media_*` (video, music, podcasts) and `knowledge_*` (docs, articles, books)
- **Reasoning:** Media consumption vs knowledge acquisition might be different dimensions

### Q5: Calendar Naming

**Question:** What should calendar events be called?

- `activity_calendar_entry`
- `activity_calendar_event`
- `activity_calendar`
- Keep as standalone `calendar_entry` (separate mini-domain)?

### Q6: Primitive Categorization

**Question:** Are there primitives that:

- Should be in a different domain?
- Shouldn't be primitives at all (synthesized at Life Events layer)?
- Are missing from the current list?

**Specific uncertainties marked with (?) in domain listings above:**

- `health_mood` - Domain placement?
- `location_place` - Primitive or reference table?
- `social_contact` - Primitive or reference table?
- `activity_project_work` - Too high-level for ontology layer?
- `finance_account_balance` - Signal or derived?
- `ambient_music` - Domain placement?
- `content_music` - Domain placement?
- `introspection_journal` - Primitive or Rich Signal?
- `introspection_mood` - Redundant with `health_mood`?
- `introspection_value` - Part of axiom system (not ontology layer)?

---

## Relationship to Life Events Layer

**Important:** This ontology layer is the **fact layer** only. The Life Events synthesis layer (Action → Event → Cycle → Arc → Chapter → Telos) is built **on top of** these primitives via:

- PELT changepoint detection
- LLM enrichment of Rich Signals
- Neo4j graph construction
- Causal pattern discovery

The ontology primitives serve as **input** to the Life Events construction, not the output.
