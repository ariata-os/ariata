//! Drafting "In your own words" from the interview answers.
//!
//! THE ONLY WRITER of narrative identity. There used to be a second —
//! `narrative_identity_gen`, which drafted a portrait from OBSERVED data
//! (recurring people, places, recent days) and overwrote this one's work.
//! Deleted 2026-08-26 as a doctrine violation, not just a race: values,
//! wounds and direction cannot be derived from behaviour. A machine guessing
//! at someone's telos from their message volume would be both wrong and, on
//! the subjects this covers, insulting. This module reads only what the
//! person wrote themselves.
//!
//! ONE ARTIFACT. The document ("In your own words") is the narrative identity
//! — for them to read and correct, and injected into every conversation
//! directly (chat.rs::build_narrative_identity, paragraph-boundary truncated).
//! There is deliberately NO abridged "capsule" beside it: that existed
//! (2026-08 → 2026-09-01, wiki_narrative_identity + a follower that re-derived
//! it on document edits) and was killed on Adam's ruling — two versions of one
//! identity meant NI and NI-lite drifting, and the abridger was caught
//! INVENTING standing directives ("be direct, don't go easy") that would have
//! silently steered every conversation. One artifact, read whole.
//!
//! Beside the document, the interview's one STRUCTURED output: wiki_chapters
//! (the gapless partition of their life), each chapter seeded with a wiki
//! article holding their own words about that era.
//!
//! THE DRAFT IS A MIRROR, NOT A VERDICT. It arranges what someone wrote and
//! hands it back for correction — in the FIRST PERSON, because it is their
//! account and reads as one they wrote (a portrait in "you" read as the
//! machine describing them back; settled 2026-09-04). Everything below is aimed at keeping it from
//! doing anything more than that — no diagnosis, no invention, no flattery, no
//! psychologising a person out of their own words.

use serde::Serialize;
use sqlx::PgPool;

use crate::error::{Error, Result};

const SYSTEM_PROMPT: &str = r#"You are arranging a person's own words into a document they will read and correct. It is called "In your own words", and that is the standard: their words, ordered — not your reading of them.

You are given the transcript of an interview about their life — the chapters of it, what makes them unlike others, who they admire, what pulls at them, what they believe, and whatever else they offered. THE INTERVIEWER'S WORDS ARE SCAFFOLDING, NEVER MATERIAL: nothing the interviewer said may appear in the document, be paraphrased into it, or shape a claim the person did not themselves make. Only the person's own turns are material.

WRITE THE DOCUMENT, then its rules. The document first. Three sections with these exact headings:

## Where I have been
## Who I am
## Where I am going

Rules for the document:
- FIRST PERSON THROUGHOUT. This is their account, written as they would write it: "I", "my", "me". Never "you", never "they", never their name as a subject. The reader is the person; the writer is the person. A stranger reading it should take it for something they wrote themselves.
- Use THEIR words. Keep their phrases, their names, their turns of speech. You are arranging, not translating. If a sentence of theirs is good, use it as it stands — they already said it in the first person.
- Present tense for who they are, past for what happened.
- Ground every sentence in something they actually wrote. If they did not say it, it does not appear. No inference about motives, no "this suggests", no filling gaps with what people are usually like.
- Never diagnose, never psychologise, never explain someone to themselves. "I lost my father in 2019 and the year after was the worst of my life" is right. "This loss clearly shapes my fear of commitment" is a violation — even in the first person, an interpretation they never made is not theirs.
- Leave the unanswered alone. If they skipped a question, that section is simply shorter. Do not note the absence, do not prompt them, do not compensate.
- Do not flatter, do not console, do not summarise their life as a lesson. No redemptive arc unless they wrote one.
- Aspirations are marked as aspirations. "I want to be more patient" — never "I am patient".
- Plain prose. No lists, no bold, no headings beyond the three above.

SECOND, after a line containing only ---RULES---: any instruction they gave about what NOT to raise, one per line, as a short imperative in their own terms ("never suggest bars", "do not mention my father unless I do"). These are drawn ONLY from what they explicitly asked for, anywhere in the transcript. Never invent one, never infer one from a sad story, never turn an observation into a rule. If they asked for nothing, write nothing after this line. Being told about a loss is not the same as being asked never to mention it.

Output the document, then ---RULES---, then the rules. Nothing else."#;

#[derive(Debug, Serialize)]
pub struct Draft {
    pub document: String,
    /// PROPOSED, not saved. Nothing here binds the assistant until the person
    /// confirms it — a rule the box invented and then obeyed would be worse
    /// than no rules at all, because it would be invisible and permanent.
    pub proposed_rules: Vec<String>,
}

/// The singleton subject id — the one narrative-identity article a box has.
/// Public because the chat prompt and the wiki identity page read the document
/// through it directly (there is no abridged copy to read instead).
pub const NAR_IDENTITY_ID: &str = "nar_identity_001";

/// The one interview chat. A fixed id, shared with the frontend, so the
/// conversation resumes forever and the drafter knows where to read.
pub const INTERVIEW_CHAT_ID: &str = "chat_narrative_interview";

