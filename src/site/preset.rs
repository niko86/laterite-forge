//! Preset format v1: the site's story, loaded and checked before anything
//! generates from it.
//!
//! The loader walks the parsed TOML by hand instead of deriving `Deserialize`,
//! so one pass reports every problem and each names its field. Unknown keys are
//! refused so a typo cannot silently drop a field.

use std::fmt;
use std::path::Path;

use laterite_ags4_validator::{DictVersion, Dictionary};
use toml::Table;
use toml::Value;

use super::cover;
use super::properties::{self as props, Domain};
use super::results;
use super::suite;
use crate::synth::bs5930::{Content, TopsoilSpec, VeryCoarse};
use crate::synth::made_ground;

/// The only schema this forge reads.
const SCHEMA: i64 = 1;

/// Presets shipped inside the binary as `(name, toml)`.
pub(crate) const SHIPPED: &[(&str, &str)] = &[
    (
        "london-basin",
        include_str!("../../data/presets/london-basin.toml"),
    ),
    (
        "tyneside-till",
        include_str!("../../data/presets/tyneside-till.toml"),
    ),
];

#[derive(Debug, Clone, PartialEq)]
pub struct Preset {
    pub name: String,
    pub version: u32,
    pub summary: String,
    pub site: Site,
    pub hole_types: Vec<HoleType>,
    /// Top-down order.
    pub units: Vec<Unit>,
    /// Empty when the preset has none.
    pub lenses: Vec<Lens>,
    /// The lab and in-situ test groups the file carries; empty when the preset
    /// has none.
    pub suite: Vec<SuiteEntry>,
    /// The topsoil and hard surfacing that can cap a hole; `None` when the
    /// preset has none (ADR 0001, "Cover layer").
    pub cover: Option<Cover>,
    /// How each hole was put down: inspection pit, casing, water-strike
    /// casing and backfill; `None` when the preset has none (ADR 0001, "Hole
    /// construction").
    pub construction: Option<Construction>,
}

/// The cover layer: what lies on the ground before the first stratum. Each
/// hole draws surfacing, topsoil or nothing, and the cover takes its thickness
/// off the top of the hole's first stratum, so nothing below it moves.
#[derive(Debug, Clone, PartialEq)]
pub struct Cover {
    pub surfacing: Option<Surfacing>,
    pub topsoil: Option<Topsoil>,
}

/// Hard surfacing: a `MADE GROUND: <kind>.` stratum with no samples.
#[derive(Debug, Clone, PartialEq)]
pub struct Surfacing {
    /// The share of holes that start with surfacing, drawn once per site.
    pub share: Range,
    /// Per-hole thickness: `min + (max - min) * u^skew` for a uniform `u`.
    pub thickness_m: Range,
    /// Each kind is one of [`cover::KINDS`], with its relative weight.
    pub kinds: Vec<SurfacingKind>,
    pub kinds_basis: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SurfacingKind {
    pub name: String,
    pub weight: f64,
}

/// Topsoil: a `<colour> <fractions> TOPSOIL with rootlets` stratum with no
/// samples.
#[derive(Debug, Clone, PartialEq)]
pub struct Topsoil {
    /// The share of holes that start with topsoil, drawn once per site.
    pub share: Range,
    pub thickness_m: Range,
    pub description: TopsoilSpec,
    pub description_basis: String,
}

/// Hole construction for holes with no installation: the `HDPH` phases, the
/// `CDIA` casing, the casing depth at the strike (`WSTG_CAS`) and the `BKFL`
/// backfill. A hole is a hand-dug inspection pit from the surface to
/// `inspection_pit_m`, then its own boring method to final depth.
#[derive(Debug, Clone, PartialEq)]
pub struct Construction {
    /// Depth of the inspection pit, drawn per hole.
    pub inspection_pit_m: Range,
    pub casing: Casing,
    pub backfill: Backfill,
    /// Installations (`PIPE`, and the backfill an installed hole carries);
    /// `None` when the preset writes none (#75).
    pub installation: Option<Installation>,
    /// `WSTG_SEAL` on strikes; `None` writes no such column (#79).
    pub seal: Option<Seal>,
}

/// The seal of a strike (`WSTG_SEAL`): the depth to which the hole is sealed
/// after the strike. A strike is sealed at `earlier_share` when another
/// strike lies below it and at `last_share` when it is the hole's last.
#[derive(Debug, Clone, PartialEq)]
pub struct Seal {
    pub earlier_share: f64,
    pub last_share: f64,
    /// Seal depth less strike depth, metres, per sealed strike.
    pub gap_m: Range,
    /// The source of the two shares.
    pub basis: String,
}

/// The installations a site's holes may carry: a plain riser over a slotted
/// section, with the backfill of an installed hole. A site draws once whether
/// it installs at all and, if so, what share of its holes; each hole then
/// draws against that share. One installation per hole.
#[derive(Debug, Clone, PartialEq)]
pub struct Installation {
    /// The share of sites that install at all; each site draws once.
    pub site_share: Range,
    /// On a site that installs, the share of its holes that are installed;
    /// drawn once per site.
    pub hole_share: Range,
    /// The kinds an installation can be, with relative weights.
    pub kinds: Vec<InstallKind>,
    pub kinds_basis: String,
    /// Zone top less the first strike's depth, metres (negative: the zone
    /// starts above the strike), for a hole with a strike.
    pub zone_offset_m: Range,
    /// Zone top, metres, for a hole with no strike.
    pub zone_top_m: Range,
    /// In a hole with several strikes, the share whose zone follows the first
    /// strike (the rest follow the deepest); drawn once per site. `None`: the
    /// zone follows the first strike (#79).
    pub zone_follows_first_share: Option<Range>,
    /// Length of the slotted section of a standpipe, metres.
    pub slotted_length_m: Range,
    /// Final depth less the base of the slotted section, metres, before the
    /// hole's own bounds are applied.
    pub base_gap_m: Range,
    /// Thickness of the concrete cap, metres.
    pub cap_m: Range,
    /// Relative weights of a gravel and a sand filter.
    pub filter_weights: [f64; 2],
    pub filter_basis: String,
}

/// What an installation is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstallKindName {
    /// A standpipe: plain from the surface to the response zone, slotted
    /// through it.
    Standpipe,
    /// A standpipe piezometer: plain, then a short tip written as slotted.
    Piezometer,
}

#[derive(Debug, Clone, PartialEq)]
pub struct InstallKind {
    pub kind: InstallKindName,
    /// `PIPE_DIAM`, millimetres.
    pub mm: u32,
    pub weight: f64,
}

/// The `PIPE_DIAM` values an installation may have (millimetres).
pub const INSTALL_DIAMETERS_MM: [u32; 2] = [19, 50];

/// The casing string(s) and how they relate to the water strike.
#[derive(Debug, Clone, PartialEq)]
pub struct Casing {
    /// The diameters a string can have, with relative weights.
    pub diameters: Vec<CasingDiameter>,
    pub diameters_basis: String,
    /// The share of holes that telescope (a larger string to part depth, then
    /// a smaller one) among those whose larger string has a smaller diameter
    /// to step down to; drawn once per site.
    pub telescope_share: Range,
    /// Where the larger string ends, as a fraction of the way from the
    /// inspection pit's base to the final casing depth.
    pub telescope_depth_fraction: Range,
    /// Final depth less final casing depth, metres, per hole.
    pub final_gap_m: Range,
    /// The share of holes whose final casing is deeper than the strike; drawn
    /// once per site.
    pub passes_strike_share: Range,
    /// Strike depth less the casing depth at the strike (`WSTG_CAS`), metres,
    /// per strike.
    pub strike_gap_m: Range,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CasingDiameter {
    pub mm: u32,
    pub weight: f64,
}

/// Backfill of a hole with no installation: arisings, bentonite, or arisings
/// over bentonite, from 0 to final depth.
#[derive(Debug, Clone, PartialEq)]
pub struct Backfill {
    /// The share of sites whose holes carry `BKFL` at all; each site draws
    /// once whether they do.
    pub site_share: Range,
    /// Relative weights of arisings alone, bentonite alone and arisings over
    /// bentonite.
    pub sequence_weights: [f64; 3],
    pub sequence_basis: String,
    /// Depth of the arisings/bentonite boundary where there is one, per hole.
    pub boundary_m: Range,
}

// Every field carries its unit as the TOML key does (`_m`).
#[allow(clippy::struct_field_names)]
#[derive(Debug, Clone, PartialEq)]
pub struct Site {
    pub extent_m: Extent,
    pub ground_level_m: Range,
    /// The water table's depth below ground level, metres.
    pub groundwater_depth_m: Range,
    /// The strike rise, metres: how far below the water table the bit is when
    /// the water is noticed (drilling lag). Absent means forge's own 0-0.5 m
    /// default.
    pub strike_rise_m: Option<Range>,
    /// `Some` lets a hole record several strikes (#79); absent, a hole
    /// records at most one.
    pub several_strikes: Option<SeveralStrikes>,
}

/// The several-strikes rule: a hole records every strike its sources give,
/// shallowest first, up to a cap (#79).
#[derive(Debug, Clone, PartialEq)]
pub struct SeveralStrikes {
    /// The most strikes a hole records; deeper ones are dropped.
    pub max_per_hole: u32,
    /// The least depth between consecutive strikes, metres: a later strike
    /// closer than this is pushed down to it (or dropped when its stratum has
    /// no room).
    pub min_gap_m: f64,
    pub basis: String,
}

/// Size of the local grid, whose origin is (0, 0).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Extent {
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct HoleType {
    /// A `LOCA_TYPE` code.
    pub code: String,
    /// Relative frequency.
    pub weight: u32,
    pub final_depth_m: Range,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Unit {
    pub id: String,
    pub name: String,
    /// `None` only on the last unit, which runs to final depth.
    pub thickness_m: Option<Range>,
    pub description: DescriptionSpec,
    /// `Some` makes this a made-ground unit: its strata are described
    /// `MADE GROUND: ` + matrix + constituents, and it takes no lab test. It
    /// never yields a water strike by the table rule or as water-bearing; a
    /// `strike` block (london-basin v9) gives it perched strikes.
    pub made_ground: Option<MadeGroundSpec>,
    /// `Some` makes the unit water-bearing: a hole that meets it strikes
    /// water at its top with probability `share`, drawn once per site in
    /// the range (ADR 0001, "Water-bearing strata"). `true` is share 1.
    /// Never set on a made-ground unit.
    pub water_bearing: Option<Range>,
    /// `Some` makes the unit strike water along its penetration: a hole that
    /// goes through it meets water at a Poisson-distributed depth below the
    /// unit's top (ADR 0001, "Water strikes in till" and "Water strikes in
    /// made ground"). Never set on a water-bearing unit.
    pub strike: Option<UnitStrike>,
    pub sampling: Sampling,
    /// Index properties (required).
    pub properties: Properties,
}

/// The water strikes of a unit that yields water along its depth (a clay
/// till whose strikes come from partings too thin to log).
#[derive(Debug, Clone, PartialEq)]
pub struct UnitStrike {
    /// Strikes per metre penetrated: one rate, or a rate per depth band.
    pub rate: StrikeRate,
    /// How far the water rises above the strike where the water table is
    /// above it, metres; the optional skew is as for a thickness.
    pub rise_m: Range,
    /// The fraction of strikes whose water does not rise at all.
    pub no_rise_share: Range,
}

/// The rate of a unit's strike process.
#[derive(Debug, Clone, PartialEq)]
pub enum StrikeRate {
    /// `rate_per_m`: one rate for the whole unit, drawn once per site.
    PerMetre(Range),
    /// `rate_by_depth`: a rate for each band of depth below ground level,
    /// ascending, the first from 0 and the last running on without end.
    ByDepth(Vec<RateBand>),
}

/// One band of a `rate_by_depth`: the rate from `from_m` down to the next
/// band's `from_m`, drawn once per site.
#[derive(Debug, Clone, PartialEq)]
pub struct RateBand {
    /// Depth below ground level at which the band starts, metres.
    pub from_m: f64,
    /// Strikes per metre penetrated in the band.
    pub rate_per_m: Range,
}

impl StrikeRate {
    /// Every range the rate is drawn from, in order.
    #[cfg(test)]
    pub fn ranges(&self) -> Vec<&Range> {
        match self {
            Self::PerMetre(r) => vec![r],
            Self::ByDepth(bands) => bands.iter().map(|b| &b.rate_per_m).collect(),
        }
    }
}

/// How a made-ground unit names its constituents. Both parts are optional:
/// without `constituents` the unit draws from the lane's default list with
/// equal weight, and without `count` a stratum names one or two, as the
/// default lane does. `made_ground = true` is both absent.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct MadeGroundSpec {
    pub constituents: Option<Vec<ConstituentShare>>,
    pub count: Option<CountWeights>,
    pub amounts: Option<AmountWeights>,
}

/// Weights for the amount word each constituent takes, independently.
#[derive(Debug, Clone, PartialEq)]
pub struct AmountWeights {
    /// Weights for rare, occasional, frequent, numerous and abundant.
    pub weights: [f64; 5],
    pub basis: String,
}

/// One constituent and the fraction of the unit's strata that mention it.
/// The site draws one value in the range, once, so a site is consistent.
#[derive(Debug, Clone, PartialEq)]
pub struct ConstituentShare {
    /// A word of the made-ground vocabulary.
    pub name: String,
    /// Fractions in `[0, 1]`; a plain number in the file has `min == max`.
    pub share: Range,
}

/// Weights for how many constituents a stratum names.
#[derive(Debug, Clone, PartialEq)]
pub struct CountWeights {
    /// Weights for 0, 1, 2 and 3 or more constituents; at least one is
    /// positive, none negative. Relative, so they need not sum to 1.
    pub weights: [f64; 4],
    pub basis: String,
}

/// A unit with a limited, lenticular footprint inside one host unit. In plan
/// it is a disc of radius R; its thickness is `t_max` at the centre and tapers
/// to nothing at the edge. Each site draws the centre, R, `t_max` and depth
/// fraction once from the ranges here.
#[derive(Debug, Clone, PartialEq)]
pub struct Lens {
    /// Unique across units and lenses.
    pub id: String,
    pub name: String,
    /// The id of a unit (never a lens).
    pub host: String,
    /// Plan radius R, metres; min > 0.
    pub radius_m: Range,
    /// Thickness at the centre, metres; min > 0.
    pub thickness_m: Range,
    /// Mid-depth of the lens as a fraction of the host's thickness at the
    /// hole, within `[0, 1]`.
    pub depth_in_host: Range,
    pub description: DescriptionSpec,
    /// As [`Unit::water_bearing`].
    pub water_bearing: Option<Range>,
    pub sampling: Sampling,
    /// Index properties (required).
    pub properties: Properties,
}

/// One test group of the preset's suite: which units it applies to and how
/// often (ADR 0001, "Output and determinism").
#[derive(Debug, Clone, PartialEq)]
pub struct SuiteEntry {
    /// An AGS group, one of [`SUITE_GROUPS`].
    pub group: String,
    /// Headings beyond KEY and REQUIRED, each made by a registered producer.
    pub headings: Vec<String>,
    /// Unit or lens ids the test applies to. Empty only for the groups that
    /// are not scheduled on samples (see [`unscheduled`]).
    pub units: Vec<String>,
    /// The fraction of eligible samples tested. `None` only for those groups.
    pub rate: Option<Range>,
}

/// The groups a suite may list. TRIT is a child of TRIG; ISPT is a LOCA child,
/// WSTG a LOCA child written wherever a hole meets the water table in coarse
/// ground, WSTD a child of WSTG; the rest hang off SAMP.
///
/// TREG and TRET come last, not beside TRIG and TRIT: a group's position here
/// keys its scheduling draws (`suite::schedule`), so putting them elsewhere
/// would move every ISPT and water row of the presets that came before them.
pub const SUITE_GROUPS: &[&str] = &[
    "LNMC", "LLPL", "LDEN", "TRIG", "TRIT", "ISPT", "WSTG", "WSTD", "TREG", "TRET",
];

/// The lab groups, which a made-ground unit may not be tested for (ISPT, an
/// in-situ test, is allowed).
const LAB_GROUPS: &[&str] = &["LNMC", "LLPL", "LDEN", "TRIG", "TREG"];

/// For a group scheduled by something other than a rate over samples, the
/// parent group its entry needs, if it has one, and why the entry has no
/// `units` or `rate`: TRIT goes with its TRIG, WSTG is decided by the water
/// table, WSTD goes with its WSTG.
fn unscheduled(group: &str) -> Option<(Option<&'static str>, &'static str)> {
    match group {
        "TRIT" => Some((Some("TRIG"), "TRIT is scheduled with TRIG")),
        "TRET" => Some((Some("TREG"), "TRET is scheduled with TREG")),
        "WSTG" => Some((None, "WSTG follows the water table")),
        "WSTD" => Some((Some("WSTG"), "WSTD is scheduled with WSTG")),
        _ => None,
    }
}

/// Constraints for the unit's description. The site layer checks the terms
/// against the BS 5930 vocabulary, not the loader.
#[derive(Debug, Clone, PartialEq)]
pub struct DescriptionSpec {
    pub principals: Vec<String>,
    pub secondaries: Vec<String>,
    pub colours: Vec<String>,
    /// Cobble and boulder content, worded per BS 5930 Table 15.
    pub very_coarse: Option<VeryCoarse>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Sampling {
    /// `SAMP_TYPE` codes.
    pub types: Vec<String>,
    pub interval_m: Range,
}

/// The filter pack runs this far above and below the slotted section
/// (metres), the clearance the crawl records.
pub const FILTER_OVERLAP_M: f64 = 0.5;

/// The thinnest seal or tail of bentonite the installed backfill writes
/// (metres).
pub const MIN_SEAL_M: f64 = 0.1;

/// A cited range: `basis` is the source of `min` and `max`.
#[derive(Debug, Clone, PartialEq)]
pub struct Range {
    pub min: f64,
    pub max: f64,
    pub basis: String,
    /// Only a unit's `thickness_m` (and the cover, construction and a unit's
    /// `strike.rise_m` ranges that read like it) carries one: a positive exponent applied to
    /// the field value before it is placed on the range (ADR 0001). `None`
    /// is the plain linear mapping.
    pub skew: Option<f64>,
}

/// A cited range with an optional depth gradient: the value at depth `z`
/// below the unit's top is the field value in `range` plus `per_m` x `z`.
#[derive(Debug, Clone, PartialEq)]
pub struct Quantity {
    pub range: Range,
    /// Change per metre of depth; its own cited range.
    pub per_m: Option<Range>,
}

/// The index properties a unit or lens may carry (ADR 0001, "Values and
/// descriptions"). `fine` is required when any principal is CLAY or SILT, and
/// `coarse` when any is SAND or GRAVEL.
#[derive(Debug, Clone, PartialEq)]
pub struct Properties {
    pub fine: Option<FineProperties>,
    pub coarse: Option<CoarseProperties>,
    /// Particle density ratio Gs.
    pub particle_density_ratio: Quantity,
    /// Sr above the water table; 1 when absent.
    pub degree_of_saturation: Option<Quantity>,
    /// The chance, per hole, unit and quantity, of an outlier; a range so the
    /// rate may vary smoothly across the site. Default
    /// [`props::OUTLIER_RATE_DEFAULT`].
    pub outlier_rate: Option<Range>,
    /// Undrained shear strength, by named relation. Required by the suite's
    /// TRIT strength headings and by `spt`.
    pub strength: Option<Strength>,
    /// SPT N from undrained strength, by named relation. Needs `strength`.
    pub spt: Option<SptFromStrength>,
    /// `spt.refusal_share`: the share of this unit's SPTs, drawn once per
    /// site, stopped by an obstruction (a cobble or boulder strike) whatever
    /// the strength. Allowed only on a coarse unit or one whose description
    /// has `very_coarse`. Refusals from strength (N of 50 or more) need
    /// nothing here.
    pub spt_refusal_share: Option<RefusalShare>,
    /// `triaxial`: multi-stage unconsolidated-undrained tests (`UUM`).
    pub triaxial: Option<Triaxial>,
    /// `effective_strength`: what the unit's effective-stress triaxial tests
    /// (TREG, TRET) are built from.
    pub effective_strength: Option<EffectiveStrength>,
}

/// Effective-stress strength of a unit (ADR 0001, "Effective-stress
/// triaxial"). Each range is drawn once per site; each TREG set then scatters
/// around the site's values.
#[derive(Debug, Clone, PartialEq)]
pub struct EffectiveStrength {
    /// Effective cohesion intercept c', kPa, in `[0, 25]`.
    pub c_eff_kpa: Range,
    /// Effective friction angle phi', degrees, in `[0, 50]`.
    pub phi_eff_deg: Range,
    /// Skempton's pore pressure parameter at failure, `A_f` = (pore pressure
    /// rise) / (deviator stress), for the unit's undrained (CU, CUM) tests, in
    /// `[-0.5, 1.5]`.
    pub skempton_af: Range,
}

/// The loader's bounds on [`EffectiveStrength`]: c' (kPa), phi' (degrees) and
/// `A_f`. c' is capped at the 25 kPa top of the range CIRIA C504 (Trenter 1999,
/// p.85) gives for clay tills; the others are the physically sensible ranges.
pub const C_EFF_BOUNDS_KPA: (f64, f64) = (0.0, 25.0);
pub const PHI_EFF_BOUNDS_DEG: (f64, f64) = (0.0, 50.0);
pub const SKEMPTON_AF_BOUNDS: (f64, f64) = (-0.5, 1.5);

/// Multi-stage UU triaxial tests in a unit (ADR 0001, "Multi-stage UU"). Each
/// TRIG test in the unit makes its own keyed draw on whether it is `UUM`: three
/// stages (`TRIT_TESN` 1 to 3) at cell pressures of 1, 2 and 4 times the
/// single-stage value, on one specimen.
#[derive(Debug, Clone, PartialEq)]
pub struct Triaxial {
    /// The share of the unit's TRIG tests that are multi-stage, drawn once
    /// per site in the range, a fraction in `[0, 1]`.
    pub multi_stage_share: Range,
    /// cu at stage k is stage 1's x (1 + gain x (k - 1)); the gain is drawn
    /// once per site in the range, in `[0, 1]`. Absent means 0.
    pub stage_gain: Option<Range>,
}

/// How the obstruction share of a unit's SPTs is given.
#[derive(Debug, Clone, PartialEq)]
pub enum RefusalShare {
    /// One share, drawn once per site in the range, at every depth.
    Share(Range),
    /// share(z) = clamp(`top` + `per_m` x z, 0, 1), z metres below ground,
    /// `top` and `per_m` each drawn once per site in their ranges (as
    /// `cu.linear_with_depth` takes `cu_top_kpa` and `per_m`).
    Linear { top: Range, per_m: Range },
}

/// The relations a preset may name for undrained strength. A preset picks one
/// by `relation = "<name>"`.
pub const STRENGTH_RELATIONS: &[&str] = &["cu.linear_with_depth"];
/// The relations a preset may name for SPT N from strength.
pub const SPT_RELATIONS: &[&str] = &["spt.stroud_butler"];

/// The undrained-strength relation a unit uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StrengthRelation {
    /// cu(z) = `cu_top` + `per_m` x z, z metres below the unit's top.
    LinearWithDepth,
}

/// Undrained strength by named relation, in kPa. The parameters are the
/// relation's; each is a cited range varying smoothly across the site.
#[derive(Debug, Clone, PartialEq)]
pub struct Strength {
    pub relation: StrengthRelation,
    /// cu at the unit's top.
    pub cu_top_kpa: Range,
    /// Change in cu per metre of depth.
    pub per_m: Range,
}

/// The SPT-from-strength relation a unit uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SptRelation {
    /// N = cu / f1 (Stroud and Butler).
    StroudButler,
}

/// SPT N from undrained strength by named relation.
#[derive(Debug, Clone, PartialEq)]
pub struct SptFromStrength {
    pub relation: SptRelation,
    /// f1, kPa per blow, varying smoothly across the site.
    pub f1: Range,
}

/// Drawn for CLAY and SILT units. Liquid limit, moisture content and the
/// consistency index are derived from these.
#[derive(Debug, Clone, PartialEq)]
pub struct FineProperties {
    pub plastic_limit_percent: Quantity,
    pub plasticity_index_percent: Quantity,
    pub liquidity_index: Quantity,
}

/// Drawn for SAND and GRAVEL units.
#[derive(Debug, Clone, PartialEq)]
pub struct CoarseProperties {
    pub spt_n: Quantity,
    pub moisture_content_percent: Quantity,
}

/// Every problem found in a preset, one line each, naming where it is.
#[derive(Debug, Clone, PartialEq)]
pub struct PresetErrors(pub Vec<String>);

impl fmt::Display for PresetErrors {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (i, e) in self.0.iter().enumerate() {
            if i > 0 {
                writeln!(f)?;
            }
            write!(f, "{e}")?;
        }
        Ok(())
    }
}

impl std::error::Error for PresetErrors {}

impl PresetErrors {
    fn one(msg: String) -> Self {
        Self(vec![msg])
    }
}

/// Resolve `arg` as a shipped preset name, else as a path to a TOML file.
pub fn resolve(arg: &str) -> Result<Preset, PresetErrors> {
    if let Some((_, text)) = SHIPPED.iter().find(|(name, _)| *name == arg) {
        return Preset::from_toml(text);
    }
    let path = Path::new(arg);
    if path.is_file() {
        let text = std::fs::read_to_string(path)
            .map_err(|e| PresetErrors::one(format!("{arg}: cannot read preset: {e}")))?;
        return Preset::from_toml(&text);
    }
    let names: Vec<&str> = SHIPPED.iter().map(|(n, _)| *n).collect();
    let shipped = if names.is_empty() {
        "none".to_string()
    } else {
        names.join(", ")
    };
    Err(PresetErrors::one(format!(
        "{arg}: not a shipped preset or a file (shipped presets: {shipped})"
    )))
}

impl Preset {
    /// Whether `unit` is the id of a made-ground unit. A lens never is.
    pub fn is_made_ground(&self, unit: &str) -> bool {
        self.units
            .iter()
            .any(|u| u.id == unit && u.made_ground.is_some())
    }

    /// Whether `unit`, a unit or lens id, is water-bearing.
    #[cfg(test)]
    pub fn is_water_bearing(&self, unit: &str) -> bool {
        self.water_share(unit).is_some()
    }

    /// The strike share range of `unit`, a unit or lens id, when it is
    /// water-bearing. `true` in the file is the range 1 to 1.
    pub fn water_share(&self, unit: &str) -> Option<&Range> {
        self.units
            .iter()
            .map(|u| (&u.id, &u.water_bearing))
            .chain(self.lenses.iter().map(|l| (&l.id, &l.water_bearing)))
            .find(|(id, _)| *id == unit)
            .and_then(|(_, wb)| wb.as_ref())
    }

    pub fn from_toml(text: &str) -> Result<Preset, PresetErrors> {
        let root: Table = text
            .parse()
            .map_err(|e: toml::de::Error| PresetErrors::one(format!("toml: {e}")))?;
        let mut errs = Vec::new();
        let preset = read_preset(&root, &mut errs);
        match preset {
            Some(p) if errs.is_empty() => Ok(p),
            _ => Err(PresetErrors(errs)),
        }
    }
}

