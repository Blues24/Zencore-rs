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
        pb.set_message("🔑 Deriving encryption key (fast mode)...");
        
        let salt = SaltString::generate(&mut OsRng);
        
        let params = Params::new(
            32768,    // 32 MB memory (was 1 GB by default!)
            3,        // 3 iterations (was 10 by default!)
            1,        // 1 thread
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

        pb.set_message("✓ Key derived successfully");

        Ok((key, salt))
    }

    pub fn encrypt_file(&self, file_path: &str) -> Result<String> {
        crate::utils::print_info("🔐 Starting encryption with AES-256-GCM...");

        let file_size = fs::metadata(file_path)?.len();
        
        let pb = ProgressBar::new(100);
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}% {msg}")
                .unwrap()
                .progress_chars("█▓▒░-"),
        );

        pb.set_position(0);
        pb.set_message("Reading file...");
        let plaintext = fs::read(file_path)?;
        pb.set_position(20);

        pb.set_message("Deriving encryption key...");
        let (key, salt) = self.derive_key(&pb)?;
        pb.set_position(40);

        pb.set_message("Initializing cipher...");
        let cipher = Aes256Gcm::new_from_slice(&key)?;
        let mut nonce_bytes = [0u8; 12];
        OsRng.fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);
        pb.set_position(50);

        pb.set_message("Encrypting data...");
        let ciphertext = cipher
            .encrypt(nonce, plaintext.as_ref())
            .map_err(|e| anyhow::anyhow!("Encryption failed: {}", e))?;
        pb.set_position(80);

        pb.set_message("Writing encrypted file...");
        let backup_path = format!("{}.bak", file_path);
        fs::rename(file_path, &backup_path)?;

        let mut encrypted_file = File::create(file_path)?;
        encrypted_file.write_all(salt.as_str().as_bytes())?;
        encrypted_file.write_all(&nonce_bytes)?;
        encrypted_file.write_all(&ciphertext)?;
        pb.set_position(100);

        pb.finish_with_message("✓ Encryption complete!");

        crate::utils::print_success(&format!(
            "File encrypted successfully ({:.2} MB)",
            file_size as f64 / 1_048_576.0
        ));

        Ok(file_path.to_string())
    }
}

pub struct Checker;

impl Checker {
    pub fn generate_checksum(file_path: &str) -> Result<String> {
        let file = File::open(file_path)?;
        let file_size = file.metadata()?.len();
        let mut reader = BufReader::new(file);
        let mut hasher = Sha256::new();
        let mut buffer = [0u8; 8192];

        let pb = ProgressBar::new(file_size);
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({eta})")
                .unwrap()
                .progress_chars("█▓▒░-"),
        );
        pb.set_message("Calculating SHA-256");

        let mut total_read = 0u64;

        loop {
            let count = reader.read(&mut buffer)?;
            if count == 0 {
                break;
            }
            hasher.update(&buffer[..count]);
            total_read += count as u64;
            pb.set_position(total_read);
        }

        pb.finish_with_message("✓ Checksum complete");

        Ok(format!("{:x}", hasher.finalize()))
    }

    pub fn verify_checksum(file_path: &str, expected: &str) -> Result<bool> {
        let actual = Self::generate_checksum(file_path)?;
        Ok(actual.eq_ignore_ascii_case(expected))
    }

    pub fn generate_checksum_file(archive_path: &str) -> Result<String> {
        crate::utils::print_info("Generating checksum file...");

        let checksum = Self::generate_checksum(archive_path)?;
        let archive_name = Path::new(archive_path)
            .file_name()
            .and_then(|n| n.to_str())
            .context("Invalid archive path")?;

        let checksum_path = format!("{}.sha256", archive_path);
        let mut checksum_file = File::create(&checksum_path)?;

        writeln!(checksum_file, "{}  {}", checksum, archive_name)?;

        crate::utils::print_success(&format!(
            "Checksum file created: {}",
            Path::new(&checksum_path)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("checksum.sha256")
        ));

        Ok(checksum_path)
    }

    pub fn verify_from_checksum_file(archive_path: &str) -> Result<bool> {
        let checksum_path = format!("{}.sha256", archive_path);

        if !Path::new(&checksum_path).exists() {
            return Err(anyhow::anyhow!("Checksum file not found: {}", checksum_path));
        }

        crate::utils::print_info("Reading checksum from file...");

        let content = fs::read_to_string(&checksum_path)?;
        let parts: Vec<&str> = content.trim().split_whitespace().collect();

        if parts.len() < 2 {
            return Err(anyhow::anyhow!("Invalid checksum file format"));
        }

        let expected_checksum = parts[0];

        crate::utils::print_info("Verifying archive integrity...");
        let actual_checksum = Self::generate_checksum(archive_path)?;

        Ok(actual_checksum.eq_ignore_ascii_case(expected_checksum))
    }

    pub fn auto_verify(archive_path: &str) -> Result<bool> {
        let checksum_path = format!("{}.sha256", archive_path);

        if Path::new(&checksum_path).exists() {
            Self::verify_from_checksum_file(archive_path)
        } else {
            crate::utils::print_warning("No checksum file found, skipping verification");
            Ok(true)
        }
    }
}
