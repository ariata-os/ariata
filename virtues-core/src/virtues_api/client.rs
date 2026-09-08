//! api_key-authenticated client for the virtues-api routes.
//!
//! Attaches the box's device api_key (from the credential vault) on every call.
//! No renewal — the key is stable; the wallet behind it is credited
//! server-side. On a `wallet_empty` 402 it triggers one auto-top-up via atlas
//! (which charges the card + credits the wallet) and retries; other 402s
//! (wallet_expired) and 401 (unknown_key → re-link) surface to the caller.
//!
//! Use this for the proxy routes (`/v1/ai/*`, `/v1/places/*`, `/v1/parallel/*`,
//! `/v1/unsplash/*`).
//!
//! ## The BYO fork
//!
//! When the user has set a BYO provider key, **every `/v1/ai/*` call** leaves
//! by the user's endpoint instead of ours — streaming via [`Self::stream`],
//! non-streaming via [`Self::post_json`]. Both consult
//! [`crate::api::settings_byo::load_byo_credential`] and divert before any
//! bearer is read, so virtues-api (wallet, markup, caps, auto-top-up) is out
//! of the inference path entirely. That is the whole point of BYO.
//!
//! The fork is keyed on [`is_ai_path`] — the same predicate that decides cost
//! capture — rather than on a separate `post_ai()` method, specifically so a
//! new AI caller cannot forget to opt in. That mattered: until 2026-08-05
//! only `stream()` honored the key, and compaction, day summaries, image
//! generation and transcription quietly billed the wallet while the UI said
//! "BYO active". Non-AI routes (`/v1/places/*`, `/v1/parallel/*`, `/v1/unsplash/*`)
//! are per-user vendor bills that BYO says nothing about, so they keep going
//! through the wallet. Plan of record: `agents/plan/byo-ai-plan.md`.
//!
//! ## Purpose tagging (vestige — no-op)
//!
//! Calls still carry an `X-Virtues-Purpose` header (`user`/`system`), but the
//! server **ignores** it — the old OS-reserve / chat-wallet two-pool split was
//! removed when billing collapsed to a single wallet. `Purpose` +
//! `with_purpose()` are kept only so background callers don't have to change;
//! they have no billing effect and the whole thing is safe to delete later.

use anyhow::{anyhow, Result};
use serde_json::{json, Value};
use sqlx::PgPool;

use super::renew;

/// Is this one of the metered *inference* routes, as opposed to the fixed-cost
/// vendor proxies (`/v1/places/*`, `/v1/parallel/*`, `/v1/unsplash/*`)?
///
/// Two things key on this and must not drift apart: whether a call may divert
/// to the user's BYO endpoint, and whether its `usage` block is captured into
/// `app_ai_calls`. A route that is one but not the other would either bill a
/// BYO call to the wallet or record a wallet call as free.
fn is_ai_path(path: &str) -> bool {
    path.starts_with("/v1/ai/")
}

/// How many times a chat completion may come back empty before we give up.
///
/// A reasoning model can spend its entire output budget thinking and return a
/// message with no content at all. Measured on the box 2026-08-11, segmenting
/// 2026-08-09 on `xai/grok-4.5`: `completion_tokens = 1009`, of which
/// `reasoning_tokens = 1009` — zero content tokens, billed in full. The caller
/// saw "LLM returned empty summary" and that day's autobiography was lost with
/// no retry; 2026-08-03 died the same way on a different model. Raising
/// `max_tokens` does not fix it (the same run recorded 7214 completion tokens
/// against a 4000 cap, so reasoning is not bounded by it), and
/// `reasoning_effort` is a no-op on THAT model — a resend is the only lever
/// there. (Sonnet 5 honors it; `completion::system_completion` sends it.)
///
/// 3 attempts: empty output is sporadic rather than deterministic, so a couple
/// of resends is the difference between losing a day and not, while still
/// bounding the spend on a model that has genuinely stopped answering.
const EMPTY_COMPLETION_ATTEMPTS: u32 = 3;

/// A successful chat completion carrying no assistant text.
///
/// Gated on `choices` being present so it only judges chat-shaped responses,
/// and on success so a real error status falls through to the caller's own
/// handling untouched.
fn is_empty_completion(resp: &ApiResponse) -> bool {
    if !resp.is_success() {
        return false;
    }
    let Some(choices) = resp.body["choices"].as_array() else {
        return false;
    };
    // An empty `choices` array is the same failure wearing a different shape.
    choices.iter().all(|c| {
        c["message"]["content"]
            .as_str()
            .map(|s| s.trim().is_empty())
            .unwrap_or(true)
    })
}

/// Turn an upstream's "HTTP 200, but actually an error" into a real error.
///
/// Some gateways answer a failed call with a 2xx whose body carries `error`
/// and no `choices`. **Verified against OpenRouter 2026-08-05**: sending audio
/// in our `image_url` data-URI shape returns HTTP 200 with
/// `error.message = "Failed to load image from data:audio/wav;base64,…"` —
/// the upstream tried to decode the audio as an image, failed, and said so at
/// status 200.
///
/// Left alone, that is the only *quiet* BYO failure. Every other one we probed
/// is loud: a wrong model id 400s, a missing model 400s, an unfunded account
/// 402s. This one passes `is_success()`, so callers fall through to parsing
/// and report something misleading — the transcription applet would raise
/// "missing choices[0].message.content", blaming its own parse for the
/// upstream's refusal, and then retry every cron tick, re-billing the user's
/// key for audio that can never succeed in that shape.
///
/// So: a 2xx carrying `error` and no `choices` becomes a 502 with the
/// upstream's own message preserved. The user reads why their provider said
/// no, which is the whole point of not translating provider errors.
fn normalize_upstream_error(resp: ApiResponse) -> ApiResponse {
    if !resp.is_success() || resp.body.get("choices").is_some() {
        return resp;
    }
    let Some(err) = resp.body.get("error") else {
        return resp;
    };
    let message = err
        .get("message")
        .and_then(|m| m.as_str())
        .unwrap_or("upstream reported an error without a message");
    tracing::warn!(
        upstream_status = resp.status,
        message,
        "BYO upstream returned an error body at a success status; treating as 502"
    );
    ApiResponse {
        status: 502,
        body: json!({
            "error": "byo_upstream_error",
            "message": message,
            "upstream_status": resp.status,
        }),
    }
}