/// A table being read: tracks which keys were consumed so the rest can be
/// reported as unknown.
struct Node<'a> {
    table: &'a Table,
    path: String,
    seen: Vec<&'static str>,
}

impl<'a> Node<'a> {
    fn new(table: &'a Table, path: impl Into<String>) -> Self {
        Self {
            table,
            path: path.into(),
            seen: Vec::new(),
        }
    }

    fn at(&self, key: &str) -> String {
        if self.path.is_empty() {
            key.to_string()
        } else {
            format!("{}.{key}", self.path)
        }
    }

    fn get(&mut self, key: &'static str) -> Option<&'a Value> {
        self.seen.push(key);
        self.table.get(key)
    }

    fn finish(self, errs: &mut Vec<String>) {
        for k in self.table.keys() {
            if !self.seen.contains(&k.as_str()) {
                let at = if self.path.is_empty() {
                    "preset"
                } else {
                    &self.path
                };
                errs.push(format!("{at}: unknown key \"{k}\""));
            }
        }
    }
}

fn req_str(n: &mut Node, key: &'static str, errs: &mut Vec<String>) -> Option<String> {
    match n.get(key) {
        None => {
            errs.push(format!("{}: missing", n.at(key)));
            None
        }
        Some(Value::String(s)) => Some(s.clone()),
        Some(_) => {
            errs.push(format!("{}: must be a string", n.at(key)));
            None
        }
    }
}

fn req_int(n: &mut Node, key: &'static str, errs: &mut Vec<String>) -> Option<i64> {
    match n.get(key) {
        None => {
            errs.push(format!("{}: missing", n.at(key)));
            None
        }
        Some(Value::Integer(i)) => Some(*i),
        Some(_) => {
            errs.push(format!("{}: must be an integer", n.at(key)));
            None
        }
    }
}

/// A finite number; TOML integers are accepted where a float is wanted.
fn num(v: Option<&Value>, at: &str, errs: &mut Vec<String>) -> Option<f64> {
    let x = match v {
        None => {
            errs.push(format!("{at}: missing"));
            return None;
        }
        Some(Value::Float(f)) => *f,
        Some(Value::Integer(i)) => *i as f64,
        Some(_) => {
            errs.push(format!("{at}: must be a number"));
            return None;
        }
    };
    if x.is_finite() {
        Some(x)
    } else {
        errs.push(format!("{at}: must be finite"));
        None
    }
}

fn req_table<'a>(n: &mut Node<'a>, key: &'static str, errs: &mut Vec<String>) -> Option<Node<'a>> {
    match n.get(key) {
        None => {
            errs.push(format!("{}: missing", n.at(key)));
            None
        }
        Some(Value::Table(t)) => Some(Node::new(t, n.at(key))),
        Some(_) => {
            errs.push(format!("{}: must be a table", n.at(key)));
            None
        }
    }
}

fn str_list(n: &mut Node, key: &'static str, errs: &mut Vec<String>) -> Option<Vec<String>> {
    let at = n.at(key);
    match n.get(key) {
        None => {
            errs.push(format!("{at}: missing"));
            None
        }
        Some(Value::Array(a)) => {
            let mut out = Vec::new();
            for (i, v) in a.iter().enumerate() {
                if let Value::String(s) = v {
                    out.push(s.clone());
                } else {
                    errs.push(format!("{at}[{i}]: must be a string"));
                }
            }
            Some(out)
        }
        Some(_) => {
            errs.push(format!("{at}: must be an array of strings"));
            None
        }
    }
}

/// Reads `{ min, max, basis }` under `key`.
fn read_range(n: &mut Node, key: &'static str, errs: &mut Vec<String>) -> Option<Range> {
    let mut r = req_table(n, key, errs)?;
    let range = range_in(&mut r, errs);
    r.finish(errs);
    range
}

/// The `min`, `max` and `basis` of the table `r`; the caller finishes `r`.
fn range_in(r: &mut Node, errs: &mut Vec<String>) -> Option<Range> {
    let min = num(r.get("min"), &r.at("min"), errs);
    let max = num(r.get("max"), &r.at("max"), errs);
    let basis = match r.get("basis") {
        None => {
            errs.push(format!("{}: no basis", r.path));
            None
        }
        Some(Value::String(s)) if s.trim().is_empty() => {
            errs.push(format!("{}: empty basis", r.path));
            None
        }
        Some(Value::String(s)) => Some(s.clone()),
        Some(_) => {
            errs.push(format!("{}.basis: must be a string", r.path));
            None
        }
    };
    if let (Some(lo), Some(hi)) = (min, max)
        && lo > hi
    {
        errs.push(format!("{}: min {lo} is greater than max {hi}", r.path));
    }
    Some(Range {
        min: min?,
        max: max?,
        basis: basis?,
        skew: None,
    })
}

/// Reads a unit's `{ min, max, basis, skew? }` under `key`. The skew belongs
/// to the range, so it needs no basis of its own. It is at least 1: below 1
/// the map's slope is unbounded near 0, so neighbouring holes could disagree.
fn read_thickness(n: &mut Node, key: &'static str, errs: &mut Vec<String>) -> Option<Range> {
    let mut r = req_table(n, key, errs)?;
    let range = range_in(&mut r, errs);
    let skew = if r.table.contains_key("skew") {
        let at = r.at("skew");
        match num(r.get("skew"), &at, errs) {
            Some(k) if k >= 1.0 => Some(k),
            Some(k) => {
                errs.push(format!("{at}: {k} must be >= 1"));
                None
            }
            None => None,
        }
    } else {
        None
    };
    let bad_skew = r.table.contains_key("skew") && skew.is_none();
    r.finish(errs);
    if bad_skew {
        return None;
    }
    range.map(|range| Range { skew, ..range })
}

/// A range that must lie inside `domain`, each bound checked and named.
fn check_domain(r: &Range, at: &str, domain: &Domain, errs: &mut Vec<String>) {
    for (which, v) in [("min", r.min), ("max", r.max)] {
        if !domain.contains(v) {
            errs.push(format!(
                "{at}: {which} {v} is outside its domain ({})",
                domain.describe()
            ));
        }
    }
}

/// Reads `{ min, max, basis, per_m? }` under `key`, inside `domain`.
fn read_quantity(
    n: &mut Node,
    key: &'static str,
    domain: &Domain,
    errs: &mut Vec<String>,
) -> Option<Quantity> {
    let mut q = req_table(n, key, errs)?;
    let range = range_in(&mut q, errs);
    if let Some(r) = &range {
        check_domain(r, &q.path, domain, errs);
    }
    let per_m = if q.table.contains_key("per_m") {
        read_range(&mut q, "per_m", errs)
    } else {
        q.get("per_m");
        None
    };
    q.finish(errs);
    Some(Quantity {
        range: range?,
        per_m,
    })
}

/// A quantity of a property set. `unusable` is why the unit cannot use the
/// set, if it cannot: the key is then refused when written, and otherwise it
/// is required.
fn set_quantity(
    p: &mut Node,
    key: &'static str,
    domain: &Domain,
    unusable: Option<&str>,
    errs: &mut Vec<String>,
) -> Option<Quantity> {
    if let Some(reason) = unusable {
        if p.table.contains_key(key) {
            p.get(key);
            errs.push(format!("{}: {reason}, so it cannot use this", p.at(key)));
        }
        None
    } else {
        read_quantity(p, key, domain, errs)
    }
}

/// The `properties` table of a unit or lens whose principals are
/// `principals`. `None` when it has problems (which are then in `errs`).
fn read_properties(
    u: &mut Node,
    principals: &[String],
    very_coarse: bool,
    errs: &mut Vec<String>,
) -> Option<Properties> {
    let has = |names: [&str; 2]| principals.iter().any(|p| names.contains(&p.as_str()));
    let (fine, coarse) = (has(["CLAY", "SILT"]), has(["SAND", "GRAVEL"]));
    let mut p = req_table(u, "properties", errs)?;
    if !fine && !coarse {
        errs.push(format!(
            "{}: no principal is fine (CLAY, SILT) or coarse (SAND, GRAVEL), so no property set applies",
            p.path
        ));
        return None;
    }
    let no_fine = (!fine).then_some("the unit has no fine principal (CLAY, SILT)");
    let no_coarse = (!coarse).then_some("the unit has no coarse principal (SAND, GRAVEL)");
    let pl = set_quantity(
        &mut p,
        "plastic_limit_percent",
        &props::PLASTIC_LIMIT_PERCENT,
        no_fine,
        errs,
    );
    let pi = set_quantity(
        &mut p,
        "plasticity_index_percent",
        &props::PLASTICITY_INDEX_PERCENT,
        no_fine,
        errs,
    );
    let li = set_quantity(
        &mut p,
        "liquidity_index",
        &props::LIQUIDITY_INDEX,
        no_fine,
        errs,
    );
    let spt = set_quantity(&mut p, "spt_n", &props::SPT_N, no_coarse, errs);
    let w = set_quantity(
        &mut p,
        "moisture_content_percent",
        &props::MOISTURE_CONTENT_PERCENT,
        no_coarse,
        errs,
    );
    let gs = read_quantity(
        &mut p,
        "particle_density_ratio",
        &props::PARTICLE_DENSITY_RATIO,
        errs,
    );
    let sr = if p.table.contains_key("degree_of_saturation") {
        read_quantity(
            &mut p,
            "degree_of_saturation",
            &props::DEGREE_OF_SATURATION,
            errs,
        )
    } else {
        p.get("degree_of_saturation");
        None
    };
    let rate = if p.table.contains_key("outlier_rate") {
        let rate = read_range(&mut p, "outlier_rate", errs);
        if let Some(r) = &rate {
            check_domain(r, &p.at("outlier_rate"), &props::OUTLIER_RATE, errs);
        }
        rate
    } else {
        p.get("outlier_rate");
        None
    };
    let strength = read_strength(&mut p, errs);
    let triaxial = read_triaxial(&mut p, errs);
    let effective_strength = read_effective_strength(&mut p, errs);
    let (spt_relation, refusal_share) =
        read_spt(&mut p, coarse || very_coarse, errs).unwrap_or((None, None));
    // A strength table with problems has reported them already.
    if spt_relation.is_some() && !p.table.contains_key("strength") {
        errs.push(format!(
            "{}: spt.stroud_butler divides strength by f1, so the unit needs strength",
            p.at("spt")
        ));
    }
    p.finish(errs);
    Some(Properties {
        fine: if fine {
            Some(FineProperties {
                plastic_limit_percent: pl?,
                plasticity_index_percent: pi?,
                liquidity_index: li?,
            })
        } else {
            None
        },
        coarse: if coarse {
            Some(CoarseProperties {
                spt_n: spt?,
                moisture_content_percent: w?,
            })
        } else {
            None
        },
        particle_density_ratio: gs?,
        degree_of_saturation: sr,
        outlier_rate: rate,
        strength,
        spt: spt_relation,
        spt_refusal_share: refusal_share,
        triaxial: triaxial.flatten(),
        effective_strength: effective_strength.flatten(),
    })
}

/// The optional `effective_strength` table: `c_eff_kpa`, `phi_eff_deg` and
/// `skempton_af`, each `{ min, max, basis }` within its bounds, all three
/// required. Outer `None` is a reported problem; inner `None` is "not
/// written".
#[allow(clippy::option_option)]
fn read_effective_strength(
    p: &mut Node,
    errs: &mut Vec<String>,
) -> Option<Option<EffectiveStrength>> {
    if !p.table.contains_key("effective_strength") {
        p.get("effective_strength");
        return Some(None);
    }
    let mut t = req_table(p, "effective_strength", errs)?;
    let bounded =
        |t: &mut Node, key: &'static str, (lo, hi): (f64, f64), errs: &mut Vec<String>| {
            let r = read_range(t, key, errs)?;
            let at = t.at(key);
            let mut fine = true;
            for (which, v) in [("min", r.min), ("max", r.max)] {
                if !(lo..=hi).contains(&v) {
                    errs.push(format!("{at}: {which} {v} is outside {lo} to {hi}"));
                    fine = false;
                }
            }
            fine.then_some(r)
        };
    let c = bounded(&mut t, "c_eff_kpa", C_EFF_BOUNDS_KPA, errs);
    let phi = bounded(&mut t, "phi_eff_deg", PHI_EFF_BOUNDS_DEG, errs);
    let af = bounded(&mut t, "skempton_af", SKEMPTON_AF_BOUNDS, errs);
    let at = t.at("skempton_af");
    t.finish(errs);
    let (c, phi, af) = (c?, phi?, af?);
    // The failure deviator is 2 (c' cos phi' + s3' sin phi') / (1 - sin phi'
    // (1 - 2 A_f)), so the lowest A_f with the highest phi' must keep the
    // denominator positive, or no deviator stress reaches the envelope.
    if phi.max.to_radians().sin() * (1.0 - 2.0 * af.min) >= 1.0 {
        errs.push(format!(
            "{at}: min {} with phi_eff_deg max {} leaves no deviator stress that reaches the envelope (needs sin(phi') x (1 - 2 A_f) < 1)",
            af.min, phi.max
        ));
        return None;
    }
    Some(Some(EffectiveStrength {
        c_eff_kpa: c,
        phi_eff_deg: phi,
        skempton_af: af,
    }))
}

/// The optional `triaxial` table: `multi_stage_share` and, with it,
/// `stage_gain`, each `{ min, max, basis }` in `[0, 1]`. Outer `None` is a
/// reported problem; inner `None` is "not written".
#[allow(clippy::option_option)]
fn read_triaxial(p: &mut Node, errs: &mut Vec<String>) -> Option<Option<Triaxial>> {
    if !p.table.contains_key("triaxial") {
        p.get("triaxial");
        return Some(None);
    }
    let mut t = req_table(p, "triaxial", errs)?;
    let has_share = t.table.contains_key("multi_stage_share");
    let mut ok = true;
    let bounded = |t: &mut Node, key: &'static str, errs: &mut Vec<String>| {
        let r = read_range(t, key, errs)?;
        let at = t.at(key);
        let mut fine = true;
        for (which, v) in [("min", r.min), ("max", r.max)] {
            if !(0.0..=1.0).contains(&v) {
                errs.push(format!("{at}: {which} {v} is outside 0 to 1"));
                fine = false;
            }
        }
        fine.then_some(r)
    };
    let share = if has_share {
        let r = bounded(&mut t, "multi_stage_share", errs);
        ok &= r.is_some();
        r
    } else {
        // `stage_gain` alone is reported below, once.
        if !t.table.contains_key("stage_gain") {
            errs.push(format!("{}: missing", t.at("multi_stage_share")));
        }
        ok = false;
        None
    };
    let gain = if t.table.contains_key("stage_gain") {
        if has_share {
            let r = bounded(&mut t, "stage_gain", errs);
            ok &= r.is_some();
            r
        } else {
            t.get("stage_gain");
            errs.push(format!(
                "{}: stage_gain needs multi_stage_share, since no test is multi-stage without one",
                t.at("stage_gain")
            ));
            None
        }
    } else {
        t.get("stage_gain");
        None
    };
    t.finish(errs);
    if !ok {
        return None;
    }
    Some(share.map(|multi_stage_share| Triaxial {
        multi_stage_share,
        stage_gain: gain,
    }))
}

/// The optional `relation` of `t`, which must be one of `known`; the error
/// names the field and lists the known names.
fn read_relation(t: &mut Node, known: &[&str], errs: &mut Vec<String>) -> Option<usize> {
    let name = req_str(t, "relation", errs)?;
    let found = known.iter().position(|k| *k == name);
    if found.is_none() {
        errs.push(format!(
            "{}: \"{name}\" is not a known relation ({})",
            t.at("relation"),
            known.join(", ")
        ));
    }
    found
}

/// The optional `strength` table: `relation` plus that relation's parameters.
fn read_strength(p: &mut Node, errs: &mut Vec<String>) -> Option<Strength> {
    if !p.table.contains_key("strength") {
        p.get("strength");
        return None;
    }
    let mut t = req_table(p, "strength", errs)?;
    let relation = read_relation(&mut t, STRENGTH_RELATIONS, errs);
    let cu_top = read_range(&mut t, "cu_top_kpa", errs);
    if let Some(r) = &cu_top {
        check_domain(r, &t.at("cu_top_kpa"), &props::UNDRAINED_STRENGTH_KPA, errs);
    }
    let per_m = read_range(&mut t, "per_m", errs);
    t.finish(errs);
    relation?;
    Some(Strength {
        relation: StrengthRelation::LinearWithDepth,
        cu_top_kpa: cu_top?,
        per_m: per_m?,
    })
}

/// What an `spt` table gives: the strength relation and/or the obstruction
/// share. Outer `None` is a reported problem; each inner `None` is "not
/// written".
type SptTable = (Option<SptFromStrength>, Option<RefusalShare>);

/// The optional `spt` table: `relation` plus that relation's parameters,
/// and/or `refusal_share`. A table of `refusal_share` alone is allowed (a
/// coarse unit has its N already). `refusal_ok` says whether the unit may
/// carry a share (coarse, or `very_coarse` in its description).
#[allow(clippy::option_option)]
fn read_spt(p: &mut Node, refusal_ok: bool, errs: &mut Vec<String>) -> Option<SptTable> {
    if !p.table.contains_key("spt") {
        p.get("spt");
        return Some((None, None));
    }
    let mut t = req_table(p, "spt", errs)?;
    let has_share = t.table.contains_key("refusal_share");
    let has_relation = t.table.contains_key("relation") || t.table.contains_key("f1");
    let mut ok = true;
    let relation = if has_relation || !has_share {
        let relation = read_relation(&mut t, SPT_RELATIONS, errs);
        let f1 = read_range(&mut t, "f1", errs);
        if let Some(r) = &f1 {
            check_domain(r, &t.at("f1"), &props::SPT_F1_KPA, errs);
        }
        if let (Some(_), Some(f1)) = (relation, f1) {
            Some(SptFromStrength {
                relation: SptRelation::StroudButler,
                f1,
            })
        } else {
            ok = false;
            None
        }
    } else {
        None
    };
    let share = if has_share {
        let at = t.at("refusal_share");
        if refusal_ok {
            read_refusal_share(&mut t, errs)
        } else {
            t.get("refusal_share");
            errs.push(format!(
                "{at}: the unit is neither coarse (SAND, GRAVEL) nor described with very_coarse, so no cobble or boulder strike can stop its SPTs"
            ));
            ok = false;
            None
        }
    } else {
        None
    };
    ok &= !(has_share && refusal_ok && share.is_none());
    t.finish(errs);
    ok.then_some((relation, share))
}

/// `refusal_share = { min, max, basis }`, or `{ share = x, basis }` as
/// `water_bearing` takes, a fraction in `[0, 1]`; or the linear form
/// `{ top = { min, max, basis }, per_m = { min, max, basis } }`, both in
/// `[-1, 1]` (a `top` below 0 keeps the share at 0 near the surface).
fn read_refusal_share(t: &mut Node, errs: &mut Vec<String>) -> Option<RefusalShare> {
    let at = t.at("refusal_share");
    let Some(Value::Table(tbl)) = t.get("refusal_share") else {
        errs.push(format!("{at}: must be a table with a share"));
        return None;
    };
    let mut r = Node::new(tbl, at.clone());
    if tbl.contains_key("top") || tbl.contains_key("per_m") {
        let top = read_range(&mut r, "top", errs);
        let per_m = read_range(&mut r, "per_m", errs);
        let mut fine = true;
        for (key, rg) in [("top", &top), ("per_m", &per_m)] {
            if let Some(rg) = rg {
                for (which, v) in [("min", rg.min), ("max", rg.max)] {
                    if !(-1.0..=1.0).contains(&v) {
                        errs.push(format!("{at}.{key}: {which} {v} is outside -1 to 1"));
                        fine = false;
                    }
                }
            }
        }
        r.finish(errs);
        return match (top, per_m) {
            (Some(top), Some(per_m)) if fine => Some(RefusalShare::Linear { top, per_m }),
            _ => None,
        };
    }
    let range = if tbl.contains_key("share") {
        read_share(&mut r, errs)
    } else {
        let range = range_in(&mut r, errs);
        let mut fine = true;
        if let Some(rg) = &range {
            for (which, v) in [("min", rg.min), ("max", rg.max)] {
                if !(0.0..=1.0).contains(&v) {
                    errs.push(format!("{at}: {which} {v} is outside 0 to 1"));
                    fine = false;
                }
            }
        }
        range.filter(|_| fine)
    };
    r.finish(errs);
    range.map(RefusalShare::Share)
}

/// The unit or lens's `properties`, which are required. They depend on the
/// principals, so a description that failed to load leaves them unchecked.
fn required_properties(
    n: &mut Node,
    description: Option<&DescriptionSpec>,
    errs: &mut Vec<String>,
) -> Option<Properties> {
    // An empty principal list is already reported; it would only cascade.
    if let Some(d) = description
        && !d.principals.is_empty()
    {
        read_properties(n, &d.principals, d.very_coarse.is_some(), errs)
    } else {
        n.get("properties");
        None
    }
}

fn read_preset(root: &Table, errs: &mut Vec<String>) -> Option<Preset> {
    let mut n = Node::new(root, "");

    match n.get("schema") {
        None => errs.push("schema: missing".to_string()),
        Some(Value::Integer(SCHEMA)) => {}
        Some(Value::Integer(i)) => errs.push(format!(
            "schema: {i} is not supported (this forge reads {SCHEMA})"
        )),
        Some(_) => errs.push("schema: must be an integer".to_string()),
    }

    let name = req_str(&mut n, "name", errs);
    if let Some(s) = &name
        && (s.is_empty()
            || !s
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-'))
    {
        errs.push(format!("name: \"{s}\" is not [a-z0-9-]+"));
    }
    let version = req_int(&mut n, "version", errs).and_then(|v| {
        if let Ok(v) = u32::try_from(v)
            && v >= 1
        {
            Some(v)
        } else {
            errs.push(format!("version: {v} must be an integer >= 1"));
            None
        }
    });
    let summary = req_str(&mut n, "summary", errs);

    let site = read_site(&mut n, errs);
    let hole_types = read_hole_types(&mut n, errs);
    let units = read_units(&mut n, errs);
    let lenses = read_lenses(&mut n, errs);
    let suite = read_suite(&mut n, errs);
    let cover = read_cover(&mut n, errs);
    let construction = read_construction(&mut n, errs);
    n.finish(errs);

    let preset = Preset {
        name: name?,
        version: version?,
        summary: summary?,
        site: site?,
        hole_types: hole_types?,
        units: units?,
        lenses: lenses?,
        suite: suite?,
        cover: cover?,
        construction: construction?,
    };
    check_cover_depth(&preset, errs);
    check_construction(&preset, errs);
    // Entries that failed to load are missing from the suite, which would
    // shift the indices the checks name.
    if errs.is_empty() {
        check_suite_values(&preset, errs);
    }
    Some(preset)
}

/// The cover must end above the water table: a strike inside the cover would
/// have no stratum to belong to. Each thickness maximum is held below the
/// shallowest water table the site allows.
fn check_cover_depth(preset: &Preset, errs: &mut Vec<String>) {
    let Some(cover) = &preset.cover else { return };
    let table = preset.site.groundwater_depth_m.min;
    let layers = [
        (
            "surfacing",
            cover.surfacing.as_ref().map(|s| &s.thickness_m),
        ),
        ("topsoil", cover.topsoil.as_ref().map(|t| &t.thickness_m)),
    ];
    for (name, thickness) in layers {
        if let Some(t) = thickness
            && t.max >= table
        {
            errs.push(format!(
                "cover.{name}.thickness_m: max {} is not above the shallowest water table \
                 (site.groundwater_depth_m min {table})",
                t.max
            ));
        }
    }
}

/// The optional `[cover]`: `None` inside when absent. The two layers are
/// mutually exclusive per hole, so their share maxima must sum to at most 1.
/// The outer `None` is a problem already reported.
// Outer `None` is a reported problem; inner `None` is "not written".
#[allow(clippy::option_option)]
fn read_cover(n: &mut Node, errs: &mut Vec<String>) -> Option<Option<Cover>> {
    if !n.table.contains_key("cover") {
        n.get("cover");
        return Some(None);
    }
    let mut c = req_table(n, "cover", errs)?;
    let surfacing =
        optional_table(&mut c, "surfacing", errs).map(|s| s.and_then(|s| read_surfacing(s, errs)));
    let topsoil =
        optional_table(&mut c, "topsoil", errs).map(|t| t.and_then(|t| read_topsoil(t, errs)));
    if !c.table.contains_key("surfacing") && !c.table.contains_key("topsoil") {
        errs.push(format!("{}: names neither surfacing nor topsoil", c.path));
    }
    c.finish(errs);
    let (surfacing, topsoil) = (surfacing?, topsoil?);
    if let (Some(s), Some(t)) = (&surfacing, &topsoil)
        && s.share.max + t.share.max > 1.0
    {
        errs.push(format!(
            "cover: surfacing share max {} plus topsoil share max {} is above 1 \
             (a hole has at most one cover layer)",
            s.share.max, t.share.max
        ));
    }
    Some(Some(Cover { surfacing, topsoil }))
}

/// The optional `[construction]`: `None` inside when absent. The outer `None`
/// is a problem already reported.
// Outer `None` is a reported problem; inner `None` is "not written".
#[allow(clippy::option_option)]
fn read_construction(n: &mut Node, errs: &mut Vec<String>) -> Option<Option<Construction>> {
    if !n.table.contains_key("construction") {
        n.get("construction");
        return Some(None);
    }
    let mut c = req_table(n, "construction", errs)?;
    let pit = read_range(&mut c, "inspection_pit_m", errs).and_then(|r| {
        if r.min > 0.0 {
            Some(r)
        } else {
            errs.push(format!(
                "{}: min {} must be > 0",
                c.at("inspection_pit_m"),
                r.min
            ));
            None
        }
    });
    let casing = req_table(&mut c, "casing", errs).and_then(|k| read_casing(k, errs));
    let backfill = req_table(&mut c, "backfill", errs).and_then(|b| read_backfill(b, errs));
    let installation = match optional_table(&mut c, "installation", errs) {
        Some(Some(i)) => read_installation(i, errs).map(Some),
        Some(None) => Some(None),
        None => None,
    };
    let seal = match optional_table(&mut c, "seal", errs) {
        Some(Some(t)) => read_seal(t, errs).map(Some),
        Some(None) => Some(None),
        None => None,
    };
    c.finish(errs);
    Some(Some(Construction {
        inspection_pit_m: pit?,
        casing: casing?,
        backfill: backfill?,
        installation: installation?,
        seal: seal?,
    }))
}

/// `seal = { earlier_share, last_share, gap_m, basis }`: both shares in
/// `[0, 1]`, a cited gap that is never negative.
fn read_seal(mut t: Node, errs: &mut Vec<String>) -> Option<Seal> {
    let mut share = |t: &mut Node, key: &'static str| {
        let at = t.at(key);
        num(t.get(key), &at, errs).and_then(|v| {
            if (0.0..=1.0).contains(&v) {
                Some(v)
            } else {
                errs.push(format!("{at}: {v} is outside 0 to 1"));
                None
            }
        })
    };
    let earlier_share = share(&mut t, "earlier_share");
    let last_share = share(&mut t, "last_share");
    let gap_m = read_gap(&mut t, "gap_m", errs);
    let basis = nonempty_str(&mut t, "basis", errs);
    t.finish(errs);
    Some(Seal {
        earlier_share: earlier_share?,
        last_share: last_share?,
        gap_m: gap_m?,
        basis: basis?,
    })
}

