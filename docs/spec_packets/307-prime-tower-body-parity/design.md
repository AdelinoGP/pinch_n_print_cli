# Design: 307-prime-tower-body-parity

## Controlling Code Paths

- Primary code path: `WipeTower::run_finalization` (`modules/core-modules/wipe-tower/src/lib.rs`) — receives all layers, currently skips tool-change-less layers and anchors purge blocks via `insert_entity_at`; this packet adds planning, `finish_layer` assembly via anchor-free `push_entity_with_priority`, and tool selection. Neighbouring path: `run_gcode_postprocess` (`modules/core-modules/machine-gcode-emit/src/lib.rs`) — the DEV-168 `printer_structure` time-lapse gate gains the smooth-suppression conjunction.
- Neighboring tests/fixtures: `modules/core-modules/wipe-tower/tests/wipe_tower_tdd.rs` (purge coexistence + default identity), `bed_bounds_tdd.rs` (max-depth validation), `finalization_live_tdd.rs` (insert/push coexistence — read-only reference, not extended), `modules/core-modules/machine-gcode-emit/tests/machine_gcode_emit_tdd.rs` (the ticket-27 `printer_structure` gate arms at its time-lapse section).
- OrcaSlicer comparison: see `requirements.md` §OrcaSlicer Reference Obligations; do not repeat delegation rules.

## Architecture Constraints

