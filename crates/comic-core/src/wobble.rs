//! The hand: a line drawn by a person is never straight. A polyline is cut into short pieces and each point is pushed
//! sideways by a smooth noise, zero at the line's two ends so that joints still meet.

use crate::geom::{Polyline, Vec2};
use crate::rng::SplitMix64;

/// How far (CSS px) and how slowly a line wanders.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Wobble {
    /// The largest sideways push.
    pub amplitude: f32,
    /// The distance between two independent pushes.
    pub step: f32,
}

impl Wobble {
    /// Figures and props.
    pub const INK: Wobble = Wobble {
        amplitude: 0.9,
        step: 14.0,
    };
    /// Panel borders: longer, steadier lines.
    pub const BORDER: Wobble = Wobble {
        amplitude: 0.8,
        step: 40.0,
    };
    /// No hand at all.
    pub const NONE: Wobble = Wobble {
        amplitude: 0.0,
        step: 1.0,
    };

    /// `line`, drawn by hand.
    pub fn apply(&self, line: &[Vec2], rng: &mut SplitMix64) -> Polyline {
        if line.len() < 2 || self.amplitude == 0.0 {
            return line.to_vec();
        }
        // Resample: no piece longer than half a step, each point keeping the direction of its segment.
        let piece = self.step * 0.5;
        let mut points: Vec<(Vec2, Vec2, f32)> = Vec::new();
        let mut arc = 0.0;
        for pair in line.windows(2) {
            let (a, b) = (pair[0], pair[1]);
            let d = b - a;
            let length = d.length();
            let normal = d.normalized().perp();
            let pieces = ((length / piece).ceil() as usize).max(1);
            for i in 0..pieces {
                let t = i as f32 / pieces as f32;
                points.push((a.lerp(b, t), normal, arc + length * t));
            }
            arc += length;
        }
        let last = line[line.len() - 1];
        let end_normal = points.last().map(|p| p.1).unwrap_or(Vec2::ZERO);
        points.push((last, end_normal, arc));
        // Knots one step apart, smoothly joined; tapered to nothing at both ends.
        let knots: Vec<f32> = (0..(arc / self.step) as usize + 3)
            .map(|_| rng.signed() * self.amplitude)
            .collect();
        let taper = (arc / 3.0).min(self.step);
        points
            .into_iter()
            .map(|(p, normal, s)| {
                let u = s / self.step;
                let j = (u as usize).min(knots.len() - 2);
                let t = u - j as f32;
                let t = t * t * (3.0 - 2.0 * t);
                let push = knots[j] + (knots[j + 1] - knots[j]) * t;
                let weight = if taper > 0.0 {
                    (s / taper).min((arc - s) / taper).clamp(0.0, 1.0)
                } else {
                    0.0
                };
                p + normal * (push * weight)
            })
            .collect()
    }
}
