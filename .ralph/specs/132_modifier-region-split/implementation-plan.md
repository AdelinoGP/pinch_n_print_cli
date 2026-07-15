# Implementation Plan: 132_modifier-region-split

## Execution Rules

- One atomic step at a time.
- Each step must map back to the packet's grouped task IDs.
- TDD first, then implementation, then the narrowest falsifying validation.
- Each step honors the context-discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. The fields below are not optional metadata — they are the budget contract for this step.

## Steps

### Step 1: Discovery — region plumbing + RegionKey derivation memo

- Task IDs:
  - `TASK-257`
- Objective: produce a bounded decision memo naming (a) the structs/maps that carry a region's
  identity + partitioned polygons from `region_partition.rs` to `Layer::Infill` /
  `Layer::InfillPostProcess` dispatch, (b) the sub-region `RegionKey`/`region_id` derivation
  (patterning paint's variant-id synthesis), (c) the modifier-mesh slicing site
  (prepass-cached vs partition-time lazy). Resolves all three `[FWD]` questions in
  `design.md`.
- Precondition: FORWARD-DEP packets 130 + 131 have reached `status: implemented` (both are
  `draft` at authoring time — this packet must not activate before they close); clean tree.
- Postcondition: memo (≤ 40 lines) appended to this packet's `design.md` §Open Questions
  answers; no code changed.
- Files allowed to read: none directly (pure-dispatch step).
- Files allowed to edit (≤ 3):
  - `.ralph/specs/132_modifier-region-split/design.md` (memo append only)
- Files explicitly out-of-bounds for this step: all source (delegated).
- Expected sub-agent dispatches:
  - the two discovery dispatches specified in `design.md` §Expected Sub-Agent Dispatches
  - "Where would per-layer modifier cross-sections be cheapest to compute (prepass cache vs
    partition-lazy)? Inspect the partition call site's available context; FACT ≤5 lines"
- Context cost: `S`
- Authoritative docs:
  - `docs/adr/0030-modifier-splits-fill-not-perimeters.md` (full), `docs/specs/modifier-region-infill.md` §M1
- OrcaSlicer refs: none.
- Verification:
  - memo present and answers all three `[FWD]`s — self-check, then a reviewer grep:
    `rg -c 'FWD-RESOLVED' .ralph/specs/132_modifier-region-split/design.md` (expect 3)
- Exit condition: three `FWD-RESOLVED` entries in the memo; approach section still valid (or
  amended as a recorded deviation).

### Step 2: RED — executor tests for split semantics

- Task IDs:
  - `TASK-257`
- Objective: author `modifier_region_split_tdd.rs` with the five executor tests
  (`modifier_split_partition_conservation`, `modifier_split_wall_source`,
  `modifier_split_no_subregion_walls`, `modifier_split_z_scoping`,
  `modifier_split_degenerate_no_split`) using programmatic object + modifier construction;
  RED against current behavior.
- Precondition: Step 1 memo complete.
- Postcondition: five tests compiled and RED (except possibly N2 if no-split is already the
  degenerate outcome); harness mod line added.
- Files allowed to read (with line-range hints when > 300 lines):
  - one neighboring executor test file (fixture idiom); `crates/slicer-model-io/src/loader.rs`
    lines 547-628 (ModifierVolume shape)
- Files allowed to edit (≤ 3):
  - `crates/slicer-runtime/tests/executor/modifier_region_split_tdd.rs` (new)
  - `crates/slicer-runtime/tests/executor/main.rs` (mod line)
