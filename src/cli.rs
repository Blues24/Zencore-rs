use anyhow::{Context, Result};
use chrono::Local;
use clap::{Parser, Subcommand};
use dialoguer::{theme::ColorfulTheme, Confirm, Password, Select};
use std::collections::HashMap;
use std::fs;

use crate::{
    archive_name::{ArchiveNamer, NamingPresets},
    compress::Archiver,
    config::Config,
    crypto::{Checker, HashAlgorithm},
    fuzzer::Fuzzer,
    remote::RemoteTransfer,
    state::{ArchiveMetadata, StateTracker},
    utils,
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

#[derive(Subcommand)]
enum RemoteAction {
    List,
    Test { remote: String },
}

impl Cli {
    pub fn run(&self) -> Result<()> {
        match &self.command {
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
            }) => self.run_backup(
                source,
                destination,
                name,
                algorithm,
                *encrypt,
                *upload,
                *level,
                *threads,
                checksums,
            ),
            Some(Commands::List) => self.run_list(),
            Some(Commands::Show { name }) => self.run_show(name),
            Some(Commands::Verify { archive, algorithm }) => self.run_verify(archive, algorithm),
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
        level: Option<i32>,
        threads: usize,
        checksums: &Option<Vec<String>>,
    ) -> Result<()> {
        let config = Config::load()?;

        // SOURCE SELECTION
        let source_path = match source {
            Some(path) => {
                let expanded = shellexpand::tilde(path).to_string();
                if !std::path::Path::new(&expanded).exists() {
                    utils::print_warning(&format!("Path not found: {}", path));
                    utils::print_info("Falling back to interactive selection...");
                    
                    let fuzzer_config = config.get_fuzzer_config();
                    let selected = Fuzzer::find_and_select_with_config(
                        &config.music_folders,
                        "music",
                        fuzzer_config,
                    )?;
                    selected.to_string_lossy().to_string()
                } else {
                    expanded
                }
            }
            None => {
                let fuzzer_config = config.get_fuzzer_config();
                match Fuzzer::find_and_select_with_config(
                    &config.music_folders,
                    "music",
                    fuzzer_config,
                ) {
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
                }
            }
        };

        // Show folder info
        if let Ok(info) = Fuzzer::get_folder_info(&source_path) {
            info.display();
        }

        // DESTINATION SELECTION
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
                    let default_dest =
                        shellexpand::tilde(&config.default_backup_destination).to_string();

                    let use_default = Confirm::with_theme(&ColorfulTheme::default())
                        .with_prompt(format!(
                            "Use default destination: {}?",
                            config.default_backup_destination
                        ))
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

        // ALGORITHM SELECTION
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

        // COMPRESSION LEVEL VALIDATION
        let compression_level = if let Some(lvl) = level {
            match algo.as_str() {
                "tar.gz" | "zip" if lvl < 0 || lvl > 9 => {
                    utils::print_warning(&format!(
                        "Invalid level {} for {}, using default",
                        lvl, algo
                    ));
                    None
                }
                "tar.zst" if lvl < 1 || lvl > 22 => {
                    utils::print_warning(&format!(
                        "Invalid level {} for tar.zst, using default",
                        lvl
                    ));
                    None
                }
                _ => Some(lvl),
            }
        } else {
            config.compression_level
        };

        // ARCHIVE NAMING (Interactive or CLI)
        let archive_name_input = if name.is_some() {
            name.clone()
        } else {
            Self::select_archive_name_interactive(&source_path, &dest_path, &algo, &config.date_format)?
        };

        let namer = ArchiveNamer::new(
            archive_name_input,
            dest_path.clone(),
            algo.clone(),
            config.date_format.clone(),
            )
            .with_source_path(source_path.clone());

        let archive_name = namer.generate()?;


        // DISPLAY CONFIGURATION
        utils::print_header("Backup Configuration");
        utils::print_summary(&[
            ("Archive name", archive_name.clone()),
            ("Source", source_path.clone()),
            ("Destination", dest_path.clone()),
            ("Algorithm", algo.clone()),
        ]);

        if let Some(lvl) = compression_level {
            utils::print_info(&format!("⚙️  Compression level: {}", lvl));
        }

        if threads > 0 {
            utils::print_info(&format!("🧵 Threads: {}", threads));
        } else {
            utils::print_info(&format!("🧵 Threads: auto ({})", num_cpus::get()));
        }

        // ENCRYPTION SETUP
        let (should_encrypt, password) = if encrypt || config.encrypt_by_default {
            if algo == "zip" {
                let do_encrypt = if encrypt {
                    utils::print_info("Encrypting ZIP archive");
                    true 
                } else {
                    Confirm::with_theme(&ColorfulTheme::default())
                        .with_prompt("Encrypt archive? (configured as default)")
                        .default(true)
                        .interact()?
                };

                if do_encrypt {
                    let pwd = Password::with_theme(&ColorfulTheme::default())
                        .with_prompt("Enter encryption password")
                        .with_confirmation("Confirm password", "Passwords don't match")
                        .interact()?;
                    (true, Some(pwd))
                } else {
                    (false, None)
                }
            } else {
                utils::print_warning(&format!(
                    "{} doesn't support built-in encryption",
                    algo
                ));
                utils::print_info("💡 Tip: Use 'zip' format for native encryption");

                let use_post_encrypt = Confirm::with_theme(&ColorfulTheme::default())
                    .with_prompt("Encrypt after compression? (AES-256-GCM wrapper)")
                    .default(encrypt)
                    .interact()?;

                if use_post_encrypt {
                    let pwd = Password::with_theme(&ColorfulTheme::default())
                        .with_prompt("Enter encryption password")
                        .with_confirmation("Confirm password", "Passwords don't match")
                        .interact()?;
                    (true, Some(pwd))
                } else {
                    (false, None)
                }
            }
        } else {
            (false, None)
        };

        let proceed = Confirm::with_theme(&ColorfulTheme::default())
            .with_prompt("Proceed with backup?")
            .default(true)
            .interact()?;

        if !proceed {
            utils::print_info("Backup cancelled");
            return Ok(());
        }

        // CREATE ARCHIVER WITH ALL OPTIONS
        let start_time = std::time::Instant::now();

        let mut archiver = Archiver::new(&source_path, &dest_path, archive_name.clone(), algo.clone())
            .with_size_sorting(config.sort_files_by_size);

        if threads > 0 {
            archiver = archiver.with_threads(threads);
        }

        if let Some(lvl) = compression_level {
            archiver = archiver.with_compression_level(lvl);
        }

        if algo == "zip" && password.is_some() {
            archiver = archiver.with_password(password.clone().unwrap());
        }

        let (archive_path, file_list) = archiver.compress()?;

        let compress_duration = start_time.elapsed();

        utils::print_success(&format!("Compressed to: {}", archive_path.display()));
        utils::print_info(&format!("Time: {}", utils::format_duration(compress_duration)));

        // MULTI-CHECKSUM GENERATION
        let checksum_algos = if let Some(algos) = checksums {
            algos.clone()
        } else if !config.default_hash_algorithm.is_empty() {
            vec![config.default_hash_algorithm.clone()]
        } else {
            vec!["sha256".to_string()]
        };

        utils::print_info(&format!("Generating checksums: {}", checksum_algos.join(", ")));

        let algorithms: Vec<HashAlgorithm> = checksum_algos
            .iter()
            .filter_map(|s| HashAlgorithm::from_str(s).ok())
            .collect();

        let checksum_results =
            Checker::generate_multiple_checksums(archive_path.to_str().unwrap(), &algorithms)?;

        let mut checksums_map = HashMap::new();

        for (algo_enum, hash) in checksum_results {
            utils::print_success(&format!("{}: {}", algo_enum.name(), hash));
            checksums_map.insert(algo_enum.name().to_string(), hash);
        }

        if config.generate_checksum_file {
            Checker::generate_checksum_file(archive_path.to_str().unwrap())?;
        }

        // POST-COMPRESSION ENCRYPTION (TAR formats)
        let encrypted = if should_encrypt && password.is_some(){
            if algo == "zip"{
                utils::print_info("Applying ZIP native encryption..."); 
            } else {
                utils::print_info("Applying age encryption to TAR...");

                let encryptor = crate::encrypt_tar::TarEncryptor::new(password.unwrap());
                encryptor.encrypt_file(archive_path.to_str().unwrap())?;

                if config.generate_checksum_file {
                    utils::print_info("Updating checksum for encrypted archive..");
                    Checker::generate_checksum_file(archive_path.to_str().unwrap())?;
                }
                 
            } else {
                utils::print_warning("No file needed to be encrypted");
            };
        };
        // VERIFY IF ENABLED
        if config.verify_after_backup {
            utils::print_info("🔍 Verifying backup integrity...");
            if Checker::auto_verify(archive_path.to_str().unwrap())? {
                utils::print_success("✓ Backup verified successfully!");
            } else {
                utils::print_error("✗ Backup verification failed!");
                return Err(anyhow::anyhow!("Backup verification failed"));
            }
        }

        // REMOTE UPLOAD
        if upload || config.remote.as_ref().map(|r| r.auto_upload && r.enabled).unwrap_or(false) {
            Self::handle_remote_upload(&config, archive_path.to_str().unwrap())?;
        }

        // SAVE METADATA
        let file_size = fs::metadata(&archive_path)?.len();
        let mut metadata = ArchiveMetadata {
            name: archive_name,
            created_at: Local::now().to_rfc3339(),
            checksum: String::new(),
            checksums: HashMap::new(),
            algorithm: algo,
            size_bytes: file_size,
            file_count: file_list.len(),
            encrypted,
            contents: file_list,
        };

        for (algo_name, hash) in checksums_map {
            metadata.add_checksum(&algo_name, hash);
        }

        let mut state = StateTracker::load()?;
        state.add_archive(metadata.clone());
        state.save()?;

        // FINAL SUMMARY
        let total_duration = start_time.elapsed();
        let original_size = if let Ok(info) = Fuzzer::get_folder_info(&source_path) {
            info.total_size
        } else {
            file_size
        };

        utils::print_header("Backup Complete");
        utils::print_summary(&[
            ("Files backed up", utils::format_number(metadata.file_count)),
            ("Original size", utils::format_bytes(original_size)),
            ("Archive size", utils::format_bytes(file_size)),
            ("Compression ratio", utils::format_compression_ratio(original_size, file_size)),
            ("Total time", utils::format_duration(total_duration)),
            ("Average speed", utils::format_speed(file_size, total_duration)),
        ]);

        if !metadata.checksums.is_empty() {
            utils::print_info("\nChecksums:");
            for (algo_name, hash) in metadata.list_checksums() {
                println!("  {} = {}", algo_name, hash);
            }
        }

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

    fn select_archive_name_interactive(
        source_path: &str,
        dest_path: &str,
        algo: &str,
        date_format: &str,
    ) -> Result<Option<String>> {
        utils::print_info("📝 Archive Naming Options");

        let choices = vec![
            "Auto-generate (date & time)",
            "Use preset template",
            "Custom name",
            "Advanced template",
        ];

        let selection = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("How do you want to name the archive?")
            .items(&choices)
            .default(0)
            .interact()?;

        match selection {
            0 => Ok(None),
            1 => {
                let presets = NamingPresets::all();
                let preset_names: Vec<String> = presets
                    .iter()
                    .map(|(name, template)| {
                        let namer = ArchiveNamer::new(
                            Some(template.to_string()),
                            dest_path.to_string(),
                            algo.to_string(),
                            date_format.to_string(),
                        )
                        .with_source_path(source_path.to_string());

                        let preview = namer.preview(template);
                        format!("{}: {}", name, preview)
                    })
                    .collect();

                let preset_selection = Select::with_theme(&ColorfulTheme::default())
                    .with_prompt("Select naming preset")
                    .items(&preset_names)
                    .interact()?;

                let (_, template) = presets[preset_selection];
                Ok(Some(template.to_string()))
            }
            2 => {
                let custom_name = dialoguer::Input::<String>::new()
                    .with_prompt("Enter archive name (without extension)")
                    .interact_text()?;

                if custom_name.trim().is_empty() {
                    utils::print_warning("Empty name, using auto-generate");
                    Ok(None)
                } else {
                    Ok(Some(custom_name))
                }
            }
            3 => {
                utils::print_info("\nAvailable variables:");
                println!("  {{date}}      - Current date/time");
                println!("  {{source}}    - Source folder name");
                println!("  {{algo}}      - Compression algorithm");
                println!("  {{year}}      - Current year (YYYY)");
                println!("  {{month}}     - Current month (MM)");
                println!("  {{day}}       - Current day (DD)");
                println!("  {{hour}}      - Current hour (HH)");
                println!("  {{minute}}    - Current minute (MM)");
                println!("  {{timestamp}} - Unix timestamp");
                println!("\nExample: backup_{{source}}_{{year}}{{month}}{{day}}");

                let template = dialoguer::Input::<String>::new()
                    .with_prompt("Enter template")
                    .interact_text()?;

                if template.trim().is_empty() {
                    Ok(None)
                } else {
                    let namer = ArchiveNamer::new(
                        Some(template.clone()),
                        dest_path.to_string(),
                        algo.to_string(),
                        date_format.to_string(),
                    )
                    .with_source_path(source_path.to_string());

                    let preview = namer.preview(&template);
                    utils::print_info(&format!("Preview: {}", preview));

                    let confirm = Confirm::with_theme(&ColorfulTheme::default())
                        .with_prompt("Use this name?")
                        .default(true)
                        .interact()?;

                    if confirm {
                        Ok(Some(template))
                    } else {
                        Ok(None)
                    }
                }
            }
            _ => Ok(None),
        }
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
            } else {
                return Err(anyhow::anyhow!(
                    "Invalid destination format. Use 'remote:path'"
                ));
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

        utils::print_header("Available Archives");

        let widths = [35, 20, 15, 10];
        utils::print_table_header(&["Name", "Created", "Size", "Files"], &widths);

        for archive in archives {
            let size_mb = archive.size_bytes as f64 / 1_048_576.0;
            let created = archive.created_at.split('T').next().unwrap_or("unknown");

            println!(
                "{:<35} {:<20} {:>10.2} MB {:>10}",
                utils::truncate_string(&archive.name, 35),
                created,
                size_mb,
                archive.file_count
            );
        }

        println!();
        Ok(())
    }

    fn run_show(&self, name: &str) -> Result<()> {
        let state = StateTracker::load()?;
        let archive = state
            .get_archive(name)
            .context("Archive not found in state")?;

        utils::print_header(&format!("Archive Details: {}", archive.name));

        println!("Created:    {}", archive.created_at);
        println!("Algorithm:  {}", archive.algorithm);

        let checksums = archive.list_checksums();
        if !checksums.is_empty() {
            println!("Checksums:");
            for (algo, hash) in checksums {
                println!("  {} = {}", algo, hash);
            }
        }

        println!("Size:       {}", utils::format_bytes(archive.size_bytes));
        println!("Files:      {}", utils::format_number(archive.file_count));
        println!(
            "Encrypted:  {}",
            if archive.encrypted { "Yes" } else { "No" }
        );

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

    fn run_verify(&self, archive: &str, algorithm: &Option<String>) -> Result<()> {
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
            let algo = if let Some(algo_str) = algorithm {
                HashAlgorithm::from_str(algo_str)?
            } else {
                HashAlgorithm::Sha256
            };

            let checksum = Checker::generate_checksum_with_algorithm(archive, algo)?;
            utils::print_success(&format!("{}: {}", algo.name(), checksum));

            let archive_name = std::path::Path::new(archive)
                .file_name()
                .and_then(|n| n.to_str())
                .context("Invalid archive path")?;

            let state = StateTracker::load()?;
            if let Some(metadata) = state.get_archive(archive_name) {
                if let Some(expected) = metadata.get_checksum(algo.name()) {
                    if Checker::verify_checksum_with_algorithm(archive, expected, algo)? {
                        utils::print_success(&format!("✓ {} matches state!", algo.name()));
                    } else {
                        utils::print_error(&format!("✗ {} mismatch with state!", algo.name()));
                    }
                } else {
                    utils::print_warning(&format!("No {} checksum in state", algo.name()));
                }

                let all_checksums = metadata.list_checksums();
                if !all_checksums.is_empty() {
                    utils::print_info("\nAvailable checksums in state:");
                    for (algo_name, hash) in all_checksums {
                        println!("  {} = {}", algo_name, hash);
                    }
                }
            } else {
                utils::print_warning("No stored checksum found in state.");
            }
        }

        Ok(())
    }

    fn run_config(&self) -> Result<()> {
        let config_path = Config::config_path()?;
        let state_dir = Config::state_dir()?;

        utils::print_header("Configuration");
        
        println!("Config file: {}", config_path.display());
        println!("State dir:   {}", state_dir.display());

        if config_path.exists() {
            let config = Config::load()?;
            
            utils::print_info("\nCurrent settings:");
            utils::print_summary(&[
                ("Default algorithm", config.default_algorithm.clone()),
                ("Date format", config.date_format.clone()),
                ("Encrypt by default", config.encrypt_by_default.to_string()),
                ("Generate checksum file", config.generate_checksum_file.to_string()),
                ("Verify after backup", config.verify_after_backup.to_string()),
                ("Compression level", 
                    config.compression_level.map(|l| l.to_string()).unwrap_or("auto".to_string())),
                ("Threads", 
                    if config.num_threads == 0 { "auto".to_string() } else { config.num_threads.to_string() }),
            ]);

            if let Some(ref remote) = config.remote {
                utils::print_info("\nRemote settings:");
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
            "Exit",
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

                self.run_backup(&None, &None, &None, &None, encrypt, upload, None, 0, &None)
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

        let fuzzer_config = config.get_fuzzer_config();
        match Fuzzer::find_and_select_with_config(&config.backup_folders, "backups", fuzzer_config) {
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
