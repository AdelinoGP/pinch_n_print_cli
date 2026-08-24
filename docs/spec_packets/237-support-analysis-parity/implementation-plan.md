# Implementation Plan: 237-support-analysis-parity

## Execution Rules

- Work one atomic step at a time; map every step to grouped task IDs.
- Use TDD, then implementation, then the narrowest falsifying validation.
- Every field below is a context-budget contract and must be filled independently; never
  write "see Step N".
- E6: every `slicer-core` test invocation carries `--features host-algos`.
- E4/T4: run `cargo xtask build-guests --check` before attributing any guest-facing failure;
  this packet's surface feeds guest builds.
- Invariant 16 / T2: every verification asserts a non-zero matched-test count in-run.
- Tee every cargo test to `target/test-output.log`; read the file, never re-run.

## Steps

### Step 1: Red tests for sharp-tails and enforce_support_layers stages

- Task IDs: `TASK-353`
- Objective: author the failing tests that name the new stage behavior in
  `support_overhang_detection_tdd.rs` (`sharp_tails_add_first_layer_contacts_when_enabled`,
  `sharp_tails_disabled_by_default_emits_none`, `enforce_support_layers_forces_full_contacts_in_leading_layers`,
  `enforce_support_layers_beyond_model_changes_nothing`), extending the existing
  `SupportContactParams` fixtures; do not implement yet — the step ends red with exact
  compile errors naming the missing params fields.
- Precondition: clean tree on `parity/support-planners-clean`; existing suite green under
  `--features host-algos`.
- Postcondition: four named tests exist and fail for the right reason (missing fields /
  unimplemented behavior); no production file edited.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-core/tests/support_overhang_detection_tdd.rs` - full (~400 lines)
  - `crates/slicer-core/src/algos/overhang_annotation.rs` - lines 160–310 only (params struct
    + function header + "Not modelled" list)
- Files allowed to edit (at most 3):
  - `crates/slicer-core/tests/support_overhang_detection_tdd.rs`
- Files explicitly out of bounds:
  - everything under `src/` this step; all other crates
- Expected sub-agent dispatches:
  - Question: canonical sharp-tail detection shape (what geometry it emits at layer 0); scope:
    `OrcaSlicerDocumented/src/libslic3r/Support/SupportMaterial.cpp`; return: `SNIPPETS`
    ≤30 lines; purpose: write an honest red assertion.
- Context cost: `S`
- Authoritative docs:
  - `docs/specs/support-families-anchored-entities-plan.md` - §7 E1/E8, §13 T6
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/Support/SupportMaterial.cpp` - `detect_overhangs`;
    delegate; never load
- Verification:
  - `mkdir -p target && cargo test -p slicer-core --features host-algos --test support_overhang_detection_tdd sharp_tails 2>&1 | tee target/test-output.log && grep -cE "^test .+ FAILED|error\[E" target/test-output.log` — expect ≥2 (red state proven)
- Exit condition: the log shows exactly the authored tests failing/compiling-error; zero
  unrelated failures.

### Step 2: Bridge-removal stage (red → green)

- Task IDs: `TASK-354`
- Objective: implement bridge removal inside `detect_support_contacts` behind a typed
  parameter (`bridge_no_support: bool` + bridge polygons input), porting canonical
  `SupportMaterialInternal::remove_bridges_from_contacts` semantics; add red-first pair
  (`bridge_areas_are_removed_from_contacts_under_bridge_no_support`,
  `bridge_removal_disabled_keeps_bridge_contacts`) then make them pass.
- Precondition: Step 1 merged in-tree (its red tests may stay red; they are separate
  behaviors).
