# Implementation Plan: 244-order-locked-extrusion-sequences

## Execution Rules

- Work one atomic step at a time; map every step to grouped task IDs.
- Use TDD, then implementation, then the narrowest falsifying validation.
- Every field below is a context-budget contract and must be filled independently; never write "see Step 1".

## Steps

### Step 1: Carrier + WIT + projection + schema bump

- Task IDs: `TASK-354`
- Objective: Add `ExtrusionPath3D.order_lock: Option<u64>` (`#[serde(default)]`), the two WIT
  `order-lock: option<u64>` records, the three `OrderedEntityView` projections (host, wasm-host,
  SDK), the macro adapter mapping, the WIT→IR converter fields, and bump
  `CURRENT_LAYER_COLLECTION_IR_SCHEMA_VERSION` to `1.4.0`.
- Precondition: tree compiles at `1.3.0`; `cargo check -p slicer-ir` is green.
- Postcondition: the field and WIT records exist end-to-end; the constant is `1.4.0`; the tree
  compiles with `cargo check --workspace --all-targets` (all exhaustive `src/` literals gained
  `order_lock: None`; test literals use FRU/waiver).
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-ir/src/slice_ir.rs` - lines 2335-2470 and 330-345
  - `crates/slicer-schema/wit/deps/types.wit` - lines 1-25
  - `crates/slicer-schema/wit/deps/ir-types.wit` - lines 255-275
  - `crates/slicer-runtime/src/layer_executor.rs` - lines 2330-2400
  - `crates/slicer-wasm-host/src/dispatch.rs` - lines 2410-2450
  - `crates/slicer-sdk/src/views.rs` - lines 805-825
  - `crates/slicer-macros/src/lib.rs` - lines 1310-1340, 2650-2690, 3185-3205
- Files allowed to edit (at most 3 primary — `slice_ir.rs`, `types.wit`, `ir-types.wit`; the
  remaining files are mechanical: projections, converters, literal sites):
  - `crates/slicer-ir/src/slice_ir.rs`
  - `crates/slicer-schema/wit/deps/types.wit`
  - `crates/slicer-schema/wit/deps/ir-types.wit`
  - `crates/slicer-runtime/src/layer_executor.rs`
  - `crates/slicer-runtime/src/visual_debug_render.rs`
  - `crates/slicer-wasm-host/src/dispatch.rs`
  - `crates/slicer-sdk/src/views.rs`
  - `crates/slicer-macros/src/lib.rs`
  - `crates/slicer-wasm-host/src/marshal/leaf.rs`
  - `crates/slicer-wasm-host/src/host.rs`
  - `crates/slicer-core/src/perimeter_utils.rs`
  - `crates/slicer-gcode/src/emit.rs`
  - `crates/slicer-sdk/src/test_support/fixtures.rs`
  - `modules/core-modules/wipe-tower/src/lib.rs`
  - `modules/core-modules/tree-support/src/lib.rs`
- Files explicitly out of bounds:
  - `modules/core-modules/infill-linker/**`, `modules/core-modules/path-optimization-default/**`
    (Packet 3)
  - `docs/**` (Step 4)
- Blast-radius discipline (mandatory when adding a new struct field or schema constant):
  - This step adds a field to `ExtrusionPath3D` (a `pub` struct under `crates/*/src`, becoming a
    **watched type** at 5 fields) and bumps `CURRENT_LAYER_COLLECTION_IR_SCHEMA_VERSION`. The
    **struct-literal blast radius** is every exhaustive `ExtrusionPath3D { … }` literal (100+ sites
    across `crates/*` and `modules/*`): production `src/` literals gain `order_lock: None`
    (compiler-enforced, exhaustive is correct in `src/`); test literals use a `..` rest (fixture
    base `slicer_sdk::test_support::extrusion_path3d_base(role)`) or an `// exhaustive:` waiver
    (`cargo xtask check-literals`-enforced). The **test-assertion fallout** for the constant is
    empty — no test hard-asserts the literal `1.3.0` for this constant (constant-sourced only).
  - Dispatch a `LOCATIONS` worker for the production-literal sites before authoring this step; cite
    the result inline below.
- Expected sub-agent dispatches:
  - Question: list every exhaustive `ExtrusionPath3D { … }` literal (no `..` rest) in `crates/*/src`
    and `modules/*/src`; scope: `crates/**/src/**/*.rs`, `modules/**/src/**/*.rs`; return:
    `LOCATIONS` (≤ 20 entries)
  - Question: confirm no test hard-asserts `SemVer { major: 1, minor: 3, patch: 0 }` for
    `CURRENT_LAYER_COLLECTION_IR_SCHEMA_VERSION`; scope: `crates/**/tests/**/*.rs`; return:
    `LOCATIONS`
- Context cost: `M`
- Authoritative docs:
  - `docs/02_ir_schemas.md` - §"IR Versioning Contract" (lines ~1633-1641) — additive field is minor
  - `docs/21_data_defaults_and_fixtures.md` - §3 watchlist + §4 waiver format
- OrcaSlicer refs:
  - none
- Verification:
  - `cargo check --workspace --all-targets` - FACT pass/fail
  - `cargo xtask check-literals` - exit code (watched-type literal gate)
  - `cargo xtask build-guests` - rebuild guests after the WIT change (drop `--check`)
  - `cargo xtask build-guests --check` - exit 0 before attributing any failure
- Exit condition: `cargo check --workspace --all-targets` green, `check-literals` exit 0,
  `build-guests --check` exit 0.

### Step 2: SDK allocator + host remap

- Task IDs: `TASK-354`
- Objective: Add `OrderLockAllocator` (SDK) and `remap_order_locks_to_global` (host), with unit
  tests for the allocator sequence and the remap (local→global, `Some(0)` rejection, unknown-global
  rejection).
- Precondition: Step 1 landed (field exists).
- Postcondition: `OrderLockAllocator::allocate()` returns `Some(1)`, `Some(2)`, … and `None` on
  exhaustion; `remap_order_locks_to_global` rewrites local tags to layer-unique global tags and
  rejects `Some(0)` / unknown globals.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-sdk/src/lib.rs` - lines 1-60 (module declarations)
  - `crates/slicer-runtime/src/layer_executor.rs` - lines 1-60 (module declarations)
