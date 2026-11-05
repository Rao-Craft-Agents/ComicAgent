//! What a strip measured wrong (a finding: the model's to fix) and what the agent did about what it could not draw as
//! asked (a problem: the answer's `problems`).

use serde::{Deserialize, Serialize};

/// What a script or its drawing breaks, said so a model can fix it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Finding {
    pub kind: FindingKind,
    /// What it is about: `title`, `panel:2`, `panel:2/caption`, `panel:2/bubble:1`, `panel:2/figure:ana`, `panel:2/prop:1`.
    pub element: String,
    /// The numbers the sentence names: what it measured, then the limit, then the length to aim for.
    pub numbers: Vec<i32>,
    /// One sentence with its fix in it.
    pub sentence: String,
}

/// The kinds of finding.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum FindingKind {
    /// Longer or more than the standard holds.
    Limit,
    /// A word the catalogue lacks, or an id no figure has.
    Unknown,
    /// Text that runs past the room drawn for it.
    Overflow,
    /// Two things drawn over each other.
    Overlap,
    /// Something past the panel's edge.
    Outside,
}

impl Finding {
    pub fn new(
        kind: FindingKind,
        element: impl Into<String>,
        numbers: Vec<i32>,
        sentence: impl Into<String>,
    ) -> Finding {
        Finding {
            kind,
            element: element.into(),
            numbers,
            sentence: sentence.into(),
        }
    }
}

/// What the agent could only do in part, as `granite-answer/1`'s `problems` carries it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Problem {
    pub kind: String,
    pub text: String,
}

impl Problem {
    /// Text that was cut to fit.
    pub fn short(text: impl Into<String>) -> Problem {
        Problem {
            kind: "short".into(),
            text: text.into(),
        }
    }

    /// Something drawn another way than asked.
    pub fn fallback(text: impl Into<String>) -> Problem {
        Problem {
            kind: "fallback".into(),
            text: text.into(),
        }
    }

    /// Something not drawn.
    pub fn skipped(text: impl Into<String>) -> Problem {
        Problem {
            kind: "skipped".into(),
            text: text.into(),
        }
    }

    /// The host's model said nothing usable.
    pub fn model(text: impl Into<String>) -> Problem {
        Problem {
            kind: "model".into(),
            text: text.into(),
        }
    }
}
