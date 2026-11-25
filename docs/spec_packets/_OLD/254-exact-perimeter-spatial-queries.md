---
status: implemented
packet: 254-exact-perimeter-spatial-queries
task_ids:
  - TASK-561
---

# 254-exact-perimeter-spatial-queries

## Goal

Add exact, independently-oracled shared per-region spatial query contexts for Classic and Arachne perimeter generation, and measure them only through a reproducible controlled Rust compiler mode.

## Problem Statement

Perimeter generation repeatedly scans region-specific geometry for distance, sign, quartile, and bridge queries. TASK-561 needs an exact spatial acceleration that preserves current floating/integer arithmetic and source-order semantics, is shared only within one region invocation, and is evaluated under a reproducible compiler contract rather than an uncontrolled numerical build. The same coherent slice must also prove real Classic/Arachne pipeline use and provide strict CPU and wall evidence.

### Amendment 1 (toolchain pin, owner-approved 2026-10-04 merge review)

The Out of Scope exclusion of a repo-wide `rust-toolchain.toml` pin is superseded for this packet's own surfaces by the owner-approved amendment recorded during merge review on 2026-10-04. Commit `ee3ea8fe` added the pin deliberately to address CI Format/Clippy toolchain skew (floating stable was 1.98.1 while local development used 1.96.0, whose rustfmt output and Clippy lint set differed) and to keep the compiler aligned with the controlled-perimeter policy's exact identity, since that policy rejects other toolchains and a floating stable would break accelerated invocations after a rustup update. The run-time identity verification described in the original Out of Scope line is retained; the repository pin supplements it rather than replacing it.

## Architecture Constraints

