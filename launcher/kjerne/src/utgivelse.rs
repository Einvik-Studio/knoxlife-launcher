//! Utgivelsen (`launcher.md` 3.10): manifestet skrives og signeres av jobben «Utgivelse» i GitHub, aldri på en PC.
//! Signeringsnøkkelen står bare i GitHubs miljø «utgivelse», som bare kan kjøres fra `main` og bare når eierne har
//! godkjent kjøringen. Den gis bare til filer GitHub selv har bygget i den samme kjøringen.
//!
//! Her er det som lager manifestet av filene og signerer det – `knoxctl lag-manifest`, `knoxctl nodmodus` og
//! `knoxctl signer` er dørene til det.

use crate::feil::{Feil, Resultat};
use crate::manifest::{Fil, Manifest, Nyhet, Server, Spillversjon, FLEST_NYHETER, FORMAT};
use crate::porten::sha256_av_fil;
use ed25519_dalek::{Signer, SigningKey};
use std::path::Path;

/// Det som trengs for en ny patch.
pub struct Utgave<'a> {
    /// Byggenummeret: kjøringens nummer i GitHub, høyere enn i forrige utgave.
    pub bygg: u64,
    /// Når (ISO 8601, UTC, «2026-11-18T20:00:00Z»).
    pub utgitt: &'a str,
    /// Spillets versjon slik spillet viser den («42.22.2»), lest av `knoxlife.porten.Versjon`.
    pub versjon: &'a str,
    /// SHA-256 av spillets `projectzomboid.jar` som patchen er bygget og prøvd mot. Jobben som signerer, har aldri
    /// selve filen – spillets filer lastes aldri opp, heller ikke mellom jobbene.
    pub spillets_jar_sha256: &'a str,
    /// Filene i patchen (agenten først).
    pub filer: &'a [&'a Path],
    /// Serveren («vert:port»).
    pub server: &'a str,
    pub testserver: Option<&'a str>,
    /// Adressen til serverens status og køen (`https://…/`).
    pub status: Option<&'a str>,
    /// Den forrige utgaven (signaturen sjekket): nyhetene og ytelsesmodus følger med videre.
    pub forrige: Option<&'a Manifest>,
    /// En nyhet fra eierne: tittel og tekst.
    pub nyhet: Option<(&'a str, &'a str)>,
}

fn feil<T>(tekst: &str) -> Resultat<T> {
    Err(Feil::Manifest(tekst.into()))
}

/// «vert:port» til en server – bare det agenten godtar ([`Server::trygg`]).
pub fn les_server(tekst: &str) -> Resultat<Server> {
    let Some((adresse, port)) = tekst.trim().rsplit_once(':') else {
        return feil("server must be host:port");
    };
    let Ok(port) = port.parse::<u16>() else {
        return feil("server port");
    };
    let s = Server { adresse: adresse.to_string(), port };
    if !s.trygg() {
        return feil("server address");
    }
    Ok(s)
}

fn sjekk_bygg(bygg: u64, forrige: Option<&Manifest>) -> Resultat<()> {
    if let Some(f) = forrige {
        if bygg <= f.bygg {
            return feil("the build number must be higher than in the last release");
        }
    }
    Ok(())
}

fn sjekk_utgitt(utgitt: &str) -> Resultat<()> {
    // 2026-11-18T20:00:00Z
    let b = utgitt.as_bytes();
    let ok = b.len() == 20
        && b.iter().enumerate().all(|(i, &c)| match i {
            4 | 7 => c == b'-',
            10 => c == b'T',
            13 | 16 => c == b':',
            19 => c == b'Z',
            _ => c.is_ascii_digit(),
        });
    if !ok {
        return feil("release time must be like 2026-11-18T20:00:00Z");
    }
    Ok(())
}

