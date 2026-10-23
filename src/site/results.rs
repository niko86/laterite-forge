//! Index-property results: the values of LNMC, LLPL and LDEN rows (ADR 0001,
//! "Values and descriptions").
//!
//! A result is the property model's value at the specimen's depth times
//! (1 + COV x e), with e a standard normal truncated at +-3, and with rare
//! outliers beyond the property's range. Every draw is keyed per (hole, unit,
//! sample, quantity) under tags of its own, so a result consumes no stream the
//! geometry, sampling, description or property draws use, and the same result
//! is recomputed exactly wherever it is needed.
//!
//! That recomputation is how the couplings across headings hold. Each group
//! has one function that works out the whole result (LL, PL and the PI between
//! them; BDEN, MC and the DDEN derived from them), and each heading's producer
//! formats one part of it. PI is therefore LL - PL of the written LL and PL, and
//! DDEN is BDEN / (1 + MC/100) of the written BDEN and MC, whichever heading
//! the suite lists and whichever is computed first. LNMC and LDEN are separate
//! tests, so their moisture contents scatter independently.
//!
//! Written precision follows each heading's TYPE in the bundled dictionary:
//! LL, PL, PI, and both MC headings (X, written as a whole number, the usual
//! reporting) are whole numbers; BDEN and DDEN are 2DP, in Mg/m3.

use super::build::{Hole, Sample, Site};
use super::draw::{Key, uniform};
use super::properties::{Band, Evaluated, IndexQuantity, OUTLIER_OVERSHOOT, beyond_range};
use super::suite::Producer;

/// Coefficient of variation of a density result: the midpoint of 1-10 %
/// (Decoding Eurocode 7, Bond and Harris, 2008, ch. 5, s5.3.1, p.136, after
/// Schneider 1997 and Phoon and Kulhawy 1999).
pub const DENSITY_COV: f64 = 0.055;
/// Coefficient of variation of a moisture content result. Basis: forge
/// judgement.
pub const MOISTURE_COV: f64 = 0.075;
/// Coefficient of variation of a liquid or plastic limit result. Basis: forge
/// judgement.
pub const LIMIT_COV: f64 = 0.075;
/// The chance a result is an outlier. Basis: forge judgement.
pub const RESULT_OUTLIER_RATE: f64 = 0.02;
/// Scatter is a standard normal truncated at this many standard deviations.
const TRUNCATION: f64 = 3.0;

/// A standard normal draw truncated at +-[`TRUNCATION`]: Box-Muller on two
/// uniforms, redrawn while it falls beyond the truncation (about 0.27 % of
/// draws). The redraws continue the same stream, so the result is a pure
/// function of the stream.
pub(super) fn truncated_normal(rng: &mut laterite_ags4_parity::Rng) -> f64 {
    loop {
        // 1 - u is in (0, 1], so the logarithm is finite.
        let u1 = 1.0 - uniform(rng, 0.0, 1.0);
        let u2 = uniform(rng, 0.0, 1.0);
        let z = (-2.0 * u1.ln()).sqrt() * (std::f64::consts::TAU * u2).cos();
        if z.abs() <= TRUNCATION {
            return z;
        }
    }
}

/// One specimen: where it is and what the property model says there.
pub(super) struct Specimen<'a> {
    pub hole_index: usize,
    pub unit: &'a str,
    /// The sample's position among all the hole's samples, top-down.
    pub sample_index: u64,
    pub model: Evaluated,
}

/// The specimen of `sample` (a reference into `hole`, which is in `site`), at the mid-depth `SPEC_DPTH` is written
/// with. `None` when the hole has no such sample or the unit has no
/// properties.
pub(super) fn specimen<'a>(site: &'a Site, hole: &Hole, sample: &Sample) -> Option<Specimen<'a>> {
    let hole_index = site.holes.iter().position(|h| h.id == hole.id)?;
    let mut index = 0u64;
    for stratum in &site.holes[hole_index].strata {
        for x in &stratum.samples {
            if std::ptr::eq(x, sample) {
                let depth = (f64::midpoint(sample.top, sample.base) * 100.0).round() / 100.0;
                return Some(Specimen {
                    hole_index,
                    unit: stratum.unit.as_str(),
                    sample_index: index,
                    model: site.properties_at(hole_index, depth)?,
                });
            }
            index += 1;
        }
    }
    None
}

