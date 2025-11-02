use anyhow::{Context, Result};
use flate2::write::GzEncoder;
use flate2::compression;
use indicatif::{ProgressBar, ProgressStyle};
use rayon::prelude::*;
use std::fs::{self, File};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use walkdir::WalkDir;
use zstd::stream::write::Encoder as ZstdEncoder;

pub struct Archiver {
    source: PathBuf,
    destination: PathBuf,
    archive_name: String,
    algorithm: String,
    // number of thread to use if 0 = auto detect and use them all
    num_threads: usize,
    // Compression level 
    compression_level: Option<i32>

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
        }
    }

    pub fn with_threads(mut self, threads: usize) -> Self {
        self.num_threads = threads;
        self
    }

    /// Set compression level
    /// - tar.gz: 0-9 (6 is the default option)
    /// - tar.zst: 1-22 (3 is the default but 19+ is extreme)
    /// - zip:  0-9 (6 is the default option)
    pub fn with_compression_level(mut self, level: i32) -> Self {
        self.compression_level = Some(level);
        self
    }

    /// Main compression entry point
    /// 
    /// Return tuple of (archive_path, list_of_files)
    pub fn compress(&self) -> Result<(PathBuf, Vec<String>)> {
        let archive_path = self.destination.join(&self.archive_name);

        crate::utils::print_info(&format!(
            "Compressing with {} algorithm...",
            self.algorithm
        ));

        let num_threads = if self.num_threads == 0{
            num_cpus::get()
        } else {
            self.num_threads
        };

        rayon::ThreadPoolbuilder::new()
            .num_threads(num_threads)
            .build_global()
            .ok();

        crate::utils::print_info(&format!("Using {} threads", num_threads));

        // Collect all files in parallel for faster scanning
        let files = self.collect_files_parallel()?;
        let total_files = files.len() as u64;

        // Create progress bar as pb
        let pb = ProgressBar::new(total_files);
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} files ({msg})")
                .unwrap()
                .progress_chars("#>-"),
        );

        // Comress based on selected algorithm
        let file_list = match self.algorithm.as_str() {
            "tar.gz" => self.compress_tar_gz(&archive_path, &files, &pb)?,
            "tar.zst" => self.compress_tar_zst(&archive_path, &files, &pb)?,
            "zip" => self.compress_zip(&archive_path, &files, &pb)?,
            _ => return Err(anyhow::anyhow!("Unsupported algorithm: {}", self.algorithm)),
        };

        pb.finish_with_message("Done, :D");

        Ok((archive_path, file_list))
    }

    fn collect_files_parallel(&self) -> Result<Vec<PathBuf>> {
        crate::utils::print_info("Scanning directory... please wait a moment!");

        // collect all entries in parallel
        let entries: Vec<_> = Walkdir::new(&self.source)
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

        // set compression level
        let level = self.compression_level.unwrap_or(6);
        let compression = Compression::new(level as u32);

        let enc = GzEncoder::new(tar_gz, compression);
        let mut tar = Builder::new(enc);

        let mut file_list = Vec::with_capacity(files.len());

        for file_path in files {
            let relative = file_path.strip_prefix(&self.source);
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
        // Set compression level
        let level = self.compression_level.unwrap_or(3);
        let encode =  ZstdEncoder::new(tar_zst, level)?;

        let mut tar = Builder::new(encode.auto_finish());

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
        let zipFile = File::create(archive_path)?;
        let mut zip = zip::ZipWriter::new(zipFile);

        // set compression level
        let level = self.compression_level.unwrap_or(6) as u32;
        let options = zip::write::FileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated)
            .compression_level(Some(level));

        let mut file_list = Vec::with_capacity(files.len());

        // Append file into the archive
        for file_path in files {
            let relative = file_path.strip_prefix(&self.source)?;
            let name = relative.to_string_lossy().to_string();

            zip.start_file(&name, options)?;
            let mut f = File::open(file_path)?;
            io::copy(&mut f, &mut zip)?;

            file_list.push(name.clone());
            pb.inc(1);
            pb.set_message(name);
        }

        zip.finish()?;
        Ok(file_list)
        
    }
       
}