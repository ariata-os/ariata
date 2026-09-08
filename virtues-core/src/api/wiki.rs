//! Wiki API - Views of entities and narratives for wiki pages
//!
//! Wiki pages are not separate constructs - they are views of:
//! - Entities: Person, Place, Organization, Thing
//! - Narratives: Telos, Act, Chapter, Day

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

use crate::error::{Error, Result};
use crate::ids;


// ============================================================================
// Wiki Page Types - Entity Views
// ============================================================================

/// A person wiki page
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WikiPerson {
    pub id: String,
    pub name: String,
    pub content: Option<String>,
    /// Machine-written wikipedia-style record (entity_article applet). Never
    /// user-edited — `content`/`notes` carry the user's own writing.
    pub article: Option<String>,
    pub article_updated_at: Option<DateTime<Utc>>,
    /// Is this article being kept up to date? Off unless the user asked.
    #[serde(default)]
    pub article_auto_update: bool,
    pub picture: Option<String>,
    pub cover_image: Option<String>,
    // vCard fields
    pub emails: Vec<String>,
    pub phones: Vec<String>,
    pub birthday: Option<NaiveDate>,
    /// When they died, if they have — with the precision the person gave
    /// ("sometime in 2019" is a real answer; see migration 0016).
    pub died_on: Option<NaiveDate>,
    pub died_precision: Option<String>,
    pub instagram: Option<String>,
    pub facebook: Option<String>,
    pub linkedin: Option<String>,
    pub x: Option<String>,
    // Metadata
    pub relationship_category: Option<String>,
    pub nickname: Option<String>,
    /// The AUTHORED line about what this person means — one sentence, the
    /// owner's verbatim, never inferred. Recency says who is around; only
    /// this says who matters. Rendered above every observed stat.
    pub bond: Option<String>,
    // `notes` retired to wiki_notes (migration 0082). The column still exists —
    // drops trail by a release — but nothing reads or writes it from here, which
    // is what lets the next migration drop it safely.
    /// Surfaces this entity also answers to (0037). Read alongside write, or an
    /// editor cannot show what is already there.
    pub aliases: Vec<String>,
    pub first_seen: Option<DateTime<Utc>>,
    pub last_seen: Option<DateTime<Utc>>,
    pub seen_count: Option<i32>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// A place wiki page
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WikiPlace {
    pub id: String,
    pub name: String,
    pub content: Option<String>,
    pub article: Option<String>,
    pub article_updated_at: Option<DateTime<Utc>>,
    /// Is this article being kept up to date? Off unless the user asked.
    #[serde(default)]
    pub article_auto_update: bool,
    pub cover_image: Option<String>,
    pub category: Option<String>,
    pub address: Option<String>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub seen_count: Option<i32>,
    pub first_seen: Option<DateTime<Utc>>,
    pub last_seen: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// An organization wiki page
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WikiOrganization {
    pub id: String,
    pub name: String,
    pub content: Option<String>,
    pub article: Option<String>,
    pub article_updated_at: Option<DateTime<Utc>>,
    /// Is this article being kept up to date? Off unless the user asked.
    #[serde(default)]
    pub article_auto_update: bool,
    pub cover_image: Option<String>,
    pub organization_type: Option<String>,
    pub relationship_type: Option<String>,
    pub role_title: Option<String>,
    pub start_date: Option<NaiveDate>,
    pub end_date: Option<NaiveDate>,
    /// Surfaces this entity also answers to (0037).
    pub aliases: Vec<String>,
    pub seen_count: Option<i32>,
    pub first_seen: Option<DateTime<Utc>>,
    pub last_seen: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// A thing wiki page (catchall entity: pets, projects, concepts, etc.)
// ============================================================================
// Wiki Page Types - Narrative Views
// ============================================================================



/// A day wiki page
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WikiDay {
    pub id: String,
    pub date: NaiveDate,
    pub start_timezone: Option<String>,
    /// The day's prose, from `wiki_day_prose`. The article page is its only
    /// home — the legacy `autobiography` column was dropped in 0106.
    pub article: Option<String>,
    pub epigraph: Option<String>,
    pub last_edited_by: Option<String>,
    pub cover_image: Option<String>,
    // act_id/chapter_id are gone: the 2026-08-18 squash dropped the columns,
    // and the fields spent months serializing a permanent None to a client
    // that never read them. `try_get(...).ok()` is what let that hide —
    // it turns schema drift into silent nulls, so prefer removal over
    // tolerance when a column dies.
    pub data_quality: Option<serde_json::Value>,
    pub snapshot: Option<serde_json::Value>,
    /// Count of entities first referenced on this day
    pub new_entity_count: i64,
    /// Count of topics first seen on this day
    pub new_topic_count: i64,
    /// Morning readiness score (0-100, from overnight HRV/RHR/sleep)
    pub readiness_score: Option<i64>,
    /// JSON breakdown of readiness components
    pub readiness_details: Option<serde_json::Value>,
    /// Sleep cycles with autonomic scores, computed at query time from
    /// data_health_sleep stages + heart rate data. Not stored.
    pub sleep_cycles: Vec<ScoredSleepCycle>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// A single sleep cycle with autonomic scoring, derived from sleep stage
/// boundaries and heart rate data during the cycle window.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoredSleepCycle {
    pub start_time: String,
    pub end_time: String,
    pub dominant_stage: String,
    pub avg_hr: Option<f64>,
    pub autonomic_z: Option<f64>,
}

// ============================================================================
// List Item Types (lighter weight for lists)
// ============================================================================

/// A person list item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WikiPersonListItem {
    pub id: String,
    pub name: String,
    pub picture: Option<String>,
    pub relationship_category: Option<String>,
    pub last_seen: Option<DateTime<Utc>>,
    /// How many records mention this entity — see `REF_COUNT` in this module.
    pub ref_count: i64,
}

/// A place list item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WikiPlaceListItem {
    pub id: String,
    pub name: String,
    pub category: Option<String>,
    pub address: Option<String>,
    pub seen_count: Option<i32>,
    /// How many records mention this entity — see `REF_COUNT` in this module.
    pub ref_count: i64,
}

/// An organization list item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WikiOrganizationListItem {
    pub id: String,
    pub name: String,
    pub organization_type: Option<String>,
    pub relationship_type: Option<String>,
    /// How many records mention this entity — see `REF_COUNT` in this module.
    pub ref_count: i64,
}

// ============================================================================
// Update Request Types
// ============================================================================

/// Request to update a person wiki page
#[derive(Debug, Deserialize)]
pub struct UpdateWikiPersonRequest {
    pub name: Option<String>,
    pub content: Option<String>,
    pub picture: Option<String>,
    pub cover_image: Option<String>,
    pub emails: Option<Vec<String>>,
    pub phones: Option<Vec<String>>,
    pub birthday: Option<NaiveDate>,
    pub died_on: Option<NaiveDate>,
    pub died_precision: Option<String>,
    pub instagram: Option<String>,
    pub facebook: Option<String>,
    pub linkedin: Option<String>,
    pub x: Option<String>,
    pub relationship_category: Option<String>,
    pub nickname: Option<String>,
    /// One sentence, theirs verbatim — see WikiPerson::bond.
    pub bond: Option<String>,
    // `notes` retired to wiki_notes (migration 0082). The column still exists —
    // drops trail by a release — but nothing reads or writes it from here, which
    // is what lets the next migration drop it safely.
    /// Surfaces this entity also answers to. 0037 calls an alias "the record of
    /// a human decision" and built the column for exactly this — then nothing
    /// ever wrote it: 3 of 573 people on a real box have one. Stored
    /// lowercased; the resolver lowercases the surface before matching, so a
    /// name linked once resolves every past and future mention of it.
    pub aliases: Option<Vec<String>>,
}

/// Request to update a place wiki page
#[derive(Debug, Deserialize)]
pub struct UpdateWikiPlaceRequest {
    pub name: Option<String>,
    pub content: Option<String>,
    pub cover_image: Option<String>,
    pub category: Option<String>,
    pub address: Option<String>,
}

/// Request to update an organization wiki page
#[derive(Debug, Deserialize)]
pub struct UpdateWikiOrganizationRequest {
    pub name: Option<String>,
    pub content: Option<String>,
    pub cover_image: Option<String>,
    pub organization_type: Option<String>,
    pub relationship_type: Option<String>,
    pub role_title: Option<String>,
    pub start_date: Option<NaiveDate>,
    pub end_date: Option<NaiveDate>,
    /// Surfaces this entity also answers to. 0037 calls an alias "the record of
    /// a human decision" and built the column for exactly this — then nothing
    /// ever wrote it: 3 of 573 people on a real box have one. Stored
    /// lowercased; the resolver lowercases the surface before matching, so a
    /// name linked once resolves every past and future mention of it.
    pub aliases: Option<Vec<String>>,
}

/// Request to update a day wiki page
#[derive(Debug, Deserialize)]
pub struct UpdateWikiDayRequest {
    pub epigraph: Option<String>,
    pub last_edited_by: Option<String>,
    pub cover_image: Option<String>,
    pub start_timezone: Option<String>,
    pub data_quality: Option<serde_json::Value>,
    pub snapshot: Option<serde_json::Value>,
}

// ============================================================================
// Person CRUD Operations
// ============================================================================

/// Get a person by ID
pub async fn get_person(pool: &PgPool, id: String) -> Result<WikiPerson> {
    let row = sqlx::query!(
        r#"
        SELECT
            id, name, content, article, article_updated_at, picture, cover_image,
            emails, phones, birthday, died_on, died_precision, instagram,
            facebook, linkedin, x,
            relationship_category, nickname, bond, aliases,
            first_seen, last_seen, seen_count,
            created_at, updated_at
        FROM wiki_people
        WHERE id = $1
        "#,
        id
    )
    .fetch_optional(pool)
    .await
    .map_err(|e| Error::Database(format!("Failed to get person: {}", e)))?
    .ok_or_else(|| Error::NotFound(format!("Person not found: {}", id)))?;

    let (article, article_updated_at, auto_update) =
        overlay_article(pool, "person", &row.id, row.article, row.article_updated_at).await;

    Ok(WikiPerson {
        id: row.id,
        name: row.name,
        content: row.content,
        article,
        article_updated_at,
        article_auto_update: auto_update,
        picture: row.picture,
        cover_image: row.cover_image,
        emails: serde_json::from_value(row.emails).unwrap_or_default(),
        phones: serde_json::from_value(row.phones).unwrap_or_default(),
        aliases: serde_json::from_value(row.aliases).unwrap_or_default(),
        birthday: row.birthday,
        died_on: row.died_on,
        died_precision: row.died_precision,
        instagram: row.instagram,
        facebook: row.facebook,
        linkedin: row.linkedin,
        x: row.x,
        relationship_category: row.relationship_category,
        nickname: row.nickname,
        bond: row.bond,
        first_seen: row.first_seen,
        last_seen: row.last_seen,
        seen_count: Some(row.seen_count as i32),
        created_at: row.created_at,
        updated_at: row.updated_at,
    })
}

/// List all people

/// Overlay a subject's article from `wiki_articles` onto the legacy column.
///
/// Prose moved to `app_pages` (migration 0081), but the per-entity `article`
/// columns from 0072 are still there — drops trail their phase by a release, so
/// a box in the middle can hold prose in either place. New articles live on the
/// page; anything written before the move still lives in the column. Read the
/// page first and fall back, so neither is lost while both exist.
async fn overlay_article(
    pool: &PgPool,
    subject_type: &str,
    subject_id: &str,
    legacy: Option<String>,
    legacy_at: Option<DateTime<Utc>>,
) -> (Option<String>, Option<DateTime<Utc>>, bool) {
    match crate::api::wiki_articles::get_article_prose(pool, subject_type, subject_id).await {
        Ok(Some(a)) => (Some(a.content), Some(a.updated_at), a.auto_update),
        // A read failure must not take the whole entity page down with it — the
        // records below the article are the more important half.
        Ok(None) => (legacy, legacy_at, false),
        Err(e) => {
            tracing::warn!(subject_id, error = %e, "article read failed; showing legacy column");
            (legacy, legacy_at, false)
        }
    }
}

/// Aliases are stored lowercased, trimmed, deduped, and never empty.
///
/// 0037 stores them lowercased and matches with `aliases ? lower(surface)`, so
/// a mixed-case alias is simply invisible to the resolver — it would look
/// saved and never resolve anything. Normalizing on the way in is the only
/// place that can be enforced once for every caller.
fn normalize_aliases(input: Option<&Vec<String>>) -> Option<serde_json::Value> {
    let list = input?;
    let mut seen: Vec<String> = Vec::with_capacity(list.len());
    for raw in list {
        let a = raw.trim().to_lowercase();
        if !a.is_empty() && !seen.contains(&a) {
            seen.push(a);
        }
    }
    Some(serde_json::json!(seen))
}

