//! Legge inn patchen og sette spillet tilbake.
//!
//! Spillet har ingen måte å laste Java-mods på. Vi legger agenten vår først blant Java-valgene:
//! - Windows og Linux: `-javaagent:<sti>` først i `vmArgs` i `ProjectZomboid64.json`
//! - Mac: agenten kopieres inn i app-pakken (`Contents/Java/knoxlife/`), og `-javaagent:<sti>` står først i
//!   `JVMOptions` i `Contents/Info.plist`
//! - Windows: i tillegg kopieres spillets egen `jre64\bin\jli.dll` ved siden av `ProjectZomboid64.exe`. Java laster
//!   agenten med `instrument.dll`, som trenger `jli.dll` – og Windows leter etter den ved siden av .exe-en, ikke i
//!   `jre64\bin`. Uten kopien stopper spillet med en gang når Steam starter det (prøven på PC-en 18.11.2026).
//!
//! Rekkefølgen er det som gjør det trygt:
//! 1. Står merket igjen fra forrige gang (launcheren ble drept), settes alt tilbake først.
//! 2. Kopien av originalen skrives – renset for linjer fra oss, så en gammel rest aldri blir «originalen».
//! 3. Merket skrives **før** filen endres.
//! 4. Filen endres.
//!
//! Når spillet avsluttes, skrives kopien tilbake byte for byte, filene vi la i app-pakken fjernes, og merket slettes.

use crate::feil::{Feil, Resultat};
use crate::spillet::{Plattform, Spill};
use crate::sti::jvm_sti;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::fs;
use std::io::Cursor;
use std::path::{Path, PathBuf};

/// Navnet på agentens fil. Linjer som peker på en fil med dette navnet, er våre.
pub const AGENT_FIL: &str = "knoxlife-agent.jar";
/// Systemverdiene våre begynner med dette.
pub const VERDI_FORAN: &str = "-Dknoxlife.";
/// Stenger for agenter som kobler seg på etter at spillet har startet – et juksverktøy kan ellers gjøre det
/// underveis. Juksesperren i agenten melder fra hvis valget mangler.
pub const UTEN_DYNAMISKE_AGENTER: &str = "-XX:-EnableDynamicAgentLoading";
const AGENT_FLAGG: &str = "-javaagent:";
const MERKE_FIL: &str = "lagt-inn.json";
const KOPI_MAPPE: &str = "kopi";
const MAC_MAPPE: &str = "knoxlife";
/// Filen `instrument.dll` trenger på Windows.
pub const JLI: &str = "jli.dll";

/// Det som står i merket mens patchen er lagt inn.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Merke {
    /// Filen som ble endret.
    pub fil: PathBuf,
    /// SHA-256 av originalen (kopien).
    pub original: String,
    /// SHA-256 av filen slik vi skrev den – `None` til den er skrevet.
    pub patchet: Option<String>,
    pub bygg: u64,
    /// Filene vi la i app-pakken (bare Mac), så nøyaktig de fjernes igjen.
    pub lagt_til: Vec<PathBuf>,
    /// Mappene vi laget i app-pakken, innerste først.
    pub mapper_laget: Vec<PathBuf>,
    /// Filene vi kopierte inn i spillmappen fra spillets egen Java (Windows: `jli.dll`). De fjernes bare når de
    /// fortsatt er slik vi la dem.
    #[serde(default)]
    pub kopiert: Vec<Kopiert>,
}

/// En fil vi kopierte inn i spillmappen, med SHA-256 av innholdet.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Kopiert {
    pub fil: PathBuf,
    pub sha: String,
}

/// Windows: `jli.dll` fra spillets egen Java, til mappen med `ProjectZomboid64.exe`. `None` når det ikke trengs: på de
/// andre plattformene, når spillet ikke har filen, eller når det alt ligger en der – den er ikke vår og røres ikke.
fn jli_kopi(spill: &Spill) -> Option<(PathBuf, Kopiert)> {
    if spill.plattform != Plattform::Windows {
        return None;
    }
    let fra = spill.rot.join("jre64").join("bin").join(JLI);
    let til = spill.rot.join(JLI);
    if til.exists() {
        return None;
    }
    let bytes = fs::read(&fra).ok()?;
    Some((fra, Kopiert { fil: til, sha: sha256(&bytes) }))
}

