# Test-Quality Remediation Plan

**Status:** program plan (waves 0/A complete; crate waves pending).
**Prepared:** 2026-09-09 from `tmp/test_audit_consolidated.md` (M1–M5, gitignored
and perishable — this file is self-contained and replaces it as the evidence
home). **Companion docs:** `docs/22_test_quality.md` (authoring standard),
`docs/adr/0064-existing-tests-retire-if-unjustified.md` (retirement standard),
`docs/adr/0065-test-quality-gate-with-delayed-enforce-mode.md` (enforcement
rollout), CONTEXT.md (vocabulary).

---

## 1. Program goal and non-negotiables

Two goals, one program:

1. **Execute the audit:** every work item below reaches a disposition (FIX,
   KEEP with evidence, DEFER with the blocking question), and every FIX is
   implemented with its survivor map and verification command recorded.
2. **Raise the authoring floor:** `docs/22_test_quality.md` + the
   `check-test-quality` gate (report mode now, enforce mode at program close)
   + AGENTS.md/doc-index/CONTEXT.md hooks make the false-green patterns
   detectable and the legitimate weak forms explicit.

Non-negotiables, inherited from `AGENTS.md` and binding on every wave:

- **Canonical parity supremacy.** Self-captured baselines may stay red; parity
  families (audit §7) are protected and only ever strengthened. Never loosen a
  tolerance or alter production output to make a test pass.
- **Census reconciliation.** Every retirement/merge is an accounted delta
  against the Wave-0 census. A binary-count drop between two runs is a
  reconciliation failure until explained.
- **Feature-correct invocations.** 41 of the 329 census targets are
  feature-gated (`slicer-core` 19 × `host-algos` [1 × also
  `voronoi-panic-regression`], `slicer-sdk` 21 × `test` [self dev-dep], `pnp-cli`
  1 × `integrated-classic-perimeters`). A narrow run without the feature is
  blind; verify against `docs/specs/test-quality-remediation-census.json`.
- **Freshness gate.** Wave 0 fixed staleness (47 guests rebuilt, lockfiles
  synced 2026-09-09, `cargo xtask build-guests --check` exit 0). Any wave
  failing guest/component/module-dispatch tests re-runs `--check` before
  attributing the failure.
- **Symbol-cited evidence.** Every claim in this file cites crate-qualified
  symbols. Re-derive before acting; the audit itself demonstrated symbol drift
  (`slice_ir_schema_version_is_4_8` was actually `slice_ir_schema_version_is_4_9`).
- **Log discipline.** Every test command tees to `target/test-output.log`;
  findings are read from the log, never by re-running.

## 2. Wave 0 — census baseline (recorded)

**Freshness:** fixed in this session. `--sync-locks` synced 43 guest lockfiles
(4 root-workspace members correctly skipped: `sdk-finalization-guest`,
`sdk-layer-infill-guest`, `sdk-postpass-text-guest`, `sdk-prepass-guest`);
47 guests rebuilt; final `--check` returned exit 0.

**Discovery census (this session, from `cargo metadata --no-deps`):**
35 workspace members carry test targets; 9 members carry none
(`slicer-integrated-modules`, `pnp-cli-locator`, `witness`, `raft-default`,
`sdk-finalization-guest`, `sdk-layer-infill-guest`, `sdk-postpass-text-guest`,
`sdk-prepass-guest`, `xtask`). Full machine-readable manifest, committed at
`docs/specs/test-quality-remediation-census.json` (crate →
[{target, required-features}]; regenerate any time via the same metadata +
`[[test]]`-stanza parse). Totals at plan time (per-target counts, not executed
tests):

| Crate | test targets | gated |
|---|---|---|
| slicer-core | 86 | 19 `host-algos` (1 also `voronoi-panic-regression`) |
| slicer-sdk | 28 | 21 `test` |
| pnp-cli | 23 | 1 `integrated-classic-perimeters` |
| slicer-ir | 20 | — |
| slicer-gcode | 14 | — |
| slicer-model-io | 13 | — |
| slicer-runtime | 17 | — |
| pnp-cli (total) | 23 | — |
| slicer-macros | 7 | — |
| slicer-scheduler | 7 | — |
| slicer-wasm-host | 5 | — |
| slicer-helpers | 4 | — |
| slicer-schema | 1 | — |
| 22 modules/core-modules guests | 84 | — |
| **Total** | **329** | **41** |

Note: the audit's `support_family_routing` registration finding (REG) means the
runtime census above (17) may already double-count one file — Wave 1 for
slicer-runtime resolves it and records the correction in the ledger.

