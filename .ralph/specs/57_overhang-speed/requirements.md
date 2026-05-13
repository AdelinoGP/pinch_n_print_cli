# Requirements: 57_overhang-speed

## Packet Metadata

- Grouped task IDs:
  - `TASK-182`
- Backlog source: `docs/07_implementation_status.md`
- Packet status: `draft`
- Aggregate context cost: `M`
- Remediation reference: `docs/DEVIATION_LOG.md` DEV-009 (Phase H quality-modulated feedrate gap).

## Problem Statement

Packet 52 registered all four `overhang_N_4_speed` config keys in `FeedrateConfig` and in the wider speed schema, but those keys are dead code today:

1. `resolve_feedrate` in `crates/slicer-host/src/gcode_emit.rs:154` has no dispatch arm for them; it only chooses among the per-role base speeds.
2. No upstream stage produces quartile data on `Point3WithWidth`; the IR carries no `overhang_quartile` field.
3. The `point3-with-width` WIT record at `wit/deps/types.wit:7-11` carries no overhang field either, so even if a host stage produced one, it would not survive a WIT roundtrip into modules that consume layer IR.

OrcaSlicer slows down wall extrusions when they print over insufficiently-supported previous-layer geometry, using the `overhang_{1,2,3,4}_4_speed` schedule (1/4 = least supported / slowest, 4/4 = most supported / fastest). The four keys are wired in OrcaSlicer through `ExtrusionProcessor.hpp::estimate_points_properties` and `GCode.cpp::estimate_extrusion_quality`, which bucketize each vertex by signed distance to the previous-layer support polygons.

This packet closes the gap end-to-end: WIT field + Rust mirror, classifier prepass, `resolve_feedrate` dispatch, pipeline wire-in, regression tests, and remediation notes.

This packet does **not** reopen or supersede packet 52; it consumes packet 52's registered keys without modifying them.

## In Scope

- WIT record extension: add `overhang-quartile: option<u8>` to `point3-with-width` at `wit/deps/types.wit:7-11`.
- Host-side mirror: add `#[serde(default)] pub overhang_quartile: Option<u8>` to `Point3WithWidth` at `crates/slicer-ir/src/slice_ir.rs:1218`; bump the schema minor-version constant.
- Binding fan-out: update every `wit_host.rs`, `dispatch.rs`, and `wit_guest` site that converts between the WIT record and the Rust struct so that the new field survives the boundary.
- New crate-local module `crates/slicer-core/src/aabb_lines_2d.rs` providing `LinesDistancer2D` with `new`, `signed_distance`, `nearest_distance`.
- New crate-local module `crates/slicer-host/src/overhang_classifier.rs` providing `pub fn classify_layers(layers: &mut [LayerCollectionIR], feedrate_config: &FeedrateConfig)`.
- `resolve_feedrate` signature change to accept `overhang_quartile: Option<u8>`; dispatch for `OuterWall | InnerWall | ThinWall` to the four overhang speed keys (× 60 × clamped `speed_factor`).
- Per-point emission site update at `crates/slicer-host/src/gcode_emit.rs:388` to pass `point.overhang_quartile`.
- Z-hop site and any other `resolve_feedrate` callers pass `None`.
- Pipeline wire-in (both slicer-cli and WASM arms) in `crates/slicer-host/src/pipeline.rs`: insert `overhang_classifier::classify_layers(&mut layer_irs, &feedrate_config)` between layer finalization and `emit_gcode`.
- New TDD test file `crates/slicer-host/tests/overhang_speed_tdd.rs` covering AC-1 … AC-5 and the negative AC-N1.
- New IR roundtrip test for AC-6 in `crates/slicer-ir/tests/point3_overhang_quartile_roundtrip.rs` (or wherever the existing slice-IR roundtrip tests live — confirm in Step 1 via LOCATIONS dispatch).
- Append remediation note to `docs/DEVIATION_LOG.md` for DEV-009.
- Close `TASK-182` row in `docs/07_implementation_status.md` at packet-completion gate.

## Out of Scope