- Files explicitly out-of-bounds for this step: production source.
- Expected sub-agent dispatches:
  - "Run `cargo test -p slicer-runtime --test executor -- modifier_split 2>&1 | tee
    target/test-output.log | grep -E '^test |^test result'`; FACT per-test pass/fail" — RED
    confirmation
- Context cost: `M`
- Authoritative docs: ADR-0030 (AC semantics).
- OrcaSlicer refs: none.
- Verification:
  - the dispatch above — FACT (expect RED on 1/2/3/5)
- Exit condition: suite compiles; RED state recorded.

### Step 3: GREEN — split + wall-source + no-walls plumbing

- Task IDs:
  - `TASK-257`
- Objective: implement the partition-time split per the Step-1 memo: modifier cross-section
  slicing, intersection with the four polygons, sub-region minting (deterministic ids), base
  remainder, `wall_source_region_id` modifier arm, perimeter-dispatch exclusion.
- Precondition: Step 2 RED state.
- Postcondition: executor tests 1/2/3/5 + N2 GREEN; workspace compiles.
- Files allowed to read (with line-range hints when > 300 lines):
  - `crates/slicer-runtime/src/region_partition.rs` (full — primary surface)
  - the memo's named plumbing sites (ranged)
- Files allowed to edit (≤ 3):
  - `crates/slicer-runtime/src/region_partition.rs`
  - the modifier-slicing site named by the memo
  - the 130 wall-source predicate site (modifier arm)
- Files explicitly out-of-bounds for this step:
  - `region_mapping.rs` (Step 4), test files (except reading failures)
- Expected sub-agent dispatches:
  - "Run `cargo test -p slicer-runtime --test executor -- modifier_split …`; FACT + counts;
    SNIPPETS ≤20 on failure" — iterate to green
  - "Run `cargo check --workspace --all-targets`; FACT or LOCATIONS ≤30"
- Context cost: `M`
- Authoritative docs: ADR-0030 Decision points 1-2.
- OrcaSlicer refs: none.
- Verification:
  - `cargo test -p slicer-runtime --test executor -- modifier_split 2>&1 | tee target/test-output.log | grep "^test result"` — FACT
- Exit condition: AC-1/2/3/5/N2 commands green.

### Step 4: Config binding — geometric ModifierScope + AC-4 composition test

- Task IDs:
  - `TASK-257`
- Objective: extend `ModifierScope` beyond `AllFeatures`; `stamp_modifier_config_deltas`
  binds the modifier's delta to the sub-region `RegionKey`; add the AC-4 contract test (guest
  reads 0.40 in sub-region, 0.15 on base via the 131 accessor).
- Precondition: Step 3 exit condition.
- Postcondition: AC-4 green; 131's `per_region_config_*` and 130's
  `infill_postprocess_wall_source` still green (cross-step invariant).
- Files allowed to read (with line-range hints when > 300 lines):
  - `crates/slicer-core/src/algos/region_mapping.rs` — lines 260-320 + 600-640 only
  - the 131 contract test (idiom)
- Files allowed to edit (≤ 3):
  - `crates/slicer-core/src/algos/region_mapping.rs`
  - `crates/slicer-runtime/tests/contract/modifier_split_subregion_density_tdd.rs` (new) +
    harness mod line
- Files explicitly out-of-bounds for this step: everything else.
- Expected sub-agent dispatches:
  - "Run `cargo test -p slicer-runtime --test contract -- modifier_split_subregion_density
    …`; FACT"
  - "Run `cargo test -p slicer-runtime --test contract -- 'per_region_config|infill_postprocess_wall_source'
    …`; FACT" — invariant guard
- Context cost: `M`
- Authoritative docs: ADR-0030 Decision point 3.
- OrcaSlicer refs: none.
- Verification:
  - both dispatches above — FACT
- Exit condition: AC-4 green; no regression in 130/131 contract tests.

### Step 5: Byte-identity guard + Doc Impact + gates

- Task IDs:
  - `TASK-257`
- Objective: run the wedge SHA guard (AC-N1); land the `docs/02_ir_schemas.md` modifier
  sub-region subsection; run packet gates.
- Precondition: Step 4 exit condition.
- Postcondition: AC-N1 green; Doc Impact grep hits; gates green.
- Files allowed to read (with line-range hints when > 300 lines):
  - `docs/02_ir_schemas.md` — rg-located partition/SlicedRegion sections only
- Files allowed to edit (≤ 3):
  - `docs/02_ir_schemas.md`
- Files explicitly out-of-bounds for this step: code.
- Expected sub-agent dispatches:
  - "Run `cargo test -p slicer-runtime --test e2e -- wedge …`; FACT"
  - "Run `cargo clippy --workspace --all-targets -- -D warnings`; FACT"
  - "Run `rg -q 'modifier sub-region' docs/02_ir_schemas.md && echo HIT`; FACT"
- Context cost: `S`
- Authoritative docs: `docs/02_ir_schemas.md` (target).
- OrcaSlicer refs: none.
- Verification:
  - the three dispatches — FACT each
- Exit condition: AC-N1 + Doc Impact + gates green.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | S | pure-dispatch discovery memo |
| Step 2 | M | five executor tests |
| Step 3 | M | the split implementation |
| Step 4 | M | ModifierScope + composition test |
| Step 5 | S | guard + docs + gates |

## Packet Completion Gate

- All steps complete.
- Every step exit condition is met.
- Packet acceptance criteria green (each verification command dispatched and returned PASS).
- `docs/07_implementation_status.md` updated for TASK-257 (via worker dispatch — never edited
  by loading the full backlog into the implementer's context).
- Reopened or superseded packet status transitions reconciled (none expected).
- `packet.spec.md` ready to move to `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed acceptance criterion command from `packet.spec.md`.
- Confirm packet-level verification commands are green.
- Record any remaining packet-local risk explicitly before moving to `status: implemented`.
- Confirm the implementer's peak context usage stayed under 70%; if not, log it as a
  packet-authoring lesson for future spec-packet-generator runs.
