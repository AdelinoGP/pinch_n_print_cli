# Requirements: core-retire

## Packet Metadata

- Grouped task IDs: `core/RETIRE (excluding flow)`
- Backlog source: `docs/specs/test-quality-remediation-plan.md`
- Packet status: `draft`
- Aggregate context cost: `M`

## Problem Statement

Two `slicer-core` tests carry names that claim behavioral protection their bodies cannot deliver, and ADR-0064 puts the burden of proof on the test. `clip_operation_variants_are_distinct` asserts `assert_ne!(ClipOperation::Union, ClipOperation::Difference)` over a fieldless enum with a derived `PartialEq`, so discriminant inequality holds for every possible compilation and no production defect can falsify it. `slice_closing_radius_zero_is_noop` sets `let r = 0.0_f32` and branches on `if r > 0.0`, which is statically false, so it compares `polygons.clone()` against `polygons` and never calls `apply_slice_closing_radius`; deleting the real gate in `crates/slicer-core/src/algos/prepass_slice.rs` leaves it green, and its own doc comment concedes it simulates the caller. This is one coherent slice because both are the same failure mode under `docs/22_test_quality.md` gate rule R1, whose legitimate-form column reads "none - retire instead", and because the second one's intent is worth preserving rather than discarding: no test in `slicer-core` drives the closing-radius gate today. The `flow.rs` retirement rows stay with `core-flow-consolidation`.

## In Scope

- Delete `clip_operation_variants_are_distinct` from the inline `tests` module of `crates/slicer-core/src/polygon_ops.rs`, taking that file's `#[test]` count from 21 to 20. The named regression input it fails to catch — a swapped match arm in `clip_polygons`, e.g. `Union => ClipType::Intersection` — is instead caught by `core-strengthen`'s strengthened `boolean_ops_produce_expected_presence_for_overlapping_squares`, which pins per-operation areas, bounds, component counts, and pairwise distinction.
- Delete `slice_closing_radius_zero_is_noop` from `crates/slicer-core/tests/triangle_mesh_slicer_tdd.rs`, taking that file's `#[test]` count from 12 to 11, while retaining the `// AC-7 / NEG-3` section banner, the `unit_square_expolygon` helper, and the `apply_slice_closing_radius` import — all still used by the surviving `slice_closing_radius_fuses_gap_within_two_r`.
- Add `prepass_slice_closing_radius_gate_applies_only_when_positive` to the `host-algos`-gated target `crates/slicer-core/tests/algo_prepass_slice_tdd.rs`, appended after the existing six tests, taking that file to 7. It re-homes the NEG-3 intent by driving the real gate in `execute_prepass_slice_single_layer_impl` rather than a local copy of it.
- Add the local fixture helpers that test needs to the same file: a `two_island_mesh()` whose `z = 5.0` cross-section is the unit square `x in [0.0, 1.0]` plus the unit square `x in [1.05, 2.05]`, both `y in [0.0, 1.0]`, and a region-map builder that interns one `ResolvedConfig` (the `ObjectMesh` and `ActiveRegion` literals in that fixture are watched types and take a `..Default::default()` rest) and inserts one `RegionPlan` under `RegionKey { global_layer_index: 0, object_id: "islands", region_id: 0, variant_chain: Vec::new() }`. Extend the file's `slicer_ir` import with `RegionKey`, `RegionMapIR`, `RegionPlan`, and `ResolvedConfig`.
- Assert both arms with independently derived expectations: `slice_closing_radius = 0.0` yields exactly 2 islands, combined area `200_000_000` unit-squared, bounds `(0,0)`-`(1,1)` mm and `(1.05,0)`-`(2.05,1)` mm; `slice_closing_radius = 0.04` yields exactly 1 island with bounds `(0,0)`-`(2.05,1)` mm, because `2r = 0.08` exceeds the 0.05 mm gap.
- Pass a populated `Option<&RegionMapIR>` in both arms so the contrast isolates the config value rather than the region map's presence, and set `slice_closing_radius = 0.0` explicitly, because `ResolvedConfig::default()` is `0.049` and `RegionMapIR::default()` pre-seeds `configs[0]` with it.
- During implementation, update only the `core` row inside the real §7 Ledger span in `docs/specs/test-quality-remediation-plan.md`: preserve accumulated prior evidence, keep state `partial`, record the three function-count deltas `21->20`, `12->11`, `6->7`, note that the target-scoped census is unchanged because no target is added or removed, and replace `core-retire` in the remaining-gap column with the `census-target-scope` program-level gap alongside the four later core packets.

