use anyhow::{bail, Result};

/// Niveaux de permission du plan (section 9) :
/// 0 = lecture, 1 = réversible, 2 = sensible, 3 = critique.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Permission {
    Read,
    Reversible,
    Sensitive,
    Critical,
}

impl Permission {
    /// Garde-fou central : rien ne s'exécute sans passer par ici.
    /// MVP : lecture et actions réversibles automatiques. Le reste est refusé
    /// en attendant le mécanisme de confirmation (interface tactile + voix).
    pub fn check(level: Permission) -> Result<()> {
        match level {
            Permission::Read | Permission::Reversible => Ok(()),
            Permission::Sensitive => bail!("action sensible, confirmation requise (pas encore implémentée)"),
            Permission::Critical => bail!("action critique, confirmation explicite requise"),
        }
    }
}
