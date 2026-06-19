# Design: support-plan-raft-plan-and-raftinfill-role

## Controlling Code Paths

- Primary code paths:
  - `crates/slicer-ir/src/slice_ir.rs::SupportPlanIR` — additive `raft_plan: Vec<RaftPlan>` field; new `RaftPlan` + `RaftLayerSpec` structs; `ExtrusionRole::RaftInfill` variant; schema_version minor bump.
  - `crates/slicer-sdk/src/views.rs::should_emit` — new role/claim arm (post-74710fa location confirmed via Step 1 dispatch).
  - `modules/core-modules/support-planner/src/lib.rs` — replace placeholder raft block (lines 381-423) with `RaftPlan` emission.
  - `modules/core-modules/traditional-support/src/lib.rs` — extend lead `//!` block with one explicit non-consumption sentence.
- Neighboring tests/fixtures:
  - `crates/slicer-ir/tests/support_plan_ir_schema_version_bumped.rs` — new file (AC-7).
  - `crates/slicer-sdk/tests/should_emit_raft_fill_claim.rs` — new file (AC-N2; or extension to an existing test file if one is conventional).
  - `crates/slicer-runtime/tests/integration/raft_plan_emission_tdd.rs` — new file (AC-4, AC-5, AC-N1).
  - `docs/02_ir_schemas.md` — extended.
- OrcaSlicer comparison surface: see `requirements.md` §OrcaSlicer Reference Obligations.

## Architecture Constraints

<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and, if `STALE:` is reported, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.

<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.

- The `raft_plan` addition is ADDITIVE. Existing consumers of `SupportPlanIR` continue to compile; the deserialized `raft_plan` is an empty `Vec` when reading from older serialized blobs (default via `#[serde(default)]` if serde is used — confirm).
- The `ExtrusionRole::RaftInfill` enum variant is additive but extends an exhaustive `match` surface in every `should_emit` consumer; the implementer's Step 3 dispatch lists all `match role` sites to update. Forgetting one results in either a compile failure (non-exhaustive match warning escalated to error) or a silent "always emit" path (the `_ => true` fallback).
- The `claim:raft-fill` string is a NEW claim. Existing `should_emit` consumers that don't hold it return `false` (per the existing `held_claims.iter().any` semantics) — this is correct per AC-N2.
- The schema_version bump is minor (additive). The implementer MUST NOT bump major.

## Code Change Surface

- Selected approach: targeted IR + SDK + planner edits. No new module is introduced.
- Exact functions/structs/manifests/tests to change:
  - `slicer_ir::ExtrusionRole` — new variant.
  - `slicer_ir::SupportPlanIR` — new field + schema_version bump.
  - `slicer_ir::{RaftPlan, RaftLayerSpec}` — new structs.
  - `slicer_sdk::views::should_emit` — new arm.
  - `support_planner::plan_for_object` — placeholder raft block deletion + `RaftPlan` emission.
  - `traditional_support` lead `//!` block — one sentence added.
  - Three new test files.
  - `docs/02_ir_schemas.md` — extended.
- Rejected alternatives:
  - **Replace `SupportPlanIR.entries` with a `tagged enum { Branch(SupportPlanEntry), Raft(RaftPlanEntry) }`** — rejected: breaking change to every consumer. Additive sibling field is the ADR-0009 choice.
  - **Make `raft_plan` per-region (`HashMap<RegionId, RaftPlan>`)** — rejected: raft is per-object per ADR-0009 D5; per-region keying re-introduces the duplication problem the ADR resolved.
  - **Add raft renderer code in this packet** — rejected: explicit out of scope; ADR-0009 splits at the rendering boundary.
  - **Make `support_raft_layers > 0` configurable per-region** — rejected: raft is a per-object decision in Orca + ADR-0009.

## Files in Scope (read + edit)

The packet edits 5 source files + 3 new test files + 1 doc file (9 total).

- `crates/slicer-ir/src/slice_ir.rs` — role: IR additions; expected change: ≈30 lines added.
- `crates/slicer-sdk/src/views.rs` — role: claim arm; expected change: 1 line.
- `modules/core-modules/support-planner/src/lib.rs` — role: emission rewrite; expected change: lines 381-423 replaced (≈50 lines net).
- `modules/core-modules/traditional-support/src/lib.rs` — role: doc sentence; expected change: 1 sentence added.
- `crates/slicer-ir/tests/support_plan_ir_schema_version_bumped.rs` — new.
- `crates/slicer-sdk/tests/should_emit_raft_fill_claim.rs` — new (or extend existing test for `should_emit`).
- `crates/slicer-runtime/tests/integration/raft_plan_emission_tdd.rs` — new.
- `docs/02_ir_schemas.md` — extended.
- `modules/core-modules/traditional-support/traditional-support.toml` — AC-9 verification only; no edit expected (manifest should already be clean).

## Read-Only Context

