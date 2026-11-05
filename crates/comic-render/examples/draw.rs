//! Draws a script to a PNG, for looking at the drawing while it is worked on:
//!
//!     cargo run --release -p comic-render --example draw -- <script.comic.json> <out.png> [gpu|cpu] [seed]
//!
//! Prints the findings, the problems and the timings.

use comic_core::{Script, Style, compose};
use comic_render::{Choice, RenderOptions, choose, png};
use std::time::Instant;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        eprintln!("usage: draw <script.comic.json> <out.png> [gpu|cpu|auto] [seed]");
        std::process::exit(2);
    }
    let bytes = std::fs::read(&args[1]).expect("the script reads");
    let script = Script::from_json(&bytes).unwrap_or_else(|e| panic!("{e}"));
    let choice = args
        .get(3)
        .and_then(|w| Choice::parse(w))
        .unwrap_or(Choice::Auto);
    let seed = args
        .get(4)
        .and_then(|s| s.parse().ok())
        .unwrap_or_else(|| comic_core::rng::hash(script.title.as_deref().unwrap_or("comic")));
    for finding in script.check() {
        println!("check: {}", finding.sentence);
    }
    let (drawn, problems) = script.clamped();
    for problem in &problems {
        println!("problem: {} — {}", problem.kind, problem.text);
    }
    let t = Instant::now();
    let composed = compose(&drawn, &Style::default(), seed);
    let compose_ms = t.elapsed().as_secs_f64() * 1000.0;
    for finding in &composed.findings {
        println!("finding: {}", finding.sentence);
    }
    let t = Instant::now();
    let (mut renderer, fallback) = choose(choice).unwrap_or_else(|e| panic!("{e}"));
    let setup_ms = t.elapsed().as_secs_f64() * 1000.0;
    if let Some(p) = fallback {
        println!("problem: {} — {}", p.kind, p.text);
    }
    let t = Instant::now();
    let raster = renderer
        .render(&composed.scene, &RenderOptions::strip())
        .unwrap_or_else(|e| panic!("{e}"));
    let render_ms = t.elapsed().as_secs_f64() * 1000.0;
    let t = Instant::now();
    let bytes = png::encode(&raster, Some(&drawn.to_json()));
    let png_ms = t.elapsed().as_secs_f64() * 1000.0;
    std::fs::write(&args[2], &bytes).expect("the PNG writes");
    println!(
        "{} {}x{} · {} capsules · compose {compose_ms:.1} ms · {} setup {setup_ms:.1} ms · render {render_ms:.1} ms · png {png_ms:.1} ms · {} KB · hash {:016x}",
        renderer.name(),
        raster.width,
        raster.height,
        composed.scene.capsule_count(),
        renderer.name(),
        bytes.len() / 1024,
        raster.hash()
    );
}
