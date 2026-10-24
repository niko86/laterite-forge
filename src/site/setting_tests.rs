//! The made ground a built site shows, measured over many seeds, against the
//! tables it is built from (niko86/laterite-forge#86).

#![allow(
    clippy::float_cmp,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_lossless
)]

use super::build::{self, Hole, Stratum};
use super::preset::{Preset, resolve};
use super::setting::{GENERIC_ID, Patches, Setting, Settlement, Targets};
use super::water;

/// The nearest-rank percentile of `v` (sorted in place).
fn pct(v: &mut [f64], p: f64) -> f64 {
    v.sort_by(f64::total_cmp);
    v[(((v.len() - 1) as f64) * p).round() as usize]
}

fn tyneside(setting: Setting, settlement: Settlement) -> Preset {
    let mut p = resolve("tyneside-till").expect("shipped preset loads");
    p.set_made_ground(Some(setting), Some(settlement))
        .expect("tyneside-till has the block");
    p
}

/// The made-ground stratum of a hole, if it has one.
fn made(h: &Hole) -> Option<&Stratum> {
    h.strata.first().filter(|s| s.unit == GENERIC_ID)
}

/// What a run of sites shows.
#[derive(Default)]
struct Shown {
    /// Per site: the fraction of its holes with made ground.
    shares: Vec<f64>,
    /// Per hole with made ground: its thickness.
    thick: Vec<f64>,
    /// Holes with made ground, and how many of them end in it.
    with_mg: usize,
    ending_in_mg: usize,
    /// Sum and count of |thickness difference| between a made-ground hole and
    /// its nearest hole where that has made ground too, and over all pairs of
    /// made-ground holes.
    nn: (f64, usize),
    all: (f64, usize),
    /// Made-ground holes whose nearest hole has made ground, and the number
    /// expected if made ground were placed at random.
    nn_is_mg: (usize, f64),
}

fn shown(p: &Preset, seeds: std::ops::Range<u64>, holes: usize) -> Shown {
    let mut out = Shown::default();
    for seed in seeds {
        let site = build::build(p, seed, holes);
        let mg: Vec<Option<f64>> = site
            .holes
            .iter()
            .map(|h| made(h).map(|s| s.base - s.top))
            .collect();
        let n = mg.iter().flatten().count();
        out.shares.push(n as f64 / holes as f64);
        out.thick.extend(mg.iter().flatten());
        out.with_mg += n;
        out.ending_in_mg += site
            .holes
            .iter()
            .filter(|h| made(h).is_some_and(|s| s.base >= h.final_depth))
            .count();
        for (i, hi) in site.holes.iter().enumerate() {
            let Some(ti) = mg[i] else { continue };
            for tj in mg.iter().skip(i + 1).flatten() {
                out.all.0 += (ti - tj).abs();
                out.all.1 += 1;
            }
            let dist = |k: usize| (site.holes[k].x - hi.x).hypot(site.holes[k].y - hi.y);
            let near = (0..holes)
                .filter(|j| *j != i)
                .min_by(|a, b| dist(*a).total_cmp(&dist(*b)))
                .expect("two holes");
            out.nn_is_mg.1 += (n - 1) as f64 / (holes - 1) as f64;
            if let Some(tj) = mg[near] {
                out.nn.0 += (ti - tj).abs();
                out.nn.1 += 1;
                out.nn_is_mg.0 += 1;
            }
        }
    }
    out
}

impl Shown {
    /// Mean |thickness difference| of nearest neighbours over that of all
    /// pairs: below 1 when made ground is patchy.
    fn patchiness(&self) -> f64 {
        (self.nn.0 / self.nn.1 as f64) / (self.all.0 / self.all.1 as f64)
    }
}

fn shipped_text(name: &str) -> &'static str {
    super::preset::SHIPPED
        .iter()
        .find(|(n, _)| *n == name)
        .expect("a shipped preset")
        .1
}

/// tyneside-till's text with one exact snippet replaced.
fn tyneside_with(from: &str, to: &str) -> String {
    let text = shipped_text("tyneside-till");
    assert!(text.contains(from), "tyneside-till lacks {from:?}");
    text.replacen(from, to, 1)
}

