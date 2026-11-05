//! comic-core: the `craft-comic/1` script, and everything that turns one into ink. A script is checked (findings the
//! model fixes), clamped to what a strip holds (problems the answer reports), then composed: laid out in CSS px and
//! inked by hand — Hershey lettering, stick figures, props, a seeded wobble — into a scene of capsules and paper fills
//! in device pixels, which `comic-render` draws on the GPU or the CPU.
//!
//! Nothing here touches a GPU, a file or a clock, and every number comes from IEEE adds, multiplies, divides and square
//! roots, so a script and a seed give the same scene on every machine.

pub mod compose;
pub mod figure;
pub mod geom;
pub mod hershey;
pub mod layout;
pub mod lettering;
pub mod props;
pub mod rng;
pub mod scene;
pub mod script;
pub mod spec_png;
pub mod wobble;

pub use compose::{Composed, compose};
pub use layout::Style;
pub use scene::{Capsule, ConvexFill, Layer, Scene};
pub use script::{Finding, FindingKind, Problem, Script};

/// The script standard's id.
pub const SCHEMA: &str = "craft-comic/1";
