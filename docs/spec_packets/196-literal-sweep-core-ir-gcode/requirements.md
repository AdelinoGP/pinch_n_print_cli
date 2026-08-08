# Requirements: 196-literal-sweep-core-ir-gcode

## Packet Metadata

- Grouped task IDs: `TASK-318`
- Backlog source: `docs/07_implementation_status.md` (new row added at completion; TASK-318 was allocated by `docs/specs/struct-literal-churn-gate-plan.md` — re-derive the highest existing TASK id before writing the row)
- Packet status: `draft`
- Aggregate context cost: `M`

## Problem Statement

Queue row #3 of `docs/specs/struct-literal-churn-gate-plan.md`. The plan's measured churn (165-file sweep in `a579fc18` after `Point3WithWidth` gained a field) comes from exhaustive watched-type literals in test code. Packets 194/195 built the gate and the FRU bases; this packet is the first of three area sweeps: drive `cargo xtask check-literals` to zero violations for `slicer-ir`, `slicer-core`, and `slicer-gcode` without changing what any test asserts. It is one coherent slice because the three crates form the IR-to-G-code data spine, share the same fixture decisions (see `design.md`), and their combined violation surface is the smallest of the three sweep areas.

## In Scope

- Convert exhaustive watched-type struct literals to FRU (`..Default::default()` or `..<fixture>()`) in:
  - `crates/slicer-ir/tests/**` and the `#[cfg(test)]` mod(s) in `crates/slicer-ir/src/slice_ir.rs`;
  - `crates/slicer-core/tests/**`, `crates/slicer-core/benches/**`, and `#[cfg(test)]` mods in `crates/slicer-core/src/**` (candidate files measured 2026-08-07, re-derive from the Step-1 report: `lib.rs`, `perimeter_utils.rs`, `arachne/generate_toolpaths.rs`, `arachne/remove_small.rs`);
  - `crates/slicer-gcode/tests/**` and the `#[cfg(test)]` mod in `crates/slicer-gcode/src/emit.rs`.
- Conversion rule (locked decision 2 of the plan; `docs/21_data_defaults_and_fixtures.md` is the normative wording): keep explicitly-meaningful non-default fields spelled (e.g. `flow_factor: 1.0` where the test exercises flow), OMIT fields equal to the base's value, append `..Default::default()` or `..<fixture>()`. Never spell-all-fields-plus-FRU (`clippy::needless_update`). Never change a test's assertions or expected values.
- Per-test-file helper fns are prime targets: `fn point3` / `fn junction` helpers (measured 2026-08-07 in `slicer-core` tests `stitch.rs`, `simplify.rs`, `remove_small.rs`, `region_order_tdd.rs`, `arachne_postprocess_order.rs`, `arachne_simplify_distance_gates.rs`, `arachne_remove_small_per_line_min_width.rs`; `slicer-gcode` tests `emit_tool_guard_tdd.rs`, `estimator.rs`, `finalization_aware_travel_tdd.rs`) — `Point3WithWidth` derives `Default`, so these convert to plain FRU.
- `slicer-gcode` `PrintEntity` sites (8 test files measured 2026-08-07; re-derive): add `slicer-sdk = { path = "../slicer-sdk", features = ["test"] }` to `[dev-dependencies]` in `crates/slicer-gcode/Cargo.toml` and convert via `slicer_sdk::test_support::fixtures::print_entity_base` (packet-195 export).
- `slicer-ir` class-b sites (`fn make_entity` `PrintEntity` helpers in `entity_id_invariants_tdd.rs` and `ir_validation_tdd.rs`) and the single `slicer-core` `WallLoop` helper (`fn make_wall` in `wall_sequence_reorder_tdd.rs`): keep the helper's literal exhaustive with a `// exhaustive: <reason>` waiver — see `design.md` for the grounded dev-dep rationale and the exact reason text.
- Waivers where exhaustiveness IS the test's intent: `slicer-ir` carrier/roundtrip tests (`extrusion_line_roundtrip.rs`, `point3_overhang_distance_roundtrip.rs`, `point3_overhang_quartile_roundtrip.rs`) assert every field travels; their exhaustive literals get waivers with a carrier-test reason, never FRU.
- Baseline capture before any edit (suite summaries, assert-macro count, `#[test]` count) into `target/sweep-196-*` scratch files; post-sweep invariance proof against them.
- `cargo xtask build-guests --check` at close (and rebuild without `--check` if `STALE:`), because `crates/slicer-ir/src` and `crates/slicer-core/src` are guest-WASM input paths.

## Out of Scope

