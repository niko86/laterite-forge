//! Water strikes: `WSTG` and `WSTD` rows where a hole meets water (ADR 0001,
//! "Ground").
//!
//! Two rules decide a hole's strike. A hole has at most one strike group (see
//! below), and the water-bearing rule takes precedence over the table rule.
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
//! **One strike per hole.** AGS allows several `WSTG` rows per hole, but one
//! strike per hole keeps the keyed draw count and the file shape of the
//! table-only rule. A hole that meets several water-bearing strata records the
//! first. A table strike in a shallower coarse stratum is dropped in favour of
//! the water-bearing strike, which the issue makes the rule for such a hole
//! (`forge judgement`).
//!
//! The default rise, the logging minutes and the decay are `forge judgement`: the
//! ordinary UK 5-minute, 20-minute logging pattern, not a published curve.

use laterite_ags4_validator::{DictVersion, Dictionary};

use super::build::Site;
use super::draw::{Key, uniform};
use super::preset::Preset;
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

/// The strikes of every hole in file order: at most one per hole.
pub(super) fn strikes(site: &Site, preset: &Preset, seed: u64) -> Vec<Strike> {
    let (rise_min, rise_max) = preset
        .site
        .strike_rise_m
        .as_ref()
        .map_or(DEFAULT_RISE_M, |r| (r.min, r.max));
    let shares = site_shares(preset, seed);
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
            // Made Ground yields no strike, whatever its properties: perched
            // water is out of scope (niko86/laterite-forge#42, M2).
            .filter(|s| !preset.is_made_ground(&s.unit) && site.properties.is_coarse(&s.unit));
        // The first water-bearing stratum down the hole that yields water,
        // struck at its written top (below the cover).
        let bearing = hole
            .strata
            .iter()
            .enumerate()
            .find(|(_, s)| yields(&shares, seed, hi, &s.unit))
            .map(|(i, _)| hole.written_top(i));
        if in_table.is_none() && bearing.is_none() {
            continue;
        }
        let rise = uniform(&mut Key::hole("strike", hi).rng(seed), rise_min, rise_max);
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
        // A water-bearing stratum takes precedence over the table rule.
        out.extend(by_stratum.or(by_table));
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

/// The `WSTG` group, or `None` when no hole strikes water.
pub(super) fn wstg(site: &Site, strikes: &[Strike]) -> Option<Group> {
    let rows = strikes
        .iter()
        .map(|s| Row::owned(vec![site.holes[s.hole].id.clone(), m2(s.depth)]))
        .collect();
    build("WSTG", &["LOCA_ID", "WSTG_DPTH"], strikes, rows)
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
    fn made_ground_never_yields_a_strike_even_with_coarse_properties() {
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
}
