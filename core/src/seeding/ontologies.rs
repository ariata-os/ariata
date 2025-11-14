//! Ontology domain primitives seeding
//!
//! Generates realistic test data for health, social, location, and activity domains

use crate::database::Database;
use crate::Result;
use chrono::{Duration, Utc};
use fake::faker::internet::en::SafeEmail;
use fake::faker::name::en::Name;
use fake::faker::phone_number::en::PhoneNumber;
use fake::Fake;
use rand::Rng;
use sqlx::PgPool;
use uuid::Uuid;

/// Seeds ~1000 ontology domain primitive records
pub async fn seed_ontologies(db: &Database) -> Result<usize> {
    let pool = db.pool();
    let mut total_count = 0;

    // Use a deterministic seed source UUID (ariata-app source from migrations)
    let ariata_source_id =
        Uuid::parse_str("00000000-0000-0000-0000-000000000001").expect("Valid UUID");

    // Get or create a test stream for seed data
    let seed_stream_id = get_or_create_seed_stream(pool, ariata_source_id).await?;

    // Seed health data (~400 records)
    total_count += seed_health_data(pool, seed_stream_id).await?;

    // Seed social data (~300 records)
    total_count += seed_social_data(pool, seed_stream_id).await?;

    // Seed location data (~200 records)
    total_count += seed_location_data(pool, seed_stream_id).await?;

    // Seed activity data (~100 records)
    total_count += seed_activity_data(pool, seed_stream_id).await?;

    Ok(total_count)
}

/// Get or create a stream for seed data
async fn get_or_create_seed_stream(pool: &PgPool, source_id: Uuid) -> Result<Uuid> {
    let stream_id = sqlx::query_scalar!(
        r#"
        INSERT INTO elt.streams (source_id, stream_name, table_name, is_enabled)
        VALUES ($1, 'seed_data', 'stream_seed_data', true)
        ON CONFLICT (source_id, stream_name)
        DO UPDATE SET updated_at = NOW()
        RETURNING id
        "#,
        source_id
    )
    .fetch_one(pool)
    .await?;

    Ok(stream_id)
}

