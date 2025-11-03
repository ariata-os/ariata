# Ariata Ontology Design

## Purpose

Normalized, queryable facts across sources for:

1. Deterministic event timeboxing (PELT/changepoint detection input)
2. Base elements for narrative event construction
3. Fact-based retrieval layer
4. Knowledge graph construction (Neo4j sync)

**Structure**: Domain-based tables (`health_heart_rate`, `social_email`, etc.)
**Granularity**: Both infinitesimal signals (heart rate readings) AND discrete events (calendar meetings)
**Storage**: Postgres (relational, time-series optimized) → Neo4j (graph relationships)

## Philosophy

- **Domain-Oriented**: 8 fundamental domains of human experience
- **Time is Orthogonal**: Time is not a domain - it's a universal field on all primitives (via `timestamp`, `start_time/end_time`)
- **Event-Oriented**: Optimized for narrative event synthesis and autobiography generation
- **Lossless Transformation**: No filtering during transformation - preserve all data
- **Naming Convention**: `domain_concept` pattern (e.g., `health_heart_rate`, `social_email`)
- **Temporal Types**: Primitives are either signals (infinitesimal) or temporal (bounded duration)
- **Graph-Ready**: All primitives designed for Neo4j sync with temporal/causal edges

---

## Architecture Overview

The Ontology Layer contains two types of primitives:

1. **Domain Primitives**: Normalized facts about events/signals (e.g., `social_email`, `health_heart_rate`)
2. **Entity Primitives**: Canonical identities that appear across your life (e.g., `entities_person`, `entities_place`)

Domain primitives reference entity primitives via foreign keys (nullable, filled post-resolution), enabling queries like "all emails with Sarah" or "all activities at the gym."

**Two-Layer Architecture:**

- **Postgres (Ontology Layer)**: Single source of truth for facts (this document)
- **Neo4j (Knowledge Graph Layer)**: Relationships, temporal edges, causal inference (future)

**Semantic Search:**

- Public figures, media, concepts → **Vector embeddings** (not deterministic entities)
- Query "when did I think about Stoicism?" → Semantic search across primitives
- No explicit `reference_entities` table needed

---

## Entity Primitives

Entity primitives resolve and canonicalize identities across all sources. Unlike domain primitives (which are time-bound facts), entities persist and accumulate relationships over time.

### `entities_person`

**Definition**: Canonical identities of people you directly interact with (colleagues, friends, family, service providers).

**Fields:**

- `id` - UUID primary key
- `canonical_name` - Primary display name
- `email_addresses[]` - All known email addresses
- `phone_numbers[]` - All known phone numbers
- `display_names[]` - All seen name variations
- `relationship_category` - colleague, friend, family, service_provider
- `first_interaction` - Timestamp of first interaction
- `last_interaction` - Timestamp of most recent interaction
- `interaction_count` - Total interaction count
- `metadata` - JSONB for flexible attributes
- `created_at`, `updated_at` - Standard timestamps

---

### `entities_place`

**Definition**: Canonical locations you physically visit (home, office, gym, restaurants, parks).

**Fields:**

- `id` - UUID primary key
- `canonical_name` - Place name (e.g., "Home", "Office")
- `category` - home, work, gym, restaurant, park, etc.
- `geo_center` - GEOGRAPHY(POINT) centroid of visit cluster
- `bounding_box` - GEOGRAPHY(POLYGON) convex hull
- `cluster_radius_meters` - Max distance from center
- `visit_count` - Total number of visits
- `total_time_minutes` - Aggregate time spent
- `first_visited`, `last_visited` - Temporal bounds
- `metadata` - JSONB (address, business info, etc.)
- `created_at`, `updated_at` - Standard timestamps

---

### `entities_topic`

**Definition**: Projects, interests, and themes in your life (work projects, learning goals, personal interests).

**Fields:**

- `id` - UUID primary key
- `name` - Topic name
- `category` - project, skill, interest, goal
- `keywords[]` - Related terms for matching
- `first_mentioned`, `last_mentioned` - Temporal bounds
- `mention_count` - Total mentions
- `sources` - JSONB tracking which primitives reference this
- `metadata` - JSONB for flexible attributes
- `created_at`, `updated_at` - Standard timestamps

---

## 8 Ontological Domains

### 1. `health_*`

**Definition**: Physiological measurements and bodily states, including both physical and mental health signals.

**Inclusion:** Body metrics, sleep, exercise, nutrition, medication, mental health states
**Exclusions:** Reflective thoughts → `introspection_journal`, Appointments → `activity_calendar_entry`, Articles → `knowledge_document`