/// Re-address a request body for the user's endpoint.
///
/// **A model id is an address on one gateway, not a portable name.** Callers
/// build bodies with ours — `spacexai/grok-4.5` is where *Vercel* keeps the
/// chat model — and every caller names one explicitly. OpenRouter spells that
/// same model `x-ai/grok-4.5` and does not carry `google/gemini-3-flash` at
/// all. So
/// sending our string to their endpoint is not a near miss; it is the wrong
/// kind of thing.
///
/// The fix is to let the address belong to the route while the slot keeps only
/// the role. We turn the body's model back into the slot it stands for, then
/// look up what the user calls that slot. Nothing here knows or cares *which*
/// slot — Omni is not special-cased, and must not be. Which model actually
/// suits a role is advice, and advice belongs in the UI where it can change
/// without a release.
///
/// Untranslated cases pass through deliberately:
///
/// - **The user's map has no entry for the slot.** Their endpoint may well use
///   our ids (Vercel does; a LiteLLM can be aliased to). If it does not, the
///   route answers with a loud 400 naming the model, which reads better than
///   anything we could substitute.
/// - **The body's model is not a slot default**, i.e. the user pinned an
///   arbitrary model from the picker. That choice is theirs; we do not
///   second-guess it.
/// - **The body names no model at all.** No caller does this today, but the
///   legacy `default_model` still fills it if set.
fn apply_byo_model(body: &Value, byo: &crate::api::settings_byo::ByoCredential) -> Value {
    let mut body = body.clone();
    let Value::Object(map) = &mut body else {
        return body;
    };

    let ours = map.get("model").and_then(|m| m.as_str()).map(String::from);
    let Some(ours) = ours else {
        // Unpinned body: the legacy single-model field is the only thing that
        // can speak to it, since we have no slot to look up.
        if let Some(fallback) = byo.default_model.as_deref() {
            map.insert("model".into(), Value::String(fallback.into()));
        }
        return body;
    };

    if let Some(slot) = crate::api::model_catalog::slot_for_model(&ours) {
        if let Some(theirs) = byo.models.get(slot.as_str()) {
            tracing::debug!(slot = slot.as_str(), ours = %ours, theirs = %theirs, "re-addressed for BYO route");
            map.insert("model".into(), Value::String(theirs.clone()));
        }
    }
    body
}

/// The sentence for a metered 402, with its code — prose for the owner, code
/// for the support conversation, in one string because every caller here
/// surfaces a single `Error::ExternalApi(String)`.
///
/// Reads the box's last-known entitlement to tell two states apart that the
/// wire cannot: virtues-api answers `wallet_empty` to a never-subscribed
/// account and to a paying one whose balance ran out, and "Usage limit
/// reached" / "add credits" is the wrong instruction to the first. Someone who
/// never subscribed has not hit a limit; they need a subscription. This is the
/// moment-of-need — the one place the offer is honest — so the sentence must
/// name the right door.
pub fn payment_required_message(body: &Value, feature: &str) -> String {
    let code = body["error"]["code"].as_str().unwrap_or("payment_required");
    let what = feature.replace('_', " ");
    let free = matches!(
        crate::api::subscription::last_known_entitlement(),
        Some(crate::virtues_api::renew::Entitlement::Free)
    );
    let msg = match code {
        "wallet_empty" | "insufficient_budget" if free => format!(
            "{what} needs a Virtues subscription — there is no wallet behind this server yet. Subscribe in Settings → Billing, or bring your own AI key."
        ),
        "wallet_empty" | "insufficient_budget" => format!(
            "Wallet empty — {what} is paused until it is topped up (Settings → Billing)."
        ),
        "topup_disabled" => format!(
            "Wallet empty and auto top-up is off — {what} is paused. Top up in Settings → Billing."
        ),
        "card_declined" | "authentication_required" => format!(
            "Auto top-up failed (card) — {what} is paused. Update your card through Stripe in Settings → Billing."
        ),
        "monthly_cap_reached" => format!(
            "Monthly spending cap reached — {what} is paused until next month, or raise the cap in Settings → Billing."
        ),
        "subscription_inactive" | "wallet_expired" => format!(
            "Your subscription has lapsed — {what} is paused until it renews."
        ),
        "unknown_key" | "invalid_api_key" => format!(
            "This server's Virtues key is not recognized — reconnect your account in Settings → Billing. {what} is paused."
        ),
        "call_too_expensive" => format!("{what} was refused: the call would exceed the wallet."),
        _ => format!("Payment required — {what} is paused."),
    };
    format!("{msg} [{code}]")
}

