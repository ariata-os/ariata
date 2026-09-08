# App Store review access

How an Apple reviewer — who owns no box and cannot reach yours — exercises the
iOS app well enough to clear guideline 2.1.

## The problem

Pairing is limited to the LAN, and not by any check we could relax:

- `/api/pair/consume` is plain HTTP to the box's own origin. The box has no TLS
  surface and no public inbound port (`virtues-core/src/server/mod.rs`).
- The iroh relay cannot carry the pairing step: `crates/virtues-iroh/src/server.rs`
  rejects connections from peers that are not already allowlisted, and pairing
  is *how* a peer gets allowlisted. The relay ticket is the output of consume,
  not an input.

There is no co-location logic to remove. `consume_handler` performs no IP or
subnet validation, and the client's `normalize_server`
(`apps/web/plugins/reach/src/lib.rs`) accepts any `http(s)` origin. "Same
Wi-Fi" is UI copy, not enforcement. So a box that *is* publicly reachable
already pairs from anywhere — the missing piece was only a code that lives long
enough for a review cycle.

Existing code lifetimes are far too short: `oneoff` is 5–30 min, `standing`
rotates every ~20 min. A reviewer may open the app days after submission, and
resubmissions add rounds.

## The mechanism

`VIRTUES_REVIEW_PAIR_CODE` in **`/var/lib/virtues/virtues.env`** — the file
the unit actually reads (`EnvironmentFile=-/var/lib/virtues/virtues.env`), and
the one the installer writes. This doc said `/etc/virtues/env` until 2026-09-03;
on a real box that directory exists and is **empty**, so the variable goes into
a file nothing loads, no row is installed, and the only symptom is a reviewer
who cannot pair. At startup `api::pair::ensure_review_code` installs one
`app_pair_token` row with `kind = 'review'`, `status = 'authorized'`, and a
nominal 10-year expiry.

Nothing else changes. `claim_pair_token` already accepts any authorized,
unexpired row and only consumes `kind = 'oneoff'`, so a review code is
multi-use and permanent for free. The `'review'` value in the `kind` CHECK
constraint arrived as `0058_review_pair_code.sql`, which folded into
`0001_initial.sql` in the 2026-08-18 squash — there is no separate migration to
look for any more, and no release since carries the one without the other.

The code stays **6 digits** because the mobile pairing input is
`inputmode="numeric"` with `maxlength="7"` (`src-tauri/ui/connect.html`). A
longer, higher-entropy token would be untypeable there. Startup refuses
anything that is not exactly 6 digits rather than installing a code a reviewer
cannot enter.

### Why the env gate is the whole safety story

A 1M keyspace on a public origin is acceptable only because two things hold at
once. First, `/api/pair/consume` is rate-limited to **10 attempts per IP per
30-minute window**, which puts a single-source sweep of the space far out of
reach. Second, such a box holds synthetic seed data — a successful guess exposes
a fake life. Absent the env var no review row is ever created, so customer boxes
cannot grow a permanent remote-pairing credential. **Never set this variable on a
box holding a real person's data.** An active review code logs a warning on every
boot for exactly this reason.

> **And the first of those two did not hold, for the whole life of this
> document.** `rate_limit_ip` believes `X-Forwarded-For` only when
> `VIRTUES_TRUSTED_PROXY` is set — off by default, and correctly so, because a
> stock box has no proxy and would otherwise let a LAN client mint a fresh
> budget per request. Behind Caddy, with the variable unset, `consume_handler`
> falls back to the socket peer, which is `127.0.0.1` — **and loopback is exempt
> from the limiter by design**, because an unforwarded loopback request is
> already treated as the owner. So the limiter never ran at all. Measured on the
> review box on 2026-09-03 before the fix: twelve consecutive bad codes, twelve
> 401s, no 429. Unlimited guesses against a 1M keyspace, where a hit earns a
> permanent allowlisted iroh device.
>
> **`VIRTUES_TRUSTED_PROXY=1` is therefore load-bearing on a review box, not a
> tuning knob** — it is provisioning step 6a, and the box now logs
> `REVIEW PAIR CODE IS UNRATE-LIMITED` at boot when the review code is active
> without it. Trusting the header is safe here specifically because the code
> takes the **right-most** entry: a client can prepend arbitrary hops, but
> cannot stop Caddy appending the real peer last. After the fix, the same twelve
> requests give ten 401s and then 429.

