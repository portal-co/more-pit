use anyhow::{bail, Result};
use pit_packaging::ManifestMerge;

#[derive(Clone, Copy, Debug, Default)]
pub struct ReplaceFile;

impl ManifestMerge for ReplaceFile {
    fn id(&self) -> &'static str {
        "replace"
    }
    fn apply(&self, _path: &str, incoming: &str, _existing: Option<&str>) -> Result<String> {
        Ok(incoming.to_string())
    }
}

#[derive(Clone, Debug)]
pub struct AppendToPath;

impl ManifestMerge for AppendToPath {
    fn id(&self) -> &'static str {
        "append"
    }
    fn apply(&self, _path: &str, incoming: &str, existing: Option<&str>) -> Result<String> {
        Ok(match existing {
            Some(e) => format!("{e}\n{incoming}"),
            None => incoming.to_string(),
        })
    }
}

#[derive(Clone, Debug)]
pub struct CargoMetadataOverlay {
    pub section: String,
}

impl ManifestMerge for CargoMetadataOverlay {
    fn id(&self) -> &'static str {
        "cargo-metadata"
    }
    fn apply(&self, path: &str, incoming: &str, existing: Option<&str>) -> Result<String> {
        if path != "Cargo.toml" {
            bail!("CargoMetadataOverlay only applies to Cargo.toml");
        }
        let base = existing.unwrap_or(
            "[package]\nname = \"placeholder\"\nversion = \"0.0.0\"\nedition = \"2021\"\n",
        );
        let marker = format!("[package.metadata.{}]", self.section);
        if base.contains(&marker) {
            Ok(format!(
                "{}\n{incoming}\n",
                base.trim_end(),
            ))
        } else {
            Ok(format!(
                "{}\n\n{marker}\n{incoming}\n",
                base.trim_end(),
            ))
        }
    }
}
