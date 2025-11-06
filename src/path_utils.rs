// ============================================
// src/path_utils.rs - Cross-Platform Path Handling
// ============================================

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

/// Cross-platform path utilities
/// Handles Windows, Linux, and macOS path differences
pub struct PathUtils;

impl PathUtils {
    /// Expand tilde (~) and environment variables in path
    /// Works on Windows (using %USERPROFILE%) and Unix (using $HOME)
    ///
    /// # Examples
    /// ```
    /// // Unix: ~/Music -> /home/user/Music
    /// // Windows: ~/Music -> C:\Users\user\Music
    /// let expanded = PathUtils::expand_path("~/Music")?;
    /// ```
    pub fn expand_path(path: &str) -> Result<PathBuf> {
        // Handle tilde expansion
        let expanded = if path.starts_with('~') {
            // Get home directory cross-platform
            let home = dirs::home_dir()
                .context("Cannot determine home directory")?;
            
            // Replace ~ with home path
            let rest = &path[1..];
            let rest = rest.strip_prefix('/').or_else(|| rest.strip_prefix('\\')).unwrap_or(rest);
            home.join(rest)
        } else {
            PathBuf::from(path)
        };
        
        // Expand environment variables
        // Windows: %USERPROFILE%, %APPDATA%, etc.
        // Unix: $HOME, $USER, etc.
        let expanded = shellexpand::full(&expanded.to_string_lossy())?;
        
        Ok(PathBuf::from(expanded.as_ref()))
    }
    
    /// Normalize path separators for current OS
    /// Windows: Convert / to \
    /// Unix: Convert \ to / (if any)
    pub fn normalize_separators(path: &Path) -> PathBuf {
        #[cfg(windows)]
        {
            // Windows: Ensure backslashes
            let s = path.to_string_lossy().replace('/', "\\");
            PathBuf::from(s)
        }
        
        #[cfg(not(windows))]
        {
            // Unix: Ensure forward slashes
            let s = path.to_string_lossy().replace('\\', "/");
            PathBuf::from(s)
        }
    }
    
    /// Get default config directory based on OS
    /// - Linux: ~/.config/zencore
    /// - macOS: ~/Library/Application Support/zencore
    /// - Windows: %APPDATA%\zencore
    pub fn config_dir() -> Result<PathBuf> {
        let proj_dirs = directories::ProjectDirs::from("com", "blues24", "zencore")
            .context("Cannot determine config directory")?;
        
        Ok(proj_dirs.config_dir().to_path_buf())
    }
    
    /// Get default data directory based on OS
    /// - Linux: ~/.local/share/zencore
    /// - macOS: ~/Library/Application Support/zencore
    /// - Windows: %APPDATA%\zencore
    pub fn data_dir() -> Result<PathBuf> {
        let proj_dirs = directories::ProjectDirs::from("com", "blues24", "zencore")
            .context("Cannot determine data directory")?;
        
        Ok(proj_dirs.data_dir().to_path_buf())
    }
    
    /// Check if path is valid and accessible
    pub fn is_valid_path(path: &Path) -> bool {
        path.exists() && (path.is_dir() || path.is_file())
    }
    
    /// Get common music folder locations based on OS
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
    
    /// Get common backup folder locations based on OS
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