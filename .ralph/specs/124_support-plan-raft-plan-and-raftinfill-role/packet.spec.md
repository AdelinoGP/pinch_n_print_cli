---
status: implemented
packet: 124
task_ids:
  - TASK-289
backlog_source: docs/07_implementation_status.md
context_cost_estimate: M
---

# Packet Contract: support-plan-raft-plan-and-raftinfill-role

## Goal

Land the `RaftInfill` role/claim extension that ADR-0009 commits: add `ExtrusionRole::RaftInfill` to the Rust enum and its WIT mirror, add the `claim:raft-fill` arm to `should_emit`, bump `CURRENT_SUPPORT_PLAN_IR_SCHEMA_VERSION` minor to reflect the additive `ExtrusionRole` variant, and audit every workspace `match role` site to guard against the silent-true-fallback at `views.rs:504`. The packet does NOT introduce geometry — `SupportPlanIR.raft_plan` is the config-only record §C6 mandates and the renderer is `raft-default-module` (separate spec).

## Scope Boundaries

Touches the role/claim dispatch only: `crates/slicer-ir/src/slice_ir.rs` (add `ExtrusionRole::RaftInfill` variant near the existing `RaftInfill` neighbors at line 1659; bump the `CURRENT_SUPPORT_PLAN_IR_SCHEMA_VERSION` literal), `crates/slicer-sdk/src/views.rs::should_emit` (one new role/claim arm at line 498), `crates/slicer-schema/wit/deps/types.wit` (one new `raft-infill` line in the `variant extrusion-role` block at lines 13-18), and the WIT rebuild ceremony. The packet does NOT modify `SupportPlanIR`, `RaftPlan`, or `support-planner` — those landed in packet 119 per §C6. The packet does NOT touch `traditional-support` — its lead `//!` block already documents the C7 non-consumption (lines 22-27) and the manifest at `traditional-support.toml` already excludes `SupportPlanIR` from `reads`. Both are verified, not edited.

## Prerequisites and Blockers

- Depends on: packet 119 (`SupportPlanIR.raft_plan: Option<RaftPlan>` config-only record per §C6; `CURRENT_SUPPORT_PLAN_IR_SCHEMA_VERSION = 1.2.0`); packet 116 (`traditional-support` lead `//!` block established); ADR-0009 (the role/claim decision).
- Unblocks: `raft-default-module` spec (which reads `SupportPlanIR.raft_plan` and synthesizes raft polygons on `SliceRegionView`); any future infill module that declares `claim:raft-fill`.
- Activation blockers: none. §C6 contract is canonical, ADR-0009 is the design authority, the role/claim infrastructure is live and stable. The packet ships a single additive variant + a schema minor bump + a workspace-wide `match role` audit.

## Acceptance Criteria

- **AC-1. Given** `crates/slicer-ir/src/slice_ir.rs`, **when** parsed, **then** the `pub enum ExtrusionRole` (line 1659) contains a `RaftInfill` variant. | `rg -A40 'pub enum ExtrusionRole' crates/slicer-ir/src/slice_ir.rs | rg -q '\bRaftInfill\b'`
- **AC-2. Given** `crates/slicer-sdk/src/views.rs::should_emit` (line 497), **when** searched, **then** the `match role` block contains the arm `ExtrusionRole::RaftInfill => "claim:raft-fill"`. The arm follows the same shape as the existing `TopSolidInfill => "claim:top-fill"` arm at line 499. | `rg -q 'ExtrusionRole::RaftInfill => "claim:raft-fill"' crates/slicer-sdk/src/views.rs`
- **AC-3. Given** `crates/slicer-schema/wit/deps/types.wit` (the WIT mirror of `ExtrusionRole` in the `geometry` interface, lines 13-18), **when** searched, **then** the `variant extrusion-role { ... }` includes `raft-infill` (snake_case per WIT convention). | `rg -q 'raft-infill' crates/slicer-schema/wit/deps/types.wit`
- **AC-4. Given** `slicer-sdk`'s `should_emit_raft_fill_claim_tdd` test, **when** a `ModuleDispatchView` is constructed with `held_claims = ["claim:raft-fill"]` and queried with `should_emit(ExtrusionRole::RaftInfill)`, **then** the return is `true`. The test lives at `crates/slicer-sdk/tests/should_emit_raft_fill_claim_tdd.rs` and is wired into the sdk's test binary. | `cargo test -p slicer-sdk --test should_emit_raft_fill_claim_tdd --nocapture 2>&1 | tee target/test-output.log`
- **AC-5. Given** `CURRENT_SUPPORT_PLAN_IR_SCHEMA_VERSION` at `crates/slicer-ir/src/slice_ir.rs`, **when** parsed, **then** the literal value is `SemVer { major: 1, minor: 3, patch: 0 }` (bumped from `1.2.0` to `1.3.0` per ADR-0009 §Consequences: "Schema bump on `ExtrusionRole` (semver minor — additive)"). | `rg -B1 -A2 'CURRENT_SUPPORT_PLAN_IR_SCHEMA_VERSION' crates/slicer-ir/src/slice_ir.rs | rg -q 'minor: 3'`
- **AC-6. Given** every workspace `match role` site identified in the audit (14 entries per the discovery dispatch — see Authoritative Docs), **when** parsed, **then** every site that has an explicit per-variant arm list (i.e., does NOT use `_ =>`) includes a `ExtrusionRole::RaftInfill =>` arm. Sites that already use a `_ =>` wildcard fallback are exempt (the new variant falls into the wildcard). | `rg -l 'match .*ExtrusionRole' crates/ modules/ --type rust | xargs -I{} sh -c 'rg -q "ExtrusionRole::RaftInfill" "{}" || rg -q "_ =>" "{}"'`
- **AC-7. Given** the `ExtrusionRole` WIT enum was extended with `raft-infill`, **when** the WIT-to-Rust round-trip is exercised via the existing `crates/slicer-wasm-host/tests/contract/wit_boundary_tdd.rs`, **then** the round-trip succeeds. | `cargo test -p slicer-wasm-host --test wit_boundary_tdd 2>&1 | tee target/test-output.log`
- **AC-8. Given** `modules/core-modules/traditional-support/src/lib.rs` lead `//!` block, **when** searched, **then** the C7 non-consumption statement ("does **not** declare `SupportPlanIR` as a read ... does **not** consume `PrePass::SupportGeometry` output") is present at lines 22-27. This is a verification of the existing state, not a new edit. | `rg -q 'does \*\*not\*\* declare .SupportPlanIR. as a read' modules/core-modules/traditional-support/src/lib.rs`
- **AC-9. Given** `modules/core-modules/traditional-support/traditional-support.toml` `[ir-access].reads`, **when** searched, **then** the list contains `SliceIR` and `SurfaceClassificationIR` and does NOT contain `SupportPlanIR`. This is a verification of the existing state, not a new edit. | `rg -A2 '\[ir-access\]' modules/core-modules/traditional-support/traditional-support.toml | rg -q 'SliceIR' && rg -A2 '\[ir-access\]' modules/core-modules/traditional-support/traditional-support.toml | rg -q 'SurfaceClassificationIR' && ! rg -A2 '\[ir-access\]' modules/core-modules/traditional-support/traditional-support.toml | rg -q 'SupportPlanIR'`

