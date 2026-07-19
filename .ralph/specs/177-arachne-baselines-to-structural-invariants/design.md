# Design: 177-arachne-baselines-to-structural-invariants

## Domain Model

- **Self-captured baseline:** serialized PnP output from an earlier run. It is
  regression history, not correctness evidence, and this packet removes it
  from the active Arachne test pipeline.
- **Coverage subject:** source geometry that can run through classic and
  Arachne at the same aligned Z planes. Only coverage subjects contribute to
  `observed_min`.
- **Structural invariant:** a ratio, count, topology, tolerance, or
  spacing-domain cap. It never compares a captured absolute coordinate.
- **Repeatability margin:** the maximum same-subject/same-Z repeated-run delta,
  capped at `0.02`. It absorbs measurement instability, not fixture spread.
- **Synthetic reproduction fixture:** a deliberately constructed input or
  ratio that represents a known benchy error class. D5 is synthetic evidence,
  not a threshold subject.

## Controlling Code Paths

- `ArachneParams::default()` in
  `crates/slicer-core/src/arachne/pipeline.rs` and
  `BeadingFactoryParams::default()` in
  `crates/slicer-core/src/beading/factory.rs` — the two production defaults
  corrected to the canonical even literal `10`.
- `run_arachne_pipeline` in
  `crates/slicer-core/src/arachne/pipeline.rs` — core-only source-geometry
  path used by in-memory structural tests.
- `dedup_same_claim_modules_with_wall_generator` and
  `wall_generator_preferred_module_id` in
  `crates/slicer-scheduler/src/execution_plan.rs` — existing runtime selection
  of classic versus Arachne.
- `run_pipeline_capturing_perimeters` — extracted into
  `crates/slicer-runtime/tests/common/perimeter_harness.rs`; runs the real
  runtime pipeline and returns per-layer `PerimeterIR`.
- `crates/slicer-runtime/tests/arachne_structural_invariants.rs` — standalone
  runtime test binary for paired coverage, D5 discrimination, and the
  tapered-wedge structural comparison.
- `crates/slicer-core/tests/arachne_invariants.rs` — core-only home for graph,
  default, and spacing-domain bead invariants; it does not own classic
  coverage because classic has no standalone slicer-core generator.

## Architecture Constraints

- Coordinate units remain `1 unit = 100 nm`; use `Point2::from_mm` or
  `mm_to_units` at every boundary.
- Classic and Arachne measurements compare the same source geometry at the
  same `global_layer_index`/Z plane. A ratio across misaligned planes is
  invalid.
- Arachne bead widths are flow-spacing values. Name the cap
  `2 * optimal_spacing_mm`; do not compare raw extrusion widths to it. The D4
  `19.6 mm` value is a historical failure observation.
- `WallToolPaths.cpp::generate` is the authority for an always-even
  `2 * inset_count` maximum. `LimitedBeadingStrategy.cpp::compute` does not
  have the claimed odd-count giant-center branch; do not cite one.
- Absolute-coordinate equality against a captured snapshot is forbidden.
- The threshold is a floor. If repeatability is greater than `0.02`, or if the
  derived floor admits `0.668`, stop rather than tune.

## Selected Approach

1. Correct the two production defaults and every odd test helper to `10`.
2. Replace JSON consumers with source-geometry structural cases and delete all
   nineteen JSON oracle files: eight core snapshots and eleven perimeter
   expected-IR snapshots.
3. Extract the runtime perimeter capture harness and add a standalone paired
   coverage binary.
4. Measure the five Arachne source fixtures at aligned Z planes. Repeat each
   subject at the same plane; set `margin = max(repeat_delta)`, reject a delta
   above `0.02`, and record `threshold = observed_min - margin`.
5. Keep D5's `0.668` and `0.990` as synthetic discrimination values only.
6. Rehome the nine red tests and correct the stale runtime header.
7. Update the recovery doc, ADR-0042, deviation row, and glossary.

## Coverage Subjects

| Subject | Source input | Paired run | Threshold role |
| --- | --- | --- | --- |
| `tapered_wedge` | `crates/slicer-runtime/tests/fixtures/perimeter_parity/tapered_wedge/tapered_wedge.stl` | classic + Arachne | measured |
| `narrow_strip_widening` | `crates/slicer-runtime/tests/fixtures/perimeter_parity/narrow_strip_widening/narrow_strip_widening.stl` | classic + Arachne | measured |
| `max_bead_count_cap` | `crates/slicer-runtime/tests/fixtures/perimeter_parity/max_bead_count_cap/max_bead_count_cap.stl` | classic + Arachne | measured |
| `complex_multi_feature` | `crates/slicer-runtime/tests/fixtures/perimeter_parity/complex_multi_feature/complex_multi_feature.stl` | classic + Arachne | measured |
| `cube_4color_arachne` | `crates/slicer-runtime/tests/fixtures/perimeter_parity/cube_4color_arachne/cube_4color_arachne.stl` | classic + Arachne | measured |
| `d5_benchy_call1` | `crates/slicer-core/tests/fixtures/arachne/d5_benchy_call1.txt` | synthetic ratio only | sanity-only; excluded from minimum |