fn refusals(text: &str) -> Vec<String> {
    Preset::from_toml(text).expect_err("should be refused").0
}

#[test]
fn an_unknown_setting_is_refused_naming_the_choices() {
    let errs = refusals(&tyneside_with(
        "setting = \"corridor\"",
        "setting = \"swamp\"",
    ));
    assert_eq!(
        errs,
        [
            "made_ground.setting: \"swamp\" is not one of greenfield, brownfield, developed, corridor"
        ]
    );
}

#[test]
fn an_unknown_settlement_is_refused_naming_the_choices() {
    let errs = refusals(&tyneside_with(
        "settlement = \"town\"",
        "settlement = \"metropolis\"",
    ));
    assert_eq!(
        errs,
        ["made_ground.settlement: \"metropolis\" is not one of village, town, city"]
    );
}

#[test]
fn a_block_without_its_setting_is_refused() {
    let errs = refusals(&tyneside_with("setting = \"corridor\"\n", ""));
    assert_eq!(errs, ["made_ground.setting: missing"]);
}

#[test]
fn a_patch_wavelength_that_is_not_above_zero_or_runs_backwards_is_refused() {
    let errs = refusals(&tyneside_with(
        "wavelength_m = { min = 50.0, max = 200.0,",
        "wavelength_m = { min = 0.0, max = 200.0,",
    ));
    assert_eq!(errs, ["made_ground.wavelength_m: min 0 must be above 0"]);
    let errs = refusals(&tyneside_with(
        "wavelength_m = { min = 50.0, max = 200.0,",
        "wavelength_m = { min = 300.0, max = 200.0,",
    ));
    assert_eq!(
        errs,
        ["made_ground.wavelength_m: min 300 is greater than max 200"]
    );
}

#[test]
fn a_patch_wavelength_without_a_basis_is_refused() {
    let text = tyneside_with(
        "wavelength_m = { min = 50.0, max = 200.0, basis = \"",
        "wavelength_m = { min = 50.0, max = 200.0, xbasis = \"",
    );
    let errs = refusals(&text);
    assert!(
        errs.iter()
            .any(|e| e == "made_ground.wavelength_m: no basis"),
        "{errs:?}"
    );
}

#[test]
fn a_block_beside_a_unit_called_mg_is_refused() {
    let errs = refusals(&tyneside_with("id = \"WT\"", "id = \"MG\""));
    assert!(
        errs.iter()
            .any(|e| e.contains("a unit or lens already uses")),
        "{errs:?}"
    );
}

#[test]
fn a_block_needs_the_properties_a_made_ground_unit_does() {
    let text = tyneside_with("[made_ground.properties]", "[made_ground.nothing]");
    let errs = refusals(&text);
    assert!(errs.iter().any(|e| e.contains("made_ground")), "{errs:?}");
}

#[test]
fn a_block_may_give_its_own_make_up_in_place_of_the_settings() {
    let text = tyneside_with(
        "sampling = { types = [\"B\", \"D\"]",
        "make_up = { count = { weights = [1.0, 0.0, 0.0, 0.0], basis = \"test fixture\" } }\nsampling = { types = [\"B\", \"D\"]",
    );
    let mut p = Preset::from_toml(&text).expect("loads");
    let own = p.units[0].made_ground.clone().expect("a made-ground unit");
    assert!(
        own.constituents.is_none(),
        "only the count is the block's own"
    );
    assert_eq!(own.count.expect("count").weights, [1.0, 0.0, 0.0, 0.0]);
    // A setting override keeps the block's own weights.
    p.set_made_ground(Some(Setting::Brownfield), None).unwrap();
    assert!(
        p.units[0]
            .made_ground
            .as_ref()
            .unwrap()
            .constituents
            .is_none()
    );
}

#[test]
fn a_setting_override_swaps_the_units_make_up_for_the_new_settings() {
    let mut p = resolve("tyneside-till").unwrap();
    let before = p.units[0].made_ground.clone();
    p.set_made_ground(Some(Setting::Developed), None).unwrap();
    let after = p.units[0].made_ground.clone();
    assert_ne!(before, after);
    let m = p.made_ground.as_ref().unwrap();
    assert_eq!(
        (m.setting, m.settlement),
        (Setting::Developed, Settlement::Town)
    );
}

