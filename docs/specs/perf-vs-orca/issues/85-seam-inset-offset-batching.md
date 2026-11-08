# Seam planner inset-offset batching

Type: task
Label: wayfinder:task
Status: resolved (2026-10-25; human-approved KEEP — gate met in all four
pre-declared base classic cells: wall −7.46 / −2.72 / −7.03 / −3.86 %,
CPU +0.34…+0.47 %, 24/24 pairs byte-identical, zero exclusions; seam stage
8.36 s → 1.74 s = ~7x batch occupancy; candidate committed — see ## Answer)
Assignee: 2026-10-25 OpenCode wayfinder session — first frontier ticket, no
ticket named by the human ("continue the perf vs orca map")
Parent: [Slice performance vs OrcaSlicer](../map.md)
Blocked by: none
Graduated from: [Base common-floor attribution](84-base-common-floor-attribution.md)

## Question

Can the seam planner's 495 per-region inset offsets be issued as **one**
`offset_polygons_batch` call — with per-request exactness and entry order
preserved — and how much base-cell wall does the recovered parallelism deliver?

## Measured basis

[ticket 84](84-base-common-floor-attribution.md) decomposed
`PrePass::SeamPlanning` on the two base classic cells. The stage wall is
**92.4 / 93.2 %** one serial native call per region:

| term | classic-off | classic-on |
| --- | ---: | ---: |
| stage wall | 9.14 s (9.84 % of cell) | 8.97 s (4.30 % of cell) |
| scope `t84.offset_project` | **8,195 ms (92.4 %)** | **8,177 ms (93.2 %)** |
| candidate generation + scoring + harvest | 169 ms | 155 ms |
| host view projection | 4 ms | 11 ms |
| host harvest | 58 ms | 45 ms |

495 regions × one `host::offset_polygons` call at
`delta = -0.5 × chosen_width` (Miter, arc tolerance 0.0) in
`run_region_planning_entries`
(`modules/core-modules/seam-planner-default/src/lib.rs`). Each call is
independent: it insets one region's polygons so the chosen boundary vertex can
be projected onto the toolpath centreline
(`project_point_onto_inset_boundary`, `seam-planner-default/src/contours.rs`).

`offset_polygons_batch` already exists (`crates/slicer-wasm-host/src/host.rs`,
`crates/slicer-sdk/src/host_batch.rs`) and routes through the same
input-ordered, work-estimating `map_batch` fan-out t78 used for
`clip_polygons_batch`. No host capability work is required.

Measured ceiling (ticket 84's `summary.json`):

| cell | upper bound (all offset wall removed) | at t78's 2.6x effective |
| --- | ---: | ---: |
| base classic supports-off | 8.83 % of wall | 5.43 % |
| base classic supports-on | 3.92 % of wall | 2.41 % |

Both are far above the map's 0.80 % floor. The ceiling is **not** claimed as the
expected result: the candidate must measure its own batch occupancy, and a
`map_batch` effective speedup below **1.15x** on the seam request set is a
pre-declared DROP.

## Scope

Guest-side restructure only:

1. Pass A: for every admitted region, in the existing sorted/deduped order,
   compute the chosen candidate and record `(region, chosen_position)` plus one
   `OffsetRequest` (exact same polygons, delta, join, arc tolerance, no miter
   limit).
2. One `host::offset_polygons_batch` call for the whole print.
3. Pass B: project each chosen position onto its own returned inset and push the
   entry — same entry fields, same order, same fallback path.

No `SeamPlanIR` shape change, no seam-selection change, no host change. The
candidate must keep the zero-candidate fallback (`fallback_region_position`)
and the paint-enforcement scoring untouched.

## Pre-declared gate

- **Output identity (16/16)**: every cell of {benchy, base} × {classic, arachne}
  × {supports-off, on} byte-identical to its current reference SHA (t81's
  campaign set; the classic base refs are `292df501…` / `22200671…`).
  Determinism: one SHA per arm across repeats.
- **Standing paired A/B** on the two base classic cells, ordinary +
  accelerated, 6 alternating pairs each: every retained pair byte-identical,
  median wall must fall, and process CPU must not rise above **+3.0 %** on any
  cell (work-preserving ceiling, the t78 precedent).
- Disclose the measured batch occupancy (effective speedup) and the residual
  serial terms (candidates, choose, harvest, projection).

Out-of-scope for this ticket: reducing the candidate list, changing the inset
delta or the projection rule, and any geometry change. Those are separate
questions (ticket 84 measured the candidate list at 0.29 % of wall; a
representation-safe projection change is not priced).

## Answer

**Resolved 2026-10-25, human-approved KEEP. Gate met in all four
pre-declared cells** — wall falls, process CPU inside the work-preserving band,
every retained pair byte-identical, one SHA per arm, zero exclusions:

| cell | paired wall | paired CPU | pairs |
| --- | ---: | ---: | ---: |
| ordinary base classic off | **−7.46 %** | +0.45 % | 6/6 |
| ordinary base classic on | **−2.72 %** | +0.34 % | 6/6 |
| accelerated base classic off | **−7.03 %** | +0.47 % | 6/6 |
| accelerated base classic on | **−3.86 %** | +0.35 % | 6/6 |

The candidate is the ticket's scope, unchanged: pass A chooses every region's
seam and records its `OffsetRequest`, **one** `offset_polygons_batch` covers the
whole print, pass B projects each seam onto its own returned inset. Per-request
arguments are identical to the loop's (same polygons, `-0.5 × width`, Miter,
arc tolerance 0.0, no miter limit), entry order and the zero-candidate fallback
are untouched, and no IR shape or host capability changed
(`PendingSeamEntry` / `run_region_planning_entries`,
`modules/core-modules/seam-planner-default/src/lib.rs`).

**Batch occupancy ≈ 7x**, measured, not borrowed: the seam stage falls
**8,356 → 1,737 ms** (off) and **8,288 → 1,703 ms** (on) in instrumented
captures, and ticket 84's measured 7.55 % non-offset share of the same job's
guest call puts the offset term at ≈7,724 → ≈1,105 ms. The conservative bound
(all remaining stage time attributed to the offset term) is still **4.4x**, far
above this ticket's pre-declared **1.15x** DROP floor. t78's 2.6x pricing — the
number ticket 84 used to predict 5.43 % / 2.41 % — was too pessimistic, which is
why the measured off-cell recovery lands just under the 8.83 % "every offset
vanished" bound rather than at it. The stage is now 1.7 s of a 72 s cell (2.3 %)
and has stopped being a common-floor term; the residual is ≈210 ms of guest
compute (candidates / choose / unscoped) plus the unchanged host projection and
harvest (~50–70 ms).

**Output identity 16/16**: both arms byte-identical in every cell, the 8
ordinary cells equal to the post-t81 reference set (base classic off
`292df501…`, on `22200671…`), and cross-mode equality 8/8. The accelerated cells
have no post-t81 reference of their own (t81's campaign was ordinary-only), so
they are gated arm-to-arm and reported against the ordinary bytes.

**Parity/exactness**: `batched_inset_offsets_match_the_serial_reference_per_region`
(`modules/core-modules/seam-planner-default/src/lib.rs`) drives a multi-region
fixture — two different scoring widths plus a paint-blocked fallback region —
and asserts entry-for-entry equality with the pre-batching serial loop *and*
fixture-derived projected positions, so a result-to-region mis-association
cannot pass. No geometry changed, so the byte-identity regime is the real gate.

**One infra prerequisite was repaired inside this take** and is recorded as
[ticket 86](86-accelerated-shim-workspace-root.md): a cold accelerated build
died in `proc-macro2`'s build script because the rustc shim resolved the
workspace root from CWD. Without it the accelerated arm of this gate could not
be measured.

Disclosures: the two arms' `pnp_cli.exe` differ (the seam planner is linked into
the host through `slicer-integrated-modules`; both arms run
`--no-integrated-modules`, so the linked copy is inert and the guest module is
the only executing difference), guest fuel rises 3.858 B → 3.978 B (+3.1 %),
instrumented walls are attribution only, and the pre-campaign sanity pair taken
under desktop load is not used. Full method, tables and reproduce recipe:
[`evidence/t85-seam-inset-batching/`](../evidence/t85-seam-inset-batching/FINDINGS.md)
(`verify.py --self-test` exits 0 checkout-only, 8/8 negative controls rejected).

Frontier now: **no open child remains**; the fog and the next route are the
human's. Standing paired acceptance and KEEP/DROP rules unchanged.

