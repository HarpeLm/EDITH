mod brain;
mod config;
mod core;
mod security;
mod tools;
mod voice;

use anyhow::{Context, Result};
use brain::ollama::Ollama;
use core::Assistant;
use voice::{MacSay, WhisperCli};

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

    // Voix optionnelle : tout marche aussi sans elle (mode texte).
    let (say, stt) = match &cfg.voice {
        Some(v) => (
            Some(MacSay::new(v.tts_voice.clone())),
            Some(WhisperCli::new(v.whisper_model.clone())),
        ),
        None => (None, None),
    };

    println!("Edith prête. Entrée = parler au micro · ou tape ton message · 'exit' pour quitter");
    loop {
        print!("toi > ");
        use std::io::Write;
        std::io::stdout().flush()?;
        let mut line = String::new();
        std::io::stdin().read_line(&mut line)?;
        let typed = line.trim().to_string();
        match typed.as_str() {
            "exit" => break,
            "" => {
                // Ligne vide = on écoute le micro.
                let Some(stt) = &stt else { continue };
                println!("🎙  j'écoute…");
                let heard = match stt.listen().await {
                    Ok(t) if !t.is_empty() => t,
                    Ok(_) => {
                        println!("(je n'ai rien entendu)");
                        continue;
                    }
                    Err(e) => {
                        tracing::error!("écoute impossible : {e:#}");
                        continue;
                    }
                };
                println!("toi (voix) > {heard}");
                reply(&mut edith, &say, &heard).await;
            }
            _ => reply(&mut edith, &say, &typed).await,
        }
    }
    Ok(())
}

async fn reply(edith: &mut Assistant, say: &Option<MacSay>, input: &str) {
    match edith.handle(input).await {
        Ok(answer) => {
            println!("edith > {answer}");
            if let Some(say) = say {
                if let Err(e) = say.speak(&answer).await {
                    tracing::error!("lecture vocale impossible : {e:#}");
                }
            }
        }
        Err(e) => {
            tracing::error!("erreur : {e:#}");
            println!("edith > J'ai eu un problème, réessaie.");
        }
    }
}
