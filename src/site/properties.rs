//! The property model: index properties as smooth fields with depth trends
//! and rare outliers, density from the phase relations, overburden stress
//! and groundwater (ADR 0001, "Values and descriptions").
//!
//! Fine units draw plastic limit, plasticity index and liquidity index; liquid
//! limit, moisture content and the consistency index are derived from them
//! (so the plastic limit cannot go negative, which drawing the liquid limit
//! and plasticity index would allow). Coarse units draw SPT N and moisture
//! content. Nothing here is written to the file yet; the description term
//! and the results groups read it later (#41).
//!
//! Every draw is keyed (`draw::Key`) under tags of its own, so a preset
//! without properties consumes exactly the streams it always did.

use super::build::{Hole, Site};
use super::draw::{Key, uniform};
use super::ground::Field;
use super::preset::{Extent, Preset, Properties, Quantity, Range};
use crate::catalogue;

/// What values a quantity can physically take. The loader refuses a range
/// outside it, and evaluation clamps a drawn value to it (not to the range,
/// so a depth gradient may carry a value past its range, but never past
/// physics).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Domain {
    lo: f64,
    /// The lower bound itself is not allowed.
    lo_open: bool,
    hi: f64,
}

/// How far above an open lower bound a clamped value sits.
const OPEN_BOUND_MARGIN: f64 = 1e-6;

impl Domain {
    const fn at_least(lo: f64) -> Self {
        Domain {
            lo,
            lo_open: false,
            hi: f64::INFINITY,
        }
    }

    const fn above(lo: f64) -> Self {
        Domain {
            lo,
            lo_open: true,
            hi: f64::INFINITY,
        }
    }

    const fn between(lo: f64, hi: f64) -> Self {
        Domain {
            lo,
            lo_open: false,
            hi,
        }
    }

    const fn above_to(lo: f64, hi: f64) -> Self {
        Domain {
            lo,
            lo_open: true,
            hi,
        }
    }

    /// Whether `v` is a value this domain allows.
    pub fn contains(&self, v: f64) -> bool {
        let above = if self.lo_open {
            v > self.lo
        } else {
            v >= self.lo
        };
        v.is_finite() && above && v <= self.hi
    }

    /// `v` moved into the domain (just above an open lower bound).
    pub fn clamp(&self, v: f64) -> f64 {
        let floor = if self.lo_open {
            self.lo + OPEN_BOUND_MARGIN
        } else {
            self.lo
        };
        v.clamp(floor, self.hi)
    }

    /// The domain in words, as `> 0 and <= 1`.
    pub fn describe(&self) -> String {
        let lower = format!("{} {}", if self.lo_open { ">" } else { ">=" }, self.lo);
        if self.hi.is_finite() {
            format!("{lower} and <= {}", self.hi)
        } else {
            lower
        }
    }
}

/// Plastic limit, percent: a water content, positive (a soil with no plastic
/// limit is non-plastic and has no consistency).
pub const PLASTIC_LIMIT_PERCENT: Domain = Domain::above(0.0);
/// Plasticity index, percent: positive, because PI <= 0 means non-plastic and
/// the consistency index Ic = (LL - w) / PI is then undefined.
pub const PLASTICITY_INDEX_PERCENT: Domain = Domain::above(0.0);
/// Liquidity index: -1 to 3. Desiccated clay sits a little below 0 (w below
/// PL) and a sensitive soft clay well above 1; beyond these the soil is not
/// a plausible plastic soil. Evaluation also keeps it from taking the
/// moisture content below zero.
pub const LIQUIDITY_INDEX: Domain = Domain::between(-1.0, 3.0);
/// SPT N, blows: a count, so not negative.
pub const SPT_N: Domain = Domain::at_least(0.0);
/// Moisture content, percent: a mass ratio of water to solids, not negative.
pub const MOISTURE_CONTENT_PERCENT: Domain = Domain::at_least(0.0);
/// Particle density ratio Gs: above 1 (denser than water) and below 4, which
/// clears the densest common soil minerals (iron oxides sit near 5 but are
/// not soil-forming here) with room.
pub const PARTICLE_DENSITY_RATIO: Domain = Domain::above_to(1.0, 4.0);
/// Degree of saturation Sr: a fraction, positive (the phase relations divide
/// by it) and at most 1 (full saturation).
pub const DEGREE_OF_SATURATION: Domain = Domain::above_to(0.0, 1.0);
/// A probability.
pub const OUTLIER_RATE: Domain = Domain::between(0.0, 1.0);
/// Depth of the water table below ground level, metres: not above ground.
pub const GROUNDWATER_DEPTH_M: Domain = Domain::at_least(0.0);

