use anyhow::{Context, Result};
use std::path::PathBuf;
use std::time::Duration;

/// STT minimal : whisper.cpp en ligne de commande, modèle local.
/// La crate `whisper-rs` (liaison directe) viendra plus tard si besoin de latence.
pub struct WhisperCli {
    model: PathBuf,
}

/// Durée d'écoute par appui sur Entrée (MVP simple, pas de détection de silence).
pub const RECORD_DURATION: Duration = Duration::from_secs(7);

impl WhisperCli {
    pub fn new(model: impl Into<PathBuf>) -> Self {
        Self { model: model.into() }
    }

    /// Enregistre le micro avec sox pendant RECORD_DURATION.
    pub async fn record(&self, out: &PathBuf) -> Result<()> {
        tokio::process::Command::new("rec")
            .args([
                "-q",
                out.to_str().context("chemin audio invalide")?,
                "rate",
                "16000",
                "channels",
                "1",
                "trim",
                "0",
                &RECORD_DURATION.as_secs().to_string(),
            ])
            .status()
            .await
            .context("commande 'rec' (sox) indisponible")?;
        Ok(())
    }

    /// Transcrit un fichier wav 16 kHz mono en texte.
    pub async fn transcribe_file(&self, wav: &PathBuf) -> Result<String> {
        let out = tokio::process::Command::new("whisper-cli")
            .args([
                "-m",
                self.model.to_str().context("chemin modèle invalide")?,
                "-l",
                "fr",
                "-nt",
                "-f",
                wav.to_str().context("chemin audio invalide")?,
            ])
            .output()
            .await
            .context("whisper-cli indisponible")?;
        anyhow::ensure!(out.status.success(), "whisper-cli a échoué: {}", String::from_utf8_lossy(&out.stderr));
        let text = String::from_utf8_lossy(&out.stdout).trim().to_string();
        Ok(text)
    }

    /// Écoute et transcrit : enregistre puis renvoie le texte.
    pub async fn listen(&self) -> Result<String> {
        let wav = std::env::temp_dir().join("edith_input.wav");
        self.record(&wav).await?;
        let text = self.transcribe_file(&wav).await?;
        let _ = std::fs::remove_file(&wav);
        Ok(text)
    }
}
