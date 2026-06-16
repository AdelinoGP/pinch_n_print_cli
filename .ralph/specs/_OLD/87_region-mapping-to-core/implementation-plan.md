# Packet 87 — Implementation Plan

## Execution Rules

- Each step ends with a falsifying check that gates green before the next step starts.
- `region_mapping.rs` is 628 LOC; NEVER load in full. Grep + line-range reads only.
- The packet closure gate runs narrow per-crate tests, NOT `cargo test --workspace`.
- P85 MUST be closed before this packet starts (Step 0 verifies).
- No guest-feeding path is edited; `cargo xtask build-guests --check` should stay clean.

---

## Step 0 — Verify P85 closure + capture pre-packet g-code SHA baseline

**Objective.** Confirm `ExecutionPlan` and `CompiledModuleStatic` are in `slicer-scheduler` (not `slicer-runtime`). Capture g-code SHA baseline.

**Precondition.** P85 is `implemented` (and P86 is `implemented`; the carried-forward g-code SHA is `89a329ad3a4c1b7febca839edfca8b6302e562d8d2a390ee144252fd54e65a2b`, byte-identical across P81→P86). Working tree clean.

**Postcondition.** Two log entries: P85-state verification + baseline SHA.

**Files allowed to read.** None directly.
**Files allowed to edit.** None.

**Expected sub-agent dispatches.**
- Dispatch: confirm `rg -l 'pub struct CompiledModuleStatic' crates/ | grep -qE '^crates/slicer-scheduler/' && test -f crates/slicer-scheduler/Cargo.toml`. Return FACT pass/fail.
- Dispatch: g-code SHA — `cargo run --bin pnp_cli --release -- slice --model resources/benchy.stl --module-dir modules/core-modules --output /tmp/p87-baseline.gcode && sha256sum /tmp/p87-baseline.gcode`. Return FACT `<hex>`.

**Context cost: S.**

**Narrow verification.** Both positive.

**Falsifying check / exit condition.** P85 verification fails → abort.

---

## Step 1 — Enumerate `&ExecutionPlan` reads, test consumers, Blackboard commit method

**Objective.** Surface every input needed to define the projection correctly.

**Precondition.** Step 0 green.

**Postcondition.** Three lists in the log per design.md dispatches #1, #2, #3.

**Files allowed to read.** None directly.
**Files allowed to edit.** None.

**Expected sub-agent dispatches.** #1 (plan.<field> reads), #2 (test consumers), #3 (blackboard commit method).

**Context cost: S.**

**Narrow verification.** Three returns populated.

**Falsifying check / exit condition.** Dispatch #1 returns > 5 fields → revisit the projection design; maybe a single `&ExecutionPlan`-shaped trait would be cleaner.

---

## Step 2 — Define `RegionMappingPlanProjection` + scaffold `slicer-core/src/algos/region_mapping.rs`

**Objective.** New `slicer-core` algo module exists with a projection struct and stub `execute_region_mapping`. Build green.

**Precondition.** Step 1 lists in hand.

**Postcondition.** `cargo build -p slicer-core` green with the new module declaring `RegionMappingPlanProjection` and a stub `execute_region_mapping` (body: `unimplemented!()` initially — populated in Step 3).

**Files allowed to read.** `crates/slicer-core/src/algos/mod.rs` (to slot the new module).
**Files allowed to edit.**
1. `crates/slicer-core/src/algos/region_mapping.rs` — CREATE with projection struct + stub fn.
2. `crates/slicer-core/src/algos/mod.rs` — add `pub mod region_mapping;` + re-exports.

**Expected sub-agent dispatch.**
- Dispatch: `cargo build -p slicer-core`. Return FACT pass/fail.

**Context cost: S.**

**Narrow verification.** Build green.

**Falsifying check / exit condition.** Build fails → projection struct's IR-type imports are wrong; check `slicer-ir`'s exports.

---

## Step 3 — Move kernel body into `slicer-core/src/algos/region_mapping.rs`; delete from `slicer-runtime/src/region_mapping.rs`

**Objective.** The full kernel (~410 LOC of `execute_region_mapping_inner` plus helpers, plus `RegionMappingError`) lives in `slicer-core`. The `slicer-runtime` file is deleted.

**Precondition.** Step 2 complete.

