//! Manifestet: hva patchen består av og hva den er laget for.
//!
//! `manifest.json` signeres med ed25519 når GitHub har bygget patchen og eierne har godkjent utgivelsen.
//! `manifest.sig` er signaturen i hex. Launcheren sjekker signaturen mot den offentlige nøkkelen som er bygget inn,
//! **før** ett tegn i manifestet leses – et manifest med feil signatur blir aldri tolket.
//!
//! Byggenummeret går bare oppover: launcheren husker det høyeste den har sett og tar aldri et lavere, så et gammelt,
//! ekte manifest kan ikke mates inn for å få tilbake en patch med en kjent feil.

use crate::feil::{Feil, Resultat};
use crate::patch::AGENT_FIL;
use ed25519_dalek::{Signature, VerifyingKey};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::path::Path;

/// Formatet launcheren forstår. Et nytt format krever en ny launcher.
pub const FORMAT: u32 = 1;
/// Største fil i patchen vi godtar. Agenten er noen få MB.
pub const STØRSTE_FIL: u64 = 200 * 1024 * 1024;
const SIST_BYGG_FIL: &str = "sist-bygg.txt";

/// Adressen til manifestet, bygget inn når launcheren bygges (`KNOXLIFE_MANIFEST_URL`). Ikke en innstilling.
pub const ADRESSE: Option<&str> = option_env!("KNOXLIFE_MANIFEST_URL");
/// Den offentlige nøkkelen i hex, bygget inn når launcheren bygges (`KNOXLIFE_MANIFEST_NOKKEL`).
const NØKKEL_HEX: Option<&str> = option_env!("KNOXLIFE_MANIFEST_NOKKEL");

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Manifest {
    pub format: u32,
    /// Byggenummeret – går bare oppover.
    pub bygg: u64,
    /// Når patchen ble utgitt (ISO 8601). Bare til visning.
    pub utgitt: String,
    pub spillet: Spillversjon,
    pub filer: Vec<Fil>,
    pub server: Server,
    /// Nødmodus (3.9 punkt 9): spillet startes uten patch.
    #[serde(default, rename = "nodmodus")]
    pub nødmodus: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Spillversjon {
    /// Versjonen slik spillet viser den, for eksempel «42.22.2». Bare til visning.
    pub versjon: String,
    /// SHA-256 av spillets `projectzomboid.jar` – patchen passer bare denne.
    pub jar_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Fil {
    /// Filnavnet, uten mapper.
    pub navn: String,
    pub sha256: String,
    #[serde(rename = "storrelse")]
    pub størrelse: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Server {
    pub adresse: String,
    pub port: u16,
}

/// Den innebygde nøkkelen, eller `None` når launcheren er bygget uten (en prøvebygging).
pub fn innebygd_nøkkel() -> Option<VerifyingKey> {
    nøkkel_fra_hex(NØKKEL_HEX?)
}

pub fn nøkkel_fra_hex(hex_tekst: &str) -> Option<VerifyingKey> {
    let bytes: [u8; 32] = hex::decode(hex_tekst.trim()).ok()?.try_into().ok()?;
    VerifyingKey::from_bytes(&bytes).ok()
}

fn er_sha256(s: &str) -> bool {
    s.len() == 64 && s.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

/// Et filnavn uten mapper og uten triks: bare bokstaver, tall, `.`, `_` og `-`, og ikke `.` først.
pub fn trygt_filnavn(navn: &str) -> bool {
    !navn.is_empty()
        && navn.len() <= 64
        && !navn.starts_with('.')
        && navn.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'.' || b == b'_' || b == b'-')
}

impl Manifest {
    /// Sjekker signaturen og leser manifestet. Signaturen sjekkes først.
    pub fn les_signert(json: &[u8], sig: &[u8], nøkkel: &VerifyingKey) -> Resultat<Manifest> {
        let sig_hex = std::str::from_utf8(sig).map_err(|_| Feil::Signatur)?.trim();
        let sig_bytes: [u8; 64] = hex::decode(sig_hex).map_err(|_| Feil::Signatur)?.try_into().map_err(|_| Feil::Signatur)?;
        nøkkel.verify_strict(json, &Signature::from_bytes(&sig_bytes)).map_err(|_| Feil::Signatur)?;
        let m: Manifest = serde_json::from_slice(json).map_err(|e| Feil::Manifest(e.to_string()))?;
        m.sjekk()?;
        Ok(m)
    }

    /// Det som må stemme i et manifest selv om signaturen er riktig – en feil i byggejobben skal ikke slippe gjennom.
    pub fn sjekk(&self) -> Resultat<()> {
        let feil = |s: &str| Err(Feil::Manifest(s.into()));
        if self.format != FORMAT {
            return feil("unknown format – update the launcher");
        }
        if self.bygg == 0 {
            return feil("build 0");
        }
        if !er_sha256(&self.spillet.jar_sha256) {
            return feil("game hash");
        }
        if self.server.adresse.trim().is_empty() || self.server.adresse.contains(char::is_whitespace) {
            return feil("server address");
        }
        if self.filer.is_empty() {
            return feil("no files");
        }
        let mut sett = HashSet::new();
        for f in &self.filer {
            if !trygt_filnavn(&f.navn) {
                return feil("file name");
            }
            if !sett.insert(f.navn.to_ascii_lowercase()) {
                return feil("file listed twice");
            }
            if !er_sha256(&f.sha256) {
                return feil("file hash");
            }
            if f.størrelse == 0 || f.størrelse > STØRSTE_FIL {
                return feil("file size");
            }
        }
        if !self.filer.iter().any(|f| f.navn == AGENT_FIL) {
            return feil("no agent");
        }
        Ok(())
    }

    /// Nekter et bygg som er eldre enn det launcheren sist så.
    pub fn sjekk_bygg(&self, sist: Option<u64>) -> Resultat<()> {
        match sist {
            Some(sist) if self.bygg < sist => Err(Feil::EldreBygg { ny: self.bygg, sist }),
            _ => Ok(()),
        }
    }

    pub fn agent(&self) -> &Fil {
        self.filer.iter().find(|f| f.navn == AGENT_FIL).expect("sjekket i sjekk")
    }
}

/// Det høyeste bygget launcheren har sett.
pub fn sist_bygg(tilstand: &Path) -> Option<u64> {
    fs::read_to_string(tilstand.join(SIST_BYGG_FIL)).ok()?.trim().parse().ok()
}

/// Husker bygget – bare hvis det er høyere enn det forrige.
pub fn husk_bygg(tilstand: &Path, bygg: u64) -> Resultat<()> {
    if sist_bygg(tilstand).is_some_and(|s| s >= bygg) {
        return Ok(());
    }
    fs::create_dir_all(tilstand)?;
    let tmp = tilstand.join(format!("{SIST_BYGG_FIL}.tmp"));
    fs::write(&tmp, bygg.to_string())?;
    fs::rename(tmp, tilstand.join(SIST_BYGG_FIL))?;
    Ok(())
}

#[cfg(test)]
pub(crate) mod prøver {
    use super::*;
    use crate::prøvehjelp::Mappe;
    use ed25519_dalek::{Signer, SigningKey};

    pub fn nøkkelpar() -> SigningKey {
        SigningKey::generate(&mut rand::rngs::OsRng)
    }

    pub fn eksempel(bygg: u64) -> Manifest {
        Manifest {
            format: FORMAT,
            bygg,
            utgitt: "2026-11-17T20:00:00Z".into(),
            spillet: Spillversjon { versjon: "42.22.2".into(), jar_sha256: "ab".repeat(32) },
            filer: vec![Fil { navn: AGENT_FIL.into(), sha256: "cd".repeat(32), størrelse: 1234 }],
            server: Server { adresse: "play.example.invalid".into(), port: 16261 },
            nødmodus: false,
        }
    }

    pub fn signer(m: &Manifest, n: &SigningKey) -> (Vec<u8>, Vec<u8>) {
        let json = serde_json::to_vec_pretty(m).unwrap();
        let sig = hex::encode(n.sign(&json).to_bytes()).into_bytes();
        (json, sig)
    }

    #[test]
    fn riktig_signatur_leses() {
        let n = nøkkelpar();
        let (json, sig) = signer(&eksempel(7), &n);
        let m = Manifest::les_signert(&json, &sig, &n.verifying_key()).unwrap();
        assert_eq!(m, eksempel(7));
        assert_eq!(m.agent().navn, AGENT_FIL);
    }

    #[test]
    fn feil_nøkkel_eller_endret_tekst_nektes() {
        let n = nøkkelpar();
        let (json, sig) = signer(&eksempel(7), &n);
        assert!(matches!(Manifest::les_signert(&json, &sig, &nøkkelpar().verifying_key()), Err(Feil::Signatur)));
        let mut endret = json.clone();
        let i = endret.iter().position(|&b| b == b'7').unwrap();
        endret[i] = b'9';
        assert!(matches!(Manifest::les_signert(&endret, &sig, &n.verifying_key()), Err(Feil::Signatur)));
        assert!(matches!(Manifest::les_signert(&json, b"ikke hex", &n.verifying_key()), Err(Feil::Signatur)));
        assert!(matches!(Manifest::les_signert(&json, b"", &n.verifying_key()), Err(Feil::Signatur)));
    }

    #[test]
    fn signert_tull_tolkes_ikke_som_manifest() {
        let n = nøkkelpar();
        let json = b"{\"format\":1}".to_vec();
        let sig = hex::encode(n.sign(&json).to_bytes()).into_bytes();
        assert!(matches!(Manifest::les_signert(&json, &sig, &n.verifying_key()), Err(Feil::Manifest(_))));
    }

    #[test]
    fn farlige_filnavn_nektes() {
        for navn in ["../knoxlife-agent.jar", "a/b.jar", "a\\b.jar", ".skjult", "", "C:x.jar", "æ.jar"] {
            let mut m = eksempel(1);
            m.filer.push(Fil { navn: navn.into(), sha256: "ef".repeat(32), størrelse: 1 });
            assert!(matches!(m.sjekk(), Err(Feil::Manifest(_))), "{navn}");
        }
    }

    #[test]
    fn manifest_uten_agent_eller_med_dobbel_fil_nektes() {
        let mut m = eksempel(1);
        m.filer[0].navn = "annet.jar".into();
        assert!(m.sjekk().is_err());
        let mut m = eksempel(1);
        m.filer.push(m.filer[0].clone());
        assert!(m.sjekk().is_err());
        let mut m = eksempel(1);
        m.spillet.jar_sha256 = "AB".repeat(32);
        assert!(m.sjekk().is_err());
        let mut m = eksempel(1);
        m.format = 2;
        assert!(m.sjekk().is_err());
    }

    #[test]
    fn bygget_går_bare_oppover() {
        let mappe = Mappe::ny();
        assert_eq!(sist_bygg(mappe.sti()), None);
        husk_bygg(mappe.sti(), 12).unwrap();
        husk_bygg(mappe.sti(), 9).unwrap();
        assert_eq!(sist_bygg(mappe.sti()), Some(12));
        assert!(matches!(eksempel(11).sjekk_bygg(sist_bygg(mappe.sti())), Err(Feil::EldreBygg { ny: 11, sist: 12 })));
        assert!(eksempel(12).sjekk_bygg(Some(12)).is_ok());
        assert!(eksempel(13).sjekk_bygg(Some(12)).is_ok());
    }

    #[test]
    fn nøkkel_fra_hex_tåler_mellomrom_og_nekter_tull() {
        let n = nøkkelpar();
        let hex_tekst = format!(" {}\n", hex::encode(n.verifying_key().to_bytes()));
        assert_eq!(nøkkel_fra_hex(&hex_tekst), Some(n.verifying_key()));
        assert_eq!(nøkkel_fra_hex("abc"), None);
    }
}