/// A scattered value, before it is rounded to its written precision.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct Scattered {
    pub value: f64,
    pub outlier: bool,
}

/// The result of `group`'s `name` quantity for `spec`: `model` x (1 + `cov` x
/// e), or an outlier beyond `band`.
///
/// The scatter and the outlier come from two streams, keyed
/// `result:<group>:<name>` and `result:<group>:<name>:outlier` on (hole, unit,
/// sample), each always consumed in the same order so the outlier rate changes
/// only the results whose status changes.
pub(super) fn scatter(
    seed: u64,
    spec: &Specimen<'_>,
    (group, name): (&str, &str),
    band: Option<Band>,
    model: f64,
    cov: f64,
) -> Scattered {
    let tag = format!("result:{group}:{name}");
    let key = |tag: &str| {
        Key {
            tag,
            hole: spec.hole_index as u64,
            unit: spec.unit,
            test: spec.sample_index,
        }
        .rng(seed)
    };
    let e = truncated_normal(&mut key(&tag));
    let mut rng = key(&format!("{tag}:outlier"));
    let chance = uniform(&mut rng, 0.0, 1.0);
    let overshoot = uniform(&mut rng, OUTLIER_OVERSHOOT.0, OUTLIER_OVERSHOOT.1);
    match band {
        Some(b) if chance < RESULT_OUTLIER_RATE => Scattered {
            value: beyond_range(model, (b.lo, b.hi), overshoot, &b.domain),
            outlier: true,
        },
        _ => Scattered {
            value: model * (1.0 + cov * e),
            outlier: false,
        },
    }
}

/// A whole number, rounded half away from zero.
#[allow(clippy::cast_possible_truncation)]
fn whole(v: f64) -> i64 {
    v.round() as i64
}

/// One quantity of `spec` scattered under `group`.
fn quantity(
    site: &Site,
    spec: &Specimen<'_>,
    group: &str,
    (name, q, cov): (&str, IndexQuantity, f64),
    model: f64,
) -> f64 {
    let props = &site.properties;
    scatter(
        props.seed(),
        spec,
        (group, name),
        props.band(spec.unit, q),
        model,
        cov,
    )
    .value
}

/// Moisture content, percent, whole: the LNMC result.
fn lnmc_mc(site: &Site, hole: &Hole, sample: &Sample) -> Option<i64> {
    let spec = specimen(site, hole, sample)?;
    let w = quantity(
        site,
        &spec,
        "LNMC",
        (
            "moisture_content",
            IndexQuantity::MoistureContent,
            MOISTURE_COV,
        ),
        spec.model.moisture_content_percent,
    );
    Some(whole(w).max(0))
}

/// The written liquid limit, plastic limit and the PI between them, percent,
/// all whole.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct LlPl {
    pub ll: i64,
    pub pl: i64,
}

impl LlPl {
    pub fn pi(self) -> i64 {
        self.ll - self.pl
    }
}

/// The whole LLPL result. LL and PL scatter independently and round to whole
/// numbers; PL is then held under LL (LL at least 2, PL between 1 and LL - 1)
/// so that PI = LL - PL of the written values is positive. `None` where the
/// unit has no fine properties.
pub(super) fn llpl(site: &Site, hole: &Hole, sample: &Sample) -> Option<LlPl> {
    let spec = specimen(site, hole, sample)?;
    let fine = spec.model.fine.as_ref()?;
    let ll = quantity(
        site,
        &spec,
        "LLPL",
        ("liquid_limit", IndexQuantity::LiquidLimit, LIMIT_COV),
        fine.liquid_limit_percent,
    );
    let pl = quantity(
        site,
        &spec,
        "LLPL",
        ("plastic_limit", IndexQuantity::PlasticLimit, LIMIT_COV),
        fine.plastic_limit_percent,
    );
    let ll = whole(ll).max(2);
    let pl = whole(pl).clamp(1, ll - 1);
    Some(LlPl { ll, pl })
}

/// The written bulk density (hundredths of Mg/m3), moisture content (whole
/// percent) and the dry density derived from them (2DP, Mg/m3).
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct Lden {
    pub bden_hundredths: i64,
    pub mc: i64,
}

