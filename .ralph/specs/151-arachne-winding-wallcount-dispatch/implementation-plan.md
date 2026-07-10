# Implementation Plan: 151-arachne-winding-wallcount-dispatch

## Execution Rules

- One atomic step at a time; each maps to gaps G1/G2/G7/G8/G9 (+ the wall_count
  wiring bug) from `docs/18_arachne_parity_audit.md`.
- TDD first (red gap tests exist); wall_count wiring precedes the count/winding
  gaps.
- Honor the context-discipline preamble; the per-step fields are the budget
  contract.

## Steps

### Step 1: Wire wall_count → max_bead_count = 2 × wall_count

- Gaps: wall_count wiring bug (prerequisite).
- Objective: register `wall_count`; read it in `arachne_params_from_config`; set
  `max_bead_count = 2 × wall_count`.
- Precondition: packet active (150 implemented).
- Postcondition: AC-1 — distinct Outer/Inner indices on a solid square equal
  `wall_count` (e.g. `{0,1,2}` for 3).
- Files read: `arachne-perimeters/src/lib.rs:108-225`.
- Files edit (≤3): `modules/core-modules/arachne-perimeters/src/lib.rs`,
  `modules/core-modules/arachne-perimeters/arachne-perimeters.toml`.
- Out-of-bounds: `crates/slicer-core/src/beading/**`.
- Dispatches: "Run `cargo test -p slicer-runtime --test arachne_parity_gaps --
  arachne_parity_wall_count_wires_max_bead_count --exact`; FACT."; "From
  `WallToolPaths.cpp:525`, confirm `max_bead_count = 2 * inset_count`; FACT."
- Context cost: `M`.
- Docs: `docs/15_config_keys_reference.md` (delegate `wall_count` entry).
- OrcaSlicer refs: `WallToolPaths.cpp:525` — delegate.
- Verification: the AC-1 packet-authored test.
- Exit condition: AC-1 green; DEVIATION_LOG entry drafted for the bug.

### Step 2: Register + wire wall_direction winding (G1)

- Gaps: G1.
- Objective: register `wall_direction` (enum, default `counter_clockwise`); add a
  shoelace-based normalization at emission (`lib.rs:467-497`) that reverses
  contour point order to the requested winding; holes wound opposite.
- Precondition: Step 1 landed (correct baseline).
- Postcondition: AC-2 + AC-N2 — flipping the key flips the outer signed-area sign;
  absent key preserves prior default winding.
- Files read: `arachne-perimeters/src/lib.rs:467-497`,
  `crates/slicer-ir/src/slice_ir.rs:1783-1792`.
- Files edit (≤3): `arachne-perimeters/src/lib.rs`, `arachne-perimeters.toml`.
- Out-of-bounds: `slicer-ir` (read-only — no shared-type change).
- Dispatches: "From `PerimeterGenerator.cpp:527-545`, contour vs hole winding
  rule under wall_direction; SUMMARY ≤200 words."; "Run G1 gap test; FACT."
- Context cost: `M`.
- Docs: none beyond the Orca dispatch.
- OrcaSlicer refs: `PerimeterGenerator.cpp:527-545`.
- Verification: G1 gap test + `arachne_parity` (AC-N2 default preserved).
- Exit condition: AC-2 green, AC-N2 green.

### Step 3: only_one_wall_first_layer (G2)

- Gaps: G2.
- Objective: register the key; on layer 0, force a single wall (reduce the
  effective wall count / max_bead_count to yield one emitted index).
- Precondition: Steps 1-2 landed.
- Postcondition: AC-3 — layer 0 emits `{0}`; layer 1 emits the full count.
- Files read: `arachne-perimeters/src/lib.rs:108-225`.
- Files edit (≤3): `arachne-perimeters/src/lib.rs`, `arachne-perimeters.toml`.
- Out-of-bounds: beading engine.
- Dispatches: "Run G2 gap test; FACT."
- Context cost: `S`.
- Docs: none.
- OrcaSlicer refs: `PerimeterGenerator.cpp:2137-2139`.
- Verification: G2 gap test.
- Exit condition: AC-3 green.

### Step 4: overhang_reverse odd-layer reversal (G7)

- Gaps: G7.
- Objective: read `overhang_reverse`/`detect_overhang_wall`; register
  `overhang_reverse_threshold` (`float_or_percent`, packet-150 type); when
  `detect_overhang_wall=false` && `overhang_reverse=true`, reverse wall direction
  on odd layers (compose with G1: base winding XOR odd-layer reversal).
- Precondition: Step 2 landed (winding machinery exists).
- Postcondition: AC-4 — odd-layer outer signed area opposite the non-reversed run.
- Files read: `arachne-perimeters/src/lib.rs:295-306,467-497`.
- Files edit (≤3): `arachne-perimeters/src/lib.rs`, `arachne-perimeters.toml`.
- Out-of-bounds: beading engine.
- Dispatches: "From `PerimeterGenerator.cpp:58-98,422-429`, the odd-layer reversal
  + steep-mark rule; SUMMARY ≤200 words."; "Run G7 gap test; FACT."
- Context cost: `M`.
- Docs: none.
- OrcaSlicer refs: `PerimeterGenerator.cpp:58-98,422-429`.
- Verification: G7 gap test.
- Exit condition: AC-4 green; D-104c marked closed.

### Step 5: wall_maximum_resolution/deviation (G9)

- Gaps: G9.
- Objective: register both keys; read and feed
  `smallest_line_segment_squared = wall_maximum_resolution²`,
  `allowed_error_distance_squared = wall_maximum_deviation²` (mm²; no ÷100).
- Precondition: independent of Steps 2-4.
- Postcondition: AC-6 (registration flips G9 gap test) + AC-6b (wiring, packet
  test).
- Files read: `arachne-perimeters/src/lib.rs:180-225`,
  `crates/slicer-core/src/arachne/pipeline.rs:145-208`.
- Files edit (≤3): `arachne-perimeters/src/lib.rs`, `arachne-perimeters.toml`.
- Out-of-bounds: `OrcaSlicerDocumented/**`.
- Dispatches: "From `WallToolPaths.cpp:487-503,702-719`, do wall_maximum_*
  REPLACE or supplement meshfix_* for the wall simplify gate? SUMMARY ≤200
  words." (resolves the [FWD]); "Run G9 gap test + `cargo test -p
  arachne-perimeters --lib -- wall_maximum_resolution_wired`; FACT."
- Context cost: `M`.
- Docs: `docs/08_coordinate_system.md` (mm² note).
- OrcaSlicer refs: `WallToolPaths.cpp:487-503,702-719`.
- Verification: G9 gap test + wiring test.
- Exit condition: AC-6 + AC-6b green.

### Step 6: Spiral vase forces classic (G8)

- Gaps: G8.
- Objective: extract a spiral-vase bool from `config_source` in
  `execution_plan_live.rs` (like `wall_generator`); thread into
  `dedup_same_claim_modules`; force `classic-perimeters` for the
  `perimeter-generator` claim when spiral is active.
- Precondition: independent of Steps 1-5 (different crates).
- Postcondition: AC-5 (G8 gap test — the selection path mentions/handles spiral)
  and AC-N1 (fallback fires only when spiral active).
- Files read: `crates/slicer-scheduler/src/execution_plan.rs:180-275`,
  `crates/slicer-wasm-host/src/execution_plan_live.rs:201-216`.
- Files edit (≤3): `crates/slicer-scheduler/src/execution_plan.rs`,
  `crates/slicer-wasm-host/src/execution_plan_live.rs`.
- Out-of-bounds: the module.
- Dispatches: "Run G8 gap test + `cargo test -p slicer-scheduler --test contract
  -- spiral_vase_arachne_dispatch`; FACT."
- Context cost: `M`.
- Docs: `docs/04_host_scheduler.md` (delegate selection section).
- OrcaSlicer refs: `LayerRegion.cpp:138-141`.
- Verification: G8 gap test + the contract test (AC-N1).
- Exit condition: AC-5 + AC-N1 green.

### Step 7: Docs, deviation closure, guest freshness

- Gaps: bookkeeping.
- Objective: update docs/15 (six keys), docs/04 (spiral fallback), docs/18
  (G1/G2/G7/G8/G9 closed), DEVIATION_LOG (close D-104c; add wall_count entry).
- Precondition: Steps 1-6 green.
- Postcondition: every Doc Impact grep hits.
- Files read/edit (docs only, sequential): the four docs.
- Dispatches: "Run each Doc Impact grep; FACT all-hit / misses."
- Context cost: `S`.
- Verification: Doc Impact grep suite; `cargo xtask build-guests --check` clean.
- Exit condition: all greps hit; guests fresh.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | M | wall_count wiring (baseline for 2-4) |
| Step 2 | M | winding machinery (new; no reuse) |
| Step 3 | S | first-layer single wall |
| Step 4 | M | overhang reversal (composes with Step 2) |
| Step 5 | M | G9 register + wire |
| Step 6 | M | spiral dispatch (different crates) |
| Step 7 | S | docs + closure |

Aggregate: `M` (one M step at a time; no L).

## Packet Completion Gate

- All 7 steps complete; every exit condition met.
- G1/G2/G7/G8/G9 gap tests green; the wall_count packet test green; other gap
  tests (G3/G4/G5/G6/G10) unchanged (G4/G5/G6 already green from 150).
- 14 `arachne_parity.rs` locks green (shifts validated as wall_count-correct).
- `cargo check`/`clippy --workspace --all-targets` clean;
  `cargo xtask build-guests --check` clean.
- Doc Impact greps hit; D-104c closed; wall_count bug logged.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed AC command from `packet.spec.md`.
- Workspace gate via sub-agent: `cargo xtask test --summary --workspace` — FACT
  PASS/FAIL + failing-test list only.
- Record any wall-count baseline shift and its validation explicitly before
  `status: implemented`.
- Confirm implementer peak context < 70%; log if not.
