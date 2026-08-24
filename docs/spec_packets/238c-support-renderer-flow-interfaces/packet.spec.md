---
status: draft
packet: 238c-support-renderer-flow-interfaces
task_ids:
  - TASK-381
  - TASK-382
  - TASK-383
  - TASK-384
  - TASK-385
  - TASK-386
  - TASK-387
  - TASK-388
  - TASK-389
  - TASK-390
  - TASK-391
  - TASK-392
  - TASK-393
  - TASK-394
  - TASK-395
  - TASK-396
  - TASK-397
  - TASK-398
backlog_source: docs/07_implementation_status.md
context_cost_estimate: M
---

# Packet Contract: 238c-support-renderer-flow-interfaces

## Goal

Make both support-family renderers canonically faithful in flow, density, and interface
semantics: hollow tree walls, flow-derived densities (no percent-as-fraction mis-scale),
canonical 10.0 mm branch-radius cap, raise-to-base under interfaces, canonical roof/floor
band counts, the base-interface (`num_top_base_interface_layers`) role end to end, one
shared `interface_regularize`, and the DEV-129/145/146 dispositions.

## Scope Boundaries

Renderer-side semantics only: the two family renderers (`tree-support`,
`traditional-support`), their manifests, the shared regularize consolidation, the
tree-planner radius/band/classification surfaces this packet owns, and the WIT/IR/gcode
carrier for the new base-interface role. Planner algorithms (smoothing, collision,
styles, circles) belong to 238b and are done there; AGG rasterization is 241; raft is 240.

## Prerequisites and Blockers

- Depends on: `238b-tree-planner-canonical-fidelity` — FORWARD DEPENDENCY: 238b is
  authored (status `draft`) ahead of this packet in the queue rooted at
  `236-support-stabilization`; this packet must not activate until 238b reaches
  `implemented`. Chain: 236 → 237 → 238a → 238b → 238c.
- Unblocks: `239-support-independent-layer-z`, `241-support-agg-rasterizer`,
  `242-support-family-orca-closure`.
- Activation blockers: none beyond the dependency above; `[BLOCK]`-tagged questions live
  in `design.md` §Open Questions.

## Acceptance Criteria

- **AC-1 (G-10 hollow walls).** Given the tree family rendering a large support body
  region (≥ 20×20 mm square) at default `tree_support_wall_count = 1`, when the plan is
  rendered through the module's public run path, **then** the emitted body paths consist
  of exactly `tree_support_wall_count` concentric wall loops per contour inset ~half
  `line_width` from the boundary plus an interior fill pitched at the AC-2 density (line
  count far below a solid `line_width`-pitch raster), and no path duplicates the filled
  body the old `render_polygon` produced. | `cargo test -p tree-support --test tree_support_tdd -- tree_bodies_render_hollow_concentric_walls --exact 2>&1 | tee target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0`
- **AC-2 (G-11 density math).** Given `support_line_width = 0.4`,
  `support_base_pattern_spacing = 2.0`, and an effective layer height (e.g. 0.2), when
  either family renderer computes body fill
  pitch and interface pitches, **then** body density is exactly
  `min(1, support_flow_spacing / (support_base_pattern_spacing + support_flow_spacing))`
  with `support_flow_spacing = line_width_to_spacing(resolved support_line_width,
  effective_layer_height)` (binary helper: `slicer_core::flow::line_width_to_spacing(width,
  layer_height) -> Result<f32, NegativeSpacingError>`; tests exercise both arguments), top
  interface density is exactly
  `min(1, interface_flow_spacing / (support_interface_spacing + interface_flow_spacing))`,
  bottom interface density is analogous over `support_bottom_interface_spacing`, and every
  resulting pitch is ≥ its extrusion width (no overlapping passes). | `cargo test -p slicer-core --features host-algos --test support_flow_semantics_tdd -- canonical_density_derivations_match_formulas --exact 2>&1 | tee target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0`
- **AC-3 (G-12 radius cap).** Given the tree planner's radius pipeline, **when** a raw
  radius exceeds the cap, **then** it clamps at `MAX_BRANCH_RADIUS_MM = 10.0` (canonical
  `MIN_BRANCH_RADIUS = 0.4` / `MAX_BRANCH_RADIUS = 10.0` from `TreeSupport.hpp`), and the
  source constant reads `10.0`. | `rg -q 'MAX_BRANCH_RADIUS_MM: f32 = 10\.0' modules/core-modules/tree-support-planner/src/lib.rs && cargo test -p tree-support-planner --test tree_family_tdd -- branch_radius_clamps_at_canonical_maximum --exact 2>&1 | tee target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0`
