//! Water strikes: `WSTG` and `WSTD` rows where a hole meets water (ADR 0001,
//! "Ground").
//!
//! Three rules give a hole its strikes (a made-ground unit takes the third, see
//! "Made-ground strikes"). By default a hole has at most one
//! (see "One strike per hole"): the shallowest of a lens-top (water-bearing)
//! strike and a till strike wins, and the table rule counts only when neither
//! occurs. With `site.several_strikes` a hole records every strike the rules
//! give (see "Several strikes").
//!
//! **The water table.** The table is the site's smooth field
//! (`groundwater_depth`). A hole strikes when the table falls inside one of its
//! coarse (permeable) strata; a table inside clay gives no strike, because clay
//! does not yield water at drilling speed. The site model has no clock, so a
//! strike has no date.
//!
//! - The strike depth is the water table plus the strike rise (the drilling
//!   lag: the bit is below the water when it is noticed). The rise is drawn
//!   from the preset's `site.strike_rise_m`, else from [`DEFAULT_RISE_M`], and
//!   clamped to the base of the stratum, so the strike is always in the coarse
//!   stratum and above final depth. The rise is one keyed draw per hole, which
//!   moves no other byte.
//! - `WSTD` records the water rising from the strike depth back to the table
//!   at 5, 10, 15 and 20 minutes ([`MINUTES`]). The depth to water follows a
//!   decay normalised to reach the table exactly at 20 minutes.
//!
//! **Water-bearing strata.** A unit or lens flagged `water_bearing` is a
//! stratum that yields water whether or not the table reaches it: thin
//! glaciofluvial sand and gravel lenses in till are "frequently water-bearing",
//! perched or sub-artesian (CIRIA C504 Fig 6.3, p.117; Eyles & Sladen 1981,
//! p.134), and a thin lens rarely contains the table.
//!
//! - A hole that meets a water-bearing stratum strikes at its top. The first
//!   one going down is the strike.
//! - The same keyed draw gives the rise `r` (it is not added to the strike, as
//!   the bit is already at the top of the water). Where the table is above the
//!   strike the water rises by `r` but not above the table: the readings settle
//!   at `max(table, strike - r)`.
//! - Where the table is at or below the strike the water is perched: it is
//!   held in the lens, with no head from the regional table to lift it, so the
//!   readings stay at the strike depth. The draw is made and unused, so a
//!   preset's draws do not depend on which case a hole falls in.
//!
//! **Till strikes.** A unit with a `strike` table (tyneside-till v8) yields
//! water along its depth, not at its top: the thin sand and silt partings and
//! fissures in a clay till are too small to log separately (owner's practice
//! note, `forge judgement`), and the BGS crawl of NE England has 129 of its
//! 437 strikes in till.
//!
//! - A hole is a Poisson process along its penetration of each such unit,
//!   with the unit's rate per metre (drawn once per site). The first event
//!   below the written top of the unit's stratum is the strike, so the strike
//!   is uniform in the stratum by construction. A unit a lens splits into two
//!   pieces continues one stream, so the pieces are one process. The stream is
//!   keyed by (hole, unit): changing another unit's rate moves none of it.
//! - The water always rises by a draw from the unit's `rise_m`, whatever the
//!   depth of the table, but not above ground level; a draw under the unit's
//!   `no_rise_share` (drawn once per site) leaves it at the strike. The head on
//!   a sand or silt parting in a clay till is not the site's water table, so
//!   the table bounds nothing (`forge judgement`). This differs from the lens
//!   rule above. The rise draws are keyed by (hole, unit).
//! - The first unit down that strikes is the hole's till strike.
//!
//! **Made-ground strikes** (london-basin v9, #84). A made-ground unit may take
//! the same `strike` block as a till, and the table rule still never strikes
//! it: the exclusion in the table rule is unchanged, so a made-ground unit
//! without a block never strikes. Three things differ from a till:
//!
//! - The rate may be a `rate_by_depth`: bands of depth below ground level,
//!   constant within a band, so the process stays Poisson. An event is placed
//!   by walking the bands in turn with one exponential draw of exposure
//!   ([`SiteRate`]). Each band draws its rate once per site from a stream keyed
//!   by (unit, band) (`strike-band-rate`), so changing one unit's bands moves
//!   no other stream. A flat `rate_per_m` draws and walks exactly as before.
//! - The water is perched, so an event at or below the hole's water table is
//!   dropped (for made ground only, never a till), and the gap rule drops a
//!   made-ground strike rather than push it to the table. The water in a
//!   made-ground strike is above its depth, which is above the table, so it
//!   stands above the water in any deeper strike that settles at or below the
//!   table. This "perched" is not the lens's: a perched lens strike stays at
//!   its strike depth, while made-ground water rises by its own draw (not
//!   above ground) from a strike that is above the table.
//! - The process runs from the top of the stratum, not its written top, and
//!   drops the events in the cover above the written top (counting them in
//!   the unit's stream), so a cover moves none of the strikes below it. A
//!   till's process starts at its written top, as before.
//!
//! **One strike per hole.** Without `site.several_strikes` a hole records the
//! shallowest of its lens-top strike (the first water-bearing stratum that
//! yields) and its till strike, the lens on a tie. A table strike in a coarse
//! stratum counts only when neither occurs, even if the table is shallower
//! (`forge judgement`; in the crawl the till strike is the hole's first in 74%
//! of holes that have one). This keeps the keyed draw count and the file shape
//! of the table-only rule.
//!
//! **Several strikes** (`site.several_strikes`, tyneside-till v9, #79). A hole
//! records every strike the rules give, shallowest first, up to
//! `max_per_hole`. There is no drawn count; the strikes come from the same
//! sources:
//!
//! - every water-bearing stratum that yields strikes once, at its top;
//! - every till's (and made ground's) Poisson process runs on past its first
//!   event, using draws
//!   the one-strike rule never makes, so the first event is where it was. A
//!   second event in the same unit draws its rise from a new key
//!   (`till-strike-rise-n`, by the event's index);
//! - the water table strikes a coarse stratum that has no strike of its own.
//!   With no lens strike in the hole it draws its rise from the key the
//!   one-strike rule uses.
//!
//! The strikes are sorted by depth. One closer than `min_gap_m` to the strike
//! above is pushed down to it, or dropped when its stratum has no room, and
//! strikes beyond `max_per_hole` are dropped. Each strike settles by its own
//! source's rule (the lens and table rule, or the till rule) and has its own
//! `WSTD` rows. A lens strike no longer gives way to a shallower till strike,
//! so the lens's `water_bearing` share is the share of holes with the lens
//! that strike in it. Construction (`WSTG_CAS`, `WSTG_SEAL`, the final casing
//! and the installation zone) follows each hole's strikes: see
//! `construction.rs`.
//!
//! The default rise, the logging minutes and the decay are `forge judgement`: the
//! ordinary UK 5-minute, 20-minute logging pattern, not a published curve.

use laterite_ags4_parity::Rng;
use laterite_ags4_validator::{DictVersion, Dictionary};

use super::build::Site;
use super::construction::Plan;
use super::draw::{Key, uniform};
use super::preset::{Preset, Range, SeveralStrikes, StrikeRate, UnitStrike};
use crate::synth::model::{Group, Row};

/// The default strike rise range, metres, used when the preset gives none:
/// how far below the water table the bit is when the water is noticed
/// (`forge judgement`).
const DEFAULT_RISE_M: (f64, f64) = (0.0, 0.5);

/// The minutes after the strike at which the rise is logged (`forge
/// judgement`).
pub(super) const MINUTES: [u32; 4] = [5, 10, 15, 20];

/// The decay rate of the rise, per minute (`forge judgement`).
const DECAY_PER_MIN: f64 = 0.15;

/// One strike: the hole it is in, where water was met, and where it settles.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct Strike {
    pub hole: usize,
    /// Depth water was met, metres, 2 decimal places.
    pub depth: f64,
    /// The depth the readings settle at after 20 minutes, metres, 2 decimal
    /// places: the water table, or for a water-bearing stratum the higher of
    /// the table and the strike less the rise (the strike depth when perched).
    pub settle: f64,
}

impl Strike {
    /// The depth to water `minutes` after the strike. Equal to the strike
    /// depth at 0 and the table at 20, rising in between.
    pub(super) fn reading(&self, minutes: u32) -> f64 {
        let end = MINUTES[MINUTES.len() - 1];
        let decay = |m: u32| (-DECAY_PER_MIN * f64::from(m)).exp();
        let left = (decay(minutes) - decay(end)) / (1.0 - decay(end));
        round2(self.settle + (self.depth - self.settle) * left)
    }
}

/// The strikes of every hole in file order, shallowest first within a hole:
/// at most one per hole, or with `site.several_strikes` up to its cap.
pub(super) fn strikes(site: &Site, preset: &Preset, seed: u64) -> Vec<Strike> {
    let (rise_min, rise_max) = preset
        .site
        .strike_rise_m
        .as_ref()
        .map_or(DEFAULT_RISE_M, |r| (r.min, r.max));
    if let Some(several) = &preset.site.several_strikes {
        return several_strikes(site, preset, seed, several, (rise_min, rise_max));
    }
    let shares = site_shares(preset, seed);
    let rates = till_rates(preset, seed);
    let mut out = Vec::new();
    for (hi, hole) in site.holes.iter().enumerate() {
        let table = round2(site.properties.groundwater_depth(hole));
        // Nothing is struck in the cover: a table above its base (which the
        // loader rules out, since the cover is thinner than the shallowest
        // table) would strike nothing.
        let cover_base = hole.cover.as_ref().map_or(0.0, |c| c.base);
        let in_table = hole
            .strata
            .iter()
            .find(|s| table >= s.top && table < s.base)
            .filter(|_| table >= cover_base)
            // The table rule never strikes Made Ground, whatever its
            // properties (niko86/laterite-forge#42, M2); a made-ground unit's
            // own perched strikes come from its `strike` block.
            .filter(|s| !preset.is_made_ground(&s.unit) && site.properties.is_coarse(&s.unit));
        // The first water-bearing stratum down the hole that yields water,
        // struck at its written top (below the cover).
        let bearing = hole
            .strata
            .iter()
            .enumerate()
            .find(|(_, s)| yields(&shares, seed, hi, &s.unit))
            .map(|(i, _)| hole.written_top(i));
        let by_till = till_strike(site, hi, &rates, seed);
        if in_table.is_none() && bearing.is_none() && by_till.is_none() {
            continue;
        }
        // The table-rule and lens-top rise: one keyed draw per hole, made
        // only where one of those rules can strike, as before till strikes.
        let rise = if in_table.is_some() || bearing.is_some() {
            uniform(&mut Key::hole("strike", hi).rng(seed), rise_min, rise_max)
        } else {
            0.0
        };
        let by_table = in_table.map(|s| Strike {
            hole: hi,
            depth: round2(table + rise).min(s.base),
            settle: table,
        });
        let by_stratum = bearing.map(|top| Strike {
            hole: hi,
            depth: top,
            // Perched water has no head above the strike to rise into.
            settle: if table < top {
                table.max(round2(top - rise))
            } else {
                top
            },
        });
        let by_till = by_till.map(|t| t.strike(hi, seed, 0));
        // The shallowest of a lens-top strike and a till strike wins, the lens
        // on a tie; a table strike counts only when neither occurs.
        let first = match (by_stratum, by_till) {
            (Some(a), Some(b)) => Some(if b.depth < a.depth { b } else { a }),
            (a, b) => a.or(b),
        };
        out.extend(first.or(by_table));
    }
    out
}

/// Where a candidate strike came from, with what its settled level needs.
enum Source<'a> {
    /// The water table inside a coarse stratum.
    Table,
    /// The top of a water-bearing stratum; `rise` is its drawn rise.
    Lens { rise: f64 },
    /// A Poisson event in a till or made-ground unit; `n` is its index in the
    /// unit's stream.
    Till { ts: TillStrike<'a>, n: u64 },
}

/// A strike before the gap rule: its depth, the deepest it may be pushed to
/// (just above its stratum's base) and its source.
struct Candidate<'a> {
    depth: f64,
    limit: f64,
    source: Source<'a>,
}

