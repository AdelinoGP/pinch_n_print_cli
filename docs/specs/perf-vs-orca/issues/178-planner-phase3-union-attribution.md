# Planner Phase-3 batched union attribution and mechanism screen

Type: task / attribution + mechanism screen (no production commit in this take;
probe reverted if used)
Label: wayfinder:task
Status: **resolved 2026-11-21** — attribution + 29-mechanism screen completed (no production edit; `probe.patch` cleanly reverted, both freshness gates exit 0). Mechanisms A/B/C and `subject=all` falsified as sufficient levers; **host-side fast convex-containment pruning + bbox CC + Morton Z-order spatial hierarchical union (`M9n_host_nested_prune_hier_k8`)** cuts Phase-3 wall **−91.4 %** / thread **−92.4 %** on `base` (`< 1 µm` boundary Hausdorff, `−4.05 s` / `−14.3 %` cell projected, `17.9×` floor) and **−69.3 %** wall on `benchy` (`0.40 µm` Hausdorff, `−107 ms` / `−3.1 %` cell, `3.9×` floor), with zero WIT/guest changes and 100 % supports-off inertness (`clip_polygons_batch` has only one caller in the repo). Graduated [Host-side spatial hierarchical Phase-3 union with convex-containment pruning](182-host-spatial-hierarchical-phase3-union.md).
Assignee: agent (2026-11-21)
Parent: [Slice performance vs OrcaSlicer](../map.md)
Blocked by: none
Graduated from: [Destination ruling: CPU-corroboration semantics and sample count for cell closure](176-destination-cpu-corroboration-ruling.md)
and [Tiled collision operand for the planner's phase-1 carve](175-tiled-collision-carve.md)
([`evidence/t175-tiled-collision-carve/FINDINGS.md`](../evidence/t175-tiled-collision-carve/FINDINGS.md)),
which reduced `host:native` clip thread on `base-classic-on` from **173.90 s →
56.02 s**, leaving Phase-3 batched union (`planner::batched_phase3_union`,
`51.60 s` thread / `5.21 s` wall, 480 calls, `13.5 M` points in
[t173](173-support-geometry-boolean-volume.md)) at **~92.1 %** of remaining
`host:native` clip thread on `base-classic-on` (**~89.6 %** on
`base-arachne-on`).

## Question

**Why do the 480 Phase-3 `ClipOperation::Union` calls inside
`modules/core-modules/tree-support-planner/src/lib.rs::build_roles_batched`
burn ~51.6 s of thread-time (~5.21 s wall) on `base` supports-on, and what
concrete reduction mechanism captures enough of it to push `base-classic-on`
(~1.221x est.) and `base-arachne-on` (~1.201x est.) across `1.0x` (toward 4/8)?**

Concretely:

1. **Profile the Phase-3 operands and call distribution** in
   `build_roles_batched` Phase 3
   (`modules/core-modules/tree-support-planner/src/lib.rs`):
   - Break down the 480 calls (`base-classic-on` and `benchy-classic-on`) by
     role (`ROLE_BODY`, `ROLE_ROOF`, `ROLE_BASE`, `ROLE_FLOOR`), input polygon
     count per call, vertex count per call, output polygon/vertex count, and
     per-call thread-time / wall tail (max vs median request per batch).
   - Measure the **bbox-connected component structure** of `carved[i][role]`
     before Phase 3: how many input polygons are **bbox-disjoint singletons**
     (cannot intersect any sibling in the same `(layer, role)`), how many fall
     into small multi-polygon overlapping components vs one giant component,
     and what fraction of the `13.5 M` input points sits in singletons or small
     components?
   - Check whether `carved[i][role]` inputs are already clean/canonical from
     Phase 1 + `largest_part` (so disjoint singletons need zero union sweep vs
     whether any self-intersecting/non-canonical singletons still require
     normalization).
2. **Screen and price concrete candidate mechanisms** (on real `base-classic-on`
   and `benchy-classic-on` Phase-3 payloads, checking both projected wall/CPU
   saving and output equivalence):
   - **Mechanism A — Bbox-connected component partitioning + singleton bypass:**
     partition `carved[i][role]` into bbox-overlapping connected components
     (union-find / sweep-line); pass through canonical singletons without a
     Clipper2 call, and dispatch each multi-polygon component as an independent
     `ClipOperation::Union` inside `run_clip_batches` (shrinking per-call
     operands and raising parallelism width across all 12 threads).
   - **Mechanism B — Phase-4 `expolygons_simplify` ordering / pre-union
     dedup or tolerance check:** measure whether simplifying or filtering
     degenerate rings before vs after Phase 3 reduces Phase-3 vertices, and
     whether it preserves geometry or requires a parity ruling.
   - **Mechanism C — Parallelism / batch-chunking tail check:** after `t143`'s
     `EMIT_BATCH_LAYERS = 12`, measure whether Phase-3's `9.9x` fan-out still
     leaves a per-batch critical-path tail (longest single `(layer, role)` call)
     that component splitting eliminates.
   - **Any other mechanism surfaced by the operand census** (e.g., identical
     layer-role memoization, redundant multi-body overlap, or Clipper2
     `Union` call structure `subject = all, clip = empty` vs `head / tail`).
3. **Check the Clipper2 distant-scanbeam interaction (`t175` precedent):**
   verify on the real Phase-3 calls whether splitting a whole-layer union into
   bbox-connected components is byte-identical or triggers `vendor/clipper2-rust/src/engine.rs::add_new_intersect_node`
   ~1 µm distant-scanbeam unclamping (which requires an explicit human KEEP
   ruling per `map.md`'s `t137` note).
4. **Price route impact:** state each candidate mechanism's projected wall
   saving (ms and % of cell wall), remaining `base-classic-on` / `base-arachne-on`
   estimated ratio vs `t174` + `t175` (checking whether it crosses `< 1.0x`),
   and whether it clears `t137`'s 4-cell supports-on gate cleanly.

## Scope

- Attribution and offline/probe mechanism screen only; revert any diagnostic
  probe cleanly (preserve as `probe.patch` in the evidence dir) and leave
  production code untouched.
- If a clear winning mechanism emerges (clearing `t137` by wide margin and
  closing most or all of `base` supports-on's remaining ~16–18 % cut), graduate
  a dedicated implementation + standing-gate ticket.

## Pre-declared gate

**Attribution + screen only (no landing gate).** Correctness bar:

- `cargo xtask build-guests --check` exits 0 before and after probe revert.
- Unmodified baseline probe runs reproduce `t175`'s active `reference.json`
  byte-for-byte (`exit 0`, `degraded = false`, `0` code-1200).
- Every candidate's ceiling and operand reduction is re-derived from real
  captured `base-classic-on` / `benchy-classic-on` Phase-3 inputs (never
  extrapolated from synthetic shapes alone).

## Acceptance

- Evidence directory `docs/specs/perf-vs-orca/evidence/t178-planner-phase3-union/`
  with `FINDINGS.md`, `REPRODUCE.md`, reduced JSONs, `probe.patch` (if used),
  and checkout-only `verify.py` (`verify.py` + `verify.py --self-test` exit 0).
- Updated `## Answer` in this ticket, `map.md` update, and (if a mechanism
  wins) a newly graduated implementation ticket.

## Answer

**Resolved 2026-11-21 — Phase-3 union is 99.99 % `ROLE_BODY` monolithic Clipper2 sweeps over deeply overlapping 100-gon branch ellipses (`7.64×` area overlap, `28.1 %` strictly contained inside a single larger convex sibling, `88.5 %` in giant connected components up to `1,023` polygons); Mechanisms A, B, C, and `subject=all` are falsified as sufficient levers, while host-side fast convex-containment pruning + bbox CC + Morton Z-order spatial hierarchical union (`M9n_host_nested_prune_hier_k8`) cuts Phase-3 wall −91.4 % on `base` and −69.3 % on `benchy` at `< 1 µm` worst-case boundary Hausdorff and graduates as [Host-side spatial hierarchical Phase-3 union with convex-containment pruning](182-host-spatial-hierarchical-phase3-union.md).** Evidence: [`evidence/t178-planner-phase3-union/FINDINGS.md`](../evidence/t178-planner-phase3-union/FINDINGS.md) (`verify.py --self-test` PASS, 6/6 checks, 14/14 negative controls rejected).

1. **Qualification & clean production Phase-3 baseline (zero production edits landed; `probe.patch` cleanly reverted):**
   - **12/12** capture runs (`4` `--instrument-stderr` qualification passes + `8` clean uninstrumented timing passes across all 4 supports-on cells, `RAYON_NUM_THREADS=12`) exit `0`, `degraded = false`, `fatal = 0`, `non_fatal = 0`, `code-1200 = 0`, and match `t175`'s active `reference.json` byte-for-byte. Classic and Arachne emit **100 % identical** Phase-3 operands (`classic_arachne_operand_identity = {benchy: true, base: true}`).
   - Clean uninstrumented Phase-3 batch wall / clip thread-time (median of 3 passes on classic, 1 on arachne): **`base-classic-on`** **`4,434.3 ms`** (`15.68 %` of `28.28 s` cell wall) / **`45,519.2 ms`** (`10.27×` par, `sum(batch_max) = 4,395.3 ms = 99.1 %` wall, WIT/IR conversion `0.23 %`); **`base-arachne-on`** **`4,404.6 ms`** (`15.39 %`) / **`45,261.3 ms`** (`10.28×`); **`benchy-classic-on`** **`154.5 ms`** (`4.48 %` of `3.45 s`) / **`1,548.6 ms`** (`10.03×`, `sum(batch_max) = 98.6 %`, conv `0.65 %`); **`benchy-arachne-on`** **`153.8 ms`** (`4.32 %`) / **`1,547.4 ms`** (`10.06×`).
   - **Role split:** `ROLE_BODY` owns **99.99 %** (`45,536 ms`, `431/480` calls, `136,256` polys / `13.48 M` verts → `2,662` polys / `1.21 M` verts, `p50/p95/p99/max = 85.2 / 291.2 / 400.0 / 414.2 ms`, max `1,024` polys / `102,400` verts on lower trunk layers) on `base` and **99.95 %** (`197/202` calls, max `179` polys / `19.6 ms`) on `benchy`; `ROLE_ROOF` is `5.0 ms` (`43` calls `base`), `ROLE_FLOOR` is `0.46 ms` (`6` calls), `ROLE_BASE` is `0`.
2. **Structural census (100 % of real captured payloads; exact vertex-for-vertex baseline replay 480/480 `base` and 202/202 `benchy`, 0 mismatches):**
   - **Connected components:** singletons are only **`0.39 %`** (`530/136,440` polys `base`) and **`0.29 %`** (`63/22,058` `benchy`) — **100 % clean/canonical** (`530/530` and `63/63` pass winding + self-union area/ring exactness with `0` failures). Meanwhile **88.5 %** of `base` vertices sit in giant components (`65..256`: `59.06 %`, `257+`: `29.48 %`, max **`1,023` polys** in one component; `benchy` `65..256`: `58.74 %`, max `129`).
   - **Overlap & exact convex containment:** `base` has a **7.64× area overlap factor** (`4.08 M → 534 k mm²`, `11.0×` vertex collapse) and **`38,351 / 136,440` (`28.11 %`, `3.74 M` verts)** polygons strictly contained inside a single larger convex CCW sibling (`benchy`: `3.50×` area overlap, **`6,335 / 22,058` = `28.72 %`** strictly contained).
   - **Simplification & duplicates:** `avg_nodes = 8` (`base`) / `5` (`benchy`) (`<= 200`), so Phase-4 `expolygons_simplify` executes on **0/480** and **0/202** calls; exact within-call duplicate polygons = `0`; identical cross-layer `(layer, role)` input repeats = `0`.
3. **29-mechanism offline screen (`RAYON_NUM_THREADS=12`, 3 interleaved rounds, exact 12-layer batch boundaries):**
   - **Falsified / rejected mechanisms:**
     - **Mechanism A (`M2` CC + singleton bypass alone):** fails as a standalone lever (`−8.7 %` to `+12.4 %` wall noise on `base` across takes, `+0.4…+18.5 %` thread) because `88.5 %` of vertices remain inside giant components.
     - **Mechanism B (`M6` pre-union RDP simplification):** `50u` / `125u` give only `+7.3 %` / `+47.4 %` wall while inflicting catastrophic `870 mm²` / `3,902 mm²` XOR area and `583 µm` / `3,420 µm` Hausdorff errors. Rejected.
     - **Mechanism C (`M5a` LPT within batch12 / `M5b` single global batch):** `M5a` is noise (`−1.5 %`); `M5b` hits the `45.5 s / 12 = 3.79 s` floor (`+12.9 %` wall, `0 %` CPU cut).
     - **`M1` (`subject = all, clip = []`):** byte-identical (`0/480` diffs) and speed-neutral.
   - **Winning mechanism — `M9n_host_nested_prune_hier_k8` (fast exact convex-containment pruning + bbox CC + Morton Z-order spatial hierarchical union inside host workers):**
     - `M3` (fast inscribed-disk + $O(\log V)$ convex-containment pruning alone) cuts thread-time **−43.3 %** (`base`) / **−53.3 %** (`benchy`) at `0.224 µm` Hausdorff.
     - Combining pruning + CC + Morton `k = 8` spatial tree-reduction inside host workers (`M9n_host_nested_prune_hier_k8`) cuts **`base` Phase-3 wall −91.38 % (`5,080.5 → 437.7 ms`, 11.6× faster) / thread −92.43 % (`50.36 → 3.81 s`)** with worst-case symmetric boundary Hausdorff **`0.949 µm`** (total XOR `8.76 × 10⁻³ mm² = 1.64 × 10⁻⁸` relative), and **`benchy` Phase-3 wall −69.26 % (`182.2 → 56.0 ms`) / thread −74.14 %** with Hausdorff **`0.400 µm`** (XOR `4.93 × 10⁻⁹` relative). (`M9_k4` hits `−92.12 %` wall / `−93.48 %` thread at `2.309 µm` Hausdorff; guest-side 2-level `M4c_2lvl_k16` reaches `−83.62 %` wall at `0.316 µm` Hausdorff before WASM prep / WIT marshal penalties.)
   - **Architectural placement discovery:** `git grep clip_polygons_batch` proves `modules/core-modules/tree-support-planner/src/lib.rs::run_clip_batches` is the **sole caller** of `clip_polygons_batch` in the repository. Wiring `slicer_core::polygon_ops::union_spatial_tree` into `crates/slicer-wasm-host/src/host.rs::clip_polygons_batch` (plus `slicer-sdk` native fallback) strictly for `ClipOperation::Union` requires **zero WIT/schema/guest edits**, keeps `37` batches / `480` requests, parallelizes prep across 12 native threads, and is **100 % structurally inert on all 4 supports-off cells**.
4. **Clipper2 distant-scanbeam proof (Question 3):**
   - Exhaustive dummy-far-Y auditing (`audit_cc_scanbeam`) proved **`147/147` (100.0 %) `base`** and **`27/27` (100.0 %) `benchy`** of `M2`'s differing calls are caused solely by `vendor/clipper2-rust/src/engine.rs::add_new_intersect_node` clamping shallow intersections to unrelated sibling components' global `scanline_list` Y coordinates (**0 unexplained**, true boundary Hausdorff `0.162 µm` `base` / `0.130 µm` `benchy`; raw vertex-to-vertex shift `68–113 µm` reflects collinear chord vertex placement along tangent 100-gon intersections). Per `t175` / `t137`, `t182` will require an explicit human KEEP ruling for `< 1 µm` `Support` / `Support interface` shifts with zero part-geometry movement.
5. **Route pricing (`ESTIMATE ONLY` vs `t174 × t175`) & graduation:**
   - `M9n_host_nested_prune_hier_k8` clears `t137`'s 4-cell supports-on floor by **3.7×–17.9×**: `base-classic-on` **`−4,052 ms` (`−14.33 %`, `17.9×`) → `1.046x` ord (`1.002x` acc)**, `base-arachne-on` **`−4,025 ms` (`−14.06 %`, `17.6×`) → `1.032x` ord (`0.991x` acc — crosses `< 1.0x` on accelerated)**, `benchy-classic-on` **`−107 ms` (`−3.10 %`, `3.9×`) → `1.521x`**, `benchy-arachne-on` **`−107 ms` (`−2.99 %`, `3.7×`) → `1.347x`**.
   - Because clean Phase 3 is `15.68 %` / `15.39 %` of `base` supports-on against `18.07 %` / `16.75 %` required cuts (`1.029x` / `1.016x` ord even at zero Phase 3), `M9n_k8` closes **79.3 % / 84.0 %** of the remaining ordinary gap; stacking with half to full [`SupportGeometry` post-planner host tail attribution and reduction screen](181-support-geometry-post-planner-host-tail.md) (`7.8 % / 7.0 %` post-planner host tail) crosses `< 1.0x` cleanly on both ordinary `base` cells (**`~0.998x → 0.951x`** and **`~0.990x → 0.948x`**, toward **4/8**).
   - **Graduated:** [Host-side spatial hierarchical Phase-3 union with convex-containment pruning](182-host-spatial-hierarchical-phase3-union.md) (AFK candidate + `t137` standing gate + human KEEP ruling).