/// Entity indexes sort by how many records mention the entity, not by name.
///
/// The People index had no order at all: it sorted by `name`, and the
/// column that was supposed to carry importance — `seen_count` — is 0 on
/// every row on a real box, because nothing has ever written it. So an address
/// book of 573 contacts arrived alphabetically, with `no-reply@slack.com` sitting
/// level with the people you actually talk to.
///
/// The signal was always there: `wiki_refs` holds 130k message refs
/// across 314 people. Counting them sorts the wall on its own — people you
/// message rise, contacts with no traffic sink, transactional senders land at
/// the bottom with two email refs each. No classifier, no model, no deletion:
/// the noise does not need removing, it needs ordering.
///
/// **Computed per query, not materialized.** The obvious move is a counter
/// column, but that needs a refresh path and can drift, and this is a sort key
/// rather than a fact. Measured on the real corpus (131k refs, 573 people) the
/// aggregate runs in 11 ms, which is cheaper than being wrong. Materialize it
/// when the index gets slow, and not before.
///
/// Deliberately NOT `seen_count` or `wiki_places.seen_count`: those are
/// two different quantities on two tables (and visits are not refs), so reusing
/// either would make "the default sort" mean something different per index.
/// Both are legacy; this is the one uniform measure.
pub async fn list_people(pool: &PgPool) -> Result<Vec<WikiPersonListItem>> {
    let rows = sqlx::query!(
        r#"
        SELECT
            p.id, p.name, p.picture, p.relationship_category, p.last_seen,
            COALESCE(r.n, 0) AS "ref_count!"
        FROM wiki_people p
        LEFT JOIN (
            SELECT entity_id, count(*) AS n
            FROM wiki_refs WHERE entity_type = 'person' GROUP BY entity_id
        ) r ON r.entity_id = p.id
        ORDER BY COALESCE(r.n, 0) DESC, p.name ASC
        "#
    )
    .fetch_all(pool)
    .await
    .map_err(|e| Error::Database(format!("Failed to list people: {}", e)))?;

    Ok(rows
        .into_iter()
        .map(|row| WikiPersonListItem {
            id: row.id,
            name: row.name,
            picture: row.picture,
            relationship_category: row.relationship_category,
            last_seen: row.last_seen,
            ref_count: row.ref_count,
        })
        .collect())
}

/// Update a person
pub async fn update_person(
    pool: &PgPool,
    id: String,
    req: UpdateWikiPersonRequest,
) -> Result<WikiPerson> {
    let emails_json: Option<serde_json::Value> = req.emails.as_ref().map(|e| serde_json::json!(e));
    let phones_json: Option<serde_json::Value> = req.phones.as_ref().map(|p| serde_json::json!(p));
    let aliases_json = normalize_aliases(req.aliases.as_ref());

    sqlx::query!(
        r#"
        UPDATE wiki_people
        SET
            name = COALESCE($2, name),
            content = COALESCE($3, content),
            picture = COALESCE($4, picture),
            cover_image = COALESCE($5, cover_image),
            emails = COALESCE($6, emails),
            phones = COALESCE($7, phones),
            birthday = COALESCE($8, birthday),
            died_on = COALESCE($16, died_on),
            died_precision = COALESCE($17, died_precision),
            bond = COALESCE($18, bond),
            instagram = COALESCE($9, instagram),
            facebook = COALESCE($10, facebook),
            linkedin = COALESCE($11, linkedin),
            x = COALESCE($12, x),
            relationship_category = COALESCE($13, relationship_category),
            nickname = COALESCE($14, nickname),
            aliases = COALESCE($15, aliases),
            updated_at = now()
        WHERE id = $1
        "#,
        id,
        req.name,
        req.content,
        req.picture,
        req.cover_image,
        emails_json,
        phones_json,
        req.birthday,
        req.instagram,
        req.facebook,
        req.linkedin,
        req.x,
        req.relationship_category,
        req.nickname,
        aliases_json,
        req.died_on,
        req.died_precision,
        req.bond
    )
    .execute(pool)
    .await
    .map_err(|e| Error::Database(format!("Failed to update person: {}", e)))?;

    get_person(pool, id).await
}

// ============================================================================
// Place CRUD Operations
// ============================================================================

/// Get a place by ID (wiki view with content fields)
pub async fn get_wiki_place(pool: &PgPool, id: String) -> Result<WikiPlace> {
    let row = sqlx::query!(
        r#"
        SELECT
            id, name, content, article, article_updated_at, cover_image, category, address,
            latitude, longitude,
            seen_count, first_seen, last_seen,
            created_at, updated_at
        FROM wiki_places
        WHERE id = $1
        "#,
        id
    )
    .fetch_optional(pool)
    .await
    .map_err(|e| Error::Database(format!("Failed to get place: {}", e)))?
    .ok_or_else(|| Error::NotFound(format!("Place not found: {}", id)))?;

    let (article, article_updated_at, auto_update) =
        overlay_article(pool, "place", &row.id, row.article.clone(), row.article_updated_at).await;

    Ok(WikiPlace {
        id: row.id,
        name: row.name.clone(),
        content: row.content.clone(),
        article,
        article_updated_at,
        article_auto_update: auto_update,
        cover_image: row.cover_image.clone(),
        category: row.category.clone(),
        address: row.address.clone(),
        latitude: row.latitude,
        longitude: row.longitude,
        seen_count: Some(row.seen_count as i32),
        first_seen: row.first_seen,
        last_seen: row.last_seen,
        created_at: row.created_at,
        updated_at: row.updated_at,
    })
}

/// List all places (wiki view with content fields)
pub async fn list_wiki_places(pool: &PgPool) -> Result<Vec<WikiPlaceListItem>> {
    let rows = sqlx::query!(
        r#"
        SELECT
            p.id, p.name, p.category, p.address, p.seen_count,
            COALESCE(r.n, 0) AS "ref_count!"
        FROM wiki_places p
        LEFT JOIN (
            SELECT entity_id, count(*) AS n
            FROM wiki_refs WHERE entity_type = 'place' GROUP BY entity_id
        ) r ON r.entity_id = p.id
        ORDER BY COALESCE(r.n, 0) DESC, p.name ASC
        "#
    )
    .fetch_all(pool)
    .await
    .map_err(|e| Error::Database(format!("Failed to list places: {}", e)))?;

    Ok(rows
        .into_iter()
        .map(|row| WikiPlaceListItem {
            id: row.id,
            name: row.name,
            category: row.category,
            address: row.address,
            seen_count: Some(row.seen_count as i32),
            ref_count: row.ref_count,
        })
        .collect())
}

/// Update a place wiki content
pub async fn update_wiki_place(
    pool: &PgPool,
    id: String,
    req: UpdateWikiPlaceRequest,
) -> Result<WikiPlace> {
    sqlx::query!(
        r#"
        UPDATE wiki_places
        SET
            name = COALESCE($2, name),
            content = COALESCE($3, content),
            cover_image = COALESCE($4, cover_image),
            category = COALESCE($5, category),
            address = COALESCE($6, address),
            updated_at = now()
        WHERE id = $1
        "#,
        id,
        req.name,
        req.content,
        req.cover_image,
        req.category,
        req.address
    )
    .execute(pool)
    .await
    .map_err(|e| Error::Database(format!("Failed to update place: {}", e)))?;

    get_wiki_place(pool, id).await
}

// ============================================================================
// Organization CRUD Operations
// ============================================================================

/// Get an organization by ID
pub async fn get_organization(pool: &PgPool, id: String) -> Result<WikiOrganization> {
    let row = sqlx::query!(
        r#"
        SELECT
            id, name, content, article, article_updated_at, cover_image,
            organization_type, relationship_type, role_title, aliases,
            start_date, end_date, seen_count,
            first_seen, last_seen,
            created_at, updated_at
        FROM wiki_orgs
        WHERE id = $1
        "#,
        id
    )
    .fetch_optional(pool)
    .await
    .map_err(|e| Error::Database(format!("Failed to get organization: {}", e)))?
    .ok_or_else(|| Error::NotFound(format!("Organization not found: {}", id)))?;

    let (article, article_updated_at, auto_update) =
        overlay_article(pool, "organization", &row.id, row.article, row.article_updated_at).await;

    Ok(WikiOrganization {
        id: row.id,
        name: row.name,
        content: row.content,
        article,
        article_updated_at,
        article_auto_update: auto_update,
        cover_image: row.cover_image,
        organization_type: row.organization_type,
        relationship_type: row.relationship_type,
        role_title: row.role_title,
        start_date: row.start_date,
        end_date: row.end_date,
        aliases: serde_json::from_value(row.aliases).unwrap_or_default(),
        seen_count: Some(row.seen_count as i32),
        first_seen: row.first_seen,
        last_seen: row.last_seen,
        created_at: row.created_at,
        updated_at: row.updated_at,
    })
}

/// List all organizations
pub async fn list_organizations(pool: &PgPool) -> Result<Vec<WikiOrganizationListItem>> {
    let rows = sqlx::query!(
        r#"
        SELECT
            o.id, o.name, o.organization_type, o.relationship_type,
            COALESCE(r.n, 0) AS "ref_count!"
        FROM wiki_orgs o
        LEFT JOIN (
            SELECT entity_id, count(*) AS n
            FROM wiki_refs WHERE entity_type = 'organization' GROUP BY entity_id
        ) r ON r.entity_id = o.id
        ORDER BY COALESCE(r.n, 0) DESC, o.name ASC
        "#
    )
    .fetch_all(pool)
    .await
    .map_err(|e| Error::Database(format!("Failed to list organizations: {}", e)))?;

    Ok(rows
        .into_iter()
        .map(|row| WikiOrganizationListItem {
            id: row.id,
            name: row.name,
            organization_type: row.organization_type,
            relationship_type: row.relationship_type,
            ref_count: row.ref_count,
        })
        .collect())
}

/// Update an organization
pub async fn update_organization(
    pool: &PgPool,
    id: String,
    req: UpdateWikiOrganizationRequest,
) -> Result<WikiOrganization> {
    sqlx::query!(
        r#"
        UPDATE wiki_orgs
        SET
            name = COALESCE($2, name),
            content = COALESCE($3, content),
            cover_image = COALESCE($4, cover_image),
            organization_type = COALESCE($5, organization_type),
            relationship_type = COALESCE($6, relationship_type),
            role_title = COALESCE($7, role_title),
            start_date = COALESCE($8, start_date),
            end_date = COALESCE($9, end_date),
            aliases = COALESCE($10, aliases),
            updated_at = now()
        WHERE id = $1
        "#,
        id,
        req.name,
        req.content,
        req.cover_image,
        req.organization_type,
        req.relationship_type,
        req.role_title,
        req.start_date,
        req.end_date,
        normalize_aliases(req.aliases.as_ref())
    )
    .execute(pool)
    .await
    .map_err(|e| Error::Database(format!("Failed to update organization: {}", e)))?;

    get_organization(pool, id).await
}

// ============================================================================
// Narrative Identity
// ============================================================================

/// The user's narrative identity — the "In your own words" DOCUMENT, read
/// straight from its wiki article's page. There is no separate stored copy
/// (the abridged capsule was retired 2026-09-01): editing happens on the page
/// itself, which is why this view is read-only and carries the page id.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NarrativeIdentity {
    pub content: String,
    pub updated_at: DateTime<Utc>,
    /// The document's page — where editing happens. Empty until the interview
    /// has been written up.
    pub page_id: String,
}

/// Get the narrative identity. Before the interview is written up there is no
/// article yet, so we return an empty placeholder (content = "") rather than
/// 500ing — the clients treat empty content as "not authored yet".
pub async fn get_narrative_identity(pool: &PgPool) -> Result<NarrativeIdentity> {
    let article = crate::api::wiki_articles::get_article(
        pool,
        "narrative_identity",
        crate::api::narrative_draft::NAR_IDENTITY_ID,
    )
    .await?;
    let Some(article) = article else {
        return Ok(NarrativeIdentity {
            content: String::new(),
            updated_at: Utc::now(),
            page_id: String::new(),
        });
    };
    let prose = crate::api::wiki_articles::get_article_prose(
        pool,
        "narrative_identity",
        crate::api::narrative_draft::NAR_IDENTITY_ID,
    )
    .await?;
    Ok(match prose {
        Some(p) => NarrativeIdentity {
            content: p.content,
            updated_at: p.updated_at,
            page_id: article.page_id,
        },
        None => NarrativeIdentity {
            content: String::new(),
            updated_at: Utc::now(),
            page_id: article.page_id,
        },
    })
}

// ============================================================================
// Telos CRUD Operations
// ============================================================================



// ============================================================================
// Act CRUD Operations
// ============================================================================



// ============================================================================
// Chapter CRUD Operations
// ============================================================================



// ============================================================================
// Day CRUD Operations
// ============================================================================

/// Get a day by date (creates if not exists)
pub async fn get_or_create_day(pool: &PgPool, date: NaiveDate) -> Result<WikiDay> {
    let date_str = date.format("%Y-%m-%d").to_string();

    // Try to get existing day
    let existing: Option<sqlx::postgres::PgRow> = sqlx::query(
        r#"
        SELECT
            id, date, start_timezone,
            (SELECT dp.prose FROM wiki_day_prose dp WHERE dp.day_id = wiki_days.id) AS article,
            epigraph,
            last_edited_by, cover_image,
            data_quality, snapshot, readiness_score, readiness_details, created_at, updated_at
        FROM wiki_days
        WHERE date = $1
        "#,
    )
    .bind(date)
    .fetch_optional(pool)
    .await
    .map_err(|e| Error::Database(format!("Failed to get day: {}", e)))?;

    if let Some(row) = existing {
        let (ne, nt) = get_day_novelty_counts(pool, &date_str).await?;
        let mut day = wiki_day_from_row_with_counts(&row, date, ne, nt)?;
        day.sleep_cycles = compute_sleep_cycles(pool, date).await;
        return Ok(day);
    }

    // Create new day
    let day_id = ids::generate_id(ids::WIKI_DAY_PREFIX, &[&date_str]);
    let row: sqlx::postgres::PgRow = sqlx::query(
        r#"
        INSERT INTO wiki_days (id, date)
        VALUES ($1, $2)
        RETURNING
            id, date, start_timezone,
            epigraph,
            last_edited_by, cover_image,
            data_quality, snapshot, readiness_score, readiness_details, created_at, updated_at
        "#,
    )
    .bind(&day_id)
    .bind(date)
    .fetch_one(pool)
    .await
    .map_err(|e| Error::Database(format!("Failed to create day: {}", e)))?;

    wiki_day_from_row(&row, date)
}