#### Signals (Infinitesimal)

**`health_heart_rate`**

- `bpm` - Beats per minute
- `measurement_context` - resting, active, workout, sleep
- `timestamp` - When measured
- Source tracking + metadata

**`health_hrv`**

- `hrv_ms` - Heart rate variability in milliseconds
- `measurement_type` - rmssd, sdnn, pnn50
- `timestamp` - When measured
- Source tracking + metadata

**`health_blood_oxygen`**

- `spo2_percent` - Blood oxygen saturation percentage
- `timestamp` - When measured
- Source tracking + metadata

**`health_blood_pressure`**

- `systolic_mmhg`, `diastolic_mmhg` - Blood pressure readings
- `timestamp` - When measured
- Source tracking + metadata

**`health_blood_glucose`**

- `glucose_mg_dl` - Blood glucose level
- `meal_context` - fasting, pre_meal, post_meal, random
- `timestamp` - When measured
- Source tracking + metadata

**`health_body_temperature`**

- `temperature_celsius` - Body temperature
- `measurement_location` - oral, forehead, wrist, ear
- `timestamp` - When measured
- Source tracking + metadata

**`health_respiratory_rate`**

- `breaths_per_minute` - Breathing rate
- `timestamp` - When measured
- Source tracking + metadata

**`health_steps`**

- `step_count` - Number of steps
- `timestamp` - When measured
- Source tracking + metadata

#### Temporal (Bounded Duration)

**`health_sleep`**

- `sleep_stages` - JSONB array of sleep stages with durations
- `total_duration_minutes` - Total sleep time
- `sleep_quality_score` - 0.0-100.0 quality rating
- `start_time`, `end_time` - Sleep session bounds
- Source tracking + metadata (awakenings, restfulness)

**`health_workout`**

- `activity_type` - running, cycling, strength_training, yoga, swimming
- `intensity` - low, moderate, high, max
- `calories_burned`, `average_heart_rate`, `max_heart_rate`, `distance_meters` - Metrics
- `place_id` - FK to entities_place (nullable)
- `start_time`, `end_time` - Workout session bounds
- Source tracking + metadata (route, elevation, equipment)

**`health_meal`**

- `meal_type` - breakfast, lunch, dinner, snack
- `foods` - JSONB array of food items with nutrition
- `total_calories`, `protein_grams`, `carbs_grams`, `fat_grams` - Nutrition totals
- `place_id` - FK to entities_place (nullable)
- `timestamp` - When eaten
- Source tracking + metadata (photos, notes)

**`health_medication`**

- `medication_name`, `dosage`, `route` - Medication details
- `timestamp` - When taken
- Source tracking + metadata (prescription_id, reason, side_effects)

**`health_symptom`**

- `symptom_name`, `severity`, `body_location` - Symptom details
- `start_time`, `end_time` - Symptom duration (end_time nullable if ongoing)
- Source tracking + metadata (notes, triggers, treatments)

**`health_mood`**

- `valence` - -1.0 (negative) to 1.0 (positive)
- `arousal` - -1.0 (low energy) to 1.0 (high energy)
- `mood_category` - happy, sad, anxious, calm, stressed, energized
- `measurement_method` - self_reported, hrv_derived, activity_inferred
- `timestamp` - When assessed
- Source tracking + metadata

---

### 2. `location_*`

**Definition**: Geographic position and movement data, capturing where you are and where you've been.

**Inclusion:** GPS coordinates, location points, time at places, movement patterns
**Exclusions:** Place names → `entities_place`, Calendar events at locations → `activity_calendar_entry`, Travel bookings → `finance_transaction`

#### Signals (Infinitesimal)

**`location_point`**

- `coordinates` - GEOGRAPHY(POINT)
- `latitude`, `longitude`, `altitude_meters` - Coordinate components
- `accuracy_meters` - GPS accuracy
- `speed_meters_per_second`, `course_degrees` - Motion context
- `timestamp` - When recorded
- Source tracking + metadata (device, battery, activity_type)

#### Temporal (Bounded Duration)

**`location_visit`**

- `place_id` - FK to entities_place (nullable, resolved via clustering)
- `centroid_coordinates` - GEOGRAPHY(POINT) visit centroid
- `latitude`, `longitude` - Centroid components
- `start_time`, `end_time` - Visit duration
- Source tracking + metadata (arrival_method, radius_meters)

---

### 3. `social_*`

