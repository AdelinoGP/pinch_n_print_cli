# ModularSlicer — Architecture Gate Metrics

This document defines objective thresholds for the Architecture Acceptance Gate.

## Determinism

- Repeat-run test count: 10 runs per reference fixture.
- Input/config must be byte-identical for all runs.
- PASS criteria:
  - `LayerCollectionIR` canonical hash identical across all runs.
  - Claim holder map identical for every `(layer, object, region, claim)`.

Canonical hash method (normative):

- Serialize `LayerCollectionIR` with deterministic field order.
- Normalize numeric values to canonical scaled-int form where applicable.
- Hash algorithm: `SHA-256` over serialized bytes.
- Exclude telemetry/runtime-only fields (`elapsed_ms`, timestamps, UUID run ids).

## Recoverability

- Failure-injection tests must cover at least:
  - one fatal module error,
  - one non-fatal module error,
  - one host contract validation failure.
- PASS criteria:
  - fatal path aborts immediately and emits fatal event,
  - non-fatal path completes with `degraded=true`.

## Resource Bounds

- Memory budget: peak RSS <= 512 MB on 500-layer reference fixture.
- Time budget: full slice <= 10 seconds on 50-layer benchy reference fixture.
- Layer budget: host rejects plans with `GlobalLayer.index >= 100_000`.

## Reference Fixture Set (Normative)

- `benchy_50l_0p2_single_tool`
- `multi_object_dual_material_sync_200l`
- `paint_overlap_material_fuzzy_support_120l`
- `high_region_count_modifier_stress_500l`
- `support_enforcer_blocker_conflict_80l`

<!-- VERIFY: at the time of writing, none of these named fixtures are
     materialised under `resources/`. The only Benchy-family fixtures present
     are `benchy.stl`, `cube_4color.3mf`, and `cube_cilindrical_modifier.3mf`. The
     named-fixture set above is the gate-evidence target; the fixtures (with
     deterministic config snapshots) must be added before a gate run can
     produce evidence matching these IDs. -->

Fixture governance:

- Any fixture change requires changelog entry and baseline refresh.
- Gate reports must list fixture IDs and git revision of fixture definitions.
- **Size budget (Packet 89):** any newly-authored test fixture must be ≤ 100 KB on disk. Fixtures exceeding this budget should be regenerated with a coarser mesh, reduced feature count, or replaced by an existing smaller asset. Rationale: minimise repository bloat while maintaining sufficient feature coverage. The benchy_4color (3.1 MB) and benchy_fuzzyPainted (2.1 MB) fixtures retired in Packet 89 violated this in the legacy pre-Phase 1 era; they were replaced by the 37 KB / 27 KB cube fixtures (see catalog below).

### Fixture Catalog (Packet 89)

Engineered cube fixtures introduced by cherry-pick `5c272ef…` for paint-segmentation parity validation. Per-face semantics are locked by test fixtures; reordering them breaks the P0a–P4 packet chain.

- **`resources/cube_4color.3mf`** (37 KB, MaterialMMU semantics):
  - `+X` face — `Material(ToolIndex(1))` (green).
  - `-X` face — `Material(ToolIndex(0))` (orange) with hex-subdivided mixed regions.
  - `+Y` face — `Material(ToolIndex(2))` (blue).
  - `-Y` face — Mixed, banded by height (multi-ToolIndex per Z-band).
  - `±Z` faces — Mixed or unpainted (varies by test).
- **`resources/cube_fuzzyPainted.3mf`** (27 KB, fuzzy-skin semantics):
  - `+X`, `+Y`, `+Z` faces — `FuzzySkin(Flag(true))`.
  - `-X`, `-Y`, `-Z` faces — unpainted (default fuzzy-skin behaviour applies, which is OFF).

Cherry-pick `5c272ef970fee2b861081799169a3ddb87e179c9` lands both fixtures plus 24 RED tests in `crates/slicer-runtime/tests/executor/cube_4color_paint_tdd.rs` and `cube_fuzzy_painted_tdd.rs`. These RED tests are the paint-pipeline parity validation gates that P0a (P89) through P4 (P95+) consume in sequence.

### Test Performance Regression Gate (Normative — Packet 90)

Wall-clock regression on a test fixture is acceptable when structurally justified. The gate is:

| Outcome | Action |
|---------|--------|
| Improvement | No action. Record the new baseline. |
| Regression with documented structural cause | Accept. Record the cause in the packet closure-log; commit profile traces / cache-key analysis / per-invocation cost breakdown alongside the new baseline. |
| Regression with no documented cause | Block packet close until cause is identified. Test infrastructure overhead, cache thrashing, or silently weakened assertions are NOT acceptable causes. |

The investigation record MUST include enough evidence to reproduce the regression analysis: profile traces (e.g. `cargo flamegraph`), `cargo nextest --status-level` per-test wall-clock breakdown, and a one-paragraph rationale tying the regression to a specific code-path change (e.g. Packet 90's wedge fixture forces real tree-support generation where the prior benchy fixture short-circuited through a bridge pass-through; +120 s is structurally explained).

### Fixture Feature Inventory Verification (Normative — Packet 90)

Every newly-authored fixture must carry a measured-bounds inventory verified at commitment. The inventory is a `KEY=VALUE` block (one key per line) capturing the geometric features the fixture exercises. Standard keys:

```
bounding_box_x_mm = <f32>
bounding_box_y_mm = <f32>
bounding_box_z_mm = <f32>
triangle_count = <u32>
max_overhang_angle_deg = <f32>    # from horizontal
largest_flat_top_area_mm2 = <f32>
flat_bottom_area_mm2 = <f32>
bridge_gap_width_mm = <f32>       # total overhang-class facet extent (Packet 90 redefinition)
ALL_FEATURES_OK = true|false
```

The block lives in the packet closure-log (under §Feature Inventory) and is regenerated by a dispatched binary-STL parser at fixture-commit time. Packet 90's `regression_wedge.stl` shipped with `bounding_box_height_mm=40.0`, `bounding_box_x_mm=50.4`, `max_overhang_angle_deg=51.3402`, `largest_flat_top_area_mm2=2500`, `flat_bottom_area_mm2=1250`, `bridge_gap_width_mm=50`, `ALL_FEATURES_OK=true`. Future fixture packets use the same block format and the same verification ceremony.

## Coupling Control

- Zero undeclared IR access violations in validation output.
- Zero unresolved write conflicts.
- Zero ambiguous claim holders.

## Compatibility

- Startup compatibility checks must pass for Host/WIT/IR/manifest matrix.
- At least one representative compatible module set and one incompatible set must be validated.

## Operability

- Progress events validate against schema v1.
- Required event set present for each run:
  - `phase_start` + `phase_complete` for all phases,
  - `layer_start` + `layer_complete` for every processed layer,
  - `slice_complete` exactly once.
