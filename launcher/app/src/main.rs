//! Knox Life-launcheren med vindu (Tauri). Alt launcheren gjør med spillet, ligger i kjernen (`knoxlife-kjerne`); her
//! er bare vinduet, nettet og maskinen: Steam, prosessene og klokken.
//!
//! Vinduet (`ui/`) kaller kommandoene under og får hvert steg i «Play» som hendelsen `steg`.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod nett;

use knoxlife_kjerne::ko::{self, Kosvar, Serverstatus};
use knoxlife_kjerne::manifest::{self, Manifest, Nyhet};
use knoxlife_kjerne::okt::{self, Maskin, Nett, Oppsett, Steg, Utfall};
use knoxlife_kjerne::porten::{self, Svar};
use knoxlife_kjerne::spillet::{self, Plattform, Spill};
use knoxlife_kjerne::{patch, rapport, start, ytelsesmodus, Feil, Resultat};
use serde::Serialize;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_clipboard_manager::ClipboardExt;
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_opener::OpenerExt;

const VERSJON: &str = env!("CARGO_PKG_VERSION");

/// Det launcheren husker mens den er åpen.
#[derive(Default)]
struct Launcher {
    /// Feilen vinduet viste sist – med i rapporten.
    siste_feil: Mutex<Option<String>>,
    /// «Play» er trykket og økten er ikke ferdig.
    i_gang: AtomicBool,
    /// Adressen til serverens status og køen fra det siste signerte manifestet, og om det sa nødmodus.
    server: Mutex<(Option<String>, bool)>,
}

fn hjem() -> PathBuf {
    std::env::home_dir().unwrap_or_else(|| PathBuf::from("."))
}

/// Launcherens egen mappe (merket, kopien og patchene).
fn tilstand() -> PathBuf {
    spillet::launcherens_mappe(
        Plattform::denne(),
        &hjem(),
        std::env::var_os("LOCALAPPDATA").map(PathBuf::from),
        std::env::var_os("XDG_DATA_HOME").map(PathBuf::from),
    )
}

/// Mappen alle Windows-brukere kan lese, til agenten når stien til launcherens mappe ikke kan leses av Java.
fn reserve() -> Option<PathBuf> {
    match Plattform::denne() {
        Plattform::Windows => std::env::var_os("PUBLIC").map(|p| PathBuf::from(p).join("KnoxLife")),
        _ => None,
    }
}

/// Spillet: i mappen spilleren valgte med «Choose folder», ellers der Steam har det.
fn finn_spillet() -> Resultat<Spill> {
    match spillet::valgt_spill(&tilstand(), Plattform::denne()) {
        Some(spill) => Ok(spill),
        None => spillet::finn_spillet(&spillet::steam_mapper_her(&hjem()), Plattform::denne()),
    }
}

// ---------- maskinen ----------

struct EkteMaskin {
    plattform: Plattform,
    /// Steams mappe (Windows: der `steam.exe` ligger).
    steam: Option<PathBuf>,
    /// Linux: `steam` finnes i PATH.
    steam_i_path: bool,
}

impl EkteMaskin {
    fn ny() -> EkteMaskin {
        let plattform = Plattform::denne();
        let steam = match plattform {
            Plattform::Windows => {
                spillet::steam_mapper_her(&hjem()).into_iter().find(|s| s.join("steam.exe").is_file())
            }
            _ => None,
        };
        let steam_i_path = plattform == Plattform::Linux
            && std::env::var_os("PATH")
                .is_some_and(|p| std::env::split_paths(&p).any(|d| d.join("steam").is_file()));
        EkteMaskin { plattform, steam, steam_i_path }
    }
}

impl Maskin for EkteMaskin {
    fn spillet_kjører(&self) -> bool {
        start::spillet_kjører(self.plattform)
    }

    fn start_steam(&self) -> Resultat<()> {
        let (program, argumenter) = start::steam_kommando(self.plattform, self.steam.as_deref(), self.steam_i_path);
        let mut barn = std::process::Command::new(program)
            .args(argumenter)
            .spawn()
            .map_err(|e| Feil::SteamStartetIkke(e.to_string()))?;
        // Steam gir straks fra seg; ventes det ikke på, blir prosessen hengende som en død oppføring på Linux.
        std::thread::spawn(move || {
            let _ = barn.wait();
        });
        Ok(())
    }

