//! The cover layer: topsoil or hard surfacing at the top of a hole (ADR 0001,
//! "Cover layer").
//!
//! Each hole makes one keyed draw that gives it surfacing, topsoil or nothing.
//! The cover takes its thickness off the top of the hole's first stratum:
//! that stratum is written from the cover base, and every other boundary,
//! sample, test and strike is where it would have been without a cover.
//! Nothing is sampled in the cover. Every draw has its own keyed stream, so a
//! preset without a `[cover]` block draws nothing and keeps its bytes.

use laterite_ags4_parity::Rng;

use super::build::{SAMPLE_LENGTH_M, Sample, Stratum};
use super::draw::{Key, uniform};
use super::preset::{Cover, Range, Surfacing, Topsoil};
use crate::synth::bs5930;
use crate::synth::made_ground;

/// The surfacing kinds a preset may weight, as they read after
/// `MADE GROUND: ` (forge judgement, following Norbury 2010's Made Ground
/// convention; the BGS crawls word these surfaces the same ways).
pub const KINDS: [&str; 3] = ["Tarmacadam", "Concrete", "Paving slabs"];

/// The least a first stratum keeps when a cover is drawn on it, metres. A
/// cover that would leave less is cut back to leave this much (forge
/// judgement: a sliver of ground under the cover would be a logging artefact,
/// and taking the cover into the next stratum would move that boundary).
pub const MIN_REMAINING_M: f64 = 0.05;

/// The thinnest cover written: one unit in the last decimal place.
const MIN_THICKNESS_M: f64 = 0.01;

/// What the cover is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoverKind {
    Surfacing,
    Topsoil,
}

/// The cover of one hole: a stratum from 0 to `base` that holds no sample.
#[derive(Debug, Clone, PartialEq)]
pub struct CoverLayer {
    pub kind: CoverKind,
    /// Where the cover ends and the first stratum begins, 2 decimal places.
    pub base: f64,
    pub description: String,
}

/// The cover on one site: each layer's share of holes, drawn once.
#[derive(Debug, Clone, PartialEq)]
pub struct SiteCover<'a> {
    cover: &'a Cover,
    surfacing_share: f64,
    topsoil_share: f64,
}

impl<'a> SiteCover<'a> {
    /// The site's shares, one draw each in the layer's range from a stream
    /// of its own.
    pub fn new(cover: &'a Cover, seed: u64) -> Self {
        let share = |tag: &str, r: Option<&Range>| {
            r.map_or(0.0, |r| {
                uniform(&mut Key::site(tag).rng(seed), r.min, r.max)
            })
        };
        SiteCover {
            cover,
            surfacing_share: share(
                "cover-share-surfacing",
                cover.surfacing.as_ref().map(|s| &s.share),
            ),
            topsoil_share: share(
                "cover-share-topsoil",
                cover.topsoil.as_ref().map(|t| &t.share),
            ),
        }
    }

    /// The kind hole `hi` gets, if any. One uniform draw in `[0, 1)`:
    /// surfacing below its share, topsoil above one less its share. The loader
    /// holds the share maxima to a sum of at most 1, so the two never meet,
    /// and a share moves only its own layer's holes.
    fn kind(&self, hi: usize, seed: u64) -> Option<CoverKind> {
        let u = uniform(&mut Key::hole("cover", hi).rng(seed), 0.0, 1.0);
        if self.cover.surfacing.is_some() && u < self.surfacing_share {
            Some(CoverKind::Surfacing)
        } else if self.cover.topsoil.is_some() && u >= 1.0 - self.topsoil_share {
            Some(CoverKind::Topsoil)
        } else {
            None
        }
    }

    /// Put hole `hi`'s cover on `strata` (top-down, the first from 0), or
    /// `None` when the hole has none. The first stratum keeps its top and its
    /// description; only its samples change.
    ///
    /// The drawn thickness is cut back so the first stratum keeps
    /// [`MIN_REMAINING_M`]; a cover cut to nothing is absent. A sample whose
    /// top falls in the cover is dropped, except that the first such sample
    /// moves to the cover base when it still fits in its stratum and clears
    /// the next sample.
    pub fn apply(&self, hi: usize, seed: u64, strata: &mut [Stratum]) -> Option<CoverLayer> {
        let kind = self.kind(hi, seed)?;
        let first = strata.first_mut()?;
        let range = match (kind, &self.cover.surfacing, &self.cover.topsoil) {
            (CoverKind::Surfacing, Some(s), _) => &s.thickness_m,
            (CoverKind::Topsoil, _, Some(t)) => &t.thickness_m,
            _ => unreachable!("a kind is drawn only for a layer the preset has"),
        };
        let thickness = thickness(range, &mut Key::hole("cover-thickness", hi).rng(seed));
        let room = round2(first.base - first.top - MIN_REMAINING_M);
        let thickness = thickness.min(room);
        if thickness < MIN_THICKNESS_M {
            return None;
        }
        let base = round2(first.top + thickness);
        first.samples = displace(std::mem::take(&mut first.samples), base, first.base);
        let description = match (kind, &self.cover.surfacing, &self.cover.topsoil) {
            (CoverKind::Surfacing, Some(s), _) => surfacing_description(s, hi, seed),
            (CoverKind::Topsoil, _, Some(t)) => topsoil_description(t, hi, seed),
            _ => unreachable!("a kind is drawn only for a layer the preset has"),
        };
        Some(CoverLayer {
            kind,
            base,
            description,
        })
    }
}

