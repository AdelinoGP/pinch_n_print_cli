# Support Generation Remediation — Approved Plan

Status: approved (2026-08-11, grill-with-docs session)
Source: `docs/specs/support-generation-defect-verified-findings.md` (verified accurate).

## Problem

PNP's support generation is broken: with `enable_support: true`, the slicer emits
"support" inside the model's own cross-section on every layer instead of
supporting pieces under overhangs that rise from the build plate. Four verified
root causes (RC-1..RC-4) plus expanded scope (raft geometry, interface layers,
4-variant support model, G-code end-to-end).

## Verified root causes

- **RC-1** — support-planner lone propagated nodes emit nothing (no MST edge,
  `dist_to_top > 0`); columns vanish mid-air instead of reaching the build plate.
- **RC-2** — traditional-support and tree-support fallback fillers fill the whole
  region polygon, never clipped to `overhang_areas()`.
- **RC-3** — `needs_support` hardcoded `true` at the WIT boundary
  (`crates/slicer-wasm-host/src/marshal/in_.rs:410`); `SlicedRegion` has no field.
- **RC-4** — `tapered_radius` returns 0 at `dist_to_top == 0`; zero-width contact tips.

## Design decisions (resolved)

- **RC-1:** emit degenerate per-layer segments for lone surviving nodes
  (`dist_to_top > 0`, no MST edge, not dropped) — vertical columns, matching Orca
  `drop_nodes`/`draw_circles`.
- **RC-2:** fill `region.overhang_areas()` instead of `region.polygons()` in both
  fallback fillers (enforced regions still fill full `polygons()`).
- **RC-3:** `needs_support = !overhang_areas.is_empty()` at the marshalling boundary.
- **RC-4:** floor `tapered_radius` lower clamp at `MIN_BRANCH_RADIUS = 0.4`
  (keep `MAX_BRANCH_RADIUS_MM = 6.0`).
- **Raft:** new module at `Layer::Support`, claim `raft-generator`, reads
  `SupportPlanIR` (RaftPlan) + `SliceIR` + `LayerPlanIR`, writes `SupportIR.raft_paths`.
  Object footprint + margin; first/base/interface pattern per RaftPlan; negative-layer raft prefix.
- **Interface:** refactor to planner-plans / module-generates. Planner emits
  interface-layer plan data; support modules generate scan-line geometry for both
  top and bottom interface layers. Removes the code-1003 "not implemented" warning.
- **4-variant model:** keep the 2-way module split; add a separate auto/manual mode
  flag read by the planner. `tree(auto)/tree(manual)` → tree-support;
  `classic(auto)/classic(manual)` → traditional-support. Auto = overhang detection +
  enforcers; Manual = enforcers-only (planner skips `detect_overhang_facets`).
- **G-code e2e:** visual-debug gcode-mode verification that fixed support flows to
  final G-code with correct `SupportMaterial`/`SupportInterface` roles.

## Cross-cutting requirement

Visual-debug is an **inherent gate** on every geometry-changing packet (A–E), not
just the defect fixes. Each packet's acceptance criteria include a
`pnp_cli visual-debug` render + `manifest.json` check. Orca parity is
source-adjudicated + structural invariants + human-authored gcode visual-debug
(no Orca binary available, D-109/D-112).

## Packet Queue

| # | packet slug | goal (one sentence) | task ids | depends on | status | packet dir |
|---|-------------|---------------------|----------|------------|--------|------------|
| 1 | support-planner-defect-fix | Fix RC-1 (lone-node vertical columns) and RC-4 (tip-radius floor) in the support-planner. | TASK-322 | - | superseded | docs/spec_packets/213-support-planner-defect-fix/ |
| 2 | support-fallback-overhang-clip | Fix RC-2 (clip fallback fill to overhang_areas) and RC-3 (needs_support = has overhang) in the fallback fillers and marshalling boundary. | TASK-323 | - | superseded | docs/spec_packets/214-support-fallback-overhang-clip/ |
| 3 | raft-geometry | Add a new raft-generator module (Layer::Infill, claim:raft-fill per ADR-0009) emitting full raft geometry (object footprint + margin, first/base/interface pattern) as SupportIR.raft_paths, and migrate IR layer-index fields (LayerPlanIR/SliceIR/SupportIR) from u32 to i32 for negative raft prefix layers. | TASK-324 | #1 | generated | docs/spec_packets/215-raft-geometry/ |
| 4 | support-interface-layers | Refactor interface layers to planner-plans/module-generates and implement bottom interface layers, removing the code-1003 warning. | TASK-325 | #1 | generated | docs/spec_packets/216-support-interface-layers/ |
| 5 | support-type-variants | Implement the 4-variant support model (tree/classic × auto/manual) via a planner mode flag, keeping the 2-way module split. | TASK-326 | #1 | generated | docs/spec_packets/217-support-type-variants/ |
| 6 | support-gcode-e2e | Add visual-debug gcode-mode end-to-end verification that fixed support flows to final G-code with correct roles. | TASK-327 | #1,#2,#3,#4 | generated | docs/spec_packets/218-support-gcode-e2e/ |