/// Read the answers, write the document.
///
/// Refuses on an empty interview rather than producing a document about
/// nobody — an invented identity handed to someone as their own would be the
/// worst single output this product could generate.
///
/// ONE WRITER, ONCE. The document lands as a wiki article (a real page: the
/// editor, history, marginalia), and from that moment the person owns it —
/// this function never runs the model again while the article exists. That is
/// the entire ownership model: the machine may write this document only while
/// it is empty. (The platform enforces it too: a pool-only write to a
/// CRDT-backed page is silently discarded.) A repeat call hands back what
/// stands, so a lost response or a retry cannot double-spend or overwrite.
pub async fn draft_from_interview(pool: &PgPool) -> Result<Draft> {
    if let Some(prose) = crate::api::wiki_articles::get_article_prose(
        pool,
        "narrative_identity",
        NAR_IDENTITY_ID,
    )
    .await?
    {
        return Ok(Draft {
            document: prose.content,
            proposed_rules: Vec::new(),
        });
    }

    // The interview CHAT is the source — the whole transcript, in order. The
    // drafter's prompt firewalls the interviewer's turns (scaffolding, never
    // material), but they stay in the input because a person's answer often
    // only makes sense against the question it answered.
    let turns: Vec<(String, String)> = sqlx::query_as(
        "SELECT role, content FROM app_chat_messages \
         WHERE chat_id = $1 AND role IN ('user', 'assistant') \
           AND content <> '' \
         ORDER BY sequence_num ASC",
    )
    .bind(INTERVIEW_CHAT_ID)
    .fetch_all(pool)
    .await
    .map_err(|e| Error::Database(format!("read interview transcript: {e}")))?;

    let they_spoke = turns.iter().any(|(role, _)| role == "user");
    if !they_spoke {
        return Err(Error::Other(
            "nothing said yet — talk for a bit first".into(),
        ));
    }

    let mut prompt = String::from("The transcript:\n");
    for (role, content) in &turns {
        let speaker = if role == "user" { "THEM" } else { "INTERVIEWER" };
        prompt.push_str(&format!("\n{speaker}: {}\n", content.trim()));
    }

    let raw = call_model(pool, SYSTEM_PROMPT, &prompt, "narrative_draft").await?;
    let (document, proposed_rules) = split_draft(&raw);

    if document.trim().is_empty() {
        return Err(Error::ExternalApi("draft came back empty".into()));
    }

    // The document becomes the singleton narrative-identity article — a page
    // seeded with this markdown, which the person edits from here on.
    // (`create_article` is idempotent: a concurrent first draft cannot strand
    // a second page.)
    crate::api::wiki_articles::create_article(
        pool,
        "narrative_identity",
        NAR_IDENTITY_ID,
        "In your own words",
        &document,
    )
    .await?;

    tracing::info!(
        turns = turns.len(),
        document_chars = document.len(),
        "narrative draft written from the interview"
    );

    Ok(Draft {
        document,
        proposed_rules,
    })
}

/// Chapters extraction — the interview's ONE structured output. Reads the
/// same transcript as the drafter and emits JSON rows for wiki_chapters.
/// Same firewall: the interviewer's words are scaffolding, except that a
/// name or year the person confirmed in a playback counts as theirs.
const CHAPTERS_PROMPT: &str = r#"You are extracting the CHAPTERS of a person's life from an interview transcript — the eras they themselves named, with their rough years.

Output STRICT JSON only, no prose, no code fences: an array in chronological order, each element:
{"title": string|null, "start_year": int, "start_month": int|null, "end_year": int|null, "end_month": int|null, "changepoint": string|null, "summary": string|null}

Rules:
- Only chapters the person themselves gave. The interviewer's words are scaffolding — but a name or year the interviewer played back and the person confirmed counts as theirs. Anything never confirmed does not exist.
- "title": their name for the era, verbatim or near-verbatim. null ONLY for a stretch they deliberately left unnamed.
- "start_year"/"end_year": the rough year they said — "about '09" is 2009. When they wavered ("'08 or '09"), take the one they settled on, or the later mention. "end_year": null means the chapter is still running.
- "start_month"/"end_month": 1-12, ONLY when they gave a month ("Aug 2023" is start_year 2023, start_month 8; "July 20 2024" is 2024, 7). A bare year is null. Never invent a month to make eras line up.
- Several eras can begin in one year — keep every one of them, with its month. Two eras with the same start year AND the same month (or both no month) are one era: keep the one they said more about.
- "changepoint": what ENDED the era, in their words, if they said. Otherwise null.
- "summary": one or two of their own sentences about the era, kept close to verbatim. Otherwise null. Never diagnose, never interpret.
- If they gave no chapters at all, output [].
"#;

/// One extracted chapter, as the model returns it (rough years, their words).
///
/// Months are optional and honored when given. The first shape was
/// year-only, and `plan_chapters` deduped on the year — so a person who
/// answered in months ("camper van Aug 2023, married July 2024, new job Sep
/// 2024") silently lost every era but the first of each year. People do not
/// answer in years when a month is what they remember.
#[derive(Debug, serde::Deserialize)]
struct ExtractedChapter {
    title: Option<String>,
    start_year: i32,
    #[serde(default)]
    start_month: Option<u32>,
    end_year: Option<i32>,
    #[serde(default)]
    end_month: Option<u32>,
    changepoint: Option<String>,
    summary: Option<String>,
}

