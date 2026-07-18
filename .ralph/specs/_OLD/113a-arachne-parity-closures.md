---
status: implemented
packet: 113a-arachne-parity-closures
task_ids: []
---

# 113a-arachne-parity-closures

## Goal

Close 4 Arachne-pipeline deviations and 2 audit findings by implementing the 6 independent S/M items identified in the packet 112 audit: Visvalingam-Whyatt simplification, 7 unwired Arachne config keys, MMU test fix (two-level approach), loader source-guard tightening, `cube_4color_arachne/` fixture directory, and closure-log file-count correction.

## Problem Statement

The packet 112 audit (commit `d9466fd7`) identified 4 residual `D-112-*` deviations and 4 audit findings that block true OrcaSlicer parity. The implementation produced a from-first-principles adaptation (Douglas-Peucker, depth-floor centrality, folded transition marking, per-EDGE bead count, unwired `arachne_params_from_config` keys, weakened MMU test assertion, no MMU `cube_4color_arachne/` fixture directory, closure-log with no commit diff stat) rather than a literal port of OrcaSlicer's algorithms or a clean manifest↔config contract. The adaptations are functionally equivalent for the tested fixtures but are not algorithm-faithful — a future reviewer comparing the code to OrcaSlicer's source would see divergence. This packet implements 6 of the 8 total residual items: the 6 that do NOT depend on the synthetic quad/rib topology pass (which is L-effort and deferred to P113b). These 6 items are genuine parity closures: they replace the adaptation with OrcaSlicer's algorithm (DP → VW), wire the 4 config keys that P111 registered in the manifest but P112 left unread in `arachne_params_from_config` (plus add 3 net-new keys for parameters that have no manifest entry at all), fix the MMU executor test to be honest about what it asserts, add a `paint_segmentation` unit test for the geometric partition invariant (necessary but not sufficient to close `D-112-MMU-TOPOLOGY` — that deviation remains open and re-targets to P113b's quad/rib pass), tighten the loader source-guard to pin the exact renamed entry point, create the missing `cube_4color_arachne/` fixture directory + golden, and add the commit diff stat to the closure-log. The remaining 2 items (quad/rib topology pass, faithful centrality/bead_count/transitions/connectJunctions) belong to P113b.

## Architecture Constraints

<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and, if `STALE:` is reported, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.

<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.

- **Atomic rename constraint:** The `dp_epsilon` → `visvalingam_area_threshold` rename is a WIT breaking change to the `arachne-params` record in `common.wit`. The host-service bridge in `slicer-sdk/src/host.rs` and the WIT host impl in `slicer-wasm-host/src/host.rs` must be updated in the same commit. No external consumers exist (the bridge is WASM-internal), so the break is contained.

- **Visvalingam no-op on current input:** `simplify_toolpaths` will hit the `n <= 2` early return on the current 2-junction fragments from P112's `generate_toolpaths`. The Visvalingam port ships the faithful algorithm code-ready; it becomes active when P113b's faithful `connectJunctions` produces multi-junction input. The simplify fixture re-recording is deferred to P113b.

## Data and Contract Notes

- **WIT contract change:** The `arachne-params` record in `common.wit:26-39` renames `dp-epsilon: f32` to `visvalingam-area-threshold: f32`. This is a WIT breaking change to the host-service interface, but the interface is unreleased (WASMinternal). The semantic also changes: `dp-epsilon` was a perpendicular-distance threshold in mm; `visvalingam-area-threshold` is a width-weighted area threshold in mm².
- **ArachneParams field additions:** `wall_transition_length: f64`, `wall_transition_angle: f64`, `initial_layer_min_bead_width: f64`, `outer_wall_offset: f64` — all in mm except `wall_transition_angle` which is in degrees (converted to radians at the factory boundary).
- **BeadingFactoryParams field additions:** `wall_transition_angle: f64` (radians) + `initial_layer_min_bead_width: f64` (mm). The other 2 keys (`wall_transition_length`, `outer_wall_offset`) already have factory fields at lines 75 and 92; they are just not threaded from `ArachneParams`.
- **MMU unit test invariant:** Per-color `ExPolygon` sets from `slicer_core::algos::paint_segmentation` must be (a) non-overlapping (intersection empty within `SCALED_EPSILON` ≈ 1 unit tolerance), (b) contained in the model XY bounding box, (c) non-zero area, (d) the union of all per-color cells must cover the full painted face area.
- **No schema bump:** No IR types change; no `CURRENT_SLICE_IR_SCHEMA_VERSION` bump needed.

