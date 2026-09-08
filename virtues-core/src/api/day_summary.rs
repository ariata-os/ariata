//! Daily Summary Generation
//!
//! Gathers a day's structured data (sources, health aggregates, messages),
//! builds a text prompt, calls an LLM via virtues-api, and saves the result
//! as the day's ARTICLE PAGE with structured timeline events.

use chrono::{NaiveDate, TimeZone};
use chrono_tz::Tz;
use sqlx::PgPool;
use virtues_registry::models::ModelSlot;

use crate::error::Result;

use super::wiki::{
    create_temporal_event, delete_auto_events_for_day, get_day_sources, get_or_create_day,
    update_day, CreateTemporalEventRequest, DaySource, UpdateWikiDayRequest, WikiDay,
};

// ── Constants ────────────────────────────────────────────────────────────────

/// THE DETECTIVE — the Chat slot. Fuse the day's witnesses into events.
///
/// This is not grunt work and it is not prose. It is adjudication: every source
/// lies a little (calendars run long, GPS drifts, diarization miscounts), the
/// boundaries between events are latent, and the truth is the convergence of
/// noisy signals. That is a best-model job, so it runs on Chat — once, nightly,
/// on a completed day. It reads a compact DOSSIER (clean rollups: visits,
/// calendar, sleep, audio sessions, messages, health) rather than a raw dump, and
/// a few days of recent event labels to disambiguate the ambiguous stretches.
const SEGMENT_PROMPT: &str = r#"You are the event detective for a personal life-log. You are handed a DOSSIER of a single day's evidence and must reconstruct the day as a clean, gapless timeline of events. Output ONLY a raw JSON array — no markdown, no code fences, no prose, no commentary.

Output format:
[{"start": "HH:MM", "end": "HH:MM", "label": "Brief label", "summary": "1-3 factual sentences grounded in the dossier.", "topics": ["2-4 lowercase topical tags"]}]

HOW TO READ THE DOSSIER:
The dossier is a time-ordered list of the day's evidence, each item formatted for its kind. The kinds play different roles:
- **Location visits** are your PRIMARY boundary and your single strongest line: a change of place was MEASURED, not claimed.
- **Calendar events are PLANS, NOT EVIDENCE.** They are the weakest line in the dossier and are never a boundary on their own — see CALENDAR EVENTS ARE INTENTIONS below.
- **Device presence** (`[device]` lines) is a stretch the owner was demonstrably AT a machine — typing, clicking, or holding the screen awake. It is weak evidence of WHAT they were doing and strong evidence of WHERE THEY WERE NOT: a body at a keyboard is not a body at a dinner. Read the tail of the line — `screen locked` and `machine slept` mean they stopped; `collector stopped` means WE stopped watching and says nothing at all about them.
- **Sleep** spans are hard boundaries, BUT DO NOT EMIT YOUR OWN "Sleep" EVENT. The system stamps the authoritative sleep block separately from deterministic sleep-tracking data. Treat the overnight sleep span as a boundary and leave that stretch as "Unknown" — do not label it "Sleep" yourself.
- **Audio sessions** color the day and are CANDIDATE boundaries — weigh them, do not obey them. An audio session's content tells you what a stretch actually was (a conversation, a drive, airport noise, quiet work, sickness in bed) even when there is no location or calendar to anchor it. This is how you name a day spent entirely at home, or entirely on the road, where location never changes.
- **Messages** (`[messages]` lines) are a burst of a single thread, placed in time, with a few excerpts. `you:` is the owner; `them:` is the other person. Content here is the strongest evidence of INTENT in the whole dossier — it says what something was FOR, which no other source can. Read it that way, and read the two rules below before you use it: MESSAGES ARE PLANS, and DO NOT QUOTE PEOPLE.
- **Health** (heart rate, steps) is texture, never a boundary on its own.
- **Purchases** (`[purchase]` / `[refund]` lines) are precise evidence of what a stretch was — a meal, a shop, a checkout; the merchant names the activity.
- **Movement** (`[movement]` lines) tell you when, and how fast, the owner was actually travelling — see MOVEMENT AND TRANSIT.

CALENDAR EVENTS ARE INTENTIONS, NOT ATTENDANCE:
A `[calendar]` line records what was SCHEDULED. It is not evidence that anyone went. Treating it as evidence is the worst failure you can produce here — it writes a confident, detailed memory of a day that did not happen, and the owner cannot tell it from a real one. Hard rules:
- A calendar line may NEVER, on its own, name a stretch of the timeline. It needs corroboration from a TRACE — something that exists only if the owner's body was there: a `[visit]` at or near the event's place, `[movement]` toward it, a `[purchase]` there, or `[audio]` whose content actually matches the occasion.
- Corroborated → name the stretch by the calendar title. Uncorroborated → the stretch is "Unknown". An honest gap where a plan was is CORRECT output. Never fall back to the title.
- CONTRADICTED beats uncorroborated. If `[device]` puts the owner at a machine, or `[visit]` puts them somewhere else, for the bulk of a calendar block, THEY DID NOT GO. Name the stretch by what the device, visit and audio show, and do not mention the calendar entry at all.
- `SUBSCRIBED — someone else's calendar` means these are NOT the owner's plans; it is another household's or organisation's schedule that they can merely see. NEVER narrate a subscribed event as something the owner did, however well-corroborated the hour looks — evidence at that hour shows they were somewhere, not that they were HERE.
- `owner DECLINED` means they did not go. Full stop.
- `owner never replied`, or NO RSVP tag at all, means NOTHING in either direction — most events carry no RSVP, so its absence is not evidence. Do not read silence as attendance OR as absence.
- NEVER move detail from one source onto a block named by another. If the `[audio]` inside a calendar block is a piano and a dog at home, then the block IS a piano and a dog at home — it is not a scheduled dinner that happened to have piano music. Detail belongs to the source that recorded it, and borrowing it across sources is how a plan grows false sensory memories.

MESSAGES ARE PLANS TOO — THE CALENDAR RULES APPLY TO THEM:
A message arranging something is a PLAN, exactly as a calendar entry is, and every rule in the section above applies to it unchanged. "7:30 at the wine bar?" is an intention; it is not evidence that anyone went. This is the easiest mistake to make with message text, because a plan written in a person's own voice reads far more like a memory than a calendar row does — and it is the same failure, with better prose.
- A `[messages]` plan may NEVER, on its own, name a stretch of the timeline. It needs a TRACE at that hour — a `[visit]`, `[movement]` toward it, a `[purchase]` there, or `[audio]` that matches.
- Corroborated → name the stretch by what the plan says it was. Uncorroborated → "Unknown". Contradicted by a `[device]` run or a `[visit]` elsewhere → they did not go, and do not mention the plan.
- A plan for a LATER DAY is not evidence about this day at all. Ignore it.
- Messages that are not plans — the exchange itself, reacting to something, arranging nothing — are ordinary evidence of what a stretch was, like audio. The distinction is whether the text is about a FUTURE time.

DO NOT QUOTE PEOPLE:
You may READ every message in the dossier, both directions. You may NOT reproduce another person's words in a `summary`. Their text is here so you can cut and name the day correctly, not so it can be printed back. Write what the exchange was ABOUT ("arranging a coffee at the Hayes cafe for 07:30"), never what either party said in their own words. The owner's own words are the one exception, and even then prefer describing to quoting.

WHAT MAKES A BOUNDARY:
A boundary is a change of CONTEXT — where you are, what is scheduled, who you are with — never a change of TOPIC. A single conversation at one desk that drifts from work to lunch to weekend plans is ONE event, not three. Do not split on what is being talked about; split on the situation changing.

MOVEMENT AND TRANSIT:
The dossier includes **[movement]** lines — each a stretch the owner was actually moving, with distance and average pace (km/h) computed from GPS. That is ALL you know about travel: distance and speed, nothing more. Hard rules:
- NEVER name or infer a MODE of travel — not "walked", not "cycling", not "drove", not "tram"/"bus"/"train"/"flight"/"Uber", nothing. GPS pace cannot reliably tell a walk from a slow bike from a car in traffic, so ANY mode is a guess, and a guess is a fabrication. Describe travel ONLY by its distance and pace — "moved 1.3 km at ~18 km/h", "a 0.5 km trip" — and let the numbers stand.
- If a stretch has NO [movement] line, they were NOT travelling. Do not call it "transit", "commute", "a drive", or "a ride". A stationary window — a checkout, a wait, a call at a desk — is a STOP, not a trip; a purchase or a conversation there is what it was. If you cannot otherwise name it, it is "Unknown".
When a move has CONTENT (a conversation, a call), headline the span by that content, with the movement as the setting. Genuinely empty movement you may leave "Unknown"; the system marks it transit afterward.

WHAT AN EVENT IS:
Each event is one of exactly two kinds:
1. **A definitively understood block** — the dossier evidences a specific, nameable activity. The `label` is a short noun phrase (2-5 words). The `summary` is 1-3 plain factual sentences grounded in the actual evidence (place, who, durations, message counts, what the audio content shows, heart rate). No mood, no motivation, no invention.
2. **Unknown** — the dossier does not support a specific classification for this stretch. The `label` is exactly "Unknown" and the `summary` is omitted. Do NOT invent "Morning routine" / "Rest" / "Quiet time" to fill it. A genuine gap is more truthful than a guess.

RULES:
- The timeline MUST cover the full 24 hours: first event starts "00:00", last ends "24:00", contiguous, no gaps, no overlaps. Fill any stretch the evidence cannot name with a single "Unknown" block.
- Use 24-hour local time (HH:MM). Do not emit "Sleep" (the system owns it); leave overnight/rest stretches with no waking activity as "Unknown".
- Event count scales with evidence. A rich, mobile, talkative day might have 10-16 events; a quiet day might have 3-5. Do not pad to a minimum, do not fragment a coherent context to inflate the count.
- RECENT CONTEXT (if provided) is the last few days' event labels — use it only to disambiguate a stretch the dossier leaves ambiguous ("Unknown 18:00-19:00" that lines up with a nightly gym pattern), never to invent evidence this day lacks.
- The `summary` is the single most load-bearing field: the user reads it AND it is embedded to measure how novel the event was. Make it factual and specific — "Forty minutes at Blue Bottle on Hayes; six messages with Maya about the lease; heart rate mid-70s." Not "a pleasant coffee.""#;

/// THE ARTICLE — the Chat slot. The day's page, written from the record.
///
/// It reads the EVENTS, not the raw sources. It is NOT the log (the event timeline
/// already lists what happened when) — it is a lede plus body sections, an article
/// in the wiki's sense. The old sentence quota is gone; what replaced it as the
/// guard against invention is the evidence ceiling (every sentence must trace to
/// the dossier) and the anti-transcription bar on sections.
///
/// This deliberately dropped the old "elevated moves" (fabricated behavioural
/// fingerprints, forced quantified closers), the literary epigraph, and the W6H
/// data-quality block — all of which pushed the model to invent meaning the day did
/// not carry. See agents/record/event-timeline.md and the essay "A Day, Well Written": the
/// machine records what happened and hands the meaning back.
const NARRATE_PROMPT: &str = r#"You write the ARTICLE OF THE DAY for a personal wiki — the day's page, in the sense a wikipedia gives that word. It is NOT a log (the event timeline beneath the article already lists what happened, when) and it is not a summary squeezed into a sentence quota. It is prose about the day, written from the record, that read back weeks later drops the reader straight into it.

STRUCTURE — A LEDE, THEN BODY SECTIONS:
- Open with a LEDE: one short paragraph, no heading, that carries the shape of the day — the few lines that say what this day was. It sets the register for everything beneath it.
- Beneath the lede, write the article's body as sections under `## ` headings, where the evidence supports them. A section holds something that spans the day or connects its parts — a thread that ran through it, a conversation that turned, a piece of work that moved, the errand that broke the routine. Give each a plain, specific heading (the thing itself, not a category).
- THE SECTION BAR IS ANTI-TRANSCRIPTION: a section may only exist if it says something the timeline does not already say. Retelling the event list in paragraphs is the one way to fail here. If you cannot name what a section adds beyond the timeline, it does not exist.

LENGTH FOLLOWS THE EVIDENCE — NOT A QUOTA, AND NOT PADDING:
- A dense day whose record holds real threads earns a real article: a lede and two or three sections. An ordinary day earns a lede and perhaps one. A thin day earns a few lines and stops. There is no sentence ceiling and no floor — the ceiling is the evidence itself: every sentence must trace to something in the dossier.
- Never pad. An article stretched past its evidence is worse than a short one, because the stretching is where invention lives. A genuinely unremarkable day should say so plainly ("A day much like its neighbours — the office, home, the usual"), never be inflated into significance.

