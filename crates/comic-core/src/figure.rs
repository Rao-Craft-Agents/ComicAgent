//! Stick figures: a circle for a head and lines for a body, arms and legs, in ten poses. A pose is a skeleton in a unit
//! box (x from −0.5 to 0.5, facing right; y from 0 at the crown to 1 at the feet), scaled to the figure's height and
//! mirrored to face left.

use crate::geom::{Polyline, Rect, Vec2, circle};
use crate::script::{Facing, Pose};

/// A pose as lines in the unit box.
#[derive(Clone, Debug, PartialEq)]
pub struct Skeleton {
    pub head: (f32, f32),
    pub radius: f32,
    pub lines: Vec<Vec<(f32, f32)>>,
    /// Where a held sign is: left, top, right, bottom.
    pub sign: Option<[f32; 4]>,
    /// Where the hands are that rest on a desk, when the pose has them there.
    pub desk_height: Option<f32>,
}

const NECK: (f32, f32) = (0.0, 0.22);
const SHOULDER: (f32, f32) = (0.0, 0.30);
const HIP: (f32, f32) = (0.0, 0.60);
const BODY: [(f32, f32); 2] = [NECK, HIP];
const BACK_ARM: [(f32, f32); 2] = [SHOULDER, (-0.12, 0.58)];

fn legs(spread: f32) -> [Vec<(f32, f32)>; 2] {
    [vec![HIP, (spread, 1.0)], vec![HIP, (-spread, 1.0)]]
}

fn standing(arms: Vec<Vec<(f32, f32)>>, spread: f32) -> Skeleton {
    let mut lines = vec![BODY.to_vec()];
    lines.extend(arms);
    lines.extend(legs(spread));
    Skeleton {
        head: (0.0, 0.11),
        radius: 0.11,
        lines,
        sign: None,
        desk_height: None,
    }
}

/// The skeleton of a pose.
pub fn skeleton(pose: Pose) -> Skeleton {
    match pose {
        Pose::Standing => standing(
            vec![vec![SHOULDER, (0.13, 0.58)], vec![SHOULDER, (-0.13, 0.58)]],
            0.14,
        ),
        Pose::Pointing => standing(vec![vec![SHOULDER, (0.40, 0.26)], BACK_ARM.to_vec()], 0.14),
        Pose::ArmsUp => standing(
            vec![vec![SHOULDER, (0.26, 0.04)], vec![SHOULDER, (-0.26, 0.04)]],
            0.14,
        ),
        Pose::Shrugging => standing(
            vec![
                vec![SHOULDER, (0.17, 0.44), (0.28, 0.31)],
                vec![SHOULDER, (-0.17, 0.44), (-0.28, 0.31)],
            ],
            0.12,
        ),
        Pose::Facepalm => standing(
            vec![
                vec![SHOULDER, (0.17, 0.42), (0.09, 0.18)],
                BACK_ARM.to_vec(),
            ],
            0.14,
        ),
        Pose::Waving => standing(
            vec![
                vec![SHOULDER, (0.20, 0.38), (0.30, 0.14)],
                BACK_ARM.to_vec(),
            ],
            0.14,
        ),
        Pose::HoldingSign => {
            let mut s = standing(
                vec![
                    vec![SHOULDER, (0.12, 0.40), (0.18, 0.36)],
                    vec![SHOULDER, (0.10, 0.50), (0.18, 0.50)],
                ],
                0.14,
            );
            s.sign = Some([0.16, 0.22, 0.86, 0.58]);
            s
        }
        Pose::Walking => Skeleton {
            head: (0.04, 0.11),
            radius: 0.11,
            lines: vec![
                vec![(0.03, 0.22), (0.0, 0.60)],
                vec![(0.02, 0.30), (0.17, 0.53)],
                vec![(0.02, 0.30), (-0.15, 0.53)],
                vec![(0.0, 0.60), (0.13, 0.80), (0.22, 1.0)],
                vec![(0.0, 0.60), (-0.08, 0.80), (-0.20, 1.0)],
            ],
            sign: None,
            desk_height: None,
        },
        Pose::Sitting => Skeleton {
            head: (0.0, 0.31),
            radius: 0.11,
            lines: vec![
                vec![(0.0, 0.42), (0.0, 0.76)],
                vec![(0.0, 0.50), (0.17, 0.60), (0.33, 0.64)],
                vec![(0.0, 0.50), (0.13, 0.62), (0.27, 0.65)],
                // Seen from the side: one leg, the other behind it.
                vec![(0.0, 0.76), (0.23, 0.76), (0.23, 1.0)],
                // The chair: seat, back, two legs.
                vec![(-0.12, 0.79), (0.12, 0.79)],
                vec![(-0.12, 0.79), (-0.13, 0.50)],
                vec![(-0.10, 0.79), (-0.11, 1.0)],
                vec![(0.10, 0.79), (0.10, 1.0)],
            ],
            sign: None,
            desk_height: Some(0.65),
        },
        Pose::LyingDown => Skeleton {
            // On its back along the ground, the head toward the facing side: one leg straight, one knee up, an arm
            // resting on the ground and a hand on the stomach.
            head: (0.36, 0.89),
            radius: 0.11,
            lines: vec![
                vec![(0.25, 0.89), (-0.10, 0.90)],
                vec![(0.20, 0.89), (0.04, 0.985)],
                vec![(0.20, 0.89), (0.07, 0.83), (-0.03, 0.88)],
                vec![(-0.10, 0.90), (-0.48, 0.97)],
                vec![(-0.10, 0.90), (-0.26, 0.74), (-0.40, 0.985)],
            ],
            sign: None,
            desk_height: None,
        },
    }
}

