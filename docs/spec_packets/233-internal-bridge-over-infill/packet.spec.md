---
status: draft
packet: internal-bridge-over-infill
task_ids:
  - ISSUE-82
backlog_source: docs/specs/orca-feature-gap/issues/82-author-packet-p75-quality-bridging-bridge-over-infill.md
context_cost_estimate: M
---

# Packet Contract: internal-bridge-over-infill

Queue row #1 of `docs/specs/bridge-parity-plan.md` (work item W-C, sequencing D3: internal-first).
Owns config keys `dont_filter_internal_bridges`, `enable_extra_bridge_layer`, `internal_bridge_angle`.

## Goal

Introduce the internal bridge-over-infill decision at the post-surface/infill seam (`Layer::InfillPostProcess`), constructing anchored bridge polygons with a windowed-mean angle per canonical `PrintObject::bridge_over_infill`, and thread a proper `ExtrusionRole::InternalBridgeInfill` variant through IR/WIT/host/marshal/gcode — bundling the sparse ±90° alternation fix (D11/F7) and canonical `bridging_flow` spacing + decoupled bridge feedrate (F5/F6). This is a NEW decision, not a move: at HEAD no internal-bridge decision exists in the prepass (see AC-N2).

## Scope Boundaries

In: internal bridge-over-infill introduction at the post-surface/infill seam with anchor-polyline generation, `determine_bridging_angle` (±18° sliding-window length-weighted mean) and `construct_anchored_polygon` (scan lines every `bridging_flow.scaled_spacing()`, clipped to anchors/walls) ports; the `InternalBridgeInfill` enum variant retiring the stash's `Custom("InternalBridge")` tag (stash-only — nothing emits it at HEAD) and the dead/reserved `"InternalBridge"` feedrate mapping; `bridging_flow` canonicalization (`bridge_line_width` selection + `BRIDGE_EXTRA_SPACING` 0.05 mm); per-role feedrate decoupling; removal of odd-layer +90° sparse alternation. Out: external-bridge false-site gating / unsupported-span test (packet 234, W-A) and external orientation `detect_bridging_direction` port (packet 235, W-B); `assemble_bridge_areas` stays untouched.

## Prerequisites and Blockers