**Postcondition.** `slicer-core` builds with the real kernel body. `cargo build -p slicer-core` green.

**Files allowed to read.** `crates/slicer-runtime/src/region_mapping.rs` — line-range only. Grep for `execute_region_mapping_inner`, `RegionMappingError`, helpers.
**Files allowed to edit.**
1. `crates/slicer-core/src/algos/region_mapping.rs` — populate with the kernel body. Replace the `&ExecutionPlan` parameter with the projection borrow throughout.
2. Delete `crates/slicer-runtime/src/region_mapping.rs`.

**Expected sub-agent dispatch.**
- Dispatch: `cargo build -p slicer-core`. Return FACT pass/fail.

**Context cost: M.**

**Narrow verification.** Build green. `! grep -qE 'ExecutionPlan' crates/slicer-core/src/algos/region_mapping.rs`.

**Falsifying check / exit condition.** Build fails on missing IR type → check the kernel body for any `slicer_scheduler::*` or `slicer_runtime::*` imports that should be IR-only.

---

## Step 4 — Create wrapper `region_mapping_producer.rs` in slicer-runtime

**Objective.** The wrapper exists and resolves to the new kernel via projection unpack.

**Precondition.** Step 3 complete.

**Postcondition.** `crates/slicer-runtime/src/builtins/region_mapping_producer.rs` exists with the `BuiltinProducer` static + the unpack body, ≤ 100 LOC.

**Files allowed to read.** `crates/slicer-runtime/src/builtins/mod.rs` (other producer patterns), `crates/slicer-scheduler/src/execution_plan.rs` (for the field projections).
**Files allowed to edit.**
1. `crates/slicer-runtime/src/builtins/region_mapping_producer.rs` — CREATE.
2. `crates/slicer-runtime/src/builtins/mod.rs` — add `pub mod region_mapping_producer;` + re-export.

**Expected sub-agent dispatch.**
- (Build check happens in Step 5.)

**Context cost: S.**

**Narrow verification.** File exists; `wc -l ≤ 100`.

---

## Step 5 — Rewire `slicer-runtime/src/lib.rs`; update `runtime_builtins()` order; migrate tests

**Objective.** Workspace builds; tests pass.

**Precondition.** Step 4 complete.

**Postcondition.** `cargo build --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo test --features slicer-core/host-algos --features slicer-sdk/test -p slicer-core -p slicer-runtime -p pnp-cli` all green. AC-N3 holds (every surviving `pub use slicer_core::algos::region_mapping::` re-export carries a `// kept:` annotation; dead re-exports deleted).

**Test-import stability rule (P84/P85/P86 pattern).** `--all-targets` compiles test binaries. Until Step 6 relocates them, runtime tests referencing `slicer_runtime::execute_region_mapping` (or `_with_cap`, or `RegionMappingError`) must still resolve. Retain the necessary `pub use slicer_core::algos::region_mapping::{...};` re-exports as flat top-level lines (NEVER brace-form `pub mod region_mapping { pub use slicer_core::algos::region_mapping::*; }` — forbidden by AC-3 word boundary). Step 6 prunes dead re-exports, annotating survivors with `// kept:`.

**Files allowed to read.** `crates/slicer-runtime/src/lib.rs`. Test files from dispatch #2.
**Files allowed to edit.**
1. `crates/slicer-runtime/src/lib.rs` — drop `pub mod region_mapping;`, add flat `pub use slicer_core::algos::region_mapping::{...};` re-exports per Step 1 dispatch #2's consumer list (these become transitional; Step 6 prunes them), update `runtime_builtins()` to reference `builtins::region_mapping_producer::REGION_MAPPING_PRODUCER`.
2. Test files from dispatch #2 — move or rewire imports.
3. `crates/slicer-runtime/tests/{integration,executor}/main.rs` aggregators — drop `mod` declarations for moved tests.

**Expected sub-agent dispatches.**
- Dispatch: `cargo build --workspace`. Return FACT pass/fail.
- Dispatch: `cargo clippy --workspace --all-targets -- -D warnings`. Return FACT pass/fail.
- Dispatch: `cargo test --features slicer-core/host-algos --features slicer-sdk/test -p slicer-core -p slicer-runtime -p pnp-cli`. Return FACT pass/fail + counts. (Flags mandatory per P85 closure.)

**Context cost: M.**

