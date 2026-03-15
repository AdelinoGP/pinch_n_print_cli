# ModularSlicer — Architecture Gate Metrics

This document defines objective thresholds for the Architecture Acceptance Gate.

## Determinism
- Repeat-run test count: 10 runs per reference fixture.
- Input/config must be byte-identical for all runs.
- PASS criteria:
  - `LayerCollectionIR` canonical hash identical across all runs.
  - Claim holder map identical for every `(layer, object, region, claim)`.

Canonical hash method (normative):
- Serialize `LayerCollectionIR` with deterministic field order.
- Normalize numeric values to canonical scaled-int form where applicable.
- Hash algorithm: `SHA-256` over serialized bytes.
- Exclude telemetry/runtime-only fields (`elapsed_ms`, timestamps, UUID run ids).

## Recoverability
- Failure-injection tests must cover at least:
  - one fatal module error,
  - one non-fatal module error,
  - one host contract validation failure.
- PASS criteria:
  - fatal path aborts immediately and emits fatal event,
  - non-fatal path completes with `degraded=true`.

## Resource Bounds
- Memory budget: peak RSS <= 512 MB on 500-layer reference fixture.
- Time budget: full slice <= 10 seconds on 50-layer benchy reference fixture.
- Layer budget: host rejects plans with `GlobalLayer.index >= 100_000`.

## Reference Fixture Set (Normative)
- `benchy_50l_0p2_single_tool`
- `multi_object_dual_material_sync_200l`
- `paint_overlap_material_fuzzy_support_120l`
- `high_region_count_modifier_stress_500l`
- `support_enforcer_blocker_conflict_80l`

Fixture governance:
- Any fixture change requires changelog entry and baseline refresh.
- Gate reports must list fixture IDs and git revision of fixture definitions.

## Coupling Control
- Zero undeclared IR access violations in validation output.
- Zero unresolved write conflicts.
- Zero ambiguous claim holders.

## Compatibility
- Startup compatibility checks must pass for Host/WIT/IR/manifest matrix.
- At least one representative compatible module set and one incompatible set must be validated.

## Operability
- Progress events validate against schema v1.
- Required event set present for each run:
  - `phase_start` + `phase_complete` for all phases,
  - `layer_start` + `layer_complete` for every processed layer,
  - `slice_complete` exactly once.
