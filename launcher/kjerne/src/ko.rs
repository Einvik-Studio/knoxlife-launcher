//! Serverens status og køen når serveren er full (`launcher.md` 3.8 punkt 6).
//!
//! Køen kan ikke ligge i spillet: den som venter i spillets egen kø, holder en av forbindelsene til spillets nett, og
//! når serveren er full, er det ingen igjen. Derfor venter spilleren i launcheren. Serveren holder plassen for den som
//! står først i 5 minutter, og launcheren starter spillet når det er spillerens tur.
//!
//! Adressen står i det signerte manifestet (`status`). Launcheren spør `status` (tallene, ingen navn) og – bare når
//! serveren er full og spilleren har trykket «Play» – `ko` med Steam-ID-en, den samme spillet sender når det kobler til.
//! Her er bare det som leses og sjekkes; nettet er i vinduet.

use crate::feil::{Feil, Resultat};
use serde::{Deserialize, Serialize};

/// Svarene er små; noe større er ikke fra Knox Life.
pub const STØRSTE_SVAR: u64 = 4096;
/// Så ofte launcheren spør mens spilleren står i køen (tjeneren tar den ut etter 35 sekunder uten spørsmål).
pub const SPØR_SEKUNDER: u64 = 10;
/// Så ofte statusen hentes mens vinduet er åpent.
pub const STATUS_SEKUNDER: u64 = 30;
const FORMAT: u32 = 1;
const LENGSTE_ADRESSE: usize = 200;

/// Serverens status slik tjeneren gir den (`GET <status>status`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Serverstatus {
    pub format: u32,
    /// Spillet går.
    pub oppe: bool,
    /// Rett før en omstart, eller nede: ingen slipper inn akkurat nå.
    pub stengt: bool,
    pub spillere: u32,
    pub maks: u32,
    /// En vanlig spiller kommer ikke inn nå – eller andre står alt i køen.
    pub fullt: bool,
    /// Hvor mange som står i køen.
    pub ko: u32,
    /// Neste omstart (ms siden 1970), når den er planlagt.
    #[serde(default)]
    pub omstart: Option<u64>,
}

/// Det tjeneren svarer når spilleren står i køen (`POST <status>ko`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "ko", rename_all = "snake_case")]
pub enum Kosvar {
    /// Spillerens tur: serveren holder plassen til fristen (ms siden 1970; ingen frist for KCSO og EMS, som har sine
    /// egne plasser).
    Klar { frist: Option<u64> },
    /// Nummer `plass` av `av` i køen.
    Venter { plass: u32, av: u32 },
    /// Køen er så lang at ingen flere får stå i den.
    KoenErFull,
    /// Spillet går ikke (omstart eller nede).
    Nede,
    /// Steam-kontoen er ikke på whitelisten.
    IkkePaWhitelisten,
}

/// Svaret fra tjeneren slik det står, før det er sjekket.
#[derive(Deserialize)]
struct Rått {
    klar: Option<bool>,
    frist: Option<u64>,
    plass: Option<u32>,
    av: Option<u32>,
    full: Option<bool>,
    nede: Option<bool>,
    whitelist: Option<bool>,
}

/// Adressen til statusen i manifestet: HTTPS, uten brukernavn og passord, uten mellomrom, og med `/` til slutt –
/// `status` og `ko` legges rett etter.
pub fn gyldig_adresse(adresse: &str) -> bool {
    let Some(resten) = adresse.strip_prefix("https://") else {
        return false;
    };
    let vert = resten.split('/').next().unwrap_or_default();
    adresse.len() <= LENGSTE_ADRESSE
        && adresse.ends_with('/')
        && !vert.is_empty()
        && !vert.contains('@')
        && adresse.bytes().all(|b| b.is_ascii_graphic())
        && !adresse.contains(['?', '#', '\\'])
}

pub fn les_status(json: &[u8]) -> Resultat<Serverstatus> {
    let s: Serverstatus = serde_json::from_slice(json).map_err(|e| Feil::Nett(format!("server status: {e}")))?;
    if s.format != FORMAT {
        return Err(Feil::Nett("server status: unknown format".into()));
    }
    if s.maks == 0 || s.maks > 1000 || s.spillere > s.maks.max(1000) || s.ko > 100_000 {
        return Err(Feil::Nett("server status: numbers out of range".into()));
    }
    Ok(s)
}

pub fn les_kosvar(json: &[u8]) -> Resultat<Kosvar> {
    let r: Rått = serde_json::from_slice(json).map_err(|e| Feil::Nett(format!("line: {e}")))?;
    if r.klar == Some(true) {
        return Ok(Kosvar::Klar { frist: r.frist });
    }
    if r.whitelist == Some(false) {
        return Ok(Kosvar::IkkePaWhitelisten);
    }
    if r.nede == Some(true) {
        return Ok(Kosvar::Nede);
    }
    if r.full == Some(true) {
        return Ok(Kosvar::KoenErFull);
    }
    match (r.plass, r.av) {
        (Some(plass), Some(av)) if plass >= 1 && plass <= av => Ok(Kosvar::Venter { plass, av }),
        _ => Err(Feil::Nett("line: unexpected answer".into())),
    }
}

