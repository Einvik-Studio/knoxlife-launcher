//! Filene i patchen i launcherens egen mappe (`…/KnoxLife/patch/<bygg>/`) – aldri i spillmappen.
//!
//! Hver fil sjekkes mot størrelsen og SHA-256 i manifestet. En fil som ikke stemmer, slettes og hentes én gang til; stemmer
//! den fortsatt ikke, stopper launcheren med «The download was damaged». De to siste byggene beholdes, så en spiller kan
//! starte med en gang selv om nettet er tregt rett etter en ny utgave.

use crate::feil::{Feil, Resultat};
use crate::manifest::{trygt_filnavn, Fil, Manifest};
use crate::patch::sha256;
use std::fs;
use std::path::{Path, PathBuf};

/// Hvor mange bygg som beholdes.
pub const BEHOLD: usize = 2;

/// Henter én fil i patchen. Vinduet henter over nett; prøvene og `knoxctl` fra en mappe.
pub trait Henter {
    fn hent(&self, manifest: &Manifest, fil: &Fil) -> Resultat<Vec<u8>>;
}

/// Henter fra en mappe på disken (`knoxctl` og prøvene).
pub struct FraMappe(pub PathBuf);

impl Henter for FraMappe {
    fn hent(&self, _: &Manifest, fil: &Fil) -> Resultat<Vec<u8>> {
        Ok(fs::read(self.0.join(&fil.navn))?)
    }
}

pub fn patch_mappe(rot: &Path, bygg: u64) -> PathBuf {
    rot.join("patch").join(bygg.to_string())
}

/// Stemmer bytene med filen i manifestet?
pub fn stemmer(fil: &Fil, bytes: &[u8]) -> bool {
    bytes.len() as u64 == fil.størrelse && sha256(bytes) == fil.sha256
}

fn ligger_riktig(mappe: &Path, fil: &Fil) -> bool {
    let sti = mappe.join(&fil.navn);
    match fs::metadata(&sti) {
        Ok(m) if m.len() == fil.størrelse => fs::read(&sti).map(|b| sha256(&b) == fil.sha256).unwrap_or(false),
        _ => false,
    }
}

/// Filene som mangler eller ikke stemmer. De som ikke stemmer, slettes.
pub fn mangler<'m>(manifest: &'m Manifest, mappe: &Path) -> Vec<&'m Fil> {
    manifest
        .filer
        .iter()
        .filter(|f| {
            let ok = ligger_riktig(mappe, f);
            if !ok {
                let _ = fs::remove_file(mappe.join(&f.navn));
            }
            !ok
        })
        .collect()
}

/// Henter det som mangler, til patchen ligger helt og riktig i mappen. Gir mappen.
pub fn hent_det_som_mangler(manifest: &Manifest, rot: &Path, henter: &dyn Henter) -> Resultat<PathBuf> {
    let mappe = patch_mappe(rot, manifest.bygg);
    fs::create_dir_all(&mappe)?;
    for fil in mangler(manifest, &mappe) {
        assert!(trygt_filnavn(&fil.navn), "sjekket i Manifest::sjekk");
        let mut forsøk = 0;
        loop {
            forsøk += 1;
            let bytes = henter.hent(manifest, fil);
            match bytes {
                Ok(b) if stemmer(fil, &b) => {
                    let tmp = mappe.join(format!("{}.del", fil.navn));
                    fs::write(&tmp, &b)?;
                    fs::rename(&tmp, mappe.join(&fil.navn))?;
                    break;
                }
                _ if forsøk < 2 => continue,
                Ok(_) => return Err(Feil::NedlastingØdelagt(fil.navn.clone())),
                Err(e) => return Err(Feil::NedlastingØdelagt(format!("{}: {e}", fil.navn))),
            }
        }
    }
    // Til slutt én sjekk av alt, så ingenting halvt ligger igjen.
    if let Some(f) = mangler(manifest, &mappe).first() {
        return Err(Feil::NedlastingØdelagt(f.navn.clone()));
    }
    Ok(mappe)
}