- Files allowed to edit (at most 3 primary — `crates/slicer-sdk/src/order_lock.rs` (new),
  `crates/slicer-sdk/src/lib.rs` (register the module), `crates/slicer-runtime/src/layer_executor.rs`
  (or a new `crates/slicer-runtime/src/order_lock.rs`); the remaining files are mechanical: test
  files):
  - `crates/slicer-sdk/src/order_lock.rs` (new)
  - `crates/slicer-sdk/src/lib.rs` (register the module)
  - `crates/slicer-runtime/src/layer_executor.rs` (or a new `crates/slicer-runtime/src/order_lock.rs`)
  - `crates/slicer-sdk/tests/finalization_builder_tdd.rs` (allocator test)
  - `crates/slicer-runtime/tests/unit/layer_collection_builder_tdd.rs` (remap test)
- Files explicitly out of bounds:
  - `modules/**`, `docs/**`
- Blast-radius discipline: not applicable (new types/functions, no existing struct/constant changed).
- Expected sub-agent dispatches:
  - none
- Context cost: `S`
- Authoritative docs:
  - `docs/specs/wave-overhangs-bridge-fill-plan.md` - §"Namespace and allocation" (D11)
- OrcaSlicer refs:
  - none
- Verification:
  - `cargo test -p slicer-sdk --test finalization_builder_tdd -- order_lock_allocator_sequence --exact 2>&1 | tee target/test-output.log | grep -qE "^test result: ok\. 1 passed"` - FACT pass/fail
  - `cargo test -p slicer-runtime --test unit -- order_lock_remap_local_to_global --exact 2>&1 | tee target/test-output.log | grep -qE "^test result: ok\. 1 passed"` - FACT pass/fail
- Exit condition: both tests pass.

### Step 3: Host enforcement at the four mutation points

