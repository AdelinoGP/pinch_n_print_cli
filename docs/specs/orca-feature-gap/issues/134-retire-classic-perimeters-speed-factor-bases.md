# 134 — Retire classic-perimeters' module-private wall-speed factor bases

Type: task
Status: open
Assignee: —
Blocked by: —
Map: ../map.md

## Question

Filed by wayfinder ticket 114 (resolved 2026-09-05), step 5 of which decided:
**retire the `speed_factor = module-key / private-constant` pattern wherever the
host `FeedrateConfig` reads the same key as the factor's absolute base.** Ticket
114 executed the decision for the three infill modules
(`sparse_infill_speed`); this ticket is the one same-shape instance remaining
in the tree, found by the inventory the decision required.

`classic-perimeters` computes wall speed factors against a module-private
base (`BASE_SPEED = 50.0`,
`crates/slicer-core/src/perimeter_utils.rs`), while the emitted feedrate is
`FeedrateConfig::<role>_speed × 60 × factor`
(`DefaultGCodeEmitter::resolve_feedrate`, `crates/slicer-gcode/src/emit.rs`)
— and `FeedrateConfig` reads the **same raw keys** from the same config source
(`FeedrateConfig::from_raw_config`, `crates/slicer-runtime/src/run.rs`).

Measured shape (same defect as ticket 114's gyroid/lightning instance, verified
2026-09-05):

- `from_config` (`modules/core-modules/classic-perimeters/src/lib.rs`) reads
  `outer_wall_speed` / `inner_wall_speed` (fallbacks 30.0 / 45.0) and stores
  `outer_speed_factor = value / 50.0`, `inner_speed_factor = value / 50.0`.
- At defaults the value the module receives is `ResolvedConfig`'s 50.0
  (emitted via `to_config_map`, shadowing any manifest default), so
  `50 / 50 = 1.0` — **right by coincidence against the host base 60.0**
  (`FeedrateConfig::default`), i.e. `F = 60·60·1.0 = 3600` (60 mm/s).
- A user-set `outer_wall_speed = 30` reaches **both** the host base (30.0)
  and the module factor (`30 / 50 = 0.6`), emitting `F = 30·60·0.6 = 1080`
  (18 mm/s) — not the configured 30 mm/s. Same multiplication-through for
  `inner_wall_speed`, and for gap fill at the `gap_infill_speed / BASE_SPEED`
  site in the same module (line ~1145; `gap_infill_speed` is also a
  `FeedrateConfig` key).
- The wrong contract is pinned by a test: `speed_factor_from_config`
  (`modules/core-modules/classic-perimeters/tests/classic_perimeters_tdd.rs`)
  asserts `30/50 = 0.6` and `60/50 = 1.2` as the module's output contract.

**What the fix looks like (ticket 114's precedent):**

1. Wall and gap-fill paths carry factor `1.0`; delete the `BASE_SPEED`
   dependency from `crates/slicer-core/src/perimeter_utils.rs` if nothing else
   consumes it, and re-point the module's `from_config` reads at parse-only or
   drop them per the seam's needs.
2. Align `from_config`'s dead fallbacks (30.0 / 45.0) to the canonical
   defaults the host actually ships (60.0 / 60.0) while the reads exist, or
   drop the reads entirely (the keys' values reach the emitter directly).
3. Re-pin the contract tests at factor 1.0 and add a non-default end-to-end
   assertion (configured wall speed → exact F) since the unit-level factor
   test is what encoded the double-count.

**Care required (DEV-166 precedent):** wall *emission ordering* is delicate and
the classic-perimeters suite (24 tests) is the acceptance gate for anything
touching this module; verify geometry first, and treat a re-baseline as the
last resort. The pattern contrast that makes this a clean retirement: wave
overhangs' `speed_factor = wave_overhang_print_speed / bridge_speed`
(`modules/core-modules/wave-overhangs/src/lib.rs`) is a ratio of **two
different keys** that self-cancels against the bridge base — a legitimate use
that stays. Per-path modifiers (overhang quartiles, per-point profiles) also
stay.

Not a queue key (host-owned plumbing correction, ticket 113/114 class) — like
114 it changes no queue count. The related `support_ironing_speed`
adjudication (ticket 109, kept out of `SPEED_KEYS`, fallback aligned to 30.0)
was separately decided and stays as ruled.

## Answer
