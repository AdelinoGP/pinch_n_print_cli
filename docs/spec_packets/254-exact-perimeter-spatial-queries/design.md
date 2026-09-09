# Design: exact-perimeter-spatial-queries

## Controlling Code Paths

- Primary code path: `crates/slicer-core/src/perimeter_utils.rs` legacy evaluators; new `crates/slicer-core/src/perimeter_spatial.rs`; Classic `run_perimeters`, `emit_walls`, `emit_nonplanar_shells`; Arachne `run_perimeters`, `build_walls`, `emit_only_one_wall_top_second_pass`.
- Neighboring tests/fixtures: `crates/slicer-runtime/tests/common/perimeter_harness.rs` (`PerimeterCapturingLayerStageRunner`), runtime integration registration in `crates/slicer-runtime/tests/integration/main.rs`, core polygon predicate tests, and committed perimeter fixtures.
- OrcaSlicer comparison: see `requirements.md` §OrcaSlicer Reference Obligations; do not repeat delegation rules.

## Architecture Constraints

- One immutable context per region is constructed outside all relevant wall passes. It owns four independent indexes and compact source-ordinal records; no cross-region cache or public context field is added.
- The legacy evaluator remains independent and is used for exact re-evaluation and all fallback paths. Preserve f32 endpoint conversion for distance, direct f64 winding conversion, integer bridge inputs, strict boundaries, source order, signed zero, `None`/`Some`, and final normalization.
- Distance boxes use `a` and `a + (b-a)`, not raw endpoint `b`. Sign/quartile use neutral-X two-dimensional trees; rstar DIM1 is forbidden.
- Exact radius formula (selected, settled): for seed squared distance `D`, `radius = D.next_up().sqrt().next_up().next_up()`; query bounds `(q - radius).next_down()` / `(q + radius).next_up()` per axis; retrieval via `locate_in_envelope_intersecting` (inclusive); exact re-evaluation of every candidate with the unchanged legacy expression; winner initialized `None` and reduced lexicographically by `(distance_sq.total_cmp(&best), source_ordinal)`. The seed only bounds the search; it never preempts an earlier equal-distance edge. Proof boundary: each rounded component square ≤ rounded sum ≤ `D`; `next_up(D)` bounds exact squares of candidates able to beat/tie the seed; `next_up(sqrt(next_up(D)))` covers subtraction rounding; outward box bounds cover addition rounding; monotone finite multiply/add over clamped t bounds every computed closest point by `a`/`a+(b-a)`; ordinal reduction reproduces the strict-minimum first-edge tie rule. Premise: the audited toolchain lowers `powi(2)` to ordinary multiply without fast-math flags (verified for release host/wasm and the controlled optimized-core debug configuration; unoptimized debug lowers to C `pow` and is never accelerated).
- Sign/quartile Y intervals use the winding predicate's direct `i64 → f64 / 10_000.0` domain: for each contour edge, include raw endpoint Ys and computed `a_y + (b_y - a_y)`, expand extrema by `f64::from_bits(1).sqrt().next_up()`, and outward-round final bounds; query at `[0.0, query_y]` and evaluate the unchanged `point_in_polygon_winding`. Nonfinite queries and index-creation failures use the complete legacy scan.
- Bridge guard (before any bbox rejection): compute global integer-coordinate spans once; per query compute widened extents; require `span_x <= i64::MAX as u128`, `span_y <= i64::MAX as u128`, `query_x_extent <= i64::MAX as u128`, `query_y_extent <= i64::MAX as u128`, and `span_x * query_y_extent + span_y * query_x_extent <= i64::MAX as u128` via checked u128 arithmetic. A failed guard runs the complete original `point_in_any_polygon` scan in source order, preserving debug-panic/release-wrapping behavior. Small sets (`<= rstar::DefaultParams::MAX_SIZE` relevant records per index, checked independently) skip indexing entirely.
- Native controls are scoped/thread-bound around real dispatch. Feature-gated dependency support is visible to integration dependencies and is not gated by `cfg(all(test,feature))`.
- No tolerance comparisons, global counters, timing metrics in production, or native-vs-WASM oracle.
<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and inspect its exit code: exit 0 means fresh, non-zero means stale (a distinct exit code signals `wasm-tools` is unavailable). Never use `rg -q 'STALE:'` — a `wasm-tools`-missing infrastructure error prints no `STALE:` and would read as fresh. If stale, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.
<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.
- The controlled driver is one owned `RUSTC` shim: clear wrappers, save absolute rustc, validate actual argv including probes, inject `pnp_perimeter_spatial_accelerated` only for canonical core lib and canonical core unit tests after policy validation, and latch any policy rejection even if swallowed.
- The allowlist is exact rustc 1.96.0, commit `ac68faa20c58cbccd01ee7208bf3b6e93a7d7f96`, host `x86_64-pc-windows-msvc`, LLVM 22.1.2, targets `x86_64-pc-windows-msvc` and `wasm32-unknown-unknown`; optimized-core debug is `opt-level=3`, debuginfo 2, assertions on, guests release.

