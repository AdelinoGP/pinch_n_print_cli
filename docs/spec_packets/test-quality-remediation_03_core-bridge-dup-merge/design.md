# Design: core-bridge-dup-merge

## Controlling Code Paths

- Primary code path: existing public `gate_bridge_areas_by_unsupported_span` and `assemble_bridge_areas` in `crates/slicer-core/src/algos/prepass_slice.rs`; no function bodies change and this packet does not edit that file.
- Neighboring tests/fixtures: integration tests only in `crates/slicer-core/tests/bridge_false_site_gating_tdd.rs` (115 lines, six `#[test]` fns plus the `square`/`region_with_bridge` helpers). There is no inline twin in production source for this target.
- OrcaSlicer comparison: not applicable; this test-only merge does not port canonical code.

## Architecture Constraints

- The exact edit surface is `crates/slicer-core/tests/bridge_false_site_gating_tdd.rs` alone; no production behavior or export changes.
- The single test binary is `bridge_false_site_gating_tdd`; `crates/slicer-core/Cargo.toml` carries `required-features = ["host-algos"]` for it, so every command uses `--features host-algos` to avoid feature-blind verification under the core-wave policy.
- Preserve the merge direction pinned by the plan §5.1 DUP-CORE row: survivor `solid_underneath_span_produces_no_bridge_area`, absorbed `fully_supported_candidate_rejected_zero_bridge_area`.
- Preserve all five distinct guards verbatim, including `unsupported_span_retains_bridge_area`'s exact `difference`/`intersection` assertions and its `// AC-2:` comment, the empty-start `bridge_areas` setup in `ungated_candidates_cannot_silently_return` (it proves the candidate originates from `assemble_bridge_areas` stamping, not the fixture), `no_lower_layer_clears_bridge_areas`' `None` clearing, and `existing_empty_lower_layer_retains_bridge_area`'s `Some(&[])` full retention.
- The surviving rejection witness keeps its exact `assert!(region.bridge_areas.is_empty());` — do not weaken to a non-empty or size-only check.
- No schema, WIT, coordinate conversion, scheduler, manifest, or WASM contract is touched. The test fixtures use `Point2::from_mm` only as pre-existing input construction; no unit conversion is authored by this packet.

<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.

## Code Change Surface

- Selected approach: apply the pre-grounded survivor map — delete the one `#[test]` fn `fully_supported_candidate_rejected_zero_bridge_area` whose body is textually identical to `solid_underneath_span_produces_no_bridge_area` (same `Some(&[bridge])` fully supported span, same empty assertion) — and touch nothing else; the five distinct guards, both helpers, and all imports stay.
- Exact functions/types/fields/tests: delete `fully_supported_candidate_rejected_zero_bridge_area`; retain `solid_underneath_span_produces_no_bridge_area`, `unsupported_span_retains_bridge_area`, `ungated_candidates_cannot_silently_return`, `no_lower_layer_clears_bridge_areas`, `existing_empty_lower_layer_retains_bridge_area`, helpers `square` and `region_with_bridge`, and imports `assemble_bridge_areas`, `gate_bridge_areas_by_unsupported_span`, `difference`, `intersection`, `BridgeRegion`, `ExPolygon`, `ObjectSurfaceData`, `Point2`, `Polygon`, `SlicedRegion`, `SurfaceClassificationIR`.
- Rejected alternatives: renaming the survivor instead of deleting (the plan pins the survivor name; renaming would churn AC discovery commands for no behavioral gain); merging any of the five distinct guards into parameterized forms (each asserts a materially different input/assertion contract — unsupported-span difference, ungated stamping control, `None` clearing, empty-slice retention — and merging would weaken a distinct regression input); strengthening oracles (owned by `core-strengthen`/`core-parity`, not this packet); touching production `prepass_slice.rs` (outside the approved test-only scope).

## Files in Scope (read + edit)

