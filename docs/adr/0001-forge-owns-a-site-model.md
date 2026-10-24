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
  published sources. A textbook or report counts: the basis names it as found
  (title, edition, chapter, printed page, table or figure) and adds the
  original paper wherever the book names one. A value that describes the
  synthetic site rather than the ground (a local datum, an investigation
  choice) or a modelling choice (a negative thickness minimum for patchy
  cover) carries the basis `forge judgement`, so a reader can tell choices
  from data. (Amended 2026-10-22, #24.) "Standards tables are never transcribed" rules out copying a
  standard's text and using a standard's typical-value tables as range sources.
  Boundaries that turn a value into a description term, such as SPT N bands or
  consistency-index bands, may be used as cited facts.
- A preset carries `schema` (unknown versions are refused) and `version`. The
  generated file records the preset's name and version. A version bump that
  changes output means notifying consumers and re-baselining, the same
  obligation as the `describe` pin. Old versions are not kept. The promise
  starts at `london-basin` v3 (see Slices).
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
- A unit's `thickness_m` may carry an optional `skew`, a number of at least 1. The
  smooth field value u in [0, 1] is raised to that power before it is placed on
  the range, so a skew above 1 puts most of the site near `min` with a long
  tail toward `max`, as real thicknesses are right-skewed. With a negative `min`
  this also sets the absent fraction, so a range and skew can approach a
  cited P10/P50/P90 plus an absence rate (a fit, not an exact match: the field
  is compressed about 0.5). The map is monotone and continuous, so
  neighbouring holes still agree; its slope is at most `skew` times the
  field's, so the smoothness bound scales by the skew. The skew is part of the
  range and needs no basis of its own beyond the range's. It is allowed on
  unit thickness only: lens thickness is not drawn from a field, and other
  ranges have no use for it. An absent skew is the linear mapping, with the
  same arithmetic and no extra draws, so existing presets are unchanged.
  The loader refuses a skew that is not a number or is below 1, where the
  map's slope would be unbounded near 0.
- A unit gives exactly one stratum per hole. Weathered and unweathered parts are
  separate units. The one exception is a lens: a unit with limited footprint
  inside a single host unit, which splits its host into a stratum above it and
  a stratum below it.
- The preset gives a final-depth range for each hole type and a sampling
  schedule for each unit. Samples never straddle a stratum boundary.
  `LOCA_FDEP` is emitted.
- Physics covers couplings, depth trends, overburden from unit weights, and a
  groundwater level. Stress history is deferred and will be per unit.
  Groundwater stays internal until values depend on it. Water strikes are a
  suite addition: `WSTG` and `WSTD` rows (no dates, the model has no clock)
  where a hole's water table falls in a coarse stratum, the strike a strike
  rise (the drilling lag) below the table, rising to the table over 20
  minutes. The preset may give the rise as `site.strike_rise_m`; without it
  the default is 0-0.5 m (`forge judgement`).
- Water-bearing strata. A unit or lens may set `water_bearing = true` (an
  additive part of schema 1; the loader refuses it on a made-ground unit and
  on anything but a boolean or a share table). Thin glaciofluvial sand and gravel lenses in till are
  frequently water-bearing, perched or sub-artesian (CIRIA C504 Fig 6.3,
  p.117; Eyles & Sladen 1981, p.134), and a thin lens rarely contains the
  table, so the table rule almost never strikes there. A hole that meets a
  water-bearing stratum strikes at its top, the first one going down. The
  strike rise is then the rise of the water, not a drilling lag: the readings
  rise from the strike to `max(table, strike - rise)`, the rise drawn from
  `site.strike_rise_m` or its default with the same keyed draw. Where the
  table is at or below the strike, the water is perched and the readings stay
  at the strike depth. A hole records one strike, and the water-bearing rule
  takes precedence over a table strike in a shallower coarse stratum (AGS
  allows several `WSTG` rows per hole; one keeps the file shape and the draw
  count of the table rule, `forge judgement`). From tyneside-till v8 a till
  strike joins the water-bearing strike in that precedence (below). Without
  the flag nothing moves.
  Not every hole that meets such a stratum records a strike in it (a BGS
  crawl of NE England found 89 of 249 holes meeting a granular layer, 36%), so
  the flag may instead be a table, `water_bearing = { share = { min, max,
  basis } }` (or `{ share = 0.36, basis = "..." }`), the share read like a
  made-ground share and refused outside 0 to 1, without a basis, or on a
  made-ground unit. The share is drawn once per site in its range from a
  stream keyed by the unit or lens id. Each hole that meets the stratum then
  draws once, keyed by hole and stratum, and yields water when the draw is
  under the share. A hole that does not yield gets no strike from that stratum
  (the next water-bearing stratum down is tried, then the table rule). `true`
  is share 1 and makes no draw, so it stays byte-identical.
- Water strikes in till (#77, tyneside-till v8). A unit may take a `strike`
  table: `rate_per_m` (strikes per metre penetrated, drawn once per site in its
  range), `rise_m` (a cited range with an optional skew, as a thickness has)
  and `no_rise_share` (0 to 1). Each is cited, and the loader refuses a rate or
  rise below 0, a share outside 0 to 1, a missing basis, and the table on a
  made-ground unit (which never yields water) or a water-bearing unit (which
  already strikes at its top). A lens takes none. In the crawl of NE England
  (793 holes, 38 projects) 129 of 437 strikes are in till, in 112 holes
  (18.7% of the 598 that go through till), yet the tills gave no strike before
  v8, so only about 7% of generated holes struck against 41% in the crawl. The
  strikes follow penetrated thickness, about 0.3 per 10 m of till, not a flat
  share per hole.
  - **Placement.** A hole is a Poisson process along its penetration of each
    such unit, keyed by (hole, unit): the first event below the written top of
    the unit's stratum (below the cover, and continuing one stream across the
    pieces of a unit a lens splits) is the strike, so it falls evenly through
    the stratum, as the crawl's strikes do (median 0.43 to 0.62 of the way
    down). The rate is drawn once per site from a stream keyed by the unit id.
    The rates are fitted to the share of holes through each unit that strike in
    it, not to the crawl's per-metre rate, because the units differ in what the
    holes cross: the grey till runs to final depth, so the file's holes cross
    19 m of it against the crawl's 4.6 m.
  - **One strike per hole.** The shallowest of a lens-top strike and a till
    strike wins (the lens on a tie), and a water-table strike in a coarse
    stratum counts only when neither occurs. In the crawl the till strike is
    the hole's first in 74% of holes that have one. The GFSG lens share is
    raised from 0.36 to 0.44 so that, with the shallower till strike taking
    precedence in some holes, the holes with the lens strike in it at the
    crawl's 36%. Several strikes per hole remain out of scope.
  - **Rise.** Unlike the lens rule, the table bounds nothing. The settled
    level is `max(0, strike - rise)`, the rise drawn from the unit's `rise_m`
    (a stream keyed by hole and unit), and a draw under `no_rise_share` (drawn
    once per site) leaves the water at the strike. The head on a sand or silt
    parting in a clay till is not the site water table, and the crawl's till
    strikes almost all rise (1 of 90 fell, 12% none; `forge judgement`). The
    readings follow the same decay as every strike, so they
    are monotone and settle exactly at the 20-minute level. The tills carry
    the same pooled rise (crawl n 90, 17 projects), because weathered till has
    10 readings.
  - **Basis.** The crawl, plus the owner's practice note (`forge judgement`):
    strikes in clay till come from thin sand or silt partings and fissures too
    small to log separately. Only 5 to 14% of the crawl's till strikes have a
    logged parting. The note sits beside the lens citations above (Eyles &
    Sladen 1981, p.134; CIRIA C504 Fig 6.3, p.117), which are about lenses.
  - **Keys.** New tags `till-strike-rate`, `till-strike-norise`,
    `till-strike` and `till-strike-rise` draw the new values and no existing
    stream is consumed, so a preset without the table writes the bytes of the
    version before it, and changing one unit's rate moves only the strike-
    dependent rows (`WSTG`, `WSTD`, `WSTG_CAS`, the installation zone, `BKFL`
    and `PIPE`) of holes whose strike changes.