## Code Change Surface

- Selected approach: implement exact query indexes and scoped capture first, then add the driver/mode plumbing, then run focused parity and acceptance validation in a single serialized lane.
- Exact functions, traits, manifests, tests, and fixtures: `perimeter_utils.rs` consumers and independent oracle; new `perimeter_spatial.rs` with public API `PerimeterSpatialContext::new(boundary, overhang_bands, bridge_areas)`, methods `signed_distance_to_boundary(x: f32, y: f32) -> f32`, `overhang_quartile(x: f32, y: f32) -> Option<u8>`, `is_bridge(point: &Point2) -> bool`, plus `expolygon_to_path3d_indexed(contour, z, width, &context, mode)` where `mode` is `PathAnnotationMode::{Planar, NonPlanarNoQuartile}`; reserved cfg name `pnp_perimeter_spatial_accelerated` (declared via `cargo::rustc-check-cfg` and injected only by the driver); test-support feature name `perimeter-spatial-test-support` on `slicer-core` (non-default, visible to integration dev-dependencies via `cfg(feature = "perimeter-spatial-test-support")`, never `cfg(all(test, feature))`); Classic/Arachne region construction and pass calls; `crates/slicer-wasm-host/src/dispatch.rs` prepared region projection; runtime harness/integration registration; `xtask/src/main.rs`, `test.rs`, `dist.rs`, `build_guests.rs`, new `rustc_driver.rs`; mode metadata/freshness and committed synthetic corpus validator.
- Rejected alternatives and reasons: one mixed tagged tree (coordinate domains differ); one-dimensional rstar tree (unsupported); cross-region cache (identity/host plumbing and region-specific geometry); shared oracle/pruning implementation (can hide exactness errors); build.rs-only cfg gate (does not control compiler arithmetic); Arc/public context fields (contract churn — consistent with ADR-0012 `docs/adr/0012-spatial-indexing-as-reconstruction-only-companions.md`, which is **Superseded (Packet 95)** and is cited here as historical precedent only; its Arc-companion mandate is not treated as active, and this packet introduces no spatial index on IR at all); timing before exactness or extra runs after overlap (invalid evidence).
- Driver policy grammar (authoritative, from the confirmed design record): exact rustc identity `1.96.0` / commit `ac68faa20c58cbccd01ee7208bf3b6e93a7d7f96` / host `x86_64-pc-windows-msvc` / LLVM `22.1.2`; targets implicit-or-explicit `x86_64-pc-windows-msvc` and `wasm32-unknown-unknown`; reject duplicate keyed `-C`/`--emit`/`--target`/`--sysroot` controls (repeated `--print`/`--cfg`/`--check-cfg`/`-L`/`--extern`/lint flags are legitimate); reject `@response` files, `-C llvm-args`, `-C target-cpu`, `-C target-feature`, codegen-backend/sysroot overrides, `-Z` unstable options, and unaudited link/LLVM options; accept structural `-l` links and profile codegen families per mode; version/print probes classify separately from compilation; a denied probe latches session failure even when a build script swallows the child's exit; accelerated host debug applies `profile.dev.package.slicer-core.opt-level=3` (debuginfo 2, assertions on), guests keep release; accelerated doctests unsupported; cache namespaces and published artifact metadata record mode/policy/toolchain identity, with mode-aware guest freshness (exit 0/1/3 semantics preserved; `--check` has an explicit accelerated mode and never validates an opposite-mode artifact).

## Files in Scope (read + edit)

- `crates/slicer-core/src/perimeter_spatial.rs` - role: spatial context and exact candidate reduction; expected change: new module and local diagnostics.
- `modules/core-modules/classic-perimeters/src/lib.rs` - role: Classic region/pass integration; expected change: context construction and reuse.
- `modules/core-modules/arachne-perimeters/src/lib.rs` - role: Arachne region/pass integration; expected change: context construction and reuse.
- `crates/slicer-runtime/tests/common/perimeter_harness.rs` - role: real output capture; expected change: scoped mode/capture support.
- `crates/slicer-wasm-host/src/dispatch.rs` - role: actual prepared region projection; expected change: feature-gated capture hook.
- `xtask/src/rustc_driver.rs` - role: controlled compiler policy; expected change: owned shim and session latch.
- `xtask/src/main.rs`, `xtask/src/test.rs`, `xtask/src/dist.rs`, `xtask/src/build_guests.rs` - role: explicit mode entry points, profiles, cache/freshness/publication; expected change: private invocation context and commands.
- Focused test/fixture files and same-packet documentation targets listed in the plan.