    fn vent_ett_sekund(&self) {
        std::thread::sleep(Duration::from_secs(1));
    }
}

// ---------- kommandoene ----------

/// Det vinduet viser med en gang, uten nett.
#[derive(Serialize)]
struct Oversikt {
    launcher: &'static str,
    spillet: bool,
    steam_navn: Option<String>,
}

#[tauri::command]
fn oversikt() -> Oversikt {
    let steam = spillet::steam_brukeren(&spillet::steam_mapper_her(&hjem()));
    Oversikt { launcher: VERSJON, spillet: finn_spillet().is_ok(), steam_navn: steam.map(|s| s.navn) }
}

/// Serverens oppdatering slik launcheren ser den: hentet og sjekket, uten at noe legges inn.
#[derive(Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
enum Status {
    /// Patchen passer spillet: «Play» virker.
    Klar { spillversjon: String, bygg: u64, nyheter: Vec<Nyhet> },
    /// Nødmodus (3.9 punkt 9).
    Nodmodus { spillversjon: String, bygg: u64, nyheter: Vec<Nyhet> },
    /// Spillet hos spilleren er ikke det patchen er laget for (en ny versjon av spillet).
    VenterPaSpillet { spillversjon: String, bygg: u64, nyheter: Vec<Nyhet> },
    /// Oppdateringen kunne ikke hentes eller sjekkes.
    Feil { melding: String },
}

fn hent_status(server: &Mutex<(Option<String>, bool)>) -> Resultat<Status> {
    let nett = nett::Https::innebygd().ok_or_else(uten_adresse)?;
    let nøkkel = manifest::innebygd_nøkkel().ok_or(Feil::Signatur)?;
    let (json, sig) = nett.manifest()?;
    let m = Manifest::les_signert(&json, &sig, &nøkkel)?;
    m.sjekk_bygg(manifest::sist_bygg(&tilstand()))?;
    *server.lock().unwrap_or_else(|e| e.into_inner()) = (m.status.clone(), m.nødmodus);
    let (spillversjon, bygg, nyheter) = (m.spillet.versjon.clone(), m.bygg, m.nyheter.clone());
    Ok(match finn_spillet().map(|s| porten::sjekk(&s, &m)) {
        Ok(Ok(Svar::Nødmodus)) => Status::Nodmodus { spillversjon, bygg, nyheter },
        Ok(Err(Feil::SpilletOppdatert { .. })) => Status::VenterPaSpillet { spillversjon, bygg, nyheter },
        // Uten spillet, eller med en jar som ikke kan leses, sier «Play» hva som er galt.
        _ => Status::Klar { spillversjon, bygg, nyheter },
    })
}

fn uten_adresse() -> Feil {
    Feil::Nett("this copy of the launcher has no update address".into())
}

#[tauri::command]
async fn status(app: AppHandle) -> Status {
    let svar = tauri::async_runtime::spawn_blocking(move || hent_status(&app.state::<Launcher>().server)).await;
    match svar {
        Ok(Ok(s)) => s,
        Ok(Err(e)) => Status::Feil { melding: e.to_string() },
        Err(e) => Status::Feil { melding: e.to_string() },
    }
}

// ---------- serverens status og køen (launcher.md 3.8 punkt 6) ----------

/// Serverens status for statuslinjen, og om «Play» må stå i køen først.
#[derive(Serialize)]
struct Serverinfo {
    #[serde(flatten)]
    status: Serverstatus,
    /// Serveren er full: «Play» stiller spilleren i køen.
    vent: bool,
}

fn statusnett(launcher: &Launcher) -> Option<(nett::Statusnett, bool)> {
    let (adresse, nødmodus) = launcher.server.lock().unwrap_or_else(|e| e.into_inner()).clone();
    Some((nett::Statusnett::ny(&adresse?), nødmodus))
}