- `docs/specs/support-modules-orca-port.md` §C6, §C7, §D5, §D6 — directly.
- `docs/adr/0009-raft-as-layer-infill-role.md` — directly.
- `docs/specs/raft-default-module.md` — directly (the consumer of this packet's IR seam).
- `crates/slicer-ir/src/slice_ir.rs` — range-read existing `SupportPlanIR`, `ExtrusionRole`, `Point3WithWidth`.
- `crates/slicer-sdk/src/views.rs` — range-read `should_emit` and surrounding match arms.

## Out-of-Bounds Files

- `OrcaSlicerDocumented/**` — delegate.
- Other consumer crates beyond the listed surface — do not browse for "consistency."
- `target/`, `Cargo.lock`, generated code.
- Post-74710fa fill-partition test files — out of scope.

## Expected Sub-Agent Dispatches

- "Summarize OrcaSlicer `SupportCommon.cpp::generate_raft_base` for raft footprint computation, expansion factor, gap-layer Z; return SUMMARY ≤ 200 words. No code snippets."
- "Locate `should_emit` function (or post-74710fa equivalent dispatcher) in `crates/slicer-sdk/src/views.rs`; return SNIPPETS ≤ 30 lines showing the `match role` arms." — purpose: confirm Step 3 edit site.
- "Locate all `match role` sites in the workspace that switch on `ExtrusionRole`; return LOCATIONS ≤ 20 entries." — purpose: confirm exhaustive update.
- "Locate current `SupportPlanIR.schema_version` value in `crates/slicer-ir/src/slice_ir.rs`; return FACT (the `SemVer` literal)." — purpose: Step 2 bump arithmetic.
- "Locate `crates/slicer-ir/src/slice_ir.rs` lines defining `SupportPlanEntry`, `SupportPlanIR`, `Point3WithWidth`, `ExtrusionRole`; return LOCATIONS file:line." — purpose: edit targets.
- "Run `cargo build --workspace`; return FACT pass/fail; SNIPPETS ≤ 30 lines FIRST error." — purpose: post-IR-change compile gate.
- "Run `cargo xtask build-guests --check`; return FACT clean / STALE." — purpose: WASM gate.
- "Run AC-1 through AC-10 + AC-N1 + AC-N2 commands; return FACT PASS/FAIL list." — purpose: packet gate.

## Data and Contract Notes

- IR contracts touched: `SupportPlanIR` (additive); `ExtrusionRole` (new variant); schema_version minor bump.
- WIT boundary considerations: if `ExtrusionRole` crosses WIT (it does — see `crates/slicer-schema/wit/deps/ir-types.wit`), the new `RaftInfill` variant must be added to the WIT enum as well. The implementer's Step 3 dispatch lists WIT type sites.
- Determinism: raft plan emission is deterministic given the same inputs (footprint geometry, config).
- The `RaftPlan.footprint` is computed from `SupportGeometryView.outlines` (the same data the avoidance cache reads). Step 4 confirms via dispatch whether the data lives at a single canonical path or multiple.

## Locked Assumptions and Invariants

- `support-planner` is the sole writer of `SupportPlanIR` (single-writer-per-IR rule). This packet does NOT change that.
- `raft_plan` is per-object, keyed by `object_id`. Per-region duplication is forbidden.
- `RaftPlan.layers[*]` ordering is top-of-stack to bottom (highest `z` first). Per-layer Z values are populated using the formula from `docs/specs/support-modules-orca-port.md` §C6:
  ```
  z_bed = layer_plan.layers[0].z - layer_plan.layers[0].effective_layer_height
  raft_layer_i_z = z_bed - (raft_layers - i) * raft_layer_height_mm
  ```
- `RaftPlan` is only emitted for objects whose `entries` is non-empty (an object that gets no support branches gets no raft per ADR-0009 — adhesion-raft for objects without supports is future work).

## Risks and Tradeoffs

- **Risk**: post-74710fa, the `should_emit` dispatcher may have moved or been generalized. **Mitigation**: Step 1 dispatch confirms location before editing; if the structure changed substantially, the packet's Step 3 adapts and surfaces the deviation in a packet-author note.
- **Risk**: the `ExtrusionRole` WIT type addition triggers guest rebuild of all 20 guests (not just support modules). **Mitigation**: this is one-time cost; `cargo xtask build-guests` handles it.
- **Risk**: removing the placeholder raft block breaks any existing tests that asserted on the placeholder (`tests/orca_parity_tdd.rs` or similar). **Mitigation**: Step 4 dispatches a search for those tests and either migrates them to the new emission shape or notes them as expected breakage.
- **Tradeoff**: the WIT enum addition requires guest rebuild ceremony. Acceptable: enum additions are the cheapest WIT changes.

## Context Cost Estimate

- Aggregate: `M`
- Largest step: `M` (Step 4 — placeholder block removal + new emission).
- Highest-risk dispatch: "Locate all `match role` sites in the workspace" — return MUST be LOCATIONS ≤ 20 entries; never paste source.

## Open Questions

- `[FWD]` Post-74710fa `should_emit` location: confirmed via Step 1 dispatch before Step 3.
- `[FWD]` Whether the WIT side requires updating `crates/slicer-schema/wit/deps/ir-types.wit` (for the WIT-mirrored `ExtrusionRole`) — Step 3 confirms via dispatch.
