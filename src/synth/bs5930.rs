//! A constraint-valid **BS 5930:2015+A1:2020 Section 6** soil-description
//! generator — the realistic `GEOL_DESC` engine.
//!
//! The open vocabularies (strength/consistency, relative density, colour,
//! particle angularity, size sub-bands) are parsed from the vendored skill
//! data under `data/bs5930/` — see that dir's `PROVENANCE.md` — so they
//! stay synced to that vendored first-party source. The *constraint
//! engine* (the secondary-constituent proportion bands of Tables 16/17,
//! the cumulative-≤100% rule, the silty/clayey mutual exclusion, the
//! coarse-then-fine word order, the colour lightness·chroma·hue order) is
//! encoded here in Rust, citing `proportion-rules.json`.
//!
//! The variety the owner asked for is **combinatorial**, not a finite list
//! of canned strings: principal × strength/density × (lightness × chroma ×
//! hue) × secondary constituents × proportion bands gives millions of
//! distinct, *standard-compliant* descriptions. The percentages are drawn
//! first, respecting the cumulative rule **by construction**, then mapped
//! to their band qualifier — so a generated description can never say the
//! impossible "very sandy very gravelly CLAY".
//!
//! Scope (v1): natural inorganic coarse (SAND/GRAVEL) and fine (SILT/CLAY)
//! soils — the two lanes that share the standard word order. Very coarse
//! (BOULDERS/COBBLES), peat/organic and anthropogenic ground use different
//! apportionment/word orders and are left for later (as the source skill
//! itself stages them).

use laterite_ags4_parity::Rng;
use serde::Deserialize;

// Vendored skill data (data/bs5930/PROVENANCE.md). Parsed once into `Vocab`.
const TERMS_JSON: &str = include_str!("../../data/bs5930/terms.json");
const PARTICLE_SIZES_JSON: &str = include_str!("../../data/bs5930/particle-sizes.json");

/// Hues with no chroma form (BS 5930 Table 7 note): a chroma word like
/// "reddish" must never precede these.
const ACHROMATIC_HUES: &[&str] = &["white", "grey", "black", "cream"];

/// The hues a peat actually takes — clause 33.4.6 describes it as "usually dark
/// brown or black". Constraining the draw to these is exposing the existing
/// palette under a constraint, not authoring new vocabulary.
const PEAT_HUES: &[&str] = &["brown", "black", "grey"];

/// A `terms.json` entry — only the fields the generator reads (serde
/// ignores the rest).
#[derive(Deserialize)]
struct Term {
    #[serde(rename = "term")]
    text: String,
    #[serde(default)]
    group: Option<String>,
    #[serde(default, rename = "cuKpa")]
    cu_kpa: Option<String>,
}

#[derive(Deserialize)]
struct Terms {
    #[serde(rename = "strengthConsistency")]
    strength_consistency: Vec<Term>,
    #[serde(rename = "relativeDensity")]
    relative_density: Vec<Term>,
    colour: Vec<Term>,
    angularity: Vec<Term>,
    // The organic/peat lane's vocabularies. Already vendored — the v1 scope
    // note staged the terms and left the lane unbuilt, so nothing here is
    // newly authored.
    #[serde(default, rename = "peatTypes")]
    peat_types: Vec<Term>,
    #[serde(default, rename = "peatCondition")]
    peat_condition: Vec<Term>,
    #[serde(default, rename = "secondaryOrganic")]
    secondary_organic: Vec<OrganicTerm>,
    #[serde(default, rename = "tertiaryAmounts")]
    tertiary_amounts: Vec<Term>,
}

/// A `secondaryOrganic` entry — Table 20 pairs each amount with the colour
/// that typically accompanies it, and the pairing is the point: "Very organic"
/// reads wrong against a pale hue.
#[derive(Deserialize)]
struct OrganicTerm {
    term: String,
    #[serde(rename = "typicalColour")]
    typical_colour: String,
}

#[derive(Deserialize)]
struct Subdiv {
    name: String,
}
#[derive(Deserialize)]
struct Fraction {
    name: String,
    #[serde(default)]
    subdivisions: Vec<Subdiv>,
}
#[derive(Deserialize)]
struct ParticleSizes {
    fractions: Vec<Fraction>,
}

/// The parsed BS 5930 vocabularies (built once via [`Vocab::load`]).
pub struct Vocab {
    /// Table-8 hand-test consistency terms (Very soft … Very stiff) — the
    /// leading word for a fine soil. (The cu-banded Table-9 terms are
    /// excluded.)
    consistency: Vec<String>,
    /// Relative-density terms (Very loose … Very dense) — the leading word
    /// for a coarse soil.
    density: Vec<String>,
    lightness: Vec<String>,
    chroma: Vec<String>,
    hue: Vec<String>,
    /// Particle angularity (subangular …) — used in the grading sentence.
    angularity: Vec<String>,
    /// SAND/GRAVEL size sub-bands (fine/medium/coarse).
    grading: Vec<String>,
    /// Table 21 peat condition (Firm/Spongy/Plastic) — the leading word for a
    /// peat, standing where consistency or density stands in the other lanes.
    peat_condition: Vec<String>,
    /// Clause 33.4.6 peat types (PEAT, Fibrous PEAT, …). The fibre descriptor
    /// is part of the term, not a separate qualifier.
    peat_types: Vec<String>,
    /// Table 20 organic amounts, each with its typical colour.
    secondary_organic: Vec<(String, String)>,
    /// Vendored `tertiaryAmounts` (`with rare` … `with abundant`), rarest
    /// first.
    tertiary_amounts: Vec<String>,
}

impl Vocab {
    /// The Table-8 consistency terms, in order (Very soft … Very stiff).
    #[cfg(test)]
    pub fn consistency_terms(&self) -> &[String] {
        &self.consistency
    }

    /// The vendored tertiary amount terms, rarest first.
    pub(crate) fn tertiary_amounts(&self) -> &[String] {
        &self.tertiary_amounts
    }

    /// Parse the vendored skill data. Panics only if the in-repo JSON is
    /// malformed (a build-time guarantee, asserted by the tests).
    pub fn load() -> Vocab {
        let t: Terms = serde_json::from_str(TERMS_JSON).expect("vendored terms.json is valid");
        let ps: ParticleSizes = serde_json::from_str(PARTICLE_SIZES_JSON)
            .expect("vendored particle-sizes.json is valid");
        let by_group = |g: &str| -> Vec<String> {
            t.colour
                .iter()
                .filter(|c| c.group.as_deref() == Some(g))
                .map(|c| c.text.clone())
                .collect()
        };
        Vocab {
            // The Table-8 consistency terms carry no cu band.
            consistency: t
                .strength_consistency
                .iter()
                .filter(|x| x.cu_kpa.is_none())
                .map(|x| x.text.clone())
                .collect(),
            density: t.relative_density.iter().map(|x| x.text.clone()).collect(),
            lightness: by_group("lightness"),
            chroma: by_group("chroma"),
            hue: by_group("hue"),
            angularity: t.angularity.iter().map(|x| x.text.clone()).collect(),
            grading: ps
                .fractions
                .iter()
                .find(|f| f.name == "SAND")
                .map(|f| f.subdivisions.iter().map(|s| s.name.clone()).collect())
                .unwrap_or_default(),
            peat_condition: t.peat_condition.iter().map(|x| x.text.clone()).collect(),
            peat_types: t.peat_types.iter().map(|x| x.text.clone()).collect(),
            secondary_organic: t
                .secondary_organic
                .iter()
                .map(|x| (x.term.clone(), x.typical_colour.clone()))
                .collect(),
            tertiary_amounts: t.tertiary_amounts.iter().map(|x| x.text.clone()).collect(),
        }
    }
}

