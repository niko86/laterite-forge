# london-basin v3: sources and calibration

This records where the values in `data/presets/london-basin.toml` (v3, #48)
come from, how they were calibrated, and what the generated files measure.
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
- The no-strike fraction comes out at about 70%, well above BGS's 32-40%.
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
