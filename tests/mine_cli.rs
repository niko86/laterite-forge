//! Where `mine` finds the corpus it subtracts, through the real binary (#15).
//!
//! The vendored python-ags4 corpus is LGPL data, so the binary does not
//! carry it. With no `--corpus` it is read from the oracle checkout, the same
//! one `--oracle-project` names, and a corpus that yields nothing says so
//! rather than reading as "everything is a gap".

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const CORPUS: &str = "vendor/pyags4-tests";

fn repo() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// The smallest Rust-only mine: one rule at a time, one seed.
fn mine(project: &Path, out: &Path, extra: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_laterite-ags4-forge"))
        .args(["mine", "--min-k", "1", "--max-k", "1", "--seeds", "1"])
        .args(["--no-oracle", "--json", "--out-dir"])
        .arg(out)
        .arg("--oracle-project")
        .arg(project)
        .args(extra)
        .env_remove("AGS4_FORGE_ORACLE_PROJECT")
        .output()
        .expect("the binary runs")
}

fn corpus_files(out: &Output) -> u64 {
    let doc: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap_or_else(|e| {
        panic!(
            "stdout is not JSON ({e}): {}\nstderr: {}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        )
    });
    doc["corpus_files"].as_u64().expect("corpus_files")
}

/// A checkout elsewhere holding part of the corpus: that part is what is
/// subtracted, not the corpus of the checkout the binary was built from.
#[test]
fn the_default_corpus_is_the_oracle_checkouts() {
    let project = tempfile::tempdir().expect("tempdir");
    let out = tempfile::tempdir().expect("tempdir");
    let corpus = project.path().join(CORPUS);
    std::fs::create_dir_all(&corpus).unwrap();
    for f in ["example1.ags", "4.1-rule2.ags"] {
        std::fs::copy(repo().join(CORPUS).join(f), corpus.join(f)).expect(f);
    }
    let run = mine(project.path(), out.path(), &[]);
    assert_eq!(
        corpus_files(&run),
        2,
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
}

/// An oracle checkout with no corpus: mine still runs, subtracts nothing,
/// and stderr names the path it looked in and how to supply one.
#[test]
fn a_checkout_without_the_corpus_says_so() {
    let project = tempfile::tempdir().expect("tempdir");
    let out = tempfile::tempdir().expect("tempdir");
    let run = mine(project.path(), out.path(), &[]);
    assert_eq!(corpus_files(&run), 0);
    let stderr = String::from_utf8_lossy(&run.stderr);
    let looked = project.path().join(CORPUS).display().to_string();
    assert!(
        stderr.contains(&looked),
        "stderr does not name {looked}: {stderr}"
    );
    assert!(stderr.contains("--corpus"), "{stderr}");
}

/// An explicit `--corpus` wins over the checkout, and an empty one is
/// reported the same way.
#[test]
fn an_empty_explicit_corpus_says_so() {
    let out = tempfile::tempdir().expect("tempdir");
    let empty = tempfile::tempdir().expect("tempdir");
    let run = mine(
        &repo(),
        out.path(),
        &["--corpus", empty.path().to_str().unwrap()],
    );
    assert_eq!(corpus_files(&run), 0);
    let stderr = String::from_utf8_lossy(&run.stderr);
    assert!(
        stderr.contains(&empty.path().display().to_string()),
        "stderr does not name the corpus: {stderr}"
    );
}
