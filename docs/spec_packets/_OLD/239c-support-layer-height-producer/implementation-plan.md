# Implementation Plan: 239c-support-layer-height-producer

## Execution Rules

- Work one atomic step at a time; map every step to grouped task IDs.
- Use TDD, then implementation, then the narrowest falsifying validation.
- Every field below is a context-budget contract and must be filled independently; never write
  "see Step 1".
- This packet edits guest-feeding paths in every step from Step 1 onward. Run
  `cargo xtask build-guests --check` and read its **exit code** (0 fresh / 1 stale / 3
  `wasm-tools` infrastructure error) before attributing any failure to your own change. Never
  grep for `STALE:`.
- Both dependency packets (`239a-anchored-host-seams`, `239b-anchored-wit-contract`) must be
  `status: implemented` before Step 1 begins. Their `packet.spec.md` files are the contract
  surface; their `design.md` and `implementation-plan.md` are out of bounds.

## Steps

### Step 1: Declare `independent_support_layer_height` and capture the pre-change baseline

- Task IDs: `TASK-515`
- Objective: add `[config.schema.independent_support_layer_height]` (`type = "bool"`,
  `default = true`) to both `*-support-planner` manifests, prove red-first that an undeclared
  read of this key fails plan construction with
  `ExecutionPlanError::UndeclaredConfigKey`, prove the shipped manifests bind it through
  `bind_module_config_view`, and record the pre-change distinct `;Z:` sequence that AC-N1 will
  compare against.
- Precondition: both dependencies are `implemented`; `rg -c 'independent_support_layer_height'`
  over `modules/` and `crates/` returns zero matches (verified zero at authoring time — if it
  is non-zero, someone else declared the key and this step must be reconciled, not repeated).
- Postcondition: AC-6 and AC-N3 pass; the baseline `;Z:` sequence for the tracked fixture and
  `orca-matched-config.json` exists as a constant in
  `crates/slicer-runtime/tests/integration/support_family_closure.rs`, captured **before** any
  planner behaviour changes.
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/tree-support-planner/tree-support-planner.toml` - whole file
  - `modules/core-modules/traditional-support-planner/traditional-support-planner.toml` - whole
    file
  - `crates/slicer-scheduler/src/execution_plan.rs` - only the module-binding loop containing the
    declared-read guard, and `bind_module_config_view`
  - `crates/slicer-ir/src/slice_ir.rs` - only the `impl ConfigView` block
    (`from_declared`, `get_bool`)
  - `crates/slicer-runtime/tests/contract/config_view_binding_tdd.rs` - whole file (small);
    `bind_module_config_view_hides_undeclared_keys_entirely` is the pattern to follow
  - `crates/slicer-runtime/tests/integration/support_family_closure.rs` - only the helper block
    (`support_test_path`, `matched_config_path`, `matched_config_for`, `run_slice_for_family`)
- Files allowed to edit (**at most 6 — cap raised for this step, justified below**):
  - `modules/core-modules/tree-support-planner/tree-support-planner.toml`
  - `modules/core-modules/traditional-support-planner/traditional-support-planner.toml`
  - `crates/slicer-runtime/tests/contract/config_view_binding_tdd.rs` (AC-N3)
  - `crates/slicer-runtime/tests/executor/support_config_surface_tdd.rs` (AC-6)
  - `crates/slicer-runtime/tests/integration/support_family_closure.rs` (the AC-N1 baseline
    constant, and any `pub fn` check this packet adds to the integration binary)
  - `crates/slicer-runtime/tests/integration/main.rs` — the **wrapper site**. Under the wrapper
    convention this packet commits to (`packet.spec.md` §"Test-naming convention for the
    `mod`-aggregated binaries"), a `pub fn` check in `support_family_closure.rs` is only reachable
    once a `#[test]` wrapper here calls it; without the wrapper the bare AC-1/AC-N1/AC-N2 filters
    match zero tests.

  **Cap justification (blast-radius clause).** The default 3-file cap is raised to 6 because this
  step owns a *declaration radius*: one config key declared identically in two manifests, plus the
  three-binary test surface that proves the declaration is real (contract = rejection of an
  undeclared read, executor = binding on both planners, integration = the pre-change baseline),
  plus the aggregator wrapper that convention makes mandatory rather than optional. The four test
  files are test-only, add no production surface, and touch no `modules/core-modules/*/src/**`
  file — which remains this step's out-of-bounds list and its exit condition. Splitting them out
  would separate a declaration from the tests that falsify it, which is the coupling this cap
  exists to protect, not the churn it exists to prevent.
- Files explicitly out of bounds:
  - `crates/slicer-ir/src/resolved_config.rs` (this is a module key, not a
    `declare_resolved_config!` host key)
  - `docs/config/host-keys.toml`, `crates/slicer-runtime/tests/unit/host_keys_doc_lock_tdd.rs`
  - `docs/15_config_keys_reference.md` (generated; regenerated in Step 8)
  - every `modules/core-modules/*/src/**` file (behaviour lands in Steps 2 and 4)
