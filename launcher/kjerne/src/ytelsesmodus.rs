//! Ytelsesmodus (3.8 punkt 5 i systemdokumentet): faste verdier i spillets `Zomboid/options.ini` mens Knox Life går,
//! for PC-er som sliter. Spilleren slår den på under «Details»; den er av som standard.
//!
//! Hver verdi er sjekket mot spillets egen kode (`zombie.core.Core` og `MainOptions.lua` i B42.22) og er bare
//! utseende – ingenting spilleren trenger for å se eller høre andre spillere. Manifestet kan bytte listen uten ny
//! launcher ([`crate::manifest::Manifest::ytelsesmodus`]).
//!
//! Før spillet startes, huskes verdiene spilleren hadde i `ytelsesmodus.json` i launcherens mappe, og først da skrives
//! filen. Når spillet er avsluttet, settes hver verdi tilbake – bare der den fortsatt er den vi skrev: endret spilleren
//! den selv i spillet, beholdes spillerens valg. Ble launcheren stoppet midt i, settes det tilbake ved neste start.

use crate::feil::Resultat;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Verdiene, med det de betyr i spillet. Bare utseende; ingenting om lyd, skjermen, kontrollene eller det man ser av
/// andre spillere.
pub const PROFIL: &[(&str, &str)] = &[
    // Vannet: 0 høy, 1 middels, 2 lav (`water`, PerformanceSettings.waterQuality).
    ("water", "2"),
    // Søleputtene: 0 høy, 1 middels, 2 lav (`puddles`).
    ("puddles", "2"),
    // Hvor søleputter tegnes: 0 alle, 1 bakken og hjulsporene, 2 bare bakken, 3 ingen (`perfPuddles`).
    ("perfPuddles", "2"),
    // Himmelen: 0 høy, 1 middels, 2 stillestående (`perfSkybox`).
    ("perfSkybox", "2"),
    // Speilingene (`bPerfReflections`) – krever omstart, som launcheren gir.
    ("bPerfReflections", "false"),
    // Kropper som faller med fysikk samtidig: 5–60, spillet har 20 (`maxActiveRagdolls`).
    ("maxActiveRagdolls", "10"),
    // Skyggene under kropper (`corpseShadows`).
    ("corpseShadows", "false"),
    // Blodflekker på bakken: 0–10, spillet har 10 (`bloodDecals`).
    ("bloodDecals", "5"),
    // Ting som er satt ut i verden, i høy oppløsning (`highResPlacedItems`).
    ("highResPlacedItems", "false"),
    // Hvor ofte vinduene i spillet tegnes: 10–120 per sekund, spillet har 60 (`uiRenderFPS`).
    ("uiRenderFPS", "30"),
];

/// Høyst så mange verdier i manifestets liste.
pub const FLEST: usize = 30;
const VALGT_FIL: &str = "ytelsesmodus.txt";
const HUSKET_FIL: &str = "ytelsesmodus.json";

/// Verdiene som brukes: manifestets når det har en liste, ellers [`PROFIL`].
pub fn verdiene(fra_manifestet: Option<&BTreeMap<String, String>>) -> BTreeMap<String, String> {
    match fra_manifestet {
        Some(m) => m.clone(),
        None => PROFIL.iter().map(|(n, v)| (n.to_string(), v.to_string())).collect(),
    }
}

/// Et navn i `options.ini`: bokstaver, tall og `_`, som spillets egne.
pub fn gyldig_navn(navn: &str) -> bool {
    !navn.is_empty() && navn.len() <= 40 && navn.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
}

/// En verdi: et tall, `true` eller `false` – aldri et linjeskift eller noe som kan bli en ny linje i filen.
pub fn gyldig_verdi(verdi: &str) -> bool {
    !verdi.is_empty() && verdi.len() <= 20 && verdi.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'.' || b == b'-')
}

/// Stien til spillets innstillinger.
pub fn options_sti(hjem: &Path) -> PathBuf {
    crate::start::zomboid_mappe(hjem).join("options.ini")
}