impl ExtractedChapter {
    /// The era's start as a date plus the precision the person gave.
    fn start(&self) -> Option<(chrono::NaiveDate, &'static str)> {
        date_at(self.start_year, self.start_month)
    }

    /// The era's end as they gave it, if they gave one.
    fn end(&self) -> Option<(chrono::NaiveDate, &'static str)> {
        self.end_year.and_then(|y| date_at(y, self.end_month))
    }
}

/// A rough date: the first of the month when a month was given, else the
/// first of the year — with the matching `wiki_chapters` precision so the
/// page can render "2023" rather than "January 2023" for a bare year.
fn date_at(year: i32, month: Option<u32>) -> Option<(chrono::NaiveDate, &'static str)> {
    match month {
        Some(m @ 1..=12) => chrono::NaiveDate::from_ymd_opt(year, m, 1).map(|d| (d, "month")),
        _ => chrono::NaiveDate::from_ymd_opt(year, 1, 1).map(|d| (d, "year")),
    }
}

/// What the `write_it_up` tool reports back to the interviewer.
#[derive(Debug, Serialize)]
pub struct FinalizeOutcome {
    /// The document's page — the frontend opens this beside the chat.
    pub document_page_id: String,
    /// True when the document already stood (one writer, once) and this call
    /// changed nothing about it.
    pub document_already_existed: bool,
    pub chapters_written: usize,
    /// Chapters ride best-effort beside the document: a failed extraction is
    /// reported here for the interviewer to relay, never fatal — the document
    /// must not be lost to a second model call's bad day.
    ///
    /// This is the OUTCOME in plain words, never the mechanism. The first
    /// version put the serde error here, and the interviewer relayed
    /// "a technical error parsing them" to a person who had just finished
    /// telling it their life. The detail goes to the log.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chapters_error: Option<String>,
    /// Always true: the room is over. Spelled out in the result so the
    /// interviewer's last message is grounded in what the screen now shows —
    /// the composer is gone and the person cannot reply here.
    pub interview_closed: bool,
    pub person_can_reply_here: bool,
}

/// What the interviewer claims when it asks to close.
///
/// The tool used to take no arguments, and the only guard on an irreversible
/// close was prose asking the model to keep a six-territory ledger in its head.
/// A tester wrote "thats more complete" after their chapter list and the
/// interviewer read it as "I'm done" and closed after territory one. The
/// arguments make the model state what it is claiming, and [`close_gate`]
/// checks the parts a server can check.
#[derive(Debug, Default, Clone, serde::Deserialize)]
pub struct CloseRequest {
    /// Territories (1..=6) the person has actually answered.
    #[serde(default)]
    pub territories_covered: Vec<u8>,
    /// The person's own words asking to close or saying yes to it, verbatim.
    #[serde(default)]
    pub their_words: String,
    /// Territories are uncovered AND the person, told which, said yes anyway.
    #[serde(default)]
    pub close_early_confirmed: bool,
}

impl CloseRequest {
    /// The person closed it themselves (the HTTP button): nothing to gate.
    pub fn by_person() -> Self {
        Self {
            territories_covered: (1..=6).collect(),
            their_words: "(closed from the page)".into(),
            close_early_confirmed: true,
        }
    }
}

/// How many territories the interview has.
pub const TERRITORIES: u8 = 6;

/// Fewer replies than this and six territories cannot have been covered;
/// even an early close needs an ask and an answer, so two is the floor.
const MIN_REPLIES_TO_CLOSE: usize = 2;

/// Refuse a close the server can see is premature. Returns the sentence the
/// interviewer should act on.
///
/// The server cannot verify what the person said, but it can count their
/// replies and it can hold the model to its own claim: uncovered territories
/// need the person's confirmed yes, and every close needs their words.
pub fn close_gate(req: &CloseRequest, their_replies: usize) -> std::result::Result<(), String> {
    if their_replies < MIN_REPLIES_TO_CLOSE {
        return Err(format!(
            "not yet: the person has replied {their_replies} time(s); this closes after the \
             territories are covered, or after they have asked to stop and said yes to closing early"
        ));
    }
    if req.their_words.trim().is_empty() {
        return Err("not yet: pass their_words — the person's own words asking to close or \
                    agreeing to it. If they have not said so, ask, do not close."
            .into());
    }
    let mut covered: Vec<u8> = req
        .territories_covered
        .iter()
        .copied()
        .filter(|t| (1..=TERRITORIES).contains(t))
        .collect();
    covered.sort_unstable();
    covered.dedup();
    let missing: Vec<String> = (1..=TERRITORIES)
        .filter(|t| !covered.contains(t))
        .map(|t| t.to_string())
        .collect();
    if !missing.is_empty() && !req.close_early_confirmed {
        return Err(format!(
            "not yet: territories {} are uncovered. Tell the person which in one sentence and ask \
             whether to close anyway; call again with close_early_confirmed=true only after they say yes.",
            missing.join(", ")
        ));
    }
    Ok(())
}

