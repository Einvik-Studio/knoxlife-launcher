//! Økten: alt som skjer fra spilleren trykker «Play» til spillet er avsluttet (3.2 i systemdokumentet), i rekkefølge.
//!
//! Nettet og maskinen (prosessene, Steam og klokken) gis inn, så hele økten kan prøves uten nett, Steam eller spillet.
//! Vinduet kaller [`kjør`] og viser hvert [`Steg`]; `knoxctl` og prøvene gjør det samme med sine egne.
//!
//! Når patchen først er lagt inn, settes spillet alltid tilbake før [`kjør`] gir svar – også når Steam ikke starter.

use crate::feil::{Feil, Resultat};
use crate::manifest::{self, Fil, Manifest};
use crate::nedlasting::{self, Henter};
use crate::patch::{self, Patch, AGENT_FIL};
use crate::porten::{self, Svar};
use crate::spillet::Spill;
use crate::start::{self, Billett, Tilstand, Vakt};
use ed25519_dalek::VerifyingKey;
use serde::Serialize;
use std::cell::RefCell;
use std::path::Path;

/// Det launcheren henter over nett.
pub trait Nett {
    /// `manifest.json` og `manifest.sig`, som de ligger.
    fn manifest(&self) -> Resultat<(Vec<u8>, Vec<u8>)>;
    /// Én fil i patchen. Bytene sjekkes mot manifestet etterpå.
    fn fil(&self, manifest: &Manifest, fil: &Fil) -> Resultat<Vec<u8>>;
}

/// Det launcheren gjør med maskinen.
pub trait Maskin {
    fn spillet_kjører(&self) -> bool;
    /// Starter spillet gjennom Steam.
    fn start_steam(&self) -> Resultat<()>;
    /// Venter ett sekund. Prøvene lar tiden gå uten å vente.
    fn vent_ett_sekund(&self);
}

/// Det vinduet viser underveis.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "steg", rename_all = "snake_case")]
pub enum Steg {
    /// En rest fra forrige gang settes tilbake.
    Rydder,
    /// Manifestet hentes og sjekkes.
    HenterOppdateringen,
    /// Fil `nr` av `av` lastes ned.
    LasterNed { nr: usize, av: usize },
    /// Patchen legges inn.
    LeggerInn,
    StarterSteam,
    /// Steam starter spillet.
    Venter,
    /// Agenten har skrevet stempelet sitt: Knox Life går.
    Lastet,
    /// Spillet går, men patchen ble ikke lastet – vinduet viser «Copy report».
    IkkeLastet,
    /// Spillet er avsluttet, og filene settes tilbake.
    SetterTilbake,
}

/// Hvordan økten endte.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "utfall", rename_all = "snake_case")]
pub enum Utfall {
    /// Spillet ble startet med patchen og er avsluttet. `lastet` er om agenten skrev stempelet sitt.
    Spilt { bygg: u64, lastet: bool },
    /// Nødmodus (3.9 punkt 9): spillet ble startet uten patch, og det er ingenting å sette tilbake.
    Nødmodus,
}

/// Det økten trenger å vite om spilleren og maskinen.
pub struct Oppsett<'a> {
    pub spill: &'a Spill,
    /// Launcherens egen mappe (merket, kopien, patchene og det høyeste bygget).
    pub tilstand: &'a Path,
    /// Spillerens hjemmemappe (`Zomboid/KnoxLife` ligger der).
    pub hjem: &'a Path,
    /// Mappen agenten kopieres til når Java ikke kan lese stien til launcherens mappe (Windows).
    pub reserve: Option<&'a Path>,
    /// Den offentlige nøkkelen manifestet er signert med. `None` i en prøvebygging: da kan ingenting sjekkes.
    pub nøkkel: Option<&'a VerifyingKey>,
    /// Engangsbilletten fra tjeneren, når spillet skal rett inn på serveren.
    pub billett: Option<&'a Billett>,
}

/// Henter filene i patchen og sier fra om hver.
struct MedFremdrift<'a, 'f> {
    nett: &'a dyn Nett,
    av: usize,
    nr: RefCell<usize>,
    steg: &'a RefCell<&'f mut dyn FnMut(Steg)>,
}

