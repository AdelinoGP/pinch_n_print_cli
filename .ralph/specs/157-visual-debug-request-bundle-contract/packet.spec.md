---
status: implemented
packet: 157-visual-debug-request-bundle-contract
task_ids:
  - TASK-267
backlog_source: docs/07_implementation_status.md
context_cost_estimate: M
copy_note: Packet 157 is the first visual-debug contract slice; implementation is intentionally deferred to swarm execution.
---

# Packet Contract: 157-visual-debug-request-bundle-contract

## Goal

Define and implement the opt-in `pnp_cli visual-debug` request, validation, output-bundle lifecycle, overwrite policy, and versioned manifest contract without stage taps or rendering.

## Scope Boundaries

This packet owns the dedicated command boundary, versioned request parsing and validation, atomic success/failure lifecycle for the output directory, explicit overwrite behavior, and the machine-readable manifest model. It does not capture stage data, execute tap dependency closures, render PNGs, parse final G-code, or add an agent skill.

## Prerequisites and Blockers

- Depends on: ADR-0039, accepted visual-pipeline-debug proposal, and the existing `pnp_cli` command surface.
- Unblocks: TASK-268 typed tap capture and TASK-270 final G-code renderer.
- Activation blockers: None; the requested packet status is `active`, and an independent reviewer will run preflight.

## Acceptance Criteria

- **AC-1. Given** a model-mode request with `schema_version: "1.0.0"`, `source.kind: "model"`, `source.model`, `source.config`, `source.module_dirs`, `layers`, `taps`, `visualizations`, and `resolution_scale: 1`, **when** `pnp_cli visual-debug --request request.json --output bundle-dir` is executed against a writable, empty output directory, **then** request validation succeeds, the command is recognized independently of `pnp_cli slice`, and the bundle lifecycle reaches a successful manifest-producing state with no stage-tap or PNG-rendering work performed by this packet. | `cargo test -p pnp-cli --all-targets --test visual_debug_request_bundle_tdd -- ac_model_request_accepts_and_creates_manifest_state --exact`
- **AC-2. Given** a standalone request with `schema_version: "1.0.0"`, `source.kind: "gcode"`, `source.path`, `layers`, `taps`, `visualizations`, and `resolution_scale: 1`, **when** the request is validated, **then** it is accepted as the sole source mode and its normalized representation retains `source.kind: "gcode"` and `source.path` without accepting model-only source fields. | `cargo test -p pnp-cli --all-targets --test visual_debug_request_bundle_tdd -- ac_gcode_request_accepts_as_exclusive_source --exact`
- **AC-3. Given** a valid request and a successful bundle lifecycle, **when** the manifest model is serialized, **then** `manifest.json` is the sole machine-readable index and each image-entry shape has exact fields for source mode, requested tap, layer index and applicable Z, visualization type, PNG path, shared viewport, legend version, source IR schema or G-code parser version, and warnings. | `cargo test -p pnp-cli --all-targets --test visual_debug_request_bundle_tdd -- ac_manifest_serializes_required_index_and_entry_fields --exact`
- **AC-4. Given** `resolution_scale` omitted or set to `1`, `2`, or `3`, **when** the request is validated, **then** omitted scale normalizes to `1`, accepted scales are exactly `1`, `2`, and `3`, and the manifest contract records the selected scale while preserving the documented 1024 x 1024 base raster semantics. | `cargo test -p pnp-cli --all-targets --test visual_debug_request_bundle_tdd -- ac_resolution_scale_contract --exact`
- **AC-5. Given** an existing non-empty output directory, **when** `pnp_cli visual-debug --request request.json --output bundle-dir --overwrite` is executed with an otherwise valid request, **then** explicit overwrite permits the lifecycle to replace the prior bundle contents, while a successful run still requires the manifest lifecycle to complete rather than reporting a partial bundle. | `cargo test -p pnp-cli --all-targets --test visual_debug_request_bundle_tdd -- ac_explicit_overwrite_replaces_non_empty_bundle --exact`
- **AC-N1. Given** a request that supplies both `source.kind: "model"` and `source.kind: "gcode"` inputs, **when** validation runs, **then** the command rejects the request with a structured validation error identifying mutually exclusive source modes and does not create a successful bundle. | `cargo test -p pnp-cli --all-targets --test visual_debug_request_bundle_tdd -- ac_n1_rejects_mixed_source_modes --exact`
- **AC-N2. Given** a request with neither a model source nor a G-code source, **when** validation runs, **then** the command rejects the request with a structured validation error identifying the missing source mode and does not create a successful bundle. | `cargo test -p pnp-cli --all-targets --test visual_debug_request_bundle_tdd -- ac_n2_rejects_missing_source_mode --exact`
- **AC-N3. Given** a request with `resolution_scale` equal to `0` or `4`, **when** validation runs, **then** the command rejects the request with a structured validation error identifying the allowed values `1`, `2`, and `3`. | `cargo test -p pnp-cli --all-targets --test visual_debug_request_bundle_tdd -- ac_n3_rejects_out_of_range_resolution_scale --exact`
- **AC-N4. Given** a standalone G-code request selecting `filled_areas` without `gcode_line_width_mm`, **when** validation runs, **then** the command rejects the request and does not infer bead width from E values. | `cargo test -p pnp-cli --all-targets --test visual_debug_request_bundle_tdd -- ac_n4_requires_gcode_line_width_for_standalone_filled_areas --exact`
- **AC-N5. Given** a valid request and a non-empty output directory without `--overwrite`, **when** `pnp_cli visual-debug --request request.json --output bundle-dir` is executed, **then** it fails before replacing existing contents and reports that explicit `--overwrite` is required. | `cargo test -p pnp-cli --all-targets --test visual_debug_request_bundle_tdd -- ac_n5_rejects_non_empty_output_without_overwrite --exact`
- **AC-N6. Given** a directory or PNG write failure during bundle creation, **when** the command runs, **then** it fails rather than reporting success and does not expose a partial bundle as trustworthy evidence. | `cargo test -p pnp-cli --all-targets --test visual_debug_request_bundle_tdd -- ac_n6_write_failure_is_fatal_and_not_success --exact`

