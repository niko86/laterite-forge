//! Smooth fields over the site: unit thicknesses and ground level vary across
//! the site, but slowly, so neighbouring holes agree.
//!
//! A field is a few long cosine waves summed and scaled into `[0, 1]`. The
//! shortest wavelength is the site's longer side, so a field can rise and fall
//! about once across the site but never jumps between nearby holes. A range is
//! mapped onto the field, so the value at a hole always lies within the range.
//!
//! [`Field::with_wavelengths`] sets the wavelength range itself: generic made
//! ground uses tens to hundreds of metres for its patches (#86).

use std::f64::consts::TAU;

use laterite_ags4_parity::Rng;

use super::draw::uniform;
use super::preset::{Extent, Range};

const WAVES: usize = 3;

/// A smooth scalar field over the site, valued in `[0, 1]`.
#[derive(Debug, Clone, PartialEq)]
pub struct Field {
    /// (wavenumber x, wavenumber y, phase, amplitude) per wave.
    waves: [(f64, f64, f64, f64); WAVES],
}

impl Field {
    /// A field for a site of `extent`, drawn from `rng`.
    pub fn new(rng: &mut Rng, extent: Extent) -> Self {
        let longest = extent.x.max(extent.y);
        Self::with_wavelengths(rng, longest, 3.0 * longest)
    }

    /// A field whose waves each have a wavelength drawn from `shortest` to
    /// `longest` metres. [`Field::new`] is this with 1 to 3 times the site's
    /// longer side; made-ground patches use tens to hundreds of metres.
    pub fn with_wavelengths(rng: &mut Rng, shortest: f64, longest: f64) -> Self {
        let waves = std::array::from_fn(|_| {
            let wavelength = uniform(rng, shortest, longest);
            let angle = uniform(rng, 0.0, TAU);
            let k = TAU / wavelength;
            (
                k * angle.cos(),
                k * angle.sin(),
                uniform(rng, 0.0, TAU),
                uniform(rng, 0.5, 1.0),
            )
        });
        Field { waves }
    }

    /// The field's value at (`x`, `y`), in `[0, 1]`.
    pub fn at(&self, x: f64, y: f64) -> f64 {
        let total: f64 = self.waves.iter().map(|w| w.3).sum();
        let sum: f64 = self
            .waves
            .iter()
            .map(|&(kx, ky, phase, amp)| amp * (kx * x + ky * y + phase).cos())
            .sum();
        0.5 + 0.5 * sum / total
    }

    /// An upper bound on how fast the field changes, per metre: two points
    /// `d` metres apart differ by at most `max_slope() * d`. The tests hold
    /// the field and the strata built on it to this bound.
    #[cfg(test)]
    pub fn max_slope(&self) -> f64 {
        let total: f64 = self.waves.iter().map(|w| w.3).sum();
        let weighted: f64 = self
            .waves
            .iter()
            .map(|&(kx, ky, _, amp)| amp * kx.hypot(ky))
            .sum();
        0.5 * weighted / total
    }

    /// `range` mapped onto the field at (`x`, `y`). A range with a skew
    /// raises the field value to that power first: the map is monotone and
    /// smooth, fixes 0 and 1, and for a skew above 1 puts most of the site
    /// near `min` with a long tail toward `max`. Its slope is at most
    /// `skew` times the field's, so the bound scales by the skew.
    pub fn in_range(&self, range: &Range, x: f64, y: f64) -> f64 {
        let u = self.at(x, y);
        let u = match range.skew {
            None => u,
            Some(skew) => u.powf(skew),
        };
        range.min + (range.max - range.min) * u
    }
}

// A range's minimum is an exact endpoint of the mapping, so it is compared
// exactly.
#[allow(clippy::float_cmp)]
#[cfg(test)]
mod tests {
    use super::*;
    use crate::site::draw::Key;

    const EXTENT: Extent = Extent { x: 400.0, y: 300.0 };

    #[test]
    fn a_field_stays_in_the_unit_interval() {
        for seed in 0..20 {
            let f = Field::new(&mut Key::site("t").rng(seed), EXTENT);
            for i in 0..50 {
                for j in 0..50 {
                    let v = f.at(f64::from(i) * 8.0, f64::from(j) * 6.0);
                    assert!((0.0..=1.0).contains(&v), "seed {seed}: {v}");
                }
            }
        }
    }

    /// Nearby points agree: the change between two points never exceeds the
    /// slope bound times their distance, and the bound itself is gentle
    /// (well under a full swing per 50 m on a 400 m site).
    #[test]
    fn a_field_changes_slowly_across_the_site() {
        for seed in 0..20 {
            let mut rng = Key::site("t").rng(seed);
            let field = Field::new(&mut rng, EXTENT);
            let slope = field.max_slope();
            assert!(slope * 50.0 < 0.5, "seed {seed}: {slope}");
            for _ in 0..200 {
                let mut point = || (uniform(&mut rng, 0.0, 400.0), uniform(&mut rng, 0.0, 300.0));
                let (a, b) = (point(), point());
                let dist = (a.0 - b.0).hypot(a.1 - b.1);
                let change = (field.at(a.0, a.1) - field.at(b.0, b.1)).abs();
                assert!(
                    change <= slope * dist + 1e-12,
                    "seed {seed}: {change} over {dist} m"
                );
            }
        }
    }