/// Manifestet for en ny patch – sjekket slik launcheren sjekker det.
pub fn lag(u: &Utgave) -> Resultat<Manifest> {
    sjekk_bygg(u.bygg, u.forrige)?;
    sjekk_utgitt(u.utgitt)?;
    let versjon = u.versjon.trim();
    if versjon.is_empty() || versjon.len() > 40 || versjon.chars().any(char::is_control) {
        return feil("game version");
    }
    let mut filer = Vec::new();
    for sti in u.filer {
        let navn = sti.file_name().and_then(|n| n.to_str()).unwrap_or_default().to_string();
        let størrelse = std::fs::metadata(sti)?.len();
        filer.push(Fil { navn, sha256: sha256_av_fil(sti)?, størrelse });
    }
    let dato = &u.utgitt[..10];
    let mut nyheter = Vec::new();
    match u.nyhet {
        Some((tittel, tekst)) => nyheter.push(Nyhet { dato: dato.into(), tittel: tittel.trim().into(), tekst: tekst.trim().into() }),
        // En ny versjon av spillet får en nyhet av seg selv, så spillerne ser at Knox Life er klar for den.
        None if u.forrige.is_some_and(|f| f.spillet.versjon != versjon) => nyheter.push(Nyhet {
            dato: dato.into(),
            tittel: format!("Ready for Project Zomboid {versjon}"),
            tekst: "Knox Life is ready for the new game version. Press Play as usual.".into(),
        }),
        None => {}
    }
    if let Some(f) = u.forrige {
        nyheter.extend(f.nyheter.iter().cloned());
    }
    nyheter.truncate(FLEST_NYHETER);
    let m = Manifest {
        format: FORMAT,
        bygg: u.bygg,
        utgitt: u.utgitt.into(),
        spillet: Spillversjon { versjon: versjon.into(), jar_sha256: u.spillets_jar_sha256.trim().into() },
        filer,
        server: les_server(u.server)?,
        testserver: u.testserver.map(les_server).transpose()?,
        nødmodus: false,
        nyheter,
        ytelsesmodus: u.forrige.and_then(|f| f.ytelsesmodus.clone()),
        status: u.status.map(str::to_string),
    };
    m.sjekk()?;
    Ok(m)
}

/// Nødmodus på eller av (3.9 punkt 9): den forrige utgaven med et nytt byggenummer og den samme agenten. Det spillet
/// trenger, finnes alt hos spillerne – nødmodus starter spillet uten patch, og «av» gir tilbake patchen som var.
pub fn nødmodus(forrige: &Manifest, på: bool, bygg: u64, utgitt: &str) -> Resultat<Manifest> {
    sjekk_bygg(bygg, Some(forrige))?;
    sjekk_utgitt(utgitt)?;
    if forrige.nødmodus == på {
        return feil(if på { "emergency mode is already on" } else { "emergency mode is already off" });
    }
    let mut m = forrige.clone();
    m.bygg = bygg;
    m.utgitt = utgitt.into();
    m.nødmodus = på;
    m.sjekk()?;
    Ok(m)
}

/// Manifestet som bytene som signeres og publiseres.
pub fn json(m: &Manifest) -> Resultat<Vec<u8>> {
    Ok(serde_json::to_vec_pretty(m)?)
}

/// Signaturen (det som står i `manifest.sig`) og den offentlige nøkkelen, begge i hex. Nøkkelen er de 32 bytene i hex.
/// Signaturen sjekkes med en gang på samme måte som launcheren gjør, så en feil stopper utgivelsen her.
pub fn signer(json: &[u8], nøkkel_hex: &str) -> Resultat<(String, String)> {
    let bytes: [u8; 32] = hex::decode(nøkkel_hex.trim())
        .ok()
        .and_then(|b| b.try_into().ok())
        .ok_or_else(|| Feil::Manifest("the signing key must be 64 hex characters".into()))?;
    let nøkkel = SigningKey::from_bytes(&bytes);
    let sig = hex::encode(nøkkel.sign(json).to_bytes());
    let offentlig = nøkkel.verifying_key();
    Manifest::les_signert(json, sig.as_bytes(), &offentlig)?;
    Ok((sig, hex::encode(offentlig.to_bytes())))
}