/// Parse a WikiDay from a raw `PgRow`
fn wiki_day_from_row(row: &sqlx::postgres::PgRow, date: NaiveDate) -> Result<WikiDay> {
    wiki_day_from_row_with_counts(row, date, 0, 0)
}

fn wiki_day_from_row_with_counts(row: &sqlx::postgres::PgRow, date: NaiveDate, new_entity_count: i64, new_topic_count: i64) -> Result<WikiDay> {
    use sqlx::Row;

    let id: String = row
        .try_get("id")
        .map_err(|e| Error::Database(format!("Missing day ID: {e}")))?;
    Ok(WikiDay {
        id,
        date,
        start_timezone: row.try_get("start_timezone").ok().flatten(),
        // Absent from the INSERT..RETURNING path (a just-created day has no
        // prose anyway) — `.ok()` makes that read as None rather than an error.
        article: row.try_get("article").ok().flatten(),
        epigraph: row.try_get("epigraph").ok().flatten(),
        last_edited_by: row.try_get("last_edited_by").ok().flatten(),
        cover_image: row.try_get("cover_image").ok().flatten(),
        data_quality: row.try_get("data_quality").ok().flatten(),
        snapshot: row.try_get("snapshot").ok().flatten(),
        new_entity_count,
        new_topic_count,
        readiness_score: row.try_get::<Option<i32>, _>("readiness_score").ok().flatten().map(|v| v as i64),
        readiness_details: row.try_get("readiness_details").ok().flatten(),
        sleep_cycles: vec![], // populated after construction
        created_at: row.try_get("created_at").unwrap_or_else(|_| Utc::now()),
        updated_at: row.try_get("updated_at").unwrap_or_else(|_| Utc::now()),
    })
}

/// Compute scored sleep cycles for a day from sleep stage data + heart rate readings.
/// Derives cycle boundaries by splitting sleep_stages at "awake" entries,
/// then computes avg HR per cycle and z-scores against a 14-day sleep HR baseline.
async fn compute_sleep_cycles(pool: &PgPool, date: NaiveDate) -> Vec<ScoredSleepCycle> {
    use sqlx::Row;

    let start = date.and_hms_opt(0, 0, 0).unwrap().and_utc();
    let end = (date + chrono::Duration::days(1)).and_hms_opt(0, 0, 0).unwrap().and_utc();

    // 1. Get sleep record for this night (overlaps with this calendar day)
    let sleep_row: Option<sqlx::postgres::PgRow> = sqlx::query(
        r#"SELECT sleep_stages FROM data_health_sleep
           WHERE started_at >= $1
             AND started_at < $2
           ORDER BY started_at ASC LIMIT 1"#,
    )
    .bind(start)
    .bind(end)
    .fetch_optional(pool)
    .await
    .ok()
    .flatten();

    // sleep_stages is JSONB in pg — sqlx decodes directly into serde_json::Value.
    let stages: Vec<serde_json::Value> = match sleep_row {
        Some(row) => match row.try_get::<Option<serde_json::Value>, _>("sleep_stages") {
            Ok(Some(serde_json::Value::Array(arr))) => arr,
            _ => return vec![],
        },
        None => return vec![],
    };

    // Group consecutive non-awake stages into cycles
    let mut cycles: Vec<(String, String, String)> = vec![]; // (start, end, dominant_stage)
    let mut cycle_start: Option<String> = None;
    let mut cycle_end: Option<String> = None;
    let mut stage_durations: std::collections::HashMap<String, i64> = std::collections::HashMap::new();

    for stage in &stages {
        let stage_name = stage["stage"].as_str().unwrap_or("unknown");
        let start = stage["start"].as_str().unwrap_or("");
        let end = stage["end"].as_str().unwrap_or("");

        if stage_name == "awake" {
            // Close current cycle if we have one
            if let (Some(cs), Some(ce)) = (&cycle_start, &cycle_end) {
                let dominant = stage_durations
                    .iter()
                    .max_by_key(|(_, v)| *v)
                    .map(|(k, _)| k.clone())
                    .unwrap_or_else(|| "core".to_string());
                cycles.push((cs.clone(), ce.clone(), dominant));
                cycle_start = None;
                cycle_end = None;
                stage_durations.clear();
            }
        } else {
            if cycle_start.is_none() {
                cycle_start = Some(start.to_string());
            }
            cycle_end = Some(end.to_string());

            // Estimate duration in minutes for dominant stage calculation
            if let (Ok(s), Ok(e)) = (
                DateTime::parse_from_rfc3339(start),
                DateTime::parse_from_rfc3339(end),
            ) {
                let mins = (e - s).num_minutes();
                let key = stage_name.replace("asleep_", "");
                *stage_durations.entry(key).or_insert(0) += mins;
            }
        }
    }
    // Close final cycle
    if let (Some(cs), Some(ce)) = (&cycle_start, &cycle_end) {
        let dominant = stage_durations
            .iter()
            .max_by_key(|(_, v)| *v)
            .map(|(k, _)| k.clone())
            .unwrap_or_else(|| "core".to_string());
        cycles.push((cs.clone(), ce.clone(), dominant));
    }

    if cycles.is_empty() {
        return vec![];
    }

    // 3. Get 14-day sleep HR baseline (median of nightly avg HRs)
    let baseline_start = (date - chrono::Duration::days(14))
        .and_hms_opt(0, 0, 0).unwrap().and_utc();
    let baseline_hrs: Vec<f64> = sqlx::query_scalar(
        r#"SELECT AVG(CAST(hr.bpm AS REAL))
           FROM data_health_heart_rate hr
           INNER JOIN data_health_sleep s
             ON hr.occurred_at >= s.started_at AND hr.occurred_at < s.ended_at
           WHERE s.started_at >= $1
             AND s.started_at < $2
           GROUP BY s.id"#,
    )
    .bind(baseline_start)
    .bind(end)
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    let (baseline_mean, baseline_std) = if baseline_hrs.len() >= 2 {
        let mean = baseline_hrs.iter().sum::<f64>() / baseline_hrs.len() as f64;
        let variance =
            baseline_hrs.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / baseline_hrs.len() as f64;
        let std = variance.sqrt().max(1.0); // floor at 1 bpm to avoid div-by-zero
        (mean, std)
    } else {
        (0.0, 0.0) // insufficient baseline
    };

    // 4. Score each cycle
    let mut scored: Vec<ScoredSleepCycle> = vec![];
    for (start, end, dominant) in &cycles {
        // Get avg HR during this cycle window
        let avg_hr: Option<f64> = sqlx::query_scalar(
            r#"SELECT AVG(CAST(bpm AS REAL))
               FROM data_health_heart_rate
               WHERE occurred_at >= $1 AND occurred_at < $2"#,
        )
        .bind(start)
        .bind(end)
        .fetch_optional(pool)
        .await
        .ok()
        .flatten();

        let autonomic_z = match (avg_hr, baseline_std > 0.0) {
            (Some(hr), true) => {
                // For sleep: lower HR = better recovery = more negative z
                let z = (hr - baseline_mean) / baseline_std;
                Some(z.clamp(-3.0, 3.0))
            }
            _ => None,
        };

        scored.push(ScoredSleepCycle {
            start_time: start.clone(),
            end_time: end.clone(),
            dominant_stage: dominant.clone(),
            avg_hr,
            autonomic_z,
        });
    }

    scored
}

/// Count new entities and new topics for a date.
/// "New entity" = an entity whose earliest wiki_refs.timestamp falls on this date.
/// "New topic" = a topic in search_topic_cache whose created_at falls on this date.
async fn get_day_novelty_counts(pool: &PgPool, date_str: &str) -> Result<(i64, i64)> {
    // New entities: count distinct entity_ids where their earliest ref timestamp is on this date
    let next_date = chrono::NaiveDate::parse_from_str(date_str, "%Y-%m-%d")
        .map(|d| (d + chrono::Duration::days(1)).format("%Y-%m-%d").to_string())
        .unwrap_or_default();
    let new_entities: i64 = sqlx::query_scalar(
        r#"SELECT COUNT(DISTINCT r.entity_id)
           FROM wiki_refs r
           WHERE r.occurred_at >= ($1 || 'T00:00:00Z')::timestamptz
             AND r.occurred_at < ($2 || 'T00:00:00Z')::timestamptz
             AND NOT EXISTS (
               SELECT 1 FROM wiki_refs r2
               WHERE r2.entity_id = r.entity_id
                 AND r2.occurred_at < ($1 || 'T00:00:00Z')::timestamptz
             )"#,
    )
    .bind(date_str)
    .bind(&next_date)
    .fetch_one(pool)
    .await
    .unwrap_or(0);

    // New topics: count topics from this day's events that don't appear in prior days
    let new_topics: i64 = sqlx::query_scalar(
        r#"SELECT COUNT(DISTINCT jt)
           FROM wiki_events e, jsonb_array_elements_text(e.topics) jt
           WHERE e.day_id = 'day_' || $1
             AND jt != 'sleep'
             AND NOT EXISTS (
               SELECT 1 FROM wiki_events e2, jsonb_array_elements_text(e2.topics) jt2
               WHERE e2.day_id != e.day_id
                 AND e2.started_at < e.started_at
                 AND jt2 = jt
             )"#,
    )
    .bind(date_str)
    .fetch_one(pool)
    .await
    .unwrap_or(0);

    Ok((new_entities, new_topics))
}

/// Update a day
pub async fn update_day(
    pool: &PgPool,
    date: NaiveDate,
    req: UpdateWikiDayRequest,
) -> Result<WikiDay> {
    // Get or create the day first
    let day = get_or_create_day(pool, date).await?;
    let day_id_str = day.id.to_string();

    sqlx::query(
        r#"
        UPDATE wiki_days
        SET
            epigraph = COALESCE($2, epigraph),
            last_edited_by = COALESCE($3, last_edited_by),
            cover_image = COALESCE($4, cover_image),
            start_timezone = COALESCE($5, start_timezone),
            data_quality = COALESCE($6, data_quality),
            snapshot = COALESCE($7, snapshot),
            updated_at = now()
        WHERE id = $1
        "#,
    )
    .bind(&day_id_str)
    .bind(&req.epigraph)
    .bind(&req.last_edited_by)
    .bind(&req.cover_image)
    .bind(&req.start_timezone)
    .bind(&req.data_quality)
    .bind(&req.snapshot)
    .execute(pool)
    .await
    .map_err(|e| Error::Database(format!("Failed to update day: {}", e)))?;

    get_or_create_day(pool, date).await
}

/// List days in a date range
pub async fn list_days(
    pool: &PgPool,
    start_date: NaiveDate,
    end_date: NaiveDate,
) -> Result<Vec<WikiDay>> {
    use sqlx::Row;

    let rows: Vec<sqlx::postgres::PgRow> = sqlx::query(
        r#"
        SELECT
            id, date, start_timezone,
            (SELECT dp.prose FROM wiki_day_prose dp WHERE dp.day_id = wiki_days.id) AS article,
            epigraph,
            last_edited_by, cover_image,
            data_quality, snapshot, readiness_score, readiness_details, created_at, updated_at
        FROM wiki_days
        WHERE date >= $1 AND date <= $2
        ORDER BY date DESC
        "#,
    )
    .bind(start_date)
    .bind(end_date)
    .fetch_all(pool)
    .await
    .map_err(|e| Error::Database(format!("Failed to list days: {}", e)))?;

    rows.iter()
        .map(|row| {
            // `date` is a Postgres DATE — decode it as NaiveDate. (This used
            // to try_get::<String> inside a filter_map, which failed to decode
            // on every row and silently returned an empty list.)
            let date: NaiveDate = row
                .try_get("date")
                .map_err(|e| Error::Database(format!("Failed to decode day date: {}", e)))?;
            wiki_day_from_row(row, date)
        })
        .collect()
}

/// One day of the wiki activity calendar: how much recorded life the day
/// holds. Event count is the honest signal — it exists as soon as the day is
/// segmented, independent of whether the nightly narration has run yet.
#[derive(Debug, Serialize)]
pub struct DayActivity {
    pub date: NaiveDate,
    pub event_count: i64,
    pub narrated: bool,
}

