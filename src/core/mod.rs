pub mod conversation;

use crate::brain::{Brain, BrainReply};
use crate::security::{self, ConfirmationHandle, Verdict};
use crate::tools::Registry;
use anyhow::Result;
use conversation::{Conversation, Message, ToolCallReq};

/// La boucle principale d'Edith : LLM -> outils (avec permissions) -> réponse.
pub struct Assistant {
    brain: Box<dyn Brain>,
    tools: Registry,
    conversation: Conversation,
    confirm: ConfirmationHandle,
}

/// Nombre maximum d'allers-retours LLM/outils pour une seule demande.
const MAX_TOOL_ROUNDS: usize = 5;

impl Assistant {
    pub fn new(brain: Box<dyn Brain>, tools: Registry, confirm: ConfirmationHandle) -> Self {
        Self {
            brain,
            tools,
            conversation: Conversation::new(),
            confirm,
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
    /// Niveaux 2-3 : l'exécution est suspendue jusqu'à l'accord explicite.
    async fn run_tool(&self, call: ToolCallReq) -> Message {
        let result = match self.tools.get(&call.name) {
            None => format!("Erreur : l'outil '{}' n'existe pas.", call.name),
            Some(tool) => match security::check(tool.level(), &describe(&call)) {
                Verdict::Allowed => self.execute(tool.as_ref(), &call).await,
                Verdict::NeedsConfirmation { description, critical } => {
                    let level = if critical { "CRITIQUE" } else { "sensible" };
                    tracing::info!(tool = %call.name, "action {level} : confirmation demandée");
                    if self
                        .confirm
                        .ask(security::ConfirmRequest { description, critical })
                        .await
                    {
                        self.execute(tool.as_ref(), &call).await
                    } else {
                        "Refusé par l'utilisateur.".to_string()
                    }
                }
            },
        };
        Message::ToolResult {
            call_id: call.id,
            content: result,
        }
    }

    async fn execute(
        &self,
        tool: &dyn crate::tools::Tool,
        call: &ToolCallReq,
    ) -> String {
        match tool.execute(&call.arguments).await {
            Ok(out) => {
                tracing::info!(tool = %call.name, "outil exécuté");
                out
            }
            Err(e) => format!("Erreur pendant '{}': {}", call.name, e),
        }
    }
}

/// Description humaine de l'action demandée, pour la question de confirmation.
fn describe(call: &ToolCallReq) -> String {
    let args: Vec<String> = call
        .arguments
        .as_object()
        .map(|o| {
            o.iter()
                .map(|(k, v)| format!("{k}: {v}"))
                .collect()
        })
        .unwrap_or_default();
    if args.is_empty() {
        call.name.clone()
    } else {
        format!("{} ({})", call.name, args.join(", "))
    }
}
