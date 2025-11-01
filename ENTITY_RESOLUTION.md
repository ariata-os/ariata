# Entity Resolution Design

## Overview

This document describes the entity resolution architecture for Ariata's ontology layer. Entity resolution canonicalizes identities (people, places, topics) across all data sources, enabling queries like "all emails with Sarah" or "time spent at the gym."

## Architecture

### Hybrid Approach

Entity resolution uses a **dual approach**:

1. **Phase 1: Formulaic (Deterministic)** - Implemented in Rust, runs during transformation
2. **Phase 2: AI-Powered (Optional)** - Python prototyping, future Rust integration

### Data Flow

```
Stream Layer (raw provider data)
    ↓ [Sync Job]
Stream Tables (e.g., stream_google_gmail)
    ↓ [Transform Job with Entity Resolution]
Ontology Primitives + Entities
    ├─ Domain Primitives (social_email, health_heart_rate)
    └─ Entity Primitives (entities_person, entities_place, entities_topic)
```

---

## Phase 1: Formulaic Entity Resolution (Current)

### Implemented in Rust

All entity resolution logic runs **during transformation jobs**, creating or linking to canonical entities as primitives are created.

### Resolution Strategies by Entity Type

#### **1. `entities_person`**

**Resolution Algorithm:**

1. **Exact Email Match** (Primary)
   - Normalize email to lowercase
   - Query: `SELECT id FROM entities_person WHERE $1 = ANY(email_addresses)`
   - If found: Return existing entity ID
   - If not: Create new entity

2. **Exact Phone Match** (Secondary)
   - Normalize phone to E.164 format (+15551234567)
   - Query: `SELECT id FROM entities_person WHERE $1 = ANY(phone_numbers)`
   - If found: Return existing entity ID, add email if new
   - If not: Continue to email match

3. **Display Name Aggregation**
   - Collect all seen name variations
   - Store in `display_names[]` array
   - Pick `canonical_name` as most frequently seen full name

**Example:**

- Email 1: `"Sarah Jones" <sarah.jones@company.com>`
- Email 2: `"Sarah J." <sarah.jones@company.com>`
- Result: 1 entity with `canonical_name: "Sarah Jones"`, `display_names: ["Sarah Jones", "Sarah J."]`

**Implementation Location:** `core/src/entity_resolution/person.rs` (to be created)

---

#### **2. `entities_place`**

**Resolution Algorithm:**

1. **Geo-Clustering (DBSCAN)**
   - Group `location_point` records within radius threshold
   - Urban: `eps = 50 meters`
   - Suburban: `eps = 200 meters`
   - Minimum points: 3 visits to form cluster

2. **Centroid Calculation**
   - Calculate geographic center of cluster
   - Store as `geo_center` (PostGIS POINT)

3. **Bounding Box**
   - Create convex hull of all visit points
   - Store as `bounding_box` (PostGIS POLYGON)

**Implementation Location:** `core/src/entity_resolution/place.rs` (to be created)

**Dependencies:**

- `geo` crate for geospatial calculations
- PostGIS extension in Postgres
- Haversine distance formula

---

#### **3. `entities_topic`**

**Resolution Algorithm:**

1. **Exact Keyword Match** (Primary)
   - Normalize keyword (lowercase, trim)
   - Query: `SELECT id FROM entities_topic WHERE name = $1`
   - If found: Return existing entity ID
   - If not: Create new entity

2. **Keyword Expansion**
   - Store related terms in `keywords[]` array
   - Example: Topic "Q4 Fundraising" has keywords: `["fundraising", "deck", "Q4", "presentation"]`

**Implementation Location:** `core/src/entity_resolution/topic.rs` (to be created)

---

### Audit Trail

All entity creation and merging is logged in `entity_merge_log`:

```sql
INSERT INTO elt.entity_merge_log (
    entity_type,
    source_entity_ids,
    target_entity_id,
    merge_method,
    confidence_score,
    merge_reason,
    matching_features,
    approved_by
) VALUES (
    'person',
    ARRAY[]::UUID[], -- Empty for new entity creation
    $entity_id,
    'exact_email_match',
    1.0,
    'Created new person entity from email address',
    '{"email": "exact"}',
    'system'
);
```

