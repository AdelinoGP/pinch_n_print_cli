# 152 — Asset: branch behaviour audit

Audited tip: `b23fd6d3` (the branch before the six remediation commits
`1905618e..a75c7e4c`). Compared against `origin/master` `d4f83e07`.

Roster: 32 commits = the 30 behavioural commits in
`git log --no-merges origin/master..b23fd6d3 -- crates modules` plus the two
rename riders that changed behaviour (`1be6a5af`, `4cdf5692`). The eight pure
`pnp(wayfinder-10x)` rename commits and the two docs-only commits
(`22ace5fd`, `f6a76001`) are excluded.

Buckets, per ticket 152: `keep` / `questionable-for-ruling` /
`superseded-by-master` / `defective`.

## Summary

| bucket | count | commits |
|---|---|---|
| keep | 23 | see table below |
| questionable-for-ruling | 4 | `71a4c832`, `ac035ed8`, `a21e70ee`, `f16c9ca5` |
| superseded-by-master | 2 | `202f812d`, `e7404ff1` |
| defective | 3 | `2d8d71c1`, `4cdf5692`, `b33f25f6` |

The three defects are fixed under the branch's "fix the code first" rule; the
four questionable items and the two superseded items are carried to ticket 153
for ruling.

## Defective

### `2d8d71c1` — six `*_filament_id` keys are declared nowhere, so P39 cannot fire

- **What:** wires six `*_filament_id` role selectors through
  `parse_feature_filament_selection` into `feature_tool_for_role` on both
  entity-resolution paths.
- **Evidence:** `FeatureFilamentSelection` / `feature_tool_for_role`
  (`crates/slicer-runtime/src/layer_executor.rs`);
  `parse_feature_filament_selection` (`crates/slicer-runtime/src/run.rs`).
- **Master:** `06958e77` replaced the raw-source option parsing with
  `resolved_host_options(&default_resolved_config)` and registered only
  `support_filament` / `support_interface_filament` in `HOST_RUNTIME_KEYS`.
  The six keys are declared nowhere, so `ConfigIngestor::prepare_entry`
  (`crates/slicer-config/src/ingestion.rs`) warn-then-drops them as
  `UnrecognizedKey`, `to_config_map()` yields `None` for all six, and the
  feature cannot fire outside tests.
- **Test:** `layer_executor.rs::tests::feature_filament_selection_routes_each_role_to_its_configured_tool`
  and `run.rs::tests::parse_feature_filament_selection_rebases_configured_one_based_indices`
  both hand-build the map, bypassing the drop.
- **Disposition:** fix — add the six `HOST_RUNTIME_KEYS` rows (typed int,
  `default: None`, no denied scopes) and one ingestion → resolution → executor
  test. Retiring the feature would repudiate ticket 46's human-ruled scope.

### `4cdf5692` — `fatal=false` is honoured at only 2 of 5 typed wasm sites

- **What:** makes `ModuleError::fatal=false` log-and-continue at the layer and
  prepass typed wasm sites, flips three test guests to `fatal`, and
  rebaselines four tests to `wall_loops = 2`.
- **Evidence:** `dispatch_layer_call` / `dispatch_prepass_call`
  (`crates/slicer-wasm-host/src/dispatch.rs`) honour it;
  `dispatch_finalization_call` does not.
- **Master:** `e7b3c68e` rewrote `dispatch_finalization_call`'s tail to
  `call_result.map_err(...)`, re-escalating `fatal=false`; the two postpass wasm
  sites (`dispatch_postpass_gcode_call`, `dispatch_postpass_text_call`) have
  carried the same unconditional escalation since 2026-06-01 and were never
  covered.
- **Test:** NONE for `fatal=false`; the guests now emit only
  `ModuleError::fatal`, and
  `layer_executor_tdd::layer_executor_continues_on_non_fatal_module_error` is
  vacuous (its runner discards the result and returns `Ok(None)`).
- **Disposition:** fix — honour `fatal=false` at finalization and both postpass
  sites, restore a non-fatal typed guest witness, and repair the vacuous
  executor test. Decide separately whether native entries must also degrade.

### `b33f25f6` — wave-overhangs reads a legacy flag production never sets

- **What:** declares four bridge keys on wave-overhangs and selects
  internal/external bridge density, thick flag and flow per branch; aligns
  `bridge_density` / `internal_bridge_density` max 120 → 125.
