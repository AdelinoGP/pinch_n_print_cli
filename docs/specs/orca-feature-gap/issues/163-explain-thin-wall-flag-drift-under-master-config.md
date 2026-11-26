# 163 — Explain the `is_thin_wall` flag drift under master-era perimeter config

Type: task
Status: open
Assignee: —
Blocked by: —
Map: ../map.md

## Question

The perimeter-spatial self-baselines
(`crates/slicer-runtime/tests/integration/perimeter_spatial_capture.rs`, tests
`perimeter_spatial_self_baseline_{native_indexed,native_legacy,wasm}`) failed in
PR #11 CI. Master recorded them with `detect_thin_wall = true`. The branch's
user-ruled default is `false`, so layer 28 no longer emits its `ThinWall` loop.
The baselines were re-recorded for the new default in commit
`test(perimeter-spatial): re-record self-baselines for detect_thin_wall=false
default`, which is on `master` through the PR #11 merge.

While diagnosing that, one difference stayed unexplained. With master-era
config forced on (`detect_thin_wall = true` **and**
`small_perimeter_threshold = 0.8`), the branch's output still differs from
master's old baseline by **12 `is_thin_wall` point flags**. In that comparison
`classify_narrow_island` (`modules/core-modules/classic-perimeters/src/lib.rs`)
was byte-identical between master and the branch, so the drift comes from
somewhere else.

This does not affect shipped defaults (the thin-wall path is off by default).
It was not re-recorded away. Find the cause and rule on it.

## Acceptance

- Reproduce: record the perimeter-spatial fixture on current `master` with
  `detect_thin_wall = true` and `small_perimeter_threshold = 0.8`. Decode it
  against master's pre-merge baseline from
  `crates/slicer-runtime/tests/fixtures/perimeter_spatial/` (as it was before
  PR #11) and confirm the 12-flag delta. Re-derive the count; don't quote it.
- Find the commit or code path that changes those flags. Candidates are any
  config key renamed or re-defaulted on the branch that feeds classic-perimeters'
  thin-wall or narrow-loop decisions: renamed keys silently dropping to defaults,
  as `first_layer_height` → `initial_layer_print_height` did in the fixture's own
  `INPUT_CONFIG`, and per-point flag propagation downstream of
  `classify_narrow_island`.
- Rule on it: canonical-correct (cite the canonical OrcaSlicer function by
  name, never line number), or a regression (fix it, plus a regression test
  that pins the flags under that config).
- The perimeter-spatial suite must be run with its feature on:
  `cargo test -p slicer-runtime --features perimeter-spatial-test-support --test integration perimeter_spatial`.
  Without the feature the file compiles to one ignored stub and reports green.

## Notes

Surfaced 2026-11-26 during the PR #11 (`orca-feature-gaps` → `master`) CI fix.
