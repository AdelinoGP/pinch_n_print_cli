---
status: draft
packet: 235-external-bridge-orientation
task_ids: []
backlog_source: docs/specs/orca-feature-gap/issues/84-author-packet-p77-quality-bridging-classic-perimeters.md (bridge_angle half; see task-map.md for the explicit split)
context_cost_estimate: M
---

# Packet Contract: 235-external-bridge-orientation

## Goal

Replace the longest-anchor-run external bridge orientation heuristic with a faithful port of canonical's active inline `detect_bridging_direction` semantics — floating-edge candidates over the gated bridge geometry, principal-component fallback, `SCALED_EPSILON` anchor expand, the ADR-0061 deterministic tie-break, and degrees-mod-180 boundary representation — so external bridge lines satisfy invariant I3.

## Scope Boundaries

This packet replaces `compute_bridge_direction_deg` (`crates/slicer-core/src/algos/mesh_analysis.rs`, private, takes `&[AnchorRun]`) with a pure port of the active inline `detect_bridging_direction` overloads declared in canonical `BridgeDetector.hpp`, computed where the gated geometry lives (post-234 gate seam). It consumes the gated `bridge_areas` produced by 234's `gate_bridge_areas_by_unsupported_span` and the raw previous-layer contours, and writes degrees mod 180 into the existing `bridge_orientation_deg` surface. It does not implement the user-facing `bridge_angle` override config key, does not touch `counterbore_hole_bridging`, does not port the legacy `BridgeDetector::detect_angle` sweep class, and does not change IR/WIT schemas.

## Prerequisites and Blockers

- Depends on: `docs/spec_packets/233-internal-bridge-over-infill/` (status draft) and `docs/spec_packets/234-bridge-false-site-gating/` (status draft). Their net-new symbols — 233's `ExtrusionRole::InternalBridgeInfill`, `bridge_over_infill.rs` module, `bridge_extrusion_spacing`/`BRIDGE_EXTRA_SPACING`; 234's `gate_bridge_areas_by_unsupported_span` + optional `grow_anchor_areas` (`crates/slicer-core/src/algos/prepass_slice.rs`) and `crates/slicer-core/tests/bridge_false_site_gating_tdd.rs` — are PLANNED exports referenced as prerequisites, never claimed to exist at HEAD.
- Unblocks: the remaining `bridge_angle`-key scope parked under ISSUE-84 (override-key plumbing) and future coverage/anchoring parity work that assumes canonical external orientation.
- Activation blockers: none.

## Acceptance Criteria

State ACs only here; `requirements.md` references their IDs.

