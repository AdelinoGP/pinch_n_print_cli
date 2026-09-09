---
status: draft
packet: 299-object-level-shell-infill-planning
task_ids: []
backlog_source: docs/specs/orca-feature-gap/issues/80-author-packet-p73-strength-advanced-strength-object-level-planning.md (wayfinder map: Close the OrcaSlicer FFF feature gap)
context_cost_estimate: M
copy_note: Queue packet from the wayfinder map "Close the OrcaSlicer FFF feature gap"; authored under ticket 80 (P73).
---

# Packet Contract: 299-object-level-shell-infill-planning

## Goal

Make the 4 P73 object-level planning keys drive the tree pipeline at canonical value semantics — vertical-shell guarantee, extra-solid layer insertion, and multi-layer sparse-infill combination — with zero declaration-only keys and a pinned default-path change (vertical shells newly active at the canonical `ensure_all` default).

## Scope Boundaries

P73 is four Tier B keys owned by object-level planning (`PrintObject.cpp` in canonical: `discover_vertical_shells`, `discover_horizontal_shells`, `combine_infill`), all live in canonical's slicing pipeline and zero-occurrence as behaviour in this tree. Claim-time grounding (ticket 80) holds all four in: no live decision point exists, no draft packet owns the decision, and the tier-table owner is accurate but narrowed for this tree — the seam is the host prepass `commit_shell_classification_builtin` plus the five-way fill partition and the sparse-fill emission in the infill modules (ticket-36 precedent). The packet wires all four through new `ResolvedConfig` fields (per-object via the existing overlay, explicitly not ticket 125's tool axis): the shell pair drives two new prepass stages, the combination pair drives a grouping stage whose height metadata the sparse emitters consume. Rule 4 does not fire: the modes branch inside one prepass seam plus one sparse-merge emission, not cross-module algorithm selection — there are no claim holders to create, mirroring the `seam_position` precedent in the map Notes.

## Prerequisites and Blockers

