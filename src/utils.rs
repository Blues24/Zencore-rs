use colored::*;

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
    ____  __                   ________                              
   / __ )/ /_  _____  _____   /_  / ___  ____  _________  _________ 
  / __  / / / / / _ \/ ___/    / / __ \/ __ \/ ___/ __ \/ ___/ _ \
 / /_/ / / /_/ /  __(__  )    / / /_/ / / / / /__/ /_/ / /  /  __/
/_____/_/\__,_/\___/____/    /_/\____/_/ /_/\___/\____/_/   \___/ 
    "#;

    println!("{}", banner.bright_magenta());
    println!("{}", "v1.0.0 - Minimalist Music Backup Tool".bright_cyan());
    println!();
}