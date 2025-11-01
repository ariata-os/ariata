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

## Architecture Overview

The Ontology Layer contains two types of primitives:

1. **Domain Primitives**: Normalized facts about events/signals (e.g., `social_email`, `health_heart_rate`)
2. **Entity Primitives**: Canonical identities that appear across your life (e.g., `entities_person`, `entities_place`)

Domain primitives reference entity primitives via foreign keys, enabling queries like "all emails with Sarah" or "all activities at the gym."

---

## Entity Primitives

Entity primitives resolve and canonicalize identities across all sources. Unlike domain primitives (which are time-bound facts), entities persist and accumulate relationships over time.

### Entity Types

#### 1. **Personal Entities** (Full Normalization)

Entities you **directly interact with** in your life:

- **People you know**: Colleagues, friends, family, service providers
- **Places you visit**: Home, office, gym, favorite restaurants
- **Your topics**: Projects you work on, skills you're learning, personal interests

These get full entity resolution with temporal tracking, relationship dynamics, and rich metadata.

#### 2. **Reference Entities** (Lightweight Tags)

Entities you **consume or think about** but don't directly interact with:

- **Public figures**: Musicians, athletes, authors, historical figures
- **Media**: Books, podcasts, movies, articles
- **Concepts**: Philosophies, frameworks, academic subjects

These get simple tags with mention counts, not full relationship tracking.

### Entity Tables

#### `entities_person`

**Purpose**: Canonical identities of people in your life

**Schema**:

```sql
CREATE TABLE elt.entities_person (
    id UUID PRIMARY KEY,
    canonical_name TEXT NOT NULL,
    email_addresses TEXT[] NOT NULL DEFAULT '{}',
    phone_numbers TEXT[] DEFAULT '{}',
    display_names TEXT[] DEFAULT '{}', -- All seen name variations
    relationship_category TEXT, -- colleague, friend, family, service_provider
    first_interaction TIMESTAMPTZ NOT NULL,
    last_interaction TIMESTAMPTZ NOT NULL,
    interaction_count INTEGER DEFAULT 0,
    metadata JSONB, -- Flexible storage for future attributes
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_entities_person_email ON elt.entities_person USING GIN(email_addresses);
CREATE INDEX idx_entities_person_phone ON elt.entities_person USING GIN(phone_numbers);
```

**Resolution Strategy** (Formulaic):

1. **Exact email match**: `adam@gmail.com` appears in 50 emails → create/link entity
2. **Exact phone match**: `+15551234567` (normalized format) → link to existing or create
3. **Fuzzy name match** (Phase 2): Jaro-Winkler similarity > 0.9 + same company domain → merge
4. **Nickname dictionary** (Phase 2): Bob ↔ Robert, Liz ↔ Elizabeth

**Example**:

- Email 1: `from: "Sarah Jones" <sarah.jones@company.com>`
- Email 2: `from: "Sarah J." <sarah.jones@company.com>`
- Calendar: `attendee: sarah.jones@company.com`
- Result: 1 entity with `canonical_name: "Sarah Jones"`, `display_names: ["Sarah Jones", "Sarah J."]`

---

#### `entities_place`

**Purpose**: Canonical locations you physically visit

**Schema**:

```sql
CREATE TABLE elt.entities_place (
    id UUID PRIMARY KEY,
    canonical_name TEXT, -- "Home", "Office", "Equinox Gym - Mission"
    category TEXT, -- home, work, gym, restaurant, park, etc.
    geo_center GEOGRAPHY(POINT) NOT NULL, -- Centroid of visit cluster
    bounding_box GEOGRAPHY(POLYGON), -- Convex hull of all visit points
    cluster_radius_meters FLOAT, -- Max distance from center
    visit_count INTEGER DEFAULT 0,
    total_time_minutes INTEGER DEFAULT 0, -- Aggregate time spent
    first_visited TIMESTAMPTZ NOT NULL,
    last_visited TIMESTAMPTZ NOT NULL,
    metadata JSONB, -- Address, business info, etc.
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_entities_place_geocenter ON elt.entities_place USING GIST(geo_center);
CREATE INDEX idx_entities_place_category ON elt.entities_place(category);
```

