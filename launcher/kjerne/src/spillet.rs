//! Finne Steam og Project Zomboid, og hvor filene ligger på hver plattform.
//!
//! Spillet ligger i `<bibliotek>/steamapps/common/<installdir>`, der `installdir` står i
//! `appmanifest_108600.acf`. Inni den mappen ligger Java-delen ulikt:
//! - Windows: rett i mappen (`ProjectZomboid64.json`, `projectzomboid.jar`)
//! - Linux: i undermappen `projectzomboid/`
//! - Mac: i app-pakken, `Project Zomboid.app/Contents/Java/`, og innstillingene for Java i
//!   `Project Zomboid.app/Contents/Info.plist` – Mac har ingen `ProjectZomboid64.json`.

use crate::feil::{Feil, Resultat};
use crate::vdf;
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};

/// Steams nummer for Project Zomboid (spillet, ikke den dedikerte serveren, som er 380870).
pub const APP_ID: u32 = 108600;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Plattform {
    Windows,
    Linux,
    Mac,
}

impl Plattform {
    pub fn denne() -> Plattform {
        if cfg!(target_os = "windows") {
            Plattform::Windows
        } else if cfg!(target_os = "macos") {
            Plattform::Mac
        } else {
            Plattform::Linux
        }
    }
}

/// Et installert spill.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Spill {
    /// `<bibliotek>/steamapps/common/<installdir>`
    pub rot: PathBuf,
    pub plattform: Plattform,
}

impl Spill {
    pub fn ny(rot: impl Into<PathBuf>, plattform: Plattform) -> Spill {
        Spill { rot: rot.into(), plattform }
    }

    /// Mappen der `projectzomboid.jar` ligger.
    pub fn java_mappe(&self) -> PathBuf {
        match self.plattform {
            Plattform::Windows => self.rot.clone(),
            Plattform::Linux => self.rot.join("projectzomboid"),
            Plattform::Mac => self.app_pakke().join("Contents").join("Java"),
        }
    }

    pub fn jar(&self) -> PathBuf {
        self.java_mappe().join("projectzomboid.jar")
    }

    /// `ProjectZomboid64.json` – finnes ikke på Mac.
    pub fn json(&self) -> Option<PathBuf> {
        match self.plattform {
            Plattform::Mac => None,
            _ => Some(self.java_mappe().join("ProjectZomboid64.json")),
        }
    }

    pub fn app_pakke(&self) -> PathBuf {
        self.rot.join("Project Zomboid.app")
    }

    /// `Info.plist` – bare på Mac.
    pub fn info_plist(&self) -> Option<PathBuf> {
        match self.plattform {
            Plattform::Mac => Some(self.app_pakke().join("Contents").join("Info.plist")),
            _ => None,
        }
    }

    /// Filen patchen endrer på denne plattformen.
    pub fn innstillingsfil(&self) -> PathBuf {
        self.json().or_else(|| self.info_plist()).expect("hver plattform har én av dem")
    }

    /// Ser det ut som et ekte spill her?
    pub fn finnes(&self) -> bool {
        self.jar().is_file() && self.innstillingsfil().is_file()
    }
}

/// Stedene Steam pleier å ligge, i den rekkefølgen de prøves. På Windows leses registeret først (`steam_fra_registeret`).
pub fn steam_kandidater(plattform: Plattform, hjem: &Path) -> Vec<PathBuf> {
    match plattform {
        Plattform::Windows => vec![
            PathBuf::from(r"C:\Program Files (x86)\Steam"),
            PathBuf::from(r"C:\Program Files\Steam"),
        ],
        Plattform::Linux => vec![
            hjem.join(".steam").join("steam"),
            hjem.join(".local").join("share").join("Steam"),
            hjem.join(".steam").join("root"),
            // Flatpak-utgaven av Steam
            hjem.join(".var").join("app").join("com.valvesoftware.Steam").join(".local").join("share").join("Steam"),
            // Snap-utgaven
            hjem.join("snap").join("steam").join("common").join(".local").join("share").join("Steam"),
        ],
        Plattform::Mac => vec![hjem.join("Library").join("Application Support").join("Steam")],
    }
}

