use super::{Tool, Registry};
use crate::security::Permission;
use anyhow::Result;
use serde_json::{json, Value};
use std::sync::Arc;

/// Outils réseau : météo Open-Meteo (ouvert, sans clé) et recherche web
/// SearXNG (instance auto-hébergée). Ces deux exceptions au « local par
/// défaut » sont listées dans la section 4 du plan.

const HTTP_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(15);

pub struct GetWeather {
    /// Ville par défaut quand la demande n'en précise pas.
    default_location: String,
}

#[async_trait::async_trait]
impl Tool for GetWeather {
    fn name(&self) -> &str {
        "get_weather"
    }
    fn description(&self) -> &str {
        "Donne la météo actuelle d'une ville (température, ressenti, vent, temps général)."
    }
    fn level(&self) -> Permission {
        Permission::Read
    }
    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "location": {"type": "string", "description": "Ville, ex: Paris, Lyon"}
            }
        })
    }
    async fn execute(&self, args: &Value) -> Result<String> {
        let location = args["location"]
            .as_str()
            .filter(|s| !s.is_empty())
            .unwrap_or(&self.default_location);

        let http = reqwest::Client::builder().timeout(HTTP_TIMEOUT).build()?;

        // 1. Géocodage : ville -> coordonnées (API ouverte Open-Meteo).
        let geo: Value = http
            .get("https://geocoding-api.open-meteo.com/v1/search")
            .query(&[("name", location), ("count", "1"), ("language", "fr")])
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        let place = &geo["results"][0];
        anyhow::ensure!(!place.is_null(), "ville « {location} » introuvable");

        // 2. Météo courante.
        let wx: Value = http
            .get("https://api.open-meteo.com/v1/forecast")
            .query(&[
                ("latitude", place["latitude"].to_string()),
                ("longitude", place["longitude"].to_string()),
                ("current", "temperature_2m,apparent_temperature,weather_code,wind_speed_10m".to_string()),
            ])
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;

        let cur = &wx["current"];
        Ok(format!(
            "{} : {}, température {} °C (ressenti {} °C), vent {} km/h.",
            place["name"].as_str().unwrap_or(location),
            weather_description(cur["weather_code"].as_i64().unwrap_or(0)),
            cur["temperature_2m"].as_f64().unwrap_or_default(),
            cur["apparent_temperature"].as_f64().unwrap_or_default(),
            cur["wind_speed_10m"].as_f64().unwrap_or_default(),
        ))
    }
}

/// Codes WMO -> description française.
fn weather_description(code: i64) -> &'static str {
    match code {
        0 => "ciel dégagé",
        1 | 2 => "peu nuageux",
        3 => "nuageux",
        45 | 48 => "brouillard",
        51..=57 => "bruine",
        61..=67 => "pluie",
        71..=77 => "neige",
        80..=82 => "averses",
        85 | 86 => "averses de neige",
        95..=99 => "orage",
        _ => "conditions inconnues",
    }
}

pub struct SearchWeb {
    /// URL de l'instance SearXNG (auto-hébergée de préférence).
    base_url: String,
}

#[async_trait::async_trait]
impl Tool for SearchWeb {
    fn name(&self) -> &str {
        "search_web"
    }
    fn description(&self) -> &str {
        "Recherche sur le web et renvoie les principaux résultats (titre, URL, extrait)."
    }
    fn level(&self) -> Permission {
        Permission::Read
    }
    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "query": {"type": "string", "description": "Requête de recherche"}
            },
            "required": ["query"]
        })
    }
    async fn execute(&self, args: &Value) -> Result<String> {
        let query = args["query"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("paramètre 'query' manquant"))?;
        let http = reqwest::Client::builder()
            .timeout(HTTP_TIMEOUT)
            .user_agent("edith-assistant/0.1")
            .build()?;
        let resp: Value = http
            .get(format!("{}/search", self.base_url.trim_end_matches('/')))
            .query(&[("q", query), ("format", "json")])
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;

        let results = resp["results"]
            .as_array()
            .map(|a| a.as_slice())
            .unwrap_or(&[]);
        anyhow::ensure!(!results.is_empty(), "aucun résultat pour « {query} »");

        // 5 premiers résultats, format compact pour le contexte LLM.
        let lines: Vec<String> = results
            .iter()
            .take(5)
            .map(|r| {
                format!(
                    "- {} — {}",
                    r["title"].as_str().unwrap_or("(sans titre)"),
                    r["content"].as_str().unwrap_or("").chars().take(200).collect::<String>()
                )
            })
            .collect();
        Ok(lines.join("\n"))
    }
}

impl GetWeather {
    pub fn new(default_location: impl Into<String>) -> Self {
        Self {
            default_location: default_location.into(),
        }
    }
}

impl SearchWeb {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
        }
    }
}

/// Enregistre les outils réseau si leurs paramètres sont configurés.
pub fn register_web_tools(reg: &mut Registry, cfg: &crate::config::WebToolsConfig) {
    reg.register(Arc::new(GetWeather::new(cfg.default_location.clone())));
    if !cfg.searxng_url.is_empty() {
        reg.register(Arc::new(SearchWeb::new(cfg.searxng_url.clone())));
    }
}