## Out of Scope

- The `flow.rs` inline-versus-test consolidation rows in the audit's RETIRE item, owned by `core-flow-consolidation`.
- Any production implementation change, including `clip_polygons`, `ClipOperation`, `apply_slice_closing_radius`, `execute_prepass_slice_single_layer`, and the gate itself; all are read-only context.
- New fixtures on disk, new test files, new Cargo targets, feature definitions, manifests, public APIs, IR/WIT/schema changes, or generated artifacts.
- Editing `docs/specs/test-quality-remediation-census.json`. The census enumerates integration-test targets and this packet adds and removes none, so it stays byte-identical; extending it to cover lib targets is a program-level change recorded as a remaining gap for the final wave.
- Contour vertex-order or winding golden assertions for the sliced islands, and any OrcaSlicer parity claim about the closing-radius round-trip.
- Executing `core-strengthen`. Its strengthened boolean test is cited as the survivor rationale for AC-1, but this packet's acceptance never runs it and does not require it to be implemented first.
- Queue-row bookkeeping in the approved input plan; the parent batch controller owns queue updates. The implementation step owns only the §7 progress row.

## Authoritative Docs

- `docs/specs/test-quality-remediation-plan.md` - over 300 lines; delegated summary of row #10's approval, the three approved dispositions, the census decision, the §7 six-column ledger, and predecessor exports.
- `docs/adr/0064-existing-tests-retire-if-unjustified.md` - short; direct read of the Decision section, both carve-outs, and the census-reconciliation clause.
- `docs/22_test_quality.md` - direct read of §1, §2.1, §2.8, §3, §4 questions 1/2/6, and the §5 gate table rows R1/R2.
- `docs/21_data_defaults_and_fixtures.md` - direct read of §3 watchlist derivation and §4 waiver format.
- `docs/08_coordinate_system.md` - direct read of the 100 nm unit and `Point2::from_mm`.
- `docs/adr/0065-test-quality-gate-with-delayed-enforce-mode.md` - direct read; the gate stays report-mode until final-wave promotion.
- `docs/spec_packets/test-quality-remediation_09_core-strengthen/` - delegated predecessor summary; draft, independently `PREFLIGHT PASS`, with no dependency exports.

## Acceptance Summary

Reference, never copy, criteria from `packet.spec.md`.

- Positive: `AC-1` through `AC-5`.
- Negative: `AC-N1`. A retirement packet's characteristic silent failure is collateral deletion or reordering, so the roster check is the rejection case: all three edited sites — including the inline `tests` module of `crates/slicer-core/src/polygon_ops.rs` — must present their exact surviving names in file order, and neither retired name may resolve anywhere under `crates/`.
- Cross-packet impact: generation depends serially on row #9, which exports no API, file, fixture, schema, WIT type, manifest entry, or test target. This packet exports none. It shares the §7 `core` ledger row with `core-strengthen`; AC-5 deliberately does not require that packet's tokens, so neither imposes an implementation order on the other.

## Verification Commands

