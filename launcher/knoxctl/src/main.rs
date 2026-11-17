//! `knoxctl` – launcheren uten vindu. Hvert steg for seg, til prøver på en ekte PC og til feilsøking.
//!
//! ```text
//! knoxctl finn                                  hvor spillet og launcherens mappe ligger
//! knoxctl status                                er patchen lagt inn nå?
//! knoxctl jar-sha                               SHA-256 av spillets projectzomboid.jar
//! knoxctl legg-inn <agent.jar> <bygg>           legg inn patchen med denne agenten
//! knoxctl sett-tilbake                          sett spillet tilbake
//! knoxctl sjekk-manifest <json> <sig> [nøkkel]  sjekk signaturen og innholdet
//! knoxctl kjører                                går spillet nå?
//! ```
//!
//! `--spill <mappe>` velger spillmappen i stedet for å lete (`steamapps/common/ProjectZomboid`).

use knoxlife_kjerne::manifest::{self, Manifest};
use knoxlife_kjerne::patch::{self, Patch};
use knoxlife_kjerne::spillet::{self, Plattform, Spill};
use knoxlife_kjerne::{porten, start, Resultat};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

fn hjem() -> PathBuf {
    std::env::home_dir().unwrap_or_else(|| PathBuf::from("."))
}

fn tilstand() -> PathBuf {
    spillet::launcherens_mappe(
        Plattform::denne(),
        &hjem(),
        std::env::var_os("LOCALAPPDATA").map(PathBuf::from),
        std::env::var_os("XDG_DATA_HOME").map(PathBuf::from),
    )
}

fn spillet(valgt: Option<&Path>) -> Resultat<Spill> {
    let plattform = Plattform::denne();
    match valgt {
        Some(rot) => {
            let s = Spill::ny(rot, plattform);
            if s.finnes() {
                Ok(s)
            } else {
                Err(knoxlife_kjerne::Feil::SpilletIkkeFunnet)
            }
        }
        None => spillet::finn_spillet(&spillet::steam_mapper_her(&hjem()), plattform),
    }
}

fn kjør(args: &[String]) -> Resultat<()> {
    let mut valgt_spill = None;
    let mut rest = Vec::new();
    let mut it = args.iter();
    while let Some(a) = it.next() {
        if a == "--spill" {
            valgt_spill = it.next().map(PathBuf::from);
        } else {
            rest.push(a.as_str());
        }
    }
    let spill = || spillet(valgt_spill.as_deref());

    match rest.as_slice() {
        ["finn"] => {
            let s = spill()?;
            println!("plattform: {:?}", s.plattform);
            println!("spillet:   {}", s.rot.display());
            println!("jar:       {}", s.jar().display());
            println!("endres:    {}", s.innstillingsfil().display());
            println!("launcher:  {}", tilstand().display());
            println!("zomboid:   {}", start::knox_mappe(&hjem()).display());
        }
        ["status"] => {
            let s = spill()?;
            match patch::les_merke(&tilstand())? {
                None => println!("ikke lagt inn"),
                Some(m) => {
                    let nå = patch::er_lagt_inn(&s, &tilstand())?;
                    println!("merket står: bygg {}, fil {}", m.bygg, m.fil.display());
                    println!("{}", if nå { "lagt inn" } else { "merket står, men filen er endret – kjør sett-tilbake" });
                }
            }
        }
        ["jar-sha"] => println!("{}", porten::sha256_av_fil(&spill()?.jar())?),
        ["legg-inn", agent, bygg] => {
            let s = spill()?;
            let agent = std::fs::canonicalize(agent)?;
            let bygg: u64 = bygg.parse().map_err(|_| std::io::Error::other("bygg må være et tall"))?;
            let stempel = start::stempel_sti(&start::knox_mappe(&hjem()));
            if !knoxlife_kjerne::sti::kan_leses_av_java(&agent) {
                return Err(std::io::Error::other("stien til agenten kan ikke leses av Java på denne PC-en").into());
            }
            patch::legg_inn(&s, &tilstand(), &Patch { agent: &agent, bygg, stempel: &stempel })?;
            println!("lagt inn: bygg {bygg} i {}", s.innstillingsfil().display());
        }
        ["sett-tilbake"] => {
            let s = spill()?;
            if patch::sett_tilbake(&s, &tilstand())? {
                println!("satt tilbake");
            } else {
                println!("ingenting å sette tilbake");
            }
        }
        ["sjekk-manifest", json, sig, rest @ ..] => {
            let nøkkel = match rest.first() {
                Some(hex) => manifest::nøkkel_fra_hex(hex),
                None => manifest::innebygd_nøkkel(),
            }
            .ok_or_else(|| std::io::Error::other("ingen gyldig offentlig nøkkel"))?;
            let m = Manifest::les_signert(&std::fs::read(json)?, &std::fs::read(sig)?, &nøkkel)?;
            m.sjekk_bygg(manifest::sist_bygg(&tilstand()))?;
            println!("gyldig: bygg {} for spillet {} ({} filer)", m.bygg, m.spillet.versjon, m.filer.len());
            if let Ok(s) = spill() {
                match porten::sjekk(&s, &m) {
                    Ok(svar) => println!("porten: {svar:?}"),
                    Err(e) => println!("porten: {e}"),
                }
            }
        }
        ["kjører"] => println!("{}", start::spillet_kjører(Plattform::denne())),
        _ => {
            eprintln!("{}", include_str!("bruk.txt"));
            return Err(std::io::Error::other("ukjent kommando").into());
        }
    }
    Ok(())
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match kjør(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{e}");
            ExitCode::FAILURE
        }
    }
}
