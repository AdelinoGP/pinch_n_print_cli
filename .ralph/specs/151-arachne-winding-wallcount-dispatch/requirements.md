# Requirements: 151-arachne-winding-wallcount-dispatch

## Packet Metadata

- Grouped task IDs:
  - `none` (audit-driven; backlog `docs/18_arachne_parity_audit.md`)
- Backlog source: `docs/18_arachne_parity_audit.md`
- Packet status: `draft`
- Aggregate context cost: `M`

## Problem Statement

Five Arachne parity gaps plus one latent wiring bug all concern how many walls
are emitted and in which direction:

- **wall_count wiring bug (prerequisite, discovered in planning):**
  `arachne-perimeters` never reads `wall_count`. It reads `max_bead_count`, which
  is unregistered on the module and silently defaults to 9; `LimitedBeadingStrategy`'s
  over-cap branch then yields ~5 walls on a 10 mm square (the audit's `{0,1,2,3,4}`
  index anomaly). OrcaSlicer sets `max_bead_count = 2 × inset_count`
  (`WallToolPaths.cpp:525`). Without this, G2's "force single wall" has no correct
  baseline to reduce from.
- **G1 `wall_direction`:** zero readers anywhere; contour winding cannot be
  controlled (Orca CCW/CW via `make_counter_clockwise`/`make_clockwise`, holes
  opposite the contour).
- **G2 `only_one_wall_first_layer`:** unregistered; layer 0 never reduced to one
  wall (Orca forces `loop_number = 0`).
- **G7 `overhang_reverse`:** `overhang_reverse`/`overhang_reverse_internal_only`/
  `detect_overhang_wall` are registered but have zero readers; the tuning key
  `overhang_reverse_threshold` is unregistered. Toggling changes nothing.
- **G8 spiral vase:** generator selection keys only off `wall_generator`; a
  spiral-vase job with `wall_generator=arachne` still selects the Arachne module,
  where Orca forces classic (`wall_generator == Arachne && !spiral_mode`).
- **G9 `wall_maximum_resolution`/`wall_maximum_deviation`:** unregistered; the
  simplification tolerances are compile-time constants sourced from `meshfix_*`.

These are one coherent slice: they all live in `arachne-perimeters` wall emission
(or, for G8, the one-line scheduler selection path), and the wall_count fix is the
shared baseline the winding and single-wall behaviors act on.

## In Scope

- Register `wall_count` on `arachne-perimeters`; read it and set
  `max_bead_count = 2 × wall_count` in `arachne_params_from_config`.
- Register `wall_direction` (enum, default `counter_clockwise`); add a
  signed-area-based winding normalization at emission (none exists today) that
  reverses contour point order to match the requested winding, with holes wound
  opposite the contour.
- Register `only_one_wall_first_layer`; force a single wall on layer 0.
- Read the already-registered `overhang_reverse` / `detect_overhang_wall` keys;
  register `overhang_reverse_threshold` as `float_or_percent` (packet-150 type);
  reverse odd-layer wall direction when `detect_overhang_wall=false` and
  `overhang_reverse=true`.
- Register `wall_maximum_resolution` (0.5 mm) and `wall_maximum_deviation`
  (0.025 mm); read and wire into `smallest_line_segment_squared` /
  `allowed_error_distance_squared` (square the mm value; mm²).
- Thread a spiral-vase input through the generator-selection path
  (`execution_plan_live.rs` extracts it from `config_source` like `wall_generator`;
  `dedup_same_claim_modules` forces classic when spiral is active).
- Close D-104c; add a DEVIATION_LOG entry for the wall_count bug; update docs.

## Out of Scope

- Flow spacing, thick_bridges, percent config TYPE machinery — packet 150.
- `only_one_wall_top`, top-surface second pass, `removeSmallLines` top exception,
  any `common.wit` change — packet 152.
- The `wall_sequence` InnerOuterInner ownership question (DEV-070, separate).

## Authoritative Docs