/// Sletter gamle bygg og beholder de [`BEHOLD`] nyeste. `i_bruk` slettes aldri (patchen som er lagt inn nå).
pub fn rydd_gamle(rot: &Path, i_bruk: Option<u64>) -> Resultat<Vec<u64>> {
    let patch = rot.join("patch");
    let Ok(lest) = fs::read_dir(&patch) else {
        return Ok(Vec::new());
    };
    let mut bygg: Vec<u64> = lest
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_dir())
        .filter_map(|e| e.file_name().to_str()?.parse().ok())
        .collect();
    bygg.sort_unstable_by(|a, b| b.cmp(a));
    let mut slettet = Vec::new();
    for b in bygg.into_iter().skip(BEHOLD) {
        if Some(b) == i_bruk {
            continue;
        }
        fs::remove_dir_all(patch.join(b.to_string()))?;
        slettet.push(b);
    }
    Ok(slettet)
}

#[cfg(test)]
mod prøver {
    use super::*;
    use crate::manifest::prøver::eksempel;
    use crate::prøvehjelp::Mappe;
    use std::cell::Cell;

    fn med_fil(bytes: &[u8]) -> Manifest {
        let mut m = eksempel(5);
        m.filer[0].sha256 = sha256(bytes);
        m.filer[0].størrelse = bytes.len() as u64;
        m
    }

    struct Teller<'a> {
        svar: Vec<&'a [u8]>,
        kall: Cell<usize>,
    }

    impl Henter for Teller<'_> {
        fn hent(&self, _: &Manifest, _: &Fil) -> Resultat<Vec<u8>> {
            let i = self.kall.get();
            self.kall.set(i + 1);
            Ok(self.svar[i.min(self.svar.len() - 1)].to_vec())
        }
    }

    #[test]
    fn henter_og_lar_riktige_filer_ligge() {
        let rot = Mappe::ny();
        let m = med_fil(b"agenten");
        let h = Teller { svar: vec![b"agenten"], kall: Cell::new(0) };
        let mappe = hent_det_som_mangler(&m, rot.sti(), &h).unwrap();
        assert_eq!(fs::read(mappe.join(crate::patch::AGENT_FIL)).unwrap(), b"agenten");
        hent_det_som_mangler(&m, rot.sti(), &h).unwrap();
        assert_eq!(h.kall.get(), 1, "en fil som stemmer, hentes ikke på nytt");
    }

    #[test]
    fn ødelagt_fil_hentes_én_gang_til() {
        let rot = Mappe::ny();
        let m = med_fil(b"agenten");
        let h = Teller { svar: vec![b"agentem", b"agenten"], kall: Cell::new(0) };
        hent_det_som_mangler(&m, rot.sti(), &h).unwrap();
        assert_eq!(h.kall.get(), 2);

        let rot = Mappe::ny();
        let h = Teller { svar: vec![b"feil"], kall: Cell::new(0) };
        assert!(matches!(hent_det_som_mangler(&m, rot.sti(), &h), Err(Feil::NedlastingØdelagt(_))));
        assert_eq!(h.kall.get(), 2);
        assert!(!patch_mappe(rot.sti(), 5).join(crate::patch::AGENT_FIL).exists());
    }

    #[test]
    fn en_fil_noen_har_endret_byttes() {
        let rot = Mappe::ny();
        let m = med_fil(b"agenten");
        let mappe = patch_mappe(rot.sti(), 5);
        fs::create_dir_all(&mappe).unwrap();
        fs::write(mappe.join(crate::patch::AGENT_FIL), b"agentEN").unwrap();
        let h = Teller { svar: vec![b"agenten"], kall: Cell::new(0) };
        hent_det_som_mangler(&m, rot.sti(), &h).unwrap();
        assert_eq!(fs::read(mappe.join(crate::patch::AGENT_FIL)).unwrap(), b"agenten");
    }

    #[test]
    fn beholder_de_to_nyeste_og_den_i_bruk() {
        let rot = Mappe::ny();
        for b in [3, 10, 7, 12] {
            fs::create_dir_all(patch_mappe(rot.sti(), b)).unwrap();
        }
        fs::create_dir_all(rot.sti().join("patch").join("ikke-et-tall")).unwrap();
        let mut slettet = rydd_gamle(rot.sti(), Some(3)).unwrap();
        slettet.sort();
        assert_eq!(slettet, vec![7]);
        assert!(patch_mappe(rot.sti(), 3).is_dir());
        assert!(patch_mappe(rot.sti(), 12).is_dir());
        assert!(rot.sti().join("patch").join("ikke-et-tall").is_dir());
    }
}
