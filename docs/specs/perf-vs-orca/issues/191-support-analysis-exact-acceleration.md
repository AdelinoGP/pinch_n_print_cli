# Exact `PrePass::SupportAnalysis` acceleration: parallel print-plane slicing, exact cantilever distance certificate, parallel layer union, and LPT overlapped scheduling

Type: task / measured candidate (100 % output-preserving host-only acceleration + standing `t137` 4-cell planner gate; zero WIT/guest changes, supports-off structurally inert, zero geometry exceptions, zero gate waivers)
Label: wayfinder:task
Status: open
Assignee: unassigned
Parent: [Slice performance vs OrcaSlicer](../map.md)
Blocked by: none
Graduated from: [`PrePass::SupportAnalysis` attribution and speedup screen](188-support-analysis-attribution-and-speedup-screen.md) ([`evidence/t188-support-analysis-screen/FINDINGS.md`](../evidence/t188-support-analysis-screen/FINDINGS.md)), which attributed the entire post-t185 `PrePass::SupportAnalysis` stage (`16/16` probe runs byte-identical to `t182/reference.json`, classic ≡ arachne bit-exact inside `PrePass::SupportAnalysis`) and screened the host-only exact bundle `SA-1 + SA-2 + SA-3 + SA-4 + SA-5 + SA-6` across `52/52` (`50` measured + `2` warmup) runs (`100 %` byte-identical to `t182/reference.json`, `12/12` A/B wall wins, `+6.72…+11.32 %` wall = `8.39×–14.15×` floor, CPU corroborating `4/4`, `PrePass::SupportAnalysis` `−58.8…−70.4 %` / `2.42×–3.38×`, `host:support_analysis` calls `−23.4 / −32.5 %` and thread-time `−22.8…−43.4 %`, supports-off structurally inert `2/2`, strict monotonic ablation across all steps).

## Question

**Does landing the 100 % output-preserving, host-only bundle `SA-1 + SA-2 + SA-3 + SA-4 + SA-5 + SA-6` inside `crates/slicer-core/src/triangle_mesh_slicer.rs`, `crates/slicer-core/src/algos/overhang_annotation.rs`, and `crates/slicer-runtime/src/builtins/support_analysis_producer.rs` (zero WIT, guest, or Clipper2 changes; `slice_mesh_ex` unchanged so supports-off stays structurally inert) pass the standing `t137` 4-cell planner gate (`>= 0.80 %` paired ordinary + accelerated wall and CPU win on all 4 supports-on cells, structurally inert with non-regression on all 4 supports-off cells, `16/16` byte-identical to `t182/reference.json`)?**

## What is already known (t188)

- **Screen results (host-only edits, untouched `modules/core-modules/`, `RAYON_NUM_THREADS=12`, unsandboxed, `52/52` byte-identical to `t182/reference.json`):**
  - `benchy-classic-on`: **`+6.72 %`** wall (`3.3239 → 3.1007 s`, `8.39×` floor, `3/3`), **`+4.88 %`** CPU, `PrePass::SupportAnalysis` `328 → 97 ms` (`−70.43 %`, `3.38×`), `host:support_analysis` `0.9835 → 0.7593 s` (`−22.79 %`, calls `1,366 → 1,046` = `−23.43 %`).
  - `benchy-arachne-on`: **`+7.94 %`** wall (`3.4424 → 3.1690 s`, `9.93×` floor, `3/3`), **`+3.72 %`** CPU, `PrePass::SupportAnalysis` `293 → 113 ms` (`−61.43 %`, `2.59×`), `host:support_analysis` `1.0232 → 0.7886 s` (`−22.93 %`, calls `1,366 → 1,046` = `−23.43 %`).
  - `base-classic-on`: **`+11.32 %`** wall (`25.9686 → 23.0296 s`, `14.15×` floor, `3/3`), **`+7.07 %`** CPU, `PrePass::SupportAnalysis` `3,069 → 1,266 ms` (`−58.75 %`, `2.42×`), `host:support_analysis` `22.8855 → 13.7998 s` (`−39.70 %`, calls `4,430 → 2,990` = `−32.51 %`).
  - `base-arachne-on`: **`+10.57 %`** wall (`26.5024 → 23.7002 s`, `13.22×` floor, `3/3`), **`+4.75 %`** CPU, `PrePass::SupportAnalysis` `3,382 → 1,149 ms` (`−66.03 %`, `2.94×`), `host:support_analysis` `22.3549 → 12.6515 s` (`−43.41 %`, calls `4,430 → 2,990` = `−32.51 %`).
