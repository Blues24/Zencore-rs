use anyhow::{Context, Result};
use dialoguer::{theme::ColorfulTheme, FuzzySelect};
use std::path::PathBuf;
use walkdir::WalkDir;

pub struct Fuzzer;

impl Fuzzer {
    pub fn find_and_select(base_paths: &[String], target: &str) -> Result<PathBuf> {
        crate::utils::print_info(&format!("🔍 Searching for {} folders...", target));

        let mut folders_found = Vec::new();

        for base in base_paths {
            let expanded = shellexpand::tilde(base).to_string();
            let folders = Self::find_target_folders(&expanded, target);
            folders_found.extend(folders);
        }

        if folders_found.is_empty() {
            return Err(anyhow::anyhow!("No {} folders found", target));
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
        WalkDir::new(base)
            .max_depth(5)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_dir())
            .filter(|e| {
                e.file_name()
                    .to_str()
                    .map(|s| s.to_lowercase() == target.to_lowercase())
                    .unwrap_or(false)
            })
            .map(|e| e.path().to_path_buf())
            .collect()
    }
}