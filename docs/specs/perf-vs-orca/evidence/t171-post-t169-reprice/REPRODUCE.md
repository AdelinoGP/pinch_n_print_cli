# t171 reproduce — post-t169 re-price at HEAD

No production edit: this take measures, reduces, and re-derives arithmetic.
Ticket: [`171-post-t169-prepass-reprice.md`](../../../issues/171-post-t169-prepass-reprice.md).

## Prerequisites

- `tmp/3dbenchy.stl` (the 2026-11-12 replacement) and `tmp/base.stl`: user-supplied,
  never committed.
- `target/release/pnp_cli.exe` built from the ticket's HEAD; its SHA-256 is
  recorded in `reduced/instrumented.json` (via `runs.json`'s arm) and in the
  FINDINGS build identity section.
- No OrcaSlicer run: this ticket re-prices within the PnP tree and composes with
  t169's committed walls of record.
- `pwsh` not needed; `python` (stdlib only), a 12-logical-core box.

## 1. Freshness and build

```bash
cargo xtask build-guests --check      # 0
cargo build --release -p pnp-cli      # record the binary SHA-256
sha256sum target/release/pnp_cli.exe  # b2e8261e… (t169's candidate build)
```

## 2. The instrumented split (shares, attribution only)

```bash
cd docs/specs/perf-vs-orca/evidence/t171-post-t169-reprice
RAYON_NUM_THREADS=12 python run-instrumented.py     # 8 cells x 3 passes
```

Each pass writes `<cell>.pass<N>.{jsonl,gcode}` into `target/t171-instrumented/`
and is hash-checked against t169's active reference set; a mismatch fails the
run. All 24 passes exited 0 with `degraded: false`.

## 3. The per-op census (attribution only)

```bash
python run_profile_t171.py --tag clean              # 8 cells, one pass each
```

Writes `target/t171-profile/clean/<cell>.stderr.txt` and
`reduced/profile-clean.json`. `--profile` inflates wall and sums concurrent
spans, so no number from this pass is a wall claim; the ratios are the point.

## 4. Reduce and re-derive

```bash
python reduce_instrumented.py   # raw event streams -> reduced/instrumented.json
python arithmetic.py            # split + route + rescaled prices -> reduced/arithmetic.json
python verify.py                # 0 = every claim re-derives
python verify.py --self-test    # 8/8 negative controls rejected
```

Only `reduce_instrumented.py` needs the gitignored raw tree under `target/`;
`verify.py` is stdlib-only and reads the committed `reduced/` set plus the
sibling tickets' committed artifacts.

## Regimes, and why they are never mixed

- **Shares are instrumented.** `--instrument-stderr` inflates wall, so an
  instrumented wall is never a wall claim. This take's instrumented walls run
  +2.6 % to +6.5 % over t169's paired walls of record.
- **Stage brackets are sequential inside the prepass**, so their sum reconciles
  with the phase bracket (verified per cell, worst case +1.6 %).
- **The implied stage wall** is `share x wall of record`; it is a composition
  across two windows (shares here, walls in t169's campaign), never a
  subtraction of an instrumented bracket from an uninstrumented wall.
- **The route table** is t169's own composition (`t163 ratio x t169 saving`)
  re-derived from committed artifacts. It is an estimate, not a ratio of record;
  t161 forbids carrying a required cut across windows as a measurement.
- **S5/S6 prices are rescaled, not re-measured.** t131's census ran on a
  pre-t169 probe build, and neither this take's stage brackets nor its
  `--profile` channel separates the internal-bridge offsets from the rest of
  `PrePass::ShellClassification`, nor the guest-invoked occupancy offsets from
  anything else.