/// `min + (max - min) * u^skew`, rounded to the written precision.
fn thickness(range: &Range, rng: &mut Rng) -> f64 {
    let u = uniform(rng, 0.0, 1.0);
    let u = range.skew.map_or(u, |k| u.powf(k));
    round2(range.min + (range.max - range.min) * u)
}

fn surfacing_description(s: &Surfacing, hi: usize, seed: u64) -> String {
    let total: f64 = s.kinds.iter().map(|k| k.weight).sum();
    let mut at = uniform(&mut Key::hole("cover-kind", hi).rng(seed), 0.0, total);
    let kind = s
        .kinds
        .iter()
        .find(|k| {
            if at < k.weight {
                true
            } else {
                at -= k.weight;
                false
            }
        })
        .or(s.kinds.last())
        .expect("the loader requires a kind");
    format!("{}{}.", made_ground::TAG, kind.name)
}

fn topsoil_description(t: &Topsoil, hi: usize, seed: u64) -> String {
    bs5930::describe_topsoil(
        bs5930::vocab(),
        &mut Key::hole("cover-description", hi).rng(seed),
        &t.description,
    )
    .expect("problems() was checked before build")
}

/// The samples left once the cover down to `cover_base` is off the stratum
/// that ends at `stratum_base`. A sample whose top is in the cover is dropped,
/// except the first, which moves to the cover base if it then ends inside the
/// stratum and before the next sample's top.
fn displace(mut samples: Vec<Sample>, cover_base: f64, stratum_base: f64) -> Vec<Sample> {
    let split = samples.iter().take_while(|s| s.top < cover_base).count();
    let rest = samples.split_off(split);
    let base = round2(cover_base + SAMPLE_LENGTH_M);
    let fits = base <= stratum_base && rest.first().is_none_or(|next| next.top >= base);
    let moved = samples.into_iter().next().filter(|_| fits).map(|s| Sample {
        top: cover_base,
        base,
        ..s
    });
    moved.into_iter().chain(rest).collect()
}

fn round2(v: f64) -> f64 {
    (v * 100.0).round() / 100.0
}

// Depths and shares are exact literals or exact endpoints; the counts widen to
// floats to be compared with shares.
#[allow(
    clippy::float_cmp,
    clippy::cast_precision_loss,
    clippy::many_single_char_names
)]
#[cfg(test)]
mod tests {
    use super::*;
    use crate::pipeline::rust_check;
    use crate::site::build::tests::FIXTURE;
    use crate::site::build::{Grid, Hole, build, lower, problems};
    use crate::site::preset::{Preset, resolve};
    use crate::site::water::strikes;
    use crate::synth::emit::emit_to_string;
    use crate::synth::model::{ProjectModel, Row};
    use laterite_ags4_parity::RustResult;

    /// A cover on the main fixture. The shares sum to at most 0.7, and both
    /// thickness maxima are under the fixture's shallowest water table (1 m).
    const COVER: &str = r#"
[cover.surfacing]
share = { min = 0.2, max = 0.3, basis = "test fixture" }
thickness_m = { min = 0.05, max = 0.5, skew = 2.0, basis = "test fixture" }
kinds = [
    { name = "Tarmacadam", weight = 2 },
    { name = "Concrete", weight = 1 },
]
kinds_basis = "test fixture"

[cover.topsoil]
share = { min = 0.3, max = 0.4, basis = "test fixture" }
thickness_m = { min = 0.1, max = 0.8, basis = "test fixture" }

[cover.topsoil.description]
colours = [
    { colour = "dark brown", weight = 3 },
    { colour = "brown", weight = 1 },
]
secondaries = [
    { fraction = "sand", share = 0.8 },
    { fraction = "gravel", share = 0.5 },
]
basis = "test fixture"
"#;

    fn text() -> String {
        format!("{FIXTURE}{COVER}")
    }

    /// The fixture with one exact snippet of the cover replaced.
    fn with(from: &str, to: &str) -> String {
        assert!(COVER.contains(from), "cover fixture lacks {from:?}");
        text().replacen(from, to, 1)
    }

    /// Both shares set to `s`.
    fn shares(s: &str) -> String {
        with(
            "share = { min = 0.2, max = 0.3,",
            &format!("share = {{ min = {s}, max = {s},"),
        )
        .replacen(
            "share = { min = 0.3, max = 0.4,",
            &format!("share = {{ min = {s}, max = {s},"),
            1,
        )
    }

