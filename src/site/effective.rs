//! Effective-stress triaxial results: the TREG sets and their TRET stages (ADR
//! 0001, "Effective-stress triaxial").
//!
//! A set is one TREG row with three TRET rows (`TRET_TESN` 1 to 3), built from
//! the unit's site values of c', phi' and Skempton's `A_f`
//! ([`super::properties::EffectiveValues`]).
//!
//! - **Type.** Drawn per set by the weights of the NE England crawl: CUM 22,
//!   CDM 15, CU 10, CD 10 of its 57 sets (about 39%, 26%, 18%, 18%). The M
//!   types are one specimen sheared in three stages; CU and CD are three
//!   specimens, one per stage. Both are written the way the crawl's files write
//!   them: one TREG row, one specimen key (`SPEC_REF`, `SPEC_DPTH`), and the
//!   three `TRET_TESN` values telling the stages or specimens apart.
//! - **Set scatter.** The set's own c' and phi' scatter around the site's
//!   values ([`C_SCATTER_SD_KPA`], [`PHI_SCATTER_SD_DEG`]).
//! - **Stages.** The first stage's effective confining stress is
//!   [`SIGMA3_KPA_PER_M`] x the specimen depth, and stages 2 and 3 are twice
//!   and four times it. The back pressure is [`BACK_PRESSURE_KPA`], so
//!   `TRET_CELL` is the effective stress plus the back pressure, `TRET_PWPI` is
//!   the back pressure and `TRET_CONP` is the effective stress.
//! - **Failure.** See [`failure_deviator`]. Drained tests (CD, CDM) fail on the
//!   envelope with the pore pressure unchanged, `TRET_DEVF` scattered by
//!   [`DEVF_COV`]. Undrained tests (CU, CUM) draw `A_f` for each stage around the
//!   unit's value, and the pore pressure rises by `A_f` x the deviator stress.
//! - **Envelope.** `TREG_COH` and `TREG_PHI` are the least-squares line through
//!   the set's three written failure points in (s', t) space, as a laboratory
//!   reports them (in the crawl they agree with its stages to about half a
//!   degree), so the TREG row and its TRET rows always agree.
//!
//! Every draw is keyed per (hole, unit, sample) under tags of its own, with the
//! stage or specimen number in the test index, so a set is recomputed exactly
//! wherever it is needed and changing one unit moves only its rows.

use super::build::{Hole, Sample, Site};
use super::draw::{Key, uniform};
use super::preset::{C_EFF_BOUNDS_KPA, PHI_EFF_BOUNDS_DEG, SKEMPTON_AF_BOUNDS};
use super::properties::IndexQuantity;
use super::results::{
    DENSITY_COV, MOISTURE_COV, Specimen, lognormal_factor, quantity, specimen, truncated_normal,
    whole,
};

/// Back pressure, kPa, every set. Forge judgement (owner decision, #68); the
/// crawl's 165950 and 169655 use 300 and 174059 about 500.
pub const BACK_PRESSURE_KPA: i64 = 300;
/// Effective confining stress of stage 1 per metre of specimen depth, kPa/m:
/// the crawl's median is 10.0 (P10 to P90 9.1 to 12.4), about the buoyant
/// unit weight times depth for a 20 kN/m3 soil (forge analysis of BGS NGDC AGS accessions 165950,
/// 169655, 174059). Owner decision, #68.
pub const SIGMA3_KPA_PER_M: f64 = 10.0;
/// Stages 1 to 3 as multiples of the stage-1 effective confining stress: the
/// crawl's stage ratios are 2.0 at the median (88% within 1.8 to 2.2).
pub const STAGE_FACTORS: [i64; 3] = [1, 2, 4];
/// Standard deviation of a set's c' about the site's value, kPa. Forge
/// judgement: modest against the crawl's 2.7 to 7.1 kPa within a unit (which
/// also holds site-to-site spread, drawn separately here).
pub const C_SCATTER_SD_KPA: f64 = 2.0;
/// Standard deviation of a set's phi' about the site's value, degrees. Forge
/// judgement: modest against the crawl's 2.4 to 7.1 degrees within a unit.
pub const PHI_SCATTER_SD_DEG: f64 = 1.5;
/// Standard deviation of a stage's `A_f` about the site's value. Forge
/// judgement, below the crawl's within-unit spread (which also holds site and
/// stress-level differences).
pub const AF_SCATTER_SD: f64 = 0.10;
/// Coefficient of variation of the deviator stress at failure of a drained
/// stage about the envelope. Calibrated to the crawl: the failure points of its
/// 142 stages differ from the envelope their set reports by P10 -4% and P90
/// +3% of the shear stress t.
pub const DEVF_COV: f64 = 0.03;

