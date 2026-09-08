# Implementation Plan: 239a-anchored-host-seams

## Execution Rules

- Work one atomic step at a time; map every step to grouped task IDs.
- Use TDD, then implementation, then the narrowest falsifying validation.
- Every field below is a context-budget contract and must be filled independently; never write
  "see Step N".
- Every `cargo test` invocation names one test with `--exact`, tees to
  `target/test-output.log`, and asserts a non-zero matched count in-run (the plan doc's numbered
  §6 item 16). Read the log file for detail; never re-run a test to see more output.
- **Module-prefixed test names (load-bearing).** `crates/slicer-runtime/tests/integration/offgrid_rows_tdd.rs`
  and `pipeline_tdd.rs` are mounted by bare `mod` lines with `#[test]` in place, so libtest names
  their tests `offgrid_rows_tdd::<fn>` and `pipeline_tdd::<fn>`. Verified against a recorded
  `integration`-binary run whose lines read `test pipeline_tdd::access_audits_live_path ... ok`.
  Every `--exact` filter below uses the prefixed form; a bare function name matches zero tests,
  exits 0, and reads green. Only the top-level `#[test] fn` wrappers declared in
  `crates/slicer-runtime/tests/integration/main.rs` — `anchored_event_ordering`,
  `anchored_parallel_determinism`, `anchored_z_validation`, `anchored_z_span_validation`,
  `anchored_event_accounting` — are filtered without a prefix.
- **Plan-doc invariants are cited by quoted phrase, never by ordinal.** In
  `docs/specs/support-families-anchored-entities-plan.md` §6, items 1–14 are an unnumbered
  semicolon-separated prose parenthetical; only 15 and 16 are numbered list items. The phrases
  used below are "same-Z support in ordinary ordering", "Z-spanning atomicity",
  "serial/parallel determinism", "support-disabled emits nothing", and "planar anchored output on
  declared Z".
- **ADR-0059 governs anchor attribution and Z-spanning placement.** See `design.md`
  §ADR Conformance and `docs/adr/0059-support-families-and-anchored-entities.md`.
- `cargo test --workspace` and `cargo xtask test --workspace` appear in **no** step. This packet
  closes on targeted commands plus `cargo check`/`cargo clippy`/`cargo xtask check-literals`.
- `crates/slicer-runtime` has features `default = ["report"]`, `report = []`, no
  `required-features` on any test target, and no test file gated by `#![cfg(feature = ...)]`.
  Do not add `--features` flags on the assumption of feature-gated blindness.
- Before attributing any guest, component, or module-dispatch failure to a step's edits, run
  `cargo xtask build-guests --check` and read its **exit code** (0 fresh / 1 stale / 3
  `wasm-tools` missing). Never grep for `STALE:`.
- All `cargo check` and `cargo clippy` invocations use `--all-targets`.

## Steps

### Step 1: Additive `PipelineConfig.anchored_entities` field and its full blast radius

- Task IDs: `TASK-399`
- Objective: add `pub anchored_entities: Vec<slicer_ir::AnchoredEntity>` to `PipelineConfig`
  (`crates/slicer-runtime/src/pipeline.rs`) and close its **entire** struct-literal blast radius
  in this one step. The field is written but not yet read by any code path — the step is
  behaviour-neutral and must remain so.
- Precondition: clean tree on `parity/support-features`; `cargo check --workspace --all-targets`
  green before any edit. `PipelineConfig` has no `Default` impl and is not `#[non_exhaustive]`,
  so every literal site must name the field or inherit it through FRU.
- Postcondition: `cargo check --workspace --all-targets` and
  `cargo clippy --workspace --all-targets -- -D warnings` are green; `cargo xtask check-literals`
  passes; no test changes colour anywhere; no signature in `pipeline.rs` changed.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-runtime/src/pipeline.rs` - lines `40-200` and `300-360` (the struct plus both
    `let PipelineConfig { ... }` destructuring patterns)
  - `crates/slicer-runtime/src/run.rs` - lines around `run_slice_with_collector` only (file is
    1431 lines; locate the symbol first, then read ±60 lines)
  - `crates/slicer-runtime/tests/common/mod.rs` - `pipeline_config_base` only (file is 604 lines)
  - `crates/pnp-cli/tests/e2e_integration_tdd.rs` - `pipeline_config_base` only (360 lines)
  - `crates/slicer-runtime/tests/contract/dispatch_infill_output_tdd.rs` - only the three
    `PipelineConfig` literals and their `// exhaustive:` waivers; locate each by symbol, then
    read ±20 lines
- Files allowed to edit:
  - `crates/slicer-runtime/src/pipeline.rs`
  - `crates/slicer-runtime/src/run.rs`
  - `crates/slicer-runtime/tests/common/mod.rs`
  - `crates/pnp-cli/tests/e2e_integration_tdd.rs`
  - `crates/slicer-runtime/tests/contract/dispatch_infill_output_tdd.rs`
  The template's three-file cap yields here to the blast-radius clause below: all **five** files
  are required for `cargo check --workspace --all-targets` to pass, and four of them receive one
  to three single-line field initializers each and nothing else.
  **Step-splitting question, resolved: this step is NOT split.** Adding the fifth file grows the
  edit from 3 to 6 one-line initializers; splitting the six sites across two steps would leave
  `cargo check --workspace --all-targets` **red between the two steps**, which the plan's own
  execution rules prohibit and which would make Step 1's postcondition unachievable for the first
  half. The step therefore keeps a single task ID (`TASK-399`) and a single step number, and no
  task ID beyond `TASK-399`..`TASK-408` is minted.
- Files explicitly out of bounds:
  - `crates/slicer-runtime/src/layer_executor.rs` (Step 4 owns it)
  - `crates/slicer-gcode/src/emit.rs`
  - `crates/pnp-cli/src/visual_debug.rs` (Step 8 owns it)
  - `crates/slicer-schema/wit/**`, `modules/**`, all other packet directories
