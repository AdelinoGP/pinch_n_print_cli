# Implementation Plan: 226-authored-coloring-carrier

## Execution Rules

- Work one atomic step at a time; map every step to grouped task IDs.
- Use TDD, then implementation, then the narrowest falsifying validation.
- Every field below is a context-budget contract and must be filled independently; never write "see Step 1".

## Steps

### Step 1: WIT field + IR mirror + schema bump + full production literal blast radius

- Task IDs: `TASK-337`
- Objective: Add `tool-index: option<u32>` to `extrusion-path3d`, add `pub tool_index: Option<u32>` (`#[serde(default)]`) to `ExtrusionPath3D`, bump `CURRENT_LAYER_COLLECTION_IR_SCHEMA_VERSION` from 1.2.0 additively, and close every production `src/` struct literal so `cargo check --workspace --all-targets` can compile the new field.
- Precondition: packet 225 is `implemented` (toolchain bumped, guests green) so this step compiles on the post-bump toolchain.
- Postcondition: the WIT record, the IR struct, and the schema constant all carry the new field/bump; every production `ExtrusionPath3D { ... }` literal gains `tool_index: None` (or the real value where the site already has tool context); `crates/slicer-ir/tests/ir_tests.rs`'s `LayerCollectionIR::default().schema_version` assertion still passes against the bumped constant.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-schema/wit/deps/types.wit` - lines 1-22
  - `crates/slicer-ir/src/slice_ir.rs` - lines 185-327 (constants), 1744-1972 (structs)
  - `crates/slicer-ir/tests/ir_tests.rs` - lines 745-760 (the schema-version assertion)
  - every production literal site from the Step-1 dispatch (below) - grep-confirmed line only
- Files allowed to edit (at most 3, extended by blast-radius discipline):
  - `crates/slicer-schema/wit/deps/types.wit`
  - `crates/slicer-ir/src/slice_ir.rs` (struct + const + doc comment)
  - `crates/slicer-ir/tests/ir_tests.rs` (bump fallout — the `LayerCollectionIR::default()` assertion now references the bumped constant; it already compares to the constant, so no literal change is needed unless it hard-pins a value, but verify)
  - Blast-radius list (production src literals): every enumerated production `ExtrusionPath3D {` site gains `tool_index: None` except the four `slicer-macros` WIT↔IR path converters, which gain the real `tool_index: p.tool_index` in Step 3 (they are the only production sites with a WIT source to map from). The production literal files are: `crates/slicer-core/src/perimeter_utils.rs`, `crates/slicer-runtime/src/layer_executor.rs`, `crates/slicer-gcode/src/emit.rs`, `crates/slicer-ir/src/slice_ir.rs` (6 sites incl. `variable_width`), `crates/slicer-wasm-host/src/host.rs`, `crates/slicer-wasm-host/src/marshal/{in_,native,leaf}.rs`, `crates/slicer-sdk/src/test_support/{capture,fixtures,assert_paths}.rs`, `crates/slicer-wasm-host/test-guests/sdk-{finalization,layer-infill}-guest/src/lib.rs`, and the 11 module src files (`fuzzy-skin`, `skirt-brim`, `wipe-tower`, `top-surface-ironing`, `traditional-support`, `tree-support`, `gyroid-infill`, `classic-perimeters`, `rectilinear-infill`, `support-surface-ironing`, `lightning-infill`). All production literals stay exhaustive per docs/21.
- Files explicitly out of bounds:
  - `target/`, `Cargo.lock`, `**/wit-guest/**` generated code, `OrcaSlicerDocumented/`, test files (test fallout is Step 2)