/// The chance, per (hole, unit, quantity), of an outlier when a preset gives
/// no `outlier_rate`. Basis: forge judgement. Rare enough that a unit's trend
/// reads clearly, common enough to show up in a modest file.
pub const OUTLIER_RATE_DEFAULT: f64 = 0.02;

/// Water's density, kg/m3, for the phase relations and the pore pressure.
const WATER_DENSITY_KG_M3: f64 = 1000.0;

/// An outlier lands beyond the nearer range bound by this fraction of the
/// range width, drawn between the two. Forge judgement.
const OUTLIER_OVERSHOOT: (f64, f64) = (0.25, 0.75);

/// Outlier-stream indices (the `test` part of the key) of the drawn
/// quantities. Gs and Sr are smooth only: an outlier particle density is not
/// a thing a site investigation finds.
const Q_PLASTIC_LIMIT: u64 = 0;
const Q_PLASTICITY_INDEX: u64 = 1;
const Q_LIQUIDITY_INDEX: u64 = 2;
const Q_SPT_N: u64 = 3;
const Q_MOISTURE_CONTENT: u64 = 4;

/// One drawn quantity of one unit: its range and gradient mapped onto smooth
/// fields over the site.
#[derive(Debug, Clone, PartialEq)]
struct Drawn {
    quantity: Quantity,
    domain: Domain,
    /// Outlier-stream index, or `None` for a smooth-only quantity.
    index: Option<u64>,
    field: Field,
    gradient: Option<Field>,
}

impl Drawn {
    fn new(
        quantity: &Quantity,
        name: &str,
        domain: Domain,
        index: Option<u64>,
        (unit, seed, extent): (&str, u64, Extent),
    ) -> Self {
        let field = Field::new(
            &mut Key::unit(&format!("property:{name}"), unit).rng(seed),
            extent,
        );
        let gradient = quantity.per_m.as_ref().map(|_| {
            Field::new(
                &mut Key::unit(&format!("property:{name}:gradient"), unit).rng(seed),
                extent,
            )
        });
        Drawn {
            quantity: quantity.clone(),
            domain,
            index,
            field,
            gradient,
        }
    }

    /// The smooth value at (`x`, `y`), `z` metres below the unit's top.
    fn smooth(&self, x: f64, y: f64, z: f64) -> f64 {
        let slope = match (&self.gradient, &self.quantity.per_m) {
            (Some(field), Some(per_m)) => field.in_range(per_m, x, y),
            _ => 0.0,
        };
        self.domain
            .clamp(self.field.in_range(&self.quantity.range, x, y) + slope * z)
    }

    /// The value of this quantity in one hole: smooth, or an outlier when the
    /// hole's draw for it falls under `rate`.
    fn value(&self, at: &Spot<'_>, rate: f64) -> f64 {
        let v = self.smooth(at.x, at.y, at.z);
        let Some(index) = self.index else { return v };
        let key = Key {
            tag: "outlier",
            hole: at.hole as u64,
            unit: at.unit,
            test: index,
        };
        // The status and the overshoot are drawn in the same order whatever
        // the rate, so changing the rate moves only the values whose status
        // changes.
        let mut rng = key.rng(at.seed);
        let chance = uniform(&mut rng, 0.0, 1.0);
        let overshoot = uniform(&mut rng, OUTLIER_OVERSHOOT.0, OUTLIER_OVERSHOOT.1);
        if chance >= rate {
            return v;
        }
        let r = &self.quantity.range;
        let width = r.max - r.min;
        let beyond = if (v - r.min).abs() <= (r.max - v).abs() {
            r.min - overshoot * width
        } else {
            r.max + overshoot * width
        };
        self.domain.clamp(beyond)
    }
}

/// Where and which: the inputs of one evaluation.
struct Spot<'a> {
    seed: u64,
    hole: usize,
    unit: &'a str,
    x: f64,
    y: f64,
    /// Metres below the unit's top in this hole.
    z: f64,
}

#[derive(Debug, Clone, PartialEq)]
struct FineDrawn {
    plastic_limit: Drawn,
    plasticity_index: Drawn,
    liquidity_index: Drawn,
}

#[derive(Debug, Clone, PartialEq)]
struct CoarseDrawn {
    spt_n: Drawn,
    moisture_content: Drawn,
}

/// The property model of one unit or lens.
#[derive(Debug, Clone, PartialEq)]
struct UnitModel {
    id: String,
    fine: Option<FineDrawn>,
    coarse: Option<CoarseDrawn>,
    particle_density: Drawn,
    saturation: Option<Drawn>,
    /// The outlier rate over the site, when the preset gives one.
    rate: Option<(Range, Field)>,
}

