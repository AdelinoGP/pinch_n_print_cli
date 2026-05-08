# Implementation Plan: finalization-mutation-enum-refactor

## Execution Rules

- One atomic step at a time.
- Each step maps to TASK-172.
- TDD first (Step 1); then type definitions (Step 2); then SDK API replacement (Step 3); then WIT alignment + host-impl simplification (Step 4); then drain-back wiring + WASM round-trip test (Step 5); then acceptance ceremony (Step 6). Step 0 is read-only discovery + variant audit.
- Each step honors the context-discipline preamble.
- The implementer never reads `OrcaSlicerDocumented/`, `target/`, `Cargo.lock`, or any file > 600 lines in full.
- The packet's load-bearing invariant: **closure-bound generic signatures are absent from `FinalizationOutputBuilder`'s mutation methods after Step 3 lands**. NEG-4 is the canary.
- Packet 40's print-quality fix (`benchy_top_surface_precedes_ironing`) MUST stay green at every step. Step 5 explicitly re-runs it.

## Steps

### Step 0: Discovery + variant audit

- Task IDs: `TASK-172`
- Objective: read-only discovery + locked decisions on `EntityMutation` / `SortKey` variant lists. Answer the seven 🔍 questions in `design.md`.
- Precondition: Packet 40 is `implemented` (confirmed at packet generation; Step 0 re-confirms via FACT).
- Postcondition: locked variant lists; alignment-or-drift verdict for WIT names; existence verdict for `merge_ops()` accessor; existence verdict for `PrintEntity.object_id`; existence verdict for the round-trip test file (must NOT exist).
- Files allowed to read: none directly (delegate only).
- Files allowed to edit (≤ 0): none.
- Expected sub-agent dispatches:
  - "FACT: read `.ralph/specs/40_finalization-mutation-builder/packet.spec.md` frontmatter; quote `status:` line. Confirm `TASK-171` exists in `docs/07_implementation_status.md` (≤ 3-line quote)."
  - "FACT: in `crates/slicer-sdk/src/traits.rs`, locate `FinalizationOutputBuilder` struct, the `MergeOp` enum, `priority_pushes()` accessor, and `merge_ops()` accessor (if present). Quote each declaration site (≤ 5 lines each, with file:line). Report whether `merge_ops()` accessor exists today."
  - "FACT: in `wit/world-finalization.wit`, quote the existing `entity-mutation`, `sort-key`, `synthetic-layer-data` definitions (≤ 30 lines total, with file:line)."
  - "FACT: in `crates/slicer-macros/src/lib.rs` `build_finalization_world_glue` (around line 948–974), quote the inline WIT for the same three types (≤ 30 lines total). Confirm name match with canonical WIT or report drift."
  - "SNIPPETS: `crates/slicer-host/src/wit_host.rs` `HostFinalizationOutputBuilder` impl methods for `modify_entity` / `sort_layer_by` / `insert_synthetic_layer_after` — verbatim ≤ 30 lines each."
  - "SUMMARY ≤ 200 words: read `.ralph/specs/40_finalization-mutation-builder/design.md` Open Questions and the four future-module references (`SequentialPrintOrder`, `MinLayerTimeEnforcer`, `FlushVolumeCalculator`, `PrimeTower`). For each module, list the `PrintEntity` (or `ExtrusionPath3D`) fields it plausibly mutates. This drives the locked `EntityMutation` variant list."
  - "FACT: in `crates/slicer-ir/src/slice_ir.rs`, does `PrintEntity` carry an `object_id` field (or equivalent like `region_key.object_id`)? Quote the field declaration if present (≤ 3 lines, with file:line). If absent, recommend dropping `SortKey::ByObjectIdThenPriority` or deferring."
  - "FACT: list every `Cargo.toml` under `test-guests/`. Return `[package].name` for each (LOCATIONS, ≤ 10 entries). Confirm that `finalization-mutation-roundtrip-guest` does NOT already exist."
  - "FACT: list every test file under `crates/slicer-host/tests/` (LOCATIONS only). Confirm `finalization_mutation_roundtrip_tdd.rs` does NOT already exist."
  - "FACT: grep `crates/slicer-sdk/tests/finalization_builder_tdd.rs` for closure literals (`Box::new(|` and `|e|` not nested inside helper fns). Count occurrences. If > 12, flag for Step 1 split."
