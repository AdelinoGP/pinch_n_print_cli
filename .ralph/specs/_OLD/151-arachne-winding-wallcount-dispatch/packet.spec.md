---
status: implemented
packet: 151-arachne-winding-wallcount-dispatch
task_ids:
  - none
backlog_source: docs/18_arachne_parity_audit.md
context_cost_estimate: M
---

# Packet Contract: 151-arachne-winding-wallcount-dispatch

## Goal

Make the Arachne wall generator honor wall count and winding config — fix the
`wall_count` → `max_bead_count` wiring bug, add `wall_direction`,
`only_one_wall_first_layer`, `overhang_reverse` (whole), and
`wall_maximum_resolution/deviation`, and force the classic generator under
spiral vase — flipping gap tests G1, G2, G7, G8, G9.

## Scope Boundaries

Touches the `arachne-perimeters` module (wall_count wiring, winding enforcement,
first-layer single wall, overhang_reverse odd-layer reversal, max-resolution
tolerances) and the scheduler/runtime generator-selection path (spiral-vase
fallback to classic). Depends on packet 150 for the `float_or_percent` config
type used by `overhang_reverse_threshold`. No flow/percent-model, top-surface,
or WIT-contract work — those are packets 150/152.

## Prerequisites and Blockers

- Depends on: packet 150 (`float_or_percent` config type for
  `overhang_reverse_threshold`; `wall_count`/`nozzle_diameter` plumbing patterns).
- Unblocks: packet 152 (correct first-pass wall count is the baseline the
  topmost single-wall reduction and inset renumbering build on).
- Activation blockers: packet 150 must be `status: implemented` before 151
  activates (only one active packet at a time; and 150 ships the config type
  151 declares against).

## Acceptance Criteria

Acceptance Criteria are stated **once**, here.

- **AC-1 (wall_count wiring). Given** `wall_count=3`, `optimal_width=0.4 mm` on a
  10 mm square with `max_bead_count` not explicitly set in the config source,
  **when** `arachne-perimeters` emits walls, **then** the module reads
  `wall_count` and sets `max_bead_count = 2 × wall_count = 6`, and distinct
  Outer/Inner `perimeter_index` values are `{0,1,2}` — not the current
  `{0,1,2,3,4}` (module-side `unwrap_or(9)` fallback at `lib.rs:119-122`; the
  key IS registered with `default = 9` at `arachne-perimeters.toml:159`, but
  `ConfigView` never merges schema defaults, so unset ⇒ `get_int` → `None`).
  Precedence: an explicitly-set `max_bead_count` (`get_int` → `Some`) still
  wins; only the unset case takes `2 × wall_count`. The AC test is APPENDED to
  `arachne_parity_gaps.rs` (existing test bodies are arbiters — never modify
  them). |
  `cargo test -p slicer-runtime --test arachne_parity_gaps -- arachne_parity_wall_count_wires_max_bead_count --exact`
- **AC-2 (G1 wall_direction). Given** `wall_direction` registered
  (default `counter_clockwise`), **when** it is flipped to `clockwise`, **then**
  the outer (perimeter_index 0, Outer) contour's shoelace signed area reverses
  sign; holes wind opposite the contour. |
  `cargo test -p slicer-runtime --test arachne_parity_gaps -- arachne_parity_pipeline_wall_direction_controls_winding --exact`
- **AC-3 (G2 only_one_wall_first_layer). Given** `only_one_wall_first_layer=true`
  and `wall_count=3`, **when** `arachne-perimeters` runs on layer 0, **then** it
  emits exactly one distinct perimeter index (`{0}`); on layer 1 it still emits
  the full count. |
  `cargo test -p slicer-runtime --test arachne_parity_gaps -- arachne_parity_pipeline_only_one_wall_first_layer_forces_single_wall --exact`
- **AC-4 (G7 overhang_reverse). Given** `detect_overhang_wall=false` and
  `overhang_reverse=true` with `overhang_reverse_threshold` registered
  (`float_or_percent`), **when** walls are emitted on an odd layer (index 1),
  **then** the outer-wall signed area is opposite that of the same config with
  `overhang_reverse=false` (odd-layer direction flip). |
  `cargo test -p slicer-runtime --test arachne_parity_gaps -- arachne_parity_pipeline_overhang_reverse_flips_odd_layer_walls --exact`
- **AC-5 (G8 spiral vase). Given** `wall_generator=arachne` and spiral vase
  active, **when** the generator is selected, **then** the selection path
  (`execution_plan.rs` and/or `run.rs`) falls back to `classic-perimeters`
  regardless of `wall_generator`. |
  `cargo test -p slicer-runtime --test arachne_parity_gaps -- arachne_parity_pipeline_spiral_vase_forces_classic_generator --exact`
- **AC-6 (G9 registration). Given** the manifest, **when** inspected, **then**
  `wall_maximum_resolution` (0.5 mm) and `wall_maximum_deviation` (0.025 mm) are
  registered `[config.schema.*]` keys — this alone flips the G9 gap test, which
  asserts registration only. |
  `cargo test -p slicer-runtime --test arachne_parity_gaps -- arachne_parity_pipeline_wall_max_resolution_deviation_registered --exact`
- **AC-6b (G9 wiring, packet-authored). Given** the two keys read by the module,
  **when** `ArachneParams` is built, **then** `smallest_line_segment_squared =
  wall_maximum_resolution²` and `allowed_error_distance_squared =
  wall_maximum_deviation²` (both in mm² — `ArachneParams` is mm-based, so the
  mm config value is squared directly; NO ÷100, unlike toolpath coordinates),
  replacing the compile-time `meshfix_*`-sourced defaults (0.0025 / 0.000025).
  The test lives in a packet-authored `#[cfg(test)] mod tests` in
  `arachne-perimeters/src/lib.rs` — the crate has NO in-file test module today
  (tests live in `tests/*.rs`), so without authoring it the `--lib` filter
  matches nothing and false-passes ("running 0 tests … ok"). |
  `cargo test -p arachne-perimeters --lib -- wall_maximum_resolution_wired --nocapture`