**Definition**: Communication and interpersonal interactions across all channels (digital and in-person).

**Inclusion:** Messages (email/text/chat), voice/video calls, in-person meetings, social media posts
**Exclusions:** Person identities → `entities_person`, Scheduled meetings (before they happen) → `activity_calendar_entry`, Transcriptions → `speech_transcription`

#### Temporal (Bounded Duration)

**`social_email`** ✅ **[IMPLEMENTED]**

- `message_id`, `thread_id` - Email identifiers
- `subject`, `body_plain`, `body_html`, `snippet` - Content
- `from_address`, `to_addresses[]`, `cc_addresses[]`, `bcc_addresses[]` - Participants (raw)
- `from_person_id`, `to_person_ids[]`, `cc_person_ids[]` - Resolved entities (nullable)
- `direction` - sent, received
- `labels[]`, `is_read`, `is_starred`, `has_attachments` - Metadata
- `thread_position`, `thread_message_count` - Threading
- `timestamp` - When sent/received
- Source tracking + metadata

**`social_message`**

- `message_id`, `thread_id`, `channel` - Message identifiers (channel: sms, imessage, slack, whatsapp, discord)
- `body` - Message content
- `from_identifier`, `to_identifiers[]` - Participants (raw: phone/username)
- `from_person_id`, `to_person_ids[]` - Resolved entities (nullable)
- `direction` - sent, received
- `is_read` - Read status
- `timestamp` - When sent/received
- Source tracking + metadata (reactions, attachments, channel_name)

**`social_call`**

- `call_type` - voice, video
- `direction` - incoming, outgoing
- `call_status` - answered, missed, declined, voicemail
- `caller_identifier`, `callee_identifiers[]` - Participants (raw)
- `caller_person_id`, `callee_person_ids[]` - Resolved entities (nullable)
- `duration_seconds` - Call duration (NULL if missed/declined)
- `start_time`, `end_time` - Call bounds
- Source tracking + metadata (app_name, quality_score)

**`social_interaction`**

- `interaction_type` - meeting, gathering, event, casual_encounter
- `title`, `description` - Interaction details
- `participant_identifiers[]` - Participants (raw: names/emails)
- `participant_person_ids[]` - Resolved entities (nullable)
- `place_id`, `location_name` - Location (FK + raw string)
- `start_time`, `end_time` - Interaction duration
- Source tracking + metadata (calendar_event_id, conference_url)

**`social_post`**

- `platform` - twitter, instagram, facebook, linkedin
- `post_id`, `post_type` - Post identifiers (type: original, repost, reply, quote)
- `content`, `media_urls[]` - Post content
- `like_count`, `repost_count`, `comment_count` - Engagement metrics
- `timestamp` - When posted
- Source tracking + metadata (hashtags, mentions, visibility)

---

### 4. `activity_*`

**Definition**: Time allocation and attention tracking - observable behaviors showing where time is spent and what you're doing.

**Inclusion:** Calendar events, app usage, screen time, web browsing, focus sessions
**Exclusions:** Content consumed → `knowledge_*`, Physical exercise → `health_workout`, Social interactions → `social_interaction`

#### Temporal (Bounded Duration)

**`activity_calendar_entry`**

- `title`, `description`, `calendar_name` - Event details
- `event_type` - meeting, appointment, reminder, focus_block
- `organizer_identifier`, `attendee_identifiers[]` - Participants (raw)
- `organizer_person_id`, `attendee_person_ids[]` - Resolved entities (nullable)
- `topic_id`, `topic_keywords[]` - Topic reference (FK + raw keywords)
- `location_name`, `place_id` - Location (raw + FK)
- `conference_url`, `conference_platform` - Virtual meeting details
- `start_time`, `end_time`, `is_all_day` - Timing
- `status`, `response_status` - Event and response status
- Source tracking + metadata (recurrence, reminders, color)

**`activity_app_usage`**

- `app_name`, `app_bundle_id`, `app_category` - App details
- `window_title`, `document_path` - Context details
- `start_time`, `end_time` - Usage session
- Source tracking + metadata (idle_time, active_time, music_playing for Spotify)

**`activity_screen_time`**

- `device_name`, `device_type` - Device details
- `total_screen_time_seconds`, `unlock_count` - Usage metrics
- `start_time`, `end_time` - Session bounds
- Source tracking + metadata (by_category breakdown, pickups_per_hour)

**`activity_web_browsing`**

