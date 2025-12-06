use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use zeroize::Zeroize;
use crate::core::crypto::{encrypt, decrypt};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Secret {
    pub name: String,
    pub encrypted_value: Vec<u8>,
    /// Expiration date as Unix timestamp (None = no expiration)
    pub expires_at: Option<i64>,
}

impl Secret {
    /// Checks if the secret is expired
    pub fn is_expired(&self) -> bool {
        if let Some(expires_at) = self.expires_at {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs() as i64)
                .unwrap_or(0);
            now > expires_at
        } else {
            false
        }
    }

    /// Returns the number of days remaining before expiration (None if no expiration)
    pub fn days_until_expiration(&self) -> Option<i64> {
        if let Some(expires_at) = self.expires_at {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs() as i64)
                .unwrap_or(0);
            let remaining_secs = expires_at - now;
            Some(remaining_secs / 86400) // 86400 seconds per day
        } else {
            None
        }
    }

    /// Formats the expiration date for display
    pub fn expiration_display(&self) -> String {
        match self.days_until_expiration() {
            Some(days) if days < 0 => "⚠️ EXPIRED".to_string(),
            Some(days) if days == 0 => "⚠️ Expires today".to_string(),
            Some(days) if days == 1 => "⚠️ Expires tomorrow".to_string(),
            Some(days) if days <= 7 => format!("⚠️ {} days", days),
            Some(days) => format!("{} days", days),
            None => "∞ Permanent".to_string(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SecretsStore {
    pub secrets: HashMap<String, Secret>,
    #[serde(skip)]
    path: Option<PathBuf>,
}

impl SecretsStore {
    pub fn new() -> Self {
        Self {
            secrets: HashMap::new(),
            path: None,
        }
    }

    pub fn load(locker_dir: &PathBuf, key: &[u8]) -> Result<Self> {
        let file_path = locker_dir.join("secrets.json");
        if file_path.exists() {
            let data = fs::read(&file_path)?;
            let decrypted = decrypt(&data, key)?;
            let mut store: SecretsStore = serde_json::from_slice(&decrypted)?;
            store.path = Some(file_path);
            Ok(store)
        } else {
            Ok(Self {
                secrets: HashMap::new(),
                path: Some(file_path),
            })
        }
    }

    /// Loads from a specific path (used by agent)
    pub fn load_from_path(path: &PathBuf, key: &[u8]) -> Result<Self> {
        if path.exists() {
            let data = fs::read(path)?;
            let decrypted = decrypt(&data, key)?;
            let mut store: SecretsStore = serde_json::from_slice(&decrypted)?;
            store.path = Some(path.clone());
            Ok(store)
        } else {
            Ok(Self {
                secrets: HashMap::new(),
                path: Some(path.clone()),
            })
        }
    }

    /// Returns the secrets file path
    pub fn get_path(&self) -> &PathBuf {
        self.path.as_ref().expect("Store path not set")
    }

    pub fn save(&self, locker_dir: &PathBuf, key: &[u8]) -> Result<()> {
        let json = serde_json::to_vec(self)?;
        let encrypted = encrypt(&json, key)?;
        fs::write(locker_dir.join("secrets.json"), encrypted)?;
        Ok(())
    }

    pub fn add_secret(
        &mut self,
        name: String,
        value: String,
        expiration_days: Option<u32>,
        locker_dir: &PathBuf,
        key: &[u8],
    ) -> Result<()> {
        let encrypted_value = encrypt(value.as_bytes(), key)?;
        
        let expires_at = expiration_days.map(|days| {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs() as i64)
                .unwrap_or(0);
            now + (days as i64 * 86400)
        });
        
        let secret = Secret {
            name: name.clone(),
            encrypted_value,
            expires_at,
        };
        self.secrets.insert(name, secret);
        self.save(locker_dir, key)?;
        Ok(())
    }

    pub fn get_secret(&self, name: &str) -> Option<&Secret> {
        self.secrets.get(name)
    }

    pub fn list_secrets(&self) -> Vec<&Secret> {
        let mut secrets: Vec<_> = self.secrets.values().collect();
        secrets.sort_by(|a, b| a.name.cmp(&b.name));
        secrets
    }

    pub fn delete_secret(&mut self, name: &str, locker_dir: &PathBuf, key: &[u8]) -> Result<()> {
        self.secrets.remove(name);
        self.save(locker_dir, key)?;
        Ok(())
    }

    pub fn decrypt_secret(&self, name: &str, key: &[u8]) -> Result<String> {
        if let Some(secret) = self.get_secret(name) {
            let decrypted = decrypt(&secret.encrypted_value, key)?;
            let value = String::from_utf8(decrypted)?;
            Ok(value)
        } else {
            Err(anyhow::anyhow!("Secret not found"))
        }
    }

    /// Decrypts all secrets and returns a HashMap name -> value
    pub fn decrypt_all(&self, key: &[u8]) -> Result<HashMap<String, String>> {
        let mut result = HashMap::new();
        for secret in self.secrets.values() {
            let decrypted = decrypt(&secret.encrypted_value, key)?;
            let value = String::from_utf8(decrypted)?;
            result.insert(secret.name.clone(), value);
        }
        Ok(result)
    }
}

impl Drop for SecretsStore {
    fn drop(&mut self) {
        for secret in self.secrets.values_mut() {
            secret.encrypted_value.zeroize();
        }
    }
}