impl Henter for MedFremdrift<'_, '_> {
    fn hent(&self, manifest: &Manifest, fil: &Fil) -> Resultat<Vec<u8>> {
        let nr = {
            let mut n = self.nr.borrow_mut();
            *n = (*n + 1).min(self.av);
            *n
        };
        (self.steg.borrow_mut())(Steg::LasterNed { nr, av: self.av });
        self.nett.fil(manifest, fil)
    }
}

/// Hele økten. `steg` får hvert steg idet det begynner.
pub fn kjør(o: &Oppsett, nett: &dyn Nett, maskin: &dyn Maskin, steg: &mut dyn FnMut(Steg)) -> Resultat<Utfall> {
    let steg = RefCell::new(steg);
    let si = |s: Steg| (steg.borrow_mut())(s);

    // 1–2. En rest fra forrige gang settes tilbake før noe annet, og spillet kan ikke gå allerede.
    si(Steg::Rydder);
    patch::rydd_etter_forrige(o.spill, o.tilstand)?;
    if maskin.spillet_kjører() {
        return Err(Feil::SpilletKjører);
    }

    // 3. Manifestet: signaturen før noe leses, og aldri et eldre bygg enn det vi har sett.
    si(Steg::HenterOppdateringen);
    let nøkkel = o.nøkkel.ok_or(Feil::Signatur)?;
    let (json, sig) = nett.manifest()?;
    let m = Manifest::les_signert(&json, &sig, nøkkel)?;
    m.sjekk_bygg(manifest::sist_bygg(o.tilstand))?;
    manifest::husk_bygg(o.tilstand, m.bygg)?;

    // 4. Versjonsporten.
    let knox = start::knox_mappe(o.hjem);
    if porten::sjekk(o.spill, &m)? == Svar::Nødmodus {
        if let Some(b) = o.billett {
            start::skriv_billett(&knox, b)?;
        }
        si(Steg::StarterSteam);
        maskin.start_steam()?;
        return Ok(Utfall::Nødmodus);
    }

    // 5. Filene i patchen, hver sjekket mot manifestet.
    let mappe = {
        let av = nedlasting::mangler(&m, &nedlasting::patch_mappe(o.tilstand, m.bygg)).len();
        let henter = MedFremdrift { nett, av, nr: RefCell::new(0), steg: &steg };
        nedlasting::hent_det_som_mangler(&m, o.tilstand, &henter)?
    };
    nedlasting::rydd_gamle(o.tilstand, Some(m.bygg))?;

    // 7. Patchen legges inn. Et gammelt stempel fjernes først, så det aldri tas for et nytt.
    si(Steg::LeggerInn);
    start::fjern_stempel(&knox)?;
    let agent = mappe.join(AGENT_FIL);
    let stempel = start::stempel_sti(&knox);
    patch::legg_inn(
        o.spill,
        o.tilstand,
        &Patch { agent: &agent, bygg: m.bygg, stempel: &stempel, hjem: Some(o.hjem), reserve: o.reserve },
    )?;

    // 8–11. Fra her settes spillet tilbake uansett hvordan det går.
    let spilt = (|| -> Resultat<bool> {
        if let Some(b) = o.billett {
            start::skriv_billett(&knox, b)?;
        }
        si(Steg::StarterSteam);
        maskin.start_steam()?;
        si(Steg::Venter);
        let mut vakt = Vakt::ny(m.bygg);
        let mut sist = Tilstand::Venter;
        let mut lastet = false;
        for sek in 0.. {
            let nå = vakt.steg(sek, maskin.spillet_kjører(), start::les_stempel(&stempel));
            if nå != sist {
                match nå {
                    Tilstand::Lastet => {
                        lastet = true;
                        si(Steg::Lastet);
                    }
                    Tilstand::IkkeLastet => si(Steg::IkkeLastet),
                    Tilstand::Avsluttet => break,
                    Tilstand::Venter => {}
                }
                sist = nå;
            }
            maskin.vent_ett_sekund();
        }
        Ok(lastet)
    })();

    // 12. Tilbake – det viktigste, så en feil her går foran en feil over.
    si(Steg::SetterTilbake);
    patch::sett_tilbake(o.spill, o.tilstand).map_err(|e| match e {
        Feil::SpillmappenBorte(_) | Feil::KunneIkkeSetteTilbake(_) => e,
        annet => Feil::KunneIkkeSetteTilbake(annet.to_string()),
    })?;
    // Billetten er bare til denne starten.
    let _ = std::fs::remove_file(start::billett_sti(&knox));
    Ok(Utfall::Spilt { bygg: m.bygg, lastet: spilt? })
}