/// Seeds health domain primitives
async fn seed_health_data(pool: &PgPool, stream_id: Uuid) -> Result<usize> {
    let mut count = 0;
    let now = Utc::now();
    let mut rng = rand::thread_rng();

    // Generate 30 days of health data
    for day in 0..30 {
        let base_time = now - Duration::days(day);

        // Heart rate measurements (every 15 minutes during waking hours = ~64 per day, ~100 total for variety)
        for hour in 7..23 {
            for minute in (0..60).step_by(15) {
                let timestamp = base_time
                    .date_naive()
                    .and_hms_opt(hour, minute, 0)
                    .unwrap()
                    .and_utc();

                let bpm = match hour {
                    7..=9 => rng.gen_range(65..85),   // Morning
                    10..=12 => rng.gen_range(70..90), // Mid-morning
                    13..=17 => rng.gen_range(75..95), // Afternoon
                    18..=22 => rng.gen_range(65..80), // Evening
                    _ => rng.gen_range(60..75),
                };

                let context = match hour {
                    7..=9 => "active",
                    10..=17 => "resting",
                    18..=19 => "workout",
                    _ => "recovery",
                };

                sqlx::query!(
                    r#"
                    INSERT INTO elt.health_heart_rate
                    (bpm, measurement_context, timestamp, source_stream_id, source_table, source_provider)
                    VALUES ($1, $2, $3, $4, 'stream_seed_data', 'seed')
                    ON CONFLICT DO NOTHING
                    "#,
                    bpm,
                    context,
                    timestamp,
                    stream_id
                )
                .execute(pool)
                .await?;

                count += 1;
            }
        }

        // Sleep records (1 per day = 30 total)
        let sleep_start = base_time
            .date_naive()
            .and_hms_opt(23, 0, 0)
            .unwrap()
            .and_utc();
        let sleep_duration = rng.gen_range(360..540); // 6-9 hours
        let sleep_end = sleep_start + Duration::minutes(sleep_duration);

        sqlx::query!(
            r#"
            INSERT INTO elt.health_sleep
            (total_duration_minutes, sleep_quality_score, start_time, end_time,
             source_stream_id, source_table, source_provider)
            VALUES ($1, $2, $3, $4, $5, 'stream_seed_data', 'seed')
            ON CONFLICT DO NOTHING
            "#,
            sleep_duration as i32,
            rng.gen_range(0.6..1.0),
            sleep_start,
            sleep_end,
            stream_id
        )
        .execute(pool)
        .await?;

        count += 1;

        // Workout records (every 2-3 days = ~12 total)
        if day % 2 == 0 || day % 3 == 0 {
            let workout_start = base_time
                .date_naive()
                .and_hms_opt(18, 0, 0)
                .unwrap()
                .and_utc();
            let workout_duration = rng.gen_range(30..90);
            let workout_end = workout_start + Duration::minutes(workout_duration);

            let activities = vec!["running", "cycling", "swimming", "weightlifting", "yoga"];
            let intensities = vec!["moderate", "high", "max"];

            sqlx::query!(
                r#"
                INSERT INTO elt.health_workout
                (activity_type, intensity, calories_burned, average_heart_rate,
                 start_time, end_time, source_stream_id, source_table, source_provider)
                VALUES ($1, $2, $3, $4, $5, $6, $7, 'stream_seed_data', 'seed')
                ON CONFLICT DO NOTHING
                "#,
                activities[rng.gen_range(0..activities.len())],
                intensities[rng.gen_range(0..intensities.len())],
                rng.gen_range(200..800),
                rng.gen_range(120..170),
                workout_start,
                workout_end,
                stream_id
            )
            .execute(pool)
            .await?;

            count += 1;
        }

        // Steps (1 per day = 30 total)
        let steps_time = base_time
            .date_naive()
            .and_hms_opt(23, 59, 0)
            .unwrap()
            .and_utc();
        sqlx::query!(
            r#"
            INSERT INTO elt.health_steps
            (step_count, timestamp, source_stream_id, source_table, source_provider)
            VALUES ($1, $2, $3, 'stream_seed_data', 'seed')
            ON CONFLICT DO NOTHING
            "#,
            rng.gen_range(4000..15000),
            steps_time,
            stream_id
        )
        .execute(pool)
        .await?;

        count += 1;
    }

    Ok(count)
}