/// Har spilleren slått den på?
pub fn valgt(tilstand: &Path) -> bool {
    fs::read_to_string(tilstand.join(VALGT_FIL)).is_ok_and(|s| s.trim() == "on")
}

/// «Performance mode» under «Details».
pub fn velg(tilstand: &Path, på: bool) -> Resultat<()> {
    fs::create_dir_all(tilstand)?;
    skriv_trygt(&tilstand.join(VALGT_FIL), if på { b"on\n" } else { b"off\n" })
}

/// Det som huskes mens spillet går.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct Husket {
    /// Verdiene vi skrev.
    skrevet: BTreeMap<String, String>,
    /// Verdiene spilleren hadde – `None` der linjen ikke fantes.
    før: BTreeMap<String, Option<String>>,
}

/// Verdien for `navn` i filen, slik den står.
fn les(tekst: &str, navn: &str) -> Option<String> {
    tekst.lines().find_map(|l| {
        let (n, v) = l.split_once('=')?;
        (n == navn).then(|| v.trim_end_matches('\r').to_string())
    })
}

/// Filen med endringene: en verdi byttes der linjen står, `None` tar linjen bort, og en ny legges til nederst. Alle
/// andre linjer står som de var, med de samme linjeskiftene.
fn flett(tekst: &str, endringer: &BTreeMap<String, Option<String>>) -> String {
    let nl = if tekst.contains("\r\n") { "\r\n" } else { "\n" };
    let mut ut = String::with_capacity(tekst.len() + 256);
    let mut sett = std::collections::HashSet::new();
    for linje in tekst.split_inclusive('\n') {
        let uten = linje.trim_end_matches('\n').trim_end_matches('\r');
        if let Some((n, _)) = uten.split_once('=') {
            if let Some(ny) = endringer.get(n) {
                sett.insert(n.to_string());
                if let Some(v) = ny {
                    ut.push_str(n);
                    ut.push('=');
                    ut.push_str(v);
                    ut.push_str(&linje[uten.len()..]);
                }
                continue;
            }
        }
        ut.push_str(linje);
    }
    for (n, ny) in endringer {
        if let Some(v) = ny {
            if !sett.contains(n) {
                if !ut.is_empty() && !ut.ends_with('\n') {
                    ut.push_str(nl);
                }
                ut.push_str(n);
                ut.push('=');
                ut.push_str(v);
                ut.push_str(nl);
            }
        }
    }
    ut
}

fn skriv_trygt(sti: &Path, bytes: &[u8]) -> Resultat<()> {
    let tmp = sti.with_extension("knoxlife-tmp");
    fs::write(&tmp, bytes)?;
    fs::rename(&tmp, sti)?;
    Ok(())
}

/// Slår ytelsesmodus på før spillet startes. Finnes ikke `options.ini` ennå (spillet er aldri startet), gjøres
/// ingenting: spillet lager filen selv første gang. Gir `true` når verdiene ble skrevet.
pub fn slå_på(options: &Path, tilstand: &Path, verdier: &BTreeMap<String, String>) -> Resultat<bool> {
    // En rest fra forrige gang settes tilbake først, så det som huskes, alltid er spillerens egne verdier.
    sett_tilbake(options, tilstand)?;
    let verdier: BTreeMap<String, String> =
        verdier.iter().filter(|(n, v)| gyldig_navn(n) && gyldig_verdi(v)).map(|(n, v)| (n.clone(), v.clone())).collect();
    let Ok(tekst) = fs::read_to_string(options) else {
        return Ok(false);
    };
    if verdier.is_empty() {
        return Ok(false);
    }
    let før = verdier.keys().map(|n| (n.clone(), les(&tekst, n))).collect();
    let husket = Husket { skrevet: verdier.clone(), før };
    fs::create_dir_all(tilstand)?;
    skriv_trygt(&tilstand.join(HUSKET_FIL), &serde_json::to_vec_pretty(&husket)?)?;
    let endringer = verdier.into_iter().map(|(n, v)| (n, Some(v))).collect();
    if let Err(e) = skriv_trygt(options, flett(&tekst, &endringer).as_bytes()) {
        let _ = fs::remove_file(tilstand.join(HUSKET_FIL));
        return Err(e);
    }
    Ok(true)
}

