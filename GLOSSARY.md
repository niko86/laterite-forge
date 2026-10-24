# laterite-ags4-forge

Forge generates AGS4 files, checks each one with laterite and with python-ags4, and reports where the two disagree. It is also growing a site model, so that the files it generates carry believable synthetic ground data.

## Generation

**Scaffold**:
A named shape of generated file that fixes which groups appear and how they nest (`minimal`, `loca-samp`, `wide`).

**Divergence**:
A file on which laterite and python-ags4 reach different verdicts.
_Avoid_: mismatch, disagreement

## Site model

**Site model**:
The generated ground beneath a site: its units across the whole site, plus groundwater.
_Avoid_: geology model, ground model

**Preset**:
A named site description that a site model is generated from. Forge ships some presets, and a consumer can supply their own.
_Avoid_: site file, template

**Basis**:
The cited source for a range or parameter in a preset.
_Avoid_: reference, provenance

**Skew**:
An optional exponent on a unit's thickness range. The smooth field value is raised to it before being placed on the range, so a skew above 1 makes thin ground common and thick ground a long tail.
_Avoid_: bias, curve

**Suite**:
The groups a preset generates, and for each group the result headings it fills.
_Avoid_: test schedule

**Unit**:
One geological unit across the site, such as London Clay.
_Avoid_: layer, stratum (for this meaning)

**Stratum**:
The part of one unit met in one hole, recorded as one GEOL row.
_Avoid_: layer, unit (for this meaning)

**Made-ground unit**:
A unit whose preset entry sets `made_ground`. Its strata are described `MADE GROUND: ` + matrix + constituents. It takes no lab tests and never yields a water strike.
_Avoid_: fill, artificial ground

**Cover layer**:
What lies on the ground before a hole's first stratum: topsoil, hard surfacing or nothing, one keyed draw per hole from a preset's `[cover]` block. It takes its thickness off the top of the first stratum, so no other boundary, sample, test or strike moves. Nothing is sampled or tested in it.
_Avoid_: overburden, capping

**Topsoil**:
A cover layer worded "<colour> <minor fractions> TOPSOIL with rootlets." (for example "Dark brown slightly gravelly sandy TOPSOIL with rootlets."), from the topsoil lane of the BS 5930 engine. Not a unit and not made ground.
_Avoid_: soil layer, A horizon

**Surfacing**:
A cover layer of hard standing, a Made Ground stratum worded "MADE GROUND: Tarmacadam.", "MADE GROUND: Concrete." or "MADE GROUND: Paving slabs.", the kind drawn by weight.
_Avoid_: paving, pavement, hardstanding

**Water-bearing stratum**:
A stratum of a unit or lens whose preset entry sets `water_bearing`. A hole that meets one strikes water at its top, and the readings rise toward the water table, or stay at the strike when the table is deeper (perched water). Optionally a table with a `share` (0 to 1, cited, drawn once per site), the fraction of holes meeting the stratum that yield water in it; `true` is share 1. A hole that does not yield gets no strike from the stratum, and the water-table rule applies as before. Never a made-ground unit.
_Avoid_: aquifer, wet layer

**Till strike**:
A water strike a hole meets along its depth in a unit whose preset entry sets a `strike` table (`rate_per_m`, `rise_m`, `no_rise_share`, each cited). The hole is a Poisson process along its penetration of the unit, so the strike falls evenly through the stratum. Without `site.several_strikes` the shallowest of a lens-top strike and a till strike is the hole's one strike, and a water-table strike counts only when neither occurs. With it, the Poisson process runs on past its first event and every event is a strike, up to the preset's cap. The water always rises by the drawn rise, whatever the depth of the water table, but not above ground level. Never on a made-ground or water-bearing unit.
_Avoid_: till lens, partings (a till strike is not tied to a logged parting)

**Several strikes**:
A hole records every strike its sources give, shallowest first, when the preset sets `site.several_strikes` (`max_per_hole`, `min_gap_m`, cited). A strike closer than the gap to the one above is pushed down or dropped. A preset without it records at most one strike per hole.
_Avoid_: multiple strikes, strike group

**Seal**:
`WSTG_SEAL`, the depth to which a strike is sealed by casing, written when the preset's `[construction]` sets `seal`. A drawn gap below the strike, held at or above the next strike's casing or, for the last strike, the final casing.
_Avoid_: grout depth

**Refusal**:
An SPT stopped at 50 blows before the full 300 mm main drive, written as N 50 (not extrapolated), `ISPT_MAIN` 50, `ISPT_NPEN` 150 plus the penetration reached, and `ISPT_REP` "50/<mm>". Any drawn N of 50 or more is one (strength), and a unit may add a `spt.refusal_share` for cobble and boulder strikes (obstruction).
_Avoid_: capped N, extrapolated N

**Very coarse clause**:
The optional "with low cobble content" part of a description (BS 5930 Table 15), set by a unit's `very_coarse` constraint as a level for cobbles, boulders or both. It closes the soil description proper, and consumes no random draws.
_Avoid_: cobble tag, oversize

**Lens**:
A unit with limited lateral extent, lying within a single host unit. A hole that passes through a lens records its host as two strata, one above the lens and one below.
_Avoid_: band, pocket, inclusion

**Property model**:
How a unit's values are produced, from phase relations and correlations.

**Phase relation**:
An exact relationship between soil quantities, such as bulk density from moisture content, particle density and degree of saturation.

**Correlation**:
An empirical relationship between soil quantities, taken from a cited published source and applied with scatter.
_Avoid_: rule of thumb

**Relation**:
A named way of working out one quantity from others, such as `cu.linear_with_depth`. A preset picks a relation by name and cites the relation's parameters.
_Avoid_: model, formula

**Outlier**:
A stratum whose drawn property lies outside its unit's range, as real ground occasionally does.
_Avoid_: anomaly, bad value
