---
status: implemented
packet: test-quality-remediation_12_core-brittle
task_ids:
  - 'core/BRITTLE'
---

# test-quality-remediation_12_core-brittle

## Goal

Resolve the three `slicer-core` timing guards through earn-their-keep review, keeping every guard with its correctness witness and recorded Benchy regression rationale unless migration to the existing criterion bench wins on the evidence.

## Problem Statement

The §5.1 `BRITTLE` row names three timing guards that protect real 3D Benchy regressions (118s MeshAnalysis from per-facet incremental union, O(layers) overhang sweep, ~28s Slice from per-layer footprint recomputation) but assert wall-clock thresholds, which are machine-dependent and therefore brittle in CI. The plan disposition is KEEP-review: move to a benchmark or deterministic accounting, or justify retention — never drop a perf guard without replacement. This packet is the one coherent slice that runs that review to a recorded decision for all three guards without touching production code or any other core surface.

## Architecture Constraints

- Test-only change: no production signature, threshold, branch, or output changes; all three test names are preserved and no test is added or removed.
- Thresholds are pinned, never loosened: `from_secs(3)`, `from_secs(1)`, and `cached_elapsed * 3 < uncached_elapsed * 2` stay exactly as authored (canonical parity supremacy applied to perf guards: loosening a threshold to make CI green is the timing analogue of loosening a tolerance).
- Feature gating is load-bearing two ways: the lib tests compile to zero under default features via the `host-algos` module gate in `crates/slicer-core/src/algos/mod.rs`, and the integration target is skipped outright without the feature via its `required-features` stanza (no inner cfg header), so every test command uses `--features host-algos`.
- Determinism: no new timing, sleep, ordering, or global-state assertions are added; the review records why each retained wall-clock guard is acceptable (documented regression input, large headroom, self-normalizing ratio).

## Data and Contract Notes

- IR/manifest contracts: none; no IR field, config key, manifest entry, or schema version is asserted or changed.
- WIT boundary: none; host-side tests with no guest build input.
- Determinism/scheduler constraints: the packet's subject is timing brittleness itself; the decision keeps deterministic correctness witnesses (`len() == 1200`, `is_empty`, ratio shape) alongside the retained wall-clock guards with recorded rationale.

## Locked Assumptions and Invariants

- All three test names survive with thresholds unchanged; no test is added, removed, or renamed.
- The 6-name integration roster holds in file order, optionally followed by `core-retire`'s planned 7th name (explicit FORWARD-DEP on that draft packet, name reconciled); this packet adds no name to the file.
- The 1200/1200/800 facet and 20-layer inputs stay exactly as authored; headroom figures cited in comments are the tests' own documented measurements, never recomputed here.

## Risks and Tradeoffs

- A retained absolute gate can still flake on a pathological CI runner; mitigation: the ledger records the headroom figures (50x/7x) and the ratio guard's self-normalizing property so a future flake is diagnosed as infrastructure, with the criterion bench named as the deferred migration path rather than deletion.
- Rationale-comment edits could drift from the code they describe; mitigation: comments name exact symbols and measured inputs already present in the test bodies, verified by the AC literal pins.