## Negative Test Cases

- **AC-N1. Given** a `ModuleDispatchView` constructed with `held_claims = ["claim:sparse-fill"]` (does NOT hold the new `claim:raft-fill`), **when** queried with `should_emit(ExtrusionRole::RaftInfill)`, **then** the return is `false`. The new arm mirrors the existing 4-claim pattern at `views.rs:498-503`; modules without the claim don't emit. | `cargo test -p slicer-sdk --test should_emit_raft_fill_claim_tdd -- nocapture 2>&1 | tee target/test-output.log`
- **AC-N2. Given** every workspace `match role` site that uses an explicit per-variant arm list (no `_ =>` wildcard), **when** the new `RaftInfill` variant is added to the enum, **then** `cargo build --workspace` succeeds. A missing arm at any non-wildcard site escalates to a non-exhaustive-match error. | `cargo build --workspace 2>&1 | tee target/test-output.log`
- **AC-N3. Given** the silent-true-fallback risk at `views.rs:504` (`_ => return true`), **when** `should_emit` is called with `ExtrusionRole::RaftInfill` on a `ModuleDispatchView` with `held_claims = []` (the empty-claims suppression branch at lines 507-509), **then** the return is `false` (the empty-claims branch fires before the role lookup, matching existing `TopSolidInfill` behavior). | `cargo test -p slicer-sdk --test should_emit_raft_fill_claim_tdd -- nocapture 2>&1 | tee target/test-output.log`

## Verification

- `cargo xtask build-guests --check`
- `cargo build --workspace --all-targets`
- `cargo test -p slicer-sdk --test should_emit_raft_fill_claim_tdd 2>&1 | tee target/test-output.log`
- `cargo test -p slicer-wasm-host --test wit_boundary_tdd 2>&1 | tee target/test-output.log`
- `cargo clippy --workspace --all-targets -- -D warnings`

## Authoritative Docs

- `docs/adr/0009-raft-as-layer-infill-role.md` — directly (94 lines; full read OK).
- `docs/specs/support-modules-orca-port.md` §C6 (line 380-410) — directly. Defines the `RaftPlan` config-only record and explicitly defers geometry to packet 124 + the sibling `raft-default-module.md`.
- `docs/specs/raft-default-module.md` — read for consumer alignment only. The synthesizer that consumes `SupportPlanIR.raft_plan` and populates raft polygon carriers. Not edited in this packet.
- `crates/slicer-ir/src/slice_ir.rs` — range-read `ExtrusionRole` (lines 1655-1700) and `CURRENT_SUPPORT_PLAN_IR_SCHEMA_VERSION` (lines 245-260).
- `crates/slicer-sdk/src/views.rs` — range-read `should_emit` (lines 480-520).
- `crates/slicer-schema/wit/deps/types.wit` — full read (small).
- `modules/core-modules/traditional-support/src/lib.rs` — read first 30 lines (lead `//!` block).
- `modules/core-modules/traditional-support/traditional-support.toml` — full read.
- The 14 `match role` sites discovered by the audit dispatch — see `requirements.md` §Authoritative Docs for the file:line list.

## Doc Impact Statement (Required)

- `none` — no IR, WIT (interface), scheduler, claim, manifest, host-service, or SDK contract is changed. The WIT `extrusion-role` enum gains one additive variant; the Rust `ExtrusionRole` enum gains one additive variant; the `should_emit` mapping gains one arm; the schema constant bumps minor. None of these changes require `docs/02_ir_schemas.md` or `docs/01_system_architecture.md` updates — the docs already document the role/claim extension pattern. `docs/specs/support-modules-orca-port.md` §C6 already names the packet. `docs/adr/0009-raft-as-layer-infill-role.md` is the design authority and is already current.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/Support/SupportCommon.cpp::generate_raft_base` — out of scope for this packet. The packet does not render raft; it only adds the role/claim arm. Future `raft-default-module` work will consult this for the synthesizer's polygon expansion factors.

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- obey the shared absolute context bands: 120k reading budget with hand-off at 150k (standard); the extended band (240k reading / 300k hard stop) only via swarm's escalation protocol

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation (an extended-band run may carry a single L step only when `design.md` justifies why it cannot be split).