- Blast-radius discipline: this step adds **no** struct field and **no** schema/version
  constant. It adds two manifest TOML tables. `ConfigBoundsIndex::from_modules`
  (`crates/slicer-scheduler/src/config_resolution.rs`) intersects declared bounds across every
  module declaring a key, so the two tables must be byte-identical in `type` and `default`; a
  mismatch is a silent per-family behaviour split, not a compile error. No struct-literal
  `LOCATIONS` dispatch is required.
- Expected sub-agent dispatches:
  - Question: confirm `independent_support_layer_height` in `init_fff_params`
    (`PrintConfig.cpp`) is `coBool` with default true; scope:
    `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp`; return: `FACT` (type + default)
  - Question: does any file under `crates/` or `modules/` already reference
    `independent_support_layer_height`; scope: `crates/`, `modules/`; return: `LOCATIONS`
    (<= 10 entries)
- Context cost: `M`
- Authoritative docs:
  - `CLAUDE.md` §"Config Key Naming Convention" - direct read; snake_case is mandatory
  - `docs/specs/support-independent-layer-z-split-plan.md` - canonical reference block, direct
    ranged read
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` - `init_fff_params`; delegate, never
    load
- Verification:
  - `cargo xtask build-guests --check && echo FRESH` - FACT exit code
  - `mkdir -p target && cargo test -p slicer-runtime --test contract -- config_view_binding_tdd::undeclared_independent_support_layer_height_fails_plan_build --exact 2>&1 | tee target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` - AC-N3, FACT pass/fail
  - `mkdir -p target && cargo test -p slicer-runtime --test executor -- support_config_surface_tdd::independent_support_layer_height_is_declared_and_bound_on_both_planners --exact 2>&1 | tee target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` - AC-6, FACT pass/fail
  - `cargo check --workspace --all-targets` - FACT pass/fail
- Exit condition: both manifests declare the key with identical `type`/`default`; AC-6 and
  AC-N3 pass; the AC-N1 baseline `;Z:` sequence is committed as a constant with a comment
  naming the commit it was captured at; no `modules/core-modules/*/src/**` file has changed.

### Step 2: Derive support planes in both planners (canonical enabled/disabled semantics)

- Task IDs: `TASK-516`
- Objective: make `SupportPlanEntry.anchor_z` the declared support print plane. Enabled: planes
  are free-floating, produced by the canonical stepping and group-midpoint rules and decoupled
  from `layer_plan.layers[..].z`. Disabled: `anchor_z` equals the object layer's Z exactly,
  reproducing `sync_gap_with_object_layer`.
- Precondition: Step 1's exit condition holds; the `LOCATIONS` dispatch below has returned every
  `anchor_z` assignment site and every `layer_plan.layers[..].z` / `effective_layer_height` read
  site, so neither large planner file is full-read.
- Postcondition: with the key enabled and a support pitch finer than the object layer height, at
  least one `anchor_z` differs from its object layer's Z by more than
  `AnchoredGeometryContract::COORDINATE_TOLERANCE_UNITS`; with the key disabled every `anchor_z`
  is integer-equal to its object layer's Z.
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/tree-support-planner/src/lib.rs` - **very large**; only the bounded
    ranges the `LOCATIONS` dispatch returns, plus `SupportPlanner::from_config`
  - `modules/core-modules/traditional-support-planner/src/lib.rs` - **large**; only its
    `anchor_z` assignment sites and its `from_config`
  - `crates/slicer-ir/src/slice_ir.rs` - only the `SupportPlanEntry` definition
  - `crates/slicer-sdk/src/prepass_builders.rs` - only the `SupportGeometryOutput` `impl` block
  - `crates/slicer-core/src/algos/support_geometry.rs` - only `build_emit_schedule` - purpose:
    understand the existing `support_layer_height_mm` grid decimation this step must not break
- Files allowed to edit (at most 3):
  - `modules/core-modules/tree-support-planner/src/lib.rs`
  - `modules/core-modules/traditional-support-planner/src/lib.rs`
- Files explicitly out of bounds:
  - `crates/slicer-core/src/algos/support_geometry.rs` (read-only; the grid decimation is a
    separate concern and must keep working)
  - both renderer `src/lib.rs` files (Step 4)
  - `crates/slicer-gcode/src/emit.rs` (Steps 5 and 6)
  - `crates/slicer-schema/wit/**` and everything else 239a/239b own
- Blast-radius discipline: this step adds **no** struct field and **no** schema constant. It
  changes the *value* written into an existing `SupportPlanEntry.anchor_z` field. Before
  editing, dispatch a `LOCATIONS` sweep for every reader of `anchor_z` across `crates/` and
  `modules/` — at authoring time the only sites found were the planners' own writers and the
  `crates/slicer-sdk/src/prepass_types.rs` mirror field, but that is a ledger fact and must be
  re-derived. Any reader that assumes `anchor_z == object layer Z` must be listed in this step's
  edit budget or the step must be split.
- Expected sub-agent dispatches:
  - Question: every site assigning `anchor_z` or reading `layer_plan.layers[..].z` /
    `effective_layer_height` in the two planner `src/lib.rs` files; scope: those two files;
    return: `LOCATIONS` (<= 20 entries)
  - Question: every reader of `SupportPlanEntry.anchor_z` outside the two planners; scope:
    `crates/`, `modules/`; return: `LOCATIONS` (<= 20 entries)
  - Question: `bottom_contact_layer`'s enabled vs disabled `print_z` behaviour and the
    `sync_gap_with_object_layer` call; scope:
    `OrcaSlicerDocumented/src/libslic3r/Support/SupportMaterial.cpp`; return: `SUMMARY`
    (<= 200 words)
  - Question: `generate_support_layers`' grouping predicate, midpoint Z, group height, and the
    `n_layers_extra` / `step` / `print_z` stepping; scope:
    `OrcaSlicerDocumented/src/libslic3r/Support/SupportCommon.cpp`; return: `SUMMARY`
    (<= 200 words)
- Context cost: `M`
- Authoritative docs:
  - `docs/08_coordinate_system.md` - delegated SUMMARY of the mm↔unit boundary rules only
  - `docs/specs/support-independent-layer-z-split-plan.md` - F8 and the canonical block, direct
    ranged read
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/Support/SupportMaterial.cpp` -
    `PrintObjectSupportMaterial::bottom_contact_layer`; delegate, never load
  - `OrcaSlicerDocumented/src/libslic3r/Support/SupportCommon.cpp` - `generate_support_layers`;
    delegate, never load
  - `OrcaSlicerDocumented/src/libslic3r/Slicing.cpp` - the flag-FALSE gap rounding; delegate,
    never load
- Verification:
  - `cargo xtask build-guests --check` - exit code must be `0` before any test run in this step
  - `mkdir -p target && cargo test -p tree-support-planner --test tree_family_tdd 2>&1 | tee target/test-output.log && grep -q '^test result: ok' target/test-output.log` - existing tree planner suite still green, FACT pass/fail
  - `mkdir -p target && cargo test -p traditional-support-planner --test traditional_family_tdd 2>&1 | tee target/test-output.log && grep -q '^test result: ok' target/test-output.log` - FACT pass/fail
  - `cargo check --workspace --all-targets` - FACT pass/fail
- Exit condition: both planners read the key in `from_config` and derive `anchor_z` from the
  canonical rules; the disabled branch is byte-identical in behaviour to pre-change; the `[FWD]`
  decision on the `support_layer_height_mm == 0.0` sentinel (see `design.md` §Open Questions) is
  recorded in a code comment naming which of the two options was taken and why.

### Step 3: Red-first enabled/disabled off-grid matrix at module level

- Task IDs: `TASK-517`
- Objective: prove, per family, that off-grid support planes are produced when the key is
  enabled and are **not** produced when it is disabled — AC-2 and AC-3.
- Precondition: Step 2's exit condition holds and a guest freshness check has returned exit `0`.
- Postcondition: AC-2 and AC-3 pass; each test fails if the Step 2 derivation is reverted.
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/tree-support-planner/tests/tree_family_tdd.rs` - whole file, for its
    existing `LayerPlanView` construction helpers
  - `modules/core-modules/traditional-support-planner/tests/traditional_family_tdd.rs` - whole
    file
  - `modules/core-modules/tree-support-planner/src/lib.rs` - only `SupportPlanner::from_config`
    and the Step 2 derivation site
  - `crates/slicer-ir/src/slice_ir.rs` - only the `SupportPlanEntry` definition
- Files allowed to edit (at most 3):
  - `modules/core-modules/tree-support-planner/tests/tree_family_tdd.rs`
  - `modules/core-modules/traditional-support-planner/tests/traditional_family_tdd.rs`
- Files explicitly out of bounds:
  - both planner `src/lib.rs` files (if a test cannot be made to pass without editing them,
    Step 2 was incomplete — go back, do not patch forward)
  - both renderer `src/lib.rs` files
- Blast-radius discipline: not applicable — no struct field, no schema constant. Test-only step.
  Struct literals authored here are subject to the churn gate: use a `..` rest or an
  `// exhaustive: <reason>` waiver for any watched type (`SupportPlanEntry` has 13 named fields
  and is `pub` under `crates/*/src`, so it is watched).
- Expected sub-agent dispatches:
  - Question: which helper in each planner test file constructs a `LayerPlanView` with a chosen
    layer pitch; scope: the two named test files; return: `LOCATIONS` (<= 6 entries)
- Context cost: `S`
- Authoritative docs:
  - `docs/21_data_defaults_and_fixtures.md` - delegated SUMMARY of the struct-literal churn gate
    and the waiver format
- OrcaSlicer refs:
  - none for this step; the canonical semantics were resolved in Step 2 and are asserted here
    against the ported behaviour, not re-derived
- Verification:
  - `cargo xtask build-guests --check` - exit code `0` before the runs below
  - `mkdir -p target && cargo test -p tree-support-planner --test tree_family_tdd -- enabled_independent_height_produces_free_floating_anchor_z --exact 2>&1 | tee target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` - AC-2, FACT pass/fail
  - `mkdir -p target && cargo test -p traditional-support-planner --test traditional_family_tdd -- disabled_independent_height_copies_object_layer_print_z_exactly --exact 2>&1 | tee target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` - AC-3, FACT pass/fail
  - `cargo xtask check-literals` - FACT pass/fail
- Exit condition: AC-2 and AC-3 pass; each was observed red before Step 2's derivation was in
  place (or, if authored after, was verified red by temporarily reverting the derivation and the
  red observation recorded in the step log).

### Step 4: Renderers emit at the declared plane and route off-grid work through the anchored drain

- Task IDs: `TASK-518`
- Objective: replace `let z = region.z();` in both `run_support` implementations with the
  plan-declared plane from the `SupportPlanEntry` the renderer already fetches, and send
  off-grid paths out as an `OrderedEventCollection` through 239b's SDK drain while leaving the
  on-grid `push_support_path` route untouched.
- Precondition: Step 3's exit condition holds; `239b-anchored-wit-contract` is at
  `status: implemented` and its two-builder `layer-support` `run` is present, confirmed by the
  dispatch below and the answer recorded. The design question is already settled (`design.md`
  §Open Questions `[RESOLVED]`): the renderers reach `set_anchored_event_collection` through the
  `collection: &mut LayerCollectionBuilder` parameter their own `LayerModule::run_support` now
  receives. **There are no fallbacks.** A confirmation that returns `no` means 239b has not
  landed (or has regressed) — halt and raise it against 239b; do not substitute a different route
  in this packet.
- Postcondition: AC-4 passes; an off-grid plan entry produces extrusions whose points sit at
  `entry.anchor_z`, carried as an anchored collection whose declared plane equals `anchor_z` and
  whose anchor index equals `entry.anchor_layer_index`.
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/tree-support/src/lib.rs` - the `impl LayerModule for TreeSupport`
    block and `run_support` only
  - `modules/core-modules/traditional-support/src/lib.rs` - the
    `impl LayerModule for TraditionalSupport` block and `run_support` only
  - `crates/slicer-sdk/src/builders.rs` - only the `SupportOutputBuilder` `impl` block
  - `crates/slicer-sdk/src/layer_collection_builder.rs` - only the anchored proposal helpers
    (`set_anchored_event_collection`, `anchored_proposal`), which already exist in that file;
    what 239b adds is wiring them to the WIT drain
  - `docs/spec_packets/239b-anchored-wit-contract/packet.spec.md` - whole file; its `design.md`
    is out of bounds
  - `crates/slicer-sdk/src/traits.rs` - only `PaintRegionLayerView::support_plan_entries_for`
  - `crates/slicer-sdk/src/views.rs` - only `SliceRegionView::z`
