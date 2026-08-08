# ADR-0053 — Overhang speed becomes continuous emission-time speed sections, carried by a prepass-stamped `overhang_distance_mm`

<!-- filename: 0053-overhang-emission-time-speed-sections -->

## Status

Accepted (2026-07-25). **Records a maintainer ruling**, not a proposal:
`docs/spec_packets/_OLD/190-smoothed-overhang-speed.md` §Open Questions put three ways
forward to the maintainer — (A) conform, (B) supersede, (C) prepass carrier —
and the maintainer selected **option (C)**. This ADR is the amendment that
ruling requires.

Scope covers packets 190 *and* 191. `docs/spec_packets/_OLD/191-overhang-add-intersections.md`
carries an explicit §Option-(C) Contingency warning that an ADR scoped to 190
alone will not cover 191's geometry mutation; both are therefore in scope here.

## Context

`overhang-classifier-default` is a `PostPass::LayerFinalization` guest that reads
`Point3WithWidth.overhang_quartile` — one of four discrete bands stamped by
`PrePass::OverhangAnnotation` — and applies one whole-entity
`EntityMutation::SetSpeedFactor`. Canonical does something materially different:
it builds a table of **speed sections** from the overlap percentages
`{90, 75, 50, 25, 13, 0}`, evaluates them against wall extrusion geometry at
G-code emission time, and interpolates continuously between bracketing pairs
rather than snapping to a band.

`crates/slicer-core/src/algos/overhang_annotation.rs`'s module doc-comment
records the gap as an accepted deviation: canonical's six bands applied to wall
extrusion geometry at emission time, versus PnP's four bands evaluated at
pre-pass time against raw cross-section geometry. `docs/DEVIATION_LOG.md` uses
"six-band" three times and "four-band" zero times.

Restoring the canonical behaviour needs a *continuous* distance per point, not a
band index. Packet 190 as originally designed obtained it by re-adding
cross-layer wall-distance scanning inside the finalization module
(`distance_to_prev_boundary`) — which reverses three separately recorded
decisions and re-introduces the very wall-inset proxy ADR-0031 moved
classification away from.

`[BLOCK-3]` of packet 190 asked whether a finalization guest could instead read
the prepass's quartile polygons directly, which would have dissolved the
problem. **Measured, and it cannot** — re-verified against the tree while
authoring this ADR:

- `slice-region-view` is declared in `crates/slicer-schema/wit/deps/ir-types.wit`
  (package `slicer:ir-handles`), and every `use` of it is in
  `crates/slicer-schema/wit/deps/world-layer/world-layer.wit`.
- `world-finalization.wit` **never imports `slicer:ir-handles`**. Its only
  imports are `slicer:common/host-services` and `slicer:config/config-types`,
  and `run-finalization` takes
  `(layers: list<layer-collection-view>, output: finalization-output-builder, config: config-view)`.
- `layer-collection-view` exposes **exactly six** methods: `layer-index`, `z`,
  `entity-count`, `ordered-entities`, `tool-changes`, `z-hops`.
- `host-services` exposes **exactly fifteen** functions — `log`,
  `raycast-z-down`, `surface-normal-at`, `object-bounds`, `clip-polygons`,
  `offset-polygons`, `simplify-polygon`, `medial-axis`,
  `generate-arachne-walls`, `now-us`, and **five** `*-batch` forms
  (`offset-polygons-batch`, `clip-polygons-batch`, `simplify-polygon-batch`,
  `raycast-z-down-batch`, `surface-normal-at-batch`; the fifth arrived with
  [ADR-0049](./0049-batched-host-services-over-threaded-guests.md)). None is
  surface-, region- or quartile-related.

Reaching region data from finalization would therefore require a
`world-finalization` **world** change plus a rebuild of every guest — not an
`[ir-access]` manifest entry, which gates DAG validation rather than what the
WIT hands the guest.

## Decision

**Option (C): a prepass carrier.**

Add a continuous `overhang_distance_mm` to `Point3WithWidth`, **beside** the
existing `overhang_quartile`, stamped by the same prepass that already stamps
the quartile. Consume it at emission time for continuous speed interpolation
across the restored speed sections.

Concretely:

