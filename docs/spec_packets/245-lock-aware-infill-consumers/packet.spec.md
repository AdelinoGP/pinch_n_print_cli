---
status: draft
packet: 245-lock-aware-infill-consumers
task_ids:
  - TASK-355
depends_on:
  - 244-order-locked-extrusion-sequences
backlog_source: docs/07_implementation_status.md
context_cost_estimate: M
---

# Packet Contract: 245-lock-aware-infill-consumers

## Goal

Make the three infill consumers — the infill linker, the path optimizer, and G-code emission — honor
`ExtrusionPath3D.order_lock` sequences: locked paths bypass linking/clipping/simplification and are
appended verbatim, untagged fill is carved around their swept footprint, and all-`None` slices remain
byte-identical to today.

## Scope Boundaries

This packet changes behavior only for paths carrying a non-`None` `order_lock` (the carrier and host
enforcement are introduced by packet 244). It does NOT add any new producer of locks, does NOT change the
`ExtrusionPath3D`/WIT/`OrderedEntityView` surface, and does NOT touch the host enforcement points.
The linker gains a locked-passthrough branch plus a module-local swept-footprint carve (ADR-0026
single-caller rule); the optimizer treats each locked block as one non-reversible nearest-neighbor
candidate; the emitter bypasses Douglas-Peucker and `min_segment_length` pruning for locked paths.
Structural parity tests prove all-`None` neutrality.

## Prerequisites and Blockers

- Depends on: 244-order-locked-extrusion-sequences (the `order_lock: Option<u64>` carrier, the
  `OrderedEntityView.order_lock` projection, and the host enforcement contract this packet's
  consumers honor).
- Unblocks: 246-wave-overhang-bridge-fill (the wave module emits order-locked `BridgeInfill` paths
  and relies on these consumers to preserve them).
- Activation blockers: none known; the change is additive and gated on `order_lock.is_some()`.

## Acceptance Criteria

State ACs only here; `requirements.md` references their IDs.

- **AC-1. Given** a region whose `InfillIR` contains one or more `ExtrusionPath3D` with
  `order_lock: Some(tag)`, **when** the infill linker's `process_bucket_role` runs, **then** every
  locked path is appended verbatim per region in emission order — same points, same order, same
  direction, same widths — with no boundary lookup, no linking, no overlap-offset trimming, and no
  clipping. |
  `cargo test -p infill-linker --test orchestrate_tdd -- locked_paths_bypass_linking_and_clipping --exact 2>&1 | tee target/test-output.log | grep -qE "^test result: ok\. 1 passed" && echo P245_LINKER_PASSTHROUGH`
- **AC-2. Given** a region with locked paths whose swept footprint (one endpoint-width trapezoid per
  segment plus a round disk at every vertex) overlaps untagged fill of another role, **when** the
  linker's carve pass runs, **then** the swept footprint is differenced out of every untagged role
  bucket of the same region, and the locked paths themselves are never carved. |
  `cargo test -p infill-linker --test orchestrate_tdd -- locked_swept_footprint_carved_from_untagged_fill --exact 2>&1 | tee target/test-output.log | grep -qE "^test result: ok\. 1 passed" && echo P245_LINKER_CARVE`
- **AC-3. Given** an `OrderedEntityView` slice containing a locked block (two or more adjacent
  entities sharing a non-`None` `order_lock`), **when** `group_then_nearest_neighbor` runs, **then**
  the block is emitted as one nearest-neighbor candidate (authored first start, last end), is never
  reversed and never split, and its internal order is preserved. |
  `cargo test -p path-optimization-default --lib -- locked_block_is_single_non_reversible_candidate --exact 2>&1 | tee target/test-output.log | grep -qE "^test result: ok\. 1 passed" && echo P245_OPTIMIZER_BLOCK`
- **AC-4. Given** a locked `ExtrusionPath3D` with many collinear/interior points, **when**
  `DefaultGCodeEmitter::emit_gcode` runs, **then** every authored point is emitted — the path
  bypasses both `simplify_polyline_mm` (Douglas-Peucker at `tolerance_for_role`) and
  `drop_short_segments_mm` (`min_segment_length`) — while coordinate formatting at serialization
  still applies. |
  `cargo test -p slicer-gcode --test gcode_emit_tdd -- locked_paths_bypass_simplification_and_min_segment --exact 2>&1 | tee target/test-output.log | grep -qE "^test result: ok\. 1 passed" && echo P245_EMITTER_BYPASS`