- `url`, `domain`, `page_title` - Page details
- `visit_duration_seconds`, `scroll_depth_percent` - Visit metrics
- `timestamp` - When visited
- Source tracking + metadata (referrer, browser, search_query, tab_count)

**`activity_focus_session`**

- `session_type` - deep_work, pomodoro, flow_state
- `task_description` - What you were working on
- `topic_id` - FK to entities_topic (nullable)
- `distraction_count`, `focus_score` - Focus metrics
- `start_time`, `end_time` - Session duration
- Source tracking + metadata (app_used, interruptions)

---

### 5. `finance_*`

**Definition**: Monetary transactions, account balances, and financial resources.

**Inclusion:** Purchases, payments, income, subscriptions, account balances
**Exclusions:** Financial planning docs → `knowledge_document`, Bill reminders → `activity_calendar_entry`

#### Signals (Infinitesimal)

**`finance_balance`**

- `account_name`, `account_type`, `institution_name` - Account details
- `balance_cents`, `currency` - Balance (stored as cents)
- `timestamp` - When recorded
- Source tracking + metadata (available_balance, pending_balance)

#### Temporal (Bounded Duration)

**`finance_transaction`**

- `transaction_id`, `transaction_type` - Transaction identifiers
- `description`, `merchant_name` - Transaction details
- `amount_cents`, `currency` - Amount (negative for expenses, positive for income)
- `account_name`, `account_type` - Account details
- `category`, `subcategory` - Transaction categorization
- `place_id` - FK to entities_place (nullable)
- `timestamp` - When occurred
- Source tracking + metadata (tags, receipt_url, split_with, pending)

**`finance_subscription`**

- `service_name`, `subscription_type` - Subscription details
- `amount_cents`, `currency`, `billing_period_days` - Billing details
- `status` - active, cancelled, paused, trial
- `start_date`, `end_date`, `next_billing_date` - Timing
- Source tracking + metadata (plan_name, renewal_auto, payment_method)

---

### 6. `ambient_*`

**Definition**: External environmental conditions and signals surrounding you (weather, air quality, noise, light, audio classification).

**Inclusion:** Weather, air quality, noise levels, light levels, environmental audio classification
**Exclusions:** Music/podcasts you intentionally listen to → `activity_app_usage`, Location → `location_*`

#### Signals (Infinitesimal)

**`ambient_weather`**

- `temperature_celsius`, `feels_like_celsius`, `humidity_percent` - Temperature/humidity
- `precipitation_mm`, `wind_speed_kmh`, `wind_direction_degrees` - Precipitation/wind
- `condition_category`, `condition_description` - Weather conditions
- `pressure_hpa`, `uv_index` - Pressure/UV
- `place_id`, `latitude`, `longitude` - Location
- `timestamp` - When recorded
- Source tracking + metadata

**`ambient_air_quality`**

- `aqi`, `aqi_category` - Air Quality Index
- `pm25`, `pm10`, `ozone`, `no2`, `co`, `so2` - Pollutant levels
- `place_id`, `latitude`, `longitude` - Location
- `timestamp` - When recorded
- Source tracking + metadata (monitoring_station, dominant_pollutant)

**`ambient_noise_level`**

- `decibels`, `noise_category` - Noise measurement
- `place_id` - FK to entities_place (nullable)
- `timestamp` - When recorded
- Source tracking + metadata (device, calibration)

**`ambient_light_level`**

- `lux`, `light_category` - Light measurement
- `place_id` - FK to entities_place (nullable)
- `timestamp` - When recorded
- Source tracking + metadata (device, sensor_type)

#### Temporal (Bounded Duration)

**`ambient_audio_classification`**

- `audio_class` - music, conversation, traffic, nature, silence, wind, bar_inside, office, construction, etc.
- `confidence` - ML model confidence (0.0-1.0)
- `audio_subclass` - Genre/type refinement (e.g., jazz, birds, rain)
- `volume_level_db` - Average loudness
- `place_id` - FK to entities_place (nullable)
- `start_time`, `end_time` - Duration
- Source tracking + metadata (model_version, raw_predictions, sampling_rate)

---

### 7. `knowledge_*`

**Definition**: Semantic artifacts and documents - structured information objects you create, curate, or sync (NOT consumption tracking, but the artifacts themselves).

**Inclusion:** Synced documents, curated collections, saved/bookmarked content, search queries
**Exclusions:** Time spent reading/watching → `activity_*`, Creating docs → `activity_app_usage` + synced as `knowledge_document`, Public figures/media → Vector embeddings

