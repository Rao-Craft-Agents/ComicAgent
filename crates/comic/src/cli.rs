//! The commands a person runs at a terminal; anything else is the runner's.

use std::path::{Path, PathBuf};

use comic_core::{Script, Style, compose, spec_png};
use comic_render::{Choice, Raster, RenderOptions};
use granite_agents::{AgentHost, AnyAgentPackage, Entry, RosterEntry, json};
use serde_json::{Value, json};

use crate::ComicAgent;

const ICON: &str = r#"{"schema":"craft-comic/1","panels":[{"figures":[{"id":"ana","pose":"pointing","at":"left"}],"props":[{"kind":"graph","at":"right","text":"up"}],"bubbles":[{"by":"ana","text":"AH."}]}]}"#;

fn option<'a>(arguments: &'a [String], name: &str) -> Option<&'a str> {
    arguments
        .iter()
        .position(|argument| argument == name)
        .and_then(|at| arguments.get(at + 1))
        .map(String::as_str)
}

fn fail(said: impl std::fmt::Display) -> i32 {
    eprintln!("comic: {said}");
    1
}

/// Runs a command, and says how it ended; none when the arguments are the runner's.
pub fn run(arguments: &[String]) -> Option<i32> {
    let command = arguments.get(1)?.as_str();
    let rest = &arguments[2..];
    let code = match command {
        "draw" => draw(rest),
        "describe" => {
            let entry = Entry::of(&RosterEntry::Package(AnyAgentPackage::of::<ComicAgent>()));
            println!(
                "{}",
                String::from_utf8_lossy(&json::pretty(&entry).unwrap_or_default())
            );
            0
        }
        "sample" => {
            print!("{}", crate::SAMPLE);
            0
        }
        "extract" => match rest.first().map(std::fs::read) {
            Some(Ok(bytes)) => match spec_png::embedded(&bytes) {
                Some(script) => {
                    println!("{script}");
                    0
                }
                None => fail("that PNG carries no comic script"),
            },
            Some(Err(error)) => fail(error),
            None => fail("usage: comic extract <strip.png>"),
        },
        "icon" => icon(option(rest, "--out").unwrap_or("Assets/icon.png")),
        "compare" => compare(rest),
        "help" | "--help" | "-h" => {
            eprintln!(
                "usage: comic [draw <script|png> | describe | sample | extract <png> | icon | compare <a> <b>]"
            );
            0
        }
        _ => return None,
    };
    Some(code)
}

fn draw(arguments: &[String]) -> i32 {
    let Some(script) = arguments.first() else {
        return fail(
            "usage: comic draw <script.comic.json | strip.png> [--out x.png] [--seed N] [--renderer gpu|cpu|auto]",
        );
    };
    let folder = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let mut input = json!({"script": script, "directory": folder});
    if let Some(out) = option(arguments, "--out") {
        input["output"] = json!(out);
    }
    if let Some(seed) = option(arguments, "--seed").and_then(|seed| seed.parse::<u64>().ok()) {
        input["seed"] = json!(seed);
    }
    if let Some(renderer) = option(arguments, "--renderer") {
        input["renderer"] = json!(renderer);
    }
    let runtime = tokio::runtime::Runtime::new().expect("a runtime");
    let host = AgentHost::none()
        .with_directory(folder.to_string_lossy())
        .with_note(|line| eprintln!("{line}"));
    let answered = runtime
        .block_on(AnyAgentPackage::of::<ComicAgent>().perform(input.to_string().as_bytes(), host));
    match answered {
        Ok(bytes) => {
            let answer: Value = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
            let shown = json!({
                "path": answer["path"], "width": answer["width"], "height": answer["height"],
                "renderer": answer["renderer"], "findings": answer["findings"], "problems": answer["problems"],
            });
            println!(
                "{}",
                String::from_utf8_lossy(&json::pretty(&shown).unwrap_or_default())
            );
            0
        }
        Err(error) => fail(error),
    }
}

fn icon(out: &str) -> i32 {
    let script = Script::from_json(ICON.as_bytes()).expect("the icon is a strip");
    let composed = compose(&script, &Style::default(), 7);
    let raster = match comic_render::choose(Choice::Auto)
        .and_then(|(mut renderer, _)| renderer.render(&composed.scene, &RenderOptions::strip()))
    {
        Ok(raster) => raster,
        Err(error) => return fail(error),
    };
    let square = rounded_square(&raster, 1024);
    let path = Path::new(out);
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    match std::fs::write(path, comic_render::png::encode(&square, None)) {
        Ok(()) => {
            println!("{}", path.display());
            0
        }
        Err(error) => fail(error),
    }
}

