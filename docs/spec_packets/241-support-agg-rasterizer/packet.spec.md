---
status: draft
packet: 241-support-agg-rasterizer
task_ids:
  - TASK-419
  - TASK-420
  - TASK-421
  - TASK-422
  - TASK-423
  - TASK-424
  - TASK-425
  - TASK-426
  - TASK-427
  - TASK-428
depends_on: 238c-support-renderer-flow-interfaces
backlog_source: docs/specs/support-families-anchored-entities-plan.md
context_cost_estimate: M
---

# Packet Contract: 241-support-agg-rasterizer

## Goal

Port the canonical `SupportGridPattern` AGG rasterization path (`SupportMaterial.cpp`) into the
traditional planner's area propagation as `support_area_rasterizer = agg` (canonical, default),
keeping the current propagate-without-growth semantic selectable as `legacy_semantic`, with
before/after wall-leakage (collision freedom) and column-continuity (coverage) measurements as
the acceptance gate (plan §3 Rulings 7/8, §7 E1/E2).

## Scope Boundaries

The traditional planner's per-layer area propagation only:
`modules/core-modules/traditional-support-planner/` — one new grid-rasterizer module in the
guest, one manifest knob, and the propagation loop that consumes it. Renderer flow/density is
238c (done there); tree-side rasterization does not exist canonically for tree styles (canonical
maps every tree style to `smsGrid`, but this packet wires the knob only where PnP's traditional
planner propagates area); raft is 240a-support-raft-substrate / 240b-support-raft-module;
independent support-layer Z is 239-support-independent-layer-z.

## Prerequisites and Blockers

