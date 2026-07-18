# Requirements: 130_infill-postprocess-contract

## Packet Metadata

- Grouped task IDs:
  - `TASK-255`
- Backlog source: `docs/07_implementation_status.md`
- Packet status: `implemented`
- Aggregate context cost: `M`

## Problem Statement

`Layer::InfillPostProcess` exists in `STAGE_ORDER` and the trait hook exists, but the stage is
unusable as the infill-linker's home: the host hands the hook a fresh empty builder (it cannot
read what `Layer::Infill` emitted), and `PerimeterRegionView` lacks the four partitioned fill
polygons plus any tool/wall-sharing identity, so a linker could neither re-clip against the
right boundary nor apply the wall-sharing-group connection predicate (ADR-0025 §Amendment).
Without this contract change, Architecture A (raw emit + central linker) cannot ship — every
downstream packet in the infill-parity roadmap (131–140) reads this contract.

## In Scope

- `crates/slicer-schema/wit/deps/ir-types.wit`: `perimeter-region-view` gains
  `sparse-infill-area`, `top-solid-fill`, `bottom-solid-fill`, `bridge-areas` (all
  `list<ex-polygon>`), `tool-index` (`u32`), `wall-source-region-id` (`option<region-id>` or
  the WIT-idiomatic equivalent used by existing optional ids).
- `crates/slicer-schema/wit/deps/world-layer/world-layer.wit`: `run-infill-postprocess` gains
  a read-only `prior-infill` input mirroring `InfillIR`'s region buckets
  (`object-id`, `region-id`, sparse/solid/ironing path lists); version 1.0.0 → 1.1.0.
- `crates/slicer-sdk/src/views.rs`: `PerimeterRegionView` fields + accessors;
  `crates/slicer-sdk/src/traits.rs`: `run_infill_postprocess` signature gains the prior-infill
  parameter; `crates/slicer-sdk/src/test_support/fixtures.rs`: builder setters.
- `crates/slicer-macros/src/lib.rs`: bindgen glue for the new fields/param.
- `crates/slicer-wasm-host/src/dispatch.rs` (~lines 435-454 arm): populate the four polygons
  from the `SliceIR` region, derive `tool-index` (variant-chain material →
  `RegionMapIR::config_for(region_key).extensions["extruder"]` → 0) and
  `wall-source-region-id` (absence of a per-variant `PerimeterIR` entry per
  `region_partition.rs:123-144`), marshal `prior-infill`
  from the blackboard's committed `InfillIR`; `crates/slicer-wasm-host/src/marshal/out.rs`:
  new-field marshaling.
- Workspace blast-radius sweep: every exhaustive construction/match on `PerimeterRegionView`
  (~30 files per the 2026-07-01 grep survey) gains the new fields (empty/None defaults where
  the test doesn't care).
- New contract tests + a postprocess echo test-guest (pattern: existing
  `crates/slicer-wasm-host/test-guests/*`); update
  `crates/slicer-runtime/tests/contract/wit_drift_detection_tdd.rs` for the new types.
- Full guest rebuild ceremony (`cargo xtask build-guests`).

## Out of Scope

- Any linking logic, overlap offset, or clipping (packet 133).
- `LayerStageCommit::InfillPostProcess` changes — replace stays; the full-re-emit contract is
  the module's obligation (ADR-0028 §Amendment point 2).
- `InfillIR` struct changes — the prior-infill input is a read-only view of the existing IR;
  no IR schema bump (deviation from ADR-0028's pre-amendment text, which anticipated a bump;
  record in the closure log if the implementation confirms no struct change).
- Per-region config delivery (packet 131), modifier splits (packet 132).

## Authoritative Docs

- `docs/adr/0028-infill-postprocess-contract-prior-ir-and-partitioned-polygons.md` (~200 lines) — load in full; §Amendment 2026-07-01 is binding.
- `docs/adr/0025-infill-linker-as-raw-emit-post-pass.md` §Amendment — only the predicate/field rationale (delegate a SUMMARY).
- `docs/03_wit_and_manifest.md` (large) — rg-targeted sections only (`world-layer`,
  `perimeter-region-view`, claim catalog untouched here).
- `docs/02_ir_schemas.md` (large) — `InfillIR` section only, to confirm no struct change.
- `CLAUDE.md` §WIT/Type Changes Checklist — the edit ceremony (canonical WIT at
  `crates/slicer-schema/wit/`; `cargo build --tests` after WIT changes; type-identity search
  across `wit_host.rs` / `dispatch.rs` / `wit_guest` modules).

## Acceptance Summary

- Positive cases: `AC-1`–`AC-6` in `packet.spec.md`. Refinements: AC-1's echo guest must echo
  per-region counts keyed by `(object_id, region_id)` — a flat total hides bucket mixups;
  AC-3's three precedence cases live in one test fn or three; AC-6 expects `rg -c` ≥ 6 hits on
  ir-types.wit.
- Negative cases: `AC-N1` (absent module preserves IR), `AC-N2` (existing contract suite
  unchanged with defaulted fields).
- Cross-packet impact: 131 and 133 read this contract; any field rename here invalidates their
  packet text — do not rename after activation of those packets.

## Verification Commands

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo build --tests` (after WIT edits) | WIT checklist step; catches bindgen breaks early | FACT pass/fail |
| `cargo check --workspace --all-targets` | blast-radius sweep complete | FACT pass/fail |
| `cargo clippy --workspace --all-targets -- -D warnings` | lint gate | FACT pass/fail |
| `cargo xtask build-guests --check` (then rebuild if STALE) | guest freshness after WIT/SDK edits | FACT clean/STALE list |
| `cargo test -p slicer-runtime --test contract 2>&1 \| tee target/test-output.log \| grep "^test result"` | contract suite incl. new tests | FACT + counts |
| `cargo test -p slicer-sdk --features test 2>&1 \| tee target/test-output.log \| grep "^test result"` | SDK builder/view tests (`test_support` is feature-gated; plain `-p slicer-sdk` does not compile — pre-existing) | FACT + counts |
| `rg -q 'wall-source-region-id' docs/03_wit_and_manifest.md && rg -q 'prior-infill\|prior_infill' docs/05_module_sdk.md && echo DOCS-OK` | Doc Impact greps | FACT |

## Step Completion Expectations

- Cross-step invariant: after Step 1 (WIT + SDK types), the workspace is EXPECTED to be
  red until Step 3 completes the sweep — do not "fix" intermediate redness by reverting WIT
  changes; the checklist order is WIT → build --tests → sweep.
- Step ordering rationale: host population (Step 2) precedes the sweep (Step 3) because the
  dispatch arm is the semantic core; the sweep is mechanical churn validated by
  `cargo check --workspace --all-targets`.

## Context Discipline Notes

- Large files in the read-only path that MUST be ranged or delegated:
  `crates/slicer-wasm-host/src/dispatch.rs` (>1500 lines — the postprocess arm ~435-454 and
  the region-view builder only), `crates/slicer-sdk/src/views.rs` (~600+ lines — the
  `PerimeterRegionView` at 521 only), `docs/03_wit_and_manifest.md` (rg-targeted only).
- Likely temptation reads: the full ~30-file blast-radius list — do NOT open them
  speculatively; drive the sweep from compiler errors (`cargo check --workspace --all-targets`
  output delegated, errors grouped by file).
- Sub-agent return-format hints: the sweep dispatch returns LOCATIONS (file:line + error
  one-liner, grouped, ≤30 entries per batch); cargo gates return FACT.
