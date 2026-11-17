//! Kjernen i Knox Life-launcheren – alt launcheren gjør med filene, uten vindu og uten nett, så hvert steg kan prøves
//! for seg. Vinduet (Tauri) og nedlastingen bygger på dette.
//!
//! Rekkefølgen når spilleren trykker «Play» (systemdokumentet `launcher.md` 3.2 i det private repoet, sammendraget i
//! `README.md` her):
//! 1. [`patch::rydd_etter_forrige`] – en rest fra forrige gang settes tilbake før noe annet.
//! 2. [`start::spillet_kjører`] – nekter hvis spillet alt er i gang.
//! 3. [`manifest::Manifest::les_signert`] – signaturen sjekkes før noe i manifestet leses.
//! 4. [`porten::sjekk`] – spillets `projectzomboid.jar` må være nøyaktig den patchen er laget for.
//! 5. [`nedlasting`] – filene i patchen sjekkes mot SHA-256 i manifestet.
//! 6. [`patch::legg_inn`] – kopi, merke, så endringen.
//! 7. [`start`] – billetten, Steam, stempelet og vakten til spillet avsluttes.
//! 8. [`patch::sett_tilbake`].

pub mod feil;
pub mod manifest;
pub mod nedlasting;
pub mod patch;
pub mod porten;
pub mod spillet;
pub mod start;
pub mod sti;
pub mod vdf;

pub use feil::{Feil, Resultat};

#[cfg(test)]
pub(crate) mod prøvehjelp {
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicU64, Ordering};

    /// En tom mappe som slettes når prøven er ferdig.
    pub struct Mappe(PathBuf);

    impl Mappe {
        pub fn ny() -> Mappe {
            static TELLER: AtomicU64 = AtomicU64::new(0);
            let n = TELLER.fetch_add(1, Ordering::Relaxed);
            let sti = std::env::temp_dir().join(format!("knoxlife-prøve-{}-{n}", std::process::id()));
            let _ = std::fs::remove_dir_all(&sti);
            std::fs::create_dir_all(&sti).unwrap();
            Mappe(sti)
        }

        pub fn sti(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for Mappe {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
}
