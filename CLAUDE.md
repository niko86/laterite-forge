# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this is

`laterite-ags4-forge`: a single-crate Rust binary (`publish = false`) that generates AGS4 files, checks each one with laterite's Rust validator (run in-process) and with python-ags4, and reports where the two disagree (a "divergence"). It embeds no LLM. An agent writes a declarative `strategy.toml` and the binary does the deterministic work. Forge lived in `niko86/laterite` until recently. Every `laterite-*` dependency comes from crates.io, and dependencies run one way only: forge depends on laterite, never the reverse. Issue references take the form `niko86/laterite#N`.

## Commands

CI (`.github/workflows/ci.yml`) runs exactly these:

```sh
cargo fmt --check
cargo clippy --all-targets --locked -- -D warnings   # clippy::pedantic is on (Cargo.toml [lints.clippy])
uv sync --locked                                      # python-ags4 oracle env
FORGE_REQUIRE_ORACLE=1 cargo test --locked
cargo deny check
```

- Run a single test with `cargo test <name>`, e.g. `cargo test synth_output_bytes_are_pinned`. Most tests are inline `#[cfg(test)]` modules. `tests/edit_cli.rs` drives the real binary end to end.
- `rust-toolchain.toml` pins rustc 1.95.0 so the pedantic `-D warnings` gate gives the same result everywhere. Bump it deliberately.
- Without `uv sync`, the oracle test skips and prints the reason. `FORGE_REQUIRE_ORACLE=1` turns that skip into a failure.
- Run the binary with `cargo run --release -- <subcommand>`, e.g. `cargo run --release -- check vendor/pyags4-tests/example1.ags`. `--help` lists the subcommands. `--readme` prints `README-cli.md`, which is embedded with `include_str!`.

## Architecture

- `main.rs` dispatches to `cmd.rs`, which has one function per subcommand. `cli.rs` holds the clap definitions. The `#[arg]` doc comments there are both rustdoc and the user-facing `--help` text, which is why `rustdoc::invalid_html_tags` is allowed.
- **CLI contract** (`laterite-cliutil`): results go to stdout (`--output table|json|ndjson`; ndjson is automatic when stdout is piped), progress goes to stderr, and `--compact` gives the token-lean agent view. Exit codes: `0` clean or a documented divergence, `1` a parity action (a real Rust↔python divergence), `3` I/O error, `5` bad arguments.
- **Dual validation** (`pipeline.rs`): the shared `laterite-ags4-parity` crate (`check::dual_validate`, `classify`/`reconcile`, `PyOracle`) does the work, so verdicts match `laterite-ags4-corpus-qa` exactly. Don't reimplement verdict logic here. The oracle runs `uv run python tools/py_ags4_check_json.py` in a uv project: `--oracle-project` or `$AGS4_FORGE_ORACLE_PROJECT` when given, else the repo root (`CARGO_MANIFEST_DIR`), which a downloaded release cannot rely on. If python-ags4 is missing, forge falls back to a Rust-only verdict and says why on stderr instead of failing.
- **python-ags4 is pinned, not floored**, at `==1.2.0` in `pyproject.toml`. The parity reconcile arms encode 1.2.0 behaviour. After the self-check, `OracleSite::oracle` compares the reported version with `laterite_ags4_parity::EXPECTED_PYAGS4` and, on a mismatch or an unreadable version, warns once on stderr and still uses the oracle. Bumping the pin invalidates those arms with no test failure.
- **Synthesizer** (`synth/`): builds a `ProjectModel` (groups of string columns by position, not typed holes or strata) from the validator's bundled v4.2 dictionary and emits it (`synth/emit.rs`). There are three scaffolds: `minimal`, `loca-samp`, and `wide` (dictionary-driven breadth in `breadth.rs` plus lab-test depth below SAMP in `depth.rs`). Output is clean by construction, and a test asserts that, so a generator bug shows up as a failing test rather than as a "finding". `bs5930.rs` generates BS 5930 soil descriptions from `data/bs5930/*.json`.
- **Site model** (`site/`, ADR 0001): `gen --site <preset>` and `scale --site <preset>` (visible in `--help` and `README-cli.md` since london-basin v3) build believable ground from a preset (`site/preset.rs`, schema v1) instead of a scaffold. `site/build.rs` is a typed layer (holes, strata) lowered into `ProjectModel`, sharing nothing with the scaffold builder. Every draw has its own RNG keyed by (seed, tag, hole, unit, test) (`site/draw.rs`), so hole k is the same whatever the hole count and changing one unit moves only its rows; tests hold both. Unit surfaces and ground level are smooth fields (`site/ground.rs`).- **Injectors** (`ops.rs`): each one mutates the model at a seeded placement (a separate salted RNG), never by rewriting bytes in the emitted text. Faults can mask or cascade into one another, so a combination's real rule set is always read back from the validator, never assumed from the injectors' targets.
- **Search loop**: `evolve.rs` (`run`) is gated by the Beta–Bernoulli oracle-confidence ledger in `confidence.rs`. `mine.rs` sweeps every rule combination, removes the shapes the vendored python-ags4 corpus (`vendor/pyags4-tests/` in the oracle checkout; it is LGPL data, so the binary does not carry it) already covers, and spends oracle calls only on novel divergence-prone signatures. `minimize.rs` runs ddmin to shrink a finding to a minimal reproducer. `strategy.rs` loads `strategy.toml`. `scale.rs` calibrates a `wide` file to a target byte size, optionally spreading faults across a set fraction of sites (`--density`).
- **`edit.rs`** changes existing files and synthesizes nothing. It splices lines: lines it doesn't touch are written back byte-for-byte, and lines it does touch are rebuilt with the canonical `laterite-ags4-types::quote_field`. Operations resolve against the file as it arrived (row numbers count the original rows) and are applied in a fixed order. Shapes that would be ambiguous are refused rather than guessed at. `minimize`'s field pass also goes through this layer, because splitting on commas without quote awareness corrupts quoted values.
- Run output goes to `--out-dir`, else `$AGS4_FORGE_DIR`, else `./forge-runs/runs/<id>/` (gitignored). The `--out-dir` must not resolve inside the validator's `tests/fixtures/`.

