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
| GFSG `water_bearing` | share 0.44 | BGS: 89 of 249 holes with a granular layer strike in it (36%); raised from 0.36 in v8 so the file's lens holes strike in it at 35.8% |
| WT, RT, GT `strike.rate_per_m` | 0.0375, 0.0425, 0.0116 per m | fitted to the BGS share of holes through each till that strike in it (7.3%, 15.0%, 17.7%), plus the owner's practice note (forge judgement) ("Water strikes in till") |
| WT, RT, GT `strike.rise_m` | 0.1-6.9 m, skew 2.6 | fitted to the BGS till strike rise over all the file's till strikes (P50 0.68, P90 4.11 m, n = 90), pooled over the tills |
| WT, RT, GT `strike.no_rise_share` | 0.12 | BGS: 11 of 90 till strikes (12%) with a level within 0.05 m of the strike |
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
  still strikes where the water table falls inside it. (From v8 the share is
  0.44; see "Water strikes in till".)
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
- Effective-stress TREG/TRET is not written by v4; v5 writes it (next
  section).

## Effective-stress triaxial (v5, #68)

v5 gives each till an `effective_strength` table (c', phi' and Skempton's A_f)
and the suite TREG and TRET entries (ADR 0001, "Effective-stress triaxial").
The data are the NE crawl's TREG and TRET groups, de-duplicated by file content
(an earlier count of 91 sets in 5 files counted one file twice through a
symlink). Contains data supplied by UKRI, OGL v3. Aggregates only.

### The crawl

57 TREG sets and 159 TRET rows in 4 files (BGS NGDC AGS accessions 165950,
169655, 174053 and 174059). Types: CUM 22, CDM 15, CU 10, CD 10. 51 sets have 3
TRET rows and 6 have 1 (all of 174053, and 2 CD of 174059). The 14 CU and CD
sets of 3 rows, all in 174059, are three specimens, and the files write them
under one TREG row, one `SPEC_REF` and `SPEC_DPTH` and `TRET_TESN` 1 to 3, as
for the multi-stage types. Units were classed from the GEOL description at the
specimen depth (colour words, as for the other lab data): 25 of the sets are in
the three tills (weathered 3, red and brown 15, grey 7), 24 in laminated clay.

Stage 1's effective confining stress (cell pressure less initial pore
pressure) over the specimen depth has median 10.0 kPa/m (P10 9.1, P90 20.0, the
top being 174053, which uses 20). Stage ratios are 2.0 at the median, 88% within
1.8 to 2.2. Back pressure is 300 kPa in 165950 and 169655 and about 500 in
174059. `TREG_FCR` is "Maximum effective principal stress ratio" in 169655
(16 CUM), "Maximum Deviator Stress" in 174053 and part of 174059, and blank in
20 sets.

### c', phi' and A_f by till unit

| Unit | Accessions | Sets with c', phi' | c' mean (SD), kPa | phi' mean (SD), deg | A_f stages (sets) | A_f mean (SD) |
|---|---|---|---|---|---|---|
| Weathered till | 165950, 174059 | 3 | 5.3 (3.1) | 31.1 (2.4) | 6 (2, CUM) | 0.19 (0.07) |
| Red and brown till | 169655, 174053, 174059 | 13 | 9.2 (7.1) | 26.0 (7.1) | 17 (7) | 0.46 (0.28) |
| Grey till | 174053, 174059 | 7 | 3.3 (2.7) | 30.6 (3.8) | 8 (4) | 0.03 (0.15) |

A_f is (PWPF - PWPI) / DEVF at the failure instant of each CU and CUM stage.
Over all 88 stages (every unit, including laminated clay at 0.65) it has mean
0.45; over the 31 till stages 0.30 (SD 0.29). The sets are few, so the table's
spreads are the crawl's, not a population's.

Cross-checks. Hashemi 2002 PhD (Univ. Newcastle), Tables 5.1 to 5.3, pp.91-92,
effective-strength means by unit and area from site-investigation tests:
Unit 1 (upper weathered till) c' 6.2 kPa and phi' 23.3 deg in Northumberland
(n = 27), 11.9 and 27.0 in Tyne and Wear with Durham (n = 9); Unit 2 (upper
till) 6.8 and 25.3 (n = 48), 13.7 and 23.1 (n = 13); Unit 3 (lower grey till)
8.6 and 27.7 (n = 39), 14.0 and 26.2 (n = 41). The tables were read from a text
extract and checked against the text's means, not against the page images. The
crawl's weathered till at 31 deg sits above all of them. CIRIA C504 (Trenter
1999), p.85: c' usually varies between zero and 25 kN/m2 for clay tills (after
Sladen and Wrigley 1983, not seen), and is often assumed zero; the loader's cap
on c' is that 25. No published A_f was found for these tills.

### What the preset takes

c' and phi' take each unit's crawl mean (above), with a range 4 kPa or 4
degrees wide for site-to-site variation (forge judgement; the 3 to 13 sets per
unit cannot give one) centred so that the file's pooled values land on the mean.
A_f takes each unit's mean with a range 0.2 wide. Each set scatters about the
site's c' and phi' with SD 2.0 kPa and 1.5 deg, and each stage about its A_f
with SD 0.10 (forge judgement, below the crawl's within-unit spreads, which also
hold site and stress-level differences). The failure deviator is on the
envelope, with a 3% lognormal scatter for drained stages; the crawl's 142
stages with a reported envelope sit within P10 -4% and P90 +3% of it.

