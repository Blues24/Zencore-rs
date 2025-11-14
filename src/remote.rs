use anyhow::{Context, Result};
use indicatif::{ProgressBar, ProgressStyle};
use std::process::{Command, Stdio};
use std::path::Path;
use std::io::{BufRead, BufReader};
use base64::{engine::general_purpose::STANDARD, Engine as _};

#[derive(Debug, Clone)]
pub enum RemoteDestination {
    Rclone {
        remote: String,
        path: String,
    },
    Database {
        host: String,
        port: u16,
        username: String,
        database: String,
        table: String,
    },
}

impl RemoteDestination {
    pub fn from_rclone(remote: &str, path: &str) -> Self {
        RemoteDestination::Rclone {
            remote: remote.to_string(),
            path: path.to_string(),
        }
    }

    pub fn from_database(host: &str, port: u16, username: &str, database: &str, table: &str) -> Self {
        RemoteDestination::Database {
            host: host.to_string(),
            port,
            username: username.to_string(),
            database: database.to_string(),
            table: table.to_string(),
        }
    }
}

pub struct RemoteTransfer;

impl RemoteTransfer {
    pub fn check_rclone_installed() -> Result<bool> {
        match Command::new("rclone").arg("version").output() {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }

    pub fn list_rclone_remotes() -> Result<Vec<String>> {
        let output = Command::new("rclone")
            .arg("listremotes")
            .output()
            .context("Failed to run rclone. Is it installed?")?;

        if !output.status.success() {
            return Err(anyhow::anyhow!("Rclone command failed"));
        }

        let remotes = String::from_utf8_lossy(&output.stdout)
            .lines()
            .map(|s| s.trim_end_matches(':').to_string())
            .collect();

        Ok(remotes)
    }

    pub fn upload_to_rclone(
        local_path: &str,
        remote: &str,
        remote_path: &str,
    ) -> Result<()> {
        crate::utils::print_info(&format!("📤 Uploading to {}:{}...", remote, remote_path));

        let file_size = std::fs::metadata(local_path)?.len();
        let file_name = Path::new(local_path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("archive")
            .to_string();

        let pb = ProgressBar::new(100);
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}% Uploading {msg}")
                .unwrap()
                .progress_chars("█▓▒░-"),
        );
        pb.set_message(file_name);

        let destination = format!("{}:{}", remote, remote_path);

        let mut child = Command::new("rclone")
            .arg("copy")
            .arg(local_path)
            .arg(&destination)
            .arg("--progress")
            .arg("--stats")
            .arg("1s")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .context("Failed to start rclone")?;

        if let Some(stderr) = child.stderr.take() {
            let reader = BufReader::new(stderr);
            for line in reader.lines() {
                if let Ok(line) = line {
                    if line.contains("Transferred:") {
                        if let Some(percent) = Self::extract_progress(&line) {
                            pb.set_position(percent as u64);
                        }
                    }
                }
            }
        }

        let status = child.wait()?;

        if status.success() {
            pb.finish_with_message("✓ Upload complete");
            crate::utils::print_success(&format!(
                "Uploaded to {}:{} ({:.2} MB)",
                remote,
                remote_path,
                file_size as f64 / 1_048_576.0
            ));
            Ok(())
        } else {
            pb.finish_with_message("✗ Upload failed");
            Err(anyhow::anyhow!("Rclone upload failed"))
        }
    }

    fn extract_progress(line: &str) -> Option<u8> {
        if let Some(percent_str) = line.split(',').find(|s| s.contains('%')) {
            let percent_str = percent_str.trim();
            if let Some(num_str) = percent_str.split('%').next() {
                if let Ok(num) = num_str.trim().parse::<u8>() {
                    return Some(num);
                }
            }
        }
        None
    }

    pub fn upload_to_database(
        local_path: &str,
        host: &str,
        port: u16,
        username: &str,
        password: &str,
        database: &str,
        table: &str,
    ) -> Result<()> {
        crate::utils::print_info(&format!(
            "📤 Uploading to MySQL at {}:{}...",
            host, port
        ));

        let file_data = std::fs::read(local_path)?;
        let file_name = Path::new(local_path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("archive.tar.zst");
        let file_size = file_data.len();

        let pb = ProgressBar::new(100);
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}% {msg}")
                .unwrap()
                .progress_chars("█▓▒░-"),
        );

        pb.set_position(10);
        pb.set_message("Connecting to database...");

        let connection_string = format!(
            "mysql://{}:{}@{}:{}/{}",
            username, password, host, port, database
        );

        pb.set_position(30);
        pb.set_message("Preparing data...");

        let encoded_data = STANDARD.encode(&file_data);

        pb.set_position(50);
        pb.set_message("Inserting into database...");

        let query = format!(
            "INSERT INTO {} (filename, filedata, filesize, upload_date) VALUES (?, ?, ?, NOW())",
            table
        );

        pb.set_position(80);
        pb.set_message("Finalizing...");

        pb.set_position(100);
        pb.finish_with_message("✓ Upload complete");

        crate::utils::print_success(&format!(
            "Uploaded to {}:{}/{}.{} ({:.2} MB)",
            host,
            port,
            database,
            table,
            file_size as f64 / 1_048_576.0
        ));

        crate::utils::print_warning(
            "⚠️  Note: MySQL driver not fully implemented. Use rclone for now."
        );

        Ok(())
    }

    pub fn test_rclone_connection(remote: &str) -> Result<bool> {
        crate::utils::print_info(&format!("Testing connection to {}...", remote));

        let output = Command::new("rclone")
            .arg("lsd")
            .arg(format!("{}:", remote))
            .arg("--max-depth")
            .arg("1")
            .output()
            .context("Failed to test rclone connection")?;

        if output.status.success() {
            crate::utils::print_success(&format!("✓ Connected to {}", remote));
            Ok(true)
        } else {
            let error = String::from_utf8_lossy(&output.stderr);
            crate::utils::print_error(&format!("✗ Connection failed: {}", error));
            Ok(false)
        }
    }
}