/// Per-day activity for a date range, for the wiki's calendar heatmap.
/// Deliberately tiny — the full `WikiDay` list is heavyweight (narration
/// text, snapshots) and this gets called for a year at a time.
pub async fn day_activity(
    pool: &PgPool,
    start_date: NaiveDate,
    end_date: NaiveDate,
) -> Result<Vec<DayActivity>> {
    use sqlx::Row;

    let rows = sqlx::query(
        r#"
        SELECT
            d.date,
            COUNT(e.id) FILTER (WHERE e.user_hidden = false) AS event_count,
            (d.narrated_at IS NOT NULL) AS narrated
        FROM wiki_days d
        LEFT JOIN wiki_events e ON e.day_id = d.id
        WHERE d.date >= $1 AND d.date <= $2
        GROUP BY d.id, d.date, d.narrated_at
        ORDER BY d.date
        "#,
    )
    .bind(start_date)
    .bind(end_date)
    .fetch_all(pool)
    .await
    .map_err(|e| Error::Database(format!("Failed to load day activity: {}", e)))?;

    rows.iter()
        .map(|row| {
            Ok(DayActivity {
                date: row
                    .try_get("date")
                    .map_err(|e| Error::Database(format!("Failed to decode date: {}", e)))?,
                event_count: row
                    .try_get("event_count")
                    .map_err(|e| Error::Database(format!("Failed to decode event_count: {}", e)))?,
                narrated: row
                    .try_get("narrated")
                    .map_err(|e| Error::Database(format!("Failed to decode narrated: {}", e)))?,
            })
        })
        .collect()
}


/// A past year's entry for the same calendar date — the wiki front page's
/// "on this day" register.
#[derive(Debug, Serialize)]
pub struct OnThisDayEntry {
    pub date: NaiveDate,
    pub epigraph: Option<String>,
    pub narrated: bool,
    pub event_count: i64,
}

/// Days from earlier years sharing `date`'s month and day, newest first.
pub async fn on_this_day(pool: &PgPool, date: NaiveDate) -> Result<Vec<OnThisDayEntry>> {
    use sqlx::Row;

    let rows = sqlx::query(
        r#"
        SELECT
            d.date,
            d.epigraph,
            (d.narrated_at IS NOT NULL) AS narrated,
            COUNT(e.id) FILTER (WHERE e.user_hidden = false) AS event_count
        FROM wiki_days d
        LEFT JOIN wiki_events e ON e.day_id = d.id
        WHERE EXTRACT(MONTH FROM d.date) = $1
          AND EXTRACT(DAY FROM d.date) = $2
          AND d.date < $3
        GROUP BY d.id, d.date, d.epigraph, d.narrated_at
        ORDER BY d.date DESC
        "#,
    )
    .bind(chrono::Datelike::month(&date) as i32)
    .bind(chrono::Datelike::day(&date) as i32)
    .bind(date)
    .fetch_all(pool)
    .await
    .map_err(|e| Error::Database(format!("Failed to load on-this-day: {}", e)))?;

    rows.iter()
        .map(|row| {
            Ok(OnThisDayEntry {
                date: row
                    .try_get("date")
                    .map_err(|e| Error::Database(format!("Failed to decode date: {}", e)))?,
                epigraph: row
                    .try_get("epigraph")
                    .map_err(|e| Error::Database(format!("Failed to decode epigraph: {}", e)))?,
                narrated: row
                    .try_get("narrated")
                    .map_err(|e| Error::Database(format!("Failed to decode narrated: {}", e)))?,
                event_count: row
                    .try_get("event_count")
                    .map_err(|e| Error::Database(format!("Failed to decode event_count: {}", e)))?,
            })
        })
        .collect()
}


// ============================================================================
// ID Resolution - Parse entity type from ID
// ============================================================================

/// Result of resolving an ID to its entity type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdResolution {
    pub entity_type: String,
    pub id: String,
}

/// Resolve an entity ID to find its type
/// IDs follow the format: {type}_{hash} (e.g., person_abc123, place_xyz789)
/// For days, the ID format is: day_{YYYY-MM-DD}
pub fn resolve_id(id: &str) -> Result<IdResolution> {
    // Parse the prefix from the ID
    let parts: Vec<&str> = id.splitn(2, '_').collect();
    if parts.len() != 2 {
        return Err(Error::NotFound(format!(
            "Invalid entity ID format: {}",
            id
        )));
    }

    let entity_type = parts[0];

    // Validate known entity types
    let valid_types = ["person", "place", "org", "day", "telos", "act", "chapter", "page", "chat", "year", "source"];
    if !valid_types.contains(&entity_type) {
        return Err(Error::NotFound(format!(
            "Unknown entity type in ID: {}",
            id
        )));
    }

    Ok(IdResolution {
        entity_type: entity_type.to_string(),
        id: id.to_string(),
    })
}

// ============================================================================
// Temporal Event Types
// ============================================================================

