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

## Consequences

- Orientation output is stable across builds and runs, so invariant tests may assert
  exact angles where canonical cannot.
- On ties we may emit a different direction than a particular OrcaSlicer binary;
  tie cases are expected to be rare (degenerate/symmetric floating-edge sets) and
  any such difference is adjudicated against the ADR, not treated as a parity bug.