## Read-Only Context

- `crates/slicer-core/src/perimeter_utils.rs` - named symbols `signed_distance_to_boundary`, `expolygon_to_path3d`, `point_in_any_polygon` only.
- `crates/slicer-ir/src/polygon_predicate.rs` - `point_in_polygon_winding` only.
- `crates/slicer-wasm-host/src/dispatch.rs` - `push_slice_regions` and `sliced_region_to_data_with_prepared` only.
- `crates/slicer-runtime/tests/common/perimeter_harness.rs` - `PerimeterCapturingLayerStageRunner` and `run_pipeline_capturing_perimeters` only.
- `tmp/perf-next/DESIGN-DECISIONS.md` - bounded slices 1-371, historical provenance only; the normative contracts live in this packet's own files (see Code Change Surface / Architecture Constraints).
- `OrcaSlicerDocumented/**` - delegate only; never load directly.

## Out-of-Bounds Files

- `target/`, Cargo lockfiles, generated code, vendored dependencies, and local model/corpus payloads.
- WIT/IR/scheduler/public host contracts and unrelated packets.
- `docs/07_implementation_status.md` during authoring; completion status is a later worker action.

## Expected Sub-Agent Dispatches

- Question: locate exact generator call sites and pass boundaries; scope: `modules/core-modules/*-perimeters/src/lib.rs`; return: `LOCATIONS`; purpose: bind construction outside every relevant pass.
- Question: verify integration test binary setup and capture seams; scope: `crates/slicer-runtime/tests/**`, `crates/slicer-wasm-host/src/dispatch.rs`; return: `LOCATIONS`; purpose: ensure AC commands drive real dispatch.
- Question: audit driver argv/probe surfaces and private literal blast radius; scope: `xtask/src/{main,test,dist,build_guests}.rs`; return: `LOCATIONS`; purpose: policy and mode plumbing.
- Question: compare canonical behavior; scope: named OrcaSlicer files in `requirements.md`; return: `SUMMARY`; purpose: parity evidence.

## Data and Contract Notes

- IR/manifest contracts: unchanged; module manifests and config keys remain unchanged.
- WIT boundary: unchanged; dispatch capture observes the already prepared `SliceRegionData` after filtering.
- Determinism/scheduler constraints: source ordinal controls ties; scoped/thread-bound controls avoid global mutable behavior; no scheduler change.

## Locked Assumptions and Invariants

- Exact equality is to the legacy oracle compiled in the same supported configuration, not a universal cross-compiler bit guarantee.
- Small sets use independent linear evaluation per index; nonfinite/guard-failed inputs use complete legacy scans.
- Acceptance is ABBA/BAAB, warmup 1, four samples, 12 threads, three workloads, both generators, CPU and wall strict range separation; overlap stops as inconclusive.
- Test-support features are absent from timing/dist production artifacts; normal mode remains linear.

## Risks and Tradeoffs

- Conservative envelopes may reduce pruning but cannot reject a candidate under the audited arithmetic; structural cutoffs avoid index overhead for small sets.
- Driver policy is deliberately strict and requires maintaining exact toolchain identity and mode-aware metadata.
- WASM and native adapter details may differ; each mode compares against its own preserved baseline.

## Context Cost Estimate

- Aggregate: `M`
- Largest step: `M`
- Highest-risk dispatch and required return format: driver argv/policy audit; `LOCATIONS` or bounded `FACT`, never full compiler output.

## Open Questions

- `[FWD]` Step 5 driver: confirm the exact `Invocation`/`VersionProbes` extension surface (private fields, no public `GuestSpec` change) against the live file before editing; the packet fixes behavior, not private struct layout.
- `[FWD]` Step 4 capture: the scoped capture observer's exact wiring inside `push_slice_regions` (test-support-gated callback around the prepared-region push) is placed at implementation time from the bounded LOCATIONS dispatch; scope and semantics are fixed here.

No activation blockers. The design, policy, acceptance protocol, and scope are settled; only implementation-resolvable placement details remain.
