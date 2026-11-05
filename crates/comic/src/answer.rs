use granite_agents::{AgentAnswer, AgentExpected, AgentMade, AgentPayload, AgentProblem};
use serde::Serialize;

/// What a call answers: `granite-answer/1`, and the strip's own keys beside it.
#[derive(Clone, Debug, Serialize)]
pub struct ComicAnswer {
    /// The PNG, absolute.
    pub path: String,
    pub paths: Vec<String>,
    /// In pixels: twice the strip's size.
    pub width: u32,
    pub height: u32,
    pub panels: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// The aside to show under the picture.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alt: Option<String>,
    /// How many times the strip was measured.
    pub rounds: u32,
    /// What the strip written still measures wrong, one sentence each; empty when it measured clean.
    pub findings: Vec<String>,
    /// `gpu` or `cpu`.
    pub renderer: String,
    pub made: Vec<AgentMade>,
    pub expected: Vec<AgentExpected>,
    pub problems: Vec<AgentProblem>,
    pub payloads: Vec<AgentPayload>,
    /// What the calling model is told when findings are left: the call is done.
    ///
    /// PIN: FINDINGS LEFT ARE THE STRIP'S, NOT THE CALLER'S. A calling model that reads findings calls again with
    /// a reworded brief and draws the same strip until its turn runs out (Plate's lesson), so it is told the call
    /// is done.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

impl AgentAnswer for ComicAnswer {}

impl ComicAnswer {
    pub fn done_note(rounds: u32) -> String {
        let rounds = if rounds == 1 {
            "1 round".to_owned()
        } else {
            format!("{rounds} rounds")
        };
        format!(
            "The PNG is written and this call is done. The findings are what the strip's own drawing kept after {rounds} of fixing: calling again with the same brief draws the same strip, so tell the person what is left instead of calling again."
        )
    }
}
