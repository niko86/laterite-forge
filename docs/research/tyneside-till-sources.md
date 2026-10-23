# tyneside-till: sources and calibration

This records where the values in `data/presets/tyneside-till.toml` come from,
how they were calibrated, and what the generated files measure (v1, #13).
It holds aggregate statistics and accession IDs only: no raw data, no
scripts, and no individual borehole rows. The papers cited are copyrighted,
so they are cited by value and page or table only.

## The site

Late Devensian lowland till of NE England (Tyneside, Northumberland,
Durham). The engineering literature describes it as tripartite (Clarke,
Hughes & Hashemi 2008, Fig. 3, p.68; Hughes, Clarke & Money 1998, Fig. 7,
p.224, both after Robertson et al. 1994):

| Unit | Preset id | Description used |
|---|---|---|
| Weathered upper till | WT | mottled orange, brown and grey (Clarke 2008 Fig. 3; Hughes 1998 Fig. 7) |
| Red till | RT | "stiff dark reddish brown sandy silty CLAY with some fine to coarse gravel, occasional cobbles and boulders" (Hughes 1998 Fig. 7, p.224); 5YR 3/3 (Eyles & Sladen 1981, p.134) |
| Grey till | GT | "stiff and very stiff dark grey sandy silty CLAY with some to much fine to coarse gravel, some cobbles and occasional boulders" (Hughes 1998 Fig. 7, p.224); 2.5Y 4/1 (Eyles & Sladen p.134) |
| Glaciofluvial sand and gravel lens | GFSG | "dense silty SAND and GRAVEL with thickly laminated CLAY" (Hughes 1998 Fig. 7); "light brown or grey, silty SAND, SAND and GRAVEL" (Culshaw et al. 2017, Appendix 6.1, Wear Till Formation) |

Eyles & Sladen 1981 (pp.134-135) and Clarke 2008 (pp.74, 76) read the red
till as the weathered top of one grey lodgement till. The preset keeps the
three units that Clarke 2008 Table 1 tabulates separately. The grey till runs
to final depth: the site model has no bedrock.

"Occasional" and "some" cobbles and boulders are written as "low" content,
the modern wording (Norbury 2010, Table 8.4, p.112: cobbles low < 10%,
boulders low < 5%; Table 2.9, p.36 records the change from "with occasional
cobbles"). That mapping is forge judgement.

## BGS data

### Access route and licence

As for london-basin (`docs/research/london-basin-sources.md`, "Access
route"): the BGS OGC borehole index, the AGS4 export API, and the original
donated files for lab data. The data are under the Open Government Licence
v3.

**Contains data supplied by UKRI.**

### Sample

793 holes with GEOL, from 38 projects in 31 accessions, in about 40 requests.

| Area | Holes | Projects |
|---|---|---|
| Tyne and Wear | 399 | 19 |
| Co. Durham | 207 | 10 |
| Teesside | 105 | 6 |
| Northumberland | 81 | 3 |

Accessions: 42158, 121869, 125313, 126770, 128125, 138587, 163546, 163565,
163688, 163846, 164065, 165950, 165951, 167411, 169655, 169656, 169657,
169722, 173968, 173969, 173970, 173971, 174053, 174059, 174066, 186779,
186814, 188500, 188659, 191205, 192051.

Lab data come from 7 original files: 174059, 163846, 169655, 186814, 174053,
186779, 165950 (till w n = 666, LL/PL n = 559).

### Classification

- `GEOL_GEOL` codes were used first, but 78% of strata have none, so the
  descriptions decided the rest.
- Till type comes from colour words: mottled, orange or yellow is weathered;
  grey, black or blue is grey; red, purple or plain brown is red. Lumping
  plain brown with red is a judgement.
- SPT N was parsed from `ISPT_REP`. A refusal is N of 50 or more, `*`, or a
  main drive under 300 mm. N is uncorrected.

### Statistics used

Thickness, from the 200 holes that reach rock (21 projects):

| Unit | Absent | P10 / P50 / P90 where present (m) |
|---|---|---|
| Weathered till | 65% | 0.75 / 1.8 / 3.9 |
| Red and brown till | 42% | 0.67 / 3.3 / 11.0 |
| Grey till | 53% | 0.73 / 3.05 / 12.2 |
| Rockhead depth | — | 3.5 / 10.9 / 27.8 |

Grey till is absent so often largely because the holes reach rock. A
glaciofluvial layer is present in 38% of till holes, and a lens with till
above and below in 16% (18% of the holes that reach rock). Glaciofluvial
layers within till are P10 / P50 / P90 0.30 / 1.0 / 3.6 m thick (n = 169,
28 projects).

Water strikes: 437 in 324 holes, from 29 projects.

| P10 / P50 / P90 | Granular strata (n = 90, 20 projects) | Till |
|---|---|---|
| Strike depth, m | 3.7 / 8.9 / 17.8 | 2.0 / 6.7 / 16.0 |
| Level 20 minutes after the strike, m | 2.9 / 7.6 / 14.4 (n = 74) | 2.0 / 5.8 / 11.6 |
| Rise (strike less that level), m | 0.23 / 1.05 / 6.8 (mean 2.5) | 0.10 / 0.80 / 4.4 |

36% of the holes with a granular layer strike in it (89 of 249); for
granular layers with till above, 36% (55 of 154).

SPT N in glaciofluvial sand and gravel with till above: 207 tests (20
projects), 28% refusals; refusals excluded, mean 24.9, SD 12.8, P10 / P50 /
P90 8 / 23.5 / 42.

SPT N in till, refusals excluded (cross-checks only): weathered mean 22.6
(167 tests, 24 projects), red and brown 22.9 (709, 26), grey 26.6 (419, 17;
33% refusals).

Lab means, with Clarke 2008 Table 1 in brackets:

| Type | w | PL | LL | cu kPa (n) |
|---|---|---|---|---|
| Weathered | 20.6 (21.5) | 19.8 (20.3) | 41.9 (46.5) | 109 (20) [99] |
| Red and brown | 18.1 (18.1) | 17.1 (17.6) | 34.6 (38.7) | 110, median 81 (22) [100] |
| Grey | 17.2 (12.5) | 16.7 (14.9) | 33.6 (31.3) | 98 (15) [172] |

The grey class is probably mixed with misclassified red till, so the preset
does not use the BGS grey lab values; Clarke 2008 is the basis there.

## Literature

| Source | Numbers used |
|---|---|
| Clarke, Hughes & Hashemi (2008), Physical characteristics of subglacial tills, Geotechnique 58(1):67-76, doi:10.1680/geot.2008.58.1.67 | Table 1, p.71 (15 investigations, about 200 km², over 5,500 samples), mean (SD): upper red till cu 99 kPa (45), n = 526; w 21.5 (3.5), PL 20.3 (2.8), LL 46.5 (6.6), bulk density 2.06 Mg/m3 (0.076). Lower red till cu 100 (45), n = 1,879; w 18.1 (3.2), PL 17.6 (2.4), LL 38.7 (5.3), 2.06 (0.078). Lower grey till cu 172 (83), n = 1,121; w 12.5 (1.5), PL 14.9 (1.6), LL 31.3 (3.0), 2.18 (0.063). p.73: no depth trend in cu; p.74: the characteristic profile is vertical. The text on p.73 gives different cu means from Table 1; the table is used. Fig. 3, p.68: the units. |
| Hughes, Clarke & Money (1998), The glacial succession in lowland Northern England, QJEG 31:211-234 | p.223: lenses mostly under 1 m; grey till up to 40 m. p.224: weathering rarely exceeding 8 m. Fig. 7, p.224: the unit descriptions. |
| Eyles & Sladen (1981), Stratigraphy and geotechnical properties of weathered lodgement till in Northumberland, England, QJEG 14:129-141 | Summary, p.129: intraformational sands, gravels and laminated clays in the till. p.134: sand and gravel ribbons, usually under 1.5 m thick and at most 12 m, under 200 m across, frequently water-bearing; colours 5YR 3/3 and 2.5Y 4/1. Table 2, p.137: weathering zone depths (IV 3 m, III 8 m, II 10 m at most). Table 3, p.138: zone I LI -0.20 to -0.05; zones III-IV LI -0.15 to +0.30. |
| Trenter (1999), Engineering in glacial tills, CIRIA C504 | p.82: Stroud and Butler 1975, f1 5.2 for tills of PI 25 or less. p.83 and Table 4.5: Northern Ireland, Case Study 13, average f1 4.4; f1 is site-specific. p.97: typical Gs 2.67. |
| ICE Manual of Geotechnical Engineering Vol. 1 (2012), ch. 31, Fig. 31.14, p.378 | f1 4.9 for northern England. |
| ICE Manual of Geotechnical Engineering Vol. 2 (2012), ch. 53, s53.7.2, p.791 | Sampling typically every 1.5 m (as london-basin). |
| Culshaw et al. (2017), Material properties and geohazards, Geol. Soc. London Eng. Geol. Spec. Publ. 28, ch. 6, doi:10.1144/EGSP28.6 | Table 6.7: Oadby Till Member particle density from the BGS Geotechnical Properties Database, n = 152, 2.60 / 2.65 / 2.70. Appendix 6.1, Wear Till Formation: the lens description. |
| Norbury (2010), Soil and Rock Description in Engineering Practice | Table 8.4, p.112: cobble and boulder content terms. |
| Head, Manual of Soil Laboratory Testing Vol. 1, 3rd ed., Table 3.1, p.126 | Quartz 2.65 (as london-basin's RTD), for the lens. |
| Bond and Harris (2008), Decoding Eurocode 7, ch. 4, s4.5.4, Figs 4.14 and 4.15, p.108; Example 4.3, p.124 | Minimum tests per stratum with no comparable experience (as london-basin). |

No NE England particle density statistics were found, so the Oadby Till
values stand in (forge judgement). No source gives a water content for the
lenses or a depth for them.

## The preset's values

| Field | Value | Basis |
|---|---|---|
| `groundwater_depth_m` | 2.9-14.4 m | BGS 20-minute level in granular strata, P10-P90 |
| `strike_rise_m` | 0.2-6.8 m | BGS rise in granular strata, P10-P90 |
| `final_depth_m` | 15-30 m | forge judgement (into the grey till; no bedrock); BGS rockhead cross-check |
| WT `thickness_m` | -8.8 to 6.5 m | fitted to the BGS weathered-till absence and percentiles ("Thickness fit") |
| RT `thickness_m` | -1.85 to 19.25 m, skew 3.1 | fitted to the BGS red-till absence and percentiles |
| PL, PI, LI per till | see the preset | fitted to Clarke 2008 Table 1 PL, LL and w per unit (upper red for WT, lower red for RT, lower grey for GT); BGS lab cross-checks for WT and RT |
| `particle_density_ratio` | WT 2.60-2.70; RT and GT 2.60-2.62 | Culshaw 2017 Table 6.7; RT and GT held to the low end towards Clarke 2008's bulk densities |
| `cu_top_kpa` per till | see the preset | fitted to Clarke 2008 Table 1 cu mean and SD per unit |
| `per_m` | 0 | Clarke 2008 p.73-74 |
| `f1` | 4.4-5.2 | C504 p.82, p.83 and Table 4.5; ICE Vol. 1 Fig. 31.14 (4.9 inside) |
| GFSG `host` | GT | forge judgement: the sands are lenses intercalated in the lodgement till (Eyles & Sladen p.131), of which the grey till is the unweathered part, and occur at the reddened-grey contact (p.134); placed there for depth ("Lens depth") |
| GFSG `radius_m` | 50-83 m | fitted to the BGS 16-18% of holes with a lens between till; Eyles & Sladen's "under 200 m across" as a cross-check |
| GFSG `thickness_m` | 0.3-3.6 m | BGS layers within till, P10-P90, as the centre thickness (forge judgement) |
| GFSG `depth_in_host` | 0.03-0.70 | forge judgement calibrated to the BGS strike depths in granular strata |
| GFSG `water_bearing` | share 0.36 | BGS: 89 of 249 holes with a granular layer strike in it |
| GFSG `spt_n` | 5-46 | fitted to the BGS N in sand and gravel with till above |
| GFSG `moisture_content_percent` | 11-19 | forge judgement |
| GFSG `particle_density_ratio` | 2.65 | Head Vol. 1 Table 3.1 (quartz) |
| suite rates | LNMC 0.7-1.0, LLPL 0.3-0.45, LDEN 0.5-0.7, TRIG 0.5-0.7, ISPT 1.0 | Decoding EC7 minimums per stratum, the three tills as one stratum ("Testing rates") |

## Calibration method

The method is london-basin's ("Calibration method" there): generate
`gen --site` files over many seeds, read the written rows back, assign each
row to a unit from `GEOL_DESC` (a SAND or GRAVEL principal is the lens,
"reddish brown" is RT, "dark grey" is GT, the rest is WT), pool each heading
per unit, and move the ranges until the pooled means and spreads sit on the
targets. The result COVs are the forge-wide constants (`MOISTURE_COV` 0.115,
`LIMIT_COV` 0.088, `STRENGTH_COV` 0.345, `SPT_COV` 0.42, `DENSITY_COV`
0.055), unchanged. The measuring scripts stayed outside the repository.

### Thickness fit

As london-basin's Made Ground fit: one run with WT and RT each 0-100 m
linear and holes 300 m deep gives each hole's field value u; every
candidate (min, max by 0.05 and 0.25 m, skew 1.0-4.0 by 0.1) is evaluated on
those u values with the build's rounding and absence rule, scored by the
relative errors of P10, P50, P90 and absence, P50 and absence weighted
double.

| Unit | Candidate | Score | P10 / P50 / P90 (m) | Absent |
|---|---|---|---|---|
| WT | BGS target | 0 | 0.75 / 1.8 / 3.9 | 65% |
| WT | **-8.8, 6.5, no skew (shipped, best)** | **0.74** | **0.31 / 1.79 / 4.44** | **65.3%** |
| WT | -7.75, 6.75, skew 1.1 | 0.77 | 0.34 / 1.79 / 4.58 | 63.8% |
| RT | BGS target | 0 | 0.67 / 3.3 / 11.0 | 42% |
| RT | **-1.85, 19.25, skew 3.1 (shipped, best)** | **0.28** | **0.50 / 3.30 / 10.86** | **42.2%** |
| RT | -1.95, 19.25, skew 3.0 | 0.29 | 0.53 / 3.39 / 11.01 | 41.5% |

Neither fit can lift P10: the field is compressed about its middle, so a
range that puts P50 and absence on target leaves the thinnest present layers
too thin. A skew above 1 only made WT worse.

### Lens depth

The literature gives no depth for the lenses. In the first draft the lens
sat in RT at depth fraction 0.6-1.0, and its strikes were far too shallow:
RT is absent from 43% of holes and only a few metres thick where present,
and 13% of the lens strata reached the ground surface. Even at fraction 1.0
(the RT base) the strike P50 was 3.4 m against the BGS 8.9 m. The lens was
therefore moved to the upper part of the grey till, which every hole
reaches, and its depth fraction and radius swept (radius re-fitted to keep a
lens in about 18% of holes):

| Host, depth_in_host, radius (m) | Seeds | Holes with a lens | Strike P10 / P50 / P90 (m) |
|---|---|---|---|
| BGS granular strata | — | 16-18% | 3.7 / 8.9 / 17.8 |
| RT, 0.6-1.0, 71-118 (draft) | 2,000 | 18.1% | 0.00 / 3.23 / 9.31 |
| RT, 1.0, 71-118 | 1,000 | 18.0% | 0.00 / 3.41 / 10.04 |
| GT, 0.1-0.4, 71-118 | 1,000 | 31.5% | 3.74 / 7.63 / 12.05 |
| GT, 0.1-0.6, 54-90 | 1,000 | 20.6% | 4.29 / 8.98 / 14.86 |
| GT, 0.05-0.75, 54-90 | 1,000 | 20.6% | 4.07 / 10.13 / 17.32 |
| GT, 0.0-0.7, 51-85 | 2,000 | 19.0% | 3.07 / 9.11 / 15.69 |
| GT, 0.03-0.72, 51-85 | 2,000 | 19.0% | 3.66 / 9.51 / 16.09 |
| **GT, 0.03-0.70, 50-83 (shipped)** | **5,000** | **18.0%** | **3.35 / 9.41 / 15.92** |

Final depth (15-30 m) caps the deepest strikes, so P90 stays under the BGS
17.8 m whatever the fraction; the shipped range trades a little P50 for P90.

### Testing rates

On the smallest default site (3 holes), over 5,000 seeds, the share of sites
with fewer than 3 tests in a unit, and the fewest tests with the three tills
together:

| Stratum | LLPL | LNMC | LDEN | TRIG |
|---|---|---|---|---|
| WT alone, short | 94% | 92% | 96% | 96% |
| RT alone, short | 71% | 68% | 76% | 76% |
| GT alone, short | 0.2% | 0.1% | 0.3% | 0.3% |
| WT, RT and GT, fewest | 5 | 7 | 3 | 4 |

Even at rate 1.0, 90% of 3-hole sites have fewer than 3 WT U samples and 64%
fewer than 3 RT U samples, and GT alone falls short on a few sites, so the
three tills count as one till stratum (forge judgement, after Eyles &
Sladen's reading of the red till as the weathered top of one till). A test
holds the minimum over 1,000 seeds.

## Result

5,000 seeds (24,921 holes), tyneside-till v1.

| Quantity | Unit | n | Synthetic | Target |
|---|---|---|---|---|
| Thickness, absent / P10 / P50 / P90 | WT | 24,921 holes | 63% / 0.33 / 1.79 / 4.52 m | 65% / 0.75 / 1.8 / 3.9 m (BGS) |
| Thickness, absent / P10 / P50 / P90 | RT | 24,921 holes | 43% / 0.47 / 3.15 / 10.21 m | 42% / 0.67 / 3.3 / 11.0 m (BGS) |
| PL (LLPL_PL) | WT / RT / GT | 4,449 / 16,416 / 137,068 | 20.4 (2.9) / 17.6 (2.4) / 14.9 (1.4) | 20.3 (2.8) / 17.6 (2.4) / 14.9 (1.6) (Clarke) |
| LL (LLPL_LL) | WT / RT / GT | same | 46.5 (7.1) / 38.8 (5.5) / 31.3 (2.9) | 46.5 (6.6) / 38.7 (5.3) / 31.3 (3.0) |
| PI (LLPL_PI) | WT / RT / GT | same | 26.1 / 21.2 / 16.4 | 26.2 / 21.1 / 16.4 (derived) |
| w (LNMC_MC) | WT / RT / GT | 5,006 / 18,485 / 156,441 | 21.5 (3.7) / 18.1 (3.2) / 12.5 (1.6) | 21.5 (3.5) / 18.1 (3.2) / 12.5 (1.5) |
| cu (TRIT_CU), kPa | WT / RT / GT | 3,474 / 12,981 / 110,088 | 98 (47) / 99 (44) / 173 (84) | 99 (45) / 100 (45) / 172 (83) |
| Bulk density (LDEN_BDEN), Mg/m3 | WT / RT / GT | 3,448 / 12,993 / 110,122 | 2.05 / 2.09 / 2.21 | 2.06 / 2.06 / 2.18 |
| SPT N (ISPT_NVAL), cross-check | WT / RT / GT | 6,059 / 21,664 / 182,982 | 20.4 / 20.7 / 36.2 | BGS non-refusal 22.6 / 22.9 / 26.6 |
| SPT N (ISPT_NVAL) | GFSG | 1,077 | mean 26.2, P50 23; below 50: 23.1 (10.8), 10 / 22 / 38; 8% at 50 or more | non-refusal 24.9 (12.8), 8 / 23.5 / 42; 28% refusals |
| Lens strata thickness | GFSG | 4,491 | 0.16 / 0.84 / 2.24 m | BGS layers within till 0.30 / 1.0 / 3.6 m |

Means with SDs in brackets. Groundwater:

| Measure | Synthetic | BGS (granular strata) |
|---|---|---|
| Holes with a lens | 18.0% (4,491 of 24,921) | 16-18% with a lens between till |
| Lens holes with a strike | 40.2% | 36% |
| Strike depth, P10 / P50 / P90 | 3.35 / 9.41 / 15.92 m (n = 1,805) | 3.7 / 8.9 / 17.8 m |
| Level 20 minutes after, P10 / P50 / P90 | 3.30 / 8.09 / 13.06 m | 2.9 / 7.6 / 14.4 m |
| Rise, P10 / P50 / P90 | 0.00 / 0.54 / 4.22 m | 0.23 / 1.05 / 6.8 m |

Before the lens moved (RT host, 0.6-1.0, 2,000 seeds) the strikes measured
P10 / P50 / P90 0.00 / 3.23 / 9.31 m, the 20-minute level 0.00 / 3.22 /
8.46 m and the rise 0.00 / 0.00 / 1.10 m.

Testing rates, per sample: LNMC 0.43, LLPL 0.37, LDEN 0.30, TRIG 0.30 in
every till; ISPT runs at every D sample, about 0.5 per sample.

Sample descriptions:

- Stiff dark reddish brown slightly gravelly sandy CLAY with low boulder or
  cobble content.
- Very stiff dark grey slightly gravelly sandy silty CLAY with low boulder or
  cobble content.
- Stiff greyish brown slightly gravelly sandy silty CLAY.
- Medium dense light brown very sandy silty GRAVEL.
- Dense grey very gravelly silty SAND.

## Caveats

- The BGS classes come from colour words, and 78% of the strata carry no
  code. Refusals are often cobble strikes. Strikes are under-recorded in
  rotary holes, and the deepest holes per project were selected. The 93
  Pennine (A66) holes differ from the rest, but leaving them out changes
  little.
- Clarke 2008's upper red till is taken as the weathered till (forge
  judgement); its text and Table 1 disagree on the cu means, and Table 1 is
  used.
- Bulk density runs above Clarke's means (RT 2.09 against 2.06, GT 2.21
  against 2.18): with full saturation below the table, the Table 1 water
  contents and Gs of 2.60 or more cannot reach them.
- SPT N in the grey till runs above the BGS non-refusal mean (36 against
  26.6) because it comes from cu over f1, and Clarke's grey cu is high; the
  BGS mean drops a third of the tests as refusals, and its refusal-aware P50
  is 34.
- The lens fit uses the strike depths of all granular strata, which include
  sands below the till and layers this site does not model. The share of
  lens holes with a strike (40%) is above the 0.36 share because a dry lens
  still strikes where the water table falls inside it.
- The synthetic rise is smaller than BGS's: a perched lens (table below the
  lens top) settles at the strike depth, with no rise.


## SPT refusals (v2, #63)

v2 writes a refusal when a test reaches 50 blows before the 300 mm main drive:
`ISPT_NVAL` 50 (not extrapolated), `ISPT_MAIN` 50, `ISPT_NPEN` 150 plus the
penetration p reached, `ISPT_REP` "50/p"; an ordinary test writes MAIN = N,
NPEN 450 and REP "N=<N>". The convention is forge judgement (owner decision,
#63). Two causes add up: strength (any drawn N of 50 or more, p = 300 x 50 / N,
at least 1 and under 300), and a per-unit `spt.refusal_share` for cobble and
boulder strikes (p uniform 25-275 mm). Every N above 50 that v1 wrote is now
50.

### Targets

From the NE crawl (`analyse_ne`, as above; a refusal is an N of 50 or more, a
`*` in `ISPT_REP`, or a main drive under 300 mm):

| Unit | Crawl tests | Projects | Crawl refusal rate | Per-project range |
|---|---|---|---|---|
| WT | 167 | 24 | 12% | 0.00-0.40 |
| RT (red and brown) | 709 | 26 | 25% | 0.00-0.57 |
| GT | 419 | 17 | 33% | 0.00-0.62 |
| GFSG (granular with till above) | 207 | 20 | 28% | 0.00-0.42 |

Refusal-aware N (refusal = 50) has P50 23 (WT), 26 (red and brown) and 34
(grey). Refusals rise with depth in the crawl: 8% at 0-3 m, 15% at 3-6 m, 19%
at 6-10 m, 32% at 10-15 m and 55% below 15 m (1,305 till tests).

### Crawl by unit and depth

Refusals/tests (rate) by depth below ground, from the same crawl (forge analysis,
over the 1,295 WT, RT and GT tests and 207 GFSG tests):

| Unit | 0-3 m | 3-6 m | 6-10 m | 10-15 m | over 15 m |
|---|---|---|---|---|---|
| WT | 3/44 (0.07) | 3/49 (0.06) | 2/35 (0.06) | 3/22 (0.14) | 9/17 (0.53) |
| RT | 5/78 (0.06) | 22/155 (0.14) | 33/176 (0.19) | 50/150 (0.33) | 70/150 (0.47) |
| GT | 5/42 (0.12) | 22/102 (0.22) | 24/94 (0.26) | 32/95 (0.34) | 57/86 (0.66) |
| GFSG | 0/5 | 9/45 (0.20) | 17/57 (0.30) | 9/38 (0.24) | 22/62 (0.35) |
| Tills pooled | 13/164 (0.08) | 47/306 (0.15) | 59/305 (0.19) | 85/267 (0.32) | 136/253 (0.54) |

### Fit

The shares were fitted over 150,000 seeds. Strength refusals do not depend on
the share, so the fit takes the model's own test counts and strength-refusal
counts per 0.5 m of depth and solves for `top` and `per_m` of each unit
(weighted least squares against the crawl's per-unit bands, with the unit
totals and the pooled till bands as heavy terms), then the fit is checked by
generating files. The weathered till takes a plain share (its tests are almost
all above 6 m), and the red till's `very_coarse` clause (low cobbles and
boulders), which a share needs; this is forge judgement (the weathered till is
the red till's top). GFSG's `spt_n` range was raised from 5-46 to 6-53 so the
non-refusal mean stays at the crawl's 24.9 now that N of 50 or more are
refusals (file: mean 25.0, SD 11.2, P10-P50-P90 11-24-41 against 24.9, 12.8
and 8-23.5-42).

| Unit | Share | Fitted (range) | Strength only | Total target | Total achieved |
|---|---|---|---|---|---|
| WT | plain | 0.11 (0.055-0.165) | 1.1% | 12% | 12.0% |
| RT | top, per_m | 0.146 (0.117-0.175), 0.0207 (0.0145-0.027) | 0.8% | 25% | 25.2% |
| GT | top, per_m | -0.375 (-0.425 to -0.325), 0.0385 (0.027-0.05) | 18.5% | 33% | 33.1% |
| GFSG | top, per_m | 0.030 (0.021-0.039), 0.015 (0.0105-0.0195) | 12.3% | 28% | 28.1% |

Refusal rate by depth, crawl against file (the file's bands use the model's
own test depths, which are not the crawl's):

| Unit | 0-3 m | 3-6 m | 6-10 m | 10-15 m | over 15 m |
|---|---|---|---|---|---|
| WT crawl / file | 0.07 / 0.12 | 0.06 / 0.12 | 0.06 / - | 0.14 / - | 0.53 / - |
| RT crawl / file | 0.06 / 0.18 | 0.14 / 0.24 | 0.19 / 0.31 | 0.33 / 0.40 | 0.47 / 0.49 |
| GT crawl / file | 0.12 / 0.19 | 0.22 / 0.19 | 0.26 / 0.19 | 0.34 / 0.28 | 0.66 / 0.50 |
| GFSG crawl / file | 0.00 / 0.17 | 0.20 / 0.21 | 0.30 / 0.25 | 0.24 / 0.31 | 0.35 / 0.38 |
| Pooled tills target / file | 0.08 / 0.17 | 0.15 / 0.20 | 0.19 / 0.21 | 0.32 / 0.28 | 0.55 / 0.50 |

- The pooled bands fit less closely than the totals. The model's units sit
  differently in depth from the crawl's (file tests: WT 85% above 3 m, RT 68%
  above 6 m, GT 38% below 15 m; the crawl's red and brown till is far deeper),
  so a per-unit total of 25% on mostly shallow red till and a rising crawl
  profile cannot both hold. Totals were given priority. The shallow bands
  also cannot fall below the grey till's strength refusals (18.5% at every
  depth, as cu has no depth trend there).
- Refusal-aware P50 N in the file: WT 20, RT 23, GT 38, GFSG 31 (targets 23,
  26, 34 and 34 for GF with till above). Non-refusal means are WT 20.0, RT
  20.3 and GT 29.9 against the crawl's 22.6, 22.9 and 26.6; the f1 ranges were
  not refitted.


## Cover layer (v3, #58)

v3 adds a `[cover]` block with topsoil only (47% of holes) and no surfacing,
taken off the top of the hole's first stratum (ADR 0001, "Cover layer"). The
data are the NE crawl above (787 holes logged from the surface, 38 projects),
classified as in the Classification section (stratum code first, then
description; a "MADE GROUND (Grass over ... topsoil)" stratum within 0.05 m of
the surface counts as topsoil). Contains data supplied by UKRI, OGL v3.
Aggregates only.

### Targets and the fit

| Quantity | Crawl | File (7,401 holes, seeds 0-1499) |
|---|---|---|
| Holes starting with topsoil | 370 of 787 (47.0%) | 3,555 (48.0%) |
| Topsoil thickness P10 / P50 / P90 | 0.10 / 0.30 / 0.50 m (mean 0.30, SD 0.18, min 0.05, max 1.20) | 0.10 / 0.30 / 0.50 m (mean 0.30) |

The thickness range is linear, 0.06 to 0.56 m (no skew), fitted by simulating
the files, cap included; 0.05 to 0.55 gave 0.09 / 0.29 / 0.49. The file's
share is 1.0 point above the crawl's, within a binomial's two standard
deviations.

Colours (of the 196 first-stratum topsoil descriptions with a colour; greyish
brown 4, black 4, brownish grey 3, grey 2 and blackish brown 2 are left out,
so the weights are 181 of 196):

| Colour | Crawl | File (3,555) |
|---|---|---|
| dark brown | 83 (45.9%) | 47.0% |
| brown | 68 (37.6%) | 37.5% |
| light brown | 17 (9.4%) | 8.4% |
| dark grey | 7 (3.9%) | 3.8% |
| dark greyish brown | 6 (3.3%) | 3.3% |

Minor fractions, as the chance each is named (of the 170 with a soil
description):

| Fraction | Crawl | File |
|---|---|---|
| sandy | 146 (85.9%) | 85.9% |
| gravelly | 108 (63.5%) | 64.9% |
| clayey | 93 (54.7%) | 54.5% |
| silty | 26 (15.3%) | 14.6% |

### Caveats

- The crawl logs Made Ground in 64% of holes and the preset has no Made
  Ground unit, so the topsoil sits on the weathered till or the red till
  directly; the crawl's topsoil over Made Ground is not represented.
- Many crawl topsoils open "Grass over ..." or "Grass and TOPSOIL"; the file
  words the topsoil alone.
- "With rootlets" is on every topsoil; the crawl names rootlets in 78 of 370
  (21%), so this is forge judgement.
- The qualifier (slightly, plain, very) is a forge draw, as for london-basin.
- The format follows Norbury 2010's Made Ground convention (ch. 14,
  pp.208-214) and is forge judgement.

## Multi-stage UU triaxial (v4, #2)

v4 gives each till a `triaxial` table: the share of its TRIG tests that are
multi-stage unconsolidated-undrained (`TRIG_TYPE` `UUM`, three TRIT rows) and
the rise in cu per stage (ADR 0001, "Multi-stage UU"). The data are the NE
crawl's TRIG and TRIT groups, de-duplicated by file content (one file is
present twice through a symlink). Contains data supplied by UKRI, OGL v3.
Aggregates only.