- Context cost: `S`.
- Authoritative docs: none beyond the dispatches.
- OrcaSlicer refs: none.
- Verification: nine FACT/SUMMARY/SNIPPETS returns recorded.
- Exit condition: implementer can write the locked decisions into Step 1's TDD authoring without further discovery. Step 1 begins with concrete variant lists.

### Step 1: Author failing TDD tests (red bar)

- Task IDs: `TASK-172`
- Objective: migrate the 8 existing `finalization_builder_tdd` tests from closure-form to enum-form (compile-fail expected) AND author the new host-side round-trip test file (assertion-fail expected). Author the new WASM test guest skeleton.
- Three test scopes:
  - `crates/slicer-sdk/tests/finalization_builder_tdd.rs` — migrate 8 closure-form tests; add `modify_entity_set_speed_factor_applies` (AC-1), `modify_entity_set_flow_factor_applies` (AC-2 — per-point flow_factor multiplier; the path-level `SetExtrusionWidthFactor` was rejected because `ExtrusionPath3D` carries no such field today and IR shape changes are out of scope), `closure_api_is_fully_removed` (NEG-4).
  - `crates/slicer-host/tests/finalization_mutation_roundtrip_tdd.rs` (NEW) — three tests: `modify_entity_round_trips_through_wit`, `modify_entity_unknown_id_round_trips_error`, `drain_back_forwards_merge_ops`.
  - `test-guests/finalization-mutation-roundtrip-guest/` (NEW crate) — `Cargo.toml` + `src/lib.rs` skeleton implementing `FinalizationModule`. The `run_finalization` body calls `output.modify_entity(layer, 1, EntityMutation::SetSpeedFactor(0.5))` (and optionally a second variant for the unknown-id NEG via a config switch or sibling impl).
- Precondition: Step 0 complete; locked variant lists in hand.
- Postcondition: tests authored; targeted runs either compile-fail (acceptable) OR compile-and-fail with expected assertion failures.
- Files allowed to read:
  - `crates/slicer-sdk/tests/finalization_builder_tdd.rs` (existing — full read OK).
  - `crates/slicer-sdk/src/traits.rs` (narrow — the `FinalizationOutputBuilder` impl block, for closure-form signature reference).
  - `test-guests/sdk-finalization-guest/Cargo.toml` and `src/lib.rs` (full read — small precedent for the new guest).
  - `crates/slicer-host/tests/manifest_ingestion_tdd.rs` (narrow — for module-loading helper conventions on the host side).
  - `crates/slicer-ir/src/slice_ir.rs` (narrow — `PrintEntity`, `ExtrusionPath3D`, `LayerCollectionIR` shapes).
- Files allowed to edit (≤ 5 — exception to ≤ 3 because the new guest is a multi-file scaffold AND the `test-guests/build-test-guests.sh` GUESTS array MUST be updated; can split into Step 1a / Step 1b if scope is too tight):
  - `crates/slicer-sdk/tests/finalization_builder_tdd.rs` (migration + 3 new tests)
  - `crates/slicer-host/tests/finalization_mutation_roundtrip_tdd.rs` (NEW)
  - `test-guests/finalization-mutation-roundtrip-guest/Cargo.toml` (NEW)
  - `test-guests/finalization-mutation-roundtrip-guest/src/lib.rs` (NEW)
  - `test-guests/build-test-guests.sh` (one-line addition to the `GUESTS=(...)` array — `test-guests/*` are NOT workspace members, so this script is the only path that produces the `.component.wasm` the host test loads)
