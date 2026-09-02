use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Un message dans la conversation, indépendant du moteur LLM.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "role", rename_all = "lowercase")]
pub enum Message {
    System { content: String },
    User { content: String },
    Assistant { content: String },
    /// Demande d'outils émise par le LLM (stockée pour le contexte multi-tours).
    ToolCall { calls: Vec<ToolCallReq> },
    /// Résultat renvoyé au LLM après exécution d'un outil.
    ToolResult { call_id: String, content: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallReq {
    pub id: String,
    pub name: String,
    pub arguments: Value,
}

/// Mémoire de conversation (court terme).
#[derive(Debug, Default)]
pub struct Conversation {
    messages: Vec<Message>,
}

/// Fenêtre glissante : on ne renvoie pas tout l'historique au LLM.
const CONTEXT_WINDOW: usize = 20;

impl Conversation {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, msg: Message) {
        self.messages.push(msg);
    }

    pub fn context(&self) -> &[Message] {
        let start = self.messages.len().saturating_sub(CONTEXT_WINDOW);
        &self.messages[start..]
    }

    pub fn reset(&mut self) {
        self.messages.clear();
    }
}
