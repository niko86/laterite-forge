//! The **made-ground** lane of the BS 5930 description engine.
//!
//! Takes an already drawn matrix [`SoilDescription`] and returns the made
//! ground description: `MADE GROUND: ` + the matrix text + man-made
//! constituents as a tertiary `with …` clause at the end of the soil
//! description proper (before any grading sentence), e.g.
//! `MADE GROUND: Firm dark brown slightly sandy gravelly CLAY with frequent
//! brick and occasional concrete fragments.`
//!
//! Amounts come from the vendored `tertiaryAmounts`; the constituents come
//! from forge-authored `data/made-ground/constituents.json` (see its
//! PROVENANCE.md). The lane draws from an RNG of its own, so it never adds a
//! draw to `describe` / `describe_with`.

use super::bs5930::{SoilDescription, Vocab};
use laterite_ags4_parity::Rng;
use serde::Deserialize;

const CONSTITUENTS_JSON: &str = include_str!("../../data/made-ground/constituents.json");

/// The tag that opens every made-ground description.
pub const TAG: &str = "MADE GROUND: ";

/// The forge-authored constituent vocabulary.
#[derive(Debug, Deserialize)]
pub struct ConstituentList {
    /// Where the list comes from.
    pub basis: String,
    /// Word closing the clause (`fragments`).
    pub suffix: String,
    pub constituents: Vec<String>,
}

impl ConstituentList {
    /// Parse and check the embedded data. Panics on malformed or invalid
    /// data (a build-time guarantee, asserted by the tests).
    pub fn load() -> ConstituentList {
        let c: ConstituentList =
            serde_json::from_str(CONSTITUENTS_JSON).expect("made-ground constituents.json parses");
        c.check().expect("made-ground constituents.json is valid");
        c
    }

    fn check(&self) -> Result<(), String> {
        if self.basis.trim().is_empty() {
            return Err("basis is empty".into());
        }
        if self.suffix.trim().is_empty() {
            return Err("suffix is empty".into());
        }
        if self.constituents.len() < 2 {
            return Err("fewer than two constituents".into());
        }
        for (i, c) in self.constituents.iter().enumerate() {
            if c.trim().is_empty() || c != c.trim() {
                return Err(format!("blank or padded constituent at {i}"));
            }
            if self.constituents[..i].contains(c) {
                return Err(format!("duplicate constituent '{c}'"));
            }
        }
        Ok(())
    }
}

/// The process-wide constituent vocabulary, parsed once.
pub fn constituents() -> &'static ConstituentList {
    static C: std::sync::OnceLock<ConstituentList> = std::sync::OnceLock::new();
    C.get_or_init(ConstituentList::load)
}

/// One man-made constituent with its amount.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Constituent {
    /// A vendored `tertiaryAmounts` term, e.g. `with frequent`.
    pub amount: String,
    /// A constituent from `constituents.json`, e.g. `brick`.
    pub name: String,
}

/// A made-ground description: the matrix fields stay honest and the
/// constituents are exposed alongside.
#[derive(Debug, Clone)]
pub struct MadeGround {
    /// `text` is the full made-ground text; every other field is the
    /// matrix's.
    pub description: SoilDescription,
    /// One or two, in order of decreasing frequency.
    pub constituents: Vec<Constituent>,
}

/// Salt for the lane's own RNG stream, so drawing constituents never moves the
/// matrix draw. `describe --made-ground` and the tests share it, so a seed
/// gives the same constituents in both.
pub const SALT: u64 = 0x4d41_4445_4752_4e44;

