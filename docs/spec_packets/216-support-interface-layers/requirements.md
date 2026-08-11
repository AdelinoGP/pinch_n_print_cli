# Requirements: support-interface-layers

## Packet Metadata

- Grouped task IDs: `TASK-325`
- Backlog source: `docs/07_implementation_status.md`
- Packet status: `draft`
- Aggregate context cost: `M`

## Problem Statement

The planner currently mixes top-interface scan-line geometry into `SupportPlanEntry.branch_segments`, while bottom-interface configuration produces code `1003`. This slice establishes a typed plan-to-module boundary and makes both declared support-generator winners consume it.

## In Scope

- Add canonical `SupportInterfacePlanEntry` and `SupportInterfaceKind` (`Top`, `Bottom`) to the IR and SDK prepass types; add `SupportPlanIR.interface_plan` with `#[serde(default)]` and update default and every explicit `SupportPlanIR` literal.
- Add the WIT record and output push method in `crates/slicer-schema/wit/deps/prepass-support-geometry/prepass-support-geometry.wit`; update SDK output builder/types, `crates/slicer-macros` conversion, and `crates/slicer-wasm-host` collection and marshal/harvest owners.
- Bump `CURRENT_SUPPORT_PLAN_IR_SCHEMA_VERSION` from `1.3.0` to `1.4.0`; update its documentation and all old-version assertions/literals identified in the plan.
- Refactor `support-planner` to emit plan records and no interface scan-line geometry or code `1003`.
- Add `SupportPlanIR` to `modules/core-modules/traditional-support/traditional-support.toml` reads; both support modules generate planned dense paths with role, flag, spacing, and paint precedence.
- Add focused tests, `tmp/support-config-interface.json`, and `tmp/visual-debug-support-interface.json`; the request selects bottom layer `0` and top layer `125`.
- Update the three authoritative documentation sections named by `packet.spec.md`.

## Out of Scope

- Raft geometry, branch propagation, fallback overhang clipping, support-type variants, and final G-code.
- Numerical Orca parity beyond structural top/bottom interface-band behavior.

## Authoritative Docs

- `docs/specs/support-generation-remediation-plan.md` - direct read; approved interface scope.
- `docs/specs/support-generation-defect-verified-findings.md` - direct read through line 260; verified current behavior.
- `docs/01_system_architecture.md`, `docs/02_ir_schemas.md`, `docs/03_wit_and_manifest.md`, `docs/08_coordinate_system.md` - delegated relevant sections.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/Support/TreeSupport.cpp` - top/bottom interface band and alternating dense-fill behavior.

## Acceptance Summary

Reference criteria in `packet.spec.md`: positive `AC-1` through `AC-5`; negative `AC-N1`. Cross-packet impact: consumes TASK-322's branch-radius prerequisite and exports `SupportInterfacePlanEntry`, `SupportInterfaceKind`, and `SupportPlanIR.interface_plan` for TASK-327.

## Verification Commands

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo test -p support-planner --test diagnostics_tdd --all-targets` | code-1003 removal and disabled behavior | FACT pass/fail |
| `cargo test -p tree-support --test interface_layers_tdd --all-targets && cargo test -p traditional-support --test interface_layers_tdd --all-targets` | planned paths, flags, spacing, precedence | FACT pass/fail |
| `cargo run -q -p pnp-cli --bin pnp_cli -- visual-debug --request tmp/visual-debug-support-interface.json --output target/vd-support-interface --overwrite >/dev/null && jq -e '([.images[] | select(.tap == "Layer::Support" and .layer_index == 0) | .typed_capture.value.interface_paths[]? | select(.[1] == false)] | length > 0) and ([.images[] | select(.tap == "Layer::Support" and .layer_index == 125) | .typed_capture.value.interface_paths[]? | select(.[1] == true)] | length > 0)' target/vd-support-interface/manifest.json` | visual gate, asserting non-empty bottom and top captures | FACT pass/fail |
| `cargo check --workspace --all-targets` | additive IR/WIT literal blast radius | FACT pass/fail |
| `cargo clippy --workspace --all-targets -- -D warnings` | lint gate | FACT pass/fail |

## Step Completion Expectations

The planner is the sole source of layer/kind/density/spacing decisions. `traditional-support` and `tree-support` both declare and consume `SupportPlanIR`; no interface path is emitted without a matching plan record. The additive field remains defaultable for old serialized captures.

## Context Discipline Notes

Delegate authoritative docs and Orca reads; use bounded `LOCATIONS` for every `SupportPlanIR` literal, schema assertion, WIT binding owner, and host collection owner before edits. Do not read generated WASM or `target/`.
