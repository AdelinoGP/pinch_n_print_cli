# Reproduce the ticket-124 scoreboard

Everything runs from a plain checkout. The only gitignored inputs are the two
user-supplied fixtures (`tmp/3dbenchy.stl`, `tmp/base.stl`) and the Orca vendor
tree (`target/orca-vendor/`); the campaign's raw G-code and per-run CSVs land
under `target/matched-pair/` and are not committed. OrcaSlicer 2.5 must be
installed at `C:\Program Files\OrcaSlicer\orca-slicer.exe`.

## 1. Verify the committed numbers

```bash
python docs/specs/perf-vs-orca/evidence/t124-post-t123-scoreboard/verify.py --self-test
```

Exits 0 with a PASS line and ten rejected negative controls. The positive checks
re-derive the roster, the PnP output identity, the Orca pins, the medians and
ratios from `scoreboard.csv`, and compare every PnP hash to the committed t123
candidate set (`../t123-facet-retirement/reduced/summary.json`).

## 2. Prepare the arms and the gates

```bash
cargo build --release -p pnp-cli
cargo xtask dist --accelerated --edition developer
pwsh -NoProfile -File docs/specs/perf-vs-orca/evidence/orca-rig-repair/prepare_orca_vendor.ps1
cargo xtask build-guests --check               # exit 0 required
cargo xtask build-guests --accelerated --check # exit 0 required
```

The committed take's binaries are `7737a74f…` (ordinary) and `a6ae2adc…`
(accelerated). The rig's preflight also runs `module diagnose` on both arms
(24 external modules, provenance `external`) and fails closed on the vendor
manifest.

## 3. Run the campaign

A fresh take can run all eight cells in one batch:

```bash
pwsh -NoProfile -File docs/specs/perf-vs-orca/evidence/matched-pair/run_scoreboard.ps1 \
  -BatchId t124-repro
```

`-File` binds `-Fixtures`/`-Cells` as single strings, so pass no arrays (the
script defaults are the full matrix) or drive the script through `-Command`.
The committed take used three batches because the host killed the first one
mid-cell:

```bash
pwsh -NoProfile -File .../run_scoreboard.ps1 -BatchId t124-post-t123
pwsh -NoProfile -File .../run_scoreboard.ps1 -Fixtures base -Cells classic-on  -BatchId t124-resume-base-classic-on
pwsh -NoProfile -File .../run_scoreboard.ps1 -Fixtures base -Cells arachne-on -BatchId t124-resume-base-arachne-on
```

Each batch writes `target/matched-pair/<batch>/scoreboard-rows.csv`, 12 rows
per cell (1 warmup + 3 measured × 3 tools).

## 4. Reduce and re-verify

```bash
python docs/specs/perf-vs-orca/evidence/t124-post-t123-scoreboard/reduce_t124.py \
  --rows target/matched-pair/t124-post-t123/scoreboard-rows.csv \
  --rows target/matched-pair/t124-resume-base-classic-on/scoreboard-rows.csv \
  --rows target/matched-pair/t124-resume-base-arachne-on/scoreboard-rows.csv \
  --out-dir docs/specs/perf-vs-orca/evidence/t124-post-t123-scoreboard
python docs/specs/perf-vs-orca/evidence/t124-post-t123-scoreboard/verify.py
```

The reducer requires every cell to be complete in exactly one batch; a partial
cell is discarded whole and recorded in `discarded_partial_sources`. With a
single fresh batch, pass that batch's CSV once.
