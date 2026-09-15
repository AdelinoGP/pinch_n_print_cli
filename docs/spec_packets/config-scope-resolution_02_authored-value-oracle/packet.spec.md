---
status: draft
packet: config-scope-resolution_02_authored-value-oracle
task_ids:
  - TASK-563
backlog_source: docs/07_implementation_status.md
context_cost_estimate: M
copy_note: Approved config-scope-resolution plan Packet Queue row 2; this packet remains a deliberately red oracle and does not edit the queue or backlog.
---

# Packet Contract: authored-value-oracle

## Goal

Add a registry-derived executor oracle that independently decodes authored values from `resources/cube_4color.3mf`, drives every declaring module through a live scheduler plan in which that module survives claim deduplication, and fails red when the module's bound `ConfigView` does not contain the exact registry-typed authored value.

## Scope Boundaries

This packet adds one executor test module, registers it, and adds only the net-new `slicer-config` runtime dev-dependency. It reuses the already-present `zip`, `serde_json`, and `slicer-model-io` dev-dependencies. Production ingestion changes belong to packet 03; packet 02 closes with the principal oracle failing and its selector, population, and comparator controls passing.

## Prerequisites and Blockers

- Depends on: packet 01, `docs/spec_packets/config-scope-resolution_01_config-schema-registry/`, now `implemented` (landed); this was a forward dependency until its exact registry API landed.
- Unblocks: packet 03, which must turn the principal oracle green without weakening its derivation or assertions.
- Activation blockers: packet 01 must land and an independent packet preflight must pass.

## Acceptance Criteria

