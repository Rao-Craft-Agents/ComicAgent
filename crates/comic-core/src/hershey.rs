//! The Hershey Simplex Roman font: single-stroke glyphs made for pen plotters, so each letter is already a list of pen
//! strokes. The data (`fonts/futural.jhf`) keeps the format James Hurt gave the Hershey fonts; its acknowledgement and
//! use restriction are in `fonts/HERSHEY-LICENSE.txt`.
//!
//! A record is a glyph number (columns 0–4), its count of coordinate pairs (5–7), then the pairs, each letter's code
//! minus `R`'s: first the left and right bearings, then the strokes, a pen-up written ` R`. A long record may run on to
//! the next line; the count says how many pairs to read.

use std::sync::LazyLock;

/// One glyph: its bearings and its strokes, in font units (y down; capitals span −12 to 9, the baseline).
#[derive(Clone, Debug, PartialEq)]
pub struct Glyph {
    pub left: i8,
    pub right: i8,
    pub strokes: Vec<Vec<(i8, i8)>>,
}

impl Glyph {
    /// The advance, in font units.
    pub fn advance(&self) -> f32 {
        f32::from(self.right) - f32::from(self.left)
    }
}

/// A font: one glyph for each ASCII character from the space on.
#[derive(Clone, Debug, PartialEq)]
pub struct HersheyFont {
    glyphs: Vec<Glyph>,
}

/// The top of a capital, in font units.
pub const CAP_TOP: f32 = -12.0;
/// The baseline, in font units.
pub const BASELINE: f32 = 9.0;
/// A capital's height, in font units.
pub const CAP_HEIGHT: f32 = BASELINE - CAP_TOP;

static SIMPLEX: LazyLock<HersheyFont> = LazyLock::new(|| {
    HersheyFont::parse(include_str!("../fonts/futural.jhf")).expect("futural.jhf is a Hershey font")
});

impl HersheyFont {
    /// Hershey Simplex Roman, parsed on first use.
    pub fn simplex() -> &'static HersheyFont {
        &SIMPLEX
    }

    /// Reads a `.jhf` file: glyph `i` is ASCII 32 + `i`.
    pub fn parse(text: &str) -> Result<HersheyFont, String> {
        let mut lines = text.lines().filter(|line| !line.trim().is_empty());
        let mut glyphs = Vec::new();
        while let Some(line) = lines.next() {
            if line.len() < 8 || !line.is_ascii() {
                return Err(format!(
                    "record {} is not a Hershey record",
                    glyphs.len() + 1
                ));
            }
            let count: usize = line[5..8]
                .trim()
                .parse()
                .map_err(|_| format!("record {} has no pair count", glyphs.len() + 1))?;
            let mut data = line[8..].to_string();
            while data.len() < 2 * count {
                match lines.next() {
                    Some(more) => data.push_str(more),
                    None => return Err(format!("record {} ends early", glyphs.len() + 1)),
                }
            }
            let bytes = data.as_bytes();
            let at = |i: usize| bytes[i] as i16 - b'R' as i16;
            let (left, right) = (at(0) as i8, at(1) as i8);
            let mut strokes = vec![Vec::new()];
            for k in 1..count {
                if &bytes[2 * k..2 * k + 2] == b" R" {
                    strokes.push(Vec::new());
                } else if let Some(stroke) = strokes.last_mut() {
                    stroke.push((at(2 * k) as i8, at(2 * k + 1) as i8));
                }
            }
            strokes.retain(|s| !s.is_empty());
            glyphs.push(Glyph {
                left,
                right,
                strokes,
            });
        }
        if glyphs.len() < 95 {
            return Err(format!(
                "the font has {} glyphs; ASCII needs 95",
                glyphs.len()
            ));
        }
        Ok(HersheyFont { glyphs })
    }

    /// The glyph for a printable ASCII character.
    pub fn glyph(&self, c: char) -> Option<&Glyph> {
        let code = c as u32;
        if (32..127).contains(&code) {
            self.glyphs.get((code - 32) as usize)
        } else {
            None
        }
    }

    pub fn len(&self) -> usize {
        self.glyphs.len()
    }

    pub fn is_empty(&self) -> bool {
        self.glyphs.is_empty()
    }
}
