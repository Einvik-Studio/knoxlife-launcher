//! Versjonsporten: patchen passer bare den nøyaktige `projectzomboid.jar` den er laget for.
//!
//! Java-modene griper inn i spillets egne metoder. Har spillet endret seg, kan en endring enten gjøre ingenting eller krasje
//! – og det første merkes ikke før etter flere uker. Derfor sammenlignes hele filen (SHA-256), ikke versjonsnummeret, og
//! det finnes ingen knapp for å gå forbi.

use crate::feil::{Feil, Resultat};
use crate::manifest::Manifest;
use crate::spillet::Spill;
use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::Read;
use std::path::Path;

/// SHA-256 av en fil, lest i biter (spillets jar er flere titalls MB).
pub fn sha256_av_fil(sti: &Path) -> Resultat<String> {
    let mut f = File::open(sti)?;
    let mut h = Sha256::new();
    let mut buf = vec![0u8; 1 << 20];
    loop {
        let n = f.read(&mut buf)?;
        if n == 0 {
            break;
        }
        h.update(&buf[..n]);
    }
    Ok(hex::encode(h.finalize()))
}

/// Hva porten sa.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Svar {
    /// Spillet er det patchen er laget for.
    Passer,
    /// Nødmodus er på: spillet startes uten patch (3.9 punkt 9).
    Nødmodus,
}

pub fn sjekk(spill: &Spill, manifest: &Manifest) -> Resultat<Svar> {
    if manifest.nødmodus {
        return Ok(Svar::Nødmodus);
    }
    let jar = spill.jar();
    let har = sha256_av_fil(&jar).map_err(|_| Feil::SpillfilØdelagt("projectzomboid.jar".into()))?;
    if har != manifest.spillet.jar_sha256 {
        return Err(Feil::SpilletOppdatert { har, trenger: manifest.spillet.versjon.clone() });
    }
    Ok(Svar::Passer)
}

#[cfg(test)]
mod prøver {
    use super::*;
    use crate::manifest::prøver::eksempel;
    use crate::patch::sha256;
    use crate::prøvehjelp::Mappe;
    use crate::spillet::Plattform;
    use std::fs;

    #[test]
    fn bare_nøyaktig_samme_jar_slipper_gjennom() {
        let m = Mappe::ny();
        let spill = Spill::ny(m.sti(), Plattform::Windows);
        fs::write(spill.jar(), b"spillet 42.22.2").unwrap();
        let mut manifest = eksempel(3);
        manifest.spillet.jar_sha256 = sha256(b"spillet 42.22.2");
        assert_eq!(sjekk(&spill, &manifest).unwrap(), Svar::Passer);

        fs::write(spill.jar(), b"spillet 42.23.0").unwrap();
        assert!(matches!(sjekk(&spill, &manifest), Err(Feil::SpilletOppdatert { .. })));

        manifest.nødmodus = true;
        assert_eq!(sjekk(&spill, &manifest).unwrap(), Svar::Nødmodus);
    }

    #[test]
    fn stor_fil_leses_i_biter_med_samme_svar() {
        let m = Mappe::ny();
        let sti = m.sti().join("stor.jar");
        let bytes: Vec<u8> = (0..3_500_000u32).map(|i| (i % 251) as u8).collect();
        fs::write(&sti, &bytes).unwrap();
        assert_eq!(sha256_av_fil(&sti).unwrap(), sha256(&bytes));
    }
}
