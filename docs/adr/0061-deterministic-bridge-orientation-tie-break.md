# Deterministic tie-break for bridge-orientation candidate selection

Canonical OrcaSlicer's inline `detect_bridging_direction` (`BridgeDetector.hpp`)
accumulates per-normal costs into an `std::unordered_map` and selects the minimum by
iteration order, so equal-cost candidates resolve by hash-table accident — the
"winner" can differ between C++ standard-library builds. Our structural invariants
require reproducible output, so PnP pins a deterministic rule: among candidates whose
cost equals the minimum (exact equality on the accumulated dot-product sum), choose
the smallest quantized angle (`ceil(atan2(n.y, n.x) · 1000)`), matching canonical's
quantization key. This is an intentional, documented divergence from canonical — not
a DEVIATION_LOG entry, which this repository reserves for wrongful deviations.

Status: accepted

## Key-space well-definedness: signed-zero canonicalization (packet 235)

The quantization key is only a valid total order over candidate normals if
geometrically identical normals always produce the same key. IEEE-754 signed
zero breaks that: the right normal `(dy, −dx)` of an exactly vertical edge
yields `-0.0` for the x-component, and `atan2(-0.0, -1.0) = −π` while
`atan2(0.0, -1.0) = +π` — two keys ≈ 6283 apart for the same direction. Under
canonical's hash-order first-wins selection this never matters; under this
ADR's smallest-key rule it would silently invert tie outcomes (e.g. the
equal-cost cross fixture of AC-N1 would resolve to 90° instead of 0°).
Implementation rule: canonicalize `-0.0` component values to `0.0` BEFORE
computing `atan2` for the key (and before any angle conversion). This is not a
semantic divergence from canonical — it removes an artifact canonical never
observes because it never orders by key.

## Consequences

- Orientation output is stable across builds and runs, so invariant tests may assert
  exact angles where canonical cannot.
- On ties we may emit a different direction than a particular OrcaSlicer binary;
  tie cases are expected to be rare (degenerate/symmetric floating-edge sets) and
  any such difference is adjudicated against the ADR, not treated as a parity bug.