/// Which lane of the standard word order a description follows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrincipalClass {
    /// SAND / GRAVEL — leading term is relative density.
    Coarse,
    /// SILT / CLAY — leading term is consistency.
    Fine,
    /// PEAT — leading term is the Table 21 condition, and the fibre descriptor
    /// rides on the principal itself ("Pseudo-fibrous PEAT"), so this lane does
    /// not share the other two's word order. Clause 33.4.6.
    Peat,
}

/// Which description lanes `describe_with` may draw from.
///
/// **`STANDARD` is not merely the default — it is a byte-compatibility
/// contract.** The organic lane needs a third branch where the engine has a
/// coarse/fine coin flip, and changing that draw shifts every draw after it,
/// so every seed would yield a different file. Nothing would fail:
/// `synth_is_deterministic_per_seed_and_varies_across_seeds` compares one
/// binary against itself and `scaffolds_reach_a_pinned_set_of_ags_types` reads
/// only TYPE rows, while a description lands in a `GEOL_DESC` cell typed `X`
/// either way. What would break is comparability — `gen-bench-fixtures.sh` and
/// the two bench scripts draw their fixtures from this engine.
///
/// So `STANDARD` runs the ORIGINAL two-way branch, not a three-way branch with
/// one arm disabled. `synth_output_bytes_are_pinned` is what holds that, and it
/// was verified to redden against a simulated extra draw.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Lanes {
    /// Draw organic soils and peat as well as the natural inorganic lanes.
    pub organic: bool,
}

impl Lanes {
    /// The v1 lanes: natural inorganic coarse and fine. Draws exactly as the
    /// engine drew before the organic lane existed.
    pub const STANDARD: Lanes = Lanes { organic: false };
    /// Every lane, including organic soils and peat.
    pub const ALL: Lanes = Lanes { organic: true };
}

/// One secondary constituent, with the field-estimate percentage it was
/// drawn at and the band qualifier that percentage maps to.
#[derive(Debug, Clone)]
pub struct Secondary {
    /// Lowercase fraction adjective stem (`sand`/`gravel`/`silt`/`clay`).
    pub soil: &'static str,
    /// Visual field-estimate percentage by mass (excl. cobbles/boulders);
    /// `0` for a qualitative fine-in-fine secondary.
    pub pct: u32,
    /// `slightly` / "" (plain) / `very`; empty for a qualitative fine-in-fine.
    pub qualifier: &'static str,
}

impl Secondary {
    /// Whether this is a fine fraction (silt/clay) vs a coarse one
    /// (sand/gravel) — derived from `soil`.
    pub fn is_fine(&self) -> bool {
        matches!(self.soil, "silt" | "clay")
    }

    /// The adjective as it appears in the description (`very gravelly`,
    /// `slightly clayey`, `silty`).
    fn adjective(&self) -> String {
        let stem = match self.soil {
            "sand" => "sandy",
            "gravel" => "gravelly",
            "silt" => "silty",
            "clay" => "clayey",
            other => other,
        };
        if self.qualifier.is_empty() {
            stem.to_string()
        } else {
            format!("{} {stem}", self.qualifier)
        }
    }
}

/// A generated, standard-compliant soil description: the assembled text
/// plus the structured fields the constraint tests assert on.
#[derive(Debug, Clone)]
pub struct SoilDescription {
    pub text: String,
    /// The leading term — a consistency, a relative density, or a peat
    /// condition. Carried rather than recovered from `text`, because it is one
    /// word in "Firm" and two in "Very dense", and a lane that rebuilt the
    /// sentence by splitting on the first space silently dropped half of it.
    pub lead: String,
    pub principal: &'static str,
    pub principal_class: PrincipalClass,
    /// Coarse secondaries first, then any fine secondary (file order).
    pub secondaries: Vec<Secondary>,
}

/// Tables 16/17 for a **coarse** principal (SAND/GRAVEL): a secondary —
/// coarse or fine — bands at `<5` / `5–20` / `>20`.
fn band_coarse_principal(pct: u32) -> &'static str {
    if pct < 5 {
        "slightly"
    } else if pct <= 20 {
        ""
    } else {
        "very"
    }
}

/// Table 17 'Coarse soil' column for a **fine** principal (SILT/CLAY): a
/// coarse secondary bands at `<35` / `35–65` / `>65`.
fn band_fine_principal(pct: u32) -> &'static str {
    if pct < 35 {
        "slightly"
    } else if pct <= 65 {
        ""
    } else {
        "very"
    }
}

/// The base hue a chroma word modifies (`greenish` → `green`), so a chroma
/// is never paired with its own hue ("greenish green").
fn chroma_base(chroma: &str) -> &str {
    match chroma {
        "pinkish" => "pink",
        "reddish" => "red",
        "orangeish" => "orange",
        "yellowish" => "yellow",
        "brownish" => "brown",
        "greenish" => "green",
        "bluish" => "blue",
        "greyish" => "grey",
        other => other,
    }
}

/// Assemble a colour per BS 5930 Table 7 word order — `lightness? chroma?
/// hue`. A chroma word is only emitted before a hue that has a chroma form
/// (not white/grey/black/cream) and is a *different* colour family;
/// lightness is skipped for the lightness extremes themselves (no
/// "light black" / "dark white").
fn colour(vocab: &Vocab, rng: &mut Rng) -> String {
    let hue = rng.choose(&vocab.hue).clone();
    let mut parts: Vec<String> = Vec::new();
    // ~40% carry a lightness, except on black/white (contradictory).
    if !vocab.lightness.is_empty() && !matches!(hue.as_str(), "black" | "white") && rng.below(5) < 2
    {
        parts.push(rng.choose(&vocab.lightness).clone());
    }
    // Chroma only for chromatic hues, and only from a different family.
    if !ACHROMATIC_HUES.contains(&hue.as_str()) && rng.below(2) == 0 {
        let choices: Vec<String> = vocab
            .chroma
            .iter()
            .filter(|c| chroma_base(c) != hue)
            .cloned()
            .collect();
        if !choices.is_empty() {
            parts.push(rng.choose(&choices).clone());
        }
    }
    parts.push(hue);
    parts.join(" ")
}

/// `true` with probability `pct/100`.
fn chance(rng: &mut Rng, pct: u64) -> bool {
    rng.below(100) < pct
}