/// Det patchen trenger å vite.
#[derive(Debug, Clone)]
pub struct Patch<'a> {
    /// Agenten slik den ligger i launcherens mappe.
    pub agent: &'a Path,
    pub bygg: u64,
    /// Filen agenten skriver stempelet sitt i (3.2 punkt 10 i systemdokumentet).
    pub stempel: &'a Path,
    /// Spillerens hjemmemappe. Stempelet skrives da som `~/…`, og agenten finner hjemmemappen selv – så et brukernavn
    /// med «ø» aldri havner i spillets fil.
    pub hjem: Option<&'a Path>,
    /// En mappe Java alltid kan lese (Windows: `C:\Users\Public\KnoxLife`). Agenten kopieres dit bare når stien til
    /// launcherens mappe har tegn utenfor ASCII og disken ikke har korte 8.3-navn.
    pub reserve: Option<&'a Path>,
}

pub fn sha256(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

fn skriv_trygt(sti: &Path, bytes: &[u8]) -> Resultat<()> {
    let tmp = sti.with_extension("knoxlife-tmp");
    fs::write(&tmp, bytes)?;
    fs::rename(&tmp, sti)?;
    Ok(())
}

fn merke_sti(tilstand: &Path) -> PathBuf {
    tilstand.join(MERKE_FIL)
}

fn kopi_sti(tilstand: &Path, fil: &Path) -> PathBuf {
    tilstand.join(KOPI_MAPPE).join(fil.file_name().unwrap_or_default())
}

pub fn les_merke(tilstand: &Path) -> Resultat<Option<Merke>> {
    match fs::read(merke_sti(tilstand)) {
        Ok(b) => Ok(Some(serde_json::from_slice(&b)?)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e.into()),
    }
}

fn skriv_merke(tilstand: &Path, merke: &Merke) -> Resultat<()> {
    fs::create_dir_all(tilstand)?;
    skriv_trygt(&merke_sti(tilstand), &serde_json::to_vec_pretty(merke)?)
}

/// Er dette en linje vi har lagt inn?
pub fn er_vår(valg: &str) -> bool {
    (valg.starts_with(AGENT_FLAGG) && valg.trim_end_matches(['"', '\'']).ends_with(AGENT_FIL))
        || valg.starts_with(VERDI_FORAN)
        || valg == UTEN_DYNAMISKE_AGENTER
}

fn våre_valg(p: &Patch) -> Resultat<Vec<String>> {
    // Java leser filen i Windows' gamle tegnsett: en sti med tegn utenfor ASCII peker på ingenting, og agenten hoppes
    // over uten at noen sier fra. Da nektes det heller her.
    let stempel = match p.hjem.and_then(|h| p.stempel.strip_prefix(h).ok()) {
        Some(rel) => format!("~/{}", rel.to_string_lossy().replace('\\', "/")),
        None => jvm_sti(p.stempel),
    };
    if !crate::sti::kan_leses_av_java(p.agent) {
        return Err(Feil::StiKanIkkeLeses(p.agent.to_path_buf()));
    }
    if cfg!(windows) && !stempel.is_ascii() {
        return Err(Feil::StiKanIkkeLeses(p.stempel.to_path_buf()));
    }
    Ok(vec![
        format!("{AGENT_FLAGG}{}", jvm_sti(p.agent)),
        format!("{VERDI_FORAN}bygg={}", p.bygg),
        format!("{VERDI_FORAN}stempel={stempel}"),
        UTEN_DYNAMISKE_AGENTER.to_string(),
    ])
}

// ---------- ProjectZomboid64.json ----------

fn json_les(bytes: &[u8], navn: &str) -> Resultat<Value> {
    if bytes.starts_with(&[0xEF, 0xBB, 0xBF]) {
        return Err(Feil::Bom(navn.into()));
    }
    let v: Value = serde_json::from_slice(bytes).map_err(|_| Feil::SpillfilØdelagt(navn.into()))?;
    if !v.get("vmArgs").is_some_and(Value::is_array) {
        return Err(Feil::SpillfilØdelagt(navn.into()));
    }
    Ok(v)
}

fn json_vm_args(v: &mut Value) -> &mut Vec<Value> {
    v.get_mut("vmArgs").and_then(Value::as_array_mut).expect("sjekket i json_les")
}

/// Originalen uten noe fra oss – byte for byte lik det Steam la der når det ikke var noe å fjerne.
pub fn json_renset(bytes: &[u8]) -> Resultat<Vec<u8>> {
    let mut v = json_les(bytes, "ProjectZomboid64.json")?;
    let args = json_vm_args(&mut v);
    let før = args.len();
    args.retain(|a| !a.as_str().is_some_and(er_vår));
    if args.len() == før {
        return Ok(bytes.to_vec());
    }
    Ok((serde_json::to_string_pretty(&v)? + "\n").into_bytes())
}

/// Filen med våre valg først i `vmArgs`. Java leser agentene i rekkefølge, og vår skal først.
pub fn json_med_patch(bytes: &[u8], p: &Patch) -> Resultat<Vec<u8>> {
    let mut v = json_les(bytes, "ProjectZomboid64.json")?;
    let args = json_vm_args(&mut v);
    args.retain(|a| !a.as_str().is_some_and(er_vår));
    for (i, valg) in våre_valg(p)?.into_iter().enumerate() {
        args.insert(i, Value::String(valg));
    }
    Ok((serde_json::to_string_pretty(&v)? + "\n").into_bytes())
}

// ---------- Info.plist (Mac) ----------

fn plist_les(bytes: &[u8]) -> Resultat<plist::Value> {
    let v = plist::Value::from_reader(Cursor::new(bytes)).map_err(|_| Feil::SpillfilØdelagt("Info.plist".into()))?;
    if !v.as_dictionary().and_then(|d| d.get("JVMOptions")).is_some_and(|o| o.as_array().is_some()) {
        return Err(Feil::SpillfilØdelagt("Info.plist".into()));
    }
    Ok(v)
}

fn plist_valg(v: &mut plist::Value) -> &mut Vec<plist::Value> {
    v.as_dictionary_mut()
        .and_then(|d| d.get_mut("JVMOptions"))
        .and_then(plist::Value::as_array_mut)
        .expect("sjekket i plist_les")
}

fn plist_skriv(v: &plist::Value) -> Resultat<Vec<u8>> {
    let mut ut = Vec::new();
    v.to_writer_xml(&mut ut).map_err(|e| Feil::SpillfilØdelagt(format!("Info.plist ({e})")))?;
    ut.push(b'\n');
    Ok(ut)
}

pub fn plist_renset(bytes: &[u8]) -> Resultat<Vec<u8>> {
    let mut v = plist_les(bytes)?;
    let valg = plist_valg(&mut v);
    let før = valg.len();
    valg.retain(|o| !o.as_string().is_some_and(er_vår));
    if valg.len() == før {
        return Ok(bytes.to_vec());
    }
    plist_skriv(&v)
}

pub fn plist_med_patch(bytes: &[u8], p: &Patch) -> Resultat<Vec<u8>> {
    let mut v = plist_les(bytes)?;
    let valg = plist_valg(&mut v);
    valg.retain(|o| !o.as_string().is_some_and(er_vår));
    for (i, linje) in våre_valg(p)?.into_iter().enumerate() {
        valg.insert(i, plist::Value::String(linje));
    }
    plist_skriv(&v)
}

// ---------- felles for begge filene ----------

fn renset(bytes: &[u8], plattform: Plattform) -> Resultat<Vec<u8>> {
    match plattform {
        Plattform::Mac => plist_renset(bytes),
        _ => json_renset(bytes),
    }
}

fn med_patch(bytes: &[u8], p: &Patch, plattform: Plattform) -> Resultat<Vec<u8>> {
    match plattform {
        Plattform::Mac => plist_med_patch(bytes, p),
        _ => json_med_patch(bytes, p),
    }
}

/// Har filen det samme innholdet, selv om mellomrommene er ulike?
fn samme_innhold(a: &[u8], b: &[u8], plattform: Plattform) -> bool {
    match plattform {
        Plattform::Mac => match (plist_les(a), plist_les(b)) {
            (Ok(x), Ok(y)) => x == y,
            _ => false,
        },
        _ => match (serde_json::from_slice::<Value>(a), serde_json::from_slice::<Value>(b)) {
            (Ok(x), Ok(y)) => x == y,
            _ => false,
        },
    }
}

fn inneholder_vårt(bytes: &[u8], plattform: Plattform) -> bool {
    renset(bytes, plattform).map(|r| r != bytes).unwrap_or(false)
}

// ---------- legge inn og sette tilbake ----------

/// Legger inn patchen. `tilstand` er launcherens egen mappe for merket og kopien.
pub fn legg_inn(spill: &Spill, tilstand: &Path, p: &Patch) -> Resultat<()> {
    // 1. En rest fra forrige gang settes tilbake først.
    rydd_etter_forrige(spill, tilstand)?;

    let fil = spill.innstillingsfil();
    let bytes = fs::read(&fil).map_err(|_| Feil::SpillfilØdelagt(filnavn(&fil)))?;
    let original = renset(&bytes, spill.plattform)?;

    // Mac: agenten må ligge inni app-pakken, der spillets egen Java leter. Windows: i reservemappen når Java ikke kan lese
    // stien til launcherens mappe. Ellers brukes den der den ligger.
    let kopi_til = if spill.plattform == Plattform::Mac {
        Some(spill.java_mappe().join(MAC_MAPPE))
    } else if !crate::sti::kan_leses_av_java(p.agent) {
        p.reserve.map(|r| r.join("patch").join(p.bygg.to_string()))
    } else {
        None
    };
    let (agent_i_spillet, lagt_til, mapper_laget) = match kopi_til {
        Some(mappe) => {
            let agent = mappe.join(AGENT_FIL);
            // Mappene vi lager, innerste først, så nøyaktig de fjernes igjen.
            let mut mapper = Vec::new();
            let mut m = mappe.clone();
            while !m.exists() {
                mapper.push(m.clone());
                if !m.pop() {
                    break;
                }
            }
            (agent.clone(), vec![agent], mapper)
        }
        None => (p.agent.to_path_buf(), Vec::new(), Vec::new()),
    };
    let jli = jli_kopi(spill);

    // 2. Kopien av originalen, lest tilbake for å være sikker.
    let kopi = kopi_sti(tilstand, &fil);
    fs::create_dir_all(kopi.parent().expect("kopien har en mappe"))?;
    skriv_trygt(&kopi, &original)?;
    if sha256(&fs::read(&kopi)?) != sha256(&original) {
        return Err(Feil::KunneIkkeSetteTilbake("the backup could not be saved".into()));
    }

    // 3. Merket før noe i spillmappen endres – også før mappen og agenten på Mac.
    let mut merke = Merke {
        fil: fil.clone(),
        original: sha256(&original),
        patchet: None,
        bygg: p.bygg,
        lagt_til,
        mapper_laget,
        kopiert: jli.iter().map(|(_, k)| k.clone()).collect(),
    };
    skriv_merke(tilstand, &merke)?;

    // 4. Endre.
    for mappe in merke.mapper_laget.iter().rev() {
        fs::create_dir_all(mappe)?;
    }
    if agent_i_spillet != p.agent {
        fs::copy(p.agent, &agent_i_spillet)?;
    }
    if let Some((fra, k)) = &jli {
        fs::copy(fra, &k.fil)?;
    }
    let i_spillet = Patch { agent: &agent_i_spillet, ..p.clone() };
    let ny = med_patch(&original, &i_spillet, spill.plattform)?;
    skriv_trygt(&fil, &ny)?;
    merke.patchet = Some(sha256(&ny));
    skriv_merke(tilstand, &merke)?;
    Ok(())
}

/// Setter spillet tilbake hvis merket står der. `false` når det ikke var noe å sette tilbake.
pub fn sett_tilbake(spill: &Spill, tilstand: &Path) -> Resultat<bool> {
    let Some(merke) = les_merke(tilstand)? else {
        return Ok(false);
    };
    let mappe = merke.fil.parent().map(Path::to_path_buf).unwrap_or_default();
    if !mappe.is_dir() {
        // Spillet er flyttet, avinstallert eller ligger på en disk som ikke er koblet til. Merket blir stående, så
        // spillet settes tilbake når det er der igjen.
        return Err(Feil::SpillmappenBorte(mappe));
    }
    let kopi = kopi_sti(tilstand, &merke.fil);
    let original = fs::read(&kopi).map_err(|e| Feil::KunneIkkeSetteTilbake(format!("backup missing: {e}")))?;
    if sha256(&original) != merke.original {
        return Err(Feil::KunneIkkeSetteTilbake("the backup does not match".into()));
    }

    // Filen slik den er nå:
    // - borte, eller nøyaktig slik vi skrev den: originalen skrives tilbake.
    // - endret av noen andre (Steam har bekreftet filene eller oppdatert spillet) men med linjer fra oss: bare
    //   linjene våre fjernes, så endringen deres står.
    // - uten noe fra oss: den røres ikke.
    match fs::read(&merke.fil) {
        Err(_) => skriv_trygt(&merke.fil, &original)?,
        Ok(nå) if merke.patchet.as_deref() == Some(sha256(&nå).as_str()) => skriv_trygt(&merke.fil, &original)?,
        Ok(nå) if inneholder_vårt(&nå, spill.plattform) => {
            let uten = renset(&nå, spill.plattform)?;
            let ut = if samme_innhold(&uten, &original, spill.plattform) { original.clone() } else { uten };
            skriv_trygt(&merke.fil, &ut)?;
        }
        Ok(_) => {}
    }
    for fil in &merke.lagt_til {
        match fs::remove_file(fil) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(e.into()),
        }
    }
    for mappe in &merke.mapper_laget {
        let _ = fs::remove_dir(mappe); // bare hvis den er tom
    }
    for k in &merke.kopiert {
        // Er filen byttet ut mens spillet gikk (en oppdatering av spillet), er den ikke lenger vår.
        if fs::read(&k.fil).map(|b| sha256(&b) == k.sha).unwrap_or(false) {
            fs::remove_file(&k.fil)?;
        }
    }
    fs::remove_file(merke_sti(tilstand))?;
    Ok(true)
}

