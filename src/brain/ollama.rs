use super::{Brain, BrainReply, ToolDef};
use crate::core::conversation::Message;
use anyhow::{Context, Result};
use serde_json::{json, Value};

/// Implémentation du cerveau via l'API locale d'Ollama (aucune donnée ne sort du réseau).
pub struct Ollama {
    url: String,
    model: String,
    http: reqwest::Client,
}

/// Le prompt système : Edith, en français, outils seulement, pas de commandes directes.
const SYSTEM_PROMPT: &str = "Tu es Edith, l'assistante vocale personnelle de l'utilisateur. \
Réponds en français, de façon concise et naturelle (la réponse sera lue à voix haute). \
Quand une demande correspond à un outil disponible, appelle cet outil plutôt que de deviner la réponse.";

impl Ollama {
    pub fn new(url: String, model: String) -> Self {
        Self {
            url,
            model,
            http: reqwest::Client::new(),
        }
    }

    fn to_ollama_message(msg: &Message) -> Value {
        match msg {
            Message::System { content } => json!({"role": "system", "content": content}),
            Message::User { content } => json!({"role": "user", "content": content}),
            Message::Assistant { content } => json!({"role": "assistant", "content": content}),
            Message::ToolCall { calls } => json!({
                "role": "assistant",
                "tool_calls": calls.iter().map(|c| json!({
                    "function": {
                        "name": c.name,
                        "arguments": c.arguments,
                    }
                })).collect::<Vec<_>>(),
            }),
            Message::ToolResult { call_id, content } => json!({
                "role": "tool",
                "tool_name": call_id,
                "content": content,
            }),
        }
    }
}

#[async_trait::async_trait]
impl Brain for Ollama {
    async fn chat(&self, context: &[Message], tools: &[ToolDef]) -> Result<BrainReply> {
        let mut messages = vec![json!({"role": "system", "content": SYSTEM_PROMPT})];
        messages.extend(context.iter().map(Self::to_ollama_message));

        let body = json!({
            "model": self.model,
            "messages": messages,
            "stream": false,
            "tools": tools.iter().map(|t| json!({
                "type": "function",
                "function": {
                    "name": t.name,
                    "description": t.description,
                    "parameters": t.parameters,
                }
            })).collect::<Vec<_>>(),
        });

        let resp: Value = self
            .http
            .post(format!("{}/api/chat", self.url))
            .json(&body)
            .send()
            .await
            .context("Ollama injoignable — est-il lancé ?")?
            .json()
            .await
            .context("réponse Ollama illisible")?;

        let msg = &resp["message"];
        if let Some(calls) = msg["tool_calls"].as_array() {
            let parsed: Vec<_> = calls
                .iter()
                .enumerate()
                .map(|(i, c)| {
                    let func = &c["function"];
                    crate::core::conversation::ToolCallReq {
                        id: format!("call_{}", i),
                        name: func["name"].as_str().unwrap_or_default().to_string(),
                        arguments: func["arguments"].clone(),
                    }
                })
                .collect();
            if !parsed.is_empty() {
                return Ok(BrainReply::ToolCalls(parsed));
            }
        }

        let text = msg["content"]
            .as_str()
            .unwrap_or("Je n'ai pas réussi à formuler une réponse.")
            .to_string();
        Ok(BrainReply::Answer(text))
    }
}
