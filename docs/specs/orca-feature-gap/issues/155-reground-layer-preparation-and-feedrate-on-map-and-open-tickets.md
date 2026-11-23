# 155 — Re-ground layer-preparation (`ADRs 0066/0071/0073/0074`) and feedrate (`ADR-0072`) across Map Notes and open tickets

Type: research
Status: open
Assignee: —
Blocked by: —
Map: ../map.md

## Question

**How did `origin/master`'s `layer-module-preparation` series (ADRs 0066, 0071, 0073, 0074) and context-aware feedrate amendment (`ADR-0072`) invalidate or reshape `map.md`'s standing architecture Notes, open tickets `133`, `134`, and `148`, and the cross-layer / speed draft packets?**

Two major non-config architectural shifts landed on `origin/master` while this map paused:

### Part A — `layer-module-preparation` (ADRs 0066, 0071, 0073, 0074)
- **Private whole-print preparation (`LayerPreparation::prepare_print`, `crates/slicer-sdk/src/preparation.rs`, ADR-0066)** and **lightning migration (`LightningInfill::prepare_print`, `modules/core-modules/lightning-infill/src/preparation.rs`, ADR-0074):** `LightningTreeIR` was retired; whole-print planning moved inside the consuming Layer module instead of a host prepass + public plan IR crossing WIT.
  - Check `map.md`'s standing Note *"Before correcting a tier-table owner away from a module…"* (which still teaches `LightningTreeIR → lightning-infill` as the cross-layer analysis+apply idiom) and define the replacement architectural rubric: when does cross-layer work use private `LayerPreparation::prepare_print`, when a public prepass IR, and when a host built-in?
  - Check open ticket [148 — Rule the home of the slice-mutating passes](./148-rule-slice-mutating-pass-home.md) (conical overhang `297`, XY size comp `305`, elephant foot `303`, polyhole `304`) and draft packet `306-interlocking-beams-slice-postprocess` (which explicitly copied the retired `LightningTreeIR` pattern): can `prepare_print` host slice-mutating or interlocking planning, or does `prepare_print` read slices read-only while `Layer::SlicePostProcess` still hits the single-mutator DAG cycle? Check `STAGE_ORDER` (`crates/slicer-scheduler/src/execution_plan.rs`) vs `run_prepass` (`crates/slicer-runtime/src/prepass.rs`) as well.
- **`[[region_split]]` vs `paint_only = true` (`crates/slicer-scheduler/src/manifest.rs`, ADR-0071)** and **full source variant-chain identity (`SupportAnalysisIR::family_assignments`, `crates/slicer-ir/src/slice_ir.rs`, ADR-0073):**
  - Check which standing Map Notes / fog patches (e.g., seam-plan painted variants, support packets `260a`/`260b`/`265`/`298`, prime-tower / bed-exclude packets `256`/`307`) are affected or unblocked by full variant-chain identity and `paint_only`.

### Part B — Context-aware feedrate resolution (`ADR-0072`) & emitter/perf shifts
- `DefaultGCodeEmitter::resolve_feedrate` now delegates to `role_base_speed_mm_per_s` and `resolve_extrusion_feedrate` (`crates/slicer-gcode/src/emit.rs`), supporting zero-configured role speeds derived from live tool/width/height/flow context while keeping the factor contract.
  - Re-check open ticket [133 — Retire the invented `max = 300.0` on the three module-owned speed keys](./133-retire-invented-speed-maxima-on-module-owned-speeds.md) and open ticket [134 — Retire classic-perimeters' module-private wall-speed factor bases](./134-retire-classic-perimeters-speed-factor-bases.md) against `crates/slicer-gcode/src/emit.rs` and `modules/core-modules/classic-perimeters/src/lib.rs`: do their premises still hold, and did `perf-vs-orca` or `config-scope-resolution_10` already touch any of those sites?
  - Identify every speed/emitter contract change from ADR-0072 and `perf-vs-orca` that Phase 2 speed packets (`253`, `255`, `262b`, `263`, `282`–`285`, `289`–`293`) must design against.

### Deliverable

- A linked markdown asset (`./155-asset-layer-prep-and-feedrate-regrounding.md`) giving verified symbol-level answers for Part A and Part B, exact updates needed for `map.md` Notes, premise updates for open tickets `133`, `134`, and `148`, and an architecture checklist for [158 — Triage all 52 draft packets for architectural drift, SOTA quality gaps, and Phase 2 batching](./158-triage-draft-packets-for-drift-and-phase2-batching.md).
- **Read-only research ticket:** do not edit `map.md` or resolve `133`/`134`/`148` here.

### Obligations

- Cite in-tree code by symbol name + crate-qualified path; never bare line numbers.
- Re-derive every symbol and behaviour from the merged tree at claim time.

## Answer