- Blast-radius discipline (mandatory): before editing, dispatch a `LOCATIONS` worker for every `ExtrusionPath3D {` (IR) and `ExtrusionPath3d {` (WIT) literal site under `crates/` and `modules/`, split production-src vs test. Grounded counts (authoring time): 47 production IR literals across 14 crate src files + 19 across 11 module src files; 121 test IR literals across 62 files; 33 module test IR literals across 18 files; 35 WIT literals across 18 files. Budget each family in the relevant step; this step owns only the production-src half.
- Expected sub-agent dispatches:
  - Question: `LOCATIONS` of every `ExtrusionPath3D {` and `ExtrusionPath3d {` literal, split (production src / test / fixture / test-guest); scope: `crates/ modules/`; return: `LOCATIONS`
  - Question: `LOCATIONS` of every `CURRENT_LAYER_COLLECTION_IR_SCHEMA_VERSION` hard-asserting test; scope: `crates/ modules/`; return: `LOCATIONS`
- Context cost: `M`
- Authoritative docs:
  - `docs/21_data_defaults_and_fixtures.md` - direct read (production-exemption rationale)
- OrcaSlicer refs:
  - none (no parity obligation)
- Verification:
  - `rg -n 'record extrusion-path3d \{' -A1 crates/slicer-schema/wit/deps/types.wit | rg 'tool-index: option<u32>'` - FACT pass
  - `rg -n 'pub tool_index: Option<u32>' crates/slicer-ir/src/slice_ir.rs` - FACT pass
  - `python3 -c "…"` (AC-6, const > 1.2.0) - FACT pass
  - `cargo check --workspace --all-targets 2>&1 | tail -30` - FACT exit 0 (proves the production blast radius is closed; test fallout is Step 2)
- Exit condition: all four verification commands pass; AC-1, AC-2, AC-6 satisfied at the production-src level.

### Step 2: Close the test/fixture struct-literal blast radius

- Task IDs: `TASK-337`
- Objective: Update every test/fixture/guest `ExtrusionPath3D {` and WIT `ExtrusionPath3d {` literal so all targets compile after the new field, per docs/21 (prefer `..Default::default()` in test fixtures; the converters and any site with real tool context get the real value in Step 3).
- Precondition: Step 1 compiled the production half; the Step-1 `LOCATIONS` dispatch enumerated the test half.
- Postcondition: every test/fixture/guest literal compiles; the SDK `test_support` fixtures (`fixtures.rs`, `capture.rs`, `assert_paths.rs`) use `..Default::default()` so downstream tests inherit the new field transparently.
- Files allowed to read, with ranges when over 300 lines:
  - each test/fixture/guest literal site from the dispatch - grep-confirmed line only
- Files allowed to edit (at most 3, extended by blast-radius discipline):
  - the 62 crate test files, 18 module test files, and test-guest src files enumerated by the dispatch (each is a one-line `tool_index: None` or a FRU conversion)
- Files explicitly out of bounds:
  - `target/`, `Cargo.lock`, `**/wit-guest/**` generated code, `OrcaSlicerDocumented/`
- Blast-radius discipline (mandatory): cite the dispatch counts inline. This step deliberately owns the test half; it is mechanical (no logic), and the `ExtrusionPath3D` struct stays below the docs/21 watchlist threshold (4 fields), so FRU is optional — shared SDK fixtures get FRU, isolated test literals get explicit `tool_index: None`.
- Expected sub-agent dispatches:
  - Question: for the test/fixture/guest files, list the exact literal line and whether the surrounding test already constructs `Point3WithWidth` via FRU (which implies the path literal should follow suit); scope: `crates/ modules/`; return: `LOCATIONS`
- Context cost: `M`
- Authoritative docs:
  - `docs/21_data_defaults_and_fixtures.md` - direct read (FRU/waiver rules)
- OrcaSlicer refs:
  - none
- Verification:
  - `cargo check --workspace --all-targets 2>&1 | tail -30` - FACT exit 0 (now includes tests/benches/examples)
  - `cargo clippy --workspace --all-targets -- -D warnings 2>&1 | tail -30` - FACT exit 0
- Exit condition: both gate commands exit 0; the full struct-literal blast radius is closed.

### Step 3: Map the field through the marshal and macro converters

