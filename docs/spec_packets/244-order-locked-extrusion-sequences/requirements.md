# Requirements: 244-order-locked-extrusion-sequences

## Packet Metadata

- Grouped task IDs: `TASK-354`
- Backlog source: `docs/07_implementation_status.md`
- Packet status: `draft`
- Aggregate context cost: `M`

## Problem Statement

Wave-overhang bridge fill (Packet 4) produces extrusion paths whose print order and direction are
physically load-bearing: fronts must be deposited anchored-first, and chained zigzag runs break if
reversed. Two downstream stages destroy such sequences today — the infill linker re-clips, chains,
and reverses bridge-role paths, and path optimization nearest-neighbor permutes role groups and may
reverse entities. A dedicated `ExtrusionRole` variant was rejected (one module's need hardcoded into
every consumer's match arms); a `Custom("…")` string convention was rejected (invisible typing,
per-consumer string matching). This packet lands the typed carrier — `ExtrusionPath3D.order_lock:
Option<u64>` — plus its WIT/`OrderedEntityView` projection, the SDK local-tag allocator, and the
host-side remap + enforcement contract, so Packet 3 can make consumers honor it and Packet 4 can
mint it. It changes nothing for existing slices: every path is `None` today, and all-`None` paths
take the old-equivalent branches.

## In Scope

- Add `ExtrusionPath3D.order_lock: Option<u64>` (`#[serde(default)]`) in
  `crates/slicer-ir/src/slice_ir.rs`.
- Add WIT `order-lock: option<u64>` to `record extrusion-path3d`
  (`crates/slicer-schema/wit/deps/types.wit`) and to `record ordered-entity-view`
  (`crates/slicer-schema/wit/deps/ir-types.wit`). Canonical source edit — host bindgen and guest
  macro read these files directly; the `slicer:types/geometry` package is unversioned (ADR-0044), so
  no WIT version tax.
- Project the field end-to-end: host `OrderedEntityView`
  (`crates/slicer-runtime/src/layer_executor.rs::project_ordered_entities`), wasm-host
  `project_ordered_entities_from` (`crates/slicer-wasm-host/src/dispatch.rs`), SDK `OrderedEntityView`
  (`crates/slicer-sdk/src/views.rs`), macro adapter (`crates/slicer-macros/src/lib.rs`), and the
  wasm-host marshal out.
- Bump `CURRENT_LAYER_COLLECTION_IR_SCHEMA_VERSION` from `1.3.0` to `1.4.0` (additive minor, the
  packet-226 `tool_index` precedent).
- Add the SDK local-tag allocator type (`OrderLockAllocator`) and the host remap function
  (`remap_order_locks_to_global`).
- Add host enforcement at the four mutation points (InfillPostProcess commit,
  `apply_entity_order_proposal`, finalization `apply_to`, `apply_cross_layer_tool_rotation`).
- Land ADR-0062 and the two `CONTEXT.md` glossary terms.
- Update `docs/02_ir_schemas.md` §"IR 12 — LayerCollectionIR".

## Out of Scope

- Making any consumer honor locks (infill linker, path optimizer, G-code emitter) — Packet 3.
- The swept-footprint carve and the anchor-band exception (D4) — Packet 3.
- The wave-overhangs module, its config keys, and internal-bridge exclusion — Packet 4.
- Any `ExtrusionRole` variant or `Custom("…")` convention.
- The second ADR ("Sequence-locked paths may occupy neighboring fill domains") and the
  `docs/02_ir_schemas.md` §"Post-`Layer::Perimeters` invariant" amendment — Packet 3.

## Authoritative Docs

- `docs/02_ir_schemas.md` - ~1650 lines; direct range reads only: §"IR 12 — LayerCollectionIR"
  (lines ~1185-1195) and §"IR Versioning Contract" (lines ~1633-1641). Delegate any other section.
- `docs/specs/wave-overhangs-bridge-fill-plan.md` - normative plan; §"Packet 2" (lines ~122-183),
  Appendix A (ADR draft), Appendix B (glossary).
