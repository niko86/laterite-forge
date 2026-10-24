//! Hole construction: `HDPH`, `CDIA`, `WSTG_CAS`, `BKFL` and, for installed
//! holes, `PIPE` (ADR 0001, "Hole construction").
//!
//! A preset with a `[construction]` block gives every hole:
//!
//! - **HDPH.** Two phases that tile the hole: a hand-dug inspection pit (`IP`)
//!   from the surface to the pit depth, then the hole's own method to final
//!   depth. `LOCA_TYPE` is the phases joined by `+` (`IP+CP`), as the crawls
//!   write it; both the Rust validator and python-ags4 accept the joined
//!   code when `ABBR` lists each part (see `collect_abbr`).
//! - **CDIA.** One casing string to the final casing depth, or two when the
//!   hole telescopes: a larger diameter to part depth and a smaller below, so
//!   a diameter never increases with depth. The final casing depth is the
//!   final depth less a drawn gap, never shallower than the pit base. There is
//!   no `HDIA`: it is rare in the crawls.
//! - **`WSTG_CAS`** on every strike: the strike depth less a drawn gap, never
//!   negative and never deeper than the strike or than the final casing. In a
//!   hole with several strikes (#79) a later strike's casing is then raised to
//!   at least the strike above it, or to that strike's seal where one is
//!   written. Each strike has its own keyed gap draw.
//! - **`WSTG_SEAL`** (a `[construction]` `seal` key, #79). A strike is sealed
//!   at `earlier_share` when another strike lies below it and at `last_share`
//!   when it is the hole's last. The seal sits a drawn gap below its strike,
//!   held at or above the next strike's casing and, for the last strike, the
//!   final casing. The dictionary lists `WSTG_SEAL` before `WSTG_CAS`.
//! - **BKFL.** A site draws once whether its holes carry backfill. If so, each
//!   hole is arisings alone, bentonite alone, or arisings over bentonite, the
//!   layers tiling 0 to final depth, written as legend codes only.
//!
//! **Casing and the strike.** The final casing passes the strike at the
//! preset's rate. A hole that passes draws its gap from the preset's
//! distribution cut off where the casing would end at or above the strike
//! (the same stream, scaled, so it is the distribution conditioned on
//! passing). A hole that does not pass ends its casing somewhere between the
//! pit base and the strike. In a hole with several strikes the final casing
//! passes every strike but the last, and the last at the same rate; a casing
//! that does not pass ends between the strike above and the last strike.
//!
//! **Installations** (`[construction.installation]`, #75). A site draws once
//! whether it installs at all and, if so, what share of its holes; each hole
//! draws against that share, and an installed hole has one installation: a
//! standpipe (plain from the surface to the response zone, slotted through it)
//! or a piezometer (plain, then a 0.3 m tip written as slotted). It writes a
//! `PIPE` row per run and its own backfill, replacing whatever the site's
//! backfill draw gave the hole.
//!
//! The response zone is placed in whole centimetres. Its top is the first
//! strike plus a drawn offset, or a drawn depth when the hole has no strike.
//! With `zone_follows_first_share` (#79) a hole with several strikes follows
//! the first at that share and the deepest otherwise, drawn under its own key.
//! It is then clamped, not resampled, into the room the hole leaves: not above
//! the pit base, nor so high that the cap, a seal of at least 0.1 m and the
//! 0.5 m of filter above the slot do not fit; not so low that 0.5 m of filter
//! and a tail of bentonite of at least 0.1 m do not fit above the final depth.
//! The drawn gap to the final depth is raised to that 0.6 m tail when it is
//! shorter. The preset loader rules out a final depth too shallow for the
//! longest slotted section, so the clamp always has room. The backfill, top
//! down: concrete cap (906), bentonite seal (903), a gravel (902) or sand
//! (901) filter from 0.5 m above the slot to 0.5 m below it, bentonite (903)
//! to the final depth. The layers tile 0 to the final depth exactly, and none
//! has zero thickness.
//!
//! Every draw is its own keyed stream under a `cons-` tag, so a preset without
//! the block draws nothing and keeps its bytes, and changing the block moves
//! only the construction rows (and `LOCA_TYPE`, and `WSTG_CAS`).

use laterite_ags4_validator::{DictVersion, Dictionary};

use super::build::Site;
use super::draw::{Key, uniform};
use super::preset::{
    CasingDiameter, Construction, FILTER_OVERLAP_M, InstallKindName, Installation, MIN_SEAL_M,
    Preset, Range,
};
use super::water;
use crate::synth::model::{Group, Row};

/// The inspection pit's `HDPH_TYPE` and `LOCA_TYPE` code.
const PIT: &str = "IP";

/// `BKFL_LEG`: arisings.
pub(super) const ARISINGS: &str = "905";

/// `BKFL_LEG`: bentonite.
pub(super) const BENTONITE: &str = "903";

/// `BKFL_LEG`: concrete.
const CONCRETE: &str = "906";

/// `BKFL_LEG`: gravel.
const GRAVEL: &str = "902";

/// `BKFL_LEG`: sand.
const SAND: &str = "901";

/// `PIPE_TYPE` of a plain run.
const PLAIN: &str = "PLAIN";

/// `PIPE_TYPE` of a slotted run (the piezometer tip too: the dictionary has
/// no other value).
const SLOTTED: &str = "SLOTTED";

/// `PIPE_REF` of the hole's one installation.
const PIPE_REF: &str = "1";

/// Length of a piezometer's tip, centimetres.
const TIP_CM: i64 = 30;

/// Backfill layers, top-down: `(BKFL_TOP, BKFL_BASE, BKFL_LEG)`.
type Layers = Vec<(f64, f64, &'static str)>;

/// What construction put into every hole.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct Plan {
    pub holes: Vec<HoleBuild>,
    /// Whether `WSTG` carries a `WSTG_SEAL` column.
    pub has_seal: bool,
}

/// One hole's construction.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct HoleBuild {
    /// Base of the inspection pit, 2 decimal places.
    pub pit_base: f64,
    /// The hole's own method: the phase from the pit base to final depth.
    pub phase: String,
    /// `(CDIA_DPTH, CDIA_DIAM)` top-down: one string or two, the diameter
    /// never increasing.
    pub casing: Vec<(f64, u32)>,
    /// `WSTG_CAS` for each of the hole's strikes in the file, shallowest
    /// first; empty when it has none.
    pub strike_casing: Vec<f64>,
    /// `WSTG_SEAL` for each strike, in the same order; `None` where the
    /// strike is not sealed. Empty when the preset writes no seals.
    pub strike_seal: Vec<Option<f64>>,
    /// `(BKFL_TOP, BKFL_BASE, BKFL_LEG)` top-down; empty when the site's holes
    /// carry no backfill.
    /// For an installed hole this is the installed sequence.
    pub backfill: Vec<(f64, f64, &'static str)>,
    /// The hole's installation, when it has one.
    pub installation: Option<Installed>,
}

/// One installation: `PIPE_DIAM` and the slotted section.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct Installed {
    pub kind: InstallKindName,
    pub mm: u32,
    /// Top and base of the slotted section, 2 decimal places.
    pub zone: (f64, f64),
}

impl HoleBuild {
    /// The `LOCA_TYPE` the phases make: `IP+CP`.
    pub(super) fn loca_type(&self) -> String {
        format!("{PIT}+{}", self.phase)
    }
}

/// The draws that belong to the whole site, made once.
struct SiteDraws {
    telescope_share: f64,
    passes_strike_share: f64,
    has_backfill: bool,
    /// The share of holes installed, when the site installs at all.
    install_share: Option<f64>,
    /// In a hole with several strikes, the share whose installation follows
    /// the first strike, when the preset gives one.
    zone_follows_first: Option<f64>,
}

impl SiteDraws {
    fn new(c: &Construction, seed: u64) -> Self {
        let share = |tag: &str, r: &Range| uniform(&mut Key::site(tag).rng(seed), r.min, r.max);
        let backfill_share = share("cons-backfill-share", &c.backfill.site_share);
        let site_u = uniform(&mut Key::site("cons-backfill-site").rng(seed), 0.0, 1.0);
        SiteDraws {
            telescope_share: share("cons-telescope-share", &c.casing.telescope_share),
            passes_strike_share: share("cons-pass-share", &c.casing.passes_strike_share),
            has_backfill: site_u < backfill_share,
            install_share: c.installation.as_ref().and_then(|i| {
                let site_u = uniform(&mut Key::site("cons-install-site").rng(seed), 0.0, 1.0);
                let share = uniform(
                    &mut Key::site("cons-install-share").rng(seed),
                    i.hole_share.min,
                    i.hole_share.max,
                );
                let site_share = uniform(
                    &mut Key::site("cons-install-site-share").rng(seed),
                    i.site_share.min,
                    i.site_share.max,
                );
                (site_u < site_share).then_some(share)
            }),
            zone_follows_first: c
                .installation
                .as_ref()
                .and_then(|i| i.zone_follows_first_share.as_ref())
                .map(|r| {
                    uniform(
                        &mut Key::site("cons-install-follows-share").rng(seed),
                        r.min,
                        r.max,
                    )
                }),
        }
    }
}