#[test]
fn a_preset_without_the_block_has_no_generic_made_ground_to_set() {
    let mut london = resolve("london-basin").unwrap();
    assert!(london.made_ground.is_none());
    assert_eq!(
        london.set_made_ground(Some(Setting::Brownfield), None),
        Err("this preset has no generic made ground".to_string())
    );
    assert_eq!(
        london.set_made_ground(None, Some(Settlement::City)),
        Err("this preset has no generic made ground".to_string())
    );
    // Its own Made Ground unit is the one it always had.
    assert_eq!(london.units[0].id, "MG");
    assert!(
        london.units[0]
            .thickness_m
            .as_ref()
            .is_some_and(|t| t.min == -1.0)
    );
}

/// Each setting's site share and thickness, at a town, over 400 sites of 40
/// holes, against the table the build is fitted to.
#[test]
fn each_setting_gives_its_share_and_thickness_at_a_town() {
    for setting in Setting::ALL {
        let p = tyneside(setting, Settlement::Town);
        let t = Targets::new(setting, Settlement::Town);
        let mut s = shown(&p, 0..400, 40);
        let (share, p10, p50, p90) = (
            pct(&mut s.shares, 0.5),
            pct(&mut s.thick, 0.1),
            pct(&mut s.thick, 0.5),
            pct(&mut s.thick, 0.9),
        );
        assert!(
            (share - t.share_median()).abs() < 0.08,
            "{}: median share {share} against {}",
            setting.name(),
            t.share_median()
        );
        for (got, want, which) in [
            (p10, t.p10, "P10"),
            (p50, t.p50, "P50"),
            (p90, t.p90, "P90"),
        ] {
            assert!(
                (got / want - 1.0).abs() < 0.15,
                "{} {which}: {got} against {want}",
                setting.name()
            );
        }
        // Every site's share lies in the setting's range (plus the sampling
        // of 40 holes).
        let lo = s.shares.iter().copied().fold(1.0, f64::min);
        let hi = s.shares.iter().copied().fold(0.0, f64::max);
        assert!(lo >= t.share_min - 0.25 && hi <= t.share_max + 0.25);
    }
}

/// Setting orders the made ground the way the tables say: brownfield is the
/// most and the thickest, greenfield the least and the thinnest.
#[test]
fn settings_rank_from_greenfield_thinnest_to_brownfield_thickest() {
    let median = |setting| {
        let mut s = shown(&tyneside(setting, Settlement::Town), 0..200, 40);
        (pct(&mut s.shares, 0.5), pct(&mut s.thick, 0.5))
    };
    let (green, corridor, brown) = (
        median(Setting::Greenfield),
        median(Setting::Corridor),
        median(Setting::Brownfield),
    );
    assert!(
        green.0 < corridor.0 && corridor.0 < brown.0,
        "{green:?} {corridor:?} {brown:?}"
    );
    assert!(
        green.1 < corridor.1 && corridor.1 < brown.1,
        "{green:?} {corridor:?} {brown:?}"
    );
}

/// A village has a smaller share and thinner made ground than a town, a city
/// more and thicker, by the settlement tables (share x 0.6 and thickness
/// x 0.7; half the gap to 1 and thickness x 1.2).
#[test]
fn a_village_has_less_made_ground_than_a_town_and_a_city_more() {
    for setting in [Setting::Corridor, Setting::Brownfield] {
        let at = |settlement| {
            let mut s = shown(&tyneside(setting, settlement), 0..400, 40);
            let mean: f64 = s.shares.iter().sum::<f64>() / s.shares.len() as f64;
            (mean, pct(&mut s.thick, 0.5))
        };
        let (village, town, city) = (
            at(Settlement::Village),
            at(Settlement::Town),
            at(Settlement::City),
        );
        assert!(
            village.0 < town.0 && town.0 < city.0,
            "{}: {village:?} {town:?} {city:?}",
            setting.name()
        );
        assert!(
            (village.1 / town.1 - 0.7).abs() < 0.07,
            "{}: village thickness {:.2} of the town's",
            setting.name(),
            village.1 / town.1
        );
        assert!(
            (city.1 / town.1 - 1.2).abs() < 0.1,
            "{}: city thickness {:.2} of the town's",
            setting.name(),
            city.1 / town.1
        );
        let t = Targets::new(setting, Settlement::Village);
        assert!(village.0 <= t.share_max, "{}: {village:?}", setting.name());
    }
}