/// The person's replies so far in the interview transcript.
pub async fn their_reply_count(pool: &PgPool) -> Result<usize> {
    let n: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM app_chat_messages \
         WHERE chat_id = $1 AND role = 'user' AND content <> ''",
    )
    .bind(INTERVIEW_CHAT_ID)
    .fetch_one(pool)
    .await
    .map_err(|e| Error::Database(format!("count interview replies: {e}")))?;
    Ok(n.max(0) as usize)
}

/// The interview's finisher: the document (draft_from_interview), then the
/// structured chapters with their seeded articles. Everything downstream of
/// the transcript happens here, so the tool, the HTTP endpoint, and any
/// future caller share one path.
pub async fn finalize_interview(pool: &PgPool, req: &CloseRequest) -> Result<FinalizeOutcome> {
    let replies = their_reply_count(pool).await?;
    if let Err(why) = close_gate(req, replies) {
        return Err(Error::InvalidInput(why));
    }

    let existed_before = crate::api::wiki_articles::get_article(
        pool,
        "narrative_identity",
        NAR_IDENTITY_ID,
    )
    .await?
    .is_some();

    draft_from_interview(pool).await?;

    let article = crate::api::wiki_articles::get_article(pool, "narrative_identity", NAR_IDENTITY_ID)
        .await?
        .ok_or_else(|| Error::Other("document written but its article is missing".into()))?;

    let (chapters_written, chapters_error) = match chapters_from_interview(pool).await {
        Ok(n) => (n, None),
        Err(e) => {
            tracing::warn!(error = %e, "chapters extraction failed; document stands");
            (
                0,
                Some("the chapters were not written this time; the document is safe".to_string()),
            )
        }
    };

    Ok(FinalizeOutcome {
        document_page_id: article.page_id,
        document_already_existed: existed_before,
        chapters_written,
        chapters_error,
        interview_closed: true,
        person_can_reply_here: false,
    })
}

/// Extract the chapters and write wiki_chapters — one writer, once, same as
/// the document: rows present mean the person owns the table and the machine
/// never writes it again.
async fn chapters_from_interview(pool: &PgPool) -> Result<usize> {
    let already: bool = sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM wiki_chapters)")
        .fetch_one(pool)
        .await
        .map_err(|e| Error::Database(format!("check wiki_chapters: {e}")))?;
    if already {
        // The partition stands (one writer, once) — but backfill any chapter
        // whose article is missing, e.g. rows written before seeding existed.
        ensure_chapter_articles(pool).await;
        return Ok(0);
    }

    let turns: Vec<(String, String)> = sqlx::query_as(
        "SELECT role, content FROM app_chat_messages \
         WHERE chat_id = $1 AND role IN ('user', 'assistant') \
           AND content <> '' \
         ORDER BY sequence_num ASC",
    )
    .bind(INTERVIEW_CHAT_ID)
    .fetch_all(pool)
    .await
    .map_err(|e| Error::Database(format!("read interview transcript: {e}")))?;

    let mut prompt = String::from("The transcript:\n");
    for (role, content) in &turns {
        let speaker = if role == "user" { "THEM" } else { "INTERVIEWER" };
        prompt.push_str(&format!("\n{speaker}: {}\n", content.trim()));
    }

    let raw = call_model(pool, CHAPTERS_PROMPT, &prompt, "narrative_chapters").await?;
    let extracted: Vec<ExtractedChapter> = serde_json::from_str(json_array_in(&raw))
        .map_err(|e| Error::ExternalApi(format!("chapters came back unparseable: {e}")))?;

    let planned = plan_chapters(extracted);
    if planned.is_empty() {
        return Ok(0);
    }

    let mut tx = pool
        .begin()
        .await
        .map_err(|e| Error::Database(format!("begin chapters tx: {e}")))?;

    for ch in &planned {
        let (id, title) = (&ch.id, ch.title.as_deref());
        let (kind, started_at, ended_at) = (ch.kind, ch.started_at, ch.ended_at);
        sqlx::query(
            "INSERT INTO wiki_chapters \
               (id, kind, title, started_at, ended_at, started_precision, ended_precision, \
                changepoint, summary) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
        )
        .bind(id)
        .bind(kind)
        .bind(title)
        .bind(started_at)
        .bind(ended_at)
        .bind(ch.started_precision)
        .bind(ch.ended_precision)
        .bind(ch.changepoint.as_deref())
        .bind(ch.summary.as_deref())
        .execute(&mut *tx)
        .await
        .map_err(|e| Error::Database(format!("insert chapter {id}: {e}")))?;
    }

    tx.commit()
        .await
        .map_err(|e| Error::Database(format!("commit chapters: {e}")))?;

    ensure_chapter_articles(pool).await;

    tracing::info!(chapters = planned.len(), "wiki_chapters written from the interview");
    Ok(planned.len())
}

