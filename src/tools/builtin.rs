use super::{Tool, Registry};
use crate::security::Permission;
use anyhow::Result;
use serde_json::{json, Value};
use std::sync::Arc;

/// Outils du MVP : lecture du temps et ouverture d'application.

pub struct GetTime;

#[async_trait::async_trait]
impl Tool for GetTime {
    fn name(&self) -> &str {
        "get_time"
    }
    fn description(&self) -> &str {
        "Donne la date et l'heure actuelles de l'utilisateur."
    }
    fn level(&self) -> Permission {
        Permission::Read
    }
    fn parameters(&self) -> Value {
        json!({"type": "object", "properties": {}})
    }
    async fn execute(&self, _args: &Value) -> Result<String> {
        let now = chrono::Local::now();
        Ok(now.format("%A %d %B %Y, %H:%M").to_string())
    }
}

pub struct OpenApplication;

#[async_trait::async_trait]
impl Tool for OpenApplication {
    fn name(&self) -> &str {
        "open_application"
    }
    fn description(&self) -> &str {
        "Ouvre une application sur l'ordinateur de l'utilisateur."
    }
    fn level(&self) -> Permission {
        Permission::Reversible
    }
    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "name": {"type": "string", "description": "Nom de l'application, ex: Safari, Musique, Terminal"}
            },
            "required": ["name"]
        })
    }
    async fn execute(&self, args: &Value) -> Result<String> {
        let name = args["name"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("paramètre 'name' manquant"))?;
        // `open -a` sur macOS : échoue proprement si l'application n'existe pas.
        let status = tokio::process::Command::new("open")
            .args(["-a", name])
            .status()
            .await?;
        if status.success() {
            Ok(format!("Application {} ouverte.", name))
        } else {
            anyhow::bail!("impossible d'ouvrir '{}'", name)
        }
    }
}

pub struct SetVolume;

#[async_trait::async_trait]
impl Tool for SetVolume {
    fn name(&self) -> &str {
        "set_volume"
    }
    fn description(&self) -> &str {
        "Règle le volume sonore de l'ordinateur de 0 (muet) à 100 (maximum)."
    }
    fn level(&self) -> Permission {
        Permission::Reversible
    }
    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "level": {"type": "integer", "minimum": 0, "maximum": 100,
                           "description": "Niveau de volume en pourcentage"}
            },
            "required": ["level"]
        })
    }
    async fn execute(&self, args: &Value) -> Result<String> {
        let level = args["level"]
            .as_i64()
            .ok_or_else(|| anyhow::anyhow!("paramètre 'level' manquant"))?;
        anyhow::ensure!((0..=100).contains(&level), "le niveau doit être entre 0 et 100");
        run_osascript(&format!("set volume output volume {level}")).await?;
        Ok(format!("Volume réglé à {level} %."))
    }
}

pub struct PlayMusic;

#[async_trait::async_trait]
impl Tool for PlayMusic {
    fn name(&self) -> &str {
        "play_music"
    }
    fn description(&self) -> &str {
        "Contrôle Apple Music (l'app Musique). Sans query : lecture/pause. Avec query : cherche et joue. Actions disponibles: play (défaut), pause, next, previous."
    }
    fn level(&self) -> Permission {
        Permission::Reversible
    }
    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "query": {"type": "string", "description": "Titre, artiste ou album à chercher dans la bibliothèque Apple Music. Omis pour simplement lecture/pause."},
                "action": {"type": "string", "enum": ["play", "pause", "next", "previous"],
                            "description": "Action de contrôle, défaut: play"}
            }
        })
    }
    async fn execute(&self, args: &Value) -> Result<String> {
        let action = args["action"].as_str().unwrap_or("play");
        let query = args["query"].as_str().filter(|q| !q.is_empty());

        let script = match (action, query) {
            ("pause", _) => r#"tell application "Music" to pause"#.to_string(),
            ("next", _) => r#"tell application "Music" to next track"#.to_string(),
            ("previous", _) => r#"tell application "Music" to previous track"#.to_string(),
            (_, Some(q)) => format!(
                r#"tell application "Music"
                    set results to (search playlist "Library" for "{}")
                    if length of results > 0 then
                        play item 1 of results
                        return name of item 1 of results
                    else
                        return ""
                    end if
                end tell"#,
                q.replace('"', "")
            ),
            (_, None) => r#"tell application "Music" to playpause"#.to_string(),
        };
        let out = run_osascript(&script).await?;
        match (action, query) {
            (_, Some(q)) if out.is_empty() => Ok(format!("Rien trouvé pour « {q} » dans la bibliothèque.")),
            (_, Some(_)) => Ok(format!("Lecture : {out}")),
            ("pause", _) => Ok("Musique en pause.".into()),
            ("next", _) => Ok("Piste suivante.".into()),
            ("previous", _) => Ok("Piste précédente.".into()),
            _ => Ok("Lecture/pause basculé.".into()),
        }
    }
}

/// Exécute un AppleScript et renvoie sa sortie texte (trimée).
async fn run_osascript(script: &str) -> Result<String> {
    let out = tokio::process::Command::new("osascript")
        .arg("-e")
        .arg(script)
        .output()
        .await?;
    anyhow::ensure!(
        out.status.success(),
        "osascript a échoué: {}",
        String::from_utf8_lossy(&out.stderr).trim()
    );
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

/// Construit le registre du MVP (section 15 du plan).
pub fn mvp_registry() -> Registry {
    let mut reg = Registry::new();
    reg.register(Arc::new(GetTime));
    reg.register(Arc::new(OpenApplication));
    reg.register(Arc::new(SetVolume));
    reg.register(Arc::new(PlayMusic));
    reg
}
