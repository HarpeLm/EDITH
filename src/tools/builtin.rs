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
                "name": {"type": "string", "description": "Nom de l'application, ex: Safari, Spotify, Terminal"}
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

/// Construit le registre du MVP (section 15 du plan).
pub fn mvp_registry() -> Registry {
    let mut reg = Registry::new();
    reg.register(Arc::new(GetTime));
    reg.register(Arc::new(OpenApplication));
    reg
}
