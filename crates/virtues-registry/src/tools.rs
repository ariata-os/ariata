//! Built-in tool registry
//!
//! This module defines BUILT-IN tools that are part of Virtues core.
//! These are executed as native Rust functions via the ToolExecutor.
//!
//! # Tool Types
//!
//! - `builtin` - Native Rust implementation (web_search, sql_query, create_page, get_page_content, edit_page)
//! - `mcp` - MCP protocol (user-connected servers, stored in Postgres)

use serde::{Deserialize, Serialize};

/// Tool type - distinguishes built-in vs MCP tools
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ToolType {
    /// Built-in tool - native Rust implementation
    Builtin,
    /// MCP tool - executed via MCP protocol
    Mcp,
}

/// Tool category for UI grouping
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ToolCategory {
    Search,
    Data,
    Edit,
}

/// Built-in tool configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ToolConfig {
    /// Unique tool identifier
    pub id: String,
    /// Human-readable name
    pub name: String,
    /// Short description for UI
    pub description: String,
    /// Detailed description for LLM (helps model decide when to use)
    pub llm_description: String,
    /// JSON Schema for parameters
    pub parameters: serde_json::Value,
    /// Tool type (builtin for registry tools)
    pub tool_type: ToolType,
    /// Category for grouping in UI
    pub category: ToolCategory,
    /// Iconify icon name
    pub icon: String,
    /// Display order in UI
    pub display_order: i32,
    /// Whether this tool is system-only (not shown to users in regular chats).
    /// System tools are only available to action runners and internal callers.
    pub is_system: bool,
}

/// Get default built-in tool configurations
///
/// These are the core tools that ship with Virtues:
/// - web_search: Search the web using Exa AI
/// - sql_query: Read-only SQL queries against user data
/// - code_interpreter: Execute Python code for calculations and analysis
/// - create_page: Create a new page with content
/// - get_page_content: Read current page content
/// - edit_page: Apply edits using find/replace
/// - update_memory: Persist notes across conversations
/// - set_user_name: Set user's preferred name
/// - set_assistant_name: Set AI assistant's name
pub fn default_tools() -> Vec<ToolConfig> {
    vec![
        think_tool(),
        propose_narrative_identity_tool(),
        write_it_up_tool(),
        update_memory_tool(),
        set_user_name_tool(),
        set_assistant_name_tool(),
        web_search_tool(),
        semantic_search_tool(),
        sql_query_tool(),
        sql_write_tool(),
        code_interpreter_tool(),
        dispatch_subagents_tool(),
        create_page_tool(),
        get_page_content_tool(),
        edit_page_tool(),
        setup_applet_tool(),
        update_applet_memory_tool(),
        list_applets_tool(),
        get_applet_tool(),
        edit_applet_tool(),
        delete_applet_tool(),
        run_applet_tool(),
        dayline_event_tool(),
        get_project_item_tool(),
        generate_image_tool(),
        read_asset_tool(),
    ]
}

/// Generate Image tool — text-to-image via the gateway image model.
fn generate_image_tool() -> ToolConfig {
    ToolConfig {
        id: "generate_image".to_string(),
        name: "Generate Image".to_string(),
        description: "Generate an image from a text description".to_string(),
        llm_description: r#"Generate an image from a text description using an AI image model.

Use this tool when:
- The user asks you to create, draw, generate, or illustrate an image
- A picture would clearly help (a scene, concept, mockup, or design)

Write a vivid, specific prompt: subject, style, composition, lighting, and mood.
The image is shown to the user automatically — after it returns, give a brief
caption, not a long description of what you generated.

Returns: the generated image (rendered inline to the user)."#.to_string(),
        parameters: serde_json::json!({
            "type": "object",
            "required": ["prompt"],
            "properties": {
                "prompt": {
                    "type": "string",
                    "description": "Detailed description of the image to generate (subject, style, composition, mood)"
                }
            }
        }),
        tool_type: ToolType::Builtin,
        category: ToolCategory::Edit,
        icon: "ri:image-add-line".to_string(),
        display_order: 22,
        is_system: false,
    }
}

/// Propose an addition to the user's narrative identity — never write one.
fn propose_narrative_identity_tool() -> ToolConfig {
    ToolConfig {
        id: "propose_narrative_identity_edit".to_string(),
        name: "Propose identity note".to_string(),
        description: "Suggest something for the user's narrative identity".to_string(),
        llm_description: r#"Propose an addition to the user's narrative identity — the short document of who they are: values, aspirations, character, temperament, what they want.

This document goes into EVERY conversation you have with them, so it is the most consequential text in the system and it is not yours to edit. This tool does not change it. It leaves a note the user sees, with Add and Dismiss; nothing happens unless they choose.

Use it RARELY. Not for facts (those belong in the record), not for preferences about how to format an answer, and never for something they told you in passing. Use it when they say something durable about who they are or what they are for — the kind of thing that would still be true in a year and that would change how you understand a future question.

Say it in their own register, one or two sentences, as an addition to the document rather than a report about the conversation. Write "I" as the user, because the document is theirs.

Good: "I'd rather ship something imperfect early than polish in private."
Bad: "The user mentioned they prefer shipping early." (a report, not the document)
Bad: "The user's favourite editor is vim." (a fact, not an identity)

If you are unsure whether something qualifies, it does not."#.to_string(),
        parameters: serde_json::json!({
            "type": "object",
            "required": ["text", "why"],
            "properties": {
                "text": {
                    "type": "string",
                    "description": "The sentence(s) to add, written in the user's voice"
                },
                "why": {
                    "type": "string",
                    "description": "What in this conversation prompted it — shown to the user so they can judge"
                }
            }
        }),
        tool_type: ToolType::Builtin,
        category: ToolCategory::Edit,
        icon: "ri:compass-3-line".to_string(),
        display_order: 0,
        is_system: false,
    }
}

/// The narrative interview's ONE tool: turn the transcript into the person's
/// document and chapters. Interview-mode only (see get_tools_for_agent_mode);
/// is_system keeps it out of every other room's tool set.
fn write_it_up_tool() -> ToolConfig {
    ToolConfig {
        id: "write_it_up".to_string(),
        name: "Write it up and close the interview".to_string(),
        description: "Close the interview: arrange it into the person's document and chapters".to_string(),
        // The WHEN lives in the interview prompt ("Pacing and the close"),
        // in one place. This describes only the arguments and the outcome,
        // so the two cannot drift apart again.
        llm_description: r#"Close the narrative interview. A separate drafter reads the transcript and writes the "In your own words" document and the chapters. This runs ONCE and cannot be undone: the composer retires and the person cannot reply here afterwards. The interview prompt says when to call it; never compose the document yourself in the chat.

Arguments state your claim, and the box checks it:
- territories_covered: the territory numbers (1-6) the person has actually answered.
- their_words: the person's own words asking to close or saying yes to closing, verbatim. If they have not said so, do not call this; ask.
- close_early_confirmed: true only when territories are uncovered, you told the person which, and they said yes anyway.

A refused call is not an error: it returns the sentence to act on, and the interview goes on."#.to_string(),
        parameters: serde_json::json!({
            "type": "object",
            "properties": {
                "territories_covered": {
                    "type": "array",
                    "items": { "type": "integer", "minimum": 1, "maximum": 6 },
                    "description": "Territory numbers (1-6) the person has answered."
                },
                "their_words": {
                    "type": "string",
                    "description": "The person's own words asking to close or agreeing to it, verbatim."
                },
                "close_early_confirmed": {
                    "type": "boolean",
                    "description": "True only when territories are uncovered and the person, told which, said yes to closing anyway."
                }
            },
            "required": ["territories_covered", "their_words"]
        }),
        tool_type: ToolType::Builtin,
        category: ToolCategory::Edit,
        icon: "ri:quill-pen-line".to_string(),
        display_order: 0,
        is_system: true,
    }
}

