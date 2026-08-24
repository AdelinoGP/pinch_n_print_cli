---
status: draft
packet: 234a-internal-bridge-support-gating
task_ids: []
backlog_source: docs/specs/orca-feature-gap/issues/82-author-packet-p75-quality-bridging-bridge-over-infill.md (internal-bridge filtering-parity half; see task-map.md)
context_cost_estimate: M
---

# Packet Contract: 234a-internal-bridge-support-gating

## Goal

Gate internal bridge-over-infill construction by canonical `PrintObject.cpp::bridge_over_infill` lower-layer support semantics — candidates are upper-layer internal-solid interface surfaces qualified by unsupported span against the layer below, never raw current-layer sparse infill — relocating construction from the parallel-slicing InfillPostProcess arm into the sequential ShellClassification prepass beside 234's false-site gate, so internal bridges appear only where infill is genuinely unsupported (measured target: canonical emits exactly ONE internal-bridge site on calicat; our tree currently emits 148).

## Scope Boundaries

This packet replaces the unqualified candidate selection at `LayerStageCommit::InfillPostProcess` in `crates/slicer-runtime/src/layer_executor.rs` (the `candidate_voids = difference(sparse_infill_area, bridge_areas)` block with its sliver-guard filter) with: (1) pure support-math functions in `crates/slicer-core/src/algos/bridge_over_infill.rs` porting canonical's unsupported-area computation (closing of lower-layer fill polygons, shrink by `expansion_multiplier*spacing` with mult=3 default, subtraction of grown lower-layer solids, per-surface unsupported intersection, `9*spacing^2` partially-supported area gate, `expand(unsupported, 4*spacing)` clip to surface); (2) a new sequential pass in `crates/slicer-runtime/src/slice_postprocess_prepass.rs` inside `commit_shell_classification_builtin`'s stage, ordered after 234's `gate_bridge_areas_by_unsupported_span`, that walks region timelines across committed layers and performs qualification + anchored-line construction + `InfillRegion.internal_bridge_infill` population; (3) removal of the InfillPostProcess-time construction. Note: `internal_bridge_infill` lives on `InfillRegion` (the Infill-stage IR), not `SliceRegion` — Step 2 must confirm the committed infill artifact is reachable at ShellClassification before wiring population. It maps the boolean `dont_filter_internal_bridges` onto canonical's enum behaviour (`false` = `ibfDisabled` full filter, `true` = `ibfNofilter` bypass of the area/partial gate). It does NOT change the angle algorithm (`determine_bridging_angle` stays), does not add IR fields or WIT types, does not touch `counterbore_hole_bridging` or the external-site orientation path from 235, and does not alter module manifests.

## Prerequisites and Blockers

- Depends on: `docs/spec_packets/233-internal-bridge-over-infill/` (net-new symbols `ExtrusionRole::InternalBridgeInfill`, `determine_bridging_angle`, `construct_anchored_polygon` in `crates/slicer-core/src/algos/bridge_over_infill.rs`, field `InfillRegion.internal_bridge_infill`), `docs/spec_packets/234-bridge-false-site-gating/` (the ShellClassification seam pattern: `gate_bridge_areas_by_unsupported_span` in `crates/slicer-core/src/algos/prepass_slice.rs`, invoked from `commit_shell_classification_builtin` in `crates/slicer-runtime/src/slice_postprocess_prepass.rs`; `build_region_timelines(slices: &[SliceIR])` multi-layer access), and `docs/spec_packets/235-external-bridge-orientation/` (post-gate ordering precedent; external row must not regress). All prerequisite symbols verified present on disk during this packet's authoring session (2026-08-24); verify symbol presence directly at implementation time rather than re-deriving from those packets' status lines.
- Supersedes part of 233's delivered design: 233's AC-N2 recorded "prepass stays free of internal-bridge logic". This packet reverses that placement decision explicitly — rationale in `design.md` Architecture Constraints. The reversal is a designed consequence of the same parallel-slicing limitation 234 itself hit, not an undocumented deviation.
- Unblocks: honest I2 (no-flooding) parity on real models; future coverage/anchoring work that assumes filtered internal-bridge sites.
- Activation blockers: none.

## Acceptance Criteria

State ACs only here; `requirements.md` references their IDs.

