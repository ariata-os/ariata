//! Account + ledger data layer — linked prepaid model (v1).
//!
//! One row per account in `accounts`:
//!   `account_id → (balance_micros, expires_at)`
//!
//! `account_id` is an opaque string atlas assigns per customer. The box
//! authenticates with a rotatable api_key (see `device_keys`); the wallet is
//! keyed by the stable account, so rotating/losing a key never moves money.
//!
//! ## Source of truth: the ledger
//!
//! `accounts.balance_micros` is a fast **projection**. The append-only
//! `ledger` table is the truth: every grant (renewal), topup, charge, and
//! refund is one immutable row, and `balance == SUM(ledger.micros)`. The two
//! are always written in the same transaction; the projection is rebuildable.
//!
//! ## Charge model
//!
//! Every `charge()`:
//!   1. 20% universal markup (env `USAGE_MARKUP_BASIS_POINTS=2000`) on the
//!      real upstream cost — how Virtues makes money on usage.
//!   2. Per-call cap ($5 billed).
//!   3. Atomic decrement guarded by `expires_at > now()` AND
//!      `balance >= billed`, plus a `ledger` 'charge' row, in one transaction.
//!
//! There is **no per-day spend wall** — the Cursor-style model uses a
//! user-settable MONTHLY top-up ceiling (`customers.monthly_cap_micros`,
//! enforced atlas-side on top-up) as the only spend ceiling.
//!
//! On `InsufficientBudget` the box surfaces "wallet empty" (optionally one
//! auto-topup via atlas, which credits the wallet here).

use anyhow::{Context, Result};
use chrono::{DateTime, Datelike, Duration, Utc};
use sqlx::PgPool;

/// Per-call cost cap. Rejects pathologically large prompts. Fixed defense,
/// not user-tunable. Applied AFTER markup — `billed > PER_CALL_CAP_MICROS` 400s.
pub const PER_CALL_CAP_MICROS: i64 = 5_000_000; // $5/call billed

/// Universal markup applied to the real upstream cost before debit.
/// 2000 basis points = 20%.
pub const DEFAULT_MARKUP_BASIS_POINTS: i64 = 2_000;

/// Returns the configured markup basis points (env: `USAGE_MARKUP_BASIS_POINTS`).
pub fn markup_basis_points() -> i64 {
    std::env::var("USAGE_MARKUP_BASIS_POINTS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(DEFAULT_MARKUP_BASIS_POINTS)
}

/// Compute the billed amount from a real upstream cost. Single source of truth
/// for the markup formula.
#[inline]
pub fn apply_markup(real_micros: i64) -> i64 {
    let bp = markup_basis_points();
    real_micros.saturating_mul(10_000 + bp) / 10_000
}

/// Convert a USD float (as upstreams report it — e.g. Exa's `costDollars.total`,
/// the AI gateway's `usage.cost`) into integer micros. Shared so every paid
/// proxy resolves real cost the same way before `charge`/`settle`.
#[inline]
pub fn usd_to_micros(usd: f64) -> i64 {
    (usd * 1_000_000.0).round() as i64
}

/// Resolved account, carried by bearer-auth on every gated request.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct Account {
    pub account_id: String,
    pub balance_micros: i64,
    pub expires_at: DateTime<Utc>,
}

/// Resolve an account from a device api_key hash (the auth hot path). Joins
/// `device_keys → accounts`.
pub async fn resolve_account_by_key(
    pool: &PgPool,
    api_key_hash: &[u8],
) -> Result<Option<Account>> {
    let row = sqlx::query_as::<_, Account>(
        r#"
        SELECT a.account_id, a.balance_micros, a.expires_at
        FROM device_keys k
        JOIN accounts a ON a.account_id = k.account_id
        WHERE k.api_key_hash = $1
        "#,
    )
    .bind(api_key_hash)
    .fetch_optional(pool)
    .await
    .context("resolve account by api_key hash")?;
    Ok(row)
}

/// Look up an account by id (used by internal funding ops + tests).
pub async fn get_by_account_id(pool: &PgPool, account_id: &str) -> Result<Option<Account>> {
    let row = sqlx::query_as::<_, Account>(
        r#"
        SELECT account_id, balance_micros, expires_at
        FROM accounts
        WHERE account_id = $1
        "#,
    )
    .bind(account_id)
    .fetch_optional(pool)
    .await
    .context("select account by id")?;
    Ok(row)
}

