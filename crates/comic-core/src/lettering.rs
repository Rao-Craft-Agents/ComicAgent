//! Hand lettering: capitals of Hershey Simplex, condensed and jittered a little glyph by glyph, the way a person
//! letters a comic. Text is folded to ASCII first (curly quotes, dashes, accents); a character the font still lacks is
//! drawn as a small box and reported, never dropped in silence.

use crate::geom::{Polyline, Vec2, small_sin_cos};
use crate::hershey::{CAP_HEIGHT, CAP_TOP, Glyph, HersheyFont};
use crate::rng::SplitMix64;

/// How a block of lines lines up on its origin.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Align {
    /// The origin is the block's top-left corner.
    Left,
    /// The origin is the middle of the block's top edge.
    Center,
}

/// Lettering in one hand.
#[derive(Clone, Copy, Debug)]
pub struct Lettering {
    font: &'static HersheyFont,
    /// Horizontal scale of every glyph: hand lettering is narrower than the plotter's.
    pub condense: f32,
    /// Most a glyph turns either way, in radians.
    pub tilt: f32,
    /// Most a glyph moves off its baseline, in CSS px.
    pub bounce: f32,
    /// Most a glyph's point moves, in CSS px.
    pub shake: f32,
}

/// Lettered text: the strokes and the characters the font lacked.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Lettered {
    pub strokes: Vec<Polyline>,
    pub missing: Vec<char>,
}

/// The width a character the font lacks takes, in font units: a box.
const MISSING_ADVANCE: f32 = 14.0;

impl Lettering {
    /// The comic's hand.
    pub fn hand() -> Lettering {
        Lettering {
            font: HersheyFont::simplex(),
            condense: 0.8,
            tilt: 0.035,
            bounce: 0.5,
            shake: 0.3,
        }
    }

    /// The text as it is lettered: capitals, folded to ASCII, whitespace as single spaces.
    pub fn prepare(text: &str) -> String {
        let mut out = String::with_capacity(text.len());
        for c in text.chars() {
            match fold(c) {
                Some(folded) => out.push_str(folded),
                None if c.is_whitespace() => out.push(' '),
                None => out.extend(c.to_uppercase()),
            }
        }
        let mut squeezed = String::with_capacity(out.len());
        for word in out.split(' ').filter(|w| !w.is_empty()) {
            if !squeezed.is_empty() {
                squeezed.push(' ');
            }
            squeezed.push_str(word);
        }
        squeezed.to_ascii_uppercase()
    }

    /// Height of a line, baseline to baseline.
    pub fn line_height(cap: f32) -> f32 {
        cap * 1.45
    }

    /// Height of a block of `lines` lines: the first cap to the last baseline.
    pub fn block_height(lines: usize, cap: f32) -> f32 {
        if lines == 0 {
            0.0
        } else {
            cap + (lines - 1) as f32 * Lettering::line_height(cap)
        }
    }

