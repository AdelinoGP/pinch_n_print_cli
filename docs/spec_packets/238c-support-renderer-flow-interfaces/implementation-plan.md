# Implementation Plan: 238c-support-renderer-flow-interfaces

## Execution Rules

- Work one atomic step at a time; map every step to grouped task IDs
  (TASK-381..TASK-398, consecutive, no gaps).
- Use TDD (red test first), then implementation, then the narrowest falsifying validation.
- Every field below is a context-budget contract and must be filled independently; never
  write "see Step 1".
- After any step touching `modules/*/src/**`, `modules/*/*.toml`, or
  `crates/slicer-schema/wit/**`, run `cargo xtask build-guests --check` (exit 0 required
  before attributing failures; rebuild without `--check` if stale). After WIT edits, also
  `cargo build --tests` BEFORE the freshness probe.
- Every verification command tees to `target/test-output.log` and guards non-zero matched
  tests (invariant 16). Read results from the log; never re-run for more output.
- All `cargo check`/`clippy`/`test` gate commands use `--all-targets` where applicable.

## Steps

### Step 1: DEV-145 manifest defaults + config-doc regeneration

- Task IDs: `TASK-381`
- Objective: flip `support_bottom_interface_spacing` default −1.0 → 0.5 in both family
  manifests (min stays −1.0 so the legacy mirror-top sentinel remains expressible).
