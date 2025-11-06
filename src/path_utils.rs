use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

pub struct PathUtils;

impl PathUtils {
    pub fn expand_path(path: &str) -> Result<PathBuf> {
        let expanded = if path.starts_with('~') {
            let home = dirs::home_dir()
                .context("Cannot determine home directory")?;

            let rest = &path[1..];
            let rest = rest.strip_prefix('/').or_else(|| rest.strip_prefix('\\')).unwrap_or(rest);
            home.join(rest)
        } else {
            PathBuf::from(path)
        };
        let binding = &expanded.to_string_lossy();
        let expanded = shellexpand::full(binding)?;

        Ok(PathBuf::from(expanded.as_ref()))
    }

    pub fn normalize_separators(path: &Path) -> PathBuf {
        #[cfg(windows)]
        {
            let separator = path.to_string_lossy().replace('/', "\\");
            PathBuf::from(separator)
        }
        #[cfg(not(windows))]
        {
            let separator = path.to_string_lossy().replace('\\', "/");
            PathBuf::from(separator)
        }
    }

    pub fn config_dir() -> Result<PathBuf> {
        let proj_dirs = directories::ProjectDirs::from("com", "Blues24", "zencore")
            .context("Failed to determine config dir")?;

        Ok(proj_dirs.config_dir().to_path_buf())
    }

    pub fn data_dir() -> Result<PathBuf> {
        let proj_dirs = directories::ProjectDirs::from("com", "Blues24", "zencore")
            .context("Failed to determine data dir")?;

        Ok(proj_dirs.data_dir().to_path_buf())
    }

    pub fn is_valid_path(path: &Path) -> bool {
        path.exists() && (path.is_dir() || path.is_file())
    }

    pub fn default_music_folders() -> Vec<String> {
        #[cfg(windows)]
        {
            vec![
                "%USERPROFILE%\\Music".to_string(),
                "%USERPROFILE%\\Documents\\Music".to_string(),
                "C:\\Music".to_string(),
                "D:\\Music".to_string(),
            ]
        }

        #[cfg(target_os = "macos")]
        {
            vec![
                "~/Music".to_string(),
                "~/Documents/Music".to_string(),
                "/Volumes/Music".to_string(),
            ]
        }

        #[cfg(target_os = "linux")]
        {
            vec![
                "~/Music".to_string(),
                "~/music".to_string(),
                "~/Documents/Music".to_string(),
                "/mnt/Music".to_string(),
            ]
        }

        #[cfg(not(any(windows, target_os = "macos", target_os = "linux")))]
        {
            vec!["~/Music".to_string()]
        }
    }

    pub fn default_backup_folders() -> Vec<String> {
        #[cfg(windows)]
        {
            vec![
                "%USERPROFILE%\\Backups".to_string(),
                "%USERPROFILE%\\Documents\\Backups".to_string(),
                "C:\\Backups".to_string(),
                "D:\\Backups".to_string(),
            ]
        }

        #[cfg(target_os = "macos")]
        {
            vec![
                "~/Backups".to_string(),
                "~/Documents/Backups".to_string(),
                "/Volumes/Backups".to_string(),
            ]
        }

        #[cfg(target_os = "linux")]
        {
            vec![
                "~/Backups".to_string(),
                "~/backups".to_string(),
                "~/Documents/Backups".to_string(),
                "/mnt/backup".to_string(),
            ]
        }

        #[cfg(not(any(windows, target_os = "macos", target_os = "linux")))]
        {
            vec!["~/Backups".to_string()]
        }
    }
}