/// The construction of every hole, or `None` when the preset has none.
pub(super) fn plan(site: &Site, preset: &Preset, seed: u64) -> Option<Plan> {
    let c = preset.construction.as_ref()?;
    // A strike is in the file only when the suite lists WSTG; casing is
    // arranged around it only then.
    let strikes = if preset.suite.iter().any(|e| e.group == "WSTG") {
        water::strikes(site, preset, seed)
    } else {
        Vec::new()
    };
    let draws = SiteDraws::new(c, seed);
    let holes = site
        .holes
        .iter()
        .enumerate()
        .map(|(hi, hole)| {
            let depths: Vec<f64> = strikes
                .iter()
                .filter(|s| s.hole == hi)
                .map(|s| s.depth)
                .collect();
            build_hole(
                c,
                &draws,
                seed,
                hi,
                hole.final_depth,
                &hole.loca_type,
                &depths,
            )
        })
        .collect();
    Some(Plan {
        holes,
        has_seal: c.seal.is_some(),
    })
}

fn build_hole(
    c: &Construction,
    draws: &SiteDraws,
    seed: u64,
    hi: usize,
    final_depth: f64,
    phase: &str,
    strikes: &[f64],
) -> HoleBuild {
    let pit_base = round2(uniform(
        &mut Key::hole("cons-pit", hi).rng(seed),
        c.inspection_pit_m.min,
        c.inspection_pit_m.max,
    ));
    let final_casing = final_casing(c, draws, seed, hi, final_depth, pit_base, strikes);
    let casing = casing_strings(c, draws, seed, hi, pit_base, final_casing);
    let strike_seal = seals(c, seed, hi, strikes, final_casing);
    let strike_casing = strike_casings(c, seed, hi, strikes, &strike_seal, final_casing);
    let installed = c
        .installation
        .as_ref()
        .zip(draws.install_share)
        .and_then(|(i, share)| {
            install(
                i,
                (share, draws.zone_follows_first),
                seed,
                hi,
                final_depth,
                pit_base,
                strikes,
            )
        });
    // The site's own backfill draw is made whether or not the hole is
    // installed, so installing it moves nothing else.
    let site_backfill = backfill(c, draws, seed, hi, final_depth);
    let (installation, backfill) = match installed {
        Some((inst, layers)) => (Some(inst), layers),
        None => (None, site_backfill),
    };
    HoleBuild {
        pit_base,
        phase: phase.to_string(),
        casing,
        strike_casing,
        strike_seal,
        backfill,
        installation,
    }
}

/// The `WSTG_CAS` of each strike: the strike depth less a drawn gap, never
/// negative, never past the strike or the final casing. A later strike's
/// casing is raised to at least the strike above it, or to that strike's seal
/// where one is written. The first strike's draw is the one a hole with a
/// single strike has always made; each later strike has its own keyed draw.
fn strike_casings(
    c: &Construction,
    seed: u64,
    hi: usize,
    strikes: &[f64],
    seals: &[Option<f64>],
    final_casing: f64,
) -> Vec<f64> {
    strikes
        .iter()
        .enumerate()
        .map(|(j, &s)| {
            let key = if j == 0 {
                Key::hole("cons-strike-casing", hi)
            } else {
                Key {
                    test: j as u64,
                    ..Key::hole("cons-strike-casing", hi)
                }
            };
            let u = uniform(&mut key.rng(seed), 0.0, 1.0);
            let drawn = round2((s - skewed(&c.casing.strike_gap_m, u)).max(0.0));
            let floor = match j.checked_sub(1) {
                None => 0.0,
                Some(k) => seals.get(k).copied().flatten().unwrap_or(strikes[k]),
            };
            drawn.max(floor).min(final_casing).min(s)
        })
        .collect()
}

/// The `WSTG_SEAL` of each strike, or nothing when the preset writes none.
/// A strike is sealed at the preset's share for an earlier strike or for the
/// last one. The seal sits a drawn gap below its strike, but no deeper than
/// the next strike (it must not pass it) and the final casing: an earlier
/// strike's seal then lies at or above the next strike's casing, and the last
/// strike's at or above the final casing.
fn seals(
    c: &Construction,
    seed: u64,
    hi: usize,
    strikes: &[f64],
    final_casing: f64,
) -> Vec<Option<f64>> {
    let Some(seal) = &c.seal else {
        return Vec::new();
    };
    strikes
        .iter()
        .enumerate()
        .map(|(j, &s)| {
            let at = |tag| Key {
                test: j as u64,
                ..Key::hole(tag, hi)
            };
            let u_has = uniform(&mut at("cons-seal-has").rng(seed), 0.0, 1.0);
            let u_gap = uniform(&mut at("cons-seal-gap").rng(seed), 0.0, 1.0);
            let next = strikes.get(j + 1);
            let share = if next.is_some() {
                seal.earlier_share
            } else {
                seal.last_share
            };
            (u_has < share).then(|| {
                let drawn = round2(s + skewed(&seal.gap_m, u_gap));
                drawn.min(next.map_or(final_casing, |n| n.min(final_casing)))
            })
        })
        .collect()
}

/// The installation of a hole, with its backfill, or `None` when the hole is
/// not installed. Depths are worked in whole centimetres.
fn install(
    i: &Installation,
    (share, follows_first): (f64, Option<f64>),
    seed: u64,
    hi: usize,
    final_depth: f64,
    pit_base: f64,
    strikes: &[f64],
) -> Option<(Installed, Layers)> {
    let u_in = uniform(&mut Key::hole("cons-install-hole", hi).rng(seed), 0.0, 1.0);
    if u_in >= share {
        return None;
    }
    let mut kind_rng = Key::hole("cons-install-kind", hi).rng(seed);
    let u_kind = uniform(&mut kind_rng, 0.0, 1.0);
    let mut zone_rng = Key::hole("cons-install-zone", hi).rng(seed);
    let (u_top, u_len, u_gap) = (
        uniform(&mut zone_rng, 0.0, 1.0),
        uniform(&mut zone_rng, 0.0, 1.0),
        uniform(&mut zone_rng, 0.0, 1.0),
    );
    let mut fill_rng = Key::hole("cons-install-filter", hi).rng(seed);
    let (u_cap, u_filter) = (
        uniform(&mut fill_rng, 0.0, 1.0),
        uniform(&mut fill_rng, 0.0, 1.0),
    );
    let weights: Vec<f64> = i.kinds.iter().map(|k| k.weight).collect();
    let kind = &i.kinds[pick_index(&weights, u_kind)];

    let fd = cm(final_depth);
    let overlap = cm(FILTER_OVERLAP_M);
    let cap = cm(skewed(&i.cap_m, u_cap));
    let tail = overlap + cm(MIN_SEAL_M);
    // The highest the zone top may be: below the pit, and with the cap, a
    // seal and the filter above it.
    let lo = cm(pit_base).max(cap + cm(MIN_SEAL_M) + overlap);
    let len = match kind.kind {
        InstallKindName::Standpipe => cm(skewed(&i.slotted_length_m, u_len)).max(1),
        InstallKindName::Piezometer => TIP_CM,
    };
    // The drawn gap, raised to the tail the filter and seal need.
    let gap = cm(skewed(&i.base_gap_m, u_gap)).max(tail);
    let base_max = (fd - gap).max(lo + len);
    // The zone follows the first strike, or in a hole with several (and a
    // share to draw against) the deepest.
    let follows = match (strikes, follows_first) {
        ([first, .., last], Some(share)) => {
            let u = uniform(
                &mut Key::hole("cons-install-zone-strike", hi).rng(seed),
                0.0,
                1.0,
            );
            Some(if u < share { *first } else { *last })
        }
        _ => strikes.first().copied(),
    };
    let raw = match follows {
        Some(s) => cm(s) + cm(skewed(&i.zone_offset_m, u_top)),
        None => cm(skewed(&i.zone_top_m, u_top)),
    };
    let top = raw.min(base_max - len).max(lo);
    let base = top + len;
    let (ft, fb) = (top - overlap, base + overlap);
    let leg = if pick_index(&i.filter_weights, u_filter) == 0 {
        GRAVEL
    } else {
        SAND
    };
    let layers = [
        (0, cap, CONCRETE),
        (cap, ft, BENTONITE),
        (ft, fb, leg),
        (fb, fd, BENTONITE),
    ]
    .into_iter()
    .filter(|(t, b, _)| b > t)
    .map(|(t, b, l)| (m(t), m(b), l))
    .collect();
    Some((
        Installed {
            kind: kind.kind,
            mm: kind.mm,
            zone: (m(top), m(base)),
        },
        layers,
    ))
}