THE ONE HARD RULE — OBSERVE, NEVER INFER:
- Write only what the evidence shows. Warmth comes from OBSERVED detail (the low sun, the quiet train, the water) — NEVER from asserting an inner state. Do not write that the reader was "content", "productive", "happy", or "tired" as a feeling; do not say they did something "because" of a motive you are guessing at. State a departure or a goodbye as a fact ("the last coffee before she moves"); do not narrate how it felt.
- No inferred emotion, motive, meaning, or verdict. Never call a day good or bad, well-spent or wasted. Record what happened; hand the meaning back to the reader.
- This applies to the lede and to every section equally. No feelings, no motives, no verdicts, no invented sensory detail.
- If given RECENT DAYS (the last two weeks), use them only to recognise a real recurrence or a genuine first ("the first kayak in months", "the same thread as Saturday") — never to manufacture a pattern that isn't plainly there. Empty means a cold start: just say what the day was.

FORMAT:
- Plain, warm prose — a perceptive friend reflecting the day back, not a novelist. No lists, no bullet points, no epigraph, no closing metric, no "data quality" note. Headings only for body sections; the lede never has one.
- LINK entities: when you mention a person or place listed under "Entities you may link" below, link it by copying its exact markdown link, e.g. [Maya](/person/person_ab12). Link a given entity once, on first mention. Never invent a link or link anything not in that list. NEVER reproduce the list itself in the output — it is an instruction to you, not a line of the article.
- Second person, past tense. Output ONLY the prose (markdown), nothing else."#;

// ── Timezone helpers ─────────────────────────────────────────────────────────

/// Compute day boundaries in the user's timezone, converted to UTC RFC3339 strings.
/// Falls back to wide UTC window (00:00 → 12:00 next day) if timezone is None or invalid.
pub fn day_boundaries_utc(date: NaiveDate, timezone: Option<&str>) -> (String, String) {
    if let Some(tz_str) = timezone {
        if let Ok(tz) = tz_str.parse::<Tz>() {
            let start_local = date.and_hms_opt(0, 0, 0).unwrap();
            let end_local = date.succ_opt().unwrap().and_hms_opt(0, 0, 0).unwrap();

            let start_utc = tz
                .from_local_datetime(&start_local)
                .earliest()
                .map(|dt| dt.with_timezone(&chrono::Utc));
            let end_utc = tz
                .from_local_datetime(&end_local)
                .earliest()
                .map(|dt| dt.with_timezone(&chrono::Utc));

            if let (Some(s), Some(e)) = (start_utc, end_utc) {
                return (s.to_rfc3339(), e.to_rfc3339());
            }
        }
    }

    // Fallback: a true 24h UTC day when no/invalid timezone is available. (This
    // should rarely execute — home_timezone is seeded from the server's own
    // system clock; see agents/record/timezone-model.md.)
    let start = date.and_hms_opt(0, 0, 0).unwrap().and_utc();
    let end = date
        .succ_opt()
        .unwrap()
        .and_hms_opt(0, 0, 0)
        .unwrap()
        .and_utc();
    (start.to_rfc3339(), end.to_rfc3339())
}

// ── Public API ───────────────────────────────────────────────────────────────

/// Generate a daily summary from the day's data and save it as the day's article.
/// How many things you actually DID before a day is worth narrating.
///
/// Not a cost knob — a truth condition. You cannot segment a day into 8–16 events
/// from two records, and asking a model to try produces a plausible story about a
/// day that did not happen. Three is the floor at which the day has a shape of its
/// own rather than one the model supplies.
///
/// The right long-run measure is COVERAGE — how much of the waking day the events
/// actually account for — but coverage is computed from events, and events come
/// from this call. Counting what you did is the honest thing available before the
/// model runs.
const MIN_ACTIVATION_SOURCES: usize = 3;

/// How many SPANS a day needs before it has a shape of its own.
///
/// A `wiki_event` is a span, and the doctrine wants 8–16 of them in a day. You
/// cannot cut that out of one thing — and one thing is what most of history holds.
/// Measured on the real box, the distribution is not a gradient, it is a cliff:
///
/// ```text
///   13–373 spans   7 days    ← transcripts + visits: the week the collectors ran
///        2 spans   6 days    ← a couple of calendar entries
///        1 span   84 days    ← one calendar entry, sometimes an all-day one
/// ```
///
/// An all-day calendar event is 24 hours long and bounds nothing. A day with one
/// meeting in it is a day the model would have to invent 15 waking hours of.
///
/// Three separates the days that happened from the days we merely have a receipt
/// for. It is deliberately strict: the cost of skipping a real day is that it stays
/// unwritten until the collectors fill it in; the cost of narrating an empty one is
/// a confident, permanent, searchable account of a life nobody lived.
const MIN_SPANS: usize = 3;

pub async fn segment_day_events(pool: &PgPool, date: NaiveDate) -> Result<u32> {
    // 1. Gather structured sources (calendar, locations, transactions, chats, pages, etc.)
    let sources = get_day_sources(pool, date, None).await?;

    // 2. Compute date boundaries using the per-day "where the owner was" timezone
    //    (fixed at the day's start), falling back to the box's home_timezone.
    //    See agents/record/timezone-model.md.
    let home_tz = super::profile::get_timezone(pool)
        .await
        .unwrap_or(None)
        .unwrap_or_else(|| "UTC".to_string());
    let day_tz = crate::timezone::resolve_day_timezone(pool, date, &home_tz).await;
    let (start_str, end_str) = day_boundaries_utc(date, Some(&day_tz));
    let timezone: Option<String> = Some(day_tz);

    // 2b. Is this a day that HAPPENED, or a day you wore a watch?
    //
    // The old gate asked whether ANY ontology had data — and heart rate counts. So
    // a day whose only record was your pulse passed, and an LLM was asked to
    // narrate it. On a real box, 449 of 533 days hold nothing but passive data.
    // Every one of them was an Opus call away from a confident account of a day
    // nobody lived. That is not a cost bug; it is the same sin as building a day
    // around a broken query: FABRICATION.
    //
    // `is_activation_signal` has been declared on every ontology since the
    // beginning and read by nobody — its doc comment describes exactly this. A day
    // needs things you DID (a visit, a call, a meeting, a message) — not a sensor
    // noticing that you exist.
    let activation: Vec<&str> = virtues_registry::ontologies::activation_source_types();
    let spans: Vec<&str> = virtues_registry::ontologies::span_source_types();

    let acted = sources
        .iter()
        .filter(|s| activation.contains(&s.source_type.as_str()))
        .count();
    // Shape: something with a beginning and an end. An event IS a span, and you
    // cannot cut a day into spans using things that have no duration — a thousand
    // text messages never say when anything started. Asked to segment a day of pure
    // moments, the model invents the boundaries, and the boundaries are the one
    // thing it must not invent.
    let shaped = sources
        .iter()
        .filter(|s| spans.contains(&s.source_type.as_str()))
        .count();

    if acted < MIN_ACTIVATION_SOURCES || shaped == 0 {
        tracing::info!(
            date = %date,
            did = acted,
            spans = shaped,
            total_sources = sources.len(),
            "not enough of a day to narrate — skipping summary (no LLM call)"
        );
        return Ok(0);
    }

    // Never narrate a day that has not happened — checked in the day's OWN timezone,
    // not raw UTC. On a US box the UTC clock rolls to tomorrow at ~6–7pm local, so a
    // raw-UTC check calls the current, still-in-progress day "over" — it did exactly
    // that, running July 15 while it was still July 15 evening in Austin. Resolve
    // "today" in the day's timezone so the gate tracks the owner's clock. (Also still
    // rejects genuinely future dates — 146 calendar events on the box run out to 2029.)
    let today_in_tz = timezone
        .as_deref()
        .and_then(|s| s.parse::<Tz>().ok())
        .map(|tz| chrono::Utc::now().with_timezone(&tz).date_naive())
        .unwrap_or_else(|| chrono::Utc::now().date_naive());
    if date >= today_in_tz {
        tracing::info!(date = %date, "day is not over in the owner's timezone — nothing to summarise yet");
        return Ok(0);
    }

    // Idempotence — the same guard that let this run repeatedly without harm.
    //
    // Re-segmenting DELETES and re-creates every auto event, and an event's id is
    // content-addressed from its boundaries — so a re-cut mints new ids, strands
    // their index chunks, and throws away their scores. The fingerprint is the
    // day's source set; unchanged means untouched, and we return before spending a
    // best-model call. Checked before building the dossier so a settled day costs
    // nothing.
    let fingerprint = fingerprint_sources(&sources);
    let prior: Option<Option<String>> =
        sqlx::query_scalar("SELECT sources_fingerprint FROM wiki_days WHERE date = $1")
            .bind(date)
            .fetch_optional(pool)
            .await?;
    if prior.flatten().as_deref() == Some(fingerprint.as_str()) {
        tracing::debug!(date = %date, "sources unchanged since last segmentation — nothing to re-cut");
        return Ok(0);
    }

    // 3. Build the dossier — one compact, time-ordered feature list from the clean
    //    rollups (visits, calendar, sleep, audio sessions, chats, messages, health).
    //    High-cardinality streams (messages, and later email) are folded into bounded
    //    AGGREGATES (participant counts), never dumped row-by-row — that is what keeps
    //    the whole thing bounded without a blunt total-length truncation.
    let tz_for_display: Option<Tz> = timezone.as_deref().and_then(|s| s.parse().ok());
    let dossier = build_dossier(
        pool,
        date,
        &start_str,
        &end_str,
        timezone.as_deref(),
        tz_for_display.as_ref(),
    )
    .await;

    // 4. A light recency signal — the last few days' event labels — to disambiguate
    //    an ambiguous stretch. The detective's job is cutting, not remembering, so
    //    this stays small.
    let recent = recent_event_labels(pool, date, tz_for_display.as_ref()).await;

    let mut prompt = dossier;
    if !recent.is_empty() {
        prompt.push_str("\n\n## Recent days (for disambiguation only)\n");
        prompt.push_str(&recent);
    }

    tracing::info!(
        date = %date,
        prompt_chars = prompt.len(),
        source_count = sources.len(),
        "segmenting day into events (detective)"
    );

    // Chat slot: fusing noisy witnesses into a gapless timeline is adjudication,
    // not extraction — a best-model job, run once nightly on a completed day.
    // The completion helper resolves the slot's DEFAULT, never the profile's
    // pinned chat model — see virtues_api::completion for the ZDR class.
    let raw_response = call_virtues_api(pool, SEGMENT_PROMPT, ModelSlot::Chat, &prompt).await?;

    // An empty parse is a FAILURE, not a day with nothing in it.
    //
    // `store_structured_events` deletes every auto event for the day before
    // inserting, and the fingerprint written below means the day is never
    // re-cut. So one unparseable model response used to erase a day that had
    // fourteen good events, replace it with a single "Unknown" block, and mark
    // the work done — permanently, on a day the owner actually lived. The log
    // said `events = 0`, which reads exactly like a quiet day.
    //
    // Bail before the delete and before the fingerprint, and keep the raw
    // response at error level so the next run has something to look at. The day
    // stays un-segmented, which is what "we could not read the answer" should
    // look like, and the next scheduled pass tries again.
    let events = match parse_events_salvaging(&raw_response) {
        Some(ev) if !ev.is_empty() => ev,
        _ => {
            tracing::error!(
                date = %date,
                response = %raw_response.chars().take(2000).collect::<String>(),
                "could not parse any events from the model response — leaving the \
                 day un-segmented rather than erasing it"
            );
            return Err(crate::Error::Other(format!(
                "day {date}: no events could be parsed from the model response"
            )));
        }
    };
    let n = events.len() as u32;

    let day_stub = get_or_create_day(pool, date).await?;
    store_structured_events(pool, &day_stub, date, timezone.as_deref(), &events).await;

    sqlx::query(
        "UPDATE wiki_days SET sources_fingerprint = $1, \
         start_timezone = COALESCE(start_timezone, $2) WHERE date = $3",
    )
    .bind(&fingerprint)
    .bind(timezone.as_deref())
    .bind(date)
    .execute(pool)
    .await?;

    tracing::info!(date = %date, events = n, "day segmented");
    Ok(n)
}

/// What the day's sources looked like, so we can tell whether anything changed.
///
/// Count and latest timestamp per source type — enough to notice a new visit, a
/// new transcript, another hour of messages; cheap enough to compute every hour.
fn fingerprint_sources(sources: &[DaySource]) -> String {
    use std::collections::BTreeMap;
    let mut by_type: BTreeMap<&str, (usize, i64)> = BTreeMap::new();
    for s in sources {
        let e = by_type.entry(s.source_type.as_str()).or_insert((0, 0));
        e.0 += 1;
        e.1 = e.1.max(s.timestamp.timestamp());
    }
    let mut h = <sha2::Sha256 as sha2::Digest>::new();
    for (k, (n, ts)) in by_type {
        sha2::Digest::update(&mut h, format!("{k}:{n}:{ts};").as_bytes());
    }
    format!("{:x}", sha2::Digest::finalize(h))
}

/// How many events a day needs before it is worth WRITING about.
///
/// Your rule, and it was unstatable until now: the events did not exist until the
/// narration ran, so "narrate a day that has enough events" was a circle. Split
/// the call and it becomes a sentence.
///
/// A day the segmenter could only cut into two or three blocks — most of them
/// "Unknown" — has nothing for prose to be about. Asked to write it up anyway, the
/// model fills the silence, and what it fills it with is invention.
const MIN_EVENTS_TO_NARRATE: usize = 4;

