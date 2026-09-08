//! HTTP server for data ingestion and API

pub mod api;
pub mod faces;
pub mod webhook;
pub mod yjs;

use axum::{
    extract::DefaultBodyLimit,
    middleware,
    response::IntoResponse,
    routing::{delete, get, patch, post, put},
    Json, Router,
};

use std::env;
use std::sync::Arc;

// Re-exported: `AppState` is the handler state for the whole crate, and
// modules outside `server` (api::display, …) legitimately name it. Importing it
// privately here made `crate::server::AppState` fail to resolve for them.
pub use self::webhook::AppState;
use self::yjs::yjs_websocket_handler;
use crate::error::Result;
use crate::middleware::auth::AuthUser;
use crate::Virtues;

/// Run the HTTP ingestion server with integrated scheduler
pub async fn run(client: Virtues, host: &str, port: u16) -> Result<()> {
    // Validate required environment variables early
    validate_environment()?;

    // Prove the lake is writable before serving. Every applet that ingests
    // anything writes here as the `virtues` user, and when the directory was
    // root-owned the only symptom was each applet failing with EACCES on its
    // own schedule — a 500 every five minutes, forever, that no health surface
    // asked about and no human saw. One probe at boot turns days of silent
    // failure into a line in the log at the moment it becomes true.
    //
    // A warning, not a refusal: the box still serves chat, search and the UI
    // without ingest, and refusing to boot would take away the surfaces someone
    // needs in order to FIX this. `is_healthy` carries the remedy.
    match crate::storage::Storage::file(
        crate::storage::lake::lake_root().display().to_string(),
    ) {
        Ok(storage) => match storage.health_check().await {
            Ok(h) if h.is_healthy => tracing::info!("{}", h.message),
            Ok(h) => tracing::error!("{}", h.message),
            Err(e) => tracing::error!(error = %e, "could not probe the lake for writability"),
        },
        Err(e) => tracing::error!(error = %e, "could not open the lake for a write probe"),
    }


    // Reap runs left in `running` by a crash/restart mid-execution, so a stale
    // lock doesn't survive a reboot. (The concurrency gate also age-bounds stale
    // runs at request time; this just keeps the runs table honest on boot.)
    match crate::scheduler::applets::cleanup_stale_runs(client.database.pool()).await {
        Ok(n) if n > 0 => tracing::info!("Reaped {} stale 'running' action run(s) on startup", n),
        Ok(_) => {}
        Err(e) => tracing::warn!("Failed to reap stale action runs: {}", e),
    }

    // Auto-detect server readiness (skips setup screen if previously hydrated)
    if let Err(e) = crate::api::ensure_server_status(client.database.pool()).await {
        tracing::warn!("Failed to ensure server status: {}", e);
    }

    // Seed home_timezone from the box's own system clock once, before the
    // scheduler resolves cron timezones. Idempotent. See agents/record/timezone-model.md.
    if let Err(e) = crate::api::profile::ensure_home_timezone(client.database.pool()).await {
        tracing::warn!("Failed to seed home_timezone: {}", e);
    }

    // Face-reader grants: idempotent default-deny SELECT surface for applet
    // faces (data_*/wiki_* tables + applet_* schemas). Best-effort.
    if let Err(e) = faces::ensure_applet_db_grants(client.database.pool()).await {
        tracing::warn!("face reader grants failed: {e}");
    }

    // Eager identity bringup: ensure the loopback console device exists so the
    // box's own browser is authenticated. Best-effort — a failure here must not
    // stop the box from serving. (The box's TLS identity is its own cert,
    // obtained at relay spawn; no keypair to mint here.)
    {
        let pool = client.database.pool();
        if let Err(e) = crate::middleware::auth::ensure_console_device(pool).await {
            tracing::warn!("identity bringup: ensure_console_device failed: {e}");
        }
    }

    // Sanity-check the pgvector-backed search_vectors table is reachable.
    // Schema creation happens via 0008_search_and_vectors.sql; this probe just
    // confirms the migration ran.
    {
        let search_engine = crate::search::SemanticSearchEngine::new(
            Arc::new(client.database.pool().clone()),
        );
        if let Err(e) = search_engine.ensure_vec_table().await {
            tracing::warn!("Failed to probe search_vectors table: {}", e);
        }
    }

    // Initialize Yjs state early (needed by both server and scheduler)
    let yjs_state = yjs::YjsState::new(client.database.pool().clone());
    yjs_state.start_save_processor();
    tracing::info!("Yjs WebSocket server initialized");

    // System telemetry: the Jetson GPU monitor (idle-gated tegrastats) and the
    // box-local time-series sampler (1/min → app_system_samples) behind the
    // System/Telemetry views. Both are no-ops/best-effort on non-Jetson hosts.
    crate::api::system_telemetry::start_gpu_monitor();
    crate::api::system_telemetry::start_system_sampler(client.database.pool().clone());

    // Reconcile action templates from per-folder manifests — creates/updates
    // system action rows. Safe to call on every startup (user-managed runtime
    // state preserved).
    if let Err(e) = crate::applet_templates::reconcile_templates(client.database.pool()).await {
        tracing::warn!("Failed to reconcile action templates: {}", e);
    }

    // Model facts (prices, context windows, which ids still exist) are fetched
    // from virtues-api, never compiled in. Refreshes on boot and 6-hourly; an
    // unreachable cloud keeps the last snapshot rather than emptying the
    // picker. See api::model_catalog.
    crate::api::model_catalog::spawn(client.database.pool().clone());

    // Start the scheduler in the background
    let db_pool = client.database.pool().clone();
    let scheduler_yjs = yjs_state.clone();
    let _scheduler_handle = tokio::spawn(async move {
        match crate::Scheduler::new(db_pool, scheduler_yjs).await {
            Ok(mut sched) => {
                match sched.sync_jobs().await {
                    Ok(n) => tracing::info!("Scheduled {n} cron actions"),
                    Err(e) => tracing::warn!("Failed to schedule cron actions: {}", e),
                }
                if let Err(e) = sched.start().await {
                    tracing::warn!("Failed to start scheduler: {}", e);
                } else {
                    tracing::info!("Scheduler started successfully");
                    // Never returns: re-derives the job set on a timer (so a
                    // source connected while the box is running gets scheduled
                    // without a restart) and owns the JobScheduler, which must
                    // stay in scope for its jobs to keep firing.
                    sched.run_refresh_loop().await;
                }
            }
            Err(e) => {
                tracing::warn!("Failed to create scheduler: {}", e);
            }
        }
    });

    // Auth-table sweeper: deletes expired pair tokens + sudo requests every
    // 10 minutes, archives `app_auth_event` rows older than 90 days. Lives
    // in-process — same lifecycle as the HTTP server, ends when the daemon
    // ends. See `crate::maintenance::sweeper`.
    crate::maintenance::sweeper::spawn(client.database.pool().clone());

    // Release preparation: on the stable channel, fetch + preflight the next
    // release ahead of time so installing it is a restart rather than a
    // download. Never activates anything — see `api::updates`.
    crate::api::updates::spawn();

    // Pair-code rotator: keeps a fresh universal standing pair code alive at all
    // times (with an overlap window) so the panel and `virtues pair` always have
    // a valid code to display. See `crate::maintenance::pair_rotator`.
    crate::maintenance::pair_rotator::spawn(client.database.pool().clone());

    // Setup access point. An appliance arrives with no network and a display
    // its owner cannot type on, so the box raises its own wifi and the phone
    // does the typing. Up while unclaimed, down once a device pairs — NOT down
    // when the box gets wifi, which would drop the network the phone is still
    // sitting on mid-provision. No-op on a DIY box. See maintenance::setup_ap.
    crate::maintenance::setup_ap::spawn(client.database.pool().clone());

    // BLE wifi provisioning — the Improv service, and the PRIMARY setup path
    // (the AP above is the frozen fallback). Advertised while unclaimed, gone
    // once a device pairs. No-op on a DIY box and on non-Linux dev hosts. See
    // maintenance::ble_provision for the week of hardware findings that led
    // here.
    crate::maintenance::ble_provision::spawn(client.database.pool().clone());

    // The button behind the case. Held for three seconds, it forgets every
    // paired device — and nothing else: not the network, not the account, not
    // the data, and not the phrase. Anyone who can open the case can make that
    // nuisance; only someone holding the four words can then claim the box.
    // No-op off an appliance. See maintenance::reset_button.
    crate::maintenance::reset_button::spawn(client.database.pool().clone());

    // Persistent review pair code, for App Store review boxes only. No-op
    // unless VIRTUES_REVIEW_PAIR_CODE is set, so customer boxes are untouched.
    // A failure here is loud but not fatal: a demo box that came up without
    // its code is useless to a reviewer, and the operator needs to see that,
    // but it must not take down a box that is otherwise healthy.
    {
        let pool = client.database.pool().clone();
        tokio::spawn(async move {
            match crate::api::pair::ensure_review_code(&pool).await {
                Ok(Some(_)) => {
                    tracing::warn!(
                        "REVIEW PAIR CODE ACTIVE — this box accepts a permanent pairing code. \
                         Only ever correct on a disposable box holding synthetic data."
                    );
                    // A review box is public and therefore behind a reverse
                    // proxy, and that combination silently disarms the only
                    // thing standing between a 6-digit code and a permanent
                    // allowlisted device. `rate_limit_ip` believes
                    // `X-Forwarded-For` only when VIRTUES_TRUSTED_PROXY is set;
                    // otherwise `consume_handler` falls back to the socket peer,
                    // which behind a proxy is loopback — and loopback is exempt
                    // from the limiter by design. Net effect: unlimited guesses
                    // at a 1M keyspace, with nothing logged and nothing to see.
                    // Measured on the review box on 2026-09-03: twelve straight
                    // attempts, twelve 401s, no 429.
                    if !crate::middleware::trusted_proxy_configured() {
                        tracing::error!(
                            "REVIEW PAIR CODE IS UNRATE-LIMITED — VIRTUES_TRUSTED_PROXY is not \
                             set. If this box is public behind a reverse proxy, /api/pair/consume \
                             sees every request as loopback and the 10-per-IP-per-30-min limit \
                             never runs, so the code can be brute-forced. Set \
                             VIRTUES_TRUSTED_PROXY=1 and restart."
                        );
                    }
                }
                Ok(None) => {}
                Err(e) => tracing::error!("review pair code not installed: {e:#}"),
            }
        });
    }

    // Entity resolver: periodically turns raw lake primitives (location points,
    // transactions, calendar attendees) into ontology surfaces (visits/places,
    // merchant orgs, people) via `entity_resolution::resolve_entities`. Without
    // this the resolution only ran from the CLI, so the day page / timeline had
    // nothing to show even while the lake filled. See `maintenance::entity_resolver`.
    crate::maintenance::entity_resolver::spawn(client.database.clone());

    // Create ToolExecutor (optional - fails gracefully if VIRTUES_API_INTERNAL_SECRET not set)
    let tool_executor = crate::tools::ToolExecutor::from_env(client.database.pool().clone())
        .map(Arc::new)
        .ok();

    if tool_executor.is_some() {
        tracing::info!("ToolExecutor initialized successfully");
    } else {
        tracing::warn!("ToolExecutor not initialized - VIRTUES_API_INTERNAL_SECRET may not be set");
    }

    // Initialize chat cancellation state for stopping in-progress requests
    let chat_cancel_state = crate::api::chat::ChatCancellationState::new();

    // Create drive config with shared storage backend
    let drive_config = crate::api::DriveConfig::new(client.storage.clone());

    let state = AppState {
        db: client.database.clone(),
        storage: client.storage.clone(),
        drive_config,
        tool_executor,
        yjs_state: yjs_state.clone(),
        chat_cancel_state,
    };

    // ============================================================
    // Public routes (no authentication required)
    // ============================================================
    let public_routes = Router::new()
        // Health check
        .route("/health", get(health))
        // App server info (for device pairing)
        .route("/api/app/server-info", get(server_info))
        // Public, LAN-reachable box health — boot gates + inference resolution.
        // No secrets; the first-run web page / appliance screen poll this
        // before any owner session exists. (Full identity detail stays behind
        // the session-authed /api/box/status.)
        .route(
            "/api/box/health",
            get(crate::api::box_status::box_health_handler),
        )
        // Setup/onboarding state machine (agents/build/onboarding.md) — public-on-LAN
        // for the same reason as /api/box/health: the wizard + panel render it
        // pre-auth, and it carries only booleans + step copy.
        // Who is this box — name + claimed, for discovery chips. Public like
        // its neighbours; the name is already broadcast over the air (AP SSID,
        // BLE advertisement), so the LAN learns nothing new. See api/identity.
        .route(
            "/api/box/identity",
            get(crate::api::identity::identity_handler),
        )
        .route(
            "/api/setup/state",
            get(crate::api::box_status::setup_state_handler),
        )
        // What the box actually holds, counted — the reveal's first movement.
        // Read-only and derived entirely from tables the caller could already
        // read, so it adds no reach, only arithmetic.
        .route("/api/census", get(crate::api::census::census_handler))
        // Draft the document from the answers. POST because it spends money and
        // rewrites the document — not something a refresh should trigger.
        .route(
            "/api/narrative/draft",
            post(crate::api::narrative_draft::draft_handler),
        )
        // The rules the assistant must obey. Read to review them, POST to
        // replace the set with what was confirmed.
        .route(
            "/api/narrative/rules",
            get(crate::api::narrative_draft::rules_handler)
                .post(crate::api::narrative_draft::save_rules_handler),
        )
        .route(
            "/api/setup/skip-onboarding",
            post(crate::api::box_status::skip_onboarding_handler),
        )
        // What the attached 7" display renders. Registered here because the
        // kiosk draws before any device is paired, but UNLIKE its neighbours
        // above it carries the live pair code — so the handler itself refuses
        // anything that isn't loopback. Proximity is the authority: a stranger
        // on the wifi who cannot see the screen must not be able to claim the
        // box. See api/display.rs.
        .route(
            "/api/display/state",
            get(crate::api::display::display_state_handler),
        )
        // Lets the panel latch "an upgrade is running" while this server is
        // still up to say so — after it stops, the kiosk's page is gone with
        // it. See api/display.rs.
        .route(
            "/api/display/updating",
            get(crate::api::display::display_updating_handler),
        )
        // The case button at 1s cadence — the 30s ambient poll cannot see a
        // 3s hold. See api/display.rs.
        .route(
            "/api/display/button",
            get(crate::api::display::display_button_handler),
        )
        // `/api/display/qr` and `/api/display/link-qr` are gone — the panel is
        // one screen now and renders no QR at all. See api/display.rs.
        //
        // Wifi provisioning over the setup AP. The one unauthenticated WRITE
        // surface on the box, and unauthenticated by necessity: the phone that
        // just joined the AP has no credential yet, because obtaining one is
        // what the rest of onboarding is for. Each handler re-checks both gates
        // itself — caller is on the AP subnet (or loopback), and the box is
        // still unclaimed — rather than trusting placement in this router.
        // See api/provision.rs.
        .route(
            "/api/provision/networks",
            get(crate::api::provision::networks_handler),
        )
        .route(
            "/api/provision/join",
            post(crate::api::provision::join_handler),
        )
        .route(
            "/api/provision/status",
            get(crate::api::provision::status_handler),
        )
        // `/portal` and `/provision` are GONE (2026-08-17), and the deletion is
        // the point rather than a cleanup.
        //
        // They were the browser half of onboarding: a phone joins the setup AP,
        // a captive sheet opens, the owner hands over their home wifi. Every
        // part of that is now impossible or unwanted. Pairing needs a held iroh
        // key, so the browser could provision wifi and then strand the owner one
        // step from the end — it served a user who cannot exist. The captive
        // sheet itself was suppressed rather than exploited (iOS renders it in a
        // stripped WebKit, force-reopens it, and caches it per-SSID across
        // upgrades). And BLE carries the whole conversation now, on a radio that
        // survives the switchover the AP path died on.
        //
        // What replaced them: `maintenance::ble_provision` (Improv), and
        // `/api/network/*` for a claimed box that needs to move networks.
        //
        // The `/provision` → `/portal` redirect is gone with them. It existed to
        // un-teach phones that had cached the old SPA URL, and every such phone
        // met a box during the two weeks that flow was live — none of which are
        // customer boxes.
        // Auth — pair-only model. Public consume + session probe (returns the
        // AuthUser resolved from the request's proven iroh key, if any).
        // /api/pair/{mint,confirm,deny,status} are auth'd and live under the
        // protected_routes block below.
        .route(
            "/api/pair/consume",
            post(crate::api::pair::consume_handler),
        )
        .route("/auth/session", get(api::auth_session_handler))
        // Applet faces — the CORS-permissive, token-gated leaves only. The
        // mint route is AUTHENTICATED (in protected_routes): the token is the
        // sole gate on the data door, so obtaining one must require owner auth.
        // The query bridge validates the token; the file routes serve inert
        // assets. These carry no data without a token minted by the authed app.
        .route(
            "/api/face/query",
            post(faces::face_query_handler).options(faces::face_query_preflight),
        )
        .route("/face/:applet_id/", get(faces::face_index_handler))
        .route("/face/:applet_id/*path", get(faces::face_file_handler))
        // Public page sharing (token-based access, no session needed)
        .route("/api/s/:token", get(api::get_shared_page_handler))
        .route(
            "/api/s/:token/files/:file_id",
            get(api::shared_file_download_handler),
        )
        // Webhook ingestion. Authenticated primarily by the proven iroh key
        // (Option<AuthUser>) — the owner's devices POST over iroh — with the
        // legacy Bearer device-token kept only as a fallback for external,
        // non-iroh callers. Lives in public_routes so the bearer fallback path
        // isn't force-rejected by the AuthUser route_layer.
        // Per-route body limit override (router-wide cap is 105MB): iOS audio
        // batches are base64 AAC and can dwarf the other streams on backfill.
        // A body over the cap is rejected by the Json extractor before the
        // handler runs, which historically surfaced as a bogus "no stream
        // selector" action error. See webhook.rs for the rejection handling.
        .route(
            "/webhook/:applet_id",
            post(webhook::webhook).layer(DefaultBodyLimit::max(512 * 1024 * 1024)),
        )
        // Device re-fetch for stream → applet_id map. Used by paired devices
        // whose Keychain entry predates the webhook unification, or after
        // templates.toml adds a new stream. Same device-token bearer auth as
        // the webhook endpoint.
        .route(
            "/api/devices/applet-ids",
            get(api::device_applet_ids_handler),
        )
        // Device-scoped run history for one of the caller's own applets, so the
        // app can show real server-side outcome per stream. Device-token bearer
        // auth + credential-ownership check (see handler). Distinct from the
        // session-authed /api/applets/:id/runs.
        .route(
            "/api/devices/applets/:id/runs",
            get(api::device_applet_runs_handler),
        );

    // ============================================================
    // Protected routes (authentication required via route_layer)
    // ============================================================
    let protected_routes = Router::new()
        // Face-token mint — AUTHENTICATED. The authed app mints a short-lived
        // per-applet token and passes it into the iframe `src` (?vt=). This is
        // the gate on the whole face data door (faces.rs).
        .route("/api/applets/:id/face-token", get(faces::mint_face_token_handler))
        // Timeline day (location chunks for movement map)
        .route("/api/timeline/day/:date", get(api::timeline_get_day_handler))
        // Today streams — location/calendar/audio spans, pre-synthesis (homepage)
        .route("/api/today/:date/streams", get(api::today_streams_handler))
        // Map tiles — the Atlas: box-cached tiles (private + offline). agents/record/map-atlas-plan.md
        .route("/api/map/tiles/:style/:z/:x/:y", get(api::map_tile_handler))
        // Home-page loops — weather · upcoming calendar · unnamed-place backlog
        .route("/api/weather/current", get(api::weather_now_handler))
        .route("/api/calendar/upcoming", get(api::calendar_upcoming_handler))
        .route("/api/places/unnamed", get(api::unnamed_places_handler))
        // ─── Pair-only auth: "+ Add device" from a paired session ─────
        .route("/api/pair/mint",          post(crate::api::pair::mint_handler))
        .route("/api/pair/mint-collector", post(crate::api::pair::mint_collector_handler))
        .route("/api/pair/status/:id",    get(crate::api::pair::status_handler))
        .route("/api/pair/deny/:id",      post(crate::api::pair::deny_handler))
        // ─── Devices: unified list + revoke ───────────────────────────
        .route("/api/devices",            get(crate::api::devices::list_handler))
        .route("/api/devices/self/node-id", post(crate::api::devices::set_self_node_id))
        .route("/api/devices/self/reach",   get(crate::api::devices::get_self_reach))
        .route("/api/devices/enroll-peer",  post(crate::api::devices::enroll_peer))
        .route("/api/devices/:id",        axum::routing::delete(crate::api::devices::revoke_handler))
        // ─── Sudo: gate for high-sensitivity actions ──────────────────
        .route("/api/sudo/request",       post(crate::api::sudo::request_handler))
        .route("/api/sudo/status/:id",    get(crate::api::sudo::status_handler))
        // ─── Audit log ────────────────────────────────────────────────
        .route("/api/audit/auth",         get(crate::api::audit::list_handler))
        // ─── Billing settings (BYO key) ───────────────────────────────
        // BYO routes inference around virtues-api entirely: box calls
        // upstream directly. Save/delete are sudo-gated (change_byo_key);
        // status is a non-secret read for the Billing page.
        .route("/api/settings/byo-key",   get(crate::api::settings_byo::status_handler)
                                          .post(crate::api::settings_byo::save_handler)
                                          .delete(crate::api::settings_byo::delete_handler))
        // ─── Web bundle (the box IS the update server) ────────────────
        // What UI build this box serves, and the build itself. A client that
        // can only run a bundle the box handed it cannot get ahead of the box,
        // which is the point — see api/web_bundle.rs.
        .route("/api/web-bundle/version", get(crate::api::web_bundle::version_handler))
        .route("/api/web-bundle/tarball", get(crate::api::web_bundle::tarball_handler))
        // ─── Billing-state aggregator (local view) ────────────────────
        .route("/api/billing/state",           get(crate::api::billing_state::state_handler))
        .route("/api/billing/auto-topup",      post(crate::api::billing_state::set_auto_topup_handler))
        // Setup wizard transitions (agents/build/onboarding.md) — session-authed; the
        // wizard reads progress from the public /api/setup/state.
        .route("/api/setup/subscribe/start",   post(crate::api::setup::subscribe_start_handler))
        .route("/api/setup/login/start",       post(crate::api::setup::login_start_handler))
        .route("/api/setup/link/poll",         post(crate::api::setup::link_poll_handler))
        // ─── Source OAuth + API-key connect flows ────────────────────
        // Device pairing (iOS / Mac / sensor) lives at /api/pair/* (above).
        // The legacy /api/pairing/initiate + /api/pairing/complete routes
        // were removed in v1 — iOS now pairs via /api/pair/consume with
        // kind = "mobile_app".
        .route(
            "/api/connect/:source_id/start",
            post(crate::api::source_auth::oauth_start_handler),
        )
        .route(
            "/api/connect/:source_id/complete",
            post(crate::api::source_auth::apikey_complete_handler),
        )
        .route(
            "/oauth/callback",
            axum::routing::get(crate::api::source_auth::oauth_callback_handler),
        )
        // Box health for the phone app's status screen (same data as the
        // `virtues status` CLI — one source of truth in api::box_status).
        .route(
            "/api/box/status",
            get(crate::api::box_status::box_status_handler),
        )
        // Device-health endpoint (used by mobile/admin UIs).
        .route("/api/devices/health", get(api::device_health_check_handler))
        // Actions API
        .route(
            "/api/applets",
            get(api::list_applets_handler).post(api::create_applet_handler),
        )
        .route(
            "/api/applets/:id",
            get(api::get_applet_handler)
                .patch(api::patch_applet_handler)
                .delete(api::delete_applet_handler),
        )
        .route("/api/applets/:id/run", post(api::trigger_applet_handler))
        .route("/api/applets/:id/message", post(api::message_applet_handler))
        .route("/api/applets/:id/data", get(api::get_applet_data_handler))
        // Read the applet's own code. Read-only, owner-authed like everything
        // in this group; see api/applet_source.rs for why it guards harder than
        // the face server does.
        .route(
            "/api/applets/:id/source",
            get(crate::api::applet_source::list_handler),
        )
        .route(
            "/api/applets/:id/source/*path",
            get(crate::api::applet_source::file_handler),
        )
        .route(
            "/api/applets/:id/fork",
            post(crate::api::applet_source::fork_handler),
        )
        // Chat-export upload (Tier 3 one-time import). Per-route body limit
        // overrides the router-wide 105MB cap — ChatGPT exports can be larger.
        .route(
            "/api/chat-import/upload",
            post(api::chat_import_upload_handler)
                .layer(DefaultBodyLimit::max(512 * 1024 * 1024)),
        )
        .route(
            "/api/chat-import/status",
            get(api::chat_import_status_handler),
        )
        .route("/api/applets/:id/runs", get(api::list_applet_runs_handler))
        .route("/api/applets/:id/log", get(api::applet_log_handler))
        .route("/api/applets/runs/:id", get(api::get_applet_run_handler))
        .route("/api/runs", get(api::list_runs_handler))
        // Credentials API
        .route("/api/credentials", get(api::list_credentials_handler))
        .route(
            "/api/credentials/:id",
            patch(api::patch_credential_handler).delete(api::delete_credential_handler),
        )
        // Source catalog (drives the Sources tile grid)
        .route("/api/sources", get(api::list_sources_handler))
        // Profile API
        .route("/api/profile", get(api::get_profile_handler))
        .route("/api/profile", put(api::update_profile_handler))
        // Entities API - Places
        .route(
            "/api/entities/places",
            get(api::list_places_handler).post(api::create_place_handler),
        )
        .route(
            "/api/entities/places/:id",
            get(api::get_place_handler)
                .put(api::update_place_handler)
                .delete(api::delete_place_handler),
        )
        .route(
            "/api/assistant/memories",
            axum::routing::get(api::list_assistant_memories_handler),
        )
        .route(
            "/api/assistant/memories/:id",
            axum::routing::put(api::edit_assistant_memory_handler)
                .delete(api::retire_assistant_memory_handler),
        )
        .route(
            "/api/wiki/notes/:subject_type/:subject_id",
            axum::routing::get(api::list_notes_handler).post(api::create_note_handler),
        )
        .route(
            "/api/wiki/notes/:id/resolve",
            axum::routing::put(api::resolve_note_handler),
        )
        .route(
            "/api/wiki/notes-open-count",
            axum::routing::get(api::open_notes_count_handler),
        )
        .route(
            "/api/wiki/lifeline",
            axum::routing::get(api::lifeline_handler),
        )
        .route(
            "/api/wiki/lifeline/ground",
            axum::routing::get(api::lifeline_ground_handler),
        )
        .route(
            "/api/wiki/lifeline/clock",
            axum::routing::get(api::lifeline_clock_handler),
        )
        .route(
            "/api/wiki/lifeline/feed",
            axum::routing::get(api::lifeline_feed_handler),
        )
        .route(
            "/api/wiki/lifeline/processed",
            axum::routing::get(api::lifeline_processed_handler),
        )
        .route(
            "/api/wiki/history",
            axum::routing::get(api::history_feed_handler),
        )
        .route(
            "/api/wiki/articles/:subject_type/:subject_id/history",
            axum::routing::get(api::article_history_handler),
        )
        .route(
            "/api/wiki/subjects/:subject_type/:subject_id/backlinks",
            axum::routing::get(api::subject_backlinks_handler),
        )
        .route(
            "/api/wiki/articles/:subject_type/:subject_id",
            get(api::get_article_handler).post(api::write_article_handler),
        )
        .route(
            "/api/wiki/articles/:subject_type/:subject_id/auto-update",
            axum::routing::put(api::set_article_auto_update_handler),
        )
        .route(
            "/api/entities/people",
            axum::routing::post(api::create_person_handler),
        )
        .route(
            "/api/entities/people/:id",
            axum::routing::delete(api::delete_person_handler),
        )
        .route(
            "/api/entities/orgs",
            axum::routing::post(api::create_org_handler),
        )
        .route(
            "/api/entities/orgs/:id",
            axum::routing::delete(api::delete_org_handler),
        )
        .route(
            "/api/entities/people/:id/reclassify-as-org",
            axum::routing::post(api::reclassify_person_handler),
        )
        .route(
            "/api/entities/places/:id/set-home",
            post(api::set_place_as_home_handler),
        )
        // Places API (Google Places proxy)
        .route(
            "/api/places/autocomplete",
            get(api::places_autocomplete_handler),
        )
        .route("/api/places/details", get(api::places_details_handler))
        // Assistant Profile API
        .route(
            "/api/assistant-profile",
            get(api::get_assistant_profile_handler),
        )
        .route(
            "/api/assistant-profile",
            put(api::update_assistant_profile_handler),
        )
        // Models API
        .route("/api/models", get(api::list_models_handler))
        .route(
            "/api/models/recommended",
            get(api::list_models_with_slots_handler),
        )
        .route("/api/models/:id", get(api::get_model_handler))
        // Personas API
        .route("/api/personas", get(api::list_personas_handler))
        .route("/api/personas", post(api::create_persona_handler))
        .route("/api/personas/:id", get(api::get_persona_handler))
        .route("/api/personas/:id", put(api::update_persona_handler))
        .route("/api/personas/:id", delete(api::hide_persona_handler))
        .route(
            "/api/personas/:id/unhide",
            post(api::unhide_persona_handler),
        )
        .route("/api/personas/reset", post(api::reset_personas_handler))
        // Per-stream ingest freshness — surfaces a stalled source instead of
        // letting it rot silently.
        .route("/api/streams/health", get(api::stream_health_handler))
        .route("/api/streams/days", get(api::stream_days_handler))
        // Subscription & Billing API
        .route("/api/subscription", get(api::get_subscription_handler))
        .route(
            "/api/billing/portal",
            post(api::create_billing_portal_handler),
        )
        .route("/api/billing/claim", post(api::claim_billing_handler))
        .route("/api/billing/subscribe", post(api::subscribe_billing_handler))
        // Wallet balance + recent ledger (proxied from virtues-api /v1/usage).
        .route("/api/billing/usage", get(api::billing_usage_handler))
        // Box-local AI spend breakdown (app_ai_calls) for the Usage tab.
        .route("/api/usage/summary", get(api::usage_summary_handler))
        // Paged individual AI calls (app_ai_calls) for the Usage page's log.
        .route("/api/telemetry/ai-calls", get(api::ai_calls_handler))
        // Device-authorization link flow (web "Connect subscription").
        .route(
            "/api/billing/link/start",
            post(api::billing_link_start_handler),
        )
        .route(
            "/api/billing/link/status",
            get(api::billing_link_status_handler),
        )
        // Search API (Exa) — reaches outside the box
        .route("/api/search/web", post(api::web_search_handler))
        // Local content search — the ⌘K palette. Never leaves the box.
        .route("/api/search/local", post(api::search_local_handler))
        // Unsplash API (cover image search)
        .route("/api/unsplash/search", post(api::unsplash_search_handler))
        // Annotations API (document highlights + margin notes)
        .route(
            "/api/annotations",
            get(api::list_annotations_handler).post(api::create_annotation_handler),
        )
        .route(
            "/api/annotations/:id",
            patch(api::update_annotation_handler).delete(api::delete_annotation_handler),
        )
        .route(
            "/api/notebooks/:id/annotations",
            get(api::list_notebook_annotations_handler),
        )
        // Bulk annotation export as markdown (D4.3)
        .route(
            "/api/annotations/export",
            get(api::export_file_annotations_handler),
        )
        .route(
            "/api/notebooks/:id/annotations/export",
            get(api::export_notebook_annotations_handler),
        )
        // Drive API (user file storage)
        .route(
            "/api/drive/files/:id/reextract",
            post(api::reextract_drive_file_handler),
        )
        .route("/api/drive/usage", get(api::get_drive_usage_handler))
        .route("/api/backup/status", get(api::get_backup_status_handler))
        .route("/api/drive/files", get(api::list_drive_files_handler))
        .route(
            "/api/drive/files/:id",
            get(api::get_drive_file_handler).delete(api::delete_drive_file_handler),
        )
        .route(
            "/api/drive/files/:id/download",
            get(api::download_drive_file_handler),
        )
        .route(
            "/api/drive/files/:id/move",
            put(api::move_drive_file_handler),
        )
        .route("/api/drive/upload", post(api::upload_drive_file_handler))
        .route("/api/drive/folders", post(api::create_drive_folder_handler))
        .route(
            "/api/drive/reconcile",
            post(api::reconcile_drive_usage_handler),
        )
        // Drive trash endpoints
        .route("/api/drive/media", get(api::list_drive_media_handler))
        .route("/api/drive/trash", get(api::list_drive_trash_handler))
        .route(
            "/api/drive/trash/empty",
            post(api::empty_drive_trash_handler),
        )
        .route(
            "/api/drive/files/:id/restore",
            post(api::restore_drive_file_handler),
        )
        .route(
            "/api/drive/files/:id/purge",
            delete(api::purge_drive_file_handler),
        )
        // Media API (content-addressed storage for page-embedded media)
        .route("/api/media/upload", post(api::upload_media_handler))
        .route("/api/media/:id", get(api::get_media_handler))
        // Wiki API
        .route("/api/wiki/resolve/:id", get(api::wiki_resolve_id_handler))
        // Wiki - Person
        // Mention review queue (entity resolution HITL)
        .route("/api/wiki/people", get(api::wiki_list_people_handler))
        .route(
            "/api/wiki/person/:id",
            get(api::wiki_get_person_handler).put(api::wiki_update_person_handler),
        )
        // Wiki - Place
        .route("/api/wiki/places", get(api::wiki_list_places_handler))
        .route(
            "/api/wiki/place/:id",
            get(api::wiki_get_place_handler).put(api::wiki_update_place_handler),
        )
        // Wiki - Organization (table `wiki_orgs`; both URL forms supported)
        .route(
            "/api/wiki/organizations",
            get(api::wiki_list_organizations_handler),
        )
        .route(
            "/api/wiki/orgs",
            get(api::wiki_list_organizations_handler),
        )
        .route(
            "/api/wiki/organization/:id",
            get(api::wiki_get_organization_handler).put(api::wiki_update_organization_handler),
        )
        .route(
            "/api/wiki/org/:id",
            get(api::wiki_get_organization_handler).put(api::wiki_update_organization_handler),
        )
        // Wiki - Thing: retired. Things are gone entirely as of the wiki_things
        // drop — topics are universals, things were particulars, and a
        // particular now accumulates as a floating mention until something
        // promotes it. This comment used to point at /api/things, which no
        // longer exists.
        // Wiki - Narrative Identity. Read-only: the document is edited on its
        // page, and the retired abridged copy took its PUT with it.
        .route(
            "/api/wiki/narrative-identity",
            get(api::wiki_get_narrative_identity_handler),
        )
        // Wiki - Telos
        // Wiki - Act
        // Wiki - Chapter (the life's partition, written by the interview)
        .route(
            "/api/wiki/chapters",
            get(crate::api::narrative_draft::chapters_handler),
        )
        // Wiki - Day
        .route("/api/wiki/days", get(api::wiki_list_days_handler))
        .route("/api/wiki/activity", get(api::wiki_day_activity_handler))
        .route("/api/wiki/on-this-day", get(api::wiki_on_this_day_handler))
        .route(
            "/api/wiki/entity/:id/records",
            get(api::wiki_entity_records_handler),
        )
        .route(
            "/api/wiki/entity/:id/records/facets",
            get(api::wiki_entity_record_facets_handler),
        )
        .route(
            "/api/wiki/day/:date",
            get(api::wiki_get_day_handler).put(api::wiki_update_day_handler),
        )
        // Wiki - Temporal Events
        .route(
            "/api/wiki/day/:date/events",
            get(api::wiki_get_day_events_handler),
        )
        .route("/api/wiki/events", post(api::wiki_create_event_handler))
        .route(
            "/api/wiki/events/:id",
            put(api::wiki_update_event_handler).delete(api::wiki_delete_event_handler),
        )
        .route(
            "/api/wiki/day/:day_id/auto-events",
            delete(api::wiki_delete_auto_events_handler),
        )
        // Wiki - Day Sources (ontology data)
        .route(
            "/api/wiki/day/:date/sources",
            get(api::wiki_get_day_sources_handler),
        )
        // Wiki - Day Chats (in-app + external AI conversations)
        .route(
            "/api/wiki/day/:date/chats",
            get(api::wiki_get_day_chats_handler),
        )
        // Wiki - Day Streams (dynamic ontology queries)
        .route(
            "/api/wiki/day/:date/streams",
            get(api::wiki_get_day_streams_handler),
        )
        // Wiki - Day heart rate (the Autonomic chart)
        .route(
            "/api/wiki/day/:date/heart-rate",
            get(api::day_heart_rate_handler),
        )
        // Admin API — LLM-authoring on-ramp for new actions
        .route("/api/admin/reconcile", post(api::admin_reconcile_handler))
        .route(
            "/api/admin/applets/import-git",
            post(api::import_git_applets_handler),
        )
        // System (operator surface — apps + logs)
        // Live host snapshot + persisted history for the System/Telemetry views.
        .route(
            "/api/system/telemetry",
            get(crate::api::system_telemetry::telemetry_handler),
        )
        .route(
            "/api/system/history",
            get(crate::api::system_telemetry::history_handler),
        )
        // Developer API
        .route("/api/developer/sql", post(api::execute_sql_handler))
        .route("/api/developer/tables", get(api::list_tables_handler))
        // Lake API
        .route("/api/lake/summary", get(api::get_lake_summary_handler))
        .route("/api/lake/streams", get(api::list_lake_streams_handler))
        // Pages API
        .route(
            "/api/pages",
            get(api::list_pages_handler).post(api::create_page_handler),
        )
        .route(
            "/api/pages/search/refs",
            get(api::search_refs_handler),
        )
        .route(
            "/api/pages/:id",
            get(api::get_page_handler)
                .put(api::update_page_handler)
                .delete(api::delete_page_handler),
        )
        // Raw record viewer — one life-graph row by (ontology, id)
        .route(
            "/api/records/:ontology/:record_id",
            get(api::get_record_handler),
        )
        // Page References (backlinks) API
        .route(
            "/api/pages/:id/backlinks",
            get(api::get_page_backlinks_handler),
        )
        // Append a markdown block through Yjs (safe with an open editor) — the
        // synthesis bridge's write path.
        .route("/api/pages/:id/append", post(api::append_page_handler))
        // Page Share API
        .route(
            "/api/pages/:id/share",
            post(api::create_page_share_handler)
                .get(api::get_page_share_handler)
                .delete(api::delete_page_share_handler),
        )
        // Page Versions API
        .route(
            "/api/pages/:id/versions",
            get(api::list_page_versions_handler).post(api::create_page_version_handler),
        )
        .route(
            "/api/pages/versions/:version_id",
            get(api::get_page_version_handler),
        )
        // Box network management (Settings → Box → Network) — the authed
        // successors to the setup-phase /api/provision/* surface, which
        // correctly evaporates at claim time. Born of a box marooned on a
        // captive guest network with no way to leave (2026-08-11). See
        // api/network.rs.
        .route("/api/network/status", get(crate::api::network::status_handler))
        .route("/api/network/scan",   get(crate::api::network::scan_handler))
        .route("/api/network/join",   post(crate::api::network::join_handler))
        // The rendezvous, named and switchable (open-relay-plan §Work 2).
        .route(
            "/api/network/relay",
            get(crate::api::network::relay_status_handler)
                .put(crate::api::network::relay_toggle_handler),
        )
        // Box updates (Settings → Box)
        .route("/api/system/update", get(api::update_status_handler))
        .route(
            "/api/system/update/channel",
            put(api::set_channel_handler),
        )
        .route(
            "/api/system/update/apply",
            post(api::apply_update_handler),
        )
        // The box's attached screen (Settings → Display). Deliberately NOT in
        // the loopback-only /api/display/* family: that module's uniform
        // box-local rule is its security argument, and these are the paired
        // device's side of the glass — panel facts, the ambient face choice,
        // and the restart verb. Nothing here carries the setup phrase.
        .route(
            "/api/system/display",
            get(crate::api::system_display::get_display_settings_handler),
        )
        .route(
            "/api/system/display/face",
            put(crate::api::system_display::set_display_face_handler),
        )
        .route(
            "/api/system/display/hours",
            put(crate::api::system_display::set_display_hours_handler),
        )
        .route(
            "/api/system/display/restart",
            post(crate::api::system_display::restart_display_handler),
        )
        // Re-open onboarding: revoke every device, keep the data. Sits beside
        // the update routes because it is the same kind of thing — a box-wide
        // action a paired device may take, guarded by being paired.
        .route(
            "/api/pair/reopen-onboarding",
            post(crate::api::pair::reopen_onboarding_handler),
        )
        // Bookmarks API (saved web content — the manual capture door)
        .route(
            "/api/bookmarks",
            get(api::list_bookmarks_handler).post(api::save_bookmark_handler),
        )
        .route("/api/bookmarks/{id}", get(api::get_bookmark_handler))
        // The note has its own route rather than a general PATCH: every other
        // column here belongs to a source or to the enrichment pass, and an
        // endpoint that could write them would eventually be used to.
        .route(
            "/api/bookmarks/{id}/note",
            axum::routing::patch(api::update_bookmark_note_handler),
        )
        // Sidebar pins API
        .route(
            "/api/pins",
            get(api::list_pins_handler).post(api::create_pin_handler),
        )
        .route("/api/pins/reorder", put(api::reorder_pins_handler))
        .route(
            "/api/pins/:id",
            patch(api::update_pin_handler).delete(api::delete_pin_handler),
        )
        // Notebooks API (the "room" a chat lives in)
        .route(
            "/api/notebooks",
            get(api::list_notebooks_handler).post(api::create_notebook_handler),
        )
        .route(
            "/api/notebooks/:id",
            get(api::get_notebook_handler)
                .put(api::update_notebook_handler)
                .delete(api::delete_notebook_handler),
        )
        // Notebook membership (items come back inside GET /api/notebooks/:id)
        .route(
            "/api/notebooks/:id/items",
            post(api::add_notebook_item_handler).delete(api::remove_notebook_item_handler),
        )
        .route(
            "/api/notebooks/:id/items/reorder",
            put(api::reorder_notebook_items_handler),
        )
        .route(
            "/api/notebooks/:id/items/role",
            put(api::set_notebook_item_role_handler),
        )
        .route("/api/notebooks/:id/graph", get(api::notebook_graph_handler))
        // Chats API
        .route(
            "/api/chats",
            get(api::list_chats_handler).post(api::create_chat_handler),
        )
        .route(
            "/api/chats/:id",
            get(api::get_chat_handler)
                .patch(api::update_chat_handler)
                .delete(api::delete_chat_handler),
        )
        .route("/api/chats/title", post(api::generate_chat_title_handler))
        // Chat Usage & Compaction API
        .route("/api/chats/:id/usage", get(api::get_chat_usage_handler))
        .route("/api/chats/:id/compact", post(api::compact_chat_handler))
        // Chat API (streaming)
        .route("/api/chat", post(api::chat_handler))
        .route("/api/chat/cancel", post(api::cancel_chat_handler))
        .route("/api/ai/complete", post(api::ai_complete_handler))
        // Chat Edit Permissions API
        .route(
            "/api/chats/:id/permissions",
            get(api::list_chat_permissions_handler).post(api::add_chat_permission_handler),
        )
        .route(
            "/api/chats/:id/permissions/:entity_id",
            delete(api::remove_chat_permission_handler),
        )
        // Terminal API (WebSocket)
        .route(
            "/ws/terminal",
            get(crate::api::terminal::terminal_ws_handler),
        )
        // Paste/drop a file into the terminal: writes it under the user's home
        // and returns the path, which the frontend types at the cursor.
        .route(
            "/api/terminal/paste",
            post(crate::api::terminal::terminal_paste_handler)
                .layer(DefaultBodyLimit::max(25 * 1024 * 1024)),
        )
        // Yjs WebSocket (real-time collaborative editing)
        .route("/ws/yjs/:page_id", get(yjs_websocket_handler))
        // Blanket auth: all routes in this group require a resolved AuthUser
        // (proven iroh key / loopback console / dev fallback).
        .route_layer(middleware::from_extractor_with_state::<AuthUser, _>(state.clone()));

    // Merge public + protected, apply shared state and body limits, then
    // wrap in the security layers (response headers).
    //
    // The connectivity-probe interceptor that used to sit outermost here is
    // gone with `/portal`. It answered iOS/Android/Windows probes with their
    // vendor's success token so the captive sheet would never open — a real
    // fix, but for a condition that only arises on the setup AP's own subnet,
    // and its other half redirected `10.42.0.1/` to a portal that no longer
    // exists. It also ran a Host-header comparison on every request to every
    // box forever, to serve a network that a customer box never raises.
    let app = public_routes
        .merge(protected_routes)
        .with_state(state.clone())
        .layer(middleware::from_fn(crate::middleware::security::headers_layer))
        .layer(DefaultBodyLimit::max(260 * 1024 * 1024)); // 260MB (slightly above 250MB file limit for multipart overhead)

    // API namespaces must NEVER fall through to the SPA fallback below: an
    // unknown /api path answered with a cacheable 200 index.html poisons
    // clients — the browser caches HTML against the API URL and keeps serving
    // it after the route ships (same failure class as the /health story in
    // apps/web/vite.config.ts). Unknown API routes are an honest JSON 404.
    let app = app
        .route("/api/*__unmatched", axum::routing::any(api_not_found_handler))
        .route("/auth/*__unmatched", axum::routing::any(api_not_found_handler));

    // Add static file serving for SPA frontend
    // This serves the SvelteKit static build and falls back to 200.html for SPA routing
    let static_dir =
        std::env::var("STATIC_DIR").unwrap_or_else(|_| "../../apps/web/build".to_string());
    let static_path = std::path::Path::new(&static_dir);

    let app = if static_path.exists() && static_path.is_dir() {
        use tower_http::services::{ServeDir, ServeFile};

        let fallback_file = static_path.join("200.html");
        let serve_dir = if fallback_file.exists() {
            ServeDir::new(&static_dir).fallback(ServeFile::new(fallback_file))
        } else {
            // Try index.html as fallback if 200.html doesn't exist
            let index_file = static_path.join("index.html");
            ServeDir::new(&static_dir).fallback(ServeFile::new(index_file))
        };

        tracing::info!("Static file serving enabled from: {}", static_dir);
        // HTML DOCUMENTS ARE NEVER CACHED. `ServeDir` sends `last-modified` and
        // no `cache-control`, which licenses a browser to cache heuristically —
        // and on 2026-08-10 that made the appliance's panel keep rendering a
        // three-day-old UI after an upgrade, through a service restart and a
        // power cycle. The shell names content-hashed JS chunks, so a stale
        // shell resurrects the entire stale page while the box serves the new
        // one, and the only symptom is a screen that quietly lies about its own
        // version.
        //
        // Scoped to documents on purpose: `/_app/immutable/*` is content-hashed
        // and *should* be cached hard. It is the shell that must always be
        // re-fetched, because it is the thing that names the rest.
        app.fallback_service(tower::ServiceBuilder::new()
            .layer(axum::middleware::from_fn(no_store_for_documents))
            .service(serve_dir))
    } else {
        tracing::info!(
            "No static directory found at: {} - static serving disabled",
            static_dir
        );
        app
    };

    // CORS: the app is a bundled SPA at its own `tauri://` origin that calls
    // this API cross-origin over the iroh loopback, so some cross-origin access
    // must be allowed. It is an ALLOWLIST, not `Any`.
    //
    // `Any` was wrong here, and the reasoning that justified it — "auth is the
    // proven iroh key, not Origin/cookies, so relaxing same-origin never
    // relaxes the transport allowlist" — was true of the iroh credential and
    // missed the second one. There are two ways to be the owner:
    //
    //   1. a paired iroh key (what that comment was about), and
    //   2. being on loopback (`middleware/auth.rs` — a request from 127.0.0.1
    //      with no forwarding header IS the owner).
    //
    // And the desktop app binds `127.0.0.1:7117` and splices whatever connects
    // to it over its own paired identity. So: the owner runs the app, then
    // visits any web page — an ad, a forum, a compromised site. That page runs
    // `fetch('http://127.0.0.1:7117/api/drive/files')`. The box authenticates it
    // as the owner, and `Access-Control-Allow-Origin: *` let the attacker's page
    // READ the reply. `allow_methods(Any)` + `allow_headers(Any)` meant
    // preflighted POSTs succeeded too — including `/api/developer/sql`.
    //
    // A remote page's origin is `https://whatever.example`, which matches none
    // of the arms below, so the browser refuses to hand it the response. The
    // app, the box's own web UI, and local development all still match.
    //
    // `server/faces.rs` keeps its own `*` header deliberately: faces are served
    // into an opaque-origin iframe under a strict CSP and carry no ambient
    // authority. `api/terminal.rs` already does an explicit same-origin check
    // for the same reason this layer now exists.
    let app = app.layer(
        tower_http::cors::CorsLayer::new()
            .allow_origin(tower_http::cors::AllowOrigin::predicate(
                |origin: &axum::http::HeaderValue, req| {
                    origin.to_str().is_ok_and(|o| {
                        // A face lives in `<iframe sandbox="allow-scripts">`,
                        // whose opaque origin serializes as the literal
                        // "null". This layer answers the CORS preflight before
                        // faces.rs's own `*` header can, so without this arm
                        // the sandboxed face's fetch to its bridge is refused
                        // and the panel silently renders no data. Scoped to
                        // the face routes only: a face carries no ambient
                        // authority (face token + face_reader role), and
                        // everything else keeps rejecting "null".
                        face_origin_allowed(o, req.uri.path())
                    })
                },
            ))
            .allow_credentials(false)
            .allow_methods(tower_http::cors::Any)
            .allow_headers(tower_http::cors::Any)
            // Cross-origin callers (the app's own tauri:// origin) may READ the
            // build stamp — it's how a page notices the box moved under it.
            .expose_headers([axum::http::HeaderName::from_static(
                "x-virtues-box-build",
            )]),
    );

    // Outermost, so every response — API, static, fallback — carries the build
    // stamp the SPA's staleness watcher compares.
    let app = app.layer(axum::middleware::from_fn(stamp_box_build));

    // iroh reach: the box is an iroh Endpoint that serves this same axum app
    // (LAN-direct → hole-punch → our relay), reachable by EndpointId with no
    // public inbound port. Serves a clone of `app`; the :8000 TCP listener below
    // keeps serving LAN/loopback + the desktop :7117 helper. See `crate::relay`.
    crate::relay::maybe_spawn(client.database.pool().clone(), app.clone());

    // Hours — the screen's sleep schedule, enforced server-side because sleep
    // is a precedence state (a held button must wake dark glass). No-op off
    // an appliance. See api::system_display::sleep_engine.
    crate::api::system_display::sleep_engine::spawn(client.database.pool().clone());

    let transport = build_transport(host, port);
    let listener = transport.bind().await?;

    tracing::info!("Server listening on {}", transport.describe());

    // DIY discovery aid: the operator ran `compose up` and knows their host, so
    // just point them at the web UI + the CLI dashboard. `0.0.0.0` means "all
    // interfaces" — they reach it at this box's LAN IP.
    {
        let shown = if host == "0.0.0.0" || host == "::" {
            format!("http://<this-box-ip>:{port}")
        } else {
            format!("http://{host}:{port}")
        };
        tracing::info!("Open the Virtues web UI at {shown}  ·  run `virtues status` for setup steps");
    }

    // Run the server with graceful shutdown — Ctrl+C / SIGTERM.
    let shutdown_signal = async move {
        let _ = tokio::signal::ctrl_c().await;
        tracing::info!("shutdown signal received");
    };

    // Plain HTTP on :8000 is the only listener. The box has no TLS surface —
    // paired daemons reach the box over a WG tunnel (which provides encryption
    // + authentication), and the box's own browser hits localhost (Secure
    // Context per W3C, no cert required). See [[localhost-daemon-trust]] in
    // MEMORY.md for the architectural commitment.
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal)
    .await?;

    // Flush queued page edits before the process goes away.
    //
    // The note that used to sit here said no flush was needed — true of the old
    // StreamWriter, never true of the Yjs save queue, which holds the owner's
    // most recent typing for up to ~2.5s. Without this, every `systemctl
    // restart virtues` and every self-update dropped it.
    yjs_state.flush_pending_saves().await;
    tracing::info!("Server shutting down gracefully");

    // Note: scheduler runs in background and will stop when the process exits
    // The handle is dropped here, but the task continues running

    Ok(())
}

