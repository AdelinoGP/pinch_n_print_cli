---
status: implemented
packet: test-quality-remediation_03_core-bridge-dup-merge
task_ids:
  - core/DUP-CORE (bridge)
---

# test-quality-remediation_03_core-bridge-dup-merge

## Goal

Merge the textually duplicated supported-bridge rejection test in `crates/slicer-core/tests/bridge_false_site_gating_tdd.rs` into its named survivor while preserving every distinct unsupported-span and ungated-candidate guard exactly.

## Problem Statement

The Wave-0 audit confirmed `fully_supported_candidate_rejected_zero_bridge_area` (`crates/slicer-core/tests/bridge_false_site_gating_tdd.rs`) is textually identical to `solid_underneath_span_produces_no_bridge_area` in the same file: both build the same `(0.0, 0.0)–(10.0, 10.0)` bridge polygon via `region_with_bridge`, call `gate_bridge_areas_by_unsupported_span(&mut region, Some(&[bridge]))` over a fully supported span, and assert `region.bridge_areas.is_empty()`. The approved `core-bridge-dup-merge` slice (plan Packet Queue row #3) merges the duplicate into the plan-named survivor while preserving the five distinct guards — the exact difference-and-disjointness unsupported-span assertion, the ungated-stamping control, the `None` lower-layer clearing, and the empty-`Some(&[])` retention — without touching production code or any other core test.

## Architecture Constraints

- The exact edit surface is `crates/slicer-core/tests/bridge_false_site_gating_tdd.rs` alone; no production behavior or export changes.
- The single test binary is `bridge_false_site_gating_tdd`; `crates/slicer-core/Cargo.toml` carries `required-features = ["host-algos"]` for it, so every command uses `--features host-algos` to avoid feature-blind verification under the core-wave policy.
- Preserve the merge direction pinned by the plan §5.1 DUP-CORE row: survivor `solid_underneath_span_produces_no_bridge_area`, absorbed `fully_supported_candidate_rejected_zero_bridge_area`.
- Preserve all five distinct guards verbatim, including `unsupported_span_retains_bridge_area`'s exact `difference`/`intersection` assertions and its `// AC-2:` comment, the empty-start `bridge_areas` setup in `ungated_candidates_cannot_silently_return` (it proves the candidate originates from `assemble_bridge_areas` stamping, not the fixture), `no_lower_layer_clears_bridge_areas`' `None` clearing, and `existing_empty_lower_layer_retains_bridge_area`'s `Some(&[])` full retention.
- The surviving rejection witness keeps its exact `assert!(region.bridge_areas.is_empty());` — do not weaken to a non-empty or size-only check.
- No schema, WIT, coordinate conversion, scheduler, manifest, or WASM contract is touched. The test fixtures use `Point2::from_mm` only as pre-existing input construction; no unit conversion is authored by this packet.

<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.

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
