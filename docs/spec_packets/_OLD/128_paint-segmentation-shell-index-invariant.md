---
status: implemented
packet: 128_paint-segmentation-shell-index-invariant
task_ids:
  - TASK-253
---

# 128_paint-segmentation-shell-index-invariant

## Goal

Scope the paint_segmentation shell-depth propagation by `object_id` so each object's regions carry their own per-object depths, and lock the per-object-per-layer invariant with a `debug_assert!`, a multi-object mixed-height test, and a propagation-block doc contract.

## Problem Statement

Packet `126_mmu-painted-cube-parity` shipped an ad-hoc fix for a Phase 6/7 None arm that left `top_shell_index = None` on freshly-created regions (post-mortem root cause: the None arm created a `SlicedRegion { ..Default::default() }` outside the propagation block's harmonisation loop). That fix is correct for the single-object case and the 233/233-green suite passed — but a grilling session found the deeper root cause one function up: the propagation block at `mod.rs:887-916` harmonises shell depths **per-layer-global** via `saved_top_idx = saved_top_idx.or(r.top_shell_index)` (first-`Some`-wins) across ALL regions on a layer, with no `object_id` guard.

The producer (`slice_postprocess_prepass.rs:362-373`) computes shell depths **per-object** — each object's regions get depths from that object's own shell zone. On a multi-object mixed-height build (e.g. a 10 mm cube and a 50 mm cube on one plate), at a layer near the short cube's top, the short cube's region is `Some(0)` (exposed) and the tall cube's region is `None` (deep interior, outside any shell zone of that object). The per-layer-global `.or()` picks the short cube's `Some(0)` and stamps it onto the tall cube's regions — causing `top-surface-ironing` (lib.rs:321) to iron the tall cube's mid-body, `gyroid-infill` (lib.rs:189) to route it to the exposed-solid-fill role, and `only_one_wall_top` (classic-perimeters lib.rs:198, arachne-perimeters lib.rs:204) to drop walls on the wrong object.

Why this matters now: no existing paint_segmentation test uses more than one object (the `phase6_7_none_arm_stamps_shell_index_on_new_region` fixture at mod.rs:2391-2581 creates a single `"obj1"`), so the latent cross-object corruption is uncaught by a green suite. The ad-hoc Phase 6/7 fix was a symptom treatment; this packet closes the underlying scope bug and locks the correct invariant so it cannot silently regress.

## Architecture Constraints

- Packet-specific constraint: the propagation block's accumulator MUST be scoped by `SlicedRegion.object_id` (field at `crates/slicer-ir/src/slice_ir.rs:1228`, type `ObjectId`). The producer (`slice_postprocess_prepass.rs:362-373`) computes depths per-object; the propagation must preserve that per-object identity, not coalesce across objects on a layer.
- Packet-specific constraint: the `debug_assert!` invariant is per-object-per-layer, NOT per-layer-global. Cross-object regions on the same layer are explicitly allowed to disagree. The negative test AC-N2 exists to prevent the wrong (per-layer-global) invariant from being re-introduced.
- Packet-specific constraint: no `SlicedRegion` schema change. The fields `object_id`, `top_shell_index`, `bottom_shell_index` already exist with correct types; this packet only changes how the propagation block populates the latter two. No WIT bump, no guest WASM rebuild.
- (No `wasm-staleness` snippet — host-only change, no path feeds guest WASM.)
- (No `coord-system` snippet — packet touches shell-depth bookkeeping, not geometry or mm↔unit conversion.)

## Data and Contract Notes

- IR contracts touched: `SlicedRegion.top_shell_index` / `bottom_shell_index` population semantics (not the schema). The contract change is: these fields now carry per-object depths end-to-end (producer → propagation → consumers), where previously the propagation block corrupted them across objects. The fields' types and doc comments are unchanged.
- WIT boundary considerations: none. `SlicedRegion` is host-internal IR; it does not cross the WIT boundary. Guest modules read shell-index-derived *roles* (e.g. `solid_fill_role()`) via the SDK, not the raw `Option<u8>`.
- Determinism or scheduler constraints: the per-object HashMap iteration order must be deterministic (use `ObjectId` ordering or a `BTreeMap` if `ObjectId` is not `Hash`+`Ord` — confirm via sub-agent if the compiler rejects `HashMap`). The propagation block runs once per layer in `execute_paint_segmentation`; no scheduler re-entry.

## Locked Assumptions and Invariants

- `SlicedRegion.object_id: ObjectId` exists at `slice_ir.rs:1228` and uniquely identifies the owning object per region (confirmed via sub-agent FACT during grilling). No indirection needed.
- The producer computes shell depths per-object (confirmed at `slice_postprocess_prepass.rs:362-373`). The propagation block MUST preserve per-object identity; cross-object coalescence is a bug.
- The invariant to lock: **for each layer, all regions sharing the same `object_id` MUST share the same `top_shell_index` and `bottom_shell_index`**. Cross-object disagreement on a layer is legal and expected for mixed-height builds.
- The four Phase 5 `SlicedRegion { ..Default::default() }` sites at mod.rs:724-802 are pre-propagation and are harmonised by the propagation block; they do NOT need editing.
- The existing single-object tests (including `phase6_7_none_arm_stamps_shell_index_on_new_region`) remain valid because the per-object invariant degenerates to per-layer agreement when only one object exists.

## Risks and Tradeoffs

- `ObjectId` may not implement `Hash`+`Eq` (required for `HashMap` key). Mitigation: confirm via a sub-agent `cargo check` dispatch; if missing, use `BTreeMap` (needs `Ord`) or a `Vec<(ObjectId, _)>` linear scan (small N — regions per layer is typically ≤ tens).
- The HashMap allocation per layer adds a small constant cost to `execute_paint_segmentation`. Mitigation: N (regions per layer) is small; the HashMap is built and dropped per layer. No bench in the gate; if a regression is suspected post-merge, `cargo bench -p slicer-runtime --bench per_stage` is the follow-up (not this packet).
- The `debug_assert!` runs only under `#[cfg(debug_assertions)]`; release builds skip it. This matches `compose_variants.rs:166` precedent. Risk: a per-object mismatch in release is silent. Tradeoff: acceptable — the structural invariant tests catch the mismatch in CI (debug), and the cost of a runtime check in release on every layer is unjustified for a bookkeeping invariant.
- Reframing the invariant from per-layer-global (original post-mortem) to per-object (this packet) changes the contract that packet 126's ad-hoc fix assumed. Risk: 126's single-object tests still pass (the per-object invariant degenerates correctly), confirmed by AC-4 and AC-N4. No 126 test assumes cross-object sharing.