/// The strikes of every hole when the preset allows several. The sources are
/// the single-strike rule's, each yielding all its strikes: every
/// water-bearing stratum that yields strikes once at its top, every till's
/// Poisson process runs on past its first event, and the water table strikes
/// a coarse stratum that has no strike of its own. They are sorted by depth;
/// one closer than `min_gap_m` to the strike above is pushed down to it, or
/// dropped when its stratum has no room; the deepest beyond `max_per_hole`
/// are dropped. Each strike settles by its own source's rule, from its own
/// keyed draws.
fn several_strikes(
    site: &Site,
    preset: &Preset,
    seed: u64,
    rule: &SeveralStrikes,
    (rise_min, rise_max): (f64, f64),
) -> Vec<Strike> {
    let shares = site_shares(preset, seed);
    let rates = till_rates(preset, seed);
    let max = rule.max_per_hole as usize;
    let mut out = Vec::new();
    for (hi, hole) in site.holes.iter().enumerate() {
        let table = round2(site.properties.groundwater_depth(hole));
        let cover_base = hole.cover.as_ref().map_or(0.0, |c| c.base);
        let limit = |base: f64| round2(base - 0.01);
        let mut cands: Vec<Candidate> = Vec::new();
        let mut bearing_in_table = false;
        let mut lenses = 0_u64;
        for (i, s) in hole.strata.iter().enumerate() {
            if !yields(&shares, seed, hi, &s.unit) {
                continue;
            }
            // The first lens strike draws from the key a single-strike hole
            // uses; each later one has its own.
            let key = if lenses == 0 {
                Key::hole("strike", hi)
            } else {
                Key {
                    test: lenses,
                    ..Key::hole("strike", hi)
                }
            };
            lenses += 1;
            let rise = uniform(&mut key.rng(seed), rise_min, rise_max);
            bearing_in_table |= table >= s.top && table < s.base;
            cands.push(Candidate {
                depth: hole.written_top(i),
                limit: limit(s.base),
                source: Source::Lens { rise },
            });
        }
        // The table strikes a coarse stratum only where that stratum has no
        // strike of its own.
        if !bearing_in_table
            && table >= cover_base
            && let Some(s) = hole
                .strata
                .iter()
                .find(|s| table >= s.top && table < s.base)
                .filter(|s| !preset.is_made_ground(&s.unit) && site.properties.is_coarse(&s.unit))
        {
            // A table strike with no lens strike in the hole draws its rise
            // from the key a single-strike hole uses; beside a lens strike it
            // has its own.
            let key = if lenses == 0 {
                Key::hole("strike", hi)
            } else {
                Key::hole("strike-table", hi)
            };
            let rise = uniform(&mut key.rng(seed), rise_min, rise_max);
            cands.push(Candidate {
                depth: round2(table + rise).min(s.base),
                limit: limit(s.base),
                source: Source::Table,
            });
        }
        for e in till_events(site, hi, &rates, seed, max, false) {
            // A made-ground strike is perched, so the gap rule may not push it
            // to the water table or below: it is dropped instead.
            let base = if preset.is_made_ground(e.ts.unit) {
                e.base.min(table)
            } else {
                e.base
            };
            cands.push(Candidate {
                depth: e.ts.depth,
                limit: limit(base),
                source: Source::Till { ts: e.ts, n: e.n },
            });
        }
        cands.sort_by(|a, b| a.depth.total_cmp(&b.depth));
        let mut kept = 0;
        let mut above: Option<f64> = None;
        for c in cands {
            if kept >= max {
                break;
            }
            let mut depth = c.depth;
            if let Some(a) = above {
                depth = depth.max(round2(a + rule.min_gap_m));
                if depth > c.limit.max(c.depth) {
                    continue;
                }
            }
            above = Some(depth);
            kept += 1;
            out.push(match c.source {
                Source::Table => Strike {
                    hole: hi,
                    depth,
                    settle: table,
                },
                Source::Lens { rise } => Strike {
                    hole: hi,
                    depth,
                    settle: if table < depth {
                        table.max(round2(depth - rise))
                    } else {
                        depth
                    },
                },
                Source::Till { ts, n } => TillStrike { depth, ..ts }.strike(hi, seed, n),
            });
        }
    }
    out
}

/// A strike unit's rate on this site, with its spec. A till or, from #84, a
/// made-ground unit: the till names also carry made ground.
struct TillRate<'a> {
    unit: &'a str,
    spec: &'a UnitStrike,
    /// Strikes per metre penetrated, drawn once per site.
    rate: SiteRate,
    /// The share of strikes whose water does not rise, drawn once per site.
    no_rise: f64,
    /// Whether the unit is made ground, whose strikes are perched: an event at
    /// or below the hole's water table is dropped.
    made_ground: bool,
}

/// A unit's strike rate once drawn for the site.
enum SiteRate {
    Flat(f64),
    /// `(from_m, rate)` per band, ascending, the first from 0.
    Banded(Vec<(f64, f64)>),
}

impl SiteRate {
    /// Whether no depth has a rate above zero.
    fn is_dead(&self) -> bool {
        match self {
            Self::Flat(r) => *r <= 0.0,
            Self::Banded(b) => b.iter().all(|&(_, r)| r <= 0.0),
        }
    }

    /// Where a banded process that has gathered `e` units of exposure (an
    /// exponential draw) from `depth` down has its next event: constant rate
    /// within each band, so the bands are walked in turn. Infinite when the
    /// rest of the hole has no event.
    fn banded_advance(bands: &[(f64, f64)], depth: f64, mut e: f64) -> f64 {
        let mut d = depth;
        for (i, &(from, rate)) in bands.iter().enumerate() {
            let end = bands.get(i + 1).map_or(f64::INFINITY, |next| next.0);
            if d >= end {
                continue;
            }
            let start = d.max(from);
            if rate > 0.0 {
                let room = rate * (end - start);
                if e <= room {
                    return start + e / rate;
                }
                e -= room;
            }
            d = end;
        }
        f64::INFINITY
    }
}

/// The site's rate for each unit with a `strike` table (a till or made
/// ground), drawn once in the
/// unit's range from a stream keyed by its id (a banded rate draws each band
/// from its own stream, keyed by the unit and the band).
fn till_rates(preset: &Preset, seed: u64) -> Vec<TillRate<'_>> {
    preset
        .units
        .iter()
        .filter_map(|u| {
            let spec = u.strike.as_ref()?;
            let n = &spec.no_rise_share;
            let rate = match &spec.rate {
                StrikeRate::PerMetre(r) => SiteRate::Flat(uniform(
                    &mut Key::unit("till-strike-rate", &u.id).rng(seed),
                    r.min,
                    r.max,
                )),
                StrikeRate::ByDepth(bands) => SiteRate::Banded(
                    bands
                        .iter()
                        .enumerate()
                        .map(|(i, b)| {
                            let key = Key {
                                test: i as u64,
                                ..Key::unit("strike-band-rate", &u.id)
                            };
                            let r = &b.rate_per_m;
                            (b.from_m, uniform(&mut key.rng(seed), r.min, r.max))
                        })
                        .collect(),
                ),
            };
            Some(TillRate {
                unit: &u.id,
                spec,
                rate,
                no_rise: uniform(
                    &mut Key::unit("till-strike-norise", &u.id).rng(seed),
                    n.min,
                    n.max,
                ),
                made_ground: preset.is_made_ground(&u.id),
            })
        })
        .collect()
}

/// A strike found along a till unit's penetration, before its rise.
struct TillStrike<'a> {
    unit: &'a str,
    spec: &'a UnitStrike,
    no_rise: f64,
    depth: f64,
}

impl TillStrike<'_> {
    /// The strike with its settled level. The water always rises by the drawn
    /// rise, whatever the table's depth, but not above ground level. A
    /// no-rise draw (the unit's share of strikes) leaves the water at the
    /// strike. The head on a parting in a clay till is not the site's water
    /// table, so the table bounds nothing.
    fn strike(&self, hole: usize, seed: u64, n: u64) -> Strike {
        // The unit's first event has always drawn from this key; a later one
        // in the same unit has its own.
        let key = if n == 0 {
            Key::stratum("till-strike-rise", hole, self.unit)
        } else {
            Key {
                test: n,
                ..Key::stratum("till-strike-rise-n", hole, self.unit)
            }
        };
        let mut rng = key.rng(seed);
        let rise = skewed(&self.spec.rise_m, uniform(&mut rng, 0.0, 1.0));
        let none = uniform(&mut rng, 0.0, 1.0) < self.no_rise;
        let rise = if none { 0.0 } else { rise };
        Strike {
            hole,
            depth: self.depth,
            settle: round2(self.depth - rise).max(0.0),
        }
    }
}

/// `min + (max - min) * u^skew`.
fn skewed(r: &Range, u: f64) -> f64 {
    r.min + (r.max - r.min) * u.powf(r.skew.unwrap_or(1.0))
}

/// The hole's first till strike going down, if any. Each unit with a `strike`
/// table is a Poisson process along the hole's penetration of it: the first
/// event below the written top of its stratum is the strike, which is
/// uniform in the stratum by construction. A unit met in several pieces (split
/// by a lens) continues one stream, so the pieces are one process. The
/// stream is keyed by (hole, unit), so changing another unit's rate moves no
/// draw here.
fn till_strike<'a>(
    site: &Site,
    hole: usize,
    rates: &[TillRate<'a>],
    seed: u64,
) -> Option<TillStrike<'a>> {
    till_events(site, hole, rates, seed, 1, true)
        .into_iter()
        .next()
        .map(|e| e.ts)
}

/// A Poisson event along a till unit, with the base of its stratum and its
/// index in the unit's stream.
struct TillEvent<'a> {
    ts: TillStrike<'a>,
    base: f64,
    n: u64,
}

/// The hole's till and made-ground events going down. With `first_only` the walk stops at the
/// first event of any unit, which is [`till_strike`]. Otherwise each unit's
/// stream runs on past its first event, up to `per_unit` events: the draws
/// after the first are ones the first-only walk never makes. Events are in
/// the order the strata meet them.
fn till_events<'a>(
    site: &Site,
    hole: usize,
    rates: &[TillRate<'a>],
    seed: u64,
    per_unit: usize,
    first_only: bool,
) -> Vec<TillEvent<'a>> {
    let h = &site.holes[hole];
    // Per unit: its stream and how many events it has had.
    let mut streams: Vec<(&str, Rng, usize, u64)> = Vec::new();
    let mut out = Vec::new();
    for (i, s) in h.strata.iter().enumerate() {
        let Some(t) = rates.iter().find(|t| t.unit == s.unit) else {
            continue;
        };
        let at = if let Some(at) = streams.iter().position(|(u, _, _, _)| *u == s.unit) {
            at
        } else {
            streams.push((
                t.unit,
                Key::stratum("till-strike", hole, t.unit).rng(seed),
                0,
                0,
            ));
            streams.len() - 1
        };
        // One draw per piece of the unit, hit or not.
        let mut u = uniform(&mut streams[at].1, 0.0, 1.0);
        let written = h.written_top(i);
        if t.rate.is_dead() || s.base - written < 0.02 {
            continue;
        }
        // A made-ground unit's process runs from the stratum's own top, and
        // the events in the cover above its written top are dropped, so a
        // cover moves none of the strikes below it. A till's process starts
        // at its written top.
        let top = if t.made_ground { s.top } else { written };
        // A made-ground strike is perched, so it is above the hole's water
        // table; an event at or below it is dropped.
        let table = t
            .made_ground
            .then(|| round2(site.properties.groundwater_depth(h)));
        // Distance down from the stratum's top to the latest event.
        let mut off = 0.0;
        loop {
            let exposure = -(1.0 - u).ln();
            off = match &t.rate {
                SiteRate::Flat(r) => off + exposure / r,
                SiteRate::Banded(bands) => {
                    SiteRate::banded_advance(bands, top + off, exposure) - top
                }
            };
            if off >= s.base - top {
                break;
            }
            let depth = round2(top + off).clamp(round2(top + 0.01), round2(s.base - 0.01));
            if table.is_some_and(|w| depth >= w) {
                break;
            }
            // The event's index in the unit's stream, covered or not, keys its
            // rise; only recorded events count toward the cap.
            let n = streams[at].3;
            streams[at].3 += 1;
            if depth < written {
                // In the cover: not recorded and not counted toward the cap.
                u = uniform(&mut streams[at].1, 0.0, 1.0);
                continue;
            }
            streams[at].2 += 1;
            out.push(TillEvent {
                ts: TillStrike {
                    unit: t.unit,
                    spec: t.spec,
                    no_rise: t.no_rise,
                    depth,
                },
                base: s.base,
                n,
            });
            if first_only {
                return out;
            }
            if streams[at].2 >= per_unit {
                break;
            }
            u = uniform(&mut streams[at].1, 0.0, 1.0);
        }
    }
    out
}