/// Each chapter is an ENTITY: seed its wiki article with the person's own
/// words about the era, so "the startup years" is a page they can open and
/// keep writing rather than a band on a drawing. Idempotent and best-effort —
/// create_article returns the existing article when one stands, and a failed
/// page never loses the partition. Runs on every finalize, so chapters
/// written before article seeding existed get their pages on the next call.
async fn ensure_chapter_articles(pool: &PgPool) {
    let rows = match list_chapters(pool).await {
        Ok(rows) => rows,
        Err(e) => {
            tracing::warn!(error = %e, "chapter article seeding skipped: {e}");
            return;
        }
    };
    for ch in rows {
        let title = ch
            .title
            .clone()
            .unwrap_or_else(|| span_label(ch.started_at, ch.ended_at));
        let mut content = String::new();
        if let Some(s) = ch.summary.as_deref().filter(|s| !s.trim().is_empty()) {
            content.push_str(s.trim());
        }
        if let Some(c) = ch.changepoint.as_deref().filter(|c| !c.trim().is_empty()) {
            if !content.is_empty() {
                content.push_str("\n\n");
            }
            content.push_str(&format!("What ended it: {}", c.trim()));
        }
        if let Err(e) =
            crate::api::wiki_articles::create_article(pool, "chapter", &ch.id, &title, &content)
                .await
        {
            tracing::warn!(chapter = %ch.id, error = %e, "chapter article seed failed; row stands");
        }
    }
}

/// "1997 – 2009", "2025 – now": the page title for an era the person left
/// unnamed. Never invents a name — the span is the only honest label.
fn span_label(started_at: chrono::NaiveDate, ended_at: Option<chrono::NaiveDate>) -> String {
    use chrono::Datelike;
    match ended_at {
        Some(e) => format!("{} – {}", started_at.year(), e.year()),
        None => format!("{} – now", started_at.year()),
    }
}

/// One chapter, as the wiki identity page lists them.
#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct ChapterRow {
    pub id: String,
    pub kind: String,
    pub title: Option<String>,
    pub started_at: chrono::NaiveDate,
    pub ended_at: Option<chrono::NaiveDate>,
    pub is_current: bool,
    pub changepoint: Option<String>,
    pub summary: Option<String>,
}

pub async fn list_chapters(pool: &PgPool) -> Result<Vec<ChapterRow>> {
    sqlx::query_as::<_, ChapterRow>(
        "SELECT id, kind, title, started_at, ended_at, is_current, changepoint, summary \
         FROM wiki_chapters ORDER BY started_at",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| Error::Database(format!("list chapters: {e}")))
}

