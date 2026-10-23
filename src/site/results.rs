//! Index-property results: the values of LNMC, LLPL and LDEN rows (ADR 0001,
//! "Values and descriptions").
//!
//! A result is the property model's value at the specimen's depth times a
//! lognormal factor with mean 1 and the quantity's COV (see
//! [`lognormal_factor`]), and with rare outliers beyond the property's range.
//! Lognormal, not normal, so a result is never zero or negative however large
//! the COV, and skews right as SPT N and strength data do. Every draw is keyed per (hole, unit,
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
//! Strength results follow the same pattern. TRIT's undrained strength is the
//! model's cu at the specimen scattered with the strength COV, and the deviator
//! stress at failure is twice the written value. The cell pressure is the
//! total overburden at the specimen. TRIT's density and moisture content scatter
//! under TRIT keys of their own. An SPT N is cu at the test's top (scattered
//! under an ISPT key) over f1 in a unit with the `spt.stroud_butler` relation,
//! and otherwise the model's N scattered.
//!
//! Written precision follows each heading's TYPE in the bundled dictionary:
//! LL, PL, PI, both MC headings (X, written as a whole number, the usual
//! reporting), the strengths, cell pressure and N are whole numbers; BDEN and
//! DDEN are 2DP, in Mg/m3.

use super::build::{Hole, Sample, Site};
use super::draw::{Key, uniform};
use super::properties::{
    Band, Evaluated, IndexQuantity, OUTLIER_OVERSHOOT, UNDRAINED_STRENGTH_KPA, beyond_range,
};
use super::suite::Producer;

/// Coefficient of variation of a density result: the midpoint of 1-10 %
/// (Decoding Eurocode 7, Bond and Harris, 2008, ch. 5, s5.3.1, p.136, after
/// Schneider 1997 and Phoon and Kulhawy 1999).
pub const DENSITY_COV: f64 = 0.055;
/// Coefficient of variation of a moisture content result. Calibrated (forge
/// analysis, #48) against london-basin v3 over 400 seeds: the pooled `LNMC_MC`
/// COV is 0.118 in LC and 0.119 in WLC, just under the 0.12 of the BGS crawl
/// (London Clay w, n = 217) with Skempton and Henkel 1957 giving the mean.
/// Inside the site-specific clay range of ISSMGE-TC304 (2021),
/// doi:10.53243/R0001, Table 1.2 (w 3.5-46 %).
pub const MOISTURE_COV: f64 = 0.115;
/// Coefficient of variation of a liquid or plastic limit result. Calibrated
/// (forge analysis, #48) against london-basin v3 over 400 seeds: the pooled
/// `LLPL_PL` COV is 0.089 in LC and 0.090 in WLC, at the 0.09 of the BGS crawl
/// and Cooling and Skempton (9.4 %, in ISSMGE-TC304 2021, Appendix), and the
/// written PI spans P10-P90 34-54 against 33-54 (Jones and Terrington 2011).
/// Inside ISSMGE-TC304 (2021), doi:10.53243/R0001, Table 1.2 (LL 3.4-39 %, PL
/// 2.9-38.1 %).
pub const LIMIT_COV: f64 = 0.088;
/// Coefficient of variation of an undrained strength result. Calibrated (forge
/// analysis, #48) against london-basin v3 over 400 seeds: LC `TRIT_CU` over the
/// Brosse et al. 2025 mean line (83.1 + 7.4z kPa) has COV 0.343, against
/// their sigma/mu of about 0.34 (doi:10.3850/GRF-25280825_isgsr-185-P110-cd),
/// and WLC `TRIT_CU` pools to COV 0.44 (their weathered row, 0.44). Inside
/// ISSMGE-TC304 (2021), doi:10.53243/R0001, Table 1.2 (su 6-56 %).
pub const STRENGTH_COV: f64 = 0.345;
/// Coefficient of variation of an SPT N result in a unit with its own N.
/// Calibrated (forge analysis, #48) against london-basin v3 over 400 seeds:
/// RTD `ISPT_NVAL` pools to mean 27.1, COV 0.47, inside the 0.40-0.50 between
/// the BGS crawl (0.39) and Royse et al. 2009 (about 0.5). Inside the sand
/// range of ISSMGE-TC304 (2021), doi:10.53243/R0001, Table 1.3 (SPT-N
/// 18.4-62 %).
pub const SPT_COV: f64 = 0.42;
/// The TRIT headings that need the unit to have a `strength`.
pub const TRIT_STRENGTH_HEADINGS: &[&str] = &["TRIT_CU", "TRIT_DEVF"];
/// The LLPL headings that need the unit to have `fine` properties.
pub const LLPL_HEADINGS: &[&str] = &["LLPL_LL", "LLPL_PL", "LLPL_PI"];
/// The chance a result is an outlier. Basis: forge judgement.
pub const RESULT_OUTLIER_RATE: f64 = 0.02;
/// The scatter draw is a standard normal truncated at this many standard
/// deviations.
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

