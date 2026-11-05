---
status: implemented
packet: test-quality-remediation_14_core-parity
task_ids:
  - core/PARITY
---

# test-quality-remediation_14_core-parity

## Goal

Add the nine core parity assertions named in plan §5.1 (PARITY row) as test-only additions in `crates/slicer-core/tests/`, preserving every existing exact canonical pin, and record the packet's progress in plan §7's `core` ledger row only.

## Problem Statement

The plan §5.1 PARITY row (`core/PARITY`, disposition FIX — additive; never remove exact pins) names nine parity surfaces in `crates/slicer-core/tests/` whose current assertions are weaker than the canonical contract they protect: the beding side-table radius boundary is exercised only by loose per-vertex contract checks and never at the exact inclusive radius; the transitions perpendicular-foot test only covers horizontal source segments where linear interpolation coincides with the foot; the rib-split test leaves an exact-position assertion as a `let _ =` placeholder; node distances are locked by bbox/non-negativity bounds rather than the F5-fix semantics the builder now implements; the dumbbell topology test asserts only presence (`>= 4` junctions) instead of the dissolve/not-dissolve topology; the limited-beading cap tests have no at-cap (`bead_count == max_bead_count`) case pinning the single centre-sentinel odd shape; junction reachability is asserted via loose ordering/span inequalities while the file documents exact interpolated positions; the arachne module-fallback contract pins only 2 of the 7 wired-key defaults; and the postprocess-order test's "old order" branch is degenerate (2-junction line — simplify is a no-op). This packet adds the missing exact assertions, one file per step, preserving every existing exact pin and touching no production code.

## Architecture Constraints

- **Additive-only, test-only:** no production edit, no threshold/tolerance change, no exact-pin removal. Every step adds `#[test]` functions (or replaces the explicitly-marked `let _ = max_deviation_x; // placeholder` line with a real assertion); existing assertion bodies are not weakened.
- **Feature-correct invocation:** the eight arachne files are auto-discovered integration targets gated in-file by `#![cfg(feature = "host-algos")]`; `beading_limited` is an explicit `[[test]]` stanza with no `required-features`. All AC runs use `--features host-algos`; AC-N1 proves the gate matters. (Census: all nine targets recorded with `required: []`; the gates live in the files, not the manifest.)
<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.
- Struct-literal churn gate (`docs/21_data_defaults_and_fixtures.md`): new test literals of watched types (`SkeletalTrapezoidationGraph`, `STVertex`, `STHalfEdge`, `Beading`, `ExtrusionLine`, `ArachneParams`, `Point2`) must use `..` rest (FRU) or an `// exhaustive:` waiver; the touched files' existing FRU style is the template. `cargo xtask check-literals` runs before committing.
- Test-quality gate (`docs/22_test_quality.md`, report mode): every new assertion must be independently falsifiable — exact value/array pins, no self-referential oracles. Where a measured baseline is required (items 5, 7), the measured values are pinned as literal expected arrays with the canonical claim documented in a comment.
- Red/parity discipline: assertions encode canonical contract; where current behavior is documented as divergent (item 2's non-retained source projection), the test pins exactly what the implementation guarantees and documents the divergence in a comment — it never pins a wrong value as canonical.

## Data and Contract Notes

- IR/manifest contracts: none — test-only additions; no IR schema, manifest, or WIT surface changes.
- WIT boundary: none; no guest WASM artifacts are touched (wasm-staleness considerations intentionally omitted).
- Determinism/scheduler constraints: the added tests must stay deterministic — no timing, no iteration-order dependence; all fixtures are hand-built or fixed polygons.

## Locked Assumptions and Invariants

- Canonical parity supremacy: existing exact pins are never removed or loosened; all sixteen pre-existing test-name anchors in the nine files must still resolve (AC-N2).
- The §5.1 PARITY row is authoritative over the older audit text; items are grounded to on-disk symbols only.
- Ambiguity commitments: item 6 = at-cap odd-centre cap boundary in `beading/limited.rs`; item 8 = local fallback contract in `arachne_pipeline.rs` (no Orca counterpart).
- The plan §7 `core` row is the only ledger mutation; `Packet Queue` and the census never change.

## Risks and Tradeoffs

- Measured-baseline pins (S5, S7's `xs[1]`): the exact numbers are captured at implementation time; the canonical *claims* (dissolve rule, interpolation formula) are fixed by the file docs — if a measured value contradicts the documented expectation, the implementation must stop and record the divergence in the ledger rather than silently pin it.
- F-family (S2/S3/S4) pass/fail state: grounded green against the landed fixes (`insert_node` faithful port, F5 fix in `make_node_vd`/`make_rib`); if any assertion proves red at implementation, the step records the measured state (self-captured baseline with divergence comment) and the ledger row notes it — the ACs must then still pass with the pinned baseline.
- Shared ledger row: later core-wave packets may touch the same §7 row; AC-10 accepts any `core` row carrying this packet's tokens.
