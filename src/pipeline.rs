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

use std::path::{Path, PathBuf};
use std::time::Duration;

pub use laterite_ags4_parity::check::{Outcome, dual_validate, rust_check};
use laterite_ags4_parity::{OracleError, PyOracle};
use laterite_cliutil::report::note;

/// The wrapper script, relative to the uv project that runs it.
const WRAPPER: &str = "tools/py_ags4_check_json.py";

/// The usual fix when the oracle is missing: name a checkout that has one.
const HOW: &str = "Point --oracle-project (or $AGS4_FORGE_ORACLE_PROJECT) at a \
                   laterite-forge checkout and run `uv sync` there.";

/// Why python-ags4 could not be reached, and what the user can do about it.
pub struct Unavailable {
    why: String,
    fix: String,
}

impl Unavailable {
    fn new(why: String) -> Self {
        Self {
            why,
            fix: HOW.to_string(),
        }
    }
}

impl std::fmt::Display for Unavailable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "python-ags4 unavailable: {}. {}", self.why, self.fix)
    }
}

/// Where the oracle runs: a uv project whose `pyproject.toml` pins
/// python-ags4, holding the wrapper script. The parity crate does not find
/// them for itself (niko86/laterite#1108), so forge names them.
pub struct OracleSite {
    pub wrapper: PathBuf,
    pub project: PathBuf,
}

impl OracleSite {
    /// `project` is `--oracle-project` / `$AGS4_FORGE_ORACLE_PROJECT`.
    /// Without one it is the checkout this binary was built from: right for
    /// `cargo run`, and a CI directory that no longer exists for a
    /// downloaded release (#5).
    pub fn new(project: Option<&Path>) -> Self {
        let project = project.map_or_else(
            || PathBuf::from(env!("CARGO_MANIFEST_DIR")),
            Path::to_path_buf,
        );
        Self {
            wrapper: project.join(WRAPPER),
            project,
        }
    }

    /// python-ags4 at this site and its version, or why not and how to
    /// supply one. The layout is checked before uv runs: uv given a missing
    /// working directory fails to spawn, which reads as "no uv" when it is
    /// really "wrong directory".
    pub fn oracle(&self, timeout_secs: u64) -> Result<(PyOracle, String), Unavailable> {
        if !self.project.join("pyproject.toml").is_file() {
            return Err(Unavailable::new(format!(
                "no uv project at {} (no pyproject.toml)",
                self.project.display()
            )));
        }
        if !self.wrapper.is_file() {
            return Err(Unavailable::new(format!(
                "{} has no {WRAPPER}",
                self.project.display()
            )));
        }
        let oracle = PyOracle::new(
            "uv",
            self.wrapper.clone(),
            self.project.clone(),
            Duration::from_secs(timeout_secs),
        );
        match oracle.selfcheck() {
            Ok(sc) => {
                if let Some(warning) = drift(sc.python_ags4.as_deref(), &self.project) {
                    note(warning);
                }
                Ok((oracle, sc.python_ags4.unwrap_or_else(|| "unknown".into())))
            }
            Err(e) => Err(reason(&e, &self.project)),
        }
    }

    /// [`Self::oracle`] for callers that degrade to Rust-only. `fallback`
    /// says what the command does instead (`"gen: Rust-only verdicts"`); it
    /// and the reason go to stderr as one line, and the oracle is absent.
    pub fn build_oracle(&self, timeout_secs: u64, fallback: &str) -> Option<(PyOracle, String)> {
        self.oracle(timeout_secs)
            .map_err(|e| note(format!("{fallback} — {e}")))
            .ok()
    }
}

/// A warning when python-ags4 is not the version the parity reconcile arms
/// were written for. The oracle is still used: the arms may still hold, and
/// the parity crate leaves the call to the caller, asking only that drift is
/// not silent.
fn drift(reported: Option<&str>, project: &Path) -> Option<String> {
    let expected = laterite_ags4_parity::EXPECTED_PYAGS4;
    let p = project.display();
    let found = match reported {
        Some(v) if v == expected => return None,
        // The wrapper reports "?" when python-ags4 has no `__version__`.
        Some(v) if v != "?" => format!("python-ags4 {v} in {p}"),
        _ => format!("could not read the python-ags4 version in {p}"),
    };
    Some(format!(
        "warning: {found}, but the parity verdicts are written for {expected} and may be \
         wrong. Pin python-ags4=={expected} in {p} and run `uv sync` there."
    ))
}

