// ============================================
// src/state.rs - Archive State Tracking
// ============================================

use anyhow::Result;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ArchiveMetadata {
    pub name: String,
    pub created_at: String,
    pub checksum: String,
    pub algorithm: String,
    pub size_bytes: u64,
    pub file_count: usize,
    pub encrypted: bool,
    /// List of files in archive (relative paths)
    pub contents: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct StateTracker {
    archives: HashMap<String, ArchiveMetadata>,
}

impl StateTracker {
    pub fn load() -> Result<Self> {
        let state_path = Self::state_file()?;
        
        if !state_path.exists() {
            return Ok(Self::default());
        }
        
        let content = fs::read_to_string(state_path)?;
        Ok(serde_json::from_str(&content)?)
    }
    
    pub fn save(&self) -> Result<()> {
        let state_path = Self::state_file()?;
        
        if let Some(parent) = state_path.parent() {
            fs::create_dir_all(parent)?;
        }
        
        let content = serde_json::to_string_pretty(self)?;
        fs::write(state_path, content)?;
        
        Ok(())
    }
    
    pub fn add_archive(&mut self, metadata: ArchiveMetadata) {
        self.archives.insert(metadata.name.clone(), metadata);
    }
    
    pub fn get_archive(&self, name: &str) -> Option<&ArchiveMetadata> {
        self.archives.get(name)
    }
    
    pub fn list_archives(&self) -> Vec<&ArchiveMetadata> {
        self.archives.values().collect()
    }
    
    fn state_file() -> Result<PathBuf> {
        Ok(Config::state_dir()?.join("archives.json"))
    }
}
