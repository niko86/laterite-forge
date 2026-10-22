// The catalogue is not called from any command yet; it is wired in by #9.
#![allow(dead_code)]
//! Catalogue of named, cited phase relations and description-term mappings.
//!
//! Part of the property model (ADR 0001): a preset picks entries by name and
//! never writes formulas. Phase relations are exact definitions, so there is no
//! scatter. All quantities are SI: densities in kg/m3, unit weight in kN/m3,
//! gravity in m/s2. Moisture content is a fraction for the phase relations
//! (`w_fraction`, 0.25 = 25 %) and a percentage for the Atterberg indices
//! (`*_percent`), as laboratory results are reported. Inputs outside their
//! physical domain return [`DomainError`], never NaN.

use std::fmt;
use std::sync::OnceLock;

use serde::Deserialize;

const TERMS_JSON: &str = include_str!("../data/bs5930/terms.json");

/// Standard gravity, m/s2 (the conventional value, a definition).
pub const STANDARD_GRAVITY_M_S2: f64 = 9.80665;

/// A catalogue input was outside its physical domain.
#[derive(Debug, Clone, PartialEq)]
pub struct DomainError {
    pub param: &'static str,
    pub value: f64,
    pub expected: &'static str,
}

impl fmt::Display for DomainError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} = {} is outside its domain ({})",
            self.param, self.value, self.expected
        )
    }
}

impl std::error::Error for DomainError {}

fn check(
    ok: bool,
    param: &'static str,
    value: f64,
    expected: &'static str,
) -> Result<(), DomainError> {
    if ok && value.is_finite() {
        Ok(())
    } else {
        Err(DomainError {
            param,
            value,
            expected,
        })
    }
}

/// Void ratio e = w * Gs / Sr.
///
/// `w_fraction` >= 0 (0.25 = 25 %), `particle_density_ratio_gs` > 0
/// (dimensionless), `degree_of_saturation_sr` in (0, 1].
pub fn void_ratio(
    w_fraction: f64,
    particle_density_ratio_gs: f64,
    degree_of_saturation_sr: f64,
) -> Result<f64, DomainError> {
    check(w_fraction >= 0.0, "w_fraction", w_fraction, ">= 0")?;
    check(
        particle_density_ratio_gs > 0.0,
        "particle_density_ratio_gs",
        particle_density_ratio_gs,
        "> 0",
    )?;
    check(
        degree_of_saturation_sr > 0.0 && degree_of_saturation_sr <= 1.0,
        "degree_of_saturation_sr",
        degree_of_saturation_sr,
        "in (0, 1]",
    )?;
    Ok(w_fraction * particle_density_ratio_gs / degree_of_saturation_sr)
}

/// Bulk density in kg/m3: rho = Gs * `rho_w` * (1 + w) / (1 + e), with e from
/// [`void_ratio`]. `water_density_kg_m3` > 0.
pub fn bulk_density_kg_m3(
    w_fraction: f64,
    particle_density_ratio_gs: f64,
    degree_of_saturation_sr: f64,
    water_density_kg_m3: f64,
) -> Result<f64, DomainError> {
    let e = void_ratio(
        w_fraction,
        particle_density_ratio_gs,
        degree_of_saturation_sr,
    )?;
    check(
        water_density_kg_m3 > 0.0,
        "water_density_kg_m3",
        water_density_kg_m3,
        "> 0",
    )?;
    Ok(particle_density_ratio_gs * water_density_kg_m3 * (1.0 + w_fraction) / (1.0 + e))
}

/// Dry density in kg/m3: `rho_d` = rho / (1 + w). `bulk_density_kg_m3` > 0,
/// `w_fraction` >= 0.
pub fn dry_density_kg_m3(bulk_density_kg_m3: f64, w_fraction: f64) -> Result<f64, DomainError> {
    check(
        bulk_density_kg_m3 > 0.0,
        "bulk_density_kg_m3",
        bulk_density_kg_m3,
        "> 0",
    )?;
    check(w_fraction >= 0.0, "w_fraction", w_fraction, ">= 0")?;
    Ok(bulk_density_kg_m3 / (1.0 + w_fraction))
}

/// Unit weight in kN/m3: gamma = rho * g, with g = [`STANDARD_GRAVITY_M_S2`]
/// and the 1000 N/kN conversion. `density_kg_m3` > 0.
pub fn unit_weight_kn_m3(density_kg_m3: f64) -> Result<f64, DomainError> {
    check(density_kg_m3 > 0.0, "density_kg_m3", density_kg_m3, "> 0")?;
    Ok(density_kg_m3 * STANDARD_GRAVITY_M_S2 / 1000.0)
}

