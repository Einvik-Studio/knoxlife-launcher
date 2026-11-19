//! Filmene til kinoen (J11): modet skriver hvilke filmer ukens program trenger, og launcheren laster dem ned i
//! bakgrunnen til `Zomboid/KnoxLife/kino/<id>.knf`. Hver fil sjekkes mot størrelsen og SHA-256 i listen, og bare
//! utgivelser i dette repoet godtas som kilde. Filer som ikke står i listen lenger, slettes. Spilleren kan slå det av
//! («Download drive-in films» under «Details»); da er alt annet likt, bare uten bildet på lerretet.

use crate::feil::{Feil, Resultat};
use sha2::{Digest, Sha256};
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

/// Filen modet skriver i `Zomboid/Lua/`, én linje per film: `<id> <sha256> <bytes> <url>`.
pub const ØNSKER: &str = "KnoxLife_kino_onsker.txt";
/// Bare filer herfra lastes ned.
pub const KILDE: &str = "https://github.com/Einvik-Studio/knoxlife-launcher/releases/download/";
/// Største film som godtas (bytes).
pub const STØRSTE: u64 = 2 * 1024 * 1024 * 1024;
/// Flest filmer i listen.
pub const FLEST: usize = 12;
const VALGT_FIL: &str = "kino.txt";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ønske {
    pub id: String,
    pub sha256: String,
    pub bytes: u64,
    pub url: String,
}

fn gyldig_id(id: &str) -> bool {
    (1..=40).contains(&id.len()) && id.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_' || b == b'-')
}

/// Linjene i ønskelisten. Linjer som ikke stemmer med formatet, eller peker et annet sted enn [`KILDE`], hoppes over.
pub fn les_ønsker(tekst: &str) -> Vec<Ønske> {
    let mut ut: Vec<Ønske> = Vec::new();
    for linje in tekst.lines() {
        let deler: Vec<&str> = linje.split_whitespace().collect();
        let [id, sha, bytes, url] = deler[..] else { continue };
        let Ok(bytes) = bytes.parse::<u64>() else { continue };
        let ok = gyldig_id(id)
            && sha.len() == 64
            && sha.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
            && bytes > 0
            && bytes <= STØRSTE
            && url.starts_with(KILDE)
            && url.ends_with(&format!("/{id}.knf"))
            && !url[KILDE.len()..].contains("..")
            && url[KILDE.len()..].bytes().all(|b| b.is_ascii_alphanumeric() || b"/._-".contains(&b));
        if ok && !ut.iter().any(|ø| ø.id == id) && ut.len() < FLEST {
            ut.push(Ønske { id: id.into(), sha256: sha.into(), bytes, url: url.into() });
        }
    }
    ut
}

/// Ønskelisten fra modet, eller ingen når filen ikke finnes.
pub fn ønsker(zomboid: &Path) -> Vec<Ønske> {
    fs::read_to_string(zomboid.join("Lua").join(ØNSKER)).map(|t| les_ønsker(&t)).unwrap_or_default()
}

/// Mappen filmene ligger i.
pub fn mappe(zomboid: &Path) -> PathBuf {
    zomboid.join("KnoxLife").join("kino")
}

/// «Download drive-in films»: står på når spilleren ikke har slått det av.
pub fn valgt(tilstand: &Path) -> bool {
    fs::read_to_string(tilstand.join(VALGT_FIL)).map(|s| s.trim() != "off").unwrap_or(true)
}

pub fn velg(tilstand: &Path, på: bool) -> Resultat<()> {
    fs::create_dir_all(tilstand)?;
    fs::write(tilstand.join(VALGT_FIL), if på { "on\n" } else { "off\n" })?;
    Ok(())
}

/// SHA-256 av en fil, lest i biter.
pub fn sha256_fil(sti: &Path) -> Resultat<String> {
    let mut f = fs::File::open(sti)?;
    let mut h = Sha256::new();
    let mut buf = vec![0u8; 1 << 16];
    loop {
        let n = f.read(&mut buf)?;
        if n == 0 {
            break;
        }
        h.update(&buf[..n]);
    }
    Ok(hex::encode(h.finalize()))
}

