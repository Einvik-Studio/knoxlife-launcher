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
//!
//! Utgivelsen i GitHub (`.github/workflows/utgivelse.yml`, `launcher.md` 3.10) bruker de tre siste kommandoene i
//! `bruk.txt`: `lag-manifest`, `nodmodus` og `signer`. Nøkkelen til `signer` står bare i miljøet
//! (`KNOXLIFE_SIGNERINGSNOKKEL`), aldri i argumentene, så den ikke vises i listen over prosesser eller i loggen.

use knoxlife_kjerne::manifest::{self, Manifest};
use knoxlife_kjerne::patch::{self, Patch};
use knoxlife_kjerne::spillet::{self, Plattform, Spill};
use knoxlife_kjerne::utgivelse::{self, Utgave};
use knoxlife_kjerne::{porten, start, Resultat};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

/// Miljøvariabelen med signeringsnøkkelen (32 bytes i hex) – bare i GitHubs miljø «utgivelse».
const NØKKEL_MILJØ: &str = "KNOXLIFE_SIGNERINGSNOKKEL";

fn ugyldig(tekst: impl Into<String>) -> knoxlife_kjerne::Feil {
    std::io::Error::other(tekst.into()).into()
}

/// `--navn verdi`-parene etter kommandoen. Et navn kan stå flere ganger (`--fil`).
struct Valg<'a>(Vec<(&'a str, &'a str)>);

impl<'a> Valg<'a> {
    fn les(args: &[&'a str]) -> Resultat<Valg<'a>> {
        let mut par = Vec::new();
        let mut it = args.iter();
        while let Some(a) = it.next() {
            let Some(navn) = a.strip_prefix("--") else {
                return Err(ugyldig(format!("ventet --navn, fikk {a}")));
            };
            let verdi = it.next().ok_or_else(|| ugyldig(format!("--{navn} mangler verdien")))?;
            par.push((navn, *verdi));
        }
        Ok(Valg(par))
    }

    fn en(&self, navn: &str) -> Option<&'a str> {
        self.0.iter().rev().find(|(n, _)| *n == navn).map(|(_, v)| *v)
    }

    fn må(&self, navn: &str) -> Resultat<&'a str> {
        self.en(navn).ok_or_else(|| ugyldig(format!("--{navn} mangler")))
    }

    fn alle(&self, navn: &str) -> Vec<&'a str> {
        self.0.iter().filter(|(n, _)| *n == navn).map(|(_, v)| *v).collect()
    }

    fn bygg(&self) -> Resultat<u64> {
        self.må("bygg")?.parse().map_err(|_| ugyldig("--bygg må være et tall"))
    }

    /// Den forrige utgaven: `manifest.json` og `manifest.sig` i mappen `--forrige`, sjekket med `--nokkel`.
    fn forrige(&self) -> Resultat<Option<Manifest>> {
        let Some(mappe) = self.en("forrige") else {
            return Ok(None);
        };
        let nøkkel = manifest::nøkkel_fra_hex(self.må("nokkel")?).ok_or_else(|| ugyldig("--nokkel er ikke en gyldig nøkkel"))?;
        let mappe = Path::new(mappe);
        let m = Manifest::les_signert(&std::fs::read(mappe.join("manifest.json"))?, &std::fs::read(mappe.join("manifest.sig"))?, &nøkkel)?;
        Ok(Some(m))
    }
}

fn skriv_manifest(ut: &str, m: &Manifest) -> Resultat<()> {
    std::fs::write(ut, utgivelse::json(m)?)?;
    println!("{ut}: bygg {} for spillet {} ({} filer, nødmodus {})", m.bygg, m.spillet.versjon, m.filer.len(), m.nødmodus);
    Ok(())
}

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
            let hjem = hjem();
            let reserve = std::env::var_os("PUBLIC").map(|p| PathBuf::from(p).join("KnoxLife"));
            let p = Patch { agent: &agent, bygg, stempel: &stempel, hjem: Some(&hjem), reserve: reserve.as_deref() };
            patch::legg_inn(&s, &tilstand(), &p)?;
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
        ["lag-manifest", ut, valg @ ..] => {
            let v = Valg::les(valg)?;
            let forrige = v.forrige()?;
            let filer: Vec<PathBuf> = v.alle("fil").into_iter().map(PathBuf::from).collect();
            let filer: Vec<&Path> = filer.iter().map(PathBuf::as_path).collect();
            let nyhet = v.en("nyhet").map(|n| n.split_once('|').unwrap_or((n, "")));
            let m = utgivelse::lag(&Utgave {
                bygg: v.bygg()?,
                utgitt: v.må("utgitt")?,
                versjon: v.må("versjon")?,
                spillets_jar_sha256: v.må("spillet-sha256")?,
                filer: &filer,
                server: v.må("server")?,
                testserver: v.en("testserver").filter(|s| !s.is_empty()),
                status: v.en("status").filter(|s| !s.is_empty()),
                forrige: forrige.as_ref(),
                nyhet: nyhet.filter(|(tittel, _)| !tittel.trim().is_empty()),
            })?;
            skriv_manifest(ut, &m)?;
        }
        ["nodmodus", på_av, ut, valg @ ..] => {
            let på = match *på_av {
                "på" | "pa" => true,
                "av" => false,
                _ => return Err(ugyldig("nodmodus tar «på» eller «av»")),
            };
            let v = Valg::les(valg)?;
            let forrige = v.forrige()?.ok_or_else(|| ugyldig("--forrige mangler"))?;
            skriv_manifest(ut, &utgivelse::nødmodus(&forrige, på, v.bygg()?, v.må("utgitt")?)?)?;
        }
        ["signer", json] => {
            let nøkkel = std::env::var(NØKKEL_MILJØ).map_err(|_| ugyldig(format!("{NØKKEL_MILJØ} er ikke satt")))?;
            let (sig, offentlig) = utgivelse::signer(&std::fs::read(json)?, &nøkkel)?;
            let sti = Path::new(json).with_file_name("manifest.sig");
            std::fs::write(&sti, sig)?;
            println!("signert: {}", sti.display());
            println!("offentlig nøkkel: {offentlig}");
        }
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
