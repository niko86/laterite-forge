//! Which lab-test groups need which minimum sample quality class, and which
//! quality class each `SAMP_TYPE` code can deliver.
//!
//! A forge-authored draft for owner review (`data/sample-quality/README.md`).
//! The site suite uses `permitted_samp_types`; choosing `SAMP_TYPE` by unit is
//! the rest of niko86/laterite-forge#3, so the module allows `dead_code` until
//! then.
#![allow(dead_code)]

use std::collections::BTreeMap;

use anyhow::{Context, Result, bail};
use laterite_ags4_validator::Dictionary;
use serde::Deserialize;

const TABLE: &str = include_str!("../data/sample-quality/table.json");

/// The only bases a row may carry. No table or clause numbers, no other
/// citations.
const BASES: &[&str] = &[
    "BS EN 1997-2",
    "BS EN ISO 22475-1",
    "BS EN 1997-2 and BS EN ISO 22475-1",
    "forge judgement",
];

#[derive(Deserialize)]
struct RawTable {
    tests: Vec<RawTest>,
    samp_types: Vec<RawSampType>,
}

#[derive(Deserialize)]
struct RawTest {
    group: String,
    min_class: u8,
    basis: String,
}

#[derive(Deserialize)]
struct RawSampType {
    code: String,
    /// `None` is "not a soil sample for lab testing" (water, gas, concrete).
    class: Option<u8>,
    basis: String,
}

/// A validated table. Class 1 is the best quality, 5 the worst; a sample of
/// class `c` can carry a test needing at most `min_class` when `c <= min_class`.
#[derive(Debug)]
pub struct SampleQuality {
    tests: BTreeMap<String, u8>,
    samp_types: BTreeMap<String, Option<u8>>,
}

fn check_basis(basis: &str, what: &str) -> Result<()> {
    if !BASES.contains(&basis) {
        bail!("{what}: basis {basis:?} is not one of {BASES:?}");
    }
    Ok(())
}

fn check_class(class: u8, what: &str) -> Result<()> {
    if !(1..=5).contains(&class) {
        bail!("{what}: quality class {class} is outside 1-5");
    }
    Ok(())
}

impl SampleQuality {
    /// The table shipped in `data/sample-quality/table.json`, checked against
    /// the bundled dictionary.
    pub fn bundled(dict: &Dictionary<'static>) -> Result<Self> {
        Self::from_json(TABLE, dict)
    }

    /// Parse and validate a table. Refuses a test group or `SAMP_TYPE` code the
    /// dictionary does not define, a class outside 1-5, a basis outside the
    /// allowed set, a duplicate row, and a test group no `SAMP_TYPE` may carry.
    pub fn from_json(json: &str, dict: &Dictionary<'static>) -> Result<Self> {
        let raw: RawTable =
            serde_json::from_str(json).context("parsing the sample-quality table")?;
        let known: Vec<&str> = dict.abbr_codes("SAMP_TYPE");
        let mut samp_types = BTreeMap::new();
        for r in raw.samp_types {
            let what = format!("SAMP_TYPE {}", r.code);
            if !known.contains(&r.code.as_str()) {
                bail!("{what}: not in the bundled dictionary's SAMP_TYPE list");
            }
            if let Some(c) = r.class {
                check_class(c, &what)?;
            }
            check_basis(&r.basis, &what)?;
            if samp_types.insert(r.code, r.class).is_some() {
                bail!("{what}: duplicate row");
            }
        }
        let mut tests = BTreeMap::new();
        for r in raw.tests {
            let what = format!("test group {}", r.group);
            if dict.group(&r.group).is_none() {
                bail!("{what}: not in the bundled dictionary");
            }
            check_class(r.min_class, &what)?;
            check_basis(&r.basis, &what)?;
            if tests.insert(r.group, r.min_class).is_some() {
                bail!("{what}: duplicate row");
            }
        }
        let table = Self { tests, samp_types };
        for g in table.tests.keys() {
            if table.permitted_samp_types(g).is_empty() {
                bail!("test group {g}: no SAMP_TYPE can carry it");
            }
        }
        Ok(table)
    }

    /// The minimum quality class a test group needs, if the group has a row.
    pub fn min_class(&self, group: &str) -> Option<u8> {
        self.tests.get(group).copied()
    }

    /// The best class a `SAMP_TYPE` can deliver. `None` for an unknown code and
    /// for a code that is not a soil sample for lab testing.
    pub fn class_of(&self, samp_type: &str) -> Option<u8> {
        self.samp_types.get(samp_type).copied().flatten()
    }

