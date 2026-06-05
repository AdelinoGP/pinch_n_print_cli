# ADR-0008 — Overhang Annotation is a FinalizationModule, Not a New Stage

## Status

Accepted

## Context

The overhang-classification algorithm (originally in `slicer-core/src/algos/overhang_classifier.rs`) was a host-only path called directly by `slicer-gcode`'s emit function. This baked overhang-feedrate selection into the host serializer, leaving zero swap-point for users who want different overhang behavior.

The deepening-plan grilling (Q3, Q6) explored whether to add a new `PostPass::OverhangAnnotation` stage with a dedicated WIT export, or to ship a `FinalizationModule` core-module.

## Decision

Overhang annotation is implemented as a `FinalizationModule` core-module (`overhang-classifier-default`) that:
- Owns the complete classification algorithm (relocated from `slicer-core`)
- Reads config from `config-view` (the four `overhang_*_4_speed` fields)
- Emits `set-speed-factor` mutations through `FinalizationOutputBuilder`
- Claims no new stage, no new WIT export
- Has no host fallback — users opt out by curating their module dir

## Consequences

- **No WIT contract change**: existing 20 core-modules are unaffected (no rebuild churn).
- **User opt-out**: removing `overhang-classifier-default` from the module dir means no overhang annotation — the slice runs at base feedrate for all walls.
- **LSB-precision trade-off**: the multiplicative `factor = overhang_speed / base_speed` path may introduce feedrate decimal shifts in the 3rd–6th decimal vs the old direct-lookup path. This is acceptable for real-world printers.
- **Algorithm self-containment**: the guest module does NOT depend on `slicer-core` (preventing the `host-algos` feature gate from contaminating the guest dep tree). The algorithm is fully relocated.
- **Future reviewers** should not re-suggest a dedicated stage (unnecessary scope) or re-locate the algorithm to `slicer-core` (breaks the self-containment invariant).