fn read_installation(mut i: Node, errs: &mut Vec<String>) -> Option<Installation> {
    let site_share = read_unit_range(&mut i, "site_share", errs);
    let hole_share = read_unit_range(&mut i, "hole_share", errs);
    let kinds = read_kinds(&mut i, errs);
    let kinds_basis = nonempty_str(&mut i, "kinds_basis", errs);
    let zone_offset_m = read_thickness(&mut i, "zone_offset_m", errs);
    let zone_top_m = read_positive(&mut i, "zone_top_m", errs);
    let zone_follows_first_share = if i.table.contains_key("zone_follows_first_share") {
        read_unit_range(&mut i, "zone_follows_first_share", errs).map(Some)
    } else {
        i.get("zone_follows_first_share");
        Some(None)
    };
    let slotted_length_m = read_positive(&mut i, "slotted_length_m", errs);
    let base_gap_m = read_gap(&mut i, "base_gap_m", errs);
    let cap_m = read_positive(&mut i, "cap_m", errs);
    let filter = match read_weights::<2>(&mut i, "filter", "gravel and sand", errs) {
        Some(Some(w)) => Some(w),
        Some(None) => {
            errs.push(format!("{}: missing", i.at("filter")));
            None
        }
        None => None,
    };
    i.finish(errs);
    let (filter_weights, filter_basis) = filter?;
    Some(Installation {
        site_share: site_share?,
        hole_share: hole_share?,
        kinds: kinds?,
        kinds_basis: kinds_basis?,
        zone_offset_m: zone_offset_m?,
        zone_top_m: zone_top_m?,
        zone_follows_first_share: zone_follows_first_share?,
        slotted_length_m: slotted_length_m?,
        base_gap_m: base_gap_m?,
        cap_m: cap_m?,
        filter_weights,
        filter_basis,
    })
}

/// `{ min, max, basis, skew? }`, a length in metres that is positive
/// throughout.
fn read_positive(n: &mut Node, key: &'static str, errs: &mut Vec<String>) -> Option<Range> {
    let r = read_thickness(n, key, errs)?;
    if r.min <= 0.0 {
        errs.push(format!("{}: min {} must be > 0", n.at(key), r.min));
        return None;
    }
    Some(r)
}

/// `kinds = [{ kind, mm, weight }, ...]`: `standpipe` or `piezometer`, each
/// once, a `PIPE_DIAM` from [`INSTALL_DIAMETERS_MM`] and a weight above zero.
fn read_kinds(i: &mut Node, errs: &mut Vec<String>) -> Option<Vec<InstallKind>> {
    let at = i.at("kinds");
    let items: Vec<Node> = match i.get("kinds") {
        None => {
            errs.push(format!("{at}: missing"));
            return None;
        }
        Some(Value::Array(a)) => {
            let mut nodes = Vec::new();
            for (k, v) in a.iter().enumerate() {
                if let Value::Table(t) = v {
                    nodes.push(Node::new(t, format!("{at}[{k}]")));
                } else {
                    errs.push(format!("{at}[{k}]: must be a table"));
                    return None;
                }
            }
            nodes
        }
        Some(_) => {
            errs.push(format!("{at}: must be an array of tables"));
            return None;
        }
    };
    if items.is_empty() {
        errs.push(format!("{at}: at least one is required"));
        return None;
    }
    let mut out: Vec<InstallKind> = Vec::new();
    let mut ok = true;
    for mut d in items {
        let kind = req_str(&mut d, "kind", errs).and_then(|s| match s.as_str() {
            "standpipe" => Some(InstallKindName::Standpipe),
            "piezometer" => Some(InstallKindName::Piezometer),
            _ => {
                errs.push(format!(
                    "{}: \"{s}\" is not a kind (standpipe or piezometer)",
                    d.at("kind")
                ));
                None
            }
        });
        let mm = req_int(&mut d, "mm", errs).and_then(|v| match u32::try_from(v) {
            Ok(v) if INSTALL_DIAMETERS_MM.contains(&v) => Some(v),
            _ => {
                errs.push(format!(
                    "{}: {v} is not an installation diameter (19 or 50 mm)",
                    d.at("mm")
                ));
                None
            }
        });
        let weight = num(d.get("weight"), &d.at("weight"), errs).and_then(|w| {
            if w > 0.0 {
                Some(w)
            } else {
                errs.push(format!("{}: {w} must be > 0", d.at("weight")));
                None
            }
        });
        let here = d.path.clone();
        d.finish(errs);
        match (kind, mm, weight) {
            (Some(kind), Some(mm), Some(weight)) => {
                if out.iter().any(|x| x.kind == kind) {
                    errs.push(format!("{here}: this kind is listed twice"));
                    ok = false;
                }
                out.push(InstallKind { kind, mm, weight });
            }
            _ => ok = false,
        }
    }
    ok.then_some(out)
}

fn read_casing(mut k: Node, errs: &mut Vec<String>) -> Option<Casing> {
    let diameters = read_diameters(&mut k, errs);
    let diameters_basis = nonempty_str(&mut k, "diameters_basis", errs);
    let telescope_share = read_unit_range(&mut k, "telescope_share", errs);
    let telescope_depth_fraction = read_open_fraction(&mut k, "telescope_depth_fraction", errs);
    let final_gap_m = read_gap(&mut k, "final_gap_m", errs);
    let passes_strike_share = read_unit_range(&mut k, "passes_strike_share", errs);
    let strike_gap_m = read_gap(&mut k, "strike_gap_m", errs);
    k.finish(errs);
    Some(Casing {
        diameters: diameters?,
        diameters_basis: diameters_basis?,
        telescope_share: telescope_share?,
        telescope_depth_fraction: telescope_depth_fraction?,
        final_gap_m: final_gap_m?,
        passes_strike_share: passes_strike_share?,
        strike_gap_m: strike_gap_m?,
    })
}

/// `diameters = [{ mm, weight }, ...]`: positive whole millimetres, each
/// once, with non-negative weights that are not all zero.
fn read_diameters(k: &mut Node, errs: &mut Vec<String>) -> Option<Vec<CasingDiameter>> {
    let at = k.at("diameters");
    let items: Vec<Node> = match k.get("diameters") {
        None => {
            errs.push(format!("{at}: missing"));
            return None;
        }
        Some(Value::Array(a)) => {
            let mut nodes = Vec::new();
            for (i, v) in a.iter().enumerate() {
                if let Value::Table(t) = v {
                    nodes.push(Node::new(t, format!("{at}[{i}]")));
                } else {
                    errs.push(format!("{at}[{i}]: must be a table"));
                    return None;
                }
            }
            nodes
        }
        Some(_) => {
            errs.push(format!("{at}: must be an array of tables"));
            return None;
        }
    };
    if items.is_empty() {
        errs.push(format!("{at}: at least one is required"));
        return None;
    }
    let mut out: Vec<CasingDiameter> = Vec::new();
    let mut ok = true;
    for mut d in items {
        let mm = req_int(&mut d, "mm", errs).and_then(|v| match u32::try_from(v) {
            Ok(v) if v >= 1 => Some(v),
            _ => {
                errs.push(format!("{}: {v} must be an integer >= 1", d.at("mm")));
                None
            }
        });
        let weight = num(d.get("weight"), &d.at("weight"), errs).and_then(|w| {
            if w >= 0.0 {
                Some(w)
            } else {
                errs.push(format!("{}: {w} is negative", d.at("weight")));
                None
            }
        });
        let at = d.path.clone();
        d.finish(errs);
        match (mm, weight) {
            (Some(mm), Some(weight)) => {
                if out.iter().any(|x| x.mm == mm) {
                    errs.push(format!("{at}: {mm} mm is listed twice"));
                    ok = false;
                }
                out.push(CasingDiameter { mm, weight });
            }
            _ => ok = false,
        }
    }
    if ok && out.iter().map(|d| d.weight).sum::<f64>() <= 0.0 {
        errs.push(format!("{}: weights sum to zero", k.at("diameters")));
        ok = false;
    }
    ok.then_some(out)
}

fn read_backfill(mut b: Node, errs: &mut Vec<String>) -> Option<Backfill> {
    let site_share = read_unit_range(&mut b, "site_share", errs);
    let sequence = match read_weights::<3>(
        &mut b,
        "sequence",
        "arisings, bentonite and arisings over bentonite",
        errs,
    ) {
        Some(Some(w)) => Some(w),
        Some(None) => {
            errs.push(format!("{}: missing", b.at("sequence")));
            None
        }
        None => None,
    };
    let boundary_m = read_thickness(&mut b, "boundary_m", errs).and_then(|r| {
        if r.min > 0.0 {
            Some(r)
        } else {
            errs.push(format!("{}: min {} must be > 0", b.at("boundary_m"), r.min));
            None
        }
    });
    b.finish(errs);
    let (sequence_weights, sequence_basis) = sequence?;
    Some(Backfill {
        site_share: site_share?,
        sequence_weights,
        sequence_basis,
        boundary_m: boundary_m?,
    })
}

/// `{ min, max, basis }` with both bounds a fraction in `[0, 1]`.
fn read_unit_range(n: &mut Node, key: &'static str, errs: &mut Vec<String>) -> Option<Range> {
    let r = read_range(n, key, errs)?;
    let at = n.at(key);
    let mut fine = true;
    for (which, v) in [("min", r.min), ("max", r.max)] {
        if !(0.0..=1.0).contains(&v) {
            errs.push(format!("{at}: {which} {v} is outside 0 to 1"));
            fine = false;
        }
    }
    fine.then_some(r)
}

/// `{ min, max, basis, skew? }` with both bounds strictly between 0 and 1.
fn read_open_fraction(n: &mut Node, key: &'static str, errs: &mut Vec<String>) -> Option<Range> {
    let r = read_thickness(n, key, errs)?;
    let at = n.at(key);
    let mut fine = true;
    if r.min <= 0.0 {
        errs.push(format!("{at}: min {} must be > 0", r.min));
        fine = false;
    }
    if r.max >= 1.0 {
        errs.push(format!("{at}: max {} must be < 1", r.max));
        fine = false;
    }
    fine.then_some(r)
}

/// `{ min, max, basis, skew? }`, a distance in metres that is never negative.
fn read_gap(n: &mut Node, key: &'static str, errs: &mut Vec<String>) -> Option<Range> {
    let r = read_thickness(n, key, errs)?;
    if r.min < 0.0 {
        errs.push(format!("{}: min {} is negative", n.at(key), r.min));
        return None;
    }
    Some(r)
}

/// The construction must fit every hole it is put in: the inspection pit and
/// the backfill boundary end above the shortest final depth, and the hole's
/// own method must be a `HDPH_TYPE` code (it is the second phase).
fn check_construction(preset: &Preset, errs: &mut Vec<String>) {
    let Some(c) = &preset.construction else {
        return;
    };
    let Some((at, shortest)) = preset
        .hole_types
        .iter()
        .enumerate()
        .map(|(i, t)| (i, t.final_depth_m.min))
        .min_by(|a, b| a.1.total_cmp(&b.1))
    else {
        return;
    };
    for (key, r) in [
        ("inspection_pit_m", &c.inspection_pit_m),
        ("backfill.boundary_m", &c.backfill.boundary_m),
    ] {
        if r.max >= shortest {
            errs.push(format!(
                "construction.{key}: max {} is not above the shortest final depth \
                 (hole_types[{at}].final_depth_m min {shortest})",
                r.max
            ));
        }
    }
    if let Some(i) = &c.installation {
        // An installed hole must fit the pit, the cap and seal, the longest
        // slotted section and the filter and seal below it.
        let needs = c
            .inspection_pit_m
            .max
            .max(i.cap_m.max + MIN_SEAL_M + FILTER_OVERLAP_M)
            + i.slotted_length_m.max
            + FILTER_OVERLAP_M
            + MIN_SEAL_M;
        if needs >= shortest {
            errs.push(format!(
                "construction.installation: the pit, cap, seal, longest slotted section \
                 and filter need {needs} m, which is not below the shortest final depth \
                 (hole_types[{at}].final_depth_m min {shortest})"
            ));
        }
    }
    let dict = Dictionary::bundled(DictVersion::V4_2);
    let phases = dict.abbr_codes("HDPH_TYPE");
    for (i, t) in preset.hole_types.iter().enumerate() {
        if t.code == "IP" || !phases.contains(&t.code.as_str()) {
            errs.push(format!(
                "hole_types[{i}].code: \"{}\" cannot follow the inspection pit as a \
                 construction phase (not a HDPH_TYPE code, or IP itself)",
                t.code
            ));
        }
    }
}

/// `key` as a table node when written. Outer `None`: written but not a
/// table (reported). Inner `None`: not written.
// Outer `None` is a reported problem; inner `None` is "not written".
#[allow(clippy::option_option)]
fn optional_table<'a>(
    n: &mut Node<'a>,
    key: &'static str,
    errs: &mut Vec<String>,
) -> Option<Option<Node<'a>>> {
    if n.table.contains_key(key) {
        req_table(n, key, errs).map(Some)
    } else {
        n.get(key);
        Some(None)
    }
}

/// A cover layer's thickness: a cited range with an optional skew, positive
/// throughout (a layer of no thickness is absent, not drawn).
fn read_cover_thickness(n: &mut Node, errs: &mut Vec<String>) -> Option<Range> {
    let r = read_thickness(n, "thickness_m", errs)?;
    if r.min <= 0.0 {
        errs.push(format!(
            "{}: min {} must be > 0",
            n.at("thickness_m"),
            r.min
        ));
        return None;
    }
    Some(r)
}

/// A non-empty string under `key` (a basis), reported by name.
fn nonempty_str(n: &mut Node, key: &'static str, errs: &mut Vec<String>) -> Option<String> {
    let at = n.at(key);
    match req_str(n, key, errs) {
        Some(s) if s.trim().is_empty() => {
            errs.push(format!("{at}: empty"));
            None
        }
        other => other,
    }
}

/// The inline tables of the array under `key`, each as a node; the array is
/// required and not empty.
fn table_list<'a>(
    n: &mut Node<'a>,
    key: &'static str,
    errs: &mut Vec<String>,
) -> Option<Vec<Node<'a>>> {
    let at = n.at(key);
    match n.get(key) {
        None => {
            errs.push(format!("{at}: missing"));
            None
        }
        Some(Value::Array(a)) if a.is_empty() => {
            errs.push(format!("{at}: must not be empty"));
            None
        }
        Some(Value::Array(a)) => {
            let mut out = Vec::new();
            let mut ok = true;
            for (i, v) in a.iter().enumerate() {
                if let Value::Table(t) = v {
                    out.push(Node::new(t, format!("{at}[{i}]")));
                } else {
                    errs.push(format!("{at}[{i}]: must be a table"));
                    ok = false;
                }
            }
            ok.then_some(out)
        }
        Some(_) => {
            errs.push(format!("{at}: must be an array of tables"));
            None
        }
    }
}

/// A weight: a finite, non-negative number.
fn weight(n: &mut Node, key: &'static str, errs: &mut Vec<String>) -> Option<f64> {
    let at = n.at(key);
    let w = num(n.get(key), &at, errs)?;
    if w < 0.0 {
        errs.push(format!("{at}: {w} is negative"));
        return None;
    }
    Some(w)
}

fn read_surfacing(mut s: Node, errs: &mut Vec<String>) -> Option<Surfacing> {
    let share = read_share(&mut s, errs);
    let thickness = read_cover_thickness(&mut s, errs);
    let mut kinds = Vec::new();
    let mut kinds_ok = true;
    match table_list(&mut s, "kinds", errs) {
        None => kinds_ok = false,
        Some(list) => {
            for mut k in list {
                let name_at = k.at("name");
                let name = req_str(&mut k, "name", errs);
                let w = weight(&mut k, "weight", errs);
                k.finish(errs);
                match (name, w) {
                    (Some(name), Some(weight)) => {
                        if !cover::KINDS.contains(&name.as_str()) {
                            errs.push(format!(
                                "{}: unknown kind \"{name}\" (known: {})",
                                name_at,
                                cover::KINDS.join(", ")
                            ));
                            kinds_ok = false;
                        } else if kinds.iter().any(|x: &SurfacingKind| x.name == name) {
                            errs.push(format!("{name_at}: \"{name}\" is listed twice"));
                            kinds_ok = false;
                        } else {
                            kinds.push(SurfacingKind { name, weight });
                        }
                    }
                    _ => kinds_ok = false,
                }
            }
            if kinds_ok && kinds.iter().map(|k| k.weight).sum::<f64>() <= 0.0 {
                errs.push(format!("{}: weights sum to zero", s.at("kinds")));
                kinds_ok = false;
            }
        }
    }
    let kinds_basis = nonempty_str(&mut s, "kinds_basis", errs);
    s.finish(errs);
    kinds_ok.then_some(())?;
    Some(Surfacing {
        share: share?,
        thickness_m: thickness?,
        kinds,
        kinds_basis: kinds_basis?,
    })
}

fn read_topsoil(mut t: Node, errs: &mut Vec<String>) -> Option<Topsoil> {
    let share = read_share(&mut t, errs);
    let thickness = read_cover_thickness(&mut t, errs);
    let d = req_table(&mut t, "description", errs);
    let spec = d.and_then(|mut d| {
        let colours = read_topsoil_colours(&mut d, errs);
        let secondaries = read_topsoil_secondaries(&mut d, errs);
        let basis = nonempty_str(&mut d, "basis", errs);
        d.finish(errs);
        Some((
            TopsoilSpec {
                colours: colours?,
                secondaries: secondaries?,
            },
            basis?,
        ))
    });
    t.finish(errs);
    let (description, description_basis) = spec?;
    Some(Topsoil {
        share: share?,
        thickness_m: thickness?,
        description,
        description_basis,
    })
}

fn read_topsoil_colours(d: &mut Node, errs: &mut Vec<String>) -> Option<Vec<(String, f64)>> {
    let list = table_list(d, "colours", errs)?;
    let mut out = Vec::new();
    let mut ok = true;
    for mut c in list {
        let colour = req_str(&mut c, "colour", errs);
        let w = weight(&mut c, "weight", errs);
        c.finish(errs);
        match (colour, w) {
            (Some(colour), Some(w)) => out.push((colour, w)),
            _ => ok = false,
        }
    }
    if ok && out.iter().map(|c| c.1).sum::<f64>() <= 0.0 {
        errs.push(format!("{}: weights sum to zero", d.at("colours")));
        ok = false;
    }
    ok.then_some(out)
}

fn read_topsoil_secondaries(d: &mut Node, errs: &mut Vec<String>) -> Option<Vec<(String, f64)>> {
    let at = d.at("secondaries");
    // An empty list is allowed: a topsoil with no minor fractions.
    let Some(Value::Array(items)) = d.get("secondaries") else {
        errs.push(format!("{at}: must be an array of tables"));
        return None;
    };
    let mut out: Vec<(String, f64)> = Vec::new();
    let mut ok = true;
    for (i, v) in items.iter().enumerate() {
        let Value::Table(t) = v else {
            errs.push(format!("{at}[{i}]: must be a table"));
            ok = false;
            continue;
        };
        let mut s = Node::new(t, format!("{at}[{i}]"));
        let fraction = req_str(&mut s, "fraction", errs);
        let share = num(s.get("share"), &format!("{at}[{i}].share"), errs);
        let (share_at, fraction_at) = (s.at("share"), s.at("fraction"));
        s.finish(errs);
        if let Some(sh) = share
            && !(0.0..=1.0).contains(&sh)
        {
            errs.push(format!("{share_at}: {sh} is outside 0 to 1"));
            ok = false;
        }
        match (fraction, share) {
            (Some(f), Some(sh)) if (0.0..=1.0).contains(&sh) => {
                if out.iter().any(|(g, _)| *g == f) {
                    errs.push(format!("{fraction_at}: \"{f}\" is listed twice"));
                    ok = false;
                } else {
                    out.push((f, sh));
                }
            }
            _ => ok = false,
        }
    }
    ok.then_some(out)
}

/// The suite headings whose values a unit can only supply if its properties
/// say how: a TRIT strength heading needs `strength`, `ISPT_NVAL` needs
/// `strength` and `spt` (a fine unit) or `spt_n` (a coarse one), and an LLPL
/// heading needs the fine properties. Each refusal names the suite entry and
/// the unit.
fn check_suite_values(preset: &Preset, errs: &mut Vec<String>) {
    let properties = |id: &str| {
        preset
            .units
            .iter()
            .map(|u| (&u.id, &u.properties))
            .chain(preset.lenses.iter().map(|l| (&l.id, &l.properties)))
            .find(|(x, _)| x.as_str() == id)
            .map(|(_, p)| p)
    };
    let trig = preset
        .suite
        .iter()
        .position(|e| e.group == "TRIG")
        .map(|t| (t, &preset.suite[t].units));
    let tested_by_trig = |id: &str| trig.is_some_and(|(_, units)| units.iter().any(|u| u == id));
    for (id, p) in preset
        .units
        .iter()
        .map(|u| (&u.id, &u.properties))
        .chain(preset.lenses.iter().map(|l| (&l.id, &l.properties)))
    {
        if p.triaxial.is_some() && !tested_by_trig(id) {
            errs.push(format!(
                "{id}.properties.triaxial: multi-stage tests are TRIG tests, but no TRIG entry in the suite tests unit \"{id}\""
            ));
        }
    }
    let treg = preset
        .suite
        .iter()
        .find(|e| e.group == "TREG")
        .map(|e| &e.units);
    for (id, p) in preset
        .units
        .iter()
        .map(|u| (&u.id, &u.properties))
        .chain(preset.lenses.iter().map(|l| (&l.id, &l.properties)))
    {
        if p.effective_strength.is_some() && !treg.is_some_and(|units| units.contains(id)) {
            errs.push(format!(
                "{id}.properties.effective_strength: effective-stress triaxial results are TREG tests, but no TREG entry in the suite tests unit \"{id}\""
            ));
        }
    }
    for (i, entry) in preset.suite.iter().enumerate() {
        if entry.group == "TREG" {
            for (j, u) in entry.units.iter().enumerate() {
                if properties(u).is_some_and(|p| p.effective_strength.is_none()) {
                    errs.push(format!(
                        "suite[{i}].units[{j}]: TREG needs effective_strength (c', phi' and A_f) to build its sets and stages, but unit \"{u}\" has none"
                    ));
                }
            }
        }
        if LAB_GROUPS.contains(&entry.group.as_str()) {
            for (j, u) in entry.units.iter().enumerate() {
                if preset.is_made_ground(u) {
                    errs.push(format!(
                        "suite[{i}].units[{j}]: {} may not test made-ground unit \"{u}\" (no lab tests in Made Ground)",
                        entry.group
                    ));
                }
            }
        }
        for (j, h) in entry.headings.iter().enumerate() {
            let at = format!("suite[{i}].headings[{j}]");
            if entry.group == "ISPT"
                && results::ISPT_DEPENDENT_HEADINGS.contains(&h.as_str())
                && !entry.headings.iter().any(|x| x == "ISPT_NVAL")
            {
                errs.push(format!(
                    "{at}: {h} describes the test that ISPT_NVAL counts, so ISPT_NVAL must be listed too"
                ));
            }
            if entry.group == "TRIT" && results::TRIT_STRENGTH_HEADINGS.contains(&h.as_str()) {
                // TRIT is scheduled with TRIG, so it tests TRIG's units.
                let Some((t, units)) = trig else { continue };
                for u in units {
                    if properties(u).is_some_and(|p| p.strength.is_none()) {
                        errs.push(format!(
                            "{at}: {h} needs undrained strength, but unit \"{u}\" (in suite[{t}], TRIG) has no strength"
                        ));
                    }
                }
            } else if entry.group == "LLPL" && results::LLPL_HEADINGS.contains(&h.as_str()) {
                for u in &entry.units {
                    if properties(u).is_some_and(|p| p.fine.is_none()) {
                        errs.push(format!(
                            "{at}: {h} needs fine properties (liquid and plastic limits), but unit \"{u}\" has none"
                        ));
                    }
                }
            } else if h == "ISPT_NVAL" {
                for u in &entry.units {
                    let Some(p) = properties(u) else { continue };
                    if p.spt.is_none() && p.coarse.is_none() {
                        let need = if p.fine.is_some() {
                            "strength and spt (a fine unit)"
                        } else {
                            "spt_n (a coarse unit)"
                        };
                        errs.push(format!("{at}: {h} needs {need}, but unit \"{u}\" lacks it"));
                    }
                }
            }
        }
    }
}

fn read_site(n: &mut Node, errs: &mut Vec<String>) -> Option<Site> {
    let mut s = req_table(n, "site", errs)?;
    let extent = read_extent(&mut s, errs);
    let ground = read_range(&mut s, "ground_level_m", errs);
    let groundwater = read_range(&mut s, "groundwater_depth_m", errs);
    if let Some(r) = &groundwater {
        check_domain(
            r,
            &s.at("groundwater_depth_m"),
            &props::GROUNDWATER_DEPTH_M,
            errs,
        );
    }
    let strike_rise = if s.table.contains_key("strike_rise_m") {
        let r = read_range(&mut s, "strike_rise_m", errs);
        if let Some(r) = &r {
            check_domain(r, &s.at("strike_rise_m"), &props::STRIKE_RISE_M, errs);
        }
        r
    } else {
        None
    };
    let several_strikes = match optional_table(&mut s, "several_strikes", errs) {
        Some(Some(t)) => read_several_strikes(t, errs).map(Some),
        Some(None) => Some(None),
        None => None,
    };
    s.finish(errs);
    let (x, y) = extent?;
    Some(Site {
        extent_m: Extent { x, y },
        ground_level_m: ground?,
        groundwater_depth_m: groundwater?,
        strike_rise_m: strike_rise,
        several_strikes: several_strikes?,
    })
}

/// The most strikes a hole may record under `several_strikes`.
const MAX_STRIKES_PER_HOLE: i64 = 16;

/// `several_strikes = { max_per_hole, min_gap_m, basis }`: a cap of 2 to
/// [`MAX_STRIKES_PER_HOLE`] and a positive gap.
fn read_several_strikes(mut t: Node, errs: &mut Vec<String>) -> Option<SeveralStrikes> {
    let at = t.at("max_per_hole");
    let max = req_int(&mut t, "max_per_hole", errs).and_then(|v| {
        if (2..=MAX_STRIKES_PER_HOLE).contains(&v) {
            u32::try_from(v).ok()
        } else {
            errs.push(format!("{at}: {v} is outside 2 to {MAX_STRIKES_PER_HOLE}"));
            None
        }
    });
    let at = t.at("min_gap_m");
    let gap = num(t.get("min_gap_m"), &at, errs).and_then(|v| {
        if v > 0.0 {
            Some(v)
        } else {
            errs.push(format!("{at}: {v} must be > 0"));
            None
        }
    });
    let basis = nonempty_str(&mut t, "basis", errs);
    t.finish(errs);
    Some(SeveralStrikes {
        max_per_hole: max?,
        min_gap_m: gap?,
        basis: basis?,
    })
}