/// The drawing inside the panel's border, set on a white square scaled to `side` by averaging, its corners cut
/// round and outlined in ink so it reads on a light page and a dark one.
fn rounded_square(raster: &Raster, side: u32) -> Raster {
    let ink = |x: u32, y: u32| raster.rgba[((y * raster.width + x) * 4) as usize] < 128;
    // The border is the outermost ink; the drawing is the ink well inside it.
    let (mut left, mut top, mut right, mut bottom) = (raster.width, raster.height, 0, 0);
    for y in 0..raster.height {
        for x in 0..raster.width {
            if ink(x, y) {
                (left, top, right, bottom) = (left.min(x), top.min(y), right.max(x), bottom.max(y));
            }
        }
    }
    let inset = 16;
    let (outer_left, outer_top, outer_right, outer_bottom) = (
        left + inset,
        top + inset,
        right.saturating_sub(inset),
        bottom.saturating_sub(inset),
    );
    let (mut left, mut top, mut right, mut bottom) = (raster.width, raster.height, 0, 0);
    for y in outer_top..outer_bottom {
        for x in outer_left..outer_right {
            if ink(x, y) {
                (left, top, right, bottom) = (left.min(x), top.min(y), right.max(x), bottom.max(y));
            }
        }
    }
    let content = ((right - left).max(bottom - top) as f64) * 1.3;
    let (center_x, center_y) = ((left + right) as f64 / 2.0, (top + bottom) as f64 / 2.0);
    let (origin_x, origin_y) = (center_x - content / 2.0, center_y - content / 2.0);
    let scale = content / side as f64;
    let radius = side as f64 * 0.2;
    let outline = side as f64 * 0.012;
    let half = side as f64 / 2.0;
    let mut rgba = vec![0u8; (side * side * 4) as usize];
    let steps = scale.ceil().max(1.0) as u32;
    for y in 0..side {
        for x in 0..side {
            let mut sum = 0f64;
            for dy in 0..steps {
                for dx in 0..steps {
                    let (sx, sy) = (
                        origin_x + (x as f64 + (dx as f64 + 0.5) / steps as f64) * scale,
                        origin_y + (y as f64 + (dy as f64 + 0.5) / steps as f64) * scale,
                    );
                    let inside = sx >= outer_left as f64
                        && sy >= outer_top as f64
                        && sx < outer_right as f64
                        && sy < outer_bottom as f64;
                    sum += if inside {
                        raster.rgba[((sy as u32 * raster.width + sx as u32) * 4) as usize] as f64
                    } else {
                        255.0
                    };
                }
            }
            let mut value = sum / (steps * steps) as f64;
            // Signed distance to the rounded square: negative inside.
            let (qx, qy) = (
                (x as f64 + 0.5 - half).abs() - (half - radius),
                (y as f64 + 0.5 - half).abs() - (half - radius),
            );
            let distance =
                (qx.max(0.0).powi(2) + qy.max(0.0).powi(2)).sqrt() + qx.max(qy).min(0.0) - radius;
            let rim = (distance + outline + 0.5).clamp(0.0, 1.0);
            value *= 1.0 - rim;
            let at = ((y * side + x) * 4) as usize;
            for channel in 0..3 {
                rgba[at + channel] = value.round() as u8;
            }
            rgba[at + 3] = ((0.5 - distance).clamp(0.0, 1.0) * 255.0).round() as u8;
        }
    }
    Raster {
        width: side,
        height: side,
        rgba,
    }
}

fn compare(arguments: &[String]) -> i32 {
    let (Some(a), Some(b)) = (arguments.first(), arguments.get(1)) else {
        return fail("usage: comic compare <a.png> <b.png> [--tolerance 3]");
    };
    let tolerance = option(arguments, "--tolerance")
        .and_then(|said| said.parse().ok())
        .unwrap_or(3);
    let read = |path: &str| {
        std::fs::read(path)
            .map_err(|error| format!("{path}: {error}"))
            .and_then(|bytes| comic_render::png::decode(&bytes))
    };
    let (a, b) = match (read(a), read(b)) {
        (Ok(a), Ok(b)) => (a, b),
        (Err(error), _) | (_, Err(error)) => return fail(error),
    };
    match comic_render::compare::compare(&a, &b, tolerance) {
        Ok(diff) => {
            println!(
                "{}",
                json!({"max": diff.max, "over": diff.over, "total": diff.total, "tolerance": tolerance})
            );
            if diff.over == 0 { 0 } else { 1 }
        }
        Err(error) => fail(error),
    }
}
