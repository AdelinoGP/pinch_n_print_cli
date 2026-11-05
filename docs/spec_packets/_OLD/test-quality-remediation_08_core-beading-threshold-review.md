---
status: implemented
packet: test-quality-remediation_08_core-beading-threshold-review
task_ids:
  - core/DUP-CORE (beading factory)
---

# test-quality-remediation_08_core-beading-threshold-review

## Goal

Preserve the three distinct factory threshold witnesses and add an independently expected non-default full-stack case so default seeds, computed propagation, and clamp bounds remain separately falsifiable.

## Problem Statement

The §5.1 `DUP-CORE` entry names threshold propagation in `beading_factory.rs`. The grounded test body contains three related but distinct witnesses: `beading_factory_passes_split_middle_thresholds` protects the public `BeadingFactoryParams::default()` `.99/.99` seeds; `beading_factory_threshold_propagates_through_full_stack` protects forwarding through `Limited → OuterWallInset → Widening → Redistribute → Distributed`; and `beading_factory_threshold_clamp_bounds_are_canonical` protects lower, interior, and upper formula outcomes. The approved row-#8 scope explicitly preserves all three rather than presuming they are redundant.

The propagation test's existing optional-stack case uses shipped default widths, so its expected `.99/.99` values overlap the default seed and upper-clamp witnesses. The additive contrasting case uses independently derived literals: split `(2 × 3000 / 5000) - 1 = 0.20` and add `3000 / 4000 = 0.75`. This catches a wrapper or factory regression that silently hard-codes `.99` or swaps the two getter paths, while leaving the original propagation case intact. Runtime production calls and fixed numeric assertions are the acceptance oracle; later code review verifies that the authored assertions remain in the selected test.

## Architecture Constraints

<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.

- The two threshold outputs are dimensionless ratios. `min_output_width`, `optimal_width`, and `preferred_bead_width_outer` are already slicer-unit widths; the test's `.20` and `.75` expectations are hand-derived literals and require no unit conversion.
- `BeadingFactoryParams::default()` seeds the public fields at `.99/.99`, while `create_stack` recomputes local values from the width inputs. The packet must preserve and explain this distinction; it must not change defaults or make the factory consume the seeded fields.
- The original optional-stack propagation case is retained verbatim: `outer_wall_offset = 300.0`, `print_thin_walls = true`, default widths, and `.99/.99` getter assertions. The contrasting case is additive, uses the same optional-stack flags, and expects `.20/.75` independently.
- All edited `BeadingFactoryParams` literals retain `..Default::default()`; the type is a watched public struct under `docs/21_data_defaults_and_fixtures.md`.
- No guest/WASM surface is fed by this host-side test-only edit, so the `wasm-staleness` snippet does not apply and no guest artifact check belongs to the packet.

## Data and Contract Notes

- IR/manifest contracts: none; `BeadingFactoryParams` is an internal core parameter bundle and no field is added or changed.
- WIT boundary: none; no host/module or component type is touched.
- Determinism/scheduler constraints: none beyond fixed literal inputs and deterministic getter results; no pipeline or scheduler execution is introduced.

## Locked Assumptions and Invariants

- All three threshold tests remain registered and named exactly as they are today; none is retired, renamed, or moved.
- The original propagation case's `.99/.99` assertions remain alongside the additive `.20/.75` case; the latter is not a replacement.
- `.20` is the independent expected result of `(2 × 3000 / 5000) - 1`; `.75` is the independent expected result of `3000 / 4000`; neither is calculated by implementation code or a test-local production mirror.
- The default `.99/.99` field assertions remain separate from `create_stack`'s local recomputation, and the clamp test remains a separate lower/interior/upper matrix.
- No production symbol, public signature, test target, fixture, schema, WIT type, or manifest entry changes.
- The §7 implementation ledger remains `partial`; this packet does not close the core wave or claim prior/later packets are implemented.

## Risks and Tradeoffs

- The propagation test now contains two cases in one function, so a failure in either case reports through one exact filter; independent literals prevent the default case from masking a hard-coded `.99` implementation, and later code review verifies the authored ordinary Rust assertions.
- The default seed fields are not the values consumed by `create_stack`; comments must make that deliberate distinction explicit rather than suggesting field propagation.
- The explicit `--features host-algos` form is retained for core-wave discipline even though `beading_factory` has no `required-features` entry; omitting it would make this packet inconsistent with §6, not more correct.
- The target is host-only and no guest freshness check is needed; a future guest test failure must not be attributed to this packet without the repository freshness gate.