/// Convert an `AutoTopupOutcome` non-Funded variant into the same 402
/// error shape virtues-api would have returned. Lets iOS handle every
/// error via one contract regardless of whether the box did recovery.
fn synthesize_topup_failure(outcome: renew::AutoTopupOutcome) -> ApiResponse {
    let (code, extra) = match &outcome {
        renew::AutoTopupOutcome::CardDeclined { stripe_code, message } => (
            "card_declined",
            json!({ "stripe_code": stripe_code, "message": message }),
        ),
        renew::AutoTopupOutcome::AuthenticationRequired { payment_intent } => (
            "authentication_required",
            json!({ "payment_intent": payment_intent }),
        ),
        renew::AutoTopupOutcome::MonthlyCapReached { cap_micros, charged_micros } => (
            "monthly_cap_reached",
            json!({
                "monthly_cap_micros": cap_micros,
                "monthly_charges_micros": charged_micros,
            }),
        ),
        renew::AutoTopupOutcome::SubscriptionInactive => (
            "subscription_inactive",
            json!({ "message": "subscription is not active" }),
        ),
        // Funded shouldn't reach here.
        renew::AutoTopupOutcome::Funded { .. } => (
            "internal",
            json!({ "message": "unexpected Funded in synthesize" }),
        ),
    };
    let mut error = json!({ "code": code });
    if let Some(obj) = extra.as_object() {
        for (k, v) in obj {
            error[k] = v.clone();
        }
    }
    ApiResponse {
        status: 402,
        body: json!({ "error": error }),
    }
}

/// Synthesize the same 402 shape used for top-up failures, but with code
/// `topup_disabled`. Bubbles up when the user has turned auto-top-up off
/// or the breaker tripped after 3 consecutive failures in 24h.
fn synthesize_topup_disabled() -> ApiResponse {
    ApiResponse {
        status: 402,
        body: json!({
            "error": {
                "code": "topup_disabled",
                "message":
                    "auto-top-up is disabled (either by you or by the \
                     3-failure circuit breaker). Top up manually from \
                     Settings → Billing, or set a BYO provider key.",
            }
        }),
    }
}

/// Check whether the box should attempt auto-top-up for the next 402.
/// False when the user has explicitly disabled it, or when the
/// `auto_topup_failures_24h` counter has reached the breaker threshold.
async fn auto_topup_allowed(pool: &sqlx::PgPool) -> bool {
    let row: Option<(bool, i32)> = sqlx::query_as(
        "SELECT auto_topup_enabled, auto_topup_failures_24h \
         FROM app_user_profile \
         WHERE id = '00000000-0000-0000-0000-000000000001'",
    )
    .fetch_optional(pool)
    .await
    .ok()
    .flatten();
    match row {
        Some((enabled, failures)) => enabled && failures < AUTO_TOPUP_FAILURE_THRESHOLD,
        // Singleton row should always exist; if it somehow doesn't, default
        // to allowed (matches the column default).
        None => true,
    }
}

/// Record a successful top-up: reset the failure counter, and enforce the
/// RATE guard. Counts auto-top-ups within a rolling 24h window; if the count
/// exceeds `AUTO_TOPUP_MAX_PER_WINDOW`, trip the breaker
/// (`auto_topup_enabled = FALSE`) so a runaway loop can't keep charging the
/// card unboundedly (the failure breaker only catches *failed* charges). The
/// monthly cap bounds total spend; this bounds the velocity.
async fn record_topup_success(pool: &sqlx::PgPool) -> Result<(), sqlx::Error> {
    let now = chrono::Utc::now();
    let row: Option<(i32, Option<chrono::DateTime<chrono::Utc>>)> = sqlx::query_as(
        "SELECT auto_topup_count_window, auto_topup_window_start \
         FROM app_user_profile \
         WHERE id = '00000000-0000-0000-0000-000000000001'",
    )
    .fetch_optional(pool)
    .await?;
    // Roll the window if it's stale (or unset); otherwise increment within it.
    let (count, window_start) = match row {
        Some((c, Some(start))) if now.signed_duration_since(start).num_seconds() < AUTO_TOPUP_WINDOW_SECS => {
            (c + 1, start)
        }
        _ => (1, now),
    };
    let tripped = count > AUTO_TOPUP_MAX_PER_WINDOW;
    if tripped {
        tracing::warn!(
            count,
            "auto-top-up rate guard tripped — disabling auto-top-up (too many refills in 24h)"
        );
    }
    sqlx::query(
        "UPDATE app_user_profile \
         SET auto_topup_failures_24h = 0, \
             auto_topup_count_window = $1, \
             auto_topup_window_start = $2, \
             auto_topup_enabled = CASE WHEN $3 THEN FALSE ELSE auto_topup_enabled END, \
             auto_topup_disabled_at = CASE WHEN $3 THEN now() ELSE NULL END \
         WHERE id = '00000000-0000-0000-0000-000000000001'",
    )
    .bind(count)
    .bind(window_start)
    .bind(tripped)
    .execute(pool)
    .await
    .map(|_| ())
}

/// Increment the failure counter; if we hit the breaker threshold, flip
/// `auto_topup_enabled = FALSE` and record the timestamp. The user can
/// re-enable from `/settings/billing` after fixing whatever was wrong
/// (declined card, payment method, etc).
async fn record_topup_failure(pool: &sqlx::PgPool) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE app_user_profile \
         SET auto_topup_failures_24h = auto_topup_failures_24h + 1, \
             auto_topup_enabled = CASE \
                 WHEN auto_topup_failures_24h + 1 >= $1 THEN FALSE \
                 ELSE auto_topup_enabled \
             END, \
             auto_topup_disabled_at = CASE \
                 WHEN auto_topup_failures_24h + 1 >= $1 THEN now() \
                 ELSE auto_topup_disabled_at \
             END \
         WHERE id = '00000000-0000-0000-0000-000000000001'",
    )
    .bind(AUTO_TOPUP_FAILURE_THRESHOLD)
    .execute(pool)
    .await
    .map(|_| ())
}