/// Seeds social domain primitives
async fn seed_social_data(pool: &PgPool, stream_id: Uuid) -> Result<usize> {
    let mut count = 0;
    let now = Utc::now();
    let mut rng = rand::thread_rng();

    // Generate 30 days of social data
    for day in 0..30 {
        let base_time = now - Duration::days(day);

        // Email messages (5-10 per day = ~225 total)
        let email_count = rng.gen_range(5..11);
        for i in 0..email_count {
            let timestamp = base_time
                .date_naive()
                .and_hms_opt(rng.gen_range(8..20), rng.gen_range(0..60), 0)
                .unwrap()
                .and_utc();

            let subjects = vec![
                "Weekly sync",
                "Project update",
                "Quick question",
                "Follow up",
                "Meeting notes",
                "FYI",
                "Action items",
            ];

            let directions = vec!["sent", "received"];
            sqlx::query!(
                r#"
                INSERT INTO elt.social_email
                (message_id, subject, from_address, from_name, to_addresses, direction, timestamp,
                 source_stream_id, source_table, source_provider)
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, 'stream_seed_data', 'seed')
                ON CONFLICT DO NOTHING
                "#,
                format!("seed-email-{}-{}", day, i),
                subjects[rng.gen_range(0..subjects.len())],
                SafeEmail().fake::<String>(),
                Name().fake::<String>(),
                &vec![SafeEmail().fake::<String>()],
                directions[rng.gen_range(0..directions.len())],
                timestamp,
                stream_id
            )
            .execute(pool)
            .await?;

            count += 1;
        }

        // Messages (10-20 per day = ~450 total)
        let message_count = rng.gen_range(10..21);
        for i in 0..message_count {
            let timestamp = base_time
                .date_naive()
                .and_hms_opt(rng.gen_range(8..23), rng.gen_range(0..60), 0)
                .unwrap()
                .and_utc();

            let channels = vec!["sms", "imessage", "whatsapp", "slack"];
            let directions = vec!["sent", "received"];

            sqlx::query!(
                r#"
                INSERT INTO elt.social_message
                (message_id, channel, body, from_identifier, direction, timestamp,
                 source_stream_id, source_table, source_provider)
                VALUES ($1, $2, $3, $4, $5, $6, $7, 'stream_seed_data', 'seed')
                ON CONFLICT DO NOTHING
                "#,
                format!("seed-msg-{}-{}", day, i),
                channels[rng.gen_range(0..channels.len())],
                "Sample message content",
                PhoneNumber().fake::<String>(),
                directions[rng.gen_range(0..directions.len())],
                timestamp,
                stream_id
            )
            .execute(pool)
            .await?;

            count += 1;
        }

        // Calls (1-3 per day = ~60 total)
        let call_count = rng.gen_range(1..4);
        for _ in 0..call_count {
            let start_time = base_time
                .date_naive()
                .and_hms_opt(rng.gen_range(9..20), rng.gen_range(0..60), 0)
                .unwrap()
                .and_utc();
            let duration = rng.gen_range(60..1800); // 1-30 minutes
            let end_time = start_time + Duration::seconds(duration);

            let call_types = vec!["voice", "video"];
            let directions = vec!["incoming", "outgoing"];
            let statuses = vec!["answered", "missed", "declined"];

            sqlx::query!(
                r#"
                INSERT INTO elt.social_call
                (call_type, direction, call_status, caller_identifier, duration_seconds,
                 start_time, end_time, source_stream_id, source_table, source_provider)
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, 'stream_seed_data', 'seed')
                ON CONFLICT DO NOTHING
                "#,
                call_types[rng.gen_range(0..call_types.len())],
                directions[rng.gen_range(0..directions.len())],
                statuses[rng.gen_range(0..statuses.len())],
                PhoneNumber().fake::<String>(),
                duration as i32,
                start_time,
                end_time,
                stream_id
            )
            .execute(pool)
            .await?;

            count += 1;
        }
    }

    Ok(count)
}

/// Seeds location domain primitives
async fn seed_location_data(pool: &PgPool, stream_id: Uuid) -> Result<usize> {
    let mut count = 0;
    let now = Utc::now();
    let mut rng = rand::thread_rng();

    // Base coordinates (San Francisco area)
    let base_lat = 37.7749;
    let base_lon = -122.4194;

    // Generate 30 days of location data
    for day in 0..30 {
        let base_time = now - Duration::days(day);

        // Location points (every 30 minutes = ~48 per day, sample 5 to keep it reasonable = 150 total)
        for hour in [9, 12, 15, 18, 21].iter() {
            let timestamp = base_time
                .date_naive()
                .and_hms_opt(*hour, 0, 0)
                .unwrap()
                .and_utc();

            // Add some random variation
            let lat = base_lat + rng.gen_range(-0.05..0.05);
            let lon = base_lon + rng.gen_range(-0.05..0.05);

            sqlx::query!(
                r#"
                INSERT INTO elt.location_point
                (latitude, longitude, coordinates, accuracy_meters, timestamp,
                 source_stream_id, source_table, source_provider)
                VALUES ($1, $2, ST_GeogFromText($3), $4, $5, $6, 'stream_seed_data', 'seed')
                ON CONFLICT DO NOTHING
                "#,
                lat,
                lon,
                format!("POINT({} {})", lon, lat),
                rng.gen_range(5.0..50.0),
                timestamp,
                stream_id
            )
            .execute(pool)
            .await?;

            count += 1;
        }

        // TODO: Location visits seeding - disabled until location_visit table is created
        // See TODO.md for implementing location_point → location_visit transformation
        // Once the table exists and transform is implemented, uncomment this section:
        //
        // // Location visits (1-2 per day = ~45 total)
        // let visit_count = rng.gen_range(1..3);
        // for _ in 0..visit_count {
        //     let start_time = base_time
        //         .date_naive()
        //         .and_hms_opt(rng.gen_range(9..20), 0, 0)
        //         .unwrap()
        //         .and_utc();
        //     let duration = rng.gen_range(30..180); // 30 mins to 3 hours
        //     let end_time = start_time + Duration::minutes(duration);
        //
        //     let lat = base_lat + rng.gen_range(-0.05..0.05);
        //     let lon = base_lon + rng.gen_range(-0.05..0.05);
        //
        //     sqlx::query!(
        //         r#"
        //         INSERT INTO elt.location_visit
        //         (latitude, longitude, centroid_coordinates, start_time, end_time,
        //          source_stream_id, source_table, source_provider)
        //         VALUES ($1, $2, ST_GeogFromText($3), $4, $5, $6, 'stream_seed_data', 'seed')
        //         ON CONFLICT DO NOTHING
        //         "#,
        //         lat,
        //         lon,
        //         format!("POINT({} {})", lon, lat),
        //         start_time,
        //         end_time,
        //         stream_id
        //     )
        //     .execute(pool)
        //     .await?;
        //
        //     count += 1;
        // }
    }

    Ok(count)
}

