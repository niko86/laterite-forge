//! A preset's suite: which test groups a `--site` file carries and on which
//! samples (ADR 0001, "Output and determinism").
//!
//! A preset's suite writes KEY and REQUIRED headings, plus the headings it
//! lists, whose values come from [`PRODUCERS`] (index results live in
//! `results`). Everything here draws from its own keyed streams, so a suite
//! moves no LOCA, GEOL or SAMP byte.
//!
//! Scheduling:
//! - `SAMP`-child groups (LNMC, LLPL, LDEN, TRIG) test a sample when its
//!   `SAMP_TYPE` can carry the group (`SampleQuality::permitted_samp_types`)
//!   and a keyed draw falls under the group's rate. The row's KEY is the SAMP
//!   KEY plus `SPEC_REF = 1` and `SPEC_DPTH` at the sample's mid-depth.
//! - TRIT is scheduled with TRIG: one row (`TRIT_TESN = 1`) per TRIG row.
//! - ISPT is a LOCA child keyed (`LOCA_ID`, `ISPT_TOP`): one test at the top of
//!   each D sample, thinned by the rate. Sample tops are unique in a hole, so
//!   no two SPTs share a top.
//!
//! The rate is one draw per site and group, uniform in the entry's range, so a
//! site tests consistently across its holes.

use std::collections::BTreeMap;

use laterite_ags4_parity::Rng;
use laterite_ags4_validator::{DictVersion, Dictionary};

use super::build::{Hole, Numbered, Sample, Site, numbered};
use super::draw::{Key, uniform};
use super::preset::{Preset, SUITE_GROUPS, SuiteEntry};
use crate::sample_quality::SampleQuality;
use crate::synth::model::{Group, Row};

/// Computes the value of one non-KEY, non-REQUIRED heading of a test row from
/// the site, the hole and the sample under test.
pub type Produce = fn(&Site, &Hole, &Sample) -> String;

/// One heading a suite can ask for, and how its value is made.
pub struct Producer {
    pub group: &'static str,
    pub heading: &'static str,
    pub value: Produce,
}

/// Every heading beyond KEY and REQUIRED the forge can produce. The loader
/// refuses a suite heading not listed here; lowering calls `value`. Later
/// slices add entries (TRIG and TRIT results, ISPT N values) without touching
/// the loader.
pub const PRODUCERS: &[Producer] = super::results::PRODUCERS;

/// The producer for `heading` of `group`, if one is registered.
pub fn producer(group: &str, heading: &str) -> Option<&'static Producer> {
    PRODUCERS
        .iter()
        .find(|p| p.group == group && p.heading == heading)
}

/// The test type a suite ISPT row sits on.
const SPT_SAMPLE_TYPE: &str = "D";

/// The lowered suite groups, in [`SUITE_GROUPS`] order, for the entries the
/// preset lists. A group no sample was tested for is left out.
pub(super) fn groups(
    site: &Site,
    preset: &Preset,
    seed: u64,
    producers: &[Producer],
) -> Vec<Group> {
    if preset.suite.is_empty() {
        return Vec::new();
    }
    let dict = Dictionary::bundled(DictVersion::V4_2);
    let quality = SampleQuality::bundled(&dict).expect("the shipped sample-quality table loads");
    let mut out = Vec::new();
    for (index, &code) in SUITE_GROUPS.iter().enumerate() {
        let Some(entry) = preset.suite.iter().find(|e| e.group == code) else {
            continue;
        };
        if code == "TRIT" {
            continue; // written with TRIG
        }
        let permitted = quality.permitted_samp_types(code);
        let eligible = |s: &Sample| {
            if code == "ISPT" {
                s.code == SPT_SAMPLE_TYPE
            } else {
                permitted.contains(&s.code.as_str())
            }
        };
        let tested = schedule(site, entry, index, seed, &eligible);
        let ctx = Ctx {
            dict: &dict,
            site,
            producers,
        };
        let trit = preset.suite.iter().find(|e| e.group == "TRIT");
        if let Some(g) = ctx.group(entry, &tested) {
            out.push(g);
        }
        if code == "TRIG"
            && let Some(g) = trit.and_then(|t| ctx.group(t, &tested))
        {
            out.push(g);
        }
    }
    out
}

