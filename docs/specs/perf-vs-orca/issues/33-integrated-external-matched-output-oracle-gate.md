# Integrated/external matched-output oracle gate

Type: task
Status: resolved (2026-10-02; gate FAILED — native postprocess-view projection root cause; see ## Answer)
Assignee: current OpenCode session (wayfinder, ses_f00fabe75ffeQ2rDpY4safgAqj)
Parent: [Slice performance vs OrcaSlicer](../map.md)
Blocked by: 23

## Question

After [Integrated-parity oracle experiment](23-integrated-parity-oracle.md)'s
native view repairs, is the integrated path output-equivalent to external/WASM
for the map's matched job, such that integrated timings can inform module
performance work? Do not assume so from narrow projection or single-module
tests: the earlier all-integrated run emitted different G-code TYPE counts
(`docs/specs/perf-vs-orca/evidence/perf-split/FINDINGS.md` §Finding 3).

Run matched integrated and external slices from complete, version-pinned
snapshots on the same benchy/base configuration, beginning with the formerly
divergent infill/bridge/surface classes. Record actual module provenance,
effective config, diagnostics/degraded status, TYPE counts, output bytes, and
geometry-relevant output differences per mode. Verify guest freshness before
attributing any mismatch. If the outputs differ, localize the first differing
stage or view, repair its native/WASM parity, and repeat the output gate;
otherwise explicitly bound the equivalence claim to the cells examined. Only
then may a separate ordinary/accelerated timing comparison use integrated
runs as a module-work oracle. Integrated execution must never be substituted
for the external matched-pair verdict itself.

AFK and parallel-takeable; this is a correctness/measurement qualification,
not authorization to auto-commit a performance candidate.

## Answer

**Gate FAILED on the first examined cell — the integrated leg is
output-divergent, and the divergence is root-located in code.** Integrated
timings remain disqualified as a module-work oracle.

Measured on provenance-proved fresh snapshots (24 external vs 24 integrated,
guests fresh, matched-pair configs, benchy classic supports-off): the
integrated leg's G-code reproduces perf-split Finding 3's exact shape on a
2026-10-02 tree — Sparse infill 204→143, Bridge 22→7, Bottom surface 24→7,
Top surface 70→31 (`;TYPE:` counts, external vs integrated), all other
sections equal, completion clean on both legs, bytes 4,330,081 vs 4,550,163.
Corroborated on the frozen t44 job (external leg byte-identical to the
ticket-44 accepted fresh baseline `1b71f83d…`, so the arms differ only by
dispatch provenance): Sparse 139→118, Bridge 18→3, Bottom 23→4, Top 32→18.

Root cause (static, code-verified): the native `Layer::InfillPostProcess`
request (`build_native_layer_request_impl`,
`crates/slicer-wasm-host/src/marshal/native.rs`) builds its
`perimeter_regions` from `PerimeterIR` via `PerimeterRegionView::from_ir`
(`crates/slicer-sdk/src/views.rs`), which cannot carry the four partitioned
role polygons — they live only on the arena's `SliceIR` after
`sync_perimeter_infill_areas_into_slice`
(`crates/slicer-runtime/src/region_partition.rs`). The WASM leg's dispatch
arm enriches its views from that partitioned `SliceIR`
(`push_infill_postprocess_regions`,
`crates/slicer-wasm-host/src/dispatch.rs`, including `tool_index` and
`wall_source_region_id`); the native leg omits the enrichment entirely. The
infill linker then sees all-empty partitions,
`RoleBoundaries::is_partitioned`
(`modules/core-modules/infill-linker/src/orchestrate.rs`) is false, and
`for_role` silently hands every role the *union* boundary instead of its own
partition — so the native leg prints paths the external leg clips away
(+572.5 mm Sparse, +698.7 mm Top surface, +760.6 mm Internal solid infill,
+470.1 mm Bottom surface, +441.7 mm Internal Bridge, +95.2 mm Bridge over the
t44 job) and merges sections into fewer `;TYPE:` transitions. On
same-geometry layers the move multisets are identical and only
ordering/`M73`/`M106` differ (the emitter and remaining-time estimator are
order-sensitive); the section collapse follows the same re-grouping. Ticket
23's view-seam repairs gate/configure the `SliceRegionView` list and do not
cover this projection. Ticket 37's empty-commit `Some(empty)` verdict is also
unreachable through the real native request builder (the all-empty
`RoleBoundaries` never counts as partitioned), though the direct contract
tests on both legs still pass.

Also repaired to run the gate at all: `cargo xtask dist --edition integrated`
could not build — packet 240b added `raft-default` after ADR-0056/0057 and it
was never given its 24th integration row. Added the dependency+feature rows
(`crates/slicer-integrated-modules/Cargo.toml`), the registry entry with the
standard labels and the `Layer` family (`crates/slicer-integrated-modules/src/lib.rs`),
and the `integrated-raft-default` passthrough feature
(`crates/pnp-cli/Cargo.toml`). `cargo check --workspace --all-targets`,
`cargo clippy --workspace --all-targets -- -D warnings`,
`cargo xtask check-literals`, `check-test-quality --report`, and
`cargo xtask build-guests --check` (exit 0) all pass in-session; the staged
integrated snapshot diagnoses 24/24 `integrated`, 0 external.

No fix of the divergence, no re-timing, no commit of a perf change, and no
adoption/production change occurred. Evidence:
[t33 FINDINGS](../evidence/t33-integrated-oracle/FINDINGS.md) with the run
scripts, per-arm CSVs and raw proof outputs. Named fix for the next take:
enrich native `Layer::InfillPostProcess` perimeter views from the partitioned
`SliceIR` the same way `push_infill_postprocess_regions` does, then re-run
this gate from proof.
