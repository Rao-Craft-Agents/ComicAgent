//! Points, rectangles and polylines. Layout works in CSS pixels; the scene holds device pixels (CSS × the style's
//! scale). Every number here comes from adds, multiplies, divides and square roots, so a script and a seed draw the
//! same picture on every machine: the circle is a table of literals, and a rotation's sine is a short polynomial.

use std::f64::consts::FRAC_1_SQRT_2;
use std::ops::{Add, AddAssign, Mul, Neg, Sub};

/// A point or a direction.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

impl Vec2 {
    pub const ZERO: Vec2 = Vec2 { x: 0.0, y: 0.0 };

    pub const fn new(x: f32, y: f32) -> Vec2 {
        Vec2 { x, y }
    }

    pub fn dot(self, other: Vec2) -> f32 {
        self.x * other.x + self.y * other.y
    }

    pub fn length(self) -> f32 {
        self.dot(self).sqrt()
    }

    /// A quarter turn: (x, y) → (−y, x).
    pub fn perp(self) -> Vec2 {
        Vec2::new(-self.y, self.x)
    }

    /// Unit length, or zero for a zero vector.
    pub fn normalized(self) -> Vec2 {
        let length = self.length();
        if length > 1e-6 {
            self * (1.0 / length)
        } else {
            Vec2::ZERO
        }
    }

    pub fn lerp(self, other: Vec2, t: f32) -> Vec2 {
        self + (other - self) * t
    }

    /// Turned about the origin by an angle given as its sine and cosine.
    pub fn rotated(self, sin: f32, cos: f32) -> Vec2 {
        Vec2::new(self.x * cos - self.y * sin, self.x * sin + self.y * cos)
    }
}

impl Add for Vec2 {
    type Output = Vec2;
    fn add(self, other: Vec2) -> Vec2 {
        Vec2::new(self.x + other.x, self.y + other.y)
    }
}

impl AddAssign for Vec2 {
    fn add_assign(&mut self, other: Vec2) {
        self.x += other.x;
        self.y += other.y;
    }
}

impl Sub for Vec2 {
    type Output = Vec2;
    fn sub(self, other: Vec2) -> Vec2 {
        Vec2::new(self.x - other.x, self.y - other.y)
    }
}

impl Mul<f32> for Vec2 {
    type Output = Vec2;
    fn mul(self, k: f32) -> Vec2 {
        Vec2::new(self.x * k, self.y * k)
    }
}

impl Neg for Vec2 {
    type Output = Vec2;
    fn neg(self) -> Vec2 {
        Vec2::new(-self.x, -self.y)
    }
}

/// An axis-aligned rectangle: its top-left corner and its size.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Rect {
    pub const fn new(x: f32, y: f32, w: f32, h: f32) -> Rect {
        Rect { x, y, w, h }
    }

    /// The rectangle two corners span, in either order.
    pub fn spanning(a: Vec2, b: Vec2) -> Rect {
        Rect::new(
            a.x.min(b.x),
            a.y.min(b.y),
            (a.x - b.x).abs(),
            (a.y - b.y).abs(),
        )
    }

    /// The smallest rectangle holding every point; empty at the origin when there are none.
    pub fn around(points: &[Vec2]) -> Rect {
        let Some(first) = points.first() else {
            return Rect::default();
        };
        let (mut min, mut max) = (*first, *first);
        for p in points {
            min = Vec2::new(min.x.min(p.x), min.y.min(p.y));
            max = Vec2::new(max.x.max(p.x), max.y.max(p.y));
        }
        Rect::spanning(min, max)
    }

    pub fn right(&self) -> f32 {
        self.x + self.w
    }

    pub fn bottom(&self) -> f32 {
        self.y + self.h
    }

    pub fn center(&self) -> Vec2 {
        Vec2::new(self.x + self.w * 0.5, self.y + self.h * 0.5)
    }

    /// Grown by `d` on every side (shrunk when negative).
    pub fn expanded(&self, d: f32) -> Rect {
        Rect::new(self.x - d, self.y - d, self.w + 2.0 * d, self.h + 2.0 * d)
    }

    pub fn union(&self, other: &Rect) -> Rect {
        let x = self.x.min(other.x);
        let y = self.y.min(other.y);
        Rect::new(
            x,
            y,
            self.right().max(other.right()) - x,
            self.bottom().max(other.bottom()) - y,
        )
    }

    /// Whether the two share any area (touching edges do not count).
    pub fn intersects(&self, other: &Rect) -> bool {
        self.x < other.right()
            && other.x < self.right()
            && self.y < other.bottom()
            && other.y < self.bottom()
    }

    /// Whether the two overlap left to right, whatever their heights.
    pub fn overlaps_horizontally(&self, other: &Rect) -> bool {
        self.x < other.right() && other.x < self.right()
    }

    /// Whether `other` lies wholly inside.
    pub fn holds(&self, other: &Rect) -> bool {
        other.x >= self.x
            && other.y >= self.y
            && other.right() <= self.right()
            && other.bottom() <= self.bottom()
    }

    /// The four corners, clockwise from the top left on screen.
    pub fn corners(&self) -> [Vec2; 4] {
        [
            Vec2::new(self.x, self.y),
            Vec2::new(self.right(), self.y),
            Vec2::new(self.right(), self.bottom()),
            Vec2::new(self.x, self.bottom()),
        ]
    }

    /// The outline as a closed polyline.
    pub fn outline(&self) -> Polyline {
        let c = self.corners();
        vec![c[0], c[1], c[2], c[3], c[0]]
    }
}