- Any assertion, expected-value, tolerance, or fixture-semantic change — the repo rule "never weaken assertions to get green" applies verbatim.
- Adding `Default` (impl or derive) to `PrintEntity`, `WallLoop`, `Diagnostic` (either crate), `DeferredRetract`, `DeferredTravelMove` (packet-195 negative locks).
- Production `src/` literals outside `#[cfg(test)]` mods — exhaustive on purpose (plan's counter-evidence: compiler-enforced propagation checkpoints).
- Sweeps of `slicer-runtime`, `slicer-scheduler`, `slicer-wasm-host`, `pnp-cli` (packet 197), `slicer-sdk`, `modules/core-modules` (packet 198), and residue crates outside all three sweep areas (packet 199's workspace-wide flip must absorb these; reviewer-verified 2026-08-07: `slicer-helpers` 0, `slicer-model-io` 1, `slicer-macros` 1 test files with watched-type literals — earlier broader-grep figures of 3/2/1 counted unwatched `MeshIR`-style matches).
- Wiring `check-literals` into `cargo xtask test` or CLAUDE.md required-before-commit (packet 199).
- Adding an sdk dev-dep to `slicer-ir` or `slicer-core` (rejected on grounded feature-unification evidence; see `design.md` and the `[FWD]` question).
- Editing `xtask/src/check_literals.rs` or its tests (packet 194 owns the tool; a tool bug found here is a deviation, not a local patch).

## Authoritative Docs

- `docs/specs/struct-literal-churn-gate-plan.md` - short; direct read.
- `docs/21_data_defaults_and_fixtures.md` - authored by packet 194, size unknown at authoring; direct read at implementation time; delegate a SUMMARY if it exceeds 300 lines.
- `CLAUDE.md` §Test Discipline (feature-gated slicer-core targets; tee rule), §Guest WASM Staleness - direct read of the named sections only.
- `docs/adr/0054-host-side-test-support-crate.md` + `docs/adr/0004-test-support-lives-in-slicer-sdk.md` - amended by packet 195 ("single IR-fixture home"); delegate a FACT check only if the fixture-consumption decision needs re-confirmation.

## Acceptance Summary

Reference, never copy, criteria from `packet.spec.md`.

- Positive: `AC-1` through `AC-6`.
- Negative: `AC-N1` through `AC-N3`.
- Cross-packet impact: AC-1's clean area is one third of packet 199's workspace-wide enforcement precondition; AC-5's dev-dep is the pattern packet 197 repeats for `pnp-cli`. Waivers added here (count re-derived at 199 time via `rg -c '// exhaustive:'`) are inputs to 199's final audit.

## Verification Commands

This is the authoritative full matrix; `packet.spec.md` lists only 2-3 gate commands.

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo xtask check-literals --report crates/slicer-ir crates/slicer-core crates/slicer-gcode \| tee target/sweep-196-report.txt \| tail -1` | Step-1 enumeration of violations driving the sweep (report mode always exits 0) | FACT: final summary line |
| `cargo xtask check-literals crates/slicer-ir; test $? -eq 0 && echo PASS` | per-crate green after the `slicer-ir` step | FACT PASS/FAIL |
| `cargo xtask check-literals crates/slicer-gcode; test $? -eq 0 && echo PASS` | per-crate green after the `slicer-gcode` step | FACT PASS/FAIL |
| `cargo xtask check-literals crates/slicer-core; test $? -eq 0 && echo PASS` | per-crate green after the `slicer-core` step | FACT PASS/FAIL |
| `cargo xtask check-literals crates/slicer-ir crates/slicer-core crates/slicer-gcode; test $? -eq 0 && echo PASS` | AC-1 area gate | FACT PASS/FAIL |
| AC-2 pipe-suffixed command (see `packet.spec.md`) | `slicer-ir` suite green + count-invariant vs baseline | FACT PASS/FAIL; on FAIL, `grep -E 'FAILED\|panicked at' -C 5 target/test-output.log` SNIPPETS ≤20 lines |
| AC-3 pipe-suffixed command (see `packet.spec.md`) | `slicer-core` suite green under `--features host-algos` + count-invariant | FACT PASS/FAIL; same failure grep |
| AC-4 pipe-suffixed command (see `packet.spec.md`) | `slicer-gcode` suite green + count-invariant | FACT PASS/FAIL; same failure grep |
| AC-5, AC-N1, AC-N2, AC-N3 pipe-suffixed commands (see `packet.spec.md`) | structural guards | FACT PASS/FAIL |
| `cargo xtask build-guests --check; test $? -eq 0 && echo PASS` | AC-6 guest freshness after possible `slicer-ir`/`slicer-core` src (cfg-test) edits; rebuild without `--check` first if `STALE:` | FACT PASS/FAIL |
| `cargo check --workspace --all-targets` | workspace compile gate incl. all test/bench targets | FACT pass/fail |
| `cargo clippy --workspace --all-targets -- -D warnings` | lint gate; also catches `clippy::needless_update` from wrong conversions | FACT pass/fail |

All `cargo test` invocations tee to `target/test-output.log` (CLAUDE.md rule); inspect the log, never re-run for output.

## Step Completion Expectations

- Step 1 writes the shared scratch state every later step consumes and the close step diffs against: `target/sweep-196-report.txt` (violation list), `target/sweep-196-{slicer-ir,slicer-core,slicer-gcode}-baseline.txt` (sorted, time-stripped `test result` multisets), `target/sweep-196-assert-baseline.txt`, `target/sweep-196-testattr-baseline.txt`. These are implementation-time scratch files, never committed and never quoted into the packet as frozen counts.
- The `slicer-core` baseline and post runs MUST both use `--features host-algos --no-fail-fast`; a binary-count drop between them means the run was blind, never that tests disappeared (CLAUDE.md rule).
- Sweep steps (2-4) are order-independent of each other but all follow Step 1 and precede Step 5.

## Context Discipline Notes

- Never open `target/sweep-196-report.txt` in full if large; grep it per-crate (`grep '^crates/slicer-ir/' ...`).
- `crates/slicer-ir/src/slice_ir.rs` is >2000 lines: ranged reads around reported line numbers only.
- Do not open packet 194's or 195's `design.md`/`implementation-plan.md`; their exports are restated here and in `design.md`.
