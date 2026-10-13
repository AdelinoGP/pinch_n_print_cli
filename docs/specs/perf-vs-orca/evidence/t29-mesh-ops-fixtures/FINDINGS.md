# t29 — mesh_ops fixture limits: repair and decimate leaves now measure success

Ticket: [mesh_ops fixture limits](../../issues/29-mesh-ops-fixture-limits.md).
Status: **resolved** — fixture strategy chosen and recorded, both leaves
replaced, guards added, fresh baselines captured.

## What was wrong (pre-ticket, recorded by t15)

[Criterion bench refresh](../../issues/15-criterion-bench-refresh.md) found the
two `mesh_ops` leaves measuring real work a fixture could not let succeed:

1. **`repair/cube` timed a no-op scan.** The old bench fed `repair` a
   `cube.step` mesh imported *through the repairing importer* — 12 clean
   triangles. Measured at the pre-ticket shape: `degenerate_removed: 0,
   faces_reoriented: 0, open_edges_closed: 0` every iteration (t15's
   `fixture-probes/PROBES.md`; re-confirmed for this take, below).
2. **`decimate/cube_default` timed rejected work.** 12 → 12 against
   `target_ratio(0.5)` because the default `max_error = 0.01` is far below the
   >0.4 this box needs to collapse (t15's sweep: 0.01–0.4 stay 12; 0.5/0.6
   give 6).

Both numbers were reproducible work on a fixture that cannot exhibit a
successful operation — bench-trustworthiness, not a slice-path cost.

## Fixture strategy (the choice the ticket asked to record)

| Leaf | Fixture | Why |
| --- | --- | --- |
| `mesh_ops/repair/wedge` | `resources/regression_wedge.stl` (tracked, 80 tri, 5 components) | Real damage: repaired at HEAD this take as `faces_reoriented: 12, open_edges_closed: 56` — the same shape t15's probe measured. |
| `mesh_ops/decimate/calicat_default` | `resources/calicat.stl` (tracked, 876 tri) | Dense enough that the **default** error budget reaches `target_ratio(0.5)`: 876 → 438, `target_reached: true`. `regression_wedge` cannot reach the target (80 → 80), so it was not reused for decimate. |

`cube.step` is untouched: it is shared by `import_step_tdd.rs`,
`step_fixtures/mod.rs`, and the `pnp-cli` `helpers_cli` tests. No fixture any
other test depends on was swapped or modified. The bench loads its fixtures
through `slicer_model_io::load_model` — the same production STL path
`pnp_cli mesh repair|decimate` uses — replacing the STEP-only importer load.

## Falsifiability (guards added)

- In-bench guards (`crates/slicer-helpers/benches/mesh_ops.rs`): each leaf
  probes its fixture before timing and panics if repair touches nothing or
  decimate stops above the declared target.
- Companion test file
  `crates/slicer-helpers/tests/mesh_ops_bench_fixtures_tdd.rs` repeats both
  predicates in the test suite, plus a **negative control**
  (`pre_ticket_cube_fixture_would_fail_both_guards`) that runs the same
  predicates against the pre-ticket `cube.step` load path and asserts both
  fail. The control proves the predicates are falsifiable by the historical
  defect, not vacuously true.

## Fresh baselines

Run `cargo bench -p slicer-helpers --bench mesh_ops` at HEAD `3d7dc946`
(2026-10-07), reduced from criterion's own `target/criterion/**/new/*.json` by
[`reduce-baselines.py`](reduce-baselines.py) into
[`baselines.json`](baselines.json). `base == new` for every leaf — this is the
first run, so these are the baselines going forward.

| Leaf | Mean | Median | 95% CI (mean) | Outliers (recomputed) |
| --- | ---: | ---: | ---: | ---: |
| `mesh_ops/repair/wedge` | 36.296 µs | 32.892 µs | 35.154 – 37.504 µs | 0 |
| `mesh_ops/decimate/calicat_default` | 185.572 µs | 178.290 µs | 181.912 – 189.655 µs | 7 (6 mild / 1 severe) |
| `mesh_ops/import_step/cube` | 1.4108 ms | 1.3570 ms | 1.3834 – 1.4398 ms | 5 mild |
| `mesh_ops/import_step/assembly` | 2.4065 ms | 2.3872 ms | 2.3896 – 2.4244 ms | 5 mild |

`import_step` leaves are unchanged (same fixtures) and their numbers are
within the t15 vintage's spread — recorded for completeness, not re-baselined
in meaning.

**Pre-ticket leaves are superseded and not comparable.** `mesh_ops/repair/cube`
(t15: 2.947 µs mean) and `mesh_ops/decimate/cube_default` (t15: 3.745 µs mean)
measured no-op/rejected work on a different fixture; their criterion directories
were replaced by the new leaf names. The bench doc comment and `baselines.json`
(`pre_ticket_leaves_superseded`) both carry this note.

## Negative control at HEAD (pre-ticket shape re-derived)

Run with the same-day `target/debug/pnp_cli.exe`, i.e. the production CLI path:

```
$ pnp_cli mesh repair --input <cube.stl from cube.step>  --stats
{"components":1,"degenerate_removed":0,"faces_reoriented":0,"open_edges_closed":0,...,"warnings":[]}      # exit 0 — no-op scan
$ pnp_cli mesh decimate --input <same> --target-ratio 0.5 --stats
{"achieved_error":0.0,"final_triangle_count":12,"original_triangle_count":12,"target_reached":false}        # exit 1 — rejected
$ pnp_cli mesh repair --input resources/regression_wedge.stl --stats
{"components":5,"degenerate_removed":0,"faces_reoriented":12,"open_edges_closed":56,"warnings":["multiple_components"]}  # exit 1 — real repair
$ pnp_cli mesh decimate --input resources/calicat.stl --target-ratio 0.5 --stats
{"achieved_error":0.0001603...,"final_triangle_count":438,"original_triangle_count":876,"target_reached":true}           # exit 0 — reached
```

Exit codes 1 on the first two are the pre-ticket defect's signature (warnings
for a no-op repair; target-not-reached for decimate); `regression_wedge`
repair's exit 1 is its documented multiple-components warning, a successful
repair.

## Validation and reproduction

- `cargo test -p slicer-helpers` — all suites green, including the 3-test
  fixture-guard file (log: `target/test-output.log`).
- `cargo clippy --workspace --all-targets -- -D warnings` — clean;
  `cargo fmt --check -p slicer-helpers` — clean.
- `cargo xtask check-literals` — 0 violations; touched-file
  `check-test-quality --report` — 0 findings.
- `cargo build --workspace` — clean.
- Re-derive baselines after re-running the bench:
  `python docs/specs/perf-vs-orca/evidence/t29-mesh-ops-fixtures/reduce-baselines.py`
- Verify the reduced set from tracked files alone:
  `python docs/specs/perf-vs-orca/evidence/t29-mesh-ops-fixtures/verify-t29.py`

Limits: criterion's persisted `tukey.json` carries fences, not bucket counts;
outlier buckets are recomputed from raw times (mirroring t15's discipline) and
match the console's printed blocks. The benchmark is a helper microbench
(repair/decimate/import have no production caller on the slice path), so this
take moves no map cell; it repairs bench trustworthiness only. No timing
campaign, adoption, commit or route claim is part of this ticket.
