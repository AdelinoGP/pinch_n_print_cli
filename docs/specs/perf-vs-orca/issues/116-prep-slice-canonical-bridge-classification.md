# `PrePass::Slice` candidate: canonical flat-bridge classification (P1)

Type: task
Label: wayfinder:task
Status: open
Assignee: (unclaimed)
Parent: [Slice performance vs OrcaSlicer](../map.md)
Blocked by: none
Graduated from: [ticket 112](112-prep-slice-parity-phase1.md)'s Phase-1
recommendation (2026-11-03: build P1 — replace the enclosure gate with canonical
flat-bridge classification).

## Question

Can the enclosure gate in `assemble_flat_bridge_areas`
(`crates/slicer-core/src/algos/prepass_slice.rs`) be replaced by canonical
OrcaSlicer's flat-bridge classification — `opening_ex(flat_bridge, 0.1 ×
external_perimeter_width, Miter, 3.0)`, every surviving component kept — and pass
the standing paired gate at the **0.80 %** floor in **all eight** cells with the
pre-declared output delta confined to flat-unsupported bridge area?

## Context — what t112 established

- Canonical has **no enclosure closing**. `PrintObject::detect_surfaces_type`
  marks every unsupported bottom `stBottomBridge` after
  `opening_ex(diff_ex(slices, safety_offset(lower_slices), Yes),
  external_perimeter_width / 10)`. The only canonical bridge closing is
  `merge_bridges`' `closing_ex` at ≈0.15–0.17 mm on the bridge ∪ wave-anchor
  surface. PnP's 12 mm support-set closing is a local discriminator with no
  canonical counterpart and no `DEV` row.
- **Ceiling** (measured at `2dade816`): `min(closing thread-time, PrePass::Slice
  stage wall) − canonical opening` = **1,893 ms** on the benchy cells and
  **37,682 ms** on the base cells, i.e. **19.1x–62.5x** the 0.80 % floor in all
  eight cells. The closings are 86.8 % / 70.8 % of the stage's own accumulated
  thread-time; the replacement opening costs 89.9 / 246.7 ms (146x / 853x
  cheaper).
- **Delta** (measured on both heavy fixtures): **182 / 239** benchy and
  **437 / 469** base committed layers change; net **+8.86 mm² (+2.0 %)** /
  **+115.90 mm² (+35.9 %)** against the gate's emitted flat area. The gate
  rejects 68.4 / 187.2 mm² of free-edge and cantilever area canonical calls
  bridges; the 0.04 mm opening removes 59.58 / 71.27 mm² of sub-0.08 mm features.
  Layer 0 also pays a 12 mm closing whose result
  `gate_bridge_areas_by_unsupported_span` clears (595.8 ms benchy, 1.1 ms base).
- **Arachne cells share the classic prepass cost** by source structure:
  `PrePass::Slice` precedes shell/perimeter generation and reads only
  `slice_closing_radius` and `flat_bridge_closing_join`.
- Evidence: [`evidence/t112-prep-slice-parity/`](../evidence/t112-prep-slice-parity/FINDINGS.md)
  (`verify.py --self-test` exits 0, 9/9 negative controls).

## Scope

1. Replace the closing and the fraction gate in `assemble_flat_bridge_areas`
   with the canonical opening. Delete `FLAT_BRIDGE_ENCLOSURE_RADIUS_MM`,
   `FLAT_BRIDGE_ENCLOSURE_MIN_FRACTION`, `MAX_FLAT_BRIDGE_SPAN_MM` and the
   `closing_ex` import.
2. Add canonical's `ApplySafetyOffset::Yes`: inflate the **lower** slice set by
   `SCALED_EPSILON` = 1e-4 mm = **1 PnP unit** before the difference, so
   boundary-coincident material is not labelled bridge.
3. Resolve `external_perimeter_width` as
   `resolve_contact_params`
   (`crates/slicer-runtime/src/builtins/support_analysis_producer.rs`) does:
   `outer_wall_line_width` extension when present, else `line_width`. Do not
   hardcode 0.4 mm.
4. Retire the now-unused `flat_bridge_closing_join` key from the reference doc
   and the manifests in the same change.
5. Re-derive the enclosure unit tests in
   `crates/slicer-core/tests/algo_prepass_slice_tdd.rs`,
   `crates/slicer-core/tests/bridge_false_site_gating_tdd.rs` and
   `crates/slicer-runtime/tests/unit/bridge_detector_tdd.rs` under the canonical
   rule, and re-record the reference G-code set under the 16-cell contract.
6. Audit the candidate-vs-old-reference G-code delta **before** timing, then run
   the standing paired ordinary/accelerated gate.

## Stop rule

If the audited G-code delta escapes the flat-unsupported bridge area (perimeters,
support geometry, tool paths outside `bridge_areas`), stop and report it as a
parity bug, not noise. If the paired gate fails the 0.80 % floor in any of the
eight cells, apply the standing KEEP/DROP rule; do not narrow the gate to four
cells. Do not touch the facet-derived `assemble_bridge_areas` path or
`expansion_margin_mm` — that is P2, deferred.

## Deliverable

A candidate patch plus the 16-cell proof: both arms byte-identical within
themselves, the baseline reproducing the post-t81 set, the audited G-code delta,
and the paired wall/CPU table with the floor multiple per cell. Evidence under
`evidence/t116-prep-slice-canonical-bridge/` (`verify.py` exits 0 from a plain
checkout).

## Evidence starts

- [`evidence/t112-prep-slice-parity/FINDINGS.md`](../evidence/t112-prep-slice-parity/FINDINGS.md)
  — canonical difference, consumer map, ceilings, delta, and the exact shape.
- [`evidence/t112-prep-slice-parity/ceilings.json`](../evidence/t112-prep-slice-parity/reduced/ceilings.json)
  — the eight-cell arithmetic.
- Canonical `PrintObject::detect_surfaces_type` (`PrintObject.cpp`) and
  `LayerRegion::process_external_surfaces` / `merge_bridges` (`LayerRegion.cpp`).
- `grep -rn "closing_ex(" crates modules` — the Slice stage's only `closing_ex`
  caller is `assemble_flat_bridge_areas`; `apply_slice_closing_radius` uses
  `polygon_ops::offset`.
- [`evidence/t59-flat-bridge-closing/FINDINGS.md`](../evidence/t59-flat-bridge-closing/FINDINGS.md)
  — why no shape can preserve the gate's verdicts (the artefact field).
- [`evidence/matched-pair/run_scoreboard.ps1`](../evidence/matched-pair/run_scoreboard.ps1)
  and `configs/pnp-classic-supports-{off,on}.json`.