- Precondition: 238b implemented; guests fresh (`cargo xtask build-guests --check` exit 0).
- Postcondition: both manifests read `default = 0.5`; config docs regenerated; renderer
  parsers still accept negative as mirror-top legacy.
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/traditional-support/traditional-support.toml` - lines 60–90
  - `modules/core-modules/tree-support/tree-support.toml` - lines 60–90
- Files allowed to edit (at most 3):
  - `modules/core-modules/traditional-support/traditional-support.toml`
  - `modules/core-modules/tree-support/tree-support.toml`
  - `docs/15_config_keys_reference.md` (regen gate output only)
- Files explicitly out of bounds:
  - `modules/core-modules/*/src/**` (no behavior change in this step)
- Blast-radius discipline: manifest-only default change; verify no test hard-asserts the
  −1.0 default via `rg -n 'support_bottom_interface_spacing' modules/ crates/` before
  editing; fix any assertion found in the same step.
- Expected sub-agent dispatches:
  - Question: current manifest blocks + test references; scope: `modules/core-modules/*/*.toml`, `modules/core-modules/*/src/lib.rs`; return: `SNIPPETS`
- Context cost: `S`
- Authoritative docs:
  - `docs/specs/support-families-anchored-entities-plan.md` - §10 DEV-145 correction range
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` - delegate: `PrintConfigDef::build` default 0.5 min 0
- Verification:
  - `rg -q 'default = 0\.5' modules/core-modules/traditional-support/traditional-support.toml && rg -q 'default = 0\.5' modules/core-modules/tree-support/tree-support.toml && echo DEV145-OK`
  - `cargo test -p traditional-support --test traditional_support_tdd 2>&1 | tee target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0`
  - `cargo xtask build-guests --check`
- Exit condition: both greps pass, family suites green, guests fresh.

### Step 2: Density derivation module (red-first) in slicer-core

- Task IDs: `TASK-382`
- Objective: create `crates/slicer-core/src/support_regularize.rs` (initially hosting the
  density/pitch helpers) with canonical formulas + clamp/guard behavior; red tests first
  in `crates/slicer-core/tests/support_flow_semantics_tdd.rs`.
- Precondition: Step 1 green.
- Postcondition: `body_density(flow, base_spacing)`,
  `interface_density(flow, interface_spacing)`, `bottom_interface_density` helpers exist
  with exact canonical closed forms; clamp + non-positive-guard tests pass.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-core/src/flow.rs` - range around `line_width_to_spacing(width,
    layer_height)` (~line 88) and its inverse helper
  - `modules/core-modules/tree-support/src/lib.rs` - lines 70–200 (current config/pitch)
- Files allowed to edit (at most 3):
  - `crates/slicer-core/src/support_regularize.rs` (new)
  - `crates/slicer-core/tests/support_flow_semantics_tdd.rs` (new)
  - `crates/slicer-core/src/lib.rs` (module declaration only)
- Files explicitly out of bounds:
  - both renderer modules (consumed in Steps 7/9)
- Helper call-shape contract (binding for Steps 2/7/9/13): every density helper takes the
  resolved line width AND the effective layer height, and calls
  `slicer_core::flow::line_width_to_spacing(width, layer_height)` — the live signature is
  binary (`(width: f32, layer_height: f32) -> Result<f32, NegativeSpacingError>`);
  tests exercise BOTH arguments (e.g. `(0.4, 0.2)` vs `(0.4, 0.3)` produce different
  spacings) so a unary mis-call cannot pass; `NegativeSpacingError` propagates as a
  structured decline.
- Expected sub-agent dispatches:
  - Question: confirm exact `line_width_to_spacing` signature + error type + how renderers obtain per-layer effective layer height; scope: `crates/slicer-core/src/flow.rs`, both renderer lib.rs; return: `SNIPPETS`
- Context cost: `S`
- Authoritative docs:
  - `docs/21_data_defaults_and_fixtures.md` - literal-churn gate rules (helpers are fns, no watched struct literals)
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/Support/SupportParameters.hpp` - delegate: constructor density derivations
- Verification:
  - `cargo test -p slicer-core --features host-algos --test support_flow_semantics_tdd -- canonical_density_derivations_match_formulas densities_clamp_to_one_solid_pitch nonpositive_interface_flow_falls_back_to_default --exact 2>&1 | tee target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0`
- Exit condition: all three named tests green under `--features host-algos` (E6).

### Step 3: DEV-129 verify-close-or-finish

- Task IDs: `TASK-383`
- Objective: verify bottom-interface emission truth; remove the stale "Not yet
  implemented" comment above `support_interface_bottom_layers` in
  `tree-support-planner.toml`; close DEV-129 in DEVIATION_LOG as implemented (or finish a
  real gap first — falsifying-exit alternative below).
- Precondition: guests fresh (T4 probe before attributing anything).
- Postcondition: manifest carries no stale claim; `diagnostics_tdd` green; DEV-129 row
  closed with evidence pointers.
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/tree-support-planner/tests/diagnostics_tdd.rs` - lines 250–420
  - `modules/core-modules/tree-support-planner/tree-support-planner.toml` - lines 115–140
  - `docs/DEVIATION_LOG.md` - DEV-129 row range only (grep-located)
- Files allowed to edit (at most 3):
  - `modules/core-modules/tree-support-planner/tree-support-planner.toml`
  - `docs/DEVIATION_LOG.md`
  - (only if finish-branch taken) `modules/core-modules/tree-support-planner/src/lib.rs`
- Files explicitly out of bounds:
  - renderer modules; other DEV rows
- Expected sub-agent dispatches:
  - Question: does a real slice attribute InterfaceRole::Floor bands; scope: planner tests + `git grep Floor`; return: `FACT`
- Context cost: `S`
- Authoritative docs:
  - `docs/specs/support-families-anchored-entities-plan.md` - §10 DEV-129 range
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/Support/SupportParameters.hpp` - delegate: `number_of_support_interface_bottom_layers`
- Verification:
  - `! rg -q 'Not yet implemented' modules/core-modules/tree-support-planner/tree-support-planner.toml && cargo test -p tree-support-planner --test diagnostics_tdd -- interface_bottom_layers_is_supported_and_warns_nothing --exact 2>&1 | tee target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0`
  - `cargo xtask check-deviations`
- Falsifying exit: if Floor bands do NOT emit on a real plan, STOP the close; implement
  the missing floor emission in this step's third edit slot (lib.rs), re-run, and only
  then close. Reporting "closed" with a red suite is a step failure.
- Exit condition: grep negative, test green, deviations check clean.

### Step 4: G-12 radius cap 6.0 → 10.0

- Task IDs: `TASK-384`
- Objective: raise `MAX_BRANCH_RADIUS_MM` to canonical 10.0; update clamp test.
- Precondition: Step 3 green.
- Postcondition: constant = 10.0; clamp test asserts the new cap; planner suite green;
  golden drift classified per E3 (expected none).
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/tree-support-planner/src/lib.rs` - lines 40–95, 4400–4440, 5630–5650
- Files allowed to edit (at most 3):
  - `modules/core-modules/tree-support-planner/src/lib.rs`
  - `modules/core-modules/tree-support-planner/tests/tree_family_tdd.rs` (new clamp test `branch_radius_clamps_at_canonical_maximum`)
- Files explicitly out of bounds:
  - golden fixtures (regeneration only via E3-gated env, never silent)
- Expected sub-agent dispatches:
  - Question: confirm no other 6.0 radius literals; scope: `modules/core-modules/tree-support-planner/`; return: `LOCATIONS`
- Context cost: `S`
- Authoritative docs:
  - `docs/specs/support-parity-gap-register.md` - G-12 row
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/Support/TreeSupport.hpp` - delegate: MIN/MAX_BRANCH_RADIUS constants
- Verification:
  - `rg -q 'MAX_BRANCH_RADIUS_MM: f32 = 10\.0' modules/core-modules/tree-support-planner/src/lib.rs && ! rg -q 'MAX_BRANCH_RADIUS_MM: f32 = 6\.0' modules/core-modules/tree-support-planner/src/lib.rs`
  - `cargo test -p tree-support-planner --test tree_family_tdd -- branch_radius_clamps_at_canonical_maximum --exact 2>&1 | tee target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0`
  - `cargo xtask build-guests --check`
- Exit condition: constant grep pair passes, test green, guests fresh.

### Step 5: G-13 raise-to-base under interfaces

- Task IDs: `TASK-385`
- Objective: implement canonical raise-to-`base_radius` when
  `support_interface_top_layers > 0` in the planner radius pipeline.
- Precondition: Step 4 green.
- Postcondition: red-first test `radius_raises_to_base_under_interfaces` green (raises
  with top=2, unchanged with top=0).
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/tree-support-planner/src/lib.rs` - radius fn range (Step 4 ranges) + interface-layer plumbing near node arena
- Files allowed to edit (at most 3):
  - `modules/core-modules/tree-support-planner/src/lib.rs`
  - `modules/core-modules/tree-support-planner/tests/tree_family_tdd.rs`
- Files explicitly out of bounds:
  - renderer modules; collision/avoidance volumes (238b scope)
- Expected sub-agent dispatches:
  - Question: SUMMARY of canonical mm-to-top `calc_branch_radius` raise mechanics; scope: `TreeSupport.cpp`; return: `SUMMARY`
- Context cost: `M`
- Authoritative docs:
  - `docs/specs/support-parity-gap-register.md` - G-13 row
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/Support/TreeSupport.cpp` - delegate: `calc_branch_radius` (mm_to_top variant)
- Verification:
  - `cargo test -p tree-support-planner --test tree_family_tdd -- radius_raises_to_base_under_interfaces --exact 2>&1 | tee target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0`
  - `cargo xtask build-guests --check`
- Exit condition: test green; guests fresh.

### Step 6: G-18 floor-band structure (traditional + tree gating)

- Task IDs: `TASK-386`
- Objective: adopt canonical bottom-band semantics so top=2/bottom=2 yields 3 interface
  blocks (Orca-measured), preserving `ee27ac94` pins; replicate the `draw_circles` floor
  gate in the tree path where conditions diverge.
- Precondition: Step 5 green; baseline count recorded (2) from a fresh slice.
- Postcondition: new `interface_band_counts_match_canonical_structure` integration test
  green; `interface_layer_count_follows_config` rows still green.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-runtime/tests/integration/support_family_closure.rs` - lines 830–990
  - `modules/core-modules/traditional-support/src/lib.rs` - band-planning range
- Files allowed to edit (at most 3):
  - `modules/core-modules/traditional-support/src/lib.rs`
  - `modules/core-modules/tree-support/src/lib.rs` (gate alignment only)
  - `crates/slicer-runtime/tests/integration/support_family_closure.rs` (new test)
- Files explicitly out of bounds:
  - `traditional-support-planner` band anchoring (contact-inclusive logic from ee27ac94 stays)
- Expected sub-agent dispatches:
  - Question: SUMMARY of `draw_circles` floor-block gating + band splitting vs PnP; scope: `TreeSupport.cpp` + planner loop; return: `SUMMARY`
- Context cost: `M`
- Authoritative docs:
  - `docs/specs/support-parity-gap-register.md` - G-18 row
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/Support/TreeSupport.cpp` - delegate: `draw_circles` floor block
- Verification:
  - `cargo test -p slicer-runtime --test integration -- interface_band_counts_match_canonical_structure interface_layer_count_follows_config --exact 2>&1 | tee target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0`
  - `cargo xtask build-guests --check`
- Exit condition: both integration tests green; guests fresh.

### Step 7: G-10 + G-11 tree renderer hollow walls + density model

- Task IDs: `TASK-387`
- Objective: replace `render_polygon` filled-body model with wall/fill split consuming
  Step-2 helpers; remove `support_density` percent handling; wire
  `support_base_pattern_spacing` + typed line width.
- Precondition: Steps 2/6 green.
- Postcondition: `tree_bodies_render_hollow_concentric_walls` green (wall-loop count +
  interior pitch assertions, E1); `fill_pitch_honours_support_density` replaced by
  density-derivation assertions; manifest `support_density` removed.
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/tree-support/src/lib.rs` - full (636 lines, one pass)
  - `modules/core-modules/tree-support/tests/tree_support_tdd.rs` - full
- Files allowed to edit (at most 3):
  - `modules/core-modules/tree-support/src/lib.rs`
  - `modules/core-modules/tree-support/tests/tree_support_tdd.rs`
  - `modules/core-modules/tree-support/tree-support.toml` (remove `support_density` block)
- Files explicitly out of bounds:
  - `traditional-support` (Step 9)
- Blast-radius discipline: removing a manifest key changes the declared schema — grep
  `rg -n 'support_density' modules/core-modules/tree-support/ crates/ docs/` first; every
  consumer found is edited in this step; DEVIATION_LOG removal row drafted (filed Step 17).
- Expected sub-agent dispatches:
  - Question: SUMMARY of `_make_loops`/`make_perimeter_and_infill` wall-then-fill order; scope: `SupportCommon.cpp`; return: `SUMMARY`
- Context cost: `M`
- Authoritative docs:
  - `docs/specs/support-parity-gap-register.md` - G-10/G-11 rows
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/Support/TreeSupport.cpp` - delegate: `generate_toolpaths` sheath rendering
  - `OrcaSlicerDocumented/src/libslic3r/Support/SupportCommon.cpp` - delegate: `tree_supports_generate_paths`
- Verification:
  - `cargo test -p tree-support --test tree_support_tdd -- tree_bodies_render_hollow_concentric_walls --exact 2>&1 | tee target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0`
  - `! rg -q 'support_density' modules/core-modules/tree-support/tree-support.toml`
  - `cargo xtask build-guests --check`
- Exit condition: structure test green; key absent; guests fresh.

### Step 8: interface_regularize consolidation

- Task IDs: `TASK-388`
- Objective: move `regularize_entry_roles` into
  `crates/slicer-core/src/support_regularize.rs`; delete both module copies; repoint
  consumers.
- Precondition: Step 7 green (tree renderer stable before its import surface changes).
- Postcondition: AC-9 grep triple passes; moved tests green in slicer-core; both family
  suites green.
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/tree-support/src/interface_regularize.rs` - full (362 lines)
- Files allowed to edit (at most 3):
  - `crates/slicer-core/src/support_regularize.rs`
  - `crates/slicer-core/tests/support_interface_regularize_tdd.rs` (moved tests)
  - `modules/core-modules/tree-support/src/lib.rs` + `modules/core-modules/traditional-support/src/lib.rs` (import lines only; counted as one edit class)
- Files explicitly out of bounds:
  - `rectilinear-infill` (DEV-127 remainder)
- Note: deletion of the two copies is part of the same step (the AC greps assert
  absence); if the ≤3-edit budget strains, the two `use`-line flips may be a same-task
  follow-up commit within this step.
- Expected sub-agent dispatches:
  - Question: confirm both copies still byte-identical + consumer call sites; scope: `modules/core-modules/*/src/`; return: `FACT`
- Context cost: `S`
- Authoritative docs:
  - none beyond in-file canonical comments
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/Support/SupportCommon.cpp` - delegate: `generate_interface_layers` `regularize` lambda (behavior unchanged; reference only)
- Verification:
  - `test ! -f modules/core-modules/tree-support/src/interface_regularize.rs && test ! -f modules/core-modules/traditional-support/src/interface_regularize.rs && rg -q 'pub fn regularize_entry_roles' crates/slicer-core/src/support_regularize.rs`
  - `cargo test -p slicer-core --features host-algos --test support_interface_regularize_tdd 2>&1 | tee target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0`
  - `cargo xtask build-guests --check`