/// The scatter factor for a standard normal draw `e`: lognormal with mean 1
/// and coefficient of variation `cov`, exp(s e - s^2 / 2) with s^2 =
/// ln(1 + cov^2). Truncating `e` at +-[`TRUNCATION`] trims the upper tail,
/// moving the mean under 0.5 % and the COV under 3 % at a COV of 0.42.
pub(super) fn lognormal_factor(cov: f64, e: f64) -> f64 {
    let s = cov.mul_add(cov, 1.0).ln().sqrt();
    (s * e - s * s / 2.0).exp()
}

/// One specimen: where it is and what the property model says there.
pub(super) struct Specimen<'a> {
    pub hole_index: usize,
    /// Metres below ground, as written.
    pub depth: f64,
    pub unit: &'a str,
    /// The sample's position among all the hole's samples, top-down.
    pub sample_index: u64,
    pub model: Evaluated,
}

/// The specimen of `sample` (a reference into `hole`, which is in `site`), at
/// the mid-depth `SPEC_DPTH` is written with. `None` when the hole has no such
/// sample or the unit has no properties.
pub(super) fn specimen<'a>(site: &'a Site, hole: &Hole, sample: &Sample) -> Option<Specimen<'a>> {
    specimen_at(site, hole, sample, f64::midpoint(sample.top, sample.base))
}

