//! Generic made ground, set by the site's setting and settlement (ADR 0001,
//! "Generic made ground", niko86/laterite-forge#86).
//!
//! A preset's `[made_ground]` block makes a first unit, `MG`, whose presence
//! and thickness the site's **setting** (greenfield, brownfield, developed or
//! corridor) and **settlement** (village, town or city) decide, from the
//! cited tables in `data/made-ground/settings.json`. Each site draws a share
//! of holes with made ground once, from the setting's range after the
//! settlement's shift. Made ground is patchy at tens to hundreds of metres, so
//! it has a field of its own, much shorter in wavelength than the unit fields:
//! a hole has made ground where the field ranks in the top `share` of the
//! site, and the thickness rises with the field above that rank.

use std::collections::BTreeMap;
use std::sync::OnceLock;

use serde::Deserialize;

use super::draw::{Key, uniform};
use super::ground::Field;
use super::preset::{AmountWeights, ConstituentShare, CountWeights, Extent, MadeGroundSpec, Range};

const SETTINGS_JSON: &str = include_str!("../../data/made-ground/settings.json");

/// The id of the generic made-ground unit.
pub const GENERIC_ID: &str = "MG";

/// What the site is for, which decides how much made ground it has.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Setting {
    Greenfield,
    Brownfield,
    Developed,
    /// Road, rail and flood schemes.
    Corridor,
}

impl Setting {
    pub const ALL: [Setting; 4] = [
        Setting::Greenfield,
        Setting::Brownfield,
        Setting::Developed,
        Setting::Corridor,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Setting::Greenfield => "greenfield",
            Setting::Brownfield => "brownfield",
            Setting::Developed => "developed",
            Setting::Corridor => "corridor",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|v| v.name() == s)
    }
}

/// How large the place the site is in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Settlement {
    Village,
    Town,
    City,
}

impl Settlement {
    pub const ALL: [Settlement; 3] = [Settlement::Village, Settlement::Town, Settlement::City];

    pub fn name(self) -> &'static str {
        match self {
            Settlement::Village => "village",
            Settlement::Town => "town",
            Settlement::City => "city",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|v| v.name() == s)
    }
}

/// A preset's `[made_ground]` block: the defaults and what the unit borrows.
#[derive(Debug, Clone, PartialEq)]
pub struct GenericMadeGround {
    pub setting: Setting,
    pub settlement: Settlement,
    /// The patch field's wavelengths, metres: each of its waves draws one in
    /// this range.
    pub wavelength_m: Range,
    /// The block's own constituent, count and amount weights, when it gives
    /// them; else the setting's from `settings.json`.
    pub make_up: Option<MadeGroundSpec>,
}

#[derive(Debug, Deserialize)]
struct Data {
    settings: BTreeMap<String, SettingData>,
    settlements: BTreeMap<String, SettlementData>,
}

#[derive(Debug, Deserialize)]
struct SettingData {
    share: ShareData,
    thickness_m: ThicknessData,
    make_up: MakeUpData,
}

#[derive(Debug, Deserialize)]
struct ShareData {
    min: f64,
    max: f64,
    #[serde(default)]
    skew: Option<f64>,
    basis: String,
}

#[derive(Debug, Deserialize)]
struct ThicknessData {
    p10: f64,
    p50: f64,
    p90: f64,
    basis: String,
}

#[derive(Debug, Deserialize)]
struct MakeUpData {
    constituents: Vec<ConstituentData>,
    count: Weights<4>,
    amounts: Weights<5>,
}

#[derive(Debug, Deserialize)]
struct ConstituentData {
    name: String,
    share: f64,
    basis: String,
}

#[derive(Debug, Deserialize)]
struct Weights<const N: usize> {
    #[serde(with = "serde_arrays")]
    weights: [f64; N],
    basis: String,
}

mod serde_arrays {
    use serde::{Deserialize, Deserializer};

    pub fn deserialize<'de, D: Deserializer<'de>, const N: usize>(
        d: D,
    ) -> Result<[f64; N], D::Error> {
        let v = Vec::<f64>::deserialize(d)?;
        let n = v.len();
        v.try_into()
            .map_err(|_| serde::de::Error::custom(format!("needs {N} weights, found {n}")))
    }
}

#[derive(Debug, Deserialize)]
struct SettlementData {
    share_scale: f64,
    gap_scale: f64,
    thickness_scale: f64,
    basis: String,
}

fn data() -> &'static Data {
    static DATA: OnceLock<Data> = OnceLock::new();
    DATA.get_or_init(|| {
        let d: Data =
            serde_json::from_str(SETTINGS_JSON).expect("made-ground settings.json parses");
        d.check().expect("made-ground settings.json is valid");
        d
    })
}

