---
status: draft
packet: 216-support-interface-layers
task_ids:
  - TASK-325
backlog_source: docs/07_implementation_status.md
context_cost_estimate: M
---

# Packet Contract: 216-support-interface-layers

## Goal

Move top and bottom support-interface decisions into `SupportPlanIR.interface_plan` using canonical `SupportInterfacePlanEntry` records, then make both support generator modules emit the planned dense paths without diagnostic code `1003`.

## Scope Boundaries

This packet owns the planner interface records, the IR/WIT/SDK/macro/host transport, schema-version fallout, the `traditional-support` manifest read, and both `Layer::Support` consumers. It does not change branch propagation, fallback clipping, raft geometry, support-type variants, or final G-code emission.

## Prerequisites and Blockers

- Depends on: `docs/spec_packets/213-support-planner-defect-fix/` (`TASK-322`), including its printable branch-radius floor.
- Unblocks: `support-gcode-e2e` (`TASK-327`).
- Activation blockers: none.

## Acceptance Criteria

- **AC-1. Given** the additive contract is implemented, **when** the IR, SDK, WIT, macro, host marshal, and host collection surfaces are inspected, **then** the one canonical record name is `SupportInterfacePlanEntry`, `SupportPlanIR` has the additive `interface_plan` field, the WIT output has a typed interface-plan push, and `CURRENT_SUPPORT_PLAN_IR_SCHEMA_VERSION` has a minor version exactly one greater than the live pre-activation constant currently defined as `SemVer { major: 1, minor: 3, patch: 0 }`. | `rg -q 'SupportInterfacePlanEntry' crates/slicer-ir/src/slice_ir.rs crates/slicer-sdk/src/prepass_types.rs crates/slicer-schema/wit/deps/prepass-support-geometry/prepass-support-geometry.wit crates/slicer-macros/src/lib.rs crates/slicer-wasm-host/src/marshal/in_.rs crates/slicer-wasm-host/src/host.rs && rg -q 'pub interface_plan: Vec<SupportInterfacePlanEntry>' crates/slicer-ir/src/slice_ir.rs && ! rg -q 'SupportInterfacePlan[^E]' crates/slicer-ir/src crates/slicer-sdk/src crates/slicer-schema/wit crates/slicer-macros/src crates/slicer-wasm-host/src && python3 -c "import pathlib,re,subprocess; p='crates/slicer-ir/src/slice_ir.rs'; pat=r'CURRENT_SUPPORT_PLAN_IR_SCHEMA_VERSION: SemVer = SemVer \\{\\s*major: (\\d+),\\s*minor: (\\d+),\\s*patch: (\\d+),'; old=re.search(pat,subprocess.check_output(['git','show','HEAD:'+p],text=True)).groups(); new=re.search(pat,pathlib.Path(p).read_text()).groups(); assert int(new[0])==int(old[0]) and int(new[1])==int(old[1])+1 and int(new[2])==int(old[2]), (old,new)"`
- **AC-2. Given** `SupportPlanIR.interface_plan` exists, **when** `traditional-support` is scheduled, **then** `modules/core-modules/traditional-support/traditional-support.toml` reads `SupportPlanIR` and both support modules consume the same planned entries rather than planner-emitted scan-line geometry. | `rg -q 'reads.*SupportPlanIR' modules/core-modules/traditional-support/traditional-support.toml && rg -q 'SupportPlanIR' modules/core-modules/tree-support/tree-support.toml modules/core-modules/traditional-support/traditional-support.toml && ! rg -q 'push_interface_scan_lines\(' modules/core-modules/support-planner/src/lib.rs`
- **AC-3. Given** `support_interface_top_layers = 2` and a planned top band, **when** either generator runs, **then** top interface paths have `is_top_interface = true`, role `ExtrusionRole::SupportInterface`, and plan-derived `spacing_mm`; given `support_interface_bottom_layers = 3`, exactly three bottom layers have `is_top_interface = false` and no code `1003` diagnostic is emitted. | `cargo test -p tree-support --test interface_layers_tdd --all-targets && cargo test -p traditional-support --test interface_layers_tdd --all-targets && cargo test -p support-planner --test diagnostics_tdd --all-targets`
- **AC-4. Given** blocker, enforcer, and ordinary regions overlap a planned interface band, **when** the plan is consumed, **then** blocker precedence suppresses paths, enforcer precedence permits paths only inside planned regions, and ordinary regions receive no unplanned interface fill. | `cargo test -p tree-support --test interface_layers_tdd --all-targets -- paint_precedence_rejects_unplanned_interface_fill && cargo test -p traditional-support --test interface_layers_tdd --all-targets -- paint_precedence_rejects_unplanned_interface_fill`
- **AC-5. Given** `tmp/visual-debug-support-interface.json` selects bottom layer `0` and top layer `125`, **when** visual-debug runs, **then** the `Layer::Support` capture at layer `0` contains at least one interface tuple whose second element is `false`, the capture at layer `125` contains at least one tuple whose second element is `true`, and both PNGs are emitted. | `cargo run -q -p pnp-cli --bin pnp_cli -- visual-debug --request tmp/visual-debug-support-interface.json --output target/vd-support-interface --overwrite >/dev/null && jq -e '([.images[] | select(.tap == "Layer::Support" and .layer_index == 0) | .typed_capture.value.interface_paths[]? | select(.[1] == false)] | length > 0) and ([.images[] | select(.tap == "Layer::Support" and .layer_index == 125) | .typed_capture.value.interface_paths[]? | select(.[1] == true)] | length > 0) and ([.images[] | select(.tap == "Layer::Support" and (.layer_index == 0 or .layer_index == 125)) | .png_path] | length == 2)' target/vd-support-interface/manifest.json`