/// Build the server transport for this build profile.
///
/// Selection is compile-time — the `dev-transport` feature flips to the
/// loopback-only dev profile. Release builds compile only the `real`
/// arm; the dev profile is not reachable at runtime.
#[cfg(feature = "dev-transport")]
fn build_transport(
    _host: &str,
    port: u16,
) -> Box<dyn virtues_helpers::transport::ServerTransport> {
    Box::new(virtues_helpers::transport::DevLocalServerTransport::new(port))
}

#[cfg(not(feature = "dev-transport"))]
fn build_transport(
    host: &str,
    port: u16,
) -> Box<dyn virtues_helpers::transport::ServerTransport> {
    Box::new(virtues_helpers::transport::RealServerTransport::new(host, port))
}

/// Validate required environment variables at startup
fn validate_environment() -> Result<()> {
    // Log storage path being used. Resolved, not re-derived — a log line that
    // disagreed with the writer would be worse than no log line at all.
    tracing::info!(
        "Using storage path: {}",
        crate::storage::lake::lake_root().display()
    );

    tracing::debug!("Environment validation passed");
    Ok(())
}

/// Honest 404 for unknown /api and /auth paths — see the comment where this
/// is routed. `no-store` so a transient miss can never poison an HTTP cache.
/// Stamp `Cache-Control: no-store` on every HTML document the static server
/// hands out, leaving hashed assets alone.
///
/// See the call site for the incident. Short version: `ServeDir` sends
/// `last-modified` and no `cache-control`, a browser may then cache
/// heuristically, and the appliance's kiosk did — pinning the panel to a
/// three-day-old UI across an upgrade, a service restart, and a power cycle.
///
/// Keyed on the response's own content type rather than the request path, so it
/// covers the SPA fallback (`200.html`, served for arbitrary routes) without
/// having to enumerate which paths are documents.
async fn no_store_for_documents(
    req: axum::extract::Request,
    next: axum::middleware::Next,
) -> axum::response::Response {
    let mut res = next.run(req).await;
    let is_document = res
        .headers()
        .get(axum::http::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|ct| ct.starts_with("text/html"));
    if is_document {
        res.headers_mut().insert(
            axum::http::header::CACHE_CONTROL,
            axum::http::HeaderValue::from_static("no-store"),
        );
        // A validator left beside `no-store` is a mixed message, and some
        // caches honour the weaker half.
        res.headers_mut().remove(axum::http::header::LAST_MODIFIED);
        res.headers_mut().remove(axum::http::header::ETAG);
    }
    res
}