### Fit (300 seeds)

| Quantity | Crawl | File |
|---|---|---|
| TREG sets per TRIG test | 57 / 59 = 0.97 | 7,248 / 7,429 = 0.98 |
| CUM, CDM, CU, CD share | 39%, 26%, 18%, 18% | 38.3%, 25.8%, 18.3%, 17.6% |
| Weathered till c', phi', A_f | 5.3, 31.1, 0.19 | 5.30, 31.09, 0.18 |
| Red and brown till c', phi', A_f | 9.2, 26.0, 0.46 | 9.20, 25.91, 0.47 |
| Grey till c', phi', A_f | 3.3, 30.6, 0.03 | 3.33, 30.56, 0.03 |

### Caveats

- The crawl's three-specimen CU and CD sets have different back pressures,
  dimensions and sometimes missing pore pressures; the model gives all three
  the same back pressure and writes every heading.
- Laboratories step the stages otherwise at shallow depth (increments instead of
  ratios) and cap the stress at about 1.2 to 1.5 MPa at depth; the model is
  always 1:2:4 and has no cap, so a specimen at 19 m reaches 756 kPa in
  stage 3 and 1.6 MPa deviator stress.
- c' is floored at 0, and a set whose line would cut below 0 is refitted through
  the origin, as laboratories report c' = 0; the crawl has c' = 0 in 9 of its
  55 sets with values.
- A_f is a pooled mean over few stages. The model's drained and undrained types
  use the same c' and phi' for a unit; the crawl's type-by-type means are not
  material at this n.
- The effective-stress parameters of the laminated clay (the crawl's 24 sets,
  10.4 kPa and 21.9 deg) are not used: it is not a unit of this preset.

## Hole construction (v6, #74)

v6 adds a `[construction]` block (ADR 0001, "Hole construction"): an inspection
pit and a bore phase (HDPH), casing (CDIA), the casing depth at each strike
(`WSTG_CAS`) and backfill (BKFL) for holes with no installation. The data are
the NE crawl's HDPH, CDIA, WSTG and BKFL groups, de-duplicated by file content
(52 unique files; holes de-duplicated by project, `LOCA_ID` and final depth,
1,041 holes with GEOL, 34 projects). Contains data supplied by UKRI, OGL v3.
Aggregates only; forge analysis. The method is the same as for the other
groups: the scripts read the files as untrusted data and write statistics only.

### The crawl

- HDPH is in 6 accessions (165950, 169655, 174053, 174059, 186779, 186814),
  289 holes (28%). The inspection pit (`HDPH_TYPE` IP) is 0 to 1.2 m in 188 of
  188 rows, and the last phase's base is the final depth in every hole.
  `LOCA_TYPE` is the phases joined by `+` in 83% of those holes. The crawl also
  writes COM, SB and SC, which are not in the dictionary's pick list; the
  preset writes IP and CP only.
- CDIA is in 23 accessions, 622 holes with GEOL (60%). 475 holes have one
  casing diameter, 132 two, 15 three or four. The first string is 200 mm in 242
  holes, 150 mm in 187, 125 mm in 110, 250 mm in 33, 140 mm in 19 and 116 mm in
  14. 138 holes (22.2%) step down to a smaller diameter with depth, with the
  larger string ending at a fraction of the deepest casing depth of P10 0.14,
  P50 0.50, P90 0.91. Final depth less the deepest casing depth: P10 0.1 m, P25
  0.7 m, P50 3.0 m, P75 10.5 m, P90 20.0 m. Of 458 holes with CDIA and a strike
  the deepest casing is below the strike in 376 (82%). HDIA is in 5 accessions
  and is not modelled.
- `WSTG_CAS` is in 16 accessions: 227 strikes in 170 holes. Strike depth less
  casing depth at the strike has P10 0.0 m, P25 0.2 m, P50 0.5 m, P75 1.6 m,
  P90 4.8 m. `WSTG_SEAL` is left out.
- BKFL is in 26 accessions; 405 holes have BKFL and no PIPE or MONG. Their
  sequences, top to bottom: bentonite alone 118, arisings alone 113, arisings
  over bentonite 65, then grout, concrete and longer sequences. The base of the
  arisings over bentonite is at P10 0.2 m, P50 0.5 m, P90 1.2 m (n = 65).
  Backfill is a project-level practice: of 15 projects, 11 have it in 92% or
  more of their CP-type holes and 4 in none.
- The inspection pit's 1.2 m is also the owner's practice note (forge
  judgement): a hand-dug inspection pit to clear buried services is standard UK
  practice before boring.

### What the preset takes

