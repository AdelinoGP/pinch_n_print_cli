---
status: implemented
packet: 130_infill-postprocess-contract
task_ids:
  - TASK-255
---

# 130_infill-postprocess-contract

## Goal

Make `Layer::InfillPostProcess` usable as the infill-linker's home: `run_infill_postprocess`
gains a read-only `prior-infill` input mirroring `InfillIR`'s region buckets (ADR-0028 Option
1b), and `perimeter-region-view` gains six fields — the four partitioned fill polygons plus
`tool-index` and `wall-source-region-id` — with `world-layer` bumped 1.0.0 → 2.0.0.

## Problem Statement

`Layer::InfillPostProcess` exists in `STAGE_ORDER` and the trait hook exists, but the stage is
unusable as the infill-linker's home: the host hands the hook a fresh empty builder (it cannot
read what `Layer::Infill` emitted), and `PerimeterRegionView` lacks the four partitioned fill
polygons plus any tool/wall-sharing identity, so a linker could neither re-clip against the
right boundary nor apply the wall-sharing-group connection predicate (ADR-0025 §Amendment).
Without this contract change, Architecture A (raw emit + central linker) cannot ship — every
downstream packet in the infill-parity roadmap (131–140) reads this contract.

## Architecture Constraints

<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and, if `STALE:` is reported, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.
- CLAUDE.md §WIT/Type Changes Checklist is binding: edit the canonical WIT at
  `crates/slicer-schema/wit/` only (host bindgen and guest macro both read it); search all
  `wit_host.rs`, `dispatch.rs`, and `wit_guest` modules for `PerimeterRegionView` /
  `perimeter-region-view` type identity; run `cargo build --tests` immediately after WIT
  edits.
- The commit stays replace (`layer_executor.rs:1768` untouched); the no-module
  preservation guarantee comes from the zero-iteration stage loop (`layer_executor.rs:288`)
  and is pinned by AC-N1, not by new host code.
- Per-region config stays invisible at this stage (single global `ConfigView`,
  `dispatch.rs:1634-1650`) — packet 131's concern; do not entangle it here.

## Data and Contract Notes

- IR contracts: `InfillIR` / `InfillRegion` / `ExtrusionPath3D` are READ, not changed — no IR
  schema bump expected; if implementation finds a struct change is unavoidable, that is a
  deviation to record, not silently absorb.
- WIT boundary: `world-layer` 1.0.0 → 2.0.0; every guest rebuilds; `wit_drift_detection_tdd`
  must assert the new types.
- The `prior-infill` view is read-only: the guest gets copies/views, never mutable access;
  the output path remains exclusively `InfillOutputBuilder`.
- Determinism: field population order does not matter, but `tool-index` precedence order does
  — pin it in one host function with the three-case unit/contract test (AC-3).

## Locked Assumptions and Invariants

- Option 1b, full-re-emit replace commit, and the six-field list are LOCKED (ADR-0028
  §Amendment). Do not re-open 1a, host-merge, or field trimming.
- `wall-source-region-id = None` means "owns walls"; `Some(base)` strictly means "shares the
  base's walls" — no third meaning may be invented later without an ADR amendment.
- The four polygon fields mirror `SliceRegionView`'s partitioned polygons exactly (same
  source data at dispatch); they are not re-derived or re-clipped here.
- Builder default for all six fields is empty/None so existing fixtures stay valid (AC-N2).

## Risks and Tradeoffs

- ~30-file sweep churn: mechanical but wide; mitigated by compiler-driven batching and
  empty/None defaults. This is the standard schema-bump cost (ADR-0002/0009/0010 precedent).
- The echo test-guest adds a new guest to the build set — it must join the shared test-guests
  target dir convention or `build-guests` won't cover it.
- Marshaling `prior-infill` copies path data across the boundary per dispatch — acceptable at
  current path counts; if profiling later objects, that is a packet-133+ concern, not a
  contract change.
