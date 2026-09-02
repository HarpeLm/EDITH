use anyhow::{Context, Result};
use std::path::Path;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, Command};
use tokio::time::timeout;

/// Écouteur de mot d'activation : pilote le processus Vosk (`wakeword/listen.py`)
/// et traduit ses événements stdout en signaux pour Edith.
/// Le traitement audio reste 100 % local.
pub struct WakeWord {
    child: Child,
    lines: BufReader<tokio::process::ChildStdout>,
}

/// Temps laissé à l'utilisateur pour formuler sa demande après « Edith ».
const TRANSCRIPT_TIMEOUT: Duration = Duration::from_secs(15);

impl WakeWord {
    pub fn spawn(project_root: &Path) -> Result<Self> {
        let python = project_root.join("wakeword/.venv/bin/python");
        let script = project_root.join("wakeword/listen.py");
        let mut child = Command::new(python)
            .arg(script)
            .current_dir(project_root)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null())
            .spawn()
            .context("impossible de lancer l'écoute du wake word (venv wakeword/.venv ?)")?;
        let stdout = child.stdout.take().context("stdout du wake word indisponible")?;
        Ok(Self {
            child,
            lines: BufReader::new(stdout),
        })
    }

    /// Attend que le modèle soit prêt.
    pub async fn ready(&mut self) -> Result<()> {
        let mut line = String::new();
        self.lines.read_line(&mut line).await?;
        anyhow::ensure!(line.trim() == "READY", "wake word: démarrage inattendu: {line}");
        Ok(())
    }

    /// Bloque jusqu'à « Edith », puis renvoie la commande dite juste après
    /// (None si l'utilisateur n'a rien dit : il faudra écouter activement).
    pub async fn wait(&mut self) -> Result<Option<String>> {
        loop {
            let mut line = String::new();
            let n = self.lines.read_line(&mut line).await?;
            anyhow::ensure!(n > 0, "le processus wake word s'est arrêté");
            match line.trim() {
                "WAKE" => continue,
                t if t.starts_with("TRANSCRIPT") => {
                    let said = t.strip_prefix("TRANSCRIPT").unwrap_or("").trim();
                    return Ok((!said.is_empty()).then(|| said.to_string()));
                }
                _ => continue,
            }
        }
    }

    /// Attend une commande après le wake word, avec délai. None = silence.
    pub async fn command_after(&mut self) -> Option<String> {
        match timeout(TRANSCRIPT_TIMEOUT, self.wait_no_wake()).await {
            Ok(Some(cmd)) => Some(cmd),
            _ => None,
        }
    }

    async fn wait_no_wake(&mut self) -> Option<String> {
        loop {
            let mut line = String::new();
            let n = self.lines.read_line(&mut line).await.ok()?;
            if n == 0 {
                return None;
            }
            let t = line.trim();
            if let Some(said) = t.strip_prefix("TRANSCRIPT") {
                let said = said.trim();
                return (!said.is_empty()).then(|| said.to_string());
            }
        }
    }
}

impl Drop for WakeWord {
    fn drop(&mut self) {
        let _ = self.child.start_kill();
    }
}
