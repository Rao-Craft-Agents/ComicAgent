//! What a renderer draws: ink as capsules (a segment with a radius) and paper as convex fills, in device pixels, in
//! layers drawn in order. Within a layer, paper never hides ink; a later layer hides an earlier one where it has paper
//! or ink. `Ink` collects strokes in CSS px as the layout makes them and becomes a scene at the end.

use crate::geom::{Polyline, Vec2, quantize};

/// A line segment with a radius: half the stroke's width.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Capsule {
    pub a: Vec2,
    pub b: Vec2,
    pub r: f32,
}

/// A convex polygon filled with paper, its points in order around it.
#[derive(Clone, Debug, PartialEq)]
pub struct ConvexFill {
    pub points: Vec<Vec2>,
}

/// One layer of ink and paper.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Layer {
    pub capsules: Vec<Capsule>,
    pub fills: Vec<ConvexFill>,
}

/// A picture to draw, in device pixels.
#[derive(Clone, Debug, PartialEq)]
pub struct Scene {
    pub width: u32,
    pub height: u32,
    pub layers: Vec<Layer>,
}

impl Scene {
    pub fn capsule_count(&self) -> usize {
        self.layers.iter().map(|l| l.capsules.len()).sum()
    }
}

/// Which layer a stroke goes in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Depth {
    /// Borders, figures, props.
    World,
    /// What is drawn over the world: bubbles, captions, held signs, clouds.
    Overlay,
}

/// Strokes and fills in CSS px, as the layout draws them.
#[derive(Clone, Debug, Default)]
pub struct Ink {
    strokes: [Vec<(Polyline, f32)>; 2],
    fills: [Vec<Vec<Vec2>>; 2],
}

fn index(depth: Depth) -> usize {
    match depth {
        Depth::World => 0,
        Depth::Overlay => 1,
    }
}

impl Ink {
    /// A line `width` CSS px wide.
    pub fn stroke(&mut self, depth: Depth, line: Polyline, width: f32) {
        if line.len() >= 2 {
            self.strokes[index(depth)].push((line, width));
        }
    }

    /// Every line, `width` wide.
    pub fn strokes(&mut self, depth: Depth, lines: impl IntoIterator<Item = Polyline>, width: f32) {
        for line in lines {
            self.stroke(depth, line, width);
        }
    }

    /// A convex polygon of paper.
    pub fn fill(&mut self, depth: Depth, points: Vec<Vec2>) {
        if points.len() >= 3 {
            self.fills[index(depth)].push(points);
        }
    }

    /// The scene, `scale` device pixels to the CSS pixel, every coordinate to a sixteenth of a pixel.
    pub fn into_scene(self, width: u32, height: u32, scale: f32) -> Scene {
        let at = |p: Vec2| Vec2::new(quantize(p.x * scale), quantize(p.y * scale));
        let [world_strokes, overlay_strokes] = self.strokes;
        let [world_fills, overlay_fills] = self.fills;
        let layer = |strokes: Vec<(Polyline, f32)>, fills: Vec<Vec<Vec2>>| Layer {
            capsules: strokes
                .into_iter()
                .flat_map(|(line, width)| {
                    let r = quantize(width * scale * 0.5);
                    line.windows(2)
                        .map(|w| Capsule {
                            a: at(w[0]),
                            b: at(w[1]),
                            r,
                        })
                        .collect::<Vec<_>>()
                })
                .collect(),
            fills: fills
                .into_iter()
                .map(|points| ConvexFill {
                    points: points.into_iter().map(at).collect(),
                })
                .collect(),
        };
        Scene {
            width,
            height,
            layers: vec![
                layer(world_strokes, world_fills),
                layer(overlay_strokes, overlay_fills),
            ],
        }
    }
}
