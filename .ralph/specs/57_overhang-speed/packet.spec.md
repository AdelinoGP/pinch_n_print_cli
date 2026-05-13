---
status: draft
packet: 57_overhang-speed
task_ids:
  - TASK-182
backlog_source: docs/07_implementation_status.md
context_cost_estimate: M
---

# Packet Contract: 57_overhang-speed

## Goal

Wire OrcaSlicer-parity overhang quartile speed end-to-end on the live G-code path: extend the `point3-with-width` WIT record with an `overhang-quartile: option<u8>` field, add a host-side `overhang_classifier` prepass that buckets wall-family vertices by signed distance to the previous-layer support polygons, and extend `resolve_feedrate` to dispatch `OuterWall | InnerWall | ThinWall` points to the four `overhang_{1,2,3,4}_4_speed` keys registered by packet 52. First remediation against DEV-009 for *quality-modulated* feedrate beyond the bare per-role tokens.

## Scope Boundaries

- In scope:
  - `point3-with-width` WIT record field addition (`overhang-quartile: option<u8>`) at `wit/deps/types.wit` and host/guest binding fan-out per the WIT checklist in `CLAUDE.md`.
  - Rust mirror on `Point3WithWidth` in `crates/slicer-ir/src/slice_ir.rs` with `#[serde(default)]`; bump the schema minor-version constant.
  - New module `crates/slicer-core/src/aabb_lines_2d.rs` providing `LinesDistancer2D` (naïve linear scan + bbox prefilter; BVH deferred).
  - New module `crates/slicer-host/src/overhang_classifier.rs` with `pub fn classify_layers(layers: &mut [LayerCollectionIR], feedrate_config: &FeedrateConfig)`.
  - `resolve_feedrate` signature extension + per-point dispatch in `crates/slicer-host/src/gcode_emit.rs`.
  - Pipeline wire-in (both slicer-cli and WASM arms) in `crates/slicer-host/src/pipeline.rs`.
  - New test file `crates/slicer-host/tests/overhang_speed_tdd.rs` covering the six ACs plus the negative case.
  - Remediation note appended to `docs/DEVIATION_LOG.md` for DEV-009.

