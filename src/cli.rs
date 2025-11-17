use anyhow::{Context, Result};
use chrono::Local;
use clap::{Parser, Subcommand};
use dialoguer::{theme::ColorfulTheme, Confirm, Password, Select};
use std::fs;

use crate::{
    archive_name::ArchiveNamer, archive_name::NamingPresets, 
    compress::Archiver, config::Config, 
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

    let source_path = match source {
        Some(path) => {
            let expanded = shellexpand::tilde(path).to_string();
            if !std::path::Path::new(&expanded).exists() {
                utils::print_warning(&format!("Path not found: {}", path));
                utils::print_info("Falling back to interactive selection...");
                let selected = Fuzzer::find_and_select(&config.music_folders, "music")?;
                selected.to_string_lossy().to_string()
            } else {
                expanded 
            }
        }

        None => match Fuzzer::find_and_select(&config.music_folders, "music") {
        Ok(selected) => selected.to_string_losssy().to_string(),
        Err(_) => {
            utils::print_warning("No Target folder found in config paths");
            utils::print_info("Please enter source path manually: ");

            let manual_path = dialoguer::Input::<String>::new()
                .with_prompt("Source folder")
                .interact_text()?;

            let expanded = shellexpand::tilde(&manual_path).to_string();
            if !std::path::Path::new(&expanded).exists() {
                return Err(anyhow::anyhow!("Path does not exists: {}", expanded));
             }
            expanded 
          }
        },
    };

    let dest_path = match destination {
            Some(path) => {
                let expanded = shellexpand::tilde(path).to_string();
                if !std::path::Path::new(&expanded).exists() {
                    utils::print_warning(&format!("Path not found: {}", path));

                    let create = Confirm::with_theme(&ColorfulTheme::default())
                        .with_prompt("Create destination folder?")
                        .default(true)
                        .interact()?;

                    if create {
                        fs::create_dir_all(&expanded)?;
                        utils::print_success(&format!("Created: {}", expanded));
                        expanded
                    } else {
                        return Err(anyhow::anyhow!("Destination folder required"));
                    }
                } else {
                    expanded
                }
            }
            None => {
                if !config.default_backup_destination.is_empty() {
                    let default_dest = shellexpand::tilde(&config.default_backup_destination).to_string();

                    let use_default = Confirm::with_theme(&ColorfulTheme::default())
                        .with_prompt(format!("Use default destination: {}?", config.default_backup_destination))
                        .default(true)
                        .interact()?;

                    if use_default {
                        if !std::path::Path::new(&default_dest).exists() {
                            fs::create_dir_all(&default_dest)?;
                            utils::print_success(&format!("Created: {}", default_dest));
                        }
                        default_dest
                    } else {
                        Self::select_destination_interactive(&config)?
                    }
                } else {
                    Self::select_destination_interactive(&config)?
                }
            }
        };

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
    state.add_archive(metadata.clone());
    state.save()?;

    utils::print_success("✓ Backup completed successfully!");
    utils::print_info(&format!("Files backed up: {}", file_count));
    utils::print_info(&format!(
        "Archive size: {:.2} MB",
        file_size as f64 / 1_048_576.0
    ));

    if !metadata.checksums.is_empty(){
        utils::print_info("\nChecksums: ");
        for (algo, hash) in metadata.list_checksums() {
            println!(" {} = {}", algo, hash);
        }
    }

    Ok(())
}

fn run_verify(&self, archive: &str, algorithm: &option<String>) -> Result<()> {
    utils::print_info("🔍 Verifying archive integrity...");

    let checksum_path = format!("{}.sha256", archive);

    if std::path::Path::new(&checksum_path).exists() {
        utils::print_info(&format!("Found checksum file: {}", checksum_path));

        if crate::crypto::Checker::verify_from_checksum_file(archive)? {
            utils::print_success("✓ Checksum matches! Archive is intact.");
        } else {
            utils::print_error("✗ Checksum mismatch! Archive may be corrupted.");
            return Err(anyhow::anyhow!("Checksum verification failed"));
        }
    } else {
        // Use specified algorithm or default to SHA-256
        let algo = if let Some(algo_str) = algorithm {
            crate::crypto::HashAlgorithm::from_str(algo_str)?
        } else {
            crate::crypto::HashAlgorithm::Sha256
        };

        let checksum = crate::crypto::Checker::generate_checksum_with_algorithm(
            archive,
            algo
        )?;
        utils::print_success(&format!("{}: {}", algo.name(), checksum));

        let archive_name = std::path::Path::new(archive)
            .file_name()
            .and_then(|n| n.to_str())
            .context("Invalid archive path")?;

        let state = StateTracker::load()?;
        if let Some(metadata) = state.get_archive(archive_name) {
            if let Some(expected) = metadata.get_checksum(algo.name()) {
                if crate::crypto::Checker::verify_checksum_with_algorithm(
                    archive,
                    expected,
                    algo
                )? {
                    utils::print_success(&format!("✓ {} matches state!", algo.name()));
                } else {
                    utils::print_error(&format!("✗ {} mismatch with state!", algo.name()));
                }
            } else {
                utils::print_warning(&format!("No {} checksum in state", algo.name()));
            }

            // Show all available checksums
            let all_checksums = metadata.list_checksums();
            if !all_checksums.is_empty() {
                utils::print_info("\nAvailable checksums in state:");
                for (algo_name, hash) in all_checksums {
                    println!("  {} = {}", algo_name, hash);
                }
            }
        } else {
            utils::print_warning("No stored checksum found in state.");
            utils::print_info("💡 Tip: Checksum file will be auto-generated next time");
        }
    }

    Ok(())
}

fn run_show(&self, name: &str) -> Result<()> {
    let state = StateTracker::load()?;
    let archive = state.get_archive(name).context("Archive not found in state")?;

    println!("\n📦 Archive Details: {}\n", archive.name);
    println!("Created:    {}", archive.created_at);
    println!("Algorithm:  {}", archive.algorithm);

    // Show all checksums
    let checksums = archive.list_checksums();
    if !checksums.is_empty() {
        println!("Checksums:");
        for (algo, hash) in checksums {
            println!("  {} = {}", algo, hash);
        }
    } else if !archive.checksum.is_empty() {
        println!("Checksum:   {}", archive.checksum);
    }

    println!("Size:       {:.2} MB", archive.size_bytes as f64 / 1_048_576.0);
    println!("Files:      {}", archive.file_count);
    println!("Encrypted:  {}", if archive.encrypted { "Yes" } else { "No" });

    println!("\n📄 Contents ({} files):\n", archive.contents.len());

    let limit = 50.min(archive.contents.len());
    for (i, file) in archive.contents.iter().take(limit).enumerate() {
        println!("  {}. {}", i + 1, file);
    }

    if archive.contents.len() > limit {
        println!("\n  ... and {} more files", archive.contents.len() - limit);
    }

    println!();
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