- Expected sub-agent dispatches:
  - "Run `cargo test -p slicer-sdk --test finalization_builder_tdd 2>&1 | tail -50`; FACT compile-fail or assertion-fail. Document each failure mode."
  - "Run `cargo test -p slicer-host --test finalization_mutation_roundtrip_tdd 2>&1 | tail -30`; FACT compile-fail expected (the new test guest's WIT bindings + `EntityMutation` symbols don't yet exist)."
  - "Confirm `test-guests/build-test-guests.sh` `GUESTS=(...)` array now contains an entry for `finalization-mutation-roundtrip-guest:finalization_mutation_roundtrip_guest`. Quote the line. FACT pass/fail. (`test-guests/*` are NOT workspace members; the script is the only build path.)"
  - "Run `./test-guests/build-test-guests.sh 2>&1 | tail -20`; FACT compile-fail expected (guest references `EntityMutation` / WIT bindings that don't yet exist) — the goal at Step 1 is just to confirm the script picks up the new entry, not that it builds clean."
- Context cost: `M`.
- Authoritative docs: `docs/02_ir_schemas.md`, `docs/05_module_sdk.md`, `docs/03_wit_and_manifest.md`.
- OrcaSlicer refs: none.
- Verification:
  - new + migrated tests compile-fail at the missing `EntityMutation` / `SortKey` / `SyntheticLayerData` symbols, OR assertion-fail.
  - existing benchy assertion `benchy_gcode_contains_ironing_evidence` and `benchy_top_surface_precedes_ironing` still PASS unchanged.
- Exit condition: 11+ tests authored across three scopes; new test guest scaffolded.

### Step 2: Define `EntityMutation`, `SortKey`, `SyntheticLayerData`

- Task IDs: `TASK-172`
- Objective: define the three new types in `slicer-sdk` (or `slicer-ir` if shared with WIT — Step 0 decision). Add re-exports in `slicer-sdk/src/lib.rs` if not already in prelude.
- Precondition: Step 1 complete; locked variant lists in hand.
- Postcondition: `cargo build -p slicer-sdk` PASSES; the three new types are constructible from a test crate.
- Files allowed to read:
  - `crates/slicer-sdk/src/traits.rs` — narrow.
  - `crates/slicer-sdk/src/lib.rs` — full read OK (small).
  - `crates/slicer-ir/src/slice_ir.rs` — narrow (`ExtrusionPath3D` for `SyntheticLayerData.paths`).
- Files allowed to edit (≤ 2):
  - `crates/slicer-sdk/src/traits.rs`
  - `crates/slicer-sdk/src/lib.rs` (re-exports if needed)
- Expected sub-agent dispatches:
  - "Run `cargo build -p slicer-sdk 2>&1 | tail -10`; FACT pass/fail."
  - "Run `cargo test -p slicer-sdk --test finalization_builder_tdd closure_api_is_fully_removed 2>&1 | tail -15`; FACT — likely still FAIL because Step 3 hasn't removed the closure methods yet. Acceptable."
- Context cost: `S`.
- Authoritative docs: `docs/02_ir_schemas.md`.
- OrcaSlicer refs: none.
- Verification:
  - `cargo build -p slicer-sdk` PASS.
  - Types are introspectable via Rust syntax-check (compile-pass is sufficient).
- Exit condition: three types compiled clean; ready for Step 3 to consume them.

### Step 3: SDK API replacement — closure methods → enum methods

- Task IDs: `TASK-172`
- Objective: replace the three closure-typed methods on `FinalizationOutputBuilder` with enum-typed forms. Refactor `MergeOp` enum to plain serializable variants. Rewrite `apply_to` to consume the new shape. Add `merge_ops()` accessor if Step 0 confirmed it absent.
- Precondition: Step 2 complete.
- Postcondition: 11/11+ `finalization_builder_tdd` tests PASS. Closure-bound generic signatures are GONE from `FinalizationOutputBuilder` (NEG-4 PASS).
- Files allowed to read:
  - `crates/slicer-sdk/src/traits.rs` — full re-read of the impl block + `apply_to`.
  - `crates/slicer-sdk/tests/finalization_builder_tdd.rs` — to see what the migrated tests expect.
  - `crates/slicer-ir/src/slice_ir.rs` — narrow (`PrintEntity`, `ExtrusionPath3D` field paths for `EntityMutation` apply logic).
- Files allowed to edit (≤ 1):
  - `crates/slicer-sdk/src/traits.rs`
