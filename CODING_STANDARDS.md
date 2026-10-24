# Coding standards

Judgement calls for a reviewer reading a diff. Anything a tool can check is a test or a CI job instead (see the end).

## Preset bases

Every value in `data/presets/` carries a `basis`: the evidence a reader checks the value against. A reviewer reads every basis the diff adds or changes, and holds each to these rules.

**Allowed kinds.** A basis is one or more of:

- a real source, cited with its page, table or figure, plus the value it gives. A copyrighted source is cited by value and page only;
- `forge judgement: <why>`;
- `forge calibration to <source>: <what was fitted>`;
- an owner's practice note, cited as "owner's practice note (forge judgement)".

A citation or page that cannot be traced to a source or a findings file comes out.

**Traceable figures.** Every number in a basis appears in the cited source, in `docs/research/`, or in the findings the change was built from. A count from the BGS crawl is of files de-duplicated by content hash, since one accession is reachable by two paths.

**Quantifiers match their numbers.** "All", "every", "most" and "almost all" agree with the counts beside them. Write the count itself where the two could part: "23 of 32 sets", not "all sets". Two slips got this far before: #72 claimed all 32 sets where 23 held, and #80 said "almost all" next to a 12% no-rise share.

**Unit words match the unit.** A basis carried over from another unit has its unit names, figures and descriptions changed to the new unit's. #73 left "weathered till at 31 deg" on the red and grey tills. Bases pooled on purpose stay identical and say they are pooled.

**File figures are current.** A "the file gives X" figure is re-measured after any re-fit of a key it depends on, not only the key it sits under.

**Misses are stated.** Where the file cannot match the source, the basis says which figure misses and why, and the PR lists it under known misses.

## PR bodies

Each "proved by" names its tests in backticks by full `fn` name, so the claim can be checked against the branch.

## Checked by tools, not by review

- **NGDC attribution:** a basis citing the BGS NGDC carries "Contains data supplied by UKRI, OGL v3" (`every_shipped_basis_citing_the_bgs_ngdc_carries_the_ogl_attribution`).
- **Empty bases:** a basis that is missing or empty is refused by the preset loader.
- **Test names in a PR body:** each backticked test name must exist (CI job `pr-body`).
- **Pinned outputs:** the pin tests listed in CLAUDE.md.
