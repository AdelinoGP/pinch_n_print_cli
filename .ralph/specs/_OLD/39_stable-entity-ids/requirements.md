# Requirements: stable-entity-ids

## Packet Metadata

- Grouped task IDs:
  - `TASK-170` (NEW; foundation for Packet 40 and future PostPass mutators)
- Backlog source: `docs/07_implementation_status.md`
- Packet status: `draft`
- Aggregate context cost: `M`
- Supersedes: none
- Unblocks: `40_finalization-mutation-builder`; future `SequentialPrintOrder`, `MinLayerTimeEnforcer`, `FlushVolumeCalculator`, `PrimeTower` modules

## Problem Statement

`LayerCollectionIR.travel_moves` currently anchors each travel by an integer index into `ordered_entities` (working hypothesis: `TravelMove.entity_idx: u32`). This makes the index a fragile foreign-key reference: any operation that reorders, inserts into, or removes from `ordered_entities` must hand-rewrite every dependent travel anchor, otherwise G-code emits travels at wrong positions.

This bit Packet `38-rev1_top-surface-ironing` indirectly: the host's finalization-merge site (`crates/slicer-host/src/dispatch.rs:2877`) uses `splice(0..0, fin_entities)` — literal prepend — partly because any other insertion site would have to walk the layer's travel anchors and increment them. The result: top-surface-ironing entities currently print BEFORE top-fill (semantically wrong) because the host has no cheap way to position them after fill.

The architecture document `docs/01_system_architecture.md:328-363` declares `PostPass::LayerFinalization` semantics as *"Vec<LayerCollectionIR> (all layers, mutable — may append entities or insert synthetic layers)"*, but the current implementation enforces a strictly weaker push-only contract because the data structure cannot tolerate mutation cheaply. Several upcoming modules listed in that same doc — `SequentialPrintOrder`, `MinLayerTimeEnforcer`, `FlushVolumeCalculator`, `PrimeTower` — need richer mutation primitives (reorder, in-place edit, synthetic layer insert). All of those primitives are easy with stable IDs and combinatorially expensive without.

This packet introduces `entity_id: u64` as a per-layer-monotonic stable identifier on every entity in `ordered_entities`, and migrates `TravelMove` to anchor by `entity_id` instead of position. Producers issue IDs at construction. `gcode_emit` builds a one-shot `entity_id → index` map per layer. **No G-code byte-level behavioral change** — it is a pure structural refactor whose payoff is unblocking the next packet (`40_finalization-mutation-builder`) and every PostPass mutator downstream.

## In Scope

- Add `entity_id: u64` field to whichever struct sits in `LayerCollectionIR.ordered_entities` (Step 0 confirms exact name; working hypothesis `PrintEntity` per session memory near `crates/slicer-ir/src/slice_ir.rs:1228`).
- Migrate `TravelMove`'s anchor field to `entity_id: u64` (current name confirmed by Step 0; working hypothesis `entity_idx`).
- Provide a `LayerEntityIdGen` helper in `slicer-ir` that issues monotonic `u64` IDs per layer. The host owns one instance per `LayerCollectionIR` constructed.
- Migrate every workspace producer site:
  - `crates/slicer-host/src/layer_executor.rs:605-617` (PerimeterIR walls)
  - `crates/slicer-host/src/layer_executor.rs:619-636` (InfillIR fills)
  - `crates/slicer-host/src/layer_executor.rs:638-659` (SupportIR paths)
  - `crates/slicer-host/src/dispatch.rs:2861-2877` (finalization merge — issues IDs to module-pushed entities at merge time)
- Migrate `crates/slicer-host/src/gcode_emit.rs:182, 285-295` to use `HashMap<u64, usize>` lookup for travel anchor resolution.
- Add an IR validation helper in `slicer-ir` (function or impl) that scans a `LayerCollectionIR` and returns `Err` if any `TravelMove.entity_id` is dangling. Used by tests; may also be wired into a host pre-emit assertion in debug builds (out of scope to wire into release).
- Sweep workspace test fixtures touching the migrated fields. Step 0 LOCATIONS dispatch returns the exhaustive list. Each fixture site is mechanically migrated (a one-pattern edit per call site).
- Bump IR schema version per `docs/02_ir_schemas.md` versioning rules; record the bump in `docs/14_deviation_audit_history.md` if the schema-version policy requires it (Step 0 FACT confirms).
- Insert `TASK-170` row into `docs/07_implementation_status.md` at acceptance ceremony.

## Out of Scope

- `FinalizationOutputBuilder` API additions: `push_entity_with_priority`, `modify_entity`, `sort_layer_by`, `insert_synthetic_layer_after` — owned by Packet 40.
- Replacing `dispatch.rs:2877` `splice(0..0, ...)` with `extend` + role-priority sort — owned by Packet 40.
- `ExtrusionRole::default_priority()` const fn / role priority table — owned by Packet 40.
- Any module-level call-site change (`top-surface-ironing`, `skirt-brim`, `wipe-tower`, etc.) — owned by Packet 40 (one-line migrate of `top-surface-ironing` only).
- Changing the G-code byte output for any input STL — explicitly forbidden by this packet's regression contract (AC-2).
- Adding mutation/query APIs (`output.layer(idx).entities()`, `output.modify_entity(...)`, etc.) — owned by Packet 40.
- Synthetic layer insertion API — owned by Packet 40.
- Wiring the dangling-anchor validator into the release-build emit path — out of scope; tests-only for this packet.
- Concurrent producer ID issuance via a `Send + Sync` shared counter — out of scope; layers are constructed by a single executor task per `docs/04_host_scheduler.md`. Step 0 confirms; if false, the helper uses an `AtomicU64`.

