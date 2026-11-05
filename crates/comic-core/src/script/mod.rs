//! The script standard, `craft-comic/1`: what a strip says, panel by panel, as the host's model writes it. A panel
//! holds figures in poses from a catalogue, props from a catalogue, what the figures say, and a caption. The agent
//! draws it; nothing in a script says where anything goes beyond a slot (left, center, right).
//!
//! Decoding is loose, because a model writes it: an element of a list that does not read is dropped, a pose or prop
//! the catalogue lacks is kept by its word (and drawn as the nearest it has), and `check` says what the model should
//! fix. `from_json` refuses only what cannot be a strip at all.

mod check;
mod finding;
mod limits;
mod loose;

pub use finding::{Finding, FindingKind, Problem};
pub use limits::Limits;

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Value;
use std::fmt;

use crate::SCHEMA;

/// A strip.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Script {
    /// Always `craft-comic/1`.
    pub schema: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// The aside a reader finds after the strip: never drawn.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alt: Option<String>,
    pub panels: Vec<Panel>,
}

/// One panel: one beat of the strip.
#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct Panel {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub caption: Option<Caption>,
    pub figures: Vec<Figure>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub props: Vec<Prop>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub bubbles: Vec<Bubble>,
}

/// The narrator's box.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Caption {
    pub text: String,
    pub at: CaptionAt,
}

/// Where a caption sits.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum CaptionAt {
    #[default]
    Above,
    Below,
}

/// A stick figure.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Figure {
    /// Short and lowercase; the same across panels for the same person.
    pub id: String,
    pub pose: LoosePose,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub facing: Option<Facing>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub at: Option<Slot>,
    /// Written under the figure's feet.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// What a figure holding a sign has written on it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sign: Option<String>,
}

/// A pose from the catalogue, or the word a script used for one it lacks.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LoosePose {
    Known(Pose),
    Unknown(String),
}

impl LoosePose {
    /// The pose drawn: the catalogue's, or standing.
    pub fn pose(&self) -> Pose {
        match self {
            LoosePose::Known(pose) => *pose,
            LoosePose::Unknown(_) => Pose::Standing,
        }
    }

    pub fn word(&self) -> &str {
        match self {
            LoosePose::Known(pose) => pose.word(),
            LoosePose::Unknown(word) => word,
        }
    }
}

impl Serialize for LoosePose {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.word())
    }
}

/// The poses a figure can take.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Pose {
    Standing,
    Pointing,
    ArmsUp,
    Shrugging,
    Facepalm,
    Walking,
    Sitting,
    LyingDown,
    HoldingSign,
    Waving,
}

impl Pose {
    pub const ALL: [Pose; 10] = [
        Pose::Standing,
        Pose::Pointing,
        Pose::ArmsUp,
        Pose::Shrugging,
        Pose::Facepalm,
        Pose::Walking,
        Pose::Sitting,
        Pose::LyingDown,
        Pose::HoldingSign,
        Pose::Waving,
    ];

    pub fn word(self) -> &'static str {
        match self {
            Pose::Standing => "standing",
            Pose::Pointing => "pointing",
            Pose::ArmsUp => "arms-up",
            Pose::Shrugging => "shrugging",
            Pose::Facepalm => "facepalm",
            Pose::Walking => "walking",
            Pose::Sitting => "sitting",
            Pose::LyingDown => "lying-down",
            Pose::HoldingSign => "holding-sign",
            Pose::Waving => "waving",
        }
    }

    /// The pose a word names, in any case, with `_` or a space for `-`.
    pub fn from_word(word: &str) -> Option<Pose> {
        let word = normal_word(word);
        Pose::ALL.into_iter().find(|pose| pose.word() == word)
    }
}

/// Which way a figure looks.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Facing {
    Left,
    Right,
}

impl Facing {
    pub fn from_word(word: &str) -> Option<Facing> {
        match normal_word(word).as_str() {
            "left" => Some(Facing::Left),
            "right" => Some(Facing::Right),
            _ => None,
        }
    }

