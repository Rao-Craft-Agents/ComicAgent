//! The CPU's picture: every pixel shaded by `sdf`, tile rows shared among the machine's cores. Exact, and the same on
//! every machine, so it is what golden tests hold and what draws where there is no GPU.

use crate::bins::{TILE, bin};
use crate::sdf::{pack, shade};
use crate::{Raster, RenderError, RenderOptions, Renderer, check_size, unpremultiply};
use comic_core::Scene;
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

/// Draws on the CPU.
#[derive(Clone, Debug, Default)]
pub struct CpuRenderer {
    /// How many threads; the machine's cores when zero.
    pub threads: usize,
}

impl CpuRenderer {
    pub fn new() -> CpuRenderer {
        CpuRenderer { threads: 0 }
    }
}

impl Renderer for CpuRenderer {
    fn name(&self) -> &'static str {
        "cpu"
    }

    fn render(&mut self, scene: &Scene, options: &RenderOptions) -> Result<Raster, RenderError> {
        check_size(scene)?;
        let bins = bin(scene);
        let (width, height) = (scene.width as usize, scene.height as usize);
        let mut rgba = vec![0u8; width * height * 4];
        let band = width * 4 * TILE as usize;
        let rows: Vec<Mutex<(usize, &mut [u8])>> =
            rgba.chunks_mut(band).enumerate().map(Mutex::new).collect();
        let next = AtomicUsize::new(0);
        let threads = if self.threads > 0 {
            self.threads
        } else {
            std::thread::available_parallelism()
                .map(|n| n.get())
                .unwrap_or(4)
        };
        std::thread::scope(|s| {
            for _ in 0..threads.min(rows.len()).max(1) {
                s.spawn(|| {
                    loop {
                        let k = next.fetch_add(1, Ordering::Relaxed);
                        let Some(row) = rows.get(k) else { break };
                        let mut guard = row.lock().unwrap_or_else(|e| e.into_inner());
                        let (ty, pixels) = &mut *guard;
                        let ty = *ty as u32;
                        let lines = pixels.len() / (width * 4);
                        for line in 0..lines {
                            let y = ty * TILE + line as u32;
                            for x in 0..width as u32 {
                                let color = shade(
                                    &bins,
                                    x / TILE,
                                    ty,
                                    x as f32 + 0.5,
                                    y as f32 + 0.5,
                                    options,
                                );
                                let at = (line * width + x as usize) * 4;
                                pixels[at..at + 4].copy_from_slice(&pack(color));
                            }
                        }
                    }
                });
            }
        });
        drop(rows);
        unpremultiply(&mut rgba);
        Ok(Raster {
            width: scene.width,
            height: scene.height,
            rgba,
        })
    }
}