## Pinned outputs: don't "fix" a red pin test

- `synth_output_bytes_are_pinned` (`synth/mod.rs`) pins the SHA-256 and length of each scaffold's output for seeds 0–2. laterite's benchmark tooling draws fixtures from this engine. Re-roll it only on purpose, and regenerate the bench fixtures in the same change.
- `the_unflagged_draw_is_pinned_to_pre_lane_bytes` (`synth/bs5930.rs`) pins the default `describe` draw. Downstream consumers commit output from it, so moving it is a change they must be told about (`docs/dec-forge-audience-boundary.md`). New behaviour therefore goes behind a flag (e.g. `--organic`), not into the default draw.
- `scaffolds_reach_a_pinned_set_of_ags_types` (`synth/mod.rs`) pins which AGS TYPEs the scaffolds can produce.
- `shipped_preset_output_bytes_are_pinned` and `shipped_presets_reach_a_pinned_set_of_ags_types` (`site/build.rs`) pin each shipped preset (`data/presets/`). Changing a preset's output means bumping its `version`, re-pinning, and telling consumers (ADR 0001). That promise is public from london-basin v3: any output change bumps the version and consumers re-baseline.
- `each_setting_and_settlement_writes_pinned_bytes` (`site/build.rs`) pins each `[made_ground]` setting and settlement. Changing `data/made-ground/settings.json` changes the output of every preset with the block, so it bumps each such preset's version.

## Docs

`docs/` is the design record carried over from laterite's `ags-wiki/`. It is deliberately left as it was: `[[wikilinks]]` and `repo:` paths point into the laterite repo, where forge lived under `rust-packages/laterite-ags4-forge/`. New decisions go in `docs/adr/`. ADR 0001 (forge owns a site model) supersedes `dec-forge-audience-boundary`. `docs/laterite-ags4-forge.md` is the fullest description of every subcommand's behaviour. `README-cli.md` (the `--readme` text) covers `check`, `gen`, and the site model (`gen --site`, `scale --site`).

## Agent skills

### Issue tracker

Issues live in this repo's GitHub Issues (`niko86/laterite-forge`), managed with the `gh` CLI. See `docs/agents/issue-tracker.md`, which also carries the PR rules: every change goes through a PR, the maintainer merges (code-only PRs excepted), and commits and PR bodies carry no AI-assistant references.

### Build agents and review

- A sub-agent building an issue follows `docs/agents/build-brief.md`.
- Before editing `data/presets/`, or reviewing a diff that does, read `CODING_STANDARDS.md`.
- `tools/preset_outline.py <preset> [filter]` prints a preset's values with the bases cut short.

### Triage labels

The five default labels, each spelled the same as its role: `needs-triage`, `needs-info`, `ready-for-agent`, `ready-for-human`, `wontfix`. See `docs/agents/triage-labels.md`.

### Domain docs

Single-context: one `GLOSSARY.md` and `docs/adr/` at the repo root, created only when first needed. See `docs/agents/domain.md`.