- Files allowed to edit (at most 3):
  - `modules/core-modules/tree-support/src/lib.rs`
  - `modules/core-modules/traditional-support/src/lib.rs`
  - `modules/core-modules/tree-support/tests/tree_family_tdd.rs`
- Files explicitly out of bounds:
  - `crates/slicer-sdk/src/layer_collection_builder.rs`, `crates/slicer-schema/wit/**`,
    `crates/slicer-wasm-host/src/dispatch.rs`, `crates/slicer-wasm-host/src/marshal/native.rs`
    - all 239b-owned; read-only here
  - `crates/slicer-runtime/src/layer_executor.rs`, `crates/slicer-runtime/src/pipeline.rs` -
    239a-owned; read-only here
  - both planner `src/lib.rs` files (Step 2 is closed)
- Blast-radius discipline: no struct field and no schema constant is added. The renderers'
  emitted point Z changes value, not type. Before editing, dispatch a `LOCATIONS` sweep for
  tests that assert a support extrusion's Z equals its layer's Z — those are the tests that will
  flip, and every one of them must appear in this step's budget or the step must be split.
- Expected sub-agent dispatches:
  - Question: **confirm** 239b's landed surface — that
    `crates/slicer-schema/wit/deps/layer-support/layer-support.wit`'s `run` takes
    `collection: layer-collection-builder` and `LayerModule::run_support`
    (`crates/slicer-sdk/src/traits.rs`) takes `&mut LayerCollectionBuilder`, making the anchored
    drain reachable from a module whose manifest `stage.id` is `Layer::Support`; scope:
    `crates/slicer-sdk/`, `crates/slicer-schema/wit/`, `crates/slicer-wasm-host/src/dispatch.rs`;
    return: `FACT` (yes/no + the reachable symbol name). **This is an activation precondition
    check, not a design decision — the seam is already resolved.**
  - Question: every test asserting that a support extrusion's Z equals the layer Z; scope:
    `modules/core-modules/`, `crates/slicer-runtime/tests/`; return: `LOCATIONS` (<= 20 entries)
