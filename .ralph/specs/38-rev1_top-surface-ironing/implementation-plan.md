# Implementation Plan: top-surface-ironing-rev1

## Execution Rules

- One atomic step at a time.
- Each step maps to TASK-169.
- TDD first (Step 1 sets up failing tests using object-scope fixtures); then skeleton (Step 2); then optional SDK extension contingent on Step 0 finding (Step 0a); then implementation (Step 3); then workspace-test reconciliation (Step 4); then acceptance ceremony (Step 5).
- Each step honors the context-discipline preamble.
- The implementer never reads `OrcaSlicerDocumented/`, `target/`, `Cargo.lock`, or any file > 600 lines in full.
- The implementer rewrites in place — the `top-surface-ironing/` directory already exists from the superseded predecessor. There is no "create new directory" step.

## Steps

### Step 0: Discovery — resolve five FACTs / one SUMMARY before touching code

- Task IDs: `TASK-169`
- Objective: read-only discovery. Answer the five 🔍 questions in `design.md`. The answers determine (a) the manifest's `[ir-access].writes` exact string, (b) whether Step 0a is needed, (c) which `slicer-helpers` API the implementation uses, (d) the manifest_ingestion fix shape, (e) whether the claim-test fix is in scope.
- Precondition: Step 0 not yet run.
- Postcondition: five FACTs and one SUMMARY recorded in the implementer's working notes; implementer makes a go/no-go on Step 0a.
- Files allowed to read: none directly (delegate only).
- Files allowed to edit (≤ 3): none.
- Expected sub-agent dispatches:
  - "FACT: at `crates/slicer-host/src/dispatch.rs:2877`, quote the exact `splice` call line. Is the index `0` (literal prepend) or a variable (computed per-entity)? If a variable, what determines it? Cite file:line."
  - "FACT: in `crates/slicer-ir/src/slice_ir.rs`, search for the canonical kebab-case field-name for ironing on `LayerCollectionIR`. Look for `#[serde(rename = ...)]` and any schema-export macro. Compare with `skirt-brim`'s `LayerCollectionIR.skirt-brim`. Return the exact string the manifest's `[ir-access].writes` should use. Cite file:line."
  - "FACT: in `crates/slicer-helpers/src/`, does any function compute a polygon union or convex hull over a slice of paths/polygons? Symbol search. Return function name + signature + file:line. If absent, return 'no helper exists'."
  - "FACT: in `crates/slicer-host/tests/manifest_ingestion_tdd.rs`, locate `core_modules_all_have_placeholder_wasm_flag_set`. Quote the assertion (≤ 5 lines). Does the test enforce `placeholder_wasm = true` for every core module, or does it allowlist? If every-module, where is the host-side default for the flag set when the manifest omits it? Cite file:line."
  - "SUMMARY ≤ 200 words: in `crates/slicer-host/tests/claim_transition_matrix_tdd.rs`, summarize what `stable_holder_across_layers_is_valid_for_non_transitionable_claim` asserts. The predecessor pass observed `MissingDependency { module: \"fill-role-claim:claim:top-fill\", requires: \"no module holds claim:top-fill\" }` after the new top-surface-ironing module joined the registry. Identify which of these is the cause: (a) a hardcoded list of expected claim holders that needs the new module added/removed; (b) a count-driven invariant; (c) packet 37 fill-role-claim machinery expecting an exact module set; (d) a real claim-graph derivation bug. Recommend whether the fix is mechanical (in-scope for this packet) or substantive (would require its own packet)."
- Context cost: `S`.
- Authoritative docs: none beyond the dispatches.
- OrcaSlicer refs: none.
- Verification: the five returns, recorded.
- Exit condition: implementer can answer the five 🔍 questions without further reading; Step 0a in/out decided; if Step 0 finding (e) recommends substantive fix, escalate to user before proceeding (claim-test fix may exceed packet scope).

### Step 0a (CONDITIONAL): SDK extension for `FinalizationOutputBuilder` insertion mode