/// Plasticity index PI = LL - PL, in percent. Both limits must be >= 0. A
/// result <= 0 means non-plastic; it is returned as is.
pub fn plasticity_index_percent(
    liquid_limit_percent: f64,
    plastic_limit_percent: f64,
) -> Result<f64, DomainError> {
    check(
        liquid_limit_percent >= 0.0,
        "liquid_limit_percent",
        liquid_limit_percent,
        ">= 0",
    )?;
    check(
        plastic_limit_percent >= 0.0,
        "plastic_limit_percent",
        plastic_limit_percent,
        ">= 0",
    )?;
    Ok(liquid_limit_percent - plastic_limit_percent)
}

/// Consistency index Ic = (LL - w) / PI, all in percent. `Ok(None)` when
/// PI <= 0 (non-plastic): there is no Ic. `moisture_percent` >= 0.
pub fn consistency_index(
    liquid_limit_percent: f64,
    plastic_limit_percent: f64,
    moisture_percent: f64,
) -> Result<Option<f64>, DomainError> {
    let pi = plasticity_index_percent(liquid_limit_percent, plastic_limit_percent)?;
    check(
        moisture_percent >= 0.0,
        "moisture_percent",
        moisture_percent,
        ">= 0",
    )?;
    if pi <= 0.0 {
        return Ok(None);
    }
    Ok(Some((liquid_limit_percent - moisture_percent) / pi))
}

/// The consistency terms Ic maps to; identical to the BS 5930 engine's
/// Table 8 vocabulary (asserted by a test).
pub const CONSISTENCY_TERMS: [&str; 5] = ["Very soft", "Soft", "Firm", "Stiff", "Very stiff"];

/// Consistency term from Ic, with BS EN ISO 14688-2 boundaries, lower bound
/// inclusive: < 0.25 Very soft; [0.25, 0.50) Soft; [0.50, 0.75) Firm;
/// [0.75, 1.00) Stiff; >= 1.00 Very stiff. `ic` must be finite.
pub fn consistency_term_from_ic(ic: f64) -> Result<&'static str, DomainError> {
    check(true, "ic", ic, "finite")?;
    let i = if ic < 0.25 {
        0
    } else if ic < 0.50 {
        1
    } else if ic < 0.75 {
        2
    } else if ic < 1.00 {
        3
    } else {
        4
    };
    Ok(CONSISTENCY_TERMS[i])
}

#[derive(Deserialize)]
struct RawDensity {
    term: String,
    #[serde(rename = "sptN")]
    spt_n: String,
    #[serde(rename = "ref")]
    reference: String,
}

#[derive(Deserialize)]
struct RawTerms {
    #[serde(rename = "relativeDensity")]
    relative_density: Vec<RawDensity>,
}

/// One relative-density band, read from the vendored data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SptBand {
    pub term: String,
    /// Exclusive lower bound (`None` for the first band, which starts at 0).
    pub above: Option<u32>,
    /// Inclusive upper bound (`None` for the open-ended top band).
    pub up_to: Option<u32>,
    /// The data's own `ref`.
    pub basis: String,
}

fn parse_band(spt_n: &str) -> (Option<u32>, Option<u32>) {
    if let Some(rest) = spt_n.strip_prefix('>') {
        return (Some(rest.trim().parse().expect("sptN lower bound")), None);
    }
    let (lo, hi) = spt_n.split_once('-').expect("sptN band is 'a - b'");
    let lo: u32 = lo.trim().parse().expect("sptN lower bound");
    let hi: u32 = hi.trim().parse().expect("sptN upper bound");
    (if lo == 0 { None } else { Some(lo) }, Some(hi))
}

/// The SPT N bands from `data/bs5930/terms.json`, in order.
///
/// The data states shared edges ("0 - 4", "4 - 10") and an exclusive top
/// (">50"), so bands are read as (`above`, `up_to`]: N = 4 is Very loose and
/// N = 50 is Dense. The first band also includes N = 0.
pub fn spt_bands() -> &'static [SptBand] {
    static BANDS: OnceLock<Vec<SptBand>> = OnceLock::new();
    BANDS.get_or_init(|| {
        let raw: RawTerms = serde_json::from_str(TERMS_JSON).expect("vendored terms.json is valid");
        raw.relative_density
            .into_iter()
            .map(|d| {
                let (above, up_to) = parse_band(&d.spt_n);
                SptBand {
                    term: d.term,
                    above,
                    up_to,
                    basis: d.reference,
                }
            })
            .collect()
    })
}

