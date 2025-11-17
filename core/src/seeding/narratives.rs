//! Narrative chunks seeding with embeddings
//!
//! Generates realistic narrative chunks linked to ontology primitives

use crate::database::Database;
use crate::Result;
use chrono::{Duration, Utc};
use rand::Rng;
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

/// Seeds ~100 narrative chunks with hierarchical structure
pub async fn seed_narratives(db: &Database) -> Result<usize> {
    let pool = db.pool();
    let mut total_count = 0;
    let mut rng = rand::thread_rng();
    let now = Utc::now();

    // Get some ontology primitive IDs to reference
    let health_ids = get_sample_ontology_ids(pool, "health_heart_rate", 50).await?;
    let sleep_ids = get_sample_ontology_ids(pool, "health_sleep", 30).await?;
    let workout_ids = get_sample_ontology_ids(pool, "health_workout", 15).await?;
    let email_ids = get_sample_ontology_ids(pool, "social_email", 100).await?;
    let message_ids = get_sample_ontology_ids(pool, "social_message", 100).await?;
    let location_ids = get_sample_ontology_ids(pool, "location_point", 100).await?;
    let calendar_ids = get_sample_ontology_ids(pool, "activity_calendar_entry", 50).await?;

    // Generate narratives for the last 7 days
    for day_offset in 0..7 {
        let day_start = (now - Duration::days(day_offset))
            .date_naive()
            .and_hms_opt(0, 0, 0)
            .unwrap()
            .and_utc();
        let day_end = day_start + Duration::days(1);

        // Create a day-level narrative
        let day_narrative_text = format!(
            "Day {} of your journey. Started with morning exercise and heart rate monitoring. \
             Throughout the day, you maintained communication with colleagues via email and messages. \
             Your location data shows regular movement patterns. Ended the day with {} steps.",
            day_start.format("%B %d, %Y"),
            rng.gen_range(6000..12000)
        );

        // Build ontology reference for the day (aggregate of various primitives)
        let day_ontology_refs = json!({
            "health_heart_rate": sample_ids(&health_ids, &mut rng, 5),
            "health_sleep": sample_ids(&sleep_ids, &mut rng, 1),
            "social_email": sample_ids(&email_ids, &mut rng, 5),
            "social_message": sample_ids(&message_ids, &mut rng, 10),
            "location_point": sample_ids(&location_ids, &mut rng, 5),
        });

        let embedding_vec = generate_placeholder_embedding();
        let embedding_str = format_embedding(&embedding_vec);

        let day_narrative_id: Uuid = sqlx::query_scalar(
            r#"
            INSERT INTO data.narrative_chunks
            (narrative_text, narrative_type, time_start, time_end, time_granularity,
             ontology_primitive_ids, embedding, token_count, confidence_score,
             generation_model, generated_by)
            VALUES ($1, $2, $3, $4, $5, $6, $7::vector, $8, $9, $10, $11)
            RETURNING id
            "#,
        )
        .bind(&day_narrative_text)
        .bind("day")
        .bind(day_start)
        .bind(day_end)
        .bind("day")
        .bind(&day_ontology_refs)
        .bind(&embedding_str)
        .bind(count_tokens(&day_narrative_text))
        .bind(rng.gen_range(0.85..0.95))
        .bind("seed-generator-v1")
        .bind("seed_script")
        .fetch_one(pool)
        .await?;

        total_count += 1;

        // Create 3-5 event-level narratives for this day
        let event_count = rng.gen_range(3..6);
        let mut event_ids = Vec::new();

        for event_idx in 0..event_count {
            let event_start = day_start + Duration::hours((event_idx * 4 + 8) as i64);
            let event_duration = rng.gen_range(1..4);
            let event_end = event_start + Duration::hours(event_duration);

            let event_narratives = vec![
                "Morning workout session. Heart rate elevated to cardio zone. Completed a 5km run followed by stretching.",
                "Mid-morning work block. Responded to several emails from the team. Calendar shows three scheduled meetings.",
                "Lunch break and location change. Moved to a nearby restaurant. Brief messaging with friends.",
                "Afternoon deep work session. Focus on project deliverables. Multiple app switches between development tools.",
                "Evening wind-down. Light activity and meal preparation. Heart rate returning to resting baseline.",
                "Social interaction time. Phone calls and messages with family and friends. Sharing updates about the day.",
            ];

            let event_text = if event_idx < event_narratives.len() {
                event_narratives[event_idx].to_string()
            } else {
                format!(
                    "Event {} of the day. Various activities and interactions.",
                    event_idx + 1
                )
            };

            // Different events reference different types of primitives
            let event_ontology_refs = match event_idx {
                0 => json!({
                    // Morning workout
                    "health_workout": sample_ids(&workout_ids, &mut rng, 1),
                    "health_heart_rate": sample_ids(&health_ids, &mut rng, 3),
                    "location_point": sample_ids(&location_ids, &mut rng, 2),
                }),
                1 => json!({
                    // Work emails
                    "social_email": sample_ids(&email_ids, &mut rng, 3),
                    "activity_calendar_entry": sample_ids(&calendar_ids, &mut rng, 2),
                }),
                2 => json!({
                    // Lunch
                    "location_point": sample_ids(&location_ids, &mut rng, 2),
                    "social_message": sample_ids(&message_ids, &mut rng, 3),
                }),
                3 => json!({
                    // Afternoon work
                    "social_email": sample_ids(&email_ids, &mut rng, 2),
                    "activity_calendar_entry": sample_ids(&calendar_ids, &mut rng, 1),
                }),
                _ => json!({
                    // Evening
                    "health_heart_rate": sample_ids(&health_ids, &mut rng, 2),
                    "social_message": sample_ids(&message_ids, &mut rng, 5),
                }),
            };

            let event_embedding_vec = generate_placeholder_embedding();
            let event_embedding_str = format_embedding(&event_embedding_vec);

            let event_id: Uuid = sqlx::query_scalar(
                r#"
                INSERT INTO data.narrative_chunks
                (narrative_text, narrative_type, time_start, time_end, time_granularity,
                 parent_narrative_id, ontology_primitive_ids, embedding, token_count,
                 confidence_score, generation_model, generated_by)
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8::vector, $9, $10, $11, $12)
                RETURNING id
                "#,
            )
            .bind(&event_text)
            .bind("event")
            .bind(event_start)
            .bind(event_end)
            .bind("hour")
            .bind(day_narrative_id)
            .bind(&event_ontology_refs)
            .bind(&event_embedding_str)
            .bind(count_tokens(&event_text))
            .bind(rng.gen_range(0.80..0.92))
            .bind("seed-generator-v1")
            .bind("seed_script")
            .fetch_one(pool)
            .await?;

            event_ids.push(event_id);
            total_count += 1;

            // Create 2-3 action-level narratives for some events
            if event_idx % 2 == 0 {
                let action_count = rng.gen_range(2..4);
                let mut action_ids = Vec::new();

                for action_idx in 0..action_count {
                    let action_start = event_start + Duration::minutes((action_idx * 20) as i64);
                    let action_end = action_start + Duration::minutes(rng.gen_range(5..20));

                    let action_texts = vec![
                        "Checked heart rate - elevated but within normal range for activity level.",
                        "Sent quick email response to team member about project timeline.",
                        "Received incoming message - replied immediately.",
                        "Location logged - moved from previous spot.",
                        "Brief calendar check - upcoming meeting in 30 minutes.",
                    ];

                    let action_text = if action_idx < action_texts.len() {
                        action_texts[action_idx]
                    } else {
                        "Brief action or micro-interaction."
                    };

                    // Actions reference very specific primitives (1-2 items)
                    let action_ontology_refs = match action_idx % 4 {
                        0 => json!({"health_heart_rate": sample_ids(&health_ids, &mut rng, 1)}),
                        1 => json!({"social_email": sample_ids(&email_ids, &mut rng, 1)}),
                        2 => json!({"social_message": sample_ids(&message_ids, &mut rng, 1)}),
                        _ => json!({"location_point": sample_ids(&location_ids, &mut rng, 1)}),
                    };

                    let action_embedding_vec = generate_placeholder_embedding();
                    let action_embedding_str = format_embedding(&action_embedding_vec);

                    let action_id: Uuid = sqlx::query_scalar(
                        r#"
                        INSERT INTO data.narrative_chunks
                        (narrative_text, narrative_type, time_start, time_end, time_granularity,
                         parent_narrative_id, ontology_primitive_ids, embedding, token_count,
                         confidence_score, generation_model, generated_by)
                        VALUES ($1, $2, $3, $4, $5, $6, $7, $8::vector, $9, $10, $11, $12)
                        RETURNING id
                        "#,
                    )
                    .bind(action_text)
                    .bind("action")
                    .bind(action_start)
                    .bind(action_end)
                    .bind("minute")
                    .bind(event_id)
                    .bind(&action_ontology_refs)
                    .bind(&action_embedding_str)
                    .bind(count_tokens(action_text))
                    .bind(rng.gen_range(0.75..0.88))
                    .bind("seed-generator-v1")
                    .bind("seed_script")
                    .fetch_one(pool)
                    .await?;

                    action_ids.push(action_id);
                    total_count += 1;
                }

                // Update event with child action IDs
                sqlx::query(
                    r#"
                    UPDATE data.narrative_chunks
                    SET child_narrative_ids = $1
                    WHERE id = $2
                    "#,
                )
                .bind(&action_ids)
                .bind(event_id)
                .execute(pool)
                .await?;
            }
        }

        // Update day narrative with child event IDs
        sqlx::query(
            r#"
            UPDATE data.narrative_chunks
            SET child_narrative_ids = $1
            WHERE id = $2
            "#,
        )
        .bind(&event_ids)
        .bind(day_narrative_id)
        .execute(pool)
        .await?;
    }

    // Create one week-level narrative that aggregates the 7 days
    let week_start = (now - Duration::days(6))
        .date_naive()
        .and_hms_opt(0, 0, 0)
        .unwrap()
        .and_utc();
    let week_end = now.date_naive().and_hms_opt(23, 59, 59).unwrap().and_utc();

    let week_narrative_text = format!(
        "Week summary from {} to {}. Maintained consistent exercise routine with {} workouts. \
         Stayed connected through {} emails and {} messages. Sleep patterns remained stable. \
         Overall health metrics within target ranges.",
        week_start.format("%B %d"),
        week_end.format("%B %d"),
        workout_ids.len().min(10),
        email_ids.len().min(50),
        message_ids.len().min(100)
    );

    let week_ontology_refs = json!({
        "health_heart_rate": sample_ids(&health_ids, &mut rng, 10),
        "health_sleep": sample_ids(&sleep_ids, &mut rng, 7),
        "health_workout": sample_ids(&workout_ids, &mut rng, 5),
        "social_email": sample_ids(&email_ids, &mut rng, 15),
        "social_message": sample_ids(&message_ids, &mut rng, 30),
        "location_point": sample_ids(&location_ids, &mut rng, 15),
        "activity_calendar_entry": sample_ids(&calendar_ids, &mut rng, 10),
    });

    let week_embedding_vec = generate_placeholder_embedding();
    let week_embedding_str = format_embedding(&week_embedding_vec);

    sqlx::query(
        r#"
        INSERT INTO data.narrative_chunks
        (narrative_text, narrative_type, time_start, time_end, time_granularity,
         ontology_primitive_ids, embedding, token_count, confidence_score,
         generation_model, generated_by)
        VALUES ($1, $2, $3, $4, $5, $6, $7::vector, $8, $9, $10, $11)
        "#,
    )
    .bind(&week_narrative_text)
    .bind("week")
    .bind(week_start)
    .bind(week_end)
    .bind("week")
    .bind(&week_ontology_refs)
    .bind(&week_embedding_str)
    .bind(count_tokens(&week_narrative_text))
    .bind(rng.gen_range(0.88..0.96))
    .bind("seed-generator-v1")
    .bind("seed_script")
    .execute(pool)
    .await?;

    total_count += 1;

    Ok(total_count)
}

