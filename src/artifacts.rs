//! Run-versioned artifact paths — the corpus-qa `paths.rs` convention.
//!
//! Work dir resolution: explicit `--out-dir` → `$AGS4_FORGE_DIR` →
//! `./forge-runs` (CWD-relative — the least-surprise convention; a
//! copied release binary must not write back into the source checkout
//! via a compile-time `CARGO_MANIFEST_DIR`). Each run gets a sortable
//! UTC dir under `<out>/runs/<id>/` (suffixed `-002`… when a run in the
//! same second already holds the id); a `runs/latest` pointer *file*
//! (not a symlink — Windows-robust) names the newest.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use chrono::Utc;

/// `--out-dir` → `$AGS4_FORGE_DIR` → `./forge-runs`.
pub fn forge_dir(flag: Option<&Path>) -> PathBuf {
    if let Some(p) = flag {
        return p.to_path_buf();
    }
    if let Some(env) = std::env::var_os("AGS4_FORGE_DIR") {
        return PathBuf::from(env);
    }
    std::env::current_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join("forge-runs")
}

/// Sortable UTC run id — lexical order == chronological.
fn new_run_id() -> String {
    Utc::now().format("%Y%m%dT%H%M%SZ").to_string()
}

pub fn run_dir(out: &Path, run_id: &str) -> PathBuf {
    out.join("runs").join(run_id)
}

/// Create a fresh run dir under `<out>/runs/` and return its id. The id is
/// [`new_run_id`], with `-002`, `-003`… appended when a run started in the
/// same second already holds it, so two runs never share a dir. The create
/// is exclusive, so concurrent runs cannot both claim one id.
pub fn claim_run(out: &Path) -> Result<String> {
    claim_run_as(out, &new_run_id())
}

fn claim_run_as(out: &Path, base: &str) -> Result<String> {
    let runs = out.join("runs");
    std::fs::create_dir_all(&runs).with_context(|| format!("create {}", runs.display()))?;
    for n in 1..=999 {
        let id = if n == 1 {
            base.to_string()
        } else {
            format!("{base}-{n:03}")
        };
        match std::fs::create_dir(runs.join(&id)) {
            Ok(()) => return Ok(id),
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(e) => {
                return Err(e).with_context(|| format!("create {}", runs.join(&id).display()));
            }
        }
    }
    anyhow::bail!("999 runs already started at {base} in {}", runs.display())
}

/// Point `runs/latest` at `run_id`.
pub fn set_latest_run(out: &Path, run_id: &str) -> Result<()> {
    let runs = out.join("runs");
    std::fs::create_dir_all(&runs).with_context(|| format!("create {}", runs.display()))?;
    std::fs::write(runs.join("latest"), run_id).with_context(|| "write runs/latest pointer")
}

/// Reject an out-dir that resolves inside the validator's asserted-
/// clean fixtures tree — forge must NEVER write there (corpus-qa e2e
/// asserts that dir hard-error-free; confirmed reproducers are evidence
/// for a niko86/laterite issue, not fixtures). The guard still matters when
/// forge runs from inside a laterite checkout.
pub fn guard_out_dir(out: &Path) -> Result<()> {
    let abs = std::path::absolute(out).unwrap_or_else(|_| out.to_path_buf());
    let s = abs.to_string_lossy().replace('\\', "/");
    if s.contains("laterite-ags4-validator/tests/fixtures") {
        anyhow::bail!(
            "refusing --out-dir inside laterite-ags4-validator/tests/fixtures \
             (that dir is asserted hard-error-free; forge output must not \
             pollute it)"
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runs_started_in_the_same_second_get_separate_dirs() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let ids: Vec<String> = (0..3)
            .map(|_| claim_run_as(tmp.path(), "20261024T120000Z").expect("claim"))
            .collect();
        assert_eq!(
            ids,
            [
                "20261024T120000Z",
                "20261024T120000Z-002",
                "20261024T120000Z-003"
            ]
        );
        for id in &ids {
            assert!(run_dir(tmp.path(), id).is_dir(), "{id}");
        }
    }

    #[test]
    fn a_claimed_id_sorts_after_the_runs_before_it() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let a = claim_run_as(tmp.path(), "20261024T120000Z").expect("claim");
        let b = claim_run_as(tmp.path(), "20261024T120000Z").expect("claim");
        let c = claim_run_as(tmp.path(), "20261024T120001Z").expect("claim");
        assert!(a < b && b < c, "{a} {b} {c}");
    }
}