- `crates/slicer-core/tests/bridge_false_site_gating_tdd.rs` - role: sole edit surface, the duplicate's owner; expected change: delete the one duplicate `#[test]` fn only.
- `docs/specs/test-quality-remediation-plan.md` §7 only - role: mandatory program bookkeeping; expected change: update only the `core` ledger row with partial bridge status, the survivor map, actual validations, and the remaining `core-region-mapping-dup-merges` gap.

## Read-Only Context

- `crates/slicer-core/tests/bridge_false_site_gating_tdd.rs` - lines 1–115 - current six-test inventory and the textually identical duplicate pair.
- `crates/slicer-core/Cargo.toml` - lines 113–117 only - the `bridge_false_site_gating_tdd` `[[test]]` stanza with `required-features = ["host-algos"]`.
- `docs/specs/test-quality-remediation-plan.md` - ranges §1, §4, §5.1, §6, §7 Ledger, Packet Queue/continuation-approval paragraphs only.

## Out-of-Bounds Files

- All `crates/slicer-core/src/**`, including `algos/prepass_slice.rs`, `flow.rs`, and `lib.rs`.
- All other `crates/slicer-core/tests/*` files, including `flow_tdd.rs`.
- Parent plan sections other than the implementation-time §7 Ledger edit explicitly allowed above, `docs/07_implementation_status.md`, canonical backlog, other packet directories (including `core-flow-consolidation`), generated artifacts, `target/`, and `Cargo.lock`.
- `OrcaSlicerDocumented/...` - no inspection is applicable; if implementation discovers a parity claim requiring external inspection, stop and request a delegated reference lookup rather than inventing a path.

## Expected Sub-Agent Dispatches

- Question: confirm pre-edit discovery is exactly six tests including the duplicate and post-edit exactly five with the absorbed name absent; scope: `crates/slicer-core/tests/bridge_false_site_gating_tdd.rs`; return: `FACT` with the two counts; purpose: survivor-map proof and census reconciliation.
- Question: run the feature-correct exact target checks and gates; scope: commands in `requirements.md`; return: `FACT` pass/fail, with at most 20 failure lines; purpose: implementation validation. (No generation-time test execution.)

## Data and Contract Notes

- IR/manifest contracts: none.
- WIT boundary: none.
- Determinism/scheduler constraints: none; the retained guards keep deterministic fixture geometry (`square` spans in millimetres via `Point2::from_mm`) and exact polygon set assertions.

## Locked Assumptions and Invariants

- The survivor is `solid_underneath_span_produces_no_bridge_area`; the absorbed name is removed from the file and from discovery.
- Post-merge discovery is exactly five tests; the one-test delta is the accounted census retirement, recorded in the ledger row rather than frozen here.
- All five distinct guards keep their exact assertion strength: empty-set rejection, exact `difference` equality plus `intersection` disjointness, non-empty-stamped-then-emptied-by-gating control, `None` clearing, and `Some(&[])` full retention.
- No new public symbol is introduced and no net-new export is expected.

## Risks and Tradeoffs

- A careless `#[cfg(test)]`-block or helper deletion could orphan imports or helpers; the change is a single-function deletion and the helper/import inventory is pre-verified as still used by the five survivors.
- Merging by test name alone could lose a distinct input; the pre-grounded survivor map shows the duplicate is textually identical (same fixture helper, same span argument, same empty assertion), and the five survivors are provably distinct in their span shapes (`None`, `Some(&[])`, partial anchor, empty-start stamping control).
- Feature flags can make a narrow run appear green without compiling the intended target; `--features host-algos` and exact-name guards are mandatory in every command.

## Context Cost Estimate

- Aggregate: `S`
- Largest step: `S`
- Highest-risk dispatch and required return format: pre/post discovery census, `FACT` with two counts.

## Open Questions

None. `[FWD]` None — the survivor name and merge direction are pinned by the plan; no implementer choice is required. `[BLOCK]` If the duplicate is discovered to differ textually from the survivor at implementation time (divergent fixture, span argument, or assertion), stop the packet rather than silently changing either body; that would be a scope-changing premise error in the plan.