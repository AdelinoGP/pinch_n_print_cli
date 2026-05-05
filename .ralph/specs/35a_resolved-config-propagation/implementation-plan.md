# Implementation Plan: 35a_resolved-config-propagation

## Execution Rules

- One atomic step at a time.
- Each step must map back to the packet's grouped task IDs (`TASK-166`).
- TDD first, then implementation, then the narrowest falsifying validation.
- Each step honors the context-discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. The fields below are not optional metadata — they are the budget contract for this step.

## Steps

### Step 1: Pre-flight — confirm scope is still accurate

- Task IDs:
  - `TASK-166`
- Objective: Confirm the producer-side default at `dispatch.rs::harvest_layer_plan_ir` still hardcodes `ResolvedConfig::default()` and that DEV-040 is still Open before doing any work. If either has changed, surface it and stop.
- Precondition: clean checkout, no in-flight edits in the slicer-host crate.
- Postcondition: a single FACT line confirming the bug surface is unchanged and DEV-040 row is `Open`.
- Files allowed to read (with line-range hints when > 300 lines):
  - `crates/slicer-host/src/dispatch.rs` lines `1640-1660`
  - `docs/DEVIATION_LOG.md` (delegate; row DEV-040 only)
- Files allowed to edit (≤ 3): none (read-only step).
- Files explicitly out-of-bounds for this step: everything else under `crates/slicer-host/src/`.
- Expected sub-agent dispatches:
  - "Read `crates/slicer-host/src/dispatch.rs:1640-1660`; return FACT (the literal value of the `resolved_config:` field)."
  - "Read the DEV-040 row in `docs/DEVIATION_LOG.md`; return FACT of its Status cell."
- Context cost: `S`
- Authoritative docs:
  - `docs/DEVIATION_LOG.md` — single-row read.
- OrcaSlicer refs: none.
- Verification:
  - Both sub-agents return the expected FACT (`ResolvedConfig::default()` and `Open` respectively). No `cargo` run.
- Exit condition: confirmed the packet is still applicable; if either FACT diverges, halt and update the packet before continuing.

### Step 2: TDD — author the resolver and unit tests

- Task IDs:
  - `TASK-166`
- Objective: Land `crates/slicer-host/src/config_resolution.rs` with `resolve_global_config`, `resolve_per_object_configs`, and `ConfigResolutionError`, plus `crates/slicer-host/tests/config_resolution_tdd.rs` covering all positive and negative AC unit cases.
- Precondition: Step 1 confirmed the bug surface is still present.
- Postcondition: the resolver compiles and all 4 resolver tests in `config_resolution_tdd.rs` pass:
  - `resolver_maps_top_shell_layers`
  - `resolver_unknown_key_routes_to_extensions`
  - `resolver_per_object_overrides_global`
  - `resolver_rejects_string_for_top_shell_layers`
- Files allowed to read:
  - `crates/slicer-ir/src/slice_ir.rs` lines `570-660` (ResolvedConfig fields, types, defaults).
  - `crates/slicer-host/src/execution_plan.rs` lines `45-211` (parse_cli_config_source semantics — read-only confirmation).
- Files allowed to edit (≤ 3):
  - `crates/slicer-host/src/config_resolution.rs` (NEW)
  - `crates/slicer-host/src/lib.rs` (add `pub mod config_resolution;` and re-exports)
  - `crates/slicer-host/tests/config_resolution_tdd.rs` (NEW)
- Files explicitly out-of-bounds for this step:
  - `crates/slicer-host/src/region_mapping.rs` (next step).
  - `crates/slicer-host/src/main.rs` (Step 5).
  - `crates/slicer-host/src/wit_host.rs`, `dispatch.rs` (everywhere except the lines noted in Step 1).
- Expected sub-agent dispatches:
  - "Run `cargo test -p slicer-host --test config_resolution_tdd`; return FACT pass/fail. On fail, return SNIPPETS of failing assertions + ≤ 20 lines."
  - "Run `cargo build -p slicer-host`; return FACT pass/fail." (cheap pre-test sanity.)
- Context cost: `M`
- Authoritative docs:
  - `docs/02_ir_schemas.md` lines `~575-660` (ResolvedConfig surface).
- OrcaSlicer refs: none.
- Verification:
  - `cargo test -p slicer-host --test config_resolution_tdd -- --nocapture`
- Exit condition: 4 named tests pass; resolver compiles cleanly.

### Step 3: Extend `commit_region_mapping_builtin` to stamp resolved configs

- Task IDs:
  - `TASK-166`
