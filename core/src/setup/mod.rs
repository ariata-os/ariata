//! Interactive setup wizard for Ariata

pub mod validation;

use console::style;
use dialoguer::{theme::ColorfulTheme, Confirm, Input, Password, Select};

use crate::error::Result;
use validation::{display_error, display_info, display_success};

/// Storage type for configuration
#[derive(Debug, Clone)]
pub enum StorageType {
    Local { path: String },
    S3 { endpoint: String, bucket: String, access_key: String, secret_key: String },
}

/// Configuration collected from the setup wizard
#[derive(Debug, Clone)]
pub struct SetupConfig {
    pub database_url: String,
    pub storage: StorageType,
    pub encryption_key: Option<String>,
    pub run_migrations: bool,
}

/// Run the interactive setup wizard
pub async fn run_init() -> Result<SetupConfig> {
    println!();
    println!("{}", style("🎯 Ariata Setup").bold().cyan());
    println!();

    // Step 1: Database configuration
    let database_url = setup_database().await?;

    // Step 2: Run migrations?
    let run_migrations = Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt("Run migrations now?")
        .default(true)
        .interact()
        .unwrap_or(true);

    // Step 3: Encryption key
    let encryption_key = setup_encryption_key()?;

    // Step 4: Storage configuration
    let storage = setup_storage().await?;

    Ok(SetupConfig {
        database_url,
        storage,
        encryption_key,
        run_migrations,
    })
}

/// Database setup step
async fn setup_database() -> Result<String> {
    println!("{}", style("📦 Database Configuration").bold());

    let database_url: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("PostgreSQL connection string")
        .default("postgresql://postgres:postgres@localhost/ariata".to_string())
        .interact_text()
        .map_err(|e| crate::error::Error::Other(format!("Input error: {}", e)))?;

    // Test connection
    display_info("Testing connection...");
    match validation::test_database_connection(&database_url).await {
        Ok(_) => {
            display_success("Connected!");
        }
        Err(e) => {
            display_error(&format!("Connection failed: {}", e));
            return Err(e);
        }
    }

    println!();
    Ok(database_url)
}

/// Encryption key setup step
fn setup_encryption_key() -> Result<Option<String>> {
    println!("{}", style("🔐 Security").bold());

    let generate = Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt("Generate encryption key for OAuth tokens?")
        .default(true)
        .interact()
        .unwrap_or(true);

    let key = if generate {
        let key = generate_encryption_key();
        display_success("Encryption key generated");
        Some(key)
    } else {
        None
    };

    println!();
    Ok(key)
}

/// Storage setup step
async fn setup_storage() -> Result<StorageType> {
    println!("{}", style("💾 Storage Configuration").bold());

    let options = vec!["Local filesystem", "S3/MinIO"];
    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Storage type")
        .items(&options)
        .default(0)
        .interact()
        .map_err(|e| crate::error::Error::Other(format!("Selection error: {}", e)))?;

    let storage = match selection {
        0 => setup_local_storage().await?,
        1 => setup_s3_storage().await?,
        _ => unreachable!(),
    };

    println!();
    Ok(storage)
}

/// Local filesystem storage setup
async fn setup_local_storage() -> Result<StorageType> {
    let path: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("Storage path")
        .default("./data".to_string())
        .interact_text()
        .map_err(|e| crate::error::Error::Other(format!("Input error: {}", e)))?;

    // Test local storage
    display_info("Testing local storage...");
    match validation::test_local_storage(&path).await {
        Ok(_) => {
            display_success(&format!("Using: {}", path));
        }
        Err(e) => {
            display_error(&format!("Storage test failed: {}", e));
            return Err(e);
        }
    }

    Ok(StorageType::Local { path })
}

