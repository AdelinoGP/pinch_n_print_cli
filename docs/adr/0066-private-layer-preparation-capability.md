# ADR-0066 — Private preparation belongs to the consuming Layer module

Status: **Accepted.** Confirmed with the approved source plan (interview Q23);
not yet implemented.

Cross-layer strategies such as lightning need whole-print planning before
ordinary Layer execution. We choose an opt-in preparation capability in the same
artifact as the consuming Layer module, with a private immutable plan whose
format belongs to that module. The host owns transport, lifetime, access checks,
and selection, while module authors own planning and interpretation. Full design
and acceptance gates: [Generalized private Layer-module preparation](../specs/layer-module-preparation-plan.md).

## Why this shape

The module remains in one scheduled stage. A separately versioned preparation
capability extends ADR-0045's export model without recreating a monolithic tier
world. Selecting or replacing the module selects or replaces preparation and
consumption together. This gives private plans better locality than a companion
planner artifact, which would require a dependency and payload-compatibility
agreement between independently replaceable halves. A shared plan catalog adds
provider resolution and cross-module compatibility that this ownership model
does not need.

Preparation runs after the relevant completed PrePass products and before Layer
execution. It receives declaration-gated whole-print context plus distinct
selected targets with effective configuration. Layers continue run-to-completion;
the host retains plan bytes, not unfinished arenas or live module state. Native
and WASM adapters follow ADR-0056's single module model.

## Consequences

- Plans are module-private named byte pieces, published atomically on successful
  preparation. A selected module requires preparation success; successful empty
  output is a ready no-work plan, not a missing prerequisite.
- Owner-wide reads are immutable and bounded by explicit requested ranges. Plans
  are print-scoped in-memory data, including anchored consumers. Initial resource
  policy is accounting and measurement, with no new retained-plan quota.
- Shared full-identity targeting/configuration is a separate blocking
  prerequisite. Existing variant-key delivery defects must not be frozen into
  the preparation contract.
- `visual-debug` support is optional per module but included in the framework.
  Requested preparation views produce standard diagnostic geometry during
  preparation, committed atomically alongside the opaque plan and read by the
  ordinary post-commit capture path. Initial views are XY. There is no separate
  visualization export or host decoder for the private plan format.
- A real combined-export/native pilot must prove resource and import composition,
  fresh-instance reads, and ordinary-guest compatibility before integration.
  Framework completion is independent of lightning migration correctness.
- Lightning migration requires a demonstrated canonical-correct PrePass input
  strategy and portable module-owned preparation. When that gate and migration
  acceptance pass, remove the old host producer, `LightningTreeIR` path, and
  `lightning-tree-segments` accessor directly, with affected contract/binding
  changes. There is no deprecation shim or approximate-geometry fallback.

## Relationship to existing decisions

- **ADR-0045:** extend one scheduled-stage export with a declared, independently
  versioned preparation capability. Preserve explicit typed compatibility and
  host-owned imported resource identity.
- **ADR-0056:** preserve one ingestion, selection, and override model with
  equivalent native/WASM preparation and Layer adapters.
- **ADR-0029:** replace its host-owned lightning production decision when the
  gated migration lands; retain the requirement for whole-print computation
  before per-layer sampling. Until then its implementation remains present.
- **ADR-0037 / ADR-0040:** extend post-commit, dependency-closure-aware visual
  captures with generic module-provided diagnostic projections.
