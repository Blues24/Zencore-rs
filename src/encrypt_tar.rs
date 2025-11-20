use anyhow::{Context, Result};
use indicatif::{ProgressBar, ProgressStyle};
use std::fs::{self, File};
use std::io::{BufReader, BufWriter, Read, Write};

pub struct TarEncryptor {
    password: String,
}

impl TarEncryptor {
    pub fn new(password: String) -> Self {
        Self { password }
    }

    pub fn encrypt_file(&self, tar_path: &str) -> Result<String> {
        crate::utils::print_info("🔒 Encrypting TAR with age...");

        let file_size = fs::metadata(tar_path)?.len();
        let encrypted_path = format!("{}.age", tar_path);

        let pb = ProgressBar::new(file_size);
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} [{bar:40.cyan/blue}] {bytes}/{total_bytes} {msg}")
                .unwrap()
                .progress_chars("█▓░"),
        );
        pb.set_message("Encrypting...");

        let passphrase = secrecy::SecretString::from(self.password.clone());
        
        let encryptor = age::Encryptor::with_user_passphrase(passphrase);

        let input_file = File::open(tar_path)?;
        let output_file = File::create(&encrypted_path)?;

        let mut input = BufReader::new(input_file);
        
        let armor_output = age::armor::ArmoredWriter::wrap_output(
            output_file,
            age::armor::Format::AsciiArmor
            )
            .context("Failed to create armored writer")?;
        
        let mut encrypted_writer = encryptor
            .wrap_output(armor_output)
            .context("Failed to create encrypted writer")?;

        let mut buffer = [0u8; 65536];
        let mut total_read = 0u64;

        loop {
            let count = input.read(&mut buffer)?;
            if count == 0 {
                break;
            }

            encrypted_writer.write_all(&buffer[..count])?;
            total_read += count as u64;
            pb.set_position(total_read);
        }

        encrypted_writer
            .finish()
            .and_then(|w| w.finish())
            .context("Failed to finalize encryption")?;
        
        pb.finish_with_message("✓ Encrypted");

        let backup_path = format!("{}.bak", tar_path);
        fs::rename(tar_path, &backup_path)?;
        fs::rename(&encrypted_path, tar_path)?;

        crate::utils::print_success(&format!(
            "Encrypted: {:.2} MB",
            file_size as f64 / 1_048_576.0
        ));

        Ok(tar_path.to_string())
    }

    pub fn decrypt_file(&self, encrypted_path: &str) -> Result<String> {
        crate::utils::print_info("🔓 Decrypting age file...");

        let file_size = fs::metadata(encrypted_path)?.len();
        let decrypted_path = encrypted_path.trim_end_matches(".age");

        let pb = ProgressBar::new(file_size);
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} [{bar:40.cyan/blue}] {bytes}/{total_bytes} {msg}")
                .unwrap()
                .progress_chars("█▓░"),
        );
        pb.set_message("Decrypting...");

        let input_file = File::open(encrypted_path)?;
        let input = BufReader::new(input_file);

        let decryptor = age::Decryptor::new(input)?;

        let passphrase = secrecy::SecretString::from(self.password.clone());
        let identity = age::scrypt::Identity::new(passphrase);

        let mut decrypted_reader = decryptor
            .decrypt(std::iter::once(&identity as &dyn age::Identity))
            .context("Decryption failed - wrong password or corrupted file")?;

        let output_file = File::create(decrypted_path)?;
        let mut output = BufWriter::new(output_file);

        let mut buffer = [0u8; 65536];
        let mut total_read = 0u64;

        loop {
            let count = decrypted_reader.read(&mut buffer)?;
            if count == 0 {
                break;
            }

            output.write_all(&buffer[..count])?;
            total_read += count as u64;
            pb.set_position(total_read);
        }

        output.flush()?;
        pb.finish_with_message("✓ Decrypted");

        crate::utils::print_success("Decryption complete");

        Ok(decrypted_path.to_string())
    }

    pub fn is_age_encrypted(file_path: &str) -> bool {
        if file_path.ends_with(".age") {
            return true;
        }

        if let Ok(mut file) = File::open(file_path) {
            let mut header = [0u8; 32];
            if file.read_exact(&mut header).is_ok() {
                let header_str = String::from_utf8_lossy(&header);
                return header_str.contains("age-encryption.org");
            }
        }

        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_encrypt_decrypt_roundtrip() -> Result<()> {
        let temp_dir = tempfile::tempdir()?;
        let test_file = temp_dir.path().join("test.tar");

        let mut file = File::create(&test_file)?;
        file.write_all(b"Test data for encryption")?;
        drop(file);

        let encryptor = TarEncryptor::new("test_password_123".to_string());

        let encrypted = encryptor.encrypt_file(test_file.to_str().unwrap())?;
        assert!(TarEncryptor::is_age_encrypted(&encrypted));

        let decrypted = encryptor.decrypt_file(&encrypted)?;
        let content = fs::read_to_string(&decrypted)?;
        
        assert_eq!(content, "Test data for encryption");

        Ok(())
    }

    #[test]
    fn test_is_age_encrypted() {
        assert!(TarEncryptor::is_age_encrypted("backup.tar.age"));
        assert!(TarEncryptor::is_age_encrypted("file.tar.zst.age"));
        assert!(!TarEncryptor::is_age_encrypted("backup.tar.zst"));
        assert!(!TarEncryptor::is_age_encrypted("backup.tar.gz"));
    }
}
