//! Starte spillet og følge det til det avsluttes (3.2 punkt 8–12 i systemdokumentet).
//!
//! - **Billetten** (`Zomboid/KnoxLife/reise.txt`): serverens adresse og en engangsbillett. Agenten leser den ved oppstart,
//!   kobler rett til serveren og sletter den.
//! - **Serverne** (`Zomboid/KnoxLife/servere.txt`): Knox Lifes servere fra det signerte manifestet, `navn=adresse:port`.
//!   Agenten sender spillet bare til dem (J5, testserveren).
//! - **Stempelet** (`Zomboid/KnoxLife/stempel.txt`): agenten skriver at den er lastet, med byggenummeret.
//! - **Vakten**: de første 90 sekundene avgjøres ingenting – Steam starter spillet i flere steg, og en prosess kan komme,
//!   forsvinne og komme igjen. Kommer ikke stempelet innen 5 minutter, er patchen ikke lastet.

use crate::feil::Resultat;
use crate::manifest::Manifest;
use crate::spillet::{Plattform, APP_ID};
use std::fs;
use std::path::{Path, PathBuf};

/// Spillets prosess på hver plattform.
pub fn spillets_prosess(plattform: Plattform) -> &'static str {
    match plattform {
        Plattform::Windows => "ProjectZomboid64.exe",
        Plattform::Linux => "ProjectZomboid64",
        // Spillets egen oppstarter i app-pakken (`Contents/MacOS/JavaAppLauncher`).
        Plattform::Mac => "JavaAppLauncher",
    }
}

/// Går spillet nå?
pub fn spillet_kjører(plattform: Plattform) -> bool {
    use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, System};
    let mut sys = System::new();
    sys.refresh_processes_specifics(ProcessesToUpdate::All, true, ProcessRefreshKind::nothing());
    let navn = spillets_prosess(plattform);
    sys.processes().values().any(|p| p.name().to_string_lossy().eq_ignore_ascii_case(navn))
}

/// Spillerens `Zomboid`-mappe (lagringene, loggene og innstillingene) – den samme på alle plattformene.
pub fn zomboid_mappe(hjem: &Path) -> PathBuf {
    hjem.join("Zomboid")
}

/// Mappen launcheren og agenten deler i `Zomboid`-mappen.
pub fn knox_mappe(hjem: &Path) -> PathBuf {
    zomboid_mappe(hjem).join("KnoxLife")
}

/// Kommandoen som starter spillet gjennom Steam, som et program og argumentene. Steam sjekker eierskapet og oppdaterer
/// spillet som vanlig.
pub fn steam_kommando(plattform: Plattform, steam: Option<&Path>, steam_i_path: bool) -> (PathBuf, Vec<String>) {
    let lenke = format!("steam://run/{APP_ID}");
    match plattform {
        Plattform::Windows => match steam {
            Some(s) => (s.join("steam.exe"), vec!["-applaunch".into(), APP_ID.to_string()]),
            None => ("explorer.exe".into(), vec![lenke]),
        },
        Plattform::Linux if steam_i_path => ("steam".into(), vec!["-applaunch".into(), APP_ID.to_string()]),
        // Flatpak og Snap legger ikke `steam` i PATH, men lenken virker.
        Plattform::Linux => ("xdg-open".into(), vec![lenke]),
        Plattform::Mac => ("open".into(), vec![lenke]),
    }
}

// ---------- billetten ----------

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Billett {
    pub adresse: String,
    pub port: u16,
    /// Engangsbilletten fra tjeneren. Bare gyldig i 10 minutter og bare for denne Steam-kontoen.
    pub billett: String,
}

pub fn billett_sti(knox: &Path) -> PathBuf {
    knox.join("reise.txt")
}

/// Skriver billetten. Linjene er `nøkkel=verdi`, ASCII, så Java leser dem likt på alle PC-er.
pub fn skriv_billett(knox: &Path, b: &Billett) -> Resultat<()> {
    let ren = |s: &str| s.chars().filter(|c| c.is_ascii_graphic()).collect::<String>();
    fs::create_dir_all(knox)?;
    let tekst = format!("adresse={}\nport={}\nbillett={}\n", ren(&b.adresse), b.port, ren(&b.billett));
    let tmp = knox.join("reise.txt.tmp");
    fs::write(&tmp, tekst)?;
    fs::rename(tmp, billett_sti(knox))?;
    Ok(())
}

pub fn servere_sti(knox: &Path) -> PathBuf {
    knox.join("servere.txt")
}