/// Første steg ved hver start: står merket igjen, settes spillet tilbake før noe annet.
pub fn rydd_etter_forrige(spill: &Spill, tilstand: &Path) -> Resultat<bool> {
    sett_tilbake(spill, tilstand)
}

/// Er patchen lagt inn nå (merket står, og filen er slik vi skrev den)?
pub fn er_lagt_inn(spill: &Spill, tilstand: &Path) -> Resultat<bool> {
    let Some(merke) = les_merke(tilstand)? else {
        return Ok(false);
    };
    let Ok(nå) = fs::read(spill.innstillingsfil()) else {
        return Ok(false);
    };
    Ok(merke.patchet.as_deref() == Some(sha256(&nå).as_str()))
}

fn filnavn(p: &Path) -> String {
    p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()
}

#[cfg(test)]
mod prøver {
    use super::*;
    use crate::prøvehjelp::Mappe;

    /// Slik Steam legger filen på Windows (forkortet).
    const JSON: &str = "{\n\t\"mainClass\": \"zombie/gameStates/MainScreenState\",\n\t\"classpath\": [\n\t\t\"projectzomboid.jar\"\n\t],\n\t\"vmArgs\": [\n\t\t\"-Djava.awt.headless=true\",\n\t\t\"-Xmx3072m\",\n\t\t\"-XX:+UseZGC\"\n\t],\n\t\"windows\": {\n\t\t\"vmArgs\": []\n\t}\n}\n";

