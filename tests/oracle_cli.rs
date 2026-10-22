//! Where `check` finds python-ags4, through the real binary (#5).
//!
//! A release binary is built in CI, so the compile-time default names a
//! directory that does not exist on the user's machine. `--oracle-project`
//! (or `$AGS4_FORGE_ORACLE_PROJECT`) names the uv project instead, and a
//! missing oracle says why and how to supply one rather than degrading to
//! Rust-only without a word.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn repo() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn example() -> String {
    repo()
        .join("vendor/pyags4-tests/example1.ags")
        .display()
        .to_string()
}

fn forge(args: &[&str], env: &[(&str, &Path)]) -> Output {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_laterite-ags4-forge"));
    cmd.args(args).env_remove("AGS4_FORGE_ORACLE_PROJECT");
    for (k, v) in env {
        cmd.env(k, v);
    }
    cmd.output().expect("the binary runs")
}

fn json(out: &Output) -> serde_json::Value {
    serde_json::from_slice(&out.stdout).unwrap_or_else(|e| {
        panic!(
            "stdout is not JSON ({e}): {}\nstderr: {}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        )
    })
}

/// A directory with no uv project in it: Rust-only, and stderr names the
/// directory and the flag that fixes it.
#[test]
fn a_project_without_a_uv_project_falls_back_and_says_why() {
    let dir = tempfile::tempdir().expect("tempdir");
    let out = forge(
        &[
            "check",
            &example(),
            "--oracle-project",
            dir.path().to_str().unwrap(),
            "--json",
        ],
        &[],
    );
    assert_eq!(out.status.code(), Some(0));
    assert_eq!(json(&out)["oracle"], false);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains(&dir.path().display().to_string()),
        "stderr does not name the project: {stderr}"
    );
    assert!(
        stderr.contains("--oracle-project"),
        "stderr does not say how to supply one: {stderr}"
    );
}

/// The environment variable names the project when the flag does not.
#[test]
fn the_environment_variable_names_the_project() {
    let dir = tempfile::tempdir().expect("tempdir");
    let out = forge(
        &["check", &example(), "--json"],
        &[("AGS4_FORGE_ORACLE_PROJECT", dir.path())],
    );
    assert_eq!(out.status.code(), Some(0));
    assert_eq!(json(&out)["oracle"], false);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains(&dir.path().display().to_string()),
        "stderr does not name the project: {stderr}"
    );
}

/// Pointed at a synced checkout, `check` consults python-ags4. This is what
/// a downloaded binary does with `--oracle-project`. Without uv and
/// python-ags4 it skips with the reason printed; with
/// `FORGE_REQUIRE_ORACLE=1` (CI sets it) the skip is a failure.
#[test]
fn a_synced_checkout_is_used_as_the_oracle() {
    let out = forge(
        &[
            "check",
            &example(),
            "--oracle-project",
            repo().to_str().unwrap(),
            "--json",
        ],
        &[],
    );
    let doc = json(&out);
    if doc["oracle"] == false {
        assert!(
            std::env::var_os("FORGE_REQUIRE_ORACLE").is_none(),
            "FORGE_REQUIRE_ORACLE is set but the checkout gave no oracle: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        eprintln!(
            "SKIP a_synced_checkout_is_used_as_the_oracle: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        );
        return;
    }
    // The exit code is the file's verdict (example1 has rust-only FYIs), not
    // the oracle's, so it is not asserted here.
    assert_eq!(doc["oracle"], true);
    assert_ne!(doc["verdict"], "RUST_ONLY", "python was not consulted");
}