/// NIGHTLY. Say what the day was.
///
/// Reads the EVENTS — not the raw sources. The prompt always claimed it did ("the
/// event timeline already does that") while being handed the sources anyway. Now
/// it is true: the narrative stands on the segmentation, which stands on the data.
///
/// Returns `None` when the day did not earn a story.
pub async fn narrate_day(pool: &PgPool, date: NaiveDate) -> Result<Option<WikiDay>> {
    let day = get_or_create_day(pool, date).await?;

    // The events, now carrying the SCORES that scoring computed between the
    // detective and here. `novelty_z` is what lets the narrative name the day's
    // standout — the whole reason scoring sits between the two agents.
    let events: Vec<DayEventRow> = sqlx::query_as(
        // `COALESCE(..., '(unlabeled)')` so a NULL label can never fail the String
        // decode and abort narration for the whole day.
        "SELECT COALESCE(user_label, auto_label, '(unlabeled)') AS label, event_summary, \
                started_at, ended_at, novelty_z \
         FROM wiki_events \
         WHERE day_id = $1 AND NOT is_unknown AND NOT user_hidden \
         ORDER BY started_at",
    )
    .bind(&day.id)
    .fetch_all(pool)
    .await?;

    // ALREADY WRITTEN, AND NOTHING ASKED FOR A REWRITE.
    //
    // `segment_day_events` returns early on an unchanged source fingerprint;
    // narration had no equivalent guard at all, so every caller — the catch-up
    // queue, the API, the CLI — paid for a fresh best-model call and wrote back
    // prose that was usually identical. `narrated_at` is the marker the queue
    // itself keys on, so honouring it here makes a repeat call free rather than
    // merely redundant.
    //
    // Deliberately NOT a content fingerprint: re-narrating a day whose events
    // genuinely changed is the right behaviour, and the caller that knows they
    // changed clears `narrated_at`. Guarding on prose alone would make a re-cut
    // day permanently unwritable.
    // `WikiDay` does not carry `narrated_at`, so read it directly rather than
    // widening a struct that a dozen surfaces deserialize.
    let already: Option<Option<chrono::DateTime<chrono::Utc>>> =
        sqlx::query_scalar("SELECT narrated_at FROM wiki_days WHERE date = $1")
            .bind(date)
            .fetch_optional(pool)
            .await?;
    if already.flatten().is_some() {
        tracing::debug!(date = %date, "already narrated — nothing asked for a rewrite");
        return Ok(None);
    }

    if events.len() < MIN_EVENTS_TO_NARRATE {
        tracing::info!(
            date = %date,
            events = events.len(),
            "not enough of a day to write about — skipping narration (no LLM call)"
        );
        return Ok(None);
    }

    let home_tz = super::profile::get_timezone(pool)
        .await
        .unwrap_or(None)
        .unwrap_or_else(|| "UTC".to_string());
    let day_tz = crate::timezone::resolve_day_timezone(pool, date, &home_tz).await;
    let tz: Option<Tz> = day_tz.parse().ok();
    let (start_str, end_str) = day_boundaries_utc(date, Some(&day_tz));

    // The most-novel event, if scoring has run — a SOFT hint the model may lean on
    // (cold until the baseline warms; brevity does the selection regardless). Kept
    // because reading `novelty_z` here is what the day-pipeline guard asserts:
    // scoring sits between the detective and the biography for a reason.
    let standout = events
        .iter()
        .enumerate()
        .filter_map(|(i, e)| e.novelty_z.map(|z| (i, z)))
        .max_by(|a, b| a.1.total_cmp(&b.1))
        .filter(|(_, z)| *z > 0.5)
        .map(|(i, _)| i);

    // The events, as the source material — a clean list. The biography does NOT
    // re-list them; brevity forces it to keep only what distinguished the day.
    let fmt = |t: &chrono::DateTime<chrono::Utc>| match tz {
        Some(z) => t.with_timezone(&z).format("%H:%M").to_string(),
        None => t.format("%H:%M").to_string(),
    };
    let mut prompt = format!(
        "# {}\n\n## The day's events (already logged — do NOT re-list them; write the memory of the day)\n\n",
        date.format("%A, %B %-d, %Y")
    );
    for (i, e) in events.iter().enumerate() {
        prompt.push_str(&format!("- {}–{} {}", fmt(&e.started_at), fmt(&e.ended_at), e.label));
        if let Some(s) = e.event_summary.as_deref().filter(|s| !s.trim().is_empty()) {
            prompt.push_str(&format!(": {s}"));
        }
        if Some(i) == standout {
            prompt.push_str("  (the most unusual beat of the day)");
        }
        prompt.push('\n');
    }

    if let Some(h) = build_health_snapshot(pool, &start_str, &end_str).await {
        append_section(&mut prompt, &h);
    }

    // The last 14 days — only to recognise a real recurrence or a genuine first
    // ("first kayak in months", "same thread as Saturday"), never to invent a
    // pattern. Empty on a cold start.
    let case_file = recent_event_case_file(pool, date, tz.as_ref()).await;
    if !case_file.is_empty() {
        prompt.push_str("\n## Recent days (the last two weeks)\n\n");
        prompt.push_str(&case_file);
    }

    // The day's resolved people + places, each as its exact ref-link, so the
    // biography can cite them the way chat/pages do — `[Name](/person/person_x)` —
    // which the day page renders as an entity pill (link-when-reading).
    let entities = day_entities_for_refs(pool, &start_str, &end_str).await;
    if !entities.is_empty() {
        prompt.push_str("\n## Entities you may link (copy the exact markdown link)\n");
        prompt.push_str(&entities.join("\n"));
        prompt.push('\n');
    }

    // Chat slot: this is the narrative call, and the only one left that earns it.
    // Slot DEFAULT via the completion helper, never the pinned chat model.
    let raw = call_virtues_api(pool, NARRATE_PROMPT, ModelSlot::Chat, &prompt).await?;
    let mut parsed = parse_virtues_api_response(&raw);
    parsed.diary = strip_prompt_echo(&parsed.diary);
    parsed.diary = unlink_uninvited_refs(&parsed.diary, &entities);

    let day = update_day(
        pool,
        date,
        UpdateWikiDayRequest {
            epigraph: parsed.epigraph,
            last_edited_by: Some("ai".to_string()),
            cover_image: None,
            start_timezone: Some(day_tz),
            data_quality: parsed
                .data_quality
                .as_deref()
                .and_then(|s| serde_json::from_str::<serde_json::Value>(s).ok()),
            snapshot: None,
        },
    )
    .await?;

    save_day_article(pool, &day.id, date, &parsed.diary).await?;

    sqlx::query("UPDATE wiki_days SET narrated_at = now() WHERE date = $1")
        .bind(date)
        .execute(pool)
        .await?;

    // Re-fetch: `day` was read before the article landed, so its `article`
    // field predates the write — returning it as-is showed callers (the CLI,
    // the API response) yesterday's prose under a "narrated" banner.
    let day = crate::api::wiki::get_or_create_day(pool, date).await?;
    Ok(Some(day))
}

/// Land the narration in the day's article page — the one prose store.
///
/// An article has exactly one pen at a time. A day article starts KEPT
/// (`auto_update = true`): the nightly narration is its maintenance, and the
/// record may rewrite it. Editing the article claims it — the Yjs layer
/// flips `auto_update` off on the first real user edit — and from then on
/// this writer refuses and stamps `dirty_at`; new evidence for a claimed day
/// belongs in notes, never in prose the user owns.
///
/// Even for a kept article, a pool-side rewrite is only safe while
/// `yjs_state IS NULL` — once a CRDT exists, an UPDATE of `content` would be
/// clobbered by the next debounced save. A kept-but-opened page is skipped
/// with a dirty stamp; a server-side (Yjs-aware) writer can pick it up.
async fn save_day_article(
    pool: &PgPool,
    day_id: &str,
    date: NaiveDate,
    prose: &str,
) -> Result<()> {
    if prose.trim().is_empty() {
        return Ok(());
    }

    let existing = crate::api::wiki_articles::get_article(pool, "day", day_id).await?;
    let Some(article) = existing else {
        // Title matches 0083's backfill: "3 March 2026". `%-d` drops the
        // zero-padding, as `FMDD` did in the migration's to_char.
        let title = date.format("%-d %B %Y").to_string();
        let created =
            crate::api::wiki_articles::create_article(pool, "day", day_id, &title, prose).await?;
        // Day articles are kept by default — narration IS their maintenance.
        // (Entity articles stay opt-in; their consent is the explicit toggle.)
        if let Err(e) = sqlx::query("UPDATE wiki_articles SET auto_update = true WHERE id = $1")
            .bind(&created.id)
            .execute(pool)
            .await
        {
            tracing::warn!(date = %date, error = %e, "could not mark the day article kept");
        }
        return Ok(());
    };

    if !article.auto_update {
        tracing::info!(
            date = %date,
            page_id = %article.page_id,
            "day article is claimed (yours) — narration files nothing; marked dirty"
        );
        sqlx::query("UPDATE wiki_articles SET dirty_at = now() WHERE id = $1")
            .bind(&article.id)
            .execute(pool)
            .await?;
        return Ok(());
    }

    let updated = sqlx::query(
        "UPDATE app_pages SET content = $1, updated_at = now() \
         WHERE id = $2 AND yjs_state IS NULL",
    )
    .bind(prose)
    .bind(&article.page_id)
    .execute(pool)
    .await?;

    if updated.rows_affected() > 0 {
        sqlx::query(
            "UPDATE wiki_articles SET last_written_at = now(), dirty_at = NULL WHERE id = $1",
        )
        .bind(&article.id)
        .execute(pool)
        .await?;
    } else {
        tracing::info!(
            date = %date,
            page_id = %article.page_id,
            "kept day article has a live CRDT — pool write would be clobbered; marked dirty"
        );
        sqlx::query("UPDATE wiki_articles SET dirty_at = now() WHERE id = $1")
            .bind(&article.id)
            .execute(pool)
            .await?;
    }
    Ok(())
}

// ── Section builders ─────────────────────────────────────────────────────────

/// A prompt section with a heading and body
struct PromptSection {
    heading: String,
    body: String,
}

/// Build health snapshot from aggregation queries
async fn build_health_snapshot(
    pool: &PgPool,
    start_str: &str,
    end_str: &str,
) -> Option<PromptSection> {
    let mut lines = Vec::new();

    // Heart rate
    let hr: Option<(Option<i32>, Option<i32>, Option<f64>, i32)> = sqlx::query_as(
        r#"
        SELECT MIN(bpm), MAX(bpm), ROUND(AVG(bpm)), COUNT(*)
        FROM data_health_heart_rate
        WHERE occurred_at >= $1::timestamptz AND occurred_at <= $2::timestamptz
        "#,
    )
    .bind(start_str)
    .bind(end_str)
    .fetch_optional(pool)
    .await
    .ok()
    .flatten();

    if let Some((Some(min_hr), Some(max_hr), Some(avg_hr), count)) = hr {
        if count > 0 {
            lines.push(format!(
                "- Heart rate: avg {:.0}, min {}, max {} ({} readings)",
                avg_hr, min_hr, max_hr, count
            ));
        }
    }

    // Steps
    let steps: Option<(Option<i64>,)> = sqlx::query_as(
        r#"
        SELECT SUM(step_count)
        FROM data_health_steps
        WHERE occurred_at >= $1::timestamptz AND occurred_at <= $2::timestamptz
        "#,
    )
    .bind(start_str)
    .bind(end_str)
    .fetch_optional(pool)
    .await
    .ok()
    .flatten();

    if let Some((Some(total_steps),)) = steps {
        if total_steps > 0 {
            lines.push(format!("- Steps: {}", total_steps));
        }
    }

    if lines.is_empty() {
        None
    } else {
        Some(PromptSection {
            heading: "Health Snapshot".to_string(),
            body: lines.join("\n"),
        })
    }
}

/// Append a section to the prompt string
fn append_section(prompt: &mut String, section: &PromptSection) {
    prompt.push_str(&format!("\n## {}\n{}\n", section.heading, section.body));
}

// ── The dossier ────────────────────────────────────────────────────────────────

/// An event row for narration. `novelty_z` is a soft selection hint (and the
/// scoring-reaches-the-biography guard); the biography leans on brevity, not scores.
#[derive(sqlx::FromRow)]
struct DayEventRow {
    label: String,
    event_summary: Option<String>,
    started_at: chrono::DateTime<chrono::Utc>,
    ended_at: chrono::DateTime<chrono::Utc>,
    novelty_z: Option<f64>,
}

