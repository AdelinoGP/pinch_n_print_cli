---
status: implemented
packet: 157-visual-debug-request-bundle-contract
task_ids:
  - TASK-267
---

# 157-visual-debug-request-bundle-contract

## Goal

Define and implement the opt-in `pnp_cli visual-debug` request, validation, output-bundle lifecycle, overwrite policy, and versioned manifest contract without stage taps or rendering.

## Problem Statement

Visual-defect investigation needs a deterministic, opt-in artifact command rather than ad-hoc G-code inspection or a `slice` flag. TASK-267 is the contract foundation for later tap and renderer packets: it must validate one versioned request, establish a trustworthy bundle lifecycle, reject unsafe overwrite and partial-success cases, and define the manifest index without implementing stage taps or rendering.

## Architecture Constraints

- Visual debugging is a separate opt-in artifact command; ordinary `pnp_cli slice` must remain unchanged and must not acquire visual-debug capture, serialization, rendering, or process overhead.
- The command accepts exactly one source mode and must never mix model-backed and standalone G-code provenance in one bundle.
- Request keys are snake_case; the request is a versioned visual-debug document, not existing print-config JSON.
- Directory or PNG write failure is fatal, and a partial bundle must never be reported as successful.
- A non-empty output directory is rejected unless `--overwrite` is explicit.
- `manifest.json` is the sole machine-readable index; later tap/render packets extend its producers without changing this lifecycle boundary.
- The manifest and CLI JSON surfaces follow the governance rule that additive fields are minor version changes and renames, removals, type changes, or semantic shifts are major changes.

## Data and Contract Notes

- IR/manifest contracts: this packet defines a visual-debug `manifest.json` artifact index, not a module manifest or IR schema. Image entries record source mode, requested tap, layer index/Z where applicable, visualization type, PNG path, viewport, legend version, IR schema version or G-code parser version, and warnings.
- WIT boundary: none; no new WIT resource, method, or module-visible access is permitted.
- Determinism/scheduler constraints: request validation and manifest serialization must be deterministic; no tap creates scheduler edges, module invocations, or module-visible access in this packet because taps are deferred.

## Locked Assumptions and Invariants

- The command syntax is `pnp_cli visual-debug --request request.json --output bundle-dir`, with optional `--overwrite`.
- `schema_version` is `"1.0.0"`; all request keys are snake_case.
- Exactly one source mode is required; standalone `filled_areas` requires `gcode_line_width_mm`.
- `resolution_scale` defaults to `1` and accepts only `1`, `2`, or `3`.
- No successful result may represent a partial bundle, and non-empty output requires explicit overwrite.

## Risks and Tradeoffs

- The proposal describes manifest entry fields but leaves implementation-language names open; the worker must preserve the documented JSON field names and avoid inventing renderer-specific fields in this packet.
- Overwrite replacement must not expose old and new artifacts as one successful bundle; tests must prove the failure and replacement boundaries.
- Future tap/render packets depend on this model, so compatibility changes require the governance versioning policy rather than ad-hoc fields.
