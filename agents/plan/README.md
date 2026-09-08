# Plan — what we intend

**Normative and temporary.** Designs for things being built. A plan is the one
genre here with an expiry date: **when the thing ships, delete the plan.** What
survives is a record of what was built and a page in the manual — not a
document describing an intention that has since become a fact.

That rule is why this directory exists. `docs/` accreted 63 files because
nothing ever left it; a plan with no death condition becomes indistinguishable
from a description of the system, and gets read as one.

**These never publish.** A plan read by an outsider is a promise, and most of
these are not promises.


| Doc | Status | What it's for |
|---|---|---|
| [ai-door-plan.md](ai-door-plan.md) | Planned | One AI door: tokens are resolved, never guessed. The proxy stops inventing a 4096 cap and forwards the `reasoning` object; background jobs declare a thinking mode instead of a number; the stream emits `finish` with a reason. Replaces the two 16k hotfixes. |
| [applet-authoring-plan.md](applet-authoring-plan.md) | Planned | Phase 3: chat intent → folder → check → reconcile → gate → enabled applet. The capability and param-schema contract. |
| [applets-overhaul-plan.md](applets-overhaul-plan.md) | Planned | "A user-space systemd with an AI author." Design locked 2026-07-19. Supersedes architecture.md at the concept/UX layer; the execution engine stays. |
| [attention-plan.md](attention-plan.md) | Partly built | How the wiki learns which parts of a day mattered. A record has no tense — only calendar knows what time it is *about* — so aftermath is invisible by construction. Attention's four phases, what may be linked vs noted vs asserted, and the refuted list. |
| [audio-collector-plan.md](audio-collector-plan.md) | Partly built | Porting continuous mic capture to Tauri. The hardest collector: iOS audio-session lifecycle, and the only continuous, high-volume, binary stream. |
| [backup-plan.md](backup-plan.md) | Partly built | Surviving the loss of the box. Pillars 1–5 and the volume path landed; Phase A is the gate, and both ends of the pipeline are still missing. |
| [bookmarks-plan.md](bookmarks-plan.md) | Planned | Capture, enrichment, and retrieval for `data_content_bookmark` — a table with a complete ontology descriptor and zero producers. |
| [byo-ai-plan.md](byo-ai-plan.md) | — | _Needs a line._ |
| [chat-ux-roadmap.md](chat-ux-roadmap.md) | Planned | The chat surface overhaul, in independently shippable tracks A–L, each ending in a verification gate. |
| [cross-platform-apps-plan.md](cross-platform-apps-plan.md) | Planned | Shipping the viewer on Windows, Linux, and Android. Collectors deliberately out of scope, with the seams left in place for them. |
| [display-plan.md](display-plan.md) | — | _Needs a line._ |
| [getting-started-plan.md](getting-started-plan.md) | Planned | Onboarding shrinks to the founder's letter; Home is rebuilt as a lifeline-spined page whose getting-started sections individually retire. No mode switch, no completion flag — the page sheds. |
| [linking-plan.md](linking-plan.md) | Partly built | Step 2 of 3 — the account link, in depth: why linking must precede pairing (reach rides the relay and the relay rides the account, discovered live at an office), the inline sign-in contract, and the persona branch. **Its header diagram still says pairing uses "the on-screen code";** `0x83` has been codeless since 2026-08-24. |
| [mac-presence-plan.md](mac-presence-plan.md) | Planned | `data_activity_app_usage` is close to an inversion of the truth — real focused work is invisible while artifacts are the headline numbers. |
| [mobile-ux-plan.md](mobile-ux-plan.md) | Partly built | Pragmatic mobile next steps: keep the tab strip on phone, block split view, keyboard work gets a spike first. Edge-to-edge safe-area theming already shipped. |
| [narrative-resolution-plan.md](narrative-resolution-plan.md) | — | _Needs a line._ |
| [notebooks-plan.md](notebooks-plan.md) | Partly built | Notebook as a workspace lens over the life-graph: a Library of materials, scoped grounded chat, source-level citations. |
| [onboarding-plan.md](onboarding-plan.md) | Partly built | The build order from what existed to the paradigm — the paradigm says what and why, this says in what order. **Its "Where we are" section is stale:** `0x82`/`0x83` are listed as "built, never observed" and were verified end-to-end on 2026-08-24 (see one-wire-plan.md), and the "three-step display" is now seven states. |
| [open-relay-plan.md](open-relay-plan.md) | Planned | The relay drops admission for a flat cap — reachability stops being a tier. The live relay's `/relay/authorize` callout is the privacy linkage to delete (the SNI/HMAC control plane turned out to be legacy prose); checkout leaves the airlock, pairing collapses to one token doctrine, clients go iroh-only. |
| [plaid-plan.md](plaid-plan.md) | Partly built | Plaid as the *general* finance collector — FinanceKit reaches only Apple's own products, so Plaid is the path that has to work for everyone else. |
| [publishing-plan.md](publishing-plan.md) | Planned | One frozen artifact, three surfaces (screen, print, publish). Publications replace page shares; a face is already the right container because its constraints ARE print's and static publishing's. v1 builds no ingress — the box keeps its zero internet-facing surface. |
| [reach-reliability-plan.md](reach-reliability-plan.md) | Planned | "100% reachable whenever the box is up." Root cause is upstream iroh #4289; the fix is `network_change()` on `NWPathMonitor` plus a watchdog rebuild. |
| [references.md](references.md) | Planned | One primitive for `@`, peek, and open, so a person / place / file / page stops being rendered by a different code path each time. |
| [researcher-plan.md](researcher-plan.md) | Planned | The researcher/PhD archetype, built complete in v1 — corpus, annotation, scholarly metadata, synthesis bridge. Extends notebooks-plan Phase D. |
| [schema-cleanup-checklist.md](schema-cleanup-checklist.md) | In progress | The do-list for the 2026-08-28 schema audit, ordered fix → guard → rm → decide → later; evidence lives in [the audit record](../record/schema-audit-2026-08-28.md). FIX and GUARD are checked off; delete this when the RM/DECIDE tiers land. |
| [sources-packages-plan.md](sources-packages-plan.md) | — | _Needs a line._ |
| [spa-delivery-plan.md](spa-delivery-plan.md) | Ready to schedule | One UI-delivery architecture for phone and Mac: baked bundle, OTA overlay pulled from the box, local data. Unlocks the offline editing already built into Yjs, and makes UI-ahead-of-box skew impossible. |
| [ui-overhaul-plan.md](ui-overhaul-plan.md) | Planned | Nineteen layout/settings/design items triaged against the codebase, each marked ship / spike / defer / drop. |
| [version-compat-plan.md](version-compat-plan.md) | Planned | When the app outruns the server. Phones update themselves, servers update when someone types a command, so the app is always ahead — and v0.1.6 making `model` optional killed chat on every older server. Both version coordinates and the prerelease ordering already exist; what is missing is a declared minimum, a comparison, and a CI ratchet. |
| [update-manifold-plan.md](update-manifold-plan.md) | Planned | Generalizes the box paradigm to the whole fleet, and adds the cross-component version negotiation the box doc never needed. |
| [wiki-plan.md](wiki-plan.md) | — | _Needs a line._ |