impl Lden {
    pub fn bden(self) -> f64 {
        self.bden_hundredths as f64 / 100.0
    }

    /// DDEN = BDEN / (1 + MC/100), of the written BDEN and MC.
    pub fn dden(self) -> f64 {
        self.bden() / (1.0 + self.mc as f64 / 100.0)
    }
}

/// The whole LDEN result. BDEN is the model's bulk density (Mg/m3) scattered
/// and written to 2DP (at least 0.01); MC scatters from the model's moisture
/// content independently of any LNMC result.
pub(super) fn lden(site: &Site, hole: &Hole, sample: &Sample) -> Option<Lden> {
    let spec = specimen(site, hole, sample)?;
    let bden = quantity(
        site,
        &spec,
        "LDEN",
        ("bulk_density", IndexQuantity::BulkDensity, DENSITY_COV),
        spec.model.bulk_density_kg_m3,
    );
    let mc = quantity(
        site,
        &spec,
        "LDEN",
        (
            "moisture_content",
            IndexQuantity::MoistureContent,
            MOISTURE_COV,
        ),
        spec.model.moisture_content_percent,
    );
    // kg/m3 -> Mg/m3 -> hundredths.
    Some(Lden {
        bden_hundredths: whole(bden / 10.0).max(1),
        mc: whole(mc).max(0),
    })
}

fn text<T: std::fmt::Display>(v: Option<T>) -> String {
    v.map(|v| v.to_string()).unwrap_or_default()
}

/// The producers this module registers (see [`super::suite::PRODUCERS`]). A
/// heading the specimen's unit has no value for (LL, PL and PI in a unit with
/// no fine properties) is written blank.
pub(super) const PRODUCERS: &[Producer] = &[
    Producer {
        group: "LNMC",
        heading: "LNMC_MC",
        value: |s, h, x| text(lnmc_mc(s, h, x)),
    },
    Producer {
        group: "LLPL",
        heading: "LLPL_LL",
        value: |s, h, x| text(llpl(s, h, x).map(|r| r.ll)),
    },
    Producer {
        group: "LLPL",
        heading: "LLPL_PL",
        value: |s, h, x| text(llpl(s, h, x).map(|r| r.pl)),
    },
    Producer {
        group: "LLPL",
        heading: "LLPL_PI",
        value: |s, h, x| text(llpl(s, h, x).map(LlPl::pi)),
    },
    Producer {
        group: "LDEN",
        heading: "LDEN_BDEN",
        value: |s, h, x| text(lden(s, h, x).map(|r| format!("{:.2}", r.bden()))),
    },
    Producer {
        group: "LDEN",
        heading: "LDEN_DDEN",
        value: |s, h, x| text(lden(s, h, x).map(|r| format!("{:.2}", r.dden()))),
    },
    Producer {
        group: "LDEN",
        heading: "LDEN_MC",
        value: |s, h, x| text(lden(s, h, x).map(|r| r.mc)),
    },
];

#[cfg(test)]
mod tests {
    use super::super::build::{self, Grid, tests::FIXTURE};
    use super::super::preset::Preset;
    use super::*;
    use crate::pipeline::rust_check;
    use crate::synth::emit::emit_to_string;
    use crate::synth::model::ProjectModel;
    use laterite_ags4_parity::RustResult;
    use std::fmt::Write as _;

    fn suite(headings: &[(&str, &[&str])]) -> String {
        let mut out = String::new();
        for (group, hs) in headings {
            let list: Vec<String> = hs.iter().map(|h| format!("\"{h}\"")).collect();
            let _ = write!(
                out,
                "\n[[suite]]\ngroup = \"{group}\"\nheadings = [{}]\nunits = [\"RTD\", \"WLC\", \"LC\"]\nrate = {{ min = 1.0, max = 1.0, basis = \"test fixture\" }}\n",
                list.join(", ")
            );
        }
        out
    }

    const ALL: &[(&str, &[&str])] = &[
        ("LNMC", &["LNMC_MC"]),
        ("LLPL", &["LLPL_LL", "LLPL_PL", "LLPL_PI"]),
        ("LDEN", &["LDEN_BDEN", "LDEN_DDEN", "LDEN_MC"]),
    ];

