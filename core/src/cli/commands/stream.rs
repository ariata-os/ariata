//! Stream command handlers - manage data streams for sources

use crate::cli::types::StreamCommands;
use crate::Ariata;
use crate::storage::stream_writer::StreamWriter;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Handle stream management commands
pub async fn handle_stream_command(
    ariata: Ariata,
    stream_writer: Arc<Mutex<StreamWriter>>,
    action: StreamCommands,
) -> Result<(), Box<dyn std::error::Error>> {
    match action {
        StreamCommands::List { source_id } => {
            let source_id = source_id.parse()?;
            let streams = crate::list_source_streams(ariata.database.pool(), source_id).await?;

            if streams.is_empty() {
                println!("No streams found for this source");
                return Ok(());
            }

            println!("Streams for source {}:", source_id);
            println!(
                "{:<20} {:<10} {:<25} {}",
                "Stream", "Status", "Schedule", "Last Sync"
            );
            println!("{}", "-".repeat(80));

            for stream in streams {
                let status = if stream.is_enabled {
                    "enabled"
                } else {
                    "disabled"
                };
                let schedule = stream.cron_schedule.unwrap_or_else(|| "manual".to_string());
                let last_sync = stream
                    .last_sync_at
                    .map(|t| t.format("%Y-%m-%d %H:%M").to_string())
                    .unwrap_or_else(|| "never".to_string());

                println!(
                    "{:<20} {:<10} {:<25} {}",
                    stream.stream_name, status, schedule, last_sync
                );
            }
        }

        StreamCommands::Show {
            source_id,
            stream_name,
        } => {
            let source_id = source_id.parse()?;
            let stream =
                crate::get_stream_info(ariata.database.pool(), source_id, &stream_name).await?;

            println!("Stream: {} / {}", source_id, stream.stream_name);
            println!("  Table: {}", stream.table_name);
            println!(
                "  Status: {}",
                if stream.is_enabled {
                    "enabled"
                } else {
                    "disabled"
                }
            );

            if let Some(schedule) = stream.cron_schedule {
                println!("  Schedule: {}", schedule);
            } else {
                println!("  Schedule: manual");
            }

            if let Some(last_sync) = stream.last_sync_at {
                println!("  Last Sync: {}", last_sync);
            } else {
                println!("  Last Sync: never");
            }

            // Show config if it's not an empty object
            if let serde_json::Value::Object(map) = &stream.config {
                if !map.is_empty() {
                    println!(
                        "  Config: {}",
                        serde_json::to_string_pretty(&stream.config)?
                    );
                }
            }
        }

        StreamCommands::Enable {
            source_id,
            stream_name,
        } => {
            let source_id_uuid = source_id.parse()?;

            println!("Enabling stream: {} / {}", source_id_uuid, stream_name);

            // Enable with default config (None = use defaults)
            crate::enable_stream(ariata.database.pool(), &*ariata.storage, stream_writer.clone(), source_id_uuid, &stream_name, None).await?;

            println!("✅ Stream enabled successfully");
        }

        StreamCommands::Disable {
            source_id,
            stream_name,
        } => {
            let source_id = source_id.parse()?;

            println!("Disabling stream: {} / {}", source_id, stream_name);

            crate::disable_stream(ariata.database.pool(), source_id, &stream_name).await?;

            println!("✅ Stream disabled successfully");
        }

        StreamCommands::Schedule {
            source_id,
            stream_name,
            cron,
        } => {
            let source_id = source_id.parse()?;

            if let Some(cron_schedule) = cron {
                println!(
                    "Setting schedule for {} / {}: {}",
                    source_id, stream_name, cron_schedule
                );
                crate::update_stream_schedule(
                    ariata.database.pool(),
                    source_id,
                    &stream_name,
                    Some(cron_schedule),
                )
                .await?;
                println!("✅ Schedule updated successfully");
            } else {
                println!("Clearing schedule for {} / {}", source_id, stream_name);
                crate::update_stream_schedule(
                    ariata.database.pool(),
                    source_id,
                    &stream_name,
                    None,
                )
                .await?;
                println!("✅ Schedule cleared (stream will be manual only)");
            }
        }

        StreamCommands::Sync {
            source_id,
            stream_name,
        } => {
            let source_id_uuid = source_id.parse()?;

            println!("Creating sync job for: {} / {}...", source_id_uuid, stream_name);

            // Use full refresh mode for all syncs
            // This ensures compatibility with streams that don't support incremental sync
            let sync_mode = crate::SyncMode::full_refresh();

            let response =
                crate::api::jobs::trigger_stream_sync(ariata.database.pool(), &*ariata.storage, stream_writer.clone(), source_id_uuid, &stream_name, Some(sync_mode)).await?;

            println!("\n✅ Sync job created!");
            println!("  Job ID: {}", response.job_id);
            println!("  Status: {}", response.status);
            println!("  Started at: {}", response.started_at);
            println!("\nNote: Job is running in the background. Use 'ariata jobs status {}' to monitor progress.", response.job_id);
        }

        StreamCommands::History {
            source_id,
            stream_name,
            limit,
        } => {
            let source_id = source_id.parse()?;

            let jobs = crate::api::jobs::get_job_history(
                ariata.database.pool(),
                source_id,
                &stream_name,
                limit,
            )
            .await?;

            if jobs.is_empty() {
                println!("No sync history found for this stream");
                return Ok(());
            }

            println!(
                "Sync History for {} / {} (showing {} most recent):",
                source_id,
                stream_name,
                jobs.len()
            );
            println!(
                "{:<26} {:<10} {:<10} {:<10} {}",
                "Started", "Status", "Records", "Duration", "Error"
            );
            println!("{}", "-".repeat(80));

            for job in jobs {
                let records = job.records_processed;
                let duration = if let (Some(completed), started) = (job.completed_at, job.started_at) {
                    let duration_ms = (completed - started).num_milliseconds();
                    format!("{}ms", duration_ms)
                } else {
                    "-".to_string()
                };
                let error = job
                    .error_message
                    .map(|e| {
                        if e.len() > 30 {
                            format!("{}...", &e[..27])
                        } else {
                            e
                        }
                    })
                    .unwrap_or_else(|| "-".to_string());

                println!(
                    "{} {:<10} {:<10} {:<10} {}",
                    job.started_at.format("%Y-%m-%d %H:%M:%S"),
                    job.status,
                    records,
                    duration,
                    error
                );
            }
        }

        StreamCommands::Transform {
            source_id,
            stream_name,
        } => {
            // Manual transform triggers are deprecated in the direct transform architecture
            eprintln!("❌ Manual transform triggers are not supported.");
            eprintln!("   Transforms are automatically triggered after sync jobs complete.");
            eprintln!("   To transform data for '{}', run a sync job instead:", stream_name);
            eprintln!("   ariata sync {}", source_id);
            return Err("Manual transforms not supported".into());
        }
    }

    Ok(())
}
