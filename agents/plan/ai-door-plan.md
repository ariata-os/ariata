# One AI door: the plan for the model and API layer

> Status: **Planned, v2.** Written 2026-09-08 after the interview drafter and
> the day summary failed the same way four days apart; revised the same day
> after review (the review register is at the bottom). The two 16k hotfixes
> on `wave` are the interim; this plan is what replaces them. Delete this file
> when Phase 3 ships.

## The story, in one paragraph

On 2026-08-27 the Chat slot moved from a model that answers to a model that
thinks first. Nothing else changed. Every token cap in the tree had been set by
hand for the old model, and Anthropic counts thinking inside that cap, so two
background writers began spending their whole budget thinking and returning
nothing. Day summary noticed first because it runs hourly and bills. The
interview drafter noticed second, through a beta tester, because it runs once
per box. Both were "fixed" by raising a number. The number was never the bug.
The bug is that **the model is chosen in one place and the cap is guessed in
twelve others**, and nothing tells a caller when its guess stopped being true.
This plan gives the cap an owner, makes every request say what it wants
instead of how many tokens it may spend, and makes every stream say how it
ended.

## Goal

Three sentences that, once true, close the class:

1. **A caller declares intent, never a number.** Background jobs ask for a
   thinking mode (off, low, high). Live chat asks for nothing. Token caps are
   derived from facts the gateway owns about the model.
2. **Nothing crosses a boundary silently.** A field the box sends either
   reaches the model or fails to compile. One request type on both sides;
   the proxy forwards and merges, and logs anything it does not know. It
   never drops quietly.
3. **Every stream says how it ended.** `finish` carries the reason, tool
   failures arrive as tool errors, and a truncated turn is visible in the UI
   instead of indistinguishable from a complete one.

**Non-goals.** Changing the slot doctrine (the registry stores five strings and
the gateway owns model facts; that stays). Moving the agent loop client-side.
Replacing the raw `reqwest` proxy with an SDK. A new billing model.

## What is true today

Read against the code on 2026-09-08. Each line is a fact this plan changes or
depends on.

| Where | What | Why it matters |
|---|---|---|
| `services/virtues-api/src/routes/streaming.rs:107`, `ai.rs:146` | Proxy fills `max_tokens: 4096` and `temperature: 0.7` when the caller omits them | Live chat sends no cap on purpose; the proxy puts one back, on a thinking model with tools |
| `streaming.rs:64-81`, `ai.rs:69-86` | Typed request structs, hand-mirrored from the box's body builders. Anything not named is dropped by serde | `providerOptions` and `thought_signature` from the box die here; the box's Claude-3 thinking budget in `agent/stream.rs:338` has been dead since the proxy was typed on 2026-06-08 |
| `agent/stream.rs:80-101` | Live chat sends no `temperature` | Today it runs at the proxy's 0.7 default. Remove the default and chat silently moves to the provider's 1.0 |
| `virtues_api/client.rs` `Route::Byo` | The completion helper serves BYO endpoints as well as the gateway | A gateway extension field sent to an Ollama or OpenAI endpoint is rejected or ignored |
| `chat.rs` (fixed 2026-09-08) | Ghost chats were persisted in full: chat row, both messages, usage | The client promised "never persisted" and sent a flag the box had no field for. Now honored; the ghost transcript rides in on the request every turn |
| `ai.rs:158`, `streaming.rs:118` | Proxy writes its own `providerOptions.gateway.zeroDataRetention` | Correct, and it must survive any merge |
| `virtues_api/completion.rs:65` | One helper for background calls, takes `max_tokens: u32` and `reasoning_effort: Option<&str>` | The right chokepoint, wrong parameters |
| `completion.rs:104` | Empty content is the failure signal; `finish_reason` is never read | "Ran out of room" and "said nothing" are the same error |
| `agent/mod.rs:220` | Live chat passes `None` for the cap | Right. The proxy undoes it |
| `api/chat.rs:1692` | `StepComplete` and `Done` agent events are dropped; no `finish`, `start-step`, `finish-step`, or `tool-output-error` is ever emitted | The client cannot know a turn was cut short |
| `ChatView.svelte:2059-2090`, `ThinkingBlock.svelte:244` | UI branches on `output-error` and on a state called `pending` | Neither is ever sent. Dead code that looks like error handling |
| `chatInstances.svelte.ts:196-236` | Transport sends the full `messages` array | `chat.rs:1164` reads only the last user message and rebuilds history from its store |
| `chatInstances.svelte.ts:305-322` | Overrides the SDK's private `state.replaceMessage` | Why `ai` is pinned to the patch version |
| `catalog.rs:59-98` | Gateway parses `tags` and `supported_parameters` but not `reasoning_options`; `CuratedModel` forwards no reasoning facts | The gateway knows which models think and cannot say so to a box |
| `registry/models.rs:344` | Test forbids model facts in the registry crate | Correct. The cap logic therefore lives in the catalog and the helper, not the registry |
| Gateway docs, 2026-09-08 | Chat completions accept a `reasoning` object: `enabled`, `effort`, `max_tokens`, `exclude`. Sonnet 5 honors `enabled: false`. Fable 5 cannot turn thinking off. Anthropic reports no reasoning-token breakdown | The lever exists; the proxy does not forward it; the usage column cannot verify it |
| Gateway docs, 2026-09-08 | Claude 5 models omit thinking text unless `providerOptions.anthropic.thinking.display` is `summarized`; Gemini needs `includeThoughts`. Every assistant message carries a normalized `reasoning_details` array meant to be echoed back so the model can resume thought after tool results | The box's reasoning events fire on an empty stream today, and the box echoes nothing back |

