//! Prudent Context Job
//!
//! LLM-powered background job that computes "prudent context" - the right context at the right time.
//! Runs 4x daily (6am, 12pm, 6pm, 10pm) to maintain fresh baseline context for AI conversations.
//!
//! The job:
//! 1. Gathers raw data from axiology (values) and ontology (facts) tables
//! 2. Uses Claude to curate what matters MOST right now (not just what's recent)
//! 3. Stores structured context in data.prudent_context_snapshot
//! 4. Context is automatically loaded via MCP resources in AI conversations

use chrono::{DateTime, Datelike, Timelike, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::sync::Arc;
use uuid::Uuid;

use crate::api::rate_limit::{check_rate_limit, record_usage, RateLimits, TokenUsage};
use crate::llm::{LLMClient, LLMRequest};

/// Prudent context structure returned by LLM
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrudentContext {
    pub time_context: TimeContext,
    pub values: ValuesContext,
    pub facts: FactsContext,
    pub cross_references: Vec<CrossReference>,
    pub summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeContext {
    pub computed_at: DateTime<Utc>,
    pub time_of_day: String,
    pub day_of_week: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValuesContext {
    pub active_telos: Option<TelosItem>,
    pub prioritized_tasks: Vec<TaskItem>,
    pub todays_habits: Vec<HabitItem>,
    pub relevant_virtues: Vec<VirtueItem>,
    pub active_vices: Vec<ViceItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FactsContext {
    pub todays_calendar: Vec<CalendarItem>,
    pub recent_salient_events: Vec<SalientEvent>,
    pub upcoming_significant: Vec<UpcomingItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrossReference {
    pub fact: String,
    pub aligns_with: String,
    pub suggestion: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelosItem {
    pub id: String,
    pub title: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskItem {
    pub id: String,
    pub title: String,
    pub tags: Vec<String>,
    pub status: String,
    pub why_now: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HabitItem {
    pub id: String,
    pub title: String,
    pub frequency: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VirtueItem {
    pub id: String,
    pub title: String,
    pub relevance: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ViceItem {
    pub id: String,
    pub title: String,
    pub watch_for: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalendarItem {
    pub time: String,
    pub title: String,
    pub details: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SalientEvent {
    pub event: String,
    pub when: String,
    pub why_salient: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpcomingItem {
    pub when: String,
    pub what: String,
    pub significance: String,
}

/// Raw data gathered from database before LLM curation
#[derive(Debug, Clone, Serialize)]
struct RawContextData {
    // Axiology & Actions
    telos: Option<serde_json::Value>,
    tasks: Vec<serde_json::Value>,
    habits: Vec<serde_json::Value>,
    virtues: Vec<serde_json::Value>,
    vices: Vec<serde_json::Value>,

    // Ontology facts
    todays_events: Vec<serde_json::Value>,
    recent_activities: Vec<serde_json::Value>,
    upcoming_events: Vec<serde_json::Value>,
}

pub struct PrudentContextJob {
    pool: Arc<PgPool>,
    llm_client: Arc<dyn LLMClient>,
}

impl PrudentContextJob {
    pub fn new(pool: Arc<PgPool>, llm_client: Arc<dyn LLMClient>) -> Self {
        Self { pool, llm_client }
    }

    /// Execute the job: gather data, curate with LLM, store snapshot
    pub async fn execute(&self) -> Result<(), String> {
        tracing::info!("PrudentContextJob: Starting execution");

        // Calculate expiration (next scheduled run)
        let expires_at = self.calculate_next_run();

        // Gather raw data from database
        let raw_data = self.gather_raw_data().await?;

        // Use LLM to curate prudent context
        let prudent_context = self.llm_curate_context(raw_data).await?;

        // Store in database
        self.store_context(&prudent_context, expires_at).await?;

        tracing::info!("PrudentContextJob: Completed successfully");
        Ok(())
    }

    /// Gather raw data from axiology and ontology tables
    async fn gather_raw_data(&self) -> Result<RawContextData, String> {
        tracing::debug!("Gathering raw context data from database");

        // Get active telos (should be singular)
        let telos_row = sqlx::query(
            "SELECT id, title, description FROM data.axiology_telos WHERE is_active = true LIMIT 1",
        )
        .fetch_optional(self.pool.as_ref())
        .await
        .map_err(|e| format!("Failed to fetch telos: {}", e))?;

        let telos = telos_row.map(|row| {
            use sqlx::Row;
            serde_json::json!({
                "id": row.get::<uuid::Uuid, _>("id"),
                "title": row.get::<String, _>("title"),
                "description": row.get::<Option<String>, _>("description")
            })
        });

        // Get active tasks (short-term goals)
        let task_rows = sqlx::query(
            "SELECT id, title, tags, description, status, progress_percent FROM data.actions_task WHERE is_active = true ORDER BY created_at DESC"
        )
        .fetch_all(self.pool.as_ref())
        .await
        .map_err(|e| format!("Failed to fetch tasks: {}", e))?;

        let tasks: Vec<serde_json::Value> = task_rows
            .into_iter()
            .map(|row| {
                use sqlx::Row;
                serde_json::json!({
                    "id": row.get::<uuid::Uuid, _>("id"),
                    "title": row.get::<String, _>("title"),
                    "tags": row.get::<Option<Vec<String>>, _>("tags"),
                    "status": row.get::<Option<String>, _>("status"),
                    "progress_percent": row.get::<Option<i32>, _>("progress_percent"),
                    "description": row.get::<Option<String>, _>("description")
                })
            })
            .collect();

        // Get habits
        let habit_rows = sqlx::query(
            "SELECT id, title, description, frequency FROM data.axiology_habit WHERE is_active = true",
        )
        .fetch_all(self.pool.as_ref())
        .await
        .map_err(|e| format!("Failed to fetch habits: {}", e))?;

        let habits: Vec<serde_json::Value> = habit_rows
            .into_iter()
            .map(|row| {
                use sqlx::Row;
                serde_json::json!({
                    "id": row.get::<uuid::Uuid, _>("id"),
                    "title": row.get::<String, _>("title"),
                    "description": row.get::<Option<String>, _>("description"),
                    "frequency": row.get::<Option<String>, _>("frequency")
                })
            })
            .collect();

        // Get virtues
        let virtue_rows = sqlx::query(
            "SELECT id, title, description FROM data.axiology_virtue WHERE is_active = true",
        )
        .fetch_all(self.pool.as_ref())
        .await
        .map_err(|e| format!("Failed to fetch virtues: {}", e))?;

        let virtues: Vec<serde_json::Value> = virtue_rows
            .into_iter()
            .map(|row| {
                use sqlx::Row;
                serde_json::json!({
                    "id": row.get::<uuid::Uuid, _>("id"),
                    "title": row.get::<String, _>("title"),
                    "description": row.get::<Option<String>, _>("description")
                })
            })
            .collect();

        // Get vices to watch for
        let vice_rows =
            sqlx::query("SELECT id, title, description FROM data.axiology_vice WHERE is_active = true")
                .fetch_all(self.pool.as_ref())
                .await
                .map_err(|e| format!("Failed to fetch vices: {}", e))?;

        let vices: Vec<serde_json::Value> = vice_rows
            .into_iter()
            .map(|row| {
                use sqlx::Row;
                serde_json::json!({
                    "id": row.get::<uuid::Uuid, _>("id"),
                    "title": row.get::<String, _>("title"),
                    "description": row.get::<Option<String>, _>("description")
                })
            })
            .collect();

        // Get today's calendar events (if available in ontology)
        // Note: This queries calendar events - table may not exist yet
        let todays_events_rows = sqlx::query(
            r#"
            SELECT title, description, start_time, end_time
            FROM social_calendar_event
            WHERE DATE(start_time) = CURRENT_DATE
            ORDER BY start_time
            LIMIT 20
            "#,
        )
        .fetch_all(self.pool.as_ref())
        .await
        .unwrap_or_default();

        let todays_events: Vec<serde_json::Value> = todays_events_rows
            .into_iter()
            .map(|row| {
                use sqlx::Row;
                serde_json::json!({
                    "title": row.get::<String, _>("title"),
                    "description": row.get::<Option<String>, _>("description"),
                    "start_time": row.get::<chrono::DateTime<chrono::Utc>, _>("start_time"),
                    "end_time": row.get::<Option<chrono::DateTime<chrono::Utc>>, _>("end_time")
                })
            })
            .collect();

        // Get recent activity (last 48 hours)
        // This is a placeholder - tables may not exist yet
        let recent_activities: Vec<serde_json::Value> = vec![];

        // Get upcoming events (next 7 days)
        let upcoming_events_rows = sqlx::query(
            r#"
            SELECT title, description, start_time
            FROM social_calendar_event
            WHERE start_time > NOW() AND start_time < NOW() + INTERVAL '7 days'
            ORDER BY start_time
            LIMIT 10
            "#,
        )
        .fetch_all(self.pool.as_ref())
        .await
        .unwrap_or_default();

        let upcoming_events: Vec<serde_json::Value> = upcoming_events_rows
            .into_iter()
            .map(|row| {
                use sqlx::Row;
                serde_json::json!({
                    "title": row.get::<String, _>("title"),
                    "description": row.get::<Option<String>, _>("description"),
                    "start_time": row.get::<chrono::DateTime<chrono::Utc>, _>("start_time")
                })
            })
            .collect();

        Ok(RawContextData {
            telos,
            tasks,
            habits,
            virtues,
            vices,
            todays_events,
            recent_activities,
            upcoming_events,
        })
    }

    /// Use LLM to curate raw data into prudent context
    async fn llm_curate_context(&self, raw: RawContextData) -> Result<PrudentContext, String> {
        let now = Utc::now();
        let time_of_day = match now.hour() {
            0..=5 => "night",
            6..=11 => "morning",
            12..=17 => "afternoon",
            18..=21 => "evening",
            _ => "night",
        };
        let day_of_week = now.weekday().to_string();

        let prompt = format!(
            r#"You are curating prudent context for a personal AI assistant.

**Current Time**: {} ({}, {})

**Raw Data Available:**

AXIOLOGY (Values Framework):
- Telos (Life Purpose): {}
- Goals: {} active goals
- Habits: {} daily/weekly habits
- Virtues to cultivate: {} items
- Vices to resist: {} items

ONTOLOGY (Facts):
- Today's calendar: {} events
- Recent activities (48h): {} types
- Upcoming events (7d): {} events

**FULL DATA:**
```json
{}
```

**TASK:**
Determine the "right context at the right time" - what matters MOST right now? With the human person, there is both recency bias in time, relevance in the current domains of action present or locations, and importance in salience of past and upcoming events.

**Guidelines:**
1. Prioritize by PRUDENCE (what's timely and relevant), not just recency
2. Cross-reference facts with values (e.g., calendar event aligns with which task?)
3. Keep it lightweight: top 3-5 tasks, top 5 salient events/habits
4. For each item, explain WHY it matters right now
5. Generate actionable cross-references between facts and values
6. Write a concise summary (2-3 sentences) of today's focus

**OUTPUT FORMAT (strict JSON):**
```json
{{
  "time_context": {{
    "computed_at": "{}",
    "time_of_day": "{}",
    "day_of_week": "{}"
  }},
  "values": {{
    "active_telos": {{"id": "...", "title": "...", "description": "..."}},
    "prioritized_tasks": [
      {{"id": "...", "title": "...", "tags": ["work", "creative"], "status": "active", "why_now": "Explain why this task matters TODAY"}}
    ],
    "todays_habits": [
      {{"id": "...", "title": "...", "frequency": "daily|weekly"}}
    ],
    "relevant_virtues": [
      {{"id": "...", "title": "...", "relevance": "Why this virtue matters today"}}
    ],
    "active_vices": [
      {{"id": "...", "title": "...", "watch_for": "Specific situations to be aware of"}}
    ]
  }},
  "facts": {{
    "todays_calendar": [
      {{"time": "HH:MM", "title": "...", "details": "..."}}
    ],
    "recent_salient_events": [
      {{"event": "...", "when": "timestamp or relative", "why_salient": "Why this matters"}}
    ],
    "upcoming_significant": [
      {{"when": "relative time", "what": "...", "significance": "Why it matters"}}
    ]
  }},
  "cross_references": [
    {{
      "fact": "Specific calendar event or activity",
      "aligns_with": "Specific goal or virtue",
      "suggestion": "Concrete actionable suggestion"
    }}
  ],
  "summary": "2-3 sentence summary of what matters most today"
}}
```

Return ONLY the JSON, no other text."#,
            now.format("%Y-%m-%d %H:%M:%S UTC"),
            time_of_day,
            day_of_week,
            if raw.telos.is_some() {
                "1 telos"
            } else {
                "none set"
            },
            raw.tasks.len(),
            raw.habits.len(),
            raw.virtues.len(),
            raw.vices.len(),
            raw.todays_events.len(),
            raw.recent_activities.len(),
            raw.upcoming_events.len(),
            serde_json::to_string_pretty(&raw).unwrap_or_default(),
            now.to_rfc3339(),
            time_of_day,
            day_of_week
        );

        // Check rate limit before making LLM call
        check_rate_limit(self.pool.as_ref(), "background_job", &RateLimits::default())
            .await
            .map_err(|e| format!("Rate limit exceeded: {}", e))?;

        tracing::debug!("Calling LLM for context curation");

        let response = self
            .llm_client
            .generate(LLMRequest {
                model: "anthropic/claude-sonnet-4".to_string(), // AI Gateway model format
                prompt,
                max_tokens: 4096,
                temperature: 0.3,
                system: Some(
                    "You are a context curation specialist. Return only valid JSON.".to_string(),
                ),
            })
            .await?;

        tracing::info!(
            "LLM curation used {} input + {} output tokens",
            response.usage.input_tokens,
            response.usage.output_tokens
        );

        // Record API usage for rate limiting and cost tracking
        record_usage(
            self.pool.as_ref(),
            "background_job",
            TokenUsage {
                input: response.usage.input_tokens as u32,
                output: response.usage.output_tokens as u32,
                model: "anthropic/claude-sonnet-4".to_string(),
            },
        )
        .await
        .map_err(|e| format!("Failed to record usage: {}", e))?;

        // Parse the JSON response
        let context: PrudentContext = serde_json::from_str(&response.content).map_err(|e| {
            format!(
                "Failed to parse LLM response as PrudentContext: {}\n\nResponse was:\n{}",
                e, response.content
            )
        })?;

        Ok(context)
    }

    /// Store curated context in database
    async fn store_context(
        &self,
        context: &PrudentContext,
        expires_at: DateTime<Utc>,
    ) -> Result<(), String> {
        let context_json = serde_json::to_value(context)
            .map_err(|e| format!("Failed to serialize context: {}", e))?;

        let token_count = context_json.to_string().len() as i32; // Rough estimate
        let id = Uuid::new_v4();
        let model = "anthropic/claude-sonnet-4";

        sqlx::query(
            r#"
            INSERT INTO data.prudent_context_snapshot
            (id, computed_at, expires_at, context_data, llm_model, token_count)
            VALUES ($1, $2, $3, $4, $5, $6)
            "#,
        )
        .bind(id)
        .bind(context.time_context.computed_at)
        .bind(expires_at)
        .bind(context_json)
        .bind(model)
        .bind(token_count)
        .execute(self.pool.as_ref())
        .await
        .map_err(|e| format!("Failed to store context: {}", e))?;

        tracing::info!("Stored prudent context snapshot (expires: {})", expires_at);
        Ok(())
    }

    /// Calculate when this context expires (next scheduled run)
    fn calculate_next_run(&self) -> DateTime<Utc> {
        let now = Utc::now();
        let current_hour = now.hour();

        // Schedule: 6am, 12pm, 6pm, 10pm
        let next_hour = if current_hour < 6 {
            6
        } else if current_hour < 12 {
            12
        } else if current_hour < 18 {
            18
        } else if current_hour < 22 {
            22
        } else {
            6 // Next day
        };

        if next_hour == 6 && current_hour >= 22 {
            // Next run is tomorrow at 6am
            now.date_naive()
                .succ_opt()
                .unwrap()
                .and_hms_opt(6, 0, 0)
                .unwrap()
                .and_utc()
        } else {
            // Next run is today
            now.date_naive()
                .and_hms_opt(next_hour, 0, 0)
                .unwrap()
                .and_utc()
        }
    }
}
