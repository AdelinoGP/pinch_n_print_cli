# Design: core-brittle

## Controlling Code Paths

- Primary code path: `compute_xy_footprint` batched-union path guarded by the 1200-facet 3s test in `crates/slicer-core/src/algos/mesh_analysis.rs`; `annotate_overhangs` band-partition sweep guarded by the 1200-layer 1s test in `crates/slicer-core/src/algos/overhang_annotation.rs`; `execute_prepass_slice_single_layer` vs `execute_prepass_slice_single_layer_with_cache` guarded by the 20-layer 1.5x ratio test in `crates/slicer-core/tests/algo_prepass_slice_tdd.rs`.
- Neighboring tests/fixtures: `disjoint_triangle_mesh`, `identity_transform`, `stacked_cubes_mesh`, `footprints`, `mesh_with_bottom_facets`, `surface_classification_with_quartile_bands`, `make_global_layer`, `batch_slice_objects_by_layer`, `batch_bottom_surface_footprints`, `PrepassSliceCache`; the 6 neighboring integration tests plus `core-retire`'s contrast pair sharing the same target file.
- OrcaSlicer comparison: none; no OrcaSlicer behavior is consulted by this packet.

## Architecture Constraints

- Test-only change: no production signature, threshold, branch, or output changes; all three test names are preserved and no test is added or removed.
- Thresholds are pinned, never loosened: `from_secs(3)`, `from_secs(1)`, and `cached_elapsed * 3 < uncached_elapsed * 2` stay exactly as authored (canonical parity supremacy applied to perf guards: loosening a threshold to make CI green is the timing analogue of loosening a tolerance).
- Feature gating is load-bearing two ways: the lib tests compile to zero under default features via the `host-algos` module gate in `crates/slicer-core/src/algos/mod.rs`, and the integration target is skipped outright without the feature via its `required-features` stanza (no inner cfg header), so every test command uses `--features host-algos`.
- Determinism: no new timing, sleep, ordering, or global-state assertions are added; the review records why each retained wall-clock guard is acceptable (documented regression input, large headroom, self-normalizing ratio).

## Code Change Surface

- Selected approach: KEEP all three guards with hardened rationale comments naming the Benchy regression input each protects (118s incremental-union, O(layers) sweep, 28s per-layer recomputation). Migration to the existing criterion bench (`benches/polygon_ops.rs`) was evaluated and rejected: benches measure without failing, `cargo bench` runs in no gate, and the polygon_ops bench covers a different surface, so migration would delete protection, not move it. Deterministic call-count accounting was evaluated and rejected: it requires production instrumentation (new counters/parameters on `compute_xy_footprint` and the prepass cache path), which is outside this test-only packet and would expand the blast radius into every caller.
- Exact functions, traits, manifests, tests, and fixtures:
  - `mesh_analysis.rs` `tests::compute_xy_footprint_is_fast_for_thousands_of_disjoint_facets` — retain the exact `FACET_COUNT: usize = 1200` const, the line-anchored live `assert_eq!(footprint.len(), FACET_COUNT, ...)` and the `from_secs(3)` guard; add exactly this marker line above the test: `// KEEP-review (core-brittle): 118s Benchy regression guard; batched union measured 61ms vs 20.6s incremental (338x); 3s threshold keeps ~50x headroom.`
  - `overhang_annotation.rs` `tests::annotate_overhangs_is_fast_for_many_stacked_layers` — retain the exact `CUBE_COUNT: usize = 1200` const, the line-anchored live `assert!(result.is_empty(), ...)` and the `from_secs(1)` guard; add exactly this marker line above the test: `// KEEP-review (core-brittle): O(layers) sweep guard; timed region excludes one-time slicing setup; 1s threshold on 1200 pre-sliced layers.`
  - `algo_prepass_slice_tdd.rs` `prepass_slice_caches_bottom_surface_footprint_across_layers` — retain the line-anchored live `assert!(cached_elapsed * 3 < uncached_elapsed * 2, ...)`; add exactly this marker line above the test: `// KEEP-review (core-brittle): per-layer footprint-recomputation guard; cached-vs-uncached ratio self-normalizes across machines; 1.5x minimum (measured ~3.5x debug / ~2.3x release).`
