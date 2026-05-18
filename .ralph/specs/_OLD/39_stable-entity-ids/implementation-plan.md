# Implementation Plan: stable-entity-ids

## Execution Rules

- One atomic step at a time.
- Each step maps to TASK-170.
- TDD first (Step 1 sets up failing tests); then schema + helper (Steps 2–3); then producer migration (Step 4); then emit migration (Step 5); then test fixture sweep (Step 6); then acceptance ceremony (Step 7).
- Each step honors the context-discipline preamble.
- The implementer never reads `OrcaSlicerDocumented/`, `target/`, `Cargo.lock`, or any file > 600 lines in full.
- The implementation contract is **regression-free at G-code byte level**. AC-2 (`benchy_end_to_end_tdd` PASS unchanged) is the canary — any deviation is a defect.

## Steps

### Step 0: Discovery — six FACTs / one LOCATIONS sweep before touching code

- Task IDs: `TASK-170`
- Objective: read-only discovery. Answer the six 🔍 questions in `design.md`. The answers determine (a) the entity struct name + line, (b) the `TravelMove` anchor field name + type, (c) the workspace `entity_idx` footprint (sizes Step 6), (d) WIT boundary exposure (gates packet scope), (e) schema-version bump location, (f) producer context shape.
- Precondition: Step 0 not yet run.
- Postcondition: six FACTs and one LOCATIONS recorded in the implementer's working notes; implementer makes a go/no-go on WIT scope expansion.
- Files allowed to read: none directly (delegate only).
- Files allowed to edit (≤ 3): none.
- Expected sub-agent dispatches:
  - "FACT: in `crates/slicer-ir/src/slice_ir.rs`, what is the exact struct name + line of the entity stored in `LayerCollectionIR.ordered_entities`? Quote the struct definition (≤ 10 lines). Also locate `TravelMove`: quote its definition and the exact name + type of its `ordered_entities` anchor field."
  - "LOCATIONS: every workspace site referencing `entity_idx` (case-sensitive). Use ripgrep with `--type rust`. Return file:line + a 1-line snippet for each. ≤ 30 entries; if > 30, paginate by crate and report the total count."
  - "FACT: in `crates/slicer-host/src/dispatch.rs:2861-2877`, quote the finalization-merge code block exactly (≤ 20 lines). Confirm the `splice(0..0, ...)` is still at the same site after Packet 38-rev1 closure."
  - "FACT: in `crates/slicer-host/src/layer_executor.rs:600-665`, quote each of the three producer sites (~lines 605-617, 619-636, 638-659) at ≤ 8 lines each. Identify whether each site has access to a per-layer context struct (e.g., a `LayerCtx` parameter) that could carry `LayerEntityIdGen`, or whether the generator must be passed as an explicit `&mut` parameter."
  - "FACT: in `docs/02_ir_schemas.md`, where is the IR schema version constant defined (file:line) and what is the documented bump rule for an additive field? Quote ≤ 3 lines. Does the policy require a `docs/14_deviation_audit_history.md` entry for an additive bump?"
  - "FACT: search `wit/` and `crates/slicer-host/src/wit_host.rs` for any reference to `TravelMove` or to the anchor field name returned by FACT 1. Report file:line for each match. If no matches, return `no WIT boundary exposure`."
- Context cost: `S`.
- Authoritative docs: none beyond the dispatches.
- OrcaSlicer refs: none.
- Verification: the seven returns, recorded.
- Exit condition: implementer can answer the six 🔍 questions without further reading; if WIT exposure is positive, packet stops and escalates to user before any code change.

### Step 1: Author failing TDD tests

- Task IDs: `TASK-170`
- Objective: create three new test files specifying the contract:
  - `crates/slicer-ir/tests/entity_id_invariants_tdd.rs` (4 tests: `unique_per_layer_and_resolvable`, `entity_id_round_trips_through_serde`, `id_gen_is_strictly_monotonic`, `id_gen_no_collision_under_contention`)
  - `crates/slicer-ir/tests/ir_validation_tdd.rs` (1 test: `dangling_travel_anchor_rejected`)
  - `crates/slicer-host/tests/gcode_emit_travel_anchor_tdd.rs` (2 tests: `travel_emitted_at_entity_id_endpoints`, `travel_survives_entity_reorder`)
