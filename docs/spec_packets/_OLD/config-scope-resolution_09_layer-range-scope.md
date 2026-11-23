---
status: implemented
packet: config-scope-resolution_09_layer-range-scope
task_ids:
  - TASK-570
---

# config-scope-resolution_09_layer-range-scope

## Goal

Ingest OrcaSlicer's per-object `Metadata/layer_config_ranges.xml` into a first-class typed layer-range scope; compose `layer_height` ranges into a host-side per-object layer-height profile (canonical earlier-starting-wins trimming with gap fill) and derive each object's explicit top-Z schedule; carry that schedule across the `PrePass::LayerPlanning` WIT boundary so the guest layer planner emits variable-step layers; apply all other admitted range values through the shared scope resolver per layer in both production setup paths; and expose the scheduled Z sequence in the visual-debug manifest.

## Problem Statement

PnP currently has no source or typed representation for OrcaSlicer's per-object layer configuration ranges, so authored ranges cannot influence either layer-plan construction or per-layer region configuration. This packet closes the coherent vertical slice from canonical 3MF XML through registry-aware typed ingestion to packet-05's two resolver queries and the two runtime setup paths, while preserving packet-07 denial enforcement.

### Amendment 1 (scope expansion, owner-approved 2026-09-26)

Reconciliation (Step 1) proved the layer Z schedule is produced by guest `layer-planner-default` from scalar per-object heights through WIT `slicer:prepass-layer-planning@2.0.0` (`object-layer-config`), with no host breakpoint/profile API and no variable-step path anywhere. `layer_height` ranges therefore cannot affect the schedule — nor can AC-3 observe a profile — without a WIT change and a guest algorithm change. The owner approved expanding this packet in place:

- WIT `slicer:prepass-layer-planning` bumps `@2.0.0 → @3.0.0`; `object-layer-config` gains `layer-zs: list<f64>` (object-local top Zs, empty = uniform fallback).
- Host `slicer-config` gains `query_layer_height_profile` (literal `HeightProfileSegment` list) and `layer_top_zs` (explicit top-Z schedule), and `ResolutionTarget` gains `layer_top_z: Option<f64>`.
- The guest consumes explicit schedules with variable-step `effective_layer_height` and catch-up, preserving the existing uniform path when `layer-zs` is empty.
- Non-`layer_height` ranges are applied per layer in the runtime region-mapping builtin by re-resolving through `resolve_scope_stack` with `layer_top_z = layer.z` (canonical `GlobalLayer.z` is the layer top).
- `manifest.json` gains additive `scheduled_layer_zs` (schema 1.3) so the real visual-debug gate is observable.

No other WIT package, guest module, module manifest, or public IR serialization field changes. The `requirements.md` §Out of Scope line forbidding WIT/guest changes is superseded only for this packet's own surfaces.

### Approved plan-amendment note (attribution correction, not a divergence)

The source plan's Resolution paragraph says “later-starting range winning” for overlapping `layer_height` ranges. That parenthetical is inaccurate and is not implemented. Delegated canonical inspection of `layer_height_profile_from_ranges` (`Slicing.cpp`) established that its `std::map` iteration is ascending by range start and its trim step computes `lo = max(lo, last_pushed_high)`, so an earlier-starting range retains the overlap and the later range is clipped or removed; the fixed first-layer range is pushed first and likewise retains its interval. Uncovered gaps use the base layer height. This packet therefore implements **earlier-starting wins by trimming later ranges**, records the correction here without editing the approved plan or predecessor packets, and requires no deviation because it restores canonical parity. Conflicting overlaps for non-`layer_height` keys remain load errors under the approved owner policy.

## Architecture Constraints

- XML shape is exact: `objects/object@id/range@min_z,@max_z/option@opt_key`, with option value in text. `object@id` is a one-based object-list ordinal, not a 3MF object XML id.
- `RawLayerConfigRange` preserves strings and millimetre bounds; `map_layer_config_ranges` maps ordinals after model load. Registry-aware ingestion, not model IO, creates typed `ConfigValue`s and scope denials.
- `LayerConfigRange` is host-side resolution data. Do not add it to serialized IR or WIT. `LayerRangeInput` is the cross-crate transport value (object id + bounds + raw string values).
- Range membership is finite world-space Z millimetres, half-open `[min_z, max_z)`, evaluated against layer top print Z; this also defines catch-up inheritance.
- `layer_height` composition sorts by `(min_z, max_z, source_index)`, inserts the fixed first-layer interval first, retains earlier coverage, trims each later low to the last retained high, drops emptied segments, and fills gaps from the resolved object base height. It never overlays later values onto earlier overlap. The profile returns literal segments `(z_start, z_end, height)` and `layer_top_zs` evaluates the schedule `h0, h0+h1, …` until object height.
- Non-`layer_height` overlap validation groups by object/key and rejects only non-empty overlaps with unequal typed values. Equal values may overlap; selector/denied statements reject before any range is committed.
- `ResolutionTarget.layer_top_z: Option<f64>` is the only new resolver input; `None` means "no layer context" and range deltas never apply. A `Some(z)` matches ranges with `min_z <= z < max_z`.
- Guest `layer-zs` are object-local top Zs (raft offset applied by the guest); host `layer_top_zs` returns the same object-local sequence that `run.rs` sends. `layer-zs` empty preserves the uniform formula.
- Additive `ConfigScope::LayerRange` requires an inventory of all exhaustive matches and serialized/display spellings; `scope_denial_label` returns `"layer_range"`. No public IR/schema version bump because the type is host-only.
- WIT change is a breaking package bump to `@3.0.0`; guests must be rebuilt via `cargo xtask build-guests` before integration/visual-debug runs.
- The manifest addition is additive and version-gated (schema 1.3); older manifest schemas remain valid.