/// The index at cumulative weight `u` of the total; never a zero-weight one.
fn pick_index(weights: &[f64], u: f64) -> usize {
    let target = u * weights.iter().sum::<f64>();
    let mut cum = 0.0;
    for (k, w) in weights.iter().enumerate() {
        cum += w;
        if *w > 0.0 && target < cum {
            return k;
        }
    }
    weights
        .iter()
        .rposition(|w| *w > 0.0)
        .expect("a weight is positive")
}

/// Metres to whole centimetres.
#[allow(clippy::cast_possible_truncation)]
fn cm(v: f64) -> i64 {
    (v * 100.0).round() as i64
}

/// Whole centimetres to metres.
#[allow(clippy::cast_precision_loss)]
fn m(c: i64) -> f64 {
    c as f64 / 100.0
}

/// The final casing depth: the final depth less a drawn gap, arranged around
/// the strike as the module doc says.
fn final_casing(
    c: &Construction,
    draws: &SiteDraws,
    seed: u64,
    hi: usize,
    final_depth: f64,
    pit_base: f64,
    strikes: &[f64],
) -> f64 {
    let strike = strikes.last().copied();
    // The strike above the last, which the casing always passes.
    let earlier = strikes.len().checked_sub(2).map(|k| strikes[k]);
    let mut rng = Key::hole("cons-casing", hi).rng(seed);
    let u_gap = uniform(&mut rng, 0.0, 1.0);
    let u_pass = uniform(&mut rng, 0.0, 1.0);
    let u_above = uniform(&mut rng, 0.0, 1.0);
    let passes = u_pass < draws.passes_strike_share;
    let deepest_above = |floor: f64| {
        // The deepest the casing may end below the final depth: the gap
        // cannot take it above `floor`.
        let room = (final_depth - floor).max(0.0);
        let r = &c.casing.final_gap_m;
        let cap = if room <= r.min {
            0.0
        } else if r.max <= r.min {
            1.0
        } else {
            ((room - r.min) / (r.max - r.min))
                .powf(1.0 / r.skew.unwrap_or(1.0))
                .min(1.0)
        };
        let gap = skewed(r, u_gap * cap).min(room);
        round2(final_depth - gap).max(round2(floor))
    };
    match strike {
        // Passing the strike: the casing ends below it.
        Some(s) if passes && s + 0.01 <= final_depth => {
            deepest_above(pit_base.max(round2(s + 0.01)))
        }
        // Not passing: between the pit base (or, in a hole with several
        // strikes, the strike above) and the last strike.
        Some(s) => {
            let hi_ = s.min(final_depth);
            let lo = earlier.map_or(pit_base, |e| pit_base.max(round2(e + 0.01)));
            round2(lo + (hi_ - lo).max(0.0) * u_above).min(final_depth)
        }
        None => deepest_above(pit_base),
    }
}

/// The casing strings: `(base depth, diameter)` top-down.
fn casing_strings(
    c: &Construction,
    draws: &SiteDraws,
    seed: u64,
    hi: usize,
    pit_base: f64,
    final_casing: f64,
) -> Vec<(f64, u32)> {
    let mut rng = Key::hole("cons-diameter", hi).rng(seed);
    let u_first = uniform(&mut rng, 0.0, 1.0);
    let u_tele = uniform(&mut rng, 0.0, 1.0);
    let u_second = uniform(&mut rng, 0.0, 1.0);
    let u_part = uniform(&mut rng, 0.0, 1.0);
    let all: Vec<&CasingDiameter> = c.casing.diameters.iter().collect();
    let first = pick(&all, u_first).mm;
    let smaller: Vec<&CasingDiameter> = all.into_iter().filter(|d| d.mm < first).collect();
    let step = (u_tele < draws.telescope_share && smaller.iter().any(|d| d.weight > 0.0))
        .then(|| pick(&smaller, u_second).mm);
    if let Some(second) = step {
        let part = round2(
            pit_base
                + skewed(&c.casing.telescope_depth_fraction, u_part) * (final_casing - pit_base),
        );
        if part > pit_base && part < final_casing {
            return vec![(part, first), (final_casing, second)];
        }
    }
    vec![(final_casing, first)]
}

/// The diameter at cumulative weight `u` of the total; never a zero-weight one.
fn pick<'a>(items: &[&'a CasingDiameter], u: f64) -> &'a CasingDiameter {
    let total: f64 = items.iter().map(|d| d.weight).sum();
    let target = u * total;
    let mut cum = 0.0;
    for d in items {
        cum += d.weight;
        if d.weight > 0.0 && target < cum {
            return d;
        }
    }
    items
        .iter()
        .rev()
        .find(|d| d.weight > 0.0)
        .expect("a diameter has weight")
}

/// The backfill layers of a hole, top-down, tiling 0 to `final_depth`.
fn backfill(
    c: &Construction,
    draws: &SiteDraws,
    seed: u64,
    hi: usize,
    final_depth: f64,
) -> Vec<(f64, f64, &'static str)> {
    let mut rng = Key::hole("cons-backfill", hi).rng(seed);
    let u_seq = uniform(&mut rng, 0.0, 1.0);
    let u_boundary = uniform(&mut rng, 0.0, 1.0);
    if !draws.has_backfill {
        return Vec::new();
    }
    let w = c.backfill.sequence_weights;
    let target = u_seq * w.iter().sum::<f64>();
    let pick = if target < w[0] {
        0
    } else if target < w[0] + w[1] {
        1
    } else {
        2
    };
    // A zero weight is never chosen: the cumulative test above skips an empty
    // band only when the band lies at the end, so settle those by weight.
    let pick = if w[pick] > 0.0 {
        pick
    } else {
        (0..3)
            .rev()
            .find(|&i| w[i] > 0.0)
            .expect("a weight is positive")
    };
    let boundary =
        round2(skewed(&c.backfill.boundary_m, u_boundary)).clamp(0.01, round2(final_depth - 0.01));
    match pick {
        0 => vec![(0.0, final_depth, ARISINGS)],
        1 => vec![(0.0, final_depth, BENTONITE)],
        _ => vec![
            (0.0, boundary, ARISINGS),
            (boundary, final_depth, BENTONITE),
        ],
    }
}

/// `min + (max - min) * u^skew`.
fn skewed(r: &Range, u: f64) -> f64 {
    r.min + (r.max - r.min) * u.powf(r.skew.unwrap_or(1.0))
}

/// The construction groups, in file order: `HDPH`, `CDIA`, then `BKFL` when
/// the site's holes carry it, then `PIPE` when a hole is installed.
pub(super) fn groups(site: &Site, plan: &Plan) -> Vec<Group> {
    let hole = |i: usize| (&site.holes[i], &plan.holes[i]);
    let hdph = (0..site.holes.len()).flat_map(|i| {
        let (h, b) = hole(i);
        [
            Row::owned(vec![h.id.clone(), m2(0.0), m2(b.pit_base), PIT.to_string()]),
            Row::owned(vec![
                h.id.clone(),
                m2(b.pit_base),
                m2(h.final_depth),
                b.phase.clone(),
            ]),
        ]
    });
    let cdia = (0..site.holes.len()).flat_map(|i| {
        let (h, b) = hole(i);
        b.casing
            .iter()
            .map(|&(depth, mm)| Row::owned(vec![h.id.clone(), m2(depth), mm.to_string()]))
            .collect::<Vec<_>>()
    });
    let bkfl = (0..site.holes.len()).flat_map(|i| {
        let (h, b) = hole(i);
        b.backfill
            .iter()
            .map(|&(top, base, leg)| {
                Row::owned(vec![h.id.clone(), m2(top), m2(base), leg.to_string()])
            })
            .collect::<Vec<_>>()
    });
    let pipe: Vec<Row> = (0..site.holes.len())
        .flat_map(|i| {
            let (h, b) = hole(i);
            b.installation
                .iter()
                .flat_map(|inst| {
                    let (top, base) = inst.zone;
                    [(0.0, top, PLAIN), (top, base, SLOTTED)]
                        .into_iter()
                        .map(|(t, bs, ty)| {
                            Row::owned(vec![
                                h.id.clone(),
                                PIPE_REF.to_string(),
                                m2(t),
                                m2(bs),
                                inst.mm.to_string(),
                                ty.to_string(),
                            ])
                        })
                        .collect::<Vec<_>>()
                })
                .collect::<Vec<_>>()
        })
        .collect();
    let mut out = vec![
        dict_group(
            "HDPH",
            &["LOCA_ID", "HDPH_TOP", "HDPH_BASE", "HDPH_TYPE"],
            hdph.collect(),
        ),
        dict_group(
            "CDIA",
            &["LOCA_ID", "CDIA_DPTH", "CDIA_DIAM"],
            cdia.collect(),
        ),
    ];
    let bkfl: Vec<Row> = bkfl.collect();
    if !bkfl.is_empty() {
        out.push(dict_group(
            "BKFL",
            &["LOCA_ID", "BKFL_TOP", "BKFL_BASE", "BKFL_LEG"],
            bkfl,
        ));
    }
    if !pipe.is_empty() {
        out.push(dict_group(
            "PIPE",
            &[
                "LOCA_ID",
                "PIPE_REF",
                "PIPE_TOP",
                "PIPE_BASE",
                "PIPE_DIAM",
                "PIPE_TYPE",
            ],
            pipe,
        ));
    }
    out
}