This is the authoritative full matrix; `packet.spec.md` lists only the closure-gate commands.

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `set -euo pipefail; if rg -q 'clip_operation_variants_are_distinct' crates/slicer-core/src/polygon_ops.rs; then exit 1; fi; n=$(rg -c '#\[test\]' crates/slicer-core/src/polygon_ops.rs); test "$n" -eq 20` | Prove the inline clip-op retirement and its 21 → 20 count | FACT pass/fail; exit code only |
| `set -euo pipefail; mkdir -p target; cargo test -p slicer-core --test triangle_mesh_slicer_tdd -- --exact slice_closing_radius_fuses_gap_within_two_r 2>&1 \| tee target/test-output.log >/dev/null; grep -Eq '^test result: ok\. 1 passed; 0 failed;' target/test-output.log` | Prove the survivor still compiles and passes after the neighbouring deletion, keeping the import and helper live | One anchored result line, one passed, zero failed |
| `set -euo pipefail; mkdir -p target; cargo test -p slicer-core --features host-algos --test algo_prepass_slice_tdd -- --exact prepass_slice_closing_radius_gate_applies_only_when_positive 2>&1 \| tee target/test-output.log >/dev/null; grep -Eq '^test result: ok\. 1 passed; 0 failed;' target/test-output.log` | Prove the re-homed contrast pair runs under the feature-correct invocation | One anchored result line, one passed, zero failed |
| `set -euo pipefail; cargo test -p slicer-core --features host-algos --test algo_prepass_slice_tdd -- --list 2>&1 \| grep -c ': test$'` | Registration check; a zero-match or unchanged count means the new test never compiled | FACT: integer equal to 7 |
| `set -euo pipefail; mkdir -p target; cargo xtask check-literals 2>&1 \| tee target/core-retire-literals.log >/dev/null` | Enforce the watched-struct literal rule after the new `ObjectMesh` and `ActiveRegion` fixture literals (both watched; `ResolvedConfig` is a documented scanner blind spot and is not) | FACT pass/fail; exit 0 |
| `set -euo pipefail; mkdir -p target; cargo xtask check-test-quality --report 2>&1 \| tee target/core-retire-quality.log >/dev/null` | Record report-mode findings without pretending the delayed enforcement gate is active | FACT pass/fail; exit 0 |
| AC-N1's roster command, reading all three edited sites directly | Prove no collateral deletion, rename, or reorder, and that neither retired name resolves under `crates/` | One `AC-N1 PASS` line, or the mismatching roster |
| AC-5's real-plan predicate command, reading `docs/specs/test-quality-remediation-plan.md` directly | Prove the accumulated `core` row records both retirements, the replacement, the three count deltas, its own validation command, and the updated gap | One `core-retire ledger predicate: PASS` line; an unedited row is expected to fail before implementation |
| `set -euo pipefail; mkdir -p target; cargo check --workspace --all-targets 2>&1 \| tee target/core-retire-check.log >/dev/null` | Compile all targets after the deletions and the new test | FACT pass/fail |
| `set -euo pipefail; mkdir -p target; cargo clippy --workspace --all-targets -- -D warnings 2>&1 \| tee target/core-retire-clippy.log >/dev/null` | Catch any import or helper left unused by a deletion | FACT pass/fail |

Every Cargo invocation against `algo_prepass_slice_tdd` uses `--features host-algos`; without it Cargo skips the target outright, so an explicit `--test algo_prepass_slice_tdd` fails with `requires the features: host-algos` and a bare crate-wide run silently omits it. `triangle_mesh_slicer_tdd` is ungated and needs no feature. All test runs tee combined output to `target/test-output.log` and are read from the log, never re-run for more output. No workspace test suite is required by this packet.

## Step Completion Expectations

- The two deletions and the addition are independent edits, but the ledger step runs last because it records counts that only exist once all three have landed.
- The ledger step preserves accumulated prior row content; AC-5 does not check for `core-strengthen`'s tokens, so preservation is verified by diff review rather than by the predicate.
- No step may satisfy a count assertion by deleting or renaming a neighbouring test; AC-N1's ordered rosters are the guard, and they must be re-derived from disk rather than copied from this document if the files change before implementation.
- The new test must be appended after `prepass_slice_caches_bottom_surface_footprint_across_layers`, because AC-N1 pins roster order.

## Context Discipline Notes

`crates/slicer-ir/src/slice_ir.rs` is far over 600 lines: read it by symbol only, never in full. `ResolvedConfig` has no hand-written struct definition — it is generated by `declare_resolved_config!` in `crates/slicer-ir/src/resolved_config.rs`, so grep the declaration lines rather than hunting for a `struct` keyword. Do not load `target/`, lockfiles, the census JSON, or OrcaSlicer sources. Cargo runs are delegated and return only the anchored result line or a bounded failure excerpt.