/// Setter spillerens egne verdier tilbake etter spillet – og etter en launcher som ble stoppet midt i. Bare verdiene
/// som fortsatt er de vi skrev; endret spilleren en av dem i spillet, står spillerens valg. Gir `true` når noe ble
/// husket fra før.
pub fn sett_tilbake(options: &Path, tilstand: &Path) -> Resultat<bool> {
    let husket_sti = tilstand.join(HUSKET_FIL);
    let Ok(bytes) = fs::read(&husket_sti) else {
        return Ok(false);
    };
    let Ok(husket) = serde_json::from_slice::<Husket>(&bytes) else {
        // En ødelagt fil kan ikke settes tilbake fra; den tas bort så den ikke stopper neste gang.
        fs::remove_file(&husket_sti)?;
        return Ok(true);
    };
    if let Ok(tekst) = fs::read_to_string(options) {
        let endringer: BTreeMap<String, Option<String>> = husket
            .skrevet
            .iter()
            .filter(|(n, v)| gyldig_navn(n) && les(&tekst, n).as_deref() == Some(v.as_str()))
            .map(|(n, _)| (n.clone(), husket.før.get(n).cloned().flatten()))
            .collect();
        if !endringer.is_empty() {
            let ny = flett(&tekst, &endringer);
            if ny != tekst {
                skriv_trygt(options, ny.as_bytes())?;
            }
        }
    }
    fs::remove_file(&husket_sti)?;
    Ok(true)
}

#[cfg(test)]
mod prøver {
    use super::*;
    use crate::prøvehjelp::Mappe;

    fn endr(par: &[(&str, Option<&str>)]) -> BTreeMap<String, Option<String>> {
        par.iter().map(|(n, v)| (n.to_string(), v.map(str::to_string))).collect()
    }

    fn verdier(par: &[(&str, &str)]) -> BTreeMap<String, String> {
        par.iter().map(|(n, v)| (n.to_string(), v.to_string())).collect()
    }

    #[test]
    fn profilen_er_gyldig_og_uten_skjermen_og_lyden() {
        let v = verdiene(None);
        assert_eq!(v.len(), PROFIL.len());
        assert!(v.len() <= FLEST);
        for (n, verdi) in &v {
            assert!(gyldig_navn(n) && gyldig_verdi(verdi), "{n}={verdi}");
        }
        for n in ["width", "height", "fullScreen", "frameRate", "vsync", "soundVolume", "voiceEnable", "fontSize"] {
            assert!(!v.contains_key(n), "{n} er spillerens eget valg");
        }
        assert_eq!(verdiene(Some(&verdier(&[("water", "1")]))), verdier(&[("water", "1")]));
    }

    #[test]
    fn flettingen_rører_bare_verdiene_og_beholder_linjeskiftene() {
        let tekst = "version=8\r\nwater=0\r\nzoom=true\r\n";
        assert_eq!(flett(tekst, &endr(&[("water", Some("2"))])), "version=8\r\nwater=2\r\nzoom=true\r\n");
        assert_eq!(flett(tekst, &endr(&[("water", None)])), "version=8\r\nzoom=true\r\n");
        assert_eq!(flett(tekst, &endr(&[("puddles", Some("2"))])), "version=8\r\nwater=0\r\nzoom=true\r\npuddles=2\r\n");
        assert_eq!(flett("version=8", &endr(&[("puddles", Some("2"))])), "version=8\npuddles=2\n");
        // Et navn som bare begynner likt, røres ikke.
        assert_eq!(flett("waterX=1\n", &endr(&[("water", Some("2"))])), "waterX=1\nwater=2\n");
        assert_eq!(les("a=1\r\nwater=0\r\n", "water").as_deref(), Some("0"));
        assert_eq!(les("a=1\n", "water"), None);
    }

