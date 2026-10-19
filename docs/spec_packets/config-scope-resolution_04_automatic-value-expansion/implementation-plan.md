# Implementation Plan: automatic-value-expansion

## Execution Rules

- Work one atomic step at a time; every step maps to `TASK-565`.
- Use TDD, then implementation, then the narrowest falsifying validation.
- Re-derive ledger counts and packet-1 status at use time; do not copy counts from this packet into code or tests.
- Every cargo command is delegated and every test invocation tees combined output to `target/test-output.log`.

## Steps

### Step 1: Clear the registry FORWARD-DEP and classify expansion bases

- Task IDs: `TASK-565`
- Objective: Verify active packet 1 has completed and its six named dependency surfaces have the exact implemented shapes in `design.md` §Open Questions; classify existing percent reads into config-only typed bases versus geometry/context-dependent rules; lock the exact manifest/base-key edit set—including all four overhang speeds over `outer_wall_speed`—without editing code.
- Precondition: packet 1 is active, but its FORWARD-DEP is unsatisfied until that packet completes and its exports are verified; active status alone does not permit packet-4 implementation.
- Postcondition: a bounded inventory confirms packet 1 is complete, all six dependency shapes match, and only config-only bases are listed for this packet; if packet 1 remains active/incomplete or any shape differs, implementation stops and packet 4 remains draft.
- Files allowed to read, with ranges when over 300 lines:
  - `docs/spec_packets/config-scope-resolution_01_config-schema-registry/{packet.spec.md,requirements.md,design.md}` - only export/status sections
  - `modules/core-modules/*/*.toml` - only `percent`/`float_or_percent` entries and `base_key`/legacy `ratio_over` lines
  - `crates/**/src/*.rs` - only `get_abs_value`, `resolve_role_width`, `resolve_support_line_width_mm`, and `read_speed` matches
- Files allowed to edit (at most 3):
  - None; discovery step.
- Files explicitly out of bounds:
  - `OrcaSlicerDocumented/`, `target/`, lockfiles, generated code, packet 1 files, scope resolver implementation files
- Blast-radius discipline:
  - No struct field or schema constant is added in this step. Record the `RoleWidthContext` literal locations for Step 6; do not edit them here.
- Expected sub-agent dispatches:
  - Question: do the six packet-1 exports resolve with exactly the documented shapes, and which percent reads require only config values?; scope: packet 1 plus exact config/reader greps; return: `FACT` for exports and `LOCATIONS` ≤20 for reads.
- Context cost: `S`
- Authoritative docs:
  - `docs/specs/config-scope-resolution-plan.md` - Expansion section and queue rows 4/10
  - `docs/adr/0068-config-scope-is-a-wire-encoding.md` - amendment Phase B/C
- OrcaSlicer refs:
  - None in this discovery step; upstream behavior is delegated in Step 2.
- Verification:
  - `rg -n 'RegistryEntry|ConfigSchemaRegistry|ModuleDeclaration|HostChannels|AssemblyOutcome|assemble_registry' docs/spec_packets/config-scope-resolution_01_config-schema-registry/{design.md,implementation-plan.md}` - FACT names/shapes reconciled.
- Exit condition: PASS only if all six exports match and every candidate retained for this packet can be expanded from config/nozzle/tool bases alone; all four overhang-speed percentages are retained in Phase B, while only volumetric or geometry/per-move candidates are recorded as packet-10-owned and excluded.

### Step 2: Author the expansion engine red tests and implementation

