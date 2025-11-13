# Ariata Architecture

## Data Pipeline & Query Architecture

```mermaid
graph TB
    %% Data Ingestion Pipeline
    Sources["Sources"] --> Streams["Streams"]

    %% Core Data Layers
    Streams --> Ontologies["Ontologies<br/>Facts & Events<br/>Postgres"]
    Streams --> S3["Data Lake<br/>Encrypted Raw Storage"]
    Ontologies -->|Changepoint + LLM| Narrative["Narrative<br/>pgvector"]

    %% Axiology Layer (Values System)
    Axiology["Axiology<br/>Values System<br/>Postgres"]

    %% Future Layers (Post-MVP)
    Narrative -.->|CDC Sync| LifePrimitives["Life Primitives**<br/>Neo4j"]
    LifePrimitives -.-> Patterns["Patterns**"]

    %% Prudent Context Generation
    PrudentContextJob["Prudent Context Job<br/>4x Daily via Claude Sonnet 4"]

    Ontologies --> PrudentContextJob
    Axiology --> PrudentContextJob
    Narrative --> PrudentContextJob
    LifePrimitives -.-> PrudentContextJob
    Patterns -.-> PrudentContextJob

    %% MCP Server (Model Context Protocol)
    subgraph MCP [MCP Server]
        direction TB
        BasePrudence["Base Prudence<br/>Curated Context State"]
        Agent1["SQL Agent<br/>Facts & Time"]
        Agent2["RAG+Rerank Agent<br/>Semantic Search"]
        Agent3["Graph Agent**<br/>Cypher Patterns"]
        Agent4["Causal Agent**<br/>Why & What-If"]
    end

    PrudentContextJob --> BasePrudence
    Ontologies --> Agent1
    Narrative --> Agent2
    LifePrimitives -.-> Agent3
    Patterns -.-> Agent4

    %% User Interface
    MCP --> UI["Human<br/>UI/UX"]

    %% Styling
    classDef default font-size:14px
```

---

## Inference Time Architecture

This diagram shows what happens when a user interacts with Ariata's AI assistant at inference time:

```mermaid
sequenceDiagram
    participant User
    participant UI as Web UI<br/>(SvelteKit)
    participant API as API Server<br/>(+server.ts)
    participant MCP as MCP Server<br/>(Rust)
    participant DB as PostgreSQL
    participant LLM as Claude Sonnet 4<br/>(via Vercel AI Gateway)

    User->>UI: Starts conversation<br/>"Help me plan my day"

    UI->>API: POST /api/chat

    Note over API: Agent Orchestrator<br/>initializes conversation

    API->>MCP: get_prudent_context()

    MCP->>DB: SELECT context_data FROM<br/>prudent_context_snapshot<br/>WHERE expires_at > NOW()

    DB-->>MCP: Returns curated context:<br/>• Prioritized goals (with why_now)<br/>• Today's habits & virtues<br/>• Calendar events<br/>• Salient recent events<br/>• Cross-references<br/>• Actionable suggestions

    MCP-->>API: Structured context (JSON)

    Note over API: Injects prudent context<br/>into system prompt

    API->>LLM: Conversation + Context:<br/>User values, goals, schedule,<br/>what matters RIGHT NOW

    LLM-->>API: Personalized, value-aligned<br/>response

    API-->>UI: Streaming response

    UI-->>User: Shows AI response<br/>with context awareness

    Note over User,LLM: Context refreshed 4x daily<br/>(6am, 12pm, 6pm, 10pm)<br/>by PrudentContextJob
```

### Key Components

#### 1. Prudent Context Snapshot

- Pre-computed baseline context that's refreshed 4x daily
- Contains curated intersection of **Values** (Axiology) and **Facts** (Ontology)
- LLM-prioritized by PRUDENCE: what's timely and relevant RIGHT NOW
- Stored in PostgreSQL as JSONB with expiration timestamp

#### 2. MCP Server

- Exposes `get_prudent_context()` tool to AI assistants
- Provides fast, low-latency context retrieval (no computation at inference time)
- Returns structured data: goals, habits, calendar, events, cross-references

#### 3. Agent Orchestrator

- Manages conversation flow in API layer
- Injects prudent context into system prompt at conversation start
- Routes queries to appropriate retrieval agents (SQL, RAG, etc.)

#### 4. Context Refresh Cycle

```mermaid
graph LR
    A[6am Run] --> B[12pm Run]
    B --> C[6pm Run]
    C --> D[10pm Run]
    D --> A

    style A fill:#ffe082
    style B fill:#fff59d
    style C fill:#ffb74d
    style D fill:#90a4ae
```