/// Why the self-check failed, in terms of what the user can do about it.
fn reason(err: &OracleError, project: &Path) -> Unavailable {
    let p = project.display();
    match err {
        OracleError::NotImportable => Unavailable {
            why: format!("it is not importable in {p}"),
            fix: format!(
                "Run `uv sync` in {p}, or point --oracle-project (or \
                 $AGS4_FORGE_ORACLE_PROJECT) at a synced laterite-forge checkout."
            ),
        },
        OracleError::Unavailable(e) => Unavailable {
            why: format!("could not run uv ({e})"),
            fix: format!("Install uv (https://docs.astral.sh/uv/), then run `uv sync` in {p}."),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use laterite_ags4_parity::RustResult;

    use crate::ops::{Injection, synth_combined, synth_injected};
    use crate::synth::{Scaffold, synth};

    /// A uv project without the wrapper is caught before uv runs, and the
    /// reason names what is missing and how to supply an oracle.
    #[test]
    fn a_project_without_the_wrapper_says_so() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(dir.path().join("pyproject.toml"), "").unwrap();
        let Err(why) = OracleSite::new(Some(dir.path())).oracle(5) else {
            panic!("an oracle without a wrapper");
        };
        let why = why.to_string();
        assert!(why.contains("tools/py_ags4_check_json.py"), "{why}");
        assert!(why.contains("--oracle-project"), "{why}");
    }

    /// uv ran but python-ags4 would not import: the fix is `uv sync` in that
    /// project, so the reason says exactly that.
    #[test]
    fn python_ags4_not_importable_says_to_sync_the_project() {
        let why = reason(&OracleError::NotImportable, Path::new("/srv/forge")).to_string();
        assert!(why.contains("not importable in /srv/forge"), "{why}");
        assert!(why.contains("Run `uv sync` in /srv/forge"), "{why}");
    }

    /// uv itself would not start: the fix is installing uv, not pointing
    /// forge somewhere else.
    #[test]
    fn uv_that_will_not_run_says_to_install_uv() {
        let why = reason(
            &OracleError::Unavailable("No such file or directory (os error 2)".into()),
            Path::new("/srv/forge"),
        )
        .to_string();
        assert!(
            why.contains("could not run uv (No such file or directory"),
            "{why}"
        );
        assert!(why.contains("Install uv"), "{why}");
    }

    /// The pinned version is the one the reconcile arms were written for, so
    /// it says nothing.
    #[test]
    fn the_pinned_python_ags4_is_not_drift() {
        assert_eq!(
            drift(
                Some(laterite_ags4_parity::EXPECTED_PYAGS4),
                Path::new("/srv/forge")
            ),
            None
        );
    }

    /// Another version still runs, but the user is told the verdicts rest
    /// on arms written for a different one, and how to get the pinned one.
    #[test]
    fn another_python_ags4_names_both_versions() {
        let msg = drift(Some("1.3.0"), Path::new("/srv/forge")).expect("drift");
        assert!(msg.contains("python-ags4 1.3.0 in /srv/forge"), "{msg}");
        assert!(msg.contains("written for 1.2.0"), "{msg}");
        assert!(
            msg.contains("Pin python-ags4==1.2.0 in /srv/forge and run `uv sync`"),
            "{msg}"
        );
    }

    /// A self-check that reports no version, or the wrapper's "?" for a
    /// python-ags4 without `__version__`, cannot be trusted to match.
    #[test]
    fn an_unreadable_python_ags4_version_is_drift() {
        for reported in [None, Some("?")] {
            let msg = drift(reported, Path::new("/srv/forge")).expect("drift");
            assert!(
                msg.contains("could not read the python-ags4 version in /srv/forge"),
                "{msg}"
            );
            assert!(msg.contains("written for 1.2.0"), "{msg}");
        }
    }

    /// The default oracle site is this repo's own root: the wrapper and the
    /// uv project that pins python-ags4 must both be where `OracleSite`
    /// says. Needs no python, so a wrong path fails here rather than
    /// degrading every run to Rust-only.
    #[test]
    fn the_default_oracle_site_is_this_repos_wrapper_and_project() {
        let site = OracleSite::new(None);
        assert!(site.wrapper.is_file(), "missing {}", site.wrapper.display());
        assert!(
            site.project.join("pyproject.toml").is_file(),
            "no uv project at {}",
            site.project.display()
        );
    }

    /// One vendored python-ags4 file through both engines. Without uv and
    /// python-ags4 this skips with the reason printed; with
    /// `FORGE_REQUIRE_ORACLE=1` (CI sets it) a missing oracle is a failure,
    /// so the skip cannot hide there.
    #[test]
    fn the_oracle_returns_a_verdict_on_a_vendored_file() {
        let Some((oracle, version)) = OracleSite::new(None).build_oracle(120, "test: skipped")
        else {
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