- Context cost: `M`
- Authoritative docs:
  - `docs/05_module_sdk.md` - delegated SUMMARY of what a `Layer::Support` module may emit
  - `docs/03_wit_and_manifest.md` - delegated SUMMARY of the one-stage-per-manifest rule
  - `docs/specs/support-independent-layer-z-split-plan.md` - F6 (the SDK helpers had no drain
    glue and no callers before 239b), direct ranged read
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/GCode.cpp` - `collect_layers_to_print`, for the
    object/support merge semantics the declared plane must be compatible with; delegate, never
    load. 239a owns the implementation; this step only must not contradict it.
- Verification:
  - `cargo xtask build-guests --check` - exit code `0` before the runs below; **rebuild without
    `--check` after editing module sources, or every test below is meaningless**
  - `mkdir -p target && cargo test -p tree-support --test tree_family_tdd -- offgrid_plan_entry_renders_at_declared_anchor_z --exact 2>&1 | tee target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` - AC-4, FACT pass/fail
  - `mkdir -p target && cargo test -p traditional-support 2>&1 | tee target/test-output.log && grep -q '^test result: ok' target/test-output.log` - traditional renderer suite still green, FACT pass/fail
  - `cargo check --workspace --all-targets` - FACT pass/fail
- Exit condition: AC-4 passes; no `region.z()` remains as the Z source for a support extrusion
  in either renderer; the confirmation dispatch's `FACT` (239b's two-builder `run` present, with
  the reachable symbol named) is written into this packet's step log.

### Step 5: Measure the `height_delta` flow behaviour — measurement only, no source edit

- Task IDs: `TASK-519`
- Objective: execute rules 1–3 and observation O-1 of the measure-first protocol
  (`requirements.md` §Measure-First Flow Protocol) and record the verdict plus the numbers in
  `docs/07_implementation_status.md` under `TASK-519`.
- Precondition: Step 4's exit condition holds, so a real off-grid row can be produced. **No
  edit to `crates/slicer-gcode/src/emit.rs` has been made or may be made in this step.**
- Postcondition: `docs/07_implementation_status.md` contains, under `TASK-519`: the verdict word
  (`MISSCALE_FIXED` or `CONSISTENT`), the applied height term, the declared plane delta, and the
  resulting E for the off-grid pass; plus the same three numbers for the immediately following
  object pass (observation O-1). Every number is a measurement, labelled with the command that
  produced it.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-gcode/src/emit.rs` - **read-only, and only the two bounded ranges the
    dispatch returns**: the `height_delta` derivation inside
    `DefaultGCodeEmitter::emit_gcode`'s layer loop, and the volumetric-E line inside its
    per-point loop
  - `crates/slicer-ir/src/slice_ir.rs` - only the `Point3WithWidth` definition (`width`,
    `flow_factor`)
  - `docs/spec_packets/239a-anchored-host-seams/packet.spec.md` - whole file; the
    payload-capturing `GCodeEmitter` test fixture it exports is the measurement harness
  - `docs/07_implementation_status.md` - **tail only**, via a worker dispatch; never a full read