Diameters 200, 150 and 125 mm at the first-string counts above (242, 187, 110).
A telescoped hole steps down to a smaller size in that list by the same
weights, so only 200 and 150 mm can step down (79.6% of holes), and the share is
0.28 = 22.2% / 79.6%. The larger string ends at 0.1 to 0.9 of the way from the
pit base to the final casing. The final casing passes the strike in 82% of
holes. A hole that passes draws its gap from `final_gap_m` cut off where the
casing would end at or above the strike; a hole that does not ends its casing
between the pit base and the strike. `strike_gap_m` is the gap from the strike
up to the casing at the strike, never below 0, and the casing at the strike
cannot be deeper than the final casing. BKFL is carried by 0.7 of sites (11 of
15 projects), and each hole draws arisings alone, bentonite alone, or arisings
over bentonite at 113 : 118 : 65, in legend codes 905 and 903 only.

### Fit (1,500 seeds, 8 holes each)

| Quantity | Crawl | File |
|---|---|---|
| Holes that step down | 22.2% | 22.6% |
| First string 200, 150, 125 mm | 44.9%, 34.7%, 20.4% (of 539) | 45.5%, 34.7%, 19.8% |
| Casing passes the strike | 82% | 81.9% (871 strikes) |
| Step-down depth fraction P10, P50, P90 | 0.14, 0.50, 0.91 | 0.18, 0.49, 0.81 |
| Final depth less final casing P25, P50, P75, P90 (m) | 0.7, 3.0, 10.5, 20.0 | 0.4, 2.9, 9.2, 15.5 |
| Strike less `WSTG_CAS` P50, P75, P90 (m) | 0.5, 1.6, 4.8 | 0.49, 1.3, 2.6 |
| BKFL: arisings, bentonite, both | 38.2%, 39.9%, 22.0% | 38.5%, 39.3%, 22.2% |
| Sites carrying BKFL | 11 of 15 projects (0.73) | 1,032 of 1,500 (0.69) |
| Arisings-over-bentonite boundary P10, P50, P90 (m) | 0.2, 0.5, 1.2 | 0.21, 0.50, 1.19 |

### Caveats

- The site is 15 to 30 m of clay till with no rock, so the casing is never left
  short of a rock head, which is where the crawl's long gaps (P90 20 m) come
  from. The skew is set for the median and the file's upper quartile and P90
  stay under the crawl's.
- Only 7% of holes strike water (the lens), so the pass rate and the strike gap
  are checked on 871 strikes in 12,000 holes.
- The crawl's practice is strongly project-dependent (a few dozen contractors).
  The telescope and pass rates are pooled over projects.
- The pass rate is the owner's figure from the earlier count (376 of 458); a
  count by each hole's first strike gives 279 of 333 (84%).
- BKFL for installed holes (pipes, standpipes and their filter packs) is
  written from v7 (see "Installations").

## Installations (v7, #75)

v7 adds a `[construction.installation]` table (ADR 0001, "Hole construction",
"Installations"): `PIPE`, and the backfill an installed hole carries in place of
the site's. The data are the NE crawl's PIPE, WSTG and BKFL groups,
de-duplicated by file content (52 unique files; holes by project, `LOCA_ID` and
final depth, 1,041 holes with GEOL, 34 projects). Contains data supplied by
UKRI, OGL v3. Aggregates only; forge analysis. The scripts read the files as
untrusted data and write statistics only.

### The crawl

- PIPE is in 17 accessions (121869, 128125, 163565, 165950, 165951, 167411,
  169655, 169656, 169657, 173968, 173969, 173971, 174053, 174059, 174066,
  186779, 186814): 334 holes, 32% of the 1,041.
- Of 19 projects with five or more CP-type holes, 10 install: 0.43, 0.44, 0.49,
  0.64, 0.64, 0.67, 0.77, 0.83, 0.83 and 1.00 of their CP-type holes have PIPE.
  Nine have none.
- Of the 334 PIPE holes, 162 have a 50 mm installation, 88 a 19 mm one or a
  piezometer tip row, 82 record no diameter and 2 another size. Piezometer tip
  rows are 0.3 m (n = 43).
- Slotted sections: length P10 0.5 m, P50 2.25 m, P90 6.0 m (n = 180 rows). The
  top, in the 89 holes with no recorded strike, is P10 1.0 m, P50 3.0 m, P90
  11.6 m; in the 137 with a strike, the top less the first strike's depth is
  P10 -2.7 m, P50 0.0 m, P90 +3.0 m. Final depth less the deepest `PIPE_BASE`
  is P10 0.5 m, P50 3.5 m, P90 20.5 m (n = 310).
- The notes' earlier median of 4.5 m for that gap counted holes with no
  `PIPE_BASE` as a base of 0; excluding them gives 3.5 m, which is used.
- By keyword classification of `GEOL_DESC`, the dominant ground over 225 zones
  is fine or till-led in about 120, granular in 56, rock in 34 and made ground
  in 15. The preset places the zone by depth alone.
- In holes with PIPE, the concrete layer from the surface is P10 0.2 m, P50
  0.5 m, P90 0.5 m (n = 266). Of 223 holes whose slotted section lies inside a
  gravel or sand layer, 165 are gravel (902) and 58 sand (901). The filter
  reaches 0.5 m above and 0.5 m below the slotted section.
- `MONG` restates `PIPE` and is not written.

### Fit (6,000 seeds)

