---
status: implemented
packet: layer-module-preparation_06_preparation-plan-visuals
task_ids:
  - TASK-575
---

# layer-module-preparation_06_preparation-plan-visuals

## Goal

Publish optional owner-private typed XY diagnostic snapshots atomically with prepared plans, and render them through the existing visual-debug bundle path with truthful execution evidence.

## Problem Statement

Metadata-only preparation captures cannot show module-private planning geometry. Interpreting plan bytes on the host would violate private codec ownership; a separate drawing export would violate committed diagnostic capture. One optional typed snapshot published in the existing transaction solves both while preserving ordinary output.

## Architecture Constraints

- ADR-0066 owns optional committed XY diagnostics alongside immutable opaque pieces. No module drawing export, provider argument, new stage, ordinary IR accessor or lightning special case.
- ADR-0040's three tap classes describe scheduler-stage taps. The preparation selector is a fourth, non-stage capture class authorized by ADR-0066: its source is the post-Ready committed snapshot, not a Blackboard slot or `LayerArena`. In a mixed request the furthest stage tap still selects the ADR-0040 closure; the preparation tap adds no stage and does not alter it. ADR-0040's "none adds WIT" clause remains true of the three stage classes.
- ADR-0041 requires fail-closed selection. The strict `TapSelector` deserialization and named errors below are the mechanism: no preparation selection may disappear through the untagged `Detail` fallback.
- Additive companions avoid widening existing public request/capture/manifest/IR structs. Add a CapturedIr enum variant and update its true exhaustive arms in the same step; StageCapture layout remains unchanged. Existing hard-version assertions and constructor literals remain untouched.
<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and inspect its exit code: exit 0 means fresh, non-zero means stale (a distinct exit code signals `wasm-tools` is unavailable). Never use `rg -q 'STALE:'` — a `wasm-tools`-missing infrastructure error prints no `STALE:` and would read as fresh. If stale, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.
<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.