- **AC-7 (regression lock). Given** the 15 `arachne_parity.rs` locks, **when**
  the wall_count wiring changes the emitted wall counts, **then** all 15 still
  pass (locks that assert wall counts must be re-verified against the corrected
  baseline; any shift is validated as the wall_count-correct value, not
  rebaselined blindly). |
  `cargo test -p slicer-runtime --test arachne_parity`

## Negative Test Cases

- **AC-N1 (spiral does not leak to non-spiral). Given** `wall_generator=arachne`
  and spiral vase INACTIVE, **when** the generator is selected, **then**
  `arachne-perimeters` is still selected — the spiral fallback fires only when
  spiral vase is active, never unconditionally. Packet-authored: new
  `crates/slicer-scheduler/tests/contract/spiral_vase_arachne_dispatch_tdd.rs`,
  registered via `mod` in `tests/contract/main.rs` (the binary is named
  `scheduler_contract` — declared in `slicer-scheduler/Cargo.toml` `[[test]]` —
  NOT `contract`). |
  `cargo test -p slicer-scheduler --test scheduler_contract -- spiral_vase_arachne_dispatch --nocapture`
- **AC-N2 (winding default preserved). Given** `wall_direction` absent from
  config, **when** walls are emitted, **then** the winding matches the prior
  (pre-packet) default — registering the key must not silently flip existing
  output. |
  `cargo test -p slicer-runtime --test arachne_parity`

## Verification

- `cargo xtask build-guests --check` (module + manifest edits feed the guest build)
- `cargo check --workspace --all-targets` and
  `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test -p slicer-runtime --test arachne_parity_gaps -- arachne_parity_pipeline_wall_direction_controls_winding arachne_parity_pipeline_only_one_wall_first_layer_forces_single_wall arachne_parity_pipeline_overhang_reverse_flips_odd_layer_walls arachne_parity_pipeline_spiral_vase_forces_classic_generator arachne_parity_pipeline_wall_max_resolution_deviation_registered`

## Authoritative Docs

- `docs/04_host_scheduler.md` — claim dedup / generator selection (G8); delegate
  the claim-conflict section.
- `docs/15_config_keys_reference.md` — key provenance for the five new keys;
  delegate the specific entries.
- `docs/08_coordinate_system.md` — unit conversion for G9 tolerances (load).
- `docs/DEVIATION_LOG.md` — `D-104c-OVERHANG-REVERSE-NONE` (G7,
  `DEVIATION_LOG.md:80`) closes here; the wall_count bug gets a new entry
  (`D-151-WALLCOUNT-MAXBEAD-UNWIRED`).

## Doc Impact Statement (Required)

Changes config contracts and scheduler dispatch, so `none` is not eligible:

- `docs/15_config_keys_reference.md` — add `wall_direction`,
  `only_one_wall_first_layer`, `overhang_reverse_threshold`,
  `wall_maximum_resolution`, `wall_maximum_deviation`, `wall_count` (arachne) —
  `rg -q 'wall_maximum_deviation' docs/15_config_keys_reference.md`
- `docs/04_host_scheduler.md` — ADD a generator-selection subsection
  (`wall_generator` dispatch is currently undocumented there — 0 hits) covering
  the `wall_generator` dedup and the new spiral-vase fallback —
  `rg -q 'spiral' docs/04_host_scheduler.md`
- `docs/DEVIATION_LOG.md` — close `D-104c-OVERHANG-REVERSE-NONE`; add the
  wall_count wiring entry `D-151-WALLCOUNT-MAXBEAD-UNWIRED` —
  `rg -q 'D-104c-OVERHANG-REVERSE-NONE.*(CLOSED|closed)' docs/DEVIATION_LOG.md`
  and `rg -q 'D-151-WALLCOUNT-MAXBEAD-UNWIRED' docs/DEVIATION_LOG.md`
- `docs/18_arachne_parity_audit.md` — mark G1/G2/G7/G8/G9 closed in each Gn
  row's PnP-status column, following the file's existing `… closed` style
  (cf. the `(D-104 closed)` / `(D-104e closed)` entries) —
  `rg -q 'G9.*closed' docs/18_arachne_parity_audit.md`

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/PerimeterGenerator.cpp:527-545` —
  `make_counter_clockwise`/`make_clockwise`; holes opposite contour (G1).
- `OrcaSlicerDocumented/src/libslic3r/PerimeterGenerator.cpp:2137-2139` —
  `only_one_wall_first_layer` forces `loop_number = 0` (G2).
- `OrcaSlicerDocumented/src/libslic3r/PerimeterGenerator.cpp:58-98,422-429` —
  `detect_steep_overhang` + odd-layer reversal (G7).
- `OrcaSlicerDocumented/src/libslic3r/Arachne/WallToolPaths.cpp:525` —
  `max_bead_count = 2 * inset_count` (wall_count wiring).
- `OrcaSlicerDocumented/src/libslic3r/Arachne/WallToolPaths.cpp:487-503,702-719` —
  outline prep + `simplifyToolPaths` consuming max resolution/deviation (G9).
- `OrcaSlicerDocumented/src/libslic3r/LayerRegion.cpp:138-141` — Arachne dispatch
  gated on `!spiral_mode` (G8).

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- stop reading at 60% context and hand off at 85%

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation.
