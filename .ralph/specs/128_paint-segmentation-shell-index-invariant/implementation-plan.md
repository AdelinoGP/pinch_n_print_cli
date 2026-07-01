# Implementation Plan — Packet 128: Paint-Segmentation Shell-Index Invariant

## Execution Rules

- One atomic step at a time.
- Each step must map back to the packet's grouped task IDs (`TASK-253`).
- TDD first, then implementation, then the narrowest falsifying validation. Exception: Step 1 (the propagation refactor) precedes the tests because the multi-object test asserts against the NEW per-object behaviour and would red against the old per-layer-global code for the wrong reason (fixture has two objects, not invariant helper wrong). Step 1 lands the refactor + invariant helper; Steps 2–3 add the tests that lock it.
- Each step honors the context-discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. The fields below are the budget contract for this step.

## Steps

### Step 1: Refactor propagation scope to per-object + invariant helper + debug_assert + doc comment

- Task IDs:
  - `TASK-253`
- Objective: Replace the per-layer-global `saved_top_idx` / `saved_bottom_idx` scalar accumulator at mod.rs:887-916 with a per-`ObjectId` accumulator (HashMap or BTreeMap — see [FWD] open question in design.md). Update the stamping at 912-913 to look up by `new_region.object_id`. Update the Phase 6/7 None arm at 1252-1296 to look up by the new region's `object_id` (removing the `working[l].regions` sibling-query). Add a `fn assert_per_object_shell_index_invariant` helper (or inline closure) at the end of `execute_paint_segmentation` (before `Ok(Arc::new(working))` at line 1333) that groups `working[l].regions` by `object_id` and `debug_assert!`s within-group agreement on `top_shell_index` and `bottom_shell_index`, gated `#[cfg(debug_assertions)]`. Add an `// INVARIANT:` doc comment at mod.rs:887 stating the per-object scope and that cross-object disagreement is legal.
- Precondition: `cargo check --workspace --all-targets` passes on the baseline tree (confirm via dispatch before starting).
- Postcondition: `execute_paint_segmentation` compiles; the propagation block groups by `ObjectId`; the None arm consumes per-object state; the debug_assert helper is defined; the doc comment is present.
- Files allowed to read (with line-range hints when > 300 lines):
  - `crates/slicer-core/src/algos/paint_segmentation/mod.rs` — lines 600-916, 1252-1333 (NOT the full file)
  - `crates/slicer-ir/src/slice_ir.rs` — lines 1226-1300 (SlicedRegion struct + field types)
- Files allowed to edit (≤ 3):
  - `crates/slicer-core/src/algos/paint_segmentation/mod.rs`
- Files explicitly out-of-bounds for this step:
  - `crates/slicer-core/src/algos/slice_postprocess_prepass.rs` (read-only confirm only; do NOT edit the producer)
  - `CONTEXT.md`, `docs/07_implementation_status.md` (doc writes are Step 4)
  - Any guest WASM path
- Expected sub-agent dispatches:
  - "Run `cargo check --workspace --all-targets`; return FACT pass/fail + the first error if fail" — purpose: baseline confirm + post-edit compile.
  - "Check whether `ObjectId` implements `Hash`+`Eq`; return FACT (yes/no, with the impl location file:line)" — purpose: resolve the [FWD] open question; choose HashMap vs BTreeMap.
  - "Run `rg -n 'HashMap<.*ObjectId>|saved_top_idx.*insert|saved_top_idx\.get' crates/slicer-core/src/algos/paint_segmentation/mod.rs | head -10`; return LOCATIONS" — purpose: AC-2.
  - "Run `rg -n 'working\[.*\]\.regions.*(top|bottom)_shell_index' crates/slicer-core/src/algos/paint_segmentation/mod.rs`; return FACT (0 matches in 1252-1296) or LOCATIONS" — purpose: AC-3.
- Context cost: `M` (the propagation block, None arm, debug_assert, and doc comment are one cohesive refactor; reading two line ranges of one large file + one small struct definition).
- Authoritative docs:
  - `docs/02_ir_schemas.md` — delegate the `SlicedRegion` section only; confirm field names match the code.
