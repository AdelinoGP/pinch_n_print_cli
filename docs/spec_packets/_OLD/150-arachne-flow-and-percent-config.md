---
status: implemented
packet: 150-arachne-flow-and-percent-config
task_ids:
  - none
---

# 150-arachne-flow-and-percent-config

## Goal

Wire OrcaSlicer flow physics into the Arachne wall generator — feed Flow
**spacing** (not raw width) into bead placement, give `thick_bridges` a real
round-cross-section flow factor, and add a `percent` / `float_or_percent`
config type so nozzle-relative keys rescale — flipping gap tests G4, G5, G6.

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

## Architecture Constraints

<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and, if `STALE:` is reported, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.

<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.

- **WIT/Type Changes checklist (CLAUDE.md):** the `config-value` variant is read
  by both host (`bindgen!`) and guest (`wit_bindgen::generate!` per module). The
  new arms must be added to the WIT variant AND every exhaustive `match` on it
  (`slicer-macros __slicer_adapt_config`) or guests fail to compile. Type
  identity must match across the boundary; run `cargo build --tests` after the
  WIT edit and `cargo xtask build-guests` (not `--check`) to regenerate guests.

- **Unit-consistency hazard (the crux of AC-3):** widths in the module config
  are in slicer units (`optimal_width = mm_to_units(0.4) = 4000`) while
  `layer_height` arrives in mm (the G4 test passes `.float("layer_height", 0.2)`
  = 0.2 mm, not units). `line_width_to_spacing`'s formula `w − h·(1−π/4)` is
  linear, so it is scale-invariant ONLY if `w` and `h` share a unit. The module
  MUST convert `layer_height` to the same unit as the widths before subtracting
  (e.g. `mm_to_units(0.2) = 2000`), yielding `4000 − 2000·0.2146 ≈ 3571 units
  ≈ 0.3571 mm`. Feeding `4000 − 0.2·0.2146 ≈ 4000` (mixed units) leaves the gap
  at 0.4 mm and AC-3 fails. The red test is the falsifier — do not hand-wave the
  conversion.
- Percent resolution is **module-side at read time**, not host pre-resolution:
  Orca resolves each percent against a per-call-site base (nozzle diameter for
  `min_feature_size`, wall width for `min_width_top_surface`). The host cannot
  know the base, so `ConfigValue::Percent` survives into `ConfigView` and the
  module supplies the base at read time.

## Data and Contract Notes

- IR/manifest contracts touched: `ConfigValue` gains variants (additive; existing
  `get_float`/`get_int` must keep returning `None` for percent values, not
  coerce); manifest config-type set widens.
- WIT boundary: **yes** — `config.wit`'s `variant config-value` gains
  `percent-val`/`float-or-percent-val` (confirmed the deciding boundary via a
  read-only dispatch: `config.wit:4-7` + adapter `slicer-macros/src/lib.rs:590-601`).
  The variant crosses host→guest per module; both the `bindgen!` host side and
  every guest's `wit_bindgen::generate!` regenerate from it, so all guests
  rebuild. `common.wit` is NOT touched (that boundary is packet 152).
- Determinism: spacing is a pure function of width/height; no scheduler impact.

## Locked Assumptions and Invariants

- The `config-value` WIT variant and `slicer_ir::ConfigValue` must stay 1:1;
  the `slicer-macros __slicer_adapt_config` match is exhaustive, so a new WIT
  arm without a matching Rust arm (or vice versa) is a compile error by
  construction — do not add a catch-all `_ =>` arm that would hide a future drift.
- `line_width_to_spacing` stays the single source of the spacing formula; the
  module must not inline a second copy.
- The 14 `arachne_parity.rs` locks are invariant (AC-6). The
  `precise_outer_wall` lock's relative-delta assertion is the reason the spacing
  change is safe; if any lock asserts an absolute wall position it must be
  surfaced, not silently rebaselined.
- `get_float("min_feature_size")` on a now-percent key returns `None` (type
  mismatch), so any existing reader that used `get_float` on these three keys
  must migrate to `get_abs_value` — audit the module for such readers before
  retyping (the three keys' current readers are in `arachne_params_from_config`).

## Risks and Tradeoffs

- **Spacing change moves every Arachne wall.** Highest regression risk; mitigated
  by AC-6 and the relative-delta nature of the surviving lock. Self-captured e2e
  baselines (D-109) may shift — re-verify, and if a baseline moves, confirm the
  new value equals the spacing-correct position before rebaselining.
- **Retyping keys that a reader still consumes via `get_float`** would silently
  zero them (get_float → None → unwrap_or(default)). The Locked-Assumptions
  audit prevents this.
- **Percent variant crossing the SDK config boundary** is the one unknown that
  could force a larger change (see Open Questions [BLOCK]).