    /// −1 for left, 1 for right.
    pub fn sign(self) -> f32 {
        match self {
            Facing::Left => -1.0,
            Facing::Right => 1.0,
        }
    }
}

/// Where in a panel something stands.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Slot {
    Left,
    Center,
    Right,
}

impl Slot {
    pub const ALL: [Slot; 3] = [Slot::Left, Slot::Center, Slot::Right];

    pub fn from_word(word: &str) -> Option<Slot> {
        match normal_word(word).as_str() {
            "left" => Some(Slot::Left),
            "center" | "centre" | "middle" => Some(Slot::Center),
            "right" => Some(Slot::Right),
            _ => None,
        }
    }
}

/// What a figure says, thinks or shouts; `by` is a figure's id in the panel, or `off` for a voice from outside it.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Bubble {
    pub by: String,
    pub text: String,
    #[serde(skip_serializing_if = "BubbleKind::is_speech")]
    pub kind: BubbleKind,
}

/// How a line is said.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum BubbleKind {
    #[default]
    Speech,
    Thought,
    Shout,
}

impl BubbleKind {
    fn is_speech(&self) -> bool {
        *self == BubbleKind::Speech
    }

    pub fn from_word(word: &str) -> Option<BubbleKind> {
        match normal_word(word).as_str() {
            "speech" | "say" | "says" | "speak" => Some(BubbleKind::Speech),
            "thought" | "think" | "thinks" => Some(BubbleKind::Thought),
            "shout" | "shouts" | "yell" => Some(BubbleKind::Shout),
            _ => None,
        }
    }
}

/// A thing in the panel.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Prop {
    pub kind: LooseProp,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub at: Option<Slot>,
    /// What a whiteboard, signpost, cloud or arrow says; for a graph, the shape of its line.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
}

/// A prop from the catalogue, or the word a script used for one it lacks.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LooseProp {
    Known(PropKind),
    Unknown(String),
}

impl LooseProp {
    pub fn word(&self) -> &str {
        match self {
            LooseProp::Known(kind) => kind.word(),
            LooseProp::Unknown(word) => word,
        }
    }

    pub fn known(&self) -> Option<PropKind> {
        match self {
            LooseProp::Known(kind) => Some(*kind),
            LooseProp::Unknown(_) => None,
        }
    }
}

impl Serialize for LooseProp {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.word())
    }
}

/// The props a panel can hold.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PropKind {
    Desk,
    Laptop,
    Whiteboard,
    Box,
    Arrow,
    Cloud,
    Graph,
    Door,
    Tree,
    Signpost,
}

impl PropKind {
    pub const ALL: [PropKind; 10] = [
        PropKind::Desk,
        PropKind::Laptop,
        PropKind::Whiteboard,
        PropKind::Box,
        PropKind::Arrow,
        PropKind::Cloud,
        PropKind::Graph,
        PropKind::Door,
        PropKind::Tree,
        PropKind::Signpost,
    ];

    pub fn word(self) -> &'static str {
        match self {
            PropKind::Desk => "desk",
            PropKind::Laptop => "laptop",
            PropKind::Whiteboard => "whiteboard",
            PropKind::Box => "box",
            PropKind::Arrow => "arrow",
            PropKind::Cloud => "cloud",
            PropKind::Graph => "graph",
            PropKind::Door => "door",
            PropKind::Tree => "tree",
            PropKind::Signpost => "signpost",
        }
    }

    pub fn from_word(word: &str) -> Option<PropKind> {
        let word = normal_word(word);
        PropKind::ALL.into_iter().find(|kind| kind.word() == word)
    }

    /// Whether it says something written on it.
    pub fn holds_text(self) -> bool {
        matches!(
            self,
            PropKind::Whiteboard
                | PropKind::Signpost
                | PropKind::Cloud
                | PropKind::Arrow
                | PropKind::Graph
        )
    }
}