- Precondition: Step 0 complete; WIT exposure resolved.
- Postcondition: tests authored; `cargo test -p slicer-ir` and `cargo test -p slicer-host` either fail to compile (acceptable until Step 2 lands the new fields) OR compile-and-fail with expected assertion failures.
- Files allowed to read:
  - `modules/core-modules/skirt-brim/tests/skirt_brim_tdd.rs` and `finalization_live_tdd.rs` for fixture-construction patterns (precedent set in Packet 38-rev1).
  - `crates/slicer-host/tests/benchy_end_to_end_tdd.rs::benchy_gcode_contains_ironing_evidence` lines around it — small context for the regression contract (do not edit; AC-2 asserts it stays unchanged).
- Files allowed to edit (≤ 3):
  - `crates/slicer-ir/tests/entity_id_invariants_tdd.rs`
  - `crates/slicer-ir/tests/ir_validation_tdd.rs`
  - `crates/slicer-host/tests/gcode_emit_travel_anchor_tdd.rs`
- Expected sub-agent dispatches:
  - "Run `cargo test -p slicer-ir --test entity_id_invariants_tdd 2>&1 | tail -40`; FACT compile-fail or assertion-fail with ≤ 20 lines of failing assertion."
  - "Run `cargo test -p slicer-host --test gcode_emit_travel_anchor_tdd 2>&1 | tail -40`; FACT compile-fail or assertion-fail."
- Context cost: `M`.
- Authoritative docs: `docs/02_ir_schemas.md` for `TravelMove`/entity struct shape.
- OrcaSlicer refs: none.
- Verification:
  - new tests compile against the (yet-to-be-migrated) IR shape OR fail-to-compile at the field name — acceptable.
  - benchy regression test still PASSES (no IR changes yet).
- Exit condition: 7 tests authored across 3 new test files.

### Step 2: IR schema migration — entity_id field + TravelMove anchor swap + LayerEntityIdGen

- Task IDs: `TASK-170`
- Objective: add `pub entity_id: u64` to the entity struct (per Step 0 FACT 1); change `TravelMove`'s anchor field to `entity_id: u64`; add `LayerEntityIdGen` helper in a new `crates/slicer-ir/src/entity_id.rs`; bump IR schema version per Step 0 FACT 5.
- Precondition: Step 1 complete.
- Postcondition: `slicer-ir` package builds; entity-id invariant tests compile (assertion failures acceptable until producers are migrated in Step 4).
- Files allowed to read:
  - `crates/slicer-ir/src/slice_ir.rs` — narrow ranges only (entity struct + `TravelMove` ± 40 lines).
  - `crates/slicer-ir/src/lib.rs` — full read (small).
  - `docs/02_ir_schemas.md` — schema version and bump rule (per Step 0 FACT 5).
- Files allowed to edit (≤ 3):
  - `crates/slicer-ir/src/slice_ir.rs`
  - `crates/slicer-ir/src/entity_id.rs` (new)
  - `crates/slicer-ir/src/lib.rs` (re-export)
- Expected sub-agent dispatches:
  - "Run `cargo build -p slicer-ir`; FACT pass/fail with ≤ 10 lines of error on FAIL."
  - "Run `cargo test -p slicer-ir --test entity_id_invariants_tdd id_gen_is_strictly_monotonic 2>&1 | tail -20`; FACT pass/fail."
- Context cost: `M`.
- Authoritative docs: `docs/02_ir_schemas.md`.
- OrcaSlicer refs: none.
- Verification:
  - `cargo build -p slicer-ir` PASS.
  - `id_gen_is_strictly_monotonic` PASS.
  - The other 6 tests compile (assertion failures expected until Step 4–5).
- Exit condition: schema is migrated; helper exists; `slicer-ir` builds.

### Step 3: Add validation helper

- Task IDs: `TASK-170`
- Objective: implement `pub fn validate_travel_anchors(layer: &LayerCollectionIR) -> Result<(), ValidateError>` in a new or appended `crates/slicer-ir/src/validation.rs`. Make `dangling_travel_anchor_rejected` PASS. Diagnostic message must contain the literal substring `entity_id` and the offending ID number.
- Precondition: Step 2 complete.
- Postcondition: `dangling_travel_anchor_rejected` PASSES.
- Files allowed to read:
  - `crates/slicer-ir/src/slice_ir.rs` — only the migrated entity + `TravelMove` types.
  - `crates/slicer-ir/src/lib.rs`.
- Files allowed to edit (≤ 2):
  - `crates/slicer-ir/src/validation.rs` (new or appended)
  - `crates/slicer-ir/src/lib.rs` (re-export)
