//! CLI headless commands for CI/CD and scripting
//!
//! Provides non-interactive commands for automation:
//! - `init --passphrase <PASS>` - Initialize a new locker
//! - `token add/get/list/remove` - Manage tokens
//! - `import` - Import from .env files

use anyhow::{Context, Result};
use std::collections::HashMap;
use std::io::{self, BufRead, Read};
use std::path::PathBuf;

use crate::core::crypto::decrypt;
use crate::core::init::Locker;
use crate::core::store::SecretsStore;

/// Environment variable for passphrase (more secure than CLI argument)
const PASSPHRASE_ENV_VAR: &str = "LAZY_LOCKER_PASSPHRASE";

/// Gets passphrase from argument or environment variable
/// Priority: argument > environment variable
pub fn get_passphrase(arg_passphrase: Option<&str>) -> Result<String> {
    if let Some(pass) = arg_passphrase {
        return Ok(pass.to_string());
    }

    std::env::var(PASSPHRASE_ENV_VAR).context(format!(
        "Passphrase required. Use --passphrase <PASS> or set {} environment variable",
        PASSPHRASE_ENV_VAR
    ))
}

/// Output format for list/get commands
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum OutputFormat {
    Human,
    Json,
    Env,
}

impl OutputFormat {
    pub fn from_args(json: bool, env: bool) -> Self {
        if json {
            OutputFormat::Json
        } else if env {
            OutputFormat::Env
        } else {
            OutputFormat::Human
        }
    }
}

// ============================================================================
// INIT COMMAND
// ============================================================================

/// Initialize a new locker with the given passphrase
pub fn cmd_init(passphrase: &str, force: bool) -> Result<()> {
    let locker_dir = get_locker_dir()?;
    let salt_path = locker_dir.join("salt");

    if salt_path.exists() && !force {
        anyhow::bail!(
            "Locker already exists at {:?}. Use --force to overwrite.",
            locker_dir
        );
    }

    if force && salt_path.exists() {
        // Remove existing locker files
        std::fs::remove_file(locker_dir.join("salt")).ok();
        std::fs::remove_file(locker_dir.join("hash")).ok();
        std::fs::remove_file(locker_dir.join("secrets.json")).ok();
    }

    // Initialize with passphrase
    let _locker = Locker::init_or_load_with_passphrase(passphrase)?;

    println!("✅ Locker initialized at {:?}", locker_dir);
    Ok(())
}

// ============================================================================
// TOKEN COMMANDS
// ============================================================================

/// Add a new token
pub fn cmd_token_add(
    name: &str,
    value: Option<&str>,
    stdin: bool,
    expires_days: Option<u32>,
    passphrase: &str,
) -> Result<()> {
    let secret_value = if stdin {
        read_value_from_stdin()?
    } else if let Some(v) = value {
        v.to_string()
    } else {
        anyhow::bail!("Value required. Provide as argument or use --stdin");
    };

    let locker = Locker::init_or_load_with_passphrase(passphrase)?;
    let key = locker.get_key().context("Failed to get encryption key")?;
    let locker_dir = locker.base_dir().clone();

    let mut store = SecretsStore::load(&locker_dir, key)?;
    store.add_secret(
        name.to_string(),
        secret_value,
        expires_days,
        &locker_dir,
        key,
    )?;

    println!("✅ Token '{}' added", name);
    if let Some(days) = expires_days {
        println!("   Expires in {} days", days);
    }

    Ok(())
}

/// Get a token value
pub fn cmd_token_get(name: &str, format: OutputFormat, passphrase: &str) -> Result<()> {
    let locker = Locker::init_or_load_with_passphrase(passphrase)?;
    let key = locker.get_key().context("Failed to get encryption key")?;
    let locker_dir = locker.base_dir().clone();

    let store = SecretsStore::load(&locker_dir, key)?;
    let secret = store
        .get_secret(name)
        .context(format!("Token '{}' not found", name))?;

    if secret.is_expired() {
        anyhow::bail!("Token '{}' has expired", name);
    }

    let value = decrypt(&secret.encrypted_value, key)?;
    let value_str = String::from_utf8(value)?;

    match format {
        OutputFormat::Human => println!("{}", value_str),
        OutputFormat::Json => {
            let obj = serde_json::json!({
                "name": name,
                "value": value_str,
                "expires_at": secret.expires_at,
            });
            println!("{}", serde_json::to_string_pretty(&obj)?);
        }
        OutputFormat::Env => println!("{}={}", name, value_str),
    }

    Ok(())
}