**Resolution Strategy** (Formulaic):

1. **Geo-clustering**: DBSCAN with `eps=50m` for urban, `eps=200m` for suburban
2. **Cluster naming** (Phase 2): Most frequent location visit → lookup via reverse geocoding API
3. **Category inference** (Phase 2): Time-of-day patterns (9am-5pm weekdays = "work")

**Example**:

- Location points: `[(37.7749, -122.4194), (37.7750, -122.4195), ...]` (500 visits)
- Geo-clustering: All within 30m → single cluster
- Result: 1 entity with `canonical_name: "Equinox Gym - Mission"`, `category: "gym"`, `visit_count: 500`

---

#### `entities_topic`

**Purpose**: Projects, interests, and themes in your life

**Schema**:

```sql
CREATE TABLE elt.entities_topic (
    id UUID PRIMARY KEY,
    name TEXT NOT NULL,
    category TEXT, -- project, skill, interest, goal
    keywords TEXT[], -- Related terms for matching
    first_mentioned TIMESTAMPTZ NOT NULL,
    last_mentioned TIMESTAMPTZ NOT NULL,
    mention_count INTEGER DEFAULT 0,
    sources JSONB, -- Which primitives reference this topic
    metadata JSONB,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_entities_topic_name ON elt.entities_topic(name);
CREATE INDEX idx_entities_topic_keywords ON elt.entities_topic USING GIN(keywords);
```

**Resolution Strategy** (Formulaic):

1. **Exact keyword match**: "fundraising deck" appears in calendar, emails, docs → same topic
2. **TF-IDF similarity** (Phase 2): "Q4 fundraising" and "fundraising deck" → merge
3. **LLM extraction** (Phase 3): Extract topics from journal entries, transcriptions

**Example**:

- Calendar event: "Fundraising deck review"
- Email subject: "Re: Q4 fundraising materials"
- Journal entry: "Working on the fundraising presentation"
- Result: 1 entity with `name: "Q4 Fundraising"`, `keywords: ["fundraising", "deck", "Q4", "presentation"]`

---

#### `reference_entities`

**Purpose**: Lightweight tags for public figures, media, and concepts you don't directly interact with

**Schema**:

```sql
CREATE TABLE elt.reference_entities (
    id UUID PRIMARY KEY,
    entity_type TEXT NOT NULL, -- 'public_figure', 'media', 'concept'
    canonical_name TEXT NOT NULL,
    external_id TEXT, -- Spotify artist ID, Wikipedia page ID, ISBN, etc.
    mention_count INTEGER DEFAULT 0,
    first_mentioned TIMESTAMPTZ,
    last_mentioned TIMESTAMPTZ,
    metadata JSONB, -- Flexible storage (genre, author, etc.)
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_reference_entities_type ON elt.reference_entities(entity_type);
CREATE INDEX idx_reference_entities_name ON elt.reference_entities(canonical_name);
```

**Example**:

- Transcription: "I've been reading Marcus Aurelius's Meditations"
- Podcast: "Listened to Tim Ferriss interview"
- Music: "Taylor Swift's new album"
- Result: 3 reference entities (`type: 'public_figure'` for Marcus Aurelius, Tim Ferriss, Taylor Swift)

---

#### `primitive_reference_links`

**Purpose**: Many-to-many links between domain primitives and reference entities

**Schema**:

```sql
CREATE TABLE elt.primitive_reference_links (
    primitive_table TEXT NOT NULL, -- 'content_transcription', 'introspection_journal'
    primitive_id UUID NOT NULL,
    reference_entity_id UUID REFERENCES elt.reference_entities(id),
    context TEXT, -- How they were mentioned (optional snippet)
    created_at TIMESTAMPTZ DEFAULT NOW(),
    PRIMARY KEY (primitive_table, primitive_id, reference_entity_id)
);

CREATE INDEX idx_primitive_ref_links_entity ON elt.primitive_reference_links(reference_entity_id);
```