/// Made ground is patchy, not a blanket: a made-ground hole's nearest hole is
/// more often made ground than chance, and the thickness differs less
/// between near holes than between any two.
#[test]
fn made_ground_is_patchy_at_the_scale_of_the_hole_spacing() {
    let p = tyneside(Setting::Corridor, Settlement::Town);
    let s = shown(&p, 0..300, 40);
    assert!(s.patchiness() < 0.8, "ratio {}", s.patchiness());
    let (near, expected) = (s.nn_is_mg.0 as f64, s.nn_is_mg.1);
    assert!(
        near > 1.1 * expected,
        "{near} nearest holes are made ground against {expected} at random"
    );
    // And not a smooth blanket either: some holes at a site have none while
    // others have it.
    let mixed = s.shares.iter().filter(|x| **x > 0.1 && **x < 0.9).count();
    assert!(mixed > 50, "{mixed} sites of 300 have both");
}

/// The patches lie at tens of metres, not across the site: made-ground
/// thickness two holes 20 m apart is closer than two holes 120 m apart.
#[test]
fn made_ground_thickness_agrees_at_short_range_and_less_at_long_range() {
    let preset = tyneside(Setting::Brownfield, Settlement::Town);
    let block = preset.made_ground.as_ref().unwrap();
    let (mut near, mut far, mut count) = (0.0, 0.0, 0);
    for seed in 0..200 {
        let patches = Patches::new(block, preset.site.extent_m, seed);
        for i in 0..40 {
            let (east, north) = (f64::from(i) * 6.0 + 20.0, f64::from(i % 7) * 25.0 + 10.0);
            let (here, close, distant) = (
                patches.thickness_at(east, north),
                patches.thickness_at(east + 20.0, north),
                patches.thickness_at(east + 120.0, north),
            );
            if here > 0.0 && close > 0.0 && distant > 0.0 {
                near += (here - close).abs();
                far += (here - distant).abs();
                count += 1;
            }
        }
    }
    assert!(count > 1000);
    assert!(near < 0.8 * far, "near {near} far {far}");
}

/// Made ground is the first unit: each natural unit starts where it ends and
/// the hole is one run of strata from the ground to its final depth.
#[test]
fn made_ground_is_stacked_first_and_the_natural_units_start_at_its_base() {
    let p = tyneside(Setting::Brownfield, Settlement::Town);
    let mut with = 0;
    for seed in 0..60 {
        for h in build::build(&p, seed, 12).holes {
            assert_eq!(h.strata[0].top, 0.0);
            for w in h.strata.windows(2) {
                assert_eq!(w[0].base, w[1].top, "seed {seed} {}", h.id);
            }
            assert_eq!(h.strata.last().unwrap().base, h.final_depth);
            if let Some(m) = made(&h) {
                with += 1;
                assert!(m.base > m.top);
                assert!(h.strata.iter().skip(1).all(|s| s.unit != GENERIC_ID));
            }
        }
    }
    assert!(with > 300, "{with}");
}

/// The setting moves only the made ground: the hole positions, depths and
/// ground levels, and each natural unit's own thickness, are those of a
/// site with any other setting.
#[test]
fn changing_the_setting_moves_only_the_made_ground_and_what_it_pushes_down() {
    let a = tyneside(Setting::Greenfield, Settlement::Town);
    let b = tyneside(Setting::Brownfield, Settlement::City);
    let mut compared = 0;
    for seed in 0..40 {
        let (sa, sb) = (build::build(&a, seed, 10), build::build(&b, seed, 10));
        for (ha, hb) in sa.holes.iter().zip(&sb.holes) {
            assert_eq!(
                (ha.x, ha.y, ha.ground_level, ha.final_depth),
                (hb.x, hb.y, hb.ground_level, hb.final_depth)
            );
            for ua in ha.strata.iter().filter(|s| s.unit == "WT") {
                // Thick enough that the final depth cuts neither.
                let Some(ub) = hb.strata.iter().find(|s| s.unit == "WT") else {
                    continue;
                };
                if ua.base < ha.final_depth && ub.base < hb.final_depth {
                    assert!(((ua.base - ua.top) - (ub.base - ub.top)).abs() < 0.0101);
                    compared += 1;
                }
            }
        }
    }
    assert!(compared > 100, "{compared}");
}

