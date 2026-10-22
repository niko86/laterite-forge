---
status: accepted
decided: 2026-10-22
supersedes: docs/dec-forge-audience-boundary.md
---

# Forge owns a site model

`dec-forge-audience-boundary` (2026-08-25) made forge a description service
with a calibration side-job. It ruled out profile-derived depths and
strata-keyed values, on the grounds that the geological model belongs to the
consumer. Forge now owns that model. An opt-in `--site <preset>` generates
believable synthetic ground, mainly for demos and secondarily for realistic
perf fixtures. The default output does not change. This record supersedes the
boundary decision and gathers the decisions behind the new scope: the ones
settled on 2026-10-17 (niko86/laterite#1112, carried into #4) and the ones
settled on 2026-10-22. Terms are as defined in `GLOSSARY.md`.

## Why the boundary moved

The boundary held while realism meant better text in an `X`-typed field. Issues
#1–#3 asked for result values, multi-stage tests and test-aware sample types.
The prototype on `prototype/forge-results` showed that any of them, done
independently, produces impossible files (PL > LL), so values have to come
from a model of the ground. Having each consumer rebuild that model costs more
than forge owning it once, provided the site's story stays in a preset that a
consumer can replace.

## Ownership and contract

- Forge owns the machinery: the site-model engine and a catalogue of named,
  cited correlations and phase relations. A preset owns the site's story. It
  picks catalogue entries by name and sets their parameters. It never writes
  formulas.
- Every range row and parameter carries a cited `basis`. The loader refuses a
  preset with any row missing one, whether forge ships it or a consumer passes
  it in. CI runs the loader over the shipped presets.
- Ranges come from licensed open data (BGS under OGL, BRO under CC0) or cited
  papers. "Standards tables are never transcribed" rules out copying a
  standard's text and using a standard's typical-value tables as range sources.
  Boundaries that turn a value into a description term, such as SPT N bands or
  consistency-index bands, may be used as cited facts.
- A preset carries `schema` (unknown versions are refused) and `version`. The
  generated file records the preset's name and version. A version bump that
  changes output means notifying consumers and re-baselining, the same
  obligation as the `describe` pin. Old versions are not kept.
- Every `--site` file carries a mandatory synthetic marker in a field that
  survives a round-trip.

## Output and determinism

- With `--site` off, output stays byte-identical. A `--site` file is its own
  base, not the default file with extra columns, and each preset has its own
  byte pin, TYPE-set pin and clean-validation check. There is no standalone
  `--results` flag: values need a unit, and only a site model has units.
- `--site` and `--scaffold` are mutually exclusive. The preset's suite decides
  the groups, not the 123 groups of the `wide` scaffold.
- The suite lists groups, and for each group the headings beyond KEY and
  REQUIRED. The loader refuses a heading that no catalogue entry can produce.
  Under `--site` every value is either structural (keys, depths, IDs) or comes
  from a property model. Nothing is filled with arbitrary values by TYPE.
- The site model is a typed layer (site, units and surfaces, holes, strata,
  samples, tests) that is lowered into `ProjectModel`. It shares nothing with
  the scaffold builder, which keeps the scaffold pins out of reach.
- Each draw has its own RNG, keyed by seed, a domain tag, hole, unit and test.
  The preset version is not part of the key. Changing one unit changes only
  the rows that involve it, and a test asserts this.
- Hole k is identical whatever the hole count, so a smaller `scale` file is a
  prefix of a larger one, group by group.
- `--inject` and `--combine` run unchanged on a site-model base.

## Ground

- Units have smooth surfaces across the site, so neighbouring holes agree. A
  unit whose thickness at a hole is zero or less is absent there.
- A unit gives exactly one stratum per hole. Weathered and unweathered parts are
  separate units. The one exception is a lens: a unit with limited footprint
  inside a single host unit, which splits its host into a stratum above it and
  a stratum below it.
- The preset gives a final-depth range for each hole type and a sampling
  schedule for each unit. Samples never straddle a stratum boundary.
  `LOCA_FDEP` is emitted.
- Physics covers couplings, depth trends, overburden from unit weights, and a
  groundwater level. Stress history is deferred and will be per unit.
  Groundwater stays internal until values depend on it, and water strikes
  follow as a suite addition.
- Coordinates are on a local grid by default. `--grid bng --origin <E>,<N>`
  adds national-grid coordinates for tools that only plot BNG, at an origin the
  consumer chooses and is responsible for. No shipped preset carries a real
  origin.

## Values and descriptions

- The property model is the single source of truth at a given depth.
  `GEOL_DESC` takes its principal, secondaries and palette from the unit's
  description constraints. It takes its strength or density term from the
  model, not from an independent draw.
- For fine soils the term comes from the consistency index Ic = (LL − w) / PI,
  using cited BS EN ISO 14688-2 boundaries. That keeps the description in step
  with the LLPL and LNMC rows in the same file. Undrained strength follows its
  own correlation. For coarse soils the term comes from SPT N.

## Presets

`london-basin` comes first, starting in natural ground: River Terrace
Deposits, weathered London Clay, London Clay. Made Ground waits for a
description lane for man-made ground, because the BS 5930 engine has none and
a hand-written placeholder would be the only unconstrained text in the file.
A glacial preset follows.

## Slices

1. Site layer and preset contract: typed layer, keyed draws, loader with
   `schema`, `version` and the `basis` gate, units and surfaces, local grid and
   `--grid bng`, `LOCA_FDEP`, strata, sampling schedule, synthetic marker,
   `--site` excluding `--scaffold`, prefix stability, per-preset pins.
2. Property-model core: catalogue, index properties and phase relations,
   overburden and groundwater, and the strength or density term in `GEOL_DESC`.
3. Result headings (#1).
4. Multi-stage tests (#2).
5. Test and sample-type selection by unit (#3).
6. Lenses.
7. Water strikes.
8. A description lane for man-made ground, then Made Ground in `london-basin`.
9. A glacial preset.

`--site` stays hidden from `--help` and `README-cli.md` through slice 2,
because its descriptions are incomplete on purpose and every slice re-rolls the
preset pins. It becomes visible and documented with slice 3, and the preset
versioning promise starts at that point.

## Considered and rejected

- **Real national-grid coordinates by default.** A believable log on real land
  can be mistaken for real site data. No widely recognised coordinate system
  exists for a fictional place, so the honest default is a local grid.
- **Formulas written in presets.** That would make the preset format a
  language, with citations and tests that forge can't check.
- **Reading the standards rule strictly** (no numbers from standards at all).
  Forge couldn't turn a value into a description term, and it would have to
  remove the SPT N bands it already vendors.
- **Consistency from undrained strength.** BS 5930:2015 treats consistency as a
  field description, separate from measured strength. The consistency index
  ties the term to index tests that are in the same file.