/// Describe a coarse soil (SAND/GRAVEL). Secondaries (the other coarse
/// fraction + an optional fine fraction) are kept to ≤45% total so the
/// principal stays the dominant fraction (cumulative ≤100% by construction).
// `rng.range(1, hi) as u32` below always returns a value in `[1, hi]`, and
// `hi` is `budget.min(40)` — so the result is always in `1..=40`, comfortably
// inside u32.
#[allow(clippy::cast_possible_truncation)]
fn describe_coarse(vocab: &Vocab, rng: &mut Rng) -> SoilDescription {
    let coarse_pair = [("SAND", "gravel"), ("GRAVEL", "sand")];
    let (principal, other_coarse) = *rng.choose(&coarse_pair);

    let mut secondaries = Vec::new();
    let mut budget = 45u32; // headroom for both secondaries
    // Coarse secondary (the other coarse fraction).
    if chance(rng, 60) {
        let pct = rng.range(1, i64::from(budget.min(40))) as u32;
        budget -= pct.min(budget);
        secondaries.push(Secondary {
            soil: other_coarse,
            pct,
            qualifier: band_coarse_principal(pct),
        });
    }
    // Fine secondary — silt OR clay, never both (mutual exclusion).
    if budget >= 1 && chance(rng, 55) {
        let fine = *rng.choose(&["silt", "clay"]);
        let pct = rng.range(1, i64::from(budget.min(40))) as u32;
        secondaries.push(Secondary {
            soil: fine,
            pct,
            qualifier: band_coarse_principal(pct),
        });
    }

    let density = rng.choose(&vocab.density).clone();
    let col = colour(vocab, rng);
    let mut words = vec![density.clone(), col];
    // Coarse-then-fine order is already the push order above.
    for s in &secondaries {
        words.push(s.adjective());
    }
    words.push(principal.to_string());
    let mut text = words.join(" ");
    text.push('.');
    // Optional grading sentence for the coarse fraction.
    if !vocab.grading.is_empty() && !vocab.angularity.is_empty() && chance(rng, 50) {
        text.push_str(&grading_sentence(principal, vocab, rng));
    }

    SoilDescription {
        text,
        lead: density,
        principal,
        principal_class: PrincipalClass::Coarse,
        secondaries,
    }
}

/// Describe a fine soil (SILT/CLAY). The coarse secondaries (sandy and/or
/// gravelly) are assessed **separately**, each against Table 17, but their
/// combined percentage is held ≤80% so the cumulative-≤100% rule holds and
/// "very + very" is structurally impossible. A fine-in-fine secondary
/// (silty CLAY / clayey SILT) is qualitative — no percentage, no qualifier.
// `rng.range(1, budget.min(72)) as u32` below always returns a value in
// `[1, 72]` (the `budget >= 1` guard rules out an empty/invalid range),
// comfortably inside u32.
#[allow(clippy::cast_possible_truncation)]
fn describe_fine(vocab: &Vocab, rng: &mut Rng) -> SoilDescription {
    let fine_pair = [("CLAY", "silt"), ("SILT", "clay")];
    let (principal, other_fine) = *rng.choose(&fine_pair);

    // Draw the two coarse secondaries with a shared ≤80% budget.
    let mut coarse: Vec<Secondary> = Vec::new();
    let mut budget = 80i64;
    let sand_first = rng.below(2) == 0;
    let order: [&'static str; 2] = if sand_first {
        ["sand", "gravel"]
    } else {
        ["gravel", "sand"]
    };
    for soil in order {
        if budget >= 1 && chance(rng, 55) {
            let pct = rng.range(1, budget.min(72)) as u32;
            budget -= i64::from(pct);
            coarse.push(Secondary {
                soil,
                pct,
                qualifier: band_fine_principal(pct),
            });
        }
    }
    // Order the coarse secondaries by increasing proportion (word-order rule).
    coarse.sort_by_key(|s| s.pct);

    let mut secondaries = coarse;
    // Optional fine-in-fine (qualitative; mutually exclusive by construction).
    if chance(rng, 35) {
        secondaries.push(Secondary {
            soil: other_fine,
            pct: 0,
            qualifier: "",
        });
    }

    let consistency = rng.choose(&vocab.consistency).clone();
    let col = colour(vocab, rng);
    let mut words = vec![consistency.clone(), col];
    for s in &secondaries {
        words.push(s.adjective());
    }
    words.push(principal.to_string());
    let mut text = words.join(" ");
    text.push('.');

    SoilDescription {
        text,
        lead: consistency,
        principal,
        principal_class: PrincipalClass::Fine,
        secondaries,
    }
}

/// A trailing grading sentence, e.g. " Gravel is subangular fine to medium."
// `below(n)` returns a value < n by construction, and every n here is a
// `Vec::len()` widened to u64, so each narrowing back to usize is lossless.
#[allow(clippy::cast_possible_truncation)]
fn grading_sentence(principal: &str, vocab: &Vocab, rng: &mut Rng) -> String {
    let ang = rng.choose(&vocab.angularity).clone();
    // "fine", "fine to medium", or "fine to coarse" style spans.
    let lo = rng.below(vocab.grading.len() as u64) as usize;
    let hi =
        (lo + rng.below((vocab.grading.len() - lo) as u64) as usize).min(vocab.grading.len() - 1);
    let sizes = if lo == hi {
        vocab.grading[lo].clone()
    } else {
        format!("{} to {}", vocab.grading[lo], vocab.grading[hi])
    };
    let fraction = if principal == "GRAVEL" {
        "Gravel"
    } else {
        "Sand"
    };
    format!(" {fraction} is {ang} {sizes}.")
}

/// The process-wide vocabulary, parsed once from the vendored JSON — so a
/// strata-heavy synthesis (many `describe` calls) pays the parse cost a
/// single time.
pub fn vocab() -> &'static Vocab {
    static V: std::sync::OnceLock<Vocab> = std::sync::OnceLock::new();
    V.get_or_init(Vocab::load)
}

/// An organic soil: a fine or coarse principal carrying a Table 20 organic
/// amount in the qualifier slot, e.g. `Very soft dark grey slightly sandy
/// organic CLAY.`
///
/// The amount is drawn WITH its typical colour rather than against the general
/// palette — Table 20 pairs them, and "Very organic" against a pale hue reads
/// wrong to anyone who logs soil. That pairing is the whole reason the vendored
/// data carries `typicalColour` alongside the term.
fn describe_organic(vocab: &Vocab, rng: &mut Rng) -> SoilDescription {
    let mut d = if rng.below(2) == 0 {
        describe_coarse(vocab, rng)
    } else {
        describe_fine(vocab, rng)
    };

    let (amount, colour) = rng.choose(&vocab.secondary_organic).clone();
    // Rebuild rather than patch the string: the organic qualifier sits
    // immediately before the principal, and the colour Table 20 pairs with the
    // amount replaces the drawn one, so the word order stays the standard one.
    let mut words = vec![d.lead.clone(), colour.to_lowercase()];
    for s in &d.secondaries {
        words.push(s.adjective());
    }
    words.push(amount.to_lowercase());
    words.push(d.principal.to_string());
    d.text = words.join(" ");
    d.text.push('.');
    d
}

/// A peat: `Firm dark brown pseudo-fibrous PEAT.`
///
/// Its own lane rather than a principal in the fine one — the leading word is
/// the Table 21 condition, not a consistency, and the fibre descriptor is part
/// of the clause-33.4.6 term rather than a separate qualifier. Peat takes no
/// secondaries here.
///
/// The von Post humification scale is deliberately absent: the vendored data
/// carries `vonPost` as an EMPTY list with a note recording that BS 5930
/// Section 6 does not print it and only references it as an external scheme.
/// Inventing those terms would be authoring vocabulary rather than exposing it.
fn describe_peat(vocab: &Vocab, rng: &mut Rng) -> SoilDescription {
    let condition = rng.choose(&vocab.peat_condition).clone();
    // The hues clause 33.4.6 names, drawn from the same vocabulary the other
    // lanes use rather than a new one — "usually dark brown or black".
    let peat_hues: Vec<String> = vocab
        .hue
        .iter()
        .filter(|h| PEAT_HUES.contains(&h.as_str()))
        .cloned()
        .collect();
    let hue = rng.choose(&peat_hues).clone();
    let col = if rng.below(2) == 0 {
        format!("dark {hue}")
    } else {
        hue
    };
    let kind = rng.choose(&vocab.peat_types).clone();
    // "Fibrous PEAT" carries its own principal; bare "PEAT" needs none.
    let mut text = format!("{condition} {col} {}", kind.to_lowercase());
    // Restore the principal's capitalisation — it is PEAT in every term.
    text = text.replace("peat", "PEAT");
    text.push('.');

    SoilDescription {
        text,
        lead: condition,
        principal: "PEAT",
        principal_class: PrincipalClass::Peat,
        secondaries: Vec::new(),
    }
}