    /// The `SAMP_TYPE` codes that may carry `group`, sorted. Empty for a group
    /// with no row.
    pub fn permitted_samp_types(&self, group: &str) -> Vec<&str> {
        let Some(need) = self.min_class(group) else {
            return Vec::new();
        };
        self.samp_types
            .iter()
            .filter(|(_, c)| c.is_some_and(|c| c <= need))
            .map(|(code, _)| code.as_str())
            .collect()
    }

    /// Every test group with a row, sorted.
    pub fn groups(&self) -> impl Iterator<Item = &str> {
        self.tests.keys().map(String::as_str)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::synth::depth::safe_samp_children;
    use laterite_ags4_validator::DictVersion;

    fn dict() -> Dictionary<'static> {
        Dictionary::bundled(DictVersion::V4_2)
    }

    fn table() -> SampleQuality {
        SampleQuality::bundled(&dict()).expect("the shipped table loads")
    }

    #[test]
    fn every_generated_lab_test_group_has_a_row_and_a_permissible_samp_type() {
        let t = table();
        for g in safe_samp_children(&dict()) {
            assert!(t.min_class(g).is_some(), "no row for {g}");
            assert!(
                !t.permitted_samp_types(g).is_empty(),
                "no permissible SAMP_TYPE for {g}"
            );
        }
    }

    #[test]
    fn the_table_has_no_group_the_generator_does_not_emit() {
        let emitted = safe_samp_children(&dict());
        for g in table().groups() {
            assert!(emitted.contains(&g), "{g} is not an emitted lab-test group");
        }
    }

    #[test]
    fn every_samp_type_row_is_in_the_dictionary_and_every_dictionary_code_has_a_row() {
        let d = dict();
        let t = table();
        for code in d.abbr_codes("SAMP_TYPE") {
            assert!(
                t.samp_types.contains_key(code),
                "no row for SAMP_TYPE {code}"
            );
        }
        for code in t.samp_types.keys() {
            assert!(d.abbr_codes("SAMP_TYPE").contains(&code.as_str()));
        }
    }

    #[test]
    fn triaxial_does_not_permit_bulk_and_limits_permit_bulk_and_small_disturbed() {
        let t = table();
        for g in ["TRIG", "TREG"] {
            let ok = t.permitted_samp_types(g);
            assert!(!ok.contains(&"B"), "{g} permits B");
            assert!(ok.contains(&"U") && ok.contains(&"UT") && ok.contains(&"BLK"));
        }
        let ok = t.permitted_samp_types("LLPL");
        assert!(ok.contains(&"B") && ok.contains(&"D"), "LLPL: {ok:?}");
        for not_soil in ["W", "EW", "G", "CONCB", "CONCC"] {
            assert_eq!(t.class_of(not_soil), None);
            assert!(!t.permitted_samp_types("CHOC").contains(&not_soil));
        }
    }

    #[test]
    fn every_row_has_an_allowed_non_empty_basis_with_no_clause_numbers() {
        let raw: RawTable = serde_json::from_str(TABLE).unwrap();
        let bases = raw
            .tests
            .iter()
            .map(|r| &r.basis)
            .chain(raw.samp_types.iter().map(|r| &r.basis));
        for b in bases {
            assert!(!b.is_empty());
            assert!(BASES.contains(&b.as_str()), "{b}");
            let lower = b.to_lowercase();
            assert!(!lower.contains("table") && !lower.contains("clause") && !b.contains('§'));
        }
    }

    #[test]
    fn the_loader_refuses_unknown_names_bad_classes_and_unservable_groups() {
        let d = dict();
        let good = r#"{"tests":[{"group":"LLPL","min_class":4,"basis":"forge judgement"}],
            "samp_types":[{"code":"B","class":4,"basis":"forge judgement"}]}"#;
        assert!(SampleQuality::from_json(good, &d).is_ok());
        let cases = [
            good.replace("LLPL", "NOPE"),
            good.replace("\"B\"", "\"ZZ\""),
            good.replace("\"class\":4", "\"class\":6"),
            good.replace("\"min_class\":4", "\"min_class\":0"),
            good.replace("forge judgement", "BS 5930 Table 4"),
            good.replace("\"min_class\":4", "\"min_class\":1"),
        ];
        for c in cases {
            assert!(SampleQuality::from_json(&c, &d).is_err(), "{c}");
        }
    }
}
