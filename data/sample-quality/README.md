# Sample quality table

`table.json` is a forge-authored draft for owner review. It has not been checked against the standards it names.

- `tests`: each SAMP-child lab-test group the `wide` scaffold emits, with `min_class`, the worst quality class (1 best, 5 worst) of sample it can be run on.
- `samp_types`: each `SAMP_TYPE` code in the bundled v4.2 dictionary, with `class`, the best quality class it can deliver. `null` means not a soil sample for lab testing (water, gas, concrete).
- `basis` is `BS EN 1997-2`, `BS EN ISO 22475-1`, both, or `forge judgement`. It never carries a table or clause number.

A sample may carry a test when its class is no worse than the test's `min_class`. Lookup and checks are in `src/sample_quality.rs`. Nothing uses the table yet; selecting `SAMP_TYPE` by unit is the rest of niko86/laterite-forge#3.