/// The strike share of each water-bearing unit or lens on this site, drawn
/// once in the unit's range from a stream keyed by its id.
fn site_shares(preset: &Preset, seed: u64) -> Vec<(&str, f64)> {
    let ids = preset
        .units
        .iter()
        .map(|u| u.id.as_str())
        .chain(preset.lenses.iter().map(|l| l.id.as_str()));
    ids.filter_map(|id| {
        let r = preset.water_share(id)?;
        let share = uniform(&mut Key::unit("water-share", id).rng(seed), r.min, r.max);
        Some((id, share))
    })
    .collect()
}

/// Whether a hole's stratum of `unit` is water-bearing and yields water. A
/// share of 1 makes no draw, so `water_bearing = true` moves nothing.
fn yields(shares: &[(&str, f64)], seed: u64, hole: usize, unit: &str) -> bool {
    let Some(&(_, share)) = shares.iter().find(|(id, _)| *id == unit) else {
        return false;
    };
    share >= 1.0
        || uniform(
            &mut Key::stratum("water-bearing", hole, unit).rng(seed),
            0.0,
            1.0,
        ) < share
}

/// The `WSTG` group, or `None` when no hole strikes water. With hole
/// construction each row also carries `WSTG_CAS`, the casing depth at the
/// strike.
pub(super) fn wstg(site: &Site, strikes: &[Strike], construction: Option<&Plan>) -> Option<Group> {
    let rows = strikes
        .iter()
        .enumerate()
        .map(|(at, s)| {
            let mut v = vec![site.holes[s.hole].id.clone(), m2(s.depth)];
            if let Some(plan) = construction {
                // This strike's place among its hole's strikes.
                let k = strikes[..at].iter().filter(|p| p.hole == s.hole).count();
                let hole = &plan.holes[s.hole];
                // The dictionary lists WSTG_SEAL before WSTG_CAS.
                if plan.has_seal {
                    v.push(hole.strike_seal[k].map_or_else(String::new, m2));
                }
                v.push(m2(hole.strike_casing[k]));
            }
            Row::owned(v)
        })
        .collect();
    let headings: &[&str] = match construction {
        Some(plan) if plan.has_seal => &["LOCA_ID", "WSTG_DPTH", "WSTG_SEAL", "WSTG_CAS"],
        Some(_) => &["LOCA_ID", "WSTG_DPTH", "WSTG_CAS"],
        None => &["LOCA_ID", "WSTG_DPTH"],
    };
    build("WSTG", headings, strikes, rows)
}

/// The `WSTD` group: four readings per strike.
pub(super) fn wstd(site: &Site, strikes: &[Strike]) -> Option<Group> {
    let rows = strikes
        .iter()
        .flat_map(|s| {
            MINUTES.iter().map(move |&m| {
                Row::owned(vec![
                    site.holes[s.hole].id.clone(),
                    m2(s.depth),
                    m.to_string(),
                    m2(s.reading(m)),
                ])
            })
        })
        .collect();
    build(
        "WSTD",
        &["LOCA_ID", "WSTG_DPTH", "WSTD_NMIN", "WSTD_POST"],
        strikes,
        rows,
    )
}

/// `code` over `headings`, with the units and types from the dictionary.
fn build(code: &str, headings: &[&str], strikes: &[Strike], rows: Vec<Row>) -> Option<Group> {
    if strikes.is_empty() {
        return None;
    }
    let dict = Dictionary::bundled(DictVersion::V4_2);
    let entry = |h: &str| dict.heading(code, h).expect("heading exists");
    Some(Group {
        code: code.to_string(),
        headings: headings.iter().map(ToString::to_string).collect(),
        units: headings.iter().map(|h| entry(h).unit.to_string()).collect(),
        types: headings
            .iter()
            .map(|h| entry(h).ags_type.to_string())
            .collect(),
        rows,
    })
}

fn m2(v: f64) -> String {
    format!("{v:.2}")
}

fn round2(v: f64) -> f64 {
    (v * 100.0).round() / 100.0
}

#[cfg(test)]
mod tests {
    use super::super::build::{self, Grid, tests::FIXTURE};
    use super::super::preset::Preset;
    use super::*;
    use crate::pipeline::rust_check;
    use crate::synth::emit::emit_to_string;
    use crate::synth::model::ProjectModel;
    use laterite_ags4_parity::RustResult;

    const WATER: &str = r#"
[[suite]]
group = "WSTG"
headings = []

[[suite]]
group = "WSTD"
headings = []
"#;

    /// [`FIXTURE`] with the water groups: RTD is sand and gravel, WLC and LC
    /// are clay, and the table sits 1-4 m down, so some holes strike in RTD
    /// and some meet the table in clay.
    fn wet() -> Preset {
        Preset::from_toml(&format!("{FIXTURE}{WATER}")).expect("water fixture loads")
    }

    /// The same ground with the table below every hole.
    fn dry_with(water: &str) -> Preset {
        let text = format!("{FIXTURE}{water}").replace(
            "groundwater_depth_m = { min = 1.0, max = 4.0,",
            "groundwater_depth_m = { min = 60.0, max = 60.0,",
        );
        Preset::from_toml(&text).expect("dry fixture loads")
    }

    fn file(preset: &Preset, seed: u64, holes: usize) -> ProjectModel {
        let site = build::build(preset, seed, holes);
        build::lower(&site, preset, seed, Grid::Local)
    }

