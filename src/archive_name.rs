use anyhow::Result;
use chrono::Local;
use std::path::Path;

pub struct ArchiveNamer {
    base_name: Option<String>,
    destination: String,
    algorithm: String,
    date_format: String,
}

impl ArchiveNamer {
    pub fn new(
        base_name: Option<String>,
        destination: String,
        algorithm: String,
        date_format: String,
    ) -> Self {
        Self {
            base_name,
            destination,
            algorithm,
            date_format,
        }
    }

    pub fn generate(&self) -> Result<String> {
        let base = match &self.base_name {
            Some(name) => name.clone(),
            None => Local::now().format(&self.date_format).to_string(),
        };

        let ext = self.get_extension();
        let mut final_name = format!("{}.{}", base, ext);
        let mut full_path = Path::new(&self.destination).join(&final_name);

        if full_path.exists() {
            let mut counter = 1;
            loop {
                final_name = format!("{}.{}.{}", base, counter, ext);
                full_path = Path::new(&self.destination).join(&final_name);

                if !full_path.exists() {
                    break;
                }

                counter += 1;

                if counter > 9999 {
                    final_name = format!("{}.copy.{}", base, ext);
                    break;
                }
            }
        }

        Ok(final_name)
    }

    fn get_extension(&self) -> &str {
        match self.algorithm.as_str() {
            "tar.gz" => "tar.gz",
            "tar.zst" => "tar.zst",
            "zip" => "zip",
            _ => "archive",
        }
    }
}