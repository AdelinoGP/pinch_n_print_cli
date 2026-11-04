# ADR-0074 — Lightning ownership moves on recorded behavioural equivalence; the canonical-input gate keeps only strategy changes

## Status

Accepted (2026-10-20) for packet
`layer-module-preparation_08_lightning-migration`. It is the decision record for
the rule that the 2026-10-20 amendments in
[ADR-0066](./0066-private-layer-preparation-capability.md) and
[ADR-0029](./0029-lightning-prepass-tree-generator.md) put in place: those
amendments carry the clause-by-clause replacement in place, and this ADR records
the decision, the contested clauses and the evidence standard in one place so a
reviewer does not have to reconstruct them from two amendment sections.

It **supersedes** ADR-0029's host-ownership decision (its `## Decision` item 1
placement of tree generation in a host-side PrePass producer, and the
trade-off sentence that reserved modularisation for "a future packet") and it
**amends** ADR-0066's retirement precondition (its `## Consequences` sixth
bullet and the `ADR-0029` bullet under `## Relationship to existing
decisions`). It does **not** touch ADR-0029's Future-Reviewer Note, which
continues to govern the shape of preparation; see "What is not superseded"
below.

Registered deviations: `D-LMP08-ADR-0066-RETIREMENT-PRECONDITION` and
`D-LMP08-ADR-0029-HOST-OWNERSHIP` in `docs/DEVIATION_LOG.md`.

## Context and contested clauses

### ADR-0066 §Consequences, sixth bullet (amended)

> - Lightning migration requires a demonstrated canonical-correct PrePass input
>   strategy and portable module-owned preparation. When that gate and migration
>   acceptance pass, remove the old host producer, `LightningTreeIR` path, and
>   `lightning-tree-segments` accessor directly, with affected contract/binding
>   changes. There is no deprecation shim or approximate-geometry fallback.

### ADR-0066 §Relationship to existing decisions, `ADR-0029` bullet (amended)

> - **ADR-0029:** replace its host-owned lightning production decision when the
>   gated migration lands; retain the requirement for whole-print computation
>   before per-layer sampling. Until then its implementation remains present.

### ADR-0029 §Decision, item 1 (superseded in its placement clause)

> 1. **A new `PrePass::LightningTreeGen` stage** is added to `STAGE_ORDER`
>    (`crates/slicer-scheduler/src/execution_plan.rs`), executed by a **host-side
>    producer** in `crates/slicer-core/src/algos/lightning/` … It is skipped
>    entirely (zero cost) when no region's `sparse_fill_holder` is
>    `lightning-infill`.

and the matching trade-off under ADR-0029 §Consequences, "Trade-offs we
explicitly accept", second bullet (superseded in its closing sentence):

> - The generator could theoretically run as a WASM prepass module instead of a
>   host built-in. Rejected for v1: … A future packet may modularize it; the IR
>   contract is the stable seam.

### ADR-0029 §Future-Reviewer Notes, first bullet (NOT superseded)

> - **Do not move tree generation back into the per-layer module.** The
>   algorithm is irreducibly cross-layer (`generateTrees` iterates all layers
>   twice). A per-layer approximation is exactly the stub this ADR retires.

### Why the two bundled obligations had to be separated

ADR-0066's bullet and its ADR-0029 bullet each bundled **two different
obligations behind one gate**: relocating an existing planner into the consuming
artifact, and *improving* it to a canonical-correct strategy. Read literally,
the migration could not start until a new scientific result existed. In this
tree that conjunction is unsatisfiable, and the record is re-derivable rather
than asserted:

- **Gate A is recorded `fail`.** `docs/07_implementation_status.md` carries `TASK-576 Gate A outcome: fail`, and `docs/specs/layer-module-preparation-plan.md`'s `## Lightning geometry/portability gate outcomes` section records the same verdict with the per-case findings (cross-region seam-contributor multiplicity and a truncated boundary summary) and `migration_eligible: false`. Re-derive the outcome there; do not quote this ADR for it.
- Row 09's `feasibility-kernel` (`modules/core-modules/lightning-infill/feasibility-kernel/`)
  has empty `planner.rs`/`tree.rs` stubs, so Gate B never had a planner to run.

