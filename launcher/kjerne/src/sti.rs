//! Stier skrevet slik Java i spillet leser dem.
//!
//! På Windows leser spillets oppstarter `ProjectZomboid64.json` i Windows' gamle tegnsett, ikke i UTF-8. En sti med
//! «ø» (en bruker som heter Bjørn) blir da to feil tegn, Java finner ingenting der og hopper over agenten uten å si fra
//! – og spilleren spiller uten patch. Korte 8.3-navn er bare ASCII, så de leses likt i alle tegnsett. (Lærdom fra
//! Project '97-launcheren.)

use std::path::Path;

/// Stien med `/` og, på Windows, korte 8.3-navn når den har tegn utenfor ASCII.
pub fn jvm_sti(sti: &Path) -> String {
    let lang = sti.to_string_lossy().replace('\\', "/");
    if lang.is_ascii() {
        return lang;
    }
    match kort_navn(sti) {
        Some(kort) if kort.is_ascii() => kort.replace('\\', "/"),
        _ => lang,
    }
}

/// Om stien kan leses av Java på denne PC-en. Launcheren nekter å legge inn en patch på en sti som ikke kan.
pub fn kan_leses_av_java(sti: &Path) -> bool {
    !cfg!(windows) || jvm_sti(sti).is_ascii()
}

#[cfg(windows)]
fn kort_navn(sti: &Path) -> Option<String> {
    use std::os::windows::ffi::{OsStrExt, OsStringExt};
    use windows_sys::Win32::Storage::FileSystem::GetShortPathNameW;

    let bred: Vec<u16> = sti.as_os_str().encode_wide().chain(std::iter::once(0)).collect();
    // Først lengden, så selve navnet.
    let lengde = unsafe { GetShortPathNameW(bred.as_ptr(), std::ptr::null_mut(), 0) };
    if lengde == 0 {
        return None;
    }
    let mut ut = vec![0u16; lengde as usize];
    let skrevet = unsafe { GetShortPathNameW(bred.as_ptr(), ut.as_mut_ptr(), lengde) };
    if skrevet == 0 || skrevet >= lengde {
        return None;
    }
    ut.truncate(skrevet as usize);
    Some(std::ffi::OsString::from_wide(&ut).to_string_lossy().into_owned())
}

#[cfg(not(windows))]
fn kort_navn(_sti: &Path) -> Option<String> {
    None
}

#[cfg(test)]
mod prøver {
    use super::*;

    #[test]
    fn ascii_er_urørt_med_skråstrek_fremover() {
        assert_eq!(jvm_sti(Path::new(r"C:\Users\ola\AppData\Local\KnoxLife\patch\12\knoxlife-agent.jar")),
                   "C:/Users/ola/AppData/Local/KnoxLife/patch/12/knoxlife-agent.jar");
    }

    #[cfg(not(windows))]
    #[test]
    fn utenfor_windows_står_æøå_som_de_er() {
        assert_eq!(jvm_sti(Path::new("/home/bjørn/x.jar")), "/home/bjørn/x.jar");
        assert!(kan_leses_av_java(Path::new("/home/bjørn/x.jar")));
    }
}