/// Turn a drawn matrix description into made ground, drawing from `rng`
/// (use a stream of its own, not the one that drew the matrix).
// `below(n)` returns a value < n, and every n here is a small `Vec::len()`
// widened to u64, so the narrowing back to usize is lossless.
#[allow(clippy::cast_possible_truncation)]
pub fn made_ground(matrix: &SoilDescription, vocab: &Vocab, rng: &mut Rng) -> MadeGround {
    let list = &constituents().constituents;
    let amounts = vocab.tertiary_amounts();
    let want = if amounts.len() >= 2 && rng.below(2) == 0 {
        2
    } else {
        1
    };

    // Distinct names, then distinct amount indices ordered most frequent
    // first.
    let mut names: Vec<&String> = Vec::new();
    while names.len() < want {
        let n = &list[rng.below(list.len() as u64) as usize];
        if !names.contains(&n) {
            names.push(n);
        }
    }
    let mut idx: Vec<usize> = Vec::new();
    while idx.len() < want {
        let i = rng.below(amounts.len() as u64) as usize;
        if !idx.contains(&i) {
            idx.push(i);
        }
    }
    idx.sort_unstable_by(|a, b| b.cmp(a));

    let parts: Vec<Constituent> = names
        .into_iter()
        .zip(idx)
        .map(|(n, i)| Constituent {
            amount: amounts[i].clone(),
            name: n.clone(),
        })
        .collect();

    // The first amount keeps its `with`; the second follows `and`.
    let clause = parts
        .iter()
        .enumerate()
        .map(|(i, k)| {
            let amount = if i == 0 {
                k.amount.as_str()
            } else {
                k.amount.strip_prefix("with ").unwrap_or(&k.amount)
            };
            format!("{amount} {}", k.name)
        })
        .collect::<Vec<_>>()
        .join(" and ");

    // Tertiaries close the soil description proper: before the first full
    // stop, so a grading sentence stays after them.
    let body = &matrix.text;
    let stop = body.find('.').unwrap_or(body.len());
    let (head, tail) = body.split_at(stop);
    let tail = if tail.is_empty() { "." } else { tail };
    let text = format!("{TAG}{head} {clause} {}{tail}", constituents().suffix);

    MadeGround {
        description: SoilDescription {
            text,
            ..matrix.clone()
        },
        constituents: parts,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::synth::bs5930::{Lanes, describe_with, vocab};

    fn draw(seed: u64, lanes: Lanes) -> MadeGround {
        let m = describe_with(vocab(), &mut Rng::seeded(seed), lanes);
        made_ground(&m, vocab(), &mut Rng::seeded(seed ^ SALT))
    }

    #[test]
    fn the_constituent_data_loads_and_is_sound() {
        let c = ConstituentList::load();
        assert_eq!(c.basis, "common UK logging practice");
        assert!(!c.constituents.is_empty());
        let mut sorted = c.constituents.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(sorted.len(), c.constituents.len(), "no duplicates");
    }

    #[test]
    fn a_duplicate_or_basisless_list_is_refused() {
        let mut c = ConstituentList::load();
        c.constituents.push("brick".into());
        assert!(c.check().is_err());
        let mut c = ConstituentList::load();
        c.basis = String::new();
        assert!(c.check().is_err());
    }

    #[test]
    fn every_made_ground_description_is_tagged_and_ends_with_a_with_clause() {
        let c = constituents();
        let amounts = vocab().tertiary_amounts();
        assert!(!amounts.is_empty());
        for seed in 0..500 {
            let m = describe_with(vocab(), &mut Rng::seeded(seed), Lanes::ALL);
            let g = draw(seed, Lanes::ALL);
            let t = &g.description.text;
            assert!(t.starts_with(TAG), "seed {seed}: {t}");
            assert!(t.contains(m.principal), "seed {seed}: {t}");
            assert_eq!(g.description.principal, m.principal);
            assert_eq!(g.description.lead, m.lead);
            assert_eq!(g.description.secondaries.len(), m.secondaries.len());
            let first = t.split_once('.').unwrap().0;
            assert!(!g.constituents.is_empty() && g.constituents.len() <= 2);
            let clause = first.split_once(" with ").map_or("", |x| x.1);
            assert!(first.ends_with(&c.suffix), "seed {seed}: {t}");
            let mut names = Vec::new();
            for k in &g.constituents {
                assert!(amounts.contains(&k.amount), "seed {seed}: {}", k.amount);
                assert!(c.constituents.contains(&k.name), "seed {seed}: {}", k.name);
                assert!(clause.contains(&k.name), "seed {seed}: {t}");
                names.push(&k.name);
            }
            names.sort();
            names.dedup();
            assert_eq!(names.len(), g.constituents.len(), "distinct, seed {seed}");
            // Decreasing frequency: the index in the vendored list falls.
            let idx: Vec<usize> = g
                .constituents
                .iter()
                .map(|k| amounts.iter().position(|a| *a == k.amount).unwrap())
                .collect();
            assert!(idx.windows(2).all(|w| w[0] > w[1]), "order, seed {seed}");
        }
    }

    #[test]
    fn both_one_and_two_constituents_occur() {
        let counts: Vec<usize> = (0..200)
            .map(|s| draw(s, Lanes::STANDARD).constituents.len())
            .collect();
        assert!(counts.contains(&1) && counts.contains(&2));
    }

    #[test]
    fn a_grading_sentence_follows_the_constituents() {
        // Seed 2's matrix is "Very loose cream very gravelly clayey SAND.
        // Sand is angular fine."; the clause lands before the first stop.
        assert_eq!(
            draw(2, Lanes::STANDARD).description.text,
            "MADE GROUND: Very loose cream very gravelly clayey SAND with frequent metal \
             fragments. Sand is angular fine."
        );
    }

    #[test]
    fn made_ground_does_not_move_the_matrix_draw() {
        let a = describe_with(vocab(), &mut Rng::seeded(7), Lanes::ALL).text;
        let _ = draw(7, Lanes::ALL);
        let b = describe_with(vocab(), &mut Rng::seeded(7), Lanes::ALL).text;
        assert_eq!(a, b);
    }
}
