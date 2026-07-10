# Requirements: 152-arachne-topmost-layer-behavior

## Packet Metadata

- Grouped task IDs:
  - `none` (audit-driven; backlog `docs/18_arachne_parity_audit.md`)
- Backlog source: `docs/18_arachne_parity_audit.md`
- Packet status: `draft`
- Aggregate context cost: `M`

## Problem Statement

Two Arachne parity gaps concern the topmost printed layer, and both need a
"topmost layer" signal the pipeline cannot currently express:

- **G3 `only_one_wall_top` (consumes the `min_width_top_surface` behavior
  deferred under `D-104d-MIN-WIDTH-TOP-SURFACE-NONE`):** OrcaSlicer forces a single wall on the
  topmost layer (`loop_number = 0` when `upper_slices == nullptr`) and, for
  NON-topmost layers whose surface is partly a top surface, runs a SECOND
  `Arachne::WallToolPaths` pass over the non-top sub-area with `inner_loop_number
  + 1` walls, merging with `inset_idx` renumbering (`PerimeterGenerator.cpp:2140-
  2246`). `arachne-perimeters` reads `only_one_wall_top` and immediately discards
  it (`src/lib.rs:305-306`); it reads no `top_shell_index`/`top_solid_fill`, so a
  top region gets the full wall count.
- **G10 `removeSmallLines` top exception:** Orca keeps short odd unclosed walls on
  top OR bottom layers via a lenient `min_width/2` threshold
  (`is_top_or_bottom_layer = is_bottom_layer || is_topmost_layer`,
  `WallToolPaths.cpp:684-700`). PnP's `remove_small_lines` keys the lenient
  threshold on `is_initial_layer` (layer 0) ONLY; neither it nor
  `run_arachne_pipeline` can express "topmost layer", so top-surface thin walls
  are dropped and top surfaces show gaps.

They are one slice because both require the same new plumbing: a top/topmost-layer
signal threaded from the module through the `generate-arachne-walls` host service
(the `arachne-params` WIT record) into `run_arachne_pipeline` and
`remove_small_lines`. This is the only `common.wit` change of the three packets,
deliberately isolated here to contain guest-rebuild risk.

## In Scope

- Extend the `arachne-params` WIT record (`crates/slicer-schema/wit/deps/common.wit:26-50`)
  with `is-bottom-layer` and `is-topmost-layer` bools; mirror in the Rust
  `ArachneParams` (`crates/slicer-core/src/arachne/pipeline.rs`) and set them in
  the module from region top/bottom metadata. The record↔struct conversions that
  must gain the two fields are: `crates/slicer-sdk/src/host.rs` (`:551` native
  path, `:690` WIT path — the adapter lives HERE, not in `slicer-macros`, which
  has zero `ArachneParams` code) and `crates/slicer-wasm-host/src/host.rs`
  (`:1773-1794` host-side field-by-field mapping).
- Change `remove_small_lines` to key the lenient `min_width/2` threshold on
  `is_bottom_layer || is_topmost_layer` (retaining or subsuming `is_initial_layer`
  after auditing its other consumers); thread the flags through
  `run_arachne_pipeline`.
- Detect the topmost region in the module via `SliceRegionView::top_shell_index`
  (no such read exists today); implement G3 part 1 (topmost single wall) and
  part 2 (the second `WallToolPaths` pass: top-area derivation, bridge exclusion,
  `min_width_top_surface` filter using packet-150 `get_abs_value` resolution,
  `offset2_ex` shrink/expand, second pass over non-top area, `inset_idx += 1`
  renumbering, merge, empty-top fallback rerun).
- Author locking tests for the G3 second pass (not covered by the red test).
- Narrow `D-104d-MIN-WIDTH-TOP-SURFACE-NONE` (arachne half lands here; split the
  classic-perimeters remainder into
  `D-152-CLASSIC-MIN-WIDTH-TOP-SURFACE-REMAINDER`); update docs/03 (WIT record),
  docs/15, docs/18.

## Out of Scope

- Flow/percent, winding, wall_count, spiral — packets 150/151.
- `classic-perimeters`' `min_width_top_surface` threshold behavior (the other
  half of `D-104d-MIN-WIDTH-TOP-SURFACE-NONE`; read-and-discarded at
  `classic-perimeters/src/lib.rs:224-239`) — split into the successor deviation,
  not implemented here.
- `interface_shells` per-region upper-slice handling beyond what the topmost
  detection needs (Orca has an `interface_shells` branch at `:2190`; port only if
  a locking test requires it — otherwise record as a follow-up deviation).

## Authoritative Docs

- `docs/03_wit_and_manifest.md` — WIT world + host-boundary enforcement; delegate
  the `arachne-params`/host-service section.
