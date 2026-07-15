# Design: 130_infill-postprocess-contract

## Controlling Code Paths

- Primary code path: `crates/slicer-schema/wit/deps/ir-types.wit` (`perimeter-region-view`
  resource) + `crates/slicer-schema/wit/deps/world-layer/world-layer.wit`
  (`run-infill-postprocess` export) → `crates/slicer-macros/src/lib.rs` (guest glue) →
  `crates/slicer-sdk/src/{views.rs,traits.rs,test_support/fixtures.rs}` →
  `crates/slicer-wasm-host/src/{dispatch.rs,marshal/out.rs}` (population + marshaling).
- Neighboring tests or fixtures: `crates/slicer-runtime/tests/contract/` (new
  `infill_postprocess_*` tests + `wit_drift_detection_tdd.rs` update);
  `crates/slicer-wasm-host/test-guests/` (new postprocess echo guest, pattern the existing
  sdk-layer guests); `crates/slicer-sdk` builder tests.
- OrcaSlicer comparison surface: none — pure PnP contract work.

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

## Code Change Surface

- Selected approach (locked by ADR-0028 §Amendment): Option 1b — a read-only `prior-infill`
  parameter mirroring `InfillIR` buckets: per region `{ object-id, region-id,
  sparse-infill: list<extrusion-path3d>, solid-infill: list<extrusion-path3d>,
  ironing: list<extrusion-path3d> }`. The exact WIT shape (record list vs. resource with
  accessors) follows whichever idiom `ir-types.wit` already uses for list-of-record views —
  match the existing marshaling idiom rather than inventing one `[FWD]`.
- Exact changes: six fields on `perimeter-region-view` + accessors; `run-infill-postprocess`
  new param; SDK `PerimeterRegionView` struct/accessors/builder; `run_infill_postprocess`
  trait signature; macros arm; dispatch population (four polygons copied from the `SliceIR`
  region; `tool-index` precedence variant-chain material →
  `RegionMapIR::config_for(key).extensions["extruder"]` (`extensions` lives on the interned
  `ResolvedConfig`, not on `RegionMapIR` itself) → `DEFAULT_TOOL(0)`, reusing the extraction
  already used by `region_mapping.rs:645-676`; `wall-source-region-id` from the
  per-variant-PerimeterIR-entry absence check that `region_partition.rs:123-144` performs —
  hoist/share that predicate rather than duplicating it); marshal/out.rs; ~30-file sweep;
  echo test-guest; contract tests.
- Rejected alternatives: Option 1a pre-populated builder (rejected in the grilling — muddies
  write-only builder semantics); host-side merge commit (rejected — needs consumed-path
  bookkeeping; full-re-emit is simpler and testable).

## Files in Scope (read + edit)

Primary (semantic core — justifying >3: this is a contract packet; the surfaces are the
contract):
- `crates/slicer-schema/wit/deps/ir-types.wit` — six fields on `perimeter-region-view`.
- `crates/slicer-schema/wit/deps/world-layer/world-layer.wit` — `prior-infill` param + 1.1.0.
- `crates/slicer-sdk/src/views.rs` — struct + accessors (`PerimeterRegionView` at 521, impl at 531).
- `crates/slicer-sdk/src/traits.rs` — hook signature (`run_infill_postprocess` at 385).
- `crates/slicer-sdk/src/test_support/fixtures.rs` — builder setters.
- `crates/slicer-macros/src/lib.rs` — glue arm.
- `crates/slicer-wasm-host/src/dispatch.rs` — population arm (~435-454) only.
- `crates/slicer-wasm-host/src/marshal/out.rs` — marshaling.
- `crates/slicer-runtime/tests/contract/` — new tests + drift update.
- `crates/slicer-wasm-host/test-guests/<new echo guest>/` — new.
Sweep files (mechanical, compiler-driven): the ~30 constructors/matches — edited only where
`cargo check --workspace --all-targets` errors point.

## Read-Only Context

- `crates/slicer-runtime/src/layer_executor.rs` — lines 283-300 and 1700-1775 only — confirm
  loop-skip and replace-commit behavior for AC-N1 test design (the
  `LayerStageCommit::InfillPostProcess` arm is at 1768; 1706 is the
  `PerimetersPostProcess` arm, a different variant).
- `crates/slicer-runtime/src/region_partition.rs` — lines 112-145 only — the virtual-variant
  predicate to hoist (the `perim_index` build at 117 + missing-entry skip at 124-137; lines
  1-76 precede the target `sync_perimeter_infill_areas_into_slice` fn).
- `crates/slicer-core/src/algos/region_mapping.rs` — lines 640-680 only — material-tool
  extraction idiom (`chain_tool_index` at 645, `ToolIndex` extraction at 662).
- `crates/slicer-ir/src/slice_ir.rs` — lines 1770-2000 only — `ExtrusionPath3D` (1778),
  `InfillRegion` (1968), `InfillIR` (1983) shapes.
- One existing test-guest directory (e.g. an sdk-layer guest) — structure only, as the echo
  guest template.

## Out-of-Bounds Files

- `OrcaSlicerDocumented/**` — not needed; never load.
- `target/`, `Cargo.lock`, generated bindgen output — never load.
- `modules/core-modules/**` module bodies — sweep only touches constructors the compiler
  flags; do not browse modules.
- `docs/03_wit_and_manifest.md` in full — rg-targeted sections only.

## Expected Sub-Agent Dispatches

- "Run `cargo build --tests 2>&1 | tail -40`; return FACT pass or LOCATIONS of first-error
  batch (file:line + one-line error, ≤30 entries)" — after WIT edit (checklist step).
- "Run `cargo check --workspace --all-targets 2>&1`; group errors by file; return LOCATIONS
  ≤30 entries per batch" — drive the sweep; repeat until clean.
- "Run `cargo xtask build-guests --check`; FACT clean or STALE list; rebuild if stale and
  re-return" — after each guest-feeding edit wave.
- "Run `cargo test -p slicer-runtime --test contract 2>&1 | tee target/test-output.log | grep
  '^test result'`; FACT + counts; SNIPPETS ≤20 lines on failure".
- "rg `perimeter-region-view|PerimeterRegionView` across `crates/slicer-wasm-host/src`,
  `crates/slicer-macros`, guest shims; return LOCATIONS" — type-identity check (WIT
  checklist).

## Data and Contract Notes

- IR contracts: `InfillIR` / `InfillRegion` / `ExtrusionPath3D` are READ, not changed — no IR
  schema bump expected; if implementation finds a struct change is unavoidable, that is a
  deviation to record, not silently absorb.
- WIT boundary: `world-layer` 1.0.0 → 1.1.0; every guest rebuilds; `wit_drift_detection_tdd`
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

## Context Cost Estimate

- Aggregate: `M`
- Largest single step: `M` (the workspace sweep)
- Highest-risk dispatch: the sweep-error LOCATIONS batches — must stay grouped and ≤30
  entries; an ungrouped full compiler dump would blow the budget.

## Open Questions

- `[FWD]` Exact WIT shape of `prior-infill` (list of records vs resource view) — match the
  existing `ir-types.wit` idiom for list-of-record data; semantics (region-bucketed,
  read-only) are locked.
- `[FWD]` Whether the `region_partition.rs:123-144` virtual-variant predicate is hoisted into
  a shared helper or re-derived at dispatch — prefer hoisting; decide at the dispatch arm.