/// `code` over `headings`, with the units and types from the dictionary.
fn dict_group(code: &str, headings: &[&str], rows: Vec<Row>) -> Group {
    let dict = Dictionary::bundled(DictVersion::V4_2);
    let entry = |h: &str| dict.heading(code, h).expect("heading exists");
    Group {
        code: code.to_string(),
        headings: headings.iter().map(ToString::to_string).collect(),
        units: headings.iter().map(|h| entry(h).unit.to_string()).collect(),
        types: headings
            .iter()
            .map(|h| entry(h).ags_type.to_string())
            .collect(),
        rows,
    }
}

fn m2(v: f64) -> String {
    format!("{v:.2}")
}

fn round2(v: f64) -> f64 {
    (v * 100.0).round() / 100.0
}

#[cfg(test)]
// Depths are written to 2 decimal places and compared as the exact values
// the plan rounded to.
#[allow(clippy::float_cmp)]
mod tests {
    use super::super::build::{self, Grid, tests::FIXTURE};
    use super::super::preset::Preset;
    use super::*;
    use crate::pipeline::rust_check;
    use crate::synth::emit::emit_to_string;
    use crate::synth::model::ProjectModel;
    use laterite_ags4_parity::RustResult;
    use std::collections::BTreeMap;

    /// Water groups, so there are strikes to case.
    const WATER: &str = r#"
[[suite]]
group = "WSTG"
headings = []

[[suite]]
group = "WSTD"
headings = []
"#;

    /// A construction block on the main fixture. Its shortest final depth is
    /// 20 m, above every pit and boundary maximum.
    const BLOCK: &str = r#"
[construction]
inspection_pit_m = { min = 1.0, max = 1.4, basis = "test fixture" }

[construction.casing]
diameters = [
    { mm = 200, weight = 3 },
    { mm = 150, weight = 2 },
    { mm = 125, weight = 1 },
]
diameters_basis = "test fixture"
telescope_share = { min = 0.5, max = 0.5, basis = "test fixture" }
telescope_depth_fraction = { min = 0.2, max = 0.8, basis = "test fixture" }
final_gap_m = { min = 0.0, max = 10.0, skew = 2.0, basis = "test fixture" }
passes_strike_share = { min = 0.8, max = 0.8, basis = "test fixture" }
strike_gap_m = { min = 0.0, max = 3.0, skew = 2.0, basis = "test fixture" }

[construction.backfill]
site_share = { min = 0.5, max = 0.5, basis = "test fixture" }
sequence = { weights = [1, 1, 1], basis = "test fixture" }
boundary_m = { min = 0.3, max = 1.5, basis = "test fixture" }
"#;

    fn text() -> String {
        format!("{FIXTURE}{WATER}{BLOCK}")
    }

    /// The fixture text with one exact snippet of the block replaced.
    fn with(from: &str, to: &str) -> String {
        assert!(BLOCK.contains(from), "block fixture lacks {from:?}");
        text().replacen(from, to, 1)
    }

    fn preset() -> Preset {
        Preset::from_toml(&text()).expect("the construction fixture loads")
    }

    fn model(p: &Preset, seed: u64, holes: usize) -> ProjectModel {
        build::lower(&build::build(p, seed, holes), p, seed, Grid::Local)
    }

