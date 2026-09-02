use anyhow::{Context, Result};
use std::path::Path;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, Command};

/// Écouteur de mot d'activation : pilote le processus Vosk (`wakeword/listen.py`)
/// et traduit ses événements stdout en signaux pour Edith.
/// Le traitement audio reste 100 % local.
pub struct WakeWord {
    child: Child,
    lines: BufReader<tokio::process::ChildStdout>,
}

impl WakeWord {
    pub fn spawn(project_root: &Path) -> Result<Self> {
        let python = project_root.join("wakeword/.venv/bin/python");
        let script = project_root.join("wakeword/listen.py");
        let mut child = Command::new(python)
            .arg(script)
            .current_dir(project_root)
            .stdout(std::process::Stdio::piped())
            // Les erreurs Python (traceback, micro indisponible…) restent
            // visibles dans le terminal au lieu d'être avalées.
            .stderr(std::process::Stdio::inherit())
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

}

impl Drop for WakeWord {
    fn drop(&mut self) {
        let _ = self.child.start_kill();
    }
}

/// Écouteur auto-réparant : si le processus Python meurt (bug, micro coupé),
/// on le relance au lieu de perdre l'écoute continue pour de bon.
pub struct WatchedWakeWord {
    root: std::path::PathBuf,
    inner: Option<WakeWord>,
}

impl WatchedWakeWord {
    pub async fn start(root: &Path) -> Result<Self> {
        let mut w = WakeWord::spawn(root)?;
        w.ready().await?;
        Ok(Self { root: root.to_path_buf(), inner: Some(w) })
    }

    /// Comme `WakeWord::wait`, mais relance le processus en cas de crash.
    pub async fn wait(&mut self) -> Result<Option<String>> {
        loop {
            let Some(w) = self.inner.as_mut() else { break };
            match w.wait().await {
                Ok(res) => return Ok(res),
                Err(e) => {
                    tracing::warn!("écoute interrompue ({e:#}), relance…");
                    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                    match WakeWord::spawn(&self.root) {
                        Ok(mut w) => {
                            if w.ready().await.is_ok() {
                                tracing::info!("écoute relancée");
                                self.inner = Some(w);
                            } else {
                                self.inner = None;
                                anyhow::bail!("impossible de relancer l'écoute");
                            }
                        }
                        Err(_) => {
                            self.inner = None;
                            anyhow::bail!("impossible de relancer l'écoute");
                        }
                    }
                }
            }
        }
        anyhow::bail!("écoute indisponible")
    }
}