Neither gate is evidence about *ownership*. A relocation that reproduces the
existing output exactly is a different claim from one that replaces the
strategy, and only the second is what a canonical-input proof can establish.

## Decision

1. **Ownership move — authorized on recorded behavioural equivalence.** The
   existing planner relocates into the consuming artifact's whole-print
   `prepare_print` and the host producer, the public plan IR, the scheduler
   stage and the `lightning-tree-segments` accessor are deleted directly, with
   no deprecation shim and no approximate-geometry fallback.

   The evidence standard is a **recorded baseline of the live producer**,
   captured from the real pipeline and committed before any removal:
   per `(global_layer_index, object_id, region_id, ordered variant_chain)` the
   directed edge multiset and grounding/root witnesses, the final
   caller-attributed segment multiset, and the sha256 of both the plan and the
   exact inputs it was derived from. Native and fresh-WASM preparations must
   each reproduce it independently; edges compare as a multiset with
   multiplicity, identity includes the ordered chain, and a case with an empty
   population fails its own control.

   **Input-digest pinning is the discriminator.** The digest of the inputs the
   planner consumed is recorded separately from the plan digest, so a run whose
   inputs differ is reported as an input/plumbing divergence and says nothing
   about the algorithm. This is what replaces the canonical-correct-input gate:
   it does not certify that the inputs are canonical, it certifies that the
   move fed the same inputs and computed the same plan and segments.

2. **Strategy change — the gate keeps its full force.** Replacing this planner
   with one that is canonical-correct by construction still requires a
   demonstrated canonical-correct PrePass input strategy, and is still governed
   by the rows 07/09 gates. That obligation now lives inside the module rather
   than across a host/module boundary; nothing in this ADR weakens it.

3. **Whole-print placement is preserved, so the Future-Reviewer Note stands.**
   Preparation plans once, for the whole print, before any per-layer sampling:
   the relocated kernel still performs its cross-layer passes before a single
   `Layer::Infill` call. The move is therefore not the "per-layer
   approximation" ADR-0029's note retires, and the note needs no amendment.
   Splitting planning across per-layer invocations would be a strategy change
   under decision 2, not a consequence of this ADR.

4. **Equivalence is not parity.** A green equivalence run states "ownership
   changed, output did not". It preserves whatever the live producer does,
   including any existing deviation from OrcaSlicer, and must never be written
   up as a parity result. The recorded Gate A `fail` and the unexecuted Gate B
   are neither consumed nor improved by this decision.

## Consequences

**Positive.**

- Ownership becomes true: the artifact that consumes the plan also produces it,
  and the plan's lifetime and identity belong to that artifact.
- The retirement is falsifiable before it happens. Every deletion is gated on a
  comparison that can fail, so a silent behaviour change cannot ship disguised
  as a refactor.
- The two obligations stop blocking each other: strategy work can proceed (or
  not) on its own gate, and the freeze on the host producer ends.

**Negative.**

- The guarantee is bounded by the recorded cases. Equivalence is only as good as
  the baseline's population; a case the baseline does not cover is unverified,
  and the packet must not claim more than the recorded set.
- A baseline is a maintenance surface: a later strategy change invalidates it by
  construction and must re-record rather than reinterpret it.
- The retirement is wide — a public IR, a scheduler stage, a WIT package and its
  hardcoded verifier expectations, plus every test and doc that names them.

**Trade-offs we explicitly accept.**

- A relocation that cannot reproduce the baseline stops for re-approval instead
  of adjusting the baseline. "Fix the oracle" is not an available move.
- A recorded input-digest mismatch stops the step even when the output happens
  to match, because a matching output from different inputs is not evidence of
  a behaviour-preserving move.

## What is not superseded

- **ADR-0029's Future-Reviewer Note**, for the reason given in decision 3.
- **ADR-0029's "raw emit only, no module-side linking/clipping" note.** The
  module still emits; clipping and connection remain the infill linker's
  (ADR-0025/0026).
