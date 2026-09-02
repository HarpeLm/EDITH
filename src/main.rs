mod brain;
mod config;
mod core;
mod security;
mod tools;
mod voice;

use anyhow::{Context, Result};
use brain::ollama::Ollama;
use core::Assistant;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info".into()),
        )
        .init();

    let cfg = config::Config::load(std::path::Path::new("config.toml"))
        .context("config.toml introuvable à la racine du projet")?;

    let brain = Box::new(Ollama::new(cfg.brain.ollama_url.clone(), cfg.brain.model.clone()));
    let mut edith = Assistant::new(brain, tools::builtin::mvp_registry());

    println!("Edith prête. (tape 'exit' pour quitter, 'reset' pour oublier la conversation)");
    loop {
        let mut line = String::new();
        print!("toi > ");
        use std::io::Write;
        std::io::stdout().flush()?;
        std::io::stdin().read_line(&mut line)?;
        let input = line.trim();
        match input {
            "" => continue,
            "exit" => break,
            "reset" => {
                edith = Assistant::new(
                    Box::new(Ollama::new(cfg.brain.ollama_url.clone(), cfg.brain.model.clone())),
                    tools::builtin::mvp_registry(),
                );
                println!("Edith a oublié la conversation.");
                continue;
            }
            _ => {}
        }
        let reply = edith.handle(input).await?;
        println!("edith > {reply}");
    }
    Ok(())
}
