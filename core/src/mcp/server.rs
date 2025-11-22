//! MCP Server implementation for Ariata
//!
//! This module implements the ServerHandler trait from rmcp to expose
//! Ariata's data warehouse as an MCP server.

use rmcp::{
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{
        Annotated, CallToolResult, Content, ErrorData as McpError, Implementation,
        ListResourcesResult, PaginatedRequestParam, ProtocolVersion, RawResource,
        ReadResourceRequestParam, ReadResourceResult, Resource, ResourceContents,
        ServerCapabilities, ServerInfo,
    },
    service::RequestContext,
    tool, tool_handler, tool_router, RoleServer, ServerHandler,
};
use sqlx::PgPool;
use std::sync::Arc;

use crate::mcp::tools;

/// Ariata MCP Server
///
/// Exposes Ariata's personal data warehouse to AI assistants via the Model Context Protocol.
#[derive(Debug, Clone)]
pub struct AriataMcpServer {
    pool: Arc<PgPool>,
    tool_router: ToolRouter<AriataMcpServer>,
}

#[tool_router]
impl AriataMcpServer {
    /// Create a new Ariata MCP server
    pub fn new(pool: PgPool) -> Self {
        let tool_router = Self::tool_router();
        let tool_count = tool_router.list_all().len();
        tracing::info!("MCP Server initialized with {} tools", tool_count);
        if tool_count > 0 {
            tracing::debug!(
                "Available MCP tools: {:?}",
                tool_router
                    .list_all()
                    .iter()
                    .map(|t| &t.name)
                    .collect::<Vec<_>>()
            );
        } else {
            tracing::warn!("No MCP tools registered! Check #[tool_router] macro expansion");
        }
        Self {
            pool: Arc::new(pool),
            tool_router,
        }
    }