impl Data {
    fn check(&self) -> Result<(), String> {
        for s in Setting::ALL {
            let d = self
                .settings
                .get(s.name())
                .ok_or_else(|| format!("no setting {}", s.name()))?;
            let at = s.name();
            if d.share.basis.trim().is_empty() || d.thickness_m.basis.trim().is_empty() {
                return Err(format!("{at}: empty basis"));
            }
            if !(0.0..=1.0).contains(&d.share.min)
                || !(d.share.min..=1.0).contains(&d.share.max)
                || d.share.skew.is_some_and(|k| k <= 0.0)
            {
                return Err(format!("{at}: bad share range"));
            }
            let t = &d.thickness_m;
            if !(0.0 < t.p10 && t.p10 < t.p50 && t.p50 < t.p90) {
                return Err(format!("{at}: thickness quantiles must rise from above 0"));
            }
            let known = crate::synth::made_ground::constituents();
            for c in &d.make_up.constituents {
                if !known.vocabulary().any(|k| *k == c.name) {
                    return Err(format!("{at}: unknown constituent {}", c.name));
                }
                if !(0.0..=1.0).contains(&c.share) || c.basis.trim().is_empty() {
                    return Err(format!("{at}: bad constituent {}", c.name));
                }
            }
            for w in [&d.make_up.count.weights[..], &d.make_up.amounts.weights[..]] {
                if w.iter().any(|x| *x < 0.0) || w.iter().sum::<f64>() <= 0.0 {
                    return Err(format!("{at}: bad weights"));
                }
            }
            if d.make_up.count.basis.trim().is_empty() || d.make_up.amounts.basis.trim().is_empty()
            {
                return Err(format!("{at}: empty make-up basis"));
            }
        }
        for s in Settlement::ALL {
            let d = self
                .settlements
                .get(s.name())
                .ok_or_else(|| format!("no settlement {}", s.name()))?;
            if d.basis.trim().is_empty()
                || d.share_scale <= 0.0
                || d.share_scale > 1.0
                || !(0.0..=1.0).contains(&d.gap_scale)
                || d.thickness_scale <= 0.0
            {
                return Err(format!("{}: bad settlement", s.name()));
            }
        }
        Ok(())
    }
}

/// A setting's targets after the settlement's shift.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Targets {
    /// The range the site's share of holes with made ground is drawn from,
    /// and the skew on the draw.
    pub share_min: f64,
    pub share_max: f64,
    pub share_skew: Option<f64>,
    /// Thickness at the 10th, 50th and 90th percentile of holes with made
    /// ground, metres.
    pub p10: f64,
    pub p50: f64,
    pub p90: f64,
}

impl Targets {
    /// The targets for `setting` at `settlement`.
    pub fn new(setting: Setting, settlement: Settlement) -> Self {
        let d = data();
        let s = &d.settings[setting.name()];
        let t = &d.settlements[settlement.name()];
        // The share keeps `gap_scale` of its distance from 1.
        let shift = |v: f64| 1.0 - (1.0 - v * t.share_scale) * t.gap_scale;
        Targets {
            share_min: shift(s.share.min),
            share_max: shift(s.share.max),
            share_skew: s.share.skew,
            p10: s.thickness_m.p10 * t.thickness_scale,
            p50: s.thickness_m.p50 * t.thickness_scale,
            p90: s.thickness_m.p90 * t.thickness_scale,
        }
    }

    /// The middle of the share draw: its median.
    #[cfg(test)]
    pub fn share_median(&self) -> f64 {
        let u = 0.5_f64.powf(self.share_skew.unwrap_or(1.0));
        self.share_min + (self.share_max - self.share_min) * u
    }
}

impl GenericMadeGround {
    /// The made-ground spec the unit takes: the block's own weights when it
    /// gives them, else the setting's.
    pub fn spec(&self) -> MadeGroundSpec {
        if let Some(own) = &self.make_up {
            return own.clone();
        }
        let m = &data().settings[self.setting.name()].make_up;
        MadeGroundSpec {
            constituents: Some(
                m.constituents
                    .iter()
                    .map(|c| ConstituentShare {
                        name: c.name.clone(),
                        share: Range {
                            min: c.share,
                            max: c.share,
                            basis: c.basis.clone(),
                            skew: None,
                        },
                    })
                    .collect(),
            ),
            count: Some(CountWeights {
                weights: m.count.weights,
                basis: m.count.basis.clone(),
            }),
            amounts: Some(AmountWeights {
                weights: m.amounts.weights,
                basis: m.amounts.basis.clone(),
            }),
        }
    }