- **ADR-0066's atomicity, lifetime, accounting and no-shim clauses.** Only the
  retirement *precondition* changes.
- **ADR-0073.** Full-source variant-chain identity is unaffected: the specialist
  query package disappears because the query disappears, and identity continues
  to travel in the prepared plan entries.

## Amendment — 2026-10-25 (canonical fidelity pass)

The recorded-equivalence premise governed a **relocation**, not the current
kernel. Four fidelity reviews compared the port against OrcaSlicer and the
kernel was corrected where it was not canonical: the seeding model is now
top-down (erode the current fill surface, subtract the un-eroded layer above),
the planner consumes `stInternal`/`stInternalVoid` fill surfaces with holes as
sibling rings, grounding and reconnection follow `getBestGroundingLocation` /
`reconnectRoots`, and `convertToPolylines` chooses its continuation child from
the root-seeded `mt19937_64` draw.

Consequences:

- The pre-migration baseline in
  `crates/slicer-runtime/tests/fixtures/lightning_baseline/` no longer matches.
  `crates/slicer-runtime/tests/e2e/lightning_migration_tdd.rs` is red on the
  baseline-comparison tests; the baseline **must not** be regenerated to make
  them pass. Its recorder was retired with the host producer.
- The equivalence claim in this ADR describes the pre-fidelity kernel and is
  historical. It does not authorize reverting the canonical behaviour.
- Canonical parity remains the governing goal; Gate B's measured counts and
  remaining divergence are recorded in
  `docs/specs/layer-module-preparation-plan.md`.
- This amendment is a **strategy change**, not a relocation. The relocation
  decision itself (module ownership, no host fallback, no shim) is unchanged.

## Amendment — 2026-11-02 (baseline re-recorded from the module)

The 2026-10-25 amendment's operational prohibition is superseded: the committed
baseline **was** re-recorded from the module by
`crates/slicer-runtime/tests/e2e/lightning_migration_tdd.rs::record_lightning_baseline`,
which is committed and re-runnable.

- The fixture is now a **regression snapshot** of the current module's emitted
  `Layer::Infill` population, not evidence of equivalence to the pre-migration
  host producer. Its `producer`, `recorder` and `derivation` fields name that
  origin; the historical equivalence claim above still describes the
  pre-fidelity kernel and is unchanged.
- The migration's authorization is unchanged: module ownership, no host
  fallback, no shim (ADR-0066 / ADR-0029 as amended 2026-10-20). Canonical parity
  remains Gate B, still recorded `fail` in
  `docs/specs/layer-module-preparation-plan.md`; this re-record makes no
  canonical-parity claim.
- The baseline's case material keeps the recorder's historical 100x mesh scale;
  `derivation.mesh` states it so the artifact cannot be read as canonical-scale
  geometry.

## References

- [ADR-0066](./0066-private-layer-preparation-capability.md) — private
  preparation ownership; its `## Amendment — 2026-10-20` section is the in-place
  record this ADR governs.
- [ADR-0029](./0029-lightning-prepass-tree-generator.md) — host-owned producer;
  its `## Amendment — 2026-10-20` section is the in-place record, and its
  Future-Reviewer Notes remain in force.
- [ADR-0073](./0073-full-source-variant-chain-identity-and-versioned-specialist-queries.md)
  — full-source identity and versioned specialist query packages.
- [ADR-0025](./0025-infill-linker-as-raw-emit-post-pass.md) — raw emit and
  linker-owned clipping.
- `docs/specs/layer-module-preparation-plan.md` — queue rows 01–08, the row-08
  re-scope note and the Gate A/B records.
- `docs/spec_packets/layer-module-preparation_08_lightning-migration/` — the
  packet that implements this decision; its acceptance criteria are the
  equivalence and retirement witnesses described above.
- `docs/DEVIATION_LOG.md` — `D-LMP08-ADR-0066-RETIREMENT-PRECONDITION`,
  `D-LMP08-ADR-0029-HOST-OWNERSHIP`.
