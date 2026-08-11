---
status: draft
packet: 215-raft-geometry
task_ids:
  - TASK-324
backlog_source: docs/07_implementation_status.md
context_cost_estimate: M
---

# Packet Contract: 215-raft-geometry

## Goal

Add the ADR-0009 `com.core.raft-default` Layer::Infill synthesizer, connect its `claim:raft-fill` carrier to deterministic rectilinear rendering, and carry signed raft-prefix indices through the IR, schedule, SDK, macro, host, runtime, and visual-debug paths.

## Scope Boundaries

This packet owns raft-plan-to-footprint synthesis, Layer::Infill claim ownership, negative-prefix scheduling, the signed scheduled-layer IR migration, and the Layer::Infill `s32` contract migration. It owns every affected struct literal, conversion, test assertion, fixture, and typed-capture verification site. It does not add a Layer::Support raft renderer, change support planning, or alter final G-code role ordering.

## Prerequisites and Blockers

- Depends on: `docs/spec_packets/213-support-planner-defect-fix/` (`TASK-322`) producing `SupportPlanIR.raft_plan`.
- Unblocks: `support-gcode-e2e` (`TASK-327`).
- Activation blockers: none; packet remains `draft` until implementation and preflight closure.

## Acceptance Criteria

- **AC-1. Given** the new module manifest is inspected, **when** its module contract is parsed, **then** it declares id `com.core.raft-default`, stage `Layer::Infill`, holds `claim:raft-fill`, reads `SupportPlanIR`, `SliceIR`, and `LayerPlanIR`, and writes the `SliceIR` raft carrier `SlicedRegion.raft_fill`. | `test -f modules/core-modules/raft-default/raft-default.toml && rg -q 'id\s*=\s*"com.core.raft-default"' modules/core-modules/raft-default/raft-default.toml && rg -q 'Layer::Infill' modules/core-modules/raft-default/raft-default.toml && rg -q 'claim:raft-fill' modules/core-modules/raft-default/raft-default.toml && rg -q 'SupportPlanIR' modules/core-modules/raft-default/raft-default.toml && rg -q 'SliceIR' modules/core-modules/raft-default/raft-default.toml && rg -q 'LayerPlanIR' modules/core-modules/raft-default/raft-default.toml && rg -q 'raft_fill' modules/core-modules/raft-default/raft-default.toml`
- **AC-2. Given** a `RaftPlan` with `raft_layers = 4`, `base_raft_layers = 2`, `interface_raft_layers = 1`, and `raft_first_layer_density = 0.4`, **when** the synthesizer runs on a non-empty footprint, **then** the carrier has four non-empty raft layers keyed by signed `global_layer_index` values `-1`, `-2`, `-3`, and `-4`, with deterministic prefix ordering and no unsigned cast. | `cargo test -p raft-default --test raft_geometry_tdd --all-targets -- raft_plan_emits_negative_prefix_layers`
- **AC-3. Given** `SliceIR.regions[].polygons` contains the object footprint, **when** raft polygons are synthesized, **then** every `SlicedRegion.raft_fill` polygon is within the documented expanded footprint and no carrier polygon is outside it. | `cargo test -p raft-default --test raft_geometry_tdd --all-targets -- raft_carrier_is_clipped_to_expanded_footprint`
- **AC-4. Given** `tmp/visual-debug-raft.json` requests `[-1, -2, -3, -4, 10]`, taps `Layer::Infill` and `Layer::Support`, and enables raft on `tmp/SupportTest.stl`, **when** visual-debug runs, **then** the output contains a non-empty typed `raft_paths` capture. If the renderer still rejects negative selectors, the command must print `PNG layer selection unsupported` and use a second positive-selector request only to establish the non-empty typed capture gate; that unsupported PNG condition is not a pass. | `set +e; cargo run -q -p pnp-cli --bin pnp_cli -- visual-debug --request tmp/visual-debug-raft.json --output target/vd-raft --overwrite >/tmp/vd-raft.log 2>&1; rc=$?; set -e; if test "$rc" -eq 0; then test -s target/vd-raft/manifest.json && jq -e '.. | objects | select(has("raft_paths")) | .raft_paths | length > 0' target/vd-raft/manifest.json >/dev/null; else printf '%s\n' 'PNG layer selection unsupported'; cargo run -q -p pnp-cli --bin pnp_cli -- visual-debug --request tmp/visual-debug-raft-typed.json --output target/vd-raft-typed --overwrite >/dev/null && jq -e '.. | objects | select(has("raft_paths")) | .raft_paths | length > 0' target/vd-raft-typed/manifest.json >/dev/null; fi`
- **AC-5. Given** identical `RaftPlan`, `SliceIR`, and `LayerPlanIR` inputs, **when** the synthesizer and claim holder run twice, **then** carrier polygon ordering and rendered `SupportIR.raft_paths` ordering, points, Z values, roles, widths, and counts are identical. | `cargo test -p raft-default --test raft_geometry_tdd --all-targets -- raft_output_is_deterministic`
- **AC-6. Given** signed raft layers are scheduled, **when** IR, SDK, macro, host, runtime, and visual-debug boundary tests compile, **then** `GlobalLayer.index`, `ObjectLayerRef.local_layer_index`, `ObjectLayerRef.global_layer_index`, `SliceIR.global_layer_index`, and `SupportIR.global_layer_index` are `i32`; `LayerModule::run_infill` and its SDK guest implementations use `i32`; WIT `layer-idx` remains `s32`; negative selectors are not cast to `u32`; and all affected literals and hard assertions compile. | `rg -q 'pub index: i32' crates/slicer-ir/src/slice_ir.rs && rg -q 'pub local_layer_index: i32' crates/slicer-ir/src/slice_ir.rs && rg -q 'pub global_layer_index: i32' crates/slicer-ir/src/slice_ir.rs && rg -q 'fn run_infill[[:space:]]*(' crates/slicer-sdk/src/traits.rs && rg -q '_layer_index: i32|layer_index: i32' crates/slicer-sdk/src/traits.rs crates/slicer-wasm-host/test-guests/sd*/src/lib.rs && rg -q 'type layer-idx = s32' crates/slicer-schema/wit/deps/ir-types.wit && cargo test -p slicer-ir --all-targets`