- Files allowed to edit (at most 3):
  - `docs/07_implementation_status.md` (the `TASK-519` record only)
- Files explicitly out of bounds:
  - `crates/slicer-gcode/src/emit.rs` - **read-only in this step; editing it here voids the
    packet's falsifiability contract**
  - `crates/slicer-gcode/tests/**` - the verdict test is Step 6's
  - every `modules/core-modules/**` file
- Blast-radius discipline: not applicable — no struct field, no schema constant, no source edit.
- Expected sub-agent dispatches:
  - Question: for a minimal off-grid case driven through `DefaultGCodeEmitter::emit_gcode`,
    report the height term actually applied to the off-grid pass, that pass's declared plane
    delta (its own Z minus the previous extrusion Z), and the resulting E; then the same three
    for the immediately following object pass; scope: `crates/slicer-gcode/`; return: `FACT`
    (six numbers plus the verdict word derived by rule 2). **Highest-risk dispatch in the
    packet.** A return containing emitter source instead of numbers has failed; re-issue it.
  - Question: append the `TASK-519` record to `docs/07_implementation_status.md`; scope: that
    file; return: `FACT` (the appended row) — dispatch the write, never full-read the backlog
- Context cost: `S`
- Authoritative docs:
  - `docs/specs/support-parity-gap-register.md` - the range around row `G-02` only; its
    "Unverified risk" sentence is what this step converts into a measurement
  - `docs/17_agent_debugging.md` - delegated SUMMARY only, if the measurement needs
    instrumentation
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/GCode.cpp` - `_extrude`, which reads the precomputed
    `path.mm3_per_mm`; delegate, never load
  - `OrcaSlicerDocumented/src/libslic3r/Flow.cpp` - `Flow::mm3_per_mm`, the per-entity baked
    height term; delegate, never load. This is a **comparison target only** and does not
    pre-decide the verdict.
- Verification:
  - `rg -q 'TASK-519' docs/07_implementation_status.md && rg -qE 'MISSCALE_FIXED|CONSISTENT' docs/07_implementation_status.md` - FACT pass/fail
  - `git diff --name-only -- crates/slicer-gcode/src/emit.rs` returns **empty** - FACT pass/fail;
    this is the step's own falsification guard
- Exit condition: the verdict and all six numbers are recorded under `TASK-519`;
  `crates/slicer-gcode/src/emit.rs` is unmodified; no fix/no-fix decision has been acted on yet.

### Step 6: Act on the verdict — conditional emitter fix, or an assert-only lock

- Task IDs: `TASK-520`
- Objective: on `MISSCALE_FIXED`, carry per-entity plane-Z context so an off-grid pass uses its
  declared plane delta while grid passes stay bit-identical. On `CONSISTENT`, make no emitter
  change. In both cases author the AC-5 verdict test naming the recorded branch.
- Precondition: the `TASK-519` record exists in `docs/07_implementation_status.md` and has been
  read. Opening `crates/slicer-gcode/src/emit.rs` for editing before that record exists is
  prohibited. On the `MISSCALE_FIXED` branch, the E-assertion `LOCATIONS` sweep below has
  already returned.
- Postcondition: AC-5 passes and its assertion message contains the literal recorded branch
  name. On `MISSCALE_FIXED`, every pre-existing E assertion still passes **without any tolerance
  having been widened**.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-gcode/src/emit.rs` - only the ranges Step 5's dispatch returned
  - `crates/slicer-gcode/tests/gcode_emit_tdd.rs` - only `emit_e_uses_volumetric_flow_formula`
  - `crates/slicer-gcode/tests/gcode_feedrate_emission_tdd.rs` - only
    `first_layer_volumetric_e_uses_configured_first_layer_height`
  - `docs/07_implementation_status.md` - the `TASK-519` record only, via dispatch