/// Generate one constraint-valid BS 5930 soil description for `rng`, using the
/// v1 lanes only. Byte-compatible with every seed drawn before the organic
/// lane existed — see [`Lanes`].
pub fn describe(vocab: &Vocab, rng: &mut Rng) -> SoilDescription {
    describe_with(vocab, rng, Lanes::STANDARD)
}

/// Generate one description, drawing from `lanes`.
///
/// The two-way branch under `STANDARD` is the ORIGINAL code path, not a
/// three-way branch with an arm disabled — see [`Lanes`] for why that
/// distinction is the whole contract.
pub fn describe_with(vocab: &Vocab, rng: &mut Rng, lanes: Lanes) -> SoilDescription {
    if !lanes.organic {
        return if rng.below(2) == 0 {
            describe_coarse(vocab, rng)
        } else {
            describe_fine(vocab, rng)
        };
    }
    // Organic ground is a minority of most profiles, so the two natural lanes
    // keep the bulk of the draw rather than each lane taking a third.
    match rng.below(4) {
        0 => describe_coarse(vocab, rng),
        1 => describe_fine(vocab, rng),
        2 => describe_organic(vocab, rng),
        _ => describe_peat(vocab, rng),
    }
}

/// What a site-model unit allows a description to say, so every stratum of the
/// unit reads as the same soil. Owned strings, so preset data maps straight on.
///
/// Validate with [`UnitDescription::problems`]; [`describe_unit`] refuses a
/// constraint that has any.
// Not called from any command yet; niko86/laterite-forge#21 wires the site
// model in.
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnitDescription {
    /// Allowed principal soils: `SAND`, `GRAVEL`, `SILT` or `CLAY`.
    pub principals: Vec<String>,
    /// Allowed secondary fractions (`sand`, `gravel`, `silt`, `clay`); empty
    /// means the description carries none.
    pub secondaries: Vec<String>,
    /// Whole colour phrases, each used verbatim (`"dark grey"`).
    pub colours: Vec<String>,
}

/// The four principals a unit may name, with the lowercase fraction each is
/// made of and the lane it draws in.
const UNIT_PRINCIPALS: [(&str, &str, PrincipalClass); 4] = [
    ("SAND", "sand", PrincipalClass::Coarse),
    ("GRAVEL", "gravel", PrincipalClass::Coarse),
    ("SILT", "silt", PrincipalClass::Fine),
    ("CLAY", "clay", PrincipalClass::Fine),
];

const UNIT_SECONDARIES: [&str; 4] = ["sand", "gravel", "silt", "clay"];

/// Why `phrase` is not a colour the engine could compose, or `None` when it is
/// one: `lightness? chroma? hue` (`colourWordOrder`), with the exclusions
/// [`colour`] observes (no chroma before white, black or cream or before its
/// own family, no lightness on black/white). Unlike [`colour`], a palette may
/// put a chroma before grey ("bluish grey"): BS 5930's "no chroma form exists
/// for grey" rules out a "-ish" form of grey, not a chroma in front of it.
#[allow(dead_code)]
fn colour_problem(vocab: &Vocab, phrase: &str) -> Option<String> {
    let bad = |why: &str| Some(format!("colour {phrase:?} {why}"));
    let mut words: Vec<&str> = phrase.split(' ').collect();
    let hue = words.pop().unwrap_or_default();
    if !vocab.hue.iter().any(|h| h == hue) {
        return bad("does not end in a hue");
    }
    if words
        .first()
        .is_some_and(|w| vocab.lightness.iter().any(|l| l == w))
    {
        if matches!(hue, "black" | "white") {
            return bad("puts a lightness on black or white");
        }
        words.remove(0);
    }
    if let Some(w) = words.first()
        && vocab.chroma.iter().any(|c| c == w)
    {
        if hue != "grey" && ACHROMATIC_HUES.contains(&hue) {
            return bad("puts a chroma before white, black or cream");
        }
        if chroma_base(w) == hue {
            return bad("repeats its own hue as a chroma");
        }
        words.remove(0);
    }
    if words.is_empty() {
        None
    } else {
        bad("is not in lightness, chroma, hue order")
    }
}

#[allow(dead_code)]
impl UnitDescription {
    /// Every problem in this constraint against `vocab`, empty when it is usable.
    pub fn problems(&self, vocab: &Vocab) -> Vec<String> {
        let mut out = Vec::new();
        if self.principals.is_empty() {
            out.push("no principals".to_string());
        }
        for p in &self.principals {
            if !UNIT_PRINCIPALS.iter().any(|(name, _, _)| name == p) {
                out.push(format!("unknown principal {p:?}"));
            }
        }
        for sec in &self.secondaries {
            if !UNIT_SECONDARIES.contains(&sec.as_str()) {
                out.push(format!("unknown secondary {sec:?}"));
            } else if !self.principals.is_empty()
                && self.principals.iter().all(|p| {
                    UNIT_PRINCIPALS
                        .iter()
                        .any(|(name, own, _)| name == p && own == sec)
                })
            {
                // Only a fraction no listed principal can carry is refused: the
                // draw skips the drawn principal's own fraction.
                out.push(format!(
                    "secondary {sec:?} is the own fraction of every listed principal"
                ));
            }
        }
        if self.colours.is_empty() {
            out.push("empty palette".to_string());
        }
        out.extend(self.colours.iter().filter_map(|c| colour_problem(vocab, c)));
        out
    }
}

/// The `'static` spelling of a validated fraction name.
fn fraction_static(soil: &str) -> &'static str {
    UNIT_SECONDARIES
        .into_iter()
        .find(|s| *s == soil)
        .expect("validated fraction")
}

/// The leading terms a caller supplies to [`describe_unit`], one per lane.
/// The unit draws its own principal, so the caller cannot know which lane
/// will be needed: it gives the term for each lane the unit can reach and the
/// draw uses the one matching the principal it drew.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Lead<'a> {
    /// A consistency term, used when the principal is SILT or CLAY.
    pub fine: Option<&'a str>,
    /// A relative-density term, used when the principal is SAND or GRAVEL.
    pub coarse: Option<&'a str>,
}

