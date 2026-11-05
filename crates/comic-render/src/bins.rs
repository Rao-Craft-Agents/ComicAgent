//! A scene sorted into 16-pixel tiles, layer by layer: each tile lists the capsules and fills that can touch it, so a
//! pixel looks at a few dozen strokes, not thousands. The GPU dispatches one workgroup per tile; the CPU walks the same
//! lists. A fill's edges carry their outward normals, worked out here so neither backend normalizes anything.

use bytemuck::{Pod, Zeroable};
use comic_core::Scene;

/// Tile size in pixels; the shader's workgroup size.
pub const TILE: u32 = 16;

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable)]
pub struct GpuCapsule {
    pub a: [f32; 2],
    pub b: [f32; 2],
    pub r: f32,
    pub pad: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable)]
pub struct GpuFill {
    pub start: u32,
    pub count: u32,
    pub pad0: u32,
    pub pad1: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable)]
pub struct GpuEdge {
    pub p: [f32; 2],
    pub n: [f32; 2],
}

/// A tile's lists in `items`: its capsules, then its fills.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable)]
pub struct GpuTile {
    pub capsules: u32,
    pub capsule_count: u32,
    pub fills: u32,
    pub fill_count: u32,
}

/// The scene, binned.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Bins {
    pub width: u32,
    pub height: u32,
    pub tiles_x: u32,
    pub tiles_y: u32,
    pub layers: u32,
    pub capsules: Vec<GpuCapsule>,
    pub fills: Vec<GpuFill>,
    pub edges: Vec<GpuEdge>,
    /// `layers` × `tiles_x` × `tiles_y`, layer first.
    pub tiles: Vec<GpuTile>,
    pub items: Vec<u32>,
}

impl Bins {
    pub fn tile_count(&self) -> u32 {
        self.tiles_x * self.tiles_y
    }

    pub fn tile(&self, layer: u32, tx: u32, ty: u32) -> &GpuTile {
        &self.tiles[(layer * self.tile_count() + ty * self.tiles_x + tx) as usize]
    }
}

/// A run of tiles: first column, first row, last column, last row.
type Span = (u32, u32, u32, u32);
/// Each capsule or fill of a layer, by its index, and the tiles it touches.
type Spans = Vec<(u32, Span)>;

/// The tiles a box (in pixels) touches, clamped to the picture.
fn tile_span(min: [f32; 2], max: [f32; 2], tiles_x: u32, tiles_y: u32) -> Option<Span> {
    let t = TILE as f32;
    if max[0] < 0.0 || max[1] < 0.0 {
        return None;
    }
    let x0 = (min[0].max(0.0) / t) as u32;
    let y0 = (min[1].max(0.0) / t) as u32;
    let x1 = ((max[0] / t) as u32).min(tiles_x - 1);
    let y1 = ((max[1] / t) as u32).min(tiles_y - 1);
    (x0 <= x1 && y0 <= y1).then_some((x0, y0, x1, y1))
}