- OrcaSlicer refs:
  - None.
- Verification:
  - `cargo check --workspace --all-targets` — dispatch as FACT pass/fail.
  - `rg -n "HashMap<.*ObjectId>|saved_top_idx.*insert|saved_top_idx\.get" crates/slicer-core/src/algos/paint_segmentation/mod.rs | head -10` — LOCATIONS, expect ≥ 1.
  - `rg -n "working\[.*\]\.regions.*(top|bottom)_shell_index" crates/slicer-core/src/algos/paint_segmentation/mod.rs` — FACT 0 matches in the 1252-1296 range.
- Exit condition: compiles; AC-2 and AC-3 greps return the expected shape; doc comment present at mod.rs:887.

### Step 2: Add the multi-object mixed-height invariant test

- Task IDs:
  - `TASK-253`
- Objective: Add `shell_index_invariant_multi_object` to the inline `#[cfg(test)] mod tests` in mod.rs. Build a two-object `LayerPlanIR` fixture (a 10 mm cube "objA" and a 50 mm cube "objB" on one build plate). Run `execute_paint_segmentation`. Assert that at a layer near objA's top (where objA's region is `Some(0)` and objB's region is `None`), the returned `SliceIR` preserves that distinction — objB's regions are NOT stamped `Some(0)`. Model the fixture construction on the existing `phase6_7_none_arm_stamps_shell_index_on_new_region` test (mod.rs:2391-2581) but with a second object.
- Precondition: Step 1 complete (propagation block is per-object; invariant helper exists).
- Postcondition: `shell_index_invariant_multi_object` passes; it would fail against the old per-layer-global code (the test no existing fixture exercises).
- Files allowed to read (with line-range hints when > 300 lines):
  - `crates/slicer-core/src/algos/paint_segmentation/mod.rs` — lines 2391-2581 (existing test to model on)
  - `crates/slicer-ir/src/slice_ir.rs` — lines 1226-1300 (SlicedRegion fields for fixture construction)
- Files allowed to edit (≤ 3):
  - `crates/slicer-core/src/algos/paint_segmentation/mod.rs` (add the test fn to the existing inline test module)
- Files explicitly out-of-bounds for this step:
  - Any file other than mod.rs (the fixture is inline; do not create a separate test file — match the existing inline-test convention confirmed during grilling).
- Expected sub-agent dispatches:
  - "Run `cargo test -p slicer-core --lib paint_segmentation::tests::shell_index_invariant_multi_object --nocapture`; return FACT pass, or SNIPPETS fail with the per-object `top_shell_index` values from the fixture (≤ 20 lines)" — purpose: AC-1.
- Context cost: `S` (read one test to model on + one small struct def; write one test fn).
- Authoritative docs:
  - `docs/02_ir_schemas.md` — delegate if the fixture needs an IR field the struct-def read didn't surface.
- OrcaSlicer refs:
  - None.
- Verification:
  - `cargo test -p slicer-core --lib paint_segmentation::tests::shell_index_invariant_multi_object --nocapture` — FACT pass/fail; SNIPPETS on fail with the per-object `top_shell_index` values.
- Exit condition: test passes; the fixture demonstrably has two objects with distinct shell-index values at the assertion layer (the SNIPPETS-on-fail return format forces this to be visible if it reds).

### Step 3: Add single-object multi-colour test + debug_assert negative tests

- Task IDs:
  - `TASK-253`