/// The test type of a set (`TREG_TYPE`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// Consolidated undrained, pore pressure measured, one specimen per stage.
    Cu,
    /// As [`Kind::Cu`], three stages on one specimen.
    Cum,
    /// Consolidated drained, one specimen per stage.
    Cd,
    /// As [`Kind::Cd`], three stages on one specimen.
    Cdm,
}

/// The kinds with the number of the crawl's 57 sets of each (CUM 22, CDM 15, CU
/// 10, CD 10), which are their weights. Forge calibration, #68.
pub const KIND_WEIGHTS: [(Kind, u32); 4] = [
    (Kind::Cum, 22),
    (Kind::Cdm, 15),
    (Kind::Cu, 10),
    (Kind::Cd, 10),
];

impl Kind {
    /// The `TREG_TYPE` code.
    pub fn code(self) -> &'static str {
        match self {
            Kind::Cu => "CU",
            Kind::Cum => "CUM",
            Kind::Cd => "CD",
            Kind::Cdm => "CDM",
        }
    }

    /// One specimen sheared in three stages, not three specimens.
    pub fn multi_stage(self) -> bool {
        matches!(self, Kind::Cum | Kind::Cdm)
    }

    /// The pore pressure does not change in shear.
    pub fn drained(self) -> bool {
        matches!(self, Kind::Cd | Kind::Cdm)
    }

    /// `TREG_FCR`: the crawl's wording, by the criterion each type uses (CUM
    /// fails at the maximum effective principal stress ratio, the rest at the
    /// maximum deviator stress).
    pub fn failure_criterion(self) -> &'static str {
        match self {
            Kind::Cum => "Maximum effective principal stress ratio",
            _ => "Maximum Deviator Stress",
        }
    }
}

/// One TRET row: a stage of a multi-stage set or a specimen of a single-stage
/// one. Stresses in kPa, whole; density in hundredths of Mg/m3.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Stage {
    pub tesn: i64,
    pub cell: i64,
    pub pwpi: i64,
    pub devf: i64,
    pub pwpf: i64,
    pub conp: i64,
    pub bden_hundredths: i64,
    pub imc: i64,
}

impl Stage {
    /// The written value of a TRET heading, `None` for one that is not the
    /// stage's (a KEY heading other than `TRET_TESN`).
    pub fn value(&self, heading: &str) -> Option<String> {
        Some(match heading {
            "TRET_TESN" => self.tesn.to_string(),
            "TRET_CELL" => self.cell.to_string(),
            "TRET_PWPI" => self.pwpi.to_string(),
            "TRET_DEVF" => self.devf.to_string(),
            "TRET_PWPF" => self.pwpf.to_string(),
            "TRET_CONP" => self.conp.to_string(),
            "TRET_BDEN" => format!("{:.2}", self.bden_hundredths as f64 / 100.0),
            "TRET_IMC" => self.imc.to_string(),
            _ => return None,
        })
    }

    /// Effective minor principal stress at failure, kPa: cell pressure less the
    /// pore pressure at failure.
    pub fn sigma3_failure(&self) -> i64 {
        self.cell - self.pwpf
    }
}

/// One TREG set: the type, the fitted envelope as written, and the stages.
#[derive(Debug, Clone, PartialEq)]
pub struct Set {
    pub kind: Kind,
    /// `TREG_COH`, kPa, whole.
    pub coh: i64,
    /// `TREG_PHI`, degrees, to a tenth.
    pub phi_deg: f64,
    pub stages: [Stage; 3],
}