- Task IDs: `TASK-565`
- Objective: Add the public expansion API and independent tests for literal width and overhang-speed percentages, zero, `-1`, tool-base, ordering, atomic-failure, missing-base, and non-positive-base behavior.
- Precondition: Step 1 cleared the packet-1 FORWARD-DEP shapes.
- Postcondition: `expand_automatic_values` stages base-first replacements and atomically commits only a fully valid expansion; AC-1, AC-2, and AC-N1–N3 pass, including literal overhang expectations `15.0`, `30.0`, `45.0`, and `60.0` from a `60.0` base.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-config/src/lib.rs` - registry definitions/accessors only
  - `crates/slicer-ir/src/{resolved_config.rs,slice_ir.rs}` - `ResolvedConfig::to_config_map`, `apply_cli_key`, `ConfigValue`, and equality/hash ranges only
  - `docs/22_test_quality.md` - independent-oracle and false-green sections via bounded SUMMARY
- Files allowed to edit (at most 3):
  - `crates/slicer-config/src/lib.rs`
  - `crates/slicer-config/tests/automatic_value_expansion_tdd.rs`
- Files explicitly out of bounds:
  - Runtime wiring, guest modules, emitter, WIT, schema/version constants, generated docs
- Blast-radius discipline:
  - `ExpansionContext` is net-new and has no struct-literal blast radius. Tests use FRU/default only if the type has five or more fields; its specified two-field shape does not trigger the watched-literal rule.
- Expected sub-agent dispatches:
  - Question: review the test expectations for self-referential or resolver-derived oracles; scope: the new test file; return: `FACT` pass/fail with offending test names only.
- Context cost: `M`
- Authoritative docs:
  - `docs/specs/config-scope-resolution-plan.md` - Expansion Phase B and oracle-independence requirement
  - `docs/adr/0067-unified-config-schema-registry.md` - typed base metadata
  - `docs/adr/0068-config-scope-is-a-wire-encoding.md` - evaluation placement
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/Flow.cpp` - delegated SUMMARY of `new_from_config_width`/`auto_extrusion_width`
  - `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` - delegated SUMMARY of the overhang family's `coFloatOrPercent`/`ratio_over = outer_wall_speed` declarations
  - `OrcaSlicerDocumented/src/libslic3r/GCode.cpp` - delegated SUMMARY distinguishing numeric-zero role fallback from volumetric auto
  - `OrcaSlicerDocumented/src/libslic3r/Support/SupportParameters.hpp` - delegated SUMMARY of bottom-layer mirror
- Verification:
  - `bash -lc 'set -euo pipefail; mkdir -p target; cargo test --all-targets -p slicer-config --test automatic_value_expansion_tdd -- --nocapture 2>&1 | tee target/test-output.log >/dev/null; rg -q "test result: ok" target/test-output.log'` - FACT pass/fail; SNIPPETS ≤20 lines on failure.
- Exit condition: all named positive and negative tests are listed and pass; deleting any one rule or changing any one typed base makes at least one literal assertion fail; an error leaves the original config unchanged.

### Step 3: Declare the support-width and speed-percent typed bases

- Task IDs: `TASK-565`
- Objective: Add `base_key = "nozzle_diameter"` to the existing `support_line_width` declaration; retype all four overhang-speed manifest/host declarations as `float_or_percent`; and make registry assembly retain `base_key = "outer_wall_speed"` for the speed family without changing numeric zero.
- Precondition: Step 2's engine consumes `RegistryEntry.base_key`; packet-1 registry assembly is green.
- Postcondition: real-manifest assembly succeeds; `support_line_width` carries exact `float_or_percent`/`nozzle_diameter` metadata; all four overhang entries carry exact `float_or_percent`/`outer_wall_speed` metadata across module and host channels; and each default remains numeric `0.0`.
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/tree-support-planner/tree-support-planner.toml` - `support_line_width` section only
  - `modules/core-modules/overhang-classifier-default/overhang-classifier-default.toml` - four overhang speed sections and `outer_wall_speed`
  - `crates/slicer-ir/src/feedrate.rs` - `SPEED_META`, `SPEED_KEYS`, `read_speed`, and `FeedrateConfig::from_raw_config` only
  - `crates/slicer-config/src/lib.rs` - registry assembly only
  - `crates/slicer-config/tests/{registry_assembly_tdd.rs,automatic_value_expansion_tdd.rs}` - affected fixtures only
- Files allowed to edit (at most 3 per atomic sub-step):
  - **3a:** `modules/core-modules/tree-support-planner/tree-support-planner.toml`, `crates/slicer-config/tests/automatic_value_expansion_tdd.rs`, `crates/slicer-config/tests/registry_assembly_tdd.rs`
  - **3b:** `modules/core-modules/overhang-classifier-default/overhang-classifier-default.toml`, `crates/slicer-ir/src/feedrate.rs`, `crates/slicer-config/tests/registry_assembly_tdd.rs`
- Files explicitly out of bounds:
  - Other manifests, the `SPEED_KEYS` tuple shape/function-pointer API, `FeedrateConfig` field types, ad hoc percentage resolution inside `read_speed`, emitter behavior, config-schema wire constants
- Blast-radius discipline:
  - No struct field or schema/version constant is added. Existing `HostKeyMeta.wire_type`, manifest `base_key`, and manifest `type` values change only; `SPEED_META` retains its positional `SPEED_KEY_COUNT` assertion and no struct-literal blast radius escapes `crates/slicer-ir/src/feedrate.rs`.
- Expected sub-agent dispatches:
  - Question: do the real support and four overhang declarations assemble as `support_line_width -> float_or_percent/nozzle_diameter` and each `overhang_*_speed -> float_or_percent/outer_wall_speed`, with host/module type agreement and zero defaults?; scope: the two manifests, `SPEED_META`, and registry test; return: `FACT` with key→type/base/default only.
- Context cost: `S`
- Authoritative docs:
  - `docs/specs/config-scope-resolution-plan.md` - typed base-key framework, RC-8's dropped percent-authored overhang defect, explicit Phase-B speed-percent ownership, and row 10's volumetric remainder
  - `docs/03_wit_and_manifest.md` - `base_key` manifest field section added by packet 1, ranged/delegated
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` - `PrintConfigDef::init_fff_params` overhang float-or-percent declarations and `outer_wall_speed` ratio base, delegated.
  - `OrcaSlicerDocumented/src/libslic3r/GCode.cpp` - numeric-zero role fallback versus volumetric auto, delegated.
