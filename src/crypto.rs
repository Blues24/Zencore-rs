use aes_gcm::{
    aead::{Aead, KeyInit, OsRng},
    Aes256Gcm, Nonce as AesNonce,
};
use anyhow::{Context, Result};
use argon2::{
    password_hash::{rand_core::RngCore, PasswordHasher, SaltString},
    Argon2,
};
use blake3;
use chacha20poly1305::{ChaCha20Poly1305, Nonce as ChachaNonce};
use sha2::{Digest, Sha256};
use sha3::Sha3_256;
use std::fs::{self, File};
use std::io::{BufReader, Read, Write};

#[derive(Debug, Clone, Copy)]
pub enum CipherAlgorithm {
    Aes256Gcm,
    ChaCha20Poly1305,
}

impl CipherAlgorithm {
    pub fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "aes-256" | "aes256" | "aes" => Ok(CipherAlgorithm::Aes256Gcm),
            "chacha20" | "chacha" => Ok(CipherAlgorithm::ChaCha20Poly1305),
            _ => Err(anyhow::anyhow!("Unknown cipher: {}", s)),
        }
    }

    pub fn name(&self) -> &str {
        match self {
            CipherAlgorithm::Aes256Gcm => "AES-256-GCM",
            CipherAlgorithm::ChaCha20Poly1305 => "ChaCha20-Poly1305",
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum HashAlgorithm {
    Sha256,
    Sha3_256,
    Blake3,
}

impl HashAlgorithm {
    pub fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "sha256" | "sha-256" => Ok(HashAlgorithm::Sha256),
            "sha3" | "sha3-256" => Ok(HashAlgorithm::Sha3_256),
            "blake3" => Ok(HashAlgorithm::Blake3),
            _ => Err(anyhow::anyhow!("Unknown hash algorithm: {}", s)),
        }
    }

    pub fn name(&self) -> &str {
        match self {
            HashAlgorithm::Sha256 => "SHA-256",
            HashAlgorithm::Sha3_256 => "SHA3-256",
            HashAlgorithm::Blake3 => "BLAKE3",
        }
    }
}

pub struct Encryptor {
    password: String,
    cipher: CipherAlgorithm,
}

impl Encryptor {
    pub fn new(password: String) -> Self {
        Self {
            password,
            cipher: CipherAlgorithm::Aes256Gcm,
        }
    }

    pub fn with_cipher(password: String, cipher: CipherAlgorithm) -> Self {
        Self { password, cipher }
    }

    fn derive_key(&self) -> Result<([u8; 32], SaltString)> {
        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();

        let password_hash = argon2
            .hash_password(self.password.as_bytes(), &salt)
            .map_err(|e| anyhow::anyhow!("Password hashing failed: {}", e))?;

        let hash_string = password_hash.hash.context("Failed to extract hash")?;
        let hash_bytes = hash_string.as_bytes();

        let mut key = [0u8; 32];
        key.copy_from_slice(&hash_bytes[..32]);

        Ok((key, salt))
    }

    pub fn encrypt_file(&self, file_path: &str) -> Result<String> {
        crate::utils::print_info(&format!("Encrypting with {}...", self.cipher.name()));

        let plaintext = fs::read(file_path)?;
        let (key, salt) = self.derive_key()?;

        let (ciphertext, nonce_bytes) = match self.cipher {
            CipherAlgorithm::Aes256Gcm => {
                let cipher = Aes256Gcm::new_from_slice(&key)?;
                let mut nonce_bytes = [0u8; 12];
                OsRng.fill_bytes(&mut nonce_bytes);
                let nonce = AesNonce::from_slice(&nonce_bytes);

                let ciphertext = cipher
                    .encrypt(nonce, plaintext.as_ref())
                    .map_err(|e| anyhow::anyhow!("AES encryption failed: {}", e))?;

                (ciphertext, nonce_bytes.to_vec())
            }
            CipherAlgorithm::ChaCha20Poly1305 => {
                let cipher = ChaCha20Poly1305::new_from_slice(&key)?;
                let mut nonce_bytes = [0u8; 12];
                OsRng.fill_bytes(&mut nonce_bytes);
                let nonce = ChachaNonce::from_slice(&nonce_bytes);

                let ciphertext = cipher
                    .encrypt(nonce, plaintext.as_ref())
                    .map_err(|e| anyhow::anyhow!("ChaCha20 encryption failed: {}", e))?;

                (ciphertext, nonce_bytes.to_vec())
            }
        };

        let backup_path = format!("{}.bak", file_path);
        fs::rename(file_path, &backup_path)?;

        let mut encrypted_file = File::create(file_path)?;
        encrypted_file.write_all(&[1u8])?; // version
        encrypted_file.write_all(&[match self.cipher {
            CipherAlgorithm::Aes256Gcm => 0u8,
            CipherAlgorithm::ChaCha20Poly1305 => 1u8,
        }])?;
        encrypted_file.write_all(salt.as_str().as_bytes())?;
        encrypted_file.write_all(&nonce_bytes)?;
        encrypted_file.write_all(&ciphertext)?;

        crate::utils::print_success(&format!("Encrypted with {}", self.cipher.name()));

        Ok(file_path.to_string())
    }

