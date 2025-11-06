use anyhow::Result;
use clap::Parser;

// Module declarations
mod archive_name;
mod cli;
mod compress;
mod config;
mod crypto;
mod crypto_extended;  // NEW: Advanced encryption
mod fuzzer;
mod path_utils;  // NEW: Cross-platform paths
mod state;
mod utils;

use cli::Cli;

/// Main entry point
/// Parses CLI arguments and runs the application
fn main() -> Result<()> {
    // Parse command-line arguments
    let cli = Cli::parse();
    
    // Show application banner
    utils::ConsoleTemplate::show_banner();
    
    // Run the CLI command
    cli.run()
}
