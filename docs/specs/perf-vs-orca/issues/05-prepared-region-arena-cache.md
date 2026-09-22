# Prepared-region arena cache

Type: task
Status: resolved

## Question

Should host-side slice-region preparation be memoized per region in
`LayerArena` (`crates/slicer-runtime/src/blackboard.rs`) instead of re-derived
on every module dispatch?

## Answer

**KEEP** — landed `bf11d055` ("perf: memoize host region preparation in
LayerArena"), 2026-09-06 (`docs/specs/perf-vs-orca/evidence/PERF-HANDOFF.md` §11).

Built: `PreparedRegionData` (`crates/slicer-ir/src/prepared_regions.rs`) with
pure derivation in `prepare_regions` / `prepare_slice_regions` /
`prepare_perimeter_source_regions`
(`crates/slicer-wasm-host/src/marshal/prepared.rs`, replacing duplicated logic
in the in/native marshal modules); two arena slots with
`ensure_prepared_regions` / `ensure_prepared_perimeter_source_regions`;
invalidation riding the existing take/set/reset lifecycle (arena mutation audit
found every slice mutation passes through it — no generation counter needed);
per-stage gating in `prepare_dispatch_region_views`
(`crates/slicer-runtime/src/layer_executor.rs`); `LayerStageInput`
(`crates/slicer-wasm-host/src/binding.rs`) carrying IR-typed borrows per
ADR-0005. Dispatch semantics unchanged; both adapters fall back to per-dispatch
derivation when no projection is supplied.

Measured (5-run medians benchy, 2+2 interleaved base, 12 threads, external
wall/CPU/peak-WS): process CPU **−3.8%** benchy (non-overlapping ranges) and
**~−5.8%** base (worst candidate beats best baseline by 135 s); wall improved
but noisy on benchy and inconclusive on base (descheduling); peak WS unchanged;
guest fuel untouched (0.00014% delta); ~66% of preparation calls served from
cache (benchy derive 642 / reuse 1,278). Base's DEV-174 non-fatal error count
identical before/after.

A "too small to keep?" challenge was settled by the base measurement and the
eliminated derivation volume. Verification: full gate set, five arena lifecycle
tests, and `prepared_region_projection_matches_fallback_projection`
(`crates/slicer-wasm-host/tests/contract/view_seam_identity_tdd.rs`).
