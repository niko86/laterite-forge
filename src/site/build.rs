//! The typed site layer (site, holes, strata) and its lowering into the
//! `ProjectModel` the emitter writes.
//!
//! Nothing here touches the scaffold builder, so the scaffold pins are out of
//! reach (ADR 0001). Every draw comes from a keyed stream (`draw::Key`), so a
//! hole depends only on its own index, and a stratum only on its hole and
//! unit.

use laterite_ags4_validator::{DictVersion, Dictionary};

use super::draw::{Key, uniform};
use super::ground::Field;
use super::preset::{Preset, Unit};
use crate::synth::bs5930::{self, UnitDescription};
use crate::synth::generic;
use crate::synth::model::{Group, ProjectModel, Row, collect_abbr, collect_type, collect_unit};

/// How holes are located beyond the local grid.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Grid {
    /// Local site grid only (`LOCA_LOCX`/`LOCY`).
    Local,
    /// Also British National Grid coordinates, offset from an origin the
    /// consumer chose (`--grid bng --origin E,N`).
    Bng { easting: f64, northing: f64 },
}

/// One generated site: its holes, each with its strata.
#[derive(Debug, Clone, PartialEq)]
pub struct Site {
    pub holes: Vec<Hole>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Hole {
    pub id: String,
    /// Its `LOCA_TYPE` code.
    pub loca_type: String,
    /// Local grid position, metres from the site origin.
    pub x: f64,
    pub y: f64,
    pub ground_level: f64,
    /// Final depth, rounded to the 2 decimal places it is written with.
    pub final_depth: f64,
    /// Top-down, contiguous from 0 to `final_depth`.
    pub strata: Vec<Stratum>,
}

/// The part of one unit met in one hole.
#[derive(Debug, Clone, PartialEq)]
pub struct Stratum {
    pub unit: String,
    pub top: f64,
    pub base: f64,
    pub description: String,
}

/// The hole count `gen --site` uses when nothing sets one: 3–7, like a
/// scaffold file, drawn from its own stream so it moves no hole.
pub fn default_hole_count(seed: u64) -> usize {
    // `range(3, 7)` is in 3..=7.
    #[allow(clippy::cast_possible_truncation)]
    let n = Key::site("hole-count").rng(seed).range(3, 7) as usize;
    n
}

/// Every problem that stops `preset` generating, beyond what the loader
/// checks: description constraints the BS 5930 vocabulary cannot meet.
pub fn problems(preset: &Preset) -> Vec<String> {
    let vocab = bs5930::vocab();
    preset
        .units
        .iter()
        .enumerate()
        .flat_map(|(i, u)| {
            unit_description(u)
                .problems(vocab)
                .into_iter()
                .map(move |p| format!("units[{i}] ({}).description: {p}", u.id))
        })
        .collect()
}

fn unit_description(u: &Unit) -> UnitDescription {
    UnitDescription {
        principals: u.description.principals.clone(),
        secondaries: u.description.secondaries.clone(),
        colours: u.description.colours.clone(),
    }
}

/// Generate `holes` holes of `preset` under `seed`. Call [`problems`] first:
/// a preset with problems panics here.
pub fn build(preset: &Preset, seed: u64, holes: usize) -> Site {
    let extent = preset.site.extent_m;
    let ground = Field::new(&mut Key::site("ground-level").rng(seed), extent);
    let surfaces: Vec<Field> = preset
        .units
        .iter()
        .map(|u| Field::new(&mut Key::unit("thickness", &u.id).rng(seed), extent))
        .collect();
    let total_weight: u64 = preset.hole_types.iter().map(|t| u64::from(t.weight)).sum();

    let holes = (0..holes)
        .map(|k| {
            let mut rng = Key::hole("hole", k).rng(seed);
            let x = round2(uniform(&mut rng, 0.0, extent.x));
            let y = round2(uniform(&mut rng, 0.0, extent.y));
            let mut pick = rng.below(total_weight);
            let hole_type = preset
                .hole_types
                .iter()
                .find(|t| {
                    let w = u64::from(t.weight);
                    if pick < w {
                        true
                    } else {
                        pick -= w;
                        false
                    }
                })
                .expect("pick < total weight");
            let r = &hole_type.final_depth_m;
            let final_depth = round2(uniform(&mut rng, r.min, r.max));
            Hole {
                id: format!("BH{}", k + 1),
                loca_type: hole_type.code.clone(),
                x,
                y,
                ground_level: round2(ground.in_range(&preset.site.ground_level_m, x, y)),
                final_depth,
                strata: strata(preset, &surfaces, seed, k, x, y, final_depth),
            }
        })
        .collect();
    Site { holes }
}

/// The strata of hole `k`: one per unit present, top-down, from 0 to
/// `final_depth`. A unit whose thickness here is zero or less is absent; the
/// last unit runs to final depth; nothing continues below final depth.
fn strata(
    preset: &Preset,
    surfaces: &[Field],
    seed: u64,
    k: usize,
    x: f64,
    y: f64,
    final_depth: f64,
) -> Vec<Stratum> {
    let vocab = bs5930::vocab();
    let mut out = Vec::new();
    let mut top = 0.0;
    for (unit, surface) in preset.units.iter().zip(surfaces) {
        if top >= final_depth {
            break;
        }
        let base = match &unit.thickness_m {
            Some(range) => {
                let thickness = surface.in_range(range, x, y);
                if thickness <= 0.0 {
                    continue;
                }
                round2(top + thickness).min(final_depth)
            }
            None => final_depth,
        };
        // A sliver thinner than the written precision would give a
        // zero-thickness row.
        if base <= top {
            continue;
        }
        let mut rng = Key::stratum("description", k, &unit.id).rng(seed);
        let description = bs5930::describe_unit(vocab, &mut rng, &unit_description(unit), None)
            .expect("problems() was checked before build");
        out.push(Stratum {
            unit: unit.id.clone(),
            top,
            base,
            description: description.text,
        });
        top = base;
    }
    out
}

fn round2(v: f64) -> f64 {
    (v * 100.0).round() / 100.0
}

fn m2(v: f64) -> String {
    format!("{v:.2}")
}

/// The synthetic marker every `--site` file carries in `PROJ_MEMO`.
pub fn marker(preset: &Preset) -> String {
    format!(
        "SYNTHETIC: generated by laterite-ags4-forge from site preset {} v{}. \
         Not real ground data.",
        preset.name, preset.version
    )
}

/// Lower `site` into the groups the emitter writes: PROJ (with the marker),
/// TRAN, LOCA, GEOL, then ABBR/UNIT/TYPE covering exactly what those use.
pub fn lower(site: &Site, preset: &Preset, seed: u64, grid: Grid) -> ProjectModel {
    let dict = Dictionary::bundled(DictVersion::V4_2);
    let mut groups = vec![proj(preset, seed), tran(seed), loca(site, grid)];
    groups.push(geol(site));
    let abbr = collect_abbr(&groups, &dict);
    let unit = collect_unit(&groups);
    let types = collect_type(&groups);
    groups.extend([abbr, unit, types]);
    ProjectModel { groups }
}

fn group(code: &str, columns: &[(&str, &str, &str)], rows: Vec<Row>) -> Group {
    Group {
        code: code.to_string(),
        headings: columns.iter().map(|c| c.0.to_string()).collect(),
        units: columns.iter().map(|c| c.1.to_string()).collect(),
        types: columns.iter().map(|c| c.2.to_string()).collect(),
        rows,
    }
}

fn proj(preset: &Preset, seed: u64) -> Group {
    let mut rng = Key::site("proj").rng(seed);
    group(
        "PROJ",
        &[
            ("PROJ_ID", "", "ID"),
            ("PROJ_NAME", "", "X"),
            ("PROJ_MEMO", "", "X"),
        ],
        vec![Row::owned(vec![
            format!("P{:04}", rng.range(1000, 9999)),
            generic::project_name(&mut rng),
            marker(preset),
        ])],
    )
}

fn tran(seed: u64) -> Group {
    let mut rng = Key::site("tran").rng(seed);
    group(
        "TRAN",
        &[
            ("TRAN_ISNO", "", "X"),
            ("TRAN_DATE", "yyyy-mm-dd", "DT"),
            ("TRAN_PROD", "", "X"),
            ("TRAN_STAT", "", "X"),
            ("TRAN_AGS", "", "X"),
            ("TRAN_RECV", "", "X"),
            ("TRAN_DLIM", "", "X"),
            ("TRAN_RCON", "", "X"),
        ],
        vec![Row::owned(vec![
            "1".to_string(),
            generic::iso_date(&mut rng),
            generic::producer(&mut rng),
            "Draft".to_string(),
            "4.2".to_string(),
            generic::producer(&mut rng),
            "|".to_string(),
            "+".to_string(),
        ])],
    )
}

/// LOCA, in dictionary heading order. National-grid columns only with
/// `Grid::Bng`.
fn loca(site: &Site, grid: Grid) -> Group {
    let mut columns = vec![("LOCA_ID", "", "ID"), ("LOCA_TYPE", "", "PA")];
    if matches!(grid, Grid::Bng { .. }) {
        columns.extend([
            ("LOCA_NATE", "m", "2DP"),
            ("LOCA_NATN", "m", "2DP"),
            ("LOCA_GREF", "", "PA"),
        ]);
    }
    columns.extend([
        ("LOCA_GL", "m", "2DP"),
        ("LOCA_FDEP", "m", "2DP"),
        ("LOCA_LOCX", "m", "2DP"),
        ("LOCA_LOCY", "m", "2DP"),
        ("LOCA_LREF", "", "X"),
    ]);
    let rows = site
        .holes
        .iter()
        .map(|h| {
            let mut v = vec![h.id.clone(), h.loca_type.clone()];
            if let Grid::Bng { easting, northing } = grid {
                v.extend([m2(easting + h.x), m2(northing + h.y), "OSGB".to_string()]);
            }
            v.extend([
                m2(h.ground_level),
                m2(h.final_depth),
                m2(h.x),
                m2(h.y),
                "Synthetic site grid".to_string(),
            ]);
            Row::owned(v)
        })
        .collect();
    group("LOCA", &columns, rows)
}

fn geol(site: &Site) -> Group {
    let rows = site
        .holes
        .iter()
        .flat_map(|h| {
            h.strata.iter().map(|s| {
                Row::owned(vec![
                    h.id.clone(),
                    m2(s.top),
                    m2(s.base),
                    s.description.clone(),
                ])
            })
        })
        .collect();
    group(
        "GEOL",
        &[
            ("LOCA_ID", "", "ID"),
            ("GEOL_TOP", "m", "2DP"),
            ("GEOL_BASE", "m", "2DP"),
            ("GEOL_DESC", "", "X"),
        ],
        rows,
    )
}

// Depths are compared exactly on purpose: contiguous strata share the one
// rounded value, so any difference is a gap or an overlap.
#[allow(clippy::float_cmp)]
#[cfg(test)]
mod tests {
    use super::*;
    use crate::pipeline::rust_check;
    use crate::synth::emit::emit_to_string;
    use laterite_ags4_parity::RustResult;