/// The day's resolved people + places, each as its exact ref-link route, so the
/// biography can cite them the way chat/pages do — `[Name](/person/person_x)` — and
/// the day page renders them as entity pills.
async fn day_entities_for_refs(pool: &PgPool, start_str: &str, end_str: &str) -> Vec<String> {
    use sqlx::Row;
    let rows = sqlx::query(
        "SELECT 'person' AS kind, pe.id AS id, pe.name AS name \
         FROM wiki_refs er JOIN wiki_people pe ON pe.id = er.entity_id \
         WHERE er.entity_type = 'person' \
           AND er.occurred_at >= $1::timestamptz AND er.occurred_at <= $2::timestamptz \
         UNION \
         SELECT 'place', p.id, p.name \
         FROM wiki_refs er JOIN wiki_places p ON p.id = er.entity_id \
         WHERE er.entity_type = 'place' \
           AND er.occurred_at >= $1::timestamptz AND er.occurred_at <= $2::timestamptz \
         UNION \
         SELECT 'org', o.id, o.name \
         FROM wiki_refs er JOIN wiki_orgs o ON o.id = er.entity_id \
         WHERE er.entity_type = 'organization' \
           AND er.occurred_at >= $1::timestamptz AND er.occurred_at <= $2::timestamptz",
    )
    .bind(start_str)
    .bind(end_str)
    .fetch_all(pool)
    .await
    .unwrap_or_default();
    rows.iter()
        .filter_map(|r| {
            let kind: String = r.get("kind");
            let id: String = r.get("id");
            let name = r
                .try_get::<Option<String>, _>("name")
                .ok()
                .flatten()
                .filter(|s| !s.trim().is_empty())?;
            Some(format!("- [{name}](/{kind}/{id})"))
        })
        .collect()
}

/// Cap a free-text field to `n` chars, appending an ellipsis when it was clipped.
/// This is what bounds the dossier by construction — audio content especially.
fn cap(s: &str, n: usize) -> String {
    let t = s.trim();
    if t.chars().count() <= n {
        return t.to_string();
    }
    let mut out: String = t.chars().take(n).collect();
    out.push('…');
    out
}

/// Segment the day's GPS trace into MOVING stretches, computed from `speed` (never
/// stored — movement is the negative space between stays). A stretch is a run of
/// fixes above a walking-still threshold, coalescing brief pauses (a light, a
/// checkout) so one trip is one stretch, and kept only if it actually covered
/// ground. Keyed on the raw trace, NOT on visits, so a stretch with no clustered
/// visit (a whole evening at an unrecognised home) still gets grounded — and,
/// crucially, a mostly-stationary window (standing at a till) yields NO stretch, so
/// the detective has no license to call it transit. Returns
/// `(start, end, distance_km, avg_moving_kmh)`.
async fn day_movement_segments(
    pool: &PgPool,
    start_str: &str,
    end_str: &str,
) -> Vec<(
    chrono::DateTime<chrono::Utc>,
    chrono::DateTime<chrono::Utc>,
    f64,
    Option<f64>,
)> {
    use sqlx::Row;
    let rows = sqlx::query(
        "SELECT occurred_at, latitude, longitude, speed FROM data_location_point \
         WHERE occurred_at >= $1::timestamptz AND occurred_at <= $2::timestamptz \
         ORDER BY occurred_at",
    )
    .bind(start_str)
    .bind(end_str)
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    let pts: Vec<(chrono::DateTime<chrono::Utc>, f64, f64, Option<f64>)> = rows
        .iter()
        .map(|r| {
            (
                r.get::<chrono::DateTime<chrono::Utc>, _>("occurred_at"),
                r.get::<f64, _>("latitude"),
                r.get::<f64, _>("longitude"),
                r.try_get::<Option<f64>, _>("speed").ok().flatten(),
            )
        })
        .collect();

    const MOVING_MPS: f64 = 1.0; // ~3.6 km/h — above GPS jitter, still catches a walk
    const MERGE_GAP_S: i64 = 180; // fold still-pauses under 3 min into one trip
    const MIN_DIST_M: f64 = 150.0; // discard jitter that never really went anywhere

    let haversine = |a: (f64, f64), b: (f64, f64)| -> f64 {
        let (lat1, lon1) = (a.0.to_radians(), a.1.to_radians());
        let (lat2, lon2) = (b.0.to_radians(), b.1.to_radians());
        let (dlat, dlon) = (lat2 - lat1, lon2 - lon1);
        let h = (dlat / 2.0).sin().powi(2) + lat1.cos() * lat2.cos() * (dlon / 2.0).sin().powi(2);
        6_371_000.0 * 2.0 * h.sqrt().asin()
    };
    // Effective speed per fix: the device's reported `speed` when present, else
    // derived from the previous fix (distance / time). So movement is detected from
    // the raw trace even when the `speed` column is absent — historical rows before
    // migration 0047, or any source that doesn't report per-fix speed.
    let eff: Vec<f64> = (0..pts.len())
        .map(|i| {
            pts[i].3.unwrap_or_else(|| {
                if i == 0 {
                    return 0.0;
                }
                let d = haversine((pts[i - 1].1, pts[i - 1].2), (pts[i].1, pts[i].2));
                let dt = (pts[i].0 - pts[i - 1].0).num_seconds().max(1) as f64;
                d / dt
            })
        })
        .collect();
    let moving: Vec<bool> = eff.iter().map(|&s| s > MOVING_MPS).collect();

    let mut segs = Vec::new();
    let mut i = 0;
    while i < pts.len() {
        if !moving[i] {
            i += 1;
            continue;
        }
        // Grow a run from i, bridging still-gaps shorter than MERGE_GAP_S.
        let start = i;
        let mut end = i;
        let mut j = i + 1;
        while j < pts.len() {
            if moving[j] {
                end = j;
                j += 1;
            } else {
                let mut k = j;
                while k < pts.len() && !moving[k] {
                    k += 1;
                }
                if k < pts.len() && (pts[k].0 - pts[end].0).num_seconds() <= MERGE_GAP_S {
                    j = k; // brief pause — same trip
                } else {
                    break;
                }
            }
        }
        // Distance over the run, and average speed of its moving fixes.
        let mut dist = 0.0;
        for w in start..end {
            dist += haversine((pts[w].1, pts[w].2), (pts[w + 1].1, pts[w + 1].2));
        }
        let (mut sspeed, mut nspeed) = (0.0, 0usize);
        for w in start..=end {
            if eff[w] > MOVING_MPS {
                sspeed += eff[w];
                nspeed += 1;
            }
        }
        if dist >= MIN_DIST_M {
            let avg_kmh = (nspeed > 0).then(|| sspeed / nspeed as f64 * 3.6);
            segs.push((pts[start].0, pts[end].0, dist / 1000.0, avg_kmh));
        }
        i = end + 1;
    }
    segs
}

/// Runs of time the owner was demonstrably AT a machine.
///
/// The dossier was missing the one signal that settles attendance. Location says
/// where a PHONE was, and a phone left on a counter reads identically to a phone
/// in a pocket. A keyboard cannot be used from another building: an active app
/// session is a body in a chair, and a body in a chair is not a body at the event
/// its calendar claims. This is deliberately a NEGATIVE instrument — it is far
/// better at proving where someone wasn't than at describing what they did.
///
/// Sessions are already correctly built upstream (`applets/mac_ingest/sessionize.rs`
/// holds them open across upload batches), so this only has to merge them into
/// runs. Individual sessions are useless here — a working hour is a hundred app
/// switches, which would bury the dossier — so consecutive sessions are folded
/// into one presence run and described by the apps that filled it.
///
/// Returns `(start, end, top apps by time, any input observed, how the run ended)`.
async fn day_device_presence(
    pool: &PgPool,
    start_str: &str,
    end_str: &str,
) -> Vec<(
    chrono::DateTime<chrono::Utc>,
    chrono::DateTime<chrono::Utc>,
    Vec<String>,
    bool,
    String,
)> {
    use sqlx::Row;

    /// A short break is still presence — someone who steps away for coffee and
    /// comes back never left the building. Longer than this and the gap is real,
    /// so the runs stay separate and the stretch between them is genuinely open.
    const MERGE_GAP_S: i64 = 600;
    /// A run this short is a glance at a notification, not evidence of anything.
    const MIN_RUN_S: i64 = 120;
    /// Enough to characterise a stretch; more is noise in a prompt.
    const TOP_APPS: usize = 4;

    let rows = sqlx::query(
        "SELECT app_name, started_at, ended_at, attention, closed_by, is_open \
         FROM data_activity_app_session \
         WHERE ended_at >= $1::timestamptz AND started_at <= $2::timestamptz \
         ORDER BY started_at",
    )
    .bind(start_str)
    .bind(end_str)
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    struct Run {
        start: chrono::DateTime<chrono::Utc>,
        end: chrono::DateTime<chrono::Utc>,
        apps: Vec<(String, i64)>,
        /// `active` means keys and clicks — a person, present. `watching` means
        /// the app merely held the display awake, which a video can do to an
        /// empty room. Both are usage; only one is a body. Worth a word in the
        /// dossier, because a run with no input at all is much weaker evidence
        /// of where someone was.
        any_active: bool,
        ended: String,
    }
    let mut runs: Vec<Run> = Vec::new();

    for r in &rows {
        let s: chrono::DateTime<chrono::Utc> = r.get("started_at");
        let e: chrono::DateTime<chrono::Utc> = r.get("ended_at");
        if e < s {
            continue;
        }
        let app: String = r.try_get("app_name").unwrap_or_default();
        let secs = (e - s).num_seconds().max(0);
        let active = r
            .try_get::<String, _>("attention")
            .map(|a| a == "active")
            .unwrap_or(true);
        // `closed_by` explains the gap that FOLLOWS the run, which is the whole
        // reason it is worth carrying: `stale` means the collector died, so the
        // silence after it is our failure and not the owner walking away. Reading
        // that silence as absence would be the same category error this file
        // exists to stop, just pointed at a different source.
        let ended = if r.try_get::<bool, _>("is_open").unwrap_or(false) {
            "open".to_string()
        } else {
            r.try_get::<Option<String>, _>("closed_by")
                .ok()
                .flatten()
                .unwrap_or_else(|| "unknown".to_string())
        };

        let extend = runs
            .last()
            .is_some_and(|last| (s - last.end).num_seconds() <= MERGE_GAP_S);
        if extend {
            let last = runs.last_mut().expect("checked by `extend`");
            if e > last.end {
                last.end = e;
            }
            last.ended = ended;
            last.any_active |= active;
            match last.apps.iter_mut().find(|(n, _)| n == &app) {
                Some((_, t)) => *t += secs,
                None => last.apps.push((app, secs)),
            }
        } else {
            runs.push(Run {
                start: s,
                end: e,
                apps: vec![(app, secs)],
                any_active: active,
                ended,
            });
        }
    }

    runs.into_iter()
        .filter(|r| (r.end - r.start).num_seconds() >= MIN_RUN_S)
        .map(|mut r| {
            r.apps.sort_by(|a, b| b.1.cmp(&a.1));
            let apps = r
                .apps
                .into_iter()
                .take(TOP_APPS)
                .map(|(n, _)| n)
                .filter(|n| !n.trim().is_empty())
                .collect();
            (r.start, r.end, apps, r.any_active, r.ended)
        })
        .collect()
}

/// One run of messages in a single thread, with the text that makes it legible.
struct MessageBurst {
    start: chrono::DateTime<chrono::Utc>,
    end: chrono::DateTime<chrono::Utc>,
    /// The person on the other end, resolved where possible. A thread with a
    /// brand-new correspondent has no `wiki_people` row, so this falls back to
    /// whatever the message itself carried — the thread is still real and still
    /// belongs on the spine. That fallback is the whole point: the events that
    /// matter most are often with someone the graph has never seen.
    counterpart: String,
    sent: usize,
    received: usize,
    /// `(from_me, text)` — bounded excerpts, oldest first.
    excerpts: Vec<(bool, String)>,
}

