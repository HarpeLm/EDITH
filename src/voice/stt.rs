use anyhow::{Context, Result};
use serde_json::Value;
use std::path::PathBuf;
use std::time::Duration;
use tokio::process::{Child, Command};

/// STT : serveur whisper.cpp lancé une fois pour toutes (modèle chargé au
/// démarrage) puis interrogé en HTTP local. Chaque transcription est ainsi
/// quasi instantanée, contrairement à whisper-cli qui recharge le modèle.
pub struct WhisperServer {
    url: String,
    _child: Child,
}

/// Durée d'écoute par prise de parole.
pub const RECORD_DURATION: Duration = Duration::from_secs(7);

impl WhisperServer {
    pub fn start(model: &str, port: u16) -> Result<Self> {
        let child = Command::new("whisper-server")
            .args(["-m", model, "-l", "fr", "--port", &port.to_string()])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .context("whisper-server indisponible (brew install whisper-cpp)")?;
        Ok(Self {
            url: format!("http://127.0.0.1:{port}"),
            _child: child,
        })
    }

    /// Attend que le serveur réponde (chargement du modèle au premier appel).
    pub async fn wait_ready(&self) -> Result<()> {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(120))
            .build()?;
        for _ in 0..60 {
            if http.get(format!("{}/load", self.url)).send().await.is_ok() {
                return Ok(());
            }
            tokio::time::sleep(Duration::from_millis(500)).await;
        }
        anyhow::bail!("le serveur whisper ne répond pas")
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

    /// Transcrit un fichier wav 16 kHz mono via le serveur local.
    pub async fn transcribe_file(&self, wav: &PathBuf) -> Result<String> {
        let bytes = std::fs::read(wav).context("lecture du fichier audio")?;
        let part = reqwest::multipart::Part::bytes(bytes)
            .file_name("input.wav")
            .mime_str("audio/wav")?;
        let form = reqwest::multipart::Form::new().part("file", part);
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(60))
            .build()?;
        let resp: Value = http
            .post(format!("{}/inference", self.url))
            .query(&[("response_format", "json"), ("temperature", "0")])
            .multipart(form)
            .send()
            .await
            .context("serveur whisper injoignable")?
            .json()
            .await
            .context("réponse whisper illisible")?;
        let text = resp["text"]
            .as_str()
            .unwrap_or_default()
            .trim()
            .trim_start_matches(['-', '\u{2013}', '\u{2014}', ' '])
            .to_string();
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

impl Drop for WhisperServer {
    fn drop(&mut self) {
        let _ = self._child.start_kill();
    }
}