- Expected sub-agent dispatches:
  - "Run `cargo build -p slicer-sdk 2>&1 | tail -10`; FACT pass/fail."
  - "Run `cargo test -p slicer-sdk --test finalization_builder_tdd 2>&1 | tail -50`; FACT pass/fail per test."
  - "Run `cargo test -p slicer-sdk --test finalization_module_tdd 2>&1 | tail -20`; FACT (regression — must remain green)."
  - "Run `cargo build -p slicer-host 2>&1 | tail -10`; FACT (regression — `wit_host.rs` may need Step 4 simplification, but should compile against the new SDK API as long as the WIT-host's translation layer adapts)."
- Context cost: `M`.
- Authoritative docs: `docs/05_module_sdk.md`.
- OrcaSlicer refs: none.
- Verification:
  - `cargo build -p slicer-sdk` PASS.
  - 11+ `finalization_builder_tdd` tests PASS (8 migrated + 3 new).
  - 7/7 `finalization_module_tdd` regression PASS.
  - `cargo build -p slicer-host` PASS.
- Exit condition: SDK API fully closure-free for the three mutation methods; all SDK-side tests green; host still compiles.

### Step 4: WIT alignment + host-impl simplification

- Task IDs: `TASK-172`
- Objective: align the WIT shapes (`wit/world-finalization.wit` + `crates/slicer-macros/src/lib.rs` inline WIT) with the new SDK names. Simplify `crates/slicer-host/src/wit_host.rs` `HostFinalizationOutputBuilder` impl methods to direct forwards (no closure construction).
- Precondition: Step 3 complete; SDK API is closure-free.
- Postcondition: `cargo build --workspace` PASS; `./modules/core-modules/build-core-modules.sh` PASS; benchy regression PASS; the host-impl translation layer is shorter / simpler than before.
- Files allowed to read:
  - `wit/world-finalization.wit` — full (small).
  - `crates/slicer-host/src/wit_host.rs` — narrow (HostFinalizationOutputBuilder impl block).
  - `crates/slicer-macros/src/lib.rs` — narrow (lines 948–974 inline WIT).
- Files allowed to edit (≤ 3):
  - `wit/world-finalization.wit` (alignment only — likely no edit if Step 0 confirms names match)
  - `crates/slicer-host/src/wit_host.rs`
  - `crates/slicer-macros/src/lib.rs` (inline WIT alignment only at this step; the drain-back loop is Step 5)
- Expected sub-agent dispatches:
  - "Run `cargo build --workspace 2>&1 | tail -15`; FACT pass/fail."
  - "Run `./modules/core-modules/build-core-modules.sh 2>&1 | tail -15`; FACT (WASM rebuild canary)."
  - "Run `cargo test -p slicer-host --test benchy_end_to_end_tdd 2>&1 | tail -25`; FACT (regression)."
  - "Run `cargo test -p slicer-host --test manifest_ingestion_tdd 2>&1 | tail -15`; FACT (regression)."
- Context cost: `M`.
- Authoritative docs: `docs/03_wit_and_manifest.md`.
- OrcaSlicer refs: none.
- Verification:
  - `cargo build --workspace` PASS.
  - `./modules/core-modules/build-core-modules.sh` PASS.
  - benchy 30/30 PASS.
  - manifest_ingestion 21/21 PASS.
- Exit condition: WIT/SDK names are aligned; host-impl is a direct forward; all builds pass; regression canaries green.

### Step 5: Drain-back wiring + WASM round-trip validation

- Task IDs: `TASK-172`
- Objective: extend `crates/slicer-macros/src/lib.rs` `run_finalization` drain-back to iterate `merge_ops()` and forward each variant via WIT. Author the test guest's `run_finalization` body and the host-side end-to-end test. AC-5 (modify_entity round-trip) must PASS at the end of this step.
- Precondition: Step 4 complete.
- Postcondition: 3+ `finalization_mutation_roundtrip_tdd` tests PASS; benchy 30/30 still PASS; macro drain-back emits a forward call per `MergeOp` variant.
- Files allowed to read:
  - `crates/slicer-macros/src/lib.rs` — narrow (lines 1198–1214 drain-back).
  - `test-guests/finalization-mutation-roundtrip-guest/src/lib.rs` (was scaffolded at Step 1).
  - `crates/slicer-host/tests/finalization_mutation_roundtrip_tdd.rs` (was scaffolded at Step 1).
  - `test-guests/sdk-finalization-guest/src/lib.rs` (read-only reference for guest-side WIT call patterns).
