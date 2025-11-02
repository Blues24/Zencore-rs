// Config handler

use anyhow::{Context, Result};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    // Default compression algorithm
    #[serde(default = "default_compress_algorithm")]
    pub default_compress_algorithm: String,

    // Date format for autogen archive names
    #[serde(default = "default_date_format")]
    pub date_format: String,

    // Default source dir to be backup
    #[serde(default)]
    pub source_folders: Vec<String>,

    // Default destination dir 
    #[serde(default)]
    pub dest_folders: Vec<String>,

    // Use encryption settings by default
    #[serde(default)]
    pub encrypt_by_default: bool,
}

fn default_compress_algorithm() -> String {
    "tar.zst".to_string()
}

fn default_date_format() -> String {
    "%Y%m%d_%H%M%S".to_string()
}

impl Default for Config {
    fn default() -> Self {
        Self {
            
            default_compress_algorithm: default_compress_algorithm(),
            date_format: default_date_format(),
            source_folders: vec![
                "~/Documents".to_string(),
                "~/documents".to_string(),
            ],
            dest_folders: vec![
                "~/Backups".to_string(),
                "~/backups".to_string(),
            ],
            default_backups_dest: String::new(),
            encrypt_by_default: false,

        }
    }
}

impl Config{
    pub fn load() -> Result<Self> {
        let config_path = Self::config_path()?;

        if !config_path.exists() {
            let config = Self::default();
            config.save()?;
            return Ok(config);
        }

        let content = fs::read_to_string(&config_path)
            .context("Failed to read config file")?;

        // Support for json and TOML 
        if config_path.extension().and_then(|s| s.to_str() == Some("json")){
            serde_json::from_str(&content).context("Failed to parse config with JSON type")
        } else {
            toml::from_str(&content).context("Failed to parse config with TOML type")
        }
    }

    pub fn save(&self) -> Result<()> {
        let config_path = Self::config_path()?;

        if let Some(parent) = config_path.parent(){
            fs::create_dir_all(parent)?;
        }

        let content = toml::to_string_pretty(self)?;
        fs::write(&config_path, content)?;

        Ok(())
    }

    pub fn config_path() -> Result<PathBuf> {
        let proj_dirs = ProjectDirs::from("com", "blues24", "zencore")
            .context("Failed to determine config dir")?;

        Ok(proj_dirs.config_dir().join("config.toml"))
    } 

    pub fn state_dir() -> Result<PathBuf> {
        let proj_dirs = ProjectDirs::from("com", "blues24", "zencore")
            .context("Failed to determine state dir")?;

        Ok(proj_dirs.data_dir().to_path_buf())
    }
}