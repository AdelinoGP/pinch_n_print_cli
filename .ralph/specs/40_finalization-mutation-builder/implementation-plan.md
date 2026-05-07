# Implementation Plan: finalization-mutation-builder

## Execution Rules

- One atomic step at a time.
- Each step maps to TASK-171.
- TDD first (Step 1); then role-priority table (Step 2); then builder API (Step 3); then host merge replacement (Step 4); then top-surface-ironing migration (Step 5); then acceptance ceremony (Step 6).
- Each step honors the context-discipline preamble.
- The implementer never reads `OrcaSlicerDocumented/`, `target/`, `Cargo.lock`, or any file > 600 lines in full.
- The packet's load-bearing invariant: **the role-priority table must produce an order equal to producer-emit order under stable-sort**. Step 1's `default_priority_orders_correctly` test is the canary.

## Steps

### Step 0: Discovery — seven FACTs / one LOCATIONS sweep before touching code

- Task IDs: `TASK-171`
- Objective: read-only discovery. Answer the six 🔍 questions in `design.md` plus a packet-39-implemented sanity check.
- Precondition: Step 0 not yet run; Packet 39 acceptance ceremony complete.
- Postcondition: seven returns recorded; implementer makes go/no-go on WIT scope.
- Files allowed to read: none directly (delegate only).
- Files allowed to edit (≤ 3): none.
- Expected sub-agent dispatches:
  - "FACT: read `.ralph/specs/39_stable-entity-ids/packet.spec.md` frontmatter; quote the `status:` line. Confirm Packet 39 is `implemented`. Also FACT-search `docs/07_implementation_status.md` for `TASK-170` (≤ 3 lines around the row); confirm presence."
  - "LOCATIONS: every workspace site with `impl FinalizationModule for` (case-sensitive). Use `rg --type rust 'impl FinalizationModule for'`. Return file:line + module name for each. Also extract each module's stage from its manifest (look in the same module dir for a `.toml` with `[stage]`)."
  - "FACT: locate `FinalizationOutputBuilder` definition. Use ripgrep across `crates/slicer-sdk/`. Return file:line + file size in lines. If size < 300, OK. If > 300, return SUMMARY ≤ 200 words listing impl methods + signatures."
  - "FACT: at the post-Packet-39 finalization-merge site (working hypothesis: `crates/slicer-host/src/dispatch.rs` near line 2877), quote the merge code block ≤ 25 lines. Confirm whether the merge already stamps `entity_id` via `LayerEntityIdGen` (Packet 39 outcome). Cite file:line."
  - "FACT: in `modules/core-modules/top-surface-ironing/src/lib.rs`, quote the existing `output.push_entity_to_layer(...)` call ≤ 5 lines (file:line). Confirm there is exactly one such call site; if more, list each."
  - "FACT: search `wit/`, `crates/slicer-host/src/wit_host.rs`, and `crates/slicer-sdk/wit/` for any reference to `FinalizationOutputBuilder` or its method names. If positive, the WIT boundary is involved and packet scope must expand; report file:line. If negative, return `no WIT exposure`."
  - "FACT: in `docs/05_module_sdk.md`, quote (≤ 10 lines) the existing paragraph about `FinalizationOutputBuilder` mutation/reorder semantics. We need to know if the doc currently makes any claim about ordering."
- Context cost: `S`.
- Authoritative docs: none beyond the dispatches.
- OrcaSlicer refs: none.
- Verification: the seven returns recorded.
- Exit condition: implementer can answer the 🔍 questions; if WIT exposure positive, packet stops and escalates before any code change; if Packet 39 not `implemented`, packet stops and waits.

### Step 1: Author failing TDD tests