- `docs/04_host_scheduler.md` — claim dedup / generator selection (G8); delegate.
- `docs/15_config_keys_reference.md` — key provenance; delegate the entries.
- `docs/08_coordinate_system.md` — short; load for G9 mm² note.
- `docs/DEVIATION_LOG.md` — D-104c + wall_count entries.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/PerimeterGenerator.cpp:527-545` — winding (G1).
- `OrcaSlicerDocumented/src/libslic3r/PerimeterGenerator.cpp:2137-2139` — first-layer single wall (G2).
- `OrcaSlicerDocumented/src/libslic3r/PerimeterGenerator.cpp:58-98,422-429` — overhang reverse (G7).
- `OrcaSlicerDocumented/src/libslic3r/Arachne/WallToolPaths.cpp:525` — `max_bead_count = 2*inset_count`.
- `OrcaSlicerDocumented/src/libslic3r/Arachne/WallToolPaths.cpp:487-503,702-719` — resolution/deviation consumers (G9).
- `OrcaSlicerDocumented/src/libslic3r/LayerRegion.cpp:138-141` — spiral gate (G8).

## Acceptance Summary

- Positive: `AC-1` (wall_count→max_bead_count=2×), `AC-2` (G1 winding flip),
  `AC-3` (G2 first-layer single wall), `AC-4` (G7 odd-layer reversal),
  `AC-5` (G8 spiral→classic), `AC-6`/`AC-6b` (G9 registration + wiring),
  `AC-7` (14 locks green against the corrected wall-count baseline).
- Negative: `AC-N1` (spiral fallback fires ONLY when spiral active),
  `AC-N2` (absent `wall_direction` preserves prior default winding).
- Cross-packet: depends on 150 (`float_or_percent`); unblocks 152 (correct
  first-pass baseline for renumbering).

## Verification Commands

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo test -p slicer-runtime --test arachne_parity_gaps -- arachne_parity_pipeline_wall_direction_controls_winding --exact` | G1 | FACT pass/fail |
| `cargo test -p slicer-runtime --test arachne_parity_gaps -- arachne_parity_pipeline_only_one_wall_first_layer_forces_single_wall --exact` | G2 | FACT pass/fail |
| `cargo test -p slicer-runtime --test arachne_parity_gaps -- arachne_parity_pipeline_overhang_reverse_flips_odd_layer_walls --exact` | G7 | FACT pass/fail |
| `cargo test -p slicer-runtime --test arachne_parity_gaps -- arachne_parity_pipeline_spiral_vase_forces_classic_generator --exact` | G8 | FACT pass/fail |
| `cargo test -p slicer-runtime --test arachne_parity_gaps -- arachne_parity_pipeline_wall_max_resolution_deviation_registered --exact` | G9 registration | FACT pass/fail |
| `cargo test -p slicer-runtime --test arachne_parity_gaps -- arachne_parity_wall_count_wires_max_bead_count --exact` | wall_count bug (packet-authored) | FACT pass/fail |
| `cargo test -p arachne-perimeters --lib -- wall_maximum_resolution_wired` | G9 wiring | FACT pass/fail |
| `cargo test -p slicer-scheduler --test contract -- spiral_vase_arachne_dispatch` | AC-N1 | FACT pass/fail |
| `cargo test -p slicer-runtime --test arachne_parity` | AC-7/AC-N2 14 locks | FACT pass/fail; SNIPPETS on fail |
| `cargo check --workspace --all-targets` | compile gate | FACT pass/fail |
| `cargo clippy --workspace --all-targets -- -D warnings` | lint gate | FACT pass/fail |
| `cargo xtask build-guests --check` | guest freshness (module/manifest edits) | FACT clean/STALE |

## Step Completion Expectations

- Cross-step invariant: the wall_count wiring step MUST land before the G2/G7
  winding steps — those assert exact index sets / winding against the corrected
  baseline; running them against `max_bead_count=9` gives false reds/greens.
- Cross-step invariant: no step regresses the 14 `arachne_parity.rs` locks
  (AC-7). Locks that assert wall counts will legitimately shift when wall_count
  wiring lands — each shift must be validated as the wall_count-correct value
  (2×N produces N walls on the test squares), not blindly rebaselined.
- G8 lives in a different crate (`slicer-scheduler`/`slicer-wasm-host`) from the
  other four gaps; it has no ordering dependency and may land first or last.

## Context Discipline Notes

- Large files: `arachne-perimeters/src/lib.rs` (>500 lines — range-read
  `:108-225` params, `:295-306` overhang keys, `:467-497` emission);
  `execution_plan.rs` and `execution_plan_live.rs` (range-read the dedup +
  loader signature only, `:250-262` / `:201-216`).
- Likely temptation: reading `LimitedBeadingStrategy` to "understand" the
  `{0,1,2,3,4}` anomaly — unnecessary; the anomaly is fully explained by the
  unread `wall_count` (see design.md), and the fix is a config read, not a
  strategy change. Skip `crates/slicer-core/src/beading/**`.
- Heaviest dispatch: the winding-normalization Orca query — return SUMMARY ≤200
  words or one ≤30-line SNIPPET.