#[cfg(test)]
mod prøver {
    use super::*;
    use crate::manifest::prøver::{eksempel, nøkkelpar, signer};
    use crate::patch::sha256;
    use crate::prøvehjelp::Mappe;
    use crate::spillet::Plattform;
    use ed25519_dalek::SigningKey;
    use std::cell::Cell;
    use std::fs;
    use std::path::PathBuf;

    const JSON: &str = "{\n\t\"mainClass\": \"zombie/gameStates/MainScreenState\",\n\t\"vmArgs\": [\n\t\t\"-Xmx3072m\"\n\t]\n}\n";
    const AGENT: &[u8] = b"agenten, bygg 12";
    const JAR: &[u8] = b"spillet 42.22.2";

    struct Verden {
        _m: Mappe,
        spill: Spill,
        tilstand: PathBuf,
        hjem: PathBuf,
        nøkkel: SigningKey,
    }

    fn verden() -> Verden {
        let m = Mappe::ny();
        let spill = Spill::ny(m.sti().join("ProjectZomboid"), Plattform::Linux);
        fs::create_dir_all(spill.java_mappe()).unwrap();
        fs::write(spill.jar(), JAR).unwrap();
        fs::write(spill.innstillingsfil(), JSON).unwrap();
        let tilstand = m.sti().join("launcher");
        let hjem = m.sti().join("hjem");
        fs::create_dir_all(&hjem).unwrap();
        Verden { _m: m, spill, tilstand, hjem, nøkkel: nøkkelpar() }
    }

    fn manifest(bygg: u64) -> Manifest {
        let mut m = eksempel(bygg);
        m.spillet.jar_sha256 = sha256(JAR);
        m.filer[0].sha256 = sha256(AGENT);
        m.filer[0].størrelse = AGENT.len() as u64;
        m
    }

    struct FalskNett {
        manifest: (Vec<u8>, Vec<u8>),
        agent: Vec<u8>,
        hentet: Cell<usize>,
    }

    impl Nett for FalskNett {
        fn manifest(&self) -> Resultat<(Vec<u8>, Vec<u8>)> {
            Ok(self.manifest.clone())
        }
        fn fil(&self, _: &Manifest, _: &Fil) -> Resultat<Vec<u8>> {
            self.hentet.set(self.hentet.get() + 1);
            Ok(self.agent.clone())
        }
    }

    fn nett(v: &Verden, m: &Manifest) -> FalskNett {
        FalskNett { manifest: signer(m, &v.nøkkel), agent: AGENT.to_vec(), hentet: Cell::new(0) }
    }

    /// Spillet går fra `går.0` til `går.1` sekunder etter at Steam ble startet, og agenten skriver stempelet ved
    /// `stempel_ved` (med bygget i `stempel_bygg`).
    struct FalskMaskin {
        kjører_før: bool,
        går: (u64, u64),
        stempel: Option<(u64, u64, PathBuf)>,
        steam_feiler: bool,
        klokke: Cell<Option<u64>>,
        så_filen: RefCell<Option<Vec<u8>>>,
        fil: PathBuf,
    }

    impl FalskMaskin {
        fn ny(v: &Verden) -> FalskMaskin {
            FalskMaskin {
                kjører_før: false,
                går: (5, 600),
                stempel: Some((20, 12, start::stempel_sti(&start::knox_mappe(&v.hjem)))),
                steam_feiler: false,
                klokke: Cell::new(None),
                så_filen: RefCell::new(None),
                fil: v.spill.innstillingsfil(),
            }
        }
    }

    impl Maskin for FalskMaskin {
        fn spillet_kjører(&self) -> bool {
            match self.klokke.get() {
                None => self.kjører_før,
                Some(t) => t >= self.går.0 && t < self.går.1,
            }
        }
        fn start_steam(&self) -> Resultat<()> {
            if self.steam_feiler {
                return Err(Feil::SteamStartetIkke("prøve".into()));
            }
            *self.så_filen.borrow_mut() = Some(fs::read(&self.fil).unwrap());
            self.klokke.set(Some(0));
            Ok(())
        }
        fn vent_ett_sekund(&self) {
            let t = self.klokke.get().expect("venter bare etter Steam") + 1;
            self.klokke.set(Some(t));
            if let Some((ved, bygg, sti)) = &self.stempel {
                if t == *ved {
                    fs::create_dir_all(sti.parent().unwrap()).unwrap();
                    fs::write(sti, format!("bygg={bygg}\n")).unwrap();
                }
            }
            assert!(t < 10_000, "vakten ble aldri ferdig");
        }
    }

