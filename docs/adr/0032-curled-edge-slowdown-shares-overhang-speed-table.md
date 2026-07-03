# ADR-0032: Curled-Edge Slowdown Shares the Overhang Speed Table, Computed Transiently

## Status

Accepted.

## Context

DEV-009's "curled-edge slowdown" sub-item (OrcaSlicer's `prev_curled_extrusions` concept: slow
down printing near wall material that already curled/lifted on the layer below) was long deferred
as "blocked on an unbuilt support-spot generator." That premise was false — verified against
`OrcaSlicerDocumented/src/libslic3r/Support/SupportSpotsGenerator.hpp/.cpp`: the file's
support-point-placement code is entirely dead/commented-out upstream (only referenced from a header
name that no longer matches its own content). The only LIVE code in that file is curl *estimation*
(`get_flow_width`, `estimate_curled_up_height`, `estimate_malformations`, ~150 lines), which runs as
an independent post-slice step over each layer's own already-generated wall geometry — nothing to do
with support generation. It's consumed at the same speed-calculation site as overhang slowdown
(`OrcaSlicerDocumented/src/libslic3r/GCode/ExtrusionProcessor.hpp`), whose own comment states the
intent directly: curl proximity is synthesized into an "artificial distance" specifically so it can
run through the *same* distance→speed curve as overhang, "keeping printer tuning in one table
instead of adding a separate curled-speed profile."

Two design questions followed directly from unblocking this sub-item:

1. **Does curl need its own speed-configuration surface**, or should it reuse `overhang_quartile`'s
   existing `overhang_1_4_speed`..`overhang_4_4_speed` config keys and `BAND_BOUNDARY_MULTIPLIERS`
   distance bands (`crates/slicer-core/src/algos/overhang_annotation.rs`)?
2. **Does curl need a persisted IR/WIT field** (mirroring `overhang_quartile`'s own construction —
   a WIT record field, IR schema-version bump, and marshal fan-out across every `Point3WithWidth`
   site), or can it stay entirely internal to the module that computes and consumes it?

## Decision

**Reuse the overhang speed table.** Curl distance is synthesized into an **artificial curl distance**
(`crates/slicer-core`-adjacent constants duplicated locally in
`modules/core-modules/overhang-classifier-default/src/lib.rs`, since this WASM guest module
intentionally does not depend on `slicer-core`'s native-only dependencies), bucketed through the
*same* `BAND_BOUNDARY_MULTIPLIERS` thresholds real overhang uses, and merged via
`max(overhang_quartile, curl_quartile)` before a single lookup through the existing
`overhang_1_4_speed`..`overhang_4_4_speed` keys. This is mathematically identical to upstream's
`min(curled_speed, extrusion_speed)`, since both draw from the same monotonic table (more severe
band ⇒ slower speed) — "more cautious of the two" is preserved exactly. No new curl-specific speed
config keys were added.

**Compute `curled_height` transiently, not as a persisted field.** Unlike `overhang_quartile`
(a genuine cross-module IR/WIT field, since it's produced by a PrePass stage and consumed by a
different Layer-tier module across the WASM guest boundary), curl estimation and its consumption are
both computed inside a single `FinalizationModule::run_finalization` call — the module already
receives `layers: &[LayerCollectionView]`, the entire committed layer set with full point geometry,
so it can walk layers in Z order, keep the previous layer's curled points as local state, and emit
`SetSpeedFactor` mutations directly. Nothing else in this codebase reads `curled_height` back out, so
adding a WIT field, an IR schema-version bump, and marshal fan-out across every `Point3WithWidth`
construction site would be speculative work against a need that doesn't exist.

The curl-height formula itself (`estimate_curled_up_height`, ported from
`SupportSpotsGenerator.cpp:199-236`) and the cross-layer proximity lookup are implemented directly in
`overhang-classifier-default`, extending it rather than adding a sibling `FinalizationModule` — one
module computing curl, merging it with `overhang_quartile`, and emitting a single `SetSpeedFactor`
mutation per entity sidesteps any question of how two different `FinalizationModule`s' mutations on
the same entity would resolve.

## Consequences

- **No new WIT surface, no IR schema-version bump.** `curled_height` never crosses the WASM
  guest/host boundary as data; it exists only as a `Vec<(f32, f32, f32)>` local to one function call.
- **No new config keys.** Curl slowdown is tuned entirely through the existing overhang speed keys —
  a user cannot configure curl-avoidance speed independently of overhang-avoidance speed. If that
  independent control is ever needed, it is new scope, not a bug in this decision.
- **Curvature is a standard discrete estimate, not a verbatim OrcaSlicer port.** Upstream's own
  curvature/distance annotation (`estimate_points_properties`) lives outside the ~150 live lines this
  port scoped and has its own `AABBTreeLines` infrastructure. `discrete_curvature` (angle-over-arc-length)
  is functionally equivalent for `estimate_curled_up_height`'s convex-turn bonus term but is not a
  byte-identical port.
- **Cross-layer lookup is a linear/bounding-box scan, not an AABB tree.** Reasonable at this
  codebase's per-layer wall-vertex counts; revisit if profiling ever shows this module as a hot spot.
- **Layer 0 always contributes `curled_height = 0.0` reference points** (never estimated, since there
  is no layer below to compare against) rather than being omitted entirely — this lets layer 1 still
  have geometry to measure distance against, while matching this codebase's "no previous layer ⇒ no
  signal" precedent already used for `overhang_quartile` at layer 0.

## Alternatives considered

- **Separate curl-specific speed config keys**, decoupling curl tuning from overhang tuning. Rejected:
  contradicts upstream's own explicit design intent (one shared table), and no user need for
  independent tuning has been identified.
- **Persist `curled_height` as a `Point3WithWidth` sibling field**, mirroring `overhang_quartile`'s
  construction exactly. Rejected: `overhang_quartile` needs persistence because a PrePass stage
  produces it and a different Layer-tier WASM module consumes it — a genuine cross-boundary need.
  Curl estimation and consumption happen in the same function call of the same module; there is no
  boundary to cross, so persistence would be unused surface area.
- **A new sibling `FinalizationModule`** for curl, separate from `overhang-classifier-default`.
  Rejected: would require deciding module ordering and reconciling two `SetSpeedFactor` mutations on
  the same entity from two different modules — pure added complexity with no corresponding benefit,
  since both features already need the same `layers` slice and produce the same mutation type.

## Cross-references

- `docs/DEVIATION_LOG.md` DEV-009 — the sub-item this ADR closes; also documents the correction of the
  false "blocked on unbuilt support-spot generator" premise.
- ADR-0031 (overhang classification at PrePass) — the `overhang_quartile` field and speed table this
  ADR's curl work reuses.
- `CONTEXT.md` — **overhang quartile**, **curled height**, **artificial curl distance** vocabulary.
