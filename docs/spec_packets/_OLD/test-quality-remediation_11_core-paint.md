---
status: implemented
packet: test-quality-remediation_11_core-paint
task_ids:
  - 'core/PAINT'
---

# test-quality-remediation_11_core-paint

## Goal

Repair the eight `host-algos`-gated paint unit tests so every short-circuit, prune, propagation, and probe claim is falsifiable by a real production behavior while keeping all eight names and their distinct intents.

## Problem Statement

The §5.1 `PAINT` row names eight `host-algos`-gated paint unit tests whose current assertions cannot falsify the production behavior they name: three `driver_v2_*` short-circuit tests assert output equality without proving the input was nonempty enough to matter; `prune_one_arc_preserves_border_nodes` ends in no-panic comments with an explicit MAY on the `deleted` outcome; `from_colored_lines_twin_propagation_is_question_mark` claims `?` propagation from a single valid-input `is_ok`; and three dependency probes range from a pure third-party builder call to a meaningful color round-trip. This packet is the one coherent slice that makes each of the eight falsifiable or records the evidenced reason it stays weak, without touching production code or any other core surface.

## Architecture Constraints

- Test-only change: no production signature, default, branch, or output changes; all eight existing names are preserved and exactly one new Err-test name is added.
- Feature gating is load-bearing: the module gate at `crates/slicer-core/src/algos/mod.rs` compiles all eight existing tests to zero under default features, so every test command uses `--features host-algos --lib` with substring filters; the bare-feature negative control in AC-N1 guards the silent-green hazard.
- Struct-literal discipline: any new fixture literal of a watched `pub` struct with ≥5 named fields uses a `..` rest or an `// exhaustive:` waiver; `ResolvedConfig` is a documented scanner blind spot, not a watched literal, per the `core-retire` grounding.
- Coordinate units: geometry-adjacent only for the prune `pt(0,0)`/`pt(100,0)` fixture, which is preserved verbatim, not recomputed.
- Determinism: no timing, sleep, ordering, or global-state assertions are added; the driver equality witnesses are structural (`len`, region count, polygon equality), not pointer identity.

## Data and Contract Notes

- IR/manifest contracts: none; no IR field, config key, manifest entry, or schema version is asserted or changed.
- WIT boundary: none; host-side unit tests with no guest build input.
- Determinism/scheduler constraints: none; no scheduler, ordering, or timing surface is touched.

## Locked Assumptions and Invariants

- All eight existing test names survive; the single added Err-test name is fixed by AC-3.
- The two-node prune fixture (`Border` arc from node 0 to node 1) and the square `synthetic_square_input` are preserved; strengthening adds assertions, not new geometries.
- The `42`/`0` color distinction, the `is_primary` existence claim, and the driver polygon-equality witnesses remain independently expected literals, never derived from the code under test.

## Risks and Tradeoffs

- The twin Err input depends on the `to_i32` overflow guard (`|v| > i32::MAX` checked before the zero-length skip, so `&[]` and degenerate in-range segments stay `Ok`); mitigation: the discovery dispatch re-derives the guard before the twin edit, and the fixed `2_147_483_648` coordinate (`i32::MAX + 1`, not a magic number) keeps the intent readable if the limit ever moves.
- Prune post-state derivation may show the current MAY comment was right to hedge (e.g. `delete_arc` clears both endpoints); mitigation: pin whatever the implementation deterministically does, verified by running the single filter, rather than asserting a preferred semantic.
- Probe waiver may be misread as permission for future success-only tests; mitigation: the waiver names the single protected build surface and cites the earn-their-keep decision, and the gate step checks the touched scope stays at zero unwaived findings.