#### Temporal (Bounded Duration)

**`knowledge_document`**

- `title`, `content`, `content_summary`, `document_type` - Document details
- `external_id`, `external_url` - External reference (Notion page ID, Google Doc ID)
- `topic_id`, `tags[]` - Topic reference + tags
- `is_authored` - TRUE if you wrote it, FALSE if synced from elsewhere
- `created_time`, `last_modified_time` - Document timestamps
- Source tracking + metadata (parent_page, database_properties, collaborators)

**`knowledge_playlist`**

- `name`, `description`, `playlist_type` - Playlist details (type: music, video, podcast, reading_list, watch_later)
- `external_id`, `external_url` - External reference (Spotify/YouTube playlist ID)
- `item_count`, `items` - Content (JSONB array of items)
- `is_public` - Privacy setting
- `created_time`, `last_modified_time` - Playlist timestamps
- Source tracking + metadata (cover_image, genre, collaborative, follower_count)

**`knowledge_bookmark`**

- `url`, `title`, `description` - Bookmark details
- `page_content` - Saved page text
- `topic_id`, `tags[]` - Topic reference + tags
- `saved_at` - When bookmarked
- Source tracking + metadata (domain, author, reading_time, archive_url)

**`knowledge_search`**

- `query`, `search_engine` - Search details (engine: google, chatgpt, perplexity, notion, github)
- `result_count`, `clicked_result_url` - Results info
- `topic_id`, `inferred_keywords[]` - Topic inference
- `timestamp` - When searched
- Source tracking + metadata (conversation_id for ChatGPT, filters_applied, location)

---

### 8. `speech_*`

**Definition**: Transcribed spoken audio - raw audio converted to text, serving as an intermediate primitive for further structuring.

**Inclusion:** Voice memos transcribed, microphone recordings transcribed, phone call transcriptions
**Exclusions:** Structured content extracted from speech → `introspection_journal`, `social_interaction` (via multi-stage transform), Podcast transcriptions → Not captured

#### Temporal (Bounded Duration)

**`speech_transcription`**

- `audio_file_path`, `audio_duration_seconds` - Audio source
- `transcript_text`, `language`, `confidence_score` - Transcription details
- `speaker_count`, `speaker_labels` - Speaker information (JSONB with speaker segments)
- `recorded_at` - When recorded
- Source tracking + metadata (transcription_service, is_processed for multi-stage transforms)

---

### 9. `introspection_*`

**Definition**: Self-reflection, metacognition, and user-attested narratives about internal states, goals, and values.

**Inclusion:** Journal entries, goals, gratitude logs, reflections, dream journals
**Exclusions:** Measurable mood → `health_mood`, Goal documents → `knowledge_document`, Goal calendar events → `activity_calendar_entry`

#### Temporal (Bounded Duration)

**`introspection_journal`**

- `title`, `content` - Journal entry
- `sentiment_score` - -1.0 (negative) to 1.0 (positive)
- `topic_ids[]`, `tags[]` - Topic references + tags
- `entry_type` - written, voice_transcribed, prompted
- `entry_date` - When written
- Source tracking + metadata (prompt_used, extracted_from_speech_id, location)

**`introspection_goal`**

- `title`, `description`, `goal_type` - Goal details
- `topic_id` - FK to entities_topic (nullable)
- `status`, `progress_percent` - Progress tracking
- `created_date`, `target_date`, `completed_date` - Timing
- Source tracking + metadata (milestones, why_important, blockers)

**`introspection_gratitude`**

- `content` - Gratitude entry
- `gratitude_category` - people, experiences, achievements, material, health
- `person_ids[]`, `place_ids[]` - Entity references
- `entry_date` - When written
- Source tracking + metadata

**`introspection_reflection`**

- `title`, `content` - Reflection entry
- `reflection_type` - daily, weekly, event, decision, lesson_learned
- `topic_ids[]`, `tags[]` - References
- `reflection_date` - When written
- Source tracking + metadata (related_events, insights, action_items)

**`introspection_dream`**

- `title`, `description` - Dream entry
- `vividness`, `emotional_tone` - Dream characteristics
- `tags[]` - Recurring elements, people, places, themes
- `dream_date`, `recorded_at` - When dreamed and recorded
- Source tracking + metadata (sleep_quality_that_night, interpretation_notes)

---

## Standard Fields (All Primitives)

Every ontology primitive includes these standard fields:

**Identity:**

- `id` - UUID primary key (auto-generated)

**Source Tracking (Required):**