## Data and Contract Notes

- IR/manifest contracts: no MeshIR/SliceIR change. Manifest gains additive `scheduled_layer_zs` under schema 1.3; `layer_range` remains the existing packet-01/07 scope spelling; runtime config key strings stay snake_case.
- WIT boundary: `slicer:prepass-layer-planning@3.0.0`; `layer-zs` carries host-derived object-local top Zs only; raw XML/strings do not cross.
- Determinism/scheduler constraints: object ordinal mapping follows loaded object-list order; ranges normalize by `(min_z, max_z, source_index)`; range indices are assigned after normalization; option maps are ordered.
- Error boundary: XML syntax/shape errors originate in model IO; object mapping, registry typing, denied scope, invalid interval, and overlap conflicts become one model/config load failure before execution-plan/prepass mutation.
- Unit boundary: authored model Z values are millimetres (`f64`); guest schedules remain `f32`; host `layer_top_zs` is `f64` and compared with a 1e-9 tolerance in tests.

## Locked Assumptions and Invariants

- `ConfigScope::LayerRange { object_id, range_index }` is net-new in this packet; packet 03/05/07 have no layer-range variant, carrier, or accessor.
- Packet 05's reserved precedence slot is filled here; no second resolver is introduced.
- Packet 07's `admission_set` + `ResolutionError::ScopeDenied` are the only denial authority; range ingestion adds no hard-coded roster.
- Canonical XML uses one-based model-object ordinals and text-valued options.
- Canonical `layer_height` overlap is earlier-starting-wins through trimming, not later-starting replacement. This is an attribution correction, not a divergence.
- Gaps use resolved base `layer_height`; the fixed first-layer interval has precedence because it is retained first.
- Non-height conflicting overlap and selector/denied range statements are atomic load errors.
- World-Z interval membership is half-open on `LayerConfigRange::covers` — the layer top print Z (`GlobalLayer.z`, an `f32`) against each bound taken as the smaller of its authored `f64` value and its `f32` image, so the transported form of a top placed on a bound still resolves to that bound. This also defines catch-up inheritance.
- WIT `layer-zs` is object-local top Zs; empty means uniform fallback.

## Risks and Tradeoffs

- Adding an enum variant, a `ScopedConfig` field, and a `ResolutionTarget` field has a broad compile blast radius; inventories (Step 1) bound it and caller updates are split into ≤3-file substeps.
- WIT `@3.0.0` requires guest rebuilds; `cargo xtask build-guests --check` must pass before interpreting integration/visual-debug failures.
- Object ordinals can be confused with 3MF object XML ids. The fixture deliberately uses a model id that is not the ordinal and asserts mapping to close that regression.
- Floating endpoints can invite fuzzy/closed comparisons, and the layer top the host evaluates is an `f32` (`GlobalLayer.z`) while authored bounds are `f64`. `LayerConfigRange::covers` is therefore the single membership authority: it compares each bound as the smaller of the authored `f64` value and its `f32` image, which keeps the authored half-open rule in both representations instead of making `max_z` effectively inclusive. It is deliberately not a general epsilon or half-ULP shave — a power-of-two bound has unequal forward and backward `f32` gaps, so half the forward gap lands on a genuinely interior representable top and excludes it; the minimum-of-both rule has no such edge. A bound lying between two `f32` values is the documented limit: narrowing moves it by up to half a local `f32` ULP, so a top within that distance resolves to the bound rather than its authored side. Every ordinary print height is representable, so the band does not arise there. The resolver and the runtime kernel both call it, so they cannot disagree.
- A normalized profile test can become self-referential. Expected segments and resolved values are literal authored values, with a negative control that would expose later-wins replacement.
- Visual-debug currently has a separate config-source setup; AC-5/AC-6 prevent it from silently omitting model-authored ranges.
- `merge_different_heights`' first-layer credit uses a `last_z == 0.0` sentinel; explicit schedules keep the same convention because host top Zs are strictly positive.