- Exit condition: greps pass; moved suite green; guests fresh.

### Step 9: traditional renderer density model + interface pitch (DEV-146)

- Task IDs: `TASK-389`
- Objective: wire Step-2 helpers into `traditional-support` (body + interface pitches);
  add `support_interface_flow` key (percent, default 100) to both manifests; pitch
  derivation per AC-11.
- Precondition: Steps 2/8 green.
- Postcondition: `interface_pitch_derives_from_interface_flow_over_line_width` green;
  both manifests declare `support_interface_flow`; config docs regenerated.
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/traditional-support/src/lib.rs` - full (622 lines, one pass)
  - `modules/core-modules/traditional-support/tests/traditional_support_tdd.rs` - pitch-assertion ranges
- Files allowed to edit (at most 3):
  - `modules/core-modules/traditional-support/src/lib.rs`
  - `modules/core-modules/traditional-support/traditional-support.toml`
  - `modules/core-modules/tree-support/tree-support.toml` (same key declaration)
- Files explicitly out of bounds:
  - `tree-support/src/lib.rs` (its flow wiring landed Step 7; key declaration only here)
- Expected sub-agent dispatches:
  - Question: SUMMARY of `support_material_interface_flow` derivation incl. fallback; scope: `Flow.cpp`; return: `SUMMARY`
- Context cost: `M`
- Authoritative docs:
  - `docs/21_data_defaults_and_fixtures.md` - manifest-default fixtures
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/Flow.cpp` - delegate: `support_material_interface_flow`
- Verification:
  - `cargo test -p traditional-support --test traditional_support_tdd -- interface_pitch_derives_from_interface_flow_over_line_width --exact 2>&1 | tee target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0`
  - `rg -q 'support_interface_flow' modules/core-modules/traditional-support/traditional-support.toml && rg -q 'support_interface_flow' modules/core-modules/tree-support/tree-support.toml`
  - `cargo xtask build-guests --check`
