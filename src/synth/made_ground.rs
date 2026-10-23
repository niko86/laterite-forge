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
    /// The default draw's candidates. Their order is part of the pinned
    /// default draw: never append to or reorder this list.
    pub constituents: Vec<String>,
    /// Further words a preset's weights may name. The default draw never
    /// picks from them, so adding one leaves it byte-identical.
    #[serde(default)]
    pub extended: Vec<String>,
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
        let all: Vec<&String> = self.vocabulary().collect();
        for (i, c) in all.iter().enumerate() {
            if c.trim().is_empty() || *c != c.trim() {
                return Err(format!("blank or padded constituent at {i}"));
            }
            if all[..i].contains(c) {
                return Err(format!("duplicate constituent '{c}'"));
            }
        }
        Ok(())
    }

    /// Every word a preset may name: the default candidates, then the
    /// extended ones.
    pub fn vocabulary(&self) -> impl Iterator<Item = &String> {
        self.constituents.iter().chain(&self.extended)
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
    /// In order of decreasing frequency: one or two from [`made_ground`],
    /// none to [`MAX_COUNT`] from [`made_ground_mix`].
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

    compose(matrix, parts)
}

/// Build the description text from the matrix and the chosen constituents,
/// most frequent first. No constituents leaves the matrix text under the tag,
/// without a `with` clause.
fn compose(matrix: &SoilDescription, parts: Vec<Constituent>) -> MadeGround {
    // The first amount keeps its `with`; the rest follow `and` (or a comma).
    let phrases: Vec<String> = parts
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
        .collect();
    let clause = match phrases.split_last() {
        None => String::new(),
        Some((last, [])) => last.clone(),
        Some((last, rest)) => format!("{} and {last}", rest.join(", ")),
    };

    // Tertiaries close the soil description proper: before the first full
    // stop, so a grading sentence stays after them.
    let body = &matrix.text;
    let stop = body.find('.').unwrap_or(body.len());
    let (head, tail) = body.split_at(stop);
    let tail = if tail.is_empty() { "." } else { tail };
    let text = if clause.is_empty() {
        format!("{TAG}{head}{tail}")
    } else {
        format!("{TAG}{head} {clause} {}{tail}", constituents().suffix)
    };

    MadeGround {
        description: SoilDescription {
            text,
            ..matrix.clone()
        },
        constituents: parts,
    }
}

/// The most constituents a stratum names. The count distribution's `3+`
/// bucket draws 3, 4 or 5 with equal chance (`forge judgement`: BGS reports
/// the bucket, not its spread).
pub const MAX_COUNT: usize = 5;

/// A preset's weighted constituents: which words, how often each is
/// mentioned, and how many a stratum names.
///
/// Each stratum draws its count `n` from the count distribution, then a set
/// of `n` distinct constituents by conditional Poisson sampling, whose
/// inclusion probabilities are exact. The tilt weights are fitted so that the
/// fraction of strata mentioning each constituent comes out as its share,
/// wherever the shares and the count distribution agree (their sum equals the
/// mean count); where they do not, the fit is a compromise.
#[derive(Debug, Clone)]
pub struct Mix {
    /// Constituents with a non-zero share, most shared first.
    names: Vec<String>,
    shares: Vec<f64>,
    tilt: Vec<f64>,
    /// `P(count = k)` for `k` in `0..=MAX_COUNT`.
    count: [f64; MAX_COUNT + 1],
}

/// The elementary symmetric polynomials `e_0..=e_n` of `w`.
fn elementary(w: impl IntoIterator<Item = f64>, n: usize) -> Vec<f64> {
    let mut e = vec![0.0; n + 1];
    e[0] = 1.0;
    for x in w {
        for k in (1..=n).rev() {
            e[k] += x * e[k - 1];
        }
    }
    e
}

impl Mix {
    /// `shares` pairs a constituent with the fraction of strata that mention
    /// it (0 drops it). `count` holds the weights for 0, 1, 2 and 3+
    /// constituents, `None` meaning one or two with equal chance (the default
    /// lane's). `amounts` is how many amount terms the vocabulary has, which
    /// bounds the count.
    ///
    /// Callers have checked the names, the shares and the weights.
    pub fn new(shares: &[(&str, f64)], count: Option<[f64; 4]>, amounts: usize) -> Mix {
        let mut kept: Vec<(&str, f64)> = shares.iter().copied().filter(|(_, s)| *s > 0.0).collect();
        kept.sort_by(|a, b| b.1.total_cmp(&a.1));
        let cap = MAX_COUNT.min(amounts).min(kept.len());

        let w = count.unwrap_or([0.0, 1.0, 1.0, 0.0]);
        let total: f64 = w.iter().sum();
        let raw = [w[0], w[1], w[2], w[3] / 3.0, w[3] / 3.0, w[3] / 3.0];
        let mut count = [0.0; MAX_COUNT + 1];
        for (k, p) in raw.iter().enumerate() {
            count[k.min(cap)] += p / total;
        }

        let names = kept.iter().map(|(n, _)| (*n).to_string()).collect();
        let shares: Vec<f64> = kept.iter().map(|(_, s)| *s).collect();
        let mut mix = Mix {
            names,
            tilt: shares.iter().map(|s| s / (1.0 - s.min(0.999))).collect(),
            shares,
            count,
        };
        mix.fit();
        mix
    }