/// The tested samples: `(hole index, sample)` in file order. Each (hole, unit)
/// has its own stream, drawn once per sample in the unit whether or not the
/// sample is eligible, so eligibility never shifts another sample's draw.
fn schedule<'a>(
    site: &'a Site,
    entry: &SuiteEntry,
    group_index: usize,
    seed: u64,
    eligible: &dyn Fn(&Sample) -> bool,
) -> Vec<(usize, Numbered<'a>)> {
    let rate = site_rate(entry, seed);
    let mut out = Vec::new();
    for (hi, hole) in site.holes.iter().enumerate() {
        let mut streams: BTreeMap<&str, Rng> = BTreeMap::new();
        for n in numbered(hole) {
            let unit = n.stratum.unit.as_str();
            if !entry.units.iter().any(|u| u == unit) {
                continue;
            }
            let rng = streams.entry(unit).or_insert_with(|| {
                Key {
                    tag: "suite",
                    hole: hi as u64,
                    unit,
                    test: group_index as u64,
                }
                .rng(seed)
            });
            let draw = uniform(rng, 0.0, 1.0);
            if eligible(n.sample) && draw < rate {
                out.push((hi, n));
            }
        }
    }
    out
}

/// The entry's rate for this site: one draw in its range.
fn site_rate(entry: &SuiteEntry, seed: u64) -> f64 {
    let r = entry.rate.as_ref().expect("a scheduled entry has a rate");
    let key = Key {
        unit: &entry.group,
        ..Key::site("suite-rate")
    };
    uniform(&mut key.rng(seed), r.min, r.max)
}

struct Ctx<'a> {
    dict: &'a Dictionary<'static>,
    site: &'a Site,
    producers: &'a [Producer],
}

