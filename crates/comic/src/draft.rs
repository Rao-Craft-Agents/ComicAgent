//! What the host's model is asked: the rules of a good strip, an example on another subject, the script's JSON
//! Schema, and the three asks (draft, change, revise).
//!
//! PIN: NEVER LEAK THE EXAMPLE. The example is a bakery's strip; a draft that copies its title, or three of its
//! word pairs in one panel, is sent back as a finding (`echoes`), as Plate's `PlateSheetDraft.echoes` does.

use std::collections::BTreeSet;

use comic_core::{Finding, FindingKind, Script};
use serde_json::{Map, Value};

pub const RULES: &str = r#"You write one short stick-figure comic strip that explains one concept the way a webcomic does: a few panels, one or two figures, a line or two each, a turn at the end. You answer with the strip's script, one JSON object. The kit draws it; you decide what it says.

How a good strip reads:
- One idea. Three to six panels, or one for a single gag. Each panel is one beat; the last lands the point or the joke.
- `title` names the strip (at most 48 characters). `alt` (at most 240) is the aside a reader finds after: a second joke, the caveat, the footnote.
- A panel has one to three `figures`. Each has an `id` (short, lowercase, the same across panels for the same person) and a `pose` from the catalogue: standing, pointing, arms-up, shrugging, facepalm, walking, sitting, lying-down, holding-sign, waving. `facing` (left, right) and `at` (left, center, right) only when they matter. `label` (at most 20 characters) under a figure only when who they are matters; `sign` (at most 24) only with holding-sign.
- `bubbles` are what is said, in order, at most three a panel. `by` is a figure's id in that panel, or "off" for a voice from outside it. `text` is at most 110 characters, and shorter is funnier. `kind` is speech (left out), thought, or shout.
- `caption` (at most 140 characters) is the narrator's box `above` or `below` the drawing: the setup, the time, the name of a step. Not every panel needs one.
- `props` (at most three a panel): desk, laptop, whiteboard, box, arrow, cloud, graph, door, tree, signpost. `at` says where it stands; `text` is what a whiteboard, signpost, cloud or arrow says (at most 48 characters), or for a graph one of up, down, spike, wiggle. A figure sitting at a desk needs a desk at its slot.
- Say it in your own words, for this concept and this person's ask. State facts only when you are sure of them; the humour is in the figures' plain lines, never in a real person's name."#;

/// A strip on another subject: the shape to copy, never the words.
pub const EXAMPLE: &str = r#"{"title":"Why bread rises","alt":"The yeast does not know it is making bread. It thinks it is making beer and getting it slightly wrong.","panels":[{"caption":{"text":"A bowl of flour and water, left on the counter.","at":"above"},"figures":[{"id":"baker","pose":"pointing","at":"left"}],"props":[{"kind":"whiteboard","at":"right","text":"FLOUR + WATER + TIME"}],"bubbles":[{"by":"baker","text":"The yeast eats the sugar in the flour."}]},{"figures":[{"id":"friend","pose":"standing","at":"left"},{"id":"baker","pose":"shrugging","at":"right"}],"bubbles":[{"by":"friend","text":"And then?"},{"by":"baker","text":"It burps. Carbon dioxide, a few billion times."}]},{"caption":{"text":"Six hours later.","at":"below"},"figures":[{"id":"friend","pose":"facepalm","at":"left"},{"id":"baker","pose":"arms-up","at":"right"}],"props":[{"kind":"desk","at":"center"},{"kind":"box","at":"center"}],"bubbles":[{"by":"baker","text":"The burps are trapped. That is the loaf."},{"by":"friend","text":"I am never eating bread again."}]}]}"#;

const STANDARD: &str = include_str!("../../../Schema/comic.schema.json");

pub fn system() -> String {
    format!(
        "{RULES}\n\nA strip on another subject, to copy the shape of and never the words:\n{EXAMPLE}"
    )
}

pub fn revise_system() -> String {
    format!(
        "{RULES}\n\nYou are fixing a script you wrote, after it was drawn and measured. Keep every panel the findings do not name; make the fewest changes that clear every finding. Answer with the whole script, fixed."
    )
}

/// The first ask: what the person asked, what to explain, how many panels.
pub fn draft(ask: Option<&str>, brief: Option<&str>, panels: Option<u32>) -> String {
    let mut parts = Vec::new();
    if let Some(ask) = ask.filter(|ask| !ask.trim().is_empty()) {
        parts.push(format!("The person asked: \"{}\"", ask.trim()));
    }
    if let Some(brief) = brief.filter(|brief| !brief.trim().is_empty()) {
        parts.push(format!("What to explain: {}", brief.trim()));
    }
    if let Some(panels) = panels {
        parts.push(format!(
            "Draw it in {panels} panel{}.",
            if panels == 1 { "" } else { "s" }
        ));
    }
    parts.push("Write the strip's script: a comic that explains it, in your own words.".into());
    parts.join("\n\n")
}