- **AC-1. Given** raw authored strings independently decoded from `Metadata/project_settings.config`, packet-01 registry entries, module ownership derived from loaded manifest schemas, and two live plans whose only selector difference is `wall_generator = "arachne"` versus `"classic"`, **when** `oracle_authored_values_reach_owning_module_config_views` runs, **then** it fails red, reports no `MISSING_MODULE`, reports neither `spiral_mode` nor `bridge_density` as a mismatch, and reports all seven exact owner/value observations below (presence+absence contract: all seven required lines must be present, with no `MISSING_MODULE` and no `spiral_mode`/`bridge_density` mismatch; extra mismatches from the same coercion bug do not fail AC-1): `skirt_loops`/`com.core.skirt-brim`/`"1"`/`Int(1)`/`Bool(true)`; `brim_width`/`com.core.skirt-brim`/`"0"`/`Float(0.0)`/`Bool(false)`; `filter_out_gap_fill`/`com.core.classic-perimeters`/`"0"`/`Float(0.0)`/`Bool(false)`; `tree_support_wall_count` at both `com.core.tree-support` and `com.core.tree-support-planner`/`"0"`/`Int(0)`/`Bool(false)`; and `support_interface_bottom_layers` at both `com.core.traditional-support-planner` and `com.core.tree-support-planner`/`"0"`/`Int(0)`/`Bool(false)`. | `bash -lc 'set -uo pipefail; mkdir -p target; set +e; cargo test -p slicer-runtime --all-targets --test executor ingestion_fidelity_oracle_tdd::oracle_authored_values_reach_owning_module_config_views -- --exact --nocapture 2>&1 | tee target/test-output.log >/dev/null; code=${PIPESTATUS[0]}; set -e; test "$code" -ne 0; rg -q "oracle_authored_values_reach_owning_module_config_views \.\.\. FAILED" target/test-output.log; for line in "MISMATCH key=skirt_loops module=com.core.skirt-brim authored=\"1\" expected=Int(1) delivered=Bool(true)" "MISMATCH key=brim_width module=com.core.skirt-brim authored=\"0\" expected=Float(0.0) delivered=Bool(false)" "MISMATCH key=filter_out_gap_fill module=com.core.classic-perimeters authored=\"0\" expected=Float(0.0) delivered=Bool(false)" "MISMATCH key=tree_support_wall_count module=com.core.tree-support authored=\"0\" expected=Int(0) delivered=Bool(false)" "MISMATCH key=tree_support_wall_count module=com.core.tree-support-planner authored=\"0\" expected=Int(0) delivered=Bool(false)" "MISMATCH key=support_interface_bottom_layers module=com.core.traditional-support-planner authored=\"0\" expected=Int(0) delivered=Bool(false)" "MISMATCH key=support_interface_bottom_layers module=com.core.tree-support-planner authored=\"0\" expected=Int(0) delivered=Bool(false)"; do rg -Fq "$line" target/test-output.log || exit 1; done; ! rg -q "MISSING_MODULE|MISMATCH key=spiral_mode |MISMATCH key=bridge_density " target/test-output.log; echo "PASS: exact red owner/value observations"'`
- **AC-2. Given** the production `prepare_prepass_context` path and the two selector inputs, **when** `oracle_selector_matrix_exposes_each_perimeter_owner` runs, **then** the arachne plan contains `com.core.arachne-perimeters` and not `com.core.classic-perimeters`, while the classic plan contains `com.core.classic-perimeters` and not `com.core.arachne-perimeters`; retained support-family modules remain available in both plans. | `bash -lc 'set -euo pipefail; mkdir -p target; cargo test -p slicer-runtime --all-targets --test executor ingestion_fidelity_oracle_tdd::oracle_selector_matrix_exposes_each_perimeter_owner -- --exact --nocapture 2>&1 | tee target/test-output.log >/dev/null; rg -q "oracle_selector_matrix_exposes_each_perimeter_owner \.\.\. ok" target/test-output.log; echo "PASS: live selector matrix"'`
- **AC-3. Given** the fixture, registry, object-level restatement set, and automatic-value predicate, **when** `oracle_population_derivation_controls` runs, **then** the population is non-empty; excludes undeclared `spiral_mode`, percent-authored `bridge_density = "100%"`, a real non-scalar list, a synthetic narrower-scope restatement, a synthetic integer `-1` sentinel, and a synthetic base-key zero; and includes `line_width = "0.45"`, `skirt_height = "3"`, `enable_support = "0"`, plus default-coincident `internal_bridge_angle = "0"` and `internal_bridge_flow = "1"`. | `bash -lc 'set -euo pipefail; mkdir -p target; cargo test -p slicer-runtime --all-targets --test executor ingestion_fidelity_oracle_tdd::oracle_population_derivation_controls -- --exact --nocapture 2>&1 | tee target/test-output.log >/dev/null; rg -q "oracle_population_derivation_controls \.\.\. ok" target/test-output.log; echo "PASS: derived population controls"'`
- **AC-4. Given** registry-directed expected variants, **when** `oracle_comparator_is_type_aware_negative_control` runs, **then** `Float(0.0)` versus `Bool(false)` and `Int(1)` versus `Bool(true)` are mismatches, while exact same-variant float and bool pairs match. | `bash -lc 'set -euo pipefail; mkdir -p target; cargo test -p slicer-runtime --all-targets --test executor ingestion_fidelity_oracle_tdd::oracle_comparator_is_type_aware_negative_control -- --exact --nocapture 2>&1 | tee target/test-output.log >/dev/null; rg -q "oracle_comparator_is_type_aware_negative_control \.\.\. ok" target/test-output.log; echo "PASS: type-aware comparator"'`
- **AC-5. Given** the executor aggregator, **when** the oracle filter is listed, **then** exactly the four named oracle tests are discoverable and the list is non-empty. | `bash -lc 'set -euo pipefail; mkdir -p target; cargo test -p slicer-runtime --all-targets --test executor ingestion_fidelity_oracle_tdd::oracle_ -- --list 2>&1 | tee target/test-output.log >/dev/null; for n in oracle_authored_values_reach_owning_module_config_views oracle_selector_matrix_exposes_each_perimeter_owner oracle_population_derivation_controls oracle_comparator_is_type_aware_negative_control; do rg -q "${n}: test" target/test-output.log || exit 1; done; test "$(rg -c ": test$" target/test-output.log)" -eq 4; echo "PASS: four oracle tests registered"'`

## Negative Test Cases

