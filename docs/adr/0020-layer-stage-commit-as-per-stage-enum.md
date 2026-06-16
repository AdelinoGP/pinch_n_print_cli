# ADR-0020 — Layer-Stage Commit Is a Per-Stage Enum Applied by One Arena-Side `apply`

## Status

Accepted

## Context

ADR-0005 split the per-layer stage dispatch across a crate seam: WIT→IR
conversion happens in `slicer-wasm-host` (`deconstruct_layer_ctx`, inside the
`LayerStageRunner::run_stage` impl), and the resulting IR-typed value is applied
to the per-layer `LayerArena` in `slicer-runtime`. The value carried across the
seam was `slicer_ir::LayerStageCommitData` — a single wide struct with ~14
fields serving all 8 per-layer stages.

That struct is a **passive value-bag**, and three of its fields are not data:

- `needs_seam_injection: bool` — a control flag instructing the consumer to run
  a post-commit step the producer used to run itself.
- `after_entity_index: 0` on deferred g-code commands — a placeholder the
  producer cannot compute (it has no arena) and the consumer must override.
- `entity_order_proposal` — added late, after it was silently dropped.

The consumer (`execute_single_layer_inner`) had to replay an ordered, prose-only
protocol over this struct: extract proposal + flag before the move → apply the
entity-order proposal **before** commit → `commit_layer_outputs` → inject seam
**after** commit → override every placeholder anchor. The ordering existed only
as comments cross-referencing "the original `dispatch.rs::run_stage` ordering."

When this protocol was first cut across the seam (P83 Step 4d), **three of its
steps fell into the gap** and shipped as regressions: the post-commit seam
injection for `Layer::Perimeters`, the `set_entity_order` proposal, and the real
deferred-command anchor (placeholder `0` reached g-code). All three share one
root cause: **a data-only type can carry values but not a protocol**, and the
three lost steps are precisely the ones whose correct value cannot be computed
on the producer side (they need the committed arena, the assembled entity count,
or mutate staged arena entities). A flag, a lie-value, and silence — and silence
is what a move drops.

A further latent defect: the seam back-fill was written **twice** with different
code — `Layer::Perimeters` reconstructs `SeamPosition { point, wall_index }`
field-by-field inline, while `Layer::PerimetersPostProcess` does
`Some(entry.chosen_candidate.clone())`. They agree only because `SeamPosition`
has exactly those two fields today; a third field would silently drop on the
inline path.

## Decision

Replace `LayerStageCommitData` with a **flat per-stage enum**,
`slicer_ir::LayerStageCommit`, consumed by a single arena-side `apply`. The
`stage_id` string stops crossing the seam: the producer collapses it into a
variant once; the consumer's `apply` is one exhaustive `match`.

1. **Variants — one per stage, mirroring `slicer-schema::STAGES`** (the single
   source of truth per ADR-0006): `Perimeters`, `PerimetersPostProcess`,
   `Infill`, `InfillPostProcess`, `Support`, `SupportPostProcess`,
   `SlicePostProcess`, `PathOptimization`, plus `SeedLayerCollection`
   (documented test-only — see below). `run_stage` returns
   `Option<LayerStageCommit>`; `None` is the empty/`MissingComponent` case
   ("this invocation committed nothing" is the absence of a commit, not a kind
   of commit). Illegal `(stage, output)` pairings — e.g.
   `PerimetersPostProcess` carrying `InfillIR` — become unrepresentable, and the
   old `_ => {}` silent-ignore arm becomes a compile error.

2. **`apply` owns exactly one module invocation, including its pre/post hooks.**
   The ordered protocol stops being a global dance and becomes per-arm
   sequencing the type enforces: the `PathOptimization` arm does
   apply-order-proposal → compute-anchor-from-arena → accumulate; the
   `Perimeters` arm does replace → partition → seam-inject. The executor keeps
   the **end-of-layer** assembly/drain (assemble or take `LayerCollectionIR`,
   drain deferred queues, resolve travel-move `entity_id` against the final
   entity list) — that is cross-stage finalization, not a per-invocation commit.
   Signature: `apply(&mut LayerArena, commit, ctx: &StageApplyContext)`, where
   `StageApplyContext` is an extensible read-only borrow-struct — the
   output-side twin of ADR-0005's `LayerStageInput`.

3. **`apply` arms delegate to a small named arena-op vocabulary**
   (`replace_slot`, `merge_slot`, `mutate_slice`, `accumulate_deferred`) but are
   **not** collapsed into operation-keyed variants, because the perimeter
   post-hooks (fill partition, seam injection) are local to their arms.

4. **No lie-values.** The `PathOptimization` variant omits the anchor for the
   four end-of-layer command groups (z-hops, annotations, retracts, travel
   moves); `apply` stamps `anchor = ordered_entities.len()-1` from arena state.
   `tool_changes` keep their guest-provided `after_entity_index` (genuine domain
   behavior — they anchor per-command, the others "at end of layer"). The
   `needs_seam_injection` flag is deleted — it is implied by the `Perimeters`
   variant. The two divergent seam-injection sites collapse into one arm,
   closing the `SeamPosition` field-drift defect.

