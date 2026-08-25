---
status: implemented
packet: 238a-support-pattern-config-keys
task_ids:
  - TASK-363
  - TASK-364
  - TASK-365
  - TASK-366
  - TASK-367
  - TASK-368
depends_on: 236-support-stabilization
backlog_source: docs/specs/support-parity-gap-register.md
context_cost_estimate: M
---

# Packet Contract: 238a-support-pattern-config-keys

## Goal

Declare and wire the support pattern/expansion/bottom-z/line-width config surface with
canonical semantics — typed host keys, manifest declarations that defeat T8 silent defaults,
bounds enforcement, one canonical layer-height transport rule — so 237/238b/238c consume
keys that provably exist.

## Scope Boundaries

This packet owns gap-register rows G-03 (key half), G-04, G-05 (host half), G-08, G-09,
G-16 plus the issue-20/37 intersecting keys: every key gets a canonical type, default,
range, and transport, and the two divergent `effective_layer_height` marshal rules collapse
to one. Pattern-generator algorithms (non-rectilinear fills), planner geometric fidelity for
bottom gaps, raft keys, and renderer density consumption are excluded (see
`requirements.md` §Out of Scope).
The tree-support-planner manifest also declares `nozzle_diameter`, mirroring the
arachne/classic-perimeters manifests, so the line-width reader has a `get_abs_value`
base during its migration.

## Prerequisites and Blockers

- Depends on: `236-support-stabilization` — FORWARD DEPENDENCY (draft at this packet's
  authoring; re-derive 236's `packet.spec.md` frontmatter status at activation rather than
  trusting this sentence).
  This packet does not touch any file in 236's change surface; composition is limited to
  coexisting in the same scheduler/runtime graph. If 236's landed shape changes manifest or
  `ResolvedConfig` conventions this packet relies on, Steps 2–5 reconcile before closing
  (same gated-composition rule 237 established).
- Unblocks: `238b-tree-planner-canonical-fidelity` (consumes `support_style`,
  `max_bridge_length`, bottom-z semantics), `238c-support-renderer-flow-interfaces`
  (consumes `support_base_pattern_spacing`, pattern enum, `support_line_width`),
  and closes 237's `[FWD]→238a` on `bridge_no_support` spelling.
- Activation blockers: none. All `[FWD]` questions are implementer-resolvable with recorded
  decisions.

## Acceptance Criteria

State ACs only here; `requirements.md` references their IDs.

- **AC-1. Given** `modules/core-modules/tree-support-planner/tree-support-planner.toml`,
  **when** its `[config.schema]` is inspected, **then** each of the four tables exists
  under its own header — `support_branch_merge_distance_mm` (float, default 0.8, min 0),
  `support_max_branches_per_layer` (int, default 1024, min 1, max 10000),
  `max_bridge_length` (float, default 10.0, min 0), and `support_style`
  (enum, default `"default"`, values `default|grid|snug|organic|tree_slim|tree_strong|tree_hybrid`
  per canonical `s_keys_map_SupportMaterialStyle`) — each asserted individually, and
  `docs/15_config_keys_reference.md` contains all four key names (T8: declaration + doc
  regen in one commit).
  | `rg -q '^\[config\.schema\.support_branch_merge_distance_mm\]' modules/core-modules/tree-support-planner/tree-support-planner.toml && rg -q '^\[config\.schema\.support_max_branches_per_layer\]' modules/core-modules/tree-support-planner/tree-support-planner.toml && rg -q '^\[config\.schema\.max_bridge_length\]' modules/core-modules/tree-support-planner/tree-support-planner.toml && rg -q '^\[config\.schema\.support_style\]' modules/core-modules/tree-support-planner/tree-support-planner.toml && rg -q 'tree_strong' modules/core-modules/tree-support-planner/tree-support-planner.toml && rg -q 'tree_hybrid' modules/core-modules/tree-support-planner/tree-support-planner.toml && rg -q 'support_branch_merge_distance_mm' docs/15_config_keys_reference.md && rg -q 'max_bridge_length' docs/15_config_keys_reference.md && echo PASS || echo FAIL`
- **AC-2. Given** the tree planner's `sample_contact_points` path, **when** the module runs
  with no explicit `max_bridge_length` in config, **then** the effective bridge length comes
  from the declared manifest default 10.0 (the in-code
  `DEFAULT_MAX_BRIDGE_LENGTH_MM` fallback is retained only as a defensive equal), and a
  config-supplied `max_bridge_length` overrides it.
  | `mkdir -p target && cargo test -p slicer-runtime --test executor support_config_surface_tdd::max_bridge_length_config_reaches_tree_planner -- --exact 2>&1 | tee target/test-output.log && grep -q "^test result: ok" target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0 && echo PASS`