#[cfg(test)]
mod prøver {
    use super::*;
    use crate::manifest::prøver::eksempel;
    use crate::manifest::{nøkkel_fra_hex, Manifest};
    use crate::patch::AGENT_FIL;
    use crate::prøvehjelp::Mappe;
    use std::fs;

    struct Filene {
        _mappe: Mappe,
        agent: std::path::PathBuf,
    }

    fn filene() -> Filene {
        let mappe = Mappe::ny();
        let agent = mappe.sti().join(AGENT_FIL);
        fs::write(&agent, b"agenten").unwrap();
        Filene { _mappe: mappe, agent }
    }

    const SPILLET: &str = "3c4a7c5f149d784758c6fb5f8fa85f3a21d572e38bc081400e89035f8120d287";

    fn utgave<'a>(filer: &'a [&'a Path], forrige: Option<&'a Manifest>) -> Utgave<'a> {
        Utgave {
            bygg: 12,
            utgitt: "2026-11-18T20:00:00Z",
            versjon: "42.22.2",
            spillets_jar_sha256: SPILLET,
            filer,
            server: "play.example.invalid:16261",
            testserver: Some("10.0.0.6:16271"),
            status: Some("https://status.example.invalid/"),
            forrige,
            nyhet: None,
        }
    }

    #[test]
    fn manifestet_lages_av_filene_og_kan_signeres_og_leses() {
        let f = filene();
        let filer = [f.agent.as_path()];
        let m = lag(&utgave(&filer, None)).unwrap();
        assert_eq!(m.bygg, 12);
        assert_eq!(m.spillet.versjon, "42.22.2");
        assert_eq!(m.spillet.jar_sha256, SPILLET);
        assert_eq!(m.filer, vec![Fil { navn: AGENT_FIL.into(), sha256: sha256_av_fil(&f.agent).unwrap(), størrelse: 7 }]);
        assert_eq!(m.server, Server { adresse: "play.example.invalid".into(), port: 16261 });
        assert_eq!(m.testserver, Some(Server { adresse: "10.0.0.6".into(), port: 16271 }));
        assert!(!m.nødmodus && m.nyheter.is_empty());

        let nøkkel = "11".repeat(32);
        let json = json(&m).unwrap();
        let (sig, offentlig) = signer(&json, &format!(" {nøkkel}\n")).unwrap();
        let lest = Manifest::les_signert(&json, sig.as_bytes(), &nøkkel_fra_hex(&offentlig).unwrap()).unwrap();
        assert_eq!(lest, m);
        // Den samme nøkkelen gir den samme offentlige nøkkelen hver gang – den eierne legger inn i launcheren.
        assert_eq!(signer(&json, &nøkkel).unwrap(), (sig, offentlig));
    }

    #[test]
    fn feil_nøkkel_eller_feil_innhold_stopper_utgivelsen() {
        for nøkkel in ["", "abc", "zz".repeat(32).as_str(), "11".repeat(33).as_str()] {
            assert!(signer(b"{}", nøkkel).is_err(), "{nøkkel}");
        }
        // Et manifest launcheren ikke ville godtatt, signeres ikke.
        assert!(matches!(signer(b"{\"format\":1}", &"11".repeat(32)), Err(Feil::Manifest(_))));

        let f = filene();
        let filer = [f.agent.as_path()];
        for (felt, u) in [
            ("server", Utgave { server: "play.example.invalid", ..utgave(&filer, None) }),
            ("port", Utgave { server: "play.example.invalid:0", ..utgave(&filer, None) }),
            ("mellomrom", Utgave { server: "play example:16261", ..utgave(&filer, None) }),
            ("testserver", Utgave { testserver: Some("play.example.invalid:16261"), ..utgave(&filer, None) }),
            ("status", Utgave { status: Some("http://status.example.invalid/"), ..utgave(&filer, None) }),
            ("versjon", Utgave { versjon: " ", ..utgave(&filer, None) }),
            ("tid", Utgave { utgitt: "18.11.2026", ..utgave(&filer, None) }),
            ("uten agent", Utgave { filer: &[], ..utgave(&filer, None) }),
            ("spillet", Utgave { spillets_jar_sha256: "ABC", ..utgave(&filer, None) }),
        ] {
            assert!(lag(&u).is_err(), "{felt}");
        }
    }

    #[test]
    fn byggenummeret_går_opp_og_nyhetene_følger_med() {
        let f = filene();
        let filer = [f.agent.as_path()];
        let mut forrige = eksempel(12);
        forrige.nyheter = (0..FLEST_NYHETER).map(|i| Nyhet { dato: "2026-11-17".into(), tittel: format!("N{i}"), tekst: String::new() }).collect();
        forrige.ytelsesmodus = Some([("water".to_string(), "1".to_string())].into_iter().collect());
        assert!(lag(&utgave(&filer, Some(&forrige))).is_err(), "samme bygg");

        // Samme versjon av spillet: ingen ny nyhet, men de forrige og ytelsesmodus følger med.
        let m = lag(&Utgave { bygg: 13, ..utgave(&filer, Some(&forrige)) }).unwrap();
        assert_eq!(m.nyheter, forrige.nyheter);
        assert_eq!(m.ytelsesmodus, forrige.ytelsesmodus);

        // En ny versjon av spillet gir en nyhet øverst, og den eldste faller ut.
        let m = lag(&Utgave { bygg: 13, versjon: "42.23.0", ..utgave(&filer, Some(&forrige)) }).unwrap();
        assert_eq!(m.nyheter.len(), FLEST_NYHETER);
        assert_eq!(m.nyheter[0].tittel, "Ready for Project Zomboid 42.23.0");
        assert_eq!(m.nyheter[0].dato, "2026-11-18");
        assert_eq!(m.nyheter[1].tittel, "N0");

        // Eiernes egen nyhet går foran.
        let m = lag(&Utgave { bygg: 13, versjon: "42.23.0", nyhet: Some(("Patch 13", " Faster cars. ")), ..utgave(&filer, Some(&forrige)) })
            .unwrap();
        assert_eq!((m.nyheter[0].tittel.as_str(), m.nyheter[0].tekst.as_str()), ("Patch 13", "Faster cars."));
        // Og en nyhet launcheren ikke ville vist, stopper utgivelsen.
        let lang = "x".repeat(301);
        assert!(lag(&Utgave { bygg: 13, nyhet: Some(("Patch 13", lang.as_str())), ..utgave(&filer, Some(&forrige)) }).is_err());
    }

    #[test]
    fn nødmodus_er_forrige_utgave_med_nytt_bygg() {
        let forrige = eksempel(12);
        let på = nødmodus(&forrige, true, 13, "2026-11-18T21:00:00Z").unwrap();
        assert!(på.nødmodus);
        assert_eq!((på.bygg, på.utgitt.as_str()), (13, "2026-11-18T21:00:00Z"));
        assert_eq!((&på.filer, &på.spillet, &på.server), (&forrige.filer, &forrige.spillet, &forrige.server));
        assert!(nødmodus(&på, true, 14, "2026-11-18T21:00:00Z").is_err(), "alt på");
        assert!(nødmodus(&forrige, true, 12, "2026-11-18T21:00:00Z").is_err(), "samme bygg");
        let av = nødmodus(&på, false, 14, "2026-11-18T22:00:00Z").unwrap();
        assert!(!av.nødmodus);
        assert_eq!(av.filer, forrige.filer);
    }
}
