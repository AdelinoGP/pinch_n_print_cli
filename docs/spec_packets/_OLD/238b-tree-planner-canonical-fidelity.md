---
status: implemented
packet: 238b-tree-planner-canonical-fidelity
task_ids:
  - TASK-369
  - TASK-370
  - TASK-371
  - TASK-372
  - TASK-373
  - TASK-374
  - TASK-375
  - TASK-376
  - TASK-377
  - TASK-378
  - TASK-379
  - TASK-380
---

# 238b-tree-planner-canonical-fidelity

## Goal

Bring the tree planner's algorithms to canonical fidelity — top-Z gap, smoothing
reinstatement, role coexistence, circle fidelity, collision/avoidance keying, miter limits,
`TreeVolumes` construction, `to_buildplate` inflation, `move_out_expolys`, STUDIO-4252 retry
args, mesh-path shim boundary, branch-A roof counter, tree styles, and emit simplify gating —
so every remaining recorded divergence (orca-divergences 1.1–5.7, 7.1–8.1; DEV-141..144) has a
canonical-matching implementation or a reasoned, tested deviation.

## Problem Statement

The tree planner (`modules/core-modules/tree-support-planner/src/lib.rs`, ~5.9k lines, port
of canonical `TreeSupport.cpp`) reached packet-224 parity on its main paths, but fourteen
recorded divergences remain open: orca-divergences rows 1.1, 2.1, 2.2, 2.3 (superseded in
part by the 224 carve work — residual is role coexistence + simplify gating), 3.2, 3.3,
4.1, 4.2, 4.3, 4.4/4.5, 4.6/5.6, 5.1, 5.5, 5.7, 7.1, 7.2, 8.1, plus DEV-141 (smoothing kernel
premise), DEV-142 (emit simplify gating), DEV-143 (f64 vs truncating integer smoothing),
DEV-144 (per-node extra-wall transport). Each is small alone; together they are one coherent
slice because they all land inside one planner module and share its guest-WASM test surface.
238a declared the keys (`support_style`, `max_bridge_length`); this packet makes their
behaviors exist (Ruling 3) and sizes DEV-128 for Ruling 4.

Canonical evidence below was pre-verified against the local checkout on 2026-08-22 by a
delegated probe (Q1-Q10); where it contradicts the plan brief, it wins and the contradiction
is recorded as a plan correction (`design.md` §Plan Corrections): DEV-141's premise names a
`clip_narrow_corner` kernel that does not exist inside this checkout's `smooth_nodes`, and
canonical `move_out_expolys` never restores `from0`.

## Architecture Constraints

<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and inspect its exit code: exit 0 means fresh, non-zero means stale (a distinct exit code signals `wasm-tools` is unavailable). Never use `rg -q 'STALE:'` — a `wasm-tools`-missing infrastructure error prints no `STALE:` and would read as fresh. If stale, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it. (This packet's primary surface IS a guest module — every step that touches `modules/core-modules/tree-support-planner/**`, `crates/slicer-ir/**`, `crates/slicer-schema/wit/**`, or the SDK host services MUST clear the check before trusting a green suite; T7 additionally demands one real-mesh validation because crate-suite green can hide empty-plan regressions.)
<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.
- E9 snake_case: any new config-key string consumed here (`support_style`) stays snake_case.
- Invariant 16: no verification command may match zero tests; every command asserts its
  matched-pass count in-run.

## Data and Contract Notes

- IR/manifest contracts: `SupportPlanIR` schema bumps MINOR (additive skeleton payload field,
  derived-at-activation per 237 precedent — no frozen future version literals in code; the
  constant literal changes in the same step as its test fallout). No manifest edits here
  (declarations are 238a's).
- WIT boundary: two additive edits — `record offset-request` optional `miter-limit` (+
  singular `offset-polygons` overload param), and `record support-plan-skeleton` gains the
  parallel `wall-counts: list<u32>` field (same length as `points`; NOT a
  `support-plan-entry` field) — canonical sources under `crates/slicer-schema/wit/deps/`
  (both host bindgen! and guest include_str! read them). `cargo build --tests` immediately
  after each WIT edit; rebuild guests in the SAME step.
- Determinism/scheduler constraints: no new claims; planner keeps its existing claim set;
  serial determinism unchanged (all edits deterministic given identical inputs); invariant
  15 (per-region entries) untouched.

## Locked Assumptions and Invariants

- F-14 exception LOCKED: the per-descendant move-pass recompute tests RAW outlines forever
  (canonical does the same); no later packet may "fix" it.
- Body/nozzle-sweep disjointness, structured declines, family attribution, support-disabled-
  emits-nothing (invariants 1–14) hold throughout; nothing here weakens a gate to get green.
- The smoothing decision's outcome is locked once recorded; flipping it later re-runs the full
  human gate.

## Risks and Tradeoffs

- T7 (highest): crate-suite green hiding empty/near-empty plans on real meshes. Mitigation:
  wedge real-mesh slice is a REQUIRED human-gate artifact, not optional; visual-debug taps at
  changed-boundary layers.
- Golden drift: circle-fidelity + simplify gating WILL move `benchy_tree_support_regression_*`
  endpoints. E3: classify first; rebless only with justification recorded; tolerances frozen.
- Largest-part carve may drop thin slivers users previously saw printed — canonical-faithful,
  but call it out in the evidence file.
- WIT additive edits ripple into both marshal legs (T9 skew hit 3× historically): both legs
  change in ONE step, verified by the seam-identity test 236 exports
  (`native_and_wasm_layer_views_are_field_identical`).
- Emitting unsimplified fine circles grows serialized plan payloads (DEV-142's retention
  reason); measure the delta during AC-13 and record it rather than silently accepting.