- Blast-radius discipline (mandatory: this step adds a struct field):
  - Measured 2026-08-28 (`rg -n 'PipelineConfig \{' crates/`, then classifying each hit by whether
    its body carries a `..` rest): `PipelineConfig` has **33** construction literals in the
    workspace (1 production, 32 test). **27** use FRU (`..`) over the two `pipeline_config_base`
    base helpers and are not edited. **6 are exhaustive** and every one must gain the new field
    initializer — note that an `// exhaustive:` waiver satisfies `check-literals` but does **not**
    make a literal FRU:
    1. `run_slice_with_collector` (`crates/slicer-runtime/src/run.rs`) — the only production
       literal
    2. `pipeline_config_base` (`crates/slicer-runtime/tests/common/mod.rs`)
    3. `pipeline_config_base` (`crates/pnp-cli/tests/e2e_integration_tdd.rs`)
    4.–6. three literals in `crates/slicer-runtime/tests/contract/dispatch_infill_output_tdd.rs`
    **Sites 4–6 were missing from this packet's first draft.** They live in the `contract` test
    binary, not `integration`, which is exactly why an integration-scoped sweep did not see them;
    without them this step's `cargo check --workspace --all-targets` postcondition cannot be met.
    Sweep every `slicer-runtime` test binary — `unit`, `contract`, `executor`, `integration` —
    plus `pnp-cli`, not just `integration`.
  - **Plus two destructuring patterns**, which a literal-site sweep does not find: the
    exhaustive `let PipelineConfig { ... } = config;` in `run_pipeline_with_events` and the one
    in `run_pipeline_core` (both `crates/slicer-runtime/src/pipeline.rs`). Neither uses a `..`
    rest today. Each must either bind the new field or `..`-ignore it. Missing one is a compile
    error, not a silent bug — but budget it here, do not let a follow-up `cargo check` discover
    it.
  - The `LOCATIONS` worker result cited above was captured at packet authoring; re-derive it at
    edit time (`PipelineConfig` sites are mutable shared state) and reconcile any difference
    before editing.
- Expected sub-agent dispatches:
  - Question: enumerate every `PipelineConfig { ... }` literal and every
    `let PipelineConfig { ... }` pattern in the workspace, marking FRU (body contains a `..` rest)
    vs exhaustive (it does not — an `// exhaustive:` comment is not a `..`) and naming the
    enclosing function and its test binary; cover **all** `slicer-runtime` test binaries
    (`unit`, `contract`, `executor`, `integration`) and `pnp-cli`, not only `integration`; scope:
    `crates/ --include *.rs`; return: `LOCATIONS` ≤20 entries
  - Question: does any test assert on `PipelineConfig`'s field count, field order, or `Debug`
    output; scope: `crates/ --include *.rs`; return: `FACT`
- Context cost: `M`
- Authoritative docs:
  - `docs/specs/support-independent-layer-z-split-plan.md` - finding F3 (no injection seam);
    direct read, 152 lines
  - `docs/21_data_defaults_and_fixtures.md` - delegated `SUMMARY` of the FRU/waiver rule for
    watched struct types
- OrcaSlicer refs:
  - none for this step; no canonical behaviour is ported here
- Verification:
  - `cargo check --workspace --all-targets` - FACT pass/fail
  - `cargo clippy --workspace --all-targets -- -D warnings` - FACT pass/fail
  - `cargo xtask check-literals` - FACT pass/fail
  - `cargo test -p slicer-runtime --test integration -- anchored_event_ordering --exact 2>&1 | tee target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` - FACT pass/fail; proves the additive field changed no anchored behaviour
- Exit condition: all four commands pass, and the diff contains exactly one new struct field,
  **six** field initializers (across five files), and two updated destructuring patterns —
  nothing else. Falsifying
  exit: any `pipeline.rs` signature differs from its pre-step text, or
  `crates/slicer-runtime/tests/visual_debug_agent_overhead_tdd.rs` goes red.

### Step 2: Payload-capturing `GCodeEmitter` fixture and the AC-6 baseline record

- Task IDs: `TASK-400`
- Objective: author `CapturedRowsEmitter` — a `GCodeEmitter` impl holding
  `Arc<Mutex<Vec<LayerCollectionIR>>>` that clones and stores every row handed to `emit_gcode`,
  with an accessor returning the captured sequence — in
  `crates/slicer-runtime/tests/common/mod.rs`; then prove it on an **existing support-free
  pipeline run** by adding `payload_capturing_emitter_records_row_sequence` to
  `crates/slicer-runtime/tests/integration/pipeline_tdd.rs`. That test also records AC-6's
  pre-change baseline: the exact `(len, global_layer_index, z)` sequence the support-free run
  produces **today**, before any executor switch exists.
- Precondition: Step 1 landed. No existing mock stores the `&[LayerCollectionIR]` payload —
  `LayerCountEmitter` (in both `crates/slicer-runtime/tests/integration/pipeline_tdd.rs` and
  `crates/pnp-cli/tests/e2e_integration_tdd.rs`) records only `.len()`, and
  `OrderTrackingEmitter` ignores the rows. The `PipelineStageRunners.emitter: Box<dyn GCodeEmitter>`
  injection seam is already proven by those two fixtures.
- Postcondition: `payload_capturing_emitter_records_row_sequence` passes, asserting a non-empty
  captured sequence with the concrete baseline values inlined in the test source. The baseline
  is now committed and cannot be back-fitted to post-switch behaviour.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-runtime/tests/integration/pipeline_tdd.rs` - the `LayerCountEmitter` and
    `OrderTrackingEmitter` definitions plus one representative pipeline test that builds a
    `PipelineStageRunners` (file is 1533 lines; do not full-read)
  - `crates/slicer-runtime/tests/common/mod.rs` - `pipeline_config_base` and neighbouring
    fixture helpers (604 lines)
  - `crates/slicer-ir/src/slice_ir.rs` - the `LayerCollectionIR` struct definition only
    (3141 lines; ranged read)
- Files allowed to edit (at most 3):
  - `crates/slicer-runtime/tests/common/mod.rs`
  - `crates/slicer-runtime/tests/integration/pipeline_tdd.rs`
- Files explicitly out of bounds:
  - `crates/slicer-gcode/src/emit.rs` — the trait is frozen; implement against it, do not read it
  - every `src/` file in the workspace this step
  - `crates/slicer-runtime/tests/integration/main.rs` (`pipeline_tdd` is already mounted)
- Blast-radius discipline: not applicable — no struct field and no schema constant is added.
  The new fixture is a test-only type. Its `LayerCollectionIR` comparisons must not construct
  new literals without FRU or an `// exhaustive:` waiver (`cargo xtask check-literals`).
- Expected sub-agent dispatches:
  - Question: which `GCodeEmitter` impls exist under `crates/slicer-runtime/tests/` and what is
    the exact method signature they implement; scope: `crates/slicer-runtime/tests/`; return:
    `LOCATIONS` ≤20 entries plus the one-line signature
- Context cost: `S`
- Authoritative docs:
  - `docs/21_data_defaults_and_fixtures.md` - delegated `SUMMARY`; the struct-literal churn gate
    applies to the new fixture's test literals
- OrcaSlicer refs:
  - none for this step
- Verification:
  - `cargo test -p slicer-runtime --test integration -- pipeline_tdd::payload_capturing_emitter_records_row_sequence --exact 2>&1 | tee target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` - FACT pass/fail
  - `cargo xtask check-literals` - FACT pass/fail
- Exit condition: the captured sequence is non-empty and its concrete values appear literally in
  the test source as the AC-6 baseline. Falsifying exit: the test passes while asserting only
  `captured.len() > 0` — that reproduces `LayerCountEmitter` and proves nothing new.

### Step 3: Red-first pipeline tests for AC-1, AC-2, and AC-N2