- **AC-4 (G-13 raise-to-base).** Given `support_interface_top_layers = 2`, **when** the
  tree planner computes a branch radius at a layer adjacent to the interface band,
  **then** the radius is raised to `max(radius, base_radius)` (canonical `calc_branch_radius`
  mm-to-top behaviour), and with `support_interface_top_layers = 0` the radius is
  unchanged. | `cargo test -p tree-support-planner --test tree_family_tdd -- radius_raises_to_base_under_interfaces --exact 2>&1 | tee target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0`
- **AC-5 (G-18 band counts).** Given the tracked fixture sliced via `run_slice` with
  `support_interface_top_layers = 2` and `support_interface_bottom_layers = 2`,
  **when** the traditional family emits interface blocks, **then** the G-code carries
  exactly 3 `;TYPE:Support interface` blocks (the Orca-reference count measured in gap
  register G-18; today 2), while the `bottom = -1` mirror-top fallback and the
  top-follows-config counts pinned by `interface_layer_count_follows_config` (commit
  `ee27ac94`) still hold. | `cargo test -p slicer-runtime --test integration -- interface_band_counts_match_canonical_structure --exact 2>&1 | tee target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0`
- **AC-6 (F-37 planner side).** Given a tree branch landing within
  `num_top_base_interface_layers` layers beneath a roof contact, **when** the planner
  classifies node circles, **then** those circles are attributed the new base-interface
  plan role (`SupportPlanRole::BaseInterface` / WIT `support-plan-role.base-interface`) in
  the emitted plan entries, disjoint from roof and body attribution. | `cargo test -p tree-support-planner --test tree_family_tdd -- base_interface_band_attributed_in_plan_roles --exact 2>&1 | tee target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0`
- **AC-7 (F-37 transport).** Given the canonical WIT sources under
  `crates/slicer-schema/wit/deps/`, **when** the base-interface role and its
  `support-output-builder` push method are added, **then** `prepass-support-geometry.wit`
  exposes `support-plan-role.base-interface`, `ir-types.wit` exposes a base-interface push
  method on `support-output-builder`, `cargo build --tests` passes, guest artifacts are
  rebuilt fresh, and `docs/02_ir_schemas.md` documents the new role. | `rg -q 'base-interface' crates/slicer-schema/wit/deps/prepass-support-geometry/prepass-support-geometry.wit && rg -q 'BaseInterface' docs/02_ir_schemas.md && cargo xtask build-guests --check && echo FRESH`
- **AC-8 (F-37 marker).** Given `ExtrusionRole::SupportBaseInterface`, **when** the G-code
  emitter labels it, **then** `orca_type_label` returns `;TYPE:Support interface` (decision
  recorded in `design.md`), and the role's `default_priority` sits between
  `SupportMaterial` (5000) and `SupportInterface` (5500). | `cargo test -p slicer-gcode --lib -- base_interface_role_maps_to_support_interface_marker --exact 2>&1 | tee target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0`
- **AC-9 (regularize single-source).** Given the two byte-identical
  `interface_regularize.rs` copies, **when** consolidation lands, **then** exactly one
  implementation exists (in `slicer-core`), both module copies are deleted, both renderers
  consume the shared one, and the moved coverage tests pass. | `test ! -f modules/core-modules/tree-support/src/interface_regularize.rs && test ! -f modules/core-modules/traditional-support/src/interface_regularize.rs && rg -q 'pub fn regularize_entry_roles' crates/slicer-core/src/support_regularize.rs && cargo test -p slicer-core --features host-algos --test support_interface_regularize_tdd -- regularized_interface_never_exceeds_layer_area --exact 2>&1 | tee target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0`
- **AC-10 (DEV-145).** Given the corrected premise (`support_bottom_interface_spacing` IS
  canonical, default 0.5 min 0 per `PrintConfig.cpp`), **when** the family manifests are
  corrected, **then** both declare `default = 0.5` (today `-1.0`), the negative
  mirror-top sentinel stays parseable as a documented non-canonical legacy value, and
  `docs/15_config_keys_reference.md` is regenerated. | `rg -q 'default = 0\.5' modules/core-modules/traditional-support/traditional-support.toml && rg -q 'default = 0\.5' modules/core-modules/tree-support/tree-support.toml && rg -q 'support_bottom_interface_spacing' docs/15_config_keys_reference.md`
