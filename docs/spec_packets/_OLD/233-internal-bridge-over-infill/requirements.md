# Requirements: internal-bridge-over-infill

## Packet Metadata

- Grouped task IDs: `ISSUE-82` (P75 slot)
- Backlog source: `docs/specs/orca-feature-gap/issues/82-author-packet-p75-quality-bridging-bridge-over-infill.md`
- Packet status: `draft`
- Aggregate context cost: `M`

## Problem Statement

Canonical OrcaSlicer runs `PrintObject::bridge_over_infill` inside `prepare_infill()` *after* `process_external_surfaces`/`clip_fill_surfaces`: it generates sparse-infill anchor polylines itself (`Layer::generate_sparse_infill_polylines_for_anchoring`), clusters anchored lines above voids, picks the angle with `determine_bridging_angle` (length-weighted mean over a ±18° sliding window of nearest-anchor orientations — why real prints get non-grid angles like 23.3°), builds polygons with `construct_anchored_polygon` (scan lines every `bridging_flow.scaled_spacing()`, clipped to anchors/walls), emits `stInternalBridge` surfaces and subtracts them from `stInternal`. PnP has **no internal bridge-over-infill decision at HEAD**: `commit_shell_classification_builtin` (`crates/slicer-runtime/src/slice_postprocess_prepass.rs`) contains zero internal-bridge logic (reviewer-verified tree search). The only bridge-labelled material at HEAD comes from `assemble_bridge_areas` (`crates/slicer-core/src/algos/prepass_slice.rs`) stamping mesh-derived candidates wherever the layer cross-section intersects the facet footprint (F1 false sites), claimed from infill roles by `region_partition.rs` precedence `bridge > bottom > top > sparse`. There is no `InternalBridgeInfill` role — the only `InternalBridge` occurrence in Rust code at HEAD is the dead/reserved feedrate mapping `"InternalBridge" => internal_bridge_speed` in `crates/slicer-gcode/src/emit.rs` (nothing emits a `Custom("InternalBridge")` role at HEAD; only the stash does). PnP couples every role's feedrate to `infill_speed / BASE_SPEED(50)` (F6), ignores configured bridge width and lacks `BRIDGE_EXTRA_SPACING` in core `bridging_flow` (F5), and rotates sparse rectilinear +90° on odd layers where canonical keeps `infill_angle` constant (F7, bundled per D11). These are one coherent slice: all are the internal-bridge decision (introduced here), its role identity, its flow/spacing, and its speed/direction behavior in the module.

## In Scope

- Introduce the internal bridge-over-infill decision at the post-surface/infill seam: stage tag `"Layer::InfillPostProcess"`, dispatch arm `LayerStageCommit::InfillPostProcess(ir)` in `crates/slicer-runtime/src/layer_executor.rs` — where canonical runs `PrintObject::bridge_over_infill` inside `prepare_infill()`. This is a NEW decision, not a move of existing prepass logic (at HEAD `commit_shell_classification_builtin` contains zero internal-bridge logic).
- Port `determine_bridging_angle` (windowed length-weighted mean, ±18°) and `construct_anchored_polygon` (scan lines every `bridging_flow.scaled_spacing()`, clipped to anchors/walls) as pure geometry in `slicer-core`, honoring `internal_bridge_angle` override (> 0 forces the angle).
- Generate or reuse sparse-infill anchor polylines at the seam (canonical: `Layer::generate_sparse_infill_polylines_for_anchoring`); cluster anchored lines above voids; honor `dont_filter_internal_bridges` and `enable_extra_bridge_layer` semantics.
- Emit `InternalBridgeInfill` role surfaces and subtract them from sparse infill (`stInternal` equivalent); region precedence `bridge > bottom > top > sparse` in `crates/slicer-runtime/src/region_partition.rs` already matches canonical — do not change the order.
- D7/F8: add `ExtrusionRole::InternalBridgeInfill` threaded through IR (`crates/slicer-ir/src/slice_ir.rs`), WIT `extrusion-role` (`crates/slicer-schema/wit/`), host, marshal, and gcode; retire the dead/reserved `"InternalBridge"` string mapping in `crates/slicer-gcode/src/emit.rs` (at HEAD), the stash's `Custom("InternalBridge")` emission, and the stash's `is_internal_bridge` flag threading. Keep the gcode label `Internal Bridge` and the `internal_bridge_speed` feedrate (field default 37.5 in `crates/slicer-ir/src/feedrate.rs`).
- F5: canonicalize `bridging_flow` (`crates/slicer-core/src/flow.rs`): `thread_diameter = bridge_line_width if set else nozzle_diameter`; `bridge_extrusion_spacing(dmr) = dmr + 0.05 mm` (`BRIDGE_EXTRA_SPACING`); retire the stash's module-level +0.05 mm shim. NOTE: `resolve_role_width` already consumes `RoleWidthContext.bridge_line_width` separately — F5 is spacing derivation, not role width; do not touch `resolve_role_width`.
- F6 (invariant I7): bridge feedrate = resolved bridge speed regardless of infill speed; plus the Q1-decided solid-role decoupling (see `design.md`).
- D11/F7: remove the odd-layer +90° sparse alternation (`layer_index.is_multiple_of(2)` in `run_infill`); rectilinear keeps `infill_angle` constant (canonical `FillRectilinear::_layer_angle` ≡ 0).
- Salvage from `stash@{0}` (popped in Step 1, D10): flag threading shape, module routing + TOML schema, gcode label mapping. Discard: orientation heuristic, contour-band expansion approximation (`INTERNAL_BRIDGE_EXPANSION_MULTIPLIER`).
- Config keys snake_case: `dont_filter_internal_bridges`, `enable_extra_bridge_layer`, `internal_bridge_angle`.