    fn rows<'a>(m: &'a ProjectModel, code: &str) -> &'a [Row] {
        m.groups
            .iter()
            .find(|g| g.code == code)
            .map_or(&[], |g| &g.rows)
    }

    fn text(m: &ProjectModel, code: &str) -> Vec<Vec<String>> {
        rows(m, code).iter().map(|r| r.values.clone()).collect()
    }

    fn is_clean(m: &ProjectModel) -> bool {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("site.ags");
        std::fs::write(&path, emit_to_string(m)).unwrap();
        rust_check(&path).0 == RustResult::Clean
    }

    #[test]
    fn a_strike_is_in_a_coarse_stratum_within_the_rise_of_the_table_and_above_final_depth() {
        let p = wet();
        let mut seen = 0;
        for seed in 0..60 {
            let site = build::build(&p, seed, 6);
            for s in strikes(&site, &p, seed) {
                seen += 1;
                let hole = &site.holes[s.hole];
                let table = round2(site.properties.groundwater_depth(hole));
                assert!((s.settle - table).abs() < 1e-9);
                assert!(s.depth >= table, "strike above the table, seed {seed}");
                assert!(
                    s.depth <= table + DEFAULT_RISE_M.1 + 1e-9,
                    "rise over {} m, seed {seed}",
                    DEFAULT_RISE_M.1
                );
                let st = hole
                    .strata
                    .iter()
                    .find(|st| table >= st.top && table < st.base)
                    .expect("the table is in a stratum");
                assert!(site.properties.is_coarse(&st.unit), "{}", st.unit);
                assert_eq!(st.unit, "RTD");
                assert!(s.depth <= st.base, "strike below its stratum, seed {seed}");
                assert!(s.depth <= hole.final_depth);
            }
        }
        assert!(seen > 30, "{seen} strikes is too few to mean anything");
    }

    #[test]
    fn the_rise_varies_across_strikes() {
        let p = wet();
        let mut rises = std::collections::BTreeSet::new();
        for seed in 0..60 {
            let site = build::build(&p, seed, 6);
            for s in strikes(&site, &p, seed) {
                rises.insert(format!("{:.2}", s.depth - s.settle));
            }
        }
        assert!(rises.len() > 10, "{} distinct rises", rises.len());
    }

    /// [`WATER`] over [`FIXTURE`] with a preset strike rise of 1-2 m.
    fn wet_with_rise() -> Preset {
        let text = format!("{FIXTURE}{WATER}").replace(
            "groundwater_depth_m = { min = 1.0, max = 4.0,",
            "strike_rise_m = { min = 1.0, max = 2.0, basis = \"test fixture\" }\n\
             groundwater_depth_m = { min = 1.0, max = 4.0,",
        );
        Preset::from_toml(&text).expect("rise fixture loads")
    }

    #[test]
    fn a_preset_strike_rise_sets_the_depth_below_the_table() {
        let p = wet_with_rise();
        let (mut seen, mut clamped) = (0, 0);
        for seed in 0..60 {
            let site = build::build(&p, seed, 6);
            for s in strikes(&site, &p, seed) {
                seen += 1;
                let hole = &site.holes[s.hole];
                let st = hole
                    .strata
                    .iter()
                    .find(|st| s.settle >= st.top && s.settle < st.base)
                    .expect("the table is in a stratum");
                let rise = s.depth - s.settle;
                if (s.depth - st.base).abs() < 1e-9 {
                    clamped += 1;
                    assert!(rise <= 2.0 + 1e-9, "seed {seed}");
                } else {
                    assert!((1.0 - 1e-9..=2.0 + 1e-9).contains(&rise), "{rise}");
                }
            }
        }
        assert!(seen > 30, "{seen} strikes is too few to mean anything");
        assert!(seen > clamped, "every strike clamped");
    }

    #[test]
    fn a_preset_without_a_strike_rise_draws_the_default_range_from_the_same_key() {
        let p = wet();
        assert!(p.site.strike_rise_m.is_none());
        let explicit = Preset::from_toml(&format!("{FIXTURE}{WATER}").replace(
            "groundwater_depth_m = { min = 1.0, max = 4.0,",
            "strike_rise_m = { min = 0.0, max = 0.5, basis = \"test fixture\" }\n\
             groundwater_depth_m = { min = 1.0, max = 4.0,",
        ))
        .unwrap();
        for seed in 0..20 {
            assert_eq!(
                emit_to_string(&file(&p, seed, 6)),
                emit_to_string(&file(&explicit, seed, 6)),
                "seed {seed}"
            );
        }
    }

    #[test]
    fn no_strike_where_the_table_is_in_a_fine_stratum() {
        let p = wet();
        let (mut fine, mut coarse) = (0, 0);
        for seed in 0..60 {
            let site = build::build(&p, seed, 6);
            let found = strikes(&site, &p, seed);
            for (hi, hole) in site.holes.iter().enumerate() {
                let table = round2(site.properties.groundwater_depth(hole));
                let st = hole
                    .strata
                    .iter()
                    .find(|st| table >= st.top && table < st.base);
                let struck = found.iter().any(|s| s.hole == hi);
                match st {
                    Some(st) if site.properties.is_coarse(&st.unit) => {
                        coarse += 1;
                        assert!(struck, "no strike in {} at {table}, seed {seed}", st.unit);
                    }
                    _ => {
                        fine += 1;
                        assert!(!struck, "strike in fine ground, seed {seed}");
                    }
                }
            }
            assert_eq!(text(&file(&p, seed, 6), "WSTG").len(), found.len());
        }
        assert!(fine > 20 && coarse > 20, "fine {fine}, coarse {coarse}");
    }

    #[test]
    fn readings_rise_monotonically_to_the_table_at_twenty_minutes() {
        let p = wet();
        let mut checked = 0;
        for seed in 0..40 {
            let site = build::build(&p, seed, 6);
            let m = build::lower(&site, &p, seed, Grid::Local);
            let (g, d) = (text(&m, "WSTG"), text(&m, "WSTD"));
            assert_eq!(d.len(), 4 * g.len(), "seed {seed}");
            for (i, strike) in g.iter().enumerate() {
                let block = &d[4 * i..4 * i + 4];
                let minutes: Vec<&str> = block.iter().map(|r| r[2].as_str()).collect();
                assert_eq!(minutes, ["5", "10", "15", "20"]);
                for r in block {
                    assert_eq!(r[0], strike[0]);
                    assert_eq!(r[1], strike[1]);
                }
                let depth: f64 = strike[1].parse().unwrap();
                let posts: Vec<f64> = block.iter().map(|r| r[3].parse().unwrap()).collect();
                assert!(posts[0] <= depth, "first reading below the strike");
                assert!(posts.windows(2).all(|w| w[1] <= w[0]), "{posts:?}");
                let hole = site.holes.iter().find(|h| h.id == strike[0]).unwrap();
                let table = round2(site.properties.groundwater_depth(hole));
                assert!((posts[3] - table).abs() < 1e-9, "seed {seed}");
                checked += 1;
            }
        }
        assert!(checked > 30);
    }

    #[test]
    fn a_reading_decays_from_the_strike_depth_to_the_table() {
        let s = Strike {
            hole: 0,
            depth: 2.0,
            settle: 2.0,
        };
        assert!(MINUTES.iter().all(|&m| (s.reading(m) - 2.0).abs() < 1e-9));
        let s = Strike { depth: 2.4, ..s };
        assert!((s.reading(0) - 2.4).abs() < 1e-9);
        assert!((s.reading(20) - 2.0).abs() < 1e-9);
        assert!(s.reading(5) < 2.4 && s.reading(5) > 2.0);
    }

    #[test]
    fn a_water_file_is_rust_clean_across_seeds() {
        let p = wet();
        let mut with_strikes = 0;
        for seed in 0..30 {
            let m = file(&p, seed, 6);
            assert!(is_clean(&m), "seed {seed}");
            with_strikes += usize::from(!rows(&m, "WSTG").is_empty());
        }
        assert!(with_strikes > 5, "only {with_strikes} files strike water");
    }

    #[test]
    fn strikes_move_no_loca_geol_or_samp_byte() {
        let plain = Preset::from_toml(FIXTURE).unwrap();
        let p = wet();
        for seed in 0..10 {
            let (with, without) = (file(&p, seed, 5), file(&plain, seed, 5));
            for g in ["TRAN", "LOCA", "GEOL", "SAMP"] {
                assert_eq!(text(&with, g), text(&without, g), "{g} at seed {seed}");
            }
        }
    }

    #[test]
    fn a_site_that_strikes_no_water_is_byte_identical_to_one_without_the_groups() {
        let (with, without) = (dry_with(WATER), dry_with(""));
        for seed in 0..10 {
            assert!(strikes(&build::build(&with, seed, 6), &with, seed).is_empty());
            assert_eq!(
                emit_to_string(&file(&with, seed, 6)),
                emit_to_string(&file(&without, seed, 6)),
                "seed {seed}"
            );
        }
    }

    #[test]
    fn a_smaller_site_is_a_prefix_of_a_larger_one() {
        let p = wet();
        for seed in 0..10 {
            let (small, large) = (file(&p, seed, 3), file(&p, seed, 8));
            for g in ["WSTG", "WSTD"] {
                assert!(
                    text(&large, g).starts_with(&text(&small, g)),
                    "{g} at seed {seed}"
                );
            }
        }
    }

    /// Made Ground with coarse properties, always 2-3 m thick, over the
    /// fixture's ground, with the table fixed at 1 m so it is always inside it.
    fn made_ground_wet(spec: &str) -> Preset {
        let text = format!(
            "{}{WATER}",
            build::made_ground_tests::with_made_ground(spec)
        )
        .replace(
            "groundwater_depth_m = { min = 1.0, max = 4.0,",
            "groundwater_depth_m = { min = 1.0, max = 1.0,",
        );
        Preset::from_toml(&text).expect("made-ground water fixture loads")
    }

    #[test]
    fn made_ground_without_a_strike_block_never_yields_a_strike_even_with_coarse_properties() {
        let plain = made_ground_wet("");
        let made = made_ground_wet("made_ground = true");
        let mut struck_plain = 0;
        for seed in 0..30 {
            let site = build::build(&made, seed, 6);
            // The skip is explicit: the unit really is coarse, and the table
            // really is inside it.
            assert!(site.properties.is_coarse("MG"));
            for hole in &site.holes {
                let st = hole.strata.iter().find(|s| 1.0 >= s.top && 1.0 < s.base);
                assert_eq!(st.map(|s| s.unit.as_str()), Some("MG"), "seed {seed}");
            }
            assert!(strikes(&site, &made, seed).is_empty(), "seed {seed}");
            // The same ground without the flag strikes in every hole.
            let site = build::build(&plain, seed, 6);
            struck_plain += strikes(&site, &plain, seed).len();
        }
        assert_eq!(struck_plain, 30 * 6);
    }

    /// A sand lens in the London Clay host, flagged by `flag`, with plan radius
    /// range `radius`.
    fn lens(flag: &str, radius: &str) -> String {
        format!(
            r#"
[[lenses]]
id = "WB"
name = "Sand lens"
host = "LC"
radius_m = {{ {radius}, basis = "test fixture" }}
thickness_m = {{ min = 1.0, max = 2.0, basis = "test fixture" }}
depth_in_host = {{ min = 0.3, max = 0.7, basis = "test fixture" }}
description = {{ principals = ["SAND"], secondaries = ["silt"], colours = ["brown"] }}
{flag}
sampling = {{ types = ["D"], interval_m = {{ min = 0.5, max = 1.0, basis = "test fixture" }} }}

[lenses.properties]
spt_n = {{ min = 10, max = 30, basis = "test fixture" }}
moisture_content_percent = {{ min = 8, max = 15, basis = "test fixture" }}
particle_density_ratio = {{ min = 2.6, max = 2.7, basis = "test fixture" }}
"#
        )
    }

    /// Reaches every hole of the 400 x 300 m site.
    const WIDE: &str = "min = 1000.0, max = 1000.0";
    /// Leaves most holes clear of the lens.
    const NARROW: &str = "min = 40.0, max = 80.0";

    /// The fixture with a lens and the water groups.
    fn with_lens(flag: &str, radius: &str) -> Preset {
        Preset::from_toml(&format!("{FIXTURE}{}{WATER}", lens(flag, radius))).expect("loads")
    }

    /// The same with the table 60 m down, below every strike.
    fn deep_table(flag: &str, radius: &str) -> Preset {
        let text = format!("{FIXTURE}{}{WATER}", lens(flag, radius)).replace(
            "groundwater_depth_m = { min = 1.0, max = 4.0,",
            "groundwater_depth_m = { min = 60.0, max = 60.0,",
        );
        Preset::from_toml(&text).expect("loads")
    }

    fn lens_top(hole: &build::Hole) -> Option<f64> {
        hole.strata.iter().find(|s| s.unit == "WB").map(|s| s.top)
    }

    #[test]
    fn a_hole_meeting_a_water_bearing_lens_strikes_at_its_top() {
        let p = with_lens("water_bearing = true", WIDE);
        let mut met = 0;
        for seed in 0..30 {
            let site = build::build(&p, seed, 6);
            let found = strikes(&site, &p, seed);
            for (hi, hole) in site.holes.iter().enumerate() {
                let Some(top) = lens_top(hole) else { continue };
                met += 1;
                let s = found.iter().find(|s| s.hole == hi).expect("strikes");
                assert!((s.depth - top).abs() < 1e-9, "seed {seed}: {s:?} vs {top}");
            }
        }
        assert!(met > 60, "{met} holes met the lens");
    }

    #[test]
    fn readings_rise_toward_the_table_by_at_most_the_rise_and_never_above_it() {
        let p = with_lens("water_bearing = true", WIDE);
        let mut checked = 0;
        for seed in 0..30 {
            let site = build::build(&p, seed, 6);
            let m = build::lower(&site, &p, seed, Grid::Local);
            let (g, d) = (text(&m, "WSTG"), text(&m, "WSTD"));
            for (i, strike) in g.iter().enumerate() {
                let hole = site.holes.iter().find(|h| h.id == strike[0]).unwrap();
                let table = round2(site.properties.groundwater_depth(hole));
                let depth: f64 = strike[1].parse().unwrap();
                assert!(table < depth, "the fixture table is above the lens");
                let posts: Vec<f64> = d[4 * i..4 * i + 4]
                    .iter()
                    .map(|r| r[3].parse().unwrap())
                    .collect();
                assert!(posts.windows(2).all(|w| w[1] <= w[0]), "{posts:?}");
                assert!(posts[0] <= depth);
                assert!(posts[3] >= table - 1e-9, "above the table: {posts:?}");
                // The fixture's default rise is at most 0.5 m.
                assert!(posts[3] >= depth - DEFAULT_RISE_M.1 - 1e-9, "{posts:?}");
                checked += 1;
            }
        }
        assert!(checked > 60);
    }

    #[test]
    fn a_rise_that_would_pass_the_table_stops_at_it() {
        // Water met at 10.00 m with the table at 9.80 m: a drawn rise of 0.5 m
        // settles at the table, one of 0.1 m at 9.90 m.
        let settle = |rise: f64| 9.8_f64.max(round2(10.0 - rise));
        let at = |rise| Strike {
            hole: 0,
            depth: 10.0,
            settle: settle(rise),
        };
        assert!((at(0.5).reading(20) - 9.8).abs() < 1e-9);
        assert!((at(0.1).reading(20) - 9.9).abs() < 1e-9);
    }

    #[test]
    fn perched_water_stays_at_the_strike_depth_when_the_table_is_deeper() {
        let p = deep_table("water_bearing = true", WIDE);
        let mut checked = 0;
        for seed in 0..30 {
            let site = build::build(&p, seed, 6);
            let m = build::lower(&site, &p, seed, Grid::Local);
            let (g, d) = (text(&m, "WSTG"), text(&m, "WSTD"));
            assert_eq!(d.len(), 4 * g.len());
            for (i, strike) in g.iter().enumerate() {
                for r in &d[4 * i..4 * i + 4] {
                    assert_eq!(r[3], strike[1], "perched water rose, seed {seed}");
                }
                checked += 1;
            }
            // The lens is the only source of a strike with the table at 60 m.
            let met = site.holes.iter().filter(|h| lens_top(h).is_some()).count();
            assert_eq!(g.len(), met, "seed {seed}");
        }
        assert!(checked > 60);
    }

    #[test]
    fn a_hole_without_the_lens_gets_no_strike_from_it() {
        let p = deep_table("water_bearing = true", NARROW);
        let (mut met, mut missed) = (0, 0);
        for seed in 0..30 {
            let site = build::build(&p, seed, 6);
            let found = strikes(&site, &p, seed);
            for (hi, hole) in site.holes.iter().enumerate() {
                let struck = found.iter().any(|s| s.hole == hi);
                if lens_top(hole).is_some() {
                    met += 1;
                    assert!(struck, "seed {seed}");
                } else {
                    missed += 1;
                    assert!(!struck, "seed {seed}");
                }
            }
        }
        assert!(met > 5 && missed > 20, "met {met}, missed {missed}");
    }

    #[test]
    fn a_water_bearing_unit_strikes_and_the_first_one_down_is_the_strike() {
        let text = format!("{FIXTURE}{}{WATER}", lens("water_bearing = true", WIDE)).replace(
            "name = \"London Clay\"\n",
            "name = \"London Clay\"\nwater_bearing = true\n",
        );
        let p = Preset::from_toml(&text).expect("loads");
        assert!(p.is_water_bearing("LC") && p.is_water_bearing("WB"));
        for seed in 0..20 {
            let site = build::build(&p, seed, 6);
            let found = strikes(&site, &p, seed);
            assert_eq!(found.len(), 6, "one strike per hole, seed {seed}");
            for (hi, hole) in site.holes.iter().enumerate() {
                // The host's first stratum lies above its lens, so it is the
                // first water-bearing stratum down the hole.
                let first = hole
                    .strata
                    .iter()
                    .find(|s| s.unit == "LC" || s.unit == "WB")
                    .unwrap();
                assert!((found[hi].depth - first.top).abs() < 1e-9, "seed {seed}");
            }
        }
    }

    #[test]
    fn an_explicit_false_flag_is_byte_identical_to_no_flag() {
        let (off, absent) = (
            with_lens("water_bearing = false", WIDE),
            with_lens("", WIDE),
        );
        for seed in 0..10 {
            assert_eq!(
                emit_to_string(&file(&off, seed, 6)),
                emit_to_string(&file(&absent, seed, 6)),
                "seed {seed}"
            );
        }
    }

    #[test]
    fn the_flag_moves_only_the_water_groups() {
        let (on, off) = (with_lens("water_bearing = true", WIDE), with_lens("", WIDE));
        for seed in 0..10 {
            let (a, b) = (file(&on, seed, 6), file(&off, seed, 6));
            for g in ["TRAN", "LOCA", "GEOL", "SAMP"] {
                assert_eq!(text(&a, g), text(&b, g), "{g} at seed {seed}");
            }
        }
    }

    #[test]
    fn a_water_bearing_file_is_rust_clean_across_seeds() {
        for p in [
            with_lens("water_bearing = true", WIDE),
            deep_table("water_bearing = true", WIDE),
        ] {
            for seed in 0..30 {
                assert!(is_clean(&file(&p, seed, 6)), "seed {seed}");
            }
        }
    }

    /// The `water_bearing` line for a plain share.
    fn share(v: f64) -> String {
        format!("water_bearing = {{ share = {v}, basis = \"test fixture\" }}")
    }

    #[test]
    fn the_struck_fraction_of_holes_meeting_the_stratum_matches_the_share() {
        let p = deep_table(&share(0.4), WIDE);
        let (mut met, mut struck) = (0_u32, 0_u32);
        for seed in 0..300 {
            let site = build::build(&p, seed, 6);
            let found = strikes(&site, &p, seed);
            for (hi, hole) in site.holes.iter().enumerate() {
                if lens_top(hole).is_some() {
                    met += 1;
                    struck += u32::from(found.iter().any(|s| s.hole == hi));
                }
            }
        }
        assert!(met > 1000, "{met} holes met the lens");
        let got = f64::from(struck) / f64::from(met);
        assert!((got - 0.4).abs() < 0.05, "{struck} of {met} struck: {got}");
    }

    #[test]
    fn a_struck_hole_strikes_at_the_top_of_the_stratum() {
        let p = deep_table(&share(0.5), WIDE);
        for seed in 0..30 {
            let site = build::build(&p, seed, 6);
            for s in strikes(&site, &p, seed) {
                let top = lens_top(&site.holes[s.hole]).expect("met the lens");
                assert!((s.depth - top).abs() < 1e-9, "seed {seed}");
            }
        }
    }

    #[test]
    fn the_share_is_drawn_once_per_site_within_its_range() {
        let p = with_lens(
            "water_bearing = { share = { min = 0.2, max = 0.8, basis = \"test fixture\" } }",
            WIDE,
        );
        let mut seen = std::collections::BTreeSet::new();
        for seed in 0..40 {
            let found = site_shares(&p, seed);
            let (_, v) = *found.iter().find(|(id, _)| *id == "WB").expect("WB");
            assert!((0.2..=0.8).contains(&v), "{v}");
            assert_eq!(found, site_shares(&p, seed), "same seed, same share");
            seen.insert(format!("{v:.4}"));
        }
        assert!(seen.len() > 30, "{} distinct shares", seen.len());
    }

    #[test]
    fn true_is_byte_identical_to_a_share_of_one() {
        let (on, one) = (
            with_lens("water_bearing = true", WIDE),
            with_lens(&share(1.0), WIDE),
        );
        let range = with_lens(
            "water_bearing = { share = { min = 1.0, max = 1.0, basis = \"test fixture\" } }",
            WIDE,
        );
        for seed in 0..10 {
            let want = emit_to_string(&file(&on, seed, 6));
            assert_eq!(want, emit_to_string(&file(&one, seed, 6)), "seed {seed}");
            assert_eq!(want, emit_to_string(&file(&range, seed, 6)), "seed {seed}");
        }
    }

    #[test]
    fn a_share_of_one_makes_no_per_hole_draw() {
        // Every hole that meets the stratum strikes at its top, and the
        // strike rise is the only draw: the same rows as the pre-share rule.
        let p = with_lens("water_bearing = true", WIDE);
        for seed in 0..10 {
            let site = build::build(&p, seed, 6);
            let found = strikes(&site, &p, seed);
            assert_eq!(found.len(), 6, "seed {seed}");
            for (hi, hole) in site.holes.iter().enumerate() {
                let first = hole.strata.iter().find(|s| s.unit == "WB");
                if let Some(first) = first {
                    assert!((found[hi].depth - first.top).abs() < 1e-9);
                }
            }
        }
    }

    #[test]
    fn a_share_of_zero_gives_no_strikes_from_the_stratum() {
        let zero = deep_table(&share(0.0), WIDE);
        for seed in 0..30 {
            let site = build::build(&zero, seed, 6);
            assert!(strikes(&site, &zero, seed).is_empty(), "seed {seed}");
            assert!(rows(&file(&zero, seed, 6), "WSTG").is_empty());
        }
    }

    #[test]
    fn a_hole_that_does_not_yield_falls_back_to_the_table_rule() {
        // With the table in reach, a share of zero is the file without the
        // flag, byte for byte.
        let (zero, absent) = (with_lens(&share(0.0), WIDE), with_lens("", WIDE));
        for seed in 0..10 {
            assert_eq!(
                emit_to_string(&file(&zero, seed, 6)),
                emit_to_string(&file(&absent, seed, 6)),
                "seed {seed}"
            );
        }
    }

    #[test]
    fn a_share_moves_only_the_water_groups() {
        let (on, off) = (with_lens(&share(0.5), WIDE), with_lens("", WIDE));
        for seed in 0..10 {
            let (a, b) = (file(&on, seed, 6), file(&off, seed, 6));
            for g in ["TRAN", "LOCA", "GEOL", "SAMP"] {
                assert_eq!(text(&a, g), text(&b, g), "{g} at seed {seed}");
            }
        }
    }

    #[test]
    fn a_shared_water_bearing_file_is_rust_clean_across_seeds() {
        for p in [with_lens(&share(0.5), WIDE), deep_table(&share(0.5), WIDE)] {
            for seed in 0..30 {
                assert!(is_clean(&file(&p, seed, 6)), "seed {seed}");
            }
        }
    }

    // ---- Water strikes in till (#77) ----

    /// The `strike` table of a till unit: a rate per metre, a rise range and a
    /// no-rise share.
    fn till_table(rate: f64, rise: (f64, f64), no_rise: f64) -> String {
        format!(
            "strike = {{ rate_per_m = {{ min = {rate}, max = {rate}, basis = \"test fixture\" }}, \
             rise_m = {{ min = {}, max = {}, basis = \"test fixture\" }}, \
             no_rise_share = {{ min = {no_rise}, max = {no_rise}, basis = \"test fixture\" }} }}",
            rise.0, rise.1
        )
    }

    /// The fixture's weathered clay (WLC, 2-8 m) and London Clay (LC, to final
    /// depth) with strike tables, and the water groups. `table` is the water
    /// table's depth range: the default (deep) puts it below every strike.
    fn tills(wlc: &str, lc: &str, table: &str) -> Preset {
        let mut text = format!("{FIXTURE}{WATER}");
        for (name, spec) in [("Weathered London Clay", wlc), ("London Clay", lc)] {
            text = text.replacen(
                &format!("name = \"{name}\"\n"),
                &format!("name = \"{name}\"\n{spec}\n"),
                1,
            );
        }
        let text = text.replace(
            "groundwater_depth_m = { min = 1.0, max = 4.0,",
            &format!("groundwater_depth_m = {{ {table},"),
        );
        Preset::from_toml(&text).expect("till fixture loads")
    }

    const DEEP: &str = "min = 60.0, max = 60.0";

    fn lc_only(rate: f64) -> Preset {
        tills("", &till_table(rate, (1.0, 2.0), 0.0), DEEP)
    }

    /// The stratum of `hole` that `depth` is in.
    fn stratum_at(hole: &build::Hole, depth: f64) -> (usize, &build::Stratum) {
        hole.strata
            .iter()
            .enumerate()
            .find(|(i, s)| hole.written_top(*i) <= depth && depth < s.base)
            .expect("the strike is in a stratum")
    }

    #[test]
    fn a_till_strike_falls_inside_its_unit_and_above_final_depth() {
        let p = lc_only(0.05);
        let mut seen = 0;
        for seed in 0..80 {
            let site = build::build(&p, seed, 6);
            for s in strikes(&site, &p, seed) {
                let hole = &site.holes[s.hole];
                let (i, st) = stratum_at(hole, s.depth);
                assert_eq!(st.unit, "LC", "seed {seed}");
                assert!(s.depth > hole.written_top(i), "at the top, seed {seed}");
                assert!(s.depth < st.base && s.depth < hole.final_depth);
                seen += 1;
            }
        }
        assert!(seen > 100, "{seen} strikes is too few to mean anything");
    }

    #[test]
    fn a_till_strike_is_spread_evenly_through_the_stratum() {
        // With a rate low enough that most holes miss, the first event given
        // that it lands in the stratum is nearly uniform in it.
        let p = tills("", &till_table(0.004, (1.0, 2.0), 0.0), DEEP);
        let (mut top_half, mut n) = (0u32, 0u32);
        for seed in 0..600 {
            let site = build::build(&p, seed, 6);
            for s in strikes(&site, &p, seed) {
                let hole = &site.holes[s.hole];
                let (i, st) = stratum_at(hole, s.depth);
                let at = (s.depth - hole.written_top(i)) / (st.base - hole.written_top(i));
                top_half += u32::from(at < 0.5);
                n += 1;
            }
        }
        assert!(n > 300, "{n} strikes");
        let share = f64::from(top_half) / f64::from(n);
        assert!((share - 0.5).abs() < 0.08, "{share} of {n} in the top half");
    }

    #[test]
    fn the_rate_gives_the_expected_share_of_holes_striking_in_the_unit() {
        let rate = 0.03;
        let p = lc_only(rate);
        let (mut expected, mut struck, mut met) = (0.0, 0u32, 0u32);
        for seed in 0..400 {
            let site = build::build(&p, seed, 6);
            let found = strikes(&site, &p, seed);
            for (hi, hole) in site.holes.iter().enumerate() {
                let metres: f64 = hole
                    .strata
                    .iter()
                    .enumerate()
                    .filter(|(_, s)| s.unit == "LC")
                    .map(|(i, s)| s.base - hole.written_top(i))
                    .sum();
                met += 1;
                expected += 1.0 - (-rate * metres).exp();
                struck += u32::from(found.iter().any(|s| s.hole == hi));
            }
        }
        let (got, want) = (
            f64::from(struck) / f64::from(met),
            expected / f64::from(met),
        );
        assert!(met > 2000, "{met} holes");
        assert!(
            (got - want).abs() < 0.03,
            "{struck} of {met}: {got} vs {want}"
        );
    }

    #[test]
    fn a_rate_of_zero_makes_no_strike_and_moves_no_byte() {
        let (zero, absent) = (lc_only(0.0), tills("", "", DEEP));
        for seed in 0..10 {
            assert!(strikes(&build::build(&zero, seed, 6), &zero, seed).is_empty());
            assert_eq!(
                emit_to_string(&file(&zero, seed, 6)),
                emit_to_string(&file(&absent, seed, 6)),
                "seed {seed}"
            );
        }
        // With the table in reach, the table rule is untouched too.
        let (zero, absent) = (
            tills(
                "",
                &till_table(0.0, (1.0, 2.0), 0.0),
                "min = 1.0, max = 4.0",
            ),
            tills("", "", "min = 1.0, max = 4.0"),
        );
        for seed in 0..10 {
            assert_eq!(
                emit_to_string(&file(&zero, seed, 6)),
                emit_to_string(&file(&absent, seed, 6)),
                "seed {seed}"
            );
        }
    }

    #[test]
    fn the_shallowest_of_a_lens_top_and_a_till_strike_wins() {
        let lens_on = format!("{}{}", lens("water_bearing = true", WIDE), WATER);
        let build_p = |flag: bool, table: &str| {
            let lens_text = if flag {
                lens_on.clone()
            } else {
                format!("{}{}", lens("", WIDE), WATER)
            };
            let text = format!("{FIXTURE}{lens_text}")
                .replacen(
                    "name = \"London Clay\"\n",
                    &format!(
                        "name = \"London Clay\"\n{}\n",
                        till_table(0.08, (1.0, 2.0), 0.0)
                    ),
                    1,
                )
                .replace(
                    "groundwater_depth_m = { min = 1.0, max = 4.0,",
                    &format!("groundwater_depth_m = {{ {table},"),
                );
            Preset::from_toml(&text).expect("loads")
        };
        let (both, till_only) = (build_p(true, DEEP), build_p(false, DEEP));
        let (mut lens_won, mut till_won) = (0, 0);
        for seed in 0..60 {
            let site = build::build(&both, seed, 6);
            let (a, b) = (
                strikes(&site, &both, seed),
                strikes(&site, &till_only, seed),
            );
            for (hi, hole) in site.holes.iter().enumerate() {
                let till = b.iter().find(|s| s.hole == hi).map(|s| s.depth);
                let top = lens_top(hole);
                let want = match (top, till) {
                    (Some(t), Some(d)) if d < t => Some(d),
                    (Some(t), _) => Some(t),
                    (None, d) => d,
                };
                let got = a.iter().find(|s| s.hole == hi).map(|s| s.depth);
                assert_eq!(got, want, "seed {seed} hole {hi}");
                if let (Some(t), Some(d)) = (top, till) {
                    if d < t {
                        till_won += 1;
                    } else {
                        lens_won += 1;
                    }
                }
            }
            assert!(a.len() <= 6, "one strike per hole");
        }
        assert!(lens_won > 15 && till_won > 15, "{lens_won} / {till_won}");
    }

    #[test]
    fn a_table_strike_in_a_coarse_stratum_counts_only_without_a_till_or_lens_strike() {
        let wet = tills(
            "",
            &till_table(0.01, (1.0, 2.0), 0.0),
            "min = 1.0, max = 4.0",
        );
        let deep = tills("", &till_table(0.01, (1.0, 2.0), 0.0), DEEP);
        let (mut by_till, mut by_table) = (0, 0);
        for seed in 0..60 {
            let site = build::build(&wet, seed, 6);
            let with_table = strikes(&site, &wet, seed);
            // The same ground with the table out of reach: the till strikes alone.
            let till = strikes(&build::build(&deep, seed, 6), &deep, seed);
            for (hi, hole) in site.holes.iter().enumerate() {
                let got = with_table.iter().find(|s| s.hole == hi);
                let table = round2(site.properties.groundwater_depth(hole));
                if let Some(t) = till.iter().find(|s| s.hole == hi) {
                    // The table sits in the shallower RTD yet loses.
                    assert_eq!(got.map(|s| s.depth), Some(t.depth), "seed {seed}");
                    by_till += usize::from(table < t.depth);
                } else {
                    let in_rtd = hole
                        .strata
                        .iter()
                        .any(|s| s.unit == "RTD" && table >= s.top && table < s.base);
                    assert_eq!(got.is_some(), in_rtd, "seed {seed}");
                    by_table += usize::from(in_rtd);
                }
            }
        }
        assert!(by_till > 20 && by_table > 20, "{by_till} / {by_table}");
    }

    /// A till strike of `depth` metres in hole 0, with the unit's rise `rise`
    /// and `no_rise` share.
    fn till_settle(depth: f64, rise: (f64, f64), no_rise: f64, seed: u64) -> f64 {
        let spec = UnitStrike {
            rate: StrikeRate::PerMetre(Range {
                min: 0.1,
                max: 0.1,
                basis: "t".into(),
                skew: None,
            }),
            rise_m: Range {
                min: rise.0,
                max: rise.1,
                basis: "t".into(),
                skew: None,
            },
            no_rise_share: Range {
                min: no_rise,
                max: no_rise,
                basis: "t".into(),
                skew: None,
            },
        };
        let t = TillStrike {
            unit: "LC",
            spec: &spec,
            no_rise,
            depth,
        };
        t.strike(0, seed, 0).settle
    }

    #[test]
    fn a_till_strike_rises_by_its_draw_whatever_the_table_and_never_above_ground() {
        // Struck at 10.00 m with a 2 m rise: the readings settle at 8.00 m
        // with the table above the strike, at it or below it.
        assert!((till_settle(10.0, (2.0, 2.0), 0.0, 1) - 8.0).abs() < 1e-9);
        // A rise larger than the depth stops at ground level.
        assert!(till_settle(1.5, (4.0, 4.0), 0.0, 1).abs() < 1e-9);
        // A skewed or wide range never rises above ground.
        for seed in 0..50 {
            let settle = till_settle(3.0, (0.1, 7.0), 0.0, seed);
            assert!((0.0..=2.9).contains(&settle), "{settle}");
        }
    }

    /// Whichever side of the strike the table is on, the file's till strikes
    /// rise by the unit's draw and settle between ground and the strike.
    #[test]
    fn a_till_strike_file_rises_with_the_table_above_or_below_the_strike() {
        let (mut above, mut below) = (0, 0);
        for table in ["min = 1.0, max = 1.0", DEEP] {
            let p = tills(
                &till_table(0.05, (1.0, 1.0), 0.0),
                &till_table(0.05, (1.0, 1.0), 0.0),
                table,
            );
            for seed in 0..30 {
                let site = build::build(&p, seed, 6);
                for s in strikes(&site, &p, seed) {
                    let hole = &site.holes[s.hole];
                    if !["WLC", "LC"].contains(&stratum_at(hole, s.depth).1.unit.as_str()) {
                        continue;
                    }
                    let want = round2(s.depth - 1.0).max(0.0);
                    assert!((s.settle - want).abs() < 1e-9, "seed {seed} {s:?}");
                    assert!(s.settle >= 0.0 && s.settle <= s.depth);
                    if table == DEEP {
                        below += 1;
                    } else {
                        above += 1;
                    }
                }
            }
        }
        assert!(above > 10 && below > 10, "{above} / {below}");
    }

    #[test]
    #[allow(clippy::float_cmp)] // the settled level is the strike depth itself
    fn the_no_rise_share_leaves_that_share_of_strikes_at_the_strike() {
        let n = 4000_u32;
        let stayed = (0..u64::from(n))
            .filter(|&seed| till_settle(10.0, (1.0, 1.0), 0.3, seed) == 10.0)
            .count();
        let share = stayed as f64 / f64::from(n);
        assert!((share - 0.3).abs() < 0.03, "{share}");
        for seed in 0..50 {
            assert_eq!(till_settle(10.0, (1.0, 1.0), 1.0, seed), 10.0);
            assert!((till_settle(10.0, (1.0, 1.0), 0.0, seed) - 9.0).abs() < 1e-9);
        }
    }

    #[test]
    fn till_readings_rise_monotonically_and_settle_at_the_twenty_minute_level() {
        let p = tills(
            &till_table(0.05, (0.2, 3.0), 0.12),
            &till_table(0.05, (0.2, 3.0), 0.12),
            "min = 3.0, max = 9.0",
        );
        let mut checked = 0;
        let (mut rose, mut stayed) = (0, 0);
        for seed in 0..60 {
            let site = build::build(&p, seed, 6);
            let found = strikes(&site, &p, seed);
            let m = build::lower(&site, &p, seed, Grid::Local);
            let (g, d) = (text(&m, "WSTG"), text(&m, "WSTD"));
            assert_eq!(g.len(), found.len());
            assert_eq!(d.len(), 4 * g.len(), "seed {seed}");
            for (i, strike) in g.iter().enumerate() {
                let depth: f64 = strike[1].parse().unwrap();
                let posts: Vec<f64> = d[4 * i..4 * i + 4]
                    .iter()
                    .map(|r| r[3].parse().unwrap())
                    .collect();
                let minutes: Vec<&str> =
                    d[4 * i..4 * i + 4].iter().map(|r| r[2].as_str()).collect();
                assert_eq!(minutes, ["5", "10", "15", "20"]);
                assert!(posts[0] <= depth + 1e-9, "{posts:?} vs {depth}");
                assert!(posts.windows(2).all(|w| w[1] <= w[0] + 1e-9), "{posts:?}");
                assert!((posts[3] - found[i].settle).abs() < 1e-9, "seed {seed}");
                assert!(found[i].settle <= depth + 1e-9);
                if (depth - posts[3]).abs() < 1e-9 {
                    stayed += 1;
                } else {
                    rose += 1;
                }
                checked += 1;
            }
        }
        assert!(checked > 100, "{checked}");
        assert!(rose > 20 && stayed > 20, "rose {rose}, stayed {stayed}");
    }

    #[test]
    fn changing_one_units_rate_moves_only_strike_rows_of_holes_that_change() {
        let base = tills(
            &till_table(0.03, (1.0, 2.0), 0.1),
            &till_table(0.03, (1.0, 2.0), 0.1),
            "min = 3.0, max = 9.0",
        );
        let more = tills(
            &till_table(0.03, (1.0, 2.0), 0.1),
            &till_table(0.06, (1.0, 2.0), 0.1),
            "min = 3.0, max = 9.0",
        );
        let (mut same, mut moved) = (0, 0);
        for seed in 0..40 {
            let (a, b) = (file(&base, seed, 6), file(&more, seed, 6));
            for g in ["TRAN", "LOCA", "GEOL", "SAMP"] {
                assert_eq!(text(&a, g), text(&b, g), "{g} at seed {seed}");
            }
            let site = build::build(&base, seed, 6);
            let (sa, sb) = (strikes(&site, &base, seed), strikes(&site, &more, seed));
            for (hi, hole) in site.holes.iter().enumerate() {
                let (x, y) = (
                    sa.iter().find(|s| s.hole == hi),
                    sb.iter().find(|s| s.hole == hi),
                );
                // A strike in the weathered clay, above the changed unit, is
                // unmoved by it; so are the readings.
                if let Some(x) = x
                    && stratum_at(hole, x.depth).1.unit == "WLC"
                {
                    assert_eq!(Some(x), y, "seed {seed} hole {hi}");
                    same += 1;
                }
                if let (Some(x), Some(y)) = (x, y)
                    && stratum_at(hole, x.depth).1.unit == "LC"
                {
                    // The same draw, a higher rate: never deeper.
                    assert!(y.depth <= x.depth + 1e-9, "seed {seed} hole {hi}");
                    moved += 1;
                }
            }
        }
        assert!(same > 20 && moved > 20, "{same} / {moved}");
    }

    #[test]
    fn till_strikes_are_a_prefix_property_across_hole_counts() {
        let p = tills(
            &till_table(0.03, (1.0, 2.0), 0.1),
            &till_table(0.03, (1.0, 2.0), 0.1),
            "min = 3.0, max = 9.0",
        );
        for seed in 0..10 {
            let (small, large) = (file(&p, seed, 3), file(&p, seed, 8));
            for g in ["WSTG", "WSTD"] {
                assert!(
                    text(&large, g).starts_with(&text(&small, g)),
                    "{g} at seed {seed}"
                );
            }
        }
    }

    #[test]
    fn a_till_strike_file_is_rust_clean_across_seeds() {
        let p = tills(
            &till_table(0.05, (0.2, 3.0), 0.12),
            &till_table(0.05, (0.2, 3.0), 0.12),
            "min = 3.0, max = 9.0",
        );
        let mut with_strikes = 0;
        for seed in 0..30 {
            let m = file(&p, seed, 6);
            assert!(is_clean(&m), "seed {seed}");
            with_strikes += usize::from(!rows(&m, "WSTG").is_empty());
        }
        assert!(with_strikes > 20, "only {with_strikes} files strike water");
    }

    #[test]
    fn till_strikes_move_no_loca_geol_or_samp_byte() {
        let plain = Preset::from_toml(&format!("{FIXTURE}{WATER}")).unwrap();
        let p = tills(
            &till_table(0.05, (0.2, 3.0), 0.12),
            &till_table(0.05, (0.2, 3.0), 0.12),
            "min = 1.0, max = 4.0",
        );
        for seed in 0..10 {
            let (with, without) = (file(&p, seed, 5), file(&plain, seed, 5));
            for g in ["TRAN", "LOCA", "GEOL", "SAMP"] {
                assert_eq!(text(&with, g), text(&without, g), "{g} at seed {seed}");
            }
        }
    }
}

