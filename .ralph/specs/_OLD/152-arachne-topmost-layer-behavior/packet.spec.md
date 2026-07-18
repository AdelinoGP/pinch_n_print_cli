---
status: implemented
packet: 152-arachne-topmost-layer-behavior
task_ids:
  - none
backlog_source: docs/18_arachne_parity_audit.md
context_cost_estimate: M
---

# Packet Contract: 152-arachne-topmost-layer-behavior

# Goal

Port OrcaSlicer's topmost-layer Arachne behavior — `only_one_wall_top` (single
wall on the topmost layer plus the second `WallToolPaths` pass with `inset_idx`
renumbering for non-topmost top surfaces) and the `removeSmallLines` top/bottom
layer exception (a distinct top-layer flag on the `arachne-params` WIT record) —
flipping gap tests G3 and G10.

## Scope Boundaries

Extends the `arachne-params` WIT record (`common.wit`) with `is-bottom-layer` /
`is-topmost-layer` bools, threads them through the SDK bridge
(`slicer-sdk/src/host.rs` — both its native-path and WIT-path conversions) AND
the host-side service impl (`slicer-wasm-host/src/host.rs` field-by-field
mapping) into `run_arachne_pipeline`, and reworks `remove_small_lines` to key the lenient
threshold on top-OR-bottom rather than layer 0. In the module, implements the
`only_one_wall_top` topmost single-wall force and the full second-pass top-surface
generation (top-area derivation, `min_width_top_surface` filter using packet
150's percent resolution, inset renumbering, merge). This is the only WIT-contract
change of the three packets; it is isolated here.

## Prerequisites and Blockers

- Depends on: packet 150 (`min_width_top_surface` resolved via `get_abs_value`)
  and packet 151 (correct first-pass `wall_count` baseline the reduction and
  renumbering build on).
- Unblocks: closes the last of gaps G1–G10 (G11 excluded).
- Activation blockers: packets 150 and 151 both `status: implemented`; only one
  packet active at a time.

## Acceptance Criteria

Acceptance Criteria are stated **once**, here.

- **AC-1 (G3 topmost single wall). Given** `only_one_wall_top=true`,
  `wall_count=3`, and a region marked topmost (`top_shell_index == Some(0)`),
  **when** `arachne-perimeters` runs, **then** it emits exactly one distinct
  perimeter index (`{0}`) — Orca forces `loop_number = 0` on the topmost layer
  (`upper_slices == nullptr`). |
  `cargo test -p slicer-runtime --test arachne_parity_gaps -- arachne_parity_arachne_path_only_one_wall_top_forces_single_wall_on_top --exact`
- **AC-2 (G3 second pass). Given** `only_one_wall_top=true`, `wall_count=3`, a
  NON-topmost region whose top surface is a sub-area (an upper slice partly
  covers it), **when** `arachne-perimeters` runs, **then** the top sub-area emits
  a single wall while the non-top remainder emits the inner walls via a second
  `WallToolPaths` pass, and merged inner-wall `inset_idx` values are incremented
  by 1 (renumbered) relative to a naive single-pass run. Packet-authored test in
  a new `tests/only_one_wall_top_tdd.rs` (module tests/ dir — follows the 10
  existing native `*_tdd.rs` files that drive `ArachnePerimeters::run_perimeters`
  via `slicer_sdk::traits::LayerModule`; standalone file ⇒ auto-registered, no
  aggregator). NOT `--lib`: the only in-file test module is the param-level unit
  mod packet 151 authors — a `--lib` filter for this name would match nothing
  and false-pass with "0 tests run". |
  `cargo test -p arachne-perimeters --test only_one_wall_top_tdd -- only_one_wall_top_second_pass --exact --nocapture`
- **AC-3 (G10 top-layer exception). Given** a 3 mm odd unclosed center line on
  the TOPMOST layer (`is_topmost_layer=true`, `is_initial_layer=false`), **when**
  `remove_small_lines` runs, **then** the line survives via the lenient
  `min_width/2` (0.2 mm) threshold — not dropped by the strict
  `min_width·min_length_factor` (8 mm) threshold. |
  `cargo test -p slicer-runtime --test arachne_parity_gaps -- arachne_parity_arachne_path_remove_small_lines_top_layer_exception --exact`
- **AC-4 (WIT record extended). Given** the `arachne-params` WIT record, **when**
  it is inspected, **then** it carries `is-bottom-layer` and `is-topmost-layer`
  bool fields (alongside the existing `is-initial-layer`), mirrored in the Rust
  `ArachneParams` struct and set by the module from region top/bottom metadata. |
  `rg -q 'is-topmost-layer' crates/slicer-schema/wit/deps/common.wit`
- **AC-5 (regression lock). Given** the 15 `arachne_parity.rs` locks and the
  green G-tests from packets 150/151, **when** the topmost behavior lands,
  **then** all stay green; the
  `arachne_parity_pipeline_only_one_wall_top_vs_min_width_top_surface` lock
  (`arachne_parity.rs:591`, probes the module source via `include_str!` for the
  string `only_one_wall_top`) remains satisfied. |
  `cargo test -p slicer-runtime --test arachne_parity`

## Negative Test Cases

- **AC-N1 (non-top layer unaffected). Given** `is_topmost_layer=false` and
  `is_bottom_layer=false` (a normal mid-stack layer), **when** `remove_small_lines`
  runs on a short odd line, **then** the strict threshold applies and the line is
  dropped — the lenient threshold fires ONLY on top/bottom layers. Packet-authored
  test in a NEW `#[cfg(test)] mod tests` inside
  `crates/slicer-core/src/arachne/remove_small.rs` (the file has no test module
  today; without authoring it, a `--lib` filter matches nothing and false-passes). |
  `cargo test -p slicer-core --lib -- arachne::remove_small::tests::non_top_layer_strict --exact --nocapture`
- **AC-N2 (only_one_wall_top off). Given** `only_one_wall_top=false` on a topmost
  region, **when** `arachne-perimeters` runs, **then** the full `wall_count` walls
  are emitted — the single-wall force fires only when the key is on. Lives in the
  same packet-authored `tests/only_one_wall_top_tdd.rs` as AC-2's test. |
  `cargo test -p arachne-perimeters --test only_one_wall_top_tdd -- only_one_wall_top_disabled --exact --nocapture`

## Verification

- `cargo xtask build-guests --check` (must be clean — this packet edits
  `common.wit` and rebuilds guests; run after every WIT/SDK edit)
- `cargo check --workspace --all-targets` and
  `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test -p slicer-runtime --test arachne_parity_gaps -- arachne_parity_arachne_path_only_one_wall_top_forces_single_wall_on_top arachne_parity_arachne_path_remove_small_lines_top_layer_exception`

## Authoritative Docs

- `docs/03_wit_and_manifest.md` — WIT world + host-boundary enforcement for the
  `arachne-params` record change; delegate the relevant section.
- `docs/02_ir_schemas.md` — `SliceRegionView` top-shell metadata
  (`top_shell_index`, `top_solid_fill`); delegate the region section.
- `docs/08_coordinate_system.md` — unit conversions for thresholds (load).
- `docs/DEVIATION_LOG.md` — `D-104d-MIN-WIDTH-TOP-SURFACE-NONE`
  (`DEVIATION_LOG.md:82`) — NOTE its scope covers BOTH modules' deferred
  `min_width_top_surface` behavior; this packet lands only the arachne half
  (see Doc Impact for the narrowing protocol). The classic-perimeters
  remainder stays open.

## Doc Impact Statement (Required)

Changes a WIT contract and module behavior, so `none` is not eligible:

- `docs/03_wit_and_manifest.md` §"arachne-params" — document the two new bool
  fields — `rg -q 'is-topmost-layer' docs/03_wit_and_manifest.md`
- `docs/15_config_keys_reference.md` — annotate the existing `only_one_wall_top`
  (`:59`) and `min_width_top_surface` (`:58`) rows as behavioral as of P152.
  NOTE: both keys ALREADY appear in this doc, so a bare key-name grep is
  pre-satisfied today and verifies nothing — the grep below tests the new
  annotation — `rg -q 'P152' docs/15_config_keys_reference.md`
- `docs/DEVIATION_LOG.md` — narrow `D-104d-MIN-WIDTH-TOP-SURFACE-NONE`: mark the
  arachne-perimeters half landed (P152) and split the classic-perimeters
  remainder into a successor entry `D-152-CLASSIC-MIN-WIDTH-TOP-SURFACE-REMAINDER`
  (do NOT mark the whole entry closed — its scope includes classic-perimeters,
  which this packet does not touch) —
  `rg -q 'D-104d-MIN-WIDTH-TOP-SURFACE-NONE.*P152' docs/DEVIATION_LOG.md` and
  `rg -q 'D-152-CLASSIC-MIN-WIDTH-TOP-SURFACE-REMAINDER' docs/DEVIATION_LOG.md`
- `docs/18_arachne_parity_audit.md` — mark G3 (row `:178`) and G10 (row `:185`)
  closed in the PnP-status column, following the file's existing `… closed`
  style — `rg -q 'G10.*closed' docs/18_arachne_parity_audit.md`

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/PerimeterGenerator.cpp:2140-2144` — topmost
  layer forces `loop_number = 0` (G3 part 1).
- `OrcaSlicerDocumented/src/libslic3r/PerimeterGenerator.cpp:2160-2246` — second
  `WallToolPaths` pass: top-area derivation, bridge exclusion,
  `min_width_top_surface` filter, `offset2_ex`, `inset_idx` renumbering, merge,
  empty-top fallback rerun (G3 part 2).
- `OrcaSlicerDocumented/src/libslic3r/Arachne/WallToolPaths.cpp:684-700` —
  `removeSmallLines` lenient `min_width/2` on top/bottom layers (G10).
- `OrcaSlicerDocumented/src/libslic3r/PerimeterGenerator.cpp:2153-2154` —
  `is_top_or_bottom_layer = is_bottom_layer || is_topmost_layer` (G10 flag).

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- stop reading at 60% context and hand off at 85%

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation.