/// A shallow hole can end in made ground, which stops at the final depth as
/// any unit does (the crawl has 16% of its made-ground holes ending in it).
#[test]
fn a_hole_can_end_in_made_ground_and_it_stops_at_the_final_depth() {
    // Without its construction, which needs deeper holes.
    let text = tyneside_with(
        "final_depth_m = { min = 15.0, max = 30.0,",
        "final_depth_m = { min = 1.0, max = 4.0,",
    );
    let text = &text[..text.find("\n[construction]").expect("a construction block")];
    let mut p = Preset::from_toml(text).expect("loads");
    p.set_made_ground(Some(Setting::Brownfield), None).unwrap();
    let mut s = shown(&p, 0..100, 20);
    let ending = s.ending_in_mg as f64 / s.with_mg as f64;
    assert!(ending > 0.2, "{ending}");
    assert!(pct(&mut s.thick, 1.0) <= 4.0);
    for seed in 0..20 {
        for h in build::build(&p, seed, 20).holes {
            if made(&h).is_some_and(|m| m.base >= h.final_depth) {
                assert_eq!(h.strata.len(), 1, "nothing below the made ground");
                assert_eq!(h.strata[0].base, h.final_depth);
            }
        }
    }
}

/// A made-ground strike is perched above the hole's water table, outside the
/// cover, and sits in the made ground it names (#84's rules on the generic
/// unit).
#[test]
fn made_ground_strikes_are_above_the_water_table_and_below_the_cover() {
    let mut struck = 0;
    for setting in [Setting::Brownfield, Setting::Corridor] {
        let p = tyneside(setting, Settlement::Town);
        for seed in 0..120 {
            let site = build::build(&p, seed, 8);
            for s in water::strikes(&site, &p, seed) {
                let hole = &site.holes[s.hole];
                let Some(m) = made(hole) else { continue };
                if s.depth < m.top || s.depth >= m.base {
                    continue;
                }
                struck += 1;
                let table = site.properties.groundwater_depth(hole);
                assert!(
                    s.depth < table,
                    "seed {seed}: {} at or below {table}",
                    s.depth
                );
                assert!(
                    s.depth >= hole.written_top(0),
                    "seed {seed}: inside the cover"
                );
                assert!(s.settle <= s.depth);
            }
        }
    }
    assert!(struck > 100, "{struck}");
}

/// The block's strike rates reach the file: made-ground strikes per metre by
/// band follow the NE bands within sampling.
#[test]
fn made_ground_strike_rates_follow_the_ne_bands() {
    let p = tyneside(Setting::Corridor, Settlement::Town);
    let (metres, count) = strikes_per_band(&p, 0..600, 8);
    for (b, want) in [0.021, 0.038, 0.048, 0.076].into_iter().enumerate() {
        let got = count[b] as f64 / metres[b];
        assert!(
            (got / want - 1.0).abs() < 0.3,
            "band {b}: {got} against {want}"
        );
    }
}

/// Metres of made ground in each of the strike bands (0-1, 1-2, 2-3, 3-5 and
/// below 5 m) and the strikes in them, over `seeds` sites of `holes` holes.
fn strikes_per_band(
    p: &Preset,
    seeds: std::ops::Range<u64>,
    holes: usize,
) -> ([f64; 5], [usize; 5]) {
    const EDGES: [(f64, f64); 5] = [
        (0.0, 1.0),
        (1.0, 2.0),
        (2.0, 3.0),
        (3.0, 5.0),
        (5.0, f64::INFINITY),
    ];
    let (mut metres, mut count) = ([0.0; 5], [0; 5]);
    for seed in seeds {
        let site = build::build(p, seed, holes);
        for s in water::strikes(&site, p, seed) {
            let hole = &site.holes[s.hole];
            if made(hole).is_some_and(|m| s.depth >= hole.written_top(0) && s.depth < m.base) {
                let band = EDGES.iter().position(|e| s.depth < e.1).expect("open band");
                count[band] += 1;
            }
        }
        for h in &site.holes {
            if let Some(m) = made(h) {
                for (b, e) in EDGES.iter().enumerate() {
                    metres[b] += (m.base.min(e.1) - h.written_top(0).max(e.0)).max(0.0);
                }
            }
        }
    }
    (metres, count)
}

