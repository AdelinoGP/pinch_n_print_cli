---
status: implemented
packet: finalization-mutation-enum-refactor
task_ids:
  - TASK-172
---

# 41_finalization-mutation-enum-refactor

## Goal

Close `DEV-041` by refactoring `FinalizationOutputBuilder`'s three mutation methods (`modify_entity`, `sort_layer_by`, `insert_synthetic_layer_after`) from closure-based APIs to **serializable-enum-based APIs** (`EntityMutation`, `SortKey`, `SyntheticLayerData`) so the WIT boundary can carry them losslessly. Wire the `slicer-macros` `run_finalization` drain-back loop to forward `merge_ops` through the WIT-bound `finalization-output-builder` resource. Add a WASM-side round-trip TDD test (a tiny new test guest under `test-guests/`) that proves a guest module's `modify_entity` call actually mutates the host-side IR. Remove the silent-no-op behavior described in `DEV-041`. Result: a WASM finalization module's mutation calls take effect end-to-end; the WIT surface for `modify-entity` / `sort-layer-by` / `insert-synthetic-layer-after` is no longer a contract lie.

## Problem Statement

Packet `40_finalization-mutation-builder` shipped a four-method WIT surface for finalization-stage mutation: `push_entity_with_priority`, `modify_entity`, `sort_layer_by`, `insert_synthetic_layer_after`. Three of those four were authored with **closure-typed** SDK signatures that cannot round-trip across the WIT boundary. The macro-generated `slicer-macros::run_finalization` drain-back loop forwards `priority_pushes` (the `push_entity_with_priority` records) but discards `merge_ops` (the closure-bearing recordings of the other three). The host-side `apply_to` then runs against an empty `merge_ops` Vec.

Result: a WASM finalization module that calls `output.modify_entity(layer, id, |e| e.path.speed_factor = 0.5)` records a closure in WASM-side memory; the macro drain-back skips it; the host applies nothing. **No error, no warning, no diagnostic** — the call vanishes. This is a contract lie: the WIT surface advertises `modify-entity` as a callable method, but invoking it from a guest module is silently a no-op.

`DEV-041` registered this gap as an "open" deviation with the rationale that no WASM module in packet 40 exercises the three closure-typed methods. That rationale has expired:

1. The macro-generated guest-side glue (Step 3b-fix in packet 40) now exposes the methods to every WASM finalization module that gets built. Future module authors will reach for them. The first one to use `modify_entity` will ship a silent regression — speed-modulation that doesn't take effect, layer-time enforcement that does nothing, flush-volume calculation that emits but never applies.
2. The four future PostPass modules listed in packet 40's design.md `## Open Questions` (`SequentialPrintOrder`, `MinLayerTimeEnforcer`, `FlushVolumeCalculator`, `PrimeTower`) are explicitly the consumers of these methods. Each was deferred under the assumption that "the API exists, just go build the module." That assumption is wrong today.

The fix is structural, not patchwork. Closures cannot cross WIT; the SDK API must be reshaped to take the same serializable values the WIT carries. This packet:

1. Defines three new types in `slicer-sdk`:
   - `EntityMutation` — enum with concrete `Set*` variants for the PrintEntity fields a near-future module will mutate. Exact variant list locked at Step 0 after auditing future-module design intent.
   - `SortKey` — enum with concrete sort-key variants. Exact list locked at Step 0.
   - `SyntheticLayerData` — record with the minimal `(z, paths)` payload sufficient to author a synthetic layer; host fills sibling fields.
2. Replaces the SDK closure-typed signatures with enum-typed ones that match the existing WIT shapes (introduced in packet 40 Step 3b).
3. Refactors the SDK's internal `MergeOp` enum to carry only serializable variants; removes `Box<dyn FnOnce>` / `Box<dyn FnMut>` storage.
4. Updates `apply_to` to translate each new `MergeOp` variant directly to a concrete in-place mutation.
5. Migrates the 8 existing `crates/slicer-sdk/tests/finalization_builder_tdd.rs` tests from closure-form to enum-form.
6. Extends the `slicer-macros` drain-back loop to forward `merge_ops` via WIT.
7. Adds a tiny new test guest at `test-guests/finalization-mutation-roundtrip-guest/` and a host-side end-to-end test that proves a guest's `modify_entity` call actually mutates the host IR — the substantive validation absent today.
8. Closes `DEV-041` in `docs/DEVIATION_LOG.md` (the live registry; the row currently sits at line 47). The legacy `docs/14_deviation_audit_history.md` is an archive and is not edited.

The result: WASM modules and native test fixtures share one API shape. The drain-back loop is straight-line forwarding, no impedance mismatch. The four future PostPass modules can be authored against a contract that actually delivers what it promises.

## Architecture Constraints