| Quantity | Crawl | File |
|---|---|---|
| Sites that install | 10 of 19 projects (0.53) | 3,115 of 6,000 (0.52) |
| Share of an installing site's holes | 0.43 to 1.0, mean about 0.72 | 0.72 |
| Holes installed | 0.32 of all holes (every type) | 0.37 of holes (29,901) |
| Standpipes among installations | 162 of 250 (64.8%) | 63.6% |
| Slotted length, standpipe P10, P50, P90 (m) | 0.5, 2.25, 6.0 | 0.49, 2.23, 6.00 |
| Zone top, no strike P10, P50, P90 (m) | 1.0, 3.0, 11.6 | 1.20, 3.11, 11.15 |
| Zone top less first strike P10, P50, P90 (m) | -2.7, 0.0, +3.0 | -2.82, -0.12, +3.14 |
| Gravel filter among installed holes | 165 of 223 (74.0%) | 74.1% |

### Caveats

- The zone cannot start above the 1.2 m inspection pit, so the file's P10 for a
  zone top is 1.2 m against the crawl's 1.0 m.
- The file's holes are 15 to 30 m, deeper than many of the crawl's, so final
  depth less the zone base is P10 7.4 m, P50 15.5 m, P90 23.8 m, against the
  crawl's 0.5, 3.5 and 20.5 m. The drawn gap (P10 0.5 m, P50 3.5 m, P90 9.9 m,
  cut off at 12 m) is only a ceiling on the zone and rarely binds. A longer
  cut-off would pull zones above the strikes they follow.
- Only 7% of holes strike water (the lens), so the zone-less-strike fit rests
  on 776 holes in 6,000 sites.
- Practice is strongly project-dependent (a few dozen contractors). The filter
  material and the kind weights are pooled over projects.
- The piezometer's filter is drawn from the same gravel and sand weights as the
  standpipe's. In the crawl the sand filters sit mostly with piezometers (47 of
  the 58), a correlation the file does not keep.

## Water strikes in till (v8, #77)

v8 adds a `strike` table to the weathered (WT), red (RT) and grey (GT) tills
(ADR 0001, "Ground", "Water strikes in till"). The data are the NE crawl's
GEOL, WSTG and WSTD groups: 793 holes with GEOL, de-duplicated by
`BGS_LOCA_ID`, from 38 projects, 29 of them reporting any WSTG. Contains data
supplied by UKRI, OGL v3. Aggregates only; forge analysis. The scripts read the
files as untrusted data (read-only, `python3 -I`, outside the data
directories, de-duplicated by content hash) and write statistics only. Till
type is classed from colour words in `GEOL_DESC` (mottled or orange as
weathered; red and brown as red; grey, black or blue as grey; 45 holes of
untyped till with one strike are dropped), contiguous strata of one class are
merged, and a strike belongs to the layer that contains its depth (at a
boundary, the layer below). The sixteen accessions that hold till strikes are
42158, 121869, 126770, 128125, 163546, 163688, 163846, 169722, 173968,
173969, 173971, 174053, 174059, 186779, 186814 and 188500.

### The crawl

- 437 strikes in 324 holes (40.9% of 793; 44.0% of the 736 holes in reporting
  projects). The host of the strike: till 129, made ground 96, granular 90,
  bedrock 44, laminated clay 36, alluvium 22. The preset has no made ground or
  bedrock, so the file's share of holes cannot reach the crawl's.
- Strikes in till, by unit:

  | Unit | Holes through it (projects) | Holes striking in it | Strikes | Per 10 m penetrated | Metres |
  |---|---|---|---|---|---|
  | WT | 260 (30) | 19 = 7.3% (10 projects) | 19 | 0.31 | 621 |
  | RT | 339 (33) | 51 = 15.0% (16) | 59 | 0.32 | 1,849 |
  | GT | 249 (27) | 44 = 17.7% (11) | 50 | 0.43 | 1,151 |
  | any till | 598 (35) | 112 = 18.7% (22) | 129 | 0.33 | 3,912 |

  In RT the share of holes striking rises with the thickness crossed (6%, 2%,
  10%, 19%, 37% for 0-1, 1-2, 2-4, 4-8 and over 8 m) while the rate per metre
  stays near 0.33 per 10 m above 2 m, so a flat share per hole does not fit
  and a rate per metre does. In GT the shares are 8%, 18%, 13%, 22% and 31%,
  and the rate is 0.43 per 10 m for 2 to 8 m and 0.30 over 8 m. WT is too thin
  to say (median layer 1.9 m).
- Position of the strike in its layer (depth below the top over the layer's
  thickness), P10 / P50 / P90: WT 0.00 / 0.62 / 0.90 (n = 19), RT 0.03 / 0.55 /
  0.92 (n = 59), GT 0.00 / 0.43 / 0.90 (n = 50): roughly even through the
  layer, unlike granular strikes, which sit at the top (median 0.00).
