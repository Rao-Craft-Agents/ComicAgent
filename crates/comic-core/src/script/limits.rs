//! How much a strip holds. The prose a model is given and `Script::check` use the same numbers; the lettering is sized
//! so text within them fits (a bubble's 110 characters in four lines of a three-panel strip).

/// The standard's limits.
pub struct Limits;

impl Limits {
    pub const PANELS: usize = 6;
    pub const FIGURES: usize = 3;
    pub const BUBBLES: usize = 3;
    pub const PROPS: usize = 3;
    pub const TITLE: usize = 48;
    pub const ALT: usize = 240;
    pub const CAPTION: usize = 140;
    pub const BUBBLE: usize = 110;
    pub const LABEL: usize = 20;
    pub const SIGN: usize = 24;
    pub const PROP_TEXT: usize = 48;
    /// Lines a bubble's lettering may run to.
    pub const BUBBLE_LINES: usize = 4;
    /// Lines a caption's lettering may run to.
    pub const CAPTION_LINES: usize = 2;

    /// About how long to make text that ran over `limit`: four-fifths of it, to a multiple of five.
    pub fn about(limit: usize) -> usize {
        (limit * 82 / 100) / 5 * 5
    }
}