- Coordinates are on a local grid by default. `--grid bng --origin <E>,<N>`
  adds national-grid coordinates for tools that only plot BNG, at an origin the
  consumer chooses and is responsible for. No shipped preset carries a real
  origin.

## Values and descriptions

- The property model is the single source of truth at a given depth.
  `GEOL_DESC` takes its principal, secondaries and palette from the unit's
  description constraints. It takes its strength or density term from the
  model, not from an independent draw.
- A description may carry a very coarse clause (BS 5930 Table 15), written
  `very_coarse = { cobbles = "low", boulders = "medium" }` inside the
  description constraints. Each side is optional but one is required, and each
  is a level: `low`, `medium` or `high`. Cobbles and boulders are separate keys
  because they are separate particle sizes in the vendored data, and a unit
  may want only one. The levels are the three vendored band qualifiers, not
  percentages, so nothing is drawn: the clause is text from the constraint,
  consumes no RNG, and a unit without the key is byte-identical. The loader
  refuses an unknown level or key and an empty table, naming the field. It is
  an additive part of schema 1, and units and lenses both take it.
  - Wording is the vendored qualifier. Both sides at one level read as the
    vendored band as it stands, "with low boulder or cobble content". Sides at
    different levels, or only one side, swap "boulder or cobble" for the
    fraction named, cobbles first: "with low cobble content and medium boulder
    content". The vendored data gives only "with low cobble content" as a
    per-fraction example, so the other per-fraction wordings are the forge's
    derivation from the band template. Norbury 2010, Table 8.4 (p.112), sets
    cobble and boulder content at different percentage limits, which is why
    the two can differ; the forge does not apply those limits.
  - Placement: the clause closes the soil description proper, before the first
    full stop and so before any grading sentence. A made-ground unit's
    constituents follow it, giving "MADE GROUND: Loose dark brown GRAVEL with
    low cobble content with occasional brick fragments." Constituents are the made-ground
    lane's own, appended last as before.
