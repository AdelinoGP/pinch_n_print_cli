# Reproduce the ticket-123 campaign

Everything runs from a plain checkout. The only gitignored inputs are the two
user-supplied fixtures (`tmp/3dbenchy.stl`, `tmp/base.stl`); the campaign's raw
G-code and CSVs land under `target/t123/` and are not committed.

## 1. Verify the committed numbers

```bash
python docs/specs/perf-vs-orca/evidence/t123-facet-retirement/verify.py --self-test
```

Exits 0 with a PASS line and the list of rejected negative controls. The
positive checks re-derive every headline number from `reduced/summary.json` and
the committed t116 reference set.

## 2. Binaries

| arm | provenance | sha256 |
| --- | --- | --- |
| baseline ordinary | `cargo build --release -p pnp-cli` at `270ec239` | `abad35997ddeefb9e8512e0ebbe1ca1d67b386ce930a1949e647b46fbbe35c83` |
| baseline accelerated | `cargo xtask dist --accelerated --edition developer` at `270ec239` | `40fab99220b5484a8f0dcfbd65e7fddfcb93b2d4c541be63250148027c544a20` |
| candidate ordinary | `cargo build --release -p pnp-cli` on the candidate tree | `7737a74fd9f7ea012681ba83e9afdcf511d067fc50a0f919cef8b877b35759b0` |
| candidate accelerated | `cargo xtask dist --accelerated --edition developer` on the candidate tree | `a6ae2adc998572217664a124ac0ac62c58977cfd46804379dcf5153fbf53e41b` |

Build the baseline before editing the tree, or in a detached worktree at
`270ec239`. Copy the four artifacts into the layout `run_t123.ps1` expects:

```
target/t123-baseline/pnp_cli-ordinary.exe
target/t123-baseline/accelerated/pnp_cli.exe
target/t123-baseline/accelerated/modules/
target/t123-candidate/pnp_cli-ordinary.exe
target/t123-candidate/accelerated/pnp_cli.exe
target/t123-candidate/accelerated/modules/
```

Guest freshness is a gate, not a courtesy:

```bash
cargo xtask build-guests --check              # exit 0 required
cargo xtask build-guests --accelerated --check # exit 0 required
```

## 3. Run the campaign

```bash
pwsh -NoProfile -File docs/specs/perf-vs-orca/evidence/t123-facet-retirement/run_t123.ps1 -Mode output -BatchId output
pwsh -NoProfile -File docs/specs/perf-vs-orca/evidence/t123-facet-retirement/run_t123.ps1 -Mode ab -Pairs 3 -BatchId ab
```

`-Mode output` writes `target/t123/output/rows.csv` (32 rows: 8 cells × 4 arms),
`-Mode ab` writes `target/t123/ab/rows.csv` (48 rows: 3 interleaved pairs ×
8 cells × 2 ordinary arms). G-code lands in the matching `gcode/` directory.

## 4. Audit the delta

```bash
mkdir -p target/t123/audit
for c in benchy-classic-off benchy-classic-on benchy-arachne-off benchy-arachne-on \
         base-classic-off base-classic-on base-arachne-off base-arachne-on; do
  python docs/specs/perf-vs-orca/evidence/t123-facet-retirement/audit_delta.py \
    --baseline "target/t123/output/gcode/$c-baseline-ordinary.gcode" \
    --candidate "target/t123/output/gcode/$c-candidate-ordinary.gcode" \
    --cell "$c" --out "target/t123/audit/$c.json"
done
```

Every row must report `structural_geometry_lines=0`. A non-zero value is the
ticket's stop rule firing, not noise.

## 5. Reduce and re-verify

```bash
python docs/specs/perf-vs-orca/evidence/t123-facet-retirement/reduce_t123.py \
  --output-rows target/t123/output/rows.csv \
  --ab-rows target/t123/ab/rows.csv \
  --audit-dir target/t123/audit \
  --candidate-binary-sha256 7737a74fd9f7ea012681ba83e9afdcf511d067fc50a0f919cef8b877b35759b0 \
  --baseline-binary-sha256 abad35997ddeefb9e8512e0ebbe1ca1d67b386ce930a1949e647b46fbbe35c83
python docs/specs/perf-vs-orca/evidence/t123-facet-retirement/verify.py
```

The reducer writes `reduced/summary.json`; the verifier re-derives the 16-cell
equality, the baseline reproduction of the t116 candidate set, the delta audit
and the paired-gate arithmetic from it.
