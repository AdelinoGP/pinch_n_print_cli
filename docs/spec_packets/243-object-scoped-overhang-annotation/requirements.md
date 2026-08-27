# Requirements: 243-object-scoped-overhang-annotation

## Packet Metadata

- Grouped task IDs: `TASK-353`
- Backlog source: `docs/07_implementation_status.md`
- Packet status: `draft`
- Aggregate context cost: `M`

## Problem Statement

`SurfaceClassificationIR` carries two host-only overhang annotation maps —
`overhang_quartile_polygons` and `prev_layer_boundaries` — both keyed by *global layer index only*.
`commit_overhang_annotation_builtin`
(`crates/slicer-runtime/src/builtins/overhang_annotation_producer.rs`) merges every object's
per-layer results into one flat map per layer, and the marshal
(`crates/slicer-wasm-host/src/marshal/in_.rs`, `sliced_region_to_data`) hands each region whichever
polygons share its layer index regardless of object identity. In a multi-object scene where two
objects overlap in XY, one object's previous-layer boundary and quartile bands leak into the other
object's `SliceRegionView`, so `classic-perimeters` / `arachne-perimeters` measure
`overhang_distance_mm` against a foreign object's boundary and the quartile gate is supplied by an
overlapping sibling. The host's own bridge gate is already object-scoped
(`crates/slicer-runtime/src/slice_postprocess_prepass.rs` builds `lower_layer_polygons` keyed
`(ObjectId, u32)` before `gate_bridge_areas_by_unsupported_span`), so the prepass computes the right
data and discards the object dimension only when it writes the flat view maps. This packet restores
the object dimension in the two maps.

## In Scope

- Replace `SurfaceClassificationIR.overhang_quartile_polygons: HashMap<u32, Vec<QuartileBand>>` with
  `HashMap<ObjectId, HashMap<u32, Vec<QuartileBand>>>` (`ObjectId = String`,
  `crates/slicer-ir/src/slice_ir.rs`).
- Replace `SurfaceClassificationIR.prev_layer_boundaries: HashMap<u32, Vec<ExPolygon>>` with
  `HashMap<ObjectId, HashMap<u32, Vec<ExPolygon>>>`.
- Bump `CURRENT_SURFACE_CLASSIFICATION_SCHEMA_VERSION` from `1.3.0` to `2.0.0` (major — field-type
  change per the IR Versioning Contract table in `docs/02_ir_schemas.md`).
- Re-key `commit_overhang_annotation_builtin` to build per-object maps (insert `per_object` /
  `per_object_boundaries` under `object.id` instead of merging into flat layer-keyed maps).
- Re-key the marshal lookups in `sliced_region_to_data` to
  `sc.overhang_quartile_polygons.get(&region.object_id).and_then(|m| m.get(&global_layer_index))`
  and the analogous `prev_layer_boundaries` lookup.
- Re-key the two `visual_debug_render.rs` consumers (`sc.overhang_quartile_polygons.values()` and
  `.get(&layer_index)`) to iterate the nested map.
- Update the mechanical test fallout (see `design.md` §Files in Scope for the full list) and rewrite
  the multi-object producer test to assert object isolation instead of quartile merge.
- Add a marshal-side contract test proving no cross-object leak.
- Update `docs/02_ir_schemas.md` §"IR 2 — SurfaceClassificationIR" (schema version, keying prose,
  packet-193 provenance note).

## Out of Scope

- Any WIT change: `SliceRegionView` accessor signatures are unchanged; the object dimension never
  crosses the WIT boundary.
- Any change to `classic-perimeters` / `arachne-perimeters` / `slicer-sdk` / `slicer-macros` — they
  consume the unchanged view accessor.
- Any change to `slice_postprocess_prepass.rs` or `gate_bridge_areas_by_unsupported_span` — the
  prepass is already object-scoped and is the reference shape, not a change target.
- The `Point3WithWidth.overhang_distance_mm` perimeter IR (packet 193's other surface) — untouched.
- Wave bridge fill, order locks, or any Packet 2/3/4 scope.

## Authoritative Docs

- `docs/02_ir_schemas.md` - ~1650 lines; direct range reads only: §"IR 2 — SurfaceClassificationIR"
  (lines ~387-400) and §"IR Versioning Contract" (lines ~1633-1641). Delegate any other section.
- `docs/specs/wave-overhangs-bridge-fill-plan.md` - normative plan; §"Packet 1 — Object-scoped
  overhang annotation" is the governing brief.

## Acceptance Summary

Reference, never copy, criteria from `packet.spec.md`.

- Positive: `AC-1` (schema major bump), `AC-2` (producer object-scoping), `AC-3` (marshal
  object-scoping).
- Negative: `AC-N1` (no cross-object leak in the marshal view).
- Cross-packet impact: Packet 244 consumes the object-scoped `prev_layer_boundaries` shape as the
  `prev_object_boundary` source for wave bridge fill (`supported_fill`); Packet 246 reads it via the
  object-scoped map. No other packet reads these maps directly.

## Verification Commands

This is the authoritative full matrix; `packet.spec.md` lists only the gate commands.

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo test -p slicer-ir --test ir_tests -- bridge_detector_schema_versions_are_constant_sourced --exact 2>&1 \| tee target/test-output.log \| grep -qE "^test result: ok\. 1 passed"` | schema constant is 2.0.0 and default-sourced | FACT pass/fail |
| `cargo test -p slicer-runtime --test executor -- overhang_annotation_scopes_bands_by_object --exact 2>&1 \| tee target/test-output.log \| grep -qE "^test result: ok\. 1 passed"` | producer emits object-keyed maps | FACT pass/fail |
| `cargo test -p slicer-runtime --test contract -- overhang_areas_object_scoped_no_cross_object_leak --exact 2>&1 \| tee target/test-output.log \| grep -qE "^test result: ok\. 1 passed"` | marshal reads object-scoped, no leak | FACT pass/fail |
| `cargo check --workspace --all-targets` | every affected crate + test target compiles | FACT pass/fail |
| `cargo clippy --workspace --all-targets -- -D warnings` | lint gate | FACT pass/fail |

## Step Completion Expectations

- Step 1 (field-type replacement + major bump) must land the constant bump and the
  `ir_tests.rs` hard-assert in the same edit — the tree is non-compiling between them.
- Step 2 (producer + marshal + visual-debug re-keying) must land before any test run; the
  field-type change breaks all three consumers at once.
- Step 3 (test fallout + new contract test) is the only step that may add net-new test functions.

## Context Discipline Notes

- `docs/02_ir_schemas.md` is over 300 lines — read only the two named ranges, delegate the rest.
- The blast radius is larger than the plan's "three fixture files" claim (see `design.md` §Files in
  Scope); do not let a follow-up `cargo check` discover the extra consumers — they are pre-baked.
