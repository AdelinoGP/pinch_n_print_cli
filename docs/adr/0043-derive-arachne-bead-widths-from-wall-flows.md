# ADR-0043 — Arachne Bead Widths Derive From the User's Wall Flows; the Internal Knob Config Keys Are Retired

<!-- filename: 0043-derive-arachne-bead-widths-from-wall-flows -->

## Status

Accepted (2026-07-16). Enacted by the D-160 fix pair (emission back-conversion
+ wall-width wiring) during the Arachne Parity Recovery campaign
(`docs/specs/arachne-parity-recovery.md` §D6).

## Context

`arachne-perimeters` exposed two Arachne-INTERNAL algorithm parameters as
user-facing config keys: `optimal_width` and `preferred_bead_width_outer`
(both `unit = "units"`, defaulting to 4000 = 0.4mm). `arachne_params_from_config`
read those keys — and never `outer_wall_line_width` / `inner_wall_line_width` —
so arachne's emitted wall width was invariant to the user's wall-width setting
(`D-160-ARACHNE-IGNORES-WALL-LINE-WIDTH`: ask for 0.8mm walls, get 0.357mm).

Canonical OrcaSlicer has no such user keys. `PerimeterGenerator` *derives*
Arachne's two bead-width targets from the user's wall flows:
`bead_width_0 = ext_perimeter_spacing = ext_perimeter_flow.scaled_spacing()`
(the outer wall's flow) and `bead_width_x = perimeter_spacing =
perimeter_flow.scaled_spacing()` (the inner wall's flow), passed into
`WallToolPaths` and thence `BeadingStrategyFactory::makeStrategy`. The
`optimal_width` manifest entry itself documented the trap: "Not a user-facing
OrcaSlicer PrintConfig.cpp option — upstream sets it internally."

## Decision

1. **The config keys `optimal_width` and `preferred_bead_width_outer` are
   retired outright** — deleted from `arachne-perimeters.toml`, no
   deprecation alias, no override escape hatch. Upstream has no such user
   keys, and keeping them as overrides would preserve the exact wiring that
   let D-160 hide (a knob that silently shadows the real setting).
2. **`arachne-perimeters` declares and reads `outer_wall_line_width` /
   `inner_wall_line_width`** (plain mm floats, default 0.4, range [0.1, 2.0],
   group Walls — mirroring `classic-perimeters`), sourcing
   `preferred_bead_width_outer` from the outer key (canonical `bead_width_0`)
   and `optimal_width` from the inner key (canonical `bead_width_x`). The
   module's existing `line_width_to_spacing` conversion stays where it was;
   only the raw source changed. (Executing D-160's originally written fix
   shape — wrapping the source in another `line_width_to_spacing` — would
   have double-converted.)
3. **`ArachneParams`' struct FIELDS keep their canonical names**
   (`optimal_width` = `bead_width_x`, `preferred_bead_width_outer` =
   `bead_width_0`). The retirement is a config-contract change only; code
   that builds `ArachneParams` directly (tests, host bridge, WIT mirror) is
   untouched and stays greppable against canonical.

## Consequences

- Users get canonical behaviour: the wall-width keys govern arachne exactly
  as they govern classic (verified end-to-end: classic and arachne both emit
  0.4000mm at default and 0.8000mm at outer=inner=0.8 on
  `regression_wedge.stl`).
- **Trade-off accepted:** tests lose the ability to steer the beading engine
  directly through config. Post-retirement, a test that wants a non-default
  bead target sets the wall-width keys (mm) or builds `ArachneParams`
  directly. 13 test files were migrated `mm_to_units(W)` → plain `W` — a
  UNIT change, where a miss is a silent 100× error (cf.
  `D-147-STITCH-TINY-POLY-UNITS`).
- The retirement is hard to reverse: reintroducing the keys later would
  reintroduce the shadow-wiring hazard, so any future need for direct beading
  control should go through `ArachneParams` construction, not config.
- Upstream models both wall-width keys as `coFloatOrPercent` with default `0`
  = "auto from nozzle"; PnP's plain float `min 0.1` cannot express auto —
  logged as its own deviation row rather than silently narrowed.