- `docs/21_data_defaults_and_fixtures.md` - direct read of §3 (watchlist derivation) and §4 (waiver
  format) — `ExtrusionPath3D` becomes a watched type (5 fields) after this packet.

## Acceptance Summary

Reference, never copy, criteria from `packet.spec.md`.

- Positive: `AC-1` (schema 1.4.0), `AC-2` (carrier end-to-end), `AC-3` (all-`None` neutrality),
  `AC-4` (proposal enforcement), `AC-5` (remap), `AC-6` (allocator).
- Negative: `AC-N1` (InfillPostProcess preserves locked block), `AC-N2` (finalization rejects
  geometry change).
- Cross-packet impact: Packet 3 consumes `order_lock` in the linker/optimizer/emitter; Packet 4
  mints locks via `OrderLockAllocator` and relies on `remap_order_locks_to_global` at the output
  boundary.

## Verification Commands

This is the authoritative full matrix; `packet.spec.md` lists only the gate commands.

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo test -p slicer-ir --test ir_tests -- bridge_detector_schema_versions_are_constant_sourced --exact 2>&1 \| tee target/test-output.log \| grep -qE "^test result: ok\. 1 passed"` | schema constant 1.4.0 + default-sourced | FACT pass/fail |
| `cargo test -p slicer-runtime --test executor -- order_lock_all_none_neutrality --exact 2>&1 \| tee target/test-output.log \| grep -qE "^test result: ok\. 1 passed"` | all-`None` neutrality | FACT pass/fail |
| `cargo test -p slicer-runtime --test unit -- order_lock_proposal_split_rejected --exact 2>&1 \| tee target/test-output.log \| grep -qE "^test result: ok\. 1 passed"` | proposal enforcement | FACT pass/fail |
| `cargo test -p slicer-runtime --test unit -- order_lock_remap_local_to_global --exact 2>&1 \| tee target/test-output.log \| grep -qE "^test result: ok\. 1 passed"` | remap + `Some(0)`/unknown-global rejection | FACT pass/fail |
| `cargo test -p slicer-runtime --test executor -- order_lock_infill_postprocess_preserves_block --exact 2>&1 \| tee target/test-output.log \| grep -qE "^test result: ok\. 1 passed"` | InfillPostProcess enforcement | FACT pass/fail |
| `cargo test -p slicer-sdk --test finalization_builder_tdd -- order_lock_finalization_rejects_geometry_change --exact 2>&1 \| tee target/test-output.log \| grep -qE "^test result: ok\. 1 passed"` | finalization enforcement | FACT pass/fail |
| `cargo test -p slicer-sdk --test finalization_builder_tdd -- order_lock_allocator_sequence --exact 2>&1 \| tee target/test-output.log \| grep -qE "^test result: ok\. 1 passed"` | allocator sequence | FACT pass/fail |
| `cargo xtask build-guests --check` | guest freshness after WIT change (exit 0 = fresh) | exit code |
| `cargo xtask check-literals` | watched-type literal gate | exit code |
| `cargo check --workspace --all-targets` | every affected crate + test target compiles | FACT pass/fail |
| `cargo clippy --workspace --all-targets -- -D warnings` | lint gate | FACT pass/fail |

## Step Completion Expectations

- Step 1 (field + WIT + projection + schema bump) must land the `ExtrusionPath3D` field, the WIT
  records, the three `OrderedEntityView` projections, and the constant bump together — the tree is
  non-compiling between the field addition and the projection/marshal updates.
- Step 2 (allocator + remap) is independent of Step 1's consumers and may land in parallel.
- Step 3 (enforcement) depends on Step 1 (the field must exist to validate) and Step 2 (the remap
  must exist to test the output boundary).
- Step 4 (docs) is last and depends on all code landing.

## Context Discipline Notes

- The `ExtrusionPath3D` struct-literal blast radius is large (100+ literals across `crates/*` and
  `modules/*`); it is compiler-enforced for `src/` literals and `cargo xtask check-literals`-enforced
  for test literals. Do not enumerate every site by hand — the two gates are the enforcement, and the
  step's "Files allowed to edit" lists the categories.
- `docs/02_ir_schemas.md` is over 300 lines — read only the two named ranges, delegate the rest.