- Mid-segment intersection insertion at distance-threshold crossings (OrcaSlicer `ADD_INTERSECTIONS`).
- Bridge-perimeter as a distinct `ExtrusionRole` variant.
- Curled-edge slowdown (`prev_curled_extrusions`).
- Production BVH inside `LinesDistancer2D` (linear scan + bbox prefilter is sufficient for packet 57; revisit if profiling shows it dominates).
- Smoothed/interpolated speed mode (Orca offers both; ship quantized only).
- `ConfigValue::FloatOrPercent` for the four keys (defer with packet 52's pattern).
- Overhang classification for `BridgeInfill | SparseInfill | SupportMaterial | SupportInterface | Skirt | TopSolidInfill | BottomSolidInfill | WipeTower | PrimeTower | Ironing`.
- Adding a new `ExtrusionRole::Overhang` variant — explicitly rejected; the role stays `OuterWall|InnerWall|ThinWall` and the per-point `overhang_quartile` carries the band info.

## Authoritative Docs

- `docs/02_ir_schemas.md` — load the `Point3WithWidth` / `LayerCollectionIR` section directly. Size: typical (~ 300+ lines); delegate a SUMMARY for the schema-version policy if the section grows.
- `docs/03_wit_and_manifest.md` — load the `point3-with-width` and host-boundary enforcement sections directly; delegate the rest as SUMMARY. Size: > 300 lines.
- `docs/08_coordinate_system.md` — load in full. Small file. Confirms mm at the emitter layer.
- `docs/13_slicer_helpers_crate.md` — load in full. Small file. Confirms `Point2::from_mm` ownership.
- `docs/DEVIATION_LOG.md` DEV-009 entry — delegate the read; append remediation progress in Step 6.
- `docs/07_implementation_status.md` — never load in full into the implementer's context. Edit the TASK-182 row via narrow `Edit` after dispatching a LOCATIONS lookup.
- `CLAUDE.md` — re-read the *WIT/Type Changes Checklist* section before Step 0.

Default rule (per skill preamble): delegate any doc > 300 lines.

## OrcaSlicer Reference Obligations

All reads MUST be delegated to a sub-agent with SNIPPETS return; never load this tree into the implementer's own context.

- `OrcaSlicerDocumented/src/libslic3r/GCode/ExtrusionProcessor.hpp:71` — `estimate_points_properties` template signature with the `ADD_INTERSECTIONS` flag.
- `OrcaSlicerDocumented/src/libslic3r/GCode/ExtrusionProcessor.hpp:147` — `ADD_INTERSECTIONS` boundary-crossing detection (we ship the no-intersection branch only).
- `OrcaSlicerDocumented/src/libslic3r/GCode/ExtrusionProcessor.hpp:397` — speed-section count and band configuration setup; borrow the band count and the `< / <=` convention.
- `OrcaSlicerDocumented/src/libslic3r/GCode/ExtrusionProcessor.hpp:514` — `calculate_speed` lambda; the piecewise-linear interpolation is deliberately NOT borrowed (quantized only in this packet).
- `OrcaSlicerDocumented/src/libslic3r/GCode/ExtrusionProcessor.hpp:535` — per-point speed clamping; mirror the clamp range.
- `OrcaSlicerDocumented/src/libslic3r/GCode.cpp:6599` — overhang overlap levels `90, 75, 50, 25, 13, 0%`. Our packet uses the four-band quartile schedule; record the deviation if implementing six bands is later required.
- `OrcaSlicerDocumented/src/libslic3r/GCode.cpp:6604-6618` — quartile→`overhang_N_4_speed` config lookup; mirror the key names.
- `OrcaSlicerDocumented/src/libslic3r/GCode.cpp:6620` — `estimate_extrusion_quality` invocation with curled-edges branch (NOT borrowed).
- `OrcaSlicerDocumented/src/libslic3r/GCode.cpp:6639` — standard `estimate_extrusion_quality` invocation (borrowed).
- `OrcaSlicerDocumented/src/libslic3r/GCode.cpp:4804` — `prepare_for_new_layer` supplies `Layer::lslices` to the estimator; mirror by passing the previous layer's wall-loop polygons into the classifier.
- `OrcaSlicerDocumented/src/libslic3r/GCode.cpp:5324` — `set_current_object` establishes object context; our equivalent is `LayerCollectionIR` iteration scope.

## Acceptance Summary

- Positive cases (copied from `packet.spec.md`):
  - AC-1 cantilever F600 emission.
  - AC-2 zero-config byte-identical baseline.
  - AC-3 non-wall roles ignore `overhang_quartile`.
  - AC-4 first-layer `overhang_quartile == None`.
  - AC-5 quartile-to-key mapping (four distinct F-values per band).
  - AC-6 IR roundtrip with `#[serde(default)]`.

- Negative case (copied):
  - AC-N1 `overhang_quartile` is never `Some(0)` in classifier output; debug-assert guards the invariant.

- Measurable outcomes:
  - `cargo test -p slicer-host --test overhang_speed_tdd` reports 6 passing tests (or 7 with the negative case test split off).
  - `cargo test -p slicer-ir --test point3_overhang_quartile_roundtrip` passes.
  - `cargo test -p slicer-host --test gcode_feedrate_emission_tdd`, `gcode_emit_tdd`, and `orca_comment_contract_tdd` remain green (regression).
  - `cargo clippy -p slicer-ir -p slicer-core -p slicer-host -- -D warnings` reports zero diagnostics.
  - `cargo check --workspace` succeeds, proving the WIT field addition did not break any module consumer of `point3-with-width`.

- Cross-packet impact:
  - Unblocks future quality-modulated feedrate packets (curled edges, bridge-perimeter role split, smoothed-speed mode, `FloatOrPercent` for the overhang keys).
  - Does not retroactively change any packet 52 acceptance result.

## Verification Commands

All commands must be dispatched to a sub-agent with FACT pass/fail return; never absorb full output.

- `cargo build -p slicer-ir -p slicer-core -p slicer-host`
- `cargo test -p slicer-host --test overhang_speed_tdd`
- `cargo test -p slicer-ir --test point3_overhang_quartile_roundtrip`
- `cargo test -p slicer-host --test gcode_feedrate_emission_tdd`
- `cargo test -p slicer-host --test gcode_emit_tdd`
- `cargo test -p slicer-host --test orca_comment_contract_tdd`
- `cargo clippy -p slicer-ir -p slicer-core -p slicer-host -- -D warnings`
- `cargo check --workspace`
- Close-time ceremony only: `cargo test --workspace` (dispatch with FACT return; ≥ 11 min; not run mid-iteration per `CLAUDE.md`'s test discipline).

## Step Completion Expectations

See `implementation-plan.md` for the per-step fields (precondition, postcondition, files-to-read, files-to-edit, dispatches, cost, exit condition). The roll-up is also mirrored in `task-map.md`.

## Context Discipline Notes

- `docs/03_wit_and_manifest.md` is > 300 lines: delegate as SUMMARY, never load in full.
- `docs/02_ir_schemas.md` may exceed 300 lines: delegate as SUMMARY; load only the `Point3WithWidth` section directly.
- `docs/07_implementation_status.md` is the largest backlog doc: never load in full; dispatch the TASK-182 closure edit.
- All `OrcaSlicerDocumented/` files MUST be delegated; never load directly. The reference list above is the agreed dispatch target.
- `crates/slicer-host/src/gcode_emit.rs` is large (> 400 lines); range-read around the named line numbers (`:154`, `:254`, `:362-393`, `:388`, `:443`) only.
- `crates/slicer-ir/src/slice_ir.rs` is large (> 1200 lines); range-read around `:1218` (`Point3WithWidth`), `:1233` (`ExtrusionRole`), `:1291` (`ExtrusionPath3D`) only.
- `wit/deps/types.wit` is small; load in full.
- Sub-agent return-format hints for the heaviest dispatches:
  - WIT-binding fan-out (Step 0): `LOCATIONS: ≤ 20 entries` listing every conversion site between Rust `Point3WithWidth` and the WIT record. Don't return any code.
  - OrcaSlicer parity check (Step 3): `SNIPPETS: ≤ 3 × 30 lines` around the cited line numbers; no surrounding context.
  - `cargo check --workspace` (Step 0 exit + Step 5): `FACT: pass | fail (<assertion>)` with at most a 20-line SNIPPET on failure.