impl UnitModel {
    fn new(id: &str, p: &Properties, seed: u64, extent: Extent) -> Self {
        let at = (id, seed, extent);
        let drawn = |q: &Quantity, name: &str, domain: Domain, index: Option<u64>| {
            Drawn::new(q, name, domain, index, at)
        };
        UnitModel {
            id: id.to_string(),
            fine: p.fine.as_ref().map(|f| FineDrawn {
                plastic_limit: drawn(
                    &f.plastic_limit_percent,
                    "plastic_limit_percent",
                    PLASTIC_LIMIT_PERCENT,
                    Some(Q_PLASTIC_LIMIT),
                ),
                plasticity_index: drawn(
                    &f.plasticity_index_percent,
                    "plasticity_index_percent",
                    PLASTICITY_INDEX_PERCENT,
                    Some(Q_PLASTICITY_INDEX),
                ),
                liquidity_index: drawn(
                    &f.liquidity_index,
                    "liquidity_index",
                    LIQUIDITY_INDEX,
                    Some(Q_LIQUIDITY_INDEX),
                ),
            }),
            coarse: p.coarse.as_ref().map(|c| CoarseDrawn {
                spt_n: drawn(&c.spt_n, "spt_n", SPT_N, Some(Q_SPT_N)),
                moisture_content: drawn(
                    &c.moisture_content_percent,
                    "moisture_content_percent",
                    MOISTURE_CONTENT_PERCENT,
                    Some(Q_MOISTURE_CONTENT),
                ),
            }),
            particle_density: drawn(
                &p.particle_density_ratio,
                "particle_density_ratio",
                PARTICLE_DENSITY_RATIO,
                None,
            ),
            saturation: p
                .degree_of_saturation
                .as_ref()
                .map(|q| drawn(q, "degree_of_saturation", DEGREE_OF_SATURATION, None)),
            rate: p.outlier_rate.as_ref().map(|r| {
                let field = Field::new(
                    &mut Key::unit("property:outlier_rate", id).rng(seed),
                    extent,
                );
                (r.clone(), field)
            }),
        }
    }

    fn outlier_rate(&self, x: f64, y: f64) -> f64 {
        match &self.rate {
            Some((range, field)) => OUTLIER_RATE.clamp(field.in_range(range, x, y)),
            None => OUTLIER_RATE_DEFAULT,
        }
    }
}

/// The site's property model: one [`UnitModel`] per unit or lens that carries
/// properties, plus the water table.
#[derive(Debug, Clone, PartialEq)]
pub struct PropertyModel {
    seed: u64,
    groundwater: Option<(Range, Field)>,
    units: Vec<UnitModel>,
}

/// The index properties and phase relations at one depth in one hole.
///
/// Percentages are percent (30.0 is 30 %). Fine units fill `fine`, coarse
/// units fill `spt_n`, and a unit with both principals fills both, taking
/// its moisture content from the fine set.
#[derive(Debug, Clone, PartialEq)]
#[allow(dead_code)] // read by tests only until the description term and results use it (#41)
pub struct Evaluated {
    pub fine: Option<FineValues>,
    /// Whole blows.
    pub spt_n: Option<u32>,
    pub moisture_content_percent: f64,
    pub particle_density_ratio: f64,
    pub degree_of_saturation: f64,
    pub bulk_density_kg_m3: f64,
    pub unit_weight_kn_m3: f64,
}

/// The fine-soil set: PL, PI and LI are drawn, the rest derived from them.
#[derive(Debug, Clone, PartialEq)]
#[allow(dead_code)] // read by tests only until the description term and results use it (#41)
pub struct FineValues {
    pub plastic_limit_percent: f64,
    pub plasticity_index_percent: f64,
    pub liquidity_index: f64,
    /// LL = PL + PI.
    pub liquid_limit_percent: f64,
    /// Ic = 1 - LI, through the catalogue (LL - w) / PI.
    pub consistency_index: f64,
}

/// Stresses at a depth in a hole, kPa.
#[derive(Debug, Clone, Copy, PartialEq)]
#[allow(dead_code)] // read by tests only until the description term and results use it (#41)
#[allow(clippy::struct_field_names)] // the unit is part of each name
pub struct Stress {
    /// Total vertical stress sigma-v.
    pub total_kpa: f64,
    /// Hydrostatic pore pressure u: zero above the water table.
    pub pore_pressure_kpa: f64,
    /// Effective vertical stress sigma-v - u.
    pub effective_kpa: f64,
}

