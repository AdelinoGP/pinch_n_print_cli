---
status: draft
packet: 161-visual-debug-agent-verification
task_ids:
  - TASK-271
backlog_source: docs/07_implementation_status.md
context_cost_estimate: M
copy_note: Packets 159 and 160 are generated/draft; renderer exports and manifest seams are forward contracts for this verification packet.
---

# Packet Contract: 161-visual-debug-agent-verification

## Goal

Add an independent visual-debug agent skill and guide examples, then prove the packet-159 and packet-160 artifact contracts, deterministic bundles, and zero ordinary-slice visual-debug overhead without implementing either renderer or the command contract.

## Scope Boundaries

This packet owns agent-facing workflow guidance, examples, focused tap/manifest contract tests, deterministic-bundle checks, and an opt-out ordinary-slice overhead proof. It consumes the visual-debug command and renderer seams from packets 159 and 160; it does not implement capture, parsing, rendering, bundle lifecycle, request validation, CLI behavior, WIT/IR/schema contracts, WASM, Orca parity, or coordinate conversion.

## Prerequisites and Blockers

- Depends on: packet `159-visual-debug-intermediate-renderer`; packet `160-visual-debug-gcode-renderer`; `ADR-0038`.
- Unblocks: closure of TASK-271 and the visual-pipeline-debug packet queue.
- Activation blockers: `[FWD-159-1]`, `[FWD-160-1]`, and `[FWD-157-1]` must be confirmed at implementation time; independent preflight review is required.

## Acceptance Criteria

- **AC-1. Given** a geometry-defect report and no timing/DAG question, **when** an agent follows the visual-debug skill, **then** the skill selects `pnp_cli visual-debug`, reads `manifest.json` before PNGs, and states that `debug-pipeline` is independent rather than a prerequisite, with a working model-mode and standalone-G-code example. | `python3 -c "from pathlib import Path; p=Path('.claude/skills/visual-debug/SKILL.md').read_text(); assert 'pnp_cli visual-debug' in p and 'manifest.json' in p and 'debug-pipeline' in p and 'independent' in p; assert 'model' in p and 'gcode' in p"`
- **AC-2. Given** the two named visual-debug guide examples, **when** each file is inspected, **then** the model-backed file and standalone-G-code file each contain the exact `pnp_cli visual-debug --request <request> --output <bundle>` command shape, their source-specific tap (`stage` or `final_gcode`), a selected layer, `resolution_scale`, a manifest-first inspection of `manifest.json` before PNGs, and the geometry-localization versus `debug-pipeline` evidence boundary. | `python3 -c "from pathlib import Path; checks={'model-backed.md':('source.kind','stage','layer','resolution_scale','manifest.json','debug-pipeline'),'standalone-gcode.md':('source.kind','final_gcode','layer','resolution_scale','manifest.json','debug-pipeline')}; base=Path('.claude/skills/visual-debug/examples'); assert all(all(token in (base/name).read_text() for token in tokens) for name,tokens in checks.items()); assert all('pnp_cli visual-debug --request' in (base/name).read_text() and '--output' in (base/name).read_text() for name in checks)"`
- **AC-3. Given** packet-159 typed captures at their published seam, **when** the runtime contract suite runs, **then** every documented intermediate tap asserts its exact source fields, post-stage identity, layer/tap association, visualization, PNG path, viewport, legend version, source schema version, and warning behavior, failing on a missing or renamed required field. | `cargo test -p slicer-runtime --all-targets --test visual_debug_agent_contract_tdd -- intermediate_tap_manifest_contracts --exact`
- **AC-4. Given** packet-160 final-G-code artifacts at the owning `pnp-cli` seam, **when** the final-renderer contract test runs, **then** every image entry asserts the exact `source.kind`, `tap`, layer index, applicable Z, visualization, `png_path`, shared viewport, legend version, parser version, and warnings. | `cargo test -p pnp-cli --all-targets --test visual_debug_gcode_renderer_tdd -- final_gcode_manifest_contracts --exact`
- **AC-5. Given** identical valid model and standalone-G-code requests, **when** each visual-debug bundle is generated twice into clean output directories, **then** complete `manifest.json` bytes, image-entry and warning ordering, layer/tap ordering, PNG paths, and every PNG byte sequence are identical for both source modes. | `cargo test -p pnp-cli --all-targets --test visual_debug_agent_determinism_tdd -- visual_debug_bundles_are_byte_deterministic --exact`
- **AC-6. Given** an ordinary valid slice invocation with visual debugging not requested, **when** it is compared with the same invocation under the packet's repeated-run measurement harness, **then** no visual-debug capture, allocation, serialization, rendering, process invocation, or visual-debug manifest/PNG is observed and the harness reports the ordinary-slice path as opt-out. | `cargo test -p slicer-runtime --all-targets --test visual_debug_agent_overhead_tdd -- ordinary_slice_has_no_visual_debug_overhead --exact`
- **AC-N1. Given** an agent asks why a slice is slow, a module-DAG edge is missing, or a manifest is invalid, **when** the visual-debug skill is applied, **then** it routes to `debug-pipeline` with the exact `slice --instrument-stderr`, `dag`, or `module diagnose` command and explicitly says not to use `pnp_cli visual-debug`. | `python3 -c "from pathlib import Path; p=Path('.claude/skills/visual-debug/SKILL.md').read_text(); assert all(x in p for x in ('slice --instrument-stderr','pnp_cli dag','pnp_cli module diagnose','timing','DAG','do not use pnp_cli visual-debug'))"`
- **AC-N2. Given** an invalid visual-debug request with an unsupported source/tap combination, **when** the published request-validation seam is exercised, **then** validation fails before renderer invocation or bundle creation and reports the invalid field without a successful manifest or PNG. | `cargo test -p pnp-cli --all-targets --test visual_debug_agent_determinism_tdd -- invalid_visual_debug_request_is_rejected_without_bundle --exact`