- Task IDs: `TASK-354`
- Objective: Add lock validation at InfillPostProcess commit, `apply_entity_order_proposal`,
  finalization `apply_to` (`modify_entity` / `sort_layer_by`), and
  `apply_cross_layer_tool_rotation` (locked blocks move as units — the rotation range extends to the
  block boundary, reading `entity.path.order_lock` on `PrintEntity`); wire
  `remap_order_locks_to_global` at the output boundaries (the `LayerStageCommit::Infill` /
  `LayerStageCommit::InfillPostProcess` commit arms of `apply` in `layer_executor.rs`, and the finalization
  merge in `apply_to` in `traits.rs`); add the `LayerStageError::OrderLockViolation` variant; add the
  all-`None` neutrality test, the enforcement tests, and the remap-wiring tests.
- Precondition: Steps 1-2 landed.
- Postcondition: a proposal that splits/interleaves/reverses/reorders a locked block returns `Err`;
  an InfillPostProcess replacement that drops/alters a locked block returns
  `LayerStageError::OrderLockViolation`; a finalization `modify_entity`/`sort_layer_by` that changes
  locked geometry/order returns `Err`; tool rotation never splits a locked block; the remap is wired
  at the output boundaries (local tags become layer-unique global tags in committed output); all-`None`
  slices are byte-identical to today.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-runtime/src/layer_executor.rs` - lines 2687-2756 and 3000-3060
  - `crates/slicer-sdk/src/traits.rs` - lines 1338-1560
  - `crates/slicer-gcode/src/emit.rs` - lines 942-1000 and 1024-1240 (inline `#[cfg(test)]` module)
  - `crates/slicer-ir/src/stage_io.rs` - lines 82-115
- Files allowed to edit (at most 3 primary — `crates/slicer-runtime/src/layer_executor.rs`,
  `crates/slicer-sdk/src/traits.rs`, `crates/slicer-gcode/src/emit.rs`; the remaining files are
  mechanical: error variant, test files, aggregator):
  - `crates/slicer-runtime/src/layer_executor.rs`
  - `crates/slicer-sdk/src/traits.rs`
  - `crates/slicer-gcode/src/emit.rs`
  - `crates/slicer-ir/src/stage_io.rs`
  - `crates/slicer-runtime/tests/unit/layer_collection_builder_tdd.rs`
  - `crates/slicer-sdk/tests/finalization_builder_tdd.rs`
  - `crates/slicer-runtime/tests/executor/order_lock_tdd.rs` (new — InfillPostProcess enforcement,
    all-`None` neutrality, remap-wiring tests)
  - `crates/slicer-runtime/tests/executor/main.rs` (aggregator: `mod order_lock_tdd;`)
- Files explicitly out of bounds:
  - `modules/**`, `docs/**`
- Blast-radius discipline: not applicable (no new field/constant; this step adds validation and
  tests).
- Expected sub-agent dispatches:
  - none
- Context cost: `M`
- Authoritative docs:
  - `docs/specs/wave-overhangs-bridge-fill-plan.md` - §"Enforcement (host-enforced invariant)"
- OrcaSlicer refs:
  - none
- Verification:
  - `cargo test -p slicer-runtime --test unit -- order_lock_proposal_split_rejected --exact 2>&1 | tee target/test-output.log | grep -qE "^test result: ok\. 1 passed"` - FACT pass/fail
  - `cargo test -p slicer-runtime --test executor -- order_lock_infill_postprocess_preserves_block --exact 2>&1 | tee target/test-output.log | grep -qE "^test result: ok\. 1 passed"` - FACT pass/fail
  - `cargo test -p slicer-sdk --test finalization_builder_tdd -- order_lock_finalization_rejects_geometry_change --exact 2>&1 | tee target/test-output.log | grep -qE "^test result: ok\. 1 passed"` - FACT pass/fail
  - `cargo test -p slicer-runtime --test executor -- order_lock_all_none_neutrality --exact 2>&1 | tee target/test-output.log | grep -qE "^test result: ok\. 1 passed"` - FACT pass/fail
  - `cargo test -p slicer-gcode --lib -- order_lock_tool_rotation_preserves_block --exact 2>&1 | tee target/test-output.log | grep -qE "^test result: ok\. 1 passed"` - FACT pass/fail
  - `cargo test -p slicer-runtime --test executor -- order_lock_remap_wired_at_output_boundary --exact 2>&1 | tee target/test-output.log | grep -qE "^test result: ok\. 1 passed"` - FACT pass/fail
  - `cargo test -p slicer-sdk --test finalization_builder_tdd -- order_lock_remap_wired_at_finalization_merge --exact 2>&1 | tee target/test-output.log | grep -qE "^test result: ok\. 1 passed"` - FACT pass/fail
  - `cargo check --workspace --all-targets` - FACT pass/fail
