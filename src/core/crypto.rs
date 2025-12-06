use aes_gcm::{Aes256Gcm, Key, Nonce};
use aes_gcm::aead::{Aead, KeyInit};
use anyhow::Result;
use rand::Rng;

pub fn encrypt(data: &[u8], key: &[u8]) -> Result<Vec<u8>> {
    let key = Key::<Aes256Gcm>::from_slice(key);
    let cipher = Aes256Gcm::new(key);
    let nonce: [u8; 12] = rand::rng().random();
    let ciphertext = cipher.encrypt(Nonce::from_slice(&nonce), data).map_err(|e| anyhow::anyhow!("Encryption error: {}", e))?;
    let mut result = nonce.to_vec();
    result.extend(ciphertext);
    Ok(result)
}

pub fn decrypt(data: &[u8], key: &[u8]) -> Result<Vec<u8>> {
    let key = Key::<Aes256Gcm>::from_slice(key);
    let cipher = Aes256Gcm::new(key);
    let (nonce, ciphertext) = data.split_at(12);
    let plaintext = cipher.decrypt(Nonce::from_slice(nonce), ciphertext).map_err(|e| anyhow::anyhow!("Decryption error: {}", e))?;
    Ok(plaintext)
}