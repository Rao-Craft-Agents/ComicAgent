//! A script laid out and inked: the title, a grid of panels, and in each panel its caption, its figures and props on
//! the ground, labels under the feet, and what is said in the sky above, each line with a tail to its speaker. What
//! does not fit is measured and said (findings, the model's to fix); the strip is drawn whatever it says.

use crate::figure;
use crate::geom::{Polyline, Rect, Vec2, circle};
use crate::layout::{Style, grid};
use crate::lettering::{Align, Lettering};
use crate::props::{self, Ground, PropInk, TextBox};
use crate::rng::{SplitMix64, stream};
use crate::scene::{Depth, Ink, Scene};
use crate::script::{
    BubbleKind, CaptionAt, Facing, Finding, FindingKind, Limits, Panel, Pose, Problem, PropKind,
    Script, Slot,
};
use crate::wobble::Wobble;

/// A strip, drawn: the scene, what it measured wrong, what was drawn other than asked, and its size in device pixels.
#[derive(Clone, Debug, PartialEq)]
pub struct Composed {
    pub scene: Scene,
    pub findings: Vec<Finding>,
    pub problems: Vec<Problem>,
    pub width: u32,
    pub height: u32,
}

/// The streams an element draws its hand from: `[panel, kind, index…]`.
mod streams {
    pub const BORDER: u32 = 0;
    pub const FIGURE: u32 = 1;
    pub const PROP: u32 = 2;
    pub const LABEL: u32 = 3;
    pub const CAPTION: u32 = 4;
    pub const BUBBLE: u32 = 5;
    pub const TAIL: u32 = 6;
    pub const SIGN: u32 = 7;
    pub const PROP_TEXT: u32 = 8;
    pub const TITLE: u32 = 9;
}

/// Lays out and inks `script` in `style`, its hand seeded by `seed`.
pub fn compose(script: &Script, style: &Style, seed: u64) -> Composed {
    let mut c = Composer {
        style,
        seed,
        ink: Ink::default(),
        findings: Vec::new(),
        problems: Vec::new(),
        hand: Lettering::hand(),
    };
    let n = script.panels.len().max(1);
    let page_w = style.page_width(n);
    let (cols, rows) = grid(n);
    let pw = style.panel_width(n);
    let art_h = (pw * style.art_ratio).round();
    let mut y = style.margin;
    if let Some(title) = script.title.as_deref().filter(|t| !t.trim().is_empty()) {
        let lines = c
            .hand
            .wrap(title, style.title_cap, page_w - 2.0 * style.margin);
        if lines.len() > 2 {
            c.findings.push(Finding::new(
                FindingKind::Overflow,
                "title",
                vec![lines.len() as i32, 2],
                format!(
                    "the title runs to {} lines and a title holds 2: say it in about {} characters",
                    lines.len(),
                    Limits::about(Limits::TITLE)
                ),
            ));
        }
        let mut rng = stream(seed, &[u32::MAX, streams::TITLE]);
        let lettered = c.hand.strokes(
            &lines,
            Vec2::new(page_w * 0.5, y),
            style.title_cap,
            Align::Center,
            &mut rng,
        );
        c.lettered(
            Depth::World,
            lettered.strokes,
            style.title_width,
            "title",
            lettered.missing,
        );
        y += Lettering::block_height(lines.len(), style.title_cap) + 18.0;
    }
    for row in 0..rows {
        let indices: Vec<usize> =
            (row * cols..((row + 1) * cols).min(script.panels.len())).collect();
        let captions: Vec<Option<(Vec<String>, CaptionAt)>> = indices
            .iter()
            .map(|&i| {
                script.panels[i].caption.as_ref().map(|cap| {
                    (
                        c.hand
                            .wrap(&cap.text, style.caption_cap, pw - 2.0 * style.caption_pad),
                        cap.at,
                    )
                })
            })
            .collect();
        let band = |at: CaptionAt| {
            captions
                .iter()
                .flatten()
                .filter(|(_, a)| *a == at)
                .map(|(lines, _)| {
                    Lettering::block_height(lines.len(), style.caption_cap)
                        + 2.0 * style.caption_pad
                        + 2.0
                })
                .fold(0.0, f32::max)
        };
        let (above, below) = (band(CaptionAt::Above), band(CaptionAt::Below));
        for (col, &i) in indices.iter().enumerate() {
            let x = style.margin + col as f32 * (pw + style.gutter);
            let frame = Rect::new(x, y, pw, above + art_h + below);
            let art = Rect::new(x, y + above, pw, art_h);
            c.panel(i, &script.panels[i], frame, art, captions[col].as_ref());
        }
        y += above + art_h + below + style.gutter;
    }
    let page_h = y - style.gutter + style.margin;
    let width = (page_w * style.scale).round() as u32;
    let height = (page_h * style.scale).ceil() as u32;
    let Composer {
        ink,
        findings,
        problems,
        ..
    } = c;
    Composed {
        scene: ink.into_scene(width, height, style.scale),
        findings,
        problems,
        width,
        height,
    }
}