- Verification:
  - `bash -lc 'set -euo pipefail; mkdir -p target; cargo test --all-targets -p slicer-config --test registry_assembly_tdd -- --nocapture 2>&1 | tee target/test-output.log >/dev/null; rg -q "test result: ok" target/test-output.log'` - FACT pass/fail.
  - `bash -lc 'set -euo pipefail; mkdir -p target; cargo test --all-targets -p slicer-config --test automatic_value_expansion_tdd tool_specific_base_wins_for_selected_tool -- --nocapture 2>&1 | tee target/test-output.log >/dev/null; rg -q "test tool_specific_base_wins_for_selected_tool \.\.\. ok" target/test-output.log'` - FACT pass/fail.
- Exit condition: registry assembly returns `Ok`; `support_line_width` resolves to `float_or_percent/nozzle_diameter`; every overhang quartile speed resolves to `float_or_percent/outer_wall_speed` with default `0.0`; and removing any one speed base/type declaration makes `phase_b_speed_percent_family_declares_typed_outer_wall_base` fail.

### Step 4: Wire Phase B through runtime and the configured prepass

- Task IDs: `TASK-565`
- Objective: Assemble one registry per run from packet-1 inputs, build `ExpansionContext`, expand global/object and the run-level emitter tool maps after their merges, preserve assembly warnings, overlay the expanded global map before plan binding/feedrate construction, and thread the same authority through `pipeline.rs` so `prepass.rs` expands the tool and paint-semantic maps it reconstructs internally. Add a registered integration regression that drives literal width and overhang-speed placeholders at global, object, tool, and paint scope through both production entry points.
- Precondition: Steps 2–3 are green; expansion metadata assembles from real modules.
- Postcondition: both entry points execute the same expansion-aware ordering and map expansion failures into actionable `SliceRunError`/`PrepassExecutionError` text; covered placeholders cannot reach plan binding, `FeedrateConfig::from_raw_config`, `commit_region_mapping_builtin`, or any `RegionMapIR::intern_config` result.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-runtime/src/run.rs` - `run_slice_with_collector`, `prepare_prepass_context`, and loaded-module diagnostics only
  - `crates/slicer-runtime/src/pipeline.rs` - `run_pipeline_with_raw_config`, `run_pipeline_with_instrumentation`, and `run_pipeline_core` prepass forwarding only
  - `crates/slicer-runtime/src/prepass.rs` - configured/collecting/instrumented forwarders, `execute_prepass_with_builtins_configured_instr_collecting`, `build_paint_semantic_configs`, tool resolution, and `commit_region_mapping_builtin` call only
  - `crates/slicer-runtime/Cargo.toml` - dependencies/features only
  - `crates/slicer-config/src/lib.rs` - public expansion/assembly signatures only
  - `crates/slicer-scheduler/src/{config_resolution.rs,execution_plan.rs}` - resolver return signatures and `bind_module_config_view` signature only; read-only
  - `crates/slicer-runtime/src/builtins/region_mapping_producer.rs` - `commit_region_mapping_builtin` map arguments only; read-only
  - `crates/slicer-core/src/algos/region_mapping.rs` - `execute_region_mapping_inner` scope fold and all `RegionMapIR::intern_config` branches only; read-only
  - `crates/slicer-runtime/tests/integration/main.rs` - integration module roster only
- Files allowed to edit (at most 3 per atomic sub-step):
  - **4a — run-level merge/binding:** `crates/slicer-runtime/Cargo.toml`, `crates/slicer-runtime/src/run.rs`. Assemble/preserve the registry outcome, construct context, expand global/object and the run-level emitter tool maps only after their current resolvers return, and construct the expanded global binding/feedrate source. Do not claim that this emitter tool map reaches RegionMapping.
  - **4b — prepass transport:** `crates/slicer-runtime/src/pipeline.rs`, `crates/slicer-runtime/src/run.rs`. Carry borrowed/shared expansion authority through every production raw-config and instrumented fork into configured prepass, and pass it directly from `prepare_prepass_context`, without adding a required field to public `PipelineConfig` (which would create unrelated struct-literal churn). Existing no-raw/default-only compatibility entry points may forward an explicit no-expansion authority only when their inputs cannot contain scoped placeholders.
  - **4c — prepass-owned maps:** `crates/slicer-runtime/src/prepass.rs`. Thread expansion authority through `execute_prepass_with_builtins_configured`, its collecting/instrumented forwarders, and the shared collecting core; expand/recheck the supplied default/object maps, then expand every internally resolved tool map with `Some(tool_index)` and every internally resolved paint-semantic map with no selected tool before `commit_region_mapping_builtin`. Convert failures to a named `PrepassExecutionError`; do not retain `unwrap_or_default`/empty-map fallback for expansion failures.
  - **4d — registered regression:** `crates/slicer-runtime/tests/integration/automatic_value_expansion_tdd.rs`, `crates/slicer-runtime/tests/integration/main.rs`. Author covered placeholders at global, `object_config:<id>:*`, `tool_config:1:*`, and `paint_config:material:*`; use material paint selecting tool 1 plus a non-support modifier child to cover all three current interning branches; drive both production entry points; inspect bound module values and every `RegionMapIR.configs` entry with literal expected absolutes.
- Files explicitly out of bounds:
  - Edits to `crates/slicer-scheduler/src/config_resolution.rs`, `crates/slicer-scheduler/src/execution_plan.rs`, `crates/slicer-runtime/src/builtins/region_mapping_producer.rs`, `crates/slicer-core/src/algos/region_mapping.rs`, emitter behavior, or WIT
- Blast-radius discipline:
  - No existing public struct field or schema constant changes. In particular, do not add expansion fields to `PipelineConfig`; use an expansion-aware internal/production call path so existing test literals do not become packet-local churn. Inline tests must use FRU/waiver for watched structs per `docs/21_data_defaults_and_fixtures.md`.
- Expected sub-agent dispatches:
  - Question: confirm each entry point's exact resolve→plan/prepass order, all early moves/borrows of loaded bindings, and the transport seam through `pipeline.rs`; then identify where `prepass.rs` rebuilds tool/paint maps before `commit_region_mapping_builtin`; scope: named symbols in `crates/slicer-runtime/src/{run.rs,pipeline.rs,prepass.rs}`; return: `SNIPPETS` ≤4 snippets, 30 lines each.
  - Question: confirm the painted fixture reaches the empty-chain, painted parent, and painted modifier-child interning branches where applicable, and that assertions inspect the whole `RegionMapIR.configs` table rather than one selected config; scope: the new integration test plus `execute_region_mapping_inner`; return: `FACT` ≤5 lines.
- Context cost: `M`
- Authoritative docs:
  - `docs/adr/0068-config-scope-is-a-wire-encoding.md` - after-merge/before-delivery ordering
  - `docs/02_ir_schemas.md` - `RegionMapIR::intern_config` invariant
- OrcaSlicer refs:
  - None; this step is host lifecycle wiring.
- Verification:
  - `bash -lc 'set -euo pipefail; mkdir -p target; cargo test --all-targets -p slicer-runtime --test integration automatic_value_expansion_tdd::runtime_expands_global_object_tool_and_paint_before_delivery -- --nocapture 2>&1 | tee target/test-output.log >/dev/null; rg -q "test automatic_value_expansion_tdd::runtime_expands_global_object_tool_and_paint_before_delivery \.\.\. ok" target/test-output.log'` - FACT pass/fail.
  - `cargo check --workspace --all-targets` - FACT pass/fail.
- Exit condition: the bounded test fails if expansion moves before a scope merge, after plan binding/feedrate construction, or after prepass hands maps to RegionMapping; if `prepass.rs` omits either its tool or paint loop; or if any bound/region-map config retains a covered placeholder. It asserts literal expanded overhang mm/s values as well as widths across global/object/tool/paint scopes, and both production entry points share one registry/context authority and configured-prepass ordering.

### Step 5: Route support width through expanded values

- Task IDs: `TASK-565`
- Objective: Delete `resolve_support_line_width_mm` and switch both host support-width consumers to the already-absolute `support_line_width` value without changing overhang feedrate handling.
- Precondition: Step 4 produces expanded configs before support consumer construction.
- Postcondition: no duplicate support-width function or production call remains, and no `FeedrateConfig` or overhang-speed code changes in this step.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-ir/src/resolved_config.rs` - `ResolvedFloatOrPercent` and `resolve_support_line_width_mm` only
  - `crates/slicer-runtime/src/run.rs` - feedrate/serializer construction and local support tests only
  - `crates/slicer-runtime/src/builtins/support_analysis_producer.rs` - support width helper only