- Expected sub-agent dispatches:
  - "Run `cargo test -p slicer-ir --test ir_validation_tdd dangling_travel_anchor_rejected 2>&1 | tail -20`; FACT pass/fail."
- Context cost: `S`.
- Authoritative docs: none.
- OrcaSlicer refs: none.
- Verification:
  - `dangling_travel_anchor_rejected` PASS.
- Exit condition: validation helper green.

### Step 4: Producer migration — layer_executor + dispatch finalization-merge

- Task IDs: `TASK-170`
- Objective: thread `LayerEntityIdGen` through layer-construction code paths. Each producer site stamps every entity with a fresh ID; each `TravelMove` constructed in the same site references the just-issued `entity_id`. The dispatch finalization-merge stamps IDs on incoming finalization-pushed entities at merge time. The `splice(0..0, ...)` semantics are preserved bit-for-bit.
- Precondition: Step 3 complete.
- Postcondition: package builds; `entity_id_invariants_tdd::unique_per_layer_and_resolvable` PASS; `entity_id_round_trips_through_serde` PASS; benchy regression test must NOT regress (AC-2 canary).
- Files allowed to read:
  - `crates/slicer-host/src/layer_executor.rs:600-665` — narrow range (lines around the three producer sites).
  - `crates/slicer-host/src/dispatch.rs:2861-2877` — narrow range (finalization merge).
  - `crates/slicer-ir/src/slice_ir.rs` (the migrated types) and `entity_id.rs` (the generator).
- Files allowed to edit (≤ 2):
  - `crates/slicer-host/src/layer_executor.rs`
  - `crates/slicer-host/src/dispatch.rs`
- Expected sub-agent dispatches:
  - "Run `cargo build -p slicer-host`; FACT pass/fail with ≤ 10 lines of error on FAIL."
  - "Run `cargo test -p slicer-ir --test entity_id_invariants_tdd unique_per_layer_and_resolvable 2>&1 | tail -20`; FACT pass/fail."
  - "Run `cargo test -p slicer-host --test benchy_end_to_end_tdd 2>&1 | tail -40`; FACT pass/fail. **Critical regression canary.**"
- Context cost: `M`.
- Authoritative docs: `docs/04_host_scheduler.md` § Composable Multi-Writer Patterns.
- OrcaSlicer refs: none.
- Verification:
  - `cargo build -p slicer-host` PASS.
  - `unique_per_layer_and_resolvable` PASS.
  - `benchy_end_to_end_tdd` PASS unchanged (regression contract).
- Exit condition: producers migrated; benchy still green.

### Step 5: Emit migration — gcode_emit travel anchor resolution by ID

- Task IDs: `TASK-170`
- Objective: at the top of the per-layer emit loop in `gcode_emit.rs:182`, build `let id_to_idx: HashMap<u64, usize> = layer.ordered_entities.iter().enumerate().map(|(i, e)| (e.entity_id, i)).collect();`. Replace travel-resolution at `gcode_emit.rs:285-295` to look up via `id_to_idx[&travel.entity_id]`. Add `debug_assert!(id_to_idx.contains_key(&travel.entity_id))`.
- Precondition: Step 4 complete.
- Postcondition: `gcode_emit_travel_anchor_tdd` (both tests) PASSES; benchy regression PASSES unchanged.
- Files allowed to read:
  - `crates/slicer-host/src/gcode_emit.rs:170-200, 280-310` — narrow ranges.
- Files allowed to edit (≤ 1):
  - `crates/slicer-host/src/gcode_emit.rs`
- Expected sub-agent dispatches:
  - "Run `cargo test -p slicer-host --test gcode_emit_travel_anchor_tdd 2>&1 | tail -30`; FACT pass/fail per test."
  - "Run `cargo test -p slicer-host --test benchy_end_to_end_tdd 2>&1 | tail -30`; FACT pass/fail (regression canary)."
- Context cost: `S`.
- Authoritative docs: none.
- OrcaSlicer refs: none.
- Verification:
  - 2/2 anchor tests PASS.
  - Benchy regression PASS unchanged.
- Exit condition: emit migrated; canary still green.

### Step 6: Test fixture sweep

- Task IDs: `TASK-170`
- Objective: migrate every workspace test fixture identified in Step 0 LOCATIONS that constructs `TravelMove` with the old `entity_idx` field. Each migration is a one-pattern edit: capture the entity's `entity_id` at construction and use it as the anchor.
- Precondition: Step 5 complete.
- Postcondition: every fixture compiles; targeted-test runs PASS for the affected crates; cargo workspace build PASSES.
- Files allowed to read:
  - the fixture files identified by Step 0 LOCATIONS (narrow ranges only — only the lines around each `entity_idx` reference).
