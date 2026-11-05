//! comic-render: a comic's scene drawn as pixels. Both backends compute the same thing for every pixel: the coverage
//! of each layer's ink (capsules) and paper (convex fills) from their signed distance, composited layer over layer,
//! with the same 16-pixel tiles (`bins`) and the same arithmetic (`sdf` and `comic.wgsl`). The CPU's picture is exact
//! and the same on every machine; the GPU's is within a few levels of it.

pub mod bins;
pub mod compare;
pub mod cpu;
#[cfg(feature = "gpu")]
pub mod gpu;
pub mod png;
pub mod sdf;

use comic_core::{Problem, Scene};
use std::fmt;

pub use cpu::CpuRenderer;
#[cfg(feature = "gpu")]
pub use gpu::GpuRenderer;

/// Draws a scene.
pub trait Renderer {
    /// `gpu` or `cpu`.
    fn name(&self) -> &'static str;
    fn render(&mut self, scene: &Scene, options: &RenderOptions) -> Result<Raster, RenderError>;
}

/// The colours a scene is drawn in.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RenderOptions {
    /// What is under everything, premultiplied RGBA.
    pub clear: [f32; 4],
    pub paper: [f32; 3],
    pub ink: [f32; 3],
}

impl RenderOptions {
    /// A strip: black ink on white paper, all of it opaque.
    pub fn strip() -> RenderOptions {
        RenderOptions {
            clear: [1.0, 1.0, 1.0, 1.0],
            paper: [1.0, 1.0, 1.0],
            ink: [0.0, 0.0, 0.0],
        }
    }

    /// Ink and paper on nothing: an icon.
    pub fn transparent() -> RenderOptions {
        RenderOptions {
            clear: [0.0, 0.0, 0.0, 0.0],
            paper: [1.0, 1.0, 1.0],
            ink: [0.0, 0.0, 0.0],
        }
    }
}

/// Pixels: rows top to bottom, RGBA, eight bits a channel, not premultiplied.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Raster {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

impl Raster {
    /// FNV-1a of the pixels: a golden test's fingerprint.
    pub fn hash(&self) -> u64 {
        comic_core::rng::hash_bytes(&self.rgba)
    }

    /// Whether every pixel is opaque.
    pub fn is_opaque(&self) -> bool {
        self.rgba.chunks_exact(4).all(|p| p[3] == 255)
    }
}

/// Which renderer to draw with.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Choice {
    Gpu,
    Cpu,
    /// The GPU when there is one, else the CPU.
    Auto,
}

impl Choice {
    pub fn parse(word: &str) -> Option<Choice> {
        match word.trim().to_lowercase().as_str() {
            "gpu" => Some(Choice::Gpu),
            "cpu" => Some(Choice::Cpu),
            "auto" => Some(Choice::Auto),
            _ => None,
        }
    }

    /// The call's choice, else `COMIC_RENDERER`, else `Auto`.
    pub fn from_env_or(input: Option<&str>) -> Choice {
        input
            .and_then(Choice::parse)
            .or_else(|| {
                std::env::var("COMIC_RENDERER")
                    .ok()
                    .as_deref()
                    .and_then(Choice::parse)
            })
            .unwrap_or(Choice::Auto)
    }
}

/// Why a scene was not drawn.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RenderError {
    /// The call asked for the GPU and there is none.
    NoGpu(String),
    /// The GPU failed while drawing.
    Gpu(String),
    /// Larger than a renderer draws.
    TooLarge { width: u32, height: u32 },
}

impl fmt::Display for RenderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RenderError::NoGpu(reason) => write!(f, "no GPU to draw on: {reason}"),
            RenderError::Gpu(reason) => write!(f, "the GPU failed: {reason}"),
            RenderError::TooLarge { width, height } => {
                write!(
                    f,
                    "a {width} × {height} picture is larger than {MAX_SIDE} pixels a side"
                )
            }
        }
    }
}

impl std::error::Error for RenderError {}

/// The longest side drawn.
pub const MAX_SIDE: u32 = 8192;

pub(crate) fn check_size(scene: &Scene) -> Result<(), RenderError> {
    if scene.width == 0 || scene.height == 0 || scene.width > MAX_SIDE || scene.height > MAX_SIDE {
        return Err(RenderError::TooLarge {
            width: scene.width,
            height: scene.height,
        });
    }
    Ok(())
}

/// The renderer `choice` asks for, and a problem when it fell back to the CPU.
pub fn choose(choice: Choice) -> Result<(Box<dyn Renderer + Send>, Option<Problem>), RenderError> {
    match choice {
        Choice::Cpu => Ok((Box::new(CpuRenderer::new()), None)),
        Choice::Gpu => gpu_renderer().map(|r| (r, None)),
        Choice::Auto => match gpu_renderer() {
            Ok(renderer) => Ok((renderer, None)),
            Err(_) => Ok((
                Box::new(CpuRenderer::new()),
                Some(Problem::fallback(
                    "drawn on the CPU: this host has no GPU; the picture is the same",
                )),
            )),
        },
    }
}

#[cfg(feature = "gpu")]
fn gpu_renderer() -> Result<Box<dyn Renderer + Send>, RenderError> {
    GpuRenderer::new().map(|r| Box::new(r) as Box<dyn Renderer + Send>)
}

#[cfg(not(feature = "gpu"))]
fn gpu_renderer() -> Result<Box<dyn Renderer + Send>, RenderError> {
    Err(RenderError::NoGpu(
        "this build of comic has no GPU path".into(),
    ))
}

/// Premultiplied pixels made straight: what a PNG holds. Opaque pixels are unchanged.
pub(crate) fn unpremultiply(rgba: &mut [u8]) {
    for p in rgba.chunks_exact_mut(4) {
        let a = u32::from(p[3]);
        if a == 255 {
            continue;
        }
        if a == 0 {
            p[0] = 0;
            p[1] = 0;
            p[2] = 0;
            continue;
        }
        for c in &mut p[..3] {
            *c = ((u32::from(*c) * 255 + a / 2) / a).min(255) as u8;
        }
    }
}
