//! Feilene kjernen kan gi. Teksten i hver feil er det spilleren ser i launcheren, så den er på engelsk og sier hva
//! spilleren kan gjøre.

use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum Feil {
    #[error("Project Zomboid was not found. Choose the folder it is installed in.")]
    SpilletIkkeFunnet,

    #[error("Steam was not found. Install Steam and Project Zomboid, then try again.")]
    SteamIkkeFunnet,

    #[error("{0} could not be read. Verify the game files in Steam (Properties > Installed Files).")]
    SpillfilØdelagt(String),

    #[error("{0} starts with a byte order mark, which stops the game from starting. Verify the game files in Steam.")]
    Bom(String),

    #[error("The game folder is gone ({0}). Reinstall the game or choose its new folder.")]
    SpillmappenBorte(PathBuf),

    #[error("The Knox Life update could not be verified. Try again later.")]
    Signatur,

    #[error("The Knox Life update is not valid ({0}). Try again later.")]
    Manifest(String),

    #[error("This Knox Life update (build {ny}) is older than one you already had (build {sist}). Try again later.")]
    EldreBygg { ny: u64, sist: u64 },

    #[error("Project Zomboid has updated. Knox Life is getting ready for it – usually within a few hours. See #announcements on Discord.")]
    SpilletOppdatert { har: String, trenger: String },

    #[error("The download was damaged ({0}). Press Play to try again.")]
    NedlastingØdelagt(String),

    #[error("The game files could not be put back ({0}). Use \"Restore game files\" in Details, or verify the game files in Steam.")]
    KunneIkkeSetteTilbake(String),

    #[error("Knox Life cannot use the folder {0} because its name has letters Java cannot read on this PC. Use \"Copy report\" and send it to staff on Discord.")]
    StiKanIkkeLeses(PathBuf),

    #[error("{0}")]
    Io(#[from] std::io::Error),

    #[error("{0}")]
    Json(#[from] serde_json::Error),
}

pub type Resultat<T> = std::result::Result<T, Feil>;
