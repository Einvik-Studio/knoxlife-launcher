//! Nettet: manifestet, signaturen og filene i patchen over HTTPS. Adressen til manifestet er bygget inn
//! ([`manifest::ADRESSE`]), og filene ligger ved siden av det. Alt som hentes, sjekkes i kjernen etterpå – signaturen
//! før manifestet leses, og hver fil mot SHA-256 i manifestet.

use knoxlife_kjerne::manifest::{self, Fil, Manifest, STØRSTE_FIL};
use knoxlife_kjerne::okt::Nett;
use knoxlife_kjerne::{Feil, Resultat};
use std::time::Duration;

/// Manifestet og signaturen er små.
const STØRSTE_MANIFEST: u64 = 1024 * 1024;

pub struct Https {
    /// Adressen til `manifest.json`.
    manifest: String,
    agent: ureq::Agent,
}

impl Https {
    /// Nettet med den innebygde adressen, eller `None` i en prøvebygging uten adresse.
    pub fn innebygd() -> Option<Https> {
        Some(Https::ny(manifest::ADRESSE?))
    }

    pub fn ny(manifest: &str) -> Https {
        let agent = ureq::Agent::config_builder()
            .timeout_connect(Some(Duration::from_secs(15)))
            .timeout_global(Some(Duration::from_secs(300)))
            .https_only(true)
            .user_agent(concat!("KnoxLifeLauncher/", env!("CARGO_PKG_VERSION")))
            .build()
            .into();
        Https { manifest: manifest.to_string(), agent }
    }

    /// Mappen manifestet ligger i, med `/` til slutt.
    fn mappe(&self) -> &str {
        match self.manifest.rfind('/') {
            Some(i) => &self.manifest[..=i],
            None => &self.manifest,
        }
    }

    fn hent(&self, adresse: &str, største: u64) -> Resultat<Vec<u8>> {
        let mut svar = self.agent.get(adresse).call().map_err(|e| Feil::Nett(kort(&e.to_string())))?;
        svar.body_mut()
            .with_config()
            .limit(største)
            .read_to_vec()
            .map_err(|e| Feil::Nett(kort(&e.to_string())))
    }
}

impl Nett for Https {
    fn manifest(&self) -> Resultat<(Vec<u8>, Vec<u8>)> {
        let json = self.hent(&self.manifest, STØRSTE_MANIFEST)?;
        let sig = self.hent(&format!("{}manifest.sig", self.mappe()), STØRSTE_MANIFEST)?;
        Ok((json, sig))
    }

    fn fil(&self, _: &Manifest, fil: &Fil) -> Resultat<Vec<u8>> {
        // Filnavnet er sjekket i manifestet (bare bokstaver, tall, «.», «_» og «-»), så det kan stå rett i adressen.
        self.hent(&format!("{}{}", self.mappe(), fil.navn), fil.størrelse.min(STØRSTE_FIL))
    }
}

/// Feilen fra nettet i én kort linje.
fn kort(tekst: &str) -> String {
    let linje = tekst.lines().next().unwrap_or_default();
    linje.chars().take(120).collect()
}

#[cfg(test)]
mod prøver {
    use super::*;

    #[test]
    fn filene_ligger_ved_siden_av_manifestet() {
        let n = Https::ny("https://example.invalid/knoxlife/manifest.json");
        assert_eq!(n.mappe(), "https://example.invalid/knoxlife/");
    }

    #[test]
    fn bare_https() {
        let n = Https::ny("http://example.invalid/manifest.json");
        assert!(matches!(n.manifest(), Err(Feil::Nett(_))));
    }
}
