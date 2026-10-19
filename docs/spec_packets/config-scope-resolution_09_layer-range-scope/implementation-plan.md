# Implementation Plan: layer-range-scope

## Execution Rules

- Work one atomic step at a time; every step maps to `TASK-570`.
- Use TDD, then implementation, then the narrowest falsifying validation.
- Reconcile packets 03, 05, and 07 before editing; do not guess around a forward-interface mismatch. (Step 1 is done; see `design.md` §Landing Notes.)
- Every cargo/xtask invocation is delegated and tee'd to `target/test-output.log` where required by repository policy.
- Owner-approved Amendment 1 expands this packet to own the `slicer:prepass-layer-planning@3.0.0` WIT bump, the guest variable-schedule algorithm, the host profile API, per-layer region-config application, and the manifest `scheduled_layer_zs` field.

## Steps

### Step 1: Reconcile forward exports and freeze blast-radius inventories (DONE)

- Task IDs: `TASK-570`
- Objective: confirm the landed packet-03/05/07 symbols and inventory every exhaustive `ConfigScope`/`ResolutionError` match, `ResolutionTarget` literal, `SliceRunOptions` literal, `prepare_prepass_context` call, and model-source adapter.
- Postcondition: bounded inventories name every required edit owner; `design.md` §Landing Notes records the reconciled reality (scalar-only `query_z_grid`, guest schedule producer, host ingestion seam, missing pnp-cli `slicer-config` dependency, single-range config carriers).
- Verification: completed by delegated `LOCATIONS`/`FACT` returns recorded in the Step 1 dispatch log.
- Exit condition: satisfied.

### Step 2a: Author the canonical single-range fixture

- Task IDs: `TASK-570`
- Objective: create a deterministic one-object 3MF whose range member has exactly AC-1's canonical XML.
- Precondition: delegated canonical XML/object-linkage evidence confirms the shape (already obtained).
- Postcondition: the committed archive has the required ordinary model members and exactly one range member/object/range/option; its model XML object id differs from the 1-based ordinal.
- Files allowed to edit (at most 3): `resources/layer_range_one_range.3mf`
- Files explicitly out of bounds: existing binary fixtures, production Rust, source plan, predecessor packets.
- Expected sub-agent dispatches: build the fixture from reviewed member text, then inspect only member names and bounded XML; `FACT: ≤5 lines`.
- Context cost: `S`
- Verification: bounded fixture FACT reports `Metadata/layer_config_ranges.xml`, one object ordinal, `[0.4,0.8)`, and `layer_height` text `0.1`.
- Exit condition: archive member shape differs, construction is nondeterministic, or an existing fixture was modified.

### Step 2b: Implement the model-IO parser and ordinal mapper

- Task IDs: `TASK-570`
- Objective: parse the exact optional XML part into raw ordinal/range/string records with atomic structural errors (`read_3mf_layer_config_ranges`, `RawLayerConfigRange`, `LayerRangeParseError`), and map ordinals to loaded object ids (`map_layer_config_ranges` → `LayerRangeInput`).
- Precondition: Step 2a fixture exists.
- Postcondition: AC-1 and AC-N3/AC-N4 pass; missing part is empty success; invalid/malformed XML never yields partial records.
- Files allowed to edit (at most 3):
  - `crates/slicer-model-io/src/layer_config_ranges.rs`
  - `crates/slicer-model-io/src/lib.rs`
  - `crates/slicer-model-io/tests/layer_config_ranges_tdd.rs`
- Files explicitly out of bounds: loader geometry parsing, slicer-config/runtime, existing binary fixtures, source plan.
- Expected sub-agent dispatches: implement + run focused test; `FACT` or `SNIPPETS ≤20 lines`.
- Context cost: `M`
- Verification:
  - `bash -lc 'set -euo pipefail; mkdir -p target; cargo test -p slicer-model-io --all-targets --test layer_config_ranges_tdd 2>&1 | tee target/test-output.log >/dev/null; rg -q "test result: ok" target/test-output.log'` — FACT.
- Exit condition: exact AC-1 record/ordinal mapping passes, each invalid class returns a named error with no partial output, and missing member returns an empty vector.

### Step 3: Add and validate the typed layer-range scope