## Verification

- `cargo test -p slicer-runtime --all-targets --test visual_debug_agent_contract_tdd -- intermediate_tap_manifest_contracts --exact`
- `cargo test -p pnp-cli --all-targets --test visual_debug_gcode_renderer_tdd -- final_gcode_manifest_contracts --exact`
- `cargo test -p pnp-cli --all-targets --test visual_debug_agent_determinism_tdd -- visual_debug_bundles_are_byte_deterministic --exact`
- `cargo test -p pnp-cli --all-targets --test visual_debug_agent_determinism_tdd -- invalid_visual_debug_request_is_rejected_without_bundle --exact`
- `cargo test -p slicer-runtime --all-targets --test visual_debug_agent_overhead_tdd -- ordinary_slice_has_no_visual_debug_overhead --exact`

## Authoritative Docs

- `docs/specs/visual-pipeline-debug.md` - direct read of the complete proposal; agent boundary, tap inventory, bundle contract, determinism, and no-overhead criteria.
- `docs/19_visual_debug.md` - direct read of the complete usage guide; request authoring, manifest-first inspection, warnings, and resolution cost.
- `docs/17_agent_debugging.md` - direct read of the complete guide; timing/DAG/manifest evidence boundary and commands that must remain with `debug-pipeline`.
- `docs/adr/0038-visual-debug-skill-pairs-with-debug-pipeline.md` - direct read of the accepted decision; independent skill pairing and evidence boundaries.
- `docs/01_system_architecture.md` - direct reads of lines 65-109, 246-387, 460-497, and 621-665; scheduler stages, IR ownership, postpass boundary, and memory model.
- `docs/11_operational_governance_and_acceptance_gate.md` - direct read of the complete governance contract; determinism, recoverability, coupling, and evidence obligations.
- `docs/07_implementation_status.md` - bounded lookup of TASK-271 at line 243; task ownership.
- `.ralph/specs/159-visual-debug-intermediate-renderer/packet.spec.md` - published draft renderer contract and `[FWD]` handoff obligations.
- `.ralph/specs/160-visual-debug-gcode-renderer/packet.spec.md` - published draft final-renderer contract and `[FWD]` handoff obligations.

## Doc Impact Statement (Required)

- **`none`** - this packet adds agent guidance and verification only; it changes no IR, WIT, scheduler, claim, manifest, host-service, SDK, or architecture contract.

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- obey the shared absolute context bands: 120k reading budget with hand-off at 150k (standard); the extended band (240k reading / 300k hard stop) only via swarm's escalation protocol

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation (an extended-band run may carry a single L step only when `design.md` justifies why it cannot be split).