- Task IDs: `TASK-401`
- Objective: create `crates/slicer-runtime/tests/integration/offgrid_rows_tdd.rs` with three
  failing tests — `offgrid_support_row_emitted_at_declared_z` (AC-1),
  `every_same_z_support_entity_routes_exactly_once` (AC-2), and
  `offgrid_entity_never_merged_into_grid_layers` (AC-N2) — each driving a full pipeline run
  through `run_pipeline_with_instrumentation` with a hand-built `ExecutionPlan`, an explicit
  `PipelineConfig.anchored_entities` payload, and `CapturedRowsEmitter`. Mount the file with one
  `mod offgrid_rows_tdd;` line in `crates/slicer-runtime/tests/integration/main.rs`.
- Precondition: Steps 1 and 2 landed. `crates/slicer-runtime/tests/integration/main.rs` is the
  `integration` binary aggregator (`[[test]] name = "integration"`) with 69 top-level `mod`
  declarations, a `#[path]`-mounted `common`, and 22 inline `#[test] fn` wrappers. The
  `anchored_*` family uses wrappers; `pipeline_tdd` does not. This file follows `pipeline_tdd`:
  its functions carry `#[test]` in place, so a bare `mod` line is the entire mount and no
  wrapper is written.
- Postcondition: all three tests exist, compile, and **fail** — specifically because the
  captured row sequence contains no row at the declared off-grid Z. No production file is edited.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-runtime/tests/integration/main.rs` - full (183 lines)
  - `crates/slicer-runtime/tests/integration/anchored_event_ordering.rs` - full; the idiom for
    building an `ExecutionPlan` with anchored entities
  - `crates/slicer-runtime/tests/integration/pipeline_tdd.rs` - the representative pipeline test
    and Step 2's new fixture usage only (1533 lines; ranged)
  - `crates/slicer-ir/src/slice_ir.rs` - `AnchoredEntity`, `AnchoredGeometryContract`, the
    `COORDINATE_TOLERANCE_UNITS` constant and its `AnchoredGeometryContract` re-export, and
    `mm_to_units` only (3141 lines; ranged)
- Files allowed to edit (at most 3):
  - `crates/slicer-runtime/tests/integration/offgrid_rows_tdd.rs` (new)
  - `crates/slicer-runtime/tests/integration/main.rs`
- Files explicitly out of bounds:
  - every `src/` file in the workspace this step
  - `crates/slicer-runtime/tests/integration/anchored_*.rs` (regression guards; read, never edit)
- Blast-radius discipline: not applicable — no struct field, no schema constant. New
  `LayerCollectionIR` / `PipelineConfig` test literals need FRU or an `// exhaustive:` waiver.
- Expected sub-agent dispatches:
  - Question: confirm `main.rs` mounts `pipeline_tdd` with a bare `mod` line and no inline
    `#[test] fn` wrapper, and list the wrapper functions that do exist; scope:
    `crates/slicer-runtime/tests/integration/main.rs`; return: `FACT` plus ≤20 `LOCATIONS`
- Context cost: `M`
- Authoritative docs:
  - `docs/specs/support-families-anchored-entities-plan.md` - §6, the "same-Z support in
    ordinary ordering" invariant (by phrase) and the numbered list item 16; delegated `SUMMARY`,
    never full-read (755 lines)
  - `docs/08_coordinate_system.md` - consulted only through the coord-system constraint in
    `design.md`; the fixture plane `z: 3000` is 0.3 mm at 1 unit = 100 nm
- OrcaSlicer refs:
  - none for this step; the merge rule is Step 5's obligation
- Verification:
  - `cargo test -p slicer-runtime --test integration -- offgrid_rows_tdd::offgrid_support_row_emitted_at_declared_z --exact 2>&1 | tee target/test-output.log && grep -qE '^test .* FAILED|panicked at' target/test-output.log` - FACT: red state proven
  - `cargo test -p slicer-runtime --test integration -- offgrid_rows_tdd::every_same_z_support_entity_routes_exactly_once --exact 2>&1 | tee target/test-output.log && grep -qE '^test .* FAILED|panicked at' target/test-output.log` - FACT: red state proven
  - `cargo test -p slicer-runtime --test integration -- offgrid_rows_tdd::offgrid_entity_never_merged_into_grid_layers --exact 2>&1 | tee target/test-output.log && grep -qE '^test .* FAILED|panicked at' target/test-output.log` - FACT: red state proven
  - `cargo test -p slicer-runtime --test integration -- anchored_event_ordering --exact 2>&1 | tee target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` - FACT pass/fail; the pre-existing guard stays green
- Exit condition: exactly the three authored tests fail, each with an assertion message naming
  the missing declared-Z row; zero unrelated failures; `cargo check --workspace --all-targets`
  clean. **Falsifying exit: any of the three passes before implementation lands.** A green
  pre-implementation test means it is not testing the gap — per finding F1 the executor routing
  partition is already total, so a test written at executor level rather than pipeline level
  will pass vacuously. Rewrite it against the emitter-captured row sequence.

### Step 4: Behaviour-neutral shared route-partition helper

- Task IDs: `TASK-402`
- Objective: replace the complementary `is_same_z_entity` / `!is_same_z_entity` filter pair with
  one shared named helper in `crates/slicer-runtime/src/layer_executor.rs`, so the totality of
  the routing partition is expressed in the code rather than inferred from two distant call
  sites. **This step flips no acceptance criterion.**
- Precondition: Steps 1–3 landed; Step 3's three tests are red. Finding F1 is established:
  `is_same_z_entity` has exactly three references — its definition, the positive filter in
  `append_same_z_entities`, and the negated filter in `execute_anchored_event_collections` —
  and the two filters are exact complements, so off-grid entities **already** reach the anchored
  collection today.
