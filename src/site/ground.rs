//! Smooth fields over the site: unit thicknesses and ground level vary across
//! the site, but slowly, so neighbouring holes agree.
//!
//! A field is a few long cosine waves summed and scaled into `[0, 1]`. The
//! shortest wavelength is the site's longer side, so a field can rise and fall
//! about once across the site but never jumps between nearby holes. A range is
//! mapped onto the field, so the value at a hole always lies within the range.

use std::f64::consts::TAU;

use laterite_ags4_parity::Rng;

use super::draw::uniform;
use super::preset::{Extent, Range};

const WAVES: usize = 3;

/// A smooth scalar field over the site, valued in `[0, 1]`.
#[derive(Debug, Clone)]
pub struct Field {
    /// (wavenumber x, wavenumber y, phase, amplitude) per wave.
    waves: [(f64, f64, f64, f64); WAVES],
}

impl Field {
    /// A field for a site of `extent`, drawn from `rng`.
    pub fn new(rng: &mut Rng, extent: Extent) -> Self {
        let longest = extent.x.max(extent.y);
        let waves = std::array::from_fn(|_| {
            let wavelength = uniform(rng, longest, 3.0 * longest);
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

    /// `range` mapped onto the field at (`x`, `y`).
    pub fn in_range(&self, range: &Range, x: f64, y: f64) -> f64 {
        range.min + (range.max - range.min) * self.at(x, y)
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
}