- Exit condition: test green; both manifest greps pass; guests fresh.

### Step 10: F-37 IR + WIT carrier (schema bump in-step)

- Task IDs: `TASK-390`
- Objective: add `SupportPlanRole::BaseInterface` +
  `ExtrusionRole::SupportBaseInterface` (priority 5250, closed-loop false) to
  `crates/slicer-ir/src/slice_ir.rs`; add WIT `base-interface` role +
  `push-base-interface-path` builder method; bump the derived-at-activation schema
  version; fix every newly-non-exhaustive match in the same step.
- Precondition: Step 9 green; LOCATIONS dispatch of all `match` sites completed.
- Postcondition: `cargo build --tests` green workspace-wide; WIT world validates.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-ir/src/slice_ir.rs` - ranges around `SupportPlanRole` (~1296), `ExtrusionRole` (~2192), priority fn (~2249)
  - `crates/slicer-schema/wit/deps/prepass-support-geometry/prepass-support-geometry.wit` - lines 1–40
  - `crates/slicer-schema/wit/deps/ir-types.wit` - lines 260–295
- Files allowed to edit (at most 3 edit classes):
  - `crates/slicer-ir/src/slice_ir.rs`
  - `crates/slicer-schema/wit/deps/prepass-support-geometry/prepass-support-geometry.wit`
  - `crates/slicer-schema/wit/deps/ir-types.wit`
  (match-arm fallout across other crates is owned by this task but batched as the
  step's dispatch-driven fix-up pass, never deferred)
- Files explicitly out of bounds:
  - `layer-support/layer-support.wit`, `layer-support-postprocess/layer-support-postprocess.wit` unless the builder method requires world re-export (verify first)
- Blast-radius discipline: enum-variant analog of struct-literal rule — the Step-10
  LOCATIONS sweep lists every `match SupportPlanRole` / `match ExtrusionRole` site; each
  gets an arm in this step (planners/renderers get real arms in Steps 11–12; host/gcode
  get theirs here).
- Expected sub-agent dispatches:
  - Question: LOCATIONS of every match on SupportPlanRole/ExtrusionRole; scope: `crates/ modules/`; return: `LOCATIONS` (≤20)
- Context cost: `M`
- Authoritative docs:
  - `docs/02_ir_schemas.md` - role/schema sections (edit in Step 12)
- OrcaSlicer refs:
  - none (structural carrier step)
- Verification:
  - `cargo build --tests 2>&1 | tail -5`
  - `rg -q 'base-interface' crates/slicer-schema/wit/deps/prepass-support-geometry/prepass-support-geometry.wit && rg -q 'push-base-interface-path' crates/slicer-schema/wit/deps/ir-types.wit`
  - `cargo xtask build-guests --check` (expect stale → rebuild without `--check`, re-verify exit 0)
- Exit condition: build green; WIT greps pass; guests rebuilt fresh.

### Step 11a: host dispatch match arm + builder impl

- Task IDs: `TASK-391`
- Objective: add the `BaseInterface` arm to the live `SupportPlanRole` → WIT-role
  dispatch match in `crates/slicer-wasm-host/src/dispatch.rs` (verified four-arm match at
  its role-mapping site) and implement `push-base-interface-path` in the host builder
  (`host.rs`).
- Precondition: Step 10 green.
- Postcondition: dispatch compiles with five exhaustive role arms; new push method
  reachable through the host builder; no marshal change yet.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-wasm-host/src/dispatch.rs` - role-mapping match range (grep-located,
    ~line 1971)
  - `crates/slicer-wasm-host/src/host.rs` - builder-impl range (grep-located)