- Postcondition: the helper exists, both call sites consume it, and every test in the crate has
  the same colour as before the step. Step 3's three tests are **still red**; the pre-existing
  `anchored_*` suite is still green.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-runtime/src/layer_executor.rs` - lines `190-470` (the anchored entry points,
    `CommittedLayerEvent`, `is_same_z_entity`, `append_same_z_entities`) and lines `2470-2600`
    (`execute_anchored_event_collections` and its `_with_accounting` / `_with_mode` /
    `_with_mode_and_feedrate` siblings). File is 3886 lines — never full-read.
- Files allowed to edit (at most 3):
  - `crates/slicer-runtime/src/layer_executor.rs`
- Files explicitly out of bounds:
  - `crates/slicer-runtime/src/pipeline.rs` (Steps 6 and 7 own it)
  - `crates/slicer-runtime/tests/integration/offgrid_rows_tdd.rs` (Step 3's red tests must not
    be edited to accommodate a refactor)
  - `crates/pnp-cli/**`, `modules/**`, `crates/slicer-schema/wit/**`
- Blast-radius discipline: not applicable — no struct field, no schema constant, and the helper
  stays private (see `design.md` §Open Questions for the placement recommendation).
- Expected sub-agent dispatches:
  - Question: after the refactor, confirm the workspace contains no remaining reference to the
    old private predicate name outside its new definition; scope: `crates/ --include *.rs`;
    return: `FACT` plus ≤20 `LOCATIONS`
- Context cost: `S`
- Authoritative docs:
  - `docs/specs/support-independent-layer-z-split-plan.md` - finding F1 verbatim; direct read
- OrcaSlicer refs:
  - none for this step
- Verification:
  - `cargo test -p slicer-runtime --test integration -- anchored_event_ordering --exact 2>&1 | tee target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` - FACT pass/fail
  - `cargo test -p slicer-runtime --test integration -- anchored_parallel_determinism --exact 2>&1 | tee target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` - FACT pass/fail
  - `cargo test -p slicer-runtime --test integration -- anchored_z_validation --exact 2>&1 | tee target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` - FACT pass/fail
  - `cargo clippy --workspace --all-targets -- -D warnings` - FACT pass/fail
- Exit condition: the three anchored guards are green and Step 3's three tests are still red.
  **Falsifying exit: any AC command changes colour across this step.** That would mean the
  extraction was not behaviour-equivalent — revert it and re-derive from F1 rather than
  reporting a fix.

### Step 5: Pure row synthesis matching the canonical epsilon merge

- Task IDs: `TASK-403`
- Objective: author `crates/slicer-runtime/src/anchored_rows.rs` with
  `pub fn synthesize_anchored_rows` turning a `Vec<CommittedLayerEvent>` into an ordered
  `Vec<LayerCollectionIR>`, declare `pub mod anchored_rows;` in
  `crates/slicer-runtime/src/lib.rs`, and prove it two ways: in-module `#[cfg(test)]` unit tests
  and the AC-5 integration test `offgrid_row_merge_matches_canonical_epsilon_rule`, which calls
  the function directly (no pipeline run needed) using
  `slicer_runtime::layer_executor::CommittedLayerEvent`.
- Precondition: Steps 1–4 landed. `CommittedLayerEvent` is a `pub enum` with exactly two
  variants, `Anchored(slicer_ir::OrderedEventCollection)` and `Model(LayerCollectionIR)`; it is
  **not** re-exported from `crates/slicer-runtime/src/lib.rs`, so tests name it
  `slicer_runtime::layer_executor::CommittedLayerEvent`.
- Postcondition: merge iff `|dz| <= COORDINATE_TOLERANCE_UNITS` in i64 units, otherwise the
  lower Z emits a solo row and the other side retries; merged rows keep the object row's `z` and
  `global_layer_index`; **a solo synthesized row adopts the `global_layer_index` of the UPPER
  global layer — the `Model` row that immediately follows it in ascending Z — per
  `docs/adr/0059-support-families-and-anchored-entities.md` ("anchored to the upper global
  layer"), falling back to the last `Model` row's index only when no upper `Model` row exists**;
  a `ZSpanning` entity is **not** given a row of its own — its paths go as one contiguous block
  into its anchor layer's ordinary `Model` row (ADR-0059's "at that layer's normal position");
  every synthesized row sets `schema_version: CURRENT_LAYER_COLLECTION_IR_SCHEMA_VERSION` read
  from the live constant, **not** a bumped or hard-coded value. AC-5's command passes.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-runtime/src/layer_executor.rs` - lines `250-300` only (the
    `CommittedLayerEvent` enum and the committed entry point's return tuple). 3886 lines; never
    full-read.
  - `crates/slicer-ir/src/slice_ir.rs` - `LayerCollectionIR`,
    `CURRENT_LAYER_COLLECTION_IR_SCHEMA_VERSION`, `COORDINATE_TOLERANCE_UNITS`, `mm_to_units`,
    `AnchoredGeometryContract`, `OrderedEventCollection` only (3141 lines; ranged)
  - `crates/slicer-runtime/src/lib.rs` - the `mod` declaration block only
- Files allowed to edit (at most 3):
  - `crates/slicer-runtime/src/anchored_rows.rs` (new)
  - `crates/slicer-runtime/src/lib.rs`
  - `crates/slicer-runtime/tests/integration/offgrid_rows_tdd.rs`
- Files explicitly out of bounds:
  - `crates/slicer-runtime/src/pipeline.rs` (Steps 6 and 7 wire the function in)
  - `crates/slicer-gcode/src/emit.rs`
  - `crates/slicer-ir/src/slice_ir.rs` — read-only; no IR field and no version bump in this
    packet
  - `crates/slicer-runtime/tests/integration/main.rs` (already mounted in Step 3)
- Blast-radius discipline: not applicable — no struct field is added and no version constant is
  bumped. The step **reads** `CURRENT_LAYER_COLLECTION_IR_SCHEMA_VERSION`; it must not write it.
  New `LayerCollectionIR` literals in the unit tests need FRU or an `// exhaustive:` waiver.
- Expected sub-agent dispatches:
  - Question: re-verify canonical `collect_layers_to_print`'s merge discipline — the two-index
    walk, `print_z_min` selection, the un-consume condition, what EPSILON is compared against,
    and which side emits when they do not merge; scope:
    `OrcaSlicerDocumented/src/libslic3r/GCode.cpp`; return: `SUMMARY` ≤200 words or `SNIPPETS`
    ≤30 lines; never a file body
- Context cost: `M`
- Authoritative docs:
  - `docs/specs/support-independent-layer-z-split-plan.md` - §Canonical OrcaSlicer reference,
    the `collect_layers_to_print` bullet; direct read
  - `docs/02_ir_schemas.md` - delegated `FACT`: read the live value of
    `CURRENT_LAYER_COLLECTION_IR_SCHEMA_VERSION` and confirm synthesized rows reuse it rather than
    bump it. Do **not** pin a version literal in code, tests, or this packet — it is a mutable
    ledger fact
  - `docs/08_coordinate_system.md` - delegated `SUMMARY`; comparisons happen in i64 units after
    one `mm_to_units` conversion at the boundary
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/GCode.cpp` - `collect_layers_to_print`; delegate, never
    load
- Verification:
  - `cargo test -p slicer-runtime --lib -- anchored_rows::tests::merge_within_epsilon_produces_one_row --exact 2>&1 | tee target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` - FACT pass/fail
  - `cargo test -p slicer-runtime --lib -- anchored_rows::tests::beyond_epsilon_lower_z_emits_solo_row --exact 2>&1 | tee target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` - FACT pass/fail
  - `cargo test -p slicer-runtime --test integration -- offgrid_rows_tdd::offgrid_row_merge_matches_canonical_epsilon_rule --exact 2>&1 | tee target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` - FACT pass/fail (AC-5)
  - `cargo xtask check-literals` - FACT pass/fail
- Exit condition: AC-5 passes and both unit tests pass, with the merge threshold sourced from
  `COORDINATE_TOLERANCE_UNITS` rather than a literal. Step 3's three pipeline tests are **still
  red** — nothing is wired yet. Falsifying exit: any test passes while the function hard-codes a
  numeric epsilon or a `SemVer` literal.

### Step 6: Switch `run_pipeline_core` and insert synthesized rows at the finalization seam

- Task IDs: `TASK-404`
- Objective: in `run_pipeline_core` (`crates/slicer-runtime/src/pipeline.rs`), replace
  `execute_per_layer_with_instrumentation_and_support_tools` with
  `execute_per_layer_with_committed_anchored_events` passing `&anchored_entities`, feed the
  resulting `Vec<CommittedLayerEvent>` through `synthesize_anchored_rows`, and bind the result as
  the `layer_irs` that flows into `execute_layer_finalization_with_instrumentation`. AC-1, AC-2,
  and AC-N2 go green.
- Precondition: Steps 1–5 landed; Step 3's three tests are red; AC-5 is green. The seam order in
  `run_pipeline_core` is: (1) the per-layer call producing
  `let (mut layer_irs, layer_audits) = ...`, (2)
  `execute_layer_finalization_with_instrumentation(..., &mut layer_irs, ...)` — the **last
  mutable seam** — and (3) `run_postpass_with_thumbnail(..., &layer_irs, ...)`, which passes an
  immutable slice to `slicer_runtime::postpass::execute_postpass_with_capture`, and that function
  deep-copies with `layer_irs.to_vec()` before `.emit_gcode`. Rows inserted at or before (2)
  therefore reach emission; rows inserted after (2) do not.
- Postcondition: AC-1, AC-2, AC-N2 pass. `run_pipeline_core`'s signature is byte-identical to
  its pre-step text, and both source-text guard tests stay green.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-runtime/src/pipeline.rs` - lines `300-520` (the `run_pipeline_core` body
    through `run_postpass_with_thumbnail`)
  - `crates/slicer-runtime/src/postpass.rs` - `execute_postpass_with_capture` only (518 lines);
    confirm the `layer_irs.to_vec()` deep copy precedes `.emit_gcode`
  - `crates/slicer-runtime/src/layer_executor.rs` - lines `250-300` only (committed entry point
    signature and return tuple). Never full-read.
- Files allowed to edit (at most 3):
  - `crates/slicer-runtime/src/pipeline.rs`
- Files explicitly out of bounds:
  - `crates/slicer-runtime/tests/integration/offgrid_rows_tdd.rs` — Step 3's assertions must not
    be relaxed to make this step pass
  - `crates/slicer-gcode/src/emit.rs`, `crates/slicer-runtime/src/anchored_rows.rs` (Step 5's
    contract is fixed)
  - `crates/pnp-cli/src/visual_debug.rs` (Step 8)
  - **Conditional:** `crates/slicer-runtime/tests/visual_debug_agent_overhead_tdd.rs` and
    `crates/pnp-cli/tests/visual_debug_typed_tap_capture_tdd.rs` are out of bounds **as long as
    no signature changes**. These two files assert the `pipeline.rs` entry-point and
    `run_pipeline_core` signature strings **verbatim** as source text. The design keeps every
    signature unchanged (the additive field arrives inside the by-value `PipelineConfig`), so
    they act as tripwires. If a signature change becomes unavoidable, this step must add **both**
    files to its edit list and both of the guard commands below to its verification set before
    making the change — never defer the fallout.
- Blast-radius discipline: not applicable — no struct field and no schema constant is added
  here; Step 1 already closed the `PipelineConfig` radius.
- Expected sub-agent dispatches:
  - Question: list every `.emit_gcode(` call site reachable from production code (excluding
    tests) and confirm `slicer_runtime::postpass::execute_postpass_with_capture` is the only one;
    scope: `crates/ --include *.rs`; return: `FACT` plus the single production path
- Context cost: `M`
- Authoritative docs:
  - `docs/specs/support-independent-layer-z-split-plan.md` - findings F2 and F4; direct read
  - `docs/specs/support-families-anchored-entities-plan.md` - §6, the "same-Z support in
    ordinary ordering" invariant (cited by phrase; §6 items 1-14 are unnumbered prose and
    positional item 6 is a different rule); delegated `SUMMARY`, never full-read
- OrcaSlicer refs:
  - none for this step; the canonical rule was pinned in Step 5
- Verification:
  - `cargo test -p slicer-runtime --test integration -- offgrid_rows_tdd::offgrid_support_row_emitted_at_declared_z --exact 2>&1 | tee target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` - FACT pass/fail (AC-1)
  - `cargo test -p slicer-runtime --test integration -- offgrid_rows_tdd::every_same_z_support_entity_routes_exactly_once --exact 2>&1 | tee target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` - FACT pass/fail (AC-2)
  - `cargo test -p slicer-runtime --test integration -- offgrid_rows_tdd::offgrid_entity_never_merged_into_grid_layers --exact 2>&1 | tee target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` - FACT pass/fail (AC-N2)
  - `cargo test -p slicer-runtime --test integration -- anchored_event_ordering --exact 2>&1 | tee target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` - FACT pass/fail (AC-N1)
  - `cargo test -p slicer-runtime --test visual_debug_agent_overhead_tdd 2>&1 | tee target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` - FACT pass/fail; signature tripwire
  - `cargo test -p pnp-cli --test visual_debug_typed_tap_capture_tdd 2>&1 | tee target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` - FACT pass/fail; signature tripwire
- Exit condition: AC-1, AC-2, AC-N2, and AC-N1 all pass, and both signature tripwires are green
  with no edit to either guard file. Falsifying exit: a tripwire goes red while the guard files
  are absent from this step's edit list — that means a signature was changed without owning the
  fallout, and the step must be redone.

### Step 7: Switch the duplicated `run_pipeline_with_events` seam; prove empty-collection equivalence

- Task IDs: `TASK-405`
- Objective: apply the identical switch and insertion to `run_pipeline_with_events`
  (`crates/slicer-runtime/src/pipeline.rs`), whose non-instrumented sequence is
  `execute_per_layer_with_events_and_support_tools` → `execute_layer_finalization` →
  `execute_postpass`; then author `support_free_slice_row_sequence_unchanged` (AC-6) and
  `support_disabled_pipeline_emits_no_anchored_rows` (AC-N3) in
  `crates/slicer-runtime/tests/integration/offgrid_rows_tdd.rs`.
- Precondition: Step 6 landed. `run_pipeline_with_events` does **not** forward to
  `run_pipeline_core` — an inline NOTE comment in its body records that it deliberately keeps a
  separate body
  because it emits a bare G-code body with no thumbnail/CONFIG_BLOCK serializer wrapper. Step 2
  committed AC-6's pre-change baseline; this step compares against that recorded baseline, not
  against a freshly captured one.
- Postcondition: AC-6 and AC-N3 pass. Both entry points call the same
  `synthesize_anchored_rows`; neither carries a second, divergent insertion form. The
  bare-body-vs-CONFIG_BLOCK difference between the two entry points is preserved.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-runtime/src/pipeline.rs` - lines `183-280` (the `run_pipeline_with_events`
    body) and lines `380-460` (Step 6's landed shape in `run_pipeline_core`, to copy it exactly)
  - `crates/slicer-runtime/tests/integration/pipeline_tdd.rs` - Step 2's
    `payload_capturing_emitter_records_row_sequence` only, for the recorded baseline values
    (1533 lines; ranged)
- Files allowed to edit (at most 3):
  - `crates/slicer-runtime/src/pipeline.rs`
  - `crates/slicer-runtime/tests/integration/offgrid_rows_tdd.rs`
- Files explicitly out of bounds:
  - `crates/slicer-runtime/tests/integration/pipeline_tdd.rs` — the AC-6 baseline is committed
    and must not be edited to match new behaviour
  - `crates/slicer-runtime/src/anchored_rows.rs` — reuse Step 5's function; do not fork it
  - `crates/slicer-gcode/src/emit.rs`, `crates/pnp-cli/src/visual_debug.rs`
  - the two source-text signature guard files, under the same conditional rule as Step 6
- Blast-radius discipline: not applicable — no struct field, no schema constant.
- Expected sub-agent dispatches:
  - Question: confirm no production or test caller depends on `run_pipeline_with_events`
    returning a row sequence that differs from `run_pipeline_core`'s beyond the documented
    bare-body serializer difference; scope: `crates/ --include *.rs`; return: `FACT` plus ≤20
    `LOCATIONS`
- Context cost: `S`
- Authoritative docs:
  - `docs/specs/support-families-anchored-entities-plan.md` - §6, the "support-disabled emits
    nothing" invariant (cited by phrase, not ordinal); delegated `SUMMARY`, never full-read
- OrcaSlicer refs:
  - none for this step
- Verification:
  - `cargo test -p slicer-runtime --test integration -- offgrid_rows_tdd::support_free_slice_row_sequence_unchanged --exact 2>&1 | tee target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` - FACT pass/fail (AC-6)
  - `cargo test -p slicer-runtime --test integration -- offgrid_rows_tdd::support_disabled_pipeline_emits_no_anchored_rows --exact 2>&1 | tee target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` - FACT pass/fail (AC-N3)
  - `cargo test -p slicer-runtime --test integration -- pipeline_tdd::payload_capturing_emitter_records_row_sequence --exact 2>&1 | tee target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` - FACT pass/fail; the Step 2 baseline test still passes unchanged
  - `cargo clippy --workspace --all-targets -- -D warnings` - FACT pass/fail
- Exit condition: AC-6 and AC-N3 pass while Step 2's baseline test is unmodified and still
  green. Falsifying exit: AC-6 passes only after the Step 2 baseline literal was edited — that
  is back-fitting, and the empty-collection equivalence claim would be worthless.

### Step 8: Switch the `pnp-cli` visual-debug call site

- Task IDs: `TASK-406`
- Objective: apply the same switch and synthesis insertion in `crates/pnp-cli/src/visual_debug.rs`,
  whose sequence is `slicer_runtime::layer_executor::execute_per_layer_with_events_and_support_tools`
  → `slicer_runtime::execute_layer_finalization` →
  `slicer_runtime::postpass::execute_postpass_with_capture`. This is the third non-anchored call
  site recorded by finding F2 and never recorded by packet 239.
- Precondition: Steps 6 and 7 landed, so `synthesize_anchored_rows` is public
  (`slicer_runtime::anchored_rows`) and its shape is settled by two in-crate consumers.
- Postcondition: all three non-anchored call sites now use the committed anchored variant; the
  visual-debug suite is green; `cargo check --workspace --all-targets` and
  `cargo clippy --workspace --all-targets -- -D warnings` pass.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/pnp-cli/src/visual_debug.rs` - lines `1120-1200` only (the per-layer →
    finalization → postpass sequence). File is 2340 lines — never full-read.
  - `crates/slicer-runtime/src/pipeline.rs` - lines `380-460` (Step 6's landed shape, to mirror)
- Files allowed to edit (at most 3):
  - `crates/pnp-cli/src/visual_debug.rs`
- Files explicitly out of bounds:
  - everything under `crates/slicer-runtime/src/` (Steps 1–7 settled it)
  - `crates/slicer-gcode/src/emit.rs`
  - `crates/pnp-cli/tests/visual_debug_typed_tap_capture_tdd.rs`, under the same conditional
    signature rule as Step 6 — it asserts `pipeline.rs` signature strings verbatim and must be
    adopted into this step's edit list if and only if a signature changes
- Blast-radius discipline: not applicable — no struct field, no schema constant. Note that
  `visual_debug.rs` does not construct a `PipelineConfig`; it calls the executor functions
  directly, so it needs an anchored-entity source of its own (an empty slice is the correct
  behaviour-preserving choice unless a visual-debug option supplies one).
- Expected sub-agent dispatches:
  - Question: which `pnp-cli` test binaries exercise the `visual_debug.rs` per-layer path, and
    what are their exact test names; scope: `crates/pnp-cli/tests/`; return: `LOCATIONS` ≤20
    entries
- Context cost: `S`
- Authoritative docs:
  - `docs/specs/support-independent-layer-z-split-plan.md` - finding F2, the third call site;
    direct read
  - `docs/19_visual_debug.md` - delegated `SUMMARY` only if the visual-debug bundle contract
    turns out to be affected; the expectation is that it is not
- OrcaSlicer refs:
  - none for this step
- Verification:
  - `cargo check --workspace --all-targets` - FACT pass/fail
  - `cargo test -p pnp-cli --test visual_debug_typed_tap_capture_tdd 2>&1 | tee target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` - FACT pass/fail
  - `cargo clippy --workspace --all-targets -- -D warnings` - FACT pass/fail
  - `cargo xtask build-guests --check` - FACT exit code (0 fresh / 1 stale / 3 `wasm-tools`
    missing); run this **before** attributing any visual-debug failure to this step
- Exit condition: no `execute_per_layer_with_events_and_support_tools` or
  `execute_per_layer_with_instrumentation_and_support_tools` call remains in any production
  file, confirmed by a `LOCATIONS` sweep; the visual-debug suite is green. Falsifying exit: the
  sweep still finds a production caller of a non-anchored variant.

### Step 9: Determinism (AC-3) and Z-spanning atomicity (AC-4)

- Task IDs: `TASK-407`
- Objective: author `offgrid_row_order_identical_serial_and_parallel` (AC-3) and
  `zspanning_support_entity_emits_atomic_single_block` (AC-4) in
  `crates/slicer-runtime/tests/integration/offgrid_rows_tdd.rs`.

  **AC-3 is scoped at the executor call, not the pipeline.** There is no `force_parallel` config
  key, env var, or `PipelineConfig` field, and this packet creates none — pipeline-level parallel
  determinism is explicitly out of scope (`packet.spec.md` §Scope Boundaries). `force_parallel` is
  a positional `bool` parameter of
  `slicer_runtime::layer_executor::execute_anchored_event_collections_with_mode(plan, entities, force_parallel, module)`,
  which forwards to the private `execute_anchored_event_collections_with_mode_and_feedrate`. AC-3
  therefore mirrors `crates/slicer-runtime/tests/integration/anchored_parallel_determinism.rs`: it
  calls that function with `false` and again with `true` over an `ExecutionPlan` carrying off-grid
  entities at three distinct intermediate planes, lowers each returned collection sequence through
  `synthesize_anchored_rows` against the **same fixed** `CommittedLayerEvent::Model` rows, and
  compares the full `(z, global_layer_index)` pair sequence and the per-row entity ordering.

  **AC-4 asserts the ADR-0059 placement.** It drives a `AnchoredGeometryContract::ZSpanning`
  `same-z-support` entity spanning several object layers and asserts its paths form **one
  contiguous block inside its anchor layer's ordinary row** (`CommittedLayerEvent::Model`'s
  `ordered_entities`, at that layer's normal position) with **no** separate synthesized row for
  it, and never per-object-layer fragments. Quote to check against:
  `docs/adr/0059-support-families-and-anchored-entities.md` — "A future atomic Z-spanning entity
  may extend outside its anchor layer's Z interval while still executing at that layer's normal
  position."
- Precondition: Steps 1–8 landed; AC-1, AC-2, AC-5, AC-6, AC-N1, AC-N2, AC-N3 are green. The
  pre-existing `anchored_parallel_determinism` test already guards the executor's committed
  ordering; AC-3 extends that guarantee through row synthesis to the emitted sequence.
- Postcondition: AC-3 and AC-4 pass. AC-3 also pins the locked `global_layer_index` rule for
  solo synthesized rows (**the upper anchor layer's index**, per ADR-0059), because a
  nondeterministic or wrongly-attributed index fails the pair-sequence comparison.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-runtime/tests/integration/anchored_parallel_determinism.rs` - full; the
    `force_parallel` idiom
  - `crates/slicer-runtime/tests/integration/anchored_z_span_validation.rs` - full; the
    `ZSpanning` fixture idiom
  - `crates/slicer-runtime/tests/integration/offgrid_rows_tdd.rs` - full (this packet's own file)
  - `crates/slicer-ir/src/slice_ir.rs` - `AnchoredGeometryContract::ZSpanning` only (3141 lines;
    ranged)
- Files allowed to edit (at most 3):
  - `crates/slicer-runtime/tests/integration/offgrid_rows_tdd.rs`
- Files explicitly out of bounds:
  - every `src/` file in the workspace — if AC-3 or AC-4 fails, the fix is a defect report
    against Step 5's synthesis contract, not a production edit smuggled into a test step
  - `crates/slicer-runtime/tests/integration/anchored_*.rs` (read, never edit)
- Blast-radius discipline: not applicable — no struct field, no schema constant. New test
  literals need FRU or an `// exhaustive:` waiver.
- Expected sub-agent dispatches:
  - Question: report the exact signature of
    `slicer_runtime::layer_executor::execute_anchored_event_collections_with_mode` and how
    `crates/slicer-runtime/tests/integration/anchored_parallel_determinism.rs` calls it for the
    serial and parallel cases. **Do not ask for a "knob name": no `force_parallel` config key,
    env var, or `PipelineConfig` field exists — it is a positional `bool` parameter.** Confirm
    that and report the positional index; scope: `crates/slicer-runtime/src/layer_executor.rs`
    and `crates/slicer-runtime/tests/integration/anchored_parallel_determinism.rs`; return:
    `FACT` plus ≤20 `LOCATIONS`
- Context cost: `M`
- Authoritative docs:
  - `docs/specs/support-families-anchored-entities-plan.md` - §6, the "Z-spanning atomicity" and
    "serial/parallel determinism" invariants (cited by phrase, not ordinal); delegated `SUMMARY`,
    never full-read
- OrcaSlicer refs:
  - none for this step; canonical says nothing about this repo's parallel scheduler
- Verification:
  - `cargo test -p slicer-runtime --test integration -- offgrid_rows_tdd::offgrid_row_order_identical_serial_and_parallel --exact 2>&1 | tee target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` - FACT pass/fail (AC-3)
  - `cargo test -p slicer-runtime --test integration -- offgrid_rows_tdd::zspanning_support_entity_emits_atomic_single_block --exact 2>&1 | tee target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` - FACT pass/fail (AC-4)
  - `cargo test -p slicer-runtime --test integration -- anchored_parallel_determinism --exact 2>&1 | tee target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` - FACT pass/fail
  - `cargo xtask check-literals` - FACT pass/fail
- Exit condition: AC-3 and AC-4 pass with no production file edited in this step. Falsifying
  exit: AC-3 passes while comparing only `z` values and ignoring `global_layer_index` — that
  would leave the index rule unpinned, which is the whole point of the pair comparison.

### Step 10: Reconciliation, doc registration, and gap-register edits

- Task IDs: `TASK-408`
- Objective: register `TASK-399..TASK-408` in `docs/07_implementation_status.md`; re-point row
  `G-02` in `docs/specs/support-parity-gap-register.md` from `239-support-independent-layer-z`
  to this packet's slice and add a new row recording that the anchored-event substrate has no
  production producer (findings F5/F6/F7); mark queue row 1 of
  `docs/specs/support-independent-layer-z-split-plan.md` with this packet's directory and
  closed status; run the full gate set; and prepare `packet.spec.md` for `status: implemented`.
- Precondition: Steps 1–9 landed; every AC command in `requirements.md` §Verification Commands
  returns PASS.
- Postcondition: all three Doc Impact greps in `packet.spec.md` return zero exit status; the
  three gate commands pass; no fixture-slice artifact, human-validation gate, or `tmp/` evidence
  file was produced (those belong to `239c-support-layer-height-producer` and would be vacuous
  here).
- Files allowed to read, with ranges when over 300 lines:
  - `docs/07_implementation_status.md` - long; read the tail and the renumbering note only,
    or delegate. `docs/07_implementation_status.md` is the authoritative task-ID identifier per
    its own renumbering note.
  - `docs/specs/support-parity-gap-register.md` - full (76 lines)
  - `docs/specs/support-independent-layer-z-split-plan.md` - full (152 lines)
  - `docs/spec_packets/239a-anchored-host-seams/task-map.md` - full; it is the verbatim source
    for the registration rows
- Files allowed to edit (at most 3):
  - `docs/07_implementation_status.md`
  - `docs/specs/support-parity-gap-register.md`
  - `docs/specs/support-independent-layer-z-split-plan.md`
- Files explicitly out of bounds:
  - all `crates/**` and `modules/**` — this is a documentation-only step
  - `docs/spec_packets/239-support-independent-layer-z/**` and every other packet directory —
    239's own supersession edit is not this packet's to make
  - `~/.claude` and any user-scoped memory — packet knowledge is version-controlled only
- Blast-radius discipline: not applicable to code. **Ledger discipline applies instead:** the
  next free `G-` row, the next free `DEV-` id, and the `docs/07_implementation_status.md`
  high-water mark are **mutable shared state** and must be re-derived at the moment of the edit,
  never quoted from a packet artifact. They were `G-27`, `DEV-157`, and `TASK-507` when
  `docs/specs/support-independent-layer-z-split-plan.md` was written; a parallel packet may have
  claimed any of them since. Re-derive the next `DEV-` id with
  `rg -o '^\| DEV-[0-9]{3}' docs/DEVIATION_LOG.md | sort -u | tail -1` and take the successor;
  derive the next `G-` row the same way from the gap register.
- Expected sub-agent dispatches:
  - Question: append the `TASK-399..TASK-408` rows from this packet's `task-map.md` to
    `docs/07_implementation_status.md`, re-deriving the current high-water mark first and
    reporting it; scope: `docs/07_implementation_status.md`; return: `FACT` (the derived
    high-water mark plus pass/fail) — never a full backlog read
  - Question: report the highest currently-used `G-` row id in the gap register and the highest
    `DEV-` id in `docs/DEVIATION_LOG.md`; scope: `docs/specs/support-parity-gap-register.md`,
    `docs/DEVIATION_LOG.md`; return: `FACT`
- Context cost: `S`
- Authoritative docs:
  - `docs/07_implementation_status.md` - the authoritative task-ID identifier, per its own
    renumbering note; delegate the append
  - `docs/specs/support-independent-layer-z-split-plan.md` - §Disposition of packet 239 and
    §Packet Queue; direct read
- OrcaSlicer refs:
  - none for this step
- Verification:
  - `rg -q '^\s*- \[[ x]\] TASK-399 ' docs/07_implementation_status.md && rg -q '^\s*- \[[ x]\] TASK-408 ' docs/07_implementation_status.md` - FACT pass/fail
  - `rg -q '239a-anchored-host-seams' docs/specs/support-parity-gap-register.md` - FACT pass/fail
  - `rg -q '^\| 1 \|.*\| closed \|.*docs/spec_packets/239a-anchored-host-seams' docs/specs/support-independent-layer-z-split-plan.md` - FACT pass/fail
    (row-form, not a bare token: the slug alone already matches row 1's `packet slug` column on
    the unmodified tree, so a bare-token grep would be vacuous; this one exits 1 today and passes
    only once row 1's `status` column reads `closed` and its `packet dir` column carries
    `docs/spec_packets/239a-anchored-host-seams/`)
  - `cargo check --workspace --all-targets` - FACT pass/fail
  - `cargo clippy --workspace --all-targets -- -D warnings` - FACT pass/fail
  - `cargo xtask check-literals` - FACT pass/fail
- Exit condition: the three greps and the three gates pass, and every ledger value written was
  re-derived in this step rather than copied from an artifact. Falsifying exit: a `G-` or `DEV-`
  id duplicating one already committed by another packet — that is the exact failure mode the
  re-derivation rule exists to prevent.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | M | Five files / six exhaustive literal sites, but each edit is one line; the cost is the cross-binary literal-site sweep (the `contract` binary carries three of the six) plus the two easily-missed destructuring patterns. Not splittable — see the step's step-splitting note. |
| Step 2 | S | Two test files; the emitter-injection idiom is already demonstrated by two existing fixtures. |
| Step 3 | M | Three pipeline-level tests plus a hand-built `ExecutionPlan` with anchored entities; must be observed genuinely red. |
| Step 4 | S | One ranged pair of reads in a 3886-line file; behaviour-neutral by construction. |
| Step 5 | M | New module plus one delegated canonical dispatch; ranged reads in two large IR/executor files. |
| Step 6 | M | Largest step: the switch plus the finalization-seam insertion, read against both the executor return tuple and the postpass deep-copy contract. |
| Step 7 | S | Mirrors Step 6's landed shape in a shorter body; two new tests over the Step 2 baseline. |
| Step 8 | S | One ranged read in a 2340-line file; a single call-site substitution. |
| Step 9 | M | Two fixtures with parallel-execution and Z-spanning setup; no production edits. |
| Step 10 | S | Documentation only; the cost is delegated ledger re-derivation, not reading. |

Aggregate is `M`; no step is rated `L`. Split before activation if that changes.

## Packet Completion Gate

- All ten steps and their exit conditions complete.
- Every pipe-suffixed AC command in `packet.spec.md` returns PASS: AC-1 through AC-6 and AC-N1
  through AC-N3.
- `docs/07_implementation_status.md` updated through a worker dispatch in Step 10, never a full
  backlog read.
- Supersession reconciled: `packet.spec.md` already carries
  `supersedes: 239-support-independent-layer-z`, and Step 10 re-points gap-register row `G-02`
  and marks the split-plan queue row. Packet 239's own `status: superseded` flip is **not** this
  packet's edit — it belongs to whichever agent closes the split-plan queue across all three
  successors.
- The honest limitation stands unchanged in `packet.spec.md`, `requirements.md`, and
  `design.md`: no production code constructs an `AnchoredEntity`, so every AC here is
  integration-level. Closure must not claim a real-slice proof.
- `packet.spec.md` is ready for `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed AC command from `packet.spec.md` and the three packet-level
  gate commands (`cargo check --workspace --all-targets`,
  `cargo clippy --workspace --all-targets -- -D warnings`, `cargo xtask check-literals`), each
  returning FACT pass/fail.
- Re-run both signature tripwires
  (`cargo test -p slicer-runtime --test visual_debug_agent_overhead_tdd`,
  `cargo test -p pnp-cli --test visual_debug_typed_tap_capture_tdd`) and
  `cargo xtask build-guests --check` (exit code only) as the final regression sweep.
- **`cargo test --workspace` is not part of this ceremony.** Every AC is proven by a targeted
  `--exact` command with a non-zero matched-count guard, and no step touches a guest, WIT, IR,
  or module surface that would justify a whole-suite run. If a reviewer requires one, it must go
  through `cargo xtask test --summary --workspace --no-fail-fast` and be dispatched to a
  sub-agent returning FACT pass/fail — never absorbed into the implementer's context.
- Record remaining packet-local risk: the seam ships proven only at integration level; a real
  producer arrives in `239c-support-layer-height-producer` and may expose adjustments,
  particularly around the locked `global_layer_index` rule for solo synthesized rows.
- Confirm context stayed at or below 150k standard, or at/below 300k only with a logged swarm
  ESCALATION; otherwise record a packet-authoring lesson.

All `cargo check`, `cargo clippy`, and `cargo test` invocations in gate and verification
commands use `--all-targets` where the flag applies, so test, bench, and example targets compile.