    fn kjør_økt(v: &Verden, nett: &dyn Nett, maskin: &dyn Maskin) -> (Resultat<Utfall>, Vec<Steg>) {
        let nøkkel = v.nøkkel.verifying_key();
        let o = Oppsett {
            spill: &v.spill,
            tilstand: &v.tilstand,
            hjem: &v.hjem,
            reserve: None,
            nøkkel: Some(&nøkkel),
            billett: None,
        };
        let mut steg = Vec::new();
        let svar = kjør(&o, nett, maskin, &mut |s| steg.push(s));
        (svar, steg)
    }

    #[test]
    fn hele_økten_legger_inn_starter_og_setter_tilbake() {
        let v = verden();
        let n = nett(&v, &manifest(12));
        let maskin = FalskMaskin::ny(&v);
        let (svar, steg) = kjør_økt(&v, &n, &maskin);
        assert_eq!(svar.unwrap(), Utfall::Spilt { bygg: 12, lastet: true });
        assert_eq!(
            steg,
            vec![
                Steg::Rydder,
                Steg::HenterOppdateringen,
                Steg::LasterNed { nr: 1, av: 1 },
                Steg::LeggerInn,
                Steg::StarterSteam,
                Steg::Venter,
                Steg::Lastet,
                Steg::SetterTilbake,
            ]
        );
        // Mens spillet startet, sto agenten i filen; etterpå er filen byte for byte som før.
        let under = String::from_utf8(maskin.så_filen.borrow().clone().unwrap()).unwrap();
        assert!(under.contains("-javaagent:") && under.contains(AGENT_FIL), "{under}");
        assert_eq!(fs::read_to_string(v.spill.innstillingsfil()).unwrap(), JSON);
        assert_eq!(manifest::sist_bygg(&v.tilstand), Some(12));

        // Neste gang ligger agenten der alt: ingen ny nedlasting.
        let maskin = FalskMaskin::ny(&v);
        let (svar, steg) = kjør_økt(&v, &n, &maskin);
        assert!(svar.is_ok());
        assert_eq!(n.hentet.get(), 1);
        assert!(!steg.iter().any(|s| matches!(s, Steg::LasterNed { .. })));
    }

    #[test]
    fn spillet_går_allerede() {
        let v = verden();
        let n = nett(&v, &manifest(12));
        let mut maskin = FalskMaskin::ny(&v);
        maskin.kjører_før = true;
        let (svar, _) = kjør_økt(&v, &n, &maskin);
        assert!(matches!(svar, Err(Feil::SpilletKjører)));
        assert_eq!(n.hentet.get(), 0);
        assert_eq!(fs::read_to_string(v.spill.innstillingsfil()).unwrap(), JSON);
    }

    #[test]
    fn feil_signatur_stopper_før_noe_annet() {
        let v = verden();
        let mut n = nett(&v, &manifest(12));
        n.manifest = signer(&manifest(12), &nøkkelpar());
        let maskin = FalskMaskin::ny(&v);
        let (svar, _) = kjør_økt(&v, &n, &maskin);
        assert!(matches!(svar, Err(Feil::Signatur)));
        assert!(maskin.så_filen.borrow().is_none(), "Steam ble startet");
        assert_eq!(manifest::sist_bygg(&v.tilstand), None);
    }

    #[test]
    fn uten_nøkkel_sjekkes_ingenting() {
        let v = verden();
        let n = nett(&v, &manifest(12));
        let maskin = FalskMaskin::ny(&v);
        let o = Oppsett { spill: &v.spill, tilstand: &v.tilstand, hjem: &v.hjem, reserve: None, nøkkel: None, billett: None };
        let svar = kjør(&o, &n, &maskin, &mut |_| {});
        assert!(matches!(svar, Err(Feil::Signatur)));
    }