/// Stamp every response with the running build identity, so an open page can
/// notice the box changed underneath it.
///
/// After an upgrade the flipped `web/` slot no longer contains the OLD page's
/// content-hashed `/_app/immutable/*` chunks, so that page's first lazy
/// navigation 404s — and nothing told any connected surface to reload: only
/// the tab that pressed the update button (`location.reload()`) and the kiosk
/// (`restart_display`) recovered. Browsers on other machines and the Mac
/// webview kept a page whose chunks were gone. The SPA watches this header
/// across its own requests and soft-reloads from the background when it moves
/// (see `$lib/build.ts`).
async fn stamp_box_build(
    req: axum::extract::Request,
    next: axum::middleware::Next,
) -> axum::response::Response {
    static VALUE: std::sync::OnceLock<axum::http::HeaderValue> = std::sync::OnceLock::new();
    let value = VALUE.get_or_init(|| {
        let commit: &str = env!("GIT_COMMIT");
        let short = &commit[..commit.len().min(7)];
        axum::http::HeaderValue::from_str(&format!("{} {}", crate::codename::version(), short))
            .unwrap_or(axum::http::HeaderValue::from_static("unknown"))
    });
    let mut res = next.run(req).await;
    res.headers_mut()
        .insert("x-virtues-box-build", value.clone());
    res
}

