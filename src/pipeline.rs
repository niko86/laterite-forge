//! The dual-validate pipeline for one candidate.
//!
//! Rust runs **in-process** on 100% of candidates via the validator
//! library (fast, no subprocess), reduced through the *shared*
//! `laterite_ags4_parity::RustResult` so forge's verdict is definitionally
//! identical to the corpus-qa parity model. python-ags4 runs via the
//! shared `PyOracle` bridge. `dual_validate` consults the oracle whenever
//! it's given one; *whether* to spend an oracle call is decided upstream in
//! the `run` loop by the adaptive Beta–Bernoulli confidence ledger
//! ([`crate::confidence`]). A missing oracle degrades to a Rust-only verdict
//! (optional QA), never an error.
//!
//! The pipeline itself lives in `laterite_ags4_parity::check` since niko86/laterite#1109,
//! where `laterite-ags4-corpus-qa check` runs the same code; this module
//! re-exports it so forge's call sites are unchanged, and keeps the tests
//! that hold forge's own generator to it.

use std::path::PathBuf;

use laterite_ags4_parity::PyOracle;
pub use laterite_ags4_parity::check::{Outcome, dual_validate, rust_check};

/// Where the oracle runs: this repo's wrapper script and the uv project at
/// this repo's root (its `pyproject.toml` pins python-ags4). The parity crate
/// does not find them for itself (niko86/laterite#1108), so forge names them.
/// The crate IS the repo root, so the manifest dir is the project.
pub fn oracle_site() -> (PathBuf, PathBuf) {
    let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    (repo.join("tools/py_ags4_check_json.py"), repo)
}

/// [`laterite_ags4_parity::check::build_oracle`] at [`oracle_site`].
pub fn build_oracle(timeout_secs: u64) -> Option<(PyOracle, String)> {
    let (wrapper, project) = oracle_site();
    laterite_ags4_parity::check::build_oracle(wrapper, project, timeout_secs)
}

#[cfg(test)]
mod tests {
    use super::*;
    use laterite_ags4_parity::RustResult;

    use crate::ops::{Injection, synth_combined, synth_injected};
    use crate::synth::{Scaffold, synth};

    /// The oracle site is this repo's own root: the wrapper and the uv
    /// project that pins python-ags4 must both be where `oracle_site` says.
    /// Needs no python, so a wrong path fails here rather than degrading
    /// every run to Rust-only without a word.
    #[test]
    fn oracle_site_names_this_repos_wrapper_and_project() {
        let (wrapper, project) = oracle_site();
        assert!(wrapper.is_file(), "missing {}", wrapper.display());
        assert!(
            project.join("pyproject.toml").is_file(),
            "no uv project at {}",
            project.display()
        );
    }

    /// One vendored python-ags4 file through both engines. Without uv and
    /// python-ags4 this skips with the reason printed; with
    /// `FORGE_REQUIRE_ORACLE=1` (CI sets it) a missing oracle is a failure,
    /// so the skip cannot hide there.
    #[test]
    fn the_oracle_returns_a_verdict_on_a_vendored_file() {
        let Some((oracle, version)) = build_oracle(120) else {
            assert!(
                std::env::var_os("FORGE_REQUIRE_ORACLE").is_none(),
                "FORGE_REQUIRE_ORACLE is set but python-ags4 is unavailable \
                 (run `uv sync` at the repo root)"
            );
            eprintln!(
                "SKIP the_oracle_returns_a_verdict_on_a_vendored_file: python-ags4 \
                 unavailable (no uv, or `uv sync` not run at the repo root)"
            );
            return;
        };
        assert_eq!(version, laterite_ags4_parity::EXPECTED_PYAGS4);
        let file =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("vendor/pyags4-tests/example1.ags");
        let out = dual_validate(&file, Some(&oracle));
        assert!(out.verdict.is_some(), "no verdict: {out:?}");
        assert!(
            matches!(&out.python, Some(Ok(_))),
            "python side failed: {:?}",
            out.python
        );
    }

    fn rust_on(text: &str) -> RustResult {
        let tmp = tempfile::tempdir().expect("tempdir");
        let p = tmp.path().join("c.ags");
        std::fs::write(&p, text.as_bytes()).unwrap();
        rust_check(&p).0
    }

