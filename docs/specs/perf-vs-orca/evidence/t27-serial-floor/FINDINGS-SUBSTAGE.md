# T27 work items 2–5 — serial prepass built-in substage attribution (PERF-T27-PROBE)

**Correction (2026-09-28):** the overhang footprint has an exact-intersection
production consumer, and `assemble_bridge_areas` consumes the distinct *bridge*
footprint. The bbox/coverage replacement proposed below is not safe as stated;
see [PREMISE-AUDIT.md](PREMISE-AUDIT.md). Measured brackets below are unchanged.

Wayfinder ticket: [Serial host floor: prepass built-ins and slice-outside wall](../../issues/27-serial-host-prepass-floor.md).
Probe vintage: 2026-09-27, HEAD `bf77f4f7` + the `PERF-T27-PROBE` temporary
brackets (removed before any commit). Guests fresh (`cargo xtask build-guests
--check` exit 0) before the batch. Four uninstrumented slices
(benchy/base × classic supports-off/on), matched-job configs, `RAYON_NUM_THREADS=12`,
process wall/CPU via `GetProcessTimes`; probe lines are the only extra output
(4–5 stderr lines, outside the JSONL stream's contract). Raw captures in
gitignored `target/matched-pair/t27-probe/` (heavy per the evidence policy);
the extracted `.probe.txt` files are committed.

**Bracket semantics (read before quoting any number).** Two classes:

- `*_wall` — calling-thread wall around a whole parallel region (honest wall).
- everything else — **accumulated across rayon workers** (the §3.3 class:
  sums of concurrent spans). They rank work, never wall. Core-side names are
  prefixed `core:`; their sums are likewise accumulated, and on heavily
  parallel stages (OverhangAnnotation) they exceed stage wall.

## Stage walls at the matched job (uninstrumented, this batch)

| Stage | benchy off | benchy on | base off | base on |
| --- | ---: | ---: | ---: | ---: |
| phase prepass wall (s) | 6.6 | 16.1 | 146.3 | 332.1 |

`PrePass::Slice`'s per-layer parallel map is **wall-bracketed inside the
stage**: 2.26 s / 2.20 s / 56.04 s / 46.93 s (benchy-off, benchy-on,
base-off, base-on). Batch caches are small (≤1.8 s). `PrePass::Slice` was
not in ticket 27's work-item list but the stage table makes it the **#2 or
#3 serial block** on base (10.7–14.5% of prepass in the five instrumented
captures) — now measured at 47–56 s wall, second only to SupportGeometry.
It is promoted into this ticket's scope (its parent stage is the prepass
serial floor this ticket owns).

`PrePass::OverhangAnnotation` has no wall bracket this batch (its brackets
are all inside the per-layer rayon closure); its stage wall is bounded above
by the instrumented vintages (11.6–23.2 s base, t22-era captures) and its
accumulated interior is 224.8 s base-off / 138.4 s base-on across threads —
heavily parallel, ~12x worker sum over wall, i.e. the stage is **not serial
in its hot path**. The same reading applies to `shell_apply_opening`
(92.8 s / 480 calls accumulated inside Pass-1's par map on base-off; the
pass itself is 8.5 s wall) and `sa_contact_detect_sum` (171.7 s accumulated
inside `sa_contact_par_wall` = 14.7 s on base-on).

## Work item 2 — ShellClassification residual (base-on: stage ≈ prepass share)

Wall decomposition (base supports-on, wall-class brackets only; stage wall
from the instrumented vintage ≈ 41.1 s / after-instr, 10.1% of prepass
in shares):

| Sub-block | base-off | base-on | class |
| --- | ---: | ---: | --- |
| `gate_phase_b_commit_wall` | 30.29 s | 25.07 s | wall (serial commit loop) |
| `gate_qualification_wall` | 3.57 s | 3.07 s | wall (parallel region) |
| `shell_pass1_wall` | 8.46 s | 6.78 s | wall (parallel region) |
| `shell_bridge_gate_par_wall` | 1.25 s | 1.09 s | wall (parallel region) |
| Pass 2 top/bottom | 0.54 s | 0.49 s | wall |
| `shell_per_region_updates_total` | 9.04 s | 7.31 s | wall (serial region loop) |