## The invariant

> **Tokens are resolved, never guessed. A request names a thinking mode; the
> helper turns the mode and the model's catalog entry into a request; the
> proxy forwards it whole; the response says how it ended.**

Every phase below serves that sentence, and the acceptance gates test it
adversarially: a caller that passes a number, a proxy that invents one, a
stream that ends without saying why.

## Phase 1. The proxy stops inventing numbers (`services/virtues-api`)

Smallest change, largest blast radius, deploys independently of any box
release. Do it first, deploy it first, and check the ECR push date before the
next box release (the cloud lagged v0.1.6 by four days and served 404s; this
must lead, not lag).

**1a. Forward what is given, omit what is not.** Remove `unwrap_or(4096)` and
`unwrap_or(0.7)` on both paths. A request with no `max_tokens` reaches the
gateway with no `max_tokens`, and the provider's own default applies, which
for every model in the slot map is its full output window.

Temperature is the one place this changes behavior: live chat sends none and
has been running at the proxy's 0.7. **Decision: the box sends 0.7 explicitly
on chat turns, in the same commit as Phase 2d, and the proxy defaults
nothing.** A default that lives in the proxy is a number nobody in the box can
see, which is the disease this plan treats. Phase 1a therefore ships in the
gateway only after the box carrying 2d is released, or it keeps the
temperature default until then and drops only the token cap. Do the latter:
two deploys, no behavior change in between.

**1b. Forward the `reasoning` object.** Add to both request structs:

```rust
/// Gateway extension. `enabled`, `effort`, `max_tokens`, `exclude`.
/// `reasoning_effort` stays as the alias the docs promise.
#[serde(default)]
reasoning: Option<serde_json::Value>,
```

Copy it verbatim when present. Keep `reasoning_effort`. The gateway resolves
precedence.

**1c. Merge `providerOptions`, never replace.** Accept `provider_options` from
the caller. Deep-merge the proxy's `gateway.zeroDataRetention` into it. ZDR
always wins: a caller cannot set `zeroDataRetention: false` on a model the
catalog says to enforce. Add a test that proves it.

**1d. One request type, both crates.** The proxy's two request structs are
hand-mirrored from the box's two body builders, and they drifted: the box sends
`provider_options` and `thought_signature` and the proxy has never had either.
The first draft of this plan proposed `deny_unknown_fields` on the proxy, which
would have 400ed every box in the field on deploy, because they all send those
two fields today. The proxy serves many box versions and must stay tolerant;
strictness belongs at the sender.

So: a small `virtues-ai-wire` crate in the workspace holding one
`ChatCompletionRequest` type. The box builds it (`agent/stream.rs`,
`virtues_api/completion.rs`, `ai_complete.rs`, the raw sites in Phase 2c), the
proxy deserializes it, and a field either exists on both sides or does not
compile. The proxy keeps `#[serde(default)]` everywhere and logs unknown fields
at `warn` with a counter, never a 400. That is "nothing crosses silently" made
mechanical, at compile time, with no compatibility cliff. The two proxy body
builders (`streaming.rs:105` and `ai.rs:141`) collapse into one function that
takes the wire type.

**1e. The catalog learns who thinks.** Parse `reasoning_options` from
`GET /v1/models` into `GatewayModel`, and derive on `CuratedModel`:

```rust
pub struct ReasoningFacts {
    /// The model reasons at all (`tags` contains `reasoning`, or any option).
    pub thinks: bool,
    /// `reasoning.enabled: false` is honored. Fable 5: false.
    pub can_disable: bool,
    /// Allowed effort values, e.g. ["low","medium","high","xhigh"]. Empty = no lever.
    pub effort_values: Vec<String>,
}
```