/// Bibliotekene Steam kjenner (`steamapps/libraryfolders.vdf`), med Steams egen mappe først.
pub fn bibliotekene(steam: &Path) -> Vec<PathBuf> {
    let mut ut = vec![steam.to_path_buf()];
    let fil = steam.join("steamapps").join("libraryfolders.vdf");
    if let Ok(tekst) = fs::read_to_string(&fil) {
        if let Some(rot) = vdf::les(&tekst) {
            if let Some(mapper) = rot.blokk("libraryfolders") {
                for (_, b) in mapper.blokker() {
                    if let Some(sti) = b.tekst("path") {
                        let sti = PathBuf::from(sti);
                        if !ut.iter().any(|s| samme_sti(s, &sti)) {
                            ut.push(sti);
                        }
                    }
                }
            }
        }
    }
    ut
}

fn samme_sti(a: &Path, b: &Path) -> bool {
    let norm = |p: &Path| p.to_string_lossy().replace('\\', "/").trim_end_matches('/').to_lowercase();
    norm(a) == norm(b)
}

/// Spillet i ett bibliotek, hvis det er installert der.
pub fn spillet_i(bibliotek: &Path, plattform: Plattform) -> Option<Spill> {
    let manifest = bibliotek.join("steamapps").join(format!("appmanifest_{APP_ID}.acf"));
    let tekst = fs::read_to_string(manifest).ok()?;
    let rot = vdf::les(&tekst)?;
    let installdir = rot.blokk("appstate")?.tekst("installdir")?;
    let spill = Spill::ny(bibliotek.join("steamapps").join("common").join(installdir), plattform);
    spill.finnes().then_some(spill)
}

/// Leter etter spillet i alle bibliotekene til den første Steam-mappen som finnes.
pub fn finn_spillet(steam_mapper: &[PathBuf], plattform: Plattform) -> Resultat<Spill> {
    let mut fant_steam = false;
    for steam in steam_mapper {
        if !steam.join("steamapps").is_dir() {
            continue;
        }
        fant_steam = true;
        for bibliotek in bibliotekene(steam) {
            if let Some(spill) = spillet_i(&bibliotek, plattform) {
                return Ok(spill);
            }
        }
    }
    Err(if fant_steam { Feil::SpilletIkkeFunnet } else { Feil::SteamIkkeFunnet })
}

/// Spillet i en mappe spilleren har valgt med «Choose folder»: spillets egen mappe, en mappe rett inni den (Linux:
/// `projectzomboid`, Mac: `Project Zomboid.app`), et Steam-bibliotek eller Steams egen mappe.
pub fn spillet_i_mappe(mappe: &Path, plattform: Plattform) -> Option<Spill> {
    let mut kandidater = vec![mappe.to_path_buf()];
    kandidater.extend(mappe.parent().map(Path::to_path_buf));
    kandidater
        .into_iter()
        .map(|k| Spill::ny(k, plattform))
        .find(Spill::finnes)
        .or_else(|| finn_spillet(&[mappe.to_path_buf()], plattform).ok())
}

/// Filen i launcherens mappe med spillmappen spilleren valgte.
const VALGT_MAPPE_FIL: &str = "spillmappe.txt";

/// Spillet i mappen spilleren valgte sist – bare hvis det fortsatt ligger der.
pub fn valgt_spill(tilstand: &Path, plattform: Plattform) -> Option<Spill> {
    let tekst = fs::read_to_string(tilstand.join(VALGT_MAPPE_FIL)).ok()?;
    let spill = Spill::ny(tekst.trim_end_matches(['\r', '\n']), plattform);
    spill.finnes().then_some(spill)
}

