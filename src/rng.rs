//! A small deterministic random number generator, so a seed reproduces a scene exactly in tests
//! and in the headless render.

use std::time::{SystemTime, UNIX_EPOCH};

/// A splitmix64 generator. It is not for cryptography; it only has to be fast, portable and the
/// same on every platform for a given seed.
#[derive(Clone, Debug)]
pub struct Rng {
    state: u64,
}

impl Rng {
    pub fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    /// Seeds from the clock, for an interactive run with no `--seed`.
    pub fn from_time() -> Self {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0x5eed);
        Self::new(nanos)
    }

    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// A value in `[0, 1)`.
    pub fn unit(&mut self) -> f32 {
        // 24 bits is every bit an f32 mantissa can hold
        (self.next_u64() >> 40) as f32 / (1u64 << 24) as f32
    }

    /// A value in `[low, high)`.
    pub fn range(&mut self, low: f32, high: f32) -> f32 {
        low + (high - low) * self.unit()
    }

    /// An index below `n` (0 when `n` is 0).
    pub fn below(&mut self, n: usize) -> usize {
        if n == 0 {
            0
        } else {
            (self.next_u64() % n as u64) as usize
        }
    }

    /// True with probability `p`.
    pub fn chance(&mut self, p: f32) -> bool {
        self.unit() < p
    }

    /// One item of a non-empty slice.
    pub fn pick<'a, T>(&mut self, items: &'a [T]) -> &'a T {
        &items[self.below(items.len())]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_same_seed_gives_the_same_sequence() {
        let mut a = Rng::new(42);
        let mut b = Rng::new(42);
        for _ in 0..100 {
            assert_eq!(a.next_u64(), b.next_u64());
        }
    }

    #[test]
    fn different_seeds_differ() {
        let mut a = Rng::new(1);
        let mut b = Rng::new(2);
        assert_ne!(a.next_u64(), b.next_u64());
    }

    #[test]
    fn unit_and_range_stay_in_bounds() {
        let mut rng = Rng::new(7);
        for _ in 0..10_000 {
            let u = rng.unit();
            assert!((0.0..1.0).contains(&u), "{u}");
            let r = rng.range(-2.0, 3.0);
            assert!((-2.0..3.0).contains(&r), "{r}");
            assert!(rng.below(5) < 5);
        }
        assert_eq!(rng.below(0), 0);
    }

    #[test]
    fn pick_returns_an_item() {
        let mut rng = Rng::new(3);
        let items = ['a', 'b', 'c'];
        for _ in 0..50 {
            assert!(items.contains(rng.pick(&items)));
        }
    }
}
