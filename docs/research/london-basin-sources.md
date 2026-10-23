# london-basin: sources and calibration

This records where the values in `data/presets/london-basin.toml` come from,
how they were calibrated, and what the generated files measure: the natural
units (v3, #48) first, then the Made Ground unit v4 adds on top (#42,
"Made Ground (v4)" below). v4 leaves the natural units' values as v3 set
them.
It holds aggregate statistics and accession IDs only: no raw data, no
scripts, and no individual borehole rows.

## BGS data

### Access route

- Borehole index: the BGS OGC API,
  `https://ogcapi.bgs.ac.uk/collections/agsboreholeindex/items?bbox=...&f=json`.
  Each item carries `bgs_loca_id`, the project, and `item_index_id` (the
  National Geotechnical Data Centre accession).
- AGS4 export: `https://agsapi.bgs.ac.uk/v7/ags_export/?bgs_loca_id=id1;id2`
  (at most 50 holes per request). It returns PROJ, LOCA, GEOL, SAMP, ISPT,
  WSTG/WSTD, CDIA, BKFL and PIPE, and no lab groups.
- Original donated files, which carry the lab data:
  `https://webservices.bgs.ac.uk/accessions/item/<id>`, then
  `.../accessions/download/<id>?fileName=...`.
- Both crawls were gentle: crawl 1 made about 38 requests and fetched 25 files
  (22 analysed); crawl 2 made 44 requests and fetched 12 export zips.
  `www.bgs.ac.uk/robots.txt` allows everything; the API hosts return 404 for
  `robots.txt`.

### Licence and attribution

The data are under the Open Government Licence v3. Each accession record says:
"This resource has been made available under the Open Government Licence
(OGL). The following statement should be used to acknowledge the source of
the information: 'Contains data supplied by UKRI.'" Publishing derived
statistics with that attribution is allowed.

**Contains data supplied by UKRI.**

### Crawl 1: index properties and SPT N

Accessions: 128985 (Thames Tideway West, 8 AGS3.1 files), 129866 (West
Drayton, AGS4), and exports for 121953 (Crossrail), 140726, 40721 and 126770.
124 holes.

Classification:

- Terrace gravel: `GEOL_GEOL` RTD, RTDU or KPG, or a description naming
  terrace, Taplow or Kempton; the stratum must be SAND or GRAVEL and not made
  ground.
- London Clay: `GEOL_GEOL` LC or LCF, or a description saying "London Clay".
- Weathered against unweathered: from the description, or brown above grey.
- SPT refusals are excluded.

| Quantity | n | Mean | SD | COV |
|---|---|---|---|---|
| SPT N, terrace gravel over London Clay | 142 | 26.8 | 10.6 | 0.39 (0.16-0.67 within a project) |
| SPT N, weathered London Clay | 91 | 24.1 | 10.3 | 0.43 |
| SPT N, unweathered London Clay (biased low: 77 refusals dropped) | 235 | 32.3 | 9.9 | 0.31 |
| w %, London Clay all / weathered / unweathered | 217 / 85 / 132 | 25.8 / 26.4 / 25.4 | 3.08 / 2.96 / 3.10 | 0.12 / 0.11 / 0.12 |
| LL %, all / weathered / unweathered | 204 / 79 / 125 | 68.6 / 69.7 / 68.0 | 6.58 / 6.33 / 6.68 | 0.096 / 0.091 / 0.098 |
| PL %, all / weathered / unweathered | 204 / 79 / 125 | 25.9 / 26.3 / 25.7 | 2.33 / 2.08 / 2.45 | 0.090 / 0.079 / 0.096 |

Testing rates in the 9 original files with lab data (accessions 128985 and
129866: 50 holes, 1,317 m, 2,504 SAMP rows):

| Group | Per SAMP row | Per m of London Clay |
|---|---|---|
| ISPT | 0.23 | — |
| LNMC | 0.13 | 0.25 |
| LLPL | 0.12 | 0.24 |
| TRIG | 0.08 | 0.23 |

LDEN is about 0 because these files carry bulk density in `TRIX_BDEN`.

### Crawl 2: groundwater

454 new holes, 109 projects, 74 accessions. Accessions with strikes: 163306
(Queen Mary Reservoir), 39638 (Greenwich Peninsula), 162465 (Silvertown
Tunnel), 167350, 167349, 167292, 167290 (Chelsea, Knightsbridge, Exhibition
Road), 83329 (LUL earthworks), 167414, 167131, 168650, 162665, 164271
(Southwark and Borough), 185262 (Beckton), 192346 (Limehouse). Holes with no
strike also came from Fulham, Kew, Liverpool Street, Croydon, Edmonton,
Loughton and Chigwell.

| Measure | n (projects) | Mean | SD | P10 / P50 / P90 |
|---|---|---|---|---|
| Strike depth, all | 73 (21) | 4.12 | 2.60 | 1.22 / 4.42 / 7.78 |
| Strike depth, excluding Queen Mary | 50 (20) | 5.39 | 2.14 | 2.50 / 5.05 / 8.31 |
| Strike depth below the gravel top | 73 | 0.98 | 1.17 | 0.0 / 0.60 / 2.64 |
| Level 20 minutes after the strike | 25 (11) | 4.81 | 1.85 | 2.70 / 4.40 / 7.24 |
| Rise (strike depth minus that level) | 25 (11) | 1.20 | 1.08 | 0.02 / 1.00 / 2.62 |

In holes through gravel into London Clay, within projects that log WSTG, 32%
had no strike in the gravel (107 holes, 29 projects), or 40% excluding Queen
Mary. No export carried monitoring readings (MONG/MOND).

## Literature