`can_disable` comes from a `toggle` entry in `reasoning_options`. When the
catalog says nothing, `thinks` is false and `can_disable` is false, and the
helper treats the model as one that thinks anyway (the safe reading, since
missing controls mean unspecified, not absent). The box's `CatalogModel`
gains the same field.

**Gate.** Unit tests on the proxy: a body without `max_tokens` produces an
upstream body without `max_tokens`; `reasoning` round-trips; a caller's
`providerOptions.anthropic` survives beside the proxy's `gateway`; an unknown
field is a 400; `zeroDataRetention: false` from a caller is overwritten. Deploy
to EC2, confirm `/v1/ai/models` returns `reasoning` on `anthropic/claude-sonnet-5`
with `can_disable: true`.

**1f. Where the cost bound lives now.** The 4096 default was, by accident, the
only per-request spend ceiling. Removing it means a 20-step tool loop can spend
the model's full output window per step. The bounds that remain are the step
limit (`AgentConfig::max_steps`, 20), the wallet gate (pre-flight, balance
only), and the model's own window. **Decision: the proxy applies the catalog's
`max_tokens` for the model as an explicit ceiling when the caller sends none.**
That is the same number the provider would use, so it changes nothing today,
but it is a fact the proxy owns rather than a guess, and a per-request ceiling
a future budget rule can lower. Record it in the same body builder as ZDR.

## Phase 2. The box asks for thinking, not tokens (`virtues-core`)

Depends on Phase 1 being deployed. Ships in the next box release after it.

**2a. One enum replaces every literal.**

```rust
/// What a background job wants from the model's reasoning. Never a token count.
pub enum Thinking {
    /// Arrangement, extraction, titles, summaries. Sends `reasoning.enabled: false`
    /// when the model can honor it; otherwise the lowest effort it exposes.
    Off,
    /// Adjudication: the day detective. Sends `reasoning.effort: low`.
    Low,
    /// Leave the model's default. The only choice live chat makes.
    Default,
    /// Reserved. No caller today.
    High,
}
```

**2b. The helper resolves the cap.** `system_completion` drops `max_tokens`
and `reasoning_effort` and takes `Thinking` plus an optional
`output_ceiling: Option<u32>`. The `reasoning` object is a gateway extension,
and the helper also serves the BYO route, which never touches the gateway. On
`Route::Byo` the helper sends at most `reasoning_effort`, never the object,
and applies no ceiling logic it cannot verify (a BYO endpoint has no catalog
entry, so `Thinking::Off` degrades to "send nothing"). The ceiling is for jobs whose output size IS the
feature (a chat title is fifty tokens because a title is short). The helper
applies it only when thinking is `Off` and the model's catalog entry says
`can_disable`; otherwise the ceiling would be spent on thinking, so the helper
ignores it and logs once. With no ceiling, the helper sends no `max_tokens`
at all and the model's window applies. The gateway `max_tokens` from the
catalog is a fact for display; it is not resent as a request cap.

**2c. Every caller migrates.** The literal disappears from each site.

| Caller | Thinking | Ceiling | Note |
|---|---|---|---|
| `day_summary` segmentation | Low | none | The 16k literal goes |
| `day_summary` narration | Low | none | |
| `narrative_draft` document | Off | none | Arrangement, not composition |
| `narrative_draft` chapters | Off | none | Strict JSON extraction |
| `entity_article_gen` | Off | none | The 900 literal goes |
| `compaction` | Off | none | Currently a raw body at 1000 on the Lite pin; moves onto the helper |
| `chats` title | Off | 50 | Ceiling honored only when thinking is off |
| `bookmark_enrichment` | Off | none | Moves onto the helper |
| `image_gen` | n/a | none | Image slot; raw body stays, cap literal goes |
| `ai_complete` | Off | 512 | Inline editing; the ceiling is the feature |

The Lite slot honors the owner's background pin, so a thinking model pinned
there used to put compaction and titles in the same hole. After 2b that cannot
happen: the helper reads the pinned model's catalog entry, not the slot's.