    pub fn targets(&self) -> Targets {
        Targets::new(self.setting, self.settlement)
    }
}

/// Samples along each side of the grid that ranks the patch field.
const GRID: usize = 64;

/// The made ground of one site: its patch field, the share of holes with
/// made ground this site drew, and the thickness curve.
#[derive(Debug, Clone, PartialEq)]
pub struct Patches {
    field: Field,
    /// The field's values over a grid on the site, sorted: a hole's rank in
    /// it says how far up the site's field it sits.
    ranked: Vec<f64>,
    share: f64,
    /// ln of the thickness at v = 0, 0.1, 0.5, 0.9 and 1, where v is how far
    /// into the holes that have made ground this one ranks.
    ln_t: [f64; 5],
}

/// The thinnest made ground drawn: one cell in the third place of a decimal
/// would round away, so the curve stays above it.
const FLOOR_M: f64 = 0.05;

impl Patches {
    /// The patches of a site of `extent` under `seed`.
    pub fn new(g: &GenericMadeGround, extent: Extent, seed: u64) -> Self {
        let t = g.targets();
        let share = {
            let u = uniform(&mut Key::site("made-ground-share").rng(seed), 0.0, 1.0);
            let u = t.share_skew.map_or(u, |k| u.powf(k));
            t.share_min + (t.share_max - t.share_min) * u
        };
        let field = Field::with_wavelengths(
            &mut Key::site("made-ground-patches").rng(seed),
            g.wavelength_m.min,
            g.wavelength_m.max,
        );
        let mut ranked: Vec<f64> = (0..GRID * GRID)
            .map(|i| {
                let (ix, iy) = (i % GRID, i / GRID);
                let x = extent.x * (ix as f64 + 0.5) / GRID as f64;
                let y = extent.y * (iy as f64 + 0.5) / GRID as f64;
                field.at(x, y)
            })
            .collect();
        ranked.sort_by(f64::total_cmp);
        // Log-linear through the three quantiles, carried on to the ends of
        // the range at the slope of the nearer pair.
        let (l10, l50, l90) = (t.p10.ln(), t.p50.ln(), t.p90.ln());
        let low = l10 - (l50 - l10) * (0.1 / 0.4);
        let high = l90 + (l90 - l50) * (0.1 / 0.4);
        Patches {
            field,
            ranked,
            share,
            ln_t: [low.max(FLOOR_M.ln()), l10, l50, l90, high],
        }
    }

    /// The share of the site's holes that have made ground, as drawn.
    #[cfg(test)]
    pub fn share(&self) -> f64 {
        self.share
    }

    /// Made-ground thickness at (`x`, `y`), metres; zero where there is none.
    pub fn thickness_at(&self, x: f64, y: f64) -> f64 {
        let f = self.field.at(x, y);
        let below = self.ranked.partition_point(|g| *g < f);
        let rank = below as f64 / self.ranked.len() as f64;
        let cutoff = 1.0 - self.share;
        if self.share <= 0.0 || rank < cutoff {
            return 0.0;
        }
        let v = (rank - cutoff) / self.share;
        self.thickness_at_rank(v)
    }