| Source | Numbers used |
|---|---|
| Brosse, Lo, Davis & Che (2025), Revisiting of London Clay simple correlations using a Bayesian approach, Proc. 9th ISGSR, pp. 867-871, doi:10.3850/GRF-25280825_isgsr-185-P110-cd (open access, CC-BY) | 2,015 Crossrail UU tests from 288 boreholes. Table 3, Model B, z below the top of the London Clay: mean 83.1 + 7.4z kPa; per-unit intercepts 82.5-84.0 kPa and slopes B 6.4, A3 6.8, C 7.3, A2 9.2 kPa/m; linear to about 30 m. sigma/mu about 0.32-0.36 at every depth (derived). Table 1, weathered row: n = 35, Su0 mean 90.0 kPa, SD 39.9 (COV 0.44). |
| Jones & Terrington (2011), Modelling volume change potential in the London Clay, QJEGH 44(1):109-122, doi:10.1144/1470-9236/08-112 (NORA 13629) | Table 1, area 3a (central London), modified PI, n = 7,460: mean 44, median 45, P10/P25/P75/P90 33/39/50/54. |
| Royse, Rutter & Entwisle (2009), Property attribution of 3D geological models in the Thames Gateway, London, Bull. Eng. Geol. Env. 68(1), doi:10.1007/s10064-008-0171-0 (NORA 4824) | Fig. 4, terrace-gravel SPT N (Boyne Hill 673, Lynch Hill 155, Taplow 358, Kempton Park + Shepperton 1,742): median "about 30", mostly N 10-50. Read from the box plots: medians about 25-31, P10-P90 12-59. COV about 0.5 (derived, lognormal). N uncorrected. |
| Skempton & Henkel (1957), Tests on London Clay from deep borings at Paddington, Victoria and the South Bank, Proc. 4th ICSMFE, London | Table 1, five sites: w 26, LL 76, PL 26, PI 50; density 2.0 Mg/m3 (w 26, Gs 2.74). Under the terrace gravels the brown clay "is rarely more than a few feet thick, and sometimes entirely absent". No n or SD. |
| ISSMGE-TC304 (2021), State-of-the-art review of inherent variability and uncertainty in geotechnical properties and models, doi:10.53243/R0001 | Table 1.2 (clay, site-specific COV %, mean and range): LL 15.6 (3.4-39), PL 13.5 (2.9-38.1), PI 23.5 (6.5-57), w 15.3 (3.5-46), su 28.2 (6-56). Table 1.3 (sand): SPT-N 34.3 (18.4-62). Appendix: Cooling & Skempton, London PL, n = 100, mean 26.7, COV 9.4%. |
| Bond and Harris (2008), Decoding Eurocode 7, ch. 4, s4.5.4, Figs 4.14 and 4.15, p.108; Example 4.3, p.124 | Minimum tests per stratum with no comparable experience: Atterberg 3-5, bulk density 3-4, TXL cu 3-6; Example 4.3 counts water content with the Atterberg limits. No SPT frequency. |
| ICE Manual of Geotechnical Engineering Vol. 2 (2012), ch. 63, s63.2.3, p.973, after Simpson and Driscoll 1998 | f1 4.5-5 kPa per blow for N = cu / f1. |
| ICE Manual of Geotechnical Engineering Vol. 1 (2012), p.630 | SPTs "typically every 1.5 m". |

No published regional statistics exist for the shallow water table in the
terrace deposits (BGS calls them "not well-instrumented"), so the BGS crawl is
the groundwater basis.

## Calibration method

A written result is the property model's value at the specimen times a
lognormal factor with mean 1 and the quantity's COV, exp(s e - s^2/2) with
s^2 = ln(1 + COV^2) and e a standard normal truncated at +-3
(`site/results.rs`).
The model value is a smooth field across the site mapped onto the preset's
range, plus any depth gradient, with about one stratum in fifty an outlier.
A pooled synthetic COV therefore has two parts: the spread of the model across
sites and depths, which the preset's range sets, and the result COV, a
forge-wide constant. The ranges were tuned first, then the constants:

1. Generate `gen --site` files for 400 seeds (3-7 holes each) and read the
   written rows back.
2. Assign each row to a unit from `GEOL_DESC`: principal GRAVEL or SAND is
   RTD, a grey colour is LC, otherwise WLC.
3. Pool each heading per unit (mean, COV, P10, P90). For LC cu, divide each
   TRIT_CU by the Brosse et al. mean line 83.1 + 7.4z (z below the LC top in
   that hole) and take the COV of the ratio, which is what their sigma/mu
   describes; the plain pooled COV of LC cu also carries the depth trend.
4. Adjust ranges until the means and percentiles sit on target, then raise
   each constant until the pooled COV sits at or just under its target.
   Outliers stay in the pool, as they would in real data.

The measuring script stayed outside the repository.

## Result

400 seeds, london-basin v3 with the shipped result COVs (`MOISTURE_COV`
0.115, `LIMIT_COV` 0.088, `STRENGTH_COV` 0.345, `SPT_COV` 0.42,
`DENSITY_COV` 0.055 unchanged).

| Quantity | Unit | n | Synthetic mean | Synthetic COV or P10-P90 | Target |
|---|---|---|---|---|---|
| PI (LLPL_PI) | LC | 14,571 | 44.1 | P10-P90 34-54 | mean 44, P10-P90 33-54 (Jones & Terrington) |
| PI | WLC | 347 | 43.5 | P10-P90 33-54 | same |
| PL (LLPL_PL) | LC | 14,571 | 26.0 | COV 0.089 | 26, COV about 0.09 |
| PL | WLC | 347 | 25.9 | COV 0.090 | same |
| LL (LLPL_LL), follows PL + PI | LC / WLC | 14,571 / 347 | 70.1 / 69.5 | COV 0.108 / 0.110 | not a target; BGS 68.6, COV 0.096; Skempton & Henkel 76 |
| w (LNMC_MC) | LC | 16,866 | 25.8 | COV 0.118 | 26, COV about 0.12 |
| w | WLC | 425 | 26.4 | COV 0.119 | same |
| Bulk density (LDEN_BDEN) | LC / WLC | 11,886 / 292 | 1.98 / 1.97 Mg/m3 | COV 0.055 / 0.053 | about 2.0 (check) |
| cu over the Brosse line (TRIT_CU) | LC | 11,807 | ratio 1.02 | COV 0.339 | about 0.34 |
| cu (TRIT_CU), pooled with depth | LC | 11,807 | 189 kPa | COV 0.501 | not a target |
| cu (TRIT_CU) | WLC | 304 | 92 kPa | COV 0.443 | 90, COV about 0.44 |
| SPT N (ISPT_NVAL) | RTD | 2,215 | 27.0 | COV 0.474, P10-P90 13-45 | about 27, COV 0.40-0.50 |
| SPT N, cross-check | WLC | 488 | 19.4 | COV 0.423 | BGS weathered 24.1, COV 0.43 |
| SPT N, cross-check | LC | 19,786 | 39.4 | COV 0.502 | BGS unweathered 32.3, COV 0.31 (refusals dropped) |