    /// Unified ontology tool - query data, list tables, or get schema
    #[tool(
        description = "Unified ontology tool with 3 operations: (1) 'query' - Execute read-only SQL queries on ontology tables (SELECT only), (2) 'list_tables' - Discover available tables with column names and row counts, (3) 'get_schema' - Get detailed schema for a specific table (column types, nullability, defaults). NOTE: For biographical questions ('who did I meet', 'what happened'), use ariata_query_narratives first instead of exploring tables."
    )]
    async fn ariata_query_ontology(
        &self,
        params: Parameters<tools::QueryOntologyRequest>,
    ) -> Result<CallToolResult, McpError> {
        match tools::query_ontology(&self.pool, params.0).await {
            Ok(result) => {
                let json_str = serde_json::to_string_pretty(&result).map_err(|e| {
                    McpError::internal_error(format!("Failed to serialize result: {}", e), None)
                })?;
                Ok(CallToolResult::success(vec![Content::text(json_str)]))
            }
            Err(e) => {
                // Categorize errors appropriately
                if e.contains("forbidden keyword")
                    || e.contains("Only SELECT")
                    || e.contains("exceeds maximum length")
                    || e.contains("Missing")
                    || e.contains("Invalid operation")
                    || e.contains("not found")
                    || e.contains("does not exist")
                {
                    Err(McpError::invalid_params(e, None))
                } else {
                    Err(McpError::internal_error(e, None))
                }
            }
        }
    }

    // DISABLED: Exposes test dataset names like "monday-in-rome"
    // /// List all data sources and their status
    // #[tool(description = "List all connected data sources (Google, iOS, Mac, Notion, etc.) and their current status, including number of enabled streams and last sync time.")]
    // async fn ariata_list_sources(
    //     &self,
    //     _params: Parameters<tools::ListSourcesRequest>,
    // ) -> Result<CallToolResult, McpError> {
    //     match tools::list_sources(&self.pool).await {
    //         Ok(result) => {
    //             let json_str = serde_json::to_string_pretty(&result)
    //                 .map_err(|e| McpError::internal_error(
    //                     format!("Failed to serialize result: {}", e),
    //                     None
    //                 ))?;
    //             Ok(CallToolResult::success(vec![Content::text(json_str)]))
    //         }
    //         Err(e) => Err(McpError::internal_error(e, None)),
    //     }
    // }

    /// Execute a read-only SQL query against axiology tables
    #[tool(
        description = "Execute a read-only SQL query against axiology tables. Available tables: data.axiology_value, data.axiology_telos, data.axiology_goal, data.axiology_virtue, data.axiology_vice, data.axiology_temperament, data.axiology_preference. Only SELECT queries are allowed. Returns the user's formal axiological framework for context-aware decision support."
    )]
    async fn ariata_query_axiology(
        &self,
        params: Parameters<tools::QueryAxiologyRequest>,
    ) -> Result<CallToolResult, McpError> {
        match tools::query_axiology(&self.pool, params.0).await {
            Ok(result) => {
                let json_str = serde_json::to_string_pretty(&result).map_err(|e| {
                    McpError::internal_error(format!("Failed to serialize result: {}", e), None)
                })?;
                Ok(CallToolResult::success(vec![Content::text(json_str)]))
            }
            Err(e) => {
                // Categorize errors appropriately
                if e.contains("forbidden keyword")
                    || e.contains("Only SELECT")
                    || e.contains("exceeds maximum length")
                {
                    Err(McpError::invalid_params(e, None))
                } else {
                    Err(McpError::internal_error(e, None))
                }
            }
        }
    }

    /// Query narrative biography summaries for specific dates, locations, or people
    #[tool(
        description = "USE THIS FIRST for biographical questions! Query your narrative biography and life story. Returns pre-synthesized prose summaries about what happened, who you met, and events on specific dates/locations. **ALWAYS call this BEFORE any other tool** when answering questions like 'who did I meet', 'what happened on [date]', 'what did I do in [location]', or any question about past events, people, or activities. This is 10x faster than querying raw data tables and contains ALL the context you need. Only use other tools if this returns no results."
    )]
    async fn ariata_query_narratives(
        &self,
        params: Parameters<tools::QueryNarrativesRequest>,
    ) -> Result<CallToolResult, McpError> {
        match tools::query_narratives(&self.pool, params.0).await {
            Ok(result) => {
                let json_str = serde_json::to_string_pretty(&result).map_err(|e| {
                    McpError::internal_error(format!("Failed to serialize result: {}", e), None)
                })?;
                Ok(CallToolResult::success(vec![Content::text(json_str)]))
            }
            Err(e) => {
                // Categorize errors appropriately
                if e.contains("Invalid date format") {
                    Err(McpError::invalid_params(e, None))
                } else {
                    Err(McpError::internal_error(e, None))
                }
            }
        }
    }

    /// Manage axiology entities (create, read, update, delete operations)
    #[tool(
        description = "Manage your axiology system - create, read, update, delete tasks, initiatives, aspirations, values, telos, virtues, vices, habits, temperaments, and preferences. Operations: 'create' (new entity), 'read' (get by ID), 'update' (modify existing), 'delete' (soft delete), 'list' (get all active). Entity types: task (daily/weekly), initiative (month/quarter), aspiration (multi-year), value (foundational principle), telos (life purpose), virtue (positive pattern), vice (negative pattern), habit (daily practice), temperament (innate disposition), preference (entity affinity)."
    )]
    async fn ariata_manage_axiology(
        &self,
        params: Parameters<tools::ManageAxiologyRequest>,
    ) -> Result<CallToolResult, McpError> {
        match tools::manage_axiology(&self.pool, params.0).await {
            Ok(result) => {
                let json_str = serde_json::to_string_pretty(&result).map_err(|e| {
                    McpError::internal_error(format!("Failed to serialize result: {}", e), None)
                })?;
                Ok(CallToolResult::success(vec![Content::text(json_str)]))
            }
            Err(e) => {
                // Categorize errors appropriately
                if e.contains("Invalid operation")
                    || e.contains("Invalid entity_type")
                    || e.contains("Missing")
                    || e.contains("not found")
                {
                    Err(McpError::invalid_params(e, None))
                } else {
                    Err(McpError::internal_error(e, None))
                }
            }
        }
    }

    // TODO: Re-enable when prudent_context_snapshot table is ready
    // /// Get pre-computed prudent context
    // #[tool(description = "Get the latest pre-computed prudent context - the right context at the right time. Refreshed 4x daily (6am, 12pm, 6pm, 10pm) via LLM-powered curation. Contains: prioritized goals, today's habits, relevant virtues/vices, today's calendar, recent salient events, and cross-references between facts and values. Use this at the start of a conversation for baseline context.")]
    // async fn ariata_get_prudent_context(
    //     &self,
    //     _params: Parameters<serde_json::Value>,
    // ) -> Result<CallToolResult, McpError> {
    //     // Fetch latest valid context snapshot
    //     let snapshot = sqlx::query(
    //         r#"
    //         SELECT context_data, computed_at
    //         FROM data.prudent_context_snapshot
    //         WHERE expires_at > NOW()
    //         ORDER BY computed_at DESC
    //         LIMIT 1
    //         "#
    //     )
    //     .fetch_optional(self.pool.as_ref())
    //     .await
    //     .map_err(|e| McpError::internal_error(format!("Database error: {}", e), None))?;
    //
    //     match snapshot {
    //         Some(snap) => {
    //             use sqlx::Row;
    //             let context_data: serde_json::Value = snap.get("context_data");
    //             let computed_at: chrono::DateTime<chrono::Utc> = snap.get("computed_at");
    //
    //             let context_json = serde_json::to_string_pretty(&serde_json::json!({
    //                 "computed_at": computed_at,
    //                 "context": context_data
    //             }))
    //             .map_err(|e| McpError::internal_error(format!("Failed to serialize context: {}", e), None))?;
    //
    //             Ok(CallToolResult::success(vec![Content::text(context_json)]))
    //         }
    //         None => {
    //             // No context available yet
    //             let fallback = serde_json::json!({
    //                 "message": "Prudent context not yet computed. First computation will run at next scheduled time (6am, 12pm, 6pm, or 10pm).",
    //                 "note": "Use ariata_query_ontology and ariata_query_axiology tools to explore data directly in the meantime."
    //             });
    //
    //             let fallback_json = serde_json::to_string_pretty(&fallback)
    //                 .map_err(|e| McpError::internal_error(format!("Failed to serialize fallback: {}", e), None))?;
    //
    //             Ok(CallToolResult::success(vec![Content::text(fallback_json)]))
    //         }
    //     }
    // }
}