- Files allowed to edit (at most 3 per atomic sub-step):
  - `crates/slicer-ir/src/resolved_config.rs`
  - `crates/slicer-runtime/src/run.rs`
  - `crates/slicer-runtime/src/builtins/support_analysis_producer.rs`
- Files explicitly out of bounds:
  - `crates/slicer-gcode/src/emit.rs`, unrelated support geometry, WIT/schema constants
- Blast-radius discipline:
  - No struct field is added. Before deleting the function, re-grep every name-resolution-equivalent `resolve_support_line_width_mm` import/call; all returned sites must be in 5a or implementation stops for plan revision.
- Expected sub-agent dispatches:
  - Question: list every import/call of `resolve_support_line_width_mm`; scope: `crates/`; return: `LOCATIONS` ≤20.
- Context cost: `S`
- Authoritative docs:
  - `docs/specs/config-scope-resolution-plan.md` - RC-8 and Phase B speed/width scope
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/Flow.cpp` - support auto width, delegated.
- Verification:
  - `bash -lc 'set -euo pipefail; mkdir -p target; cargo test --all-targets -p slicer-config --test automatic_value_expansion_tdd phase_b_expands_hand_computed_percent_zero_and_minus_one_cases -- --nocapture 2>&1 | tee target/test-output.log >/dev/null; rg -q "test phase_b_expands_hand_computed_percent_zero_and_minus_one_cases \.\.\. ok" target/test-output.log; ! rg -q "resolve_support_line_width_mm" crates'` - FACT pass/fail.
- Exit condition: support zero/percent cases use the expansion engine, both host consumers read the expanded support width, workspace source contains no `resolve_support_line_width_mm` symbol, and overhang feedrate behavior is untouched.

### Step 6: Reduce role-width fallback over expanded bases

- Task IDs: `TASK-565`
- Objective: Change `resolve_role_width` to bridge > first-layer > role > expanded `line_width`; remove the nozzle calculation and, if the Step-1 census proves no remaining semantic use, remove `RoleWidthContext.nozzle_diameter` with all literal fallout.
- Precondition: Step 4 guarantees positive expanded `line_width` before production role resolution.
- Postcondition: role tests preserve override precedence and prove zero role widths choose literal expanded base `0.45`; no role resolver computes `1.125 * nozzle_diameter`.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-core/src/flow.rs` - `RoleWidthContext` and `resolve_role_width` only
  - `crates/slicer-core/tests/flow_tdd.rs` - role-width tests only
  - `crates/slicer-runtime/src/slice_postprocess_prepass.rs` - `RoleWidthContext` literal only
  - `crates/slicer-core/src/algos/{lightning/mod.rs,paint_segmentation/mod.rs}` - `RoleWidthContext` literals only
