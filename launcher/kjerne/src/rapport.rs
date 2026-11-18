//! Feilrapporten («Copy report» i launcheren): det staff trenger for å se hva som gikk galt, som tekst spilleren selv
//! limer inn på Discord. Launcheren sender den aldri noe sted.
//!
//! Hjemmemappen skrives som `~`, så brukernavnet på PC-en ikke står i rapporten, og Steam-ID-en er ikke med.

use crate::manifest;
use crate::patch;
use crate::porten;
use crate::spillet::{Plattform, Spill};
use crate::start;
use std::fmt::Write;
use std::fs;
use std::path::Path;

/// Linjene fra spillets logg som tas med: de siste som nevner Knox Life eller Mixin.
pub const LOGGLINJER: usize = 30;
const LENGSTE_LINJE: usize = 300;

/// Det launcheren vet når spilleren trykker «Copy report».
pub struct Fakta<'a> {
    pub launcher: &'a str,
    pub plattform: Plattform,
    pub spill: Option<&'a Spill>,
    pub tilstand: &'a Path,
    pub hjem: &'a Path,
    /// Feilen launcheren viste sist, hvis noen.
    pub siste_feil: Option<&'a str>,
}

pub fn lag(f: &Fakta) -> String {
    let skjul = |s: &str| skjul_hjem(s, f.hjem);
    let mut r = String::new();
    let _ = writeln!(r, "Knox Life report");
    let _ = writeln!(r, "launcher: {} ({:?})", f.launcher, f.plattform);
    match f.spill {
        Some(s) => {
            let _ = writeln!(r, "game: {}", skjul(&s.rot.to_string_lossy()));
            let jar = porten::sha256_av_fil(&s.jar()).unwrap_or_else(|_| "not found".into());
            let _ = writeln!(r, "projectzomboid.jar: {jar}");
            let inne = match patch::er_lagt_inn(s, f.tilstand) {
                Ok(true) => "yes",
                Ok(false) => "no",
                Err(_) => "unknown",
            };
            let _ = writeln!(r, "patch in the game files now: {inne}");
        }
        None => {
            let _ = writeln!(r, "game: not found");
        }
    }
    let _ = writeln!(
        r,
        "newest update seen: {}",
        manifest::sist_bygg(f.tilstand).map_or("none".into(), |b| format!("build {b}"))
    );
    if let Ok(Some(m)) = patch::les_merke(f.tilstand) {
        let _ = writeln!(r, "mark left from build {}: {}", m.bygg, skjul(&m.fil.to_string_lossy()));
    }
    let knox = start::knox_mappe(f.hjem);
    let stempel = fs::read_to_string(start::stempel_sti(&knox)).unwrap_or_default();
    let stempel: Vec<&str> = stempel.lines().map(str::trim).filter(|l| !l.is_empty()).collect();
    let _ = writeln!(r, "stamp: {}", if stempel.is_empty() { "none".into() } else { skjul(&stempel.join(" · ")) });
    if let Some(e) = f.siste_feil {
        let _ = writeln!(r, "last error: {}", skjul(e));
    }
    let logg = start::zomboid_mappe(f.hjem).join("console.txt");
    let linjer = logglinjer(&fs::read_to_string(logg).unwrap_or_default());
    let _ = writeln!(r, "-- game log, last lines about Knox Life or Mixin ({}) --", linjer.len());
    for l in linjer {
        let _ = writeln!(r, "{}", skjul(&l));
    }
    r
}

/// De siste [`LOGGLINJER`] linjene som nevner Knox Life eller Mixin, høyst [`LENGSTE_LINJE`] tegn hver.
fn logglinjer(logg: &str) -> Vec<String> {
    let mut ut: Vec<String> = logg
        .lines()
        .filter(|l| {
            let l = l.to_ascii_lowercase();
            l.contains("knoxlife") || l.contains("mixin")
        })
        .map(|l| l.chars().take(LENGSTE_LINJE).collect())
        .collect();
    let fra = ut.len().saturating_sub(LOGGLINJER);
    ut.drain(..fra);
    ut
}

/// Hjemmemappen som `~`, med `\` og `/` begge veier.
fn skjul_hjem(tekst: &str, hjem: &Path) -> String {
    let h = hjem.to_string_lossy();
    if h.len() < 2 {
        return tekst.to_string();
    }
    let mut ut = tekst.to_string();
    for variant in [h.to_string(), h.replace('\\', "/"), h.replace('/', "\\")] {
        ut = ut.replace(&variant, "~");
    }
    ut
}

#[cfg(test)]
mod prøver {
    use super::*;
    use crate::prøvehjelp::Mappe;

    #[test]
    fn rapporten_har_det_staff_trenger_uten_brukernavnet() {
        let m = Mappe::ny();
        let hjem = m.sti().join("Bjørn");
        let spill = Spill::ny(hjem.join("Steam").join("ProjectZomboid"), Plattform::Linux);
        fs::create_dir_all(spill.java_mappe()).unwrap();
        fs::write(spill.jar(), b"jar").unwrap();
        fs::write(spill.innstillingsfil(), b"{}").unwrap();
        let tilstand = m.sti().join("launcher");
        manifest::husk_bygg(&tilstand, 12).unwrap();
        let knox = start::knox_mappe(&hjem);
        fs::create_dir_all(&knox).unwrap();
        fs::write(start::stempel_sti(&knox), "feil=IOException: denied\ntid=1\n").unwrap();
        let mut logg = String::new();
        for i in 0..40 {
            logg.push_str(&format!("LOG  : General  , {i}> [KnoxLife] line {i} in {}\n", hjem.display()));
            logg.push_str("LOG  : General  , something else\n");
        }
        logg.push_str("[mixin] ERROR could not apply\n");
        fs::write(start::zomboid_mappe(&hjem).join("console.txt"), logg).unwrap();

        let r = lag(&Fakta {
            launcher: "1.0.0",
            plattform: Plattform::Linux,
            spill: Some(&spill),
            tilstand: &tilstand,
            hjem: &hjem,
            siste_feil: Some("The Knox Life patch did not load."),
        });
        assert!(r.contains("launcher: 1.0.0 (Linux)"), "{r}");
        assert!(r.contains(&format!("projectzomboid.jar: {}", crate::patch::sha256(b"jar"))), "{r}");
        assert!(r.contains("patch in the game files now: no"), "{r}");
        assert!(r.contains("newest update seen: build 12"), "{r}");
        assert!(r.contains("stamp: feil=IOException: denied · tid=1"), "{r}");
        assert!(r.contains("last error: The Knox Life patch did not load."), "{r}");
        assert!(r.contains("game: ~/Steam/ProjectZomboid"), "{r}");
        assert!(!r.contains("Bjørn"), "brukernavnet står i rapporten:\n{r}");
        assert!(!r.contains("something else"));
        assert!(r.contains("(30) --"), "{r}");
        assert!(r.contains("[mixin] ERROR could not apply"));
        assert!(!r.contains("line 10 "), "for gamle linjer er med");
        assert!(r.contains("line 39 in ~"), "{r}");
    }

    #[test]
    fn uten_spillet() {
        let m = Mappe::ny();
        let r = lag(&Fakta {
            launcher: "1.0.0",
            plattform: Plattform::Windows,
            spill: None,
            tilstand: m.sti(),
            hjem: m.sti(),
            siste_feil: None,
        });
        assert!(r.contains("game: not found") && r.contains("stamp: none") && r.contains("newest update seen: none"));
    }
}