- Objective: Add three test fns to the inline `#[cfg(test)] mod tests` in mod.rs:
  1. `shell_index_invariant_multi_color` — single-object 3-colour partial-paint `LayerPlanIR`; run `execute_paint_segmentation`; assert every region on every layer shares the same `top_shell_index` and `bottom_shell_index` (degenerate single-object case of the per-object invariant).
  2. `shell_index_invariant_assert_fires` — under `#[cfg(debug_assertions)]`, build a hand-constructed `SliceIR` with two regions of the SAME `object_id` on one layer having mismatched `top_shell_index` (`Some(0)` vs `Some(2)`); call the invariant helper via `std::panic::catch_unwind`; assert it panicked. Follow `colorize.rs:652-654` precedent.
  3. `shell_index_invariant_cross_object_legal` — under `#[cfg(debug_assertions)]`, build a `SliceIR` with two regions of DIFFERENT `object_id` on one layer with different `top_shell_index` (`Some(0)` vs `None`); call the invariant helper; assert it did NOT panic (the guard against re-introducing the wrong per-layer-global invariant).
- Precondition: Step 1 complete (invariant helper exists). Step 2 not strictly required but recommended to land first so the multi-object fixture pattern is established.
- Postcondition: all three tests pass; AC-4, AC-N1, AC-N2 green.
- Files allowed to read (with line-range hints when > 300 lines):
  - `crates/slicer-core/src/algos/paint_segmentation/mod.rs` — lines 2391-2581 (existing test pattern), 1706-1823 (simpler tests for multi-colour fixture shapes)
  - `crates/slicer-core/src/algos/paint_segmentation/colorize.rs` — lines 640-670 (debug_assert-fires test precedent)
- Files allowed to edit (≤ 3):
  - `crates/slicer-core/src/algos/paint_segmentation/mod.rs` (add three test fns)
- Files explicitly out-of-bounds for this step:
  - `colorize.rs` (read-only precedent; do NOT edit)
- Expected sub-agent dispatches:
  - "Run `cargo test -p slicer-core --lib paint_segmentation::tests::shell_index_invariant_multi_color --nocapture`; return FACT pass/fail" — purpose: AC-4.
  - "Run `cargo test -p slicer-core --lib paint_segmentation::tests::shell_index_invariant_assert_fires --nocapture`; return FACT pass/fail" — purpose: AC-N1.
  - "Run `cargo test -p slicer-core --lib paint_segmentation::tests::shell_index_invariant_cross_object_legal --nocapture`; return FACT pass/fail" — purpose: AC-N2.
- Context cost: `S` (three small inline tests; one precedent read in colorize.rs).
- Authoritative docs:
  - None new (the IR fields are already confirmed).
- OrcaSlicer refs:
  - None.
- Verification:
  - `cargo test -p slicer-core --lib paint_segmentation::tests::shell_index_invariant_multi_color --nocapture` — FACT pass/fail.
  - `cargo test -p slicer-core --lib paint_segmentation::tests::shell_index_invariant_assert_fires --nocapture` — FACT pass/fail.
  - `cargo test -p slicer-core --lib paint_segmentation::tests::shell_index_invariant_cross_object_legal --nocapture` — FACT pass/fail.
- Exit condition: all three tests pass; the cross-object legal test is the critical one — if it reds, the invariant helper is wrongly per-layer-global and Step 1 must be revisited.

### Step 4: Doc writes (CONTEXT.md Shell depth entry + docs/07 TASK-253 row) + gate

- Task IDs:
  - `TASK-253`
- Objective: Append the **Shell depth** glossary entry to `CONTEXT.md` §Terms: *"Depth, in layers, of a region within its owning object's top or bottom shell zone. `0` = exposed surface; `None` = outside any shell zone of that object. A property of a region of an object, computed per-object — not shared across objects on a layer."* Append the `TASK-253` row to `docs/07_implementation_status.md`: `TASK-253 | [ ] | Paint-segmentation shell-depth per-object propagation (packet 128) — scope propagation block by object_id, per-object debug_assert, multi-object mixed-height invariant test, propagation-block doc contract`. Then run the full gate.
- Precondition: Steps 1–3 complete; all targeted tests green.
- Postcondition: `CONTEXT.md` and `docs/07` updated; both Doc Impact greps return exit 0; clippy clean; the 4-colour e2e gate green.
- Files allowed to read (with line-range hints when > 300 lines):
  - `CONTEXT.md` — full file (164 lines; small enough to load)
  - `docs/07_implementation_status.md` — delegate the append; do NOT load the full backlog. The dispatch appends the TASK-253 row at the correct position.
