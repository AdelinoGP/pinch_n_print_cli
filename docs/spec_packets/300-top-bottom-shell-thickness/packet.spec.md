---
status: draft
packet: 300-top-bottom-shell-thickness
task_ids: []
backlog_source: docs/specs/orca-feature-gap/issues/81-author-packet-p74-strength-top-bottom-shells-object-level-planning.md (wayfinder map: Close the OrcaSlicer FFF feature gap)
context_cost_estimate: M
copy_note: Queue packet from the wayfinder map "Close the OrcaSlicer FFF feature gap"; authored under ticket 81 (P74).
---

# Packet Contract: 300-top-bottom-shell-thickness

## Goal

Make the 2 P74 shell-thickness keys drive the tree pipeline at canonical value semantics — the solid-layer count floor grows until the printed shell reaches the configured mm thickness — with zero declaration-only keys and byte-identical defaults.

## Scope Boundaries

P74 is two Tier B keys owned by object-level planning (`PrintObject.cpp` in canonical: the `discover_horizontal_shells` top/bottom projection loops), both live in canonical's slicing pipeline and zero-occurrence as behaviour in this tree. Claim-time grounding (ticket 81) holds both in: no live decision point exists (the tree's `resolve_shell_counts` reads only the layer-count keys), no draft packet owns the decision (packet 299's P73 stages own vertical shells, extra-solid insertion, and sparse combination — not the thickness-vs-count floor), and the tier-table owner is accurate but narrowed for this tree — the seam is the host prepass `commit_shell_classification_builtin`, never a module. The packet wires both through two new `ResolvedConfig` fields (per-object via the existing overlay, explicitly not ticket 125's tool axis): the thickness pair extends the count pair's projection walks with the canonical `||` thickness arm. Rule 4 does not fire: the arms branch inside one prepass seam, not cross-module algorithm selection — there are no claim holders to create, mirroring the `seam_position` precedent in the map Notes.

## Prerequisites and Blockers

- Depends on: nothing. All symbols below are live on HEAD (verified at authoring); `commit_shell_classification_builtin`, `resolve_shell_counts`, `compute_region_updates`, `convert_small_sparse_islands`, the `declare_resolved_config!` macro, and `region_map.config_for` are landed tree code, not packet dependencies. Exact canonical formulas the wiring mirrors are borrowed via implementer-side delegated oracle reads (design.md lists them).
- Related work, not a blocker: draft packet 299 (P73 — the same prepass seam gains three stages there; this packet's two arms ride the same `resolve_shell_counts` entry and must merge with — not duplicate — its call signature; sequencing is file-merge order, not an activation dep); ticket 122 (prime-tower body — no shared helper, no dep); ticket 124 (sequential-printing validator — the `Print.cpp` reslice-invalidation entries ride it, named non-borrow); ticket 125 (per-tool model — explicitly NOT this packet's axis: both keys are canonical scalars, carried by the existing per-object overlay, 290/291/296 precedent); ticket 132 (CONFIG_BLOCK reader contract — canonical spellings ride there; no padding edits here, rule 2); packet 264 (top-bottom surfaces — no shared helper, no dep).
- Unblocks: wayfinder ticket 81 (P74 closes when this packet is authored). No edge to any other draft packet (no draft packet mentions either key; the `shell_thickness` substring hits in 299 are the P73 `ensure_vertical_shell_thickness` mode key — a different decision, not ownership).
- Activation blockers: none. One deviation ID is minted (DEV-191, verified absent from the log and all drafts at authoring); if implementation surfaces another, re-derive `max(DEV-*)` over `docs/DEVIATION_LOG.md` + `docs/spec_packets/*/` before writing the row.

## Acceptance Criteria