- Task IDs: `TASK-171`
- Objective: create three test scopes:
  - `crates/slicer-ir/tests/extrusion_role_priority_tdd.rs` (1 test: `default_priority_orders_correctly` — asserts strict ordering AND ≥ 100 gap between adjacent values).
  - `crates/slicer-sdk/tests/finalization_builder_tdd.rs` (8 tests: `push_with_priority_lands_at_sorted_position`, `modify_entity_by_id_applies_closure`, `sort_layer_by_applies_comparator`, `insert_synthetic_layer_inserts_at_position`, `legacy_push_preserves_prepend`, `modify_entity_unknown_id_errors`, `insert_synthetic_layer_out_of_bounds_errors`, `ties_preserve_insertion_order`).
  - Add new test `benchy_top_surface_precedes_ironing` to `crates/slicer-host/tests/benchy_end_to_end_tdd.rs` (the substantive print-quality assertion).
- Precondition: Step 0 complete; Packet 39 `implemented`; WIT exposure resolved.
- Postcondition: tests authored; targeted runs either compile-fail (acceptable) OR compile-and-fail with expected assertion failures.
- Files allowed to read:
  - `modules/core-modules/skirt-brim/tests/finalization_live_tdd.rs` and `skirt_brim_tdd.rs` (test fixture patterns).
  - `crates/slicer-host/tests/benchy_end_to_end_tdd.rs` lines around `benchy_gcode_contains_ironing_evidence` (small context for the new sibling test).
  - `crates/slicer-sdk/src/builders.rs` (or located path) — symbol search ONLY for the existing `FinalizationOutputBuilder`'s public signature.
- Files allowed to edit (≤ 3):
  - `crates/slicer-ir/tests/extrusion_role_priority_tdd.rs` (new file)
  - `crates/slicer-sdk/tests/finalization_builder_tdd.rs` (new file)
  - `crates/slicer-host/tests/benchy_end_to_end_tdd.rs` (append one new test)
- Expected sub-agent dispatches:
  - "Run `cargo test -p slicer-ir --test extrusion_role_priority_tdd 2>&1 | tail -30`; FACT compile-fail or assertion-fail."
  - "Run `cargo test -p slicer-sdk --test finalization_builder_tdd 2>&1 | tail -40`; FACT compile-fail or assertion-fail."
  - "Run `cargo test -p slicer-host --test benchy_end_to_end_tdd benchy_top_surface_precedes_ironing 2>&1 | tail -20`; FACT compile-fail or assertion-fail."
- Context cost: `M`.
- Authoritative docs: `docs/02_ir_schemas.md` (ExtrusionRole), `docs/05_module_sdk.md`.
- OrcaSlicer refs: none.
- Verification:
  - new tests compile against the (yet-to-be-built) API OR fail-to-compile at the new symbol — acceptable.
  - existing benchy assertion `benchy_gcode_contains_ironing_evidence` still PASSES.
- Exit condition: 10 tests authored across 3 files (1 priority + 8 builder + 1 benchy ordering).

### Step 2: Add `ExtrusionRole::default_priority`

- Task IDs: `TASK-171`
- Objective: implement `pub const fn default_priority(&self) -> u32` on `ExtrusionRole`. Use this draft table (Step 1 may have already adjusted; reconcile):

  | Variant | Priority | Rationale |
  | --- | ---: | --- |
  | `Skirt` | 0 | Frame around object — must print first. Maps from legacy `push_entity_to_layer`. |
  | `OuterWall` | 1000 | Producer emits outer first (per Packet 38-rev1 Step 0 Q5). |
  | `InnerWall` | 1500 | Producer emits inner after outer. |
  | `ThinWall` | 1700 | Between inner walls and infill. |
  | `SparseInfill` | 3000 | First fill type emitted. |
  | `BridgeInfill` | 3500 | Special-case fill before solids. |
  | `BottomSolidInfill` | 4000 | Bottom solid before top solid. |
  | `TopSolidInfill` | 4500 | Top solid surface fill. |
  | `SupportMaterial` | 5000 | Support after object infill. |
  | `SupportInterface` | 5500 | Support interface after support material. |
  | `Ironing` | 6000 | Ironing after surfaces — **the substantive change**. |
  | `WipeTower` | 8000 | Wipe tower per layer. |
  | `PrimeTower` | 8500 | Prime tower per layer. |
  | `Custom(_)` | 9000 | Default for `Custom` is "near end"; modules can override. |

