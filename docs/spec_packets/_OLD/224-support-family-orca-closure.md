---
status: superseded
packet: 224-support-family-orca-closure
superseded_by: 242-support-family-orca-closure
task_ids:
  - TASK-335
---

# 224-support-family-orca-closure

## Goal
Close the support-family sequence with fixture-driven invariants and inspected visual/differential evidence showing tree and traditional support reach valid termination surfaces without model collision and retain family roles through final G-code.

## Problem Statement
The support defect cannot close on typed captures or self-captured goldens. The decisive model and Orca reference files are present in this checkout, so closure must regenerate and inspect evidence from those fixtures.

**Closure basis (locked 2026-08-18).** This packet closes on **correctness plus honest tests**, not on canonical feature completeness. Every remaining canonical feature gap routes to a named follow-on packet through `docs/specs/support-parity-gap-register.md`. **Destinations corrected 2026-09-03: `docs/spec_packets/stubs/` never existed and no gap was ever routed there.** The real owning packets are `241-support-agg-rasterizer` (AGG rasterizer / `support_area_algorithm`), `239a-anchored-host-seams` / `239b-anchored-wit-contract` / `239c-support-layer-height-producer` / `239d-support-coarse-floating-planes` (independent support-layer Z), `238a-support-pattern-config-keys` (base/interface patterns, `support_expansion`, `support_bottom_z_distance`, dead config keys), `240a-support-raft-substrate` + `240b-support-raft-module` (raft), and `238c-support-renderer-flow-interfaces` (renderer/flow/interfaces, incl. `needs_support` eligibility follow-through). Always re-derive a gap's owner from the register's destination column, never from this sentence. A gap that is registered and routed is not a 224 blocker; an incorrect behaviour or a test that asserts nothing is.

## Architecture Constraints
- Closure proves behavioral parity only: coverage, termination, collision freedom, interfaces, independent heights, and printable construction; exact path identity is out of scope.
- `Layer::Support`, `PrePass::SupportAnalysis`, and `PrePass::SupportGeometry` are separate evidence boundaries and must all be captured. Both support stages exist: `PrePass::SupportAnalysis` (host analysis stage carrying candidates, occupancy/termination surfaces, baseline envelope, and deterministic family assignments) and `PrePass::SupportGeometry` (legacy geometry stage, still in STAGE_ORDER).
- The existing decisive fixtures are the primary closure path. A deliberately missing copied path is reserved for the negative gate.
- Final evidence must not treat PNG existence, byte size, manifest greps, or self-captured goldens as proof.
<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.
<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and, if `STALE:` is reported, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.

## Data and Contract Notes
- IR/manifest contracts: assert typed captures at `PrePass::SupportAnalysis`, `PrePass::SupportGeometry`, and `Layer::Support`; preserve structured family/body/demand roles from TASK-334.
- WIT boundary: no new WIT contract; inherited TASK-331 migration blocker is resolved. Packet 220 performed a breaking in-place replacement of the `support-plan-entry` record within `slicer:prepass-support-geometry@1.0.0` (package stays 1.0.0). Schema versions: `CURRENT_SUPPORT_PLAN_IR_SCHEMA_VERSION` 1.3.0→2.0.0, `CURRENT_SUPPORT_IR_SCHEMA_VERSION` 1.0.0→2.0.0, `CURRENT_SUPPORT_ANALYSIS_IR_SCHEMA_VERSION` 1.0.0.
- WASM boundary: packet 220's live WASM dispatch hands guests an EMPTY structural plan (the paint-view boundary does not carry plan entries); plan-consuming tests drive the renderer natively. For closure evidence, visual-debug taps and G-code role parsing must capture the host-side aggregated plan/`SupportIR` (which carry the identity), not guest-side plan reads.
- Determinism/scheduler constraints: compare forced serial/parallel fixture results and preserve anchored event order.

## Locked Assumptions and Invariants
- Exact-Z body and rendered sweep collision checks are authoritative over skeleton-only checks.
- Missing Orca references cannot be silently replaced by PNP output.

## Risks and Tradeoffs
- `TASK-163b-orca-ref` may remain externally blocked only if provenance/authority cannot be established; the existing references must still be used for primary differential review.
- Visual evidence is human-inspected and therefore cannot be reduced to a grep-only AC.