/// Messages in the window, grouped into per-thread bursts.
///
/// # Both directions are READ; only owner-authored text may ever be QUOTED
///
/// The detective sees both halves of a conversation, because half of any
/// arrangement is the other person's half — "7:30 at the usual place?" is as
/// often theirs as yours, and a detective that only sees your "perfect" cannot
/// place the event. Reading their words to CUT the day correctly is a different act
/// from REPRODUCING them to the reader, and the prompt carries that second rule
/// (see SEGMENT_PROMPT: never quote another person's words in a summary).
///
/// This is an egress decision, made deliberately: message bodies now leave the
/// box for whichever inference endpoint is configured, which under BYO AI is
/// whatever the user pointed the Chat slot at. Budgets below are what keep that
/// bounded — a talkative day cannot ship the whole inbox.
async fn day_message_bursts(
    pool: &PgPool,
    start_str: &str,
    end_str: &str,
) -> Vec<MessageBurst> {
    use sqlx::Row;

    /// Messages further apart than this in one thread are separate bursts. A
    /// conversation has pauses; a reply the next afternoon is a new occasion.
    const BURST_GAP_MINUTES: i64 = 45;
    /// A burst this small is a logistics ping, not a stretch of the day. It
    /// still counts toward its burst — this only stops single acknowledgements
    /// from each claiming a spine line.
    const MIN_BURST_MESSAGES: usize = 2;
    /// Bursts on the spine. Beyond this the dossier stops being a dossier.
    const MAX_BURSTS: usize = 24;
    /// Excerpts carried per burst, and the cap on each. Enough to show what the
    /// exchange was ABOUT; far short of reproducing a conversation.
    const MAX_EXCERPTS_PER_BURST: usize = 4;
    const EXCERPT_CHARS: usize = 140;

    // DISTINCT ON (m.id): the refs join can match twice (a message carrying both
    // a sender and a recipient ref), which would double-count the burst. Order
    // the tiebreak so a row WITH a resolved name wins over one without.
    let rows = sqlx::query(
        "SELECT DISTINCT ON (m.id) \
                m.id, m.thread_id, m.body, m.occurred_at, m.from_name, m.from_identifier, \
                COALESCE((m.metadata->>'is_from_me')::boolean, false) AS from_me, \
                pe.name AS resolved_name \
         FROM data_communication_message m \
         LEFT JOIN wiki_refs er \
           ON er.source_table = 'data_communication_message' AND er.source_id = m.id \
          AND er.entity_type = 'person' AND er.role IN ('sender', 'recipient') \
         LEFT JOIN wiki_people pe ON pe.id = er.entity_id \
         WHERE m.occurred_at >= $1::timestamptz AND m.occurred_at <= $2::timestamptz \
         ORDER BY m.id, (pe.name IS NULL)",
    )
    .bind(start_str)
    .bind(end_str)
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    struct Msg {
        thread: String,
        ts: chrono::DateTime<chrono::Utc>,
        body: Option<String>,
        from_me: bool,
        who: Option<String>,
    }

    let mut msgs: Vec<Msg> = rows
        .iter()
        .map(|r| {
            let from_me: bool = r.try_get("from_me").unwrap_or(false);
            // A message you SENT has no `from_name` (people.rs fills it only for
            // the sender of a received message), so the counterpart of an
            // outbound message is only ever known via the resolved ref or the
            // thread it sits in — which is why grouping happens first.
            let who = r
                .try_get::<Option<String>, _>("resolved_name")
                .ok()
                .flatten()
                .or_else(|| r.try_get::<Option<String>, _>("from_name").ok().flatten())
                .filter(|s| !s.trim().is_empty());
            // Threadless channels exist; fall back to the handle so a
            // conversation still groups, and only then to the message itself.
            let thread = r
                .try_get::<Option<String>, _>("thread_id")
                .ok()
                .flatten()
                .filter(|s| !s.trim().is_empty())
                .or_else(|| r.try_get::<Option<String>, _>("from_identifier").ok().flatten())
                .unwrap_or_else(|| r.get::<String, _>("id"));
            Msg {
                thread,
                ts: r.get("occurred_at"),
                body: r.try_get::<Option<String>, _>("body").ok().flatten(),
                from_me,
                who,
            }
        })
        .collect();

    msgs.sort_by(|a, b| a.thread.cmp(&b.thread).then(a.ts.cmp(&b.ts)));

    let mut bursts: Vec<MessageBurst> = Vec::new();
    let mut current: Option<(String, Vec<&Msg>)> = None;

    let flush = |acc: &(String, Vec<&Msg>), out: &mut Vec<MessageBurst>| {
        let group = &acc.1;
        if group.len() < MIN_BURST_MESSAGES {
            return;
        }
        // The counterpart is a property of the THREAD, not of any one message —
        // recovered from whichever message in the burst carried a name.
        let counterpart = group
            .iter()
            .find_map(|m| m.who.clone())
            .unwrap_or_else(|| "unknown".to_string());
        let sent = group.iter().filter(|m| m.from_me).count();
        let excerpts = group
            .iter()
            .filter_map(|m| {
                m.body
                    .as_deref()
                    .map(str::trim)
                    .filter(|b| !b.is_empty())
                    .map(|b| (m.from_me, cap(b, EXCERPT_CHARS)))
            })
            .take(MAX_EXCERPTS_PER_BURST)
            .collect();
        out.push(MessageBurst {
            start: group[0].ts,
            end: group[group.len() - 1].ts,
            counterpart,
            sent,
            received: group.len() - sent,
            excerpts,
        });
    };

    for m in &msgs {
        match &mut current {
            Some((thread, group))
                if *thread == m.thread
                    && (m.ts - group[group.len() - 1].ts).num_minutes() <= BURST_GAP_MINUTES =>
            {
                group.push(m);
            }
            _ => {
                if let Some(acc) = &current {
                    flush(acc, &mut bursts);
                }
                current = Some((m.thread.clone(), vec![m]));
            }
        }
    }
    if let Some(acc) = &current {
        flush(acc, &mut bursts);
    }

    // Busiest first for the cap, so a budget cut drops the thinnest exchanges
    // rather than the afternoon's; then back into time order for the spine.
    bursts.sort_by(|a, b| (b.sent + b.received).cmp(&(a.sent + a.received)));
    bursts.truncate(MAX_BURSTS);
    bursts.sort_by_key(|b| b.start);
    bursts
}

/// Build the DOSSIER: one compact, time-ordered feature list of the day's
/// evidence, drawn from the CLEAN rollups (visits, calendar, sleep, audio
/// sessions) plus time-placed message bursts and a health snapshot. Each item is
/// capped per-type, so the whole dossier is bounded by construction — that is
/// what lets the detective drop the old global truncation.
async fn build_dossier(
    pool: &PgPool,
    date: NaiveDate,
    start_str: &str,
    end_str: &str,
    tz_label: Option<&str>,
    tz: Option<&Tz>,
) -> String {
    use sqlx::Row;

    let fmt = |t: &chrono::DateTime<chrono::Utc>| match tz {
        Some(z) => t.with_timezone(z).format("%H:%M").to_string(),
        None => t.format("%H:%M").to_string(),
    };

    // The time-ordered spine — everything with a start (and usually an end).
    let mut spine: Vec<(chrono::DateTime<chrono::Utc>, String)> = Vec::new();

    // Visits — place resolved through wiki_places, arrival→departure.
    let visits = sqlx::query(
        "SELECT COALESCE(p.name, v.place_name) AS place, v.started_at, v.ended_at \
         FROM data_location_visit v \
         LEFT JOIN wiki_refs er \
           ON er.source_table = 'data_location_visit' AND er.source_id = v.id \
          AND er.entity_type = 'place' \
         LEFT JOIN wiki_places p ON p.id = er.entity_id \
         WHERE v.started_at >= $1::timestamptz AND v.started_at <= $2::timestamptz \
         ORDER BY v.started_at",
    )
    .bind(start_str)
    .bind(end_str)
    .fetch_all(pool)
    .await
    .unwrap_or_default();
    for r in &visits {
        let place = r
            .try_get::<Option<String>, _>("place")
            .ok()
            .flatten()
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| "Unknown place".to_string());
        let arr: chrono::DateTime<chrono::Utc> = r.get("started_at");
        let dep: Option<chrono::DateTime<chrono::Utc>> =
            r.try_get("ended_at").ok().flatten();
        let span = match dep {
            Some(d) => format!("{}–{}", fmt(&arr), fmt(&d)),
            None => format!("{}–?", fmt(&arr)),
        };
        spine.push((arr, format!("- [visit] {} — {}", span, cap(&place, 80))));
    }

    // Movement — MOVING stretches of the day's GPS trace, computed from `speed`
    // (never stored). This is the ONLY evidence the detective may use for HOW the
    // owner travelled; without it the model fabricates a mode (the "tram" over a real
    // walk). Keyed on the raw trace, not visits, so movement is grounded even where
    // no visit was clustered — and a mostly-stationary window yields NO stretch, so
    // it can never be called transit.
    for (s, e, km, avg_kmh) in day_movement_segments(pool, start_str, end_str).await {
        let line = match avg_kmh {
            Some(kmh) => format!(
                "- [movement] {}–{} — {:.1} km at ~{:.0} km/h",
                fmt(&s),
                fmt(&e),
                km,
                kmh,
            ),
            None => format!("- [movement] {}–{} — {:.1} km, pace unknown", fmt(&s), fmt(&e), km),
        };
        spine.push((s, line));
    }

    // Device presence — the negative instrument. A keyboard in use is a body that
    // was not at whatever the calendar scheduled for that hour.
    for (s, e, apps, any_active, ended) in day_device_presence(pool, start_str, end_str).await {
        // Spell the close reason out. `stale` in particular MUST read as "we
        // stopped watching", never as "they left" — the sessionizer went to real
        // trouble to keep those two apart and a terse code would throw it away.
        let tail = match ended.as_str() {
            "lock" => " — ended: screen locked",
            "suspend" => " — ended: machine slept",
            "idle" => " — ended: went idle",
            "quit" => " — ended: app quit",
            "stale" => " — ended: COLLECTOR STOPPED; the gap after this is our blind spot, not evidence they left",
            "open" => " — still open at the day's end",
            _ => "",
        };
        let what = if apps.is_empty() {
            String::new()
        } else {
            format!(" ({})", apps.join(", "))
        };
        let presence = if any_active {
            "typing/clicking at a machine"
        } else {
            "a machine held awake, NO input observed — weaker: a video plays to an empty room too"
        };
        spine.push((
            s,
            format!(
                "- [device] {}–{} — {}{}{}",
                fmt(&s),
                fmt(&e),
                presence,
                what,
                tail
            ),
        ));
    }

    // Calendar — title, start→end, plus the two tags that say whether this line
    // is even ABOUT the owner. All-day events bound nothing; flag them so the
    // detective does not treat a 24h block as a boundary.
    let cal = sqlx::query(
        "SELECT title, started_at, ended_at, is_all_day, calendar_access_role, response_status \
         FROM data_calendar_event \
         WHERE started_at >= $1::timestamptz AND started_at <= $2::timestamptz \
           AND (status IS NULL OR status <> 'cancelled') \
         ORDER BY started_at",
    )
    .bind(start_str)
    .bind(end_str)
    .fetch_all(pool)
    .await
    .unwrap_or_default();
    for r in &cal {
        let title: String = r.try_get("title").unwrap_or_default();
        let s: chrono::DateTime<chrono::Utc> = r.get("started_at");
        let e: chrono::DateTime<chrono::Utc> = r.get("ended_at");
        let all_day: bool = r.try_get("is_all_day").unwrap_or(false);
        let access: Option<String> = r.try_get("calendar_access_role").ok().flatten();
        let rsvp: Option<String> = r.try_get("response_status").ok().flatten();

        let mut tags: Vec<&str> = vec!["calendar"];
        // Both tags are OMITTED when unknown rather than defaulted. An iOS-synced
        // row has no access role and most events have no RSVP, and inventing
        // "own calendar" or "no reply" for those would manufacture exactly the
        // false confidence this whole line is meant to remove.
        match access.as_deref() {
            Some("reader") | Some("freeBusyReader") => {
                tags.push("SUBSCRIBED — someone else's calendar")
            }
            Some("owner") | Some("writer") => tags.push("own calendar"),
            _ => {}
        }
        match rsvp.as_deref() {
            Some("declined") => tags.push("owner DECLINED"),
            Some("accepted") => tags.push("owner accepted the invite in advance"),
            Some("tentative") => tags.push("owner replied tentative"),
            Some("needsAction") => tags.push("owner never replied — means nothing either way"),
            _ => {}
        }
        if all_day {
            tags.push("all-day, bounds nothing");
        }

        let line = if all_day {
            format!("- [{}] {}", tags.join(", "), cap(&title, 100))
        } else {
            format!(
                "- [{}] {}–{} — {}",
                tags.join(", "),
                fmt(&s),
                fmt(&e),
                cap(&title, 100)
            )
        };
        spine.push((s, line));
    }

    // Sleep — a hard boundary. Overlap the window (sleep starts the night before).
    let sleep = sqlx::query(
        "SELECT started_at, ended_at, duration_minutes \
         FROM data_health_sleep \
         WHERE ended_at >= $1::timestamptz AND started_at <= $2::timestamptz \
         ORDER BY started_at",
    )
    .bind(start_str)
    .bind(end_str)
    .fetch_all(pool)
    .await
    .unwrap_or_default();
    for r in &sleep {
        let s: chrono::DateTime<chrono::Utc> = r.get("started_at");
        let e: chrono::DateTime<chrono::Utc> = r.get("ended_at");
        let dur: Option<i32> = r.try_get("duration_minutes").ok().flatten();
        let dur_str = dur
            .map(|m| format!(" ({}h{:02}m)", m / 60, m % 60))
            .unwrap_or_default();
        spine.push((s, format!("- [sleep] {}–{}{}", fmt(&s), fmt(&e), dur_str)));
    }

    // Audio sessions — the coarse context rollup. Content (the stitched summaries)
    // is the reasoning material that lets the detective name a location-less day,
    // capped so a talkative day cannot bloat the prompt.
    let audio = sqlx::query(
        "SELECT started_at, ended_at, speaker_mode, content \
         FROM data_audio_session \
         WHERE started_at >= $1::timestamptz AND started_at < $2::timestamptz \
         ORDER BY started_at",
    )
    .bind(start_str)
    .bind(end_str)
    .fetch_all(pool)
    .await
    .unwrap_or_default();
    for r in &audio {
        let s: chrono::DateTime<chrono::Utc> = r.get("started_at");
        let e: chrono::DateTime<chrono::Utc> = r.get("ended_at");
        let mode: i16 = r.try_get("speaker_mode").unwrap_or(0);
        let who = match mode {
            0 => "silent/ambient",
            1 => "solo voice",
            2 => "conversation",
            _ => "group",
        };
        let content: Option<String> = r.try_get("content").ok().flatten();
        let content_part = content
            .as_deref()
            .filter(|c| !c.trim().is_empty())
            .map(|c| format!(" — {}", cap(c, 400)))
            .unwrap_or_default();
        spine.push((
            s,
            format!("- [audio, {}] {}–{}{}", who, fmt(&s), fmt(&e), content_part),
        ));
    }

    // Assistant chats — the user's own conversations with Virtues that day. A weak
    // boundary signal but real "what was I doing / thinking" context. Bounded by
    // LIMIT and the title cap.
    let chats = sqlx::query(
        "SELECT title, message_count, created_at \
         FROM app_chats \
         WHERE created_at >= $1::timestamptz AND created_at <= $2::timestamptz \
         ORDER BY created_at LIMIT 12",
    )
    .bind(start_str)
    .bind(end_str)
    .fetch_all(pool)
    .await
    .unwrap_or_default();
    for r in &chats {
        let title = r
            .try_get::<Option<String>, _>("title")
            .ok()
            .flatten()
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| "(untitled)".to_string());
        let mc: i64 = r.try_get("message_count").unwrap_or(0);
        let s: chrono::DateTime<chrono::Utc> = r.get("created_at");
        spine.push((
            s,
            format!("- [assistant chat] {} — \"{}\" ({mc} msgs)", fmt(&s), cap(&title, 80)),
        ));
    }

    // Purchases — discrete, high-meaning events, passed INDIVIDUALLY (not aggregated:
    // there are a handful a day and the merchant IS the signal). Each names what a
    // stretch actually was — a meal, a shop, a checkout — grounding windows the audio
    // alone leaves ambiguous.
    let txns = sqlx::query(
        "SELECT occurred_at, amount, currency, merchant_name, description \
         FROM data_financial_transaction \
         WHERE occurred_at >= $1::timestamptz AND occurred_at <= $2::timestamptz \
           AND is_archived IS NOT TRUE \
         ORDER BY occurred_at",
    )
    .bind(start_str)
    .bind(end_str)
    .fetch_all(pool)
    .await
    .unwrap_or_default();
    for r in &txns {
        let ts: chrono::DateTime<chrono::Utc> = r.get("occurred_at");
        let cents: i64 = r.try_get("amount").unwrap_or(0);
        let currency = r
            .try_get::<Option<String>, _>("currency")
            .ok()
            .flatten()
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| "USD".to_string());
        let merchant = r
            .try_get::<Option<String>, _>("merchant_name")
            .ok()
            .flatten()
            .filter(|s| !s.trim().is_empty())
            .or_else(|| r.try_get::<Option<String>, _>("description").ok().flatten())
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| "unknown merchant".to_string());
        // Plaid signs amounts: positive = money out (a purchase), negative = money in
        // (a refund / credit). Label by direction and show the magnitude in its own
        // currency — never a bare "$" (which would misstate a EUR/GBP charge).
        let kind = if cents < 0 { "refund" } else { "purchase" };
        let magnitude = cents.unsigned_abs() as f64 / 100.0;
        let amount = if currency == "USD" {
            format!("${magnitude:.2}")
        } else {
            format!("{magnitude:.2} {currency}")
        };
        spine.push((
            ts,
            format!("- [{kind}] {} — {} at {}", fmt(&ts), amount, cap(&merchant, 60)),
        ));
    }

    // Messages — time-placed BURSTS carrying their text, onto the spine.
    //
    // This used to be `GROUP BY who` over the whole day, rendered as
    // `- 14 with <name>` in a `## Messages` block appended AFTER the spine was
    // sorted — off the timeline entirely. Two
    // faults, and they compounded: the detective could not read a single word
    // anyone wrote, and it could not place a single message in time — so
    // messages could never corroborate a window, which is the one thing a
    // boundary needs. Every other source on the spine carries content and a
    // timestamp; the richest human-intent source in the lake carried neither.
    // A coffee arranged by text, walked to, and paid for read as an unnamed
    // purchase next to an unnamed conversation.
    for b in day_message_bursts(pool, start_str, end_str).await {
        let mut line = format!(
            "- [messages] {}–{} — {} with {} ({} sent, {} received)",
            fmt(&b.start),
            fmt(&b.end),
            b.sent + b.received,
            cap(&b.counterpart, 60),
            b.sent,
            b.received
        );
        for (from_me, text) in &b.excerpts {
            line.push_str(&format!(
                "\n    {} {}",
                if *from_me { "you:" } else { "them:" },
                text
            ));
        }
        spine.push((b.start, line));
    }
    spine.sort_by_key(|(k, _)| *k);

    // ── Assemble ──
    let day_of_week = date.format("%A").to_string();
    let date_display = date.format("%B %e, %Y").to_string();
    let tz_name = tz_label.unwrap_or("UTC");
    let mut out = format!(
        "Date: {}, {} ({} local time)\n\
         All times below are the user's local timezone ({}). \
         Emit event start/end times in the same local timezone.\n\n\
         ## Timeline evidence\n",
        day_of_week, date_display, tz_name, tz_name
    );
    if spine.is_empty() {
        out.push_str("(no located visits, calendar blocks, sleep, or audio for this day)\n");
    } else {
        for (_, line) in &spine {
            out.push_str(line);
            out.push('\n');
        }
    }

    if let Some(h) = build_health_snapshot(pool, start_str, end_str).await {
        append_section(&mut out, &h);
    }

    out
}

