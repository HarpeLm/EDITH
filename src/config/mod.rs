use anyhow::{Context, Result};
use serde::Deserialize;
use std::path::Path;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub brain: BrainConfig,
    pub voice: Option<VoiceConfig>,
    #[serde(default)]
    pub tools: WebToolsConfig,
}

#[derive(Debug, Deserialize, Default)]
pub struct WebToolsConfig {
    /// Ville par défaut pour get_weather.
    #[serde(default = "default_location")]
    pub default_location: String,
    /// Instance SearXNG pour search_web ; vide = outil désactivé.
    #[serde(default)]
    pub searxng_url: String,
}

fn default_location() -> String {
    "Paris".to_string()
}

#[derive(Debug, Deserialize)]
pub struct BrainConfig {
    pub ollama_url: String,
    pub model: String,
}

#[derive(Debug, Deserialize)]
pub struct VoiceConfig {
    pub tts_voice: String,
    pub whisper_model: String,
    /// Écoute continue du wake word « Edith » (nécessite wakeword/.venv).
    #[serde(default)]
    pub wake_word: bool,
}

impl Config {
    pub fn load(path: &Path) -> Result<Self> {
        let raw = std::fs::read_to_string(path)
            .with_context(|| format!("lecture de la config {}", path.display()))?;
        toml::from_str(&raw).context("parsing de config.toml")
    }
}