/// Draw one description inside `unit`'s constraints.
///
/// `lead` holds the leading term per lane, supplied by the caller: a lane left
/// `None` leaves the description without a term when that lane is drawn
/// (`SoilDescription::lead` is then empty). A `fine` term must be a consistency
/// term and a `coarse` one a relative-density term, whichever principal is
/// drawn. Errors only on a constraint [`UnitDescription::problems`] would
/// report, or a lead that is not a term of its lane.
///
/// A separate path from [`describe_with`]: it shares that path's band and
/// adjective helpers but draws nothing the pinned paths draw.
// `rng.range(1, hi) as u32`: the range is bounded by the budgets below (at most
// 72), well inside u32.
#[allow(clippy::cast_possible_truncation)]
pub fn describe_unit(
    vocab: &Vocab,
    rng: &mut Rng,
    unit: &UnitDescription,
    lead: Lead<'_>,
) -> Result<SoilDescription, String> {
    let problems = unit.problems(vocab);
    if !problems.is_empty() {
        return Err(problems.join("; "));
    }
    let name = rng.choose(&unit.principals);
    let (principal, own, class) = *UNIT_PRINCIPALS
        .iter()
        .find(|(n, _, _)| n == name)
        .expect("validated principal");
    for (term, lane, kind) in [
        (lead.fine, &vocab.consistency, "consistency"),
        (lead.coarse, &vocab.density, "relative-density"),
    ] {
        if let Some(term) = term
            && !lane.iter().any(|t| t == term)
        {
            return Err(format!("lead {term:?} is not a {kind} term"));
        }
    }
    let lead = if class == PrincipalClass::Fine {
        lead.fine
    } else {
        lead.coarse
    };
    let allowed = |soil: &str| unit.secondaries.iter().any(|s| s == soil);

    let mut secondaries: Vec<Secondary> = Vec::new();
    if class == PrincipalClass::Coarse {
        // Same shape as `describe_coarse`: the other coarse fraction, then one
        // fine fraction, inside a 45% budget.
        let other = if own == "sand" { "gravel" } else { "sand" };
        let mut budget = 45u32;
        if allowed(other) && chance(rng, 60) {
            let pct = rng.range(1, i64::from(budget.min(40))) as u32;
            budget -= pct.min(budget);
            secondaries.push(Secondary {
                soil: fraction_static(other),
                pct,
                qualifier: band_coarse_principal(pct),
            });
        }
        let fines: Vec<&'static str> = ["silt", "clay"]
            .into_iter()
            .filter(|f| allowed(f))
            .collect();
        if budget >= 1 && !fines.is_empty() && chance(rng, 55) {
            let pct = rng.range(1, i64::from(budget.min(40))) as u32;
            secondaries.push(Secondary {
                soil: rng.choose(&fines).to_owned(),
                pct,
                qualifier: band_coarse_principal(pct),
            });
        }
    } else {
        // Same shape as `describe_fine`: sand and gravel share an 80% budget
        // and read in increasing proportion, then a qualitative fine fraction.
        let mut budget = 80i64;
        let order: [&'static str; 2] = if rng.below(2) == 0 {
            ["sand", "gravel"]
        } else {
            ["gravel", "sand"]
        };
        for soil in order.into_iter().filter(|s| allowed(s)) {
            if budget >= 1 && chance(rng, 55) {
                let pct = rng.range(1, budget.min(72)) as u32;
                budget -= i64::from(pct);
                secondaries.push(Secondary {
                    soil,
                    pct,
                    qualifier: band_fine_principal(pct),
                });
            }
        }
        secondaries.sort_by_key(|s| s.pct);
        let other_fine = if own == "silt" { "clay" } else { "silt" };
        if allowed(other_fine) && chance(rng, 35) {
            secondaries.push(Secondary {
                soil: fraction_static(other_fine),
                pct: 0,
                qualifier: "",
            });
        }
    }

    let colour = rng.choose(&unit.colours).clone();
    let mut words: Vec<String> = Vec::new();
    if let Some(term) = lead {
        words.push(term.to_string());
    }
    words.push(colour);
    words.extend(secondaries.iter().map(Secondary::adjective));
    words.push(principal.to_string());
    let mut text = words.join(" ");
    if let Some(first) = text.get_mut(..1) {
        first.make_ascii_uppercase();
    }
    text.push('.');

    Ok(SoilDescription {
        text,
        lead: lead.unwrap_or_default().to_string(),
        principal,
        principal_class: class,
        secondaries,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fine(term: &str) -> Lead<'_> {
        Lead {
            fine: Some(term),
            coarse: None,
        }
    }

    fn coarse(term: &str) -> Lead<'_> {
        Lead {
            fine: None,
            coarse: Some(term),
        }
    }

    /// The two entry points agree: `describe` is `describe_with(STANDARD)`.
    ///
    /// This is a routing check, NOT the downstream contract — it compares the
    /// lane machinery against itself, so a change to the shared branch moves
    /// both sides together and leaves it green (verified: forcing `STANDARD`
    /// down the four-way branch reddens the byte pins and not this). What it
    /// does catch is `describe` being re-pointed at `ALL`. The contract that
    /// protects downstream consumers is
    /// `the_unflagged_draw_is_pinned_to_pre_lane_bytes`.
    #[test]
    fn standard_lanes_draw_exactly_as_the_bare_entry_point() {
        let v = Vocab::load();
        for seed in 0..40u64 {
            let a = describe(&v, &mut Rng::seeded(seed)).text;
            let b = describe_with(&v, &mut Rng::seeded(seed), Lanes::STANDARD).text;
            assert_eq!(a, b, "seed {seed}: STANDARD must be the original path");
        }
    }

    /// The **downstream contract**, pinned to bytes captured on `main` before
    /// the organic lane existed. `standard_lanes_draw_exactly_as_the_bare
    /// _entry_point` only proves the two entry points agree with each other —
    /// both would move together if the shared branch moved, and it would stay
    /// green. These hashes come from the other side of the change, so they are
    /// the assertion that the draw did not move at all.
    ///
    /// The sequence pinned is the one `forge describe --seed S --count N`
    /// walks: `Rng::seeded(S + i)` for `i` in `0..N`, texts joined by newline.
    /// That is the surface a downstream consumer reads, so a re-roll that this
    /// catches is a re-roll that would have invalidated their committed
    /// fixtures — see `docs/dec-forge-audience-boundary.md`.
    ///
    /// Raising these pins is legitimate only alongside deliberately re-rolling
    /// the default draw, which is a notify-downstream change, never a quiet one.
    #[test]
    fn the_unflagged_draw_is_pinned_to_pre_lane_bytes() {
        use sha2::{Digest, Sha256};
        let v = Vocab::load();
        for (seed, count, sha, len) in [
            (
                0u64,
                12u64,
                "8b84da7f9492c31ffef1778d9cd41fc0140c9874ed93c3496b55ddade3a0ef2b",
                751usize,
            ),
            (
                1,
                12,
                "5f596af9881dc3e748af4869d0d70e1d906f7c69c44410506469285ef3d11e80",
                748,
            ),
            (
                7,
                40,
                "76d886dc66c75bafbad0f7e763d4e492013edc2d44d23b2cfac0b6cdf5b733f7",
                1946,
            ),
        ] {
            let joined = (0..count)
                .map(|i| describe(&v, &mut Rng::seeded(seed.wrapping_add(i))).text)
                .collect::<Vec<_>>()
                .join("\n");
            let got = crate::synth::hex(&Sha256::digest(joined.as_bytes()));
            assert_eq!(
                (got.as_str(), joined.len()),
                (sha, len),
                "seed {seed} count {count}: the DEFAULT description draw moved. \
                 Downstream consumers commit output from this sequence, so this \
                 is a re-baseline for them, not just for us."
            );
        }
    }

    /// The lane is opt-in, so the default vocabulary must not reach it. This is
    /// the assertion niko86/laterite#697 exists to keep true — a downstream build that never
    /// passes the flag must never see organic ground appear.
    #[test]
    fn the_organic_lane_is_unreachable_without_the_flag() {
        let v = Vocab::load();
        for seed in 0..600u64 {
            let d = describe(&v, &mut Rng::seeded(seed));
            assert_ne!(d.principal, "PEAT", "seed {seed}: peat without the flag");
            assert_ne!(
                d.principal_class,
                PrincipalClass::Peat,
                "seed {seed}: peat class without the flag"
            );
            let lower = d.text.to_lowercase();
            assert!(
                !lower.contains("organic") && !lower.contains("peat"),
                "seed {seed}: organic vocabulary without the flag — {}",
                d.text
            );
        }
    }

    /// …and reachable with it. Both halves the issue asks for: an organic
    /// modifier on a natural soil, and PEAT as a principal in its own right.
    #[test]
    fn the_flag_reaches_both_organic_soils_and_peat() {
        let v = Vocab::load();
        let (mut organic, mut peat) = (0u32, 0u32);
        for seed in 0..600u64 {
            let d = describe_with(&v, &mut Rng::seeded(seed), Lanes::ALL);
            if d.principal == "PEAT" {
                peat += 1;
                assert_eq!(d.principal_class, PrincipalClass::Peat);
            } else if d.text.to_lowercase().contains("organic") {
                organic += 1;
                assert_ne!(
                    d.principal_class,
                    PrincipalClass::Peat,
                    "an organic modifier rides a natural principal"
                );
            }
        }
        assert!(organic > 0, "the flag reached no organic soil in 600 draws");
        assert!(peat > 0, "the flag reached no peat in 600 draws");
    }

    /// Peat does not share the other lanes' word order: the leading word is a
    /// Table 21 condition, not a consistency or a density.
    #[test]
    fn peat_leads_with_its_own_condition_vocabulary() {
        let v = Vocab::load();
        let mut seen = 0u32;
        for seed in 0..600u64 {
            let d = describe_with(&v, &mut Rng::seeded(seed), Lanes::ALL);
            if d.principal != "PEAT" {
                continue;
            }
            seen += 1;
            let lead = d.text.split(' ').next().unwrap_or_default();
            assert!(
                v.peat_condition.iter().any(|c| c == lead),
                "peat led with {lead:?}, which is not a Table 21 condition: {}",
                d.text
            );
            assert!(
                d.text.contains("PEAT"),
                "principal lost its case: {}",
                d.text
            );
            assert!(
                d.secondaries.is_empty(),
                "this lane draws no secondaries: {}",
                d.text
            );
        }
        assert!(seen > 0, "no peat drawn to check");
    }

    /// The von Post scale stays out. The vendored data records that BS 5930
    /// Section 6 does not print it, so emitting H1–H10 would be authoring
    /// vocabulary rather than exposing what was staged.
    #[test]
    fn no_von_post_humification_terms_are_invented() {
        let v = Vocab::load();
        for seed in 0..600u64 {
            let d = describe_with(&v, &mut Rng::seeded(seed), Lanes::ALL);
            for h in 1..=10 {
                assert!(
                    !d.text.contains(&format!("H{h}")),
                    "seed {seed}: von Post term H{h} in {}",
                    d.text
                );
            }
        }
    }

    fn vocab() -> Vocab {
        Vocab::load()
    }

    /// The vendored JSON parses and yields non-empty vocabularies — guards
    /// the data files + the serde shapes.
    #[test]
    fn vocab_loads_non_empty() {
        let v = vocab();
        assert!(!v.consistency.is_empty(), "consistency terms");
        assert!(!v.density.is_empty(), "density terms");
        assert!(!v.hue.is_empty() && !v.chroma.is_empty() && !v.lightness.is_empty());
        assert!(!v.angularity.is_empty() && !v.grading.is_empty());
        // The cu-banded Table-9 strength terms are excluded.
        assert!(v.consistency.iter().any(|t| t == "Firm"));
        assert!(!v.consistency.iter().any(|t| t.contains("strength")));
    }

    /// THE constraint: secondary fractions never sum past 100% (the
    /// cumulative rule, clause 33.4.4.5 NOTE 2) — across many seeds.
    #[test]
    fn cumulative_fractions_never_exceed_100() {
        let v = vocab();
        for seed in 0..500u64 {
            let mut rng = Rng::seeded(seed);
            let d = describe(&v, &mut rng);
            let sum: u32 = d.secondaries.iter().map(|s| s.pct).sum();
            assert!(
                sum <= 100,
                "seed {seed}: secondaries sum {sum}% > 100: {d:?}"
            );
        }
    }

    /// silty and clayey are mutually exclusive — never both present.
    #[test]
    fn fine_secondaries_are_mutually_exclusive() {
        let v = vocab();
        for seed in 0..500u64 {
            let mut rng = Rng::seeded(seed);
            let d = describe(&v, &mut rng);
            let fines = d.secondaries.iter().filter(|s| s.is_fine()).count();
            assert!(fines <= 1, "seed {seed}: {fines} fine secondaries: {d:?}");
        }
    }

    /// Every secondary's qualifier is exactly the band its percentage maps
    /// to for the principal's class — the percentage-first invariant.
    #[test]
    fn qualifier_matches_band() {
        let v = vocab();
        for seed in 0..500u64 {
            let mut rng = Rng::seeded(seed);
            let d = describe(&v, &mut rng);
            for s in &d.secondaries {
                if s.pct == 0 {
                    // qualitative fine-in-fine
                    assert!(s.qualifier.is_empty() && s.is_fine());
                    continue;
                }
                let expected = match d.principal_class {
                    // A fine principal bands its coarse secondaries on 35/65;
                    // a coarse principal bands everything on 5/20.
                    PrincipalClass::Fine if !s.is_fine() => band_fine_principal(s.pct),
                    _ => band_coarse_principal(s.pct),
                };
                assert_eq!(s.qualifier, expected, "seed {seed} {s:?} in {d:?}");
            }
        }
    }

    /// The impossible "very sandy very gravelly CLAY" can never be drawn
    /// (two coarse 'very' secondaries on a fine principal need >130%).
    #[test]
    fn no_double_very_coarse_on_a_fine_principal() {
        let v = vocab();
        for seed in 0..1000u64 {
            let mut rng = Rng::seeded(seed);
            let d = describe(&v, &mut rng);
            if d.principal_class == PrincipalClass::Fine {
                let verys = d
                    .secondaries
                    .iter()
                    .filter(|s| !s.is_fine() && s.qualifier == "very")
                    .count();
                assert!(
                    verys <= 1,
                    "seed {seed}: two 'very' coarse secondaries: {d:?}"
                );
            }
        }
    }

    /// Word order: coarse secondaries precede the fine secondary, and all
    /// secondaries precede the (uppercase) principal.
    #[test]
    fn word_order_coarse_then_fine_then_principal() {
        let v = vocab();
        for seed in 0..300u64 {
            let mut rng = Rng::seeded(seed);
            let d = describe(&v, &mut rng);
            assert_eq!(d.principal, d.principal.to_uppercase());
            let p = d.text.find(d.principal).expect("principal in text");
            let mut last_coarse = 0usize;
            let mut first_fine = usize::MAX;
            for s in &d.secondaries {
                let stem = match s.soil {
                    "sand" => "sandy",
                    "gravel" => "gravelly",
                    "silt" => "silty",
                    "clay" => "clayey",
                    o => o,
                };
                let at = d.text.find(stem).expect("secondary adjective in text");
                assert!(at < p, "secondary '{stem}' must precede principal: {d:?}");
                if s.is_fine() {
                    first_fine = first_fine.min(at);
                } else {
                    last_coarse = last_coarse.max(at);
                }
            }
            if first_fine != usize::MAX {
                assert!(
                    last_coarse <= first_fine,
                    "coarse secondaries must precede the fine one: {d:?}"
                );
            }
        }
    }

    /// Colour never emits a chroma word before an achromatic hue, never a
    /// redundant same-family chroma ("greenish green"), and never lightness
    /// on the lightness extremes ("light black" / "dark white").
    #[test]
    fn colour_respects_achromatic_hues() {
        let v = vocab();
        for seed in 0..2000u64 {
            let mut rng = Rng::seeded(seed);
            let c = colour(&v, &mut rng);
            for h in ACHROMATIC_HUES {
                for ch in &v.chroma {
                    assert!(
                        !c.contains(&format!("{ch} {h}")),
                        "chroma '{ch}' before achromatic '{h}': {c:?}"
                    );
                }
            }
            for ch in &v.chroma {
                assert!(
                    !c.contains(&format!("{ch} {}", chroma_base(ch))),
                    "redundant same-family chroma: {c:?}"
                );
            }
            assert!(
                !c.contains("light black")
                    && !c.contains("dark black")
                    && !c.contains("light white")
                    && !c.contains("dark white"),
                "contradictory lightness on black/white: {c:?}"
            );
        }
    }

    /// Determinism: a seed pins the bytes; different seeds genuinely vary.
    #[test]
    fn deterministic_and_varied() {
        let v = vocab();
        let one = |s: u64| describe(&v, &mut Rng::seeded(s)).text;
        assert_eq!(one(42), one(42), "same seed → identical");
        let distinct: std::collections::HashSet<_> = (0..50).map(one).collect();
        assert!(
            distinct.len() > 30,
            "50 seeds should give many distinct texts"
        );
    }

    fn unit(principals: &[&str], secondaries: &[&str], colours: &[&str]) -> UnitDescription {
        let own = |xs: &[&str]| xs.iter().map(|x| (*x).to_string()).collect::<Vec<_>>();
        UnitDescription {
            principals: own(principals),
            secondaries: own(secondaries),
            colours: own(colours),
        }
    }

    #[test]
    fn a_unit_draw_stays_inside_its_constraint() {
        let v = vocab();
        let u = unit(&["CLAY"], &["silt"], &["grey", "dark grey"]);
        let mut with_silt = 0;
        for seed in 0..300u64 {
            let d = describe_unit(&v, &mut Rng::seeded(seed), &u, fine("Firm")).unwrap();
            assert_eq!(d.principal, "CLAY", "seed {seed}");
            assert_eq!(d.principal_class, PrincipalClass::Fine);
            assert!(
                d.secondaries.iter().all(|s| s.soil == "silt"),
                "seed {seed}: {d:?}"
            );
            with_silt += d.secondaries.len();
            assert!(
                d.text == "Firm grey CLAY."
                    || d.text == "Firm dark grey CLAY."
                    || d.text == "Firm grey silty CLAY."
                    || d.text == "Firm dark grey silty CLAY.",
                "seed {seed}: {}",
                d.text
            );
        }
        assert!(with_silt > 0, "the silt secondary was never drawn");
    }

    #[test]
    fn an_empty_secondary_list_draws_none() {
        let v = vocab();
        let u = unit(&["SAND", "GRAVEL", "SILT", "CLAY"], &[], &["brown"]);
        for seed in 0..200u64 {
            let d = describe_unit(&v, &mut Rng::seeded(seed), &u, Lead::default()).unwrap();
            assert!(d.secondaries.is_empty(), "seed {seed}: {d:?}");
        }
    }

    #[test]
    fn every_listed_principal_is_reachable_and_the_lane_follows() {
        let v = vocab();
        let u = unit(&["SAND", "GRAVEL", "SILT", "CLAY"], &[], &["brown"]);
        let mut seen = std::collections::BTreeSet::new();
        for seed in 0..200u64 {
            let d = describe_unit(&v, &mut Rng::seeded(seed), &u, Lead::default()).unwrap();
            let want = if matches!(d.principal, "SAND" | "GRAVEL") {
                PrincipalClass::Coarse
            } else {
                PrincipalClass::Fine
            };
            assert_eq!(d.principal_class, want);
            seen.insert(d.principal);
        }
        assert_eq!(
            seen.into_iter().collect::<Vec<_>>(),
            ["CLAY", "GRAVEL", "SAND", "SILT"]
        );
    }

    #[test]
    fn secondaries_follow_the_engines_bands_and_order() {
        let v = vocab();
        let u = unit(&["SAND"], &["gravel", "silt"], &["brown"]);
        let mut both = 0;
        for seed in 0..400u64 {
            let d = describe_unit(&v, &mut Rng::seeded(seed), &u, coarse("Dense")).unwrap();
            let sum: u32 = d.secondaries.iter().map(|s| s.pct).sum();
            assert!(sum <= 45, "seed {seed}: {d:?}");
            for s in &d.secondaries {
                assert!(matches!(s.soil, "gravel" | "silt"), "{d:?}");
                assert_eq!(s.qualifier, band_coarse_principal(s.pct));
            }
            if d.secondaries.len() == 2 {
                both += 1;
                assert_eq!(d.secondaries[0].soil, "gravel", "coarse before fine");
                assert!(
                    d.text.contains("gravelly ") && d.text.ends_with("silty SAND."),
                    "{}",
                    d.text
                );
            }
        }
        assert!(both > 0);
    }

    #[test]
    fn a_fine_principal_bands_coarse_secondaries_on_table_17() {
        let v = vocab();
        let u = unit(&["SILT"], &["sand", "gravel", "clay"], &["grey"]);
        for seed in 0..400u64 {
            let d = describe_unit(&v, &mut Rng::seeded(seed), &u, Lead::default()).unwrap();
            let sum: u32 = d.secondaries.iter().map(|s| s.pct).sum();
            assert!(sum <= 80, "seed {seed}: {d:?}");
            let pcts: Vec<u32> = d
                .secondaries
                .iter()
                .filter(|s| !s.is_fine())
                .map(|s| s.pct)
                .collect();
            assert!(pcts.windows(2).all(|w| w[0] <= w[1]), "{d:?}");
            for s in d.secondaries.iter().filter(|s| !s.is_fine()) {
                assert_eq!(s.qualifier, band_fine_principal(s.pct));
            }
            assert!(
                d.secondaries
                    .iter()
                    .filter(|s| s.is_fine())
                    .all(|s| s.soil == "clay" && s.pct == 0),
                "{d:?}"
            );
        }
    }

    #[test]
    fn the_lead_is_the_callers_not_the_rngs() {
        let v = vocab();
        let fine_unit = unit(&["CLAY"], &[], &["grey"]);
        let coarse_unit = unit(&["SAND"], &[], &["grey"]);
        let d = describe_unit(&v, &mut Rng::seeded(1), &fine_unit, Lead::default()).unwrap();
        assert_eq!(d.lead, "");
        assert_eq!(d.text, "Grey CLAY.");
        let d = describe_unit(&v, &mut Rng::seeded(1), &fine_unit, fine("Firm")).unwrap();
        assert_eq!(d.lead, "Firm");
        assert_eq!(d.text, "Firm grey CLAY.");
        let d = describe_unit(&v, &mut Rng::seeded(1), &coarse_unit, coarse("Very dense")).unwrap();
        assert_eq!(d.lead, "Very dense");
        assert_eq!(d.text, "Very dense grey SAND.");
    }

    #[test]
    fn a_lead_in_the_wrong_lane_is_an_error() {
        let v = vocab();
        let coarse_unit = unit(&["SAND"], &[], &["grey"]);
        let fine_unit = unit(&["CLAY"], &[], &["grey"]);
        let draw = |u, lead| describe_unit(&v, &mut Rng::seeded(1), u, lead);
        assert!(draw(&coarse_unit, coarse("Firm")).is_err());
        assert!(draw(&fine_unit, fine("Dense")).is_err());
        assert!(draw(&fine_unit, fine("Nonsense")).is_err());
        // A term for the lane the unit cannot draw is still checked.
        assert!(draw(&fine_unit, coarse("Firm")).is_err());
    }

    #[test]
    fn a_mixed_unit_uses_the_term_of_the_drawn_lane() {
        let v = vocab();
        let u = unit(&["SAND", "CLAY"], &[], &["grey"]);
        let lead = Lead {
            fine: Some("Firm"),
            coarse: Some("Dense"),
        };
        let (mut sand, mut clay) = (0, 0);
        for seed in 0..50u64 {
            let d = describe_unit(&v, &mut Rng::seeded(seed), &u, lead).unwrap();
            if d.principal == "CLAY" {
                clay += 1;
                assert_eq!(d.lead, "Firm", "seed {seed}");
            } else {
                sand += 1;
                assert_eq!(d.lead, "Dense", "seed {seed}");
            }
            assert!(d.text.starts_with(&d.lead), "{}", d.text);
        }
        assert!(sand > 0 && clay > 0);
    }

    #[test]
    fn a_lane_without_a_term_draws_without_one() {
        let v = vocab();
        let u = unit(&["SAND", "CLAY"], &[], &["grey"]);
        for seed in 0..50u64 {
            let d = describe_unit(&v, &mut Rng::seeded(seed), &u, fine("Firm")).unwrap();
            assert_eq!(d.lead, if d.principal == "CLAY" { "Firm" } else { "" });
        }
    }

    #[test]
    fn a_valid_constraint_has_no_problems() {
        let v = vocab();
        let u = unit(
            &["SAND", "CLAY"],
            &["gravel", "silt"],
            &[
                "grey",
                "dark grey",
                "orangeish brown",
                "light reddish brown",
                "black",
            ],
        );
        assert_eq!(u.problems(&v), Vec::<String>::new());
    }

    #[test]
    fn problems_names_each_bad_case() {
        let v = vocab();
        let one = |u: UnitDescription| u.problems(&v);
        let p = one(unit(&["LOAM"], &[], &["grey"]));
        assert_eq!(p.len(), 1);
        assert!(
            p[0].contains("unknown principal") && p[0].contains("LOAM"),
            "{p:?}"
        );
        let p = one(unit(&["PEAT"], &[], &["grey"]));
        assert!(
            p[0].contains("unknown principal") && p[0].contains("PEAT"),
            "{p:?}"
        );
        let p = one(unit(&[], &[], &["grey"]));
        assert!(p[0].contains("no principals"), "{p:?}");
        let p = one(unit(&["CLAY"], &["loam"], &["grey"]));
        assert!(
            p[0].contains("unknown secondary") && p[0].contains("loam"),
            "{p:?}"
        );
        let p = one(unit(&["CLAY"], &["clay"], &["grey"]));
        assert!(
            p[0].contains("own fraction") && p[0].contains("clay"),
            "{p:?}"
        );
        let p = one(unit(&["SAND"], &["sand"], &["grey"]));
        assert!(p[0].contains("own fraction"), "{p:?}");
        let p = one(unit(&["CLAY"], &[], &[]));
        assert!(p[0].contains("empty palette"), "{p:?}");
    }

    /// A secondary that some listed principal can carry is usable: in a
    /// CLAY-or-SILT unit, a CLAY stratum can be silty, and the draw never
    /// gives a SILT stratum its own fraction.
    #[test]
    fn a_mixed_unit_may_list_a_fraction_one_principal_can_carry() {
        let v = vocab();
        let u = unit(&["CLAY", "SILT"], &["silt", "clay"], &["grey"]);
        assert_eq!(u.problems(&v), Vec::<String>::new());
        for seed in 0..200u64 {
            let d = describe_unit(&v, &mut Rng::seeded(seed), &u, Lead::default()).unwrap();
            let own = d.principal.to_ascii_lowercase();
            assert!(
                d.secondaries.iter().all(|s| s.soil != own),
                "seed {seed}: {d:?}"
            );
        }
    }

    #[test]
    fn problems_rejects_colours_that_break_the_composition() {
        let v = vocab();
        for bad in [
            "brown dark",
            "reddish light brown",
            "greenish green",
            "reddish white",
            "bluish black",
            "light black",
            "dark white",
            "purple",
            "orangish brown",
            "dark dark grey",
            "",
        ] {
            let p = unit(&["CLAY"], &[], &[bad]).problems(&v);
            assert_eq!(p.len(), 1, "{bad:?}: {p:?}");
            assert!(
                p[0].contains("colour") && p[0].contains(bad),
                "{bad:?}: {p:?}"
            );
        }
    }

    /// A unit palette may put a chroma before grey ("bluish grey" London
    /// Clay): BS 5930's "no chroma form exists for grey" says there is no
    /// "-ish" form of grey itself, not that grey takes no chroma. The random
    /// default draw keeps its own rule, so no pinned output moves.
    #[test]
    fn a_palette_may_put_a_chroma_before_grey() {
        let v = vocab();
        for ok in [
            "bluish grey",
            "greenish grey",
            "dark bluish grey",
            "greyish brown",
        ] {
            assert_eq!(
                unit(&["CLAY"], &[], &[ok]).problems(&v),
                Vec::<String>::new(),
                "{ok}"
            );
        }
        assert_eq!(
            unit(&["CLAY"], &[], &["greyish grey"]).problems(&v).len(),
            1
        );
    }

    #[test]
    fn problems_reports_every_problem_at_once() {
        let v = vocab();
        let p = unit(&["LOAM"], &["loam"], &[]).problems(&v);
        assert_eq!(p.len(), 3, "{p:?}");
    }

    #[test]
    fn an_invalid_constraint_is_refused_by_the_draw() {
        let v = vocab();
        let u = unit(&["CLAY"], &["clay"], &["grey"]);
        let e = describe_unit(&v, &mut Rng::seeded(1), &u, Lead::default()).unwrap_err();
        assert!(e.contains("own fraction"), "{e}");
    }

    #[test]
    fn a_unit_draw_is_deterministic_per_seed() {
        let v = vocab();
        let u = unit(&["SAND", "CLAY"], &["silt", "gravel"], &["grey", "brown"]);
        let a = describe_unit(&v, &mut Rng::seeded(9), &u, Lead::default())
            .unwrap()
            .text;
        let b = describe_unit(&v, &mut Rng::seeded(9), &u, Lead::default())
            .unwrap()
            .text;
        assert_eq!(a, b);
    }
}
