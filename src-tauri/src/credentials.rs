use aes_gcm::aead::Aead;
use aes_gcm::{Aes256Gcm, KeyInit};
use pbkdf2::pbkdf2_hmac;
use rand::{rngs::OsRng, RngCore};
use serde::{Deserialize, Serialize};
use sha2::Sha512;
use std::fs;
use std::path::PathBuf;
use tauri::{AppHandle, Manager, Runtime};
use zeroize::{Zeroize, ZeroizeOnDrop};

const PBKDF2_ITERATIONS: u32 = 600_000;
const SALT_SIZE: usize = 16;
const NONCE_SIZE: usize = 12;
const KEY_SIZE: usize = 32;
const CREDENTIALS_FILE: &str = "credentials.json";

#[derive(Serialize, Deserialize)]
pub struct EncryptedPayload {
    ciphertext: String, // hex encoded
    nonce: String,      // hex encoded
    salt: String,       // hex encoded
}

// SensitiveBuffer wrapper that zeroizes memory on drop
#[derive(Zeroize, ZeroizeOnDrop)]
struct SensitiveBuffer {
    key: [u8; KEY_SIZE],
}

fn get_credentials_path<R: Runtime>(app_handle: &AppHandle<R>) -> Result<PathBuf, String> {
    let app_dir = app_handle
        .path()
        .app_local_data_dir()
        .map_err(|e| format!("Failed to get app local data dir: {}", e))?;
    Ok(app_dir.join(CREDENTIALS_FILE))
}

pub fn encrypt_api_key(passphrase: &str, api_key: &str) -> Result<EncryptedPayload, String> {
    let mut salt = [0u8; SALT_SIZE];
    OsRng.fill_bytes(&mut salt);

    let mut key_bytes = [0u8; KEY_SIZE];
    pbkdf2_hmac::<Sha512>(
        passphrase.as_bytes(),
        &salt,
        PBKDF2_ITERATIONS,
        &mut key_bytes,
    );
    let mut key = SensitiveBuffer { key: key_bytes };

    let cipher = Aes256Gcm::new_from_slice(&key.key)
        .map_err(|e| format!("Failed to create cipher: {}", e))?;

    // Safe zeroization
    key.key.zeroize();

    let mut nonce_bytes = [0u8; NONCE_SIZE];
    OsRng.fill_bytes(&mut nonce_bytes);
    let nonce = aes_gcm::Nonce::from_slice(&nonce_bytes);

    let ciphertext = cipher
        .encrypt(nonce, api_key.as_bytes())
        .map_err(|e| format!("Encryption failed: {}", e))?;

    Ok(EncryptedPayload {
        ciphertext: hex::encode(ciphertext),
        nonce: hex::encode(nonce_bytes),
        salt: hex::encode(salt),
    })
}

pub fn decrypt_api_key(passphrase: &str, payload: &EncryptedPayload) -> Result<String, String> {
    let salt = hex::decode(&payload.salt).map_err(|e| format!("Invalid salt encoding: {}", e))?;
    let nonce_bytes =
        hex::decode(&payload.nonce).map_err(|e| format!("Invalid nonce encoding: {}", e))?;
    let ciphertext = hex::decode(&payload.ciphertext)
        .map_err(|e| format!("Invalid ciphertext encoding: {}", e))?;

    let mut key_bytes = [0u8; KEY_SIZE];
    pbkdf2_hmac::<Sha512>(
        passphrase.as_bytes(),
        &salt,
        PBKDF2_ITERATIONS,
        &mut key_bytes,
    );
    let mut key = SensitiveBuffer { key: key_bytes };

    let cipher = Aes256Gcm::new_from_slice(&key.key)
        .map_err(|e| format!("Failed to create cipher: {}", e))?;

    // Safe zeroization
    key.key.zeroize();

    let nonce = aes_gcm::Nonce::from_slice(&nonce_bytes);
    let decrypted_bytes = cipher
        .decrypt(nonce, ciphertext.as_slice())
        .map_err(|e| format!("Decryption failed (possibly incorrect passphrase): {}", e))?;

    let decrypted_str = String::from_utf8(decrypted_bytes)
        .map_err(|e| format!("Invalid UTF-8 content in decrypted key: {}", e))?;

    Ok(decrypted_str)
}

