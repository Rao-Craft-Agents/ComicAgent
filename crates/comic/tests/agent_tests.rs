//! The comic agent as a host calls it: what it says of itself, a strip drawn again with no model, a strip drafted
//! and revised by a model, every refusal, and a call over the real wire.

use std::sync::{Arc, Mutex};

use comic_agent::{ComicAgent, SAMPLE, draft};
use comic_core::{Script, spec_png};
use granite_agents::{
    AgentHost, AgentHostError, AgentHostModel, AgentProcess, AgentRefusal, AnyAgentPackage,
    BoxError, BoxFuture, Entry, GraniteSchema, RefusalKind, RosterEntry,
};
use serde_json::{Value, json};

struct FakeModel {
    answers: Mutex<Vec<Value>>,
    asked: Mutex<Vec<(String, String)>>,
}

impl FakeModel {
    fn new(answers: Vec<Value>) -> Arc<FakeModel> {
        Arc::new(FakeModel {
            answers: Mutex::new(answers),
            asked: Mutex::new(Vec::new()),
        })
    }
}

impl AgentHostModel for FakeModel {
    fn object(
        &self,
        schema: Vec<u8>,
        system: String,
        user: String,
        _: u32,
        _: f64,
    ) -> BoxFuture<'_, Result<Vec<u8>, AgentHostError>> {
        assert!(
            serde_json::from_slice::<Value>(&schema).is_ok(),
            "the ask carries a JSON Schema"
        );
        self.asked.lock().unwrap().push((system, user));
        let next = {
            let mut answers = self.answers.lock().unwrap();
            if answers.is_empty() {
                None
            } else {
                Some(answers.remove(0))
            }
        };
        Box::pin(async move {
            next.map(|value| serde_json::to_vec(&value).unwrap())
                .ok_or_else(|| AgentHostError::NoAnswer("nothing left".into()))
        })
    }

    fn text(
        &self,
        _: String,
        _: String,
        _: u32,
        _: f64,
    ) -> BoxFuture<'_, Result<String, AgentHostError>> {
        Box::pin(async { Ok(String::new()) })
    }
}

fn sample() -> Value {
    serde_json::from_str(SAMPLE).unwrap()
}

/// The sample with its first bubble far past what a bubble holds.
fn overflowing() -> Value {
    let mut script = sample();
    script["panels"][0]["bubbles"][0]["text"] = json!(
        "To store a key, I first turn it into a number by running it through a hash function that mixes every letter into one big value, then I keep only the last digit of it."
    );
    script
}

async fn call(input: Value, host: AgentHost) -> Result<Value, BoxError> {
    let bytes = AnyAgentPackage::of::<ComicAgent>()
        .perform(input.to_string().as_bytes(), host)
        .await?;
    Ok(serde_json::from_slice(&bytes)?)
}

fn in_folder(folder: &tempfile::TempDir) -> AgentHost {
    AgentHost::none().with_directory(folder.path().to_string_lossy())
}

fn refusal(error: &BoxError) -> AgentRefusal {
    AgentRefusal::of(error.as_ref()).unwrap_or_else(|| panic!("not a refusal: {error}"))
}