    /// Three units: one often absent (its thickness range dips below zero),
    /// one always present, and a last unit that runs to final depth.
    const FIXTURE: &str = r#"
schema = 1
name = "test-basin"
version = 3
summary = "A fixture."

[site]
extent_m = { x = 400.0, y = 300.0 }
ground_level_m = { min = 8.0, max = 22.0, basis = "test fixture" }

[[hole_types]]
code = "CP"
weight = 3
final_depth_m = { min = 20.0, max = 40.0, basis = "test fixture" }

[[hole_types]]
code = "RC"
weight = 1
final_depth_m = { min = 30.0, max = 50.0, basis = "test fixture" }

[[units]]
id = "RTD"
name = "River Terrace Deposits"
thickness_m = { min = -3.0, max = 6.0, basis = "test fixture" }
description = { principals = ["SAND", "GRAVEL"], secondaries = ["gravel", "sand", "clay"], colours = ["orangeish brown", "brown"] }
sampling = { types = ["B", "D"], interval_m = { min = 1.0, max = 1.5, basis = "test fixture" } }

[[units]]
id = "WLC"
name = "Weathered London Clay"
thickness_m = { min = 2.0, max = 8.0, basis = "test fixture" }
description = { principals = ["CLAY"], secondaries = ["silt"], colours = ["brown", "orangeish brown"] }
sampling = { types = ["U", "D"], interval_m = { min = 1.0, max = 1.5, basis = "test fixture" } }

[[units]]
id = "LC"
name = "London Clay"
description = { principals = ["CLAY"], secondaries = ["silt"], colours = ["grey", "dark grey"] }
sampling = { types = ["U", "D"], interval_m = { min = 1.5, max = 3.0, basis = "test fixture" } }
"#;

