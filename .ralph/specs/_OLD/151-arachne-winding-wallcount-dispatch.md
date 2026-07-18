---
status: implemented
packet: 151-arachne-winding-wallcount-dispatch
task_ids:
  - none
---

# 151-arachne-winding-wallcount-dispatch

## Goal

Make the Arachne wall generator honor wall count and winding config — fix the
`wall_count` → `max_bead_count` wiring bug, add `wall_direction`,
`only_one_wall_first_layer`, `overhang_reverse` (whole), and
`wall_maximum_resolution/deviation`, and force the classic generator under
spiral vase — flipping gap tests G1, G2, G7, G8, G9.

## Problem Statement

Five Arachne parity gaps plus one latent wiring bug all concern how many walls
are emitted and in which direction:

- **wall_count wiring bug (prerequisite, discovered in planning):**
  `arachne-perimeters` never reads `wall_count`. It reads `max_bead_count` —
  which IS registered on the module (`arachne-perimeters.toml:159`,
  `default = 9`) but effectively invisible when the user doesn't set it:
  `ConfigView` never merges schema defaults (`get_int` returns `None` for unset
  keys — `slice_ir.rs:807-814`; `bind_module_config_view` pre-filters the raw
  source without consulting defaults), so the module's own
  `.unwrap_or(defaults.max_bead_count)` (`lib.rs:119-122`) silently supplies 9;
  `LimitedBeadingStrategy`'s over-cap branch then yields ~5 walls on a 10 mm
  square (the `{0,1,2,3,4}` index anomaly observed during packet planning —
  NOT recorded in `docs/18`, which never mentions `wall_count`/`max_bead_count`;
  the new DEVIATION_LOG entry is where it gets recorded). OrcaSlicer sets
  `max_bead_count = 2 × inset_count` (`WallToolPaths.cpp:525`). Without this,
  G2's "force single wall" has no correct baseline to reduce from.
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

## Architecture Constraints

<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and, if `STALE:` is reported, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.

<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.

- **G9 unit exception:** `ArachneParams` stores mm² directly (existing defaults
  0.0025 = 0.05², 0.000025 = 0.005²). `wall_maximum_resolution` (0.5 mm) is
  squared to 0.25 mm² with NO ÷100 — the ÷100 rule applies to toolpath
  coordinates, not to these mm-based scalar params. Do not double-convert.
- **Winding has no existing helper** — the module must introduce a signed-area
  (shoelace) test + point-order reversal; there is no `make_ccw`/`reverse` to
  reuse in `slicer-ir` or `slicer-core`. Keep it local to the module's emission.
- **ADR-0011 conformance (wall sequencing):** ADR-0011 locks inter-wall
  *reordering* logic (e.g. InnerOuterInner grouping) into shared
  `slicer-perimeter-utils`. Winding is point-traversal *direction within one
  loop*, not inter-wall order, so the module-local shoelace normalization does
  not contradict it. If `classic-perimeters` later needs the same reversal
  (Orca applies `overhang_reverse` to both generators), promote the helper to
  `slicer-perimeter-utils` then — out of scope here.
- **ADR-0035 conformance (faithful emission):** the winding normalization is
  the port of OrcaSlicer's wall reorientation
  (`PerimeterGenerator.cpp:527-545` winding rule; `:58-98,422-429` odd-layer
  reversal) applied after junction emission — it is not a supplemental
  invention on top of the faithful `generateJunctions`/`connectJunctions`
  surface. Likewise the G9 rewiring only changes where
  `simplifyToolPaths`' tolerances come from (config vs `meshfix_*`), never the
  simplify algorithm itself; the replace-vs-supplement `[FWD]` dispatch must
  confirm the Orca sourcing before wiring.

## Data and Contract Notes

- IR/manifest: six new manifest keys; `overhang_reverse_threshold` uses packet
  150's `float_or_percent` type (hard dependency).
- WIT boundary: none in this packet (no `config.wit`/`common.wit` edit; the
  percent TYPE arrived in 150). Manifest + module + scheduler only.
- Determinism/scheduler: G8 changes claim resolution — must stay deterministic
  (spiral bool is a pure config read; no ordering nondeterminism introduced).

## Locked Assumptions and Invariants

- `max_bead_count = 2 × wall_count` (Orca `WallToolPaths.cpp:525`) is the wiring
  contract; the emitted distinct-index count on a solid square equals `wall_count`.
- The 15 `arachne_parity.rs` locks are invariant (AC-7); wall-count shifts are
  validated, not rebaselined.
- Default `wall_direction = counter_clockwise` must reproduce the prior
  (pre-packet) winding so absent-key configs are unchanged (AC-N2).
- G8's spiral fallback fires ONLY when spiral is active (AC-N1) — it must not
  become an unconditional classic override.

## Risks and Tradeoffs

- **wall_count wiring changes emitted counts across many existing tests.** Highest
  blast radius; mitigated by AC-7 and the "validate, don't rebaseline" rule.
- **Winding reversal touching every contour** could interact with seam placement
  / downstream path optimization — verify no seam/regression lock flips (AC-7,
  AC-N2).
- **G8 in the live loader** is exercised only on the wasm path; the contract test
  (AC-N1) plus the substring-probe red test (AC-5) both must pass so the behavior
  is real, not just string-satisfying.
