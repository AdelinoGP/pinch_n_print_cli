# Requirements: support-plan-raft-plan-and-raftinfill-role

## Packet Metadata

- Grouped task IDs: `TASK-289`
- Backlog source: `docs/07_implementation_status.md`
- Packet status: `active`
- Aggregate context cost: `M` (per step roll-up in `implementation-plan.md`; no step is L)

## Problem Statement

ADR-0009 commits raft rendering to the existing `Layer::Infill` role/claim dispatch: add `ExtrusionRole::RaftInfill` + `claim:raft-fill` so any infill module can declare the claim and render raft patterns without duplicating scan-line math. The packet is the IR-side half of that decision — the role variant, the WIT mirror, the claim arm, and the schema bump — and it is the gating prerequisite for `raft-default-module` (the synthesizer that reads `SupportPlanIR.raft_plan` and populates raft polygon carriers).

§C6 of `docs/specs/support-modules-orca-port.md` is the IR-side authority: `SupportPlanIR.raft_plan: Option<RaftPlan>` is a **config-only record** (`raft_layers`, `raft_first_layer_density`, `base_raft_layers`, `interface_raft_layers`) that `support-planner` already emits via the WIT `push-raft-plan` seam at `world-prepass.wit:163`. `RaftPlan` carries **no footprint, no layer specification, no Z gap, no raft polygon** — those are the synthesizer's responsibility. The packet must not corrupt this contract.

The packet also verifies (not edits) the C7 non-consumption decision for `traditional-support`: its lead `//!` block at `modules/core-modules/traditional-support/src/lib.rs:22-27` already states that the module does not declare `SupportPlanIR` as a read and does not consume `PrePass::SupportGeometry` output, and the manifest at `traditional-support.toml` confirms `reads = ["SliceIR", "SurfaceClassificationIR"]` with no `SupportPlanIR`.

## In Scope

