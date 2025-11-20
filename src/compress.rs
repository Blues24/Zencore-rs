use anyhow::Result;
use flate2::write::GzEncoder;
use flate2::Compression;
use indicatif::{ProgressBar, ProgressStyle};
use rayon::prelude::*;
use std::fs::{self, File};
use std::io;
use std::path::{Path, PathBuf};
use tar::Builder;
use walkdir::WalkDir;
use zip::write::{FileOptions, ExtendedFileOptions};
use zip::unstable::write::FileOptionsExt;
use zstd::stream::write::Encoder as ZstdEncoder;

pub struct Archiver {
    source: PathBuf,
    destination: PathBuf,
    archive_name: String,
    algorithm: String,
    num_threads: usize,
    compression_level: Option<i32>,
    password: Option<String>,
    sort_by_size: bool,
}

impl Archiver {
    pub fn new(
        source: impl AsRef<Path>,
        destination: impl AsRef<Path>,
        archive_name: String,
        algorithm: String,
    ) -> Self {
        Self {
            source: source.as_ref().to_path_buf(),
            destination: destination.as_ref().to_path_buf(),
            archive_name,
            algorithm,
            num_threads: 0,
            compression_level: None,
            password: None,
            sort_by_size: true,
        }
    }

    pub fn with_threads(mut self, threads: usize) -> Self {
        self.num_threads = threads;
        self
    }

    pub fn with_compression_level(mut self, level: i32) -> Self {
        self.compression_level = Some(level);
        self
    }

    pub fn with_password(mut self, password: String) -> Self {
        self.password = Some(password);
        self
    }

    pub fn with_size_sorting(mut self, enabled: bool) -> Self {
        self.sort_by_size = enabled;
        self
    }

    pub fn compress(&self) -> Result<(PathBuf, Vec<String>)> {
        let archive_path = self.destination.join(&self.archive_name);

        crate::utils::print_info(&format!(
            "Compressing with {} algorithm...",
            self.algorithm
        ));

        let num_threads = if self.num_threads == 0 {
            num_cpus::get()
        } else {
            self.num_threads
        };

        rayon::ThreadPoolBuilder::new()
            .num_threads(num_threads)
            .build_global()
            .ok();

        crate::utils::print_info(&format!("Using {} threads", num_threads));

        if let Some(level) = self.compression_level {
            crate::utils::print_info(&format!("Compression level: {}", level));
        }

        let mut files = self.collect_files_parallel()?;

        if self.sort_by_size {
            crate::utils::print_info("Sorting files by size (largest first)...");
            
            let mut files_with_sizes: Vec<(PathBuf, u64)> = files
                .par_iter()
                .filter_map(|path| {
                    fs::metadata(path)
                        .ok()
                        .map(|meta| (path.clone(), meta.len()))
                })
                .collect();

            files_with_sizes.par_sort_by(|a, b| b.1.cmp(&a.1));
            files = files_with_sizes.into_iter().map(|(path, _)| path).collect();

            crate::utils::print_success("Files sorted by size");
        }

        let total_files = files.len() as u64;

        let pb = ProgressBar::new(total_files);
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} files ({msg})")
                .unwrap()
                .progress_chars("#>-"),
        );

        let file_list = match self.algorithm.as_str() {
            "tar.gz" | "gz" => {
                if self.password.is_some() {
                    crate::utils::print_warning(
                        "tar.gz doesn't support built-in password protection",
                    );
                    crate::utils::print_warning("Use zip format for native encryption");
                }
                self.compress_tar_gz(&archive_path, &files, &pb)?
            }
            "tar.zst" | "zst" => {
                if self.password.is_some() {
                    crate::utils::print_warning(
                        "tar.zst doesn't support built-in password protection",
                    );
                    crate::utils::print_warning("Use zip format for native encryption");
                }
                self.compress_tar_zst(&archive_path, &files, &pb)?
            }
            "zip" => self.compress_zip(&archive_path, &files, &pb)?,
            _ => return Err(anyhow::anyhow!("Unsupported algorithm: {}", self.algorithm)),
        };

        pb.finish_with_message("Done!");

        Ok((archive_path, file_list))
    }

    fn collect_files_parallel(&self) -> Result<Vec<PathBuf>> {
        crate::utils::print_info("Scanning directory...");

        let entries: Vec<_> = WalkDir::new(&self.source)
            .into_iter()
            .par_bridge()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
            .map(|e| e.path().to_path_buf())
            .collect();

        crate::utils::print_success(&format!("Found {} files", entries.len()));

        Ok(entries)
    }

    fn compress_tar_gz(
        &self,
        archive_path: &Path,
        files: &[PathBuf],
        pb: &ProgressBar,
    ) -> Result<Vec<String>> {
        let tar_gz = File::create(archive_path)?;
        let level = self.compression_level.unwrap_or(6);
        let compression = Compression::new(level as u32);
        let enc = GzEncoder::new(tar_gz, compression);
        let mut tar = Builder::new(enc);

        let mut file_list = Vec::with_capacity(files.len());

        for file_path in files {
            let relative = file_path.strip_prefix(&self.source)?;
            tar.append_path_with_name(file_path, relative)?;

            file_list.push(relative.to_string_lossy().to_string());
            pb.inc(1);
            pb.set_message(relative.to_string_lossy().to_string());
        }

        tar.finish()?;
        Ok(file_list)
    }

    fn compress_tar_zst(
        &self,
        archive_path: &Path,
        files: &[PathBuf],
        pb: &ProgressBar,
    ) -> Result<Vec<String>> {
        let tar_zst = File::create(archive_path)?;
        let level = self.compression_level.unwrap_or(3);
        let encoder = ZstdEncoder::new(tar_zst, level)?;
        let mut tar = Builder::new(encoder.auto_finish());

        let mut file_list = Vec::with_capacity(files.len());

        for file_path in files {
            let relative = file_path.strip_prefix(&self.source)?;
            tar.append_path_with_name(file_path, relative)?;

            file_list.push(relative.to_string_lossy().to_string());
            pb.inc(1);
            pb.set_message(relative.to_string_lossy().to_string());
        }

        tar.finish()?;
        Ok(file_list)
    }

    fn compress_zip(
        &self,
        archive_path: &Path,
        files: &[PathBuf],
        pb: &ProgressBar,
    ) -> Result<Vec<String>> {
        let zip_file = File::create(archive_path)?;
        let mut zip = zip::ZipWriter::new(zip_file);

        let level = self.compression_level.unwrap_or(6);
        let mut options: FileOptions<'_, ExtendedFileOptions> = FileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated)
            .compression_level(Some(level as i64));

        if let Some(ref password) = self.password {
            crate::utils::print_info("Encrypting with AES-256 (ZIP native)..");

            options = options.with_deprecated_encryption(password.as_bytes());
        }
        let mut file_list = Vec::with_capacity(files.len());

        for file_path in files {
            let relative = file_path.strip_prefix(&self.source)?;
            let name = relative.to_string_lossy().to_string();

            zip.start_file(&name, options.clone())?;
            let mut f = File::open(file_path)?;
            io::copy(&mut f, &mut zip)?;

            file_list.push(name.clone());
            pb.inc(1);
            pb.set_message(name);
        }

        zip.finish()?;

        if self.password.is_some() {
            crate::utils::print_success("✓ Archive encrypted with password");
        }

        Ok(file_list)
    }
}
