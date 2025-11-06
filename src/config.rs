// ============================================
// src/config.rs - Configuration Management
// ============================================

use anyhow::{Context, Result};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    /// Default compression algorithm
    #[serde(default = "default_algorithm")]
    pub default_algorithm: String,
    
    /// Date format for auto-generated archive names
    /// Uses chrono format: %Y=year, %m=month, %d=day, %H=hour, %M=minute, %S=second
    #[serde(default = "default_date_format")]
    pub date_format: String,
    
    /// Default backup destination path (empty = always prompt)
    #[serde(default)]
    pub default_backup_destination: String,
    
    /// Folders to search for music (OS-specific defaults)
    #[serde(default = "default_music_folders")]
    pub default_music_folders: Vec<String>,
    
    /// Folders to search for backups (OS-specific defaults)
    #[serde(default = "default_backup_folders")]
    pub default_backup_folders: Vec<String>,
    
    /// Use encryption by default?
    #[serde(default)]
    pub encrypt_by_default: bool,
    
    /// Default encryption cipher (aes256, chacha20)
    #[serde(default = "default_cipher")]
    pub default_cipher: String,
    
    /// Default hash algorithm for checksums (sha256, sha3, blake3)
    #[serde(default = "default_hash_algorithm")]
    pub default_hash_algorithm: String,
    
    /// Number of threads for parallel operations (0 = auto-detect)
    #[serde(default)]
    pub num_threads: usize,
    
    /// Compression level (algorithm-specific, None = default)
    #[serde(default)]
    pub compression_level: Option<i32>,
}

fn default_algorithm() -> String {
    "tar.zst".to_string()
}

fn default_date_format() -> String {
    "%Y%m%d_%H%M%S".to_string()
}

impl Default for Config {
    fn default() -> Self {
        Self {
            default_algorithm: default_algorithm(),
            date_format: default_date_format(),
            default_music_folders: vec![
                "~/Music".to_string(),
                "~/music".to_string(),
            ],
            default_backup_folders: vec![
                "~/Backups".to_string(),
                "~/backups".to_string(),
            ],
            default_backup_destination: String::new(),
            default_cipher: String::new(),
            default_hash_algorithm: String::new(),
            compression_level: 4,
            num_threads: 2,
            encrypt_by_default: false,
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
        
        // Support both JSON and TOML
        if config_path.extension().and_then(|s| s.to_str()) == Some("json") {
            serde_json::from_str(&content).context("Failed to parse config JSON")
        } else {
            toml::from_str(&content).context("Failed to parse config TOML")
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
        let proj_dirs = ProjectDirs::from("com", "blues24", "zencore")
            .context("Failed to determine config directory")?;
        
        Ok(proj_dirs.config_dir().join("config.toml"))
    }
    
    pub fn state_dir() -> Result<PathBuf> {
        let proj_dirs = ProjectDirs::from("com", "blues24", "zencore")
            .context("Failed to determine state directory")?;
        
        Ok(proj_dirs.data_dir().to_path_buf())
    }
}