---

## Phase 2: AI-Powered Enhancement (Future)

### Overview

Phase 2 adds probabilistic and ML-based matching to improve accuracy beyond exact matches.

### Enhancements

#### **1. Fuzzy Name Matching**

**Method**: Jaro-Winkler similarity

**Rust Implementation:**

```rust
use strsim::jaro_winkler;

fn fuzzy_match_person_name(name1: &str, name2: &str) -> bool {
    let similarity = jaro_winkler(name1, name2);
    similarity > 0.9 // 90% similarity threshold
}
```

**Use Case**: Match "Katherine Edwards" with "Kathy Edwards"

---

#### **2. Phonetic Matching**

**Method**: Soundex algorithm

**Rust Implementation:**

```rust
// Simple Soundex implementation
fn soundex(name: &str) -> String {
    // Implementation details...
}

fn phonetic_match(name1: &str, name2: &str) -> bool {
    soundex(name1) == soundex(name2)
}
```

**Use Case**: Match "Smith" with "Smythe"

---

#### **3. AI-Powered Name Matching**

**Method**: LLM comparison (OpenAI GPT-4 or similar)

**Use Cases**:

- Nickname matching: "Bob" ↔ "Robert", "Liz" ↔ "Elizabeth"
- Name variations: "J. Smith" ↔ "John Smith" ↔ "John A. Smith"
- Cultural variations: "José" ↔ "Pepe", "Francisco" ↔ "Paco"
- Initials: "J.R. Smith" ↔ "James Robert Smith"

**Implementation Strategy**:

1. **Phase 1 (Transformation)**: Exact matching only (fast, deterministic)
2. **Phase 2 (Nightly Batch)**: LLM-powered duplicate detection
3. **Auto-merge**: High confidence (>0.85)
4. **Human review**: Medium confidence (0.6-0.85)

**Cost**: ~$0.001 per comparison, ~100 comparisons/night = $0.10/night ($3/month)

**Rust Implementation**: Call OpenAI API via `reqwest` crate with structured JSON output

---

#### **4. LLM-Based Entity Merging**

**Use Case**: Ambiguous cases that require context

**Example:**

- Email 1: `adam@gmail.com` (display name: "Adam Jace")
- Email 2: `adam.jace@company.com` (display name: "Adam J.")
- Question: Same person or different?

**LLM Prompt:**

```
Given these two person records, determine if they represent the same individual:

Record 1:
- Email: adam@gmail.com
- Display Names: ["Adam Jace"]
- First seen: 2023-01-01
- Interaction count: 50

Record 2:
- Email: adam.jace@company.com
- Display Names: ["Adam J.", "A. Jace"]
- First seen: 2024-06-01
- Interaction count: 20

Context: Record 2's email domain is "company.com" where Record 1 sent 15 emails.

Are these the same person? Respond with JSON:
{
  "same_person": true/false,
  "confidence": 0.0-1.0,
  "reasoning": "explanation"
}
```

**Implementation**: Call OpenAI API from Rust via `reqwest` crate, or use PyO3 to call Python LLM libraries.

---

#### **5. Splink for Probabilistic Matching**

**Python Prototyping:**

```python
import splink.duckdb as splink

# Configure Splink settings
settings = {
    "link_type": "dedupe_only",
    "blocking_rules_to_generate_predictions": [
        "l.email = r.email",  # Exact match blocking
        "l.phone = r.phone",
    ],
    "comparisons": [
        splink.comparison_library.exact_match("email", term_frequency_adjustments=True),
        splink.comparison_library.jaro_winkler_at_thresholds("name", [0.9, 0.8]),
        splink.comparison_library.exact_match("phone"),
    ],
}

# Train model
linker = splink.Linker(df, settings)
linker.estimate_u_using_random_sampling(max_pairs=1e6)
linker.estimate_parameters_using_expectation_maximisation("l.email = r.email")

# Predict matches
predictions = linker.predict(threshold_match_probability=0.8)
```

