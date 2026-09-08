# Task Map: 239a-anchored-host-seams

Crosswalk for `docs/07_implementation_status.md`, which is the authoritative task-ID identifier
per its own renumbering note. Registration is **packet-owned closure work**: Step 10
(`TASK-408`) appends these rows through a worker dispatch; nothing is registered at authoring
time. Re-derive the tail of `docs/07_implementation_status.md` before appending — the high-water
mark is a mutable ledger fact (it was `TASK-507` when
`docs/specs/support-independent-layer-z-split-plan.md` was written), and this packet's
`TASK-399..TASK-408` range is inherited from the superseded
`239-support-independent-layer-z`, so verify none of those ten ids was re-used elsewhere in the
interim.

| docs/07 task ID | Packet step | Primary docs | Expected code surface | OrcaSlicer refs | Context cost | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| `TASK-399` | `Step 1` | `docs/specs/support-independent-layer-z-split-plan.md` F3 | `crates/slicer-runtime/src/pipeline.rs`, `crates/slicer-runtime/src/run.rs`, `crates/slicer-runtime/tests/common/mod.rs`, `crates/pnp-cli/tests/e2e_integration_tdd.rs`, `crates/slicer-runtime/tests/contract/dispatch_infill_output_tdd.rs` | none | M | Closes F3: additive `PipelineConfig.anchored_entities`. Owns the full blast radius — **6** exhaustive literal sites across 5 files (27 others inherit via the two FRU bases) plus the two exhaustive destructuring patterns in `run_pipeline_with_events` and `run_pipeline_core`. Three of the six are in the `contract` binary (`dispatch_infill_output_tdd.rs`), not `integration`. Not splittable: a partial edit leaves `cargo check --workspace --all-targets` red between steps, so one step, one task ID. Behaviour-neutral; field unread. |
| `TASK-400` | `Step 2` | `docs/21_data_defaults_and_fixtures.md` | `crates/slicer-runtime/tests/common/mod.rs`, `crates/slicer-runtime/tests/integration/pipeline_tdd.rs` | none | S | Proves the missing test capability: no existing mock stores the `&[LayerCollectionIR]` payload (`LayerCountEmitter` keeps `.len()`; `OrderTrackingEmitter` discards it). Also records AC-6's pre-change baseline before any switch exists. |
| `TASK-401` | `Step 3` | `docs/specs/support-families-anchored-entities-plan.md` §6 "same-Z support in ordinary ordering" + numbered item 16 | `crates/slicer-runtime/tests/integration/offgrid_rows_tdd.rs` (new), `crates/slicer-runtime/tests/integration/main.rs` | none | M | Red-first AC-1 / AC-2 / AC-N2 at **pipeline** level. Per F1 these cannot be red at executor level, so an executor-level test would pass vacuously — that is the step's falsifying exit. |
| `TASK-402` | `Step 4` | `docs/specs/support-independent-layer-z-split-plan.md` F1 | `crates/slicer-runtime/src/layer_executor.rs` | none | S | Clarity only. F1 measured `is_same_z_entity`'s positive and negated filters as exact complements, so the partition is already total. Flips no AC; any colour change means the extraction was not equivalent. |
| `TASK-403` | `Step 5` | `docs/specs/support-independent-layer-z-split-plan.md` §Canonical OrcaSlicer reference; `docs/02_ir_schemas.md` (FACT) | `crates/slicer-runtime/src/anchored_rows.rs` (new), `crates/slicer-runtime/src/lib.rs`, `crates/slicer-runtime/tests/integration/offgrid_rows_tdd.rs` | `GCode.cpp` `collect_layers_to_print` | M | AC-5. Pure `Vec<CommittedLayerEvent>` → ordered `Vec<LayerCollectionIR>`; merge iff `\|dz\| <= COORDINATE_TOLERANCE_UNITS`, else the lower Z emits solo, and a solo row adopts the **upper** global layer's index (ADR-0059). Reuses the live `CURRENT_LAYER_COLLECTION_IR_SCHEMA_VERSION` — no bump and **no pinned version literal**; re-read the constant at edit time. |
| `TASK-404` | `Step 6` | `docs/specs/support-independent-layer-z-split-plan.md` F2, F4 | `crates/slicer-runtime/src/pipeline.rs` | none | M | Closes F2/F4 for the instrumented path: `run_pipeline_core` switches to `execute_per_layer_with_committed_anchored_events` and inserts synthesized rows at the finalization seam — the last mutable point before `execute_postpass_with_capture`'s `layer_irs.to_vec()`. AC-1, AC-2, AC-N2 go green. Signature tripwires run as verification. |
| `TASK-405` | `Step 7` | `docs/specs/support-families-anchored-entities-plan.md` §6 "support-disabled emits nothing" | `crates/slicer-runtime/src/pipeline.rs`, `crates/slicer-runtime/tests/integration/offgrid_rows_tdd.rs` | none | S | AC-6 + AC-N3. `run_pipeline_with_events` keeps its own duplicated body by design (bare G-code, no CONFIG_BLOCK wrapper), so the switch lands a second time. Compares against Step 2's committed baseline, never a re-captured one. |
| `TASK-406` | `Step 8` | `docs/specs/support-independent-layer-z-split-plan.md` F2 (third call site) | `crates/pnp-cli/src/visual_debug.rs` | none | S | The call site packet 239 never recorded. Proven by `cargo check --workspace --all-targets`, the visual-debug suite, and a `LOCATIONS` sweep showing zero production callers of the non-anchored variants remain. |
| `TASK-407` | `Step 9` | `docs/specs/support-families-anchored-entities-plan.md` §6 "Z-spanning atomicity" + "serial/parallel determinism"; `docs/adr/0059-support-families-and-anchored-entities.md` | `crates/slicer-runtime/tests/integration/offgrid_rows_tdd.rs` | none | M | AC-3 + AC-4. AC-3 is **executor-level**: `force_parallel` is a positional `bool` of `execute_anchored_event_collections_with_mode`, not a pipeline knob, and none is created here. Its `(z, global_layer_index)` pair comparison also pins the locked index rule for solo synthesized rows (the **upper** anchor layer's index, ADR-0059). AC-4 asserts the Z-spanning block sits **inside its anchor layer's ordinary row**, not on a separate synthesized row. No production edit permitted in this step. |
| `TASK-408` | `Step 10` | `docs/07_implementation_status.md`; `docs/specs/support-independent-layer-z-split-plan.md` §Disposition | `docs/07_implementation_status.md`, `docs/specs/support-parity-gap-register.md`, `docs/specs/support-independent-layer-z-split-plan.md` | none | S | This crosswalk is the verbatim registration source. Re-derive every ledger fact at edit time — next free `G-` row, next free `DEV-` id, docs/07 high-water mark (`G-27`, `DEV-157`, `TASK-507` at split time; all mutable). Documentation only; no fixture-slice or `tmp/` evidence, which belongs to `239c`. |

Costs are copied from `implementation-plan.md` §Per-Step Budget Roll-Up. Aggregate is `M`; no
row is `L`. Split before activation if that changes.

## Supersession crosswalk

This packet is successor #1 of three under `docs/specs/support-independent-layer-z-split-plan.md`
and inherits packet 239's reserved `TASK-399..TASK-408` range. The other two successors mint
fresh ids above the `docs/07_implementation_status.md` high-water mark and are **not** registered
by this packet:

| Successor | Reserved range | Relationship |
| --- | --- | --- |
| `239a-anchored-host-seams` (this packet) | `TASK-399`..`TASK-408` | Host input seam, the three executor call sites, and row synthesis. Depends on nothing. |
| `239b-anchored-wit-contract` | `TASK-508`..`TASK-514` (re-derive) | Independent of this packet; wires the orphaned anchored WIT records (F7) and the SDK drain glue (F6). |
| `239c-support-layer-height-producer` | `TASK-515`..`TASK-522` (re-derive) | Depends on both. Lands the first production `AnchoredEntity` producer (F5) and the support-Z decoupling (F8); owns every fixture-slice artifact and human-validation gate that this packet deliberately excludes. |
