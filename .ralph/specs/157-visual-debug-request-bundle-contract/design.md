# Design: 157-visual-debug-request-bundle-contract

## Controlling Code Paths

- Primary code path: the `pnp_cli` command parser and the new visual-debug request/lifecycle/manifest boundary.
- Neighboring tests/fixtures: a focused `pnp-cli` integration test target named `visual_debug_request_bundle_tdd`; temporary request JSON and output-directory fixtures owned by that target.
- OrcaSlicer comparison: no parity applies; this packet follows the Pinch 'n Print contract and does not port rendering or parsing behavior.

## Architecture Constraints

- Visual debugging is a separate opt-in artifact command; ordinary `pnp_cli slice` must remain unchanged and must not acquire visual-debug capture, serialization, rendering, or process overhead.
- The command accepts exactly one source mode and must never mix model-backed and standalone G-code provenance in one bundle.
- Request keys are snake_case; the request is a versioned visual-debug document, not existing print-config JSON.
- Directory or PNG write failure is fatal, and a partial bundle must never be reported as successful.
- A non-empty output directory is rejected unless `--overwrite` is explicit.
- `manifest.json` is the sole machine-readable index; later tap/render packets extend its producers without changing this lifecycle boundary.
- The manifest and CLI JSON surfaces follow the governance rule that additive fields are minor version changes and renames, removals, type changes, or semantic shifts are major changes.

## Code Change Surface

- Selected approach: add a narrow command-owned request value model, validator, bundle lifecycle state machine, overwrite policy, and manifest model; keep capture and rendering behind later packet boundaries.
- Exact functions, traits, manifests, tests, and fixtures: the `pnp_cli` command dispatch/parser; visual-debug request deserialization and validation; output-directory preparation/overwrite handling; manifest model serialization; `visual_debug_request_bundle_tdd` contract tests. Exact existing symbol names are to be confirmed by the implementation worker before edits.
- Rejected alternatives and reasons: adding `--visual-debug` to `slice` is rejected by ADR-0039 because it implies a full slice and conflates print output with targeted diagnostic work; extending `--report` is rejected because timing reports and geometry snapshots have separate cost, retention, and failure contracts; best-effort partial bundles are rejected because missing evidence can cause false diagnosis.

## Files in Scope (read + edit)

- `crates/pnp-cli/` - role: existing CLI command boundary; expected change: register and dispatch the separate visual-debug command.
- `crates/pnp-cli/tests/` - role: CLI contract verification; expected change: add focused request, manifest, overwrite, and fatal-lifecycle tests.

These are directory-level ownership boundaries because the current implementation symbols and test harness locations must be inventoried by the worker; no third primary file is authorized.

## Read-Only Context

- `docs/specs/visual-pipeline-debug.md` - lines 61-131 and 223-235 only - purpose: request, bundle, lifecycle, and candidate-packet contract.
- `docs/19_visual_debug.md` - lines 16-50 only - purpose: request shape, scale, manifest reading, and failure rules.
- `docs/adr/0039-visual-debug-is-a-separate-opt-in-artifact-command.md` - lines 15-40 only - purpose: accepted command separation and rejected alternatives.
- `docs/01_system_architecture.md` - lines 460-497 and 678-691 only - purpose: pipeline/CLI ownership context.
- `docs/09_progress_events.md` - lines 1-5, 22-24, 47-48, and 111-114 only - purpose: preserve independent structured-event and wire-version ownership.
- `docs/11_operational_governance_and_acceptance_gate.md` - lines 40-82 and 102-117 only - purpose: wire compatibility and acceptance categories.
- `docs/07_implementation_status.md` - lines 225-243 only - purpose: TASK-267 status and packet dependencies.

## Out-of-Bounds Files

- `docs/07_implementation_status.md` - status updates are packet-closure work owned through the prescribed worker dispatch, not this packet authoring change.
- `docs/19_visual_debug.md`, `docs/specs/visual-pipeline-debug.md`, and ADR-0039 - governing docs are read-only for this packet; no doc edits are authorized.
- `crates/slicer-runtime/`, `crates/slicer-scheduler/`, module directories, canonical WIT, IR schemas, progress-event implementation, and `pnp_cli slice` paths - no scheduler, module, WIT, IR, event, or ordinary-slice changes.
- Any stage-tap, renderer, G-code parser, viewport, palette, PNG, rendering, or agent-skill path - owned by later packets.
- `.ralph/specs/*` other than this packet directory, `task-map.md`, `target/`, `Cargo.lock`, generated code, vendored dependencies, and OrcaSlicer source/documentation - never load or edit.

## Expected Sub-Agent Dispatches

- Question: Which existing `pnp_cli` parser/dispatch symbols and test harness path own the new visual-debug command without changing `slice`?; scope: `crates/pnp-cli/src/**` and `crates/pnp-cli/tests/**`; return: `LOCATIONS`; purpose: bind the implementation surface before editing.
- Question: Do the focused visual-debug contract tests prove all packet acceptance criteria and no tap/rendering behavior?; scope: `crates/pnp-cli/tests/visual_debug_request_bundle_tdd.rs`; return: `FACT`; purpose: verify the contract boundary.
- Question: Do the changed CLI JSON and lifecycle errors preserve the governance compatibility and recoverability rules?; scope: changed `crates/pnp-cli/**`; return: `SUMMARY`; purpose: perform a bounded contract review.

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

## Context Cost Estimate

- Aggregate: `M`
- Largest step: `M`
- Highest-risk dispatch and required return format: implementation-surface inventory; `LOCATIONS` with at most 20 entries and one context line each.

## Open Questions

- [FWD] The implementation worker must resolve the exact existing `pnp_cli` module and test-harness symbols while preserving the directory-level code surface and all locked JSON names.