/// Statusen nå, eller ingenting (manifestet har ingen adresse, eller serveren svarte ikke – da starter «Play» som før).
#[tauri::command]
async fn serverstatus(app: AppHandle) -> Option<Serverinfo> {
    tauri::async_runtime::spawn_blocking(move || {
        let (nett, nødmodus) = statusnett(&app.state::<Launcher>())?;
        let status = ko::les_status(&nett.status().ok()?).ok()?;
        let vent = ko::må_stå_i_kø(&status, nødmodus);
        Some(Serverinfo { status, vent })
    })
    .await
    .ok()
    .flatten()
}

fn steam_id() -> Resultat<String> {
    spillet::steam_brukeren(&spillet::steam_mapper_her(&hjem()))
        .map(|b| b.id)
        .ok_or_else(|| Feil::Nett("no Steam account was found on this PC – log in to Steam first".into()))
}

/// Stå i køen: plassen, eller «klar» når det er spillerens tur. Vinduet spør hvert 10. sekund.
#[tauri::command]
async fn ko(app: AppHandle) -> Result<Kosvar, String> {
    tauri::async_runtime::spawn_blocking(move || -> Resultat<Kosvar> {
        let (nett, _) = statusnett(&app.state::<Launcher>()).ok_or_else(|| Feil::Nett("no server status".into()))?;
        ko::les_kosvar(&nett.ko(false, &steam_id()?)?)
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| e.to_string())
}

/// «Leave line».
#[tauri::command]
async fn forlat_ko(app: AppHandle) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || -> Resultat<()> {
        if let Some((nett, _)) = statusnett(&app.state::<Launcher>()) {
            nett.ko(true, &steam_id()?)?;
        }
        Ok(())
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| e.to_string())
}

/// «Play»: hele økten i kjernen. Hvert steg sendes til vinduet som hendelsen `steg`.
#[tauri::command]
async fn spill(app: AppHandle, launcher: State<'_, Launcher>) -> Result<Utfall, String> {
    if launcher.i_gang.swap(true, Ordering::SeqCst) {
        return Err("Knox Life is already starting.".into());
    }
    let app2 = app.clone();
    let svar = tauri::async_runtime::spawn_blocking(move || -> Resultat<Utfall> {
        let spill = finn_spillet()?;
        let nett = nett::Https::innebygd().ok_or_else(uten_adresse)?;
        let nøkkel = manifest::innebygd_nøkkel();
        let (hjem, tilstand, reserve) = (hjem(), tilstand(), reserve());
        let o = Oppsett {
            spill: &spill,
            tilstand: &tilstand,
            hjem: &hjem,
            reserve: reserve.as_deref(),
            nøkkel: nøkkel.as_ref(),
            rett_inn: true,
            ytelsesmodus: ytelsesmodus::valgt(&tilstand),
        };
        okt::kjør(&o, &nett, &EkteMaskin::ny(), &mut |s| {
            let _ = app2.emit("steg", &s);
            if let Some(vindu) = app2.get_webview_window("main") {
                match s {
                    // Mens spillet går, ligger launcheren nede (3.2 punkt 11), og den kommer fram igjen etterpå.
                    Steg::Lastet => {
                        let _ = vindu.minimize();
                    }
                    Steg::IkkeLastet | Steg::SetterTilbake => {
                        let _ = vindu.unminimize();
                    }
                    _ => {}
                }
            }
        })
    })
    .await;
    launcher.i_gang.store(false, Ordering::SeqCst);
    let svar = match svar {
        Ok(s) => s.map_err(|e| e.to_string()),
        Err(e) => Err(e.to_string()),
    };
    *launcher.siste_feil.lock().unwrap_or_else(|e| e.into_inner()) = svar.as_ref().err().cloned();
    svar
}

/// «Choose folder» når spillet ikke ble funnet: `true` når spillet ligger i mappen spilleren valgte (og den huskes),
/// `false` når spilleren lukket valget. Kjøres utenfor vinduets tråd, ellers står vinduet stille mens valget er åpent.
#[tauri::command]
async fn velg_mappe(app: AppHandle) -> Result<bool, String> {
    let Some(valgt) = app.dialog().file().set_title("Choose the Project Zomboid folder").blocking_pick_folder() else {
        return Ok(false);
    };
    let mappe = valgt.into_path().map_err(|e| e.to_string())?;
    let spill = spillet::spillet_i_mappe(&mappe, Plattform::denne()).ok_or(
        "Project Zomboid is not in that folder. Choose the folder Steam installed it in \
         (in Steam: right-click the game, Manage, Browse local files).",
    )?;
    spillet::husk_valgt_spill(&tilstand(), &spill).map_err(|e| e.to_string())?;
    Ok(true)
}