/// Look at a file — actually look, not read a description of it.
fn read_asset_tool() -> ToolConfig {
    ToolConfig {
        id: "read_asset".to_string(),
        name: "Read file".to_string(),
        description: "Look at an image or file the user has stored".to_string(),
        llm_description: r#"Look at a file in the user's drive. For an image, the image itself comes back and you see it, exactly as if the user had pasted it into the conversation.

Use this when the user refers to a specific file — a screenshot, a photo, a diagram — and answering means seeing what is actually in it.

Take the id from the file's ref URL: `/drive/dr_abc123` means `file_id: "dr_abc123"`. Notebook members list theirs.

When to reach for this instead of searching:
- A member of the active notebook carries `text="none"` — nothing was extracted from it, so semantic_search cannot see inside it and will return nothing. That is not evidence the file lacks what the user is asking about. Look at it.
- The user says "this screenshot" / "that photo" / "the image in here". Look before answering.
- A document's extracted text is not enough and the layout matters.

Do NOT use it to sweep a folder hoping to find something — one file per call, when you know which file you want. Images are sent whole and cost real context.

If the file cannot be shown you are told why. Say that plainly to the user; never describe a file you were not shown."#.to_string(),
        parameters: serde_json::json!({
            "type": "object",
            "required": ["file_id"],
            "properties": {
                "file_id": {
                    "type": "string",
                    "description": "Drive file id, e.g. `dr_abc123`. A full `/drive/dr_abc123` ref URL is also accepted."
                }
            }
        }),
        tool_type: ToolType::Builtin,
        category: ToolCategory::Search,
        icon: "ri:image-line".to_string(),
        display_order: 0,
        is_system: false,
    }
}

/// Think tool - structured reasoning scratchpad
fn think_tool() -> ToolConfig {
    ToolConfig {
        id: "think".to_string(),
        name: "Think".to_string(),
        description: "Plan your approach before acting".to_string(),
        llm_description: r#"Use this tool to think through complex problems step-by-step before taking action.

When to use:
- Before multi-step tasks: Plan which tools to call and in what order
- When the question is ambiguous: Break down what the user is really asking
- For data analysis: Decide which tables to query and how to join results
- When combining sources: Plan how to merge SQL results with web search

Example thought for "How did my spending compare to last month?":
"I need to:
1. Query data_financial_transaction for this month's total spending by category
2. Query the same for last month
3. Compare the two and highlight significant changes
Let me start with this month's data."

This tool has no side effects - it just helps you organize your reasoning.
The user can see your thoughts, so be clear and concise."#.to_string(),
        parameters: serde_json::json!({
            "type": "object",
            "required": ["thought"],
            "properties": {
                "thought": {
                    "type": "string",
                    "description": "Your step-by-step reasoning or plan"
                }
            }
        }),
        tool_type: ToolType::Builtin,
        category: ToolCategory::Data,
        icon: "ri:lightbulb-line".to_string(),
        display_order: 0,
        is_system: false,
    }
}

/// Update Memory tool — persist notes across conversations
fn update_memory_tool() -> ToolConfig {
    ToolConfig {
        id: "update_memory".to_string(),
        name: "Memory".to_string(),
        description: "Keep, revise, or retire one memory that persists across conversations".to_string(),
        llm_description: r#"Your persistent memory: per-note rows in three lanes, shown to you in <memory> with ids. The user can read and edit every note in Settings — write nothing you would not want them to read, because they will.

Lanes:
- facts — durable facts of their world (names, places, projects, the dog)
- manner — how to speak and behave with them (concise, numbered lists, no preamble)
- practices — what they are holding to and how you help

Ops (one note per call):
- {op:"add", lane, content} — one durable fact, at most 500 chars. Near-duplicates merge into the existing note automatically.
- {op:"revise", note_id, content} — rewrite one of YOUR notes (ids are in <memory>).
- {op:"retire", note_id} — remove one of your notes when it stops being true.

A note marked [theirs] was edited by the user: it is in their words and you cannot revise or retire it. A full lane refuses adds until you retire something — few, true notes beat many stale ones.

What does NOT belong here: who they are — values, wounds, telos, self-understanding. That is their narrative identity, which only they author. Also never store secrets (passwords, keys, IDs)."#.to_string(),
        parameters: serde_json::json!({
            "type": "object",
            "properties": {
                "op": {
                    "type": "string",
                    "enum": ["add", "revise", "retire"],
                    "description": "What to do. Defaults to add."
                },
                "lane": {
                    "type": "string",
                    "enum": ["facts", "manner", "practices"],
                    "description": "Which lane (add only). Defaults to facts."
                },
                "note_id": {
                    "type": "integer",
                    "description": "The note to revise or retire — its id is shown in <memory> as (#id)."
                },
                "content": {
                    "type": "string",
                    "description": "The note text (add/revise). One durable fact, at most 500 characters."
                }
            }
        }),
        tool_type: ToolType::Builtin,
        category: ToolCategory::Data,
        icon: "ri:brain-line".to_string(),
        display_order: 1,
        is_system: false,
    }
}

/// Set User Name tool — update user's preferred name
fn set_user_name_tool() -> ToolConfig {
    ToolConfig {
        id: "set_user_name".to_string(),
        name: "Set User Name".to_string(),
        description: "Set the user's preferred name".to_string(),
        llm_description: r#"Set the user's preferred name. Use this when the user tells you their name or asks to change how you address them.

This updates the user's profile so their name persists across all future conversations."#.to_string(),
        parameters: serde_json::json!({
            "type": "object",
            "required": ["name"],
            "properties": {
                "name": {
                    "type": "string",
                    "description": "The user's preferred name"
                }
            }
        }),
        tool_type: ToolType::Builtin,
        category: ToolCategory::Data,
        icon: "ri:user-line".to_string(),
        display_order: 2,
        is_system: false,
    }
}

/// Set Assistant Name tool — update the AI's name
fn set_assistant_name_tool() -> ToolConfig {
    ToolConfig {
        id: "set_assistant_name".to_string(),
        name: "Set Assistant Name".to_string(),
        description: "Set the AI assistant's name".to_string(),
        llm_description: r#"Set your own name. Use this when the user picks or types a name for you during onboarding, or asks to rename you later.

This updates your name across all future conversations."#.to_string(),
        parameters: serde_json::json!({
            "type": "object",
            "required": ["name"],
            "properties": {
                "name": {
                    "type": "string",
                    "description": "The new name for the AI assistant"
                }
            }
        }),
        tool_type: ToolType::Builtin,
        category: ToolCategory::Data,
        icon: "ri:robot-line".to_string(),
        display_order: 3,
        is_system: false,
    }
}