    const PLIST: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
	<key>CFBundleExecutable</key>
	<string>JavaAppLauncher</string>
	<key>JVMOptions</key>
	<array>
		<string>-Xmx3072m</string>
		<string>-XstartOnFirstThread</string>
	</array>
</dict>
</plist>
"#;

    struct Oppsett {
        _m: Mappe,
        spill: Spill,
        tilstand: PathBuf,
        agent: PathBuf,
        stempel: PathBuf,
    }

    fn oppsett(plattform: Plattform) -> Oppsett {
        let m = Mappe::ny();
        let spill = Spill::ny(m.sti().join("ProjectZomboid"), plattform);
        fs::create_dir_all(spill.java_mappe()).unwrap();
        fs::write(spill.jar(), b"jar").unwrap();
        let fil = spill.innstillingsfil();
        fs::create_dir_all(fil.parent().unwrap()).unwrap();
        fs::write(&fil, if plattform == Plattform::Mac { PLIST } else { JSON }).unwrap();
        let tilstand = m.sti().join("KnoxLife");
        let agent = m.sti().join("KnoxLife").join("patch").join("12").join(AGENT_FIL);
        fs::create_dir_all(agent.parent().unwrap()).unwrap();
        fs::write(&agent, b"agenten").unwrap();
        let stempel = m.sti().join("Zomboid").join("KnoxLife").join("stempel.txt");
        Oppsett { _m: m, spill, tilstand, agent, stempel }
    }