Groundwater, against crawl 2:

| Measure | Synthetic | BGS |
|---|---|---|
| No strike, holes through gravel into clay | 69.7% (1,260 of 1,809) | 32-40% |
| Strike depth | mean 5.30, P10-P90 3.96-6.71 m | mean 4.12, P10-P90 1.22-7.78 m |
| Level 20 minutes after the strike | mean 4.47, P10-P90 3.18-5.66 m | mean 4.81, P10-P90 2.70-7.24 m |
| Rise | mean 0.83, P10-P90 0.12-1.79 m | mean 1.20, P10-P90 0.02-2.62 m |

WLC thickness: mean 1.04 m over 1,963 holes, absent in 13% of them and under
1 m (or absent) in 50%, against Skempton & Henkel's "rarely more than a few
feet thick, and sometimes entirely absent".

Testing rates, per metre of London Clay (WLC and LC): LNMC 0.33, LLPL 0.28,
LDEN 0.23, TRIG 0.23 (BGS: 0.25, 0.24, about 0, 0.23). ISPT runs at every D
sample, about 0.39 per m in the clays and 0.33 per m in RTD. On the smallest
default site (3 holes), over 5,000 seeds, the fewest tests in London Clay
were LDEN 4, LLPL 7, LNMC 8 and TRIG 4, against the minimum of 3; a test
holds the minimum over 1,000 seeds. WLC on its own cannot meet a minimum: at
rate 1.0, 95% of 3-hole sites have fewer than 3 WLC U samples and 77% fewer
than 3 WLC samples of any type, which is why WLC and LC count as one London
Clay stratum.

## Caveats

- Thames Tideway riverside sites dominate crawl 1: D1019-11 alone gives 59 of
  the SPTs and 94 of the 217 water contents. BGS N is uncorrected, and
  weathering is inferred from descriptions.
- Queen Mary Reservoir supplies 23 of the 73 strikes. About 45% of strata
  types in crawl 2 are inferred, a missing WSTG may simply be unrecorded, and
  some final levels come from remarks.
- (v3; v4 brings it to about 45%, see "Made Ground (v4)".) The no-strike
  fraction comes out at about 70%, well above BGS's 32-40%.
  In the BGS holes the gravel starts some metres below ground (strikes average
  4.12 m below ground but 0.98 m below the gravel top), under cover forge does
  not model, whereas london-basin has no Made Ground and its gravel starts at
  the surface. A table 2.7-7.2 m down then falls
  below the gravel base in most holes. With v2's 1.0-5.0 m the fraction is
  about 40%, and with 1.0-4.0 m about 32%. Adding Made Ground (ADR 0001, slice
  8) would reconcile the BGS depths with the strike fraction.
- The synthetic rise is smaller than BGS's because a strike is clamped to the
  base of its gravel stratum.
- Brosse et al. measure z from the top of the London Clay; forge measures it
  from the top of the LC unit, below WLC, which is at most 3 m thick. Their
  fit is linear only to about 30 m, and LC here reaches up to 40 m below its
  top, so the deepest cu values run high.
- Results scatter lognormally, so none is zero or negative: over 400 seeds
  no RTD N is 0 and no LC cu falls below 15% of the Brosse line. Under
  normal scatter, at these COVs, 0.9% and 0.5% did. A lognormal result's
  median sits under its mean, by 1/sqrt(1 + COV^2): about 9% for RTD N.
- Jones & Terrington's values are modified PI (IP′, PI scaled by the
  fraction finer than 425 µm); using them as a PI target is forge judgement.
- The BGS samples are small and the literature is used for the primary basis
  wherever it has a larger dataset.

## Made Ground (v4)

v4 adds a Made Ground unit, `MG`, above RTD. The decisions behind it are the
owner's M1-M6 on #42. The natural units keep their v3 values.

### Method

- **Sample.** The holes of both crawls above, deduplicated by `BGS_LOCA_ID`:
  663 holes from 126 projects in 80 accessions. Access route and licence as
  above. **Contains data supplied by UKRI.**
- **Accessions.** 39638, 40721, 81109, 81709, 83329, 107613, 111549, 121953,
  126770, 128985, 129866, 133106, 138366, 138585, 138586, 140725, 140726,
  141089, 141090, 161518, 161519, 161567, 161905, 162207, 162345, 162409,
  162410, 162465, 162565, 162665, 162765, 163045, 163306, 163405, 164270,
  164271, 166866, 167034, 167131, 167132, 167290, 167291, 167292, 167344,
  167345, 167346, 167347, 167348, 167349, 167350, 167413, 167414, 167415,
  167951, 167952, 167953, 167954, 168212, 168533, 168549, 168650, 168963,
  168964, 169038, 169571, 169573, 173376, 185262, 185575, 186926, 188520,
  188540, 188542, 190850, 190946, 192298, 192311, 192312, 192346, 192401.
- **Made Ground.** `GEOL_GEOL` MG, MGR*, MGG, MADE*, FILL, CONC or HS, or a
  description containing "made ground" or "(fill)". An uncoded stratum also
  counts if it names brick, concrete, tarmac, ash, clinker or rubble.
  Embankment fill (mostly accession 83329, LUL earthworks) and topsoil are
  kept apart.