impl PropertyModel {
    pub(super) fn new(preset: &Preset, seed: u64) -> Self {
        let extent = preset.site.extent_m;
        let units = preset
            .units
            .iter()
            .map(|u| (u.id.as_str(), &u.properties))
            .chain(preset.lenses.iter().map(|l| (l.id.as_str(), &l.properties)))
            .filter_map(|(id, p)| Some(UnitModel::new(id, p.as_ref()?, seed, extent)))
            .collect();
        PropertyModel {
            seed,
            groundwater: preset.site.groundwater_depth_m.as_ref().map(|r| {
                (
                    r.clone(),
                    Field::new(&mut Key::site("groundwater").rng(seed), extent),
                )
            }),
            units,
        }
    }

    /// The water table's depth below ground at `hole`, if the preset has one.
    pub fn groundwater_depth(&self, hole: &Hole) -> Option<f64> {
        let (range, field) = self.groundwater.as_ref()?;
        Some(GROUNDWATER_DEPTH_M.clamp(field.in_range(range, hole.x, hole.y)))
    }

    /// The quantities of `unit` at `depth` in hole number `index`. `None` when
    /// the unit has no properties or is not in the hole.
    pub fn evaluate(&self, index: usize, hole: &Hole, unit: &str, depth: f64) -> Option<Evaluated> {
        let model = self.units.iter().find(|u| u.id == unit)?;
        // Depth below the unit's top in this hole, from its first stratum so
        // a host split by a lens keeps one trend.
        let top = hole.strata.iter().find(|s| s.unit == unit)?.top;
        let spot = Spot {
            seed: self.seed,
            hole: index,
            unit,
            x: hole.x,
            y: hole.y,
            z: (depth - top).max(0.0),
        };
        let rate = model.outlier_rate(hole.x, hole.y);
        let gs = model.particle_density.smooth(spot.x, spot.y, spot.z);

        let fine = model.fine.as_ref().map(|f| {
            let pl = f.plastic_limit.value(&spot, rate);
            let pi = f.plasticity_index.value(&spot, rate);
            // w = PL + LI*PI must not go below zero.
            let li = f.liquidity_index.value(&spot, rate).max(-pl / pi);
            (pl, pi, li)
        });
        let spt_n = model.coarse.as_ref().map(|c| {
            let n = c.spt_n.value(&spot, rate).round();
            // Clamped at zero and rounded, so a whole non-negative count.
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let n = n as u32;
            n
        });
        let w_percent = match (&fine, &model.coarse) {
            (Some((pl, pi, li)), _) => (pl + li * pi).max(0.0),
            (None, Some(c)) => c.moisture_content.value(&spot, rate),
            (None, None) => unreachable!("the loader requires a property set"),
        };

        let below_table = self
            .groundwater_depth(hole)
            .is_some_and(|table| depth > table);
        let sr = if below_table {
            1.0
        } else {
            model
                .saturation
                .as_ref()
                .map_or(1.0, |s| s.smooth(spot.x, spot.y, spot.z))
        };
        let density = catalogue::bulk_density_kg_m3(w_percent / 100.0, gs, sr, WATER_DENSITY_KG_M3)
            .expect("every input is inside its domain");
        let weight = catalogue::unit_weight_kn_m3(density).expect("density is positive");

        let fine = fine.map(|(pl, pi, li)| {
            let ll = pl + pi;
            FineValues {
                plastic_limit_percent: pl,
                plasticity_index_percent: pi,
                liquidity_index: li,
                liquid_limit_percent: ll,
                consistency_index: catalogue::consistency_index(ll, pl, w_percent)
                    .expect("PL, LL and w are non-negative")
                    .expect("PI is positive"),
            }
        });
        Some(Evaluated {
            fine,
            spt_n,
            moisture_content_percent: w_percent,
            particle_density_ratio: gs,
            degree_of_saturation: sr,
            bulk_density_kg_m3: density,
            unit_weight_kn_m3: weight,
        })
    }
}

#[allow(dead_code)] // used by tests only until the description term and results use it (#41)
impl Site {
    /// The properties at `depth` in hole number `hole`, in the stratum there
    /// (the lower one on a boundary). `None` outside the hole or where the
    /// unit has none.
    pub fn properties_at(&self, hole: usize, depth: f64) -> Option<Evaluated> {
        let h = self.holes.get(hole)?;
        let stratum = h
            .strata
            .iter()
            .find(|s| depth >= s.top && depth < s.base)
            .or_else(|| {
                h.strata
                    .last()
                    .filter(|s| depth >= s.top && depth <= s.base)
            })?;
        self.properties.evaluate(hole, h, &stratum.unit, depth)
    }

