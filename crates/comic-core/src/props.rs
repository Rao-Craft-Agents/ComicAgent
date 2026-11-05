//! Props: the few things a webcomic panel needs, drawn in the same ink as the figures. Sizes are in figure heights so a
//! prop always sits right beside a person; every prop stands on the ground except the arrow and the cloud.

use crate::geom::{Polyline, Rect, Vec2, bezier, circle, ellipse, small_sin_cos, turn};
use crate::scene::Depth;
use crate::script::{Facing, PropKind};

/// Where lettering goes on a prop, how big, and how many lines it holds.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TextBox {
    pub rect: Rect,
    pub cap: f32,
    pub lines: usize,
    pub depth: Depth,
}

/// A prop drawn: its lines, the paper behind the ones drawn over the world, where its text goes, and what it covers.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PropInk {
    pub world: Vec<Polyline>,
    pub overlay: Vec<Polyline>,
    /// Paper drawn under `overlay`.
    pub paper: Vec<Vec<Vec2>>,
    pub text: Option<TextBox>,
    pub bounds: Rect,
    /// A surface something can sit on: its left and right ends and its height.
    pub surface: Option<(f32, f32, f32)>,
}

/// Where a ground prop stands: its left edge, the ground, the figure height it is sized by, and the side its user is on.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Ground {
    pub left: f32,
    pub ground: f32,
    pub height: f32,
    pub user: Facing,
}

/// How much ground a prop takes, in figure heights; zero for one that does not stand on it.
pub fn width(kind: PropKind) -> f32 {
    match kind {
        PropKind::Desk => 0.62,
        PropKind::Laptop => 0.34,
        PropKind::Whiteboard | PropKind::Graph => 0.95,
        PropKind::Box => 0.40,
        PropKind::Door => 0.36,
        PropKind::Tree => 0.66,
        PropKind::Signpost => 0.72,
        PropKind::Arrow => 0.55,
        PropKind::Cloud => 0.0,
    }
}

/// A board on two legs (a whiteboard, a graph's chart): the board and its legs.
fn board(at: Ground) -> (Rect, Vec<Polyline>) {
    let h = at.height;
    let w = width(PropKind::Whiteboard) * h;
    let rect = Rect::new(at.left, at.ground - 0.98 * h, w, 0.62 * h);
    let legs = vec![
        vec![
            Vec2::new(rect.x + 0.12 * h, rect.bottom()),
            Vec2::new(rect.x + 0.07 * h, at.ground),
        ],
        vec![
            Vec2::new(rect.right() - 0.12 * h, rect.bottom()),
            Vec2::new(rect.right() - 0.07 * h, at.ground),
        ],
    ];
    (rect, legs)
}

