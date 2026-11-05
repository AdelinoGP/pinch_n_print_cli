# `PrePass::Slice` parity Phase-1: wave expansion and the facet-path retirement (P2)

Type: task / Phase-1 parity analysis
Label: wayfinder:task
Status: open
Assignee: (unclaimed)
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