**Execution census (deferred, per-crate):** per-test-fn `-- --list` counts are
captured at the start of each crate's wave and reconciled at its close. The
discovery census (above, committed) is the durable anchor; the per-target
execution census is wave-local evidence teed to `target/test-output.log`.

## 3. Wave A (this wave): docs + gate in report mode — **complete**

| # | Deliverable | Status |
|---|---|---|
| A1 | Guest freshness exit 0 | **done** |
| A2 | Discovery census manifest | **done** (`docs/specs/test-quality-remediation-census.json`) |
| A3 | `CONTEXT.md` terms | **done** (6 taxonomy terms + Test-quality gate + Earn-their-keep review) |
| A4 | `docs/22_test_quality.md` | **done** |
| A5 | ADR-0064, ADR-0065 | **done** |
| A6 | AGENTS.md Test Discipline hook, doc-index entry | **done** |
| A7 | `cargo xtask check-test-quality` (R1/R3/R6/R8 mechanical + R2/R4/R5/R7 review-guided, report mode, waivers, `xtask/src/test.rs` preflight hook + `TEST_QUALITY_ENFORCED` const) | **done** |
| A8 | Gate self-test fixtures in `xtask` (each rule's detect + waive + false-positive-avoidance case) | **done** (11 tests in `xtask/src/check_test_quality.rs`) |
| A9 | `check-literals`-style CLI: `check-test-quality [--report] [PATHS...]`, USAGE text | **done** |

**Wave A exit (met):** gate findings print and exit 0 in report mode; `cargo
test -p xtask` 143/143; `cargo clippy -p xtask --all-targets -- -D warnings`
clean; the preflight runs the gate in report mode without blocking
(`cargo xtask test` verified the full pipeline: check-literals →
check-test-quality → guest freshness → pnp_cli rebuild → tests).
**Recorded:** gate reports 12 findings in 7 files — all real
audit-confirmed patterns (2× `assert!(true)` in `slicer-macros/tests/module_test_tdd.rs`,
5 silent fixture-skips, 3 ungated sleeps, 1 duplicate role-twin in
`slicer-wasm-host/src/host.rs`), now the first work of each owning crate wave.
False-positive calibrations made during A7: field-vs-local name shadowing
(`.object_id == object_id` is meaningful — right-ident `.`-prefix suppression),
absence-of-output helpers (`assert_no_bundle_written`'s `if !exists { return }`
is negative-path control flow — R3 fires only inside `#[test]` fns), and
`std::thread::sleep` as `Expr::Call` (associated function, not method call).
Rules R2/R4/R5/R7 remain review-guided (their syntactic detection needs
per-fn body analysis that the first crate waves will calibrate); the doc table
in `docs/22_test_quality.md` records implementation status per rule.

## 4. Crate waves (order and per-wave contract)

Order: foundation-up, cross-crate items travel with their primary crate.

```
slicer-core → slicer-ir → slicer-model-io → slicer-gcode → slicer-helpers →
slicer-schema → slicer-macros → slicer-sdk → slicer-scheduler →
slicer-wasm-host → slicer-runtime → pnp-cli → final wave (PARITY + promotion)
```

Each crate wave's **exit gate**:

1. `cargo xtask build-guests --check` exit 0 (only if guest tests were touched).
2. Narrow feature-correct test runs for every touched target (§6 patterns),
   before/after counts reconciled against the census.
3. `cargo check --workspace --all-targets` clean.
4. `cargo clippy --workspace --all-targets -- -D warnings` clean.
5. `cargo xtask check-literals` clean.
6. `cargo xtask check-test-quality` report-mode findings for this crate = 0
   (fixed or waived with reasons).
7. Ledger row updated: state, retired/changed symbols, surviving coverage,
   validation command, remaining gap.

## 5. Work items by crate

Item IDs are the audit's (§4–§6 of `tmp/test_audit_consolidated.md`, which
sections 5.1–5.12 below supersede as evidence home). Each item's disposition is
FIX / KEEP-review (earn-their-keep pass required before any retirement) / DEFER.
Symbols quoted from the audit were spot-verified during consolidation; every
wave re-verifies its own items against disk before editing (audit §8 protocol).

### 5.1 Wave: slicer-core (19 gated targets → run `--features host-algos`)