- Files allowed to edit (at most 3):
  - `crates/slicer-wasm-host/src/dispatch.rs`
  - `crates/slicer-wasm-host/src/host.rs`
- Files explicitly out of bounds:
  - `marshal/` legs (Step 11b); guest module sources (Step 12)
- Expected sub-agent dispatches:
  - Question: where `push_interface_path` is implemented and where the SupportPlanRole dispatch match lives; scope: `crates/slicer-wasm-host/src/{dispatch.rs,host.rs}`; return: `LOCATIONS`
- Context cost: `S`
- Authoritative docs:
  - `docs/02_ir_schemas.md` - view-seam section (read-only here)
- OrcaSlicer refs:
  - none
- Verification:
  - `rg -q 'SupportPlanRole::BaseInterface' crates/slicer-wasm-host/src/dispatch.rs && cargo build --tests -p slicer-wasm-host 2>&1 | tail -5`
  - `cargo xtask build-guests --check`
- Exit condition: grep positive; wasm-host builds; guests fresh.

### Step 11b: both marshal legs round-trip the role

- Task IDs: `TASK-391` (continued)
- Objective: round-trip the base-interface role through BOTH marshal legs
  (`marshal/in_.rs` wasm view, `marshal/native.rs` native view) so native and wasm paths
  carry identical role attribution (T9 leg-skew guard).