/// A temporal event in a day timeline
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemporalEvent {
    pub id: String,
    pub day_id: String,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub auto_label: Option<String>,
    pub auto_location: Option<String>,
    pub user_label: Option<String>,
    pub user_location: Option<String>,
    pub user_notes: Option<String>,
    pub source_ontologies: Option<serde_json::Value>,
    pub is_unknown: Option<bool>,
    pub is_transit: Option<bool>,
    pub is_user_added: Option<bool>,
    pub is_user_edited: Option<bool>,
    // Dayline fields
    pub novelty_z: Option<f64>,
    pub avg_hr: Option<f64>,
    pub autonomic_z: Option<f64>,
    pub hr_z: Option<f64>,
    pub topics: Option<serde_json::Value>,
    pub event_summary: Option<String>,
    pub agent_action: Option<String>,
    pub is_sleep: Option<bool>,
    pub user_hidden: Option<bool>,
    // Entity/topic novelty
    pub entities: Option<serde_json::Value>,
    pub topic_novelty: Option<serde_json::Value>,
    pub entity_novelty: Option<serde_json::Value>,
    /// Map of entity_id → ISO8601 timestamp of earliest ref within event window.
    /// Sourced from wiki_refs. Used to position entity dots at their actual
    /// moment (not event center).
    pub entity_timestamps: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Request to create a temporal event
#[derive(Debug, Deserialize)]
pub struct CreateTemporalEventRequest {
    pub day_id: String,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub auto_label: Option<String>,
    pub auto_location: Option<String>,
    pub user_label: Option<String>,
    pub user_location: Option<String>,
    pub user_notes: Option<String>,
    pub source_ontologies: Option<serde_json::Value>,
    pub is_unknown: Option<bool>,
    pub is_transit: Option<bool>,
    pub is_user_added: Option<bool>,
    /// 1-3 sentence factual description of the event. Renders in the day page
    /// timeline as the expandable detail under the label. Optional.
    pub event_summary: Option<String>,
    /// Topical tags emitted by the segmenting LLM. Written on INSERT rather
    /// than a follow-up UPDATE, because this row is about to be read by
    /// `topic_entity_novelty` — which, until topics were emitted at all, scored
    /// an empty array on every cron-generated event.
    pub topics: Option<serde_json::Value>,
}

/// Request to update a temporal event
#[derive(Debug, Deserialize)]
pub struct UpdateTemporalEventRequest {
    pub user_label: Option<String>,
    pub user_location: Option<String>,
    pub user_notes: Option<String>,
}

// ============================================================================
// Temporal Event CRUD Operations
// ============================================================================

/// Get events for a day
pub async fn get_day_events(pool: &PgPool, day_id: String) -> Result<Vec<TemporalEvent>> {
    use sqlx::Row;
    use std::collections::HashMap;

    let rows: Vec<sqlx::postgres::PgRow> = sqlx::query(
        r#"
        SELECT
            id, day_id, started_at, ended_at,
            auto_label, auto_location, user_label, user_location, user_notes,
            source_ontologies, is_unknown, is_transit, is_user_added, is_user_edited,
            novelty_z, avg_hr, autonomic_z, hr_z,
            topics, event_summary, agent_action,
            is_sleep, user_hidden,
            entities, topic_novelty, entity_novelty,
            created_at, updated_at
        FROM wiki_events
        WHERE day_id = $1
        ORDER BY started_at ASC
        "#,
    )
    .bind(&day_id)
    .fetch_all(pool)
    .await
    .map_err(|e| Error::Database(format!("Failed to get day events: {}", e)))?;

    // Fetch entity timestamps for the day: for each event's window, the earliest
    // timestamp each entity appears in wiki_refs.
    let event_windows: Vec<(String, DateTime<Utc>, DateTime<Utc>)> = rows
        .iter()
        .filter_map(|row| {
            let id: String = row.try_get("id").ok()?;
            let start: DateTime<Utc> = row.try_get("started_at").ok()?;
            let end: DateTime<Utc> = row.try_get("ended_at").ok()?;
            Some((id, start, end))
        })
        .collect();

    let mut entity_ts_by_event: HashMap<String, serde_json::Value> = HashMap::new();
    for (event_id, start, end) in &event_windows {
        let ref_rows: Vec<(String, DateTime<Utc>)> = sqlx::query_as(
            r#"
            SELECT entity_id, MIN(occurred_at) as earliest
            FROM wiki_refs
            WHERE occurred_at IS NOT NULL
              AND occurred_at >= $1
              AND occurred_at < $2
            GROUP BY entity_id
            "#,
        )
        .bind(*start)
        .bind(*end)
        .fetch_all(pool)
        .await
        .unwrap_or_default();

        if !ref_rows.is_empty() {
            let map: serde_json::Map<String, serde_json::Value> = ref_rows
                .into_iter()
                .map(|(id, ts)| (id, serde_json::Value::String(ts.to_rfc3339())))
                .collect();
            entity_ts_by_event.insert(event_id.clone(), serde_json::Value::Object(map));
        }
    }

    Ok(rows
        .iter()
        .filter_map(|row| {
            let id: String = row.try_get("id").ok()?;
            let day_id: String = row.try_get("day_id").ok()?;
            // TIMESTAMPTZ columns decode directly into DateTime<Utc>; the prior
            // `try_get::<String>` + parse_from_rfc3339 failed at the decode step
            // (timestamptz is not text), so `.ok()?` dropped every row and the
            // timeline came back empty. JSONB decodes into serde_json::Value and
            // BOOLEAN into bool — no string round-trip, no `!= 0`.
            let start_time: DateTime<Utc> = row.try_get("started_at").ok()?;
            let end_time: DateTime<Utc> = row.try_get("ended_at").ok()?;
            let created_at: DateTime<Utc> = row.try_get("created_at").ok()?;
            let updated_at: DateTime<Utc> = row.try_get("updated_at").ok()?;
            let entity_timestamps = entity_ts_by_event.get(&id).cloned();

            Some(TemporalEvent {
                id,
                day_id,
                start_time,
                end_time,
                auto_label: row.try_get::<Option<String>, _>("auto_label").ok().flatten(),
                auto_location: row.try_get::<Option<String>, _>("auto_location").ok().flatten(),
                user_label: row.try_get::<Option<String>, _>("user_label").ok().flatten(),
                user_location: row.try_get::<Option<String>, _>("user_location").ok().flatten(),
                user_notes: row.try_get::<Option<String>, _>("user_notes").ok().flatten(),
                source_ontologies: row.try_get::<Option<serde_json::Value>, _>("source_ontologies").ok().flatten(),
                is_unknown: row.try_get::<Option<bool>, _>("is_unknown").ok().flatten(),
                is_transit: row.try_get::<Option<bool>, _>("is_transit").ok().flatten(),
                is_user_added: row.try_get::<Option<bool>, _>("is_user_added").ok().flatten(),
                is_user_edited: row.try_get::<Option<bool>, _>("is_user_edited").ok().flatten(),
                novelty_z: row.try_get::<Option<f64>, _>("novelty_z").ok().flatten(),
                avg_hr: row.try_get::<Option<f64>, _>("avg_hr").ok().flatten(),
                autonomic_z: row.try_get::<Option<f64>, _>("autonomic_z").ok().flatten(),
                hr_z: row.try_get::<Option<f64>, _>("hr_z").ok().flatten(),
                topics: row.try_get::<Option<serde_json::Value>, _>("topics").ok().flatten(),
                event_summary: row.try_get::<Option<String>, _>("event_summary").ok().flatten(),
                agent_action: row.try_get::<Option<String>, _>("agent_action").ok().flatten(),
                is_sleep: row.try_get::<Option<bool>, _>("is_sleep").ok().flatten(),
                user_hidden: row.try_get::<Option<bool>, _>("user_hidden").ok().flatten(),
                entities: row.try_get::<Option<serde_json::Value>, _>("entities").ok().flatten(),
                topic_novelty: row.try_get::<Option<serde_json::Value>, _>("topic_novelty").ok().flatten(),
                entity_novelty: row.try_get::<Option<serde_json::Value>, _>("entity_novelty").ok().flatten(),
                entity_timestamps,
                created_at,
                updated_at,
            })
        })
        .collect())
}

/// Get events for a day by date
pub async fn get_events_by_date(pool: &PgPool, date: NaiveDate) -> Result<Vec<TemporalEvent>> {
    let day = get_or_create_day(pool, date).await?;
    get_day_events(pool, day.id).await
}

/// Create a temporal event
pub async fn create_temporal_event(
    pool: &PgPool,
    req: CreateTemporalEventRequest,
) -> Result<TemporalEvent> {
    use sqlx::Row;

    let day_id_str = req.day_id.to_string();
    let start_time_str = req.start_time.to_rfc3339();
    let end_time_str = req.end_time.to_rfc3339();
    // `source_ontologies` is NOT NULL with a `'[]'` default, and the segmentation
    // path deliberately passes None — it is stamped afterwards by `annotate`. But
    // naming the column in the INSERT and binding None sends SQL NULL, which
    // OVERRIDES the default and violates the constraint, so EVERY event insert on
    // that path failed and no day could be segmented. Default None to an empty
    // array, which is what the column would have used had we omitted it.
    let source_ontologies_str = Some(
        req.source_ontologies
            .as_ref()
            .map(|v| serde_json::to_string(v).unwrap_or_else(|_| "[]".to_string()))
            .unwrap_or_else(|| "[]".to_string()),
    );

    let event_id = ids::generate_id(ids::WIKI_EVENT_PREFIX, &[&req.day_id, &start_time_str, &end_time_str]);

    // `kind` is the source of truth; the is_unknown/is_transit booleans are generated
    // from it, so we set kind here rather than the (unwritable) generated columns.
    // create_temporal_event never mints sleep — that is `dayline::sleep`'s job.
    let kind = if req.is_unknown == Some(true) {
        "unknown"
    } else if req.is_transit == Some(true) {
        "transit"
    } else {
        "stay"
    };

    // Runtime query (not the macro) so we can include `event_summary` without
    // regenerating the sqlx offline cache.
    let row = sqlx::query(
        r#"
        INSERT INTO wiki_events (
            id, day_id, started_at, ended_at,
            auto_label, auto_location, user_label, user_location, user_notes,
            source_ontologies, kind, is_user_added, event_summary,
            topics
        ) VALUES ($1, $2, $3::timestamptz, $4::timestamptz, $5, $6, $7, $8, $9, $10::jsonb, $11, $12, $13, $14::jsonb)
        RETURNING
            id, is_user_edited, created_at, updated_at
        "#,
    )
    .bind(&event_id)
    .bind(&day_id_str)
    .bind(&start_time_str)
    .bind(&end_time_str)
    .bind(&req.auto_label)
    .bind(&req.auto_location)
    .bind(&req.user_label)
    .bind(&req.user_location)
    .bind(&req.user_notes)
    .bind(&source_ontologies_str)
    .bind(kind)
    .bind(req.is_user_added)
    .bind(&req.event_summary)
    .bind(req.topics.clone().unwrap_or_else(|| serde_json::json!([])))
    .fetch_one(pool)
    .await
    .map_err(|e| Error::Database(format!("Failed to create temporal event: {}", e)))?;

    let id: String = row
        .try_get("id")
        .map_err(|e| Error::Database(format!("Missing event ID: {}", e)))?;
    let is_user_edited: Option<bool> = row.try_get("is_user_edited").ok().flatten();
    let created_at: DateTime<Utc> = row
        .try_get("created_at")
        .map_err(|e| Error::Database(format!("Missing created_at: {}", e)))?;
    let updated_at: DateTime<Utc> = row
        .try_get("updated_at")
        .map_err(|e| Error::Database(format!("Missing updated_at: {}", e)))?;

    Ok(TemporalEvent {
        id,
        day_id: req.day_id,
        start_time: req.start_time,
        end_time: req.end_time,
        auto_label: req.auto_label,
        auto_location: req.auto_location,
        user_label: req.user_label,
        user_location: req.user_location,
        user_notes: req.user_notes,
        source_ontologies: req.source_ontologies,
        is_unknown: req.is_unknown,
        is_transit: req.is_transit,
        is_user_added: req.is_user_added,
        is_user_edited,
        novelty_z: None,
        avg_hr: None,
        autonomic_z: None,
        hr_z: None,
        topics: None,
        event_summary: req.event_summary,
        agent_action: None,
        is_sleep: Some(false),
        user_hidden: Some(false),
        entities: None,
        topic_novelty: None,
        entity_novelty: None,
        entity_timestamps: None,
        created_at,
        updated_at,
    })
}

/// Update a temporal event (user edits)
pub async fn update_temporal_event(
    pool: &PgPool,
    id: String,
    req: UpdateTemporalEventRequest,
) -> Result<TemporalEvent> {
    let id_str = id.clone();

    let row = sqlx::query!(
        r#"
        UPDATE wiki_events
        SET
            user_label = COALESCE($2, user_label),
            user_location = COALESCE($3, user_location),
            user_notes = COALESCE($4, user_notes),
            is_user_edited = true,
            updated_at = now()
        WHERE id = $1
        RETURNING
            id, day_id, started_at, ended_at,
            auto_label, auto_location, user_label, user_location, user_notes,
            source_ontologies, is_unknown, is_transit, is_user_added, is_user_edited,
            created_at, updated_at
        "#,
        id_str,
        req.user_label,
        req.user_location,
        req.user_notes
    )
    .fetch_optional(pool)
    .await
    .map_err(|e| Error::Database(format!("Failed to update temporal event: {}", e)))?
    .ok_or_else(|| Error::NotFound(format!("Temporal event not found: {}", id)))?;

    Ok(TemporalEvent {
        id: row.id,
        day_id: row.day_id,
        start_time: row.started_at,
        end_time: row.ended_at,
        auto_label: row.auto_label,
        auto_location: row.auto_location,
        user_label: row.user_label,
        user_location: row.user_location,
        user_notes: row.user_notes,
        source_ontologies: serde_json::from_value(row.source_ontologies).ok(),
        is_unknown: Some(row.is_unknown),
        is_transit: Some(row.is_transit),
        is_user_added: Some(row.is_user_added),
        is_user_edited: Some(row.is_user_edited),
        novelty_z: None,
        avg_hr: None,
        autonomic_z: None,
        hr_z: None,
        topics: None,
        event_summary: None,
        agent_action: None,
        is_sleep: Some(false),
        user_hidden: Some(false),
        entities: None,
        topic_novelty: None,
        entity_novelty: None,
        entity_timestamps: None,
        created_at: row.created_at,
        updated_at: row.updated_at,
    })
}

/// Delete a temporal event
pub async fn delete_temporal_event(pool: &PgPool, id: String) -> Result<()> {
    let id_str = id.clone();

    let result = sqlx::query!("DELETE FROM wiki_events WHERE id = $1", id_str)
        .execute(pool)
        .await
        .map_err(|e| Error::Database(format!("Failed to delete temporal event: {}", e)))?;

    if result.rows_affected() == 0 {
        return Err(Error::NotFound(format!("Temporal event not found: {}", id)));
    }

    Ok(())
}

/// Delete all auto-generated events for a day (for regeneration).
///
/// "Auto-generated" means UNTOUCHED BY THE USER, and that is three flags, not
/// one. `is_user_added` alone was the guard, and it is only set by
/// `create_temporal_event` — relabelling an event the machine cut sets
/// `is_user_edited` (see `update_temporal_event`) and leaves `is_user_added`
/// false, so every re-cut silently deleted the user's own label. That fires
/// whenever late audio or a backfilled message changes the day's
/// `sources_fingerprint`, which is exactly when a day gets re-cut in practice.
/// `user_hidden` is the same class: a hide the re-cut eats comes back visible.
///
/// Preserved events can now overlap the fresh cut — but that was already true
/// of `is_user_added` events, so this widens an accepted condition rather than
/// introducing one. A user's judgement outranks a gapless timeline.
pub async fn delete_auto_events_for_day(pool: &PgPool, day_id: String) -> Result<u64> {
    let day_id_str = day_id;

    let result = sqlx::query!(
        r#"
        DELETE FROM wiki_events
        WHERE day_id = $1
          AND is_user_added = false
          AND is_user_edited = false
          AND user_hidden = false
        "#,
        day_id_str
    )
    .execute(pool)
    .await
    .map_err(|e| Error::Database(format!("Failed to delete auto events: {}", e)))?;

    Ok(result.rows_affected())
}

// ============================================================================
// Day Sources - Ontology records for a day
// ============================================================================

/// A data source record from an ontology table for a specific day
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaySource {
    pub source_type: String,
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub label: String,
    pub preview: Option<String>,
    /// True for high-frequency measurement streams (heart rate, steps, HRV).
    /// The day page hides these behind a filter by default since a single day
    /// can hold thousands of them.
    pub continuous: bool,
}

/// Resolve the timezone a given day should be rendered/windowed in —
/// "the timezone you woke up in", fixed at the day's start:
///   1. the locked `wiki_days.start_timezone` if a summary already ran (past days
///      keep the zone they were lived in), else
///   2. `tzf-rs(first located point of the day)` — the same "where you woke up"
///      signal the EOD lock uses, so live-today and locked-history agree even on
///      a travel day (a move surfaces as *tomorrow*, not a mid-day re-anchor), else
///   3. the viewing device's zone, but ONLY for an in-progress today with no
///      located points yet (web-only / location off), else
///   4. `home_timezone`.
/// See agents/record/timezone-model.md.
async fn resolve_render_timezone(
    pool: &PgPool,
    date: NaiveDate,
    client_tz: Option<&str>,
) -> String {
    use sqlx::Row;
    // 1. Locked per-day zone from a prior summary.
    if let Ok(Some(row)) =
        sqlx::query("SELECT start_timezone FROM wiki_days WHERE date = $1")
            .bind(date)
            .fetch_optional(pool)
            .await
    {
        if let Ok(Some(tz)) = row.try_get::<Option<String>, _>("start_timezone") {
            if !tz.is_empty() {
                return tz;
            }
        }
    }

    let home_tz = super::profile::get_timezone(pool)
        .await
        .unwrap_or(None)
        .unwrap_or_else(|| "UTC".to_string());

    // 2. Where the owner woke up that day (first located point). Authoritative and
    //    consistent with the EOD lock — does NOT drift to where the viewer is now.
    if let Some(tz) = crate::timezone::first_point_timezone(pool, date, &home_tz).await {
        return tz;
    }

    // 3. No location for the day — for an in-progress *today* only, fall back to the
    //    viewing device's zone (best available "where are you" for a web-only/
    //    location-off owner). Never applied to a past day. "Today" is in home_tz.
    let today_in_home = home_tz
        .parse::<chrono_tz::Tz>()
        .ok()
        .map(|tz| Utc::now().with_timezone(&tz).date_naive());
    if today_in_home == Some(date) {
        if let Some(tz) = client_tz {
            if !tz.is_empty() {
                return tz.to_string();
            }
        }
    }

    // 4. Home.
    home_tz
}

/// Get all ontology data sources for a specific date (registry-driven).
///
/// Iterates over all registered ontologies that have a `DaySourceConfig` and builds
/// dynamic SQL queries from the config. No arbitrary LIMITs — all data included
/// with a sanity check for overflow.
pub async fn get_day_sources(
    pool: &PgPool,
    date: NaiveDate,
    client_tz: Option<&str>,
) -> Result<Vec<DaySource>> {
    use sqlx::Row;
    use virtues_registry::ontologies::registered_ontologies;

    // Day window in the per-day "where the owner was" timezone, fixed at the
    // day's start ("the timezone you woke up in"). Resolution order:
    //   1. the locked wiki_days.start_timezone for this day (past days), else
    //   2. the viewing device's zone for an in-progress today (client_tz), else
    //   3. tzf-rs(first located point of the day) → home_timezone fallback.
    // See agents/record/timezone-model.md.
    let timezone = resolve_render_timezone(pool, date, client_tz).await;
    let (start_str, end_str) =
        super::day_summary::day_boundaries_utc(date, Some(&timezone));
    let mut sources: Vec<DaySource> = Vec::new();

    for ont in registered_ontologies() {
        let cfg = match &ont.day_source {
            Some(c) => c,
            None => continue,
        };

        // Build SELECT columns
        let source_type_col = cfg
            .source_type_sql
            .map(|sql| format!("{} as source_type_dyn", sql))
            .unwrap_or_else(|| format!("'{}' as source_type_dyn", cfg.source_type));

        let query = if cfg.use_date_filter {
            format!(
                "SELECT {id} as src_id, {ts} as src_ts, {label} as src_label, {preview} as src_preview, {st} \
                 FROM {table} t \
                 WHERE date(t.{ts_col}) = $1 \
                 {extra} \
                 ORDER BY t.{ts_col} ASC",
                id = cfg.id_sql,
                ts = ont.timestamp_column,
                label = cfg.label_sql,
                preview = cfg.preview_sql,
                st = source_type_col,
                table = ont.table_name,
                ts_col = ont.timestamp_column,
                extra = cfg.extra_where.unwrap_or(""),
            )
        } else {
            format!(
                "SELECT {id} as src_id, {ts} as src_ts, {label} as src_label, {preview} as src_preview, {st} \
                 FROM {table} t \
                 WHERE t.{ts_col} >= $1::timestamptz AND t.{ts_col} <= $2::timestamptz \
                 {extra} \
                 ORDER BY t.{ts_col} ASC",
                id = cfg.id_sql,
                ts = ont.timestamp_column,
                label = cfg.label_sql,
                preview = cfg.preview_sql,
                st = source_type_col,
                table = ont.table_name,
                ts_col = ont.timestamp_column,
                extra = cfg.extra_where.unwrap_or(""),
            )
        };

        let rows = if cfg.use_date_filter {
            sqlx::query(&query)
                .bind(date)
                .fetch_all(pool)
                .await
        } else {
            sqlx::query(&query)
                .bind(&start_str)
                .bind(&end_str)
                .fetch_all(pool)
                .await
        };

        // A day-source query that fails is not a warning. It means the day is being
        // assembled with a HOLE in it — and then an LLM writes a confident account
        // of a day it was never shown.
        //
        // Two of these were broken on the box for as long as they have existed:
        //
        //   location_visit       `encode(t.id,'hex')` on a TEXT id
        //   activity_app_session `extra_where` missing its leading AND
        //
        // Both raised here, both were swallowed with `warn!` + `continue`, and the
        // cron reported SUCCESS every single night. The result: 103 days of a real
        // life produced 2 events and zero autobiographies, and nothing anywhere said
        // a word about it.
        //
        // A missing source is a broken query, and a broken query is a bug to fix —
        // never a day to fabricate around it.
        let rows = rows.map_err(|e| {
            Error::Database(format!(
                "day source query failed for ontology `{}` — the day cannot be \
                 assembled without it, and generating a narrative from the gap would \
                 invent a day you did not live: {e}",
                ont.name
            ))
        })?;

        // Sanity check
        if rows.len() > 5000 {
            tracing::warn!(
                ontology = ont.name,
                count = rows.len(),
                "Unusually large source count for single day"
            );
        }

        for row in &rows {
            let id: String = match row.try_get("src_id") {
                Ok(v) => v,
                Err(_) => continue,
            };
            // `src_ts` aliases a TIMESTAMPTZ column — decode it directly. Reading
            // it as String (then re-parsing) failed at the decode step and
            // `continue`d past every row, so these ontologies never appeared.
            let ts: DateTime<Utc> = match row.try_get("src_ts") {
                Ok(v) => v,
                Err(_) => continue,
            };
            let label: String = row.try_get("src_label").unwrap_or_else(|_| ont.display_name.to_string());
            let preview: Option<String> = row.try_get("src_preview").ok().flatten();
            let source_type: String = row.try_get("source_type_dyn").unwrap_or_else(|_| cfg.source_type.to_string());

            sources.push(DaySource {
                source_type,
                id,
                timestamp: ts,
                label,
                preview,
                continuous: ont.temporal_type
                    == virtues_registry::ontologies::TemporalType::Continuous,
            });
        }
    }

    sources.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));
    Ok(sources)
}