1. **Classification stays upstream.** The distance is measured where the
   cross-section geometry already lives, in `PrePass::OverhangAnnotation`, and
   travels to the finalization module as per-point data. The module remains a
   consumer of upstream classification; it does **not** re-acquire wall-distance
   scanning. This is the half of
   [ADR-0031](./0031-overhang-classification-at-prepass.md) that matters, and
   option (C) preserves it — which is why it is the minimal supersession rather
   than the maximal one.
2. **Speed is resolved per point, continuously**, by interpolating between the
   two bracketing entries of the restored section table and emitting the result
   through `EntityMutation::SetPointSpeedFactors` — the factor-valued,
   side-table-carried channel established by
   [ADR-0052](./0052-per-point-speed-factor-contract.md). The whole-entity
   `SetSpeedFactor` is **replaced**, not supplemented.
3. **Packet 191 makes the module a geometry mutator.** It inserts intersection
   vertices so a section boundary can fall mid-segment, replacing `path.points`
   wholesale via `EntityMutation::SetPathPoints`. Under (C) an inserted vertex
   has no stamped value — nothing upstream stamped a point that did not exist —
   so the module **interpolates `overhang_distance_mm` between the segment
   endpoints**. That is a different algorithm with a different error profile
   from canonical, which genuinely re-measures each new point; it is an accepted
   divergence and belongs in a deviation row. Every synthetic point must carry
   an interpolated `overhang_distance_mm`; leaving it at the struct default
   silently zeroes the field the segmentation gate reads.

### What option (C) does NOT change

- **`annotate_overhangs`' four concentric quartile bands and
  `BAND_BOUNDARY_MULTIPLIERS` in `crates/slicer-core` stay untouched.** The
  `[0.5, 1.0, 1.5]` interior boundaries, the "band 4 is the rest of the region"
  rule, and the absent-key-means-no-overhang map semantics are all out of scope.
  `overhang_quartile` remains a live field with its existing consumers.
- **In scope, and what the ruling authorises, is restoring
  `{90, 75, 50, 25, 13, 0}` as emission-time speed sections.** That is the
  framing to use identically in 190 and 191. The restored table is six entries
  long: sort ascending by distance with ties broken higher-speed-first, then
  **flatten, do not de-duplicate** — canonical has no `std::unique` and no
  `erase`, and a dedup changes which pair the interpolation brackets.
- **`PrePass::OverhangAnnotation`'s position and inputs are unchanged.** It still
  runs strictly after `PrePass::Slice` and derives bands by diffing consecutive
  committed slices, per ADR-0031's in-body amendment.

### Consequent obligation

`overhang_annotation.rs`'s module doc-comment currently describes as an
intentional deviation something the tree will no longer do. It must be corrected
in the same packet that lands the restoration, or it becomes a false record.

## Amendments

This ADR **amends** three accepted ADRs. Each retired clause is quoted verbatim
below, with the source's own emphasis and with elisions marked.

### ADR-0031 — retired Decision clause

> `overhang-classifier-default` is **kept**, not retired: it shrinks to a pure finalization-tier consumer that reads `Point3WithWidth.overhang_quartile` (now populated upstream) and applies `EntityMutation::SetSpeedFactor`.

Retired in two respects. First, `EntityMutation::SetSpeedFactor` is removed from
the module under **every** option 190 offered, including (C); it is replaced by
`SetPointSpeedFactors`. Second, packet 191 makes the module a **geometry
mutator** that rewrites `path.points`, which "a pure finalization-tier consumer"
no longer describes.

What survives: the module is still kept rather than retired; it still reads
per-point data populated upstream; classification is still not re-acquired by
the module. The clause's *architecture* stands and only its two concrete
mechanisms are replaced — which is precisely the trade option (C) was chosen to
make.

**ADR-0031 already carries an in-body amendment**, headed
`### Amendment (overhang-after-Slice inversion)`, which moved
`PrePass::OverhangAnnotation` to run strictly after `PrePass::Slice`. That
amendment explicitly preserved the clause quoted above — it states that
"keeping `overhang-classifier-default` as a finalization consumer — stands
unchanged" — so it is not an escape hatch here. **This is ADR-0031's second
amendment.** Anyone reading ADR-0031 must read both.