## Out of Scope

- W-A false-site gating / unsupported-span classification test (packet 234): `assemble_bridge_areas` (`crates/slicer-core/src/algos/prepass_slice.rs`) is context only, never edited here.
- W-B external orientation port: `compute_bridge_direction_deg` (`crates/slicer-core/src/algos/mesh_analysis.rs`) and `detect_bridging_direction` are packet 235's surface.
- Fan handling for internal bridges (canonical `_INTERNAL_BRIDGE` fan markers, `enable_overhang_bridge_fan`) — deferred per Q2 decision in `design.md`.
- `infill_rotate_template` / `solid_infill_rotate_template` (`calculate_infill_rotation_angle`) — canonical features our module does not read; not added here.
- Invariants I1 (no bridge over support), I2 (site existence), I3 (external orientation) — owned by packets 234/235.
- The legacy `BridgeDetector::detect_angle` 5°-sweep class — dead code upstream; never port.

## Authoritative Docs

- `docs/specs/bridge-parity-plan.md` (269 lines) - read directly in full; the controlling plan (D1–D12, F3/F5–F8, W-C, I4–I7).
- `docs/adr/0061-deterministic-bridge-orientation-tie-break.md` - read directly; equal-cost candidates resolve to smallest quantized angle. Reference only.
- `docs/08_coordinate_system.md` - read the porting checklist section directly before converting canonical scaled constants.
- `docs/21_data_defaults_and_fixtures.md` - delegate a SUMMARY of the struct-literal `..` rest / `// exhaustive:` waiver rule.
- `docs/03_wit_and_manifest.md` - delegate a SUMMARY of the `extrusion-role` contract section.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/PrintObject.cpp` — `PrintObject::bridge_over_infill` orchestration; `determine_bridging_angle` and `construct_anchored_polygon` lambdas; the three owned keys' consumption.
- `OrcaSlicerDocumented/src/libslic3r/Fill/Fill.cpp` — `Layer::generate_sparse_infill_polylines_for_anchoring`; per-role `role_speed` assignment (Q1 evidence); `calculate_infill_rotation_angle` (not ported).
- `OrcaSlicerDocumented/src/libslic3r/Fill/FillBase.cpp` + `FillRectilinear.hpp` — `Fill::_infill_direction` gating; `FillRectilinear::_layer_angle` ≡ 0.
- `OrcaSlicerDocumented/src/libslic3r/Flow.hpp` / `Flow.cpp` — `Flow::bridging_flow`, `bridge_extrusion_spacing`, `BRIDGE_EXTRA_SPACING`.
- `OrcaSlicerDocumented/src/libslic3r/LayerRegion.cpp` — `LayerRegion::bridging_flow` thread-diameter selection; `process_external_surfaces` expansion constants.
- `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` — defaults/enum values for `dont_filter_internal_bridges` (`ibfDisabled`/`ibfNofilter`/…), `enable_extra_bridge_layer` (`eblApplyToAll`/`eblExternalBridgeOnly`/`eblInternalBridgeOnly`), `internal_bridge_angle`.
- `OrcaSlicerDocumented/src/libslic3r/ExtrusionEntity.cpp` — `erInternalBridgeInfill` label text; fan markers deliberately not borrowed.

## Acceptance Summary

Reference, never copy, criteria from `packet.spec.md`.

- Positive: `AC-1` (variant in IR+WIT), `AC-2` (feedrate mapping + label, I7 at unit level), `AC-3` (I4 self-consistent angle + override), `AC-4` (I5 density), `AC-5` (I6 disjointness), `AC-6` (I7 end-to-end reslice), `AC-7` (D11 no alternation), `AC-8` (F5 spacing), `AC-9` (snake_case keys).
- Negative: `AC-N1` (Custom tag / `is_internal_bridge` retired), `AC-N2` (prepass no longer decides internal bridges), `AC-N3` (module spacing shim retired), `AC-N4` (struct-literal churn gate).
- Nominated-model clause (AC-6): **`resources/bridge.obj` is the nominated model.** Why: it is the plan's first-listed candidate (D1) and its geometry — a bridge spanning a gap over a body — is the canonical internal-bridge-over-sparse shape, whereas `overhang.obj` is an external overhang (packet 234's W-A domain) and `ipadstand.obj`'s slot is a through-hole (external bridge). Step 6 still verifies the nomination at activation by slicing `bridge.obj` and confirming an internal-bridge-over-sparse site; if it yields none, the implementer substitutes `overhang.obj` then `ipadstand.obj` (concrete fallbacks, no placeholder) and records the substitution + Z here. The calicat measurements in plan §2 are steering evidence only (LLM-visual-oracle rule) — never adjudication.
- Cross-packet impact: adds `InternalBridgeInfill` to `ExtrusionRole` (exhaustive-match blast radius workspace-wide — see `design.md`); packet 234 consumes the anchored-polygon/expansion primitives; packet 235 reuses the D6 degrees-mod-180 boundary conversion established here.

## Verification Commands

This is the authoritative full matrix; `packet.spec.md` lists only the gate commands.

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo check --workspace --all-targets` | compile incl. test targets | FACT pass/fail |
| `cargo clippy --workspace --all-targets -- -D warnings` | lint gate | FACT pass/fail; SNIPPETS ≤20 lines |
| `cargo xtask check-literals` | struct-literal churn gate (AC-N4) | FACT pass/fail |
| `cargo xtask build-guests --check` | guest freshness after WIT/module edits; exit 0 fresh / 1 stale / 3 infra | FACT exit code |
| `cargo test -p slicer-core --test bridge_over_infill_tdd -- bridging` | AC-3/AC-4/AC-8 (angle, density, flow) | FACT pass/fail |
| `cargo test -p slicer-gcode --test gcode_feedrate_emission_tdd -- internal_bridge` | AC-2 | FACT pass/fail |
| `cargo test -p slicer-runtime --test integration -- internal_bridge_disjoint` | AC-5 (I6) | FACT pass/fail |
| `cargo test -p rectilinear-infill --test rectilinear_infill_tdd -- alternation` | AC-7 (D11) | FACT pass/fail |
| `cargo xtask test --summary -p slicer-runtime --test e2e` | guest-consuming end-to-end regression net after the seam introduction | FACT pass/fail |
| AC-6 reslice pair (see packet.spec.md) with `--module-dir modules/core-modules` | I7 end-to-end | FACT pass/fail |

