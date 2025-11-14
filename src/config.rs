use anyhow::{Context, Result};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    #[serde(default = "default_algorithm")]
    pub default_algorithm: String,

    #[serde(default = "default_date_format")]
    pub date_format: String,

    #[serde(default = "default_music_folders")]
    pub music_folders: Vec<String>,

    #[serde(default = "default_backup_folders")]
    pub backup_folders: Vec<String>,

    #[serde(default)]
    pub default_backup_destination: String,

    #[serde(default)]
    pub encrypt_by_default: bool,

    #[serde(default = "default_cipher")]
    pub default_cipher: String,

    #[serde(default = "default_hash_algorithm")]
    pub default_hash_algorithm: String,

    #[serde(default)]
    pub num_threads: usize,

    #[serde(default)]
    pub compression_level: Option<i32>,

    #[serde(default = "default_true")]
    pub generate_checksum_file: bool,

    #[serde(default = "default_true")]
    pub verify_after_backup: bool,

    #[serde(default)]
    pub remote: Option<RemoteConfig>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RemoteConfig {
    #[serde(default)]
    pub enabled: bool,

    #[serde(default)]
    pub auto_upload: bool,

    #[serde(default)]
    pub rclone: Option<RcloneConfig>,

    #[serde(default)]
    pub database: Option<DatabaseConfig>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RcloneConfig {
    pub remote_name: String,
    pub remote_path: String,

    #[serde(default = "default_true")]
    pub verify_after_upload: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DatabaseConfig {
    pub host: String,

    #[serde(default = "default_mysql_port")]
    pub port: u16,

    pub username: String,

    #[serde(skip_serializing)]
    pub password: Option<String>,

    pub database: String,

    #[serde(default = "default_table_name")]
    pub table: String,
}

fn default_algorithm() -> String {
    "tar.zst".to_string()
}

fn default_date_format() -> String {
    "%Y%m%d_%H%M%S".to_string()
}

fn default_music_folders() -> Vec<String> {
    vec![
        "~/Music".to_string(),
        "~/Documents/Music".to_string(),
    ]
}

fn default_backup_folders() -> Vec<String> {
    vec![
        "~/Backups".to_string(),
        "~/Documents/Backups".to_string(),
    ]
}

fn default_cipher() -> String {
    "aes256".to_string()
}

fn default_hash_algorithm() -> String {
    "sha256".to_string()
}

fn default_true() -> bool {
    true
}

fn default_mysql_port() -> u16 {
    3306
}

fn default_table_name() -> String {
    "backups".to_string()
}

impl Default for Config {
    fn default() -> Self {
        Self {
            default_algorithm: default_algorithm(),
            date_format: default_date_format(),
            music_folders: default_music_folders(),
            backup_folders: default_backup_folders(),
            default_backup_destination: String::new(),
            encrypt_by_default: false,
            default_cipher: default_cipher(),
            default_hash_algorithm: default_hash_algorithm(),
            num_threads: 0,
            compression_level: None,
            generate_checksum_file: true,
            verify_after_backup: true,
            remote: None,
        }
    }
}

impl Config {
    pub fn load() -> Result<Self> {
        let config_path = Self::config_path()?;

        if !config_path.exists() {
            let config = Self::default();
            config.save()?;
            return Ok(config);
        }

        let content = fs::read_to_string(&config_path)
            .context("Failed to read config file")?;

        if config_path.extension().and_then(|s| s.to_str()) == Some("json") {
            serde_json::from_str(&content).context("Failed to parse JSON config")
        } else {
            toml::from_str(&content).context("Failed to parse TOML config")
        }
    }

    pub fn save(&self) -> Result<()> {
        let config_path = Self::config_path()?;

        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let content = toml::to_string_pretty(self)?;
        fs::write(&config_path, content)?;

        Ok(())
    }

    pub fn config_path() -> Result<PathBuf> {
        let proj_dirs = ProjectDirs::from("com", "Blues24", "zencore")
            .context("Failed to determine config dir")?;

        Ok(proj_dirs.config_dir().join("config.toml"))
    }

    pub fn state_dir() -> Result<PathBuf> {
        let proj_dirs = ProjectDirs::from("com", "Blues24", "zencore")
            .context("Failed to determine state dir")?;

        Ok(proj_dirs.data_dir().to_path_buf())
    }
}