**Usage**:

```sql
-- Find all transcriptions where you mentioned Marcus Aurelius
SELECT t.*
FROM elt.content_transcription t
JOIN elt.primitive_reference_links l ON l.primitive_id = t.id AND l.primitive_table = 'content_transcription'
JOIN elt.reference_entities r ON r.id = l.reference_entity_id
WHERE r.canonical_name = 'Marcus Aurelius';
```

---

#### `entity_merge_log`

**Purpose**: Audit trail for entity merging decisions (human or AI)

**Schema**:

```sql
CREATE TABLE elt.entity_merge_log (
    id UUID PRIMARY KEY,
    entity_type TEXT NOT NULL, -- 'person', 'place', 'topic'
    source_entity_ids UUID[] NOT NULL, -- Entities being merged
    target_entity_id UUID NOT NULL, -- Resulting canonical entity
    merge_method TEXT NOT NULL, -- 'exact_email_match', 'fuzzy_name_match', 'manual_user_merge', 'llm_merge'
    confidence_score FLOAT, -- 0.0-1.0 for algorithmic merges
    merge_reason TEXT, -- Human-readable explanation
    matching_features JSONB, -- Which fields matched (email, phone, geo, etc.)
    approved_by TEXT, -- user_id if manual, 'system' if automatic
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_entity_merge_source ON elt.entity_merge_log USING GIN(source_entity_ids);
CREATE INDEX idx_entity_merge_target ON elt.entity_merge_log(target_entity_id);
```

**Example**:

```json
{
  "entity_type": "person",
  "source_entity_ids": ["e1-uuid", "e2-uuid"],
  "target_entity_id": "e_master-uuid",
  "merge_method": "exact_email_match",
  "confidence_score": 1.0,
  "merge_reason": "Same email address (adam@gmail.com) found in both entities",
  "matching_features": {"email": "exact", "name": "similar_0.85"},
  "approved_by": "system"
}
```

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
- `health_mood` - Mood assessments with `measurement_method` field (self_reported, hrv_derived, etc.)

---

### 2. `location_*`

**Purpose**: Position and movement

**Signals (infinitesimal):**

- `location_point` - GPS coordinates (lat/lng/altitude)

**Temporal (bounded duration):**

- `location_visit` - Time spent at a place (references `entities_place`)

**Note**: `location_route` is derived (computed from sequential location_visit records, not stored as primitive). `location_place` is now `entities_place` (see Entity Primitives section above).

---

### 3. `social_*`

**Purpose**: Communication and relationships

**Temporal (bounded duration):**

- `social_email` ✅ **[IMPLEMENTED]** (references `entities_person` via `from_person_id`, `to_person_ids`)
- `social_interaction` - In-person meetings, calendar events with attendees (references `entities_person`)
- `social_text_message` - SMS, iMessage (references `entities_person`)
- `social_voice_call` - Phone calls (references `entities_person`)
- `social_video_call` - Video conferencing (references `entities_person`)
- `social_chat_message` - Slack, Discord, WhatsApp (references `entities_person`)
- `social_post` - Social media posts

**Note**: `social_contact` is now `entities_person` (see Entity Primitives section above).

---

### 4. `activity_*`

**Purpose**: Time allocation, focus, planned commitments

**Temporal (bounded duration):**

- `activity_calendar_entry` - Scheduled calendar events (references `entities_person` for attendees, `entities_topic` for project-related events)
- `activity_app_usage` - App switches/usage sessions
- `activity_screen_time` - Device usage blocks
- `activity_web_browsing` - URL visits, page views
- `activity_focus_session` - Deep work blocks

**Note**: `activity_project_work` is derived (inferred from calendar events + app usage + topics, not stored as primitive).

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

**Note**: Music/audio moved to `content_*` domain. See `content_audio` below.

---

### 7. `content_*`