Gotchas carried from plan §6: `--module-dir modules/core-modules` is mandatory on every reslice; compare G-code layers by Z, never layer index; `;TYPE:` appears only on role change (carry it across layers); both outputs are M83 relative-E (positive E delta on an XY move = extrusion); leading-dot floats are legal (`E.25723`); `cargo fmt --all` is broken on this machine — format touched files individually.

## Step Completion Expectations

- Step 1 (stash pop) MUST complete before any other step edits code; guest freshness re-check (`cargo xtask build-guests --check`) is part of Step 1's exit, and the stash pop re-stales guests — rebuild before any guest-touching test.
- Step 2 (enum variant) owns the schema-version bump decision: if `ExtrusionRole` is serialized into the committed IR, bump the IR schema/version constant in the SAME step and fix every test hard-asserting the old value there.
- Steps 3–5 are order-independent relative to each other but all depend on Step 2's variant existing (module emits the variant, flow feeds its spacing).
- The new pure-geometry module (`bridge_over_infill` algos) is shared scratch state between Steps 4 and 6: invariants I4/I5 are asserted against it directly.

## Context Discipline Notes

- `OrcaSlicerDocumented/` is out of bounds for direct reads — delegate per the snippet; the grounded facts in `design.md` were verified at authoring time and should be trusted before re-dispatching.
- Never load `stash@{0}` diff in full (~1338 added lines across 19 files); triage it file-by-file per the salvage map.
- `crates/slicer-runtime/src/layer_executor.rs` and `slice_postprocess_prepass.rs` are long; ranged reads around `LayerStageCommit::InfillPostProcess` and `commit_shell_classification_builtin` only.
- Cargo runs are delegated with FACT pass/fail returns; test output always tees to `target/test-output.log`.
