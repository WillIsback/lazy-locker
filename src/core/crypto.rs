use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Key, Nonce};
use anyhow::Result;
use rand::Rng;

pub fn encrypt(data: &[u8], key: &[u8]) -> Result<Vec<u8>> {
    let key = Key::<Aes256Gcm>::from_slice(key);
    let cipher = Aes256Gcm::new(key);
    let nonce: [u8; 12] = rand::rng().random();
    let ciphertext = cipher
        .encrypt(Nonce::from_slice(&nonce), data)
        .map_err(|e| anyhow::anyhow!("Encryption error: {}", e))?;
    let mut result = nonce.to_vec();
    result.extend(ciphertext);
    Ok(result)
}

pub fn decrypt(data: &[u8], key: &[u8]) -> Result<Vec<u8>> {
    let key = Key::<Aes256Gcm>::from_slice(key);
    let cipher = Aes256Gcm::new(key);
    let (nonce, ciphertext) = data.split_at(12);
    let plaintext = cipher
        .decrypt(Nonce::from_slice(nonce), ciphertext)
        .map_err(|e| anyhow::anyhow!("Decryption error: {}", e))?;
    Ok(plaintext)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Generates a valid 32-byte test key
    fn test_key() -> [u8; 32] {
        [0x42u8; 32] // Deterministic key for testing
    }

    #[test]
    fn test_encrypt_produces_different_output() {
        let key = test_key();
        let plaintext = b"my_secret_value";

        let encrypted = encrypt(plaintext, &key).expect("Encryption should succeed");

        // Encrypted data should be different from plaintext
        assert_ne!(encrypted.as_slice(), plaintext);
        // Encrypted data should include nonce (12 bytes) + ciphertext + tag (16 bytes)
        assert!(encrypted.len() > plaintext.len() + 12);
    }

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let key = test_key();
        let plaintext = b"sensitive_api_key_12345";

        let encrypted = encrypt(plaintext, &key).expect("Encryption should succeed");
        let decrypted = decrypt(&encrypted, &key).expect("Decryption should succeed");

        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_encrypt_different_nonces() {
        let key = test_key();
        let plaintext = b"same_value";

        // Two encryptions of the same data should produce different ciphertexts
        // (due to random nonce)
        let encrypted1 = encrypt(plaintext, &key).expect("Encryption 1 should succeed");
        let encrypted2 = encrypt(plaintext, &key).expect("Encryption 2 should succeed");

        assert_ne!(
            encrypted1, encrypted2,
            "Same plaintext should encrypt to different ciphertext"
        );
    }

    #[test]
    fn test_decrypt_with_wrong_key_fails() {
        let key1 = test_key();
        let key2 = [0x99u8; 32]; // Different key
        let plaintext = b"secret";

        let encrypted = encrypt(plaintext, &key1).expect("Encryption should succeed");
        let result = decrypt(&encrypted, &key2);

        assert!(result.is_err(), "Decryption with wrong key should fail");
    }

    #[test]
    fn test_decrypt_corrupted_data_fails() {
        let key = test_key();
        let plaintext = b"secret";

        let mut encrypted = encrypt(plaintext, &key).expect("Encryption should succeed");
        // Corrupt the ciphertext (after the 12-byte nonce)
        if encrypted.len() > 15 {
            encrypted[15] ^= 0xFF;
        }

        let result = decrypt(&encrypted, &key);
        assert!(result.is_err(), "Decryption of corrupted data should fail");
    }

    #[test]
    fn test_encrypt_empty_data() {
        let key = test_key();
        let plaintext = b"";

        let encrypted = encrypt(plaintext, &key).expect("Encryption of empty data should succeed");
        let decrypted = decrypt(&encrypted, &key).expect("Decryption should succeed");

        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_encrypt_large_data() {
        let key = test_key();
        let plaintext: Vec<u8> = (0..10_000).map(|i| (i % 256) as u8).collect();

        let encrypted = encrypt(&plaintext, &key).expect("Encryption of large data should succeed");
        let decrypted = decrypt(&encrypted, &key).expect("Decryption should succeed");

        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_encrypt_unicode_data() {
        let key = test_key();
        let plaintext = "Clé secrète: 日本語 🔐 émojis".as_bytes();

        let encrypted = encrypt(plaintext, &key).expect("Encryption should succeed");
        let decrypted = decrypt(&encrypted, &key).expect("Decryption should succeed");

        assert_eq!(decrypted, plaintext);
        assert_eq!(
            String::from_utf8(decrypted).unwrap(),
            "Clé secrète: 日本語 🔐 émojis"
        );
    }
}