/// Web Search tool (Exa AI)
fn web_search_tool() -> ToolConfig {
    ToolConfig {
        id: "web_search".to_string(),
        name: "Web Search".to_string(),
        description: "Search the web for current information".to_string(),
        llm_description: r#"Search the web for current information.

Use this tool when:
- User asks about recent events, news, or current information
- You need factual information you're uncertain about
- User explicitly asks to search or look something up
- Information might have changed since your training cutoff

Do NOT use when:
- User is asking about their personal data (use sql_query instead)
- The question is purely conversational or opinion-based

You synthesize the results yourself — the search returns evidence, not answers.

For time-sensitive topics (news, sports scores, odds, prices, live data) set
max_age_hours=1 so results are fresh rather than cached.

Set `objective` to what you are actually trying to learn whenever the query
alone is ambiguous — it disambiguates a short query and improves results.

Returns: Relevant web pages with titles, URLs, and the passages judged relevant."#.to_string(),
        parameters: serde_json::json!({
            "type": "object",
            "required": ["query"],
            "properties": {
                "query": {
                    "type": "string",
                    "description": "Search query - be specific and include relevant context"
                },
                "num_results": {
                    "type": "integer",
                    "description": "Number of results (1-10)",
                    "default": 5,
                    "minimum": 1,
                    "maximum": 10
                },
                "objective": {
                    "type": "string",
                    "description": "What you are trying to learn, in a sentence. Optional, but it disambiguates a short or broad query."
                },
                "max_age_hours": {
                    "type": "integer",
                    "description": "Freshness: max age (hours) of a cached result before re-crawling live. Use 1 for news/sports/odds/live data; omit for stable info.",
                    "minimum": 0
                }
            }
        }),
        tool_type: ToolType::Builtin,
        category: ToolCategory::Search,
        icon: "ri:search-line".to_string(),
        display_order: 1,
        is_system: false,
    }
}

/// Semantic Search tool — meaning-based retrieval across user data
fn semantic_search_tool() -> ToolConfig {
    ToolConfig {
        id: "semantic_search".to_string(),
        name: "Semantic Search".to_string(),
        description: "Search personal data by meaning".to_string(),
        llm_description: r#"Search the user's personal data using natural language meaning (vector similarity).

Use this tool when:
- Finding content by topic or meaning: "emails about the project review"
- Searching across multiple data types at once (emails, messages, calendar, documents)
- The user's question is vague or conceptual rather than precise

Do NOT use when:
- You need exact counts, aggregates, or analytics (use sql_query)
- You need to filter by specific dates, amounts, or structured fields (use sql_query)
- You're looking for external/web information (use web_search)

Think of it this way:
- semantic_search = "find things ABOUT X" (meaning-based)
- sql_query = "count/sum/filter X" (structure-based)

Searchable domains: document (uploaded PDFs/files), email, message, calendar, chat, transaction, transcription, page.

IMPORTANT: `domains` is an OPTIONAL narrowing filter. OMIT it to search everything —
including the user's uploaded documents. When the conversation is grounded in a
notebook, ALWAYS omit `domains`: the notebook already scopes the results, and an
extra domain filter will wrongly exclude the notebook's materials.

Returns results in relevance order (rank 1 = best match) with title, preview, author, and timestamp. Rank is relative order within THIS result set only — it says nothing about absolute match quality, so do not describe rank-1 as a strong match unless its content shows it.
Use sql_query with the returned record_ids to get full details.

RECALL TIP: for a broad, vague, or many-worded need, pass 2-4 phrasings in `queries`
(e.g. ["how my relationship with Sam changed", "tension or arguments with Sam", "good
times with Sam"]) — they run together and their results are fused, catching evidence a
single wording would miss. Use one phrasing for a precise lookup."#.to_string(),
        parameters: serde_json::json!({
            "type": "object",
            "properties": {
                "queries": {
                    "type": "array",
                    "items": { "type": "string" },
                    "minItems": 1,
                    "maxItems": 4,
                    "description": "One to four natural-language phrasings/facets of the SAME information need. Provide multiple to widen recall on broad or vague questions; use one for a precise lookup. Preferred over `query`."
                },
                "query": {
                    "type": "string",
                    "description": "Single natural-language query. Back-compat alias for a one-element `queries`; prefer `queries`."
                },
                "domains": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "OPTIONAL narrowing filter — omit to search all sources (including uploaded documents). Valid: document, email, message, calendar, chat, transaction, transcription, page. In a notebook-grounded chat, OMIT this."
                },
                "date_after": {
                    "type": "string",
                    "description": "Only return results after this date (ISO 8601, e.g., '2026-01-01')"
                },
                "date_before": {
                    "type": "string",
                    "description": "Only return results before this date (ISO 8601)"
                },
                "entities": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "Optional filter: only return results whose source references one of these resolved entity IDs (e.g. a person/place/org id like 'person_abc'). Use when the query is about a specific known entity — it's far more reliable than matching the name semantically."
                },
                "num_results": {
                    "type": "integer",
                    "description": "Number of results (1-50, default 10)",
                    "default": 10,
                    "minimum": 1,
                    "maximum": 50
                }
            }
        }),
        tool_type: ToolType::Builtin,
        category: ToolCategory::Search,
        icon: "ri:mind-map".to_string(),
        display_order: 2,
        is_system: false,
    }
}