Sum ≈ 44 s vs phase-wall share ≈ 10% of 332 s = ~34 s (brackets overlap the
same wall in places; treat as shape, not identity). The gate's serial phase B
(25–30 s) dominates the stage — inside it: the per-surface
`expand(closing(spacing))`/`intersection` chain plus `internal_bridge_angles`
over `deep_infill_clip_area` / `internal_unsupported_area` /
`expansion_area`. The `apply_opening` accumulation (74–93 s across threads,
990 calls) says Pass 1's round-join opening is the largest *worker* term; its
wall share is bounded by pass1+pass2 walls (≤ 8.5 s). §9.1's arc-tolerance-0
anomaly lives exactly here: `apply_opening` (`crates/slicer-runtime/src/slice_postprocess_prepass.rs`)
passes arc tolerance 0 to a Round join — every other round-join morphological
pass uses `MORPH_ROUND_ARC_TOLERANCE_MM` (0.05 mm). If canonical's
`opening_ex` really defaults to `jtMiter`, the parity fix (Miter, no arcs)
would also remove ~480–990 round-join arc-tessellations per run.

Ranked candidates: (a) apply_opening join parity (Miter) — representation
check + wall A/B; (b) phase-B `internal_bridge_angles` input
pre-reduction (it intersects each qualified polygon against three areas);
(c) qualification's `unsupported_span_areas` (32–35 s accumulated; wall
3.1–3.6 s — bounded, low priority).

## Work item 3 — SupportAnalysis (base-on)

| Bracket | base-on | class |
| --- | ---: | --- |
| `sa_contact_detect_sum` | 171.7 s | accumulated across threads |
| `sa_contact_par_wall` | 14.70 s | wall (parallel region) |
| `sa_slice_sweep_wall` | 24.68 s | wall (serial sweep) |
| `sa_layer_union_build` | 0.60 s | wall (serial) |

Stage wall ≈ 40–47 s (vintage-stable in the 5 instrumented captures, 10–12%
of prepass). The serial sweep (model_occupancy insert + bounds + contact-work
build, 24.7 s on base-on) is the biggest *serial* block in this stage; the
detect pass is parallel-saturated (171.7 s accumulated / 14.7 s wall ≈ 11.7x
on 12 threads). Ranked candidate: the sweep's per-region
`derive_needs_support` (a full `intersection_ex` per region×overhang-region)
— it also runs *again* per region at dispatch time (`derive_needs_support`
(`crates/slicer-wasm-host/src/marshal/prepared.rs`)); the arena cache landed
in ticket 05 may already cover the dispatch side, but the stage-side sweep is
re-deriving what `PrePass::MeshAnalysis` already knew
(`region_needs_support`, probe: 109–165 ns scale, negligible) — a
cross-check dedup, not a new algorithm.

## Work item 4 — MeshAnalysis / `compute_xy_footprint`

| Bracket | benchy | base off | base on | class |
| --- | ---: | ---: | ---: | --- |
| `mesh_classify_object` | 0.22 s | 23.75 s | 22.35 s | serial per object |
| `core:mesh_union_ex` (in compute_xy_footprint) | 0.13 s | 20.62 s | 19.40 s | inside classify |
| `mesh_xy_footprint_overhang` | 0.10 s | 17.96 s | 16.89 s | the overhang-facet union |
| `mesh_bridge_metrics_total` | 0.12 s | 5.56 s | 5.25 s | clusters + per-cluster unions |
| `mesh_find_bridge_clusters` | 0.09 s | 2.75 s | 2.62 s | half-edge map |