- Task IDs: `TASK-570`
- Objective: extend packet-03 ingestion with `ConfigScope::LayerRange`, `LayerConfigRange`, `LayerRangeLoadError`, `scoped.layer_ranges()`, `ConfigIngestor::ingest_layer_ranges`, deterministic indices, admission denial, and non-height conflict detection.
- Precondition: Step 1 inventories complete; Step 2 produces typed inputs.
- Postcondition: AC-2 and AC-N1/AC-N2 pass; all ranges validated before immutable `ScopedConfig` exposure; `scope_denial_label` returns `"layer_range"`.
- Files allowed to edit (at most 3):
  - `crates/slicer-config/src/ingestion.rs`
  - `crates/slicer-config/src/lib.rs`
  - `crates/slicer-config/tests/layer_range_scope_tdd.rs`
- Files explicitly out of bounds: resolver/profile code, runtime, model-IO parser, manifests, IR/WIT.
- Blast-radius discipline: `ScopedConfig` gains a `layer_ranges` field; its exhaustive single-field literals (see design §Landing Notes) are updated or converted with justified FRU in this step or an adjacent ≤3-file substep.
- Context cost: `M`
- Verification:
  - AC-2 command, AC-N1 command, AC-N2 command — FACT.
- Exit condition: values are registry-typed, bounds/ordinal mapping valid, denial/conflict failures atomic, and no hard-coded denial roster exists.

### Step 4: Compose canonical Z profiles and fill both resolver seams

- Task IDs: `TASK-570`
- Objective: implement `query_layer_height_profile`, `layer_top_zs`, and `ResolutionTarget.layer_top_z`; compose earlier-starting `layer_height` trim/gap segments; insert matching range deltas between object and modifier in `resolve_scope_stack`.
- Precondition: Step 3 yields normalized validated typed ranges.
- Postcondition: AC-3 and AC-4 pass with independently authored literal segments/values.
- Files allowed to edit (at most 3):
  - `crates/slicer-config/src/resolution.rs`
  - `crates/slicer-config/src/lib.rs`
  - `crates/slicer-config/tests/layer_range_scope_tdd.rs`
- Blast-radius discipline: the four exhaustive `ResolutionTarget` literals (`crates/slicer-runtime/src/run.rs`, `crates/slicer-config/tests/scope_resolution_tdd.rs` ×3) are updated with the new field in this step; all other literals use FRU.
- Context cost: `M`
- Verification: AC-3 and AC-4 commands — FACT.
- Exit condition: earlier-starting trimming holds, gaps use base height, half-open bounds exclude `max_z`, precedence is exactly object < range < modifier, and both resolver queries consume one typed range set.

### Step 5a: Bump the WIT package and reconcile host/macro/doc surfaces

- Task IDs: `TASK-570`
- Objective: bump `slicer:prepass-layer-planning` to `@3.0.0`, add `layer-zs: list<f64>` to `object-layer-config`, and update every host surface plus `LayerPlanningObject`, the dispatch adapter, the macros glue, and the version-citing tests/docs.
- Precondition: Step 4 green; `cargo xtask build-guests --check` run before interpreting guest failures.
- Postcondition: host compiles against the new WIT; `crates/slicer-wasm-host` round-trips `layer_zs` through the record (AC-7 half).
- Files allowed to edit (at most 3 per substep; split further if needed):
  - `crates/slicer-schema/wit/deps/prepass-layer-planning/prepass-layer-planning.wit`
  - `crates/slicer-schema/src/lib.rs` (StageSpec literal)
  - `crates/slicer-sdk/src/{traits.rs,prepass_types.rs,native.rs}`
  - `crates/slicer-wasm-host/src/dispatch.rs`
  - `crates/slicer-macros/src/lib.rs` and the `@2.0.0` literals in `crates/slicer-macros/tests/binding_surface_tdd.rs`
  - `crates/slicer-runtime/src/run.rs` (mapping only)
  - `crates/slicer-wasm-host/tests/contract/prepass_layer_planning_v3_tdd.rs` (new focused contract test) and any renamed `prepass_layer_planning_v2_tdd`
  - `crates/slicer-schema/wit/README.md`, `docs/03_wit_and_manifest.md`
