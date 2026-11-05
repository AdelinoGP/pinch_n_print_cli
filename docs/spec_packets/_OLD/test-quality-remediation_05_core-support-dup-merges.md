---
status: implemented
packet: test-quality-remediation_05_core-support-dup-merges
task_ids:
  - core/DUP-CORE (support overhang)
  - core/DUP-CORE (support geometry)
---

# test-quality-remediation_05_core-support-dup-merges

## Goal

Consolidate the coplanar-step overhang witness and the two inline support-geometry twins into their stronger integration-test survivors while preserving the exact step layer, expanded-back contact area, emitted-entry, empty-plan, and per-object schedule assertions.

## Problem Statement

The support-overhang target repeats its coplanar-step input as a weaker non-empty check after a stronger test already pins the only emitted layer and contact area. Separately, `support_geometry.rs` repeats two integration tests inside a production file's `#[cfg(test)]` module. Keeping both copies creates maintenance drift without protecting another input, but deleting by name alone could silently lose the RC-1 coplanarity rationale, exact layer/area assertions, per-object schedule sets, emitted-entry witness, or the integration-only empty-plan case.

## Architecture Constraints

- This is a merge-with-union test consolidation. It may remove only `coplanar_step_does_not_hide_the_contact`, `support_geometry_emits_for_2_layer_fixture`, and the inline definition of `build_emit_schedule_two_objects_per_object_semantics`; every distinct fixture and assertion remains in a named survivor.
- Both integration targets carry `required-features = ["host-algos"]` in `crates/slicer-core/Cargo.toml` and in the census manifest. Every target command must enable `--features host-algos`; a bare narrow run is inadmissible because Cargo can skip the target and report success.
- The `support_geometry.rs` edit is confined to `#[cfg(test)] mod tests`; `support_geometry` itself is compiled only under `host-algos` in `slicer_core::algos`. The removed block is neither a doc example nor a guest-WASM input, so guest freshness is not implicated.
<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.
- Preserve canonical parity supremacy: the exact `64.0 mm²` expanded-back area witness and its `< 0.1 mm²` tolerance are retained, never loosened or replaced by non-emptiness.

## Data and Contract Notes

- IR/manifest contracts: no shape or version changes. Tests continue to inspect `SupportGeometryIR.entries`; schedule keys remain object IDs and values remain global model-layer indices.
- WIT boundary: none.
- Determinism/scheduler constraints: the exact `BTreeSet<u32>` schedule assertions remain deterministic: `obj-A` emits at `{1,3,5}` for `support_layer_height_mm = 0.4` over `0.2 mm` layers, while `obj-B` with `0.0` emits at every layer `{0..5}`.

## Locked Assumptions and Invariants

- Overhang source count is `19 → 18`; support-geometry integration count is `3 → 3`; support-geometry inline count is `2 → 0`. Every delta is accounted by the survivor maps above.
- The overhang survivor is `overhang_is_detected_once_at_the_step_layer`; its exact layer equality subsumes the absorbed non-empty assertion on an identical fixture, and its area assertion remains stronger.
- The support-geometry survivors are `emits_for_2_layer_fixture` and the integration definition of `build_emit_schedule_two_objects_per_object_semantics`; `empty_plan_produces_empty_support` is distinct and retained.
- The census manifest's role is target/feature registration only. Fresh `-- --list` output is the per-function execution census; no packet-time count may substitute for it at implementation.
- No public symbol, new test, new file, or later-packet export is introduced.
- The implementation-time ledger remains `partial`; it records this slice and leaves `core-wall-sequence-dup-merges` as remaining core work. Packet Queue edits are not part of implementation.

## Risks and Tradeoffs

- Deleting the entire inline module could accidentally remove production code if its boundary is misidentified. The edit begins at the sole `#[cfg(test)] mod tests` and ends at that module's closing brace; production ends before it.
- A count-only merge can look correct while losing assertion strength. AC-2 and AC-4 pin exact layer, area, entry, empty-plan, and schedule assertions in the surviving integration homes.
- The external and inline schedule tests have equal operands but different failure diagnostics. Copying `got {a_sched:?}` and `got {b_sched:?}` into the integration survivor preserves the useful union without retaining duplicate registration.
- Feature-gate blindness can produce a false green. Every behavioral command enables `host-algos` and confirms named discovery before accepting the run.
- A broad documentation edit could conflate implementation evidence with queue generation. Step 3 permits only the six-cell §7 `core` row; AC-7 parses the Ledger section and checks only queue header/packet-entry presence, while diff review enforces the no-queue-edit boundary.