#[tool_handler]
impl ServerHandler for AriataMcpServer {
    fn get_info(&self) -> ServerInfo {
        tracing::debug!("MCP get_info() called");
        ServerInfo {
            protocol_version: ProtocolVersion::V_2024_11_05,
            capabilities: ServerCapabilities::builder()
                .enable_tools()
                .enable_resources()
                .build(),
            server_info: Implementation {
                name: "ariata".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
                title: Some("Ariata Personal Data Warehouse".to_string()),
                website_url: Some("https://github.com/adamjace/ariata".to_string()),
                icons: None,
            },
            instructions: Some(
                r#"Ariata Personal Data Warehouse MCP Server

This server provides access to your personal data warehouse, which aggregates data from:
- Health data (HealthKit: heart rate, steps, sleep, etc.)
- Location data (GPS points, visits, places)
- Social data (email, calendar events)
- Knowledge data (documents, AI conversations)
- Axiology data (values, telos, goals, virtues, vices, habits, temperaments, preferences)

Available Tools:
1. ariata_query_narratives - Query your narrative biography and life story (use this for "what happened", "who did I meet", biographical questions)
2. ariata_query_ontology - Unified ontology tool with 3 operations: query (execute SQL), list_tables (discover tables), get_schema (get table details)
3. ariata_query_axiology - Execute read-only SQL queries against axiology tables
4. ariata_manage_axiology - Manage your axiology system (create, read, update, delete tasks, initiatives, aspirations, values, etc.)

Axiology System:
The axiology tables store the user's formal axiological framework:
- axiology_value: Foundational principles (Level 0)
- axiology_telos: Ultimate life purpose - singular active (Level 1)
- axiology_goal: Concrete pursuits with goal_type (work/character/experiential/relational) (Level 2)
- axiology_virtue: Positive character patterns to cultivate (Level 3)
- axiology_vice: Negative character patterns to resist (Level 3)
- axiology_temperament: Innate dispositions (neutral, stable) (Level 3)
- axiology_preference: Affinities with entities (Level 4)

Best Practices:
- Use ariata_query_ontology with operation='list_tables' to discover what data is available
- Use ariata_query_ontology with operation='get_schema' to understand column names and types before querying
- Queries are read-only and limited to 1000 rows max
- Use aggregate functions (COUNT, AVG, etc.) to summarize large datasets
- Filter by date ranges to narrow results
- Query axiology tables for context-aware decision support and dialectic navigation

Privacy & Data Sensitivity:
- All queries run in read-only transactions
- No data leaves your local machine without explicit export
- Sensitive PII can be filtered before results are returned to the model

**Data Sensitivity Notice:**
This server provides access to highly personal data that may include:
- Health metrics: Heart rate, sleep patterns, activity levels (PII: Medical data)
- Location history: Precise GPS coordinates, visited places (PII: Location tracking)
- Social data: Email content, calendar events (PII: Personal communications)
- Knowledge: Documents, notes, AI conversations (PII: Intellectual property)
- Axiology: Personal values, goals, virtues, preferences (PII: Psychological profile)

When crafting queries, consider privacy-conscious approaches:
- Use WHERE clauses to filter sensitive time ranges
- Use aggregate functions (COUNT, AVG) instead of raw data dumps
- Exclude specific columns that contain PII when not needed
- Limit result sets to minimize data exposure

**Example Queries:**

Privacy-conscious aggregated queries:
```sql
-- Weekly health summary (aggregated, no raw data)
SELECT DATE(timestamp), AVG(value) as avg_heart_rate, COUNT(*) as measurements
FROM data.health_heart_rate
WHERE timestamp > NOW() - INTERVAL '7 days'
GROUP BY DATE(timestamp);

-- Step count trends (no location data)
SELECT DATE(start_time), SUM(value) as total_steps
FROM data.health_steps
WHERE start_time > NOW() - INTERVAL '30 days'
GROUP BY DATE(start_time)
ORDER BY DATE(start_time);

-- Upcoming calendar (titles only, no attendees/content)
SELECT title, start_time, end_time
FROM data.praxis_calendar
WHERE start_time BETWEEN NOW() AND NOW() + INTERVAL '7 days'
ORDER BY start_time;
```

Axiology queries for context-aware assistance:
```sql
-- Active aspirations
SELECT title, description, status, target_timeframe
FROM data.praxis_aspiration
WHERE is_active = true
ORDER BY created_at DESC;

-- Active virtues to cultivate
SELECT title, description
FROM data.axiology_virtue
WHERE is_active = true;
```
"#
                .to_string(),
            ),
        }
    }