| ID | Item | Disposition |
|---|---|---|
| DUP-CORE | algorithm-overlap merges: `bridge_false_site_gating_tdd.rs` `fully_supported_candidate_rejected_zero_bridge_area` → `solid_underneath_span_produces_no_bridge_area`; `algo_region_mapping_tdd.rs` chain/cross-product pairs; `support_overhang_detection_tdd.rs` coplanar step; `algo_support_geometry_tdd.rs` ↔ `support_geometry.rs` inline twins; `wall_sequence_reorder_tdd.rs` ↔ `perimeter_utils.rs`; `flow.rs` ↔ `tests/flow_tdd.rs` (canonical spacing/threshold/error/roundtrip matrix); `segment_path_preserves_requested_endpoints` fold; `point_to_segment_distance_squared_matches` review; `beading_factory.rs` threshold propagation | FIX (merge preserving union of cases) |
| DUP-CORE-strengthen | `bridging_angle_is_deterministic` (independent angle), `flow_correction_stays_positive_for_vertical_input` (finite fallback), `triangle_intersect.rs` crossing endpoints, `polygon_ops_tdd.rs` boolean geometry distinction, `wider_bead_spacing_is_larger_than_canonical` oracle review | FIX |
| RETIRE | `clip_operation_variants_are_distinct` (`polygon_ops.rs`), `slice_closing_radius_zero_is_noop` (`triangle_mesh_slicer_tdd.rs`), `flow.rs` inline-vs-test consolidation rows | FIX (earn-their-keep each) |
| PAINT | `paint_segmentation/mod.rs` 3 vacuous drivers; `voronoi_prune.rs` `prune_one_arc_preserves_border_nodes`; `voronoi_graph.rs` `from_colored_lines_twin_propagation_is_question_mark`; dependency probes (`vertex_color_get_set_round_trip`, `edge_is_primary_callable`, `boostvoronoi_supports_line_segment_sites`) | FIX (probes: KEEP-review) |
| BRITTLE | timing gates: `mesh_analysis.rs` `compute_xy_footprint_is_fast_for_thousands_of_disjoint_facets`, `overhang_annotation.rs` `annotate_overhangs_is_fast_for_many_stacked_layers`, `algo_prepass_slice_tdd.rs` `prepass_slice_caches_bottom_surface_footprint_across_layers` | KEEP-review (move to benchmark or deterministic accounting; do not drop a perf guard without replacement) |
| CROSS | SDK simplify/offset/raycast/bounds wrappers vs core (KEEP distinct implementations) | KEEP-review |
| PARITY | additive strengthening (beding side table radius boundary, transitions perpendicular foot, rib-split geometry, node distances, dumbbell topology, odd-cap boundary, junction reachability, module fallback, postprocess order divergence fixture) | FIX (additive; never remove exact pins) |

### 5.2 Wave: slicer-ir

| ID | Item | Disposition |
|---|---|---|
| RETIRE | `test_infill_type`, `slice_ir_schema_version_is_one_one_zero`, review `test_types_ids` compiled uses (`ir_tests.rs`); `flat_layer_contract_remains_unchanged` (`ir_validation_tdd.rs`) | FIX |
| SCHED/IR | schema-pin consolidation (`schema_version_is_current`, `slice_ir_schema_version_is_4_9`, `bridge_detector_schema_versions_are_constant_sourced`); `material_boundary_widening_tdd.rs` roundtrip vs segments; `paint_value_hash_tdd.rs` consistency; `slice_ir.rs` `get_abs_value_*`/`get_float_on_*` parameterization; `fill_holder_cli_binding_tdd.rs` short-name vs CLI binding | FIX |
| SCHED | `entity_id_invariants_tdd.rs` merge monotonic+collision (preserve `assert_not_impl_any!`), repair `unique_per_layer_and_resolvable` | FIX |

### 5.3 Wave: slicer-model-io

| ID | Item | Disposition |
|---|---|---|
| DUP-MODEL | `world_z_canonical_surface_tdd.rs` ↔ `loader.rs` inline twins; `threemf_project_settings_extraction_tdd.rs` fixture-skippable duplicate; writer roundtrip consolidation; `paint_studio_output_tdd.rs` strengthening | FIX |
| PAINT | `model_loader_tdd.rs` `support_enforcer_and_multi_layer_paint_from_real_3mf_fixture_documented_gap`: retire-or-implement, intent must move to a durable tracker | FIX |
| RETIRE | `pipeline_config_accepts_mesh_ir` (verify compile witness first) | KEEP-review |
| WORLD-Z | locate `identity_object_at_z_zero_passes` / `object_above_floor_is_valid` / `large_negative_z_translation_is_rejected` successors (file absent); re-derive floor policy vs `docs/08_coordinate_system.md` | FIX (locate-first) |

### 5.4 Wave: slicer-gcode (+ DUP-GCODE)