async fn api_not_found_handler(uri: axum::http::Uri) -> impl IntoResponse {
    (
        axum::http::StatusCode::NOT_FOUND,
        [(axum::http::header::CACHE_CONTROL, "no-store")],
        axum::Json(serde_json::json!({
            "error": "not_found",
            "path": uri.path(),
        })),
    )
}

async fn health(axum::extract::State(state): axum::extract::State<AppState>) -> impl IntoResponse {
    // Check database connectivity with a simple query
    let db_status = match sqlx::query("SELECT 1").execute(state.db.pool()).await {
        Ok(_) => "connected",
        Err(_) => "disconnected",
    };

    let is_healthy = db_status == "connected";
    let status_code = if is_healthy {
        axum::http::StatusCode::OK
    } else {
        axum::http::StatusCode::SERVICE_UNAVAILABLE
    };

    let min_ios_version =
        std::env::var("MIN_IOS_APP_VERSION").unwrap_or_else(|_| "1.0".to_string());

    (
        status_code,
        Json(serde_json::json!({
            "status": if is_healthy { "healthy" } else { "unhealthy" },
            "version": crate::codename::version(),
            "channel": crate::codename::channel(),
            "commit": env!("GIT_COMMIT"),
            "built_at": env!("BUILD_TIME"),
            "min_ios_version": min_ios_version,
            "database": db_status,
            "pool": {
                "size": state.db.pool().size(),
                "idle": state.db.pool().num_idle(),
            }
        })),
    )
}