impl Ctx<'_> {
    /// `entry`'s group with one row per tested sample, or `None` if there are
    /// none. Columns are in dictionary order: KEY and REQUIRED headings, TRIG's
    /// two fixed codes, and the entry's produced headings.
    fn group(&self, entry: &SuiteEntry, tested: &[(usize, Numbered<'_>)]) -> Option<Group> {
        let code = entry.group.as_str();
        if tested.is_empty() {
            return None;
        }
        let cols: Vec<&str> = self
            .dict
            .group_headings(code)
            .iter()
            .copied()
            .filter(|&h| {
                let status = &self.dict.heading(code, h).expect("heading exists").status;
                status.contains("KEY")
                    || status.contains("REQUIRED")
                    || (code == "TRIG" && (h == "TRIG_TYPE" || h == "TRIG_COND"))
                    || entry.headings.iter().any(|x| x == h)
            })
            .collect();
        let entry_of = |h: &str| self.dict.heading(code, h).expect("heading exists");
        let rows = tested
            .iter()
            .map(|(hi, n)| {
                let hole = &self.site.holes[*hi];
                Row::owned(cols.iter().map(|h| self.value(code, h, hole, n)).collect())
            })
            .collect();
        Some(Group {
            code: code.to_string(),
            headings: cols.iter().map(ToString::to_string).collect(),
            units: cols.iter().map(|h| entry_of(h).unit.to_string()).collect(),
            types: cols
                .iter()
                .map(|h| entry_of(h).ags_type.to_string())
                .collect(),
            rows,
        })
    }

    fn value(&self, code: &str, heading: &str, hole: &Hole, n: &Numbered<'_>) -> String {
        let s = n.sample;
        match heading {
            "LOCA_ID" => hole.id.clone(),
            "SAMP_TOP" | "ISPT_TOP" => m2(s.top),
            "SAMP_REF" => n.reference.clone(),
            "SAMP_TYPE" => s.code.clone(),
            "SAMP_ID" => format!("{}-{}", hole.id, n.reference),
            "SPEC_REF" | "TRIT_TESN" => "1".to_string(),
            "SPEC_DPTH" => m2(round2(f64::midpoint(s.top, s.base))),
            "TRIG_TYPE" => "UU".to_string(),
            "TRIG_COND" => "UNDISTURBED".to_string(),
            _ => {
                let p = self
                    .producers
                    .iter()
                    .find(|p| p.group == code && p.heading == heading)
                    .unwrap_or_else(|| panic!("no producer makes {code}.{heading}"));
                (p.value)(self.site, hole, s)
            }
        }
    }
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
    use super::*;
    use crate::pipeline::rust_check;
    use crate::synth::emit::emit_to_string;
    use crate::synth::model::ProjectModel;
    use laterite_ags4_parity::RustResult;

    /// [`FIXTURE`] with a suite over every group: samples in WLC and LC are U
    /// or D, in RTD B or D.
    const SUITE: &str = r#"
[[suite]]
group = "LNMC"
headings = []
units = ["RTD", "WLC", "LC"]
rate = { min = 0.4, max = 0.4, basis = "test fixture" }

[[suite]]
group = "LLPL"
headings = []
units = ["WLC", "LC"]
rate = { min = 0.3, max = 0.5, basis = "test fixture" }

[[suite]]
group = "LDEN"
headings = []
units = ["WLC", "LC"]
rate = { min = 0.6, max = 0.6, basis = "test fixture" }

[[suite]]
group = "TRIG"
headings = []
units = ["WLC", "LC", "RTD"]
rate = { min = 0.5, max = 0.5, basis = "test fixture" }

[[suite]]
group = "TRIT"
headings = []

[[suite]]
group = "ISPT"
headings = []
units = ["RTD", "WLC", "LC"]
rate = { min = 0.5, max = 0.5, basis = "test fixture" }
"#;

    fn suite_fixture() -> Preset {
        Preset::from_toml(&format!("{FIXTURE}{SUITE}")).expect("suite fixture loads")
    }

    fn plain() -> Preset {
        Preset::from_toml(FIXTURE).expect("fixture loads")
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

    fn col(m: &ProjectModel, code: &str, heading: &str) -> usize {
        m.groups
            .iter()
            .find(|g| g.code == code)
            .and_then(|g| g.col(heading))
            .unwrap_or_else(|| panic!("{code}.{heading}"))
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
    fn a_suite_file_is_rust_clean_across_seeds_with_key_and_required_rows_only() {
        let p = suite_fixture();
        let dict = Dictionary::bundled(DictVersion::V4_2);
        for seed in 0..30 {
            let m = file(&p, seed, 6);
            assert!(is_clean(&m), "seed {seed}");
            for g in &m.groups {
                if !SUITE_GROUPS.contains(&g.code.as_str()) {
                    continue;
                }
                for h in &g.headings {
                    let st = &dict.heading(&g.code, h).unwrap().status;
                    let fixed = g.code == "TRIG" && (h == "TRIG_TYPE" || h == "TRIG_COND");
                    assert!(
                        st.contains("KEY") || st.contains("REQUIRED") || fixed,
                        "{}.{h}",
                        g.code
                    );
                }
            }
        }
        let m = file(&p, 1, 6);
        for g in SUITE_GROUPS {
            assert!(!rows(&m, g).is_empty(), "{g} has no rows at seed 1");
        }
    }

    #[test]
    fn a_suite_moves_no_loca_geol_or_samp_byte() {
        let (a, b) = (suite_fixture(), plain());
        for seed in 0..10 {
            let with = file(&a, seed, 5);
            let without = file(&b, seed, 5);
            for g in ["PROJ", "TRAN", "LOCA", "GEOL", "SAMP"] {
                assert_eq!(text(&with, g), text(&without, g), "{g} at seed {seed}");
            }
        }
    }

    #[test]
    fn a_preset_without_a_suite_gains_no_group() {
        let m = file(&plain(), 3, 4);
        for g in SUITE_GROUPS {
            assert!(rows(&m, g).is_empty(), "{g}");
        }
    }

    #[test]
    fn a_smaller_suite_site_is_a_prefix_of_a_larger_one() {
        let p = suite_fixture();
        for seed in 0..10 {
            let (small, large) = (file(&p, seed, 3), file(&p, seed, 7));
            for g in SUITE_GROUPS {
                let (s, l) = (text(&small, g), text(&large, g));
                assert!(l.starts_with(&s), "{g} at seed {seed}");
            }
        }
    }

    #[test]
    fn no_test_sits_on_a_sample_type_that_cannot_carry_it() {
        let p = suite_fixture();
        let dict = Dictionary::bundled(DictVersion::V4_2);
        let quality = SampleQuality::bundled(&dict).unwrap();
        for seed in 0..40 {
            let m = file(&p, seed, 6);
            for g in ["LNMC", "LLPL", "LDEN", "TRIG", "TRIT"] {
                let permitted = quality.permitted_samp_types(if g == "TRIT" { "TRIG" } else { g });
                for r in rows(&m, g) {
                    let t = &r.values[col(&m, g, "SAMP_TYPE")];
                    assert!(permitted.contains(&t.as_str()), "{g} on {t}, seed {seed}");
                }
            }
            for r in rows(&m, "ISPT") {
                let top = &r.values[col(&m, "ISPT", "ISPT_TOP")];
                let id = &r.values[col(&m, "ISPT", "LOCA_ID")];
                let on_d = rows(&m, "SAMP").iter().any(|s| {
                    s.values[col(&m, "SAMP", "LOCA_ID")] == *id
                        && s.values[col(&m, "SAMP", "SAMP_TOP")] == *top
                        && s.values[col(&m, "SAMP", "SAMP_TYPE")] == "D"
                });
                assert!(on_d, "SPT {id} at {top} is not on a D sample, seed {seed}");
            }
        }
    }

    /// Tested over eligible, as the pool over many seeds.
    fn fraction(group: &str, unit: &str, seeds: u64) -> f64 {
        let p = suite_fixture();
        let dict = Dictionary::bundled(DictVersion::V4_2);
        let permitted: Vec<String> = SampleQuality::bundled(&dict)
            .unwrap()
            .permitted_samp_types(group)
            .iter()
            .map(ToString::to_string)
            .collect();
        let (mut eligible, mut tested) = (0usize, 0usize);
        for seed in 0..seeds {
            let site = build::build(&p, seed, 6);
            for h in &site.holes {
                for n in numbered(h) {
                    if n.stratum.unit == unit && permitted.contains(&n.sample.code) {
                        eligible += 1;
                    }
                }
            }
            let m = build::lower(&site, &p, seed, Grid::Local);
            // Rows are in the file only if they have this unit's strata.
            for r in rows(&m, group) {
                let id = &r.values[col(&m, group, "LOCA_ID")];
                let top = &r.values[col(&m, group, "SAMP_TOP")];
                let h = site.holes.iter().find(|h| &h.id == id).unwrap();
                if numbered(h)
                    .any(|n| n.stratum.unit == unit && format!("{:.2}", n.sample.top) == *top)
                {
                    tested += 1;
                }
            }
        }
        assert!(eligible > 200, "{eligible} eligible samples is too few");
        tested as f64 / eligible as f64
    }

    #[test]
    fn the_tested_fraction_of_eligible_samples_follows_the_rate() {
        // LNMC takes every sample type; LDEN is rate 0.6 and LNMC 0.4, exactly.
        let f = fraction("LNMC", "LC", 60);
        assert!((f - 0.4).abs() < 0.06, "LNMC in LC: {f}");
        let f = fraction("LDEN", "LC", 60);
        assert!((f - 0.6).abs() < 0.06, "LDEN in LC: {f}");
    }

    #[test]
    fn a_range_rate_is_one_draw_per_site_inside_its_range() {
        let p = suite_fixture();
        let e = p.suite.iter().find(|e| e.group == "LLPL").unwrap();
        let mut seen = Vec::new();
        for seed in 0..50 {
            let r = site_rate(e, seed);
            assert!((0.3..=0.5).contains(&r), "{r}");
            seen.push(r);
        }
        assert!(
            seen.windows(2).any(|w| (w[0] - w[1]).abs() > 1e-9),
            "rate never varies"
        );
    }

    #[test]
    fn test_rows_carry_the_samp_key_with_spec_ref_and_mid_depth() {
        let p = suite_fixture();
        let m = file(&p, 2, 6);
        let samp: Vec<Vec<String>> = text(&m, "SAMP");
        for g in ["LNMC", "LLPL", "LDEN", "TRIG", "TRIT"] {
            let (top, base) = (col(&m, "SAMP", "SAMP_TOP"), col(&m, "SAMP", "SAMP_BASE"));
            for r in rows(&m, g) {
                let v = |h: &str| r.values[col(&m, g, h)].clone();
                let s = samp
                    .iter()
                    .find(|s| {
                        s[0] == v("LOCA_ID") && s[top] == v("SAMP_TOP") && s[2] == v("SAMP_REF")
                    })
                    .unwrap_or_else(|| panic!("{g}: no parent SAMP row"));
                assert_eq!(s[3], v("SAMP_TYPE"));
                assert_eq!(s[4], v("SAMP_ID"));
                assert_eq!(v("SPEC_REF"), "1");
                let mid = f64::midpoint(s[top].parse().unwrap(), s[base].parse().unwrap());
                let got = v("SPEC_DPTH").parse::<f64>().unwrap();
                assert!((got - mid).abs() <= 0.0051, "{g}: {got} vs {mid}");
            }
        }
    }

    #[test]
    fn trig_has_codes_and_one_trit_row_each_and_the_codes_reach_abbr() {
        let m = file(&suite_fixture(), 1, 6);
        let (trig, trit) = (rows(&m, "TRIG"), rows(&m, "TRIT"));
        assert!(!trig.is_empty());
        assert_eq!(trig.len(), trit.len());
        for r in trig {
            assert_eq!(r.values[col(&m, "TRIG", "TRIG_TYPE")], "UU");
            assert_eq!(r.values[col(&m, "TRIG", "TRIG_COND")], "UNDISTURBED");
        }
        for r in trit {
            assert_eq!(r.values[col(&m, "TRIT", "TRIT_TESN")], "1");
        }
        let abbr = rows(&m, "ABBR");
        let (h, c) = (col(&m, "ABBR", "ABBR_HDNG"), col(&m, "ABBR", "ABBR_CODE"));
        for (heading, code) in [("TRIG_TYPE", "UU"), ("TRIG_COND", "UNDISTURBED")] {
            assert!(
                abbr.iter()
                    .any(|r| r.values[h] == heading && r.values[c] == code),
                "{heading} {code} missing from ABBR"
            );
        }
    }

    #[test]
    fn every_new_group_has_a_unique_key_and_its_parents() {
        let dict = Dictionary::bundled(DictVersion::V4_2);
        for seed in 0..20 {
            let m = file(&suite_fixture(), seed, 6);
            for g in SUITE_GROUPS {
                let group = m.groups.iter().find(|x| x.code == *g);
                let Some(group) = group else { continue };
                let keys: Vec<usize> = dict
                    .group_headings(g)
                    .iter()
                    .filter(|h| dict.heading(g, h).unwrap().status.contains("KEY"))
                    .map(|h| group.col(h).unwrap())
                    .collect();
                let mut seen = std::collections::BTreeSet::new();
                for r in &group.rows {
                    let k: Vec<&String> = keys.iter().map(|&i| &r.values[i]).collect();
                    assert!(seen.insert(k.clone()), "{g} repeats {k:?}, seed {seed}");
                }
            }
            assert!(is_clean(&m), "seed {seed}");
        }
    }

    #[test]
    fn no_two_spts_in_a_hole_share_a_top() {
        for seed in 0..30 {
            let m = file(&suite_fixture(), seed, 6);
            let mut seen = std::collections::BTreeSet::new();
            for r in rows(&m, "ISPT") {
                let k = (
                    r.values[col(&m, "ISPT", "LOCA_ID")].clone(),
                    r.values[col(&m, "ISPT", "ISPT_TOP")].clone(),
                );
                assert!(seen.insert(k.clone()), "{k:?} repeats, seed {seed}");
            }
        }
    }

    #[test]
    fn a_registered_producer_adds_its_column_in_dictionary_order() {
        fn marker(_: &Site, h: &Hole, s: &Sample) -> String {
            format!("{}/{:.2}", h.id, s.top)
        }
        let heading = {
            let d = Dictionary::bundled(DictVersion::V4_2);
            let other: Vec<&str> = d
                .group_headings("LLPL")
                .iter()
                .copied()
                .filter(|h| {
                    let s = &d.heading("LLPL", h).unwrap().status;
                    !s.contains("KEY") && !s.contains("REQUIRED")
                })
                .collect();
            *other.first().expect("LLPL has an optional heading")
        };
        let producers = [Producer {
            group: "LLPL",
            heading,
            value: marker,
        }];
        let mut p = suite_fixture();
        p.suite
            .iter_mut()
            .find(|e| e.group == "LLPL")
            .unwrap()
            .headings = vec![heading.to_string()];
        let site = build::build(&p, 1, 6);
        let groups = groups(&site, &p, 1, &producers);
        let llpl = groups.iter().find(|g| g.code == "LLPL").unwrap();
        assert_eq!(llpl.headings.last().map(String::as_str), Some(heading));
        let id = llpl.col("LOCA_ID").unwrap();
        let top = llpl.col("SAMP_TOP").unwrap();
        let at = llpl.col(heading).unwrap();
        for r in &llpl.rows {
            assert_eq!(r.values[at], format!("{}/{}", r.values[id], r.values[top]));
        }
    }
}
