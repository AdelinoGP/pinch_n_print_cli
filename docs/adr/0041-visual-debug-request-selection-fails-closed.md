# ADR-0041 — Visual-Debug Request Selection Fails Closed

## Status

Accepted.

## Context

The visual-pipeline-debug spec's second success criterion states a valid request
"produces a manifest and every requested PNG, or the command fails. A partial
bundle is never reported as successful." The landed implementation violated this
in three places: a `diagnostic_overlay` requested against a G-code source was
silently dropped (`crates/pnp-cli/src/visual_debug.rs` gcode visualization
filter); an unrecognized visualization kind was silently skipped (the model
dispatch loop's `None`/`continue`); and `LayerSelector::Name` or a z-only
`Detail` selector was silently discarded on the model path while being hard-
rejected on the G-code path. Each produced an exit-0 bundle missing requested
evidence — the exact false-success the criterion forbids.

Two facts constrain the fix. Layers in this system are anonymous:
`GlobalLayer` carries `index`, `z`, and flags but no name, so
`LayerSelector::Name` (which exists only because the selector is
`#[serde(untagged)]`, making any JSON string parse as a name) has no resolution
target anywhere. And selector resolution needs a layer schedule that does not
exist until after model load / prepass (or, for G-code, after parsing
`;LAYER_CHANGE`/`;Z:`), so not every check can happen in the pre-load validator.

## Decision

Visual-debug request selection fails closed. No requested visualization or layer
is ever silently omitted from a bundle the command reports as successful.

- **Layer selection is by anonymous position only:** `Index`, an explicit
  `{start, end}` range (added with `deny_unknown_fields` so a malformed selector
  errors instead of parsing as an empty `Detail`), and z-only `Detail` resolved
  to a real scheduled layer. `Name` is rejected with a clear error stating layers
  are anonymous.
- **Validation is two-phase, both phases fail-closed before any bundle write.**
  Phase 1 (`validate_request`, pre-load) rejects unknown visualization kinds,
  source+visualization mismatches (`diagnostic_overlay` on a G-code source), and
  `Name` selectors, and centralizes the previously scattered silent-drop checks
  so those sites become unreachable. Phase 2 resolves `Index`/range/z-only
  selectors against the schedule (model: committed `LayerPlanIR.global_layers`;
  G-code: parsed `;Z:` order) and fails closed when a selector resolves to no
  layer.

## Consequences

- Every bundle reported successful contains exactly the requested images; a
  request that cannot be fully satisfied fails with a named field error and
  writes nothing.
- New `ValidationError` variants name the offending field; the gcode-only
  selector rejection is subsumed by the unified phase-1 checks.
- Callers must express layer ranges via `{start, end}` or index lists; there is
  no named-layer addressing. This is a deliberate limitation of an anonymous
  layer model, documented in the visual-debug guide.
- Manifest-level and per-image `warnings` remain reserved for legitimately
  rendered-with-caveats output (e.g. unclassified final extrusion), never for
  dropped selections — because selections are no longer dropped.

## Alternatives Considered

- **Warn-and-continue: render what is possible, list dropped items in manifest
  warnings.** Rejected: a requested image absent from a "successful" bundle can
  cause a false diagnosis, the same failure ADR-0039 rejected for partial
  bundles.
- **Implement named-layer resolution.** Rejected: there is no layer-name concept
  to resolve against; inventing one is a separate feature, not a bug fix.
- **Keep resolution entirely in the pre-load validator.** Rejected: the schedule
  needed to resolve z-only and range selectors does not exist until after
  prepass/parse, so a second fail-closed phase is required.