/// SQL Query tool (read-only data access)
fn sql_query_tool() -> ToolConfig {
    ToolConfig {
        id: "sql_query".to_string(),
        name: "Query Data".to_string(),
        description: "Query user's personal data with SQL".to_string(),
        llm_description: r#"Execute read-only SQL queries against the user's personal data.

Operations:
- 'list_tables': Get all tables with row counts
- 'get_schema': Get detailed columns for specific table(s)
- 'query': Execute a SELECT query (read-only, max 200 rows)

================================================================================
DATA TABLES (raw ontology from connected sources)
================================================================================

HEALTH
  data_health_heart_rate     BPM measurements from wearables
  data_health_hrv            Heart rate variability (ms)
  data_health_steps          Step counts
  data_health_sleep          Sleep sessions with duration & quality
  data_health_workout        Exercise sessions (type, duration, calories)

LOCATION  
  data_location_point        Raw GPS coordinates (high volume)
  data_location_visit        Place visits with arrival/departure times

COMMUNICATION
  data_communication_email          Email messages (subject, body, from/to)
  data_communication_message        Chat messages (iMessage, SMS, etc.)
  data_communication_transcription  Voice/audio transcriptions

CALENDAR
  data_calendar_event        Events with attendees, location, times

FINANCIAL (amounts stored in cents - divide by 100 for dollars)
  data_financial_account      Bank/credit/investment accounts
  data_financial_transaction  Purchases, transfers, payments
  data_financial_asset        Investment holdings (stocks, crypto)
  data_financial_liability    Loans, mortgages, debt

ACTIVITY
  data_activity_app_session     Desktop/mobile app usage sessions
  data_activity_web_browsing  Web browsing history

CONTENT
  data_content_document     Saved documents and notes
  data_content_conversation AI chat history (search artifact)
  data_content_bookmark     Saved/curated items (GitHub stars, bookmarks)

================================================================================
WIKI TABLES (entity resolution + temporal context)
================================================================================

ENTITIES (resolved nouns in user's life)
  wiki_people       People with names, emails, relationship info
  wiki_places       Places with name, address, coordinates, visit stats
  wiki_orgs         Organizations with type, role, interaction history

TEMPORAL (daily/yearly context)
  wiki_days         Day summaries with autobiography, context vector
  wiki_events       Timeline events within a day

REFERENCES
  entity_references Junction table linking entities to ontology records

================================================================================
NARRATIVE TABLES (life story structure — wiki_* prefix)
================================================================================

================================================================================
QUERY TIPS (PostgreSQL dialect)
================================================================================
- Use 'get_schema' to see columns before writing queries
- Date filter: WHERE occurred_at > now() - interval '7 days'
- Truncate to a period: date_trunc('month', now()), date_trunc('day', now())
- Cast a timestamp to a date: timestamp::date  (today = current_date)
- Financial: amount/100.0 for dollars
- JOIN data tables to wiki_* for resolved names
- Always LIMIT results (max 200)

================================================================================
EXAMPLE QUERIES
================================================================================

-- Spending by category this month
SELECT category, SUM(amount)/100.0 as dollars, COUNT(*) as txns
FROM data_financial_transaction
WHERE occurred_at >= date_trunc('month', now())
GROUP BY category ORDER BY dollars DESC

-- Most contacted people this week
SELECT wp.name, COUNT(*) as messages
FROM data_communication_message m
JOIN wiki_people wp ON m.sender_url = wp.url OR m.recipient_url = wp.url
WHERE m.occurred_at > now() - interval '7 days'
GROUP BY wp.name ORDER BY messages DESC LIMIT 10

-- Sleep patterns last 2 weeks
SELECT started_at::date as day, duration_minutes, sleep_quality_score
FROM data_health_sleep
WHERE started_at > now() - interval '14 days'
ORDER BY occurred_at DESC

-- Calendar events today
SELECT title, started_at, ended_at, location
FROM data_calendar_event
WHERE started_at::date = current_date
ORDER BY started_at"#.to_string(),
        parameters: serde_json::json!({
            "type": "object",
            "required": ["operation"],
            "properties": {
                "operation": {
                    "type": "string",
                    "enum": ["query", "list_tables", "get_schema"],
                    "description": "Operation to perform"
                },
                "sql": {
                    "type": "string",
                    "description": "SQL query (required for 'query' operation). SELECT only, read-only."
                },
                "tables": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "Table name(s) to get schema for (required for 'get_schema' operation)"
                },
                "limit": {
                    "type": "integer",
                    "description": "Max rows to return (default 50, max 200)",
                    "default": 50,
                    "maximum": 200
                }
            }
        }),
        tool_type: ToolType::Builtin,
        category: ToolCategory::Data,
        icon: "ri:database-2-line".to_string(),
        display_order: 3,
        is_system: false,
    }
}

/// Code Interpreter tool - execute Python code in a sandbox
fn code_interpreter_tool() -> ToolConfig {
    ToolConfig {
        id: "code_interpreter".to_string(),
        name: "Python".to_string(),
        description: "Execute Python code for calculations and data analysis".to_string(),
        llm_description: r#"Execute Python code in a secure sandboxed environment.

Use this tool when you need to:
- Perform calculations, math, statistics, or numerical analysis
- Process, transform, or analyze data (CSV, JSON, etc.)
- Financial calculations (loans, mortgages, investments, IRR, NPV)
- Work with dates, times, or complex logic

Only stdout is returned, so print() your results. There is no way to return
files or images — describe results in text rather than saving charts.

Available packages:
- Python 3.12 standard library (math, statistics, datetime, json, csv, re, decimal, etc.)
- numpy - numerical computing, arrays, linear algebra
- numpy-financial - financial functions: pmt, fv, pv, irr, npv, nper, rate
- pandas - data analysis, DataFrames, CSV/JSON loading
- scipy - scientific computing, statistics, optimization
- python-dateutil - date parsing
- pytz - timezones

The code runs in an isolated sandbox with:
- No filesystem access (except a private /tmp for temporary files)
- No network access
- A memory limit and a timeout (default 60s, max 120s)

IMPORTANT: Use print() to output your results. The stdout will be returned to you.

Example - financial calculation (mortgage payment):
{
  "code": "import numpy_financial as npf\nloan = 400000\nrate = 0.065 / 12  # 6.5% annual -> monthly\nmonths = 30 * 12\npayment = npf.pmt(rate, months, -loan)\nprint(f'Monthly payment: ${payment:,.2f}')"
}

Example - data analysis with pandas:
{
  "code": "import pandas as pd\ndata = {'month': ['Jan', 'Feb', 'Mar'], 'sales': [100, 150, 120]}\ndf = pd.DataFrame(data)\nprint(f'Total: ${df.sales.sum()}')\nprint(f'Average: ${df.sales.mean():.2f}')\nprint(f'Best month: {df.loc[df.sales.idxmax(), \"month\"]}')"
}

Example - statistics with numpy:
{
  "code": "import numpy as np\ndata = [23, 45, 67, 32, 89, 54, 38]\nprint(f'Mean: {np.mean(data):.1f}')\nprint(f'Std Dev: {np.std(data):.1f}')\nprint(f'Correlation example: {np.corrcoef([1,2,3,4], [2,4,5,8])[0,1]:.3f}')"
}"#.to_string(),
        parameters: serde_json::json!({
            "type": "object",
            "required": ["code"],
            "properties": {
                "code": {
                    "type": "string",
                    "description": "Python code to execute. Use print() to output results."
                },
                "timeout": {
                    "type": "integer",
                    "description": "Execution timeout in seconds (default: 60, max: 120)",
                    "default": 60,
                    "minimum": 5,
                    "maximum": 120
                }
            }
        }),
        tool_type: ToolType::Builtin,
        category: ToolCategory::Data,
        icon: "ri:code-s-slash-line".to_string(),
        display_order: 4,
        is_system: false,
    }
}