- **Closure-free SDK API for the three mutation methods**. This is the load-bearing invariant of the packet. After Step 2, `modify_entity` / `sort_layer_by` / `insert_synthetic_layer_after` MUST take only types that are `Serialize + Deserialize + Clone + Debug` (or whatever subset the SDK's transport convention requires). No `Box<dyn FnOnce>`, no `Box<dyn Fn>`, no impl-trait closure parameters. NEG-4 grep-asserts this contract.
- **`MergeOp` is plain data**. The internal `Vec<MergeOp>` storage on `FinalizationOutputBuilder` becomes plain serializable data. No type erasure, no boxed closures. Tests that previously used `Box::new(|e| …)` are now constructing `EntityMutation::Set*` variants directly.
- **WIT and SDK shapes are unified**. Packet 40 Step 3b introduced `entity-mutation`, `sort-key`, `synthetic-layer-data` on the WIT side. This packet ensures the SDK uses the SAME shapes (or trivially mappable equivalents). The `wit_host.rs` translation layer becomes a one-liner per method (or vanishes entirely if the bindgen can autoderive).
- **No producer-order or role-priority changes**. The role-priority table from Packet 40 stays put. `apply_to`'s 5-phase merge order stays put: (1) extend + ID-stamp, (2) stable-sort by `(priority, original_index)`, (3) apply `MergeOp::ModifyEntity`, (4) apply `MergeOp::SortLayer`, (5) apply `MergeOp::InsertSynthLayer` at outer Vec.
- **Operation recording, not direct mutation**. Every WIT call still RECORDS an op; the host applies them after the module returns. Same model as Packet 40; only the recording shape changes.
- **Stable identity**. Mutations match by `entity_id` (Packet 39 invariant). NEG-1 and NEG-3 verify the unknown-id error path at SDK and WIT layers respectively.
- **Backwards compatibility for `push_entity_with_priority` and `push_entity_to_layer`**. Both stay closure-free already; this packet does not touch them. AC-8 verifies benchy still passes, which exercises the legacy alias via skirt-brim.
- **Drain-back symmetry**. After Step 5, the drain-back forwards BOTH `priority_pushes` AND `merge_ops`. AC-7 grep-asserts the iteration site exists; the round-trip ACs (5, NEG-3) prove the forwarding works end-to-end.
- **`SyntheticLayerData` defaults**. When `apply_to` constructs a `LayerCollectionIR` from `SyntheticLayerData { z, paths }`, sibling fields (`global_layer_index`, `is_top_layer`, `is_bottom_layer`, `is_first_layer`, `is_last_layer`, `region_membership`, `travel_moves`, `ordered_entities` initial state, etc.) get sensible defaults: `global_layer_index` is the insertion index in the new outer Vec; layer-flag booleans are all `false` (synthetic layers are not first/last/top/bottom by default); `region_membership` is empty; `travel_moves` is empty; `ordered_entities` is built from the supplied `paths` with `entity_id`s stamped from a fresh `LayerEntityIdGen`. Locked at Step 0 audit; documented inline at the construction site.

## Data and Contract Notes

- IR or manifest contracts touched:
  - No new IR fields.
  - No new manifest entries.
- WIT boundary considerations: the WIT shapes for `entity-mutation`, `sort-key`, `synthetic-layer-data` already exist (Packet 40 Step 3b). Step 4 confirms alignment. If the SDK names diverge from the WIT names, Step 4 chooses one canonical form (recommend SDK adopts WIT names verbatim, or vice-versa, with consistent kebab/snake/camel conventions per Rust/WIT idiom).
- Determinism / scheduler constraints:
  - PostPass is sequential (`docs/04_host_scheduler.md:680–717`). No parallelism inside the merge.
  - Stable-sort is mandatory (Packet 40 invariant; preserved here).
  - Multiple `merge_ops` of the same kind apply in record-order (preserved from Packet 40).

## Locked Assumptions and Invariants

- `PrintEntity` shape is unchanged from Packet 39+40. `entity_id`, `path`, `role`, `region_membership`, `travel_moves` are the relevant fields.
- `ExtrusionPath3D` carries exactly three fields today: `points: Vec<Point3WithWidth>`, `role: ExtrusionRole`, `speed_factor: f32` (`crates/slicer-ir/src/slice_ir.rs:1285-1293`). There is **no** `extrusion_width_factor` field at the path level; per-point geometry lives on `Point3WithWidth { x, y, z, width: f32, flow_factor: f32 }` (`crates/slicer-ir/src/slice_ir.rs:1212-1224`). `EntityMutation::SetSpeedFactor(f32)` mutates the single path-level `speed_factor` field. `EntityMutation::SetFlowFactor(f32)` is **per-point**: `apply_to` walks `path.points` and assigns the supplied factor to every `Point3WithWidth.flow_factor`. This is the volumetric lever (matches `E_delta = distance × point.width × point.flow_factor` in `crates/slicer-host/src/gcode_emit.rs`) and is the correct hook for `FlushVolumeCalculator`. A path-level `SetExtrusionWidthFactor` was considered and explicitly rejected for this packet — `ExtrusionPath3D` carries no such field, OrcaSlicer does not carry a path-level width-factor either (it uses role-based `flow_ratio` on `mm3_per_mm` at G-code time), and adding a new IR field is outside this packet's scope. Defer until a real consumer surfaces.
- `apply_to`'s 5-phase merge order is preserved.
- The `push_entity_with_priority` method from Packet 40 is unchanged — closure-free already.
- The `push_entity_to_layer` legacy alias is unchanged.
- `LayerEntityIdGen` (from `slicer-ir`) is the canonical source for stamping `entity_id` on synthetic-layer entities; `apply_to`'s synthetic-layer construction uses a fresh generator per inserted layer (Packet 40 invariant; preserved).