- **AC-1. Given** `crates/slicer-ir/src/resolved_config.rs`, **when** the `declare_resolved_config!` invocation is inspected, **then** it declares `top_shell_thickness` (float, default `0.6`) and `bottom_shell_thickness` (float, default `0.0`), each with a `cli` binding of the same snake_case name. | `rg -q 'cli "top_shell_thickness"' crates/slicer-ir/src/resolved_config.rs && rg -q 'cli "bottom_shell_thickness"' crates/slicer-ir/src/resolved_config.rs 2>&1 | tee target/test-output.log | tail -3`
- **AC-2. Given** a 6-layer uniform 0.2 mm box fixture with `top_shell_layers = 1` and `bottom_shell_layers = 1`, sliced with `top_shell_thickness = 0.6` / `bottom_shell_thickness = 0.6` versus both `0.0` (all else default), **when** their committed `SliceIR` vectors are compared, **then** the `0.6` run stamps strictly more layers with `top_shell_index` / `bottom_shell_index` than the `0.0` run (the projection keeps walking past the count floor until the mm thickness is reached), and the `0.0` run matches the pre-packet baseline (thickness-disabled identity). | `cargo test -p slicer-runtime --lib shell_thickness_extends_projection_past_count 2>&1 | tee target/test-output.log | tail -5`
- **AC-3. Given** a 6-layer uniform 0.2 mm box fixture sliced with defaults unset and default layer counts (`top_shell_layers = 3`, `bottom_shell_layers = 3`, all else default), **when** the committed `SliceIR` is compared to the pre-packet baseline, **then** it is byte-identical (defaults are identity: 3 count-layers at 0.2 mm already cover exactly 0.6 mm, so the top arm's `EPSILON` margin admits no fourth layer; bottom `0.0` is disabled). | `cargo test -p slicer-runtime --lib shell_thickness_defaults_are_identity 2>&1 | tee target/test-output.log | tail -5`
- **AC-4. Given** the packet's disposition table and the deviation log, **when** they are read, **then** the table lists exactly the 2 P74 keys, all wired, with zero declaration-only keys, `DEV-191` names this packet's divergences in `docs/DEVIATION_LOG.md`, the regenerated `docs/15` tables contain the new keys, and the doc-15 freshness gate passes. | `rg -q 'declaration-only keys: 0' docs/spec_packets/300-top-bottom-shell-thickness/requirements.md && rg -q 'DEV-191' docs/DEVIATION_LOG.md && rg -q 'top_shell_thickness' docs/15_config_keys_reference.md && cargo xtask gen-config-docs --check 2>&1 | tee target/test-output.log | tail -5`

## Negative Test Cases

- **AC-N1. Given** a sloping fixture with locked infill paths (ADR-0062/0063 order locks on the sparse domain), **when** `top_shell_thickness = 2.0` engages the extended projection, **then** the locked paths are byte-identical to the locks-off-shape run's locked subset (thickness growth neither clips nor merges locked footprints — the producer self-clipping obligation stays with the lock emitter). | `cargo test -p slicer-runtime --lib shell_thickness_bypasses_locked_paths 2>&1 | tee target/test-output.log | tail -5`

## Verification

- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test -p slicer-runtime --lib shell_classification 2>&1 | tee target/test-output.log | tail -5`

## Authoritative Docs

- `docs/01_system_architecture.md` - delegated SUMMARY (prepass vs module seam placement; rule-4 trigger test does not fire).
- `docs/08_coordinate_system.md` - direct range read (mm↔unit boundary helpers for the thickness-vs-print_z comparison).
- `docs/DEVIATION_LOG.md` - direct read of DEV-168/169/171 scalar-vs-vector precedents + ID-convention sample.
- `docs/03_wit_and_manifest.md` - delegated SUMMARY (manifest stanza shape; `ConfigView::from_declared` whitelist does not apply to host-prepass keys).

## Doc Impact Statement (Required)

- `docs/DEVIATION_LOG.md` section "DEV-191" - `rg -q 'DEV-191' docs/DEVIATION_LOG.md`.
- `docs/15_config_keys_reference.md` generated tables gain the 2 keys - `rg -q 'top_shell_thickness' docs/15_config_keys_reference.md`.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/PrintObject.cpp` — `discover_horizontal_shells` top/bottom projection loops (the count-floor `i < itop` / `i > ibottom` arms plus the `||` thickness arms against `print_z` / `bottom_z` with the `EPSILON` margin, and the two `combine_holes` follow-ups under the `one_more_layer_below_top_bottom_surfaces = false` flag).
- `OrcaSlicerDocumented/src/libslic3r/PrintObject.cpp` — `discover_vertical_shells` first/last-layer top/bottom projection loops (the `i < itop || print_z-distance < top_shell_thickness` and `i > ibottom || bottom_z-distance < bottom_shell_thickness` arms).
- `OrcaSlicerDocumented/src/libslic3r/PrintObject.cpp` — `PrintObject::infill` scatter loops (the `int(i) - n < num_solid_layers || print_z-distance < top_shell_thickness` top arm and the `n - int(i) < num_solid_layers || bottom_z-distance < bottom_shell_thickness` bottom arm).
- `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` — canonical defaults/shapes for both keys (coFloat, top `0.6`, bottom `0.0`, min 0).

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- obey the shared absolute context bands: 120k reading budget with hand-off at 150k (standard); the extended band (240k reading / 300k hard stop) only via swarm's escalation protocol

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation (an extended-band run may carry a single L step only when `design.md` justifies why it cannot be split).