/// Filen ligger der, med riktig størrelse og sjekksum. Sjekksummen huskes i `<id>.ok` med størrelsen og tiden, så en
/// film på 400 MB ikke leses på nytt ved hver start.
pub fn ligger_riktig(mappe: &Path, ø: &Ønske) -> bool {
    let sti = mappe.join(format!("{}.knf", ø.id));
    let Ok(m) = fs::metadata(&sti) else { return false };
    if m.len() != ø.bytes {
        return false;
    }
    let tid = m.modified().ok().and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map(|d| d.as_secs());
    let merke = format!("{} {} {}", ø.sha256, ø.bytes, tid.unwrap_or(0));
    let ok_fil = mappe.join(format!("{}.ok", ø.id));
    if fs::read_to_string(&ok_fil).is_ok_and(|s| s.trim() == merke) {
        return true;
    }
    let riktig = sha256_fil(&sti).is_ok_and(|s| s == ø.sha256);
    if riktig {
        let _ = fs::write(&ok_fil, &merke);
    }
    riktig
}

/// Sletter filer i mappen som ikke hører til en film i listen (også rester etter en avbrutt nedlasting).
pub fn rydd(mappe: &Path, ønsker: &[Ønske]) -> Resultat<Vec<String>> {
    let Ok(lest) = fs::read_dir(mappe) else { return Ok(Vec::new()) };
    let mut slettet = Vec::new();
    for e in lest.filter_map(|e| e.ok()) {
        let navn = e.file_name().to_string_lossy().into_owned();
        let id = navn.split('.').next().unwrap_or("");
        let behold = ønsker.iter().any(|ø| ø.id == id) && (navn.ends_with(".knf") || navn.ends_with(".ogg") || navn.ends_with(".ok"));
        if !behold && e.path().is_file() {
            fs::remove_file(e.path())?;
            slettet.push(navn);
        }
    }
    Ok(slettet)
}

/// Henter én film til en fil. Vinduet strømmer over HTTPS; prøvene fra en mappe.
pub trait Henter {
    fn hent_til(&self, ønske: &Ønske, ut: &mut dyn Write) -> Resultat<()>;
}

/// Laster ned filmene som mangler, én om gangen. Gir id-ene som ble lastet ned. En fil som ikke stemmer, slettes, og
/// det prøves igjen neste gang.
pub fn hent(mappe: &Path, ønsker: &[Ønske], henter: &dyn Henter) -> Resultat<Vec<String>> {
    fs::create_dir_all(mappe)?;
    let mut hentet = Vec::new();
    for ø in ønsker {
        if ligger_riktig(mappe, ø) {
            continue;
        }
        let del = mappe.join(format!("{}.knf.del", ø.id));
        let mut f = fs::File::create(&del)?;
        let svar = henter.hent_til(ø, &mut f);
        drop(f);
        let stemmer = svar.is_ok()
            && fs::metadata(&del).is_ok_and(|m| m.len() == ø.bytes)
            && sha256_fil(&del).is_ok_and(|s| s == ø.sha256);
        if !stemmer {
            let _ = fs::remove_file(&del);
            return Err(svar.err().unwrap_or_else(|| Feil::NedlastingØdelagt(format!("{}.knf", ø.id))));
        }
        let _ = fs::remove_file(mappe.join(format!("{}.ogg", ø.id)));
        fs::rename(&del, mappe.join(format!("{}.knf", ø.id)))?;
        hentet.push(ø.id.clone());
    }
    Ok(hentet)
}

#[cfg(test)]
mod prøver {
    use super::*;
    use crate::prøvehjelp::Mappe;

    fn sha(b: &[u8]) -> String {
        hex::encode(Sha256::digest(b))
    }

    fn linje(id: &str, innhold: &[u8]) -> String {
        format!("{id} {} {} {KILDE}kino-1/{id}.knf", sha(innhold), innhold.len())
    }

    struct FraMinne(Vec<(String, Vec<u8>)>);

