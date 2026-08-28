---
status: implemented
packet: 234-bridge-false-site-gating
task_ids: []
backlog_source: docs/specs/bridge-parity-plan.md §4 W-A row (new packet, no prior owner)
context_cost_estimate: M
---

# Packet Contract: 234-bridge-false-site-gating

## Goal

Gate bridge classification on the canonical unsupported-span test — bridge area minus the UNGROWN lower-layer contours (canonical `voids = diff(voids, *lower_layer_covered)`) — so that mesh-derived bridge candidates become `region.bridge_areas` only where the span is genuinely unsupported, demoting the existing mesh-validity filter (`BridgeRegion.is_valid`) to at most a cheap pre-filter.

## Scope Boundaries

This packet changes how `assemble_bridge_areas` (`crates/slicer-core/src/algos/prepass_slice.rs`) decides which mesh-derived bridge candidates become `region.bridge_areas`, and where that decision runs. It introduces the canonical unsupported-span test (bridge area minus lower-layer anchor areas) and applies it before the region-partition precedence `bridge > bottom > top > sparse` (`crates/slicer-runtime/src/region_partition.rs`) claims those areas from infill roles. It does not touch external-bridge orientation (235), internal-bridge construction (233), or the sparse ±90° alternation (233). The mesh-validity filter survives only as a cheap pre-filter, with a recorded measurement plan to justify or discard it.

## Prerequisites and Blockers

- Depends on: `docs/spec_packets/233-internal-bridge-over-infill/` (status implemented). Its net-new symbols — `ExtrusionRole::InternalBridgeInfill` (`crates/slicer-ir/src/slice_ir.rs`), WIT `extrusion-role` variant `internal-bridge-infill`, module `bridge_over_infill` + `determine_bridging_angle`/`construct_anchored_polygon` (`crates/slicer-core/src/algos/bridge_over_infill.rs`), `bridge_extrusion_spacing`/`BRIDGE_EXTRA_SPACING_MM` (`crates/slicer-core/src/flow.rs`), config keys `dont_filter_internal_bridges`/`enable_extra_bridge_layer`/`internal_bridge_angle` — exist at HEAD (233 landed 2026-08-22) and are prerequisites, not this packet's surface.
- Unblocks: `docs/spec_packets/235-external-bridge-orientation/` (external orientation port).
- Activation blockers: none.

## Acceptance Criteria

State ACs only here; `requirements.md` references their IDs.

- **AC-1. Given** a `SlicedRegion` whose mesh-derived bridge candidate footprint is fully covered by the lower layer's slices (solid underneath the span), **when** `gate_bridge_areas_by_unsupported_span` runs, **then** `region.bridge_areas` is empty (zero bridge-role area survives). | `cargo test -p slicer-core --features host-algos --test bridge_false_site_gating_tdd -- solid_underneath_span_produces_no_bridge_area --nocapture`
- **AC-2. Given** a `SlicedRegion` whose bridge candidate spans a genuine unsupported gap (lower layer has no slices beneath the span), **when** `gate_bridge_areas_by_unsupported_span` runs, **then** `region.bridge_areas` retains the unsupported portion (non-empty, equal to bridge area minus lower-layer anchor areas). | `cargo test -p slicer-core --features host-algos --test bridge_false_site_gating_tdd -- unsupported_span_retains_bridge_area --nocapture`
- **AC-3. Given** `resources/bridge.obj` sliced with the core modules, **when** the emitted G-code is parsed with M83 relative-E semantics (positive E delta on an XY move = extrusion), keyed by Z (never layer index), with `;TYPE:` carried across layer changes, **then** at least one layer carries Bridge-type extrusion (site exists — the parser matches any `;TYPE:` containing "ridge", i.e. Bridge or Internal Bridge; bridge.obj's site is 233's internal-bridge-over-sparse shape, recorded in `design.md`) AND the number of Bridge-type layers is strictly less than the total layer count (no flooding — false sites suppressed). The parser is `resources/check_bridge_sites.py` — the original inline `python3 -c` script committed verbatim (the inline form’s embedded newline escapes were not runnable as written). | `cargo run --bin pnp_cli --release -- slice --model resources/bridge.obj --output target/bridge_false_site.gcode --module-dir modules/core-modules && python3 resources/check_bridge_sites.py target/bridge_false_site.gcode`
- **AC-4. Given** the region partition with gated bridge areas, **when** roles are computed by precedence `bridge > bottom > top > sparse`, **then** the four role polygons are pairwise disjoint (I6 regression guard). | `cargo test -p slicer-runtime --test integration -- region_partition_tdd::ac2_precedence_pairwise_disjoint_under_partial_overlap --nocapture`
- **AC-5. Given** `resources/overhang.obj` (solid base at Z≈17, overhanging top at Z≈23.3) sliced with the core modules, **when** the emitted G-code is parsed with M83 relative-E semantics (positive E delta on an XY move = extrusion), keyed by Z (never layer index), with `;TYPE:` carried across layer changes, **then** the lowest layer carries ZERO Bridge-type extrusion (first layer, no lower layer → demoted to bottom), the second-lowest layer carries ZERO Bridge-type extrusion (its lower layer is the solid base — the I1 solid-underneath case), at least one layer carries Bridge-type extrusion (the parser matches any `;TYPE:` containing "ridge"; measured 2026-08-23: the overhang lip is a FREE-EDGE bottom — it emits `;TYPE:Bottom surface` at z=3.2, canonical-correct per the packet-109 enclosure discriminator — and the site-existence assertion is satisfied by 233's `;TYPE:Internal Bridge` markers; this packet's external-bridge evidence is the wedge slot ceiling, e2e-verified at print_z 28.2), and Bridge-type layers are a strict subset of all layers (no flooding). The parser is `resources/check_bridge_sites.py`, invoked with `--first-layers-zero 2` to assert the two lowest layers carry no Bridge-type extrusion. | `cargo run --bin pnp_cli --release -- slice --model resources/overhang.obj --output target/overhang_false_site.gcode --module-dir modules/core-modules && python3 resources/check_bridge_sites.py --first-layers-zero 2 target/overhang_false_site.gcode`