**Purpose**: Information artifacts, media, knowledge objects

**Temporal (bounded duration):**

- `content_document` - Notion pages, Google Docs, files created/edited (may reference `entities_topic`)
- `content_bookmark` - Saved URLs, links
- `content_search` - Search queries (Google, ChatGPT prompts)
- `content_file` - Downloaded/saved files
- `content_book` - Books read/reading (may link to `reference_entities` for authors)
- `content_article` - Articles read/saved (Pocket, Instapaper)
- `content_video` - Videos watched (YouTube)
- `content_audio` - Music, podcasts, audiobooks with `audio_type` field (music/podcast/audiobook) and `listening_context` (active/background)
- `content_transcription` - Transcribed audio (from microphone, voice memos) - intermediate primitive that spawns other primitives
- `content_annotation` - Highlights, notes, comments on content
- `content_course` - Online courses, tutorials

**Note**: Consolidated music and podcasts into `content_audio`. `content_transcription` is a special intermediate primitive (see Multi-Stage Transforms section).

---

### 8. `introspection_*`

**Purpose**: Self-reflection, user-attested narratives

**Temporal (bounded duration):**

- `introspection_journal` - Journal entries (text-based, or extracted from `content_transcription`)
- `introspection_goal` - Goals, intentions, aspirations (may reference `entities_topic`)
- `introspection_gratitude` - Gratitude logs
- `introspection_reflection` - Explicit reflections on events/experiences (may link to `reference_entities` for concepts discussed)
- `introspection_dream` - Dream journals

**Note**:

- `introspection_mood` removed (redundant with `health_mood`)
- `introspection_value` (Axiom declarations) belongs to Life Events Layer, not Ontology Layer
- `introspection_journal` can be created from `content_transcription` via multi-stage transform

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

## Multi-Stage Transforms

Some streams require **chained transformations** where one primitive creates additional primitives. This is especially common for AI-powered transformations.

### Pattern: Intermediate Primitives

**Definition**: A primitive that serves as both:

1. A queryable fact in the ontology layer
2. A source for downstream transformations

**Example: Audio → Transcription → Structured Primitives**

```
stream_ios_microphone (audio file)
    ↓ [Transform 1: Whisper API]
content_transcription (text primitive)
    ↓ [Transform 2: LLM structuring]
Multiple primitives (introspection_journal, social_interaction, entities_*)
```

### Implementation Pattern

When a transform creates an intermediate primitive, it spawns additional transform jobs:

```rust
// Transform 1: Audio → Transcription
impl OntologyTransform for MicrophoneTranscriptionTransform {
    async fn transform(&self, db: &Database, source_id: Uuid) -> Result<TransformResult> {
        // 1. Fetch audio from stream_ios_microphone
        // 2. Call Whisper API
        // 3. Insert into content_transcription
        let transcript_id = insert_transcription(...).await?;

        // 4. Create follow-up transform job
        create_chained_transform_job(
            db,
            "content_transcription",
            "multiple", // Special target indicating multi-output
            transcript_id,
        ).await?;
    }
}

// Transform 2: Transcription → Structured Primitives
impl OntologyTransform for TranscriptionStructureTransform {
    async fn transform(&self, db: &Database, source_id: Uuid) -> Result<TransformResult> {
        // 1. Fetch transcription text
        // 2. Call LLM to structure (extract actions, reflections, entities)
        // 3. Create multiple primitives based on LLM output
        //    - introspection_journal for reflective content
        //    - social_interaction for "met with Sarah"
        //    - Resolve entities_person, entities_place, entities_topic
        //    - Link to reference_entities for public figures/concepts mentioned
    }
}
```

### Multi-Output Transforms

**Configured in** `jobs/sync_job.rs`:

```rust
fn get_transform_configs(stream_name: &str) -> Vec<(&str, &str, &str)> {
    match stream_name {
        "gmail" => vec![
            ("stream_google_gmail", "social_email", "social"),
        ],
        "microphone" => vec![
            // Stage 1: Audio → Transcription
            ("stream_ios_microphone", "content_transcription", "content"),
            // Stage 2: Transcription → Multiple (handled by TranscriptionStructureTransform)
        ],
        "calendar" => vec![
            // One stream can create multiple primitives
            ("stream_google_calendar", "activity_calendar_entry", "activity"),
            ("stream_google_calendar", "social_interaction", "social"), // If attendees exist
        ],
        _ => vec![],
    }
}
```

### AI-Powered Transforms

**Transcription Structuring (LLM)**:

- Input: Raw transcription text
- Output: Structured JSON with experiential vs. reflective segments
- Entity resolution: Resolve "Sarah" → `entities_person`, "the gym" → `entities_place`
- Reference tagging: "Marcus Aurelius" → `reference_entities`

**Example LLM Output**:

```json
{
  "experiential": [
    {
      "type": "social_interaction",
      "what": "coffee meeting",
      "with_person": "Sarah", // → resolve to entities_person
      "at_place": "Blue Bottle", // → resolve to entities_place
      "when": "this morning"
    }
  ],
  "reflective": [
    {
      "type": "introspection_reflection",
      "content": "thinking about Stoic philosophy",
      "references": ["Stoicism", "Marcus Aurelius"] // → reference_entities
    }
  ]
}
```

---

## Relationship to Life Events Layer

### Terminology Clarification

**"Primitives" refers to the Ontology Layer** (this document):

- Domain primitives: `social_email`, `health_heart_rate`, `location_visit`, etc.
- Entity primitives: `entities_person`, `entities_place`, `entities_topic`
- Reference entities: `reference_entities`

**"Life Event Structures" refers to the Life Events Layer** (future implementation):

- Temporal abstractions: `Action`, `Event`, `Cycle`, `Arc`, `Chapter`, `Telos`
- These are **not primitives** — they're synthesized from primitives

### The Ontology Layer Is the Foundation

The ontology layer is the **normalized fact layer**. The Life Events synthesis layer is built **on top of** these primitives via:

- **PELT changepoint detection**: Finds temporal boundaries in signals (when did sleep quality drop?)
- **LLM enrichment**: Extracts semantic meaning from Rich Signals (journals, transcriptions)
- **Neo4j graph construction**: Creates nodes (Life Event structures) and relationships (causal edges)
- **Phronesis Engine**: Discovers causal patterns (Granger causality, Transfer Entropy)

**Data flow:**

```
Ontology Primitives (Postgres)
    ↓ [PELT, LLM, statistical algorithms]
Life Event Structures (Neo4j nodes)
    ↓ [Phronesis Engine]
Causal Relationships (Neo4j edges)
    ↓ [Query + LLM synthesis]
Narrative Answers ("Why was I low energy last week?")
```

The ontology primitives serve as **input** to the Life Events construction, not the output.

### Example Query Flow

**User Question**: "Why was I low energy last week?"

1. **Query Ontology Layer (Postgres)**: Find primitives matching "low energy"
   - `health_mood WHERE valence < 0 AND when BETWEEN '2024-10-19' AND '2024-10-25'`
   - Returns: 3 low-energy events (Mon, Tues, Thurs)

2. **Query Life Events Layer (Neo4j)**: Traverse causal graph backwards
   - `MATCH (cause)-[:CAUSED]->(effect) WHERE effect.uuid IN [...]`
   - Returns: Bad sleep (0.92 confidence), Q4 Project stress (1.0 user-confirmed)

3. **Enrich with Ontology Data (Postgres)**: Get details on causes
   - `health_sleep WHERE id = 'bad_sleep_uuid'` → "4.1 hours (baseline: 7.6 hours)"
   - `introspection_journal` → "Overwhelmed by the timeline"

4. **Synthesize Answer (LLM)**: Combine structured data + narrative context
   - "Your low energy was caused by systematic bad sleep (92% confidence) and the Q4 Project kickoff, which you noted made you feel overwhelmed."

---