    #[test]
    fn a_range_maps_onto_the_field() {
        let f = Field::new(&mut Key::site("t").rng(3), EXTENT);
        let r = Range {
            min: -2.0,
            max: 6.0,
            basis: "test fixture".into(),
            skew: None,
        };
        // The field's own value is the fraction of the way along the range.
        let v = f.in_range(&r, 100.0, 100.0);
        assert!((-2.0..=6.0).contains(&v));
        let at_min = Range {
            max: -2.0,
            ..r.clone()
        };
        assert_eq!(f.in_range(&at_min, 100.0, 100.0), -2.0);
    }

    fn skewed(skew: Option<f64>) -> Range {
        Range {
            min: -1.5,
            max: 7.0,
            basis: "test fixture".into(),
            skew,
        }
    }

    /// A skew of 1 is the plain mapping, bit for bit.
    #[test]
    fn a_skew_of_one_is_no_skew() {
        for seed in 0..10 {
            let f = Field::new(&mut Key::site("t").rng(seed), EXTENT);
            for i in 0..40 {
                for j in 0..40 {
                    let (x, y) = (f64::from(i) * 10.0, f64::from(j) * 7.5);
                    assert_eq!(
                        f.in_range(&skewed(Some(1.0)), x, y).to_bits(),
                        f.in_range(&skewed(None), x, y).to_bits()
                    );
                }
            }
        }
    }

    /// A skew above 1 pulls the typical value below the unskewed one, a skew
    /// below 1 lifts it, and the ends stay reachable: the map fixes 0 and 1.
    #[test]
    fn a_skew_above_one_lowers_the_median() {
        let mut flat = Vec::new();
        let mut high = Vec::new();
        let mut low = Vec::new();
        for seed in 0..20 {
            let f = Field::new(&mut Key::site("t").rng(seed), EXTENT);
            for i in 0..20 {
                for j in 0..20 {
                    let (x, y) = (f64::from(i) * 20.0, f64::from(j) * 15.0);
                    flat.push(f.in_range(&skewed(None), x, y));
                    high.push(f.in_range(&skewed(Some(2.0)), x, y));
                    low.push(f.in_range(&skewed(Some(0.5)), x, y));
                }
            }
        }
        let median = |v: &mut Vec<f64>| {
            v.sort_by(f64::total_cmp);
            v[v.len() / 2]
        };
        let (flat, high, low) = (median(&mut flat), median(&mut high), median(&mut low));
        let mid = f64::midpoint(-1.5, 7.0);
        assert!(high < flat - 1.0, "{high} vs {flat}");
        assert!(high < mid - 1.0, "{high} vs midpoint {mid}");
        assert!(low > flat, "{low} vs {flat}");
        for k in [0.5, 2.0, 5.0] {
            assert_eq!(0.0_f64.powf(k), 0.0);
            assert_eq!(1.0_f64.powf(k), 1.0);
        }
    }

    /// Nearby points agree under a skew too: the change is at most the skew
    /// times the field's slope bound times the range width times the distance.
    #[test]
    fn a_skewed_range_changes_slowly_across_the_site() {
        let width = 8.5;
        for skew in [1.0, 2.0, 3.5] {
            let r = skewed(Some(skew));
            for seed in 0..20 {
                let mut rng = Key::site("t").rng(seed);
                let field = Field::new(&mut rng, EXTENT);
                let slope = field.max_slope() * skew * width;
                for _ in 0..200 {
                    let mut point =
                        || (uniform(&mut rng, 0.0, 400.0), uniform(&mut rng, 0.0, 300.0));
                    let (a, b) = (point(), point());
                    let dist = (a.0 - b.0).hypot(a.1 - b.1);
                    let change =
                        (field.in_range(&r, a.0, a.1) - field.in_range(&r, b.0, b.1)).abs();
                    assert!(
                        change <= slope * dist + 1e-9,
                        "skew {skew}, seed {seed}: {change} over {dist} m"
                    );
                }
            }
        }
    }

    /// The mapped value never falls as the field value rises.
    #[test]
    fn the_skew_map_is_monotone() {
        let f = Field::new(&mut Key::site("t").rng(5), EXTENT);
        let r = skewed(Some(2.5));
        let mut pairs = Vec::new();
        for i in 0..400 {
            let x = f64::from(i);
            pairs.push((f.at(x, 100.0), f.in_range(&r, x, 100.0)));
        }
        pairs.sort_by(|a, b| a.0.total_cmp(&b.0));
        assert!(pairs.windows(2).all(|w| w[0].1 <= w[1].1));
    }
}