    fn preset() -> Preset {
        Preset::from_toml(&text()).expect("the cover fixture loads")
    }

    fn off(p: &Preset) -> Preset {
        Preset {
            cover: None,
            ..p.clone()
        }
    }

    fn model(p: &Preset, seed: u64, holes: usize) -> ProjectModel {
        lower(&build(p, seed, holes), p, seed, Grid::Local)
    }

    fn refused(text: &str, needle: &str) {
        let errs = Preset::from_toml(text).expect_err("should be refused").0;
        assert!(
            errs.iter().any(|e| e.contains(needle)),
            "{needle:?} is not among {errs:#?}"
        );
    }

    #[test]
    fn the_loader_reads_a_cover_into_typed_values() {
        let p = preset();
        assert_eq!(problems(&p), Vec::<String>::new());
        let c = p.cover.as_ref().expect("a cover");
        let s = c.surfacing.as_ref().expect("surfacing");
        assert_eq!((s.share.min, s.share.max), (0.2, 0.3));
        assert_eq!((s.thickness_m.min, s.thickness_m.max), (0.05, 0.5));
        assert_eq!(s.thickness_m.skew, Some(2.0));
        let kinds: Vec<(&str, f64)> = s
            .kinds
            .iter()
            .map(|k| (k.name.as_str(), k.weight))
            .collect();
        assert_eq!(kinds, [("Tarmacadam", 2.0), ("Concrete", 1.0)]);
        let t = c.topsoil.as_ref().expect("topsoil");
        assert_eq!((t.share.min, t.share.max), (0.3, 0.4));
        assert_eq!(t.thickness_m.skew, None);
        assert_eq!(t.description.colours[0], ("dark brown".to_string(), 3.0));
        assert_eq!(t.description.secondaries[1], ("gravel".to_string(), 0.5));
    }

    #[test]
    fn a_preset_may_carry_one_layer_alone_or_none() {
        let topsoil_only = text().replace(&COVER[..COVER.find("[cover.topsoil]").unwrap()], "\n");
        let c = Preset::from_toml(&topsoil_only)
            .expect("topsoil alone loads")
            .cover
            .expect("a cover");
        assert!(c.surfacing.is_none() && c.topsoil.is_some());
        assert!(Preset::from_toml(FIXTURE).unwrap().cover.is_none());
    }

    #[test]
    fn the_loader_refuses_shares_above_one_naming_both() {
        refused(
            &with(
                "share = { min = 0.3, max = 0.4,",
                "share = { min = 0.3, max = 0.9,",
            ),
            "cover: surfacing share max 0.3 plus topsoil share max 0.9 is above 1",
        );
        // The sum of the maxima is what must hold, so 0.5 + 0.5 is allowed.
        assert!(Preset::from_toml(&shares("0.5")).is_ok());
    }

    #[test]
    fn the_loader_refuses_a_share_outside_zero_to_one() {
        refused(
            &with(
                "share = { min = 0.2, max = 0.3,",
                "share = { min = -0.1, max = 0.3,",
            ),
            "cover.surfacing.share: min -0.1 is outside 0 to 1",
        );
    }

    #[test]
    fn the_loader_refuses_a_bad_thickness_naming_the_field() {
        refused(
            &with("thickness_m = { min = 0.05,", "thickness_m = { min = 0.0,"),
            "cover.surfacing.thickness_m: min 0 must be > 0",
        );
        refused(
            &with("max = 0.5, skew = 2.0", "max = 0.5, skew = 0.5"),
            "cover.surfacing.thickness_m.skew: 0.5 must be >= 1",
        );
        refused(
            &with(
                "thickness_m = { min = 0.1, max = 0.8,",
                "thickness_m = { min = 0.9, max = 0.8,",
            ),
            "cover.topsoil.thickness_m: min 0.9 is greater than max 0.8",
        );
    }

    #[test]
    fn the_loader_refuses_a_cover_that_reaches_the_water_table() {
        refused(
            &with(
                "thickness_m = { min = 0.1, max = 0.8,",
                "thickness_m = { min = 0.1, max = 1.0,",
            ),
            "cover.topsoil.thickness_m: max 1 is not above the shallowest water table \
             (site.groundwater_depth_m min 1)",
        );
    }

    #[test]
    fn the_loader_refuses_a_missing_basis_naming_the_field() {
        refused(
            &with(
                "share = { min = 0.2, max = 0.3, basis = \"test fixture\" }",
                "share = { min = 0.2, max = 0.3 }",
            ),
            "cover.surfacing.share: no basis",
        );
        refused(
            &with("kinds_basis = \"test fixture\"\n", ""),
            "cover.surfacing.kinds_basis: missing",
        );
        refused(
            &with("]\nbasis = \"test fixture\"\n", "]\n"),
            "cover.topsoil.description.basis: missing",
        );
        refused(
            &with(
                "thickness_m = { min = 0.1, max = 0.8, basis = \"test fixture\" }",
                "thickness_m = { min = 0.1, max = 0.8 }",
            ),
            "cover.topsoil.thickness_m: no basis",
        );
    }

