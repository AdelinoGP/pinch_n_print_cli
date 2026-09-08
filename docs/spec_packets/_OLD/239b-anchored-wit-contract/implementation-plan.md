# Implementation Plan: 239b-anchored-wit-contract

## Execution Rules

- Work one atomic step at a time; map every step to grouped task IDs.
- Use TDD, then implementation, then the narrowest falsifying validation.
- Every field below is a context-budget contract and must be filled independently; never write
  "see Step 1".
- **WIT/Type Changes Checklist (`CLAUDE.md`) binds Steps 2-6.** After any `.wit` edit: search all
  `wit_host.rs`, `dispatch.rs`, and `wit_guest` modules for the affected type; verify type
  identity across the component boundary; run `cargo build --tests`; edit only the canonical
  sources under `crates/slicer-schema/wit/`.
- **Guest freshness gates Steps 3-7.** `cargo xtask build-guests --check` decides by **exit
  code** (0 fresh / 1 stale / 3 `wasm-tools` missing). Never grep for `STALE:`. Rebuild before
  attributing any failure to the step's own code.
- Tee every cargo test to `target/test-output.log`; read the file, never re-run to see more
  output. Every verification asserts a non-zero matched-test count in-run.
- **`--exact` filters against a `mod`-aggregated test binary MUST be module-qualified.**
  `crates/slicer-runtime/tests/{executor,contract}/main.rs` and
  `crates/slicer-wasm-host/tests/contract/main.rs` are `mod <file>;` aggregators, so libtest names
  tests `<mod_name>::<fn_name>`; a bare name matches zero tests and still prints
  `test result: ok`. Measured: a bare-name `--exact` run of
  `production_variants_match_world_layer_stages_exactly` printed `0 passed; ... 295 filtered out`.
  Prefixes: `anchored_events_roundtrip_tdd::`, `layer_stage_commit_stages_tdd::`,
  `anchored_events_both_legs_tdd::`, `support_anchored_reach_tdd::` (AC-8; same
  `crates/slicer-runtime/tests/executor/main.rs` binary, whose `mod` list is the authority —
  re-derive it at edit time). `crates/slicer-schema/tests/export_for_stage_id_tdd.rs` is a
  standalone target and takes no prefix. `cargo xtask build-guests --check` takes neither a
  filter nor the matched-count guard — it is judged by exit code.
- **A new file in an aggregated test binary is not compiled until it is registered.** Whenever a
  step authors a file under `crates/*/tests/<bucket>/`, that step also owns the `mod` line in the
  bucket's `main.rs`. Skipping it yields a green run over zero tests.
- Ledger facts (`TASK-###` high-water, next free `G-`/`DEV-` ids) are mutable — re-derive them at
  the moment you edit the ledger, never from a value quoted in this packet.

## Steps

### Step 1: Ground the orphan claim and inventory every registration site

- Task IDs: `TASK-508`
- Objective: re-verify, against the tree as it stands, that the five anchored records in
  `crates/slicer-schema/wit/deps/ir-types.wit` are referenced by zero interfaces, zero worlds,
  and zero function signatures; and produce the complete list of tables and `match` sites a ninth
  `Layer::*` stage plus a new producer arm must touch. This is a read-only discovery step; its
  output is an inventory, not an edit.