| ID | Item | Disposition |
|---|---|---|
| DUP-GCODE | `gcode_relative_extrusion_tdd.rs` rejects→positive pairs; `gcode_flavor_dialect_tdd.rs` `default_is_marlin`; `finalization_aware_travel_tdd.rs` wipe-tower order; `m73.rs` density fallback; `thumbnail_formats_tdd.rs` tag/framing | FIX |
| EMISSION | `gcode_feedrate_emission_tdd.rs` `rejects_only_retract_speed` / `rejects_stale_f_window`: drive the real emitter or shared oracle + corruption negative control | FIX |
| §2 twin | remove `emit.rs` duplicate of `base_interface_role_maps_to_support_interface_marker` (crate-root owner in `lib.rs` retained, `--exact` acceptance comment preserved); review `default_gcode_emitter_stores_slicer_version` | FIX |

### 5.5 Wave: slicer-helpers

| ID | Item | Disposition |
|---|---|---|
| RETIRE | `smoke_named_mesh_construction`, `smoke_step_warning_variants` (`src/lib.rs`); trim `smoke_all_types_accessible` self-equality; `decimate.rs` `dp_zero_tolerance_is_identity` + `dp_non_positive_tolerance_is_identity` merge | FIX |
| DUP | `decimate_conflict_config_error` (tests) vs `build_errors_when_both_targets_set` (lib) — keep library rejection + separate CLI conflict coverage | FIX |

### 5.6 Wave: slicer-schema (+ FRESH-adjacent stage table)

| ID | Item | Disposition |
|---|---|---|
| BRITTLE | `stage_table_has_one_entry_per_routed_export` (`src/lib.rs`): keep ABI/export contract, add coverage/uniqueness derivation | FIX |

### 5.7 Wave: slicer-macros

| ID | Item | Disposition |
|---|---|---|
| MACRO | `module_test_tdd.rs` policy assertions → real rejected-input compile tests; repair `test_14_state_isolation`; real panic-hook evidence for `test_06*`; review decorative fixtures; `variant_chain_boundary_tdd.rs` `custom_variant_chain_is_skipped_without_panicking` → correct production boundary (resolve §2 direction first); `finalization_mutation_roundtrip_tdd.rs` `drain_back_forwards_merge_ops` source-grep replacement; `binding_surface_tdd.rs` determinism trim (preserve export-name pin), associated-const review | FIX |
| BRITTLE | `all_worlds_glue_tdd.rs` / `postpass_text_glue_tdd.rs` migration guards (preserve all-world delegation/typed-config/WIT/registration/flat-vertex/placeholder-shim exclusion; strengthen `macro_inline_wit_configures_typed_config_view_resource`) | KEEP-review |

### 5.8 Wave: slicer-sdk (21 gated targets → self dev-dep `test` feature)

| ID | Item | Disposition |
|---|---|---|
| SDK | view-contract groupings (`layer_module_tdd.rs` 12–22), constructor-vs-catchup pairs (`prepass_module_tdd.rs`), payload conversion (`postpass_module_tdd.rs`), `finalization_module_tdd.rs` accessors, `finalization_builder_tdd.rs` merge (preserve `speed_profiles.is_empty()` distinction), tool-change consolidation, config-view builder table, view-builder settables incl. last-write-wins, perimeter builder both IDs, argument-threading reviews | FIX (never drop a covered field or input variant) |
| RETIRE | `test_support_fixture_bases_tdd.rs` FRU-overridden-field assertions; `test_support_output_capture_tdd.rs` debug_impl trio (keep capture/reset/drain + Default witness); `test_support_print_entity_tdd.rs` namespace-distinct (preserve export witness) | FIX |
| §2 | `_a_first_install_leaves_mesh_source_dirty` retire, `_b_second_test_sees_cleared_state_after_reset` keep | FIX |
| Prelude | preserve `test_07/test_30/test_15/test_15b/test_14/test_14b` prelude witnesses; review `test_32_multiple_implementations_coexist`, `smoke.rs` `prelude_reexports_are_available`; coordinate-system constant pin | KEEP-review |
| Migration guards | `closure_api_is_fully_removed`, `contour_points_api_is_fully_removed` | KEEP until compile-fail equivalents exist |
| CROSS | host wrappers vs wasm-host simplify: KEEP both | KEEP-review |

### 5.9 Wave: slicer-scheduler

