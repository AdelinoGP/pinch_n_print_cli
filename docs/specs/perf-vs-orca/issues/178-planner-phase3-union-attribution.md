# Planner Phase-3 batched union attribution and mechanism screen

Type: task / attribution + mechanism screen (no production commit in this take;
probe reverted if used)
Label: wayfinder:task
Status: open
Assignee: unclaimed
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

_(open)_