- **AC-5. Given** representative linker, optimizer, and emitter fixtures whose every path carries
  `order_lock: None`, **when** the new branches run, **then** the produced path/entity/G-code
  structures are identical to the pre-packet output (all-`None` neutrality; no new golden files). |
  `cargo test -p infill-linker --test orchestrate_tdd -- all_none_locks_neutrality --exact 2>&1 | tee target/test-output.log | grep -qE "^test result: ok\. 1 passed" && cargo test -p path-optimization-default --lib -- all_none_locks_neutrality --exact 2>&1 | tee -a target/test-output.log | grep -qE "^test result: ok\. 1 passed" && cargo test -p slicer-gcode --test gcode_emit_tdd -- all_none_locks_neutrality --exact 2>&1 | tee -a target/test-output.log | grep -qE "^test result: ok\. 1 passed" && echo P245_ALL_NONE_NEUTRAL`

## Negative Test Cases

- **AC-N1. Given** a locked path that extends beyond its role's partitioned polygon into a
  neighboring fill domain (the anchor-band case), **when** the linker runs, **then** the locked path
  is preserved verbatim (NOT clipped back to the partitioned polygon) and only the untagged fill of
  the same region is carved by its swept footprint. |
  `cargo test -p infill-linker --test orchestrate_tdd -- locked_path_crossing_fill_domain_not_clipped --exact 2>&1 | tee target/test-output.log | grep -qE "^test result: ok\. 1 passed" && echo P245_CROSS_DOMAIN_PRESERVED`
- **AC-N2. Given** a locked block whose nearest-neighbor permutation would reverse or split it,
  **when** `group_then_nearest_neighbor` runs, **then** the block is emitted intact (authored order
  and direction) and the reversal/split is rejected rather than applied. |
  `cargo test -p path-optimization-default --lib -- locked_block_never_split_or_reversed --exact 2>&1 | tee target/test-output.log | grep -qE "^test result: ok\. 1 passed" && echo P245_OPTIMIZER_NO_SPLIT`

## Verification

- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo xtask check-literals`
- `cargo test -p infill-linker --test orchestrate_tdd && cargo test -p path-optimization-default --lib && cargo test -p slicer-gcode --test gcode_emit_tdd`

## Authoritative Docs

- `docs/specs/wave-overhangs-bridge-fill-plan.md` - normative plan; §"Packet 3 — Lock-aware
  consumers" (C1–C4) and Appendix A (ADR draft "Sequence-locked paths may occupy neighboring fill
  domains") are the governing brief.
- `docs/02_ir_schemas.md` - direct range read of §"Post-`Layer::Perimeters` invariant: four
  canonical fill polygons" (lines ~596-626); the doc is over 300 lines so only this range is read
  directly.
- `docs/adr/0026-infill-linking-algorithms-in-linker-module.md` - single-caller/single-home rule the
  carve pass must obey.
- `docs/adr/0011-perimeter-module-owns-wall-sequencing.md` - wall-subsequence precedent the
  optimizer's locked-block handling mirrors.

## Doc Impact Statement (Required)

- `docs/adr/0063-sequence-locked-paths-may-occupy-neighboring-fill-domains.md` - new ADR, content
  from the plan's Appendix A draft (re-derived number 0063: 0062 is packet 244's order-lock ADR, and
  `docs/adr/` currently ends at 0061). |
  `rg -q '^# ADR-0063' docs/adr/0063-sequence-locked-paths-may-occupy-neighboring-fill-domains.md && echo P245_ADR_LANDED`
- `docs/02_ir_schemas.md` §"Post-`Layer::Perimeters` invariant: four canonical fill polygons" - amend
  the closing sentence ("Each fill claim holder … emits over exactly one of these polygons with zero
  polygon math") to add the order-lock exception: order-locked paths are self-clipping and may extend
  into neighboring fill domains; the linker differences untagged fill by their swept footprint. |
  `rg -q 'order-lock' docs/02_ir_schemas.md && rg -q 'self-clipping' docs/02_ir_schemas.md && echo P245_IR_DOCS_AMENDED`
- `CONTEXT.md` §"Infill" - amend the entry to note that order-locked paths may occupy neighboring
  fill domains under the self-clipping exception. |
  `rg -q 'order-lock' CONTEXT.md && echo P245_CONTEXT_AMENDED`

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- obey the shared absolute context bands: 120k reading budget with hand-off at 150k (standard); the extended band (240k reading / 300k hard stop) only via swarm's escalation protocol

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation (an extended-band run may carry a single L step only when `design.md` justifies why it cannot be split).