    /// `P(constituent i is in a set of n)` under the current tilt.
    fn inclusion_given(&self, n: usize) -> Vec<f64> {
        if n == 0 {
            return vec![0.0; self.tilt.len()];
        }
        let whole = elementary(self.tilt.iter().copied(), n)[n];
        (0..self.tilt.len())
            .map(|i| {
                let rest = self.tilt.iter().enumerate().filter(|(j, _)| *j != i);
                self.tilt[i] * elementary(rest.map(|(_, w)| *w), n - 1)[n - 1] / whole
            })
            .collect()
    }

    /// The fraction of strata that mention each constituent, in
    /// [`Mix::names`] order.
    pub fn expected_shares(&self) -> Vec<f64> {
        let mut out = vec![0.0; self.tilt.len()];
        for (n, p) in self.count.iter().enumerate().filter(|(_, p)| **p > 0.0) {
            for (o, q) in out.iter_mut().zip(self.inclusion_given(n)) {
                *o += p * q;
            }
        }
        out
    }

    fn fit(&mut self) {
        for _ in 0..300 {
            let got = self.expected_shares();
            for ((w, s), g) in self.tilt.iter_mut().zip(&self.shares).zip(got) {
                if g > 0.0 {
                    *w = (*w * (s / g).powf(0.7)).clamp(1e-9, 1e9);
                }
            }
            let top = self.tilt.iter().copied().fold(0.0, f64::max);
            for w in &mut self.tilt {
                *w /= top;
            }
        }
    }

    /// A set of `n` distinct indices by conditional Poisson sampling.
    fn pick(&self, rng: &mut Rng, n: usize) -> Vec<usize> {
        let len = self.tilt.len();
        // suffix[i][k] = e_k of the tilts from i on.
        let mut suffix = vec![vec![0.0; n + 1]; len + 1];
        suffix[len][0] = 1.0;
        for i in (0..len).rev() {
            suffix[i][0] = 1.0;
            for k in 1..=n {
                suffix[i][k] = suffix[i + 1][k] + self.tilt[i] * suffix[i + 1][k - 1];
            }
        }
        let mut out = Vec::new();
        for i in 0..len {
            let left = n - out.len();
            if left == 0 {
                break;
            }
            let p = self.tilt[i] * suffix[i + 1][left - 1] / suffix[i][left];
            // Draw even when forced, so the stream does not depend on rounding.
            let hit = unit_float(rng) < p;
            if hit || len - i == left {
                out.push(i);
            }
        }
        out
    }
}

/// A float in `[0, 1)` from 53 random bits.
#[allow(clippy::cast_precision_loss)]
fn unit_float(rng: &mut Rng) -> f64 {
    (rng.below(1 << 53) as f64) / ((1u64 << 53) as f64)
}