| ID | Item | Disposition |
|---|---|---|
| SCHED | `manifest.rs` builder parameterization; `execution_plan.rs` support_type family (retain unknown fallback + candidates-vs-selection distinction); `stage_canon_seam_support_tdd.rs` parameterize; `stage_order_tdd.rs` rename→ordering assertion; `stage_order.rs` coverage/uniqueness derivation; `execution_plan_tdd.rs` boundary merge; `config_bounds_enforcement_tdd.rs` two-route retention; `validation.rs` fold; `dag_cli.rs` merge | FIX |
| BRITTLE | `manifest_ingestion_tdd.rs` roster → derived discovery + fixture classification; `config_resolution_tdd.rs` explicit-config; `module_manifest_tdd.rs` arc tolerance (verify canonical before loosening); `execution_plan_tdd.rs` error-display word-disjunction review | FIX / KEEP-review |

### 5.10 Wave: slicer-wasm-host

| ID | Item | Disposition |
|---|---|---|
| REG (travels here) | retire 4 forwarding wrappers in `tests/contract/main.rs` (owners in their own files; preserve bodies; fix `--exact` filters) | FIX |
| §2 | typed-config boundary batch KEEP (verified); role-twins in `host.rs` merge (`layer_role_tests` / `finalization_impls` twins), retain all builtin tags | FIX |
| BRITTLE | instance/engine tests trims (`engine_creates_successfully` Debug proxy, `host_state_preserves_module_id` review); keep structured error/compile/propagation coverage; edge-case empty/missing/ownership probes KEEP-review; `production_classic_perimeters_instantiates_with_layer_linker` add dispatch evidence | FIX / KEEP-review |
| CROSS | clip trio ↔ `polygon_ops.rs` (verify vertex assertions' geometric validity; prefer area/bounds/shape); `cross_family_body_overlap` REVIEW (different object scopes); foreign-language probe KEEP as opt-in tooling | KEEP-review |
| ACCOUNTING | `wasm_instance_pool_tdd.rs` serialized-pools synchronization | FIX |

### 5.11 Wave: slicer-runtime (heaviest; REG + FRESH + RAY + WORLD-Z + PRECEDENCE + SUPPORT + EMISSION + DISPATCH + DUP-RUNTIME + ACCOUNTING + macro-CLI-adjacent items)

| ID | Item | Disposition |
|---|---|---|
| REG | `support_family_routing` registration merge (check packet/CI/script consumers first); discovery before/after capture | FIX |
| FRESH | `guest_fixture_freshness_tdd.rs`: replace mtime checks with the xtask exit-contract authority or retire once the gated entry point owns it; retire obsolete-script path; replace substring checks; update remediation messages; preserve `all_guest_component_files_exist` / `guest_components_are_valid_wasm_components` | FIX |
| RAY | `macro_mesh_raycast_z_down_tdd.rs`: repair or map each scenario to real-mesh owners; require hits; transformed-fixture world-space-only acceptance; miss contract; boundary witness; scenario-to-survivor map before deletion | FIX |
| WORLD-Z | `translated_object_z_floor_tdd.rs`, `transformed_model_world_z_tdd.rs`, `multi_object_transform_world_z_tdd.rs`: drive the real planner; independent Z bounds; preserve identity/translation/multi-object cases; re-check below-floor policy | FIX |
| PRECEDENCE | `acceptance_gate_gaps_tdd.rs` production resolver; `scenario_traces_tdd.rs` real planning + region mapping | FIX |
| SUPPORT | `support_invariants_wedge_tdd.rs` nonempty populations + named-field checks; `live_layer_support_tdd.rs` real disable/blocker decisions; `support_disabled_no_output.rs` absence-with-evidence; `support_geometry_slice_consumption_tdd.rs` bracket identity | FIX |
| EMISSION | `cube_painted_overrides_e2e_tdd.rs` semantic-effect comparison; seam-plan vs emitted start; `required_event_set_is_present_for_a_minimal_run` from real events; `medial_axis_err_produces_warn_log` real failure; `e2e_with_layers`/`e2e_output_to_file` (pnp-cli file, travels here per primary-crate rule? **No — pnp-cli wave**) | FIX (CLI rows deferred) |
| DISPATCH | duplicate-key reach; non-vacuous prepass commits; pathopt ordered/identity inspection; per-region shape; native parity content; `dag_validation_tdd.rs` injected-fallback removal + literal inventory strengthen; `path_ordering_tdd.rs` reconciliation; `runtime_wiring_tdd.rs` tier-shape evidence; `layer_slice_tdd.rs` missing-fixture pass | FIX |
| DUP-RUNTIME | the §5 merge inventory (dispatch_* pairs, config-view rows, identity trio, lightning views, version pin, cube clusters, executor pairs, diagnostic roundtrip, gcode header/skirt rows, `painted_3mf_fixture_is_committed` fold, default-slice cluster, 3MF cluster) | FIX (case-matrix preserving) |
| ACCOUNTING | `allocator.rs` LIFO/attribution; `progress_instrumentation.rs` timing; `slicer_report_html_tdd.rs` duration vs stopwatch | FIX |
| BRITTLE | `no_linker_module_degraded_raw_output` re-derivation; `run_slice_against_wedge_returns_nonempty_gcode` stale-grep removal; four protected pending-contract tests (provenance check before touching) | FIX / KEEP-review |
| RETIRE | `slice_run_options_default_tdd.rs` FRU row; `layer_executor.rs` self-equality; `instrumentation.rs` noop; `slice_end_to_end_tdd.rs` `no_un_routed_prepass_modules_remain`; `paint_region_transport_widening_tdd.rs` docs-task grep | FIX |
| REVIEW | progress-event constructor matrix; `region_map_cap_exceeded_named_contributor`; `region_key_hash_and_eq...` precise claim; `pnp_cli_freshness_tdd` helper tests; `native_adapter_tdd` family coverage; `illegal_label_rejected` parser witness; wasm engine rows; empty-view defaults; IR sentinel semantics; CLI smoke rows; macro helper-only cases; `missing_fixture_returns_error` | KEEP-review (earn-their-keep) |

### 5.12 Wave: pnp-cli (and final wave)

| ID | Item | Disposition |
|---|---|---|
| EMISSION | `e2e_integration_tdd.rs` `e2e_with_layers` / `e2e_output_to_file` production-entry-point assertions | FIX |
| BRITTLE | `visual_debug_typed_tap_capture_tdd.rs` brace counting → structured evidence (read `docs/19_visual_debug.md` first); `module_search_path_tdd.rs` controlled paths; `support_preview_tdd.rs` explicit assumptions; `integrated_provenance_tdd.rs` scope review | FIX / KEEP-review |
| CROSS | `e2e_model_load_error` merge-or-strengthen; renderer/bundle determinism KEEP; AC/N1/N2 boundary keep-per-boundary | FIX / KEEP-review |
| DUP-MACRO-CLI-HELPERS | `module_new.rs` template-vs-compiled distinction; `helpers_cmd.rs` parameterize | FIX |
| RETIRE | `visual_debug_intermediate_renderer_tdd.rs` final alias comparison removal | FIX |
| FINAL WAVE | PARITY additive strengthening (audit §7 table; Arachne fixtures/oracles; never weaken exact pins); enforce-mode promotion (`TEST_QUALITY_ENFORCED` → true; preflight-blocking; `cargo xtask test --summary --workspace` acceptance via subagent FACT protocol); census reconciliation + ledger closure | FIX |

## 6. Verification command patterns (per crate wave)

Baseline (from §2 census + `Cargo.toml` stanzas; re-derive per wave):

```bash
set -o pipefail
mkdir -p target
# runtime buckets
cargo test -p slicer-runtime --test <bucket> -- TEST_FILTER --nocapture 2>&1 | tee target/test-output.log
# feature-gated crates — feature-correct invocations ONLY:
cargo test -p slicer-core --features host-algos --test flow_tdd 2>&1 | tee target/test-output.log
cargo test -p slicer-sdk --features test --test finalization_builder_tdd 2>&1 | tee target/test-output.log
cargo test -p pnp-cli --features integrated-classic-perimeters --test integrated_provenance_tdd 2>&1 | tee target/test-output.log
# scheduler buckets are scheduler_unit / scheduler_contract / scheduler_integration
```

Registration/filter verification uses `-- --list` on the same target; a
zero-match `--exact` run is failure, not success. Never re-run to see more
output — read the log. Gate commands: §4 list. Whole-suite runs (only at
program close) go through `cargo xtask test --summary --workspace` dispatched
to a subagent returning `FACT pass/fail`.

## 7. Ledger (progress record; rows store re-derivable facts, never frozen counts)

| Wave | State | Retired/changed symbols | Surviving/new coverage | Validation | Remaining gap |
|---|---|---|---|---|---|
| 0 | done | guest lockfiles re-synced (43), 47 guests rebuilt | `cargo xtask build-guests --check` exit 0 | `cargo xtask build-guests --check` | none (freshness) |
| 0 | done | — | discovery census manifest | `cargo metadata` + Cargo.toml stanza parse | committed at `docs/specs/test-quality-remediation-census.json` |
| A | done (A1–A9) | new `xtask/src/check_test_quality.rs`; preflight hook in `xtask/src/test.rs`; USAGE in `xtask/src/main.rs`; census committed at `docs/specs/test-quality-remediation-census.json` | 11 gate self-tests; report-mode findings = the 12 audit-confirmed patterns above | `cargo test -p xtask` 143/143; clippy `-D warnings` clean; `cargo check --workspace --all-targets` clean; `cargo xtask test --summary --workspace` **VERDICT: PASS** (758 test binaries, 0 failed) | burn down 12 findings per owning crate wave |
| core | open | — | — | — | — |
| ir | open | — | — | — | — |
| model-io | open | — | — | — | — |
| gcode | open | — | — | — | — |
| helpers | open | — | — | — | — |
| schema | open | — | — | — | — |
| macros | open | — | — | — | — |
| sdk | open | — | — | — | — |
| scheduler | open | — | — | — | — |
| wasm-host | open | — | — | — | — |
| runtime | open | — | — | — | — |
| cli | open | — | — | — | — |
| final (PARITY + promotion) | open | — | — | — | — |

## Packet Queue

The user approved the core-wave queue below and generation/preflight of its first
entry on 2026-09-09. For this batch, the user explicitly approved plan wave/item
IDs instead of `TASK-###` mappings; this plan is the packets' `backlog_source`.
Each packet remains `draft` unless separately approved for activation. Generated
packets contain `packet.spec.md`, `requirements.md`, `design.md`,
`implementation-plan.md`, and `task-map.md` under `docs/spec_packets/<slug>/`.

Dependencies below serialize packet generation within the core wave; they do
not assert that preceding packets have been implemented or export new APIs.
`generated` means authored and independently `PREFLIGHT PASS`, not implemented.
The implementation progress ledger in §7 remains separate.

| # | packet slug | goal (one sentence) | task ids | depends on | status | packet dir |
|---|-------------|---------------------|----------|------------|--------|------------|
| 1 | core-flow-consolidation | Consolidate the flow tests in `flow_tdd.rs`, preserve every distinct case, and strengthen the wider-bead oracle. | core/DUP-CORE (flow); core/RETIRE (flow); core/DUP-CORE-strengthen (wider-bead) | - | generated | docs/spec_packets/core-flow-consolidation/ |
| 2 | core-dup-merges | Replaced by the bridge, region, support, wall, geometry, and beading slices in rows #3–#8. | core/DUP-CORE (excluding flow) | #1 | superseded | - |
| 3 | core-bridge-dup-merge | Merge the identical supported-bridge case while retaining the unsupported-span and ungated-candidate guards. | core/DUP-CORE (bridge) | #1 | generated | docs/spec_packets/core-bridge-dup-merge/ |
| 4 | core-region-mapping-dup-merges | Consolidate cross-product cases while preserving exact chain sets, cardinality, and ordering coverage. | core/DUP-CORE (region mapping) | #3 | generated | docs/spec_packets/core-region-mapping-dup-merges/ |
| 5 | core-support-dup-merges | Consolidate overhang and support-schedule cases while preserving exact layer, area, and schedule assertions. | core/DUP-CORE (support overhang; support geometry) | #4 | pending | - |
| 6 | core-wall-sequence-dup-merges | Consolidate wall-order cases while preserving all modes and the two-wall sandwich input. | core/DUP-CORE (wall sequence) | #5 | pending | - |
| 7 | core-geometry-dup-review | Combine related distance assertions and retain the distinct endpoint witness unless a case-preserving merge is established. | core/DUP-CORE (segment path; point-to-segment distance) | #6 | pending | - |
| 8 | core-beading-threshold-review | Document and preserve the distinct default, propagation, and clamp threshold cases. | core/DUP-CORE (beading factory) | #7 | pending | - |
| 9 | core-strengthen | Strengthen the remaining core weak-oracle tests, including vertical-flow correction. | core/DUP-CORE-strengthen (excluding wider-bead) | #8 | pending | - |
| 10 | core-retire | Resolve the remaining core retirement candidates against the earn-their-keep standard. | core/RETIRE (excluding flow) | #9 | pending | - |
| 11 | core-paint | Repair paint driver, graph, and prune assertions and justify dependency probes. | core/PAINT | #10 | pending | - |
| 12 | core-brittle | Replace or justify the core timing guards while preserving meaningful performance protection. | core/BRITTLE | #11 | pending | - |
| 13 | core-cross | Review SDK/core wrapper coverage while retaining protection for distinct implementations. | core/CROSS | #12 | pending | - |
| 14 | core-parity | Add the core parity assertions named in §5.1 while preserving exact canonical pins. | core/PARITY | #13 | pending | - |

The user subsequently requested the next packet and approved replacing
`core-dup-merges` with rows #3–#8 on 2026-09-09. The replacement separates
independent edit surfaces and preserves the distinct endpoint and beading
threshold witnesses rather than presuming that each audit candidate is
redundant. No `core-dup-merges` packet directory was authored. The next approved
generation target is `core-bridge-dup-merge`, after the flow packet passes
preflight: status `draft`, downstream context S, the standard five packet
documents, and test-only source scope
`crates/slicer-core/tests/bridge_false_site_gating_tdd.rs`.

### Approved first-packet boundary

`core-flow-consolidation` owns the test code in
`crates/slicer-core/src/flow.rs` and `crates/slicer-core/tests/flow_tdd.rs`.
The approved single test home is `crates/slicer-core/tests/flow_tdd.rs`, including
the distinct bridge-flow cases currently inline. Production behavior changes
and other core tests are excluded. In particular,
`flow_correction_stays_positive_for_vertical_input`
(`crates/slicer-core/src/lib.rs`) belongs to `core-strengthen`.
Its downstream context estimate is S. The whole program is L; the initial
approval covered the first packet, and the continuation approval above adds
generation of the bridge slice. Later entries remain queued for resumption.

### Resume and dependency exports

Resume at the first `pending` row whose dependencies are `generated`, rechecking
the tree and the preceding packet's exports before authoring. Reassess each
pending packet's S/M context cost during grounding and obtain approval for any
scope-changing split. Remaining design choices, including timing-guard
replacement mechanisms and dependency-probe dispositions, are resolved at that
packet's write gate rather than assumed here.

After the core queue is exhausted, decompose the remaining crate waves from
§§4–5 in their existing order and approve their packet queue. The core-wave
queue does not mark any later wave complete.

`core-flow-consolidation` passed independent `spec-review --preflight` on
2026-09-09: S0–S8, AC commands, and Doc Impact all PASS. Source verification
confirmed the full survivor-case union. Independent synthetic checks of `AC-7`
(`docs/spec_packets/core-flow-consolidation/packet.spec.md`) accepted a valid
partial ledger row and rejected empty-row-plus-queue, completed-core, and
wrong-column cases. The user authorized one extra correction round beyond the
initial two-round limit to resolve the ledger-check and ownership findings;
those findings are now resolved. This is an authoring preflight result, not an
implementation acceptance result; the packet remains `draft`.

`core-bridge-dup-merge` passed independent `spec-review --preflight` on
2026-09-09 after one correction round: the round-1 S5 blocker and AC-command
finding were the author's seven-test census premise (the target file has six
`#[test]` fns pre-edit, five post-merge), corrected to 6 → 5 with `-eq 5`
assertions throughout; the round-2 re-run returned S0–S8, AC commands, and Doc
Impact all PASS with 0 blockers and 0 high findings. The packet's §5.1
premise held at authoring: `fully_supported_candidate_rejected_zero_bridge_area`
(lines 58–64 at generation time) is textually identical to the survivor
`solid_underneath_span_produces_no_bridge_area` apart from its name. This is an
authoring preflight result, not an implementation acceptance result; the packet
remains `draft`.