/// A line drawn with one stroke of the pen.
pub type Polyline = Vec<Vec2>;

/// The unit circle at 48 points, counter-clockwise from +x as (cos, sin), every 7.5°.
pub const CIRCLE48: [(f64, f64); 48] = [
    (1.000000000, 0.0),
    (0.991444861, 0.130526192),
    (0.965925826, 0.258819045),
    (0.923879533, 0.382683432),
    (0.866025404, 0.500000000),
    (0.793353340, 0.608761429),
    (FRAC_1_SQRT_2, FRAC_1_SQRT_2),
    (0.608761429, 0.793353340),
    (0.500000000, 0.866025404),
    (0.382683432, 0.923879533),
    (0.258819045, 0.965925826),
    (0.130526192, 0.991444861),
    (0.0, 1.000000000),
    (-0.130526192, 0.991444861),
    (-0.258819045, 0.965925826),
    (-0.382683432, 0.923879533),
    (-0.500000000, 0.866025404),
    (-0.608761429, 0.793353340),
    (-FRAC_1_SQRT_2, FRAC_1_SQRT_2),
    (-0.793353340, 0.608761429),
    (-0.866025404, 0.500000000),
    (-0.923879533, 0.382683432),
    (-0.965925826, 0.258819045),
    (-0.991444861, 0.130526192),
    (-1.000000000, 0.0),
    (-0.991444861, -0.130526192),
    (-0.965925826, -0.258819045),
    (-0.923879533, -0.382683432),
    (-0.866025404, -0.500000000),
    (-0.793353340, -0.608761429),
    (-FRAC_1_SQRT_2, -FRAC_1_SQRT_2),
    (-0.608761429, -0.793353340),
    (-0.500000000, -0.866025404),
    (-0.382683432, -0.923879533),
    (-0.258819045, -0.965925826),
    (-0.130526192, -0.991444861),
    (0.0, -1.000000000),
    (0.130526192, -0.991444861),
    (0.258819045, -0.965925826),
    (0.382683432, -0.923879533),
    (0.500000000, -0.866025404),
    (0.608761429, -0.793353340),
    (FRAC_1_SQRT_2, -FRAC_1_SQRT_2),
    (0.793353340, -0.608761429),
    (0.866025404, -0.500000000),
    (0.923879533, -0.382683432),
    (0.965925826, -0.258819045),
    (0.991444861, -0.130526192),
];

/// The cosine and sine of the angle `k` × 7.5°, any `k`.
pub fn turn(k: i32) -> (f32, f32) {
    let (c, s) = CIRCLE48[k.rem_euclid(48) as usize];
    (c as f32, s as f32)
}

/// A circle as a closed polyline of `points` (a divisor of 48: 12, 16, 24 or 48).
pub fn circle(center: Vec2, radius: f32, points: usize) -> Polyline {
    let step = (48 / points.clamp(1, 48)) as i32;
    let mut line: Polyline = (0..48 / step)
        .map(|k| {
            let (c, s) = turn(k * step);
            Vec2::new(center.x + radius * c, center.y + radius * s)
        })
        .collect();
    line.push(line[0]);
    line
}

/// An ellipse as a closed polygon of 24 points (no closing point): for fills.
pub fn ellipse(center: Vec2, rx: f32, ry: f32) -> Vec<Vec2> {
    (0..24)
        .map(|k| {
            let (c, s) = turn(2 * k);
            Vec2::new(center.x + rx * c, center.y + ry * s)
        })
        .collect()
}

/// The sine and cosine of a small angle (|a| ≤ 0.6 rad) by Taylor polynomials: the same bits on every machine.
pub fn small_sin_cos(a: f32) -> (f32, f32) {
    let a2 = a * a;
    let sin = a * (1.0 - a2 / 6.0 * (1.0 - a2 / 20.0 * (1.0 - a2 / 42.0)));
    let cos = 1.0 - a2 / 2.0 * (1.0 - a2 / 12.0 * (1.0 - a2 / 30.0));
    (sin, cos)
}

/// To the nearest sixteenth of a pixel, so a coordinate means the same thing to every renderer.
pub fn quantize(v: f32) -> f32 {
    (v * 16.0).round() / 16.0
}

/// Cubic Bézier from `a` to `d` with controls `b`, `c`, sampled at `n` + 1 points.
pub fn bezier(a: Vec2, b: Vec2, c: Vec2, d: Vec2, n: usize) -> Polyline {
    (0..=n)
        .map(|i| {
            let t = i as f32 / n as f32;
            let u = 1.0 - t;
            a * (u * u * u) + b * (3.0 * u * u * t) + c * (3.0 * u * t * t) + d * (t * t * t)
        })
        .collect()
}
