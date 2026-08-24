# Requirements: 238c-support-renderer-flow-interfaces

## Packet Metadata

- Grouped task IDs: `TASK-381` … `TASK-398`
- Backlog source: `docs/07_implementation_status.md` (re-derive next free ID at
  registration; this packet assumes TASK-381+ after 238b's TASK-369..380)
- Packet status: `draft`
- Aggregate context cost: `M`

## Problem Statement

The support-family renderers are structurally present (packets 221/222) but their flow,
density, and interface semantics diverge from canonical OrcaSlicer in five measured ways:

1. **G-10** — tree branch bodies render **filled**; canonical renders hollow concentric
   sheath walls (`generate_toolpaths` → `tree_supports_generate_paths`).
2. **G-11** — PnP over-extrudes support 1.107× vs Orca (flow per mm of path; gap-register
   measurement). Compounding it, `support_density` arrives percent-scaled (`20.0`) but is
   consumed as a fraction, so the `min(1.0)` clamp forces solid fill above 1.
3. **G-12** — `MAX_BRANCH_RADIUS_MM = 6.0` vs canonical 10.0.
4. **G-13** — the canonical raise-to-`base_radius` rule under interfaces is absent.
5. **G-18** — at top=2/bottom=2 interface layers, traditional emits 2
   `;TYPE:Support interface` blocks vs Orca's 3: the roof/floor band structure is not
   canonical (`number_of_support_interface_bottom_layers`; `draw_circles` floor block).

Plus three structural debts owned here: **F-37 piece 2** (the base-interface
`num_top_base_interface_layers` role end-to-end — piece 1, regularization wiring, landed
in commit `050d5c3a`), the byte-identical duplication of `interface_regularize.rs` across
both renderers (explicitly flagged as the consolidation target by that commit), and
deviations DEV-129 (bottom-interface diagnostic claim vs implemented truth), DEV-145
(premise corrected in plan §10 — the key IS canonical with default 0.5; PnP defaults
−1.0), DEV-146 (interface pitch derives from generic `line_width`; canonical derives from
the interface flow width). This is one coherent slice because all items share the two
renderer modules, one config/manifest surface, and one WIT/IR role carrier.

## In Scope

- **Hollow tree walls (G-10):** rework `render_polygon`
  (`modules/core-modules/tree-support/src/lib.rs`) so body regions emit
  `tree_support_wall_count` concentric walls + density-pitched interior fill (fill only
  when the region needs infill, mirroring canonical `need_infill` gating); delete the
  filled-body model. Interface roles keep their dedicated pitch.
- **Density derivations (G-11):** replace the percent-as-fraction `support_density`
  consumption with canonical formulas from the `SupportParameters.hpp` constructor:
  body `min(1, flow_spacing / (support_base_pattern_spacing + flow_spacing))`, top
  interface `min(1, interface_flow_spacing / (support_interface_spacing +
  interface_flow_spacing))`, bottom analogous over `support_bottom_interface_spacing`.
  Retire the `support_density` manifest key from both family manifests (canonical has no
  such key for support); record the removal as a deviation row. Spacing inputs come from
  238a's typed keys.
- **Radius cap (G-12):** `MAX_BRANCH_RADIUS_MM = 6.0 → 10.0` in
  `modules/core-modules/tree-support-planner/src/lib.rs` (constant + clamp site + tests).
- **Raise-to-base (G-13):** implement canonical `calc_branch_radius` mm-to-top raise:
  when `support_interface_top_layers > 0`, radius becomes `max(radius, base_radius)`
  where the band height spans the interface layers.
- **Roof/floor band counts (G-18):** implement canonical band structure —
  `number_of_support_interface_bottom_layers` semantics (top count mirrored when bottom <
  0, else bottom count) and the `draw_circles` floor block
  (`!support_on_build_plate_only && (bottom_gap_height > EPSILON || bottom_interface_layers
  > 0)`), splitting overlapping downward-scanned bands into floor areas — so the
  traditional family emits the Orca-measured 3 blocks at top=2/bottom=2 while preserving
  the `ee27ac94` contact-inclusive top-count pins.
- **F-37 piece 2 (base-interface role):** new `SupportPlanRole::BaseInterface` IR variant,
  WIT `support-plan-role.base-interface` + `support-output-builder` push method, host
  marshal legs both directions, schema-version bump (derived-at-activation per plan §12),
  new `ExtrusionRole::SupportBaseInterface` with `;TYPE:` marker decision, planner
  attribution of base-interface circles, renderer emission through the new carrier.
  Blast radius enumerated below.
- **`interface_regularize` consolidation:** move the byte-identical pair into ONE shared
  implementation in `slicer-core`; both renderers consume it. Scope-limited to the
  support-side pair (DEV-127's rectilinear-infill third copy stays out of scope).
- **DEV-145 correction:** flip `support_bottom_interface_spacing` default −1.0 → 0.5 in
  both manifests (`traditional-support.toml`, `tree-support.toml`); keep the negative
  mirror-top sentinel parseable as legacy non-canonical input; correct the DEVIATION_LOG
  row (implementation work, done in-packet).
- **DEV-146 mechanism:** derive interface pitch from an interface flow factor over the
  resolved `support_line_width` (238a retyped it `float_or_percent`, default 0 = auto);
  declare `support_interface_flow` (percent, canonical default 100) in both manifests.
  Decision rationale in `design.md` §Plan Corrections / §Interface Width Mechanism.
- **DEV-129 resolution:** verify current truth (bottom-interface bands emit via
  `InterfaceRole::Floor`; `diagnostics_tdd.rs` already asserts no code-1003), remove the
  stale "Not yet implemented" manifest comment on `support_interface_bottom_layers`, close
  the deviation as implemented — or finish genuinely missing work first. No third state.
- Doc hygiene: gap-register destination columns G-10/G-11/G-12/G-13/G-18 → this packet;
  stub absorption record; `docs/15_config_keys_reference.md` regeneration;
  `docs/02_ir_schemas.md` role documentation; DEVIATION_LOG rows; `docs/07` registration.

### F-37 blast-radius enumeration (~10 files)

Verified against the live tree at authoring time:

| Surface | File | Change |
| --- | --- | --- |
| WIT role enum | `crates/slicer-schema/wit/deps/prepass-support-geometry/prepass-support-geometry.wit` | `support-plan-role.base-interface` |
| WIT builder method | `crates/slicer-schema/wit/deps/ir-types.wit` | `push-base-interface-path` on `support-output-builder` |
| IR enum | `crates/slicer-ir/src/slice_ir.rs` | `SupportPlanRole::BaseInterface` (+ match-arm fallout) |
| IR enum | `crates/slicer-ir/src/slice_ir.rs` | `ExtrusionRole::SupportBaseInterface` + priority + closed-loop set review |
| Host dispatch | `crates/slicer-wasm-host/src/{host.rs,dispatch.rs}` — `host.rs` builder impl; `dispatch.rs` owns the live four-arm `SupportPlanRole` → WIT-role match and gains the `BaseInterface` arm | new push method plumbing + BaseInterface dispatch arm |
| Marshal legs | `crates/slicer-wasm-host/src/marshal/{in_.rs,native.rs}` + generated bindings | role round-trip both legs (T9 hazard) |
| Planner | `modules/core-modules/tree-support-planner/src/lib.rs` | attribute base-interface circles |
| Renderer ×2 | `modules/core-modules/tree-support/src/lib.rs`, `modules/core-modules/traditional-support/src/lib.rs` | consume role, emit via carrier |
| G-code | `crates/slicer-gcode/src/emit.rs` | `orca_type_label` arm + feedrate/priority mapping |
| Schema docs | `docs/02_ir_schemas.md` | role documented |

Plus test fallout: any `match` on `SupportPlanRole`/`ExtrusionRole` across
`slicer-runtime`/`slicer-gcode`/module tests that turns non-exhaustive — discovered by
`cargo build --tests`, owned by the introducing step, never deferred.

## Out of Scope

- Tree-planner algorithms (top-Z gap, smoothing, collision/avoidance keying, circle
  fidelity, styles, miter limits): owned by 238b (authored draft; forward dependency).
- AGG rasterizer port: 241 (Rulings 7/8 knob lives there).
- Raft geometry, signed-index migration, raft keys: 240.
- DEV-127's third scan-line copy in `rectilinear-infill`: register-only remainder.
- `erSupportTransition` (G-20): register-only per prior human decision — do not conflate
  its would-be role with this packet's distinct base-interface role (plan §15 note).
- Ironing keys, filament keys: feature-gap track.
- The 1.107× figure itself is accepted as measured baseline (G-11 row); post-fix
  re-measurement belongs to 242 closure evidence, though the ACs pin structural causes.
- `cargo test --workspace` except at packet-close ceremony per repo Test Discipline.

## Authoritative Docs

- `docs/specs/support-families-anchored-entities-plan.md` - ~755 lines; ranged reads of
  §3, §6, §7, §8, §10, §12 brief, §13 T4/T5/T6/T8, §14 rules only.
- `docs/specs/support-parity-gap-register.md` - rows G-10..G-18 range only.
- `docs/spec_packets/224-support-family-orca-closure/parity-audit.md` - F-37 entry;
  delegated SUMMARY.
- `docs/spec_packets/224-support-family-orca-closure/handoffs/HANDOFF-224-s6.md` -
  F-37 piece-2 sizing line only.
- `.agents/doc-index.md` + `docs/19_visual_debug.md` / `docs/17_agent_debugging.md` -
  consult only if human-gate debugging needs them.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/Support/TreeSupport.cpp` — `generate_toolpaths` (hollow concentric sheath rendering; base areas infill only when needed; bottom-interface density consumption; `draw_circles` floor-block gating `!support_on_build_plate_only && (bottom_gap_height > EPSILON || bottom_interface_layers > 0)` with downward-scanned overlapping floor bands), `calc_branch_radius` (clamps to `MIN_BRANCH_RADIUS = 0.4` / `MAX_BRANCH_RADIUS = 10.0` from `TreeSupport.hpp`; mm-to-top variant raises radius to `base_radius` when `support_interface_top_layers > 0`).
- `OrcaSlicerDocumented/src/libslic3r/Support/SupportCommon.cpp` — `tree_supports_generate_paths` / `_make_loops` / `make_perimeter_and_infill` (wall-then-fill split; `area_group.need_infill` gating).
- `OrcaSlicerDocumented/src/libslic3r/Support/SupportParameters.hpp` — constructor density derivations: `support_density = min(1., support_material_flow.spacing() / support_spacing)` with `support_spacing = support_base_pattern_spacing + support_material_flow.spacing()`; `top_interface_density` / `bottom_interface_density` analogues; `number_of_support_interface_bottom_layers` (returns top count when bottom < 0, else bottom).
- `OrcaSlicerDocumented/src/libslic3r/Flow.cpp` — `support_material_interface_flow` (interface flow width from the interface flow ratio over `support_line_width` falling back to `line_width`; the interface pitch source is the resulting `spacing()`).
- `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` — `PrintConfigDef::build` (confirms `support_bottom_interface_spacing` default 0.5 min 0; confirms NO `support_density` percentage key and NO `support_interface_line_width` key exist canonically).

## Acceptance Summary

Reference, never copy, criteria from `packet.spec.md`.

- Positive: `AC-1` (hollow walls, E1 structural assertion) … `AC-12` (DEV-129
  close-or-finish evidence), as Given/When/Then with pipe commands there.
- Negative: `AC-N1` (density clamp to solid), `AC-N2` (non-positive interface-flow guard).
- Cross-packet impact: consumes 238a's typed keys (`float_or_percent`
  `support_line_width`, `support_threshold_overlap`, spacing keys) and 238b's wall-count
  skeleton transport (`list<u32>` `wall_counts`); hands 241 a stable pitch surface; hands
  242 the corrected interface counts and marker semantics.

## Verification Commands

This is the authoritative full matrix; `packet.spec.md` lists only 2-3 gate commands.

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo test -p tree-support --test tree_support_tdd -- tree_bodies_render_hollow_concentric_walls --exact 2>&1 \| tee target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` | AC-1 hollow-wall structure | FACT pass/fail |
| `cargo test -p slicer-core --features host-algos --test support_flow_semantics_tdd -- canonical_density_derivations_match_formulas densities_clamp_to_one_solid_pitch --exact 2>&1 \| tee target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` | AC-2 + AC-N1 density math (E6 feature flag) | FACT pass/fail |
| `rg -q 'MAX_BRANCH_RADIUS_MM: f32 = 10\.0' modules/core-modules/tree-support-planner/src/lib.rs && cargo test -p tree-support-planner --test tree_family_tdd -- branch_radius_clamps_at_canonical_maximum --exact 2>&1 \| tee target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` | AC-3 cap constant + behavior | FACT pass/fail |
| `cargo test -p tree-support-planner --test tree_family_tdd -- radius_raises_to_base_under_interfaces base_interface_band_attributed_in_plan_roles --exact 2>&1 \| tee target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` | AC-4 + AC-6 planner behavior | FACT pass/fail |
| `cargo test -p slicer-runtime --test integration -- interface_band_counts_match_canonical_structure interface_layer_count_follows_config --exact 2>&1 \| tee target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` | AC-5 band counts incl. ee27ac94 pins | FACT pass/fail |
| `cargo build --tests -p slicer-wasm-host && cargo xtask build-guests --check` | AC-7 WIT + guest freshness (exit 0) | FACT pass/fail + exit code |
| `cargo test -p slicer-gcode --lib -- base_interface_role_maps_to_support_interface_marker --exact 2>&1 \| tee target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` | AC-8 marker decision | FACT pass/fail |
| `test ! -f modules/core-modules/tree-support/src/interface_regularize.rs && test ! -f modules/core-modules/traditional-support/src/interface_regularize.rs && rg -q 'pub fn regularize_entry_roles' crates/slicer-core/src/support_regularize.rs` | AC-9 single-source proof | FACT pass/fail |
| `rg -q 'default = 0\.5' modules/core-modules/traditional-support/traditional-support.toml && rg -q 'default = 0\.5' modules/core-modules/tree-support/tree-support.toml` | AC-10 DEV-145 defaults | FACT pass/fail |
| `cargo test -p traditional-support --test traditional_support_tdd -- interface_pitch_derives_from_interface_flow_over_line_width --exact 2>&1 \| tee target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` | AC-11 DEV-146 mechanism | FACT pass/fail |
| `! rg -q 'Not yet implemented' modules/core-modules/tree-support-planner/tree-support-planner.toml` | AC-12 stale-claim removal | FACT pass/fail |
| `cargo check --workspace --all-targets` | whole-tree compile incl. test targets | FACT pass/fail |
| `cargo clippy --workspace --all-targets -- -D warnings` | lint gate | FACT pass/fail |
| `cargo xtask check-literals` | struct-literal churn gate | FACT pass/fail |

## Step Completion Expectations

- After ANY step touching `modules/*/src/**`, `modules/*/*.toml`, or
  `crates/slicer-schema/wit/**`: run `cargo xtask build-guests --check` before attributing
  any downstream failure (T4/E4); after WIT edits specifically also run
  `cargo build --tests`.
- Steps adding enum variants own every newly-non-exhaustive `match` in the same step
  (blast-radius discipline; `cargo build --tests` finds them — fix forward, never
  `_ =>`-silence a support-role match).
- Red-first: each behavior step's named test must fail before its edit and pass after.

## Context Discipline Notes

- `modules/core-modules/tree-support-planner/src/lib.rs` is ~5.9k lines: ranged reads
  around cited symbols only (`MAX_BRANCH_RADIUS_MM`, `calc_branch_radius` clamp,
  `InterfaceRole`, node-classification loop); never full-read.
- Both renderers are ~600 lines: full-read allowed once each, then symbol-targeted.
- Never load `OrcaSlicerDocumented/` or golden fixture bodies; delegate per snippet.
- Guest-freshness checks (`build-guests --check`) are cheap exit-code probes — always run
  rather than reason about staleness.