- Only 5% (WT), 14% (RT) and 6% (GT) of till strikes have a sand or gravel
  parting, lens, pocket, lamina, band or layer in the logged description (5%,
  24% and 10% with any parting, lens, pocket or band word). More than 85% have
  none, so the strike is not usually tied to a logged parting. Partings are
  about three times over-represented for RT only. "Wet", "water" or "seep"
  appear in 0%, 0% and 2%. The basis is therefore the crawl's rates, plus the
  owner's practice note (forge judgement) that strikes in clay till come from
  thin sand or silt partings and fissures too small to log separately. The
  published sources (Eyles & Sladen 1981, p.134; CIRIA C504 Fig 6.3, p.117) are
  about sand and gravel lenses in till being frequently water-bearing, and are
  context, not a source for a rate in clay.
- The till strike is the hole's first in 83 of 112 holes (74%). Among the 112,
  strikes in till per hole are 1 in 97, 2 in 13 and 3 in 2, so extra strikes
  are mostly in other units. The first strike of all 324 struck holes is made
  ground in 28%, till 26%, granular 19% and other 27%.
- Rise (strike depth less the level at 20 minutes, else the last reading; WSTG
  has no rise column; 90 of 129 till strikes have readings):

  | Unit | n (projects) | P10 / P50 / P90 (m) | Mean (m) | Rise 0.05 m or less | Over 1 m |
  |---|---|---|---|---|---|
  | WT | 10 (5), thin | 0.00 / 0.37 / 1.21 | 0.74 | 40% | 10% |
  | RT | 44 (14) | 0.10 / 0.60 / 3.36 | 1.55 | 5% | 39% |
  | GT | 35 (10) | 0.00 / 0.90 / 5.90 | 2.20 | 14% | 49% |
  | any till | 90 (17) | 0.02 / 0.68 / 4.11 | 1.74 | 12% (11 of 90) | 40% |

  Only 1 of 90 till strikes fell. Granular strikes rise 0.27 / 1.05 / 6.69 m
  (3% with no rise). `WSTG_REM` is non-empty for 16 of 129 till strikes and
  "seep" appears in none, so the crawl records no seepage convention.
- Strike depth below ground, P10 / P50 / P90 (m): WT 1.19 / 2.75 / 16.24 (n =
  19), RT 2.92 / 7.80 / 13.64 (n = 59), GT 2.27 / 5.05 / 18.17 (n = 50), any
  till 2.00 / 6.65 / 16.00 (n = 129).

### What the preset takes

A hole is a Poisson process along its penetration of each till, with a rate
per metre drawn once per site (`rate_per_m`, here a single pooled value). The
first event below the written top of the till's stratum is the strike, so it is
uniform in the stratum, as the crawl's are. One strike per hole: the
shallowest of a lens-top strike and a till strike; a table strike in a coarse
stratum counts only when neither occurs. The water always rises by a draw
from `rise_m`, whatever the depth of the water table, but not above ground
level, and a draw under `no_rise_share` leaves it at the strike. The table
bounds nothing (`forge judgement`): in a clay till the head on a sand or silt
parting is not the site water table, and the crawl's till strikes almost all
rise (1 of 90 fell, 12% none). All three tills carry the pooled rise and
no-rise share because WT has 10 readings.

The rates are fitted to the share of holes through each unit that strike in it,
as the issue asks, not to the crawl's per-metre rate:

| Unit | `rate_per_m` | Per 10 m | Crawl per 10 m | Mean metres crossed, file | Mean metres crossed, crawl |
|---|---|---|---|---|---|
| WT | 0.0375 | 0.375 | 0.31 | 2.0 | 2.4 |
| RT | 0.0425 | 0.425 | 0.32 | 4.2 | 5.5 |
| GT | 0.0116 | 0.116 | 0.43 | 19.1 | 4.6 |

The grey till runs to final depth (15 to 30 m) with no bedrock, so the file's
holes cross four times the grey till the crawl's do. At the crawl's 0.43 per
10 m, 48% of holes would strike in it (measured at 0.043 per m, 1,500 seeds),
against the crawl's 17.7%, because the crawl's own share saturates at 31% in
holes that cross more than 8 m. The fitted GT rate is therefore a quarter of
the crawl's and is not a per-metre match. WT and RT are fitted higher than the
crawl's because the file's holes cross less of each than the crawl's do, and
one strike per hole lets a shallower strike pre-empt a deeper one.

The rise range is `{ min 0.1, max 6.9, skew 2.6 }` with a no-rise share of 0.12,
fitted by sweeping the range and skew against all of the file's till strikes
(see below). The no-rise draw is the crawl's 12% on its own: the ground cap
adds only about 0.4 points on top (12.4% of strikes have a rise of 0.05 m or
less). A skewed uniform cannot match the crawl's P50, P90 and mean together
(at P50 0.68 m and P90 4.2 m the mean is 1.43 m against 1.74 m; a range wide
enough for the mean pulls P50 down to 0.5 m), so the fit holds the median and
the 90th percentile.

### Fit (20,000 seeds, 3 to 7 holes each, 100,387 holes)

