use std::fmt;

use granite_agents::{AgentRefusal, AgentRefusing, RefusalKind};

/// Why a call was not answered, each said with its fix.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ComicError {
    /// No script, and no model to draft one.
    NoModel,
    /// No script, and nothing said of what to explain.
    NoBrief,
    /// The script given could not be read as a strip.
    Script { path: String, reason: String },
    /// The model's drafts were not strips.
    Draft(String),
    /// The call asked for the GPU and there is none, or it failed.
    Device(String),
    /// The PNG could not be written.
    Unwritable { path: String, reason: String },
    /// `panels` outside 1 to 6.
    Panels(u32),
    /// `rounds` under 1.
    Rounds(u32),
}

impl fmt::Display for ComicError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ComicError::NoModel => f.write_str(
                "comic drafts with the host's model and this host has none: give `script` (a .comic.json or a strip's PNG), or call from a host with a model loaded",
            ),
            ComicError::NoBrief => f.write_str("comic was told nothing to explain: say the concept in `brief`"),
            ComicError::Script { path, reason } => write!(
                f,
                "the script {path} could not be read: {reason}; give a .comic.json that follows craft-comic/1 or a PNG comic drew"
            ),
            ComicError::Draft(reason) => write!(f, "the model's script was not a strip ({reason}): call again with a narrower brief, or say what stops it"),
            ComicError::Device(reason) => write!(f, "{reason}: leave `renderer` out to draw on the CPU"),
            ComicError::Unwritable { path, reason } => write!(f, "could not write {path} ({reason}): give `output` under a folder the host can write"),
            ComicError::Panels(panels) => write!(f, "`panels` is {panels}: a strip holds 1 to 6"),
            ComicError::Rounds(rounds) => write!(f, "`rounds` is {rounds}: give 1 or more, or leave it out"),
        }
    }
}

impl std::error::Error for ComicError {}

impl AgentRefusing for ComicError {
    fn refusal(&self) -> AgentRefusal {
        let kind = match self {
            ComicError::NoModel
            | ComicError::NoBrief
            | ComicError::Script { .. }
            | ComicError::Unwritable { .. } => RefusalKind::Call,
            ComicError::Draft(_) => RefusalKind::Model,
            ComicError::Device(_) => RefusalKind::Device,
            ComicError::Panels(_) | ComicError::Rounds(_) => RefusalKind::Input,
        };
        AgentRefusal::one(kind, self.to_string())
    }
}
