use anyhow::Result;
use clap::Parser;

mod archive_name;
mod cli;
mod compress;
mod config;
mod crypto;
mod fuzzer;
mod state;
mod utils;

use cli::Cli;

fn main() -> Result<()> {
    let cli = Cli::parse();
    utils::show_banner();
    cli.run()
}