The five measured subjects use the same mesh and layer configuration for both
runs, changing only the `wall_generator` selection. The runtime harness joins
outputs by global layer/Z and reports X extents and coverage percentages.

## Measured Coverage Baseline

Step 3 fills this table from the runtime harness. No threshold may be invented
before the table is populated.

| Fixture | Arachne X extent (mm) | Classic X extent (mm) | Coverage ratio | Z plane (mm) | Repeat delta | Notes |
| --- | ---: | ---: | ---: | ---: | ---: | --- |
| `tapered_wedge` | | | | | | measured |
| `narrow_strip_widening` | | | | | | measured |
| `max_bead_count_cap` | | | | | | measured |
| `complex_multi_feature` | | | | | | measured |
| `cube_4color_arachne` | | | | | | measured |
| `d5_benchy_call1` | n/a | n/a | `0.668` broken / `0.990` fixed | n/a | n/a | sanity-only; excluded from observed minimum |

- Observed minimum: _(fill during Step 3)_
- Repeatability margin: _(maximum same-subject repeat delta; hard cap `0.02`)_
- **Chosen threshold = observed_min - margin:** _(fill during Step 3)_
- D5 sanity values: broken `0.668`, fixed `0.990`; excluded from the minimum.
- Margin justification: describe the repeated same-Z deltas and why they are
  measurement noise. Fixture spread is not a justification.

## In-Memory Structural Cases

The old JSON fixture labels remain scenario names only; no serialized output is
loaded or written.

- Centrality cases generate square, wedge, and multi-feature polygons and
  assert central flags agree with edge topology and the edge-count bound.
- Propagation cases generate uniform, varying, and multi-feature graphs and
  assert every bead-count change has a transition marker; the old
  `propagation_fills_gap_from_central_neighbor` test must not assert the D5
  defect.
- Bead-count cases assert monotonic sequences between transition bounds.
- Toolpath cases assert spacing-domain width caps and valid junction topology.
- Perimeter-parity cases retain source-fixture structural smoke checks: nonempty
  captures, finite coordinates, walls with at least two points, expected loop
  topology, width clamping, bead caps, multi-loop shape, and tool-index counts.

## Exact Code and Test Surface

### Existing files edited

- `crates/slicer-core/src/arachne/pipeline.rs` — default literal/comment.
- `crates/slicer-core/src/beading/factory.rs` — default literal/comment.
- `crates/slicer-core/tests/arachne_invariants.rs` — defaults, spacing-domain
  cap, synthetic D5 predicate tests.
- `crates/slicer-core/tests/centrality.rs` — in-memory centrality assertions.
- `crates/slicer-core/tests/propagation.rs` — in-memory propagation assertions
  and even helper.
- `crates/slicer-core/tests/bead_count.rs` — in-memory bead-count assertions.
- `crates/slicer-core/tests/generate_toolpaths.rs` — in-memory toolpath
  assertions.
- `crates/slicer-runtime/tests/integration/perimeter_parity.rs` — remove all
  expected-IR loaders, snapshot comparisons, and recorder functions; retain
  source-fixture structural smoke assertions.
- `crates/slicer-runtime/tests/common/mod.rs` — register the shared harness.
- `crates/slicer-runtime/tests/arachne_parity.rs` — header only.
- `docs/specs/arachne-parity-recovery.md` — Track B stale-state correction.
- `docs/adr/0042-arachne-parity-structural-invariants-over-fixtures.md` —
  measured threshold consequence.
- `docs/DEVIATION_LOG.md` — D-112 closes only at the final gate.
- `CONTEXT.md` — glossary terms.

### New files

- `crates/slicer-runtime/tests/common/perimeter_harness.rs` — shared public
  capture, layer alignment, X-extent, and coverage helpers.
- `crates/slicer-runtime/tests/arachne_structural_invariants.rs` — standalone
  test binary; Cargo auto-discovers top-level `tests/*.rs` files, so no
  `Cargo.toml` test registration is needed.

### Deleted files