/// Relative-density term for an SPT N value, from [`spt_bands`].
pub fn spt_relative_density_term(spt_n: u32) -> &'static str {
    spt_bands()
        .iter()
        .find(|b| b.above.is_none_or(|a| spt_n > a) && b.up_to.is_none_or(|u| spt_n <= u))
        .map(|b| b.term.as_str())
        .expect("the vendored bands cover every N")
}

/// What kind of catalogue entry this is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    PhaseRelation,
    TermMapping,
}

/// A registered catalogue entry. Names are stable identifiers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub name: &'static str,
    pub kind: Kind,
    pub basis: String,
}

/// Every catalogue entry.
pub fn entries() -> &'static [Entry] {
    static ENTRIES: OnceLock<Vec<Entry>> = OnceLock::new();
    ENTRIES.get_or_init(|| {
        let phase = |name, basis: &str| Entry {
            name,
            kind: Kind::PhaseRelation,
            basis: basis.to_string(),
        };
        vec![
            phase("phase.void_ratio", "phase relationship (definition)"),
            phase("phase.bulk_density", "phase relationship (definition)"),
            phase("phase.dry_density", "phase relationship (definition)"),
            phase("phase.unit_weight", "phase relationship (definition)"),
            phase("plasticity.index", "Atterberg limits index (definition)"),
            phase(
                "plasticity.consistency_index",
                "Atterberg limits index (definition)",
            ),
            Entry {
                name: "term.consistency_index",
                kind: Kind::TermMapping,
                basis: "BS EN ISO 14688-2".to_string(),
            },
            Entry {
                name: "term.spt_relative_density",
                kind: Kind::TermMapping,
                basis: spt_bands()[0].basis.clone(),
            },
        ]
    })
}