/// One raw record linked to an entity via `wiki_refs` — the entity
/// page's CRM-style evidence feed. Same shape as `DaySource` plus the ref's
/// `role` (sender, attendee, merchant, location, …).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityRecord {
    pub source_type: String,
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub label: String,
    pub preview: Option<String>,
    pub role: Option<String>,
    pub continuous: bool,
}

/// One page of an entity's records, plus the true total for the query — the
/// server half of the grid's server-side pagination.
#[derive(Debug, Serialize)]
pub struct EntityRecordsPage {
    pub items: Vec<EntityRecord>,
    pub total: i64,
}

/// Per-raw-source_type counts across ALL of an entity's records, for the chip
/// rail. Computed server-side because the grid only ever holds one page —
/// chips counted from loaded rows would lie.
#[derive(Debug, Serialize)]
pub struct EntityRecordFacet {
    pub source_type: String,
    pub count: i64,
    pub continuous: bool,
}

/// Requests can't ask for unbounded pages.
const ENTITY_RECORDS_MAX_LIMIT: i64 = 100;

/// Build the UNION ALL body over every source table holding refs for this
/// entity, each subquery rendered with its ontology's `DaySourceConfig` SQL
/// (same labels/previews as the day page). Roles are merged per record inside
/// each subquery, so pagination and totals count records, not refs. Returns
/// `None` when the entity has no refs in any renderable table.
///
/// All subqueries bind the entity id as `$1`; callers add outer binds from $2.
async fn entity_records_union(pool: &PgPool, entity_id: &str) -> Result<Option<String>> {
    use virtues_registry::ontologies::registered_ontologies;

    let tables: Vec<String> = sqlx::query_scalar(
        "SELECT DISTINCT source_table FROM wiki_refs WHERE entity_id = $1",
    )
    .bind(entity_id)
    .fetch_all(pool)
    .await
    .map_err(|e| Error::Database(format!("Failed to list entity ref tables: {}", e)))?;

    let mut subqueries: Vec<String> = Vec::new();
    for table in &tables {
        let Some(ont) = registered_ontologies()
            .into_iter()
            .find(|o| o.table_name == table.as_str() && o.day_source.is_some())
        else {
            // Refs exist but no ontology knows how to render them. Unlike the
            // day pipeline (where a hole feeds an LLM), this only starves a
            // reading surface — so skip loudly instead of failing the page,
            // and treat the log line as the bug report it is.
            tracing::warn!(
                source_table = %table,
                entity_id = %entity_id,
                "entity refs point at a table with no day-source config; its records are invisible"
            );
            continue;
        };
        let cfg = ont.day_source.as_ref().unwrap();

        let source_type_col = cfg
            .source_type_sql
            .map(|sql| format!("{} as source_type_dyn", sql))
            .unwrap_or_else(|| format!("'{}' as source_type_dyn", cfg.source_type));

        // Notes on the shape:
        //  - `WHERE true` so `extra_where` (which carries its own leading AND)
        //    splices the same way it does in the day-source template.
        //  - The refs JOIN introduces a second `timestamp` column, so the
        //    ontology's timestamp must be `t.`-qualified or Postgres calls it
        //    ambiguous.
        //  - The refs unique key includes `role`, so one record can join once
        //    per role (sender AND recipient): the GROUP BY collapses those to
        //    one row with the roles aggregated. Positional GROUP BY, because
        //    the grouped expressions are registry-supplied SQL.
        //  - `src_cont` is a bare literal, which Postgres exempts from
        //    GROUP BY.
        subqueries.push(format!(
            "SELECT {id} as src_id, t.{ts} as src_ts, {label} as src_label, \
                    {preview} as src_preview, {st}, \
                    string_agg(DISTINCT er.role, ', ') as src_role, \
                    {cont} as src_cont \
             FROM {table} t \
             JOIN wiki_refs er \
               ON er.source_table = '{table}' AND er.source_id = {id} AND er.entity_id = $1 \
             WHERE true \
             {extra} \
             GROUP BY 1, 2, 3, 4, 5",
            id = cfg.id_sql,
            ts = ont.timestamp_column,
            label = cfg.label_sql,
            preview = cfg.preview_sql,
            st = source_type_col,
            cont = if ont.temporal_type == virtues_registry::ontologies::TemporalType::Continuous {
                "TRUE"
            } else {
                "FALSE"
            },
            table = ont.table_name,
            extra = cfg.extra_where.unwrap_or(""),
        ));
    }

    Ok(if subqueries.is_empty() {
        None
    } else {
        Some(subqueries.join(" UNION ALL "))
    })
}

/// Shared narrowing clause for the union: $2 = search text ('' = all),
/// $3 = raw source_type allowlist (empty array = all).
const ENTITY_RECORDS_WHERE: &str = "($2 = '' \
       OR u.src_label ILIKE '%' || $2 || '%' \
       OR COALESCE(u.src_preview, '') ILIKE '%' || $2 || '%') \
   AND (cardinality($3::text[]) = 0 OR u.source_type_dyn = ANY($3::text[]))";

fn entity_record_from_row(row: &sqlx::postgres::PgRow) -> Option<EntityRecord> {
    use sqlx::Row;
    Some(EntityRecord {
        id: row.try_get("src_id").ok()?,
        // TIMESTAMPTZ — decode directly, never via String (see get_day_sources).
        timestamp: row.try_get("src_ts").ok()?,
        label: row.try_get("src_label").unwrap_or_default(),
        preview: row.try_get("src_preview").ok().flatten(),
        source_type: row.try_get("source_type_dyn").unwrap_or_default(),
        role: row.try_get("src_role").ok().flatten(),
        continuous: row.try_get("src_cont").unwrap_or(false),
    })
}

/// One page of the records linked to an entity (registry-driven, refs-driven),
/// with search and source_type narrowing applied server-side.
pub async fn get_entity_records_page(
    pool: &PgPool,
    entity_id: &str,
    offset: i64,
    limit: i64,
    search: &str,
    types: &[String],
    newest_first: bool,
) -> Result<EntityRecordsPage> {
    let limit = limit.clamp(1, ENTITY_RECORDS_MAX_LIMIT);
    let offset = offset.max(0);

    let Some(union) = entity_records_union(pool, entity_id).await? else {
        return Ok(EntityRecordsPage { items: Vec::new(), total: 0 });
    };

    let total: i64 = sqlx::query_scalar(&format!(
        "SELECT count(*) FROM ({union}) u WHERE {ENTITY_RECORDS_WHERE}"
    ))
    .bind(entity_id)
    .bind(search)
    .bind(types)
    .fetch_one(pool)
    .await
    .map_err(|e| Error::Database(format!("entity records count failed: {e}")))?;

    let dir = if newest_first { "DESC" } else { "ASC" };
    let rows = sqlx::query(&format!(
        "SELECT * FROM ({union}) u WHERE {ENTITY_RECORDS_WHERE} \
         ORDER BY u.src_ts {dir} LIMIT $4 OFFSET $5"
    ))
    .bind(entity_id)
    .bind(search)
    .bind(types)
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await
    .map_err(|e| Error::Database(format!("entity records page failed: {e}")))?;

    Ok(EntityRecordsPage {
        items: rows.iter().filter_map(entity_record_from_row).collect(),
        total,
    })
}

/// Facet counts over ALL of an entity's records (unnarrowed), for the chips.
pub async fn get_entity_record_facets(
    pool: &PgPool,
    entity_id: &str,
) -> Result<Vec<EntityRecordFacet>> {
    use sqlx::Row;

    let Some(union) = entity_records_union(pool, entity_id).await? else {
        return Ok(Vec::new());
    };

    let rows = sqlx::query(&format!(
        "SELECT u.source_type_dyn as st, count(*) as n, bool_and(u.src_cont) as cont \
         FROM ({union}) u GROUP BY 1 ORDER BY 1"
    ))
    .bind(entity_id)
    .fetch_all(pool)
    .await
    .map_err(|e| Error::Database(format!("entity record facets failed: {e}")))?;

    Ok(rows
        .iter()
        .filter_map(|row| {
            Some(EntityRecordFacet {
                source_type: row.try_get("st").ok()?,
                count: row.try_get("n").ok()?,
                continuous: row.try_get("cont").unwrap_or(false),
            })
        })
        .collect())
}

// ============================================================================
// Timeline Day - Location chunks for movement map
// ============================================================================

/// A location chunk for the timeline day view.
///
/// One chunk per `data_location_visit` row, joined to its canonical place
/// (via `wiki_refs` → `wiki_places`) when one exists. Visits with no
/// place link have `place_id`/`place_name` set to None and the frontend
/// renders them as "Unknown".
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimelineChunk {
    #[serde(rename = "type")]
    pub chunk_type: String,
    pub start_time: String,
    pub end_time: String,
    pub place_name: Option<String>,
    pub latitude: f64,
    pub longitude: f64,
    pub place_id: Option<String>,
    pub duration_minutes: Option<i32>,
    pub place_category: Option<String>,
}

/// A raw GPS point for the movement track polyline.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimelinePoint {
    pub latitude: f64,
    pub longitude: f64,
    pub timestamp: String,
}

/// Timeline day view response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimelineDayView {
    pub date: String,
    /// Visits (clustered) — used by DayLocationTimeline + map markers
    pub chunks: Vec<TimelineChunk>,
    /// Raw GPS points — used by the map polyline (the actual path you walked)
    pub points: Vec<TimelinePoint>,
}

/// Get location visits for a day, returned as timeline chunks with their
/// canonical place link (if any).
pub async fn get_timeline_day(pool: &PgPool, date: NaiveDate) -> Result<TimelineDayView> {
    let start_of_day = date.and_hms_opt(0, 0, 0).unwrap().and_utc();
    let end_of_day = date
        .succ_opt()
        .unwrap()
        .and_hms_opt(0, 0, 0)
        .unwrap()
        .and_utc();

    // JOIN visits → wiki_refs → wiki_places.
    // er.source_id is the visit's UUID; both sides are stored as TEXT UUIDs,
    // so the join is a straight text match.
    let rows: Vec<sqlx::postgres::PgRow> = sqlx::query(
        r#"
        SELECT
            v.started_at           AS started_at,
            v.ended_at         AS ended_at,
            v.duration_minutes       AS duration_minutes,
            v.latitude               AS visit_lat,
            v.longitude              AS visit_lon,
            er.entity_id             AS place_id,
            p.name                   AS place_name,
            p.latitude               AS place_lat,
            p.longitude              AS place_lon,
            p.category               AS place_category
        FROM data_location_visit v
        LEFT JOIN wiki_refs er
            ON er.source_table = 'data_location_visit'
           AND er.source_id    = v.id
           AND er.entity_type  = 'place'
        LEFT JOIN wiki_places p ON p.id = er.entity_id
        WHERE v.started_at >= $1 AND v.started_at < $2
        ORDER BY v.started_at ASC
        "#,
    )
    .bind(start_of_day)
    .bind(end_of_day)
    .fetch_all(pool)
    .await
    .map_err(|e| Error::Database(format!("Failed to get location visits: {}", e)))?;

    use sqlx::Row;
    let chunks: Vec<TimelineChunk> = rows
        .iter()
        .filter_map(|row| {
            // started_at / ended_at are TIMESTAMPTZ in Postgres; read
            // them as DateTime<Utc> and stringify with RFC-3339 for the JSON.
            let arrival_ts: DateTime<Utc> = row.try_get("started_at").ok()?;
            let arrival = arrival_ts.to_rfc3339();
            let departure: Option<String> = row
                .try_get::<Option<DateTime<Utc>>, _>("ended_at")
                .ok()
                .flatten()
                .map(|d| d.to_rfc3339());
            let duration_minutes: Option<i32> = row.try_get("duration_minutes").ok();
            let visit_lat: f64 = row.try_get("visit_lat").ok()?;
            let visit_lon: f64 = row.try_get("visit_lon").ok()?;
            let place_id: Option<String> = row.try_get("place_id").ok();
            let place_name: Option<String> = row.try_get("place_name").ok();
            let place_lat: Option<f64> = row.try_get("place_lat").ok();
            let place_lon: Option<f64> = row.try_get("place_lon").ok();
            let place_category: Option<String> = row.try_get("place_category").ok();

            // Prefer canonical place coords over the visit centroid so all
            // visits to "Home" land on the same map pin regardless of GPS jitter.
            let lat = place_lat.unwrap_or(visit_lat);
            let lon = place_lon.unwrap_or(visit_lon);

            Some(TimelineChunk {
                chunk_type: "location".to_string(),
                start_time: arrival.clone(),
                end_time: departure.unwrap_or(arrival),
                place_name,
                latitude: lat,
                longitude: lon,
                place_id,
                duration_minutes,
                place_category,
            })
        })
        .collect();

    // Also fetch the raw GPS points so the map can render the actual path,
    // not just lines connecting visit centroids.
    let point_rows: Vec<sqlx::postgres::PgRow> = sqlx::query(
        r#"
        SELECT latitude, longitude, occurred_at
        FROM data_location_point
        WHERE occurred_at >= $1 AND occurred_at < $2
        ORDER BY occurred_at ASC
        "#,
    )
    .bind(start_of_day)
    .bind(end_of_day)
    .fetch_all(pool)
    .await
    .map_err(|e| Error::Database(format!("Failed to get location points: {}", e)))?;

    let points: Vec<TimelinePoint> = point_rows
        .iter()
        .filter_map(|row| {
            let lat: Option<f64> = row.try_get("latitude").ok();
            let lng: Option<f64> = row.try_get("longitude").ok();
            let ts: Option<String> = row
                .try_get::<Option<DateTime<Utc>>, _>("occurred_at")
                .ok()
                .flatten()
                .map(|t| t.to_rfc3339());
            match (lat, lng, ts) {
                (Some(lat), Some(lng), Some(ts)) => Some(TimelinePoint {
                    latitude: lat,
                    longitude: lng,
                    timestamp: ts,
                }),
                _ => None,
            }
        })
        .collect();

    Ok(TimelineDayView {
        date: date.to_string(),
        chunks,
        points,
    })
}

