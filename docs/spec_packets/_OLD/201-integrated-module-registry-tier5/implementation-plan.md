# Implementation Plan: 201-integrated-module-registry-tier5

## Execution Rules

- Work one atomic step at a time; map every step to grouped task IDs.
- Use TDD, then implementation, then the narrowest falsifying validation.
- Every field below is a context-budget contract and must be filled independently; never write "see Step 1".

## Steps

### Step 1: Red tests for tier-5 ingestion, dedup, and shadow diagnostic

- Task IDs: `ADR-0056` (Decision items 1–2)
- Objective: author failing integration tests for AC-1, AC-2, AC-N1, AC-N2 in a new scheduler test file.
- Precondition: clean tree; `cargo test -p slicer-scheduler --test scheduler_integration` green.
- Postcondition: new file compiles against the *planned* API (`ModuleProvenance`, `IntegratedModuleRegistration`, `load_modules_from_roots_with_integrated`) and fails to compile or fails assertions — red state recorded.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-scheduler/tests/integration/manifest_ingestion_tdd.rs` - fixture-helper region only (locate `fixture.root()` pattern via the dispatched FACT first)
  - `crates/slicer-scheduler/src/manifest.rs` - lines 480–660 (diagnostic/report types, loader loop)
- Files allowed to edit (at most 3):
  - `crates/slicer-scheduler/tests/integration/integrated_tier_tdd.rs` (new)
  - `crates/slicer-scheduler/tests/integration/main.rs` (add `mod integrated_tier_tdd;`)
- Files explicitly out of bounds:
  - `crates/slicer-scheduler/src/manifest.rs` (no production edits this step)
- Expected sub-agent dispatches:
  - Question: which fixture helper synthesizes a module root in `manifest_ingestion_tdd.rs` and what is its exact call shape; scope: `crates/slicer-scheduler/tests/**`; return: FACT ≤5 lines
- Context cost: `S`
- Authoritative docs:
  - `docs/adr/0056-integrated-modules-native-dispatch.md` - §Decision items 1–2, direct read
- OrcaSlicer refs: none
- Verification:
  - `cargo test -p slicer-scheduler --test scheduler_integration integrated 2>&1 | tee target/test-output.log` - expect compile failure or failing tests (red)
- Exit condition: red state exists and names the four planned test fns (`integrated_manifest_ingests_without_wasm`, `external_root_overrides_integrated_tier`, `external_shadow_diagnostic_names_integrated_loser`, `empty_integrated_registry_is_identity`); if the tests pass without production changes, the step is falsified — stop and re-derive.

### Step 2: Green — provenance, text-source ingestion, tier-5 loop in manifest.rs

- Task IDs: `ADR-0056` (Decision items 1–2)
- Objective: implement `ModuleProvenance`, `IntegratedModuleRegistration`, `ingest_manifest_text`, `load_modules_from_roots_with_integrated`, and the provenance-aware shadow diagnostic; re-export from `slicer_scheduler`.
- Precondition: Step 1 red state.
- Postcondition: Step 1 tests green; existing `manifest_ingestion_tdd.rs` suite untouched and green.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-scheduler/src/manifest.rs` - lines 28–400 and 566–860
  - `crates/slicer-scheduler/src/lib.rs` - whole (re-export list)
- Files allowed to edit (at most 3):
  - `crates/slicer-scheduler/src/manifest.rs`
  - `crates/slicer-scheduler/src/lib.rs`
- Files explicitly out of bounds:
  - `crates/slicer-scheduler/src/module_search_path.rs`, `execution_plan.rs`, `validation.rs`
- Blast-radius discipline (mandatory when adding a new struct field or schema constant):
  - `LoadedModule.provenance` is added behind `LoadedModuleBuilder` with default `External`; the only struct-literal site is inside `LoadedModuleBuilder::build` (verified at authoring via tree grep: most fields are `pub(crate)` — `region_splits` and `region_split_semantics` are `pub` — but a literal needs every field visible, so external constructions all go through the builder). Confirm with the dispatched LOCATIONS sweep before editing; if any in-crate literal exists outside `build()`, add it to this step's edits (still ≤3 files).
  - Dispatch a `LOCATIONS` worker for full-struct `LoadedModule` PartialEq assertions in tests; cite the result inline in the commit message.
- Expected sub-agent dispatches:
  - Question: list construction sites `LoadedModule {` and full-struct equality assertions on `LoadedModule`; scope: `crates/`; return: LOCATIONS ≤20
- Context cost: `M`
- Authoritative docs:
  - `docs/adr/0056-integrated-modules-native-dispatch.md` - Decision items 1–2
  - `docs/03_wit_and_manifest.md` - delegate a FACT on required manifest fields if uncertainty arises
- OrcaSlicer refs: none
- Verification:
  - `cargo test -p slicer-scheduler --test scheduler_integration 2>&1 | tee target/test-output.log` - whole binary green (new + regression)
- Exit condition: AC-1, AC-2, AC-N1, AC-N2 commands PASS; any pre-existing manifest test failure falsifies the refactor (disk semantics must be byte-for-byte preserved).

### Step 3: Registry crate `slicer-integrated-modules`

- Task IDs: `ADR-0056` (Decision item 1)
- Objective: create the workspace crate with per-module features, `include_str!` manifest embedding, `integrated_registrations()`, and the feature-gated proving test for `classic-perimeters`.
- Precondition: Step 2 merged (registration type exists).
- Postcondition: AC-4 command passes; default build of the crate exports an empty registration set.
- Files allowed to read, with ranges when over 300 lines:
  - `Cargo.toml` (workspace members block only)
  - `modules/core-modules/classic-perimeters/classic-perimeters.toml` - top identity section only (id sanity)
- Files allowed to edit (at most 3):
  - `crates/slicer-integrated-modules/Cargo.toml` (new)
  - `crates/slicer-integrated-modules/src/lib.rs` (new)
  - `Cargo.toml` (root; add member)
- Files explicitly out of bounds:
  - `modules/core-modules/**` (read-only), `crates/pnp-cli/**`
- Expected sub-agent dispatches: none (self-contained).
- Context cost: `S`
- Authoritative docs:
  - `docs/adr/0057-three-editions-and-integrated-tier.md` - Hybrid/Integrated feature-list rationale, direct read
- OrcaSlicer refs: none
- Verification:
  - `cargo test -p slicer-integrated-modules --features classic-perimeters 2>&1 | tee target/test-output.log` - proving test green
  - `cargo test -p slicer-integrated-modules 2>&1 | tee target/test-output.log` - compiles; zero-feature build has no registrations (assert via a default-features test that `integrated_registrations().is_empty()`)
- Exit condition: AC-4 PASS and the default-features emptiness test PASS; a non-empty default registration set falsifies the step.

### Step 4: Integrated-aware live-plan entry point in slicer-wasm-host

- Task IDs: `ADR-0056` (Decision items 1–2)
- Objective: add `load_live_modules_for_plan_with_integrated` with the provenance compile-skip guard (`wasm_component: None`, no `compile_module_component` call for Integrated provenance); make `load_live_modules_for_plan_profiled` delegate with `&[]`; add the AC-5 test.
- Precondition: Steps 1–2 green.
- Postcondition: AC-5 command passes; existing live-loading tests green.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-wasm-host/src/execution_plan_live.rs` - short (353 lines at authoring); read whole or locate by symbol, never by a pinned range
  - `crates/slicer-runtime/tests/integration/live_module_loading_tdd.rs` - test-setup region only
- Files allowed to edit (at most 3):
  - `crates/slicer-wasm-host/src/execution_plan_live.rs`
  - `crates/slicer-runtime/tests/integration/live_module_loading_tdd.rs`
- Files explicitly out of bounds:
  - `crates/slicer-wasm-host/src/dispatch.rs`, `binding.rs` (202 territory)
- Blast-radius discipline: `LiveModuleBinding` gains no field in this packet — the guard only sets the existing `wasm_component: Option<..>` to `None`; struct literals unchanged elsewhere (verified at authoring: literal sites are `execution_plan_live.rs` and `crates/slicer-runtime/tests/contract/config_view_binding_tdd.rs`, neither needing edits).
- Expected sub-agent dispatches:
  - Question: does `live_module_loading_tdd.rs` have a helper to build a synthetic module + roots usable for an integrated registration test; scope: that file; return: FACT ≤5 lines
- Context cost: `M`
- Authoritative docs:
  - `docs/adr/0005-runner-traits-in-slicer-wasm-host.md` - delegate SUMMARY only if the binding shape is unclear
- OrcaSlicer refs: none
- Verification:
  - `cargo test -p slicer-runtime --test integration live_module 2>&1 | tee target/test-output.log`
  - `cargo test -p slicer-runtime --test integration integrated_binding_skips_component_compile 2>&1 | tee target/test-output.log`
- Exit condition: AC-5 PASS; regression: `live_module_loading` tests all green. An integrated module reaching `compile_module_component` (observable as a `LiveModuleLoadError::Component` for the synthetic id) falsifies the guard.

### Step 5: Production wiring in run.rs + runtime re-export

- Task IDs: `ADR-0056` (Decision item 1)
- Objective: switch both live-loader call sites in `crates/slicer-runtime/src/run.rs` to `load_live_modules_for_plan_with_integrated`, sourcing `slicer_integrated_modules::integrated_registrations()`; add the Cargo dep; re-export `ModuleProvenance` from `crates/slicer-runtime/src/lib.rs`. **Also re-export `load_live_modules_for_plan_with_integrated` in the same `pub use slicer_wasm_host::{…}` block.** Verified at authoring: that block already re-exports `load_live_modules_for_plan` and `load_live_modules_for_plan_with_config`, so omitting the new entry point leaves the runtime's re-export set inconsistent — and packet 203 reaches the disable seam through `slicer_runtime::`. Cheaper to settle here than to discover it in 203. The two call sites use *different* entry points (`run_slice_with_collector` → `..._profiled`; `prepare_prepass_context` → `..._with_config`), so the second must pass `profile: false`.
- Precondition: Steps 3–4 green.
- Postcondition: AC-3 greps pass; default behavior unchanged (empty registry).
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-runtime/src/run.rs` - the two loader call regions only (locate by `assemble_search_roots` usages)
  - `crates/slicer-runtime/src/lib.rs` - re-export region (dispatched SNIPPETS first)
- Files allowed to edit (at most 3):
  - `crates/slicer-runtime/src/run.rs`
  - `crates/slicer-runtime/Cargo.toml`
  - `crates/slicer-runtime/src/lib.rs`
- Files explicitly out of bounds:
  - `crates/pnp-cli/**` (CLI provenance is 203), `SliceRunOptions` struct (no field changes — 203 owns the disable flag)
- Expected sub-agent dispatches:
  - Question: what does the second loader call site (near the `module_dirs: &[PathBuf]` helper) serve — slice, report, or dag path; scope: `crates/slicer-runtime/src/run.rs`; return: FACT ≤5 lines; purpose: confirm both sites should carry the integrated tier ([FWD] in design.md)
- Context cost: `S`
- Authoritative docs:
  - `docs/01_system_architecture.md` - §Module Search Path note that loader symbols re-export via `slicer_runtime::`
- OrcaSlicer refs: none
- Verification:
  - `rg -q 'load_live_modules_for_plan_with_integrated' crates/slicer-runtime/src/run.rs && rg -q 'integrated_registrations' crates/slicer-runtime/src/run.rs` - FACT pass/fail
  - `cargo check --workspace --all-targets 2>&1 | tee target/test-output.log` - FACT pass/fail
- Exit condition: AC-3 PASS and workspace check green; if the dispatched FACT shows the second call site must stay external-only (e.g. it feeds a path where integrated modules are meaningless), record that decision in the commit message and adjust AC-3's second grep target accordingly *before* closing — never silently skip a call site.

### Step 6: Docs + closure gates

- Task IDs: `ADR-0056` (Decision items 1–2)
- Objective: land the two doc edits (docs/01 §Module Search Path tier 5 + shadow diagnostic; docs/04 §Phase 1 provenance paragraph) and run the packet gates.
- Precondition: Steps 1–5 complete.
- Postcondition: AC-N3 greps pass; gates green.
- Files allowed to read, with ranges when over 300 lines:
  - `docs/01_system_architecture.md` - lines 912–1015 only
  - `docs/04_host_scheduler.md` - §Phase 1 heading region only (locate via grep)
- Files allowed to edit (at most 3):
  - `docs/01_system_architecture.md`
  - `docs/04_host_scheduler.md`
- Files explicitly out of bounds:
  - `CONTEXT.md`, `docs/adr/*`, `docs/07_implementation_status.md`
- Expected sub-agent dispatches:
  - Question: run the three gate commands + all AC commands; scope: repo root; return: FACT pass/fail each
- Context cost: `S`
- Authoritative docs:
  - `docs/adr/0056-integrated-modules-native-dispatch.md` - consequence bullets for the shadow-diagnostic wording
- OrcaSlicer refs: none
- Verification:
  - `rg -qi 'tier 5' docs/01_system_architecture.md && rg -q 'ModuleProvenance' docs/04_host_scheduler.md` - FACT pass/fail
  - `cargo clippy --workspace --all-targets -- -D warnings 2>&1 | tee target/test-output.log` - FACT pass/fail
- Exit condition: every pipe-suffixed AC command PASS; doc greps PASS.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | S | test authoring against planned API |
| Step 2 | M | manifest.rs refactor — largest step |
| Step 3 | S | new crate, self-contained |
| Step 4 | M | wasm-host entry point + guard |
| Step 5 | S | wiring + re-export |
| Step 6 | S | docs + gates |

Split before activation if aggregate cost exceeds M or any step is L.

## Packet Completion Gate

- All steps and exits complete.
- Every pipe-suffixed AC command returns PASS.
- `docs/07_implementation_status.md` is NOT updated by this packet (no TASK rows exist; plan-level [FWD] pending — see `requirements.md` §Packet Metadata).
- Reconcile reopened/superseded status transitions: none (no packet superseded).
- `packet.spec.md` is ready for `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed AC and packet-level gate command.
- Record remaining packet-local risk (feature-unification hazard; 202's planned signature extension of `load_live_modules_for_plan_with_integrated`).
- Confirm context stayed at or below 150k standard, or at/below 300k only with a logged swarm ESCALATION; otherwise record a packet-authoring lesson.

All `cargo check`, `cargo clippy`, and `cargo test` invocations in gate and verification commands must use `--all-targets` so the test, bench, and example targets compile.