- **Stratum below.** From codes first, then the description; about 40% of
  these calls are inferred.
- **Thickness** runs to the base of the contiguous Made Ground. Holes that end
  in Made Ground are left out.
- **Constituents** are counted by mention: the share of the 1,288 Made Ground
  strata with a soil principal (85 projects) that name each word.

### Statistics

Presence: the first stratum is Made Ground in 73.6% of holes (80.9% leaving
out 83329), and Made Ground or fill appears in the surface sequence of 89.9%.

| Thickness subset | n (projects) | Mean | SD | P10 / P50 / P90 |
|---|---|---|---|---|
| All | 491 (98) | 2.81 | 2.31 | 0.70 / 2.10 / 5.90 |
| Over terrace gravel over London Clay | 102 (36) | 2.25 | 1.57 | 0.71 / 1.86 / 4.58 |
| Directly over London Clay | 93 (30) | 2.27 | 2.49 | 0.50 / 1.50 / 5.38 |
| Over alluvium | 199 (43) | 3.72 | 2.49 | 1.08 / 3.20 / 6.71 |

Within 500 m of the Thames the mean is 3.53 m, and beyond it 2.63 m.

| Measure, of 1,288 Made Ground soil strata | Value |
|---|---|
| Principal | CLAY 36%, GRAVEL 29%, SAND 29%, SILT 3% |
| Commonest forms | gravelly sandy CLAY 18.5%, clayey gravelly SAND 8.9%, clayey sandy GRAVEL 8.5%, sandy GRAVEL 8.2%, gravelly SAND 7.6% |
| First colour phrase | brown 23%, dark brown 19%, dark grey 7.5%, light brown 6.6%, grey 6.2%, black 5.7%; none 10% |
| Strength and density terms | very soft 2.4%, soft 15%, firm 13%, stiff 4%, very stiff 0.9%; very loose 0.9%, loose 6%, medium dense 6%, dense 1.4%, very dense 0.5% |
| Constituents in the forge vocabulary | brick 68%, concrete 39%, clinker 15%, ash 13%, pottery/ceramic/tile 10%, glass 7.8%, wood/timber 7.5%, metal 6.3%, coal 5.7%, slag 4.2%, plastic 3.8%, tarmac/asphalt 3.3% |
| Not in the vocabulary | flint 62% (mostly the lithology of gravel clasts), cobbles 18%, rootlets 13%, stone 8%, chalk 4.1% |
| Count of vocabulary constituents | 0 in 20%, 1 in 30%, 2 in 27%, 3 or more in 23% |
| Amount words | occasional 25%, rare 14%, frequent 7%, abundant 2%, numerous 0% |

SPT N in Made Ground, a cross-check for the coarse set (blank tests left out,
refusals, N 50 or more, left out): mean 12.5, COV 0.74, P10 / P50 / P90
3 / 10 / 25 (n = 434, 30 projects, 21 of the accessions). In CLAY-led strata
the mean is 10.4 (n = 146), and in SAND- or GRAVEL-led ones 13.9 (n = 201).

Groundwater context, holes with gravel directly over London Clay (n = 128, 38
projects): the gravel top lies at P10 / P50 / P90 0.71 / 1.86 / 4.70 m under
Made Ground, and 0.30 / 1.40 / 3.79 m over all such holes. Under this
classifier 46% of them have no strike in the gravel (89 holes, 28 projects),
against 32-40% from crawl 2's classifier above; the inference choices account
for the difference. The target is therefore 32-46%.

### Literature

| Source | Numbers used |
|---|---|
| Ford, Price, Cooper & Waters (2014), An assessment of lithostratigraphy for anthropogenic deposits, Geol. Soc. London Spec. Publ. 395:55-89, doi:10.1144/SP395.12 | London 3D model, Table 1: made ground mean 1.4 m, SD 2.6, maximum 26 m; made plus infilled ground mean 1.6 m, P90 3 m. Deposits under 1 m are left out of the model. Cross-check for thickness. |
| Terrington, Silva, Waters, Smith & Thorpe (2018), Quantifying anthropogenic modification of the shallow geosphere in central London, UK, Geomorphology 319:15-34, doi:10.1016/j.geomorph.2018.07.005 | Table 5, mean thickness over terrace deposits: City 3.19 m, Tower Hamlets 2.21 m. Table 4, by distance from the Thames: 3.72 m within 500 m, 2.42 m beyond. Table 2: brick and concrete dominate the logged terms. Cross-check for thickness and constituents. |
| Rosenbaum, McMillan, Powell, Culshaw & Northmore (2003), Classification of artificial (man-made) ground, Eng. Geol. 69:399-409, doi:10.1016/S0013-7952(02)00282-X | Five BGS classes: made, worked, infilled, landscaped and disturbed ground. The unit is made ground in this sense. Context only. |
| Norbury (2010), Soil and Rock Description in Engineering Practice, ch. 14, pp.212-214 | Two stages: normal soil terms first, then the constituents. Example p.214: "MADE GROUND: dense brown sandy GRAVEL with occasional tiles, wire, glass, tyres". The description form. |

### The preset's values

