---
status: draft
packet: 302-bridge-angle-counterbore-classic-perimeters
task_ids: []
backlog_source: docs/specs/orca-feature-gap/issues/84-author-packet-p77-quality-bridging-classic-perimeters.md (wayfinder map: Close the OrcaSlicer FFF feature gap)
context_cost_estimate: M
copy_note: Queue packet from the wayfinder map "Close the OrcaSlicer FFF feature gap"; authored under ticket 84 (P77).
---

# Packet Contract: 302-bridge-angle-counterbore-classic-perimeters

## Goal

Make the P77 `bridge_angle` and `counterbore_hole_bridging` keys drive the tree pipeline at canonical value semantics — an explicit external-bridge direction override plus sacrificial/partial bridging of stepped holes — with zero declaration-only keys and byte-identical defaults.

## Scope Boundaries

P77 is two Tier B keys owned by Quality/Bridging (`LayerRegion.cpp` plus `PerimeterGenerator.cpp` in canonical), both live in canonical's slicing pipeline and zero-occurrence as behaviour in this tree. Claim-time grounding (ticket 84) holds both in: no live decision point exists for either (external orientation is always auto-detected; stepped holes get no bridge treatment), no draft packet owns either decision (packet 235 built the auto-detection the override rides, not the override; the 299/300 mentions are other packets' non-borrows), and the tier-table owner is accurate but narrowed for this tree — the seam is the host prepass `commit_shell_classification_builtin`, never a module (perimeter modules consume preclassified buckets; ticket-36 precedent). The packet wires both through two `ResolvedConfig` rows (per-object via the existing overlay, explicitly not ticket 125's tool axis): `bridge_angle > 0` overwrites the detected external orientation verbatim (the live `internal_bridge_angle` arm's exact semantics), and the counterbore stage authors hole-bearing unsupported spans into `bridge_areas` — whole uncovered spans in `filled` mode, rim spans only in `partial` mode. Rule 4 does not fire: the override branches inside one prepass seam and the stage authors existing buckets consumed by existing holders — no cross-module algorithm selection, no claim holders to create, mirroring the `seam_position` precedent in the map Notes.

## Prerequisites and Blockers

- Depends on: nothing. All symbols below are live on HEAD (verified at authoring); `commit_shell_classification_builtin` (`crates/slicer-runtime/src/slice_postprocess_prepass.rs`), `gate_bridge_areas_by_unsupported_span` + `update_external_bridge_orientation` + `detect_bridging_direction_deg` (`crates/slicer-core/src/algos/prepass_slice.rs`), `gate_internal_bridge_sites` (same runtime prepass file), `RegionMapIR::config_for` / `intern_config` (`crates/slicer-ir/src/slice_ir.rs`), the `declare_resolved_config!` macro + `extract_float` / `extract_string` (`crates/slicer-ir/src/resolved_config.rs`), `difference` / `union` (`slicer_core::polygon_ops`), and the `rectilinear-infill` bridge consumer (`modules/core-modules/rectilinear-infill/src/lib.rs`) are landed tree code, not packet dependencies. Exact canonical formulas the wiring mirrors are borrowed via implementer-side delegated oracle reads (design.md lists them).
- Related work, not a blocker: bridge-parity work in `docs/spec_packets/_OLD/` (packets 233/234/234a `status: implemented` — the buckets and gates this packet authors into); packet 235 `status: implemented` in the same `_OLD/` dir (the auto-detection semantics the override rides, whose split note explicitly returns the override-key plumbing to this ticket); draft packet 262a (owns the `align_infill_direction_to_model` rotation offset — the override applies without it, named non-borrow); draft packet 299 (P73 — the same prepass file gains stages; this packet's resolver call merges with — never duplicates — its edits; sequencing is file-merge order, not an activation dep); ticket 122 (prime-tower body — no shared helper, no dep); ticket 124 (sequential-printing validator — the `Print.cpp` reslice-invalidation entries ride it, named non-borrow); ticket 125 (per-tool model — explicitly NOT this packet's axis: both keys are canonical per-region scalars, carried by the existing per-object overlay, 290/291/296 precedent); ticket 132 (CONFIG_BLOCK reader contract — neither key has a padding twin; honest absence, true spellings ride there).
- Unblocks: wayfinder ticket 84 (P77 closes when this packet is authored). No edge to any other draft packet.
- Activation blockers: none. One deviation ID is minted (DEV-193, verified absent from the log and all drafts at authoring); if implementation surfaces another, re-derive `max(DEV-*)` over `docs/DEVIATION_LOG.md` + `docs/spec_packets/*/` before writing the row.