/// Server info endpoint for device pairing
/// Returns the API endpoint URL for iOS device configuration
async fn server_info() -> impl IntoResponse {
    // Resolution: PUBLIC_API_URL (explicit override) → BACKEND_URL → localhost fallback
    let api_endpoint = std::env::var("PUBLIC_API_URL")
        .or_else(|_| std::env::var("BACKEND_URL"))
        .unwrap_or_else(|_| "http://localhost:8000".to_string());

    Json(serde_json::json!({
        "apiEndpoint": api_endpoint
    }))
}

/// The routes a sandboxed (opaque-origin) applet face is allowed to reach:
/// its own static files and the query bridge. See the CORS predicate.
fn is_face_path(path: &str) -> bool {
    path.starts_with("/api/face/") || path.starts_with("/face/")
}

/// The CORS predicate, named so it can be tested: our own origins anywhere,
/// and the opaque origin `null` only on the face routes.
fn face_origin_allowed(origin: &str, path: &str) -> bool {
    origin_is_ours(origin) || (origin == "null" && is_face_path(path))
}

/// Is this `Origin` one of ours?
///
/// The allowlist behind the CORS layer. Kept as a named function with tests
/// because it is the thing standing between a random web page and the owner's
/// record, and a subtle parsing slip here is invisible in review.
///
/// Allowed: the app's `tauri://` origin, loopback on any port (the desktop
/// proxy on 7117, the box's own UI, `pnpm dev` on 5173), and the box's `.virtues`
/// name. A page served from a remote host has none of these origins.
pub(crate) fn origin_is_ours(origin: &str) -> bool {
    // The app's own origin: `tauri://localhost` on macOS/iOS,
    // `https://tauri.localhost` on Windows — and `virtues://` on the phone,
    // which registers its OWN scheme so an OTA bundle can answer requests
    // (see apps/web/src-tauri/src/lib.rs; the window opens at
    // `virtues://localhost/connect.html`).
    //
    // Missing `virtues://` here silently broke every data request the iOS app
    // made from 2026-08-18 until 2026-08-28: the box answered 200 and omitted
    // `Access-Control-Allow-Origin`, so WebKit discarded the response and the
    // app reported "Load failed" while the box's own logs showed the device
    // authenticating perfectly. Ten days, because the symptom looks like a
    // network fault and every trace says the network is fine.
    //
    // Safe for the same reason `tauri://` is: a custom scheme can only be
    // claimed by an installed app, so no remote page can present this origin.
    if origin.starts_with("tauri://")
        || origin.starts_with("virtues://")
        || origin == "https://tauri.localhost"
    {
        return true;
    }

    // Everything else must be an http(s) origin; anything else (file://, data:,
    // a bare "null") is not ours.
    let rest = match origin.split_once("://") {
        Some(("http", r)) | Some(("https", r)) => r,
        _ => return false,
    };

    // Strip the port. An IPv6 literal is bracketed (`[::1]:8000`), so splitting
    // on the LAST colon leaves the brackets intact and does not cut inside the
    // address.
    let host = match rest.rsplit_once(':') {
        // Only treat the tail as a port if it looks like one; otherwise the
        // colon belonged to the host (an unbracketed IPv6, which is invalid in
        // an origin anyway).
        Some((h, port)) if !port.is_empty() && port.bytes().all(|b| b.is_ascii_digit()) => h,
        _ => rest,
    };

    // Exact matches only. `localhost.evil.example` must NOT pass, which is why
    // this is not a `contains` or a suffix test.
    matches!(host, "localhost" | "127.0.0.1" | "[::1]")
        || host == "virtues"
        || host.ends_with(".virtues")
}