- Exit condition: all seven tests pass and `cargo check --workspace --all-targets` is green.

### Step 4: Docs (ADR + glossary + IR schema)

- Task IDs: `TASK-354`
- Objective: Land ADR-0062, the two `CONTEXT.md` glossary terms, and the
  `docs/02_ir_schemas.md` §"IR 10 — LayerCollectionIR" update.
- Precondition: Steps 1-3 landed; all tests green.
- Postcondition: `docs/adr/0062-order-lock-for-print-order-sensitive-extrusion-sequences.md` exists
  (content from the plan's Appendix A draft, number re-derived to 0062); `CONTEXT.md` carries the
  "Order lock" and "Anchor band" terms; `docs/02_ir_schemas.md` reads `Current schema_version: 1.4.0`
  and documents `order_lock`.
- Files allowed to read, with ranges when over 300 lines:
  - `docs/specs/wave-overhangs-bridge-fill-plan.md` - Appendix A (lines ~373-420) and Appendix B
    (lines ~452-473)
  - `docs/02_ir_schemas.md` - lines 1185-1195 only
  - `CONTEXT.md` - lines 260-300 only (infill/fill cluster)
- Files allowed to edit (at most 3):
  - `docs/adr/0062-order-lock-for-print-order-sensitive-extrusion-sequences.md` (new)
  - `CONTEXT.md`
  - `docs/02_ir_schemas.md`
- Files explicitly out of bounds:
  - `docs/07_implementation_status.md` (orchestrator-owned)
  - `docs/specs/wave-overhangs-bridge-fill-plan.md` (orchestrator-owned)
- Blast-radius discipline: not applicable.
- Expected sub-agent dispatches:
  - none
- Context cost: `S`
- Authoritative docs:
  - `docs/specs/wave-overhangs-bridge-fill-plan.md` - Appendix A + B (verbatim content source)
- OrcaSlicer refs:
  - none
- Verification:
  - `rg -q '^# ADR-0062' docs/adr/0062-order-lock-for-print-order-sensitive-extrusion-sequences.md && echo P244_ADR_LANDED` - FACT pass/fail
  - `rg -q '^### Order lock' CONTEXT.md && rg -q '^### Anchor band' CONTEXT.md && echo P244_GLOSSARY_LANDED` - FACT pass/fail
  - `rg -q 'Current schema_version: 1\.4\.0' docs/02_ir_schemas.md && rg -q 'order_lock' docs/02_ir_schemas.md && echo P244_IR_DOCS_UPDATED` - FACT pass/fail
- Exit condition: all three greps match.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | M | carrier + WIT + projection + schema bump; owns the literal blast radius |
| Step 2 | S | allocator + remap (new types/functions) |
| Step 3 | M | four enforcement points + tests |
| Step 4 | S | ADR + glossary + IR docs |

## Packet Completion Gate

- All steps and exits complete.
- Every pipe-suffixed AC command returns PASS.
- Update `docs/07_implementation_status.md` through a worker dispatch, never a full backlog read.
- Reconcile reopened/superseded status transitions (none for this packet).
- `packet.spec.md` is ready for `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed AC and packet-level gate command.
- Record remaining packet-local risk.
- Confirm context stayed at or below 150k standard, or at/below 300k only with a logged swarm ESCALATION; otherwise record a packet-authoring lesson.

All `cargo check`, `cargo clippy`, and `cargo test` invocations in gate and verification commands must use `--all-targets` so the test, bench, and example targets compile.