    /// Total vertical stress, pore pressure and effective stress at `depth` in
    /// hole number `hole`.
    ///
    /// sigma-v sums unit weight x thickness over the strata above `depth`
    /// (only the part above `depth` for the stratum that contains it), each
    /// stratum's unit weight taken at its own mid-depth. u is hydrostatic
    /// below the water table and zero above it. `None` outside the hole, or
    /// when a stratum above has no properties.
    pub fn stress_at(&self, hole: usize, depth: f64) -> Option<Stress> {
        let h = self.holes.get(hole)?;
        if !(0.0..=h.final_depth).contains(&depth) {
            return None;
        }
        let mut total = 0.0;
        for s in h.strata.iter().take_while(|s| s.top < depth) {
            let mid = f64::midpoint(s.top, s.base);
            let e = self.properties.evaluate(hole, h, &s.unit, mid)?;
            total += e.unit_weight_kn_m3 * (s.base.min(depth) - s.top);
        }
        let water_weight =
            catalogue::unit_weight_kn_m3(WATER_DENSITY_KG_M3).expect("water density is positive");
        let pore = self
            .properties
            .groundwater_depth(h)
            .map_or(0.0, |table| (depth - table).max(0.0) * water_weight);
        Some(Stress {
            total_kpa: total,
            pore_pressure_kpa: pore,
            effective_kpa: total - pore,
        })
    }
}

// The expected stresses and the clamps are exact literals or exact endpoints.
#[allow(clippy::float_cmp)]
#[cfg(test)]
mod tests {
    use super::*;
    use crate::site::build::build;
    use crate::site::draw::Key;

    #[test]
    fn domains_contain_and_clamp() {
        assert!(PLASTIC_LIMIT_PERCENT.contains(0.001));
        assert!(!PLASTIC_LIMIT_PERCENT.contains(0.0));
        assert!(LIQUIDITY_INDEX.contains(-1.0) && LIQUIDITY_INDEX.contains(3.0));
        assert!(!LIQUIDITY_INDEX.contains(3.1));
        assert!(!DEGREE_OF_SATURATION.contains(1.1));
        assert!(PARTICLE_DENSITY_RATIO.contains(4.0) && !PARTICLE_DENSITY_RATIO.contains(1.0));
        assert!(!SPT_N.contains(-0.5) && !SPT_N.contains(f64::NAN));
        assert_eq!(LIQUIDITY_INDEX.clamp(9.0), 3.0);
        assert_eq!(SPT_N.clamp(-4.0), 0.0);
        assert!(PLASTIC_LIMIT_PERCENT.clamp(-4.0) > 0.0);
        assert_eq!(DEGREE_OF_SATURATION.describe(), "> 0 and <= 1");
        assert_eq!(SPT_N.describe(), ">= 0");
    }

    /// A site of one coarse unit over a fine one. `@` tokens are replaced.
    const BASE: &str = r#"
schema = 1
name = "prop-test"
version = 1
summary = "A fixture."

[site]
extent_m = { x = 400.0, y = 300.0 }
ground_level_m = { min = 8.0, max = 22.0, basis = "test fixture" }
@GW@

[[hole_types]]
code = "CP"
weight = 1
final_depth_m = { min = 20.0, max = 20.0, basis = "test fixture" }

[[units]]
id = "RTD"
name = "River Terrace Deposits"
thickness_m = { min = 2.0, max = 2.0, basis = "test fixture" }
description = { principals = ["SAND"], secondaries = ["gravel"], colours = ["brown"] }
sampling = { types = ["B"], interval_m = { min = 1.0, max = 1.5, basis = "test fixture" } }

[units.properties]
spt_n = { min = 10, max = 10, basis = "test fixture" }
moisture_content_percent = { min = 20, max = 20, basis = "test fixture" }
particle_density_ratio = { min = 2.5, max = 2.5, basis = "test fixture" }
degree_of_saturation = { min = 0.5, max = 0.5, basis = "test fixture" }
@RTD@

[[units]]
id = "LC"
name = "London Clay"
description = { principals = ["CLAY"], secondaries = ["silt"], colours = ["grey"] }
sampling = { types = ["U"], interval_m = { min = 1.5, max = 3.0, basis = "test fixture" } }

[units.properties]
plastic_limit_percent = { min = 30, max = 30, basis = "test fixture" }
plasticity_index_percent = { min = 40, max = 40, basis = "test fixture" }
liquidity_index = { min = 0.75, max = 0.75, basis = "test fixture" }
particle_density_ratio = { min = 2.5, max = 2.5, basis = "test fixture" }
@LC@
"#;