fn read_extent(s: &mut Node, errs: &mut Vec<String>) -> Option<(f64, f64)> {
    let mut e = req_table(s, "extent_m", errs)?;
    let x = num(e.get("x"), &e.at("x"), errs);
    let y = num(e.get("y"), &e.at("y"), errs);
    for (v, k) in [(x, "x"), (y, "y")] {
        if let Some(v) = v
            && v <= 0.0
        {
            errs.push(format!("{}: {v} must be > 0", e.at(k)));
        }
    }
    e.finish(errs);
    Some((x?, y?))
}

/// The tables of an array-of-tables key, each with its index and a node path
/// like `units[1]`.
fn read_array<'a>(
    n: &mut Node<'a>,
    key: &'static str,
    errs: &mut Vec<String>,
) -> Option<Vec<(Node<'a>, usize)>> {
    match n.get(key) {
        None => {
            errs.push(format!("{key}: missing"));
            None
        }
        Some(Value::Array(a)) => {
            let mut out = Vec::new();
            for (i, v) in a.iter().enumerate() {
                if let Value::Table(t) = v {
                    out.push((Node::new(t, format!("{key}[{i}]")), i));
                } else {
                    errs.push(format!("{key}[{i}]: must be a table"));
                }
            }
            Some(out)
        }
        Some(_) => {
            errs.push(format!("{key}: must be an array of tables"));
            None
        }
    }
}

fn read_hole_types(n: &mut Node, errs: &mut Vec<String>) -> Option<Vec<HoleType>> {
    let items = read_array(n, "hole_types", errs)?;
    if items.is_empty() {
        errs.push("hole_types: at least one is required".to_string());
    }
    let dict = Dictionary::bundled(DictVersion::V4_2);
    let loca = dict.abbr_codes("LOCA_TYPE");
    let mut out = Vec::new();
    for (mut h, _) in items {
        let code = req_str(&mut h, "code", errs);
        if let Some(c) = &code
            && !loca.contains(&c.as_str())
        {
            errs.push(format!("{}: \"{c}\" is not a LOCA_TYPE code", h.at("code")));
        }
        let weight = req_int(&mut h, "weight", errs).and_then(|w| {
            if let Ok(w) = u32::try_from(w)
                && w >= 1
            {
                Some(w)
            } else {
                errs.push(format!("{}: {w} must be an integer >= 1", h.at("weight")));
                None
            }
        });
        let depth = read_range(&mut h, "final_depth_m", errs);
        if let Some(d) = &depth
            && d.min <= 0.0
        {
            errs.push(format!(
                "{}: min {} must be > 0",
                h.at("final_depth_m"),
                d.min
            ));
        }
        h.finish(errs);
        if let (Some(code), Some(weight), Some(final_depth_m)) = (code, weight, depth) {
            out.push(HoleType {
                code,
                weight,
                final_depth_m,
            });
        }
    }
    Some(out)
}

fn read_units(n: &mut Node, errs: &mut Vec<String>) -> Option<Vec<Unit>> {
    let items = read_array(n, "units", errs)?;
    if items.is_empty() {
        errs.push("units: at least one is required".to_string());
    }
    let last = items.len().saturating_sub(1);
    let dict = Dictionary::bundled(DictVersion::V4_2);
    let samp = dict.abbr_codes("SAMP_TYPE");
    let mut ids: Vec<(String, usize)> = Vec::new();
    let mut out = Vec::new();
    for (mut u, i) in items {
        let id = req_str(&mut u, "id", errs);
        if let Some(id) = &id {
            if id.is_empty()
                || !id
                    .chars()
                    .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_')
            {
                errs.push(format!("{}: \"{id}\" is not [A-Z0-9_]+", u.at("id")));
            }
            if let Some((_, first)) = ids.iter().find(|(k, _)| k == id) {
                errs.push(format!(
                    "{}: \"{id}\" is already used by units[{first}]",
                    u.at("id")
                ));
            } else {
                ids.push((id.clone(), i));
            }
            // Name the unit in later messages, as in `units[1] (LC)`.
            u.path = format!("{} ({id})", u.path);
        }
        let name = req_str(&mut u, "name", errs);

        let thickness = if u.table.contains_key("thickness_m") {
            if i == last {
                u.get("thickness_m");
                errs.push(format!(
                    "{}: forbidden on the last unit (it runs to final depth)",
                    u.at("thickness_m")
                ));
                None
            } else {
                read_thickness(&mut u, "thickness_m", errs)
            }
        } else {
            if i != last {
                errs.push(format!(
                    "{}: required on every unit but the last",
                    u.at("thickness_m")
                ));
            }
            u.get("thickness_m");
            None
        };

        let description = read_description(&mut u, errs);
        let made_ground = read_made_ground(&mut u, errs);
        let water_bearing = read_water_bearing(
            &mut u,
            made_ground.as_ref().is_some_and(Option::is_some),
            errs,
        );
        let strike = read_unit_strike(
            &mut u,
            water_bearing.as_ref().is_some_and(Option::is_some),
            errs,
        );
        let sampling = read_sampling(&mut u, &samp, errs);
        let properties = required_properties(&mut u, description.as_ref(), errs);
        u.finish(errs);
        if let (
            Some(id),
            Some(name),
            Some(description),
            Some(made_ground),
            Some(water_bearing),
            Some(strike),
            Some(sampling),
            Some(properties),
        ) = (
            id,
            name,
            description,
            made_ground,
            water_bearing,
            strike,
            sampling,
            properties,
        ) && (i == last || thickness.is_some())
        {
            out.push(Unit {
                id,
                name,
                thickness_m: thickness,
                description,
                made_ground,
                water_bearing,
                strike,
                sampling,
                properties,
            });
        }
    }
    Some(out)
}

/// The optional `[[lenses]]`: empty when absent. Ids share one namespace with
/// the units'.
fn read_lenses(n: &mut Node, errs: &mut Vec<String>) -> Option<Vec<Lens>> {
    if !n.table.contains_key("lenses") {
        n.get("lenses");
        return Some(Vec::new());
    }
    let items = read_array(n, "lenses", errs)?;
    // Ids as written, whether or not the entry itself loaded.
    let unit_ids: Vec<&str> = match n.table.get("units") {
        Some(Value::Array(a)) => a
            .iter()
            .filter_map(|u| u.as_table()?.get("id")?.as_str())
            .collect(),
        _ => Vec::new(),
    };
    let lens_ids: Vec<Option<&str>> = match n.table.get("lenses") {
        Some(Value::Array(a)) => a
            .iter()
            .map(|l| l.as_table().and_then(|t| t.get("id")?.as_str()))
            .collect(),
        _ => Vec::new(),
    };
    let dict = Dictionary::bundled(DictVersion::V4_2);
    let samp = dict.abbr_codes("SAMP_TYPE");
    let mut out = Vec::new();
    for (mut l, i) in items {
        let id = req_str(&mut l, "id", errs);
        if let Some(id) = &id {
            if id.is_empty()
                || !id
                    .chars()
                    .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_')
            {
                errs.push(format!("{}: \"{id}\" is not [A-Z0-9_]+", l.at("id")));
            }
            let first_unit = unit_ids.iter().position(|u| u == id);
            let first_lens = lens_ids.iter().position(|x| *x == Some(id.as_str()));
            if let Some(u) = first_unit {
                errs.push(format!(
                    "{}: \"{id}\" is already used by units[{u}]",
                    l.at("id")
                ));
            } else if let Some(f) = first_lens
                && f < i
            {
                errs.push(format!(
                    "{}: \"{id}\" is already used by lenses[{f}]",
                    l.at("id")
                ));
            }
            l.path = format!("{} ({id})", l.path);
        }
        let name = req_str(&mut l, "name", errs);
        let host = req_str(&mut l, "host", errs);
        if let Some(h) = &host
            && !unit_ids.contains(&h.as_str())
        {
            if lens_ids.contains(&Some(h.as_str())) {
                errs.push(format!("{}: \"{h}\" is a lens, not a unit", l.at("host")));
            } else {
                errs.push(format!("{}: \"{h}\" is not a unit", l.at("host")));
            }
        }
        let radius = read_range(&mut l, "radius_m", errs);
        let thickness = read_range(&mut l, "thickness_m", errs);
        for (r, key) in [(&radius, "radius_m"), (&thickness, "thickness_m")] {
            if let Some(r) = r
                && r.min <= 0.0
            {
                errs.push(format!("{}: min {} must be > 0", l.at(key), r.min));
            }
        }
        let depth = read_range(&mut l, "depth_in_host", errs);
        if let Some(d) = &depth {
            if d.min < 0.0 {
                errs.push(format!(
                    "{}: min {} must be >= 0",
                    l.at("depth_in_host"),
                    d.min
                ));
            }
            if d.max > 1.0 {
                errs.push(format!(
                    "{}: max {} must be <= 1",
                    l.at("depth_in_host"),
                    d.max
                ));
            }
        }
        let description = read_description(&mut l, errs);
        let water_bearing = read_water_bearing(&mut l, false, errs);
        let sampling = read_sampling(&mut l, &samp, errs);
        let properties = required_properties(&mut l, description.as_ref(), errs);
        l.finish(errs);
        if let (
            Some(id),
            Some(name),
            Some(host),
            Some(radius_m),
            Some(thickness_m),
            Some(depth_in_host),
            Some(description),
            Some(water_bearing),
            Some(sampling),
            Some(properties),
        ) = (
            id,
            name,
            host,
            radius,
            thickness,
            depth,
            description,
            water_bearing,
            sampling,
            properties,
        ) {
            out.push(Lens {
                id,
                name,
                host,
                radius_m,
                thickness_m,
                depth_in_host,
                description,
                water_bearing,
                sampling,
                properties,
            });
        }
    }
    Some(out)
}

/// The optional `[[suite]]`: empty when absent.
fn read_suite(n: &mut Node, errs: &mut Vec<String>) -> Option<Vec<SuiteEntry>> {
    if !n.table.contains_key("suite") {
        n.get("suite");
        return Some(Vec::new());
    }
    let items = read_array(n, "suite", errs)?;
    // Unit and lens ids as written, whether or not the entry itself loaded.
    let ids: Vec<&str> = ["units", "lenses"]
        .iter()
        .filter_map(|k| match n.table.get(*k) {
            Some(Value::Array(a)) => Some(a),
            _ => None,
        })
        .flatten()
        .filter_map(|u| u.as_table()?.get("id")?.as_str())
        .collect();
    let groups_written: Vec<Option<&str>> = match n.table.get("suite") {
        Some(Value::Array(a)) => a
            .iter()
            .map(|e| e.as_table().and_then(|t| t.get("group")?.as_str()))
            .collect(),
        _ => Vec::new(),
    };
    let dict = Dictionary::bundled(DictVersion::V4_2);
    let mut out = Vec::new();
    for (mut e, i) in items {
        let group = req_str(&mut e, "group", errs);
        let mut group_ok = false;
        if let Some(g) = &group {
            if !SUITE_GROUPS.contains(&g.as_str()) {
                errs.push(format!(
                    "{}: \"{g}\" is not a suite group ({})",
                    e.at("group"),
                    SUITE_GROUPS.join(", ")
                ));
            } else if dict.group(g).is_none() {
                errs.push(format!(
                    "{}: \"{g}\" is not in the bundled dictionary",
                    e.at("group")
                ));
            } else if let Some(f) = groups_written.iter().position(|x| *x == Some(g.as_str()))
                && f < i
            {
                errs.push(format!(
                    "{}: suite[{f}] already has an entry for {g}",
                    e.at("group")
                ));
            } else {
                group_ok = true;
            }
        }
        let unsched = group.as_deref().and_then(unscheduled);
        if let (Some(g), Some((Some(parent), _))) = (&group, unsched)
            && !groups_written.contains(&Some(parent))
        {
            errs.push(format!("{}: {g} needs a {parent} entry", e.path));
        }
        let headings =
            read_suite_headings(&mut e, group.as_deref().filter(|_| group_ok), &dict, errs);
        let (units, rate) = if let Some((_, why)) = unsched {
            for key in ["units", "rate"] {
                if e.table.contains_key(key) {
                    errs.push(format!("{}: {why} and has no {key} of its own", e.at(key)));
                }
            }
            e.get("units");
            e.get("rate");
            (Some(Vec::new()), Some(None))
        } else {
            (
                read_suite_units(&mut e, &ids, errs),
                read_suite_rate(&mut e, errs).map(Some),
            )
        };
        e.finish(errs);
        if let (Some(group), Some(headings), Some(units), Some(rate)) =
            (group, headings, units, rate)
        {
            out.push(SuiteEntry {
                group,
                headings,
                units,
                rate,
            });
        }
    }
    Some(out)
}

/// `headings`: each must be a heading of `group` beyond KEY and REQUIRED, and
/// some registered producer must make it.
fn read_suite_headings(
    e: &mut Node,
    group: Option<&str>,
    dict: &Dictionary<'static>,
    errs: &mut Vec<String>,
) -> Option<Vec<String>> {
    let list = str_list(e, "headings", errs)?;
    let Some(group) = group else {
        return Some(list);
    };
    let known = dict.group_headings(group);
    for (i, h) in list.iter().enumerate() {
        let at = format!("{}[{i}]", e.at("headings"));
        let entry = known.iter().any(|k| k == h).then(|| dict.heading(group, h));
        match entry.flatten() {
            None => errs.push(format!("{at}: \"{h}\" is not a heading of {group}")),
            Some(x) if x.status.contains("KEY") || x.status.contains("REQUIRED") => errs.push(
                format!("{at}: {h} is a KEY or REQUIRED heading, which is always written"),
            ),
            Some(_) if suite::producer(group, h).is_none() => {
                errs.push(format!("{at}: no producer makes {h}"));
            }
            Some(_) => {}
        }
    }
    Some(list)
}

fn read_suite_units(e: &mut Node, ids: &[&str], errs: &mut Vec<String>) -> Option<Vec<String>> {
    let list = str_list(e, "units", errs)?;
    if list.is_empty() {
        errs.push(format!("{}: must not be empty", e.at("units")));
    }
    for (i, u) in list.iter().enumerate() {
        if !ids.contains(&u.as_str()) {
            errs.push(format!(
                "{}[{i}]: \"{u}\" is not a unit or lens",
                e.at("units")
            ));
        }
    }
    Some(list)
}

fn read_suite_rate(e: &mut Node, errs: &mut Vec<String>) -> Option<Range> {
    let r = read_range(e, "rate", errs)?;
    for (which, v) in [("min", r.min), ("max", r.max)] {
        if !(0.0..=1.0).contains(&v) {
            errs.push(format!("{}: {which} {v} is outside [0, 1]", e.at("rate")));
        }
    }
    Some(r)
}

/// Reads the optional `water_bearing` of a unit or lens: `true` (share 1),
/// `false` (absent), or a table `{ share = ... }` read like a made-ground
/// share. `made_ground` is whether the unit is made ground, which never
/// yields water.
// Outer `None` is a reported problem; inner `None` is "not water-bearing".
#[allow(clippy::option_option)]
fn read_water_bearing(
    n: &mut Node,
    made_ground: bool,
    errs: &mut Vec<String>,
) -> Option<Option<Range>> {
    let at = n.at("water_bearing");
    let refuse = |errs: &mut Vec<String>| {
        errs.push(format!(
            "{at}: not allowed on a made-ground unit (Made Ground never yields a water strike)"
        ));
        None
    };
    match n.get("water_bearing") {
        None | Some(Value::Boolean(false)) => Some(None),
        Some(Value::Boolean(true) | Value::Table(_)) if made_ground => refuse(errs),
        Some(Value::Boolean(true)) => Some(Some(Range {
            min: 1.0,
            max: 1.0,
            basis: String::new(),
            skew: None,
        })),
        Some(Value::Table(t)) => {
            let mut w = Node::new(t, at);
            let share = read_share(&mut w, errs);
            w.finish(errs);
            share.map(Some)
        }
        Some(_) => {
            errs.push(format!("{at}: must be true, false or a table with a share"));
            None
        }
    }
}

/// Reads the optional `strike` table of a unit: a rate (`rate_per_m`, strikes
/// per metre penetrated, or `rate_by_depth`, one such rate per band of depth;
/// never both, never negative), `rise_m` (never negative, with an optional
/// skew) and `no_rise_share` (0 to 1), each cited. It is refused on a
/// water-bearing unit (which already strikes at its top). A made-ground unit
/// may take one.
// Outer `None` is a reported problem; inner `None` is "not written".
#[allow(clippy::option_option)]
fn read_unit_strike(
    u: &mut Node,
    water_bearing: bool,
    errs: &mut Vec<String>,
) -> Option<Option<UnitStrike>> {
    let at = u.at("strike");
    let Some(v) = u.get("strike") else {
        return Some(None);
    };
    let Value::Table(t) = v else {
        errs.push(format!("{at}: must be a table"));
        return None;
    };
    if water_bearing {
        errs.push(format!(
            "{at}: not allowed on a water-bearing unit (it already strikes at its top)"
        ));
        return None;
    }
    let mut s = Node::new(t, at);
    let mut fine = true;
    let rate = read_strike_rate(&mut s, errs);
    fine &= rate.is_some();
    let rise = read_thickness(&mut s, "rise_m", errs);
    if let Some(r) = &rise {
        let before = errs.len();
        check_domain(r, &s.at("rise_m"), &props::STRIKE_RISE_M, errs);
        fine &= errs.len() == before;
    }
    let no_rise = read_range(&mut s, "no_rise_share", errs);
    if let Some(r) = &no_rise {
        for (which, v) in [("min", r.min), ("max", r.max)] {
            if !(0.0..=1.0).contains(&v) {
                errs.push(format!(
                    "{}: {which} {v} is outside 0 to 1",
                    s.at("no_rise_share")
                ));
                fine = false;
            }
        }
    }
    s.finish(errs);
    match (rate, rise, no_rise) {
        (Some(rate), Some(rise_m), Some(no_rise_share)) if fine => Some(Some(UnitStrike {
            rate,
            rise_m,
            no_rise_share,
        })),
        _ => None,
    }
}

/// The rate of a `strike` table: exactly one of `rate_per_m` and
/// `rate_by_depth`.
fn read_strike_rate(s: &mut Node, errs: &mut Vec<String>) -> Option<StrikeRate> {
    let has_flat = s.table.contains_key("rate_per_m");
    let has_bands = s.table.contains_key("rate_by_depth");
    if has_flat && has_bands {
        // Both are consumed so only this error is reported for them.
        s.get("rate_per_m");
        s.get("rate_by_depth");
        errs.push(format!(
            "{}: has both rate_per_m and rate_by_depth (write one)",
            s.path
        ));
        return None;
    }
    if has_bands {
        return read_rate_by_depth(s, errs).map(StrikeRate::ByDepth);
    }
    let rate = read_range(s, "rate_per_m", errs)?;
    let before = errs.len();
    check_domain(&rate, &s.at("rate_per_m"), &props::STRIKE_RATE_PER_M, errs);
    (errs.len() == before).then_some(StrikeRate::PerMetre(rate))
}

/// `rate_by_depth = [{ from_m, rate_per_m = { min, max, basis } }, ...]`: the
/// first band from 0, each next from deeper, every rate non-negative.
fn read_rate_by_depth(s: &mut Node, errs: &mut Vec<String>) -> Option<Vec<RateBand>> {
    let at = s.at("rate_by_depth");
    let Some(Value::Array(items)) = s.get("rate_by_depth") else {
        errs.push(format!("{at}: must be an array of tables"));
        return None;
    };
    if items.is_empty() {
        errs.push(format!("{at}: must not be empty"));
        return None;
    }
    let mut out: Vec<RateBand> = Vec::new();
    let mut ok = true;
    for (i, v) in items.iter().enumerate() {
        let Value::Table(t) = v else {
            errs.push(format!("{at}[{i}]: must be a table"));
            ok = false;
            continue;
        };
        let mut b = Node::new(t, format!("{at}[{i}]"));
        let from = num(b.get("from_m"), &b.at("from_m"), errs);
        let rate = read_range(&mut b, "rate_per_m", errs);
        if let Some(r) = &rate {
            let before = errs.len();
            check_domain(r, &b.at("rate_per_m"), &props::STRIKE_RATE_PER_M, errs);
            ok &= errs.len() == before;
        }
        b.finish(errs);
        let (Some(from_m), Some(rate_per_m)) = (from, rate) else {
            ok = false;
            continue;
        };
        if i == 0 && from_m != 0.0 {
            errs.push(format!("{at}[0].from_m: {from_m} must be 0 (ground level)"));
            ok = false;
        } else if let Some(prev) = out.last()
            && from_m <= prev.from_m
        {
            errs.push(format!(
                "{at}[{i}].from_m: {from_m} must be deeper than the band above ({})",
                prev.from_m
            ));
            ok = false;
        }
        out.push(RateBand { from_m, rate_per_m });
    }
    ok.then_some(out)
}

/// Reads a unit's optional `made_ground`: `true` for the lane's default
/// draw, or a table with a `constituents` list and/or a `count` distribution.
/// The outer `None` is a problem already reported.
// Outer `None` is a reported problem; inner `None` is "not written".
#[allow(clippy::option_option)]
fn read_made_ground(u: &mut Node, errs: &mut Vec<String>) -> Option<Option<MadeGroundSpec>> {
    let at = u.at("made_ground");
    match u.get("made_ground") {
        None => Some(None),
        Some(Value::Boolean(true)) => Some(Some(MadeGroundSpec::default())),
        Some(Value::Table(t)) => {
            let mut m = Node::new(t, at);
            let constituents = read_constituent_shares(&mut m, errs);
            let count = read_count(&mut m, errs);
            let amounts = read_amounts(&mut m, errs);
            if t.is_empty() {
                errs.push(format!(
                    "{}: empty (write `true` for the default draw)",
                    m.path
                ));
            }
            m.finish(errs);
            Some(Some(MadeGroundSpec {
                constituents: constituents?,
                count: count?,
                amounts: amounts?,
            }))
        }
        Some(_) => {
            errs.push(format!("{at}: must be true or a table"));
            None
        }
    }
}

/// `constituents = [{ name, share }, ...]`, absent meaning the default list.
// Outer `None` is a reported problem; inner `None` is "not written".
#[allow(clippy::option_option)]
fn read_constituent_shares(
    m: &mut Node,
    errs: &mut Vec<String>,
) -> Option<Option<Vec<ConstituentShare>>> {
    let at = m.at("constituents");
    let items = match m.get("constituents") {
        None => return Some(None),
        Some(Value::Array(a)) => a,
        Some(_) => {
            errs.push(format!("{at}: must be an array of tables"));
            return None;
        }
    };
    if items.is_empty() {
        errs.push(format!("{at}: must not be empty"));
    }
    let known = made_ground::constituents();
    let mut out: Vec<ConstituentShare> = Vec::new();
    let mut ok = !items.is_empty();
    for (i, v) in items.iter().enumerate() {
        let Value::Table(t) = v else {
            errs.push(format!("{at}[{i}]: must be a table"));
            ok = false;
            continue;
        };
        let mut c = Node::new(t, format!("{at}[{i}]"));
        let name = req_str(&mut c, "name", errs);
        if let Some(n) = &name {
            if !known.vocabulary().any(|k| k == n) {
                let list: Vec<&str> = known.vocabulary().map(String::as_str).collect();
                errs.push(format!(
                    "{}: unknown constituent \"{n}\" (known: {})",
                    c.at("name"),
                    list.join(", ")
                ));
                ok = false;
            } else if out.iter().any(|o| &o.name == n) {
                errs.push(format!("{}: \"{n}\" is listed twice", c.at("name")));
                ok = false;
            }
        }
        let share = read_share(&mut c, errs);
        c.finish(errs);
        match (name, share) {
            (Some(name), Some(share)) => out.push(ConstituentShare { name, share }),
            _ => ok = false,
        }
    }
    ok.then_some(Some(out))
}

/// `share = { min, max, basis }`, or a plain number with the entry's `basis`.
/// Either way each bound is a fraction in `[0, 1]`.
fn read_share(c: &mut Node, errs: &mut Vec<String>) -> Option<Range> {
    let at = c.at("share");
    let range = match c.get("share") {
        None => {
            errs.push(format!("{at}: missing"));
            return None;
        }
        Some(Value::Table(t)) => {
            let mut r = Node::new(t, at.clone());
            let range = range_in(&mut r, errs);
            r.finish(errs);
            range?
        }
        Some(v) => {
            let share = num(Some(v), &at, errs);
            let basis = match c.get("basis") {
                None => {
                    errs.push(format!("{}: no basis", c.path));
                    None
                }
                Some(Value::String(s)) if !s.trim().is_empty() => Some(s.clone()),
                Some(_) => {
                    errs.push(format!("{}: basis must be a non-empty string", c.path));
                    None
                }
            };
            Range {
                min: share?,
                max: share?,
                basis: basis?,
                skew: None,
            }
        }
    };
    let mut fine = true;
    for (which, v) in [("min", range.min), ("max", range.max)] {
        if !(0.0..=1.0).contains(&v) {
            errs.push(format!("{at}: {which} {v} is outside 0 to 1"));
            fine = false;
        }
    }
    fine.then_some(range)
}

/// `count = { weights = [w0, w1, w2, w3plus], basis }`, absent meaning the
/// default lane's one or two.
// Outer `None` is a reported problem; inner `None` is "not written".
#[allow(clippy::option_option)]
fn read_count(m: &mut Node, errs: &mut Vec<String>) -> Option<Option<CountWeights>> {
    let w = read_weights::<4>(m, "count", "0, 1, 2 and 3+ constituents", errs)?;
    Some(w.map(|(weights, basis)| CountWeights { weights, basis }))
}

/// `amounts = { weights = [rare, occasional, frequent, numerous, abundant],
/// basis }`, absent meaning equal weights.
// Outer `None` is a reported problem; inner `None` is "not written".
#[allow(clippy::option_option)]
fn read_amounts(m: &mut Node, errs: &mut Vec<String>) -> Option<Option<AmountWeights>> {
    let w = read_weights::<5>(
        m,
        "amounts",
        "rare, occasional, frequent, numerous and abundant",
        errs,
    )?;
    Some(w.map(|(weights, basis)| AmountWeights { weights, basis }))
}