- **AC-11 (DEV-146).** Given the chosen mechanism (interface flow factor over the
  238a-retyped `float_or_percent` `support_line_width`; see `design.md` §Plan Corrections),
  **when** a family renderer pitches interface fill, **then** the pitch derives from
  `support_interface_flow` applied to the resolved `support_line_width` per the AC-2
  formula — doubling the flow ratio widens the interface pass width and correspondingly
  reduces the line count — and the key is declared in both manifests. | `rg -q 'support_interface_flow' modules/core-modules/traditional-support/traditional-support.toml && rg -q 'support_interface_flow' modules/core-modules/tree-support/tree-support.toml && cargo test -p traditional-support --test traditional_support_tdd -- interface_pitch_derives_from_interface_flow_over_line_width --exact 2>&1 | tee target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0`
- **AC-12 (DEV-129 close-or-finish).** Given bottom-interface bands demonstrably emit via
  `InterfaceRole::Floor` while the planner manifest still comments "Not yet implemented",
  **when** current truth is verified, **then** the packet closes DEV-129 as implemented —
  the stale manifest comment is removed and the diagnostics suite stays green — with the
  DEVIATION_LOG row updated; finishing missing work instead of closing is the documented
  falsifying-exit alternative (see `implementation-plan.md` Step 3). | `! rg -q 'Not yet implemented' modules/core-modules/tree-support-planner/tree-support-planner.toml && cargo test -p tree-support-planner --test diagnostics_tdd -- interface_bottom_layers_is_supported_and_warns_nothing --exact 2>&1 | tee target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0`

Every AC names exact fields, paths, counts, or output fragments and ends with its own
runnable command. Commands that dump more than 200 successful lines are tee'd to
`target/test-output.log` with a non-zero matched-count guard (invariant 16).

## Negative Test Cases

- **AC-N1 (density clamp).** Given a configuration whose derived density input exceeds 1
  (e.g. `support_base_pattern_spacing = 0`), **when** densities are computed, **then**
  every `min(1, …)` derivation clamps to exactly 1.0 yielding pitch == extrusion width
  (solid), never a negative, inverted, or sub-width pitch. | `cargo test -p slicer-core --features host-algos --test support_flow_semantics_tdd -- densities_clamp_to_one_solid_pitch --exact 2>&1 | tee target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0`
- **AC-N2 (invalid flow ratio guard).** Given `support_interface_flow ≤ 0` supplied at the
  renderer boundary, **when** the interface pitch is computed, **then** the renderer falls
  back to the canonical default ratio (100) instead of producing a zero/negative pitch, so
  no degenerate interface geometry is emitted. | `cargo test -p tree-support --test tree_support_tdd -- nonpositive_interface_flow_falls_back_to_default --exact 2>&1 | tee target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0`

## Verification

- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo xtask check-literals`
- Primary targeted proof: `cargo test -p slicer-runtime --test integration -- interface_band_counts_match_canonical_structure --exact 2>&1 | tee target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0`

## Authoritative Docs

- `docs/specs/support-families-anchored-entities-plan.md` - governing plan; §12 brief
  "238c-support-renderer-flow-interfaces", §3 rulings, §6 invariants (incl. 16), §7
  E1–E9, §8 human gate, §10 supersession (DEV-129/145), §13 traps T4/T5/T6/T8. Bounded
  ranged reads.
- `docs/specs/support-parity-gap-register.md` - rows G-10, G-11, G-12, G-13, G-18
  (routed to this packet); direct range read.
- `docs/spec_packets/224-support-family-orca-closure/parity-audit.md` - F-37 definition;
  delegated SUMMARY, range around F-37 only.
- `docs/08_coordinate_system.md` - porting checklist; consult via the coord-system
  constraint in `design.md`, do not full-read.

## Doc Impact Statement (Required)

- `docs/15_config_keys_reference.md` - regenerate after manifest key changes
  (T8; `4d1848eb` staleness lesson) - `rg -q 'support_interface_flow' docs/15_config_keys_reference.md`
- `docs/02_ir_schemas.md` - document `SupportPlanRole::BaseInterface` /
  `ExtrusionRole::SupportBaseInterface` - `rg -q 'BaseInterface' docs/02_ir_schemas.md`
- `docs/DEVIATION_LOG.md` - DEV-129 closed, DEV-145 premise/default corrected, DEV-146
  mechanism recorded, new row for the `support_density` key removal (re-derive the next
  free DEV id at implementation time; run `cargo xtask check-deviations`) -
  `rg -q 'DEV-145' docs/DEVIATION_LOG.md && rg -q 'DEV-146' docs/DEVIATION_LOG.md`
- `docs/07_implementation_status.md` - TASK-381..TASK-398 registered at packet-owned
  closure (Step 18) - `rg -q 'TASK-381' docs/07_implementation_status.md`
- No canonical doc enumerates the support `;TYPE:` marker set as an owned reference:
  a recursive probe at authoring time (`rg -l ';TYPE:Support' docs/ --include='*.md'`)
  finds only authority/record documents — this plan's §10 absorption notes, the gap
  register, and packet records under `docs/spec_packets/` — none of which is a
  canonical enumeration this packet may rewrite (the plan's §10 note records what
  `emit.rs` maps today and stays historically true). The two existing
  `docs/02_ir_schemas.md` sections that DO touch the new role are covered above:
  "Support plan entry" role enumeration (add `BaseInterface`) and "Extrusion-role default
  priority" table (add `SupportBaseInterface` | 5250). DECISION (recorded in design.md):
  no new marker-enumeration doc is created; the authoritative label source stays
  `orca_type_label` with its AC-8 unit test as the executable contract, and this packet's
  human-gate checklist records observed block counts instead.

## Human Validation Gate

Blocking per plan §8. Artifacts (produce into `tmp/`, regenerate before inspection):

- `tmp/support_test_tree_238c.gcode` — `cargo run --bin pnp_cli --release -- slice
  --input crates/slicer-runtime/tests/fixtures/support-family/SupportTest.stl --output
  tmp/support_test_tree_238c.gcode` with `tmp/support-family-config-tree-matched.json`.
- `tmp/support_test_normal_238c.gcode` — same fixture with
  `tmp/support-family-config-normal-matched.json`.
- Visual-debug bundle for this packet's boundary: interface bands + base-interface layer
  taps (`pnp_cli visual-debug`), stored under `tmp/vd-238c/`.

Checklist (each answered with layer, tap, verdict in writing; E2 — inspection only):

- [ ] Termination: branches reach the plate/model beneath their overhangs.
- [ ] Coverage: demanded overhang regions carry support on the fixture.
- [ ] Collision freedom: no support intersects model walls (hollow-wall insets intact).
- [ ] Interfaces: roofs/floors sit carved out of the body at interface pitch; base
      interface appears as interface-material passes on the tree branches under roofs.
- [ ] Block counts vs Orca references (REQUIRED for this packet): traditional
      `;TYPE:Support interface` blocks = 3 at top=2/bottom=2 (G-18); tree blocks match the
      reference count recorded in G-18 (tree 2 vs 2 baseline).
- [ ] Over-extrusion: visual wall/fill separation on tree bodies; no solid-filled
      branches.

Sign-off: `_date_ _verdict_` (packet may not flip to `status: implemented` without it).

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/Support/TreeSupport.cpp` — `generate_toolpaths` (hollow concentric sheath rendering; base areas infill only when needed; bottom-interface density consumption; `draw_circles` floor-block gating `!support_on_build_plate_only && (bottom_gap_height > EPSILON || bottom_interface_layers > 0)` with downward-scanned overlapping floor bands), `calc_branch_radius` (clamps to `MIN_BRANCH_RADIUS = 0.4` / `MAX_BRANCH_RADIUS = 10.0` from `TreeSupport.hpp`; mm-to-top variant raises radius to `base_radius` when `support_interface_top_layers > 0`).
- `OrcaSlicerDocumented/src/libslic3r/Support/SupportCommon.cpp` — `tree_supports_generate_paths` / `_make_loops` / `make_perimeter_and_infill` (wall-then-fill split; `area_group.need_infill` gating).
- `OrcaSlicerDocumented/src/libslic3r/Support/SupportParameters.hpp` — constructor density derivations: `support_density = min(1., support_material_flow.spacing() / support_spacing)` with `support_spacing = support_base_pattern_spacing + support_material_flow.spacing()`; `top_interface_density` / `bottom_interface_density` analogues; `number_of_support_interface_bottom_layers` (returns top count when bottom < 0, else bottom).
- `OrcaSlicerDocumented/src/libslic3r/Flow.cpp` — `support_material_interface_flow` (interface flow width from the interface flow ratio over `support_line_width` falling back to `line_width`; the interface pitch source is the resulting `spacing()`).
- `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` — `PrintConfigDef::build` (confirms `support_bottom_interface_spacing` default 0.5 min 0; confirms NO `support_density` percentage key and NO `support_interface_line_width` key exist canonically).

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- obey the shared absolute context bands: 120k reading budget with hand-off at 150k (standard); the extended band (240k reading / 300k hard stop) only via swarm's escalation protocol

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation (an extended-band run may carry a single L step only when `design.md` justifies why it cannot be split).