- Task IDs: `TASK-337`
- Objective: Round-trip `tool_index` in `ir_to_wit_extrusion_path` / `convert_extrusion_path` (`crates/slicer-wasm-host/src/marshal/leaf.rs`) and the four `slicer-macros` path converters, so `None` flows through support/finalization unchanged and `Some(t)` survives to the commit boundary.
- Precondition: Steps 1-2 compiled the new field across all targets.
- Postcondition: `tool_index` is mapped both directions at every converter; the two `slicer-macros` WIT→IR sites and the two IR→WIT sites carry `tool_index: p.tool_index`.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-wasm-host/src/marshal/leaf.rs` - lines 219-243 and 418-443
  - `crates/slicer-macros/src/lib.rs` - lines 1285-1330, 2590-2605, 2735-2751
- Files allowed to edit (at most 3):
  - `crates/slicer-wasm-host/src/marshal/leaf.rs`
  - `crates/slicer-macros/src/lib.rs`
- Files explicitly out of bounds:
  - `target/`, `Cargo.lock`, generated code
- Blast-radius discipline: n/a (no new field here — the field already exists; this step only threads it).
- Expected sub-agent dispatches:
  - none (ranges already read)
- Context cost: `S`
- Authoritative docs:
  - none beyond the converter ranges
- OrcaSlicer refs:
  - none
- Verification:
  - `rg -n 'tool_index' crates/slicer-wasm-host/src/marshal/leaf.rs crates/slicer-macros/src/lib.rs` - FACT (4+ matches, both directions)
  - `cargo check --workspace --all-targets 2>&1 | tail -30` - FACT exit 0
- Exit condition: AC-7 satisfied; workspace still compiles.

### Step 4: tool-count host service + SDK wrapper + fill_authored_coloring key

- Task IDs: `TASK-337`
- Objective: Add `tool-count: func() -> u32` to `slicer:common/host-services` and the SDK wrapper; add the `fill_authored_coloring: Vec<String>` config key with the net-new `extract_string_list` extractor.
- Precondition: Steps 1-3 complete (WIT already valid; this adds the second WIT change in the same interface file).
- Postcondition: `common.wit` declares `tool-count`; `HostExecutionContext` carries an authoritative `tool_count` (from `ResolvedConfig.filament_density.len()`, `max(1, …)`); `impl hs::Host::tool_count` returns it; `slicer_sdk::host::tool_count()` exists with a wasm32 import arm; `ResolvedConfig` declares `fill_authored_coloring: Vec<String> = Vec::new()` via `extract_string_list`.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-schema/wit/deps/common.wit` - lines 44-163
  - `crates/slicer-wasm-host/src/host.rs` - lines 95-148 (`config_value_to_storage`), 1134-1340 (`HostExecutionContext`), 2425-2740 (`hs::Host` impl)
  - `crates/slicer-sdk/src/host.rs` - lines 32-70 (`__sdk_host_services_import` inline WIT), 1058-1078 (`now_us` pattern for the new wrapper)
  - `crates/slicer-ir/src/resolved_config.rs` - lines 455-521 (`extract_float_list`), 809-990 (`declare_resolved_config!`)
- Files allowed to edit (at most 3, extended by blast-radius discipline):
  - `crates/slicer-schema/wit/deps/common.wit`
  - `crates/slicer-wasm-host/src/host.rs`
  - `crates/slicer-sdk/src/host.rs`
  - `crates/slicer-ir/src/resolved_config.rs`
  - Justified exception (edit-cap): the fourth edit `crates/slicer-ir/src/resolved_config.rs` is part of the same atomic WIT→host→SDK→config change; splitting it leaves `cargo check` red between steps, so it rides the same blast-radius discipline as Steps 1/2.
- Files explicitly out of bounds:
  - `target/`, `Cargo.lock`, `**/wit-guest/**` generated code (regenerated in Step 6), `OrcaSlicerDocumented/`