/// Get sample ontology primitive IDs from a table
/// Returns empty vector if table doesn't exist or has no data
async fn get_sample_ontology_ids(pool: &PgPool, table: &str, limit: i64) -> Result<Vec<Uuid>> {
    // Use dynamic query since we're querying different tables
    let query = format!(
        "SELECT id FROM data.{} WHERE source_provider = 'seed' ORDER BY created_at DESC LIMIT $1",
        table
    );

    let ids: Vec<Uuid> = match sqlx::query_scalar(&query).bind(limit).fetch_all(pool).await {
        Ok(ids) => ids,
        Err(e) => {
            // Log warning and return empty vector if table doesn't exist or query fails
            tracing::warn!(
                "Could not fetch ontology IDs from data.{}: {}. Skipping this primitive type.",
                table,
                e
            );
            vec![]
        }
    };

    Ok(ids)
}

/// Sample N random IDs from a list
fn sample_ids(ids: &[Uuid], rng: &mut impl Rng, count: usize) -> Vec<Uuid> {
    if ids.is_empty() {
        return vec![];
    }

    let actual_count = count.min(ids.len());
    let mut sampled = Vec::new();

    for _ in 0..actual_count {
        let idx = rng.gen_range(0..ids.len());
        sampled.push(ids[idx]);
    }

    sampled
}