**Output**: Pairs of records with match probabilities

**Workflow**:

1. Export entities from Postgres to CSV
2. Run Splink in Python
3. Review high-confidence matches (0.8-0.95) with human-in-the-loop UI
4. Auto-merge very high confidence (>0.95)
5. Update `entity_merge_log` with merge decisions
6. Update Postgres entities table

---

## Implementation Roadmap

### Phase 1: Formulaic Resolution (Current Sprint)

**Tasks:**

1. **Create Entity Resolution Module**
   - [x] Migration: `010_job_chaining.sql` (adds chaining support)
   - [ ] Migration: `011_entity_tables.sql` (creates entity tables)
   - [ ] Module: `core/src/entity_resolution/mod.rs`
   - [ ] Module: `core/src/entity_resolution/person.rs`
   - [ ] Module: `core/src/entity_resolution/place.rs`
   - [ ] Module: `core/src/entity_resolution/topic.rs`
   - [ ] Module: `core/src/entity_resolution/audit.rs`

2. **Implement Person Resolution**
   - [ ] `resolve_person_by_email(db, email, display_name) -> Result<Uuid>`
   - [ ] `resolve_person_by_phone(db, phone, display_name) -> Result<Uuid>`
   - [ ] `add_email_to_person(db, person_id, email) -> Result<()>`
   - [ ] `add_phone_to_person(db, person_id, phone) -> Result<()>`
   - [ ] `update_person_interaction(db, person_id) -> Result<()>`

3. **Implement Place Resolution**
   - [ ] `cluster_location_points(db, points) -> Result<Vec<Cluster>>`
   - [ ] `resolve_or_create_place(db, cluster) -> Result<Uuid>`
   - [ ] `add_visit_to_place(db, place_id, visit) -> Result<()>`

4. **Implement Topic Resolution**
   - [ ] `resolve_topic_by_keyword(db, keyword) -> Result<Uuid>`
   - [ ] `add_keyword_to_topic(db, topic_id, keyword) -> Result<()>`

5. **Update Gmail Transform**
   - [ ] Modify `GmailEmailTransform` to call `resolve_person_by_email()`
   - [ ] Add `from_person_id`, `to_person_ids` columns to `social_email`
   - [ ] Update migration `009_social_email_ontology.sql`

6. **Testing**
   - [ ] Unit tests for each resolution function
   - [ ] Integration test: Gmail transform creates entities
   - [ ] Integration test: Multiple emails to same person create 1 entity

### Phase 2: Fuzzy Matching (Next Sprint)

**Tasks:**

1. **Add Rust Fuzzy Matching**
   - [ ] Add `strsim` crate to Cargo.toml
   - [ ] Implement Jaro-Winkler matching in `person.rs`
   - [ ] Add fuzzy match threshold configuration

2. **Add Phonetic Matching**
   - [ ] Implement Soundex algorithm
   - [ ] Add to person resolution strategy

3. **Nickname Dictionary**
   - [ ] Create nickname mapping file (JSON or embedded)
   - [ ] Implement nickname lookup in person resolution

### Phase 3: AI Integration (Future Sprint)

**Tasks:**

1. **Python Prototyping Space**
   - [ ] Create `research/entity_resolution/` directory
   - [ ] Jupyter notebook for Splink experiments
   - [ ] Export script: Postgres → CSV
   - [ ] Import script: Merge decisions → Postgres

2. **LLM Entity Merging**
   - [ ] Implement OpenAI API call from Rust
   - [ ] Create prompt template for entity comparison
   - [ ] Add confidence threshold configuration
   - [ ] Integrate with `entity_merge_log`

3. **Human-in-the-Loop UI**
   - [ ] Web UI for reviewing ambiguous matches
   - [ ] Approve/reject merge decisions
   - [ ] Track inter-annotator agreement

---

## Reference Entities (Lightweight Tags)

### Overview

For entities you **don't directly interact with** (public figures, media, concepts), use lightweight reference entities instead of full entity resolution.

