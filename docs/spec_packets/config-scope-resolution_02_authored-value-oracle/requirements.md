# Requirements: authored-value-oracle

## Packet Metadata

- Grouped task IDs: `TASK-563`
- Backlog source: `docs/07_implementation_status.md`
- Packet status: `draft`
- Aggregate context cost: `M`

## Problem Statement

`read_3mf_project_settings` (`crates/slicer-model-io/src/loader.rs`) applies `coerce_string_to_config_value` before module schemas are available. Module-owned numeric strings `"0"` and `"1"` therefore reach `bind_module_config_view` as `ConfigValue::Bool` when the host declaration channel does not know their keys. The approved plan requires an independently derived ingestion-fidelity oracle that lands red under TASK-563 and becomes packet 03's non-negotiable correction gate.

The first draft could not observe every declared owner: fixture-authored `wall_generator = "arachne"` makes `dedup_same_claim_modules_with_wall_generator` remove `com.core.classic-perimeters`, so walking that one plan can never inspect classic's `filter_out_gap_fill`. This packet instead builds an arachne plan and a classic plan through `prepare_prepass_context`, proves the exact keep/drop pair, and selects the plan in which each manifest-derived owner is live.

## In Scope

- Add `crates/slicer-runtime/tests/executor/ingestion_fidelity_oracle_tdd.rs` with four tests named in `packet.spec.md`.
- Parse `Metadata/project_settings.config` directly with the existing `zip` and `serde_json` dev-dependencies for the independent raw-string expectation authority; fail loudly if the fixture, zip member, JSON object, or expected scalar shape is absent.
- Obtain the delivered source independently through the production `slicer_model_io::read_3mf_project_settings` path, then pass it through `slicer_runtime::run::prepare_prepass_context`.
- Load the live core manifests with `slicer_scheduler::load_modules_from_roots`; derive `ModuleDeclaration` inputs, registry entries, and a `key -> declaring module IDs` ownership map from those loaded schemas rather than a hand-authored roster.
- Build two production plans from identical mesh and production-decoded source maps except for `wall_generator`: the fixture-authored arachne value and a test-controlled classic value. Use the arachne plan for ordinary/arachne owners and the classic plan for `com.core.classic-perimeters`; report a missing exact owner as `MISSING_MODULE key=<key> expected_module=<module-id> selector=<arachne|classic>`.
- Derive each expected `ConfigValue` from the raw string plus `RegistryEntry.field_type` and `RegistryEntry.values`; compare exact variants, not numeric truthiness.
- Include default-coincident authored values. Equality with a declaration default is not an exclusion: the contract is authored-value delivery, and direct `ConfigView` variant comparison remains falsifiable even when a guest fallback would happen to produce the same magnitude.
- Exclude only values outside the row-2 contract: undeclared keys, non-scalar JSON, keys restated at an available narrower object scope, malformed values, and values that actually require automatic expansion (a `percent` value, a percent-marked `float_or_percent`, a base-key zero sentinel, or an integer `-1` sentinel).
- Add `mod ingestion_fidelity_oracle_tdd;` to the executor aggregator.
- Add `slicer-config = { path = "../slicer-config" }` under `crates/slicer-runtime/Cargo.toml` `[dev-dependencies]`.

## Out of Scope

- No production ingestion fix, coercion change, resolver change, scheduler change, module change, manifest change, WIT/IR change, or fixture edit.
- No new `zip` declaration: the exact deflate-only `zip = { version = "2", default-features = false, features = ["deflate"] }` entry already exists in `slicer-runtime` dev-dependencies.
- No new `serde_json` or `slicer-model-io` dependency; both already exist in `slicer-runtime` dev-dependencies.
- No automatic-value expansion; packet 04 owns expansion and this oracle excludes only authored placeholders that need it.
- No layer-range ingestion; if the fixture gains `Metadata/layer_config_ranges.xml` before packet 09, this packet stops for reconciliation rather than silently claiming narrower-scope coverage.
- No packet 01, 03, or 04 edits; no approved-plan queue or backlog edit by this packet author.

## Normative Population and Observation Rule

For each scalar string `(key, raw)` in the project sidecar:

1. `ConfigSchemaRegistry::entry(key)` must exist.
2. At least one loaded module schema must declare `key`; host-only registry keys are not module-owner observations.
3. No loaded object config may restate `key`; the derived restatement set comes from `MeshIR.objects[*].config.data`.
4. `derive_expected_value(raw, entry)` must produce the exact registry variant and validate enum membership through `RegistryEntry.values`.
5. `requires_automatic_expansion(raw, entry)` must be false. This excludes actual percent semantics and sentinels, not the whole `float_or_percent` family: an absolute nonzero `float_or_percent` remains eligible.
6. Declaration-default coincidence does not affect eligibility.
7. For each declaring module ID, choose a live plan deterministically: classic owners use the classic selector plan; arachne and every other owner use the arachne selector plan. Find the exact ID before reading its `config_view()`; never infer ownership merely because some other live binding carries the key.
8. Compare `ConfigView::get(key)` to the independently derived expected variant. Accumulate sorted diagnostics so one red run exposes all failures.

## Grounded Contract Facts

- Packet 01's complete promised `RegistryEntry` shape is `{ pub key: String, pub field_type: String, pub default: Option<String>, pub min: Option<f64>, pub max: Option<f64>, pub values: Option<Vec<String>>, pub denied_scopes: Vec<String>, pub selector: bool, pub base_key: Option<String>, pub host_meta: Option<slicer_ir::resolved_config::HostKeyMeta>, pub module_meta: Option<ModuleKeyMeta>, pub provenance: Vec<String> }`.
- The live fixture authors `wall_generator = "arachne"`, `skirt_loops = "1"`, `brim_width = "0"`, `filter_out_gap_fill = "0"`, `tree_support_wall_count = "0"`, `support_interface_bottom_layers = "0"`, `bridge_density = "100%"`, `line_width = "0.45"`, `internal_bridge_angle = "0"`, and `internal_bridge_flow = "1"` in `Metadata/project_settings.config`.
- Loaded manifests declare `filter_out_gap_fill` only for `com.core.classic-perimeters`; `tree_support_wall_count` for `com.core.tree-support` and `com.core.tree-support-planner`; and `support_interface_bottom_layers` for `com.core.traditional-support-planner` and `com.core.tree-support-planner`.
- `dedup_same_claim_modules_with_wall_generator` (`crates/slicer-scheduler/src/execution_plan.rs`) keeps exactly one perimeter generator according to `wall_generator`; support renderer/planner family candidates are explicitly retained.
- `prepare_prepass_context` (`crates/slicer-runtime/src/run.rs`) calls the live loader with the supplied config source before `build_live_execution_plan`, so each returned compiled module carries `bind_module_config_view`'s production-filtered values.

## Authoritative Docs

- `docs/specs/config-scope-resolution-plan.md` — packet-2 oracle rule and authored/automatic-value boundaries.
- `docs/22_test_quality.md` — independent expectation, non-vacuous population, no roster, no scaffolding-only assertion, and comparator negative control.
- `docs/04_host_scheduler.md` — perimeter dedup and retained support-family candidates.
- `docs/adr/0067-unified-config-schema-registry.md` and `docs/adr/0068-config-scope-is-a-wire-encoding.md` — registry and authored-value authority.
- `CONTEXT.md` — authored value, automatic value, config scope, and config schema registry vocabulary.
- `docs/07_implementation_status.md` — delegated TASK-563 lookup only.

## Acceptance Summary

- Positive: `AC-1` proves the red principal contract with exact module IDs and exact expected/delivered variants; `AC-2` proves both perimeter owners are genuinely driveable through production dedup; `AC-3` proves population inclusions/exclusions including default coincidence; `AC-4` proves comparator sensitivity; `AC-5` prevents silent non-registration.
- Negative: `AC-N1` prevents suppression of the red test; `AC-N2` prevents a five-key source roster.
- Cross-packet impact: packet 03 must make `AC-1` green without editing the expectation, ownership derivation, selector matrix, or comparator.

