---
status: implemented
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
styles, circles) belonged to 238b and are done there; per the human approver, any FURTHER
planner fixes that surface while making this packet's renderer work canonical land in
THIS packet (bounded to deltas discovered by this packet's implementation), not a new
one. AGG rasterization is 241; raft is 240.

## Prerequisites and Blockers

- Depends on: `238b-tree-planner-canonical-fidelity` — SATISFIED: 238b is
  `status: implemented`, human-signed off 2026-08-25, closed as the single squashed commit
  "238b-tree-planner-canonical-fidelity: implement packet (TASK-369..380)" (cite by
  description, not SHA). Chain: 236 → 237 → 238a → 238b → 238c. Landed 238b surfaces this
  packet consumes: `build_roles` now emits DISCRETE per-connected-component role regions
  per layer (per-node circle footprints + same-node consecutive-layer capsules; NO
  cross-branch sweeps; NO global per-role union), and the DEV-144
  `SupportPlanSkeleton.wall_counts` transport (WIT `wall-counts: list<u32>`,
  `SupportPlanIR` schema 2.1.0, both marshal legs length-asserted).
- Unblocks: `239-support-independent-layer-z`, `241-support-agg-rasterizer`,
  `242-support-family-orca-closure`.
- Activation blockers: none. The 238b dependency is satisfied; `[BLOCK]`-tagged questions
  live in `design.md` §Open Questions (currently none remaining).

## Acceptance Criteria

- **AC-1 (G-10 hollow walls).** Given the tree family rendering a large support body
  region (≥ 20×20 mm square) at default `tree_support_wall_count = 1`, when the plan is
  rendered through the module's public run path, **then** the emitted body paths consist
  of exactly `tree_support_wall_count` concentric wall loops per contour inset ~half
  `line_width` from the boundary plus an interior fill pitched at the AC-2 density (line
  count far below a solid `line_width`-pitch raster; a region narrower than one pitch
  still emits its center fill line, so sub-pitch tips are NOT hollow), and no path
  duplicates the filled body the old `render_polygon` produced. | `cargo test -p tree-support --test tree_support_tdd -- tree_bodies_render_hollow_concentric_walls --exact 2>&1 | tee target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0`
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
  source constant reads `10.0`. (Constant is still `6.0` today — this AC is open work,
  unlike AC-1's renderer baseline which 238b partially landed.) | `rg -q 'MAX_BRANCH_RADIUS_MM: f32 = 10\.0' modules/core-modules/tree-support-planner/src/lib.rs && cargo test -p tree-support-planner --test tree_family_tdd -- branch_radius_clamps_at_canonical_maximum --exact 2>&1 | tee target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0`
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
  `ee27ac94`) still hold. The new test is a NEW integration test added beside
  `final_gcode_roles` (which currently ends by chaining
  `interface_layer_count_follows_config`); it does not weaken any existing pin.
  | `cargo test -p slicer-runtime --test integration -- interface_band_counts_match_canonical_structure --exact 2>&1 | tee target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0`
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
  rebuilt fresh, and `docs/02_ir_schemas.md` documents the new role. The DEV-144
  wall-counts transport this packet's extra-wall printing consumes is ALREADY LANDED by
  238b (`wall-counts: list<u32>` in `record support-plan-skeleton`,
  `SupportPlanIR` schema 2.1.0, both marshal legs length-asserted) and is consumed as-is,
  not rebuilt. | `rg -q 'base-interface' crates/slicer-schema/wit/deps/prepass-support-geometry/prepass-support-geometry.wit && rg -q 'BaseInterface' docs/02_ir_schemas.md && cargo xtask build-guests --check && echo FRESH`
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
- **AC-13 (trunk infill pattern, measured delta 1).** Given a tree body region wide
  enough for multiple fill lines at the AC-2 body pitch, **when** the tree renderer fills
  it, **then** the fill is pitched per AC-2 AND alternates between two orthogonal
  directions on consecutive layers of the same region (Orca 45° crosshatch equivalent;
  today the fill is horizontal-only every layer — see `scan_fill_region`), so adjacent
  layers do not produce parallel un-bonded rungs. | `cargo test -p tree-support --test tree_support_tdd -- body_fill_alternates_direction_across_layers --exact 2>&1 | tee target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0`
- **AC-14 (tip solidity, measured delta 2).** Given a branch tip whose role region is
  smaller than one wall inset (a tip puck), **when** rendered, **then** the region emits
  SOLID coverage (walls plus center fill per `scan_fill_region`'s min-fill line) rather
  than concentric rings alone, matching Orca's solid tip pucks. Negative form: a region
  that fits inside the innermost wall inset must NOT emit only an unfilled ring set.
  | `cargo test -p tree-support --test tree_support_tdd -- sub_pitch_tip_region_emits_solid_center_line --exact 2>&1 | tee target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0`
- **AC-15 (top-layer interface band semantics, measured delta 3, G-18-adjacent).** Given
  the tracked fixture sliced with the matched config, **when** top-of-tree contact tips
  reach their overhangs, **then** the count and size distribution of top-layer tip
  geometry is compared against the Orca reference and the roof/interface-band attribution
  (`InterfaceRole::Roof` vs body) is reconciled wherever the delta traces to band-width
  semantics rather than to Orca's own denser tip seeding (~30 PnP vs ~50 Orca tips at the
  reference's l120; the residual after band-semantics fixes is recorded in the human-gate
  checklist as accepted-or-escalated, never silently dropped). | `cargo run --bin pnp_cli --release -- slice --model crates/slicer-runtime/tests/fixtures/support-family/SupportTest.stl --config tmp/support-family-config-tree-matched.json --module-dir modules/core-modules --output tmp/p238c-tips.gcode --no-progress-events && rg -c '^;TYPE:Support interface' tmp/p238c-tips.gcode`
- **AC-16 (branch centerline rendering decision, measured delta 4).** Given Orca renders
  branch cross-sections as circle+chord outlines around centerlines while the current
  renderer draws only the planner's discrete role regions through
  `render_polygon`, **when** this packet closes, **then** either (a) circle+chord outline
  rendering is implemented over the skeleton points, or (b) the discrete-region rendering
  is recorded in design.md §Measured renderer baseline as the deliberate disposition with
  its visual consequence named — no third silent state. | `rg -q 'circle\+chord|circle-chord|centerline' docs/spec_packets/238c-support-renderer-flow-interfaces/design.md && echo DISPOSITIONED || echo FAIL`

Every AC names exact fields, paths, counts, or output fragments and ends with its own
runnable command. Commands that dump more than 200 successful lines are tee'd to
`target/test-output.log` with a non-zero matched-count guard (invariant 16).

## Operational Traps (binding on every verification command)

- **Module registration trap (F):** `pnp_cli slice` REQUIRES
  `--module-dir modules/core-modules`; without it, support modules silently fail to
  register — zero `;TYPE:Support` blocks, no error. EVERY slice invocation in this packet
  (human-gate artifacts, AC-15, any ad-hoc evidence run) MUST include it.
- **Cargo flag position (G):** `--no-fail-fast` is a CARGO flag; it goes BEFORE the `--`
  test-filter separator (`cargo test -p <crate> --no-fail-fast --test <file>`), never
  after `--`.
- **Integration-harness filters (G):** tests inside the aggregated integration binaries
  are module-qualified: `config_bounds_enforcement_tdd::<test_name>`
  (`crates/slicer-scheduler/tests/integration/`), and the marshal seam test lives in
  slicer-wasm-host:
  `cargo test -p slicer-wasm-host --test contract -- view_seam_identity_tdd::native_and_wasm_layer_views_are_field_identical --exact`.

## Negative Test Cases

- **AC-N1 (density clamp).** Given a configuration whose derived density input exceeds 1
  (e.g. `support_base_pattern_spacing = 0`), **when** densities are computed, **then**
  every `min(1, …)` derivation clamps to exactly 1.0 yielding pitch == extrusion width
  (solid), never a negative, inverted, or sub-width pitch. | `cargo test -p slicer-core --features host-algos --test support_flow_semantics_tdd -- densities_clamp_to_one_solid_pitch --exact 2>&1 | tee target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0`
- **AC-N2 (invalid flow ratio guard).** Given `support_interface_flow ≤ 0` supplied at the
  renderer boundary, **when** the interface pitch is computed, **then** the renderer falls
  back to the canonical default ratio (100) instead of producing a zero/negative pitch, so
   no degenerate interface geometry is emitted. | `cargo test -p tree-support --test tree_support_tdd -- nonpositive_interface_flow_falls_back_to_default_module_boundary --exact 2>&1 | tee target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0`

## Verification

- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo xtask check-literals`
- Primary targeted proof: `cargo test -p slicer-runtime --test integration -- interface_band_counts_match_canonical_structure --exact 2>&1 | tee target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0`
- Any ad-hoc slice evidence MUST use the trap-compliant form (see Operational Traps):
  `cargo run --bin pnp_cli --release -- slice --model <model> --config <config.json> --module-dir modules/core-modules --output <out>.gcode --no-progress-events`

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
  closure (Step 18; range re-verified 2026-08-25 against docs/07, which currently ends at
  TASK-380 from 238b — TASK-381+ are free) - `rg -q 'TASK-381' docs/07_implementation_status.md`
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
  --model crates/slicer-runtime/tests/fixtures/support-family/SupportTest.stl --config
  tmp/support-family-config-tree-matched.json --module-dir modules/core-modules --output
  tmp/support_test_tree_238c.gcode --no-progress-events` (NOTE: the CLI's flag is
  `--model`, not `--input`; `--module-dir` is REQUIRED or no support module registers).
- `tmp/support_test_normal_238c.gcode` — same fixture with
  `tmp/support-family-config-normal-matched.json`, same flags.
- Visual-debug bundle for this packet's boundary: interface bands + base-interface layer
  taps, stored under `tmp/vd-238c/`; reusable request shapes exist at
  `tmp/vdcmp/{ours,ref}-request.json` (gcode-source requests against
  `tmp/p238b-tree-fixture.gcode` and `tmp/SupportTest_Tree_Orca.gcode`).

Checklist (each answered with layer, tap, verdict in writing; E2 — inspection only):

- [x] Termination: PASS — branches reach plate/model beneath their overhangs; verified
      visually by the human approver on the tree G-code (2026-08-27) and in
      `tmp/vd-238c/user-ours-v11/` (`filled_areas` + `filament_lines` taps, layers
      l44/l65–l67/l80–l81/l121).
- [x] Coverage: PASS — demanded overhang regions carry support on `SupportTest.stl`;
      human-verified 2026-08-27 (same bundle/taps as above).
- [x] Collision freedom: PASS — no support intersects model walls; hollow-wall insets
      intact (AC-1 structural test green; human visual verdict 2026-08-27).
- [x] Interfaces: PASS — roofs/floors carved out of the body at interface pitch;
      base-interface passes present on branches under roofs
      (`base_interface_band_attributed_in_plan_roles` + AC-8 marker test green; human
      visual verdict 2026-08-27).
- [x] Block counts vs Orca references: PASS — traditional `;TYPE:Support interface`
      blocks = 3 at top=2/bottom=2 (measured 2026-08-27, `tmp/p238c-review-normal.gcode`,
      G-18 met); tree blocks = 2 (measured 2026-08-27, `tmp/p238c-review-tips.gcode`),
      matching the G-18 tree 2-vs-2 baseline (238b re-measured 124 total `;TYPE:Support`
      blocks, delta 0 vs reference).
- [x] Over-extrusion: PASS — wall/fill separation visible on tree bodies, no
      solid-filled branches (AC-1/AC-2 density derivations green; human visual verdict
      2026-08-27).
- [x] Remaining-delta sweep (against the 2026-08-25 baseline, design.md §Measured
      renderer baseline), each with before/after state:
      trunk infill pattern — was horizontal-only rungs, now direction-alternating fill
      per AC-13 (`body_fill_alternates_direction_across_layers` green);
      tip solidity — was rings, now solid center-line pucks per AC-14
      (`sub_pitch_tip_region_emits_solid_center_line` green);
      top-layer tip count/size (AC-15) — band-semantics share reconciled (roof
      attribution + G-18 top-band widening); the residual ~30 (PnP) vs ~50 (Orca) tips at
      the reference l120 traces to Orca 2.4.1's organic engine tip seeding and
      `getRadius` tip ramp, not band semantics (handoff.md deltas 4/6) — residual
      ACCEPTED, queued as `docs/specs/support-generation-remediation-plan.md` row 7
      (TASK-441 organic-tree-engine port);
      branch centerline rendering — dispositioned per AC-16 option (b), recorded in
      design.md §Measured Renderer Baseline (2026-08-26).

Sign-off: `2026-08-27 APPROVED` — G-code artifacts verified by the human approver
(in-session confirmation, 2026-08-27: "Gcode was verified by me"); checklist recorded by
the reviewing agent from measured evidence in the same session.

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
