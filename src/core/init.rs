use anyhow::Result;
use argon2::{
    Argon2,
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString, rand_core::OsRng},
};
use directories::BaseDirs;
use std::path::PathBuf;
use zeroize::Zeroize;

pub struct Locker {
    base_dir: PathBuf,
    key: Option<Vec<u8>>, // Key derived from passphrase, zeroized at end
}

impl Locker {
    /// Tries to create the locker without prompt (checks if already initialized).
    #[allow(dead_code)]
    pub fn try_new() -> Result<Self> {
        let base_dirs = BaseDirs::new()
            .ok_or_else(|| anyhow::anyhow!("Unable to determine user directories"))?;
        let config_dir = base_dirs.config_dir();

        #[cfg(unix)]
        let sub_dir = ".lazy-locker";
        #[cfg(not(unix))]
        let sub_dir = "lazy-locker";

        let locker_dir = config_dir.join(sub_dir);
        std::fs::create_dir_all(&locker_dir)?;

        let salt_path = locker_dir.join("salt");
        if !salt_path.exists() {
            return Err(anyhow::anyhow!("Locker not initialized"));
        }

        // To load, we need passphrase, so return error to indicate it's needed
        Err(anyhow::anyhow!("Passphrase required to load locker"))
    }

    /// Initializes or loads the locker with the provided passphrase.
    pub fn init_or_load_with_passphrase(passphrase: &str) -> Result<Self> {
        let base_dirs = BaseDirs::new()
            .ok_or_else(|| anyhow::anyhow!("Unable to determine user directories"))?;
        let config_dir = base_dirs.config_dir();

        #[cfg(unix)]
        let sub_dir = ".lazy-locker";
        #[cfg(not(unix))]
        let sub_dir = "lazy-locker";

        let locker_dir = config_dir.join(sub_dir);
        std::fs::create_dir_all(&locker_dir)?;

        let salt_path = locker_dir.join("salt");
        let key = if salt_path.exists() {
            Self::load_key(&locker_dir, passphrase)?
        } else {
            Self::init_key(&locker_dir, passphrase)?
        };

        Ok(Self {
            base_dir: locker_dir,
            key: Some(key),
        })
    }

    /// Initializes the key for the first time: generates salt, asks passphrase, derives key.
    fn init_key(locker_dir: &std::path::Path, passphrase: &str) -> Result<Vec<u8>> {
        let salt = SaltString::generate(&mut OsRng);
        std::fs::write(locker_dir.join("salt"), salt.as_str())?;

        let argon2 = Argon2::default();
        let hash = argon2
            .hash_password(passphrase.as_bytes(), &salt)
            .map_err(|e| anyhow::anyhow!("Hash error: {}", e))?
            .to_string();
        std::fs::write(locker_dir.join("hash"), &hash)?;

        let mut key = [0u8; 32];
        let mut salt_bytes = [0u8; 16];
        salt.decode_b64(&mut salt_bytes)
            .map_err(|e| anyhow::anyhow!("Salt decoding error: {}", e))?;
        argon2
            .hash_password_into(passphrase.as_bytes(), &salt_bytes, &mut key)
            .map_err(|e| anyhow::anyhow!("Key derivation error: {}", e))?;

        Ok(key.to_vec())
    }

    /// Loads existing key: reads salt, asks passphrase, verifies and derives.
    fn load_key(locker_dir: &std::path::Path, passphrase: &str) -> Result<Vec<u8>> {
        let salt_str = std::fs::read_to_string(locker_dir.join("salt"))?;
        let salt =
            SaltString::from_b64(&salt_str).map_err(|e| anyhow::anyhow!("Salt error: {}", e))?;

        let hash_str = std::fs::read_to_string(locker_dir.join("hash"))?;
        let expected_hash =
            PasswordHash::new(&hash_str).map_err(|e| anyhow::anyhow!("Hash error: {}", e))?;

        let argon2 = Argon2::default();
        argon2
            .verify_password(passphrase.as_bytes(), &expected_hash)
            .map_err(|e| anyhow::anyhow!("Incorrect passphrase: {}", e))?;

        let mut salt_bytes = [0u8; 16];
        salt.decode_b64(&mut salt_bytes)
            .map_err(|e| anyhow::anyhow!("Salt decoding error: {}", e))?;
        let mut key = [0u8; 32];
        argon2
            .hash_password_into(passphrase.as_bytes(), &salt_bytes, &mut key)
            .map_err(|e| anyhow::anyhow!("Key derivation error: {}", e))?;

        Ok(key.to_vec())
    }

    /// Returns the path to a file in the locker.
    #[allow(dead_code)]
    pub fn get_path(&self, filename: &str) -> PathBuf {
        self.base_dir.join(filename)
    }

    /// Returns the key for encryption/decryption (use temporarily).
    pub fn get_key(&self) -> Option<&[u8]> {
        self.key.as_deref()
    }

    /// Returns the locker base directory.
    pub fn base_dir(&self) -> &PathBuf {
        &self.base_dir
    }
}

impl Drop for Locker {
    fn drop(&mut self) {
        if let Some(ref mut key) = self.key {
            key.zeroize();
        }
    }
}