Dependency direction is unchanged (`slicer-runtime → slicer-wasm-host`); WIT→IR
conversion stays in `slicer-wasm-host`, arena application stays in
`slicer-runtime`. This **deepens** ADR-0005's symmetric IR-typed seam rather than
reversing it.

## Alternatives considered

- **Keep the wide `LayerStageCommitData` value-bag.** Rejected: it is the bug
  class. A passive struct of values + control flags + placeholder anchors forces
  a replayed protocol whose steps live in comments, and forgetting a step
  compiles and ships.

- **Family + `Phase` enum** (`Perimeter(PerimeterIR, Phase)`,
  `Infill(InfillIR, Phase)`, …, with `Phase ∈ {Main, PostProcess}`; 5 variants
  instead of 8). **Rejected structurally, not on taste:** "PostProcess" is a
  *different* arena operation per family — perimeter does replace + field-
  preserve merge + re-partition; infill does merge (Main) vs replace
  (PostProcess); support does set (Main) vs replace (PostProcess). There is no
  shared phase semantics to factor out. A `Phase` field would force an
  `if phase == PostProcess { … }` back inside each arm, re-creating the exact
  "forgot the post-commit branch" failure mode that produced the seam-injection
  bug.

- **Operation-keyed enum** (`ReplaceSlot`, `MergeSlot`, `MutateSlice`,
  `AccumulateDeferred`; 4 variants). Rejected: the stage→operation mapping is a
  new drift-prone table that does not mirror `STAGES`, and it loses the locality
  of the perimeter post-hooks, which ride only on the two perimeter stages.

- **Introduce an `Anchor { EndOfLayer, After(EntityRef) }` type now**
  (the standalone "model the unresolved anchor as a type" idea). Deferred: with
  the anchor omitted from the producer and stamped in `apply`, there is no
  lie-value left to type. `Anchor` earns its keep only if a future change lets
  guests place deferred commands after a *specific* entity — which the host
  currently forbids by overriding the guest index. Adding it now is speculative
  machinery for a capability that does not exist.

- **Remove the `SeedLayerCollection` test hatch from the production type**
  (have the one test that uses it seed via the `Infill` variant + real
  assembly, or via a test-only executor entry). A viable, more
  production-faithful path was identified, but deferred to a separate session.
  For now the hatch is kept as a **documented, mutually-exclusive variant** —
  named for its arena effect (`SeedLayerCollection`), not its caller — which
  strictly improves on the old struct field that could legally co-occur with
  real stage output.

## Consequences

- The three P83 Step 4d regressions (seam injection, entity-order proposal,
  anchor) become compile errors rather than silent runtime defects. The
  `SeamPosition` field-drift defect is closed by unification.
- Stage-keyed dispatch drops from **three** mirrored `match stage_id` sites
  (`deconstruct_layer_ctx`, `commit_layer_outputs`, `ir_path_for_layer_stage`)
  to **one** producer-side string→variant construction plus **one** compiler-
  checked exhaustive `apply`.
- Adding a 9th per-layer stage is a compile error until every match is updated —
  the stringly-typed drift that lost the bugs is structurally removed.
- The cost of per-stage (A): the four `*PostProcess` variants carry the same
  payload type as their `Main` sibling, which reads as redundant. This is
  accepted as honest — they perform genuinely different arena operations. If a
  single stage ever needs two operations chosen by module, a sub-mode field
  returns; this is judged unlikely because the merge-vs-replace split already
  maps onto the `Main`/`PostProcess` stage pair.
- A documented asymmetry remains and is recorded on `apply`'s doc-comment:
  anchors are stamped per-invocation (`count-1` at PathOptimization time) while
  travel-move `entity_id`s are resolved at layer end against the final entity
  list. This is inherent — the final list does not exist until all stages run.
- Compatible with the active perimeter-modules parity roadmap
  (`docs/specs/perimeter-modules-orca-parity-roadmap.md`): its IR widenings and
  new wall/role variants ride inside the `Perimeters`/`SliceIR` payloads and do
  not change the enum's shape; no new per-layer stage is introduced by that
  roadmap.

## Verification

- `slicer_ir::LayerStageCommit` is a non-`#[non_exhaustive]` enum whose
  non-test variants are exactly the 8 `STAGES` rows with `world_id ==
  "slicer:world-layer@1.0.0"`; a test asserts the variant set matches `STAGES`.
- `grep -rn 'needs_seam_injection\|after_entity_index: 0\|entity_order_proposal'
  crates/slicer-runtime/src crates/slicer-wasm-host/src` returns no matches in
  the commit path (the flag and the placeholder are gone).
- The seam back-fill exists at exactly one call site (one `apply` arm), not two.
- `cargo build --workspace --all-targets` and `cargo clippy --workspace
  --all-targets -- -D warnings` are clean; the `apply` match is exhaustive with
  no catch-all arm for the layer stages.
- Regression tests pin each recovered step: `apply(Perimeters(ir))` back-fills
  `resolved_seam`; `apply(PathOptimization { order_proposal: Some(..), .. })`
  permutes `ordered_entities`; `apply(PathOptimization)` z-hop stamps
  `after_entity_index == ordered_entities.len()-1`.