Note also that ADR-0031's Context gives the reason classification moved off
walls: "Walls are merely an inset-by-`line_width/2` proxy for the true
cross-section." Option (C) honours that reason, where option (B) would have
required hand-compensating the bias the repo had already rejected outright.

### ADR-0032 — retired Consequences clause and merge rule

> - **No new config keys.** Curl slowdown is tuned entirely through the existing overhang speed keys —
>   a user cannot configure curl-avoidance speed independently of overhang-avoidance speed. If that
>   independent control is ever needed, it is new scope, not a bug in this decision.

Retired. Packet 190 adds a curl-specific control key
(`slowdown_for_curled_perimeters`). ADR-0032 itself named the reversal condition
— "If that independent control is ever needed, it is new scope" — and this is
that new scope, taken deliberately.

The Decision merge rule is likewise retired:

> Curl distance is synthesized into an **artificial curl distance** … bucketed through the
> *same* `BAND_BOUNDARY_MULTIPLIERS` thresholds real overhang uses, and merged via
> `max(overhang_quartile, curl_quartile)` before a single lookup through the existing
> `overhang_1_4_speed`..`overhang_4_4_speed` keys.

The `max(overhang_quartile, curl_quartile)` merge and `quartile_for_distance`
are deleted, replaced by canonical's `min(curled_speed, extrusion_speed)`
applied after the `original_speed` clamp.

**One thing ADR-0032 grounds that does not survive automatically.** Its
equivalence argument — that `max` of two quartiles is mathematically identical
to upstream's `min` of two speeds — rests on the shared table being *monotonic*
("more severe band ⇒ slower speed"). A continuous interpolation must
**re-establish** that monotonicity rather than inherit it. Do not carry the
equivalence claim forward unexamined.

### ADR-0008 — retired finalization-tier `set-speed-factor` decision

> - Emits `set-speed-factor` mutations through `FinalizationOutputBuilder`

Retired as to the mutation kind. The module still emits through
`FinalizationOutputBuilder`, still from a `FinalizationModule`, still with no
new stage — ADR-0008's placement decision stands, and ADR-0031's Alternatives
correctly said that decision "is unaffected by the classification-algorithm
change". What changes is that the mutation is now per-point rather than
whole-entity. ADR-0031's Cross-references note that ADR-0008's
speed-factor-application-at-finalization decision "stands" should be read with
that substitution.

ADR-0008's separate "No WIT contract change" consequence is addressed by
ADR-0052, not here.

## Consequences

- **Blast radius, re-derived at authoring time rather than quoted.**
  `rg -c 'dist_to_top_mm:' --glob '*.rs' crates modules xtask` — an exact proxy,
  since every exhaustive `Point3WithWidth` struct literal names that field once
  — returns **300 occurrences across 135 files**. That is what a new per-point
  field costs: an exhaustive struct-literal sweep plus a `point3-with-width` WIT
  record field, an `L`-sized change that must be split into `M` steps exactly as
  packet 189 splits its `LayerCollectionIR` sweep. **Do not quote this figure
  from here** — it is a ledger fact and rots; re-run the command. (Packet 190's
  §Open Questions asserts a materially higher figure than an earlier frozen
  "300 … across 136 files"; this measurement does not reproduce that claim, and
  the command above is the arbiter.)
- **This cost is a fourth packet ahead of 190.** The carrier lands before the
  consumer, as packet 189 lands before packet 190.
- **A finalization guest still cannot see region geometry**, and nothing here
  changes that. `[BLOCK-3]` is settled *against* the convenient answer: the
  measurement above is why the carrier has to be per-point data pushed from
  prepass rather than a query pulled at finalization. Do not re-probe it; if the
  answer is ever to change, the change is a `world-finalization` world revision
  plus a full guest rebuild.
- **`overhang_distance_mm` and `overhang_quartile` coexist as two
  representations of one measurement.** They must be stamped by the same pass
  from the same geometry so they cannot disagree. Two sources of truth for
  overhang severity is a real hazard; the mitigation is co-location of the
  stamping, not a runtime consistency check.
- **The interpolated-fields list for synthetic vertices grows.** Packet 191's
  §Code Change Surface names `x`, `y`, `z`, `width`, `flow_factor`,
  `overhang_quartile` and `dist_to_top_mm`. Under (C) it must also name
  `overhang_distance_mm`. A per-point field's cost is not only the sweep; it is
  that every point-synthesising site becomes a place the field can be silently
  defaulted.
