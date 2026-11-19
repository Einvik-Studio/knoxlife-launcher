//! Nettet: manifestet, signaturen og filene i patchen over HTTPS. Adressen til manifestet er bygget inn
//! ([`manifest::ADRESSE`]), og filene ligger ved siden av det. Alt som hentes, sjekkes i kjernen etterpå – signaturen
//! før manifestet leses, og hver fil mot SHA-256 i manifestet.

use knoxlife_kjerne::kino::{self, Ønske};
use knoxlife_kjerne::ko;
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

/// Filmene til kinoen (J11): strømmes rett til filen fra utgivelsene i dette repoet ([`kino::KILDE`]), bare over HTTPS.
/// Kjernen sjekker størrelsen og SHA-256 etterpå.
pub struct Filmnett {
    agent: ureq::Agent,
}

impl Filmnett {
    pub fn ny() -> Filmnett {
        let agent = ureq::Agent::config_builder()
            .timeout_connect(Some(Duration::from_secs(15)))
            .timeout_global(Some(Duration::from_secs(3 * 60 * 60)))
            .https_only(true)
            .user_agent(concat!("KnoxLifeLauncher/", env!("CARGO_PKG_VERSION")))
            .build()
            .into();
        Filmnett { agent }
    }
}

impl kino::Henter for Filmnett {
    fn hent_til(&self, ønske: &Ønske, ut: &mut dyn std::io::Write) -> Resultat<()> {
        if !ønske.url.starts_with(kino::KILDE) {
            return Err(Feil::Nett("not a Knox Life film".into()));
        }
        let mut svar = self.agent.get(&ønske.url).call().map_err(|e| Feil::Nett(kort(&e.to_string())))?;
        let mut leser = svar.body_mut().with_config().limit(ønske.bytes + 1).reader();
        std::io::copy(&mut leser, ut).map_err(|e| Feil::Nett(kort(&e.to_string())))?;
        Ok(())
    }
}

/// Serverens status og køen (`launcher.md` 3.8 punkt 6): adressen kommer fra det signerte manifestet, og bare HTTPS.
/// Korte spørsmål med kort tidsfrist – svarer ikke serveren, starter «Play» som før.
pub struct Statusnett {
    /// Adressen med `/` til slutt ([`ko::gyldig_adresse`]).
    adresse: String,
    agent: ureq::Agent,
}

impl Statusnett {
    pub fn ny(adresse: &str) -> Statusnett {
        let agent = ureq::Agent::config_builder()
            .timeout_connect(Some(Duration::from_secs(5)))
            .timeout_global(Some(Duration::from_secs(10)))
            .https_only(true)
            .user_agent(concat!("KnoxLifeLauncher/", env!("CARGO_PKG_VERSION")))
            .build()
            .into();
        Statusnett { adresse: adresse.to_string(), agent }
    }

    /// `GET status`: tallene, uten noe om spilleren.
    pub fn status(&self) -> Resultat<Vec<u8>> {
        let mut svar = self
            .agent
            .get(format!("{}status", self.adresse))
            .call()
            .map_err(|e| Feil::Nett(kort(&e.to_string())))?;
        les(svar.body_mut())
    }

    /// `POST ko` (stå i køen) eller `POST ko/forlat` («Leave line») med Steam-ID-en.
    pub fn ko(&self, forlat: bool, steam_id: &str) -> Resultat<Vec<u8>> {
        let kropp = ko::kropp(steam_id)?;
        let sti = if forlat { "ko/forlat" } else { "ko" };
        let mut svar = self
            .agent
            .post(format!("{}{sti}", self.adresse))
            .header("content-type", "application/json")
            .send(kropp)
            .map_err(|e| Feil::Nett(kort(&e.to_string())))?;
        les(svar.body_mut())
    }
}

fn les(kropp: &mut ureq::Body) -> Resultat<Vec<u8>> {
    kropp.with_config().limit(ko::STØRSTE_SVAR).read_to_vec().map_err(|e| Feil::Nett(kort(&e.to_string())))
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
    fn statusen_og_køen_bare_over_https() {
        let n = Statusnett::ny("http://example.invalid/");
        assert!(matches!(n.status(), Err(Feil::Nett(_))));
        assert!(matches!(n.ko(false, "76561198000000001"), Err(Feil::Nett(_))));
        // Uten en Steam-ID sendes ingenting.
        assert!(matches!(Statusnett::ny("https://example.invalid/").ko(false, ""), Err(Feil::Nett(_))));
    }

    #[test]
    fn bare_https() {
        let n = Https::ny("http://example.invalid/manifest.json");
        assert!(matches!(n.manifest(), Err(Feil::Nett(_))));
    }
}