/// A ground prop. `text` is only read for the shape of a graph's line.
pub fn draw(kind: PropKind, text: Option<&str>, at: Ground) -> PropInk {
    let h = at.height;
    let g = at.ground;
    let x = at.left;
    let w = width(kind) * h;
    let mut ink = PropInk {
        bounds: Rect::new(x, g - h, w, h),
        ..Default::default()
    };
    match kind {
        PropKind::Desk => {
            let top = g - 0.35 * h;
            ink.world
                .push(vec![Vec2::new(x, top), Vec2::new(x + w, top)]);
            // The leg on its user's side stands back, out of the way of their knees.
            let (left_leg, right_leg) = match at.user {
                Facing::Left => (x + 0.10 * h, x + w - 0.04 * h),
                Facing::Right => (x + 0.04 * h, x + w - 0.10 * h),
            };
            ink.world
                .push(vec![Vec2::new(left_leg, top), Vec2::new(left_leg, g)]);
            ink.world
                .push(vec![Vec2::new(right_leg, top), Vec2::new(right_leg, g)]);
            // A drawer on the side away from its user.
            let (d0, d1) = match at.user {
                Facing::Left => (x + w - 0.28 * h, x + w - 0.04 * h),
                Facing::Right => (x + 0.04 * h, x + 0.28 * h),
            };
            ink.world.push(vec![
                Vec2::new(d0, top),
                Vec2::new(d0, top + 0.13 * h),
                Vec2::new(d1, top + 0.13 * h),
            ]);
            ink.world.push(vec![
                Vec2::new((d0 + d1) * 0.5 - 0.03 * h, top + 0.065 * h),
                Vec2::new((d0 + d1) * 0.5 + 0.03 * h, top + 0.065 * h),
            ]);
            ink.bounds = Rect::new(x, top, w, g - top);
            ink.surface = Some((x + 0.02 * h, x + w - 0.02 * h, top));
        }
        PropKind::Laptop => return laptop(x + 0.04 * h, g, h, at.user),
        PropKind::Whiteboard => {
            let (rect, legs) = board(at);
            ink.world.push(rect.outline());
            ink.world.extend(legs);
            ink.world.push(vec![
                Vec2::new(rect.x + 0.10 * h, rect.bottom() + 0.025 * h),
                Vec2::new(rect.right() - 0.10 * h, rect.bottom() + 0.025 * h),
            ]);
            ink.text = Some(TextBox {
                rect: rect.expanded(-0.07 * h),
                cap: 12.0,
                lines: 3,
                depth: Depth::World,
            });
            ink.bounds = Rect::new(rect.x, rect.y, rect.w, g - rect.y);
        }
        PropKind::Graph => {
            let (rect, legs) = board(at);
            ink.world.push(rect.outline());
            ink.world.extend(legs);
            let shape = text.map(|t| t.trim().to_lowercase()).unwrap_or_default();
            let labelled = !matches!(shape.as_str(), "" | "up" | "down" | "spike" | "wiggle");
            let top = rect.y + if labelled { 0.20 * h } else { 0.08 * h };
            let origin = Vec2::new(rect.x + 0.10 * h, rect.bottom() - 0.08 * h);
            let end = rect.right() - 0.08 * h;
            ink.world.push(vec![
                Vec2::new(origin.x, top),
                origin,
                Vec2::new(end, origin.y),
            ]);
            let (span, rise) = (end - origin.x - 0.04 * h, origin.y - top - 0.04 * h);
            let p = |u: f32, v: f32| {
                Vec2::new(
                    origin.x + 0.02 * h + u * span,
                    origin.y - 0.02 * h - v * rise,
                )
            };
            let curve = match shape.as_str() {
                "down" => bezier(p(0.0, 0.92), p(0.35, 0.85), p(0.55, 0.15), p(1.0, 0.08), 16),
                "spike" => vec![
                    p(0.0, 0.12),
                    p(0.45, 0.15),
                    p(0.52, 0.95),
                    p(0.58, 0.14),
                    p(1.0, 0.12),
                ],
                "wiggle" => (0..=24)
                    .map(|k| {
                        let (_, s) = turn(k * 6);
                        p(k as f32 / 24.0, 0.5 + 0.32 * s)
                    })
                    .collect(),
                _ => bezier(p(0.0, 0.08), p(0.45, 0.15), p(0.65, 0.85), p(1.0, 0.92), 16),
            };
            ink.world.push(curve);
            if labelled {
                ink.text = Some(TextBox {
                    rect: Rect::new(
                        rect.x + 0.06 * h,
                        rect.y + 0.04 * h,
                        rect.w - 0.12 * h,
                        0.14 * h,
                    ),
                    cap: 10.0,
                    lines: 1,
                    depth: Depth::World,
                });
            }
            ink.bounds = Rect::new(rect.x, rect.y, rect.w, g - rect.y);
        }
        PropKind::Box => {
            let s = 0.30 * h;
            let (dx, dy) = (0.10 * h, 0.07 * h);
            ink.world.push(Rect::new(x, g - s, s, s).outline());
            ink.world.push(vec![
                Vec2::new(x, g - s),
                Vec2::new(x + dx, g - s - dy),
                Vec2::new(x + s + dx, g - s - dy),
                Vec2::new(x + s, g - s),
            ]);
            ink.world.push(vec![
                Vec2::new(x + s + dx, g - s - dy),
                Vec2::new(x + s + dx, g - dy),
                Vec2::new(x + s, g),
            ]);
            ink.bounds = Rect::new(x, g - s - dy, s + dx, s + dy);
        }
        PropKind::Door => {
            let top = g - 1.12 * h;
            ink.world.push(vec![
                Vec2::new(x, g),
                Vec2::new(x, top),
                Vec2::new(x + w, top),
                Vec2::new(x + w, g),
            ]);
            let knob_x = match at.user {
                Facing::Left => x + 0.07 * h,
                Facing::Right => x + w - 0.07 * h,
            };
            ink.world
                .push(circle(Vec2::new(knob_x, g - 0.55 * h), 0.025 * h, 12));
            ink.bounds = Rect::new(x, top, w, g - top);
        }
        PropKind::Tree => {
            let cx = x + w * 0.5;
            let canopy = Vec2::new(cx, g - 0.83 * h);
            let r = 0.32 * h;
            ink.world.push(vec![
                Vec2::new(cx - 0.045 * h, g),
                Vec2::new(cx - 0.04 * h, canopy.y + r - 0.01 * h),
            ]);
            ink.world.push(vec![
                Vec2::new(cx + 0.045 * h, g),
                Vec2::new(cx + 0.04 * h, canopy.y + r - 0.01 * h),
            ]);
            ink.world.push(circle(canopy, r, 24));
            ink.bounds = Rect::new(cx - r, canopy.y - r, 2.0 * r, g - (canopy.y - r));
        }
        PropKind::Signpost => {
            let cx = x + w * 0.5;
            let rect = Rect::new(cx - 0.35 * h, g - 1.08 * h, 0.70 * h, 0.33 * h);
            ink.world
                .push(vec![Vec2::new(cx, g), Vec2::new(cx, rect.bottom())]);
            ink.world.push(rect.outline());
            ink.text = Some(TextBox {
                rect: rect.expanded(-0.04 * h),
                cap: 11.0,
                lines: 2,
                depth: Depth::World,
            });
            ink.bounds = Rect::new(rect.x, rect.y, rect.w, g - rect.y);
        }
        PropKind::Arrow => {
            // At chest height, pointing away from its user.
            let y = g - 0.62 * h;
            let (a, b) = (Vec2::new(x + 0.04 * h, y), Vec2::new(x + w - 0.04 * h, y));
            return match at.user {
                Facing::Left => arrow(a, b),
                Facing::Right => arrow(b, a),
            };
        }
        PropKind::Cloud => {}
    }
    ink
}