struct Composer<'a> {
    style: &'a Style,
    seed: u64,
    ink: Ink,
    findings: Vec<Finding>,
    problems: Vec<Problem>,
    hand: Lettering,
}

/// One thing standing on a panel's ground.
#[derive(Clone, Debug)]
enum Thing {
    Figure {
        index: usize,
        pose: Pose,
        facing: Facing,
    },
    Prop {
        index: usize,
        kind: PropKind,
    },
}

/// A thing placed: its left edge and width in CSS px.
#[derive(Clone, Debug)]
struct Spot {
    thing: Thing,
    left: f32,
    width: f32,
    /// From the left edge to the feet (a figure) or to where it is drawn from (a prop).
    anchor: f32,
}

/// A figure as drawn, for the bubbles that point at it.
struct Drawn {
    id: String,
    head: Vec2,
    radius: f32,
    bounds: Rect,
}

impl Composer<'_> {
    fn rng(&self, path: &[u32]) -> SplitMix64 {
        stream(self.seed, path)
    }

    fn strokes(
        &mut self,
        depth: Depth,
        lines: Vec<Polyline>,
        width: f32,
        wobble: Wobble,
        path: &[u32],
    ) {
        let mut rng = self.rng(path);
        for line in lines {
            let drawn = wobble.apply(&line, &mut rng);
            self.ink.stroke(depth, drawn, width);
        }
    }

    /// Lettering's strokes, and a finding for the characters the font lacks.
    fn lettered(
        &mut self,
        depth: Depth,
        strokes: Vec<Polyline>,
        width: f32,
        element: &str,
        missing: Vec<char>,
    ) {
        self.ink.strokes(depth, strokes, width);
        if !missing.is_empty() {
            let shown: String = missing.iter().collect();
            self.findings.push(Finding::new(
                FindingKind::Limit,
                element,
                vec![missing.len() as i32],
                format!(
                    "{} has characters the lettering lacks ({shown}): use plain letters",
                    describe(element)
                ),
            ));
        }
    }

    /// Lettering `text` in a box, centered; a finding when it runs past the lines the box holds.
    fn boxed_text(&mut self, text: &str, at: TextBox, element: &str, what: &str, path: &[u32]) {
        let lines = self.hand.wrap(text, at.cap, at.rect.w);
        if lines.len() > at.lines {
            let chars = text.chars().count();
            let about = (chars * at.lines / lines.len()) / 5 * 5;
            self.findings.push(Finding::new(
                FindingKind::Overflow,
                element,
                vec![lines.len() as i32, at.lines as i32, about as i32],
                format!(
                    "{what} runs to {} lines and holds {}: say it in about {about} characters",
                    lines.len(),
                    at.lines
                ),
            ));
        }
        let height = Lettering::block_height(lines.len(), at.cap);
        let top = at.rect.y + ((at.rect.h - height) * 0.5).max(0.0);
        let mut rng = self.rng(path);
        let lettered = self.hand.strokes(
            &lines,
            Vec2::new(at.rect.center().x, top),
            at.cap,
            Align::Center,
            &mut rng,
        );
        self.lettered(
            at.depth,
            lettered.strokes,
            self.style.letter_width,
            element,
            lettered.missing,
        );
    }

    fn panel(
        &mut self,
        i: usize,
        panel: &Panel,
        frame: Rect,
        art: Rect,
        caption: Option<&(Vec<String>, CaptionAt)>,
    ) {
        let style = self.style;
        let p = i + 1;
        let at = format!("panel:{p}");
        let pi = i as u32;
        // The border, four strokes of the pen.
        let corners = frame.corners();
        let sides: Vec<Polyline> = (0..4)
            .map(|k| vec![corners[k], corners[(k + 1) % 4]])
            .collect();
        self.strokes(
            Depth::World,
            sides,
            style.border_width,
            Wobble::BORDER,
            &[pi, streams::BORDER],
        );
        // The caption in its band, ruled off from the drawing.
        if let Some((lines, place)) = caption {
            let pad = style.caption_pad;
            let (band_top, rule_y) = match place {
                CaptionAt::Above => (frame.y, art.y),
                CaptionAt::Below => (art.bottom(), art.bottom()),
            };
            let band = Rect::new(
                frame.x,
                band_top,
                frame.w,
                if *place == CaptionAt::Above {
                    art.y - frame.y
                } else {
                    frame.bottom() - art.bottom()
                },
            );
            self.strokes(
                Depth::World,
                vec![vec![
                    Vec2::new(frame.x, rule_y),
                    Vec2::new(frame.right(), rule_y),
                ]],
                style.border_width * 0.8,
                Wobble::BORDER,
                &[pi, streams::CAPTION, 1],
            );
            if lines.len() > Limits::CAPTION_LINES {
                let chars: usize = lines.iter().map(|l| l.chars().count() + 1).sum();
                let about = (chars * Limits::CAPTION_LINES / lines.len()) / 5 * 5;
                self.findings.push(Finding::new(
                    FindingKind::Overflow,
                    format!("{at}/caption"),
                    vec![lines.len() as i32, Limits::CAPTION_LINES as i32, about as i32],
                    format!(
                        "panel {p}'s caption runs to {} lines and a caption holds {}: say it in about {about} characters",
                        lines.len(),
                        Limits::CAPTION_LINES
                    ),
                ));
            }
            let mut rng = self.rng(&[pi, streams::CAPTION, 2]);
            let origin = Vec2::new(band.x + pad, band.y + pad + 1.0);
            let lettered =
                self.hand
                    .strokes(lines, origin, style.caption_cap, Align::Left, &mut rng);
            self.lettered(
                Depth::World,
                lettered.strokes,
                style.letter_width,
                &format!("{at}/caption"),
                lettered.missing,
            );
        }

        let h = art.h * style.figure_ratio;
        let ground = art.y + art.h * style.ground_ratio;
        let spots = self.arrange(panel, art, h, p);

        // What stands on the ground, then what sits on it.
        let mut drawn: Vec<Drawn> = Vec::new();
        let mut obstacles: Vec<Rect> = Vec::new();
        let mut surfaces: Vec<(Slot, f32, f32, f32, Facing)> = Vec::new();
        let slot_of = |thing: &Thing| match thing {
            Thing::Figure { index, .. } => panel.figures[*index].at,
            Thing::Prop { index, .. } => panel.props[*index].at,
        };
        let mut labels: Vec<Rect> = Vec::new();
        for spot in &spots {
            match &spot.thing {
                Thing::Figure {
                    index,
                    pose,
                    facing,
                } => {
                    let figure = &panel.figures[*index];
                    let foot = Vec2::new(spot.left + spot.anchor, ground);
                    let placed = figure::place(*pose, *facing, foot, h);
                    let fi = *index as u32;
                    self.strokes(
                        Depth::World,
                        placed.lines.clone(),
                        style.ink_width,
                        Wobble::INK,
                        &[pi, streams::FIGURE, fi],
                    );
                    if let Some(sign) = placed.sign {
                        self.ink.fill(Depth::Overlay, sign.corners().to_vec());
                        self.strokes(
                            Depth::Overlay,
                            vec![sign.outline()],
                            style.ink_width,
                            Wobble::INK,
                            &[pi, streams::SIGN, fi],
                        );
                        if let Some(text) = &figure.sign {
                            let rect = sign.expanded(-0.035 * h);
                            let what = format!("panel {p}'s sign held by {}", figure.id);
                            self.boxed_text(
                                text,
                                TextBox {
                                    rect,
                                    cap: style.sign_cap,
                                    lines: 2,
                                    depth: Depth::Overlay,
                                },
                                &format!("{at}/figure:{}", figure.id),
                                &what,
                                &[pi, streams::SIGN, fi, 1],
                            );
                        }
                        obstacles.push(sign);
                    }
                    if let Some(label) = &figure.label {
                        let lines = self.hand.wrap(label, style.label_cap, (1.2 * h).max(110.0));
                        let width = self.hand.width(&lines, style.label_cap);
                        let center = Vec2::new(placed.bounds.center().x, ground + 6.0);
                        let rect = Rect::new(
                            center.x - width * 0.5,
                            center.y,
                            width,
                            Lettering::block_height(lines.len(), style.label_cap),
                        );
                        if lines.len() > 1 || rect.bottom() > art.bottom() - 2.0 {
                            self.findings.push(Finding::new(
                                FindingKind::Overflow,
                                format!("{at}/figure:{}", figure.id),
                                vec![lines.len() as i32, 1],
                                format!(
                                    "panel {p}'s label under {} runs to {} lines and a label holds one: say it in about 15 characters",
                                    figure.id,
                                    lines.len()
                                ),
                            ));
                        }
                        if labels.iter().any(|r| r.expanded(4.0).intersects(&rect)) {
                            self.findings.push(Finding::new(
                                FindingKind::Overlap,
                                format!("{at}/figure:{}", figure.id),
                                vec![],
                                format!("panel {p}'s label under {} runs into the label beside it: shorten one, or move a figure to its own slot", figure.id),
                            ));
                        }
                        labels.push(rect);
                        let mut rng = self.rng(&[pi, streams::LABEL, fi]);
                        let lettered = self.hand.strokes(
                            &lines,
                            center,
                            style.label_cap,
                            Align::Center,
                            &mut rng,
                        );
                        self.lettered(
                            Depth::World,
                            lettered.strokes,
                            style.letter_width,
                            &format!("{at}/figure:{}", figure.id),
                            lettered.missing,
                        );
                    }
                    drawn.push(Drawn {
                        id: figure.id.clone(),
                        head: placed.head,
                        radius: placed.radius,
                        bounds: placed.bounds,
                    });
                }
                Thing::Prop { index, kind } => {
                    let prop = &panel.props[*index];
                    let user = self.user_side(&spots, spot);
                    let ink = props::draw(
                        *kind,
                        prop.text.as_deref(),
                        Ground {
                            left: spot.left + spot.anchor,
                            ground,
                            height: h,
                            user,
                        },
                    );
                    if let Some((l, r, top)) = ink.surface {
                        surfaces.push((
                            slot_of(&spot.thing).unwrap_or(Slot::Center),
                            l,
                            r,
                            top,
                            user,
                        ));
                    }
                    if matches!(
                        kind,
                        PropKind::Tree
                            | PropKind::Door
                            | PropKind::Signpost
                            | PropKind::Whiteboard
                            | PropKind::Graph
                            | PropKind::Arrow
                    ) {
                        obstacles.push(ink.bounds);
                    }
                    self.prop_ink(
                        ink,
                        prop.text.as_deref(),
                        *kind,
                        &at,
                        p,
                        &[pi, streams::PROP, *index as u32],
                    );
                }
            }
        }
        // Laptops on desks, arrows across the panel, clouds in the sky.
        for (index, prop) in panel.props.iter().enumerate() {
            let Some(kind) = prop.kind.known() else {
                continue;
            };
            let path = [pi, streams::PROP, index as u32];
            match kind {
                PropKind::Laptop => {
                    let surface = surfaces
                        .iter()
                        .find(|s| prop.at.is_none() || Some(s.0) == prop.at)
                        .copied();
                    if let Some((_, l, r, top, user)) = surface {
                        let left = match user {
                            Facing::Left => l + 0.02 * h,
                            Facing::Right => r - 0.36 * h,
                        };
                        let ink = props::laptop(left, top, h, user);
                        self.prop_ink(ink, None, kind, &at, p, &path);
                    }
                }
                PropKind::Cloud => {
                    let fx = match prop.at {
                        Some(Slot::Left) => 0.25,
                        Some(Slot::Right) => 0.75,
                        _ => 0.5,
                    };
                    let ink = props::cloud(
                        Vec2::new(art.x + fx * art.w, art.y + 0.11 * art.h + 0.12 * h),
                        h,
                    );
                    obstacles.push(ink.bounds);
                    self.prop_ink(ink, prop.text.as_deref(), kind, &at, p, &path);
                }
                _ => {}
            }
        }

        self.bubbles(panel, art, h, &drawn, &obstacles, p);
    }

    /// The side a prop's user is on: toward the figure beside it, else toward the panel's middle.
    fn user_side(&self, spots: &[Spot], spot: &Spot) -> Facing {
        let center = spot.left + spot.width * 0.5;
        let nearest = spots
            .iter()
            .filter(|s| matches!(s.thing, Thing::Figure { .. }))
            .map(|s| s.left + s.anchor)
            .min_by(|a, b| (a - center).abs().total_cmp(&(b - center).abs()));
        match nearest {
            Some(x) if x < center => Facing::Left,
            Some(_) => Facing::Right,
            None => Facing::Left,
        }
    }

    fn prop_ink(
        &mut self,
        ink: PropInk,
        text: Option<&str>,
        kind: PropKind,
        at: &str,
        p: usize,
        path: &[u32],
    ) {
        let style = self.style;
        for paper in ink.paper {
            self.ink.fill(Depth::Overlay, paper);
        }
        self.strokes(Depth::World, ink.world, style.ink_width, Wobble::INK, path);
        let mut over = path.to_vec();
        over.push(1);
        self.strokes(
            Depth::Overlay,
            ink.overlay,
            style.ink_width,
            Wobble::INK,
            &over,
        );
        if let (Some(text), Some(boxed)) = (text, ink.text) {
            let element = format!("{at}/prop:{}", path.get(2).map(|i| i + 1).unwrap_or(0));
            let what = format!("panel {p}'s {} text", kind.word());
            let mut letters = path.to_vec();
            letters.push(streams::PROP_TEXT);
            self.boxed_text(text, boxed, &element, &what, &letters);
        }
    }

    /// Where everything on the ground stands: groups by slot, a figure's props on the side it faces, groups pushed
    /// apart and kept inside the panel.
    fn arrange(&mut self, panel: &Panel, art: Rect, h: f32, p: usize) -> Vec<Spot> {
        let count = panel.figures.len();
        let slots: Vec<Slot> = panel
            .figures
            .iter()
            .enumerate()
            .map(|(i, f)| {
                f.at.unwrap_or(match (count, i) {
                    (1, _) => Slot::Center,
                    (2, 0) => Slot::Left,
                    (2, _) => Slot::Right,
                    (_, 0) => Slot::Left,
                    (_, 1) => Slot::Center,
                    (_, 2) => Slot::Right,
                    _ => Slot::Center,
                })
            })
            .collect();
        // Props that stand on the ground, each in a slot.
        let has_desk = panel
            .props
            .iter()
            .any(|p| p.kind.known() == Some(PropKind::Desk));
        let mut prop_slots: Vec<(usize, PropKind, Slot)> = Vec::new();
        for (index, prop) in panel.props.iter().enumerate() {
            let Some(kind) = prop.kind.known() else {
                continue;
            };
            let stands = match kind {
                PropKind::Cloud => false,
                PropKind::Laptop => !has_desk,
                _ => true,
            };
            if !stands {
                continue;
            }
            let slot = prop.at.unwrap_or_else(|| {
                let used = |s: Slot| {
                    slots.iter().filter(|&&x| x == s).count()
                        + prop_slots.iter().filter(|x| x.2 == s).count()
                };
                let sitting = panel
                    .figures
                    .iter()
                    .zip(&slots)
                    .find(|(f, _)| f.pose.pose() == Pose::Sitting)
                    .map(|(_, s)| *s);
                match (kind, sitting) {
                    (PropKind::Desk, Some(s)) => s,
                    _ => [Slot::Center, Slot::Right, Slot::Left]
                        .into_iter()
                        .min_by_key(|&s| used(s))
                        .unwrap_or(Slot::Center),
                }
            });
            prop_slots.push((index, kind, slot));
        }
        // Which way each figure faces.
        let facings: Vec<Facing> = panel
            .figures
            .iter()
            .zip(&slots)
            .enumerate()
            .map(|(i, (f, &slot))| {
                f.facing.unwrap_or_else(|| match slot {
                    Slot::Left => Facing::Right,
                    Slot::Right => Facing::Left,
                    Slot::Center => {
                        let others =
                            |s: Slot| slots.iter().enumerate().any(|(j, &x)| j != i && x == s);
                        let things = |s: Slot| prop_slots.iter().any(|x| x.2 == s);
                        if others(Slot::Left) && !others(Slot::Right) {
                            Facing::Left
                        } else if others(Slot::Right) && !others(Slot::Left) {
                            Facing::Right
                        } else if things(Slot::Left) && !things(Slot::Right) {
                            Facing::Left
                        } else {
                            Facing::Right
                        }
                    }
                })
            })
            .collect();
        // Groups, left to right.
        let gap = 0.06 * h;
        let mut groups: Vec<(Slot, Vec<Spot>, f32)> = Vec::new();
        for slot in Slot::ALL {
            let mut things: Vec<Thing> = Vec::new();
            for (i, f) in panel.figures.iter().enumerate() {
                if slots[i] == slot {
                    things.push(Thing::Figure {
                        index: i,
                        pose: f.pose.pose(),
                        facing: facings[i],
                    });
                }
            }
            let first_facing = things.iter().find_map(|t| match t {
                Thing::Figure { facing, .. } => Some(*facing),
                _ => None,
            });
            let mut stuff: Vec<Thing> = prop_slots
                .iter()
                .filter(|x| x.2 == slot)
                .map(|x| Thing::Prop {
                    index: x.0,
                    kind: x.1,
                })
                .collect();
            if first_facing == Some(Facing::Left) {
                stuff.reverse();
                stuff.extend(things);
                things = stuff;
            } else {
                things.extend(stuff);
            }
            if things.is_empty() {
                continue;
            }
            let mut spots: Vec<Spot> = Vec::new();
            let mut x = 0.0;
            for (k, thing) in things.iter().enumerate() {
                let (width, anchor) = match thing {
                    Thing::Figure { pose, facing, .. } => {
                        let (lo, hi) = figure::extent(*pose);
                        let (lo, hi) = if *facing == Facing::Left {
                            (-hi, -lo)
                        } else {
                            (lo, hi)
                        };
                        ((hi - lo + 0.12) * h, (-lo + 0.06) * h)
                    }
                    Thing::Prop { kind, .. } => (props::width(*kind) * h, 0.0),
                };
                if k > 0 {
                    // A desk tucks under the hands of the figure sitting at it.
                    let tucked = matches!(
                        (&things[k - 1], thing),
                        (
                            Thing::Figure {
                                pose: Pose::Sitting,
                                facing: Facing::Right,
                                ..
                            },
                            Thing::Prop {
                                kind: PropKind::Desk,
                                ..
                            }
                        ) | (
                            Thing::Prop {
                                kind: PropKind::Desk,
                                ..
                            },
                            Thing::Figure {
                                pose: Pose::Sitting,
                                facing: Facing::Left,
                                ..
                            }
                        )
                    );
                    x += if tucked { -0.19 * h } else { gap };
                }
                spots.push(Spot {
                    thing: thing.clone(),
                    left: x,
                    width,
                    anchor,
                });
                x += width;
            }
            groups.push((slot, spots, x));
        }
        // Where each group's middle goes, then push apart and keep inside.
        let center_used = groups.iter().any(|g| g.0 == Slot::Center);
        let side_used = groups.iter().any(|g| g.0 != Slot::Center);
        let fraction = |slot: Slot| match (slot, center_used && side_used) {
            (Slot::Left, true) => 0.20,
            (Slot::Left, false) => 0.28,
            (Slot::Center, _) => 0.50,
            (Slot::Right, true) => 0.80,
            (Slot::Right, false) => 0.72,
        };
        let pad = 10.0;
        let between = 0.08 * h;
        let mut lefts: Vec<f32> = groups
            .iter()
            .map(|g| art.x + fraction(g.0) * art.w - g.2 * 0.5)
            .collect();
        for k in 1..lefts.len() {
            lefts[k] = lefts[k].max(lefts[k - 1] + groups[k - 1].2 + between);
        }
        if let (Some(last), Some(g)) = (lefts.last().copied(), groups.last()) {
            let over = last + g.2 - (art.right() - pad);
            if over > 0.0 {
                let n = lefts.len();
                lefts[n - 1] -= over;
                for k in (0..n - 1).rev() {
                    lefts[k] = lefts[k].min(lefts[k + 1] - between - groups[k].2);
                }
            }
        }
        if let Some(&first) = lefts.first() {
            if first < art.x + pad {
                let total: f32 = groups.iter().map(|g| g.2).sum::<f32>()
                    + between * (groups.len().saturating_sub(1)) as f32;
                self.findings.push(Finding::new(
                    FindingKind::Outside,
                    format!("panel:{p}"),
                    vec![total.round() as i32, (art.w - 2.0 * pad).round() as i32],
                    format!(
                        "panel {p} stands more side by side than it holds ({} px in {} px): move a figure or a prop to another panel",
                        total.round(),
                        (art.w - 2.0 * pad).round()
                    ),
                ));
                let shift = art.x + pad - first;
                for left in &mut lefts {
                    *left += shift;
                }
            }
        }
        groups
            .into_iter()
            .zip(lefts)
            .flat_map(|((_, spots, _), left)| {
                spots.into_iter().map(move |s| Spot {
                    left: s.left + left,
                    ..s
                })
            })
            .collect()
    }

    /// What is said: each line lettered in the sky above its speaker, below any earlier line it would cover, the whole
    /// stack then lowered toward the heads so the tails stay short, each with a tail toward its speaker's head (a voice
    /// from outside the panel points at the panel's edge).
    fn bubbles(
        &mut self,
        panel: &Panel,
        art: Rect,
        h: f32,
        drawn: &[Drawn],
        obstacles: &[Rect],
        p: usize,
    ) {
        let style = self.style;
        let pi = (p - 1) as u32;
        let at = format!("panel:{p}");
        let sky_top = art.y + 12.0;
        let crown = drawn
            .iter()
            .map(|d| d.bounds.y)
            .fold(art.y + art.h * style.ground_ratio - h, f32::min);
        let sky_bottom = crown - 10.0;
        let max_width = art.w * style.bubble_width;
        let left_free = !drawn.iter().any(|d| d.head.x < art.x + art.w * 0.3);

        // Plan: lines, size and place of each.
        struct Plan {
            lines: Vec<String>,
            w: f32,
            bh: f32,
            mx: f32,
            my: f32,
            cx: f32,
            y: f32,
        }
        let mut plans: Vec<Plan> = Vec::new();
        let mut taken: Vec<Rect> = obstacles.to_vec();
        let mut corridors: Vec<Rect> = Vec::new();
        for (j, bubble) in panel.bubbles.iter().enumerate() {
            let lines = self.hand.wrap(&bubble.text, style.bubble_cap, max_width);
            if lines.len() > Limits::BUBBLE_LINES {
                let chars = bubble.text.chars().count();
                let about = (chars * Limits::BUBBLE_LINES / lines.len()) / 5 * 5;
                self.findings.push(Finding::new(
                    FindingKind::Overflow,
                    format!("{at}/bubble:{}", j + 1),
                    vec![lines.len() as i32, Limits::BUBBLE_LINES as i32, about as i32],
                    format!(
                        "panel {p}'s bubble by {} runs to {} lines and a bubble holds {}: say it in about {about} characters",
                        bubble.by,
                        lines.len(),
                        Limits::BUBBLE_LINES
                    ),
                ));
            }
            let w = self.hand.width(&lines, style.bubble_cap);
            let bh = Lettering::block_height(lines.len(), style.bubble_cap);
            let (mx, my) = match bubble.kind {
                BubbleKind::Speech => (4.0, 4.0),
                kind => {
                    props::outline_margin(Rect::new(0.0, 0.0, w, bh), kind == BubbleKind::Thought)
                }
            };
            let speaker = drawn.iter().find(|d| d.id == bubble.by);
            let target = match speaker {
                Some(d) => d.head.x,
                None if left_free => art.x + art.w * 0.18,
                None => art.right() - art.w * 0.18,
            };
            let lo = art.x + 10.0 + w * 0.5 + mx;
            let hi = (art.right() - 10.0 - w * 0.5 - mx).max(lo);
            // Where it could go: over its speaker, or either side of a tail already drawn down through the sky.
            let mut candidates = vec![target.clamp(lo, hi)];
            for c in &corridors {
                candidates.push((c.x - 8.0 - w * 0.5 - mx).clamp(lo, hi));
                candidates.push((c.right() + 8.0 + w * 0.5 + mx).clamp(lo, hi));
            }
            let settle = |cx: f32| {
                let mut y = sky_top + my;
                for _ in 0..16 {
                    let room = Rect::new(cx - w * 0.5 - mx, y - my, w + 2.0 * mx, bh + 2.0 * my);
                    match taken
                        .iter()
                        .filter(|r| r.expanded(6.0).intersects(&room))
                        .map(|r| r.bottom())
                        .reduce(f32::max)
                    {
                        Some(bottom) => y = bottom + 6.0 + my,
                        None => break,
                    }
                }
                y
            };
            let (cx, y) = candidates
                .into_iter()
                .map(|cx| (cx, settle(cx)))
                .min_by(|a, b| {
                    a.1.total_cmp(&b.1)
                        .then((a.0 - target).abs().total_cmp(&(b.0 - target).abs()))
                })
                .unwrap_or((target, sky_top));
            let room = Rect::new(cx - w * 0.5 - mx, y - my, w + 2.0 * mx, bh + 2.0 * my);
            taken.push(room);
            if let Some(d) = speaker {
                let head_top = d.head.y - d.radius;
                let x = d.head.x.clamp(room.x + 8.0, room.right() - 8.0);
                let corridor = Rect::new(
                    x - 7.0,
                    room.bottom(),
                    14.0,
                    (head_top - room.bottom()).max(0.0),
                );
                taken.push(corridor);
                corridors.push(corridor);
            }
            plans.push(Plan {
                lines,
                w,
                bh,
                mx,
                my,
                cx,
                y,
            });
        }
        // Settle: lower the whole stack toward the heads, leaving room for a tail, never into what stands below.
        let room = |plan: &Plan, dy: f32| {
            Rect::new(
                plan.cx - plan.w * 0.5 - plan.mx,
                plan.y + dy - plan.my,
                plan.w + 2.0 * plan.mx,
                plan.bh + 2.0 * plan.my,
            )
        };
        let lowest = plans
            .iter()
            .map(|plan| room(plan, 0.0).bottom())
            .fold(sky_top, f32::max);
        let mut shift = (sky_bottom - 0.32 * h - lowest).max(0.0);
        for plan in &plans {
            for obstacle in obstacles {
                let r = room(plan, 0.0);
                if r.overlaps_horizontally(&obstacle.expanded(6.0)) && obstacle.y >= r.bottom() {
                    shift = shift.min((obstacle.y - 6.0 - r.bottom()).max(0.0));
                }
            }
        }

        // Draw.
        let mut lowest = sky_top;
        let mut covered = false;
        for (j, (bubble, plan)) in panel.bubbles.iter().zip(&plans).enumerate() {
            let element = format!("{at}/bubble:{}", j + 1);
            let y = plan.y + shift;
            let block = Rect::new(plan.cx - plan.w * 0.5, y, plan.w, plan.bh);
            let path = |k: u32| [pi, streams::TAIL, j as u32, k];
            let outer = match bubble.kind {
                BubbleKind::Speech => block.expanded(plan.mx),
                BubbleKind::Thought | BubbleKind::Shout => {
                    let (outline, paper, bounds) = if bubble.kind == BubbleKind::Thought {
                        props::thought_outline(block)
                    } else {
                        props::shout_outline(block)
                    };
                    self.ink.fill(Depth::Overlay, paper);
                    self.strokes(
                        Depth::Overlay,
                        vec![outline],
                        style.ink_width * 0.9,
                        Wobble::INK,
                        &path(0),
                    );
                    bounds
                }
            };
            lowest = lowest.max(outer.bottom());
            let mut rng = self.rng(&[pi, streams::BUBBLE, j as u32]);
            let lettered = self.hand.strokes(
                &plan.lines,
                Vec2::new(plan.cx, y),
                style.bubble_cap,
                Align::Center,
                &mut rng,
            );
            self.lettered(
                Depth::Overlay,
                lettered.strokes,
                style.letter_width,
                &element,
                lettered.missing,
            );
            match drawn.iter().find(|d| d.id == bubble.by) {
                Some(d) => {
                    let head_top = Vec2::new(d.head.x, d.head.y - d.radius);
                    let start = Vec2::new(
                        head_top.x.clamp(outer.x + 8.0, outer.right() - 8.0),
                        outer.bottom() + 3.0,
                    );
                    let toward = head_top - start;
                    if toward.y > 14.0 {
                        let end = head_top - toward.normalized() * 8.0;
                        if bubble.kind == BubbleKind::Thought {
                            let dots: Vec<Polyline> = [(0.3, 5.0), (0.6, 3.5), (0.88, 2.5)]
                                .iter()
                                .map(|&(t, r)| circle(start.lerp(end, t), r, 12))
                                .collect();
                            self.strokes(
                                Depth::Overlay,
                                dots,
                                style.ink_width * 0.8,
                                Wobble::NONE,
                                &path(1),
                            );
                        } else {
                            self.strokes(
                                Depth::Overlay,
                                vec![vec![start, end]],
                                style.ink_width * 0.8,
                                Wobble::INK,
                                &path(1),
                            );
                        }
                    }
                    let over_head = drawn.iter().find(|o| {
                        Rect::new(
                            o.head.x - o.radius,
                            o.head.y - o.radius,
                            2.0 * o.radius,
                            2.0 * o.radius,
                        )
                        .expanded(3.0)
                        .intersects(&outer)
                    });
                    if let Some(other) = over_head {
                        covered = true;
                        let whose = if other.id == bubble.by {
                            "its own speaker".to_string()
                        } else {
                            other.id.clone()
                        };
                        self.findings.push(Finding::new(
                            FindingKind::Overlap,
                            element.clone(),
                            vec![],
                            format!("panel {p}'s bubble by {} covers {whose}'s head: say less, or move a line to the next panel", bubble.by),
                        ));
                    }
                }
                None => {
                    // A voice from outside: a tail to the nearer edge.
                    let left = plan.cx < art.center().x;
                    let edge = if left { art.x + 3.0 } else { art.right() - 3.0 };
                    let from = Vec2::new(
                        if left {
                            outer.x - 3.0
                        } else {
                            outer.right() + 3.0
                        },
                        outer.center().y,
                    );
                    if (edge - from.x).abs() > 10.0 {
                        self.strokes(
                            Depth::Overlay,
                            vec![vec![from, Vec2::new(edge, from.y + 6.0)]],
                            style.ink_width * 0.8,
                            Wobble::INK,
                            &path(1),
                        );
                    }
                }
            }
            if outer.w > art.w - 20.0 {
                self.findings.push(Finding::new(
                    FindingKind::Outside,
                    element.clone(),
                    vec![outer.w.round() as i32, (art.w - 20.0).round() as i32],
                    format!(
                        "panel {p}'s bubble by {} is wider than the panel: say less",
                        bubble.by
                    ),
                ));
            }
        }
        if !plans.is_empty() && lowest > sky_bottom && !covered {
            let need = (lowest - sky_top).round() as i32;
            let have = (sky_bottom - sky_top).round().max(0.0) as i32;
            self.findings.push(Finding::new(
                FindingKind::Overflow,
                at,
                vec![need, have],
                format!("panel {p}'s bubbles need {need} px of sky and have {have}: say less, or move a line to the next panel"),
            ));
        }
    }
}

/// An element's name in a sentence.
fn describe(element: &str) -> String {
    let mut parts = element.split('/');
    let head = parts.next().unwrap_or(element);
    let Some(p) = head.strip_prefix("panel:") else {
        return format!("the {head}");
    };
    match parts.next() {
        Some(rest) => format!("panel {p}'s {}", rest.replace(':', " ")),
        None => format!("panel {p}"),
    }
}