- Objective: Update `region_mapping::commit_region_mapping_builtin` to accept `&BTreeMap<String, ResolvedConfig>` (per-object map) and `&ResolvedConfig` (default fallback), and stamp each `RegionPlan.config` from the entry's `object_id`. Existing internal helpers (`execute_region_mapping`, `execute_region_mapping_with_cap`) are unchanged.
- Precondition: Step 2 complete (resolver public surface available).
- Postcondition: `commit_region_mapping_builtin` compiles with the new signature; all existing tests that call it still link (with mechanical updates if needed).
- Files allowed to read:
  - `crates/slicer-host/src/region_mapping.rs` (full file, ~260 lines).
  - `crates/slicer-host/src/blackboard.rs` (only the `region_map` / `commit_region_map` accessor surface; ranged read).
- Files allowed to edit (≤ 3):
  - `crates/slicer-host/src/region_mapping.rs`
- Files explicitly out-of-bounds for this step:
  - All callers — those land in Step 4 / Step 5.
- Expected sub-agent dispatches:
  - "Find every caller of `commit_region_mapping_builtin`; return LOCATIONS." — purpose: confirm only `prepass.rs:313,340` and any test fixtures need touching.
  - "Run `cargo build -p slicer-host`; return FACT pass/fail. On fail, return SNIPPETS of compile error." — purpose: confirm the new signature compiles before touching callers.
- Context cost: `S`
- Authoritative docs:
  - `docs/04_host_scheduler.md` §"RegionMapIR Compilation" (delegate SUMMARY).
- OrcaSlicer refs: none.
- Verification:
  - `cargo build -p slicer-host` — compile-only sanity.
- Exit condition: `region_mapping.rs` compiles standalone; sub-agent returned a LOCATIONS list with exactly the call sites we expect.

### Step 4: Update prepass call sites + RegionMapping integration test

- Task IDs:
  - `TASK-166`
- Objective: Forward the new arguments at `prepass.rs:313,340` (mechanical), then land `crates/slicer-host/tests/region_mapping_resolved_config_tdd.rs` with `commit_stamps_per_object_resolved_config` covering AC-4.
- Precondition: Step 3 complete.
- Postcondition: `prepass.rs` builds; the new RegionMapping test passes.
- Files allowed to read:
  - `crates/slicer-host/src/prepass.rs` lines `300-360` only.
  - Any small fixture builder for `LayerPlanIR` / `ExecutionPlan` already used by `region_mapping_tdd` siblings (locate via Glob, do not load other test files in full).
- Files allowed to edit (≤ 3):
  - `crates/slicer-host/src/prepass.rs`
  - `crates/slicer-host/tests/region_mapping_resolved_config_tdd.rs` (NEW)
- Files explicitly out-of-bounds for this step:
  - `crates/slicer-host/src/main.rs` (Step 5).
  - `crates/slicer-host/src/pipeline.rs` (Step 5).
- Expected sub-agent dispatches:
  - "Run `cargo test -p slicer-host --test region_mapping_resolved_config_tdd`; return FACT pass/fail. On fail, return SNIPPETS."
  - "Glob `crates/slicer-host/tests/region_mapping*tdd*.rs`; return LOCATIONS." — purpose: confirm test-file naming consistency and locate any helper modules.
- Context cost: `S`
- Authoritative docs: none new beyond Step 3.
- OrcaSlicer refs: none.
- Verification:
  - `cargo test -p slicer-host --test region_mapping_resolved_config_tdd -- --nocapture`
- Exit condition: the new test passes; `prepass.rs` callers build; no other RegionMapping tests regress.

### Step 5: Plumb resolved configs through `PipelineConfig` and `main.rs`

- Task IDs:
  - `TASK-166`
- Objective: Add `resolved_configs: Arc<BTreeMap<String, ResolvedConfig>>` and `default_resolved_config: Arc<ResolvedConfig>` fields to `pipeline::PipelineConfig`. In `main.rs`, after `parse_cli_config_source` succeeds, call `resolve_per_object_configs` with the parsed source and the list of `ObjectMesh.id` strings. On `ConfigResolutionError`, exit non-zero with a structured message containing the offending key and expected variant.
- Precondition: Steps 2-4 complete.
- Postcondition: `cargo build -p slicer-host --bin slicer-host` succeeds; the binary's behavior on a successful `--config` is unchanged for unrelated keys; on a malformed declared field the binary exits non-zero with the expected stderr fragment.
- Files allowed to read:
  - `crates/slicer-host/src/pipeline.rs` lines `30-150` only.
  - `crates/slicer-host/src/main.rs` lines `100-260` only.
- Files allowed to edit (≤ 3):
  - `crates/slicer-host/src/pipeline.rs`
  - `crates/slicer-host/src/prepass.rs` (forward new fields from PipelineConfig — counts as continued mechanical wiring from Step 4)
  - `crates/slicer-host/src/main.rs`