- Precondition: Step 1 complete.
- Postcondition: `cargo test -p slicer-ir --test extrusion_role_priority_tdd default_priority_orders_correctly` PASSES.
- Files allowed to read:
  - `crates/slicer-ir/src/slice_ir.rs` — only the `ExtrusionRole` enum and adjacent ± 20 lines.
- Files allowed to edit (≤ 1):
  - `crates/slicer-ir/src/slice_ir.rs`
- Expected sub-agent dispatches:
  - "Run `cargo build -p slicer-ir`; FACT pass/fail with ≤ 10 lines of error on FAIL."
  - "Run `cargo test -p slicer-ir --test extrusion_role_priority_tdd 2>&1 | tail -20`; FACT pass/fail."
- Context cost: `S`.
- Authoritative docs: `docs/02_ir_schemas.md` (ExtrusionRole).
- OrcaSlicer refs: none.
- Verification:
  - `cargo build -p slicer-ir` PASS.
  - `default_priority_orders_correctly` PASS.
- Exit condition: priority table green; ordering and gap invariants hold.

### Step 3: Builder API — `push_entity_with_priority`, `modify_entity`, `sort_layer_by`, `insert_synthetic_layer_after`

- Task IDs: `TASK-171`
- Objective: extend `FinalizationOutputBuilder` with the four new methods and internal `Vec<MergeOp>` storage. Keep `push_entity_to_layer` as `#[inline]` alias for `push_entity_with_priority(layer, path, region, 0)`. Choose closure-storage shape (`Box<dyn FnOnce>` with `Send` if needed; alternatively an op enum with concrete variants). Each method records a `MergeOp` variant; the builder exposes `pub(crate) fn drain_ops(&mut self) -> Vec<MergeOp>` for the host to consume.
- Precondition: Step 2 complete.
- Postcondition: 8/8 finalization-builder tests PASS.
- Files allowed to read:
  - `crates/slicer-sdk/src/builders.rs` (or located path) — full read if < 300 lines; otherwise SUMMARY-narrowed.
  - `crates/slicer-sdk/src/lib.rs` — re-export check.
  - `crates/slicer-ir/src/slice_ir.rs` — only the new `default_priority` impl.
- Files allowed to edit (≤ 2):
  - `crates/slicer-sdk/src/builders.rs`
  - `crates/slicer-sdk/src/lib.rs` (re-export, if `MergeOp` needs to be public for host consumption)
- Expected sub-agent dispatches:
  - "Run `cargo build -p slicer-sdk`; FACT pass/fail with ≤ 10 lines of error on FAIL."
  - "Run `cargo test -p slicer-sdk --test finalization_builder_tdd 2>&1 | tail -50`; FACT pass/fail per test (8 tests)."
- Context cost: `M`.
- Authoritative docs: `docs/05_module_sdk.md`.
- OrcaSlicer refs: none.
- Verification:
  - `cargo build -p slicer-sdk` PASS.
  - 8/8 finalization-builder tests PASS (positive cases AC-1..AC-4 + AC-8 + negatives NEG-1, NEG-2, NEG-3).
- Exit condition: builder API complete; all builder-level tests green.

### Step 4: Host merge replacement at `dispatch.rs`