    fn rows<'a>(m: &'a ProjectModel, code: &str) -> Vec<&'a [String]> {
        m.groups
            .iter()
            .find(|g| g.code == code)
            .map_or_else(Vec::new, |g| {
                g.rows.iter().map(|r| r.values.as_slice()).collect()
            })
    }

    fn num(s: &str) -> f64 {
        s.parse().expect("a number")
    }

    /// Every group's rows by code, for comparing two files group by group.
    fn by_group(m: &ProjectModel) -> BTreeMap<String, Vec<Vec<String>>> {
        m.groups
            .iter()
            .map(|g| {
                (
                    g.code.clone(),
                    g.rows.iter().map(|r| r.values.clone()).collect(),
                )
            })
            .collect()
    }

    /// The groups that differ between two files.
    fn differing(a: &ProjectModel, b: &ProjectModel) -> Vec<String> {
        let (a, b) = (by_group(a), by_group(b));
        let mut out: Vec<String> = a
            .keys()
            .chain(b.keys())
            .filter(|k| a.get(*k) != b.get(*k))
            .cloned()
            .collect();
        out.sort();
        out.dedup();
        out
    }

    fn refused(text: &str, needle: &str) {
        let errs = Preset::from_toml(text).expect_err("should be refused").0;
        assert!(
            errs.iter().any(|e| e.contains(needle)),
            "{needle:?} is not among {errs:#?}"
        );
    }

    #[test]
    fn the_loader_reads_the_block_into_typed_values() {
        let p = preset();
        let c = p.construction.as_ref().expect("a construction block");
        assert_eq!((c.inspection_pit_m.min, c.inspection_pit_m.max), (1.0, 1.4));
        let diameters: Vec<(u32, f64)> = c
            .casing
            .diameters
            .iter()
            .map(|d| (d.mm, d.weight))
            .collect();
        assert_eq!(diameters, [(200, 3.0), (150, 2.0), (125, 1.0)]);
        assert_eq!(c.casing.final_gap_m.skew, Some(2.0));
        assert_eq!(c.casing.telescope_depth_fraction.skew, None);
        assert_eq!(c.backfill.sequence_weights, [1.0, 1.0, 1.0]);
        assert!(Preset::from_toml(FIXTURE).unwrap().construction.is_none());
    }

    #[test]
    fn the_loader_refuses_a_share_outside_zero_to_one() {
        refused(
            &with(
                "telescope_share = { min = 0.5,",
                "telescope_share = { min = -0.1,",
            ),
            "construction.casing.telescope_share: min -0.1 is outside 0 to 1",
        );
        refused(
            &with(
                "passes_strike_share = { min = 0.8, max = 0.8,",
                "passes_strike_share = { min = 0.8, max = 1.2,",
            ),
            "construction.casing.passes_strike_share: max 1.2 is outside 0 to 1",
        );
        refused(
            &with(
                "site_share = { min = 0.5, max = 0.5,",
                "site_share = { min = 0.5, max = 1.5,",
            ),
            "construction.backfill.site_share: max 1.5 is outside 0 to 1",
        );
    }

    #[test]
    fn the_loader_refuses_a_negative_gap() {
        refused(
            &with("final_gap_m = { min = 0.0,", "final_gap_m = { min = -1.0,"),
            "construction.casing.final_gap_m: min -1 is negative",
        );
        refused(
            &with(
                "strike_gap_m = { min = 0.0,",
                "strike_gap_m = { min = -1.0,",
            ),
            "construction.casing.strike_gap_m: min -1 is negative",
        );
    }

    #[test]
    fn the_loader_refuses_a_pit_or_boundary_not_above_the_shortest_final_depth() {
        refused(
            &with(
                "inspection_pit_m = { min = 1.0, max = 1.4,",
                "inspection_pit_m = { min = 1.0, max = 20.0,",
            ),
            "construction.inspection_pit_m: max 20 is not above the shortest final depth \
             (hole_types[0].final_depth_m min 20)",
        );
        refused(
            &with(
                "boundary_m = { min = 0.3, max = 1.5,",
                "boundary_m = { min = 0.3, max = 25.0,",
            ),
            "construction.backfill.boundary_m: max 25 is not above the shortest final depth",
        );
        refused(
            &with(
                "inspection_pit_m = { min = 1.0,",
                "inspection_pit_m = { min = 0.0,",
            ),
            "construction.inspection_pit_m: min 0 must be > 0",
        );
    }

    #[test]
    fn the_loader_refuses_diameter_weights_that_do_not_sum_above_zero() {
        let zero = with("weight = 3 }", "weight = 0 }")
            .replacen("weight = 2 }", "weight = 0 }", 1)
            .replacen("weight = 1 }", "weight = 0 }", 1);
        refused(&zero, "construction.casing.diameters: weights sum to zero");
        refused(
            &with("weight = 3 }", "weight = -3 }"),
            "construction.casing.diameters[0].weight: -3 is negative",
        );
        refused(
            &with("{ mm = 150,", "{ mm = 200,"),
            "construction.casing.diameters[1]: 200 mm is listed twice",
        );
        refused(
            &with("{ mm = 150,", "{ mm = 0,"),
            "construction.casing.diameters[1].mm: 0 must be an integer >= 1",
        );
        refused(
            &with("weights = [1, 1, 1]", "weights = [0, 0, 0]"),
            "construction.backfill.sequence.weights: weights sum to zero",
        );
    }

    #[test]
    fn the_loader_refuses_a_hole_method_that_is_not_a_phase_code() {
        // CPT is a LOCA_TYPE code but not an HDPH_TYPE one.
        let dict = Dictionary::bundled(DictVersion::V4_2);
        assert!(dict.abbr_codes("LOCA_TYPE").contains(&"CPT"));
        assert!(!dict.abbr_codes("HDPH_TYPE").contains(&"CPT"));
        refused(
            &text().replacen("code = \"RC\"", "code = \"CPT\"", 1),
            "hole_types[1].code: \"CPT\" cannot follow the inspection pit",
        );
        // The same hole type is fine in a preset with no construction.
        let plain = format!("{FIXTURE}{WATER}").replacen("code = \"RC\"", "code = \"CPT\"", 1);
        assert!(Preset::from_toml(&plain).is_ok());
    }

    #[test]
    fn the_loader_refuses_a_missing_basis_unknown_key_and_missing_table() {
        refused(
            &with(
                "sequence = { weights = [1, 1, 1], basis = \"test fixture\" }",
                "sequence = { weights = [1, 1, 1] }",
            ),
            "construction.backfill.sequence: no basis",
        );
        refused(
            &with(
                "final_gap_m = { min = 0.0, max = 10.0, skew = 2.0, basis = \"test fixture\" }",
                "final_gap_m = { min = 0.0, max = 10.0, skew = 2.0 }",
            ),
            "construction.casing.final_gap_m: no basis",
        );
        refused(
            &with(
                "diameters_basis = \"test fixture\"",
                "diameters_basis = \" \"",
            ),
            "construction.casing.diameters_basis: empty",
        );
        refused(
            &with(
                "[construction.backfill]",
                "[construction.backfill]\nextra = 1",
            ),
            "construction.backfill: unknown key \"extra\"",
        );
        let cut = BLOCK.find("[construction.backfill]").unwrap();
        refused(
            &text().replace(&BLOCK[cut..], ""),
            "construction.backfill: missing",
        );
        refused(
            &with(
                "telescope_depth_fraction = { min = 0.2, max = 0.8,",
                "telescope_depth_fraction = { min = 0.2, max = 1.0,",
            ),
            "construction.casing.telescope_depth_fraction: max 1 must be < 1",
        );
    }

    #[test]
    fn the_codes_it_writes_are_dictionary_pick_list_values() {
        let dict = Dictionary::bundled(DictVersion::V4_2);
        assert!(dict.abbr_codes("HDPH_TYPE").contains(&PIT));
        assert!(dict.abbr_codes("HDPH_TYPE").contains(&"CP"));
        assert!(dict.abbr_codes("LOCA_TYPE").contains(&PIT));
        let legends = dict.abbr_codes("BKFL_LEG");
        assert!(legends.contains(&ARISINGS) && legends.contains(&BENTONITE));
    }

    #[test]
    fn hdph_is_two_phases_that_tile_the_hole_and_loca_type_joins_them() {
        let p = preset();
        for seed in 0..30 {
            let site = build::build(&p, seed, 5);
            let m = build::lower(&site, &p, seed, Grid::Local);
            let hdph = rows(&m, "HDPH");
            let loca = rows(&m, "LOCA");
            assert_eq!(hdph.len(), site.holes.len() * 2, "seed {seed}");
            for (i, hole) in site.holes.iter().enumerate() {
                let (pit, bore) = (hdph[2 * i], hdph[2 * i + 1]);
                assert_eq!(pit[0], hole.id);
                assert_eq!((pit[1].as_str(), pit[3].as_str()), ("0.00", "IP"));
                assert!((1.0..=1.4).contains(&num(&pit[2])), "pit {}", pit[2]);
                assert_eq!(bore[0], hole.id);
                assert_eq!(bore[1], pit[2], "the phases tile, seed {seed}");
                assert_eq!(num(&bore[2]), hole.final_depth, "to final depth");
                assert_eq!(bore[3], hole.loca_type);
                assert_eq!(loca[i][1], format!("IP+{}", hole.loca_type));
            }
        }
    }

    /// `(CDIA_DPTH, CDIA_DIAM)` per hole, top-down.
    fn casing(m: &ProjectModel) -> BTreeMap<String, Vec<(f64, u32)>> {
        let mut out: BTreeMap<String, Vec<(f64, u32)>> = BTreeMap::new();
        for r in rows(m, "CDIA") {
            out.entry(r[0].clone())
                .or_default()
                .push((num(&r[1]), r[2].parse().unwrap()));
        }
        out
    }

    #[test]
    fn every_hole_is_cased_with_diameters_that_never_increase_and_depths_in_bounds() {
        let p = preset();
        let (mut telescoped, mut single) = (0, 0);
        for seed in 0..60 {
            let site = build::build(&p, seed, 6);
            let m = build::lower(&site, &p, seed, Grid::Local);
            let cased = casing(&m);
            assert_eq!(cased.len(), site.holes.len(), "CDIA in every hole");
            let hdph = rows(&m, "HDPH");
            for (i, hole) in site.holes.iter().enumerate() {
                let strings = &cased[&hole.id];
                assert!(strings.len() == 1 || strings.len() == 2);
                let pit_base = num(&hdph[2 * i][2]);
                for w in strings.windows(2) {
                    assert!(w[0].0 < w[1].0, "depths increase: {strings:?}");
                    assert!(w[0].1 > w[1].1, "diameters step down: {strings:?}");
                }
                for &(depth, _) in strings {
                    assert!(depth >= pit_base, "casing above the pit base, seed {seed}");
                    assert!(depth <= hole.final_depth, "casing below the hole");
                }
                assert!(strings.iter().all(|s| [200, 150, 125].contains(&s.1)));
                if strings.len() == 2 {
                    telescoped += 1;
                } else {
                    single += 1;
                }
            }
        }
        assert!(
            telescoped > 20 && single > 20,
            "{telescoped} telescoped, {single} single"
        );
    }

    #[test]
    fn wstg_cas_is_on_every_strike_never_negative_and_within_the_casing() {
        let p = preset();
        let mut seen = 0;
        for seed in 0..80 {
            let m = model(&p, seed, 6);
            let cased = casing(&m);
            for r in rows(&m, "WSTG") {
                assert_eq!(r.len(), 3, "WSTG_CAS is written");
                let (strike, cas) = (num(&r[1]), num(&r[2]));
                let deepest = cased[&r[0]].last().unwrap().0;
                assert!(cas >= 0.0, "negative WSTG_CAS, seed {seed}");
                assert!(cas <= strike, "casing {cas} below the strike {strike}");
                assert!(
                    cas <= deepest,
                    "casing {cas} below the final casing {deepest}"
                );
                seen += 1;
            }
        }
        assert!(seen > 50, "{seen} strikes is too few to mean anything");
    }

    #[test]
    fn a_preset_without_a_water_group_writes_no_wstg() {
        let no_water = format!("{FIXTURE}{BLOCK}");
        let p = Preset::from_toml(&no_water).unwrap();
        let m = model(&p, 3, 5);
        assert!(rows(&m, "WSTG").is_empty());
        let strings: usize = casing(&m).values().map(Vec::len).sum();
        assert_eq!(rows(&m, "CDIA").len(), strings);
    }

    #[test]
    fn the_final_casing_passes_the_strike_at_the_presets_rate() {
        let p = preset();
        let (mut passed, mut total) = (0usize, 0usize);
        for seed in 0..400 {
            let site = build::build(&p, seed, 6);
            let plan = plan(&site, &p, seed).expect("a plan");
            for s in water::strikes(&site, &p, seed) {
                let deepest = plan.holes[s.hole].casing.last().unwrap().0;
                total += 1;
                passed += usize::from(deepest > s.depth);
            }
        }
        assert!(total > 600, "{total} strikes");
        let rate = passed as f64 / total as f64;
        assert!(
            (rate - 0.8).abs() < 0.05,
            "passes the strike in {rate:.3} of {total}"
        );
    }

    /// `(hole id, top, base, legend)` rows of BKFL.
    fn backfill_rows(m: &ProjectModel) -> Vec<(String, f64, f64, String)> {
        rows(m, "BKFL")
            .iter()
            .map(|r| (r[0].clone(), num(&r[1]), num(&r[2]), r[3].clone()))
            .collect()
    }

    #[test]
    fn backfill_tiles_the_hole_in_legend_codes_and_is_all_or_nothing_per_site() {
        let p = preset();
        let (mut with, mut without, mut both) = (0, 0, 0);
        for seed in 0..80 {
            let site = build::build(&p, seed, 6);
            let m = build::lower(&site, &p, seed, Grid::Local);
            let bkfl = backfill_rows(&m);
            if bkfl.is_empty() {
                without += 1;
                continue;
            }
            with += 1;
            for hole in &site.holes {
                let layers: Vec<_> = bkfl.iter().filter(|b| b.0 == hole.id).collect();
                assert!(!layers.is_empty(), "every hole carries it, seed {seed}");
                assert_eq!(layers[0].1, 0.0, "from the surface");
                assert_eq!(layers.last().unwrap().2, hole.final_depth, "to final depth");
                for w in layers.windows(2) {
                    assert_eq!(w[0].2, w[1].1, "layers tile, seed {seed}");
                    assert_ne!(w[0].3, w[1].3);
                }
                for l in &layers {
                    assert!(l.1 < l.2);
                    assert!(l.3 == ARISINGS || l.3 == BENTONITE, "{}", l.3);
                }
                if layers.len() == 2 {
                    both += 1;
                    assert_eq!(
                        (layers[0].3.as_str(), layers[1].3.as_str()),
                        (ARISINGS, BENTONITE)
                    );
                }
            }
        }
        assert!(with > 20 && without > 20, "{with} with, {without} without");
        assert!(both > 20, "{both} holes with arisings over bentonite");
    }

    #[test]
    fn the_sequence_weights_pick_the_sequence() {
        let only = |w: &str| {
            let p = Preset::from_toml(&with("weights = [1, 1, 1]", w).replace(
                "site_share = { min = 0.5, max = 0.5,",
                "site_share = { min = 1.0, max = 1.0,",
            ))
            .unwrap();
            let mut kinds = std::collections::BTreeSet::new();
            for seed in 0..10 {
                for b in backfill_rows(&model(&p, seed, 6)) {
                    kinds.insert(b.3);
                }
            }
            kinds
        };
        assert_eq!(only("weights = [1, 0, 0]"), [ARISINGS.to_string()].into());
        assert_eq!(only("weights = [0, 1, 0]"), [BENTONITE.to_string()].into());
        assert_eq!(only("weights = [0, 0, 1]").len(), 2);
    }

    #[test]
    fn hole_k_is_the_same_whatever_the_hole_count() {
        let p = preset();
        let construction = |m: &ProjectModel| {
            ["HDPH", "CDIA", "BKFL", "WSTG"]
                .iter()
                .map(|c| {
                    rows(m, c)
                        .into_iter()
                        .filter(|r| ["BH1", "BH2", "BH3"].contains(&r[0].as_str()))
                        .map(<[String]>::to_vec)
                        .collect::<Vec<_>>()
                })
                .collect::<Vec<_>>()
        };
        for seed in 0..20 {
            assert_eq!(
                construction(&model(&p, seed, 3)),
                construction(&model(&p, seed, 8)),
                "seed {seed}"
            );
        }
    }

    #[test]
    fn changing_the_block_moves_only_construction_rows() {
        let p = preset();
        let edits: [(&str, &str, &[&str]); 5] = [
            (
                "telescope_share = { min = 0.5, max = 0.5,",
                "telescope_share = { min = 0.9, max = 0.9,",
                &["CDIA"],
            ),
            (
                "sequence = { weights = [1, 1, 1],",
                "sequence = { weights = [5, 1, 1],",
                &["ABBR", "BKFL"],
            ),
            (
                "strike_gap_m = { min = 0.0, max = 3.0,",
                "strike_gap_m = { min = 0.0, max = 1.0,",
                &["WSTG"],
            ),
            (
                "diameters_basis = \"test fixture\"",
                "diameters_basis = \"another basis\"",
                &[],
            ),
            (
                "boundary_m = { min = 0.3, max = 1.5,",
                "boundary_m = { min = 0.5, max = 1.0,",
                &["BKFL"],
            ),
        ];
        let mut moved = 0;
        for (from, to, allowed) in edits {
            let q = Preset::from_toml(&with(from, to)).unwrap();
            for seed in 0..12 {
                let diff = differing(&model(&p, seed, 5), &model(&q, seed, 5));
                assert!(
                    diff.iter().all(|g| allowed.contains(&g.as_str())),
                    "{from:?}: {diff:?} moved"
                );
                moved += diff.len();
            }
        }
        assert!(moved > 0, "the edits changed nothing at all");
    }

    #[test]
    fn a_preset_without_the_block_has_none_of_its_groups() {
        let mut p = preset();
        p.construction = None;
        let m = model(&p, 2, 5);
        for code in ["HDPH", "CDIA", "BKFL"] {
            assert!(rows(&m, code).is_empty(), "{code}");
        }
        assert!(rows(&m, "WSTG").iter().all(|r| r.len() == 2));
        assert!(rows(&m, "LOCA").iter().all(|r| !r[1].contains('+')));
    }

    #[test]
    fn construction_files_are_clean_by_construction() {
        let p = preset();
        for seed in 0..12 {
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("site.ags");
            std::fs::write(&path, emit_to_string(&model(&p, seed, 6))).unwrap();
            assert_eq!(rust_check(&path).0, RustResult::Clean, "seed {seed}");
        }
    }

    /// An installation table for the main fixture: half the sites install,
    /// and on those 0.5 to 1.0 of the holes. The shortest final depth is
    /// 20 m, above the 8 m the longest zone and its surrounds need.
    const INSTALL: &str = r#"