/// The detective's LIGHT recency signal — the last few days' event labels, grouped
/// by day. Just enough to disambiguate an ambiguous stretch; the detective's job
/// is cutting, not remembering. Empty string on a cold start.
async fn recent_event_labels(pool: &PgPool, date: NaiveDate, tz: Option<&Tz>) -> String {
    let _ = tz;
    let rows = sqlx::query_as::<_, (NaiveDate, String)>(
        "SELECT d.date, COALESCE(e.user_label, e.auto_label, '(unlabeled)') AS label \
         FROM wiki_events e JOIN wiki_days d ON d.id = e.day_id \
         WHERE d.date >= $1 AND d.date < $2 AND NOT e.is_unknown AND NOT e.user_hidden \
         ORDER BY d.date, e.started_at",
    )
    .bind(date - chrono::Duration::days(3))
    .bind(date)
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    if rows.is_empty() {
        return String::new();
    }

    use std::collections::BTreeMap;
    let mut by_day: BTreeMap<NaiveDate, Vec<String>> = BTreeMap::new();
    for (d, label) in rows {
        by_day.entry(d).or_default().push(label);
    }
    by_day
        .into_iter()
        .map(|(d, labels)| format!("- {}: {}", d.format("%a %b %-d"), labels.join(", ")))
        .collect::<Vec<_>>()
        .join("\n")
}

/// The day-summary's FULL case file — the last 14 days of events, label + summary,
/// grouped by day. This is where recent context earns its keep, for voice and for
/// dated temporal echoes. Empty string on a cold start.
async fn recent_event_case_file(pool: &PgPool, date: NaiveDate, tz: Option<&Tz>) -> String {
    let _ = tz;
    let rows = sqlx::query_as::<_, (NaiveDate, String, Option<String>)>(
        "SELECT d.date, COALESCE(e.user_label, e.auto_label, '(unlabeled)') AS label, e.event_summary \
         FROM wiki_events e JOIN wiki_days d ON d.id = e.day_id \
         WHERE d.date >= $1 AND d.date < $2 AND NOT e.is_unknown AND NOT e.user_hidden \
         ORDER BY d.date, e.started_at",
    )
    .bind(date - chrono::Duration::days(14))
    .bind(date)
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    if rows.is_empty() {
        return String::new();
    }

    use std::collections::BTreeMap;
    let mut by_day: BTreeMap<NaiveDate, Vec<String>> = BTreeMap::new();
    for (d, label, summary) in rows {
        let line = match summary.as_deref().filter(|s| !s.trim().is_empty()) {
            Some(s) => format!("  - {}: {}", label, cap(s, 200)),
            None => format!("  - {}", label),
        };
        by_day.entry(d).or_default().push(line);
    }
    by_day
        .into_iter()
        .map(|(d, lines)| format!("{}\n{}", d.format("%A, %B %-d"), lines.join("\n")))
        .collect::<Vec<_>>()
        .join("\n\n")
}


// ── virtues-api call ───────────────────────────────────────────────────────────

/// Call virtues-api for the summary generation
/// One call, two jobs — so the caller says which slot and which instructions.
///
/// Both callers resolve the Chat slot today: segmentation is adjudication, not
/// extraction (see the callers' comments), and the narration is the narrative
/// call. If segmentation ever moves to Lite for cost, change it at the caller —
/// this function takes whatever slot it is handed, and the shared completion
/// helper turns the slot into a model (slot default for Chat, never the pin).
///
/// They used to be a single Opus call producing both, which is why events cost
/// narrative prices, why "only narrate a day with enough events" was circular
/// (the events did not exist until the narration ran), and why there could be no
/// hourly cron.
async fn call_virtues_api(
    pool: &PgPool,
    system_prompt: &str,
    slot: ModelSlot,
    user_prompt: &str,
) -> Result<String> {
    crate::virtues_api::completion::system_completion(
        pool,
        slot,
        "day_summary",
        system_prompt,
        user_prompt,
        // 16 events x ~60-90 tokens is 960-1440 for the events ALONE, before
        // the 180-word diary, the epigraph and the data_quality JSON. At 1000
        // a rich day truncated mid-array — and since the parse was
        // all-or-nothing, that day lost EVERY event with only a warn!.
        // Raised to 4000, and the parse now salvages besides.
        //
        // Then 4000 failed the same way one level up. The Chat slot moved to
        // `anthropic/claude-sonnet-5` (2026-08-27, on dragon from 09-04), and
        // that model THINKS before it answers — and Anthropic counts the
        // thinking against max_tokens. On the day dossier it wanted more than
        // 4000 to think, so it hit the cap with zero content tokens: 237 of
        // 276 calls, all billed, all three retries, every hour on the same
        // never-fingerprinted day. $13.61 to produce nothing, and the day
        // never narrated. Even the calls that did answer maxed at exactly
        // 4000, i.e. were truncated and salvaged.
        //
        // So the cap is sized for thinking plus answer, with real headroom
        // because the failing calls stopped AT the cap and never showed how
        // much thinking the model actually wanted. Read `reasoning_tokens` in
        // `app_ai_calls` after a few nights before tightening this.
        16_000,
        0.3,
        // Low effort: the detective job is adjudicating witnesses into a
        // timeline, not proving a theorem. The lever exists on this model
        // (see the registry's slot note) and trims the thinking tax that made
        // 4000 fatal; a model without the lever just ignores the hint.
        Some("low"),
    )
    .await
}

// ── Structured event parsing ─────────────────────────────────────────────────

/// LLM event parsed from virtues-api response
#[derive(Debug, serde::Deserialize)]
struct LlmEvent {
    start: String,
    end: String,
    label: String,
    /// 1-3 sentence factual description grounded in the source data. Optional
    /// because the model may omit it for Unknown blocks.
    #[serde(default)]
    summary: Option<String>,
    /// 2-4 lowercase topical tags. Free: the model is already reading the
    /// window to write the summary. Feeds `topic_entity_novelty`, which until
    /// now scored empty arrays on every cron-generated event because nothing
    /// but the chat tool ever wrote this column.
    #[serde(default)]
    topics: Vec<String>,
}

/// Parse the events array, salvaging complete objects from a truncated one.
///
/// The strict path first: a well-formed array parses whole, which is the
/// overwhelmingly common case.
///
/// If that fails, we do NOT throw the day away. The previous behaviour ran
/// `serde_json::from_str::<Vec<LlmEvent>>` over the entire array, so a response
/// clipped by `max_tokens` mid-event was invalid JSON — the day got ZERO events
/// and a `warn!` nobody read. Losing sixteen real events because the
/// seventeenth was cut in half is the worst possible trade.
///
/// So: scan top-level `{...}` objects, decode each independently, keep the ones
/// that are whole. A truncated tail costs you the truncated event and nothing
/// else.
fn parse_events_salvaging(raw: &str) -> Option<Vec<LlmEvent>> {
    if let Ok(events) = serde_json::from_str::<Vec<LlmEvent>>(raw) {
        return Some(events);
    }

    let mut events = Vec::new();
    let mut depth = 0usize;
    let mut start = None;
    let mut in_string = false;
    let mut escaped = false;

    for (i, c) in raw.char_indices() {
        if in_string {
            match c {
                _ if escaped => escaped = false,
                '\\' => escaped = true,
                '"' => in_string = false,
                _ => {}
            }
            continue;
        }
        match c {
            '"' => in_string = true,
            '{' => {
                if depth == 0 {
                    start = Some(i);
                }
                depth += 1;
            }
            '}' => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    if let Some(s0) = start.take() {
                        if let Ok(ev) = serde_json::from_str::<LlmEvent>(&raw[s0..=i]) {
                            events.push(ev);
                        }
                    }
                }
            }
            _ => {}
        }
    }

    if events.is_empty() {
        tracing::warn!(raw, "no salvageable events in LLM response");
        return None;
    }

    tracing::warn!(
        salvaged = events.len(),
        "events array was malformed (likely truncated) — salvaged complete events"
    );
    Some(events)
}