- Files explicitly out-of-bounds for this step:
  - `crates/slicer-host/src/wit_host.rs`, `dispatch.rs`, `layer_executor.rs`, `gcode_emit.rs`. None require changes; do not open.
- Expected sub-agent dispatches:
  - "Run `cargo build -p slicer-host --bin slicer-host`; return FACT pass/fail. On fail, return SNIPPETS of compile error."
  - "Run `cargo test -p slicer-host --tests` (no --test filter); return FACT pass/fail. On fail, return only the failing test names + ≤ 20 lines per failure." — purpose: confirm no existing test regressed.
- Context cost: `M`
- Authoritative docs: none new.
- OrcaSlicer refs: none.
- Verification:
  - `cargo build -p slicer-host --bin slicer-host`
  - `cargo test -p slicer-host --tests` (gate; expect green except for tests added by Step 6 which haven't landed yet — those are Step 6's gate).
- Exit condition: binary builds; pre-existing tests still pass; the prepass path receives the resolved-configs map without panicking on the existing regression tests.

### Step 6: Binary E2E tests — propagation + CLI rejection

- Task IDs:
  - `TASK-166`
- Objective: Append two tests to `crates/slicer-host/tests/benchy_end_to_end_tdd.rs`:
  - `benchy_user_top_shell_layers_propagates_through_binary` — runs the Benchy binary twice (`top_shell_layers = 1` and `top_shell_layers = 4`), asserts strict inequality of `;TYPE:Top surface` block counts and `;TYPE:Bottom surface` block counts.
  - `cli_rejects_top_shell_layers_string` — invokes the binary with `{"top_shell_layers": "four"}`, asserts non-zero exit, no `--output` file written, stderr contains both `top_shell_layers` and `expected Int`.
