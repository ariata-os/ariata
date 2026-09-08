//! Bearer-auth account endpoints: who am I, and what have I spent.
//!
//! `/v1/charge-test` used to live here too — a smoke endpoint that debited
//! any caller-chosen amount from the bearer's own wallet. It was mounted in
//! production; a leaked device key could drain a wallet in one call. Removed.

use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Json},
    routing::get,
    Router,
};
use serde_json::json;
use std::sync::Arc;

use crate::bearer_auth::BearerAuth;
use crate::entitlement;
use crate::AppState;

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/v1/whoami", get(whoami))
        .route("/v1/usage", get(usage))
}

/// Balance + recent ledger entries for the authenticated account. Drives the
/// box's billing/usage surface ("here's your balance, here's where it went").
async fn usage(State(state): State<Arc<AppState>>, BearerAuth(acct): BearerAuth) -> impl IntoResponse {
    match entitlement::usage_summary(&state.db, &acct.account_id, 50).await {
        Ok(summary) => (StatusCode::OK, Json(summary)).into_response(),
        Err(e) => {
            tracing::warn!("usage summary failed: {e:#}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": { "code": "internal" } })),
            )
                .into_response()
        }
    }
}

/// Returns a non-sensitive summary of the resolved account (balance, caps).
/// Useful for verifying api_key auth + account lookup work end-to-end.
async fn whoami(BearerAuth(acct): BearerAuth) -> impl IntoResponse {
    Json(json!({
        "balance_micros": acct.balance_micros,
        "expires_at": acct.expires_at,
    }))
}
