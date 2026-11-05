//! How far apart two pictures are: the largest difference in any channel, and how many pixels differ by more than a
//! tolerance. The GPU's picture is held to the CPU's this way.

use crate::Raster;

/// Two pictures compared.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Diff {
    /// The largest difference in any channel of any pixel.
    pub max: u8,
    /// Pixels with a channel differing by more than the tolerance.
    pub over: usize,
    pub total: usize,
}

impl Diff {
    /// The share of pixels over the tolerance.
    pub fn share(&self) -> f64 {
        if self.total == 0 {
            0.0
        } else {
            self.over as f64 / self.total as f64
        }
    }
}

/// `a` against `b`; an error when their sizes differ.
pub fn compare(a: &Raster, b: &Raster, tolerance: u8) -> Result<Diff, String> {
    if (a.width, a.height) != (b.width, b.height) || a.rgba.len() != b.rgba.len() {
        return Err(format!(
            "the pictures differ in size: {} × {} and {} × {}",
            a.width, a.height, b.width, b.height
        ));
    }
    let mut diff = Diff {
        max: 0,
        over: 0,
        total: (a.width * a.height) as usize,
    };
    for (p, q) in a.rgba.chunks_exact(4).zip(b.rgba.chunks_exact(4)) {
        let worst = p
            .iter()
            .zip(q)
            .map(|(x, y)| x.abs_diff(*y))
            .max()
            .unwrap_or(0);
        diff.max = diff.max.max(worst);
        if worst > tolerance {
            diff.over += 1;
        }
    }
    Ok(diff)
}