    #[test]
    fn the_loader_refuses_bad_kinds_colours_and_fractions() {
        refused(
            &with("name = \"Concrete\"", "name = \"Asphalt\""),
            "cover.surfacing.kinds[1].name: unknown kind \"Asphalt\" (known: Tarmacadam, \
             Concrete, Paving slabs)",
        );
        refused(
            &with("name = \"Concrete\"", "name = \"Tarmacadam\""),
            "\"Tarmacadam\" is listed twice",
        );
        refused(
            &with("weight = 2 }", "weight = -2 }"),
            "cover.surfacing.kinds[0].weight: -2 is negative",
        );
        refused(
            &with("weight = 2 }", "weight = 0 }").replacen("weight = 1 }", "weight = 0 }", 1),
            "cover.surfacing.kinds: weights sum to zero",
        );
        refused(
            &with("share = 0.8 }", "share = 1.5 }"),
            "cover.topsoil.description.secondaries[0].share: 1.5 is outside 0 to 1",
        );
        refused(
            &with("fraction = \"gravel\"", "fraction = \"sand\""),
            "\"sand\" is listed twice",
        );
    }

    #[test]
    fn the_loader_refuses_an_empty_cover_and_unknown_keys() {
        refused(
            &format!("{FIXTURE}\n[cover]\n"),
            "cover: names neither surfacing nor topsoil",
        );
        refused(
            &with("kinds_basis = ", "colour = \"red\"\nkinds_basis = "),
            "cover.surfacing: unknown key \"colour\"",
        );
    }

    #[test]
    fn a_colour_or_fraction_the_vocabulary_cannot_meet_is_a_problem() {
        let p = Preset::from_toml(&with("colour = \"brown\"", "colour = \"mauve\"")).unwrap();
        let found = problems(&p);
        assert_eq!(found.len(), 1, "{found:?}");
        assert!(
            found[0].starts_with("cover.topsoil.description: "),
            "{found:?}"
        );
        assert!(found[0].contains("mauve"), "{found:?}");
        let p = Preset::from_toml(&with("fraction = \"sand\"", "fraction = \"loam\"")).unwrap();
        assert!(problems(&p)[0].contains("unknown secondary \"loam\""));
    }

    /// Each share is one draw per site in its range, from its own stream, and
    /// the holes follow it.
    #[test]
    fn the_shares_are_drawn_once_per_site_and_the_holes_match_them() {
        let p = preset();
        let cover = p.cover.as_ref().unwrap();
        let mut distinct = std::collections::BTreeSet::new();
        let (mut surf_total, mut top_total, mut all) = (0.0, 0.0, 0.0);
        for seed in 0..30 {
            let site = SiteCover::new(cover, seed);
            assert_eq!(site, SiteCover::new(cover, seed), "seed {seed}");
            assert!((0.2..=0.3).contains(&site.surfacing_share), "seed {seed}");
            assert!((0.3..=0.4).contains(&site.topsoil_share), "seed {seed}");
            distinct.insert(site.surfacing_share.to_bits());
            let n = 6000;
            let (mut s, mut t) = (0, 0);
            for hi in 0..n {
                match site.kind(hi, seed) {
                    Some(CoverKind::Surfacing) => s += 1,
                    Some(CoverKind::Topsoil) => t += 1,
                    None => {}
                }
            }
            let (s, t, n) = (f64::from(s), f64::from(t), n as f64);
            assert!((s / n - site.surfacing_share).abs() < 0.03, "seed {seed}");
            assert!((t / n - site.topsoil_share).abs() < 0.03, "seed {seed}");
            surf_total += s;
            top_total += t;
            all += n;
        }
        assert!(distinct.len() > 20, "the share should vary by site");
        // Pooled over sites, the expected shares are the range midpoints.
        assert!(
            (surf_total / all - 0.25).abs() < 0.01,
            "{}",
            surf_total / all
        );
        assert!((top_total / all - 0.35).abs() < 0.01, "{}", top_total / all);
    }

    /// A share moves only its own layer's holes, and a hole's draw does not
    /// depend on how many holes there are.
    #[test]
    fn a_share_moves_only_its_own_layer_and_a_hole_ignores_the_hole_count() {
        let p = preset();
        let wide = Preset::from_toml(&with(
            "share = { min = 0.3, max = 0.4,",
            "share = { min = 0.6, max = 0.7,",
        ))
        .unwrap();
        for seed in 0..10 {
            let a = SiteCover::new(p.cover.as_ref().unwrap(), seed);
            let b = SiteCover::new(wide.cover.as_ref().unwrap(), seed);
            for hi in 0..500 {
                let surfacing = |s: &SiteCover| s.kind(hi, seed) == Some(CoverKind::Surfacing);
                assert_eq!(surfacing(&a), surfacing(&b), "hole {hi} seed {seed}");
            }
        }
        let small = build(&p, 5, 4);
        let large = build(&p, 5, 12);
        for (s, l) in small.holes.iter().zip(&large.holes) {
            assert_eq!(s, l);
        }
    }