- **Route projection (`ESTIMATE ONLY`, cross-window composition against `t185/reduced/route.json`):**
  - Standalone (`t185 × t188`): `benchy-arachne-on` `1.0846x → ~0.9985x` ord, `benchy-classic-on` `1.2036x → ~1.1227x` ord, `base-on` `~0.7480x / ~0.7452x` ord.
  - Stacked with [Exact `com.core.tree-support-planner` acceleration](190-tree-support-planner-exact-acceleration.md) (`t185 × t187 × t188`): `benchy-classic-on` **`~1.0258x` ord** (`~1.0172x` stage-delta, only `2.51 %` remaining for `PostPass` at `14.4 %`), `benchy-arachne-on` **`~0.9074x`**, `base-classic-on` **`~0.7183x`**, `base-arachne-on` **`~0.6635x`**.
- **Explicit non-goals (proven in t188):**
  - **Do NOT modify `slicer_core::triangle_mesh_slicer::slice_mesh_ex` directly** — keep `slice_mesh_ex_parallel` as a distinct `#[cfg(feature = "host-algos")]` helper called only from `collect_print_plane_occupancy` so supports-off (`PrePass::Slice`) remains 100 % structurally inert under `t137` (wiring `slice_mesh_ex_parallel` into `PrePass::Slice` is parked for the `<= 0.50x` follow-on).
  - **Do NOT include `G_holes`, `G_win`, `G_island_win`, or `G_cert`** on `grown_offset` — falsified/rejected in `t188` (no wall win and Clipper2 scanbeam/ordering drift on `2` `benchy` / `41–121` `base` layers).

## Scope

Production edits strictly within `crates/slicer-core` (host-gated where using `rayon`) and `crates/slicer-runtime` (zero WIT, schema, guest WASM, or `vendor/clipper2-rust` edits):

1. **`SA-1` — Parallel print-plane slicing (`crates/slicer-core/src/triangle_mesh_slicer.rs::slice_mesh_ex_parallel` + `crates/slicer-runtime/src/builtins/support_analysis_producer.rs::collect_print_plane_occupancy`):**
   - Derive `Copy` on `IntersectionLine` and add `#[cfg(feature = "host-algos")] pub fn slice_mesh_ex_parallel(mesh: &IndexedTriangleSet, zs: &[f32]) -> Vec<Vec<ExPolygon>>`.
   - Partition `mesh.indices` into deterministic triangle chunks (`CHUNK_TRIS = 16_384`, serial fallback below `2 * CHUNK_TRIS`), run `slice_make_lines_for_indices` per chunk in parallel, merge chunk line buffers per layer in exact chunk order `0..P` in parallel, and run `chain_lines_to_expolygons` per layer with `into_par_iter().collect()`.
   - Call `slice_mesh_ex_parallel` from `collect_print_plane_occupancy` (leaving `slice_mesh_ex` unchanged).
2. **`SA-2` — Parallel `object_layer_union` (`crates/slicer-runtime/src/builtins/support_analysis_producer.rs::commit_support_analysis_builtin`):**
   - Store `Vec<&[ExPolygon]>` per `(object_id, layer_index)` (zero-clone on single-region layers) and execute `union_ex` in parallel via `into_par_iter().collect::<BTreeMap<_, _>>()`.
3. **`SA-3` — Empty `enforcers` / `blockers` guard (`crates/slicer-runtime/src/builtins/support_analysis_producer.rs::commit_support_analysis_builtin`):**
   - Guard `enforced` and `blocked` checks with `!enforcers.is_empty()` and `!blockers.is_empty()` before calling `intersection_ex`.
4. **`SA-4` — Exact `cantilever_surfaces` distance certificate (`crates/slicer-core/src/algos/overhang_annotation.rs::detect_support_contacts_with_annotations`):**
   - Add `certify_covered_by_square_offset(contacts, lower_layer_polygons, threshold_mm)` applying the exact `OFFSET_SHORTEST_EDGE_FACTOR` (`0.005 * delta_units`) closed-ring decimation, positive-area contour + kept-hole spatial segment grid (`LowerSegGrid`), conservative `1,000`-unit (`0.10 mm`) safety margin (`r_in = delta_units - 1_000.0`), fast-path single-disk / vertex screen / narrow-strip checks, and a 2-level coarse-to-fine box cover.
   - Return `cantilever_surfaces = Vec::new()` only when mathematically proven covered; fall back to the unchanged `offset(lower, threshold_mm, Square, 0.01)` + `difference_ex` path on any uncertain or near-threshold case.
