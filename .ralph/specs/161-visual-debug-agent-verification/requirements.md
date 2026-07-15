# Requirements: 161-visual-debug-agent-verification

## Packet Metadata

- Grouped task IDs: `TASK-271`
- Backlog source: `docs/07_implementation_status.md`
- Packet status: `draft`
- Aggregate context cost: `M`
- Dependencies: packets `159-visual-debug-intermediate-renderer`, `160-visual-debug-gcode-renderer`, and `ADR-0038`; both renderer packets are generated/draft.

## Problem Statement

The visual-debug infrastructure has renderer contracts but no agent-facing operating surface or final verification slice. TASK-271 closes that gap by teaching agents when and how to use visual evidence, preserving `debug-pipeline` ownership of timing/DAG/manifest diagnosis, and proving that the two renderer outputs are contract-complete, deterministic, and absent from ordinary slicing.

## In Scope

- Add an independent `.claude/skills/visual-debug/SKILL.md` with source selection, request authoring, manifest-first bundle inspection, warning handling, scale guidance, failure handling, and cross-links to `debug-pipeline`.
- Add concrete model-backed and standalone-G-code guide examples under `.claude/skills/visual-debug/examples/`.
- Add focused typed-tap and manifest contract tests covering every documented intermediate tap source field, post-stage identity, image metadata, schema version, and warnings in `slicer-runtime`, plus the packet-160 final-renderer manifest contract at its owning `pnp-cli` seam.
- Add deterministic-bundle tests for model-backed intermediate output and standalone final-G-code output, including byte-level manifests and PNGs.
- Add a repeated-run proof that ordinary `pnp_cli slice` has no visual-debug capture, allocation, serialization, rendering, process, manifest, or PNG overhead when visual debugging is not invoked.
- Preserve explicit forward contracts for packet 159, packet 160, and their packet 157 dependency until those generated/draft exports are confirmed.

## Out of Scope

- Renderer, rasterizer, PNG encoder, typed capture adapter, final-G-code parser, or bundle lifecycle implementation.
- Request validation, source-mode selection, CLI command contract, overwrite behavior, or changes to ordinary `pnp_cli slice` behavior.
- WIT, IR schema, manifest schema ownership, scheduler edges, module manifests, host services, SDK, guest artifacts, or WASM implementation/build changes.
- Coordinate-system changes or new mm/unit conversion logic; no geometry is ported or constructed by this packet.
- OrcaSlicer parity, Orca source translation, pixel/perceptual comparison, HTML gallery, or frontend integration.

## Authoritative Docs

- `docs/specs/visual-pipeline-debug.md` - complete direct read; lines 20-35, 41-59, 99-131, 143-178, 180-221, and 223-235 govern scope, contracts, taps, agent surface, and no-overhead behavior.
- `docs/19_visual_debug.md` - complete direct read; request authoring, bundle inspection, warnings, resolution cost, and failure behavior.
- `docs/17_agent_debugging.md` - complete direct read; independent timing, DAG, and manifest diagnosis surface.
- `docs/adr/0038-visual-debug-skill-pairs-with-debug-pipeline.md` - complete direct read; independent skill decision and evidence boundaries.
- `docs/01_system_architecture.md` - lines 65-109, 246-387, 460-497, 621-665; stage ordering, typed ownership, postpass, and memory boundaries.
- `docs/11_operational_governance_and_acceptance_gate.md` - complete direct read; determinism, recoverability, coupling, and evidence rules.
- `docs/07_implementation_status.md` - delegated bounded lookup for TASK-271 at line 243.
- `.ralph/specs/159-visual-debug-intermediate-renderer/**` - published draft contract; bounded read of packet artifacts only.
- `.ralph/specs/160-visual-debug-gcode-renderer/**` - published draft contract; bounded read of packet artifacts only.

## Acceptance Summary

Reference, never copy, criteria from `packet.spec.md`.

- Positive: `AC-1` through `AC-6` cover the independent skill, examples, all-tap contract coverage, final-renderer manifest coverage, deterministic bundles, and no-overhead proof.
- Negative: `AC-N1` through `AC-N2` cover routing non-geometry diagnosis to `debug-pipeline` and rejecting an invalid source/tap request before rendering or bundle creation.
- Cross-packet impact: packet 159 owns typed intermediate rendering; packet 160 owns final-G-code parsing/rendering; packet 157 owns request and bundle lifecycle; packet 161 consumes those surfaces and owns agent guidance and verification only.
- Forward contracts: `[FWD-159-1]` packet 159 must publish the stable typed-renderer test/export seam and image-entry fields needed to assert every documented intermediate tap. `[FWD-160-1]` packet 160 must publish the stable final-G-code invocation/test seam, parser-version field, warning ordering, and image-entry fields. `[FWD-157-1]` packet 157 must publish the validated request and complete bundle handoff used by both deterministic tests. These are not replacement APIs.

## Verification Commands

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo test -p slicer-runtime --all-targets --test visual_debug_agent_contract_tdd -- intermediate_tap_manifest_contracts --exact` | Assert exact documented intermediate tap fields, manifest metadata, schema version, and warnings. | FACT pass/fail; SNIPPETS <=20 lines on failure |
| `cargo test -p pnp-cli --all-targets --test visual_debug_gcode_renderer_tdd -- final_gcode_manifest_contracts --exact` | Assert exact final-renderer output, layer/tap association, and manifest fields at the owning seam. | FACT pass/fail; SNIPPETS <=20 lines on failure |
| `cargo test -p pnp-cli --all-targets --test visual_debug_agent_determinism_tdd -- visual_debug_bundles_are_byte_deterministic --exact` | Compare model and standalone-G-code manifests and PNG bytes across clean repeated runs. | FACT pass/fail; SNIPPETS <=20 lines on failure |
| `cargo test -p slicer-runtime --all-targets --test visual_debug_agent_overhead_tdd -- ordinary_slice_has_no_visual_debug_overhead --exact` | Prove ordinary slice does not enter or allocate the visual-debug path. | FACT pass/fail; SNIPPETS <=20 lines on failure |
| `cargo check --workspace --all-targets` | Compile all changed and test targets. | FACT pass/fail |
| `cargo clippy --workspace --all-targets -- -D warnings` | Enforce workspace lint gate. | FACT pass/fail |

## Step Completion Expectations

- The skill never requires `debug-pipeline` before a geometry investigation, and both guides state their distinct evidence boundaries.
- Contract assertions enumerate exact documented source, output, layer, tap, path, viewport, legend, schema/parser, and warning fields rather than relying only on image existence or counts; final-renderer assertions run in `pnp-cli`.
- Determinism tests use clean output directories for both source modes and compare complete manifest/PNG bytes, image/layer/tap/warning ordering, paths, and warnings.
- Negative verification covers both `debug-pipeline` routing and request validation before renderer or bundle creation.
- The overhead proof observes the ordinary slice path without adding instrumentation to that path.
- Any unresolved renderer or lifecycle export remains a named `[FWD]` failure, never an inferred compatibility shim.

## Context Discipline Notes

- Read packet 159 and packet 160 only for published contracts and forward seams; do not inspect renderer implementation broadly.
- `docs/01_system_architecture.md` is large; use only the listed ranges and delegated symbol lookups.
- Cargo commands and test-output inspection are delegated and return FACT or bounded failure snippets.
