# Implementation Plan: 240a-support-raft-substrate

## Execution Rules

- Work one atomic step at a time; map every step to grouped task IDs
  (`TASK-409`..`TASK-413`, `TASK-533`..`TASK-536` only).
- Use TDD, then implementation, then the narrowest falsifying validation.
- Every field below is a context-budget contract and must be filled
  independently; never write "see Step N".
- Guest-facing steps: run `cargo xtask build-guests --check` and judge by its
  exit code before attributing any failure; a step that edits guest code
  rebuilds guests in-step.
- WIT-editing steps end with `cargo build --tests`.
- All test commands tee to `target/test-output.log`; read results from the file,
  never re-run for more output.
- A new test file under an aggregated `slicer-runtime` binary gets its
  `mod` registration in the SAME step, or it compiles to zero tests and reports
  a false pass.

## Steps

### Step 1: Author raft-marker + carrier IR tests (red)

- Task IDs: `TASK-409`
- Objective: create the failing tests pinning AC-1 and AC-6's Rust half —
  `crates/slicer-ir/tests/raft_band_ir_tdd.rs` (holding
  `is_raft_defaults_false_and_survives_roundtrip`: `GlobalLayer.is_raft`
  defaults `false`, a `LayerPlanIR` whose first two layers carry
  `is_raft: true` round-trips, and a serde fixture WITHOUT the field still
  loads) and `crates/slicer-ir/tests/sliced_region_raft_fill_tdd.rs` (holding
  `raft_fill_defaults_empty_and_survives_roundtrip`: `raft_fill` defaults
  empty, serde-default backward compat against a fixture carrying the
  CURRENT pre-bump schema version, round-trip stability).
- Precondition: clean tree; 236 confirmed `implemented` (re-derive with
  `grep '^status:' docs/spec_packets/236-support-stabilization/packet.spec.md`).
