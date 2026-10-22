//! Keyed draws: every random choice in a site comes from its own RNG stream,
//! keyed by (seed, domain tag, hole, unit, test).
//!
//! A shared stream would make every draw depend on every draw before it, so
//! adding a hole or editing one unit would move values everywhere. Keyed
//! streams give the site model its two guarantees (ADR 0001): changing one
//! unit changes only the rows that involve it, and hole k is the same whatever
//! the hole count. The preset version is deliberately not part of the key.

use laterite_ags4_parity::Rng;

/// What a draw is for, plus where it applies. `hole` and `test` are indices;
/// `unit` is the unit's id, so reordering or inserting units does not re-key
/// the others. Use `NONE` for a part of the key that does not apply.
#[derive(Debug, Clone, Copy)]
pub struct Key<'a> {
    pub tag: &'a str,
    pub hole: u64,
    pub unit: &'a str,
    pub test: u64,
}

/// The placeholder for an index that does not apply to a draw.
pub const NONE: u64 = u64::MAX;

impl<'a> Key<'a> {
    /// A key for a draw about the whole site.
    pub fn site(tag: &'a str) -> Self {
        Key {
            tag,
            hole: NONE,
            unit: "",
            test: NONE,
        }
    }

    /// A key for a draw about one hole.
    pub fn hole(tag: &'a str, hole: usize) -> Self {
        Key {
            hole: hole as u64,
            ..Key::site(tag)
        }
    }

    /// A key for a draw about one unit across the site.
    pub fn unit(tag: &'a str, unit: &'a str) -> Self {
        Key {
            unit,
            ..Key::site(tag)
        }
    }

    /// A key for a draw about one unit in one hole.
    pub fn stratum(tag: &'a str, hole: usize, unit: &'a str) -> Self {
        Key {
            hole: hole as u64,
            unit,
            ..Key::site(tag)
        }
    }

    /// The stream for this key under `seed`.
    pub fn rng(&self, seed: u64) -> Rng {
        let mut h = mix(seed ^ 0x5349_5445_4d4f_444c); // "SITEMODL"
        h = mix(h ^ fnv(self.tag));
        h = mix(h ^ self.hole);
        h = mix(h ^ fnv(self.unit));
        h = mix(h ^ self.test);
        Rng::seeded(h)
    }
}

/// `SplitMix64`'s finaliser: a bijective scramble, so distinct inputs stay
/// distinct and nearby ones land far apart.
fn mix(mut z: u64) -> u64 {
    z = z.wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// FNV-1a, to fold a tag or unit id into the key.
fn fnv(s: &str) -> u64 {
    s.bytes().fold(0xcbf2_9ce4_8422_2325, |h, b| {
        (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3)
    })
}

/// A uniform draw in `[min, max]`.
pub fn uniform(rng: &mut Rng, min: f64, max: f64) -> f64 {
    // 53 random bits → [0, 1), the usual float construction.
    let unit = (rng.below(1 << 53) as f64) / ((1u64 << 53) as f64);
    min + (max - min) * unit
}

#[cfg(test)]
mod tests {
    use super::*;

    fn first(key: Key<'_>, seed: u64) -> u64 {
        key.rng(seed).below(u64::MAX)
    }

    /// The same key gives the same stream; any part of it changing gives a
    /// different one.
    #[test]
    fn each_part_of_the_key_selects_its_own_stream() {
        let base = Key {
            tag: "desc",
            hole: 3,
            unit: "LC",
            test: 0,
        };
        assert_eq!(first(base, 7), first(base, 7));
        let others = [
            Key {
                tag: "sample",
                ..base
            },
            Key { hole: 4, ..base },
            Key {
                unit: "RTD",
                ..base
            },
            Key { test: 1, ..base },
        ];
        for k in others {
            assert_ne!(first(base, 7), first(k, 7), "{k:?}");
        }
        assert_ne!(first(base, 7), first(base, 8));
    }

    #[test]
    fn uniform_stays_in_its_range() {
        let mut rng = Key::site("t").rng(0);
        for _ in 0..1000 {
            let v = uniform(&mut rng, -2.0, 6.0);
            assert!((-2.0..=6.0).contains(&v), "{v}");
        }
    }
}