/// Look an entry up by name.
pub fn lookup(name: &str) -> Option<&'static Entry> {
    entries().iter().find(|e| e.name == name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::synth::bs5930::Vocab;

    fn close(a: f64, b: f64, tol: f64) -> bool {
        (a - b).abs() <= tol
    }

    #[test]
    fn void_ratio_worked_example() {
        assert!(close(void_ratio(0.25, 2.70, 1.0).unwrap(), 0.675, 1e-12));
        assert!(close(void_ratio(0.10, 2.65, 0.5).unwrap(), 0.53, 1e-12));
    }

    #[test]
    fn bulk_density_worked_example() {
        let rho = bulk_density_kg_m3(0.25, 2.70, 1.0, 1000.0).unwrap();
        assert!(close(rho, 2015.0, 0.5), "{rho}");
    }

    #[test]
    fn dry_density_worked_example() {
        assert!(close(
            dry_density_kg_m3(2000.0, 0.25).unwrap(),
            1600.0,
            1e-9
        ));
    }

    #[test]
    fn unit_weight_worked_example() {
        assert!(close(unit_weight_kn_m3(2000.0).unwrap(), 19.6133, 1e-9));
    }

    #[test]
    fn plasticity_worked_example() {
        assert!(close(
            plasticity_index_percent(70.0, 25.0).unwrap(),
            45.0,
            1e-12
        ));
        let ic = consistency_index(70.0, 25.0, 30.0).unwrap().unwrap();
        assert!(close(ic, 0.889, 0.0005), "{ic}");
        assert_eq!(consistency_term_from_ic(ic).unwrap(), "Stiff");
    }

    #[test]
    fn non_plastic_has_no_ic() {
        assert_eq!(consistency_index(25.0, 25.0, 10.0).unwrap(), None);
        assert_eq!(consistency_index(20.0, 25.0, 10.0).unwrap(), None);
    }

    #[test]
    fn ic_boundary_0_25_is_soft() {
        assert_eq!(consistency_term_from_ic(0.25).unwrap(), "Soft");
        assert_eq!(consistency_term_from_ic(0.2499).unwrap(), "Very soft");
    }

    #[test]
    fn ic_boundary_0_50_is_firm() {
        assert_eq!(consistency_term_from_ic(0.50).unwrap(), "Firm");
        assert_eq!(consistency_term_from_ic(0.4999).unwrap(), "Soft");
    }

    #[test]
    fn ic_boundary_0_75_is_stiff() {
        assert_eq!(consistency_term_from_ic(0.75).unwrap(), "Stiff");
        assert_eq!(consistency_term_from_ic(0.7499).unwrap(), "Firm");
    }

    #[test]
    fn ic_boundary_1_00_is_very_stiff() {
        assert_eq!(consistency_term_from_ic(1.00).unwrap(), "Very stiff");
        assert_eq!(consistency_term_from_ic(0.9999).unwrap(), "Stiff");
        assert_eq!(consistency_term_from_ic(-0.5).unwrap(), "Very soft");
        assert_eq!(consistency_term_from_ic(3.0).unwrap(), "Very stiff");
    }

    #[test]
    fn ic_terms_equal_the_bs5930_engine_vocabulary() {
        let vocab = Vocab::load();
        let engine: Vec<&str> = vocab
            .consistency_terms()
            .iter()
            .map(String::as_str)
            .collect();
        assert_eq!(engine, CONSISTENCY_TERMS);
    }

    #[test]
    fn spt_bands_are_read_from_the_data() {
        let got: Vec<(&str, Option<u32>, Option<u32>)> = spt_bands()
            .iter()
            .map(|b| (b.term.as_str(), b.above, b.up_to))
            .collect();
        assert_eq!(
            got,
            vec![
                ("Very loose", None, Some(4)),
                ("Loose", Some(4), Some(10)),
                ("Medium dense", Some(10), Some(30)),
                ("Dense", Some(30), Some(50)),
                ("Very dense", Some(50), None),
            ]
        );
    }

    #[test]
    fn spt_n_maps_to_band_and_edges() {
        let cases = [
            (0, "Very loose"),
            (2, "Very loose"),
            (4, "Very loose"),
            (5, "Loose"),
            (10, "Loose"),
            (11, "Medium dense"),
            (30, "Medium dense"),
            (31, "Dense"),
            (50, "Dense"),
            (51, "Very dense"),
            (200, "Very dense"),
        ];
        for (n, term) in cases {
            assert_eq!(spt_relative_density_term(n), term, "N = {n}");
        }
    }

    #[test]
    fn spt_basis_is_the_datas_ref() {
        let e = lookup("term.spt_relative_density").unwrap();
        assert_eq!(
            e.basis,
            "BS 5930:2015+A1:2020 Tables 10 and 11 (clause 33.3.2)"
        );
        assert_eq!(
            lookup("term.consistency_index").unwrap().basis,
            "BS EN ISO 14688-2"
        );
    }

    #[test]
    fn entries_have_unique_names_and_non_empty_basis() {
        let mut names: Vec<&str> = entries().iter().map(|e| e.name).collect();
        assert_eq!(names.len(), 8);
        assert!(entries().iter().all(|e| !e.basis.trim().is_empty()));
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), 8);
        assert_eq!(
            lookup("phase.bulk_density").unwrap().kind,
            Kind::PhaseRelation
        );
        assert_eq!(
            lookup("term.consistency_index").unwrap().kind,
            Kind::TermMapping
        );
        assert!(lookup("nope").is_none());
    }

    #[test]
    fn out_of_domain_inputs_are_errors() {
        assert!(void_ratio(0.2, 2.7, 0.0).is_err());
        assert!(void_ratio(0.2, 2.7, 1.01).is_err());
        assert!(void_ratio(0.2, 0.0, 1.0).is_err());
        assert!(void_ratio(-0.01, 2.7, 1.0).is_err());
        assert!(void_ratio(f64::NAN, 2.7, 1.0).is_err());
        assert!(bulk_density_kg_m3(0.2, 2.7, 1.0, 0.0).is_err());
        assert!(bulk_density_kg_m3(0.2, 2.7, -1.0, 1000.0).is_err());
        assert!(dry_density_kg_m3(0.0, 0.1).is_err());
        assert!(dry_density_kg_m3(2000.0, -0.1).is_err());
        assert!(unit_weight_kn_m3(-1.0).is_err());
        assert!(unit_weight_kn_m3(f64::INFINITY).is_err());
        assert!(plasticity_index_percent(-1.0, 5.0).is_err());
        assert!(consistency_index(70.0, 25.0, -1.0).is_err());
        assert!(consistency_term_from_ic(f64::NAN).is_err());
        let msg = void_ratio(0.2, 2.7, 0.0).unwrap_err().to_string();
        assert!(msg.contains("degree_of_saturation_sr"), "{msg}");
    }
}
