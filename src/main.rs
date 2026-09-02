mod brain;
mod config;
mod core;
mod security;
mod tools;
mod voice;

use anyhow::{Context, Result};
use brain::ollama::Ollama;
use core::Assistant;
use tokio::sync::mpsc;
use voice::{MacSay, WatchedWakeWord, WhisperServer};

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
    let mut registry = tools::builtin::mvp_registry();
    tools::web::register_web_tools(&mut registry, &cfg.tools);
    let mut edith = Assistant::new(brain, registry);

    let (say, stt) = match &cfg.voice {
        Some(v) => {
            let server = WhisperServer::start(&v.whisper_model, 9000)?;
            server.wait_ready().await?;
            (
                Some(MacSay::new(v.tts_voice.clone())),
                Some(server),
            )
        }
        None => (None, None),
    };

    // Clavier en tâche de fond : on peut taper même quand le wake word écoute.
    let (key_tx, mut key_rx) = mpsc::unbounded_channel::<String>();
    tokio::task::spawn_blocking(move || loop {
        let mut line = String::new();
        if std::io::stdin().read_line(&mut line).map(|n| n == 0).unwrap_or(true) {
            break;
        }
        if key_tx.send(line.trim().to_string()).is_err() {
            break;
        }
    });

    let mut wake = match &cfg.voice {
        Some(v) if v.wake_word => {
            let root = std::env::current_dir()?;
            let w = WatchedWakeWord::start(&root).await?;
            println!("🔊 Écoute continue activée — dis « Edith » (ou tape ton message, 'exit' pour quitter)");
            Some(w)
        }
        _ => {
            println!("Edith prête. Entrée = parler au micro · ou tape ton message · 'exit' pour quitter");
            None
        }
    };

    loop {
        tokio::select! {
            // Mot d'activation entendu par le processus Vosk.
            heard = async {
                match wake.as_mut() {
                    Some(w) => w.wait().await,
                    None => std::future::pending().await,
                }
            } => {
                match heard {
                    Ok(maybe_cmd) => {
                        say_oui(&say).await;
                        let cmd = match maybe_cmd {
                            Some(c) => c,
                            None => {
                                // Rien dit après « Edith » : on écoute activement au micro.
                                let Some(stt) = &stt else { continue };
                                println!("🎙  j'écoute…");
                                match stt.listen().await {
                                    Ok(t) if !t.is_empty() => t,
                                    _ => { println!("(je n'ai rien entendu)"); continue; }
                                }
                            }
                        };
                        println!("toi (voix) > {cmd}");
                        reply(&mut edith, &say, &cmd).await;
                    }
                    Err(e) => {
                        tracing::error!("wake word : {e:#}");
                        wake = None; // on continue en mode clavier/micro manuel
                    }
                }
            }
            // Saisie clavier.
            Some(typed) = key_rx.recv() => match typed.as_str() {
                "" => {
                    let Some(stt) = &stt else { continue };
                    println!("🎙  j'écoute…");
                    match stt.listen().await {
                        Ok(t) if !t.is_empty() => {
                            println!("toi (voix) > {t}");
                            reply(&mut edith, &say, &t).await;
                        }
                        _ => println!("(je n'ai rien entendu)"),
                    }
                }
                "exit" => break,
                _ => reply(&mut edith, &say, &typed).await,
            },
        }
    }
    Ok(())
}

async fn say_oui(say: &Option<MacSay>) {
    if let Some(say) = say {
        let _ = say.speak("Oui ?").await;
    }
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