/// Lowercase, trimmed, `_` and spaces read as `-`.
fn normal_word(word: &str) -> String {
    word.trim().to_lowercase().replace(['_', ' '], "-")
}

/// Why bytes are not a strip.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ScriptError {
    NotJson(String),
    NotAnObject,
    Schema(String),
    NoPanels,
}

impl fmt::Display for ScriptError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ScriptError::NotJson(reason) => write!(f, "the script is not JSON ({reason})"),
            ScriptError::NotAnObject => write!(f, "the script is not a JSON object"),
            ScriptError::Schema(said) => {
                write!(f, "the script says it is `{said}`, and a strip is {SCHEMA}")
            }
            ScriptError::NoPanels => write!(
                f,
                "the script has no panels: a strip holds 1 to {}",
                Limits::PANELS
            ),
        }
    }
}

impl std::error::Error for ScriptError {}

impl Script {
    /// A script from JSON bytes, read loosely; refused only when it cannot be a strip.
    pub fn from_json(bytes: &[u8]) -> Result<Script, ScriptError> {
        let value: Value =
            serde_json::from_slice(bytes).map_err(|e| ScriptError::NotJson(e.to_string()))?;
        Script::from_value(&value)
    }

    /// A script from a JSON value, read loosely.
    pub fn from_value(value: &Value) -> Result<Script, ScriptError> {
        loose::script(value)
    }

    /// The script as JSON, keys sorted, its `schema` stamped.
    pub fn to_value(&self) -> Value {
        let mut value = serde_json::to_value(self).unwrap_or(Value::Null);
        if let Value::Object(map) = &mut value {
            map.insert("schema".into(), Value::String(SCHEMA.into()));
        }
        sorted(value)
    }

    /// The script as one line of JSON, keys sorted.
    pub fn to_json(&self) -> String {
        self.to_value().to_string()
    }

    /// The script as indented JSON, keys sorted.
    pub fn to_json_pretty(&self) -> String {
        serde_json::to_string_pretty(&self.to_value()).unwrap_or_default()
    }

    /// The title as a file name: lowercase words joined by `-`, at most 48 characters; `comic` when there is none.
    pub fn slug(&self) -> String {
        let mut slug = String::new();
        for word in self
            .title
            .as_deref()
            .unwrap_or("")
            .split(|c: char| !c.is_ascii_alphanumeric())
        {
            if word.is_empty() {
                continue;
            }
            let word = word.to_ascii_lowercase();
            let next = if slug.is_empty() {
                word.len()
            } else {
                slug.len() + 1 + word.len()
            };
            if next > 48 {
                break;
            }
            if !slug.is_empty() {
                slug.push('-');
            }
            slug.push_str(&word);
        }
        if slug.is_empty() {
            "comic".into()
        } else {
            slug
        }
    }

    /// Every figure id the strip uses, in order of first appearance.
    pub fn cast(&self) -> Vec<String> {
        let mut ids: Vec<String> = Vec::new();
        for figure in self.panels.iter().flat_map(|p| &p.figures) {
            if !ids.contains(&figure.id) {
                ids.push(figure.id.clone());
            }
        }
        ids
    }
}

impl<'de> Deserialize<'de> for Script {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Script, D::Error> {
        let value = Value::deserialize(deserializer)?;
        Script::from_value(&value).map_err(serde::de::Error::custom)
    }
}

/// A JSON value with every object's keys in order, whatever serde_json's features.
fn sorted(value: Value) -> Value {
    match value {
        Value::Object(map) => {
            let mut entries: Vec<(String, Value)> = map.into_iter().collect();
            entries.sort_by(|a, b| a.0.cmp(&b.0));
            Value::Object(entries.into_iter().map(|(k, v)| (k, sorted(v))).collect())
        }
        Value::Array(items) => Value::Array(items.into_iter().map(sorted).collect()),
        other => other,
    }
}