    /// A hole has at most one layer: with shares of one half each, every hole
    /// has exactly one, and both layers appear.
    #[test]
    fn surfacing_and_topsoil_are_mutually_exclusive() {
        let p = Preset::from_toml(&shares("0.5")).unwrap();
        let site = SiteCover::new(p.cover.as_ref().unwrap(), 3);
        let (mut s, mut t) = (0, 0);
        for hi in 0..2000 {
            match site.kind(hi, 3).expect("every hole has one layer") {
                CoverKind::Surfacing => s += 1,
                CoverKind::Topsoil => t += 1,
            }
        }
        assert_eq!(s + t, 2000);
        assert!(s > 800 && t > 800, "{s} surfacing, {t} topsoil");
        // And in a built site, a hole carries one cover whose description is
        // its kind's.
        for h in &build(&p, 3, 30).holes {
            let c = h.cover.as_ref().expect("a cover");
            assert_eq!(
                c.description.starts_with(made_ground::TAG),
                c.kind == CoverKind::Surfacing
            );
        }
    }

    fn holes_with(p: &Preset, seeds: std::ops::Range<u64>, n: usize) -> Vec<(Hole, Hole)> {
        let bare = off(p);
        seeds
            .flat_map(|seed| {
                let (a, b) = (build(p, seed, n), build(&bare, seed, n));
                a.holes.into_iter().zip(b.holes)
            })
            .collect()
    }

    /// Everything below the cover base is where it was without the cover; the
    /// only sample that differs is the one that moved to the base.
    #[test]
    fn nothing_below_the_cover_base_moves_and_nothing_is_sampled_in_it() {
        let p = preset();
        let (mut covered, mut moved, mut dropped, mut kept_in_place) = (0, 0, 0, 0);
        for (on, bare) in holes_with(&p, 0..150, 10) {
            assert_eq!((&on.id, on.final_depth), (&bare.id, bare.final_depth));
            assert_eq!(on.strata.len(), bare.strata.len());
            assert_eq!(on.strata[1..], bare.strata[1..], "{}", on.id);
            let (a, b) = (&on.strata[0], &bare.strata[0]);
            assert_eq!(
                (&a.unit, a.top, a.base, &a.description),
                (&b.unit, b.top, b.base, &b.description)
            );
            let Some(cover) = &on.cover else {
                assert_eq!(a.samples, b.samples, "{}", on.id);
                continue;
            };
            covered += 1;
            assert!(
                cover.base > 0.0 && cover.base <= a.base - MIN_REMAINING_M + 1e-9,
                "{cover:?} over {a:?}"
            );
            assert_eq!(on.written_top(0), cover.base);
            assert!(
                a.samples.iter().all(|s| s.top >= cover.base),
                "{} sampled in the cover",
                on.id
            );
            let kept: Vec<_> = b
                .samples
                .iter()
                .filter(|s| s.top >= cover.base)
                .cloned()
                .collect();
            let tail = &a.samples[a.samples.len() - kept.len()..];
            assert_eq!(tail, kept.as_slice(), "{}", on.id);
            let in_cover = b.samples.len() - kept.len();
            match a.samples.len() - kept.len() {
                0 => dropped += usize::from(in_cover > 0),
                1 => {
                    moved += 1;
                    let s = &a.samples[0];
                    let was = &b.samples[0];
                    assert_eq!((s.top, s.rank, &s.code), (cover.base, was.rank, &was.code));
                    assert_eq!(s.base, round2(cover.base + SAMPLE_LENGTH_M));
                    assert!(s.base <= a.base);
                }
                n => panic!("{} gained {n} samples", on.id),
            }
            kept_in_place += kept.len();
        }
        assert!(covered > 100, "{covered} covered holes");
        assert!(moved > 5 && dropped > 0, "{moved} moved, {dropped} dropped");
        assert!(kept_in_place > 300);
    }