    #[test]
    fn navnene_og_verdiene_kan_ikke_lage_nye_linjer() {
        assert!(gyldig_navn("bPerfReflections"));
        assert!(!gyldig_navn("water\nfullScreen"));
        assert!(!gyldig_navn("a=b"));
        assert!(!gyldig_navn(""));
        assert!(gyldig_verdi("0.5") && gyldig_verdi("-1") && gyldig_verdi("false"));
        assert!(!gyldig_verdi("2\nfullScreen=true"));
        assert!(!gyldig_verdi(""));
    }

    #[test]
    fn på_og_tilbake_gir_filen_slik_den_var() {
        let m = Mappe::ny();
        let (options, tilstand) = (m.sti().join("Zomboid").join("options.ini"), m.sti().join("launcher"));
        fs::create_dir_all(options.parent().unwrap()).unwrap();
        let før = "version=8\r\nwater=0\r\nperfSkybox=1\r\nwidth=1920\r\n";
        fs::write(&options, før).unwrap();
        assert!(slå_på(&options, &tilstand, &verdier(&[("water", "2"), ("perfSkybox", "2"), ("puddles", "2")])).unwrap());
        let på = fs::read_to_string(&options).unwrap();
        assert_eq!(på, "version=8\r\nwater=2\r\nperfSkybox=2\r\nwidth=1920\r\npuddles=2\r\n");
        assert!(sett_tilbake(&options, &tilstand).unwrap());
        assert_eq!(fs::read_to_string(&options).unwrap(), før);
        // Ingenting husket: ingenting å gjøre.
        assert!(!sett_tilbake(&options, &tilstand).unwrap());
    }

    #[test]
    fn spillerens_egne_endringer_i_spillet_står() {
        let m = Mappe::ny();
        let (options, tilstand) = (m.sti().join("options.ini"), m.sti().join("launcher"));
        fs::write(&options, "version=8\nwater=0\nperfSkybox=1\n").unwrap();
        slå_på(&options, &tilstand, &verdier(&[("water", "2"), ("perfSkybox", "2"), ("puddles", "2")])).unwrap();
        // Spillet skriver hele filen når det avsluttes – her med vannet spilleren valgte selv og en ny linje.
        fs::write(&options, "version=8\nwater=1\nperfSkybox=2\npuddles=2\nzoom=true\n").unwrap();
        sett_tilbake(&options, &tilstand).unwrap();
        assert_eq!(fs::read_to_string(&options).unwrap(), "version=8\nwater=1\nperfSkybox=1\nzoom=true\n");
    }

    #[test]
    fn uten_options_ini_gjøres_ingenting() {
        let m = Mappe::ny();
        let (options, tilstand) = (m.sti().join("options.ini"), m.sti().join("launcher"));
        assert!(!slå_på(&options, &tilstand, &verdiene(None)).unwrap());
        assert!(!options.exists());
        assert!(!tilstand.join(HUSKET_FIL).exists());
    }

    #[test]
    fn en_rest_fra_forrige_gang_settes_tilbake_først() {
        let m = Mappe::ny();
        let (options, tilstand) = (m.sti().join("options.ini"), m.sti().join("launcher"));
        fs::write(&options, "water=0\n").unwrap();
        slå_på(&options, &tilstand, &verdier(&[("water", "2")])).unwrap();
        // Launcheren ble stoppet: neste start husker spillerens 0, ikke vår 2.
        slå_på(&options, &tilstand, &verdier(&[("water", "1")])).unwrap();
        assert_eq!(fs::read_to_string(&options).unwrap(), "water=1\n");
        sett_tilbake(&options, &tilstand).unwrap();
        assert_eq!(fs::read_to_string(&options).unwrap(), "water=0\n");
        // En ødelagt fil stopper ingenting.
        fs::write(tilstand.join(HUSKET_FIL), b"{").unwrap();
        assert!(sett_tilbake(&options, &tilstand).unwrap());
        assert!(!tilstand.join(HUSKET_FIL).exists());
    }

    #[test]
    fn valget_huskes() {
        let m = Mappe::ny();
        assert!(!valgt(m.sti()));
        velg(m.sti(), true).unwrap();
        assert!(valgt(m.sti()));
        velg(m.sti(), false).unwrap();
        assert!(!valgt(m.sti()));
    }
}
