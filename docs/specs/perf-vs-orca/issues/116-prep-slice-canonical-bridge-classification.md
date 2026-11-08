# `PrePass::Slice` candidate: canonical flat-bridge classification (P1)

Type: task
Label: wayfinder:task
Status: resolved (2026-11-05; **BUILD — 8/8 cells clear the 0.80 % floor at 11.1x–30.5x with CPU corroborating**, the 16-cell output contract met and the delta audit finding 0 structural geometry lines; see ## Answer)
Assignee: 2026-11-04 OpenCode wayfinder session — the human said "continue the perf vs orca map, pick up the next afk ticket" (no ticket named; 117 is the human's grilling, so 116 is the first AFK frontier ticket in order)
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

## Answer

**Verdict: BUILD.** The canonical rule replaces a PnP-local discriminator that had
no canonical counterpart, is output-bounded to the bridge family, and clears the
standing gate in every cell.

### What shipped

`assemble_flat_bridge_areas`
(`crates/slicer-core/src/algos/prepass_slice.rs`) applies canonical OrcaSlicer's
classification — `opening_ex(flat_bridge, 0.1 × external_perimeter_width,
Miter, 3.0)` — over PnP's flat family and keeps every surviving component. The
caller adds canonical's `ApplySafetyOffset` (inflate the lower cross-section by
`SCALED_EPSILON` = 1 PnP unit = 1e-4 mm, square join) before
`difference(current, lower)`, and skips the assembly when the object has no
previous cross-section, mirroring `PrintObject::detect_surfaces_type`'s
`lower_layer == nullptr ⇒ stBottom`. Deleted: the 12 mm `closing_ex(support)`, the
`closed − support` refill mask, the per-component `≥ 10 %` fraction gate,
`MAX_FLAT_BRIDGE_SPAN_MM`, `FLAT_BRIDGE_ENCLOSURE_RADIUS_MM`,
`FLAT_BRIDGE_ENCLOSURE_MIN_FRACTION`, `parse_flat_bridge_join` and
`expoly_set_area`. Added `resolve_external_perimeter_width_mm`, mirroring
`resolve_contact_params` (`outer_wall_line_width` extension when absolute, else
typed `line_width`, else 0.4 mm), covered by four unit tests. The
`flat_bridge_closing_join` key is retired from `ResolvedConfig` (declaration,
`PartialEq`, `Hash`), `docs/config/host-keys.toml`,
`docs/15_config_keys_reference.md`, the host-keys doc-lock test, and the scheduler
manifest's enum-metadata assertions.

Tests re-derived: the two core flat-bridge tests (`flat_bridge_canonical_rule_keeps_gap_and_free_edge`,
`flat_bridge_opening_drops_sub_width_sliver` — the closing-join vertex-count guard
retired with the closing), the two runtime bridge-detector fixtures' assertions
(behaviour preserved, comments corrected), and the wedge e2e
(`wedge_multi_layer_top_bottom_evidence`): the y-extension free-edge underside is
now a `;TYPE:Bridge`, so the whole-file `Bottom surface` count is exactly one (the
build-plate base) and the per-layer assertions require `Bridge` at print_z 29.2.

### Evidence

- **16-cell output: met.** Baseline ordinary ≡ baseline accelerated ≡ the t115
  post-merge reference hashes in all eight cells; candidate ordinary ≡ candidate
  accelerated; the candidate moves every cell. Byte deltas range from −213 B
  (benchy arachne-off) to +2,637 B (base classic-off).
- **Delta stop rule: not triggered.** `audit_delta.py` reports **0 structural
  geometry lines** in all eight cells. Every differing `Outer wall`, `Inner wall`,
  `Support` or `Support interface` line is an **extrusion-token-only** change on
  an unchanged `G1 … X Y Z … F` move (the per-vertex bridge flag feeds wall flow);
  the other structural entries are the config appendix and `M73` counters. No
  coordinate, feedrate, travel, skirt, brim or raft line moved.
- **Paired gate: 8/8**, three interleaved pairs per cell, `RAYON_NUM_THREADS=12`:

| cell | baseline | candidate | Δ | × floor |
| --- | ---: | ---: | ---: | ---: |
| benchy classic-off | 7.23 s | 5.79 s | 1.44 s (20.0 %) | 25.0x |
| benchy classic-on | 11.58 s | 10.21 s | 1.37 s (11.9 %) | 14.8x |
| benchy arachne-off | 7.27 s | 5.94 s | 1.34 s (18.4 %) | 22.9x |
| benchy arachne-on | 11.75 s | 10.12 s | 1.64 s (13.9 %) | 17.4x |
| base classic-off | 106.83 s | 83.98 s | 22.85 s (21.4 %) | 26.7x |
| base classic-on | 176.41 s | 160.73 s | 15.68 s (8.9 %) | 11.1x |
| base arachne-off | 106.39 s | 80.39 s | 26.00 s (24.4 %) | 30.5x |
| base arachne-on | 185.47 s | 164.41 s | 21.06 s (11.4 %) | 14.2x |

  Process CPU corroborates in every cell (−16.6 s benchy, −258…−289 s base).
  Seven of eight cells are strictly separated on wall and all eight on CPU; the
  exception is base classic-on, where the candidate's slowest pair (174.90 s) is
  0.55 s above the baseline's fastest (174.35 s) — disclosed, not smoothed.

### Ledger corrections and disclosures

- The baseline check uses the **t115 post-merge** reference set, not the post-t81
  set named in this ticket's deliverable: the ticket was written before the merge
  and HEAD no longer produces the post-t81 hashes.
- After the key retirement **no declared config key carries `wire_type: "enum"`**,
  so the scheduler manifest's enum-metadata assertions retire with the key; the
  macro's enum branch stays (it is schema-bridge surface), but it has no live
  instance.
- The campaign was interrupted twice — a session recycle killed the first A/B run
  mid-pair-2, and the human paused it overnight — so pairs 1–2 and pair 3 were
  measured ~4 h apart. Each pair measures both arms back-to-back; per-pair deltas
  are committed. Host load was not independently measured (the slicer saturates
  its own 12 threads).
- No Orca arm: the post-merge Orca scoreboard remains
  [ticket 121](121-post-merge-per-layer-dispatch-regression.md)'s closing
  measurement, and no ratio of record is quoted here. Both arms carry the t121
  per-layer regression, so the absolute walls are the merged tree's.
- The safety offset is numerically subsumed by the opening at 0.04 mm; it is
  implemented for canonical fidelity, not as a measured effect.

Evidence: [`evidence/t116-prep-slice-canonical-bridge/`](../evidence/t116-prep-slice-canonical-bridge/FINDINGS.md)
— `run_t116.ps1`, `audit_delta.py`, `reduce_t116.py`, `verify.py` (`verify.py`
exits 0 from a plain checkout with 316 checks; `--self-test` rejects 10/10
negative controls) and `reduced/summary.json`.
