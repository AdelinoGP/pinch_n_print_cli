# Requirements: 150-arachne-flow-and-percent-config

## Packet Metadata

- Grouped task IDs:
  - `none` (audit-driven; backlog source is `docs/18_arachne_parity_audit.md`,
    following the precedent of packets 148/149 which also declare `task_ids: none`)
- Backlog source: `docs/18_arachne_parity_audit.md`
- Packet status: `active`
- Aggregate context cost: `M`

## Problem Statement

The Arachne wall generator diverges from OrcaSlicer on three physically
load-bearing points, each an accepted-but-open deviation:

- **D-105 (flow spacing not wired):** OrcaSlicer feeds Flow *spacing* — a
  layer-height-dependent value `width − h·(1−π/4)` — into bead placement, so
  adjacent walls sit one spacing apart. PnP feeds raw `optimal_width`, so every
  wall pair is over-spaced by `h·(1−π/4)` (≈0.043 mm at 0.2 mm layers).
  `slicer_core::flow::line_width_to_spacing` already exists and returns the
  correct value; the module simply never calls it and never reads `layer_height`.
- **D-104g (thick_bridges stub):** OrcaSlicer bridges with a round cross-section
  of thread diameter `dmr`, giving ≈1.57× the volume of a flat bead. PnP's
  `bridging_flow` returns a hardcoded `1.0` for `thick_bridges==true`, so bridge
  vertices get no flow adjustment.
- **D-104h (no percent config type):** OrcaSlicer expresses several Arachne keys
  as percentages of nozzle diameter or wall width (`min_width_top_surface` 300%,
  `min_feature_size` 25%, `wall_transition_length` 100%). PnP has no percent
  config type, so these keys are pre-resolved absolute floats that silently go
  stale when the nozzle changes.

These form one coherent slice because all three need the same missing plumbing —
`layer_height`/`nozzle_diameter` in the module's resolved config — and the
percent type (D-104h) is the canonical vehicle for the nozzle-relative defaults
the other two rely on. Landing them together avoids shipping absolute defaults
that a later percent-type packet would have to migrate. A fourth, adjacent
defect is absorbed here because it shares the nozzle-diameter plumbing:
`classic-perimeters` reads `nozzle_diameter` (`src/lib.rs:183-186`) but never
registers it in its manifest, so the host never binds it and the read is dead
code that always falls back to `inner_wall_line_width`.

## In Scope

- Add `percent` and `float_or_percent` to the valid config-type set
  (`crates/slicer-schema`), wire it into live manifest validation
  (`parse_config_field_entry`) and the numeric-type predicate
  (`is_numeric_field_type`).
- Extend the `config-value` WIT variant (`crates/slicer-schema/wit/deps/config.wit:4-7`)
  with `percent-val` / `float-or-percent-val` cases, add the matching adapter
  arms in `slicer-macros` (`__slicer_adapt_config`, `src/lib.rs:590-601`), and
  extend `ConfigValue` (`crates/slicer-ir`) with `Percent` / `FloatOrPercent`
  variants + a read-time `get_abs_value(key, base)` accessor that resolves
  against a caller-supplied base (mirrors Orca's `get_abs_value`; the base
  parameter is what keeps this future-proof for runtime-computed bases). This is
  a WIT contract change ⇒ all guests rebuild (WIT/Type Changes checklist).
- Retype `min_width_top_surface` (300%), `min_feature_size` (25%),
  `wall_transition_length` (100%) in `arachne-perimeters.toml` to the canonical
  percent form.
- Register `layer_height` (default 0.2) and `nozzle_diameter` (default 0.4 mm)
  in `arachne-perimeters.toml`; read both via `ConfigView` in the module.
- Wire `line_width_to_spacing` into the widths `arachne-perimeters` feeds the
  beading pipeline so bead placement uses spacing, not raw width.
- Replace the `thick_bridges==true` 1.0 stub in
  `slicer_core::flow::bridging_flow` with the OrcaSlicer round-section factor;
  update the module call site to pass the needed inputs (nozzle diameter, bead
  width, layer height).
- Register `nozzle_diameter` in `classic-perimeters.toml` (adjacent dead-read
  fix) + a lock test.
- Close deviations D-105, D-104g, D-104h; update docs/03, docs/15, docs/18.

## Out of Scope

- `wall_direction`, `only_one_wall_first_layer`, `overhang_reverse`, spiral-vase
  dispatch, `wall_maximum_resolution/deviation`, and the `wall_count` →
  `max_bead_count` wiring bug — all packet 151.
- `only_one_wall_top` second Arachne pass and `removeSmallLines` top-layer
  exception — packet 152.
- `common.wit` edits (this packet touches only `config.wit`; the
  `removeSmallLines` top-layer flag on `arachne-params` is packet 152).
- Host-side percent pre-resolution (rejected 2026-07-09 for future-proofing:
  Orca's `get_abs_value(base)` takes the base as a parameter so it also handles
  runtime-computed bases a host-side resolver could never express; see
  `design.md` §Code Change Surface).

## Authoritative Docs

- `docs/03_wit_and_manifest.md` — ~medium; delegate a SUMMARY of the config
  type / validation section only.
- `docs/15_config_keys_reference.md` — large; delegate the three key entries.
- `docs/08_coordinate_system.md` — short; load directly for the spacing unit
  conversion.