    fn preset(headings: &[(&str, &[&str])]) -> Preset {
        Preset::from_toml(&format!("{FIXTURE}{}", suite(headings))).expect("preset loads")
    }

    fn file(p: &Preset, seed: u64, holes: usize) -> ProjectModel {
        let site = build::build(p, seed, holes);
        build::lower(&site, p, seed, Grid::Local)
    }

    fn cells(m: &ProjectModel, group: &str, heading: &str) -> Vec<String> {
        let Some(g) = m.groups.iter().find(|g| g.code == group) else {
            return Vec::new();
        };
        let c = g
            .col(heading)
            .unwrap_or_else(|| panic!("{group}.{heading}"));
        g.rows.iter().map(|r| r.values[c].clone()).collect()
    }

    fn int(s: &str) -> i64 {
        s.parse().unwrap_or_else(|_| panic!("{s:?} is not whole"))
    }

    #[test]
    fn llpl_rows_keep_pi_equal_to_ll_minus_pl_and_pl_under_ll() {
        let p = preset(ALL);
        let mut rows = 0;
        for seed in 0..40 {
            let m = file(&p, seed, 6);
            let (ll, pl, pi) = (
                cells(&m, "LLPL", "LLPL_LL"),
                cells(&m, "LLPL", "LLPL_PL"),
                cells(&m, "LLPL", "LLPL_PI"),
            );
            for i in 0..ll.len() {
                if ll[i].is_empty() {
                    // A unit with no fine properties has no limits.
                    assert!(pl[i].is_empty() && pi[i].is_empty());
                    continue;
                }
                let (l, p, i_) = (int(&ll[i]), int(&pl[i]), int(&pi[i]));
                assert!(p < l, "seed {seed}: PL {p} !< LL {l}");
                assert_eq!(i_, l - p, "seed {seed}");
                rows += 1;
            }
        }
        assert!(rows > 300, "{rows} LLPL rows is too few");
    }

    #[test]
    fn lden_rows_derive_dden_from_the_written_bden_and_mc() {
        let p = preset(ALL);
        let mut rows = 0;
        for seed in 0..40 {
            let m = file(&p, seed, 6);
            let (b, mc, d) = (
                cells(&m, "LDEN", "LDEN_BDEN"),
                cells(&m, "LDEN", "LDEN_MC"),
                cells(&m, "LDEN", "LDEN_DDEN"),
            );
            for i in 0..b.len() {
                let bden: f64 = b[i].parse().unwrap();
                let want = format!("{:.2}", bden / (1.0 + int(&mc[i]) as f64 / 100.0));
                assert_eq!(d[i], want, "seed {seed}");
                assert!(bden > 0.0);
                rows += 1;
            }
        }
        assert!(rows > 300, "{rows} LDEN rows is too few");
    }

    /// Whichever headings a suite lists, a shared cell is the same: PI from
    /// the written LL and PL when LL and PL are not listed, DDEN likewise.
    #[test]
    fn couplings_do_not_depend_on_which_headings_are_listed() {
        let full = preset(ALL);
        let pi_only = preset(&[("LLPL", &["LLPL_PI"]), ("LDEN", &["LDEN_DDEN"])]);
        let ll_only = preset(&[("LLPL", &["LLPL_LL"]), ("LDEN", &["LDEN_MC"])]);
        for seed in 0..10 {
            let (a, b, c) = (
                file(&full, seed, 5),
                file(&pi_only, seed, 5),
                file(&ll_only, seed, 5),
            );
            for (g, h, other) in [
                ("LLPL", "LLPL_PI", &b),
                ("LDEN", "LDEN_DDEN", &b),
                ("LLPL", "LLPL_LL", &c),
                ("LDEN", "LDEN_MC", &c),
            ] {
                assert_eq!(cells(&a, g, h), cells(other, g, h), "{h} at seed {seed}");
            }
        }
    }

