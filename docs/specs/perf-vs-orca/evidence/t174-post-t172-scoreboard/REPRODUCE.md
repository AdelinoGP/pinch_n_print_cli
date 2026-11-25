# t174 reproduce — post-t172 three-tool eight-cell scoreboard refresh

No production change: this ticket measures and records the post-t172 tree
(`48a5e583` / HEAD `0ce4be39`) across OrcaSlicer 2.5.1, PnP ordinary, and PnP
accelerated.

## Prerequisites

- `tmp/3dbenchy.stl` (the 2026-11-12 replacement) and `tmp/base.stl`:
  user-supplied, never committed.
- OrcaSlicer **2.5.1** installed at `C:\Program Files\OrcaSlicer\orca-slicer.exe`
  (sha256 `5013b1bd…`, vendor profile stamp `02.08.00.16` materialized at
  `target/orca-vendor/`).
- Release ordinary binary at `target/release/pnp_cli.exe` (`cargo build
  --release --bin pnp_cli`) and ordinary core modules in `modules/core-modules/`.
- Accelerated developer snapshot at `target/dist-accelerated/developer/`
  (`cargo xtask dist --accelerated --edition developer`).
- `pwsh`, `python` (stdlib only), quiet 12-logical-core Windows box (no heavy
  background CPU/RAM consumers; see `FINDINGS.md` §6 for the loaded-window
  artifact).

## 1. Freshness and preflight

```bash
cargo xtask build-guests --check                 # exit 0
cargo xtask build-guests --accelerated --check   # exit 0
cargo build --release --bin pnp_cli              # exit 0
```

Confirm both binaries pass `module diagnose` (24 modules, provenance `external`):

```bash
target/release/pnp_cli.exe module diagnose --module-dir modules/core-modules --json
target/dist-accelerated/developer/pnp_cli.exe module diagnose --module-dir target/dist-accelerated/developer/modules --json
```

## 2. Run the quiet-box campaign

```bash
pwsh -NoProfile -File docs/specs/perf-vs-orca/evidence/t174-post-t172-scoreboard/run_t174.ps1 \
  -BatchId t174-quiet-board \
  -LoadCsv target/t174-quiet-board-load.csv
```

`run_t174.ps1` starts a 20 s background CPU + free-RAM sampler, invokes
`docs/specs/perf-vs-orca/evidence/matched-pair/run_scoreboard.ps1` with
`-Fixtures benchy,base -Cells classic-off,classic-on,arachne-off,arachne-on
-Tools orca,pnp-ordinary,pnp-accelerated -Runs 3 -Warmups 1`, and stops the
sampler on exit. This writes **96 data rows** (24 warmups + 72 measured) to
`target/matched-pair/t174-quiet-board/scoreboard-rows.csv` and the load samples
to `target/t174-quiet-board-load.csv`.

*(The earlier loaded-box campaign used the same script with `-BatchId
t174-post-t172-scoreboard -LoadCsv target/t174-load.csv` and is preserved as
`reduced/loaded-window.json` + `reduced/loaded-window-load-samples.csv`.)*

## 3. Reduce and verify

```bash
cp target/t174-quiet-board-load.csv docs/specs/perf-vs-orca/evidence/t174-post-t172-scoreboard/reduced/load-samples.csv
python docs/specs/perf-vs-orca/evidence/t174-post-t172-scoreboard/reduce.py
python docs/specs/perf-vs-orca/evidence/t174-post-t172-scoreboard/verify.py
python docs/specs/perf-vs-orca/evidence/t174-post-t172-scoreboard/verify.py --self-test
```

- `reduce.py` reads the gitignored campaign CSVs under `target/matched-pair/`
  plus `docs/specs/perf-vs-orca/evidence/t172-offset-concave-crossing/reference.json`
  and writes `reduced/summary.json` (and `reduced/loaded-window.json` when the
  first take's raw CSV is present).
- `verify.py` is stdlib-only and checkout-only: it reads only the committed
  files under `reduced/` plus `t172`'s `reference.json` and does not touch
  `target/`. Expected output: `PASS: 1280/1280 checks` and
  `self-test: 18/18 mutations rejected`.

## 4. Restore

Nothing to restore: zero production files were edited. Re-check
`cargo xtask build-guests --check` and
`cargo xtask build-guests --accelerated --check` (both exit 0).
