---
status: implemented
packet: 238c-support-renderer-flow-interfaces
task_ids:
  - TASK-381
  - TASK-382
  - TASK-383
  - TASK-384
  - TASK-385
  - TASK-386
  - TASK-387
  - TASK-388
  - TASK-389
  - TASK-390
  - TASK-391
  - TASK-392
  - TASK-393
  - TASK-394
  - TASK-395
  - TASK-396
  - TASK-397
  - TASK-398
---

# 238c-support-renderer-flow-interfaces

## Goal

Make both support-family renderers canonically faithful in flow, density, and interface
semantics: hollow tree walls, flow-derived densities (no percent-as-fraction mis-scale),
canonical 10.0 mm branch-radius cap, raise-to-base under interfaces, canonical roof/floor
band counts, the base-interface (`num_top_base_interface_layers`) role end to end, one
shared `interface_regularize`, and the DEV-129/145/146 dispositions.

## Problem Statement

The support-family renderers are structurally present (packets 221/222) but their flow,
density, and interface semantics diverge from canonical OrcaSlicer in five measured ways:

1. **G-10** — tree branch bodies render **filled**; canonical renders hollow concentric
   sheath walls (`generate_toolpaths` → `tree_supports_generate_paths`). (238b's renderer
   work already landed inset-wall + scan-fill structure in `render_polygon` and the
   min-fill center line; what remains here is the canonical DENSITY model, direction
   alternation, and tip-solidity semantics — see Measured Baseline items 2/4.)
2. **G-11** — PnP over-extrudes support 1.107× vs Orca (flow per mm of path; gap-register
   measurement). Compounding it, `support_density` arrives percent-scaled (`20.0`) but is
   consumed as a fraction, so the `min(1.0)` clamp forces solid fill above 1.
3. **G-12** — `MAX_BRANCH_RADIUS_MM = 6.0` vs canonical 10.0.
4. **G-13** — the canonical raise-to-`base_radius` rule under interfaces is absent.
5. **G-18** — at top=2/bottom=2 interface layers, traditional emits 2
   `;TYPE:Support interface` blocks vs Orca's 3: the roof/floor band structure is not
   canonical (`number_of_support_interface_bottom_layers`; `draw_circles` floor block).

Plus three structural debts owned here: **F-37 piece 2** (the base-interface
`num_top_base_interface_layers` role end-to-end — piece 1, regularization wiring, landed
in commit `050d5c3a`), the byte-identical duplication of `interface_regularize.rs` across
both renderers (explicitly flagged as the consolidation target by that commit; still
byte-identical on disk 2026-08-25), and
deviations DEV-129 (bottom-interface diagnostic claim vs implemented truth), DEV-145
(premise corrected in plan §10 — the key IS canonical with default 0.5; PnP defaults
−1.0), DEV-146 (interface pitch derives from generic `line_width`; canonical derives from
the interface flow width). This is one coherent slice because all items share the two
renderer modules, one config/manifest surface, and one WIT/IR role carrier.

## Architecture Constraints

- Invariant 16 (plan §6): every acceptance command names explicit `--exact` test names or
  asserts matched-count non-zero in the same run — all AC commands tee to
  `target/test-output.log` and guard `grep -c '^test .* ok$' > 0`.
- E1 (no vacuous assertions): hollow-wall proof asserts path STRUCTURE (wall-loop count +
  interior pitch), not artefact existence; density proof asserts computed ratios against
  closed-form values.
- E6 (feature-gated blindness): slicer-core test commands carry `--features host-algos`.
- E8/E9: snake_case keys everywhere; mm↔unit conversions only at declared boundaries.
- T8 (silent config defaults): every newly-read key gets a manifest `[config.schema]`
  entry + regenerated `docs/15_config_keys_reference.md` in the same commit.
<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and inspect its exit code: exit 0 means fresh, non-zero means stale (a distinct exit code signals `wasm-tools` is unavailable). Never use `rg -q 'STALE:'` — a `wasm-tools`-missing infrastructure error prints no `STALE:` and would read as fresh. If stale, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.
<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.

Additional mandatory constraint (schema/version bump): the F-37 schema-version bump must
be authored in the SAME step that adds the variants, together with every test hard-asserting
the old version literal; locked wire formats stay pinned at their constructor literals.

## Data and Contract Notes

- IR/manifest contracts: `SupportPlanEntry.roles` carries
  `SupportPlanRoleRegion { role, regions }`; new role rides this record — no new entry
  type. `SupportPlanEntry.skeleton` carries the LANDED
  `points` + `wall_counts` parallel lists (equal lengths; count 0 = plain node, ≥1 =
  extra walls) — the renderer currently ignores it and this packet wires consumption.
  Manifest keys snake_case; undeclared keys silently default (E9/T8) so every read
  key must be declared.
- WIT boundary: canonical sources only (`crates/slicer-schema/wit/`); both `bindgen!` and
  guest macro read them; after edits `cargo build --tests` then rebuild guests.
- Determinism/scheduler constraints: no new claims; role attribution happens inside the
  existing planner claim window; serial/parallel determinism preserved (pure functions of
  plan entries + config).
- CLI evidence contract: any slice run for evidence MUST include
  `--module-dir modules/core-modules` (support modules otherwise silently fail to
  register — zero support blocks, no error) and use `--model <path>` for the input.

## Locked Assumptions and Invariants

- `;TYPE:` label decision LOCKED: `ExtrusionRole::SupportBaseInterface` →
  `;TYPE:Support interface`.
- Density formulas LOCKED to canonical forms (AC-2); no alternate scaling selectable
  (Ruling 8 applies to knobs replacing legitimate behavior, not defect fixes).
- `ee27ac94` top-count pins LOCKED passing (contact-inclusive anchoring not regressed).
- Regularize behavior byte-conserved: moved code + tests assert identical outcomes.

## Risks and Tradeoffs

- Removing `support_density` changes user-visible config surface; mitigated by deviation
  row + regenerated config docs + human-gate inspection.
- Radius-cap raise can move golden endpoints; E3 classification required before any
  rebless (expected: none, since clamped-at-6 branches are rare in the benchy fixture).
- WIT addition ripples generated bindings across crates; contained by same-step
  blast-radius ownership and `cargo build --tests`.
- Floor-band expansion may shift interface block counts in OTHER configurations (top=1,
  bottom=-1); AC-5 pins top=2/bottom=2 plus regression-pins the `ee27ac94` rows.
- Shared `slicer-core` helper grows guest dependency surface for both modules (already
  dependents; no new linkage risk).
- Per-component fill means many small regions per layer: a naive per-region fill pass is
  fine, but any cross-region optimization (e.g. chaining scan lines across components)
  must preserve deterministic ordering — keep fills per region in plan-entry order.
- The ~30-vs-~50 tip-count delta (AC-15) may partly reflect Orca's denser tip seeding
  rather than band semantics alone; AC-15 requires reconciling only what band semantics
  own and recording the residual explicitly.