- One immutable context per region is constructed outside all relevant wall passes. It owns four independent indexes and compact source-ordinal records; no cross-region cache or public context field is added.
- The legacy evaluator remains independent and is used for exact re-evaluation and all fallback paths. Preserve the distance domain (integer endpoints pass through `units_to_mm` f32 before `f64::from` promotion; the f32 query is promoted with `f64::from`; the result is `sqrt() as f32`), direct f64 winding conversion (`i64 / 10_000.0`), integer bridge inputs, strict boundaries, source order, signed zero, `None`/`Some`, and final normalization.
- Distance boxes use `a` and `a + (b-a)`, not raw endpoint `b`. Sign/quartile use neutral-X two-dimensional trees; rstar DIM1 is forbidden.
- Exact radius formula (selected, settled): for seed squared distance `D`, `radius = D.next_up().sqrt().next_up().next_up()`; query bounds `(q - radius).next_down()` / `(q + radius).next_up()` per axis; retrieval via `locate_in_envelope_intersecting` (inclusive); exact re-evaluation of every candidate with the unchanged legacy expression; winner initialized `None` and reduced lexicographically by `(distance_sq.total_cmp(&best), source_ordinal)`. The seed only bounds the search; it never preempts an earlier equal-distance edge. Proof boundary: each rounded component square ≤ rounded sum ≤ `D`; `next_up(D)` bounds exact squares of candidates able to beat/tie the seed; `next_up(sqrt(next_up(D)))` covers subtraction rounding; outward box bounds cover addition rounding; monotone finite multiply/add over clamped t bounds every computed closest point by `a`/`a+(b-a)`; ordinal reduction reproduces the strict-minimum first-edge tie rule. Premise: the audited toolchain lowers the single `powi(2)` site in `signed_distance_to_boundary` to ordinary multiply without fast-math flags (verified for release host/wasm and the controlled optimized-core debug configuration; unoptimized debug lowers to C `pow` and is never accelerated).
- Sign/quartile Y intervals use the winding predicate's direct `i64 → f64 / 10_000.0` domain: for each contour edge, include raw endpoint Ys and computed `a_y + (b_y - a_y)`, expand extrema by `f64::from_bits(1).sqrt().next_up()`, and outward-round final bounds; query at `[0.0, query_y]` and evaluate the unchanged `point_in_polygon_winding`. Nonfinite queries and index-creation failures use the complete legacy scan.
- Bridge guard (before any bbox rejection): compute global integer-coordinate spans once; per query compute widened extents; require `span_x <= i64::MAX as u128`, `span_y <= i64::MAX as u128`, `query_x_extent <= i64::MAX as u128`, `query_y_extent <= i64::MAX as u128`, and `span_x * query_y_extent + span_y * query_x_extent <= i64::MAX as u128` via checked u128 arithmetic. A failed guard runs the complete original `point_in_any_polygon` scan in source order, preserving debug-panic/release-wrapping behavior. Small sets (`<= <rstar::DefaultParams as rstar::RTreeParams>::MAX_SIZE` relevant records per index, checked independently; 6 in rstar 0.12.2) skip indexing entirely.
- Native controls are scoped/thread-bound around real dispatch. `slicer-core` is a **regular** dependency of `slicer-runtime` (not a dev-dependency), so the test-support feature reaches the `integration` binary only through a forwarding feature on `slicer-runtime`; the AC commands pass `--features perimeter-spatial-test-support` explicitly. Test-support code is gated with `cfg(feature = "perimeter-spatial-test-support")`, never `cfg(all(test, feature))`. The `perimeter_spatial_capture` integration module is always compiled; without the feature it contains one guard test that panics with `perimeter-spatial-test-support feature required`, so a feature-less run fails loudly instead of reporting zero tests.
- Prepared-region capture crosses no crate feature boundary: `slicer-wasm-host` has no `[features]` table and gains none. Instead `slicer-core::perimeter_spatial::diagnostics::observe_prepared_region(RegionCaptureRecord)` is always compiled with a no-op body unless the feature is on; `push_slice_regions` (`crates/slicer-wasm-host/src/dispatch.rs`) calls it once per prepared region with a small owned projection (layer index, region ordinal, contour/hole/bridge counts), never with host types.
- AC-2 evidence comes from the **native in-process** path (`LayerStageRunner::run_stage` with `__slicer_native_entry()`), where guest-internal counters are directly observable. The WASM path (AC-5) observes only host-side prepared-region capture plus complete output bytes; guest-internal counters are never read across the unchanged WIT boundary.
- No tolerance comparisons, global counters, timing metrics in production, or native-vs-WASM oracle.
<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and inspect its exit code: exit 0 means fresh, non-zero means stale (a distinct exit code signals `wasm-tools` is unavailable). Never use `rg -q 'STALE:'` — a `wasm-tools`-missing infrastructure error prints no `STALE:` and would read as fresh. If stale, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it. Note: `crates/slicer-core/build.rs` (new) is part of every perimeter guest's path-dependency closure, so its creation makes both perimeter guests stale by design.
<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.
- The controlled driver is one owned `RUSTC` shim: clear wrappers, save absolute rustc, validate actual argv including probes, inject `pnp_perimeter_spatial_accelerated` only for canonical core lib and canonical core unit tests after policy validation, and latch any policy rejection even if swallowed.
- The allowlist is exact rustc 1.96.0, commit `ac68faa20c58cbccd01ee7208bf3b6e93a7d7f96`, host `x86_64-pc-windows-msvc`, LLVM 22.1.2, targets `x86_64-pc-windows-msvc` and `wasm32-unknown-unknown`; optimized-core debug is `opt-level=3`, debuginfo 2, assertions on, guests release. The repo has no `rust-toolchain.toml`; the driver compares `rustc -vV` at each invocation and rejects drift, so a toolchain update makes accelerated mode fail closed rather than silently re-accept.
- The reserved cfg is declared by a new `crates/slicer-core/build.rs` emitting `cargo::rustc-check-cfg=cfg(pnp_perimeter_spatial_accelerated)`; `slicer-core` uses `[lints] workspace = true`, so a crate-local `[lints.rust] unexpected_cfgs` table cannot be added. Under `clippy --workspace --all-targets -D warnings` an undeclared cfg would fail the gate.

## Data and Contract Notes

- IR/manifest contracts: unchanged; module manifests and config keys remain unchanged.
- WIT boundary: unchanged; dispatch capture observes the already prepared `SliceRegionData` after filtering, via a small owned projection handed to `slicer-core`.
- Determinism/scheduler constraints: source ordinal controls ties; scoped/thread-bound controls avoid global mutable behavior; no scheduler change.

## Locked Assumptions and Invariants

- Exact equality is to the legacy oracle compiled in the same supported configuration, not a universal cross-compiler bit guarantee.
- Small sets use independent linear evaluation per index; nonfinite/guard-failed inputs use complete legacy scans.
- Acceptance is ABBA/BAAB, warmup 1, four samples, 12 threads, three workloads, both generators (six cells), CPU and wall strict range separation per cell; overlap stops as inconclusive.
- Test-support features are absent from timing/dist production artifacts; normal mode remains linear.

## Risks and Tradeoffs

- Conservative envelopes may reduce pruning but cannot reject a candidate under the audited arithmetic; structural cutoffs avoid index overhead for small sets.
- Driver policy is deliberately strict and requires maintaining exact toolchain identity and mode-aware metadata; with no repo toolchain pin, a rustup update turns accelerated mode into a fail-closed rejection until the allowlist is deliberately revised.
- WASM and native adapter details may differ; each mode compares against its own preserved baseline.
- The acceptance corpus is local and user-supplied; AC-4 is environment-bound by design and cannot run on a machine without it.
