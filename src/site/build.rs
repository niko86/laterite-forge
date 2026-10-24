//! The typed site layer (site, holes, strata) and its lowering into the
//! `ProjectModel` the emitter writes.
//!
//! Nothing here touches the scaffold builder, so the scaffold pins are out of
//! reach (ADR 0001). Every draw comes from a keyed stream (`draw::Key`), so a
//! hole depends only on its own index, and a stratum only on its hole and
//! unit.

use laterite_ags4_parity::Rng;
use laterite_ags4_validator::{DictVersion, Dictionary};

use super::construction;
use super::cover::{CoverLayer, SiteCover};
use super::draw::{Key, uniform};
use super::ground::Field;
use super::preset::{DescriptionSpec, Extent, Lens, MadeGroundSpec, Preset, Sampling};
use super::properties::{LeadTerms, PropertyModel};
use super::suite;
use crate::synth::bs5930::{self, Lead, UnitDescription};
use crate::synth::generic;
use crate::synth::made_ground::{self, Mix};
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
    /// The index properties, density and groundwater behind the holes. The
    /// descriptions' leading terms come from it; no value is written to the
    /// file yet (#1).
    pub(super) properties: PropertyModel,
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
    /// Top-down, contiguous from 0 to `final_depth`. With a cover the first
    /// stratum keeps its model top of 0 (its property trends and description
    /// are those it would have without the cover) but is written from the
    /// cover base: see [`Hole::written_top`].
    pub strata: Vec<Stratum>,
    /// The topsoil or surfacing on the first stratum, if the hole has one.
    pub cover: Option<CoverLayer>,
}

impl Hole {
    /// Where stratum `i` starts in the file: its top, except that a cover
    /// pushes the first stratum's top down to the cover base.
    pub fn written_top(&self, i: usize) -> f64 {
        match (&self.cover, i) {
            (Some(c), 0) => c.base,
            _ => self.strata[i].top,
        }
    }
}

/// The part of one unit met in one hole.
#[derive(Debug, Clone, PartialEq)]
pub struct Stratum {
    pub unit: String,
    pub top: f64,
    pub base: f64,
    pub description: String,
    /// Top-down, each wholly inside `top..=base`.
    pub samples: Vec<Sample>,
}

/// One sample taken from a stratum.
#[derive(Debug, Clone, PartialEq)]
pub struct Sample {
    pub top: f64,
    pub base: f64,
    /// Its `SAMP_TYPE` code.
    pub code: String,
    /// Its place in the hole's sampling schedule, counting from 0 top-down
    /// before any cover took samples off the top of the first stratum. It
    /// numbers the sample (`SAMP_REF` is `rank + 1`) and keys its draws, so a
    /// cover that removes samples leaves every other sample's number, test
    /// and result where they were; the removed ranks are simply unused.
    pub rank: usize,
}

/// The length of every sample, metres: 0.45 m is the usual UK driven-sample
/// length (a U100 tube barrel). `SAMP_BASE` is required, and a fixed length
/// keeps the sampling schedule to the one thing the preset specifies, the
/// spacing of tops.
pub const SAMPLE_LENGTH_M: f64 = 0.45;

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
    let units = preset.units.iter().enumerate().flat_map(|(i, u)| {
        unit_description(&u.description)
            .problems(vocab)
            .into_iter()
            .map(move |p| format!("units[{i}] ({}).description: {p}", u.id))
    });
    let lenses = preset.lenses.iter().enumerate().flat_map(|(i, l)| {
        unit_description(&l.description)
            .problems(vocab)
            .into_iter()
            .map(move |p| format!("lenses[{i}] ({}).description: {p}", l.id))
    });
    let topsoil = preset
        .cover
        .as_ref()
        .and_then(|c| c.topsoil.as_ref())
        .into_iter()
        .flat_map(|t| {
            t.description
                .problems(vocab)
                .into_iter()
                .map(|p| format!("cover.topsoil.description: {p}"))
        });
    units.chain(lenses).chain(topsoil).collect()
}

fn unit_description(d: &DescriptionSpec) -> UnitDescription {
    UnitDescription {
        principals: d.principals.clone(),
        secondaries: d.secondaries.clone(),
        colours: d.colours.clone(),
        very_coarse: d.very_coarse,
    }
}

fn lead(terms: &LeadTerms) -> Lead<'static> {
    Lead {
        fine: terms.fine,
        coarse: terms.coarse,
    }
}

/// The weighted constituents of a made-ground unit on this site, or `None`
/// when the unit gives no weights (`made_ground = true`) and so takes the
/// lane's default draw. Each share is drawn once per site in its range, from
/// a stream keyed by the unit and the constituent's position.
fn unit_mix(spec: &MadeGroundSpec, unit: &str, seed: u64) -> Option<Mix> {
    if spec.constituents.is_none() && spec.count.is_none() && spec.amounts.is_none() {
        return None;
    }
    let shares: Vec<(&str, f64)> = match &spec.constituents {
        Some(list) => list
            .iter()
            .enumerate()
            .map(|(i, c)| {
                let key = Key {
                    test: i as u64,
                    ..Key::unit("made-ground-share", unit)
                };
                (
                    c.name.as_str(),
                    uniform(&mut key.rng(seed), c.share.min, c.share.max),
                )
            })
            .collect(),
        // Equal shares: the fit then gives every default word the same weight.
        None => made_ground::constituents()
            .constituents
            .iter()
            .map(|n| (n.as_str(), 0.5))
            .collect(),
    };
    Some(Mix::new(
        &shares,
        spec.count.as_ref().map(|c| c.weights),
        spec.amounts.as_ref().map(|a| a.weights),
    ))
}

/// The thinnest stratum written: one unit in the last decimal place.
const MIN_THICKNESS: f64 = 0.01;

/// Where a lens sits on this site, drawn once per site from the lens's own
/// keyed stream, so adding a lens moves no other draw.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Footprint {
    x: f64,
    y: f64,
    radius: f64,
    t_max: f64,
    /// Mid-depth as a fraction of the host's thickness at a hole.
    depth: f64,
}

impl Footprint {
    fn new(lens: &Lens, extent: Extent, seed: u64) -> Self {
        let mut rng = Key::unit("lens", &lens.id).rng(seed);
        Footprint {
            x: uniform(&mut rng, 0.0, extent.x),
            y: uniform(&mut rng, 0.0, extent.y),
            radius: uniform(&mut rng, lens.radius_m.min, lens.radius_m.max),
            t_max: uniform(&mut rng, lens.thickness_m.min, lens.thickness_m.max),
            depth: uniform(&mut rng, lens.depth_in_host.min, lens.depth_in_host.max),
        }
    }

    /// The lens's thickness at (`x`, `y`): the lenticular taper
    /// `t_max * (1 - (r/R)^2)` inside the radius, rounded to the written
    /// precision. Zero where there is no lens, including where the taper is
    /// thinner than that precision.
    fn thickness_at(&self, x: f64, y: f64) -> f64 {
        let r = (x - self.x).hypot(y - self.y);
        if r >= self.radius {
            return 0.0;
        }
        let t = round2(self.t_max * (1.0 - (r / self.radius).powi(2)));
        if t < MIN_THICKNESS { 0.0 } else { t }
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
    let lenses = preset
        .lenses
        .iter()
        .map(|l| (l, Footprint::new(l, extent, seed)))
        .collect();
    let mixes = preset
        .units
        .iter()
        .map(|u| {
            u.made_ground
                .as_ref()
                .and_then(|m| unit_mix(m, &u.id, seed))
        })
        .collect();
    let layers = Layers {
        surfaces,
        lenses,
        mixes,
    };
    let properties = PropertyModel::new(preset, seed);
    let site_cover = preset.cover.as_ref().map(|c| SiteCover::new(c, seed));
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
            let mut strata = strata(preset, &layers, &properties, seed, (k, x, y), final_depth);
            let cover = site_cover
                .as_ref()
                .and_then(|c| c.apply(k, seed, &mut strata));
            Hole {
                id: format!("BH{}", k + 1),
                loca_type: hole_type.code.clone(),
                x,
                y,
                ground_level: round2(ground.in_range(&preset.site.ground_level_m, x, y)),
                final_depth,
                strata,
                cover,
            }
        })
        .collect();
    Site { holes, properties }
}

/// The site-wide fields and footprints the strata of every hole read.
struct Layers<'a> {
    /// One thickness surface per unit, in preset order.
    surfaces: Vec<Field>,
    lenses: Vec<(&'a Lens, Footprint)>,
    /// One per unit, in preset order: the weighted constituents of a made-ground
    /// unit that gives weights, else `None` (the lane's default draw).
    mixes: Vec<Option<Mix>>,
}