- Blast-radius discipline: n/a (no struct/schema field here; `ResolvedConfig` is macro-declared and gains a field, which the `declare_resolved_config!` macro handles — but verify the hand-written `PartialEq`/`Hash` blocks at `resolved_config.rs:1003-1164` are updated by the macro or manually).
- Expected sub-agent dispatches:
  - Question: confirm whether `declare_resolved_config!` regenerates `PartialEq`/`Hash` for a new field, or whether the hand-written blocks must be edited; scope: `crates/slicer-ir/src/resolved_config.rs`; return: `FACT`
  - Question: confirm the exact `ResolvedConfig` field that carries the per-tool count is `filament_density` (no `tool_count`/`extruder_count` symbol exists); scope: `crates/slicer-ir/src/resolved_config.rs` + `crates/slicer-gcode/src`; return: `FACT`
- Context cost: `M`
- Authoritative docs:
  - `docs/specs/community-modules-dragon-curve-plan.md` - Central symbol contract (already read)
- OrcaSlicer refs:
  - none
- Verification:
  - `rg -n 'tool-count: func\(\) -> u32' crates/slicer-schema/wit/deps/common.wit` - FACT pass
  - `rg -n 'pub fn tool_count' crates/slicer-sdk/src/host.rs` - FACT pass
  - `rg -n 'fill_authored_coloring' crates/slicer-ir/src/resolved_config.rs && rg -n 'pub fn extract_string_list' crates/slicer-ir/src/resolved_config.rs` - FACT pass
  - `cargo check --workspace --all-targets 2>&1 | tail -30` - FACT exit 0
- Exit condition: AC-3, AC-4, AC-5 satisfied; workspace compiles.

### Step 5: Grant predicate + marshal-boundary enforcement + new contract test