- Precondition: clean tree on the packet branch; `cargo check --workspace --all-targets` green.
- Postcondition: a written inventory naming, at minimum: the two `deconstruct_layer_ctx` call
  sites in `crates/slicer-wasm-host/src/dispatch.rs`; the `match stage_export` inside
  `commit_native_layer_response` (`crates/slicer-wasm-host/src/marshal/native.rs`) — note it
  scrutinises `stage_export`, not `stage_id`, and its support arm is the shared
  `"Layer::Support" | "Layer::SupportPostProcess"`; `slicer_schema::STAGES`;
  `slicer_schema::VALID_STAGES`; `slicer_scheduler::execution_plan::STAGE_ORDER`; the
  `HOST_ONLY_STAGES` list in `crates/slicer-scheduler/tests/contract/stage_list_consistency_tdd.rs`;
  the `production` array and `expected.len()` assertion in
  `crates/slicer-runtime/tests/contract/layer_stage_commit_stages_tdd.rs`; the
  `emit_world_preamble` call sites and `include_str!` set in `crates/slicer-macros/src/lib.rs`;
  the `.wit` literal in `crates/slicer-macros/build.rs`; and the count expectations in
  `xtask/src/wit_verify.rs`'s test module. Each entry cited by **symbol name with a
  crate-qualified path**, never by bare line number.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-schema/wit/deps/ir-types.wit` — the `anchored-*` /
    `ordered-event-collection` block and the `resource layer-collection-builder` block only
  - `crates/slicer-schema/wit/deps/layer-support/layer-support.wit` — whole (~20 lines)
  - `crates/slicer-schema/src/lib.rs` — the `StageSpec` definition, the first two `STAGES` rows,
    and `VALID_STAGES` only
  - `crates/slicer-ir/src/stage_io.rs` — the `LayerStageCommit` enum and its `stage_id()` impl
    only
  - `crates/slicer-runtime/tests/contract/layer_stage_commit_stages_tdd.rs` — whole (~75 lines)
- Files allowed to edit (at most 3):
  - none (read-only discovery step)
- Files explicitly out of bounds:
  - `OrcaSlicerDocumented/**`; `target/`; `crates/slicer-runtime/src/pipeline.rs`;
    `crates/slicer-runtime/src/layer_executor.rs` beyond `validate_anchored_entity`;
    `modules/core-modules/**`
- Expected sub-agent dispatches:
  - Question: are the five anchored records still referenced by zero interfaces, zero worlds, and
    zero function signatures across the whole `wit/` tree?; scope:
    `crates/slicer-schema/wit/**`; return: `FACT` plus a per-record reference count
  - Question: enumerate every `match stage_id` site and every stage table/list/test-array that
    must gain a `"Layer::AnchoredEvents"` entry; scope: `crates/**/*.rs`, `xtask/src/*.rs`;
    return: `LOCATIONS` ≤ 20
  - Question: the exact literal expectations in `xtask/src/wit_verify.rs`'s test module and the
    `.wit` string literal in `crates/slicer-macros/build.rs`; scope: those two files; return:
    `LOCATIONS` ≤ 10
- Context cost: `S`
- Authoritative docs:
  - `docs/specs/support-independent-layer-z-split-plan.md` — findings F5/F6/F7, direct ranged read
  - `docs/05_module_sdk.md` — delegated `SUMMARY`: what a per-layer stage row must declare
    (the `STAGES`-table / stage-method / trait conventions live here)
- OrcaSlicer refs:
  - none. This packet has no canonical port; see `requirements.md` §OrcaSlicer Reference
    Obligations.
- Verification:
  - The FACT dispatch returns "zero references" for all five records. If it returns anything
    else, **stop**: the packet's premise has changed and `requirements.md` §Problem Statement
    must be corrected before Step 2.
  - The LOCATIONS inventory contains at least the ten sites named in the postcondition.
- Exit condition: the inventory is written into the step's working notes, every site is cited by
  symbol name plus crate-qualified path, and the orphan FACT is confirmed.

### Step 2: Declare the anchored-events interface, world, and stage

- Task IDs: `TASK-509`
- Objective: create `crates/slicer-schema/wit/deps/layer-anchored-events/layer-anchored-events.wit`
  (package `slicer:layer-anchored-events@1.0.0`, interface `anchored-events` with a single `run`
  export, world `anchored-events-module`), add
  `set-anchored-event-collection: func(collection: ordered-event-collection) -> result<_, string>`
  to the existing `resource layer-collection-builder` in `ir-types.wit`, register the stage in
  `STAGES` / `VALID_STAGES` / `STAGE_ORDER`, reconcile the declaration model so the workspace
  compiles and every audit test agrees, **and author AC-4's test**
  `anchored_events_stage_is_fully_declared` in
  `crates/slicer-schema/tests/export_for_stage_id_tdd.rs` (today holding only
  `export_for_stage_id_is_total_over_stages_and_rejects_unknown`). AC-4 cannot pass without that
  test existing, and this step's postcondition demands it, so the file is owned here — it is a
  standalone integration-test target, so no `mod` registration line is needed.
- Precondition: Step 1's inventory complete and its orphan FACT confirmed.
- Postcondition: AC-4 and AC-5 commands PASS; `cargo build --tests` clean;
  `cargo test -p xtask --bin xtask wit_verify` PASS with the counts at 21; the five anchored
  records are now reachable from a world.
- Blast-radius discipline (mandatory — this step adds an entry to two schema/stage constant
  tables and one hard-coded count expectation). Every site below is listed in "Files allowed to
  edit" and budgeted here; none is deferred to a later `cargo check`. Sites, from Step 1's
  `LOCATIONS` dispatch:
  - `slicer_schema::STAGES` — one `StageSpec` row (`method: "run_anchored_events"`,
    `stage_id: "Layer::AnchoredEvents"`, `wit_export: "run"`, `tier_id: TIER_LAYER`,
    `trait_name: "LayerModule"`, `wit_dir: "layer-anchored-events"`,
    `wit_package: "slicer:layer-anchored-events@1.0.0"`, `wit_interface: "anchored-events"`,
    `wit_world: "anchored-events-module"`).
  - `slicer_schema::VALID_STAGES` — one entry (module-targetable, **not** host-only).
  - `slicer_scheduler::execution_plan::STAGE_ORDER` — one entry inside the `Layer::*` block.
  - `crates/slicer-macros/src/lib.rs` — one new `emit_world_preamble("anchored-events-module",
    "anchored_events", ...)` call site with its own `include_str!` of the new `.wit` file.
  - `crates/slicer-macros/build.rs` — the matching `rerun-if-changed` path.
  - `xtask/src/wit_verify.rs` — three `20` expectations in its `#[cfg(test)]` module become `21`
    (macro `include_str!` count, canonical-set comparison, `build.rs` watch-set count). These
    three surfaces cross-check each other; editing any one alone leaves the workspace red.
  - `crates/slicer-runtime/tests/contract/layer_stage_commit_stages_tdd.rs` — add
    `LayerStageCommit::AnchoredEvents(Vec::new())` to the `production` array and move the
    `expected.len()` assertion from `8` to `9`. Note honestly in the commit message that the
    variant was previously absent from that array while returning a stage id present in no
    `STAGES` row — registration closes that hole.
  - `crates/slicer-schema/tests/export_for_stage_id_tdd.rs` — AC-4's authoring home. One new
    `#[test] fn anchored_events_stage_is_fully_declared` asserting `stage_by_id`,
    `wit_world_for_stage_id`, `interface_for_stage_id`, `package_for_stage_id`, and
    `qualified_export_for_stage_id` all resolve for `"Layer::AnchoredEvents"`. Standalone
    integration-test target — its `--list` output is unqualified, which is why AC-4 is the one AC
    command with a bare `--exact` name.
  - `crates/slicer-scheduler/tests/contract/stage_list_consistency_tdd.rs` — read-only here; it
    must go green **because** the stage was added to `VALID_STAGES`, not by adding it to
    `HOST_ONLY_STAGES`.
  - `crates/slicer-ir/src/stage_io.rs` — **comment-only edit.** `LayerStageCommit::stage_id`'s doc
    comment states "The non-`None` set is exactly the eight `world-layer` stages — a property
    pinned by a meta-test so the enum and `STAGES` cannot drift (ADR-0020)". This packet makes it
    **nine**, so the word `eight` becomes `nine`. No code changes; the doc comment is the exact
    invariant AC-5's meta-test enforces, and leaving it at `eight` puts the prose in direct
    contradiction with the assertion in the same repo. Do not touch anything else in that file.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-schema/wit/deps/layer-support/layer-support.wit` — whole
  - `crates/slicer-schema/wit/deps/ir-types.wit` — the builder-resource block only
  - `crates/slicer-schema/src/lib.rs` — `StageSpec`, the `Layer::Support` row, `VALID_STAGES`
  - `crates/slicer-macros/src/lib.rs` — over 300 lines: **one** exemplar
    `emit_world_preamble` call site only; do not scan all fifteen
- Files allowed to edit (3 primary + a 7-file blast radius; every blast-radius entry except the
  two test files is a one-to-three-line mechanical edit, and the two test files are single
  additions to existing targets):
  - `crates/slicer-schema/wit/deps/layer-anchored-events/layer-anchored-events.wit` (new)
  - `crates/slicer-schema/wit/deps/ir-types.wit`
  - `crates/slicer-schema/src/lib.rs`
  - blast radius: `crates/slicer-scheduler/src/execution_plan.rs`,
    `crates/slicer-macros/src/lib.rs`, `crates/slicer-macros/build.rs`,
    `xtask/src/wit_verify.rs`, `crates/slicer-runtime/tests/contract/layer_stage_commit_stages_tdd.rs`,
    **`crates/slicer-schema/tests/export_for_stage_id_tdd.rs`** (AC-4's test; one new
    `#[test] fn anchored_events_stage_is_fully_declared` appended — standalone target, no `mod`
    registration required), and `crates/slicer-ir/src/stage_io.rs` (**comment only**:
    `LayerStageCommit::stage_id`'s "eight `world-layer` stages" becomes nine)
- Files explicitly out of bounds:
  - `crates/slicer-wasm-host/src/**` (Steps 4-5), `crates/slicer-sdk/src/**` (Steps 3, 6),
    `modules/core-modules/**`, `OrcaSlicerDocumented/**`
- Expected sub-agent dispatches:
  - Question: does `wit-parser` accept the new package when resolved alongside the existing
    `deps/` tree (no unresolved import, no duplicate interface name)?; scope: run
    `cargo build --tests` and report; return: `FACT pass/fail` with ≤ 20 lines on failure
- Context cost: `M`
- Authoritative docs:
  - `docs/03_wit_and_manifest.md` — delegated `SUMMARY`: world-membership and interface-declaration
    conventions
  - `docs/04_host_scheduler.md` — delegated `SUMMARY`: `STAGE_ORDER` placement for a per-layer
    stage
- OrcaSlicer refs:
  - none
- Verification:
  - `cargo build --tests` — FACT pass/fail (the `CLAUDE.md` WIT-change gate)
  - `mkdir -p target && cargo test -p slicer-schema --test export_for_stage_id_tdd -- anchored_events_stage_is_fully_declared --exact 2>&1 | tee target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` (AC-4)
  - `mkdir -p target && cargo test -p slicer-runtime --test contract -- layer_stage_commit_stages_tdd::production_variants_match_world_layer_stages_exactly --exact 2>&1 | tee target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` (AC-5)
  - `mkdir -p target && cargo test -p xtask --bin xtask wit_verify 2>&1 | tee target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0`
  - `mkdir -p target && cargo test -p slicer-scheduler --test scheduler_contract 2>&1 | tee target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` (target is `scheduler_contract`; see Implementation Deviations #2)
  - `cargo check --workspace --all-targets`
- Exit condition: all six commands green; the new `.wit` file resolves; the guest tree is now
  known-stale and Step 3 opens with a rebuild.

### Step 3: Red-first round-trip test plus the anchored-events test guest

- Task IDs: `TASK-510`
- Objective: add `run_anchored_events` to the `LayerModule` trait with a default no-op body;
  create `crates/slicer-wasm-host/test-guests/anchored-events-roundtrip-guest/` modelled on
  `finalization-mutation-roundtrip-guest` (one binary, config-parameterized by
  `anchored_event_count`, `emit_malformed_geometry` — tri-valued: `0` well-formed, `1` planar
  mismatch, `2` Z-spanning out-of-range —, `duplicate_proposal`); author the **seven**
  round-trip assertions (AC-1, AC-2, AC-3, AC-N1, AC-N2, AC-N3, AC-N4) in a new
  `crates/slicer-runtime/tests/executor/anchored_events_roundtrip_tdd.rs`; **and register that
  file as `mod anchored_events_roundtrip_tdd;` in
  `crates/slicer-runtime/tests/executor/main.rs`** — that `mod` list is how the `executor` binary
  compiles the file, and without the line it never compiles, the seven tests do not exist, and
  every AC command reports a green run over zero tests. The step ends
  **red for the right reason**: the guest builds and runs, calls the pre-existing
  `LayerCollectionBuilder::set_anchored_event_collection`, and the host receives nothing because
  no drain and no producer arm exist yet.
- Precondition: Step 2 landed. Guests rebuilt (`cargo xtask build-guests`) and
  `cargo xtask build-guests --check` returns exit `0`.
- Postcondition: the guest crate compiles to `anchored-events-roundtrip-guest.component.wasm`;
  the seven named tests exist **and are visible to libtest** — prove it with
  `cargo test -p slicer-runtime --test executor -- --list | grep -c '^anchored_events_roundtrip_tdd::'`
  returning `7`, which is the direct check that the `mod` registration landed — and they fail
  with assertions about a missing commit, not with compile errors or panics in unrelated code.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-wasm-host/test-guests/finalization-mutation-roundtrip-guest/src/lib.rs` —
    whole (46 lines)
  - `crates/slicer-wasm-host/test-guests/finalization-mutation-roundtrip-guest/Cargo.toml` —
    whole
  - `crates/slicer-runtime/tests/executor/finalization_mutation_roundtrip_tdd.rs` — whole
    (the guest-load, component-compile, and dispatch-drive harness)
  - `crates/slicer-sdk/src/layer_collection_builder.rs` — the `set_anchored_event_collection` /
    `anchored_proposal` block only
  - `crates/slicer-runtime/src/layer_executor.rs` — `validate_anchored_entity` only, via
    delegated `SNIPPETS` ≤ 20 lines (for AC-N1's asserted substring)
- Files allowed to edit (3 primary + one one-line registration):
  - `crates/slicer-wasm-host/test-guests/anchored-events-roundtrip-guest/` (new crate:
    `Cargo.toml` with `crate-type = ["cdylib"]` and a `[workspace]` sentinel, plus `src/lib.rs`)
  - `crates/slicer-runtime/tests/executor/anchored_events_roundtrip_tdd.rs` (new)
  - `crates/slicer-sdk/src/traits.rs` (one default trait method)
  - registration: `crates/slicer-runtime/tests/executor/main.rs` — **one** line,
    `mod anchored_events_roundtrip_tdd;`, appended to the existing `mod` list (alphabetical
    position is conventional there but not enforced). This is not optional and not a later
    `cargo check` discovery: it is the compile trigger for the file above.
- Files explicitly out of bounds:
  - `crates/slicer-wasm-host/src/**` (Steps 4-5), `crates/slicer-schema/wit/**` (Step 2 is
    closed), `modules/core-modules/**`
- Expected sub-agent dispatches:
  - Question: how does `crates/slicer-runtime/tests/executor/finalization_mutation_roundtrip_tdd.rs`
    locate, compile, and dispatch its guest, and what module-id / manifest shape does it use?;
    scope: that file; return: `SNIPPETS` ≤ 30 lines; purpose: copy the harness rather than
    reinvent it
  - Question: confirm test-guests are discovered by directory scan (`tg_root`) in
    `xtask/src/build_guests.rs` and that no hardcoded guest list needs an entry; scope: that
    file; return: `FACT`
- Context cost: `M`
- Authoritative docs:
  - `CLAUDE.md` §"Guest WASM Staleness" — direct read; the new guest must build into the shared
    `crates/slicer-wasm-host/test-guests/target/` `CARGO_TARGET_DIR`
- OrcaSlicer refs:
  - none
- Verification:
  - `cargo xtask build-guests` then `cargo xtask build-guests --check && echo FRESH` — the new
    guest artifact exists and every guest is fresh (exit `0`)
  - `mkdir -p target && cargo test -p slicer-runtime --test executor -- --list 2>&1 | tee target/test-output.log && test "$(grep -c '^anchored_events_roundtrip_tdd::' target/test-output.log)" -eq 7` — the registration check: the `mod` line landed and all seven tests are visible to libtest
  - `mkdir -p target && cargo test -p slicer-runtime --test executor anchored_events_roundtrip_tdd 2>&1 | tee target/test-output.log && grep -cE "^test .+ FAILED|panicked at" target/test-output.log` — expect ≥ 1, and the failures must be the authored assertions (missing commit), with zero unrelated failures
  - `cargo check --workspace --all-targets`
- Exit condition: the `--list` count is exactly `7`; the log shows exactly the seven authored
  tests failing on missing-commit assertions; the guest artifact is present and fresh; no
  production host file was edited.

### Step 4: Host lift glue — bindgen world module, accumulator, context field, resource method, converter

- Task IDs: `TASK-511`
- Objective (first and most easily missed): add the **bindgen world module** to
  `crates/slicer-wasm-host/src/host.rs` — `pub mod layer_anchored_events { wasmtime::component::bindgen!({ ... world: "anchored-events-module", ... }) }`,
  copied from the sibling `pub mod layer_support` block in the same file, plus the matching
  `pub use layer_anchored_events::LayerModule as LayerAnchoredEventsModule;` entry in that file's
  `pub use` block (exemplar: `pub use layer_support::LayerModule as LayerSupportModule;`). Without
  this module there is no generated `LayerModule` type, so Step 5a has nothing to link or
  instantiate and the guest can never run — a stage that passes AC-4 and AC-5 while being a
  permanent no-op. Then: add `AnchoredEventsCollected` to
  `crates/slicer-wasm-host/src/marshal/accumulators.rs`; add the matching field and `_mut`
  accessor to `HostExecutionContext` (`crates/slicer-wasm-host/src/host.rs`) beside
  `support_output` / `gcode_output`; implement `set_anchored_event_collection` on
  `impl ir::HostLayerCollectionBuilder for HostExecutionContext`; and add
  `convert_anchored_events` to `crates/slicer-wasm-host/src/marshal/out.rs`, re-exported from
  `crates/slicer-wasm-host/src/marshal/mod.rs`. The converter lifts the WIT record into
  `slicer_ir::OrderedEventCollection` carrying every `s64` as `i64` with **no scaling and no
  `f32` hop**. Finally, author `validate_anchored_entity_geometry` beside the converter in
  `crates/slicer-wasm-host/src/marshal/out.rs`: a `slicer-wasm-host`-local **duplicate** of the
  planar and z-span checks in `validate_anchored_entity`
  (`crates/slicer-runtime/src/layer_executor.rs`), returning `Err` messages containing the
  duplicated literals `anchored entity planar z mismatch` and `anchored entity z-span violation`,
  compared against `AnchoredGeometryContract::COORDINATE_TOLERANCE_UNITS`. Duplication is forced,
  not chosen: the dependency edge runs `slicer-runtime` → `slicer-wasm-host`
  (`crates/slicer-runtime/Cargo.toml` declares `slicer-wasm-host`; there is no reverse
  dependency), so nothing in `slicer-wasm-host` can call the original, and
  `crates/slicer-runtime/src/layer_executor.rs` is read-only in this packet. Do not attempt to
  import it, and do not describe the result as reuse.
- Precondition: Step 3 landed; AC-1 is red on a missing commit.
- Postcondition: `cargo check --workspace --all-targets` clean; the converter is unit-tested for
  exact `i64` preservation; AC-1 is still red (no producer arm yet) — this is expected, not a
  regression.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-wasm-host/src/marshal/accumulators.rs` — the `SupportOutputCollected` block
  - `crates/slicer-wasm-host/src/marshal/out.rs` — `convert_support_output_with_plan` only
  - `crates/slicer-wasm-host/src/host.rs` — over 300 lines: the `pub mod layer_support`
    `bindgen!` block and the `pub use layer_support::LayerModule as LayerSupportModule;` line, the
    `HostExecutionContext` output-collector block, `push_layer_collection_builder`, and
    `impl ir::HostLayerCollectionBuilder for HostExecutionContext` only
  - `crates/slicer-runtime/src/layer_executor.rs` — `validate_anchored_entity` only, via delegated
    `SNIPPETS` ≤ 20 lines, for the two message literals to duplicate. Read-only; never edited.
  - `crates/slicer-ir/src/slice_ir.rs` — the `OrderedEventCollection` / `AnchoredEntity` /
    `AnchoredGeometryContract` / `AnchoredEntityProvenance` / `AnchoredEventRuntimeHooks`
    definitions only
- Files allowed to edit (3 primary; the mod re-export is one line):
  - `crates/slicer-wasm-host/src/marshal/accumulators.rs`
  - `crates/slicer-wasm-host/src/marshal/out.rs` (`convert_anchored_events` +
    `validate_anchored_entity_geometry`)
  - `crates/slicer-wasm-host/src/host.rs` (**`pub mod layer_anchored_events` bindgen block +
    `LayerAnchoredEventsModule` alias**, plus the accumulator field, its `_mut` accessor, and the
    `set_anchored_event_collection` resource-method impl)
  - blast radius: `crates/slicer-wasm-host/src/marshal/mod.rs` (one `pub use` line)
- Files explicitly out of bounds:
  - `crates/slicer-wasm-host/src/dispatch.rs` and
    `crates/slicer-wasm-host/src/marshal/native.rs` (Step 5), `crates/slicer-sdk/src/**`
    (Step 6), `crates/slicer-schema/wit/**`
- Expected sub-agent dispatches:
  - Question: the full call chain from `resource support-output-builder`'s WIT declaration to
    `SupportOutputCollected` to `convert_support_output_with_plan` — every file and symbol
    touched; scope: `crates/slicer-wasm-host/src/`; return: `LOCATIONS` ≤ 15; purpose: copy the
    pattern exactly rather than invent a parallel one
  - Question: return the `pub mod layer_support` `bindgen!` block and its
    `pub use ... as LayerSupportModule` alias verbatim from `crates/slicer-wasm-host/src/host.rs`;
    scope: that file; return: `SNIPPETS` ≤ 30 lines; purpose: the exact `bindgen!` option set
    (`path:`, `world:`, `with:`, async/trappable settings) to copy for
    `pub mod layer_anchored_events`
  - Question: `validate_anchored_entity`'s planar and z-span rejection messages; scope:
    `crates/slicer-runtime/src/layer_executor.rs`; return: `SNIPPETS` ≤ 20 lines; purpose: the
    literals to duplicate into `validate_anchored_entity_geometry`
- Context cost: `M`
- Authoritative docs:
  - `docs/08_coordinate_system.md` — delegated `SUMMARY` confirming that canonical-unit integers
    cross boundaries unscaled
- OrcaSlicer refs:
  - none
- Verification:
  - `cargo build --tests` — FACT pass/fail
  - `mkdir -p target && cargo test -p slicer-wasm-host --lib convert_anchored_events 2>&1 | tee target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` — the converter's own unit test proves `planar(3000)` lifts to `Planar { z: 3000 }` and `z-spanning((3000, 5000))` to `ZSpanning { min_z: 3000, max_z: 5000 }`
  - `mkdir -p target && cargo test -p slicer-wasm-host --lib validate_anchored_entity_geometry 2>&1 | tee target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` — the duplicated validator rejects both a planar mismatch and a Z-spanning out-of-range point with the two expected substrings
  - `cargo check --workspace --all-targets`
- Exit condition: the converter unit test passes on exact integer equality; the
  `layer_anchored_events` bindgen module compiles and `LayerAnchoredEventsModule` resolves; the
  duplicated validator rejects both geometry contracts; AC-1 remains red for the documented
  reason (no producer arm).

### Step 5a: Make the guest callable - bindgen linkage and the invocation arm

- Task IDs: `TASK-512` (shared with Step 5b; the two are one verification atom and are recorded
  under a single task id with distinct step numbers - no new task id is minted, and none outside
  `TASK-508`..`TASK-514` may be)
- Objective: add the `"Layer::AnchoredEvents"` arm to the **layer-tier linker/instantiate/call**
  `match stage_id.as_str()` in `crates/slicer-wasm-host/src/dispatch.rs` - the one whose value is
  bound as `let (call_result, mut store, mem_initial_bytes) = ...`. The arm mirrors the
  `"Layer::Support"` arm end to end: build the `wasmtime::component::Linker`, call
  `add_wasi_to_linker`, call `layer_anchored_events::LayerModule::add_to_linker`, construct the
  store through `HostExecutionContextBuilder` (registering the layer-collection builder so the
  guest's `set-anchored-event-collection` call has a resource to write into), then
  `layer_anchored_events::LayerModule::instantiate` and invoke the stage export. **This is the
  step that actually runs the guest.** Without it the stage is declared, drained, and permanently
  inert.
- **Do not touch the prepass-tier `match stage_id.as_str()`** in the same file - the one bound as
  `let (call_result, mut store) = ...`. It serves a different tier and is explicitly out of scope.
  Confirm which match you are editing by the shape of its `let` binding before typing.
- Precondition: Step 4 landed; `pub mod layer_anchored_events` and `LayerAnchoredEventsModule`
  exist in `crates/slicer-wasm-host/src/host.rs`; `convert_anchored_events` and
  `validate_anchored_entity_geometry` exist in `crates/slicer-wasm-host/src/marshal/out.rs`.
- Postcondition: dispatching `Layer::AnchoredEvents` against the Step 3 guest **instantiates and
  calls it** - the guest's own host-services log line is observable - while
  `deconstruct_layer_ctx` still returns `Ok(None)` for the stage, because the producer arm does
  not exist yet. AC-1 remains red, now for a *different* documented reason than in Step 4.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-wasm-host/src/dispatch.rs` - over 300 lines: the `"Layer::Support"` arm of the
    layer-tier `match stage_id.as_str()` (the one bound as
    `let (call_result, mut store, mem_initial_bytes) = ...`) and `add_wasi_to_linker` only
  - `crates/slicer-wasm-host/src/host.rs` - the `pub mod layer_anchored_events` block authored in
    Step 4 and the `HostExecutionContextBuilder` signature only
- Files allowed to edit (1):
  - `crates/slicer-wasm-host/src/dispatch.rs` (invocation arm only - the `deconstruct_layer_ctx`
    arm is Step 5b's)
- Files explicitly out of bounds:
  - `crates/slicer-wasm-host/src/marshal/native.rs` (Step 5b), `crates/slicer-sdk/src/**`
    (Step 6), `crates/slicer-schema/wit/**`, `crates/slicer-runtime/src/layer_executor.rs`
- Expected sub-agent dispatches:
  - Question: return the complete `"Layer::Support"` arm of the `match stage_id.as_str()` bound as
    `let (call_result, mut store, mem_initial_bytes) = ...` in
    `crates/slicer-wasm-host/src/dispatch.rs`, and state explicitly which of that file's
    `match stage_id.as_str()` statements it came from; scope: that file; return: `SNIPPETS`
    <= 60 lines
- Context cost: `M`
- Authoritative docs:
  - `CLAUDE.md` §"WIT/Type Changes Checklist" - direct read; step 2 of the checklist (type
    identity across the component boundary) is what a mismatched `add_to_linker` world surfaces as
- OrcaSlicer refs:
  - none
- Verification:
  - `cargo build --tests` - FACT pass/fail
  - `cargo xtask build-guests --check && echo FRESH` - run **before** attributing any typed
    instantiation failure to this arm; exit code only, never `rg 'STALE:'`
  - `mkdir -p target && cargo test -p slicer-runtime --test executor anchored_events_roundtrip_tdd 2>&1 | tee target/test-output.log && grep -cE "^test .+ FAILED|panicked at" target/test-output.log` - the seven tests still fail, but the failures must now be *missing-commit* assertions rather than instantiation or "unknown stage" errors; read the log to confirm the failure mode changed
  - `cargo check --workspace --all-targets`
- Exit condition: the guest instantiates and its stage export is called for
  `Layer::AnchoredEvents`; the seven round-trip tests fail on missing commits, not on
  instantiation; the prepass-tier match is unchanged (`git diff` shows no edit near the
  `let (call_result, mut store) = ...` binding).

### Step 5b: Producer arm on BOTH dispatch legs

- Task IDs: `TASK-512` (shared with Step 5a - same task id, distinct step number, stated
  explicitly so no id outside `TASK-508`..`TASK-514` is minted)
- Objective: add the `"Layer::AnchoredEvents"` arm to `deconstruct_layer_ctx`
  (`crates/slicer-wasm-host/src/dispatch.rs`) - a **different `match` from Step 5a's**, running
  after the guest returns - returning `Ok(None)` when the accumulator holds no proposal,
  `Ok(Some(LayerStageCommit::AnchoredEvents(vec![collection])))` when it does, and a
  `LayerStageError::FatalModule` when `validate_anchored_entity_geometry` (authored in Step 4,
  `crates/slicer-wasm-host/src/marshal/out.rs`) rejects: message containing
  `anchored entity planar z mismatch` for a planar mismatch (AC-N1) and
  `anchored entity z-span violation` for a Z-spanning point outside `[min_z, max_z]` (AC-N4).
  **Both branches are required**: ADR-0059 mandates validation per declared contract, so a
  planar-only arm is non-conformant even though it passes AC-N1. **And** the identical twin arm in
  the `match stage_export` inside `commit_native_layer_response`
  (`crates/slicer-wasm-host/src/marshal/native.rs`) — that match scrutinises `stage_export`, not
  `stage_id`. Both legs land in this step; the repo's both-legs guard forbids splitting them.
- **The error substrings are duplicated literals, not reuse.** `validate_anchored_entity`
  (`crates/slicer-runtime/src/layer_executor.rs`) is unreachable from here - the dependency edge
  runs `slicer-runtime` -> `slicer-wasm-host` and not the reverse - and that file is read-only in
  this packet. Do not add a `slicer-runtime` dependency to `slicer-wasm-host` to "reuse" it; that
  inverts the crate graph and is a hard stop.
- Precondition: Step 5a landed; the guest is instantiated and called;
  `convert_anchored_events` and `validate_anchored_entity_geometry` available and unit-tested.
- Postcondition: AC-6, AC-N1, and AC-N4 commands PASS; AC-1 is still red (no SDK drain yet, so the
  accumulator is never populated from a real guest) - expected and documented.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-wasm-host/src/dispatch.rs` - over 300 lines: `deconstruct_layer_ctx`'s header
    and the `"Layer::Support" | "Layer::SupportPostProcess"` arm only (the `Ok(None)`-on-empty
    pattern to copy)
  - `crates/slicer-wasm-host/src/marshal/native.rs` - over 300 lines: `commit_native_layer_response`
    (its `match stage_export` block) and `collect_support` only
  - `crates/slicer-wasm-host/src/marshal/out.rs` - `validate_anchored_entity_geometry`'s signature
    only
- Files allowed to edit (3 primary + one one-line registration):
  - `crates/slicer-wasm-host/src/dispatch.rs` (the `deconstruct_layer_ctx` arm only)
  - `crates/slicer-wasm-host/src/marshal/native.rs`
  - `crates/slicer-wasm-host/tests/contract/anchored_events_both_legs_tdd.rs` (new; AC-6's home)
  - registration: `crates/slicer-wasm-host/tests/contract/main.rs` - one line,
    `mod anchored_events_both_legs_tdd;`. Without it the file never compiles and AC-6 reports a
    green run over zero tests.
  - permitted follow-on: `crates/slicer-wasm-host/src/marshal/out.rs`, **only** to adjust
    `validate_anchored_entity_geometry`'s signature or message plumbing if the arms need it. No
    other change to that file belongs here.
- Files explicitly out of bounds:
  - `crates/slicer-sdk/src/**` (Step 6), `crates/slicer-schema/wit/**`,
    `crates/slicer-runtime/src/layer_executor.rs` (read-only; and unreachable from this crate)
- Expected sub-agent dispatches:
  - Question: confirm the two `deconstruct_layer_ctx` call sites in
    `crates/slicer-wasm-host/src/dispatch.rs` and locate the arms of the `match stage_export`
    inside `commit_native_layer_response`
    (`crates/slicer-wasm-host/src/marshal/native.rs`), so no leg is missed; scope: those two files;
    return: `LOCATIONS` <= 10
- Context cost: `M`
- Authoritative docs:
  - `CLAUDE.md` §"WIT/Type Changes Checklist" - direct read; the type-identity check across the
    boundary applies to the `Option<OrderedEventCollection>` -> `Vec<OrderedEventCollection>` hop
  - `docs/adr/0059-support-families-and-anchored-entities.md` - ranged read of the anchored-entity
    paragraphs; the source of the both-contracts validation requirement
- OrcaSlicer refs:
  - none
- Verification:
  - `mkdir -p target && cargo test -p slicer-wasm-host --test contract -- --list 2>&1 | tee target/test-output.log && test "$(grep -c '^anchored_events_both_legs_tdd::' target/test-output.log)" -gt 0` - the `mod` registration landed
  - `mkdir -p target && cargo test -p slicer-wasm-host --test contract -- anchored_events_both_legs_tdd::anchored_events_native_and_wasm_legs_agree --exact 2>&1 | tee target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` (AC-6)
  - `mkdir -p target && cargo test -p slicer-runtime --test executor -- anchored_events_roundtrip_tdd::malformed_anchored_geometry_is_rejected_as_fatal --exact 2>&1 | tee target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` (AC-N1)
  - `mkdir -p target && cargo test -p slicer-runtime --test executor -- anchored_events_roundtrip_tdd::zspanning_anchored_geometry_out_of_range_is_rejected_as_fatal --exact 2>&1 | tee target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` (AC-N4)
  - `cargo check --workspace --all-targets`
  - `cargo xtask build-guests --check && echo FRESH` before attributing any failure to this step
- Exit condition: both legs produce byte-identical commits for identical input; both the planar
  and the Z-spanning malformed cases are fatal with their exact substrings; the new contract test
  is registered and visible to libtest; AC-1 remains red pending Step 6.

### Step 5c: Widen the `layer-support` world to two builders (breaking WIT change, one commit)

- Task IDs: `TASK-512` (shared with Steps 5a and 5b — same task id, distinct step number, stated
  explicitly so no id outside `TASK-508`..`TASK-514` is minted)
- Objective: implement the approved resolution of the former `[BLOCK]` (`design.md` §Open
  Questions). Add `collection: layer-collection-builder` to the `use` list and to `run` in
  `crates/slicer-schema/wit/deps/layer-support/layer-support.wit`, placed after
  `output: support-output-builder`, so the signature matches the two-builder shape
  `crates/slicer-schema/wit/deps/layer-path-optimization/layer-path-optimization.wit` already
  uses. **This is a breaking change to an existing world, so every co-moving surface lands in
  this one commit** — the workspace does not compile green partway through.
- Complete edit set, each entry verified against this tree at authoring time:
  1. `crates/slicer-schema/wit/deps/layer-support/layer-support.wit` — `use` list + `run`.
  2. `crates/slicer-sdk/src/traits.rs` — `LayerModule::run_support` gains
     `_collection: &mut LayerCollectionBuilder` after `_output`, mirroring
     `LayerModule::run_path_optimization`'s parameter order. **Do not touch
     `LayerModule::run_support_postprocess`** — `Layer::SupportPostProcess` uses the separate
     `layer-support-postprocess` world.
  3. `crates/slicer-macros/src/lib.rs` — `build_layer_support_glue`'s
     `exports::…::support::Guest::run` signature and the `run_support` forwarding call — construct
     the SDK `LayerCollectionBuilder` and pass it, exactly as `build_layer_path_optimization_glue`
     does. **Do not add the `set-anchored-event-collection` drain call here**; that lands on both
     legs together in Step 6, which is why AC-8 is red until then. Also edit the native
     `"run_support"` arm of the
     stage-method `match`, including its `NativeLayerResponse` construction.
     `emit_world_preamble("support-module", "support", …)` needs no change — it re-reads the same
     `include_str!`ed WIT.
  4. `crates/slicer-sdk/src/native.rs` — **the non-wasm mirror does need the parameter.**
     `NativeLayerResponse.support` is `Option<SupportOutputBuilder>` today and carries no
     collection; give it one, following the `NativePathOptimizationOutput { output, collection }`
     precedent in the same file. `NativeLayerResponse` has five named fields, so any **test**
     literal of it needs `..` or an `// exhaustive: <reason>` waiver
     (`docs/21_data_defaults_and_fixtures.md`); production literals stay exhaustive.
  5. `crates/slicer-wasm-host/src/dispatch.rs` — the `"Layer::Support"` arm of the **layer-tier**
     `match stage_id.as_str()`, the one bound as
     `let (call_result, mut store, mem_initial_bytes) = ...`. Add a
     `push_layer_collection_builder` call and one more `own(collection)` argument to `call_run`,
     copying the `"Layer::PathOptimization"` arm. This is **two added lines in an existing arm**,
     not a new arm. **Do not touch the prepass-tier match** (bound as
     `let (call_result, mut store) = ...`), and do not touch Step 5a's
     `"Layer::AnchoredEvents"` arm.
  6. `crates/slicer-wasm-host/src/marshal/native.rs` — `commit_native_layer_response` (the
     native-leg function taking `response: &NativeLayerResponse`, whose `match` scrutinises
     `stage_export`, **not** `stage_id`) must read the new collection field, or the legs diverge
     and AC-6's both-legs invariant is broken for support.
     **Shared-arm note — read before budgeting this item.** The relevant arm is
     `"Layer::Support" | "Layer::SupportPostProcess" => { ... }`: one arm serving **both** stages.
     Because the new collection is carried only by `Layer::Support`, **splitting that arm so only
     the Support half reads the collection is part of this step** and is inside its `M` budget.
     Splitting the match arm does **not** constitute touching
     `LayerModule::run_support_postprocess` (item 2's prohibition): the `SupportPostProcess` half
     must keep its present behaviour byte-for-byte, and no trait method changes. Do not attempt to
     satisfy item 2 by leaving the arm shared and having `SupportPostProcess` read a field it can
     never receive.
  7. `modules/core-modules/tree-support/src/lib.rs` and
     `modules/core-modules/traditional-support/src/lib.rs` — the two production `Layer::Support`
     guests (`id = "Layer::Support"` in `tree-support.toml` / `traditional-support.toml`). Both
     override `run_support`, so both signatures change. **Neither uses the parameter here** — a
     production producer is 239c's. `modules/core-modules/support-surface-ironing` binds
     `Layer::SupportPostProcess` and is untouched.
  8. Permitted follow-on only if the signature change breaks them. **This list was re-derived
     against the tree by grepping for `fn run_support` definitions and `.run_support(` call sites
     (excluding `run_support_postprocess` / `run_support_geometry`); it is materially longer than
     the two files an earlier draft named, and every entry below is a real caller or implementor
     that the `run_support` arity change breaks.** Merely constructing a
     `SupportOutputBuilder::new()` does **not** break — only a `.run_support(...)` call or a
     `fn run_support` trait impl does.
     - `modules/core-modules/tree-support/tests/slicer_module_binding_tdd.rs` and
       `modules/core-modules/traditional-support/tests/slicer_module_binding_tdd.rs` — the
       original two (they contain no direct `.run_support(` call today, so they may well survive;
       kept in the list because they assert the generated binding surface).
     - `modules/core-modules/traditional-support/tests/support_fill_geometry_tdd.rs` — defines a
       **local helper** `fn run_support(points, angle, line_width)` that constructs
       `SupportOutputBuilder::new()` and calls `TraditionalSupport::run_support` directly. Both the
       helper and its inner call must gain the collection argument.
     - `modules/core-modules/traditional-support/tests/traditional_support_tdd.rs`,
       `traditional_family_tdd.rs`, and `enforcer_blocker_tdd.rs` — direct `.run_support(...)`
       call sites on `TraditionalSupport`.
     - `modules/core-modules/tree-support/tests/tree_support_tdd.rs`, `tree_family_tdd.rs`, and
       `enforcer_blocker_tdd.rs` — direct `.run_support(...)` call sites on the tree module.
     - `crates/slicer-runtime/tests/executor/live_layer_support_tdd.rs`,
       `crates/slicer-runtime/tests/integration/traditional_support_family.rs`, and
       `crates/slicer-runtime/tests/integration/tree_support_family.rs` — each calls
       `.run_support(layer_index, &regions, &paint, &mut output, &config)` on a natively
       constructed support module.
     - `crates/slicer-sdk/tests/layer_module_tdd.rs` — both a `fn run_support` trait impl on a
       local fixture module and direct `.run_support(...)` calls.
     - `crates/slicer-macros/tests/slicer_module_tdd.rs` and
       `crates/slicer-macros/tests/binding_surface_tdd.rs` — local `fn run_support` trait impls
       that the macro expands against; they break on the trait's new arity.
     Files that appear in a `SupportOutputBuilder::new()` grep but are **not** broken and must not
     be edited: `modules/core-modules/support-surface-ironing/tests/ironing_tdd.rs` and
     `ironing_scanline_parity_tdd.rs`, and
     `crates/slicer-runtime/tests/contract/native_dispatch_parity_seam_tdd.rs` — none calls
     `run_support`.
  9. **Advisory, not a compile break — flag, do not silently skip.**
     `crates/pnp-cli/src/module_new.rs` holds the `"Layer::Support"` scaffold template as a string
     literal spelling out the old five-parameter `run_support` signature. Nothing fails to compile,
     but every newly scaffolded `Layer::Support` module would be born non-compiling, and
     `crates/slicer-macros/src/lib.rs`'s own header comment states that `#[slicer_module]` and the
     `module new` scaffold "stay in lock-step". Updating that one string is permitted in this step;
     if it is deliberately deferred, the deferral must be recorded rather than left implicit.
- **`xtask/src/wit_verify.rs` is NOT edited by this step, and its three `20`s are NOT affected.**
  Every `20` there counts distinct `.wit` **file paths** (the macro `include_str!` set, the
  canonical-set comparison, and `build.rs`'s `rerun-if-changed` watch set); none inspects a `run`
  signature, and `layer-support.wit` is already in all three sets. The separate 20→21 change this
  packet owns belongs to Step 2's **new** `layer-anchored-events.wit` file. Conflating the two
  yields 22 and a red audit.
- Precondition: Steps 5a and 5b landed. Independent of Step 6, but sequenced before it so the SDK
  drain work in Step 6 sees the final trait shape.
- Postcondition: the workspace compiles and every previously-green AC stays green; a
  `Layer::Support` guest now *receives* a `layer-collection-builder` handle. Nothing yet proves
  content crosses — that is Step 5d.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-schema/wit/deps/layer-path-optimization/layer-path-optimization.wit` — whole
    (~20 lines); the precedent to copy
  - `crates/slicer-sdk/src/traits.rs` — over 300 lines: `run_support`, `run_support_postprocess`,
    and `run_path_optimization` only
  - `crates/slicer-macros/src/lib.rs` — over 300 lines: `build_layer_support_glue`,
    `build_layer_path_optimization_glue`, and the native `"run_support"` /
    `"run_path_optimization"` match arms only
  - `crates/slicer-wasm-host/src/dispatch.rs` — over 300 lines: the `"Layer::Support"` and
    `"Layer::PathOptimization"` arms of the layer-tier `match stage_id.as_str()` only
  - `crates/slicer-sdk/src/native.rs` — `NativeLayerResponse` and
    `NativePathOptimizationOutput` only
  - `modules/core-modules/tree-support/src/lib.rs`,
    `modules/core-modules/traditional-support/src/lib.rs` — both over 300 lines: the
    `impl LayerModule` block's `run_support` signature only
  - `docs/21_data_defaults_and_fixtures.md` — the struct-literal churn-gate rule only
  - every test file named in item 8 — the `fn run_support` / `.run_support(` sites only, never the
    whole file; and `crates/pnp-cli/src/module_new.rs` — the `"Layer::Support"` template arm only
- Files allowed to edit: the nine entries enumerated above — including every file named in item 8's
  re-derived follow-on list and item 9's advisory — and nothing else.
- Files explicitly out of bounds:
  - `xtask/src/wit_verify.rs` (its counts are unaffected; see above),
    `crates/slicer-schema/wit/deps/layer-support-postprocess/**`,
    `crates/slicer-schema/wit/deps/layer-anchored-events/**`,
    `crates/slicer-schema/wit/deps/ir-types.wit` (the resource already carries the method after
    Step 2), `modules/core-modules/support-surface-ironing/**`,
    `crates/slicer-runtime/src/layer_executor.rs`
- Expected sub-agent dispatches:
  - Question: return the `"Layer::Support"` and `"Layer::PathOptimization"` arms of the
    `match stage_id.as_str()` bound as `let (call_result, mut store, mem_initial_bytes) = ...` in
    `crates/slicer-wasm-host/src/dispatch.rs`, so the two added lines are placed by diff rather
    than by guess; scope: that file; return: `SNIPPETS` ≤ 60 lines
  - Question: every `run_support` override and every `NativeLayerResponse` literal in
    `crates/`, `modules/`, and `xtask/`, so no implementor is missed; scope: those trees;
    return: `LOCATIONS` ≤ 20 entries
- Context cost: `M`
- Authoritative docs:
  - `CLAUDE.md` §"WIT/Type Changes Checklist" — direct read; all four steps apply, and step 2
    (type identity across the component boundary) is what a half-applied signature surfaces as
  - `CLAUDE.md` §"Guest WASM Staleness" — direct read; every guest is stale after this edit
  - `docs/21_data_defaults_and_fixtures.md` — the churn-gate rule for `NativeLayerResponse`
- OrcaSlicer refs:
  - none — this is a PnP-internal component contract with no canonical analogue
- Verification:
  - `cargo build --workspace --all-targets` — FACT pass/fail
  - `cargo xtask build-guests` then `cargo xtask build-guests --check && echo FRESH` — exit code
    only, never `rg 'STALE:'`
  - `mkdir -p target && cargo test -p slicer-runtime --test executor -- anchored_events_roundtrip_tdd::anchored_event_collection_round_trips_with_exact_canonical_z --exact 2>&1 | tee target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` — AC-1 must still pass if Step 6 already landed; if 5c runs before Step 6 it stays red for Step 6's documented reason, not a new one
  - `mkdir -p target && cargo test -p slicer-wasm-host --test contract -- anchored_events_both_legs_tdd::anchored_events_native_and_wasm_legs_agree --exact 2>&1 | tee target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` — AC-6 unchanged
  - `mkdir -p target && cargo test -p slicer-runtime --test executor -- live_layer_support_tdd:: 2>&1 | tee target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` — the existing live support-stage coverage must not regress (module-prefix substring filter, no `--exact`, because the set of tests in that file must be re-derived rather than pinned)
  - `cargo test -p xtask --bin xtask wit_verify 2>&1 | tee target/test-output.log` — the counts must still read `21`, proving this step moved none of them
  - `cargo clippy --workspace --all-targets -- -D warnings`
  - `cargo xtask check-literals`
- Exit condition: `layer-support.wit`'s `run` carries both builders; the workspace builds and
  clippies clean with `--all-targets`; every previously-green AC is still green; the
  `wit_verify` counts still read `21`; `git diff` shows no edit to `xtask/src/wit_verify.rs`, to
  the prepass-tier match, or to `run_support_postprocess`.

### Step 5d: Prove a `Layer::Support` guest reaches the drain (AC-8)

- Task IDs: `TASK-512` (shared with Steps 5a, 5b, and 5c — same task id, distinct step number,
  stated explicitly so no id outside `TASK-508`..`TASK-514` is minted)
- Objective: author the first `Layer::Support` test guest and the content assertion that proves
  the widening did what it was chosen for. **Verified: no `Layer::Support` test guest exists
  today** — `crates/slicer-wasm-host/test-guests/dispatch-layer-support-postprocess-guest` binds
  the *postprocess* world and `sdk-support-diagnostic-guest` implements `run_support_geometry`
  (a prepass stage), so this guest is new. Test-guests are discovered by the `tg_root` directory
  scan in `xtask/src/build_guests.rs`; there is no guest list to append to.
- **How the guest binds to `Layer::Support` — mechanism, verified against this tree.** The guest
  implements `LayerModule::run_support` under `#[slicer_module]`
  (`crates/slicer-macros/src/lib.rs`), making it the first test-guest to implement that stage
  method; `#[slicer_module]` detects the stage from the method name. The **test**, not a manifest,
  supplies the `"Layer::Support"` stage string — as the `stage_id` argument to
  `LayerStageRunner::run_stage` (`crates/slicer-wasm-host/src/traits.rs`) and as the `stage`
  argument to `LoadedModuleBuilder::new(id, version, stage, _legacy_world, wasm_path)`
  (`crates/slicer-scheduler/src/manifest.rs`).
  **Do not author a module-manifest TOML for this guest.** There are none anywhere under
  `crates/slicer-wasm-host/test-guests/` (only `Cargo.toml` / `Cargo.lock` pairs), and
  `xtask/src/build_guests.rs` sets `stage_id: None` unconditionally on its `GuestTree::TestGuest`
  branch. Manifest-bound `stage.id` is read by `parse_stage_id_from_module_manifest` for
  `modules/core-modules/<name>/<name>.toml` only and is unreachable from a test-guest.
- The guest's `run_support` calls
  `LayerCollectionBuilder::set_anchored_event_collection` with a one-event
  `ordered-event-collection` anchored at global layer 7 whose sole `anchored-entity` carries the
  canonical `s64` anchor Z `1_234_567` units — deliberately not exactly representable in `f32`,
  so any float hop corrupts it and the test fails.
- Precondition: Step 5c landed; a `Layer::Support` guest receives a `layer-collection-builder`.
- Postcondition: **AC-8 is authored RED here and turns green in Step 6**, exactly as AC-1 does —
  the SDK drain that forwards `anchored_proposal()` through the WIT method does not exist until
  Step 6, so the accumulator is never populated before then. The red must be a *missing-commit*
  assertion failure, never an instantiation or unknown-stage error. **Do not weaken AC-8 to make
  it green early.** When Step 6 lands, a support-stage guest's anchored proposal arrives
  host-side as `LayerStageCommit::AnchoredEvents` with `anchor_global_layer_index == 7`, one
  event, and the anchor Z equal to exactly `1_234_567` under `assert_eq!` on the `i64` value.
- **Scope note on the one-commit-per-dispatch shape.** `deconstruct_layer_ctx` returns **one**
  commit per dispatch, so a production `Layer::Support` dispatch cannot return both a `Support`
  and an `AnchoredEvents` commit from that single return. AC-8 asserts the proposal **reaches the
  host and converts** with its Z intact; how a production pipeline routes both commits from one
  support dispatch is `239c-support-layer-height-producer`'s to specify. Do not widen this step
  to solve it, and do not weaken AC-8 to avoid it.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-wasm-host/test-guests/finalization-mutation-roundtrip-guest/src/lib.rs` +
    `Cargo.toml` — both whole; the guest template
  - `crates/slicer-runtime/tests/executor/anchored_events_roundtrip_tdd.rs` — whole; the
    harness shape AC-8 reuses
  - `crates/slicer-runtime/tests/executor/main.rs` — the `mod` list only, to confirm the
    prefix convention at edit time
  - `crates/slicer-sdk/src/layer_collection_builder.rs` — whole
  - `crates/slicer-wasm-host/src/traits.rs` — `LayerStageRunner::run_stage`'s signature only, to
    confirm the `stage_id` argument the test supplies
  - `crates/slicer-scheduler/src/manifest.rs` — `LoadedModuleBuilder::new`'s signature only
- Files allowed to edit (2 new + one one-line registration):
  - `crates/slicer-wasm-host/test-guests/support-anchored-reach-guest/` (new crate: `Cargo.toml`,
    `src/lib.rs`, a `[workspace]` sentinel)
  - `crates/slicer-runtime/tests/executor/support_anchored_reach_tdd.rs` (new; AC-8's home)
  - registration: `crates/slicer-runtime/tests/executor/main.rs` — one line,
    `mod support_anchored_reach_tdd;`. Without it the file never compiles and AC-8 reports a
    green run over zero tests.
- Files explicitly out of bounds:
  - all of `crates/slicer-schema/wit/**` (Step 5c is closed), `crates/slicer-sdk/src/**`,
    `crates/slicer-wasm-host/src/**`, `modules/core-modules/**`
- Expected sub-agent dispatches:
  - Question: confirm `crates/slicer-runtime/tests/executor/main.rs` still declares no top-level
    `#[test]` wrapper functions, so libtest reports `<mod>::<fn>` and AC-8's prefix
    `support_anchored_reach_tdd::` is correct; scope: that file; return: `FACT` yes/no
- Context cost: `M`
- Authoritative docs:
  - `CLAUDE.md` §"Guest WASM Staleness" — direct read
  - `CLAUDE.md` §"Coordinate System Hazard" — direct read; `1 unit = 100 nm`, so `1_234_567`
    units is `0.1234567 mm` and must never be reconstructed from a millimetre `f32`
- OrcaSlicer refs:
  - none
- Verification:
  - `cargo xtask build-guests` then `cargo xtask build-guests --check && echo FRESH`
  - `mkdir -p target && cargo test -p slicer-runtime --test executor -- --list 2>&1 | tee target/test-output.log && test "$(grep -c '^support_anchored_reach_tdd::' target/test-output.log)" -gt 0` — the `mod` registration landed and libtest can see the file
  - `mkdir -p target && cargo test -p slicer-runtime --test executor support_anchored_reach_tdd 2>&1 | tee target/test-output.log && grep -cE "^test .+ FAILED|panicked at" target/test-output.log` — AC-8 is expected RED at the end of this step; read the log and confirm the failure is a missing-commit assertion, not instantiation or "unknown stage". The green form of this command is AC-8's own, run in Step 6:
  - `mkdir -p target && cargo test -p slicer-runtime --test executor -- support_anchored_reach_tdd::support_stage_guest_reaches_anchored_drain_with_exact_canonical_z --exact 2>&1 | tee target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` (AC-8; PASSes from Step 6 onward)
  - `cargo check --workspace --all-targets`
  - `cargo clippy --workspace --all-targets -- -D warnings`
- Exit condition: the guest is instantiated and called for `Layer::Support`; AC-8 exists,
  compiles, is visible to libtest under its module prefix, and fails on a missing commit with the
  exact `s64` assertion intact; the guest freshness check exits `0`.

### Step 6: SDK drain glue — make the proposal cross the boundary

- Task IDs: `TASK-513`
- Objective: wire the guest-side drain so `LayerCollectionBuilder::anchored_proposal`
  (`crates/slicer-sdk/src/layer_collection_builder.rs`) is read at the end of a
  `run_anchored_events` dispatch and forwarded through the new WIT
  `set-anchored-event-collection` method; mirror the same path in the native adapter
  (`crates/slicer-sdk/src/native.rs`) and the test capture sink
  (`crates/slicer-sdk/src/test_support/capture.rs`). Preserve the existing double-call rejection
  in `set_anchored_event_collection` and update its message to name the anchored-events dispatch
  rather than `run-path-optimization`.
  The **same drain must be emitted on the `layer_support` leg**, since Step 5c gave that world a
  `layer-collection-builder`: `build_layer_support_glue` reads `anchored_proposal()` at the end of
  its `run_support` dispatch and calls the new WIT method, identically to the anchored-events and
  path-optimization legs. Without this, AC-8 stays red.
- Precondition: Steps 5a-5d landed; both producer arms present; the `layer-support` world carries
  two builders; AC-1 and AC-8 red on an empty accumulator.
- Postcondition: **AC-1, AC-2, AC-3, AC-8, AC-N2, and AC-N3 all turn green in this step** — this
  is the step that closes the round trip, on both the anchored-events and support legs.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-sdk/src/layer_collection_builder.rs` — whole
  - `crates/slicer-sdk/src/builders.rs` — over 300 lines: `SupportOutputBuilder` only (the drain
    pattern to copy)
  - `crates/slicer-sdk/src/native.rs` — over 300 lines: the layer-collection mirror only
  - `crates/slicer-sdk/src/test_support/capture.rs` — the layer-collection sink only
- Files allowed to edit (4):
  - `crates/slicer-sdk/src/layer_collection_builder.rs`
  - `crates/slicer-sdk/src/native.rs`
  - `crates/slicer-sdk/src/test_support/capture.rs`
  - `crates/slicer-macros/src/lib.rs` — the drain call at the end of **both**
    `build_layer_anchored_events_glue` and `build_layer_support_glue` (the latter is what turns
    AC-8 green). Signatures are Step 5c's and must not be re-touched here.
- Files explicitly out of bounds:
  - `crates/slicer-wasm-host/src/**` (Steps 4-5 are closed), `crates/slicer-schema/wit/**`,
    `modules/core-modules/**`, `crates/slicer-macros/build.rs`, `xtask/src/wit_verify.rs`
- Expected sub-agent dispatches:
  - Question: how does the SDK's `SupportOutputBuilder` forward accumulated output through its
    WIT resource on the wasm leg, and where does the native mirror diverge?; scope:
    `crates/slicer-sdk/src/builders.rs`, `crates/slicer-sdk/src/native.rs`; return: `SNIPPETS`
    ≤ 30 lines
- Context cost: `M`
- Authoritative docs:
  - `CLAUDE.md` §"Config Key Naming Convention" — the guest's fixture keys stay snake_case
- OrcaSlicer refs:
  - none
- Verification:
  - `cargo xtask build-guests` then `cargo xtask build-guests --check && echo FRESH`
  - `mkdir -p target && cargo test -p slicer-runtime --test executor -- anchored_events_roundtrip_tdd::anchored_event_collection_round_trips_with_exact_canonical_z --exact 2>&1 | tee target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` (AC-1)
  - `mkdir -p target && cargo test -p slicer-runtime --test executor -- anchored_events_roundtrip_tdd::anchored_runtime_hooks_survive_the_boundary_unaltered --exact 2>&1 | tee target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` (AC-2)
  - `mkdir -p target && cargo test -p slicer-runtime --test executor -- anchored_events_roundtrip_tdd::anchored_provenance_and_capability_order_preserved --exact 2>&1 | tee target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` (AC-3)
  - `mkdir -p target && cargo test -p slicer-runtime --test executor -- anchored_events_roundtrip_tdd::guest_emitting_no_anchored_events_produces_no_commit --exact 2>&1 | tee target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` (AC-N2)
  - `mkdir -p target && cargo test -p slicer-runtime --test executor -- anchored_events_roundtrip_tdd::duplicate_anchored_proposal_is_rejected_and_commits_nothing --exact 2>&1 | tee target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` (AC-N3)
  - `mkdir -p target && cargo test -p slicer-runtime --test executor -- support_anchored_reach_tdd::support_stage_guest_reaches_anchored_drain_with_exact_canonical_z --exact 2>&1 | tee target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` (AC-8)
- Exit condition: all six commands PASS; a guest-emitted `ordered-event-collection` arrives on
  the host as `LayerStageCommit::AnchoredEvents` with exact `s64` fidelity **from both the
  `Layer::AnchoredEvents` and the `Layer::Support` legs**.

### Step 7: Docs, registration, and guest-freshness reconciliation

- Task IDs: `TASK-514`
- Objective: extend the existing `### anchored entity IR (additive)` section of
  `docs/02_ir_schemas.md` with a WIT-transport subsection (the section already exists and
  describes the types at IR level only — this **extends**, it does not create); register
  `TASK-508`..`TASK-514` in `docs/07_implementation_status.md`; add the gap-register row for F7;
  update queue row 2 of `docs/specs/support-independent-layer-z-split-plan.md`; and reconcile
  guest freshness across the whole tree.
- Precondition: Steps 1-6 landed; every AC command except AC-7 already PASS.
- Postcondition: every Doc Impact grep in `packet.spec.md` returns a match; AC-7 returns exit
  `0`; `packet.spec.md` is ready for `status: implemented`.
- Files allowed to read, with ranges when over 300 lines:
  - `docs/02_ir_schemas.md` — over 300 lines: the `### anchored entity IR (additive)` section
    under `## IR 10 — LayerCollectionIR`, plus `## IR Versioning Contract`, only
  - `docs/specs/support-independent-layer-z-split-plan.md` — the queue table and gap-register
    note only
- Files allowed to edit (at most 3 primary; the two spec-doc updates are single-row edits):
  - `docs/02_ir_schemas.md`
  - `docs/07_implementation_status.md`
  - `docs/specs/support-parity-gap-register.md`
  - blast radius: `docs/specs/support-independent-layer-z-split-plan.md` (queue row 2)
- Files explicitly out of bounds:
  - all `crates/**` and `modules/**` source; other packet directories under `docs/spec_packets/`
    (Packet Safety); `docs/15_config_keys_reference.md`
- Expected sub-agent dispatches:
  - Question: re-derive the current `TASK-###` high-water mark and the next free `G-` row;
    scope: `docs/07_implementation_status.md`, `docs/specs/support-parity-gap-register.md`;
    return: `FACT` (two values). **Do not reuse the split-plan's frozen values — another packet
    may have claimed them.**
  - Question: append the seven task rows to `docs/07_implementation_status.md`; scope: that file;
    return: `FACT` applied/not-applied. Never full-read the backlog.
  - Question: confirm no component/serialization boundary for support or anchored events exists
    in canonical; scope: `OrcaSlicerDocumented/`; return: `FACT`; purpose: discharge the weak
    parity obligation honestly rather than by assumption.
- Context cost: `S`
- Authoritative docs:
  - `docs/02_ir_schemas.md` — the two named sections, ranged read only
- OrcaSlicer refs:
  - none, by the FACT dispatch above
- Verification:
  - `rg -q 'layer-anchored-events' docs/02_ir_schemas.md && rg -q 'anchored entity IR \(additive\)' docs/02_ir_schemas.md && echo DOC_OK`
  - `rg -q 'TASK-508' docs/07_implementation_status.md && rg -q 'TASK-514' docs/07_implementation_status.md && echo BACKLOG_OK`
  - `rg -q '239b-anchored-wit-contract' docs/specs/support-parity-gap-register.md && echo GAP_OK`
  - `rg -q '239b-anchored-wit-contract' docs/specs/support-independent-layer-z-split-plan.md && echo PLAN_OK`
  - `cargo xtask build-guests --check && echo FRESH` (AC-7)
  - `cargo clippy --workspace --all-targets -- -D warnings`
  - `cargo xtask check-literals`
- Exit condition: all four doc greps print their OK marker, AC-7 exits `0`, and both commit gates
  pass.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | S | read-only inventory; three bounded dispatches, no edits |
| Step 2 | M | the declaration plus its eight-file cross-checked blast radius; largest step |
| Step 3 | M | new guest crate + new test file + one trait method; ends deliberately red |
| Step 4 | M | bindgen world module + accumulator/field/method/converter + the duplicated geometry validator, copied from the support-builder chain |
| Step 5a | M | `TASK-512`; the layer-tier linker/instantiate/call arm — the surface that actually runs the guest |
| Step 5b | M | `TASK-512` (same id, distinct step); both producer legs in one commit; both-legs guard |
| Step 5c | M | `TASK-512` (same id, distinct step); the breaking `layer-support` two-builder widening and its eight-entry blast radius, one commit |
| Step 5d | M | `TASK-512` (same id, distinct step); first `Layer::Support` test guest + AC-8, authored red |
| Step 6 | M | SDK drain on both legs; the step where AC-1/2/3, AC-8, and AC-N2/N3 turn green |
| Step 7 | S | docs, registration, freshness reconciliation |

Steps 5a, 5b, 5c, and 5d **all share `TASK-512`**; no task id outside `TASK-508`..`TASK-514` is
minted by the split, and the shared id with distinct step numbers is stated explicitly in each
step. 5a and 5b are one verification atom (5a runs a guest nothing consumes; 5b's arms are
unreachable without 5a) but two commits, because a linker/instantiate/call arm is a copied
40-plus-line pattern, not the one-to-three-line mechanical edit the blast-radius caps elsewhere in
this plan assume. 5c is split out from 5a/5b rather than folded into either because it is a
**breaking change to a different, pre-existing world** whose blast radius reaches
`crates/slicer-sdk/`, `crates/slicer-macros/`, and `modules/core-modules/` — trees 5a and 5b do
not touch at all — and folding it in would have pushed either step to `L`. 5d is split from 5c
because it adds a new test-guest crate and a new test file, which is a different kind of edit from
a signature migration; keeping them apart holds both at `M`. Neither split mints an id.

Aggregate: `M`. No step is `L`. If Step 2 measures as `L` in practice, split it at the boundary
recorded in `design.md` §Risks (2a = `.wit` + stage tables; 2b = macro/build.rs/wit_verify/
ADR-0020 gate) and treat 2a+2b as one verification atom — the workspace does not compile green
between them.

## Packet Completion Gate

- All nine steps (1, 2, 3, 4, 5a, 5b, 5c, 5d, 6, 7 — nine step bodies under seven task ids) and
  their exit conditions complete.
- The `layer-support` world carries both builders and AC-8 PASSes, proving a `Layer::Support`
  guest reaches the drain with its `s64` anchor Z intact.
- `xtask/src/wit_verify.rs`'s three count expectations read `21` — moved once, by Step 2's new
  file, and not at all by Step 5c's signature edit.
- Every pipe-suffixed AC command in `packet.spec.md` returns PASS, including all three negatives.
- `cargo xtask build-guests --check` returns exit `0` on a tree with every guest rebuilt.
- `docs/07_implementation_status.md` updated through a worker dispatch, never a full backlog
  read, with the task-id high-water mark re-derived at edit time.
- Superseded-status reconciliation: `docs/spec_packets/239-support-independent-layer-z/` names
  this packet among its successors; queue row 2 of
  `docs/specs/support-independent-layer-z-split-plan.md` is closed.
- `packet.spec.md` is ready for `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed AC command and every packet-level gate command.
- Run `cargo xtask test --summary --workspace --no-fail-fast` **once**, dispatched to a sub-agent
  with a `FACT pass/fail` return; never absorb the full output. This is the packet-close case
  permitted by `CLAUDE.md` Test Discipline, and it runs only after every narrower command above
  has passed. Reconcile the test-binary count against the narrow runs: a binary-count drop
  between a narrow run and the workspace run means the narrow run was blind.
- Record remaining packet-local risk: the transport has no production producer until
  `239c-support-layer-height-producer` lands. Closure language must say so.
- Confirm context stayed at or below 150k standard, or at/below 300k only with a logged swarm
  ESCALATION; otherwise record a packet-authoring lesson.

All `cargo check`, `cargo clippy`, and `cargo test` invocations in gate and verification commands
use `--all-targets` where the target set matters, so test, bench, and example targets compile.

## Implementation Deviations (recorded during the swarm run)

1. Step 2's `set-anchored-event-collection` WIT method addition was deferred to Step 4 to land with its host impl (the bindgen-generated `ir::HostLayerCollectionBuilder` trait requires the impl, so Step 2's green gates were impossible otherwise).
2. The packet's `cargo test -p slicer-scheduler --test contract` command is stale: the target is `scheduler_contract` (verified: `crates/slicer-scheduler/Cargo.toml` declares `[[test]] name = "scheduler_contract"`).
3. Step 3's edit list omitted the macro WIT glue (`StageGlueKind::LayerAnchoredEvents`, `resolve_stage_glue` arm, `build_layer_anchored_events_glue`) — required for the test guest to compile; the native-entry `"run_anchored_events"` match arm and the `NativeLayerResponse.anchored_events` carrier were deferred to Step 5b.
4. The packet's Step 5b postcondition claims AC-N1/AC-N4 pass at 5b; they require the Step 6 drain (the real-guest malformed proposal cannot reach the host before it), so they turn green at Step 6 — the packet's Step 6 green list omits them.
5. `xtask/src/wit_verify.rs` has a fourth count the packet missed: the per-stage count 15→16; and `crates/slicer-schema/src/lib.rs`'s `stage_table_has_one_entry_per_routed_export` test asserts `STAGES.len() == 16` with comment "Layer world: 8" — both moved to 17/"Layer world: 9".
6. `deconstruct_layer_ctx` was made `pub` in `crates/slicer-wasm-host/src/dispatch.rs` so AC-6's wasm leg can call it directly.
7. The support-arm anchored surfacing (design.md §Code Change Surface item 7, conditional) is required by AC-8 and was implemented in Step 5d on both legs: `deconstruct_layer_ctx`'s support arm and `commit_native_layer_response`'s Support arm surface `LayerStageCommit::AnchoredEvents` when support output is empty and an anchored proposal is present.
8. AC-N3's test assertion was adjusted: the original `format!("{result:?}").contains("ModuleError")` is a brittle Debug-representation proxy (the layer-tier dispatch wraps module errors as `FatalModule`); it now asserts the dispatch error's message contains a stable substring of the SDK's double-call rejection message.
9. The lift-time validation inside `convert_anchored_events` (added in Step 4) was removed in Step 6: it turned validation failures into traps via `map_err(wasmtime::Error::msg)?`, breaking AC-N1/AC-N4's `FatalModule` message assertions; validation belongs to the producer arm per the packet's design.
10. `build_layer_support_glue` uses `layer_light_helpers()` with construct-and-forward of the SDK `LayerCollectionBuilder` (no entity-order populate/drain): the light and heavy helper sets are not nested, and the anchored drain (Step 6) reads the SDK proposal directly. The Step 5c worker's helper-set swap (perimeters-postprocess to light) was reverted — it broke the fuzzy-skin guest.

Review-time repairs (2026-08-30 closure review, after the acceptance-ceremony run):

11. Two stage-count meta-tests outside the packet's planned blast radius still counted 15 WASM-dispatched stages: `wit_single_source_tdd::host_bindgen_paths_target_shared_root` (`crates/slicer-runtime/tests/contract/wit_single_source_tdd.rs`, "exactly 15 `path:` literals") and `module_new::tests::generated_manifest_comments_advertise_schema_derived_exports` (`crates/pnp-cli/src/module_new.rs`, `checked == 15`). Both expectations were raised to 16 — the ninth `Layer::*` stage is real, not drift. Close-out note: Step 1's LOCATIONS inventory (capped at ≤ 20 entries) missing a counter is exactly the completeness risk the packet's own §Risks records; a ninth stage must sweep *every* numeric stage-count literal, not only the named tables.
12. The 14 `ExtrusionPath3D` doc-example literals in `crates/slicer-sdk/src/test_support/{assert_paths.rs,capture.rs}` predate pnp-244 (`25398ebf`), which added `ExtrusionPath3D.order_lock` and updated fixtures but not these doc examples; all 13 failing doctests were repaired by adding `order_lock: None` (note: field order follows the canonical struct — `tool_index`, then `order_lock`).
13. Two further pre-existing failures were repaired at review time rather than left red, each traced to a commit outside this packet: (a) `runtime_wiring_tdd::config_schema_json_matches_documented_shape` lagged the 1.0.0 -> 1.1.0 wire bump from `a50bfc28` (documented as "whoever owns a50bfc28's fallout" in the 239a packet) — the test now compares against `slicer_scheduler::manifest::CONFIG_SCHEMA_WIRE_VERSION` so it cannot lag again; (b) `manifest_ingestion_tdd::{core_modules_directory_is_discoverable_and_all_load, core_modules_all_have_placeholder_wasm_flag_set}` lagged pnp-244b's real `com.core.wave-overhangs` module (count 22 -> 23; `com.core.wave-overhangs` added to `NON_PLACEHOLDER` — the flag is *derived* from wasm file size via `is_placeholder_wasm`, `crates/slicer-scheduler/src/manifest.rs`, not a manifest key). The two `calicat_internal_bridge_*_e2e_tdd` `Os code 3 NotFound` failures did not reproduce on re-run and were diagnosed as ceremony-time contention on `crates/slicer-runtime/target/` (the directory exists and is writable; no code change made).

Also note: the review's Low finding that `crates/slicer-runtime/tests/contract/native_dispatch_parity_seam_tdd.rs` was edited (6 lines, churn-gate repair) although design.md item 8 lists it as "not broken, do not edit" — the design analysis considered only the `run_support` arity change, not the `NativeLayerResponse.support` type change (`Option<SupportOutputBuilder>` → `Option<NativeSupportOutput>`).