- **AC-3. Given** the host config surface in `crates/slicer-ir/src/resolved_config.rs`,
  **when** `ResolvedConfig` is constructed from defaults, **then** typed fields exist for
  `support_expansion` (f32, default 0.0), `support_top_z_distance` (f32, default 0.2),
  `support_bottom_z_distance` (f32, default 0.2), `support_threshold_overlap`
  (float_or_percent, default 50%), and `support_line_width` (float_or_percent, default 0 =
  auto), and `docs/config/host-keys.toml` plus `docs/15_config_keys_reference.md` declare
  all eleven host keys of this packet including the declaration-only ones
  (`bridge_no_support`, `enforce_support_layers`, `support_critical_regions_only`,
  `support_remove_small_overhang`, `support_object_first_layer_gap`,
  `support_sharp_tails`).
  | `rg -q 'support_bottom_z_distance' crates/slicer-ir/src/resolved_config.rs && rg -q 'support_threshold_overlap' docs/config/host-keys.toml && rg -q 'support_object_first_layer_gap' docs/15_config_keys_reference.md && rg -q 'support_remove_small_overhang' docs/15_config_keys_reference.md && echo PASS || echo FAIL`
- **AC-4. Given** a slice whose regions resolve `support_top_z_distance = 0.4` and
  `support_layer_height_mm = 0.4`, **when** the support-geometry builtin commits
  `SupportGeometryIR`, **then** `SupportGeometryIR.support_top_z_distance_mm` equals the
  resolved 0.4 (not the deleted `DEFAULT_SUPPORT_TOP_Z_DISTANCE_MM` hardcode) and
  `support_layer_height_mm` equals the resolved region target 0.4 (not the literal 0.0).
  The test lives in a NEW host-algos-gated test target
  `support_geometry_config_surface_tdd` (`crates/slicer-core/tests/
  support_geometry_config_surface_tdd.rs`, registered with
  `required-features = ["host-algos"]` in `crates/slicer-core/Cargo.toml`).
  | `mkdir -p target && cargo test -p slicer-core --features host-algos --test support_geometry_config_surface_tdd support_geometry_ir_carries_resolved_distances -- --exact 2>&1 | tee target/test-output.log && grep -q "^test result: ok" target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0 && echo PASS`
- **AC-5. Given** a layer participated in by multiple objects with differing
  `effective_layer_height` values where first-match and max disagree, **when** the layer plan
  view is built for both transports, **then** the wasm leg (`project_layer_plan_view`,
  `crates/slicer-wasm-host/src/marshal/in_.rs`) and the native leg
  (`build_native_prepass_request`, `crates/slicer-wasm-host/src/marshal/native.rs`) deliver
  the identical MAX-derived height from one shared helper (G-09 canonical rule; RC-11's
  prohibition on dividing by the field stands — consumers walk actual layer Z). The test
  is a NEW file `crates/slicer-wasm-host/tests/contract/layer_height_transport_tdd.rs`,
  registered via `mod layer_height_transport_tdd;` in
  `crates/slicer-wasm-host/tests/contract/main.rs`.
  | `mkdir -p target && cargo test -p slicer-wasm-host --test contract layer_height_transport_tdd::native_and_wasm_layer_views_share_canonical_layer_height -- --exact 2>&1 | tee target/test-output.log && grep -q "^test result: ok" target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0 && echo PASS`
- **AC-6. Given** the traditional planner manifest
  (`modules/core-modules/traditional-support-planner/traditional-support-planner.toml`),
  **when** inspected, **then** `support_base_pattern_spacing` is declared (float, default
  2.5 canonical; reference-profile value 2 remains settable), and
  `support_base_pattern` documents the canonical value set
  `default|rectilinear|rectilinear-grid|honeycomb|lightning|hollow` (canonical
  `s_keys_map_SupportMaterialPattern`) with its declared default unchanged.
  | `rg -q '^\[config\.schema\.support_base_pattern_spacing\]' modules/core-modules/traditional-support-planner/traditional-support-planner.toml && rg -q 'rectilinear-grid' docs/15_config_keys_reference.md && echo PASS || echo FAIL`
- **AC-7. Given** the unified `support_line_width` surface, **when** G-code serialization
  runs for a print whose resolved `support_line_width` is 0.42, **then** the emitted
  `; support_line_width = ` header line carries 0.42 sourced from config (replacing the
  hardcoded 0.35 in `DefaultGCodeSerializer`, `crates/slicer-gcode/src/serialize.rs`).
  The test extends the existing in-file `#[cfg(test)]` module of
  `crates/slicer-gcode/src/serialize.rs`, reached through the crate's library test target.
  | `mkdir -p target && cargo test -p slicer-gcode --lib serialize::tests::support_line_width_header_sources_resolved_value -- --exact 2>&1 | tee target/test-output.log && grep -q "^test result: ok" target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0 && echo PASS`

