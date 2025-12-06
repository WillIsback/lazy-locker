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

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    /// Generates a valid 32-byte test key
    fn test_key() -> [u8; 32] {
        [0x42u8; 32]
    }

    // ========================
    // Secret struct tests
    // ========================

    #[test]
    fn test_secret_no_expiration() {
        let secret = Secret {
            name: "TEST_TOKEN".to_string(),
            encrypted_value: vec![1, 2, 3],
            expires_at: None,
        };

        assert!(!secret.is_expired());
        assert_eq!(secret.days_until_expiration(), None);
        assert_eq!(secret.expiration_display(), "∞ Permanent");
    }

    #[test]
    fn test_secret_expired() {
        let past_timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64
            - 86400; // 1 day ago

        let secret = Secret {
            name: "EXPIRED_TOKEN".to_string(),
            encrypted_value: vec![1, 2, 3],
            expires_at: Some(past_timestamp),
        };

        assert!(secret.is_expired());
        assert!(secret.days_until_expiration().unwrap() < 0);
        assert_eq!(secret.expiration_display(), "⚠️ EXPIRED");
    }

    #[test]
    fn test_secret_expires_today() {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        let secret = Secret {
            name: "EXPIRING_TODAY".to_string(),
            encrypted_value: vec![1, 2, 3],
            expires_at: Some(now + 3600), // In 1 hour
        };

        assert!(!secret.is_expired());
        assert_eq!(secret.days_until_expiration(), Some(0));
        assert_eq!(secret.expiration_display(), "⚠️ Expires today");
    }

    #[test]
    fn test_secret_expires_tomorrow() {
        let tomorrow = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64
            + 86400 + 3600; // Tomorrow + 1 hour margin

        let secret = Secret {
            name: "EXPIRING_TOMORROW".to_string(),
            encrypted_value: vec![1, 2, 3],
            expires_at: Some(tomorrow),
        };

        assert!(!secret.is_expired());
        assert_eq!(secret.days_until_expiration(), Some(1));
        assert_eq!(secret.expiration_display(), "⚠️ Expires tomorrow");
    }

    #[test]
    fn test_secret_expires_in_week() {
        let in_5_days = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64
            + 5 * 86400;

        let secret = Secret {
            name: "EXPIRING_WEEK".to_string(),
            encrypted_value: vec![1, 2, 3],
            expires_at: Some(in_5_days),
        };

        assert!(!secret.is_expired());
        assert_eq!(secret.days_until_expiration(), Some(5));
        assert_eq!(secret.expiration_display(), "⚠️ 5 days");
    }

    // ========================
    // SecretsStore tests
    // ========================

    #[test]
    fn test_store_new_is_empty() {
        let store = SecretsStore::new();
        assert!(store.secrets.is_empty());
        assert!(store.list_secrets().is_empty());
    }

    #[test]
    fn test_store_add_and_get_secret() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let key = test_key();
        let mut store = SecretsStore::new();

        store
            .add_secret(
                "MY_API_KEY".to_string(),
                "secret_value_123".to_string(),
                None,
                &temp_dir.path().to_path_buf(),
                &key,
            )
            .expect("Failed to add secret");

        assert_eq!(store.secrets.len(), 1);
        assert!(store.get_secret("MY_API_KEY").is_some());
        assert!(store.get_secret("NONEXISTENT").is_none());
    }

    #[test]
    fn test_store_decrypt_secret() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let key = test_key();
        let mut store = SecretsStore::new();

        store
            .add_secret(
                "DB_PASSWORD".to_string(),
                "super_secure_password".to_string(),
                None,
                &temp_dir.path().to_path_buf(),
                &key,
            )
            .expect("Failed to add secret");

        let decrypted = store
            .decrypt_secret("DB_PASSWORD", &key)
            .expect("Failed to decrypt");
        assert_eq!(decrypted, "super_secure_password");
    }

    #[test]
    fn test_store_decrypt_nonexistent_fails() {
        let store = SecretsStore::new();
        let key = test_key();

        let result = store.decrypt_secret("NONEXISTENT", &key);
        assert!(result.is_err());
    }

    #[test]
    fn test_store_delete_secret() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let key = test_key();
        let mut store = SecretsStore::new();

        store
            .add_secret(
                "TO_DELETE".to_string(),
                "value".to_string(),
                None,
                &temp_dir.path().to_path_buf(),
                &key,
            )
            .expect("Failed to add secret");

        assert!(store.get_secret("TO_DELETE").is_some());

        store
            .delete_secret("TO_DELETE", &temp_dir.path().to_path_buf(), &key)
            .expect("Failed to delete");

        assert!(store.get_secret("TO_DELETE").is_none());
    }

    #[test]
    fn test_store_list_secrets_sorted() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let key = test_key();
        let mut store = SecretsStore::new();

        // Add in non-alphabetical order
        for name in ["ZEBRA", "ALPHA", "MIDDLE"] {
            store
                .add_secret(
                    name.to_string(),
                    "value".to_string(),
                    None,
                    &temp_dir.path().to_path_buf(),
                    &key,
                )
                .expect("Failed to add secret");
        }

        let secrets = store.list_secrets();
        let names: Vec<_> = secrets.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(names, vec!["ALPHA", "MIDDLE", "ZEBRA"]);
    }

    #[test]
    fn test_store_decrypt_all() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let key = test_key();
        let mut store = SecretsStore::new();

        store
            .add_secret(
                "KEY1".to_string(),
                "value1".to_string(),
                None,
                &temp_dir.path().to_path_buf(),
                &key,
            )
            .expect("Failed to add secret");
        store
            .add_secret(
                "KEY2".to_string(),
                "value2".to_string(),
                None,
                &temp_dir.path().to_path_buf(),
                &key,
            )
            .expect("Failed to add secret");

        let all = store.decrypt_all(&key).expect("Failed to decrypt all");
        assert_eq!(all.len(), 2);
        assert_eq!(all.get("KEY1").unwrap(), "value1");
        assert_eq!(all.get("KEY2").unwrap(), "value2");
    }

    #[test]
    fn test_store_save_and_load() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let key = test_key();
        let mut store = SecretsStore::new();

        store
            .add_secret(
                "PERSISTENT".to_string(),
                "saved_value".to_string(),
                Some(30), // 30 days expiration
                &temp_dir.path().to_path_buf(),
                &key,
            )
            .expect("Failed to add secret");

        // Load from disk
        let loaded = SecretsStore::load(&temp_dir.path().to_path_buf(), &key)
            .expect("Failed to load store");

        assert_eq!(loaded.secrets.len(), 1);
        let decrypted = loaded
            .decrypt_secret("PERSISTENT", &key)
            .expect("Failed to decrypt");
        assert_eq!(decrypted, "saved_value");

        // Check expiration was saved
        let secret = loaded.get_secret("PERSISTENT").unwrap();
        assert!(secret.expires_at.is_some());
    }

    #[test]
    fn test_store_load_nonexistent_creates_empty() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let key = test_key();

        let store = SecretsStore::load(&temp_dir.path().to_path_buf(), &key)
            .expect("Failed to load store");

        assert!(store.secrets.is_empty());
    }

    #[test]
    fn test_store_add_secret_with_expiration() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let key = test_key();
        let mut store = SecretsStore::new();

        store
            .add_secret(
                "EXPIRING".to_string(),
                "temp_value".to_string(),
                Some(7), // 7 days
                &temp_dir.path().to_path_buf(),
                &key,
            )
            .expect("Failed to add secret");

        let secret = store.get_secret("EXPIRING").unwrap();
        assert!(secret.expires_at.is_some());
        
        // Should expire in approximately 7 days
        let days = secret.days_until_expiration().unwrap();
        assert!(days >= 6 && days <= 7);
    }

    #[test]
    fn test_store_unicode_secret_names_and_values() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let key = test_key();
        let mut store = SecretsStore::new();

        store
            .add_secret(
                "日本語_KEY".to_string(),
                "Valeur avec émojis 🔐🔑".to_string(),
                None,
                &temp_dir.path().to_path_buf(),
                &key,
            )
            .expect("Failed to add unicode secret");

        let decrypted = store
            .decrypt_secret("日本語_KEY", &key)
            .expect("Failed to decrypt");
        assert_eq!(decrypted, "Valeur avec émojis 🔐🔑");
    }
}