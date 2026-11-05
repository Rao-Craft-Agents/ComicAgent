use comic_core::geom::Vec2;
use comic_core::hershey::HersheyFont;
use comic_core::lettering::{Align, Lettering};
use comic_core::rng::SplitMix64;

#[test]
fn simplex_has_every_printable_ascii_glyph() {
    let font = HersheyFont::simplex();
    assert_eq!(font.len(), 96);
    for c in ' '..='~' {
        assert!(font.glyph(c).is_some(), "{c:?}");
    }
    let h = font.glyph('H').expect("H");
    assert_eq!((h.left, h.right, h.strokes.len()), (-11, 11, 3));
    assert!(font.glyph('é').is_none());
}

#[test]
fn a_record_may_run_on_to_the_next_line() {
    let line = "    1  9MWOMOV RUMUV ROQUQ";
    let split = "    1  9MWOMOV RU\nMUV ROQUQ";
    let one = HersheyFont::parse(line).err();
    assert!(one.is_some(), "one glyph is not an ASCII font");
    let font = |record: &str| HersheyFont::parse(&vec![record; 95].join("\n")).expect("parses");
    assert_eq!(font(line), font(split));
    assert!(HersheyFont::parse("garbage").is_err());
}

#[test]
fn text_is_folded_to_capitals_the_font_has() {
    assert_eq!(
        Lettering::prepare("“Don’t” — café…  ok"),
        "\"DON'T\" - CAFE... OK"
    );
    let hand = Lettering::hand();
    assert!(hand.missing("plain text, 42!").is_empty());
    assert_eq!(hand.missing("a ★ b ★"), vec!['★']);
}

#[test]
fn measure_grows_with_the_text() {
    let hand = Lettering::hand();
    assert_eq!(hand.measure("", 13.0), 0.0);
    let short = hand.measure("HELLO", 13.0);
    assert!(short > 30.0 && short < hand.measure("HELLO WORLD", 13.0));
    assert!((hand.measure("HELLO", 26.0) - 2.0 * short).abs() < 1e-3);
}

#[test]
fn wrap_keeps_every_word_and_every_line_inside_the_width() {
    let hand = Lettering::hand();
    let text = "One look instead of a search. That is the whole trick, and a supercalifragilisticexpialidocious word.";
    let lines = hand.wrap(text, 13.0, 160.0);
    assert!(lines.len() >= 4);
    for line in &lines {
        assert!(hand.measure(line, 13.0) <= 160.0 + 1e-3, "{line}");
    }
    assert_eq!(
        lines.concat().replace(' ', ""),
        Lettering::prepare(text).replace(' ', "")
    );
    assert_eq!(hand.wrap("", 13.0, 100.0), Vec::<String>::new());
}

#[test]
fn a_bubble_s_limit_fits_in_four_lines_of_a_three_panel_strip() {
    let hand = Lettering::hand();
    let width =
        comic_core::Style::default().panel_width(3) * comic_core::Style::default().bubble_width;
    let text = "So to find a key I hash it again, look in that one bucket, and I never read the rest of the table at all";
    assert!(text.len() <= 110);
    assert!(hand.wrap(text, 13.0, width).len() <= 4);
}

#[test]
fn the_hand_is_the_seed_s() {
    let hand = Lettering::hand();
    let lines = vec!["HELLO THERE".to_string()];
    let draw = |seed| {
        hand.strokes(
            &lines,
            Vec2::new(10.0, 10.0),
            13.0,
            Align::Left,
            &mut SplitMix64::new(seed),
        )
    };
    assert_eq!(draw(1), draw(1));
    assert_ne!(draw(1).strokes, draw(2).strokes);
    let drawn = draw(1);
    let xs: Vec<f32> = drawn.strokes.iter().flatten().map(|p| p.x).collect();
    let lo = xs.iter().copied().fold(f32::MAX, f32::min);
    let hi = xs.iter().copied().fold(f32::MIN, f32::max);
    assert!(lo > 8.0 && hi < 10.0 + hand.measure("HELLO THERE", 13.0) + 2.0);
}