    fn load(gw: &str, rtd: &str, lc: &str) -> Preset {
        let text = BASE
            .replace("@GW@", gw)
            .replace("@RTD@", rtd)
            .replace("@LC@", lc);
        Preset::from_toml(&text).unwrap_or_else(|e| panic!("{e}\n{text}"))
    }

    /// A varied fixture: every fine and coarse quantity has a real range, so
    /// fields and outliers show.
    fn varied(extra: &str) -> Preset {
        let text = BASE
            .replace(
                "@GW@",
                "groundwater_depth_m = { min = 1.0, max = 6.0, basis = \"test fixture\" }",
            )
            .replace("@RTD@", extra)
            .replace("@LC@", extra)
            .replace(
                "spt_n = { min = 10, max = 10,",
                "spt_n = { min = 10, max = 40,",
            )
            .replace(
                "moisture_content_percent = { min = 20, max = 20,",
                "moisture_content_percent = { min = 10, max = 25,",
            )
            .replace(
                "plastic_limit_percent = { min = 30, max = 30,",
                "plastic_limit_percent = { min = 20, max = 30,",
            )
            .replace(
                "plasticity_index_percent = { min = 40, max = 40,",
                "plasticity_index_percent = { min = 30, max = 50,",
            )
            .replace(
                "liquidity_index = { min = 0.75, max = 0.75,",
                "liquidity_index = { min = 0.0, max = 1.0,",
            );
        Preset::from_toml(&text).unwrap_or_else(|e| panic!("{e}\n{text}"))
    }