## Acceptance Criteria

- **AC-1. Given** `crates/slicer-ir/src/resolved_config.rs`, **when** the `declare_resolved_config!` invocation and `to_config_map` are inspected, **then** they declare `bridge_angle` (float, default `0.0`) and `counterbore_hole_bridging` (string, default `"none"`) with `cli` bindings of the same snake_case names and `to_config_map` arms emitting `ConfigValue::Float` / `ConfigValue::String`. | `rg -q 'cli "bridge_angle"' crates/slicer-ir/src/resolved_config.rs && rg -q 'cli "counterbore_hole_bridging"' crates/slicer-ir/src/resolved_config.rs && rg -q '"counterbore_hole_bridging".into()' crates/slicer-ir/src/resolved_config.rs 2>&1 | tee target/test-output.log | tail -3`
- **AC-2. Given** an external-bridge region whose detected orientation is `0.0`, sliced with `bridge_angle = 45` versus `0` (all else default), **when** the committed orientations are compared, **then** the `45` run stamps `45.0` and the `0` run keeps the detected value, and repeated `45` runs are identical. | `cargo test -p slicer-runtime --lib bridge_angle 2>&1 | tee target/test-output.log | tail -5`
- **AC-3. Given** a two-layer stepped-hole fixture (lower full slab, upper slab with a square hole over air) sliced with `counterbore_hole_bridging = sacrificiallayer` versus `none`, **when** the committed `bridge_areas` are compared, **then** the `sacrificiallayer` run covers the uncovered hole interior while the `none` run authors nothing, and the authored span carries a non-default orientation. | `cargo test -p slicer-runtime --test executor counterbore_filled_authors_hole_interior_as_bridge 2>&1 | tee target/test-output.log | tail -5`
- **AC-4. Given** the same stepped-hole fixture sliced with `counterbore_hole_bridging = partiallybridge`, **when** the committed `bridge_areas` are compared against the `sacrificiallayer` run, **then** the `partiallybridge` run authors the rim unsupported spans but its `bridge_areas` contain no hole-interior polygon. | `cargo test -p slicer-runtime --test executor counterbore_partial_leaves_hole_interior_unbridged 2>&1 | tee target/test-output.log | tail -5`
- **AC-5. Given** a single-region cuboid print sliced with both keys unset versus explicitly at defaults (`bridge_angle = 0`, `counterbore_hole_bridging = none`), **when** the committed `SliceIR` vectors are compared, **then** they are byte-identical to each other (zero is automatic = pre-packet shape; `none` is off = pre-packet shape). | `cargo test -p slicer-runtime --lib counterbore_defaults_leave_regions_untouched 2>&1 | tee target/test-output.log | tail -5`
- **AC-6. Given** the packet's disposition table and the deviation log, **when** they are read, **then** the table lists exactly the 2 P77 keys, both wired, with zero declaration-only keys, `DEV-193` names this packet's divergences in `docs/DEVIATION_LOG.md`, the regenerated `docs/15` tables contain both keys, and the doc-15 freshness gate passes. | `rg -q 'declaration-only keys: 0' docs/spec_packets/302-bridge-angle-counterbore-classic-perimeters/requirements.md && rg -q 'DEV-193' docs/DEVIATION_LOG.md && rg -q 'counterbore_hole_bridging' docs/15_config_keys_reference.md && rg -q 'bridge_angle' docs/15_config_keys_reference.md && cargo xtask gen-config-docs --check 2>&1 | tee target/test-output.log | tail -5`

## Negative Test Cases

