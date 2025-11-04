// ============================================
// src/utils.rs - Utility Functions & Console Output
// ============================================

use colored::*;
use pyfiglet::FIGfont;
use std::io::{self, Write};
use std::time::Duration;

/// Console output utilities with colored text and formatting
/// 
/// Provides consistent, beautiful output for CLI interactions
/// Uses colorama-style color coding for clear status messages
pub struct ConsoleTemplate;

impl ConsoleTemplate {
    /// Print informational message (cyan)
    /// 
    /// Used for general information, progress updates, and non-critical messages
    /// 
    /// # Arguments
    /// * `message` - Message to display
    /// 
    /// # Examples
    /// ```
    /// ConsoleTemplate::print_info("Scanning directory...");
    /// // Output: [•] Scanning directory...
    /// ```
    pub fn print_info(message: &str) {
        println!("{} {}", "[•]".cyan(), message);
    }

    /// Print success message (green)
    /// 
    /// Used for successful operations, confirmations, and positive outcomes
    /// 
    /// # Arguments
    /// * `message` - Success message to display
    /// 
    /// # Examples
    /// ```
    /// ConsoleTemplate::print_success("Backup completed!");
    /// // Output: [✓] Backup completed!
    /// ```
    pub fn print_success(message: &str) {
        println!("{} {}", "[✓]".green(), message);
    }

    /// Print warning message (yellow)
    /// 
    /// Used for warnings, non-critical issues, and important notices
    /// 
    /// # Arguments
    /// * `message` - Warning message to display
    /// 
    /// # Examples
    /// ```
    /// ConsoleTemplate::print_warning("Path not found, using fallback");
    /// // Output: [!] Path not found, using fallback
    /// ```
    pub fn print_warning(message: &str) {
        println!("{} {}", "[!]".yellow(), message);
    }

    /// Print error message (red)
    /// 
    /// Used for errors, failures, and critical issues
    /// Prints to stderr instead of stdout
    /// 
    /// # Arguments
    /// * `message` - Error message to display
    /// 
    /// # Examples
    /// ```
    /// ConsoleTemplate::print_error("Failed to read file");
    /// // Output: [✗] Failed to read file
    /// ```
    pub fn print_error(message: &str) {
        eprintln!("{} {}", "[✗]".red(), message);
    }

    /// Display ASCII art banner
    /// 
    /// Shows "Blues Zencore" in stylized ASCII art with version info
    /// Uses pyfiglet for text rendering
    /// 
    /// # Examples
    /// ```
    /// ConsoleTemplate::show_banner();
    /// // Displays:
    /// //     ____  __                   ________                              
    /// //    / __ )/ /_  _____  _____   /_  ____ ___  ____  _________  ________ 
    /// //   / __  / / / / / _ \/ ___/    / / / _ \/ __ \/ ___/ __ \/ ___/ _ \
    /// //  / /_/ / / /_/ /  __(__  )    / / /  __/ / / / /__/ /_/ / /  /  __/
    /// // /_____/_/\__,_/\___/____/    /_/  \___/_/ /_/\___/\____/_/   \___/ 
    /// // v1.0.0 - Rust Edition
    /// ```
    pub fn show_banner() {
        // Try to load FIGfont, fallback to simple text if fails
        let banner = match FIGfont::standard() {
            Ok(font) => {
                match font.convert("Blues Zencore") {
                    Some(fig) => fig.to_string(),
                    None => String::from("Blues Zencore"),
                }
            }
            Err(_) => String::from("Blues Zencore"),
        };
        
        println!("{}", banner.bright_magenta());
        println!("{}", "v1.0.0 - Rust Edition".bright_cyan());
        println!();
    }

    /// Display thank you message
    /// 
    /// Shows appreciation message at the end of successful operations
    /// 
    /// # Examples
    /// ```
    /// ConsoleTemplate::thank_you();
    /// // Output: Terimakasih telah memakai program ini!
    /// ```
    pub fn thank_you() {
        println!("{}", "Terimakasih telah memakai program ini!".green());
    }