- **AC-1. Given** a two-sided rectangular overhang (`to_cover` 40×10 mm rect, anchors overlapping the left and right ends by 2 mm) so that only the top and bottom edges float with zero accumulated cost, **when** `detect_bridging_direction_deg` runs, **then** the returned orientation is exactly 0.0 degrees mod 180 (span direction; the zero-cost tie between the two face normals is congruent mod 180 regardless of tie-break). | `cargo test -p slicer-core --features host-algos --test bridge_orientation_tdd -- two_sided_rect_gap_orients_along_span --nocapture`
- **AC-2. Given** anchor areas flush with the `to_cover` boundary versus anchors recessed 0.5 mm behind it, **when** the floating-edge helper runs, **then** the flush case yields 3 floating edges (boundary edge absorbed by `expand(anchors, SCALED_EPSILON)` = 1 unit = 10⁻⁴ mm) and the recessed case yields 4 (0.5 mm ≫ ε, edge survives). | `cargo test -p slicer-core --features host-algos --test bridge_orientation_tdd -- flush_anchor_edges_absorbed_within_scaled_epsilon recessed_anchor_keeps_floating_edge_candidates --nocapture`
- **AC-3. Given** a fully anchored overhang (four anchor blocks overlapping every side of a 16×2 mm center island so the floating-edge set is empty), **when** `detect_bridging_direction_deg` runs, **then** it returns exactly 90.0 degrees — the minor principal axis of the island area (shortest-bridge rule). | `cargo test -p slicer-core --features host-algos --test bridge_orientation_tdd -- fully_anchored_island_picks_minor_principal_axis --nocapture`
- **AC-4. Given** a degenerate overhang whose difference against anchors is empty (zero area, no principal components), **when** `detect_bridging_direction_deg` runs, **then** it returns exactly 0.0 degrees — canonical's `{1,0}` fallback — never a NaN or a panic. | `cargo test -p slicer-core --features host-algos --test bridge_orientation_tdd -- degenerate_overhang_falls_back_to_x_axis --nocapture`
- **AC-5. Given** the AC-1 fixture rotated by k·11.25° for k = 0..16, **when** `detect_bridging_direction_deg` runs for each rotation, **then** every output lies in the half-open range [0, 180) and equals (k·11.25) mod 180 within 10⁻³ degrees (degrees-mod-180 boundary convention, D6). | `cargo test -p slicer-core --features host-algos --test bridge_orientation_tdd -- rotated_rect_sweep_stays_in_half_open_range --nocapture`
- **AC-6. Given** a region with gated (non-empty) `bridge_areas` and previous-layer contours, **when** `update_external_bridge_orientation` runs at the post-gate seam, **then** `region.bridge_orientation_deg` equals `detect_bridging_direction_deg(gated_bridge_areas, lower_layer_slices)`; an rg check confirms `commit_shell_classification_builtin` (`crates/slicer-runtime/src/slice_postprocess_prepass.rs`) invokes it adjacent to 234's gate. | `cargo test -p slicer-core --features host-algos --test bridge_orientation_tdd -- orientation_written_from_gated_geometry empty_bridge_areas_leave_orientation_untouched --nocapture && rg -q 'update_external_bridge_orientation' crates/slicer-runtime/src/slice_postprocess_prepass.rs`
- **AC-7. Given** `resources/overhang.obj` sliced twice with the core modules, **when** the two G-code outputs are compared and parsed with M83 relative-E semantics (positive E delta on an XY move = extrusion), keyed by Z (never layer index), with `;TYPE:` carried across layer changes, **then** the two files are byte-identical, at least one layer carries Bridge-type extrusion, Bridge-type layers are a strict subset of all layers (234 regression guards I2/no-flooding), and all Bridge-type extrusion moves carry a single identical feedrate value (I7 untouched-surface guard). | `cargo run --bin pnp_cli --release -- slice --model resources/overhang.obj --output target/orient_a.gcode --module-dir modules/core-modules && cargo run --bin pnp_cli --release -- slice --model resources/overhang.obj --output target/orient_b.gcode --module-dir modules/core-modules && cmp target/orient_a.gcode target/orient_b.gcode && python3 -c "import sys,re\nz=None;t='';bz=set();az=set();fs=set()\nfor l in open(sys.argv[1]):\n l=l.strip()\n if l.startswith(';TYPE:'): t=l[6:].strip()\n m=re.search(r'\\bZ(-?\\d+\\.?\\d*)',l)\n if m: z=float(m.group(1)); az.add(z)\n m=re.search(r'\\bF(\\d+\\.?\\d*)',l)\n if m: f=float(m.group(1))\n m=re.search(r'\\bE(-?\\.?\\d+\\.?\\d*)',l)\n if m and float(m.group(1))>0 and z is not None and t and 'ridge' in t:\n  bz.add(z); fs.add(f)\nassert len(bz)>=1,'no bridge site'\nassert len(bz)<len(az),f'flooding {len(bz)}/{len(az)}'\nassert len(fs)==1,f'bridge feedrates not uniform: {sorted(fs)}'\nprint(f'bridge_layers={len(bz)}/{len(az)} feedrate={fs}')" target/orient_a.gcode`
- **AC-8. Given** the retirement of the mesh-stage heuristic, **when** the change surface is inspected and the mesh-analysis suite runs, **then** `compute_bridge_direction_deg` no longer exists in `crates/slicer-core/src/algos/mesh_analysis.rs`, and `algo_mesh_analysis_tdd` passes with the anchor-width/`is_valid` pre-filter logic intact. | `bash -c 'rg -q "fn compute_bridge_direction_deg" crates/slicer-core/src/algos/mesh_analysis.rs && exit 1 || exit 0' && cargo test -p slicer-core --features host-algos --test algo_mesh_analysis_tdd`

## Negative Test Cases

