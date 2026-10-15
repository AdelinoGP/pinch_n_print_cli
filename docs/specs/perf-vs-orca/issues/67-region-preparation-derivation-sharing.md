# Region-preparation derivation sharing across region projections

Type: task
Label: wayfinder:task
Status: open
Assignee: unassigned — graduated by the 2026-10-14
[host:native attribution](60-host-native-attribution.md) take
([findings](../evidence/t60-host-native/FINDINGS.md)); human route decision pending
Parent: [Slice performance vs OrcaSlicer](../map.md)
Blocked by: none

## Question

Implement and measure sharing the per-layer region-preparation derivation
between the two `LayerArena` projections, so the matched jobs no longer compute
the same per-region data twice per layer.

The t60 take measured the two derivation slots — `ensure_prepared_regions`
(ordinary) and `ensure_prepared_perimeter_source_regions` (perimeter-source),
both `slicer-wasm-host/src/marshal/prepared.rs::prepare_regions` — at 661 clip
calls per benchy layer and 1,926 per base layer **each**, once per layer, for
**40.53 s + 40.18 s accumulated on base-off** (≈46 % of the then-`host:native`
bucket), 39.42 s + 40.06 s base-on, ≈1.48 s + 1.47 s benchy cells
([findings](../evidence/t60-host-native/FINDINGS.md), `captures/`).

On the matched jobs the two projections operate on **the same region sets with
the same inputs**: `marshal/mod.rs::perimeter_source_regions` returns
`slice.regions` minus modifier-namespace entries (restoring modifier children's
fields when present), and the matched configs author no modifier sub-regions —
so the two derivations are the same computation on identical input. The
duplication exists because the projections are general (they differ when
modifier sub-regions exist) and the two slots are cached separately, though
`take_slice`/`set_slice` invalidate both together.

Candidate shape: key the per-region derived data (`derive_needs_support`,
surface group, quartile-band clips, previous-layer boundary) by region
identity and share it across the two projections — deriving once and adapting
for the other projection when the region sets coincide, keeping the separate
derivation where they differ (modifier sub-regions). Exactness is testable:
the derivation is pure, so shared data must be value-identical to the current
per-projection results; the equality of the two projections' outputs on the
matched jobs is itself checkable before sharing.

Acceptance: independent exactness qualification (prepared data value-identity
across the two projections, including the modifier-sub-region case) plus the
standing paired ordinary + accelerated A/B on the pre-declared cells
(6 retained pairs per mode, zero starvation exclusions), keep/drop to the
human, no auto-commit.

## Notes

- The cache lives in `crates/slicer-runtime/src/blackboard.rs`
  (`LayerArena`; `take_slice` clears both slots, `set_slice` starts them
  unprepared); derivation call sites are `prepare_dispatch_region_views`
  (`crates/slicer-runtime/src/layer_executor.rs`).
- The two slots are consumed by different stage families (`Layer::Perimeters`
  uses the perimeter-source projection; `Layer::Infill`,
  `Layer::SlicePostProcess`, `Layer::Support`, `Layer::AnchoredEvents`,
  `Layer::SupportPostProcess` use ordinary regions), so a shared core must
  keep both projection views intact for the adapters.
- Do not conflate with the sibling named leads from the same take: the
  classic-perimeters host-service round trips (~89 s accumulated base-off) and
  the `region_partition` 3×-per-layer re-run (~10 s) are separate questions,
  recorded in the t60 findings and the map's fog; they are not this ticket.
- Evidence: [findings](../evidence/t60-host-native/FINDINGS.md), owner table
  and per-scope reconciliation in `summary.json` / `captures/`, re-verifiable
  with `verify.py` (exit 0 from the committed set).
