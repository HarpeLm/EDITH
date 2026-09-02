pub mod builtin;
pub mod web;

use crate::brain::ToolDef;
use crate::security::Permission;
use anyhow::Result;
use serde_json::Value;
use std::collections::BTreeMap;
use std::sync::Arc;

/// Un outil : spécialisé, validé, limité. Jamais de shell générique.
#[async_trait::async_trait]
pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn level(&self) -> Permission;
    fn parameters(&self) -> Value;
    async fn execute(&self, args: &Value) -> Result<String>;
}

/// Registre des outils disponibles pour le LLM.
#[derive(Default, Clone)]
pub struct Registry {
    tools: BTreeMap<String, Arc<dyn Tool>>,
}

impl Registry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, tool: Arc<dyn Tool>) {
        self.tools.insert(tool.name().to_string(), tool);
    }

    pub fn get(&self, name: &str) -> Option<Arc<dyn Tool>> {
        self.tools.get(name).cloned()
    }

    pub fn definitions(&self) -> Vec<ToolDef> {
        self.tools
            .values()
            .map(|t| ToolDef {
                name: t.name().to_string(),
                description: t.description().to_string(),
                parameters: t.parameters(),
            })
            .collect()
    }
}