/// Made ground drawn by a preset's [`Mix`]: the count first, then that many
/// constituents, the most shared taking the most frequent amount. A count of
/// 0 leaves the tagged matrix with no `with` clause. `rng` is a stream of its
/// own, as for [`made_ground`].
// `below(n)` returns a value < n, and every n here is a small `Vec::len()`
// widened to u64, so the narrowing back to usize is lossless.
#[allow(clippy::cast_possible_truncation)]
pub fn made_ground_mix(
    matrix: &SoilDescription,
    vocab: &Vocab,
    rng: &mut Rng,
    mix: &Mix,
) -> MadeGround {
    let amounts = vocab.tertiary_amounts();
    let u = unit_float(rng);
    let mut acc = 0.0;
    let n = mix
        .count
        .iter()
        .position(|p| {
            acc += p;
            u < acc
        })
        .unwrap_or_else(|| mix.count.iter().rposition(|p| *p > 0.0).unwrap_or(0));
    let chosen = mix.pick(rng, n);

    let mut idx: Vec<usize> = Vec::new();
    while idx.len() < chosen.len() {
        let i = rng.below(amounts.len() as u64) as usize;
        if !idx.contains(&i) {
            idx.push(i);
        }
    }
    idx.sort_unstable_by(|a, b| b.cmp(a));

    let parts = chosen
        .into_iter()
        .zip(idx)
        .map(|(c, i)| Constituent {
            amount: amounts[i].clone(),
            name: mix.names[c].clone(),
        })
        .collect();
    compose(matrix, parts)
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

    fn shares() -> Vec<(&'static str, f64)> {
        vec![
            ("brick", 0.6),
            ("concrete", 0.5),
            ("ash", 0.3),
            ("pottery", 0.2),
        ]
    }

    /// Mean count about 1.6 (the `3+` bucket is capped by the four names),
    /// which is the shares' sum.
    const COUNT: [f64; 4] = [0.2, 0.3, 0.3, 0.2];

    fn mix_draw(seed: u64, mix: &Mix) -> MadeGround {
        let m = describe_with(vocab(), &mut Rng::seeded(seed), Lanes::STANDARD);
        made_ground_mix(&m, vocab(), &mut Rng::seeded(seed ^ SALT), mix)
    }

    #[test]
    fn the_extended_words_join_the_vocabulary_but_not_the_default_candidates() {
        let c = constituents();
        for w in ["pottery", "coal"] {
            assert!(c.vocabulary().any(|x| x == w), "{w}");
            assert!(!c.constituents.iter().any(|x| x == w), "{w}");
        }
        assert_eq!(c.constituents.len(), 10);
        assert_eq!(c.constituents[0], "brick");
        assert_eq!(c.constituents[9], "plastic");
    }

    #[test]
    fn an_extended_word_repeating_a_default_one_is_refused() {
        let mut c = ConstituentList::load();
        c.extended.push("brick".into());
        assert!(c.check().is_err());
    }

    #[test]
    fn the_default_draw_never_names_an_extended_word() {
        let c = constituents();
        for seed in 0..500 {
            for k in draw(seed, Lanes::ALL).constituents {
                assert!(!c.extended.contains(&k.name), "seed {seed}");
            }
        }
    }

    #[test]
    fn the_fit_reproduces_the_shares_when_they_agree_with_the_count() {
        let mix = Mix::new(&shares(), Some(COUNT), 5);
        for (got, (name, want)) in mix.expected_shares().iter().zip(shares()) {
            assert!((got - want).abs() < 0.02, "{name}: {got} vs {want}");
        }
    }

    #[test]
    #[allow(clippy::cast_precision_loss)]
    fn drawn_mention_shares_and_counts_follow_the_weights() {
        let mix = Mix::new(&shares(), Some(COUNT), 5);
        let n = 6000;
        let mut mentions = [0usize; 4];
        let mut counts = [0usize; 4];
        for seed in 0..n {
            let g = mix_draw(seed, &mix);
            counts[g.constituents.len().min(3)] += 1;
            assert!(g.description.text.starts_with(TAG));
            for k in &g.constituents {
                let i = shares().iter().position(|(s, _)| *s == k.name).unwrap();
                mentions[i] += 1;
            }
        }
        let n = n as f64;
        for (i, (name, want)) in shares().into_iter().enumerate() {
            let got = mentions[i] as f64 / n;
            assert!((got - want).abs() < 0.03, "{name}: {got} vs {want}");
        }
        for (k, want) in COUNT.iter().enumerate() {
            let got = counts[k] as f64 / n;
            assert!((got - want).abs() < 0.03, "count {k}: {got} vs {want}");
        }
    }

    #[test]
    fn a_count_of_zero_has_the_tag_and_no_with_clause() {
        let mix = Mix::new(&shares(), Some([1.0, 0.0, 0.0, 0.0]), 5);
        for seed in 0..50 {
            let g = mix_draw(seed, &mix);
            let t = &g.description.text;
            assert!(g.constituents.is_empty());
            assert!(t.starts_with(TAG), "{t}");
            assert!(!t.contains(" with "), "{t}");
            assert!(!t.contains("fragments"), "{t}");
            assert!(t.ends_with('.') || t.contains(". "), "{t}");
        }
    }

    #[test]
    fn three_or_more_constituents_read_as_a_list_in_decreasing_frequency() {
        let mix = Mix::new(&shares(), Some([0.0, 0.0, 0.0, 1.0]), 5);
        let amounts = vocab().tertiary_amounts();
        for seed in 0..100 {
            let g = mix_draw(seed, &mix);
            let t = &g.description.text;
            assert!((3..=4).contains(&g.constituents.len()), "{t}");
            assert_eq!(t.matches(" and ").count(), 1, "{t}");
            assert!(t.contains(", "), "{t}");
            let idx: Vec<usize> = g
                .constituents
                .iter()
                .map(|k| amounts.iter().position(|a| *a == k.amount).unwrap())
                .collect();
            assert!(idx.windows(2).all(|w| w[0] > w[1]), "{t}");
        }
    }

    #[test]
    fn a_mix_is_deterministic_per_seed() {
        let mix = Mix::new(&shares(), Some(COUNT), 5);
        assert_eq!(
            mix_draw(3, &mix).description.text,
            mix_draw(3, &mix).description.text
        );
    }
}