/// The strata of hole `k`: one per unit present, top-down, from 0 to
/// `final_depth`, with a unit's lenses splitting it where they reach the hole.
/// A unit whose thickness here is zero or less is absent; the last unit runs
/// to final depth; nothing continues below final depth.
///
/// Each description leads with the term the property model gives at the
/// unit's mid-depth in this hole (a lens's, at the lens's). A host split by
/// lenses keeps one description, taken over its whole span.
fn strata(
    preset: &Preset,
    layers: &Layers,
    properties: &PropertyModel,
    seed: u64,
    (k, x, y): (usize, f64, f64),
    final_depth: f64,
) -> Vec<Stratum> {
    let vocab = bs5930::vocab();
    let mut out = Vec::new();
    let mut top = 0.0;
    for ((unit, surface), mix) in preset.units.iter().zip(&layers.surfaces).zip(&layers.mixes) {
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
        let terms = properties.lead(k, (x, y), &unit.id, top, (top, base));
        let matrix = bs5930::describe_unit(
            vocab,
            &mut rng,
            &unit_description(&unit.description),
            lead(&terms),
        )
        .expect("problems() was checked before build");
        let description = if unit.made_ground.is_some() {
            // The lane's own stream, so the matrix draw above is the one a
            // plain unit would make.
            let mut rng = Key::stratum("made-ground", k, &unit.id).rng(seed);
            let made = match mix {
                Some(mix) => made_ground::made_ground_mix(&matrix, vocab, &mut rng, mix),
                None => made_ground::made_ground(&matrix, vocab, &mut rng),
            };
            made.description.text
        } else {
            matrix.text
        };
        let inside = lens_spans(&layers.lenses, &unit.id, x, y, top, base);
        if inside.is_empty() {
            // The common case, with the keys it always had.
            out.push(Stratum {
                unit: unit.id.clone(),
                top,
                base,
                description,
                samples: samples(
                    &unit.sampling,
                    Key::stratum("sampling", k, &unit.id).rng(seed),
                    top,
                    base,
                ),
            });
        } else {
            // The host splits around its lenses. Host parts keep the host's
            // description (it is the same unit) but sample on their own
            // streams, keyed by part, so two parts do not copy each other's
            // offsets.
            let mut cursor = top;
            let mut part = 0;
            let mut host_part = |out: &mut Vec<Stratum>, from: f64, to: f64| {
                // Spans lie on the 2 DP grid, so a gap is 0 or at least 0.01.
                if to - from < MIN_THICKNESS / 2.0 {
                    return;
                }
                let key = Key {
                    test: part,
                    ..Key::stratum("sampling-part", k, &unit.id)
                };
                part += 1;
                out.push(Stratum {
                    unit: unit.id.clone(),
                    top: from,
                    base: to,
                    description: description.clone(),
                    samples: samples(&unit.sampling, key.rng(seed), from, to),
                });
            };
            for span in &inside {
                host_part(&mut out, cursor, span.top);
                let lens = span.lens;
                let mut rng = Key::stratum("description", k, &lens.id).rng(seed);
                let terms = properties.lead(k, (x, y), &lens.id, span.top, (span.top, span.base));
                let text = bs5930::describe_unit(
                    vocab,
                    &mut rng,
                    &unit_description(&lens.description),
                    lead(&terms),
                )
                .expect("problems() was checked before build")
                .text;
                out.push(Stratum {
                    unit: lens.id.clone(),
                    top: span.top,
                    base: span.base,
                    description: text,
                    samples: samples(
                        &lens.sampling,
                        Key::stratum("sampling", k, &lens.id).rng(seed),
                        span.top,
                        span.base,
                    ),
                });
                cursor = span.base;
            }
            host_part(&mut out, cursor, base);
        }
        top = base;
    }
    // Number the schedule top-down, before any cover removes samples.
    for (rank, sample) in out
        .iter_mut()
        .flat_map(|s| s.samples.iter_mut())
        .enumerate()
    {
        sample.rank = rank;
    }
    out
}

/// A lens's extent in one hole, on the 2 DP grid and inside its host.
struct Span<'a> {
    lens: &'a Lens,
    top: f64,
    base: f64,
}

/// The lenses of host `host` that this hole meets, top-down. A lens sits at
/// its depth fraction of the host stratum, clipped to lie inside it. A lens
/// whose span would overlap one already placed (lenses are taken in preset
/// order) is absent here.
fn lens_spans<'a>(
    lenses: &[(&'a Lens, Footprint)],
    host: &str,
    x: f64,
    y: f64,
    top: f64,
    base: f64,
) -> Vec<Span<'a>> {
    let mut spans: Vec<Span<'a>> = Vec::new();
    for (lens, f) in lenses.iter().filter(|(l, _)| l.host == host) {
        let thickness = f.thickness_at(x, y).min(round2(base - top));
        if thickness < MIN_THICKNESS {
            continue;
        }
        let mid = top + f.depth * (base - top);
        let lens_top = round2((mid - thickness / 2.0).max(top).min(base - thickness)).max(top);
        let lens_base = round2(lens_top + thickness).min(base);
        if spans.iter().any(|s| {
            lens_top < s.base - MIN_THICKNESS / 2.0 && lens_base > s.top + MIN_THICKNESS / 2.0
        }) {
            continue;
        }
        spans.push(Span {
            lens,
            top: lens_top,
            base: lens_base,
        });
    }
    spans.sort_by(|a, b| a.top.total_cmp(&b.top));
    spans
}

