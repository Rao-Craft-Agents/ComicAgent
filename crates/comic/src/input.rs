use serde::Deserialize;

/// What a call takes. Every field may be left out; a call with no `script` drafts one with the host's model.
#[derive(Clone, Debug, Default, Deserialize)]
pub struct ComicInput {
    /// What to explain and the angle to take; the person's ask when left out.
    pub brief: Option<String>,
    /// How many panels, 1 to 6.
    pub panels: Option<u32>,
    /// A `.comic.json`, or a strip's PNG (it carries its script), to draw again.
    pub script: Option<String>,
    /// Where to write the PNG; `comics/<title>.png` when left out.
    pub output: Option<String>,
    /// How many times the model may fix what the strip measured; 3 when left out.
    pub rounds: Option<u32>,
    /// What fixes the hand's wobble.
    pub seed: Option<u64>,
    /// Which renderer draws it.
    pub renderer: Option<RendererChoice>,
    /// The host's folder.
    pub directory: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RendererChoice {
    Gpu,
    Cpu,
    Auto,
}

impl RendererChoice {
    pub fn word(self) -> &'static str {
        match self {
            RendererChoice::Gpu => "gpu",
            RendererChoice::Cpu => "cpu",
            RendererChoice::Auto => "auto",
        }
    }
}