- Postcondition: both files exist and fail to compile for exactly the intended
  reasons (missing `is_raft`, missing `raft_fill`).
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-ir/src/slice_ir.rs` - symbol-scoped ranges only, located at
    read time via `rg -n 'pub struct (GlobalLayer|SlicedRegion|LayerPlanIR)'`
    and `rg -n 'CURRENT_SLICE_IR_SCHEMA_VERSION'`
- Files allowed to edit (at most 3):
  - `crates/slicer-ir/tests/raft_band_ir_tdd.rs` (new)
  - `crates/slicer-ir/tests/sliced_region_raft_fill_tdd.rs` (new)
- Files explicitly out of bounds: everything else under `crates/`, all modules
- Expected sub-agent dispatches: none
- Context cost: `S`
- Authoritative docs: `docs/specs/support-families-anchored-entities-plan.md` section 12
- OrcaSlicer refs: none this step
- Verification:
  - `mkdir -p target && cargo test -p slicer-ir --test raft_band_ir_tdd 2>&1 | tee target/test-output.log; grep -q 'error\[' target/test-output.log` - FACT: RED confirmed
  - `mkdir -p target && cargo test -p slicer-ir --test sliced_region_raft_fill_tdd 2>&1 | tee target/test-output.log; grep -q 'error\[' target/test-output.log` - FACT: RED confirmed
- Exit condition: both logs show the new tests red for the missing fields and
  nothing else broken.
- Note: `crates/slicer-ir/tests/sliced_region_raft_fill_tdd.rs` already exists
  on disk (untracked) from a prior session and is already RED for the right
  reason; verify its content against this objective and keep or rewrite it.
  The withdrawn revision's `signed_layer_indices_tdd.rs` was already deleted
  during the re-spec — confirm its absence rather than expecting to remove it.
  Also present and NOT owned by this packet:
  `crates/slicer-sdk/tests/should_emit_raft_fill_claim_tdd.rs`, which pins the
  already-shipped `claim:raft-fill` mapping. Leave it alone; 240b's AC-2 uses it.

### Step 2: `GlobalLayer.is_raft` + WIT marker + both harvest legs

- Task IDs: `TASK-410`, `TASK-533`
- Objective: add `pub is_raft: bool` with `#[serde(default)]` to `GlobalLayer`;
  add `is-raft-prefix: bool` to `layer-proposal`
  (`crates/slicer-schema/wit/deps/prepass-layer-planning/prepass-layer-planning.wit`);
  in `harvest_layer_plan_ir_from` (`crates/slicer-wasm-host/src/marshal/in_.rs`)
  and the `PrePass::LayerPlanning` arm of
  `crates/slicer-wasm-host/src/marshal/native.rs`, copy the flag onto each
  `GlobalLayer` leaving index assignment untouched, and reject a raft-marked
  run that is not contiguous at the front with a typed error naming the
  offending push position. Author both tests in a `#[cfg(test)] mod tests`
  INSIDE `crates/slicer-wasm-host/src/marshal/in_.rs`:
  `raft_marker_identical_on_both_legs` and `noncontiguous_raft_band_rejected`.
  **They cannot live in a `tests/*.rs` file.** `harvest_layer_plan_ir_from` is
  `pub(crate)`, and `tests/` is a separate crate, so a top-level test file
  cannot name it. The native leg
  (`commit_native_prepass_response_with_inputs`,
  `crates/slicer-wasm-host/src/marshal/native.rs`) is `pub` and reachable from
  the same in-crate module, which is what makes the both-legs comparison
  possible at all. Run them with `--lib` and the FULL module path
  (`marshal::in_::tests::<name>`) — `--exact` matches a unit test's complete
  path, so a bare name matches nothing (measured; see Step 4's note).
- Precondition: Step 1 red.
- Postcondition: AC-1, AC-2 and AC-N1 green; `cargo build --tests` green after
  the WIT edit; guests rebuilt.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-wasm-host/src/marshal/in_.rs` - the `harvest_layer_plan_ir_from` range
  - `crates/slicer-wasm-host/src/marshal/native.rs` - the `PrePass::LayerPlanning` arm
- Files allowed to edit (6 primaries; the WIT edit forces every one of them, so
  the cap is a review checkpoint rather than a blank cheque — enumerate any
  literal-gate fallout in the working notes before editing):
  - `crates/slicer-ir/src/slice_ir.rs`
  - `crates/slicer-wasm-host/src/marshal/in_.rs`
  - `crates/slicer-wasm-host/src/marshal/native.rs`
  - `crates/slicer-schema/wit/deps/prepass-layer-planning/prepass-layer-planning.wit`
  - `crates/slicer-sdk/src/prepass_types.rs` — **mandatory.** The native leg
    iterates `output.layers()` yielding the SDK `LayerProposal`, whose only
    fields today are `z` and `active_regions`. Without mirroring the flag here
    the native leg has nothing to read and AC-2's "identical `(index, is_raft)`
    pairs on both legs" is impossible.
  - `crates/slicer-macros/src/lib.rs` — **mandatory.** It constructs the WIT
    `LayerProposal { z, active_regions }` in the layer-planning glue emitter;
    adding `is-raft-prefix` to the record breaks that literal, so this step's
    own `cargo build --tests` postcondition fails without it.
  - (tests go in `crates/slicer-wasm-host/src/marshal/in_.rs`'s new
    `#[cfg(test)] mod tests`, already listed as a primary above — no new file)
  - Convention note: `crates/slicer-wasm-host/src/marshal/in_.rs` ALREADY has a
    `#[cfg(all(test, not(target_arch = "wasm32")))] mod tests` that imports
    `harvest_layer_plan_ir_from`; add the two new cases to it rather than
    creating a module. Confirm the live path with
    `cargo test -p slicer-wasm-host --lib -- --list | rg marshal::in_` before
    writing the AC filter. The packet's OTHER new wasm-host test
    (`tests/raft_plan_read_accessor_tdd.rs`, Step 7) stays a loose top-level
    binary because it drives a guest end-to-end through public entry points.
- Files explicitly out of bounds: `modules/**` EXCEPT the single
  `LayerProposal { .. }` literal in
  `modules/core-modules/layer-planner-default/src/lib.rs` (4 sites), which this
  step MUST update to `is_raft: false` because its own postcondition is a green
  `cargo build --tests` and that literal is exhaustive. The raft-band EMISSION
  logic stays Step 3's. Also out of bounds: `ir-types.wit` (Steps 6-7).
- **`LayerProposal` literal fan-out (mandatory, pre-baked).** Adding
  `is-raft-prefix` to the WIT record and `is_raft` to the SDK mirror breaks every
  exhaustive literal of BOTH types. Neither type has a `Default`, and the
  wit-bindgen-generated one has no FRU escape, so the struct-literal churn gate
  never forced `..` on them (they carry only 2 fields, below the >=5 watchlist
  threshold). Counted on disk 2026-09-04 — re-derive before editing:
  - `crates/slicer-wasm-host/tests/contract/prepass_output_builder_validation_tdd.rs` (11 WIT literals)
  - `crates/slicer-sdk/tests/prepass_module_tdd.rs` (10 SDK literals)
  - `crates/slicer-runtime/tests/contract/native_dispatch_parity_seam_tdd.rs` (2 SDK literals)
  - `modules/core-modules/layer-planner-default/src/lib.rs` (4 literals)
  All four are owned by THIS step. `GlobalLayer` is separately a watched type:
  production literals stay exhaustive; test literals gain a `..` rest or an
  `// exhaustive: <reason>` waiver per `docs/21_data_defaults_and_fixtures.md`.
  Enumerate both `GlobalLayer {` and `LayerProposal {` sites in the working
  notes before the first edit.
- Expected sub-agent dispatches:
  - LOCATIONS: every `GlobalLayer {` AND every `LayerProposal {` struct literal
    (both the WIT-generated and SDK types); scope `crates/ modules/`; return
    LOCATIONS with per-file counts; purpose: literal fan-out + literal-gate
    fallout. A miss here fails this step's own `cargo build --tests`.
  - OrcaSlicer SUMMARY: `new_layers` (`PrintObjectSlice.cpp`) raft id offset;
    purpose: confirm band semantics
- Context cost: `M`
- Authoritative docs: `docs/03_wit_and_manifest.md` - delegated SUMMARY
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/PrintObjectSlice.cpp` - delegate; never load
- Verification:
  - `cargo build --tests 2>&1 | tail -3` - FACT pass/fail
  - `cargo xtask build-guests && cargo xtask build-guests --check; echo EXIT:$?` - FACT exit 0
  - `mkdir -p target && cargo test -p slicer-ir --test raft_band_ir_tdd -- is_raft_defaults_false_and_survives_roundtrip --exact --nocapture 2>&1 | tee target/test-output.log; test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` - AC-1
  - `mkdir -p target && cargo test -p slicer-wasm-host --lib -- marshal::in_::tests::raft_marker_identical_on_both_legs --exact --nocapture 2>&1 | tee target/test-output.log; test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` - AC-2
  - `mkdir -p target && cargo test -p slicer-wasm-host --lib -- marshal::in_::tests::noncontiguous_raft_band_rejected --exact --nocapture 2>&1 | tee target/test-output.log; test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` - AC-N1
  - `cargo xtask check-literals` - FACT exit 0
- Exit condition: AC-1, AC-2, AC-N1 green with non-zero counts; guests fresh
  (exit 0); literal gate exit 0; both legs assign identical `(index, is_raft)`
  pairs for the same push sequence.

### Step 3: layer-planner-default emits the raft band

- Task IDs: `TASK-534`
- Objective: teach `com.core.layer-planner-default` to read
  `support_raft_layers` (declared in its manifest `[config.schema]` so E9's
  silent-default trap cannot fire) and push exactly that many
  `is-raft-prefix: true` proposals before any model proposal, with Z computed
  in `f64` and cast once at the end, and at least one `active_regions` entry
  per raft layer. Author the two integration cases plus the monotonic-gate
  case, registering their `mod` line.
- Precondition: Step 2 green.
- Postcondition: AC-3 and AC-N2 green; guests rebuilt.
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/layer-planner-default/src/lib.rs` - the
    `generate_object_layers` and `DefaultLayerPlanner::from_config` ranges
    (there is no `LayerPlannerConfig` type; the config is read by
    `DefaultLayerPlanner`'s `from_config`, a `PrepassModule` trait-method impl)
  - `crates/slicer-runtime/tests/integration/main.rs` - registration list only
- Files allowed to edit (at most 3 primaries plus the new/registered test files):
  - `modules/core-modules/layer-planner-default/src/lib.rs`
  - `modules/core-modules/layer-planner-default/layer-planner-default.toml`
  - `crates/slicer-runtime/tests/integration/raft_band.rs` (new; holds
    `raft_band_emitted_before_model_layers`,
    `no_raft_band_when_raft_layers_zero`,
    `raft_band_satisfies_finalization_monotonic_gate`)
  - `crates/slicer-runtime/tests/integration/main.rs` (add `mod raft_band;`)
- Files explicitly out of bounds: the support planners,
  `modules/core-modules/raft-default/**`
- Expected sub-agent dispatches:
  - SNIPPETS ≤10 lines: what does `derive_layer_output_envelope_from_input`
    return for a layer with empty `active_regions`? scope
    `crates/slicer-wasm-host/src/dispatch.rs` — grounds the seeding choice
  - OrcaSlicer SUMMARY: `generate_object_layers` (`Slicing.cpp`) `coordf_t`
    discipline
- Context cost: `M`
- Authoritative docs: `docs/08_coordinate_system.md` - delegated SUMMARY
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/Slicing.cpp` - delegate; never load
- Verification:
  - `cargo xtask build-guests && cargo xtask build-guests --check; echo EXIT:$?` - FACT exit 0
  - `rg -q 'config\.schema\.support_raft_layers' modules/core-modules/layer-planner-default/layer-planner-default.toml` - FACT: key declared
  - `mkdir -p target && cargo test -p slicer-runtime --test integration -- raft_band::raft_band_emitted_before_model_layers --exact --nocapture 2>&1 | tee target/test-output.log; test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` - AC-3
  - `mkdir -p target && cargo test -p slicer-runtime --test integration -- raft_band::no_raft_band_when_raft_layers_zero --exact --nocapture 2>&1 | tee target/test-output.log; test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` - AC-3 zero case
  - `mkdir -p target && cargo test -p slicer-runtime --test integration -- raft_band::raft_band_satisfies_finalization_monotonic_gate --exact --nocapture 2>&1 | tee target/test-output.log; test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` - AC-N2
  - `grep -q 'mod raft_band;' crates/slicer-runtime/tests/integration/main.rs` - FACT registration present
- Exit condition: AC-3 and AC-N2 green; manifest key declared; registration
  grep passes; guests fresh.

### Step 4: Object-bottom predicate audit + positional regression guards

- Task IDs: `TASK-411`, `TASK-413`
- Objective: apply `design.md` section "First-Model-Layer Audit" verbatim —
  convert exactly the three sites ruled "Convert" (the sharp-tail gate and the
  `enforce_support_layers` window in `detect_support_contacts`
  (`crates/slicer-core/src/algos/overhang_annotation.rs`), and the
  `top_bottom_infill_wall_overlap` selection in `run_perimeters`
  (`modules/core-modules/classic-perimeters/src/lib.rs`)) to resolve the
  boundary from `support_raft_layers`, following the shipped DEV-124 template
  in `run_perimeters`' own wall clamp.
  **Config reach is mandatory and is the real work here.** The two
  `detect_support_contacts` sites read only `SupportContactParams`, which has no
  raft field; its sole `ResolvedConfig` bridge is `resolve_contact_params`
  (`crates/slicer-runtime/src/builtins/support_analysis_producer.rs`), which
  today hardcodes `enforce_support_layers: 0` and `layer_id: 0` and has zero
  `raft` references. Add the raft-boundary field to `SupportContactParams` AND
  populate it from `support_raft_layers` in `resolve_contact_params`, or the
  conversion rides at its default, compiles cleanly, passes a naive test, and
  does nothing. Author `resolve_contact_params_carries_raft_boundary_from_config`
  asserting a config with `support_raft_layers = 2` yields params carrying `2`.
  **It goes in `support_analysis_producer.rs`'s own `#[cfg(test)] mod tests`**,
  beside the existing
  `resolve_contact_params_uses_typed_threshold_overlap_percent_and_literal`, and
  runs under `cargo test -p slicer-runtime --lib`. It CANNOT live in the
  `contract` binary: `resolve_contact_params` is a private `fn` and `tests/` is a
  separate crate, so a `--test contract` home would not compile.
  **Filter by the FULL module path**
  (`builtins::support_analysis_producer::tests::<name>`): with `--lib`, `--exact`
  matches a unit test's complete path, so a bare name silently matches nothing
  and the run prints `running 0 tests` / `100 filtered out`. Measured 2026-09-04
  against the existing sibling test: bare name -> `0 tests`; path-qualified ->
  `1 passed`. The AC's non-zero-count guard turns that into a loud failure
  rather than a false pass, but the command must be path-qualified to ever go
  green. This trap applies only to `--lib` unit tests; tests in `tests/*` binaries
  sit at their crate root, so bare names are correct there.
  Note `enforce_support_layers` is hardcoded `0`, so the enforce-window
  predicate is always false today; assert the shifted arithmetic directly rather
  than looking for a behavioural delta. LEAVE every site the table rules "leave
  alone". Author `object_bottom_predicates_are_raft_aware` plus the AC-4
  positional guard and the AC-N3 empty-slice case, registering both `mod` lines.
- Precondition: Step 3 green.
- Postcondition: AC-4, AC-5, AC-N3, AC-N4 green.
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/arachne-perimeters/src/lib.rs` - the `is_bottom_layer`
    range ONLY, as the DEV-124 template; not edited
  - `crates/slicer-runtime/src/layer_executor.rs` - the `hydrate_slice_arena`
    range only
  - `crates/slicer-runtime/tests/{executor,contract}/main.rs` - registration lists only
- Files allowed to edit (3 primaries plus the new/registered test files; the
  third is mandatory, not optional — see Objective):
  - `crates/slicer-core/src/algos/overhang_annotation.rs`
  - `crates/slicer-runtime/src/builtins/support_analysis_producer.rs`
  - `modules/core-modules/classic-perimeters/src/lib.rs`
  - `crates/slicer-runtime/tests/contract/raft_object_bottom_tdd.rs` (new)
  - `crates/slicer-runtime/tests/contract/main.rs` (add `mod raft_object_bottom_tdd;`)
  - `crates/slicer-runtime/tests/executor/raft_positional_tdd.rs` (new; holds
    `raft_layer_index_equals_vec_position` and
    `raft_layer_below_geometry_slices_empty_not_fatal`)
  - `crates/slicer-runtime/tests/executor/main.rs` (add `mod raft_positional_tdd;`)
- Files explicitly out of bounds:
  `crates/slicer-runtime/tests/contract/only_one_wall_first_layer_tdd.rs`
  (AC-N4 asserts `git diff --quiet` on it), the DEV-124 wall clamps in both
  perimeter generators, every "leave alone" site in the audit table
- Expected sub-agent dispatches:
  - FACT: confirm `params.layer_id` in `detect_support_contacts` is fed the
    GLOBAL layer index; scope
    `crates/slicer-runtime/src/builtins/support_analysis_producer.rs`;
    return SNIPPETS ≤10 lines
- Context cost: `M`
- Authoritative docs: `docs/DEVIATION_LOG.md` - the DEV-124 row only
- OrcaSlicer refs: none this step
- Verification:
  - `mkdir -p target && cargo test -p slicer-runtime --test contract -- raft_object_bottom_tdd::object_bottom_predicates_are_raft_aware --exact --nocapture 2>&1 | tee target/ac5-a.log && test "$(grep -c '^test .* ok$' target/ac5-a.log)" -gt 0` - AC-5 predicates
  - `mkdir -p target && cargo test -p slicer-runtime --lib -- builtins::support_analysis_producer::tests::resolve_contact_params_carries_raft_boundary_from_config --exact --nocapture 2>&1 | tee target/ac5-b.log && test "$(grep -c '^test .* ok$' target/ac5-b.log)" -gt 0` - AC-5 config reach (`--lib`, NOT `--test contract`: the fn is private)
  - `git diff --quiet -- modules/core-modules/arachne-perimeters/src/lib.rs modules/core-modules/rectilinear-infill/src/lib.rs modules/core-modules/wave-overhangs/src/lib.rs modules/core-modules/overhang-classifier-default/src/lib.rs modules/core-modules/part-cooling/src/lib.rs modules/core-modules/tree-support-planner/src/lib.rs crates/slicer-core/src/algos/lightning/generator.rs` - FACT: every "leave alone" site untouched
  - `mkdir -p target && cargo test -p slicer-runtime --test executor -- raft_positional_tdd::raft_layer_index_equals_vec_position --exact --nocapture 2>&1 | tee target/test-output.log; test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` - AC-4
  - `mkdir -p target && cargo test -p slicer-runtime --test executor -- raft_positional_tdd::raft_layer_below_geometry_slices_empty_not_fatal --exact --nocapture 2>&1 | tee target/test-output.log; test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` - AC-N3
  - `mkdir -p target && cargo test -p slicer-runtime --test contract -- only_one_wall_first_layer_tdd::classic_clamp_follows_raft_layers_not_layer_zero --exact --nocapture 2>&1 | tee target/test-output.log; test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0 && git diff --quiet -- crates/slicer-runtime/tests/contract/only_one_wall_first_layer_tdd.rs` - AC-N4
  - `cargo xtask build-guests && cargo xtask build-guests --check; echo EXIT:$?` - FACT exit 0
- Exit condition: AC-4, AC-5 (BOTH halves), AC-N3, AC-N4 green; the DEV-124
  pinning file is unmodified in `git diff`; every "leave alone" site is
  unchanged per the `git diff --quiet` guard above; `rg -c raft crates/slicer-runtime/src/builtins/support_analysis_producer.rs`
  is non-zero (it is 0 before this step — a zero here means the boundary never
  reached `SupportContactParams` and AC-5 passed vacuously).

### Step 5: `SupportPlanEntry` doc-comment correction

- Task IDs: `TASK-412`
- Objective: remove every on-disk promise of a negative raft band. There are
  THREE, all verified present; fixing only one leaves the contradiction live:
  (a) the STRUCT-level doc block immediately above `pub struct SupportPlanEntry`
  (`crates/slicer-ir/src/slice_ir.rs`), which reads "raft entries carry negative
  indices (`-1, -2, ..., -raft_layers`)";
  (b) the FIELD-level doc on `SupportPlanEntry.global_layer_index` in the same
  file, "Negative values (`-1`, `-2`, ...) are reserved for raft prefix layers.";
  (c) the header comment in `crates/slicer-schema/wit/deps/ir-types.wit`,
  "Signed because raft entries committed by `PrePass::SupportGeometry` carry
  negative `global_layer_index`."
  Restate all three: the field remains `i32` for historical reasons, all
  produced values are non-negative, raft layers occupy
  `0..support_raft_layers-1`, and model layers start at `support_raft_layers`.
  Do NOT change the field type.
- Precondition: Step 4 green.
- Postcondition: no doc comment in `crates/slicer-ir/src/slice_ir.rs` promises
  a negative raft band.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-ir/src/slice_ir.rs` - the `SupportPlanEntry` range only,
    located at read time via `rg -n 'pub struct SupportPlanEntry'`
- Files allowed to edit (at most 3):
  - `crates/slicer-ir/src/slice_ir.rs`
  - `crates/slicer-schema/wit/deps/ir-types.wit` (comment only; no type change)
- Files explicitly out of bounds: every other crate; all modules
- Expected sub-agent dispatches: none
- Context cost: `S`
- Authoritative docs: none beyond `design.md`
- OrcaSlicer refs: none this step
- Verification:
  - `! rg -q 'reserved for raft prefix layers|carry negative indices' crates/slicer-ir/src/slice_ir.rs` - FACT: both slice_ir promises gone
  - `! rg -q 'carry negative' crates/slicer-schema/wit/deps/ir-types.wit` - FACT: WIT header comment corrected
  - `cargo check -p slicer-ir --all-targets 2>&1 | tail -3` - FACT pass/fail
- Exit condition: all THREE negative-band promises are absent; `slicer-ir`
  checks green. A grep that finds only one of the three is not a pass.

### Step 6: `SlicedRegion.raft_fill` carrier + WIT accessors + schema bump

- Task IDs: `TASK-535`
- Objective: add `pub raft_fill: Vec<ExPolygon>` with `#[serde(default)]` to
  `SlicedRegion`; add the `raft-fill` accessor to BOTH region resources in
  `crates/slicer-schema/wit/deps/ir-types.wit`; add
  `split_field!(raft_fill);` in `crates/slicer-runtime/src/region_partition.rs`
  beside `split_field!(internal_bridge_areas);`; project the field through the
  host accessor impls, the macro marshal legs, the SDK views and both fixture
  builders, the visual-debug render and the pnp-cli manifest emission;
  minor-bump `CURRENT_SLICE_IR_SCHEMA_VERSION` to the next MINOR above its live
  value (re-derived from `crates/slicer-ir/src/slice_ir.rs` at the moment of
  the edit — do not hardcode a literal from this plan) with a version-history
  doc-comment line naming BOTH `is_raft` and `raft_fill`, and update every test
  asserting the old value in the same step. Follow `design.md` section
  "`raft_fill` Carrier Footprint" as the site checklist.
- Precondition: Step 5 green.
- Postcondition: AC-6 green; `cargo build --tests` green after the WIT edit.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-ir/src/slice_ir.rs` - the `SlicedRegion` and
    `CURRENT_SLICE_IR_SCHEMA_VERSION` ranges, located at read time
  - `crates/slicer-runtime/src/region_partition.rs` - the `split_field!` block
- Files allowed to edit (at most 3 primaries; the remaining footprint sites
  from `design.md` are owned here and enumerated in the working notes before
  the first edit):
  - `crates/slicer-ir/src/slice_ir.rs`
  - `crates/slicer-schema/wit/deps/ir-types.wit`
  - `crates/slicer-runtime/src/region_partition.rs`
- Files explicitly out of bounds: `modules/**`, the scheduler
- Blast-radius discipline: `SlicedRegion` is a watched type. Grep
  `CURRENT_SLICE_IR_SCHEMA_VERSION` assertion sites and update every one in
  THIS step (bump plus fallout together). Production literals stay exhaustive;
  test literals need `..` or a waiver.
- Expected sub-agent dispatches:
  - LOCATIONS: `CURRENT_SLICE_IR_SCHEMA_VERSION` assertion sites plus
    `SlicedRegion {` literals; scope `crates/`; return LOCATIONS ≤20
- Context cost: `M`
- Authoritative docs: `docs/02_ir_schemas.md` - SliceIR section; delegated
  SUMMARY before editing
- OrcaSlicer refs: none this step
- Verification:
  - `cargo build --tests 2>&1 | tail -3` - FACT pass/fail
  - `mkdir -p target && cargo test -p slicer-ir --test sliced_region_raft_fill_tdd -- raft_fill_defaults_empty_and_survives_roundtrip --exact --nocapture 2>&1 | tee target/test-output.log; test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` - AC-6 Rust half
  - `test "$(rg -c 'raft-fill: func' crates/slicer-schema/wit/deps/ir-types.wit)" -eq 2 && rg -q 'split_field..raft_fill' crates/slicer-runtime/src/region_partition.rs` - FACT: both resources + region split
  - `cargo xtask check-literals` - FACT exit 0
- Exit condition: AC-6 green; exactly two WIT accessors; the split line present;
  no test still asserts the pre-bump version; literal gate exit 0.

### Step 7: `raft_plan` read accessor

- Task IDs: `TASK-536`
- Objective: declare a `raft-plan-view` record in
  `crates/slicer-schema/wit/deps/ir-types.wit` (local mirror of `RaftPlan`'s
  four verified fields — `raft-layers: u32`, `raft-first-layer-density: f32`,
  `base-raft-layers: u32`, `interface-raft-layers: u32` — no cross-world
  import) and TWO accessors on `paint-region-layer-view`:
  `raft-plan: func() -> option<raft-plan-view>` and `is-raft: func() -> bool`.
  **`is-raft` is mandatory** — 240b's `com.core.raft-default` is a
  `Layer::Infill` guest with no other way to identify a raft layer; omitting it
  ships a declared read with no WIT accessor. Add
  `PaintRegionLayerData.{raft_plan, is_raft}` in
  `crates/slicer-wasm-host/src/host.rs`, the `raft_plan` accessor pushing
  `"SupportPlanIR"` and the `is_raft` accessor pushing `"LayerPlanIR"` to
  `runtime_reads`; populate both in `build_paint_layer_data_with_plan`
  (`crates/slicer-wasm-host/src/dispatch.rs`) with no layer filter for
  `raft_plan`; mirror both in the `slicer-macros` guest shim; add
  `PaintRegionLayerView::raft_plan()` and `::is_raft()` in
  `crates/slicer-sdk/src/traits.rs`. Move the two existing
  `PaintRegionLayerData` struct literals to FRU or `Default`. Author
  `crates/slicer-wasm-host/tests/raft_plan_read_accessor_tdd.rs` holding
  `raft_plan_reaches_layer_infill_guest`, `is_raft_reaches_layer_infill_guest`,
  and `is_raft_set_on_native_leg` (the native-leg guard — without it native
  `is_raft()` returns `false` forever and the wasm-only tests still pass).
- Precondition: Step 6 green.
- Postcondition: AC-7 green (both halves, driven by a guest that actually calls
  the new accessors); `cargo build --tests` green; guests rebuilt.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-wasm-host/src/host.rs` - the `support_plan_entries` impl range
  - `crates/slicer-wasm-host/src/dispatch.rs` - the
    `build_paint_layer_data_with_plan` range
  - `crates/slicer-schema/wit/deps/layer-infill/layer-infill.wit` - 20 lines, full read
- Files allowed to edit (5 primaries plus the new test file — the WIT surface,
  its host impls, the dispatch population, the macro shim and the SDK getter
  are one indivisible transport change; each is named explicitly so the set is
  a review checkpoint rather than open-ended):
  - `crates/slicer-schema/wit/deps/ir-types.wit`
  - `crates/slicer-wasm-host/src/host.rs`
  - `crates/slicer-wasm-host/src/dispatch.rs`
  - `crates/slicer-sdk/src/traits.rs`
  - `crates/slicer-macros/src/lib.rs`
  - `crates/slicer-wasm-host/src/marshal/native.rs` — **mandatory.** The native
    `PaintRegionLayerView` is CONSTRUCTED here (`PaintRegionLayerView::new` /
    `::with_paint_regions`, then `.with_support_plan(...)`). `raft_plan()` is
    derivable from the `Arc<SupportPlanIR>` the view already holds, but
    `is_raft` is NOT — it comes from `GlobalLayer.is_raft` and must be set at
    construction. Without this edit native `is_raft()` compiles and silently
    returns `false` forever.
  - `crates/slicer-wasm-host/test-guests/layer-infill-guest/src/lib.rs` —
    **mandatory.** Adding accessors to `paint-region-layer-view` regenerates
    bindings but nothing invokes them; no test-guest calls any paint read
    accessor for raft today (`sdk-layer-infill-guest` binds paint as `_paint`).
    Follow the shipped precedent in this same guest, which calls
    `paint.lightning_tree_segments(...)` and is asserted by
    `lightning_infill_guest_calls_lightning_tree_segments`
    (`crates/slicer-wasm-host/tests/contract/`). Without this edit both AC-7
    halves can only pass vacuously. **Rebuild guests in-step** after editing.
  - `crates/slicer-wasm-host/tests/raft_plan_read_accessor_tdd.rs` (new;
    auto-discovered, no registration needed)
- Files explicitly out of bounds: `modules/**`, `region_partition.rs`
- Expected sub-agent dispatches:
  - FACT: does `ir-types.wit` resolve with a locally-declared `raft-plan-view`
    record (no cross-world import / world-satisfaction failure)? scope
    `crates/slicer-schema/wit/`; return FACT
  - LOCATIONS: every `PaintRegionLayerData` mention; scope
    `crates/slicer-wasm-host/src`; return LOCATIONS ≤20 — distinguishes the two
    literals from the functions that merely return the type
- Context cost: `M`
- Authoritative docs: `docs/03_wit_and_manifest.md` - delegated SUMMARY
- OrcaSlicer refs: none this step
- Verification:
  - `cargo build --tests 2>&1 | tail -3` - FACT pass/fail
  - `cargo xtask build-guests && cargo xtask build-guests --check; echo EXIT:$?` - FACT exit 0
  - `mkdir -p target && cargo test -p slicer-wasm-host --test raft_plan_read_accessor_tdd -- raft_plan_reaches_layer_infill_guest --exact --nocapture 2>&1 | tee target/ac7-a.log && test "$(grep -c '^test .* ok$' target/ac7-a.log)" -gt 0` - AC-7 raft-plan half
  - `mkdir -p target && cargo test -p slicer-wasm-host --test raft_plan_read_accessor_tdd -- is_raft_reaches_layer_infill_guest --exact --nocapture 2>&1 | tee target/ac7-b.log && test "$(grep -c '^test .* ok$' target/ac7-b.log)" -gt 0` - AC-7 is-raft half
- Exit condition: AC-7 green on BOTH halves; guests fresh; a `Layer::Infill`
  guest can read `raft_plan` AND `is_raft` on both legs.

### Step 8: Docs + deviation row

- Task IDs: `TASK-536`
- Objective: update `docs/02_ir_schemas.md` (SliceIR section: `raft_fill`,
  `GlobalLayer.is_raft`, the schema bump, the positive `0..N-1` raft offset
  band, that the first printed model layer is `support_raft_layers`, and that
  `index == Vec position` is preserved) and `docs/03_wit_and_manifest.md`
  (`is-raft-prefix`, `raft-plan-view`, the `raft-fill` accessors). File a
  deviation row recording that PnP's raft band is a positive offset band
  matching canonical, that this packet's earlier signed-negative specification
  was withdrawn, and that DEV-124's remedy is upheld rather than reopened —
  re-derive the next free ID at write time
  (`rg -o '^\| DEV-[0-9]{3}' docs/DEVIATION_LOG.md | sort -u | tail -1`).
- Precondition: Steps 1-7 green.
- Postcondition: every `packet.spec.md` Doc Impact Statement grep passes.
- Files allowed to read, with ranges when over 300 lines:
  - `docs/02_ir_schemas.md` - SliceIR section only
  - `docs/DEVIATION_LOG.md` - the header row, the DEV-124 row, and the last 3 rows only
- Files allowed to edit (at most 3):
  - `docs/DEVIATION_LOG.md`
  - `docs/02_ir_schemas.md`
  - `docs/03_wit_and_manifest.md`
- Files explicitly out of bounds: `docs/adr/**` (240b owns the ADR amendment),
  the perimeter generators and their contract tests
- Expected sub-agent dispatches: none
- Context cost: `S`
- Authoritative docs: `docs/specs/support-families-anchored-entities-plan.md`
  Banding decision note - direct range read, for the wording to mirror
- OrcaSlicer refs: none this step
- Verification:
  - `rg -q 'raft_fill' docs/02_ir_schemas.md && rg -q 'is_raft' docs/02_ir_schemas.md && rg -q 'raft offset band' docs/02_ir_schemas.md` - FACT
  - `rg -q 'is-raft-prefix' docs/03_wit_and_manifest.md && rg -q 'raft-plan-view' docs/03_wit_and_manifest.md` - FACT
  - `rg -q 'raft offset band' docs/DEVIATION_LOG.md && cargo xtask check-deviations; echo EXIT:$?` - FACT exit 0
- Exit condition: all Doc Impact greps pass; `check-deviations` exit 0.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | S | red tests, IR-only; delete any stale signed-band test file |
| Step 2 | M | `is_raft` + WIT marker + both harvest legs + contiguity rejection |
| Step 3 | M | planner emits band; manifest key; guest rebuild |
| Step 4 | M | audit applied verbatim; positional + DEV-124 regression guards |
| Step 5 | S | doc-comment correction only |
| Step 6 | M | carrier footprint + schema bump |
| Step 7 | M | raft-plan read path |
| Step 8 | S | docs + deviation row |

Aggregate is `M`; no step is `L`. The standard swarm band applies with no
escalation.

## Packet Completion Gate

- All steps and exits complete.
- Every pipe-suffixed AC command returns PASS.
- `cargo check --workspace --all-targets` green.
- `cargo clippy --workspace --all-targets -- -D warnings` green.
- `cargo xtask check-literals` exit 0 (count equal to the count recorded on a
  clean tree immediately BEFORE this packet's first edit — re-derive it then;
  do not trust any number written here).
- `cargo xtask build-guests --check` exit 0.
- Add `TASK-409`..`TASK-413` and `TASK-533`..`TASK-536` to
  `docs/07_implementation_status.md` through a worker dispatch, never a full
  backlog read. **These rows do not exist today** (verified 2026-09-04) — the
  gate ADDS them rather than updating them.
- Confirm 240b is unblocked: AC-1..AC-7 green.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed AC and packet-level gate command.
- Run `cargo xtask test --summary --workspace` once, dispatched to a sub-agent
  under the FACT contract. This packet adds two IR fields and bumps the schema,
  so the narrow runs alone do not bound the risk.
- Record remaining packet-local risk: the three `[FWD]` audit sites deferred to
  240b, and the silent-semantic class generally (a missed object-bottom
  predicate does not fail to compile).
- Confirm context stayed within the standard band; otherwise record a
  packet-authoring lesson.

All `cargo check`, `cargo clippy`, and `cargo test` invocations in gate and
verification commands use `--all-targets` where applicable so test, bench, and
example targets compile.