- For fine soils the term comes from the consistency index Ic = (LL − w) / PI,
  using cited BS EN ISO 14688-2 boundaries. That keeps the description in step
  with the LLPL and LNMC rows in the same file. Undrained strength follows its
  own correlation. For coarse soils the term comes from SPT N.
- A fine unit's properties are drawn as plastic limit, plasticity index and
  liquidity index; liquid limit and moisture content are derived from them
  (LL = PL + PI, w = PL + LI·PI). Drawing PL rather than LL keeps it from
  going negative, contradictions such as PL > LL cannot occur, and the
  consistency index (1 − LI) comes from a cited range rather than from
  chance. A coarse unit draws SPT N and moisture content.
- Each property is a smooth field across the site mapped onto its cited
  range, plus an optional cited depth gradient. About one stratum in fifty
  takes an outlier on a drawn quantity, beyond its range by 25–75% of the
  range's width. Outliers come from their own keyed stream and never break
  the couplings, because the derived quantities are derived after them.
- In schema v1 a preset names catalogue parameters, not catalogue relations,
  wherever a job has one relation. Where it has alternatives, the preset
  picks one by name: a unit's properties may carry `strength = { relation =
  "cu.linear_with_depth", ... }` and `spt = { relation = "spt.stroud_butler",
  ... }`, each with that relation's cited parameters. The loader refuses an
  unknown name, listing the known ones. Only the linear-with-depth cu relation
  is registered; the EC7 OCR form waits until a preset has a cited removed
  overburden. (Amended 2026-10-22, #9; amended #51.)
- **SPT refusals** (#63). A test that reaches 50 blows before the 300 mm main
  drive is a refusal, not an N. The ISPT suite may list `ISPT_MAIN`,
  `ISPT_NPEN` and `ISPT_REP` (only with `ISPT_NVAL`, which the loader
  enforces). An ordinary test writes `ISPT_MAIN` = N, `ISPT_NPEN` = 450 and
  `ISPT_REP` = "N=<N>". A refusal writes `ISPT_NVAL` = 50, not extrapolated,
  `ISPT_MAIN` = 50, `ISPT_NPEN` = 150 mm of seating plus the main-drive
  penetration p reached, and `ISPT_REP` = "50/<p>". That convention is forge
  judgement (owner decision, #63). Two causes: any drawn N of 50 or more is a
  refusal, always, with p = 300 x 50 / N (at least 1, under 300); and a unit
  or lens may carry `spt.refusal_share` (`{ min, max, basis }` or `{ share,
  basis }`), the share of its tests stopped by a cobble or boulder strike
  whatever the strength, with p uniform in 25 to 275 mm. The share is drawn
  once per site from a stream keyed by the unit, and each test then has its
  own keyed draw, so a share moves only ISPT rows. The share may instead rise
  linearly with depth, `{ top = { min, max, basis }, per_m = { min, max, basis
  } }`: share(z) = clamp(top + per_m x z, 0, 1), z metres below ground, `top`
  and `per_m` each drawn once per site from their own keyed streams (a `top`
  below 0 keeps the share at 0 near the surface). It is allowed on a coarse
  unit or one whose description has `very_coarse`, and the loader refuses it
  elsewhere, naming the field. No N above 50 is ever
  written, so the change moved every shipped preset (london-basin v5,
  tyneside-till v2).

## Made-ground units

(Added 2026-10-23, #42; the decisions are the owner's M1-M6 on that issue.)

- A unit declares itself made ground with `made_ground = true` (the lane's
  default draw) or a table: `made_ground = { constituents = [{ name, share }],
  count = { weights, basis } }`. Both parts are optional, and the key is an
  additive part of schema 1. Lenses cannot be made ground.
- Its strata are `MADE GROUND: ` + a matrix + constituents. The matrix comes
  from the same `describe_unit` draw a plain unit makes, inside the unit's
  description constraints and with the leading term from the property model,
  so a made-ground unit's matrix is the one it would have as a plain unit. The
  constituents come from the made-ground lane on a stream of their own.
- `share` is the fraction of the unit's strata that mention the constituent: a
  `{ min, max, basis }` range, drawn once per site, or a plain number with a
  `basis`. `count.weights` are relative weights for 0, 1, 2 and 3 or more
  constituents. The loader refuses an unknown constituent name, a share outside
  0-1, a missing `basis`, and count weights that are not four non-negative
  numbers with a positive sum. A count of 0 gives no `with` clause. The `3+`
  bucket draws 3, 4 or 5 with equal chance (`forge judgement`).
- `amounts = { weights = [rare, occasional, frequent, numerous, abundant],
  basis }` weights the amount word. Each constituent draws its own amount
  independently, so amounts repeat (without `amounts` they are uniform); the
  parts then run from the most to the least frequent amount, the most-shared
  constituent taking the most frequent, and neighbours sharing an amount share
  its word ("occasional brick and concrete"). The loader checks the weights
  like `count`'s: five non-negative numbers, not all zero, with a `basis`.
- A stratum draws its count, then that many distinct constituents by
  conditional Poisson sampling, the tilt fitted so each constituent's mention
  share comes out as the preset gives it. Shares and counts only agree if the
  shares sum to the mean count, so a preset reconciles them; where they differ
  the fit is a compromise.
- Made Ground never yields a water strike, whatever its properties (perched
  water is out of scope), and the loader refuses a suite that lists LNMC, LLPL,
  LDEN or TRIG over a made-ground unit: no lab tests in Made Ground. ISPT is
  allowed. Sampling is the unit's own, as for any unit.
- The lane's default draw (no preset) is unchanged, and so is any preset
  without a made-ground unit. `pottery` and `coal` join the vocabulary but not
  the default draw's candidates.

## Cover layer

(Added 2026-10-23, #58; the decisions are the owner's Q1-Q5 on that issue.)

- **Cover layer.** A preset may carry a `[cover]` block with a `surfacing`
  layer, a `topsoil` layer, or both. Each layer has a `share` (`{ min, max,
  basis }`, drawn once per site from a stream of its own), a `thickness_m`
  (`{ min, max, skew?, basis }`, above 0) and, for surfacing, weighted `kinds`
  with a `kinds_basis`, for topsoil a `description` (weighted `colours`, the
  `secondaries` with the chance each is named, and a `basis`). Each hole makes
  one keyed draw `u` in 0 to 1: surfacing when `u` is under the surfacing
  share, topsoil when `u` is at least one less the topsoil share, else
  nothing. The loader refuses shares whose maxima sum above 1, so a hole has
  at most one layer and a share moves only its own layer's holes. Thickness is
  `min + (max - min) x v^skew` for a uniform `v` from a third stream, rounded
  to 2 decimal places. A preset without `[cover]` draws nothing and keeps its
  bytes.
- **Where the cover goes.** The cover takes its thickness off the top of the
  hole's first stratum. In the file the cover is a GEOL row from 0 to its
  base and the first stratum is written from that base; every other boundary,
  every sample, test and strike below the base, and the first stratum's own
  description, are where they were without the cover. The first stratum keeps
  at least 0.05 m: a thicker cover is cut back to leave that, and one with no
  room is absent (the alternative, eating into the next stratum, would move a
  boundary the decision keeps fixed). The loader holds each layer's thickness
  maximum below the shallowest water table the site allows, so a strike is
  never inside the cover; a water-bearing first stratum strikes at the cover
  base.
- **Descriptions.** Surfacing is `MADE GROUND: <kind>.` with the kind drawn
  by weight from Tarmacadam, Concrete and Paving slabs. Topsoil is its own
  small lane, `<Colour> <fractions> TOPSOIL with rootlets.`, with the minor
  fractions in the BS 5930 engine's fine-soil banding (slightly, plain, very;
  least first), sharing an 80% budget; the `describe` command does not gain
  it and the default draw is unchanged. The format follows Norbury 2010's
  Made Ground convention (ch. 14, pp.208-214) and is forge judgement.
- **Testing.** Nothing is sampled or tested in the cover. A sample whose top
  would fall in it moves to the cover base if it then ends inside the stratum
  and before the next sample starts (only the first such sample can move), and
  is dropped otherwise. A sample keeps its place in the hole's sampling
  schedule (`rank`) whatever is removed above it, so `SAMP_REF` has a gap
  where one was dropped and every test and result below the cover keeps its
  keyed draw. london-basin v6 and tyneside-till v3 adopt the layer.

## Multi-stage UU

(Added 2026-10-23, #2; the decisions are the owner's Q1-Q4 on that issue.)

- **Scope.** Multi-stage unconsolidated-undrained triaxial only: a TRIG test
  with `TRIG_TYPE = "UUM"` has three TRIT rows, `TRIT_TESN` 1 to 3, on one
  specimen. Effective-stress TREG/TRET is a separate slice.
- **Share.** A unit or lens may carry `triaxial = { multi_stage_share = { min,
  max, basis }, stage_gain = { min, max, basis } }`. The share is drawn once
  per site from a stream keyed by the unit; each TRIG test then makes its own
  keyed draw (tag `trig-uum`, on hole, unit and sample) on whether it is
  `UUM`. A preset with no `triaxial` writes every test as `UU` with one row
  and keeps its bytes; a share moves only its own unit's TRIG type and TRIT
  rows. The loader refuses a share or gain outside 0 to 1, a `stage_gain`
  without a `multi_stage_share`, a missing basis, and a `triaxial` on a unit no
  TRIG entry tests.
- **Stages.** The cell pressure is 1, 2 and 4 times the single-stage value.
  cu at stage k is the written stage-1 cu x (1 + `stage_gain` x (k - 1)),
  rounded to the kPa, with `stage_gain` drawn once per site (0 when absent);
  `TRIT_DEVF` stays twice the written cu. `TRIT_BDEN` and `TRIT_IMC` are
  drawn once per specimen and repeated on every stage. Stage 1 is exactly the
  row a single-stage test writes. tyneside-till v4 adopts it (the owner's note
  that labs run UUM where three matched 38 mm specimens cannot be cut from a
  matrix-bound gravelly till is the basis, alongside the NE crawl's count);
  london-basin has no source and none.

## Effective-stress triaxial

(Added 2026-10-24, #68; the decisions are the owner's Q1-Q5 on that issue.)

- **Scope.** TREG sets and their TRET stages, for a preset that gives a unit
  `effective_strength`. tyneside-till v5 adopts it; london-basin waits for a
  verified peak c' and phi' source, and the London crawls have no TREG data.
- **Properties.** A unit or lens may carry `effective_strength = {
  c_eff_kpa, phi_eff_deg, skempton_af }`, each `{ min, max, basis }`, all three
  required, drawn once per site from a stream keyed by the unit (tags
  `eff-c`, `eff-phi`, `eff-af`). The loader refuses c' outside 0 to 25 kPa (the
  top of CIRIA C504's range for clay tills, Trenter 1999, p.85), phi' outside 0
  to 50 degrees, A_f outside -0.5 to 1.5, a range pair whose lowest A_f and
  highest phi' leave no deviator stress that reaches the envelope, a missing
  basis, an unknown key, a TREG entry over a unit without `effective_strength`,
  and an `effective_strength` on a unit no TREG entry tests. TREG is a lab
  group, so Made Ground may not be tested for it. A TRET entry needs a TREG
  entry and has no `units` or `rate` of its own.
- **Sets.** Each sample the TREG entry schedules (as TRIG schedules, its own
  rate and keyed draws; TREG and TRET come last in the suite order so no earlier
  group's draws move) makes a set. Its type is drawn by the crawl's counts, CUM
  22, CDM 15, CU 10, CD 10 of 57. Its c' and phi' scatter about the site's
  values (SD 2.0 kPa and 1.5 degrees, forge judgement). `CUM` and `CDM` are
  one specimen sheared in three stages, with density and moisture content
  repeated. `CU` and `CD` are three specimens, each with its own; the crawl's
  files write them under one TREG row, one `SPEC_REF` and `SPEC_DPTH`, and
  `TRET_TESN` 1 to 3, and so does the model.
- **Stages.** Stage 1's effective confining stress is 10 kPa per metre of
  specimen depth, to the kPa; stages 2 and 3 are exactly twice and four times
  that. The back pressure is 300 kPa: `TRET_PWPI` is 300, `TRET_CELL` is the
  stress plus 300 and `TRET_CONP` is the stress.
- **Failure.** The deviator stress q at failure puts the effective failure
  point on the envelope t = c' cos(phi') + s' sin(phi'), with s' = sigma3' +
  q (1/2 - A) when the pore pressure rises by A q, which gives q = 2 (c'
  cos(phi') + sigma3' sin(phi')) / (1 - sin(phi') (1 - 2 A)). A drained stage
  (CD, CDM) has A = 0, no pore pressure change (`TRET_PWPF` = `TRET_PWPI`)
  and q scattered by a lognormal factor of COV 0.03 (the crawl's failure points
  sit within P10 -4% and P90 +3% of their sets' envelopes). An undrained stage
  (CU, CUM) draws A_f about the site's value (SD 0.10, forge judgement), fails
  on the envelope exactly, and writes `TRET_PWPF` = `TRET_PWPI` + A_f x
  `TRET_DEVF`.
- **TREG headings.** `TREG_COH` and `TREG_PHI` are the least-squares line
  through the set's three written failure points in (s', t), as a laboratory
  reports them (the crawl's reported values match such a line to about half a
  degree), so the TREG row and its TRET rows agree; a line with a negative
  intercept is refitted through the origin (c' = 0), and c' is held within 0 to
  25. `TREG_COND` is `UNDISTURBED`. `TREG_FCR` is "Maximum effective principal
  stress ratio" for CUM, as the crawl's lab 169655 writes it, and "Maximum
  Deviator Stress" for the others.
- **v5.** tyneside-till takes ranges calibrated to the crawl's per-unit means
  and checked against Hashemi 2002 and CIRIA C504 (see
  `docs/research/tyneside-till-sources.md`), and a TREG rate of 0.48 to 0.68,
  0.97 of its TRIG rate. Versions of a preset without `effective_strength`
  and a TREG entry keep their bytes.

## Hole construction

(Added 2026-10-24, #74; the decisions are the owner's R1 Q1-Q2 and R2 Q1-Q7.
This is the first of two slices: holes with no installation. Pipes and
standpipes, with their backfill, are the second, below (#75).)

- **Block.** A preset may carry a `[construction]` block: an
  `inspection_pit_m` range, a `[construction.casing]` table (weighted
  `diameters` with a `diameters_basis`, `telescope_share`,
  `telescope_depth_fraction`, `final_gap_m`, `passes_strike_share` and
  `strike_gap_m`) and a `[construction.backfill]` table (`site_share`, a
  `sequence` of three weights with a basis, and `boundary_m`). Every range has
  a basis. Shares are in 0 to 1, gaps are not negative, the telescope fraction
  is strictly between 0 and 1, and the diameters are whole millimetres, each
  once, with weights that sum above 0. The pit and boundary maxima must be
  shallower than the shortest final depth any hole type allows, and every hole type's code must be an `HDPH_TYPE` pick-list code other than
  `IP`. A preset without the block draws nothing and keeps its bytes.
- **HDPH.** Two rows per hole: `IP` from 0 to the pit depth (a keyed draw per
  hole; 1.2 m in both shipped presets), then the hole's type (`CP`) to final
  depth, so the phases tile the hole. `LOCA_TYPE` is the phases joined by `+`
  (`IP+CP`), as the crawls write it. Both the Rust validator and python-ags4
  accept the joined code when `ABBR` lists each part, so `ABBR` is built from
  the parts (`collect_abbr` splits a value that is not itself a pick-list code
  when every part is one). There is no `HDIA`: it is rare in the crawls.
- **CDIA.** Every hole is cased. The first string's diameter is drawn by
  weight; with probability `telescope_share` (drawn once per site; it applies
  only where a smaller diameter exists in the list) the hole steps down to a
  smaller diameter, drawn by weight among the smaller ones, ending the larger
  string at `telescope_depth_fraction` of the way from the pit base to the
  final casing. A diameter never increases with depth, and the final casing is
  never shallower than the pit base or deeper than final depth.
- **Casing and the strike.** The final casing depth is the final depth less a
  gap drawn from `final_gap_m` (`min + (max - min) x u^skew`). With
  probability `passes_strike_share` (drawn once per site) the casing passes
  the strike, and the gap is drawn from the same distribution cut off where the
  casing would end at or above the strike (the same uniform, scaled, so it is
  the distribution conditioned on passing). Otherwise the casing ends at a
  uniform depth between the pit base and the strike. A hole with no strike
  draws the cut-off at the pit base.
- **`WSTG_CAS`.** Every strike gets one: the strike depth less a gap from
  `strike_gap_m`, rounded to 2 decimal places, never below 0 and never deeper
  than the strike or the final casing. `WSTG_SEAL` is left out. The column is
  written only in a preset that has both a `WSTG` suite entry and a
  construction block, and `WSTG` rows are what they were otherwise.
- **BKFL.** Each site makes one keyed draw on whether its holes carry BKFL
  (`site_share` is itself drawn once per site in its range). If they do, each
  hole draws a sequence by weight: arisings alone, bentonite alone, or arisings
  over bentonite with the boundary from `boundary_m`. The layers tile 0 to final
  depth exactly, in `BKFL_LEG` legend codes 905 (arisings) and 903 (bentonite)
  only, with no `BKFL_DESC`. The backfill of an installed hole replaces it
  (see "Installations" below).
- **Draws and order.** Every construction draw has a `cons-` tag of its own
  (`cons-pit`, `cons-casing`, `cons-diameter`, `cons-strike-casing`,
  `cons-backfill`, and the site-level `cons-telescope-share`,
  `cons-pass-share`, `cons-backfill-share`, `cons-backfill-site`), each a fixed
  number of draws, so hole k is the same whatever the hole count and changing the
  block moves only construction rows (HDPH, CDIA, BKFL, the `WSTG_CAS` column and
  `LOCA_TYPE`). The groups are written after the suite and are not suite
  entries, so no existing draw is re-keyed. Plan: `src/site/construction.rs`.
- **Dictionary values only.** Codes the crawls write that are not in the
  dictionary's pick lists (HDPH_TYPE COM, SB, SC; BKFL_LEG 001-005) are never
  written.
- **Versions.** london-basin v7 and tyneside-till v6 adopt it, calibrated to
  the London and NE crawls (`docs/research/*-sources.md`): casing diameters and
  step-down share, the final casing gap, the rate at which the casing passes the
  strike, the `WSTG_CAS` gap, the BKFL site share and sequence weights.
  Versions without the block keep the previous bytes.

### Installations

(Added 2026-10-24, #75; the decisions are the owner's R1 Q1-Q6.)

- **Table.** A `[construction]` block may carry `[construction.installation]`.
  It sits inside the block because an installation needs the pit, casing and
  backfill parts; one without them is refused for what is missing. Keys:
  `site_share`, `hole_share` (shares in 0 to 1), `kinds` (an array of
  `{ kind, mm, weight }` with `kind` `standpipe` or `piezometer`, each once, a
  diameter of 19 or 50 mm and a weight above 0) with `kinds_basis`,
  `zone_offset_m`, `zone_top_m`, `slotted_length_m`, `base_gap_m`, `cap_m`
  (lengths in metres; all above 0 except the offset, which may be negative,
  and the gap, which is not negative) and `filter` (two weights, gravel and
  sand, with a basis).
  Every range has a basis; an unknown key is refused. The shortest final depth
  must exceed the pit, cap, seal, longest slotted section and filter and tail
  together. A preset without the table keeps its bytes.
- **Which holes.** A site draws once whether it installs at all (`site_share`:
  0.53 tyneside-till, 10 of 19 NE projects; 0.17 london-basin, 3 of 18
  projects), then once a share of its holes (`hole_share`, 0.43 to 1.0), and
  each hole draws against it. One installation per hole.
- **Kinds.** Drawn by weight per preset. A standpipe is `PLAIN` from 0 to the
  zone top and `SLOTTED` through the zone; a piezometer is `PLAIN`, then a
  0.3 m tip written as `SLOTTED`, because the dictionary's `PIPE_TYPE` has only
  those two values. `PIPE_DIAM` is 50 or 19, `PIPE_REF` is "1". Craig's Soil
  Mechanics, 7th ed., pp.405-406 describes the construction (a standpipe of
  50 mm or smaller with a perforated end or porous tip, a sand or fine gravel
  filter, a bentonite seal above and a second seal near the surface).
- **Response zone.** With a strike in the file the zone top is the first
  strike plus a draw from `zone_offset_m`; with none it is a draw from
  `zone_top_m`. The slotted length is a draw from `slotted_length_m`
  (a piezometer's is 0.3 m). The zone is then clamped, not resampled, into the
  room the hole leaves: not above the pit base, not so high that the cap,
  a seal of at least 0.1 m and 0.5 m of filter do not fit above it, and not so
  low that the zone base is within 0.6 m (0.5 m of filter and at least 0.1 m
  of bentonite) of the final depth. `base_gap_m` is the drawn gap from the
  final depth to the zone base, a ceiling on the zone: the base is at most the
  final depth less the gap, raised to that 0.6 m. No stratum is targeted.
  Depths are worked in whole centimetres.
- **Backfill.** An installed hole's `BKFL`, top down: concrete (906) from the
  surface to the cap thickness, bentonite (903) seal, a gravel (902) or sand
  (901) filter from 0.5 m above the slotted section to 0.5 m below it, bentonite
  (903) to final depth. The layers tile 0 to final depth exactly and none has
  zero thickness, because the clamp above leaves room for each. An installed
  hole always writes `BKFL`, even on a site whose #74 draw writes none; the
  site's other holes keep their #74 sequences. Legend codes only, no
  `BKFL_DESC`.
- **No `MONG`.** It only restates `PIPE` (157 of 158 NE `MONG` holes). A later
  slice for monitoring readings would add `MONG` and `MOND` together.
- **Draws and order.** New draws have their own tags (`cons-install-site`,
  `cons-install-site-share` and `cons-install-share` for the site, and
  `cons-install-hole`, `cons-install-kind`, `cons-install-zone` and
  `cons-install-filter` per hole), each a fixed number of draws, so no existing
  stream moves and hole k is the same whatever the hole count. `PIPE` is
  written after `BKFL`, so no existing group's rows move. The #74 backfill draw
  is still made for an installed hole, so installing a hole moves only its own
  rows.
- **Versions.** london-basin v8 and tyneside-till v7 adopt it, calibrated to
  the crawls (`docs/research/*-sources.md`). Both are re-pinned.

## Presets

`london-basin` comes first, starting in natural ground: River Terrace
Deposits, weathered London Clay, London Clay. Made Ground waited for a
description lane for man-made ground, because the BS 5930 engine had none and
a hand-written placeholder would be the only unconstrained text in the file.
The lane exists and a preset can now declare made-ground units (see
"Made-ground units"); `london-basin` adopts it in v4 (#42), the first
version bump under the promise in "Slices".
A glacial preset follows.

## Slices

1. Site layer and preset contract: typed layer, keyed draws, loader with
   `schema`, `version` and the `basis` gate, units and surfaces, local grid and
   `--grid bng`, `LOCA_FDEP`, strata, sampling schedule, synthetic marker,
   `--site` excluding `--scaffold`, prefix stability, per-preset pins.
2. Property-model core: catalogue, index properties and phase relations,
   overburden and groundwater, and the strength or density term in `GEOL_DESC`.
3. Result headings (#1).
4. Multi-stage tests (#2), and effective-stress triaxial (#68).
5. Test and sample-type selection by unit (#3).
6. Lenses.
7. Water strikes.
8. A description lane for man-made ground, then Made Ground in `london-basin`.
9. A glacial preset.

`--site` stays hidden from `--help` and `README-cli.md` through slice 2,
because its descriptions are incomplete on purpose and every slice re-rolls the
preset pins. It becomes visible and documented with slice 3, and the preset
versioning promise starts at that point.

The shipped preset first carried a suite (result headings, strength and water
strikes) in `london-basin` v3 (#48), so `--site`, `--grid` and `--origin`
became visible in `--help` and `README-cli.md` there, and the promise starts at
`london-basin` v3: from v3 on, any change to what a shipped preset generates
bumps its `version`, and consumers are told and re-baseline. v1 and v2 carry
no such promise. (Amended 2026-10-23, #48.)

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
