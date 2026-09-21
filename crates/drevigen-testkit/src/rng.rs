//! A small deterministic pseudo-random generator.
//!
//! The testkit needs reproducibility far more than it needs statistical quality: the same seed
//! must produce byte-identical trees on every platform and every Rust version, so that a
//! benchmark run today is comparable with one from six months ago. Pulling in `rand` would give
//! us a generator whose output is explicitly not stable across releases, plus a dependency tree,
//! in exchange for properties we do not need.
//!
//! [`Rng`] is [xoshiro256++], seeded through SplitMix64. Both are public-domain designs a few
//! lines long, which is exactly the case the project guidelines describe: do not take a library
//! for a function you can write correctly in ten lines.
//!
//! [xoshiro256++]: https://prng.di.unimi.it/

/// A deterministic xoshiro256++ generator.
///
/// Identical seeds produce identical sequences on every platform, forever.
#[derive(Debug, Clone)]
pub struct Rng {
    state: [u64; 4],
}

impl Rng {
    /// Creates a generator from a 64-bit seed.
    ///
    /// The seed is expanded through SplitMix64, as the xoshiro authors recommend, so that even
    /// seeds like `0` or `1` produce well-distributed state.
    #[must_use]
    pub fn new(seed: u64) -> Self {
        let mut sm = SplitMix64 { state: seed };
        Self {
            state: [sm.next(), sm.next(), sm.next(), sm.next()],
        }
    }

    /// Returns the next raw 64-bit value.
    pub fn next_u64(&mut self) -> u64 {
        let result = self.state[0]
            .wrapping_add(self.state[3])
            .rotate_left(23)
            .wrapping_add(self.state[0]);

        let t = self.state[1] << 17;
        self.state[2] ^= self.state[0];
        self.state[3] ^= self.state[1];
        self.state[1] ^= self.state[2];
        self.state[0] ^= self.state[3];
        self.state[2] ^= t;
        self.state[3] = self.state[3].rotate_left(45);

        result
    }

    /// Returns a uniform value in `0..n`.
    ///
    /// Uses Lemire's multiply-shift method with rejection, so the result is free of the modulo
    /// bias that `next_u64() % n` would introduce. Returns `0` when `n` is `0`.
    pub fn below(&mut self, n: u64) -> u64 {
        if n == 0 {
            return 0;
        }
        let mut x = self.next_u64();
        let mut m = u128::from(x) * u128::from(n);
        let mut l = m as u64;
        if l < n {
            let threshold = n.wrapping_neg() % n;
            while l < threshold {
                x = self.next_u64();
                m = u128::from(x) * u128::from(n);
                l = m as u64;
            }
        }
        (m >> 64) as u64
    }

    /// Returns a uniform value in the inclusive range `low..=high`.
    ///
    /// Returns `low` when `high` is below `low`.
    pub fn range(&mut self, low: i32, high: i32) -> i32 {
        if high <= low {
            return low;
        }
        let span = (i64::from(high) - i64::from(low) + 1) as u64;
        low + self.below(span) as i32
    }

    /// Returns a uniform value in `[0, 1)`.
    pub fn unit(&mut self) -> f64 {
        // 53 bits is the full mantissa of an f64; taking the high bits keeps the best-quality
        // bits of the xoshiro output.
        ((self.next_u64() >> 11) as f64) * (1.0 / 9_007_199_254_740_992.0)
    }

    /// Returns `true` with the given probability.
    pub fn chance(&mut self, probability: f64) -> bool {
        self.unit() < probability
    }