## Locked Assumptions and Invariants

- The `simplify_toolpaths` public API signature `(lines: Vec<ExtrusionLine>, threshold: f64) -> Vec<ExtrusionLine>` is preserved (the parameter is renamed from `dp_epsilon` to `visvalingam_area_threshold`, but the position/type are unchanged; the semantic changes from distance-mm to area-mm²).
- `ArachneParams::default()` provides sensible defaults for all 4 newly-wired keys + the 3 net-new manifest keys (matching the factory's existing `BeadingFactoryParams::default()` values).
- The Visvalingam port is a no-op on P112's 2-junction input. The `simplify_toolpaths_vertex_count` test fixture continues to pass with the same vertex count (since VW is a no-op on 2-junction input). When P113b's faithful `connectJunctions` produces multi-junction input, the Visvalingam port will actually exercise vertex removal; that fixture re-baseline is P113b's scope.
- The MMU unit test reads `resources/cube_4color.3mf` (already present in the repo from P105). The test must not modify the 3MF.
- The `cube_4color_arachne/` fixture directory's `expected_perimeter_ir.json` is captured by running the arachne wall output on `cube_4color.3mf` once and committing the result. This is a one-time self-captured golden per `D-112-SELFCAPTURED-BASELINES`, not regenerated.

## Risks and Tradeoffs

- **WIT breaking change:** The `dp-epsilon` → `visvalingam-area-threshold` rename breaks the `arachne-params` record. Mitigation: the host-service interface is unreleased (WASM-internal); no external consumers to update. The change is atomic across `common.wit`, `slicer-sdk/src/host.rs`, `slicer-wasm-host/src/host.rs`, and `pipeline.rs::ArachneParams`.
- **Visvalingam no-op on P112 input:** The simplify fixture won't show vertex-count reduction until P113b's `connectJunctions` ships. Mitigation: the algorithm port is correct (verified by code review against OrcaSlicer's `calculateExtrusionAreaDeviationError`); the fixture re-baselines in P113b.
- **MMU unit test does not close `D-112-MMU-TOPOLOGY`:** The AC-4 unit test establishes a geometric partition invariant UPSTREAM of the arachne output. The actual "tens of mm outside the naive per-face footprint" symptom is governed by `arachne-perimeters` output topology (the per-edge 2-junction fragment emission pattern from `generate_toolpaths.rs`). P113b's quad/rib pass + faithful `connectJunctions` changes that emission pattern, so the deviation re-targets to P113b with a different hypothesis. The unit test added here is supporting evidence for the deviation, not a closure.
- **MMU unit test requires `resources/cube_4color.3mf` to be parseable by `slicer_core::algos::paint_segmentation`:** The test feeds the 3MF to `slicer_core::algos::paint_segmentation`. If the 3MF format or the `paint_segmentation` API has changed since P105, the test needs updating. Mitigation: P105's `cube_4color.3mf` is the reference fixture; the test uses the same loading path. The other `cube_4color_*` executor tests in the repo all read the same fixture successfully.
- **`visvalingam_area_threshold` default value:** The current DP default (`dp_epsilon: 0.025` mm) is a distance. The equivalent VW default is an area. There's no direct conversion — the implementer must choose a reasonable area threshold (likely `0.025 * 0.4 = 0.01` mm² based on the typical bead width). Mitigation: the default is a config-time parameter; users can tune it. The test fixture passes with whatever default the implementer chooses (the existing `simplify_toolpaths_vertex_count` test is a no-op on 2-junction input either way).
- **`D-112-MMU-TOPOLOGY` stays open across this packet:** the original 113a draft claimed the deviation would close. It does not. The deviation's "follow-up" column should be updated to point to P113b's quad/rib pass + `connectJunctions` as the target, not to AC-4's unit test.
