//! Couche vocale : traits posés dès maintenant pour que la phase 0 vocale
//! s'intercale sans toucher au cœur. Implémentations prévues :
//! wakeword = openWakeWord, STT = whisper.cpp, TTS = Piper/Kokoro.

/// Sortie du détecteur de mot d'activation.
pub struct WakeEvent;

/// Capture micro -> texte.
#[async_trait::async_trait]
pub trait SpeechToText: Send + Sync {
    async fn transcribe(&self, samples: &[f32]) -> anyhow::Result<String>;
}

/// Texte -> audio joué sur le haut-parleur.
#[async_trait::async_trait]
pub trait TextToSpeech: Send + Sync {
    async fn speak(&self, text: &str) -> anyhow::Result<()>;
}