- Postcondition: AC-3 and AC-N2 commands return PASS; "Not modelled" doc list drops the
  bridge entry.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-core/src/algos/overhang_annotation.rs` - lines 100–360
  - `crates/slicer-core/tests/support_overhang_detection_tdd.rs` - full
- Files allowed to edit (at most 3):
  - `crates/slicer-core/src/algos/overhang_annotation.rs`
  - `crates/slicer-core/tests/support_overhang_detection_tdd.rs`
- Files explicitly out of bounds:
  - `crates/slicer-runtime/**` (param plumbing is later steps), manifests, WIT
- Expected sub-agent dispatches:
  - Question: exact offset magnitudes and polygon inputs of
    `remove_bridges_from_contacts`; scope: `OrcaSlicerDocumented/.../SupportMaterial.cpp`;
    return: `SNIPPETS` ≤30 lines; purpose: faithful port (E8 constants).
- Context cost: `S`
- Authoritative docs:
  - `docs/08_coordinate_system.md` - mm↔unit boundary checklist (delegated SUMMARY acceptable)
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/Support/SupportMaterial.cpp` -
    `SupportMaterialInternal::remove_bridges_from_contacts`; delegate
- Verification:
  - AC-3 command from `packet.spec.md`, then AC-N2 command — each must print PASS
- Exit condition: both commands PASS and `cargo check -p slicer-core --all-targets` clean.

### Step 3: Cantilever pass + SupportAnalysisIR schema bump

- Task IDs: `TASK-355`
- Objective: post-union cantilever annotation pass recording wide spans into additive
  `SupportAnalysisIR.cantilever_surfaces`; minor-version bump of
  `CURRENT_SUPPORT_ANALYSIS_IR_SCHEMA_VERSION` with full blast radius owned here. The bump
  value derives from the live constant at activation time (live today: 1.1.0 ⇒ minor bump);
  no test or doc asserts a frozen literal — assertions reference the constant.
- Precondition: Steps 1–2 landed; dispatch of struct-literal-site enumeration completed
  (LOCATIONS ≤20 recorded below the fold in this step's working notes).
- Postcondition: AC-5 command PASS; `cargo check --workspace --all-targets` clean proving the
  literal blast radius is fully covered; version constant carries its additive minor bump.
- Blast-radius discipline (mandatory): the step's edit list includes every site that compiles
  against changed structs — expected sites (verify via the LOCATIONS dispatch before editing):
  `SupportAnalysisIR` literals/constructors in `crates/slicer-runtime/src/builtins/support_analysis_producer.rs`
  (uses `..SupportAnalysisIR::default()` — verify), marshalling projections in
  `crates/slicer-wasm-host/src/marshal/{in_,native}.rs` (field-additive, default-init),
  serde fixture JSONs asserting `schema_version` 1.1.0 (grep `1.1.0` under
  `crates/*/tests/fixtures` touching support analysis). Because the added map is
  `#[serde(default)]` and constructors use `Default`, the anticipated fallout is limited to
  schema-version assertions; any additional literal surfaced by `cargo check --workspace
  --all-targets` is fixed in THIS step, never deferred.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-core/src/algos/overhang_annotation.rs` - lines 300–370 (union_ex tail)
  - `crates/slicer-ir/src/slice_ir.rs` - lines 186–200, 276–285, 700–740 (version constants,
    `SupportAnalysisIR`)
- Files allowed to edit (at most 3):
  - `crates/slicer-core/src/algos/overhang_annotation.rs`
  - `crates/slicer-ir/src/slice_ir.rs`
  - `crates/slicer-core/tests/support_overhang_detection_tdd.rs`
  (producer-side wiring of the new output lands in Step 4's edit budget)
- Files explicitly out of bounds:
  - `crates/slicer-schema/wit/**` (no WIT change — host-only field, design decision locked)
- Expected sub-agent dispatches:
  - Question: enumerate non-test struct-literal/assertion sites pinning
    `CURRENT_SUPPORT_ANALYSIS_IR_SCHEMA_VERSION` or constructing `SupportAnalysisIR`; scope:
    `crates/`; return: `LOCATIONS` ≤20; purpose: pre-bake blast radius.
- Context cost: `M`
- Authoritative docs:
  - `docs/02_ir_schemas.md` - SupportAnalysisIR section (ranged read)
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/Support/SupportMaterial.cpp` - `detect_overhangs`
    cantilever tail (`layer.cantilevers`, `dist_max > scale_(3)`); delegate
- Verification:
  - AC-5 command PASS; `cargo check --workspace --all-targets` exit 0
- Exit condition: constant bumped by derivation from its live value, AC-5 PASS, workspace
  compiles including all test targets.

### Step 4: Divergence 5.2 routing fix + NormalAuto doc correction

- Task IDs: `TASK-356`
- Objective: restructure the producer routing branch so enforcer contacts apply under every
  `support_type` (union with thresholded contacts under auto); correct the
  `SupportType::NormalAuto`/`is_auto` doc comments in `slice_ir.rs`; red-first producer test
  `auto_support_type_unions_enforcer_contacts_with_thresholded` added to the in-file
  `#[cfg(test)]` module (reached via `--lib`, NOT via `tests/unit/`).
- Precondition: Step 3 landed (this file already touched by cantilever wiring — compose, do
  not duplicate). FORWARD-DEP gate: this is a shared-surface step per `packet.spec.md`
  Prerequisites — its green-light composition checks run after 236 reaches `implemented`.
- Postcondition: AC-1 PASS; AC-N5 (both manual-routing regressions) still PASS;
  doc comment no longer contradicts behavior.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-runtime/src/builtins/support_analysis_producer.rs` - lines 130–350
    (routing) and the in-file `#[cfg(test)]` module
  - `crates/slicer-ir/src/slice_ir.rs` - lines 1880–1930 (`SupportType` docs)
- Files allowed to edit (at most 3):
  - `crates/slicer-runtime/src/builtins/support_analysis_producer.rs`
  - `crates/slicer-ir/src/slice_ir.rs` (doc comments ONLY — behavior strings untouched)
- Files explicitly out of bounds:
  - `crates/slicer-runtime/tests/unit/main.rs` and all of `tests/` (source-module tests only
    here); `crates/slicer-scheduler/**` entirely; family minting logic beyond composing with it
- Expected sub-agent dispatches:
  - none planned; if `family_assignments` interplay surprises, dispatch FACT against the
    minting block (lines ~253–278) before changing anything there.
- Context cost: `M`
- Authoritative docs:
  - `docs/specs/support-families-anchored-entities-plan.md` - §12 237 brief divergence 5.2
    paragraph
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/Support/SupportMaterial.cpp` - `detect_contacts`
    enforcer branch; delegate
- Verification:
  - AC-1 command PASS; AC-N5 command PASS; `mkdir -p target && cargo check -p slicer-runtime --all-targets 2>&1 | tee target/test-output.log && echo CHECK-OK`
- Exit condition: three commands green; no candidate-stream reorder (sort keys unchanged).

### Step 5: G-17 producer — classify_object derivation

- Task IDs: `TASK-357`
- Objective: replace the `needs_support: true` hardcode in `classify_object` with derivation
  from overhang-region presence/footprint semantics so downstream view derivation has real
  per-object data; document the region-presence contract on `OverhangRegion.needs_support`
  (`slice_ir.rs` doc comment refresh).
- Precondition: none beyond clean tree (independent of Steps 2–4).
- Postcondition: `algo_mesh_analysis_tdd` extended coverage still green; the flag now reflects
  classification rather than a constant.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-core/src/algos/mesh_analysis.rs` - lines 100–240
  - `crates/slicer-ir/src/slice_ir.rs` - lines 640–686 (`OverhangRegion`)
- Files allowed to edit (at most 3):
  - `crates/slicer-core/src/algos/mesh_analysis.rs`
  - `crates/slicer-core/tests/algo_mesh_analysis_tdd.rs`
  - `crates/slicer-ir/src/slice_ir.rs` (doc comment only)
- Files explicitly out of bounds:
  - `crates/slicer-sdk/**` (next step), runtime, wasm-host
- Expected sub-agent dispatches:
  - Question: current assertions over `needs_support` in `algo_mesh_analysis_tdd`; scope:
    `crates/slicer-core/tests/algo_mesh_analysis_tdd.rs`; return: `LOCATIONS` ≤10; purpose:
    know which assertions the change touches.
- Context cost: `S`
- Authoritative docs:
  - `docs/02_ir_schemas.md` - §IR 2 eligibility paragraphs
- OrcaSlicer refs:
  - none (PnP-internal signal semantics; canonical comparison happens at the consumer)
- Verification:
  - `mkdir -p target && cargo test -p slicer-core --features host-algos --test algo_mesh_analysis_tdd 2>&1 | tee target/test-output.log && grep -E "test result: ok" target/test-output.log`
- Exit condition: suite green with at least one new/updated assertion exercising a false case.

### Step 6a: G-17 view derivation + SDK-level test

- Task IDs: `TASK-358`
- Objective: add `SliceRegionView::derive_needs_support(Option<&SurfaceClassificationIR>)`
  (footprint-vs-polygons disjointness → false) and wire the two marshal call sites
  (`build_native_layer_request` gains a `surface_classification` parameter — it does not
  read `input.surface_classification` today; thread the field from the `LayerStageInput`
  call site, mirroring `sliced_region_to_data`, which already projects it;
  confirming via FACT read that the
  macro shim keeps forwarding the accessor). Red-first tests: `derive_needs_support` family in
  `layer_module_tdd.rs` (AC-6).
- Precondition: Step 5 landed.
- Postcondition: AC-6 command PASS; both marshal sites compile with the derived value.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-wasm-host/src/binding.rs` - lines 79–107 (`LayerStageInput`)
  - `crates/slicer-wasm-host/src/dispatch.rs` - LOCATIONS-dispatched call-site windows only
  - `crates/slicer-sdk/tests/layer_module_tdd.rs` - existing `set_needs_support` test window
    (~lines 460–500)
- Files allowed to edit (at most 3):
  - `crates/slicer-sdk/src/views.rs`
  - `crates/slicer-wasm-host/src/marshal/native.rs`
  - `crates/slicer-sdk/tests/layer_module_tdd.rs`
- Files explicitly out of bounds:
  - `crates/slicer-wasm-host/src/marshal/in_.rs` (its wasm-side wiring is verified by read +
    check only in this sub-step; if it needs an edit, STOP and split — see 6b), 
    `crates/slicer-macros/src/lib.rs`, any WIT file, guest sources
- Expected sub-agent dispatches:
  - Question: does the macro region adapter still forward `needs_support()` unchanged, and do
    both marshal sites receive `surface_classification`?; scope:
    `crates/slicer-macros/src/lib.rs` + `crates/slicer-wasm-host/src/dispatch.rs`; return:
    `FACT`; purpose: leg-skew guard (T9) before editing.
- Context cost: `S`
- Authoritative docs:
  - `docs/specs/support-families-anchored-entities-plan.md` - §12 237 brief G-17 bullet
- OrcaSlicer refs:
  - none
- Verification:
  - AC-6 command PASS; `cargo check -p slicer-wasm-host --all-targets` exit 0
- Exit condition: AC-6 green; workspace-host crate compiles with both call sites updated.

### Step 6b: wasm-leg contract test + registration

- Task IDs: `TASK-358`
- Objective: author the `region_eligibility` test module proving BOTH legs deliver the derived
  flag for the same constructed region (native vs wasm marshalling); register it in the
  wasm-host contract aggregator; wire `in_.rs` if and only if the FACT read from 6a proved a
  gap. Red-first per invariant 16.
- Precondition: Step 6a landed; its FACT dispatch result available.
- Postcondition: AC-7 command PASS; freshness gate clean.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-wasm-host/tests/contract/main.rs` - module-registration block only
  - `crates/slicer-wasm-host/src/marshal/in_.rs` - `sliced_region_to_data` window
- Files allowed to edit (at most 3):
  - new file `crates/slicer-wasm-host/tests/contract/region_eligibility_tdd.rs`
  - `crates/slicer-wasm-host/tests/contract/main.rs` (one `mod` line)
  - `crates/slicer-wasm-host/src/marshal/in_.rs` (ONLY if the 6a FACT proved a leg gap)
- Files explicitly out of bounds:
  - any WIT file; guest sources; `crates/slicer-macros/src/lib.rs` (escalate to a dispatch
    instead of editing)
- Expected sub-agent dispatches:
  - Question: pick an existing both-legs comparison pattern to mirror; scope:
    `crates/slicer-wasm-host/tests/contract/slice_region_view_contract_tdd.rs`; return:
    `SNIPPETS` ≤30 lines; purpose: honest fixture shape.
- Context cost: `S`
- Authoritative docs:
  - `docs/specs/support-families-anchored-entities-plan.md` - §13 T9 trap text
- OrcaSlicer refs:
  - none
- Verification:
  - AC-7 command PASS; `cargo xtask build-guests --check` exit 0
- Exit condition: both legs carry derived values for the same constructed region in the
  contract test; freshness gate clean.

### Step 7: G-17 consumer gating in commit_support_analysis_builtin

- Task IDs: `TASK-359`
- Objective: suppress auto-detected candidates whose source region derives ineligible without
  enforcer coverage; keep enforcer-derived candidates exempt; keep per-region
  `family_assignments` minting intact for suppressed regions (Ruling 1). Red-first
  integration test `needs_support_false_region_yields_no_auto_candidates` in the runtime
  `integration` aggregator.
- Precondition: Steps 4 and 6 landed (routing union + derivable views).
- Postcondition: AC-8 command PASS; AC-N5 still PASS (manual path unaffected).
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-runtime/src/builtins/support_analysis_producer.rs` - full (compose with
    prior steps' shape)
  - one integration exemplar under `crates/slicer-runtime/tests/integration/` chosen by
    LOCATIONS dispatch (fixture/bootstrap pattern)
- Files allowed to edit (at most 3):
  - `crates/slicer-runtime/src/builtins/support_analysis_producer.rs`
  - `crates/slicer-runtime/tests/integration/main.rs` (module registration if required)
  - new file `crates/slicer-runtime/tests/integration/support_eligibility_signal_tdd.rs`
- Files explicitly out of bounds:
  - planner modules; renderer modules; scheduler
- Expected sub-agent dispatches:
  - Question: minimal whole-run bootstrap pattern used by existing integration tests
    (blackboard construction without full pipeline); scope:
    `crates/slicer-runtime/tests/integration/support_disabled_no_output.rs`; return:
    `SNIPPETS` ≤30 lines; purpose: honest end-to-end decline fixture.
- Context cost: `M`
- Authoritative docs:
  - `docs/specs/support-families-anchored-entities-plan.md` - §6 invariant 15
- OrcaSlicer refs:
  - none
- Verification:
  - AC-8 command PASS; AC-N5 command PASS; AC-1 re-run still PASS (composition check)
- Exit condition: ineligible region yields zero auto candidates AND its structured assignment
  record exists (asserted, not assumed).

### Step 8: Docs impact edits

- Task IDs: `TASK-360`
- Objective: apply the Doc Impact Statement edits from `packet.spec.md`:
  `docs/02_ir_schemas.md` §IR 2 eligibility paragraph (derivation contract) and
  SupportAnalysisIR section (`cantilever_surfaces`, additive schema minor bump).
- Precondition: Steps 3–7 landed.
- Postcondition: every Doc Impact grep returns a match.
- Files allowed to read, with ranges when over 300 lines:
  - `docs/02_ir_schemas.md` - targeted section headers via grep first
- Files allowed to edit (at most 3):
  - `docs/02_ir_schemas.md`
- Files explicitly out of bounds:
  - `docs/07_implementation_status.md` (Step 10), DEVIATION_LOG, config reference (238a)
- Expected sub-agent dispatches:
  - Question: locate exact section anchors to edit; scope: `docs/02_ir_schemas.md`; return:
    `LOCATIONS` ≤6; purpose: bounded ranged read.
- Context cost: `S`
- Authoritative docs:
  - `docs/02_ir_schemas.md` - the sections being edited
- OrcaSlicer refs:
  - none
- Verification:
  - `rg -q 'derive_needs_support' docs/02_ir_schemas.md && rg -q 'cantilever_surfaces' docs/02_ir_schemas.md && echo DOCS-OK`
- Exit condition: DOCS-OK printed.

### Step 9: Packet gates + human-gate artifacts

- Task IDs: `TASK-361`
- Objective: run the packet-level gates (`cargo clippy --workspace --all-targets -- -D
  warnings`, full slicer-core gated suite reconciliation, `cargo xtask build-guests --check`);
  produce the Human Validation Gate artifacts listed in `packet.spec.md` into `tmp/` and draft
  `tmp/237-human-validation.md` evidence (sign-off remains human-pending).
- Precondition: Steps 1–8 complete.
- Postcondition: all gate commands green or their failures attributed per T10; artifacts
  exist at the documented paths.
- Files allowed to read, with ranges when over 300 lines:
  - `target/test-output.log` after runs
- Files allowed to edit (at most 3):
  - `tmp/237-human-validation.md`
  - `tmp/` artifacts generated by commands (G-code/visual-debug outputs)
- Files explicitly out of bounds:
  - any `src/` file (fix-forward only via new step if a gate fails)
- Expected sub-agent dispatches:
  - Question: run `cargo xtask test --summary -p slicer-core -- --no-fail-fast --features
    host-algos` equivalent narrow reconciliation and return binary count; scope: workspace
    test runner; return: `FACT`; purpose: E6 binary-count reconciliation against expectations.
- Context cost: `M`
- Authoritative docs:
  - `AGENTS.md` - Test Discipline section (delegated SUMMARY acceptable)
- OrcaSlicer refs:
  - none
- Verification:
  - `rg -q 'status: draft' docs/spec_packets/237-support-analysis-parity/packet.spec.md && echo STILL-DRAFT`
  - artifact existence checks recorded in the evidence file (E2: existence is bookkeeping,
    the verdicts remain human inspection)
- Exit condition: gates green; evidence file lists all artifact paths and pending sign-off.

### Step 10: docs/07 registration + closure bookkeeping

- Task IDs: `TASK-362`
- Objective: register TASK-353..362 rows in `docs/07_implementation_status.md` via worker
  dispatch using the verbatim rows in `task-map.md`; flip packet status to `implemented` ONLY
  after acceptance ceremony and human sign-off land.
- Precondition: Step 9 gates green; acceptance ceremony re-run of pipe-suffixed ACs done.
- Postcondition: greps in Doc Impact Statement all match; status transition authorized.
- Files allowed to read, with ranges when over 300 lines:
  - `docs/07_implementation_status.md` - tail window only (worker-dispatched)
- Files allowed to edit (at most 3):
  - `docs/07_implementation_status.md` (via worker dispatch)
  - `docs/spec_packets/237-support-analysis-parity/packet.spec.md` (status line only, post-sign-off)
- Files explicitly out of bounds:
  - queue table of the plan file; other packets
- Expected sub-agent dispatches:
  - Question: append ten task rows verbatim from task-map.md; scope: docs/07 tail; return:
    `FACT` appended-count; purpose: registration without full-file read.
- Context cost: `S`
- Authoritative docs:
  - `.agents/doc-index.md` - docs/07 conventions (one-line lookup)
- OrcaSlicer refs:
  - none
- Verification:
  - `rg -q 'TASK-362' docs/07_implementation_status.md && echo REGISTERED`
- Exit condition: REGISTERED printed; packet ready for `status: implemented` once §8 sign-off
  is recorded.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | S | red tests, single file |
| Step 2 | S | bridge stage port |
| Step 3 | M | schema bump + blast radius |
| Step 4 | M | routing union + doc fixes |
| Step 5 | S | producer derivation |
| Step 6a | S | view derivation + native leg + AC-6 test |
| Step 6b | S | wasm-leg contract test + registration |
| Step 7 | M | consumer gating + e2e fixture |
| Step 8 | S | docs edits |
| Step 9 | M | gates + human artifacts |
| Step 10 | S | registration |

Aggregate M; no L steps; split not required.

## Packet Completion Gate

- All steps and exits complete.
- Every pipe-suffixed AC command returns PASS.
- Update `docs/07_implementation_status.md` through a worker dispatch, never a full backlog
  read (Step 10).
- Reconcile reopened/superseded status transitions: stub consumed (already deleted at
  authoring), gap-register row routed (done at authoring).
- `packet.spec.md` is ready for `status: implemented` — but only after the §8 sign-off line
  carries date + verdict.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed AC and packet-level gate command.
- Freshness first: `cargo xtask build-guests --check` exit 0 before any guest-facing claim.
- Broad-suite discipline: the only permitted whole-suite run is
  `cargo xtask test --summary --workspace -- --no-fail-fast` at closure, per AGENTS Test
  Discipline condition 2, after every narrower verification has passed; totals read from
  `target/test-output.log` (E5), never re-run.
- Record remaining packet-local risk (buildplate `[FWD]`, enforced-flag choice, golden-drift
  classification duty).
- Confirm context stayed within the standard band, or record the swarm ESCALATION lesson.

All `cargo check`, `cargo clippy`, and `cargo test` invocations in gate and verification
commands must use `--all-targets` where the gate compiles more than the named test binary.