## Negative Test Cases

- **AC-N1. Given** `support_interface_bottom_layers` is absent or `-1`, **when** the planner runs, **then** diagnostics contain no code `1003`, no `support_interface_bottom_layers is not yet implemented` message, and no bottom interface plan entries. | `cargo test -p support-planner --test diagnostics_tdd --all-targets -- absent_or_disabled_bottom_interface_has_no_code_1003`

## Verification

- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo run -q -p pnp-cli --bin pnp_cli -- visual-debug --request tmp/visual-debug-support-interface.json --output target/vd-support-interface --overwrite >/dev/null && jq -e '([.images[] | select(.tap == "Layer::Support" and .layer_index == 0) | .typed_capture.value.interface_paths[]? | select(.[1] == false)] | length > 0) and ([.images[] | select(.tap == "Layer::Support" and .layer_index == 125) | .typed_capture.value.interface_paths[]? | select(.[1] == true)] | length > 0)' target/vd-support-interface/manifest.json`

## Authoritative Docs

- `docs/specs/support-generation-remediation-plan.md` - direct read; approved packet queue and interface decision.
- `docs/specs/support-generation-defect-verified-findings.md` - direct bounded read; verified architecture and code-1003 context.
- `docs/01_system_architecture.md` - delegated relevant support-stage sections.
- `docs/02_ir_schemas.md` - delegated `SupportPlanIR` and version sections.
- `docs/03_wit_and_manifest.md` - delegated WIT and manifest sections.
- `docs/08_coordinate_system.md` - delegated geometry-unit section.

## Doc Impact Statement (Required)

- Specific same-packet doc edits: `docs/02_ir_schemas.md` SupportPlanIR section - `rg -q 'SupportInterfacePlanEntry' docs/02_ir_schemas.md`; `docs/01_system_architecture.md` support-interface section - `rg -q 'interface_plan' docs/01_system_architecture.md`; `docs/03_wit_and_manifest.md` Layer::Support manifest section - `rg -q 'traditional-support.*SupportPlanIR|SupportPlanIR.*traditional-support' docs/03_wit_and_manifest.md`.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/Support/TreeSupport.cpp` - top/bottom interface band and alternating dense-fill behavior.

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- obey the shared absolute context bands: 120k reading budget with hand-off at 150k (standard); the extended band (240k reading / 300k hard stop) only via swarm's escalation protocol

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation (an extended-band run may carry a single L step only when `design.md` justifies why it cannot be split).