/// How many consecutive auto-top-up failures we tolerate before the
/// breaker trips and auto-top-up is disabled. The counter is reset by a
/// successful top-up OR by the sweeper rolling the daily window.
const AUTO_TOPUP_FAILURE_THRESHOLD: i32 = 3;

/// Rolling window for the success-rate guard, and the max auto-top-ups allowed
/// within it before the breaker trips. At the $20 default that's ~$100/24h of
/// auto-refills before the box stops charging the card and asks the user to
/// intervene — a velocity bound on runaway loops, complementing the monthly cap.
const AUTO_TOPUP_WINDOW_SECS: i64 = 24 * 60 * 60;
const AUTO_TOPUP_MAX_PER_WINDOW: i32 = 5;

/// A fully-read virtues-api response: status + parsed JSON body.
pub struct ApiResponse {
    pub status: u16,
    pub body: Value,
}

impl ApiResponse {
    pub fn is_success(&self) -> bool {
        (200..300).contains(&self.status)
    }
}

/// Outcome of opening a streaming request. On success the body is left
/// untouched so the caller can read the SSE stream; on a non-2xx the error
/// body is already drained into `body` (we can't both peek and stream).
pub enum StreamOutcome {
    Stream(reqwest::Response),
    Error { status: u16, body: String },
}

/// What the call is for. Sets the `X-Virtues-Purpose` header for telemetry,
/// but the server ignores it (single wallet now — no per-purpose routing).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Purpose {
    /// User-initiated work (chat, agent loops, on-demand search).
    User,
    /// Box-essential background work (transcription, ER, summaries, event
    /// summaries, embeddings indexing). No billing difference from `User`.
    System,
}

impl Purpose {
    fn as_str(&self) -> &'static str {
        match self {
            Self::User => "user",
            Self::System => "system",
        }
    }
}

#[derive(Clone, Debug)]
pub struct BearerClient {
    http: reqwest::Client,
    /// For non-streaming `/v1/ai/*` POSTs. Nothing arrives until the model
    /// finishes the whole generation, so these get the completion timeout
    /// rather than the 60s request timeout — the nightly day-summary
    /// segmentation died at exactly 60s for three days straight (2026-08-09..11)
    /// before this split existed. Keyed on [`is_ai_path`], like the BYO fork
    /// and cost capture.
    completion_http: reqwest::Client,
    stream_http: reqwest::Client,
    pool: PgPool,
    api_url: String,
    atlas_url: String,
    purpose: Purpose,
    /// Feature bucket for box-local cost capture (`app_ai_calls`). Set by
    /// background callers (e.g. "transcription", "day_summary"); falls back to
    /// the purpose tag. The streaming chat path records its own row (cost only
    /// arrives in the SSE trailer), so this drives the non-streaming `post_json`
    /// capture.
    feature: Option<String>,
}

impl BearerClient {
    pub fn from_env(pool: PgPool) -> Self {
        let api_url = super::api_url();
        let atlas_url = super::atlas_url();
        Self {
            http: crate::http_client::virtues_api_client(),
            completion_http: crate::http_client::virtues_api_completion_client(),
            stream_http: crate::http_client::virtues_api_streaming_client(),
            pool,
            api_url,
            atlas_url,
            // No-op now (single wallet); just the default telemetry tag.
            purpose: Purpose::User,
            feature: None,
        }
    }

    /// Override the call purpose (telemetry only — no billing effect; see
    /// [`Purpose`]). Background callers tag `Purpose::System`.
    pub fn with_purpose(mut self, purpose: Purpose) -> Self {
        self.purpose = purpose;
        self
    }

    /// Tag the cost bucket recorded into `app_ai_calls` for this client's
    /// `post_json` calls to `/v1/ai/*` (e.g. "transcription", "day_summary").
    pub fn with_feature(mut self, feature: impl Into<String>) -> Self {
        self.feature = Some(feature.into());
        self
    }

    /// POST JSON to a virtues-api route, attaching the api_key. On a
    /// `wallet_empty`/`insufficient_budget` 402 it triggers one auto-top-up via
    /// atlas (`/credits/auto-topup`) and retries once; typed errors
    /// (`card_declined`, `monthly_cap_reached`, …) and other 402s surface to the
    /// caller. A 401 (`unknown_key`) means the box must re-link.
    ///
    /// For `/v1/ai/*` calls, the authoritative `usage.cost` in the response is
    /// captured into `app_ai_calls` here — the single chokepoint, so every
    /// non-streaming AI feature (compaction, day summaries, transcription, …)
    /// is accounted for without per-caller bookkeeping.
    ///
    /// **BYO fork.** For `/v1/ai/*` a configured BYO key diverts the call to
    /// the user's own endpoint before any bearer is read — see the module
    /// docs. Non-AI routes never divert.
    pub async fn post_json(&self, path: &str, body: &Value) -> Result<ApiResponse> {
        if is_ai_path(path) {
            if let Ok(Some(byo)) = crate::api::settings_byo::load_byo_credential(&self.pool).await {
                return self.post_direct_upstream(path, body, &byo).await;
            }
        }
        let bearer = self.ensure_bearer().await?;
        let mut attempt = 0;
        loop {
            attempt += 1;
            let resp = self.send(path, body, &bearer).await?;
            let resp = self.handle_402_and_retry_post(path, body, resp).await?;
            // Every attempt is billed, so every attempt is recorded — a retry
            // that vanished from `app_ai_calls` would make the wallet lie.
            self.record_ai_usage(path, body, &resp, crate::api::ai_calls::Route::Wallet)
                .await;
            if attempt < EMPTY_COMPLETION_ATTEMPTS && is_empty_completion(&resp) {
                tracing::warn!(
                    path,
                    attempt,
                    "completion came back with no content (all output spent on \
                     reasoning) — retrying"
                );
                continue;
            }
            return Ok(resp);
        }
    }

