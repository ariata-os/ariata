<!--
  ┌─ HEADING CONVENTION (read before editing any # / ## / ### heading) ──────────┐
  Headings are NOT plain markdown text. GitHub's sanitizer strips CSS, so to get
  a serif (Times) heading we render each one as a theme-aware SVG image wrapped in
  a real markdown heading, e.g.:

    ## <picture><source media="(prefers-color-scheme: dark)" srcset=".github/images/headings/h2-foo-dark.svg"><img alt="Foo" src=".github/images/headings/h2-foo-light.svg" height="28"></picture>

  Because the heading's visible content is an image (no text), two things follow:
    • The GitHub "Outline" sidebar shows blank labels — accepted tradeoff, which
      is why this file carries a hand-written table of contents under the badges.
    • GitHub can't auto-generate anchors, so each H2/H3 has an explicit
      `<a id="…"></a>` on the line ABOVE it. The TOC links to those ids; some
      headings carry a second, older id so existing deep links keep resolving.

  TO ADD OR EDIT A HEADING (do ALL of these, or you'll get a stale image / dead link):
    1. Edit the heading list in .github/images/headings/gen.py
       (font sizes: H1=34/h40, H2=24/h28, H3=20/h22; weight 400; light+dark each).
    2. Run:  python3 .github/images/headings/gen.py   (regenerates the SVGs).
    3. In this file, update the <picture> block's `alt`, both `srcset`/`src`
       filenames, and the `<a id>` anchor above it. Note gen.py's filename slug
       turns an apostrophe into a hyphen ("After it's running" →
       h3-after-it-s-running) while the anchor drops it ("after-its-running").
    4. Delete the SVG pair for any heading you removed.

  Keep headings short — wide SVGs (>~360px) get scaled down on mobile, breaking
  the visual size hierarchy. Times is a system font; Linux/Android fall back to
  their default serif.

  SCREENSHOTS: all four are placed. Alt text describes WHAT IS IN FRAME, never
  the word "screenshot". Capture against an anonymized copy —
  tools/anonymize-boxcopy.py — and read every frame before it is committed:
  four rounds of capture found four more tables holding real content.
  └──────────────────────────────────────────────────────────────────────────────┘
-->

![Virtues](.github/images/cover3.png)

# <picture><source media="(prefers-color-scheme: dark)" srcset=".github/images/headings/h1-virtues-dark.svg"><img alt="Virtues" src=".github/images/headings/h1-virtues-light.svg" height="40"></picture>

Self-hosted personal AI over the record of your own life.

[![CI](https://github.com/virtues-os/virtues/actions/workflows/ci.yml/badge.svg)](https://github.com/virtues-os/virtues/actions/workflows/ci.yml)
[![License: BUSL-1.1 + MIT](https://img.shields.io/badge/License-BUSL--1.1%20%2B%20MIT-blue.svg)](LICENSE)
[![Discord](https://img.shields.io/badge/Discord-Join%20Us-7289da?logo=discord&logoColor=white)](https://discord.gg/sSQKzDWqgv)

[Website](https://virtues.com) · [Docs](https://virtues.com/docs) · [Pre-order the server](https://virtues.com/pre-order) · [Discord](https://discord.gg/sSQKzDWqgv)

Virtues is a server you run at home. It pulls in what you already generate —
messages, calendar, health, location, transactions, recordings, files — keeps
it in one Postgres database on your own disk, and turns it into three things: a
written account of each day, a wiki of the people and places in it, and an
assistant that answers from your actual data. The two retrieval models run on
your hardware. The model that writes is the only thing that leaves, through a
metered proxy that keeps nothing. No inbound port, no hostname: your devices
reach the server by key.

*Early software. The [releases page](https://github.com/virtues-os/virtues/releases)
says what exists; expect rough edges, and expect us to say where they are.*

<img src=".github/images/shots/day-page.png" alt="The day page for Wednesday 2 September, open beside a sidebar of notebooks: a first-person account of the day — a call that ran past two, a slow morning, a short errand, six hours at the desk — with the people and organizations in it rendered as links into their own pages." width="100%">

<a id="diy-quickstart"></a>
## <picture><source media="(prefers-color-scheme: dark)" srcset=".github/images/headings/h2-diy-quickstart-dark.svg"><img alt="DIY quickstart" src=".github/images/headings/h2-diy-quickstart-light.svg" height="28"></picture>

Two ways to run it. **Do it yourself**: the software on your own Linux machine,
installed with one command — the path this README covers.
**[The Virtues Server](https://virtues.com/pre-order)**: the same software on a
board we build, open for pre-order. Either way you can leave with your data.

Five steps on a spare Linux machine — a VM is fine.

**1. Install.** Choose **Quick trial** when it asks about inference: a
CPU-only model server and two small models, no configuration.

```bash
curl -sSL https://virtues.com/sh | sudo sh
```

**2. Pair a device.** The installer ends by printing a 6-digit code — a
browser cannot pair, so enter it in the desktop or phone app from
[virtues.com/downloads](https://virtues.com/downloads). Pairing puts your
device's key on the server's allowlist; that key is how you reach it from
anywhere, and a fresh code is one command away:

```bash
sudo virtues pair
```

**3. Connect an account.** The app asks for one after pairing. Search runs
locally, but everything that reaches outside goes through the account: the
model that writes your days and answers questions, web search, the sign-in
flows for Google, Strava and the rest, bank data, and the relay that lets your
devices reach the server from outside your home network. Bringing your own
model key replaces only the first of those. From the server itself, the same
step is:

```bash
sudo virtues subscribe
```

**4. Connect a source** in the app: Google, Plaid, Strava, or the phone app
for HealthKit and location. The record starts filling from there.

**5. Wait a day.** Each day is written after it ends, so the first page worth
reading arrives tomorrow. `virtues status` shows what is flowing in the
meantime; `virtues doctor` explains anything that isn't.

The trial is slow by design. For real use, run your own embedding and rerank
endpoints — [the full install](#the-full-install) has the commands and models.

<a id="what-it-does"></a>
## <picture><source media="(prefers-color-scheme: dark)" srcset=".github/images/headings/h2-what-it-does-dark.svg"><img alt="What it does" src=".github/images/headings/h2-what-it-does-light.svg" height="28"></picture>

**It writes your days.** After a day ends, the server rebuilds it from evidence
— visits, movement, device presence, audio, messages, purchases, sleep — into a
timeline, then writes the diary. Plans are not evidence: a calendar entry with
no physical trace behind it becomes an honest *Unknown*, not a confident
memory. Each event is scored for novelty against your own history, never a
population norm. [Design notes](agents/record/the-day.md).

**It keeps a wiki of the people and places in them.** The Sarah in your
calendar, contacts and messages resolves to one person with a page of her own.

<img src=".github/images/shots/wiki.png" alt="The wiki index: a record of 592 people, 46 places and 204 organizations with entries since December 2017, above a sparkline of the whole record and an activity grid showing which days are dense and which are sparse." width="100%">

**It answers from your data.** The agent has read-only SQL over your tables,
semantic search over the record, a Python sandbox, web research, and page
editing. Claude, GPT or Gemini; three modes — full tools, conversation only,
read-only research.

<img src=".github/images/shots/chat.png" alt="A chat: the question — how much of last week does the record actually account for — with the agent's sql_query step against the events table above the answer: 92.4 hours placed as stays, 16.5 in transit, and 59.1 it will not claim to know, because those hours have no visit, movement, audio or purchase to corroborate them." width="100%">

**Where the data goes.** It stays on your server. Embedding and reranking must
run on a local address; the installer refuses a public one. GPS is reduced to
distance and pace before anything is sent. The assistant is the exception and
the point: a question, or the nightly day-writing, sends the relevant part of
your record to a model provider — voice as audio, not as transcript. Point it
at a local model and that stops too. The relay cannot read your traffic; that
is physics. The inference boundary is a contract with a provider, which is a
different kind of promise.
[The privacy model](agents/record/privacy-model.md#the-inference-boundary-where-your-data-does-leave)
says exactly what crosses and what never does.

<a id="why-its-shaped-this-way"></a>
## <picture><source media="(prefers-color-scheme: dark)" srcset=".github/images/headings/h2-why-it-s-shaped-this-way-dark.svg"><img alt="Why it's shaped this way" src=".github/images/headings/h2-why-it-s-shaped-this-way-light.svg" height="28"></picture>

**Virtues of the digital age.** The name does not mean *use technology to live
virtuously*. Virtues change with the age, and ours asks for ones the old lists
never named. Owning the data of your own life is the first: what is held about
you is what can be used to move you, and someone who cannot read their own
record cannot notice being moved.

**Digital subsidiarity** is that virtue as software. A matter belongs to the
smallest authority competent to handle it, so the record of a life belongs
where the life is lived — not in a datacenter that can read it, not with a
company that will outlive its interest in keeping it.

Every structural decision follows, and each is an incapability rather than a
promise: the record is on your disk; retrieval runs against local endpoints
only; nothing inbound is opened at home; the relay moves sealed bytes it has no
key to read; we are not on your allowlist.

Essays in [the Library](https://virtues.com/library). Engineering record in
[`agents/`](agents/).

<a id="the-ontology"></a>
## <picture><source media="(prefers-color-scheme: dark)" srcset=".github/images/headings/h2-the-ontology-dark.svg"><img alt="The ontology" src=".github/images/headings/h2-the-ontology-light.svg" height="28"></picture>

The schema is not an implementation detail here — it *is* the interface. It is
shown to a language model at runtime and it drives a table-driven UI, so a
column name is a term of art in two directions at once.

Everything ingested lands in one of a couple of dozen **registered
ontologies** — the list is
[`crates/virtues-registry/src/ontologies.rs`](crates/virtues-registry/src/ontologies.rs),
and it is what a life looks like once it is normalized:

| Family | Tables |
|---|---|
| Health | `data_health_sleep`, `data_health_workout`, `data_health_heart_rate`, `data_health_hrv`, `data_health_steps` |
| Place | `data_location_visit`, `data_location_point`, `data_environment_weather` |
| People | `data_communication_message`, `data_communication_email`, `data_communication_transcription`, `data_calendar_event` |
| Money | `data_financial_transaction`, `data_financial_account` |
| Attention | `data_activity_app_session`, `data_activity_web_browsing`, `data_content_document`, `data_content_bookmark`, `data_content_conversation` |
| Voice | `data_audio_session` |

Table prefixes are a namespace, not decoration: `data_` is ingested, `wiki_` is
the derived entity graph (`wiki_people`, `wiki_places`, `wiki_orgs`,
`wiki_events`, `wiki_days`), `narrative_` is the life structure
(`narrative_telos` → `narrative_acts` → `narrative_chapters` → days), `app_` is
product state, `search_` is the indexes, `elt_` is pipeline plumbing.

The column names follow a law, because the schema once had **seven** different
names for "when this happened":

- `occurred_at` — an instant, when the thing happened. `started_at` / `ended_at`
for a span.
- `created_at` / `updated_at` — when *we* wrote the row. Never the event; that
conflation is what produced `created_time` sitting beside `created_at`.
- `is_` / `has_` for booleans, never a bare adjective.
- A unit suffix on every quantity: `_cents`, `_ms`, `_bytes`, `_meters`.

The agent's `sql_query` tool reads a catalog of this schema and queries it
read-only, which is the concrete mechanism behind "an AI with real context".
Adding a data type means adding an ontology and a text extractor; the UI and
the search index follow from the descriptor.

<a id="architecture"></a>
## <picture><source media="(prefers-color-scheme: dark)" srcset=".github/images/headings/h2-architecture-dark.svg"><img alt="Architecture" src=".github/images/headings/h2-architecture-light.svg" height="28"></picture>

<!-- Replace with a light/dark SVG pair (same machinery as the headings). -->

<picture>
  <source media="(prefers-color-scheme: dark)" srcset=".github/images/architecture/architecture-dark.svg">
  <img alt="Sources flow into Virtues Core, one Rust binary on port 8000 holding Postgres and a file store and never an external API key. Core calls your own inference endpoints — /v1/embeddings on 18181 and /v1/rerank on 18182, loopback, LAN or VPN only — and reaches model providers solely through virtues-api on port 9002, which holds the external keys and enforces a budget." src=".github/images/architecture/architecture-light.svg" width="100%">
</picture>

**Core** is one Rust binary against one Postgres database: ingestion, entity
resolution, the wiki, pages, chat, and the web UI it serves. It never touches
an external API key.

**Retrieval is two HTTP contracts, not code we ship.** Core consumes an
OpenAI-style `/v1/embeddings` endpoint (required) and a `/v1/rerank` endpoint
(optional), and cannot tell what is behind them. On our own board the installer
provisions them on the NPU; on your machine you run them, and the installer
probes what you point it at and pins the model's fingerprint — so a silently
swapped model can't quietly corrupt your index.

**virtues-api** is the metered edge, and **atlas** owns identity and funding
(Stripe on one side, an opaque account id on the other; it never sees usage).
Both live in this repo under `services/`. There is **no managed hosting** —
nobody runs a server for you; those two services exist so the one you run
doesn't have to hold anyone's API keys.

| Source | Streams | Method |
|---|---|---|
| Google | Calendar, Gmail | OAuth |
| Notion | Pages | OAuth |
| Plaid | Transactions, Accounts, Investments, Liabilities | OAuth |
| Strava | Activities | OAuth |
| GitHub | Events | OAuth |
| iOS | HealthKit, Location, Microphone, Contacts, FinanceKit, EventKit | Device |
| macOS | Apps, Browser, iMessage | Device |

<a id="the-full-install"></a>
<a id="install-properly"></a>
<a id="install-linux-home-server"></a>
## <picture><source media="(prefers-color-scheme: dark)" srcset=".github/images/headings/h2-the-full-install-dark.svg"><img alt="The full install" src=".github/images/headings/h2-the-full-install-light.svg" height="28"></picture>

<a id="what-to-run-it-on"></a>
<a id="requirements"></a>
### <picture><source media="(prefers-color-scheme: dark)" srcset=".github/images/headings/h3-what-to-run-it-on-dark.svg"><img alt="What to run it on" src=".github/images/headings/h3-what-to-run-it-on-light.svg" height="22"></picture>

| Requirement | What, and why |
|---|---|
| **Host OS** | Debian 13+, Ubuntu 24.04 LTS+, or Fedora 40+, `x86_64` or `aarch64`, with systemd and root. A VM is fine; a container is not. |
| **Hardware** | 8 GB RAM and an SSD. **No GPU required** — the model that writes is remote, and of the two local retrieval models only the reranker meaningfully gains from one. |
| **Storage** | NVMe or SATA SSD. The installer classifies *and measures* the disk first: eMMC is workable to ~100k items, microSD is slow and wears out, NFS/SMB is a corruption risk. |
| **Inference** | Two endpoints you run: `/v1/embeddings` (required) and `/v1/rerank` (optional), on loopback, LAN, or VPN — never a public address. |
| **Network** | Outbound 443 only. No port forwarding, no inbound rule, no hostname. |

Machines that clear the bar: a mini PC with 16 GB and an NVMe slot; a used
small-form-factor desktop; an aarch64 SBC with 8 GB and NVMe over PCIe — not
microSD. Full reasoning behind each number in
[What to run it on](docs/setup/requirements.md).

<a id="inference"></a>
### <picture><source media="(prefers-color-scheme: dark)" srcset=".github/images/headings/h3-inference-dark.svg"><img alt="Inference" src=".github/images/headings/h3-inference-light.svg" height="22"></picture>

**Start this before you install.** The first thing the installer asks, before
it touches a package or a disk, is where the retrieval models live. We provision inference on exactly one board,
our own; we do not install GPU or NPU inference software on hardware we can't
test, so on your machine you own the endpoints and we validate them at the
door.

[llama.cpp](https://github.com/ggml-org/llama.cpp)'s `llama-server` speaks both
contracts. These are the invocations our own systemd units use:

```bash
# embedder — CPU is the right answer here (fp32 activations)
llama-server --embedding --pooling mean -m embeddinggemma-300m-qat-Q8_0.gguf \
  --host 127.0.0.1 --port 18181 -c 2048 -b 2048 -ub 2048 -np 1 --cache-ram 0 -ngl 0

# reranker — the half that wants a GPU; drop -ngl for CPU
llama-server --rerank --pooling rank -m gte-reranker-modernbert-base-Q8_0.gguf \
  --host 127.0.0.1 --port 18182 -c 2048 -b 2048 -ub 2048 -np 1 --cache-ram 0 -ngl 99
```

`-np 1` and `--cache-ram 0` take each server from ~2.5 GB resident to ~1 GB.
Known-good embedders: **embeddinggemma-300m** (768, Matryoshka to 256),
gte-small (384), bge-small-en-v1.5 (384), e5-small-v2 (384),
nomic-embed-text-v1.5 (768). Known-good rerankers:
**gte-reranker-modernbert-base**, bge-reranker-v2-m3, jina-reranker-v2.

[Setting up inference](docs/inference.md) is the full page: the exact contract,
the prompt-prefix and dimension rules, what the fingerprint pin protects you
from, and how to change model later without losing your record.

<a id="the-command"></a>
<a id="install-in-one-command"></a>
### <picture><source media="(prefers-color-scheme: dark)" srcset=".github/images/headings/h3-the-command-dark.svg"><img alt="The command" src=".github/images/headings/h3-the-command-light.svg" height="22"></picture>

```bash
curl -sSL https://virtues.com/sh | sudo sh
```

`virtues.com/sh` is the newest stable release, `sh-pre` the prerelease channel,
and the one you install on is remembered —
[Installing](docs/setup/install.md) covers pinning a version and reading the
script before you run it. On a Virtues Server you run neither: the boot medium
arrives flashed.

In about ten minutes it asks how you want inference and validates your
endpoints, measures the disk, installs Postgres 18 + pgvector and Avahi,
advertises the machine on the LAN as `virtues.local`, starts `virtues.service`
— which mints and keeps the Ed25519 secret that *is* this server's identity —
and prints a **6-digit pair code**.

```bash
sudo virtues pair   # prints a fresh code, any time
```

Enter it in the desktop or mobile app
([virtues.com/downloads](https://virtues.com/downloads)). **A browser cannot
pair** — authentication is a held Ed25519 key and a browser tab has none. The
one exception is a browser running *on the server*, trusted as the loopback
console at `http://localhost:8000`.

<a id="commands"></a>
### <picture><source media="(prefers-color-scheme: dark)" srcset=".github/images/headings/h3-commands-dark.svg"><img alt="Commands" src=".github/images/headings/h3-commands-light.svg" height="22"></picture>

| Command | What it does |
|---|---|
| `virtues status` | Health in one screen: identity, inference, subscription, devices. `--json` for the same thing in a stable shape, which is what to paste into a support thread |
| `virtues doctor` | How inference resolved, whether both endpoints are actually serving, whether the server is reachable — every finding with the command that diagnoses it |
| `virtues pair` | Print a code to connect a device |
| `virtues device ls` / `add` / `rm <id>` | Who can reach this server. Revoking is real: the next dial is refused |
| `virtues subscribe` | Connect the server to a subscription, so the assistant can reach a model |
| `virtues sudo` | Approve a pending sensitive action — proves physical access |
| `virtues upgrade` / `rollback` | Move between releases on this server's channel — nothing updates itself |
| `virtues channel` | Print or set the release channel this server follows |
| `virtues backup` / `restore` | Snapshot and restore server state |
| `virtues configure-inference` | Re-validate the embedding endpoint after a model change, and offer to re-embed |
| `virtues reindex` | Rebuild the derived search index from source data |
| `virtues uninstall` | Leave. Prints the full manifest of what it found before touching anything |

One rule for all of them: `sudo virtues …`. The daemon runs as its own
`virtues` user and owns the database, so commands that touch it re-exec
themselves as that user; the ones that change the machine — `upgrade`,
`restore`, `uninstall` — stay root.

<a id="after-its-running"></a>
### <picture><source media="(prefers-color-scheme: dark)" srcset=".github/images/headings/h3-after-it-s-running-dark.svg"><img alt="After it's running" src=".github/images/headings/h3-after-it-s-running-light.svg" height="22"></picture>

A paired server holds nothing yet. Connect a source in the app — Google, Plaid,
Strava, or a phone streaming HealthKit and location — and the record starts
filling. The search index builds as data lands, and each day is written after
that day has ended rather than live, so the first page worth reading arrives
tomorrow. `virtues status` is where you watch all three.

<a id="where-things-live"></a>
### <picture><source media="(prefers-color-scheme: dark)" srcset=".github/images/headings/h3-where-things-live-dark.svg"><img alt="Where things live" src=".github/images/headings/h3-where-things-live-light.svg" height="22"></picture>

| What | Where |
|---|---|
| Binary | `/usr/local/bin/virtues` |
| Config | `/var/lib/virtues/virtues.env` — edit, then `sudo systemctl restart virtues` |
| Data | `/var/lib/virtues` — the Postgres cluster and the file store. `DATA_DIR` at install moves it |
| Models | `/var/lib/virtues/models` |
| Units | `virtues`, plus `virtues-embed` / `virtues-rerank` on the bundled path |
| Ports | `8000` server · `5432` Postgres · `18181`/`18182` embed and rerank |
| Logs | `journalctl -u virtues -f` |

<img src=".github/images/shots/doctor.png" alt="virtues doctor output: an Inference ledger naming the accelerator, the embedding and rerank models and both endpoints serving; a Reach ledger with the LAN address and three paired devices; then two warnings — no iroh identity yet, and no relay configured — each printed with the command that resolves it." width="100%">

**When something breaks:** [When something breaks](docs/operate/recovery.md) is
the owner's page; [agents/build/recovery.md](agents/build/recovery.md) is the
longer runbook behind it.

**The manual** is in [`docs/`](docs/) and publishes to
[virtues.com/docs](https://virtues.com/docs): what to run it on, inference,
installing, reaching your server, upgrading, backup and restore, the CLI.

<a id="reaching-your-server"></a>
<a id="reaching-your-box"></a>
<a id="connect-from-another-machine-v02-preview"></a>
## <picture><source media="(prefers-color-scheme: dark)" srcset=".github/images/headings/h2-reaching-your-server-dark.svg"><img alt="Reaching your server" src=".github/images/headings/h2-reaching-your-server-light.svg" height="28"></picture>

**There is no URL that reaches your server.** Reaching it requires holding a
paired Ed25519 key, not knowing an address.

It is an **iroh endpoint** whose Ed25519 `EndpointId` *is* its identity —
mutual-key auth, no certificate authority. A paired device dials that identity
and iroh finds a path: direct on your LAN, hole-punched across NATs, or bounced
off our relay when neither works. The server dials *out* to the relay, so nothing
inbound is ever opened at home, and it works behind CGNAT, café wifi, and
IPv6-only ISPs — anywhere outbound 443 reaches. The relay moves sealed bytes it
has no key to read; it does see which two keys are talking and how much passes
between them, which is why we call it encrypted end-to-end rather than blind
([walkthrough](agents/archive/relay-walkthrough.html)).

Pairing is **local-first** — no passwords, no email, no magic links: you walk to
the machine, run `virtues pair`, and the code puts the new device's key on the
allowlist. Access is enforced twice —
iroh refuses an unpaired key *before HTTP exists*, and the app-layer bearer
remains the authorization keystone on top. Revocation is real:
`virtues device rm <id>` de-allowlists a key and the next dial is refused. Day
to day you type none of this; the apps hold the key and do the dialing, and on
your home network it answers directly on `:8000`.

<a id="build-on-it"></a>
## <picture><source media="(prefers-color-scheme: dark)" srcset=".github/images/headings/h2-build-on-it-dark.svg"><img alt="Build on it" src=".github/images/headings/h2-build-on-it-light.svg" height="28"></picture>

**Applets** are the extension system — ingestion and behavior as supervised
subprocesses, in any language, declared by a `manifest.toml`. A new data source
is an applet; so is a scheduled job. See
[`applets/AUTHORING.md`](applets/AUTHORING.md).

**Pages** are collaborative documents (ProseMirror + Yjs) with `[[entity]]`
links into the graph, version history, and AI editing. **Drive** is file storage
on an S3 backend. Every entity has a URL, which is what makes the web UI's tabs and split panes work.

<a id="development"></a>
## <picture><source media="(prefers-color-scheme: dark)" srcset=".github/images/headings/h2-development-dark.svg"><img alt="Development" src=".github/images/headings/h2-development-light.svg" height="28"></picture>

For working on the codebase itself, not for running Virtues in production.

**Prerequisites:** a recent stable Rust toolchain, Node 22+ and pnpm, and — on
macOS — Homebrew, which `make db` uses to provision `postgresql@18` + pgvector
and `make dev` uses to install `llama.cpp` for the local sidecars.

```bash
git clone https://github.com/virtues-os/virtues
cd virtues
make init   # .env with a fresh encryption key
make dev    # postgres + api :9002 + core :8000 + web :5173 + embed/rerank
```

One command runs the whole stack; Ctrl-C stops it. First run fetches ~480 MB of
GGUFs into `.data/`.

```bash
make dev WITH_EMBED=0   # skip the sidecars (search off) — UI-only or low-RAM
make dev-info           # per-tab commands, for split logs
make dev-link           # a login URL for the local stack
make help               # every target
```

Two things that surprise everyone: there is **no `./target`** — the virtues
repos share one cargo target directory set by an untracked `.cargo/config.toml`
— and the crate is `virtues`, not `virtues-core`, so tests are
`cargo test -p virtues --lib`.

Several agents work in this one checkout at the same time, which makes a few
ordinary git commands destructive here. [`CLAUDE.md`](CLAUDE.md) is the working
agreement: branch discipline, how to commit safely, and how to claim a
migration number.

<a id="the-repo"></a>
<a id="project-structure"></a>
## <picture><source media="(prefers-color-scheme: dark)" srcset=".github/images/headings/h2-the-repo-dark.svg"><img alt="The repo" src=".github/images/headings/h2-the-repo-light.svg" height="28"></picture>

```
virtues/
├── virtues-core/            # The heart: Rust daemon (crate `virtues`) — API, ingest, agent, wiki, dayline
├── applets/                 # Extension system: ingestion + behavior as subprocesses
├── crates/                  # Shared libraries: iroh transport, reach client, NPU daemon, registry
├── services/                # The metered cloud edge: virtues-api, atlas
├── apps/                    # Clients: web (also the Tauri iOS/desktop app), desktop helper, mac collector
├── docs/                    # The public manual — publishes to virtues.com/docs
├── agents/                  # The workshop: build contracts, records, plans (never published)
├── tools/                   # bootstrap.sh + virtues-installer (virtues.com/sh)
└── masters/, deploy/, vendor/
```

<a id="security"></a>
## <picture><source media="(prefers-color-scheme: dark)" srcset=".github/images/headings/h2-security-dark.svg"><img alt="Security" src=".github/images/headings/h2-security-light.svg" height="28"></picture>

Please report vulnerabilities privately — GitHub → **Security** → *Report a
vulnerability* — rather than in a public issue. The threat model, and who holds
which secret (and who deliberately doesn't), is in
[the privacy model](agents/record/privacy-model.md).

<a id="license"></a>
## <picture><source media="(prefers-color-scheme: dark)" srcset=".github/images/headings/h2-license-dark.svg"><img alt="License" src=".github/images/headings/h2-license-light.svg" height="28"></picture>

- **Server, web app, and infrastructure** — [BUSL-1.1](LICENSE):
source-available, free to self-host for personal or internal organizational
use, **not** for offering a hosted service or commercial hardware product.
Each file converts to Apache 2.0 four years after release.
- **Native apps and the data model** — MIT (see the `LICENSE` file in those
directories, e.g. `apps/web/plugins/`, `apps/mac-source/`).

The repository default is BUSL-1.1 unless a directory says otherwise.

---

<p align="center"><i>Your data. Your infrastructure. Your narrative.</i></p>