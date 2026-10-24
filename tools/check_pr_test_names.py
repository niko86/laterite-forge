"""Fail when a PR body names a test that does not exist.

A PR body proves each change by naming the tests that cover it, so a name
that matches no `fn` in `src/` or `tests/` is a claim nothing backs. The
body comes from `$PR_BODY` (CI) or a file given as the only argument.

A name is any backticked identifier with three or more underscores
(`shipped_preset_output_bytes_are_pinned`); shorter ones are preset keys
and flags (`no_rise_share`), which this does not check.
"""

import os
import pathlib
import re
import sys

NAME = re.compile(r"`([a-z][a-z0-9]*(?:_[a-z0-9]+){3,})`")


def defined(root: pathlib.Path) -> set[str]:
    fns = set()
    for d in ("src", "tests"):
        for f in (root / d).rglob("*.rs"):
            fns.update(re.findall(r"\bfn ([a-z0-9_]+)\b", f.read_text(encoding="utf-8")))
    return fns


def main() -> int:
    if len(sys.argv) == 2:
        body = pathlib.Path(sys.argv[1]).read_text(encoding="utf-8")
    else:
        body = os.environ.get("PR_BODY", "")
    root = pathlib.Path(__file__).resolve().parent.parent
    fns = defined(root)
    missing = sorted({n for n in NAME.findall(body) if n not in fns})
    for n in missing:
        print(f"PR body names `{n}`, but no `fn {n}` exists in src/ or tests/")
    return 1 if missing else 0


if __name__ == "__main__":
    sys.exit(main())