// ============================================================================
// Today Streams - the three raw record streams, as spans, before synthesis
// ============================================================================
//
// The homepage renders the day *before* the nightly synthesis has read it into
// a biography. At that point the box does not have "events" — it has three
// sensor streams, each with real start/end spans: where the phone was
// (data_location_visit), what the calendar promised (data_calendar_event), and
// when the microphone was open (data_audio_recording — the raw live chunks, NOT
// the nightly `data_audio_session` rollup, which doesn't exist mid-day). This
// endpoint returns exactly those three, tz-anchored, drawn as rectangles.

/// A location visit span (where you were, and for how long).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TodayLocationSpan {
    pub id: String,
    pub start_time: String,
    pub end_time: String,
    pub place_name: Option<String>,
    pub place_category: Option<String>,
    pub duration_minutes: Option<i32>,
}

/// A calendar event span (the day as intended).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TodayCalendarSpan {
    pub id: String,
    pub start_time: String,
    pub end_time: String,
    pub title: String,
    pub is_all_day: bool,
    pub is_sacred: bool,
    pub location_name: Option<String>,
    pub calendar_name: Option<String>,
}

/// A raw audio recording chunk (~5 min each) — the live mic capture, before any
/// sessionization. `is_silent` marks a chunk the box flagged as silence. The
/// client merges contiguous chunks into "mic was open" blocks.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TodayAudioSpan {
    pub id: String,
    pub start_time: String,
    pub end_time: String,
    pub is_silent: bool,
}

/// The three raw streams for a day, before the nightly synthesis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TodayStreamsView {
    pub date: String,
    /// The zone the spans are anchored to (see agents/record/timezone-model.md).
    pub timezone: String,
    pub location: Vec<TodayLocationSpan>,
    pub calendar: Vec<TodayCalendarSpan>,
    pub audio: Vec<TodayAudioSpan>,
}

/// One heart-rate sample, for the day page's Autonomic chart.
#[derive(Debug, serde::Serialize)]
pub struct DayHeartRateSample {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub bpm: i32,
}

/// Raw heart-rate samples across a day's window, oldest first. The client
/// draws; sparse days (a dozen samples) are normal and the chart must read
/// honestly at that density — dots joined by a line, never a smoothed curve
/// that invents continuity the record does not hold.
pub async fn get_day_heart_rate(
    pool: &PgPool,
    date: NaiveDate,
    client_tz: Option<&str>,
) -> Result<Vec<DayHeartRateSample>> {
    let timezone = resolve_render_timezone(pool, date, client_tz).await;
    let (start_str, end_str) = super::day_summary::day_boundaries_utc(date, Some(&timezone));

    let rows: Vec<(chrono::DateTime<chrono::Utc>, i32)> = sqlx::query_as(
        r#"SELECT occurred_at, bpm FROM data_health_heart_rate
           WHERE occurred_at >= $1::timestamptz AND occurred_at < $2::timestamptz
           ORDER BY occurred_at"#,
    )
    .bind(&start_str)
    .bind(&end_str)
    .fetch_all(pool)
    .await
    .map_err(|e| Error::Database(format!("Failed to load heart rate: {}", e)))?;

    Ok(rows
        .into_iter()
        .map(|(timestamp, bpm)| DayHeartRateSample { timestamp, bpm })
        .collect())
}

/// Get the three raw record streams (location, calendar, audio) for a day as
/// spans. Anchored to the day's effective timezone exactly like `get_day_sources`.
pub async fn get_today_streams(
    pool: &PgPool,
    date: NaiveDate,
    client_tz: Option<&str>,
) -> Result<TodayStreamsView> {
    use sqlx::Row;

    let timezone = resolve_render_timezone(pool, date, client_tz).await;
    let (start_str, end_str) = super::day_summary::day_boundaries_utc(date, Some(&timezone));

    // --- Location: where the phone was ---
    let loc_rows = sqlx::query(
        r#"
        SELECT
            v.id               AS id,
            v.started_at     AS started_at,
            v.ended_at   AS ended_at,
            v.duration_minutes AS duration_minutes,
            p.name             AS place_name,
            p.category         AS place_category
        FROM data_location_visit v
        LEFT JOIN wiki_refs er
            ON er.source_table = 'data_location_visit'
           AND er.source_id    = v.id
           AND er.entity_type  = 'place'
        LEFT JOIN wiki_places p ON p.id = er.entity_id
        WHERE v.started_at >= $1::timestamptz AND v.started_at < $2::timestamptz
        ORDER BY v.started_at ASC
        "#,
    )
    .bind(&start_str)
    .bind(&end_str)
    .fetch_all(pool)
    .await
    .map_err(|e| Error::Database(format!("today streams: location query failed: {e}")))?;

    let location: Vec<TodayLocationSpan> = loc_rows
        .iter()
        .filter_map(|row| {
            let arrival: DateTime<Utc> = row.try_get("started_at").ok()?;
            let departure: Option<DateTime<Utc>> =
                row.try_get::<Option<DateTime<Utc>>, _>("ended_at").ok().flatten();
            Some(TodayLocationSpan {
                id: row.try_get("id").ok()?,
                start_time: arrival.to_rfc3339(),
                end_time: departure.unwrap_or(arrival).to_rfc3339(),
                place_name: row.try_get("place_name").ok().flatten(),
                place_category: row.try_get("place_category").ok().flatten(),
                duration_minutes: row.try_get("duration_minutes").ok(),
            })
        })
        .collect();

    // --- Calendar: the day as intended ---
    let cal_rows = sqlx::query(
        r#"
        SELECT id, title, started_at, ended_at, is_all_day,
               COALESCE(is_sacred, FALSE) AS is_sacred, location_name, calendar_name
        FROM data_calendar_event
        WHERE started_at >= $1::timestamptz AND started_at < $2::timestamptz
        ORDER BY started_at ASC
        "#,
    )
    .bind(&start_str)
    .bind(&end_str)
    .fetch_all(pool)
    .await
    .map_err(|e| Error::Database(format!("today streams: calendar query failed: {e}")))?;

    let calendar: Vec<TodayCalendarSpan> = cal_rows
        .iter()
        .filter_map(|row| {
            let start: DateTime<Utc> = row.try_get("started_at").ok()?;
            let end: DateTime<Utc> = row.try_get("ended_at").ok()?;
            Some(TodayCalendarSpan {
                id: row.try_get("id").ok()?,
                start_time: start.to_rfc3339(),
                end_time: end.to_rfc3339(),
                title: row.try_get("title").unwrap_or_else(|_| "(no title)".to_string()),
                is_all_day: row.try_get("is_all_day").unwrap_or(false),
                is_sacred: row.try_get("is_sacred").unwrap_or(false),
                location_name: row.try_get("location_name").ok().flatten(),
                calendar_name: row.try_get("calendar_name").ok().flatten(),
            })
        })
        .collect();

    // --- Audio: raw recording chunks (the live mic capture) ---
    let aud_rows = sqlx::query(
        r#"
        SELECT id, started_at, ended_at, duration_seconds, COALESCE(is_silent, FALSE) AS is_silent
        FROM data_audio_recording
        WHERE started_at >= $1::timestamptz AND started_at < $2::timestamptz
        ORDER BY started_at ASC
        "#,
    )
    .bind(&start_str)
    .bind(&end_str)
    .fetch_all(pool)
    .await
    .map_err(|e| Error::Database(format!("today streams: audio query failed: {e}")))?;

    let audio: Vec<TodayAudioSpan> = aud_rows
        .iter()
        .filter_map(|row| {
            let start: DateTime<Utc> = row.try_get("started_at").ok()?;
            let ended: Option<DateTime<Utc>> =
                row.try_get::<Option<DateTime<Utc>>, _>("ended_at").ok().flatten();
            let dur_s: Option<f64> = row.try_get("duration_seconds").ok().flatten();
            // Fall back to the chunk's duration (or a nominal 5 min) when it has no end.
            let end = ended.unwrap_or_else(|| {
                start + chrono::Duration::milliseconds((dur_s.unwrap_or(300.0) * 1000.0) as i64)
            });
            Some(TodayAudioSpan {
                id: row.try_get("id").ok()?,
                start_time: start.to_rfc3339(),
                end_time: end.to_rfc3339(),
                is_silent: row.try_get("is_silent").unwrap_or(false),
            })
        })
        .collect();

    Ok(TodayStreamsView {
        date: date.to_string(),
        timezone,
        location,
        calendar,
        audio,
    })
}

// ============================================================================
// Day Streams - Dynamic Ontology Queries
// ============================================================================

/// A single record from an ontology table
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamRecord {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub end_timestamp: Option<DateTime<Utc>>,
    pub preview: serde_json::Value,
}

/// Data stream from a single ontology
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DayStream {
    pub ontology_name: String,
    pub display_name: String,
    pub domain: String,
    pub count: usize,
    pub records: Vec<StreamRecord>,
}

/// Response for GET /api/wiki/day/{date}/streams
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DayStreamsResponse {
    pub date: NaiveDate,
    pub queried_at: DateTime<Utc>,
    pub streams: Vec<DayStream>,
    pub total_count: usize,
}

/// Get all ontology data streams for a specific date
///
/// Dynamically queries all registered ontology tables using their
/// timestamp_column metadata to filter records for the given day.
pub async fn get_day_streams(pool: &PgPool, date: NaiveDate) -> Result<DayStreamsResponse> {
    use virtues_registry::ontologies::registered_ontologies;
    use sqlx::Row;

    // Calculate UTC bounds for the date
    // Expand window to cover any timezone: UTC-12 to UTC+14
    let start = date
        .and_hms_opt(0, 0, 0)
        .unwrap()
        .and_utc()
        .checked_sub_signed(chrono::Duration::hours(12))
        .unwrap();
    let end = date
        .succ_opt()
        .unwrap()
        .and_hms_opt(0, 0, 0)
        .unwrap()
        .and_utc()
        .checked_add_signed(chrono::Duration::hours(14))
        .unwrap();

    let ontologies = registered_ontologies();
    let mut streams = Vec::new();

    for ontology in &ontologies {
        // Skip non-time-series ontologies
        if should_skip_ontology_for_streams(ontology.name) {
            continue;
        }

        let table = ontology.table_name;
        let ts_col = ontology.timestamp_column;

        // Build SELECT clause for end timestamp if present
        let end_select = ontology
            .end_timestamp_column
            .map(|c| format!(", {} as end_ts", c))
            .unwrap_or_default();

        // All ontology tables (incl. location_visit) use TEXT UUID ids — see the
        // `data_location_visit` join comment above. An earlier version assumed
        // location_visit had blob ids and wrapped them in `encode(id, 'hex')`,
        // but `encode()` only accepts `bytea`, so that query failed at runtime
        // with "function encode(text, unknown) does not exist" — silently
        // breaking the day page's location rendering. Select the id directly.
        let id_select = "id";

        // Build dynamic query - select id, timestamps, and all other columns as JSON
        let sql = format!(
            "SELECT {id_select}, {ts_col} as ts{end_select}, * FROM {table}
             WHERE {ts_col} >= $1 AND {ts_col} < $2
             ORDER BY {ts_col} ASC
             LIMIT 100",
            id_select = id_select,
            ts_col = ts_col,
            end_select = end_select,
            table = table,
        );

        // Execute query with dynamic SQL
        let rows = match sqlx::query(&sql)
            .bind(start)
            .bind(end)
            .fetch_all(pool)
            .await
        {
            Ok(rows) => rows,
            Err(e) => {
                tracing::warn!(
                    "Failed to query {} for day streams: {}",
                    ontology.name,
                    e
                );
                continue;
            }
        };

        if rows.is_empty() {
            continue;
        }

        let mut records = Vec::new();
        for row in &rows {
            // Get id
            let id: String = row.try_get("id").unwrap_or_default();
            if id.is_empty() {
                continue;
            }

            // Get timestamp — `ts` aliases a TIMESTAMPTZ column, decode directly.
            let timestamp: DateTime<Utc> = match row.try_get("ts") {
                Ok(ts) => ts,
                Err(_) => continue,
            };

            // Get end timestamp if present
            let end_timestamp = if ontology.end_timestamp_column.is_some() {
                row.try_get::<Option<DateTime<Utc>>, _>("end_ts").ok().flatten()
            } else {
                None
            };

            // Build preview from key columns based on ontology type
            let preview = build_preview_for_ontology(ontology.name, &row);

            records.push(StreamRecord {
                id,
                timestamp,
                end_timestamp,
                preview,
            });
        }

        if !records.is_empty() {
            streams.push(DayStream {
                ontology_name: ontology.name.to_string(),
                display_name: ontology.display_name.to_string(),
                domain: ontology.domain.to_string(),
                count: records.len(),
                records,
            });
        }
    }

    // Sort streams by domain for consistent ordering
    streams.sort_by(|a, b| a.domain.cmp(&b.domain));

    let total_count = streams.iter().map(|s| s.count).sum();

    Ok(DayStreamsResponse {
        date,
        queried_at: Utc::now(),
        streams,
        total_count,
    })
}