- **AC-N1. Given** an equal-cost candidate set (cross fixture: one horizontal and one vertical floating edge of identical length 10 mm, so both candidate normals accumulate exactly 10.0 cost in floating-point-exact axis-aligned arithmetic), **when** the selection runs, **then** the result is exactly 0.0 degrees — the smallest quantized angle (`ceil(atan2(n.y, n.x) · 1000)` key: −1570 beats 0) per ADR-0061 — and never 90.0 or any order-dependent value (canonical hash-order first-wins and the stash's first-wins heuristic are both replaced). | `cargo test -p slicer-core --features host-algos --test bridge_orientation_tdd -- equal_cost_tie_resolves_smallest_quantized_angle --nocapture`
- **AC-N2. Given** the legacy `BridgeDetector::detect_angle` class semantics (5° sweep, coverage cost, spacing tie-break) that this packet must NOT port, **when** the change surface is inspected and the AC-N1 cross fixture is rotated 7° (expected exactly 7.0°, not a multiple of 5), **then** no sweep/coverage marker exists in the edited algorithms and the output is 7.0° within 10⁻³ (a 5°-snapping sweep would emit 5 or 10). | `bash -c 'rg -q "detect_angle|angle_step|for angle in" crates/slicer-core/src/algos/prepass_slice.rs crates/slicer-core/src/algos/mesh_analysis.rs && exit 1 || exit 0' && cargo test -p slicer-core --features host-algos --test bridge_orientation_tdd -- rotated_cross_rejects_legacy_five_degree_snap --nocapture`

## Verification

- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo xtask check-literals`
- `cargo test -p slicer-core --features host-algos --test bridge_orientation_tdd`
- `cargo xtask test --summary` (closure run; the orientation output feeds `rectilinear-infill` through the guest-visible region view)

## Authoritative Docs

- `docs/specs/bridge-parity-plan.md` - direct read (source plan: §2 baseline, §3/F2, §4 W-B, §6 invariants, decisions D5/D6/D9/D10).
- `docs/adr/0061-deterministic-bridge-orientation-tie-break.md` - direct read (short; the tie-break rule this packet implements — cite, never recreate).
- `docs/08_coordinate_system.md` - delegated SUMMARY of the mm↔unit conversion sections (SCALED_EPSILON = 1e-4 mm = 1 unit).

## Doc Impact Statement (Required)

- **`none`** - this packet replaces a private orientation function and rewires which value feeds the existing `bridge_orientation_deg` field; it adds no IR/WIT/scheduler/claim/manifest/host-service/SDK contract, keeps the degrees-mod-180 representation mandated by D6, and bumps no schema or version constant. The tie-break divergence is already recorded in ADR-0061 by design (D5), so no new doc section is created.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/BridgeDetector.hpp` — the active inline `detect_bridging_direction(Lines, Polygons)` / `(Polygons, Polygons)` overload pair: floating-edge candidate normals (`Line::normal()` = `(dy, −dx)`), `ceil(atan2·1000)` quantization, Σ|edge·normal| cost, perpendicular flip of the winner, and the principal-component minor-axis / `{1,0}` fallbacks. This is the port target.
- `OrcaSlicerDocumented/src/libslic3r/LayerRegion.cpp` — `LayerRegion::process_external_surfaces` call site: `detect_bridging_direction(to_polygons(initial), to_polygons(lower_layer->lslices))` and the storage convention `PI + atan2(dir.y, dir.x)` radians CCW-from-X.
- `OrcaSlicerDocumented/src/libslic3r/PrincipalComponents2D.cpp` — `compute_principal_components`: signed-area moment accumulation, zero-area → zero vectors, eigen decomposition returning major/minor sorted; the minor axis is the fallback direction.
- `OrcaSlicerDocumented/src/libslic3r/libslic3r.h` — `EPSILON = 1e-4` and `SCALED_EPSILON = scale_(EPSILON)` definitions backing the anchor-expansion tolerance.
- `OrcaSlicerDocumented/src/libslic3r/BridgeDetector.cpp` — `BridgeDetector::detect_angle` (5° sweep, coverage cost, spacing tie-break): a separate legacy sweep implementation that the ACTIVE call path does not reach (`LayerRegion::process_external_surfaces` selects the inline `detect_bridging_direction` overloads) — the explicit rejected alternative.

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- obey the shared absolute context bands: 120k reading budget with hand-off at 150k (standard); the extended band (240k reading / 300k hard stop) only via swarm's escalation protocol

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation (an extended-band run may carry a single L step only when `design.md` justifies why it cannot be split).