- Task IDs: `TASK-169`
- Objective: only run if Step 0 FACT (a) shows the host's splice is literal prepend AND there is no existing host-side ordering pass that would correctly place Ironing entities after fill entities at G-code emit time. Add an APPEND or `insert_after_role` mode to `FinalizationOutputBuilder` and the corresponding host merge code.
- Precondition: Step 0 complete; user-acknowledged go-ahead for Step 0a if scope expansion is needed.
- Postcondition: `FinalizationOutputBuilder` exposes a way to push entities that land AFTER existing fill entities at G-code emit time; backwards compatible with `skirt-brim` (which still wants prepend).
- Files allowed to read:
  - `crates/slicer-sdk/src/builders.rs` (or wherever `FinalizationOutputBuilder` lives — Step 0 FACT will provide the path)
  - `crates/slicer-host/src/dispatch.rs` lines 2860-2900 (narrow range covering the splice site)
- Files allowed to edit (≤ 3):
  - `crates/slicer-sdk/src/builders.rs` (or as located)
  - `crates/slicer-host/src/dispatch.rs` (only the splice block)
  - One test file under `crates/slicer-host/tests/` to cover the new mode (or extend an existing one)
- Expected sub-agent dispatches:
  - "Run `cargo build --workspace`; FACT pass/fail."
  - "Run `cargo test -p skirt-brim`; FACT pass/fail (regression check on existing finalization module)."
- Context cost: `M`.
- Authoritative docs: `docs/05_module_sdk.md` (FinalizationOutputBuilder section).
- OrcaSlicer refs: none.
- Verification:
  - skirt-brim regression test PASSES (existing prepend behavior preserved as the default)
  - workspace build PASSES
- Exit condition: APPEND-equivalent mode available; existing module behavior unchanged.

### Step 1: Author failing TDD with object-scope fixtures