    pub fn decrypt_file(&self, file_path: &str) -> Result<String> {
        let encrypted_data = fs::read(file_path)?;

        if encrypted_data.len() < 36 {
            return Err(anyhow::anyhow!("Invalid encrypted file format"));
        }

        let version = encrypted_data[0];
        if version != 1 {
            return Err(anyhow::anyhow!("Unsupported encryption version: {}", version));
        }

        let cipher_id = encrypted_data[1];
        let detected_cipher = match cipher_id {
            0 => CipherAlgorithm::Aes256Gcm,
            1 => CipherAlgorithm::ChaCha20Poly1305,
            _ => return Err(anyhow::anyhow!("Unknown cipher ID: {}", cipher_id)),
        };

        crate::utils::print_info(&format!("Detected cipher: {}", detected_cipher.name()));

        let salt_str = std::str::from_utf8(&encrypted_data[2..24])?;
        let nonce_bytes = &encrypted_data[24..36];
        let ciphertext = &encrypted_data[36..];

        let salt = SaltString::from_b64(salt_str)
            .map_err(|e| anyhow::anyhow!("Invalid salt: {}", e))?;
        let argon2 = Argon2::default();

        let password_hash = argon2
            .hash_password(self.password.as_bytes(), &salt)
            .map_err(|e| anyhow::anyhow!("Password hashing failed: {}", e))?;

        let hash_string = password_hash.hash.context("Failed to extract hash")?;
        let hash_bytes = hash_string.as_bytes();

        let mut key = [0u8; 32];
        key.copy_from_slice(&hash_bytes[..32]);

        let plaintext = match detected_cipher {
            CipherAlgorithm::Aes256Gcm => {
                let cipher = Aes256Gcm::new_from_slice(&key)?;
                let nonce = AesNonce::from_slice(nonce_bytes);

                cipher
                    .decrypt(nonce, ciphertext)
                    .map_err(|e| anyhow::anyhow!("AES decryption failed (wrong password?): {}", e))?
            }
            CipherAlgorithm::ChaCha20Poly1305 => {
                let cipher = ChaCha20Poly1305::new_from_slice(&key)?;
                let nonce = ChachaNonce::from_slice(nonce_bytes);

                cipher
                    .decrypt(nonce, ciphertext)
                    .map_err(|e| anyhow::anyhow!("ChaCha20 decryption failed (wrong password?): {}", e))?
            }
        };

        fs::write(file_path, plaintext)?;

        crate::utils::print_success("File decrypted successfully");

        Ok(file_path.to_string())
    }
}

pub struct Checker;

impl Checker {
    pub fn generate_checksum(file_path: &str, algorithm: HashAlgorithm) -> Result<String> {
        let file = File::open(file_path)?;
        let mut reader = BufReader::new(file);

        match algorithm {
            HashAlgorithm::Sha256 => {
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
            HashAlgorithm::Sha3_256 => {
                let mut hasher = Sha3_256::new();
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
            HashAlgorithm::Blake3 => {
                let mut hasher = blake3::Hasher::new();
                let mut buffer = [0u8; 8192];

                loop {
                    let count = reader.read(&mut buffer)?;
                    if count == 0 {
                        break;
                    }
                    hasher.update(&buffer[..count]);
                }

                Ok(hasher.finalize().to_hex().to_string())
            }
        }
    }

    pub fn verify_checksum(
        file_path: &str,
        expected: &str,
        algorithm: HashAlgorithm,
    ) -> Result<bool> {
        let actual = Self::generate_checksum(file_path, algorithm)?;
        Ok(actual.eq_ignore_ascii_case(expected))
    }
}