- Files allowed to edit (≤ 3):
  - `crates/slicer-macros/src/lib.rs` (drain-back loop extension only — inline WIT alignment was Step 4)
  - `test-guests/finalization-mutation-roundtrip-guest/src/lib.rs` (real `run_finalization` body)
  - `crates/slicer-host/tests/finalization_mutation_roundtrip_tdd.rs` (real assertions)
- Expected sub-agent dispatches:
  - "Run `cargo build --workspace 2>&1 | tail -15`; FACT pass/fail."
  - "Confirm `test-guests/build-test-guests.sh` `GUESTS=(...)` array (lines 17–28 baseline) contains an entry for `finalization-mutation-roundtrip-guest`. Quote the relevant line as FACT. (`test-guests/*` are NOT workspace members; this script is the only build path.)"
  - "Run `./test-guests/build-test-guests.sh 2>&1 | tail -25`; FACT pass/fail. Must produce a `.component.wasm` artifact for `finalization-mutation-roundtrip-guest` at the path expected by the host test."
  - "Run `./modules/core-modules/build-core-modules.sh 2>&1 | tail -15`; FACT pass/fail (core-modules WASM rebuild canary; does NOT build test guests)."
  - "Run `cargo test -p slicer-host --test finalization_mutation_roundtrip_tdd 2>&1 | tail -30`; FACT pass/fail per test (≥ 3 tests)."
  - "Run `cargo test -p slicer-host --test benchy_end_to_end_tdd 2>&1 | tail -25`; FACT regression."
  - "Run `cargo test -p slicer-sdk --test finalization_builder_tdd 2>&1 | tail -25`; FACT regression."
  - "Grep `crates/slicer-macros/src/lib.rs` for the AC-7 iteration pattern (regex `for\\s+\\w+\\s+in\\s+[^{]*merge_ops` OR `merge_ops\\(\\)\\s*\\.\\s*iter\\(\\)`), AND for any surviving `silently no-op` / `DEV-041` / TODO comment strings referencing `merge_ops`; FACT — must show a real iteration site is present (not a struct-field reference) and no surviving stale-comment strings remain (pre-implementation, two TODO matches at `lib.rs:1212`/`:1214` exist; both must be deleted)."
- Context cost: `M`.
- Authoritative docs: `docs/04_host_scheduler.md` lines 309–317.
- OrcaSlicer refs: none.
- Verification:
  - `cargo build --workspace` PASS.
  - WASM build PASS.
  - 3+ `finalization_mutation_roundtrip_tdd` tests PASS (AC-5, NEG-3, AC-7).
  - benchy 30/30 PASS.
  - 11+ `finalization_builder_tdd` tests still PASS (regression).
- Exit condition: round-trip ACs PASS; macro drain-back forwards `merge_ops`; the silent-no-op gap is closed.

### Step 6: Acceptance ceremony + docs/07 row + DEV-041 closure

- Task IDs: `TASK-172`
- Objective: re-run every acceptance command from `packet.spec.md`; run workspace gates; insert `TASK-172` row; annotate the existing `DEV-041` row in `docs/DEVIATION_LOG.md` as closed.
- Precondition: Step 5 complete.
- Postcondition: every AC PASSES; backlog updated; workspace closure gate PASSES; clippy clean; `DEV-041` annotated as closed in `docs/DEVIATION_LOG.md`.
- Files allowed to read: none directly (dispatch only).
- Files allowed to edit (≤ 2):
  - `docs/07_implementation_status.md` (delegated insertion)
  - `docs/DEVIATION_LOG.md` (delegated edit to the existing DEV-041 row at line 47 — change Status column from "Open" to "Closed YYYY-MM-DD" with a one-paragraph closure note referencing TASK-172). The legacy `docs/14_deviation_audit_history.md` is NOT edited.