- **Evidence:** the internal branch keys off
  `region.is_internal_bridge()` (`modules/core-modules/wave-overhangs/src/lib.rs`).
  `is_internal_bridge` is not a `SlicedRegion` field (`crates/slicer-ir/src/slice_ir.rs`),
  is not copied by `SliceRegionView::from_ir` (`crates/slicer-sdk/src/views.rs`),
  has no WIT handle, and is set only by `SliceRegionView::set_is_internal_bridge`
  and the test fixture. Production is therefore always `false`, and the
  internal-qualified fallback consumes the external twins.
- **Master:** none on the keys; `internal_bridge_flow` declares `min = 0.0`
  and no max where canonical is `min 0, max 2.0`.
- **Test:** the new wave tests reach the branch only through
  `SliceRegionFixture::is_internal_bridge(true)` — a self-referential oracle.
  The rectilinear tests are genuine differentials because that module reaches
  the class through polygon membership.
- **Disposition:** fix — select the triple per polygon from
  `region.internal_bridge_areas()` membership (as `rectilinear-infill`'s
  `bridge_groups` does), re-anchor the wave tests on a polygon-authored
  internal bridge, add `max = 2.0`, and decide whether the legacy
  `SliceRegionView::is_internal_bridge` field is deleted outright.

## Questionable for ruling

### `71a4c832` — adopt a fifth canonical fill role?

- **What:** partitions the wall inset five ways
  (`bridge > bottom > top > internal > sparse`), re-purposes
  `SlicedRegion::internal_solid_fill` from "shell-band marker" to "fifth fill
  role", and makes rectilinear/gyroid emit it as `InternalSolidInfill`.
- **Evidence:** `sync_perimeter_infill_areas_into_slice`
  (`crates/slicer-runtime/src/region_partition.rs`);
  `convert_small_sparse_islands`
  (`crates/slicer-runtime/src/slice_postprocess_prepass.rs`). The WIT handle is
  wired on both views, the SDK, the macro glue and the drift test.
- **Master:** contradiction. `origin/master`'s `docs/02_ir_schemas.md` states
  "Post-Layer::Perimeters invariant: four canonical fill polygons", and
  `crates/slicer-runtime/tests/common/property_runtime_partition.rs` lists
  `internal_solid_fill` among carriers that must be byte-identical. The branch
  merge `f899cdf2` rewrote that model to five roles with no ADR.
- **Test:** the field now means different things at different stages, and the
  stale exposed-half assertion in
  `region_partition_tdd::converted_islands_join_the_shell_band_marker_in_the_internal_bucket`
  encoded pre-t166 round-join fillets (fixed in this pass; the test was added by
  `1c027a3c`, not by this commit).
- **Ruling:** adopt the fifth role and re-issue the invariant in master's own
  docs/property charter (record a deviation or ADR), or retire the bucket and
  return converted islands to `bottom_solid_fill` under a shell-index stamp.

### `ac035ed8` — validate claim holders below whole-print scope?

- **What:** adds `validate_configured_claim_holders`, two structured
  `SchedulerError`s and fatal progress code 402 at startup; also lands ticket
  109's `support_ironing_speed` rename and 15 → 30 fallback alignment.
- **Evidence:** `validate_configured_claim_holders` /
  `validate_startup_dag_with_configured_holders`
  (`crates/slicer-scheduler/src/validation.rs`);
  `VALIDATION_CLAIM_HOLDER_CODE` (`crates/slicer-runtime/src/progress_events.rs`).
- **Master:** the check validates only the four global holders, while
  `resolve_held_claims` (`crates/slicer-wasm-host/src/selection.rs`) reads
  `resolved.top_{fill_holder,…}` per region. `*_fill_holder` has no sub-print
  denial, so an object-scoped typo still yields a hollow region with no startup
  error — ticket 110's failure mode one scope down.
- **Ruling:** deny `*_fill_holder` below whole-print scope
  (`WHOLE_PRINT_ONLY_SCOPES`), or validate at every scope where it is
  admissible.

### `a21e70ee` — ship the flush placeholders before their fallback exists?

- **What:** adds scalar `filament_flush_temp` /
  `filament_flush_volumetric_speed` placeholders (0 publishes as 0, no
  canonical fallback) and `manual_filament_change` (serializer
  `; MANUAL_TOOL_CHANGE T<n>` tag and first-`change_filament_gcode` skip).