- Files allowed to edit (at most 3 per atomic sub-step):
  - **6a:** `crates/slicer-core/src/flow.rs`, `crates/slicer-core/tests/flow_tdd.rs`
  - **6b (only if field removed):** `crates/slicer-runtime/src/slice_postprocess_prepass.rs`, `crates/slicer-core/src/algos/lightning/mod.rs`, `crates/slicer-core/src/algos/paint_segmentation/mod.rs`
- Files explicitly out of bounds:
  - Other flow formulas, beading strategies, geometry conversions, emitter
- Blast-radius discipline:
  - `RoleWidthContext` is a public watched struct. Step 1 must provide the complete literal census; every site is listed in 6a/6b, and tests retain their existing `// exhaustive:` waiver or valid FRU.
- Expected sub-agent dispatches:
  - Question: re-grep every `RoleWidthContext {` literal before editing and verify the 6a/6b set is complete; scope: `crates/`, `modules/`; return: `LOCATIONS` ≤20.
- Context cost: `S`
- Authoritative docs:
  - `docs/specs/config-scope-resolution-plan.md` - role dispatch over expanded bases
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/Flow.cpp` - precedence/auto behavior, delegated.
- Verification:
  - `bash -lc 'set -euo pipefail; mkdir -p target; cargo test --all-targets -p slicer-core --test flow_tdd role_width -- --nocapture 2>&1 | tee target/test-output.log >/dev/null; rg -q "test result: ok" target/test-output.log; ! rg -q "1\.125 \* context\.nozzle_diameter" crates/slicer-core/src/flow.rs'` - FACT pass/fail.
  - `cargo xtask check-literals` - FACT pass/fail.
- Exit condition: all role precedence tests pass, the new literal `0.45` fallback test passes, and no nozzle auto arithmetic remains in `resolve_role_width`.

### Step 7: Remove support guest mirror fallbacks

- Task IDs: `TASK-565`
- Objective: Delete the two bottom-layer and two bottom-spacing negative-mirror branches from production guests and update direct guest fixtures to provide host-expanded bottom values.
- Precondition: Step 4 proves host expansion precedes every production `ConfigView` and that prepass-rebuilt tool/paint maps are expanded before RegionMapping; Step 2 directly tests both `-1` rules.
- Postcondition: the four guests consume nonnegative expanded values and contain no production `< 0` mirror policy for the covered keys; direct tests remain meaningful without reimplementing the resolver oracle.
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/{traditional-support-planner,tree-support-planner,traditional-support,tree-support}/src/lib.rs` - from-config and mirror branches only
  - Each module's `tests/*.rs` - only fixtures containing the two bottom keys
  - `modules/core-modules/traditional-support-planner/Cargo.toml` - registered `traditional_family_tdd` target only
