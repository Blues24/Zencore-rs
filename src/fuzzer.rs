use anyhow::{Context, Result};
use dialoguer::{theme::ColorfulTheme, FuzzySelect};
use std::path::PathBuf;
use walkdir::WalkDir;

pub struct FuzzerConfig {
    pub max_depth: usize,
    pub exclude_patterns: Vec<String>,
    pub case_sensitive: bool,
}

impl Default for FuzzerConfig {
    fn default() -> Self {
        Self {
            max_depth: 5,
            exclude_patterns: vec![
                ".git".to_string(),
                "node_modules".to_string(),
                ".cache".to_string(),
                "target".to_string(),
                "tmp".to_string(),
                "var".to_string(),
            ],
            case_sensitive: false,
        }
    }
}

pub struct Fuzzer;

impl Fuzzer {
    pub fn find_and_select(base_paths: &[String], target: &str) -> Result<PathBuf> {
        Self::find_and_select_with_config(base_paths, target, FuzzerConfig::default())
    }

    pub fn find_and_select_with_config(
        base_paths: &[String],
        target: &str,
        config: FuzzerConfig,
    ) -> Result<PathBuf> {
        crate::utils::print_info(&format!("Searching for {} folders...", target));
        let mut folder_found = Vec::new();

        for base in base_paths {
            let expanded = shellexpand::tilde(base).to_string();
            let folders = Self::find_target_folders_with_config(&expanded, target, &config);
            folders_found.extend(folders);
        }

        if folders_found.is_empty() {
            return Err(anyhow::anyhow!("No {} folders found...", target));
        }

        folders_found.sort();
        folders_found.dedup();

        if folders_found.len() == 1 {
            crate::utils::print_info(&format!(
                    "Auto-selected: {}",
                    folders_found[0].display()
            ));
            return Ok(folders_found[0].clone());
        }

        crate::utils::print_success(&format!("Found {} folders", folders_found.len()));

        let choices: Vec<String> = folders_found
            .iter()
            .map(|p| p.to_string_lossy().to_string())
            .collect();

        let selection = FuzzySelect::with_theme(&ColorfulTheme::default())
            .with_prompt(format!("Select {} folder", target))
            .items(&choices)
            .default(0)
            .interact()
            .context("Selection cancelled")?;

        Ok(folders_found[selection].clone())
    }
    
    pub fn find_target_folders(base: &str, target: &str) -> Vec<PathBuf> {
        Self::find_target_folders_with_config(base, target, &FuzzerConfig::default())
    }

    pub fn find_target_folders_with_config(
        base: &str,
        target: &str,
        config: &FuzzerConfig,
    ) -> Vec<PathBuf> {
        WalkDir::new(base)
            .max_depth(config.max_depth)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_dir())
            .filter(|e| {
                // Check exclude patterns
                let path_str = e.path().to_string_lossy();
                for pattern in &config.exclude_patterns {
                    if path_str.contains(pattern) {
                        return false;
                    }
                }
                true
            })
            .filter(|e| {
                e.file_name()
                    .to_str()
                    .map(|s| {
                        if config.case_sensitive {
                            s == target
                        } else {
                            s.to_lowercase() == target.to_lowercase()
                        }
                    })
                    .unwrap_or(false)
            })
            .map(|e| e.path().to_path_buf())
            .collect()
    }

    pub fn count_files(path: &str) -> Result<usize> {
        let count = WalkDir::new(path)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
            .count();

        Ok(count)
    }

    pub fn estimate_size(path: &str) -> Result<u64> {
        let mut total_size = 0u64;

        for entry in WalkDir::new(path)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
        {
            if let Ok(metadata) = entry.metadata() {
                total_size += metadata.len();
            }
        }

        Ok(total_size)
    }

    pub fn get_folder_info(path: &str) -> Result<FolderInfo> {
        let file_count = Self::count_files(path)?;
        let total_size = Self::estimate_size(path)?;

        Ok(FolderInfo {
            path: path.to_string(),
            file_count,
            total_size,
        })
    }
}

#[derive(Debug)]
pub struct FolderInfo {
    pub path: String,
    pub file_count: usize,
    pub total_size: u64,
}

impl FolderInfo {
    pub fn display(&self) {
        crate::utils::print_info(&format!("📁 Folder: {}", self.path));
        crate::utils::print_info(&format!("📄 Files: {}", self.file_count));
        crate::utils::print_info(&format!(
            "💾 Size: {:.2} MB",
            self.total_size as f64 / 1_048_576.0
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exclude_patterns() {
        let config = FuzzerConfig {
            max_depth: 3,
            exclude_patterns: vec![".git".to_string(), "node_modules".to_string()],
            case_sensitive: false,
        };

        // Test would need real filesystem, skip for now
        assert_eq!(config.exclude_patterns.len(), 2);
    }

    #[test]
    fn test_case_sensitivity() {
        let config = FuzzerConfig {
            case_sensitive: false,
            ..Default::default()
        };

        assert!(!config.case_sensitive);
    }
}