/// Husker mappen spilleren valgte. En sti som ikke er gyldig UTF-8, huskes ikke (spillet finnes likevel denne gangen).
pub fn husk_valgt_spill(tilstand: &Path, spill: &Spill) -> Resultat<()> {
    let Some(sti) = spill.rot.to_str() else { return Ok(()) };
    fs::create_dir_all(tilstand)?;
    let tmp = tilstand.join(format!("{VALGT_MAPPE_FIL}.tmp"));
    fs::write(&tmp, sti)?;
    fs::rename(tmp, tilstand.join(VALGT_MAPPE_FIL))?;
    Ok(())
}

/// Steams mappe fra registeret på Windows (`HKCU\Software\Valve\Steam\SteamPath`). `None` andre steder.
#[cfg(windows)]
pub fn steam_fra_registeret() -> Option<PathBuf> {
    // Leses med `reg query` for å slippe en egen avhengighet til registeret; svaret er én linje med «REG_SZ».
    let ut = std::process::Command::new("reg")
        .args(["query", r"HKCU\Software\Valve\Steam", "/v", "SteamPath"])
        .output()
        .ok()?;
    let tekst = String::from_utf8_lossy(&ut.stdout);
    let linje = tekst.lines().find(|l| l.contains("SteamPath"))?;
    let verdi = linje.split("REG_SZ").nth(1)?.trim();
    (!verdi.is_empty()).then(|| PathBuf::from(verdi.replace('/', "\\")))
}

#[cfg(not(windows))]
pub fn steam_fra_registeret() -> Option<PathBuf> {
    None
}

/// Launcherens egen mappe: kopien, merket, patchene og bygget sist sett. Aldri i spillmappen.
/// - Windows: `%LOCALAPPDATA%\KnoxLife`
/// - Linux: `$XDG_DATA_HOME/KnoxLife`, ellers `~/.local/share/KnoxLife`
/// - Mac: `~/Library/Application Support/KnoxLife`
pub fn launcherens_mappe(plattform: Plattform, hjem: &Path, local_appdata: Option<PathBuf>, xdg_data: Option<PathBuf>) -> PathBuf {
    match plattform {
        Plattform::Windows => local_appdata.unwrap_or_else(|| hjem.join("AppData").join("Local")).join("KnoxLife"),
        Plattform::Linux => xdg_data
            .filter(|p| p.is_absolute())
            .unwrap_or_else(|| hjem.join(".local").join("share"))
            .join("KnoxLife"),
        Plattform::Mac => hjem.join("Library").join("Application Support").join("KnoxLife"),
    }
}

/// Steam-kontoen som er logget inn sist på denne PC-en (`config/loginusers.vdf`): Steam-ID-en og navnet spilleren har i
/// Steam. Launcheren viser navnet og kopierer ID-en under «Details» – ingenting av det sendes noe sted.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SteamBruker {
    pub id: String,
    pub navn: String,
}

pub fn steam_brukeren(steam_mapper: &[PathBuf]) -> Option<SteamBruker> {
    steam_mapper.iter().find_map(|steam| {
        let tekst = fs::read_to_string(steam.join("config").join("loginusers.vdf")).ok()?;
        let rot = vdf::les(&tekst)?;
        let brukere = rot.blokk("users")?;
        let tid = |b: &vdf::Blokk| b.tekst("Timestamp").and_then(|t| t.parse::<u64>().ok()).unwrap_or(0);
        let (id, b) = brukere
            .blokker()
            .filter(|(id, _)| id.len() == 17 && id.bytes().all(|c| c.is_ascii_digit()))
            .max_by_key(|(_, b)| (b.tekst("MostRecent") == Some("1"), tid(b)))?;
        Some(SteamBruker { id: id.to_string(), navn: b.tekst("PersonaName").unwrap_or_default().to_string() })
    })
}

