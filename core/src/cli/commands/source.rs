//! Source command handlers - manage data sources

use crate::cli::display::display_pending_pairings;
use crate::cli::types::SourceCommands;
use crate::Ariata;
use std::env;

/// Handle source management commands
pub async fn handle_source_command(
    ariata: Ariata,
    action: SourceCommands,
) -> Result<(), Box<dyn std::error::Error>> {
    match action {
        SourceCommands::List { pending } => {
            if pending {
                // Show pending pairings
                let pairings = crate::list_pending_pairings(ariata.database.pool()).await?;
                display_pending_pairings(&pairings);
            } else {
                // Show all sources
                let sources = crate::list_sources(ariata.database.pool()).await?;

                if sources.is_empty() {
                    println!("No sources configured");
                    return Ok(());
                }

                println!("Configured Sources:");
                println!("{:<38} {:<20} {:<15} {}", "ID", "Name", "Type", "Status");
                println!("{}", "-".repeat(80));

                for source in sources {
                    let status = if source.is_active {
                        "active"
                    } else {
                        "inactive"
                    };
                    println!(
                        "{} {:<20} {:<15} {}",
                        source.id, source.name, source.source, status
                    );
                }
            }
        }

        SourceCommands::Show { id } => {
            let source_id = id.parse()?;

            // Check if this is a pending pairing
            let pairing_status =
                crate::check_pairing_status(ariata.database.pool(), source_id).await?;

            match pairing_status {
                crate::PairingStatus::Pending => {
                    // Show pairing details
                    let pairings = crate::list_pending_pairings(ariata.database.pool()).await?;
                    if let Some(pairing) = pairings.iter().find(|p| p.source_id == source_id) {
                        let server_url = env::var("ARIATA_SERVER_URL")
                            .unwrap_or_else(|_| "localhost:8000".to_string());

                        println!("Source: {} (pending pairing)", pairing.name);
                        println!("Type: {}", pairing.device_type);
                        println!("Status: ⏳ Pending pairing");
                        println!();
                        println!("Pairing Details:");
                        println!("  Server URL:   {}", server_url);
                        println!("  Pairing Code: {}", pairing.code);

                        let duration = pairing.expires_at.signed_duration_since(chrono::Utc::now());
                        let minutes = duration.num_minutes();
                        let seconds = duration.num_seconds() % 60;
                        println!("  Expires In:   {} minutes, {} seconds", minutes, seconds);

                        let created_ago =
                            chrono::Utc::now().signed_duration_since(pairing.created_at);
                        println!("  Created:      {} minutes ago", created_ago.num_minutes());
                        println!();
                        println!("💡 Enter these details in your device app to complete pairing.");
                        println!();
                        println!("To cancel this pairing:");
                        println!("   ariata source delete {}", source_id);
                    }
                }
                _ => {
                    // Show regular source details
                    let source = crate::get_source(ariata.database.pool(), source_id).await?;

                    println!("Source Details:");
                    println!("  ID: {}", source.id);
                    println!("  Name: {}", source.name);
                    println!("  Provider: {}", source.source);
                    println!(
                        "  Status: {}",
                        if source.is_active {
                            "active"
                        } else {
                            "inactive"
                        }
                    );

                    if let Some(error) = source.error_message {
                        println!("  Error: {}", error);
                    }

                    println!("  Created: {}", source.created_at);
                    println!("  Updated: {}", source.updated_at);
                }
            }
        }

        SourceCommands::Status { id } => {
            let source_id = id.parse()?;
            let status = crate::get_source_status(ariata.database.pool(), source_id).await?;

            println!("Source: {} ({})", status.name, status.source);
            println!();
            println!("Sync Statistics:");
            println!("  Total Syncs: {}", status.total_syncs);
            println!("  Successful: {}", status.successful_syncs);
            println!("  Failed: {}", status.failed_syncs);

            if status.total_syncs > 0 {
                let success_rate =
                    (status.successful_syncs as f64 / status.total_syncs as f64) * 100.0;
                println!("  Success Rate: {:.1}%", success_rate);
            }

            if let Some(last_status) = status.last_sync_status {
                println!("  Last Sync Status: {}", last_status);
            }

            if let Some(duration) = status.last_sync_duration_ms {
                println!("  Last Sync Duration: {}ms", duration);
            }
        }

        SourceCommands::Delete { id, yes } => {
            let source_id = id.parse()?;

            // Get source details first
            let source = crate::get_source(ariata.database.pool(), source_id).await?;

            if !yes {
                println!("Are you sure you want to delete source:");
                println!("  Name: {}", source.name);
                println!("  Provider: {}", source.source);
                println!("  ID: {}", source.id);
                println!();
                println!("This will delete ALL data associated with this source!");
                println!();
                print!("Type 'yes' to confirm: ");

                use std::io::{self, Write};
                io::stdout().flush()?;

                let mut input = String::new();
                io::stdin().read_line(&mut input)?;

                if input.trim().to_lowercase() != "yes" {
                    println!("Cancelled");
                    return Ok(());
                }
            }

            crate::delete_source(ariata.database.pool(), source_id).await?;
            println!("✅ Source deleted successfully");
        }

        SourceCommands::History { id, limit } => {
            let source_id = id.parse()?;

            // Query jobs for this source
            let jobs = crate::api::jobs::query_jobs(
                ariata.database.pool(),
                crate::api::jobs::QueryJobsRequest {
                    source_id: Some(source_id),
                    status: None,
                    limit: Some(limit),
                },
            )
            .await?;

            if jobs.is_empty() {
                println!("No sync history found for this source");
                return Ok(());
            }

            println!("Sync History (showing {} most recent):", jobs.len());
            println!(
                "{:<26} {:<10} {:<10} {:<10} {}",
                "Started", "Status", "Records", "Duration", "Error"
            );
            println!("{}", "-".repeat(80));

            for job in jobs {
                let records = job.records_processed;
                let duration =
                    if let (Some(completed), started) = (job.completed_at, job.started_at) {
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
    }

    Ok(())
}