## Verification Commands

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `bash -lc 'set -uo pipefail; mkdir -p target; set +e; cargo test -p slicer-runtime --all-targets --test executor ingestion_fidelity_oracle_tdd::oracle_authored_values_reach_owning_module_config_views -- --exact --nocapture 2>&1 \| tee target/test-output.log >/dev/null; code=${PIPESTATUS[0]}; set -e; test "$code" -ne 0; rg -q "oracle_authored_values_reach_owning_module_config_views \.\.\. FAILED" target/test-output.log; for line in "MISMATCH key=skirt_loops module=com.core.skirt-brim authored=\"1\" expected=Int(1) delivered=Bool(true)" "MISMATCH key=brim_width module=com.core.skirt-brim authored=\"0\" expected=Float(0.0) delivered=Bool(false)" "MISMATCH key=filter_out_gap_fill module=com.core.classic-perimeters authored=\"0\" expected=Float(0.0) delivered=Bool(false)" "MISMATCH key=tree_support_wall_count module=com.core.tree-support authored=\"0\" expected=Int(0) delivered=Bool(false)" "MISMATCH key=tree_support_wall_count module=com.core.tree-support-planner authored=\"0\" expected=Int(0) delivered=Bool(false)" "MISMATCH key=support_interface_bottom_layers module=com.core.traditional-support-planner authored=\"0\" expected=Int(0) delivered=Bool(false)" "MISMATCH key=support_interface_bottom_layers module=com.core.tree-support-planner authored=\"0\" expected=Int(0) delivered=Bool(false)"; do rg -Fq "$line" target/test-output.log \|\| exit 1; done; ! rg -q "MISSING_MODULE\|MISMATCH key=spiral_mode \|MISMATCH key=bridge_density " target/test-output.log; echo "PASS: exact red owner/value observations"'` | AC-1 exact red result, seven owner/value lines, and excluded/missing-owner silence | FACT pass/fail; ≤20 relevant lines on failure |
| `bash -lc 'set -euo pipefail; mkdir -p target; for n in oracle_selector_matrix_exposes_each_perimeter_owner oracle_population_derivation_controls oracle_comparator_is_type_aware_negative_control; do cargo test -p slicer-runtime --all-targets --test executor "ingestion_fidelity_oracle_tdd::$n" -- --exact --nocapture 2>&1 \| tee target/test-output.log >/dev/null; rg -q "$n \.\.\. ok" target/test-output.log; done; echo "PASS: three controls green"'` | AC-2 through AC-4 production selector, population, and comparator controls | FACT pass/fail |
| `bash -lc 'set -euo pipefail; mkdir -p target; cargo test -p slicer-runtime --all-targets --test executor ingestion_fidelity_oracle_tdd::oracle_ -- --list 2>&1 \| tee target/test-output.log >/dev/null; for n in oracle_authored_values_reach_owning_module_config_views oracle_selector_matrix_exposes_each_perimeter_owner oracle_population_derivation_controls oracle_comparator_is_type_aware_negative_control; do rg -q "${n}: test" target/test-output.log \|\| exit 1; done; test "$(rg -c ": test$" target/test-output.log)" -eq 4'` | AC-5 exact registration count and names | FACT pass/fail |
| `bash -lc 'set -euo pipefail; ! rg -q "#\[ignore\|#\[cfg\(feature" crates/slicer-runtime/tests/executor/ingestion_fidelity_oracle_tdd.rs; ! rg -q "\"(skirt_loops\|brim_width\|filter_out_gap_fill\|tree_support_wall_count\|support_interface_bottom_layers)\"" crates/slicer-runtime/tests/executor/ingestion_fidelity_oracle_tdd.rs'` | AC-N1/AC-N2 static suppression and roster guards; AC-N1's red execution is covered by the first row | FACT pass/fail |
| `cargo check --workspace --all-targets` | Compile all production and test targets | FACT pass/fail |
| `cargo clippy --workspace --all-targets -- -D warnings` | Lint gate | FACT pass/fail |
| `cargo xtask check-literals` | Watched-literal gate | FACT pass/fail |
| `cargo xtask check-test-quality --report` | Touched-test quality review | FACT findings/no findings |

## Step Completion Expectations

- The packet closes only when the principal test fails for mismatched values, while all three control tests pass.
- Any `MISSING_MODULE` is a test-harness failure, not an acceptable red result.
- Reconcile packet 01's landed exports before editing; packet 01 owns those names and shapes.
- Capture `target/test-output.log` before any subsequent test invocation overwrites it.

## Context Discipline Notes

- Never load the binary fixture into context; inspect only bounded programmatic facts.
- Use symbol-targeted ranges for `prepare_prepass_context`, `dedup_same_claim_modules_with_wall_generator`, `bind_module_config_view`, `CompiledModuleStatic`, and `read_3mf_project_settings`.
- Delegate `docs/07_implementation_status.md` and all cargo commands with bounded returns.