- `docs/02_ir_schemas.md` — `SliceRegionView` top-shell metadata; delegate.
- `docs/08_coordinate_system.md` — short; load for offset unit conversions.
- `docs/DEVIATION_LOG.md` — `D-104d-MIN-WIDTH-TOP-SURFACE-NONE` entry (`:82`;
  covers both modules — only the arachne half lands here).

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/PerimeterGenerator.cpp:2140-2144` — topmost single wall (G3 part 1).
- `OrcaSlicerDocumented/src/libslic3r/PerimeterGenerator.cpp:2160-2246` — second pass + renumbering (G3 part 2).
- `OrcaSlicerDocumented/src/libslic3r/Arachne/WallToolPaths.cpp:684-700` — removeSmallLines lenient threshold (G10).
- `OrcaSlicerDocumented/src/libslic3r/PerimeterGenerator.cpp:2153-2154` — `is_top_or_bottom_layer` flag derivation (G10).

## Acceptance Summary

- Positive: `AC-1` (G3 topmost single wall via `top_shell_index`),
  `AC-2` (G3 second pass: single top wall + renumbered inner walls),
  `AC-3` (G10 topmost thin-wall survival), `AC-4` (WIT record carries the two
  new bools), `AC-5` (15 locks + 150/151 gap tests stay green, incl. the
  `arachne_parity_pipeline_only_one_wall_top_vs_min_width_top_surface`
  source-read lock).
- Negative: `AC-N1` (mid-stack layer still drops short odd lines — lenient
  threshold is top/bottom only), `AC-N2` (`only_one_wall_top=false` emits full
  wall count on a topmost region).
- Cross-packet: depends on 150 (`get_abs_value`) + 151 (wall_count baseline);
  final packet of G1–G10.

## Verification Commands

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo test -p slicer-runtime --test arachne_parity_gaps -- arachne_parity_arachne_path_only_one_wall_top_forces_single_wall_on_top --exact` | G3 part 1 | FACT pass/fail |
| `cargo test -p arachne-perimeters --test only_one_wall_top_tdd -- only_one_wall_top_second_pass --exact` | AC-2 second pass (packet-authored `tests/only_one_wall_top_tdd.rs`) | FACT pass/fail; SNIPPETS on fail |
| `cargo test -p slicer-runtime --test arachne_parity_gaps -- arachne_parity_arachne_path_remove_small_lines_top_layer_exception --exact` | G10 | FACT pass/fail |
| `rg -q 'is-topmost-layer' crates/slicer-schema/wit/deps/common.wit` | AC-4 WIT | FACT hit/miss |
| `cargo test -p slicer-core --lib -- arachne::remove_small::tests::non_top_layer_strict --exact` | AC-N1 (packet-authored `#[cfg(test)]` mod in remove_small.rs) | FACT pass/fail |
| `cargo test -p arachne-perimeters --test only_one_wall_top_tdd -- only_one_wall_top_disabled --exact` | AC-N2 | FACT pass/fail |
| `cargo test -p slicer-runtime --test arachne_parity` | AC-5 locks | FACT pass/fail; SNIPPETS on fail |
| `cargo check --workspace --all-targets` | compile gate | FACT pass/fail |
| `cargo clippy --workspace --all-targets -- -D warnings` | lint gate | FACT pass/fail |
| `cargo xtask build-guests --check` | guest freshness after WIT/SDK/module edits | FACT clean/STALE |

## Step Completion Expectations

- **G10 signature change forces a one-line adaptation of the G10 gap test's
  CALL, and that is permitted.** The red test calls the 4-arg
  `remove_small_lines(lines, 20.0, 0.4, false)` and asserts `!surviving.is_empty()`.
  It is provably impossible to make that exact 4-arg/`false` call survive without
  also making a normal mid-stack layer keep short odd lines (which would violate
  AC-N1). Therefore `remove_small_lines` must grow a topmost/top-or-bottom
  parameter and the test's call must pass it `true`. Updating ONLY the argument
  list while leaving the `!surviving.is_empty()` assertion intact is NOT weakening
  the test (the assertion — the locked deliverable — is unchanged); it is adapting
  a caller to a changed signature. No other edit to `arachne_parity_gaps.rs` is
  permitted.
- Cross-step invariant: the WIT record + SDK bridge step must land and guests
  rebuild before any module test is trusted (stale guests mask both gaps).
- Ordering: WIT/pipeline plumbing (G10 flags) first; then module topmost
  detection reuses the same flags for G3 part 1; G3 part 2 (second pass) last.

## Context Discipline Notes

- Large files: `arachne-perimeters/src/lib.rs` (>500 lines — range-read
  `:295-306` discard + the emission region); `PerimeterGenerator.cpp:2160-2246`
  is the densest Orca surface — delegate a SUMMARY of the second-pass algorithm,
  never load it.
- Likely temptation: the full `interface_shells` upper-slice branch — out of
  scope unless a locking test needs it; record as follow-up.
- Heaviest dispatch: the second-pass algorithm SUMMARY (top-area derivation +
  renumbering) — cap at ≤200 words or one ≤30-line SNIPPET.
