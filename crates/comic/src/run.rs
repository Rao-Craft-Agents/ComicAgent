//! One call: a script read or drafted, measured and revised, drawn once, written with its script inside.
//!
//! ```text
//! script given ─┬─ (brief + model) changed ─┐
//!               └─────────────────────────────┴─▶ measure ─▶ draw ─▶ write
//! no script ─── drafted by the model ─▶ measure ─▶ revise ─▶ measure … (up to `rounds`) ─▶ draw ─▶ write
//! ```
//!
//! PIN: AN UNCHANGED REVISION ENDS THE ROUNDS (Plate's lesson: a reviser that answers the same script again will
//! answer it again). The best round measured is the one drawn.
//! PIN: DRAWN ONCE. Findings come from the layout, not the pixels, so only the strip written is rendered.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use comic_core::{Composed, Finding, Problem, Script, Style, compose, rng, spec_png};
use comic_render::{Choice, RenderOptions};
use granite_agents::{
    AgentExpected, AgentHost, AgentHostModel, AgentMade, AgentProblem, MadeState, host_ext,
};

use crate::{ComicAnswer, ComicError, ComicInput, draft};

struct Round {
    drawn: Script,
    composed: Composed,
    cuts: Vec<Problem>,
    found: Vec<Finding>,
}

fn nonempty(text: &Option<String>) -> Option<&str> {
    text.as_deref()
        .map(str::trim)
        .filter(|text| !text.is_empty())
}

/// `path` under `folder` when relative, `~` expanded.
fn resolve(path: &str, folder: &Path) -> PathBuf {
    let expanded = match path.strip_prefix("~/") {
        Some(rest) => PathBuf::from(std::env::var("HOME").unwrap_or_default()).join(rest),
        None => PathBuf::from(path),
    };
    if expanded.is_absolute() {
        expanded
    } else {
        folder.join(expanded)
    }
}

fn read_script(file: &Path) -> Result<Script, String> {
    let bytes = std::fs::read(file).map_err(|error| match error.kind() {
        std::io::ErrorKind::NotFound => "it is not there".to_owned(),
        _ => error.to_string(),
    })?;
    if spec_png::is_png(&bytes) {
        let carried = spec_png::embedded(&bytes).ok_or("the PNG carries no comic script")?;
        return Script::from_json(carried.as_bytes()).map_err(|error| error.to_string());
    }
    Script::from_json(&bytes).map_err(|error| error.to_string())
}

/// A script the model writes, read as a strip; why not, when it is not one.
async fn ask(model: &Arc<dyn AgentHostModel>, system: &str, user: &str) -> Result<Script, String> {
    let value = host_ext::object_value(
        model.as_ref(),
        &draft::ask_schema(),
        system,
        user,
        3000,
        300.0,
    )
    .await
    .map_err(|error| error.to_string())?;
    Script::from_value(&value).map_err(|error| error.to_string())
}

fn shown(path: &Path, folder: &Path) -> String {
    path.strip_prefix(folder)
        .unwrap_or(path)
        .display()
        .to_string()
}