/// Charge an account for an upstream call.
///
/// `real_cost_micros` is what the upstream provider charged us. The 20% markup
/// is applied, validated against per-call cap / daily ceiling / balance, then
/// atomically debited together with a `ledger` 'charge' row.
pub async fn charge(
    pool: &PgPool,
    account_id: &str,
    real_cost_micros: i64,
) -> Result<ChargeOk, ChargeError> {
    if real_cost_micros <= 0 {
        return Err(ChargeError::InvalidCost);
    }

    let billed = apply_markup(real_cost_micros);
    if billed > PER_CALL_CAP_MICROS {
        return Err(ChargeError::CallTooExpensive);
    }

    let mut tx = pool.begin().await.map_err(|e| ChargeError::Db(e.into()))?;

    // Atomic decrement guarded by expiry + balance. No per-day wall — the
    // Cursor-style model uses a user-settable MONTHLY top-up ceiling
    // (enforced atlas-side on top-up) as the only spend ceiling.
    let row: Option<(i64,)> = sqlx::query_as(
        r#"
        UPDATE accounts
        SET balance_micros = balance_micros - $1
        WHERE account_id = $2
          AND expires_at > now()
          AND balance_micros >= $1
        RETURNING balance_micros
        "#,
    )
    .bind(billed)
    .bind(account_id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|e| ChargeError::Db(e.into()))?;

    if row.is_some() {
        // Journal the debit (negative) in the same transaction.
        sqlx::query(
            "INSERT INTO ledger (account_id, micros, kind, real_micros) \
             VALUES ($1, $2, 'charge', $3)",
        )
        .bind(account_id)
        .bind(-billed)
        .bind(real_cost_micros)
        .execute(&mut *tx)
        .await
        .map_err(|e| ChargeError::Db(e.into()))?;

        tx.commit().await.map_err(|e| ChargeError::Db(e.into()))?;
        return Ok(ChargeOk { billed_micros: billed });
    }

    // Gate failed — nothing was written; classify why.
    tx.rollback().await.map_err(|e| ChargeError::Db(e.into()))?;
    classify_failure(pool, account_id, billed).await
}

/// Post-paid settlement for a call whose cost is only known *after* it
/// succeeds (AI: the gateway reports `usage.cost` in the response). Unlike
/// `charge()`, it debits **unconditionally** — the response already went out,
/// so we must record the true spend even if it dips the balance negative by at
/// most one call. The pre-flight budget gate (see `routes/ai.rs`) then refuses
/// the *next* call once the balance is in the red. This is what makes the
/// wallet actually enforce on the chat path; a guarded `charge()` here would
/// plateau the balance at a few cents and leak unlimited free calls. Returns
/// the new balance.
pub async fn settle(pool: &PgPool, account_id: &str, real_cost_micros: i64) -> Result<i64> {
    if real_cost_micros <= 0 {
        return Ok(0);
    }
    let billed = apply_markup(real_cost_micros);

    let mut tx = pool.begin().await?;

    // Unconditional debit — only requires the account to still exist.
    let row: Option<(i64,)> = sqlx::query_as(
        "UPDATE accounts \
         SET balance_micros = balance_micros - $1 \
         WHERE account_id = $2 \
         RETURNING balance_micros",
    )
    .bind(billed)
    .bind(account_id)
    .fetch_optional(&mut *tx)
    .await
    .context("settle debit")?;
    let Some((balance,)) = row else {
        // Account swept between the call and settlement — nothing to debit.
        tx.rollback().await?;
        return Ok(0);
    };
    sqlx::query(
        "INSERT INTO ledger (account_id, micros, kind, real_micros) \
         VALUES ($1, $2, 'charge', $3)",
    )
    .bind(account_id)
    .bind(-billed)
    .bind(real_cost_micros)
    .execute(&mut *tx)
    .await
    .context("settle ledger row")?;
    tx.commit().await?;
    Ok(balance)
}

/// On a failed debit, disambiguate why: not found / expired / insufficient
/// balance.
async fn classify_failure(
    pool: &PgPool,
    account_id: &str,
    _billed: i64,
) -> Result<ChargeOk, ChargeError> {
    let row: Option<(DateTime<Utc>, i64)> = sqlx::query_as(
        "SELECT expires_at, balance_micros FROM accounts WHERE account_id = $1",
    )
    .bind(account_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| ChargeError::Db(e.into()))?;

    match row {
        None => Err(ChargeError::NotFound),
        Some((expires_at, _)) if expires_at <= Utc::now() => Err(ChargeError::Expired),
        Some(_) => Err(ChargeError::InsufficientBudget),
    }
}