/// A strip drawn before, changed as the brief asks.
pub fn change(script: &Script, brief: &str) -> String {
    format!(
        "The script of a strip drawn before:\n{}\n\nWhat to change: {}\n\nWrite the whole script again, changed as asked and nothing else.",
        script.to_json(),
        brief.trim()
    )
}

/// The script back, with what its drawing measured.
pub fn revise(script: &Script, findings: &[Finding]) -> String {
    let listed: Vec<String> = findings
        .iter()
        .take(16)
        .map(|finding| format!("- {}", finding.sentence))
        .collect();
    format!(
        "The script you wrote:\n{}\n\nWhat the strip measured:\n{}\n\nWrite the whole script again, fixed.",
        script.to_json(),
        listed.join("\n")
    )
}

/// The standard's schema as the model is sent it: its references inlined, `schema` left to the agent, and the
/// lengths and patterns said in the rules rather than enforced by a decoder.
pub fn ask_schema() -> Value {
    let standard: Value = serde_json::from_str(STANDARD).expect("Schema/comic.schema.json reads");
    let definitions = standard["$defs"].clone();
    let mut asked = inline(&standard, &definitions);
    if let Value::Object(map) = &mut asked {
        for key in ["$id", "$schema", "$defs", "description"] {
            map.remove(key);
        }
        if let Some(Value::Object(properties)) = map.get_mut("properties") {
            properties.remove("schema");
        }
        map.insert(
            "required".into(),
            Value::Array(vec![Value::String("panels".into())]),
        );
    }
    asked
}

fn inline(value: &Value, definitions: &Value) -> Value {
    match value {
        Value::Object(map) => {
            if let Some(Value::String(reference)) = map.get("$ref")
                && let Some(name) = reference.strip_prefix("#/$defs/")
            {
                return inline(&definitions[name], definitions);
            }
            let mut out = Map::new();
            for (key, value) in map {
                if matches!(
                    key.as_str(),
                    "maxLength" | "pattern" | "minItems" | "maxItems"
                ) {
                    continue;
                }
                out.insert(key.clone(), inline(value, definitions));
            }
            Value::Object(out)
        }
        Value::Array(items) => {
            Value::Array(items.iter().map(|item| inline(item, definitions)).collect())
        }
        other => other.clone(),
    }
}

fn words(text: &str) -> Vec<String> {
    text.split(|c: char| !c.is_alphanumeric())
        .filter(|word| word.chars().count() > 3)
        .map(str::to_lowercase)
        .collect()
}

fn pairs(texts: &[&str]) -> BTreeSet<String> {
    let mut pairs = BTreeSet::new();
    for text in texts {
        let words = words(text);
        for pair in words.windows(2) {
            pairs.insert(format!("{} {}", pair[0], pair[1]));
        }
    }
    pairs
}

fn panel_texts(panel: &comic_core::script::Panel) -> Vec<&str> {
    let mut texts: Vec<&str> = panel
        .bubbles
        .iter()
        .map(|bubble| bubble.text.as_str())
        .collect();
    texts.extend(panel.caption.as_ref().map(|caption| caption.text.as_str()));
    texts.extend(panel.props.iter().filter_map(|prop| prop.text.as_deref()));
    texts
}

/// Where a draft copied the example: its title, or three of its word pairs in one panel.
pub fn echoes(script: &Script) -> Vec<Finding> {
    let example = Script::from_json(EXAMPLE.as_bytes()).expect("the example is a strip");
    let mut found = Vec::new();
    if let (Some(title), Some(theirs)) = (&script.title, &example.title)
        && title.trim().eq_ignore_ascii_case(theirs)
    {
        found.push(Finding::new(
            FindingKind::Limit,
            "title",
            vec![],
            "the title is the example's: write this strip's own",
        ));
    }
    let known: BTreeSet<String> = example
        .panels
        .iter()
        .flat_map(|panel| pairs(&panel_texts(panel)))
        .collect();
    for (index, panel) in script.panels.iter().enumerate() {
        let shared: Vec<String> = pairs(&panel_texts(panel))
            .intersection(&known)
            .cloned()
            .collect();
        if shared.len() >= 3 {
            found.push(Finding::new(
                FindingKind::Limit,
                format!("panel:{}", index + 1),
                vec![shared.len() as i32],
                format!(
                    "panel {} says the example's words ({}): write this strip's own",
                    index + 1,
                    shared[..3].join(", ")
                ),
            ));
        }
    }
    found
}