- The body lives in the existing module behind the existing seam: no new module, no new stage, no IR field, no WIT line, no claim (`[claims]` stays empty — wall/infill/brim are one pass's internal order, not competing algorithms, so rule 4 does not fire).
<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and inspect its exit code: exit 0 means fresh, non-zero means stale (a distinct exit code signals `wasm-tools` is unavailable). Never use `rg -q 'STALE:'` — a `wasm-tools`-missing infrastructure error prints no `STALE:` and would read as fresh. If stale, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.
<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.
- Consume, never rebuild: 254a's per-layer depth model + pitch + brim builder + framework forcing, 254b's interface purge block, 255's seven wall primitives + rotated frame — all FORWARD-DEPs on drafts, reconciled by name at implementation (landing order 254a → 254b → 255 → 307).
- Purge/body coexistence: purge blocks keep their anchored `insert_entity_at` positions; body entities use anchor-free `push_entity_with_priority` (which records into `priority_pushes`, a different merge channel — no anchor collision by construction). The `MissingToolchangePurge` emitter guard is satisfied on idle layers by the body's `WipeTower`-role entities.
- Intra-layer tool discipline holds: the body never synthesises a mid-layer tool change; forced-filament bodies sit at layer-boundary tool identity (ticket 29's one constraint), which the emitter already synthesises from the first entity's `tool_index`.

## Code Change Surface

- Selected approach: two-sided assembly. Tower side (`wipe-tower`): a planning pass over all layers (depths from 254a's per-layer model → backward max-propagation → max depth → idle-entry set), then per-layer `finish_layer` emission in canonical order calling 254a/255 builders, with the five new keys gating their arms; `timelapse_type`'s reader accepts canonical `"0"`/`"1"` spellings explicitly (AC-14, ticket-100 class). Suppression side (`machine-gcode-emit`): two reader declarations + one added conjunction on the existing injection gate, with the same `"0"`/`"1"` acceptance. No host, scheduler, IR, or WIT change.
- Exact functions, traits, manifests, tests, and fixtures:
  - `modules/core-modules/wipe-tower/wipe-tower.toml` — five `[config.schema]` tables (AC-1 exact specs).
  - `modules/core-modules/wipe-tower/src/lib.rs` — `WipeTower::from_config` (five reads), new `plan_tower_depths` (propagation + max + idle set honoring `no_sparse_layers`), new `emit_finish_layer` (inner perimeter → infill → 255 wall call → 254a brim call on first layer), `purge_depth_for`/`max_purge_depth` reuse (unchanged), bed-check call moved to planned max depth, `run_finalization` orchestration (plan → per-layer purge (unchanged) + body push → forced `tool_index` when `wipe_tower_filament > 0` with fatal range validation).
  - `modules/core-modules/machine-gcode-emit/machine-gcode-emit.toml` — two reader rows (`timelapse_type`, `enable_prime_tower`) with the ticket-104 duplicate-declaration comment shape.
  - `modules/core-modules/machine-gcode-emit/src/lib.rs` — `run_gcode_postprocess` gate: `&& !(timelapse_type == "smooth" && enable_prime_tower)` on the `time_lapse_gcode` injection arm only, with the canonical `"1"` spelling accepted identically.
  - Tests: NEW `modules/core-modules/wipe-tower/tests/wipe_tower_body_tdd.rs` (AC-3–AC-9, AC-11, AC-N2, AC-N4); arms in `wipe_tower_tdd.rs`, `bed_bounds_tdd.rs`, `machine_gcode_emit_tdd.rs`, `crates/slicer-scheduler/tests/integration/config_bounds_enforcement_tdd.rs`, `crates/slicer-runtime/tests/contract/config_view_binding_tdd.rs`; schema-guard extension in `wipe_tower_config_schema_tdd.rs` (254a's file).
  - `docs/DEVIATION_LOG.md` — append `DEV-201`, `DEV-202`, `DEV-203` rows.
- Rejected alternatives and reasons:
  - Host prepass built-in for planning (tickets 94/95 shape): rejected — planning needs no cross-layer IR the module cannot see; `run_finalization` already receives all layers with `layer-parallel-safe = false`.
  - Second `Layer::SlicePostProcess` coarse mutator for walls: rejected — the stage DAG admits at most one coarse `SliceIR` mutator before cycling (ticket 95's finding); walls are built inside this module, not beside it.
  - Host-mediated smooth-active flag (module-to-module signal): rejected — no such channel exists and inventing one is a schema change for a bool two duplicate declarations already deliver (ticket-104 precedent).

## Files in Scope (read + edit)

Target at most 3 primary files; the seam forces four plus tests — justified below.

- `modules/core-modules/wipe-tower/src/lib.rs` - role: planning + finish_layer + tool-select + smooth-tower; expected change: +3 blocks (~250 lines), no existing-behaviour edits except the bed-check call site
- `modules/core-modules/wipe-tower/wipe-tower.toml` - role: five key tables; expected change: +5 tables, nothing else
- `modules/core-modules/machine-gcode-emit/src/lib.rs` - role: suppression conjunction; expected change: +1 condition on one gate (justified: inseparable from smooth — ticket 31 folds it here; the alternative host flag is a schema change)
- `modules/core-modules/machine-gcode-emit/machine-gcode-emit.toml` - role: two reader rows; expected change: +2 tables with duplicate-declaration comments
- Tests + `docs/DEVIATION_LOG.md` + generated doc (via `gen-config-docs`) — per plan steps, capped at 3 edits per step.

## Read-Only Context

- `modules/core-modules/wipe-tower/src/lib.rs` - lines `660-775` only (`run_finalization` + test helpers `config_from_pairs`/`default_config`) - purpose: orchestration shape and fixture idiom
- `modules/core-modules/wipe-tower/src/lib.rs` - lines `345-410` only (`purge_depth_for`, `max_purge_depth`, `generate_purge_paths` signature) - purpose: depth/pitch inputs the planner consumes
- `modules/core-modules/machine-gcode-emit/src/lib.rs` - lines `200-310` only (`run_gcode_postprocess` + DEV-168 gate) - purpose: suppression conjunction site
- `modules/core-modules/wipe-tower/tests/finalization_live_tdd.rs` - lines `1-60` only - purpose: push/insert coexistence idiom (packet-58 migration note)
- `docs/spec_packets/254a-prime-tower-geometry-keys/packet.spec.md` - AC-1–AC-6 only - purpose: drafted helper names/shapes consumed as FORWARD-DEP
- `docs/spec_packets/255-wipe-tower-geometry-keys/packet.spec.md` - AC-1 + ordering section only - purpose: wall helper names + landing order

## Out-of-Bounds Files

- `OrcaSlicerDocumented/...` - delegate; never load
- `target/`, `Cargo.lock`, generated code, vendored dependencies - never load
- `docs/spec_packets/254a*/`, `254b*/`, `255*/` beyond the ranges above - reconcile names only; never edit another packet's files (their re-authoring is their own tickets' work)
- `crates/slicer-gcode/src/serialize.rs` (`ORCA_CONFIG_PADDING`) - never touch (rule 2)
- Unrelated crates - delegate symbol lookups; do not browse

## Expected Sub-Agent Dispatches

- Question: drafted helper names/shapes in 254a/254b/255 (depth fn, pitch expr, brim fn, wall fns, schema-guard filename); scope: `docs/spec_packets/254a-prime-tower-geometry-keys`, `254b*`, `255*`; return: `LOCATIONS`; purpose: Steps 1–3 FORWARD-DEP reconciliation
- Question: canonical `finish_layer`/`plan_tower` edge arithmetic (propagation band, max formula, solid rule arms); scope: `OrcaSlicerDocumented/src/libslic3r/GCode/WipeTower2.cpp`; return: `LOCATIONS`; purpose: Steps 2–3
- Question: `push_entity_with_priority` merge priority vs `insert_entity_at` anchors; scope: `crates/slicer-sdk/src/traits.rs`; return: `FACT`; purpose: Step 3 ordering guarantee

## Data and Contract Notes

- IR/manifest contracts: reads `LayerCollectionIR`, writes `LayerCollectionIR.wipe-tower` (unchanged); the five keys are module-read (no `ResolvedConfig` fields, no `to_config_map` arms — honest absence, 258/264 precedent); the two emitter rows are reader-only the same way.
- WIT boundary: untouched — body entities are `ExtrusionPath3D` with pre-existing `ExtrusionRole::WipeTower` (`crates/slicer-ir/src/slice_ir.rs`) on `RegionKey "__wipe_tower__"`; no new role, no new accessor.
- Determinism/scheduler constraints: planning is a pure function of (layers, config) inside the `layer-parallel-safe = false` module — no cross-module ordering beyond the 254a→254b→255→307 landing order; new entities carry no `order_lock` tags so ADR-0062/0063 are conformed by non-contact (locked-path bypass needs no code — there is nothing locked to bypass).

## Locked Assumptions and Invariants

- Tower-disabled default emits nothing new (AC-2) — the packet's reversibility lock; any geometry step breaking it is reverted, not re-baselined.
- Area conservation across the region set on body layers: body + purge + interface entities cover the planned footprint exactly once (no double-cover with 254b's interface block — interface depth is part of the per-layer purge depth the planner propagates, not an addition to it).
- `wipe_tower_filament = 0` is auto-identity forever; canonical's `Print::validate` range rule is the module fatal (AC-N2).

## Risks and Tradeoffs

- Producer drift (254a/254b/255 still draft): helper names may move — mitigated by Step 0 reconciliation (re-verify names before writing body code) and the contingency schema-guard authorship.
- Same-file merge churn with three producers: mitigated by landing order + per-arm test files (body tests live only in `wipe_tower_body_tdd.rs`, never in producers' files).
- Default-true `prime_tower_skip_points` changes tower-enabled defaults (open vs closed wall): intended and pinned (AC-11), not a silent drift; tower-disabled output is untouched.
- Single-tower union vs canonical Type1/Type2 split (DEV-201): Type1-only framework semantics ride a Type2-planned body — recorded, not resolved; `wipe_tower_type` stays unqueued.

## Context Cost Estimate

- Aggregate: `M`
- Largest step: `M` (Step 3 finish_layer; Step 5 smooth pair)
- Highest-risk dispatch and required return format: 254a/254b/255 helper-name reconciliation — `LOCATIONS`, ≤20 entries, before any body code.

## Open Questions

- None. `[FWD]` items (soluble-lookahead arm, emitter travel-avoid, `wipe_tower_type` selection) are recorded non-borrows with owners, not questions.