    fn fixture() -> Preset {
        Preset::from_toml(FIXTURE).expect("fixture loads")
    }

    fn file(preset: &Preset, seed: u64, holes: usize, grid: Grid) -> ProjectModel {
        lower(&build(preset, seed, holes), preset, seed, grid)
    }

    fn rows<'a>(m: &'a ProjectModel, code: &str) -> &'a [Row] {
        &m.groups.iter().find(|g| g.code == code).expect(code).rows
    }

    #[test]
    fn the_fixture_has_no_problems() {
        assert_eq!(problems(&fixture()), Vec::<String>::new());
    }

    /// A description the vocabulary cannot meet is reported by unit, so
    /// `--site` can refuse the preset before generating.
    #[test]
    fn a_unit_description_the_vocabulary_cannot_meet_is_a_problem() {
        let p = Preset::from_toml(&FIXTURE.replace("\"dark grey\"", "\"mauve\"")).unwrap();
        let found = problems(&p);
        assert_eq!(found.len(), 1, "{found:?}");
        assert!(
            found[0].starts_with("units[2] (LC).description: "),
            "{found:?}"
        );
        assert!(found[0].contains("mauve"), "{found:?}");
    }

    /// Clean by construction: a generator bug fails here, not as a finding.
    #[test]
    fn a_site_file_is_rust_clean_across_seeds() {
        let p = fixture();
        for seed in 0..15 {
            for grid in [
                Grid::Local,
                Grid::Bng {
                    easting: 500_000.0,
                    northing: 200_000.0,
                },
            ] {
                let dir = tempfile::tempdir().unwrap();
                let path = dir.path().join("site.ags");
                std::fs::write(&path, emit_to_string(&file(&p, seed, 6, grid))).unwrap();
                let (r, _) = rust_check(&path);
                assert_eq!(r, RustResult::Clean, "seed {seed} {grid:?}");
            }
        }
    }

    /// Strata run from 0 to final depth with no gap, overlap or empty row,
    /// and each unit appears at most once per hole, in preset order.
    #[test]
    fn strata_are_contiguous_from_the_surface_to_final_depth() {
        let p = fixture();
        let order: Vec<&str> = p.units.iter().map(|u| u.id.as_str()).collect();
        for seed in 0..30 {
            for h in build(&p, seed, 8).holes {
                assert_eq!(h.strata[0].top, 0.0, "{h:?}");
                assert_eq!(h.strata.last().unwrap().base, h.final_depth, "{h:?}");
                for pair in h.strata.windows(2) {
                    assert_eq!(pair[0].base, pair[1].top, "{h:?}");
                }
                assert!(h.strata.iter().all(|s| s.base > s.top), "{h:?}");
                let units: Vec<&str> = h.strata.iter().map(|s| s.unit.as_str()).collect();
                let mut expected = order.clone();
                expected.retain(|u| units.contains(u));
                assert_eq!(units, expected, "{h:?}");
            }
        }
    }

    /// A unit thicker than the hole is deep ends at final depth, and no unit
    /// below it appears.
    #[test]
    fn a_unit_that_reaches_final_depth_ends_the_hole() {
        let deep = Preset::from_toml(&FIXTURE.replace(
            "thickness_m = { min = 2.0, max = 8.0",
            "thickness_m = { min = 60.0, max = 70.0",
        ))
        .unwrap();
        for seed in 0..10 {
            for h in build(&deep, seed, 6).holes {
                let last = h.strata.last().unwrap();
                assert_eq!(last.unit, "WLC", "{h:?}");
                assert_eq!(last.base, h.final_depth, "{h:?}");
            }
        }
    }

    /// RTD's thickness range dips below zero, so across the site it is
    /// sometimes absent; WLC's never does, so it is always there.
    #[test]
    fn a_unit_is_absent_where_its_thickness_is_not_positive() {
        let p = fixture();
        let (mut with_rtd, mut without_rtd) = (0, 0);
        for seed in 0..40 {
            for h in build(&p, seed, 8).holes {
                let has = |id: &str| h.strata.iter().any(|s| s.unit == id);
                assert!(has("WLC"), "{h:?}");
                if has("RTD") {
                    with_rtd += 1;
                } else {
                    without_rtd += 1;
                }
            }
        }
        assert!(
            with_rtd > 0 && without_rtd > 0,
            "{with_rtd} / {without_rtd}"
        );
    }

    /// Hole k is the same whatever the hole count: the first 10 holes of a
    /// 100-hole site are a 10-hole site, in LOCA and in GEOL.
    #[test]
    fn a_smaller_site_is_a_prefix_of_a_larger_one() {
        let p = fixture();
        for seed in [0, 7] {
            let small = file(&p, seed, 10, Grid::Local);
            let large = file(&p, seed, 100, Grid::Local);
            assert_eq!(rows(&small, "LOCA"), &rows(&large, "LOCA")[..10]);
            let n = rows(&small, "GEOL").len();
            assert_eq!(rows(&small, "GEOL"), &rows(&large, "GEOL")[..n]);
            assert_eq!(rows(&small, "PROJ"), rows(&large, "PROJ"));
        }
    }

    /// Changing one unit's description moves only that unit's descriptions;
    /// every other byte of every hole stays put.
    #[test]
    fn changing_a_units_description_moves_only_its_rows() {
        let before = fixture();
        let after = Preset::from_toml(&FIXTURE.replace(
            r#"colours = ["brown", "orangeish brown"]"#,
            r#"colours = ["light brown"]"#,
        ))
        .unwrap();
        for seed in 0..10 {
            let (a, b) = (build(&before, seed, 8), build(&after, seed, 8));
            for (ha, hb) in a.holes.iter().zip(&b.holes) {
                assert_eq!(
                    (ha.x, ha.y, ha.final_depth, ha.ground_level),
                    (hb.x, hb.y, hb.final_depth, hb.ground_level)
                );
                for (sa, sb) in ha.strata.iter().zip(&hb.strata) {
                    assert_eq!((sa.top, sa.base, &sa.unit), (sb.top, sb.base, &sb.unit));
                    if sa.unit == "WLC" {
                        // With no leading term the colour opens the sentence.
                        assert!(sb.description.starts_with("Light brown "), "{sb:?}");
                    } else {
                        assert_eq!(sa.description, sb.description);
                    }
                }
            }
        }
    }

    /// Changing one unit's thickness moves its own base and the depths below
    /// it, but no hole's position or depth, and no other unit's description
    /// or presence.
    #[test]
    fn changing_a_units_thickness_moves_only_the_depths_it_governs() {
        let before = fixture();
        let after = Preset::from_toml(&FIXTURE.replace(
            "thickness_m = { min = 2.0, max = 8.0",
            "thickness_m = { min = 4.0, max = 9.0",
        ))
        .unwrap();
        let mut moved = false;
        for seed in 0..10 {
            let (a, b) = (
                file(&before, seed, 8, Grid::Local),
                file(&after, seed, 8, Grid::Local),
            );
            assert_eq!(rows(&a, "LOCA"), rows(&b, "LOCA"));
            let (ga, gb) = (rows(&a, "GEOL"), rows(&b, "GEOL"));
            for (ra, rb) in ga.iter().zip(gb) {
                // Same hole, same unit, same description; only depths move.
                assert_eq!(ra.values[0], rb.values[0]);
                assert_eq!(ra.values[3], rb.values[3]);
                moved |= ra.values[2] != rb.values[2];
            }
            assert_eq!(ga.len(), gb.len());
        }
        assert!(moved, "the thickness change moved nothing");
    }

    /// Neighbouring holes agree: a unit's thickness differs between two holes
    /// by no more than its surface's slope bound times their distance.
    #[test]
    fn neighbouring_holes_have_similar_unit_thickness() {
        let p = fixture();
        let wlc = &p.units[1];
        let range = wlc.thickness_m.as_ref().unwrap();
        for seed in 0..10 {
            let surface = Field::new(
                &mut Key::unit("thickness", &wlc.id).rng(seed),
                p.site.extent_m,
            );
            let site = build(&p, seed, 20);
            let thickness = |h: &Hole| {
                let s = h.strata.iter().find(|s| s.unit == "WLC").unwrap();
                s.base - s.top
            };
            for pair in site.holes.windows(2) {
                let (a, b) = (&pair[0], &pair[1]);
                // Only where neither hole's WLC was cut short by final depth.
                if a.strata.last().unwrap().unit != "LC" || b.strata.last().unwrap().unit != "LC" {
                    continue;
                }
                let d = (a.x - b.x).hypot(a.y - b.y);
                let bound = (range.max - range.min) * surface.max_slope() * d + 0.02;
                let diff = (thickness(a) - thickness(b)).abs();
                assert!(
                    diff <= bound,
                    "seed {seed}: {diff} over {d} m (bound {bound})"
                );
            }
        }
    }

    /// The marker names forge, the preset and its version, in `PROJ_MEMO`.
    #[test]
    fn every_site_file_carries_the_synthetic_marker() {
        let m = file(&fixture(), 1, 3, Grid::Local);
        let proj = m.groups.iter().find(|g| g.code == "PROJ").unwrap();
        let memo = &proj.rows[0].values[proj.col("PROJ_MEMO").unwrap()];
        assert!(memo.starts_with("SYNTHETIC"), "{memo}");
        assert!(memo.contains("laterite-ags4-forge"), "{memo}");
        assert!(memo.contains("test-basin v3"), "{memo}");
    }

    /// National-grid columns appear only with `Grid::Bng`, offset from the
    /// origin the consumer chose.
    #[test]
    fn national_grid_columns_follow_the_origin() {
        let preset = fixture();
        let local = file(&preset, 2, 4, Grid::Local);
        let loca = local.groups.iter().find(|g| g.code == "LOCA").unwrap();
        assert!(loca.col("LOCA_NATE").is_none());
        let bng = file(
            &preset,
            2,
            4,
            Grid::Bng {
                easting: 500_000.0,
                northing: 200_000.0,
            },
        );
        let loca = bng.groups.iter().find(|g| g.code == "LOCA").unwrap();
        let col = |h: &str| loca.col(h).unwrap();
        for row in &loca.rows {
            let num = |h: &str| row.values[col(h)].parse::<f64>().unwrap();
            let east = num("LOCA_NATE") - 500_000.0 - num("LOCA_LOCX");
            let north = num("LOCA_NATN") - 200_000.0 - num("LOCA_LOCY");
            assert!(east.abs() < 0.006 && north.abs() < 0.006, "{row:?}");
            assert_eq!(row.values[col("LOCA_GREF")], "OSGB");
        }
    }
}