- **AC-1. Given** a lower layer whose dense-fill polygons fully cover an upper internal-solid surface (no voids), **when** the new support-math function runs, **then** it returns an empty unsupported-area set — the surface does NOT qualify for bridging. | `cargo test -p slicer-core --features host-algos --test bridge_support_gating_tdd -- fully_supported_surface_yields_no_unsupported_area --nocapture`
- **AC-2. Given** a lower layer with a rectangular void under an upper surface such that the computed unsupported region is larger than `9*spacing^2` after closing/shrink/grow math (mult=3), **when** qualification runs with filtering enabled, **then** the surface qualifies and the returned bridge polygon equals `expand(unsupported_intersection, 4*spacing)` clipped to the surface within polygon-equality tolerance. | `cargo test -p slicer-core --features host-algos --test bridge_support_gating_tdd -- unsupported_span_qualifies_and_clip_expand_matches_canonical --nocapture`
- **AC-3. Given** a partially-supported surface whose unsupported intersection area lies between zero and `9*spacing^2`, **when** qualification runs with `dont_filter_internal_bridges=false`, **then** it rejects the surface; with the key set `true` (`ibfNofilter`), **then** it accepts. | `cargo test -p slicer-core --features host-algos --test bridge_support_gating_tdd -- partial_support_area_gate_and_nofilter_bypass --nocapture`
- **AC-4. Given** the relocation, **when** the change surface is inspected and the bridging suites run, **then** `construct_anchored_polygon` no longer appears in `crates/slicer-runtime/src/layer_executor.rs` (name-resolution-tolerant rg), the new prepass pass appears in `crates/slicer-runtime/src/slice_postprocess_prepass.rs` at a LINE NUMBER strictly greater than the `gate_bridge_areas_by_unsupported_span` invocation line (order-falsifying comparison), and `bridge_over_infill_tdd` plus `bridge_false_site_gating_tdd` stay green. | `bash -c 'rg -q "construct_anchored_polygon" crates/slicer-runtime/src/layer_executor.rs && exit 1 || exit 0' && bash -c 'gate=$(rg -n "gate_bridge_areas_by_unsupported_span" crates/slicer-runtime/src/slice_postprocess_prepass.rs | head -1 | cut -d: -f1); pass=$(rg -n "construct_anchored_polygon|bridge_over_infill::" crates/slicer-runtime/src/slice_postprocess_prepass.rs | head -1 | cut -d: -f1); test -n "$gate" && test -n "$pass" && test "$pass" -gt "$gate"' && cargo test -p slicer-core --features host-algos --test bridge_over_infill_tdd && cargo test -p slicer-core --features host-algos --test bridge_false_site_gating_tdd`
- **AC-5. Given** `resources/calicat.stl` imported from this packet's measured baseline, **when** the model is sliced twice with the core modules and both outputs are parsed with M83-relative-E semantics keyed by Z with `;TYPE:` carried across layer changes, **then** the two slices are byte-identical, Internal-Bridge-labelled layers number at most 6 (canonical: exactly 1, near Z≈29.45), total bridge-labelled extrusion is at most 5000 mm (baseline before this packet: 86675.76 mm across 148 layers; OrcaSlicer reference: 950.56 mm), and the external Bridge row at Z≈3.2 keeps dominant angle within [85°, 95°] (packet-235 regression guard; baseline after 235: 90.0° / 74 segs / 324.6 mm). | `cargo test -p slicer-runtime --test e2e -- calicat_internal_bridge_gating_e2e_tdd --nocapture`
- **AC-6. Given** the wedge regression model, **when** the existing wedge-linked-infill e2e suite runs with its slot-ceiling assertions at print_z 28.2, **then** every assertion passes unchanged — the relocation must not shift Bottom/Bridge classifications the wedge pins. | `cargo test -p slicer-runtime --test e2e wedge_linked_infill_report_tdd`

## Negative Test Cases

- **AC-N1. Given** an internal-solid surface sitting ENTIRELY above lower-layer dense material so its unsupported intersection is empty, **when** `qualify_internal_bridge_surface` runs, **then** it returns None and no bridge polygon is produced — proving construction cannot fire without genuine unsupported span (root cause of the 148-layer flood). | `cargo test -p slicer-core --features host-algos --test bridge_support_gating_tdd -- fully_supported_surface_qualifies_nothing --nocapture`

## Verification

- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo xtask check-literals`
- `cargo xtask build-guests --check` (exit 0 expected; host-only change surface)
- `cargo test -p slicer-core --features host-algos --test bridge_support_gating_tdd`
- `cargo xtask test --summary --workspace` (closure run only, at acceptance ceremony)

## Authoritative Docs

- `docs/specs/bridge-parity-plan.md` - direct read (F3 finding, §4 W-C row, §6 invariant list: I2/I3/I7 wording).
- `docs/15_config_keys_reference.md` - direct read of the `dont_filter_internal_bridges` entry only (semantics change to canonical enum mapping).
- `docs/04_host_scheduler.md` - delegated SUMMARY of the `ShellClassification` stage section (stage-ordering rules the relocation must respect).

## Doc Impact Statement (Required)

- **`docs/15_config_keys_reference.md`** — update the `dont_filter_internal_bridges` row: value now selects between canonical full-filtering (`false`) and bypass (`true`) of the lower-layer support gate instead of only a sliver guard. Verification grep: `rg -n "dont_filter_internal_bridges" docs/15_config_keys_reference.md` must show the updated semantics line.
- **`docs/specs/bridge-parity-plan.md`** — append a dated addendum row to §3/F3 noting the filtering gap found by post-series calicat re-slice (148 layers / 86675.76 mm vs canonical 1 / 526.27 mm) and pointing at this packet. Verification grep: `rg -n "234a" docs/specs/bridge-parity-plan.md` must return a hit.
- No other doc sections are touched: no IR/WIT/schema/claim changes, no scheduler stage addition (relocation reuses the existing ShellClassification stage).

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/PrintObject.cpp` — `bridge_over_infill` gather pass: `unsupported_area` computation (closing of lower fills, shrink `expansion_multiplier*spacing`, minus grown lower solids), per-surface `unsupported = intersection(s, unsupported_area)`, empty + `9*spacing^2` partial gates, `expand(unsupported, 4*spacing)` clip, leftover-island remerge (`spacing^2 < area < 12*spacing^2`), and the apply phase reclassifying `stInternalSolid` → `stInternalBridge`.
- `OrcaSlicerDocumented/src/libslic3r/PrintConfig.hpp` / `PrintConfig.cpp` — `InternalBridgeFilter` enum `{ibfDisabled, ibfLimited, ibfNofilter}` and `dont_filter_internal_bridges` option definition (default `ibfDisabled`; `ibfLimited` sets `expansion_multiplier = 1`).
- `OrcaSlicerDocumented/src/libslic3r/Fill/Fill.cpp` — `erInternalBridgeInfill` assignment when `surface.is_internal_bridge()` (role-emission precedent already mirrored by 233).

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- obey the shared absolute context bands: 120k reading budget with hand-off at 150k (standard); the extended band (240k reading / 300k hard stop) only via swarm's escalation protocol

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation (an extended-band run may carry a single L step only when `design.md` justifies why it cannot be split).
