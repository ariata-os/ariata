//! Device pairing display utilities

use crate::{DeviceInfo, PairingStatus};
use chrono::{DateTime, Utc};
use console::{style, Term};
use indicatif::{ProgressBar, ProgressStyle};
use std::time::Duration;
use uuid::Uuid;

/// Result of a pairing operation
pub enum PairingResult {
    Success(DeviceInfo),
    Timeout,
    Cancelled,
}

/// Display pairing code and instructions
pub fn display_pairing_code(code: &str, server_url: &str, expires_at: &DateTime<Utc>) {
    let term = Term::stdout();

    let _ = term.write_line("");
    let _ = term.write_line(&style("━".repeat(70)).dim().to_string());
    let _ = term.write_line("");
    let _ = term.write_line(&format!(
        "  {} {}",
        style("📱").bold(),
        style("Open your device app and enter these details:").bold()
    ));
    let _ = term.write_line("");

    let _ = term.write_line(&format!(
        "  {}  {}",
        style("Server URL: ").cyan().bold(),
        style(server_url).yellow().bold()
    ));

    let _ = term.write_line(&format!(
        "  {}  {}",
        style("Pairing Code:").cyan().bold(),
        style(code).yellow().bold()
    ));
    let _ = term.write_line("");

    let duration = expires_at.signed_duration_since(Utc::now());
    let minutes = duration.num_minutes();
    let _ = term.write_line(&format!(
        "  {}",
        style(format!("This code expires in {} minutes.", minutes)).dim()
    ));
    let _ = term.write_line("");
    let _ = term.write_line(&style("━".repeat(70)).dim().to_string());
    let _ = term.write_line("");
}

/// Wait for device to complete pairing with visual feedback
pub async fn wait_for_pairing(
    db: &sqlx::PgPool,
    source_id: Uuid,
    expires_at: DateTime<Utc>,
) -> Result<PairingResult, Box<dyn std::error::Error>> {
    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.cyan} {msg}")
            .unwrap(),
    );
    spinner.set_message("Waiting for device to connect...");

    let timeout_duration = expires_at.signed_duration_since(Utc::now());
    let timeout_secs = timeout_duration.num_seconds().max(0) as u64;
    let timeout = Duration::from_secs(timeout_secs);

    let result = tokio::time::timeout(timeout, async {
        loop {
            spinner.tick();

            let status = crate::check_pairing_status(db, source_id).await?;

            match status {
                PairingStatus::Active(device_info) => {
                    return Ok(PairingResult::Success(device_info));
                }
                PairingStatus::Pending => {
                    tokio::time::sleep(Duration::from_secs(2)).await;
                }
                PairingStatus::Revoked => {
                    return Ok(PairingResult::Cancelled);
                }
            }
        }
    })
    .await;

    spinner.finish_and_clear();

    match result {
        Ok(inner_result) => inner_result,
        Err(_) => Ok(PairingResult::Timeout),
    }
}

/// Display successful pairing with device details
pub fn display_pairing_success(device_info: &DeviceInfo, source_id: Uuid) {
    let term = Term::stdout();

    let _ = term.write_line(&format!(
        "\n{}\n",
        style("✅ Device paired successfully!").green().bold()
    ));

    let _ = term.write_line(&style("Device Details:").cyan().bold().to_string());
    let _ = term.write_line(&format!("  Name:      {}", device_info.device_name));
    let _ = term.write_line(&format!("  Model:     {}", device_info.device_model));
    let _ = term.write_line(&format!("  OS:        {}", device_info.os_version));

    if let Some(app_version) = &device_info.app_version {
        let _ = term.write_line(&format!("  App:       Ariata v{}", app_version));
    }

    let _ = term.write_line(&format!("  Source ID: {}", source_id));
    let _ = term.write_line("");
}

/// Display available streams with helpful commands
pub fn display_available_streams(streams: &[crate::StreamInfo], source_id: Uuid) {
    let term = Term::stdout();

    if streams.is_empty() {
        return;
    }

    let _ = term.write_line(
        &style("📊 Available streams (all disabled by default):")
            .bold()
            .to_string(),
    );

    for stream in streams {
        let status = if stream.is_enabled { "enabled" } else { "disabled" };
        let _ = term.write_line(&format!(
            "   {} {}     {}",
            style("•").dim(),
            style(&stream.stream_name).cyan(),
            style(status).dim()
        ));
    }

    let _ = term.write_line("");
    let _ = term.write_line(&style("💡 Enable streams with:").dim().to_string());
    let _ = term.write_line(&format!(
        "   {} stream enable {} <stream_name>",
        style("ariata").cyan(),
        style(source_id.to_string()).yellow()
    ));
    let _ = term.write_line("");
}

/// Display timeout message
pub fn display_pairing_timeout() {
    let term = Term::stdout();

    let _ = term.write_line(&format!(
        "\n{}\n",
        style("❌ Pairing code expired. No device connected.").red()
    ));

    let _ = term.write_line(&format!(
        "{}",
        style("💡 Try again with the same command.").dim()
    ));
    let _ = term.write_line("");
}

/// Display cancellation message
pub fn display_pairing_cancelled(code: &str) {
    let term = Term::stdout();

    let _ = term.write_line(&format!("\n{}\n", style("⚠️  Pairing cancelled.").yellow()));

    let _ = term.write_line(&format!(
        "The pairing code {} may still be valid.",
        style(code).yellow()
    ));
    let _ = term.write_line(&format!(
        "{}",
        style("You can still complete pairing from your device.").dim()
    ));
    let _ = term.write_line("");
    let _ = term.write_line(&format!(
        "{}",
        style("To check status: ariata source list --pending").dim()
    ));
    let _ = term.write_line("");
}

/// Display pending pairings table
pub fn display_pending_pairings(pairings: &[crate::PendingPairing]) {
    if pairings.is_empty() {
        println!("No pending device pairings.");
        return;
    }

    println!("Pending Device Pairings:");
    println!("┌────────────────────────────────────┬───────┬──────────┬─────────────┐");
    println!("│ {:^34} │ {:^5} │ {:^8} │ {:^11} │", "Name", "Type", "Code", "Expires In");
    println!("├────────────────────────────────────┼───────┼──────────┼─────────────┤");

    for pairing in pairings {
        let duration = pairing.expires_at.signed_duration_since(Utc::now());
        let minutes = duration.num_minutes();
        let seconds = duration.num_seconds() % 60;
        let expires_in = if minutes > 0 {
            format!("{}m {}s", minutes, seconds)
        } else {
            format!("{}s", seconds)
        };

        let name_truncated = if pairing.name.len() > 34 {
            format!("{}...", &pairing.name[..31])
        } else {
            pairing.name.clone()
        };

        println!("│ {:<34} │ {:<5} │ {:<8} │ {:<11} │", name_truncated, pairing.device_type, pairing.code, expires_in);
    }

    println!("└────────────────────────────────────┴───────┴──────────┴─────────────┘");
    println!();
    println!("💡 Complete pairing from your device or cancel with:");
    println!("   ariata source delete <source-id>");
}
