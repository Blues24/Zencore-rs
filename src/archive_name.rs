use anyhow::Result;
use chrono::Local;
use std::path::Path;

pub struct ArchiveNamer {
    base_name: Option<String>,
    destination: String,
    algorithm: String,
    date_format: String,
    source_path: Option<String>,
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
            source_path: None,
        }
    }

    pub fn with_source_path(mut self, source: String) -> Self {
        self.source_path = Some(source);
        self
    }

    pub fn generate(&self) -> Result<String> {
        let base = match &self.base_name {
            Some(name) => self.expand_template(name),
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

    fn expand_template(&self, template: &str) -> String {
        let mut result = template.to_string();

        result = result.replace("{date}", &Local::now().format(&self.date_format).to_string());
        result = result.replace("{algo}", &self.algorithm);
        result = result.replace("{algorithm}", &self.algorithm);

        if let Some(ref source) = self.source_path {
            let source_name = Path::new(source)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("archive");
            result = result.replace("{source}", source_name);
        }

        let timestamp = Local::now().timestamp();
        result = result.replace("{timestamp}", &timestamp.to_string());

        let date_parts = Local::now();
        result = result.replace("{year}", &date_parts.format("%Y").to_string());
        result = result.replace("{month}", &date_parts.format("%m").to_string());
        result = result.replace("{day}", &date_parts.format("%d").to_string());
        result = result.replace("{hour}", &date_parts.format("%H").to_string());
        result = result.replace("{minute}", &date_parts.format("%M").to_string());

        result
    }

    fn get_extension(&self) -> &str {
        match self.algorithm.as_str() {
            "tar.gz" => "tar.gz",
            "tar.zst" => "tar.zst",
            "zip" => "zip",
            _ => "archive",
        }
    }

    pub fn preview(&self, name: &str) -> String {
        let expanded_preview = self.expand_template(name);
        let extension = self.get_extension();
        format!("{}.{}", expanded_preview, extension)
    }
}

pub struct NamingPresets;

impl NamingPresets {
    pub fn all() -> Vec<(&'static str, &'static str)> {
        vec![
            ("Date & Time", "{date}"),
            ("Source Name + Date", "{source}_{date}"),
            ("Algorithm + Date", "{algo}_{date}"),
            ("Year/Month/Day", "{year}{month}{day}_{hour}{minute}"),
            ("Source + Algorithm", "{source}_{algo}"),
            ("Timestamp", "backup_{timestamp}"),
        ]
    }

    pub fn get_example(preset: &str, namer: &ArchiveNamer) -> String {
        namer.preview(preset)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_template_expansion() {
        let namer = ArchiveNamer::new(
            Some("{source}_{algo}_{date}".to_string()),
            "/tmp".to_string(),
            "tar.zst".to_string(),
            "%Y%m%d".to_string(),
        )
        .with_source_path("/home/user/Music".to_string());

        let result = namer.expand_template("{source}_{algo}");
        assert!(result.contains("Music_tar.zst"));
    }

    #[test]
    fn test_preview() {
        let namer = ArchiveNamer::new(
            None,
            "/tmp".to_string(),
            "zip".to_string(),
            "%Y%m%d".to_string(),
        );

        let preview = namer.preview("{year}-{month}");
        assert!(preview.contains(".zip"));
    }
}