/// The deviator stress q = sigma1' - sigma3' at which a specimen with effective
/// confining stress `sigma3` (kPa) before shear fails on the Mohr-Coulomb
/// envelope of c' `c_kpa` and phi' `phi_deg`, when the pore pressure rises by
/// `af` x q (Skempton's `A_f`; 0 for a drained test).
///
/// With s' = (sigma1' + sigma3') / 2 and t = q / 2 at failure, the envelope is
/// `t = c' cos(phi') + s' sin(phi')`. The pore pressure rise A q lowers the
/// effective minor stress, sigma3f' = sigma3 - A q, so
/// `s' = sigma3 - A q + q / 2 = sigma3 + q (1/2 - A)`. Putting that and
/// `t = q / 2` into the envelope gives
/// `q / 2 = c' cos(phi') + sigma3 sin(phi') + q (1/2 - A) sin(phi')`, which
/// solves for
/// `q = 2 (c' cos(phi') + sigma3 sin(phi')) / (1 - sin(phi') (1 - 2 A))`.
/// For A = 0 this is the textbook drained result
/// `2 (c' cos(phi') + sigma3 sin(phi')) / (1 - sin(phi'))`. The denominator is
/// held to at least 0.05 so a draw beyond the loader's bounds cannot divide by
/// zero or flip the sign (the loader refuses ranges that could).
pub fn failure_deviator(c_kpa: f64, phi_deg: f64, sigma3: f64, af: f64) -> f64 {
    let (sin, cos) = phi_deg.to_radians().sin_cos();
    let denominator = (1.0 - sin * (1.0 - 2.0 * af)).max(0.05);
    2.0 * (c_kpa * cos + sigma3 * sin) / denominator
}

/// The least-squares envelope through failure points `(s', t)`: `t = a + b s'`,
/// returned as (c' in kPa, phi' in degrees) with sin(phi') = b and c' = a /
/// cos(phi'). A negative intercept is refitted through the origin (c' = 0), as
/// laboratories report c' = 0 when the line would cut below it.
pub fn fit_envelope(points: &[(f64, f64)]) -> (f64, f64) {
    let n = points.len() as f64;
    let (sum_s, sum_t) = points
        .iter()
        .fold((0.0, 0.0), |(s, t), p| (s + p.0, t + p.1));
    let sxx: f64 = points.iter().map(|p| (p.0 - sum_s / n).powi(2)).sum();
    let sxy: f64 = points
        .iter()
        .map(|p| (p.0 - sum_s / n) * (p.1 - sum_t / n))
        .sum();
    let mut slope = sxy / sxx;
    let mut intercept = sum_t / n - slope * sum_s / n;
    if intercept < 0.0 {
        let (ss, st) = points
            .iter()
            .fold((0.0, 0.0), |(a, b), p| (a + p.0 * p.0, b + p.0 * p.1));
        slope = st / ss;
        intercept = 0.0;
    }
    let slope = slope.clamp(0.0, 0.999);
    let phi = slope.asin();
    (intercept / phi.cos(), phi.to_degrees())
}

/// A copy of `spec` at test index `3 sample + k`, so each specimen of a set
/// (`k` 0 to 2) has density and moisture draws of its own.
fn specimen_k<'a>(spec: &Specimen<'a>, k: u64) -> Specimen<'a> {
    Specimen {
        hole_index: spec.hole_index,
        depth: spec.depth,
        unit: spec.unit,
        sample_index: spec.sample_index * 3 + k,
        model: spec.model.clone(),
    }
}