Residual risks on the demo box, all bounded and accepted: the rate limit is
per-IP, so a distributed sweep is slowed rather than stopped; a successful guess
earns a permanent iroh allowlist entry, could burn inference credits via
`virtues-api`, and would see anything the reviewer's device synced.

**Rotate the code whenever the box's address becomes known outside App Review**
— publishing the two together is what turns a slow, bounded guess into a targeted
one. Rotation is a one-line env change plus a restart; see
[Between review rounds](#between-review-rounds).

## Current demo box

**The live box's address, instance ID, security-group ID, and current pair code
are deliberately not in this repo.** They live in the private ops note alongside
the App Review submission record. This file describes the *shape* of the box, not
its coordinates.

| | |
|---|---|
| Instance | t4g.medium (4 GB, arm64), 30 GB gp3 |
| Address | Elastic IP + a random `demo-<rand>.virtues.ch` Route 53 record |
| Security group | 80/443 public, no SSH |
| Access | SSM via an instance profile — no key pair |
| Cost | ~$27/mo running, ~$6/mo stopped |

An obscure hostname is not a security control — it only keeps opportunistic
scanners away, and it does even that only while it stays unpublished. The real
controls are the per-IP rate limit on `/api/pair/consume` (10 attempts per
30-minute window, keyed on the proxy-appended XFF entry — but ONLY with
`VIRTUES_TRUSTED_PROXY=1`; see the box above and `consume_handler` in
`virtues-core/src/api/pair.rs`), the synthetic seed data, and the box's
disposability. Verify the limit rather than assuming it: eleven bad codes in a
row must produce a 429.

## Provisioning

1. Launch t4g.medium (arm64 — both `x86_64` and `aarch64` are built), 30 GB
   gp3, the SSM-enabled EC2 instance profile, SG with 80/443 open and 22
   closed. Allocate an Elastic IP so DNS survives stop/start.
2. Route 53 A record → the EIP.
3. Caddy in front: `demo-<rand>.virtues.ch { reverse_proxy 127.0.0.1:8000 }`.
   Port 80 must stay open for the ACME http-01 challenge.
4. 4 GB swap. 4 GB RAM is enough at rest (Postgres + `virtues` + two Q8_0 CPU
   sidecars ≈ 2.5–3 GB) but the seed index build wants headroom. Slow is fine
   here; OOM is not.
5. Install `virtues` from any current release (`gh release list`), pinning it
   with `VIRTUES_VERSION=vX.Y.Z`. Over SSM there is no TTY, and the installer
   asks two questions — so both answers have to arrive as environment:

   ```sh
   curl -fsSL https://virtues.com/sh \
     | VIRTUES_VERSION=vX.Y.Z VIRTUES_INFERENCE=bundled sh -s -- --no-init
   ```

   `--no-init` skips only the interactive pairing handoff; the service is
   enabled and started before that step regardless. `VIRTUES_INFERENCE=bundled`
   is the one that is easy to miss: without it the installer reaches the
   Inference step, finds neither our hardware nor a terminal to ask on, and
   **exits 0** having installed nothing — a failure that reads as success in a
   log you are only skimming.

   **On a release older than the libgomp fix, add `apt-get install -y libgomp1`
   afterwards.** `llama-server` links `libgomp.so.1`, a minimal Ubuntu cloud
   image does not carry it, and both inference sidecars then die at exec with
   status=127 — while the installer still exits 0 and the health check only
   warns. Check `systemctl is-active virtues-embed virtues-rerank` rather than
   trusting the install log.
6. `/var/lib/virtues/virtues.env` (NOT `/etc/virtues/env` — see above):
   `VIRTUES_PUBLIC_URL` + `VIRTUES_REVIEW_PAIR_CODE`. Draw the code randomly
   per round (`shuf -i 100000-999999 -n 1`, on the box, so it never rides in as
   a command parameter) and record it in the private ops note — never in this
   repo, a commit message, or an issue. Then `systemctl restart virtues`.
6a. `VIRTUES_TRUSTED_PROXY=1` in the same file. **Not optional on this box** —
   without it the pair-code rate limit does not run at all behind Caddy, which
   is the whole safety argument for a 6-digit code on a public origin. See
   [Why the env gate is the whole safety story](#why-the-env-gate-is-the-whole-safety-story).
   Prove it after the restart: eleven bad codes in a row must give ten 401s and
   then a 429.
7. `virtues seed` — the 12-week narrative plus the instrumented demo day, so
   the reviewer sees a life rather than an empty shell. **On a release older
   than the `morning_baseline` fix this seeds only a third of the data and says
   nothing about it** (see below); until a release carries the fix, run the
   seed files from a current checkout by hand instead. Either way the last file
   to run must be `demo_reanchor.sql`, which is what puts the instrumented day
   on today — check it: `select occurred_at::date from data_location_point
   group by 1 order by count(*) desc limit 1` should return today's date.
7a. **Install the re-anchor timer.** `demo_reanchor.sql` only runs when
   something runs it, so a box seeded once ages a day at a time and a reviewer
   opening the app two weeks after submission meets the empty Home the pass
   exists to prevent — a review cycle is measured in weeks, and a rejection
   round adds more. Copy the SQL to `/usr/local/share/virtues/demo-reanchor.sql`
   and add a `virtues-demo-reanchor` oneshot unit (`User=postgres`,
   `psql -v ON_ERROR_STOP=1 -d virtues -f <that file>`) behind an `OnCalendar=hourly`
   timer with `Persistent=true` and `OnBootSec=2min` — persistent and on-boot
   because this box is stopped between rounds and must catch up when it comes
   back. Hourly is free: the pass reads its anchor out of the data, so a run
   with nothing to do is one SELECT.

   The residual edge, worth knowing rather than fixing: the box anchors on its
   own `current_date` (UTC here) while Home asks for the *browser's* today. A
   reviewer in Pacific time after 5pm is a calendar day behind the box, so they
   land on the day before the instrumented one — which still carries
   `wiki_events`, just no raw streams. Every other direction lines up.
8. **Give the server a funded api key.** NOT the old "subscribe the account"
   step — that one is genuinely dead, see below — but the server still needs to
   be able to pay for inference, and this is easy to miss now that the two are
   decoupled. The app is chat-first: a reviewer pairs, lands in a chat, types a
   question, and without a key gets a red card reading "Connection failed: no
   virtues_api key — link a subscription first" on every message, with a Retry
   that re-fails. There is no local chat model and the BYO-key screen is
   unreachable from a phone, so the app is simply dead at that point.

   `ensure_bearer` checks `VIRTUES_API_KEY` from the environment before it
   reads the credential vault, so the whole claim/link flow can be skipped:
   generate a 64-hex-char key, insert `(sha256(key), <account_id>, box_id)`
   into **virtues-api**'s `device_keys` — that is the table `bearer_auth`
   actually resolves against; atlas's `box_key` is a mirror and plays no part —
   then put the raw key in the env file and restart. Give `box_id` a real value
   like `review-demo`: a NULL there is retired by the next labeled registration
   on that account. Revoking is a `DELETE` of that one row, and the prepaid
   balance is the damage ceiling.

   ~~The old step:~~ atlas used to issue a `relay_url` only to a subscribed
   account, so without it the reviewer paired and then lost the server the
   moment they left the network they paired from. The open-relay work deleted that coupling on both sides:
   `relay::DEFAULT_RELAY_URL` is compiled into the box ("so a box that never
   signs in is still reachable from its first boot", gated only on the
   box-install marker), and `services/virtues-atlas/src/routes/relay.rs`
   resolves the config "with no subscription requirement — reachability is part
   of ownership, not the subscription". A review box needs no atlas account, no
   claim, and no card.
9. Confirm `REVIEW PAIR CODE ACTIVE` in the boot log — that is the proof the
   row installed. A missing env var fails silently and looks like success.
10. Test-pair a real phone **over cellular**, not Wi-Fi. Wi-Fi would pass via
    the LAN path and prove nothing about the reviewer's experience.

Models: chat routes to `virtues-api`, so no local LLM is needed. Embeddings and
the reranker do run locally, CPU-only, and slowness is acceptable.

## What the first real run found (2026-09-03)

The box described above was launched 2026-07-21 and **never actually brought
up** — Caddy and the binary were installed, then it was stopped the same day.
`/etc/virtues/` was empty, there was no systemd unit, and the `virtues`
database had no tables at all, not even `_sqlx_migrations`. So no review round
has ever exercised this path, and every iOS submission since July went out with
review notes pointing at a box that was switched off. Assume nothing here has
been tested until you have tested it.

Four things broke on the way to a working box, all of them fixed in the same
change as this note. The first is the one that mattered:

- **The pair-code rate limit was not running.** The doc above justified a
  6-digit code on a public origin with "10 attempts per IP per 30 minutes", and
  behind Caddy that limiter never executed — every request looked like loopback,
  and loopback is exempt. Twelve bad codes, twelve 401s. Fixed by
  `VIRTUES_TRUSTED_PROXY=1` (now step 6a) plus a boot-time error when a review
  code is active without it. **This is the failure class to watch for here: a
  security control that is real in the code, correct on a stock box, and inert
  on the one deployment shape this document prescribes.**

- **`virtues seed` was dead.** `demo_narrative.sql` still inserted
  `wiki_days.morning_baseline`, a column migration 0011 dropped. `raw_sql` runs
  a file as one unit, so the whole 12-week narrative and the bookmarks silently
  failed and only `demo_day.sql` landed. Every developer who seeded since that
  migration got a third of the data.
- **The bundled inference sidecars could not start.** `llama-server` links
  `libgomp`, which a minimal Ubuntu does not carry; the installer never
  installed it, and the install still reported success.
- **The seed was frozen in February** — see `seeds/demo_reanchor.sql`, which now
  moves the instrumented day onto today at seed time.

None of these are visible from a green CI run, and three of them present as
success. Budget for a full bring-up, not a checklist tick — and check the
claims in this file against the running server rather than reading them.

**And the one that no amount of server-side care would have caught: the app and
the server can be too far apart to talk.** `POST /api/chat` required `model`
through v0.1.5 and validated it against the allowed list; the change that made
it optional (the server resolves the turn's model from its slot) shipped in
v0.1.6. An app built after that omits the field, so it fails to deserialize on
EVERY message against any older server — including the demo server, which was
on v0.1.5 at the time. Verified by sending the app's exact wire shape: 422
before, a real answer after upgrading.

The demo server must therefore run a release at least as new as the app being
submitted. More generally: servers upgrade only when someone runs
`sudo virtues upgrade`, while phones update themselves, so the app is
structurally the side that runs ahead. Check this pairing explicitly before
every submission — it is invisible until a reviewer types the first message.

## Between review rounds

- Wipe the box and re-seed (`virtues reset --yes`, then provisioning steps 6-7;
  the review code reinstalls itself from the env file on the next start). The
  reviewer's own device data — health, location, contacts, ambient audio — syncs
  onto this box once paired, and it should not persist or bleed into the next
  round. Re-running the seed is also what re-ages the demo data:
  `demo_reanchor.sql` is idempotent and walks the whole life forward to today.
- **Stop**, don't terminate: the EBS volume, seed data, and review-code row all
  survive, and the EIP keeps DNS valid.
- Revoke by clearing the env var and restarting, which retires the row. Rotating
  the var to a new code does the same and installs the replacement.
- Watch `paired_from_ip` in the pairing audit log for anything unexpected.

## App Review notes

Give them the URL and the code, and say the app requires a paired server with
the demo instance standing in for one. Reviewers do not SSH anywhere or run
`virtues pair` — they type an address and six digits.

**The bare hostname works now — and it did not, for one whole rejection.**
`normalize_server` (`apps/web/plugins/reach/src/lib.rs`) used to turn any
scheme-less name into `http://<host>:8000` — right for `virtues.local` or a
LAN IP, wrong for every public demo server, whose security group opens only
80/443. On 2026-09-05 App Review (1.2.16, iPad) typed exactly
`demo-<rand>.virtues.ch`, as the field's own placeholder teaches, and got
`POST http://demo-<rand>.virtues.ch:8000/api/pair/consume: … operation timed
out` in red under the Pair button. Guideline 2.1 rejection, screenshot
attached. The demo box saw nothing: no device row, no log line, because the
request never reached port 443.

Since that day a dotted public hostname normalizes to `https://<host>`; LAN
names (single label, `localhost`, `.local`/`.lan`/`.home`/`.internal`, IPv4
literal) keep `http://…:8000`; a scheme or an explicit port still passes
through as typed. Unit tests pin the reviewer's exact input. Writing
`https://` into the notes remains harmless and is still the cautious form for
any reviewer on a build older than that fix.

Refused pair attempts (bad code → 401, rate limit → 429) now log a WARN with
the client IP, so the next silent rejection can be told apart from one that
never arrived.

**CallKit.** The same rejection carried guideline 5: MIIT requires CallKit
off for apps available in China, and the audio plugin linked `CallKit` for
one read-only `CXCallObserver` (suppress the "recording paused" nudge during a
phone call). Removed the same day — the audio-session interruption hold
carries the same fact. Do not reintroduce a CallKit import for any reason
short of shipping a dialer; the alternative is dropping China from the
territory list in App Store Connect.

## Shooting App Store screenshots

The demo server doubles as the screenshot rig — synthetic data that looks like
a life, re-anchored nightly so "today" is always full. Drive a simulator
against it rather than a real phone: exact store canvas sizes, no personal data,
and `xcrun simctl status_bar override` gives the clean 9:41 full-battery bar.

Three things cost time on 2026-09-04 and will again:

- **Check the size App Store Connect actually asks for.** It wanted 1284×2778
  (6.5"); the newest Pro Max simulator gives 1320×2868 (6.9"). Different aspect
  ratios, so rescaling is not an option — `simctl create` the matching device.
- **`bind 127.0.0.1:7117: Address already in use`.** Simulator apps share the
  Mac's loopback, so the Mac app's proxy port collides. Launch with
  `SIMCTL_CHILD_VIRTUES_PROXY_PORT=<free port>`. Simulator-only; a real phone
  has its own loopback.
- **The founder's letter will not scroll under injected touches.** Not a bug —
  verified in a browser at phone size that the page scrolls normally. The way
  past it is the app's own loopback proxy: point a browser at
  `http://127.0.0.1:<proxy port>`, which IS the paired device's session, and
  click through there. `skipOnboarding` writes server-side, so the app comes
  back past the letter.

Delete any probe chats off the server before shooting the drawer. Recents shows
them, and "Reply with exactly: PREFLIGHT OK" is not what you want in a store
listing.

## Alternative not taken

An in-app demo mode (canned local data, no box, no pairing) would remove the
hosted-box dependency, survive review rounds without a babysat server, avoid
holding a reviewer's personal data, and double as a pre-purchase try-before-you-buy.
It is real product work, which is why the demo box came first. Worth revisiting
if review rounds become routine.