- Precondition: Step 5 complete.
- Postcondition: both new tests pass; `benchy_multi_layer_top_bottom_evidence` (packet 35's existing AC) still passes.
- Files allowed to read:
  - `crates/slicer-host/tests/benchy_end_to_end_tdd.rs` lines `1280-1370` (firmware-retraction E2E pattern) and lines `1599-1840` (multi-layer evidence test).
- Files allowed to edit (≤ 3):
  - `crates/slicer-host/tests/benchy_end_to_end_tdd.rs`
- Files explicitly out-of-bounds for this step:
  - The rest of `benchy_end_to_end_tdd.rs` (file is > 1800 lines; do not load in full).
  - All non-test source under `crates/slicer-host/src/`.
- Expected sub-agent dispatches:
  - "Run `cargo test -p slicer-host --test benchy_end_to_end_tdd benchy_user_top_shell_layers_propagates_through_binary cli_rejects_top_shell_layers_string -- --nocapture`; return FACT pass/fail. On fail, return SNIPPETS of failing assertion + ≤ 20 lines of stderr (NOT the produced G-code)."
  - "Run `cargo test -p slicer-host --test benchy_end_to_end_tdd benchy_multi_layer_top_bottom_evidence -- --nocapture`; return FACT pass/fail." — purpose: regression guard against packet 35.
- Context cost: `M`
- Authoritative docs: none new.
- OrcaSlicer refs: none.
- Verification:
  - `cargo test -p slicer-host --test benchy_end_to_end_tdd benchy_user_top_shell_layers_propagates_through_binary cli_rejects_top_shell_layers_string -- --nocapture`
- Exit condition: both new tests pass; `benchy_multi_layer_top_bottom_evidence` still passes.

### Step 7: Workspace gates and clippy

- Task IDs:
  - `TASK-166`
- Objective: Run the workspace-wide build, test, and clippy gates.
- Precondition: Step 6 complete.
- Postcondition: `cargo build --workspace`, `cargo test --workspace`, and `cargo clippy --workspace -- -D warnings` are all green (or any pre-existing failures are noted as not introduced by this packet).
- Files allowed to read: none directly — all dispatched.
- Files allowed to edit (≤ 3): only fixups for clippy or compilation issues exposed by the gate, scoped to files already in this packet's edit list.
- Files explicitly out-of-bounds for this step: anything outside the packet's primary or mechanical edit list.
- Expected sub-agent dispatches:
  - "Run `cargo build --workspace`; return FACT pass/fail. On fail, return SNIPPETS of compile error."
  - "Run `cargo test --workspace`; return FACT pass/fail. On fail, return only the failing test names + which crate they belong to + ≤ 20 lines per failure."
  - "Run `cargo clippy --workspace -- -D warnings`; return FACT pass/fail. On fail, return SNIPPETS of the warnings, capped at 5."
- Context cost: `S`
- Authoritative docs: none.
- OrcaSlicer refs: none.
- Verification:
  - All three workspace gates green.
- Exit condition: zero new failures introduced by this packet. Pre-existing failures (e.g. the tree-support IR-access test failure noted in `docs/07` packet 35 closure) must be explicitly reconciled — either confirmed unrelated or fixed here.

### Step 8: Update DEV-040 and TASK-166 status

- Task IDs:
  - `TASK-166`
- Objective: Flip DEV-040 in `docs/DEVIATION_LOG.md` to `Closed` with a one-line rationale citing this packet, and mark `TASK-166` as `[x]` in `docs/07_implementation_status.md` Workstream 2.
- Precondition: Step 7 green.
- Postcondition: both docs updated; `packet.spec.md` ready to flip to `status: implemented`.
- Files allowed to read: full file context only via delegated fact-reads — never load these in the implementer's own context.
- Files allowed to edit (≤ 3):
  - `docs/DEVIATION_LOG.md` (single-row edit)
  - `docs/07_implementation_status.md` (single-line edit on the TASK-166 row)
  - `.ralph/specs/35a_resolved-config-propagation/packet.spec.md` (status: draft → implemented)
- Files explicitly out-of-bounds for this step: every other doc.
- Expected sub-agent dispatches:
  - "Update DEV-040 row in `docs/DEVIATION_LOG.md` to `Closed` with rationale `Closed YYYY-MM-DD by packet 35a; resolve_global_config + per-object overlay stamps RegionPlan.config end-to-end via PrePass::RegionMapping`; return FACT confirming the row is updated and no other rows were touched."
  - "Mark TASK-166 in `docs/07_implementation_status.md` as `[x]` with closure note; return FACT confirming only that line changed."
- Context cost: `S`
- Authoritative docs: none new.
- OrcaSlicer refs: none.
- Verification:
  - `git diff --stat docs/DEVIATION_LOG.md docs/07_implementation_status.md` — exactly two files changed; small line counts.
- Exit condition: both docs reflect the closure; packet ready for the Acceptance Ceremony.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | S | Pure-dispatch precheck. |
| Step 2 | M | Resolver + 4 unit tests in one file pair. |
| Step 3 | S | Single function signature change in a short file. |
| Step 4 | S | Mechanical wiring + one new integration test. |
| Step 5 | M | PipelineConfig field + main.rs threading + workspace test gate. |
| Step 6 | M | Two new binary E2E tests. |
| Step 7 | S | Workspace gates, fully delegated. |
| Step 8 | S | Two single-line doc edits, fully delegated. |

Aggregate sum: **M** (no step is L). Largest single step: **M** (Steps 2, 5, 6 are all sized M but bounded by the file-edit and test-count caps above).

## Packet Completion Gate

- All 8 steps complete.
- Every step exit condition met.
- All 5 packet acceptance criteria green (each `cargo test ... -- --exact --nocapture` returned PASS via sub-agent).
- Both negative test cases green.
- `docs/07_implementation_status.md` marks TASK-166 as `[x]` (via worker dispatch — never edited by loading the full backlog into the implementer's context).
- `docs/DEVIATION_LOG.md` DEV-040 row marked `Closed`.
- `packet.spec.md` ready to flip to `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed acceptance criterion command from `packet.spec.md`:
  - `cargo test -p slicer-host --test config_resolution_tdd resolver_maps_top_shell_layers -- --exact --nocapture`
  - `cargo test -p slicer-host --test config_resolution_tdd resolver_unknown_key_routes_to_extensions -- --exact --nocapture`
  - `cargo test -p slicer-host --test config_resolution_tdd resolver_per_object_overrides_global -- --exact --nocapture`
  - `cargo test -p slicer-host --test region_mapping_resolved_config_tdd commit_stamps_per_object_resolved_config -- --exact --nocapture`
  - `cargo test -p slicer-host --test benchy_end_to_end_tdd benchy_user_top_shell_layers_propagates_through_binary -- --exact --nocapture`
  - `cargo test -p slicer-host --test config_resolution_tdd resolver_rejects_string_for_top_shell_layers -- --exact --nocapture`
  - `cargo test -p slicer-host --test benchy_end_to_end_tdd cli_rejects_top_shell_layers_string -- --exact --nocapture`
- Confirm packet-level verification commands (`cargo build --workspace`, `cargo clippy --workspace -- -D warnings`) green.
- Record any remaining packet-local risk explicitly before moving to `status: implemented`.
- Confirm the implementer's peak context usage stayed under 70%; if not, log it as a packet-authoring lesson for future spec-packet-generator runs.