- **AC-N1. Given** a stepped-hole fixture sliced with `counterbore_hole_bridging` set to an unrecognised spelling, **when** the committed `bridge_areas` are compared against the `none` run, **then** they are identical (unknown spellings fall back to `none`, the `flat_bridge_closing_join` precedent — never a silent new behaviour). | `cargo test -p slicer-runtime --lib counterbore_unknown_spelling_falls_back_to_none 2>&1 | tee target/test-output.log | tail -5`
- **AC-N2. Given** a two-layer overhang fixture with no holes, **when** `sacrificiallayer` versus `none` runs are compared, **then** they are identical (the stage only fires on hole-bearing spans — no hole, no authoring). | `cargo test -p slicer-runtime --lib counterbore_hole_free_spans_unaffected 2>&1 | tee target/test-output.log | tail -5`

## Verification

- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test -p slicer-runtime --test executor prepass_slice_and_shell 2>&1 | tee target/test-output.log | tail -5`

## Authoritative Docs

- `docs/01_system_architecture.md` - delegated SUMMARY (prepass vs module seam placement; rule-4 trigger test does not fire).
- `docs/03_wit_and_manifest.md` - delegated SUMMARY (macro row shape; `ConfigView::from_declared` whitelist does not apply to host-prepass keys).
- `docs/04_host_scheduler.md` - delegated SUMMARY (prepass ordering: ShellClassification runs before PaintSegmentation, so the stage sees BASE timelines).
- `docs/DEVIATION_LOG.md` - direct read of DEV-171 scalar-vs-vector precedent + DEV-192 minting convention.
- `docs/02_ir_schemas.md` - delegated SUMMARY (five-way partition invariant the authored spans must preserve: authored `bridge_areas` stay disjoint from solid fills).

## Doc Impact Statement (Required)

- `docs/DEVIATION_LOG.md` section "DEV-193" - `rg -q 'DEV-193' docs/DEVIATION_LOG.md`.
- `docs/15_config_keys_reference.md` generated tables gain the 2 keys - `rg -q 'counterbore_hole_bridging' docs/15_config_keys_reference.md` and `rg -q 'bridge_angle' docs/15_config_keys_reference.md`.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` + `PrintConfig.hpp` + `Preset.cpp` — canonical declarations (`bridge_angle` coFloat default `0`, min 0 max 180; `counterbore_hole_bridging` coEnum default `chbNone`, spellings `none`/`partiallybridge`/`sacrificiallayer`; borrowed exactly).
- `OrcaSlicerDocumented/src/libslic3r/LayerRegion.cpp` — `process_external_surfaces` top and bottom `bridge_angle` arms (`> 0` gate, absolute vs `relative_bridge_angle`-relative application, `align_infill_direction_to_model` offset; borrow the gate shape — the relative and align companions are explicitly NOT borrowed).
- `OrcaSlicerDocumented/src/libslic3r/PerimeterGenerator.cpp` — `process_no_bridge` island separation, `BridgeDetector` coverage, and filled-vs-partial handling (borrow the mode distinction; the detector math and anchor-band shaping are explicitly NOT borrowed — the span gate is the port's bridgeability test).
- `OrcaSlicerDocumented/src/libslic3r/Layer.cpp` — `make_perimeters` chbFilled extra-fill recovery (NOT borrowed — the port has no region-merge step that loses fills).
- `OrcaSlicerDocumented/src/libslic3r/PrintObject.cpp` — `detect_surfaces_type` chbFilled slice-union counting sacrificial fill as solid above (NOT borrowed — above-layer support-map restructure is out of scope, recorded divergence).
- `OrcaSlicerDocumented/src/libslic3r/Fill/FillBase.cpp` — `infill_direction` bridge-angle consumption (NOT borrowed as a separate arm — orientation flows to fillers through the existing `bridge_orientation_deg` bucket both holders already consume).

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- obey the shared absolute context bands: 120k reading budget with hand-off at 150k (standard); the extended band (240k reading / 300k hard stop) only via swarm's escalation protocol

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation (an extended-band run may carry a single L step only when `design.md` justifies why it cannot be split).