## Negative Test Cases

- **AC-N1. Given** both model and G-code source inputs, **when** request validation runs, **then** mutually exclusive source modes are rejected. | `cargo test -p pnp-cli --all-targets --test visual_debug_request_bundle_tdd -- ac_n1_rejects_mixed_source_modes --exact`
- **AC-N5. Given** a non-empty output directory without `--overwrite`, **when** the command runs, **then** existing contents are preserved and the request is rejected. | `cargo test -p pnp-cli --all-targets --test visual_debug_request_bundle_tdd -- ac_n5_rejects_non_empty_output_without_overwrite --exact`
- **AC-N6. Given** a bundle write failure, **when** the command runs, **then** it returns failure and never reports a successful partial bundle. | `cargo test -p pnp-cli --all-targets --test visual_debug_request_bundle_tdd -- ac_n6_write_failure_is_fatal_and_not_success --exact`

## Verification

- `cargo test -p pnp-cli --all-targets --test visual_debug_request_bundle_tdd`
- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`

## Authoritative Docs

- `docs/specs/visual-pipeline-debug.md` - direct read of the complete 235-line proposal; command, request, bundle, lifecycle, and packet-boundary authority.
- `docs/19_visual_debug.md` - direct read of the complete 58-line usage contract; request shape and failure behavior.
- `docs/adr/0039-visual-debug-is-a-separate-opt-in-artifact-command.md` - direct read of the complete 41-line accepted decision; command separation and failure semantics.
- `docs/01_system_architecture.md` - direct reads of lines 460-497 and 678-691; postpass ownership and CLI host boundary.
- `docs/09_progress_events.md` - direct read of the complete 196-line event contract; structured error and wire-version governance context.
- `docs/11_operational_governance_and_acceptance_gate.md` - direct read of the complete 179-line governance contract; compatibility, determinism, recoverability, and operability obligations.
- `docs/07_implementation_status.md` - bounded read of lines 225-243; TASK-267 ownership and dependency ordering.

## Doc Impact Statement (Required)

- **`none`** - this packet implements the contract already specified by the governing docs and changes no existing IR, WIT, scheduler, claim, manifest, host-service, or SDK contract.

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- obey the shared absolute context bands: 120k reading budget with hand-off at 150k (standard); the extended band (240k reading / 300k hard stop) only via swarm's escalation protocol

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation (an extended-band run may carry a single L step only when `design.md` justifies why it cannot be split).
```
