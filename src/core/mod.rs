pub mod conversation;

use crate::brain::{Brain, BrainReply};
use crate::security::Permission;
use crate::tools::Registry;
use anyhow::Result;
use conversation::{Conversation, Message, ToolCallReq};

/// La boucle principale d'Edith : LLM -> outils (avec permissions) -> réponse.
pub struct Assistant {
    brain: Box<dyn Brain>,
    tools: Registry,
    conversation: Conversation,
}

/// Nombre maximum d'allers-retours LLM/outils pour une seule demande.
const MAX_TOOL_ROUNDS: usize = 5;

impl Assistant {
    pub fn new(brain: Box<dyn Brain>, tools: Registry) -> Self {
        Self {
            brain,
            tools,
            conversation: Conversation::new(),
        }
    }

    pub async fn handle(&mut self, input: &str) -> Result<String> {
        self.conversation.push(Message::User {
            content: input.to_string(),
        });

        for _ in 0..MAX_TOOL_ROUNDS {
            let reply: BrainReply = self
                .brain
                .chat(self.conversation.context(), &self.tools.definitions())
                .await?;

            match reply {
                BrainReply::Answer(text) => {
                    self.conversation.push(Message::Assistant {
                        content: text.clone(),
                    });
                    return Ok(text);
                }
                BrainReply::ToolCalls(calls) => {
                    self.conversation.push(Message::ToolCall {
                        calls: calls.clone(),
                    });
                    for call in calls {
                        let result = self.run_tool(call).await;
                        self.conversation.push(result);
                    }
                }
            }
        }

        Ok("J'ai atteint la limite d'actions pour cette demande, je m'arrête là.".to_string())
    }

    /// Exécute un outil après vérification des permissions.
    /// C'est ici — et uniquement ici — qu'une intention du LLM devient une action.
    async fn run_tool(&self, call: ToolCallReq) -> Message {
        let result = match self.tools.get(&call.name) {
            None => format!("Erreur : l'outil '{}' n'existe pas.", call.name),
            Some(tool) => {
                if let Err(e) = Permission::check(tool.level()) {
                    tracing::warn!(tool = %call.name, "action refusée : {}", e);
                    format!("Refusé : {}", e)
                } else {
                    match tool.execute(&call.arguments).await {
                        Ok(out) => {
                            tracing::info!(tool = %call.name, "outil exécuté");
                            out
                        }
                        Err(e) => format!("Erreur pendant '{}': {}", call.name, e),
                    }
                }
            }
        };
        Message::ToolResult {
            call_id: call.id,
            content: result,
        }
    }
}