/// The specimen of `sample` at `depth`, rounded to the centimetre it is
/// written with.
fn specimen_at<'a>(
    site: &'a Site,
    hole: &Hole,
    sample: &Sample,
    depth: f64,
) -> Option<Specimen<'a>> {
    let hole_index = site.holes.iter().position(|h| h.id == hole.id)?;
    let mut index = 0u64;
    for stratum in &site.holes[hole_index].strata {
        for x in &stratum.samples {
            if std::ptr::eq(x, sample) {
                let depth = (depth * 100.0).round() / 100.0;
                return Some(Specimen {
                    hole_index,
                    depth,
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

/// The result of `group`'s `name` quantity for `spec`: `model` x
/// [`lognormal_factor`], or an outlier beyond `band`.
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
            value: model * lognormal_factor(cov, e),
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

/// The band cu spans at the specimen, for its outliers.
fn strength_band(range: (f64, f64)) -> Band {
    Band {
        lo: range.0,
        hi: range.1,
        domain: UNDRAINED_STRENGTH_KPA,
    }
}

/// The scattered cu of the specimen, kPa, under `group`'s own keys.
fn scattered_cu(site: &Site, spec: &Specimen<'_>, group: &str) -> Option<f64> {
    let cu = spec.model.strength?;
    Some(
        scatter(
            site.properties.seed(),
            spec,
            (group, "undrained_strength"),
            Some(strength_band(cu.range_kpa)),
            cu.cu_kpa,
            STRENGTH_COV,
        )
        .value,
    )
}

/// The written TRIT undrained strength, kPa, whole and at least 1.
pub(super) fn trit_cu(site: &Site, hole: &Hole, sample: &Sample) -> Option<i64> {
    let spec = specimen(site, hole, sample)?;
    Some(whole(scattered_cu(site, &spec, "TRIT")?).max(1))
}

/// The written TRIT cell pressure, kPa, whole: the total vertical stress at
/// the specimen.
fn trit_cell(site: &Site, hole: &Hole, sample: &Sample) -> Option<i64> {
    let spec = specimen(site, hole, sample)?;
    let stress = site.stress_at(spec.hole_index, spec.depth)?;
    Some(whole(stress.total_kpa).max(0))
}

/// The written TRIT initial bulk density (hundredths of Mg/m3) and moisture
/// content (whole percent), each scattered under a TRIT key.
fn trit_density(site: &Site, hole: &Hole, sample: &Sample) -> Option<(i64, i64)> {
    let spec = specimen(site, hole, sample)?;
    let m = &spec.model;
    let bden = quantity(
        site,
        &spec,
        "TRIT",
        ("bulk_density", IndexQuantity::BulkDensity, DENSITY_COV),
        m.bulk_density_kg_m3,
    );
    let mc = quantity(
        site,
        &spec,
        "TRIT",
        (
            "moisture_content",
            IndexQuantity::MoistureContent,
            MOISTURE_COV,
        ),
        m.moisture_content_percent,
    );
    Some((whole(bden / 10.0).max(1), whole(mc).max(0)))
}

/// The written SPT N at the top of `sample`, whole and at least 0. In a unit
/// with `spt.stroud_butler` it is cu there (scattered under an ISPT key) over
/// f1; otherwise the model's N scattered.
pub(super) fn ispt_nval(site: &Site, hole: &Hole, sample: &Sample) -> Option<i64> {
    let spec = specimen_at(site, hole, sample, sample.top)?;
    let n = if let Some(f1) = spec.model.spt_f1 {
        scattered_cu(site, &spec, "ISPT")? / f1
    } else {
        let n = f64::from(spec.model.spt_n?);
        quantity(
            site,
            &spec,
            "ISPT",
            ("spt_n", IndexQuantity::SptN, SPT_COV),
            n,
        )
    };
    Some(whole(n).max(0))
}

fn text<T: std::fmt::Display>(v: Option<T>) -> String {
    v.map(|v| v.to_string()).unwrap_or_default()
}

/// The producers this module registers (see [`super::suite::PRODUCERS`]). The
/// loader refuses a suite entry whose unit cannot supply a listed heading
/// (LL, PL and PI need fine properties), so a blank cell only arises for a
/// sample that has no specimen.
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
    Producer {
        group: "TRIT",
        heading: "TRIT_CU",
        value: |s, h, x| text(trit_cu(s, h, x)),
    },
    Producer {
        group: "TRIT",
        heading: "TRIT_DEVF",
        value: |s, h, x| text(trit_cu(s, h, x).map(|cu| 2 * cu)),
    },
    Producer {
        group: "TRIT",
        heading: "TRIT_CELL",
        value: |s, h, x| text(trit_cell(s, h, x)),
    },
    Producer {
        group: "TRIT",
        heading: "TRIT_BDEN",
        value: |s, h, x| {
            text(trit_density(s, h, x).map(|(b, _)| format!("{:.2}", b as f64 / 100.0)))
        },
    },
    Producer {
        group: "TRIT",
        heading: "TRIT_IMC",
        value: |s, h, x| text(trit_density(s, h, x).map(|(_, mc)| mc)),
    },
    Producer {
        group: "ISPT",
        heading: "ISPT_NVAL",
        value: |s, h, x| text(ispt_nval(s, h, x)),
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
            // LL, PL and PI need fine properties, which RTD lacks.
            let units = if *group == "LLPL" {
                "\"WLC\", \"LC\""
            } else {
                "\"RTD\", \"WLC\", \"LC\""
            };
            let _ = write!(
                out,
                "\n[[suite]]\ngroup = \"{group}\"\nheadings = [{}]\nunits = [{units}]\nrate = {{ min = 1.0, max = 1.0, basis = \"test fixture\" }}\n",
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
            // Lognormal scatter truncated at +-3 keeps the COV within 3 %.
            let want = cov;
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

    /// Lognormal scatter never reaches zero, even three deviations down at a
    /// COV of 100 %, and is right-skewed: the median result sits under the
    /// mean (the model value).
    #[test]
    fn lognormal_scatter_stays_positive_and_skews_right() {
        for cov in [0.055, 0.42, 1.0] {
            assert!(lognormal_factor(cov, -TRUNCATION) > 0.0, "{cov}");
            assert!(lognormal_factor(cov, 0.0) < 1.0, "{cov}");
            // One deviation up from the median reaches further than one down.
            let median = lognormal_factor(cov, 0.0);
            assert!(
                lognormal_factor(cov, 1.0) - median > median - lognormal_factor(cov, -1.0),
                "{cov}"
            );
        }
        assert!((lognormal_factor(0.0, 2.0) - 1.0).abs() < 1e-12);
    }

    /// The factor's mean is 1 and its COV the given one, before truncation.
    #[test]
    fn the_lognormal_factor_keeps_the_mean_and_cov() {
        let mut rng = Key::site("t").rng(9);
        for cov in [0.088, 0.42] {
            let fs: Vec<f64> = (0..200_000)
                .map(|_| lognormal_factor(cov, truncated_normal(&mut rng)))
                .collect();
            let n = fs.len() as f64;
            let mean = fs.iter().sum::<f64>() / n;
            let sd = (fs.iter().map(|f| (f - mean).powi(2)).sum::<f64>() / n).sqrt();
            // Truncation at +-3 trims the upper tail: under 0.5 % off the
            // mean and 3 % off the COV at 0.42.
            assert!((mean - 1.0).abs() < 0.005, "{cov}: mean {mean}");
            assert!((sd / cov - 1.0).abs() < 0.035, "{cov}: sd {sd}");
        }
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

    // Strength results. The fine units (WLC, LC) carry cu = 60-120 kPa + 4-8
    // kPa/m and f1 = 4-6; the coarse unit (RTD) keeps its own N.

    const GS: &str =
        "particle_density_ratio = { min = 2.6, max = 2.7, basis = \"test fixture\" }\n";
    const STRENGTH_LINES: &str = "strength = { relation = \"cu.linear_with_depth\", cu_top_kpa = { min = 60, max = 120, basis = \"test fixture\" }, per_m = { min = 4, max = 8, basis = \"test fixture\" } }\nspt = { relation = \"spt.stroud_butler\", f1 = { min = 4, max = 6, basis = \"test fixture\" } }\n";

    /// [`FIXTURE`] with strength on its two fine units.
    fn strong_fixture() -> String {
        let mut parts = FIXTURE.split(GS);
        let mut out = String::from(parts.next().unwrap());
        for (i, part) in parts.enumerate() {
            out.push_str(GS);
            if i > 0 {
                out.push_str(STRENGTH_LINES);
            }
            out.push_str(part);
        }
        out
    }

    const TRIT_ALL: &[&str] = &["TRIT_CU", "TRIT_DEVF", "TRIT_CELL", "TRIT_BDEN", "TRIT_IMC"];

    /// A suite of TRIG over the fine units with the given TRIT headings, and
    /// ISPT over every unit with `ISPT_NVAL`.
    fn strength_suite(trit: &[&str]) -> String {
        let list: Vec<String> = trit.iter().map(|h| format!("\"{h}\"")).collect();
        format!(
            r#"
[[suite]]
group = "TRIG"
headings = []
units = ["WLC", "LC"]
rate = {{ min = 1.0, max = 1.0, basis = "test fixture" }}

[[suite]]
group = "TRIT"
headings = [{}]

[[suite]]
group = "ISPT"
headings = ["ISPT_NVAL"]
units = ["RTD", "WLC", "LC"]
rate = {{ min = 1.0, max = 1.0, basis = "test fixture" }}
"#,
            list.join(", ")
        )
    }

    fn strong(trit: &[&str]) -> Preset {
        let text = format!("{}{}", strong_fixture(), strength_suite(trit));
        Preset::from_toml(&text).unwrap_or_else(|e| panic!("{e}"))
    }

    fn num(s: &str) -> f64 {
        s.parse()
            .unwrap_or_else(|_| panic!("{s:?} is not a number"))
    }

    #[test]
    fn trit_devf_is_twice_the_written_cu_whichever_headings_are_listed() {
        let both = strong(TRIT_ALL);
        let devf_only = strong(&["TRIT_DEVF"]);
        let mut rows = 0;
        for seed in 0..60 {
            let m = file(&both, seed, 6);
            let (cu, devf) = (cells(&m, "TRIT", "TRIT_CU"), cells(&m, "TRIT", "TRIT_DEVF"));
            for (c, d) in cu.iter().zip(&devf) {
                assert_eq!(int(d), 2 * int(c), "seed {seed}");
                rows += 1;
            }
            let alone = file(&devf_only, seed, 6);
            assert_eq!(cells(&alone, "TRIT", "TRIT_DEVF"), devf, "seed {seed}");
        }
        assert!(rows > 1000, "{rows} TRIT rows is too few");
    }

    #[test]
    fn trit_cell_is_the_rounded_total_stress_at_the_specimen() {
        let p = strong(TRIT_ALL);
        let mut rows = 0;
        for seed in 0..30 {
            let site = build::build(&p, seed, 6);
            let m = build::lower(&site, &p, seed, Grid::Local);
            let (loca, depth, cell) = (
                cells(&m, "TRIT", "LOCA_ID"),
                cells(&m, "TRIT", "SPEC_DPTH"),
                cells(&m, "TRIT", "TRIT_CELL"),
            );
            for i in 0..cell.len() {
                let hole = site.holes.iter().position(|h| h.id == loca[i]).unwrap();
                let total = site.stress_at(hole, num(&depth[i])).unwrap().total_kpa;
                assert_eq!(int(&cell[i]), whole(total), "seed {seed}");
                assert!(total > 0.0);
                rows += 1;
            }
        }
        assert!(rows > 300, "{rows} TRIT rows is too few");
    }

    /// (ratio to the model value, outlier?) of every strength-family result
    /// over many sites, with the COV each is scattered with.
    fn strength_ratios() -> Ratios {
        let p = strong(TRIT_ALL);
        let mut out: Ratios = vec![
            ("TRIT cu", STRENGTH_COV, Vec::new()),
            ("ISPT cu", STRENGTH_COV, Vec::new()),
            ("ISPT N (coarse)", SPT_COV, Vec::new()),
            ("TRIT BDEN", DENSITY_COV, Vec::new()),
            ("TRIT IMC", MOISTURE_COV, Vec::new()),
        ];
        for seed in 0..250 {
            let site = build::build(&p, seed, 8);
            let props = &site.properties;
            for h in &site.holes {
                for s in h.strata.iter().flat_map(|s| s.samples.iter()) {
                    let Some(spec) = specimen(&site, h, s) else {
                        continue;
                    };
                    let mut put = |i: usize, spec: &Specimen<'_>, key: (&str, &str), q, v: f64| {
                        let band = match q {
                            None => {
                                let r = spec.model.strength.unwrap().range_kpa;
                                Some(strength_band(r))
                            }
                            Some(q) => props.band(spec.unit, q),
                        };
                        let r = scatter(props.seed(), spec, key, band, v, out[i].1);
                        out[i].2.push((r.value / v, r.outlier));
                    };
                    let m = &spec.model;
                    if let Some(st) = m.strength {
                        put(0, &spec, ("TRIT", "undrained_strength"), None, st.cu_kpa);
                    }
                    put(
                        3,
                        &spec,
                        ("TRIT", "bulk_density"),
                        Some(IndexQuantity::BulkDensity),
                        m.bulk_density_kg_m3,
                    );
                    put(
                        4,
                        &spec,
                        ("TRIT", "moisture_content"),
                        Some(IndexQuantity::MoistureContent),
                        m.moisture_content_percent,
                    );
                    let at_top = specimen_at(&site, h, s, s.top).unwrap();
                    if let Some(st) = at_top.model.strength {
                        put(1, &at_top, ("ISPT", "undrained_strength"), None, st.cu_kpa);
                    }
                    // A model N of zero (a stratum outlier) has no ratio.
                    if let Some(n) = at_top.model.spt_n.filter(|&n| n > 0) {
                        put(
                            2,
                            &at_top,
                            ("ISPT", "spt_n"),
                            Some(IndexQuantity::SptN),
                            f64::from(n),
                        );
                    }
                }
            }
        }
        out
    }

    #[test]
    fn strength_scatter_has_the_default_mean_and_cov_and_outliers_the_default_rate() {
        for (name, cov, rs) in strength_ratios() {
            assert!(rs.len() > 1500, "{name}: {} results is too few", rs.len());
            let ok: Vec<f64> = rs.iter().filter(|r| !r.1).map(|r| r.0).collect();
            let n = ok.len() as f64;
            let mean = ok.iter().sum::<f64>() / n;
            let var = ok.iter().map(|r| (r - mean).powi(2)).sum::<f64>() / (n - 1.0);
            // Lognormal scatter truncated at +-3 keeps the COV within 3 %.
            let want = cov;
            assert!((mean - 1.0).abs() < 0.015, "{name}: mean {mean}");
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

    /// The calibrated result COVs stay inside the site-specific ranges their
    /// doc comments check them against (ISSMGE-TC304 2021, Tables 1.2 and
    /// 1.3), so a re-calibration cannot drift out of them unnoticed.
    #[test]
    fn the_calibrated_covs_sit_inside_the_tc304_site_specific_ranges() {
        for (name, cov, lo, hi) in [
            ("MOISTURE_COV (w)", MOISTURE_COV, 0.035, 0.46),
            ("LIMIT_COV (LL)", LIMIT_COV, 0.034, 0.39),
            ("LIMIT_COV (PL)", LIMIT_COV, 0.029, 0.381),
            ("STRENGTH_COV (su)", STRENGTH_COV, 0.06, 0.56),
            ("SPT_COV (sand SPT-N)", SPT_COV, 0.184, 0.62),
        ] {
            assert!((lo..=hi).contains(&cov), "{name} {cov} outside {lo}-{hi}");
        }
    }

    /// In a fine unit N x f1 / cu is 1 before scatter, so over many tests its
    /// median is a lognormal's, 1 / sqrt(1 + COV^2), with the mean at 1
    /// (rounding to whole blows is unbiased).
    #[test]
    fn a_fine_units_n_times_f1_over_the_model_cu_has_median_one() {
        let p = strong(TRIT_ALL);
        let mut ratios = Vec::new();
        for seed in 0..60 {
            let site = build::build(&p, seed, 8);
            let m = build::lower(&site, &p, seed, Grid::Local);
            let (loca, top, n) = (
                cells(&m, "ISPT", "LOCA_ID"),
                cells(&m, "ISPT", "ISPT_TOP"),
                cells(&m, "ISPT", "ISPT_NVAL"),
            );
            for i in 0..n.len() {
                let hole = site.holes.iter().position(|h| h.id == loca[i]).unwrap();
                let e = site.properties_at(hole, num(&top[i])).unwrap();
                let (Some(st), Some(f1)) = (e.strength, e.spt_f1) else {
                    continue;
                };
                ratios.push(num(&n[i]) * f1 / st.cu_kpa);
            }
        }
        assert!(ratios.len() > 800, "{} fine tests is too few", ratios.len());
        ratios.sort_by(f64::total_cmp);
        let median = ratios[ratios.len() / 2];
        let want = 1.0 / STRENGTH_COV.mul_add(STRENGTH_COV, 1.0).sqrt();
        assert!((median - want).abs() < 0.03, "median {median} vs {want}");
    }

    #[test]
    fn a_coarse_units_n_scatters_around_its_own_model() {
        let p = strong(TRIT_ALL);
        let mut ratios = Vec::new();
        for seed in 0..60 {
            let site = build::build(&p, seed, 8);
            let m = build::lower(&site, &p, seed, Grid::Local);
            let (loca, top, n) = (
                cells(&m, "ISPT", "LOCA_ID"),
                cells(&m, "ISPT", "ISPT_TOP"),
                cells(&m, "ISPT", "ISPT_NVAL"),
            );
            for i in 0..n.len() {
                let hole = site.holes.iter().position(|h| h.id == loca[i]).unwrap();
                let e = site.properties_at(hole, num(&top[i])).unwrap();
                if let Some(model) = e.spt_n {
                    ratios.push(num(&n[i]) / f64::from(model));
                }
            }
        }
        assert!(
            ratios.len() > 200,
            "{} coarse tests is too few",
            ratios.len()
        );
        ratios.sort_by(f64::total_cmp);
        let median = ratios[ratios.len() / 2];
        assert!((median - 1.0).abs() < 0.05, "median {median}");
    }

    #[test]
    fn a_file_with_every_strength_heading_is_rust_clean_and_parses_as_its_types() {
        let p = strong(TRIT_ALL);
        let dir = tempfile::tempdir().unwrap();
        for seed in 0..25 {
            let m = file(&p, seed, 6);
            let path = dir.path().join("site.ags");
            std::fs::write(&path, emit_to_string(&m)).unwrap();
            assert_eq!(rust_check(&path).0, RustResult::Clean, "seed {seed}");
            for (g, h, dp) in [
                ("TRIT", "TRIT_CU", 0),
                ("TRIT", "TRIT_DEVF", 0),
                ("TRIT", "TRIT_CELL", 0),
                ("TRIT", "TRIT_BDEN", 2),
                ("TRIT", "TRIT_IMC", 0),
                ("ISPT", "ISPT_NVAL", 0),
            ] {
                let c = cells(&m, g, h);
                assert!(!c.is_empty(), "{h} has no rows at seed {seed}");
                for c in c {
                    let v: f64 = c.parse().unwrap_or_else(|_| panic!("{h} {c:?}"));
                    assert!(v >= 0.0);
                    let places = c.split_once('.').map_or(0, |(_, f)| f.len());
                    assert_eq!(places, dp, "{h} {c}");
                }
            }
        }
    }

    /// A preset with strength but no new heading listed writes the bytes it
    /// wrote without strength.
    #[test]
    fn strength_in_a_preset_moves_no_byte_until_a_heading_asks_for_it() {
        let listed = format!("{}{}", strong_fixture(), suite(ALL));
        let bare = format!("{FIXTURE}{}", suite(ALL));
        let (a, b) = (
            Preset::from_toml(&listed).unwrap(),
            Preset::from_toml(&bare).unwrap(),
        );
        for seed in 0..10 {
            assert_eq!(
                emit_to_string(&file(&a, seed, 5)),
                emit_to_string(&file(&b, seed, 5)),
                "seed {seed}"
            );
        }
    }

    #[test]
    fn listing_the_strength_headings_moves_no_other_cell() {
        let with = strong(TRIT_ALL);
        let without = strong(&[]);
        let cu_only = strong(&["TRIT_CU"]);
        for seed in 0..10 {
            let (a, b, c) = (
                file(&with, seed, 5),
                file(&without, seed, 5),
                file(&cu_only, seed, 5),
            );
            for g in ["PROJ", "LOCA", "GEOL", "SAMP", "TRIG"] {
                let rows = |m: &ProjectModel| {
                    m.groups
                        .iter()
                        .find(|x| x.code == g)
                        .map(|x| x.rows.iter().map(|r| r.values.clone()).collect::<Vec<_>>())
                };
                assert_eq!(rows(&a), rows(&b), "{g}, seed {seed}");
            }
            assert_eq!(
                cells(&a, "ISPT", "ISPT_NVAL"),
                cells(&c, "ISPT", "ISPT_NVAL")
            );
        }
    }
}
