# `PrePass::Slice` parity Phase-1: wave expansion and the facet-path retirement (P2)

Type: task / Phase-1 parity analysis
Label: wayfinder:task
Status: resolved (2026-11-06; split verdict — **STOP** on the combined
  expansion shape, **BUILD** the classification-only retirement; the expansion
  zones do not exist at `PrePass::Slice`, and the expansion consumes 102.6 % of
  the classification saving on benchy; classification-only clears all eight
  floors at 12.5x–24.5x (benchy) / 89.7x–213.9x (base) — see ## Answer)
Assignee: 2026-11-06 OpenCode wayfinder session — the human said "Continue the
  perf vs orca map" (no ticket named); the frontier's first takeable ticket is
  122 (117 is the human's grilling).
Parent: [Slice performance vs OrcaSlicer](../map.md)
Blocked by: none
Graduated from: the fog entry **"The canonical bridge pipeline's second half:
  wave expansion and the facet-path retirement"** (surfaced 2026-11-03 by
  [t112](112-prep-slice-parity-phase1.md), gated on [ticket
  116](116-prep-slice-canonical-bridge-classification.md) landing — it did,
  2026-11-05).

## Question

Can PnP port canonical OrcaSlicer's bridge **expansion** half —
`expand_expolygons` → `wave_seeds` / `propagate_waves_ex` into the shell, sparse
and top expansion zones, then `merge_bridges`' `closing_ex` trim — retire the
facet-derived `assemble_bridge_areas` path, and show under t111's 16-cell
contract that the output delta is **bounded** and the ceiling clears the 0.80 %
floor in all eight cells?

## Context — what t112 left

- Canonical classifies **every** unsupported bottom `stBottomBridge` (now
  implemented by [ticket 116](116-prep-slice-canonical-bridge-classification.md)
  for PnP's flat family), then **grows** those bridges into the surrounding
  shells / sparse / top zones by wave propagation and **trims** those zones by
  the expanded result (`LayerRegion::process_external_surfaces`,
  `expand_bridges_detect_orientations`, `merge_bridges`).
- PnP's other half is mesh-facet classification:
  `assemble_bridge_areas` (`crates/slicer-core/src/algos/prepass_slice.rs`) reads
  `SurfaceClassificationIR.bridge_regions`, offsets by a fixed
  `expansion_margin_mm` (1.0 mm) and **trims nothing**. That fixed offset is not
  `propagate_waves_ex`; canonical's wave expansion has no PnP counterpart.
- t112 measured canonical's unsupported-bottom set at **1,307 mm²** (benchy) /
  **4,655 mm²** (base) = **2.58x / 9.12x** PnP's flat family, so the
  classification-level delta is material, and it **cannot be bounded** until the
  expansion exists. That is why P2 was deferred, not rejected.
- Scope note from t116: the flat family is now canonical; the facet family, the
  1.0 mm margin and the missing zone trim are the remaining divergence. The
  consumer map is t112 §1 (`wave-overhangs`, `classic`/`arachne-perimeters`,
  `gyroid`/`rectilinear-infill`, `infill-linker`, `BridgeSpatialIndex`, paint
  splits, visual-debug).

## Scope

1. Phase-1 parity analysis first, no production edit: read canonical's
   `expand_expolygons`, `wave_seeds`, `propagate_waves_ex`, `detect_bridge_directions`
   and `merge_bridges`; map every input, output and trim onto PnP's IR
   (`SlicedRegion`, `SurfaceClassificationIR`, the `bridge_areas` consumers).
2. Bound the delta: enumerate, on both heavy fixtures, how much area moves
   between `bridge`, `internal_bridge`, `top`, `bottom` and `sparse` roles, and
   whether any consumer outside those roles can move.
3. Price the ceiling: the removed facet classification + 1.0 mm offset work
   against the added wave expansion and trim, per cell, against the 0.80 % floor.
4. Only if (2) is bounded and (3) clears the floor in all eight cells: propose
   the candidate under t111's contract (fresh 16-cell reference set, both arms
   byte-identical within themselves, delta audited before timing, paired wall/CPU
   gate). No production edit before that.

## Stop rule

If the expansion's output delta cannot be bounded on the two heavy fixtures, stop
and report the bound — do not ship a shape whose G-code movement is unbounded.
Do not weaken the all-8-cell floor, and do not fold this into a facade over the
existing 1.0 mm offset.

## Deliverable

A Phase-1 analysis with the canonical-to-PnP mapping, the measured delta bound,
the eight-cell ceiling arithmetic, and a BUILD / STOP recommendation. If BUILD, a
candidate ticket with the exact shape and its acceptance contract.

## Evidence starts

- [`evidence/t112-prep-slice-parity/FINDINGS.md`](../evidence/t112-prep-slice-parity/FINDINGS.md)
  §1 (consumer map), §3 (P2 shape and why it was deferred), §5 (the 2.58x / 9.12x
  classification-level set).
- [`evidence/t116-prep-slice-canonical-bridge/FINDINGS.md`](../evidence/t116-prep-slice-canonical-bridge/FINDINGS.md)
  — what the flat family now does, and the delta-audit method this ticket reuses.
- Canonical `PrintObject::detect_surfaces_type` (`PrintObject.cpp`),
  `LayerRegion::process_external_surfaces` / `expand_bridges_detect_orientations` /
  `merge_bridges` (`LayerRegion.cpp`) and `propagate_waves_ex`
  (`BridgeDetector.cpp` / `LayerRegion.cpp`).
- `grep -rn "expansion_margin_mm" crates modules` — the fixed 1.0 mm offset that
  would be retired.

## Answer

**Split verdict: the classification half is a BUILD; the expansion half is a
STOP.** They cannot ship together, and the expansion is mis-staged as a
`PrePass::Slice` change.

### The stage finding

Canonical's `LayerRegion::process_external_surfaces` expands bridges into three
zones — `stInternalSolid` (shells), `stInternal` (sparse) and `stTop` (top) —
after perimeters and fill surfaces exist. In PnP:

- `PrePass::Slice` has `polygons`, `infill_areas` (= `polygons` at that stage)
  and `bridge_areas`; it has none of the three zones;
- `PrePass::ShellClassification` later populates top/bottom/internal-solid;
- `sparse_infill_area` and `perimeter.infill_areas` do not exist until the
  `Layer::Perimeters` commit partition
  (`sync_perimeter_infill_areas_into_slice`, `crates/slicer-runtime/src/region_partition.rs`).

So a faithful port of `expand_expolygons` / `wave_seeds` / `propagate_waves_ex`
/ `merge_bridges` is a per-layer partition change, not a prepass change. A
prepass version would need a second source of truth for the fill zones. This
alone disqualifies the combined shape under this ticket's `PrePass::Slice`
framing.

### Measured delta (bounded)

Committed layers only, both heavy fixtures; the probe reconstructs production
inputs and asserts fidelity against
`execute_prepass_slice_single_layer_with_cache` on six sampled layers. It
reproduces t112's canonical areas (1,306.72 / 4,653.28 mm² vs 1,307.30 /
4,654.62) and t116's flat areas (447.16 / 438.76 vs 447.27 / 438.92).

