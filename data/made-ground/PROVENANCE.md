# Made-ground constituent vocabulary

`constituents.json` is **forge-authored**. It is not vendored and not taken
from any standard or third-party source; the list reflects common UK logging
practice (the recorded `basis`). It is plain vocabulary: man-made materials
that appear as tertiary constituents of made ground (`with frequent brick
fragments`).

The amount terms (rare / occasional / frequent / numerous / abundant) are not
kept here. They come from the vendored `tertiaryAmounts` in
`data/bs5930/terms.json`.

The owner reviews this list. Keep it separate from `data/bs5930/`, which is
vendored third-party data with its own PROVENANCE.

## Extended words

`pottery` and `coal` are listed under `extended`, apart from the default
`constituents`. A preset's weighted constituents (ADR 0001, "Made-ground
units") may name any word of either list. The default draw picks only from
`constituents`, in their listed order, so adding a word to `extended` leaves
that draw byte-identical. Never append to `constituents`: that list's order is
part of the pinned default draw. Like the rest, the two words are forge-authored
plain vocabulary.

## Settings

`settings.json` holds the generic made ground's tables (a preset's
`[made_ground]` block, ADR 0001, "Generic made ground", #86): per setting the
share of holes with made ground, the thickness P10, P50 and P90 at a town and
the make-up weights, and per settlement the shifts. It is **forge analysis** of
the BGS NGDC AGS crawl (NE England and London) plus forge judgement, and every
value carries its own `basis`; a basis citing the crawl carries "Contains data
supplied by UKRI, OGL v3". The figures are aggregates only
(`docs/research/tyneside-till-sources.md`, "Generic made ground by setting").
The owner reviews them. A change to a value changes what `--setting` and
`--settlement` write, so bump the version of every preset with the block.