/// The set on `sample`, or `None` where its unit has no `effective_strength`.
pub fn set(site: &Site, hole: &Hole, sample: &Sample) -> Option<Set> {
    let spec = specimen(site, hole, sample)?;
    let eff = spec.model.effective?;
    let seed = site.properties.seed();
    let rng = |tag: &str, test: u64| {
        Key {
            tag,
            hole: spec.hole_index as u64,
            unit: spec.unit,
            test,
        }
        .rng(seed)
    };

    let total: u32 = KIND_WEIGHTS.iter().map(|k| k.1).sum();
    let pick = uniform(
        &mut rng("treg-type", spec.sample_index),
        0.0,
        f64::from(total),
    );
    let mut acc = 0.0;
    let kind = KIND_WEIGHTS
        .iter()
        .find(|(_, w)| {
            acc += f64::from(*w);
            pick < acc
        })
        .map_or(Kind::Cum, |k| k.0);

    let mut r = rng("treg-cphi", spec.sample_index);
    let c = (eff.c_kpa + C_SCATTER_SD_KPA * truncated_normal(&mut r))
        .clamp(C_EFF_BOUNDS_KPA.0, C_EFF_BOUNDS_KPA.1);
    let phi = (eff.phi_deg + PHI_SCATTER_SD_DEG * truncated_normal(&mut r))
        .clamp(PHI_EFF_BOUNDS_DEG.0, PHI_EFF_BOUNDS_DEG.1);

    // Stage 1 to the kPa; stages 2 and 3 are exact multiples of the written value.
    let stage_one = whole(SIGMA3_KPA_PER_M * spec.depth).max(1);
    let stage = |i: usize| {
        let test = spec.sample_index * 3 + i as u64;
        let sigma3 = stage_one * STAGE_FACTORS[i];
        // A drained stage fails with no pore pressure change, so A is 0 in the
        // deviator solve and the envelope point is scattered; an undrained one
        // draws its A_f and fails exactly on the envelope.
        let (q, af) = if kind.drained() {
            let e = truncated_normal(&mut rng("treg-devf", test));
            let q = failure_deviator(c, phi, sigma3 as f64, 0.0) * lognormal_factor(DEVF_COV, e);
            (q, 0.0)
        } else {
            let af = (eff.af + AF_SCATTER_SD * truncated_normal(&mut rng("treg-af", test)))
                .clamp(SKEMPTON_AF_BOUNDS.0, SKEMPTON_AF_BOUNDS.1);
            (failure_deviator(c, phi, sigma3 as f64, af), af)
        };
        let devf = whole(q).max(1);
        // The pore pressure rise is A_f x the written deviator stress.
        let pwpf = BACK_PRESSURE_KPA + whole(af * devf as f64);
        // One specimen in a multi-stage set, so its density and moisture
        // content repeat; three in a single-stage one.
        let sp = specimen_k(&spec, if kind.multi_stage() { 0 } else { i as u64 });
        let bden = quantity(
            site,
            &sp,
            "TRET",
            ("bulk_density", IndexQuantity::BulkDensity, DENSITY_COV),
            sp.model.bulk_density_kg_m3,
        );
        let mc = quantity(
            site,
            &sp,
            "TRET",
            (
                "moisture_content",
                IndexQuantity::MoistureContent,
                MOISTURE_COV,
            ),
            sp.model.moisture_content_percent,
        );
        Stage {
            tesn: i as i64 + 1,
            cell: sigma3 + BACK_PRESSURE_KPA,
            pwpi: BACK_PRESSURE_KPA,
            devf,
            pwpf,
            conp: sigma3,
            bden_hundredths: whole(bden / 10.0).max(1),
            imc: whole(mc).max(0),
        }
    };
    let stages = [stage(0), stage(1), stage(2)];

    let points: Vec<(f64, f64)> = stages
        .iter()
        .map(|s| {
            let t = s.devf as f64 / 2.0;
            (s.sigma3_failure() as f64 + t, t)
        })
        .collect();
    let (c_fit, phi_fit) = fit_envelope(&points);
    Some(Set {
        kind,
        coh: whole(c_fit.min(C_EFF_BOUNDS_KPA.1)).max(0),
        phi_deg: (phi_fit * 10.0).round() / 10.0,
        stages,
    })
}