/// `key = { weights = [N non-negative numbers, not all zero], basis }`.
// Outer `None` is a reported problem; inner `None` is "not written".
#[allow(clippy::option_option)]
fn read_weights<const N: usize>(
    m: &mut Node,
    key: &'static str,
    labels: &str,
    errs: &mut Vec<String>,
) -> Option<Option<([f64; N], String)>> {
    if !m.table.contains_key(key) {
        m.get(key);
        return Some(None);
    }
    let mut c = req_table(m, key, errs)?;
    let basis = match c.get("basis") {
        None => {
            errs.push(format!("{}: no basis", c.path));
            None
        }
        Some(Value::String(s)) if !s.trim().is_empty() => Some(s.clone()),
        Some(_) => {
            errs.push(format!("{}: basis must be a non-empty string", c.path));
            None
        }
    };
    let wat = c.at("weights");
    let weights = match c.get("weights") {
        None => {
            errs.push(format!("{wat}: missing"));
            None
        }
        Some(Value::Array(a)) if a.len() == N => {
            let nums: Vec<Option<f64>> = a
                .iter()
                .enumerate()
                .map(|(i, v)| num(Some(v), &format!("{wat}[{i}]"), errs))
                .collect();
            let w: Vec<f64> = nums.iter().flatten().copied().collect();
            if w.len() != N {
                None
            } else if let Some(i) = w.iter().position(|x| *x < 0.0) {
                errs.push(format!("{wat}[{i}]: {} is negative", w[i]));
                None
            } else if w.iter().sum::<f64>() <= 0.0 {
                errs.push(format!("{wat}: weights sum to zero"));
                None
            } else {
                let mut out = [0.0; N];
                out.copy_from_slice(&w);
                Some(out)
            }
        }
        Some(Value::Array(a)) => {
            errs.push(format!(
                "{wat}: needs {N} weights ({labels}), found {}",
                a.len()
            ));
            None
        }
        Some(_) => {
            errs.push(format!("{wat}: must be an array of {N} numbers"));
            None
        }
    };
    c.finish(errs);
    Some(Some((weights?, basis?)))
}

fn read_description(u: &mut Node, errs: &mut Vec<String>) -> Option<DescriptionSpec> {
    let mut d = req_table(u, "description", errs)?;
    let principals = str_list(&mut d, "principals", errs);
    let secondaries = str_list(&mut d, "secondaries", errs);
    let colours = str_list(&mut d, "colours", errs);
    for (list, key) in [(&principals, "principals"), (&colours, "colours")] {
        if let Some(l) = list
            && l.is_empty()
        {
            errs.push(format!("{}: must not be empty", d.at(key)));
        }
    }
    let very_coarse = read_very_coarse(&mut d, errs);
    d.finish(errs);
    Some(DescriptionSpec {
        principals: principals?,
        secondaries: secondaries?,
        colours: colours?,
        very_coarse: very_coarse?,
    })
}

/// A description's optional `very_coarse = { cobbles = "low", boulders =
/// "medium" }`: each side a level (low, medium or high) and either omitted,
/// but not both. The outer `None` is a problem already reported.
// Outer `None` is a reported problem; inner `None` is "not written".
#[allow(clippy::option_option)]
fn read_very_coarse(d: &mut Node, errs: &mut Vec<String>) -> Option<Option<VeryCoarse>> {
    let at = d.at("very_coarse");
    let t = match d.get("very_coarse") {
        None => return Some(None),
        Some(Value::Table(t)) => t,
        Some(_) => {
            errs.push(format!("{at}: must be a table"));
            return None;
        }
    };
    let mut v = Node::new(t, at);
    let mut level = |key: &'static str, errs: &mut Vec<String>| -> Option<Option<Content>> {
        let here = v.at(key);
        match v.get(key) {
            None => Some(None),
            Some(Value::String(s)) => {
                if let Some(c) = Content::parse(s) {
                    Some(Some(c))
                } else {
                    errs.push(format!(
                        "{here}: unknown level \"{s}\" (known: low, medium, high)"
                    ));
                    None
                }
            }
            Some(_) => {
                errs.push(format!("{here}: must be a string"));
                None
            }
        }
    };
    let cobbles = level("cobbles", errs);
    let boulders = level("boulders", errs);
    if t.is_empty() {
        errs.push(format!("{}: names neither cobbles nor boulders", v.path));
    }
    v.finish(errs);
    Some(Some(VeryCoarse {
        cobbles: cobbles?,
        boulders: boulders?,
    }))
}

fn read_sampling(u: &mut Node, samp: &[&str], errs: &mut Vec<String>) -> Option<Sampling> {
    let mut s = req_table(u, "sampling", errs)?;
    let types = str_list(&mut s, "types", errs);
    if let Some(t) = &types {
        if t.is_empty() {
            errs.push(format!("{}: must not be empty", s.at("types")));
        }
        for (i, c) in t.iter().enumerate() {
            if !samp.contains(&c.as_str()) {
                errs.push(format!(
                    "{}[{i}]: \"{c}\" is not a SAMP_TYPE code",
                    s.at("types")
                ));
            }
        }
    }
    let interval = read_range(&mut s, "interval_m", errs);
    if let Some(r) = &interval
        && r.min <= 0.0
    {
        errs.push(format!("{}: min {} must be > 0", s.at("interval_m"), r.min));
    }
    s.finish(errs);
    Some(Sampling {
        types: types?,
        interval_m: interval?,
    })
}

#[cfg(test)]
#[allow(clippy::float_cmp)] // fixture floats are exact literals
mod tests {
    use super::*;

    /// Every string under a `*basis*` key in `v`, with its path.
    fn bases<'a>(v: &'a Value, path: &str, out: &mut Vec<(String, &'a str)>) {
        match v {
            Value::Table(t) => {
                for (k, v) in t {
                    let at = format!("{path}.{k}");
                    match v {
                        Value::String(s) if k.contains("basis") => out.push((at, s)),
                        _ => bases(v, &at, out),
                    }
                }
            }
            Value::Array(a) => {
                for (i, v) in a.iter().enumerate() {
                    bases(v, &format!("{path}[{i}]"), out);
                }
            }
            _ => {}
        }
    }

    /// BGS NGDC data are OGL v3, which requires the attribution wherever they
    /// are used, so a basis citing the NGDC must carry it.
    #[test]
    fn every_shipped_basis_citing_the_bgs_ngdc_carries_the_ogl_attribution() {
        for (name, text) in SHIPPED {
            let root = Value::Table(text.parse().expect("shipped preset parses"));
            let mut all = Vec::new();
            bases(&root, name, &mut all);
            assert!(all.len() > 50, "{name}: only {} bases found", all.len());
            for (at, b) in all {
                if b.contains("NGDC") {
                    assert!(
                        b.to_lowercase()
                            .contains("contains data supplied by ukri, ogl v3"),
                        "{at}: cites the BGS NGDC without \
                         \"Contains data supplied by UKRI, OGL v3\""
                    );
                }
            }
        }
    }

    /// Obviously fake: no real citation belongs in a test fixture. Without
    /// properties or groundwater, which the loader requires; [`valid`] adds
    /// them.
    const BASE: &str = r#"
schema = 1
name = "test-basin"
version = 2
summary = "A fixture."

[site]
extent_m = { x = 400.0, y = 300 }
ground_level_m = { min = 8.0, max = 22.0, basis = "test fixture" }
groundwater_depth_m = { min = 1.0, max = 4.0, basis = "test fixture" }

[[hole_types]]
code = "CP"
weight = 3
final_depth_m = { min = 20.0, max = 40.0, basis = "test fixture" }

[[units]]
id = "RTD"
name = "River Terrace Deposits"
thickness_m = { min = -2.0, max = 6.0, basis = "test fixture" }
description = { principals = ["SAND", "GRAVEL"], secondaries = ["gravel"], colours = ["brown"] }
sampling = { types = ["B", "D"], interval_m = { min = 1.0, max = 1.5, basis = "test fixture" } }

[[units]]
id = "LC"
name = "London Clay"
description = { principals = ["CLAY"], secondaries = [], colours = ["grey"] }
sampling = { types = ["U"], interval_m = { min = 1.5, max = 3.0, basis = "test fixture" } }
"#;

    fn errors_of(text: &str) -> Vec<String> {
        Preset::from_toml(text).expect_err("should be refused").0
    }

    /// The complete fixture: [`BASE`] with a properties block on each unit.
    fn valid() -> String {
        with_props(RTD_PROPS, LC_PROPS)
    }

    /// The fixture with one exact snippet replaced, so each test changes one rule.
    fn with(from: &str, to: &str) -> String {
        let valid = valid();
        assert!(valid.contains(from), "fixture lacks {from:?}");
        valid.replacen(from, to, 1)
    }

    fn assert_refused(text: &str, expected: &str) {
        assert_eq!(errors_of(text), vec![expected.to_string()]);
    }

    #[test]
    fn a_valid_preset_loads_into_typed_values() {
        let p = Preset::from_toml(&valid()).expect("valid");
        assert_eq!(p.name, "test-basin");
        assert_eq!(p.version, 2);
        assert_eq!(p.summary, "A fixture.");
        assert_eq!(p.site.extent_m.x, 400.0);
        assert_eq!(p.site.extent_m.y, 300.0);
        assert_eq!(
            p.site.ground_level_m,
            Range {
                min: 8.0,
                max: 22.0,
                basis: "test fixture".into(),
                skew: None
            }
        );
        assert_eq!(p.hole_types.len(), 1);
        assert_eq!(p.hole_types[0].code, "CP");
        assert_eq!(p.hole_types[0].weight, 3);
        assert_eq!(p.hole_types[0].final_depth_m.max, 40.0);
        assert_eq!(p.units.len(), 2);
        assert_eq!(p.units[0].id, "RTD");
        assert_eq!(p.units[0].thickness_m.as_ref().unwrap().min, -2.0);
        assert_eq!(p.units[0].description.principals, ["SAND", "GRAVEL"]);
        assert_eq!(p.units[0].sampling.types, ["B", "D"]);
        assert_eq!(p.units[0].sampling.interval_m.max, 1.5);
        assert_eq!(p.units[1].id, "LC");
        assert_eq!(p.units[1].thickness_m, None);
        assert!(p.units[1].description.secondaries.is_empty());
    }

    const LENS: &str = r#"
[[lenses]]
id = "SB"
name = "Sand band"
host = "LC"
radius_m = { min = 30.0, max = 80.0, basis = "test fixture" }
thickness_m = { min = 0.3, max = 1.5, basis = "test fixture" }
depth_in_host = { min = 0.2, max = 0.8, basis = "test fixture" }
description = { principals = ["SAND"], secondaries = ["silt"], colours = ["brown"] }
sampling = { types = ["D"], interval_m = { min = 0.5, max = 1.0, basis = "test fixture" } }

[lenses.properties]
spt_n = { min = 5, max = 20, basis = "test fixture" }
moisture_content_percent = { min = 10, max = 20, basis = "test fixture" }
particle_density_ratio = { min = 2.6, max = 2.7, basis = "test fixture" }
"#;

    fn with_lens() -> String {
        format!("{}{LENS}", valid())
    }

    /// The lens fixture with one exact snippet replaced.
    fn lens_with(from: &str, to: &str) -> String {
        let text = with_lens();
        assert!(LENS.contains(from), "lens fixture lacks {from:?}");
        text.replacen(from, to, 1)
    }

    #[test]
    fn a_preset_without_lenses_has_none() {
        assert!(Preset::from_toml(&valid()).unwrap().lenses.is_empty());
    }

    #[test]
    fn a_valid_lens_loads_into_typed_values() {
        let p = Preset::from_toml(&with_lens()).expect("valid");
        assert_eq!(p.lenses.len(), 1);
        let l = &p.lenses[0];
        assert_eq!(
            (l.id.as_str(), l.name.as_str(), l.host.as_str()),
            ("SB", "Sand band", "LC")
        );
        assert_eq!((l.radius_m.min, l.radius_m.max), (30.0, 80.0));
        assert_eq!((l.thickness_m.min, l.thickness_m.max), (0.3, 1.5));
        assert_eq!((l.depth_in_host.min, l.depth_in_host.max), (0.2, 0.8));
        assert_eq!(l.description.principals, ["SAND"]);
        assert_eq!(l.sampling.types, ["D"]);
    }

    #[test]
    fn a_lens_with_an_unknown_host_is_refused() {
        assert_refused(
            &lens_with("host = \"LC\"", "host = \"XX\""),
            "lenses[0] (SB).host: \"XX\" is not a unit",
        );
    }

    #[test]
    fn a_lens_cannot_host_a_lens() {
        let text = format!(
            "{}{}",
            with_lens(),
            LENS.replace("\"SB\"", "\"SB2\"")
                .replace("host = \"LC\"", "host = \"SB\"")
        );
        assert_refused(&text, "lenses[1] (SB2).host: \"SB\" is a lens, not a unit");
    }

    #[test]
    fn a_lens_id_must_be_unique_across_units_and_lenses() {
        assert_refused(
            &lens_with("id = \"SB\"", "id = \"RTD\""),
            "lenses[0].id: \"RTD\" is already used by units[0]",
        );
        let text = format!("{}{LENS}", with_lens());
        assert_refused(&text, "lenses[1].id: \"SB\" is already used by lenses[0]");
    }

    #[test]
    fn lens_ranges_outside_their_domain_are_refused() {
        assert_refused(
            &lens_with("radius_m = { min = 30.0", "radius_m = { min = 0.0"),
            "lenses[0] (SB).radius_m: min 0 must be > 0",
        );
        assert_refused(
            &lens_with("thickness_m = { min = 0.3", "thickness_m = { min = 0.0"),
            "lenses[0] (SB).thickness_m: min 0 must be > 0",
        );
        assert_refused(
            &lens_with(
                "depth_in_host = { min = 0.2",
                "depth_in_host = { min = -0.1",
            ),
            "lenses[0] (SB).depth_in_host: min -0.1 must be >= 0",
        );
        assert_refused(
            &lens_with("min = 0.2, max = 0.8", "min = 0.2, max = 1.2"),
            "lenses[0] (SB).depth_in_host: max 1.2 must be <= 1",
        );
    }

    #[test]
    fn a_lens_without_a_basis_or_with_an_unknown_key_is_refused() {
        assert_refused(
            &lens_with(
                ", basis = \"test fixture\" }\nthickness_m",
                " }\nthickness_m",
            ),
            "lenses[0] (SB).radius_m: no basis",
        );
        assert_refused(
            &lens_with("host = \"LC\"", "host = \"LC\"\nhosts = 1"),
            "lenses[0] (SB): unknown key \"hosts\"",
        );
        assert_refused(
            &lens_with("host = \"LC\"\n", ""),
            "lenses[0] (SB).host: missing",
        );
    }

    #[test]
    fn unknown_schema_is_refused() {
        assert_refused(
            &with("schema = 1", "schema = 2"),
            "schema: 2 is not supported (this forge reads 1)",
        );
    }

    #[test]
    fn missing_version_is_refused() {
        assert_refused(&with("version = 2\n", ""), "version: missing");
    }

    #[test]
    fn a_range_with_no_basis_is_refused() {
        assert_refused(
            &with(
                "thickness_m = { min = -2.0, max = 6.0, basis = \"test fixture\" }",
                "thickness_m = { min = -2.0, max = 6.0 }",
            ),
            "units[0] (RTD).thickness_m: no basis",
        );
    }

    #[test]
    fn an_empty_basis_is_refused() {
        assert_refused(
            &with(
                "ground_level_m = { min = 8.0, max = 22.0, basis = \"test fixture\" }",
                "ground_level_m = { min = 8.0, max = 22.0, basis = \"\" }",
            ),
            "site.ground_level_m: empty basis",
        );
    }

    #[test]
    fn a_range_with_min_above_max_is_refused() {
        assert_refused(
            &with(
                "final_depth_m = { min = 20.0, max = 40.0,",
                "final_depth_m = { min = 50.0, max = 40.0,",
            ),
            "hole_types[0].final_depth_m: min 50 is greater than max 40",
        );
    }

    #[test]
    fn a_duplicate_unit_id_is_refused() {
        assert_refused(
            &with("id = \"LC\"", "id = \"RTD\""),
            "units[1].id: \"RTD\" is already used by units[0]",
        );
    }

    #[test]
    fn thickness_on_the_last_unit_is_refused() {
        let text = valid().replace(
            "description = { principals = [\"CLAY\"]",
            "thickness_m = { min = 1.0, max = 2.0, basis = \"test fixture\" }\ndescription = { principals = [\"CLAY\"]",
        );
        assert_refused(
            &text,
            "units[1] (LC).thickness_m: forbidden on the last unit (it runs to final depth)",
        );
    }

    const RTD_THICKNESS: &str = "thickness_m = { min = -2.0, max = 6.0, basis = \"test fixture\" }";

    fn skew_text(skew: &str) -> String {
        with(
            RTD_THICKNESS,
            &format!(
                "thickness_m = {{ min = -2.0, max = 6.0, skew = {skew}, basis = \"test fixture\" }}"
            ),
        )
    }

    #[test]
    fn a_unit_thickness_may_carry_a_skew() {
        for (text, want) in [("2.0", 2.0), ("3", 3.0), ("1", 1.0)] {
            let p = Preset::from_toml(&skew_text(text)).expect("valid");
            assert_eq!(p.units[0].thickness_m.as_ref().unwrap().skew, Some(want));
        }
        let p = Preset::from_toml(&valid()).expect("valid");
        assert_eq!(p.units[0].thickness_m.as_ref().unwrap().skew, None);
    }

    #[test]
    fn a_skew_below_one_or_not_a_number_is_refused() {
        assert_refused(
            &skew_text("0"),
            "units[0] (RTD).thickness_m.skew: 0 must be >= 1",
        );
        assert_refused(
            &skew_text("0.5"),
            "units[0] (RTD).thickness_m.skew: 0.5 must be >= 1",
        );
        assert_refused(
            &skew_text("-1.5"),
            "units[0] (RTD).thickness_m.skew: -1.5 must be >= 1",
        );
        assert_refused(
            &skew_text("\"2\""),
            "units[0] (RTD).thickness_m.skew: must be a number",
        );
    }

    #[test]
    fn a_skew_is_refused_off_unit_thickness() {
        assert_refused(
            &with(
                "ground_level_m = { min = 8.0, max = 22.0,",
                "ground_level_m = { min = 8.0, max = 22.0, skew = 2.0,",
            ),
            "site.ground_level_m: unknown key \"skew\"",
        );
    }

    #[test]
    fn missing_thickness_on_a_non_last_unit_is_refused() {
        assert_refused(
            &with(
                "thickness_m = { min = -2.0, max = 6.0, basis = \"test fixture\" }\n",
                "",
            ),
            "units[0] (RTD).thickness_m: required on every unit but the last",
        );
    }

    #[test]
    fn an_unknown_loca_type_is_refused() {
        assert_refused(
            &with("code = \"CP\"", "code = \"XX\""),
            "hole_types[0].code: \"XX\" is not a LOCA_TYPE code",
        );
    }

    #[test]
    fn an_unknown_samp_type_is_refused() {
        assert_refused(
            &with("types = [\"U\"]", "types = [\"U\", \"ZZZ\"]"),
            "units[1] (LC).sampling.types[1]: \"ZZZ\" is not a SAMP_TYPE code",
        );
    }

    #[test]
    fn an_unknown_key_is_refused() {
        assert_refused(
            &with("weight = 3", "weight = 3\nwieght = 4"),
            "hole_types[0]: unknown key \"wieght\"",
        );
        assert_refused(
            &with(
                "summary = \"A fixture.\"",
                "summary = \"A fixture.\"\nextra = 1",
            ),
            "preset: unknown key \"extra\"",
        );
    }

    #[test]
    fn empty_principals_and_colours_are_refused() {
        let text = with(
            "principals = [\"CLAY\"], secondaries = [], colours = [\"grey\"]",
            "principals = [], secondaries = [], colours = []",
        );
        assert_eq!(
            errors_of(&text),
            vec![
                "units[1] (LC).description.principals: must not be empty".to_string(),
                "units[1] (LC).description.colours: must not be empty".to_string(),
            ]
        );
    }

    #[test]
    fn non_positive_numbers_are_refused() {
        assert_refused(
            &with("x = 400.0", "x = 0.0"),
            "site.extent_m.x: 0 must be > 0",
        );
        assert_refused(
            &with("weight = 3", "weight = 0"),
            "hole_types[0].weight: 0 must be an integer >= 1",
        );
        assert_refused(
            &with("min = 20.0", "min = 0.0"),
            "hole_types[0].final_depth_m: min 0 must be > 0",
        );
        assert_refused(
            &with("interval_m = { min = 1.0", "interval_m = { min = 0.0"),
            "units[0] (RTD).sampling.interval_m: min 0 must be > 0",
        );
    }

    #[test]
    fn a_bad_name_or_unit_id_is_refused() {
        assert_refused(
            &with("name = \"test-basin\"", "name = \"Test Basin\""),
            "name: \"Test Basin\" is not [a-z0-9-]+",
        );
        assert_refused(
            &with("id = \"LC\"", "id = \"lc\""),
            "units[1].id: \"lc\" is not [A-Z0-9_]+",
        );
    }

    #[test]
    fn all_problems_are_reported_not_just_the_first() {
        let text = with("schema = 1", "schema = 3")
            .replace("code = \"CP\"", "code = \"XX\"")
            .replace("types = [\"U\"]", "types = [\"ZZZ\"]");
        assert_eq!(
            errors_of(&text),
            vec![
                "schema: 3 is not supported (this forge reads 1)".to_string(),
                "hole_types[0].code: \"XX\" is not a LOCA_TYPE code".to_string(),
                "units[1] (LC).sampling.types[0]: \"ZZZ\" is not a SAMP_TYPE code".to_string(),
            ]
        );
    }

    #[test]
    fn display_puts_each_problem_on_its_own_line() {
        let text = with("schema = 1", "schema = 3").replace("code = \"CP\"", "code = \"XX\"");
        let shown = Preset::from_toml(&text).unwrap_err().to_string();
        assert_eq!(
            shown,
            "schema: 3 is not supported (this forge reads 1)\n\
             hole_types[0].code: \"XX\" is not a LOCA_TYPE code"
        );
    }

    #[test]
    fn invalid_toml_is_refused() {
        let errs = errors_of("schema = ");
        assert_eq!(errs.len(), 1);
        assert!(errs[0].starts_with("toml: "));
    }

    #[test]
    fn resolve_reads_a_file_path() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("mine.toml");
        std::fs::write(&path, valid()).unwrap();
        let p = resolve(path.to_str().unwrap()).expect("loads");
        assert_eq!(p.name, "test-basin");
    }

    #[test]
    fn resolve_reports_file_problems() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("bad.toml");
        std::fs::write(&path, with("schema = 1", "schema = 9")).unwrap();
        let e = resolve(path.to_str().unwrap()).unwrap_err();
        assert_eq!(e.0, vec!["schema: 9 is not supported (this forge reads 1)"]);
    }

    #[test]
    fn resolve_of_an_unknown_name_lists_the_shipped_names() {
        let e = resolve("no-such-preset").unwrap_err();
        assert_eq!(e.0.len(), 1);
        assert!(
            e.0[0].starts_with("no-such-preset: not a shipped preset or a file (shipped presets: "),
            "{}",
            e.0[0]
        );
    }

    #[test]
    fn every_shipped_preset_loads() {
        for (name, text) in SHIPPED {
            if let Err(e) = Preset::from_toml(text) {
                panic!("shipped preset {name} is refused:\n{e}");
            }
            assert!(resolve(name).is_ok(), "{name} does not resolve by name");
        }
    }

    /// v7 (tyneside-till) and v8 (london-basin): installations, with every
    /// range cited to the crawl, a calibration or a judgement, and a BGS basis
    /// ending in the licence line.
    fn installation_is_cited(p: &Preset, site_share: f64) {
        let i = p
            .construction
            .as_ref()
            .and_then(|c| c.installation.as_ref())
            .expect("an installation table");
        assert_eq!(i.site_share.min, site_share);
        let kinds: Vec<(InstallKindName, u32)> = i.kinds.iter().map(|k| (k.kind, k.mm)).collect();
        assert_eq!(
            kinds,
            [
                (InstallKindName::Standpipe, 50),
                (InstallKindName::Piezometer, 19)
            ]
        );
        let bases = [
            &i.site_share.basis,
            &i.hole_share.basis,
            &i.kinds_basis,
            &i.zone_offset_m.basis,
            &i.zone_top_m.basis,
            &i.slotted_length_m.basis,
            &i.base_gap_m.basis,
            &i.cap_m.basis,
            &i.filter_basis,
        ];
        for b in bases {
            assert!(b.contains("forge analysis"), "{b}");
            assert!(
                b.ends_with("Contains data supplied by UKRI, OGL v3."),
                "{b}"
            );
        }
        assert!(i.kinds_basis.contains("pp.405-406"));
    }

    /// london-basin (#75) adds installations at v8; v9 (#84) keeps them.
    #[test]
    fn london_basin_v9_has_a_cited_installation_table() {
        let p = resolve("london-basin").unwrap();
        assert_eq!(p.version, 9);
        installation_is_cited(&p, 0.17);
    }

    /// london-basin v9 (#84): several strikes per hole, and a made-ground
    /// `strike` block with a banded rate, each range cited to the crawl.
    #[test]
    fn london_basin_v9_has_cited_made_ground_strikes() {
        let p = resolve("london-basin").unwrap();
        let several = p.site.several_strikes.as_ref().expect("v9 has the key");
        assert_eq!((several.max_per_hole, several.min_gap_m), (4, 0.5));
        assert!(several.basis.contains("OGL v3"), "{}", several.basis);
        let mg = p.units.iter().find(|u| u.id == "MG").unwrap();
        let s = mg.strike.as_ref().expect("made ground strikes");
        let StrikeRate::ByDepth(bands) = &s.rate else {
            panic!("a banded rate");
        };
        assert_eq!(
            bands.iter().map(|b| b.from_m).collect::<Vec<_>>(),
            [0.0, 1.0, 2.0, 3.0, 5.0]
        );
        assert_eq!(
            bands.iter().map(|b| b.rate_per_m.min).collect::<Vec<_>>(),
            [0.023, 0.033, 0.056, 0.104, 0.103]
        );
        assert_eq!((s.no_rise_share.min, s.no_rise_share.max), (0.34, 0.34));
        for r in s
            .rate
            .ranges()
            .into_iter()
            .chain([&s.rise_m, &s.no_rise_share])
        {
            assert!(r.basis.contains("OGL v3"), "{}", r.basis);
        }
        assert!(
            p.units
                .iter()
                .filter(|u| u.id != "MG")
                .all(|u| u.strike.is_none())
        );
    }

    /// tyneside-till (#13) loads by name as three tills over one another with
    /// a water-bearing sand and gravel lens in the grey till, and every range
    /// carries a real basis, not a placeholder.
    #[test]
    fn tyneside_till_loads_with_its_tills_and_lens() {
        let p = resolve("tyneside-till").unwrap();
        assert_eq!((p.name.as_str(), p.version), ("tyneside-till", 9));
        // v9: several strikes, and the seal and the zone share that go with
        // them (#79).
        let several = p.site.several_strikes.as_ref().expect("v9 has the key");
        assert_eq!((several.max_per_hole, several.min_gap_m), (4, 0.5));
        let c = p.construction.as_ref().expect("a construction block");
        let seal = c.seal.as_ref().expect("v9 has a seal");
        assert_eq!((seal.earlier_share, seal.last_share), (0.48, 0.25));
        let follows = c
            .installation
            .as_ref()
            .and_then(|i| i.zone_follows_first_share.as_ref())
            .expect("v9 has the zone share");
        assert_eq!((follows.min, follows.max), (0.54, 0.54));
        assert!(follows.basis.contains("THIN"), "{}", follows.basis);
        for basis in [
            &several.basis,
            &seal.basis,
            &seal.gap_m.basis,
            &follows.basis,
        ] {
            assert!(basis.contains("OGL v3"), "{basis}");
            assert!(basis.contains("163565"), "{basis}");
        }
        // v6: hole construction, with the NE crawl's casing weights.
        let c = p
            .construction
            .as_ref()
            .expect("v6 has a construction block");
        let sizes: Vec<u32> = c.casing.diameters.iter().map(|d| d.mm).collect();
        assert_eq!(sizes, [200, 150, 125]);
        installation_is_cited(&p, 0.53);
        let units: Vec<&str> = p.units.iter().map(|u| u.id.as_str()).collect();
        assert_eq!(units, ["WT", "RT", "GT"]);
        // v5: each till carries effective strength, and the suite tests it.
        for u in &p.units {
            let e = u.properties.effective_strength.as_ref().unwrap();
            assert!(
                e.c_eff_kpa.max <= 25.0 && e.phi_eff_deg.max <= 50.0,
                "{}",
                u.id
            );
            for r in [&e.c_eff_kpa, &e.phi_eff_deg, &e.skempton_af] {
                assert!(
                    r.basis.contains("forge calibration"),
                    "{}: {}",
                    u.id,
                    r.basis
                );
                assert!(
                    r.basis.contains("Contains data supplied by UKRI"),
                    "{}",
                    u.id
                );
            }
        }
        assert!(p.lenses[0].properties.effective_strength.is_none());
        let treg = p.suite.iter().find(|e| e.group == "TREG").unwrap();
        assert_eq!(treg.units, ["WT", "RT", "GT"]);
        assert!(p.suite.iter().any(|e| e.group == "TRET"));
        // london-basin is not given any (no source for its c' and phi').
        let london = resolve("london-basin").unwrap();
        assert!(
            london
                .units
                .iter()
                .all(|u| u.properties.effective_strength.is_none())
        );
        assert!(london.suite.iter().all(|e| e.group != "TREG"));
        assert!(p.units.last().unwrap().thickness_m.is_none());
        assert_eq!(p.lenses.len(), 1);
        let lens = &p.lenses[0];
        assert_eq!((lens.id.as_str(), lens.host.as_str()), ("GFSG", "GT"));
        let share = lens.water_bearing.as_ref().unwrap();
        // v9: no strike takes precedence over the lens any more, so its share
        // is the crawl's 36% again, without v8's "raised to 0.44".
        assert_eq!((share.min, share.max), (0.36, 0.36));
        assert!(share.basis.contains("89 of 249"), "{}", share.basis);
        assert!(!share.basis.contains("0.44"), "{}", share.basis);
        let text = SHIPPED
            .iter()
            .find(|(n, _)| *n == "tyneside-till")
            .unwrap()
            .1;
        for line in text.lines().filter(|l| l.contains("basis = ")) {
            for placeholder in ["\"calib\"", "\"BGS\"", "\"ICE\"", "\"EC7\"", "\"Head\""] {
                assert!(!line.contains(placeholder), "placeholder basis: {line}");
            }
            if line.contains("BGS NGDC") {
                assert!(line.contains("Contains data supplied by UKRI"), "{line}");
            }
        }
    }

    const RTD_PROPS: &str = r#"
