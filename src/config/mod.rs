use anyhow::{Context, Result};
use serde::Deserialize;
use std::path::Path;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub brain: BrainConfig,
}

#[derive(Debug, Deserialize)]
pub struct BrainConfig {
    pub ollama_url: String,
    pub model: String,
}

impl Config {
    pub fn load(path: &Path) -> Result<Self> {
        let raw = std::fs::read_to_string(path)
            .with_context(|| format!("lecture de la config {}", path.display()))?;
        toml::from_str(&raw).context("parsing de config.toml")
    }
}
