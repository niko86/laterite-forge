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

/// A copy of this checkout's oracle project (the uv project and the
/// wrapper), somewhere the compile-time default does not point.
fn oracle_project_copy() -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::create_dir(dir.path().join("tools")).unwrap();
    for f in ["pyproject.toml", "uv.lock", "tools/py_ags4_check_json.py"] {
        std::fs::copy(repo().join(f), dir.path().join(f)).expect(f);
    }
    dir
}

/// Skip (or, under `FORGE_REQUIRE_ORACLE`, fail) when the run had no oracle.
fn skipped_without_oracle(test: &str, doc: &serde_json::Value, out: &Output) -> bool {
    if doc["oracle"] != false {
        return false;
    }
    assert!(
        std::env::var_os("FORGE_REQUIRE_ORACLE").is_none(),
        "FORGE_REQUIRE_ORACLE is set but the checkout gave no oracle: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    eprintln!(
        "SKIP {test}: {}",
        String::from_utf8_lossy(&out.stderr).trim()
    );
    true
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

/// A copy of this checkout's oracle project, somewhere the compile-time
/// default does not point, stands in for the checkout a downloaded binary is
/// given. `uv run` builds its environment from the copied lock. Without uv and
/// python-ags4 the test skips with the reason printed; with
/// `FORGE_REQUIRE_ORACLE=1` (CI sets it) the skip is a failure.
#[test]
fn a_checkout_elsewhere_is_used_as_the_oracle() {
    let dir = oracle_project_copy();
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
    let doc = json(&out);
    if skipped_without_oracle("a_checkout_elsewhere_is_used_as_the_oracle", &doc, &out) {
        return;
    }
    // The exit code is the file's verdict (example1 has rust-only FYIs), not
    // the oracle's, so it is not asserted here.
    assert_eq!(doc["oracle"], true);
    assert_ne!(doc["verdict"], "RUST_ONLY", "python was not consulted");
    // The default checkout would also give an oracle, so `"oracle": true`
    // alone cannot show the flag was used. uv leaves its environment in the
    // project it ran in.
    assert!(
        dir.path().join(".venv").is_dir(),
        "uv did not run in the --oracle-project copy"
    );
    assert!(
        !String::from_utf8_lossy(&out.stderr).contains("warning:"),
        "the pinned python-ags4 warned: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

/// A project whose python-ags4 reports another version (#16): the oracle is
/// still used, and stderr says once which version it found and which one
/// the verdicts are written for. The copied wrapper's self-check is edited
/// to report the other version, which stands in for a checkout synced at a
/// different pin without installing one.
#[test]
fn another_python_ags4_is_used_with_a_warning() {
    let dir = oracle_project_copy();
    let wrapper = dir.path().join("tools/py_ags4_check_json.py");
    let text = std::fs::read_to_string(&wrapper).unwrap();
    let real = r#"getattr(python_ags4, "__version__", "?")"#;
    assert!(
        text.contains(real),
        "the wrapper's self-check has changed shape"
    );
    std::fs::write(&wrapper, text.replace(real, r#""9.9.9""#)).unwrap();
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
    let doc = json(&out);
    if skipped_without_oracle("another_python_ags4_is_used_with_a_warning", &doc, &out) {
        return;
    }
    assert_eq!(doc["oracle"], true);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_eq!(
        stderr.matches("python-ags4 9.9.9").count(),
        1,
        "expected one drift line: {stderr}"
    );
    assert!(stderr.contains("written for 1.2.0"), "{stderr}");
}

/// A path that does not exist is rejected before the oracle is probed: the
/// probe costs a uv start, and its notes would land before the real error.
#[test]
fn a_missing_file_is_rejected_before_the_oracle_is_probed() {
    let dir = tempfile::tempdir().expect("tempdir");
    let missing = dir.path().join("nope.ags");
    let out = forge(
        &[
            "check",
            missing.to_str().unwrap(),
            "--oracle-project",
            dir.path().to_str().unwrap(),
            "--json",
        ],
        &[],
    );
    assert_eq!(out.status.code(), Some(3));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!stderr.contains("python-ags4"), "probed first: {stderr}");
}
