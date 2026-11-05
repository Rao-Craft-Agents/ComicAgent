//! `comic`: a stick-figure comic strip that explains a concept, as a Granite agent package.
//!
//! The host's model writes the strip's script (`craft-comic/1`: panels, figures from a catalogue of poses, what
//! they say, a caption a panel); the agent lays it out, letters it by hand, measures what fits, asks the model to
//! fix what ran over, and draws it in ink on the GPU (or the same picture on the CPU). The PNG carries its
//! script, so a strip is drawn again, or changed, from the picture alone.
//!
//! PIN: THE MODEL SAYS, THE KIT PLACES. The script is meaning only (who stands where, what they say); the grid,
//! the lettering, the strokes and their wobble are the agent's. A local model writes a good script; it cannot
//! place forty words without collisions, and it is never asked to.

pub mod agent;
pub mod answer;
pub mod cli;
pub mod draft;
pub mod error;
pub mod input;
pub mod run;

pub use agent::{ComicAgent, ComicFrame};
pub use answer::ComicAnswer;
pub use error::ComicError;
pub use input::{ComicInput, RendererChoice};

/// The fixture the check and the tests draw: "How a hash map finds a key", three panels.
pub const SAMPLE: &str = include_str!("../../../Schema/examples/hashmap.comic.json");