| Field | Value | Basis |
|---|---|---|
| `thickness_m` | -1.0 to 5.75 m, skew 1.3 | BGS over terrace over London Clay, P10 / P50 / P90 0.71 / 1.86 / 4.58 m with about 10% absent; range and skew fitted to these (see "Thickness fit"). Cross-checked against Terrington 2018 Table 5 and Ford 2014 Table 1. |
| principals, secondaries | CLAY, GRAVEL, SAND; sand, gravel, clay | BGS principal shares and commonest forms |
| colours | brown, dark brown, dark grey, grey, black, light brown | BGS first colour phrases. The vocabulary accepts all six. |
| constituent shares | brick 0.68, concrete 0.39, clinker 0.15, ash 0.13, pottery 0.10, glass 0.078, timber 0.075, metal 0.063, coal 0.057, slag 0.042, plastic 0.038, tarmacadam 0.033 | BGS mention shares |
| `count.weights` | 0.20, 0.30, 0.27, 0.23 | BGS count distribution |
| `amounts.weights` | rare 0.14, occasional 0.25, frequent 0.07, numerous 0, abundant 0.02 | BGS amount-word shares |
| `liquidity_index` | 0.1 to 0.9 | forge judgement, calibrated to the BGS consistency terms |
| `spt_n` | 3 to 18 | forge judgement, calibrated to the BGS density terms; cross-checked against BGS N in Made Ground |
| `plastic_limit_percent`, `plasticity_index_percent` | 20-26, 20-35 | forge judgement: no lab data on Made Ground in the exports |
| `moisture_content_percent` | 10-20 | forge judgement; required by the loader, never read |
| `particle_density_ratio` | 2.60-2.70 | Head Vol. 1 Table 3.1 mineral values; their use for Made Ground is forge judgement |
| sampling | B and D every 1-1.5 m | as RTD (ICE Vol. 2 p.791; Craig p.297) |
| suite | ISPT at every D sample; no lab groups (the loader refuses them, M5) | as the other units' ISPT basis |

**Mixed principals.** The unit names CLAY, SAND and GRAVEL, so the loader
requires both property sets, and the property model carries both
(`site/properties.rs`). Each stratum draws its principal uniformly. A
CLAY-led stratum leads with the consistency term from the fine set's
Ic = 1 - LI at mid-depth, and a SAND- or GRAVEL-led one with the density term
from the coarse set's SPT N. Two consequences follow:

- Moisture content, and so bulk density and the overburden it adds to TRIT
  cell pressures below, comes from the fine set whatever the principal. The
  coarse set's moisture content is required by the loader but never read.
- Every Made Ground SPT, whatever the principal, writes the coarse set's N.
  Giving the unit a `strength` and `spt.stroud_butler` would instead derive
  every N, GRAVEL-led strata included, from an undrained strength, which a
  granular fill does not have. The BGS N barely differs by principal (10.4
  against 13.9), so one N field for the unit is the honest choice.

PL and PI are forge judgement and feed only that moisture content: the
loader refuses LLPL, LNMC, LDEN and TRIG over Made Ground. Made Ground never
yields a water strike.

**Calibration.** As for the natural units: 400 seeds, rows read back, Made
Ground strata recognised by their `MADE GROUND: ` tag. LI and SPT N ranges
were swept until the leading terms sat near the BGS proportions read within
each lane. The BGS fine terms (soft 15%, firm 13%, stiff 4%) give soft and
firm about equal with a little stiff. The coarse ones (loose 6%, medium dense
6%) give loose and medium dense about equal. A smooth field is symmetric
about its range's middle, so LI cannot match both tails: BGS has more stiff
than very soft, and LI 0.1-0.9 compromises at about 6% stiff and 9% very
soft. Narrower or shifted ranges traded one tail for the other, or put soft
well under firm.

### Thickness fit

A unit's thickness may carry a `skew` (ADR 0001, "Ground"): the smooth field
value u becomes u^skew before it is placed on the range. The field is
compressed about 0.5 (P10-P90 of u about 0.23-0.77), so no range and skew can
match the BGS P10, P50, P90 and absence exactly.

Method: one 1,000-seed run with Made Ground 0-10 m and no skew gives each
hole's field value u (4,951 holes, from the written thickness). Every
candidate (min -2.5 to 0 by 0.05 m, max 4 to 10 by 0.25 m, skew 1 to 3 by
0.1) is then evaluated on those u values with the same 2 DP rounding and
absence rule as the build. The score is the sum of the relative errors of
P10, P50 and P90 (of the holes where Made Ground is present) and of the
absent fraction, with P50 and absence weighted double. The leading
candidates were then generated in full to measure the no-strike fraction,
which the fit does not see.

| min, max, skew | Score | P10 / P50 / P90 (m) | Absent | No strike, 1,000 [400] seeds |
|---|---|---|---|---|
| BGS target | 0 | 0.71 / 1.86 / 4.58 | about 10% | 32-46% |
| **-1.0, 5.75, 1.3 (shipped)** | **0.54** | **0.49 / 1.92 / 3.90** | **9.9%** | **43.6% [45.4%]** |
| -0.6, 6.75, 1.7 (best score) | 0.50 | 0.42 / 1.86 / 4.23 | 9.9% | 46.3% [47.9%] |
| -0.65, 7.25, 1.7 | 0.51 | 0.45 / 1.99 / 4.55 | 10.0% | 46.8% [47.7%] |
| -0.7, 6.75, 1.6 | 0.55 | 0.46 / 1.95 / 4.32 | 9.8% | 46.0% [47.3%] |
| -1.3, 5.25, 1.1 | 0.53 | 0.53 / 1.92 / 3.69 | 9.9% | 42.7% [44.1%] |
| -0.5, 7.5, 1.9 | 0.51 | 0.39 / 1.86 / 4.51 | 10.2% | 47.8% [48.6%] |
| -1.5, 5.0, no skew | 0.51 | 0.54 / 1.91 / 3.58 | 10.0% | 42.5% [43.8%] |
| -0.5, 9.0, 2 | 0.85 | 0.43 / 2.12 / 5.30 | 9.9% | 49.8% |
| -0.5, 4.6, no skew (first v4 draft) | 2.16 | 0.80 / 2.07 / 3.43 | 2.2% | 38.2% [40.0%] |

Scores and percentiles are from full generation over 1,000 seeds. The 60
best candidates on the field values were all generated in full this way.

The candidates that keep the no-strike fraction inside the BGS range over
both 400 and 1,000 seeds score 0.51-0.54, which is within seed noise: over
3,000 seeds the order changes (-1.3, 5.25, 1.1 scores 0.65; the shipped fit
0.66; -1.5, 5.0 with no skew 0.67). Among these ties the preset takes skew
1.3, the one with the highest P90 (3.90 m against 3.58 m unskewed).