| Quantity | Crawl | File |
|---|---|---|
| Holes striking in WT, of holes through it | 7.3% | 7.3% (37,007 holes) |
| Holes striking in RT | 15.0% | 14.9% (57,425) |
| Holes striking in GT | 17.7% | 17.7% (99,660) |
| Holes with the GFSG lens that strike in it | 36% (89 of 249) | 35.8% (17,960 holes) |
| Till strike depth, P10 / P50 / P90 (m) | 2.0 / 6.7 / 16.0 | 1.1 / 7.2 / 19.4 |
| Rise of all till strikes, P10 / P50 / P90 (m) | 0.02 / 0.68 / 4.11 | 0.00 / 0.68 / 4.17 |
| Same, mean; over 1 m; 0.05 m or less | 1.74; 40%; 12% | 1.43; 42%; 12.4% |
| Holes with any strike | 40.9% (with made ground) | 35.3% (about 7% in v7) |

### Caveats

- **The table does not bound a till strike's rise.** The first v8 build capped
  the rise at the water table and left strikes perched where the table is at
  or below them. The table field is fitted to the granular strata's 20-minute
  levels (P10 to P90 2.9 to 14.4 m), so that rule left 55% of the file's till
  strikes perched and 61% with no rise, against the crawl's 12% (1 of 90
  fell). The owner's decision is that a till strike always rises by its draw,
  capped only at ground level (`forge judgement`: in a clay till the head on a
  parting is not the site water table). The settled levels of till strikes
  are then not tied to the pore-pressure table.
- The file's till strike depths are spread wider than the crawl's: P10 1.1 m
  and P90 19.4 m against 2.0 and 16.0 m. The grey till runs to final depth (15
  to 30 m) here, so its strikes reach deeper than the crawl's holes do.
- The strike rate is one pooled value per unit, not a range: the crawl gives no
  spread between projects that the model could use, and strikes cluster by
  project (10 to 16 projects per unit).
- One strike per hole means a shallower strike pre-empts a lens-top strike. At
  the old share of 0.36 the file's lens holes struck in the lens in 29% of
  holes (crawl 36%), so the share is raised to 0.44 and the file gives 35.8%.
  The crawl's 89 of 249 holes striking in a granular layer counts holes that
  also strike elsewhere.
- Thin cells: WT strikes (19), WT rise (10 readings, 5 projects) and all
  per-thickness cells under 40 holes.

### Re-check of hole construction and installations (6,000 seeds, 29,901 holes)