- Files allowed to edit (at most 3 per atomic sub-step):
  - **7a:** `modules/core-modules/traditional-support-planner/src/lib.rs`, `modules/core-modules/traditional-support-planner/tests/traditional_family_tdd.rs`
  - **7b:** `modules/core-modules/tree-support-planner/src/lib.rs`, `modules/core-modules/tree-support-planner/tests/tree_family_tdd.rs`, `modules/core-modules/tree-support-planner/tests/diagnostics_tdd.rs`
  - **7c:** `modules/core-modules/traditional-support/src/lib.rs`, `modules/core-modules/traditional-support/tests/traditional_support_tdd.rs`, `modules/core-modules/traditional-support/tests/traditional_family_tdd.rs`
  - **7d:** `modules/core-modules/tree-support/src/lib.rs`, `modules/core-modules/tree-support/tests/tree_support_tdd.rs`, `modules/core-modules/tree-support/tests/tree_family_tdd.rs`
- Files explicitly out of bounds:
  - Support geometry algorithms beyond the four mirror branches; other guest fallbacks owned by packet 6; packet-10 sentinel candidates
- Blast-radius discipline:
  - No struct field or schema constant changes. Test fixture edits must keep independent expected top/bottom counts or pitches; do not replace them with values computed by `expand_automatic_values` inside the guest test.
