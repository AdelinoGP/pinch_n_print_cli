# Task Map: 239b-anchored-wit-contract

Crosswalk for `docs/07_implementation_status.md`. Registration is **packet-owned closure work**:
Step 7 (`TASK-514`) appends these rows through a worker dispatch; nothing is registered at
authoring time.

Ledger warning: the `TASK-###` high-water mark, the next free `G-` gap row, and the next free
`DEV-` id are mutable shared state that changes while you work. `TASK-507` and `DEV-157` were the
values read at the 239 split, and `239a-anchored-host-seams` may register concurrently. **The
next-free `G-` row is CONTESTED** — the highest existing row in
`docs/specs/support-parity-gap-register.md` has been reported as both `G-26` and `G-19` by
different readers, so the `G-27` an earlier draft froze is not a settled figure. Derive it from
the file, never from this packet. **Re-derive
the tail of `docs/07_implementation_status.md` and the gap register at the moment you append** —
do not trust the values quoted in this paragraph or anywhere else in this packet.

| docs/07 task ID | Packet step | Primary docs | Expected code surface | OrcaSlicer refs | Context cost | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| `TASK-508` | `Step 1` | `docs/specs/support-independent-layer-z-split-plan.md` F5/F6/F7; `docs/05_module_sdk.md` (SUMMARY) | none (read-only); inventory of `deconstruct_layer_ctx`, `commit_native_layer_response`'s `match stage_export` (`crates/slicer-wasm-host/src/marshal/native.rs`), `STAGES`, `VALID_STAGES`, `STAGE_ORDER`, ADR-0020 gate | none | S | Re-verifies F7 rather than trusting it; a non-zero reference count halts the packet |
| `TASK-509` | `Step 2` | `docs/03_wit_and_manifest.md` (SUMMARY); `docs/04_host_scheduler.md` (SUMMARY); `docs/05_module_sdk.md` (SUMMARY) | `crates/slicer-schema/wit/deps/layer-anchored-events/layer-anchored-events.wit` (new), `crates/slicer-schema/wit/deps/ir-types.wit`, `crates/slicer-schema/src/lib.rs`, `crates/slicer-scheduler/src/execution_plan.rs`, `crates/slicer-macros/{src/lib.rs,build.rs}`, `xtask/src/wit_verify.rs`, `crates/slicer-runtime/tests/contract/layer_stage_commit_stages_tdd.rs`, `crates/slicer-ir/src/stage_io.rs` (**comment only** — `LayerStageCommit::stage_id`'s "eight `world-layer` stages" becomes nine) | none | M | Largest step; the three declaration-model surfaces cross-check each other and must move together. ADR-0020 gate goes 8→9 |
| `TASK-510` | `Step 3` | `CLAUDE.md` §"Guest WASM Staleness" | `crates/slicer-wasm-host/test-guests/anchored-events-roundtrip-guest/` (new), `crates/slicer-runtime/tests/executor/anchored_events_roundtrip_tdd.rs` (new) **+ `crates/slicer-runtime/tests/executor/main.rs` (`mod` line — the compile trigger)**, `crates/slicer-sdk/src/traits.rs` | none | M | Red-first; guest modelled on `finalization-mutation-roundtrip-guest`, config-parameterized so one binary serves all seven fixtures (`emit_malformed_geometry` is tri-valued: 0 clean / 1 planar / 2 z-span) |
| `TASK-511` | `Step 4` | `docs/08_coordinate_system.md` (SUMMARY) | `crates/slicer-wasm-host/src/marshal/{accumulators.rs,out.rs,mod.rs}`, `crates/slicer-wasm-host/src/host.rs` (incl. the **`pub mod layer_anchored_events` `bindgen!` module** + `LayerAnchoredEventsModule` alias) | none | M | Copies the `support-output-builder` lift chain; `s64` carried as `i64`, no `f32` hop. Also authors `validate_anchored_entity_geometry` — a **duplicate** of `validate_anchored_entity`, forced by the `slicer-runtime` → `slicer-wasm-host` edge |
| `TASK-512` | `Step 5a` | `CLAUDE.md` §"WIT/Type Changes Checklist" | `crates/slicer-wasm-host/src/dispatch.rs` — the **layer-tier linker/instantiate/call** `match stage_id.as_str()` (bound as `let (call_result, mut store, mem_initial_bytes) = ...`) | none | M | The surface that actually runs the guest. Not mechanical: a full copy of the `"Layer::Support"` arm. The prepass-tier match (`let (call_result, mut store) = ...`) is out of scope |
| `TASK-512` | `Step 5b` | `CLAUDE.md` §"WIT/Type Changes Checklist"; `docs/adr/0059-support-families-and-anchored-entities.md` | `crates/slicer-wasm-host/src/dispatch.rs` (`deconstruct_layer_ctx` arm), `crates/slicer-wasm-host/src/marshal/native.rs`, `crates/slicer-wasm-host/tests/contract/anchored_events_both_legs_tdd.rs` (new) + `crates/slicer-wasm-host/tests/contract/main.rs` (`mod` line) | none | M | Both-legs guard: the wasm arm and its native twin land in one commit. `Ok(None)` on empty output. Rejects **both** declared geometry contracts (AC-N1 planar, AC-N4 z-span). Shares `TASK-512` with Step 5a — same id, distinct step number; no id outside `TASK-508`..`TASK-514` is minted |
| `TASK-512` | `Step 5c` | `CLAUDE.md` §"WIT/Type Changes Checklist"; `CLAUDE.md` §"Guest WASM Staleness"; `docs/21_data_defaults_and_fixtures.md` | `crates/slicer-schema/wit/deps/layer-support/layer-support.wit`, `crates/slicer-sdk/src/traits.rs` (`run_support` only), `crates/slicer-macros/src/lib.rs` (`build_layer_support_glue` + native `"run_support"` arm), `crates/slicer-sdk/src/native.rs`, `crates/slicer-wasm-host/src/dispatch.rs` (two lines inside the existing `"Layer::Support"` layer-tier arm), `crates/slicer-wasm-host/src/marshal/native.rs` (`commit_native_layer_response` — its `match` is on `stage_export`, not `stage_id`; its `"Layer::Support" \| "Layer::SupportPostProcess"` arm **must be split** so only the Support half reads the collection, and that split is in scope here and is not "touching `run_support_postprocess`"), `modules/core-modules/tree-support/src/lib.rs`, `modules/core-modules/traditional-support/src/lib.rs`; permitted follow-on if the `run_support` arity change breaks them (list re-derived against the tree — see `implementation-plan.md` Step 5c item 8): both crates' `tests/slicer_module_binding_tdd.rs`, `modules/core-modules/traditional-support/tests/{support_fill_geometry_tdd,traditional_support_tdd,traditional_family_tdd,enforcer_blocker_tdd}.rs` (`support_fill_geometry_tdd.rs` has a **local** `fn run_support` helper building `SupportOutputBuilder::new()`), `modules/core-modules/tree-support/tests/{tree_support_tdd,tree_family_tdd,enforcer_blocker_tdd}.rs`, `crates/slicer-runtime/tests/executor/live_layer_support_tdd.rs`, `crates/slicer-runtime/tests/integration/{traditional_support_family,tree_support_family}.rs`, `crates/slicer-sdk/tests/layer_module_tdd.rs`, `crates/slicer-macros/tests/{slicer_module_tdd,binding_surface_tdd}.rs`; advisory non-break: `crates/pnp-cli/src/module_new.rs`'s `"Layer::Support"` scaffold template string | none | M | The approved resolution of the former `[BLOCK]`: `run` gains `collection: layer-collection-builder`, matching `layer-path-optimization.wit`'s two-builder `run`. Breaking WIT change — one commit. The `set-anchored-event-collection` drain call is **Step 6's**, not this step's. **`xtask/src/wit_verify.rs` is NOT edited and its three `20`→`21` counts are unaffected**: they count `.wit` file paths, not signatures. Shares `TASK-512` — same id, distinct step number |
| `TASK-512` | `Step 5d` | `CLAUDE.md` §"Guest WASM Staleness"; `CLAUDE.md` §"Coordinate System Hazard" | `crates/slicer-wasm-host/test-guests/support-anchored-reach-guest/` (new), `crates/slicer-runtime/tests/executor/support_anchored_reach_tdd.rs` (new) **+ `crates/slicer-runtime/tests/executor/main.rs` (`mod` line — the compile trigger)** | none | M | AC-8: the first test-guest to implement `LayerModule::run_support` under `#[slicer_module]`. **No manifest TOML** — none exists under `test-guests/` and `xtask/src/build_guests.rs` sets `stage_id: None` for `GuestTree::TestGuest`; the **test** supplies `"Layer::Support"` as `LayerStageRunner::run_stage`'s `stage_id` argument and `LoadedModuleBuilder::new`'s `stage` argument. Authored **red**; turns green in Step 6. Asserts content — commit variant, one event, `anchor_global_layer_index == 7`, and anchor Z exactly `1_234_567` units under `assert_eq!` on `i64`. Shares `TASK-512` — same id, distinct step number; no id outside `TASK-508`..`TASK-514` is minted |
| `TASK-513` | `Step 6` | `CLAUDE.md` §"Config Key Naming Convention" | `crates/slicer-sdk/src/layer_collection_builder.rs`, `crates/slicer-sdk/src/native.rs`, `crates/slicer-sdk/src/test_support/capture.rs`, `crates/slicer-macros/src/lib.rs` (drain call on **both** the anchored-events and support legs) | none | M | The step that closes the round trip: AC-1/2/3, AC-8, and AC-N2/N3 all turn green here |
| `TASK-514` | `Step 7` | `docs/02_ir_schemas.md` §`### anchored entity IR (additive)` + §`IR Versioning Contract` | `docs/02_ir_schemas.md`, `docs/07_implementation_status.md`, `docs/specs/support-parity-gap-register.md`, `docs/specs/support-independent-layer-z-split-plan.md` | delegated `FACT`: canonical has no component/serialization boundary for anchored events | S | Extends an existing docs/02 section; no schema-version bump anywhere |

`TASK-509`'s expected code surface additionally includes
`crates/slicer-schema/tests/export_for_stage_id_tdd.rs` (AC-4's authoring home; standalone target,
no `mod` registration needed).

**Activation state:** `design.md` §Open Questions carries **no open `[BLOCK]`**. The former one —
that `set-anchored-event-collection` was not reachable from a `Layer::Support` guest, the consumer
`239c-support-layer-height-producer` needs — is resolved by an approved decision: the
`layer-support` world's `run` gains `collection: layer-collection-builder`, matching
`layer-path-optimization.wit`'s existing two-builder `run`. 239b (Steps 5c/5d, `TASK-512`) owns
the change and the AC-8 proof; 239c records the same resolution. The packet is
`status: implemented` (acceptance ceremony passed 2026-08-30); all rows below are registered in
`docs/07_implementation_status.md`.

Costs are copied from `implementation-plan.md` §Per-Step Budget Roll-Up. Aggregate is `M`; no row
is `L`. If Step 2 measures `L` in practice, split at the boundary recorded in `design.md`
§Risks and Tradeoffs (2a = `.wit` + stage tables; 2b = macro/`build.rs`/`wit_verify`/ADR-0020
gate) and keep both under `TASK-509`.

## Superseded-packet reconciliation

This packet carries `supersedes: 239-support-independent-layer-z` jointly with
`239a-anchored-host-seams` (`TASK-399`..`TASK-408`, independent, implementable in parallel) and
`239c-support-layer-height-producer` (`TASK-515`..`TASK-522`, depends on both 239a and this
packet). Queue row 2 of `docs/specs/support-independent-layer-z-split-plan.md` is this packet's
row and is updated at Step 7. Packet 239 itself moves to `status: superseded` naming all three
successors — that transition is **not** owned here; do not edit
`docs/spec_packets/239-support-independent-layer-z/` from this packet.