#[test]
fn the_entry_says_what_comic_takes_and_makes() {
    let entry = Entry::of(&RosterEntry::Package(AnyAgentPackage::of::<ComicAgent>()));
    assert_eq!(
        (
            entry.word.as_str(),
            entry.module.as_str(),
            entry.r#type.as_str()
        ),
        ("comic", "ComicAgent", "ComicAgent.ComicAgent")
    );
    assert_eq!(entry.lanes, Some(1));
    let makes = entry.makes.unwrap();
    assert_eq!(
        (makes[0].kind.as_str(), makes[0].into.clone()),
        ("comic", vec!["comics/*".to_owned()])
    );
    let fields: Vec<&str> = entry
        .schema
        .as_ref()
        .unwrap()
        .fields()
        .iter()
        .map(|field| field.name.as_str())
        .collect();
    assert_eq!(
        fields,
        [
            "brief",
            "panels",
            "script",
            "output",
            "rounds",
            "seed",
            "renderer",
            "directory"
        ]
    );
    let renderer = &entry.schema.as_ref().unwrap().fields()[6];
    assert_eq!(
        renderer.schema,
        GraniteSchema::Choice(vec!["gpu".into(), "cpu".into(), "auto".into()])
    );
    assert!(
        entry
            .schema
            .unwrap()
            .fields()
            .iter()
            .all(|field| !field.required),
        "every field may be left out"
    );
    for said in [entry.summary, entry.guide.unwrap()] {
        assert!(
            !said.to_lowercase().contains("xkcd"),
            "the agent never names another's comic: {said}"
        );
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn a_script_is_drawn_again_with_no_model_and_the_same_picture_twice() {
    let folder = tempfile::tempdir().unwrap();
    std::fs::write(folder.path().join("hashmap.comic.json"), SAMPLE).unwrap();
    let input = json!({"script": "hashmap.comic.json", "seed": 7, "renderer": "cpu"});
    let answer = call(input.clone(), in_folder(&folder)).await.unwrap();
    assert_eq!(answer["format"], "granite-answer/1");
    assert_eq!(answer["agent"]["word"], "comic");
    let path = answer["path"].as_str().unwrap().to_owned();
    assert!(
        path.ends_with("comics/how-a-hash-map-finds-a-key.png"),
        "{path}"
    );
    assert_eq!(
        (
            answer["width"].clone(),
            answer["panels"].clone(),
            answer["rounds"].clone()
        ),
        (json!(3200), json!(3), json!(1))
    );
    assert_eq!(answer["renderer"], "cpu");
    assert_eq!(answer["findings"], json!([]));
    assert!(answer.get("note").is_none());
    assert_eq!(
        answer["made"][0],
        json!({"kind": "comic", "name": "how-a-hash-map-finds-a-key", "path": path, "state": "written"})
    );
    assert_eq!(
        answer["expected"],
        json!([{"kind": "comic", "name": "how-a-hash-map-finds-a-key"}])
    );
    let first = std::fs::read(&path).unwrap();
    let carried = Script::from_json(spec_png::embedded(&first).unwrap().as_bytes()).unwrap();
    assert_eq!(
        carried,
        Script::from_json(SAMPLE.as_bytes()).unwrap().clamped().0,
        "the PNG carries the script it was drawn from"
    );

    let again = call(input, in_folder(&folder)).await.unwrap();
    assert_eq!(again["made"][0]["state"], "rewritten");
    assert_eq!(
        std::fs::read(&path).unwrap(),
        first,
        "the same script and seed draw the same picture"
    );

    let from_png = call(
        json!({"script": path, "seed": 7, "renderer": "cpu", "output": "again.png"}),
        in_folder(&folder),
    )
    .await
    .unwrap();
    assert_eq!(
        std::fs::read(from_png["path"].as_str().unwrap()).unwrap(),
        first,
        "a strip is drawn again from its PNG alone"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_strip_is_drafted_measured_and_revised_by_the_hosts_model() {
    let folder = tempfile::tempdir().unwrap();
    let model = FakeModel::new(vec![overflowing(), sample()]);
    let notes = Arc::new(Mutex::new(Vec::new()));
    let heard = notes.clone();
    let host = in_folder(&folder)
        .with_ask("explain how a hash map finds a key")
        .with_model(model.clone())
        .with_note(move |line| heard.lock().unwrap().push(line.to_owned()));
    let answer = call(json!({"renderer": "cpu"}), host).await.unwrap();
    assert_eq!(answer["rounds"], 2);
    assert_eq!(answer["findings"], json!([]));
    let asked = model.asked.lock().unwrap().clone();
    assert_eq!(asked.len(), 2);
    assert!(
        asked[0].1.contains("explain how a hash map finds a key"),
        "{}",
        asked[0].1
    );
    assert!(
        asked[0].0.contains("Why bread rises"),
        "the draft is shown the example on another subject"
    );
    assert!(
        asked[1].1.contains("What the strip measured"),
        "{}",
        asked[1].1
    );
    let notes = notes.lock().unwrap().clone();
    assert!(
        notes
            .iter()
            .any(|note| note.starts_with("round 1: ") && note.contains("to fix")),
        "{notes:?}"
    );
    assert!(
        notes.iter().any(|note| note == "round 2: measured clean"),
        "{notes:?}"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn an_unchanged_revision_ends_the_rounds_and_the_answer_says_the_call_is_done() {
    let folder = tempfile::tempdir().unwrap();
    let model = FakeModel::new(vec![overflowing(), overflowing()]);
    let host = in_folder(&folder)
        .with_ask("explain hashing")
        .with_model(model.clone());
    let answer = call(json!({"renderer": "cpu"}), host).await.unwrap();
    assert_eq!(answer["rounds"], 1);
    assert!(!answer["findings"].as_array().unwrap().is_empty());
    assert!(
        answer["note"]
            .as_str()
            .unwrap()
            .starts_with("The PNG is written and this call is done")
    );
    let kinds: Vec<&str> = answer["problems"]
        .as_array()
        .unwrap()
        .iter()
        .map(|problem| problem["kind"].as_str().unwrap())
        .collect();
    assert!(
        kinds.contains(&"short"),
        "the bubble was cut to fit: {kinds:?}"
    );
    assert!(std::path::Path::new(answer["path"].as_str().unwrap()).exists());
}

#[tokio::test(flavor = "multi_thread")]
async fn a_strip_is_changed_as_a_brief_asks() {
    let folder = tempfile::tempdir().unwrap();
    std::fs::write(folder.path().join("hashmap.comic.json"), SAMPLE).unwrap();
    let mut one_panel = sample();
    one_panel["panels"] = json!([one_panel["panels"][0].clone()]);
    let model = FakeModel::new(vec![one_panel]);
    let host = in_folder(&folder).with_model(model.clone());
    let answer = call(json!({"script": "hashmap.comic.json", "brief": "keep only the first panel", "renderer": "cpu"}), host).await.unwrap();
    assert_eq!(answer["panels"], 1);
    assert_eq!(answer["width"], 1600, "one panel is half a strip's width");
    assert!(
        model.asked.lock().unwrap()[0]
            .1
            .contains("What to change: keep only the first panel")
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn each_refusal_says_whose_it_is() {
    let folder = tempfile::tempdir().unwrap();
    std::fs::write(folder.path().join("broken.comic.json"), "not a script").unwrap();

    let no_model = call(json!({"brief": "recursion"}), in_folder(&folder))
        .await
        .unwrap_err();
    assert_eq!(refusal(&no_model).kind, RefusalKind::Call);
    assert!(
        no_model
            .to_string()
            .starts_with("comic drafts with the host's model and this host has none"),
        "{no_model}"
    );

    let broken = call(json!({"script": "broken.comic.json"}), in_folder(&folder))
        .await
        .unwrap_err();
    assert_eq!(refusal(&broken).kind, RefusalKind::Call);
    assert!(
        broken
            .to_string()
            .starts_with("the script broken.comic.json could not be read: the script is not JSON"),
        "{broken}"
    );

    let missing = call(json!({"script": "gone.png"}), in_folder(&folder))
        .await
        .unwrap_err();
    assert!(
        missing
            .to_string()
            .contains("could not be read: it is not there"),
        "{missing}"
    );

    let panels = call(json!({"panels": 9}), in_folder(&folder))
        .await
        .unwrap_err();
    assert_eq!(
        refusal(&panels),
        AgentRefusal::one(RefusalKind::Input, "`panels` is 9: a strip holds 1 to 6")
    );

    let model = FakeModel::new(vec![json!({"nope": 1}), json!(7)]);
    let drafts = call(
        json!({"brief": "recursion", "renderer": "cpu"}),
        in_folder(&folder).with_model(model),
    )
    .await
    .unwrap_err();
    assert_eq!(refusal(&drafts).kind, RefusalKind::Model);
    assert!(
        drafts
            .to_string()
            .starts_with("the model's script was not a strip"),
        "{drafts}"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn the_gpu_draws_when_asked_for_and_refuses_as_a_device_when_there_is_none() {
    let folder = tempfile::tempdir().unwrap();
    std::fs::write(folder.path().join("hashmap.comic.json"), SAMPLE).unwrap();
    let asked = call(
        json!({"script": "hashmap.comic.json", "renderer": "gpu"}),
        in_folder(&folder),
    )
    .await;
    match comic_render::choose(comic_render::Choice::Gpu) {
        Ok(_) => assert_eq!(asked.unwrap()["renderer"], "gpu"),
        Err(_) => assert_eq!(refusal(&asked.unwrap_err()).kind, RefusalKind::Device),
    }
}

#[test]
fn the_model_is_asked_for_meaning_only_and_never_the_examples_words() {
    let schema = draft::ask_schema();
    let text = schema.to_string();
    assert!(
        !text.contains("$ref") && !text.contains("$defs") && !text.contains("maxLength"),
        "{text}"
    );
    assert!(
        schema["properties"].get("schema").is_none(),
        "the agent stamps the standard's id"
    );
    assert_eq!(schema["required"], json!(["panels"]));
    assert!(text.contains("holding-sign") && text.contains("signpost"));
    let mut copied = sample();
    copied["title"] = json!("Why bread rises");
    copied["panels"][0]["bubbles"][0]["text"] = json!("The yeast eats the sugar in the flour.");
    let echoes = draft::echoes(&Script::from_value(&copied).unwrap());
    let said: Vec<&str> = echoes
        .iter()
        .map(|finding| finding.sentence.as_str())
        .collect();
    assert!(
        said.contains(&"the title is the example's: write this strip's own"),
        "{said:?}"
    );
    assert!(
        said.iter()
            .any(|sentence| sentence.starts_with("panel 1 says the example's words")),
        "{said:?}"
    );
    assert!(draft::echoes(&Script::from_value(&sample()).unwrap()).is_empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn the_runner_answers_over_the_wire_as_any_host_calls_it() {
    let runner = AgentProcess::new(env!("CARGO_BIN_EXE_comic"));
    let entries = runner.describe().await.unwrap();
    assert_eq!(entries[0].word, "comic");
    let folder = tempfile::tempdir().unwrap();
    std::fs::write(folder.path().join("hashmap.comic.json"), SAMPLE).unwrap();
    let cost = Arc::new(Mutex::new(None));
    let kept = cost.clone();
    let bytes = runner
        .perform(
            "comic",
            br#"{"script":"hashmap.comic.json","renderer":"cpu","seed":7}"#,
            &in_folder(&folder),
            Some(Box::new(move |cost| *kept.lock().unwrap() = Some(cost))),
        )
        .await
        .unwrap();
    let answer: Value = serde_json::from_slice(&bytes).unwrap();
    assert!(std::path::Path::new(answer["path"].as_str().unwrap()).exists());
    assert_eq!(cost.lock().unwrap().as_ref().unwrap().status, "exit 0");
}