    /// BYO path for non-streaming AI calls — the `post_json` twin of
    /// [`Self::stream_direct_upstream`].
    ///
    /// Deliberately has **no 402 handling**: auto-top-up exists to refill our
    /// wallet, and this call never touches it. Whatever the user's provider
    /// says — 401 on a revoked key, 404 on a model their gateway does not
    /// carry, 429 on their own rate limit — is returned verbatim, because
    /// translating it would only obscure whose limit was hit. There is also no
    /// silent fallback to the wallet: spending the user's Virtues balance to
    /// paper over their misconfiguration is exactly the surprise BYO exists to
    /// prevent.
    async fn post_direct_upstream(
        &self,
        path: &str,
        body: &Value,
        byo: &crate::api::settings_byo::ByoCredential,
    ) -> Result<ApiResponse> {
        let body = apply_byo_model(body, byo);
        // Always an AI completion (only `is_ai_path` routes divert here), so
        // it gets the completion timeout like the wallet path in `send`.
        let resp = self
            .completion_http
            .post(&byo.endpoint_url)
            .header("Authorization", format!("Bearer {}", byo.api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| anyhow!("BYO upstream request failed: {e}"))?;
        let status = resp.status().as_u16();
        let resp = normalize_upstream_error(ApiResponse {
            status,
            body: resp.json::<Value>().await.unwrap_or_else(|_| json!({})),
        });
        // Still recorded, still keyed on the model actually sent. The row's
        // `cost_micros` lands at 0 because no upstream but our own gateway
        // reports `usage.cost` — and 0 is the honest number here, since
        // `app_ai_calls` measures what the *wallet* spent, which for a BYO call
        // is nothing. Presenting that as the user's total AI cost would be the
        // lie; showing tokens instead is `agents/plan/byo-ai-plan.md` phase 5.
        self.record_ai_usage(path, &body, &resp, crate::api::ai_calls::Route::Byo)
            .await;
        Ok(resp)
    }

    /// Best-effort: record one `app_ai_calls` row for a successful `/v1/ai/*`
    /// response that carries a `usage` block. Never fails the request.
    async fn record_ai_usage(
        &self,
        path: &str,
        req_body: &Value,
        resp: &ApiResponse,
        route: crate::api::ai_calls::Route,
    ) {
        if !is_ai_path(path) || !resp.is_success() {
            return;
        }
        let Some(usage) = resp.body.get("usage") else { return };
        let as_i64 = |k: &str| usage.get(k).and_then(|v| v.as_i64()).unwrap_or(0);
        let cost_micros = usage
            .get("cost")
            .and_then(|c| c.as_f64())
            .map(|c| (c * 1_000_000.0).round() as i64)
            .unwrap_or(0);
        let reasoning = usage
            .get("completion_tokens_details")
            .and_then(|d| d.get("reasoning_tokens"))
            .and_then(|v| v.as_i64())
            .unwrap_or(0);
        let feature = self
            .feature
            .clone()
            .unwrap_or_else(|| self.purpose.as_str().to_string());
        let call = crate::api::ai_calls::AiCall {
            feature,
            model: req_body
                .get("model")
                .and_then(|m| m.as_str())
                .unwrap_or_default()
                .to_string(),
            prompt_tokens: as_i64("prompt_tokens"),
            completion_tokens: as_i64("completion_tokens"),
            reasoning_tokens: reasoning,
            cost_micros,
            route,
            applet_run_id: None,
        };
        if let Err(e) = crate::api::ai_calls::record_ai_call(&self.pool, &call).await {
            tracing::warn!(error = %e, "failed to record ai_call (post_json)");
        }
    }

    /// GET a virtues-api route (api_key attached). Same recovery as `post_json`.
    pub async fn get_json(&self, path: &str) -> Result<ApiResponse> {
        let bearer = self.ensure_bearer().await?;
        let resp = self.send_get(path, &bearer).await?;
        self.handle_402_and_retry_get(path, resp).await
    }

    /// GET a virtues-api route that must work before linking (the model
    /// catalog). Attaches the api_key when the box has one, sends bare
    /// otherwise — `get_json` refuses outright when unlinked, which is
    /// exactly right for billable routes and wrong for public ones. No 402
    /// recovery: public routes never charge.
    pub async fn get_json_public(&self, path: &str) -> Result<ApiResponse> {
        let mut req = self
            .http
            .get(format!("{}{}", self.api_url.trim_end_matches('/'), path))
            .header("X-Virtues-Purpose", self.purpose.as_str());
        if let Ok(bearer) = self.ensure_bearer().await {
            req = req.header("Authorization", format!("Bearer {}", bearer));
        }
        let resp = req
            .send()
            .await
            .map_err(|e| anyhow!("virtues-api request failed: {e}"))?;
        let status = resp.status().as_u16();
        let body = resp.json::<Value>().await.unwrap_or_else(|_| json!({}));
        Ok(ApiResponse { status, body })
    }

    async fn handle_402_and_retry_post(
        &self,
        path: &str,
        body: &Value,
        resp: ApiResponse,
    ) -> Result<ApiResponse> {
        if resp.status != 402 {
            return Ok(resp);
        }
        let code = resp.body["error"]["code"].as_str().unwrap_or("");
        match code {
            "insufficient_budget" | "wallet_empty" => {
                self.auto_topup_and_retry_post(path, body).await
            }
            // wallet_expired (sub lapsed), call_too_expensive,
            // unknown_key (re-link) — not recoverable from the box; surface.
            _ => Ok(resp),
        }
    }

    async fn handle_402_and_retry_get(
        &self,
        path: &str,
        resp: ApiResponse,
    ) -> Result<ApiResponse> {
        if resp.status != 402 {
            return Ok(resp);
        }
        let code = resp.body["error"]["code"].as_str().unwrap_or("");
        match code {
            "insufficient_budget" | "wallet_empty" => {
                self.auto_topup_and_retry_get(path).await
            }
            _ => Ok(resp),
        }
    }

    /// Trigger auto-top-up via atlas, then retry the failed POST. On a
    /// non-`Funded` outcome (card declined, monthly cap reached, etc) we
    /// return a synthetic 402 response with the same shape virtues-api
    /// would have returned, so the iOS app gets a unified error contract.
    ///
    /// **Circuit breaker:** if auto-top-up is disabled by the user OR has
    /// failed 3 times in the last 24h (counter persisted on
    /// `app_user_profile`), we skip the call entirely and bubble the
    /// original 402. The user must intervene (top up manually, fix card,
    /// or switch to BYO key).
    async fn auto_topup_and_retry_post(
        &self,
        path: &str,
        body: &Value,
    ) -> Result<ApiResponse> {
        if !auto_topup_allowed(&self.pool).await {
            return Ok(synthesize_topup_disabled());
        }
        match renew::auto_topup(&self.pool, &self.http, &self.atlas_url).await? {
            renew::AutoTopupOutcome::Funded { amount_micros } => {
                let _ = record_topup_success(&self.pool).await;
                tracing::info!(amount_micros, "auto-top-up funded; retrying request");
                let bearer = self.ensure_bearer().await?;
                self.send(path, body, &bearer).await
            }
            other => {
                let _ = record_topup_failure(&self.pool).await;
                Ok(synthesize_topup_failure(other))
            }
        }
    }

    async fn auto_topup_and_retry_get(&self, path: &str) -> Result<ApiResponse> {
        if !auto_topup_allowed(&self.pool).await {
            return Ok(synthesize_topup_disabled());
        }
        match renew::auto_topup(&self.pool, &self.http, &self.atlas_url).await? {
            renew::AutoTopupOutcome::Funded { amount_micros } => {
                let _ = record_topup_success(&self.pool).await;
                tracing::info!(amount_micros, "auto-top-up funded; retrying GET");
                let bearer = self.ensure_bearer().await?;
                self.send_get(path, &bearer).await
            }
            other => {
                let _ = record_topup_failure(&self.pool).await;
                Ok(synthesize_topup_failure(other))
            }
        }
    }

    /// Open a streaming POST to a virtues-api route. Auto-top-up can only
    /// happen *before* the body starts flowing (mid-stream recovery is
    /// impossible), so on a `wallet_empty` 402 at connect time we top up and
    /// retry once. On success the response body is returned untouched for the
    /// caller to stream; non-recoverable 402s (card_declined, wallet_expired, …)
    /// come back as `StreamOutcome::Error` with the drained body.
    pub async fn stream(&self, path: &str, body: &Value) -> Result<StreamOutcome> {
        // BYO fork — the streaming twin of the one in `post_json`. Gated on
        // the same `is_ai_path` predicate so the two cannot drift, even though
        // every caller of `stream()` today is already an AI route.
        if is_ai_path(path) {
            if let Ok(Some(byo)) = crate::api::settings_byo::load_byo_credential(&self.pool).await {
                return self.stream_direct_upstream(body, &byo).await;
            }
        }

        let bearer = self.ensure_bearer().await?;
        let resp = self.send_stream(path, body, &bearer).await?;

        if resp.status().as_u16() == 402 {
            let err_body = resp.text().await.unwrap_or_default();
            if err_body.contains("insufficient_budget") || err_body.contains("wallet_empty") {
                match renew::auto_topup(&self.pool, &self.http, &self.atlas_url).await? {
                    renew::AutoTopupOutcome::Funded { amount_micros } => {
                        tracing::info!(amount_micros, "auto-top-up funded; retrying stream");
                        let bearer = self.ensure_bearer().await?;
                        let retry = self.send_stream(path, body, &bearer).await?;
                        return Ok(Self::classify_stream(retry).await);
                    }
                    other => {
                        let synth = synthesize_topup_failure(other);
                        return Ok(StreamOutcome::Error {
                            status: synth.status,
                            body: synth.body.to_string(),
                        });
                    }
                }
            }
            return Ok(StreamOutcome::Error { status: 402, body: err_body });
        }
        Ok(Self::classify_stream(resp).await)
    }

    async fn classify_stream(resp: reqwest::Response) -> StreamOutcome {
        let status = resp.status().as_u16();
        if (200..300).contains(&status) {
            StreamOutcome::Stream(resp)
        } else {
            let body = resp.text().await.unwrap_or_default();
            StreamOutcome::Error { status, body }
        }
    }

    async fn send_stream(
        &self,
        path: &str,
        body: &Value,
        bearer: &str,
    ) -> Result<reqwest::Response> {
        self.stream_http
            .post(format!("{}{}", self.api_url.trim_end_matches('/'), path))
            .header("Authorization", format!("Bearer {}", bearer))
            .header("X-Virtues-Purpose", self.purpose.as_str())
            .header("Content-Type", "application/json")
            .json(body)
            .send()
            .await
            .map_err(|e| anyhow!("virtues-api stream request failed: {e}"))
    }

    /// BYO path: POST the chat body to the user's configured upstream
    /// endpoint with their API key. v1 supports OpenAI-compatible APIs
    /// (OpenAI, xAI, OpenRouter, LiteLLM, custom proxies). For
    /// Anthropic-native or Google-native body shapes the user points BYO
    /// at a translation proxy (LiteLLM / OpenRouter) — keeps the per-
    /// provider request-shape mess out of our codebase.
    ///
    /// Model selection follows [`apply_byo_model`]. Same no-fallback rule as
    /// [`Self::post_direct_upstream`]: a failure here surfaces as the
    /// provider's own error, never as a quiet wallet charge.
    async fn stream_direct_upstream(
        &self,
        body: &Value,
        byo: &crate::api::settings_byo::ByoCredential,
    ) -> Result<StreamOutcome> {
        let body = apply_byo_model(body, byo);
        let resp = self
            .stream_http
            .post(&byo.endpoint_url)
            .header("Authorization", format!("Bearer {}", byo.api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| anyhow!("BYO upstream request failed: {e}"))?;
        Ok(Self::classify_stream(resp).await)
    }

    /// Return the device api_key to authenticate with. No renewal — the key
    /// is stable; the wallet behind it is credited server-side.
    ///
    /// Dev override: when `VIRTUES_API_KEY` is set we present it verbatim and
    /// skip the vault. Pairs with the gated dev seed in virtues-api
    /// (`ENVIRONMENT=dev`), which registers a device key keyed by
    /// `sha256(VIRTUES_API_KEY)` against a funded account. Unset in prod → no
    /// effect.
    async fn ensure_bearer(&self) -> Result<String> {
        if let Ok(key) = std::env::var("VIRTUES_API_KEY") {
            if !key.is_empty() {
                return Ok(key);
            }
        }
        renew::read_api_key(&self.pool)
            .await?
            .ok_or_else(|| anyhow!("no virtues_api key — link a subscription first"))
    }

    async fn send(&self, path: &str, body: &Value, bearer: &str) -> Result<ApiResponse> {
        // AI completions block until the model finishes generating; everything
        // else answers in request time. See `completion_http` on the struct.
        let http = if is_ai_path(path) { &self.completion_http } else { &self.http };
        let resp = http
            .post(format!("{}{}", self.api_url.trim_end_matches('/'), path))
            .header("Authorization", format!("Bearer {}", bearer))
            .header("X-Virtues-Purpose", self.purpose.as_str())
            .json(body)
            .send()
            .await
            .map_err(|e| anyhow!("virtues-api request failed: {e}"))?;
        let status = resp.status().as_u16();
        let body = resp.json::<Value>().await.unwrap_or_else(|_| json!({}));
        Ok(ApiResponse { status, body })
    }

    async fn send_get(&self, path: &str, bearer: &str) -> Result<ApiResponse> {
        let resp = self
            .http
            .get(format!("{}{}", self.api_url.trim_end_matches('/'), path))
            .header("Authorization", format!("Bearer {}", bearer))
            .header("X-Virtues-Purpose", self.purpose.as_str())
            .send()
            .await
            .map_err(|e| anyhow!("virtues-api request failed: {e}"))?;
        let status = resp.status().as_u16();
        let body = resp.json::<Value>().await.unwrap_or_else(|_| json!({}));
        Ok(ApiResponse { status, body })
    }
}

#[cfg(test)]
mod byo_fork_tests {
    use super::*;

    #[test]
    fn only_inference_routes_fork_to_byo() {
        assert!(is_ai_path("/v1/ai/chat/completions"));
        // Fixed-cost vendor proxies are per-user bills a provider key cannot
        // pay; they must keep going through the wallet.
        for path in ["/v1/places/autocomplete", "/v1/parallel/search", "/v1/unsplash/search", "/v1/usage"] {
            assert!(!is_ai_path(path), "{path} must not divert to BYO");
        }
    }

    fn completion(status: u16, body: Value) -> ApiResponse {
        ApiResponse { status, body }
    }

    #[test]
    fn a_reasoning_only_completion_reads_as_empty() {
        // The shape measured on the box: content present but blank, every
        // output token spent on reasoning.
        assert!(is_empty_completion(&completion(
            200,
            json!({"choices": [{"message": {"content": ""}}],
                   "usage": {"completion_tokens": 1009, "reasoning_tokens": 1009}}),
        )));
        // Whitespace-only is the same nothing.
        assert!(is_empty_completion(&completion(
            200,
            json!({"choices": [{"message": {"content": "  \n "}}]}),
        )));
        // No content key at all.
        assert!(is_empty_completion(&completion(
            200,
            json!({"choices": [{"message": {}}]}),
        )));
        // No choices to speak of.
        assert!(is_empty_completion(&completion(200, json!({"choices": []}))));
    }

    #[test]
    fn real_answers_and_real_errors_are_not_retried_as_empty() {
        assert!(!is_empty_completion(&completion(
            200,
            json!({"choices": [{"message": {"content": "the day began early"}}]}),
        )));
        // A non-2xx keeps its own error handling — retrying it here would
        // resend a call the caller is about to be told failed.
        assert!(!is_empty_completion(&completion(
            429,
            json!({"choices": [{"message": {"content": ""}}]}),
        )));
        // Non-chat AI responses have no `choices`; never judged empty.
        assert!(!is_empty_completion(&completion(
            200,
            json!({"data": [{"embedding": [0.1, 0.2]}]}),
        )));
    }

    fn byo(default_model: Option<&str>) -> crate::api::settings_byo::ByoCredential {
        byo_with(default_model, &[])
    }

    fn byo_with(
        default_model: Option<&str>,
        models: &[(&str, &str)],
    ) -> crate::api::settings_byo::ByoCredential {
        crate::api::settings_byo::ByoCredential {
            provider: "openrouter.ai".into(),
            api_key: "k".into(),
            endpoint_url: "https://openrouter.ai/api/v1/chat/completions".into(),
            models: models
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
            default_model: default_model.map(String::from),
        }
    }

    fn model_of(v: &Value) -> &str {
        v["model"].as_str().unwrap_or_default()
    }

    /// The whole point: our address in, theirs out. Verified against real
    /// OpenRouter spellings — it calls our `xai/…` chat model `x-ai/…`.
    #[test]
    fn our_address_is_swapped_for_theirs() {
        use virtues_registry::models::{default_model_for_slot, ModelSlot};
        let ours = default_model_for_slot(ModelSlot::Chat);
        let body = json!({"model": ours, "messages": []});
        let out = apply_byo_model(&body, &byo_with(None, &[("chat", "x-ai/grok-4.5")]));
        assert_eq!(model_of(&out), "x-ai/grok-4.5");
    }

    /// No entry for the slot means the route is assumed to use our ids — true
    /// for Vercel. Substituting anything would be a guess; a 400 naming the
    /// model is better than a wrong model answering.
    #[test]
    fn an_unmapped_slot_passes_through_untouched() {
        use virtues_registry::models::{default_model_for_slot, ModelSlot};
        let ours = default_model_for_slot(ModelSlot::Chat);
        let body = json!({"model": ours, "messages": []});
        let out = apply_byo_model(&body, &byo_with(None, &[("omni", "google/gemini-3.5-flash")]));
        assert_eq!(model_of(&out), ours);
    }

    /// A model the user pinned from the picker is not a slot default, so there
    /// is no role to look up. Their choice stands.
    #[test]
    fn a_hand_picked_model_is_never_rewritten() {
        let body = json!({"model": "some/exotic-model", "messages": []});
        let out = apply_byo_model(&body, &byo_with(None, &[("chat", "x-ai/grok-4.5")]));
        assert_eq!(model_of(&out), "some/exotic-model");
    }

    /// Nothing in the mapping knows which slot is which. Omni is re-addressed
    /// by exactly the same path as chat — the "which model suits audio"
    /// judgment lives in the UI, not here.
    #[test]
    fn every_slot_maps_by_the_same_rule_including_omni() {
        use virtues_registry::models::{default_model_for_slot, ModelSlot};
        for (slot, theirs) in [
            (ModelSlot::Omni, "google/gemini-3.5-flash"),
            (ModelSlot::Image, "google/gemini-3-pro-image"),
            (ModelSlot::Lite, "z-ai/glm-4.7"),
        ] {
            let ours = default_model_for_slot(slot);
            let body = json!({"model": ours, "messages": []});
            let out = apply_byo_model(&body, &byo_with(None, &[(slot.as_str(), theirs)]));
            assert_eq!(model_of(&out), theirs, "slot {} did not re-address", slot.as_str());
        }
    }

    /// The legacy `default_model` never clobbers a pinned model, because it
    /// cannot know which role that model was filling. Only the slot map may
    /// re-address a pinned body.
    #[test]
    fn the_legacy_default_model_never_clobbers_a_pinned_one() {
        let body = json!({"model": "google/gemini-3-flash", "messages": []});
        let out = apply_byo_model(&body, &byo(Some("x-ai/grok-4.5")));
        assert_eq!(out["model"], "google/gemini-3-flash");
    }

    #[test]
    fn the_default_model_fills_an_unpinned_body() {
        let body = json!({"messages": []});
        let out = apply_byo_model(&body, &byo(Some("x-ai/grok-4.5")));
        assert_eq!(out["model"], "x-ai/grok-4.5");
    }

    #[test]
    fn no_default_model_leaves_the_body_alone() {
        let body = json!({"messages": []});
        let out = apply_byo_model(&body, &byo(None));
        assert!(out.get("model").is_none());
    }

    /// The OpenRouter shape observed on 2026-08-05: HTTP 200, an `error`
    /// object, no `choices`. It must not read as success.
    #[test]
    fn an_error_body_at_200_becomes_a_502() {
        let resp = normalize_upstream_error(ApiResponse {
            status: 200,
            body: json!({"error": {"message": "Failed to load image from data:audio/wav;base64,…"}}),
        });
        assert_eq!(resp.status, 502);
        assert!(!resp.is_success());
        assert!(resp.body["message"].as_str().unwrap().contains("Failed to load image"));
    }

    #[test]
    fn a_real_completion_is_untouched() {
        let body = json!({"choices": [{"message": {"content": "hi"}}], "usage": {"cost": 0.01}});
        let resp = normalize_upstream_error(ApiResponse { status: 200, body: body.clone() });
        assert_eq!(resp.status, 200);
        assert_eq!(resp.body, body);
    }

    /// Some upstreams return both a completion and a non-fatal `error`. A
    /// present `choices` means we got what we asked for; don't discard it.
    #[test]
    fn choices_win_over_a_stray_error_field() {
        let resp = normalize_upstream_error(ApiResponse {
            status: 200,
            body: json!({"choices": [{"message": {"content": "hi"}}], "error": {"message": "warn"}}),
        });
        assert_eq!(resp.status, 200);
    }

    /// A genuine 4xx already carries the provider's message; normalizing would
    /// only bury it.
    #[test]
    fn real_error_statuses_pass_through_verbatim() {
        let body = json!({"error": {"message": "xai/grok-4.5 is not a valid model ID", "code": 400}});
        let resp = normalize_upstream_error(ApiResponse { status: 400, body: body.clone() });
        assert_eq!(resp.status, 400);
        assert_eq!(resp.body, body);
    }
}