/// Alle stedene Steam kan ligge på denne PC-en, registeret først på Windows.
pub fn steam_mapper_her(hjem: &Path) -> Vec<PathBuf> {
    let plattform = Plattform::denne();
    let mut ut: Vec<PathBuf> = steam_fra_registeret().into_iter().collect();
    for k in steam_kandidater(plattform, hjem) {
        if !ut.iter().any(|s| samme_sti(s, &k)) {
            ut.push(k);
        }
    }
    ut
}

#[cfg(test)]
mod prøver {
    use super::*;

    #[test]
    fn steam_brukeren_er_den_sist_innlogget() {
        let m = crate::prøvehjelp::Mappe::ny();
        let steam = m.sti().join("Steam");
        fs::create_dir_all(steam.join("config")).unwrap();
        fs::write(
            steam.join("config").join("loginusers.vdf"),
            "\"users\"\n{\n\t\"76561190000000001\"\n\t{\n\t\t\"AccountName\"\t\t\"a\"\n\t\t\"PersonaName\"\t\t\"Gammel\"\n\t\t\"MostRecent\"\t\t\"0\"\n\t\t\"Timestamp\"\t\t\"1700000000\"\n\t}\n\t\"76561190000000002\"\n\t{\n\t\t\"AccountName\"\t\t\"b\"\n\t\t\"PersonaName\"\t\t\"Bjørn\"\n\t\t\"mostrecent\"\t\t\"1\"\n\t\t\"Timestamp\"\t\t\"1600000000\"\n\t}\n}\n",
        )
        .unwrap();
        let tom = m.sti().join("ingenting");
        let b = steam_brukeren(&[tom, steam]).unwrap();
        assert_eq!(b, SteamBruker { id: "76561190000000002".into(), navn: "Bjørn".into() });
        assert_eq!(steam_brukeren(&[m.sti().join("ingenting")]), None);
    }
    use crate::prøvehjelp::Mappe;

    fn lag_spill(bibliotek: &Path, plattform: Plattform) -> Spill {
        let steamapps = bibliotek.join("steamapps");
        fs::create_dir_all(&steamapps).unwrap();
        fs::write(
            steamapps.join("appmanifest_108600.acf"),
            "\"AppState\"\n{\n\t\"appid\"\t\t\"108600\"\n\t\"installdir\"\t\t\"ProjectZomboid\"\n}\n",
        )
        .unwrap();
        let spill = Spill::ny(steamapps.join("common").join("ProjectZomboid"), plattform);
        fs::create_dir_all(spill.java_mappe()).unwrap();
        fs::write(spill.jar(), b"jar").unwrap();
        let fil = spill.innstillingsfil();
        fs::create_dir_all(fil.parent().unwrap()).unwrap();
        fs::write(fil, b"{}").unwrap();
        spill
    }

    #[test]
    fn stiene_på_hver_plattform() {
        let w = Spill::ny("/s/ProjectZomboid", Plattform::Windows);
        assert_eq!(w.jar(), PathBuf::from("/s/ProjectZomboid/projectzomboid.jar"));
        assert_eq!(w.json().unwrap(), PathBuf::from("/s/ProjectZomboid/ProjectZomboid64.json"));
        let l = Spill::ny("/s/ProjectZomboid", Plattform::Linux);
        assert_eq!(l.json().unwrap(), PathBuf::from("/s/ProjectZomboid/projectzomboid/ProjectZomboid64.json"));
        let m = Spill::ny("/s/ProjectZomboid", Plattform::Mac);
        assert_eq!(m.json(), None);
        assert_eq!(m.jar(), PathBuf::from("/s/ProjectZomboid/Project Zomboid.app/Contents/Java/projectzomboid.jar"));
        assert_eq!(m.innstillingsfil(), PathBuf::from("/s/ProjectZomboid/Project Zomboid.app/Contents/Info.plist"));
    }

