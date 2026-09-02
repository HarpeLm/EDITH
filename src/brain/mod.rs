pub mod ollama;

use crate::core::conversation::Message;
use crate::core::conversation::ToolCallReq;
use anyhow::Result;
use serde_json::Value;

/// Réponse du cerveau : soit une réponse finale, soit des demandes d'outils.
#[derive(Debug)]
pub enum BrainReply {
    Answer(String),
    ToolCalls(Vec<ToolCallReq>),
}

/// Définition d'un outil telle que le LLM la voit.
#[derive(Debug, Clone)]
pub struct ToolDef {
    pub name: String,
    pub description: String,
    /// Schéma JSON des paramètres.
    pub parameters: Value,
}

/// Abstraction du moteur LLM : remplaçable sans toucher au reste du projet.
#[async_trait::async_trait]
pub trait Brain: Send + Sync {
    async fn chat(&self, context: &[Message], tools: &[ToolDef]) -> Result<BrainReply>;
}
