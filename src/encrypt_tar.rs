use anyhow::{Context, Result};
use indicatif::{ProgressBar, ProgressStyle};
use age::{armor::ArmoredWriter, Encryptor as RageEncryptor};
use secrecy::SecretString;
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
        crate::utils::print_info("🔒 Encrypting TAR with age encryption...");

        let file_size = fs::metadata(tar_path)?.len();
        let encrypted_path = format!("{}.age", tar_path);

        let pb = ProgressBar::new(file_size);
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} [{bar:40.cyan/blue}] {bytes}/{total_bytes} {msg}")
                .unwrap()
                .progress_chars("█▓░"),
        );
        pb.set_message("Encrypting with rage...");

        let passphrase = self.password.as_str();
        
        let encryptor = RageEncryptor::with_user_passphrase(
            SecretString::from(passphrase.to_string())
        );

        let input_file = File::open(tar_path)?;
        let output_file = File::create(&encrypted_path)?;

        let mut input = BufReader::new(input_file);
        let armor = ArmoredWriter::wrap_output(output_file)?;
        
        let mut encrypted = encryptor
            .wrap_output(armor)
            .context("Failed to create encrypted output")?;

        let mut buffer = [0u8; 65536];
        let mut total_read = 0u64;

        loop {
            let count = input.read(&mut buffer)?;
            if count == 0 {
                break;
            }

            encrypted.write_all(&buffer[..count])?;
            total_read += count as u64;
            pb.set_position(total_read);
        }

        encrypted.finish()?;
        pb.finish_with_message("✓ Encryption complete");

        let backup_path = format!("{}.bak", tar_path);
        fs::rename(tar_path, &backup_path)?;
        fs::rename(&encrypted_path, tar_path)?;

        crate::utils::print_success(&format!(
            "Encrypted with age ({:.2} MB)",
            file_size as f64 / 1_048_576.0
        ));
        crate::utils::print_info("💡 Compatible with 'age' CLI tool for decryption");

        Ok(tar_path.to_string())
    }

    pub fn decrypt_file(&self, encrypted_path: &str) -> Result<String> {
        crate::utils::print_info("🔓 Decrypting age-encrypted file...");

        use age::Decryptor as RageDecryptor;

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
        let mut input = BufReader::new(input_file);

        let decryptor = match RageDecryptor::new(&mut input)? {
            RageDecryptor::Passphrase(d) => d,
            _ => return Err(anyhow::anyhow!("Not a passphrase-encrypted file")),
        };

        let passphrase = secrecy::SecretString::from(self.password.clone());
        let mut decrypted = decryptor
            .decrypt(&passphrase, None)?
            .context("Failed to decrypt - wrong password?")?;

        let output_file = File::create(decrypted_path)?;
        let mut output = BufWriter::new(output_file);

        let mut buffer = [0u8; 65536];
        let mut total_read = 0u64;

        loop {
            let count = decrypted.read(&mut buffer)?;
            if count == 0 {
                break;
            }

            output.write_all(&buffer[..count])?;
            total_read += count as u64;
            pb.set_position(total_read);
        }

        output.flush()?;
        pb.finish_with_message("✓ Decryption complete");

        crate::utils::print_success("File decrypted successfully");

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
    fn test_encrypt_decrypt_roundtrip() {
        let temp_dir = tempfile::tempdir().unwrap();
        let test_file = temp_dir.path().join("test.tar");

        let mut file = File::create(&test_file).unwrap();
        file.write_all(b"Test data for encryption").unwrap();
        drop(file);

        let encryptor = TarEncryptor::new("test_password".to_string());

        let encrypted = encryptor
            .encrypt_file(test_file.to_str().unwrap())
            .unwrap();
        assert!(TarEncryptor::is_age_encrypted(&encrypted));

        let decrypted = encryptor.decrypt_file(&encrypted).unwrap();

        let content = fs::read_to_string(&decrypted).unwrap();
        assert_eq!(content, "Test data for encryption");
    }

    #[test]
    fn test_is_age_encrypted() {
        assert!(TarEncryptor::is_age_encrypted("backup.tar.zst.age"));
        assert!(!TarEncryptor::is_age_encrypted("backup.tar.zst"));
    }
}
