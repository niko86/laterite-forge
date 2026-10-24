"""Print a preset's keys and values with the basis strings cut short.

A basis runs to a thousand characters or more, so reading a whole preset
to find one value costs tens of thousands. This prints one line per value,
with each basis shortened to its first words:

    python3 tools/preset_outline.py data/presets/tyneside-till.toml [filter]

`filter` keeps only lines whose path contains it (e.g. `units[GT]` or
`strike`). `--full` prints the bases in full, for reading one entry.
"""

import sys
import tomllib


def walk(v, path, out):
    if isinstance(v, dict):
        for k, x in v.items():
            walk(x, f"{path}.{k}" if path else k, out)
    elif isinstance(v, list) and v and all(isinstance(x, dict) for x in v):
        for i, x in enumerate(v):
            walk(x, f"{path}[{x.get('id', x.get('group', i))}]", out)
    else:
        out.append((path, v))


def main() -> int:
    args = [a for a in sys.argv[1:] if a != "--full"]
    full = "--full" in sys.argv
    if not args:
        print(__doc__.strip(), file=sys.stderr)
        return 5
    with open(args[0], "rb") as f:
        root = tomllib.load(f)
    want = args[1] if len(args) > 1 else ""
    out = []
    walk(root, "", out)
    for path, v in out:
        if want not in path:
            continue
        if isinstance(v, str) and "basis" in path.rsplit(".", 1)[-1] and not full:
            v = v if len(v) <= 60 else v[:60] + f"… ({len(v)} chars)"
        print(f"{path} = {v!r}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