- **AC-N1. Given** the red-oracle contract, **when** the oracle source and principal test result are checked, **then** no `#[ignore]` or file/test feature gate suppresses it and the principal test still fails. | `bash -lc 'set -euo pipefail; ! rg -q "#\[ignore|#\!?\[cfg\(feature" crates/slicer-runtime/tests/executor/ingestion_fidelity_oracle_tdd.rs; mkdir -p target; set +e; cargo test -p slicer-runtime --all-targets --test executor ingestion_fidelity_oracle_tdd::oracle_authored_values_reach_owning_module_config_views -- --exact --nocapture 2>&1 | tee target/test-output.log >/dev/null; code=${PIPESTATUS[0]}; set -e; test "$code" -ne 0; rg -q "oracle_authored_values_reach_owning_module_config_views \.\.\. FAILED" target/test-output.log; echo "PASS: oracle remains unsuppressed and red"'`
- **AC-N2. Given** the no-roster rule, **when** the oracle source is inspected, **then** none of the five catalogued divergent key names occurs as a Rust string literal; those observations must arise from fixture, registry, and manifest derivation. | `bash -lc 'set -euo pipefail; ! rg -q "\"(skirt_loops|brim_width|filter_out_gap_fill|tree_support_wall_count|support_interface_bottom_layers)\"" crates/slicer-runtime/tests/executor/ingestion_fidelity_oracle_tdd.rs; echo "PASS: no catalogued-key roster in oracle source"'`

## Verification

- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `bash -lc 'set -uo pipefail; mkdir -p target; set +e; cargo test -p slicer-runtime --all-targets --test executor ingestion_fidelity_oracle_tdd::oracle_ -- --nocapture 2>&1 | tee target/test-output.log >/dev/null; code=${PIPESTATUS[0]}; set -e; test "$code" -ne 0; rg -q "oracle_authored_values_reach_owning_module_config_views \.\.\. FAILED" target/test-output.log; for n in oracle_selector_matrix_exposes_each_perimeter_owner oracle_population_derivation_controls oracle_comparator_is_type_aware_negative_control; do rg -q "$n \.\.\. ok" target/test-output.log || exit 1; done; echo "PASS: principal oracle red; three controls green"'`

## Authoritative Docs

- `docs/specs/config-scope-resolution-plan.md` — §Problem, §Ingestion, §Expansion, cross-cutting test requirements, and Packet Queue row 2.
- `docs/22_test_quality.md` — self-referential-oracle, vacuous-loop, roster, scaffolding, negative-control, and derivation rules.
- `docs/04_host_scheduler.md` — `Perimeter-generator selection (wall_generator dedup + spiral-vase fallback)` and `Support-generator selection`.
- `docs/adr/0067-unified-config-schema-registry.md` and `docs/adr/0068-config-scope-is-a-wire-encoding.md` — bounded registry and authored-value sections.
- `docs/07_implementation_status.md` — delegated TASK-563 lookup only; never read in full.

## Doc Impact Statement (Required)

- **Amended (registry retype fallout):** this packet now touches two wire-visible contracts beyond test surfaces — `ResolvedConfig`'s host `initial_layer_line_width` becomes `ResolvedFloatOrPercent` (schema `host_config_keys`/wire table now reports `float_or_percent`, changing host key rows on the config-schema wire) and `FeedrateConfig`'s `internal_bridge_speed` becomes `ResolvedFloatOrPercent` (the `SPEED_KEYS` wire table reports `float_or_percent`); both registry-assembly behaviors are owned here while the ingestion fix stays with packet 03. Verification: `rg -n 'initial_layer_line_width' crates/slicer-ir/src/resolved_config.rs` (declaration, flatten, `PartialEq`/`Hash`, resolver helper), `rg -n 'internal_bridge_speed' crates/slicer-ir/src/feedrate.rs` (`SPEED_KEYS` row, `read_speed`, resolver), `rg -n 'float_or_percent' crates/slicer-config/tests/registry_census_tdd.rs` (census follow-through), the generated host-speeds section of `docs/15_config_keys_reference.md` is affected by this retype: its host `internal_bridge_speed` row must move from type `float` to `float_or_percent`, and that section is regenerated via `cargo xtask gen-config-docs` with an optional per-key `type` honored before scalar inference — `docs/config/host-keys.toml` gains the per-key type and `xtask/src/gen_config_docs.rs` honors it; verification: `cargo xtask gen-config-docs --check` (exit 0) plus `rg -n 'internal_bridge_speed' docs/15_config_keys_reference.md` showing `float_or_percent` on the host-speeds row, and `cargo test -p slicer-config --test registry_census_tdd` (4 passed) plus `cargo test -p slicer-config --test registry_assembly_tdd` (21 passed) green alongside the oracle suites.

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- obey the shared absolute context bands: 120k reading budget with hand-off at 150k (standard); the extended band (240k reading / 300k hard stop) only via swarm's escalation protocol

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation (an extended-band run may carry a single L step only when `design.md` justifies why it cannot be split).