/// List all tokens
pub fn cmd_token_list(format: OutputFormat, passphrase: &str) -> Result<()> {
    let locker = Locker::init_or_load_with_passphrase(passphrase)?;
    let key = locker.get_key().context("Failed to get encryption key")?;
    let locker_dir = locker.base_dir().clone();

    let store = SecretsStore::load(&locker_dir, key)?;
    let secrets = store.list_secrets();

    match format {
        OutputFormat::Human => {
            if secrets.is_empty() {
                println!("No tokens found.");
                return Ok(());
            }

            println!("{:<30} {:<20} STATUS", "NAME", "EXPIRES");
            println!("{:-<60}", "");

            for secret in secrets {
                let status = if secret.is_expired() {
                    "⚠️ EXPIRED"
                } else {
                    "✓"
                };
                println!(
                    "{:<30} {:<20} {}",
                    secret.name,
                    secret.expiration_display(),
                    status
                );
            }
        }
        OutputFormat::Json => {
            let list: Vec<_> = secrets
                .iter()
                .map(|s| {
                    serde_json::json!({
                        "name": s.name,
                        "expires_at": s.expires_at,
                        "is_expired": s.is_expired(),
                        "days_remaining": s.days_until_expiration(),
                    })
                })
                .collect();
            println!("{}", serde_json::to_string_pretty(&list)?);
        }
        OutputFormat::Env => {
            // For env format, we need to decrypt and output all values
            for secret in secrets {
                if !secret.is_expired() {
                    let value = decrypt(&secret.encrypted_value, key)?;
                    let value_str = String::from_utf8(value)?;
                    println!("{}={}", secret.name, value_str);
                }
            }
        }
    }

    Ok(())
}

/// Remove a token
pub fn cmd_token_remove(name: &str, passphrase: &str) -> Result<()> {
    let locker = Locker::init_or_load_with_passphrase(passphrase)?;
    let key = locker.get_key().context("Failed to get encryption key")?;
    let locker_dir = locker.base_dir().clone();

    let mut store = SecretsStore::load(&locker_dir, key)?;

    if store.get_secret(name).is_none() {
        anyhow::bail!("Token '{}' not found", name);
    }

    store.delete_secret(name, &locker_dir, key)?;
    println!("✅ Token '{}' removed", name);

    Ok(())
}

// ============================================================================
// IMPORT COMMAND
// ============================================================================

/// Import tokens from a .env file or stdin
pub fn cmd_import(
    file: Option<&str>,
    stdin: bool,
    format: &str,
    expires_days: Option<u32>,
    passphrase: &str,
) -> Result<()> {
    let content = if stdin {
        let mut buf = String::new();
        io::stdin().read_to_string(&mut buf)?;
        buf
    } else if let Some(path) = file {
        std::fs::read_to_string(path).context(format!("Failed to read file: {}", path))?
    } else {
        anyhow::bail!("Provide a file path or use --stdin");
    };

    let secrets = match format {
        "env" => parse_env_format(&content)?,
        "json" => parse_json_format(&content)?,
        _ => anyhow::bail!("Unknown format: {}. Supported: env, json", format),
    };

    if secrets.is_empty() {
        println!("⚠️  No secrets found in input");
        return Ok(());
    }

    let locker = Locker::init_or_load_with_passphrase(passphrase)?;
    let key = locker.get_key().context("Failed to get encryption key")?;
    let locker_dir = locker.base_dir().clone();

    let mut store = SecretsStore::load(&locker_dir, key)?;
    let mut count = 0;

    for (name, value) in secrets {
        store.add_secret(name.clone(), value, expires_days, &locker_dir, key)?;
        count += 1;
    }

    println!("✅ Imported {} tokens", count);
    if let Some(days) = expires_days {
        println!("   All tokens expire in {} days", days);
    }

    Ok(())
}

// ============================================================================
// EXPORT COMMAND (bonus)
// ============================================================================

/// Export all tokens to stdout
pub fn cmd_export(format: OutputFormat, passphrase: &str) -> Result<()> {
    // Reuse token list with env format for export
    cmd_token_list(format, passphrase)
}

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

fn get_locker_dir() -> Result<PathBuf> {
    use directories::BaseDirs;

    let base_dirs = BaseDirs::new().context("Unable to determine user directories")?;
    let config_dir = base_dirs.config_dir();

    #[cfg(unix)]
    let sub_dir = ".lazy-locker";
    #[cfg(not(unix))]
    let sub_dir = "lazy-locker";

    let locker_dir = config_dir.join(sub_dir);
    std::fs::create_dir_all(&locker_dir)?;

    Ok(locker_dir)
}