    /// Picks a uniformly random element, or `None` when the slice is empty.
    pub fn pick<'a, T>(&mut self, items: &'a [T]) -> Option<&'a T> {
        if items.is_empty() {
            return None;
        }
        items.get(self.below(items.len() as u64) as usize)
    }

    /// Samples a normally distributed value, clamped to `low..=high`.
    ///
    /// Uses the Box–Muller transform. Ages, lifespans and family sizes are far better modelled
    /// by a bell curve than by a uniform draw, and a genealogy that ignores that produces
    /// benchmark trees whose shape does not resemble real ones.
    pub fn normal(&mut self, mean: f64, std_dev: f64, low: f64, high: f64) -> f64 {
        // `unit()` can return exactly 0.0, which `ln` maps to negative infinity.
        let u1 = self.unit().max(f64::MIN_POSITIVE);
        let u2 = self.unit();
        let z = (-2.0 * u1.ln()).sqrt() * (std::f64::consts::TAU * u2).cos();
        (mean + z * std_dev).clamp(low, high)
    }

    /// Shuffles a slice in place with a Fisher–Yates pass.
    pub fn shuffle<T>(&mut self, items: &mut [T]) {
        for i in (1..items.len()).rev() {
            let j = self.below(i as u64 + 1) as usize;
            items.swap(i, j);
        }
    }
}

/// `SplitMix64`, used only to expand a user seed into xoshiro state.
struct SplitMix64 {
    state: u64,
}

impl SplitMix64 {
    fn next(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
}

#[cfg(test)]
mod tests {
    use super::Rng;

    #[test]
    fn same_seed_gives_same_sequence() {
        let mut a = Rng::new(42);
        let mut b = Rng::new(42);
        for _ in 0..1000 {
            assert_eq!(a.next_u64(), b.next_u64());
        }
    }

    #[test]
    fn different_seeds_diverge() {
        let mut a = Rng::new(1);
        let mut b = Rng::new(2);
        let differ = (0..64).any(|_| a.next_u64() != b.next_u64());
        assert!(differ, "two distinct seeds produced identical output");
    }

    #[test]
    fn below_stays_in_range_and_covers_it() {
        let mut rng = Rng::new(7);
        let mut seen = [false; 6];
        for _ in 0..10_000 {
            let v = rng.below(6);
            assert!(v < 6);
            seen[v as usize] = true;
        }
        assert!(
            seen.iter().all(|&s| s),
            "some values in 0..6 never occurred"
        );
    }

    #[test]
    fn below_zero_is_zero() {
        let mut rng = Rng::new(3);
        assert_eq!(rng.below(0), 0);
    }

    #[test]
    fn range_is_inclusive_and_ordered() {
        let mut rng = Rng::new(11);
        let mut low_seen = false;
        let mut high_seen = false;
        for _ in 0..10_000 {
            let v = rng.range(-3, 3);
            assert!((-3..=3).contains(&v));
            if v == -3 {
                low_seen = true;
            }
            if v == 3 {
                high_seen = true;
            }
        }
        assert!(low_seen && high_seen, "range never reached its bounds");
    }

    #[test]
    fn range_with_inverted_bounds_returns_low() {
        let mut rng = Rng::new(5);
        assert_eq!(rng.range(10, 2), 10);
        assert_eq!(rng.range(10, 10), 10);
    }

    #[test]
    fn unit_stays_in_half_open_interval() {
        let mut rng = Rng::new(13);
        for _ in 0..10_000 {
            let v = rng.unit();
            assert!((0.0..1.0).contains(&v));
        }
    }

    #[test]
    fn normal_respects_clamp_and_centres_on_mean() {
        let mut rng = Rng::new(17);
        let mut sum = 0.0;
        let n = 20_000;
        for _ in 0..n {
            let v = rng.normal(50.0, 10.0, 0.0, 100.0);
            assert!((0.0..=100.0).contains(&v));
            sum += v;
        }
        let mean = sum / f64::from(n);
        assert!((mean - 50.0).abs() < 1.0, "mean drifted to {mean}");
    }

    #[test]
    fn shuffle_is_a_permutation() {
        let mut rng = Rng::new(19);
        let mut items: Vec<u32> = (0..100).collect();
        rng.shuffle(&mut items);
        let mut sorted = items.clone();
        sorted.sort_unstable();
        assert_eq!(sorted, (0..100).collect::<Vec<_>>());
        assert_ne!(items, sorted, "shuffle left the slice in order");
    }

    #[test]
    fn pick_on_empty_slice_is_none() {
        let mut rng = Rng::new(23);
        let empty: [u8; 0] = [];
        assert!(rng.pick(&empty).is_none());
    }
}
