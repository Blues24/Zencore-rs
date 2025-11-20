use colored::*;
use std::time::Duration;

pub fn print_info(message: &str) {
    println!("{} {}", "[•]".cyan(), message);
}

pub fn print_success(message: &str) {
    println!("{} {}", "[✓]".green(), message);
}

pub fn print_warning(message: &str) {
    println!("{} {}", "[!]".yellow(), message);
}

pub fn print_error(message: &str) {
    eprintln!("{} {}", "[✗]".red(), message);
}

pub fn show_banner() {
    let banner = r#"
██████╗  ██╗     ██╗   ██╗███████╗███████╗
 ██╔══██╗██║     ██║   ██║██╔════╝██╔════╝
 ██████╔╝██║     ██║   ██║█████╗  ███████╗
 ██╔══██╗██║     ██║   ██║██╔══╝  ╚════██║
 ██████╔╝███████╗╚██████╔╝███████╗███████║
 ╚═════╝ ╚══════╝ ╚═════╝ ╚══════╝╚══════╝
 
 ███████╗███████╗███╗   ██╗ ██████╗ ██████╗ ██████╗ ███████╗
 ╚══███╔╝██╔════╝████╗  ██║██╔════╝██╔═══██╗██╔══██╗██╔════╝
   ███╔╝ █████╗  ██╔██╗ ██║██║     ██║   ██║██████╔╝█████╗  
  ███╔╝  ██╔══╝  ██║╚██╗██║██║     ██║   ██║██╔══██╗██╔══╝  
 ███████╗███████╗██║ ╚████║╚██████╗╚██████╔╝██║  ██║███████╗
 ╚══════╝╚══════╝╚═╝  ╚═══╝ ╚═════╝ ╚═════╝ ╚═╝  ╚═╝╚══════╝

    🎵 Minimalist Music Backup Tool - Fast, Secure, Beautiful 🎵
              v1.3.1 - Rust Edition - Codename Oswin Oswald                                                                                                                                                                                                                                        
    "#;

    println!("{}", banner.bright_blue());
    println!("Ready to serve!");
}

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

pub fn format_speed(bytes: u64, duration: Duration) -> String {
    if duration.as_secs() == 0 {
        return "N/A".to_string();
    }

    let bytes_per_sec = bytes as f64 / duration.as_secs_f64();
    format!("{}/s", format_bytes(bytes_per_sec as u64))
}

pub fn truncate_string(text: &str, max_length: usize) -> String {
    if text.len() <= max_length {
        text.to_string()
    } else {
        format!("{}...", &text[..max_length.saturating_sub(3)])
    }
}

pub fn format_percentage(current: u64, total: u64) -> String {
    if total == 0 {
        return "0%".to_string();
    }

    let percentage = (current as f64 / total as f64 * 100.0) as u8;
    format!("{}%", percentage)
}

pub fn print_table_header(columns: &[&str], widths: &[usize]) {
    let mut header = String::new();
    for (i, col) in columns.iter().enumerate() {
        header.push_str(&format!("{:<width$}", col, width = widths[i]));
        if i < columns.len() - 1 {
            header.push_str(" ");
        }
    }
    println!("{}", header.bold());
    println!("{}", "─".repeat(widths.iter().sum::<usize>() + widths.len() - 1));
}

pub fn print_separator(length: usize) {
    println!("{}", "─".repeat(length).bright_black());
}

pub fn print_header(title: &str) {
    let width = title.len() + 4;
    println!("\n{}", "═".repeat(width).bright_blue());
    println!("  {}", title.bold().bright_white());
    println!("{}", "═".repeat(width).bright_blue());
}

pub fn confirm_action(question: &str, default: bool) -> bool {
    use dialoguer::{theme::ColorfulTheme, Confirm};

    Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt(question)
        .default(default)
        .interact()
        .unwrap_or(default)
}

pub fn print_progress(current: u64, total: u64, prefix: &str) {
    let percentage = if total > 0 {
        (current as f64 / total as f64 * 100.0) as u8
    } else {
        0
    };

    let bar_length = 40;
    let filled = (bar_length * current as usize) / total.max(1) as usize;
    let bar = "█".repeat(filled) + &"░".repeat(bar_length - filled);

    print!(
        "\r{} {} [{}] {}/{} ({}%)",
        "⠋".cyan(),
        prefix,
        bar,
        current,
        total,
        percentage
    );

    if current >= total {
        println!();
    }
}

pub fn format_number(num: usize) -> String {
    let num_str = num.to_string();
    let chars: Vec<char> = num_str.chars().collect();
    let mut result = String::new();

    for (i, ch) in chars.iter().enumerate() {
        if i > 0 && (chars.len() - i) % 3 == 0 {
            result.push(',');
        }
        result.push(*ch);
    }

    result
}

pub fn format_compression_ratio(original: u64, compressed: u64) -> String {
    if original == 0 {
        return "N/A".to_string();
    }

    let ratio = (compressed as f64 / original as f64) * 100.0;
    format!("{:.1}%", ratio)
}

pub fn print_summary(items: &[(&str, String)]) {
    print_header("Summary");
    for (key, value) in items {
        println!("  {}: {}", key.bright_yellow(), value.white());
    }
    println!();
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
    fn test_format_percentage() {
        assert_eq!(format_percentage(50, 100), "50%");
        assert_eq!(format_percentage(1, 3), "33%");
        assert_eq!(format_percentage(0, 0), "0%");
    }

    #[test]
    fn test_format_number() {
        assert_eq!(format_number(1000), "1,000");
        assert_eq!(format_number(1000000), "1,000,000");
        assert_eq!(format_number(123), "123");
    }

    #[test]
    fn test_compression_ratio() {
        assert_eq!(format_compression_ratio(1000, 500), "50.0%");
        assert_eq!(format_compression_ratio(1000, 750), "75.0%");
        assert_eq!(format_compression_ratio(0, 100), "N/A");
    }
}
