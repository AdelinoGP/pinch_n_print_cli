---
status: draft
packet: 296-slicing-mode-prepass
task_ids: []
backlog_source: docs/specs/orca-feature-gap/issues/76-author-packet-p69-others-special-mode-layer-planner.md (wayfinder map: Close the OrcaSlicer FFF feature gap)
context_cost_estimate: M
copy_note: Queue packet from the wayfinder map "Close the OrcaSlicer FFF feature gap"; authored under ticket 76 (P69 remainder).
---

# Packet Contract: 296-slicing-mode-prepass

## Goal

Make the P69 remainder `slicing_mode` drive mesh-slice fill rule at parity with canonical `PrintObjectSlice` — `regular` / `even_odd` as the existing union path and `close_holes` as a new Positive branch — ported in the slicer-core prepass with no new module, IR field, WIT change, or manifest row.

## Scope Boundaries

P69 was two Tier B keys; `print_sequence` folds into ticket 124 (sequential printing feature — the mode has no meaning without the clearance validator and ByObject emission that ticket owns) and is not in this packet. The remainder is one live canonical key (`PrintObjectConfig` coEnum, default `regular`) consumed by the `PrintObjectSlice.cpp` slicing-mode switch into `MeshSlicingParams` fill rule. Claim-time grounding (ticket 76) holds it in: live in `libslic3r/`, zero occurrences as behaviour in this tree (no `crates/`/`modules/`/`xtask/` read, no `ORCA_CONFIG_PADDING` row, no prior packet). The tier table's `layer-planner` owner is corrected to the slicer-core prepass (ticket-27 hazard): this tree has no slicing behaviour in `layer-planner-default` (uniform layer heights only) — the union site is `slice_mesh_ex` (`crates/slicer-core/src/triangle_mesh_slicer.rs`) fed by `execute_prepass_slice_single_layer_impl` (`crates/slicer-core/src/algos/prepass_slice.rs`), so the mode lands there behind the existing `slice_closing_radius` plumbing. The packet declares one `ResolvedConfig` field (canonical default `regular`, per-object via the existing overlay), threads it through the region-map `config_for` read, and adds a mode-aware slice entry that keeps `slice_mesh_ex` byte-identical for existing callers. Canonical `regular` (NonZero) versus the port's always-EvenOdd union is a recorded simplification (DEV-188(a) — identical for valid manifold meshes per the `polygons_to_expolygons` doc comment); unknown-value rejection is enforced here where canonical only hints (DEV-188(b)).

## Prerequisites and Blockers

