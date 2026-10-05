# t17 reproduction kit

Ticket: [clipper2 cost/output verdict](../../../issues/17-clipper2-cost-output-verdict.md).
These are the scratch harnesses used for the 2026-09-24 A/B, preserved in-repo
so the evidence can be re-derived (the map's evidence policy: every cited file
lives under `evidence/`).

## Output diff (`outdiff-main.rs`, `outdiff-Cargo.toml`)

Scratch crate that dumps every clipper result to canonical JSON. Layout:

```
<scratch>/t17-outdiff/
  Cargo.toml     <- from outdiff-Cargo.toml
  src/main.rs    <- from outdiff-main.rs
```

The `Cargo.toml` **pins `clipper2-rust = "=1.1.0"` deliberately**: without a
pin, the crate's own lock floats slicer-core's caret requirement up to the
newest compatible release (1.2.0) and the run silently measures the wrong
version. Assert the resolved version from the harness's own `Cargo.lock` before
trusting a run.

```
# main tree (1.1.0):
cargo run --release -- --out <outdir-110> \
  --stl resources/regression_wedge.stl --model tmp/3dbenchy.stl --tag benchy

# throwaway worktree pinned to 1.0.3 (Cargo.toml: clipper2-rust = "=1.0.3"):
git worktree add .worktrees/<name> HEAD
# ... edit .worktrees/<name>/Cargo.toml, copy the harness in, same invocation
```

Then `diff -rq <outdir-103> <outdir-110>` and
`find <dir> -name '*.json' | sort | xargs cat | sha256sum` on both.

## Cost A/B (`interleave.py`, `combine_orders.py`)

Alternates two bench binaries, one criterion baseline per run, and reports
per-run paired deltas. Build both binaries with
`cargo bench -p slicer-core --bench polygon_ops --no-run`, then point the
harness at them — either via the `T17_EXE_A` / `T17_EXE_B` / `T17_LABEL_A` /
`T17_LABEL_B` / `T17_HOME` env overrides, or by editing `EXE_103` / `EXE_110`.
`--reverse` swaps which side runs first, to test for within-round load drift
(whichever side runs second can inherit a trend).

Three passes were used for ticket 17 and all three summaries are preserved as
JSON in the parent directory:

- forward (1.0.3 first): `cost-interleaved-forward.json`
- reverse (1.1.0 first): `cost-interleaved-reverse.json`
- **same-version cross-tree control** (both 1.1.0): `cost-control-same-version.json`
  — this one establishes the machine's resolution floor; it is the row that
  makes the other two readable.

`combine_orders.py` averages the forward and reversed per-leaf medians to
cancel order drift (`cost-order-cancelled.json`).

Each filter is invoked separately (criterion's positional arg is a substring
match, not a regex) and `--bench` is required or criterion runs in test mode
and writes nothing.

`tabulate.py <homeA> <homeB> [labelA] [labelB]` reads two `CRITERION_HOME`
trees and prints means **and** slopes side by side — t15 finding #1: the
console `time:` line is the slope in Linear mode; never mix statistics.

## Codegen comparison (`compare_bodies.py`)

`RUSTFLAGS='--emit=asm'` for the clipper2 crate at each version (see
[../CODEGEN.md](../CODEGEN.md)), then
`python compare_bodies.py <asm-103> <asm-110>` compares per-function bodies
after normalizing symbol hashes, anonymous-constant names and local labels.
The probe needs to force the exact call surfaces (boolean op, union,
`inflate_paths_64`, `ClipperOffset::execute_tree`, `Clipper64::execute`) or
rustc emits nothing for them.

## Corpus volume (`corpus_volume.py`)

`python corpus_volume.py <outdir>` prints the per-group / total fixture volume
used as the corpus's non-triviality check.
