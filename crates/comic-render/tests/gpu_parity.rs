//! The GPU draws what the CPU draws, within a few levels: skipped, with its reason, where there is no GPU.
#![cfg(feature = "gpu")]

use comic_core::{Script, Style, compose};
use comic_render::compare::compare;
use comic_render::{CpuRenderer, GpuRenderer, RenderOptions, Renderer};

const HASHMAP: &str = include_str!("../../../Schema/examples/hashmap.comic.json");
const EVERY: &str = include_str!("../../comic-core/tests/fixtures/every-pose-and-prop.comic.json");

fn parity(json: &str, seed: u64, options: RenderOptions) {
    let mut gpu = match GpuRenderer::new() {
        Ok(gpu) => gpu,
        Err(why) => {
            println!("skipped: {why}");
            return;
        }
    };
    let script = Script::from_json(json.as_bytes()).expect("reads");
    let composed = compose(&script, &Style::default(), seed);
    let on_gpu = gpu
        .render(&composed.scene, &options)
        .expect("the GPU draws");
    let on_cpu = CpuRenderer::new()
        .render(&composed.scene, &options)
        .expect("the CPU draws");
    let diff = compare(&on_gpu, &on_cpu, 3).expect("same size");
    println!("{}: {diff:?}", gpu.adapter());
    assert!(diff.max <= 8, "{diff:?}");
    assert!(diff.share() <= 0.0005, "{diff:?}");
}

#[test]
fn the_gpu_draws_the_fixture_as_the_cpu_does() {
    parity(HASHMAP, 7, RenderOptions::strip());
}

#[test]
fn the_gpu_draws_every_pose_and_prop_as_the_cpu_does() {
    parity(EVERY, 3, RenderOptions::strip());
}

#[test]
fn the_gpu_draws_on_nothing_as_the_cpu_does() {
    parity(HASHMAP, 7, RenderOptions::transparent());
}