/// Refund a previously-charged amount back to the balance, with a compensating
/// `ledger` 'refund' row. Best-effort. Refunds the **billed** amount.
pub async fn refund(pool: &PgPool, account_id: &str, billed_micros: i64) -> Result<()> {
    let mut tx = pool.begin().await?;
    let updated = sqlx::query(
        r#"
        UPDATE accounts
        SET balance_micros = balance_micros + $1
        WHERE account_id = $2
        "#,
    )
    .bind(billed_micros)
    .bind(account_id)
    .execute(&mut *tx)
    .await
    .context("refund balance")?;
    // If the account vanished (e.g. swept between charge and a slow upstream
    // failure), there's nothing to refund — skip the ledger row rather than
    // orphan it (the FK would reject it anyway). Keeps balance == SUM(ledger).
    if updated.rows_affected() == 0 {
        tx.rollback().await?;
        return Ok(());
    }
    sqlx::query("INSERT INTO ledger (account_id, micros, kind) VALUES ($1, $2, 'refund')")
        .bind(account_id)
        .bind(billed_micros)
        .execute(&mut *tx)
        .await
        .context("refund ledger row")?;
    tx.commit().await?;
    Ok(())
}

/// Register (or rotate) a device api key for an account. Ensures the account
/// exists (creating an empty, cohort-expiry wallet if new), then installs the
/// given key. The balance is never touched, so re-linking re-points access
/// without moving money.
///
/// **Scoped to a BOX when `box_id` is given.** Rotation replaces that box's own
/// key and leaves every sibling alone, so one account can hold several boxes —
/// a home and an office, a replacement mid-migration, a box bought for family.
/// Passing `None` keeps the historical behaviour (replace every key on the
/// account), which is what pre-2026-08-12 keys and any caller that does not
/// know its box get.
///
/// Until now this always replaced every key on the account, which made a second
/// box silently kill the first. Nobody hit it because linking a second box was
/// laborious; it becomes one tap as soon as the app can vouch for a link, so
/// the destruction had to go first.
pub async fn register_device(
    pool: &PgPool,
    api_key_hash: &[u8],
    account_id: &str,
    box_id: Option<&str>,
) -> Result<()> {
    let now = Utc::now();
    let mut tx = pool.begin().await?;

    sqlx::query(
        r#"
        INSERT INTO accounts (account_id, balance_micros, expires_at)
        VALUES ($1, 0, $2)
        ON CONFLICT (account_id) DO NOTHING
        "#,
    )
    .bind(account_id)
    .bind(cohort_align_after(now + Duration::days(30)))
    .execute(&mut *tx)
    .await
    .context("ensure account on device register")?;

    // Drop only what this registration replaces: this BOX's previous key when we
    // know which box it is, otherwise the whole account (the legacy path).
    match box_id {
        Some(id) => {
            // `OR box_id IS NULL` is load-bearing: `=` never matches NULL, so
            // a key registered before the account went per-box (box_id NULL)
            // would otherwise survive its box's re-link forever — an immortal
            // credential that still proxies AI calls and spends the wallet
            // (review finding, 2026-08-24). The NULL row is the account's old
            // shared key; the first labeled registration retires it.
            sqlx::query(
                "DELETE FROM device_keys WHERE account_id = $1 \
                 AND (box_id = $2 OR box_id IS NULL)",
            )
            .bind(account_id)
            .bind(id)
            .execute(&mut *tx)
            .await
            .context("clear this box's prior key")?;
        }
        None => {
            sqlx::query("DELETE FROM device_keys WHERE account_id = $1")
                .bind(account_id)
                .execute(&mut *tx)
                .await
                .context("clear prior device keys")?;
        }
    }
    sqlx::query(
        "INSERT INTO device_keys (api_key_hash, account_id, box_id) VALUES ($1, $2, $3)",
    )
    .bind(api_key_hash)
    .bind(account_id)
    .bind(box_id)
    .execute(&mut *tx)
    .await
    .context("insert device key")?;

    tx.commit().await?;
    Ok(())
}

/// How a credit lands on the balance.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CreditMode {
    /// Renewal: overwrite the balance to `amount` (fresh monthly allotment)
    /// and bump expiry to the next cohort boundary.
    Set,
    /// Top-up: add `amount` to the existing balance; expiry unchanged.
    Add,
}