- Rejected alternatives and reasons: deleting any guard as flaky (rejected: each names a measured production regression with a named smell; deletion without replacement violates the disposition); loosening thresholds (rejected: see pinning constraint above); moving guards into the polygon_ops bench (rejected: different surface, non-failing harness); adding production counters (rejected: out of test-only scope).

## Files in Scope (read + edit)

Target at most 3 primary files; justify extras and consider splitting.

- `crates/slicer-core/src/algos/mesh_analysis.rs` - role: footprint timing guard; expected change: rationale-comment hardening only.
- `crates/slicer-core/src/algos/overhang_annotation.rs` - role: overhang timing guard; expected change: rationale-comment hardening only.
- `crates/slicer-core/tests/algo_prepass_slice_tdd.rs` - role: cache ratio guard; expected change: rationale-comment hardening only, roster preserved.
- `docs/specs/test-quality-remediation-plan.md` - role: §7 `core` ledger row only; expected change: append this packet's KEEP-review dispositions and evidence while preserving accumulated content (required wave exit item; fourth file justified because it is a one-row ledger append, not a code surface).

## Read-Only Context

Include ranges for files over 300 lines.

- `crates/slicer-core/src/algos/mod.rs` - lines `1-20` only - purpose: confirm both `host-algos` module gates for AC-N1.
- `crates/slicer-core/src/algos/mesh_analysis.rs` - lines `890-900` and `949-985` only - purpose: tests module header plus guard body pre-edit.
- `crates/slicer-core/src/algos/overhang_annotation.rs` - lines `549-560` and `705-745` only - purpose: tests module header plus guard body pre-edit.
- `crates/slicer-core/tests/algo_prepass_slice_tdd.rs` - lines `1-10` and `395-470` only - purpose: header gating plus ratio guard body pre-edit.
- `crates/slicer-core/Cargo.toml` - lines `26-50` only - purpose: criterion dev-dep, bench stanza, and the integration `required-features` stanza.
- `crates/slicer-core/benches/polygon_ops.rs` - lines `1-30` only - purpose: read-only migration-option evaluation.
- `docs/22_test_quality.md` - delegated SUMMARY only - purpose: earn-their-keep mapping.
- `docs/spec_packets/test-quality-remediation_10_core-retire/packet.spec.md` - read-only predecessor boundary (AC-N1 roster source) - purpose: scope and roster conformance, never edited.

## Out-of-Bounds Files

- `OrcaSlicerDocumented/...` - delegate; never load (no parity question in this packet).
- `target/`, `Cargo.lock`, generated code, vendored dependencies - never load.
- `crates/slicer-core/src/algos/paint_segmentation/*` - owned by `core-paint`.
- `crates/slicer-core/src/polygon_ops.rs`, `crates/slicer-core/tests/triangle_mesh_slicer_tdd.rs` - owned by `core-retire`.
- `crates/slicer-core/tests/*` beyond `algo_prepass_slice_tdd.rs` - different targets, not owned here.
- `crates/slicer-core/benches/*` for editing - migration evaluation is read-only.
- `docs/specs/test-quality-remediation-census.json` - committed census; lib-unit comment edits and a roster-neutral integration edit leave target counts unchanged.
- Every other `docs/spec_packets/*` directory - never modified; predecessor files are read-only context.

## Expected Sub-Agent Dispatches

- Question: run each pipe-suffixed AC and gate command and report pass/fail from the tee'd log; scope: workspace cargo only; return: `FACT`; purpose: implementation verification without loading output.
- Question: confirm `cargo xtask check-test-quality --report` lists zero unwaived findings for the three touched files; scope: the three paths only; return: `FACT`; purpose: gate closure without loading the full report.

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

## Context Cost Estimate

- Aggregate: `S`
- Largest step: `S`
- Highest-risk dispatch and required return format: gate-findings confirmation; `FACT` over the three touched paths.

## Open Questions

None.