- Depends on: nothing (queue row #1).
- Unblocks: `234-bridge-false-site-gating`, `235-external-bridge-orientation`.
- Activation blockers: `stash@{0}` pops at the FIRST implementation session (plan D10); keep flag threading/routing/label mapping, discard the orientation heuristic and contour-band expansion (`INTERNAL_BRIDGE_EXPANSION_MULTIPLIER`) — this packet replaces them.

## Acceptance Criteria

State ACs only here; `requirements.md` references their IDs.

- **AC-1. Given** the packet implemented, **when** the IR and WIT role surfaces are inspected, **then** `ExtrusionRole::InternalBridgeInfill` exists in `crates/slicer-ir/src/slice_ir.rs` and the canonical WIT `extrusion-role` enum under `crates/slicer-schema/wit/` carries the matching variant. | `rg -q 'InternalBridgeInfill' crates/slicer-ir/src/slice_ir.rs && (rg -q 'internal-bridge-infill' crates/slicer-schema/wit || rg -q 'InternalBridgeInfill' crates/slicer-schema/wit)`
- **AC-2. Given** an extrusion path whose role is `InternalBridgeInfill`, **when** gcode feedrate is resolved, **then** the emitted feedrate equals `internal_bridge_speed` (default 37.5 mm/s → `F2250`) and the `;TYPE:` label is `Internal Bridge`, verified by a packet-added test in the existing `crates/slicer-gcode/tests/gcode_feedrate_emission_tdd.rs`. | `cargo test -p slicer-gcode --test gcode_feedrate_emission_tdd -- internal_bridge 2>&1 | tee target/test-output.log | tail -5`
- **AC-3. Given** the ported windowed-mean angle function, **when** packet-added unit tests feed it (a) an asymmetric anchor set and (b) the same set with `internal_bridge_angle = 45` override active, **then** (a) returns the length-weighted mean of the anchors' nearest orientations (test asserts the exact expected float, never a frozen 0°/90°) and (b) returns exactly 45.0. | `cargo test -p slicer-core --test bridge_over_infill_tdd -- bridging_angle 2>&1 | tee target/test-output.log | tail -5`
- **AC-4. Given** a rectangular void span of known width W and `bridging_flow` spacing S, **when** the ported anchored-polygon construction runs, **then** the produced bridge line count equals `round(W / S)` ± 1 (invariant I5), asserted by a packet-added unit test. | `cargo test -p slicer-core --test bridge_over_infill_tdd -- anchored_polygon 2>&1 | tee target/test-output.log | tail -5`
- **AC-5. Given** a committed layer containing both sparse infill and internal bridge regions, **when** the packet-added integration test inspects the role-partition polygons, **then** `InternalBridgeInfill` and `SparseInfill` polygons are pairwise disjoint (bridge area subtracted from `stInternal` equivalent) — invariant I6. | `cargo test -p slicer-runtime --test integration -- internal_bridge_disjoint 2>&1 | tee target/test-output.log | tail -5`
- **AC-6. Given** `resources/bridge.obj` (nominated model — see `requirements.md` §Acceptance Summary for the choice + why), **when** sliced twice with `--config` JSON files varying only `infill_speed` (40 vs 120), **then** internal-bridge move feedrates are identical across both runs and equal the resolved `internal_bridge_speed` (default 37.5 mm/s → `F2250`) — invariant I7. | `cargo run --bin pnp_cli --release -- slice --model resources/bridge.obj --config resources/test_config/ac6_infill_40.json --output target/ac6_a.gcode --module-dir modules/core-modules 2>&1 | tee target/test-output.log | tail -5 && cargo run --bin pnp_cli --release -- slice --model resources/bridge.obj --config resources/test_config/ac6_infill_120.json --output target/ac6_b.gcode --module-dir modules/core-modules 2>&1 | tee target/test-output.log | tail -5 && python3 -c "import re;f=lambda p:[float(m.group(1)) for m in re.finditer(r'G1 X[\d.]+ Y[\d.]+ E[\d.]+ F([\d.]+)', open(p).read().split(';TYPE:Internal Bridge')[-1])]; assert f('target/ac6_a.gcode')==f('target/ac6_b.gcode')==[2250.0]*len(f('target/ac6_a.gcode')), 'I7 violated'"`
- **AC-7. Given** the rectilinear-infill module, **when** a packet-added module test runs infill at `layer_index` 0 and 1 with identical geometry, **then** the emitted infill direction angle is identical (no odd-layer +90°) — D11/F7. | `cargo test -p rectilinear-infill --test rectilinear_infill_tdd -- alternation 2>&1 | tee target/test-output.log | tail -5`
- **AC-8. Given** `bridging_flow` canonicalization, **when** packet-added unit tests call it with `bridge_line_width` set and unset, **then** `thread_diameter = bridge_line_width` when set else `nozzle_diameter`, and `spacing = dmr + 0.05 mm` (`BRIDGE_EXTRA_SPACING`). | `cargo test -p slicer-core --test bridge_over_infill_tdd -- bridging_flow 2>&1 | tee target/test-output.log | tail -5`
- **AC-9. Given** the module manifest and host config plumbing, **when** the three owned keys are queried, **then** `dont_filter_internal_bridges`, `enable_extra_bridge_layer`, and `internal_bridge_angle` exist in snake_case. | `rg -q 'dont_filter_internal_bridges' modules/core-modules && rg -q 'enable_extra_bridge_layer' modules/core-modules && rg -q 'internal_bridge_angle' modules/core-modules`

## Negative Test Cases

- **AC-N1. Given** the D7 variant landed, **when** the retired string-tag machinery is searched, **then** the dead/reserved `"InternalBridge"` feedrate mapping (at HEAD in `crates/slicer-gcode/src/emit.rs`), the stash's `Custom("InternalBridge")` emission, and the stash's `is_internal_bridge` flag all survive nowhere in IR/WIT/gcode/module surfaces. | `! rg -q '"InternalBridge"' crates/slicer-gcode/src crates/slicer-ir/src modules/core-modules/rectilinear-infill/src && ! rg -q 'is_internal_bridge' crates/slicer-ir/src crates/slicer-schema/wit`
- **AC-N2. Given** the introduction at the seam, **when** the prepass is inspected, **then** `commit_shell_classification_builtin` in `crates/slicer-runtime/src/slice_postprocess_prepass.rs` contains no internal-bridge decision logic and no contour-band expansion constant. This is a post-implementation GUARD, not a removal: at HEAD the prepass already contains zero internal-bridge logic, and `INTERNAL_BRIDGE_EXPANSION_MULTIPLIER = 3.0` exists only inside `stash@{0}`'s copy of the file (plan §3/F4) — it never landed at HEAD. The guard prevents the seam introduction from accidentally reintroducing either. | `! rg -q 'INTERNAL_BRIDGE_EXPANSION_MULTIPLIER|[Ii]nternal[Bb]ridge' crates/slicer-runtime/src/slice_postprocess_prepass.rs`
- **AC-N3. Given** the F5 canonicalization, **when** the module is inspected, **then** the stash's module-level +0.05 mm spacing shim is retired (spacing lives only in `crates/slicer-core/src/flow.rs`). | `! rg -q '0\.05' modules/core-modules/rectilinear-infill/src/lib.rs && rg -q 'BRIDGE_EXTRA_SPACING|bridge_extrusion_spacing' crates/slicer-core/src/flow.rs`
- **AC-N4. Given** test-code struct literals touched by the new variant/fields, **when** the churn gate runs, **then** no watched-type literal lacks `..` rest or an `// exhaustive:` waiver. | `cargo xtask check-literals`

## Verification

- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo xtask build-guests --check` (exit 0 required; WIT + module edits stale the guests)
- `cargo test -p slicer-core --test bridge_over_infill_tdd` (the new geometry/flow unit suite; ungated — no `--features host-algos` needed)

## Authoritative Docs

- `docs/specs/bridge-parity-plan.md` - read directly; §0 decisions, §3 findings F3/F5/F6/F7/F8, §4 W-C row, §6 invariants I4–I7.
- `docs/adr/0061-deterministic-bridge-orientation-tie-break.md` - read directly; tie-break rule (smallest quantized angle) — reference, never recreate.
- `docs/08_coordinate_system.md` - read directly before porting any scaled/mm constant.
- `docs/21_data_defaults_and_fixtures.md` - delegated SUMMARY of the struct-literal waiver rule only.

## Doc Impact Statement (Required)

Specific same-packet doc edits; each with a verification grep:

- `docs/02_ir_schemas.md` section documenting `ExtrusionRole` - add `InternalBridgeInfill`; grep: `rg -q 'InternalBridgeInfill' docs/02_ir_schemas.md`.
- `docs/03_wit_and_manifest.md` `extrusion-role` mention - add the new variant; grep: `rg -q 'InternalBridgeInfill\|internal-bridge-infill' docs/03_wit_and_manifest.md`.
- `docs/15_config_keys_reference.md` - add `dont_filter_internal_bridges`, `enable_extra_bridge_layer`, `internal_bridge_angle`; grep: `rg -q 'internal_bridge_angle' docs/15_config_keys_reference.md`.

These greps are appended to the AC set and must pass before `status: implemented`.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/PrintObject.cpp` — `PrintObject::bridge_over_infill` orchestration inside `prepare_infill()` (after `process_external_surfaces`/`clip_fill_surfaces`); the `determine_bridging_angle` lambda (length-weighted mean over a ±18° sliding window of nearest-anchor orientations) and `construct_anchored_polygon` lambda (scan lines every `bridging_flow.scaled_spacing()`, clipped to anchors/walls); `dont_filter_internal_bridges` / `enable_extra_bridge_layer` / `internal_bridge_angle` consumption.
- `OrcaSlicerDocumented/src/libslic3r/Fill/Fill.cpp` — `Layer::generate_sparse_infill_polylines_for_anchoring` (anchor polyline provenance), `calculate_infill_rotation_angle` (rotate-template handling we deliberately do NOT port), and the per-role `role_speed` assignment block (Q1 evidence: each role gets its own config speed, no shared coupling).
- `OrcaSlicerDocumented/src/libslic3r/Fill/FillBase.cpp` + `OrcaSlicerDocumented/src/libslic3r/Fill/FillRectilinear.hpp` — `Fill::_infill_direction` applies `_layer_angle` only when not fixed-angle and not `dont_alternate_fill_direction`; `FillRectilinear::_layer_angle` returns 0 (D11/F7 ground truth).
- `OrcaSlicerDocumented/src/libslic3r/Flow.hpp` + `OrcaSlicerDocumented/src/libslic3r/Flow.cpp` — `Flow::bridging_flow`, `Flow::bridge_extrusion_spacing(dmr) = dmr + BRIDGE_EXTRA_SPACING (0.05 mm)` (F5).
- `OrcaSlicerDocumented/src/libslic3r/LayerRegion.cpp` — `LayerRegion::bridging_flow` selects `thread_diameter = bridge_line_width if set else nozzle_diameter`; `process_external_surfaces` ordering context (expansion_step = scaled(0.1), ≤5 steps, expansion_bottom_bridge = shell_width·sqrt(2)).
- `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` — defaults and enum value sets for the three owned keys.
- `OrcaSlicerDocumented/src/libslic3r/ExtrusionEntity.cpp` — `erInternalBridgeInfill` label text `Internal Bridge` (borrowed); fan-marker handling (deliberately NOT borrowed — deferred per Q2).

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- obey the shared absolute context bands: 120k reading budget with hand-off at 150k (standard); the extended band (240k reading / 300k hard stop) only via swarm's escalation protocol

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation (an extended-band run may carry a single L step only when `design.md` justifies why it cannot be split).
