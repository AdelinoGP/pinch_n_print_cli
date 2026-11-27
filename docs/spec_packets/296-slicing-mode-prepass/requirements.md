# Requirements: 296-slicing-mode-prepass

## Packet Metadata

- Grouped task IDs: `TASK-000`
- Backlog source: `docs/specs/orca-feature-gap/issues/76-author-packet-p69-others-special-mode-layer-planner.md`
- Packet status: `draft`
- Aggregate context cost: `M` (never L)

## Problem Statement

P69 pairs two unrelated keys under one `layer-planner` owner: `print_sequence` (a global print-mode selector whose only slicing meaning is the sequential-printing validator) and `slicing_mode` (a per-object mesh-slice fill rule). Ticket 32 already showed the first half cannot close here — the clearance validator does not exist and ticket 124 owns the feature — and ticket 76 folds it there. What remains is a real one-key gap: canonical closes holes and toggles winding rules from this key, while this port always unions EvenOdd and has no spelling for the key at all. The tier table's owner is wrong for this tree (no slicing behaviour lives in the layer-planner module), so the packet corrects it to the slicer-core prepass where the union site and the region-map config read already live.

## In Scope

- `slicing_mode` (canonical `PrintObjectConfig` coEnum `regular`/`even_odd`/`close_holes`, default `regular`): one `ResolvedConfig` field with per-object overlay, threading through the region-map `config_for` read into a mode-aware slice entry; `regular`/`even_odd` share the existing EvenOdd union path (identity on valid meshes), `close_holes` adds the Positive branch (all rings CCW, `FillRule::Positive` union).
- Owner correction: `layer-planner` → slicer-core prepass (`crates/slicer-core` slice kernel + `crates/slicer-ir` config surface); no manifest row anywhere.
- Deviation row DEV-188 ((a) Regular-as-EvenOdd simplification, (b) strict unknown-value rejection); 04/05 annotations (P69 2→1, `print_sequence` folded to 124).

## Out of Scope

- `print_sequence` (folds to ticket 124; no declaration, no behaviour, no CONFIG_BLOCK work here).
- Spiral-vase `PositiveLargestContour` mode (spiral packets own it; named non-borrow).
- `slice_closing_radius` interaction beyond sequencing after it (the closing round-trip runs after the mode union, unchanged).
- CONFIG_BLOCK spelling for `slicing_mode` (honest absence; canonical value spelling rides ticket 132; padding table untouched).
- Per-tool axis (ticket 125): this key is per-object, carried by the existing overlay — no tool surface.

## Authoritative Docs

- `docs/00_project_overview.md` - delegated SUMMARY (prepass-seam constraint only).
- `docs/01_system_architecture.md` - delegated SUMMARY (prepass ownership; rule 4 does not fire).
- `docs/03_wit_and_manifest.md` - delegated SUMMARY (`from_declared` whitelist inapplicability).

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` — `PrintConfigDef::init_fff_params` (borrow `slicing_mode` default + domain exactly)
- `OrcaSlicerDocumented/src/libslic3r/PrintObjectSlice.cpp` — slicing-mode switch (borrow the three-way mapping; spiral `PositiveLargestContour` arm named non-borrow)
- `OrcaSlicerDocumented/src/libslic3r/TriangleMeshSlicer.hpp` — `MeshSlicingParams::SlicingMode` (borrow the domain shape; `slicing_mode_normal_below_layer` is spiral-only, not borrowed)
- `OrcaSlicerDocumented/src/libslic3r/Preset.cpp` — preset plumbing (named non-borrow)

## Acceptance Summary

Reference, never copy, criteria from `packet.spec.md`.

- Positive: `AC-1` through `AC-4`; no measurable refinements beyond their Given/When/Then text.
- Negative: `AC-N1` through `AC-N2`.
- Cross-packet impact: none. Ticket 124 gains `print_sequence` by fold (no code edge); ticket 132 owns the reader-visible spelling (no edge); ticket 125 is explicitly not this axis.

## Verification Commands

This is the authoritative full matrix; `packet.spec.md` lists only 2-3 gate commands.

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo test -p slicer-ir --test resolved_config_slicing_mode_tdd 2>&1 \| tee target/test-output.log \| tail -5` | config surface: default, round-trip, rejection | FACT pass/fail; SNIPPETS <=20 lines on failure |
| `cargo test -p slicer-core --test slicing_mode_fill_rule_tdd 2>&1 \| tee target/test-output.log \| tail -5` | fill-rule kernel: identity + close-holes | FACT pass/fail; SNIPPETS <=20 lines on failure |
| `cargo test -p slicer-core --test algo_prepass_slice_tdd slicing_mode 2>&1 \| tee target/test-output.log \| tail -5` | prepass wiring: region-map value reaches slice | FACT pass/fail; SNIPPETS <=20 lines on failure |
| `rg -q 'slicing_mode' crates/slicer-gcode/src/serialize.rs && exit 1 \|\| exit 0` | AC-N2 honest absence | FACT pass/fail |
| `cargo check --workspace --all-targets` | closure gate | FACT pass/fail |
| `cargo clippy --workspace --all-targets -- -D warnings` | closure gate | FACT pass/fail |

## Step Completion Expectations

Step 2 (kernel) must land before Step 3 (wiring): the prepass test asserts on the kernel's CloseHoles observable, so a wiring-first order would assert on a stub. DEV-188, 04, and 05 edits land together in Step 4 with the re-derived IDs (never frozen from this file).

## Context Discipline Notes

Packet-specific hazards: `triangle_mesh_slicer.rs` and `prepass_slice.rs` are both over 300 lines — read only the ranges `design.md` names. `slice_mesh_ex` has five production callers; only the prepass call site is in scope — delegate caller enumeration rather than browsing. New test binaries are small; existing `algo_prepass_slice_tdd.rs` is large — filter by `slicing_mode` test name.
