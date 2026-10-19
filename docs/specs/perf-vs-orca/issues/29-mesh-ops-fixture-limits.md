# mesh_ops fixture limits: repair/cube no-op scan and decimate/cube_default rejected target

Type: task
Status: open

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