pub async fn run(input: ComicInput, host: &AgentHost) -> Result<ComicAnswer, ComicError> {
    if let Some(panels) = input.panels
        && !(1..=6).contains(&panels)
    {
        return Err(ComicError::Panels(panels));
    }
    if let Some(rounds) = input.rounds
        && rounds < 1
    {
        return Err(ComicError::Rounds(rounds));
    }
    let rounds = input.rounds.unwrap_or(3).min(6);
    let folder = input
        .directory
        .clone()
        .or_else(|| host.directory.clone())
        .map(PathBuf::from)
        .or_else(|| std::env::current_dir().ok())
        .unwrap_or_else(|| PathBuf::from("."));
    let model = host.model.clone();
    let brief = nonempty(&input.brief);

    // The renderer first: a call that asks for a GPU this host lacks is refused before the model is asked anything.
    let choice = Choice::from_env_or(input.renderer.map(|renderer| renderer.word()));
    let chosen = tokio::task::spawn_blocking(move || comic_render::choose(choice))
        .await
        .map_err(|error| ComicError::Device(format!("the renderer could not start: {error}")))?
        .map_err(|error| ComicError::Device(error.to_string()))?;

    let mut problems: Vec<Problem> = Vec::new();
    let (mut script, revisable) = match &input.script {
        Some(path) => {
            let file = resolve(path, &folder);
            let script = read_script(&file).map_err(|reason| ComicError::Script {
                path: path.clone(),
                reason,
            })?;
            host.note(&format!("drawing again from {}", shown(&file, &folder)));
            match (brief, &model) {
                (Some(brief), Some(model)) => {
                    host.note(&format!(
                        "changing the strip: {}",
                        brief.chars().take(80).collect::<String>()
                    ));
                    match ask(
                        model,
                        &draft::revise_system(),
                        &draft::change(&script, brief),
                    )
                    .await
                    {
                        Ok(changed) => (changed, true),
                        Err(reason) => {
                            problems.push(Problem::model(format!("the model did not change the script as the brief asked ({reason}); drawn as given")));
                            (script, false)
                        }
                    }
                }
                _ => (script, false),
            }
        }
        None => {
            let Some(model) = &model else {
                return Err(ComicError::NoModel);
            };
            if brief.is_none() && nonempty(&host.ask).is_none() {
                return Err(ComicError::NoBrief);
            }
            host.note("drafting the strip: panels, figures, what they say");
            let user = draft::draft(host.ask.as_deref(), brief, input.panels);
            let script = match ask(model, &draft::system(), &user).await {
                Ok(script) => script,
                Err(_) => ask(model, &draft::system(), &user)
                    .await
                    .map_err(ComicError::Draft)?,
            };
            (script, true)
        }
    };

    let seed = input
        .seed
        .unwrap_or_else(|| rng::hash(script.title.as_deref().unwrap_or("comic")));
    let style = Style::default();
    let mut best: Option<Round> = None;
    let mut measured = 0;
    for round in 1..=rounds {
        measured = round;
        let (drawn, cuts) = script.clamped();
        let composed = compose(&drawn, &style, seed);
        let mut found = script.check();
        if revisable {
            found.extend(draft::echoes(&script));
        }
        found.extend(composed.findings.iter().cloned());
        match found.first() {
            None => host.note(&format!("round {round}: measured clean")),
            Some(first) => host.note(&format!(
                "round {round}: {} to fix — {}",
                found.len(),
                first.sentence
            )),
        }
        let clean = found.is_empty();
        let to_fix = found.clone();
        if best
            .as_ref()
            .is_none_or(|kept| found.len() <= kept.found.len())
        {
            best = Some(Round {
                drawn,
                composed,
                cuts,
                found,
            });
        }
        if clean || round == rounds || !revisable {
            break;
        }
        let Some(model) = &model else { break };
        host.note("revising the script");
        match ask(
            model,
            &draft::revise_system(),
            &draft::revise(&script, &to_fix),
        )
        .await
        {
            Ok(revised) if revised != script => script = revised,
            Ok(_) => break,
            Err(reason) => {
                problems.push(Problem::model(format!("the model's revision could not be read ({reason}); the strip drawn is the last script that could")));
                break;
            }
        }
    }
    let Round {
        drawn,
        composed,
        cuts,
        found,
    } = best.expect("one round ran");

    let (mut renderer, fallback) = chosen;
    problems.extend(fallback);
    problems.extend(cuts);
    host.note(&format!(
        "drawing {} panels on the {}",
        drawn.panels.len(),
        renderer.name()
    ));
    let json = drawn.to_json();
    let (width, height) = (composed.width, composed.height);
    let scene = composed.scene;
    let (png, renderer_name) = tokio::task::spawn_blocking(move || {
        let raster = renderer.render(&scene, &RenderOptions::strip())?;
        Ok::<_, comic_render::RenderError>((
            comic_render::png::encode(&raster, Some(&json)),
            renderer.name(),
        ))
    })
    .await
    .map_err(|error| ComicError::Device(format!("the renderer stopped: {error}")))?
    .map_err(|error| ComicError::Device(error.to_string()))?;

    let target = match nonempty(&input.output) {
        Some(output) => {
            let path = resolve(output, &folder);
            if path.extension().is_none() {
                path.with_extension("png")
            } else {
                path
            }
        }
        None => folder.join("comics").join(format!("{}.png", drawn.slug())),
    };
    let existed = target.exists();
    let unwritable = |error: std::io::Error| ComicError::Unwritable {
        path: target.display().to_string(),
        reason: error.to_string(),
    };
    if let Some(parent) = target.parent() {
        std::fs::create_dir_all(parent).map_err(unwritable)?;
    }
    std::fs::write(&target, &png).map_err(unwritable)?;
    host.note(&format!("wrote {}", shown(&target, &folder)));

    let path = target.to_string_lossy().into_owned();
    let name = drawn.slug();
    let findings: Vec<String> = found
        .iter()
        .map(|finding| finding.sentence.clone())
        .collect();
    Ok(ComicAnswer {
        path: path.clone(),
        paths: vec![path.clone()],
        width,
        height,
        panels: drawn.panels.len(),
        title: drawn.title.clone(),
        alt: drawn.alt.clone(),
        rounds: measured,
        renderer: renderer_name.to_owned(),
        made: vec![AgentMade {
            kind: "comic".into(),
            name: name.clone(),
            path: Some(path),
            state: if existed {
                MadeState::Rewritten
            } else {
                MadeState::Written
            },
        }],
        expected: vec![AgentExpected {
            kind: "comic".into(),
            name,
        }],
        problems: problems
            .into_iter()
            .map(|problem| AgentProblem {
                kind: problem.kind,
                text: problem.text,
            })
            .collect(),
        payloads: Vec::new(),
        note: (!findings.is_empty()).then(|| ComicAnswer::done_note(measured)),
        findings,
    })
}