- `crates/slicer-core/tests/fixtures/arachne/centrality_square.json`
- `crates/slicer-core/tests/fixtures/arachne/centrality_wedge.json`
- `crates/slicer-core/tests/fixtures/arachne/centrality_multi_feature.json`
- `crates/slicer-core/tests/fixtures/arachne/propagation_uniform.json`
- `crates/slicer-core/tests/fixtures/arachne/propagation_varying.json`
- `crates/slicer-core/tests/fixtures/arachne/propagation_multi_feature.json`
- `crates/slicer-core/tests/fixtures/arachne/bead_count_tapered_wedge.json`
- `crates/slicer-core/tests/fixtures/arachne/toolpaths_tapered_wedge.json`
- `crates/slicer-runtime/tests/fixtures/perimeter_parity/tapered_wedge/expected_perimeter_ir.json`
- `crates/slicer-runtime/tests/fixtures/perimeter_parity/narrow_strip_widening/expected_perimeter_ir.json`
- `crates/slicer-runtime/tests/fixtures/perimeter_parity/max_bead_count_cap/expected_perimeter_ir.json`
- `crates/slicer-runtime/tests/fixtures/perimeter_parity/complex_multi_feature/expected_perimeter_ir.json`
- `crates/slicer-runtime/tests/fixtures/perimeter_parity/cube_4color_arachne/expected_perimeter_ir.json`
- `crates/slicer-runtime/tests/fixtures/perimeter_parity/solid_square/expected_perimeter_ir.json`
- `crates/slicer-runtime/tests/fixtures/perimeter_parity/holed_square/expected_perimeter_ir.json`
- `crates/slicer-runtime/tests/fixtures/perimeter_parity/bridge/expected_perimeter_ir.json`
- `crates/slicer-runtime/tests/fixtures/perimeter_parity/overhang_ramp/expected_perimeter_ir.json`
- `crates/slicer-runtime/tests/fixtures/perimeter_parity/multi_tool_triangle/expected_perimeter_ir.json`
- `crates/slicer-runtime/tests/fixtures/perimeter_parity/spiral_vase_cone/expected_perimeter_ir.json`

### Red-test moves

| Old path | New path |
| --- | --- |
| `arachne_parity_red_insert_node.rs` | `arachne_construction_insert_node_rib_split.rs` |
| `arachne_parity_red_apply_transitions.rs` | `arachne_construction_apply_transitions_mirror_fix.rs` |
| `arachne_parity_red_node_distance.rs` | `arachne_construction_node_distance_perp_foot.rs` |
| `arachne_parity_red_transition_ends.rs` | `arachne_construction_transition_ends_splits.rs` |
| `arachne_parity_red_chain_junctions.rs` | `arachne_stitch_chain_junctions_t_to_fix.rs` |
| `arachne_parity_red_perimeter_index.rs` | `arachne_stitch_perimeter_index_bead_index.rs` |
| `arachne_parity_red_is_odd_semantics.rs` | `arachne_beading_is_odd_semantics.rs` |
| `arachne_parity_red_junction_bands.rs` | `arachne_beading_junction_radius_bands.rs` |
| `arachne_parity_red_propagation_bug.rs` | `arachne_beading_propagation_delta_bound.rs` |

## Read-Only Context

- `crates/slicer-core/tests/arachne_d5_taper_coverage.rs` — D5 parser and
  synthetic ratio shape.
- `crates/slicer-runtime/tests/integration/perimeter_parity.rs` — bounded
  harness extraction and existing source fixtures; do not browse recorders.
- `crates/slicer-scheduler/src/execution_plan.rs` — wall-generator selection.
- `docs/adr/0042-arachne-parity-structural-invariants-over-fixtures.md` —
  structural classes and spacing cap.
- `docs/specs/arachne-parity-recovery.md` — Track B history and correction.
- `docs/DEVIATION_LOG.md` — D-112 and D-104f rows only.
- `docs/08_coordinate_system.md` — unit conversion checklist.

## Out-of-Bounds Files

- `target/`, `Cargo.lock`, generated code, vendored dependencies.
- OrcaSlicer source beyond delegated function checks.
- Other packet directories.
- Any recorder invocation against a deleted snapshot.
- The four non-Arachne perimeter fixtures not listed as coverage subjects.

## Risks and Tradeoffs

- The threshold is measured only over source inputs with a real paired path;
  this is narrower than the old JSON corpus but materially stronger evidence.
- A repeatability delta above `0.02` exposes nondeterminism instead of hiding it
  behind tolerance.
- Deleting snapshots sacrifices historical numeric diff visibility. That is
  intentional: they were self-captured correctness oracles, not independent
  evidence.
- The existing D-104f red test remains open and is not absorbed into this
  packet's structural claims.