#[cfg(test)]
mod cors_tests {
    use super::{face_origin_allowed, origin_is_ours};

    /// A remote page must not be allowed to read the box's responses.
    ///
    /// This is the CRITICAL case. The desktop app binds 127.0.0.1:7117 and
    /// splices to the box over its paired identity, and loopback counts as the
    /// owner — so with `allow_origin(Any)` any site the owner visited could
    /// `fetch()` the box and READ the reply.
    #[test]
    fn a_remote_page_is_refused() {
        for o in [
            "https://evil.example",
            "http://evil.example:7117",
            // Near-misses that a substring or suffix test would wave through.
            "http://localhost.evil.example",
            "https://tauri.localhost.evil.example",
            "http://notvirtues",
            // Near-misses on the scheme itself: only the exact `virtues://`
            // prefix is ours, never a host or path that merely contains it.
            "https://virtues.evil.example",
            "http://evil.example/virtues://",
            "http://evil.example/localhost",
            // Non-http schemes and the opaque origin.
            "null",
            "file://",
            "data:text/html,x",
        ] {
            assert!(!origin_is_ours(o), "must refuse {o}");
        }
    }

    /// ...while everything that is genuinely ours still works, or the app
    /// breaks and someone reverts the whole fix.
    #[test]
    fn our_own_origins_are_allowed() {
        for o in [
            "tauri://localhost",
            // The iOS app's own scheme. Absent from this list until
            // 2026-08-28, which is exactly how the phone lost every data
            // request for ten days without a single test going red.
            "virtues://localhost",
            "https://tauri.localhost",
            "http://localhost:5173",
            "http://127.0.0.1:7117",
            "http://[::1]:8000",
            "http://localhost",
            "http://box.virtues:8000",
            "http://virtues:8000",
        ] {
            assert!(origin_is_ours(o), "must allow {o}");
        }
    }

    /// The face is hung in `<iframe sandbox="allow-scripts">`, so its origin
    /// is the literal "null". It must clear CORS on its own routes: the panel
    /// drew background stars and nothing else from cohort launch to
    /// 2026-09-03 because it didn't, and no test went red.
    #[test]
    fn the_sandboxed_face_reaches_its_own_routes() {
        for p in [
            "/face/applet_dot_cloud/",
            "/face/applet_dot_cloud/virtues.js",
            "/api/face/query",
        ] {
            assert!(face_origin_allowed("null", p), "must allow null on {p}");
        }
    }

    /// ...and nowhere else. A sandboxed or file:// page still cannot read
    /// the box. Near-misses included: only the exact prefixes are face routes.
    #[test]
    fn null_is_refused_everywhere_else() {
        for p in [
            "/api/status",
            "/api/applets/applet_dot_cloud/face-token",
            "/display",
            "/api/faces/x",
            "/faces/x",
            "/face",
        ] {
            assert!(!face_origin_allowed("null", p), "must refuse null on {p}");
        }
    }
}
