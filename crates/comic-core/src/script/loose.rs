//! Reading a script the way a model writes one: forgiving where it can be, strict only where it must be.

use serde_json::{Map, Value};

use super::{
    Bubble, BubbleKind, Caption, CaptionAt, Facing, Figure, LoosePose, LooseProp, Panel, Pose,
    Prop, PropKind, Script, ScriptError, Slot,
};
use crate::SCHEMA;

pub(super) fn script(value: &Value) -> Result<Script, ScriptError> {
    let object = value.as_object().ok_or(ScriptError::NotAnObject)?;
    if let Some(said) = object.get("schema") {
        let said = said.as_str().map(str::trim).unwrap_or("");
        if said != SCHEMA {
            return Err(ScriptError::Schema(if said.is_empty() {
                said_of(object.get("schema"))
            } else {
                said.into()
            }));
        }
    }
    let panels: Vec<Panel> = match object.get("panels") {
        Some(Value::Array(items)) => items
            .iter()
            .filter_map(Value::as_object)
            .map(panel)
            .collect(),
        _ => Vec::new(),
    };
    if panels.is_empty() {
        return Err(ScriptError::NoPanels);
    }
    Ok(Script {
        schema: SCHEMA.into(),
        title: text(object.get("title")),
        alt: text(object.get("alt")),
        panels,
    })
}

fn said_of(value: Option<&Value>) -> String {
    value.map(Value::to_string).unwrap_or_default()
}

fn panel(object: &Map<String, Value>) -> Panel {
    let caption = match object.get("caption") {
        Some(Value::Object(c)) => text(c.get("text")).map(|text| Caption {
            text,
            at: match word(c.get("at")).as_deref() {
                Some("below") | Some("bottom") => CaptionAt::Below,
                _ => CaptionAt::Above,
            },
        }),
        other => text(other).map(|text| Caption {
            text,
            at: CaptionAt::Above,
        }),
    };
    let figures = list(object.get("figures"))
        .enumerate()
        .map(|(i, f)| figure(f, i))
        .collect();
    let props = list(object.get("props")).filter_map(prop).collect();
    let bubbles = list(object.get("bubbles")).filter_map(bubble).collect();
    Panel {
        caption,
        figures,
        props,
        bubbles,
    }
}

fn figure(object: &Map<String, Value>, index: usize) -> Figure {
    let id = word(object.get("id")).unwrap_or_else(|| format!("figure-{}", index + 1));
    let pose = match text(object.get("pose")) {
        Some(said) => Pose::from_word(&said)
            .map(LoosePose::Known)
            .unwrap_or(LoosePose::Unknown(said)),
        None => LoosePose::Known(Pose::Standing),
    };
    Figure {
        id,
        pose,
        facing: word(object.get("facing"))
            .as_deref()
            .and_then(Facing::from_word),
        at: word(object.get("at")).as_deref().and_then(Slot::from_word),
        label: text(object.get("label")),
        sign: text(object.get("sign")),
    }
}

fn prop(object: &Map<String, Value>) -> Option<Prop> {
    let said = text(object.get("kind"))?;
    let kind = PropKind::from_word(&said)
        .map(LooseProp::Known)
        .unwrap_or(LooseProp::Unknown(said));
    Some(Prop {
        kind,
        at: word(object.get("at")).as_deref().and_then(Slot::from_word),
        text: text(object.get("text")),
    })
}

fn bubble(object: &Map<String, Value>) -> Option<Bubble> {
    let text = text(object.get("text"))?;
    Some(Bubble {
        by: word(object.get("by")).unwrap_or_else(|| "off".into()),
        text,
        kind: word(object.get("kind"))
            .as_deref()
            .and_then(BubbleKind::from_word)
            .unwrap_or_default(),
    })
}

/// The objects of a list; anything else in it is dropped.
fn list(value: Option<&Value>) -> impl Iterator<Item = &Map<String, Value>> {
    value
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_object)
}

/// Text, trimmed, never empty; a number is read as its digits.
fn text(value: Option<&Value>) -> Option<String> {
    let said = match value? {
        Value::String(s) => s.trim().to_string(),
        Value::Number(n) => n.to_string(),
        _ => return None,
    };
    (!said.is_empty()).then_some(said)
}

/// A word: text, lowercase.
fn word(value: Option<&Value>) -> Option<String> {
    text(value).map(|s| s.to_lowercase())
}
