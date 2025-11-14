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
#[command(version = "1.3.0 - Oswin")]
#[command(about = "🎶 Blues Zencore - Minimalist Music Backup Tool", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

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
    },
    List,
    Show { name: String },
    Verify { archive: String },
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

#[derive(Subcommand)]
enum RemoteAction {
    List,
    Test { remote: String },
}

impl Cli {
    pub fn run(&self) -> Result<()> {
        match &self.command {
            Some(Commands::Backup { source, destination, name, algorithm, encrypt, upload }) => {
                self.run_backup(source, destination, name, algorithm, *encrypt, *upload)
            }
            Some(Commands::List) => self.run_list(),
            Some(Commands::Show { name }) => self.run_show(name),
            Some(Commands::Verify { archive }) => self.run_verify(archive),
            Some(Commands::Config) => self.run_config(),
            Some(Commands::Upload { archive, to }) => self.run_upload(archive, to),
            Some(Commands::Remote { action }) => self.run_remote(action),
            None => self.run_interactive(),
        }
    }

    fn run_backup(
        &self,
        source: &Option<String>,
        destination: &Option<String>,
        name: &Option<String>,
        algorithm: &Option<String>,
        encrypt: bool,
        upload: bool,
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
                Ok(selected) => selected.to_string_lossy().to_string(),
                Err(_) => {
                    utils::print_warning("No music folders found in config paths");
                    utils::print_info("Please enter source path manually:");
                    
                    let manual_path = dialoguer::Input::<String>::new()
                        .with_prompt("Source folder")
                        .interact_text()?;
                    
                    let expanded = shellexpand::tilde(&manual_path).to_string();
                    if !std::path::Path::new(&expanded).exists() {
                        return Err(anyhow::anyhow!("Path does not exist: {}", expanded));
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

        let namer = ArchiveNamer::new(name.clone(), dest_path.clone(), algo.clone(), config.date_format.clone());
        let archive_name = namer.generate()?;

        utils::print_info(&format!("📦 Archive name: {}", archive_name));
        utils::print_info(&format!("📁 Source: {}", source_path));
        utils::print_info(&format!("💾 Destination: {}", dest_path));
        utils::print_info(&format!("🔧 Algorithm: {}", algo));
        
        let proceed = Confirm::with_theme(&ColorfulTheme::default())
            .with_prompt("Proceed with backup?")
            .default(true)
            .interact()?;
        
        if !proceed {
            utils::print_info("Backup cancelled");
            return Ok(());
        }

        let archiver = Archiver::new(&source_path, &dest_path, archive_name.clone(), algo.clone());
        let (archive_path, file_list) = archiver.compress()?;

        utils::print_success(&format!("Compressed to: {}", archive_path.display()));

        utils::print_info("Generating SHA-256 checksum...");
        let checksum = Checker::generate_checksum(archive_path.to_str().unwrap())?;
        utils::print_success(&format!("Checksum: {}", checksum));

        if config.generate_checksum_file {
            Checker::generate_checksum_file(archive_path.to_str().unwrap())?;
        }

        let encrypted = if encrypt {
            let password = Password::with_theme(&ColorfulTheme::default())
                .with_prompt("Enter encryption password")
                .with_confirmation("Confirm password", "Passwords don't match")
                .interact()?;

            let encryptor = Encryptor::new(password);
            encryptor.encrypt_file(archive_path.to_str().unwrap())?;
            
            if config.generate_checksum_file {
                utils::print_info("Updating checksum for encrypted archive...");
                Checker::generate_checksum_file(archive_path.to_str().unwrap())?;
            }
            
            true
        } else if config.encrypt_by_default {
            let should_encrypt = Confirm::with_theme(&ColorfulTheme::default())
                .with_prompt("Encrypt archive? (configured as default)")
                .default(true)
                .interact()?;
            
            if should_encrypt {
                let password = Password::with_theme(&ColorfulTheme::default())
                    .with_prompt("Enter encryption password")
                    .with_confirmation("Confirm password", "Passwords don't match")
                    .interact()?;

                let encryptor = Encryptor::new(password);
                encryptor.encrypt_file(archive_path.to_str().unwrap())?;
                
                if config.generate_checksum_file {
                    utils::print_info("Updating checksum for encrypted archive...");
                    Checker::generate_checksum_file(archive_path.to_str().unwrap())?;
                }
                
                true
            } else {
                false
            }
        } else {
            let should_encrypt = Confirm::with_theme(&ColorfulTheme::default())
                .with_prompt("Encrypt archive with password?")
                .default(false)
                .interact()?;
            
            if should_encrypt {
                let password = Password::with_theme(&ColorfulTheme::default())
                    .with_prompt("Enter encryption password")
                    .with_confirmation("Confirm password", "Passwords don't match")
                    .interact()?;

                let encryptor = Encryptor::new(password);
                encryptor.encrypt_file(archive_path.to_str().unwrap())?;
                
                if config.generate_checksum_file {
                    utils::print_info("Updating checksum for encrypted archive...");
                    Checker::generate_checksum_file(archive_path.to_str().unwrap())?;
                }
                
                true
            } else {
                false
            }
        };

        if config.verify_after_backup {
            utils::print_info("🔍 Verifying backup integrity...");
            if Checker::auto_verify(archive_path.to_str().unwrap())? {
                utils::print_success("✓ Backup verified successfully!");
            } else {
                utils::print_error("✗ Backup verification failed!");
                return Err(anyhow::anyhow!("Backup verification failed"));
            }
        }

        let should_upload = if upload {
            true
        } else if let Some(ref remote_config) = config.remote {
            if remote_config.auto_upload && remote_config.enabled {
                true
            } else if remote_config.enabled {
                Confirm::with_theme(&ColorfulTheme::default())
                    .with_prompt("📤 Upload to remote?")
                    .default(false)
                    .interact()?
            } else {
                false
            }
        } else {
            false
        };

        if should_upload {
            Self::handle_remote_upload(&config, archive_path.to_str().unwrap())?;
        }

        let file_size = fs::metadata(&archive_path)?.len();
        let metadata = ArchiveMetadata {
            name: archive_name,
            created_at: Local::now().to_rfc3339(),
            checksum: checksum.clone(),
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
        utils::print_info(&format!("Files backed up: {}", metadata.file_count));
        utils::print_info(&format!("Archive size: {:.2} MB", file_size as f64 / 1_048_576.0));

        Ok(())
    }

    fn handle_remote_upload(config: &Config, archive_path: &str) -> Result<()> {
        if let Some(ref remote_config) = config.remote {
            if let Some(ref rclone) = remote_config.rclone {
                utils::print_info("📤 Uploading to remote storage...");
                
                RemoteTransfer::upload_to_rclone(
                    archive_path,
                    &rclone.remote_name,
                    &rclone.remote_path,
                )?;
                
                if rclone.verify_after_upload {
                    utils::print_info("Verifying remote upload...");
                    utils::print_success("✓ Upload verified (rclone checksum)");
                }
                
                return Ok(());
            }
            
            if let Some(ref db) = remote_config.database {
                utils::print_info("📤 Uploading to database...");
                
                let password = if db.password.is_some() {
                    db.password.clone().unwrap()
                } else {
                    Password::with_theme(&ColorfulTheme::default())
                        .with_prompt("Database password")
                        .interact()?
                };
                
                RemoteTransfer::upload_to_database(
                    archive_path,
                    &db.host,
                    db.port,
                    &db.username,
                    &password,
                    &db.database,
                    &db.table,
                )?;
                
                return Ok(());
            }
        }
        
        utils::print_warning("No remote configured. Skipping upload.");
        Ok(())
    }

    fn run_upload(&self, archive: &str, to: &Option<String>) -> Result<()> {
        if !std::path::Path::new(archive).exists() {
            return Err(anyhow::anyhow!("Archive not found: {}", archive));
        }

        let config = Config::load()?;

        if let Some(destination) = to {
            if destination.contains(':') {
                let parts: Vec<&str> = destination.splitn(2, ':').collect();
                let remote_name = parts[0];
                let remote_path = parts.get(1).map(|s| s.to_string()).unwrap_or_default();
                
                RemoteTransfer::upload_to_rclone(archive, remote_name, &remote_path)?;
            } else if destination.starts_with("mysql://") {
                utils::print_error("Database upload via URL not yet implemented");
                utils::print_info("Use config.toml for database settings");
            } else {
                return Err(anyhow::anyhow!("Invalid destination format. Use 'remote:path' or 'mysql://...'"));
            }
        } else {
            Self::handle_remote_upload(&config, archive)?;
        }

        Ok(())
    }

    fn run_remote(&self, action: &RemoteAction) -> Result<()> {
        match action {
            RemoteAction::List => {
                if !RemoteTransfer::check_rclone_installed()? {
                    utils::print_error("Rclone is not installed");
                    utils::print_info("Install: https://rclone.org/downloads/");
                    return Ok(());
                }

                utils::print_info("📡 Configured remotes:");
                let remotes = RemoteTransfer::list_rclone_remotes()?;
                
                if remotes.is_empty() {
                    utils::print_warning("No remotes configured");
                    utils::print_info("Run: rclone config");
                } else {
                    for remote in remotes {
                        println!("  • {}", remote);
                    }
                }
                Ok(())
            }
            RemoteAction::Test { remote } => {
                if !RemoteTransfer::check_rclone_installed()? {
                    utils::print_error("Rclone is not installed");
                    return Ok(());
                }

                RemoteTransfer::test_rclone_connection(remote)?;
                Ok(())
            }
        }
    }

    fn run_list(&self) -> Result<()> {
        let state = StateTracker::load()?;
        let archives = state.list_archives();

        if archives.is_empty() {
            utils::print_warning("No archives found. Create one with 'zencore backup'");
            return Ok(());
        }

        println!("\n📦 Available Archives:\n");
        println!("{:<30} {:<20} {:<15} {:<10}", "Name", "Created", "Size", "Files");
        println!("{}", "─".repeat(80));

        for archive in archives {
            let size_mb = archive.size_bytes as f64 / 1_048_576.0;
            let created = archive.created_at.split('T').next().unwrap_or("unknown");

            println!("{:<30} {:<20} {:>10.2} MB {:>10}", archive.name, created, size_mb, archive.file_count);
        }

        println!();
        Ok(())
    }

    fn run_show(&self, name: &str) -> Result<()> {
        let state = StateTracker::load()?;
        let archive = state.get_archive(name).context("Archive not found in state")?;

        println!("\n📦 Archive Details: {}\n", archive.name);
        println!("Created:    {}", archive.created_at);
        println!("Algorithm:  {}", archive.algorithm);
        println!("Checksum:   {}", archive.checksum);
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

    fn run_verify(&self, archive: &str) -> Result<()> {
        utils::print_info("🔍 Verifying archive integrity...");

        let checksum_path = format!("{}.sha256", archive);
        
        if std::path::Path::new(&checksum_path).exists() {
            utils::print_info(&format!("Found checksum file: {}", checksum_path));
            
            if Checker::verify_from_checksum_file(archive)? {
                utils::print_success("✓ Checksum matches! Archive is intact.");
            } else {
                utils::print_error("✗ Checksum mismatch! Archive may be corrupted.");
                return Err(anyhow::anyhow!("Checksum verification failed"));
            }
        } else {
            let checksum = Checker::generate_checksum(archive)?;
            utils::print_success(&format!("Checksum: {}", checksum));

            let archive_name = std::path::Path::new(archive)
                .file_name()
                .and_then(|n| n.to_str())
                .context("Invalid archive path")?;

            let state = StateTracker::load()?;
            if let Some(metadata) = state.get_archive(archive_name) {
                if Checker::verify_checksum(archive, &metadata.checksum)? {
                    utils::print_success("✓ Checksum matches state! Archive is intact.");
                } else {
                    utils::print_error("✗ Checksum mismatch with state!");
                }
            } else {
                utils::print_warning("No stored checksum found in state.");
                utils::print_info("💡 Tip: Checksum file will be auto-generated next time");
            }
        }

        Ok(())
    }

    fn run_config(&self) -> Result<()> {
        let config_path = Config::config_path()?;
        let state_dir = Config::state_dir()?;

        println!("\n⚙️  Configuration:");
        println!("Config file: {}", config_path.display());
        println!("State dir:   {}", state_dir.display());

        if config_path.exists() {
            let config = Config::load()?;
            println!("\nCurrent settings:");
            println!("  Default algorithm: {}", config.default_algorithm);
            println!("  Date format:       {}", config.date_format);
            println!("  Encrypt by default: {}", config.encrypt_by_default);
            println!("  Generate checksum file: {}", config.generate_checksum_file);
            println!("  Verify after backup: {}", config.verify_after_backup);
            
            if let Some(ref remote) = config.remote {
                println!("\nRemote settings:");
                println!("  Enabled: {}", remote.enabled);
                println!("  Auto-upload: {}", remote.auto_upload);
                
                if let Some(ref rclone) = remote.rclone {
                    println!("  Rclone remote: {}:{}", rclone.remote_name, rclone.remote_path);
                }
                
                if let Some(ref db) = remote.database {
                    println!("  Database: {}:{}/{}", db.host, db.port, db.database);
                }
            }
        } else {
            utils::print_warning("Config file doesn't exist yet. Will be created on first backup.");
        }

        println!();
        Ok(())
    }

    fn run_interactive(&self) -> Result<()> {
        let config = Config::load()?;
        let choices = vec![
            "Create Backup",
            "List Archives",
            "Show Archive Contents",
            "Upload to Remote",
            "Remote Management",
            "Exit"
        ];

        let selection = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("What would you like to do?")
            .items(&choices)
            .default(0)
            .interact()?;

        match selection {
            0 => {
                let encrypt = Confirm::with_theme(&ColorfulTheme::default())
                    .with_prompt("Encrypt archive with password?")
                    .default(config.encrypt_by_default)
                    .interact()?;

                let upload = if config.remote.as_ref().map(|r| r.enabled).unwrap_or(false) {
                    Confirm::with_theme(&ColorfulTheme::default())
                        .with_prompt("📤 Upload to remote after backup?")
                        .default(config.remote.as_ref().map(|r| r.auto_upload).unwrap_or(false))
                        .interact()?
                } else {
                    false
                };

                self.run_backup(&None, &None, &None, &None, encrypt, upload)
            }
            1 => self.run_list(),
            2 => {
                let state = StateTracker::load()?;
                let archives = state.list_archives();

                if archives.is_empty() {
                    utils::print_warning("No archives found");
                    return Ok(());
                }

                let names: Vec<String> = archives.iter().map(|a| a.name.clone()).collect();

                let selection = Select::with_theme(&ColorfulTheme::default())
                    .with_prompt("Select archive to show")
                    .items(&names)
                    .interact()?;

                self.run_show(&names[selection])
            }
            3 => {
                utils::print_info("Enter archive path:");
                let archive_path = dialoguer::Input::<String>::new()
                    .with_prompt("Archive")
                    .interact_text()?;

                self.run_upload(&archive_path, &None)
            }
            4 => {
                let remote_choices = vec!["List Remotes", "Test Connection", "Back"];
                let selection = Select::with_theme(&ColorfulTheme::default())
                    .with_prompt("Remote Management")
                    .items(&remote_choices)
                    .interact()?;

                match selection {
                    0 => self.run_remote(&RemoteAction::List),
                    1 => {
                        let remote = dialoguer::Input::<String>::new()
                            .with_prompt("Remote name")
                            .interact_text()?;
                        self.run_remote(&RemoteAction::Test { remote })
                    }
                    _ => Ok(()),
                }
            }
            _ => {
                utils::print_info("Goodbye! 👋");
                Ok(())
            }
        }
    }

    fn select_destination_interactive(config: &Config) -> Result<String> {
        utils::print_info("💾 Where do you want to save the backup?");
        
        match Fuzzer::find_and_select(&config.backup_folders, "backups") {
            Ok(selected) => Ok(selected.to_string_lossy().to_string()),
            Err(_) => {
                utils::print_warning("No backup folders found");
                utils::print_info("Enter destination path manually:");
                
                let manual_path = dialoguer::Input::<String>::new()
                    .with_prompt("Destination folder")
                    .interact_text()?;
                
                let expanded = shellexpand::tilde(&manual_path).to_string();
                
                if !std::path::Path::new(&expanded).exists() {
                    let create = Confirm::with_theme(&ColorfulTheme::default())
                        .with_prompt("Folder doesn't exist. Create it?")
                        .default(true)
                        .interact()?;
                    
                    if create {
                        fs::create_dir_all(&expanded)?;
                        utils::print_success(&format!("Created: {}", expanded));
                    } else {
                        return Err(anyhow::anyhow!("Destination folder required"));
                    }
                }
                
                Ok(expanded)
            }
        }
    }

    fn select_algorithm_interactive() -> Result<String> {
        utils::print_info("📦 Select compression algorithm:");
        
        let algorithms = vec![
            ("tar.zst (Recommended)", "tar.zst", "⚡ Fast & High compression"),
            ("tar.gz (Compatible)", "tar.gz", "🔧 Good compatibility"),
            ("zip (Universal)", "zip", "🌍 Works everywhere"),
        ];
        
        let choices: Vec<String> = algorithms
            .iter()
            .map(|(name, _, desc)| format!("{} - {}", name, desc))
            .collect();
        
        let selection = dialoguer::FuzzySelect::with_theme(&ColorfulTheme::default())
            .with_prompt("Compression algorithm")
            .items(&choices)
            .default(0)
            .interact()?;
        
        Ok(algorithms[selection].1.to_string())
    }
}