The overhang union is **the exact Clipper union over 326,546 projected
facets** (§6 lead 2): 17–18 s of the 22–24 s classify, i.e. ~75% of the
stage. Its consumers (same-run, verified in-tree): `region_needs_support`
uses the footprint **only through `expolygon_bbox`** (a bbox test —
`region_needs_support` (`crates/slicer-core/src/algos/mesh_analysis.rs`)),
`derive_needs_support` does one `intersection_ex` per region per
overhang-region (bbox-gated *inside* Clipper, no pre-filter), `prepass_slice`
`assemble_bridge_areas` intersects bridge footprints, and
`visual_debug_render` draws them. **No consumer needs the exact union for
its own sake; the overlap predicate is bbox-or-intersect.** Candidate: keep
the union but only for `is_valid` bridge clusters (small), and for the
overhang region replace the exact union with bbox coverage — requires an
output-equivalence check (needs_support flips) and a fairness-contract
disclosure. Wall prize ≈ 17 s serial on base (≈ 5% of the supports-on cell's
prepass).

## Work item 5 — OverhangAnnotation

All brackets are accumulated-across-threads inside the per-object/per-layer
parallel sweep; stage wall is not directly bracketed this batch.
`partition_into_bands` dominates the accumulated fuel (97.7% base-off):
per surviving layer it runs 3 × (Round-join `offset(previous, t)` +
`intersection_ex(current, grown)` + `difference_ex(within, previous)`) —
`oh_band_cumulative_step` (`crates/slicer-core/src/algos/overhang_annotation.rs`)
= 217.3 s of the 224.8 s total. The three thresholds are monotone: band k's
cumulative region ⊇ band (k−1)'s, so a single Round-join offset per threshold
is already minimal; the reducible part is `intersection_ex` + `difference_ex`
against *full-layer* current/previous geometry. Candidate (below-fold,
pending a wall bracket): derive band 4 = `overhang_area − cumulative[2]`
without re-intersecting, and reuse `grown_previous` across layers where the
previous layer is unchanged (it never is — geometry differs per layer, so the
real lever is representation-level, e.g. bounding-box-gated band arithmetic).
Stage wall evidence needed before graduating: one `_wall` bracket around
`annotate_overhangs`.

## PrePass::Slice (surfaced by item 1's deliverable; adopted into this ticket)

`slice_per_layer_par_map_wall`: 2.26 s benchy / 56.0 s base-off / 46.9 s
base-on — the parallel per-layer map dominates the stage (batch caches
≤1.8 s). Its internals are *not* yet bracketed (the per-layer body brackets
were cut when the probe module's crate placement was resolved; the runtime
wrapper sees only the region total). Next probe iteration: bracket
`classify_region_surfaces`, `assemble_bridge_areas`, `assemble_flat_bridge_areas`,
`apply_slice_closing_radius` inside
`execute_prepass_slice_single_layer_with_cache`'s layer closure — the
sub-bracket sums there are accumulated (parallel axis), paired with the
existing `_wall` bracket. This is the one stage where the ticket's "prepass
built-ins" list silently omitted the #2 serial module.

## Ranked next actions (keep/drop to the human)

1. **Bracket + attack phase B of the internal-bridge gate** (25–30 s serial
   wall base) — it is the largest confirmed serial block outside Slice, and
   its cost is per-qualified-polygon triple intersections against full-layer
   areas.
2. **`PrePass::Slice` internals** (47–56 s parallel wall): bracket the
   per-layer body, then rank classify vs bridge-assembly vs closing-radius.
3. **Overhang footprint union → bbox/coverage predicate** (17–18 s serial)
   — needs the consumer audit result above turned into an output-equivalence
   test before any A/B.
4. **apply_opening join-type parity** (Round→Miter) — small wall, clean
   parity argument, representation check required (arc removal changes
   contours only at sub-0.05 mm scale; must be disclosed).
5. **SupportAnalysis sweep dedup** — cross-check `needs_support` reuse;
   representation-neutral.

Measurement recipe notes for the next session: the probe adds ~4–5 stderr
lines per stage outside the JSONL stream; `_sum` names accumulate across
rayon threads (§3.3), `_wall` names are calling-thread wall; stage walls in
*this* batch are phase_complete deltas only for the prepass phase (no
`stage_complete` exists uninstrumented) — instrumented vintage stage walls
(t22 captures) remain the cross-check for stage shares, never for wall
claims.