    impl Henter for FraMinne {
        fn hent_til(&self, ø: &Ønske, ut: &mut dyn Write) -> Resultat<()> {
            let (_, b) = self.0.iter().find(|(id, _)| *id == ø.id).ok_or_else(|| Feil::Nett("404".into()))?;
            ut.write_all(b)?;
            Ok(())
        }
    }

    #[test]
    fn ønskelisten_godtar_bare_egne_utgivelser() {
        let tekst = [
            linje("nld", b"film"),
            "nld feil 4 https://example.com/nld.knf".into(),
            format!("andre {} 4 https://example.com/andre.knf", sha(b"film")),
            format!("rar {} 4 {KILDE}kino-1/annen.knf", sha(b"film")),
            format!("dobbel {} 4 {KILDE}../x/dobbel.knf", sha(b"film")),
            format!("STOR {} 4 {KILDE}kino-1/STOR.knf", sha(b"film")),
            linje("nld", b"igjen"),
            "".into(),
        ]
        .join("\n");
        let l = les_ønsker(&tekst);
        assert_eq!(l.len(), 1);
        assert_eq!(l[0].id, "nld");
        assert_eq!(l[0].bytes, 4);
    }

    #[test]
    fn henter_det_som_mangler_og_sjekker_sjekksummen() {
        let m = Mappe::ny();
        let ø = les_ønsker(&format!("{}\n{}", linje("nld", b"natten"), linje("dod", b"d.o.a.")));
        let henter = FraMinne(vec![("nld".into(), b"natten".to_vec()), ("dod".into(), b"d.o.a.".to_vec())]);
        assert_eq!(hent(m.sti(), &ø, &henter).unwrap(), vec!["nld".to_string(), "dod".to_string()]);
        assert_eq!(fs::read(m.sti().join("nld.knf")).unwrap(), b"natten");
        assert!(hent(m.sti(), &ø, &henter).unwrap().is_empty(), "ligger der alt");
        assert!(m.sti().join("nld.ok").exists());

        // En nedlasting som ikke stemmer, blir ikke liggende.
        let feil = FraMinne(vec![("ny".into(), b"noe annet".to_vec())]);
        let ny = les_ønsker(&linje("ny", b"det riktige"));
        assert!(hent(m.sti(), &ny, &feil).is_err());
        assert!(!m.sti().join("ny.knf").exists() && !m.sti().join("ny.knf.del").exists());
    }

    #[test]
    fn en_endret_fil_lastes_ned_igjen() {
        let m = Mappe::ny();
        let ø = les_ønsker(&linje("nld", b"natten"));
        let henter = FraMinne(vec![("nld".into(), b"natten".to_vec())]);
        hent(m.sti(), &ø, &henter).unwrap();
        fs::write(m.sti().join("nld.knf"), b"nattex").unwrap();
        assert!(!ligger_riktig(m.sti(), &ø[0]));
        assert_eq!(hent(m.sti(), &ø, &henter).unwrap(), vec!["nld".to_string()]);
    }

    #[test]
    fn rydder_det_som_ikke_er_i_listen() {
        let m = Mappe::ny();
        for f in ["nld.knf", "nld.ogg", "nld.ok", "gammel.knf", "gammel.ogg", "nld.knf.del", "annet.txt"] {
            fs::write(m.sti().join(f), b"x").unwrap();
        }
        let ø = les_ønsker(&linje("nld", b"natten"));
        let mut slettet = rydd(m.sti(), &ø).unwrap();
        slettet.sort();
        assert_eq!(slettet, vec!["annet.txt", "gammel.knf", "gammel.ogg", "nld.knf.del"]);
        assert!(m.sti().join("nld.knf").exists());
    }

    #[test]
    fn valget_står_på_til_det_slås_av() {
        let m = Mappe::ny();
        assert!(valgt(m.sti()));
        velg(m.sti(), false).unwrap();
        assert!(!valgt(m.sti()));
        velg(m.sti(), true).unwrap();
        assert!(valgt(m.sti()));
    }
}