- Depends on: `238c-support-renderer-flow-interfaces` — SATISFIED. Verified 2026-09-03:
  `docs/spec_packets/238c-support-renderer-flow-interfaces/packet.spec.md` frontmatter reads
  `status: implemented`, as do `236-support-stabilization`, `238a-support-pattern-config-keys`,
  and `238b-tree-planner-canonical-fidelity`. The chain 236 → 238a → 238b → 238c is fully
  landed, so no forward dependency blocks activation. This is a ledger fact — re-derive it at
  activation (`head -3` each dep's `packet.spec.md`) rather than trusting this line.
- Unblocks: `242-support-family-orca-closure`.
- Activation blockers: none beyond the dependency above; `[BLOCK]`-tagged questions live in
  `design.md` §Open Questions.

## Acceptance Criteria

- **AC-1 (knob declared).** Given
  `modules/core-modules/traditional-support-planner/traditional-support-planner.toml`,
  **when** its `[config.schema]` is inspected, **then** a `support_area_rasterizer` table
  exists with `type = "enum"`, `values = ["agg", "legacy_semantic"]`, `default = "agg"` —
  following the manifest enum pattern of `retract_mode`
  (`modules/core-modules/path-optimization-default/path-optimization-default.toml`) — and
  `docs/15_config_keys_reference.md` names the key with its two values (T8: declaration +
  doc regen in one commit). | `rg -q '^\[config\.schema\.support_area_rasterizer\]' modules/core-modules/traditional-support-planner/traditional-support-planner.toml && rg -q '"agg", "legacy_semantic"' modules/core-modules/traditional-support-planner/traditional-support-planner.toml && rg -q 'support_area_rasterizer' docs/15_config_keys_reference.md && echo PASS`
- **AC-2 (grid construction is canonical).** Given the new guest-side rasterizer module, **when**
  support polygons are projected onto the byte grid at default config, **then** the construction
  matches canonical `SupportGridPattern`'s `#ifdef SUPPORT_USE_AGG_RASTERIZER` branch exactly at
  PnP scale: pixel size `max(extrusion_width_scaled + 21, spacing_scaled / oversampling)` with
  `oversampling = clamp(spacing_scaled / (extrusion_width_scaled + 100), 1, 8)`, macro blocks of
  `oversampling × oversampling` cells, a one-pixel empty boundary ring, and seed-fill over each
  macro block up to the 3×3-dilated trimming mask (`seed_fill_block` + `dilate_trimming_region`
  semantics, four-direction propagation steps). | `mkdir -p target && cargo test -p traditional-support-planner --test agg_rasterizer_tdd grid_construction_matches_canonical_formulas -- --exact 2>&1 | tee target/test-output.log && grep -q "^test result: ok" target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0 && echo PASS`
- **AC-3 (contour extraction + island filtering).** Given a filled byte grid, **when**
  contours are extracted, **then** the extraction chains cell-boundary edges into closed loops
  (marching-squares equivalent of canonical `contours_simplified`), honors `fill_holes`
  left/right + top/bottom neighbor filling, applies `offset_in_grid` expansion/shrink via the
  loop offset, splits islands by the trimming polygons via
  `host::clip_polygons(.., ClipOperation::Difference)` (canonical uses `difference_ex`; in
  this tree that symbol is NOT re-exported by `slicer-sdk` and `slicer-core` is not a
  dependency of this module, so it is unavailable here — note it is NOT a host/guest boundary:
  `slicer_core::polygon_ops` is ungated and does compile to wasm32, as `arachne-perimeters`
  demonstrates. We route through the SDK host op rather than adding a `slicer-core`
  dependency, keeping this module's dependency surface unchanged), and keeps only islands
  containing an input-island sample point (canonical `extract_support`'s sample-containment
  filter — the column-continuity fix from upstream `a95607d7bf`). | `cargo test -p traditional-support-planner --test agg_rasterizer_tdd contour_extraction_filters_islands_by_samples -- --exact 2>&1 | tee target/test-output.log && grep -q "^test result: ok" target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0 && echo PASS`
- **AC-4 (in-cell expansion restriction).** Given an extracted layer polygon with positive
  `expansion_to_slice`, **when** the printed area is derived, **then** expansion happens inside
  each oversampled macro cell during extraction (per-cell `offset_in_grid`), never as a global
  polygon offset — the wall-leakage fix from upstream `fb7b995050`. No `host::offset_polygons`
  call may appear inside `agg_raster.rs` at all: island-sample generation is the one step that
  needs an offset, and it runs at the `lib.rs` call site and is passed into `extract_support`
  as its `samples` argument, so the rasterizer module is offset-free by construction. | `( ! rg -q 'offset_polygons' modules/core-modules/traditional-support-planner/src/agg_raster.rs ) && mkdir -p target && cargo test -p traditional-support-planner --test agg_rasterizer_tdd expansion_is_restricted_inside_the_macro_cell -- --exact 2>&1 | tee target/test-output.log && grep -q "^test result: ok" target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0 && echo PASS`
- **AC-5 (propagation consumes the rasterizer by default).** Given default config
  (`support_area_rasterizer` unset or `"agg"`), **when** the traditional planner propagates a
  contact region downward through ≥ 2 layers against model occupancy, **then** the emitted
  per-layer body comes from the rasterizer path (contact polygons stretched into the grid,
  trimmed by occupancy, re-extracted), while termination-layer bookkeeping (structured
  `NoRoute` decline when occupancy closes every route), interface anchoring, and demand/body ID
  threading are unchanged from today. | `cargo test -p traditional-support-planner --test agg_rasterizer_tdd default_config_routes_propagation_through_rasterizer -- --exact 2>&1 | tee target/test-output.log && grep -q "^test result: ok" target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0 && echo PASS`
- **AC-6 (wall-leakage measurement — collision freedom).** Given the tracked fixture sliced via
  `run_slice` before and after this packet (self-captured baseline vs post-port, same config),
  **when** support body outlines are tested against per-layer model occupancy grown by
  `support_object_xy_distance`, **then** the post-port run measures **zero penetration events**
  and a strictly smaller total penetrated-area sum than the pre-port baseline recorded in Step 1
  (E1: measured numbers recorded in the test output and `requirements.md`; existence checks do
  not satisfy this AC). The existing wedge invariant
  `support_segments_stay_outside_the_model_and_within_the_build_volume` stays green under both
  modes. | `cargo test -p slicer-runtime --test integration -- agg_wall_leakage_measurement_beats_baseline --exact 2>&1 | tee target/test-output.log && grep -q "^test result: ok" target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0 && echo PASS`
- **AC-7 (column-continuity measurement — coverage).** Given the same fixture runs, **when**
  per-column coverage across consecutive layers is compared (column = connected body component
  tracked down-layer), **then** the post-port run has strictly fewer abrupt column drops than the
  pre-port baseline recorded in Step 1 (columns "missing abruptly when going down" is the
  upstream `a95607d7bf` symptom), and total emitted support area changes by less than ±25%
  versus baseline so continuity is not bought by inflation. Measured deltas are recorded in the
  test output and `requirements.md`. | `cargo test -p slicer-runtime --test integration -- agg_column_continuity_measurement_beats_baseline --exact 2>&1 | tee target/test-output.log && grep -q "^test result: ok" target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0 && echo PASS`
- **AC-8 (both modes diverge measurably).** Given one fixture slice per mode
  (`support_area_rasterizer = agg` vs `"legacy_semantic"`) through `run_slice`, **when** both
  plans are compared, **then** they produce different body outline sets on at least one layer
  (proof the knob actually switches code paths), and BOTH runs complete with non-empty support
  plans reaching the plate beneath the fixture overhang. | `cargo test -p slicer-runtime --test integration -- agg_and_legacy_modes_both_function_and_diverge --exact 2>&1 | tee target/test-output.log && grep -q "^test result: ok" target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0 && echo PASS`

Every AC names exact fields, paths, counts, or output fragments and ends with its own runnable
command. Commands tee to `target/test-output.log` with a non-zero matched-count guard
(invariant 16).

## Negative Test Cases

- **AC-N1 (invalid knob value rejected).** Given a config supplying
  `support_area_rasterizer = "marching_squares"` (not in the declared enum set), **when** the
  traditional planner module parses its config view, **then** it fails with a fatal
  `ModuleError` naming the key and the allowed values — the defense-in-depth pattern already used
  by `SeamPlacer::from_config` (`modules/core-modules/seam-placer/src/lib.rs`), which rejects an
  unknown `seam_mode` with `ModuleError::fatal` even though `seam_mode` is a manifest-declared
  enum. No silent fallback to either mode. NOTE: the host rejects out-of-vocabulary enum values
  first — `ConfigBoundsIndex::from_modules` harvests `values` from every loaded module's
  `[config.schema]` and `resolve_global_config` calls `bounds.check(..)?`, aborting the slice
  with `config resolution failed: …`. The host check is therefore NOT numeric-only. The guest
  check still carries weight (it fires for a `ConfigView` built directly, when no loaded module
  declares the key, and when a colliding declaration wins `or_insert_with`), so this AC's test
  drives `from_config` on a constructed `ConfigView` rather than a full slice. | `mkdir -p target && cargo test -p traditional-support-planner --test agg_rasterizer_tdd invalid_rasterizer_value_is_rejected_not_defaulted -- --exact 2>&1 | tee target/test-output.log && grep -q "^test result: ok" target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0 && echo PASS`
- **AC-N2 (legacy mode still functions).** Given `support_area_rasterizer = "legacy_semantic"`
  explicitly selected, **when** the planner runs the full propagation suite inputs (blocked
  route → structured decline; plate termination; top-z lowering), **then** all existing
  `traditional_family_tdd` assertions hold unchanged — proving Ruling 8's "prior behavior stays
  selectable" and guarding against silent degradation of the legacy path. |
  `mkdir -p target && cargo test -p traditional-support-planner --test traditional_family_tdd 2>&1 | tee target/test-output.log && grep -q "^test result: ok" target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 10 && echo PASS`

## Verification

- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo xtask check-literals`
- Primary targeted proof: `mkdir -p target && cargo test -p traditional-support-planner --test agg_rasterizer_tdd 2>&1 | tee target/test-output.log && grep -q "^test result: ok" target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -ge 6 && echo PASS`

## Authoritative Docs

- `docs/specs/support-families-anchored-entities-plan.md` - governing plan; §12 brief
  "241-support-agg-rasterizer", §3 Rulings 7/8, §6 invariant 16, §7 E1–E9, §8 human gate,
  §13 traps T1/T4/T5/T7. Bounded ranged reads.
- `docs/specs/support-parity-gap-register.md` - row G-07 (premise corrected per Ruling 7;
  destination rerouted to this packet); direct range read.
- `docs/19_visual_debug.md` - visual-debug bundle contract for the human-gate taps; ranged
  read around `## Request Shape` and `### Tap Classes And Execution Closure`. The "Stage Tap
  Inventory" heading itself is NOT in this file — it lives in
  `docs/specs/_OLD/visual-pipeline-debug.md`, which `19_visual_debug.md` only references.
- `docs/specs/support-families-anchored-entities-plan.md` §17-agent debugging companion
  (`docs/17_agent_debugging.md`) - timing/DAG diagnosis boundaries; consult only if a gate
  command misbehaves.
- `docs/15_config_keys_reference.md` - regenerated, not read as authority.

## Doc Impact Statement (Required)

- `docs/15_config_keys_reference.md` - add `support_area_rasterizer` row (enum, default
  `"agg"`, values `"agg"|"legacy_semantic"`, owner `traditional-support-planner`) after the
  manifest lands - `rg -q 'support_area_rasterizer' docs/15_config_keys_reference.md`
- `docs/07_implementation_status.md` - TASK-419..TASK-428 registered at packet-owned closure
  (Step 9) - `rg -q 'TASK-419' docs/07_implementation_status.md`. These IDs are RESERVED for
  this packet by queue row #8 of `docs/specs/support-families-anchored-entities-plan.md` and
  are below the live high-water mark in `docs/07_implementation_status.md`. Step 9 must verify
  the reserved range is still unused (`rg -o 'TASK-4(1[9]|2[0-8])' docs/07_implementation_status.md`
  returns nothing), NOT allocate the "next free" ID — that query returns a much higher number.
- `docs/DEVIATION_LOG.md` - no edit (G-07 premise correction lives in the gap register row
  itself; no new deviation is filed by this packet — the port IS the canonical behavior).
- Queue table of `docs/specs/support-families-anchored-entities-plan.md` - orchestrator-owned;
  this packet does not touch it.

## Human Validation Gate

Blocking per plan §8. Artifacts to produce (all under `tmp/p241-*`, gitignored — verify by
direct listing, trap T1):

1. `tmp/p241-agg-fixture.gcode` — tracked fixture
   `crates/slicer-runtime/tests/fixtures/support-family/SupportTest.stl`, matched profile
   `tmp/support-family-config-normal-matched.json`, default (agg) mode:
   `cargo run --bin pnp_cli --release -- slice --model crates/slicer-runtime/tests/fixtures/support-family/SupportTest.stl --config tmp/support-family-config-normal-matched.json --output tmp/p241-agg-fixture.gcode`
   (the `slice` subcommand's model flag is `--model` per `Cmd::Slice` in
   `crates/pnp-cli/src/main.rs`; the `--input` spelling in AGENTS.md is a doc-level alias
   example — use the flag the CLI actually parses).
2. `tmp/p241-legacy-fixture.gcode` — identical except the profile JSON carries
   `"support_area_rasterizer": "legacy_semantic"` → `tmp/p241-legacy-fixture.gcode`. BOTH
   modes inspected (plan §12 brief).
3. **Non-coplanar real-mesh case (T7 — mandatory):** slice `resources/regression_wedge.stl`
   through the full pipeline in default mode → `tmp/p241-agg-wedge.gcode`.
4. Visual-debug bundle for THIS packet's boundary — wall-leakage tap (support body vs model
   occupancy at mid-height layers) and column-continuity tap (consecutive body layers), written
   under `tmp/p241-vd/` with its `manifest.json`.

Checklist to sign (each item names source, layer, tap, verdict; per E2 written inspection,
never a test claim):

- [ ] Termination: columns reach the plate/model beneath their overhangs in BOTH modes; no
      column terminates short or passes through the model.
- [ ] Coverage: demanded overhang regions carry support on the fixture; no column vanishes
      abruptly going down in agg mode (the G-07 symptom).
- [ ] Collision freedom: no support intersects model walls in agg mode — inspect the
      wall-leakage tap at thin-wall layers where the legacy path leaked.
- [ ] Interfaces: roofs/floors sit correctly in both modes (no regression vs 238c state).
- [ ] Block counts vs Orca references (REQUIRED): `;TYPE:Support material` block counts and
      total support-extrusion length for `tmp/p241-agg-fixture.gcode` vs
      `tmp/SupportTest_Normal_Orca.gcode` recorded in writing; delta stated, not guessed.
- [ ] Rasterizer-specific observations: wall-leak/column-continuity verdict per mode, with
      layer indices of any visible difference between `tmp/p241-agg-fixture.gcode` and
      `tmp/p241-legacy-fixture.gcode`.

Sign-off: `_date_ _verdict_` (packet may not flip to `status: implemented` without it).

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/Support/SupportMaterial.cpp` — class `SupportGridPattern`: constructor `smsGrid` branch (oversampling clamp formula, `m_pixel_size`, macro-block sizing, one-pixel boundary ring, `rasterize_polygons` calls, `seed_fill_block` over `dilate_trimming_region`), static `rasterize_polygons` (AGG gray8 scanline fill semantics being replicated), static `contours_simplified` (cell-edge collection, line chaining, `fill_holes` neighbor rule, `offset_in_grid` loop offset), `extract_support` (island split vs trimming polygons, `island_samples` containment filter, expansion-vs-shrink sample handling), static `seed_fill_block` / `dilate_trimming_region` (macro-cell 4-direction propagation, 3×3 dilation mask); instantiation site in the support-layers builder path (~`generate_support_layers` region) showing which callers pass `expansion_to_propagate` vs `expansion_to_slice`.
- `OrcaSlicerDocumented/src/libslic3r/Support/SupportMaterial.hpp` — `SupportGridParams` field meanings (`grid_resolution`, `extrusion_width`, `support_closing_radius`, `support_angle`, style) consumed by the constructor.

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- obey the shared absolute context bands: 120k reading budget with hand-off at 150k (standard); the extended band (240k reading / 300k hard stop) only via swarm's escalation protocol

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation (an extended-band run may carry a single L step only when `design.md` justifies why it cannot be split).
