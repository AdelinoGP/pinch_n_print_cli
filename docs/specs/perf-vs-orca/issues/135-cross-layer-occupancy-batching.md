# Staged fold: canonical skirt-brim re-take and cross-layer occupancy offset batching

Type: task / staged re-take + BUILD candidate
Label: wayfinder:task
Status: **Stage 1 resolved 2026-11-10** (the canonical skirt/brim re-take);
  Stage 2 is open, unclaimed and still gated by 137
Assignee: dev (session 39-40 — the human said "Continue the perf vs orca map"
  (no ticket named); 135 Stage 1 is the first unblocked, unclaimed frontier
  ticket in order — 136 and 135 Stage 2 are gated by 137, and 137 is the
  human's)
Parent: [Slice performance vs OrcaSlicer](../map.md)
Blocked by: [137](137-planner-scheduling-gate.md) — **Stage 2 only** (the
  four-cell gate form must be confirmed). Stage 1 is unblocked and may run
  first (2026-11-09 fold ruling).
Graduated from: [Planner effective parallelism](134-planner-effective-parallelism.md)'s
  2026-11-09 ruling: the per-layer occupancy offsets are the largest serial term left in
  the planner (**14,249 ms wall / 14,169 ms thread** on base classic-on, **0.99x**), while
  the clip core already runs at **4.81x**. The derived ceiling is **11.2 s** at that fan-out.
  **Fold ruling (2026-11-09, [Matched settings vs the 8 mm brim](130-brim-matched-settings-ruling.md)):**
  this ticket is also the brim/skirt re-take — the canonical port + matched
  adhesion/priming values + reference re-record + scoreboard re-derivation are
  its Stage 1 (staged with separate gates; a fused single candidate is rejected
  as the t115 failure mode).

## Stage 1 (runs unblocked): the t130 folded re-take

The 2026-11-09 ruling closes both matched-settings asymmetries here, staged
before the candidate so attribution survives:

1. **Canonical port.** Replace `SkirtBrim::generate_brim_entities` /
   `generate_skirt_entities` (`modules/core-modules/skirt-brim/src/lib.rs`)
   bounding-box rectangles with canonical geometry ported from the pinned
   `v2.5.0` (`40ecc66f78`): `make_brim` (`Brim.cpp`) and the skirt construction
   in `Print.cpp` (object-convex-hull anchored). The port carries the standard
   OrcaSlicer attribution header (`docs/ORCASLICER_ATTRIBUTION.md`), and pins
   `min_skirt_length` semantics.
2. **Matched keys, both arms, fail-closed.** Author `brim_width = 8`,
   `skirt_loops = 6`, `skirt_distance = 3.0`, `skirt_height = 1` in the four
   matched configs (`evidence/matched-pair/configs/`); amend
   `New-OrcaOverrideArgs` (`--brim-width=0` → `--brim-width=8`, plus
   `--skirt-loops=6 --skirt-distance=3 --skirt-height=1`) and assert all four
   resolved values per run on both sides (`Test-OrcaEvidence` and a PnP-side
   appendix assert). The t103 pin doctrine re-scopes from "2.4-era values" to
   "the matched job".
3. **Fresh 16-cell output campaign that records the NEW reference set** —
   t123's candidate-ordinary set is retired as the currency and every
   acceptance citing it re-points. Arms byte-identical within themselves.
4. **Delta audit (the port's correctness gate):** against the old set, moved
   lines are confined to skirt/brim entities and the config appendix — zero
   non-skirt/brim structural geometry lines move. **Parity oracle:** PnP's
   emitted skirt/brim loops geometrically match Orca's emitted skirt/brim loops
   for the same job in all 8 cells, within a declared tolerance (canonical
   citations by file + function at `40ecc66f78`).
5. **Old-vs-new paired A/B** (the standing paired form, wall + CPU, interleaved)
   recording the port's cost per cell — attribution only; the port is mandated,
   not a KEEP/DROP candidate.
6. **Closing gate: the 8-cells × 3-tools scoreboard re-derivation** (standing
   matched-pair protocol with the t103/t98 pins and the new asserts),
   superseding t124's ratios of record. Until then they stand as historical and
   every scoreboard keeps the disclosure.

Disclosures: `skirt_per_object` (Orca chain `1`, PnP global) is single-object
moot in this matrix; `min_skirt_length` is pinned by the port; the Orca source
vs installed-binary non-identity caveat is unchanged.

## Stage 2 question: the cross-layer occupancy batch

[Planner effective parallelism](134-planner-effective-parallelism.md) measured the
planner's remaining serial term: `inflate_model_occupancy_memoized` still issues one
singular `host::offset_polygons` per active layer for the emit check (**433 calls /
14,169 ms thread / 14,249 ms wall** on base classic-on; **186 / 585 ms / 598 ms** on
benchy classic-on). The calls are independent across layers, and the emit pass already
buffers four layers for the clip batch (`EMIT_BATCH_LAYERS`). Batch the per-layer
occupancy payloads across the buffered layers with the same `offset_polygons_batch`
service the role carve uses, then serve both memo sites from the batch result.

The measured ceiling is **11.2 s** at the planner clip core's **4.81x** fan-out
(**8.7 s** at t78's 2.6x clamp, **13.0 s** at 12x), i.e. **11.1x** the 1,010 ms floor
and **12.1 %** of base classic-on's 92.5 s required prepass cut. It is a tranche, not
the route: the planner's whole scheduling potential is below the 50.66 s share t134
priced as the route threshold.

## Why this is not t125

[Ticket 125](125-strong-planner-occupancy-offset-batching.md) batched the **per-origin
role-carve** offsets inside the bounded flush and failed the gate in six of eight cells,
with base classic-on gaining wall but regressing process CPU. This shape batches the
**per-layer emit-check** payloads (the 433 unique offsets t132 did not memoise) across
the layer chunk, and leaves the per-origin carve exactly where t132 left it. t125's
failure is the control: the candidate must report the per-call wall and CPU of both
halves and must not silently move the role-carve half into the batch.

## Pre-declared gates

**Stage 2** runs under the **t117 four-cell form**, because the four supports-off
cells are structurally inert: the whole `PrePass::SupportGeometry` stage is
**0.64–0.81x** the 0.80 % floor there (t134 measured), so no planner-only change can pass
the literal all-8-cell form. The final form is
[137](137-planner-scheduling-gate.md)'s to rule:

- Byte-identical G-code in all eight cells on both build arms (against the Stage-1
  reference set).
- Positive median wall with CPU corroborating at 0.80 % in all four supports-on cells.
- No median wall or CPU regression in the four supports-off cells.

**Stage 1** gates: the 16-cell campaign + delta audit + parity oracle (above), the
old-vs-new paired attribution, and the closing scoreboard re-derivation. The old-vs-new
delta is expected and disclosed, not gated KEEP/DROP.

## Acceptance

- Stage 1: canonical port with the attribution header; all four keys authored and
  asserted per run on both arms; a fresh 16-cell output campaign recording the new
  reference set (every hash named in this ticket's closure); the delta audit (moved
  lines confined to skirt/brim + appendix) and the skirt/brim parity oracle; the
  old-vs-new paired A/B recording the port's cost; the 8-cells × 3-tools scoreboard
  re-derivation superseding t124's ratios of record.
- Stage 2: candidate build paired against the Stage-1 baseline; four supports-on cells
  at or above the decided floor with CPU corroborating, four supports-off cells
  regression-free; a measured conversion of the occupancy thread-time to wall on the
  candidate arm, so the 4.81x proxy is replaced by the batch's own fan-out.

## Answer

Stage 1 resolved 2026-11-10. The canonical port lands, the matched job is
authored and asserted on both arms, and the **new reference set** replaces
t123's. The strict skirt/brim parity oracle is **NOT met 8/8** — 2/8 — and the
residual is attributed below rather than hidden.

**The port.** `SkirtBrim::generate` builds the brim as the canonical annulus:
`offset(filled_outline, brim_object_gap)` to `offset(filled_outline, gap +
quantised_width)`, minus the first-layer **support region**, filled with
concentric lines half a spacing inside the outer boundary and one spacing per
pass, emitting both the outer and the hole boundary of each shrunken band
(`make_brim` / `tryExPolygonOffset`, `Brim.cpp`). The skirt is the convex hull
of the first `skirt_height` layers **plus the brim footprint**, offset by
`skirt_distance + spacing/2 + i x spacing` with `jtRound` and 0.1 mm arc
tolerance (`Print::_make_skirt`, `Print.cpp`). Canonical at the pinned
`v2.5.0` = `40ecc66f78`, OrcaSlicer attribution header present. Two manifest
keys added: `brim_object_gap` (0.1) and `min_skirt_length` (0; **non-zero fails
closed** — no filament cross-section model).

Two measured corrections to the t130 instruction:

- **`brim_type = outer_only` is a fifth matched key.** The vendor chain resolves
  `brim_type = auto_brim`, and under it `make_brim` recomputes the width from
  its adhesion heuristic and discards an explicit `--brim-width=8` (measured on
  benchy classic-off: a ~10 x 5 mm, 3-loop brim). The ruling's intent — one
  8 mm adhesion feature on both arms — is what landed.
- **Support clipping must be region-level, not bead-level.** Canonical subtracts
  the support *slice contour*; the IR carries bead centerlines, so the port
  closes the bead union at `2.5 x line_width` (1.0 mm) and fills it. The
  bead-exact variant left 17 brim loops where Orca prints none on both base
  supports-on cells, and clipping ring *lines* let the brim thread between
  support beads (470 of 1,401 brim points within 0.1 mm of a bead).

**Output campaign (16 cells, two build arms).** The baseline arm (pre-port
module, identical host binary) reproduces t123's eight hashes exactly, so every
move is attributable to the port. The candidate records the new reference set,
identical on both arms: `9012df71…`, `7b199bd1…`, `dbafeb9d…`, `c86af409…`,
`27dd2114…`, `e2c3c60e…`, `32ac8f77…`, `e5f5b8f4…` (full hashes in
`evidence/t135-skirt-brim-retake/reduced/summary.json`). **Every subsequent
acceptance that cites t123's set must re-point to this one.**

**Delta audit.** Every motion line outside `Skirt`/`Brim` is identical after the
extrusion field is stripped, in all eight cells; the extrusion field itself
differs by at most **0.00098 mm** (declared tolerance 1e-3 mm, an artefact of the
running extrusion accumulator). The config block gains the two new keys.

**Parity oracle (translation-invariant; bands declared from the benchy
classic-off probe, applied unchanged).** Presence parity **8/8**; brim radial
extent **8/8 within 0.46 mm**; skirt outer area 0.40 % on benchy supports-off,
8.2–8.4 % base supports-off, 4.5 % benchy supports-on, 11.0 % base supports-on.
**Strict 8/8: not met (2/8).** The brim-work axis fails because of (a) per-arm
flow-spacing quantisation — 8.0 mm at PnP's 0.4 mm spacing vs 7.31 mm at Orca's
0.457 mm (+9.7/+9.9 % on the benchy supports-off cells), (b) Orca's per-volume-
group clipping on the base part (+25.5/+26.4 %), and (c) **Orca's support brims
riding the same `; FEATURE: Brim` label** (`tree_support_brim_width = 3`, absent
on PnP's tree-support module — the +75 % on the benchy supports-on cells). The
supports-on skirt residual is the support hull, not the skirt. None of these
live in `com.core.skirt-brim`.

**A/B (3 interleaved pairs, same host binary both arms).** Wall ratios
0.975–1.013, CPU ratios 0.992–1.004 — the port's cost is below the box's paired
noise floor; no cell wins on wall with CPU corroborating in 3/3.

**Scoreboard re-derivation (8 cells x 3 tools x (1 warmup + 3 measured)).**
**2.364x–4.091x ordinary / 2.320x–4.122x accelerated; required cut
57.7 %–75.6 %.** 96 rows, zero exclusions; every PnP cell has one hash across
both build arms equal to the new reference set; 48/48 PnP runs exit 0,
`degraded=false`, zero fatal errors; every Orca run asserts the five matched
keys from its resolved appendix. **t124's ratios are superseded, not improved**:
both arms' adhesion work changed, so the two takes are not comparable.

**Disclosures.** `auto_brim` overrides `brim_width` (above);
`min_skirt_length > 0` fails closed; support clipping is a declared closing
radius; the skirt simplify is weaker than canonical's
`Geometry::simplify_polygons` (collinear-only), so PnP keeps more vertices;
source-vs-installed-binary caveat unchanged (t128); shared 6-physical /
12-logical-core box, idle load 0.9–6.5 % before the scoreboard. Evidence:
[`evidence/t135-skirt-brim-retake/`](../evidence/t135-skirt-brim-retake/FINDINGS.md)
(`verify.py` exits 0 checkout-only, 16/16 negative controls rejected).

**Stage 2 is untouched**: the cross-layer occupancy batch remains open,
unclaimed, and gated by [137](137-planner-scheduling-gate.md).