fn read_value_from_stdin() -> Result<String> {
    let stdin = io::stdin();
    let mut value = String::new();

    // Read first line only (trim newline)
    stdin.lock().read_line(&mut value)?;

    // Remove trailing newline
    if value.ends_with('\n') {
        value.pop();
    }
    if value.ends_with('\r') {
        value.pop();
    }

    if value.is_empty() {
        anyhow::bail!("No value provided on stdin");
    }

    Ok(value)
}

fn parse_env_format(content: &str) -> Result<HashMap<String, String>> {
    let mut secrets = HashMap::new();

    for line in content.lines() {
        let line = line.trim();

        // Skip empty lines and comments
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        // Parse KEY=VALUE or KEY="VALUE" or KEY='VALUE'
        if let Some(eq_pos) = line.find('=') {
            let key = line[..eq_pos].trim().to_string();
            let mut value = line[eq_pos + 1..].trim().to_string();

            // Remove surrounding quotes
            if (value.starts_with('"') && value.ends_with('"'))
                || (value.starts_with('\'') && value.ends_with('\''))
            {
                value = value[1..value.len() - 1].to_string();
            }

            if !key.is_empty() {
                secrets.insert(key, value);
            }
        }
    }

    Ok(secrets)
}

fn parse_json_format(content: &str) -> Result<HashMap<String, String>> {
    // Support both object format and array format
    let json: serde_json::Value = serde_json::from_str(content)?;
    let mut secrets = HashMap::new();

    match json {
        serde_json::Value::Object(obj) => {
            for (key, value) in obj {
                if let Some(v) = value.as_str() {
                    secrets.insert(key, v.to_string());
                }
            }
        }
        serde_json::Value::Array(arr) => {
            for item in arr {
                if let (Some(name), Some(value)) = (
                    item.get("name").and_then(|v| v.as_str()),
                    item.get("value").and_then(|v| v.as_str()),
                ) {
                    secrets.insert(name.to_string(), value.to_string());
                }
            }
        }
        _ => anyhow::bail!("JSON must be an object or array"),
    }

    Ok(secrets)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_env_format() {
        let content = r#"
# Comment
DATABASE_URL=postgres://localhost/db
API_KEY="sk-123456"
SECRET='my_secret'
EMPTY=

SPACES = value with spaces
"#;

        let secrets = parse_env_format(content).unwrap();

        assert_eq!(
            secrets.get("DATABASE_URL"),
            Some(&"postgres://localhost/db".to_string())
        );
        assert_eq!(secrets.get("API_KEY"), Some(&"sk-123456".to_string()));
        assert_eq!(secrets.get("SECRET"), Some(&"my_secret".to_string()));
        assert_eq!(secrets.get("EMPTY"), Some(&"".to_string()));
        assert_eq!(
            secrets.get("SPACES"),
            Some(&"value with spaces".to_string())
        );
    }

    #[test]
    fn test_parse_json_object_format() {
        let content = r#"{"API_KEY": "sk-123", "DB_URL": "postgres://localhost"}"#;

        let secrets = parse_json_format(content).unwrap();

        assert_eq!(secrets.get("API_KEY"), Some(&"sk-123".to_string()));
        assert_eq!(
            secrets.get("DB_URL"),
            Some(&"postgres://localhost".to_string())
        );
    }

    #[test]
    fn test_parse_json_array_format() {
        let content = r#"[
            {"name": "API_KEY", "value": "sk-123"},
            {"name": "DB_URL", "value": "postgres://localhost"}
        ]"#;

        let secrets = parse_json_format(content).unwrap();

        assert_eq!(secrets.get("API_KEY"), Some(&"sk-123".to_string()));
        assert_eq!(
            secrets.get("DB_URL"),
            Some(&"postgres://localhost".to_string())
        );
    }

    #[test]
    fn test_output_format_from_args() {
        assert_eq!(OutputFormat::from_args(false, false), OutputFormat::Human);
        assert_eq!(OutputFormat::from_args(true, false), OutputFormat::Json);
        assert_eq!(OutputFormat::from_args(false, true), OutputFormat::Env);
        // JSON takes priority if both are set
        assert_eq!(OutputFormat::from_args(true, true), OutputFormat::Json);
    }
}