/// A laptop open on a surface at height `y`, its base starting at `left`, its screen toward its user.
pub fn laptop(left: f32, y: f32, height: f32, user: Facing) -> PropInk {
    let h = height;
    let base = 0.26 * h;
    let (near, hinge) = match user {
        Facing::Left => (left, left + base),
        Facing::Right => (
            left + 0.34 * h - 0.04 * h,
            left + 0.34 * h - 0.04 * h - base,
        ),
    };
    let back = (hinge - near).signum() * 0.03 * h;
    let screen_top = Vec2::new(hinge + back, y - 0.19 * h);
    let world = vec![
        vec![Vec2::new(near, y), Vec2::new(hinge, y)],
        vec![Vec2::new(hinge, y), screen_top],
        vec![
            Vec2::new(near, y - 0.012 * h),
            Vec2::new(hinge, y - 0.012 * h),
        ],
    ];
    let points: Vec<Vec2> = world.iter().flatten().copied().collect();
    PropInk {
        bounds: Rect::around(&points),
        world,
        ..Default::default()
    }
}

/// An arrow from `from` to `to`, its text above its middle.
pub fn arrow(from: Vec2, to: Vec2) -> PropInk {
    let d = (from - to).normalized();
    let (sin, cos) = small_sin_cos(0.45);
    let head = 13.0;
    let world = vec![
        vec![from, to],
        vec![
            to + d.rotated(sin, cos) * head,
            to,
            to + d.rotated(-sin, cos) * head,
        ],
    ];
    let span = Rect::spanning(from, to);
    let width = span.w.max(120.0);
    let text = TextBox {
        rect: Rect::new(span.center().x - width * 0.5, span.y - 30.0, width, 22.0),
        cap: 12.0,
        lines: 1,
        depth: Depth::World,
    };
    PropInk {
        bounds: span.expanded(head).union(&text.rect),
        world,
        text: Some(text),
        ..Default::default()
    }
}