/// Dispatch Subagents tool - orchestrator fan-out for Deep Research.
///
/// The main (orchestrator) agent calls this to spawn independent read-only research
/// workers in parallel. Each worker runs its own agent loop over the data + web tools,
/// then returns a compressed findings summary plus its sources. Workers cannot dispatch
/// further subagents (no recursion).
fn dispatch_subagents_tool() -> ToolConfig {
    ToolConfig {
        id: "dispatch_subagents".to_string(),
        name: "Dispatch Researchers".to_string(),
        description: "Spawn parallel research workers".to_string(),
        llm_description: r#"Spawn independent research workers that run IN PARALLEL, each chasing one line of inquiry, then return their findings for you to synthesize. This is your fan-out tool for deep research.

When to use:
- The question has several INDEPENDENT sub-questions that can be investigated separately (e.g. "query my spending trend", "check my calendar load", "find external base rates").
- You want a skeptic: dispatch one worker whose objective is to find evidence AGAINST your leading hypothesis.

How to use well:
- Dispatch the FEWEST workers that cover the independent questions — usually 2-4, never more than 5. Don't split one question into redundant workers.
- Each worker is READ-ONLY (sql_query, semantic_search, web_search, code_interpreter, think) and cannot dispatch further workers. Give each a self-contained objective — workers do NOT see the conversation, only their objective.
- Tell each worker to compute real statistics with code_interpreter where relevant, cite the specific records/sources it used, and return a CONCISE findings summary (not a transcript).
- Pick each worker's `model` by difficulty: "fast" for simple lookups, "balanced" (default) for normal research, "strong" for hard quantitative analysis.

After they return, synthesize their findings yourself — weigh agreements, surface disagreements, and never assert causation from correlation.

Example:
{
  "missions": [
    {"title": "Spending trend", "objective": "Query data_financial_transaction for monthly spending by category over the last 6 months. Use code_interpreter to compute the trend and flag the categories that grew most. Cite the rows.", "model": "balanced"},
    {"title": "Sleep correlation", "objective": "Pull data_health_sleep and monthly spending for the same period. Compute the correlation with code_interpreter. Report n and how weak/strong it is. Cite sources.", "model": "strong"},
    {"title": "Counter-evidence", "objective": "Argue against the idea that spending rose due to one cause. Look for confounders (seasonality, one-off purchases, income changes) in the data. Cite what you find.", "model": "balanced"}
  ]
}"#.to_string(),
        parameters: serde_json::json!({
            "type": "object",
            "required": ["missions"],
            "properties": {
                "missions": {
                    "type": "array",
                    "description": "1-5 independent research missions to run in parallel.",
                    "minItems": 1,
                    "maxItems": 5,
                    "items": {
                        "type": "object",
                        "required": ["title", "objective"],
                        "properties": {
                            "title": {
                                "type": "string",
                                "description": "Short label shown in the live panel, e.g. 'Sleep vs spending'."
                            },
                            "objective": {
                                "type": "string",
                                "description": "Self-contained instructions: what to find, which data/web to use, what to compute, what to cite. The worker sees only this."
                            },
                            "model": {
                                "type": "string",
                                "enum": ["fast", "balanced", "strong"],
                                "description": "Worker model tier by difficulty. Default balanced."
                            },
                            "style": {
                                "type": "string",
                                "enum": ["research", "voice"],
                                "description": "How the worker is framed. \"research\" (default): a read-only researcher that investigates and cites. \"voice\": a Council voice that speaks in first person as the perspective its objective describes (no tools but think). Use \"voice\" only in Council mode."
                            }
                        }
                    }
                }
            }
        }),
        tool_type: ToolType::Builtin,
        category: ToolCategory::Data,
        icon: "ri:team-line".to_string(),
        display_order: 6,
        // Internal orchestration tool: excluded from Chat (which filters out system tools) but
        // included in Deep Research and Council via their explicit tool allow-lists.
        is_system: true,
    }
}

/// Create Page tool - creates a new page with optional initial content
fn create_page_tool() -> ToolConfig {
    ToolConfig {
        id: "create_page".to_string(),
        name: "Create Page".to_string(),
        description: "Create a new page with content".to_string(),
        llm_description: r#"Create a new page with a title and optional initial content.

Use this tool when:
- User asks you to create a new page, document, or note
- User wants to start a new document from scratch
- You need to save information to a new page

Content supports markdown (headers, bold, lists, code blocks, etc.) and is rendered as rich text.

Example:
{
  "title": "Meeting Notes - January 29",
  "content": "Meeting Notes content here..."
}"#.to_string(),
        parameters: serde_json::json!({
            "type": "object",
            "required": ["title"],
            "properties": {
                "title": {
                    "type": "string",
                    "description": "Title for the new page"
                },
                "content": {
                    "type": "string",
                    "description": "Initial content for the page (markdown supported). Applied directly without review."
                }
            }
        }),
        tool_type: ToolType::Builtin,
        category: ToolCategory::Edit,
        icon: "ri:file-add-line".to_string(),
        display_order: 5,
        is_system: false,
    }
}

/// Get Page Content tool - reads current content of a page
fn get_page_content_tool() -> ToolConfig {
    ToolConfig {
        id: "get_page_content".to_string(),
        name: "Get Page Content".to_string(),
        description: "Read the current content of a page".to_string(),
        llm_description: r#"Read the current content of a page before editing.

Use this tool when:
- You need to see what's currently in a page before making edits
- User asks about the contents of their document
- You need context about the page to make good edits

ALWAYS call this before using edit_page so you know what text to find.

IMPORTANT - Extracting page_id:
When the user mentions a page it arrives as a route link, [Page Name](/page/page_abc123).
Extract the ID from the link: page_abc123 (the last path segment).
You MUST pass this page_id parameter when the user references a specific page.

Returns the page title, content, and content length."#.to_string(),
        parameters: serde_json::json!({
            "type": "object",
            "required": ["page_id"],
            "properties": {
                "page_id": {
                    "type": "string",
                    "description": "Page ID to read. Extract from the page link: [Name](/page/page_xxx) -> page_xxx"
                }
            }
        }),
        tool_type: ToolType::Builtin,
        category: ToolCategory::Edit,
        icon: "ri:file-text-line".to_string(),
        display_order: 6,
        is_system: false,
    }
}

/// Edit Page tool - applies edits using simple find/replace
fn edit_page_tool() -> ToolConfig {
    ToolConfig {
        id: "edit_page".to_string(),
        name: "Edit Page".to_string(),
        description: "Edit a page using find/replace".to_string(),
        llm_description: r#"Edit an existing page by finding text and replacing it. Can also rename the page title.

Use this tool when:
- User asks you to modify, update, or change their document
- User says "help me with this", "can you improve", "fix this"
- User asks to rename or change the title of a page
- You need to make changes to existing content

IMPORTANT: Call get_page_content FIRST to see the current document!

IMPORTANT - Extracting page_id:
When the user mentions a page it arrives as a route link, [Page Name](/page/page_abc123).
Extract the ID from the link: page_abc123 (the last path segment).
You MUST pass this page_id parameter when the user references a specific page.

How it works:
1. Provide 'page_id' - extracted from the page link
2. Provide 'find' - the exact text to locate in the document
3. Provide 'replace' - the new text you want instead
4. Optionally provide 'title' - new title for the page

Changes are applied immediately via real-time sync.
The 'find' text matches against the page's plain text (formatting stripped). Use 'replace' with markdown to set formatting.

Example - changing a word:
{
  "page_id": "page_abc123",
  "find": "The quick brown fox",
  "replace": "The fast brown fox"
}

Example - renaming a page (use empty find/replace if only changing title):
{
  "page_id": "page_abc123",
  "title": "New Page Title",
  "find": "",
  "replace": ""
}

Example - full document rewrite (find empty string):
{
  "page_id": "page_abc123",
  "find": "",
  "replace": "Entirely new document content here"
}

Tips:
- Use enough context in 'find' to uniquely identify the location
- Keep 'find' as short as possible while still being unique
- For large changes, prefer fewer comprehensive edits over many small ones"#.to_string(),
        parameters: serde_json::json!({
            "type": "object",
            "required": ["page_id", "find", "replace"],
            "properties": {
                "page_id": {
                    "type": "string",
                    "description": "Page ID to edit. Extract from the page link: [Name](/page/page_xxx) -> page_xxx"
                },
                "title": {
                    "type": "string",
                    "description": "New title for the page. Only provide when renaming."
                },
                "find": {
                    "type": "string",
                    "description": "Text to find in the document. Use empty string for full document replacement or title-only changes."
                },
                "replace": {
                    "type": "string",
                    "description": "Replacement text. Supports markdown (headers, bold, lists, etc.) which is rendered as rich text."
                }
            }
        }),
        tool_type: ToolType::Builtin,
        category: ToolCategory::Edit,
        icon: "ri:edit-line".to_string(),
        display_order: 7,
        is_system: false,
    }
}