- Files explicitly out of bounds: guest module source (Step 5b), resolver semantics, unrelated WIT packages.
- Context cost: `M`
- Verification: `cargo check --workspace --all-targets` — FACT; `cargo xtask build-guests --check` exit code — FACT.
- Exit condition: every version-citing surface says `@3.0.0`, host adapters carry `layer_zs`, and no stale `@2.0.0` literal remains except in intentional history.

### Step 5b: Implement the guest variable-schedule algorithm

- Task IDs: `TASK-570`
- Objective: make `layer-planner-default` emit explicit top-Z schedules when `layer-zs` is non-empty: raft-offset them, merge with variable-step participation and catch-up, preserve uniform fallback when empty, and keep `merge_same_height`/`merge_raft_sequences` behavior for uniform plans.
- Precondition: Step 5a WIT landed; guest freshness check run.
- Postcondition: AC-7's guest half passes with literal expected Z lists.
- Files allowed to edit (at most 3):
  - `modules/core-modules/layer-planner-default/src/lib.rs`
  - `modules/core-modules/layer-planner-default/tests/layer_planning_tdd.rs`
  - `modules/core-modules/layer-planner-default/tests/slicer_module_binding_tdd.rs` (version literal only)
- Files explicitly out of bounds: other guests, host runtime, resolver code.
- Context cost: `M`
- Verification:
  - `bash -lc 'set -euo pipefail; mkdir -p target; cargo test --manifest-path modules/core-modules/layer-planner-default/Cargo.toml --test layer_planning_tdd 2>&1 | tee target/test-output.log >/dev/null; rg -q "test result: ok" target/test-output.log'` — FACT.
  - `cargo xtask build-guests --force` then re-run — FACT.
- Exit condition: explicit schedules emitted verbatim, uniform fallback intact, no non-terminating loop for finite validated inputs, and guest binding test on `@3.0.0`.

### Step 6: Carry one typed range set through both production paths and apply per layer

- Task IDs: `TASK-570`
- Objective: add the typed carrier (`SliceRunOptions.layer_ranges`, `prepare_prepass_context`/`load_live_modules_for_plan_manifest_first` parameters, `PipelineConfig`/prepass authority), ingest via `ConfigIngestor::ingest_layer_ranges`, and re-resolve region config per layer in the region-mapping builtin when a range covers `layer.z`.
- Precondition: Steps 4–5b green; Step 1 caller inventories complete.
- Postcondition: AC-5 passes; both runtime paths receive value-equivalent typed ranges from their adapters; no XML parse in runtime.
- Files allowed to edit (at most 3 per substep):
  - `crates/slicer-runtime/src/{run.rs,prepass.rs,pipeline.rs,builtins/region_mapping_producer.rs}`
  - `crates/slicer-wasm-host/src/execution_plan_live.rs`
  - `crates/pnp-cli/src/{main.rs,visual_debug.rs,support_preview.rs}`, `crates/pnp-cli/Cargo.toml`
  - `crates/slicer-core/src/algos/region_mapping.rs`
  - exhaustive `SliceRunOptions`/`prepare_prepass_context` call sites named by the Step 1 inventory (≤3 per substep)
- Files explicitly out of bounds: resolver semantics, mesh IR fields, CLI argument surface, support-preview behavior changes beyond the new parameter.
- Blast-radius discipline: update the manual `Default` impl and the exhaustive literals; FRU call sites absorb the new field; never let a production path silently drop ranges.
- Context cost: `M`
- Verification: `cargo check -p pnp-cli -p slicer-runtime --all-targets` — FACT; AC-5 command — FACT.
- Exit condition: both adapters parse once, runtime carries typed data, all callers compile, per-layer range config proven at the range-covering layer, and no production path substitutes an empty range set for the fixture.

### Step 7: Expose the scheduled Z sequence in the visual-debug manifest

- Task IDs: `TASK-570`
- Objective: add additive `scheduled_layer_zs: Option<Vec<f64>>` to `Manifest` (schema 1.3), populated from the resolved `LayerPlanIR` schedule in `run_model_source`, and document it.
- Precondition: Step 6 green; `manifest.json` writer symbol identified.
- Postcondition: AC-6 observable from a real bundle; older schema versions unaffected.
- Files allowed to edit (at most 3):
  - `crates/pnp-cli/src/visual_debug.rs`
  - `docs/19_visual_debug.md`
  - one existing manifest fixture/test that asserts the full field set (if any)