- Precondition: Step 11a green.
- Postcondition: leg-equivalence asserted; a plan entry attributed `BaseInterface` on the
  wasm path is visible identically on the native path.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-wasm-host/src/marshal/native.rs` - role-mapping range
  - `crates/slicer-wasm-host/src/marshal/in_.rs` - view-construction role range
- Files allowed to edit (at most 3):
  - `crates/slicer-wasm-host/src/marshal/in_.rs`
  - `crates/slicer-wasm-host/src/marshal/native.rs`
- Files explicitly out of bounds:
  - guest module sources (Step 12); WIT files (locked Step 10)
- Expected sub-agent dispatches:
  - Question: where each marshal leg maps support roles today; scope: `crates/slicer-wasm-host/src/marshal/`; return: `LOCATIONS`
- Context cost: `S`
- Authoritative docs:
  - `docs/02_ir_schemas.md` - view-seam section (read-only here)
- OrcaSlicer refs:
  - none
- Verification:
  - `cargo build --tests -p slicer-wasm-host 2>&1 | tail -5`
  - `rg -q 'BaseInterface' crates/slicer-wasm-host/src/marshal/in_.rs && rg -q 'BaseInterface' crates/slicer-wasm-host/src/marshal/native.rs`
  - `cargo xtask build-guests --check`
- Exit condition: both-leg greps pass; wasm-host builds; guests fresh.

### Step 12: planner attribution + renderer emission + gcode marker (F-37 end-to-end)

- Task IDs: `TASK-392`
- Objective: attribute base-interface circles in the tree planner
  (`base_interface_band_attributed_in_plan_roles`), emit through the new carrier from
  both renderers, add the `orca_type_label` arm + feedrate mapping
  (`base_interface_role_maps_to_support_interface_marker`), document the role in
  `docs/02_ir_schemas.md`.
- Precondition: Steps 10/11a/11b green.
- Postcondition: AC-6/AC-7/AC-8 tests green end-to-end through a real dispatch.
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/tree-support-planner/src/lib.rs` - `InterfaceRole` + classification loop ranges
  - `crates/slicer-gcode/src/emit.rs` - label/feedrate ranges (~150–250)
- Files allowed to edit (at most 3 edit classes):
  - `modules/core-modules/tree-support-planner/src/lib.rs` (+ `tree_family_tdd.rs` test)
  - `modules/core-modules/tree-support/src/lib.rs` + `traditional-support/src/lib.rs` (carrier consumption)
  - `crates/slicer-gcode/src/emit.rs` (+ lib test) and `docs/02_ir_schemas.md`
- Files explicitly out of bounds:
  - WIT files (locked Step 10)
- Expected sub-agent dispatches:
  - Question: SUMMARY of `num_top_base_interface_layers` canonical derivation; scope: `TreeSupport.cpp`/`SupportParameters.hpp`; return: `SUMMARY`
- Context cost: `M`
- Authoritative docs:
  - `docs/02_ir_schemas.md` - add BaseInterface documentation
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/Support/TreeSupport.cpp` - delegate: base-interface band derivation (commit `050d5c3a` records the in-tree analysis)
- Verification:
  - `cargo test -p tree-support-planner --test tree_family_tdd -- base_interface_band_attributed_in_plan_roles --exact 2>&1 | tee target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0`
  - `cargo test -p slicer-gcode --lib -- base_interface_role_maps_to_support_interface_marker --exact 2>&1 | tee target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0`
  - `rg -q 'BaseInterface' docs/02_ir_schemas.md`
  - `cargo xtask build-guests --check`
- Exit condition: all green; guests fresh.

### Step 13: DEV-146 tree-side pitch + DEV row drafting

- Task IDs: `TASK-393`
- Objective: confirm tree-side interface pitch consumes the shared derivation with
  `support_interface_flow` (Step 9 key); draft DEVIATION_LOG rows (DEV-146 mechanism,
  `support_density` removal) — final filing in Step 17.
- Precondition: Step 12 green.
- Postcondition: tree interface-pitch test green; rows drafted in packet notes.
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/tree-support/src/lib.rs` - interface pitch range
- Files allowed to edit (at most 3):
  - `modules/core-modules/tree-support/src/lib.rs`
  - `modules/core-modules/tree-support/tests/tree_support_tdd.rs`