/// SQL Write tool — DML scoped to applet-owned schemas
fn sql_write_tool() -> ToolConfig {
    ToolConfig {
        id: "sql_write".to_string(),
        name: "SQL Write".to_string(),
        description: "Write to applet-owned tables".to_string(),
        llm_description: r#"Write to an applet's own tables (INSERT / UPDATE / DELETE). One statement per call.

Scope is enforced by the database role: only tables in applet_* schemas are writable — data_*, wiki_*, and app_* are read-only (use sql_query to read them). Create applet tables via setup_applet's schema_sql first.

Use this to log entries into a tracker ("log lunch: 650 kcal" -> INSERT INTO applet_calorie_tracker.meals ...), correct or delete rows the user asks about, or maintain an applet's rollup tables during its runs.

Add RETURNING to get rows back (capped at 500); otherwise the result is rows_affected."#.to_string(),
        parameters: serde_json::json!({
            "type": "object",
            "required": ["sql"],
            "properties": {
                "sql": {
                    "type": "string",
                    "description": "One INSERT/UPDATE/DELETE statement targeting an applet_* schema table"
                }
            }
        }),
        tool_type: ToolType::Builtin,
        category: ToolCategory::Data,
        icon: "ri:database-2-line".to_string(),
        display_order: 7,
        is_system: false,
    }
}

/// Setup Applet tool — materialize a chat-authored applet as a folder
fn setup_applet_tool() -> ToolConfig {
    ToolConfig {
        id: "setup_applet".to_string(),
        name: "Setup Applet".to_string(),
        description: "Create or update an applet".to_string(),
        llm_description: r#"Turn the user's intent into an applet — a small thing that runs for them: a scheduled task, a one-off reminder, a self-ending monitor, a tracker with its own tables, or a dashboard face.

Use when the user asks for anything recurring, deferred, monitored, tracked, or dashboarded. Re-calling with the same name UPDATES that applet (this is also how you edit one). Before writing SQL, check the real schema first with sql_query over information_schema (tables data_* / wiki_*) — never reference tables you haven't confirmed exist.

An applet needs EITHER an `agent` prompt OR a `face_html` (or both). Pick by what the user wants:
- A DASHBOARD / chart / "explorer" that just shows data = `face_html` ONLY. The face reads data itself with virtues.query — do NOT add an agent; it has no server-side run and needs no prompt.
- A REMINDER / digest / examen / monitor that DOES something each run (writes a page, posts a message, computes) = `agent` (+ schedule/condition). No face unless they also want a view.
- A TRACKER the user feeds = `schema_sql` + `face_html`; add an `agent` only if it should also summarize on a schedule.

WHAT THE APPLET CAN DO AT RUNTIME (its prompt may only rely on these):
- read data: sql_query (read-only) · semantic_search · web_search (queries only — it CANNOT fetch URLs/feeds)
- deliver to the user: its run result posts back into this chat
- keep notes: update_applet_memory · write pages: create_page / edit_page
- own tables: anything you create via schema_sql (schema applet_<slug>), written at runtime with sql_write
- its face reads data via virtues.query(sql) (read-only)
If the ask needs a verb not listed (send email, fetch a URL, react to incoming messages): decompose it, or decline honestly and offer the nearest real alternative. Never write a prompt that pretends a tool exists.

GATE: applets with a schedule/api/webhook trigger are created DISABLED — tell the user to review and enable on the applet page. You cannot enable them.

Parameters:
- name (required): short name. slug = lowercased name with _ (e.g. "Calorie Tracker" -> calorie_tracker); its tables live in schema applet_<slug>.
- description (required): ONE sentence of the user's intent — shown as the applet's headline.
- agent (required): the runtime prompt. It runs with a kickoff message "Run your action instruction now." and NO chat history — write it self-contained.
- schedule: 6-field cron (sec min hour day month dow), box-LOCAL timezone. "0 0 9 25 7 *" = July 25, 9am. Date-anchored one-offs: nearest future occurrence + until="once".
- triggers: subset of cron/manual/tool/api/webhook/message. Defaults: with schedule ["cron","manual","tool"], else ["manual","tool"]. Add "message" when the user should be able to TALK to the applet — their text becomes the run's opening turn and the reply is the run result. This is the front door for trackers ("I had eggs" → sql_write a row) and anything the user feeds rather than schedules; check it before declining an ask for lack of input.
- condition: SQL boolean gating each run (skipped when false). Local data only.
- until: omit = forever · "once" = archive after first success · SQL boolean = archive when true after a success.
- schema_sql: ONE MIGRATION, not the whole schema. MUST target only schema applet_<slug>. First call: CREATE SCHEMA IF NOT EXISTS applet_<slug>; then your CREATE TABLEs. Later calls: submit ONLY what changed (ALTER TABLE applet_<slug>.t ADD COLUMN ...) — re-sending a CREATE TABLE IF NOT EXISTS with an extra column silently adds nothing, and the check will refuse it with the ALTER to write. Resubmitting identical DDL is recognized as already applied.
- face_html: a complete index.html for the applet's face (sandboxed iframe; include <link rel="stylesheet" href="virtues.css"> and <script src="virtues.js"></script>; read data with await virtues.query(sql); max 48KB).
- limits: protective ceilings, user-editable. Only these keys are enforced; any other key is a check failure, because a stored-but-ignored limit reads as protection and is not:
    max_llm_cost         — DOLLARS, ceiling on model spend within one run (0.25 = 25 cents). The run stops mid-loop and records `budget_exceeded`.
    max_llm_cost_per_day — DOLLARS, ceiling across a rolling 24h; checked before the run starts.
    max_runs_per_day     — whole runs in a rolling 24h (`max_runs` means this). Manual "Run now" is exempt.
    max_runs_per_hour    — whole runs in a rolling hour. Manual "Run now" is exempt.
    timeout_s            — SECONDS of wall clock for the subprocess phase.
  Set a spend ceiling on anything scheduled that calls a model: it is the difference between a cap and a hope.

If the result status is "check_failed", fix the findings and call again — nothing was created."#.to_string(),
        parameters: serde_json::json!({
            "type": "object",
            "required": ["name", "description"],
            "properties": {
                "name": { "type": "string" },
                "description": { "type": "string", "description": "One-sentence intent — the applet's headline" },
                "agent": { "type": "string", "description": "OPTIONAL. Self-contained runtime prompt — only for applets that DO something each run. Omit for pure dashboards/views (face-only)." },
                "schedule": { "type": "string", "description": "6-field cron, box-local tz" },
                "triggers": {
                    "type": "array",
                    "items": { "type": "string", "enum": ["cron", "manual", "tool", "api", "webhook", "message"] }
                },
                "condition": { "type": "string", "description": "SQL boolean run gate" },
                "until": { "type": "string", "description": "forever (omit) | 'once' | SQL boolean" },
                "schema_sql": { "type": "string", "description": "ONE migration in schema applet_<slug> only. First call CREATEs; later calls submit only the change (ALTER TABLE ...)." },
                "face_html": { "type": "string", "description": "Complete face index.html (48KB max)" },
                "limits": {
                    "type": "object",
                    "description": "Enforced ceilings only. max_llm_cost / max_llm_cost_per_day in DOLLARS; max_runs_per_day (alias max_runs) / max_runs_per_hour as whole runs; timeout_s in seconds. Unknown keys fail the check.",
                    "properties": {
                        "max_llm_cost":         { "type": "number"  },
                        "max_llm_cost_per_day": { "type": "number"  },
                        "max_runs_per_day":     { "type": "integer" },
                        "max_runs_per_hour":    { "type": "integer" },
                        "timeout_s":            { "type": "integer" }
                    }
                }
            }
        }),
        tool_type: ToolType::Builtin,
        category: ToolCategory::Edit,
        icon: "ri:robot-2-line".to_string(),
        display_order: 8,
        is_system: false,
    }
}