**2d. The live chat turn.** Keep `None` for the cap (the proxy now respects
it, and 1f applies the model's own window). Send `temperature: 0.7`
explicitly (see 1a). Delete `build_provider_options` in `agent/stream.rs`
(the Claude-3 substring test and the legacy `budget_tokens` shape, which now
400s on Claude 4.7 and later).

**Ask for the thinking text.** Decided 2026-09-08: chat shows thinking in the
backend now, whether or not the UI renders it yet. The box already emits
reasoning start, delta, and end, and persists a reasoning column; the stream
is empty because Claude 5 omits the text unless asked. The catalog's
`ReasoningFacts` (1e) gains a `display_options: serde_json::Value` the proxy
derives from `owned_by`: `{"anthropic": {"thinking": {"type": "adaptive",
"display": "summarized"}}}` for Anthropic, `{"google": {"thinkingConfig":
{"includeThoughts": true}}}` for Google, empty otherwise. The box attaches it
as `provider_options` on every chat turn. Claude returns a summary, not the
full thinking, and bills the full thinking either way.

**Echo `reasoning_details`, then delete the thought signature.** The box's
`thought_signature` plumbing landed on 2026-06-08 for Gemini 3 and has never
reached a model: the proxy never had the field. Google still requires thought
blocks to be resent as received, and Anthropic requires its thinking-block
signatures back during tool use once thinking is on, which 2d turns on. The
gateway's answer for every provider is the `reasoning_details` array on each
assistant message, to be echoed verbatim on that message when it is resent.
The box stores it on the assistant row (a jsonb column, migration claimed
with `make migration`) and puts it back in `api_messages` for that turn's
loop and for later turns. Spike first, one day: Sonnet 5 with thinking display
on and tools across three steps; one Gemini model with tools across three
steps; assert no 400 and coherent continuation. Then delete
`thought_signature` from the request, the row, the wire type, the data part,
and the transport hook.

**2e. Read the ending.** The helper reads `choices[0].finish_reason`. `length`
becomes `Error::ExternalApi("<feature>: the model ran out of room (finish_reason=length)")`,
distinct from empty content. The `app_ai_calls` row records `finish_reason`.
Fix the comment in `day_summary` that says to read `reasoning_tokens` before
tightening: for Anthropic the gateway reports none, so the column reads zero
whether or not thinking happened. The honest signal is `finish_reason` plus
total completion tokens.

**Gate.** `grep -rn '"max_tokens"' virtues-core/src` returns only the helper
and the image path. Unit tests: `Thinking::Off` on a model with `can_disable`
sends `reasoning.enabled=false` and honors a ceiling; on a model without it,
sends the lowest effort and drops the ceiling with a log line;
`finish_reason=length` is a distinct error. Run the day detective and the
interview drafter against the live gateway once each and read the recorded
finish reasons.

## Phase 3. The stream says how it ended (`chat.rs`, `apps/web`)

Independent of Phases 1 and 2. Can ship in the same release as Phase 2.

**3a. Emit the whole protocol.** Around the existing events:

- `start` with the message id once per response.
- `start-step` and `finish-step` around each agent-loop iteration. The docs
  call `finish-step` necessary for multi-call sequences; today a tool call
  followed by text is one undifferentiated run of parts.
- `tool-output-error` with `errorText` when a tool fails. Today failures ride
  inside `tool-output-available` or become a top-level `error`, and the UI's
  error branches never run.
- `finish` with `finishReason` from the last step: `stop`, `length`,
  `tool-calls`, `error`, `other`. `AgentEvent::Done` already exists and is
  dropped; carry the reason through it.
- `abort` on cancellation, after `stop()` and `cancelChat`.

**3b. The UI reads it.** Delete the `pending` state check. Keep the
`output-error` branches, which now fire. Show a one-line "cut short" notice
under a message whose finish reason was `length`, with the regenerate action.
Persist the finish reason with the message so a reload shows the same notice:
a `finish_reason text` column on `app_chat_messages`, claimed with
`make migration` and shipped in the same migration as 2d's
`reasoning_details` column, so the two land as one number.

**3c. Protocol conformance test.** Feed a recorded SSE stream from the Rust
side into the SDK's own parser in a vitest, and assert it produces one
assistant message with the expected parts and a finish reason. This is the
gate that keeps the box honest across SDK upgrades, and it is cheap: one
fixture per shape (text only, tool then text, tool error, cancelled).

**Gate.** The vitest above passes. A deliberately tiny cap on a dev box
produces a visible "cut short" notice and a `finish_reason=length` row.

## Phase 4. Send the last message only (`apps/web`, `chat.rs`)

Small. Ships whenever.

`prepareSendMessagesRequest` sends `messages: [last]` for the
`submit-user-message` trigger and `messages: []` for
`regenerate-assistant-message`, plus the `trigger` field. The box already
reads only the last user message. Make the regenerate path explicit on the box
(today it is inferred), and delete the thought-signature fallback at
`chat.rs:1523` with 2d.

**Ghost chats are the exception, by design.** A temporary chat has no rows on
the box, so the wire is its only transcript: the transport keeps sending the
full `messages` array when `temporary` is set, and the box builds the model
context from it (`ghost_history`, shipped 2026-09-08). Everything else sends
the last message only. The `temporary` question is closed: it is honored.

**Gate.** Request body size on a long chat is constant. Regenerate works with
an empty messages array.

## Phase 5. What waits for a planned SDK bump

Not scheduled. Listed so the shape is known.

- **Native approval.** Replace the `permission_needed` output convention and
  the `regenerate()` hack with `tool-approval-request`, the
  `approval-requested` state, and `addToolApprovalResponse`. The box emits the
  request part instead of a special output, and the client answers it.
- **The reactivity patch.** Try the SDK's default reconciliation on a long
  streamed message. If it is acceptable, delete the `replaceMessage` override
  and unpin the patch version. If not, file it upstream with the benchmark and
  keep the pin, documented in the store.
- **Attribution.** Forward the feature name as a gateway reporting tag so
  `/v1/report` groups spend by feature without the ledger doing arithmetic.
  Verify the exact field in the custom-reporting docs first.

## Verify before starting

Two claims in this plan were not checked against code and must be, by whoever
picks up the phase:

- **3a** assumes the box's stream parser already reads `finish_reason` from
  the final chunk. If it does not, that is the first change in 3a.
- **3c** names "the SDK's own parser" as the vitest gate without confirming
  the export. Find the UI-message-stream reader the `ai` package exports at
  7.0.16 before writing the test.

## Order, and why

1. Phase 1, deployed alone. It is the only change that fixes live chat on
   every box already in the field, because the cap it removes is applied in
   the cloud.
2. Phase 2 and 3 together in the next box release. Phase 2 needs Phase 1
   deployed. Phase 3 needs nothing but is the same files.
3. Phase 4 with 2 and 3 or after. It is small enough to ride along.
4. Phase 5 when the SDK is bumped for another reason.

The order has one constraint the first draft missed: 1a's temperature change
and 2d's explicit temperature must not be split across a release boundary,
or chat runs hot for everyone in between. See 1a.

The two 16k hotfixes already on `wave` ship in the release before Phase 2 and
are deleted by it. They are correct for the interim: a bigger cap on a thinking
model fails less often, and a cap that never fails is what Phase 2 provides.

## What "correct" means at the end

- A `grep` for `max_tokens` in `virtues-core/src` finds the helper and nothing
  that hands it a number except the two output ceilings.
- Changing the Chat slot to any model in the catalog cannot make a background
  job return nothing. The helper reads the new model's facts.
- Changing the Lite pin to a thinking model cannot break titles or compaction.
- The proxy body sent upstream is the box's body plus ZDR, and a field the
  proxy does not know is a 400, not a silence.
- A turn that hits any limit shows it in the UI and records it in the table.
- The interview drafter and the day detective run with thinking off and low,
  respectively, and their calls carry `finish_reason=stop`.

## Refuted, so nobody re-argues it

- **"Raise the cap to 16k everywhere."** That is the interim, not the fix. It
  pays for thinking the job does not want, and it fails again on the next
  model whose default budget is larger.
- **"Put `max_output_tokens` in the registry crate."** The crate's own test
  forbids model facts, for the right reason: they are gateway facts and go
  stale. The cap is derived at call time from the catalog.
- **"Use the Lite slot for extraction."** The Lite slot honors the owner's
  background pin, which may be a BYO endpoint with no retention promise. The
  interview promised zero retention in as many words. Extraction on that
  transcript stays on the curated Chat slot with thinking off.
- **"Read `reasoning_tokens` to size the cap."** Anthropic reports none
  through the gateway. The column is zero either way.
- **"Reject unknown fields at the proxy."** The proxy serves every released
  box at once. A 400 on a field old boxes send is an outage, not a contract.
  The contract is a shared type; the proxy logs and tolerates.
- **"Ghost mode works because the append fails."** It did not fail. The
  handler created the chat row first, so every ghost message persisted.
  Fixed 2026-09-08; the box now branches on the flag it never read.

## Review register (2026-09-08)

Raised against v1 and folded in: the 400-on-unknown-fields cliff (now 1d, a
shared wire crate); the hidden temperature change (1a, decided: box sends it);
the `reasoning` object on the BYO route (2b); the vanished cost bound (1f);
the finish-reason migration (3b); thinking text for chat (2d, decided: on);
the thought-signature question (2d, `reasoning_details` echo then delete);
ghost chats versus last-message-only (Phase 4, exempt); two unverified claims
(the section above). Ghost persistence was fixed the same day rather than
planned.