    fn glyph(&self, c: char) -> Option<&'static Glyph> {
        self.font.glyph(c)
    }

    fn advance(&self, c: char, cap: f32) -> f32 {
        let units = self.glyph(c).map(Glyph::advance).unwrap_or(MISSING_ADVANCE);
        units * cap / CAP_HEIGHT * self.condense
    }

    /// The width of one line of prepared text.
    pub fn measure(&self, text: &str, cap: f32) -> f32 {
        Lettering::prepare(text)
            .chars()
            .map(|c| self.advance(c, cap))
            .sum()
    }

    /// The characters of `text` the font lacks, once each.
    pub fn missing(&self, text: &str) -> Vec<char> {
        let mut missing: Vec<char> = Vec::new();
        for c in Lettering::prepare(text).chars() {
            if self.glyph(c).is_none() && !missing.contains(&c) {
                missing.push(c);
            }
        }
        missing
    }

    /// `text` broken into lines no wider than `max_width`, at spaces; a word wider than a line is split.
    pub fn wrap(&self, text: &str, cap: f32, max_width: f32) -> Vec<String> {
        let prepared = Lettering::prepare(text);
        let space = self.advance(' ', cap);
        let mut lines: Vec<String> = Vec::new();
        let mut line = String::new();
        let mut width = 0.0;
        for word in prepared.split(' ') {
            let mut word = word.to_string();
            let mut word_width: f32 = word.chars().map(|c| self.advance(c, cap)).sum();
            while word_width > max_width && word.chars().count() > 1 {
                // Split a word that cannot fit any line.
                if !line.is_empty() {
                    lines.push(std::mem::take(&mut line));
                    width = 0.0;
                }
                let mut head = String::new();
                let mut head_width = 0.0;
                for c in word.chars() {
                    let w = self.advance(c, cap);
                    if head_width + w > max_width && !head.is_empty() {
                        break;
                    }
                    head.push(c);
                    head_width += w;
                }
                word = word[head.len()..].to_string();
                word_width -= head_width;
                lines.push(head);
            }
            if word.is_empty() {
                continue;
            }
            if line.is_empty() {
                line = word;
                width = word_width;
            } else if width + space + word_width <= max_width {
                line.push(' ');
                line.push_str(&word);
                width += space + word_width;
            } else {
                lines.push(std::mem::replace(&mut line, word));
                width = word_width;
            }
        }
        if !line.is_empty() {
            lines.push(line);
        }
        lines
    }

    /// The widest of `lines`.
    pub fn width(&self, lines: &[String], cap: f32) -> f32 {
        lines
            .iter()
            .map(|l| self.measure(l, cap))
            .fold(0.0, f32::max)
    }

    /// Strokes lettering `lines` (already wrapped) from `origin`.
    pub fn strokes(
        &self,
        lines: &[String],
        origin: Vec2,
        cap: f32,
        align: Align,
        rng: &mut SplitMix64,
    ) -> Lettered {
        let scale = cap / CAP_HEIGHT;
        let sx = scale * self.condense;
        let mut out = Lettered::default();
        for (row, line) in lines.iter().enumerate() {
            let width = self.measure(line, cap);
            let top = origin.y + row as f32 * Lettering::line_height(cap);
            let mut pen = match align {
                Align::Left => origin.x,
                Align::Center => origin.x - width * 0.5,
            };
            for c in line.chars() {
                let advance = self.advance(c, cap);
                let (sin, cos) = small_sin_cos(rng.signed() * self.tilt);
                let lift = rng.signed() * self.bounce;
                let pivot = Vec2::new(pen + advance * 0.5, top + cap * 0.5 + lift);
                let mut shake = || Vec2::new(rng.signed() * self.shake, rng.signed() * self.shake);
                match self.glyph(c) {
                    Some(glyph) => {
                        for stroke in &glyph.strokes {
                            let line: Polyline = stroke
                                .iter()
                                .map(|&(gx, gy)| {
                                    let p = Vec2::new(
                                        pen + (f32::from(gx) - f32::from(glyph.left)) * sx,
                                        top + (f32::from(gy) - CAP_TOP) * scale + lift,
                                    );
                                    pivot + (p - pivot).rotated(sin, cos) + shake()
                                })
                                .collect();
                            if line.len() == 1 {
                                // A dot: a short stroke so it shows.
                                out.strokes
                                    .push(vec![line[0], line[0] + Vec2::new(0.4, 0.0)]);
                            } else {
                                out.strokes.push(line);
                            }
                        }
                    }
                    None => {
                        if !out.missing.contains(&c) {
                            out.missing.push(c);
                        }
                        let (x0, x1) = (pen + advance * 0.15, pen + advance * 0.85);
                        let (y0, y1) = (top + cap * 0.25, top + cap);
                        let boxed = [
                            Vec2::new(x0, y0),
                            Vec2::new(x1, y0),
                            Vec2::new(x1, y1),
                            Vec2::new(x0, y1),
                            Vec2::new(x0, y0),
                        ];
                        out.strokes.push(
                            boxed
                                .iter()
                                .map(|&p| pivot + (p - pivot).rotated(sin, cos))
                                .collect(),
                        );
                    }
                }
                pen += advance;
            }
        }
        out
    }
}

/// A character's ASCII spelling, for the ones a person types and the font lacks.
fn fold(c: char) -> Option<&'static str> {
    Some(match c {
        '\u{2018}' | '\u{2019}' | '\u{201A}' | '\u{2032}' | '`' | '\u{00B4}' => "'",
        '\u{201C}' | '\u{201D}' | '\u{201E}' | '\u{2033}' | '\u{00AB}' | '\u{00BB}' => "\"",
        '\u{2010}' | '\u{2011}' | '\u{2012}' | '\u{2013}' | '\u{2014}' | '\u{2015}'
        | '\u{2212}' => "-",
        '\u{2026}' => "...",
        '\u{00A0}' | '\u{2009}' | '\u{202F}' | '\t' | '\n' | '\r' => " ",
        '\u{00D7}' => "X",
        '\u{2192}' => "->",
        '\u{2190}' => "<-",
        'à' | 'á' | 'â' | 'ã' | 'ä' | 'å' | 'À' | 'Á' | 'Â' | 'Ã' | 'Ä' | 'Å' => "A",
        'æ' | 'Æ' => "AE",
        'ç' | 'Ç' => "C",
        'è' | 'é' | 'ê' | 'ë' | 'È' | 'É' | 'Ê' | 'Ë' => "E",
        'ì' | 'í' | 'î' | 'ï' | 'Ì' | 'Í' | 'Î' | 'Ï' => "I",
        'ñ' | 'Ñ' => "N",
        'ò' | 'ó' | 'ô' | 'õ' | 'ö' | 'ø' | 'Ò' | 'Ó' | 'Ô' | 'Õ' | 'Ö' | 'Ø' => "O",
        'œ' | 'Œ' => "OE",
        'ù' | 'ú' | 'û' | 'ü' | 'Ù' | 'Ú' | 'Û' | 'Ü' => "U",
        'ý' | 'ÿ' | 'Ý' => "Y",
        'ß' => "SS",
        _ => return None,
    })
}