pub async fn chapters_handler(
    axum::extract::State(state): axum::extract::State<crate::server::AppState>,
    _user: crate::middleware::auth::AuthUser,
) -> impl axum::response::IntoResponse {
    use axum::{response::IntoResponse as _, Json};
    match list_chapters(state.db.pool()).await {
        Ok(chapters) => (
            axum::http::StatusCode::OK,
            Json(serde_json::json!({ "chapters": chapters })),
        )
            .into_response(),
        Err(e) => (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

/// A chapter row ready to insert, spans already lawful.
#[derive(Debug)]
struct PlannedChapter {
    id: String,
    kind: &'static str,
    title: Option<String>,
    started_at: chrono::NaiveDate,
    started_precision: &'static str,
    ended_at: Option<chrono::NaiveDate>,
    ended_precision: Option<&'static str>,
    changepoint: Option<String>,
    summary: Option<String>,
}

/// Normalize extracted chapters into the gapless partition the table
/// promises: sort, drop junk and duplicate starts, then CHAIN each era's
/// end to the next era's start — the '[)' ranges then tile with no gap and no
/// overlap, which is what lets "which chapter was that in?" always have
/// exactly one answer. Only the last era keeps the end the person gave, and
/// only when it is after its start (otherwise it is still running).
///
/// A start is a DATE (year, or year and month), so several eras may begin in
/// one year. Only an identical start is a duplicate.
fn plan_chapters(extracted: Vec<ExtractedChapter>) -> Vec<PlannedChapter> {
    let mut dated: Vec<(chrono::NaiveDate, &'static str, ExtractedChapter)> = extracted
        .into_iter()
        .filter(|c| (1900..=2100).contains(&c.start_year))
        .filter_map(|c| c.start().map(|(d, p)| (d, p, c)))
        .collect();
    dated.sort_by_key(|(d, _, _)| *d);
    dated.dedup_by_key(|(d, _, _)| *d);

    let starts: Vec<(chrono::NaiveDate, &'static str)> =
        dated.iter().map(|(d, p, _)| (*d, *p)).collect();
    dated
        .into_iter()
        .enumerate()
        .map(|(i, (started_at, started_precision, ch))| {
            let (ended_at, ended_precision) = match starts.get(i + 1) {
                // The next era's start, at the next era's precision.
                Some((next, p)) => (Some(*next), Some(*p)),
                None => match ch.end().filter(|(e, _)| *e > started_at) {
                    Some((e, p)) => (Some(e), Some(p)),
                    None => (None, None),
                },
            };
            let title = ch
                .title
                .as_deref()
                .map(str::trim)
                .filter(|t| !t.is_empty())
                .map(str::to_string);
            let kind = if title.is_some() { "chapter" } else { "unknown" };
            let id = crate::ids::generate_id(
                crate::ids::CHAPTER_PREFIX,
                &[title.as_deref().unwrap_or("unknown"), &started_at.to_string()],
            );
            PlannedChapter {
                id,
                kind,
                title,
                started_at,
                started_precision,
                ended_at,
                ended_precision,
                changepoint: ch.changepoint,
                summary: ch.summary,
            }
        })
        .collect()
}

/// The JSON array inside a model reply: strict JSON was asked for, but a
/// model that wraps it in a fence or a sentence should not cost the person
/// their chapters. Anything before the first '[' and after the last ']' is
/// dropped; a reply with no array at all is returned as-is so serde names
/// the real problem.
fn json_array_in(raw: &str) -> &str {
    match (raw.find('['), raw.rfind(']')) {
        (Some(a), Some(b)) if b > a => &raw[a..=b],
        _ => raw.trim(),
    }
}

/// Split on the sentinel. A model that forgets it gives us a document and no
/// rules, which is recoverable.
fn split_draft(raw: &str) -> (String, Vec<String>) {
    let (doc, rules_block) = match raw.split_once("---RULES---") {
        Some((d, r)) => (d, r),
        None => (raw, ""),
    };
    let rules = rules_block
        .lines()
        .map(|l| l.trim().trim_start_matches(['-', '*', '•']).trim())
        .filter(|l| !l.is_empty())
        .map(str::to_string)
        .collect();
    (doc.trim().to_string(), rules)
}

/// Both writes go through the shared background helper on the CHAT slot.
///
/// The Chat slot, not Lite, for BOTH — including the chapters extraction that
/// a cheaper model could do — because the interview promised a no-retention
/// agreement in as many words, and the Lite slot honors the owner's
/// background pin, which may be a BYO endpoint or a provider with no ZDR.
/// The Chat slot is the Virtues-curated map and stays ZDR-capable. (See
/// `model_choice::honors_pin` for the same ruling on the interview turns.)
///
/// The cap is sized for THINKING plus answer. It was 4000 here, and the Chat
/// slot's model reasons inside `max_tokens`: on a 19-chapter transcript the
/// extraction spent the cap thinking and returned a truncated array, which
/// serde refused, and the person was told their chapters "didn't take" for a
/// reason that had nothing to do with what they said. Same failure
/// day_summary hit under the same cap (see its 16k note). Low effort: both
/// jobs are arrangement, not composition.
async fn call_model(
    pool: &PgPool,
    system_prompt: &str,
    user_prompt: &str,
    feature: &'static str,
) -> Result<String> {
    crate::virtues_api::completion::system_completion(
        pool,
        virtues_registry::models::ModelSlot::Chat,
        feature,
        system_prompt,
        user_prompt,
        16_000,
        0.3,
        Some("low"),
    )
    .await
}

// ─── handlers ───────────────────────────────────────────────────────────────

pub async fn draft_handler(
    axum::extract::State(state): axum::extract::State<crate::server::AppState>,
    _user: crate::middleware::auth::AuthUser,
) -> impl axum::response::IntoResponse {
    use axum::Json;
    use axum::response::IntoResponse as _;
    // Same path as the interview's write_it_up tool — document and chapters
    // together — so the API can never produce half a finalize. The person
    // pressed this themselves, so the close gate has nothing to ask.
    match finalize_interview(state.db.pool(), &CloseRequest::by_person()).await {
        Ok(d) => (axum::http::StatusCode::OK, Json(d)).into_response(),
        Err(e) => {
            tracing::warn!(error = %e, "narrative draft failed");
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": e.to_string() })),
            )
                .into_response()
        }
    }
}

// ─── rules ──────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct Rule {
    pub id: String,
    pub rule: String,
    pub kind: String,
    pub active: bool,
}

/// One confirmed rule.
///
/// `kind` arrives from the client because only the person knows which it is:
/// "never mention my father" and "help me hold my fast" are both rules and they
/// are opposites. It defaults to `avoid` — the reading that cannot cause harm if
/// a client omits it.
#[derive(Debug, serde::Deserialize)]
#[serde(untagged)]
pub enum RuleInput {
    /// The wire shape every client used before `kind` existed.
    Bare(String),
    Kinded {
        rule: String,
        #[serde(default = "default_kind")]
        kind: String,
    },
}

fn default_kind() -> String {
    "avoid".to_string()
}

impl RuleInput {
    fn parts(&self) -> (&str, &str) {
        match self {
            RuleInput::Bare(r) => (r.as_str(), "avoid"),
            RuleInput::Kinded { rule, kind } => (rule.as_str(), kind.as_str()),
        }
    }
}

#[derive(Debug, serde::Deserialize)]
pub struct SaveRules {
    /// Exactly what the person confirmed, in the wording they left it in. The
    /// proposals are thrown away; only this is stored.
    pub rules: Vec<RuleInput>,
}

pub async fn list_rules(pool: &PgPool) -> Result<Vec<Rule>> {
    sqlx::query_as::<_, Rule>(
        "SELECT id, rule, kind, active FROM wiki_rules WHERE active ORDER BY created_at",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| Error::Database(format!("list rules: {e}")))
}

pub async fn rules_handler(
    axum::extract::State(state): axum::extract::State<crate::server::AppState>,
    _user: crate::middleware::auth::AuthUser,
) -> impl axum::response::IntoResponse {
    use axum::{response::IntoResponse as _, Json};
    match list_rules(state.db.pool()).await {
        Ok(rules) => (
            axum::http::StatusCode::OK,
            Json(serde_json::json!({ "rules": rules })),
        )
            .into_response(),
        Err(e) => (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

/// Replace the rule set with exactly what was confirmed.
///
/// A replace rather than an append: this is the screen where someone reviews
/// every rule their box obeys, so leaving it must mean the list now says what
/// they saw. An append would let a rule they unticked survive invisibly, which
/// is the one failure this table exists to prevent.
pub async fn save_rules_handler(
    axum::extract::State(state): axum::extract::State<crate::server::AppState>,
    _user: crate::middleware::auth::AuthUser,
    axum::Json(req): axum::Json<SaveRules>,
) -> impl axum::response::IntoResponse {
    use axum::{response::IntoResponse as _, Json};
    let pool = state.db.pool();

    let mut tx = match pool.begin().await {
        Ok(t) => t,
        Err(e) => {
            return (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": e.to_string() })),
            )
                .into_response()
        }
    };

    if let Err(e) = sqlx::query("DELETE FROM wiki_rules").execute(&mut *tx).await {
        tracing::error!(error = %e, "rules: clear failed");
        return (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        )
            .into_response();
    }

    for (i, input) in req.rules.iter().enumerate() {
        let (rule, kind) = input.parts();
        let rule = rule.trim();
        if rule.is_empty() {
            continue;
        }
        // The column has a CHECK; sending anything else would fail the whole
        // transaction and lose the rules that were fine.
        let kind = if kind == "defend" { "defend" } else { "avoid" };
        if let Err(e) = sqlx::query("INSERT INTO wiki_rules (id, rule, kind) VALUES ($1, $2, $3)")
            .bind(format!("rule_{i:03}"))
            .bind(rule)
            .bind(kind)
            .execute(&mut *tx)
            .await
        {
            tracing::error!(error = %e, "rules: insert failed");
            return (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": e.to_string() })),
            )
                .into_response();
        }
    }

    if let Err(e) = tx.commit().await {
        tracing::error!(error = %e, "rules: commit failed");
        return (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        )
            .into_response();
    }

    tracing::info!(count = req.rules.len(), "rules saved");
    (
        axum::http::StatusCode::OK,
        Json(serde_json::json!({ "saved": req.rules.len() })),
    )
        .into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_document_and_rules() {
        let (doc, rules) = split_draft(
            "## Where I have been\nBoston.\n---RULES---\n- never suggest bars\n- do not mention my father",
        );
        assert_eq!(doc, "## Where I have been\nBoston.");
        assert_eq!(rules, vec!["never suggest bars", "do not mention my father"]);
    }

    #[test]
    fn a_missing_sentinel_keeps_the_document_and_proposes_nothing() {
        let (doc, rules) = split_draft("## Where I have been\nBoston.");
        assert_eq!(doc, "## Where I have been\nBoston.");
        assert!(rules.is_empty());
    }

    #[test]
    fn no_rules_asked_for_means_no_rules_proposed() {
        // Silence here must stay silence. Someone who wrote about a loss and
        // asked for nothing has not asked for a rule, and manufacturing one
        // would put words in their mouth that then govern the assistant.
        let (doc, rules) = split_draft("Doc.\n---RULES---\n");
        assert_eq!(doc, "Doc.");
        assert!(rules.is_empty());
    }

    #[test]
    fn a_legacy_core_sentinel_stays_out_of_the_rules() {
        // A model reproducing the OLD three-part contract must not leak the
        // abridgement into the document's rules; the stray section simply
        // rides along in the document, where the person can see and delete it.
        let (_, rules) = split_draft("Doc.\n---CORE---\nShort.\n---RULES---\n- one rule");
        assert_eq!(rules, vec!["one rule"]);
    }

    fn ch(title: Option<&str>, start: i32, end: Option<i32>) -> ExtractedChapter {
        ExtractedChapter {
            title: title.map(str::to_string),
            start_year: start,
            start_month: None,
            end_year: end,
            end_month: None,
            changepoint: None,
            summary: None,
        }
    }

    fn chm(title: &str, start: (i32, u32), end: Option<(i32, u32)>) -> ExtractedChapter {
        ExtractedChapter {
            title: Some(title.to_string()),
            start_year: start.0,
            start_month: Some(start.1),
            end_year: end.map(|e| e.0),
            end_month: end.map(|e| e.1),
            changepoint: None,
            summary: None,
        }
    }

    /// THE tester's list: three eras beginning in 2024 and two in 2026. The
    /// year-keyed dedup kept one of each and threw the rest away silently.
    #[test]
    fn several_eras_in_one_year_all_survive_when_months_are_given() {
        let planned = plan_chapters(vec![
            chm("camper van", (2023, 8), Some((2024, 5))),
            chm("marriage", (2024, 7), None),
            chm("tech sales", (2024, 9), Some((2026, 3))),
            chm("virtues", (2026, 3), None),
        ]);
        let titles: Vec<_> = planned.iter().map(|c| c.title.as_deref().unwrap()).collect();
        assert_eq!(titles, vec!["camper van", "marriage", "tech sales", "virtues"]);
        assert_eq!(planned[0].started_precision, "month");
        assert_eq!(planned[0].ended_at, Some(planned[1].started_at), "chained, not the given end");
        assert_eq!(planned[0].ended_precision, Some("month"));
        assert_eq!(planned[3].ended_at, None);
    }

    /// A bare year keeps year precision, so the page can say "2009" and not
    /// "January 2009" — the precision the person did not give is not invented.
    #[test]
    fn a_bare_year_stays_year_precision() {
        let planned = plan_chapters(vec![ch(Some("college"), 2006, Some(2009))]);
        assert_eq!(planned[0].started_precision, "year");
        assert_eq!(planned[0].ended_precision, Some("year"));
    }

    #[test]
    fn a_fenced_or_narrated_array_is_still_read() {
        assert_eq!(json_array_in("```json\n[1, 2]\n```"), "[1, 2]");
        assert_eq!(json_array_in("Here are the chapters: [] and that's all."), "[]");
        assert_eq!(json_array_in("no array here"), "no array here");
    }

    fn req(covered: &[u8], words: &str, early: bool) -> CloseRequest {
        CloseRequest {
            territories_covered: covered.to_vec(),
            their_words: words.into(),
            close_early_confirmed: early,
        }
    }

    /// THE regression: one territory covered, no words from the person
    /// asking, one reply in — refused, and the refusal names what is missing.
    #[test]
    fn a_close_after_the_first_territory_is_refused() {
        let err = close_gate(&req(&[1], "thats more complete", false), 4).unwrap_err();
        assert!(err.contains("2, 3, 4, 5, 6"), "{err}");
        assert!(close_gate(&req(&[1], "thats more complete", false), 1).is_err());
    }

    #[test]
    fn six_covered_with_their_yes_closes() {
        assert!(close_gate(&req(&[1, 2, 3, 4, 5, 6], "yes, write it up", false), 12).is_ok());
    }

    #[test]
    fn an_early_close_needs_their_confirmed_yes_and_their_words() {
        assert!(close_gate(&req(&[1, 2], "I'm done, close it", true), 5).is_ok());
        assert!(close_gate(&req(&[1, 2], "", true), 5).is_err(), "no words, no close");
        assert!(close_gate(&req(&[1, 2], "I'm done", false), 5).is_err(), "unconfirmed");
    }

    #[test]
    fn junk_territory_numbers_do_not_count_as_coverage() {
        assert!(close_gate(&req(&[1, 2, 3, 4, 5, 9], "yes", false), 12).is_err());
    }

    /// The partition promise: every era's end is the next era's start, so the
    /// '[)' ranges tile with no gap and no overlap regardless of what end
    /// years the model reported for the middle eras.
    #[test]
    fn chapters_chain_gaplessly() {
        let planned = plan_chapters(vec![
            ch(Some("college"), 2006, Some(2009)), // deliberately wrong end
            ch(Some("growing up"), 1997, Some(2008)),
            ch(Some("the startup years"), 2021, None),
        ]);
        assert_eq!(planned.len(), 3);
        assert_eq!(planned[0].title.as_deref(), Some("growing up"));
        assert_eq!(planned[0].ended_at, Some(planned[1].started_at));
        assert_eq!(planned[1].ended_at, Some(planned[2].started_at));
        assert_eq!(planned[2].ended_at, None, "last era with no end is still running");
    }

    /// The last era keeps a person-given end only when it is after its start;
    /// a degenerate or missing end means the chapter is current.
    #[test]
    fn a_closed_final_chapter_keeps_its_end() {
        let planned = plan_chapters(vec![ch(Some("abroad"), 2012, Some(2014))]);
        assert_eq!(planned[0].ended_at, chrono::NaiveDate::from_ymd_opt(2014, 1, 1));
        let degenerate = plan_chapters(vec![ch(Some("abroad"), 2012, Some(2012))]);
        assert_eq!(degenerate[0].ended_at, None);
    }

    /// An unnamed stretch is still part of the shape of a life: it stays in
    /// the partition as kind='unknown' with no title — never an invented one.
    #[test]
    fn an_unnamed_stretch_becomes_unknown() {
        let planned = plan_chapters(vec![ch(None, 2010, None), ch(Some("  "), 2015, None)]);
        assert!(planned.iter().all(|c| c.kind == "unknown" && c.title.is_none()));
    }

    /// Junk years and duplicate starts are dropped rather than inserted — a
    /// row the no-overlap constraint would refuse must never abort the whole
    /// set of good ones.
    #[test]
    fn junk_and_duplicate_starts_are_dropped() {
        let planned = plan_chapters(vec![
            ch(Some("real"), 2000, None),
            ch(Some("twin"), 2000, None),
            ch(Some("junk"), 12, None),
        ]);
        assert_eq!(planned.len(), 1);
        assert_eq!(planned[0].title.as_deref(), Some("real"));
    }
}