### Share

The crawl has 59 TRIG tests in 5 files (BGS NGDC AGS accessions 174059,
165950, 169655, 174053 and 186779): 32 are UUM and 27 UU, a share of 0.54.
The UUM tests are in three files (174059 all UUM, 169655 all UUM, 165950 4 of
7); the other two are all UU, so the share varies between projects far more
than 59 tests can map. The preset takes 0.49 to 0.59, +-0.05 about the crawl
(forge judgement). The reason the till is multi-stage is the owner's practice
note (forge judgement): labs run multi-stage UU where three matched 38 mm
specimens cannot be cut from a matrix-bound gravelly till. That reason is why
the share sits on the tills and why london-basin, whose clay is not that
case and has no source, takes none.

### Stage gain

For each of the 32 UUM sets (all with TRIT_CU at stages 1 to 3), cu at stages
2 and 3 over stage 1:

| Quantity | Stage 2 / stage 1 | Stage 3 / stage 1 |
|---|---|---|
| Mean | 1.19 | 1.40 |
| Median | 1.11 | 1.21 |
| P10 / P90 | 1.01 / 1.76 | 1.02 / 1.78 |

The cell pressures of the sets are 1, 2 and 4 times stage 1 in 23 of 32;
the rest step otherwise (stage 3 at 2.25 to 3.6 times stage 1, and one set
whose stages 2 and 3 are 20 and 40 times stage 1). With cu(k) = cu(1) x (1 + g x (k - 1)), the least-squares g over both
stages is 0.20 pooled (mean of the 32 per-set values 0.20, median 0.11, P10 to
P90 0.01 to 0.47, one set at 1.77). The preset takes 0.10 to 0.30, +-0.10
about the mean (forge judgement), and the mean was chosen over the median
because the file's cu is then right on average and the crawl's skew is a few
large rises.

### Fit

| Quantity | Crawl | File (300 seeds, 7,429 TRIG tests) |
|---|---|---|
| UUM share of TRIG tests | 32 of 59 (0.54) | 4,000 (0.538) |
| Mean cu stage 2 / stage 1 | 1.19 | 1.20 |
| Mean cu stage 3 / stage 1 | 1.40 | 1.41 |

### Caveats

- The cell pressure in the file is always x1/x2/x4; 9 of the crawl's 32 sets
  step otherwise.
- cu is scattered under the strength scatter before the stage rise, so the
  file's stage ratios are those of the written stage-1 value, not of an
  underlying strength.
- Effective-stress TREG/TRET (83 of 91 crawl sets are 3-stage) is not
  written; that is a separate slice.