/// Credit an account (subscription renewal or top-up), writing the matching
/// `ledger` row so `balance == SUM(ledger)` holds. Returns the new balance.
///
/// `Set` records the **delta** that brings the balance to `amount` (so a
/// monthly overwrite that claws back unused credit stays journal-consistent).
pub async fn credit(
    pool: &PgPool,
    account_id: &str,
    amount_micros: i64,
    mode: CreditMode,
    reference: Option<&str>,
) -> Result<i64> {
    let now = Utc::now();
    let mut tx = pool.begin().await?;

    let existing: Option<(i64,)> =
        sqlx::query_as("SELECT balance_micros FROM accounts WHERE account_id = $1 FOR UPDATE")
            .bind(account_id)
            .fetch_optional(&mut *tx)
            .await
            .context("lock account for credit")?;
    let old_balance = existing.map(|(b,)| b).unwrap_or(0);

    let (new_balance, delta, kind) = match mode {
        CreditMode::Set => (amount_micros, amount_micros - old_balance, "grant"),
        CreditMode::Add => (old_balance + amount_micros, amount_micros, "topup"),
    };

    match mode {
        CreditMode::Set => {
            sqlx::query(
                r#"
                INSERT INTO accounts (account_id, balance_micros, expires_at)
                VALUES ($1, $2, $3)
                ON CONFLICT (account_id) DO UPDATE
                SET balance_micros = EXCLUDED.balance_micros,
                    expires_at = EXCLUDED.expires_at
                "#,
            )
            .bind(account_id)
            .bind(new_balance)
            .bind(cohort_align_after(now + Duration::days(30)))
            .execute(&mut *tx)
            .await
            .context("credit set")?;
        }
        CreditMode::Add => {
            sqlx::query(
                r#"
                INSERT INTO accounts (account_id, balance_micros, expires_at)
                VALUES ($1, $2, $3)
                ON CONFLICT (account_id) DO UPDATE
                SET balance_micros = accounts.balance_micros + $2
                "#,
            )
            .bind(account_id)
            .bind(amount_micros)
            .bind(cohort_align_after(now + Duration::days(30)))
            .execute(&mut *tx)
            .await
            .context("credit add")?;
        }
    }

    sqlx::query("INSERT INTO ledger (account_id, micros, kind, ref) VALUES ($1, $2, $3, $4)")
        .bind(account_id)
        .bind(delta)
        .bind(kind)
        .bind(reference)
        .execute(&mut *tx)
        .await
        .context("credit ledger row")?;

    tx.commit().await?;
    Ok(new_balance)
}

/// Balance + recent ledger entries, for the user-facing usage surface.
pub async fn usage_summary(
    pool: &PgPool,
    account_id: &str,
    limit: i64,
) -> Result<UsageSummary> {
    let acct = get_by_account_id(pool, account_id).await?;
    let Some(acct) = acct else {
        return Ok(UsageSummary::default());
    };

    // Month-to-date spend (sum of charges since the 1st, UTC).
    let month_start = month_start_utc(Utc::now());
    let (mtd,): (Option<i64>,) = sqlx::query_as(
        "SELECT -SUM(micros) FROM ledger \
         WHERE account_id = $1 AND kind = 'charge' AND ts >= $2",
    )
    .bind(account_id)
    .bind(month_start)
    .fetch_one(pool)
    .await
    .context("month-to-date spend")?;

    let entries = sqlx::query_as::<_, LedgerEntry>(
        "SELECT ts, micros, kind, real_micros FROM ledger \
         WHERE account_id = $1 ORDER BY ts DESC LIMIT $2",
    )
    .bind(account_id)
    .bind(limit)
    .fetch_all(pool)
    .await
    .context("recent ledger entries")?;

    Ok(UsageSummary {
        balance_micros: acct.balance_micros,
        month_to_date_micros: mtd.unwrap_or(0),
        expires_at: Some(acct.expires_at),
        entries,
    })
}

#[derive(Debug, Default, serde::Serialize)]
pub struct UsageSummary {
    pub balance_micros: i64,
    pub month_to_date_micros: i64,
    pub expires_at: Option<DateTime<Utc>>,
    pub entries: Vec<LedgerEntry>,
}

#[derive(Debug, serde::Serialize, sqlx::FromRow)]
pub struct LedgerEntry {
    pub ts: DateTime<Utc>,
    pub micros: i64,
    pub kind: String,
    pub real_micros: Option<i64>,
}

/// Outcome of a successful charge. The real upstream cost and the resulting
/// balance are persisted on the ledger row and the account respectively;
/// callers only need what the user was billed.
#[derive(Debug, Clone, Copy)]
pub struct ChargeOk {
    /// What the user was charged (post-markup).
    pub billed_micros: i64,
}