| fixture | current bridge | canonical bridge | added | removed | expansion reach bound | combined movement |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| benchy | 706.73 | 1,306.72 | 618.44 | 18.45 | 300.64 | 937.53 |
| base | 4,473.93 | 4,653.28 | 564.17 | 384.82 | 513.85 | 1,462.83 |

Areas in mm². The expansion reach is the wall-inset zone-proxy upper bound
(the solid/top reach 1.2928 mm contains the sparse reach 0.35708 mm, so one
band bounds all three zones). `merge_bridges`' closing shrinks the union
slightly (−1.32 / −9.86 mm²). The current facet path's polygon sum before the
gate is 14,232.05 / 419,106.22 mm²; at most 5.0 % / 1.1 % survives. The benchy
bridge set grows 1.85x under canonical classification; the base set is 1.04x.

Consumer confinement: `bridge_areas` feeds the direction update, the
internal-bridge gate, the `bridge > bottom > top > sparse` partition, paint
splits, `BridgeSpatialIndex`'s per-vertex `is_bridge`, the six guests and visual
debug. The expansion trims exactly top/internal-solid/sparse; it does not target
bottom. The only consumer outside the four fill roles is the bridge-derived
per-vertex flag. Porting canonical's `detect_bridge_directions` would rotate
lines *inside* the bridge polygons — a separate consumer-visible rewrite not
priced here; the candidate must keep PnP's direction path or audit it
separately.

### Eight-cell ceiling

Floors are 0.80 % of the t121 post-merge ordinary walls; multiples are the
probe's net thread-time over the floor.

| cell | floor | classification-only | combined (tight) |
| --- | ---: | ---: | ---: |
| benchy classic-off | 45.84 ms | 24.50x | **−0.63x** |
| benchy classic-on | 85.12 ms | 13.19x | **−0.34x** |
| benchy arachne-off | 48.08 ms | 23.36x | **−0.60x** |
| benchy arachne-on | 90.08 ms | 12.47x | **−0.32x** |
| base classic-off | 448.64 ms | 211.42x | 200.33x |
| base classic-on | 1,057.92 ms | 89.66x | 84.96x |
| base arachne-off | 443.44 ms | 213.90x | 202.68x |
| base arachne-on | 994.88 ms | 95.34x | 90.34x |

The expansion's cost is **102.6 %** of the classification saving on benchy
(1,151.9 vs 1,122.9 ms thread-time) and **5.2 %** on base (4,976.6 vs
94,852.4 ms). The all-8-cell gate therefore fails for the combined shape: all
four benchy cells are negative. The classification-only shape clears all eight.

### Recommendation

- **STOP** the combined expansion shape. No candidate ticket.
- **BUILD** the classification-only retirement: delete
  `classify_region_surfaces`, `assemble_bridge_areas`,
  `assemble_flat_bridge_areas` and the redundant
  `gate_bridge_areas_by_unsupported_span`; classify canonical's whole
  unsupported bottom set with the same opening. Graduated as
  [ticket 123](123-facet-bridge-path-retirement.md), with the exact shape, the
  measured delta, the test moves, the `DEV` row for the unported expansion, and
  t111's 16-cell acceptance contract.
- Do not fold the expansion in as a facade over the 1.0 mm offset: the offset
  is deleted, and the missing expansion is recorded as a divergence.

Evidence:
[`evidence/t122-wave-expansion/`](../evidence/t122-wave-expansion/FINDINGS.md)
— `verify.py` exits 0 checkout-only; `--self-test` rejects 14/14 negative
controls.