Each run queries fresh data from Axiology + Ontology, sends to Claude for curation, and stores with expiration = next scheduled run.

---

## Query Method Comparison

| Method | Layer | Use Case | Example |
|--------|-------|----------|---------|
| **SQL Query** | Ontologies | Factual lookups, time-based filters | "All emails from Sarah last month" |
| **RAG + Rerank** | Narrative | Story-based, semantic search | "When did I feel most creative?" |
| **Graph Traversal** | Life Primitives | Relationship patterns, entity connections | "Who do I meet most often at the gym?" |
| **Causal Inference** | Patterns | Why/what-if questions, root cause | "What caused my productivity spike?" |
| **Value Alignment** | Axiology | Should/telos questions, decision support | "Should I take this opportunity?" |

---

## Technology Stack

| Component | Technology | Purpose |
|-----------|------------|---------|
| **Database** | PostgreSQL 16+ | Streams, Ontologies, Narrative, Axiology, Prudent Context |
| **Vector Search** | pgvector | Semantic similarity for narrative retrieval |
| **Graph Database** | Neo4j | Life primitives, relationship traversal |
| **LLM Provider** | Claude Sonnet 4 (Anthropic) | Narrative generation, prudent context curation |
| **LLM Gateway** | Vercel AI Gateway | Multi-provider LLM routing, API management |
| **MCP Server** | Model Context Protocol (Rust) | Exposes prudent context and tools to AI assistants |
| **Embeddings** | OpenAI ada-002 | Vector embeddings for semantic search |
| **Job Scheduler** | Cron-based (tokio-cron-scheduler) | Scheduled context refresh 4x daily |
| **CDC Pipeline** | Debezium + Kafka (or manual sync) | Postgres → Neo4j sync |

---

## Implementation Status

### Core Data Pipeline (Implemented)

**Data Ingestion:**

- ✅ Sources → Streams (multi-source data ingestion)
- ✅ Streams → Ontologies (ELT transformation to 47 primitives)
- ✅ Ontologies → Narrative (Changepoint detection + LLM generation)

**Values System (Axiology):**

- ✅ Axiology schema (5-level hierarchy: values → telos → goals → patterns → preferences)
- ✅ Telos management (ultimate purpose)
- ✅ Goals tracking (work, character, experiential, relational)
- ✅ Habits, virtues, vices, temperaments

**Prudent Context System:**

- ✅ PrudentContextJob (LLM-curated context generation)
- ✅ Scheduled execution framework (4x daily: 6am, 12pm, 6pm, 10pm)
- ✅ Database schema (prudent_context_snapshot with JSONB storage)
- ✅ MCP Server integration (get_prudent_context tool)
- ⚠️  Scheduler activation (implementation complete, needs activation in server startup)

**Storage:**

- ✅ PostgreSQL (Streams, Ontologies, Narrative, Axiology, Prudent Context)
- ✅ pgvector (Narrative embeddings for semantic search)

**Retrieval:**

- ✅ SQL Agent (query ontologies for facts & time ranges)
- ✅ RAG+Rerank Agent (semantic search on narrative)
- ✅ MCP Server (prudent context retrieval)

**UI/UX:**

- ✅ Vercel AI SDK chat interface
- ✅ Timeline view (chronological narrative)
- ✅ Agent orchestrator with context injection

### Future Development (Post-MVP)

**Data Layers:**

- ❌ Neo4j / Life Primitives (hierarchical graph structure)
- ❌ Patterns (causal inference engine)

**Retrieval Agents:**

- ❌ Graph Agent (Cypher queries on Neo4j)
- ❌ Causal Agent (why/what-if queries)

**Infrastructure:**

- ❌ CDC Pipeline (Debezium + Kafka for Postgres → Neo4j sync)
- ❌ Entity Resolution (fuzzy matching/deduplication)

---

## Key Design Documents

- **[CHANGEPOINT.md](CHANGEPOINT.md)**: Detailed specification for event boundary detection and narrative segmentation
- **[ONTOLOGIES.md](ONTOLOGIES.md)**: Complete ontology schema (47 primitives across 9 domains)
- **[ENTITY_RESOLUTION.md](ENTITY_RESOLUTION.md)**: Entity deduplication strategy (deferred for MVP)

---

**Legend:**

- **Layers without asterisk**: 3-week MVP implementation
- **Layers with `**`**: Post-MVP future development