    /// The one label a clean synthetic base cannot avoid: `Wide` carries the
    /// standard LBST group, whose KEY tuple contains SAMP's while it declares
    /// LBSG, so `FYI (Related to Rule 10c)` fires on the DICTIONARY's shape
    /// rather than on anything the generator did (niko86/laterite#759 / O-56). Named here
    /// rather than relaxing the net to "no errors", because the net's job is
    /// that EVERY other finding is a generator bug.
    const DICTIONARY_SHAPE_ADVISORY: &str = "FYI (Related to Rule 10c)";

    fn clean_by_construction(r: &RustResult) -> bool {
        match r {
            RustResult::Clean => true,
            RustResult::Rules(s) => s.iter().all(|k| k == DICTIONARY_SHAPE_ADVISORY),
            _ => false,
        }
    }

    /// THE SAFETY NET: the un-injected synthetic base must carry nothing but
    /// the dictionary-shape advisory above, for **every seed** — the generator
    /// is varied, so this asserts variety never breaks clean-by-construction. A
    /// generator bug fails *here*, never as a reported "finding".
    #[test]
    fn varied_baseline_is_rust_clean_across_seeds() {
        for seed in 0..25 {
            for sc in [Scaffold::Minimal, Scaffold::LocaSamp, Scaffold::Wide] {
                let r = rust_on(&synth(sc, seed));
                assert!(
                    clean_by_construction(&r),
                    "{sc:?} seed {seed} not clean in Rust: {r:?}"
                );
            }
        }
    }

    /// The carve-out above is a hole in the net, so it is held to its exact
    /// size: `Wide` must ACTUALLY carry the advisory (otherwise the allowance
    /// is dead and hiding a future finding), and `Minimal` — which has no LBST
    /// — must still be `Clean` outright.
    #[test]
    fn the_dictionary_shape_advisory_is_the_only_hole_in_the_net() {
        let wide = rust_on(&synth(Scaffold::Wide, 0));
        assert_eq!(
            wide,
            RustResult::Rules(
                [DICTIONARY_SHAPE_ADVISORY.to_string()]
                    .into_iter()
                    .collect()
            ),
            "Wide should carry the LBST advisory and nothing else"
        );
        assert_eq!(rust_on(&synth(Scaffold::Minimal, 0)), RustResult::Clean);
    }

    /// A single-rule injection yields exactly the targeted rule (the
    /// defect is single + isolable — the honesty principle).
    #[test]
    fn rule10a_injection_trips_exactly_rule_10a() {
        let r = rust_on(&synth_injected(
            Scaffold::LocaSamp,
            7,
            Injection::DupSampKeyTuple,
        ));
        match r {
            RustResult::Rules(s) => {
                assert!(
                    s.contains("AGS Format Rule 10a"),
                    "expected Rule 10a, got {s:?}"
                );
            }
            other => panic!("expected Rules with 10a, got {other:?}"),
        }
    }

    /// A *combination* of two non-masking injectors trips BOTH target
    /// rules in the real validator — the honesty check behind `--combine`:
    /// the file's actual rule-set really does carry every fault we asked
    /// for (Rule 8 bad date + Rule 19 five-letter group), so a synthesized
    /// combination's signature is trustworthy. Across seeds, so it holds
    /// for the varied base, not one lucky placement.
    #[test]
    fn combination_trips_both_target_rules() {
        for seed in [2, 8, 16] {
            let text = synth_combined(
                Scaffold::LocaSamp,
                seed,
                &[Injection::BadDtValue, Injection::FiveLetterGroup],
            );
            match rust_on(&text) {
                RustResult::Rules(s) => {
                    assert!(
                        s.contains("AGS Format Rule 8") && s.contains("AGS Format Rule 19"),
                        "seed {seed}: combination must trip both Rule 8 and Rule 19, got {s:?}"
                    );
                }
                other => panic!("seed {seed}: expected Rules, got {other:?}"),
            }
        }
    }

    /// The regression guard: each model-mutation injector still trips its
    /// declared `target_rule` in the real validator — across seeds, so it
    /// holds for the varied base, not one lucky file. The compliance/
    /// parity matrix relies on this injector→rule mapping.
    #[test]
    fn every_injector_trips_its_target_rule() {
        for seed in [3, 11, 19] {
            for &inj in Injection::ALL {
                let target = inj.target_rule().expect("injector has a target rule");
                let r = rust_on(&synth_injected(Scaffold::LocaSamp, seed, inj));
                match r {
                    RustResult::Rules(s) => assert!(
                        s.contains(target),
                        "seed {seed} {inj}: expected {target}, got {s:?}"
                    ),
                    other => panic!("seed {seed} {inj}: expected Rules({target}), got {other:?}"),
                }
            }
        }
    }
}
