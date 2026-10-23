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