    /// Create loading bar iterator for progress display
    /// 
    /// Wraps an iterable with a progress indicator for visual feedback
    /// Uses indicatif crate for beautiful progress bars
    /// 
    /// # Arguments
    /// * `task_name` - Name of the task being performed
    /// * `iterable` - Iterator to wrap with progress
    /// 
    /// # Returns
    /// Iterator that yields items while updating progress
    /// 
    /// # Examples
    /// ```
    /// let files = vec!["file1.txt", "file2.txt", "file3.txt"];
    /// for file in ConsoleTemplate::loading_bar("Processing", &files) {
    ///     // Process file
    ///     process_file(file);
    /// }
    /// // Output: Processing [████████████] 3/3
    /// ```
    /// 
    /// # Note
    /// This is a simplified version. For production use, consider using
    /// indicatif's ProgressBar directly for more control.
    pub fn loading_bar<'a, I>(task_name: &'a str, iterable: I) -> LoadingBarIterator<'a, I::IntoIter>
    where
        I: IntoIterator,
        I::IntoIter: ExactSizeIterator,
    {
        LoadingBarIterator::new(task_name, iterable.into_iter())
    }

    /// Print separator line
    /// 
    /// Displays a horizontal line for visual separation
    /// Useful for dividing sections in output
    /// 
    /// # Arguments
    /// * `length` - Length of the separator line (default: 80)
    /// 
    /// # Examples
    /// ```
    /// ConsoleTemplate::print_separator(50);
    /// // Output: ──────────────────────────────────────────────────
    /// ```
    pub fn print_separator(length: usize) {
        println!("{}", "─".repeat(length).bright_black());
    }

    /// Print header with surrounding decoration
    /// 
    /// Displays a styled header for section titles
    /// 
    /// # Arguments
    /// * `title` - Header title to display
    /// 
    /// # Examples
    /// ```
    /// ConsoleTemplate::print_header("Configuration Settings");
    /// // Output:
    /// // ════════════════════════════════════
    /// //    Configuration Settings
    /// // ════════════════════════════════════
    /// ```
    pub fn print_header(title: &str) {
        let width = title.len() + 4;
        println!("\n{}", "═".repeat(width).bright_blue());
        println!("  {}", title.bold().bright_white());
        println!("{}", "═".repeat(width).bright_blue());
    }

    /// Prompt user for yes/no confirmation
    /// 
    /// Displays a question and waits for y/n input
    /// 
    /// # Arguments
    /// * `question` - Question to ask the user
    /// 
    /// # Returns
    /// `true` if user confirms (y/Y), `false` otherwise
    /// 
    /// # Examples
    /// ```
    /// if ConsoleTemplate::confirm("Proceed with backup?") {
    ///     // User said yes
    ///     perform_backup();
    /// }
    /// ```
    pub fn confirm(question: &str) -> bool {
        print!("{} {} (y/n): ", "[?]".bright_yellow(), question);
        io::stdout().flush().unwrap();
        
        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();
        
        matches!(input.trim().to_lowercase().as_str(), "y" | "yes")
    }

    /// Display a spinning loader for background tasks
    /// 
    /// Shows an animated spinner to indicate ongoing work
    /// 
    /// # Arguments
    /// * `message` - Message to display with spinner
    /// 
    /// # Examples
    /// ```
    /// ConsoleTemplate::spinner("Loading...");
    /// // Output: ⠋ Loading...
    /// ```
    /// 
    /// # Note
    /// This is a placeholder. For actual spinning animation,
    /// use indicatif::ProgressBar with .enable_steady_tick()
    pub fn spinner(message: &str) {
        print!("\r{} {}...", "⠋".cyan(), message);
        io::stdout().flush().unwrap();
    }

    /// Clear the current line in terminal
    /// 
    /// Useful for updating status messages in place
    /// 
    /// # Examples
    /// ```
    /// ConsoleTemplate::print_info("Processing file 1/100");
    /// // Later...
    /// ConsoleTemplate::clear_line();
    /// ConsoleTemplate::print_info("Processing file 2/100");
    /// ```
    pub fn clear_line() {
        print!("\r{}\r", " ".repeat(80));
        io::stdout().flush().unwrap();
    }

    /// Print key-value pair with formatting
    /// 
    /// Displays configuration or info in key: value format
    /// 
    /// # Arguments
    /// * `key` - Key name
    /// * `value` - Value to display
    /// 
    /// # Examples
    /// ```
    /// ConsoleTemplate::print_kv("Algorithm", "tar.zst");
    /// // Output: Algorithm:  tar.zst
    /// ```
    pub fn print_kv(key: &str, value: &str) {
        println!("  {}: {}", key.bright_yellow(), value.white());
    }

    /// Print list item with bullet point
    /// 
    /// Displays an item in a bulleted list
    /// 
    /// # Arguments
    /// * `item` - Item text to display
    /// 
    /// # Examples
    /// ```
    /// ConsoleTemplate::print_item("Scanning files");
    /// // Output:   • Scanning files
    /// ```
    pub fn print_item(item: &str) {
        println!("  {} {}", "•".bright_blue(), item);
    }

    /// Print step in a multi-step process
    /// 
    /// Displays numbered steps with highlighting
    /// 
    /// # Arguments
    /// * `step` - Step number
    /// * `total` - Total number of steps
    /// * `description` - Step description
    /// 
    /// # Examples
    /// ```
    /// ConsoleTemplate::print_step(1, 3, "Scanning directories");
    /// // Output: [1/3] Scanning directories
    /// ```
    pub fn print_step(step: usize, total: usize, description: &str) {
        println!(
            "{} {}",
            format!("[{}/{}]", step, total).bright_cyan().bold(),
            description
        );
    }
}

/// Iterator wrapper for loading bar functionality
/// 
/// Provides progress tracking for iterables
pub struct LoadingBarIterator<'a, I> {
    task_name: &'a str,
    inner: I,
    total: usize,
    current: usize,
}