    /// List available resources
    fn list_resources(
        &self,
        _request: Option<PaginatedRequestParam>,
        _context: RequestContext<RoleServer>,
    ) -> impl std::future::Future<Output = Result<ListResourcesResult, McpError>> + Send + '_ {
        async move {
            // Define the baseline context resource
            let baseline_resource = RawResource {
                uri: "ariata://context/baseline".to_string(),
                name: "Baseline Context".to_string(),
                title: Some("Prudent Baseline Context".to_string()),
                description: Some(
                    "Essential context for every conversation: user identity, current date/time, and session information."
                        .to_string(),
                ),
                mime_type: Some("text/markdown".to_string()),
                size: None,
                icons: None,
            };

            // Wrap RawResource in Annotated to create Resource
            let resource: Resource = Annotated::new(baseline_resource, None);

            Ok(ListResourcesResult {
                resources: vec![resource],
                next_cursor: None,
            })
        }
    }

    /// Read a specific resource
    fn read_resource(
        &self,
        request: ReadResourceRequestParam,
        _context: RequestContext<RoleServer>,
    ) -> impl std::future::Future<Output = Result<ReadResourceResult, McpError>> + Send + '_ {
        async move {
            match request.uri.as_str() {
                "ariata://context/baseline" => {
                    // Get current date/time with timezone
                    let now = chrono::Local::now();
                    let formatted_date = now.format("%A, %B %d, %Y, %I:%M %p %Z").to_string();

                    // Build baseline context markdown
                    let baseline_text = format!(
                        r#"# Baseline Context

**User**: Adam Jace
**Current Date/Time**: {}
**Timezone**: {} (UTC{})

---

## Query Strategy - INTERNAL DECISION LOGIC

### CRITICAL: ALWAYS CHECK NARRATIVES FIRST

**For ANY biographical question** (what happened, who met, what did, where was), your **FIRST and ONLY initial action** must be:

1. Call `ariata_query_narratives` with the date
2. If narratives exist → answer the question from narratives
3. ONLY if narratives are empty → then consider other tools

**DO NOT:**
- ❌ Call `ariata_query_ontology` with operation='list_tables' first
- ❌ Call `ariata_query_ontology` with operation='query' before checking narratives
- ❌ "Explore" the data before checking narratives
- ❌ Check multiple data sources when narratives have the answer


**Why narratives first?**
- Narratives are pre-synthesized prose with ALL context (people, places, events)
- 10x faster than querying raw tables
- More accurate for biographical questions
- Contain the "story" not just raw data points

**When to use other tools:**
- `ariata_query_ontology` operation='list_tables': Only when you need to discover what raw data types exist
- `ariata_query_ontology` operation='query': Only for specific metrics (exact heart rate at 3:42pm) or when narratives don't exist
- `ariata_query_axiology`: For values, goals, habits, virtues, vices queries

---

## User-Facing Principles - CRITICAL

When you find information:
- **Answer conversationally**: Say "Let me check your Monday in Rome..." then provide the answer
- **Hide ALL implementation details**: Never mention query syntax, table names, database structures, or WHERE clauses
- **Keep focus on meaning**: Emphasize what happened, who was involved, and why it matters
- **Avoid observability language**: Don't say "I queried...", "the data shows...", "the table returned...", or explain your query process

### Never Say These Things:
❌ "Let me use the ariata_query_narratives tool..."
❌ "Querying the narrative_chunks table..."
❌ "The data.narrative_chunks table shows..."
❌ "SELECT narrative_text FROM..."
❌ "Here's what the praxis_calendar table returned..."
❌ "The location_point data indicates..."
❌ "Based on ios_microphone_transcription..."


---

This context is provided at the start of every conversation to ensure natural, contextually-grounded responses without exposing technical implementation.
"#,
                        formatted_date,
                        now.format("%Z"),
                        now.format("%:z")
                    );

                    Ok(ReadResourceResult {
                        contents: vec![ResourceContents::text(baseline_text, request.uri)],
                    })
                }
                _ => Err(McpError::invalid_params(
                    format!("Unknown resource URI: {}", request.uri),
                    None,
                )),
            }
        }
    }
}