`core-region-mapping-dup-merges` passed independent `spec-review --preflight` on
2026-09-09 (round 1: 4 blockers / 2 highs — a falsified premise claiming the
AC-3 enumeration wording was candidate-unique when the survivor's doc comment
already carries `Verifies SET membership of the enumerated chains.`, a 14-vs-13
helper-count error, symbol paths placing `enumerate_canonical_chains` under
`crates/slicer-core/src/**` when it lives in `crates/slicer-ir/src/region_split_registry.rs`,
and a trailing-newline snippet mismatch, plus two under-proving AC commands;
round 2 re-run returned S0–S8, AC commands, and Doc Impact all PASS with 0
blockers and 0 highs). At authoring time the target file held 24 `#[test]` fns;
the packet's premise held: both absorbed tests are strict assertion subsets of
`region_mapping_two_semantics_produces_cross_product_cardinality` (one
count-only, one set-only) on an identical fixture, and only the AC-4 formula
prose `entries.len() == layers × active_regions × ∏(1 + K_i)` is candidate-unique.
This is an authoring preflight result, not an implementation acceptance result;
the packet remains `draft`.

Dependency exports from `core-flow-consolidation`: none (no new production API).
Dependency exports from `core-bridge-dup-merge`: none (test-only merge; no new
production API; no new test files). Dependency exports from
`core-region-mapping-dup-merges`: none (test-only merge; no new production API;
no new test files). The next eligible pending entry is `core-support-dup-merges`
(row #5, depends on #4, now generated); its scope is the approved §5.1
support-overhang/support-geometry slice, but its packet directory has not been
authored. Resume generation from that entry using this plan. The §7
implementation ledger remains open; no source or test changes were implemented
during packet generation.

Commit this plan's queue update together with the generated packet directory.