[construction.installation]
site_share = { min = 0.5, max = 0.5, basis = "test fixture" }
hole_share = { min = 0.5, max = 1.0, basis = "test fixture" }
kinds = [
    { kind = "standpipe", mm = 50, weight = 3 },
    { kind = "piezometer", mm = 19, weight = 1 },
]
kinds_basis = "test fixture"
zone_offset_m = { min = -2.0, max = 2.0, basis = "test fixture" }
zone_top_m = { min = 1.0, max = 12.0, skew = 1.5, basis = "test fixture" }
slotted_length_m = { min = 0.5, max = 6.0, basis = "test fixture" }
base_gap_m = { min = 0.5, max = 8.0, basis = "test fixture" }
cap_m = { min = 0.5, max = 0.5, basis = "test fixture" }
filter = { weights = [3, 1], basis = "test fixture" }
"#;

    fn install_text() -> String {
        format!("{}{INSTALL}", text())
    }

    /// The installation text with one exact snippet replaced.
    fn install_with(from: &str, to: &str) -> String {
        assert!(INSTALL.contains(from), "install fixture lacks {from:?}");
        format!("{}{}", text(), INSTALL.replacen(from, to, 1))
    }

    fn install_preset() -> Preset {
        Preset::from_toml(&install_text()).expect("the installation fixture loads")
    }

    /// The groups an installation may change in a file.
    const INSTALL_GROUPS: [&str; 5] = ["ABBR", "BKFL", "PIPE", "TYPE", "UNIT"];

    #[test]
    fn the_loader_reads_the_installation_into_typed_values() {
        let p = install_preset();
        let i = p
            .construction
            .as_ref()
            .unwrap()
            .installation
            .as_ref()
            .expect("an installation table");
        assert_eq!((i.site_share.min, i.hole_share.min), (0.5, 0.5));
        let kinds: Vec<(InstallKindName, u32, f64)> =
            i.kinds.iter().map(|k| (k.kind, k.mm, k.weight)).collect();
        assert_eq!(
            kinds,
            [
                (InstallKindName::Standpipe, 50, 3.0),
                (InstallKindName::Piezometer, 19, 1.0)
            ]
        );
        assert_eq!(i.zone_top_m.skew, Some(1.5));
        assert_eq!(i.filter_weights, [3.0, 1.0]);
        assert!(preset().construction.unwrap().installation.is_none());
    }

    #[test]
    fn the_loader_refuses_an_installation_with_bad_values() {
        for (from, to, needle) in [
            (
                "site_share = { min = 0.5, max = 0.5,",
                "site_share = { min = 0.5, max = 1.5,",
                "construction.installation.site_share: max 1.5 is outside 0 to 1",
            ),
            (
                "hole_share = { min = 0.5, max = 1.0,",
                "hole_share = { min = -0.1, max = 1.0,",
                "construction.installation.hole_share: min -0.1 is outside 0 to 1",
            ),
            (
                "slotted_length_m = { min = 0.5,",
                "slotted_length_m = { min = 0.0,",
                "construction.installation.slotted_length_m: min 0 must be > 0",
            ),
            (
                "zone_top_m = { min = 1.0,",
                "zone_top_m = { min = -1.0,",
                "construction.installation.zone_top_m: min -1 must be > 0",
            ),
            (
                "cap_m = { min = 0.5,",
                "cap_m = { min = 0.0,",
                "construction.installation.cap_m: min 0 must be > 0",
            ),
            (
                "base_gap_m = { min = 0.5,",
                "base_gap_m = { min = -0.5,",
                "construction.installation.base_gap_m: min -0.5 is negative",
            ),
            (
                "{ kind = \"standpipe\", mm = 50, weight = 3 }",
                "{ kind = \"standpipe\", mm = 50, weight = 0 }",
                "construction.installation.kinds[0].weight: 0 must be > 0",
            ),
            (
                "{ kind = \"piezometer\", mm = 19, weight = 1 }",
                "{ kind = \"standpipe\", mm = 19, weight = 1 }",
                "construction.installation.kinds[1]: this kind is listed twice",
            ),
            (
                "{ kind = \"piezometer\", mm = 19, weight = 1 }",
                "{ kind = \"inclinometer\", mm = 19, weight = 1 }",
                "construction.installation.kinds[1].kind: \"inclinometer\" is not a kind",
            ),
            (
                "{ kind = \"piezometer\", mm = 19, weight = 1 }",
                "{ kind = \"piezometer\", mm = 25, weight = 1 }",
                "construction.installation.kinds[1].mm: 25 is not an installation diameter",
            ),
            (
                "filter = { weights = [3, 1],",
                "filter = { weights = [0, 0],",
                "construction.installation.filter.weights: weights sum to zero",
            ),
            (
                "slotted_length_m = { min = 0.5, max = 6.0,",
                "slotted_length_m = { min = 0.5, max = 19.0,",
                "construction.installation: the pit, cap, seal, longest slotted section",
            ),
        ] {
            refused(&install_with(from, to), needle);
        }
    }

    #[test]
    fn the_loader_refuses_a_missing_basis_unknown_key_and_missing_part_of_the_installation() {
        refused(
            &install_with("kinds_basis = \"test fixture\"\n", ""),
            "construction.installation.kinds_basis: missing",
        );
        refused(
            &install_with(
                "cap_m = { min = 0.5, max = 0.5, basis = \"test fixture\" }",
                "cap_m = { min = 0.5, max = 0.5 }",
            ),
            "construction.installation.cap_m: no basis",
        );
        refused(
            &install_with("kinds_basis =", "mystery = 1\nkinds_basis ="),
            "construction.installation: unknown key \"mystery\"",
        );
        refused(
            &install_with(
                "filter = { weights = [3, 1], basis = \"test fixture\" }",
                "",
            ),
            "construction.installation.filter: missing",
        );
        // The installation table cannot stand without the casing and backfill
        // parts of the block that holds it.
        let bare = format!(
            "{FIXTURE}{WATER}\n[construction]\ninspection_pit_m = {{ min = 1.0, max = 1.4, \
             basis = \"test fixture\" }}\n{INSTALL}"
        );
        refused(&bare, "construction.casing: missing");
        refused(&bare, "construction.backfill: missing");
    }

    /// `(id, ref, top, base, diameter, type)` rows of PIPE.
    type PipeRow = (String, String, f64, f64, String, String);

    fn pipe_rows(m: &ProjectModel) -> Vec<PipeRow> {
        rows(m, "PIPE")
            .iter()
            .map(|r| {
                (
                    r[0].clone(),
                    r[1].clone(),
                    num(&r[2]),
                    num(&r[3]),
                    r[4].clone(),
                    r[5].clone(),
                )
            })
            .collect()
    }

    /// Checks one installation's PIPE rows against the module doc's bounds and
    /// returns the slotted section.
    fn assert_installation_fits(
        pipes: &[&PipeRow],
        pit_base: f64,
        fd: f64,
        ctx: &str,
    ) -> (f64, f64) {
        assert_eq!(pipes.len(), 2, "plain then slotted, {ctx}");
        let (plain, slotted) = (pipes[0], pipes[1]);
        assert_eq!((plain.2, plain.5.as_str()), (0.0, "PLAIN"), "{ctx}");
        assert_eq!(
            (slotted.5.as_str(), slotted.2),
            ("SLOTTED", plain.3),
            "{ctx}"
        );
        assert_eq!(plain.1, "1");
        assert_eq!(slotted.1, "1");
        assert_eq!(plain.4, slotted.4, "one diameter, {ctx}");
        match plain.4.as_str() {
            "50" => assert!(slotted.3 - slotted.2 >= 0.5 - 1e-9, "{ctx}"),
            "19" => assert!((slotted.3 - slotted.2 - 0.3).abs() < 1e-9, "the tip, {ctx}"),
            d => panic!("diameter {d}, {ctx}"),
        }
        let (top, base) = (slotted.2, slotted.3);
        assert!(top >= pit_base - 1e-9, "below the pit, {ctx}");
        assert!(top >= 1.1 - 1e-9, "room for the cap and seal, {ctx}");
        assert!(base <= fd - 0.6 + 1e-9, "room for the tail, {ctx}");
        (top, base)
    }

    #[test]
    fn pipe_is_plain_then_slotted_from_the_surface_with_the_zone_inside_the_hole() {
        let p = install_preset();
        let (mut standpipes, mut piezometers, mut holes) = (0, 0, 0);
        for seed in 0..60 {
            let site = build::build(&p, seed, 8);
            let m = build::lower(&site, &p, seed, Grid::Local);
            let pipes = pipe_rows(&m);
            let hdph = rows(&m, "HDPH");
            for hole in &site.holes {
                let mine: Vec<_> = pipes.iter().filter(|r| r.0 == hole.id).collect();
                if mine.is_empty() {
                    continue;
                }
                holes += 1;
                let pit_base = hdph
                    .iter()
                    .find(|r| r[0] == hole.id)
                    .map(|r| num(&r[2]))
                    .unwrap();
                assert_installation_fits(
                    &mine,
                    pit_base,
                    hole.final_depth,
                    &format!("seed {seed} {}", hole.id),
                );
                match mine[0].4.as_str() {
                    "50" => standpipes += 1,
                    _ => piezometers += 1,
                }
            }
        }
        assert!(holes > 100, "{holes} installed holes");
        assert!(
            standpipes > 30 && piezometers > 10,
            "{standpipes}/{piezometers}"
        );
        // 3 : 1 by weight.
        let share = f64::from(standpipes) / f64::from(holes);
        assert!((0.65..0.85).contains(&share), "{share}");
    }

    #[test]
    fn an_installed_holes_backfill_tiles_the_hole_around_a_bracketed_filter() {
        let p = install_preset();
        let (mut gravel, mut sand, mut n) = (0, 0, 0);
        for seed in 0..60 {
            let site = build::build(&p, seed, 8);
            let m = build::lower(&site, &p, seed, Grid::Local);
            let (pipes, bkfl) = (pipe_rows(&m), backfill_rows(&m));
            for hole in &site.holes {
                let mine: Vec<_> = pipes.iter().filter(|r| r.0 == hole.id).collect();
                if mine.is_empty() {
                    continue;
                }
                n += 1;
                let (top, base) = (mine[1].2, mine[1].3);
                let layers: Vec<_> = bkfl.iter().filter(|b| b.0 == hole.id).collect();
                let ctx = format!("seed {seed} {}", hole.id);
                // Cap, seal, filter, tail: four layers, because the room
                // checked above leaves each some thickness.
                assert_eq!(layers.len(), 4, "{ctx}: {layers:?}");
                assert_eq!(
                    (layers[0].1, layers[0].2, layers[0].3.as_str()),
                    (0.0, 0.5, "906"),
                    "{ctx}"
                );
                assert_eq!(layers[1].3, BENTONITE, "{ctx}");
                assert!((layers[2].1 - (top - 0.5)).abs() < 1e-9, "{ctx}");
                assert!((layers[2].2 - (base + 0.5)).abs() < 1e-9, "{ctx}");
                match layers[2].3.as_str() {
                    "902" => gravel += 1,
                    "901" => sand += 1,
                    l => panic!("{ctx}: filter {l}"),
                }
                assert_eq!(layers[3].3, BENTONITE, "{ctx}");
                assert_eq!(layers[3].2, hole.final_depth, "to final depth, {ctx}");
                for w in layers.windows(2) {
                    assert_eq!(w[0].2, w[1].1, "tile, {ctx}");
                }
                assert!(layers.iter().all(|l| l.1 < l.2), "{ctx}");
            }
        }
        assert!(n > 100, "{n} installed holes");
        // 3 : 1 by weight.
        assert!(gravel > sand && sand > 5, "{gravel} gravel, {sand} sand");
    }

    #[test]
    fn an_installed_hole_always_has_backfill_and_the_others_keep_the_site_draw() {
        let p = install_preset();
        let mut q = p.clone();
        q.construction.as_mut().unwrap().installation = None;
        let (mut on_bare_site, mut kept) = (0, 0);
        for seed in 0..80 {
            let site = build::build(&p, seed, 8);
            let (with, without) = (
                build::lower(&site, &p, seed, Grid::Local),
                build::lower(&site, &q, seed, Grid::Local),
            );
            let installed: Vec<String> = pipe_rows(&with).into_iter().map(|r| r.0).collect();
            let (b_with, b_without) = (backfill_rows(&with), backfill_rows(&without));
            if b_without.is_empty() && !installed.is_empty() {
                on_bare_site += 1;
            }
            for hole in &site.holes {
                let mine: Vec<_> = b_with.iter().filter(|b| b.0 == hole.id).collect();
                let theirs: Vec<_> = b_without.iter().filter(|b| b.0 == hole.id).collect();
                if installed.contains(&hole.id) {
                    assert!(!mine.is_empty(), "seed {seed} {}", hole.id);
                } else {
                    assert_eq!(mine, theirs, "seed {seed} {}", hole.id);
                    kept += 1;
                }
            }
            // Everything but the installation's groups is what the site
            // without installations wrote, for every hole.
            let moved = differing(&with, &without);
            assert!(
                moved.iter().all(|g| INSTALL_GROUPS.contains(&g.as_str())),
                "seed {seed}: {moved:?}"
            );
        }
        assert!(
            on_bare_site > 5,
            "{on_bare_site} installed holes on a bare site"
        );
        assert!(kept > 100, "{kept}");
    }

    #[test]
    fn the_site_and_hole_draws_give_the_presets_install_share() {
        let p = install_preset();
        let (mut sites, mut installing, mut holes, mut installed) = (0, 0, 0, 0);
        for seed in 0..400 {
            let site = build::build(&p, seed, 8);
            let m = build::lower(&site, &p, seed, Grid::Local);
            let n = pipe_rows(&m).len() / 2;
            sites += 1;
            installing += usize::from(n > 0);
            if n > 0 {
                holes += 8;
                installed += n;
            }
        }
        // Half the sites install; those install 0.5 to 1.0 of 8 holes. A site
        // can draw no installed hole, so the observed share sits just under.
        let site_share = installing as f64 / f64::from(sites);
        assert!((0.38..0.58).contains(&site_share), "{site_share}");
        let hole_share = installed as f64 / f64::from(holes);
        assert!((0.65..0.85).contains(&hole_share), "{hole_share}");
    }

    #[test]
    fn a_zone_drawn_too_deep_or_too_shallow_is_clamped_into_the_hole() {
        for (from, to) in [
            (
                "zone_top_m = { min = 1.0, max = 12.0, skew = 1.5,",
                "zone_top_m = { min = 17.0, max = 19.0,",
            ),
            (
                "zone_top_m = { min = 1.0, max = 12.0, skew = 1.5,",
                "zone_top_m = { min = 0.1, max = 0.3,",
            ),
            (
                "zone_offset_m = { min = -2.0, max = 2.0,",
                "zone_offset_m = { min = 30.0, max = 40.0,",
            ),
            (
                "zone_offset_m = { min = -2.0, max = 2.0,",
                "zone_offset_m = { min = -40.0, max = -30.0,",
            ),
        ] {
            let p = Preset::from_toml(&install_with(from, to)).unwrap();
            let mut n = 0;
            for seed in 0..40 {
                let site = build::build(&p, seed, 8);
                let m = build::lower(&site, &p, seed, Grid::Local);
                let (pipes, bkfl) = (pipe_rows(&m), backfill_rows(&m));
                let hdph = rows(&m, "HDPH");
                for hole in &site.holes {
                    let mine: Vec<_> = pipes.iter().filter(|r| r.0 == hole.id).collect();
                    if mine.is_empty() {
                        continue;
                    }
                    n += 1;
                    let pit = hdph
                        .iter()
                        .find(|r| r[0] == hole.id)
                        .map(|r| num(&r[2]))
                        .unwrap();
                    let ctx = format!("{to} seed {seed} {}", hole.id);
                    assert_installation_fits(&mine, pit, hole.final_depth, &ctx);
                    let layers: Vec<_> = bkfl.iter().filter(|b| b.0 == hole.id).collect();
                    assert_eq!(layers.len(), 4, "{ctx}");
                    assert_eq!(layers.last().unwrap().2, hole.final_depth, "{ctx}");
                    assert!(layers.iter().all(|l| l.1 < l.2), "{ctx}");
                }
            }
            assert!(n > 20, "{to}: {n} installed holes");
        }
    }

    #[test]
    fn changing_the_installation_moves_only_installation_rows() {
        let p = install_preset();
        let edits: [(&str, &str, &[&str]); 5] = [
            (
                "filter = { weights = [3, 1],",
                "filter = { weights = [1, 5],",
                &["ABBR", "BKFL"],
            ),
            (
                "zone_top_m = { min = 1.0, max = 12.0, skew = 1.5,",
                "zone_top_m = { min = 2.0, max = 10.0, skew = 1.5,",
                &["BKFL", "PIPE"],
            ),
            (
                "{ kind = \"standpipe\", mm = 50, weight = 3 }",
                "{ kind = \"standpipe\", mm = 50, weight = 1 }",
                &["ABBR", "BKFL", "PIPE", "TYPE", "UNIT"],
            ),
            (
                "kinds_basis = \"test fixture\"",
                "kinds_basis = \"another\"",
                &[],
            ),
            (
                "cap_m = { min = 0.5, max = 0.5,",
                "cap_m = { min = 0.7, max = 0.7,",
                &["BKFL", "PIPE"],
            ),
        ];
        let mut moved = 0;
        for (from, to, allowed) in edits {
            let q = Preset::from_toml(&install_with(from, to)).unwrap();
            for seed in 0..12 {
                let diff = differing(&model(&p, seed, 6), &model(&q, seed, 6));
                assert!(
                    diff.iter().all(|g| allowed.contains(&g.as_str())),
                    "{from:?}: {diff:?} moved"
                );
                moved += diff.len();
            }
        }
        assert!(moved > 0, "the edits changed nothing at all");
    }

    #[test]
    fn hole_k_keeps_its_installation_whatever_the_hole_count() {
        let p = install_preset();
        let mine = |m: &ProjectModel| {
            ["PIPE", "BKFL", "HDPH", "CDIA", "WSTG"]
                .iter()
                .map(|c| {
                    rows(m, c)
                        .into_iter()
                        .filter(|r| ["BH1", "BH2", "BH3"].contains(&r[0].as_str()))
                        .map(<[String]>::to_vec)
                        .collect::<Vec<_>>()
                })
                .collect::<Vec<_>>()
        };
        let mut had_pipe = 0;
        for seed in 0..40 {
            let (small, large) = (model(&p, seed, 3), model(&p, seed, 8));
            had_pipe += usize::from(!rows(&small, "PIPE").is_empty());
            assert_eq!(mine(&small), mine(&large), "seed {seed}");
        }
        assert!(
            had_pipe > 5,
            "{had_pipe} seeds installed a hole in the prefix"
        );
    }

    #[test]
    fn a_preset_without_the_installation_table_has_no_pipe() {
        let m = model(&preset(), 3, 6);
        assert!(rows(&m, "PIPE").is_empty());
        assert!(rows(&m, "MONG").is_empty());
    }

    #[test]
    fn installation_files_are_clean_by_construction() {
        let p = install_preset();
        let mut with_pipe = 0;
        for seed in 0..30 {
            let m = model(&p, seed, 6);
            with_pipe += usize::from(!rows(&m, "PIPE").is_empty());
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("site.ags");
            std::fs::write(&path, emit_to_string(&m)).unwrap();
            assert_eq!(rust_check(&path).0, RustResult::Clean, "seed {seed}");
        }
        assert!(with_pipe > 5, "{with_pipe} files with PIPE");
    }
}