/// Several strikes per hole (`site.several_strikes`, #79), on the shipped
/// tyneside-till.
#[cfg(test)]
// Depths are written to 2 decimal places and compared as the exact values
// the plan rounded to.
#[allow(clippy::float_cmp)]
mod several_tests {
    use super::super::build::{self, Grid, default_hole_count};
    use super::super::construction;
    use super::super::preset::{Preset, resolve};
    use super::*;
    use crate::pipeline::rust_check;
    use crate::synth::emit::emit_to_string;
    use laterite_ags4_parity::RustResult;

    fn shipped() -> Preset {
        resolve("tyneside-till").expect("shipped preset loads")
    }

    /// The preset with the key removed: the single-strike rule.
    fn single(p: &Preset) -> Preset {
        let mut q = p.clone();
        q.site.several_strikes = None;
        q
    }

    fn site_of(p: &Preset, seed: u64) -> Site {
        build::build(p, seed, default_hole_count(seed))
    }

    fn of_hole(all: &[Strike], hole: usize) -> Vec<Strike> {
        all.iter().filter(|s| s.hole == hole).copied().collect()
    }

    #[test]
    fn a_hole_records_several_strikes_in_order_within_the_cap_and_the_gap() {
        let p = shipped();
        let rule = p.site.several_strikes.clone().expect("v9 has the key");
        let mut by_count = [0_usize; 5];
        for seed in 0..600 {
            let site = site_of(&p, seed);
            let all = strikes(&site, &p, seed);
            for (hi, hole) in site.holes.iter().enumerate() {
                let mine = of_hole(&all, hi);
                assert!(mine.len() <= rule.max_per_hole as usize, "seed {seed}");
                by_count[mine.len()] += 1;
                for s in &mine {
                    assert!(s.depth > 0.0 && s.depth <= hole.final_depth, "{s:?}");
                    let host = hole
                        .strata
                        .iter()
                        .any(|u| u.top <= s.depth && s.depth <= u.base);
                    assert!(host, "seed {seed}: {s:?} is in no stratum");
                    assert!(s.settle <= s.depth && s.settle >= 0.0, "{s:?}");
                }
                for w in mine.windows(2) {
                    assert!(
                        w[1].depth - w[0].depth >= rule.min_gap_m - 1e-9,
                        "seed {seed} hole {hi}: {w:?}"
                    );
                }
            }
        }
        assert!(by_count[2] > 50, "{by_count:?}");
        assert!(by_count[3] > 10, "{by_count:?}");
        assert!(by_count[4] > 0, "{by_count:?}");
    }