- Files allowed to edit (at most 3):
  - `crates/slicer-gcode/tests/gcode_emit_tdd.rs` (the AC-5 verdict test; both branches)
  - `crates/slicer-gcode/src/emit.rs` (**`MISSCALE_FIXED` branch only**)
- Files explicitly out of bounds:
  - every `modules/core-modules/**` file
  - `crates/slicer-gcode/tests/golden_emit_tdd.rs` - its golden line is hand-built IR, not
    emitter-computed; if it moves, the change was wider than intended and the step must stop
  - any test file whose only relevance is a tolerance that would have to be widened
- Blast-radius discipline: this step changes **behaviour that many tests hard-assert**, which is
  the moral equivalent of a schema-constant bump. Before touching `emit.rs`, dispatch the
  `LOCATIONS` sweep below and add every returned file to the verification list. Authoring-time
  inventory, to be re-derived rather than trusted: `emit_e_uses_volumetric_flow_formula`
  (`crates/slicer-gcode/tests/gcode_emit_tdd.rs`) and
  `first_layer_volumetric_e_uses_configured_first_layer_height`
  (`crates/slicer-gcode/tests/gcode_feedrate_emission_tdd.rs`) are the two strongest binders;
  `purge_volume_within_tolerance` (`crates/slicer-gcode/tests/gcode_toolchange_wrapping.rs`),
  `crates/slicer-runtime/tests/executor/cube_4color_arachne.rs`, and
  `crates/slicer-runtime/tests/e2e/wave_overhang_bridge_fill_e2e_tdd.rs` are volume/flow derived
  and next most fragile. **Never widen a tolerance to make one pass.**
- Expected sub-agent dispatches:
  - Question: every test in the workspace that hard-asserts an emitted E value or a `;HEIGHT:`
    value; scope: `crates/`, `modules/`; return: `LOCATIONS` (file + test fn, <= 25 entries).
    Required on the `MISSCALE_FIXED` branch **before** any edit.