    #[test]
    fn eldre_bygg_enn_sist_nektes() {
        let v = verden();
        manifest::husk_bygg(&v.tilstand, 13).unwrap();
        let n = nett(&v, &manifest(12));
        let (svar, _) = kjør_økt(&v, &n, &FalskMaskin::ny(&v));
        assert!(matches!(svar, Err(Feil::EldreBygg { ny: 12, sist: 13 })));
    }

    #[test]
    fn oppdatert_spill_slipper_ikke_gjennom() {
        let v = verden();
        fs::write(v.spill.jar(), b"spillet 42.23.0").unwrap();
        let n = nett(&v, &manifest(12));
        let maskin = FalskMaskin::ny(&v);
        let (svar, _) = kjør_økt(&v, &n, &maskin);
        assert!(matches!(svar, Err(Feil::SpilletOppdatert { .. })));
        assert_eq!(n.hentet.get(), 0);
        assert!(maskin.så_filen.borrow().is_none());
    }

    #[test]
    fn ødelagt_nedlasting_legger_ikke_inn() {
        let v = verden();
        let mut n = nett(&v, &manifest(12));
        n.agent = b"noe annet".to_vec();
        let maskin = FalskMaskin::ny(&v);
        let (svar, _) = kjør_økt(&v, &n, &maskin);
        assert!(matches!(svar, Err(Feil::NedlastingØdelagt(_))));
        assert_eq!(n.hentet.get(), 2, "én gang til før den gir opp");
        assert_eq!(fs::read_to_string(v.spill.innstillingsfil()).unwrap(), JSON);
        assert!(patch::les_merke(&v.tilstand).unwrap().is_none());
    }

    #[test]
    fn patchen_lastet_ikke_men_spillet_settes_tilbake() {
        let v = verden();
        let n = nett(&v, &manifest(12));
        let mut maskin = FalskMaskin::ny(&v);
        maskin.stempel = None;
        let (svar, steg) = kjør_økt(&v, &n, &maskin);
        assert_eq!(svar.unwrap(), Utfall::Spilt { bygg: 12, lastet: false });
        assert_eq!(steg.iter().filter(|s| **s == Steg::IkkeLastet).count(), 1);
        assert_eq!(fs::read_to_string(v.spill.innstillingsfil()).unwrap(), JSON);
    }

    #[test]
    fn steam_starter_ikke_men_spillet_settes_tilbake() {
        let v = verden();
        let n = nett(&v, &manifest(12));
        let mut maskin = FalskMaskin::ny(&v);
        maskin.steam_feiler = true;
        let (svar, steg) = kjør_økt(&v, &n, &maskin);
        assert!(matches!(svar, Err(Feil::SteamStartetIkke(_))));
        assert_eq!(steg.last(), Some(&Steg::SetterTilbake));
        assert_eq!(fs::read_to_string(v.spill.innstillingsfil()).unwrap(), JSON);
        assert!(patch::les_merke(&v.tilstand).unwrap().is_none());
    }

    #[test]
    fn nødmodus_starter_uten_patch() {
        let v = verden();
        let mut m = manifest(12);
        m.nødmodus = true;
        let n = nett(&v, &m);
        let maskin = FalskMaskin::ny(&v);
        let (svar, _) = kjør_økt(&v, &n, &maskin);
        assert_eq!(svar.unwrap(), Utfall::Nødmodus);
        assert_eq!(n.hentet.get(), 0);
        assert_eq!(String::from_utf8(maskin.så_filen.borrow().clone().unwrap()).unwrap(), JSON);
    }

    #[test]
    fn billetten_skrives_før_steam_og_fjernes_etterpå() {
        let v = verden();
        let n = nett(&v, &manifest(12));
        let maskin = FalskMaskin::ny(&v);
        let nøkkel = v.nøkkel.verifying_key();
        let billett = Billett { adresse: "play.example.invalid".into(), port: 16261, billett: "abc".into() };
        let o = Oppsett {
            spill: &v.spill,
            tilstand: &v.tilstand,
            hjem: &v.hjem,
            reserve: None,
            nøkkel: Some(&nøkkel),
            billett: Some(&billett),
        };
        let knox = start::knox_mappe(&v.hjem);
        let mut så_billetten = false;
        kjør(&o, &n, &maskin, &mut |s| {
            if s == Steg::Venter {
                så_billetten = start::billett_sti(&knox).is_file();
            }
        })
        .unwrap();
        assert!(så_billetten);
        assert!(!start::billett_sti(&knox).exists());
    }
}