The shipped fit's residuals are P10 -31%, P50 +3%, P90 -15% and absence
-1%. The best-scoring candidate, -0.6 to 6.75 m at skew 1.7, fits slightly
better (P50 exact, P90 -8%, P10 -41%; 0.60 over 3,000 seeds) but puts the
no-strike fraction at 46.3%, just above the top of the BGS 32-46% range
(47.9% over 400 seeds, 47.3% over 3,000). The trade-off is the owner's call:
the alternative is a one-line change to the range.

### Result

400 seeds (1,963 holes), london-basin v4. The 1,000-seed figures, where
given, are in brackets.

| Quantity | Synthetic | Target |
|---|---|---|
| **No strike, holes through gravel into clay** | **45.4% (822 of 1,809) [43.6%]** | **32-46% (BGS)** |
| Strike depth | mean 5.75, P10-P90 4.38-7.19 m | mean 4.12, P10-P90 1.22-7.78 m |
| Level 20 minutes after the strike | mean 4.75, P10-P90 3.53-5.84 m | mean 4.81, P10-P90 2.70-7.24 m |
| Rise | mean 1.00, P10-P90 0.16-2.07 m | mean 1.20, P10-P90 0.02-2.62 m |
| MG thickness, where present | P10 / P50 / P90 0.55 / 2.00 / 3.91 m, mean 2.09 [0.49 / 1.92 / 3.90, mean 2.05] | 0.71 / 1.86 / 4.58 m, mean 2.25 |
| Holes with no MG | 10.2% (201 of 1,963) [9.9%] | about 10% |
| Gravel top under MG | P10 / P50 / P90 0.55 / 1.99 / 3.92 m [0.50 / 1.92 / 3.89] | 0.71 / 1.86 / 4.70 m |
| Principal | CLAY 34%, GRAVEL 33%, SAND 33% | 36%, 29%, 29% (SILT 3%) |
| Colour | each of the six 16-18% | brown 23%, dark brown 19%, then 6-8% each |
| Leading term, CLAY-led (599 strata) [1,476] | soft 39%, firm 45%, stiff 5%, very soft 9.5%, very stiff 1% [41 / 44 / 6 / 9 / 1] | soft 15 : firm 13 : stiff 4 (47 : 41 : 12) |
| Leading term, SAND- or GRAVEL-led (1,163) [2,983] | loose 50%, medium dense 46.5%, very loose 3.5% [48 / 49 / 3] | loose 6 : medium dense 6 (50 : 50) |
| SPT N in MG (1,131) | mean 10.8, COV 0.58, P10-P90 4-19 | BGS cross-check mean 12.5, COV 0.74, P10-P90 3-25 |

Constituents, 400 seeds (1,762 Made Ground strata) [1,000 seeds, 4,459]:

| | Synthetic | Preset |
|---|---|---|
| brick | 0.646 [0.655] | 0.68 |
| concrete | 0.379 [0.377] | 0.39 |
| clinker | 0.147 [0.140] | 0.15 |
| ash | 0.114 [0.117] | 0.13 |
| pottery | 0.096 [0.096] | 0.10 |
| glass | 0.077 [0.075] | 0.078 |
| timber | 0.074 [0.077] | 0.075 |
| metal | 0.059 [0.061] | 0.063 |
| coal | 0.041 [0.051] | 0.057 |
| slag | 0.037 [0.038] | 0.042 |
| plastic | 0.035 [0.036] | 0.038 |
| tarmacadam | 0.027 [0.030] | 0.033 |
| count 0 / 1 / 2 / 3+ | 0.211 / 0.289 / 0.274 / 0.226 [0.200 / 0.293 / 0.279 / 0.229] | 0.20 / 0.30 / 0.27 / 0.23 |
| strata using rare / occasional / frequent / numerous / abundant | 0.38 / 0.58 / 0.21 / 0 / 0.08 | relative weights 0.14 / 0.25 / 0.07 / 0 / 0.02 |

