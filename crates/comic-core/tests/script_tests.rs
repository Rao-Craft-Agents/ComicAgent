use comic_core::script::{
    BubbleKind, CaptionAt, Limits, LoosePose, LooseProp, Pose, PropKind, ScriptError, Slot,
};
use comic_core::{FindingKind, SCHEMA, Script};

const HASHMAP: &str = include_str!("../../../Schema/examples/hashmap.comic.json");

fn script(json: &str) -> Script {
    Script::from_json(json.as_bytes()).expect("the script reads")
}

#[test]
fn the_fixture_reads_and_keeps_the_standard() {
    let s = script(HASHMAP);
    assert_eq!(s.schema, SCHEMA);
    assert_eq!(s.title.as_deref(), Some("How a hash map finds a key"));
    assert_eq!(s.panels.len(), 3);
    assert_eq!(s.cast(), vec!["ana", "ben"]);
    assert_eq!(
        s.panels[0].caption.as_ref().map(|c| c.at),
        Some(CaptionAt::Above)
    );
    assert_eq!(
        s.panels[2].caption.as_ref().map(|c| c.at),
        Some(CaptionAt::Below)
    );
    assert_eq!(s.panels[2].figures[0].pose, LoosePose::Known(Pose::Sitting));
    assert_eq!(
        s.panels[2].props[1].kind,
        LooseProp::Known(PropKind::Laptop)
    );
    assert_eq!(s.panels[0].figures[0].at, Some(Slot::Left));
    assert!(s.check().is_empty(), "{:?}", s.check());
    assert_eq!(s.slug(), "how-a-hash-map-finds-a-key");
}

#[test]
fn what_cannot_be_a_strip_is_refused_with_its_reason() {
    assert!(matches!(
        Script::from_json(b"not json"),
        Err(ScriptError::NotJson(_))
    ));
    assert_eq!(Script::from_json(b"[1, 2]"), Err(ScriptError::NotAnObject));
    assert_eq!(
        Script::from_json(br#"{"schema": "craft-timeseries/1", "panels": [{}]}"#),
        Err(ScriptError::Schema("craft-timeseries/1".into()))
    );
    assert_eq!(
        Script::from_json(br#"{"panels": []}"#),
        Err(ScriptError::NoPanels)
    );
    assert_eq!(
        Script::from_json(br#"{"panels": ["a panel", 3]}"#),
        Err(ScriptError::NoPanels)
    );
    assert!(ScriptError::NoPanels.to_string().contains("1 to 6"));
}

#[test]
fn a_model_s_script_is_read_loosely() {
    let s = script(
        r#"{"title": "  Loose  ", "panels": [
            "not a panel",
            {"caption": "a plain caption",
             "figures": [{"pose": "Arms Up", "facing": "LEFT", "at": "middle"}, {"id": "Bo", "pose": "dancing"}, 7],
             "props": [{"kind": "WhiteBoard", "text": 42}, {"text": "no kind"}, {"kind": "sofa"}],
             "bubbles": [{"by": "bo", "text": "hi", "kind": "yell"}, {"by": "bo"}, {"text": "who said this"}]}
        ]}"#,
    );
    assert_eq!(s.schema, SCHEMA, "a missing schema reads as the standard");
    assert_eq!(s.title.as_deref(), Some("Loose"));
    let p = &s.panels[0];
    assert_eq!(
        p.caption.as_ref().map(|c| (c.text.as_str(), c.at)),
        Some(("a plain caption", CaptionAt::Above))
    );
    assert_eq!(p.figures.len(), 2);
    assert_eq!(p.figures[0].id, "figure-1");
    assert_eq!(p.figures[0].pose, LoosePose::Known(Pose::ArmsUp));
    assert_eq!(p.figures[0].at, Some(Slot::Center));
    assert_eq!(p.figures[1].id, "bo", "ids are lowercase");
    assert_eq!(p.figures[1].pose, LoosePose::Unknown("dancing".into()));
    assert_eq!(p.props.len(), 2, "a prop without a kind is dropped");
    assert_eq!(p.props[0].text.as_deref(), Some("42"));
    assert_eq!(p.props[1].kind, LooseProp::Unknown("sofa".into()));
    assert_eq!(p.bubbles.len(), 2, "a bubble without text is dropped");
    assert_eq!(p.bubbles[0].kind, BubbleKind::Shout);
    assert_eq!(
        p.bubbles[1].by, "off",
        "a bubble by no one is a voice from outside"
    );
}

#[test]
fn check_names_what_to_fix_and_clamped_draws_it_anyway() {
    let long = "word ".repeat(40);
    let json = format!(
        r#"{{"panels": [{{"figures": [{{"id": "a", "pose": "dancing"}}],
             "props": [{{"kind": "sofa"}}],
             "bubbles": [{{"by": "a", "text": "{long}"}}, {{"by": "zed", "text": "hello"}}]}}]}}"#
    );
    let s = script(&json);
    let found = s.check();
    let kinds: Vec<FindingKind> = found.iter().map(|f| f.kind).collect();
    assert_eq!(
        kinds,
        vec![
            FindingKind::Unknown,
            FindingKind::Limit,
            FindingKind::Unknown,
            FindingKind::Unknown
        ],
        "{found:?}"
    );
    assert!(
        found[0]
            .sentence
            .contains("pose dancing, which the catalogue lacks: use one of standing, pointing")
    );
    assert_eq!(found[1].element, "panel:1/bubble:1");
    assert_eq!(found[1].numbers, vec![199, 110, 90]);
    assert!(
        found[1].sentence.ends_with("say it in about 90"),
        "{}",
        found[1].sentence
    );
    assert!(
        found[2]
            .sentence
            .contains("no figure in panel 1 has that id")
    );
    assert!(
        found[3]
            .sentence
            .contains("a sofa, which the catalogue lacks")
    );

    let (drawn, problems) = s.clamped();
    let p = &drawn.panels[0];
    assert_eq!(p.figures[0].pose, LoosePose::Known(Pose::Standing));
    assert!(
        p.bubbles[0].text.chars().count() <= Limits::BUBBLE && p.bubbles[0].text.ends_with("...")
    );
    assert_eq!(p.bubbles[1].by, "off");
    assert!(p.props.is_empty());
    let kinds: Vec<&str> = problems.iter().map(|p| p.kind.as_str()).collect();
    assert_eq!(
        kinds,
        vec!["fallback", "short", "fallback", "skipped"],
        "{problems:?}"
    );
    assert!(
        drawn.check().is_empty(),
        "a clamped script keeps the standard: {:?}",
        drawn.check()
    );
}