- Add `ExtrusionRole::RaftInfill` variant to the Rust enum at `crates/slicer-ir/src/slice_ir.rs:1659`. The variant is additive (`#[non_exhaustive]` is already on the enum at line 1657). Doc-comment it as "Raft infill (ADR-0009: rendered by whichever `Layer::Infill` module declares `claim:raft-fill`)."
- Add `raft-infill` to the WIT mirror at `crates/slicer-schema/wit/deps/types.wit:13-18` in the `variant extrusion-role` block.
- Add `ExtrusionRole::RaftInfill => "claim:raft-fill"` arm in `should_emit` at `crates/slicer-sdk/src/views.rs:498`. Place the arm immediately after the existing `TopSolidInfill` arm (line 499) to match the role/claim grouping pattern.
- Bump `CURRENT_SUPPORT_PLAN_IR_SCHEMA_VERSION` from `1.2.0` to `1.3.0` at `crates/slicer-ir/src/slice_ir.rs:251-255`. The bump is semver-minor (additive) per ADR-0009 §Consequences.
- Audit every workspace `match role` site (14 entries) and add an explicit `RaftInfill =>` arm at every site that uses a per-variant arm list without a `_ =>` wildcard. Sites that already use `_ =>` are exempt — the new variant falls into the wildcard. Discovery confirmed at least one such site (`overhang-classifier-default/src/lib.rs:48`) that lists every variant explicitly.
- Author `crates/slicer-sdk/tests/should_emit_raft_fill_claim_tdd.rs` covering AC-4, AC-N1, AC-N3.
- Run `cargo xtask build-guests` after the WIT enum addition (the WIT change triggers a 20-guest rebuild — the WIT file is read by every guest's bindgen).
- Verify (do not edit) the C7 state at `modules/core-modules/traditional-support/src/lib.rs:22-27` and `modules/core-modules/traditional-support/traditional-support.toml` per AC-8 and AC-9.

## Out of Scope

- Modifying `SupportPlanIR`, `RaftPlan`, or `support-planner/src/lib.rs`. The `RaftPlan` config-only record at `slice_ir.rs:1132-1142` and the planner's existing `push_raft_plan` call at `support-planner/src/lib.rs:257-266` are the canonical §C6 implementation; this packet does not touch them.
- Raft geometry (footprint, layers, Z gap, first_layer_density in polygons). Deferred to `raft-default-module.md`.
- Bumping the WIT `push-raft-plan` interface signature. The interface at `world-prepass.wit:163` already accepts the config-only `RaftPlan`; the new role/claim arm works against it.
- The `tree-support` module. `tree-support` reads `SupportPlanEntry.branch_segments` only and is unaffected by `RaftInfill`.
- Editing `traditional-support` lead `//!` block or manifest. The C7 state is already correct; the packet verifies it.
- `docs/02_ir_schemas.md` and `docs/01_system_architecture.md`. The role/claim pattern is already documented; the additive variant + arm do not require a doc update.
- Editing the `support-planner` `to_buildplate` pruning, contact-to-buildplate logic, or any other planner internals. Step 4 of the original draft packet (degenerate-block removal) is dead work — the block never existed.

## Authoritative Docs

- `docs/adr/0009-raft-as-layer-infill-role.md` — 94 lines; direct full read. Design authority for the role/claim extension.
- `docs/specs/support-modules-orca-port.md` §C6 (lines 380-410) — direct range read. `RaftPlan` config-only contract; schema 1.2.0 baseline; `push-raft-plan` WIT seam.
- `docs/specs/support-modules-orca-port.md` §C7 (lines 412-418) — direct range read. The C7 decision: `traditional-support` does not consume `SupportPlanIR`; the doc-comment already records it.
- `docs/specs/raft-default-module.md` — read for consumer alignment only. Not edited.
- `crates/slicer-ir/src/slice_ir.rs` — range-read `ExtrusionRole` (lines 1655-1700), `CURRENT_SUPPORT_PLAN_IR_SCHEMA_VERSION` (lines 245-260), `SupportPlanIR.raft_plan` doc-comment (lines 1160-1165).
- `crates/slicer-sdk/src/views.rs` — range-read `should_emit` (lines 480-520).
- `crates/slicer-schema/wit/deps/types.wit` — full read (small).
- `crates/slicer-schema/wit/deps/world-prepass/world-prepass.wit:163` — `push-raft-plan` interface; full read of the `support-geometry-output` resource block.
- `modules/core-modules/traditional-support/src/lib.rs:1-30` — lead `//!` block.
- `modules/core-modules/traditional-support/traditional-support.toml` — full read.
- The 14 `match role` sites (from the discovery dispatch):
  - `crates/slicer-gcode/src/emit.rs:145` and `:226`
  - `crates/slicer-gcode/src/serialize.rs:27`
  - `crates/slicer-gcode/tests/gcode_feedrate_emission_tdd.rs:93`
  - `crates/slicer-runtime/src/visual_debug_render.rs:598`
  - `crates/slicer-macros/src/lib.rs:684` and `:706`
  - `crates/slicer-sdk/src/views.rs:498`
  - `crates/slicer-wasm-host/src/marshal/leaf.rs:188` and `:369`
  - `crates/slicer-runtime/tests/integration/infill_partitioned_input_tdd.rs:102`
  - `crates/slicer-runtime/tests/integration/overhang_classifier_refactor_regression_tdd.rs:67`
  - `modules/core-modules/overhang-classifier-default/src/lib.rs:48`
  - `modules/core-modules/path-optimization-default/src/lib.rs:152`

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/Support/SupportCommon.cpp::generate_raft_base` — out of scope for this packet. The packet does not render raft; it only adds the role/claim arm. The synthesizer (`raft-default-module`) will consult this for polygon expansion factors in a later packet.

## Acceptance Summary

Reference, never copy, criteria from `packet.spec.md`.

- Positive: AC-1 (variant), AC-2 (claim arm), AC-3 (WIT mirror), AC-4 (held-claims acceptance), AC-5 (schema 1.2.0 → 1.3.0), AC-6 (14-site `match role` audit), AC-7 (WIT round-trip), AC-8 (C7 doc verification), AC-9 (C7 manifest verification).
- Negative: AC-N1 (held-claims rejection), AC-N2 (workspace compile), AC-N3 (empty-claims suppression).
- Cross-packet impact: this packet completes the IR-side half of ADR-0009. The `raft-default-module` spec becomes implementable (synthesizer reads `SupportPlanIR.raft_plan` and writes raft polygons to `SliceRegionView`). The WIT enum addition triggers 20-guest rebuild. `tree-support` is unaffected.

## Verification Commands

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `rg -A40 'pub enum ExtrusionRole' crates/slicer-ir/src/slice_ir.rs \| rg -q '\bRaftInfill\b'` | AC-1 Rust enum variant. | FACT pass/fail |
| `rg -q 'ExtrusionRole::RaftInfill => "claim:raft-fill"' crates/slicer-sdk/src/views.rs` | AC-2 claim arm. | FACT pass/fail |
| `rg -q 'raft-infill' crates/slicer-schema/wit/deps/types.wit` | AC-3 WIT mirror. | FACT pass/fail |
| `cargo test -p slicer-sdk --test should_emit_raft_fill_claim_tdd --nocapture 2>&1 \| tee target/test-output.log` | AC-4, AC-N1, AC-N3 behavioral tests. | FACT pass/fail; SNIPPETS ≤ 30 lines on failure |
| `rg -B1 -A2 'CURRENT_SUPPORT_PLAN_IR_SCHEMA_VERSION' crates/slicer-ir/src/slice_ir.rs \| rg -q 'minor: 3'` | AC-5 schema bump. | FACT pass/fail |
| `rg -l 'match .*ExtrusionRole' crates/ modules/ --type rust \| xargs -I{} sh -c 'rg -q "ExtrusionRole::RaftInfill" "{}" \|\| rg -q "_ =>" "{}"'` | AC-6 exhaustive audit. | FACT pass/fail (exit 0 only if every site either has the arm or a wildcard) |
| `cargo test -p slicer-wasm-host --test wit_boundary_tdd 2>&1 \| tee target/test-output.log` | AC-7 WIT round-trip. | FACT pass/fail |
| `rg -q 'does \*\*not\*\* declare .SupportPlanIR. as a read' modules/core-modules/traditional-support/src/lib.rs` | AC-8 C7 doc verification. | FACT pass/fail |
| `rg -A2 '\[ir-access\]' modules/core-modules/traditional-support/traditional-support.toml \| rg -q 'SliceIR' && rg -A2 '\[ir-access\]' modules/core-modules/traditional-support/traditional-support.toml \| rg -q 'SurfaceClassificationIR' && ! rg -A2 '\[ir-access\]' modules/core-modules/traditional-support/traditional-support.toml \| rg -q 'SupportPlanIR'` | AC-9 C7 manifest verification. | FACT pass/fail |
| `cargo xtask build-guests --check` | Guest WASM freshness after WIT enum addition. | FACT clean / STALE |
| `cargo build --workspace --all-targets` | AC-N2 workspace compile. | FACT pass/fail; SNIPPETS ≤ 30 lines FIRST error |
| `cargo clippy --workspace --all-targets -- -D warnings` | Workspace lint. | FACT pass/fail; SNIPPETS ≤ 20 lines FIRST error |

## Step Completion Expectations

- Step 1 (discovery) must produce: (a) the exact line of the new `RaftInfill` insertion in the Rust enum (after the `RaftInfill` neighbors, with no orphan), (b) the line of the new `claim:raft-fill` arm in `should_emit`, (c) the 14-site `match role` audit list, (d) the 20-guest rebuild ceremony plan.
- Step 2 (IR + schema bump) is load-bearing first. Subsequent steps depend on the `RaftInfill` variant being durable in the enum. AC-1 and AC-5 must both PASS before Step 3.
- Step 3 (claim arm + WIT mirror + guest rebuild) is atomic with Step 4 (match role audit): the new variant must not be added to the enum until every non-wildcard `match role` site has been audited, otherwise a partial state lands a non-exhaustive-match error in `cargo build --workspace`.
- Step 4 (match role audit) is independent of Steps 2/3 only on the audit side; the actual arm additions depend on Step 2 having added the variant. The audit dispatch (Step 4 read-only pass) can run in parallel with Step 1, but the arm-addition edits must serialize after Step 2.
- Step 5 (sdk test) is the behavioral gate for AC-4/AC-N1/AC-N3. The test must be wired into the slicer-sdk test binary (the existing convention for `*_tdd.rs` integration-style tests).
- Step 6 (final verification) re-runs every pipe-suffixed AC command and confirms `cargo xtask build-guests --check` is clean.

## Context Discipline Notes

- The 14 `match role` sites are listed in §Authoritative Docs. Workers must range-read each site before editing — none of them should be loaded in full. The largest is `crates/slicer-wasm-host/src/marshal/leaf.rs` (multiple sites); the audit dispatch returns ≤ 30-line snippets per site, never full files.
- `crates/slicer-ir/src/slice_ir.rs` is over 2400 lines. Range-read the `ExtrusionRole` enum (lines 1655-1700) and `CURRENT_SUPPORT_PLAN_IR_SCHEMA_VERSION` literal (lines 245-260) only. Do not browse the file.
- `docs/specs/support-modules-orca-port.md` is large. Range-read §C6 (lines 380-410) and §C7 (lines 412-418) only. The §C6 contract is the authority for what the packet must NOT touch.
- `OrcaSlicerDocumented/` is out of bounds entirely. The packet does not need it.
- Likely temptation reads (skip these): `support-planner/src/lib.rs` (no edits; the planner is done), the future `raft-default-module` spec (do not pre-design), the `infill_partitioned_input_tdd.rs` integration test (out of scope; the new dispatch it exercises is separate from this packet's role/claim addition).
- Heavy-dispatch return limits: `cargo build --workspace --all-targets` returns FACT pass/fail only; SNIPPETS ≤ 30 lines FIRST error on failure. `cargo xtask build-guests --check` returns FACT clean / STALE; never paste the rebuild log.