## Negative Test Cases

- **AC-N1. Given** a config supplying `max_bridge_length = -1.0`, **when** config resolution
  merges against the `ConfigBoundsIndex` built from the new declarations, **then** resolution
  fails with `ConfigResolutionError::OutOfRange` naming `max_bridge_length` (same
  rejection path as `out_of_range` precedent in
  `crates/slicer-scheduler/tests/integration/config_bounds_enforcement_tdd.rs`).
  | `mkdir -p target && cargo test -p slicer-scheduler --test scheduler_integration config_bounds_enforcement_tdd::rejects_max_bridge_length_below_min -- --exact 2>&1 | tee target/test-output.log && grep -q "^test result: ok" target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0 && echo PASS`
- **AC-N2. Given** a config supplying `support_max_branches_per_layer = 0`, **when** config
  resolution runs, **then** it is rejected `OutOfRange` (declared min 1 matches the planner's
  `clamp(1, 10_000)` floor — no silent clamp of out-of-band input).
  | `mkdir -p target && cargo test -p slicer-scheduler --test scheduler_integration config_bounds_enforcement_tdd::rejects_support_max_branches_per_layer_zero -- --exact 2>&1 | tee target/test-output.log && grep -q "^test result: ok" target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0 && echo PASS`
- **AC-N3. Given** a config supplying `support_branch_merge_distance_mm = -0.5`, **when**
  config resolution runs, **then** it is rejected `OutOfRange` (negative merge distances are
  meaningless; declared min 0).
  | `mkdir -p target && cargo test -p slicer-scheduler --test scheduler_integration config_bounds_enforcement_tdd::rejects_negative_support_branch_merge_distance -- --exact 2>&1 | tee target/test-output.log && grep -q "^test result: ok" target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0 && echo PASS`
- **AC-N4. Given** the previously-undeclared keys
  (`support_branch_merge_distance_mm`, `support_max_branches_per_layer`), **when** the
  existing guest-config tests that inject them re-run after declaration
  (`prepass_support_geometry_layer_plan_tdd.rs`,
  `support_geometry_config_normalization_tdd.rs` — values 0.8 / 1024 are in-bounds),
  **then** they still pass (declaration must not break in-bounds historical injectors).
  | `cargo test -p slicer-runtime --test executor --no-fail-fast 2>&1 | tee target/test-output.log && grep -q "^test result: ok" target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0 && echo PASS`
- **AC-N5. Given** the packet closed, **when** `docs/15_config_keys_reference.md`'s
  generated tables are checked, **then** `cargo xtask gen-config-docs --check` exits 0
  (T8 stale-doc regression guard: a past deletion left the doc stale).
  | `cargo xtask gen-config-docs --check && echo PASS || echo FAIL`

## Verification

- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo xtask build-guests --check` (exit 0 — this packet edits `modules/core-modules/**`
  and `crates/slicer-ir/**`, all inside the staleness snippet's applicability list;
  E4/T4 before attributing any guest-facing failure)
- `cargo xtask gen-config-docs --check` (exit 0 — T8 same-commit doc rule)

## Authoritative Docs

- `docs/specs/support-families-anchored-entities-plan.md` - §3 rulings, §6 invariant 16,
  §7 E1–E9, §8 human validation gate, §12 brief "238a-support-pattern-config-keys",
  §13 traps T4/T5/T6/T8 (direct ranged reads at authoring time; done)
- `docs/specs/support-parity-gap-register.md` - rows G-03, G-04, G-05, G-08, G-09, G-16
  (destinations updated to this packet at authoring time)
- `docs/15_config_keys_reference.md` - generated tables (never hand-edited; regenerate)
- `docs/spec_packets/224-support-family-orca-closure/design.md` - §RC-11 (walk-actual-Z
  prohibition; ranged read only)
- `docs/spec_packets/224-support-family-orca-closure/handoffs/orca-divergences.md` -
  divergence 5.4 (flow-derived widths vs config keys; ranged read)

## Human Validation Gate

Blocking per plan §8: this packet may not flip to `status: implemented` without a dated
sign-off line at the bottom of this section.

Artifacts to produce (all under `tmp/`, gitignored — verify by direct listing, trap T1):

1. Tree G-code: slice `tmp/SupportTest.stl` with the matched tree profile
   `tmp/support-family-config-tree-matched.json`.
2. Traditional G-code: same fixture with `tmp/support-family-config-normal-matched.json`.
3. Non-default boundary slice: a third profile extending a matched profile with THIS
   packet's boundary settings — non-default `support_base_pattern_spacing`,
   `support_expansion`, `support_bottom_z_distance`, and `support_line_width` — sliced for
   the family it configures; recorded as
   `tmp/support-family-config-238a-nondefault.json` with its provenance noted.
4. Visual-debug bundle for THIS packet's boundary: the non-default slice captured at layers
   where the expansion/bottom-z/pattern-spacing deltas are visible, indexed by its
   `manifest.json`.

Checklist to sign (each item names source, layer, tap, verdict; per E2 written inspection,
never a test claim):

- Termination: non-default bottom-z/expansion settings do not push column bottoms through
  the model or leave columns terminating short of the plate.
- Coverage: expanded contacts still cover the overhang; no overhang loses coverage under the
  non-default profile.
- Collision freedom: expanded support bodies do not intersect the model on their own layers.
- Interfaces: top-interface bands unchanged under the re-typed `support_line_width`.
- Block counts vs Orca references: candidate/plan-entry counts measured against
  `tmp/SupportTest_Tree_Orca.gcode` / `tmp/SupportTest_Normal_Orca.gcode`, recorded as
  numeric deltas in the evidence file.

Evidence file: `tmp/238a-human-validation.md` recording commands run, artifact paths,
layer indices inspected, and block-count deltas.

Sign-off: `2026-08-25 — approved` (human validation gate satisfied; packet closed to `status: implemented`).

## Doc Impact Statement (Required)

- `docs/15_config_keys_reference.md` generated tables (all new/retyped keys) -
  `rg -q 'support_base_pattern_spacing' docs/15_config_keys_reference.md`
- `docs/02_ir_schemas.md` `SupportGeometryIR` paragraph (`support_layer_height_mm` now
  carries the resolved region value instead of a literal 0.0) -
  `rg -q 'support_layer_height_mm' docs/02_ir_schemas.md && rg -qi 'resolved' docs/02_ir_schemas.md`
- `docs/DEVIATION_LOG.md` new row recording the divergence-5.4 decision (key-based
  `support_line_width` mapping; percent/auto resolve against `nozzle_diameter`) -
  `rg -q 'support_line_width' docs/DEVIATION_LOG.md`
- `docs/07_implementation_status.md` - TASK-363..368 rows registered by the packet-owned
  closure step (TASK-368), per `task-map.md` - `rg -q 'TASK-368' docs/07_implementation_status.md`

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` — `PrintConfigDef::build` /
  `init_fff_params`: canonical types/defaults/ranges for `support_base_pattern` (coEnum
  `SupportMaterialPattern`, default `default`), `support_base_pattern_spacing` (coFloat 2.5),
  `support_expansion` (coFloat 0), `support_top_z_distance` / `support_bottom_z_distance`
  (coFloat 0.2 each), `support_line_width` (coFloatOrPercent `{0,false}` ratio-over
  nozzle), `max_bridge_length` (coFloat 10), `support_style` (coEnum
  `SupportMaterialStyle`, default `default`), `support_threshold_overlap`
  (coFloatOrPercent 50%), `enforce_support_layers` (coInt 0), `bridge_no_support`
  (coBool false), `support_remove_small_overhang` (coBool true),
  `support_critical_regions_only` (coBool false), `support_object_first_layer_gap`
  (coFloat 0.2)
- `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` — `s_keys_map_SupportMaterialPattern`
  and `s_keys_map_SupportMaterialStyle`: the exact enum-string sets this packet documents
- `OrcaSlicerDocumented/src/libslic3r/Flow.cpp` — `auto_extrusion_width(frSupportMaterial,
  nozzle_diameter)` returning `nozzle_diameter` for support roles (others 1.125x) and
  `opt_key_to_flow_role` mapping `support_line_width` → `frSupportMaterial`: the canonical
  half of the divergence-5.4 mapping decision
- `OrcaSlicerDocumented/src/libslic3r/Support/SupportParameters.hpp` — ctor normalization
  `smsDefault` → `smsTreeOrganic`(tree)/`smsGrid`(normal) and tree/non-tree style rejection:
  informs what `support_style` declaration must admit (behavior itself is 238b)
- `OrcaSlicerDocumented/src/libslic3r/Support/TreeSupport.cpp` — ctor `is_strong =
  is_tree && style == smsTreeStrong` and hybrid `ePolygon` contact minting under large flat
  overhangs (`thresh_big_overhang`, `!is_sharp_tail`): confirms declared style values are
  the complete behavior surface 238b will key on

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- obey the shared absolute context bands: 120k reading budget with hand-off at 150k (standard); the extended band (240k reading / 300k hard stop) only via swarm's escalation protocol

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation (an extended-band run may carry a single L step only when `design.md` justifies why it cannot be split).