- **Evidence:** `ResolvedConfig::filament_flush_temp` /
  `filament_flush_volumetric_speed` (`crates/slicer-ir/src/resolved_config.rs`);
  `DefaultGCodeSerializer.manual_filament_change`
  (`crates/slicer-gcode/src/serialize.rs`); skip in `run_gcode_postprocess`
  (`modules/core-modules/machine-gcode-emit/src/lib.rs`).
- **Master:** none.
- **Test:** the flush substitution is pinned; the manual half is pinned
  separately in `gcode_emit_tdd::serialize_tool_change_with_manual_filament_change_emits_tag_line`.
- **Ruling:** ship the placeholders with 0-publishes-as-0 and raw singular
  names, or withhold them until canonical's
  `nozzle_temperature_range_high` / `filament_max_volumetric_speed` fallback
  exists.

### `f16c9ca5` — the speed floor rejects ADR-0072's AUTO sentinel

- **What:** adds `SPEED_BOUNDS` / `speed_bounds()` and seeds them into
  `ConfigBoundsIndex::from_modules`, so all host speed keys get an enforced
  lower bound; retires `max = 300.0` from twelve manifest rows.
- **Evidence:** `SPEED_BOUNDS` (`crates/slicer-ir/src/feedrate.rs`) sets
  `1.0` for `outer_wall_speed`, `inner_wall_speed` and `gap_infill_speed`.
  The bounds are live: `resolve_scope_stack` calls `check_extension_scalar`
  (`crates/slicer-config/src/resolution.rs`), which rejects
  `OutOfRange`. `docs/config/host-keys.toml` declares those three keys
  `range = ">= 0"` with `note = "0 selects emitter-owned volumetric AUTO
  (ADR-0072)"`, and `role_base_speed_mm_per_s` (`crates/slicer-gcode/src/emit.rs`)
  implements the zero placeholder.
- **Master:** ADR-0072 landed after this commit; the zero sentinel is master's
  contract, so the branch currently hard-rejects a documented input.
- **Test:** `config_bounds_enforcement_tdd::zero_is_rejected_for_a_speed_that_has_no_zero_meaning`
  pins the now-wrong contract.
- **Ruling:** drop the floor to `0.0` for the three AUTO keys (recommended) and
  retarget that test, or keep the floor and rescind ADR-0072's zero sentinel.
  Independent of the ruling, `skirt_speed` / `wipe_speed` doc rows and
  `SPEED_BOUNDS` disagree.

## Superseded by master

### `202f812d` — the overlay engine is production-dead

- **What:** replaces a hand-written 28-of-83 overlay diff with a macro-emitted
  `overlay_onto` that copies a field only when it differs from its resolution
  `origin`.
- **Evidence:** `ResolvedConfig::overlay_onto` / `__drc_overlay_arms!`
  (`crates/slicer-ir/src/resolved_config.rs`); the caller `overlay_resolved`
  (`crates/slicer-core/src/algos/region_mapping.rs`) is reachable only under
  `host_config.is_none()`.
- **Master:** production always passes `Some((resolved_configs,
  default_resolved_config))` from `commit_region_mapping_builtin`
  (`crates/slicer-runtime/src/builtins/region_mapping_producer.rs`), and
  `resolve_scope_stack` already expresses "reset to default" through delta
  presence.
- **Test:** only `overlay_onto`'s own drift guard and `host_config = None`
  fixtures remain; the four ticket-126 region-mapping tests are not at HEAD.
- **Disposition:** remove/unify — delete `overlay_onto` / `overlay_resolved`
  and the `host_config: None` branches, or rebuild the entry point on
  `ScopedConfig` + `resolve_scope_stack`. Do not add tests against the overlay
  path.

### `e7404ff1` — `support_base_pattern` is in a split state

- **What:** wires `bridge_no_support` and per-region bridge polygons into
  support-contact detection and removes the `support_base_pattern` input.
- **Evidence:** `resolve_contact_params` /
  `commit_support_analysis_builtin`
  (`crates/slicer-runtime/src/builtins/support_analysis_producer.rs`) are live;
  `SupportPlanner::support_base_pattern`
  (`modules/core-modules/traditional-support-planner/src/lib.rs`) was restored
  by the 2026-10-19 master merge.
- **Master:** still declares and reads `support_base_pattern`, and open ticket
  135 says the key must stay holder-only.
- **Disposition:** remove/unify — let master's key win, retire the reverted
  removal half, restore the deleted `traditional-base-pattern` capability
  assertion, and reconcile ticket 135's holder-only rule.