- `docs/DEVIATION_LOG.md` — load only the D-105/D-104g/D-104h entries.
- `docs/02_ir_schemas.md` — delegate the `ConfigValue`/`ConfigView` section for
  the new-variant contract.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/Flow.cpp` / `Flow.hpp` — spacing formula
  (`Flow.hpp:67`) and thick-bridge `dmr` derivation + round section
  (`Flow.hpp:106`); the `dmr` formula is the arbiter of AC-4's ≈1.57 factor.
- `OrcaSlicerDocumented/src/libslic3r/PerimeterGenerator.cpp:2129,2172-2173` —
  confirms spacing (not width) feeds `WallToolPaths`.
- `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp:1498-1511,7169-7178,7217-7226`
  — percent defaults for the three retyped keys.
- `OrcaSlicerDocumented/src/libslic3r/LayerRegion.cpp:31-50,135` — bridging_flow
  call context (flow factor only; LayerRegion plumbing not ported).

## Acceptance Summary

- Positive cases: `AC-1` (percent types + canonical defaults), `AC-2`
  (`get_abs_value` resolution), `AC-3` (Flow-spacing gap ≈0.3571 mm),
  `AC-4` (thick-bridge factor ≠1.0), `AC-5` (classic nozzle read),
  `AC-6` (14 `arachne_parity.rs` locks stay green) from `packet.spec.md`.
  Refinements: AC-3's tolerance is 0.02 mm and the assertion is on
  perimeter_index-0-vs-1 min-x delta; AC-1 requires the default VALUES too
  (300/25/100%), not merely the type string.
- Negative cases: `AC-N1` (malformed percent default rejected with a
  key-naming diagnostic), `AC-N2` (non-positive base → `None`/documented
  fallback, no NaN).
- Cross-packet impact: unblocks 151 (`float_or_percent` for
  `overhang_reverse_threshold`) and 152 (`min_width_top_surface` via
  `get_abs_value`). No packet is blocked by this one.

## Verification Commands

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo test -p slicer-runtime --test arachne_parity_gaps -- arachne_parity_pipeline_percent_config_type_for_arachne_keys --exact` | AC-1 flips green | FACT pass/fail |
| `cargo test -p slicer-ir --lib config -- percent` | AC-2 + AC-N2 resolver | FACT pass/fail; SNIPPETS ≤20 on fail |
| `cargo test -p slicer-runtime --test arachne_parity_gaps -- arachne_parity_pipeline_wall_gap_uses_flow_spacing_not_width --exact` | AC-3 flips green | FACT pass/fail; SNIPPETS ≤20 on fail |
| `cargo test -p slicer-runtime --test arachne_parity_gaps -- arachne_parity_pipeline_thick_bridges_flow_factor_not_stubbed_to_one --exact` | AC-4 flips green | FACT pass/fail; SNIPPETS ≤20 on fail |
| `cargo test -p classic-perimeters --lib -- nozzle_diameter` | AC-5 dead-read fix | FACT pass/fail |
| `cargo test -p slicer-scheduler --test contract -- config_percent_type` | AC-N1 malformed rejection | FACT pass/fail; SNIPPETS ≤20 on fail |
| `cargo test -p slicer-runtime --test arachne_parity` | AC-6 14 locks stay green | FACT pass/fail; SNIPPETS ≤20 on fail |
| `cargo test -p slicer-core --lib flow` | bridging_flow + spacing unit tests | FACT pass/fail |
| `cargo check --workspace --all-targets` | compiles incl. test/bench targets | FACT pass/fail |
| `cargo clippy --workspace --all-targets -- -D warnings` | lint gate | FACT pass/fail |
| `cargo xtask build-guests --check` | guest WASM fresh after module/IR/schema edits | FACT clean/STALE |

## Step Completion Expectations

- Cross-step invariant: no step may regress the 14 `arachne_parity.rs` locks
  (AC-6), even steps that do not edit that file. The flow-spacing step (which
  moves wall positions) is the one most likely to trip it — the
  `precise_outer_wall` lock survives only because it asserts a relative delta;
  verify AC-6 immediately after that step, not only at packet close.
- Ordering rationale: the percent-type step (schema + IR) precedes the manifest
  retype step, which precedes the module read-time resolution — each is a
  precondition of the next. The flow-spacing and bridging-flow steps are
  independent of the percent work and may land in either order after
  `layer_height`/`nozzle_diameter` are registered.

## Context Discipline Notes

- Large files in the read-only path that MUST be ranged or delegated:
  `docs/15_config_keys_reference.md` (delegate the three key entries),
  `modules/core-modules/arachne-perimeters/src/lib.rs` (>500 lines — range-read
  around `arachne_params_from_config` `:108-225` and the bridging call `:436-438`),
  `crates/slicer-runtime/tests/arachne_parity.rs` (>800 lines — never read in
  full; grep for a failing test name).
- Likely temptation reads: the full beading strategy stack under
  `crates/slicer-core/src/beading/` — not needed; the spacing change is upstream
  of it (only the width value fed in changes). Skip unless AC-3 fails in a way
  that a FACT dispatch traces into beading.
- Heaviest dispatch return-format hint: the `dmr` formula query to
  `OrcaSlicerDocumented/Flow.cpp` must return `SUMMARY` (≤200 words) or a single
  ≤30-line `SNIPPET` — not the file.
