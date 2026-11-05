//! A seeded stream of numbers: SplitMix64, integers only until the last step, so a seed gives the same wobble on every
//! machine. Each element of a strip draws from a stream of its own (`sub`), so revising one panel reshakes only that
//! panel.

/// SplitMix64.
#[derive(Clone, Debug)]
pub struct SplitMix64 {
    state: u64,
}

impl SplitMix64 {
    pub fn new(seed: u64) -> SplitMix64 {
        SplitMix64 { state: seed }
    }

    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        mix(self.state)
    }

    /// In [0, 1), with 24 bits: exact in an f32.
    pub fn unit(&mut self) -> f32 {
        (self.next_u64() >> 40) as f32 / 16_777_216.0
    }

    /// In [−1, 1).
    pub fn signed(&mut self) -> f32 {
        self.unit() * 2.0 - 1.0
    }
}

fn mix(mut z: u64) -> u64 {
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// The seed of one element's stream: the strip's seed, then the path to the element (panel, kind, index…).
pub fn sub(seed: u64, path: &[u32]) -> u64 {
    path.iter()
        .fold(mix(seed ^ 0xC0FF_EE00_D15C_0001), |h, &p| {
            mix(h ^ (u64::from(p).wrapping_add(1)).wrapping_mul(0x9E37_79B9_7F4A_7C15))
        })
}

/// A stream for one element.
pub fn stream(seed: u64, path: &[u32]) -> SplitMix64 {
    SplitMix64::new(sub(seed, path))
}

/// FNV-1a, 64 bits: a stable hash of text (a strip's default seed is its title's).
pub fn hash(text: &str) -> u64 {
    hash_bytes(text.as_bytes())
}

/// FNV-1a, 64 bits, of bytes.
pub fn hash_bytes(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xCBF2_9CE4_8422_2325, |h, &b| {
        (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01B3)
    })
}