/// Prints tyneside-till v10's figures for the bases (run with `--ignored
/// --nocapture`, in release).
#[test]
#[ignore = "a calibration run that prints figures; not a check"]
fn measure_tyneside_till_v10() {
    let p = tyneside(Setting::Corridor, Settlement::Town);
    for holes in [7, 12, 20, 40] {
        let s = shown(&p, 0..2000, holes);
        println!(
            "{holes} holes: patchiness {:.2}; nearest hole of an MG hole is MG {:.1}% vs {:.1}% at random",
            s.patchiness(),
            100.0 * s.nn_is_mg.0 as f64 / s.with_mg as f64,
            100.0 * s.nn_is_mg.1 / (s.with_mg as f64),
        );
        println!(
            "   nearest-is-MG {} of {} MG holes; random expectation {:.0}",
            s.nn_is_mg.0, s.with_mg, s.nn_is_mg.1
        );
    }
    let mut rises = Vec::new();
    let mut all = 0;
    for seed in 0..3000 {
        let site = build::build(&p, seed, 8);
        for s in water::strikes(&site, &p, seed) {
            let hole = &site.holes[s.hole];
            if made(hole).is_some_and(|m| s.depth >= hole.written_top(0) && s.depth < m.base) {
                all += 1;
                let rise = s.depth - s.settle;
                if rise > 0.05 {
                    rises.push(rise);
                }
            }
        }
    }
    println!(
        "MG strikes {all}, rising {} ({:.1}% with no rise); rise P10/25/50/75/90 {:.2}/{:.2}/{:.2}/{:.2}/{:.2} max {:.2}",
        rises.len(),
        100.0 * (1.0 - rises.len() as f64 / all as f64),
        pct(&mut rises, 0.1),
        pct(&mut rises, 0.25),
        pct(&mut rises, 0.5),
        pct(&mut rises, 0.75),
        pct(&mut rises, 0.9),
        pct(&mut rises, 1.0),
    );
    let (metres, count) = strikes_per_band(&p, 0..3000, 8);
    for b in 0..5 {
        println!(
            "band {b}: {} strikes in {:.0} m = {:.4} per m",
            count[b],
            metres[b],
            count[b] as f64 / metres[b]
        );
    }
}

/// Prints the file's figures for the bases (run with `--ignored --nocapture`,
/// in release).
#[test]
#[ignore = "a calibration run that prints figures; not a check"]
fn measure_the_file_against_the_tables() {
    for setting in Setting::ALL {
        for settlement in Settlement::ALL {
            let p = tyneside(setting, settlement);
            let mut s = shown(&p, 0..1500, 40);
            let t = Targets::new(setting, settlement);
            println!(
                "{:<10} {:<8} share P10/50/90 {:.2}/{:.2}/{:.2} (draw median {:.2}, range {:.2}-{:.2}); thickness P10/50/90 {:.2}/{:.2}/{:.2} vs {:.2}/{:.2}/{:.2}; ends in MG {:.1}%; patchiness {:.2}",
                setting.name(),
                settlement.name(),
                pct(&mut s.shares, 0.1),
                pct(&mut s.shares, 0.5),
                pct(&mut s.shares, 0.9),
                t.share_median(),
                t.share_min,
                t.share_max,
                pct(&mut s.thick, 0.1),
                pct(&mut s.thick, 0.5),
                pct(&mut s.thick, 0.9),
                t.p10,
                t.p50,
                t.p90,
                100.0 * s.ending_in_mg as f64 / s.with_mg.max(1) as f64,
                s.patchiness(),
            );
        }
    }
}