- Context cost: `M`
- Authoritative docs:
  - `docs/02_ir_schemas.md` - the `LayerCollectionIR` section only, to confirm
    `CURRENT_LAYER_COLLECTION_IR_SCHEMA_VERSION` is **not bumped by this packet** and is not
    otherwise disturbed. Compare the doc against the live constant in
    `crates/slicer-ir/src/slice_ir.rs` at that moment; do not check against any literal quoted in
    this packet, which quotes none on purpose
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/Flow.cpp` - `Flow::mm3_per_mm`; delegate, never load
- Verification:
  - `mkdir -p target && cargo test -p slicer-gcode --test gcode_emit_tdd -- offgrid_pass_height_delta_matches_recorded_verdict --exact 2>&1 | tee target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` - AC-5, FACT pass/fail
  - `mkdir -p target && cargo test -p slicer-gcode --test gcode_emit_tdd 2>&1 | tee target/test-output.log && grep -q '^test result: ok' target/test-output.log` - blast radius, FACT pass/fail
  - `mkdir -p target && cargo test -p slicer-gcode --test gcode_feedrate_emission_tdd 2>&1 | tee target/test-output.log && grep -q '^test result: ok' target/test-output.log` - blast radius, FACT pass/fail
  - On the `MISSCALE_FIXED` branch only, additionally every binary the `LOCATIONS` sweep returned
  - `cargo clippy --workspace --all-targets -- -D warnings` - FACT pass/fail
- Exit condition: AC-5 passes and names the recorded branch; on `CONSISTENT`,
  `git diff --stat -- crates/slicer-gcode/src/emit.rs` is empty; on `MISSCALE_FIXED`, every test
  in the swept blast radius passes and a diff review confirms no tolerance was widened and no
  grid-pass value changed.

### Step 7: Freshness gate, human-gate artifacts, and the reference existence gate

- Task IDs: `TASK-521`
- Objective: produce this packet's human-validation evidence — the two family slices, the
  visual-debug bundle, and the written checklist — each preceded by a passing guest freshness
  check, and record the reference existence gate result verbatim.
- Precondition: Steps 1–6 complete; AC-1, AC-N1, and AC-N2 pass. `cargo xtask build-guests
  --check` returns exit `0` immediately before **each** artifact is produced, not once at the
  start.
- Postcondition: `tmp/239c-human-validation.md` exists carrying the six checklist items each
  answered with layer, tap, and verdict; the reference existence gate result recorded verbatim
  as `REFS-PRESENT` or `REFS-ABSENT-GATE-OPEN`; and an unsigned `_date_ _verdict_` sign-off line
  awaiting a human.
- Files allowed to read, with ranges when over 300 lines:
  - `docs/spec_packets/239-support-independent-layer-z/packet.spec.md` - the §Human Validation
    Gate section only; this packet inherits it
  - `docs/specs/support-families-anchored-entities-plan.md` - §7 evidence standard E2, §8 human
    gate, §13 trap T11 only; bounded ranged reads
  - `tmp/support-family-config-tree-matched.json`, `tmp/support-family-config-normal-matched.json`
    - both small; both **VERIFIED present** at authoring time
  - `docs/19_visual_debug.md` - delegated SUMMARY of the request-JSON shape for
    `pnp_cli visual-debug --request ... --output ...`
- Files allowed to edit (at most 3):
  - `tmp/239c-human-validation.md` (new)
  - `tmp/support-family-config-tree-matched.json` (add the new key; do not rewrite the file)
  - `tmp/support-family-config-normal-matched.json` (same)
- Files explicitly out of bounds:
  - `tmp/p239-orca-ref-tree-independent.gcode`, `tmp/p239-orca-ref-normal-independent.gcode` -
    **HUMAN-generated preconditions.** This packet gates on their existence and never creates,
    edits, or synthesizes them.
  - every pre-existing `tmp/` reference sliced with the feature disabled - trap T11; they cannot
    measure this gap and the "205 vs 150" figure derived from them is VOID and must not be
    requoted
  - every `crates/**` and `modules/**` source file
- Blast-radius discipline: not applicable — no struct field, no schema constant. Evidence step.
- Expected sub-agent dispatches:
  - Question: run the two slice commands and the visual-debug bundle and report the produced
    file paths and their `;TYPE:Support` / `;TYPE:Support interface` line counts; scope:
    `tmp/`; return: `FACT` (paths + four counts)
- Context cost: `M`
- Authoritative docs:
  - `docs/19_visual_debug.md` - delegated SUMMARY; bundle layout and `manifest.json` index
  - `docs/specs/support-families-anchored-entities-plan.md` - §8 human gate, bounded read
- OrcaSlicer refs:
  - none. Reference comparison at this gate is **human inspection only**;
    `assert_no_test_reads_orca_gcode`
    (`crates/slicer-runtime/tests/integration/support_family_closure.rs`) forbids encoding it as
    a test.
- Verification:
  - `cargo xtask build-guests --check && echo FRESH` - AC-7, FACT exit code, immediately before
    each artifact
  - `cargo run --bin pnp_cli --release -- slice --model crates/slicer-runtime/tests/fixtures/support-family/SupportTest.stl --config tmp/support-family-config-tree-matched.json --output tmp/p239c-support-indep-tree.gcode --module-dir modules/core-modules` - FACT exit code
  - `cargo run --bin pnp_cli --release -- slice --model crates/slicer-runtime/tests/fixtures/support-family/SupportTest.stl --config tmp/support-family-config-normal-matched.json --output tmp/p239c-support-indep-normal.gcode --module-dir modules/core-modules` - FACT exit code
  - `test -f tmp/p239-orca-ref-tree-independent.gcode && test -f tmp/p239-orca-ref-normal-independent.gcode && echo REFS-PRESENT` - FACT literal; record `REFS-ABSENT-GATE-OPEN` when it does not print
- Exit condition: all artifacts exist, every checklist item is answered in writing with layer,
  tap, and verdict, the existence-gate result is recorded verbatim, and the sign-off line is
  present and **unsigned**. The packet stops here until a human signs.

### Step 8: Reconciliation, docs/07 registration, and `G-02` closure

- Task IDs: `TASK-522`
- Objective: register `TASK-515`..`TASK-522`, regenerate the config-key reference, close
  gap-register row `G-02` against this packet, update the split plan's queue row 3, and
  reconcile the superseded/superseding status transitions.
- Precondition: Steps 1–7 complete. Ledger facts re-derived **at this moment**, not quoted from
  this document.
- Postcondition: every grep in `packet.spec.md` §Doc Impact Statement passes.
- Files allowed to read, with ranges when over 300 lines:
  - `docs/07_implementation_status.md` - **tail only**, via a worker dispatch; never full-read
  - `docs/specs/support-parity-gap-register.md` - the range around row `G-02` only
  - `docs/specs/support-independent-layer-z-split-plan.md` - the queue table only
  - `docs/spec_packets/239c-support-layer-height-producer/task-map.md` - whole file; it is the
    verbatim source for the registration rows
- Files allowed to edit (at most 3):
  - `docs/07_implementation_status.md`
  - `docs/specs/support-parity-gap-register.md`
  - `docs/specs/support-independent-layer-z-split-plan.md`
  Plus the generated `docs/15_config_keys_reference.md`, which is produced by
  `cargo xtask gen-config-docs` and never hand-edited.
- Files explicitly out of bounds:
  - `docs/spec_packets/239a-anchored-host-seams/**` and
    `docs/spec_packets/239b-anchored-wit-contract/**` - other packets own their own closure
  - every `crates/**` and `modules/**` source file
- Blast-radius discipline: not applicable — no struct field, no schema constant. Docs step.
- Expected sub-agent dispatches:
  - Question: re-derive the current `docs/07_implementation_status.md` task high-water mark, the
    next free `G-` row in `docs/specs/support-parity-gap-register.md`, and the next free
    `DEV-###` in `docs/DEVIATION_LOG.md`; scope: those three files; return: `FACT` (three
    values). **This packet quotes none of the three on purpose** — all are mutable shared state,
    the task and `DEV-###` high-water marks both moved during authoring, and the next-free `G-`
    row is additionally **CONTESTED** between reviewers. The dispatch must return what the files
    say now; never reuse a figure read from any packet document.
  - Question: append the `task-map.md` rows to `docs/07_implementation_status.md`; scope: that
    file; return: `FACT` (appended row count)
- Context cost: `S`
- Authoritative docs:
  - `docs/07_implementation_status.md` - registration target; tail-only, dispatched
  - `docs/15_config_keys_reference.md` - regenerated, then grepped; never hand-edited
- OrcaSlicer refs:
  - none
- Verification:
  - `cargo xtask gen-config-docs && rg -q 'independent_support_layer_height' docs/15_config_keys_reference.md` - FACT pass/fail
  - `rg -q 'TASK-515' docs/07_implementation_status.md && rg -q 'TASK-522' docs/07_implementation_status.md && rg -q 'TASK-519' docs/07_implementation_status.md` - FACT pass/fail
  - `rg -q '239c-support-layer-height-producer' docs/specs/support-parity-gap-register.md` - FACT pass/fail
  - `rg -q 'docs/spec_packets/239c-support-layer-height-producer' docs/specs/support-independent-layer-z-split-plan.md` - FACT pass/fail
  - `cargo xtask check-literals` - FACT pass/fail
  - `cargo clippy --workspace --all-targets -- -D warnings` - FACT pass/fail
- Exit condition: all Doc Impact greps pass; `G-02` names this packet as its destination and is
  marked closed; the split plan's queue row 3 carries this packet's directory; the packet is
  ready for `status: implemented` **except** for the unsigned human gate, which remains the
  single outstanding blocker.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | M | two manifests + red-first declared-read guard + AC-N1 baseline capture |
| Step 2 | M | two very large planner files, ranged reads only; canonical enabled/disabled derivation |
| Step 3 | S | test-only enabled/disabled matrix per family |
| Step 4 | M | two renderers + the anchored drain, reached through the `collection` parameter 239b's two-builder `run_support` supplies; no fallback branch |
| Step 5 | S | measurement + record only; zero source edits, guarded by an empty-diff check |
| Step 6 | M | conditional emitter fix with a swept E-assertion blast radius |
| Step 7 | M | freshness-gated artifact production + written human-gate checklist |
| Step 8 | S | registration, doc regeneration, `G-02` closure, queue reconciliation |

Aggregate is `M`; no step is `L`. Split before activation if that stops being true. The former
`L` risk on Step 4 — a fallback requiring a whole new core module — is **gone**: the seam is
resolved in favour of 239b's two-builder `layer-support` `run`, so Step 4 edits the two existing
renderers and nothing else.

## Packet Completion Gate

- All steps and exits complete.
- Every pipe-suffixed AC command returns PASS.
- `cargo xtask build-guests --check` returns exit `0` at closure.
- The Step 5 verdict and its six numbers are recorded under `TASK-519`, and AC-5's test names
  that recorded branch.
- Update `docs/07_implementation_status.md` through a worker dispatch, never a full backlog read.
- Reconcile the superseded/superseding status transitions: at closure,
  `239-support-independent-layer-z` **is transitioned to** `superseded`, naming all three
  successors; this packet carries `supersedes: 239-support-independent-layer-z`. Its current
  status is a ledger fact — check it (`rg -n '^status:' docs/spec_packets/239-support-independent-layer-z/packet.spec.md`)
  rather than assuming; if it is already `superseded`, verify it names all three successors. That
  packet is **out of this packet's edit scope**; the transition is handled by whoever owns 239.
- The Human Validation Gate is signed. **This packet may not reach `status: implemented` with an
  unsigned gate, and the gate cannot be signed while the reference existence gate reports
  `REFS-ABSENT-GATE-OPEN`.**
- `packet.spec.md` is ready for `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed AC and packet-level gate command.
- Run `cargo xtask test --summary --workspace --no-fail-fast` **through a sub-agent** with a
  `FACT pass/fail` return; never absorb the full output. This is the one permitted broad run and
  it goes through the gated `cargo xtask test` entry point so the guest-freshness preflight
  fires.
- Record remaining packet-local risk, including any residual delta recorded at the human gate.
- Confirm context stayed at or below 150k standard, or at/below 300k only with a logged swarm
  ESCALATION; otherwise record a packet-authoring lesson.

All `cargo check`, `cargo clippy`, and `cargo test` invocations in gate and verification commands
must use `--all-targets` so the test, bench, and example targets compile.