**Narrow verification.** All green; `[ $(grep -cE '_PRODUCER as &dyn Producer' crates/slicer-runtime/src/lib.rs) -eq 8 ]`.

**Falsifying check / exit condition.** Build/test fails → check Step 1 dispatch outputs for any missed consumer.

---

## Step 6 — Add the AC-8 per-algorithm test in `slicer-core/tests/`

**Objective.** A test exercises `execute_region_mapping` without runtime/scheduler scope.

**Precondition.** Step 5 green.

**Postcondition.** `cargo test -p slicer-core --test algo_region_mapping_tdd` (or equivalent name) passes.

**Files allowed to read.** Output of `cargo build --workspace 2>&1 | grep -E '^error\[E04(32|33)\]'` after each pruning round to find live consumers.
**Files allowed to edit.**
1. `crates/slicer-core/tests/algo_region_mapping_tdd.rs` — CREATE.
2. `crates/slicer-runtime/src/lib.rs` — **prune dead re-exports** (P85/P86 closure-cleanup rule, codified by AC-N3). For each transitional `pub use slicer_core::algos::region_mapping::X;` line added in Step 5, run `rg 'slicer_runtime::X\b' crates/ docs/`. Zero hits → delete the re-export. Hits → keep the line and add a `// kept: consumed by <file>` comment immediately above OR below the line so AC-N3's grep passes. Re-run `cargo build --workspace --all-targets` after each prune to confirm nothing broke.

The test:
- Constructs a small `LayerPlanIR` with 2 layers, 2 objects, 2 regions each.
- Builds a `RegionMappingPlanProjection` from manually-populated `HashMap`s (matching the projection struct's shape).
- Calls `execute_region_mapping_with_cap(&layer_plan, &projection, None, &configs, &objects, DEFAULT_REGION_MAP_CAP)` — or the simpler `execute_region_mapping(...)` delegator.
- Asserts the returned `RegionMapIR` has the expected per-(layer, object, region) shape.
- Imports zero `slicer_runtime::*` and zero `slicer_scheduler::*` types.

**Expected sub-agent dispatch.**
- Dispatch: `cargo test -p slicer-core --test algo_region_mapping_tdd`. Return FACT pass/fail.

**Context cost: S.**

**Narrow verification.** Test passes.

---

## Step 7 — AC-7 g-code SHA parity

**Objective.** Post-packet SHA = Step 0 baseline.

**Precondition.** Step 6 green.

**Postcondition.** SHAs match.

**Files allowed to read.** None.
**Files allowed to edit.** None.

**Expected sub-agent dispatch.**
- Dispatch: `cargo run --bin pnp_cli --release -- slice --model resources/benchy.stl --module-dir modules/core-modules --output /tmp/benchy-p87.gcode && sha256sum /tmp/benchy-p87.gcode`. Return FACT `<hex>`. Compare to Step 0 baseline.

**Context cost: S.**

**Narrow verification.** SHAs match.

**Falsifying check / exit condition.** SHA divergence → bisect; most likely culprit is a wrong field name in the projection unpack (e.g., wrong HashMap reference).

---

## Per-Step Budget Roll-Up

| Step | Cost |
|---|---|
| 0 P85 verify + baseline | S |
| 1 Enumerate consumers + reads | S |
| 2 Scaffold projection + stub | S |
| 3 Move kernel | M |
| 4 Create wrapper | S |
| 5 Rewire runtime lib.rs + tests | M |
| 6 AC-8 test | S |
| 7 SHA parity | S |

Aggregate: **M.** No L step. Total step count: 8.

## Packet Completion Gate

1. `cargo build --workspace` — green.
2. `cargo clippy --workspace --all-targets -- -D warnings` — green.
3. `cargo xtask build-guests --check` — clean.
4. `cargo test --features slicer-core/host-algos --features slicer-sdk/test -p slicer-core -p slicer-runtime -p pnp-cli` — green.
5. AC-7 post-packet SHA = Step 0 baseline.

## Acceptance Ceremony

- All 9 ACs and 2 negative cases gate green.
- No ADR follow-up.
- Implementation log records: Step 0 baseline SHA, Step 7 post-packet SHA, final `RegionMappingPlanProjection` field set, list of moved tests.
- `status: draft` → `status: implemented` after gate green AND user confirms closure. (`superseded` is reserved for packets replaced by a later spec.)