    #[test]
    fn listing_headings_moves_no_other_cell() {
        let bare = preset(&[("LNMC", &[]), ("LLPL", &[]), ("LDEN", &[])]);
        let full = preset(ALL);
        for seed in 0..10 {
            let (a, b) = (file(&bare, seed, 5), file(&full, seed, 5));
            for (g, h) in [
                ("LNMC", "SAMP_TOP"),
                ("LLPL", "SPEC_DPTH"),
                ("LDEN", "SAMP_ID"),
            ] {
                assert_eq!(cells(&a, g, h), cells(&b, g, h), "{g}.{h}, seed {seed}");
            }
            for g in ["PROJ", "LOCA", "GEOL", "SAMP"] {
                let rows = |m: &ProjectModel| {
                    m.groups
                        .iter()
                        .find(|x| x.code == g)
                        .map(|x| x.rows.iter().map(|r| r.values.clone()).collect::<Vec<_>>())
                };
                assert_eq!(rows(&a), rows(&b), "{g}, seed {seed}");
            }
        }
    }

    type Ratios = Vec<(&'static str, f64, Vec<(f64, bool)>)>;

    /// Every (specimen, quantity) scattered result over many sites:
    /// (ratio to the model value, outlier?) per quantity.
    fn ratios() -> Ratios {
        let p = preset(ALL);
        let mut out: Ratios = vec![
            ("LNMC", MOISTURE_COV, Vec::new()),
            ("LL", LIMIT_COV, Vec::new()),
            ("PL", LIMIT_COV, Vec::new()),
            ("BDEN", DENSITY_COV, Vec::new()),
            ("LDEN MC", MOISTURE_COV, Vec::new()),
        ];
        for seed in 0..60 {
            let site = build::build(&p, seed, 8);
            for h in &site.holes {
                for s in h.strata.iter().flat_map(|s| s.samples.iter()) {
                    let Some(spec) = specimen(&site, h, s) else {
                        continue;
                    };
                    let seed_p = site.properties.seed();
                    let m = &spec.model;
                    let mut put = |i: usize, g: &str, n: &str, q: IndexQuantity, v: f64| {
                        let cov = out[i].1;
                        let r = scatter(
                            seed_p,
                            &spec,
                            (g, n),
                            site.properties.band(spec.unit, q),
                            v,
                            cov,
                        );
                        out[i].2.push((r.value / v, r.outlier));
                    };
                    put(
                        0,
                        "LNMC",
                        "moisture_content",
                        IndexQuantity::MoistureContent,
                        m.moisture_content_percent,
                    );
                    if let Some(f) = &m.fine {
                        put(
                            1,
                            "LLPL",
                            "liquid_limit",
                            IndexQuantity::LiquidLimit,
                            f.liquid_limit_percent,
                        );
                        put(
                            2,
                            "LLPL",
                            "plastic_limit",
                            IndexQuantity::PlasticLimit,
                            f.plastic_limit_percent,
                        );
                    }
                    put(
                        3,
                        "LDEN",
                        "bulk_density",
                        IndexQuantity::BulkDensity,
                        m.bulk_density_kg_m3,
                    );
                    put(
                        4,
                        "LDEN",
                        "moisture_content",
                        IndexQuantity::MoistureContent,
                        m.moisture_content_percent,
                    );
                }
            }
        }
        out
    }

    #[test]
    fn scatter_has_the_default_mean_and_cov_and_outliers_the_default_rate() {
        for (name, cov, rs) in ratios() {
            assert!(rs.len() > 1500, "{name}: {} results is too few", rs.len());
            let ok: Vec<f64> = rs.iter().filter(|r| !r.1).map(|r| r.0).collect();
            let n = ok.len() as f64;
            let mean = ok.iter().sum::<f64>() / n;
            let var = ok.iter().map(|r| (r - mean).powi(2)).sum::<f64>() / (n - 1.0);
            // A standard normal truncated at +-3 has sd 0.9866.
            let want = cov * 0.9866;
            assert!((mean - 1.0).abs() < 0.01, "{name}: mean {mean}");
            assert!(
                (var.sqrt() - want).abs() < 0.08 * want,
                "{name}: COV {} vs {want}",
                var.sqrt()
            );
            let rate = rs.iter().filter(|r| r.1).count() as f64 / rs.len() as f64;
            assert!(
                (rate - RESULT_OUTLIER_RATE).abs() < 0.008,
                "{name}: outlier rate {rate}"
            );
        }
    }

    #[test]
    fn the_density_cov_is_the_midpoint_of_one_to_ten_percent() {
        assert!((DENSITY_COV - f64::midpoint(0.01, 0.10)).abs() < 1e-12);
    }

    #[test]
    fn the_truncated_normal_stays_inside_three_and_reaches_near_it() {
        let mut rng = Key::site("t").rng(5);
        let zs: Vec<f64> = (0..200_000).map(|_| truncated_normal(&mut rng)).collect();
        assert!(zs.iter().all(|z| z.abs() <= 3.0));
        assert!(zs.iter().any(|z| z.abs() > 2.9));
        let mean = zs.iter().sum::<f64>() / zs.len() as f64;
        let sd = (zs.iter().map(|z| (z - mean).powi(2)).sum::<f64>() / zs.len() as f64).sqrt();
        assert!(
            mean.abs() < 0.01 && (sd - 0.9866).abs() < 0.01,
            "{mean} {sd}"
        );
    }

    #[test]
    fn an_outlier_lands_beyond_the_range_by_a_quarter_to_three_quarters_of_its_width() {
        let p = preset(ALL);
        let mut seen = 0;
        for seed in 0..60 {
            let site = build::build(&p, seed, 8);
            for h in &site.holes {
                for s in h.strata.iter().flat_map(|s| s.samples.iter()) {
                    let Some(spec) = specimen(&site, h, s) else {
                        continue;
                    };
                    let Some(f) = &spec.model.fine else { continue };
                    let band = site
                        .properties
                        .band(spec.unit, IndexQuantity::PlasticLimit)
                        .unwrap();
                    let r = scatter(
                        site.properties.seed(),
                        &spec,
                        ("LLPL", "plastic_limit"),
                        Some(band),
                        f.plastic_limit_percent,
                        LIMIT_COV,
                    );
                    if !r.outlier {
                        continue;
                    }
                    seen += 1;
                    let w = band.hi - band.lo;
                    let out = (band.lo - r.value).max(r.value - band.hi);
                    // Low outliers can reach the physical floor and clamp.
                    assert!(
                        r.value <= band.lo - 0.25 * w + 1e-9
                            && r.value >= band.lo - 0.75 * w - 1e-9
                            || r.value >= band.hi + 0.25 * w - 1e-9
                                && r.value <= band.hi + 0.75 * w + 1e-9
                            || r.value < 1e-5,
                        "{} outside {}..{} (out {out})",
                        r.value,
                        band.lo,
                        band.hi
                    );
                }
            }
        }
        assert!(seen > 10, "{seen} outliers is too few");
    }

    #[test]
    fn a_file_with_every_result_heading_is_rust_clean_and_parses_as_its_types() {
        let p = preset(ALL);
        let dir = tempfile::tempdir().unwrap();
        for seed in 0..25 {
            let m = file(&p, seed, 6);
            let path = dir.path().join("site.ags");
            std::fs::write(&path, emit_to_string(&m)).unwrap();
            assert_eq!(rust_check(&path).0, RustResult::Clean, "seed {seed}");
            for (g, h, dp) in [
                ("LNMC", "LNMC_MC", 0),
                ("LLPL", "LLPL_LL", 0),
                ("LLPL", "LLPL_PL", 0),
                ("LLPL", "LLPL_PI", 0),
                ("LDEN", "LDEN_BDEN", 2),
                ("LDEN", "LDEN_DDEN", 2),
                ("LDEN", "LDEN_MC", 0),
            ] {
                for c in cells(&m, g, h) {
                    if c.is_empty() {
                        assert_eq!(g, "LLPL", "{h} blank at seed {seed}");
                        continue;
                    }
                    let v: f64 = c.parse().unwrap_or_else(|_| panic!("{h} {c:?}"));
                    assert!(v >= 0.0);
                    let places = c.split_once('.').map_or(0, |(_, f)| f.len());
                    assert_eq!(places, dp, "{h} {c}");
                }
            }
        }
    }

    #[test]
    fn results_are_deterministic() {
        let p = preset(ALL);
        let a = emit_to_string(&file(&p, 4, 5));
        assert_eq!(a, emit_to_string(&file(&p, 4, 5)));
    }
}