## Keep (23)

| sha | subject | why it stays |
|---|---|---|
| `e9cca240` | single-source resolved-target keys | encoder is one function with collision tests; the 191 census edit died in the master merge — do not resurrect |
| `44e10bcb` | vector-tolerant ingest | index-0 scalar subset is the documented DEV-171 behaviour; full vectors ride ticket 125 |
| `148b76fb` | JSON integer reaches modules | still guards `extensions`-borne ints; retarget one test through `bind_module_config_view` if production proof is wanted |
| `1c027a3c` | the two integration reds | strengthening: path-based metric 1.070 vs 3.271, threshold 2.0 |
| `b6df1a6b` | staggered inner seams | term-for-term port of `Slic3r::place_seam`; order/direction and conservation pinned |
| `49dde38b` | wipe-tower width x depth validation | depth shared by emitter and validator; three new negative tests fail pre-fix |
| `dc2584bb` | wipe-tower purge speed | canonical `min(max_purge, sparse_infill)` cap; both arms pinned |
| `951b7ef1` | canonical support-ironing gate | separates `support_ironing` from `ironing_enabled`; negatives pinned |
| `f2e04727` | support_line_width declaration | matches `Flow::support_material_flow`'s chain; explicit/auto arms pinned |
| `2659c05b` | minimum_sparse_infill_area | key drives `convert_small_sparse_islands`; guard and zero arms pinned |
| `a67c1525` | post_process export seam | canonical export step; e2e rewrites the artifact |
| `b846d04f` | gcode_add_line_number | canonical export step; e2e prefixes every line |
| `6ab3ee66` | pressure advance | one line at start and after each tool change; pinned |
| `62afe4de` | grab_length | reduces purge volume at the single decision point; pinned |
| `8804ee0c` | per-variant machine limits | `silent_mode` selects the stealth pair; pinned |
| `766afa79` | support_sharp_tails retired | user-ruled; object-bottom exception frozen on; pinned |
| `9108cdcb` | flush matrix live | per-pair purge volumes feed depth and prime; pinned |
| `4b021f21` | printer_structure time-lapse gate | suppresses for non-i3 single-tool; e2e pinned |
| `e0becffc` | printable_height gate | rejects over-height objects; e2e pinned |
| `c25b200b` | sparse_infill_speed 100 | fixes the double-count; default and neutral factor pinned |
| `e8935174` | Arachne min feature size | 25 % emits the 0.15 mm strip, 50 % rejects it |
| `fcaf69da` | guest lockfile convergence | uniform `--sync-locks` shape; `build-guests --check` green at the tip |
| `1be6a5af` | fuzzy-skin Orca keys | defaults are user-ruled; the default-value test is tautological (see below) |

## Test-quality notes

- `1be6a5af`'s `fuzzy_skin_tdd::empty_config_uses_canonical_fuzzy_skin_defaults`
  compares the empty config against explicit 0.2/0.3 values; both sides use the
  same fallback constants, so a reverted read site still passes. Add a
  non-default differential or delete the test.
- `4cdf5692`'s executor "continues on non-fatal" test is vacuous (runner
  discards the result); the fix above restores a real witness.
- `b33f25f6`'s wave tests are a self-referential oracle over a flag production
  never sets; the fix above re-anchors them on polygon membership.
- Three commit messages cite pre-renumber deviation IDs (`a67c1525`
  DEV-197→206, `8804ee0c` DEV-200→207, `e0becffc` DEV-167→205); the rows carry
  the trails, so these are stale messages, not missing deviations.

## Merge-order notes

- `f16c9ca5`'s `SPEED_BOUNDS` and ADR-0072's zero sentinel collide at HEAD;
  the ruling above must land before the PR.
- `c25b200b` retired the `value / BASE_SPEED` double-count in gyroid and
  lightning only. `classic-perimeters`, `tree-support`, `traditional-support`,
  `top-surface-ironing` and `support-surface-ironing` still hold the shape, and
  `crates/slicer-core/src/perimeter_utils.rs` still defines `BASE_SPEED`.
  Those are pre-existing and ride follow-up tickets, not this merge.
- `dc2584bb`'s `wipe_tower_max_purge_speed` and `f16c9ca5`'s bounds row are
  internally consistent; any merge resolution that restores the old
  `wipe_tower_speed` spelling must also update `SPEED_KEYS` / `SPEED_BOUNDS`.