/// «Copy» ved Steam-ID-en under «Details».
#[tauri::command]
fn kopier_steam_id(app: AppHandle) -> Result<(), String> {
    let bruker = spillet::steam_brukeren(&spillet::steam_mapper_her(&hjem()))
        .ok_or("No Steam account was found on this PC. Log in to Steam first.")?;
    app.clipboard().write_text(bruker.id).map_err(|e| e.to_string())
}

/// «Restore game files»: `true` når noe ble satt tilbake.
#[tauri::command]
fn sett_tilbake(launcher: State<'_, Launcher>) -> Result<bool, String> {
    if launcher.i_gang.load(Ordering::SeqCst) || start::spillet_kjører(Plattform::denne()) {
        return Err(Feil::SpilletKjører.to_string());
    }
    let tilstand = tilstand();
    let options = ytelsesmodus::sett_tilbake(&ytelsesmodus::options_sti(&hjem()), &tilstand).map_err(|e| e.to_string())?;
    let spill = finn_spillet().map_err(|e| e.to_string())?;
    Ok(patch::sett_tilbake(&spill, &tilstand).map_err(|e| e.to_string())? || options)
}

/// «Performance mode» under «Details»: står den på?
#[tauri::command]
fn ytelsesmodus_valgt() -> bool {
    ytelsesmodus::valgt(&tilstand())
}

/// Slår «Performance mode» av eller på. Gjelder fra neste «Play».
#[tauri::command]
fn velg_ytelsesmodus(paa: bool) -> Result<(), String> {
    ytelsesmodus::velg(&tilstand(), paa).map_err(|e| e.to_string())
}

/// «Copy report»: rapporten legges på utklippstavlen, og spilleren limer den inn selv.
#[tauri::command]
fn kopier_rapport(app: AppHandle, launcher: State<'_, Launcher>) -> Result<(), String> {
    let spill = finn_spillet().ok();
    let siste = launcher.siste_feil.lock().unwrap_or_else(|e| e.into_inner()).clone();
    let (hjem, tilstand) = (hjem(), tilstand());
    let tekst = rapport::lag(&rapport::Fakta {
        launcher: VERSJON,
        plattform: Plattform::denne(),
        spill: spill.as_ref(),
        tilstand: &tilstand,
        hjem: &hjem,
        siste_feil: siste.as_deref(),
    });
    app.clipboard().write_text(tekst).map_err(|e| e.to_string())
}

/// «Open logs folder»: spillets `Zomboid`-mappe, der `console.txt` ligger.
#[tauri::command]
fn apne_loggmappen(app: AppHandle) -> Result<(), String> {
    let mappe = start::zomboid_mappe(&hjem());
    app.opener().open_path(mappe.to_string_lossy(), None::<&str>).map_err(|e| e.to_string())
}

fn main() {
    // En rest fra forrige gang (launcheren ble drept, PC-en gikk av) settes tilbake straks, når spillet ikke går.
    if !start::spillet_kjører(Plattform::denne()) {
        let _ = ytelsesmodus::sett_tilbake(&ytelsesmodus::options_sti(&hjem()), &tilstand());
        if let Ok(spill) = finn_spillet() {
            let _ = patch::rydd_etter_forrige(&spill, &tilstand());
        }
    }
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _, _| {
            if let Some(vindu) = app.get_webview_window("main") {
                let _ = vindu.unminimize();
                let _ = vindu.set_focus();
            }
        }))
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .manage(Launcher::default())
        .invoke_handler(tauri::generate_handler![
            oversikt,
            status,
            spill,
            velg_mappe,
            kopier_steam_id,
            sett_tilbake,
            ytelsesmodus_valgt,
            velg_ytelsesmodus,
            kopier_rapport,
            apne_loggmappen,
            serverstatus,
            ko,
            forlat_ko
        ])
        .run(tauri::generate_context!())
        .expect("vinduet kunne ikke startes");
}
