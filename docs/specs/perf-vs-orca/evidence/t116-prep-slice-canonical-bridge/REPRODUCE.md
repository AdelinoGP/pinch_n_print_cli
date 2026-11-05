# Reproduce the ticket-116 campaign

Everything runs from a plain checkout. The only gitignored inputs are the two
user-supplied fixtures (`tmp/3dbenchy.stl`, `tmp/base.stl`); the campaign's raw
G-code and CSVs land under `target/t116/` and are not committed.

## 1. Verify the committed numbers

```bash
python docs/specs/perf-vs-orca/evidence/t116-prep-slice-canonical-bridge/verify.py --self-test
```

Exits 0 with a PASS line and the list of rejected negative controls. The
positive checks re-derive every headline number from `reduced/summary.json` and
the committed t115 reference set.

## 2. Binaries

| arm | provenance | sha256 |
| --- | --- | --- |
| baseline ordinary | `cargo build --release -p pnp-cli` at `456bd349` | `2fbfec0a653c84c0c8f96a687a536ba289d3fee454f35eb7b8169d65a34fafe9` |
| baseline accelerated | `cargo xtask dist --accelerated --edition developer` at `456bd349` | `8e70f4c5b8ebfc42e2fddadf8adf506d95d59a31f898d279272e0e320df12106` |
| candidate ordinary | `cargo build --release -p pnp-cli` on the candidate tree | `0ef2dcb77cdbef6556d5ff370ff34d2aac2e78e448761d5f4d6f08093e6ff622` |
| candidate accelerated | `cargo xtask dist --accelerated --edition developer` on the candidate tree | `05ebe21e87c5d63b0d737bb42ee0cdb06029bc24a151408ace7b60fa9d35978d` |

Rebuild the baseline in a detached worktree so the working tree's candidate is
not staged over it:

```bash
git worktree add ../pnp-t116-baseline 456bd349
cd ../pnp-t116-baseline
cargo build --release -p pnp-cli
cargo xtask dist --accelerated --edition developer
```

Copy the four artifacts into the layout `run_t116.ps1` expects:

```
target/t116-baseline/pnp_cli-ordinary.exe
target/t116-baseline/accelerated/pnp_cli.exe
target/t116-baseline/accelerated/modules/
target/t116-candidate/pnp_cli-ordinary.exe
target/t116-candidate/accelerated/pnp_cli.exe
target/t116-candidate/accelerated/modules/
```

Guest freshness is a gate, not a courtesy:

```bash
cargo xtask build-guests --check              # exit 0 required
cargo xtask build-guests --accelerated --check # exit 0 required
```

## 3. Run the campaign

```bash
pwsh -NoProfile -File docs/specs/perf-vs-orca/evidence/t116-prep-slice-canonical-bridge/run_t116.ps1 -Mode output -BatchId output
pwsh -NoProfile -File docs/specs/perf-vs-orca/evidence/t116-prep-slice-canonical-bridge/run_t116.ps1 -Mode ab -Pairs 3 -BatchId ab
```

`-Mode output` writes `target/t116/output/rows.csv` (32 rows: 8 cells × 4 arms),
`-Mode ab` writes `target/t116/ab/rows.csv` (48 rows: 3 interleaved pairs ×
8 cells × 2 ordinary arms). G-code lands in the matching `gcode/` directory.

## 4. Audit the delta

```bash
mkdir -p target/t116/audit
for c in benchy-classic-off benchy-classic-on benchy-arachne-off benchy-arachne-on \
         base-classic-off base-classic-on base-arachne-off base-arachne-on; do
  python docs/specs/perf-vs-orca/evidence/t116-prep-slice-canonical-bridge/audit_delta.py \
    --baseline "target/t116/output/gcode/$c-baseline-ordinary.gcode" \
    --candidate "target/t116/output/gcode/$c-candidate-ordinary.gcode" \
    --cell "$c" --out "target/t116/audit/$c.json"
done
```

Every row must report `structural_geometry_lines=0`. A non-zero value is the
ticket's stop rule firing, not noise.

## 5. Reduce and re-verify

```bash
python docs/specs/perf-vs-orca/evidence/t116-prep-slice-canonical-bridge/reduce_t116.py \
  --output-rows target/t116/output/rows.csv \
  --ab-rows target/t116/ab/rows.csv \
  --audit-dir target/t116/audit \
  --candidate-binary-sha256 0ef2dcb77cdbef6556d5ff370ff34d2aac2e78e448761d5f4d6f08093e6ff622 \
  --baseline-binary-sha256 2fbfec0a653c84c0c8f96a687a536ba289d3fee454f35eb7b8169d65a34fafe9
python docs/specs/perf-vs-orca/evidence/t116-prep-slice-canonical-bridge/verify.py
```

The reducer writes `reduced/summary.json`; the verifier re-derives the 16-cell
equality, the baseline reproduction, the delta audit and the paired-gate
arithmetic from it.
