use aes_gcm::{
    aead::{Aead, KeyInit, OsRng},
    Aes256Gcm, Nonce as AesNonce,
};
use chacha20poly1305::{
    ChaCha20Poly1305,
    Nonce as ChachaNonce,
};
use anyhow::{Context, Result};
use argon2::{
    password_hash::{rand_core::RngCore, SaltString},
    Argon2, PasswordHasher,
};
use blake3;
use sha2::{Digest, Sha256};
use sha3::Sha3_256;
use std::fs::{self, File};
use std::io::{BufReader, Read, Write};


#[derive(Debug, Clone, Copy)]
pub enum CipherAlgorithm{
    // AES-256-GCM (Default, Hardware accelerated on modern CPUs)
    Aes256Gcm,
    /// ChaCha20-Poly1305 (Pure software, constant-time, good for older CPUs)
    ChaCha20Poly1305
}

impl CipherAlgorithm {
    pub fn from_str(s: &str) -> Result<Self>{
        match s.to_lowercase().to_str(){
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

/// Supported hash algorithms for checksums
#[derive(Debug, Clone, Copy)]
pub enum HashAlgorithm {
    /// SHA-256 (Default, widely supported)
    Sha256,
    /// SHA3-256 (Modern, NIST standard)
    Sha3_256,
    /// BLAKE3 (Fastest, parallel processing)
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

/// Advanced encryptor with multiple cipher support
pub struct AdvancedEncryptor {
    password: String,
    cipher: CipherAlgorithm,
}

impl AdvancedEncryptor {
    /// Create new encryptor with specified cipher
    ///
    /// # Arguments
    /// * `password` - User password for key derivation
    /// * `cipher` - Cipher algorithm to use
    pub fn new(password: String, cipher: CipherAlgorithm) -> Self {
        Self { password, cipher }
    }
    
    /// Derive 32-byte key from password using Argon2
    /// 
    /// Argon2 is memory-hard, making brute-force attacks expensive
    /// Uses random salt for each encryption
    fn derive_key(&self) -> Result<([u8; 32], SaltString)> {
        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();
        
        let password_hash = argon2
            .hash_password(self.password.as_bytes(), &salt)?
            .hash
            .context("Failed to hash password")?;
        
        let mut key = [0u8; 32];
        let hash_bytes = password_hash.as_bytes();
        key.copy_from_slice(&hash_bytes[..32]);
        
        Ok((key, salt))
    }
    
    /// Encrypt file with selected cipher
    ///
    /// File format: [version(1)][cipher_id(1)][salt(22)][nonce(12)][ciphertext]
    pub fn encrypt_file(&self, file_path: &str) -> Result<String> {
        crate::utils::print_info(&format!("Encrypting with {}...", self.cipher.name()));
        
        // Read plaintext
        let plaintext = fs::read(file_path)?;
        
        // Derive key
        let (key, salt) = self.derive_key()?;
        
        // Encrypt based on cipher
        let (ciphertext, nonce_bytes) = match self.cipher {
            CipherAlgorithm::Aes256Gcm => {
                // AES-256-GCM encryption
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
                // ChaCha20-Poly1305 encryption
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
        
        // Backup original file
        let backup_path = format!("{}.bak", file_path);
        fs::rename(file_path, &backup_path)?;
        
        // Write encrypted file with metadata
        let mut encrypted_file = File::create(file_path)?;
        
        // Version byte (for future compatibility)
        encrypted_file.write_all(&[1u8])?;
        
        // Cipher ID (0=AES, 1=ChaCha20)
        let cipher_id = match self.cipher {
            CipherAlgorithm::Aes256Gcm => 0u8,
            CipherAlgorithm::ChaCha20Poly1305 => 1u8,
        };
        encrypted_file.write_all(&[cipher_id])?;
        
        // Salt (22 bytes)
        encrypted_file.write_all(salt.as_str().as_bytes())?;
        
        // Nonce (12 bytes)
        encrypted_file.write_all(&nonce_bytes)?;
        
        // Ciphertext
        encrypted_file.write_all(&ciphertext)?;
        
        crate::utils::print_success(&format!("Encrypted with {}", self.cipher.name()));
        
        Ok(file_path.to_string())
    }
    
    /// Decrypt file (auto-detects cipher from metadata)
    pub fn decrypt_file(&self, file_path: &str) -> Result<String> {
        let encrypted_data = fs::read(file_path)?;
        
        // Parse metadata
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
        
        // Derive key with same salt
        let salt = SaltString::from_b64(salt_str)?;
        let argon2 = Argon2::default();
        
        let password_hash = argon2
            .hash_password(self.password.as_bytes(), &salt)?
            .hash
            .context("Failed to hash password")?;
        
        let mut key = [0u8; 32];
        let hash_bytes = password_hash.as_bytes();
        key.copy_from_slice(&hash_bytes[..32]);
        
        // Decrypt based on detected cipher
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
        
        // Write decrypted file
        fs::write(file_path, plaintext)?;
        
        crate::utils::print_success("File decrypted successfully");
        
        Ok(file_path.to_string())
    }
}

/// Advanced checksum generator with multiple hash algorithms
pub struct AdvancedChecker;

impl AdvancedChecker {
    /// Generate checksum using specified algorithm
    ///
    /// # Performance:
    /// - BLAKE3: ~7 GB/s (parallel, fastest)
    /// - SHA-256: ~500 MB/s (single-threaded)
    /// - SHA3-256: ~200 MB/s (single-threaded)
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
                // BLAKE3 is fastest, especially for large files
                // Uses SIMD and parallel processing automatically
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
    
    /// Verify checksum against expected value
    pub fn verify_checksum(
        file_path: &str,
        expected: &str,
        algorithm: HashAlgorithm,
    ) -> Result<bool> {
        let actual = Self::generate_checksum(file_path, algorithm)?;
        Ok(actual.eq_ignore_ascii_case(expected))
    }
}