The shares sum to 1.84 and the count distribution's mean is 1.76, so the fit
is a compromise and every share comes out about 5% low. Amount words are
compared as relative weights: in the synthetic strata rare, occasional,
frequent and abundant are 30%, 46%, 17% and 6% of the amounts used, against
29%, 53%, 14% and 4% in BGS. BGS strata carry fewer amount words in all
because many name their constituents inside the gravel description ("Gravel
is ... of flint, brick and concrete") rather than in a `with` clause.

Five sample descriptions:

- MADE GROUND: Firm dark brown gravelly CLAY with frequent brick, and
  occasional concrete, clinker and timber fragments.
- MADE GROUND: Loose dark grey clayey GRAVEL with abundant concrete and rare
  pottery fragments.
- MADE GROUND: Medium dense brown very clayey GRAVEL with abundant brick,
  occasional concrete, ash and glass, and rare coal fragments.
- MADE GROUND: Medium dense black very gravelly SAND with occasional brick,
  concrete and clinker, and rare ash and pottery fragments.
- MADE GROUND: Medium dense black clayey GRAVEL with occasional concrete and
  rare tarmacadam fragments.

**Natural units.** The v3 targets still hold. Every draw is keyed, so the
units' fields, thicknesses and test selection are unchanged: RTD and WLC
lengths, WLC thickness (mean 1.04 m, absent in 13%, under 1 m in 50%) and the
WLC row counts are identical. What moves is depth: every lower stratum starts
deeper by the Made Ground thickness, so LC is about 7% shorter within the
same final depths and carries fewer rows, and TRIT cell pressures carry the
Made Ground overburden. Each written result's scatter is keyed by the
sample's position in its hole, and Made Ground samples come first, so the
scatter draws below are renumbered. The pooled statistics move within noise:

| Quantity | v3 (same script, 400 seeds) | v4 | Target |
|---|---|---|---|
| PI, LC / WLC | 44.1 / 43.5, P10-P90 34-54 / 34-54 | 44.1 / 43.7, 34-54 / 34-55 | 44, 33-54 |
| PL, LC / WLC | 26.0 / 26.0, COV 0.089 / 0.088 | 26.0 / 26.0, COV 0.089 / 0.096 | 26, about 0.09 |
| w, LC / WLC | 25.8 / 26.4, COV 0.118 / 0.118 | 25.8 / 26.4, COV 0.116 / 0.119 | 26, about 0.12 |
| Bulk density, LC / WLC | 1.98 / 1.97, COV 0.055 / 0.053 | 1.98 / 1.97, COV 0.055 / 0.055 | about 2.0 |
| LC cu over the Brosse line | 1.02, COV 0.339 | 1.02, COV 0.339 | about 0.34 |
| WLC cu | 92, COV 0.443 | 91, COV 0.421 | 90, about 0.44 |
| RTD N | 27.0, COV 0.474, P10-P90 13-45 | 26.9, COV 0.467, 14-44 | about 27, 0.40-0.50 |
| Testing per m of London Clay: LNMC / LLPL / LDEN / TRIG | 0.33 / 0.28 / 0.23 / 0.23 | 0.33 / 0.28 / 0.23 / 0.23 | realism check |

`london_basin_meets_the_ec7_minimums_on_the_smallest_default_site` still
passes over 1,000 seeds. Over 400 seeds no RTD N is 0 and no LC cu falls
below 15% of the Brosse line. Every file of 1,000 seeds is Rust-clean, and no Made Ground
stratum carries a strike or a lab test.

### Caveats

- **Thickness fit.** The field is compressed about 0.5, so the fit is a
  compromise: P10 comes out 31% low and P90 15% low. A fit with a better P90
  pushes the no-strike fraction to the top of the BGS range or past it (see
  "Thickness fit"). Seed 0's site happens to lie where Made Ground is
  absent, so the seed-0 file has none.
- **Riverside bias.** The Tideway accessions dominate, and 17% of holes lie
  within 500 m of the Thames, where Made Ground is thicker.
- **Logging.** Contractors log Made Ground differently. Constituents are
  counted by mention, not weighted by amount. The stratum below is inferred
  in about 40% of calls.
- **Censoring.** Leaving out holes that end in Made Ground biases the
  thickness low.
- **Colours.** The description engine draws a colour uniformly from the
  palette, so the six colours come out about equal, not in the BGS order.
- **Properties.** No lab data on Made Ground came with the exports, so its
  fine and coarse sets are forge judgement calibrated to description terms.
  The BGS N cross-check comes from 30 projects and spreads wider (COV 0.74)
  than the synthetic N (0.56).


## SPT refusals (v5, #63)

v5 writes a refusal when a test reaches 50 blows before the 300 mm main drive:
`ISPT_NVAL` 50 (not extrapolated), `ISPT_MAIN` 50, `ISPT_NPEN` 150 plus the
penetration p reached, `ISPT_REP` "50/p"; an ordinary test writes MAIN = N,
NPEN 450 and REP "N=<N>". The convention is forge judgement (owner decision,
#63). Any drawn N of 50 or more is a refusal (p = 300 x 50 / N, at least 1 and
under 300), so every N above 50 that v4 wrote is now 50. The preset sets no
`refusal_share`, so every refusal in the file is a strength one.

### Method

The two London crawls (the 80 accessions listed under Made Ground above; 169 AGS files; 668 holes after de-duplicating by
position and depth) were re-read for ISPT rows. A refusal is an `ISPT_NVAL` of
50 or more, a `*` in `ISPT_REP`, or a main drive under 300 mm given as "for
NNmm" in the report string. Each test was assigned to the stratum at its top
plus 0.225 m, classified as in the earlier method (RTD from codes or
terrace-gravel descriptions and a granular description; LC from code or
description). Aggregates only.

### Result

| Unit | Crawl tests | Crawl refusals | File refusals (strength only) |
|---|---|---|---|
| RTD | 541 (42 files) | 46 (8.5%; file bootstrap 5.2-13.5%) | 5.5% |
| LC | 977 (34 files) | 140 (14.3%) | 22.5% |
| WLC | not measured | not measured | 0.5% |
| MG | not measured | not measured | 0.04% |

- **RTD gets no share.** Of the 46 RTD refusals, 44 have an N of 50 or more and
  only 2 report an N under 50, so the crawl shows no obstruction refusals
  beyond the strength ones. The file's strength rule gives 5.5%, inside the
  bootstrap interval (resampling files, 5,000 draws). The non-refusal tests
  have mean 24.9, SD 10.3 (n = 412) in the crawl and 25.0, SD 9.7 in the file.
- **LC is strength only** (owner decision), and f1 is not refitted. The file
  refuses 22.5% of London Clay tests against 14.3% in the crawl, with a
  non-refusal mean N of 29.9 against 29.1. f1 is held to the cited 4.5-5
  (ICE Manual Vol. 2, ch. 63, s63.2.3, p.973). At f1 = 5, the top of that
  range, the file gives mean 29.2 and 19.3% refusals, so no f1 in the range
  gives both targets and v4's range is kept; cu (Brosse et al. 2025) is
  untouched. The gap is the depth mix of the tests, not the rate at depth. By
  depth below ground the crawl refuses 0/74 (0-5 m), 1/213 (5-10 m), 8/184
  (10-15 m), 19/257 (15-25 m) and 112/249 (over 25 m): from 15 m down 131 of
  506 tests (26%), against 29% in the file at f1 = 5, but only 52% of the
  crawl's LC tests are that deep against 63% of the file's.
- **MG** takes no share (forge judgement): it is fill, and the crawl was not
  read for its refusals.
- File rates are pooled over 100,000 seeds of the default hole counts.


## Cover layer (v6, #58)

v6 adds a `[cover]` block: each hole draws hard surfacing (25.2%), topsoil
(12.8%) or nothing, and the cover takes its thickness off the top of the
hole's first stratum (ADR 0001, "Cover layer"). The data are the Made Ground
(v4) crawls above: the 80 accessions of the two crawls, 663 holes from 126
projects, with the strata classified as in the Made Ground method (codes
first, then descriptions). Contains data supplied by UKRI, OGL v3. Aggregates
only.

### Method

For each hole the first stratum after any leading gap (core loss, no
recovery) is taken. It is surfacing when it is Made Ground whose description
opens (after "MADE GROUND:" or "(MADE GROUND)") with concrete, tarmac,
asphalt, macadam, bitumen, paving, setts, floor slab or screed. It is topsoil
when its code is TS, TOP, TPS or TOPSOIL, or its description names topsoil,
soil, turf or grass and the stratum is not natural. Surfacing thickness is
taken from the surfacing strata that name no soil principal (n = 196, any
position in the surface sequence), topsoil thickness from the 40 holes with
topsoil and no Made Ground below. Kinds are counted on the first-stratum
surfacing descriptions, with tarmac, asphalt, macadam and bitumen as one kind,
concrete, floor slab and screed as one, paving and setts as one. Topsoil
colours and fractions are read from the first-stratum topsoil descriptions
(85): 32 name a colour and 21 name a soil with minor fractions, the rest being
drillers' "TOPSOIL" or "Grass and TOPSOIL".

### Targets and the fit

| Quantity | Crawl | File (7,401 holes, seeds 0-1499) |
|---|---|---|
| Holes starting with surfacing | 167 of 663 (25.2%) | 1,748 (23.6%) |
| Holes starting with topsoil | 85 of 663 (12.8%) | 1,008 (13.6%) |
| Topsoil over Made Ground | 6.8% of holes | arises where a topsoil hole has Made Ground below |
| Surfacing thickness P10 / P50 / P90 | 0.07 / 0.20 / 0.60 m | 0.07 / 0.19 / 0.59 m |
| Topsoil thickness P10 / P50 / P90 | 0.20 / 0.43 / 1.11 m | 0.20 / 0.43 / 1.10 m |

The file's shares are 1.6 percentage points below and 0.8 above the crawl's,
about two standard deviations of a binomial over 7,401 holes in each case
(the draw is unbiased: 200,000 seeds at each of holes 0 to 6 give 25.0% and
12.8%). A cover cut back to leave the first stratum 0.05 m removes 1.1% of
surfacing and 0.7% of topsoil draws.

The thickness is `min + (max - min) x v^skew` for a uniform `v` and was fitted
by simulating the files, cap included (forge analysis):

| Layer | min | max | skew | Uncapped P10 / P50 / P90 | File P10 / P50 / P90 |
|---|---|---|---|---|---|
| Surfacing | 0.065 | 0.75 | 2.3 | 0.07 / 0.20 / 0.60 | 0.07 / 0.19 / 0.59 |
| Topsoil | 0.20 | 1.50 | 2.0 | 0.21 / 0.53 / 1.25 | 0.20 / 0.43 / 1.10 |

The topsoil range is wider than the crawl's percentiles alone give because
the first stratum caps it (uncapped, the range gives a median of 0.53 m): the uncapped fit (0.19 to 1.35 at skew 2.3, which
matches 0.20 / 0.43 / 1.11) gives 0.19 / 0.36 / 0.96 m in the file, where
Made Ground is often thin. The fit was found by sweeping range and skew over
1,200 seeds; neighbours (0.2 to 1.55 at 2.1, 0.2 to 1.6 at 2.3) give a median
of 0.40 to 0.43 and a P90 of 1.09 to 1.12 m.

Surfacing kinds, as a share of surfacing holes:

| Kind | Crawl (167) | Weight | File (1,748) |
|---|---|---|---|
| Tarmacadam (tarmac, asphalt, macadam, bitumen) | 77 (46.1%) | 77 | 839 (48.0%) |
| Concrete (concrete, floor slab, screed) | 72 (43.1%) | 72 | 748 (42.8%) |
| Paving slabs (paving, setts) | 18 (10.8%) | 18 | 161 (9.2%) |

The crawl words paving as "Paving stones" (5) and "paving slab" or "slabs"
(5), and once as blocks or cobble sets; "Paving slabs" is the one wording
the file uses.

Topsoil colours (of the 32 with a colour; orange and red, once each, are left
out as not topsoil colours, so the weights are 30 of 32):

| Colour | Crawl | File (1,008) |
|---|---|---|
| dark brown | 14 (46.7%) | 46% |
| brown | 9 (30.0%) | 31% |
| grey | 3 (10.0%) | 10% |
| light brown | 2 (6.7%) | 7% |
| dark grey | 1 (3.3%) | 3% |
| black | 1 (3.3%) | 3% |

Minor fractions, as the chance each is named in a description (of the 21 with
a soil description):

| Fraction | Crawl | File |
|---|---|---|
| gravelly | 16 (76%) | 75% |
| sandy | 10 (48%) | 49% |
| clayey | 9 (43%) | 43% |
| silty | 3 (14%) | 13% |

### Caveats

- The qualifier (slightly, plain, very) is a forge draw (a percentage of 1 to
  70, shared 80%, banded as BS 5930 Table 17's fine-soil column), not a
  measure: the crawl's wording splits about evenly between "slightly" and
  plain.
- "With rootlets" is on every topsoil. The crawl names rootlets in 19 of 85
  (22%), so this is forge judgement (a topsoil is described with them).
- The Made Ground unit's thickness fit is unchanged. The crawl's Made Ground
  thickness includes surfacing, so a hole that draws surfacing has its Made
  Ground (as written) thinner by the surfacing.
- Combined surfaces in the crawl ("Tarmac overlying concrete", "paving
  slabs over sand over asphalt") are counted as one kind, tested in the order
  tarmac, asphalt, macadam or bitumen; then paving or setts; then concrete,
  floor slab or screed, wherever the word falls in the description.
- The description format follows Norbury 2010's Made Ground convention (ch.
  14, pp.208-214) and is forge judgement; the crawl was read for its wording,
  not for a standard's.