    const NO_WATER: &str = "";
    const RATE_HALF: &str = "outlier_rate = { min = 0.5, max = 0.5, basis = \"test fixture\" }";

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() <= 1e-9 * (1.0 + b.abs())
    }

    /// Worked example, by hand. Gs = 2.5 throughout, water 9.80665 kN/m3
    /// (1000 kg/m3 x standard gravity).
    ///
    /// RTD, 0-2 m, above the 1.5 m table at its mid-depth (1 m), Sr = 0.5,
    /// w = 20 %: e = 0.2 x 2.5 / 0.5 = 1, rho = 2.5 x 1000 x 1.2 / 2
    /// = 1500 kg/m3, gamma = 14.709975 kN/m3.
    /// LC, 2-20 m, mid-depth 11 m below the table, Sr = 1, w = 30 + 0.75 x 40
    /// = 60 %: e = 0.6 x 2.5 = 1.5, rho = 2.5 x 1000 x 1.6 / 2.5 = 1600
    /// kg/m3, gamma = 1.6 x 9.80665 = 15.69064 kN/m3.
    ///
    /// At 4 m: sigma-v = 2 x 14.709975 + 2 x 15.69064 = 29.41995 + 31.38128
    /// = 60.80123; u = (4 - 1.5) x 9.80665 = 24.516625;
    /// sigma'-v = 60.80123 - 24.516625 = 36.284605.
    /// At 3 m: sigma-v = 29.41995 + 15.69064 = 45.11059; u = 1.5 x 9.80665
    /// = 14.709975; sigma'-v = 45.11059 - 14.709975 = 30.400615.
    /// At 1 m, above the table: sigma-v = 14.709975, u = 0.
    #[test]
    fn a_two_stratum_hole_gives_the_hand_computed_stresses() {
        let p = load(
            "groundwater_depth_m = { min = 1.5, max = 1.5, basis = \"test fixture\" }",
            NO_WATER,
            NO_WATER,
        );
        let site = build(&p, 3, 2);
        for hole in 0..2 {
            for (depth, total, pore, effective) in [
                (1.0, 14.709_975, 0.0, 14.709_975),
                (3.0, 45.110_59, 14.709_975, 30.400_615),
                (4.0, 60.801_23, 24.516_625, 36.284_605),
            ] {
                let s = site.stress_at(hole, depth).expect("inside the hole");
                assert!(close(s.total_kpa, total), "{depth}: {s:?}");
                assert!(close(s.pore_pressure_kpa, pore), "{depth}: {s:?}");
                assert!(close(s.effective_kpa, effective), "{depth}: {s:?}");
            }
        }
    }

    #[test]
    fn stress_is_none_outside_the_hole_or_without_properties() {
        let p = load(NO_WATER, NO_WATER, NO_WATER);
        let site = build(&p, 3, 1);
        assert!(site.stress_at(0, -1.0).is_none());
        assert!(site.stress_at(0, 20.5).is_none());
        assert!(site.stress_at(1, 1.0).is_none());
        // No water table: u is zero everywhere and sigma' = sigma.
        let s = site.stress_at(0, 4.0).unwrap();
        assert_eq!(s.pore_pressure_kpa, 0.0);
        assert_eq!(s.effective_kpa, s.total_kpa);
        // A preset without properties has nothing to answer with.
        let bare = crate::site::preset::resolve("london-basin").unwrap();
        let site = build(&bare, 3, 1);
        assert!(site.properties_at(0, 1.0).is_none());
        assert!(site.stress_at(0, 4.0).is_none());
    }

    #[test]
    fn evaluation_gives_the_worked_density_and_derived_values() {
        let p = load(
            "groundwater_depth_m = { min = 1.5, max = 1.5, basis = \"test fixture\" }",
            NO_WATER,
            NO_WATER,
        );
        let site = build(&p, 3, 1);
        let rtd = site.properties_at(0, 1.0).unwrap();
        assert_eq!(rtd.spt_n, Some(10));
        assert!(rtd.fine.is_none());
        assert!(close(rtd.degree_of_saturation, 0.5));
        assert!(close(rtd.bulk_density_kg_m3, 1500.0), "{rtd:?}");
        assert!(close(rtd.unit_weight_kn_m3, 14.709_975));
        let lc = site.properties_at(0, 10.0).unwrap();
        let f = lc.fine.as_ref().unwrap();
        assert!(close(f.liquid_limit_percent, 70.0));
        assert!(close(lc.moisture_content_percent, 60.0));
        assert!(close(f.consistency_index, 0.25));
        assert!(close(lc.degree_of_saturation, 1.0));
        assert!(close(lc.bulk_density_kg_m3, 1600.0));
    }

    /// PL < LL, PI = LL - PL, w = PL + LI x PI and Ic = 1 - LI, on every
    /// seed, hole and depth, with outliers on half of all draws.
    #[test]
    fn couplings_hold_everywhere_outliers_included() {
        let p = varied(RATE_HALF);
        let mut outliers = 0;
        for seed in 0..8 {
            let site = build(&p, seed, 12);
            for hole in 0..12 {
                for depth in [2.5, 6.0, 11.0, 19.9] {
                    let e = site.properties_at(hole, depth).unwrap();
                    let f = e.fine.as_ref().unwrap();
                    assert!(f.plastic_limit_percent < f.liquid_limit_percent);
                    assert!(close(
                        f.plasticity_index_percent,
                        f.liquid_limit_percent - f.plastic_limit_percent
                    ));
                    assert!(close(
                        e.moisture_content_percent,
                        f.plastic_limit_percent + f.liquidity_index * f.plasticity_index_percent
                    ));
                    assert!(close(f.consistency_index, 1.0 - f.liquidity_index));
                    assert!(e.moisture_content_percent >= 0.0);
                    outliers += usize::from(!(20.0..=30.0).contains(&f.plastic_limit_percent));
                }
            }
        }
        assert!(outliers > 50, "the test saw only {outliers} outliers");
    }

    fn rate(r: f64) -> String {
        format!("outlier_rate = {{ min = {r}, max = {r}, basis = \"test fixture\" }}")
    }

    /// Plastic limit of the fine unit at `depth` in `hole`.
    fn pl_at(site: &Site, hole: usize, depth: f64) -> f64 {
        site.properties_at(hole, depth)
            .unwrap()
            .fine
            .unwrap()
            .plastic_limit_percent
    }

    /// The fine unit (2-20 m) at 10 m.
    fn pl(site: &Site, hole: usize) -> f64 {
        pl_at(site, hole, 10.0)
    }

    /// Neighbouring holes agree: with outliers off, the plastic limit differs
    /// by no more than the width of its range times the field's slope bound
    /// times the distance.
    #[test]
    fn neighbouring_holes_have_similar_properties() {
        let p = varied(&rate(0.0));
        for seed in 0..10 {
            let field = Field::new(
                &mut Key::unit("property:plastic_limit_percent", "LC").rng(seed),
                p.site.extent_m,
            );
            let site = build(&p, seed, 30);
            for (i, pair) in site.holes.windows(2).enumerate() {
                let d = (pair[0].x - pair[1].x).hypot(pair[0].y - pair[1].y);
                let bound = 10.0 * field.max_slope() * d + 1e-9;
                let diff = (pl(&site, i) - pl(&site, i + 1)).abs();
                assert!(
                    diff <= bound,
                    "seed {seed}: {diff} over {d} m (bound {bound})"
                );
            }
        }
    }

    fn with_gradient(per_m: f64) -> Preset {
        let text = BASE
            .replace("@GW@", "")
            .replace("@RTD@", "")
            .replace("@LC@", "")
            .replace(
                "plastic_limit_percent = { min = 30, max = 30, basis = \"test fixture\" }",
                &format!(
                    "plastic_limit_percent = {{ min = 30, max = 30, basis = \"test fixture\", \
                     per_m = {{ min = {per_m}, max = {per_m}, basis = \"test fixture\" }} }}"
                ),
            );
        Preset::from_toml(&text).unwrap_or_else(|e| panic!("{e}"))
    }

    /// A cited gradient shows: 4 m deeper is 4 x gradient more, past the
    /// range's own bound; and when the trend would leave physics the value
    /// stops at the domain, not the range.
    #[test]
    fn a_gradient_shows_and_clamps_to_the_domain() {
        // The fine unit starts at 2 m, so these depths are z = 3 and z = 7.
        let site = build(&with_gradient(0.5), 4, 3);
        for hole in 0..3 {
            let z0 = pl_at(&site, hole, 5.0);
            let z1 = pl_at(&site, hole, 9.0);
            assert!((z1 - z0 - 0.5 * 4.0).abs() < 1e-9, "{z0} {z1}");
            assert!(z1 > 30.0, "the gradient is not limited to the range");
        }
        // 30 - 5 x 3 = 15 at 5 m; 30 - 5 x 17 < 0 at 19 m.
        let steep = build(&with_gradient(-5.0), 4, 1);
        assert!(close(pl_at(&steep, 0, 5.0), 15.0));
        let deep = pl_at(&steep, 0, 19.0);
        assert!(deep > 0.0 && deep < 1e-3, "{deep}");
    }

    fn outlier_share(p: &Preset, holes: usize) -> f64 {
        let site = build(p, 11, holes);
        let n = (0..holes)
            .filter(|&h| !(20.0..=30.0).contains(&pl(&site, h)))
            .count();
        n as f64 / holes as f64
    }

    #[test]
    fn the_outlier_rate_matches_its_setting() {
        let share = outlier_share(&varied(&rate(0.1)), 2000);
        assert!((share - 0.1).abs() < 0.03, "{share}");
        // No rate given: the default.
        let share = outlier_share(&varied(""), 3000);
        assert!((share - OUTLIER_RATE_DEFAULT).abs() < 0.01, "{share}");
    }

    #[test]
    fn with_a_zero_rate_every_value_is_in_its_range() {
        let p = varied(&rate(0.0));
        for seed in 0..4 {
            let site = build(&p, seed, 100);
            for h in 0..100 {
                let rtd = site.properties_at(h, 1.0).unwrap();
                assert!((10..=40).contains(&rtd.spt_n.unwrap()), "{rtd:?}");
                assert!((10.0..=25.0).contains(&rtd.moisture_content_percent));
                let f = site.properties_at(h, 10.0).unwrap().fine.unwrap();
                assert!((20.0..=30.0).contains(&f.plastic_limit_percent));
                assert!((30.0..=50.0).contains(&f.plasticity_index_percent));
                assert!((0.0..=1.0).contains(&f.liquidity_index));
            }
        }
    }

    /// Raising the rate turns some values into outliers and moves nothing
    /// else: every value is either unchanged or an outlier in the higher-rate
    /// site that was in range at the lower rate.
    #[test]
    fn changing_the_rate_moves_only_the_values_whose_status_changes() {
        let (low, high) = (varied(&rate(0.05)), varied(&rate(0.3)));
        let (a, b) = (build(&low, 5, 1000), build(&high, 5, 1000));
        let (mut moved, mut same) = (0, 0);
        for h in 0..1000 {
            let (x, y) = (pl(&a, h), pl(&b, h));
            if (x - y).abs() < 1e-12 {
                same += 1;
            } else {
                moved += 1;
                assert!(!(20.0..=30.0).contains(&y), "hole {h}: {x} became {y}");
                assert!((20.0..=30.0).contains(&x), "hole {h}: an outlier moved");
            }
        }
        assert!(moved > 100 && same > 500, "{moved} moved, {same} same");
    }

    /// An outlier is beyond the nearer bound by 25-75 % of the width.
    #[test]
    fn an_outlier_overshoots_the_nearer_bound_by_a_quarter_to_three_quarters() {
        let site = build(&varied(&rate(1.0)), 2, 200);
        for h in 0..200 {
            let v = pl(&site, h);
            let beyond = if v < 25.0 { 20.0 - v } else { v - 30.0 };
            assert!((2.5 - 1e-9..=7.5 + 1e-9).contains(&beyond), "hole {h}: {v}");
        }
    }
}
