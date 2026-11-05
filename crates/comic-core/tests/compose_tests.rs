use comic_core::geom::Vec2;
use comic_core::rng::SplitMix64;
use comic_core::wobble::Wobble;
use comic_core::{FindingKind, Script, Style, compose, spec_png};

const HASHMAP: &str = include_str!("../../../Schema/examples/hashmap.comic.json");
const EVERY: &str = include_str!("fixtures/every-pose-and-prop.comic.json");

fn script(json: &str) -> Script {
    Script::from_json(json.as_bytes()).expect("the script reads")
}

#[test]
fn the_fixture_composes_clean_at_seed_7() {
    let composed = compose(&script(HASHMAP), &Style::default(), 7);
    assert!(composed.findings.is_empty(), "{:?}", composed.findings);
    assert!(composed.problems.is_empty());
    assert_eq!(composed.width, 3200);
    assert_eq!(
        (composed.scene.width, composed.scene.height),
        (composed.width, composed.height)
    );
    assert_eq!(composed.scene.layers.len(), 2);
    assert!(composed.scene.capsule_count() > 1000);
}

#[test]
fn every_pose_and_prop_draws_clean() {
    let s = script(EVERY);
    assert!(s.check().is_empty(), "{:?}", s.check());
    let composed = compose(&s, &Style::default(), 3);
    assert!(composed.findings.is_empty(), "{:?}", composed.findings);
    assert!(
        !composed.scene.layers[1].fills.is_empty(),
        "clouds, thoughts and signs have paper behind them"
    );
}

#[test]
fn a_seed_draws_the_same_picture_and_another_seed_another() {
    let s = script(HASHMAP);
    let a = compose(&s, &Style::default(), 7);
    assert_eq!(a, compose(&s, &Style::default(), 7));
    assert_ne!(a.scene, compose(&s, &Style::default(), 8).scene);
}

#[test]
fn revising_one_panel_reshakes_only_that_panel() {
    let s = script(HASHMAP);
    let mut revised = s.clone();
    revised.panels[2].bubbles[0].text = "So I hash it again and look in one bucket?".into();
    let (a, b) = (
        compose(&s, &Style::default(), 7),
        compose(&revised, &Style::default(), 7),
    );
    let (wa, wb) = (&a.scene.layers[0].capsules, &b.scene.layers[0].capsules);
    assert_eq!(wa, wb, "the world layer holds no bubble");
    let (oa, ob) = (&a.scene.layers[1].capsules, &b.scene.layers[1].capsules);
    let first_two_panels = oa.iter().take_while(|c| c.a.x < 2100.0).count();
    assert!(first_two_panels > 100);
    assert_eq!(oa[..first_two_panels], ob[..first_two_panels]);
}

#[test]
fn strips_are_as_wide_as_their_panel_count_says() {
    let panel = r#"{"figures": [{"id": "a"}], "bubbles": [{"by": "a", "text": "hi"}]}"#;
    let mut heights = Vec::new();
    for n in 1..=6 {
        let s = script(&format!(r#"{{"panels": [{}]}}"#, vec![panel; n].join(",")));
        let composed = compose(&s, &Style::default(), 1);
        assert_eq!(
            composed.width,
            if n == 1 { 1600 } else { 3200 },
            "{n} panels"
        );
        assert!(
            composed.findings.is_empty(),
            "{n} panels: {:?}",
            composed.findings
        );
        heights.push(composed.height);
    }
    assert!(heights[3] > heights[2], "four panels are two rows");
    assert!(heights[5] > heights[2]);
}

#[test]
fn too_much_said_is_measured_and_named() {
    let long = "This is a very long thing to say in one bubble, longer than any bubble holds, and it goes on and on and on.";
    let s = script(&format!(
        r#"{{"panels": [{{"figures": [{{"id": "a"}}, {{"id": "b"}}], "bubbles": [
            {{"by": "a", "text": "{long} {long}"}}, {{"by": "b", "text": "{long}"}}, {{"by": "a", "text": "{long}"}}]}},
            {{"figures": [{{"id": "a"}}]}}]}}"#
    ));
    let composed = compose(&s, &Style::default(), 1);
    let overflow: Vec<&str> = composed
        .findings
        .iter()
        .filter(|f| f.kind == FindingKind::Overflow)
        .map(|f| f.sentence.as_str())
        .collect();
    assert!(
        overflow.iter().any(
            |f| f.starts_with("panel 1's bubble by a runs to") && f.contains("say it in about")
        ),
        "{overflow:?}"
    );
    assert!(
        composed
            .findings
            .iter()
            .any(|f| f.sentence.contains("px of sky") || f.kind == FindingKind::Overlap),
        "{:?}",
        composed.findings
    );
}

#[test]
fn the_hand_wanders_a_little_and_keeps_its_ends() {
    let line = vec![Vec2::new(0.0, 0.0), Vec2::new(200.0, 0.0)];
    let drawn = Wobble::INK.apply(&line, &mut SplitMix64::new(9));
    assert!(drawn.len() > 20);
    assert_eq!((drawn[0], drawn[drawn.len() - 1]), (line[0], line[1]));
    assert!(
        drawn
            .iter()
            .all(|p| p.y.abs() <= Wobble::INK.amplitude + 1e-4)
    );
    assert!(drawn.iter().any(|p| p.y.abs() > 0.1));
}

#[test]
fn the_script_rides_inside_the_png() {
    let fake = |chunks: &[(&[u8; 4], &[u8])]| {
        let mut png = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
        for (kind, data) in chunks {
            png.extend_from_slice(&(data.len() as u32).to_be_bytes());
            png.extend_from_slice(*kind);
            png.extend_from_slice(data);
            let mut typed = kind.to_vec();
            typed.extend_from_slice(data);
            png.extend_from_slice(&spec_png::crc32(&typed).to_be_bytes());
        }
        png
    };
    let png = fake(&[(b"IHDR", &[0; 13]), (b"IEND", &[])]);
    assert_eq!(spec_png::embedded(&png), None);
    let json = script(HASHMAP).to_json();
    let carried = spec_png::embed(&png, &json);
    assert_eq!(spec_png::embedded(&carried).as_deref(), Some(json.as_str()));
    assert!(carried.ends_with(&png[png.len() - 12..]), "IEND stays last");
    assert_eq!(spec_png::embed(b"not a png", &json), b"not a png".to_vec());
    assert_eq!(spec_png::crc32(b"IEND"), 0xAE42_6082);
}