pub fn decrypt_stored_api_key<R: Runtime>(
    app_handle: &AppHandle<R>,
    passphrase: &str,
) -> Result<String, String> {
    let credentials_path = get_credentials_path(app_handle)?;
    if !credentials_path.exists() {
        return Err("No API Key configured.".to_string());
    }
    let file_content = fs::read_to_string(credentials_path)
        .map_err(|e| format!("Failed to read credentials: {}", e))?;
    let payload: EncryptedPayload = serde_json::from_str(&file_content)
        .map_err(|e| format!("Failed to parse credentials: {}", e))?;
    decrypt_api_key(passphrase, &payload)
}

#[tauri::command]
pub fn save_gemini_api_key<R: Runtime>(
    app_handle: AppHandle<R>,
    passphrase: String,
    api_key: String,
) -> Result<(), String> {
    let payload = encrypt_api_key(&passphrase, &api_key)?;
    let credentials_path = get_credentials_path(&app_handle)?;

    if let Some(parent) = credentials_path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("Failed to create directories: {}", e))?;
    }

    let json_content = serde_json::to_string(&payload)
        .map_err(|e| format!("Failed to serialize payload: {}", e))?;

    // Atomic write
    let temp_path = credentials_path.with_extension("tmp");
    fs::write(&temp_path, json_content)
        .map_err(|e| format!("Failed to write temp credentials file: {}", e))?;
    fs::rename(&temp_path, &credentials_path)
        .map_err(|e| format!("Failed to rename credentials file: {}", e))?;

    Ok(())
}

#[tauri::command]
pub fn has_gemini_api_key<R: Runtime>(app_handle: AppHandle<R>) -> Result<bool, String> {
    let credentials_path = get_credentials_path(&app_handle)?;
    Ok(credentials_path.exists())
}

#[tauri::command]
pub fn delete_gemini_api_key<R: Runtime>(app_handle: AppHandle<R>) -> Result<(), String> {
    let credentials_path = get_credentials_path(&app_handle)?;
    if credentials_path.exists() {
        fs::remove_file(credentials_path)
            .map_err(|e| format!("Failed to delete credentials file: {}", e))?;
    }
    Ok(())
}

// NOTA DE SEGURIDAD: check_passphrase está definido para futuras comprobaciones
// internas de contraseña, pero no está expuesto en el invoke_handler de lib.rs
// en esta iteración para minimizar la superficie de ataque RPC.
#[tauri::command]
pub fn check_passphrase<R: Runtime>(
    app_handle: AppHandle<R>,
    passphrase: String,
) -> Result<bool, String> {
    let credentials_path = get_credentials_path(&app_handle)?;
    if !credentials_path.exists() {
        return Ok(false);
    }

    let file_content = fs::read_to_string(credentials_path)
        .map_err(|e| format!("Failed to read credentials: {}", e))?;
    let payload: EncryptedPayload = serde_json::from_str(&file_content)
        .map_err(|e| format!("Failed to parse credentials: {}", e))?;

    match decrypt_api_key(&passphrase, &payload) {
        Ok(_) => Ok(true),
        Err(_) => Ok(false),
    }
}

// Unit tests for credentials
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encryption_decryption_roundtrip() {
        let passphrase = "my-secure-passphrase";
        let api_key = "AIzaSyDummyKeyContent";

        let encrypted = encrypt_api_key(passphrase, api_key).expect("Encryption should succeed");
        assert_ne!(encrypted.ciphertext, api_key);
        assert_ne!(encrypted.salt, "");
        assert_ne!(encrypted.nonce, "");

        let decrypted = decrypt_api_key(passphrase, &encrypted).expect("Decryption should succeed");
        assert_eq!(decrypted, api_key);
    }

    #[test]
    fn test_decryption_with_wrong_passphrase() {
        let passphrase = "my-secure-passphrase";
        let wrong_passphrase = "wrong-passphrase";
        let api_key = "AIzaSyDummyKeyContent";

        let encrypted = encrypt_api_key(passphrase, api_key).expect("Encryption should succeed");
        let result = decrypt_api_key(wrong_passphrase, &encrypted);

        assert!(
            result.is_err(),
            "Decryption with wrong passphrase must fail"
        );
    }
}
