# laterite-forge

`laterite-ags4-forge` generates AGS4 files and checks them with two
validators: [laterite](https://github.com/niko86/laterite)'s Rust validator,
run in-process, and the official python-ags4. It can:

- synthesize a valid AGS4 file from the standard dictionary, at any size;
- plant one or more rule violations on purpose (`--inject`, `--combine`);
- edit an existing file with structured operations (`edit`);
- run both validators over existing files and report where they disagree
  (`check`);
- search for new disagreements and shrink each one to a minimal file (`run`,
  `mine`, `minimize`).

## Install

Download the archive for your platform from the
[releases page](https://github.com/niko86/laterite-forge/releases), check it
against `SHA256SUMS`, and put `laterite-ags4-forge` on your `PATH`.

| Platform | Archive |
|---|---|
| Linux x86_64 | `laterite-ags4-forge-<version>-x86_64-unknown-linux-gnu.tar.gz` |
| macOS (Apple Silicon) | `laterite-ags4-forge-<version>-aarch64-apple-darwin.tar.gz` |
| Windows x86_64 | `laterite-ags4-forge-<version>-x86_64-pc-windows-msvc.zip` |

To build from source instead:

```sh
cargo build --release
```

## Use

```sh
laterite-ags4-forge --help      # every subcommand and its exit codes
laterite-ags4-forge --readme    # the full CLI guide
```

For example, write a synthesized file with one Rule 10a violation planted:

```sh
laterite-ags4-forge gen --scaffold loca-samp --inject rule10a --out-dir forge-runs
```

## The python-ags4 side

Every subcommand works without python-ags4 and reports a Rust-only result.
The comparison with python-ags4 needs [uv](https://docs.astral.sh/uv/) and a
checkout of this repo built from source:

```sh
uv sync
cargo run --release -- check vendor/pyags4-tests/example1.ags
```

`uv sync` installs the pinned python-ags4 from `pyproject.toml`.

## Licence

MIT, with third-party data notices in [LICENSE](LICENSE). The design record
for forge is in [docs/](docs/README.md).