- `source_stream_id` - UUID reference to source stream record
- `source_table` - Name of source table (e.g., "stream_google_gmail")
- `source_provider` - Provider name (e.g., "google", "ios", "mac")

**Timestamps (Required):**

- `created_at` - When record was created in ontology (default NOW())
- `updated_at` - When record was last updated (auto-updated via trigger)

**Metadata (Optional):**

- `metadata` - JSONB field for flexible provider-specific or context-specific data

---

## Source → Ontology Mappings

### Implemented ✅

- `stream_google_gmail` → `social_email` ✅

### Planned (Priority Order)

**Google Calendar:**

- → `activity_calendar_entry`
- → `social_interaction` (if attendees exist)

**iOS HealthKit:**

- → `health_heart_rate`, `health_hrv`, `health_blood_oxygen`, `health_steps`
- → `health_sleep`, `health_workout`

**iOS Location:**

- → `location_point`, `location_visit`

**Mac iMessage:**

- → `social_message`

**Mac Browser:**

- → `activity_web_browsing`

**Mac Apps:**

- → `activity_app_usage`

**Notion Pages:**

- → `knowledge_document`

**iOS Microphone:**

- → `speech_transcription` → Multi-stage transforms

---

## Design Principles

1. **Lossless**: Transform preserves all data from raw streams
2. **Traceable**: Every ontology record references its source stream record via `source_stream_id`
3. **Normalized**: Cross-source data (e.g., heart rate from multiple devices) shares same schema
4. **Semantic**: Table names are human-readable and self-documenting
5. **Event-first**: Optimized for temporal queries and narrative generation
6. **1:Many Transforms**: A single source stream can create records in multiple ontology tables
7. **Graph-Ready**: Designed for Neo4j sync with temporal/causal edges
8. **Vector-Enhanced**: Public figures/concepts use semantic search, not deterministic entities

---

## Multi-Stage Transforms

Some streams require **chained transformations** where one primitive creates additional primitives.

**Example: Audio → Transcription → Structured Primitives**

```
stream_ios_microphone (audio file)
    ↓ [Transform 1: Whisper API]
speech_transcription (text primitive)
    ↓ [Transform 2: LLM structuring]
Multiple primitives (introspection_journal, social_interaction, etc.)
```

The transform creates the intermediate primitive and spawns follow-up transform jobs via `chained_transforms` in `TransformResult`.

---

## Relationship to Life Events Layer

### Two-Layer Architecture

**Ontology Layer (Postgres)** - This document:

- Domain primitives: `social_email`, `health_heart_rate`, `location_visit`, etc.
- Entity primitives: `entities_person`, `entities_place`, `entities_topic`
- Single source of truth for facts

**Knowledge Graph Layer (Neo4j)** - Future:

- Nodes: Events (aggregated from primitives), Entities
- Edges: `PRECEDES`, `FOLLOWS`, `CAUSED`, `INVOLVES_PERSON`, `AT_PLACE`, `ABOUT_TOPIC`
- Temporal/causal relationships derived from ontology
- Vector embeddings for semantic search (public figures, concepts)

### Data Flow

```
Ontology Primitives (Postgres)
    ↓ [PELT, LLM, statistical algorithms]
Life Event Structures (Neo4j nodes)
    ↓ [Phronesis Engine]
Causal Relationships (Neo4j edges)
    ↓ [Query + LLM synthesis]
Narrative Answers ("Why was I low energy last week?")
```

### Semantic Search vs. Deterministic Entities

**Deterministic Entities** (Postgres):

- `entities_person` - People you interact with
- `entities_place` - Places you visit
- `entities_topic` - Your projects/interests

**Semantic Search** (Vector Embeddings):

- Public figures (Marcus Aurelius, Tim Ferriss)
- Media (books, podcasts, articles)
- Concepts (Stoicism, productivity frameworks)

Query: "When did I think about Stoicism?" → Semantic search across primitive text fields, not explicit entity links.

---

## Summary

This ontology design provides:

- **44 domain primitives** across 9 domains (health, location, social, activity, finance, ambient, knowledge, speech, introspection)
- **3 entity tables** for people, places, topics
- **Consistent patterns** for traceability, timestamps, foreign keys
- **Graph-ready architecture** for Neo4j sync
- **Vector-enhanced** semantic search instead of deterministic reference entities
- **Clear domain boundaries** with strict inclusion/exclusion criteria

The ontology layer serves as the **normalized fact layer** for all downstream analysis, narrative generation, and causal inference.

---