    impl Oppsett {
        fn patch(&self) -> Patch<'_> {
            Patch { agent: &self.agent, bygg: 12, stempel: &self.stempel, hjem: None, reserve: None }
        }
        fn fil(&self) -> Vec<u8> {
            fs::read(self.spill.innstillingsfil()).unwrap()
        }
    }

    fn vm_args(bytes: &[u8]) -> Vec<String> {
        let v: Value = serde_json::from_slice(bytes).unwrap();
        v["vmArgs"].as_array().unwrap().iter().map(|a| a.as_str().unwrap().to_string()).collect()
    }

    fn jvm_options(bytes: &[u8]) -> Vec<String> {
        let v = plist::Value::from_reader(Cursor::new(bytes)).unwrap();
        v.as_dictionary().unwrap()["JVMOptions"].as_array().unwrap().iter().map(|o| o.as_string().unwrap().to_string()).collect()
    }

    #[test]
    fn json_legges_inn_først_og_settes_tilbake_byte_for_byte() {
        for plattform in [Plattform::Windows, Plattform::Linux] {
            let o = oppsett(plattform);
            legg_inn(&o.spill, &o.tilstand, &o.patch()).unwrap();
            let args = vm_args(&o.fil());
            assert!(args[0].starts_with("-javaagent:") && args[0].ends_with(AGENT_FIL), "{args:?}");
            assert_eq!(args[1], "-Dknoxlife.bygg=12");
            assert!(args[2].starts_with("-Dknoxlife.stempel="));
            assert_eq!(args[3], UTEN_DYNAMISKE_AGENTER);
            assert_eq!(&args[4..], ["-Djava.awt.headless=true", "-Xmx3072m", "-XX:+UseZGC"]);
            assert!(!args[0].contains('\\'), "Java får skråstrek fremover");
            assert!(er_lagt_inn(&o.spill, &o.tilstand).unwrap());
            // Rekkefølgen på nøklene er den samme som Steam skrev.
            let tekst = String::from_utf8(o.fil()).unwrap();
            assert!(tekst.find("mainClass").unwrap() < tekst.find("windows").unwrap());

            assert!(sett_tilbake(&o.spill, &o.tilstand).unwrap());
            assert_eq!(o.fil(), JSON.as_bytes());
            assert!(!er_lagt_inn(&o.spill, &o.tilstand).unwrap());
            assert!(!sett_tilbake(&o.spill, &o.tilstand).unwrap(), "andre gang er det ingenting å gjøre");
        }
    }

    #[test]
    fn stempelet_skrives_fra_hjemmemappen_så_brukernavnet_ikke_står_i_filen() {
        let o = oppsett(Plattform::Windows);
        let hjem = o.stempel.parent().unwrap().parent().unwrap().parent().unwrap().to_path_buf();
        legg_inn(&o.spill, &o.tilstand, &Patch { hjem: Some(&hjem), ..o.patch() }).unwrap();
        assert_eq!(vm_args(&o.fil())[2], "-Dknoxlife.stempel=~/Zomboid/KnoxLife/stempel.txt");
        sett_tilbake(&o.spill, &o.tilstand).unwrap();
        assert_eq!(o.fil(), JSON.as_bytes());
    }

    #[test]
    fn mac_legger_agenten_i_app_pakken_og_fjerner_den_igjen() {
        let o = oppsett(Plattform::Mac);
        legg_inn(&o.spill, &o.tilstand, &o.patch()).unwrap();
        let i_pakken = o.spill.java_mappe().join(MAC_MAPPE).join(AGENT_FIL);
        assert_eq!(fs::read(&i_pakken).unwrap(), b"agenten");
        let valg = jvm_options(&o.fil());
        assert_eq!(valg[0], format!("-javaagent:{}", jvm_sti(&i_pakken)));
        assert_eq!(&valg[4..], ["-Xmx3072m", "-XstartOnFirstThread"]);

        sett_tilbake(&o.spill, &o.tilstand).unwrap();
        assert_eq!(o.fil(), PLIST.as_bytes());
        assert!(!i_pakken.exists());
        assert!(!o.spill.java_mappe().join(MAC_MAPPE).exists(), "mappen vi laget, er borte");
    }

    #[test]
    fn launcheren_drept_midt_i_økten_ryddes_ved_neste_start() {
        let o = oppsett(Plattform::Windows);
        legg_inn(&o.spill, &o.tilstand, &o.patch()).unwrap();
        // Launcheren dør her. Neste start legger inn på nytt – uten at resten blir «originalen».
        legg_inn(&o.spill, &o.tilstand, &Patch { bygg: 13, ..o.patch() }).unwrap();
        let args = vm_args(&o.fil());
        assert_eq!(args.iter().filter(|a| er_vår(a)).count(), 4);
        assert_eq!(args[1], "-Dknoxlife.bygg=13");
        sett_tilbake(&o.spill, &o.tilstand).unwrap();
        assert_eq!(o.fil(), JSON.as_bytes());
    }

    #[test]
    fn rester_uten_merke_blir_ikke_originalen() {
        // Merket er borte (noen slettet launcherens mappe), men linjene våre står i filen.
        let o = oppsett(Plattform::Linux);
        legg_inn(&o.spill, &o.tilstand, &o.patch()).unwrap();
        fs::remove_dir_all(&o.tilstand).unwrap();
        fs::create_dir_all(o.agent.parent().unwrap()).unwrap();
        fs::write(&o.agent, b"agenten").unwrap();
        legg_inn(&o.spill, &o.tilstand, &o.patch()).unwrap();
        sett_tilbake(&o.spill, &o.tilstand).unwrap();
        let args = vm_args(&o.fil());
        assert_eq!(args, ["-Djava.awt.headless=true", "-Xmx3072m", "-XX:+UseZGC"]);
    }

    #[test]
    fn steam_har_skrevet_filen_på_nytt_mens_spillet_gikk() {
        let o = oppsett(Plattform::Windows);
        legg_inn(&o.spill, &o.tilstand, &o.patch()).unwrap();
        // Steam bekreftet filene: originalen fra Steam, med en ny linje, og ingenting fra oss.
        let ny = JSON.replace("-Xmx3072m", "-Xmx4096m");
        fs::write(o.spill.innstillingsfil(), &ny).unwrap();
        sett_tilbake(&o.spill, &o.tilstand).unwrap();
        assert_eq!(o.fil(), ny.as_bytes(), "Steams nye fil røres ikke");
    }

    #[test]
    fn noen_andre_har_endret_filen_med_linjene_våre_i() {
        let o = oppsett(Plattform::Windows);
        legg_inn(&o.spill, &o.tilstand, &o.patch()).unwrap();
        let endret = String::from_utf8(o.fil()).unwrap().replace("-Xmx3072m", "-Xmx6144m");
        fs::write(o.spill.innstillingsfil(), &endret).unwrap();
        sett_tilbake(&o.spill, &o.tilstand).unwrap();
        assert_eq!(vm_args(&o.fil()), ["-Djava.awt.headless=true", "-Xmx6144m", "-XX:+UseZGC"]);
    }

    #[test]
    fn bom_nektes_og_ingenting_endres() {
        let o = oppsett(Plattform::Windows);
        let mut med_bom = vec![0xEF, 0xBB, 0xBF];
        med_bom.extend_from_slice(JSON.as_bytes());
        fs::write(o.spill.innstillingsfil(), &med_bom).unwrap();
        assert!(matches!(legg_inn(&o.spill, &o.tilstand, &o.patch()), Err(Feil::Bom(_))));
        assert_eq!(o.fil(), med_bom);
        assert_eq!(les_merke(&o.tilstand).unwrap(), None);
    }

    #[test]
    fn ødelagt_fil_nektes() {
        let o = oppsett(Plattform::Windows);
        fs::write(o.spill.innstillingsfil(), b"{ \"vmArgs\": ").unwrap();
        assert!(matches!(legg_inn(&o.spill, &o.tilstand, &o.patch()), Err(Feil::SpillfilØdelagt(_))));
        fs::write(o.spill.innstillingsfil(), b"{ \"vmArgs\": \"ikke en liste\" }").unwrap();
        assert!(matches!(legg_inn(&o.spill, &o.tilstand, &o.patch()), Err(Feil::SpillfilØdelagt(_))));
    }

    #[test]
    fn spillmappen_borte_lar_merket_stå() {
        let o = oppsett(Plattform::Windows);
        legg_inn(&o.spill, &o.tilstand, &o.patch()).unwrap();
        fs::remove_dir_all(&o.spill.rot).unwrap();
        assert!(matches!(sett_tilbake(&o.spill, &o.tilstand), Err(Feil::SpillmappenBorte(_))));
        assert!(les_merke(&o.tilstand).unwrap().is_some());
    }

    #[test]
    fn kopien_som_ikke_stemmer_brukes_ikke() {
        let o = oppsett(Plattform::Windows);
        legg_inn(&o.spill, &o.tilstand, &o.patch()).unwrap();
        fs::write(kopi_sti(&o.tilstand, &o.spill.innstillingsfil()), b"{}").unwrap();
        assert!(matches!(sett_tilbake(&o.spill, &o.tilstand), Err(Feil::KunneIkkeSetteTilbake(_))));
    }

    #[test]
    fn andres_agenter_er_ikke_våre() {
        assert!(er_vår("-javaagent:C:/x/knoxlife-agent.jar"));
        assert!(er_vår("-Dknoxlife.bygg=3"));
        assert!(er_vår("-XX:-EnableDynamicAgentLoading"));
        assert!(!er_vår("-XX:+EnableDynamicAgentLoading"));
        assert!(!er_vår("-javaagent:C:/x/plz-agent.jar"));
        assert!(!er_vår("-javaagent:C:/x/knoxlife-agent.jar.bak"));
        assert!(!er_vår("-Xmx3072m"));
    }
    fn med_jli(o: &Oppsett) -> PathBuf {
        let jli = o.spill.rot.join("jre64").join("bin").join(JLI);
        fs::create_dir_all(jli.parent().unwrap()).unwrap();
        fs::write(&jli, b"jli fra spillet").unwrap();
        o.spill.rot.join(JLI)
    }

    #[test]
    fn windows_legger_jli_ved_siden_av_spillet_og_fjerner_den_igjen() {
        let o = oppsett(Plattform::Windows);
        let ved_siden = med_jli(&o);
        legg_inn(&o.spill, &o.tilstand, &o.patch()).unwrap();
        assert_eq!(fs::read(&ved_siden).unwrap(), b"jli fra spillet");
        // Launcheren dør og starter igjen: kopien ryddes og legges inn på nytt.
        legg_inn(&o.spill, &o.tilstand, &o.patch()).unwrap();
        assert!(ved_siden.exists());
        sett_tilbake(&o.spill, &o.tilstand).unwrap();
        assert!(!ved_siden.exists());
        assert!(o.spill.rot.join("jre64").join("bin").join(JLI).exists(), "spillets egen står");
    }

    #[test]
    fn jli_som_ikke_er_vår_røres_ikke() {
        let o = oppsett(Plattform::Windows);
        let ved_siden = med_jli(&o);
        fs::write(&ved_siden, b"noen andres").unwrap();
        legg_inn(&o.spill, &o.tilstand, &o.patch()).unwrap();
        sett_tilbake(&o.spill, &o.tilstand).unwrap();
        assert_eq!(fs::read(&ved_siden).unwrap(), b"noen andres");

        // Byttet ut mens spillet gikk (en oppdatering): blir stående.
        fs::remove_file(&ved_siden).unwrap();
        legg_inn(&o.spill, &o.tilstand, &o.patch()).unwrap();
        fs::write(&ved_siden, b"fra en oppdatering").unwrap();
        sett_tilbake(&o.spill, &o.tilstand).unwrap();
        assert_eq!(fs::read(&ved_siden).unwrap(), b"fra en oppdatering");
    }

    #[test]
    fn jli_bare_på_windows_og_gamle_merker_leses() {
        let o = oppsett(Plattform::Linux);
        let ved_siden = med_jli(&o);
        legg_inn(&o.spill, &o.tilstand, &o.patch()).unwrap();
        assert!(!ved_siden.exists());
        // Et merke fra før feltet fantes.
        let mut v: Value = serde_json::from_slice(&fs::read(merke_sti(&o.tilstand)).unwrap()).unwrap();
        v.as_object_mut().unwrap().remove("kopiert");
        fs::write(merke_sti(&o.tilstand), serde_json::to_vec(&v).unwrap()).unwrap();
        assert!(sett_tilbake(&o.spill, &o.tilstand).unwrap());
    }
}