- Task IDs: `TASK-337`
- Objective: Add the pure grant predicate, thread a grant/tool-count context into `convert_infill_output`, strip ungranted/out-of-range `Some(tool)` to the region tool (silent), honor granted valid `Some(t)` as an override, and prefer the validated per-path tool in `assemble_ordered_entities`'s infill arm.
- Precondition: Steps 1-4 complete (field, converters, tool-count, and config key all landed).
- Postcondition: `authored_coloring_granted(held_fill_claims, fill_authored_coloring, disclosed_authored) -> bool` exists (net-new, pub in `crates/slicer-wasm-host/src/marshal/out.rs`); `convert_infill_output` takes a grant context and strips/clamps per path; `assemble_ordered_entities` infill arm prefers `path.tool_index` over the spatial/variant default; the new `authored_coloring_grant_and_strip_tdd` contract test drives `convert_infill_output` and `authored_coloring_granted` directly.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-wasm-host/src/marshal/out.rs` - lines 51-153 (`convert_infill_output`)
  - `crates/slicer-wasm-host/src/dispatch.rs` - lines 2085-2203 (`resolve_region_tool_index` + call site), 2483-2535 (held-claims resolution), 3277-3293 (deconstruct call site)
  - `crates/slicer-wasm-host/src/marshal/native.rs` - lines 721-855 (`collect_infill` + `commit_native_layer_response`)
  - `crates/slicer-runtime/src/layer_executor.rs` - lines 1845-1892 (infill arm)
  - `crates/slicer-wasm-host/tests/contract/infill_holder_resolution_painted_region_tdd.rs` - lines 120-148 (`run_infill_stage` harness pattern)
- Files allowed to edit (at most 3, extended by blast-radius discipline):
  - `crates/slicer-wasm-host/src/marshal/out.rs`
  - `crates/slicer-wasm-host/src/dispatch.rs`
  - `crates/slicer-wasm-host/src/marshal/native.rs`
  - `crates/slicer-runtime/src/layer_executor.rs`
  - `crates/slicer-wasm-host/tests/contract/authored_coloring_grant_and_strip_tdd.rs` (new)
  - `crates/slicer-wasm-host/tests/contract/main.rs` (aggregator: add `mod authored_coloring_grant_and_strip_tdd;`)
  - Justified exception (edit-cap): the grant predicate threads a grant/tool-count context from the dispatch deconstruct (`dispatch.rs`) through `convert_infill_output` (`marshal/out.rs`) and the native commit path (`marshal/native.rs`) into the infill arm (`layer_executor.rs`); all four source edits are one atomic behavior change that cannot compile until the whole chain lands. The new test (`authored_coloring_grant_and_strip_tdd.rs`) plus its aggregator registration in `main.rs` must land in the same step or the contract test silently runs zero cases (S7 false-pass hazard), so they ride the same blast-radius discipline as Steps 1/2.
- Files explicitly out of bounds:
  - `target/`, `Cargo.lock`, `**/wit-guest/**` generated code
- Blast-radius discipline: n/a (no struct field; the new function is net-new, and the new test file must be aggregator-registered in the same step to avoid a silent 0-test false pass — S7).
- Expected sub-agent dispatches:
  - Question: confirm the exact `convert_infill_output` call sites (dispatch deconstruct + native commit) and their argument lists; scope: `crates/slicer-wasm-host/src/{dispatch.rs,marshal/native.rs}`; return: `LOCATIONS`
- Context cost: `M`
- Authoritative docs:
  - `docs/adr/0058-authored-coloring-per-path-tool-carrier.md` - direct read (Consequences)
  - `docs/specs/community-modules-dragon-curve-infill.md` §2 - direct read (grant intersection + enforcement)
- OrcaSlicer refs:
  - none
- Verification:
  - `rg -n 'pub fn authored_coloring_granted' crates/slicer-wasm-host/src/marshal/out.rs` - FACT pass
  - `cargo test -p slicer-wasm-host --test contract authored_coloring_grant_and_strip_tdd 2>&1 | tail -25` - FACT exit 0
  - `cargo check --workspace --all-targets 2>&1 | tail -30` - FACT exit 0
- Exit condition: AC-N1, AC-N2, AC-N4 satisfied; the new test binary reports non-zero tests run.

### Step 6: Infill-linker tool-equality guard + test

- Task IDs: `TASK-337`
- Objective: Add tool equality to `paths_compatible` and `compatible_paths`, make `chain_or_connect_infill` refuse/split across differing per-path tools, and pin it with `cross_tool_paths_not_chained`.
- Precondition: Steps 1-5 complete (the field carries real values by this point).
- Postcondition: both predicates require `first.tool_index == second.tool_index`; the linker never chains paths of differing per-path tool; `cross_tool_paths_not_chained` is green.
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/infill-linker/src/orchestrate.rs` - lines 358-395 (`compatible_regions`, `paths_compatible`)
  - `modules/core-modules/infill-linker/src/connect.rs` - lines 290-349 (`chain_or_connect_infill`), 488-517 (`compatible_paths`, `endpoint_widths_compatible`)
  - `modules/core-modules/infill-linker/tests/connect_tdd.rs` - lines 24-70 (`path()` fixture + `raw_paths()`)
- Files allowed to edit (at most 3):
  - `modules/core-modules/infill-linker/src/orchestrate.rs`
  - `modules/core-modules/infill-linker/src/connect.rs`
  - `modules/core-modules/infill-linker/tests/connect_tdd.rs`
- Files explicitly out of bounds:
  - `target/`, `Cargo.lock`, `**/wit-guest/**` generated code
- Blast-radius discipline: n/a.
- Expected sub-agent dispatches:
  - none (ranges already read)
- Context cost: `S`
- Authoritative docs:
  - `docs/adr/0058-authored-coloring-per-path-tool-carrier.md` - Consequences (linker guard + wipe-tower cost note)
- OrcaSlicer refs:
  - none
- Verification:
  - `rg -n 'tool_index' modules/core-modules/infill-linker/src/orchestrate.rs modules/core-modules/infill-linker/src/connect.rs` - FACT pass
  - `cargo test -p infill-linker --test connect_tdd cross_tool_paths_not_chained 2>&1 | tail -25` - FACT exit 0
  - `cargo clippy --workspace --all-targets -- -D warnings 2>&1 | tail -30` - FACT exit 0
- Exit condition: AC-8 and AC-N3 satisfied.

### Step 7: Guest rebuild + docs + DEV-135 deviation row

- Task IDs: `TASK-337`
- Objective: Rebuild every guest for the WIT change (mandatory staleness closure), update `docs/02_ir_schemas.md` and `docs/03_wit_and_manifest.md`, and add the net-new `DEV-135` row to `docs/DEVIATION_LOG.md`.
- Precondition: Steps 1-6 complete.
- Postcondition: `cargo xtask build-guests --check` fails (stale) then `cargo xtask build-guests` succeeds then `--check` is green; the two docs carry the new field/claim/host-service; `DEV-135` row exists in the log.
- Files allowed to read, with ranges when over 300 lines:
  - `docs/02_ir_schemas.md` - lines 671-720 (IR 7 PerimeterIR / ExtrusionPath3D), 781-799 (IR 8 InfillIR), 1020-1140 (IR 10 LayerCollectionIR + schema-version note)
  - `docs/03_wit_and_manifest.md` - lines 308-320 (host-api.wit function list), 661-694 (Known claim IDs)
  - `docs/DEVIATION_LOG.md` - delegated read of the last two `DEV-*` rows only (never the full file)
- Files allowed to edit (at most 3):
  - `docs/02_ir_schemas.md`
  - `docs/03_wit_and_manifest.md`
  - `docs/DEVIATION_LOG.md`
- Files explicitly out of bounds:
  - `target/`, `Cargo.lock`, generated code, `docs/07_implementation_status.md` (updated via worker dispatch at completion, not hand-edited here)
- Blast-radius discipline: n/a.
- Expected sub-agent dispatches:
  - Question: run `cargo xtask build-guests --check` then `cargo xtask build-guests` then `cargo xtask build-guests --check`; return the tail of each; scope: workspace; return: `SNIPPETS` (≤20 lines each)
  - Question: confirm `DEV-134` is the highest `DEV-NNN` in `docs/DEVIATION_LOG.md` and sample the `DEV-NNN` row format; scope: `docs/DEVIATION_LOG.md`; return: `FACT` + `SNIPPETS`
- Context cost: `M`
- Authoritative docs:
  - `docs/adr/0058-authored-coloring-per-path-tool-carrier.md` - Consequences (wipe-tower cost note for the deviation row)
- OrcaSlicer refs:
  - none
- Verification:
  - `cargo xtask build-guests 2>&1 | tail -20 && cargo xtask build-guests --check 2>&1 | tail -20` - FACT exit 0
  - `rg -q 'tool_index: Option<u32>' docs/02_ir_schemas.md && rg -q 'tool_index' docs/02_ir_schemas.md` - FACT pass
  - `rg -q 'tool-count' docs/03_wit_and_manifest.md && rg -q 'claim:authored-coloring' docs/03_wit_and_manifest.md` - FACT pass
  - `rg -n '^\| DEV-135 \|' docs/DEVIATION_LOG.md` - FACT pass
  - `cargo test -p slicer-runtime --test contract wit_drift_detection_tdd 2>&1 | tail -20` - FACT exit 0
- Exit condition: AC-9 satisfied; guest staleness closed; docs greps green; WIT drift gate green.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | M | WIT+IR+const bump + production literal blast radius |
| Step 2 | M | test/fixture literal blast radius (mechanical) |
| Step 3 | S | converter threading |
| Step 4 | M | tool-count + config key (WIT #2) |
| Step 5 | M | grant predicate + enforcement + new test |
| Step 6 | S | linker guard + test |
| Step 7 | M | guest rebuild + docs + deviation |

Split before activation if aggregate cost exceeds M or any step is L. Aggregate = M; Steps 1/2 are the only large ones and each is mechanical and grep-bounded.

## Packet Completion Gate

- All steps and exits complete.
- Every pipe-suffixed AC command returns PASS.
- Update `docs/07_implementation_status.md` through a worker dispatch, never a full backlog read.
- Reconcile reopened/superseded status transitions.
- `packet.spec.md` is ready for `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed AC and packet-level gate command.
- Record remaining packet-local risk.
- Confirm context stayed at or below 150k standard, or at/below 300k only with a logged swarm ESCALATION; otherwise record a packet-authoring lesson.

All `cargo check`, `cargo clippy`, and `cargo test` invocations in gate and verification commands must use `--all-targets` so the test, bench, and example targets compile.
