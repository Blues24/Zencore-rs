use anyhow::{Context, Result};
use chrono::Local;
use clap::{Parser, Subcommand};
use dialoguer::{theme::ColorfulTheme, Confirm, Password, Select};
use std::fs;

use crate::{
    archive_name::ArchiveNamer, compress::Archiver, config::Config, 
    crypto::{Checker, Encryptor},
    fuzzer::Fuzzer, remote::RemoteTransfer,
    state::{ArchiveMetadata, StateTracker}, utils,
};

#[derive(Parser)]
#[command(name = "zencore")]
#[command(author = "Blues24")]
#[command(version = "1.3.1 - Oswin")]
#[command(about = "🎶 Blues Zencore - Minimalist Music Backup Tool", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}
// cli.rs - Key additions for advanced features

#[derive(Subcommand)]
enum Commands {
    Backup {
        #[arg(short, long)]
        source: Option<String>,
        
        #[arg(short, long)]
        destination: Option<String>,
        
        #[arg(short, long)]
        name: Option<String>,
        
        #[arg(short, long)]
        algorithm: Option<String>,
        
        #[arg(short, long)]
        encrypt: bool,
        
        #[arg(long)]
        upload: bool,

        /// Compression level (tar.gz: 0-9, tar.zst: 1-22, zip: 0-9)
        #[arg(short = 'l', long)]
        level: Option<i32>,

        /// Number of threads (0 = auto)
        #[arg(short = 't', long, default_value = "0")]
        threads: usize,

        /// Checksum algorithms (comma-separated: sha256,blake3,sha3)
        #[arg(long, value_delimiter = ',')]
        checksums: Option<Vec<String>>,
    },
    
    List,
    Show { name: String },
    
    Verify { 
        archive: String,
        
        /// Checksum algorithm to verify
        #[arg(short, long, default_value = "sha256")]
        algorithm: Option<String>,
    },
    
    Config,
    
    Upload {
        archive: String,
        #[arg(long)]
        to: Option<String>,
    },
    
    Remote {
        #[command(subcommand)]
        action: RemoteAction,
    },
}