impl<'a, I> LoadingBarIterator<'a, I>
where
    I: ExactSizeIterator,
{
    /// Create new loading bar iterator
    fn new(task_name: &'a str, inner: I) -> Self {
        let total = inner.len();
        Self {
            task_name,
            inner,
            total,
            current: 0,
        }
    }

    /// Update progress display
    fn update_progress(&self) {
        let percentage = (self.current as f64 / self.total as f64 * 100.0) as usize;
        let bar_length = 40;
        let filled = (bar_length * self.current) / self.total;
        let bar = "█".repeat(filled) + &"░".repeat(bar_length - filled);
        
        print!(
            "\r{} {} [{}] {}/{} ({}%)",
            "⠋".cyan(),
            self.task_name,
            bar,
            self.current,
            self.total,
            percentage
        );
        io::stdout().flush().unwrap();
        
        if self.current == self.total {
            println!(); // New line after completion
        }
    }
}

impl<'a, I> Iterator for LoadingBarIterator<'a, I>
where
    I: Iterator + ExactSizeIterator,
{
    type Item = I::Item;

    fn next(&mut self) -> Option<Self::Item> {
        self.update_progress();
        
        let item = self.inner.next();
        if item.is_some() {
            self.current += 1;
        }
        
        item
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl<'a, I> ExactSizeIterator for LoadingBarIterator<'a, I>
where
    I: ExactSizeIterator,
{
    fn len(&self) -> usize {
        self.inner.len()
    }
}

/// Format bytes into human-readable size
/// 
/// Converts byte count to KB, MB, GB, etc.
/// 
/// # Arguments
/// * `bytes` - Number of bytes
/// 
/// # Returns
/// Formatted string (e.g., "1.5 GB")
/// 
/// # Examples
/// ```
/// let size = format_bytes(1_500_000_000);
/// assert_eq!(size, "1.40 GB");
/// ```
pub fn format_bytes(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB", "PB"];
    
    if bytes == 0 {
        return "0 B".to_string();
    }
    
    let base: f64 = 1024.0;
    let exp = (bytes as f64).log(base).floor() as usize;
    let exp = exp.min(UNITS.len() - 1);
    
    let size = bytes as f64 / base.powi(exp as i32);
    format!("{:.2} {}", size, UNITS[exp])
}

/// Format duration into human-readable time
/// 
/// Converts Duration to readable format (e.g., "2m 30s")
/// 
/// # Arguments
/// * `duration` - Duration to format
/// 
/// # Returns
/// Formatted string
/// 
/// # Examples
/// ```
/// let duration = Duration::from_secs(150);
/// let formatted = format_duration(duration);
/// assert_eq!(formatted, "2m 30s");
/// ```
pub fn format_duration(duration: Duration) -> String {
    let secs = duration.as_secs();
    
    if secs < 60 {
        format!("{}s", secs)
    } else if secs < 3600 {
        format!("{}m {}s", secs / 60, secs % 60)
    } else {
        format!("{}h {}m", secs / 3600, (secs % 3600) / 60)
    }
}

/// Truncate string to specified length with ellipsis
/// 
/// Shortens long strings for display purposes
/// 
/// # Arguments
/// * `text` - Text to truncate
/// * `max_length` - Maximum length (including ellipsis)
/// 
/// # Returns
/// Truncated string
/// 
/// # Examples
/// ```
/// let long_text = "This is a very long filename.txt";
/// let short = truncate_string(long_text, 20);
/// assert_eq!(short, "This is a very lo...");
/// ```
pub fn truncate_string(text: &str, max_length: usize) -> String {
    if text.len() <= max_length {
        text.to_string()
    } else {
        format!("{}...", &text[..max_length.saturating_sub(3)])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_bytes() {
        assert_eq!(format_bytes(0), "0 B");
        assert_eq!(format_bytes(1024), "1.00 KB");
        assert_eq!(format_bytes(1_048_576), "1.00 MB");
        assert_eq!(format_bytes(1_073_741_824), "1.00 GB");
    }

    #[test]
    fn test_format_duration() {
        assert_eq!(format_duration(Duration::from_secs(30)), "30s");
        assert_eq!(format_duration(Duration::from_secs(90)), "1m 30s");
        assert_eq!(format_duration(Duration::from_secs(3661)), "1h 1m");
    }

    #[test]
    fn test_truncate_string() {
        assert_eq!(truncate_string("short", 10), "short");
        assert_eq!(truncate_string("this is a long string", 10), "this is...");
        assert_eq!(truncate_string("exact", 5), "exact");
    }

    #[test]
    fn test_loading_bar_iterator() {
        let items = vec![1, 2, 3, 4, 5];
        let mut iter = ConsoleTemplate::loading_bar("Test", items);
        
        assert_eq!(iter.len(), 5);
        assert_eq!(iter.next(), Some(1));
        assert_eq!(iter.len(), 4);
    }
}