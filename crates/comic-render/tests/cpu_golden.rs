//! The CPU's picture of the fixture is pinned: its pixels' hash and a golden PNG. `COMIC_UPDATE_GOLDEN=1` writes them
//! again and prints the hash to paste here.

use comic_core::{Script, Style, compose};
use comic_render::compare::compare;
use comic_render::{CpuRenderer, RenderOptions, Renderer, png};

const HASHMAP: &str = include_str!("../../../Schema/examples/hashmap.comic.json");
const GOLDEN: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/golden/hashmap.cpu.png");
const GOLDEN_HASH: u64 = 0x4d04_9cde_ba57_11e0;

#[test]
fn the_fixture_draws_the_golden_picture_on_the_cpu() {
    let script = Script::from_json(HASHMAP.as_bytes()).expect("reads");
    let composed = compose(&script, &Style::default(), 7);
    let raster = CpuRenderer::new()
        .render(&composed.scene, &RenderOptions::strip())
        .expect("draws");
    assert_eq!(
        (raster.width, raster.height),
        (composed.width, composed.height)
    );
    assert!(raster.is_opaque());
    if std::env::var("COMIC_UPDATE_GOLDEN").is_ok() {
        std::fs::write(GOLDEN, png::encode(&raster, Some(&script.to_json())))
            .expect("writes the golden");
        println!("golden hash: 0x{:016x}", raster.hash());
        return;
    }
    assert_eq!(
        raster.hash(),
        GOLDEN_HASH,
        "the CPU's picture changed: look at it, then COMIC_UPDATE_GOLDEN=1"
    );
    let golden =
        png::decode(&std::fs::read(GOLDEN).expect("the golden is there")).expect("decodes");
    let diff = compare(&raster, &golden, 0).expect("same size");
    assert_eq!(diff.over, 0, "{diff:?}");
}

#[test]
fn one_thread_draws_what_many_draw() {
    let script = Script::from_json(HASHMAP.as_bytes()).expect("reads");
    let composed = compose(&script, &Style::default(), 7);
    let many = CpuRenderer::new()
        .render(&composed.scene, &RenderOptions::strip())
        .expect("draws");
    let one = CpuRenderer { threads: 1 }
        .render(&composed.scene, &RenderOptions::strip())
        .expect("draws");
    assert_eq!(one, many);
}