    #[test]
    fn finner_spillet_i_et_annet_bibliotek() {
        let m = Mappe::ny();
        let steam = m.sti().join("Steam");
        let annet = m.sti().join("Bibliotek 2");
        fs::create_dir_all(steam.join("steamapps")).unwrap();
        fs::write(
            steam.join("steamapps").join("libraryfolders.vdf"),
            format!(
                "\"libraryfolders\" {{ \"0\" {{ \"path\" \"{}\" }} \"1\" {{ \"path\" \"{}\" }} }}",
                steam.display().to_string().replace('\\', "\\\\"),
                annet.display().to_string().replace('\\', "\\\\")
            ),
        )
        .unwrap();
        let forventet = lag_spill(&annet, Plattform::Linux);
        let funnet = finn_spillet(&[m.sti().join("finnes-ikke"), steam], Plattform::Linux).unwrap();
        assert_eq!(funnet, forventet);
    }

    #[test]
    fn steam_uten_spillet_og_uten_steam() {
        let m = Mappe::ny();
        let steam = m.sti().join("Steam");
        fs::create_dir_all(steam.join("steamapps")).unwrap();
        assert!(matches!(finn_spillet(&[steam], Plattform::Windows), Err(Feil::SpilletIkkeFunnet)));
        assert!(matches!(finn_spillet(&[m.sti().join("nei")], Plattform::Windows), Err(Feil::SteamIkkeFunnet)));
    }

    #[test]
    fn halv_installasjon_regnes_ikke() {
        let m = Mappe::ny();
        let spill = lag_spill(m.sti(), Plattform::Mac);
        fs::remove_file(spill.jar()).unwrap();
        assert!(spillet_i(m.sti(), Plattform::Mac).is_none());
    }

    #[test]
    fn launcherens_mappe_på_hver_plattform() {
        let h = Path::new("/h");
        assert_eq!(
            launcherens_mappe(Plattform::Windows, h, Some(PathBuf::from("/h/AppData/Local")), None),
            PathBuf::from("/h/AppData/Local/KnoxLife")
        );
        assert_eq!(launcherens_mappe(Plattform::Linux, h, None, None), PathBuf::from("/h/.local/share/KnoxLife"));
        assert_eq!(
            launcherens_mappe(Plattform::Linux, h, None, Some(PathBuf::from("relativ"))),
            PathBuf::from("/h/.local/share/KnoxLife"),
            "en relativ XDG-sti brukes ikke"
        );
        assert_eq!(
            launcherens_mappe(Plattform::Mac, h, None, None),
            PathBuf::from("/h/Library/Application Support/KnoxLife")
        );
    }

    #[test]
    fn valgt_mappe_finner_spillet_og_huskes() {
        let m = Mappe::ny();
        let bibliotek = m.sti().join("Spill");
        let spill = lag_spill(&bibliotek, Plattform::Linux);
        assert_eq!(spillet_i_mappe(&spill.rot, Plattform::Linux), Some(spill.clone()), "spillets egen mappe");
        assert_eq!(spillet_i_mappe(&spill.java_mappe(), Plattform::Linux), Some(spill.clone()), "en mappe inni spillet");
        assert_eq!(spillet_i_mappe(&bibliotek, Plattform::Linux), Some(spill.clone()), "Steam-biblioteket");
        assert_eq!(spillet_i_mappe(&m.sti().join("tom"), Plattform::Linux), None);

        let tilstand = m.sti().join("KnoxLife");
        assert_eq!(valgt_spill(&tilstand, Plattform::Linux), None);
        husk_valgt_spill(&tilstand, &spill).unwrap();
        assert_eq!(valgt_spill(&tilstand, Plattform::Linux), Some(spill.clone()));
        fs::remove_file(spill.jar()).unwrap();
        assert_eq!(valgt_spill(&tilstand, Plattform::Linux), None, "mappen huskes bare mens spillet ligger der");
    }

    #[test]
    fn flatpak_er_med_på_linux() {
        let k = steam_kandidater(Plattform::Linux, Path::new("/home/a"));
        assert!(k.iter().any(|p| p.to_string_lossy().contains("com.valvesoftware.Steam")));
    }
}