    /// The same, on the rows the file writes, for a preset with the whole suite
    /// and its water: every row deeper than a hole's cover base is the row the
    /// preset writes without a cover.
    #[test]
    fn a_cover_changes_no_row_below_its_base_in_any_group() {
        let p = resolve("london-basin").expect("shipped");
        let bare = off(&p);
        let mut checked = 0;
        for seed in 0..25 {
            let (a, b) = (build(&p, seed, 12), build(&bare, seed, 12));
            let base_of = |id: &str| {
                a.holes
                    .iter()
                    .find(|h| h.id == id)
                    .and_then(|h| h.cover.as_ref())
                    .map(|c| c.base)
            };
            let ma = lower(&a, &p, seed, Grid::Local);
            let mb = lower(&b, &bare, seed, Grid::Local);
            assert_eq!(ma.groups.len(), mb.groups.len());
            for (ga, gb) in ma.groups.iter().zip(&mb.groups) {
                assert_eq!(ga.code, gb.code);
                if matches!(ga.code.as_str(), "ABBR" | "UNIT" | "TYPE") {
                    continue;
                }
                let id = ga.headings.iter().position(|h| h == "LOCA_ID");
                let depth = ga.headings.iter().position(|h| {
                    ["GEOL_TOP", "SAMP_TOP", "ISPT_TOP", "WSTG_DPTH"].contains(&h.as_str())
                });
                let below = |r: &&Row| match (id, depth) {
                    (Some(i), Some(d)) => base_of(&r.values[i])
                        .is_none_or(|base| r.values[d].parse::<f64>().unwrap() > base),
                    _ => true,
                };
                let ra: Vec<&Row> = ga.rows.iter().filter(below).collect();
                let rb: Vec<&Row> = gb.rows.iter().filter(below).collect();
                assert_eq!(ra, rb, "{} at seed {seed}", ga.code);
                checked += ra.len();
            }
        }
        assert!(checked > 5000, "{checked} rows compared");
    }

    #[test]
    fn the_first_stratum_is_written_from_the_cover_base() {
        let p = preset();
        let site = build(&p, 4, 40);
        let m = lower(&site, &p, 4, Grid::Local);
        let geol = &m.groups.iter().find(|g| g.code == "GEOL").unwrap().rows;
        let mut covered = 0;
        for h in &site.holes {
            let rows: Vec<_> = geol.iter().filter(|r| r.values[0] == h.id).collect();
            let skip = usize::from(h.cover.is_some());
            assert_eq!(rows.len(), h.strata.len() + skip);
            if let Some(c) = &h.cover {
                covered += 1;
                assert_eq!(
                    rows[0].values[1..],
                    [
                        "0.00".to_string(),
                        format!("{:.2}", c.base),
                        c.description.clone()
                    ]
                );
                assert_eq!(rows[1].values[1], format!("{:.2}", c.base));
            }
            for pair in rows.windows(2) {
                assert_eq!(
                    pair[0].values[2], pair[1].values[1],
                    "contiguous in {}",
                    h.id
                );
            }
        }
        assert!(covered > 5);
    }

    /// A first stratum keeps `MIN_REMAINING_M`: a cover thicker than that is
    /// cut back to it, and a stratum with no room has no cover. Cutting back
    /// keeps every boundary below the first where it was; taking the cover
    /// into the next stratum would not.
    #[test]
    fn a_thin_first_stratum_caps_the_cover() {
        let thin = text().replacen(
            "thickness_m = { min = -3.0, max = 6.0,",
            "thickness_m = { min = 0.2, max = 0.3,",
            1,
        );
        let thick = |t: &str| {
            t.replacen(
                "thickness_m = { min = 0.05, max = 0.5, skew = 2.0,",
                "thickness_m = { min = 0.5, max = 0.9,",
                1,
            )
            .replacen(
                "thickness_m = { min = 0.1, max = 0.8,",
                "thickness_m = { min = 0.5, max = 0.9,",
                1,
            )
            .replacen(
                "share = { min = 0.2, max = 0.3,",
                "share = { min = 0.5, max = 0.5,",
                1,
            )
            .replacen(
                "share = { min = 0.3, max = 0.4,",
                "share = { min = 0.5, max = 0.5,",
                1,
            )
        };
        let p = Preset::from_toml(&thick(&thin)).unwrap();
        let mut capped = 0;
        for (on, bare) in holes_with(&p, 0..10, 10) {
            let cover = on.cover.as_ref().expect("room for a cover");
            assert_eq!(
                cover.base,
                round2(on.strata[0].base - MIN_REMAINING_M),
                "{}",
                on.id
            );
            assert_eq!(on.strata[1..], bare.strata[1..]);
            capped += 1;
        }
        assert_eq!(capped, 100);
        // No room at all: a first stratum of 0.05 m gets no cover.
        let none = thin.replacen("min = 0.2, max = 0.3,", "min = 0.05, max = 0.05,", 1);
        let p = Preset::from_toml(&thick(&none)).unwrap();
        assert!(build(&p, 1, 30).holes.iter().all(|h| h.cover.is_none()));
    }

    fn sample(top: f64, rank: usize) -> Sample {
        Sample {
            top,
            base: round2(top + SAMPLE_LENGTH_M),
            code: format!("S{rank}"),
            rank,
        }
    }