/// S3/MinIO storage setup
async fn setup_s3_storage() -> Result<StorageType> {
    let endpoint: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("S3 Endpoint (leave empty for AWS S3)")
        .allow_empty(true)
        .interact_text()
        .map_err(|e| crate::error::Error::Other(format!("Input error: {}", e)))?;

    let bucket: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("Bucket name")
        .interact_text()
        .map_err(|e| crate::error::Error::Other(format!("Input error: {}", e)))?;

    let access_key: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("Access Key")
        .interact_text()
        .map_err(|e| crate::error::Error::Other(format!("Input error: {}", e)))?;

    let secret_key: String = Password::with_theme(&ColorfulTheme::default())
        .with_prompt("Secret Key")
        .interact()
        .map_err(|e| crate::error::Error::Other(format!("Input error: {}", e)))?;

    // Test S3 connection
    display_info("Testing S3 connection...");
    let endpoint_opt = if endpoint.is_empty() {
        None
    } else {
        Some(endpoint.clone())
    };

    match validation::test_s3_connection(endpoint_opt.clone(), &bucket, &access_key, &secret_key)
        .await
    {
        Ok(_) => {
            display_success("Connected!");
        }
        Err(e) => {
            display_error(&format!("S3 connection failed: {}", e));
            return Err(e);
        }
    }

    Ok(StorageType::S3 {
        endpoint,
        bucket,
        access_key,
        secret_key,
    })
}

/// Generate a random 32-byte encryption key
fn generate_encryption_key() -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let key: [u8; 32] = rng.gen();
    base64::Engine::encode(&base64::engine::general_purpose::STANDARD, key)
}

/// Save configuration to .env file
pub fn save_config(config: &SetupConfig) -> Result<()> {
    // Check if .env already exists
    if std::path::Path::new(".env").exists() {
        println!();
        println!(
            "{} {}",
            style("⚠️").yellow().bold(),
            style(".env already exists in this directory").yellow().bold()
        );

        let overwrite = Confirm::with_theme(&ColorfulTheme::default())
            .with_prompt("Overwrite existing .env file?")
            .default(false)
            .interact()
            .unwrap_or(false);

        if !overwrite {
            println!();
            println!(
                "{} Configuration cancelled. Your existing .env was not modified.",
                style("✓").green().bold()
            );
            println!();
            return Ok(());
        }
    }

    let mut content = String::new();

    content.push_str("# Generated by ariata init\n\n");

    // Database
    content.push_str("# Database (required)\n");
    content.push_str(&format!("DATABASE_URL={}\n\n", config.database_url));

    // Storage
    match &config.storage {
        StorageType::Local { path } => {
            content.push_str("# Storage (local filesystem)\n");
            content.push_str(&format!("STORAGE_PATH={}\n\n", path));
        }
        StorageType::S3 {
            endpoint,
            bucket,
            access_key,
            secret_key,
        } => {
            content.push_str("# Storage (S3/MinIO)\n");
            if !endpoint.is_empty() {
                content.push_str(&format!("S3_ENDPOINT={}\n", endpoint));
            }
            content.push_str(&format!("S3_BUCKET={}\n", bucket));
            content.push_str(&format!("S3_ACCESS_KEY={}\n", access_key));
            content.push_str(&format!("S3_SECRET_KEY={}\n\n", secret_key));
        }
    }

    // Encryption key
    if let Some(key) = &config.encryption_key {
        content.push_str("# Security (recommended)\n");
        content.push_str(&format!("ARIATA_ENCRYPTION_KEY={}\n\n", key));
    }

    // OAuth Proxy (informational)
    content.push_str("# OAuth Proxy (uses public proxy by default)\n");
    content.push_str("# OAUTH_PROXY_URL=https://auth.ariata.com\n");

    // Write to .env file
    std::fs::write(".env", content)
        .map_err(|e| crate::error::Error::Other(format!("Failed to write .env: {}", e)))?;

    display_success("Configuration saved to .env");
    Ok(())
}

/// Display completion message with next steps
pub fn display_completion() {
    println!();
    println!("{}", style("Done! Try these commands:").bold().green());
    println!("  {} - Browse available integrations", style("ariata catalog sources").cyan());
    println!("  {} - Connect your Notion workspace", style("ariata add notion").cyan());
    println!("  {} - List connected sources", style("ariata source list").cyan());
    println!();
}
