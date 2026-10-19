# ADR-0072 — Context-aware feedrate resolution preserves the factor contract

## Status

Accepted (2026-09-30). The user authorized Q8, "Amend ADR", in the current
conversation for `config-scope-resolution_10_remaining-automatic-values`.
This is a narrow architectural mechanism amendment, not a clarification or
packet closure. Registered deviation: `D-CSR10-ADR-0052-AMENDED` in
`docs/DEVIATION_LOG.md`. Architectural acceptance remains in force independently
of packet closure. The subsequent cold review reopened the packet; its current
status and TASK-571 must be read from the packet contract and backlog, not
inferred from this decision's Accepted status.

## Context and contested clauses

[ADR-0052](./0052-per-point-speed-factor-contract.md) §Decision 1 requires:

> `resolve_feedrate`'s signature and body are **unchanged**.

> This keeps `resolve_feedrate` the single place speed is resolved across the tier
> boundary: base-speed selection and the `clamp(0.05, 5.0)` stay host-side, in one
> function, for both the whole-entity and per-point paths.

It also specifies the direct production call:

> `self.resolve_feedrate(role, profile.and_then(|p| p.get(original_index).copied()).unwrap_or(entity.path.speed_factor))`.

The inspected `DefaultGCodeEmitter` in `crates/slicer-gcode/src/emit.rs` now
delegates `resolve_feedrate` to private `role_base_speed_mm_per_s` and
`feedrate_from_base_mm_per_s`; extrusion production calls private
`resolve_extrusion_feedrate`. Keeping factor-valued inputs does not by itself
satisfy the unchanged-body, single-function-placement and direct-call clauses.
Preflight S8 correctly blocked the previous conformance-only claim.

Configured role speed zero needs the active tool and live width, height delta
and flow to derive an automatic base. Config-only, context-free role selection
cannot derive that base. Retaining the original body and direct call would
require simulating move context in emitter configuration, cloning emitters for
context-specific speeds, or abandoning geometry-dependent automatic speed.
Those alternatives obscure the move-owned context or drop the selected feature.

## Decision — narrowly superseding ADR-0052 §Decision 1's mechanism

- Retain the public `DefaultGCodeEmitter::resolve_feedrate(&self, role:
  &ExtrusionRole, speed_factor: f32) -> Option<f32>` signature and role/factor
  semantics. Its body may delegate to private helpers. It remains context-free:
  configured role zero resolves to the role-only zero placeholder, not an
  automatic extrusion speed. Production extrusion must use the private
  context-aware resolver instead.
- Keep base selection and factor policy host-side, but allow private
  `role_base_speed_mm_per_s` and `resolve_extrusion_feedrate` to select the base,
  and one private `feedrate_from_base_mm_per_s` to apply `clamp(0.05, 5.0)` and
  mm/s-to-mm/min conversion. Both per-point and whole-entity factors, for
  automatic and explicit bases, share that single clamp/conversion policy.
- Exactly zero configured role speed selects the resolved active-tool limit
  divided by `width × height_delta × flow_factor`; only an absent tool config
  falls back to the resolved global config. Present zero or invalid limits do
  not widen to global. Inputs, product, quotient and narrowed base must be
  finite positive; automatic factors must be finite. The final converted,
  rounded/narrowed `F` must be finite positive or fail closed through
  `GCodeEmitError::Emit`. Explicit positive configured speeds remain
  intentionally uncapped by this automatic fallback.
- The inspected conversion helper multiplies in f64, rounds to three decimal
  places, then narrows to f32. This describes the actual shared implementation;
  it is not a claim that all explicit outputs are byte-identical or that tests
  have passed. Final automatic-F safety is checked after this conversion.

Only the unchanged-body requirement, exact single-function placement, and
literal direct-production-call requirement are superseded. References to the
direct fallback call in ADR-0052's unchanged Consequences are read through this
mechanism amendment, not as permission to alter fallback semantics. All other
constraints remain: `EntitySpeedProfile.factors` is `Vec<f32>` of multipliers,
never absolute speeds; the `entity_id`-keyed carrier, profile length check at
mutation application, replacement rather than append, per-point replacement
rather than composition with the entity scalar, original-index lookup and
absent-profile entity-factor fallback, and mutation/application rules are
retained. Producers do not send absolute mm/s. No WIT, IR layout, public factor
interface or other ADR decision changes. Existing declared-extension transport
retains RegionMapIR 3.0.0 and its independent pre-change Postcard fixture.

## Consequences and verification

The private move resolver owns geometry-dependent selection without moving
speed policy into factor-producing guests. The original ADR-0052 Decision,
Consequences, reconciliation and 2026-08-05 amendment remain textually intact;
its packet-10 append points here. The deviation records the intentional
mechanism departure; its current disposition is authoritative in
`docs/DEVIATION_LOG.md`, not duplicated in this decision record.

Packet ACs require independent literal emitted-F assertions, automatic invalid
input/final-F failures, inline model-emitter controls, and the actual CLI
global-8/tool-0-12 control with the independent 12/8 ratio. Source inspection and
documentation greps are not behavior proof. Historical preflight/closure
claims do not establish the reopened packet's current test results. Re-derive
status and evidence from
`docs/spec_packets/config-scope-resolution_10_remaining-automatic-values/packet.spec.md`,
`docs/07_implementation_status.md` and the packet's
[review remediation](../spec_packets/config-scope-resolution_10_remaining-automatic-values/review-remediation.md).
This accepted decision does not authorize a commit or close the packet.
Performance impact is unmeasured; no timing claim is made by this decision
record.