    /// The first sample in the cover moves to the base when it fits; the rest
    /// in the cover are dropped; ranks stay as scheduled.
    #[test]
    fn displacing_samples_moves_the_first_or_drops_it() {
        let s = |tops: &[f64]| -> Vec<Sample> {
            tops.iter()
                .enumerate()
                .map(|(i, t)| sample(*t, i))
                .collect()
        };
        let tops = |v: Vec<Sample>| v.iter().map(|x| (x.top, x.rank)).collect::<Vec<_>>();
        // Nothing in the cover.
        assert_eq!(
            tops(displace(s(&[0.6, 2.0]), 0.5, 9.0)),
            [(0.6, 0), (2.0, 1)]
        );
        // One in the cover, room before the next: it moves, keeping its rank.
        assert_eq!(
            tops(displace(s(&[0.2, 2.0]), 0.5, 9.0)),
            [(0.5, 0), (2.0, 1)]
        );
        // The next sample starts inside the moved one (0.5 to 0.95): dropped.
        assert_eq!(tops(displace(s(&[0.2, 0.9]), 0.5, 9.0)), [(0.9, 1)]);
        // The next starts exactly where the moved one ends: it moves.
        assert_eq!(
            tops(displace(s(&[0.2, 0.95]), 0.5, 9.0)),
            [(0.5, 0), (0.95, 1)]
        );
        // The moved sample would pass the stratum base: dropped.
        assert_eq!(tops(displace(s(&[0.2]), 0.5, 0.9)), []);
        assert_eq!(tops(displace(s(&[0.2]), 0.5, 0.95)), [(0.5, 0)]);
        // Two in the cover: the first moves, the second is dropped, and the
        // sample after them keeps rank 2.
        assert_eq!(
            tops(displace(s(&[0.0, 0.3, 1.2]), 0.6, 9.0)),
            [(0.6, 0), (1.2, 2)]
        );
        // A sample exactly at the base is not in the cover.
        assert_eq!(tops(displace(s(&[0.5]), 0.5, 9.0)), [(0.5, 0)]);
    }

    /// A removed sample leaves a gap in the numbering and nothing else: the
    /// ranks stay increasing, and some hole lost one.
    #[test]
    fn a_removed_sample_leaves_the_numbering_of_the_rest() {
        let p = resolve("london-basin").expect("shipped");
        let mut gaps = 0;
        for seed in 0..25 {
            let site = build(&p, seed, 12);
            for h in &site.holes {
                let ranks: Vec<usize> = h
                    .strata
                    .iter()
                    .flat_map(|s| s.samples.iter().map(|x| x.rank))
                    .collect();
                assert!(ranks.windows(2).all(|w| w[0] < w[1]), "{ranks:?}");
                gaps += ranks.iter().enumerate().filter(|(i, r)| i != *r).count();
            }
        }
        assert!(
            gaps > 0,
            "some hole should have lost a sample from its numbering"
        );
    }

    fn all_covers(p: &Preset, n: u64, holes: usize) -> Vec<(CoverKind, String)> {
        (0..n)
            .flat_map(|seed| build(p, seed, holes).holes)
            .filter_map(|h| h.cover.map(|c| (c.kind, c.description)))
            .collect()
    }

    #[test]
    fn a_surfacing_is_made_ground_of_a_weighted_kind() {
        let p = preset();
        let all = all_covers(&p, 30, 40);
        let surf: Vec<&str> = all
            .iter()
            .filter(|(k, _)| *k == CoverKind::Surfacing)
            .map(|(_, d)| d.as_str())
            .collect();
        assert!(surf.len() > 200);
        let tarmac = surf
            .iter()
            .filter(|d| **d == "MADE GROUND: Tarmacadam.")
            .count();
        let concrete = surf
            .iter()
            .filter(|d| **d == "MADE GROUND: Concrete.")
            .count();
        assert_eq!(tarmac + concrete, surf.len(), "only the listed kinds");
        let share = tarmac as f64 / surf.len() as f64;
        assert!((share - 2.0 / 3.0).abs() < 0.06, "tarmacadam share {share}");
    }