## Negative Test Cases

- **AC-N1. Given** `raft_plan = None`, `raft_layers = 0`, or an empty footprint, **when** the Layer::Infill synthesizer is dispatched, **then** it succeeds, writes no `SlicedRegion.raft_fill`, and produces zero `SupportIR.raft_paths`; a module holding only `claim:sparse-fill` does not render `ExtrusionRole::RaftInfill`. | `cargo test -p raft-default --test raft_geometry_tdd --all-targets -- rejects_missing_or_zero_raft_inputs && cargo test -p slicer-sdk --test should_emit_raft_fill_claim_tdd --all-targets`

## Verification

- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo xtask build-guests --check`

## Authoritative Docs

- `docs/adr/0009-raft-as-layer-infill-role.md` - direct read; controlling stage, synthesizer, carrier, and claim decision.
- `docs/specs/support-generation-remediation-plan.md` - direct read; TASK-324 queue and approved raft scope.
- `docs/specs/support-generation-defect-verified-findings.md` - direct bounded read; verified IR/output evidence.
- `docs/01_system_architecture.md`, `docs/02_ir_schemas.md`, `docs/03_wit_and_manifest.md`, `docs/08_coordinate_system.md` - delegated relevant sections.

## Doc Impact Statement (Required)

- Specific same-packet doc edits: `docs/01_system_architecture.md` raft stage/claim row - `rg -q 'raft-default|claim:raft-fill' docs/01_system_architecture.md`; `docs/02_ir_schemas.md` signed scheduled indices and raft carrier sections - `rg -q 'GlobalLayer|i32|raft_paths|raft_fill' docs/02_ir_schemas.md`; `docs/03_wit_and_manifest.md` module/claim and Layer::Infill `s32` catalog - `rg -q 'com.core.raft-default|claim:raft-fill|layer-idx' docs/03_wit_and_manifest.md`.

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- obey the shared absolute context bands: 120k reading budget with hand-off at 150k (standard); the extended band (240k reading / 300k hard stop) only via swarm's escalation protocol

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation (an extended-band run may carry a single L step only when `design.md` justifies why it cannot be split).