    #[test]
    fn a_lower_cap_keeps_the_shallowest_strikes_and_a_wider_gap_spaces_them() {
        let p = shipped();
        let mut two = p.clone();
        two.site.several_strikes.as_mut().unwrap().max_per_hole = 2;
        let mut wide = p.clone();
        wide.site.several_strikes.as_mut().unwrap().min_gap_m = 3.0;
        let mut dropped = 0;
        for seed in 0..400 {
            let site = site_of(&p, seed);
            let four = strikes(&site, &p, seed);
            let capped = strikes(&site, &two, seed);
            let spaced = strikes(&site, &wide, seed);
            for hi in 0..site.holes.len() {
                let (a, b) = (of_hole(&four, hi), of_hole(&capped, hi));
                assert!(b.len() <= 2);
                assert_eq!(b[..], a[..b.len()], "seed {seed} hole {hi}");
                dropped += a.len() - b.len();
                for w in of_hole(&spaced, hi).windows(2) {
                    assert!(w[1].depth - w[0].depth >= 3.0 - 1e-9, "seed {seed}: {w:?}");
                }
            }
        }
        assert!(dropped > 5, "{dropped} strikes were dropped by the cap");
    }

    /// The Poisson stream continues past its first event using draws the
    /// single-strike rule never made, so every strike that rule records is
    /// still there (unless the one above it pushed it by under the gap), and a
    /// till strike settles where it did.
    #[test]
    fn every_strike_of_the_single_rule_is_still_struck() {
        let p = shipped();
        let one = single(&p);
        let (mut kept, mut extra) = (0, 0);
        for seed in 0..400 {
            let site = site_of(&p, seed);
            let (many, old) = (strikes(&site, &p, seed), strikes(&site, &one, seed));
            for o in &old {
                let mine = of_hole(&many, o.hole);
                let same = mine.iter().find(|s| s.depth == o.depth);
                let above = mine
                    .iter()
                    .any(|s| s.depth < o.depth && o.depth - s.depth < 0.5);
                assert!(
                    same.is_some() || above,
                    "seed {seed}: {o:?} lost from {mine:?}"
                );
                if let Some(s) = same {
                    kept += 1;
                    let hole = &site.holes[o.hole];
                    let in_till = hole
                        .strata
                        .iter()
                        .any(|u| u.top <= o.depth && o.depth < u.base && u.unit != "GFSG");
                    if in_till {
                        assert_eq!(s.settle, o.settle, "seed {seed}");
                    }
                }
            }
            extra += many.len() - old.len();
        }
        assert!(
            kept > 100 && extra > 50,
            "{kept} kept, {extra} more strikes"
        );
    }