### Implementation

**When to use:**

- Transcription mentions "Marcus Aurelius" (public figure)
- Journal entry discusses "Stoicism" (concept)
- Podcast about "Taylor Swift" (musician)

**Resolution Strategy:**

1. **Exact canonical name match**
2. **External ID lookup** (Wikipedia ID, Spotify Artist ID)
3. **Create if not exists** (no fuzzy matching needed)

**Linking:**

- Use `primitive_reference_links` table
- Many-to-many: 1 transcription can reference multiple public figures

**Example:**

```rust
async fn link_reference_entity(
    db: &PgPool,
    primitive_table: &str,
    primitive_id: Uuid,
    entity_name: &str,
    entity_type: &str,
) -> Result<()> {
    // Find or create reference entity
    let entity_id = sqlx::query_scalar::<_, Uuid>(
        "SELECT id FROM elt.reference_entities WHERE canonical_name = $1 AND entity_type = $2"
    )
    .bind(entity_name)
    .bind(entity_type)
    .fetch_optional(db)
    .await?;

    let entity_id = match entity_id {
        Some(id) => id,
        None => {
            // Create new reference entity
            sqlx::query_scalar::<_, Uuid>(
                "INSERT INTO elt.reference_entities (canonical_name, entity_type) VALUES ($1, $2) RETURNING id"
            )
            .bind(entity_name)
            .bind(entity_type)
            .fetch_one(db)
            .await?
        }
    };

    // Create link
    sqlx::query(
        "INSERT INTO elt.primitive_reference_links (primitive_table, primitive_id, reference_entity_id)
         VALUES ($1, $2, $3) ON CONFLICT DO NOTHING"
    )
    .bind(primitive_table)
    .bind(primitive_id)
    .bind(entity_id)
    .execute(db)
    .await?;

    Ok(())
}
```

---

## Performance Considerations

### Indexing Strategy

**Person Entity Lookups:**

```sql
-- Fast email lookups
CREATE INDEX idx_entities_person_email ON elt.entities_person USING GIN(email_addresses);

-- Fast phone lookups
CREATE INDEX idx_entities_person_phone ON elt.entities_person USING GIN(phone_numbers);
```

**Place Entity Lookups:**

```sql
-- Fast geospatial queries
CREATE INDEX idx_entities_place_geocenter ON elt.entities_place USING GIST(geo_center);
```

**Primitive → Entity Foreign Keys:**

```sql
-- Fast joins from primitives to entities
CREATE INDEX idx_social_email_from_person ON elt.social_email(from_person_id);
CREATE INDEX idx_social_email_to_persons ON elt.social_email USING GIN(to_person_ids);
```

### Caching Strategy

**In-Memory Caching (Optional):**

- Cache recently resolved entities in Rust HashMap
- Key: email/phone, Value: entity_id
- TTL: 5 minutes
- Reduces DB queries during bulk transformations

**Example:**

```rust
use std::collections::HashMap;
use std::time::{Duration, Instant};

struct EntityCache {
    email_cache: HashMap<String, (Uuid, Instant)>,
    ttl: Duration,
}

impl EntityCache {
    fn get(&mut self, email: &str) -> Option<Uuid> {
        if let Some((id, timestamp)) = self.email_cache.get(email) {
            if timestamp.elapsed() < self.ttl {
                return Some(*id);
            }
        }
        None
    }

    fn insert(&mut self, email: String, entity_id: Uuid) {
        self.email_cache.insert(email, (entity_id, Instant::now()));
    }
}
```

---

## Error Handling

### Transactional Entity Creation

**Requirement**: Entity creation and primitive creation must be atomic.

**Implementation**: Use Postgres transactions