/// Check if an ontology should be skipped for day streams
fn should_skip_ontology_for_streams(name: &str) -> bool {
    // Skip entity tables (not time-series events)
    name.starts_with("entities_")
        // Skip financial accounts (reference data, not events)
        || name == "financial_account"
        // Skip location points (use visits instead)
        || name == "location_point"
}

/// Build a preview JSON object for a specific ontology type
fn build_preview_for_ontology(ontology_name: &str, row: &sqlx::postgres::PgRow) -> serde_json::Value {
    use sqlx::Row;

    match ontology_name {
        "calendar_event" => {
            serde_json::json!({
                "title": row.try_get::<String, _>("title").ok(),
                "location": row.try_get::<String, _>("location_name").ok(),
            })
        }
        "communication_email" => {
            serde_json::json!({
                "subject": row.try_get::<String, _>("subject").ok(),
                "from": row.try_get::<String, _>("from_email").ok(),
                "direction": row.try_get::<String, _>("direction").ok(),
            })
        }
        "communication_message" => {
            let body: Option<String> = row.try_get("body").ok();
            let preview = body.map(|c| {
                let truncated: String = c.chars().take(100).collect();
                if truncated.len() < c.len() {
                    format!("{truncated}...")
                } else {
                    truncated
                }
            });
            serde_json::json!({
                "from": row.try_get::<String, _>("from_name").ok(),
                "channel": row.try_get::<String, _>("channel").ok(),
                "preview": preview,
            })
        }
        "location_visit" => {
            serde_json::json!({
                "place_name": row.try_get::<String, _>("place_name").ok(),
                "duration_minutes": row.try_get::<i32, _>("duration_minutes").ok(),
            })
        }
        "health_workout" => {
            serde_json::json!({
                "workout_type": row.try_get::<String, _>("workout_type").ok(),
                "duration_minutes": row.try_get::<i32, _>("duration_minutes").ok(),
                "calories": row.try_get::<i32, _>("calories_burned").ok(),
            })
        }
        "health_sleep" => {
            serde_json::json!({
                "duration_minutes": row.try_get::<i32, _>("duration_minutes").ok(),
                "quality_score": row.try_get::<f64, _>("sleep_quality_score").ok(),
            })
        }
        "health_heart_rate" => {
            serde_json::json!({
                "bpm": row.try_get::<i32, _>("bpm").ok(),
            })
        }
        "health_steps" => {
            serde_json::json!({
                "step_count": row.try_get::<i32, _>("step_count").ok(),
            })
        }
        "financial_transaction" => {
            let amount_cents: Option<i64> = row.try_get("amount").ok();
            serde_json::json!({
                "merchant": row.try_get::<String, _>("merchant_name").ok(),
                "amount": amount_cents.map(|c| c as f64 / 100.0),
                "category": row.try_get::<String, _>("merchant_category").ok(),
            })
        }
        "activity_app_usage" => {
            serde_json::json!({
                "app_name": row.try_get::<String, _>("app_name").ok(),
                "window_title": row.try_get::<String, _>("window_title").ok(),
            })
        }
        "activity_web_browsing" => {
            serde_json::json!({
                "domain": row.try_get::<String, _>("domain").ok(),
                "page_title": row.try_get::<String, _>("page_title").ok(),
            })
        }
        "content_conversation" => {
            let content: Option<String> = row.try_get("content").ok();
            let preview = content.map(|c| {
                let truncated: String = c.chars().take(100).collect();
                if truncated.len() < c.len() {
                    format!("{truncated}...")
                } else {
                    truncated
                }
            });
            serde_json::json!({
                "role": row.try_get::<String, _>("role").ok(),
                "provider": row.try_get::<String, _>("provider").ok(),
                "preview": preview,
            })
        }
        "content_document" => {
            serde_json::json!({
                "title": row.try_get::<String, _>("title").ok(),
                "document_type": row.try_get::<String, _>("document_type").ok(),
            })
        }
        "communication_transcription" => {
            let text: Option<String> = row.try_get("text").ok();
            let preview = text.map(|t| {
                let truncated: String = t.chars().take(100).collect();
                if truncated.len() < t.len() {
                    format!("{truncated}...")
                } else {
                    truncated
                }
            });
            serde_json::json!({
                "duration_seconds": row.try_get::<f64, _>("duration_seconds").ok(),
                "preview": preview,
            })
        }
        _ => {
            // Generic fallback - just return empty object
            serde_json::json!({})
        }
    }
}

// ============================================================================
// Day Chats - In-app Virtues chats + external AI conversations
// ============================================================================

/// A single chat conversation surfaced on a day's wiki page.
///
/// Unifies two sources:
/// - In-app Virtues chats (table: `chats`) — navigable, source = "virtues"
/// - External AI conversations from ontology imports (table:
///   `data_content_conversation`) — Claude.ai, Gemini, ChatGPT, etc.
///   Not navigable; only displayed with a provider badge.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DayChat {
    pub id: String,
    /// "virtues" for in-app chats, "external" for ontology-imported chats.
    pub source: String,
    /// External provider name (e.g. "claude", "gemini", "chatgpt"). None for in-app.
    pub provider: Option<String>,
    pub title: String,
    pub message_count: i64,
    pub started_at: DateTime<Utc>,
}

/// Get all AI chats (in-app + external) that started on the given day.
///
/// Day window matches `get_day_sources`: UTC midnight → noon next day,
/// which covers every timezone.
pub async fn get_day_chats(pool: &PgPool, date: NaiveDate) -> Result<Vec<DayChat>> {
    use sqlx::Row;

    let start_of_day = date.and_hms_opt(0, 0, 0).unwrap().and_utc();
    let end_of_day = date
        .succ_opt()
        .unwrap()
        .and_hms_opt(12, 0, 0)
        .unwrap()
        .and_utc();
    let mut chats: Vec<DayChat> = Vec::new();

    // ── In-app Virtues chats ────────────────────────────────────────────────
    let in_app_rows = sqlx::query(
        r#"
        SELECT id, title, message_count, created_at
        FROM app_chats
        WHERE created_at >= $1 AND created_at <= $2
        ORDER BY created_at ASC
        "#,
    )
    .bind(start_of_day)
    .bind(end_of_day)
    .fetch_all(pool)
    .await
    .map_err(|e| Error::Database(format!("Failed to query in-app chats: {}", e)))?;

    for row in &in_app_rows {
        let id: String = match row.try_get("id") {
            Ok(v) => v,
            Err(_) => continue,
        };
        let title: String = row.try_get("title").unwrap_or_else(|_| "Untitled chat".to_string());
        let message_count: i64 = row.try_get("message_count").unwrap_or(0);
        let started_at: DateTime<Utc> = match row.try_get("created_at") {
            Ok(v) => v,
            Err(_) => continue,
        };
        chats.push(DayChat {
            id,
            source: "virtues".to_string(),
            provider: None,
            title,
            message_count,
            started_at,
        });
    }

    // ── External AI conversations (ontology-imported) ───────────────────────
    // Group messages by conversation_id in Rust to keep SQL simple. Excludes
    // any rows with source_provider='virtues' so we don't double-count an
    // in-app chat that was also synced into the ontology lake.
    let ext_rows = sqlx::query(
        r#"
        SELECT conversation_id, role, content, provider, occurred_at
        FROM data_content_conversation
        WHERE occurred_at >= $1 AND occurred_at <= $2
          AND source_provider != 'virtues'
        ORDER BY conversation_id, occurred_at ASC
        "#,
    )
    .bind(start_of_day)
    .bind(end_of_day)
    .fetch_all(pool)
    .await
    .map_err(|e| Error::Database(format!("Failed to query external chats: {}", e)))?;

    use std::collections::BTreeMap;
    struct ExtAccum {
        provider: Option<String>,
        first_ts: Option<DateTime<Utc>>,
        first_user_content: Option<String>,
        count: i64,
    }
    let mut groups: BTreeMap<String, ExtAccum> = BTreeMap::new();

    for row in &ext_rows {
        let conv_id: String = match row.try_get("conversation_id") {
            Ok(v) => v,
            Err(_) => continue,
        };
        let role: String = row.try_get("role").unwrap_or_default();
        let content: String = row.try_get("content").unwrap_or_default();
        let provider: Option<String> = row.try_get("provider").ok();
        // `timestamp` is a TIMESTAMPTZ column — decode directly. Reading it as
        // String failed at decode and `continue`d past every message, so
        // external AI conversations never showed up on the day page.
        let ts: DateTime<Utc> = match row.try_get("occurred_at") {
            Ok(v) => v,
            Err(_) => continue,
        };

        let entry = groups.entry(conv_id).or_insert(ExtAccum {
            provider: None,
            first_ts: None,
            first_user_content: None,
            count: 0,
        });
        entry.count += 1;
        if entry.provider.is_none() {
            entry.provider = provider;
        }
        if entry.first_ts.map(|t| ts < t).unwrap_or(true) {
            entry.first_ts = Some(ts);
        }
        if role == "user" && entry.first_user_content.is_none() && !content.trim().is_empty() {
            entry.first_user_content = Some(content);
        }
    }

    for (conv_id, acc) in groups {
        let started_at = match acc.first_ts {
            Some(t) => t,
            None => continue,
        };
        let title = acc
            .first_user_content
            .as_deref()
            .map(truncate_title)
            .unwrap_or_else(|| match acc.provider.as_deref() {
                Some(p) => format!("{} conversation", p),
                None => "AI conversation".to_string(),
            });
        chats.push(DayChat {
            id: conv_id,
            source: "external".to_string(),
            provider: acc.provider,
            title,
            message_count: acc.count,
            started_at,
        });
    }

    chats.sort_by(|a, b| a.started_at.cmp(&b.started_at));
    Ok(chats)
}

/// Truncate the first user message to a short, single-line title.
fn truncate_title(s: &str) -> String {
    let first_line = s.lines().next().unwrap_or("").trim();
    let chars: Vec<char> = first_line.chars().collect();
    if chars.len() <= 80 {
        first_line.to_string()
    } else {
        let truncated: String = chars.iter().take(80).collect();
        format!("{}…", truncated.trim_end())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The whole point of normalizing is that 0037 matches with
    /// `aliases ? lower(surface)`. An alias stored with capitals or padding is
    /// not "slightly wrong" — it is invisible to the resolver, so it looks
    /// saved and resolves nothing. These cases are the ones a human actually
    /// types.
    #[test]
    fn aliases_are_lowercased_trimmed_and_deduped() {
        let input = vec![
            "  Sarah ".to_string(),
            "SARAH".to_string(), // same surface, different case
            "sarah".to_string(), // exact duplicate
            "Mum".to_string(),
            "   ".to_string(), // whitespace only
            "".to_string(),
        ];
        let out = normalize_aliases(Some(&input)).expect("some input");
        assert_eq!(out, serde_json::json!(["sarah", "mum"]));
    }

    /// `None` must stay `None`: the update statement is
    /// `aliases = COALESCE($n, aliases)`, so a null leaves the column alone.
    /// Returning an empty array instead would silently erase every alias on
    /// any request that simply did not mention them.
    #[test]
    fn absent_aliases_do_not_clear_the_column() {
        assert!(normalize_aliases(None).is_none());
    }

    /// Clearing has to remain possible, and is distinct from "not mentioned".
    #[test]
    fn an_explicit_empty_list_clears() {
        let empty: Vec<String> = vec![];
        assert_eq!(
            normalize_aliases(Some(&empty)).expect("some"),
            serde_json::json!([])
        );
    }
}
