---
status: implemented
packet: layer-module-preparation_04_whole-print-input-views
task_ids:
  - TASK-574
---

# layer-module-preparation_04_whole-print-input-views

## Goal

Expose bounded, declaration-gated typed committed whole-print inputs to native and WASM preparation, with separate owner-selected source/configuration projections from the shared ordinary targeting seam.

## Problem Statement

The tree's `PrepassStageInput` has mesh and optional layer/slice/map/support inputs but no surface classification and is not a preparation resource. Existing ordinary views do not grant preparation access. A host borrow alone cannot give guest algorithms typed shell, bridge, surface and complete source/configuration context. Input transport is one bounded slice of TASK-574, separate from scheduling and private algorithm payloads.

## Architecture Constraints

- ADR-0066: same-module owner-private preparation, completed PrePass input and separate selected targets, no unfinished arenas/provider catalog. ADR-0045/0056: imported typed identity, independently versioned capability, one native/WASM selection model. No amendment or new ADR.
- No change to IR field layouts or schema constants; no guessed future schema version. Extend only the draft preparation capability before its production release, keeping row 03's qualified `interface()` identity. Plain old ordinary artifacts retain their worlds/imports/signatures.
<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and inspect its exit code: exit 0 means fresh, non-zero means stale (a distinct exit code signals `wasm-tools` is unavailable). Never use `rg -q 'STALE:'` — a `wasm-tools`-missing infrastructure error prints no `STALE:` and would read as fresh. If stale, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.
<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.