- Out of scope:
  - Mid-segment intersection insertion (OrcaSlicer `ADD_INTERSECTIONS`) — splitting paths at distance-threshold crossings rather than classifying existing vertices.
  - Bridge-perimeter as a distinct `ExtrusionRole` variant. Today an outer-wall path over a bridge is `OuterWall` and is geometry-classified to quartile 1.
  - Curled-edge slowdown (`prev_curled_extrusions`) — depends on a support-spot generator not yet implemented.
  - Production BVH inside `LinesDistancer2D`.
  - Smoothed/interpolated speed (Orca offers both; ship quantized first).
  - `ConfigValue::FloatOrPercent` (defer with packet 52's pattern).
  - Overhang classification for `BridgeInfill`, `SparseInfill`, supports, ironing, skirt.

## Prerequisites and Blockers

- Depends on:
  - Packet 52 (`status: implemented`) — registers `overhang_{1,2,3,4}_4_speed` in `FeedrateConfig` and the speed schema. Without packet 52 the four keys are not parseable.
- Unblocks:
  - Future packets that need quality-modulated feedrate (curled-edge slowdown, bridge-perimeter as distinct role, smoothed-speed mode, `FloatOrPercent` percentage derivation).
- Activation blockers:
  - None outstanding. Open questions (see `design.md`) are limited to OrcaSlicer threshold endpoint reconciliation which is resolved inside Step 3, not before activation.

## Acceptance Criteria

- **AC-1.** **Given** a two-layer step fixture — layer 1 a 20×20 mm square, layer 2 a 20×30 mm rectangle extending +10 mm in `+y` beyond layer 1 — and `overhang_1_4_speed = 10.0` with the other three keys at default `0.0`, **when** `DefaultGCodeEmitter::emit_gcode` runs, **then** the outer-wall print move crossing into the cantilever region emits an F-token of exactly `F600` (= `10 × 60`). | `cargo test -p slicer-host --test overhang_speed_tdd -- cantilever_emits_overhang_speed --nocapture`

- **AC-2.** **Given** all four `overhang_N_4_speed` keys set to `0.0` (the default) on the same scene as AC-1, **when** `emit_gcode` runs, **then** the produced G-code is byte-identical to a pre-feature baseline string captured inside the test (the classifier short-circuits and no `Point3WithWidth.overhang_quartile` is `Some(_)`). | `cargo test -p slicer-host --test overhang_speed_tdd -- zero_config_byte_identical_baseline --nocapture`

- **AC-3.** **Given** `overhang_1_4_speed = 5.0`, `overhang_2_4_speed = 10.0`, `overhang_3_4_speed = 20.0`, `overhang_4_4_speed = 40.0` and an `ExtrusionPath3D` of role `SparseInfill | BridgeInfill | SupportMaterial | SupportInterface | Skirt | TopSolidInfill | BottomSolidInfill | WipeTower | PrimeTower | Ironing` whose vertices all sit fully overhanging the previous layer, **when** `emit_gcode` runs, **then** every F-token emitted on those paths matches the role's base speed (`sparse_infill_speed × 60`, `bridge_speed × 60`, etc.) — `resolve_feedrate` ignores `overhang_quartile` for non-wall roles. The test asserts F-tokens point-by-point against the role-base-speed schedule. | `cargo test -p slicer-host --test overhang_speed_tdd -- non_wall_roles_ignore_overhang_quartile --nocapture`

- **AC-4.** **Given** a single-layer scene (no previous layer) and any non-zero overhang speed configuration, **when** `classify_layers` runs, **then** every `Point3WithWidth.overhang_quartile` in layer 0 is `None` and every emitted F-token on that layer matches the role base speed. | `cargo test -p slicer-host --test overhang_speed_tdd -- first_layer_quartile_is_none --nocapture`

- **AC-5.** **Given** a synthetic fixture with four wall segments engineered to fall one each into the four signed-distance bands — fully supported (`d ≥ 0`), `[−0.25w, 0)`, `[−0.5w, −0.25w)`, `[−∞, −0.5w)` — and `overhang_1_4_speed=10`, `overhang_2_4_speed=20`, `overhang_3_4_speed=30`, `overhang_4_4_speed=40`, **when** `emit_gcode` runs, **then** the four corresponding outer-wall F-tokens are exactly `F2400` (Q4 = 40×60), `F1800` (Q3 = 30×60), `F1200` (Q2 = 20×60), `F600` (Q1 = 10×60) respectively. | `cargo test -p slicer-host --test overhang_speed_tdd -- quartile_to_key_mapping --nocapture`

- **AC-6.** **Given** a `LayerCollectionIR` containing a `Point3WithWidth` with `overhang_quartile = Some(2)`, **when** the IR is serialized via `serde_json::to_string` and deserialized back, **then** the resulting struct has `overhang_quartile == Some(2)`; **and given** a JSON payload from an older producer that omits the `overhang_quartile` field, **when** deserialized, **then** the resulting struct has `overhang_quartile == None` (serde `#[serde(default)]`). Both branches must pass in the same test. | `cargo test -p slicer-ir --test point3_overhang_quartile_roundtrip -- --nocapture`

## Negative Test Cases

- **AC-N1.** **Given** any classifier output, **then** `overhang_quartile` is never `Some(0)` — `Some(0)` is reserved and would indicate a bucketization bug. The classifier carries a `debug_assert!(q >= 1 && q <= 4)` on every assignment, and a test synthesizes the failing path and asserts the debug assertion fires (in `debug_assertions` builds) or that the public classifier output enumeration contains only `None | Some(1..=4)` over a 1000-point random fixture (in release-style assertion-off runs). | `cargo test -p slicer-host --test overhang_speed_tdd -- quartile_zero_is_reserved --nocapture`

## Verification

Supplemental (per-criterion commands above are the authoritative gate; the entries below are packet-level regression guards and the close-time workspace ceremony):

- `cargo build -p slicer-ir -p slicer-core -p slicer-host`
- `cargo test -p slicer-host --test overhang_speed_tdd`
- `cargo test -p slicer-host --test gcode_feedrate_emission_tdd` *(regression: packet 52)*
- `cargo test -p slicer-host --test gcode_emit_tdd` *(regression: emit shape)*
- `cargo test -p slicer-host --test orca_comment_contract_tdd` *(regression: `;TYPE:` labels unchanged)*
- `cargo clippy -p slicer-ir -p slicer-core -p slicer-host -- -D warnings`
- `cargo check --workspace` *(catches WIT-binding drift in modules that consume `point3-with-width`)*
- Acceptance-ceremony only (Step 7, dispatched to a sub-agent with FACT pass/fail return — never absorbed): `cargo test --workspace`

## Authoritative Docs

- `docs/02_ir_schemas.md` — load the `Point3WithWidth` / `LayerCollectionIR` section directly; delegate any > 300-line read.
- `docs/03_wit_and_manifest.md` — load the `point3-with-width` and host-boundary enforcement sections directly; delegate the rest.
- `docs/08_coordinate_system.md` — load in full (small); confirms `Point3WithWidth.x/y/z` are `f32` mm at the emitter layer, so the classifier works in mm.
- `docs/13_slicer_helpers_crate.md` — load in full (small); confirms `Point2` / `Point2::from_mm` ownership.
- `docs/DEVIATION_LOG.md` — read the DEV-009 entry only (delegate); append remediation progress note as part of Step 6.
- `docs/07_implementation_status.md` — TASK-182 row only; never load the file in full into the implementer's context. Dispatch the closure edit.
- `CLAUDE.md` — re-read the *WIT/Type Changes Checklist* section before Step 0.

## OrcaSlicer Reference Obligations

All reads delegated; never load this tree into the implementer's own context.

- `OrcaSlicerDocumented/src/libslic3r/GCode/ExtrusionProcessor.hpp` lines around `:71`, `:147`, `:397`, `:514`, `:535` — quartile bucketization, distance-threshold comparison, speed-band lookup, per-point clamping. Borrow the `<` vs `<=` convention at quartile boundaries verbatim.
- `OrcaSlicerDocumented/src/libslic3r/GCode.cpp` lines around `:4804`, `:5324`, `:6599`, `:6604-6618`, `:6620`, `:6639` — overhang overlap levels (`90, 75, 50, 25, 13, 0%`), quartile→speed config lookup, prev-layer `Layer::lslices` boundary handoff, `estimate_extrusion_quality` invocation paths (standard + curled-edges branch). Borrow only the standard branch; curled-edges deferred.
- Deliberately NOT borrowed: the smoothed-speed/interpolation mode (we ship quantized first), the `ADD_INTERSECTIONS` template branch (mid-segment splitting), and the `prev_curled_extrusions` slowdown.

## Packet Files

- `requirements.md`
- `design.md`
- `implementation-plan.md`
- `task-map.md` (this packet has a single task ID but reopens DEV-009 and spans the WIT + host boundary, so the map is included for the WIT/host audit trail)

## Context Discipline Note

This packet was generated against the context-discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list;
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly;
- delegate every `cargo`, every OrcaSlicer read, and every full-doc fact-check to a sub-agent with one of the contracted return formats (FACT / LOCATIONS / SNIPPETS / SUMMARY);
- stop reading at 60% context and hand off at 85%.

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. No single step is rated L; if a future revision changes that, the packet must be split before activation.
