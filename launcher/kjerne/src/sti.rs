//! Stier skrevet slik Java i spillet leser dem.
//!
//! På Windows leser spillets oppstarter `ProjectZomboid64.json` i Windows' gamle tegnsett, ikke i UTF-8. En sti med
//! «ø» (en bruker som heter Bjørn) blir da to feil tegn, Java finner ingenting der og hopper over agenten uten å si fra
//! – og spilleren spiller uten patch. Korte 8.3-navn er bare ASCII, så de leses likt i alle tegnsett. (Lærdom fra
//! Project '97-launcheren.)

use std::path::Path;

/// Stien med `/` og, på Windows, korte 8.3-navn for mappene når den har tegn utenfor ASCII.
///
/// Filnavnet står alltid som det er – launcheren kjenner igjen sine egne linjer på det (`knoxlife-agent.jar` ville ellers
/// blitt `KNOXLI~1.JAR`). Deler av stien som ikke finnes ennå (stempelet skrives først når spillet starter), står også som
/// de er, etter den korte formen av mappen som finnes.
pub fn jvm_sti(sti: &Path) -> String {
    let lang = sti.to_string_lossy().replace('\\', "/");
    if lang.is_ascii() {
        return lang;
    }
    match kort_sti(sti) {
        Some(kort) if kort.is_ascii() => kort.replace('\\', "/"),
        _ => lang,
    }
}

/// Den korte formen av mappene som finnes, med resten av stien lagt på slik den er.
fn kort_sti(sti: &Path) -> Option<String> {
    let mut rest = Vec::new();
    let mut mappe = sti.to_path_buf();
    rest.push(mappe.file_name()?.to_os_string());
    mappe.pop();
    while !mappe.is_dir() {
        rest.push(mappe.file_name()?.to_os_string());
        if !mappe.pop() {
            return None;
        }
    }
    let mut ut = kort_navn(&mappe)?;
    for del in rest.iter().rev() {
        if !ut.ends_with('\\') && !ut.ends_with('/') {
            ut.push('\\');
        }
        ut.push_str(&del.to_string_lossy());
    }
    Some(ut)
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

    #[cfg(windows)]
    #[test]
    fn bjørn_får_korte_mapper_men_beholder_filnavnet() {
        let mappe = std::env::temp_dir().join(format!("bjørn-{}", std::process::id()));
        std::fs::create_dir_all(&mappe).unwrap();
        let agent = mappe.join("knoxlife-agent.jar");
        std::fs::write(&agent, b"x").unwrap();
        let sti = jvm_sti(&agent);
        assert!(sti.is_ascii(), "{sti}");
        assert!(sti.ends_with("/knoxlife-agent.jar"), "{sti}");
        let stempel = jvm_sti(&mappe.join("Zomboid").join("KnoxLife").join("stempel.txt"));
        assert!(stempel.is_ascii() && stempel.ends_with("/Zomboid/KnoxLife/stempel.txt"), "{stempel}");
        assert!(kan_leses_av_java(&agent));
        std::fs::remove_dir_all(&mappe).unwrap();
    }

    #[cfg(not(windows))]
    #[test]
    fn utenfor_windows_står_æøå_som_de_er() {
        assert_eq!(jvm_sti(Path::new("/home/bjørn/x.jar")), "/home/bjørn/x.jar");
        assert!(kan_leses_av_java(Path::new("/home/bjørn/x.jar")));
    }
}