- Files allowed to edit (≤ 3):
  - `CONTEXT.md`
  - `docs/07_implementation_status.md` (via worker dispatch — the implementer triggers the append, not hand-edits the full file)
- Files explicitly out-of-bounds for this step:
  - `mod.rs` (code changes are Steps 1–3; this step is docs + gate only)
- Expected sub-agent dispatches:
  - "Append the line `TASK-253 | [ ] | Paint-segmentation shell-depth per-object propagation (packet 128) — scope propagation block by object_id, per-object debug_assert, multi-object mixed-height invariant test, propagation-block doc contract` to `docs/07_implementation_status.md` at the position after the TASK-252 row; return FACT done" — purpose: docs/07 mutation without loading the backlog.
  - "Run `cargo clippy --workspace --all-targets -- -D warnings`; return FACT pass/fail" — purpose: AC-N3.
  - "Run `cargo test -p slicer-runtime --test executor -- cube_4color_ironing_per_painted_top_color_tdd --nocapture`; return FACT pass/fail" — purpose: AC-N4.
  - "Run `rg -q '### Shell depth' CONTEXT.md`; return FACT exit 0" — purpose: Doc Impact grep.
  - "Run `rg -q 'TASK-253' docs/07_implementation_status.md`; return FACT exit 0" — purpose: Doc Impact grep.
- Context cost: `S` (two small doc edits via dispatch + gate runs).
- Authoritative docs:
  - `CONTEXT.md` — load directly (small file).
  - `docs/07_implementation_status.md` — delegate the edit.
- OrcaSlicer refs:
  - None.
- Verification:
  - `rg -q '### Shell depth' CONTEXT.md` — FACT exit 0.
  - `rg -q 'TASK-253' docs/07_implementation_status.md` — FACT exit 0.
  - `cargo clippy --workspace --all-targets -- -D warnings` — FACT pass/fail.
  - `cargo test -p slicer-runtime --test executor -- cube_4color_ironing_per_painted_top_color_tdd --nocapture` — FACT pass/fail.
- Exit condition: both Doc Impact greps exit 0; clippy clean; e2e gate green; packet ready for acceptance ceremony.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | M | Propagation block + None arm + debug_assert + invariant helper + doc comment in one cohesive refactor; largest step |
| Step 2 | S | One multi-object test; models on existing fixture pattern |
| Step 3 | S | Three inline tests (one positive, two debug_assert negative) |
| Step 4 | S | Two doc writes via dispatch + gate runs |

Aggregate: M (sum = M + S + S + S, which stays under L). No step is L. Packet is activatable.

## Packet Completion Gate

- All steps complete.
- Every step exit condition is met.
- Packet acceptance criteria green (each verification command dispatched and returned PASS): AC-1, AC-2, AC-3, AC-4, AC-N1, AC-N2, AC-N3, AC-N4.
- Doc Impact greps green: `rg -q '### Shell depth' CONTEXT.md` and `rg -q 'TASK-253' docs/07_implementation_status.md`.
- `docs/07_implementation_status.md` updated for TASK-253 (via worker dispatch — never edited by loading the full backlog into the implementer's context).
- `packet.spec.md` ready to move to `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed acceptance criterion command from `packet.spec.md` (AC-1 through AC-N4).
- Re-dispatch the two Doc Impact greps.
- Confirm packet-level verification commands are green (`cargo check --workspace --all-targets`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test -p slicer-core --lib paint_segmentation::tests::shell_index_invariant_multi_object --nocapture`).
- Record any remaining packet-local risk explicitly before moving to `status: implemented` (the deferred ADR on the harmonisation-scope decision is the known follow-up; the `ObjectId` Hash/Eq resolution is recorded in design.md Open Questions as [FWD]).
- Confirm the implementer's peak context usage stayed under 70%; if not, log it as a packet-authoring lesson for future spec-packet-generator runs.