/// Parsed day summary from LLM response
struct ParsedDaySummary {
    diary: String,
    epigraph: Option<String>,
    data_quality: Option<String>,
    events: Option<Vec<LlmEvent>>,
}

/// Split virtues-api response into diary text, epigraph, data quality, and optional events JSON.
/// Expected format:
///   [diary text]
///   ---EPIGRAPH---
///   [one-line epigraph]
///   ---DATA_QUALITY---
///   {"coverage":{...},"overall":3,"note":"..."}
///   ---EVENTS---
/// Drop prompt-instruction echo from the article prose.
///
/// Observed live (2025-12-16): the model opened its output with
/// "Entities you may link: [David](/person/…), …" followed by a `---` rule —
/// the prompt's own instruction header, reproduced as if it were the
/// article's front matter. The prompt now forbids it, but a prompt is a
/// request; this is the guarantee. Any line carrying the header is dropped,
/// along with a horizontal rule left stranded directly beneath it.
fn strip_prompt_echo(prose: &str) -> String {
    let mut out: Vec<&str> = Vec::new();
    let mut dropping_rule = false;
    for line in prose.lines() {
        if line.trim_start().starts_with("Entities you may link") {
            dropping_rule = true;
            continue;
        }
        if dropping_rule {
            let t = line.trim();
            if t.is_empty() {
                continue;
            }
            dropping_rule = false;
            if t.chars().all(|c| c == '-') && t.len() >= 3 {
                continue;
            }
        }
        out.push(line);
    }
    // The drop can leave leading blank lines where the header sat.
    let joined = out.join("\n");
    joined.trim_start_matches('\n').to_string()
}

/// Unlink any entity ref-link the candidate list did not sanction.
///
/// Observed live (2025-12-16): a day whose window held ZERO entity refs got
/// no "Entities you may link" section at all — and the model, knowing the
/// format from its instructions, fabricated ids in the right shape
/// (`/person/person_5a4c`) and linked them through the prose. A dead link in
/// a day article is worse than no link: it looks like the record knows
/// someone it does not.
///
/// The candidate list is the ONLY sanctioned source of ref-links, so this is
/// exactly decidable: a `/person/`, `/place/` or `/org/` link whose URL is
/// not in the list is replaced by its own text. The name survives — it is
/// real prose; the link was the fabrication.
fn unlink_uninvited_refs(prose: &str, candidates: &[String]) -> String {
    // The candidate lines are `[Name](/kind/id)`; sanctioned = their URLs.
    let allowed: std::collections::HashSet<&str> = candidates
        .iter()
        .filter_map(|line| {
            let open = line.find("](")?;
            let close = line[open + 2..].find(')')?;
            Some(&line[open + 2..open + 2 + close])
        })
        .collect();

    let mut out = String::with_capacity(prose.len());
    let mut rest = prose;
    while let Some(start) = rest.find('[') {
        let Some(mid) = rest[start..].find("](") else {
            break;
        };
        let text_end = start + mid;
        let url_start = text_end + 2;
        let Some(url_len) = rest[url_start..].find(')') else {
            break;
        };
        let url = &rest[url_start..url_start + url_len];
        let is_ref = url.starts_with("/person/") || url.starts_with("/place/") || url.starts_with("/org/");
        out.push_str(&rest[..start]);
        if is_ref && !allowed.contains(url) {
            // The text, shorn of its invented link.
            out.push_str(&rest[start + 1..text_end]);
        } else {
            out.push_str(&rest[start..url_start + url_len + 1]);
        }
        rest = &rest[url_start + url_len + 1..];
    }
    out.push_str(rest);
    out
}

///   [JSON events]
///
/// All markers except the diary are optional. Handles markdown code fences around JSON.
fn parse_virtues_api_response(response: &str) -> ParsedDaySummary {
    // 1. Split off events JSON first (it's always at the end)
    let (before_events, events) = if let Some(idx) = response.find("---EVENTS---") {
        let before = &response[..idx];
        let mut events_str = response[idx + "---EVENTS---".len()..].trim();
        events_str = events_str
            .trim_start_matches("```json")
            .trim_start_matches("```")
            .trim_end_matches("```")
            .trim();
        let parsed = parse_events_salvaging(events_str);
        (before, parsed)
    } else {
        (response, None)
    };

    // 2. Split off data_quality from the remaining text
    let (before_quality, data_quality) = if let Some(idx) = before_events.find("---DATA_QUALITY---")
    {
        let before = &before_events[..idx];
        let mut dq_str = before_events[idx + "---DATA_QUALITY---".len()..].trim();
        dq_str = dq_str
            .trim_start_matches("```json")
            .trim_start_matches("```")
            .trim_end_matches("```")
            .trim();
        // Validate it's parseable JSON, then store as raw string
        let validated: Option<String> = serde_json::from_str::<serde_json::Value>(dq_str)
            .map_err(|e| {
                tracing::warn!(error = %e, raw = dq_str, "Failed to parse data_quality from LLM");
                e
            })
            .ok()
            .map(|v| v.to_string());
        (before, validated)
    } else {
        (before_events, None)
    };

    // 3. Split off epigraph from the remaining text
    let (diary, epigraph) = if let Some(idx) = before_quality.find("---EPIGRAPH---") {
        let d = before_quality[..idx].trim().to_string();
        let e_raw = before_quality[idx + "---EPIGRAPH---".len()..].trim();
        // Epigraph is a single line — take only the first non-empty line
        let e = e_raw
            .lines()
            .map(str::trim)
            .find(|l| !l.is_empty())
            .map(|l| l.trim_matches(['"', '\'', '—', '–']).trim().to_string())
            .filter(|l| !l.is_empty());
        (d, e)
    } else {
        (before_quality.trim().to_string(), None)
    };

    ParsedDaySummary {
        diary,
        epigraph,
        data_quality,
        events,
    }
}

/// Store LLM-identified events as wiki_events rows.
///
/// Creates events in DB with location extraction. Embedding and novelty scoring
/// are handled separately by the dayline novelty pipeline (Phase 1).
async fn store_structured_events(
    pool: &PgPool,
    day: &WikiDay,
    date: NaiveDate,
    timezone: Option<&str>,
    events: &[LlmEvent],
) {
    // Clear previous auto events
    if let Err(e) = delete_auto_events_for_day(pool, day.id.clone()).await {
        tracing::warn!(error = %e, "Failed to delete existing auto events");
        return;
    }

    let tz: Option<Tz> = timezone.and_then(|s| s.parse().ok());

    // Backfill gaps to ensure perfect 24h coverage (00:00–24:00)
    let all_events = backfill_24h_events(events, date, tz.as_ref());

    let mut created_count = 0;

    for event in &all_events {
        let start_rfc = event.start_utc.to_rfc3339();
        let end_rfc = event.end_utc.to_rfc3339();

        // Extract auto_location from location_visit data (longest visit in time range)
        let auto_location = extract_event_location(pool, &start_rfc, &end_rfc).await;

        // Create the event row
        let created = create_temporal_event(
            pool,
            CreateTemporalEventRequest {
                day_id: day.id.clone(),
                start_time: event.start_utc,
                end_time: event.end_utc,
                auto_label: Some(event.label.clone()),
                auto_location,
                user_label: None,
                user_location: None,
                user_notes: None,
                // `source_ontologies` and `entities` are stamped afterwards by
                // `dayline::annotate` from the event's own time window — they
                // are facts about what the window contains, not about what the
                // model said.
                source_ontologies: None,
                is_unknown: Some(event.is_unknown),
                is_transit: Some(false),
                is_user_added: Some(false),
                event_summary: event.summary.clone(),
                topics: Some(serde_json::json!(event.topics)),
            },
        )
        .await;

        match created {
            Ok(_) => created_count += 1,
            Err(e) => {
                tracing::warn!(error = %e, label = event.label, "Failed to create temporal event");
            }
        }
    }

    tracing::info!(
        date = %date,
        event_count = all_events.len(),
        created_count,
        "Stored structured events"
    );
}

/// Extract the primary location for an event's time range from location_visit data.
/// Returns the place name with the longest visit duration, or None if no location data.
async fn extract_event_location(pool: &PgPool, start: &str, end: &str) -> Option<String> {
    use sqlx::Row;
    // `data_location_visit.place_name` is never populated by entity resolution —
    // the resolved name lives in `wiki_places`, linked via `wiki_refs`
    // (same shape the timeline reader uses). JOIN through to get the real name;
    // selecting the visit's own `place_name` column always returned NULL.
    let row: Option<sqlx::postgres::PgRow> = sqlx::query(
        "SELECT p.name AS place_name \
         FROM data_location_visit v \
         JOIN wiki_refs er \
           ON er.source_table = 'data_location_visit' \
          AND er.source_id = v.id \
          AND er.entity_type = 'place' \
         JOIN wiki_places p ON p.id = er.entity_id \
         WHERE v.started_at >= $1::timestamptz AND v.started_at <= $2::timestamptz \
         ORDER BY v.duration_minutes DESC LIMIT 1",
    )
    .bind(start)
    .bind(end)
    .fetch_optional(pool)
    .await
    .ok()
    .flatten();

    row.and_then(|r| r.try_get::<Option<String>, _>("place_name").ok().flatten())
        .filter(|s| !s.is_empty())
}

/// An event with pre-computed UTC times (either from LLM or gap-filled).
struct ResolvedEvent {
    start_utc: chrono::DateTime<chrono::Utc>,
    end_utc: chrono::DateTime<chrono::Utc>,
    label: String,
    summary: Option<String>,
    is_unknown: bool,
    topics: Vec<String>,
}

/// Take LLM events and produce a perfect 24h timeline (00:00–24:00) by filling gaps
/// with "Unknown" events. Events are sorted by start time and clamped to day boundaries.
fn backfill_24h_events(
    llm_events: &[LlmEvent],
    date: NaiveDate,
    tz: Option<&Tz>,
) -> Vec<ResolvedEvent> {
    // Day boundaries in UTC
    let day_start = parse_hhmm_to_utc("00:00", date, tz)
        .unwrap_or_else(|| date.and_hms_opt(0, 0, 0).unwrap().and_utc());
    let day_end = parse_hhmm_to_utc("00:00", date + chrono::Duration::days(1), tz)
        .unwrap_or_else(|| (date + chrono::Duration::days(1)).and_hms_opt(0, 0, 0).unwrap().and_utc());

    // Parse and sort LLM events
    let mut parsed: Vec<ResolvedEvent> = llm_events
        .iter()
        .filter_map(|e| {
            let start = parse_hhmm_to_utc(&e.start, date, tz)?;
            let end = parse_hhmm_to_utc(&e.end, date, tz)?;
            if end <= start { return None; } // skip invalid
            // Treat a literal "Unknown" label as an unknown block even when the
            // LLM emits it explicitly — keeps downstream classification honest.
            let is_unknown = e.label.eq_ignore_ascii_case("unknown");
            Some(ResolvedEvent {
                start_utc: start.max(day_start),
                end_utc: end.min(day_end),
                label: e.label.clone(),
                summary: e.summary.clone().filter(|s| !s.trim().is_empty()),
                is_unknown,
                topics: e.topics.clone(),
            })
        })
        .collect();
    parsed.sort_by_key(|e| e.start_utc);

    // Resolve overlaps: if event B starts before event A ends, truncate A's end to B's start.
    // If that makes A zero-width, drop it.
    let mut resolved: Vec<ResolvedEvent> = Vec::new();
    for event in parsed {
        if let Some(prev) = resolved.last_mut() {
            if event.start_utc < prev.end_utc {
                // Overlap: truncate previous event
                prev.end_utc = event.start_utc;
                if prev.end_utc <= prev.start_utc {
                    resolved.pop(); // zero-width, remove it
                }
            }
        }
        resolved.push(event);
    }

    // Build complete timeline with gaps filled
    let mut result: Vec<ResolvedEvent> = Vec::new();
    let mut cursor = day_start;

    for event in resolved {
        // Fill gap before this event
        if event.start_utc > cursor {
            result.push(ResolvedEvent {
                start_utc: cursor,
                end_utc: event.start_utc,
                label: "Unknown".to_string(),
                summary: None,
                is_unknown: true,
                topics: Vec::new(),
            });
        }
        cursor = event.end_utc;
        result.push(event);
    }

    // Fill gap after last event to end of day
    if cursor < day_end {
        result.push(ResolvedEvent {
            start_utc: cursor,
            end_utc: day_end,
            label: "Unknown".to_string(),
            summary: None,
            is_unknown: true,
            topics: Vec::new(),
        });
    }

    // Merge consecutive Unknown blocks into one — keeps the timeline cleaner
    // when the LLM emits its own "Unknown" event adjacent to a backfilled gap.
    let mut merged: Vec<ResolvedEvent> = Vec::with_capacity(result.len());
    for ev in result {
        if let Some(last) = merged.last_mut() {
            if last.is_unknown && ev.is_unknown && last.end_utc == ev.start_utc {
                last.end_utc = ev.end_utc;
                continue;
            }
        }
        merged.push(ev);
    }
    merged
}