/// An ellipse's outline pushed outward by a wave of even pieces: `wave(t)` for t across each piece, times `amp` px.
/// The pieces are even along the outline, so a long ellipse has more of them, never bigger ones.
fn waved(center: Vec2, rx: f32, ry: f32, piece: f32, amp: f32, wave: fn(f32) -> f32) -> Polyline {
    let mut dense: Vec<Vec2> = Vec::with_capacity(193);
    for k in 0..48 {
        let (c0, s0) = turn(k);
        let (c1, s1) = turn(k + 1);
        let a = Vec2::new(center.x + rx * c0, center.y + ry * s0);
        let b = Vec2::new(center.x + rx * c1, center.y + ry * s1);
        for i in 0..4 {
            dense.push(a.lerp(b, i as f32 / 4.0));
        }
    }
    dense.push(dense[0]);
    let mut arcs = vec![0.0f32];
    for w in dense.windows(2) {
        let last = arcs[arcs.len() - 1];
        arcs.push(last + (w[1] - w[0]).length());
    }
    let total = arcs[arcs.len() - 1];
    let pieces = (total / piece).round().max(6.0);
    let mut line: Polyline = dense
        .iter()
        .zip(&arcs)
        .map(|(&p, &s)| {
            let u = s * pieces / total;
            let t = u - u.floor();
            p + (p - center).normalized() * (amp * wave(t))
        })
        .collect();
    let first = line[0];
    if let Some(last) = line.last_mut() {
        *last = first;
    }
    line
}

/// A cloud's bump: a parabola, zero at both ends.
fn hump(t: f32) -> f32 {
    4.0 * t * (1.0 - t)
}

/// A burst's spike: a triangle, zero at both ends.
fn spike(t: f32) -> f32 {
    1.0 - (2.0 * t - 1.0).abs()
}

/// The radii of an ellipse around a block, `pad` px wider than it and tall enough to hold its corners.
fn around(block: Rect, pad_x: f32, pad_y: f32) -> (f32, f32) {
    let (hw, hh) = (block.w * 0.5, block.h * 0.5);
    let rx = hw + pad_x;
    let k = (1.0 - (hw / rx) * (hw / rx)).max(0.05);
    (rx, (hh + pad_y).max(hh / k.sqrt() + 4.0))
}

/// A cloud in the sky: its outline over the world, paper inside, its text in the middle.
pub fn cloud(center: Vec2, height: f32) -> PropInk {
    let (rx, ry) = (0.52 * height, 0.22 * height);
    let outline = waved(center, rx, ry, 0.20 * height, 0.06 * height, hump);
    PropInk {
        overlay: vec![outline],
        paper: vec![ellipse(center, rx * 1.02, ry * 1.02)],
        text: Some(TextBox {
            rect: Rect::new(
                center.x - rx * 0.82,
                center.y - ry * 0.62,
                rx * 1.64,
                ry * 1.24,
            ),
            cap: 12.0,
            lines: 2,
            depth: Depth::Overlay,
        }),
        bounds: Rect::new(
            center.x - rx - 0.07 * height,
            center.y - ry - 0.07 * height,
            2.0 * rx + 0.14 * height,
            2.0 * ry + 0.14 * height,
        ),
        ..Default::default()
    }
}

/// A thought's outline around a block of lettering: a cloud whose ellipse holds the block's corners.
pub fn thought_outline(block: Rect) -> (Polyline, Vec<Vec2>, Rect) {
    let c = block.center();
    let (rx, ry) = around(block, 20.0, 14.0);
    let amp = 7.0;
    let outline = waved(c, rx, ry, 30.0, amp, hump);
    (
        outline,
        ellipse(c, rx, ry),
        Rect::new(
            c.x - rx - amp,
            c.y - ry - amp,
            2.0 * (rx + amp),
            2.0 * (ry + amp),
        ),
    )
}

/// A shout's outline around a block of lettering: a burst of even spikes.
pub fn shout_outline(block: Rect) -> (Polyline, Vec<Vec2>, Rect) {
    let c = block.center();
    let (rx, ry) = around(block, 18.0, 12.0);
    let amp = 11.0;
    let outline = waved(c, rx, ry, 24.0, amp, spike);
    (
        outline,
        ellipse(c, rx, ry),
        Rect::new(
            c.x - rx - amp,
            c.y - ry - amp,
            2.0 * (rx + amp),
            2.0 * (ry + amp),
        ),
    )
}

/// How much margin a bubble's outline takes around its lettering, for laying it out before it is drawn.
pub fn outline_margin(block: Rect, thought: bool) -> (f32, f32) {
    let (rx, ry) = if thought {
        around(block, 20.0, 14.0)
    } else {
        around(block, 18.0, 12.0)
    };
    let amp = if thought { 7.0 } else { 11.0 };
    (rx + amp - block.w * 0.5, ry + amp - block.h * 0.5)
}