## Authoritative Docs

- `docs/02_ir_schemas.md` — `LayerCollectionIR`, `TravelMove`, schema versioning rules. Read directly; one section.
- `docs/01_system_architecture.md` lines 328–363 — `PostPass::LayerFinalization` mutability contract motivating this refactor. Read directly; small section.
- `docs/04_host_scheduler.md` § Composable Multi-Writer Patterns (lines 309–317) — describes producer concurrency model for `ordered_entities`. Read directly; narrow.
- `docs/05_module_sdk.md` — only the existing `FinalizationOutputBuilder::push_entity_to_layer` signature; delegate SUMMARY ≤ 100 words for that section.

## OrcaSlicer Reference Obligations

None. OrcaSlicer's IR uses different identity semantics (per-region path indices); not a parity reference. All OrcaSlicer reads remain forbidden by the project's context-discipline preamble.

## Acceptance Summary

- Positive cases:
  1. Every entity has a unique `entity_id` per layer; every `TravelMove.entity_id` resolves to a present entity.
  2. Benchy end-to-end regression: every existing assertion in `benchy_end_to_end_tdd` continues to PASS.
  3. Travel `G0` move endpoints match the entity referenced by `entity_id` (verified at the parsed-G-code level).
  4. Travel anchors survive a synthetic permutation of `ordered_entities` — proves the index-independence the refactor exists to provide.
  5. `entity_id` and `TravelMove.entity_id` round-trip through bincode/serde.
  6. `LayerEntityIdGen::next()` is strictly monotonic.
- Negative cases:
  1. Dangling `TravelMove.entity_id` is rejected by the IR validation helper with a diagnostic naming the offending ID.
  2. `LayerEntityIdGen` is contention-safe (atomic interior mutability OR statically `!Send` and tested via `static_assertions`).
- Measurable outcomes:
  - `cargo test -p slicer-ir --test entity_id_invariants_tdd` PASS (4 tests).
  - `cargo test -p slicer-ir --test ir_validation_tdd dangling_travel_anchor_rejected` PASS.
  - `cargo test -p slicer-host --test gcode_emit_travel_anchor_tdd` PASS (2 tests).
  - `cargo test -p slicer-host --test benchy_end_to_end_tdd` PASS (regression).
  - `cargo build --workspace` PASS.
  - `./modules/core-modules/build-core-modules.sh` PASS.
  - `cargo clippy --workspace -- -D warnings` PASS.
  - `cargo test --workspace` PASS at acceptance ceremony.
- Cross-packet impact:
  - Unblocks `40_finalization-mutation-builder`. Packet 40 cannot start until `entity_id` is in place.

## Verification Commands

- `cargo test -p slicer-ir --test entity_id_invariants_tdd -- --nocapture`
- `cargo test -p slicer-ir --test ir_validation_tdd -- --nocapture`
- `cargo test -p slicer-host --test gcode_emit_travel_anchor_tdd -- --nocapture`
- `cargo test -p slicer-host --test benchy_end_to_end_tdd -- --nocapture`
- `cargo test -p slicer-host --test manifest_ingestion_tdd -- --nocapture`
- `cargo test -p slicer-host --test claim_transition_matrix_tdd -- --nocapture`
- `cargo build --workspace`
- `./modules/core-modules/build-core-modules.sh`
- `cargo clippy --workspace -- -D warnings`
- `cargo test --workspace` (closure gate only)

## Step Completion Expectations

For each step in `implementation-plan.md`:

- Precondition stated explicitly.
- Postcondition observable.
- Falsifying check.
- Files allowed to read with line ranges where > 300 lines.
- Files allowed to edit ≤ 3.
- Expected sub-agent dispatches.
- Step context cost: S or M (no L).

## Context Discipline Notes

- Large files in the read-only path (delegate; do NOT load full):
  - `crates/slicer-host/src/dispatch.rs` — only the finalization-merge site (lines ~2861–2877) and any `entity_idx` reference. FACT-narrowed reads only.
  - `crates/slicer-host/src/gcode_emit.rs` — only the layer-emit loop (~line 182) and the travel-resolution site (~lines 285–295).
  - `crates/slicer-host/src/layer_executor.rs` — only the producer sites (~lines 605–659).
- Likely temptation reads (avoid):
  - All of `crates/slicer-ir/src/slice_ir.rs` beyond the `LayerCollectionIR` and `TravelMove` and `PrintEntity` (or whatever the entity struct is named) sections.
  - All other crates not listed above.
- Sub-agent return formats:
  - cargo runs → FACT pass/fail with failing-assertion ≤ 20 lines on FAIL.
  - Workspace `entity_idx` sweep → LOCATIONS (≤ 30 entries; if more, paginate by crate).
  - Reference SDK API discovery → FACT or LOCATIONS (file:line).