```rust
async fn transform_with_entity_resolution(
    db: &Database,
    email_record: EmailRecord,
) -> Result<()> {
    let mut tx = db.pool.begin().await?;

    // 1. Resolve or create person entity
    let person_id = resolve_person_by_email_tx(&mut tx, &email_record.from_email).await?;

    // 2. Insert primitive referencing entity
    sqlx::query(
        "INSERT INTO elt.social_email (from_person_id, ...) VALUES ($1, ...)"
    )
    .bind(person_id)
    .execute(&mut *tx)
    .await?;

    // 3. Log entity resolution in audit trail
    log_entity_resolution_tx(&mut tx, person_id, "exact_email_match").await?;

    // 4. Commit transaction
    tx.commit().await?;

    Ok(())
}
```

### Conflict Resolution

**Scenario**: Two concurrent transforms try to create the same entity

**Solution**: Use `ON CONFLICT DO NOTHING` or `ON CONFLICT UPDATE`

```sql
INSERT INTO elt.entities_person (
    id, canonical_name, email_addresses, first_interaction, last_interaction
) VALUES ($1, $2, ARRAY[$3], NOW(), NOW())
ON CONFLICT (id) DO UPDATE SET
    email_addresses = array_append(entities_person.email_addresses, $3),
    last_interaction = NOW(),
    interaction_count = entities_person.interaction_count + 1;
```

---

## Monitoring & Observability

### Metrics to Track

1. **Entity Creation Rate**
   - New entities created per day
   - Trend: Should stabilize as entities are discovered

2. **Entity Resolution Accuracy**
   - Manual review of sample entities
   - Inter-annotator agreement on merge decisions

3. **Transform Performance**
   - Time spent in entity resolution per transform
   - Cache hit rate (if caching implemented)

4. **Merge Activity**
   - Number of merges per day (Phase 2+)
   - Merge confidence score distribution

### Logging

**Structured Logging:**

```rust
tracing::info!(
    email = %email_address,
    person_id = %entity_id,
    method = "exact_email_match",
    "Resolved person entity"
);
```

**Audit Trail Queries:**

```sql
-- See all merges in the last 7 days
SELECT * FROM elt.entity_merge_log
WHERE created_at > NOW() - INTERVAL '7 days'
ORDER BY created_at DESC;

-- See all entities created by exact email match
SELECT * FROM elt.entity_merge_log
WHERE merge_method = 'exact_email_match'
AND created_at > NOW() - INTERVAL '30 days';
```

---

## Future Enhancements

### 1. Entity Deduplication Dashboard

**Purpose**: UI for reviewing and merging duplicate entities

**Features**:

- List potential duplicates (from Splink or LLM)
- Side-by-side comparison
- Approve/reject merge
- Undo capability

### 2. Entity Graph Visualization

**Purpose**: Visualize relationships between entities

**Features**:

- Network graph of people (who knows whom via shared emails)
- Heatmap of places (time spent at each location)
- Topic evolution over time

### 3. Active Learning for Entity Matching

**Purpose**: Iteratively improve matching model with minimal labels

**Workflow**:

1. Model suggests ambiguous pairs (confidence 0.4-0.6)
2. Human labels 30-40 examples
3. Retrain matching model
4. Repeat until accuracy threshold met

---

## References

### Academic Papers

- "Deep Entity Matching with Pre-Trained Language Models" (Ditto, VLDB 2020)
- "Pre-trained Embeddings for Entity Resolution" (VLDB 2023)
- "Geospatial Entity Resolution" (WWW 2022)

### Libraries & Tools

- **Splink**: Probabilistic entity resolution (Python)
- **strsim-rs**: String similarity metrics (Rust)
- **geo**: Geospatial calculations (Rust)
- **Sentence-Transformers**: Semantic embeddings (Python)

### Industry Approaches

- **AWS Entity Resolution**: Hybrid (rules + ML + 3rd party)
- **Snowflake Cortex**: LLM + Splink integration
- **Databricks ARC**: Automated record connector (Splink-based)
- **Google BigQuery**: Entity Reconciliation API

---

## Appendix: SQL Schema Reference

See [ONTOLOGIES.md](ONTOLOGIES.md) for complete schema definitions of:

- `entities_person`
- `entities_place`
- `entities_topic`
- `reference_entities`
- `primitive_reference_links`
- `entity_merge_log`
