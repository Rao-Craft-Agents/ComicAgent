use comic_core::geom::Vec2;
use comic_core::{Capsule, Layer, Scene, Script, Style, compose, spec_png};
use comic_render::bins::{TILE, bin};
use comic_render::compare::compare;
use comic_render::{
    Choice, CpuRenderer, Raster, RenderError, RenderOptions, Renderer, choose, png,
};

const HASHMAP: &str = include_str!("../../../Schema/examples/hashmap.comic.json");

fn one_line() -> Scene {
    let capsule = Capsule {
        a: Vec2::new(10.0, 20.25),
        b: Vec2::new(50.0, 20.25),
        r: 2.0,
    };
    Scene {
        width: 64,
        height: 48,
        layers: vec![
            Layer {
                capsules: vec![capsule],
                fills: vec![],
            },
            Layer::default(),
        ],
    }
}

#[test]
fn a_capsule_is_listed_only_in_the_tiles_it_touches() {
    let bins = bin(&one_line());
    assert_eq!((bins.tiles_x, bins.tiles_y, bins.layers), (4, 3, 2));
    for ty in 0..3 {
        for tx in 0..4 {
            let count = bins.tile(0, tx, ty).capsule_count;
            let touches = ty == 1 && tx <= 3 && (tx * TILE) as f32 <= 53.0;
            assert_eq!(count, u32::from(touches), "tile {tx},{ty}");
            assert_eq!(bins.tile(1, tx, ty).capsule_count, 0);
        }
    }
}

#[test]
fn ink_is_black_on_white_with_a_soft_edge() {
    let raster = CpuRenderer::new()
        .render(&one_line(), &RenderOptions::strip())
        .expect("draws");
    let at = |x: u32, y: u32| &raster.rgba[((y * raster.width + x) * 4) as usize..][..4];
    assert_eq!(at(30, 20), [0, 0, 0, 255]);
    assert_eq!(at(30, 5), [255, 255, 255, 255]);
    let edge = at(30, 22)[0];
    assert!(edge > 0 && edge < 255, "the edge is anti-aliased: {edge}");
}

#[test]
fn on_nothing_ink_is_opaque_and_the_rest_clear() {
    let raster = CpuRenderer::new()
        .render(&one_line(), &RenderOptions::transparent())
        .expect("draws");
    let at = |x: u32, y: u32| &raster.rgba[((y * raster.width + x) * 4) as usize..][..4];
    assert_eq!(at(30, 20), [0, 0, 0, 255]);
    assert_eq!(at(30, 5), [0, 0, 0, 0]);
    assert!(!raster.is_opaque());
}

#[test]
fn a_png_reads_back_with_its_script() {
    let script = Script::from_json(HASHMAP.as_bytes()).expect("reads");
    let composed = compose(&script, &Style::default(), 7);
    let raster = CpuRenderer::new()
        .render(&composed.scene, &RenderOptions::strip())
        .expect("draws");
    let bytes = png::encode(&raster, Some(&script.to_json()));
    assert_eq!(bytes[25], 2, "an opaque strip is RGB");
    assert_eq!(png::decode(&bytes).expect("decodes"), raster);
    let carried = spec_png::embedded(&bytes).expect("the script rides inside");
    assert_eq!(
        Script::from_json(carried.as_bytes()).expect("reads"),
        script
    );
    let icon = CpuRenderer::new()
        .render(&one_line(), &RenderOptions::transparent())
        .expect("draws");
    let bytes = png::encode(&icon, None);
    assert_eq!(bytes[25], 6, "transparency is RGBA");
    assert_eq!(png::decode(&bytes).expect("decodes"), icon);
    assert!(png::decode(b"not a png").is_err());
}

#[test]
fn compare_finds_a_picture_moved_by_a_pixel() {
    let raster = CpuRenderer::new()
        .render(&one_line(), &RenderOptions::strip())
        .expect("draws");
    let same = compare(&raster, &raster, 0).expect("same size");
    assert_eq!((same.max, same.over, same.total), (0, 0, 64 * 48));
    let mut moved = raster.rgba.clone();
    moved.rotate_right(4);
    let shifted = Raster {
        rgba: moved,
        ..raster.clone()
    };
    let diff = compare(&raster, &shifted, 3).expect("same size");
    assert!(diff.max > 100 && diff.over > 0);
    let small = Raster {
        width: 1,
        height: 1,
        rgba: vec![0; 4],
    };
    assert!(compare(&raster, &small, 0).is_err());
}

#[test]
fn a_renderer_is_chosen_by_word() {
    assert_eq!(Choice::parse(" GPU "), Some(Choice::Gpu));
    assert_eq!(Choice::parse("metal"), None);
    assert_eq!(Choice::from_env_or(Some("cpu")), Choice::Cpu);
    let (renderer, problem) = choose(Choice::Cpu).expect("the CPU is always there");
    assert_eq!((renderer.name(), problem), ("cpu", None));
    let (renderer, problem) = choose(Choice::Auto).expect("auto always draws");
    match renderer.name() {
        "gpu" => assert_eq!(problem, None),
        _ => assert_eq!(problem.map(|p| p.kind), Some("fallback".to_string())),
    }
}

#[test]
fn a_picture_too_large_is_refused() {
    let scene = Scene {
        width: 9000,
        height: 10,
        layers: vec![],
    };
    assert_eq!(
        CpuRenderer::new().render(&scene, &RenderOptions::strip()),
        Err(RenderError::TooLarge {
            width: 9000,
            height: 10
        })
    );
}