/// Seeds activity domain primitives
async fn seed_activity_data(pool: &PgPool, stream_id: Uuid) -> Result<usize> {
    let mut count = 0;
    let now = Utc::now();
    let mut rng = rand::thread_rng();

    // Generate 30 days of activity data
    for day in 0..30 {
        let base_time = now - Duration::days(day);

        // Calendar entries (2-4 per day = ~90 total)
        let cal_count = rng.gen_range(2..5);
        for _ in 0..cal_count {
            let start_time = base_time
                .date_naive()
                .and_hms_opt(rng.gen_range(9..17), 0, 0)
                .unwrap()
                .and_utc();
            let duration = rng.gen_range(30..120); // 30 mins to 2 hours
            let end_time = start_time + Duration::minutes(duration);

            let titles = vec![
                "Team standup",
                "1:1 meeting",
                "Project review",
                "Client call",
                "Planning session",
            ];

            sqlx::query!(
                r#"
                INSERT INTO elt.activity_calendar_entry
                (title, calendar_name, start_time, end_time, is_all_day,
                 source_stream_id, source_table, source_provider)
                VALUES ($1, $2, $3, $4, $5, $6, 'stream_seed_data', 'seed')
                ON CONFLICT DO NOTHING
                "#,
                titles[rng.gen_range(0..titles.len())],
                "Work",
                start_time,
                end_time,
                false,
                stream_id
            )
            .execute(pool)
            .await?;

            count += 1;
        }

        // App usage (5-10 per day = ~225 total)
        let app_count = rng.gen_range(5..11);
        for _ in 0..app_count {
            let start_time = base_time
                .date_naive()
                .and_hms_opt(rng.gen_range(8..22), rng.gen_range(0..60), 0)
                .unwrap()
                .and_utc();
            let duration = rng.gen_range(5..120); // 5 mins to 2 hours
            let end_time = start_time + Duration::minutes(duration);

            let apps = vec![
                ("VS Code", "code"),
                ("Chrome", "browsing"),
                ("Slack", "communication"),
                ("Terminal", "development"),
                ("Notion", "productivity"),
            ];
            let (app_name, app_category) = apps[rng.gen_range(0..apps.len())];

            sqlx::query!(
                r#"
                INSERT INTO elt.activity_app_usage
                (app_name, app_category, start_time, end_time,
                 source_stream_id, source_table, source_provider)
                VALUES ($1, $2, $3, $4, $5, 'stream_seed_data', 'seed')
                ON CONFLICT DO NOTHING
                "#,
                app_name,
                app_category,
                start_time,
                end_time,
                stream_id
            )
            .execute(pool)
            .await?;

            count += 1;
        }
    }

    Ok(count)
}
