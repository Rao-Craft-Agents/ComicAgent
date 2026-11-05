//! The arithmetic of one pixel, as the shader does it, in the same order: a capsule's signed distance, a convex fill's,
//! the coverage a distance gives (a one-pixel ramp: the anti-aliasing), the layers composited, the colour packed.

use crate::RenderOptions;
use crate::bins::{Bins, GpuCapsule, GpuEdge};

/// The signed distance from (x, y) to a capsule: negative inside.
#[inline]
pub fn sd_capsule(x: f32, y: f32, c: &GpuCapsule) -> f32 {
    let (pax, pay) = (x - c.a[0], y - c.a[1]);
    let (bax, bay) = (c.b[0] - c.a[0], c.b[1] - c.a[1]);
    let h = ((pax * bax + pay * bay) / (bax * bax + bay * bay).max(1e-6)).clamp(0.0, 1.0);
    let (dx, dy) = (pax - bax * h, pay - bay * h);
    (dx * dx + dy * dy).sqrt() - c.r
}

/// The signed distance from (x, y) to a convex polygon given by its edges: the farthest of its half-planes.
#[inline]
pub fn sd_convex(x: f32, y: f32, edges: &[GpuEdge]) -> f32 {
    edges.iter().fold(-1e9f32, |d, e| {
        d.max((x - e.p[0]) * e.n[0] + (y - e.p[1]) * e.n[1])
    })
}

/// How much of a pixel a distance covers.
#[inline]
pub fn coverage(sd: f32) -> f32 {
    (0.5 - sd).clamp(0.0, 1.0)
}

/// The premultiplied colour of the pixel whose centre is (x, y), in tile (tx, ty).
pub fn shade(bins: &Bins, tx: u32, ty: u32, x: f32, y: f32, options: &RenderOptions) -> [f32; 4] {
    let mut color = options.clear;
    for layer in 0..bins.layers {
        let tile = bins.tile(layer, tx, ty);
        if tile.capsule_count == 0 && tile.fill_count == 0 {
            continue;
        }
        let mut ink = 0.0f32;
        for &i in &bins.items[tile.capsules as usize..(tile.capsules + tile.capsule_count) as usize]
        {
            ink = ink.max(coverage(sd_capsule(x, y, &bins.capsules[i as usize])));
        }
        let mut fill = 0.0f32;
        for &i in &bins.items[tile.fills as usize..(tile.fills + tile.fill_count) as usize] {
            let f = &bins.fills[i as usize];
            fill = fill.max(coverage(sd_convex(
                x,
                y,
                &bins.edges[f.start as usize..(f.start + f.count) as usize],
            )));
        }
        let a = ink + fill * (1.0 - ink);
        let under = 1.0 - a;
        for ((c, ink_c), paper_c) in color.iter_mut().zip(options.ink).zip(options.paper) {
            *c = ink_c * ink + paper_c * (fill * (1.0 - ink)) + *c * under;
        }
        color[3] = a + color[3] * under;
    }
    color
}

/// A colour as eight bits a channel: round half up.
#[inline]
pub fn pack(color: [f32; 4]) -> [u8; 4] {
    color.map(|v| (v.clamp(0.0, 1.0) * 255.0 + 0.5).floor() as u8)
}