/// How far a pose reaches left and right of its feet, in the unit box, facing right.
pub fn extent(pose: Pose) -> (f32, f32) {
    let s = skeleton(pose);
    let mut lo = s.head.0 - s.radius;
    let mut hi = s.head.0 + s.radius;
    for &(x, _) in s.lines.iter().flatten() {
        lo = lo.min(x);
        hi = hi.max(x);
    }
    if let Some([l, _, r, _]) = s.sign {
        lo = lo.min(l);
        hi = hi.max(r);
    }
    (lo, hi)
}

/// A figure placed: its lines in CSS px (by hand later), its head, its bounds, and the sign it holds.
#[derive(Clone, Debug, PartialEq)]
pub struct Placed {
    pub lines: Vec<Polyline>,
    pub head: Vec2,
    pub radius: f32,
    pub bounds: Rect,
    pub sign: Option<Rect>,
    /// The height a desk's top should be at for this pose's hands, in CSS px from the top.
    pub desk_top: Option<f32>,
}

/// `pose`, `height` tall, its feet at `foot`, facing `facing`.
pub fn place(pose: Pose, facing: Facing, foot: Vec2, height: f32) -> Placed {
    let s = skeleton(pose);
    let dir = facing.sign();
    let at = |(x, y): (f32, f32)| Vec2::new(foot.x + x * dir * height, foot.y + (y - 1.0) * height);
    let head = at(s.head);
    let radius = s.radius * height;
    let mut lines: Vec<Polyline> = vec![circle(head, radius, 24)];
    lines.extend(
        s.lines
            .iter()
            .map(|l| l.iter().map(|&p| at(p)).collect::<Polyline>()),
    );
    let points: Vec<Vec2> = lines.iter().flatten().copied().collect();
    let mut bounds = Rect::around(&points);
    let sign = s
        .sign
        .map(|[l, t, r, b]| Rect::spanning(at((l, t)), at((r, b))));
    if let Some(sign) = sign {
        bounds = bounds.union(&sign);
    }
    Placed {
        lines,
        head,
        radius,
        bounds,
        sign,
        desk_top: s.desk_height.map(|y| foot.y + (y - 1.0) * height),
    }
}
