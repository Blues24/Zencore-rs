use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ArchiveMetadata {
    pub name: String,
    pub created_at: String,
    
    #[serde(default)]
    pub checksum: String,
    #[serde(default)]
    pub checksums: HashMap<String, String>,
    
    pub algorithm: String,
    
    pub size_bytes: u64,
    pub file_count: usize,
    pub encrypted: bool,
    pub contents: Vec<String>,
}

impl ArchiveMetadata {
    pub fn get_checksum(&self, algorithm: &str) -> Option<&String> {
        let algo_upper = algorithm.to_uppercase();

        self.checksums.get(&algo_upper).or_else(|| {
            if algo_upper == "SHA-256" || algo_upper == "SHA256" {
                if !self.checksum.is_empty(){
                    Some(&self.checksum)
                } else {
                    None
                }
            } else {
                None 
            }
        })
    }

    pub fn add_checksum(&mut self, algorithm: &str, hash: String) {
        let algo_upper = algorithm.to_uppercase();
        self.checksums.insert(algo_upper.clone(), hash.clone());

        if algo_upper == "SHA256" || algo_upper == "SHA-256" {
            self.checksum = hash;
        }
    }

    pub fn list_checksums(&self) -> Vec<(String, String)> {
    let mut result: Vec<(String, String)> = self.checksums
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();

        if !self.checksum.is_empty() && !self.checksums.contains_key("SHA-256") {
            result.push(("SHA-256".to_string(), self.checksum.clone()));

            result.sort_by(|a, b| a.0.cmp(&b.0));
            result 
        }
    }

    #[derive(Debug, Serialize, Deserialize, Clone)]
    pub struct StateTracker {
        archives: HashMap<String, ArchiveMetadata>,
    }

    impl Default for StateTracker {
        fn default() -> Self {
            Self {
                archives: HashMap::new(),
            }
        }
    }

    impl StateTracker {
        pub fn load() -> Result<Self> {
            let state_path = Self::state_file()?;

            if !state_path.exists() {
                return Ok(Self::default());
            }

            let content = fs::read_to_string(state_path)?;
            let mut tracker: Self = serde_json::from_str(&content)?;

            tracker.migrate_old_format();

            Ok(tracker)
        }

        fn migrate_old_format(&mut self) {
            for metadata in self.archives.values_mut(){
                if !metadata.checksum.is_empty() && metadata.checksums.is_empty() {
                    metadata.checksums.insert("SHA-256".to_string(), metadata.checksum.clone());
                }
            }
        }

        pub fn save(&self) -> Result<()> {
            let state_path = Self::state_file()?;

            if let Some(parent) = state_path.parent(){
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
            let mut archives: Vec<&ArchiveMetadata> {
                archives.sort_by(|a, b| b.created_at.cmp(&a.created_at));
                archives 
            }
        }

        pub fn remove_archve(&mut self, name: &str) -> Option<ArchiveMetadata> {
            self.archives.remove(name)
        }

        pub fn archive_count(&self) -> usize {
            self.archives.len()
        }

        fn state_file() -> Result<PathBuf> {
            Ok(crate::config::Config::state_dir().join("archives.json"))
        }
    }
}
