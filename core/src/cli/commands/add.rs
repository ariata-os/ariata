//! Add command - add new OAuth or device sources

use crate::client::Ariata;
use std::env;

/// Handle adding a new source (OAuth or device)
pub async fn handle_add_source(
    ariata: Ariata,
    source_type: &str,
    _device_id: Option<String>,
    name: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("🔐 Adding {} source...", source_type);

    // Check if this is a device source
    let descriptor = crate::get_source_info(source_type);

    let is_device_source = descriptor
        .map(|d| matches!(d.auth_type, crate::registry::AuthType::Device))
        .unwrap_or(false);

    if is_device_source {
        // Handle device pairing flow
        let name = name.ok_or_else(|| "name is required for device sources".to_string())?;
        return handle_device_pairing(ariata, source_type, &name).await;
    }

    // Handle OAuth flow
    let redirect_uri = "http://localhost:8080";
    let response = crate::initiate_oauth_flow(source_type, Some(redirect_uri.to_string()), None)
        .await
        .map_err(|e| format!("Failed to initiate OAuth flow: {e}"))?;

    println!("\n🌐 Please visit the following URL to authorize:");
    println!("{}", response.authorization_url);
    println!("\nPress Enter after you've authorized and been redirected...");

    // Open browser automatically
    #[cfg(not(target_os = "windows"))]
    std::process::Command::new("open")
        .arg(&response.authorization_url)
        .spawn()
        .ok();
    #[cfg(target_os = "windows")]
    std::process::Command::new("cmd")
        .args(&["/C", "start", &response.authorization_url])
        .spawn()
        .ok();

    // Wait for user input
    use std::io::{self, Write};
    io::stdout().flush()?;
    let mut _input = String::new();
    io::stdin().read_line(&mut _input)?;

    // Get the authorization code from the redirect URL
    println!("\n📋 Please paste the full redirect URL here:");
    io::stdout().flush()?;
    let mut redirect_url = String::new();
    io::stdin().read_line(&mut redirect_url)?;

    // Parse callback URL parameters
    let callback_params = parse_callback_url(&redirect_url, source_type)?;

    // Handle callback and create source
    let source = crate::handle_oauth_callback(ariata.database.pool(), &callback_params).await?;

    println!("\n✅ Source created successfully!");
    println!("   Name: {}", source.name);
    println!("   ID: {}", source.id);

    // List available streams
    let streams = crate::list_source_streams(ariata.database.pool(), source.id).await?;
    if !streams.is_empty() {
        println!("\n📊 Available streams (all disabled by default):");
        for stream in streams {
            println!(
                "   - {} ({})",
                stream.stream_name,
                if stream.is_enabled {
                    "enabled"
                } else {
                    "disabled"
                }
            );
        }
        println!(
            "\n💡 Enable streams with: ariata stream enable {} <stream_name>",
            source.id
        );
    }

    Ok(())
}

/// Parse OAuth callback URL into parameters
fn parse_callback_url(
    url: &str,
    provider: &str,
) -> Result<crate::OAuthCallbackParams, Box<dyn std::error::Error>> {
    use oauth2::url::Url;

    // Trim whitespace (including newline from stdin)
    let url = url.trim();

    let parsed_url = Url::parse(url)?;
    let params: std::collections::HashMap<_, _> = parsed_url.query_pairs().collect();

    // Extract common OAuth parameters
    let code = params.get("code").map(|s| s.to_string());
    let access_token = params.get("access_token").map(|s| s.to_string());
    let refresh_token = params.get("refresh_token").map(|s| s.to_string());
    let expires_in = params
        .get("expires_in")
        .and_then(|s| s.parse::<i64>().ok());
    let state = params.get("state").map(|s| s.to_string());

    // Notion-specific fields
    let workspace_id = params.get("workspace_id").map(|s| s.to_string());
    let workspace_name = params.get("workspace_name").map(|s| s.to_string());
    let bot_id = params.get("bot_id").map(|s| s.to_string());

    // Validate that we have either code or access_token
    if code.is_none() && access_token.is_none() {
        return Err("OAuth callback URL must contain either 'code' or 'access_token' parameter".into());
    }

    Ok(crate::OAuthCallbackParams {
        code,
        access_token,
        refresh_token,
        expires_in,
        provider: provider.to_string(),
        state,
        workspace_id,
        workspace_name,
        bot_id,
    })
}

/// Handle device pairing flow
async fn handle_device_pairing(
    ariata: Ariata,
    device_type: &str,
    name: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    use crate::cli::display::*;

    let server_url = env::var("ARIATA_SERVER_URL").unwrap_or_else(|_| "localhost:8000".to_string());

    println!("🔐 Adding {} source...", device_type);
    println!();

    let pairing = crate::initiate_device_pairing(ariata.database.pool(), device_type, name).await?;

    display_pairing_code(&pairing.code, &server_url, &pairing.expires_at);

    println!("⏳ Waiting for device to connect...");
    let result = wait_for_pairing(ariata.database.pool(), pairing.source_id, pairing.expires_at).await?;

    match result {
        PairingResult::Success(device_info) => {
            display_pairing_success(&device_info, pairing.source_id);

            let streams = crate::list_source_streams(ariata.database.pool(), pairing.source_id).await?;
            display_available_streams(&streams, pairing.source_id);
        }
        PairingResult::Timeout => {
            display_pairing_timeout();
        }
        PairingResult::Cancelled => {
            display_pairing_cancelled(&pairing.code);
        }
    }

    Ok(())
}
