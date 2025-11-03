use aes_gcm:: {
    aead::{Aead, KeyInit, OsRng},
    Aes256Gcm, Nonce,
};

use anyhow::{Context, Result};
use argon2::{
    password_hash::{rand_core::RngCore, SaltString},
    Argon2, PasswordHasher,
};

use sha2::{Digest, Sha256};
use std::fs::{self, File};
use std::io::{BufReader, Read, Write};

pub struct Encryptor {
    password: String,
}

impl Encryptor {
    pub fn new(password: String) -> Self {
        Self { password }
    }

    pub fn encrypt_file(&self, file_path: &str) -> Result<String> {
        // Read file
        let plaintext = fs::read(file_path)?;
        
        // Derive key from password using Argon2
        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();

        let password_hash = argon2
            .hash_password(self.password.as_bytes(), &salt)?
            .hash
            .context("Failed to hash password")?;

        let key = password_hash.as_bytes();
        let cipher = Aes256Gcm::new_from_slice(&key[..32])?;

        let mut nonce_bytes = [0u8; 12];
        OsRng.fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        let ciphertext = cipher
            .encrypt(nonce, plaintext.as_ref())
            .map_err(|e| anyhow::anyhow!("Encryption failed: {}", e))?;

        let backup_path = format!("{}.bak", file_path);
        fs::rename(file_path, &backup_path)?;

        let mut encrypted_file = File::create(file_path)?;

        encrypted_file.write_all(salt.as_str().as_bytes())?;
        encrypted_file.write_all(&nonce_bytes)?;
        encrypted_file.write_all(&ciphertext)?;

        crate::utils::print_success("File encrypted successfully");

        Ok(file_path..to_string())
    }

    pub fn decrypt_file(&self, file_path: &str) -> Result<String> {
        let encrypted_data = fs::read(file_path)?;

        // parse metadata
        if  encrypted_data.len() < 34 {
            return Err(anyhow::anyhow!("Invalid encrypted file"));

        }

        let salt_str = std::str::from_utf8(&encrypted_data[..22])?;
        let nonce_bytes = &encrypted_data[22..34];
        let ciphertext = &encrypted_data[34..];

        // Derive key
        let salt = SaltString::from_b64(salt_str)?;
        let argon2 = Argon2::default();

        let password_hash = argon2
            .hash_password(self.password.as_bytes(), &salt)?
            .hash
            .context("Failed to hash password")?;

        let key = password_hash.as_bytes();
        let cipher = Aes256Gcm::new_from_slice(&key[..32])?;
        let nonce = Nonce::from_slice(nonce_bytes);

        // Decrypt process
        let plaintext = cipher
            .decrypt(nonce, ciphertext)
            .map_err(|e| anyhow::anyhow!("Decryption failed: {}", e))?;

        // Write decrypted file
        fs::write(file_path, plaintext)?;

        crate::utils::print_success("File decrypted successfully");

        Ok(file_path.to_string())

    }
}

pub struct Checker;

impl Checker {
    pub fn generate_checksum(file_path: &str) -> Result<String> {
        let file = File::open(file_path)?;
        let mut reader = BufReader::new(file);
        let mut hasher = Sha256::new();
        let mut buffer = [0u8; 8192];

        loop {
            let count = reader.read(&mut buffer)?;
            if count == 0 {
                break;
            }

            hasher.update(&buffer[..count]);
        }

        Ok(format!("{:x}", hasher.finalize()))
    }

    pub fn verify_checksum(file_path: &str, expected: &str) -> Result<bool> {
        let actual = Self::generate_checksum(file_path)?;
        Ok(actual == expected)
    }
}