    #[test]
    fn a_second_strike_in_a_till_has_its_own_rise_draw() {
        let p = shipped();
        let (mut pairs, mut equal) = (0, 0);
        for seed in 0..2000 {
            let site = site_of(&p, seed);
            let all = strikes(&site, &p, seed);
            for (hi, hole) in site.holes.iter().enumerate() {
                let mine = of_hole(&all, hi);
                let unit = |d: f64| {
                    hole.strata
                        .iter()
                        .find(|u| u.top <= d && d < u.base)
                        .map(|u| u.unit.as_str())
                };
                for w in mine.windows(2) {
                    let u = unit(w[0].depth);
                    if u == unit(w[1].depth) && u != Some("GFSG") {
                        pairs += 1;
                        let rise = |s: &Strike| round2(s.depth - s.settle);
                        equal += usize::from(rise(&w[0]) == rise(&w[1]));
                    }
                }
            }
        }
        assert!(pairs > 100, "{pairs} pairs in one till");
        // Zero rises (the no-rise share, and ground level) are equal, but not
        // most pairs.
        assert!(equal * 5 < pairs, "{equal} of {pairs} pairs rise alike");
    }

    #[test]
    fn a_later_strike_has_its_own_wstd_rows() {
        let p = shipped();
        for seed in 0..40 {
            let site = site_of(&p, seed);
            let m = build::lower(&site, &p, seed, Grid::Local);
            let rows = |code: &str| -> Vec<Vec<String>> {
                m.groups
                    .iter()
                    .find(|g| g.code == code)
                    .map_or(Vec::new(), |g| {
                        g.rows.iter().map(|r| r.values.clone()).collect()
                    })
            };
            let (wstg, wstd) = (rows("WSTG"), rows("WSTD"));
            assert_eq!(wstd.len(), wstg.len() * MINUTES.len(), "seed {seed}");
            // Each WSTG key (hole, depth) has its four readings.
            for g in &wstg {
                let n = wstd.iter().filter(|d| d[0] == g[0] && d[1] == g[1]).count();
                assert_eq!(n, MINUTES.len(), "seed {seed}: {g:?}");
            }
        }
    }

    #[test]
    fn wstg_lists_the_seal_before_the_casing_as_the_dictionary_does() {
        let p = shipped();
        let m = build::lower(&site_of(&p, 1), &p, 1, Grid::Local);
        let g = m.groups.iter().find(|g| g.code == "WSTG").expect("WSTG");
        assert_eq!(
            g.headings,
            ["LOCA_ID", "WSTG_DPTH", "WSTG_SEAL", "WSTG_CAS"]
        );
        // No seal key, no seal column.
        let mut bare = p.clone();
        bare.construction.as_mut().unwrap().seal = None;
        let m = build::lower(&site_of(&bare, 1), &bare, 1, Grid::Local);
        let g = m.groups.iter().find(|g| g.code == "WSTG").expect("WSTG");
        assert_eq!(g.headings, ["LOCA_ID", "WSTG_DPTH", "WSTG_CAS"]);
    }

    /// The seal key adds a column and raises a later strike's casing to the
    /// seal above it; every other group is the same.
    #[test]
    fn the_seal_key_moves_no_group_but_wstg() {
        let p = shipped();
        let mut bare = p.clone();
        bare.construction.as_mut().unwrap().seal = None;
        for seed in 0..15 {
            let site = site_of(&p, seed);
            let (a, b) = (
                build::lower(&site, &p, seed, Grid::Local),
                build::lower(&site, &bare, seed, Grid::Local),
            );
            assert_eq!(a.groups.len(), b.groups.len(), "seed {seed}");
            for (x, y) in a.groups.iter().zip(&b.groups) {
                if x.code != "WSTG" {
                    assert_eq!(x, y, "{} at seed {seed}", x.code);
                }
            }
        }
    }

    /// The casing and the seal against the strikes, for every hole.
    #[test]
    fn casing_and_seals_lie_where_the_rules_put_them() {
        let p = shipped();
        let (mut sealed, mut later, mut passing) = (0, 0, 0);
        for seed in 0..600 {
            let site = site_of(&p, seed);
            let all = strikes(&site, &p, seed);
            let plan = construction::plan(&site, &p, seed).expect("a construction block");
            for (hi, b) in plan.holes.iter().enumerate() {
                let mine = of_hole(&all, hi);
                assert_eq!(b.strike_casing.len(), mine.len());
                assert_eq!(b.strike_seal.len(), mine.len());
                let fc = b.casing.last().expect("a casing string").0;
                if let Some((last, earlier)) = mine.split_last() {
                    passing += usize::from(fc > last.depth);
                    for e in earlier {
                        assert!(fc > e.depth, "seed {seed}: casing {fc} stops above {e:?}");
                    }
                }
                for (j, s) in mine.iter().enumerate() {
                    let cas = b.strike_casing[j];
                    assert!(cas <= s.depth && cas <= fc && cas >= 0.0, "seed {seed}");
                    if let Some(prev) = j.checked_sub(1) {
                        later += 1;
                        let floor = b.strike_seal[prev].unwrap_or(mine[prev].depth);
                        assert!(cas >= floor, "seed {seed}: casing {cas} above {floor}");
                    }
                    let Some(seal) = b.strike_seal[j] else {
                        continue;
                    };
                    sealed += 1;
                    assert!(seal >= cas, "seed {seed}: seal {seal} above casing {cas}");
                    match b.strike_casing.get(j + 1) {
                        Some(next) => assert!(seal <= *next, "seed {seed}: seal past the casing"),
                        None => assert!(seal <= fc, "seed {seed}: seal {seal} below casing {fc}"),
                    }
                }
            }
        }
        assert!(sealed > 100 && later > 50 && passing > 100);
    }

    #[test]
    fn a_zone_follows_the_first_or_the_deepest_strike_at_the_share() {
        let p = shipped();
        let with_share = |share: Option<f64>| {
            let mut changed = p.clone();
            let install = changed
                .construction
                .as_mut()
                .unwrap()
                .installation
                .as_mut()
                .unwrap();
            match share {
                Some(v) => {
                    let range = install.zone_follows_first_share.as_mut().unwrap();
                    (range.min, range.max) = (v, v);
                }
                None => install.zone_follows_first_share = None,
            }
            changed
        };
        let (first, deepest, bare) = (
            with_share(Some(1.0)),
            with_share(Some(0.0)),
            with_share(None),
        );
        let (mut deeper, mut multi) = (0, 0);
        for seed in 0..200 {
            let site = site_of(&p, seed);
            let all = strikes(&site, &p, seed);
            let zones = |q: &Preset| -> Vec<Option<(f64, f64)>> {
                construction::plan(&site, q, seed)
                    .unwrap()
                    .holes
                    .iter()
                    .map(|b| b.installation.as_ref().map(|i| i.zone))
                    .collect()
            };
            let (f, d, none) = (zones(&first), zones(&deepest), zones(&bare));
            // Following the first strike is what a preset without the share
            // does.
            assert_eq!(f, none, "seed {seed}");
            for (hi, (zf, zd)) in f.iter().zip(&d).enumerate() {
                assert_eq!(zf.is_some(), zd.is_some(), "seed {seed}");
                let (Some(zf), Some(zd)) = (zf, zd) else {
                    continue;
                };
                assert!(zd.0 >= zf.0, "seed {seed} hole {hi}");
                if of_hole(&all, hi).len() < 2 {
                    assert_eq!(zf, zd, "one strike, one zone");
                } else {
                    multi += 1;
                    deeper += usize::from(zd.0 > zf.0);
                }
            }
        }
        assert!(multi > 20 && deeper * 2 > multi, "{deeper} of {multi}");
    }

    #[test]
    fn several_strikes_files_are_clean_by_construction() {
        let p = shipped();
        for seed in 0..60 {
            let site = site_of(&p, seed);
            let text = emit_to_string(&build::lower(&site, &p, seed, Grid::Local));
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("site.ags");
            std::fs::write(&path, text).unwrap();
            assert_eq!(rust_check(&path).0, RustResult::Clean, "seed {seed}");
        }
    }
}

/// Water strikes in made ground (a `strike` block on a made-ground unit,
/// london-basin v9, #84), and the banded rate behind them.
#[cfg(test)]
#[allow(clippy::float_cmp)]
mod made_ground_strike_tests {
    use super::super::build::{self, Grid, default_hole_count};
    use super::super::preset::{Preset, resolve};
    use super::*;
    use crate::pipeline::rust_check;
    use crate::synth::emit::emit_to_string;
    use laterite_ags4_parity::RustResult;

    const WATER: &str = r#"
[[suite]]
group = "WSTG"
headings = []

[[suite]]
group = "WSTD"
headings = []
"#;

