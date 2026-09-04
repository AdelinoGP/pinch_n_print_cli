# Task Map: 240a-support-raft-substrate

Crosswalk for this packet's share of queue row #7 of
`docs/specs/support-families-anchored-entities-plan.md`. Row #7 originally
allocated `TASK-409`..`TASK-418` to a single `240-support-raft` packet; that
packet was split at preflight into **240a-support-raft-substrate** (this one,
`TASK-409`..`TASK-413`) and **240b-support-raft-module**
(`TASK-414`..`TASK-418`). The split exposed scope the original allocation did
not cover, so this packet also carries `TASK-533`..`TASK-536`. **Re-derive the
free range before allocating any further ID** —
`rg -o 'TASK-[0-9]{3}' docs/ -N --no-filename | sort -u | tail -1` — rather
than trusting any boundary implied here.

**None of these task IDs exists in `docs/07_implementation_status.md` today**
(verified 2026-09-04). The completion gate ADDS the rows; it does not update
them. Re-derive this before acting on it.

**Banding note.** This packet was re-authored on 2026-09-04 from a signed
negative band to a positive offset band. The `u32` to `i32` migration that
dominated the earlier `TASK-410`/`TASK-411` allocation is withdrawn; those IDs
now carry the marker field and the object-bottom audit instead. See
`requirements.md` section "Banding Decision".

| docs/07 task ID | Packet step | Primary docs | Expected code surface | OrcaSlicer refs | Context cost | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| `TASK-409` | `Step 1` | plan section 12 | `crates/slicer-ir/tests/{raft_band_ir_tdd,sliced_region_raft_fill_tdd}.rs` | none | S | Red-first IR contract; the stale signed-band test file was already deleted in the re-spec |
| `TASK-410` | `Step 2` | `docs/03_wit_and_manifest.md` | `crates/slicer-ir/src/slice_ir.rs` (`GlobalLayer.is_raft`) | `PrintObjectSlice.cpp::new_layers` (delegated) | M | Marker field + literal-gate fallout |
| `TASK-533` | `Step 2` | `docs/03_wit_and_manifest.md` | `prepass-layer-planning.wit`, `crates/slicer-wasm-host/src/marshal/{in_,native}.rs`, `crates/slicer-sdk/src/prepass_types.rs` (SDK `LayerProposal` mirror — the native leg reads it), `crates/slicer-macros/src/lib.rs` (WIT `LayerProposal` literal) (tests in `in_.rs`'s existing `#[cfg(test)] mod tests`; the harvest fn is `pub(crate)`) | `PrintObjectSlice.cpp::new_layers` (delegated) | M | `is-raft-prefix` on both harvest legs + contiguity rejection |
| `TASK-534` | `Step 3` | `docs/08_coordinate_system.md` | `modules/core-modules/layer-planner-default/*`, `crates/slicer-runtime/tests/integration/raft_band.rs` | `Slicing.cpp::generate_object_layers` (delegated) | M | Planner emits the `0..N-1` band; manifest key declared (E9); guest rebuild |
| `TASK-411` | `Step 4` | `design.md` section "First-Model-Layer Audit" | `detect_support_contacts` + `SupportContactParams` (`crates/slicer-core/src/algos/overhang_annotation.rs`), `resolve_contact_params` (`crates/slicer-runtime/src/builtins/support_analysis_producer.rs`), `run_perimeters` (`modules/core-modules/classic-perimeters/src/lib.rs`) | none | M | Three object-bottom conversions + the config bridge that makes two of them non-inert; every other site explicitly untouched |
| `TASK-413` | `Step 4` | `docs/DEVIATION_LOG.md` (DEV-124 row) | `crates/slicer-runtime/tests/{contract,executor}/*` | none | M | Positional-contract + DEV-124-upheld regression guards |
| `TASK-412` | `Step 5` | `design.md` section "Locked Assumptions" | `crates/slicer-ir/src/slice_ir.rs` (`SupportPlanEntry` struct-level AND field-level docs), `crates/slicer-schema/wit/deps/ir-types.wit` (header comment) | none | S | Removes all THREE stale negative-band promises; no type change |
| `TASK-535` | `Step 6` | `docs/02_ir_schemas.md` | `slice_ir.rs`, `ir-types.wit`, `region_partition.rs` + carrier footprint | none | M | `raft_fill` carrier + both accessors + schema minor bump (next minor above the live `CURRENT_SLICE_IR_SCHEMA_VERSION`, re-derived at edit time) |
| `TASK-536` | `Steps 7+8` | `docs/03_wit_and_manifest.md`, `docs/02_ir_schemas.md` | `ir-types.wit`, `host.rs`, `dispatch.rs`, `traits.rs`, `slicer-macros/src/lib.rs`, `test-guests/layer-infill-guest/src/lib.rs` (must CALL the accessors or AC-7 is vacuous); then docs + `DEVIATION_LOG.md` | none | M+S | `raft_plan` AND `is-raft` read paths (the latter is what lets 240b's `Layer::Infill` guest see a raft layer); positive-band deviation row (DEV-124 upheld, NOT reopened) |

Copy costs from `implementation-plan.md`. Aggregate is `M`; no row is `L`.