[units.properties]
spt_n = { min = 8, max = 40, basis = "test fixture", per_m = { min = 0.5, max = 1.5, basis = "test fixture" } }
moisture_content_percent = { min = 8, max = 20, basis = "test fixture" }
particle_density_ratio = { min = 2.6, max = 2.7, basis = "test fixture" }
degree_of_saturation = { min = 0.6, max = 0.9, basis = "test fixture" }
"#;

    const LC_PROPS: &str = r#"
[units.properties]
plastic_limit_percent = { min = 20, max = 30, basis = "test fixture" }
plasticity_index_percent = { min = 30, max = 50, basis = "test fixture" }
liquidity_index = { min = -0.1, max = 0.5, basis = "test fixture" }
particle_density_ratio = { min = 2.65, max = 2.75, basis = "test fixture" }
outlier_rate = { min = 0.0, max = 0.05, basis = "test fixture" }
"#;

    /// The fixture with a properties block on each unit.
    fn with_props(rtd: &str, lc: &str) -> String {
        let text = BASE.replacen(
            "\n[[units]]\nid = \"LC\"",
            &format!("{rtd}\n[[units]]\nid = \"LC\""),
            1,
        );
        format!("{text}{lc}")
    }

    /// The valid property blocks with one exact snippet replaced.
    fn props_with(from: &str, to: &str) -> String {
        let both = format!("{RTD_PROPS}{LC_PROPS}");
        assert!(both.contains(from), "property fixtures lack {from:?}");
        let (rtd, lc) = (
            RTD_PROPS.replacen(from, to, 1),
            LC_PROPS.replacen(from, to, 1),
        );
        with_props(&rtd, &lc)
    }

    #[test]
    fn a_valid_properties_block_loads() {
        let p = Preset::from_toml(&valid()).expect("valid");
        let coarse = &p.units[0].properties;
        assert!(coarse.fine.is_none());
        let c = coarse.coarse.as_ref().unwrap();
        assert_eq!((c.spt_n.range.min, c.spt_n.range.max), (8.0, 40.0));
        assert_eq!(c.spt_n.per_m.as_ref().unwrap().max, 1.5);
        assert_eq!(coarse.degree_of_saturation.as_ref().unwrap().range.min, 0.6);
        assert_eq!(coarse.outlier_rate, None);
        let fine = &p.units[1].properties;
        assert!(fine.coarse.is_none());
        let f = fine.fine.as_ref().unwrap();
        assert_eq!(f.liquidity_index.range.min, -0.1);
        assert_eq!(f.plastic_limit_percent.per_m, None);
        assert_eq!(fine.outlier_rate.as_ref().unwrap().max, 0.05);
        assert_eq!(fine.degree_of_saturation, None);
    }

    #[test]
    fn properties_are_required_on_every_unit() {
        let text = BASE.replacen(
            "\n[[units]]\nid = \"LC\"",
            &format!("{RTD_PROPS}\n[[units]]\nid = \"LC\""),
            1,
        );
        assert_refused(&text, "units[1] (LC).properties: missing");
        assert_refused(
            &with_props("", LC_PROPS),
            "units[0] (RTD).properties: missing",
        );
        let both_missing = errors_of(BASE);
        assert!(both_missing.contains(&"units[0] (RTD).properties: missing".to_string()));
        assert!(both_missing.contains(&"units[1] (LC).properties: missing".to_string()));
    }

    #[test]
    fn properties_are_required_on_a_lens() {
        let text = with_lens().replacen("[lenses.properties]", "[lenses.unrelated]", 1);
        let errs = errors_of(&text);
        assert!(
            errs.contains(&"lenses[0] (SB).properties: missing".to_string()),
            "{errs:?}"
        );
    }

    #[test]
    fn groundwater_depth_is_required_on_the_site() {
        let text = with(
            "groundwater_depth_m = { min = 1.0, max = 4.0, basis = \"test fixture\" }\n",
            "",
        );
        assert_refused(&text, "site.groundwater_depth_m: missing");
    }

    #[test]
    fn the_site_carries_a_groundwater_depth() {
        let g = Preset::from_toml(&valid())
            .unwrap()
            .site
            .groundwater_depth_m;
        assert_eq!((g.min, g.max), (1.0, 4.0));
    }

    const RISE: &str = "strike_rise_m = { min = 1.0, max = 2.0, basis = \"test fixture\" }\n";

    fn with_rise(rise: &str) -> String {
        with(
            "groundwater_depth_m = { min = 1.0, max = 4.0, basis = \"test fixture\" }\n",
            &format!(
                "groundwater_depth_m = {{ min = 1.0, max = 4.0, basis = \"test fixture\" }}\n{rise}"
            ),
        )
    }

    const SEVERAL: &str =
        "several_strikes = { max_per_hole = 4, min_gap_m = 0.5, basis = \"test fixture\" }\n";

    #[test]
    fn several_strikes_are_optional_and_load_when_given() {
        assert_eq!(
            Preset::from_toml(&valid()).unwrap().site.several_strikes,
            None
        );
        let r = Preset::from_toml(&with_rise(SEVERAL))
            .unwrap()
            .site
            .several_strikes
            .expect("given");
        assert_eq!((r.max_per_hole, r.min_gap_m), (4, 0.5));
        assert_eq!(r.basis, "test fixture");
    }

    #[test]
    fn several_strikes_need_a_cap_of_two_to_sixteen_a_positive_gap_and_a_basis() {
        for (from, to, want) in [
            (
                "max_per_hole = 4",
                "max_per_hole = 1",
                "site.several_strikes.max_per_hole: 1 is outside 2 to 16",
            ),
            (
                "max_per_hole = 4",
                "max_per_hole = 17",
                "site.several_strikes.max_per_hole: 17 is outside 2 to 16",
            ),
            (
                "max_per_hole = 4",
                "max_per_hole = 4.5",
                "site.several_strikes.max_per_hole: must be an integer",
            ),
            (
                "min_gap_m = 0.5",
                "min_gap_m = 0.0",
                "site.several_strikes.min_gap_m: 0 must be > 0",
            ),
            (
                "min_gap_m = 0.5",
                "min_gap_m = -1.0",
                "site.several_strikes.min_gap_m: -1 must be > 0",
            ),
            (
                "basis = \"test fixture\"",
                "basis = \" \"",
                "site.several_strikes.basis: empty",
            ),
            (
                ", basis = \"test fixture\"",
                "",
                "site.several_strikes.basis: missing",
            ),
        ] {
            let errs = errors_of(&with_rise(&SEVERAL.replace(from, to)));
            assert!(errs.contains(&want.to_string()), "{want}: {errs:?}");
        }
        let missing = with_rise(&SEVERAL.replace("min_gap_m = 0.5, ", ""));
        assert!(
            errors_of(&missing).contains(&"site.several_strikes.min_gap_m: missing".to_string())
        );
    }

    /// The shipped tyneside-till text with one exact snippet replaced.
    fn tyneside_with(from: &str, to: &str) -> String {
        let text = SHIPPED
            .iter()
            .find(|(n, _)| *n == "tyneside-till")
            .unwrap()
            .1;
        assert!(text.contains(from), "tyneside-till lacks {from:?}");
        text.replacen(from, to, 1)
    }

    #[test]
    fn a_seal_loads_with_two_shares_a_gap_and_a_basis() {
        let text = tyneside_with("earlier_share = 0.48", "earlier_share = 0.5");
        let seal = Preset::from_toml(&text).unwrap().construction.unwrap().seal;
        let seal = seal.expect("given");
        assert_eq!((seal.earlier_share, seal.last_share), (0.5, 0.25));
        assert_eq!((seal.gap_m.min, seal.gap_m.skew), (0.0, Some(1.4)));
        // A preset without the key has no seal.
        let text = tyneside_with("seal = {", "unused = {");
        let errs = errors_of(&text);
        assert!(
            errs.iter().any(|e| e.contains("unknown key \"unused\"")),
            "{errs:?}"
        );
    }

    #[test]
    fn a_seal_share_must_be_a_fraction_and_the_gap_never_negative() {
        for (from, to, want) in [
            (
                "earlier_share = 0.48",
                "earlier_share = 1.5",
                "construction.seal.earlier_share: 1.5 is outside 0 to 1",
            ),
            (
                "last_share = 0.25",
                "last_share = -0.1",
                "construction.seal.last_share: -0.1 is outside 0 to 1",
            ),
            (
                "gap_m = { min = 0.0,",
                "gap_m = { min = -0.5,",
                "construction.seal.gap_m: min -0.5 is negative",
            ),
            (
                "last_share = 0.25, ",
                "",
                "construction.seal.last_share: missing",
            ),
        ] {
            let errs = errors_of(&tyneside_with(from, to));
            assert!(errs.contains(&want.to_string()), "{want}: {errs:?}");
        }
    }

    #[test]
    fn a_zone_share_is_a_fraction_with_a_basis() {
        let from = "zone_follows_first_share = { min = 0.54, max = 0.54,";
        let text = tyneside_with(from, "zone_follows_first_share = { min = 0.4, max = 0.6,");
        let r = Preset::from_toml(&text)
            .unwrap()
            .construction
            .unwrap()
            .installation
            .unwrap()
            .zone_follows_first_share
            .unwrap();
        assert_eq!((r.min, r.max), (0.4, 0.6));
        let text = tyneside_with(from, "zone_follows_first_share = { min = 0.4, max = 1.6,");
        let errs = errors_of(&text);
        assert!(
            errs.contains(
                &"construction.installation.zone_follows_first_share: max 1.6 is outside 0 to 1"
                    .to_string()
            ),
            "{errs:?}"
        );
    }

    #[test]
    fn the_strike_rise_is_optional_and_loads_when_given() {
        assert_eq!(
            Preset::from_toml(&valid()).unwrap().site.strike_rise_m,
            None
        );
        let r = Preset::from_toml(&with_rise(RISE))
            .unwrap()
            .site
            .strike_rise_m
            .expect("given");
        assert_eq!((r.min, r.max), (1.0, 2.0));
    }

    #[test]
    fn the_strike_rise_needs_a_basis() {
        assert_refused(
            &with_rise("strike_rise_m = { min = 1.0, max = 2.0 }\n"),
            "site.strike_rise_m: no basis",
        );
    }

    #[test]
    fn the_strike_rise_may_not_be_negative() {
        assert_refused(
            &with_rise(&RISE.replace("min = 1.0", "min = -1.0")),
            "site.strike_rise_m: min -1 is outside its domain (>= 0)",
        );
    }

    #[test]
    fn the_strike_rise_min_may_not_exceed_its_max() {
        assert_refused(
            &with_rise(&RISE.replace("min = 1.0, max = 2.0", "min = 3.0, max = 2.0")),
            "site.strike_rise_m: min 3 is greater than max 2",
        );
    }

    #[test]
    fn a_lens_carries_properties() {
        let p = Preset::from_toml(&with_lens()).expect("valid");
        assert!(p.lenses[0].properties.coarse.is_some());
    }

    #[test]
    fn the_groundwater_depth_may_not_be_negative() {
        let text = with(
            "groundwater_depth_m = { min = 1.0,",
            "groundwater_depth_m = { min = -1.0,",
        );
        assert_refused(
            &text,
            "site.groundwater_depth_m: min -1 is outside its domain (>= 0)",
        );
    }

    #[test]
    fn a_coarse_unit_must_give_the_coarse_set() {
        let text = with_props(&RTD_PROPS.replace("spt_n = {", "spt_x = {"), LC_PROPS);
        let errs = errors_of(&text);
        assert_eq!(
            errs,
            vec![
                "units[0] (RTD).properties.spt_n: missing".to_string(),
                "units[0] (RTD).properties: unknown key \"spt_x\"".to_string(),
            ]
        );
    }

    #[test]
    fn a_fine_unit_must_give_the_fine_set() {
        let text = with_props(
            RTD_PROPS,
            &LC_PROPS.replace("liquidity_index", "liquidity_ndx"),
        );
        let errs = errors_of(&text);
        assert!(errs.contains(&"units[1] (LC).properties.liquidity_index: missing".to_string()));
    }

    #[test]
    fn particle_density_is_required() {
        let text = with_props(
            RTD_PROPS,
            &LC_PROPS.replace("particle_density_ratio", "particle_density"),
        );
        assert!(
            errors_of(&text)
                .contains(&"units[1] (LC).properties.particle_density_ratio: missing".to_string())
        );
    }

    #[test]
    fn a_property_the_unit_cannot_use_is_refused() {
        let fine_on_coarse = format!(
            "{RTD_PROPS}plastic_limit_percent = {{ min = 20, max = 30, basis = \"test fixture\" }}\n"
        );
        assert_refused(
            &with_props(&fine_on_coarse, LC_PROPS),
            "units[0] (RTD).properties.plastic_limit_percent: the unit has no fine principal (CLAY, SILT), so it cannot use this",
        );
        let coarse_on_fine =
            format!("{LC_PROPS}spt_n = {{ min = 20, max = 30, basis = \"test fixture\" }}\n");
        assert_refused(
            &with_props(RTD_PROPS, &coarse_on_fine),
            "units[1] (LC).properties.spt_n: the unit has no coarse principal (SAND, GRAVEL), so it cannot use this",
        );
    }

    #[test]
    fn a_unit_with_both_kinds_of_principal_needs_both_sets() {
        let text = with(
            "principals = [\"CLAY\"]",
            "principals = [\"CLAY\", \"SAND\"]",
        );
        assert_eq!(
            errors_of(&text),
            vec![
                "units[1] (LC).properties.spt_n: missing".to_string(),
                "units[1] (LC).properties.moisture_content_percent: missing".to_string(),
            ]
        );
    }

    #[test]
    fn properties_on_a_unit_with_no_fine_or_coarse_principal_are_refused() {
        let text = with("principals = [\"CLAY\"]", "principals = [\"PEAT\"]");
        assert_refused(
            &text,
            "units[1] (LC).properties: no principal is fine (CLAY, SILT) or coarse (SAND, GRAVEL), so no property set applies",
        );
    }

    #[test]
    fn an_unknown_property_key_is_refused() {
        let text = with_props(
            RTD_PROPS,
            &format!(
                "{LC_PROPS}undrained_strength = {{ min = 1, max = 2, basis = \"test fixture\" }}\n"
            ),
        );
        assert_refused(
            &text,
            "units[1] (LC).properties: unknown key \"undrained_strength\"",
        );
        let text = props_with(
            "min = 0.5, max = 1.5, basis = \"test fixture\" }",
            "min = 0.5, max = 1.5, basis = \"test fixture\", rate = 1 }",
        );
        assert_refused(
            &text,
            "units[0] (RTD).properties.spt_n.per_m: unknown key \"rate\"",
        );
    }

    #[test]
    fn a_property_without_a_basis_is_refused() {
        let text = with_props(
            &RTD_PROPS.replace(
                "moisture_content_percent = { min = 8, max = 20, basis = \"test fixture\" }",
                "moisture_content_percent = { min = 8, max = 20 }",
            ),
            LC_PROPS,
        );
        assert_refused(
            &text,
            "units[0] (RTD).properties.moisture_content_percent: no basis",
        );
        let text = props_with(
            "per_m = { min = 0.5, max = 1.5, basis = \"test fixture\" }",
            "per_m = { min = 0.5, max = 1.5 }",
        );
        assert_refused(&text, "units[0] (RTD).properties.spt_n.per_m: no basis");
    }

    #[test]
    fn ranges_outside_a_physical_domain_are_refused_by_name() {
        for (from, to, expected) in [
            (
                "plastic_limit_percent = { min = 20,",
                "plastic_limit_percent = { min = 0,",
                "units[1] (LC).properties.plastic_limit_percent: min 0 is outside its domain (> 0)",
            ),
            (
                "plasticity_index_percent = { min = 30, max = 50,",
                "plasticity_index_percent = { min = -5, max = 50,",
                "units[1] (LC).properties.plasticity_index_percent: min -5 is outside its domain (> 0)",
            ),
            (
                "liquidity_index = { min = -0.1, max = 0.5,",
                "liquidity_index = { min = -0.1, max = 3.5,",
                "units[1] (LC).properties.liquidity_index: max 3.5 is outside its domain (>= -1 and <= 3)",
            ),
            (
                "particle_density_ratio = { min = 2.65, max = 2.75,",
                "particle_density_ratio = { min = 0.9, max = 2.75,",
                "units[1] (LC).properties.particle_density_ratio: min 0.9 is outside its domain (> 1 and <= 4)",
            ),
            (
                "outlier_rate = { min = 0.0, max = 0.05,",
                "outlier_rate = { min = 0.0, max = 1.5,",
                "units[1] (LC).properties.outlier_rate: max 1.5 is outside its domain (>= 0 and <= 1)",
            ),
        ] {
            let text = with_props(RTD_PROPS, &LC_PROPS.replacen(from, to, 1));
            assert_refused(&text, expected);
        }
        for (from, to, expected) in [
            (
                "spt_n = { min = 8,",
                "spt_n = { min = -1,",
                "units[0] (RTD).properties.spt_n: min -1 is outside its domain (>= 0)",
            ),
            (
                "moisture_content_percent = { min = 8,",
                "moisture_content_percent = { min = -8,",
                "units[0] (RTD).properties.moisture_content_percent: min -8 is outside its domain (>= 0)",
            ),
            (
                "degree_of_saturation = { min = 0.6, max = 0.9,",
                "degree_of_saturation = { min = 0.6, max = 1.2,",
                "units[0] (RTD).properties.degree_of_saturation: max 1.2 is outside its domain (> 0 and <= 1)",
            ),
        ] {
            let text = with_props(&RTD_PROPS.replacen(from, to, 1), LC_PROPS);
            assert_refused(&text, expected);
        }
    }

    const SUITE: &str = r#"
[[suite]]
group = "LLPL"
headings = []
units = ["LC"]
rate = { min = 0.3, max = 0.5, basis = "test fixture" }

[[suite]]
group = "TRIG"
headings = []
units = ["LC"]
rate = { min = 0.1, max = 0.2, basis = "test fixture" }

[[suite]]
group = "TRIT"
headings = []