/// The written value of a TRET `heading` for the first stage of the set on
/// `sample`: what a TRET producer gives (`super::suite` writes the other
/// stages from the whole [`Set`]).
pub fn first_stage(site: &Site, hole: &Hole, sample: &Sample, heading: &str) -> Option<String> {
    set(site, hole, sample)?.stages[0].value(heading)
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

    /// The worked example, by hand. c' = 5 kPa, phi' = 30 degrees (sin 0.5, cos
    /// 0.8660254), sigma3' = 100 kPa, drained: q = 2 (5 x 0.8660254 + 100 x
    /// 0.5) / (1 - 0.5) = 2 x 54.330127 / 0.5 = 217.3205. Checked against the
    /// failure plane: sigma1' = 317.3205, so the plane's normal stress is
    /// (417.3205 / 2) - (217.3205 / 2) x 0.5 = 154.330127 and its shear stress
    /// (217.3205 / 2) x 0.8660254 = 94.1025, and c' + sigma-n tan(30) = 5 +
    /// 154.330127 x 0.5773503 = 94.1025.
    #[test]
    fn a_drained_stage_fails_at_the_hand_worked_deviator() {
        let q = failure_deviator(5.0, 30.0, 100.0, 0.0);
        assert!((q - 217.3205).abs() < 1e-3, "{q}");
        // c' = 0, phi' = 30: q = 2 x 100 x 0.5 / 0.5 = 200 (sigma1'/sigma3' = 3).
        assert!((failure_deviator(0.0, 30.0, 100.0, 0.0) - 200.0).abs() < 1e-9);
    }

    /// The same soil at `A_f` = 0.5: the pore pressure rise is half the deviator
    /// stress, so the mean effective stress s' is unchanged at 100 and the
    /// denominator is 1: q = 2 x 54.330127 = 108.6603, du = 54.3301, sigma3f' =
    /// 100 - 54.3301 = 45.6699, sigma1f' = 154.3301, so s' = 100, t = 54.3301
    /// = c' cos + s' sin = 4.3301 + 50.
    #[test]
    fn an_undrained_stage_at_af_a_half_fails_at_the_hand_worked_deviator() {
        let q = failure_deviator(5.0, 30.0, 100.0, 0.5);
        assert!((q - 108.6603).abs() < 1e-3, "{q}");
        let (du, s3f) = (0.5 * q, 100.0 - 0.5 * q);
        assert!((du - 54.3301).abs() < 1e-3 && (s3f - 45.6699).abs() < 1e-3);
        // A_f = 0.25: the denominator is 1 - 0.5 x (1 - 0.5) = 0.75, so q =
        // 2 x 54.330127 / 0.75 = 144.8803.
        let q = failure_deviator(5.0, 30.0, 100.0, 0.25);
        assert!((q - 144.8803).abs() < 1e-3, "{q}");
    }

    #[test]
    fn a_line_through_exact_points_is_recovered_and_a_negative_intercept_goes_through_the_origin() {
        // t = 4.3301 + 0.5 s' (c' 5, phi' 30) at s' = 100, 200, 400.
        let pts = [(100.0, 54.3301), (200.0, 104.3301), (400.0, 204.3301)];
        let (c, phi) = fit_envelope(&pts);
        assert!(
            (c - 5.0).abs() < 1e-3 && (phi - 30.0).abs() < 1e-3,
            "{c} {phi}"
        );
        // Points with intercept -10 are refitted through the origin: slope =
        // sum(s t) / sum(s s) = (100x40 + 200x90 + 400x190) / (100^2 + 200^2 +
        // 400^2) = 98000 / 210000 = 0.46667, phi = asin(0.46667) = 27.82.
        let (c, phi) = fit_envelope(&[(100.0, 40.0), (200.0, 90.0), (400.0, 190.0)]);
        assert!(c.abs() < 1e-12, "{c}");
        assert!((phi - 27.82).abs() < 0.01, "{phi}");
    }

    #[test]
    fn the_kind_weights_are_the_crawls_counts_and_types_code_their_stages() {
        assert_eq!(KIND_WEIGHTS.iter().map(|k| k.1).sum::<u32>(), 57);
        assert!(Kind::Cum.multi_stage() && Kind::Cdm.multi_stage());
        assert!(!Kind::Cu.multi_stage() && !Kind::Cd.multi_stage());
        assert!(Kind::Cd.drained() && Kind::Cdm.drained());
        assert!(!Kind::Cu.drained() && !Kind::Cum.drained());
        assert_eq!(
            Kind::Cum.failure_criterion(),
            "Maximum effective principal stress ratio"
        );
        assert_eq!(Kind::Cd.failure_criterion(), "Maximum Deviator Stress");
    }

    const GS: &str =
        "particle_density_ratio = { min = 2.6, max = 2.7, basis = \"test fixture\" }\n";

    /// `effective_strength` with fixed c', phi' and `A_f`.
    pub(super) fn eff_line(c: f64, phi: f64, af: f64) -> String {
        format!(
            "effective_strength = {{ c_eff_kpa = {{ min = {c}, max = {c}, basis = \"test fixture\" }}, phi_eff_deg = {{ min = {phi}, max = {phi}, basis = \"test fixture\" }}, skempton_af = {{ min = {af}, max = {af}, basis = \"test fixture\" }} }}\n"
        )
    }

    const TREG_SUITE: &str = r#"
[[suite]]
group = "TREG"
headings = ["TREG_TYPE", "TREG_COND", "TREG_COH", "TREG_PHI", "TREG_FCR"]
units = ["WLC", "LC"]
rate = { min = 1.0, max = 1.0, basis = "test fixture" }

[[suite]]
group = "TRET"
headings = ["TRET_CELL", "TRET_PWPI", "TRET_DEVF", "TRET_PWPF", "TRET_CONP", "TRET_BDEN", "TRET_IMC"]
"#;

    /// [`FIXTURE`] with `wlc` and `lc` as the `effective_strength` lines of
    /// those units, and the TREG and TRET suite.
    pub(super) fn preset(wlc: &str, lc: &str) -> Preset {
        let mut parts = FIXTURE.split(GS);
        let mut out = String::from(parts.next().unwrap());
        for (i, part) in parts.enumerate() {
            out.push_str(GS);
            out.push_str(match i {
                1 => wlc,
                2 => lc,
                _ => "",
            });
            out.push_str(part);
        }
        out.push_str(TREG_SUITE);
        Preset::from_toml(&out).unwrap_or_else(|e| panic!("{e}"))
    }

    fn file(p: &Preset, seed: u64, holes: usize) -> ProjectModel {
        let site = build::build(p, seed, holes);
        build::lower(&site, p, seed, Grid::Local)
    }

    type RowMap = std::collections::BTreeMap<String, String>;

    fn rows(m: &ProjectModel, code: &str) -> Vec<RowMap> {
        let Some(g) = m.groups.iter().find(|g| g.code == code) else {
            return Vec::new();
        };
        g.rows
            .iter()
            .map(|r| {
                g.headings
                    .iter()
                    .cloned()
                    .zip(r.values.iter().cloned())
                    .collect()
            })
            .collect()
    }

    fn num(s: &str) -> f64 {
        s.parse()
            .unwrap_or_else(|_| panic!("{s:?} is not a number"))
    }

    /// The TRET rows of each TREG row, by the parent's key.
    fn sets(m: &ProjectModel) -> Vec<(RowMap, Vec<RowMap>)> {
        let tret = rows(m, "TRET");
        let key = |r: &RowMap| {
            (
                r["LOCA_ID"].clone(),
                r["SAMP_TOP"].clone(),
                r["SAMP_REF"].clone(),
                r["SPEC_REF"].clone(),
                r["SPEC_DPTH"].clone(),
            )
        };
        rows(m, "TREG")
            .into_iter()
            .map(|g| {
                let k = key(&g);
                let own: Vec<RowMap> = tret.iter().filter(|r| key(r) == k).cloned().collect();
                (g, own)
            })
            .collect()
    }

    /// Distance of a stage's written failure point from the written envelope,
    /// as a fraction of its shear stress t, and in kPa.
    fn off_envelope(treg: &RowMap, stage: &RowMap) -> (f64, f64) {
        let (c, phi) = (num(&treg["TREG_COH"]), num(&treg["TREG_PHI"]).to_radians());
        let devf = num(&stage["TRET_DEVF"]);
        let s3f = num(&stage["TRET_CELL"]) - num(&stage["TRET_PWPF"]);
        let t = devf / 2.0;
        let s = s3f + t;
        let on = c * phi.cos() + s * phi.sin();
        ((t - on).abs() / t, (t - on).abs())
    }

    #[test]
    fn every_set_has_three_stages_with_1_2_4_times_the_effective_stress() {
        let p = preset(&eff_line(8.0, 28.0, 0.3), &eff_line(5.0, 30.0, 0.1));
        let mut n = 0;
        for seed in 0..15 {
            let m = file(&p, seed, 5);
            for (treg, stages) in sets(&m) {
                n += 1;
                assert_eq!(stages.len(), 3, "seed {seed}");
                let tesn: Vec<&str> = stages.iter().map(|r| r["TRET_TESN"].as_str()).collect();
                assert_eq!(tesn, ["1", "2", "3"]);
                let conp: Vec<f64> = stages.iter().map(|r| num(&r["TRET_CONP"])).collect();
                assert_eq!((conp[1], conp[2]), (2.0 * conp[0], 4.0 * conp[0]));
                // 10 kPa per metre of specimen depth, to the kPa.
                let depth = num(&stages[0]["SPEC_DPTH"]);
                assert!(
                    (conp[0] - 10.0 * depth).abs() <= 0.5 + 1e-9,
                    "{conp:?} {depth}"
                );
                for r in &stages {
                    assert_eq!(r["TRET_PWPI"], "300");
                    assert!((num(&r["TRET_CELL"]) - num(&r["TRET_CONP"]) - 300.0).abs() < 1e-9);
                }
                assert!(["CU", "CUM", "CD", "CDM"].contains(&treg["TREG_TYPE"].as_str()));
                assert_eq!(treg["TREG_COND"], "UNDISTURBED");
            }
        }
        assert!(n > 100, "{n} sets is too few");
    }

    /// Multi-stage sets are one specimen, so density and moisture repeat;
    /// single-stage sets are three, and they differ.
    #[test]
    fn multi_stage_sets_repeat_density_and_single_stage_sets_do_not() {
        let p = preset(&eff_line(8.0, 28.0, 0.3), &eff_line(5.0, 30.0, 0.1));
        let (mut multi, mut single, mut varied) = (0, 0, 0);
        for seed in 0..25 {
            for (treg, stages) in sets(&file(&p, seed, 5)) {
                let same = stages.iter().all(|r| {
                    r["TRET_BDEN"] == stages[0]["TRET_BDEN"]
                        && r["TRET_IMC"] == stages[0]["TRET_IMC"]
                });
                if treg["TREG_TYPE"].ends_with('M') {
                    multi += 1;
                    assert!(same, "seed {seed}: {stages:?}");
                } else {
                    single += 1;
                    varied += usize::from(!same);
                }
            }
        }
        assert!(multi > 30 && single > 30, "{multi} multi, {single} single");
        assert!(varied * 10 > single * 8, "{varied} of {single} vary");
    }

    #[test]
    fn the_fcr_follows_the_type() {
        let p = preset(&eff_line(8.0, 28.0, 0.3), &eff_line(5.0, 30.0, 0.1));
        let mut kinds = std::collections::BTreeSet::new();
        for seed in 0..15 {
            for (treg, _) in sets(&file(&p, seed, 5)) {
                let want = if treg["TREG_TYPE"] == "CUM" {
                    "Maximum effective principal stress ratio"
                } else {
                    "Maximum Deviator Stress"
                };
                assert_eq!(treg["TREG_FCR"], want);
                kinds.insert(treg["TREG_TYPE"].clone());
            }
        }
        assert_eq!(kinds.len(), 4, "{kinds:?}");
    }

    /// The types come in the crawl's proportions.
    #[test]
    fn the_types_follow_the_crawl_weights() {
        let p = preset(&eff_line(8.0, 28.0, 0.3), &eff_line(5.0, 30.0, 0.1));
        let mut count = std::collections::BTreeMap::<String, usize>::new();
        let mut all = 0;
        for seed in 0..60 {
            for (treg, _) in sets(&file(&p, seed, 5)) {
                *count.entry(treg["TREG_TYPE"].clone()).or_default() += 1;
                all += 1;
            }
        }
        for (kind, weight) in KIND_WEIGHTS {
            let share = count[kind.code()] as f64 / f64::from(all);
            let want = f64::from(weight) / 57.0;
            assert!(
                (share - want).abs() < 0.04,
                "{} {share} vs {want}",
                kind.code()
            );
        }
    }

    /// Drained stages: the pore pressure does not move and the failure point
    /// is on the written envelope to within the 3% scatter (9% at the 3-sigma
    /// truncation) and the rounding.
    #[test]
    fn drained_stages_fail_on_the_envelope_with_no_pore_pressure_change() {
        let p = preset(&eff_line(8.0, 28.0, 0.3), &eff_line(5.0, 30.0, 0.1));
        let (mut n, mut worst) = (0, 0.0_f64);
        for seed in 0..40 {
            for (treg, stages) in sets(&file(&p, seed, 5)) {
                if !treg["TREG_TYPE"].starts_with("CD") {
                    continue;
                }
                for r in &stages {
                    n += 1;
                    assert_eq!(r["TRET_PWPF"], r["TRET_PWPI"], "seed {seed}");
                    let (rel, abs) = off_envelope(&treg, r);
                    assert!(rel < 0.15 || abs < 3.0, "seed {seed}: {rel} {abs} {r:?}");
                    worst = worst.max(rel);
                }
            }
        }
        assert!(n > 150, "{n} drained stages");
        assert!(worst > 0.005, "no scatter at all");
    }

    /// Undrained stages: the pore pressure rises by `A_f` x q, with `A_f` about the
    /// fixed 0.4 under the 0.10 scatter, and the effective failure point is on
    /// the envelope (to the rounding of the written integers and the fit).
    #[test]
    fn undrained_stages_fail_on_the_envelope_with_the_pore_pressure_rise_af_times_q() {
        let p = preset(&eff_line(8.0, 28.0, 0.4), &eff_line(6.0, 30.0, 0.4));
        let (mut n, mut sum) = (0, 0.0);
        for seed in 0..40 {
            for (treg, stages) in sets(&file(&p, seed, 5)) {
                if !treg["TREG_TYPE"].starts_with("CU") {
                    continue;
                }
                for r in &stages {
                    n += 1;
                    let af = (num(&r["TRET_PWPF"]) - num(&r["TRET_PWPI"])) / num(&r["TRET_DEVF"]);
                    // 3 sigma of the A_f scatter (0.30) and the rounding of du.
                    assert!((af - 0.4).abs() < 0.33, "seed {seed}: A_f {af}");
                    sum += af;
                    let (rel, abs) = off_envelope(&treg, r);
                    assert!(rel < 0.02 || abs < 3.0, "seed {seed}: {rel} {abs} {r:?}");
                }
            }
        }
        assert!(n > 150, "{n} undrained stages");
        let mean = sum / f64::from(n);
        assert!((mean - 0.4).abs() < 0.03, "mean A_f {mean}");
    }

    /// The written envelope is the fit to the set's points, so it recovers the
    /// site's c' and phi' on average.
    #[test]
    fn the_written_envelope_recovers_the_site_values_on_average() {
        let p = preset(&eff_line(8.0, 28.0, 0.3), &eff_line(8.0, 28.0, 0.3));
        let (mut n, mut c, mut phi) = (0, 0.0, 0.0);
        for seed in 0..60 {
            for (treg, _) in sets(&file(&p, seed, 5)) {
                n += 1;
                c += num(&treg["TREG_COH"]);
                phi += num(&treg["TREG_PHI"]);
            }
        }
        let (c, phi) = (c / f64::from(n), phi / f64::from(n));
        assert!((c - 8.0).abs() < 0.6, "c' {c}");
        assert!((phi - 28.0).abs() < 0.6, "phi' {phi}");
    }

    /// TREG is scheduled like TRIG: one set per tested sample, on a sample type
    /// that can carry it, and a smaller site is a prefix of a larger one.
    #[test]
    fn a_smaller_site_is_a_prefix_of_a_larger_one() {
        let p = preset(&eff_line(8.0, 28.0, 0.3), &eff_line(5.0, 30.0, 0.1));
        for seed in 0..10 {
            let (small, large) = (file(&p, seed, 3), file(&p, seed, 7));
            for g in ["TREG", "TRET"] {
                let (s, l) = (rows(&small, g), rows(&large, g));
                assert!(l.starts_with(&s), "{g} seed {seed}");
            }
        }
    }

    /// Changing one unit's effective strength moves only that unit's TREG and
    /// TRET rows.
    #[test]
    fn a_change_in_one_units_values_moves_only_its_rows() {
        let wlc = eff_line(8.0, 28.0, 0.3);
        let base = preset(&wlc, &eff_line(5.0, 30.0, 0.1));
        let other = preset(&wlc, &eff_line(15.0, 22.0, 0.6));
        let mut moved = 0;
        for seed in 0..10 {
            let site = build::build(&base, seed, 5);
            let a = build::lower(&site, &base, seed, Grid::Local);
            let b = file(&other, seed, 5);
            let of_wlc = |m: &ProjectModel, g: &str| {
                let in_wlc = |r: &RowMap| {
                    let hole = site.holes.iter().find(|h| h.id == r["LOCA_ID"]).unwrap();
                    let depth = num(&r["SPEC_DPTH"]);
                    hole.strata
                        .iter()
                        .find(|s| depth >= s.top && depth <= s.base)
                        .unwrap()
                        .unit
                        == "WLC"
                };
                rows(m, g).into_iter().filter(in_wlc).collect::<Vec<_>>()
            };
            for g in ["TREG", "TRET"] {
                assert_eq!(of_wlc(&a, g), of_wlc(&b, g), "{g} seed {seed}");
            }
            moved += usize::from(rows(&a, "TRET") != rows(&b, "TRET"));
        }
        assert!(moved > 5, "LC's values moved nothing");
    }

    #[test]
    fn an_effective_file_is_rust_clean_across_seeds() {
        let p = preset(&eff_line(8.0, 28.0, 0.3), &eff_line(5.0, 30.0, 0.1));
        let dir = tempfile::tempdir().unwrap();
        let mut sets_seen = 0;
        for seed in 0..25 {
            let m = file(&p, seed, 6);
            sets_seen += rows(&m, "TREG").len();
            let path = dir.path().join("site.ags");
            std::fs::write(&path, emit_to_string(&m)).unwrap();
            assert_eq!(rust_check(&path).0, RustResult::Clean, "seed {seed}");
        }
        assert!(sets_seen > 50);
    }

    /// A preset with no `effective_strength` and no TREG entry gains no group.
    #[test]
    fn a_preset_without_the_table_writes_no_treg_or_tret() {
        let m = file(&Preset::from_toml(FIXTURE).unwrap(), 2, 5);
        assert!(
            m.groups
                .iter()
                .all(|g| g.code != "TREG" && g.code != "TRET")
        );
    }
}