Nothing was refitted. The strikes feed `WSTG_CAS` (#74) and the response zone
(#75).

| Quantity | Crawl | v7 file (strikes in the lens only) | v8 file |
|---|---|---|---|
| Holes with a strike | 40.9% | about 7% | 34.4% (10,271 strikes) |
| Final casing passes the strike | 82% | 81.9% | 83.9% |
| Strike less `WSTG_CAS`, P50 / P75 / P90 (m) | 0.5 / 1.6 / 4.8 | 0.49 / 1.3 / 2.6 | 0.40 / 1.16 / 1.96 |
| Strike less `WSTG_CAS`, P10 / P25 (m) | 0.0 / 0.2 | not reported | 0.00 / 0.06 |
| Zone top less first strike, P10 / P50 / P90 (m) | -2.7 / 0.0 / +3.0 | -2.82 / -0.12 / +3.14 | -3.13 / 0.01 / +3.12 (3,875 holes) |
| Zone top with no strike, P10 / P50 / P90 (m) | 1.0 / 3.0 / 11.6 | 1.20 / 3.11 / 11.15 | 1.20 / 3.11 / 11.10 (7,296 holes) |
| Final depth less pipe base, P10 / P50 / P90 (m) | 0.5 / 3.5 / 20.5 | 7.4 / 15.5 / 23.8 | 6.1 / 14.5 / 23.3 |

The casing gap now has a thinner upper tail than the crawl (P75 1.16 m against
1.6 m, P90 1.96 m against 4.8 m). A likely cause, not tested, is that till
strikes are shallower than the lens strikes were and a strike near the
surface cannot have a gap larger than its own depth. A refit of
`strike_gap_m`'s upper tail would raise it; it is not changed here. The pass
rate (83.9% against 82%) and the zone offsets are within a point or two of the
crawl. The final depth less the pipe base is as far from the crawl as it was
(the file's holes are deeper than the crawl's).

## Several strikes per hole (v9, #79), 2026-10-24

v9 lets a hole record several strikes (`site.several_strikes`), writes
`WSTG_SEAL` (a `[construction]` `seal` key) and lets a multi-strike hole's
installation zone follow the first or the deepest strike
(`zone_follows_first_share`). Contains data supplied by UKRI, OGL v3.
Aggregates only; forge analysis.

### The crawl

51 unique AGS4 files from the NE England crawl, de-duplicated by content hash
(one AGS3 file has no `WSTG` group and is left out), 996 holes de-duplicated
by (`PROJ_NAME`, `LOCA_ID`), 955 with GEOL, 38 projects. This sample is larger
than v8's 793 holes because it includes the `orig` accessions and the `prior`
folders, so its figures are not directly comparable with the v8 section above.
The accessions that hold multi-strike holes are 42158, 121869, 126770, 128125,
163565, 163688, 163846, 165951, 169655, 173968, 173969, 173971, 174053, 174059,
186779 and 186814. A strike is a distinct `WSTG_DPTH` in a hole; strikes are
ordered by depth; the host stratum is the layer holding the depth (the layer
below at a boundary). Rise is the strike depth less `WSTD_POST` at 20 minutes,
else the last reading. A figure is THIN under 20 holes or under 5 projects.
The scripts read the files as untrusted data (read-only, `python3 -I`, outside
the data directories) and write statistics only.

- Holes with any `WSTG`: 424 in 29 projects. Strikes per struck hole: 1 in
  322 (75.9%), 2 in 81 (19.1%), 3 in 18 (4.2%, 11 projects: THIN), 4 in 3
  (0.7%, THIN); the maximum is 4. Of all 955 holes with GEOL, 10.7% have two or
  more strikes (102 holes, 21 projects; the top project holds 24 of them).
- Hosts of the first and second strike in the 102 multi-strike holes: made
  ground 31 and 5, granular 17 and 28, weathered till 4 and 2, red till 16 and
  20, grey till 8 and 16, laminated clay 11 and 9, alluvium 11 and 4, rock 0
  and 14, other 4 and 4, untyped till 1 and 0. The second strike is in the same
  stratum (a contiguous run of one class) as the first in 15 of 102 (15%), the
  same logged GEOL row in 8, the same unit class in 21.
- Gap between consecutive strikes, P10 / P50 / P90: 1.75 / 4.71 / 12.23 m (126
  pairs, 102 holes, 21 projects); none is 0.5 m or closer. Within one stratum
  1.59 / 3.30 / 5.62 m (20 pairs: THIN).
- Casing: 57 later strikes have `WSTG_CAS` (45 holes, 13 projects); it is at or
  below the strike above in 54 (95%). Casing less the strike above, P10 / P50 /
  P90: 0.50 / 2.50 / 9.10 m. Casing less the strike's own depth: -5.38 / -0.60
  / 0.00 m.
- Seals: `WSTG_SEAL` is on 60 of 126 earlier strikes (48%) and 25 of 102 last
  strikes (25%). Seal less strike, P10 / P50 / P90: 0.00 / 1.00 / 3.44 m (85
  seals, 58 holes, 13 projects), with 8 seals above the strike and 2 level. Of
  32 sealed strikes whose next strike has a casing, 31 (97%) are sealed at or
  above that casing.
- Readings: 83% of first strikes (85 of 102) and of later strikes (105 of 126)
  have `WSTD`. Rise P10 / P50 / P90: first strikes 0.01 / 0.25 / 1.60 m (19
  projects), later strikes 0.20 / 1.50 / 5.40 m (17 projects); no rise (0.05 m
  or less) in 14% and 4%.
- Installations: 39 multi-strike holes have `PIPE`, 26 of them (6 projects:
  THIN) a slotted or piezometer zone. The zone top is nearest the first strike
  in 14 and the deepest in 12; none is nearest a middle strike.

### The final casing, re-measured

The decisions ask for the share of holes whose final casing passes the last
strike, re-measured on multi-strike holes. The deepest `CDIA_DPTH` is below the
last strike in 61 of 84 multi-strike holes with `CDIA` (72.6%, 17 projects), 221
of 297 struck holes with `CDIA` (74.4%, 23 projects) and 160 of 213 single-strike
holes (75.1%). It is below the first strike in 83 of the 84 multi-strike holes
and below every earlier strike in 81 (96.4%); in 22 the casing passes the first
strike but not the last. The preset's 0.82 (376 of 458 holes with `CDIA` and a
strike, the v6 sample) is kept, as the decisions say; this sample, de-duplicated
by project and `LOCA_ID`, gives a lower share, both in multi-strike holes (72.6%)
and overall (74.4%), so the difference is not a multi-strike effect.

### The fit

Rates, per metre penetrated (WT, RT, GT): v8 0.0375, 0.0425, 0.0116. With the
lens share back at 0.36 and the same rates, the file gives 7.18%, 15.59% and
20.04% of holes striking in each till (10,000 seeds), because the grey till's
later events now count and a shallower till strike no longer pre-empts the
lens. The rates are re-fitted to 0.0376, 0.0415 and 0.00995 (0.376, 0.415 and
0.0995 per 10 m). The seal gap (`gap_m`, 0 to 4.5 m, skew 1.4) is fitted to the
median and P90 of the crawl's seal gap; the cap of 4 and the 0.5 m gap are the
crawl's. Nothing else is fitted: the other figures below are reported.

### The file against the crawl

The file figures are over 40,000 seeds of 3 to 7 holes (200,336 holes, 64,122
with a strike); a second 40,000 seeds (200,150 holes) gives 7.28%, 15.04% and
17.79% in the tills and 36.69% in the lens.

| Quantity | Crawl | v9 file |
|---|---|---|
| Holes striking in WT | 7.3% | 7.30% (5,411 of 74,109) |
| Holes striking in RT | 15.0% | 14.98% (17,224 of 115,017) |
| Holes striking in GT | 17.7% | 17.71% (35,370 of 199,688) |
| Holes with the lens striking in it | 36% | 36.04% (13,062 of 36,244) |
| Holes with a strike | 44.4% of holes with GEOL | 32.0% |
| Struck holes with 1 / 2 / 3 / 4 strikes | 75.9 / 19.1 / 4.2 / 0.7% | 81.0 / 16.3 / 2.3 / 0.4% |
| Holes with 2 or more strikes | 10.7% | 6.1% (12,160 holes) |
| Later strike in the same stratum as the one above | 15% | 47.5% (5,771 of 12,160); same unit 49.1% |
| Gap between consecutive strikes, P10 / P50 / P90 (m) | 1.75 / 4.71 / 12.23 | 0.94 / 5.61 / 14.90 (14,126 pairs; least 0.50) |
| Casing at or below the strike above | 95% | 100% |
| Casing less the strike above, P10 / P50 / P90 (m) | 0.50 / 2.50 / 9.10 | 0.50 / 4.47 / 13.71 |
| Earlier strikes sealed | 48% | 47.6% (6,729 of 14,126) |
| Last strikes sealed | 25% | 24.9% (15,953 of 64,122); 25.3% in multi-strike holes |
| Seal less strike, P10 / P50 / P90 (m) | 0.00 / 1.00 / 3.44 | -0.46 / 1.09 / 3.51 (22,682 seals); 11.8% above the strike (crawl 9%) |
| Seal at or above the next strike's casing | 97% | 100% |
| Last seal at or above the final casing | not measured | 100% |
| Final casing passes the last strike | 82% in v6; 74.4% (multi-strike 72.6%) re-measured | 83.3% (multi-strike 82.1%) |
| Final casing passes every earlier strike | 96.4% | 100% |
| Rise at first strikes of multi-strike holes, P10 / P50 / P90 (m) | 0.01 / 0.25 / 1.60 | 0.00 / 0.59 / 3.54 |
| Rise at later strikes, P10 / P50 / P90 (m) | 0.20 / 1.50 / 5.40 | 0.00 / 0.94 / 4.74 |
| Installed multi-strike holes, zone nearest the first / the deepest strike | 14 / 12 (54% / 46%) | 2,625 / 2,102 (55.5% / 44.5%) |

The file's rise rows are for every source (lens, table and till); its till
strikes alone give P50 0.71 m and P90 4.25 m, a mean of 1.46 m, 43% over 1 m and
12.1% with no rise (65,185 strikes), against v8's fitted 0.68 and 4.11 m, 1.74 m,
40% and 12%.

### Known misses

- The multi-strike share (6.1% of holes against 10.7%) and the strikes-per-hole
  spread. The file has no made-ground, laminated-clay, alluvium or rock strikes;
  in the crawl 31 of the 102 first strikes are in made ground and 14 of the
  second strikes in rock. Made-ground water is #84.
- The same-stratum share (47.5% against 15%). The grey till is one thick
  stratum with a lens in it, so most pairs of grey-till events are in the same
  stratum.
- The gap between consecutive strikes is wider at the upper end (P90 14.9 m
  against 12.2 m) and closer at the lower (P10 0.94 m against 1.75 m); the
  rise at first strikes is higher than the crawl's (0.59 against 0.25 m) and
  at later strikes lower (0.94 against 1.5 m). Both draw from one rise range
  per source, as the decisions set (not fitted).
- Casing less the strike above, P50 4.47 m against 2.50 m. Each strike keeps
  #77's strike-gap draw, and the crawl's later strikes sit nearer to their own
  depth with a longer upper tail (-5.38 / -0.60 / 0.00 m) than the draw gives.
- The seal gap's P10 (-0.46 m against 0.00 m): the last strike's seal is held
  at or above the final casing, so where the final casing stops above the strike
  the seal lies above it (11.8% of seals against the crawl's 9%).
- The final casing passes the last strike in 83% of struck holes, from the
  preset's 0.82; this sample's 74.4% is lower (see above).
- The zone's offset from the first strike has a higher upper tail in multi-strike
  holes (all struck holes P10 / P50 / P90 -3.11 / +0.24 / +3.69 m against the
  crawl's -2.7 / 0.0 / +3.0 m), because 45% follow the deepest strike.

### Figures re-measured for other bases

Over the same 40,000 seeds: the final depth less the final casing, P25 / P50 /
P75 / P90 0.37 / 2.78 / 9.18 / 15.95 m; the strike less `WSTG_CAS`, P10 / P25 /
P50 / P75 / P90 0.00 / 0.05 / 0.37 / 1.10 / 1.87 m (78,248 strikes; v8 gives
0.41 m at P50, so the 0.49 m the basis quoted was out of date before v9); the lens
strike depth, P10 / P50 / P90 3.25 / 9.63 / 16.27 m; the zone top less the
strike in holes with one strike, -3.58 / -0.10 / +3.08 m (19,760 holes).

### Pins re-taken

`shipped_preset_output_bytes_are_pinned` for tyneside-till seeds 0 to 2:
`decbeb1d...dc86` (34,478 bytes), `a9d773f9...ced8` (23,513) and
`f880f79c...9431` (41,562). `a_preset_without_its_several_strikes_writes_the_previous_versions_bytes`
holds the file without the three keys, and with v8's rates and lens share, to
the v8 pins (`d51bc607...4e0f`, `231ffbc5...b284`, `a22dafe3...a5a8`).
london-basin is unchanged.