#[derive(Debug)]
pub enum ChargeError {
    InsufficientBudget,
    Expired,
    NotFound,
    InvalidCost,
    CallTooExpensive,
    Db(anyhow::Error),
}

impl std::fmt::Display for ChargeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InsufficientBudget => write!(f, "wallet empty — add credits"),
            Self::Expired => write!(f, "subscription wallet expired — reconnect"),
            Self::NotFound => write!(f, "account not found"),
            Self::InvalidCost => write!(f, "real_cost_micros must be > 0"),
            Self::CallTooExpensive => write!(f, "single call exceeds per-call cap"),
            Self::Db(e) => write!(f, "db error: {e}"),
        }
    }
}

/// First-of-month 00:00 UTC at or after `dt` — the cohort-aligned wallet
/// expiry boundary. (Moved from the deleted voucher module.)
pub fn cohort_align_after(dt: DateTime<Utc>) -> DateTime<Utc> {
    let d = dt.date_naive();
    let (y, m) = (d.year(), d.month());
    let (ny, nm) = if m == 12 { (y + 1, 1) } else { (y, m + 1) };
    chrono::NaiveDate::from_ymd_opt(ny, nm, 1)
        .expect("valid first-of-month")
        .and_hms_opt(0, 0, 0)
        .expect("midnight")
        .and_utc()
}

/// Start of the current calendar month, 00:00 UTC.
fn month_start_utc(now: DateTime<Utc>) -> DateTime<Utc> {
    let d = now.date_naive();
    chrono::NaiveDate::from_ymd_opt(d.year(), d.month(), 1)
        .expect("valid first-of-month")
        .and_hms_opt(0, 0, 0)
        .expect("midnight")
        .and_utc()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `charge()` debits the balance, refuses once the wallet is empty, and
    /// writes a ledger row each time, keeping `balance == SUM(ledger)`. (No
    /// daily cap — the only spend ceiling is the monthly top-up cap atlas-side.)
    #[sqlx::test]
    async fn charge_debits_and_journals(pool: PgPool) {
        // Fund $10 via a renewal credit.
        credit(&pool, "acct-low", 10_000_000, CreditMode::Set, None)
            .await
            .unwrap();

        // billed = 4_800_000 each; two land (9.6M ≤ 10M), the third has no
        // balance left and trips InsufficientBudget.
        charge(&pool, "acct-low", 4_000_000).await.expect("1st ok");
        charge(&pool, "acct-low", 4_000_000).await.expect("2nd ok");
        let third = charge(&pool, "acct-low", 4_000_000).await;
        assert!(
            matches!(third, Err(ChargeError::InsufficientBudget)),
            "third charge should exhaust the wallet, got {third:?}"
        );

        // balance projection == SUM(ledger).
        let acct = get_by_account_id(&pool, "acct-low").await.unwrap().unwrap();
        // SUM(int8) is NUMERIC in Postgres; cast so it decodes as i64.
        let (sum,): (Option<i64>,) =
            sqlx::query_as("SELECT SUM(micros)::bigint FROM ledger WHERE account_id = $1")
                .bind("acct-low")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(acct.balance_micros, sum.unwrap(), "balance must equal ledger sum");
    }

    /// Rotating the device key (recovery) preserves the balance: the same
    /// funds are chargeable under the new key.
    #[sqlx::test]
    async fn key_rotation_preserves_balance(pool: PgPool) {
        credit(&pool, "acct-r", 50_000_000, CreditMode::Set, None)
            .await
            .unwrap();
        register_device(&pool, b"keyhash-old-0000000000000000", "acct-r", None)
            .await
            .unwrap();
        charge(&pool, "acct-r", 1_000_000).await.expect("charge under old key path");

        let before = get_by_account_id(&pool, "acct-r").await.unwrap().unwrap().balance_micros;
        // Rotate to a new key for the SAME account.
        register_device(&pool, b"keyhash-new-0000000000000000", "acct-r", None)
            .await
            .unwrap();
        let resolved = resolve_account_by_key(&pool, b"keyhash-new-0000000000000000")
            .await
            .unwrap()
            .expect("new key resolves the account");
        assert_eq!(resolved.balance_micros, before, "balance preserved across rotation");
        // Old key no longer resolves.
        assert!(resolve_account_by_key(&pool, b"keyhash-old-0000000000000000")
            .await
            .unwrap()
            .is_none());
    }
}