/// The samples of one stratum: the first top falls within one interval of the
/// stratum top, and each next top is a drawn interval below the last (never
/// overlapping the sample above). A sample whose base would pass the stratum
/// base is dropped, and so is everything below it. Depths are rounded to the 2
/// decimal places they are written with, and `top` and `base` are already
/// such values, so a rounded sample can never leave its stratum.
fn samples(sampling: &Sampling, mut rng: Rng, top: f64, base: f64) -> Vec<Sample> {
    let range = &sampling.interval_m;
    let mut out = Vec::new();
    let mut at = round2(top + uniform(&mut rng, 0.0, range.max));
    loop {
        let sample_base = round2(at + SAMPLE_LENGTH_M);
        if at < top || sample_base > base {
            break;
        }
        let pick = usize::try_from(rng.below(sampling.types.len() as u64)).expect("small index");
        out.push(Sample {
            top: at,
            base: sample_base,
            code: sampling.types[pick].clone(),
            rank: 0,
        });
        let step = uniform(&mut rng, range.min, range.max);
        at = round2(at + step).max(sample_base);
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
/// TRAN, LOCA, GEOL, SAMP, the preset's suite (if any), then ABBR/UNIT/TYPE covering exactly what those use.
pub fn lower(site: &Site, preset: &Preset, seed: u64, grid: Grid) -> ProjectModel {
    let dict = Dictionary::bundled(DictVersion::V4_2);
    let plan = construction::plan(site, preset, seed);
    let mut groups = vec![
        proj(preset, seed),
        tran(seed),
        loca(site, grid, plan.as_ref()),
    ];
    groups.push(geol(site));
    groups.push(samp(site));
    groups.extend(suite::groups(
        site,
        preset,
        seed,
        suite::PRODUCERS,
        plan.as_ref(),
    ));
    if let Some(plan) = &plan {
        groups.extend(construction::groups(site, plan));
    }
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
fn loca(site: &Site, grid: Grid, construction: Option<&construction::Plan>) -> Group {
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
        .enumerate()
        .map(|(i, h)| {
            // With construction the type is the phases, `IP+CP`.
            let loca_type =
                construction.map_or_else(|| h.loca_type.clone(), |p| p.holes[i].loca_type());
            let mut v = vec![h.id.clone(), loca_type];
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
            let cover = h.cover.iter().map(|c| {
                Row::owned(vec![
                    h.id.clone(),
                    m2(0.0),
                    m2(c.base),
                    c.description.clone(),
                ])
            });
            let strata = h.strata.iter().enumerate().map(|(i, s)| {
                Row::owned(vec![
                    h.id.clone(),
                    m2(h.written_top(i)),
                    m2(s.base),
                    s.description.clone(),
                ])
            });
            cover.chain(strata)
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

/// A hole's sample with its `SAMP_REF`.
pub(super) struct Numbered<'a> {
    /// The sample's 1-based schedule rank (`rank + 1`): a count of the hole's
    /// samples top-down, with gaps where a cover removed some.
    pub reference: String,
    pub stratum: &'a Stratum,
    pub sample: &'a Sample,
}

/// The hole's samples top-down, numbered as `SAMP` writes them, so the test
/// groups that hang off SAMP carry the same KEY.
pub(super) fn numbered(hole: &Hole) -> impl Iterator<Item = Numbered<'_>> {
    hole.strata
        .iter()
        .flat_map(|st| st.samples.iter().map(move |sample| (st, sample)))
        .map(|(stratum, sample)| Numbered {
            reference: (sample.rank + 1).to_string(),
            stratum,
            sample,
        })
}

/// SAMP, in dictionary heading order: the five-part KEY, then `SAMP_BASE`.
/// `SAMP_REF` counts the hole's samples top-down, which with the top makes
/// the KEY unique; `SAMP_ID` repeats it with the hole so it reads on its own.
fn samp(site: &Site) -> Group {
    let rows = site
        .holes
        .iter()
        .flat_map(|h| {
            numbered(h).map(|n| {
                Row::owned(vec![
                    h.id.clone(),
                    m2(n.sample.top),
                    n.reference.clone(),
                    n.sample.code.clone(),
                    format!("{}-{}", h.id, n.reference),
                    m2(n.sample.base),
                ])
            })
        })
        .collect();
    group(
        "SAMP",
        &[
            ("LOCA_ID", "", "ID"),
            ("SAMP_TOP", "m", "2DP"),
            ("SAMP_REF", "", "X"),
            ("SAMP_TYPE", "", "PA"),
            ("SAMP_ID", "", "ID"),
            ("SAMP_BASE", "m", "2DP"),
        ],
        rows,
    )
}

// Depths are compared exactly on purpose: contiguous strata share the one
// rounded value, so any difference is a gap or an overlap.
#[allow(clippy::float_cmp)]
#[cfg(test)]
pub(super) mod tests {
    use super::*;
    use crate::pipeline::rust_check;
    use crate::synth::emit::emit_to_string;
    use laterite_ags4_parity::RustResult;

    /// Three units: one often absent (its thickness range dips below zero),
    /// one always present, and a last unit that runs to final depth.
    pub(in crate::site) const FIXTURE: &str = r#"
schema = 1
name = "test-basin"
version = 3
summary = "A fixture."

[site]
extent_m = { x = 400.0, y = 300.0 }
ground_level_m = { min = 8.0, max = 22.0, basis = "test fixture" }
groundwater_depth_m = { min = 1.0, max = 4.0, basis = "test fixture" }

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

[units.properties]
spt_n = { min = 10, max = 30, basis = "test fixture" }
moisture_content_percent = { min = 8, max = 15, basis = "test fixture" }
particle_density_ratio = { min = 2.6, max = 2.7, basis = "test fixture" }

[[units]]
id = "WLC"
name = "Weathered London Clay"
thickness_m = { min = 2.0, max = 8.0, basis = "test fixture" }
description = { principals = ["CLAY"], secondaries = ["silt"], colours = ["brown", "orangeish brown"] }
sampling = { types = ["U", "D"], interval_m = { min = 1.0, max = 1.5, basis = "test fixture" } }

[units.properties]
plastic_limit_percent = { min = 20, max = 30, basis = "test fixture" }
plasticity_index_percent = { min = 30, max = 50, basis = "test fixture" }
liquidity_index = { min = 0.0, max = 0.3, basis = "test fixture" }
particle_density_ratio = { min = 2.6, max = 2.7, basis = "test fixture" }

[[units]]
id = "LC"
name = "London Clay"
description = { principals = ["CLAY"], secondaries = ["silt"], colours = ["grey", "dark grey"] }
sampling = { types = ["U", "D"], interval_m = { min = 1.5, max = 3.0, basis = "test fixture" } }

[units.properties]
plastic_limit_percent = { min = 20, max = 30, basis = "test fixture" }
plasticity_index_percent = { min = 30, max = 50, basis = "test fixture" }
liquidity_index = { min = 0.0, max = 0.3, basis = "test fixture" }
particle_density_ratio = { min = 2.6, max = 2.7, basis = "test fixture" }
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

    /// Two lenses on the fixture: a sand band in the (thin) Weathered London
    /// Clay and a sand body in the (deep) London Clay. `{R}` is the radius range.
    const LENSES: &str = r#"
[[lenses]]
id = "SB"
name = "Sand band"
host = "WLC"
radius_m = { {R}, basis = "test fixture" }
thickness_m = { min = 0.3, max = 1.2, basis = "test fixture" }
depth_in_host = { min = 0.2, max = 0.8, basis = "test fixture" }
description = { principals = ["SAND"], secondaries = ["silt"], colours = ["brown"] }
sampling = { types = ["D", "B"], interval_m = { min = 0.3, max = 0.6, basis = "test fixture" } }

[lenses.properties]
spt_n = { min = 10, max = 30, basis = "test fixture" }
moisture_content_percent = { min = 8, max = 15, basis = "test fixture" }
particle_density_ratio = { min = 2.6, max = 2.7, basis = "test fixture" }

[[lenses]]
id = "CL"
name = "Clayey sand body"
host = "LC"
radius_m = { {R}, basis = "test fixture" }
thickness_m = { min = 1.0, max = 3.0, basis = "test fixture" }
depth_in_host = { min = 0.3, max = 0.7, basis = "test fixture" }
description = { principals = ["SAND"], secondaries = ["clay"], colours = ["grey"] }
sampling = { types = ["D"], interval_m = { min = 0.5, max = 1.0, basis = "test fixture" } }

[lenses.properties]
spt_n = { min = 10, max = 30, basis = "test fixture" }
moisture_content_percent = { min = 8, max = 15, basis = "test fixture" }
particle_density_ratio = { min = 2.6, max = 2.7, basis = "test fixture" }
"#;

    /// `FIXTURE` plus `LENSES` with the given plan radius range.
    fn lens_preset_with(radius: &str) -> Preset {
        let text = format!("{FIXTURE}{}", LENSES.replace("{R}", radius));
        Preset::from_toml(&text).expect("lens fixture loads")
    }

    /// Radii that reach most of the 400 x 300 m site.
    fn lens_preset() -> Preset {
        lens_preset_with("min = 100.0, max = 200.0")
    }

    /// Radii that leave most of the site untouched.
    fn small_lens_preset() -> Preset {
        lens_preset_with("min = 30.0, max = 60.0")
    }

    fn footprints(p: &Preset, seed: u64) -> Vec<(&Lens, Footprint)> {
        p.lenses
            .iter()
            .map(|l| (l, Footprint::new(l, p.site.extent_m, seed)))
            .collect()
    }

    fn reaches(f: &Footprint, h: &Hole) -> bool {
        (h.x - f.x).hypot(h.y - f.y) < f.radius
    }

    #[test]
    fn the_lens_fixture_has_no_problems() {
        assert_eq!(problems(&lens_preset()), Vec::<String>::new());
    }

    #[test]
    fn a_lens_description_the_vocabulary_cannot_meet_is_a_problem() {
        let text = format!("{FIXTURE}{}", LENSES.replace("{R}", "min = 1.0, max = 2.0")).replace(
            "colours = [\"grey\"] }\nsampling = { types = [\"D\"]",
            "colours = [\"mauve\"] }\nsampling = { types = [\"D\"]",
        );
        let found = problems(&Preset::from_toml(&text).unwrap());
        assert_eq!(found.len(), 1, "{found:?}");
        assert!(
            found[0].starts_with("lenses[1] (CL).description: "),
            "{found:?}"
        );
    }

    /// Where a lens reaches, its host splits into host-above / lens /
    /// host-below (or a clipped variant); elsewhere the host is one stratum.
    /// Either way the column is contiguous with unique GEOL keys, host parts
    /// share their description, and samples stay inside their stratum.
    #[test]
    fn a_hole_through_a_lens_records_host_lens_host() {
        let p = lens_preset();
        let (mut split, mut whole, mut lens_rows) = (0, 0, 0);
        for seed in 0..40 {
            let fps = footprints(&p, seed);
            for h in build(&p, seed, 10).holes {
                assert_eq!(h.strata[0].top, 0.0, "{h:?}");
                assert_eq!(h.strata.last().unwrap().base, h.final_depth, "{h:?}");
                for pair in h.strata.windows(2) {
                    assert_eq!(pair[0].base, pair[1].top, "{h:?}");
                }
                assert!(h.strata.iter().all(|s| s.base > s.top), "{h:?}");
                let mut keys: Vec<(String, String)> =
                    h.strata.iter().map(|s| (m2(s.top), m2(s.base))).collect();
                keys.sort_unstable();
                keys.dedup();
                assert_eq!(keys.len(), h.strata.len(), "{h:?}");
                for s in &h.strata {
                    for m in &s.samples {
                        assert!(m.top >= s.top && m.base <= s.base, "{h:?} {m:?}");
                    }
                }
                for (lens, f) in &fps {
                    let parts: Vec<usize> = (0..h.strata.len())
                        .filter(|&i| h.strata[i].unit == lens.host)
                        .collect();
                    let lens_at: Vec<usize> = (0..h.strata.len())
                        .filter(|&i| h.strata[i].unit == lens.id)
                        .collect();
                    if !reaches(f, &h) {
                        assert!(lens_at.is_empty(), "{h:?}");
                        assert!(parts.len() <= 1, "{h:?}");
                        whole += usize::from(parts.len() == 1);
                        continue;
                    }
                    if lens_at.is_empty() {
                        // Too thin at this distance, or the host is absent.
                        assert!(parts.len() <= 1, "{h:?}");
                        continue;
                    }
                    assert_eq!(lens_at.len(), 1, "{h:?}");
                    lens_rows += 1;
                    let i = lens_at[0];
                    // Host parts sit directly above and below the lens, in
                    // that order, and nowhere else.
                    for &j in &parts {
                        assert!(j + 1 == i || j == i + 1, "{h:?}");
                    }
                    if parts.len() == 2 {
                        split += 1;
                        assert_eq!(
                            h.strata[parts[0]].description,
                            h.strata[parts[1]].description
                        );
                    }
                }
            }
        }
        assert!(
            split > 20 && whole > 20 && lens_rows > 50,
            "{split} {whole} {lens_rows}"
        );
    }

    /// The lens thickness at a hole is the taper at that distance, to
    /// rounding, unless the host cut it short; it is never more than the taper.
    #[test]
    #[allow(clippy::many_single_char_names)]
    fn lens_thickness_follows_the_taper() {
        let p = lens_preset();
        let mut checked = 0;
        for seed in 0..40 {
            for (lens, f) in footprints(&p, seed) {
                for h in build(&p, seed, 12).holes {
                    let r = (h.x - f.x).hypot(h.y - f.y);
                    let expected = if r < f.radius {
                        f.t_max * (1.0 - (r / f.radius).powi(2))
                    } else {
                        0.0
                    };
                    let found = h.strata.iter().position(|s| s.unit == lens.id);
                    let Some(i) = found else {
                        // Absent: outside, thinner than the precision, or no host.
                        let host_here = h.strata.iter().any(|s| s.unit == lens.host);
                        assert!(expected < 0.0151 || !host_here, "{expected} {h:?}");
                        continue;
                    };
                    let s = &h.strata[i];
                    let t = s.base - s.top;
                    let interior = i > 0
                        && i + 1 < h.strata.len()
                        && h.strata[i - 1].unit == lens.host
                        && h.strata[i + 1].unit == lens.host;
                    if interior {
                        assert!((t - expected).abs() <= 0.0101, "{t} vs {expected}");
                        checked += 1;
                    } else {
                        assert!(t <= expected + 0.0101, "{t} vs {expected}");
                    }
                }
            }
        }
        assert!(checked > 100, "{checked}");
    }

    /// The lens is thickest at its centre: a hole within a few metres of the
    /// centre carries nearly the full `t_max`, and thickness never grows with
    /// distance among interior lens strata of the deep host.
    #[test]
    #[allow(clippy::many_single_char_names)]
    fn the_lens_is_thickest_near_its_centre() {
        let p = lens_preset();
        let mut near = 0;
        for seed in 0..120 {
            let (lens, f) = footprints(&p, seed)[1];
            assert_eq!(lens.id, "CL");
            let mut seen: Vec<(f64, f64)> = Vec::new();
            for h in build(&p, seed, 60).holes {
                let Some(s) = h.strata.iter().find(|s| s.unit == "CL") else {
                    continue;
                };
                let d = (h.x - f.x).hypot(h.y - f.y);
                let t = s.base - s.top;
                if d < 0.12 * f.radius {
                    near += 1;
                    assert!(t >= f.t_max * (1.0 - 0.0144) - 0.0101, "{t} vs {}", f.t_max);
                }
                seen.push((d, t));
            }
            for a in &seen {
                for b in &seen {
                    if a.0 < b.0 {
                        assert!(a.1 >= b.1 - 0.0201, "{a:?} {b:?}");
                    }
                }
            }
        }
        assert!(near > 0, "no hole fell near a lens centre");
    }

    /// Holes outside every lens are byte-identical, in LOCA, GEOL and SAMP,
    /// with and without the lenses.
    #[test]
    fn holes_outside_every_lens_are_unchanged() {
        let (plain, with) = (fixture(), small_lens_preset());
        let (mut same, mut changed) = (0, 0);
        for seed in 0..30 {
            let fps = footprints(&with, seed);
            let (a, b) = (build(&plain, seed, 12), build(&with, seed, 12));
            let (fa, fb) = (
                lower(&a, &plain, seed, Grid::Local),
                lower(&b, &with, seed, Grid::Local),
            );
            for (ha, hb) in a.holes.iter().zip(&b.holes) {
                let untouched = fps.iter().all(|(_, f)| !reaches(f, hb));
                if untouched {
                    same += 1;
                    assert_eq!(ha, hb);
                    for code in ["LOCA", "GEOL", "SAMP"] {
                        let pick = |m: &ProjectModel| -> Vec<Row> {
                            rows(m, code)
                                .iter()
                                .filter(|r| r.values[0] == ha.id)
                                .cloned()
                                .collect()
                        };
                        assert_eq!(pick(&fa), pick(&fb), "{code} {}", ha.id);
                    }
                } else {
                    changed += usize::from(ha != hb);
                }
            }
        }
        assert!(same > 100 && changed > 0, "{same} {changed}");
    }

    /// The two parts of a split host take their samples from their own
    /// streams: their offsets from the part top are not copies of each other.
    #[test]
    fn split_host_parts_do_not_share_sample_offsets() {
        let p = lens_preset();
        let (mut pairs, mut equal) = (0, 0);
        for seed in 0..40 {
            for h in build(&p, seed, 10).holes {
                let wlc: Vec<&Stratum> = h.strata.iter().filter(|s| s.unit == "WLC").collect();
                if let [above, below] = wlc[..]
                    && let (Some(a), Some(b)) = (above.samples.first(), below.samples.first())
                {
                    pairs += 1;
                    equal += usize::from((a.top - above.top - (b.top - below.top)).abs() < 1e-9);
                }
            }
        }
        assert!(pairs > 20, "{pairs}");
        assert!(equal * 3 < pairs, "{equal} of {pairs} identical");
    }

    /// A lens thinner than the written precision is absent, leaving the host
    /// whole and the hole exactly as without the lens.
    #[test]
    fn a_lens_thinner_than_the_precision_is_absent() {
        let thin = format!(
            "{FIXTURE}{}",
            LENSES.replace("{R}", "min = 100.0, max = 200.0")
        )
        .replace("min = 0.3, max = 1.2", "min = 0.001, max = 0.004")
        .replace("min = 1.0, max = 3.0", "min = 0.001, max = 0.004");
        let thin = Preset::from_toml(&thin).unwrap();
        let plain = fixture();
        for seed in 0..10 {
            assert_eq!(build(&thin, seed, 8).holes, build(&plain, seed, 8).holes);
        }
    }

    /// A lens as thick as its host leaves no host part, and one at the host's
    /// top leaves no part above it; neither writes a zero-thickness row.
    #[test]
    fn host_parts_thinner_than_the_precision_are_dropped() {
        let filled = format!(
            "{FIXTURE}{}",
            LENSES.replace("{R}", "min = 100.0, max = 200.0")
        )
        .replace("min = 0.3, max = 1.2", "min = 50.0, max = 60.0");
        let filled = Preset::from_toml(&filled).unwrap();
        let top = Preset::from_toml(
            &format!(
                "{FIXTURE}{}",
                LENSES.replace("{R}", "min = 100.0, max = 200.0")
            )
            .replace("min = 0.2, max = 0.8", "min = 0.0, max = 0.0"),
        )
        .unwrap();
        let (mut covered, mut at_top) = (0, 0);
        for seed in 0..30 {
            for h in build(&filled, seed, 8).holes {
                assert!(h.strata.iter().all(|s| s.base > s.top), "{h:?}");
                for pair in h.strata.windows(2) {
                    assert_eq!(pair[0].base, pair[1].top, "{h:?}");
                }
                if let Some(i) = h.strata.iter().position(|s| s.unit == "SB") {
                    // Fills the host from top to base: nothing of WLC is left.
                    if h.strata.iter().all(|s| s.unit != "WLC") {
                        covered += 1;
                        assert!(i > 0 || h.strata[0].top == 0.0);
                    }
                }
            }
            for h in build(&top, seed, 8).holes {
                assert!(h.strata.iter().all(|s| s.base > s.top), "{h:?}");
                if let Some(i) = h.strata.iter().position(|s| s.unit == "SB") {
                    // Nothing of WLC sits above the lens.
                    if i == 0 || h.strata[i - 1].unit != "WLC" {
                        at_top += 1;
                    }
                }
            }
        }
        assert!(covered > 5 && at_top > 5, "{covered} {at_top}");
    }

    /// Two lenses in one host never overlap, whatever their placement.
    #[test]
    fn two_lenses_in_one_host_stay_contiguous_and_clean() {
        let text = format!(
            "{FIXTURE}{}",
            LENSES.replace("{R}", "min = 150.0, max = 250.0")
        )
        .replace("host = \"LC\"", "host = \"WLC\"");
        let p = Preset::from_toml(&text).unwrap();
        for seed in 0..40 {
            for h in build(&p, seed, 10).holes {
                assert_eq!(h.strata[0].top, 0.0, "{h:?}");
                assert_eq!(h.strata.last().unwrap().base, h.final_depth, "{h:?}");
                for pair in h.strata.windows(2) {
                    assert_eq!(pair[0].base, pair[1].top, "{h:?}");
                }
                assert!(h.strata.iter().all(|s| s.base > s.top), "{h:?}");
            }
        }
    }

    #[test]
    fn a_lens_site_is_rust_clean_across_seeds() {
        let p = lens_preset();
        for seed in 0..15 {
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("site.ags");
            std::fs::write(&path, emit_to_string(&file(&p, seed, 6, Grid::Local))).unwrap();
            let (r, _) = rust_check(&path);
            assert_eq!(r, RustResult::Clean, "seed {seed}");
        }
    }

    /// Hole k is the same whatever the hole count, with lenses too.
    #[test]
    fn a_lens_site_is_prefix_stable() {
        let p = lens_preset();
        for seed in [0, 7] {
            let small = file(&p, seed, 10, Grid::Local);
            let large = file(&p, seed, 100, Grid::Local);
            for code in ["LOCA", "GEOL", "SAMP"] {
                let n = rows(&small, code).len();
                assert_eq!(rows(&small, code), &rows(&large, code)[..n], "{code}");
            }
        }
    }

    /// A preset without lenses gives the same bytes whatever the lens code
    /// does. The hashes were first taken from the pre-lens code and re-taken
    /// when the descriptions gained their property-model leading term (#41),
    /// the only change since; the fixture carries properties for it.
    #[test]
    fn a_preset_without_lenses_is_byte_identical_to_before() {
        use sha2::{Digest, Sha256};
        let p = fixture();
        for (seed, sha, len) in [
            (
                0u64,
                "eb3867ad5abc2a094fbb1b431ecd8cdce969701aba9bbf29838768b0bc763f48",
                7679usize,
            ),
            (
                1,
                "b2a23f970d75aa1cb8af90008165695e9d11c27dd9d9eed0504150754d4b618c",
                7131,
            ),
            (
                2,
                "3081accfb70f0c2208734dd6de7d2428fc869c6173c744b5553bd0e0ae8892f5",
                8053,
            ),
        ] {
            let out = emit_to_string(&file(&p, seed, 6, Grid::Local));
            let got = crate::synth::hex(&Sha256::digest(out.as_bytes()));
            assert_eq!((got.as_str(), out.len()), (sha, len), "seed {seed}");
        }
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
            let n = rows(&small, "SAMP").len();
            assert_eq!(rows(&small, "SAMP"), &rows(&large, "SAMP")[..n]);
            assert_eq!(rows(&small, "PROJ"), rows(&large, "PROJ"));
        }
    }

    /// No sample straddles a stratum boundary or reaches below final depth,
    /// and the schedule actually produces samples.
    #[test]
    fn samples_lie_wholly_inside_their_stratum() {
        let p = fixture();
        let mut count = 0;
        for seed in 0..60 {
            for h in build(&p, seed, 8).holes {
                for s in &h.strata {
                    for m in &s.samples {
                        count += 1;
                        assert!(m.top >= s.top && m.base <= s.base, "{h:?} {m:?}");
                        assert!(m.top < m.base && m.base <= h.final_depth, "{h:?} {m:?}");
                    }
                    for pair in s.samples.windows(2) {
                        assert!(pair[0].base <= pair[1].top, "{pair:?}");
                    }
                }
            }
        }
        assert!(count > 1000, "{count}");
    }

    /// Sampling is a pass of its own: strip the samples and LOCA and GEOL do
    /// not change by a byte.
    #[test]
    fn sampling_moves_no_loca_or_geol_byte() {
        let p = fixture();
        for seed in [0, 3, 11] {
            let with = build(&p, seed, 8);
            let mut without = with.clone();
            for s in without.holes.iter_mut().flat_map(|h| &mut h.strata) {
                s.samples.clear();
            }
            let (a, b) = (
                lower(&with, &p, seed, Grid::Local),
                lower(&without, &p, seed, Grid::Local),
            );
            for code in ["PROJ", "TRAN", "LOCA", "GEOL"] {
                assert_eq!(rows(&a, code), rows(&b, code), "{code}");
            }
            assert!(rows(&b, "SAMP").is_empty());
            assert!(!rows(&a, "SAMP").is_empty());
        }
    }

    /// SAMP keys are unique and every `SAMP_TYPE` is in ABBR.
    #[test]
    fn samp_keys_are_unique_and_types_are_abbreviated() {
        let p = fixture();
        let m = file(&p, 4, 12, Grid::Local);
        let mut keys: Vec<&[String]> = rows(&m, "SAMP").iter().map(|r| &r.values[..5]).collect();
        let n = keys.len();
        keys.sort();
        keys.dedup();
        assert_eq!(keys.len(), n);
        for r in rows(&m, "SAMP") {
            assert!(
                rows(&m, "ABBR")
                    .iter()
                    .any(|a| a.values[0] == "SAMP_TYPE" && a.values[1] == r.values[3]),
                "{r:?}"
            );
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
                        // The leading term comes first, then the colour.
                        assert!(sb.description.contains(" light brown "), "{sb:?}");
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

    fn with_wlc_skew(skew: &str) -> Preset {
        let text = FIXTURE.replacen(
            "thickness_m = { min = 2.0, max = 8.0,",
            &format!("thickness_m = {{ min = 2.0, max = 8.0, skew = {skew},"),
            1,
        );
        Preset::from_toml(&text).expect("skewed fixture loads")
    }

    /// A skew of 1 changes nothing in the built site; a skew of 2 does.
    #[test]
    fn a_skew_of_one_builds_the_same_site() {
        let plain = build(&fixture(), 4, 25);
        assert_eq!(build(&with_wlc_skew("1.0"), 4, 25), plain);
        assert_ne!(build(&with_wlc_skew("2.0"), 4, 25), plain);
    }

    /// Neighbouring holes still agree under a skew, the bound scaled by it.
    #[test]
    fn neighbouring_holes_have_similar_skewed_unit_thickness() {
        let skew = 2.0;
        let p = with_wlc_skew("2.0");
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
                if a.strata.last().unwrap().unit != "LC" || b.strata.last().unwrap().unit != "LC" {
                    continue;
                }
                let d = (a.x - b.x).hypot(a.y - b.y);
                let bound = (range.max - range.min) * surface.max_slope() * skew * d + 0.02;
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

    /// The expected leading term of a stratum: the catalogue's term for the
    /// property model's value at the stratum's mid-depth.
    fn expected_term(site: &Site, hole: usize, s: &Stratum) -> (&'static str, bool) {
        let mid = f64::midpoint(s.top, s.base);
        let e = site.properties_at(hole, mid).expect("inside the hole");
        match (e.fine, e.spt_n) {
            (Some(f), None) => (
                crate::catalogue::consistency_term_from_ic(f.consistency_index).unwrap(),
                true,
            ),
            (None, Some(n)) => (crate::catalogue::spt_relative_density_term(n), false),
            other => panic!("{other:?}: a fixture unit has one kind of property set"),
        }
    }

    #[test]
    fn a_description_leads_with_the_term_of_its_mid_depth_properties() {
        let p = fixture();
        let (mut checked, mut terms) = (0, std::collections::BTreeSet::new());
        for seed in 0..40 {
            let site = build(&p, seed, 6);
            for (k, hole) in site.holes.iter().enumerate() {
                for s in &hole.strata {
                    let (term, fine) = expected_term(&site, k, s);
                    // RTD is the only coarse unit of the fixture.
                    assert_eq!(fine, s.unit != "RTD", "{s:?}");
                    assert!(
                        s.description.starts_with(&format!("{term} ")),
                        "seed {seed} hole {k} {s:?}: expected {term:?}"
                    );
                    terms.insert(term);
                    checked += 1;
                }
            }
        }
        assert!(checked > 200, "{checked}");
        assert!(terms.len() >= 3, "the fixture varies its terms: {terms:?}");
    }

    /// The lens carries its own term, from its own properties at its own
    /// mid-depth; a host split around it keeps one description for all parts.
    #[test]
    fn a_lens_and_a_split_host_lead_with_their_own_terms() {
        let p = lens_preset();
        let mut lens_strata = 0;
        for seed in 0..20 {
            let site = build(&p, seed, 6);
            for (k, hole) in site.holes.iter().enumerate() {
                for s in &hole.strata {
                    if s.unit == "SB" || s.unit == "CL" {
                        let (term, fine) = expected_term(&site, k, s);
                        assert!(!fine, "{s:?}");
                        assert!(s.description.starts_with(&format!("{term} ")), "{s:?}");
                        lens_strata += 1;
                    }
                }
                for unit in ["WLC", "LC"] {
                    let parts: Vec<_> = hole.strata.iter().filter(|s| s.unit == unit).collect();
                    assert!(
                        parts
                            .windows(2)
                            .all(|w| w[0].description == w[1].description),
                        "{parts:?}"
                    );
                }
            }
        }
        assert!(lens_strata > 10, "{lens_strata}");
    }

    /// A unit with both a fine and a coarse principal carries both property
    /// sets, and each description takes the term of the principal it drew.
    #[test]
    fn a_mixed_unit_leads_with_the_term_of_the_principal_drawn() {
        let mixed = FIXTURE.replace(
            r#"description = { principals = ["CLAY"], secondaries = ["silt"], colours = ["grey", "dark grey"] }"#,
            r#"description = { principals = ["CLAY", "SAND"], secondaries = ["silt"], colours = ["grey"] }"#,
        ) + "spt_n = { min = 10, max = 30, basis = \"test fixture\" }\nmoisture_content_percent = { min = 8, max = 15, basis = \"test fixture\" }\n";
        let p = Preset::from_toml(&mixed).unwrap();
        let (mut fine, mut coarse) = (0, 0);
        for seed in 0..40 {
            let site = build(&p, seed, 6);
            for (k, hole) in site.holes.iter().enumerate() {
                for s in hole.strata.iter().filter(|s| s.unit == "LC") {
                    let mid = f64::midpoint(s.top, s.base);
                    let e = site.properties_at(k, mid).unwrap();
                    let ic = e.fine.as_ref().unwrap().consistency_index;
                    let consistency = crate::catalogue::consistency_term_from_ic(ic).unwrap();
                    let density = crate::catalogue::spt_relative_density_term(e.spt_n.unwrap());
                    if s.description.ends_with("CLAY.") {
                        assert!(
                            s.description.starts_with(&format!("{consistency} ")),
                            "{s:?}"
                        );
                        fine += 1;
                    } else {
                        assert!(s.description.starts_with(&format!("{density} ")), "{s:?}");
                        assert!(s.description.ends_with("SAND."), "{s:?}");
                        coarse += 1;
                    }
                }
            }
        }
        assert!(fine > 10 && coarse > 10, "{fine} fine, {coarse} coarse");
    }
}

#[cfg(test)]
mod shipped_pins {
    use super::*;
    use crate::pipeline::rust_check;
    use crate::site::preset::{SHIPPED, resolve};
    use crate::synth::emit::emit_to_string;
    use laterite_ags4_parity::RustResult;

    /// The file `gen --site <name> --seed <seed>` writes before any injection.
    fn shipped(name: &str, seed: u64) -> String {
        let preset = resolve(name).expect("shipped preset loads");
        assert_eq!(problems(&preset), Vec::<String>::new(), "{name}");
        let site = build(&preset, seed, default_hole_count(seed));
        emit_to_string(&lower(&site, &preset, seed, Grid::Local))
    }

    /// Each shipped preset's default file, pinned like the scaffolds
    /// (`synth_output_bytes_are_pinned`). Consumers commit output from these,
    /// so moving a pin is a preset version bump they must be told about
    /// (ADR 0001): bump `version`, re-pin, and say so in the PR. Re-taken for
    /// london-basin v2 (#41), which adds the properties the descriptions'
    /// leading terms come from, for v3 (#48), which adds the suite and the
    /// calibrated ranges, and for v4 (#42), which adds Made Ground on top.
    /// The versioning promise starts at v3, so v4 is the first bump under it.
    /// tyneside-till (#13) ships at v1, under the promise from the start.
    /// Re-taken for london-basin v5 and tyneside-till v2 (#63), which add SPT
    /// refusals: every N of 50 or more is written as a refusal and the ISPT
    /// suite lists `ISPT_MAIN`, `ISPT_NPEN` and `ISPT_REP`. Re-taken for
    /// london-basin v6 and tyneside-till v3 (#58), which add the cover layer:
    /// the version in the marker moves every file, and a hole with a cover
    /// gains a GEOL row (`a_preset_without_its_cover_writes_the_previous_versions_bytes`
    /// holds the rest of the file to the v5 and v2 pins).
    /// Re-taken for tyneside-till v5 (#68), which adds TREG and TRET to the
    /// suite: the file grows by those two groups and the version in the marker
    /// moves. london-basin has none and is not re-taken.
    /// Re-taken for london-basin v7 and tyneside-till v6 (#74), which add hole
    /// construction: HDPH, CDIA and BKFL, `WSTG_CAS`, and a joined `LOCA_TYPE`
    /// (`a_preset_without_its_construction_writes_the_previous_versions_bytes`
    /// holds the rest of the file to the v6 and v5 pins).
    /// Re-taken for london-basin v8 and tyneside-till v7 (#75), which add
    /// installations: PIPE, and the installed holes' own BKFL
    /// (`a_preset_without_its_installation_writes_the_previous_versions_bytes`
    /// holds the rest of the file to the v7 and v6 pins).
    /// Re-taken for tyneside-till v8 (#77), which adds water strikes in till
    /// (`a_preset_without_its_till_strikes_writes_the_previous_versions_bytes`
    /// holds the rest of the file to the v7 pins; london-basin stays at v8).
    #[test]
    fn shipped_preset_output_bytes_are_pinned() {
        use sha2::{Digest, Sha256};
        for (name, seed, sha, len) in [
            (
                "london-basin",
                0u64,
                "6cc9c72be03891c9fb3cf3222f3296a6fe8d1455f1896653dc36568da9b7de56",
                21355usize,
            ),
            (
                "london-basin",
                1,
                "5cddfccace03f5c48117e63e3284c429842ae47f2702c43e439beb4fe67e510d",
                16410,
            ),
            (
                "london-basin",
                2,
                "7686d64a5df38a684b88e9bfb9a6ac6a59c34f1715f87f7e706ad87e01e649cc",
                30802,
            ),
            (
                "tyneside-till",
                0,
                "d51bc60794b1fd0873087724eec52afd085a54e3ab580d6e555ecb262e0a4e0f",
                34284,
            ),
            (
                "tyneside-till",
                1,
                "231ffbc54eb05485f29221245822bbd99353060e635e949f05f2a760bd5b3284",
                23484,
            ),
            (
                "tyneside-till",
                2,
                "a22dafe3c02a0c0c8a9d5e8d255c549b0c3eab15a0ebc482cae848be1a0ea5a8",
                41357,
            ),
        ] {
            let text = shipped(name, seed);
            let got = crate::synth::hex(&Sha256::digest(text.as_bytes()));
            assert_eq!((got.as_str(), text.len()), (sha, len), "{name} seed {seed}");
        }
        let pinned: std::collections::BTreeSet<&str> = ["london-basin", "tyneside-till"].into();
        let shipped_names: std::collections::BTreeSet<&str> =
            SHIPPED.iter().map(|(n, _)| *n).collect();
        assert_eq!(
            shipped_names, pinned,
            "every shipped preset needs a byte pin"
        );
    }

    /// A preset without its `[cover]` block, at the version before it, writes
    /// the bytes that version pinned (london-basin v5, tyneside-till v2): the
    /// cover layer, the sample ranks it added and the suite's draw order move
    /// nothing in a preset that has no cover.
    #[test]
    fn a_preset_without_its_cover_writes_the_previous_versions_bytes() {
        use sha2::{Digest, Sha256};
        for (name, version, seed, sha, len) in [
            (
                "london-basin",
                5,
                0u64,
                "49e93f1fc529556ef5c06a2bbeedc0b9a81cc2f3e5c06b9905a72bd9c2ccde2b",
                20115usize,
            ),
            (
                "london-basin",
                5,
                1,
                "0a04448b3b53eb7b0a5969a1b0acb32d0a8d8fe95b2a209424c1c4ca4a855c50",
                15378,
            ),
            (
                "london-basin",
                5,
                2,
                "1c26c8428b1f1216f1dd876ab174e1d6f642d7abaddde6bd66b8535f7e458251",
                28109,
            ),
            (
                "tyneside-till",
                2,
                0,
                "46bce99b86cc02884b1546a446154d1488115cf5d26ba9571b0e4a4faa3ec157",
                18263,
            ),
            (
                "tyneside-till",
                2,
                1,
                "dd0abb7db5afe612229bfde54edbd29599acad5b45178c30972037867d27dc66",
                13395,
            ),
            (
                "tyneside-till",
                2,
                2,
                "ea6251e4496bcdbe3bff37b4615faf3ab8d6e3890e6d0d5a2ff744f3dc7f7728",
                24087,
            ),
        ] {
            let mut preset = resolve(name).expect("shipped preset loads");
            preset.cover = None;
            // And v6 and v7's hole construction (#74).
            preset.construction = None;
            // tyneside-till v4's multi-stage share (#2) is the other thing
            // since v2 that changes bytes; without it the bytes are v2's.
            for p in preset
                .units
                .iter_mut()
                .map(|u| &mut u.properties)
                .chain(preset.lenses.iter_mut().map(|l| &mut l.properties))
            {
                p.triaxial = None;
                p.effective_strength = None;
            }
            // And v5's effective-stress triaxial (#68): no TREG or TRET entry,
            // so the groups are not written and no draw order moves.
            preset
                .suite
                .retain(|e| e.group != "TREG" && e.group != "TRET");
            // And v8's till strikes (#77): no strike table on any unit.
            for u in &mut preset.units {
                u.strike = None;
            }
            preset.version = version;
            let site = build(&preset, seed, default_hole_count(seed));
            let text = emit_to_string(&lower(&site, &preset, seed, Grid::Local));
            let got = crate::synth::hex(&Sha256::digest(text.as_bytes()));
            assert_eq!(
                (got.as_str(), text.len()),
                (sha, len),
                "{name} v{version} seed {seed}"
            );
        }
    }

    /// A preset without its `[construction]` block, at the version before it,
    /// writes the bytes that version pinned (london-basin v6, tyneside-till
    /// v5): hole construction moves nothing in a preset that has none.
    #[test]
    fn a_preset_without_its_construction_writes_the_previous_versions_bytes() {
        use sha2::{Digest, Sha256};
        for (name, version, seed, sha, len) in [
            (
                "london-basin",
                6,
                0u64,
                "de3114e4cb7bfc1cb9d144e6f119d59790fae69d4968da0b0dbfd50e3a0991ad",
                20115usize,
            ),
            (
                "london-basin",
                6,
                1,
                "6e8a903ab5ddb8988c11533cc916247d711264b9a2ed019c9567afb38660be58",
                15378,
            ),
            (
                "london-basin",
                6,
                2,
                "b8fd1d966322a8d6b5c807b0a6992ee8d276d6ae8aa4c926059c5cbdbe48a26c",
                28219,
            ),
            (
                "tyneside-till",
                5,
                0,
                "0df73e8de31e7b86b369b22ae3533b381705d41c68a5b2f4994cda09d248240a",
                31497,
            ),
            (
                "tyneside-till",
                5,
                1,
                "dc9a0cf79dd85582949f764655f0a23e1e3af4fde42d9826f2e12b0aa84d8683",
                22453,
            ),
            (
                "tyneside-till",
                5,
                2,
                "8ad47c0c1b8ff3407a95fef972c12c6e0623eb690b273c040f329a5225fa58b2",
                38233,
            ),
        ] {
            let mut preset = resolve(name).expect("shipped preset loads");
            preset.construction = None;
            // And v8's till strikes (#77): no strike table on any unit.
            for u in &mut preset.units {
                u.strike = None;
            }
            preset.version = version;
            let site = build(&preset, seed, default_hole_count(seed));
            let text = emit_to_string(&lower(&site, &preset, seed, Grid::Local));
            let got = crate::synth::hex(&Sha256::digest(text.as_bytes()));
            assert_eq!(
                (got.as_str(), text.len()),
                (sha, len),
                "{name} v{version} seed {seed}"
            );
        }
    }

    /// A preset without its `[construction.installation]` table, at the
    /// version before it, writes the bytes that version pinned (london-basin
    /// v7, tyneside-till v6): installations move nothing in a preset that has
    /// none.
    #[test]
    fn a_preset_without_its_installation_writes_the_previous_versions_bytes() {
        use sha2::{Digest, Sha256};
        for (name, version, seed, sha, len) in [
            (
                "london-basin",
                7,
                0u64,
                "8b9b58c6874a5b27b393c7abc40c01c2decf8559af3cd5cc4d77c797388cc105",
                21355usize,
            ),
            (
                "london-basin",
                7,
                1,
                "1ecf4d875bb7a08cb3358ad8ab1f6cc1a0a1ebce640d47689878f3d4d2987ae2",
                16410,
            ),
            (
                "london-basin",
                7,
                2,
                "a5a9255a0b2f1c4f16b2c0e2bbc23d01f90be2f120903a22b47a39ab5ddddc97",
                29577,
            ),
            (
                "tyneside-till",
                6,
                0,
                "8d18dcc67fb77ba27d357b5f6a765feabab72d8ed0975c05f477e476044ecb6a",
                32675,
            ),
            (
                "tyneside-till",
                6,
                1,
                "8d6e7b5fd3d004f858c0a558ecf0be88ad28bd55b372b0cd27f86b905c004692",
                23484,
            ),
            (
                "tyneside-till",
                6,
                2,
                "00a73ad4f60ee059c67a81f6b0dabc782da03da41e1ec8091ec9e08957d05ed8",
                39537,
            ),
        ] {
            let mut preset = resolve(name).expect("shipped preset loads");
            preset
                .construction
                .as_mut()
                .expect("a construction block")
                .installation = None;
            // And v8's till strikes (#77): no strike table on any unit.
            for u in &mut preset.units {
                u.strike = None;
            }
            preset.version = version;
            let site = build(&preset, seed, default_hole_count(seed));
            let text = emit_to_string(&lower(&site, &preset, seed, Grid::Local));
            let got = crate::synth::hex(&Sha256::digest(text.as_bytes()));
            assert_eq!(
                (got.as_str(), text.len()),
                (sha, len),
                "{name} v{version} seed {seed}"
            );
        }
    }

    /// A preset without its `strike` tables, at the version before them,
    /// writes the bytes that version pinned (london-basin v8, which has none
    /// and did not change, and tyneside-till v7): till strikes move nothing in
    /// a preset that has none.
    #[test]
    fn a_preset_without_its_till_strikes_writes_the_previous_versions_bytes() {
        use sha2::{Digest, Sha256};
        for (name, version, seed, sha, len) in [
            (
                "london-basin",
                8,
                0u64,
                "6cc9c72be03891c9fb3cf3222f3296a6fe8d1455f1896653dc36568da9b7de56",
                21355usize,
            ),
            (
                "london-basin",
                8,
                1,
                "5cddfccace03f5c48117e63e3284c429842ae47f2702c43e439beb4fe67e510d",
                16410,
            ),
            (
                "london-basin",
                8,
                2,
                "7686d64a5df38a684b88e9bfb9a6ac6a59c34f1715f87f7e706ad87e01e649cc",
                30802,
            ),
            (
                "tyneside-till",
                7,
                0,
                "962eeb79f812b52a8dfc134415ca3c94899a9ea548a9729f1d455c1d46ab0442",
                33710,
            ),
            (
                "tyneside-till",
                7,
                1,
                "6e7b81dbf31a369e17999ca002203f97069b965730d16d0ecefff47cd521322a",
                23484,
            ),
            (
                "tyneside-till",
                7,
                2,
                "238207c4075ae6d4f9c3a8cc4c832f9dd76643222b112cf357384fff69cf5955",
                40773,
            ),
        ] {
            let mut preset = resolve(name).expect("shipped preset loads");
            for u in &mut preset.units {
                u.strike = None;
            }
            preset.version = version;
            let site = build(&preset, seed, default_hole_count(seed));
            let text = emit_to_string(&lower(&site, &preset, seed, Grid::Local));
            let got = crate::synth::hex(&Sha256::digest(text.as_bytes()));
            assert_eq!(
                (got.as_str(), text.len()),
                (sha, len),
                "{name} v{version} seed {seed}"
            );
        }
    }

    /// The AGS TYPEs each shipped preset's files declare, across seeds. v3's
    /// suite adds 0DP (`LLPL_LL`, `LLPL_PI`, `TRIT_CU`, `TRIT_DEVF`, `TRIT_CELL`,
    /// `ISPT_NVAL`) and XN (`LLPL_PL`). v4's Made Ground adds none: its rows are
    /// GEOL, SAMP and ISPT, whose TYPEs v3 already declares. tyneside-till
    /// (#13) writes the same groups, so it declares the same TYPEs; its v5
    /// (#68) adds `TREG_PHI` (1DP).
    #[test]
    fn shipped_presets_reach_a_pinned_set_of_ags_types() {
        for (name, expected) in [
            (
                "london-basin",
                &["0DP", "2DP", "DT", "ID", "PA", "X", "XN"][..],
            ),
            (
                "tyneside-till",
                &["0DP", "1DP", "2DP", "DT", "ID", "PA", "X", "XN"][..],
            ),
        ] {
            let mut types = std::collections::BTreeSet::new();
            for seed in 0..10 {
                let text = shipped(name, seed);
                for line in text.lines().filter_map(|l| l.strip_prefix("\"TYPE\",")) {
                    types.extend(line.split(',').map(|f| f.trim_matches('"').to_string()));
                }
            }
            let expected: std::collections::BTreeSet<String> =
                expected.iter().map(|t| (*t).to_string()).collect();
            assert_eq!(types, expected, "{name}");
        }
    }

    /// tyneside-till over more seeds than the shared check below: clean by
    /// construction, with a lens stratum and a water strike in some files.
    #[test]
    fn tyneside_till_is_rust_clean_across_seeds() {
        let (mut lens, mut struck) = (0, 0);
        for seed in 0..40 {
            let text = shipped("tyneside-till", seed);
            // Only the lens has a coarse principal; the tills' are CLAY.
            lens += usize::from(text.contains(" SAND") || text.contains(" GRAVEL"));
            struck += usize::from(text.contains("\"GROUP\",\"WSTG\""));
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("site.ags");
            std::fs::write(&path, text).unwrap();
            assert_eq!(rust_check(&path).0, RustResult::Clean, "seed {seed}");
        }
        assert!(
            lens > 5 && struck > 5,
            "{lens} files with a lens, {struck} with a strike"
        );
    }

    /// Clean by construction across seeds.
    #[test]
    fn shipped_presets_are_rust_clean() {
        for (name, _) in SHIPPED {
            for seed in 0..10 {
                let dir = tempfile::tempdir().unwrap();
                let path = dir.path().join("site.ags");
                std::fs::write(&path, shipped(name, seed)).unwrap();
                assert_eq!(rust_check(&path).0, RustResult::Clean, "{name} seed {seed}");
            }
        }
    }
}

/// Made-ground units (niko86/laterite-forge#42).
#[cfg(test)]
pub(super) mod made_ground_tests {
    use super::tests::FIXTURE;
    use super::*;
    use crate::pipeline::rust_check;
    use crate::synth::emit::emit_to_string;
    use crate::synth::made_ground::TAG;
    use laterite_ags4_parity::RustResult;

    /// A coarse made-ground candidate on top, always 2-3 m thick. `{SPEC}`
    /// is the unit's `made_ground` line, or nothing for a plain unit.
    const MG_UNIT: &str = r#"[[units]]
id = "MG"
name = "Made Ground"
thickness_m = { min = 2.0, max = 3.0, basis = "test fixture" }
description = { principals = ["SAND", "GRAVEL"], secondaries = ["clay"], colours = ["dark brown", "black"] }
{SPEC}
sampling = { types = ["B", "D"], interval_m = { min = 1.0, max = 1.5, basis = "test fixture" } }

[units.properties]
spt_n = { min = 4, max = 30, basis = "test fixture" }
moisture_content_percent = { min = 8, max = 15, basis = "test fixture" }
particle_density_ratio = { min = 2.6, max = 2.7, basis = "test fixture" }

"#;

    /// [`FIXTURE`] with [`MG_UNIT`] above the terrace deposits, carrying
    /// `spec` as its `made_ground` line.
    pub(in crate::site) fn with_made_ground(spec: &str) -> String {
        FIXTURE.replacen(
            "[[units]]\nid = \"RTD\"",
            &format!("{}[[units]]\nid = \"RTD\"", MG_UNIT.replace("{SPEC}", spec)),
            1,
        )
    }

    fn preset(spec: &str) -> Preset {
        Preset::from_toml(&with_made_ground(spec)).expect("made-ground fixture loads")
    }

    const WEIGHTED: &str = r#"made_ground = { constituents = [{ name = "brick", share = 0.6, basis = "test fixture" }, { name = "concrete", share = { min = 0.5, max = 0.5, basis = "test fixture" } }, { name = "ash", share = 0.3, basis = "test fixture" }, { name = "pottery", share = 0.2, basis = "test fixture" }], count = { weights = [0.2, 0.3, 0.3, 0.2], basis = "test fixture" } }"#;

    const NO_CONSTITUENTS: &str =
        r#"made_ground = { count = { weights = [1, 0, 0, 0], basis = "test fixture" } }"#;

    fn words(text: &str) -> Vec<&str> {
        text.split(|c: char| !c.is_ascii_alphabetic()).collect()
    }

    fn mg_strata(site: &Site) -> impl Iterator<Item = &Stratum> {
        site.holes
            .iter()
            .flat_map(|h| &h.strata)
            .filter(|s| s.unit == "MG")
    }

    #[test]
    fn a_made_ground_unit_describes_every_stratum_as_made_ground_within_its_constraints() {
        let p = preset(WEIGHTED);
        assert!(problems(&p).is_empty());
        let mut seen = 0;
        for seed in 0..40 {
            for s in mg_strata(&build(&p, seed, 6)) {
                seen += 1;
                let t = &s.description;
                assert!(t.starts_with(TAG), "{t}");
                let w = words(t);
                assert!(w.contains(&"SAND") || w.contains(&"GRAVEL"), "{t}");
                assert!(!w.contains(&"CLAY") && !w.contains(&"SILT"), "{t}");
                assert!(t.contains("dark brown") || t.contains("black"), "{t}");
            }
        }
        assert!(seen >= 200, "{seen}");
    }

    #[test]
    fn a_made_ground_matrix_is_the_plain_units_with_its_term_from_the_model() {
        // With a count of 0 the description is the tag over the matrix, and
        // the matrix is exactly what the same unit draws when it is not made
        // ground: same constraints, same property-model term. Every other
        // unit is untouched.
        let plain = preset("");
        let made = preset(NO_CONSTITUENTS);
        for seed in 0..20 {
            let (a, b) = (build(&plain, seed, 6), build(&made, seed, 6));
            for (ha, hb) in a.holes.iter().zip(&b.holes) {
                assert_eq!(ha.strata.len(), hb.strata.len());
                for (sa, sb) in ha.strata.iter().zip(&hb.strata) {
                    if sa.unit == "MG" {
                        assert_eq!(sb.description, format!("{TAG}{}", sa.description));
                    } else {
                        assert_eq!(sa, sb, "seed {seed}");
                    }
                }
            }
        }
    }

    #[test]
    #[allow(clippy::cast_precision_loss)]
    fn mention_shares_and_the_count_distribution_follow_the_preset() {
        let p = preset(WEIGHTED);
        let names = ["brick", "concrete", "ash", "pottery"];
        let want = [0.6, 0.5, 0.3, 0.2];
        let weights = [0.2, 0.3, 0.3, 0.2];
        let (mut n, mut mentions, mut counts) = (0.0_f64, [0.0_f64; 4], [0.0_f64; 4]);
        for seed in 0..300 {
            for s in mg_strata(&build(&p, seed, 6)) {
                n += 1.0;
                let w = words(&s.description);
                let mut count = 0;
                for (i, name) in names.iter().enumerate() {
                    if w.contains(name) {
                        mentions[i] += 1.0;
                        count += 1;
                    }
                }
                counts[usize::min(count, 3)] += 1.0;
            }
        }
        for i in 0..4 {
            let got = mentions[i] / n;
            assert!((got - want[i]).abs() < 0.04, "{}: {got}", names[i]);
            let got = counts[i] / n;
            assert!((got - weights[i]).abs() < 0.04, "count {i}: {got}");
        }
    }

    #[test]
    fn a_unit_with_amount_weights_uses_only_the_weighted_amounts() {
        let p = preset(
            r#"made_ground = { count = { weights = [0, 0, 0, 1], basis = "test fixture" }, amounts = { weights = [0, 1, 0, 0, 0], basis = "test fixture" } }"#,
        );
        let mut seen = 0;
        for seed in 0..20 {
            for s in mg_strata(&build(&p, seed, 6)) {
                seen += 1;
                let t = &s.description;
                assert_eq!(t.matches("occasional").count(), 1, "{t}");
                for other in ["rare", "frequent", "numerous", "abundant"] {
                    assert!(!words(t).contains(&other), "{t}");
                }
            }
        }
        assert!(seen > 0);
    }

    #[test]
    fn a_count_of_zero_leaves_no_with_clause_but_keeps_the_tag() {
        let p = preset(NO_CONSTITUENTS);
        let mut seen = 0;
        for seed in 0..20 {
            for s in mg_strata(&build(&p, seed, 6)) {
                seen += 1;
                assert!(s.description.starts_with(TAG), "{}", s.description);
                assert!(!s.description.contains(" with "), "{}", s.description);
            }
        }
        assert!(seen > 0);
    }

    #[test]
    fn made_ground_true_takes_the_lanes_default_draw() {
        let p = preset("made_ground = true");
        let default = made_ground::constituents();
        for seed in 0..40 {
            for s in mg_strata(&build(&p, seed, 6)) {
                let t = &s.description;
                assert!(t.starts_with(TAG), "{t}");
                let named: Vec<&str> = words(t)
                    .into_iter()
                    .filter(|w| default.vocabulary().any(|c| c == w))
                    .collect();
                assert!((1..=2).contains(&named.len()), "{t}");
                assert!(
                    named
                        .iter()
                        .all(|n| default.constituents.iter().any(|c| c == n)),
                    "{t}"
                );
                assert!(t.contains("fragments"), "{t}");
            }
        }
    }

    #[test]
    fn a_made_ground_unit_samples_like_any_unit() {
        let p = preset(WEIGHTED);
        let site = build(&p, 3, 6);
        let mut seen = 0;
        for s in mg_strata(&site) {
            for smp in &s.samples {
                seen += 1;
                assert!(["B", "D"].contains(&smp.code.as_str()), "{}", smp.code);
                assert!(smp.top >= s.top - 1e-9 && smp.base <= s.base + 1e-9);
            }
        }
        assert!(seen > 0);
    }

    #[test]
    fn a_made_ground_site_lowers_to_a_clean_file() {
        let p = preset(WEIGHTED);
        for seed in 0..4 {
            let m = lower(&build(&p, seed, 6), &p, seed, Grid::Local);
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("site.ags");
            std::fs::write(&path, emit_to_string(&m)).unwrap();
            assert_eq!(rust_check(&path).0, RustResult::Clean, "seed {seed}");
        }
    }

    /// [`with_made_ground`] with the made-ground unit's description also
    /// carrying `very_coarse`, written as a TOML inline table.
    fn with_very_coarse(spec: &str, very_coarse: &str) -> String {
        let text = with_made_ground(spec);
        let from = r#"colours = ["dark brown", "black"] }"#;
        assert!(text.contains(from));
        text.replacen(
            from,
            &format!(r#"colours = ["dark brown", "black"], very_coarse = {very_coarse} }}"#),
            1,
        )
    }

    const ONE_BRICK: &str = r#"made_ground = { constituents = [{ name = "brick", share = 1.0, basis = "test fixture" }], count = { weights = [0, 1, 0, 0], basis = "test fixture" } }"#;

    #[test]
    fn very_coarse_content_precedes_the_made_ground_constituents() {
        let p = Preset::from_toml(&with_very_coarse(
            ONE_BRICK,
            r#"{ cobbles = "low", boulders = "medium" }"#,
        ))
        .expect("loads");
        let mut seen = 0;
        for seed in 0..10 {
            for s in mg_strata(&build(&p, seed, 6)) {
                seen += 1;
                let clause = " with low cobble content and medium boulder content with ";
                assert!(s.description.starts_with(TAG), "{}", s.description);
                assert!(
                    s.description.ends_with(" brick fragments."),
                    "constituents close the description: {}",
                    s.description
                );
                let at = s.description.find(clause).expect(&s.description);
                let principal = s.description[..at].rsplit(' ').next().unwrap();
                assert!(
                    ["SAND", "GRAVEL"].contains(&principal),
                    "the clause follows the principal: {}",
                    s.description
                );
                assert_eq!(s.description.matches('.').count(), 1, "{}", s.description);
            }
        }
        assert!(seen > 0);
    }

    #[test]
    fn very_coarse_content_closes_a_made_ground_with_no_constituents() {
        let p = Preset::from_toml(&with_very_coarse(
            NO_CONSTITUENTS,
            r#"{ cobbles = "high", boulders = "high" }"#,
        ))
        .expect("loads");
        for s in mg_strata(&build(&p, 2, 6)) {
            assert!(
                s.description
                    .ends_with(" with high boulder or cobble content."),
                "{}",
                s.description
            );
        }
    }

    /// Without the field a unit's text is the one it had, plain or made ground.
    #[test]
    fn a_unit_without_very_coarse_is_unchanged_by_the_field_existing() {
        for spec in [WEIGHTED, ""] {
            let plain = build(&preset(spec), 5, 6);
            let with = build(
                &Preset::from_toml(&with_very_coarse(spec, r#"{ cobbles = "low" }"#))
                    .expect("loads"),
                5,
                6,
            );
            let strata = |s: &Site| -> Vec<(String, String)> {
                s.holes
                    .iter()
                    .flat_map(|h| &h.strata)
                    .map(|st| (st.unit.clone(), st.description.clone()))
                    .collect()
            };
            let (plain, with) = (strata(&plain), strata(&with));
            assert_eq!(plain.len(), with.len());
            for ((u, a), (_, b)) in plain.iter().zip(&with) {
                if u == "MG" {
                    assert_ne!(a, b);
                    assert!(b.contains(" with low cobble content"), "{b}");
                } else {
                    assert_eq!(a, b, "unit {u} moved");
                }
            }
        }
    }

    #[test]
    fn a_very_coarse_site_lowers_to_a_clean_file() {
        for spec in [WEIGHTED, ""] {
            let p = Preset::from_toml(&with_very_coarse(
                spec,
                r#"{ cobbles = "low", boulders = "medium" }"#,
            ))
            .expect("loads");
            for seed in 0..4 {
                let m = lower(&build(&p, seed, 6), &p, seed, Grid::Local);
                let dir = tempfile::tempdir().unwrap();
                let path = dir.path().join("site.ags");
                std::fs::write(&path, emit_to_string(&m)).unwrap();
                assert_eq!(rust_check(&path).0, RustResult::Clean, "seed {seed}");
            }
        }
    }
}