/// Bins `scene`.
pub fn bin(scene: &Scene) -> Bins {
    let tiles_x = scene.width.div_ceil(TILE).max(1);
    let tiles_y = scene.height.div_ceil(TILE).max(1);
    let tile_count = (tiles_x * tiles_y) as usize;
    let layers = scene.layers.len() as u32;
    let mut bins = Bins {
        width: scene.width,
        height: scene.height,
        tiles_x,
        tiles_y,
        layers,
        ..Default::default()
    };
    // Every capsule and fill, and the tiles each touches.
    let mut capsule_spans: Vec<Spans> = Vec::new();
    let mut fill_spans: Vec<Spans> = Vec::new();
    for layer in &scene.layers {
        let mut spans = Vec::new();
        for c in &layer.capsules {
            let index = bins.capsules.len() as u32;
            bins.capsules.push(GpuCapsule {
                a: [c.a.x, c.a.y],
                b: [c.b.x, c.b.y],
                r: c.r,
                pad: 0.0,
            });
            let m = c.r + 1.0;
            let span = tile_span(
                [c.a.x.min(c.b.x) - m, c.a.y.min(c.b.y) - m],
                [c.a.x.max(c.b.x) + m, c.a.y.max(c.b.y) + m],
                tiles_x,
                tiles_y,
            );
            if let Some(span) = span {
                spans.push((index, span));
            }
        }
        capsule_spans.push(spans);
        let mut spans = Vec::new();
        for f in &layer.fills {
            let n = f.points.len();
            if n < 3 {
                continue;
            }
            let centroid = f
                .points
                .iter()
                .fold((0.0f32, 0.0f32), |acc, p| (acc.0 + p.x, acc.1 + p.y));
            let centroid = (centroid.0 / n as f32, centroid.1 / n as f32);
            let start = bins.edges.len() as u32;
            for k in 0..n {
                let (p, q) = (f.points[k], f.points[(k + 1) % n]);
                let (dx, dy) = (q.x - p.x, q.y - p.y);
                let length = (dx * dx + dy * dy).sqrt();
                if length < 1e-4 {
                    continue;
                }
                let mut normal = [-dy / length, dx / length];
                if (centroid.0 - p.x) * normal[0] + (centroid.1 - p.y) * normal[1] > 0.0 {
                    normal = [-normal[0], -normal[1]];
                }
                bins.edges.push(GpuEdge {
                    p: [p.x, p.y],
                    n: normal,
                });
            }
            let count = bins.edges.len() as u32 - start;
            let index = bins.fills.len() as u32;
            bins.fills.push(GpuFill {
                start,
                count,
                pad0: 0,
                pad1: 0,
            });
            let (mut lo, mut hi) = ([f32::MAX; 2], [f32::MIN; 2]);
            for p in &f.points {
                lo = [lo[0].min(p.x), lo[1].min(p.y)];
                hi = [hi[0].max(p.x), hi[1].max(p.y)];
            }
            if let Some(span) = tile_span(
                [lo[0] - 1.0, lo[1] - 1.0],
                [hi[0] + 1.0, hi[1] + 1.0],
                tiles_x,
                tiles_y,
            ) {
                spans.push((index, span));
            }
        }
        fill_spans.push(spans);
    }
    // Count, then lay each tile's lists out side by side.
    let mut counts = vec![(0u32, 0u32); tile_count * layers as usize];
    let each = |spans: &Spans, f: &mut dyn FnMut(usize, u32)| {
        for &(index, (x0, y0, x1, y1)) in spans {
            for ty in y0..=y1 {
                for tx in x0..=x1 {
                    f((ty * tiles_x + tx) as usize, index);
                }
            }
        }
    };
    for layer in 0..layers as usize {
        let base = layer * tile_count;
        each(&capsule_spans[layer], &mut |t, _| counts[base + t].0 += 1);
        each(&fill_spans[layer], &mut |t, _| counts[base + t].1 += 1);
    }
    let mut offset = 0u32;
    bins.tiles = counts
        .iter()
        .map(|&(c, f)| {
            let tile = GpuTile {
                capsules: offset,
                capsule_count: c,
                fills: offset + c,
                fill_count: f,
            };
            offset += c + f;
            tile
        })
        .collect();
    bins.items = vec![0; offset as usize];
    let mut cursor: Vec<(u32, u32)> = bins.tiles.iter().map(|t| (t.capsules, t.fills)).collect();
    for layer in 0..layers as usize {
        let base = layer * tile_count;
        let items = &mut bins.items;
        each(&capsule_spans[layer], &mut |t, index| {
            let at = &mut cursor[base + t].0;
            items[*at as usize] = index;
            *at += 1;
        });
        each(&fill_spans[layer], &mut |t, index| {
            let at = &mut cursor[base + t].1;
            items[*at as usize] = index;
            *at += 1;
        });
    }
    bins
}