## Negative Test Cases

- **AC-N1. Given** a bridge candidate whose span is fully supported by the lower layer (solid underneath), **when** the gating runs, **then** ZERO bridge-role extrusion is emitted for that span — the candidate is rejected, not merely shrunk to a sliver. | `cargo test -p slicer-core --features host-algos --test bridge_false_site_gating_tdd -- fully_supported_candidate_rejected_zero_bridge_area --nocapture`
- **AC-N2. Given** a solid-underneath span, **when** the ungated `assemble_bridge_areas` output and the gated output are compared, **then** they must differ (ungated non-empty, gated empty) — ungated candidates cannot silently return. The gate is the only `bridge_areas` mutation between stamping (`PrePass::Slice`) and partition consumption (`PrePass::ShellClassification` runs before `region_partition`), so a bypass would feed ungated candidates to partition. | `cargo test -p slicer-core --features host-algos --test bridge_false_site_gating_tdd -- ungated_candidates_cannot_silently_return --nocapture`

## Verification

- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo xtask check-literals`
- `cargo test -p slicer-core --features host-algos --test bridge_false_site_gating_tdd`

## Authoritative Docs

- `docs/04_host_scheduler.md` - 1673 lines; delegated SUMMARY of §"Fixed Stage Order" and §"PrePass Execution" (the `OverhangAnnotation`/`ShellClassification`/`prev_layer_boundaries` facts that resolve Q3).
- `docs/08_coordinate_system.md` - direct read (coordinate/expansion-constant conversion).
- `docs/specs/bridge-parity-plan.md` - direct read (this packet's source plan; §2 baseline, §3/F1, §4 W-A, §6 invariants, §7.3 Q3).

## Doc Impact Statement (Required)

- **`docs/02_ir_schemas.md`** — §"Post-`Layer::Perimeters` invariant: four canonical fill polygons" (~lines 596–619): the invariant now states that `bridge_areas` is claimed directly from the gated `SlicedRegion.bridge_areas` and may extend beyond the wall-inset polygon, while `bottom_solid_fill`/`top_solid_fill`/`sparse_infill_area` remain pairwise disjoint subsets of `infill_areas` (all four remain pairwise disjoint via precedence dedup). Verification greps (each must hit):
  - `rg -n "four canonical fill polygons" docs/02_ir_schemas.md`
  - `rg -n "claimed directly from the gated" docs/02_ir_schemas.md`
- **Everything else: `none`** — the packet reuses the existing `bridge_areas`/`SlicedRegion` fields, adds no IR/WIT/scheduler/claim/manifest/host-service/SDK contract, and introduces no new stage (the gating runs inside the existing `PrePass::ShellClassification`). No schema version bump, no `STAGE_ORDER` change.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/LayerRegion.cpp` — `LayerRegion::process_external_surfaces` (both overloads): the unsupported-area analysis flow, the `stBottomBridge` surface-type assignment, the `voids = diff(voids, *lower_layer_covered)` removal of lower-layer-supported voids, and the `detect_bridging_direction(to_polygons(initial), to_polygons(lower_layer->lslices))` call that is the canonical "bridge area minus anchor areas" test.
- `OrcaSlicerDocumented/src/libslic3r/BridgeDetector.hpp` — the `detect_bridging_direction(const Polygons &to_cover, const Polygons &anchors_area)` overload and its floating-edge computation (polyline difference of the bridge boundary against `expand(anchors, SCALED_EPSILON)`), the precise port target for the unsupported-span test.
- `OrcaSlicerDocumented/src/libslic3r/BridgeDetector.cpp` — `BridgeDetector::unsupported_edges` (`diff_pl(to_polylines(bridge_expolygon), grown_lower)`), the concrete "bridge area minus lower-layer anchors" geometry (its `grown_lower` is a single offset by extrusion spacing for direction detection; this packet's span test subtracts UNGROWN contours per the dead-overload `voids = diff(voids, *lower_layer_covered)`).

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- obey the shared absolute context bands: 120k reading budget with hand-off at 150k (standard); the extended band (240k reading / 300k hard stop) only via swarm's escalation protocol

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation (an extended-band run may carry a single L step only when `design.md` justifies why it cannot be split).