/// Skriver listen over Knox Lifes servere fra manifestet: `hoved` og `test` (når den finnes). En adresse agenten ikke
/// ville godta, står ikke med.
pub fn skriv_servere(knox: &Path, m: &Manifest) -> Resultat<()> {
    let mut tekst = String::new();
    for (navn, s) in [("hoved", Some(&m.server)), ("test", m.testserver.as_ref())] {
        if let Some(s) = s.filter(|s| s.trygg()) {
            tekst.push_str(&format!("{navn}={}:{}\n", s.adresse, s.port));
        }
    }
    fs::create_dir_all(knox)?;
    let tmp = knox.join("servere.txt.tmp");
    fs::write(&tmp, tekst)?;
    fs::rename(tmp, servere_sti(knox))?;
    Ok(())
}

// ---------- stempelet ----------

pub fn stempel_sti(knox: &Path) -> PathBuf {
    knox.join("stempel.txt")
}

/// Fjerner et gammelt stempel før spillet startes, så et stempel fra forrige økt aldri tas for et nytt.
pub fn fjern_stempel(knox: &Path) -> Resultat<()> {
    match fs::remove_file(stempel_sti(knox)) {
        Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(e.into()),
        _ => Ok(()),
    }
}

/// Byggenummeret i stempelet, hvis agenten har skrevet det.
pub fn les_stempel(sti: &Path) -> Option<u64> {
    let tekst = fs::read_to_string(sti).ok()?;
    tekst.lines().find_map(|l| l.trim().strip_prefix("bygg=")?.trim().parse().ok())
}

// ---------- vakten ----------

/// Sekundene før vakten avgjør noe som helst.
pub const STEAM_VINDU: u64 = 90;
/// Sekundene uten stempel før patchen regnes som ikke lastet.
pub const STEMPEL_FRIST: u64 = 300;
/// Spillet må være borte så lenge før det regnes som avsluttet (en prosess som starter på nytt, rekker å komme tilbake).
pub const BORTE_I: u64 = 10;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tilstand {
    /// Spillet starter.
    Venter,
    /// Agenten er lastet med riktig bygg.
    Lastet,
    /// Spillet går, men agenten er ikke lastet (eller med et annet bygg). Launcheren viser «Copy report».
    IkkeLastet,
    /// Spillet er avsluttet (eller kom aldri i gang): sett tilbake.
    Avsluttet,
}

/// Følger spillet fra «Play» til det avsluttes. Tiden gis inn, så vakten kan prøves uten å vente.
#[derive(Debug, Clone)]
pub struct Vakt {
    bygg: u64,
    sett_spillet: bool,
    lastet: bool,
    feil_bygg: bool,
    borte_fra: Option<u64>,
}

impl Vakt {
    pub fn ny(bygg: u64) -> Vakt {
        Vakt { bygg, sett_spillet: false, lastet: false, feil_bygg: false, borte_fra: None }
    }

    /// Ett steg: `sek` er sekunder siden spillet ble startet.
    pub fn steg(&mut self, sek: u64, spillet_går: bool, stempel: Option<u64>) -> Tilstand {
        match stempel {
            Some(b) if b == self.bygg => self.lastet = true,
            Some(_) => self.feil_bygg = true,
            None => {}
        }
        if spillet_går {
            self.sett_spillet = true;
            self.borte_fra = None;
        } else if self.borte_fra.is_none() {
            self.borte_fra = Some(sek);
        }

        let borte_lenge = self.borte_fra.is_some_and(|fra| sek.saturating_sub(fra) >= BORTE_I);
        if sek >= STEAM_VINDU && !spillet_går && borte_lenge && (self.sett_spillet || sek >= STEMPEL_FRIST) {
            return Tilstand::Avsluttet;
        }
        if self.lastet && !self.feil_bygg {
            return Tilstand::Lastet;
        }
        if self.feil_bygg || (sek >= STEMPEL_FRIST && self.sett_spillet) {
            return Tilstand::IkkeLastet;
        }
        Tilstand::Venter
    }
}

#[cfg(test)]
mod prøver {
    use super::*;
    use crate::prøvehjelp::Mappe;

    #[test]
    fn steam_på_hver_plattform() {
        let (p, a) = steam_kommando(Plattform::Windows, Some(Path::new(r"C:\Steam")), false);
        assert!(p.ends_with("steam.exe"));
        assert_eq!(a, ["-applaunch", "108600"]);
        assert_eq!(steam_kommando(Plattform::Linux, None, true).0, PathBuf::from("steam"));
        assert_eq!(steam_kommando(Plattform::Linux, None, false), ("xdg-open".into(), vec!["steam://run/108600".into()]));
        assert_eq!(steam_kommando(Plattform::Mac, None, false).0, PathBuf::from("open"));
    }

