//! Steams tekstformat (KeyValues, «VDF»): `"nøkkel" "verdi"` og `"nøkkel" { … }`. Brukes til
//! `libraryfolders.vdf` og `appmanifest_<id>.acf`. Bare det Steam faktisk skriver, med `//`-kommentarer og
//! `\"`, `\\` og `\n` inni tekst.

use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verdi {
    Tekst(String),
    Blokk(Blokk),
}

/// Nøklene i en blokk, i den rekkefølgen de sto. Steam skriver nøklene uten å bry seg om store og små bokstaver,
/// så de lagres med små bokstaver.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Blokk {
    pub rader: Vec<(String, Verdi)>,
}

impl Blokk {
    pub fn tekst(&self, nøkkel: &str) -> Option<&str> {
        let nøkkel = nøkkel.to_lowercase();
        self.rader.iter().find_map(|(k, v)| match v {
            Verdi::Tekst(t) if *k == nøkkel => Some(t.as_str()),
            _ => None,
        })
    }

    pub fn blokk(&self, nøkkel: &str) -> Option<&Blokk> {
        let nøkkel = nøkkel.to_lowercase();
        self.rader.iter().find_map(|(k, v)| match v {
            Verdi::Blokk(b) if *k == nøkkel => Some(b),
            _ => None,
        })
    }

    pub fn blokker(&self) -> impl Iterator<Item = (&str, &Blokk)> {
        self.rader.iter().filter_map(|(k, v)| match v {
            Verdi::Blokk(b) => Some((k.as_str(), b)),
            _ => None,
        })
    }

    pub fn som_kart(&self) -> BTreeMap<&str, &Verdi> {
        self.rader.iter().map(|(k, v)| (k.as_str(), v)).collect()
    }
}

#[derive(Debug, PartialEq, Eq)]
enum Bit {
    Tekst(String),
    Inn,
    Ut,
}

fn bitene(tekst: &str) -> Option<Vec<Bit>> {
    let mut ut = Vec::new();
    let mut tegn = tekst.chars().peekable();
    while let Some(&c) = tegn.peek() {
        match c {
            c if c.is_whitespace() => {
                tegn.next();
            }
            '/' => {
                // `//` til slutten av linjen
                tegn.next();
                if tegn.peek() == Some(&'/') {
                    for c in tegn.by_ref() {
                        if c == '\n' {
                            break;
                        }
                    }
                } else {
                    return None;
                }
            }
            '{' => {
                tegn.next();
                ut.push(Bit::Inn);
            }
            '}' => {
                tegn.next();
                ut.push(Bit::Ut);
            }
            '"' => {
                tegn.next();
                let mut s = String::new();
                let mut lukket = false;
                while let Some(c) = tegn.next() {
                    match c {
                        '"' => {
                            lukket = true;
                            break;
                        }
                        '\\' => match tegn.next() {
                            Some('n') => s.push('\n'),
                            Some('t') => s.push('\t'),
                            Some(annet) => s.push(annet),
                            None => return None,
                        },
                        annet => s.push(annet),
                    }
                }
                if !lukket {
                    return None;
                }
                ut.push(Bit::Tekst(s));
            }
            _ => {
                // Nøkler uten anførselstegn finnes i noen eldre filer: les til mellomrom eller klamme.
                let mut s = String::new();
                while let Some(&c) = tegn.peek() {
                    if c.is_whitespace() || c == '{' || c == '}' || c == '"' {
                        break;
                    }
                    s.push(c);
                    tegn.next();
                }
                ut.push(Bit::Tekst(s));
            }
        }
    }
    Some(ut)
}

fn blokk(bitene: &[Bit], pos: &mut usize, øverst: bool) -> Option<Blokk> {
    let mut b = Blokk::default();
    loop {
        match bitene.get(*pos) {
            None if øverst => return Some(b),
            None => return None,
            Some(Bit::Ut) if !øverst => {
                *pos += 1;
                return Some(b);
            }
            Some(Bit::Ut) | Some(Bit::Inn) => return None,
            Some(Bit::Tekst(nøkkel)) => {
                *pos += 1;
                let nøkkel = nøkkel.to_lowercase();
                match bitene.get(*pos) {
                    Some(Bit::Tekst(v)) => {
                        *pos += 1;
                        b.rader.push((nøkkel, Verdi::Tekst(v.clone())));
                    }
                    Some(Bit::Inn) => {
                        *pos += 1;
                        let indre = blokk(bitene, pos, false)?;
                        b.rader.push((nøkkel, Verdi::Blokk(indre)));
                    }
                    _ => return None,
                }
            }
        }
    }
}

/// Leser en hel VDF-fil. `None` når filen ikke er gyldig.
pub fn les(tekst: &str) -> Option<Blokk> {
    let tekst = tekst.strip_prefix('\u{feff}').unwrap_or(tekst);
    let bitene = bitene(tekst)?;
    let mut pos = 0;
    blokk(&bitene, &mut pos, true)
}

#[cfg(test)]
mod prøver {
    use super::*;

    const BIBLIOTEKENE: &str = r#"
"libraryfolders"
{
	"0"
	{
		"path"		"C:\\Program Files (x86)\\Steam"
		"label"		""
		"apps"
		{
			"228980"		"297753640"
		}
	}
	"1"
	{
		"path"		"D:\\SteamLibrary"
		// en kommentar
		"apps"
		{
			"108600"		"6123456789"
		}
	}
}
"#;

    #[test]
    fn leser_bibliotekene_med_skråstreker_og_kommentarer() {
        let rot = les(BIBLIOTEKENE).unwrap();
        let mapper = rot.blokk("libraryfolders").unwrap();
        let stier: Vec<_> = mapper.blokker().map(|(_, b)| b.tekst("path").unwrap().to_string()).collect();
        assert_eq!(stier, vec!["C:\\Program Files (x86)\\Steam", "D:\\SteamLibrary"]);
        assert!(mapper.blokk("1").unwrap().blokk("apps").unwrap().tekst("108600").is_some());
    }

    #[test]
    fn nøklene_er_uten_store_bokstaver() {
        let rot = les(r#""AppState" { "installdir" "ProjectZomboid" "StateFlags" "4" }"#).unwrap();
        let app = rot.blokk("appstate").unwrap();
        assert_eq!(app.tekst("InstallDir"), Some("ProjectZomboid"));
        assert_eq!(app.tekst("stateflags"), Some("4"));
    }

    #[test]
    fn ødelagt_fil_gir_none() {
        assert!(les(r#""a" { "b" "c" "#).is_none());
        assert!(les(r#""a" "b" }"#).is_none());
        assert!(les(r#""a" "ikke lukket"#).is_none());
    }

    #[test]
    fn tom_fil_og_bom() {
        assert_eq!(les("").unwrap().rader.len(), 0);
        assert!(les("\u{feff}\"a\" \"b\"").unwrap().tekst("a").is_some());
    }
}
