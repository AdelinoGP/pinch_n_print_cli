# Requirements: 157-visual-debug-request-bundle-contract

## Packet Metadata

- Grouped task IDs: `TASK-267`
- Backlog source: `docs/07_implementation_status.md`
- Packet status: `active`
- Aggregate context cost: `M`

## Problem Statement

Visual-defect investigation needs a deterministic, opt-in artifact command rather than ad-hoc G-code inspection or a `slice` flag. TASK-267 is the contract foundation for later tap and renderer packets: it must validate one versioned request, establish a trustworthy bundle lifecycle, reject unsafe overwrite and partial-success cases, and define the manifest index without implementing stage taps or rendering.

## In Scope

- Add the separate `pnp_cli visual-debug --request <JSON> --output <DIR>` command boundary.
- Validate request `schema_version: "1.0.0"` and snake_case request keys.
- Accept exactly one source mode: model-backed source with `model`, `config`, and `module_dirs`, or standalone G-code source with `path`.
- Validate `layers`, `taps`, `visualizations`, optional `gcode_line_width_mm`, and `resolution_scale` according to the visual-debug contract.
- Default `resolution_scale` to `1`; accept only `1`, `2`, or `3`, with the documented 1024 x 1024 base raster semantics.
- Create a bundle lifecycle that treats directory and PNG write failure as fatal and never reports a partial bundle as successful.
- Reject a non-empty output directory unless explicit `--overwrite` is supplied; define replacement behavior for the overwrite path.
- Define versioned `manifest.json` as the sole machine-readable index and its documented image-entry fields.
- Add focused contract tests for positive and negative validation/lifecycle behavior.

## Out of Scope

- Stage taps, post-stage capture adapters, or scheduler dependency-closure execution.
- PNG rendering, viewport projection, palettes, legends, or raster encoding.
- Final G-code parsing, unclassified extrusion handling, or unsupported-command warnings.
- Changes to `pnp_cli slice`, ordinary slice behavior, scheduler stages, module manifests, WIT contracts, IR schemas, or progress-event emission.
- Rendering, scheduler closure, stage taps, task-map artifacts, or implementation in this packet.

## Authoritative Docs

- `docs/specs/visual-pipeline-debug.md` - complete direct read; proposal and exact TASK-267 candidate boundary.
- `docs/19_visual_debug.md` - complete direct read; user-facing request, bundle, and failure behavior.
- `docs/adr/0039-visual-debug-is-a-separate-opt-in-artifact-command.md` - complete direct read; accepted command and lifecycle decision.
- `docs/01_system_architecture.md` - lines 460-497 and 678-691; postpass and CLI host context.
- `docs/09_progress_events.md` - complete direct read; structured event/version context to preserve while adding no event surface.
- `docs/11_operational_governance_and_acceptance_gate.md` - complete direct read; compatibility, determinism, recoverability, and operability gates.
- `docs/07_implementation_status.md` - lines 225-243; TASK-267 and packet sequence.

## Acceptance Summary

Reference the authoritative Given/When/Then criteria in `packet.spec.md`.

- Positive: `AC-1` through `AC-5` establish command separation, source exclusivity, manifest entry shape, scale bounds/default, and explicit overwrite behavior.
- Negative: `AC-N1` through `AC-N6` cover mixed/missing sources, invalid scale, missing standalone width, unsafe overwrite, and fatal write failure.
- Cross-packet impact: TASK-268 and TASK-270 may consume the request and manifest contracts; they must not broaden this packet to taps or rendering.

## Verification Commands

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo test -p pnp-cli --all-targets --test visual_debug_request_bundle_tdd` | Run the focused request, manifest, overwrite, and fatal-lifecycle contract tests. | FACT pass/fail; SNIPPETS <=20 lines on failure |
| `cargo check --workspace --all-targets` | Compile the command and contract test targets. | FACT pass/fail |
| `cargo clippy --workspace --all-targets -- -D warnings` | Enforce workspace lint cleanliness for the new command contract. | FACT pass/fail |

## Step Completion Expectations

The request model, validation result, lifecycle state, and manifest serialization must agree on source exclusivity, version, scale, and failure semantics. A successful lifecycle is only observable after the manifest index is committed; later tap/render packets may add image producers without weakening this invariant.

## Context Discipline Notes

The proposal is 235 lines and should be read only at the ranges relevant to request and bundle sections after this packet is emitted. Do not load implementation-wide architecture or generated artifacts to infer a contract that the named docs already define. Cargo verification is delegated and must return only FACT pass/fail, with bounded failure snippets.
