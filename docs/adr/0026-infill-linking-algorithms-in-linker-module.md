# ADR-0026 — Infill Linking Algorithms Live Inside the Infill-Linker Module, Not in `slicer-core`

## Status

Proposed (lands with the infill-parity effort; companion to ADR-0025).

## Context

The infill-parity grilling (2026-07-01) initially proposed placing the shared
infill helpers (`connect_infill`, `chain_or_connect_infill`,
`BoundaryInfillGraph`, `infill_direction`, `ExPolygonWithOffset`,
`adjust_solid_spacing`, `remove_short_polylines`) in a new
`slicer-core::infill_ops` module, sibling to `polygon_ops.rs`, re-exported from
`slicer-sdk`. The rationale was to match the existing `polygon_ops` /
`slicer-sdk::host` precedent: algorithm in `slicer-core`, thin wrapper in
`slicer-sdk`.

After ADR-0025 chose Architecture A (raw emit, post-pass links all), the linker
module became the *sole* caller of `connect_infill`. The question became: does
the algorithm live in `slicer-core::infill_ops` (shared, reusable) or inside the
linker module (single caller, single home)?

The project owner chose the latter. This ADR records the rejection of the
`slicer-core::infill_ops` proposal and the rationale.

## Decision

Infill-specific linking algorithms live **inside the infill-linker module**
(`modules/core-modules/infill-linker/src/lib.rs` and its submodules), NOT in
`slicer-core`. Specifically:

- `connect_infill` (port of FillBase.cpp:1497-2201)
- `chain_or_connect_infill` (port of FillBase.cpp:2201-2300)
- `BoundaryInfillGraph` (arc-length boundary parametrization, FillBase.cpp:1530-1620)
- `remove_short_polylines` (FillGyroid.cpp:356-359)
- The infill overlap offset application (`INFILL_OVERLAP_OVER_SPACING = 0.45`)

`slicer-core` gains **only** `clip_polylines` — a generic Clipper2
polyline-vs-`ExPolygon` intersection operation in `polygon_ops.rs`. This is not
infill-specific: any consumer that needs to clip a polyline against a polygon
(perimeter gap-fill, support interface, ironing) can use it. It matches the
existing `polygon_ops` precedent (generic geometry, no domain logic).

## Consequences

**Positive**:
- `slicer-core` stays generic geometry. No infill-specific algorithm leaks into
  the shared crate. A reviewer of `slicer-core` sees geometry primitives, not
  fill-pattern logic.
- The multi-language module promise is preserved: a C++ or Zig TPMS-infill
  component does not need to depend on a Rust linking helper. Linking is a
  host-side concern handled by the linker module, not a shared library a guest
  must link against.
- The linker module is self-contained: it owns its algorithm end-to-end.
  Swapping the linking strategy (closest-neighbor, monotonic, anchor-based) is a
  one-module swap, with no `slicer-core` change.

**Negative**:
- If a future module wants to self-link (e.g. lightning-infill transitioning to
  Architecture A, or a third-party module that wants connected output without
  the linker), it cannot reuse `connect_infill` from `slicer-core`. It must
  either duplicate the algorithm or depend on the linker module (awkward for a
  `Layer::Infill` module depending on a `Layer::InfillPostProcess` module).
- `lightning-infill` (out of parity scope) currently self-links its own output.
  Under this decision, when lightning switches to Architecture A it will either
  delete its self-linking code (raw emit) and rely on the linker, or keep its
  own copy. The raw-emit path is the intended end state; the transitional
  self-linking is tracked in DEV-081.

**Trade-offs we explicitly accept**:
- Algorithm duplication is possible if a future module self-links. We accept
  this risk because Architecture A's whole point is that modules *don't*
  self-link. If a module wants to self-link, it is choosing Architecture B for
  itself, and the duplication is the cost of that choice.
- `infill_direction` (angle resolution with π/2 offset + reference point) is
  arguably generic, but it is infill-specific (the π/2 is because infill lines
  are perpendicular to the angle). It stays in the linker. If a future non-fill
  consumer needs angle resolution, it can be promoted then.

## Future-Reviewer Notes

- **Do not re-suggest `slicer-core::infill_ops`.** The `polygon_ops` precedent
  applies to *generic* geometry (Clipper2 ops), not to infill-specific linking.
  The two are different: `polygon_ops` is a reusable primitive any consumer
  benefits from; `connect_infill` is a domain algorithm with one caller under
  Architecture A.
- **Do not extract `connect_infill` to `slicer-core` "for reuse" without a
  second concrete consumer.** A hypothetical future self-linking module is not
  a concrete consumer. YAGNI applies.

## References

- `docs/adr/0025-infill-linker-as-raw-emit-post-pass.md` — Architecture A decision.
- `crates/slicer-core/src/polygon_ops.rs` — generic geometry precedent.
- `crates/slicer-core/src/lib.rs:26` — `pub mod polygon_ops` (not `host-algos`-gated; available on wasm32).
- OrcaSlicer `src/libslic3r/Fill/FillBase.cpp:1497-2300` — `connect_infill` source.