/// List actions — lightweight catalog for chat-driven discovery
fn list_applets_tool() -> ToolConfig {
    ToolConfig {
        id: "list_applets".to_string(),
        name: "List Applets".to_string(),
        description: "List scheduled actions".to_string(),
        llm_description: r#"List the user's scheduled actions (both system and user-owned). Returns id, name, owner, enabled, schedule, triggers, and last_run for each.

Use this when:
- User asks "what automations do I have?"
- You need to find an action by name before editing/running it
- Before suggesting a new action, to check whether something similar already exists

Optional filters:
- owner: "system" or "user" (system = built-in, user = user-created)
- enabled: true/false
- trigger: "cron" | "manual" | "tool" | "api" | "webhook" | "message"
"#.to_string(),
        parameters: serde_json::json!({
            "type": "object",
            "properties": {
                "owner": { "type": "string", "enum": ["system", "user"] },
                "enabled": { "type": "boolean" },
                "trigger": { "type": "string", "enum": ["cron", "manual", "tool", "api", "webhook"] }
            }
        }),
        tool_type: ToolType::Builtin,
        category: ToolCategory::Data,
        icon: "ri:list-check".to_string(),
        display_order: 10,
        is_system: false,
    }
}

/// Get a single action's full details + recent runs
fn get_applet_tool() -> ToolConfig {
    ToolConfig {
        id: "get_applet".to_string(),
        name: "Get Applet".to_string(),
        description: "Fetch a single action".to_string(),
        llm_description: r#"Fetch a single action by id, including its full configuration (agent, schedule, triggers, condition, memory, config) and its last 10 runs with status + summary.

Use this when:
- User asks "what does this action do?"
- You need to read the current agent prompt before suggesting an edit
- You're debugging why an action is failing and need recent run errors
"#.to_string(),
        parameters: serde_json::json!({
            "type": "object",
            "required": ["id"],
            "properties": {
                "id": { "type": "string", "description": "The action id (e.g. 'applet_user__weekly_planner')" }
            }
        }),
        tool_type: ToolType::Builtin,
        category: ToolCategory::Data,
        icon: "ri:file-search-line".to_string(),
        display_order: 11,
        is_system: false,
    }
}

/// Edit an existing action — partial update with system-owner guard
fn edit_applet_tool() -> ToolConfig {
    ToolConfig {
        id: "edit_applet".to_string(),
        name: "Edit Applet".to_string(),
        description: "Update an action's configuration".to_string(),
        llm_description: r#"Update one or more fields on an existing action. Send only the fields you want to change as a `patch` object.

Editable fields:
- name (user rows only)
- agent (user rows only; the LLM prompt)
- schedule (nullable — set to null to remove)
- enabled (bool)
- config (object — full replace)
- condition (nullable SQL expression; user rows only)
- triggers (array of cron|manual|tool|api|webhook; user rows only)
- memory (nullable markdown scratchpad)

System-owned rows (built-in pipelines like day_summary_eod) only accept: enabled, schedule, config, memory. Attempting to edit other fields on a system row will error with a clear message.

Use this when the user asks to:
- Change an action's prompt
- Reschedule it
- Disable/enable it
- Update its memory"#.to_string(),
        parameters: serde_json::json!({
            "type": "object",
            "required": ["id", "patch"],
            "properties": {
                "id": { "type": "string" },
                "patch": {
                    "type": "object",
                    "description": "Fields to update. Unknown fields are rejected.",
                    "properties": {
                        "name": { "type": "string" },
                        "agent": { "type": ["string", "null"] },
                        "schedule": { "type": ["string", "null"] },
                        "enabled": { "type": "boolean" },
                        "config": { "type": "object" },
                        "condition": { "type": ["string", "null"] },
                        "triggers": { "type": "array", "items": { "type": "string" } },
                        "memory": { "type": ["string", "null"] }
                    }
                }
            }
        }),
        tool_type: ToolType::Builtin,
        category: ToolCategory::Edit,
        icon: "ri:edit-2-line".to_string(),
        display_order: 12,
        is_system: false,
    }
}

/// Delete a user-owned action
fn delete_applet_tool() -> ToolConfig {
    ToolConfig {
        id: "delete_applet".to_string(),
        name: "Delete Applet".to_string(),
        description: "Delete a user-owned action".to_string(),
        llm_description: r#"Delete an action by id. Only user-owned actions can be deleted — system rows (built-in pipelines like day_summary_eod, embedding_index, trash_purge) are protected and will return an error. Tell the user to disable them instead.

This is destructive. Confirm with the user before calling unless the request is explicit ("delete the weekly planner action")."#.to_string(),
        parameters: serde_json::json!({
            "type": "object",
            "required": ["id"],
            "properties": {
                "id": { "type": "string" }
            }
        }),
        tool_type: ToolType::Builtin,
        category: ToolCategory::Edit,
        icon: "ri:delete-bin-line".to_string(),
        display_order: 13,
        is_system: false,
    }
}

/// Manually run an action
fn run_applet_tool() -> ToolConfig {
    ToolConfig {
        id: "run_applet".to_string(),
        name: "Run Applet".to_string(),
        description: "Trigger an action to run now".to_string(),
        llm_description: r#"Manually dispatch an action to run immediately. The action must have `tool` in its triggers list.

Returns a run_id and final status (success / skipped / error / forbidden / not_found). For agent actions, `summary` contains the LLM's final message; for subprocess actions, it's the binary's result string.

Optional parameters:
- payload: arbitrary JSON forwarded to the action as context
- date: YYYY-MM-DD override for date-scoped actions (e.g. day_summary_eod). Merged into the action's config.date.

Use when the user asks to "run it now" or "re-run yesterday's summary"."#.to_string(),
        parameters: serde_json::json!({
            "type": "object",
            "required": ["id"],
            "properties": {
                "id": { "type": "string" },
                "payload": { "description": "Arbitrary JSON forwarded to the action as context" },
                "date": { "type": "string", "description": "YYYY-MM-DD override for date-scoped actions" }
            }
        }),
        tool_type: ToolType::Builtin,
        category: ToolCategory::Edit,
        icon: "ri:play-circle-line".to_string(),
        display_order: 14,
        is_system: false,
    }
}

/// Update action memory — persistent markdown scratchpad for actions
fn update_applet_memory_tool() -> ToolConfig {
    ToolConfig {
        id: "update_applet_memory".to_string(),
        name: "Update Applet Memory".to_string(),
        description: "Update this action's persistent memory".to_string(),
        llm_description: r#"Save persistent memory that will be available on your next run.
Use this to remember facts, preferences, or state across runs. The content is markdown.
This tool is only available when running as an action."#.to_string(),
        parameters: serde_json::json!({
            "type": "object",
            "required": ["content"],
            "properties": {
                "content": {
                    "type": "string",
                    "description": "Markdown content to save as this action's memory. Replaces previous memory entirely."
                }
            }
        }),
        tool_type: ToolType::Builtin,
        category: ToolCategory::Edit,
        icon: "ri:brain-line".to_string(),
        display_order: 9,
        is_system: true,
    }
}