- Expected sub-agent dispatches:
  - 12 narrow AC commands from `packet.spec.md` `## Acceptance Criteria` (8) and `## Negative Test Cases` (4), each as a separate FACT pass/fail.
  - "Run `cargo test --workspace --no-fail-fast 2>&1 | tail -40`; FACT pass/fail with failing test list (≤ 20 lines)."
  - "Run `cargo clippy --workspace -- -D warnings 2>&1 | tail -20`; FACT pass/fail."
  - "Run `./modules/core-modules/build-core-modules.sh 2>&1 | tail -15`; FACT pass/fail."
  - "Insert a TASK-172 row into `docs/07_implementation_status.md`. Return the inserted line as FACT (file:line, contents). Do NOT load the whole file."
  - "In `docs/DEVIATION_LOG.md`, locate the DEV-041 row (currently at line 47) and update its Status column from `Open` to `Closed YYYY-MM-DD` with a one-paragraph closure note that references TASK-172 and packet `41_finalization-mutation-enum-refactor`. Return the before/after of that row only as FACT. Do NOT modify any other DEV-XXX row, and do NOT touch `docs/14_deviation_audit_history.md`."
- Context cost: `S`.
- Authoritative docs: `docs/07_implementation_status.md` (delegated-only); `docs/DEVIATION_LOG.md` (delegated-only).
- OrcaSlicer refs: none.
- Verification: every pipe-suffixed AC command from `packet.spec.md`.
- Exit condition: every AC PASSES; `cargo test --workspace` PASSES; `cargo clippy --workspace -- -D warnings` PASSES; `docs/07` carries TASK-172; `docs/DEVIATION_LOG.md` shows `DEV-041` as closed; packet ready to move to `status: implemented`.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 0 | S | Nine FACT/SUMMARY/SNIPPETS dispatches; locked variant lists. |
| Step 1 | M | TDD migration (8 tests) + 3 new SDK tests + 3 new host tests + new test guest scaffold. |
| Step 2 | S | Three new types in slicer-sdk. |
| Step 3 | M | SDK API replacement + MergeOp refactor + apply_to rewrite. |
| Step 4 | M | WIT alignment + wit_host simplification + macros inline-WIT alignment. |
| Step 5 | M | Drain-back wiring + WASM round-trip validation (the substantive DEV-041 closure). |
| Step 6 | S | Acceptance + docs/07 row + DEV-041 closure note. |

Aggregate: `M`. No single step is `L`.

## Packet Completion Gate

- All steps complete.
- Every AC verification command from `packet.spec.md` PASSES (8 AC + 4 negatives = 12 commands).
- `cargo test --workspace` PASSES.
- `cargo clippy --workspace -- -D warnings` PASSES.
- `./modules/core-modules/build-core-modules.sh` PASSES.
- `cargo test -p slicer-host --test finalization_mutation_roundtrip_tdd modify_entity_round_trips_through_wit` PASSES (the substantive DEV-041 closure validation).
- `cargo test -p slicer-host --test benchy_end_to_end_tdd` PASSES (Packet 40 print-quality fix preserved).
- `cargo test -p slicer-sdk --test finalization_builder_tdd closure_api_is_fully_removed` PASSES (NEG-4: closure API genuinely removed).
- `docs/07_implementation_status.md` carries TASK-172.
- `docs/DEVIATION_LOG.md` row for `DEV-041` shows Status = `Closed YYYY-MM-DD` with closure note. (`docs/14_deviation_audit_history.md` is NOT edited; it is an archive only.)
- `packet.spec.md` ready to move to `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed AC command (12 commands).
- Confirm `cargo test --workspace`, `cargo clippy --workspace -- -D warnings`, and `./modules/core-modules/build-core-modules.sh` PASS.
- Confirm WIT round-trip works end-to-end (AC-5).
- Confirm closure API is genuinely removed (NEG-4).
- Confirm benchy presence + ordering regressions PASS (Packet 40 invariants preserved).
- Confirm implementer's peak context usage stayed under 70%.