- Depends on: nothing. All symbols below are live on HEAD (verified at authoring); `commit_shell_classification_builtin`, `resolve_shell_counts`, `convert_small_sparse_islands`, `sync_perimeter_infill_areas_into_slice`, `solid_fill_role`, the `declare_resolved_config!` macro, `region_map.config_for`, `CURRENT_SLICE_IR_SCHEMA_VERSION`, and `SliceRegionView` are landed tree code, not packet dependencies. Exact canonical formulas the wiring mirrors are borrowed via implementer-side delegated oracle reads (design.md lists them).
- Related work, not a blocker: ticket 122 (prime-tower body — no shared helper, no dep); ticket 124 (sequential-printing validator — the `Print.cpp` reslice-invalidation entries ride it, named non-borrow); ticket 125 (per-tool model — explicitly NOT this packet's axis: all four keys are canonical scalars, carried by the existing per-object overlay, 290/291/296 precedent); ticket 132 (CONFIG_BLOCK reader contract — canonical spellings ride there; no padding edits here, rule 2); packet 264 (top-bottom surfaces — gyroid's solid-density omission stands, named non-borrow); packet 275 (top-fill order — no shared helper, no dep).
- Unblocks: wayfinder ticket 80 (P73 closes when this packet is authored). No edge to any other draft packet (no draft packet mentions any of the four outside a handoff suggestion; 234a's `extra_solid_infills` non-borrow is doc-only, not ownership).
- Activation blockers: none. One deviation ID is minted (DEV-190, verified absent from the log and all drafts at authoring); if implementation surfaces another, re-derive `max(DEV-*)` over `docs/DEVIATION_LOG.md` + `docs/spec_packets/*/` before writing the row.

## Acceptance Criteria

- **AC-1. Given** `crates/slicer-ir/src/resolved_config.rs`, **when** the `declare_resolved_config!` invocation is inspected, **then** it declares `ensure_vertical_shell_thickness` (string, default `"ensure_all"`), `extra_solid_infills` (string, default `""`), `infill_combination` (bool, default `false`), and `infill_combination_max_layer_height` (float-or-percent, default `"100%"` percent-true), each with a `cli` binding of the same snake_case name. | `rg -q 'cli "ensure_vertical_shell_thickness"' crates/slicer-ir/src/resolved_config.rs && rg -q 'cli "extra_solid_infills"' crates/slicer-ir/src/resolved_config.rs && rg -q 'cli "infill_combination"' crates/slicer-ir/src/resolved_config.rs && rg -q 'cli "infill_combination_max_layer_height"' crates/slicer-ir/src/resolved_config.rs 2>&1 | tee target/test-output.log | tail -3`
- **AC-2. Given** a sloping-wall fixture (top surfaces overhanging sparse by ≥1 layer) sliced with `ensure_vertical_shell_thickness = "ensure_all"` versus `"none"` (all else default), **when** their committed `SliceIR` vectors are compared, **then** the `"ensure_all"` run carries strictly more `internal_solid_fill` area on the slope-adjacent layers and the `"none"` run matches the pre-packet baseline (no vertical shells), while the default (unset-key) run matches the `"ensure_all"` run (canonical default active — one intended default output change). | `cargo test -p slicer-runtime --lib vertical_shell_mode_drives_solid_fill 2>&1 | tee target/test-output.log | tail -5`
- **AC-3. Given** a 5-layer box fixture sliced with `extra_solid_infills = "3"` versus unset (all else default), **when** their committed `SliceIR` vectors are compared, **then** layer 3 (1-based) of the `"3"` run has its sparse area re-typed to `internal_solid_fill` and every other layer is identical to the unset run; `extra_solid_infills = "2#2"` converts layers 2–3 and nothing else. | `cargo test -p slicer-runtime --lib extra_solid_pattern_inserts_solid_layers 2>&1 | tee target/test-output.log | tail -5`
- **AC-4. Given** a 4-layer uniform-height box fixture sliced with `infill_combination = true` and `infill_combination_max_layer_height = "100%"` versus `infill_combination = false` (all else default), **when** their committed `SliceIR` vectors are compared, **then** the `true` run groups the sparse layers into one combined print (uppermost grouped layer carries the summed height metadata, lower grouped layers carry the void marker) and the `false` run matches the pre-packet baseline; capping the height at `"0.2"` (below 2× layer height) yields strictly fewer combined layers than `"100%"`. | `cargo test -p slicer-runtime --lib infill_combination_groups_sparse_layers 2>&1 | tee target/test-output.log | tail -5`
- **AC-5. Given** the same combination-`true` fixture, **when** the sparse emitter runs over the grouped layers, **then** the uppermost grouped layer emits sparse paths at the combined height (line count differs from the uncombined run) while wall roles emit at the original per-layer height (walls keep original height — canonical `combine_infill` wall exclusion). | `cargo test -p rectilinear-infill --lib combined_height_drives_sparse_only 2>&1 | tee target/test-output.log | tail -5`
- **AC-6. Given** the packet's disposition table and the deviation log, **when** they are read, **then** the table lists exactly the 4 P73 keys, all wired, with zero declaration-only keys, `DEV-190` names this packet's divergences in `docs/DEVIATION_LOG.md`, the regenerated `docs/15` tables contain the new keys, and the doc-15 freshness gate passes. | `rg -q 'declaration-only keys: 0' docs/spec_packets/299-object-level-shell-infill-planning/requirements.md && rg -q 'DEV-190' docs/DEVIATION_LOG.md && rg -q 'ensure_vertical_shell_thickness' docs/15_config_keys_reference.md && cargo xtask gen-config-docs --check 2>&1 | tee target/test-output.log | tail -5`

## Negative Test Cases

- **AC-N1. Given** `ensure_vertical_shell_thickness = "bogus_mode"`, **when** the config resolves, **then** resolution fails with a `TypeMismatch` error naming the key (strict-parse rejection — unknown enum strings never silently decay to a neighbouring mode). | `cargo test -p slicer-runtime --lib unknown_shell_mode_rejects 2>&1 | tee target/test-output.log | tail -5`
- **AC-N2. Given** a sloping fixture with locked infill paths (ADR-0062/0063 order locks on the sparse domain), **when** `ensure_vertical_shell_thickness = "ensure_all"` and `infill_combination = true` both engage, **then** the locked paths are byte-identical to the locks-off-shape run's locked subset (combination and shell growth neither clip nor merge locked footprints — the producer self-clipping obligation stays with the lock emitter). | `cargo test -p slicer-runtime --lib combination_and_shell_bypass_locked_paths 2>&1 | tee target/test-output.log | tail -5`

## Verification

- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test -p slicer-runtime --lib shell_classification 2>&1 | tee target/test-output.log | tail -5`

## Authoritative Docs

- `docs/01_system_architecture.md` - delegated SUMMARY (prepass vs module seam placement; rule-4 trigger test does not fire).
- `docs/08_coordinate_system.md` - direct range read (mm↔unit boundary helpers for shell-margin geometry).
- `docs/DEVIATION_LOG.md` - direct read of DEV-168/DEV-169 rows (tower-body and purge-volume scalar precedents) + ID-convention sample.
- `docs/03_wit_and_manifest.md` - delegated SUMMARY (manifest stanza shape; `ConfigView::from_declared` whitelist does not apply to host-prepass keys).

## Doc Impact Statement (Required)

- `docs/DEVIATION_LOG.md` section "DEV-190" - `rg -q 'DEV-190' docs/DEVIATION_LOG.md`.
- `docs/15_config_keys_reference.md` generated tables gain the 4 keys - `rg -q 'ensure_vertical_shell_thickness' docs/15_config_keys_reference.md`.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/PrintObject.cpp` — `discover_vertical_shells` shell-projection + regularization radii (what geometry is added per mode; `evstAll` gate).
- `OrcaSlicerDocumented/src/libslic3r/PrintObject.cpp` — `discover_horizontal_shells` margin branches (3× vs 1× solid-width gates; `evstAll` early-continue; `evstCriticalOnly`/`evstNone` search-stop) and `extra_solid_infills` insertion point.
- `OrcaSlicerDocumented/src/libslic3r/utils.cpp` — `check_layer_id_pattern` 1-based / `N` / `N#K` / comma-list matching (exact edge semantics for the pattern parser).
- `OrcaSlicerDocumented/src/libslic3r/PrintObject.cpp` — `combine_infill` grouping cap (`get_abs_value(nozzle_diameter)`), pattern-dependent clearance offsets, thickness/thickness_layers write-back, first-layer skip, wall-height exclusion.
- `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` — canonical defaults/shapes for all four keys (enum values, empty-string, bool false, 100%-percent).

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- obey the shared absolute context bands: 120k reading budget with hand-off at 150k (standard); the extended band (240k reading / 300k hard stop) only via swarm's escalation protocol

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation (an extended-band run may carry a single L step only when `design.md` justifies why it cannot be split).