5. **`SA-5` + `SA-6` — Concurrent `rayon::join` + LPT contact scheduling + zero-clone `ContactWork<'a>` (`crates/slicer-runtime/src/builtins/support_analysis_producer.rs::commit_support_analysis_builtin`):**
   - Borrow `polygons: &'a [ExPolygon]` directly from `slices` in `ContactWork<'a>`.
   - Overlap `collect_print_plane_occupancy` with `(object_layer_union → ModifierGeometry::slice → contacts_par)` via `rayon::join`.
   - Schedule `contact_work` in descending `(lower_verts + region_verts)` weight order (tie-broken by deterministic index) with `.into_par_iter().with_max_len(1)`, then sort surviving `(idx, Contact)` pairs by `idx` ascending so committed candidate ordering is 100 % deterministic.
6. **Required unit & regression tests:**
   - `crates/slicer-core/src/triangle_mesh_slicer.rs` (or `crates/slicer-core/tests/`): `slice_mesh_ex_parallel` equals `slice_mesh_ex` bit-for-bit across multi-chunk meshes (> 32,768 triangles), grazing facets, holes, and empty inputs.
   - `crates/slicer-core/tests/support_overhang_detection_tdd.rs`: `certify_covered_by_square_offset` / `detect_support_contacts_with_annotations` parity against uncertified `offset(lower, 3.0) + difference_ex` across covered rims, near-threshold (`2.85 / 2.95 / 3.05 mm`) contacts, true cantilevers (`> 3.0 mm`), C/L-shaped strips, contacts spanning large holes in `lower`, degenerate/sub-decimation contours, empty `lower`, and CW/reversed rings.
   - `crates/slicer-runtime/src/builtins/support_analysis_producer.rs`: multi-region layer union parity, empty vs non-empty enforcer/blocker flags, deterministic candidate ordering under LPT scheduling, and `enable_support: false` inertness.
7. **Standing `t137` 4-cell planner paired ordinary + accelerated + null-band gate (`RAYON_NUM_THREADS=12`, unsandboxed):**
   - **Guest freshness note:** editing `crates/slicer-core/src/` marks guests that path-depend on `slicer-core` stale under `cargo xtask build-guests --check`, so rebuild both ordinary and accelerated guests (`cargo xtask build-guests` and `cargo xtask build-guests --accelerated`; verify both `--check` commands exit `0`), AND remember to build both ordinary and accelerated host binaries for the paired gate.
   - **Output parity (fail-closed, zero reference change):** baseline AND candidate MUST reproduce `t182`'s active `reference.json` byte-for-byte on **16/16** `(8 cells × {ordinary, accelerated})` runs (`exit 0`, `degraded = false`, `0` fatal/non-fatal errors, `0` code-1200 diagnostics).
   - **Timing (standing `t137` 4-cell gate):** 5 interleaved pairs/cell ordinary + 3-pair null control + 3-pair accelerated corroboration across all 4 supports-on cells (clearing `>= 0.80 %` wall with corroborating lower process CPU and accelerated corroboration), plus non-regression check on all 4 structurally inert supports-off cells.
   - **Pre-commit code gates:** `cargo test -p slicer-core --features host-algos --test support_overhang_detection_tdd`, `cargo test -p slicer-runtime --lib support_analysis_producer`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo xtask check-literals`, `cargo xtask check-test-quality --report`, `cargo xtask build-guests --check`, and `cargo xtask build-guests --accelerated --check`.

## Acceptance

- Clean, tested production implementation within `crates/slicer-core` and `crates/slicer-runtime` (zero WIT/guest source edits; both `cargo xtask build-guests --check` and `--accelerated --check` exit `0`).
- Evidence directory `docs/specs/perf-vs-orca/evidence/t191-support-analysis-exact-accel/` with reduced paired ordinary + null + accelerated + `--profile` JSONs, `FINDINGS.md`, `REPRODUCE.md`, and checkout-only `verify.py` (`verify.py --self-test` exit `0`).
- Updated `## Answer`, `map.md` update, and clean commit after all gates pass (zero reference-set change).