- Task IDs: `TASK-171`
- Objective: replace the post-Packet-39 finalization-merge code at the dispatch site with the new merge sequence:
  1. Extend `layer.ordered_entities` with `fin_entities` (each carrying optional explicit priority).
  2. Stamp `entity_id` on every newly-pushed entity via the layer's `LayerEntityIdGen`.
  3. Stable-sort `ordered_entities` by `(effective_priority, original_index_at_post-extend)`. Use `slice::sort_by_key` with the tuple, OR `sort_by` (stable) — the test `ties_preserve_insertion_order` is the canary.
  4. For each `MergeOp::ModifyEntity { layer, entity_id, op }`: lookup the entity by `entity_id`; apply the closure; surface `Err` (return or panic, depending on host's diagnostic convention — Step 4 design choice) if the ID is dangling, with diagnostic naming `entity_id` and the offending value.
  5. For each `MergeOp::SortLayer { layer, key_fn }`: apply the comparator (stable).
  6. After all per-layer merges complete, for each `MergeOp::InsertSynthLayer { idx, new_layer }`: validate `idx <= layers.len()`; surface `Err` with diagnostic naming `synthetic` and the offending `idx` if out of bounds; otherwise `Vec::insert`.
- Precondition: Step 3 complete.
- Postcondition: `cargo build -p slicer-host` PASS; benchy regression PASS unchanged for the existing assertions; new test `benchy_top_surface_precedes_ironing` may still FAIL until Step 5 ships ironing's priority migration.
- Files allowed to read:
  - `crates/slicer-host/src/dispatch.rs` — narrow range (≤ 50 lines around the merge site).
  - `crates/slicer-sdk/src/builders.rs` — to confirm `MergeOp` shape and `drain_ops` signature.
- Files allowed to edit (≤ 1):
  - `crates/slicer-host/src/dispatch.rs`
- Expected sub-agent dispatches:
  - "Run `cargo build -p slicer-host`; FACT pass/fail with ≤ 10 lines of error on FAIL."
  - "Run `cargo test -p slicer-host --test benchy_end_to_end_tdd benchy_gcode_contains_ironing_evidence 2>&1 | tail -20`; FACT pass/fail (regression canary)."
  - "Run `cargo test -p top-surface-ironing --test top_surface_ironing_emission_tdd 2>&1 | tail -30`; FACT pass/fail (regression — should still 8/8)."
- Context cost: `M`.
- Authoritative docs: `docs/04_host_scheduler.md` § 309–317.
- OrcaSlicer refs: none.
- Verification:
  - `cargo build -p slicer-host` PASS.
  - `benchy_gcode_contains_ironing_evidence` PASS unchanged.
  - `top_surface_ironing_emission_tdd` PASS 8/8 (the existing module-level tests are agnostic to priority).
- Exit condition: host merge replaced; existing assertions still green; new ordering test may still fail (depends on Step 5).

### Step 5: Top-surface-ironing migration (one-line change)

- Task IDs: `TASK-171`
- Objective: in `modules/core-modules/top-surface-ironing/src/lib.rs`, change the existing `output.push_entity_to_layer(layer, path, region)` call (per Step 0 FACT 5) to `output.push_entity_with_priority(layer, path, region, ExtrusionRole::Ironing.default_priority())`. Rebuild WASM. Verify Benchy ordering test now PASSES.
- Precondition: Step 4 complete.
- Postcondition: `benchy_top_surface_precedes_ironing` PASSES; module-level tests still PASS 8/8; WASM rebuild clean.
- Files allowed to read:
  - `modules/core-modules/top-surface-ironing/src/lib.rs` — full read (small).
  - `crates/slicer-ir/src/slice_ir.rs` — only the `ExtrusionRole::default_priority` impl.
- Files allowed to edit (≤ 1):
  - `modules/core-modules/top-surface-ironing/src/lib.rs`
- Expected sub-agent dispatches:
  - "Run `cargo build -p top-surface-ironing`; FACT pass/fail."
  - "Run `./modules/core-modules/build-core-modules.sh`; FACT pass/fail with failing module name on fail."
  - "Run `cargo test -p top-surface-ironing --test top_surface_ironing_emission_tdd 2>&1 | tail -30`; FACT pass/fail per test (8 tests; should remain 8/8 PASS)."
  - "Run `cargo test -p slicer-host --test benchy_end_to_end_tdd 2>&1 | tail -40`; FACT pass/fail per test. The new `benchy_top_surface_precedes_ironing` should now PASS."
- Context cost: `S`.
- Authoritative docs: none.
- OrcaSlicer refs: none.
- Verification:
  - `cargo build -p top-surface-ironing` PASS.
  - `./modules/core-modules/build-core-modules.sh` PASS.
  - `top_surface_ironing_emission_tdd` 8/8 PASS.
  - `benchy_top_surface_precedes_ironing` PASS (the substantive print-quality fix).
  - `benchy_gcode_contains_ironing_evidence` PASS unchanged.
- Exit condition: ironing emits AFTER top-fill in benchy output; print-quality fix verified.

### Step 6: Acceptance ceremony + docs/07 row

- Task IDs: `TASK-171`
- Objective: re-run every acceptance command from `packet.spec.md`; run workspace gates; insert `TASK-171` row.
- Precondition: Step 5 complete.
- Postcondition: every AC PASSES; backlog updated; workspace closure gate PASSES; clippy clean.
- Files allowed to read: none directly (dispatch only).
- Files allowed to edit (≤ 1):
  - `docs/07_implementation_status.md` (delegate insertion via worker).
- Expected sub-agent dispatches:
  - 11 narrow AC commands from `packet.spec.md` `## Acceptance Criteria` (8) and `## Negative Test Cases` (3), each as a separate FACT pass/fail.
  - "Run `cargo test --workspace --no-fail-fast 2>&1 | tail -40`; FACT pass/fail with failing test list (≤ 20 lines)."
  - "Run `cargo clippy --workspace -- -D warnings 2>&1 | tail -20`; FACT pass/fail."
  - "Run `./modules/core-modules/build-core-modules.sh`; FACT pass/fail."
  - "Insert a TASK-171 row into `docs/07_implementation_status.md` describing this packet's deliverable. Return the inserted line as FACT (file:line, contents). Do NOT load the whole file."
- Context cost: `S`.
- Authoritative docs: `docs/07_implementation_status.md` (delegate-only).
- OrcaSlicer refs: none.
- Verification: every pipe-suffixed AC command from `packet.spec.md`.
- Exit condition: every AC PASSES; `cargo test --workspace` PASSES; `cargo clippy --workspace -- -D warnings` PASSES; `docs/07` carries TASK-171; packet ready to move to `status: implemented`.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 0 | S | Seven FACT/LOCATIONS dispatches. |
| Step 1 | M | TDD authoring (10 tests across 3 files). |
| Step 2 | S | Role priority table. |
| Step 3 | M | Builder API + 8 tests. |
| Step 4 | M | Host merge replacement. |
| Step 5 | S | One-line module migration + WASM rebuild. |
| Step 6 | S | Acceptance + docs row insertion. |

Aggregate: `M`. No single step is `L`.

## Packet Completion Gate

- All steps complete.
- Every AC verification command from `packet.spec.md` PASSES (8 AC + 3 negatives = 11 commands).
- `cargo test --workspace` PASSES.
- `cargo clippy --workspace -- -D warnings` PASSES.
- `./modules/core-modules/build-core-modules.sh` PASSES.
- `cargo test -p slicer-host --test benchy_end_to_end_tdd benchy_top_surface_precedes_ironing` PASSES (the substantive print-quality fix).
- `cargo test -p slicer-host --test benchy_end_to_end_tdd benchy_gcode_contains_ironing_evidence` PASSES unchanged (regression canary).
- `cargo test -p top-surface-ironing --test top_surface_ironing_emission_tdd` PASSES 8/8 unchanged (regression canary).
- `docs/07_implementation_status.md` carries TASK-171.
- `packet.spec.md` ready to move to `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed AC command (11 commands).
- Confirm `cargo test --workspace`, `cargo clippy --workspace -- -D warnings`, and `./modules/core-modules/build-core-modules.sh` PASS.
- Confirm benchy ordering fix (AC-6) and benchy presence regression (AC-7) both PASS.
- Confirm skirt-brim regression (AC-8) PASSES.
- Confirm top-surface-ironing module-level tests (8/8) PASS unchanged.
- Confirm implementer's peak context usage stayed under 70%.