- Context cost: `S`
- Verification: `cargo check -p pnp-cli --all-targets` — FACT.
- Exit condition: field present and additive; no existing manifest assertion weakened.

### Step 8: Add runtime convergence and visual-debug regressions

- Task IDs: `TASK-570`
- Objective: prove the fixture-equivalent typed input reaches both setup paths with identical schedules and produce a real visual-debug bundle whose `scheduled_layer_zs` is non-uniform.
- Precondition: Steps 5–7 green; guest freshness check exits 0 before diagnosing integration failure.
- Postcondition: AC-5 and AC-6 pass with registered, non-vacuous tests.
- Files allowed to edit (at most 3):
  - `crates/slicer-runtime/tests/integration/layer_range_scope_tdd.rs` (new)
  - `crates/slicer-runtime/tests/integration/main.rs`
  - `crates/pnp-cli/tests/layer_range_scope_visual_debug_tdd.rs` (new)
- Files explicitly out of bounds: production code, existing fixtures/tests, snapshot baselines, guest artifacts.
- Blast-radius discipline: test structs with ≥5 public fields use FRU or a contract-naming waiver; no assertion derives expected Zs from production profile output.
- Context cost: `M`
- Verification: AC-5 and AC-6 commands — FACT.
- Exit condition: runtime test proves identical schedules/values for both paths, visual-debug writes a valid non-empty bundle with the literal `[0.2, 0.4, 0.5, 0.6, 0.7, 0.8, 1.0]` schedule, and a uniform-grid implementation fails at least one literal assertion.

### Step 9: Update contracts and run closure gates

- Task IDs: `TASK-570`
- Objective: document corrected canonical semantics, shared resolver ownership, the WIT `layer-zs` field, and `scheduled_layer_zs`; run every packet/quality gate.
- Precondition: Steps 1–8 pass.
- Postcondition: AC-8 and all packet verification commands pass; no unrelated files or version constants changed.
- Files allowed to edit (at most 3):
  - `docs/02_ir_schemas.md`
  - `docs/04_host_scheduler.md`
  - (`docs/03_wit_and_manifest.md` / `docs/19_visual_debug.md` only if not already edited in Steps 5a/7)
- Files explicitly out of bounds: source plan, predecessor packets, ADRs, deviation log, unrelated docs/code.
- Expected sub-agent dispatches: run AC-8, all-target check/clippy, literals, test-quality report, focused suites, freshness; `FACT: ≤5 lines` each.
- Context cost: `S`
- Verification: AC-8 command; every command in `requirements.md` §Verification Commands.
- Exit condition: docs say earlier-starting trim (not later-starting replacement), all gates pass, and diff scope is limited to design-owned paths.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | S | reconciliation and blast-radius inventories (done) |
| Step 2a | S | deterministic canonical fixture |
| Step 2b | M | XML parser, ordinal mapping, parser failures |
| Step 3 | M | enum/carrier and atomic validation |
| Step 4 | M | canonical profile, top-Z API, both resolver seams |
| Step 5a | M | WIT 3.0.0 bump and host surface reconciliation |
| Step 5b | M | guest variable-schedule algorithm |
| Step 6 | M | runtime carrier and per-layer application |
| Step 7 | S | manifest `scheduled_layer_zs` |
| Step 8 | M | two production-path tests and visual gate |
| Step 9 | S | docs and delegated closure gates |

## Packet Completion Gate

- All steps and exits complete; every pipe-suffixed AC command returns PASS.
- `cargo xtask build-guests --check` exits 0 before interpreting integration/visual-debug failures; guests rebuilt for `@3.0.0`.
- `cargo check --workspace --all-targets`, clippy, literals, and touched-file test-quality review pass.
- Update `docs/07_implementation_status.md` through a worker dispatch, never a full backlog read.
- Confirm the plan-amendment note remains in requirements, while the source plan and packets 01–08 remain untouched.
- `packet.spec.md` is ready for `status: implemented` only after forward dependencies reconcile and all gates pass.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed AC and packet-level gate command.
- Re-inspect only the new fixture's member list/XML through a bounded worker return.
- Record remaining packet-local risk and confirm no IR/serialized manifest-schema version changed beyond the declared WIT `@3.0.0`.
- Confirm the WIT/macro/guest rebuild chain is green and the manifest addition is additive.
