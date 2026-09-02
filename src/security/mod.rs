use tokio::sync::mpsc;

/// Niveaux de permission du plan (section 9) :
/// 0 = lecture, 1 = réversible, 2 = sensible, 3 = critique.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[allow(dead_code)] // les niveaux s'activent au fil des nouveaux outils
pub enum Permission {
    Read,
    Reversible,
    Sensitive,
    Critical,
}

/// Verdict du garde-fou central.
#[derive(Debug)]
pub enum Verdict {
    /// Exécution directe autorisée.
    Allowed,
    /// Exécution suspendue : accord explicite de l'utilisateur requis.
    /// Le booléen `critical` distingue les niveaux 2 (confirmation) et 3
    /// (confirmation explicite + règles supplémentaires).
    NeedsConfirmation { description: String, critical: bool },
}

/// Demande de confirmation envoyée du cœur vers l'interface (terminal + voix).
#[derive(Debug)]
pub struct ConfirmRequest {
    pub description: String,
    pub critical: bool,
}

/// Passe entre le cœur (qui demande) et la boucle d'interaction (qui répond).
#[derive(Clone)]
pub struct ConfirmationHandle {
    tx: mpsc::UnboundedSender<(ConfirmRequest, mpsc::UnboundedSender<bool>)>,
}

impl ConfirmationHandle {
    /// Demande l'accord de l'utilisateur. Renvoie false en cas d'erreur
    /// de communication (on échoue côté refus : fail-closed).
    pub async fn ask(&self, req: ConfirmRequest) -> bool {
        let (tx, mut rx) = mpsc::unbounded_channel();
        if self.tx.send((req, tx)).is_err() {
            return false;
        }
        rx.recv().await.unwrap_or(false)
    }
}

/// Récepteur côté interface.
pub struct ConfirmationReceiver {
    rx: mpsc::UnboundedReceiver<(ConfirmRequest, mpsc::UnboundedSender<bool>)>,
}

pub fn channel() -> (ConfirmationHandle, ConfirmationReceiver) {
    let (tx, rx) = mpsc::unbounded_channel();
    (ConfirmationHandle { tx }, ConfirmationReceiver { rx })
}

impl ConfirmationReceiver {
    /// Attend la prochaine demande et renvoie (requête, canal de réponse).
    pub async fn next(&mut self) -> Option<(ConfirmRequest, mpsc::UnboundedSender<bool>)> {
        self.rx.recv().await
    }
}

/// Garde-fou central : rien ne s'exécute sans passer par ici.
pub fn check(level: Permission, description: &str) -> Verdict {
    match level {
        Permission::Read | Permission::Reversible => Verdict::Allowed,
        Permission::Sensitive => Verdict::NeedsConfirmation {
            description: description.to_string(),
            critical: false,
        },
        Permission::Critical => Verdict::NeedsConfirmation {
            description: description.to_string(),
            critical: true,
        },
    }
}
