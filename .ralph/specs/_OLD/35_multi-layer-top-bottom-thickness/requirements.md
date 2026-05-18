# Requirements: multi-layer-top-bottom-thickness

## Packet Metadata

- Grouped task IDs:
  - `TASK-165` (NEW — to be added to `docs/07_implementation_status.md`)
- Backlog source: `docs/07_implementation_status.md`
- Packet status: `draft`
- Aggregate context cost: `M`

## Problem Statement

Packet `12-rev1` flags a region as `is_top_surface=true` only on the single layer immediately below a TopSurface facet (and symmetrically for bottom). Real-world prints need multiple solid layers to cap the top/bottom of an object — the codebase defaults to `top_shell_layers = 3` and `bottom_shell_layers = 3`. With only one layer flagged, top/bottom surfaces are too thin and may delaminate or show infill bleed.

The classifier algorithm from packet 12-rev1 is correct in shape; it just needs a wider Z window driven by per-region resolved config. This packet plumbs `RegionMapIR` (already on the blackboard, produced by `PrePass::RegionMapping`) through `execute_layer_slice` into the classifier so each region's window is computed from its own `top_shell_layers` and `bottom_shell_layers` keys.

The same `RegionMapIR` plumbing primitive will be reused by packets 36 (bridge config: anchor width, min length, expansion margin) and 37 (per-claim module selection). Doing the plumbing once here pays off three packets later.

## In Scope

- Extend `execute_layer_slice` signature with `region_map: Option<&RegionMapIR>`.
- Extend `classify_region_surfaces` signature to accept `top_shell_layers: u32, bottom_shell_layers: u32` (or compute them inline from `region_map`).
- Window construction: walk `LayerPlanIR.global_layers[layer_idx + 1 .. layer_idx + 1 + N]` for top, take the last Z (or `f32::INFINITY` if truncated). Symmetric for bottom.
- Per-region config lookup via `RegionMapIR.entries[(layer_idx, object_id, region_id)].config`.
- Default values: `top_shell_layers = 3`, `bottom_shell_layers = 3` (codebase default; deviates from Orca's 4/3 by design — see design.md deviation note).
- Production caller in `crates/slicer-host/src/layer_executor.rs:295-310` reads `blackboard.region_map()` and forwards to `execute_layer_slice`.
- New TDD `crates/slicer-host/tests/multi_layer_thickness_tdd.rs`.
- New Benchy E2E test asserting count of top/bottom blocks ≥ `top_shell_layers` / `bottom_shell_layers`.
- Mechanical updates to existing `execute_layer_slice` test callers to pass `None` for the new argument unless they're exercising multi-layer behavior.
- Config schema documentation update in `docs/02_ir_schemas.md` if `top_shell_layers` / `bottom_shell_layers` are not already declared.

## Out of Scope

- Bridge-detector parity (packet 36).
- Polygon-polygon overlap replacing centroid/any-vertex test (packet 36).
- Per-surface fill pattern variation (packet 37).
- Top-surface ironing (packet 38).
- Any change to dispatch, WIT, SDK, or scheduler claim system.
- Per-region overrides beyond what `RegionMapIR.entries[*].config` already supports.
- Variable layer height handling beyond what `LayerPlanIR.global_layers[*].z` already encodes (the Z-window walks the actual `z` values, not a synthetic `N × layer_height`).

## Authoritative Docs

- `docs/02_ir_schemas.md` — `RegionMapIR.entries[*].config` and resolved-config schema. Read directly; only the relevant section (search by symbol).
- `docs/04_host_scheduler.md` — § "RegionMapIR Compilation"; § "Per-Layer Execution"; § "Blackboard Structure". Document is > 600 lines: delegate SUMMARY ≤ 200 words for each section needed.
- `docs/03_wit_and_manifest.md` — config-key declaration rules. Delegate FACT confirming whether `top_shell_layers` / `bottom_shell_layers` are already declared.

## OrcaSlicer Reference Obligations

- `OrcaSlicerDocumented/src/libslic3r/PrintObject.cpp` — `discover_horizontal_shells()` defines the canonical multi-layer propagation (the algorithm we're mirroring at slice time). Delegate FACT confirming defaults and propagation direction.
- `OrcaSlicerDocumented/src/libslic3r/Print.hpp` — `PrintObject::process_external_surfaces()` declaration.

All OrcaSlicer reads MUST be delegated.

## Acceptance Summary

- Positive cases (see `packet.spec.md` §Acceptance Criteria):
  - Multi-layer window flags N layers (top and bottom).
  - Window truncates correctly at object extents.
  - Default value (3) applies when config absent.
  - `execute_layer_slice` honors `RegionMapIR` per-region config.
  - Benchy E2E count of top/bottom blocks scales with `top_shell_layers` / `bottom_shell_layers`.
- Negative cases:
  - `region_map: None` falls back to codebase defaults (`3, 3`), NOT to `1`.
  - `top_shell_layers = 0` disables the flag.
- Measurable outcomes:
  - All AC commands PASS.
  - `cargo test --workspace` PASS.
  - `cargo clippy --workspace -- -D warnings` PASS.
- Cross-packet impact:
  - Unblocks 36, 37 (RegionMap plumbing reuse).
  - Unblocks 38 (precise topmost-layer detection).

## Verification Commands

- `cargo test -p slicer-host --test multi_layer_thickness_tdd -- --nocapture`
- `cargo test -p slicer-host --test benchy_end_to_end_tdd benchy_multi_layer_top_bottom_evidence -- --nocapture`
- `cargo test --workspace`
- `cargo clippy --workspace -- -D warnings`

## Step Completion Expectations

For each step in `implementation-plan.md`:

- Precondition stated explicitly.
- Postcondition observable.
- Falsifying check: a single command that fails until the step is done.
- Files allowed to read with line ranges where > 300 lines.
- Files allowed to edit ≤ 3.
- Expected sub-agent dispatches.
- Step context cost: S or M (no L).

## Context Discipline Notes

- Large files in the read-only path:
  - `docs/04_host_scheduler.md` (> 600 lines) — delegate § RegionMapIR + § Per-Layer Execution + § Blackboard Structure summaries.
  - `crates/slicer-host/src/layer_executor.rs` — read only lines `280-360`.
- OrcaSlicer trees the implementer must NOT load directly: all of `OrcaSlicerDocumented/`.
- Likely temptation reads:
  - `crates/slicer-host/src/dispatch.rs` — out of scope; do not open.
  - `crates/slicer-host/src/region_mapping.rs` — read only the public `commit_region_mapping_builtin` signature; do not open file in full.
- Sub-agent return formats:
  - cargo runs → FACT pass/fail.
  - OrcaSlicer FACT delegations → one-line FACT each.