/// Dayline event tool — structured event CRUD for hourly/EOD actions
fn dayline_event_tool() -> ToolConfig {
    ToolConfig {
        id: "dayline_event".to_string(),
        name: "Dayline Event".to_string(),
        description: "Create or update dayline timeline events".to_string(),
        llm_description: r#"Create, continue, revise, or mark timeline events for the Dayline.

Actions:
- NEW: Create a new event. Requires: event_summary, start_time, end_time. Optional: topics, auto_label, source_ontologies.
- CONTINUE: Extend the current event. Requires: event_id, end_time. Optional: event_summary (updated), topics.
- REVISE: Modify a previous event (merge, split, update). Requires: event_id. Optional: event_summary, start_time, end_time, auto_label, topics.
- NO_DATA: Mark this time period as unknown. Requires: start_time, end_time.

Event summaries should be 1-3 factual sentences. Be specific: name people, places, apps, projects. Include all data sources, even minor ones."#.to_string(),
        parameters: serde_json::json!({
            "type": "object",
            "required": ["action"],
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["NEW", "CONTINUE", "REVISE", "NO_DATA"],
                    "description": "The action to perform"
                },
                "event_id": {
                    "type": "string",
                    "description": "ID of existing event (for CONTINUE, REVISE)"
                },
                "event_summary": {
                    "type": "string",
                    "description": "1-3 factual sentences describing the event"
                },
                "topics": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "Activity contexts (e.g., 'code review', 'commute', 'exercise')"
                },
                "start_time": {
                    "type": "string",
                    "description": "ISO 8601 timestamp for event start"
                },
                "end_time": {
                    "type": "string",
                    "description": "ISO 8601 timestamp for event end"
                },
                "auto_label": {
                    "type": "string",
                    "description": "Short label (e.g., 'Work session', 'Lunch')"
                },
                "source_ontologies": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "Ontology record IDs that informed this event"
                }
            }
        }),
        tool_type: ToolType::Builtin,
        category: ToolCategory::Edit,
        icon: "ri:timeline-line".to_string(),
        display_order: 9,
        is_system: true,
    }
}

/// Get Project Item tool - fetches the full content of a reference in an attached project.
fn get_project_item_tool() -> ToolConfig {
    ToolConfig {
        id: "get_project_item".to_string(),
        name: "Get Project Item".to_string(),
        description: "Read the full content of a referenced page, chat, space, or entity".to_string(),
        llm_description: r#"Fetch the full content of a referenced item by its url.

Use this when:
- The user @-mentions something — a markdown link like [name](/chat/chat_xxx),
  [name](/page/page_xxx), or [name](/space/space_xxx) in their message — and its
  content is RELEVANT to answering. The @-mention is a pointer; pull it in only
  if you actually need it.
- An attached_project lists items and you need one's full content.

Supported urls: /page/, /chat/, /notebook/, /person/, /place/, /org/.
Returns the item's content (page text, recent chat messages, space members,
person/place/org details). Don't fetch a reference you don't need."#.to_string(),
        parameters: serde_json::json!({
            "type": "object",
            "required": ["item_url"],
            "properties": {
                "item_url": {
                    "type": "string",
                    "description": "URL of the item to fetch, e.g. /page/page_xxx, /chat/chat_xxx, /space/space_xxx, /person/person_xxx"
                }
            }
        }),
        tool_type: ToolType::Builtin,
        category: ToolCategory::Data,
        icon: "ri:folder-open-line".to_string(),
        display_order: 5,
        is_system: false,
    }
}

/// Get default enabled tools configuration (for assistant profile)
pub fn default_enabled_tools() -> serde_json::Value {
    serde_json::json!({
        "think": true,
        "update_memory": true,
        "set_user_name": true,
        "set_assistant_name": true,
        "web_search": true,
        "semantic_search": true,
        "sql_query": true,
        "code_interpreter": true,
        "create_page": true,
        "get_page_content": true,
        "edit_page": true,
        "setup_applet": true,
        "get_project_item": true
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_tools() {
        let tools = default_tools();
        // No exact count: it only ever fires when someone adds a tool, which is
        // not a bug, so it gets bumped without thought — or, as happened here,
        // left red for eight tools running. What matters is below: every tool is
        // well-formed, and the load-bearing ones are present.
        assert!(!tools.is_empty(), "the registry ships tools");

        // Verify all tools have required fields
        for tool in &tools {
            assert!(!tool.id.is_empty());
            assert!(!tool.name.is_empty());
            assert!(!tool.llm_description.is_empty(), "LLM description is required");
            assert!(tool.parameters.is_object(), "Parameters must be JSON object");
            assert_eq!(tool.tool_type, ToolType::Builtin, "Registry tools should be builtin type");
        }

        // Verify specific tools exist
        let ids: Vec<&str> = tools.iter().map(|t| t.id.as_str()).collect();
        assert!(ids.contains(&"think"));
        assert!(ids.contains(&"update_memory"));
        assert!(ids.contains(&"set_user_name"));
        assert!(ids.contains(&"set_assistant_name"));
        assert!(ids.contains(&"web_search"));
        assert!(ids.contains(&"semantic_search"));
        assert!(ids.contains(&"sql_query"));
        assert!(ids.contains(&"code_interpreter"));
        assert!(ids.contains(&"create_page"));
        assert!(ids.contains(&"get_page_content"));
        assert!(ids.contains(&"edit_page"));
        assert!(ids.contains(&"setup_applet"));
        assert!(ids.contains(&"dayline_event"));
    }

    #[test]
    fn test_default_enabled_tools() {
        let enabled = default_enabled_tools();
        assert!(enabled.is_object());
        assert_eq!(enabled.get("think"), Some(&serde_json::json!(true)));
        assert_eq!(enabled.get("update_memory"), Some(&serde_json::json!(true)));
        assert_eq!(enabled.get("set_user_name"), Some(&serde_json::json!(true)));
        assert_eq!(enabled.get("set_assistant_name"), Some(&serde_json::json!(true)));
        assert_eq!(enabled.get("web_search"), Some(&serde_json::json!(true)));
        assert_eq!(enabled.get("semantic_search"), Some(&serde_json::json!(true)));
        assert_eq!(enabled.get("sql_query"), Some(&serde_json::json!(true)));
        assert_eq!(enabled.get("code_interpreter"), Some(&serde_json::json!(true)));
        assert_eq!(enabled.get("create_page"), Some(&serde_json::json!(true)));
        assert_eq!(enabled.get("get_page_content"), Some(&serde_json::json!(true)));
        assert_eq!(enabled.get("edit_page"), Some(&serde_json::json!(true)));
    }

    #[test]
    fn test_tool_parameters_have_type() {
        for tool in default_tools() {
            assert_eq!(
                tool.parameters.get("type"),
                Some(&serde_json::json!("object")),
                "Tool {} parameters should have type: object",
                tool.id
            );
        }
    }
}