/// Må spilleren stå i kø før spillet startes? Bare når serveren går og er full – og aldri i nødmodus, der serveren går
/// uten Java-modene og ingen plasser holdes (3.9 punkt 9).
pub fn må_stå_i_kø(s: &Serverstatus, nødmodus: bool) -> bool {
    s.oppe && !s.stengt && s.fullt && !nødmodus
}

/// Kroppen launcheren sender til `ko` og `ko/forlat`: bare Steam-ID-en.
pub fn kropp(steam_id: &str) -> Resultat<String> {
    if steam_id.len() != 17 || !steam_id.starts_with("7656119") || !steam_id.bytes().all(|b| b.is_ascii_digit()) {
        return Err(Feil::Nett("no Steam account on this PC – log in to Steam first".into()));
    }
    Ok(format!("{{\"steam\":\"{steam_id}\"}}"))
}

#[cfg(test)]
mod prøver {
    use super::*;

    #[test]
    fn adressen_må_være_https_med_skråstrek_til_slutt() {
        assert!(gyldig_adresse("https://status.example.invalid/"));
        assert!(gyldig_adresse("https://example.invalid/knoxlife/"));
        for feil in [
            "http://status.example.invalid/",
            "https://status.example.invalid",
            "https:///",
            "https://bruker:passord@example.invalid/",
            "https://example.invalid/ a/",
            "https://example.invalid/?x=1/",
            "https://example.invalid/#/",
            "ftp://example.invalid/",
            "",
        ] {
            assert!(!gyldig_adresse(feil), "{feil}");
        }
        assert!(!gyldig_adresse(&format!("https://{}/", "a".repeat(200))));
    }

    #[test]
    fn statusen_leses_og_tall_som_ikke_stemmer_avvises() {
        let s = les_status(br#"{"format":1,"server":"hoved","oppe":true,"stengt":false,"spillere":212,"maks":254,"fullt":false,"ko":0,"omstart":1763521200000}"#).unwrap();
        assert_eq!(s, Serverstatus { format: 1, oppe: true, stengt: false, spillere: 212, maks: 254, fullt: false, ko: 0, omstart: Some(1763521200000) });
        let uten_omstart = les_status(br#"{"format":1,"oppe":false,"stengt":true,"spillere":0,"maks":254,"fullt":false,"ko":0,"omstart":null}"#).unwrap();
        assert_eq!(uten_omstart.omstart, None);
        assert!(les_status(br#"{"format":2,"oppe":true,"stengt":false,"spillere":1,"maks":254,"fullt":false,"ko":0}"#).is_err());
        assert!(les_status(br#"{"format":1,"oppe":true,"stengt":false,"spillere":1,"maks":0,"fullt":false,"ko":0}"#).is_err());
        assert!(les_status(br#"{"format":1,"oppe":true,"stengt":false,"spillere":-1,"maks":254,"fullt":false,"ko":0}"#).is_err());
        assert!(les_status(b"<html>").is_err());
    }

    #[test]
    fn svarene_fra_køen() {
        assert_eq!(les_kosvar(br#"{"klar":true,"frist":1763521200000}"#).unwrap(), Kosvar::Klar { frist: Some(1763521200000) });
        assert_eq!(les_kosvar(br#"{"klar":true,"frist":null}"#).unwrap(), Kosvar::Klar { frist: None });
        assert_eq!(les_kosvar(br#"{"klar":false,"plass":4,"av":9}"#).unwrap(), Kosvar::Venter { plass: 4, av: 9 });
        assert_eq!(les_kosvar(br#"{"klar":false,"plass":null,"av":1000,"full":true}"#).unwrap(), Kosvar::KoenErFull);
        assert_eq!(les_kosvar(br#"{"klar":false,"nede":true}"#).unwrap(), Kosvar::Nede);
        assert_eq!(les_kosvar(br#"{"klar":false,"whitelist":false}"#).unwrap(), Kosvar::IkkePaWhitelisten);
        assert!(les_kosvar(br#"{"klar":false,"plass":5,"av":4}"#).is_err());
        assert!(les_kosvar(br#"{"klar":false,"plass":0,"av":4}"#).is_err());
        assert!(les_kosvar(br#"{"feil":"for_mange"}"#).is_err());
    }

    #[test]
    fn kø_bare_når_serveren_går_og_er_full_og_aldri_i_nødmodus() {
        let mut s = les_status(br#"{"format":1,"oppe":true,"stengt":false,"spillere":241,"maks":254,"fullt":true,"ko":3}"#).unwrap();
        assert!(må_stå_i_kø(&s, false));
        assert!(!må_stå_i_kø(&s, true));
        s.fullt = false;
        assert!(!må_stå_i_kø(&s, false));
        s.fullt = true;
        s.stengt = true;
        assert!(!må_stå_i_kø(&s, false)); // ingen kø mens serveren starter på nytt
    }

    #[test]
    fn bare_steam_id_en_sendes() {
        assert_eq!(kropp("76561198000000001").unwrap(), r#"{"steam":"76561198000000001"}"#);
        for feil in ["", "7656119800000000", "12345678901234567", "7656119800000000\""] {
            assert!(kropp(feil).is_err(), "{feil}");
        }
    }
}