- Expected sub-agent dispatches:
  - Question: list direct-test fixtures containing `support_interface_bottom_layers = -1` or `support_bottom_interface_spacing = -1` and identify the shared fixture helper, if any; scope: four support module test trees; return: `LOCATIONS` ≤20.
- Context cost: `M`
- Authoritative docs:
  - `docs/specs/config-scope-resolution-plan.md` - no-placeholder delivery invariant
  - `docs/22_test_quality.md` - fixture/oracle independence, delegated SUMMARY
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/Support/SupportParameters.hpp` - bottom-layer mirror, delegated; production ownership moves host-side but behavior stays.
- Verification:
  - `bash -lc 'set -euo pipefail; mkdir -p target; cargo test --all-targets -p traditional-support-planner --test traditional_family_tdd expanded_bottom_interface_layer_count_is_consumed_without_guest_mirroring -- --nocapture 2>&1 | tee target/test-output.log >/dev/null; rg -q "test expanded_bottom_interface_layer_count_is_consumed_without_guest_mirroring \.\.\. ok" target/test-output.log'` - FACT pass/fail; this target is explicitly wired by `modules/core-modules/traditional-support-planner/Cargo.toml`.
  - `bash -lc 'set -euo pipefail; mkdir -p target; cargo test --all-targets -p tree-support-planner 2>&1 | tee target/test-output.log >/dev/null; rg -q "test result: ok" target/test-output.log'` - FACT pass/fail.
  - `bash -lc 'set -euo pipefail; mkdir -p target; cargo test --all-targets -p traditional-support 2>&1 | tee target/test-output.log >/dev/null; rg -q "test result: ok" target/test-output.log'` - FACT pass/fail.
  - `bash -lc 'set -euo pipefail; mkdir -p target; cargo test --all-targets -p tree-support 2>&1 | tee target/test-output.log >/dev/null; rg -q "test result: ok" target/test-output.log'` - FACT pass/fail.
- Exit condition: all four guest crates pass and a bounded source grep finds no negative mirror branch for either covered bottom key.

### Step 8: Add deterministic visual evidence, generated base-key support, and contract docs

- Task IDs: `TASK-565`
- Objective: Commit the bounded visual-debug request/config, render the Phase-B zero-width path, validate the required manifest fields/PNG, add `base_key` to `KeyRow`/module parsing/rendering with the inline `render_table_preserves_base_key` regression, document expansion placement, and regenerate the config-key reference.
- Precondition: Steps 2–7 are green and guest artifacts are fresh.
- Postcondition: AC-6 and AC-7 pass; docs state placement; the generator has a tested `Base key` column; and generated support plus all four overhang rows carry their typed bases; compatibility checklist remains no-bump.
- Files allowed to read, with ranges when over 300 lines:
  - `docs/19_visual_debug.md` - Request Shape, Reading A Bundle, Tap Classes only
  - `docs/02_ir_schemas.md` - `ResolvedConfig` and `RegionMapIR` sections only
  - `docs/15_config_keys_reference.md` - generated affected rows only
  - `xtask/src/gen_config_docs.rs` - `KeyRow`, `module_rows`, `host_table_rows`, `render_table`, and inline tests only
  - `crates/pnp-cli/tests/visual_debug_request_bundle_tdd.rs` - real request shape fixture only
- Files allowed to edit (at most 3 per atomic sub-step):
  - **8a:** `crates/pnp-cli/tests/fixtures/automatic_value_expansion/visual-debug.json`, `crates/pnp-cli/tests/fixtures/automatic_value_expansion/config.json`
  - **8b:** `xtask/src/gen_config_docs.rs`, `docs/02_ir_schemas.md`, `docs/15_config_keys_reference.md`
- Files explicitly out of bounds:
  - Visual-debug implementation/renderers, `resources/regression_wedge.stl`, IR/WIT/wire constants, other docs
- Blast-radius discipline:
  - `KeyRow` is private to `xtask/src/gen_config_docs.rs`; update both production constructors and every inline test literal in the same 8b edit. No public Rust struct or schema constant changes. The request uses schema `1.0.0`; no visual-debug schema bump is authorized.
- Expected sub-agent dispatches:
  - Question: run the exact AC-6 command and return only manifest field values plus pass/fail; scope: committed request and target bundle; return: `FACT` ≤5 lines.
  - Question: verify `KeyRow.base_key` is populated from module TOML and rendered in a stable `Base key` column, with all constructors updated and `render_table_preserves_base_key` proving the populated cell; scope: named symbols in `xtask/src/gen_config_docs.rs`; return: `FACT` ≤5 lines.
- Context cost: `S`
- Authoritative docs:
  - `docs/19_visual_debug.md` - request/manifest/closure contract ranges
  - `docs/02_ir_schemas.md` - resolved config/interner contract
  - `docs/11_operational_governance_and_acceptance_gate.md` - compatibility checklist
- OrcaSlicer refs:
  - None; visual evidence validates PnP's pipeline result.
- Verification:
  - Run AC-6 verbatim - FACT pass/fail with manifest fields only.
  - `bash -lc 'set -euo pipefail; mkdir -p target; cargo test --all-targets -p xtask render_table_preserves_base_key 2>&1 | tee target/test-output.log >/dev/null; rg -q "render_table_preserves_base_key .* ok" target/test-output.log; cargo xtask gen-config-docs --check 2>&1 | tee target/test-output.log >/dev/null; rg -q "Phase B automatic-value expansion" docs/02_ir_schemas.md; rg -q "support_line_width.*nozzle_diameter" docs/15_config_keys_reference.md; for key in overhang_1_4_speed overhang_2_4_speed overhang_3_4_speed overhang_4_4_speed; do rg -q "$key.*outer_wall_speed" docs/15_config_keys_reference.md; done'` - FACT pass/fail.
  - `bash -lc 'set -euo pipefail; mkdir -p target; cargo xtask build-guests --check 2>&1 | tee target/test-output.log >/dev/null'` - FACT exit 0/nonzero.
- Exit condition: deterministic fixture renders one or more real PNGs with all AC-6 manifest fields; deleting `KeyRow.base_key`, module parsing, its rendered cell, or any one populated generated row makes the inline/generator checks fail; docs greps pass; doc generation is clean; and guest freshness exits 0.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | S | Read-only FORWARD-DEP/base census. |
| Step 2 | M | Core API plus independent positive/negative tests. |
| Step 3 | S | Support base plus the four-member speed family, split into ≤3-edit sub-steps. |
| Step 4 | M | Runtime/pipeline/prepass seam, split into four ≤3-edit sub-steps; highest lifecycle risk. |
| Step 5 | S | Feedrate/support duplicate retirement. |
| Step 6 | S | Role resolver and bounded literal fallout. |
| Step 7 | M | Four guest families split into four ≤3-edit sub-steps. |
| Step 8 | S | Two fixtures, private generator support, and two docs in ≤3-edit sub-steps. |

Aggregate remains `M`; no individual step is `L`.

## Packet Completion Gate

- All steps and exits complete.
- Every pipe-suffixed AC command returns PASS.
- `cargo clippy --workspace --all-targets -- -D warnings` passes.
- `cargo xtask check-literals` passes.
- `cargo xtask check-test-quality --report` has no unaddressed finding in touched tests.
- `cargo xtask build-guests --check` exits 0 after any required rebuild.
- Update `docs/07_implementation_status.md` through a worker dispatch, never a full backlog read.
- Active packet 1 has completed and the formerly unsatisfied FORWARD-DEP is shape-compatible; packets 5 and 10 receive the exports in `task-map.md`, with packet 10 receiving no ownership of the already-expanded overhang percentages.
- `packet.spec.md` is ready for `status: implemented` without changing any public schema/version constant.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed AC and packet-level gate command; do not run `cargo test --workspace`.
- Read only `target/test-output.log` slices for failures; never rerun solely because console output was truncated.
- Record remaining packet-local risk, especially any percent entry excluded as geometry/context-dependent for packet 10; the four overhang-speed percentages must not appear in that exclusion list.
- Confirm context stayed at or below 150k standard, or at/below 300k only with a logged swarm ESCALATION; otherwise record a packet-authoring lesson.

All `cargo check`, `cargo clippy`, and `cargo test` invocations in gate and verification commands use `--all-targets` so test, bench, and example targets compile.