    #[test]
    fn a_topsoil_reads_colour_fractions_then_topsoil_with_rootlets() {
        let p = preset();
        let all = all_covers(&p, 30, 40);
        let topsoil: Vec<&str> = all
            .iter()
            .filter(|(k, _)| *k == CoverKind::Topsoil)
            .map(|(_, d)| d.as_str())
            .collect();
        assert!(topsoil.len() > 300);
        let (mut dark, mut sandy, mut gravelly, mut both) = (0, 0, 0, 0);
        for d in &topsoil {
            let body = d
                .strip_suffix(" TOPSOIL with rootlets.")
                .unwrap_or_else(|| panic!("{d}"));
            assert!(body.chars().next().unwrap().is_uppercase(), "{d}");
            let lower = body.to_lowercase();
            let colour = ["dark brown", "brown"]
                .into_iter()
                .find(|c| lower.starts_with(c))
                .unwrap_or_else(|| panic!("{d}"));
            dark += usize::from(colour == "dark brown");
            let words: Vec<&str> = lower[colour.len()..].split_whitespace().collect();
            // Each fraction is an adjective, optionally after slightly or very.
            let mut i = 0;
            let mut named = Vec::new();
            while i < words.len() {
                if matches!(words[i], "slightly" | "very") {
                    i += 1;
                }
                assert!(["sandy", "gravelly"].contains(&words[i]), "{d}");
                named.push(words[i]);
                i += 1;
            }
            let (s, g) = (named.contains(&"sandy"), named.contains(&"gravelly"));
            sandy += usize::from(s);
            gravelly += usize::from(g);
            both += usize::from(s && g);
        }
        let n = topsoil.len() as f64;
        assert!(
            (dark as f64 / n - 0.75).abs() < 0.06,
            "dark brown {}",
            dark as f64 / n
        );
        assert!(
            (sandy as f64 / n - 0.8).abs() < 0.06,
            "sandy {}",
            sandy as f64 / n
        );
        assert!(
            (gravelly as f64 / n - 0.5).abs() < 0.06,
            "gravelly {}",
            gravelly as f64 / n
        );
        assert!(both > 0);
    }

    #[test]
    fn no_row_of_any_group_lies_in_a_cover() {
        let p = resolve("london-basin").expect("shipped");
        let mut checked = 0;
        for seed in 0..25 {
            let site = build(&p, seed, 12);
            let m = lower(&site, &p, seed, Grid::Local);
            for h in &site.holes {
                let Some(c) = &h.cover else { continue };
                checked += 1;
                for g in &m.groups {
                    let Some(i) = g.headings.iter().position(|x| x == "LOCA_ID") else {
                        continue;
                    };
                    let Some(d) = g
                        .headings
                        .iter()
                        .position(|x| ["SAMP_TOP", "ISPT_TOP", "WSTG_DPTH"].contains(&x.as_str()))
                    else {
                        continue;
                    };
                    for r in g.rows.iter().filter(|r| r.values[i] == h.id) {
                        let depth: f64 = r.values[d].parse().unwrap();
                        assert!(
                            depth >= c.base,
                            "{} row at {depth} in the cover of {} (seed {seed})",
                            g.code,
                            h.id
                        );
                    }
                }
            }
        }
        assert!(checked > 20);
    }

    /// A strike at the top of a water-bearing first stratum is at the cover
    /// base, not under the cover; a table inside the cover cannot happen
    /// because the loader holds the cover above the shallowest table.
    #[test]
    fn a_water_bearing_first_stratum_strikes_at_the_cover_base() {
        let wet = text()
            .replacen(
                "name = \"River Terrace Deposits\"\n",
                "name = \"River Terrace Deposits\"\nwater_bearing = true\n",
                1,
            )
            .replacen(
                "thickness_m = { min = -3.0, max = 6.0,",
                "thickness_m = { min = 0.6, max = 1.5,",
                1,
            );
        let p = Preset::from_toml(&wet).unwrap();
        let (mut under, mut bare_top) = (0, 0);
        for seed in 0..20 {
            let site = build(&p, seed, 12);
            for s in strikes(&site, &p, seed) {
                let h = &site.holes[s.hole];
                assert_eq!(s.depth, h.written_top(0), "seed {seed} {}", h.id);
                match &h.cover {
                    Some(c) => {
                        assert_eq!(s.depth, c.base);
                        under += 1;
                    }
                    None => bare_top += 1,
                }
            }
        }
        assert!(
            under > 10 && bare_top > 10,
            "{under} under a cover, {bare_top} bare"
        );
    }

    #[test]
    fn a_zero_share_writes_the_file_it_wrote_without_a_cover() {
        let p = Preset::from_toml(&shares("0.0")).unwrap();
        for seed in 0..5 {
            assert_eq!(
                emit_to_string(&model(&p, seed, 8)),
                emit_to_string(&model(&off(&p), seed, 8)),
                "seed {seed}"
            );
        }
    }

    #[test]
    fn a_cover_file_is_rust_clean_across_seeds() {
        let p = preset();
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("site.ags");
        for seed in 0..30 {
            std::fs::write(&path, emit_to_string(&model(&p, seed, 8))).unwrap();
            assert_eq!(rust_check(&path).0, RustResult::Clean, "seed {seed}");
        }
    }

    #[test]
    fn the_shipped_presets_are_rust_clean_with_many_covers() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("site.ags");
        for name in ["london-basin", "tyneside-till"] {
            let p = resolve(name).expect("shipped");
            assert!(p.cover.is_some(), "{name} has a cover");
            for seed in 0..15 {
                std::fs::write(&path, emit_to_string(&model(&p, seed, 14))).unwrap();
                assert_eq!(rust_check(&path).0, RustResult::Clean, "{name} seed {seed}");
            }
        }
    }
}
