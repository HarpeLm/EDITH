use anyhow::{Context, Result};

/// TTS minimal : la synthèse vocale native de macOS, entièrement locale,
/// pilotée par la commande `say`. Remplaçable plus tard par Piper/Kokoro
/// via le même trait `TextToSpeech`.
pub struct MacSay {
    voice: String,
}

impl MacSay {
    pub fn new(voice: impl Into<String>) -> Self {
        Self {
            voice: voice.into(),
        }
    }

    /// Lit le texte à voix haute (bloque jusqu'à la fin de la lecture).
    pub async fn speak(&self, text: &str) -> Result<()> {
        // On nettoie le texte pour la lecture : pas de code ni de markdown.
        tokio::process::Command::new("say")
            .args(["-v", &self.voice, text])
            .status()
            .await
            .context("commande 'say' indisponible")?;
        Ok(())
    }
}