/// Parse "HH:MM" string into UTC DateTime for the given date and timezone.
/// Handles "24:00" as midnight of the next day.
fn parse_hhmm_to_utc(
    hhmm: &str,
    date: NaiveDate,
    tz: Option<&Tz>,
) -> Option<chrono::DateTime<chrono::Utc>> {
    let parts: Vec<&str> = hhmm.split(':').collect();
    if parts.len() != 2 {
        return None;
    }
    let hour: u32 = parts[0].parse().ok()?;
    let minute: u32 = parts[1].parse().ok()?;

    // "24:00" means midnight of the next day
    if hour == 24 {
        let next_day = date + chrono::Duration::days(1);
        let naive = next_day.and_hms_opt(0, 0, 0)?;
        return if let Some(tz) = tz {
            tz.from_local_datetime(&naive)
                .earliest()
                .map(|dt| dt.with_timezone(&chrono::Utc))
        } else {
            Some(naive.and_utc())
        };
    }

    let naive = date.and_hms_opt(hour, minute, 0)?;

    if let Some(tz) = tz {
        tz.from_local_datetime(&naive)
            .earliest()
            .map(|dt| dt.with_timezone(&chrono::Utc))
    } else {
        Some(naive.and_utc())
    }
}


#[cfg(test)]
mod dossier_tests {
    use super::*;

    /// The exact shape observed live on 2025-12-16: instruction header with
    /// resolved links, a stranded rule beneath it, then the real article.
    /// The guard must remove the first two and not touch a legitimate `---`
    /// elsewhere in the prose.
    #[test]
    fn prompt_echo_is_stripped_and_real_rules_survive() {
        let leaked = "Entities you may link: [David](/person/p_1), [Jess](/person/p_2).\n\n---\n\nA clear, cold Tuesday.\n\n## The dashboard\n\nBody with [David](/person/p_1) linked inline.\n\n---\n\nA closing aside.";
        let cleaned = strip_prompt_echo(leaked);
        assert!(cleaned.starts_with("A clear, cold Tuesday."));
        assert!(!cleaned.contains("Entities you may link"));
        assert!(
            cleaned.contains("---"),
            "a rule that belongs to the article must survive"
        );
        assert!(cleaned.contains("[David](/person/p_1) linked inline"));

        // Clean prose passes through untouched.
        let clean = "A lede.\n\n## A section\n\nProse.";
        assert_eq!(strip_prompt_echo(clean), clean);
    }

    /// The 2025-12-16 fabrication: zero candidates offered, yet the model
    /// linked invented ids through the prose. Sanctioned links survive
    /// verbatim; invented ones lose the link and keep the name; non-ref
    /// markdown links are not the guard's business.
    #[test]
    fn invented_ref_links_are_unlinked_and_sanctioned_ones_survive() {
        let candidates = vec!["- [Maya](/person/person_demo_maya)".to_string()];
        let prose = "Standup with [Maya](/person/person_demo_maya) and \
                     [David](/person/person_5a4c), then the run at \
                     [Mueller trails](/place/place_5daf). See \
                     [the doc](https://example.com/x).";
        let cleaned = unlink_uninvited_refs(prose, &candidates);
        assert!(cleaned.contains("[Maya](/person/person_demo_maya)"));
        assert!(cleaned.contains("and David,"), "invented link keeps its name: {cleaned}");
        assert!(!cleaned.contains("person_5a4c"));
        assert!(!cleaned.contains("place_5daf"));
        assert!(cleaned.contains("[the doc](https://example.com/x)"));

        // Zero candidates: every ref-link is invented by definition.
        let none = unlink_uninvited_refs("Met [Jess](/person/person_9c2e).", &[]);
        assert_eq!(none, "Met Jess.");
    }

    /// The GAP regression: a subscribed calendar said "Community Dinner" while the
    /// owner sat at a Mac the whole evening, and the dossier showed only the plan.
    /// Both corrections have to reach the prompt or the detective cannot possibly
    /// get this right — it can only reason about lines it is given.
    ///
    /// ```sh
    /// DATABASE_URL=postgres://virtues:virtues@localhost:5432/virtues_mig_check \
    ///   cargo test -p virtues dossier_ -- --ignored --nocapture
    /// ```
    #[tokio::test]
    #[ignore = "needs Postgres with the migration chain applied"]
    async fn dossier_carries_device_presence_and_calendar_provenance() {
        let url = std::env::var("DATABASE_URL").expect("DATABASE_URL");
        let pool = PgPool::connect(&url).await.expect("connect");

        let date = NaiveDate::from_ymd_opt(2026, 7, 26).unwrap();
        let (start_str, end_str) = day_boundaries_utc(date, Some("UTC"));

        for t in ["data_calendar_event", "data_activity_app_session"] {
            sqlx::query(&format!("DELETE FROM {t}"))
                .execute(&pool)
                .await
                .expect("clean");
        }

        sqlx::query(
            "INSERT INTO data_calendar_event \
             (id,title,started_at,ended_at,is_all_day,source_stream_id,source_table, \
              source_provider,calendar_access_role,response_status) \
             VALUES ('g1','GAP Community Dinner','2026-07-26T18:30:00Z','2026-07-26T20:30:00Z', \
                     false,'gs1','google_calendar','google','reader',NULL)",
        )
        .execute(&pool)
        .await
        .expect("seed calendar");

        // Three sessions with sub-10-minute gaps: one presence RUN, not three lines.
        for (i, (app, s, e)) in [
            ("Claude", "18:34:00", "19:10:00"),
            ("Steam", "19:14:00", "19:40:00"),
            ("Slack", "19:45:00", "20:07:00"),
        ]
        .iter()
        .enumerate()
        {
            sqlx::query(
                "INSERT INTO data_activity_app_session \
                 (id,app_name,started_at,ended_at,source_stream_id,source_table, \
                  source_provider,attention,is_open,closed_by) \
                 VALUES ($1,$2,$3::timestamptz,$4::timestamptz,$5,'mac_apps','mac','active',false,$6)",
            )
            .bind(format!("a{i}"))
            .bind(app)
            .bind(format!("2026-07-26T{s}Z"))
            .bind(format!("2026-07-26T{e}Z"))
            .bind(format!("as{i}"))
            .bind(if i == 2 { "lock" } else { "switch" })
            .execute(&pool)
            .await
            .expect("seed session");
        }

        let dossier =
            build_dossier(&pool, date, &start_str, &end_str, Some("UTC"), None).await;
        println!("{dossier}");

        assert!(
            dossier.contains("SUBSCRIBED — someone else's calendar"),
            "a read-only calendar must be flagged as not the owner's plan"
        );
        assert!(
            dossier.contains("[device]"),
            "app sessions must reach the dossier at all — this is the whole fix"
        );
        assert!(
            dossier.contains("18:34–20:07"),
            "the three sessions must merge into ONE presence run spanning the event"
        );
        assert!(
            dossier.contains("ended: screen locked"),
            "the run's close reason explains the silence that follows it"
        );
        assert!(
            dossier.contains("typing/clicking"),
            "observed input is the strong form of presence and must be said so"
        );
        // No RSVP was recorded, and silence is not evidence in either direction.
        assert!(
            !dossier.contains("owner never replied") && !dossier.contains("accepted"),
            "a NULL response_status must produce NO rsvp claim"
        );
    }

    /// The arranged-occasion regression, in miniature. A coffee is arranged by
    /// text and confirmed that morning; the day page used to render that as a
    /// bare card transaction next to an unnamed conversation, because the
    /// detective was handed `- 14 with <name>` and nothing else. Messages must
    /// arrive PLACED IN TIME and CARRYING THEIR TEXT, or no prompt wording can
    /// rescue the cut. See agents/plan/attention-plan.md.
    ///
    /// Also pins the two properties that make that safe: bursts split on a real
    /// gap rather than smearing a day into one line, and a thread whose
    /// counterpart has no `wiki_people` row still reaches the spine — the events
    /// that matter most are often with someone the graph has never seen.
    ///
    /// ```sh
    /// DATABASE_URL=postgres://virtues:virtues@localhost:5432/virtues_mig_check \
    ///   cargo test -p virtues dossier_ -- --ignored --nocapture
    /// ```
    #[tokio::test]
    #[ignore = "needs Postgres with the migration chain applied"]
    async fn dossier_carries_message_bursts_with_text_and_time() {
        let url = std::env::var("DATABASE_URL").expect("DATABASE_URL");
        let pool = PgPool::connect(&url).await.expect("connect");

        let date = NaiveDate::from_ymd_opt(2026, 7, 27).unwrap();
        let (start_str, end_str) = day_boundaries_utc(date, Some("UTC"));

        sqlx::query("DELETE FROM data_communication_message")
            .execute(&pool)
            .await
            .expect("clean");

        // An unresolved correspondent: no wiki_people row, no entity ref. The old
        // aggregate would have filed this under "unknown"; the burst must still
        // carry the thread, both directions, and the text.
        let msgs: [(&str, &str, bool, &str); 5] = [
            ("m1", "07:02:00", false, "still on for 7:30 at the Hayes cafe?"),
            ("m2", "07:03:00", true, "yes! see you there"),
            ("m3", "07:04:00", false, "perfect"),
            // Four hours later — a SEPARATE occasion, not the same burst.
            ("m4", "11:20:00", true, "good to catch up"),
            ("m5", "11:31:00", false, "it really was"),
        ];
        for (id, hhmmss, from_me, body) in msgs {
            sqlx::query(
                "INSERT INTO data_communication_message \
                 (id,message_id,thread_id,channel,body,from_identifier,from_name, \
                  timestamp,source_stream_id,source_table,source_provider,metadata) \
                 VALUES ($1,$1,'th_demo','imessage',$2,$3,$4, \
                         $5::timestamptz,$1,'mac_imessage','mac',$6::jsonb)",
            )
            .bind(id)
            .bind(body)
            .bind(if from_me { "me" } else { "+15550101" })
            .bind(if from_me { None } else { Some("Sam") })
            .bind(format!("2026-07-27T{hhmmss}Z"))
            .bind(format!(r#"{{"is_from_me": {from_me}}}"#))
            .execute(&pool)
            .await
            .expect("seed message");
        }

        let dossier = build_dossier(&pool, date, &start_str, &end_str, Some("UTC"), None).await;
        println!("{dossier}");

        assert!(
            dossier.contains("[messages]"),
            "messages must reach the dossier as spine lines — this is the whole fix"
        );
        assert!(
            dossier.contains("07:02–07:04"),
            "a burst must be PLACED IN TIME so it can corroborate a window"
        );
        assert!(
            dossier.contains("11:20–11:31"),
            "a 4-hour gap is a new occasion, not a continuation of the morning"
        );
        assert!(
            dossier.contains("still on for 7:30 at the Hayes cafe?"),
            "the received half carries the arrangement — reading both directions \
             is exactly what lets the detective name the block"
        );
        assert!(
            dossier.contains("yes! see you there"),
            "the owner's own words must survive too"
        );
        assert!(
            dossier.contains("with Sam"),
            "an unresolved correspondent still names the thread from the message itself"
        );
        assert!(
            dossier.contains("(1 sent, 2 received)"),
            "direction counts must be honest — they are what the quoting rule keys on"
        );
        assert!(
            !dossier.contains("## Messages"),
            "the whole-day aggregate block is gone; a count off the spine taught \
             the detective nothing it could place"
        );
    }

    /// Runs narration's SQL against the schema the migrations actually build.
    ///
    /// `sqlx::query_as` is untyped, so a renamed column dies at RUNTIME, not
    /// build time — and it did: after the 2026-08-17 renames, the events query
    /// here still selected `start_time`, so `narrate_day` errored every night
    /// and no day was ever narrated (the door's "a page will be waiting for
    /// you" was silently false). An empty day exercises both statements — the
    /// events SELECT and the `narrated_at` read — and must come back `None`
    /// (below MIN_EVENTS_TO_NARRATE), never `Err`.
    #[sqlx::test]
    async fn narrate_day_sql_matches_schema(pool: sqlx::PgPool) {
        let date = chrono::NaiveDate::from_ymd_opt(2026, 8, 25).unwrap();
        let out = narrate_day(&pool, date)
            .await
            .expect("narration must not die on its own SQL");
        assert!(out.is_none(), "an empty day earns no story");
    }
}