    #[test]
    fn billetten_er_ren_ascii() {
        let m = Mappe::ny();
        let b = Billett { adresse: "play.example.invalid".into(), port: 16261, billett: "abc 123\nport=1".into() };
        skriv_billett(m.sti(), &b).unwrap();
        let tekst = fs::read_to_string(billett_sti(m.sti())).unwrap();
        assert_eq!(tekst, "adresse=play.example.invalid\nport=16261\nbillett=abc123port=1\n");
    }

    #[test]
    fn serverne_fra_manifestet() {
        let m = Mappe::ny();
        let mut man = crate::manifest::prøver::eksempel(7);
        skriv_servere(m.sti(), &man).unwrap();
        assert_eq!(fs::read_to_string(servere_sti(m.sti())).unwrap(), "hoved=play.example.invalid:16261\n");
        man.testserver = Some(crate::manifest::Server { adresse: "10.0.0.6".into(), port: 16271 });
        skriv_servere(m.sti(), &man).unwrap();
        assert_eq!(fs::read_to_string(servere_sti(m.sti())).unwrap(), "hoved=play.example.invalid:16261\ntest=10.0.0.6:16271\n");
        // En hovedadresse agenten ikke godtar, står ikke med (den brukes bare til billetten).
        man.server.adresse = "[::1]".into();
        skriv_servere(m.sti(), &man).unwrap();
        assert_eq!(fs::read_to_string(servere_sti(m.sti())).unwrap(), "test=10.0.0.6:16271\n");
    }

    #[test]
    fn stempelet_leses_og_fjernes() {
        let m = Mappe::ny();
        assert_eq!(les_stempel(&stempel_sti(m.sti())), None);
        fs::write(stempel_sti(m.sti()), "bygg=14\ntid=1763400000\n").unwrap();
        assert_eq!(les_stempel(&stempel_sti(m.sti())), Some(14));
        fjern_stempel(m.sti()).unwrap();
        fjern_stempel(m.sti()).unwrap();
        assert!(!stempel_sti(m.sti()).exists());
    }

    #[test]
    fn prosessen_som_forsvinner_og_kommer_igjen_regnes_ikke_som_avsluttet() {
        // P97 sin feil: launcheren trodde spilleren hadde avsluttet, satte filen tilbake, og spillet gikk uten patch.
        let mut v = Vakt::new_for_prøve();
        assert_eq!(v.steg(5, true, None), Tilstand::Venter);
        assert_eq!(v.steg(20, false, None), Tilstand::Venter);
        assert_eq!(v.steg(60, false, None), Tilstand::Venter);
        assert_eq!(v.steg(70, true, None), Tilstand::Venter);
        assert_eq!(v.steg(80, true, Some(14)), Tilstand::Lastet);
        assert_eq!(v.steg(3_600, true, Some(14)), Tilstand::Lastet);
    }

    #[test]
    fn avsluttet_først_når_spillet_har_vært_borte_en_stund() {
        let mut v = Vakt::new_for_prøve();
        v.steg(30, true, Some(14));
        assert_eq!(v.steg(500, false, Some(14)), Tilstand::Lastet);
        assert_eq!(v.steg(505, false, Some(14)), Tilstand::Lastet);
        assert_eq!(v.steg(511, false, Some(14)), Tilstand::Avsluttet);
    }

    #[test]
    fn uten_stempel_etter_fem_minutter_er_patchen_ikke_lastet() {
        let mut v = Vakt::new_for_prøve();
        assert_eq!(v.steg(100, true, None), Tilstand::Venter);
        assert_eq!(v.steg(300, true, None), Tilstand::IkkeLastet);
        assert_eq!(v.steg(900, false, None), Tilstand::IkkeLastet);
        assert_eq!(v.steg(911, false, None), Tilstand::Avsluttet);
    }

    #[test]
    fn feil_bygg_i_stempelet_er_ikke_lastet() {
        let mut v = Vakt::new_for_prøve();
        assert_eq!(v.steg(40, true, Some(13)), Tilstand::IkkeLastet);
    }

    #[test]
    fn spillet_som_aldri_kom_i_gang_gir_avsluttet_etter_fristen() {
        let mut v = Vakt::new_for_prøve();
        assert_eq!(v.steg(100, false, None), Tilstand::Venter);
        assert_eq!(v.steg(250, false, None), Tilstand::Venter);
        assert_eq!(v.steg(300, false, None), Tilstand::Avsluttet);
    }

    impl Vakt {
        fn new_for_prøve() -> Vakt {
            Vakt::ny(14)
        }
    }
}