- **Four pre-existing tests in
  `modules/core-modules/overhang-classifier-default/tests/basic_tdd.rs` assert
  the behaviour being changed** — two assert the `SetSpeedFactor` mutation, two
  cover the `max(overhang_q, curl_q)` merge. They must be migrated, not deleted.
  If one is genuinely obsolete rather than changed, it is recorded as
  deliberately retired in the progress paragraph, never silently dropped.
- **Amendment rows are owed.** The `D-<n>-ADR-<nnnn>-AMENDED` convention requires
  a row per amended ADR in `docs/DEVIATION_LOG.md`, quoting the contested
  clauses. The `D-` series has its own counter, separate from `DEV-###`;
  re-derive the next free number at the moment of writing
  (`rg -o '^\| D-[0-9]{3}' docs/DEVIATION_LOG.md | sort -u | tail -1`) and trust
  no `D-` number quoted in any packet or ADR, including this one.

## Alternatives considered

The three the maintainer chose between, recorded as offered:

- **(A) Conform** — drop continuous interpolation and keep the four-band snap.
  Rejected: packet 190 would then have no honest content and the queue row would
  need re-planning; the canonical parity gap stays open indefinitely.
- **(B) Supersede** — keep packet 190's original design, re-add
  `distance_to_prev_boundary` cross-layer wall-distance scanning inside the
  finalization module, and author one ADR reversing all three decisions.
  Rejected: it re-introduces the inset-by-half-a-line-width wall proxy ADR-0031
  explicitly moved away from, and then proposes to hand-compensate the resulting
  bias — compensating a proxy the repo already rejected. It also drags in a
  deviation row and two open `[FWD]` questions that option (C) dissolves
  entirely.
- **(C) Prepass carrier** — **selected.** It minimises the supersession rather
  than avoiding it. It keeps classification upstream and the module a
  consumer of upstream data, and it eliminates the bias story. It does **not**
  make packet 190 ADR-0031-conforming, and claiming so would be overstated:
  `SetSpeedFactor` is removed under every option, and applying `SetSpeedFactor`
  is named in ADR-0031's Decision text. (C) still needs this amendment — a
  narrower one, covering the `SetSpeedFactor` removal and ADR-0032, rather than
  the full three-decision reversal (B) required. Its cost is the per-point
  blast radius above and a divergence from canonical on inserted vertices
  (interpolated, not re-measured).

Also considered and rejected earlier in the analysis:

- **Reach `SliceRegionView` from finalization**, removing the need for any
  carrier. Rejected because it is structurally impossible on the current world —
  see the measurement in §Context.

## Cross-references

- ADR-0031 (overhang classification at PrePass) — amended a **second** time by
  this ADR; it already carries `### Amendment (overhang-after-Slice inversion)`
  in-body.
- ADR-0032 (curled-edge slowdown shares the overhang speed table) — its "No new
  config keys." consequence and its `max(overhang_quartile, curl_quartile)`
  merge are retired here.
- ADR-0008 (overhang as a `FinalizationModule`) — its `set-speed-factor`
  mutation-kind decision is retired; its placement decision stands.
- ADR-0052 (per-point speed-factor contract) — the channel this ADR's continuous
  speeds travel through, and the resolution of ADR-0008's "No WIT contract
  change" clause.
- ADR-0049 (batched host services) — source of the five `*-batch` functions in
  the `host-services` count above.
- `docs/spec_packets/_OLD/190-smoothed-overhang-speed.md` §Open Questions — where the three
  options were put to the maintainer; `[BLOCK-1]`, `[BLOCK-2]`, `[BLOCK-3]`.
- `docs/spec_packets/_OLD/191-overhang-add-intersections.md` §Option-(C) Contingency — the
  three places 191 changes shape under this ruling.
- `crates/slicer-core/src/algos/overhang_annotation.rs` — `annotate_overhangs`,
  `BAND_BOUNDARY_MULTIPLIERS`, and the doc-comment that must be corrected.
- `crates/slicer-schema/wit/deps/world-finalization/world-finalization.wit` —
  the world whose imports settle `[BLOCK-3]`.
- `docs/DEVIATION_LOG.md` — the parity item this work advances.