    /// The made-ground line and a `strike` block with the given rate keys.
    fn block(rate: &str) -> String {
        format!(
            "made_ground = true\nstrike = {{ {rate}, \
             rise_m = {{ min = 0.2, max = 1.0, basis = \"test fixture\" }}, \
             no_rise_share = {{ min = 0.0, max = 0.0, basis = \"test fixture\" }} }}"
        )
    }

    fn flat(rate: f64) -> String {
        format!("rate_per_m = {{ min = {rate}, max = {rate}, basis = \"test fixture\" }}")
    }

    fn band(from: f64, rate: f64) -> String {
        format!(
            "{{ from_m = {from}, rate_per_m = {{ min = {rate}, max = {rate}, basis = \"test fixture\" }} }}"
        )
    }

    const DEEP: &str = "min = 60.0, max = 60.0";

    /// The fixture's 2-3 m of made ground over the terrace deposits, with the
    /// unit's `made_ground` and `strike` lines `spec` and the water table
    /// `table` metres down.
    fn preset(spec: &str, table: &str) -> Preset {
        let text = format!(
            "{}{WATER}",
            build::made_ground_tests::with_made_ground(spec)
        )
        .replace(
            "groundwater_depth_m = { min = 1.0, max = 4.0,",
            &format!("groundwater_depth_m = {{ {table},"),
        );
        Preset::from_toml(&text).expect("made-ground strike fixture loads")
    }

    fn in_mg(hole: &build::Hole, s: &Strike) -> bool {
        hole.strata
            .iter()
            .any(|st| st.unit == "MG" && s.depth >= st.top && s.depth < st.base)
    }

    #[test]
    fn a_banded_rate_walks_the_bands_in_turn() {
        let bands = [(0.0, 1.0), (1.0, 2.0), (3.0, 0.0), (4.0, 4.0)];
        let go = SiteRate::banded_advance;
        assert!((go(&bands, 0.0, 0.5) - 0.5).abs() < 1e-12);
        // 1 of exposure in the first band, then 0.5 at 2 per m.
        assert!((go(&bands, 0.0, 1.5) - 1.25).abs() < 1e-12);
        // Starting inside a band.
        assert!((go(&bands, 1.5, 0.5) - 1.75).abs() < 1e-12);
        // 1 + 4 of exposure to reach 3 m, a dead band, then 4 per m from 4 m.
        assert!((go(&bands, 0.0, 5.0 + 2.0) - 4.5).abs() < 1e-12);
        // Nothing below a last band with no rate.
        assert!(go(&[(0.0, 1.0), (1.0, 0.0)], 0.0, 2.0).is_infinite());
    }

    #[test]
    fn a_one_band_rate_draws_the_same_events_as_the_flat_rate() {
        let flat_p = preset(&block(&flat(0.7)), DEEP);
        let band_p = preset(
            &block(&format!("rate_by_depth = [{}]", band(0.0, 0.7))),
            DEEP,
        );
        let mut seen = 0;
        for seed in 0..20 {
            let a = strikes(&build::build(&flat_p, seed, 6), &flat_p, seed);
            let b = strikes(&build::build(&band_p, seed, 6), &band_p, seed);
            let depths = |v: &[Strike]| v.iter().map(|s| (s.hole, s.depth)).collect::<Vec<_>>();
            assert_eq!(depths(&a), depths(&b), "seed {seed}");
            seen += a.len();
        }
        assert!(seen > 20, "{seen}");
    }

    #[test]
    fn a_banded_rate_strikes_only_in_bands_with_a_rate() {
        let upper = preset(
            &block(&format!(
                "rate_by_depth = [{}, {}]",
                band(0.0, 3.0),
                band(1.0, 0.0)
            )),
            DEEP,
        );
        let lower = preset(
            &block(&format!(
                "rate_by_depth = [{}, {}]",
                band(0.0, 0.0),
                band(1.0, 3.0)
            )),
            DEEP,
        );
        let (mut up, mut down) = (0, 0);
        for seed in 0..20 {
            for s in strikes(&build::build(&upper, seed, 6), &upper, seed) {
                assert!(s.depth < 1.0, "seed {seed}: {}", s.depth);
                up += 1;
            }
            for s in strikes(&build::build(&lower, seed, 6), &lower, seed) {
                assert!(s.depth >= 1.0, "seed {seed}: {}", s.depth);
                down += 1;
            }
        }
        assert!(up > 30 && down > 30, "{up} / {down}");
    }

    #[test]
    fn made_ground_strikes_are_above_the_water_table_and_rise_to_ground_at_most() {
        // A high rate, so an unconstrained hole would strike in its 2-3 m of
        // made ground, and a table 1.2-2.4 m down that cuts it.
        let p = preset(&block(&flat(2.0)), "min = 1.2, max = 2.4");
        let (mut struck, mut cut) = (0, 0);
        for seed in 0..40 {
            let site = build::build(&p, seed, 6);
            let all = strikes(&site, &p, seed);
            for (hi, hole) in site.holes.iter().enumerate() {
                let table = round2(site.properties.groundwater_depth(hole));
                for s in all.iter().filter(|s| s.hole == hi && in_mg(hole, s)) {
                    assert!(
                        s.depth < table,
                        "seed {seed} hole {hi}: {} >= {table}",
                        s.depth
                    );
                    assert!(s.settle >= 0.0 && s.settle <= s.depth);
                    struck += 1;
                }
                cut += usize::from(table < 2.0);
            }
        }
        assert!(struck > 30 && cut > 30, "{struck} / {cut}");
    }

    #[test]
    fn a_made_ground_strike_block_is_all_that_lets_it_strike() {
        // The table rule still excludes made ground: with the table in the
        // made ground and no block there is no strike, and with the block the
        // strikes are the block's, all above the table.
        let none = preset("made_ground = true", "min = 1.0, max = 1.0");
        let some = preset(&block(&flat(2.0)), "min = 1.0, max = 1.0");
        let mut got = 0;
        for seed in 0..10 {
            assert!(strikes(&build::build(&none, seed, 6), &none, seed).is_empty());
            for s in strikes(&build::build(&some, seed, 6), &some, seed) {
                assert!(s.depth < 1.0);
                got += 1;
            }
        }
        assert!(got > 20, "{got}");
    }

    #[test]
    fn one_units_bands_move_no_other_units_rate() {
        let rate_of = |p: &Preset, id: &str| {
            till_rates(p, 7)
                .into_iter()
                .find(|t| t.unit == id)
                .map(|t| match t.rate {
                    SiteRate::Flat(r) => vec![r],
                    SiteRate::Banded(b) => b.into_iter().map(|(_, r)| r).collect(),
                })
        };
        let bands = |lo: f64, hi: f64| {
            let r = format!("rate_per_m = {{ min = {lo}, max = {hi}, basis = \"t\" }}");
            format!("rate_by_depth = [{{ from_m = 0.0, {r} }}, {{ from_m = 2.0, {r} }}]")
        };
        let lc = "strike = { rate_per_m = { min = 0.05, max = 0.2, basis = \"t\" }, \
                  rise_m = { min = 0.1, max = 1.0, basis = \"t\" }, \
                  no_rise_share = { min = 0.1, max = 0.1, basis = \"t\" } }";
        let text = |mg: &str| {
            format!(
                "{}{WATER}",
                build::made_ground_tests::with_made_ground(&block(mg))
            )
            .replacen(
                "name = \"London Clay\"\n",
                &format!("name = \"London Clay\"\n{lc}\n"),
                1,
            )
        };
        let a = Preset::from_toml(&text(&bands(0.1, 0.3))).unwrap();
        let b = Preset::from_toml(&text(&bands(0.5, 0.9))).unwrap();
        assert_ne!(rate_of(&a, "MG"), rate_of(&b, "MG"));
        assert_eq!(rate_of(&a, "LC"), rate_of(&b, "LC"));
        // And one unit's bands are drawn apart from each other.
        let r = rate_of(&a, "MG").unwrap();
        assert_ne!(r[0], r[1]);
    }

    fn london() -> Preset {
        resolve("london-basin").expect("shipped preset loads")
    }

    #[test]
    fn london_basin_made_ground_water_stands_above_the_deeper_strikes_water() {
        let p = london();
        assert!(p.site.several_strikes.is_some());
        let (mut mg_holes, mut pairs) = (0, 0);
        for seed in 0..40 {
            let site = build::build(&p, seed, 40);
            let all = strikes(&site, &p, seed);
            for (hi, hole) in site.holes.iter().enumerate() {
                let mine: Vec<&Strike> = all.iter().filter(|s| s.hole == hi).collect();
                let Some(first) = mine.iter().find(|s| in_mg(hole, s)) else {
                    continue;
                };
                mg_holes += 1;
                for deeper in mine
                    .iter()
                    .filter(|s| !in_mg(hole, s) && s.depth > first.depth)
                {
                    // Perched: the made-ground water is shallower than the
                    // water in any deeper strike.
                    assert!(
                        first.settle < deeper.settle,
                        "seed {seed} hole {hi}: {} vs {}",
                        first.settle,
                        deeper.settle
                    );
                    pairs += 1;
                }
            }
        }
        assert!(mg_holes > 20 && pairs > 10, "{mg_holes} / {pairs}");
    }

    #[test]
    fn london_basin_files_with_made_ground_strikes_are_clean_by_construction() {
        let p = london();
        let mut mg = 0;
        for seed in 0..40 {
            let site = build::build(&p, seed, default_hole_count(seed));
            let text = emit_to_string(&build::lower(&site, &p, seed, Grid::Local));
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("site.ags");
            std::fs::write(&path, text).unwrap();
            assert_eq!(rust_check(&path).0, RustResult::Clean, "seed {seed}");
            mg += strikes(&site, &p, seed)
                .iter()
                .filter(|s| in_mg(&site.holes[s.hole], s))
                .count();
        }
        assert!(mg > 10, "{mg} made-ground strikes");
    }

    #[test]
    fn the_gap_rule_never_pushes_a_made_ground_strike_to_the_water_table() {
        // Many close events (4 per m, a 0.5 m gap) in 2-3 m of made ground,
        // with the table 1.5-3.5 m down: a push would reach the table often.
        let mut p = preset(&block(&flat(4.0)), "min = 1.5, max = 3.5");
        p.site.several_strikes = Some(SeveralStrikes {
            max_per_hole: 4,
            min_gap_m: 0.5,
            basis: "test fixture".into(),
        });
        let (mut mg, mut pairs) = (0, 0);
        for seed in 0..60 {
            let site = build::build(&p, seed, 8);
            let all = strikes(&site, &p, seed);
            for (hi, hole) in site.holes.iter().enumerate() {
                let table = round2(site.properties.groundwater_depth(hole));
                let mine: Vec<&Strike> = all.iter().filter(|s| s.hole == hi).collect();
                for m in mine.iter().filter(|s| in_mg(hole, s)) {
                    assert!(
                        m.depth < table,
                        "seed {seed} hole {hi}: {} >= {table}",
                        m.depth
                    );
                    mg += 1;
                    for d in mine.iter().filter(|s| !in_mg(hole, s) && s.depth > m.depth) {
                        assert!(m.settle < d.settle, "seed {seed} hole {hi}");
                        pairs += 1;
                    }
                }
            }
        }
        assert!(mg > 100 && pairs > 20, "{mg} / {pairs}");
    }

    #[test]
    fn a_cover_moves_no_made_ground_strike_below_its_base() {
        let preset = london();
        let mut bare = preset.clone();
        bare.cover = None;
        let (mut covered, mut compared) = (0, 0);
        for seed in 0..60 {
            let (a, b) = (
                build::build(&preset, seed, 20),
                build::build(&bare, seed, 20),
            );
            let (sa, sb) = (strikes(&a, &preset, seed), strikes(&b, &bare, seed));
            for (hi, hole) in a.holes.iter().enumerate() {
                let Some(cover) = &hole.cover else { continue };
                covered += 1;
                let below = |all: &[Strike], site: &Site| {
                    all.iter()
                        .filter(|s| {
                            s.hole == hi && s.depth > cover.base && in_mg(&site.holes[hi], s)
                        })
                        .copied()
                        .collect::<Vec<_>>()
                };
                // The cap can differ in a hole the bare file fills with
                // shallower strikes; compare the holes under it.
                if sb.iter().filter(|s| s.hole == hi).count() >= 4 {
                    continue;
                }
                let (with, without) = (below(&sa, &a), below(&sb, &b));
                assert_eq!(with, without, "seed {seed} hole {hi}");
                compared += with.len();
            }
        }
        assert!(covered > 20 && compared > 10, "{covered} / {compared}");
    }
}