// Update run_backup signature
fn run_backup(
    &self,
    source: &Option<String>,
    destination: &Option<String>,
    name: &Option<String>,
    algorithm: &Option<String>,
    encrypt: bool,
    upload: bool,
    level: Option<i32>,
    threads: usize,
    checksums: &Option<Vec<String>>,
) -> Result<()> {
    let config = Config::load()?;

    // ... (existing source/destination logic) ...

    let algo = match algorithm {
        Some(a) => {
            let normalized = a.to_lowercase();
            if !["tar.gz", "tar.zst", "zip"].contains(&normalized.as_str()) {
                utils::print_warning(&format!("Unknown algorithm: {}", a));
                Self::select_algorithm_interactive()?
            } else {
                normalized
            }
        }
        None => Self::select_algorithm_interactive()?,
    };

    // Validate compression level
    let compression_level = if let Some(lvl) = level {
        match algo.as_str() {
            "tar.gz" | "zip" if lvl < 0 || lvl > 9 => {
                utils::print_warning(&format!("Invalid level {} for {}, using default", lvl, algo));
                None
            }
            "tar.zst" if lvl < 1 || lvl > 22 => {
                utils::print_warning(&format!("Invalid level {} for tar.zst, using default", lvl));
                None
            }
            _ => Some(lvl)
        }
    } else {
        config.compression_level
    };

    let namer = ArchiveNamer::new(
        name.clone(),
        dest_path.clone(),
        algo.clone(),
        config.date_format.clone(),
    );
    let archive_name = namer.generate()?;

    utils::print_info(&format!("📦 Archive name: {}", archive_name));
    utils::print_info(&format!("📁 Source: {}", source_path));
    utils::print_info(&format!("💾 Destination: {}", dest_path));
    utils::print_info(&format!("🔧 Algorithm: {}", algo));
    
    if let Some(lvl) = compression_level {
        utils::print_info(&format!("⚙️  Compression level: {}", lvl));
    }
    
    if threads > 0 {
        utils::print_info(&format!("🧵 Threads: {}", threads));
    } else {
        utils::print_info(&format!("🧵 Threads: auto ({})", num_cpus::get()));
    }

    let proceed = Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt("Proceed with backup?")
        .default(true)
        .interact()?;

    if !proceed {
        utils::print_info("Backup cancelled");
        return Ok(());
    }

    // Create archiver with advanced options
    let mut archiver = Archiver::new(
        &source_path,
        &dest_path,
        archive_name.clone(),
        algo.clone(),
    );

    if threads > 0 {
        archiver = archiver.with_threads(threads);
    }

    if let Some(lvl) = compression_level {
        archiver = archiver.with_compression_level(lvl);
    }

    let (archive_path, file_list) = archiver.compress()?;

    utils::print_success(&format!("Compressed to: {}", archive_path.display()));

    // Handle multiple checksums
    let checksum_algos = if let Some(algos) = checksums {
        algos.clone()
    } else if !config.default_hash_algorithm.is_empty() {
        vec![config.default_hash_algorithm.clone()]
    } else {
        vec!["sha256".to_string()]
    };

    utils::print_info(&format!(
        "Generating checksums: {}",
        checksum_algos.join(", ")
    ));

    let mut checksums_map = std::collections::HashMap::new();
    
    for algo in &checksum_algos {
        let checksum = match algo.to_lowercase().as_str() {
            "sha256" | "sha-256" => {
                crate::crypto::Checker::generate_checksum(archive_path.to_str().unwrap())?
            }
            "blake3" => {
                // TODO: Implement BLAKE3 (next iteration)
                utils::print_warning("BLAKE3 not yet implemented, using SHA-256");
                crate::crypto::Checker::generate_checksum(archive_path.to_str().unwrap())?
            }
            "sha3" => {
                // TODO: Implement SHA3 (next iteration)
                utils::print_warning("SHA3 not yet implemented, using SHA-256");
                crate::crypto::Checker::generate_checksum(archive_path.to_str().unwrap())?
            }
            _ => {
                utils::print_warning(&format!("Unknown algorithm {}, using SHA-256", algo));
                crate::crypto::Checker::generate_checksum(archive_path.to_str().unwrap())?
            }
        };

        checksums_map.insert(algo.to_uppercase(), checksum.clone());
        utils::print_success(&format!("{}: {}", algo.to_uppercase(), checksum));
    }

    if config.generate_checksum_file {
        crate::crypto::Checker::generate_checksum_file(archive_path.to_str().unwrap())?;
    }

    // ... (rest of encryption, upload, save metadata logic) ...

    let file_size = fs::metadata(&archive_path)?.len();
    let metadata = ArchiveMetadata {
        name: archive_name,
        created_at: Local::now().to_rfc3339(),
        checksum: checksums_map.get("SHA256")
            .or_else(|| checksums_map.values().next())
            .unwrap_or(&String::new())
            .clone(),
        algorithm: algo,
        size_bytes: file_size,
        file_count: file_list.len(),
        encrypted,
        contents: file_list,
    };

    let mut state = StateTracker::load()?;
    state.add_archive(metadata);
    state.save()?;

    utils::print_success("✓ Backup completed successfully!");
    utils::print_info(&format!("Files backed up: {}", file_count));
    utils::print_info(&format!(
        "Archive size: {:.2} MB",
        file_size as f64 / 1_048_576.0
    ));

    Ok(())
}

// Update match in run() method
Some(Commands::Backup { 
    source, 
    destination, 
    name, 
    algorithm, 
    encrypt, 
    upload,
    level,
    threads,
    checksums,
}) => {
    self.run_backup(
        source, 
        destination, 
        name, 
        algorithm, 
        *encrypt, 
        *upload,
        *level,
        *threads,
        checksums,
    )
}
