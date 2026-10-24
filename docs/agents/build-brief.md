# Build brief

Standing rules for a sub-agent building an issue on a branch. The lead agent's brief adds the issue, its decisions and its figures; these rules hold under every brief.

## Start

1. Rebase onto the current `main` before the first edit: `git fetch origin && git rebase origin/main`. A worktree can be made from a stale local `main`.
2. Read `CLAUDE.md`, then the issue and its triage comment. Before editing `data/presets/`, read `CODING_STANDARDS.md`.
3. Read presets with `tools/preset_outline.py <preset> [filter]`, and use `--full` for the one basis you are editing.

## Build

- **Keyed draws.**
  - A new feature takes new `Key` tags (`src/site/draw.rs`) and consumes no existing stream.
  - New AGS groups go last.
  - A new preset block is opt-in, so a preset without it writes the previous version's bytes. Prove that with an `a_preset_without_its_<block>_writes_the_previous_versions_bytes` test.
- **Pins.**
  - Pin tests turn red on purpose; a red pin is a finding.
  - Re-pin only a preset whose `version` the brief bumps, in the same commit.
- **Calibration.**
  - Measure file figures over many seeds.
  - Give each figure in the basis beside the source's figure.
  - A figure that cannot be matched is reported as a miss, not tuned away silently.
- **Out-dirs.** Give each run its own `--out-dir`, or let forge suffix the run ID.

## Finish

1. Run all five gates from CLAUDE.md, checking each exit status directly rather than through `grep`.
2. Commit only when every gate is green. The commit message carries no AI-assistant reference.
3. Leave pushing to the lead agent, who reviews the diff first.
4. Work in the foreground and stop every background job before reporting.
5. Report once, at the end:
   - the values chosen;
   - the file's figures against the source's;
   - the misses;
   - the test count and each gate's exit status;
   - the commit SHA.
