//! The package: what `comic` is called, what it takes, what it makes, and its run.

use std::collections::BTreeMap;

use granite_agents::{AgentContext, AgentMake, AgentPackage, AgentRefusing, BoxError};

use crate::{ComicAnswer, ComicInput, run};

#[derive(Default)]
pub struct ComicAgent;

/// What a run streams: the strip, once written.
#[derive(Clone, Debug)]
pub enum ComicFrame {
    Written(Box<ComicAnswer>),
}

impl AgentPackage for ComicAgent {
    type Input = ComicInput;
    type Event = ComicFrame;
    type Output = ComicAnswer;
    type Compute = ();

    const WORD: &'static str = "comic";
    const SUMMARY: &'static str = "draws a stick-figure comic strip that explains a concept, as a PNG: the host's model writes the script (panels, figures, what they say, the captions), the agent inks and letters it by hand in the style of webcomics, and the picture carries its script so it can be drawn again.";

    fn fields() -> BTreeMap<String, String> {
        [
            ("brief", "what to explain and the angle to take: the concept, who is confused by it, where the joke is; the ask when left out"),
            ("panels", "how many panels, 1 to 6; left out, the model picks three to five, or one for a gag"),
            ("script", "a strip's PNG from an earlier call (it carries its script), or a .comic.json, to draw again; with a brief, changed as the brief asks"),
            ("output", "where to write the PNG; comics/<title>.png when left out"),
            ("rounds", "how many times the model may fix what the strip measured; 3 when left out"),
            ("seed", "a number that fixes the hand's wobble; the same script and seed draw the same picture"),
            ("renderer", "gpu, cpu, or auto (left out): the GPU when there is one, the same picture on the CPU when there is none"),
        ]
        .into_iter()
        .map(|(field, note)| (field.to_owned(), note.to_owned()))
        .collect()
    }

    fn guide() -> String {
        "Ask comic for one strip of one idea: a concept explained in a few panels by stick figures, the way a webcomic does, \
         with a turn at the end. Say in `brief` what to explain and where the confusion is; `panels` only when the person \
         asked for a count. The host's model writes the script (figures from a small catalogue of poses, what each says, a \
         caption a panel, a title and an alt line) and the agent draws it: it letters every word by hand, measures what \
         fits, and asks the model to fix what ran over, up to `rounds`; a call takes a minute or two. It answers with the \
         PNG at 2x, which carries the script it was drawn from: to change a strip, call again with `script` set to its PNG \
         and the change in `brief`; to draw one again, `script` alone, no model needed. Show the answer's `alt` under the \
         picture as the strip's aside. Not for charts, diagrams, portraits or anything photographic; a longer story is \
         several calls."
            .into()
    }

    /// One GPU device draws every strip: calls take turns.
    fn lanes() -> Option<u32> {
        Some(1)
    }

    fn makes() -> Vec<AgentMake> {
        vec![AgentMake::new(
            "comic",
            "a stick-figure comic strip that explains a concept, as a PNG with its script inside",
            &["comics/*"],
            Some("a picture of a real person, a chart of numbers, or the script inside the answer"),
        )]
    }

    fn module() -> Option<String> {
        Some("ComicAgent".into())
    }

    async fn compute() -> Result<(), BoxError> {
        Ok(())
    }

    fn output(events: Vec<ComicFrame>) -> Option<ComicAnswer> {
        events
            .into_iter()
            .rev()
            .map(|ComicFrame::Written(answer)| *answer)
            .next()
    }

    async fn run(
        &self,
        input: ComicInput,
        context: AgentContext<(), ComicFrame>,
    ) -> Result<(), BoxError> {
        match run::run(input, &context.host).await {
            Ok(answer) => {
                context.deliver(ComicFrame::Written(Box::new(answer)));
                Ok(())
            }
            Err(error) => Err(error.refused()),
        }
    }
}
