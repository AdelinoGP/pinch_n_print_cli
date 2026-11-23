# mesh_ops fixture limits: repair/cube no-op scan and decimate/cube_default rejected target

Type: task
Status: resolved (2026-10-07; fixture strategy recorded, both leaves replaced, fresh baselines captured — see ## Answer)
Assignee: agpen (current OpenCode wayfinder session), 2026-10-07 — human selected the small non-timing family; 29 first in number order

## Question

[Criterion bench refresh](15-criterion-bench-refresh.md) recorded two
`mesh_ops` fixture limits and deliberately fixed neither (fixing would have
invalidated that session's baseline comparability). Both leave a leaf
measuring real work that the fixture cannot let succeed:

1. **`repair/cube` scans a clean mesh.** `bench_repair`
   (`crates/slicer-helpers/benches/mesh_ops.rs`) loads
   `crates/slicer-helpers/tests/resources/cube.step` (10 mm cube, 12 triangles
   after import). `pnp_cli mesh repair --stats` on it reports
   `degenerate_removed: 0`, `faces_reoriented: 0`, `open_edges_closed: 0` — the
   bench measures a scan returning the mesh unchanged. The same harness on
   `resources/regression_wedge.stl` repairs real damage
   (`faces_reoriented: 12`, `open_edges_closed: 56`).
2. **`decimate/cube_default` cannot reach its declared target.** 12 → 12
   against `target_ratio(0.5)` in `bench_decimate`, `achieved_error: 0.0`,
   because `DecimateConfigBuilder::default`'s `max_error = 0.01`
   (`crates/slicer-helpers/src/decimate.rs`) is below what this box needs — a
   swept threshold: 0.01–0.4 stays 12; 0.5 and 0.6 give 6. The op itself is
   fine: the same call at the same default on a dense mesh
   (`resources/calicat.stl`, 876 tri) reaches 438/876. The 3.75 µs leaf is
   meshopt work the budget rejects.

Reproduction for both: `evidence/t15-criterion-refresh/fixture-probes/PROBES.md`.

## Why this is not urgent

No current decision needs `mesh_ops` repair/decimate numbers: `repair`,
`decimate`, and `import_step` (`crates/slicer-helpers/src/`) have no production
caller on the slice path — the only production callers are the
`pnp_cli mesh repair|decimate|import` subcommands
(`crates/pnp-cli/src/helpers_cmd.rs`). These leaves cannot taint the
matched-pair scoreboard or any wall claim. This is bench-trustworthiness work:
take it when a decision wants these numbers, or as parallel polish. It is not
on the timing chain.

## Work

- Choose the fixture strategy and record the choice (do **not** silently swap
  a fixture other tests depend on):
  - `cube.step` is shared: `crates/slicer-helpers/tests/import_step_tdd.rs`,
    `crates/slicer-helpers/tests/step_fixtures/mod.rs`, and
    `crates/pnp-cli/tests/helpers_cli.rs` also load it — any change to it must
    keep those green.
  - Either point the two benches at fixtures with real damage / denser
    geometry (`resources/regression_wedge.stl`; `resources/calicat.stl`), or
    add dedicated fixtures under
    `crates/slicer-helpers/tests/resources/`.
  - For `decimate`, choose a `max_error` that reaches the declared target on
    the chosen fixture (the bench's `config` is built inline in
    `bench_decimate`).
- Make a no-op fixture falsifiable: a companion test (or in-bench assertion)
  that fails when repair removes/reorients nothing, or decimate does not reach
  its target, on the bench fixture — so the leaf cannot silently regress into
  the pre-ticket shape.
- Re-run `mesh_ops`, record fresh baselines, and note in the bench that
  pre-ticket baselines are not comparable (fixture changed).

Deliverable: `repair` and `decimate` leaves that measure a successful
operation, with fresh baselines and a check that fails on a no-op fixture.

## Answer — resolved 2026-10-07

**Both leaves now measure a successful operation; the fixture choice is
recorded and the no-op shape is falsifiable.** Full evidence:
[`evidence/t29-mesh-ops-fixtures/FINDINGS.md`](../evidence/t29-mesh-ops-fixtures/FINDINGS.md);
machine-readable baselines in
[`baselines.json`](../evidence/t29-mesh-ops-fixtures/baselines.json) (reduced by
`reduce-baselines.py`, checkout-verified by `verify-t29.py`, whose sabotage
control exits 1).

**Strategy (recorded, not silent):**

- `mesh_ops/repair/cube` → `mesh_ops/repair/wedge` on
  `resources/regression_wedge.stl` (tracked): real damage
  (`faces_reoriented: 12`, `open_edges_closed: 56` at HEAD).
- `mesh_ops/decimate/cube_default` → `mesh_ops/decimate/calicat_default` on
  `resources/calicat.stl` (tracked): the **default** `max_error = 0.01`
  reaches `target_ratio(0.5)` (876 → 438); `regression_wedge` could not
  (80 → 80), so no `max_error` change was needed — the fixture density is the
  fix, keeping the declared target and default budget intact.
- `cube.step` is **untouched**; no fixture any other test depends on was
  swapped. The bench loads fixtures through `slicer_model_io::load_model` (the
  production `pnp_cli mesh repair|decimate` STL path) via a new dev-dependency
  on `slicer-model-io` (no cycle; `slicer-model-io` does not depend on
  `slicer-helpers`).

**Falsifiability:** in-bench guards in `benches/mesh_ops.rs` panic before
timing if repair touches nothing or decimate stops above target;
`tests/mesh_ops_bench_fixtures_tdd.rs` repeats both predicates plus a negative
control (`pre_ticket_cube_fixture_would_fail_both_guards`) that asserts the
pre-ticket `cube.step` load path fails both — proving the predicates catch the
historical defect rather than being vacuously true.

**Fresh baselines** (`cargo bench -p slicer-helpers --bench mesh_ops`,
2026-10-07, HEAD `3d7dc946`, `base == new` on all four leaves):
`repair/wedge` 36.296 µs mean / 32.892 µs median; `decimate/calicat_default`
185.572 µs / 178.290 µs; `import_step/cube` 1.4108 ms / 1.3570 ms;
`import_step/assembly` 2.4065 ms / 2.3872 ms. The pre-ticket leaves
(2.947 µs / 3.745 µs) are superseded and not comparable; the note lives in the
bench doc comment, `baselines.json`, and a pointer added to t15's
`BASELINES.md`/`FINDINGS.md`.

**Gates:** `cargo test -p slicer-helpers` all green (guard file 3/3);
`cargo build --workspace`; `cargo clippy --workspace --all-targets -- -D
warnings`; `cargo fmt --check`; `cargo xtask check-literals`; touched-file
`check-test-quality --report` — all clean. No timing campaign, adoption,
commit or route claim is part of this ticket; it moves no map cell.