- Files explicitly out of bounds:
  - `docs/DEVIATION_LOG.md` (Step 17 owns it)
- Expected sub-agent dispatches: none.
- Context cost: `S`
- Authoritative docs:
  - `docs/specs/support-parity-gap-register.md` - DEV-146 context
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/Support/SupportParameters.hpp` - delegate: `top_interface_spacing` derivation
- Verification:
  - `cargo test -p tree-support --test tree_support_tdd -- interface_pitch_derives_from_interface_flow_over_line_width --exact 2>&1 | tee target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0`
  - `cargo xtask build-guests --check`
- Exit condition: test green; guests fresh.

### Step 14: negative-case hardening (AC-N1/AC-N2 surfaces in modules)

- Task IDs: `TASK-394`
- Objective: renderer-boundary guards for degenerate config (zero spacings, non-positive
  flow) producing solid pitch / canonical-default fallback; tests in both family suites.
- Precondition: Step 13 green.
- Postcondition: both negative tests green; no degenerate geometry emitted.
- Files allowed to read, with ranges when over 300 lines:
  - both renderer `src/lib.rs` config-parse ranges
- Files allowed to edit (at most 3):
  - `modules/core-modules/tree-support/src/lib.rs`
  - `modules/core-modules/tree-support/tests/tree_support_tdd.rs`
  - `modules/core-modules/traditional-support/src/lib.rs` + its test file (same edit class)
- Files explicitly out of bounds: slicer-core helpers (locked Step 2 behavior).
- Expected sub-agent dispatches: none.
- Context cost: `S`
- Authoritative docs: `docs/21_data_defaults_and_fixtures.md` - fixture-base rules
- OrcaSlicer refs: none beyond Step 2 evidence.
- Verification:
  - `cargo test -p tree-support --test tree_support_tdd -- nonpositive_interface_flow_falls_back_to_default_module_boundary --exact 2>&1 | tee target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0`
  - `cargo xtask build-guests --check`
- Exit condition: negative tests green in both suites; guests fresh.

### Step 15: workspace compile + clippy + literals gates

- Task IDs: `TASK-395`
- Objective: whole-tree gates green after all behavior steps.
- Precondition: Steps 1–14 green.
- Postcondition: check/clippy/literals clean.
- Files allowed to read: gate output logs only.
- Files allowed to edit: whatever the gates flag, ≤3 files, all within the packet surface.
- Files explicitly out of bounds: pre-existing G-15 literal debt (T10 — count unchanged).
- Expected sub-agent dispatches:
  - Question: run gates + return FACT; scope: workspace; return: `FACT pass/fail`
- Context cost: `S`
- Authoritative docs: `.agents/aux-commands.md` only if a gate command is unknown.
- OrcaSlicer refs: none.
- Verification:
  - `cargo check --workspace --all-targets 2>&1 | tail -3`
  - `cargo clippy --workspace --all-targets -- -D warnings 2>&1 | tail -3`
  - `cargo xtask check-literals`
- Exit condition: all three clean.

### Step 16: human validation gate artifacts + checklist

- Task IDs: `TASK-396`
- Objective: produce `tmp/support_test_tree_238c.gcode`,
  `tmp/support_test_normal_238c.gcode`, `tmp/vd-238c/` bundle; run the packet.spec.md
  checklist incl. interface block counts vs references; record sign-off line.
- Precondition: Step 15 green.
- Postcondition: artifacts exist; checklist answered in writing (E2); sign-off recorded
  in `packet.spec.md` §Human Validation Gate.
- Files allowed to read: generated G-code summaries (grep counts, never full loads),
  visual-debug manifest.
- Files allowed to edit (at most 3):
  - `docs/spec_packets/238c-support-renderer-flow-interfaces/packet.spec.md` (sign-off line)
  - `tmp/**` artifacts
- Files explicitly out of bounds: reference G-code regeneration (human-owned §9).
- Expected sub-agent dispatches:
  - Question: run the two slice commands + count `;TYPE:Support interface` blocks; scope: CLI; return: `FACT` counts
- Context cost: `S`
- Authoritative docs: `docs/19_visual_debug.md` - bundle layout (ranged)
- OrcaSlicer refs: none (references already on disk under `tmp/`; regenerate is
  human-owned).
- Verification:
  - `test -f tmp/support_test_tree_238c.gcode && test -f tmp/support_test_normal_238c.gcode && echo ARTIFACTS-OK`
  - block-count greps recorded in the checklist (traditional must read 3)
- Exit condition: artifacts + written checklist verdicts + sign-off line present.

### Step 17: deviation + doc closure

- Task IDs: `TASK-397`
- Objective: file DEVIATION_LOG rows — DEV-129 closed, DEV-145 corrected, DEV-146
  mechanism recorded, new row for `support_density` removal — and finish doc greps.
- Precondition: Steps 12–16 green.
- Postcondition: `cargo xtask check-deviations` clean; every Doc Impact grep passes.
- Files allowed to read: DEVIATION_LOG row ranges; doc anchors.
- Files allowed to edit (at most 3):
  - `docs/DEVIATION_LOG.md`
  - `docs/02_ir_schemas.md` (if Step 12 deferred any anchor)
  - `docs/15_config_keys_reference.md` (final regen)
- Files explicitly out of bounds: other packets' docs; plan file.
- Expected sub-agent dispatches:
  - Question: next free DEV id; scope: `docs/DEVIATION_LOG.md`; return: `FACT`
- Context cost: `S`
- Authoritative docs: `docs/DEVIATION_LOG.md` header rules (ranged).
- OrcaSlicer refs: none.
- Verification:
  - `cargo xtask check-deviations`
  - `rg -q 'support_interface_flow' docs/15_config_keys_reference.md && rg -q 'BaseInterface' docs/02_ir_schemas.md`
- Exit condition: checks green.

### Step 18: task registration + packet status flip

- Task IDs: `TASK-398`
- Objective: register TASK-381..398 in `docs/07_implementation_status.md` via worker
  dispatch; run the packet-level gates; flip status to `implemented` (human gate already
  signed Step 16).
- Precondition: Step 17 green.
- Postcondition: `docs/07` rows present; packet-level gates green.
- Files allowed to edit (at most 3):
  - `docs/07_implementation_status.md`
  - `docs/spec_packets/238c-support-renderer-flow-interfaces/packet.spec.md` (status line only)
- Files explicitly out of bounds: queue table in the plan (orchestrator-owned).
- Expected sub-agent dispatches:
  - Question: append TASK rows; scope: `docs/07`; return: `FACT`
- Context cost: `S`
- Authoritative docs: `docs/07_implementation_status.md` format (ranged).
- OrcaSlicer refs: none.
- Verification:
  - `rg -q 'TASK-398' docs/07_implementation_status.md`
  - `cargo test -p slicer-runtime --test integration -- interface_band_counts_match_canonical_structure --exact 2>&1 | tee target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0`
- Exit condition: registration present; gate green.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | S | manifest flips + regen |
| Step 2 | S | helpers + formulas |
| Step 3 | S | DEV-129 close |
| Step 4 | S | constant flip |
| Step 5 | M | raise-to-base rule |
| Step 6 | M | floor bands |
| Step 7 | M | hollow walls + density |
| Step 8 | S | consolidation |
| Step 9 | M | traditional pitch + key |
| Step 10 | M | WIT/IR carrier + blast radius |
| Step 11a | S | dispatch.rs arm + builder impl |
| Step 11b | S | both marshal legs |
| Step 12 | M | end-to-end role |
| Step 13 | S | tree pitch + drafts |
| Step 14 | S | negative cases |
| Step 15 | S | gates |
| Step 16 | S | human-gate artifacts |
| Step 17 | S | deviations/docs |
| Step 18 | S | registration |

Split before activation if aggregate cost exceeds M or any step is L.

## Packet Completion Gate

- All steps and exits complete.
- Every pipe-suffixed AC command returns PASS.
- Update `docs/07_implementation_status.md` through a worker dispatch, never a full
  backlog read.
- Reconcile reopened/superseded status transitions (DEV rows; stub absorption record
  lives in `requirements.md` §Problem Statement).
- `packet.spec.md` is ready for `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed AC and packet-level gate command.
- Record remaining packet-local risk.
- Confirm context stayed at or below 150k standard, or at/below 300k only with a logged
  swarm ESCALATION; otherwise record a packet-authoring lesson.
- The `cargo test --workspace` full-suite run is permitted here only per repo Test
  Discipline (packet-close ceremony, every narrower command already green) and MUST go
  through `cargo xtask test --summary --workspace --no-fail-fast` with a FACT pass/fail
  return.