[[suite]]
group = "ISPT"
headings = []
units = ["RTD"]
rate = { min = 1, max = 1, basis = "test fixture" }
"#;

    fn with_suite() -> String {
        format!("{}{SUITE}", valid())
    }

    fn suite_with(from: &str, to: &str) -> String {
        assert!(SUITE.contains(from), "suite fixture lacks {from:?}");
        with_suite().replacen(from, to, 1)
    }

    fn suite_errors(text: &str) -> String {
        errors_of(text).join("\n")
    }

    #[test]
    fn a_preset_without_a_suite_has_none() {
        assert!(Preset::from_toml(&valid()).unwrap().suite.is_empty());
    }

    #[test]
    fn a_valid_suite_loads_into_typed_values() {
        let p = Preset::from_toml(&with_suite()).expect("valid");
        let groups: Vec<&str> = p.suite.iter().map(|e| e.group.as_str()).collect();
        assert_eq!(groups, ["LLPL", "TRIG", "TRIT", "ISPT"]);
        assert_eq!(p.suite[0].units, ["LC"]);
        let rate = p.suite[0].rate.as_ref().unwrap();
        assert_eq!((rate.min, rate.max), (0.3, 0.5));
        assert!(p.suite[2].rate.is_none() && p.suite[2].units.is_empty());
    }

    #[test]
    fn a_suite_group_outside_the_scope_is_refused() {
        let e = suite_errors(&suite_with("\"LLPL\"", "\"CMPG\""));
        assert!(
            e.contains("suite[0].group: \"CMPG\" is not a suite group"),
            "{e}"
        );
        let e = suite_errors(&suite_with("\"LLPL\"", "\"NOPE\""));
        assert!(
            e.contains("suite[0].group: \"NOPE\" is not a suite group"),
            "{e}"
        );
    }

    #[test]
    fn an_unknown_unit_id_is_refused() {
        let e = suite_errors(&suite_with(
            "units = [\"LC\"]\nrate = { min = 0.3",
            "units = [\"ZZ\"]\nrate = { min = 0.3",
        ));
        assert!(
            e.contains("suite[0].units[0]: \"ZZ\" is not a unit or lens"),
            "{e}"
        );
    }

    #[test]
    fn a_rate_outside_zero_to_one_is_refused() {
        let e = suite_errors(&suite_with("min = 0.3, max = 0.5", "min = 0.3, max = 1.5"));
        assert!(
            e.contains("suite[0].rate: max 1.5 is outside [0, 1]"),
            "{e}"
        );
        let e = suite_errors(&suite_with("min = 0.3, max = 0.5", "min = -0.1, max = 0.5"));
        assert!(
            e.contains("suite[0].rate: min -0.1 is outside [0, 1]"),
            "{e}"
        );
    }

    #[test]
    fn a_trit_entry_needs_trig_and_has_no_rate_or_units() {
        let no_trig = with_suite().replacen(
            "[[suite]]\ngroup = \"TRIG\"\nheadings = []\nunits = [\"LC\"]\nrate = { min = 0.1, max = 0.2, basis = \"test fixture\" }\n",
            "",
            1,
        );
        let e = suite_errors(&no_trig);
        assert!(e.contains("suite[1]: TRIT needs a TRIG entry"), "{e}");
        let e = suite_errors(&suite_with(
            "group = \"TRIT\"\nheadings = []",
            "group = \"TRIT\"\nheadings = []\nrate = { min = 0.1, max = 0.2, basis = \"x\" }",
        ));
        assert!(
            e.contains("suite[2].rate: TRIT is scheduled with TRIG and has no rate of its own"),
            "{e}"
        );
    }

    #[test]
    fn a_wstd_entry_needs_wstg_and_the_water_groups_have_no_rate_or_units() {
        let wstg = "[[suite]]\ngroup = \"WSTG\"\nheadings = []\n";
        let wstd = "[[suite]]\ngroup = \"WSTD\"\nheadings = []\n";
        let e = suite_errors(&format!("{}\n{wstd}", with_suite()));
        assert!(e.contains("WSTD needs a WSTG entry"), "{e}");
        let ok = format!("{}\n{wstg}\n{wstd}", with_suite());
        assert!(Preset::from_toml(&ok).is_ok());
        let e = suite_errors(&format!(
            "{}\n{}\nunits = [\"RTD\"]\n\n{wstd}",
            with_suite(),
            wstg.trim_end()
        ));
        assert!(
            e.contains("WSTG follows the water table and has no units of its own"),
            "{e}"
        );
    }

    #[test]
    fn a_heading_no_producer_can_make_is_refused() {
        let e = suite_errors(&suite_with(
            "headings = []\nunits = [\"LC\"]\nrate = { min = 0.3",
            "headings = [\"LLPL_PREP\"]\nunits = [\"LC\"]\nrate = { min = 0.3",
        ));
        assert!(
            e.contains("suite[0].headings[0]: no producer makes LLPL_PREP"),
            "{e}"
        );
        let e = suite_errors(&suite_with(
            "headings = []\nunits = [\"LC\"]\nrate = { min = 0.3",
            "headings = [\"NOPE\"]\nunits = [\"LC\"]\nrate = { min = 0.3",
        ));
        assert!(
            e.contains("suite[0].headings[0]: \"NOPE\" is not a heading of LLPL"),
            "{e}"
        );
    }

    #[test]
    fn a_duplicate_group_or_missing_field_is_refused() {
        let dup = format!(
            "{}\n[[suite]]\ngroup = \"LLPL\"\nheadings = []\nunits = [\"LC\"]\nrate = {{ min = 0.1, max = 0.2, basis = \"x\" }}\n",
            with_suite()
        );
        assert!(suite_errors(&dup).contains("already has an entry"), "dup");
        let e = suite_errors(&suite_with(
            "units = [\"LC\"]\nrate = { min = 0.3",
            "rate = { min = 0.3",
        ));
        assert!(e.contains("suite[0].units: missing"), "{e}");
    }

    const STRENGTH: &str = "strength = { relation = \"cu.linear_with_depth\", cu_top_kpa = { min = 60, max = 120, basis = \"test fixture\" }, per_m = { min = 4, max = 8, basis = \"test fixture\" } }\n";
    const SPT: &str = "spt = { relation = \"spt.stroud_butler\", f1 = { min = 4, max = 6, basis = \"test fixture\" } }\n";

    /// The valid preset with `extra` lines added to the fine unit's properties.
    fn lc_with(extra: &str) -> String {
        with_props(RTD_PROPS, &format!("{LC_PROPS}{extra}"))
    }

    #[test]
    fn a_unit_may_carry_strength_and_spt_by_named_relation() {
        let p = Preset::from_toml(&lc_with(&format!("{STRENGTH}{SPT}"))).expect("valid");
        let props = &p.units[1].properties;
        let s = props.strength.as_ref().unwrap();
        assert_eq!(s.relation, StrengthRelation::LinearWithDepth);
        assert_eq!((s.cu_top_kpa.min, s.cu_top_kpa.max), (60.0, 120.0));
        assert_eq!((s.per_m.min, s.per_m.max), (4.0, 8.0));
        let t = props.spt.as_ref().unwrap();
        assert_eq!(t.relation, SptRelation::StroudButler);
        assert_eq!((t.f1.min, t.f1.max), (4.0, 6.0));
        let none = Preset::from_toml(&valid()).unwrap();
        assert!(
            none.units[1].properties.strength.is_none() && none.units[1].properties.spt.is_none()
        );
    }

    #[test]
    fn an_unknown_relation_is_refused_naming_the_field_and_the_known_names() {
        let e = errors_of(&lc_with(
            &STRENGTH.replace("cu.linear_with_depth", "cu.ocr"),
        ));
        assert_eq!(
            e,
            [
                "units[1] (LC).properties.strength.relation: \"cu.ocr\" is not a known relation (cu.linear_with_depth)"
            ]
        );
        let spt = SPT.replace("spt.stroud_butler", "spt.nope");
        let e = errors_of(&lc_with(&format!("{STRENGTH}{spt}")));
        assert_eq!(
            e,
            [
                "units[1] (LC).properties.spt.relation: \"spt.nope\" is not a known relation (spt.stroud_butler)"
            ]
        );
    }

    #[test]
    fn a_missing_relation_parameter_or_basis_is_refused_naming_the_field() {
        let per_m = ", per_m = { min = 4, max = 8, basis = \"test fixture\" }";
        assert_eq!(
            errors_of(&lc_with(&STRENGTH.replace(per_m, ""))),
            ["units[1] (LC).properties.strength.per_m: missing"]
        );
        let f1 = ", f1 = { min = 4, max = 6, basis = \"test fixture\" }";
        assert_eq!(
            errors_of(&lc_with(&format!("{STRENGTH}{}", SPT.replace(f1, "")))),
            ["units[1] (LC).properties.spt.f1: missing"]
        );
        let no_basis = STRENGTH.replace("max = 120, basis = \"test fixture\"", "max = 120");
        assert_eq!(
            errors_of(&lc_with(&no_basis)),
            ["units[1] (LC).properties.strength.cu_top_kpa: no basis"]
        );
        let no_relation = STRENGTH.replace("relation = \"cu.linear_with_depth\", ", "");
        assert_eq!(
            errors_of(&lc_with(&no_relation)),
            ["units[1] (LC).properties.strength.relation: missing"]
        );
    }

    #[test]
    fn strength_parameters_outside_their_domain_are_refused() {
        assert_eq!(
            errors_of(&lc_with(&STRENGTH.replace("min = 60", "min = 0"))),
            ["units[1] (LC).properties.strength.cu_top_kpa: min 0 is outside its domain (> 0)"]
        );
        let spt = SPT.replace("min = 4,", "min = -1,");
        assert_eq!(
            errors_of(&lc_with(&format!("{STRENGTH}{spt}"))),
            ["units[1] (LC).properties.spt.f1: min -1 is outside its domain (> 0)"]
        );
    }

    #[test]
    fn spt_without_strength_is_refused() {
        assert_eq!(
            errors_of(&lc_with(SPT)),
            [
                "units[1] (LC).properties.spt: spt.stroud_butler divides strength by f1, so the unit needs strength"
            ]
        );
    }

    #[test]
    fn a_suite_listing_a_trit_strength_heading_over_a_unit_without_strength_is_refused() {
        let text = |props: &str, headings: &str| {
            let suite = SUITE.replacen(
                "group = \"TRIT\"\nheadings = []",
                &format!("group = \"TRIT\"\nheadings = [{headings}]"),
                1,
            );
            format!("{}{suite}", lc_with(props))
        };
        // TRIG tests LC, which has no strength here.
        let e = suite_errors(&text("", "\"TRIT_CU\", \"TRIT_DEVF\""));
        assert!(
            e.contains("suite[2].headings[0]: TRIT_CU needs undrained strength, but unit \"LC\" (in suite[1], TRIG) has no strength"),
            "{e}"
        );
        assert!(e.contains("suite[2].headings[1]: TRIT_DEVF needs"), "{e}");
        // Headings that do not need strength are fine, and so is strength.
        Preset::from_toml(&text("", "\"TRIT_CELL\", \"TRIT_BDEN\", \"TRIT_IMC\"")).expect("valid");
        Preset::from_toml(&text(STRENGTH, "\"TRIT_CU\", \"TRIT_DEVF\"")).expect("valid");
    }

    // Multi-stage UU (#2): `triaxial` on a unit TRIG tests.

    const TRIAXIAL: &str = "triaxial = { multi_stage_share = { min = 0.4, max = 0.6, basis = \"test fixture\" }, stage_gain = { min = 0.1, max = 0.3, basis = \"test fixture\" } }\n";

    /// The fixture with `extra` on LC (which TRIG tests) and the suite.
    fn triaxial_text(extra: &str) -> String {
        format!("{}{SUITE}", lc_with(extra))
    }

    #[test]
    fn a_unit_may_carry_a_multi_stage_share_and_a_stage_gain() {
        let p = Preset::from_toml(&triaxial_text(TRIAXIAL)).expect("valid");
        let t = p.units[1].properties.triaxial.as_ref().unwrap();
        assert_eq!(
            (t.multi_stage_share.min, t.multi_stage_share.max),
            (0.4, 0.6)
        );
        assert_eq!(t.multi_stage_share.basis, "test fixture");
        let g = t.stage_gain.as_ref().unwrap();
        assert_eq!((g.min, g.max), (0.1, 0.3));
        // The gain is optional (0 when absent), and a unit without the table
        // has none.
        let share_only = "triaxial = { multi_stage_share = { min = 0.4, max = 0.6, basis = \"test fixture\" } }\n";
        let p = Preset::from_toml(&triaxial_text(share_only)).expect("valid");
        assert!(
            p.units[1]
                .properties
                .triaxial
                .as_ref()
                .unwrap()
                .stage_gain
                .is_none()
        );
        let p = Preset::from_toml(&triaxial_text("")).expect("valid");
        assert!(p.units[1].properties.triaxial.is_none());
    }

    #[test]
    fn a_stage_gain_without_a_share_is_refused() {
        let gain_only =
            "triaxial = { stage_gain = { min = 0.1, max = 0.3, basis = \"test fixture\" } }\n";
        assert_eq!(
            errors_of(&triaxial_text(gain_only)),
            [
                "units[1] (LC).properties.triaxial.stage_gain: stage_gain needs multi_stage_share, since no test is multi-stage without one"
            ]
        );
        assert_eq!(
            errors_of(&triaxial_text("triaxial = {}\n")),
            ["units[1] (LC).properties.triaxial.multi_stage_share: missing"]
        );
    }

    #[test]
    fn a_share_or_gain_outside_zero_to_one_or_without_a_basis_is_refused() {
        assert_eq!(
            errors_of(&triaxial_text(&TRIAXIAL.replace("max = 0.6", "max = 1.5"))),
            ["units[1] (LC).properties.triaxial.multi_stage_share: max 1.5 is outside 0 to 1"]
        );
        assert_eq!(
            errors_of(&triaxial_text(&TRIAXIAL.replace("min = 0.1", "min = -0.1"))),
            ["units[1] (LC).properties.triaxial.stage_gain: min -0.1 is outside 0 to 1"]
        );
        assert_eq!(
            errors_of(&triaxial_text(&TRIAXIAL.replace(
                "max = 0.3, basis = \"test fixture\"",
                "max = 1.2, basis = \"test fixture\""
            ))),
            ["units[1] (LC).properties.triaxial.stage_gain: max 1.2 is outside 0 to 1"]
        );
        let no_basis = TRIAXIAL.replacen(", basis = \"test fixture\"", "", 1);
        assert_eq!(
            errors_of(&triaxial_text(&no_basis)),
            ["units[1] (LC).properties.triaxial.multi_stage_share: no basis"]
        );
        let unknown = TRIAXIAL.replace("stage_gain", "stage_rise");
        assert_eq!(
            errors_of(&triaxial_text(&unknown)),
            ["units[1] (LC).properties.triaxial: unknown key \"stage_rise\""]
        );
    }

    #[test]
    fn a_unit_no_trig_entry_tests_may_not_carry_a_multi_stage_share() {
        // The suite's TRIG tests LC only.
        let text = format!(
            "{}{SUITE}",
            with_props(&format!("{RTD_PROPS}{TRIAXIAL}"), LC_PROPS)
        );
        let e = suite_errors(&text);
        assert!(
            e.contains("RTD.properties.triaxial: multi-stage tests are TRIG tests, but no TRIG entry in the suite tests unit \"RTD\""),
            "{e}"
        );
        // And with no suite at all there is no TRIG to be multi-stage in.
        let e = suite_errors(&lc_with(TRIAXIAL));
        assert!(e.contains("LC.properties.triaxial"), "{e}");
    }

    // Effective-stress triaxial (#68): `effective_strength` on a unit TREG
    // tests.

    const EFFECTIVE: &str = "effective_strength = { c_eff_kpa = { min = 3, max = 7, basis = \"test fixture\" }, phi_eff_deg = { min = 24, max = 30, basis = \"test fixture\" }, skempton_af = { min = 0.2, max = 0.5, basis = \"test fixture\" } }\n";

    const TREG_SUITE: &str = r#"
[[suite]]
group = "TREG"
headings = ["TREG_COH", "TREG_PHI"]
units = ["LC"]
rate = { min = 0.3, max = 0.5, basis = "test fixture" }