#[test]
fn too_many_panels_is_said_and_cut_to_six() {
    let panels = [r#"{"figures": [{"id": "a"}]}"#; 8].join(",");
    let s = script(&format!(r#"{{"panels": [{panels}]}}"#));
    assert_eq!(s.check()[0].numbers, vec![8, 6]);
    let (drawn, problems) = s.clamped();
    assert_eq!(drawn.panels.len(), 6);
    assert_eq!(
        problems[0].text,
        "panels 7 to 8 were not drawn: a strip holds 6"
    );
}

#[test]
fn json_out_is_sorted_stamped_and_reads_back() {
    let s = script(HASHMAP);
    let json = s.to_json();
    assert!(json.starts_with(r#"{"alt":"#), "{json}");
    assert!(json.contains(r#""schema":"craft-comic/1""#));
    assert_eq!(Script::from_json(json.as_bytes()).expect("reads back"), s);
    assert_eq!(
        serde_json::from_str::<Script>(&json).expect("serde reads it"),
        s
    );
}

#[test]
fn a_slug_is_a_file_name() {
    let mut s = script(HASHMAP);
    s.title = Some("Why does   O(1) beat O(n)?!".into());
    assert_eq!(s.slug(), "why-does-o-1-beat-o-n");
    s.title = None;
    assert_eq!(s.slug(), "comic");
    s.title = Some("a ".repeat(60));
    assert!(s.slug().len() <= 48);
}

#[test]
fn about_is_four_fifths_to_a_multiple_of_five() {
    assert_eq!(Limits::about(110), 90);
    assert_eq!(Limits::about(48), 35);
    assert_eq!(Limits::about(140), 110);
}