- Depends on: nothing. All symbols below are live on HEAD (verified at authoring); the slice union site, the prepass region-map read, and the `declare_resolved_config!` seam are landed tree code, not packet dependencies.
- Related work, not a blocker: ticket 124 (sequential printing — `print_sequence` folds there; no shared helper, no dep); ticket 125 (per-tool model — explicitly NOT this packet's axis: `slicing_mode` is per-object `PrintObjectConfig`, carried by the existing per-object overlay, not the tool axis); ticket 132 (CONFIG_BLOCK reader contract — canonical spelling rides there; this packet leaves the padding table untouched).
- Unblocks: wayfinder ticket 76 (P69 closes with `print_sequence` folded to 124 and `slicing_mode` in here). No edge to any other draft packet.
- Activation blockers: none. DEV-188 is the next collision-free ID (LOG max DEV-171; drafts claim DEV-172–DEV-187 — re-derive `max(DEV-*)` over `docs/DEVIATION_LOG.md` + `docs/spec_packets/*/` before writing the row).

## Acceptance Criteria

- **AC-1. Given** default config, **when** `ResolvedConfig` is inspected, **then** the `slicing_mode` key exists with canonical default `regular` (domain `regular`/`even_odd`/`close_holes`) and a `close_holes` config round-trips through `apply_cli_key` exactly. | `cargo test -p slicer-ir --test resolved_config_slicing_mode_tdd schema_declares_slicing_mode 2>&1 | tee target/test-output.log | tail -5`
- **AC-2. Given** default config (`regular`), **when** a valid manifold mesh (cube, annulus) is sliced, **then** the layer output is byte-identical to the pre-packet baseline, and `even_odd` slices the same mesh identically (regular/even_odd identity on valid meshes). | `cargo test -p slicer-core --test slicing_mode_fill_rule_tdd regular_and_even_odd_are_identity_on_valid_mesh 2>&1 | tee target/test-output.log | tail -5`
- **AC-3. Given** `slicing_mode = close_holes` over an annulus mesh (outer contour + hole), **when** the mode-aware slice entry runs, **then** every emitted `ExPolygon` carries zero holes and the filled area equals the outer-contour area within 1% (holes closed, islands stay solid). | `cargo test -p slicer-core --test slicing_mode_fill_rule_tdd close_holes_fills_annulus_hole 2>&1 | tee target/test-output.log | tail -5`
- **AC-4. Given** a region-map config with `slicing_mode = close_holes` over the same annulus object, **when** `execute_prepass_slice_single_layer_impl` runs, **then** the resulting `SliceIR` carries zero holes where the `regular` config carries one (region-map value reaches the slice, not just the unit kernel). | `cargo test -p slicer-core --test algo_prepass_slice_tdd slicing_mode_close_holes_changes_slice_ir 2>&1 | tee target/test-output.log | tail -5`

## Negative Test Cases

- **AC-N1. Given** `slicing_mode = diagonal`, **when** config resolution runs, **then** resolution rejects with a `TypeMismatch`-family error naming the key (unknown enum values never fall through; canonical never enforces — DEV-188(b)). | `cargo test -p slicer-ir --test resolved_config_slicing_mode_tdd unknown_slicing_mode_rejected 2>&1 | tee target/test-output.log | tail -5`
- **AC-N2. Given** the authored tree, **when** the CONFIG_BLOCK padding table is inspected, **then** no `slicing_mode` row exists in `ORCA_CONFIG_PADDING` and the table is untouched by this packet (host-only omission per the packet-287/288 precedent — per-object slicing directives need no reader-visible spelling; canonical spelling rides ticket 132). | `rg -q 'slicing_mode' crates/slicer-gcode/src/serialize.rs && exit 1 || exit 0`

## Verification

- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test -p slicer-core --test slicing_mode_fill_rule_tdd 2>&1 | tee target/test-output.log | tail -5`

## Authoritative Docs

- `docs/00_project_overview.md` - delegated SUMMARY (modular pipeline + community extensibility constraint on the prepass-seam shape)
- `docs/01_system_architecture.md` - delegated SUMMARY (prepass ownership only — the mode parameterises the existing slice stage; rule 4 trigger test does not fire: in-stage fill-rule parameter, not cross-module algorithm selection)
- `docs/03_wit_and_manifest.md` - delegated SUMMARY (`from_declared` whitelist section only — inapplicable by design: slicer-core reads `ResolvedConfig` via region-map, so no manifest row is declared; ticket-34 lesson cited as the reason no manifest work exists)

## Doc Impact Statement (Required)

- `docs/DEVIATION_LOG.md` section "DEV-188" - `rg -q 'DEV-188' docs/DEVIATION_LOG.md` (filed in Step 4 with (a)+(b) clauses)
- `docs/specs/orca-feature-gap/issues/04-asset-tier-assignment.md` section "Others / Special mode" - `rg -q 'slicing_mode.*slicer-core' docs/specs/orca-feature-gap/issues/04-asset-tier-assignment.md` (Step 4 owner correction + P69 2→1 fold note)
- `docs/specs/orca-feature-gap/issues/05-asset-packet-list.md` section "P69" - `rg -q '296-slicing-mode' docs/specs/orca-feature-gap/issues/05-asset-packet-list.md` (Step 4: 1 key in at packet 296, `print_sequence` folded to 124)

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` — `PrintConfigDef::init_fff_params` (the `slicing_mode` coEnum declaration with `regular`/`even_odd`/`close_holes` and default `Regular`; borrow the default and domain exactly)
- `OrcaSlicerDocumented/src/libslic3r/PrintObjectSlice.cpp` — slicing-mode switch (borrow the Regular→Regular / EvenOdd→EvenOdd / CloseHoles→Positive mapping; port CloseHoles as the Positive branch)
- `OrcaSlicerDocumented/src/libslic3r/TriangleMeshSlicer.hpp` — `MeshSlicingParams::SlicingMode` (borrow the Regular/EvenOdd/Positive/PositiveLargestContour domain shape; PositiveLargestContour is spiral-only and explicitly not borrowed)
- `OrcaSlicerDocumented/src/libslic3r/Preset.cpp` — preset-plumbing entries (named non-borrow: GUI/preset routing, not slicing behaviour)

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- obey the shared absolute context bands: 120k reading budget with hand-off at 150k (standard); the extended band (240k reading / 300k hard stop) only via swarm's escalation protocol

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation (an extended-band run may carry a single L step only when `design.md` justifies why it cannot be split).