[[suite]]
group = "TRET"
headings = ["TRET_DEVF"]
"#;

    /// The fixture with `extra` on LC (which TREG tests) and the TREG suite.
    fn effective_text(extra: &str) -> String {
        format!("{}{TREG_SUITE}", lc_with(extra))
    }

    #[test]
    fn a_unit_may_carry_effective_strength_and_a_treg_entry_tests_it() {
        let p = Preset::from_toml(&effective_text(EFFECTIVE)).expect("valid");
        let e = p.units[1].properties.effective_strength.as_ref().unwrap();
        assert_eq!((e.c_eff_kpa.min, e.c_eff_kpa.max), (3.0, 7.0));
        assert_eq!((e.phi_eff_deg.min, e.phi_eff_deg.max), (24.0, 30.0));
        assert_eq!((e.skempton_af.min, e.skempton_af.max), (0.2, 0.5));
        assert_eq!(e.skempton_af.basis, "test fixture");
        assert!(p.units[0].properties.effective_strength.is_none());
        let groups: Vec<&str> = p.suite.iter().map(|e| e.group.as_str()).collect();
        assert_eq!(groups, ["TREG", "TRET"]);
        assert_eq!(p.suite[0].units, ["LC"]);
        assert!(p.suite[1].units.is_empty() && p.suite[1].rate.is_none());
    }

    #[test]
    fn effective_strength_outside_its_bounds_is_refused() {
        let at = "units[1] (LC).properties.effective_strength";
        for (from, to, want) in [
            (
                "min = 24, max = 30",
                "min = 24, max = 55",
                "phi_eff_deg: max 55 is outside 0 to 50",
            ),
            (
                "min = 24, max = 30",
                "min = -2, max = 30",
                "phi_eff_deg: min -2 is outside 0 to 50",
            ),
            (
                "min = 3, max = 7",
                "min = -1, max = 7",
                "c_eff_kpa: min -1 is outside 0 to 25",
            ),
            (
                "min = 3, max = 7",
                "min = 3, max = 30",
                "c_eff_kpa: max 30 is outside 0 to 25",
            ),
            (
                "min = 0.2, max = 0.5",
                "min = 0.2, max = 1.8",
                "skempton_af: max 1.8 is outside -0.5 to 1.5",
            ),
            (
                "min = 0.2, max = 0.5",
                "min = -0.9, max = 0.5",
                "skempton_af: min -0.9 is outside -0.5 to 1.5",
            ),
        ] {
            let text = EFFECTIVE.replacen(from, to, 1);
            assert_eq!(errors_of(&effective_text(&text)), [format!("{at}.{want}")]);
        }
    }

    #[test]
    fn effective_strength_needs_all_three_ranges_a_basis_and_no_unknown_key() {
        let at = "units[1] (LC).properties.effective_strength";
        let no_af = EFFECTIVE.replace(
            ", skempton_af = { min = 0.2, max = 0.5, basis = \"test fixture\" }",
            "",
        );
        assert_eq!(
            errors_of(&effective_text(&no_af)),
            [format!("{at}.skempton_af: missing")]
        );
        let no_basis = EFFECTIVE.replacen(", basis = \"test fixture\"", "", 1);
        assert_eq!(
            errors_of(&effective_text(&no_basis)),
            [format!("{at}.c_eff_kpa: no basis")]
        );
        let unknown = EFFECTIVE.replace("skempton_af", "skempton_a");
        let e = errors_of(&effective_text(&unknown));
        assert!(e.contains(&format!("{at}.skempton_af: missing")), "{e:?}");
        assert!(
            e.contains(&format!("{at}: unknown key \"skempton_a\"")),
            "{e:?}"
        );
    }

    /// Failure needs sin(phi') x (1 - 2 `A_f`) < 1 at the extremes the ranges
    /// allow, or the deviator at failure has no solution.
    #[test]
    fn a_range_pair_with_no_failure_deviator_is_refused() {
        // sin(50) = 0.766, and A_f = -0.5 gives 0.766 x 2 = 1.53.
        let text = EFFECTIVE
            .replacen("min = 24, max = 30", "min = 24, max = 50", 1)
            .replacen("min = 0.2, max = 0.5", "min = -0.5, max = 0.5", 1);
        let e = errors_of(&effective_text(&text));
        assert_eq!(e.len(), 1, "{e:?}");
        assert!(
            e[0].starts_with("units[1] (LC).properties.effective_strength.skempton_af: min -0.5 with phi_eff_deg max 50 leaves no deviator stress"),
            "{e:?}"
        );
    }

    #[test]
    fn treg_needs_effective_strength_on_every_unit_it_tests_and_tret_needs_treg() {
        // TREG over LC with nothing on LC.
        let e = errors_of(&effective_text(""));
        assert_eq!(
            e,
            [
                "suite[0].units[0]: TREG needs effective_strength (c', phi' and A_f) to build its sets and stages, but unit \"LC\" has none"
            ]
        );
        // effective_strength on a unit no TREG entry tests, and no suite at all.
        let e = errors_of(&lc_with(EFFECTIVE));
        assert_eq!(
            e,
            [
                "LC.properties.effective_strength: effective-stress triaxial results are TREG tests, but no TREG entry in the suite tests unit \"LC\""
            ]
        );
        // TRET without its TREG, and with a rate of its own.
        let no_treg = format!(
            "{}{}",
            lc_with(EFFECTIVE),
            TREG_SUITE.replace("group = \"TREG\"", "group = \"TRIG\"")
        );
        let e = errors_of(&no_treg).join("\n");
        assert!(e.contains("suite[1]: TRET needs a TREG entry"), "{e}");
        let rate = TREG_SUITE.replace(
            "headings = [\"TRET_DEVF\"]",
            "headings = [\"TRET_DEVF\"]\nrate = { min = 0.1, max = 0.2, basis = \"x\" }",
        );
        let e = errors_of(&format!("{}{rate}", lc_with(EFFECTIVE))).join("\n");
        assert!(
            e.contains("suite[1].rate: TRET is scheduled with TREG and has no rate of its own"),
            "{e}"
        );
    }

    #[test]
    fn a_made_ground_unit_may_not_be_tested_for_treg() {
        let text = format!(
            "{}{}",
            with_made_ground_effective(),
            TREG_SUITE.replace("units = [\"LC\"]", "units = [\"MG\"]")
        );
        let e = errors_of(&text).join("\n");
        assert!(
            e.contains("TREG may not test made-ground unit \"MG\" (no lab tests in Made Ground)"),
            "{e}"
        );
    }

    /// The fixture with a made-ground unit MG (given effective strength so only
    /// the made-ground rule is in play) above LC.
    fn with_made_ground_effective() -> String {
        let mg = format!(
            "[[units]]\nid = \"MG\"\nname = \"Made Ground\"\nthickness_m = {{ min = 1.0, max = 2.0, basis = \"test fixture\" }}\ndescription = {{ principals = [\"SAND\", \"GRAVEL\"], secondaries = [\"clay\"], colours = [\"brown\"] }}\nmade_ground = true\nsampling = {{ types = [\"B\"], interval_m = {{ min = 1.0, max = 1.5, basis = \"test fixture\" }} }}\n\n[units.properties]\nspt_n = {{ min = 4, max = 30, basis = \"test fixture\" }}\nmoisture_content_percent = {{ min = 8, max = 15, basis = \"test fixture\" }}\nparticle_density_ratio = {{ min = 2.6, max = 2.7, basis = \"test fixture\" }}\n{EFFECTIVE}\n"
        );
        lc_with("").replacen(
            "[[units]]\nid = \"RTD\"",
            &format!("{mg}[[units]]\nid = \"RTD\""),
            1,
        )
    }

    #[test]
    fn a_suite_listing_an_llpl_heading_over_a_unit_without_fine_properties_is_refused() {
        let text = |headings: &str, unit: &str| {
            let suite = SUITE.replacen(
                "group = \"LLPL\"\nheadings = []\nunits = [\"LC\"]",
                &format!("group = \"LLPL\"\nheadings = [{headings}]\nunits = [\"{unit}\"]"),
                1,
            );
            format!("{}{suite}", lc_with(""))
        };
        // RTD is a coarse unit with no fine properties.
        let e = suite_errors(&text("\"LLPL_LL\", \"LLPL_PL\", \"LLPL_PI\"", "RTD"));
        for (j, h) in ["LLPL_LL", "LLPL_PL", "LLPL_PI"].iter().enumerate() {
            let want = format!(
                "suite[0].headings[{j}]: {h} needs fine properties (liquid and plastic limits), but unit \"RTD\" has none"
            );
            assert!(e.contains(&want), "{e}");
        }
        // No headings, or a fine unit, is fine.
        Preset::from_toml(&text("", "RTD")).expect("valid");
        Preset::from_toml(&text("\"LLPL_LL\", \"LLPL_PL\", \"LLPL_PI\"", "LC")).expect("valid");
    }

    #[test]
    fn a_suite_listing_ispt_nval_over_a_unit_without_a_source_of_n_is_refused() {
        let text = |props: &str, unit: &str| {
            let suite = SUITE.replace(
                "group = \"ISPT\"\nheadings = []\nunits = [\"RTD\"]",
                &format!("group = \"ISPT\"\nheadings = [\"ISPT_NVAL\"]\nunits = [\"{unit}\"]"),
            );
            format!("{}{suite}", lc_with(props))
        };
        for props in ["", STRENGTH] {
            let e = suite_errors(&text(props, "LC"));
            assert!(
                e.contains("suite[3].headings[0]: ISPT_NVAL needs strength and spt (a fine unit), but unit \"LC\" lacks it"),
                "{e}"
            );
        }
        Preset::from_toml(&text(&format!("{STRENGTH}{SPT}"), "LC")).expect("valid");
        // A coarse unit has its own N.
        Preset::from_toml(&text("", "RTD")).expect("valid");
    }

    /// The suite with ISPT listing `headings` over RTD.
    fn ispt_suite(headings: &str) -> String {
        let suite = SUITE.replace(
            "group = \"ISPT\"\nheadings = []",
            &format!("group = \"ISPT\"\nheadings = [{headings}]"),
        );
        format!("{}{suite}", valid())
    }

    #[test]
    fn ispt_main_npen_and_rep_are_refused_without_nval_naming_the_field() {
        for h in ["ISPT_MAIN", "ISPT_NPEN", "ISPT_REP"] {
            let e = suite_errors(&ispt_suite(&format!("\"{h}\"")));
            assert!(
                e.contains(&format!(
                    "suite[3].headings[0]: {h} describes the test that ISPT_NVAL counts, so ISPT_NVAL must be listed too"
                )),
                "{e}"
            );
            Preset::from_toml(&ispt_suite(&format!("\"ISPT_NVAL\", \"{h}\""))).expect("valid");
        }
        Preset::from_toml(&ispt_suite(
            "\"ISPT_NVAL\", \"ISPT_MAIN\", \"ISPT_NPEN\", \"ISPT_REP\"",
        ))
        .expect("valid");
    }

    const REFUSAL: &str =
        "spt = { refusal_share = { min = 0.1, max = 0.3, basis = \"test fixture\" } }\n";

    /// The fixture with `extra` added to RTD's properties.
    fn rtd_with(extra: &str) -> String {
        with_props(&format!("{RTD_PROPS}{extra}"), LC_PROPS)
    }

    #[test]
    fn a_coarse_unit_takes_a_refusal_share_as_a_range_or_a_plain_share() {
        let p = Preset::from_toml(&rtd_with(REFUSAL)).expect("valid");
        let Some(RefusalShare::Share(r)) = p.units[0].properties.spt_refusal_share.as_ref() else {
            panic!("a plain range");
        };
        assert_eq!((r.min, r.max), (0.1, 0.3));
        assert!(p.units[0].properties.spt.is_none(), "no relation was given");
        let plain = "spt = { refusal_share = { share = 0.2, basis = \"test fixture\" } }\n";
        let p = Preset::from_toml(&rtd_with(plain)).expect("valid");
        let Some(RefusalShare::Share(r)) = p.units[0].properties.spt_refusal_share.as_ref() else {
            panic!("a plain range");
        };
        assert_eq!((r.min, r.max), (0.2, 0.2));
        assert_eq!(r.basis, "test fixture");
        let none = Preset::from_toml(&valid()).unwrap();
        assert!(none.units[0].properties.spt_refusal_share.is_none());
    }

    const LINEAR: &str = "spt = { refusal_share = { top = { min = -0.1, max = 0.05, basis = \"test fixture\" }, per_m = { min = 0.01, max = 0.03, basis = \"test fixture\" } } }\n";

    #[test]
    fn a_refusal_share_may_rise_linearly_with_depth() {
        let p = Preset::from_toml(&rtd_with(LINEAR)).expect("valid");
        let Some(RefusalShare::Linear { top, per_m }) =
            p.units[0].properties.spt_refusal_share.as_ref()
        else {
            panic!("the linear form");
        };
        assert_eq!(
            (top.min, top.max, per_m.min, per_m.max),
            (-0.1, 0.05, 0.01, 0.03)
        );
    }

    #[test]
    fn a_bad_linear_refusal_share_is_refused_naming_the_field() {
        let e = errors_of(&rtd_with(&LINEAR.replace("max = 0.03", "max = 2")));
        assert_eq!(
            e,
            ["units[0] (RTD).properties.spt.refusal_share.per_m: max 2 is outside -1 to 1"]
        );
        let e = errors_of(&rtd_with(&LINEAR.replace(
            ", per_m = { min = 0.01, max = 0.03, basis = \"test fixture\" }",
            "",
        )));
        assert_eq!(
            e,
            ["units[0] (RTD).properties.spt.refusal_share.per_m: missing"]
        );
        let e = errors_of(&rtd_with(&LINEAR.replace(
            "min = -0.1, max = 0.05, basis = \"test fixture\"",
            "min = -0.1, max = 0.05",
        )));
        assert_eq!(
            e,
            ["units[0] (RTD).properties.spt.refusal_share.top: no basis"]
        );
        let e = errors_of(&rtd_with(&LINEAR.replace("top = ", "share = 0.2, top = ")));
        assert_eq!(
            e,
            ["units[0] (RTD).properties.spt.refusal_share: unknown key \"share\""]
        );
    }

    #[test]
    fn a_fine_unit_takes_a_refusal_share_only_with_very_coarse() {
        let e = errors_of(&lc_with(REFUSAL));
        assert_eq!(e.len(), 1, "{e:?}");
        assert!(
            e[0].starts_with(
                "units[1] (LC).properties.spt.refusal_share: the unit is neither coarse"
            ),
            "{e:?}"
        );
        let coarse_clay = valid().replacen(
            r#"description = { principals = ["CLAY"], secondaries = [], colours = ["grey"] }"#,
            r#"description = { principals = ["CLAY"], secondaries = [], colours = ["grey"], very_coarse = { cobbles = "low" } }"#,
            1,
        );
        let text = format!("{coarse_clay}{STRENGTH}{SPT}").replace(
            "spt = { relation = \"spt.stroud_butler\", f1 = { min = 4, max = 6, basis = \"test fixture\" } }",
            "spt = { relation = \"spt.stroud_butler\", f1 = { min = 4, max = 6, basis = \"test fixture\" }, refusal_share = { min = 0.1, max = 0.3, basis = \"test fixture\" } }",
        );
        let p = Preset::from_toml(&text).expect("valid");
        let props = &p.units[1].properties;
        assert!(props.spt.is_some() && props.spt_refusal_share.is_some());
    }

    #[test]
    fn a_bad_refusal_share_is_refused_naming_the_field() {
        let out_of_range = REFUSAL.replace("max = 0.3", "max = 1.5");
        assert_eq!(
            errors_of(&rtd_with(&out_of_range)),
            ["units[0] (RTD).properties.spt.refusal_share: max 1.5 is outside 0 to 1"]
        );
        let no_basis = REFUSAL.replace(", basis = \"test fixture\"", "");
        assert_eq!(
            errors_of(&rtd_with(&no_basis)),
            ["units[0] (RTD).properties.spt.refusal_share: no basis"]
        );
        // An spt table with neither a relation nor a share is still refused.
        assert_eq!(
            errors_of(&rtd_with("spt = {}\n")),
            [
                "units[0] (RTD).properties.spt.relation: missing",
                "units[0] (RTD).properties.spt.f1: missing"
            ]
        );
    }

    const RTD_DESC: &str = r#"description = { principals = ["SAND", "GRAVEL"], secondaries = ["gravel"], colours = ["brown"] }"#;

    /// `text` with RTD made ground, `spec` being its `made_ground` value.
    fn rtd_made_ground(text: &str, spec: &str) -> String {
        assert!(text.contains(RTD_DESC), "fixture lacks the RTD description");
        text.replacen(RTD_DESC, &format!("{RTD_DESC}\nmade_ground = {spec}"), 1)
    }

    fn mg_errors(spec: &str) -> String {
        errors_of(&rtd_made_ground(&valid(), spec)).join("\n")
    }

    const BRICK: &str = r#"{ name = "brick", share = 0.5, basis = "test fixture" }"#;
    const MG_COUNT: &str = r#"{ weights = [1, 2, 2, 1], basis = "test fixture" }"#;

    #[test]
    fn a_unit_without_made_ground_is_plain() {
        let p = Preset::from_toml(&valid()).unwrap();
        assert!(p.units.iter().all(|u| u.made_ground.is_none()));
        assert!(!p.is_made_ground("RTD"));
    }

    #[test]
    fn made_ground_true_loads_with_no_weights() {
        let p = Preset::from_toml(&rtd_made_ground(&valid(), "true")).expect("valid");
        assert_eq!(p.units[0].made_ground, Some(MadeGroundSpec::default()));
        assert!(p.is_made_ground("RTD") && !p.is_made_ground("LC"));
    }

    #[test]
    fn made_ground_weights_load_into_typed_values() {
        let spec = format!(
            r#"{{ constituents = [{BRICK}, {{ name = "coal", share = {{ min = 0.1, max = 0.2, basis = "test fixture" }} }}], count = {MG_COUNT} }}"#
        );
        let p = Preset::from_toml(&rtd_made_ground(&valid(), &spec)).expect("valid");
        let m = p.units[0].made_ground.as_ref().unwrap();
        let c = m.constituents.as_ref().unwrap();
        assert_eq!(c[0].name, "brick");
        assert_eq!((c[0].share.min, c[0].share.max), (0.5, 0.5));
        assert_eq!(c[0].share.basis, "test fixture");
        // The extended vocabulary is nameable.
        assert_eq!(c[1].name, "coal");
        assert_eq!((c[1].share.min, c[1].share.max), (0.1, 0.2));
        assert_eq!(m.count.as_ref().unwrap().weights, [1.0, 2.0, 2.0, 1.0]);
        assert_eq!(m.count.as_ref().unwrap().basis, "test fixture");
        // Either part alone is enough.
        for alone in [
            format!("{{ constituents = [{BRICK}] }}"),
            format!("{{ count = {MG_COUNT} }}"),
        ] {
            Preset::from_toml(&rtd_made_ground(&valid(), &alone)).expect("valid");
        }
    }

    #[test]
    fn an_unknown_constituent_is_refused_naming_the_field_and_the_known_words() {
        let e = mg_errors(
            r#"{ constituents = [{ name = "gold", share = 0.5, basis = "test fixture" }] }"#,
        );
        assert!(
            e.starts_with(
                "units[0] (RTD).made_ground.constituents[0].name: unknown constituent \"gold\" (known: brick,"
            ),
            "{e}"
        );
        assert!(e.contains("pottery") && e.contains("coal"), "{e}");
    }

    #[test]
    fn a_duplicate_constituent_is_refused() {
        let e = mg_errors(&format!("{{ constituents = [{BRICK}, {BRICK}] }}"));
        assert_eq!(
            e,
            "units[0] (RTD).made_ground.constituents[1].name: \"brick\" is listed twice"
        );
    }

    #[test]
    fn shares_outside_zero_to_one_are_refused_naming_the_field() {
        let e = mg_errors(
            r#"{ constituents = [{ name = "brick", share = 1.2, basis = "test fixture" }] }"#,
        );
        assert_eq!(
            e,
            "units[0] (RTD).made_ground.constituents[0].share: min 1.2 is outside 0 to 1\n\
             units[0] (RTD).made_ground.constituents[0].share: max 1.2 is outside 0 to 1"
        );
        let e = mg_errors(
            r#"{ constituents = [{ name = "brick", share = { min = -0.1, max = 0.5, basis = "test fixture" } }] }"#,
        );
        assert_eq!(
            e,
            "units[0] (RTD).made_ground.constituents[0].share: min -0.1 is outside 0 to 1"
        );
    }

    #[test]
    fn a_share_without_a_basis_is_refused_naming_the_field() {
        let e = mg_errors(r#"{ constituents = [{ name = "brick", share = 0.5 }] }"#);
        assert_eq!(e, "units[0] (RTD).made_ground.constituents[0]: no basis");
        let e = mg_errors(
            r#"{ constituents = [{ name = "brick", share = { min = 0.4, max = 0.5 } }] }"#,
        );
        assert_eq!(
            e,
            "units[0] (RTD).made_ground.constituents[0].share: no basis"
        );
    }

    #[test]
    fn count_weights_that_are_missing_short_negative_or_zero_are_refused() {
        let count = |inner: &str| mg_errors(&format!("{{ count = {{ {inner} }} }}"));
        assert_eq!(
            count(r"weights = [1, 2, 2, 1]"),
            "units[0] (RTD).made_ground.count: no basis"
        );
        assert_eq!(
            count(r#"basis = "test fixture""#),
            "units[0] (RTD).made_ground.count.weights: missing"
        );
        assert_eq!(
            count(r#"weights = [1, 2, 2], basis = "test fixture""#),
            "units[0] (RTD).made_ground.count.weights: needs 4 weights (0, 1, 2 and 3+ constituents), found 3"
        );
        assert_eq!(
            count(r#"weights = [1, -2, 2, 1], basis = "test fixture""#),
            "units[0] (RTD).made_ground.count.weights[1]: -2 is negative"
        );
        assert_eq!(
            count(r#"weights = [0, 0, 0, 0], basis = "test fixture""#),
            "units[0] (RTD).made_ground.count.weights: weights sum to zero"
        );
        assert_eq!(
            count(r#"weights = [1, "x", 2, 1], basis = "test fixture""#),
            "units[0] (RTD).made_ground.count.weights[1]: must be a number"
        );
    }

    #[test]
    fn amount_weights_load_and_are_refused_like_count_weights() {
        let amounts = |inner: &str| mg_errors(&format!("{{ amounts = {{ {inner} }} }}"));
        let p = Preset::from_toml(&rtd_made_ground(
            &valid(),
            r#"{ amounts = { weights = [0.14, 0.25, 0.07, 0.0, 0.02], basis = "test fixture" } }"#,
        ))
        .expect("valid");
        let a = p.units[0].made_ground.as_ref().unwrap().amounts.as_ref();
        assert_eq!(a.unwrap().weights, [0.14, 0.25, 0.07, 0.0, 0.02]);
        assert_eq!(a.unwrap().basis, "test fixture");
        assert_eq!(
            amounts(r"weights = [1, 2, 2, 1, 1]"),
            "units[0] (RTD).made_ground.amounts: no basis"
        );
        assert_eq!(
            amounts(r#"basis = "test fixture""#),
            "units[0] (RTD).made_ground.amounts.weights: missing"
        );
        assert_eq!(
            amounts(r#"weights = [1, 2, 2, 1], basis = "test fixture""#),
            "units[0] (RTD).made_ground.amounts.weights: needs 5 weights (rare, occasional, frequent, numerous and abundant), found 4"
        );
        assert_eq!(
            amounts(r#"weights = [1, 2, -2, 1, 1], basis = "test fixture""#),
            "units[0] (RTD).made_ground.amounts.weights[2]: -2 is negative"
        );
        assert_eq!(
            amounts(r#"weights = [0, 0, 0, 0, 0], basis = "test fixture""#),
            "units[0] (RTD).made_ground.amounts.weights: weights sum to zero"
        );
    }

    #[test]
    fn an_empty_or_malformed_made_ground_is_refused() {
        assert_eq!(
            mg_errors("{}"),
            "units[0] (RTD).made_ground: empty (write `true` for the default draw)"
        );
        assert_eq!(
            mg_errors("false"),
            "units[0] (RTD).made_ground: must be true or a table"
        );
        assert_eq!(
            mg_errors("{ constituents = [] }"),
            "units[0] (RTD).made_ground.constituents: must not be empty"
        );
        assert_eq!(
            mg_errors(&format!("{{ colour = 1, count = {MG_COUNT} }}")),
            "units[0] (RTD).made_ground: unknown key \"colour\""
        );
    }

    #[test]
    fn water_bearing_loads_on_a_unit_and_a_lens_and_defaults_to_false() {
        let p = Preset::from_toml(&valid()).unwrap();
        assert!(p.units.iter().all(|u| u.water_bearing.is_none()));
        let on = rtd_made_ground(&valid(), "false")
            .replace("made_ground = false", "water_bearing = true");
        let p = Preset::from_toml(&on).expect("valid");
        assert!(p.units[0].water_bearing.is_some() && p.is_water_bearing("RTD"));
        assert!(!p.is_water_bearing("LC"));
        let lens = lens_with("host = \"LC\"", "host = \"LC\"\nwater_bearing = true");
        let p = Preset::from_toml(&lens).expect("valid");
        assert!(p.lenses[0].water_bearing.is_some() && p.is_water_bearing(&p.lenses[0].id));
    }

    #[test]
    fn water_bearing_is_refused_on_a_made_ground_unit_naming_the_field() {
        let text = rtd_made_ground(&valid(), "true").replace(
            "made_ground = true",
            "made_ground = true\nwater_bearing = true",
        );
        let e = errors_of(&text).join("\n");
        assert!(
            e.contains("units[0] (RTD).water_bearing: not allowed on a made-ground unit"),
            "{e}"
        );
    }

    #[test]
    fn water_bearing_must_be_a_boolean() {
        let text = rtd_made_ground(&valid(), "false")
            .replace("made_ground = false", "water_bearing = \"yes\"");
        assert_eq!(
            errors_of(&text).join("\n"),
            "units[0] (RTD).water_bearing: must be true, false or a table with a share"
        );
        let lens = lens_with("host = \"LC\"", "host = \"LC\"\nwater_bearing = 1");
        let e = errors_of(&lens).join("\n");
        assert!(
            e.contains(".water_bearing: must be true, false or a table with a share"),
            "{e}"
        );
    }

    fn wb_unit(wb: &str) -> String {
        rtd_made_ground(&valid(), "false")
            .replace("made_ground = false", &format!("water_bearing = {wb}"))
    }

    #[test]
    fn water_bearing_takes_a_share_range_or_a_plain_share_with_a_basis() {
        let range = |text: &str| {
            Preset::from_toml(text).unwrap().units[0]
                .water_bearing
                .clone()
        };
        let t = range(&wb_unit("true")).expect("true is a share");
        assert_eq!((t.min, t.max), (1.0, 1.0));
        assert!(range(&wb_unit("false")).is_none());
        let r = range(&wb_unit(
            r#"{ share = { min = 0.3, max = 0.4, basis = "test fixture" } }"#,
        ))
        .unwrap();
        assert_eq!((r.min, r.max, r.basis.as_str()), (0.3, 0.4, "test fixture"));
        let r = range(&wb_unit(r#"{ share = 0.36, basis = "test fixture" }"#)).unwrap();
        assert_eq!((r.min, r.max), (0.36, 0.36));
        let lens = lens_with(
            "host = \"LC\"",
            "host = \"LC\"\nwater_bearing = { share = 0.5, basis = \"test fixture\" }",
        );
        assert!(
            Preset::from_toml(&lens).unwrap().lenses[0]
                .water_bearing
                .is_some()
        );
    }

    #[test]
    fn a_water_bearing_share_is_refused_when_out_of_range_or_without_a_basis() {
        let e = errors_of(&wb_unit(r#"{ share = 1.2, basis = "test fixture" }"#)).join("\n");
        assert!(
            e.contains("units[0] (RTD).water_bearing.share: min 1.2 is outside 0 to 1"),
            "{e}"
        );
        let e = errors_of(&wb_unit(
            r#"{ share = { min = -0.1, max = 0.5, basis = "test fixture" } }"#,
        ))
        .join("\n");
        assert!(
            e.contains("water_bearing.share: min -0.1 is outside 0 to 1"),
            "{e}"
        );
        let e = errors_of(&wb_unit("{ share = 0.5 }")).join("\n");
        assert!(e.contains("units[0] (RTD).water_bearing: no basis"), "{e}");
        let e = errors_of(&wb_unit("{ share = { min = 0.4, max = 0.5 } }")).join("\n");
        assert!(
            e.contains("units[0] (RTD).water_bearing.share: no basis"),
            "{e}"
        );
        let e = errors_of(&wb_unit("{}")).join("\n");
        assert!(
            e.contains("units[0] (RTD).water_bearing.share: missing"),
            "{e}"
        );
    }

    #[test]
    fn a_water_bearing_table_is_refused_on_a_made_ground_unit_naming_the_field() {
        let text = rtd_made_ground(&valid(), "true").replace(
            "made_ground = true",
            "made_ground = true\nwater_bearing = { share = 0.5, basis = \"test fixture\" }",
        );
        let e = errors_of(&text).join("\n");
        assert!(
            e.contains("units[0] (RTD).water_bearing: not allowed on a made-ground unit"),
            "{e}"
        );
    }

    #[test]
    fn made_ground_belongs_to_units_not_lenses() {
        let e = errors_of(&lens_with(
            "host = \"LC\"",
            "host = \"LC\"\nmade_ground = true",
        ))
        .join("\n");
        assert!(e.contains("unknown key \"made_ground\""), "{e}");
    }

    #[test]
    fn a_suite_listing_a_lab_group_over_a_made_ground_unit_is_refused_naming_entry_and_unit() {
        let mg = |text: &str| rtd_made_ground(text, "true");
        // LLPL
        let e = suite_errors(&mg(&suite_with(
            "units = [\"LC\"]\nrate = { min = 0.3",
            "units = [\"RTD\"]\nrate = { min = 0.3",
        )));
        assert_eq!(
            e,
            "suite[0].units[0]: LLPL may not test made-ground unit \"RTD\" (no lab tests in Made Ground)"
        );
        // TRIG
        let e = suite_errors(&mg(&suite_with(
            "units = [\"LC\"]\nrate = { min = 0.1",
            "units = [\"LC\", \"RTD\"]\nrate = { min = 0.1",
        )));
        assert_eq!(
            e,
            "suite[1].units[1]: TRIG may not test made-ground unit \"RTD\" (no lab tests in Made Ground)"
        );
    }

    #[test]
    fn a_suite_may_still_list_ispt_over_a_made_ground_unit() {
        // The suite fixture's ISPT runs over RTD.
        let p = Preset::from_toml(&rtd_made_ground(&with_suite(), "true")).expect("valid");
        let ispt = p.suite.iter().find(|e| e.group == "ISPT").unwrap();
        assert_eq!(ispt.units, ["RTD"]);
    }

    /// `RTD_DESC` with `very_coarse` written as `spec`.
    fn rtd_very_coarse(spec: &str) -> String {
        let desc = RTD_DESC.replace(" }", &format!(", very_coarse = {spec} }}"));
        with(RTD_DESC, &desc)
    }

    #[test]
    fn a_description_without_very_coarse_is_plain() {
        let p = Preset::from_toml(&valid()).expect("valid");
        assert!(p.units.iter().all(|u| u.description.very_coarse.is_none()));
    }

    #[test]
    fn very_coarse_loads_either_side_or_both() {
        let load = |spec: &str| {
            Preset::from_toml(&rtd_very_coarse(spec))
                .expect("valid")
                .units[0]
                .description
                .very_coarse
        };
        assert_eq!(
            load(r#"{ cobbles = "low", boulders = "high" }"#),
            Some(VeryCoarse {
                cobbles: Some(Content::Low),
                boulders: Some(Content::High)
            })
        );
        assert_eq!(
            load(r#"{ boulders = "medium" }"#),
            Some(VeryCoarse {
                cobbles: None,
                boulders: Some(Content::Medium)
            })
        );
        assert_eq!(
            load(r#"{ cobbles = "low" }"#),
            Some(VeryCoarse {
                cobbles: Some(Content::Low),
                boulders: None
            })
        );
    }

    #[test]
    fn very_coarse_refuses_an_unknown_level_naming_the_field() {
        assert_refused(
            &rtd_very_coarse(r#"{ cobbles = "some" }"#),
            "units[0] (RTD).description.very_coarse.cobbles: unknown level \"some\" (known: low, medium, high)",
        );
        assert_refused(
            &rtd_very_coarse(r#"{ boulders = "Low" }"#),
            "units[0] (RTD).description.very_coarse.boulders: unknown level \"Low\" (known: low, medium, high)",
        );
        assert_refused(
            &rtd_very_coarse(r"{ boulders = 3 }"),
            "units[0] (RTD).description.very_coarse.boulders: must be a string",
        );
    }

    #[test]
    fn very_coarse_refuses_an_unknown_key_naming_the_field() {
        assert_refused(
            &rtd_very_coarse(r#"{ cobbles = "low", gravel = "low" }"#),
            "units[0] (RTD).description.very_coarse: unknown key \"gravel\"",
        );
    }

    #[test]
    fn very_coarse_refuses_naming_neither_fraction_and_a_non_table() {
        assert_refused(
            &rtd_very_coarse("{}"),
            "units[0] (RTD).description.very_coarse: names neither cobbles nor boulders",
        );
        assert_refused(
            &rtd_very_coarse(r#""low""#),
            "units[0] (RTD).description.very_coarse: must be a table",
        );
    }
    const STRIKE: &str = "strike = { \
        rate_per_m = { min = 0.02, max = 0.04, basis = \"test fixture\" }, \
        rise_m = { min = 0.1, max = 5.0, skew = 2.0, basis = \"test fixture\" }, \
        no_rise_share = { min = 0.1, max = 0.2, basis = \"test fixture\" } }";

    /// The fixture with `strike` written on the London Clay unit (the last).
    fn strike_with(spec: &str) -> String {
        valid().replacen(
            "name = \"London Clay\"",
            &format!("name = \"London Clay\"\n{spec}"),
            1,
        )
    }

    #[test]
    fn a_unit_strike_loads_with_its_three_cited_ranges() {
        let p = Preset::from_toml(&valid()).unwrap();
        assert!(p.units.iter().all(|u| u.strike.is_none()));
        let p = Preset::from_toml(&strike_with(STRIKE)).expect("valid");
        assert!(p.units[0].strike.is_none());
        let s = p.units[1].strike.as_ref().expect("LC strikes");
        assert_eq!(
            (s.rate.ranges()[0].min, s.rate.ranges()[0].max),
            (0.02, 0.04)
        );
        assert_eq!(s.rise_m.skew, Some(2.0));
        assert_eq!((s.no_rise_share.min, s.no_rise_share.max), (0.1, 0.2));
        assert_eq!(s.rate.ranges()[0].basis, "test fixture");
    }

    #[test]
    fn a_unit_strike_is_refused_without_a_basis_or_a_part() {
        assert_refused(
            &strike_with(&STRIKE.replace(
                "rate_per_m = { min = 0.02, max = 0.04, basis = \"test fixture\" }",
                "rate_per_m = { min = 0.02, max = 0.04 }",
            )),
            "units[1] (LC).strike.rate_per_m: no basis",
        );
        assert_refused(
            &strike_with(&STRIKE.replace(
                "no_rise_share = { min = 0.1, max = 0.2, basis = \"test fixture\" }",
                "no_rise_share = { min = 0.1, max = 0.2, basis = \"\" }",
            )),
            "units[1] (LC).strike.no_rise_share: empty basis",
        );
        assert_refused(
            &strike_with(&STRIKE.replace(
                "rise_m = { min = 0.1, max = 5.0, skew = 2.0, basis = \"test fixture\" }, ",
                "",
            )),
            "units[1] (LC).strike.rise_m: missing",
        );
    }

    #[test]
    fn a_unit_strike_rate_and_rise_may_not_be_negative_or_not_finite() {
        assert_refused(
            &strike_with(&STRIKE.replace("min = 0.02, max = 0.04", "min = -0.01, max = 0.04")),
            "units[1] (LC).strike.rate_per_m: min -0.01 is outside its domain (>= 0)",
        );
        assert_refused(
            &strike_with(&STRIKE.replace("min = 0.1, max = 5.0", "min = -0.1, max = 5.0")),
            "units[1] (LC).strike.rise_m: min -0.1 is outside its domain (>= 0)",
        );
        assert_refused(
            &strike_with(&STRIKE.replace("min = 0.02, max = 0.04", "min = 0.02, max = inf")),
            "units[1] (LC).strike.rate_per_m.max: must be finite",
        );
        assert_refused(
            &strike_with(&STRIKE.replace("min = 0.02, max = 0.04", "min = 0.04, max = 0.02")),
            "units[1] (LC).strike.rate_per_m: min 0.04 is greater than max 0.02",
        );
    }

    #[test]
    fn a_unit_strike_rise_skew_is_at_least_one_and_the_no_rise_share_is_a_fraction() {
        assert_refused(
            &strike_with(&STRIKE.replace("skew = 2.0", "skew = 0.5")),
            "units[1] (LC).strike.rise_m.skew: 0.5 must be >= 1",
        );
        assert_refused(
            &strike_with(&STRIKE.replace("min = 0.1, max = 0.2", "min = 0.1, max = 1.5")),
            "units[1] (LC).strike.no_rise_share: max 1.5 is outside 0 to 1",
        );
        assert_refused(
            &strike_with(&STRIKE.replace("min = 0.1, max = 0.2", "min = -0.1, max = 0.2")),
            "units[1] (LC).strike.no_rise_share: min -0.1 is outside 0 to 1",
        );
    }

    #[test]
    fn a_unit_strike_must_be_a_table_with_known_keys() {
        assert_refused(
            &strike_with("strike = 0.03"),
            "units[1] (LC).strike: must be a table",
        );
        assert_eq!(
            errors_of(&strike_with(
                &STRIKE.replace("no_rise_share", "no_rise_shar")
            )),
            [
                "units[1] (LC).strike.no_rise_share: missing",
                "units[1] (LC).strike: unknown key \"no_rise_shar\""
            ]
        );
    }

    #[test]
    fn a_unit_strike_is_allowed_on_a_made_ground_unit_but_refused_on_a_water_bearing_one() {
        let text = rtd_made_ground(&valid(), "true").replace(
            "made_ground = true",
            &format!("made_ground = true\n{STRIKE}"),
        );
        let p = Preset::from_toml(&text).expect("made ground may strike");
        assert!(p.units[0].strike.is_some());
        let text = rtd_made_ground(&valid(), "false").replace(
            "made_ground = false",
            &format!("water_bearing = true\n{STRIKE}"),
        );
        assert_refused(
            &text,
            "units[0] (RTD).strike: not allowed on a water-bearing unit (it already strikes at its top)",
        );
        // Made ground that is also water-bearing is still refused.
        let text = rtd_made_ground(&valid(), "true").replace(
            "made_ground = true",
            &format!("made_ground = true\nwater_bearing = true\n{STRIKE}"),
        );
        assert!(!errors_of(&text).is_empty());
    }

    const BANDS: &str = "rate_by_depth = [\
        { from_m = 0.0, rate_per_m = { min = 0.02, max = 0.02, basis = \"test fixture\" } }, \
        { from_m = 1.0, rate_per_m = { min = 0.03, max = 0.05, basis = \"test fixture\" } }, \
        { from_m = 3.0, rate_per_m = { min = 0.0, max = 0.1, basis = \"test fixture\" } }]";

    fn banded(bands: &str) -> String {
        strike_with(&STRIKE.replace(
            "rate_per_m = { min = 0.02, max = 0.04, basis = \"test fixture\" }",
            bands,
        ))
    }

    #[test]
    fn a_banded_strike_rate_loads_with_a_cited_range_per_band() {
        let p = Preset::from_toml(&banded(BANDS)).expect("valid");
        let s = p.units[1].strike.as_ref().expect("LC strikes");
        let StrikeRate::ByDepth(b) = &s.rate else {
            panic!("banded");
        };
        assert_eq!(
            b.iter().map(|b| b.from_m).collect::<Vec<_>>(),
            [0.0, 1.0, 3.0]
        );
        assert_eq!((b[1].rate_per_m.min, b[1].rate_per_m.max), (0.03, 0.05));
        assert_eq!(s.rate.ranges().len(), 3);
        assert!(s.rate.ranges().iter().all(|r| r.basis == "test fixture"));
    }

    #[test]
    fn a_strike_block_with_both_rates_or_neither_is_refused() {
        let both = strike_with(&STRIKE.replace("rise_m", &format!("{BANDS}, rise_m")));
        assert_refused(
            &both,
            "units[1] (LC).strike: has both rate_per_m and rate_by_depth (write one)",
        );
        let neither = strike_with(&STRIKE.replace(
            "rate_per_m = { min = 0.02, max = 0.04, basis = \"test fixture\" }, ",
            "",
        ));
        assert_refused(&neither, "units[1] (LC).strike.rate_per_m: missing");
    }

    #[test]
    fn a_banded_strike_rate_starts_at_ground_ascends_and_is_cited_and_non_negative() {
        assert_refused(
            &banded(&BANDS.replace("from_m = 0.0", "from_m = 0.5")),
            "units[1] (LC).strike.rate_by_depth[0].from_m: 0.5 must be 0 (ground level)",
        );
        assert_refused(
            &banded(&BANDS.replace("from_m = 3.0", "from_m = 1.0")),
            "units[1] (LC).strike.rate_by_depth[2].from_m: 1 must be deeper than the band above (1)",
        );
        assert_refused(
            &banded(&BANDS.replace("min = 0.0, max = 0.1", "min = -0.1, max = 0.1")),
            "units[1] (LC).strike.rate_by_depth[2].rate_per_m: min -0.1 is outside its domain (>= 0)",
        );
        assert_refused(
            &banded(&BANDS.replace(
                "{ min = 0.02, max = 0.02, basis = \"test fixture\" }",
                "{ min = 0.02, max = 0.02 }",
            )),
            "units[1] (LC).strike.rate_by_depth[0].rate_per_m: no basis",
        );
        assert_refused(
            &banded("rate_by_depth = []"),
            "units[1] (LC).strike.rate_by_depth: must not be empty",
        );
        assert_refused(
            &banded("rate_by_depth = 0.02"),
            "units[1] (LC).strike.rate_by_depth: must be an array of tables",
        );
        assert_refused(
            &banded(&BANDS.replace("from_m = 1.0, ", "")),
            "units[1] (LC).strike.rate_by_depth[1].from_m: missing",
        );
    }

    #[test]
    fn a_lens_takes_no_strike() {
        let lens = lens_with("host = \"LC\"", &format!("host = \"LC\"\n{STRIKE}"));
        let e = errors_of(&lens).join("\n");
        assert!(e.contains("lenses[0] (SB): unknown key \"strike\""), "{e}");
    }

    #[test]
    fn tyneside_till_strikes_in_each_till_with_cited_ranges() {
        let p = resolve("tyneside-till").unwrap();
        for id in ["WT", "RT", "GT"] {
            let u = p.units.iter().find(|u| u.id == id).unwrap();
            let s = u.strike.as_ref().unwrap_or_else(|| panic!("{id} strikes"));
            for r in s
                .rate
                .ranges()
                .into_iter()
                .chain([&s.rise_m, &s.no_rise_share])
            {
                assert!(r.basis.contains("OGL v3"), "{id}: {}", r.basis);
            }
            assert!(s.rate.ranges()[0].basis.contains("practice note"), "{id}");
            assert!(
                s.rate.ranges()[0]
                    .basis
                    .contains("CIRIA C504 Fig 6.3, p.117")
            );
            assert!(
                s.rate.ranges()[0]
                    .basis
                    .contains("Eyles & Sladen 1981, p.134")
            );
        }
        assert!(p.lenses.iter().all(|l| l.water_bearing.is_some()));
    }
}
