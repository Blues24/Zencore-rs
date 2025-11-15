use aes_gcm::{
    aead::{Aead, KeyInit, OsRng},
    Aes256Gcm, Nonce,
};
use anyhow::{Context, Result};
use argon2::{
    password_hash::{rand_core::RngCore, PasswordHasher, SaltString},
    Argon2, Params, Version,
};
use indicatif::{ProgressBar, ProgressStyle};
use sha2::{Digest, Sha256};
use std::fs::{self, File};
use std::io::{BufReader, Read, Write};
use std::path::Path;

pub struct Encryptor {
    password: String,
}

impl Encryptor {
    pub fn new(password: String) -> Self {
        Self { password }
    }

    fn derive_key(&self, pb: &ProgressBar) -> Result<([u8; 32], SaltString)> {
        pb.set_message("🔑 Deriving encryption key...");
        
        let salt = SaltString::generate(&mut OsRng);
        
        let params = Params::new(
            32768,
            3,
            1,
            None
        ).map_err(|e| anyhow::anyhow!("Failed to create Argon2 params: {}", e))?;
        
        let argon2 = Argon2::new(
            argon2::Algorithm::Argon2id,
            Version::V0x13,
            params
        );

        let password_hash = argon2
            .hash_password(self.password.as_bytes(), &salt)
            .map_err(|e| anyhow::anyhow!("Password hashing failed: {}", e))?;

        let hash_string = password_hash.hash.context("Failed to extract hash")?;
        let hash_bytes = hash_string.as_bytes();

        let mut key = [0u8; 32];
        key.copy_from_slice(&hash_bytes[..32]);

        pb.set_message("✓ Key derived");

        Ok((key, salt))
    }

    pub fn encrypt_file(&self, file_path: &str) -> Result<String> {
        crate::utils::print_info("🔒 Encrypting with AES-256-GCM...");

        let file_size = fs::metadata(file_path)?.len();
        
        let pb = ProgressBar::new(100);
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} [{bar:40.cyan/blue}] {pos}% {msg}")
                .unwrap()
                .progress_chars("█▓░-"),
        );

        pb.set_position(10);
        pb.set_message("Reading file...");
        let plaintext = fs::read(file_path)?;
        
        pb.set_position(30);
        let (key, salt) = self.derive_key(&pb)?;
        
        pb.set_position(50);
        pb.set_message("Encrypting...");
        let cipher = Aes256Gcm::new_from_slice(&key)?;
        let mut nonce_bytes = [0u8; 12];
        OsRng.fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);
        
        let ciphertext = cipher
            .encrypt(nonce, plaintext.as_ref())
            .map_err(|e| anyhow::anyhow!("Encryption failed: {}", e))?;
        
        pb.set_position(80);
        pb.set_message("Writing...");
        
        let backup_path = format!("{}.bak", file_path);
        fs::rename(file_path, &backup_path)?;

        let mut encrypted_file = File::create(file_path)?;
        encrypted_file.write_all(salt.as_str().as_bytes())?;
        encrypted_file.write_all(&nonce_bytes)?;
        encrypted_file.write_all(&ciphertext)?;
        
        pb.finish_with_message("✓ Done!");

        crate::utils::print_success(&format!(
            "Encrypted ({:.2} MB)",
            file_size as f64 / 1_048_576.0
        ));

        Ok(file_path.to_string())
    }
}

pub enum HashAlgorithm {
    Sha256,
    Blake3,
    Sha3,
}

impl HashAlgorithm {
    pub fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "sha256" | "sha-256" => Ok(Self::Sha256),
            "blake3" => Ok(Self::Blake3),
            "sha3" | "sha3-256" => Ok(Self::Sha3),
            _ => Err(anyhow::anyhow!("Unknown algorithm: {}", s)),
        }
    }

    pub fn name(&self) -> &str {
        match self {
            Self::Sha256 => "SHA-256",
            Self::Blake3 => "BLAKE3",
            Self::Sha3 => "SHA3-256",
        }
    }
}

pub struct Checker;

impl Checker {
    pub fn generate_checksum(file_path: &str) -> Result<String> {
        Self::generate_checksum_with_algorithm(file_path, HashAlgorithm::Sha256)
    }

    pub fn generate_checksum_with_algorithm(
        file_path: &str, 
        algorithm: HashAlgorithm
    ) -> Result<String> {
        let file = File::open(file_path)?;
        let file_size = file.metadata()?.len();
        let mut reader = BufReader::new(file);

        let pb = ProgressBar::new(file_size);
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} [{bar:40.cyan/blue}] {bytes}/{total_bytes} {msg}")
                .unwrap()
                .progress_chars("█▓░-"),
        );
        pb.set_message(format!("Calculating {}", algorithm.name()));

        match algorithm {
            HashAlgorithm::Sha256 => {
                let mut hasher = Sha256::new();
                let mut buffer = [0u8; 65536];
                let mut total_read = 0u64;

                loop {
                    let count = reader.read(&mut buffer)?;
                    if count == 0 { break; }
                    hasher.update(&buffer[..count]);
                    total_read += count as u64;
                    pb.set_position(total_read);
                }

                pb.finish_with_message("✓ Done");
                Ok(format!("{:x}", hasher.finalize()))
            }
            HashAlgorithm::Blake3 => {
                // TODO: Implement with blake3 crate
                pb.finish_with_message("⚠ Not implemented");
                Err(anyhow::anyhow!("BLAKE3 not yet implemented"))
            }
            HashAlgorithm::Sha3 => {
                // TODO: Implement with sha3 crate
                pb.finish_with_message("⚠ Not implemented");
                Err(anyhow::anyhow!("SHA3 not yet implemented"))
            }
        }
    }

    pub fn verify_checksum(file_path: &str, expected: &str) -> Result<bool> {
        let actual = Self::generate_checksum(file_path)?;
        Ok(actual.eq_ignore_ascii_case(expected))
    }

    pub fn generate_checksum_file(archive_path: &str) -> Result<String> {
        crate::utils::print_info("Generating .sha256 file...");

        let checksum = Self::generate_checksum(archive_path)?;
        let archive_name = Path::new(archive_path)
            .file_name()
            .and_then(|n| n.to_str())
            .context("Invalid path")?;

        let checksum_path = format!("{}.sha256", archive_path);
        let mut checksum_file = File::create(&checksum_path)?;
        writeln!(checksum_file, "{}  {}", checksum, archive_name)?;

        crate::utils::print_success(&format!(
            "Created: {}", 
            Path::new(&checksum_path).file_name()
                .and_then(|n| n.to_str()).unwrap_or("checksum.sha256")
        ));

        Ok(checksum_path)
    }

    pub fn verify_from_checksum_file(archive_path: &str) -> Result<bool> {
        let checksum_path = format!("{}.sha256", archive_path);

        if !Path::new(&checksum_path).exists() {
            return Err(anyhow::anyhow!("Checksum file not found"));
        }

        let content = fs::read_to_string(&checksum_path)?;
        let parts: Vec<&str> = content.trim().split_whitespace().collect();

        if parts.len() < 2 {
            return Err(anyhow::anyhow!("Invalid checksum file"));
        }

        let expected = parts[0];
        let actual = Self::generate_checksum(archive_path)?;

        Ok(actual.eq_ignore_ascii_case(expected))
    }

    pub fn auto_verify(archive_path: &str) -> Result<bool> {
        let checksum_path = format!("{}.sha256", archive_path);

        if Path::new(&checksum_path).exists() {
            Self::verify_from_checksum_file(archive_path)
        } else {
            crate::utils::print_warning("No checksum file, skipping");
            Ok(true)
        }
    }
}