- Files allowed to edit (≤ 3 per worker dispatch; if > 3 sites remain, multiple dispatches):
  - the identified fixture files (per dispatch batch).
- Expected sub-agent dispatches (per batch of ≤ 10 fixtures):
  - "Migrate the following N fixture sites: [list from Step 0 LOCATIONS]. Each: replace `entity_idx: <expr>` with `entity_id: <captured-id>`. Return: list of files changed; pass/fail of the targeted-test run for each affected crate."
- Context cost: `S`–`M` (depends on Step 0 LOCATIONS volume).
- Authoritative docs: none.
- OrcaSlicer refs: none.
- Verification:
  - `cargo build --workspace` PASS.
  - `cargo test -p <crate>` PASS for each affected crate.
- Exit condition: zero remaining `entity_idx` references in workspace test code; all targeted-test runs green.

### Step 7: Acceptance ceremony + docs/07 row

- Task IDs: `TASK-170`
- Objective: re-run every acceptance command from `packet.spec.md`; run workspace gates (`cargo test --workspace`, `cargo clippy --workspace -- -D warnings`); insert `TASK-170` row in `docs/07_implementation_status.md`. Record schema-version bump in `docs/14_deviation_audit_history.md` if Step 0 FACT 5 mandated it.
- Precondition: Step 6 complete.
- Postcondition: every AC PASSES; backlog updated; workspace closure gate PASSES; clippy clean.
- Files allowed to read: none directly (dispatch only).
- Files allowed to edit (≤ 2):
  - `docs/07_implementation_status.md` (delegate insertion via worker).
  - `docs/14_deviation_audit_history.md` only if Step 0 FACT 5 required it.
- Expected sub-agent dispatches:
  - 8 narrow AC commands from `packet.spec.md` `## Acceptance Criteria` and `## Negative Test Cases`, each as a separate FACT pass/fail.
  - "Run `cargo test --workspace --no-fail-fast 2>&1 | tail -40`; FACT pass/fail with failing test list (≤ 20 lines)."
  - "Run `cargo clippy --workspace -- -D warnings 2>&1 | tail -20`; FACT pass/fail."
  - "Run `./modules/core-modules/build-core-modules.sh`; FACT pass/fail."
  - "Insert a TASK-170 row into `docs/07_implementation_status.md` describing this packet's deliverable. Return the inserted line as FACT (file:line, contents). Do NOT load the whole file."
- Context cost: `S`.
- Authoritative docs: `docs/07_implementation_status.md` (delegate-only).
- OrcaSlicer refs: none.
- Verification: every pipe-suffixed AC command from `packet.spec.md`.
- Exit condition: every AC PASSES; `cargo test --workspace` PASSES; `cargo clippy --workspace -- -D warnings` PASSES; `docs/07` carries TASK-170; packet ready to move to `status: implemented`.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 0 | S | Six FACT/LOCATIONS dispatches. |
| Step 1 | M | TDD authoring (7 tests across 3 files). |
| Step 2 | M | IR schema migration + helper. |
| Step 3 | S | Validation helper. |
| Step 4 | M | Producer migration; benchy canary. |
| Step 5 | S | Emit migration. |
| Step 6 | S–M | Fixture sweep (sized by Step 0 LOCATIONS). |
| Step 7 | S | Acceptance + docs row insertion. |

Aggregate: `M`. No single step is `L`.

## Packet Completion Gate

- All steps complete.
- Every AC verification command from `packet.spec.md` PASSES.
- `cargo test --workspace` PASSES.
- `cargo clippy --workspace -- -D warnings` PASSES.
- `./modules/core-modules/build-core-modules.sh` PASSES.
- `cargo test -p slicer-host --test benchy_end_to_end_tdd` PASSES unchanged from before this packet (regression canary).
- `docs/07_implementation_status.md` carries TASK-170.
- `packet.spec.md` ready to move to `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed AC command (8 commands).
- Confirm `cargo test --workspace`, `cargo clippy --workspace -- -D warnings`, and `./modules/core-modules/build-core-modules.sh` PASS.
- Confirm benchy regression canary unchanged.
- Confirm implementer's peak context usage stayed under 70%.
- Record IR schema-version bump in `docs/14_deviation_audit_history.md` only if Step 0 FACT 5 required it.