    /// The thickness curve at `v`, 0 to 1 through the holes with made ground.
    fn thickness_at_rank(&self, v: f64) -> f64 {
        const KNOTS: [f64; 5] = [0.0, 0.1, 0.5, 0.9, 1.0];
        let i = KNOTS
            .windows(2)
            .position(|w| v < w[1])
            .unwrap_or(KNOTS.len() - 2);
        let (v0, v1) = (KNOTS[i], KNOTS[i + 1]);
        let a = (v - v0) / (v1 - v0);
        (self.ln_t[i] + (self.ln_t[i + 1] - self.ln_t[i]) * a).exp()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_embedded_settings_are_valid_and_every_value_has_a_basis() {
        let d = data();
        for s in Setting::ALL {
            let d = &d.settings[s.name()];
            assert!(d.share.basis.len() > 40);
            assert!(d.thickness_m.basis.len() > 40);
            for c in &d.make_up.constituents {
                assert!(c.basis.len() > 40, "{}", c.name);
            }
        }
    }

    #[test]
    fn every_basis_citing_the_bgs_ngdc_carries_the_ogl_attribution() {
        let d = data();
        let mut bases: Vec<&str> = Vec::new();
        for s in d.settings.values() {
            bases.push(&s.share.basis);
            bases.push(&s.thickness_m.basis);
            bases.push(&s.make_up.count.basis);
            bases.push(&s.make_up.amounts.basis);
            bases.extend(s.make_up.constituents.iter().map(|c| c.basis.as_str()));
        }
        bases.extend(d.settlements.values().map(|s| s.basis.as_str()));
        for b in bases {
            if b.contains("NGDC") || b.contains("BGS") {
                assert!(b.contains("Contains data supplied by UKRI, OGL v3"), "{b}");
            }
        }
    }

    #[test]
    fn a_village_shrinks_the_share_and_thickness_and_a_city_halves_the_gap_to_one() {
        for s in Setting::ALL {
            let town = Targets::new(s, Settlement::Town);
            let village = Targets::new(s, Settlement::Village);
            let city = Targets::new(s, Settlement::City);
            assert!((village.share_max - 0.6 * town.share_max).abs() < 1e-12);
            assert!((village.p50 - 0.7 * town.p50).abs() < 1e-12);
            assert!((city.share_min - (1.0 - (1.0 - town.share_min) / 2.0)).abs() < 1e-12);
            assert!((city.p90 - 1.2 * town.p90).abs() < 1e-12);
        }
    }

    #[test]
    fn a_settings_targets_at_a_town_are_the_tables() {
        let t = Targets::new(Setting::Corridor, Settlement::Town);
        assert!((t.share_min - 0.07).abs() < 1e-12 && (t.share_max - 1.0).abs() < 1e-12);
        assert_eq!((t.p10, t.p50, t.p90), (0.6, 2.0, 6.5));
    }

    fn block(setting: Setting) -> GenericMadeGround {
        GenericMadeGround {
            setting,
            settlement: Settlement::Town,
            wavelength_m: Range {
                min: 50.0,
                max: 200.0,
                basis: "test".into(),
                skew: None,
            },
            make_up: None,
        }
    }

    const EXTENT: Extent = Extent { x: 300.0, y: 200.0 };

    /// The grid cells the site's rank is taken over, as (x, y) points.
    fn cells() -> impl Iterator<Item = (f64, f64)> {
        (0..GRID * GRID).map(|i| {
            (
                EXTENT.x * ((i % GRID) as f64 + 0.5) / GRID as f64,
                EXTENT.y * ((i / GRID) as f64 + 0.5) / GRID as f64,
            )
        })
    }

    #[test]
    fn a_site_has_made_ground_over_the_share_it_drew() {
        for setting in Setting::ALL {
            for seed in 0..30 {
                let p = Patches::new(&block(setting), EXTENT, seed);
                let covered = cells()
                    .filter(|(x, y)| p.thickness_at(*x, *y) > 0.0)
                    .count();
                let got = covered as f64 / (GRID * GRID) as f64;
                assert!(
                    (got - p.share()).abs() < 0.01,
                    "{} seed {seed}: {got} of the site against a share of {}",
                    setting.name(),
                    p.share()
                );
            }
        }
    }

    #[test]
    fn made_ground_thickens_as_the_field_rises_above_its_threshold() {
        for seed in 0..30 {
            let p = Patches::new(&block(Setting::Corridor), EXTENT, seed);
            let mut points: Vec<(f64, f64)> = cells()
                .map(|(x, y)| (p.field.at(x, y), p.thickness_at(x, y)))
                .filter(|(_, t)| *t > 0.0)
                .collect();
            points.sort_by(|a, b| a.0.total_cmp(&b.0));
            for w in points.windows(2) {
                assert!(w[0].1 <= w[1].1 + 1e-12, "seed {seed}: {w:?}");
            }
        }
    }

    #[test]
    fn a_sites_share_is_one_keyed_draw_the_unit_fields_do_not_move() {
        // The same seed draws the same patches whatever else changes.
        let a = Patches::new(&block(Setting::Brownfield), EXTENT, 9);
        let b = Patches::new(&block(Setting::Brownfield), EXTENT, 9);
        assert_eq!(a, b);
        assert_ne!(a, Patches::new(&block(Setting::Brownfield), EXTENT, 10));
    }

    #[test]
    fn each_settings_make_up_loads_as_a_made_ground_spec() {
        for s in Setting::ALL {
            let g = GenericMadeGround {
                setting: s,
                settlement: Settlement::Town,
                wavelength_m: Range {
                    min: 50.0,
                    max: 200.0,
                    basis: "test".into(),
                    skew: None,
                },
                make_up: None,
            };
            let spec = g.spec();
            assert_eq!(spec.constituents.as_ref().unwrap().len(), 12);
            assert!(spec.count.is_some() && spec.amounts.is_some());
        }
    }
}