- Task IDs: `TASK-169`
- Objective: rewrite `modules/core-modules/top-surface-ironing/tests/top_surface_ironing_emission_tdd.rs` (existing file from predecessor; full content replacement) with eight tests using `Vec<LayerCollectionIR>` fixtures. The AC-TSI-3 fixture must build a 6-layer object with `top_shell_layers = 3` where layers 3, 4, 5 carry real `TopSolidInfill` paths over the same XY region — substantive interior-vs-topmost discrimination, not an empty region.
- Precondition: Step 0 complete; Step 0a complete or skipped per Step 0 finding.
- Postcondition: tests authored; `cargo test -p top-surface-ironing` either fails to compile (acceptable until Step 2 lands the new struct/trait shape) OR compiles-and-fails with the expected assertion failures.
- Files allowed to read:
  - `modules/core-modules/skirt-brim/tests/finalization_live_tdd.rs` (full read; small) — template for object-scope fixture construction
  - `modules/core-modules/skirt-brim/tests/skirt_brim_tdd.rs` (full read; small) — alternate fixture style
  - `modules/core-modules/top-surface-ironing/tests/top_surface_ironing_emission_tdd.rs` (full read of predecessor's existing content for reference; will be overwritten)
  - `crates/slicer-host/tests/benchy_end_to_end_tdd.rs` lines around `benchy_gcode_contains_ironing_evidence` (predecessor's test; verify still applies and update only if needed)
- Files allowed to edit (≤ 3):
  - `modules/core-modules/top-surface-ironing/tests/top_surface_ironing_emission_tdd.rs` (full rewrite)
  - `crates/slicer-host/tests/benchy_end_to_end_tdd.rs` (only if Step 0 indicated the existing E2E test needs adjustment for the new stage's output channel — likely not, since the assertion is on G-code text)
- Expected sub-agent dispatches:
  - "Run `cargo test -p top-surface-ironing --test top_surface_ironing_emission_tdd 2>&1 | tail -40`; FACT compile-fail or assertion-fail with ≤ 20 lines of failing assertion."
  - "Run `cargo test -p slicer-host --test benchy_end_to_end_tdd benchy_gcode_contains_ironing_evidence 2>&1 | tail -20`; FACT pass/fail."
- Context cost: `M`.
- Authoritative docs: `docs/05_module_sdk.md` (test-fixture pattern for `FinalizationModule`).
- OrcaSlicer refs: none.
- Verification:
  - new tests compile against the (yet-to-be-rewritten) module OR fail-to-compile at the import line — acceptable.
  - host E2E test still compiles + still FAILS (no `;TYPE:Ironing` in output yet).
- Exit condition: 8 module-level tests authored using object-scope fixtures; AC-TSI-3 fixture has real interior-of-stack geometry (NOT an empty region); host E2E test verified to still fail.

### Step 2: Rewrite module skeleton (Cargo.toml + manifest + src/lib.rs stub)

- Task IDs: `TASK-169`
- Objective: rewrite `modules/core-modules/top-surface-ironing/{Cargo.toml, top-surface-ironing.toml, src/lib.rs}` to mirror `skirt-brim` exactly: `FinalizationModule` trait, `run_finalization` callback (stub: returns `Ok(())` without emitting), `on_print_start` with config validation. Manifest declares stage `PostPass::LayerFinalization`, ir-access per Step 0 FACT, claims empty, hints `layer-parallel-safe = false`, Orca-aligned config defaults.
- Precondition: Step 1 complete.
- Postcondition: module package builds; `top-surface-ironing` registered in workspace; build script discovers and produces a (placeholder-behavior) wasm.
- Files allowed to read:
  - `modules/core-modules/skirt-brim/Cargo.toml`, `skirt-brim.toml`, `src/lib.rs` (full reads; all small)
  - `modules/core-modules/top-surface-ironing/Cargo.toml`, `top-surface-ironing.toml`, `src/lib.rs` (predecessor content for reference; will be overwritten)
  - `Cargo.toml` (workspace root) — verify member listing covers `modules/core-modules/top-surface-ironing/` (predecessor pass already added it)
- Files allowed to edit (≤ 3):
  - `modules/core-modules/top-surface-ironing/Cargo.toml`
  - `modules/core-modules/top-surface-ironing/top-surface-ironing.toml`
  - `modules/core-modules/top-surface-ironing/src/lib.rs`
- Expected sub-agent dispatches:
  - "Run `cargo build -p top-surface-ironing`; FACT pass/fail."
  - "Run `./modules/core-modules/build-core-modules.sh`; FACT pass/fail with failing module name on fail."
- Context cost: `M`.
- Authoritative docs: `docs/03_wit_and_manifest.md`, `docs/05_module_sdk.md`.
- OrcaSlicer refs: none.
- Verification:
  - `cargo build -p top-surface-ironing` PASS
  - `./modules/core-modules/build-core-modules.sh` PASS
  - `cargo test -p top-surface-ironing --test top_surface_ironing_emission_tdd 2>&1 | tail -40`: tests now COMPILE (against the new struct/trait shape), and FAIL on the assertion side (because Step 3 hasn't implemented the body yet)
- Exit condition: package builds; WASM rebuild succeeds; module-level tests compile but fail.

### Step 2a: Sync wit-guest crate

- Task IDs: `TASK-169`
- Objective: rewrite `modules/core-modules/top-surface-ironing/wit-guest/{Cargo.toml, src/lib.rs}` to mirror `modules/core-modules/skirt-brim/wit-guest/` — re-export the `TopSurfaceIroning` struct so the `#[slicer_module]`-generated WIT bindings produce a valid cdylib for a `FinalizationModule`.
- Precondition: Step 2 complete.
- Postcondition: `./modules/core-modules/build-core-modules.sh` produces a fresh `top-surface-ironing.wasm` artifact, no longer marked STALE.
- Files allowed to read:
  - `modules/core-modules/skirt-brim/wit-guest/Cargo.toml`, `wit-guest/src/lib.rs` (full reads; small)
- Files allowed to edit (≤ 3):
  - `modules/core-modules/top-surface-ironing/wit-guest/Cargo.toml`
  - `modules/core-modules/top-surface-ironing/wit-guest/src/lib.rs`
- Expected sub-agent dispatches:
  - "Run `./modules/core-modules/build-core-modules.sh`; FACT pass/fail with failing module name on fail. Specifically check the line for `top-surface-ironing.wasm` is no longer STALE."
- Context cost: `S`.
- Authoritative docs: `docs/03_wit_and_manifest.md`.
- OrcaSlicer refs: none.
- Verification:
  - build-core-modules.sh PASS with no STALE entries.
- Exit condition: `top-surface-ironing.wasm` rebuilds clean.

### Step 3: Implement ironing path generation

- Task IDs: `TASK-169`
- Objective: implement the body of `run_finalization` in `modules/core-modules/top-surface-ironing/src/lib.rs`. Algorithm exactly as in `design.md` § Code Change Surface. All eight module-level tests must PASS.
- Precondition: Step 2 and Step 2a complete; Step 0a complete or skipped.
- Postcondition: every test in `top_surface_ironing_emission_tdd.rs` PASSES.
- Files allowed to read:
  - `modules/core-modules/top-surface-ironing/src/lib.rs` (the stub from Step 2)
  - `modules/core-modules/top-surface-ironing/tests/top_surface_ironing_emission_tdd.rs` (the contract)
  - `modules/core-modules/skirt-brim/src/lib.rs` (full read; reference)
  - `crates/slicer-sdk/src/views.rs` — symbol search ONLY for `LayerCollectionView`, `FinalizationOutputBuilder::push_entity_to_layer`, `ConfigView::get_*`, `ExtrusionPath3D` constructor, `ExtrusionRole`
  - `crates/slicer-helpers/src/` — symbol search ONLY for the union/bounding helper located in Step 0 FACT (c)
- Files allowed to edit (≤ 3):
  - `modules/core-modules/top-surface-ironing/src/lib.rs`
- Expected sub-agent dispatches:
  - "Run `cargo test -p top-surface-ironing --test top_surface_ironing_emission_tdd -- --nocapture 2>&1 | tail -60`; FACT pass/fail per test with ≤ 20 lines of failing assertion on FAIL."
  - "Run `./modules/core-modules/build-core-modules.sh`; FACT pass/fail (post-edit rebuild)."
- Context cost: `M`.
- Authoritative docs: `docs/05_module_sdk.md`, `docs/02_ir_schemas.md`, `docs/08_coordinate_system.md`.
- OrcaSlicer refs: `Fill.cpp::make_ironing` (delegate SUMMARY only if implementation parity is challenged by a test; predecessor already produced the SUMMARY).
- Verification:
  - 8/8 module tests PASS
  - WASM rebuild PASSES
- Exit condition: all module-level tests pass; WASM rebuild succeeds; entity-push contract verified by tests regardless of host merge order.

### Step 4: Workspace test reconciliation

- Task IDs: `TASK-169`
- Objective: bring `manifest_ingestion_tdd` and (conditionally) `claim_transition_matrix_tdd` to PASS. The manifest_ingestion fix is mechanical per Step 0 FACT (d). The claim_transition_matrix fix is in scope only if Step 0 SUMMARY (e) recommended a mechanical fix; otherwise this step records the failure as a known regression and escalates to a separate packet.
- Precondition: Step 3 complete.
- Postcondition: `manifest_ingestion_tdd` PASSES; `claim_transition_matrix_tdd` either PASSES or is documented as out-of-scope with a recommended follow-up packet.
- Files allowed to read:
  - `crates/slicer-host/tests/manifest_ingestion_tdd.rs` — only the failing test functions (FACT-narrowed)
  - `crates/slicer-host/tests/claim_transition_matrix_tdd.rs` — only the failing test (FACT-narrowed) — only if Step 0 (e) recommended in-scope fix
- Files allowed to edit (≤ 3):
  - `crates/slicer-host/tests/manifest_ingestion_tdd.rs`
  - `crates/slicer-host/tests/claim_transition_matrix_tdd.rs` (conditional)
  - `modules/core-modules/top-surface-ironing/top-surface-ironing.toml` (only if Step 0 (d) shows the manifest must declare `placeholder_wasm = true` — preferred over editing the test if the host-side default semantics support it)
- Expected sub-agent dispatches:
  - "Run `cargo test -p slicer-host --test manifest_ingestion_tdd 2>&1 | tail -40`; FACT pass/fail per test."
  - "Run `cargo test -p slicer-host --test claim_transition_matrix_tdd 2>&1 | tail -40`; FACT pass/fail per test."
- Context cost: `S` if both fixes are mechanical; `M` if claim_transition_matrix needs investigation. Hard cap: if context cost would exceed `M`, stop and escalate.
- Authoritative docs: `docs/03_wit_and_manifest.md` for `placeholder_wasm` semantics if applicable.
- OrcaSlicer refs: none.
- Verification:
  - `cargo test -p slicer-host --test manifest_ingestion_tdd` PASS
  - `cargo test -p slicer-host --test claim_transition_matrix_tdd` PASS or out-of-scope-documented
- Exit condition: both targeted tests green, OR a documented escalation note for claim_transition_matrix recommending a follow-up packet.

### Step 5: Acceptance ceremony + docs/07 row

- Task IDs: `TASK-169`
- Objective: re-run every acceptance command from `packet.spec.md`; run workspace gates (`cargo test --workspace`, `cargo clippy --workspace -- -D warnings`); insert `TASK-169` row in `docs/07_implementation_status.md`.
- Precondition: Step 4 complete (or escalation acknowledged).
- Postcondition: every AC PASSES; backlog updated.
- Files allowed to read: none directly (dispatch only).
- Files allowed to edit (≤ 3):
  - `docs/07_implementation_status.md` (delegate the insertion via a worker — the file is large and must not be loaded into the planner)
- Expected sub-agent dispatches:
  - 9 narrow AC commands from `packet.spec.md` `## Acceptance Criteria` and `## Negative Test Cases`, each as a separate FACT pass/fail.
  - "Run `cargo test --workspace --no-fail-fast 2>&1 | tail -40`; FACT pass/fail with failing test list (≤ 20 lines)."
  - "Run `cargo clippy --workspace -- -D warnings 2>&1 | tail -20`; FACT pass/fail."
  - "Run `./modules/core-modules/build-core-modules.sh`; FACT pass/fail."
  - "Insert a TASK-169 row into `docs/07_implementation_status.md` describing this packet's deliverable. Return the inserted line as FACT (file:line, contents). Do NOT load the whole file."
- Context cost: `S`.
- Authoritative docs: `docs/07_implementation_status.md` (delegate-only).
- OrcaSlicer refs: none.
- Verification: every pipe-suffixed AC command from `packet.spec.md`.
- Exit condition: every AC PASSES; `cargo test --workspace` PASSES (closure gate); `cargo clippy --workspace -- -D warnings` PASSES; `docs/07` carries TASK-169.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 0 | S | Five FACT/SUMMARY dispatches. |
| Step 0a | M | CONDITIONAL — only if Step 0 reveals SDK extension is needed. |
| Step 1 | M | TDD authoring with object-scope fixtures (8 tests). |
| Step 2 | M | Skeleton rewrite + manifest. |
| Step 2a | S | wit-guest sync. |
| Step 3 | M | Implementation body. |
| Step 4 | S–M | Workspace test reconciliation. |
| Step 5 | S | Acceptance + docs row insertion. |

Aggregate: `M`. No single step is `L`.

## Packet Completion Gate

- All steps complete (Step 0a may be skipped per Step 0 finding).
- Every AC verification command from `packet.spec.md` PASSES.
- `cargo test --workspace` PASSES.
- `cargo clippy --workspace -- -D warnings` PASSES.
- `./modules/core-modules/build-core-modules.sh` PASSES.
- `docs/07_implementation_status.md` carries TASK-169.
- `packet.spec.md` ready to move to `status: implemented`.
- Predecessor `38_top-surface-ironing/packet.spec.md` already at `status: superseded` (planner-set during packet authoring).

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed AC command (9 commands).
- Confirm `cargo test --workspace`, `cargo clippy --workspace -- -D warnings`, and `./modules/core-modules/build-core-modules.sh` PASS.
- Record any remaining packet-local risk (especially: any chosen-different-from-Orca defaults — none, since this packet is Orca-aligned; insertion-order assumption if Step 0a was skipped).
- Confirm implementer's peak context usage stayed under 70%.
- Confirm no entries added to `docs/DEVIATION_LOG.md` (Orca alignment is the chosen approach; no deviation to register).