/// Generate a placeholder embedding vector (1536 dimensions)
/// In a real implementation, this would call an embedding API
fn generate_placeholder_embedding() -> Vec<f32> {
    let mut rng = rand::thread_rng();
    // Generate random normalized vector
    let mut vec: Vec<f32> = (0..1536).map(|_| rng.gen_range(-1.0..1.0)).collect();

    // Normalize to unit length (as real embeddings would be)
    let magnitude: f32 = vec.iter().map(|x| x * x).sum::<f32>().sqrt();
    if magnitude > 0.0 {
        vec.iter_mut().for_each(|x| *x /= magnitude);
    }

    vec
}

/// Rough token count estimation (4 characters ≈ 1 token)
fn count_tokens(text: &str) -> i32 {
    (text.len() / 4).max(1) as i32
}

/// Format embedding vector for PostgreSQL pgvector type
/// Converts Vec<f32> to string like "[0.1, 0.2, 0.3, ...]"
fn format_embedding(vec: &[f32]) -> String {
    format!(
        "[{}]",
        vec.iter()
            .map(|x| x.to_string())
            .collect::<Vec<_>>()
            .join(",")
    )
}

/// Seed the Monday in Rome day narrative from CSV
pub async fn seed_rome_monday_narrative(db: &Database) -> Result<usize> {
    let pool = db.pool();

    tracing::info!("Seeding Rome Monday narrative...");

    // Hardcode the narrative for now - simpler than CSV parsing with commas in text
    let narrative_text = "Monday November 10, 2025 in Rome, Italy. Attended the AI & Ethics conference at the Vatican. Around mid-morning, randomly ran into Jacob from my venture cohort at a café. He was with his wife, Lucy. We ordered croissants and coffee while catching up. Franklin mentioned he's from the San Antonio area in Texas and was heading back to Dallas after the conference. It was great to finally meet in person after knowing each other through the cohort. Later in the day, enjoyed local wine (Montepulciano) and continued exploring Rome's cafés and restaurants.";
    let narrative_type = "day";
    let time_granularity = "day";
    let confidence_score = 0.90;
    let generation_model = "seed-manual-v1";
    let generated_by = "manual_seed_adam";
    let generation_prompt_version = "v1";

    let mut count = 0;

    // Get microphone transcription IDs to link
    let microphone_ids = get_sample_ontology_ids(pool, "ios_microphone_transcription", 10).await?;

    // Build ontology references
    let ontology_refs = json!({
        "ios_microphone_transcription": microphone_ids,
    });

    // Skip embedding for now - can generate later if needed
    // Generate a zero vector for now (pgvector requires a vector)
    let embedding_str = format!("[{}]", vec!["0.0"; 1536].join(","));

    // Parse timestamps - hardcode for now since we know the format
    let time_start_parsed = chrono::NaiveDate::from_ymd_opt(2025, 11, 10)
        .unwrap()
        .and_hms_opt(0, 0, 0)
        .unwrap()
        .and_utc();

    let time_end_parsed = chrono::NaiveDate::from_ymd_opt(2025, 11, 10)
        .unwrap()
        .and_hms_opt(23, 59, 59)
        .unwrap()
        .and_utc();

    // Insert narrative chunk
    sqlx::query(
        r#"
        INSERT INTO data.narrative_chunks
        (narrative_text, narrative_type, time_start, time_end, time_granularity,
         ontology_primitive_ids, embedding, token_count, confidence_score,
         generation_model, generated_by, generation_prompt_version)
        VALUES ($1, $2, $3, $4, $5, $6, $7::vector, $8, $9, $10, $11, $12)
        "#,
    )
    .bind(narrative_text)
    .bind(narrative_type)
    .bind(time_start_parsed)
    .bind(time_end_parsed)
    .bind(time_granularity)
    .bind(&ontology_refs)
    .bind(&embedding_str)
    .bind(count_tokens(narrative_text))
    .bind(confidence_score)
    .bind(generation_model)
    .bind(generated_by)
    .bind(generation_prompt_version)
    .execute(pool)
    .await?;

    count += 1;
    tracing::info!("✓ Seeded Rome Monday day narrative");

    Ok(count)
}
