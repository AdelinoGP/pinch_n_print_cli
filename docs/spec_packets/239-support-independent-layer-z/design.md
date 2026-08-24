# Design: 239-support-independent-layer-z

## Controlling Code Paths

- Primary code path: `crates/slicer-runtime/src/layer_executor.rs` (exact-Z routing) →
  `crates/slicer-runtime/src/pipeline.rs` (first production call of the anchored executor)
  → `crates/slicer-gcode/src/emit.rs` (measure-first `height_delta` protocol).
- Neighboring tests/fixtures:
  `crates/slicer-runtime/tests/integration/{anchored_event_ordering,anchored_event_accounting,anchored_parallel_determinism,anchored_z_span_validation,anchored_z_validation}.rs`
  (pre-existing substrate regression net; bare wrappers in
  `crates/slicer-runtime/tests/integration/main.rs`), new integration files mounted in the
  same aggregator, `crates/slicer-gcode/src/emit.rs` unit tests.
- OrcaSlicer comparison: see `requirements.md` §OrcaSlicer Reference Obligations; do not repeat delegation rules.

## Approach

Two verified blockers, one measured risk — each gets its own surface.

### 1. Exact-Z routing replaces the on-grid filter (`layer_executor.rs`)

Today `is_same_z_entity` returns true only when a same-z-support entity's declared planar Z
sits within `AnchoredGeometryContract::COORDINATE_TOLERANCE_UNITS` of its anchor layer's
`mm_to_units(anchor.z)`; `append_same_z_entities` consumes matches and the committed-event
filter (`!is_same_z_entity`) excludes them from anchored work. Net effect: an off-grid
plane matches neither route and vanishes. Replace with exact-Z semantics that partition,
not filter:

- **On-grid branch (unchanged behavior):** tolerance match ⇒ append into the anchor layer's
  ordinary `ordered_entities` exactly as today (invariant 6: same-Z support prints inside
  ordinary ordering).
- **Off-grid branch (new):** no tolerance match ⇒ entity stays in the anchored route and is
  emitted as anchored work at its declared plane. The routing predicate is total: every
  same-z-support entity lands on exactly one branch (AC-2); non-same-z-support entities are
  unaffected (they were never candidates for either branch).
- The predicate becomes one shared helper consulted by both consumers so the two filters can
  never disagree about an entity's route.

### 2. Production enablement of the anchored executor (`pipeline.rs`)

`run_pipeline_core` currently calls `execute_per_layer_with_events_and_support_tools` /
`execute_per_layer_with_instrumentation_and_support_tools`, which return only model layers;
anchored collections never enter the production stream. Switch both paths to the committed
variant (`execute_per_layer_with_committed_anchored_events`), threading the anchored entity
list the pipeline already owns post-prepass, then split `CommittedLayerEvent::Model` /
`CommittedLayerEvent::Anchored` results into the shapes downstream expects today:
model rows feed finalization/postpass as before; anchored collections synthesize support-only
print rows (a `LayerCollectionIR` per distinct declared plane, ordered by physical Z then
stable local id via `OrderedEventCollection::sort_deterministically` semantics) inserted at
their Z position among the model rows. Rows carry their own `z`; nothing divides by or
derives from row spacing (the G-09 prohibition stands).

The empty-collection case must be byte-equivalent with today's output ordering: when no
anchored collections exist, the committed stream is all `Model` events and the synthesized
row list equals the old layer list — this is AC-N1/N3's safety property and the rollback
shape if activation finds a blocker.

### 3. Measure-first `height_delta` protocol (`emit.rs`)

`height_delta` is derived per emitted row from neighbouring row Zs and feeds volumetric E
(`distance · point.width · height_delta · point.flow_factor / filament_area`). Whether an
off-grid support pass inherits a wrong height term is UNVERIFIED. Protocol:

1. **Measure (Step 5):** dispatch a worker to construct the minimal off-grid case through
   the real emitter and record, for an off-grid pass: the pass's row Z delta used by the
   code path, the pass's declared plane delta (its own Z minus previous extrusion Z), and
   the resulting E. Verdict rule — `MISSCALE_FIXED` required iff the height term applied to
   the off-grid pass differs from its declared plane delta by more than
   `1e-6` (absolute, f32-computed constants); otherwise `CONSISTENT`.
2. **Record:** verdict + measured numbers go under TASK-403 in
   `docs/07_implementation_status.md` before any decision.
3. **Conditional fix (Step 6):** on `MISSCALE_FIXED`, carry per-entity Z context into the E
   computation so an off-grid pass uses its declared plane delta; on `CONSISTENT`, make no
   emitter change and lock current behavior with the verdict test. Either way the AC-5 test
   exists and names the recorded branch; skipping the measurement is the falsifying exit.
   Whether and how canonical flow differs is determined ONLY by delegated inspection of
   `OrcaSlicerDocumented/src/libslic3r/GCode.cpp::_extrude` (cited by file + function,
   never line number) at implementation time — its returned shape feeds this protocol's fix
   branch; no canonical behavior is asserted here in advance.

## Plan Corrections

None. Every §12-brief claim re-verified live this session: `is_same_z_entity`'s tolerance
match (`layer_executor.rs`), both non-anchored pipeline call sites, the bare
`anchored_event_ordering` wrapper registration in `integration/main.rs`, and the `height_delta`/
E product lines in `emit.rs`. The stub's content is carried here verbatim where relevant
(G-02 blockers, disabled-reference note) and the stub file is deleted at authoring time.

## Architecture Constraints

- Invariants bound this packet (plan §6): 6 (same-Z support in ordinary ordering — AC-N1),
  8 (planar anchored output on declared Z), 9 (Z-spanning atomicity — AC-4), 12
  (serial/parallel determinism — AC-3), 13 (support-disabled emits nothing — AC-N3), 15
  (per-region attribution untouched — planner surface is out of bounds), 16 (non-zero
  matched tests on every command).
- Evidence standards: E1 (no vacuous assertions — every new test judges stream content, not
  artifact existence), E2 (human gate is inspection-only), E4 (freshness gate before
  slice-level evidence — AC-6), E7 (delegated Orca reads, file+function citations), E8
  (coordinate discipline below).
- <!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and inspect its exit code: exit 0 means fresh, non-zero means stale (a distinct exit code signals `wasm-tools` is unavailable). Never use `rg -q 'STALE:'` — a `wasm-tools`-missing infrastructure error prints no `STALE:` and would read as fresh. If stale, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.
- Rationale for inclusion: the change surface feeds guest execution indirectly —
  `layer_executor.rs` drives `dispatch_layer_call` against guest modules and the pipeline
  switch alters which executor entry point performs those dispatches. Host-side-only edits
  would normally skip the snippet; any evidence run that slices through guest modules does
  not.
- <!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.
- Rationale for inclusion: routing compares declared planar Z (canonical i64 units) against
  `mm_to_units(anchor.z)`; row synthesis and the conditional emitter fix handle mm floats
  (`LayerCollectionIR.z`, G-code coordinates). The existing fixture planes use unit-scale
  values (`z: 3000` = 0.3 mm).

## Code Change Surface

- Selected approach: exact-Z partition predicate + committed-event production enablement +
  support-only row synthesis + measure-first emitter protocol (above).
- Exact functions, traits, manifests, tests, and fixtures:
  - `is_same_z_entity` → replaced by a total route-partition helper (same crate, private);
    both consumers (`append_same_z_entities`, the committed-event exclusion filter) call it.
  - `execute_per_layer_with_anchored_events` /
    `execute_per_layer_with_committed_anchored_events` — consumed, signature unchanged.
  - `pipeline.rs`: swap the two per-layer call sites; add anchored-entity input plumbing and
    committed-stream splitting + support-only row synthesis.
  - `emit.rs`: NO edit unless Step 5's verdict is `MISSCALE_FIXED`; then the minimal
    height-term correction plus the AC-5 test.
  - New tests: ≥5 integration tests (AC-1..AC-4, AC-N1..AC-N3 minus the pre-existing
    `anchored_event_ordering`), 1 `slicer-gcode` lib test (AC-5).
  - Fixtures: reuse the in-file builders of `anchored_event_ordering.rs` (hand-built
    `ExecutionPlan` + `SliceIR` + anchored events; planes at unit scale); fixture-slice runs
    use the tracked `SupportTest.stl` + matched configs only for human-gate artifacts.
- Rejected alternatives and reasons:
  - Route everything (including on-grid) through anchored work: breaks invariant 6 and the
    pinned `anchored_event_ordering` expectations for zero benefit.
  - Snap off-grid planes to the nearest grid layer: destroys the feature being implemented
    (independent Z) and silently moves geometry.
  - Fix `height_delta` preemptively without measurement: contradicts the gap register's
    "stated, not measured" status and risks changing correct behavior (E1/T11 discipline).

## Files in Scope (read + edit)

Target at most 3 primary files; justify extras and consider splitting.

- `crates/slicer-runtime/src/layer_executor.rs` - role: exact-Z route partition; expected
  change: replace `is_same_z_entity` with the total predicate; keep both consumers aligned.
  Very long file — ranged reads around named symbols only.
- `crates/slicer-runtime/src/pipeline.rs` - role: production enablement; expected change:
  swap two call sites to the committed variant, thread anchored entities, synthesize
  support-only rows.
- `crates/slicer-gcode/src/emit.rs` - role: emission correctness; expected change: none on
  `CONSISTENT`; minimal height-term correction + test on `MISSCALE_FIXED`.

Test files added under existing aggregators (not "primary"): new integration test module +
`main.rs` mounts in `crates/slicer-runtime/tests/integration/`; one `#[cfg(test)]` block or
test module in `crates/slicer-gcode`. Task-ID registration touches
`docs/07_implementation_status.md` at closure only.

## Read-Only Context

Include ranges for files over 300 lines.

- `crates/slicer-runtime/src/layer_executor.rs` - lines around `is_same_z_entity`,
  `append_same_z_entities`, `execute_per_layer_with_committed_anchored_events`,
  `commit_ordered_event_collections` only - purpose: preserve exact current semantics while
  partitioning.
- `crates/slicer-runtime/src/blackboard.rs` - the `anchored_event_collections` arena-slot
  region only - purpose: confirm the apply seam needs no change.
- `crates/slicer-ir/src/slice_ir.rs` - `AnchoredEntity`, `AnchoredGeometryContract`,
  `OrderedEventCollection`, `StageOutput::AnchoredEvents` definitions only - purpose:
  contract shapes consumed unchanged.

## Out-of-Bounds Files

- `OrcaSlicerDocumented/...` - delegate; never load
- `target/`, `Cargo.lock`, generated code, vendored dependencies - never load
- Planner/renderer modules (`modules/core-modules/tree-support*`,
  `modules/core-modules/traditional-support*`) - packets 238b/238c territory; do not edit.
- Raft surfaces (`com.core.raft-default`, signed-index migration targets) - packet 240.
- AGG rasterization - packet 241.
- Unrelated crates - delegate symbol lookups; do not browse.

## Expected Sub-Agent Dispatches

- Question: measure the off-grid `height_delta` behavior end-to-end (Step 5 protocol);
  scope: `crates/slicer-gcode/src/emit.rs` + a throwaway harness under `target/`-excluded
  temp; return: `FACT` with the three measured numbers and the verdict string; purpose:
  gate Step 6.
- Question: LOCATIONS of every `is_same_z_entity` consumer and every
  `execute_per_layer*` caller (re-verify blockers live); scope: `crates/slicer-runtime`;
  return: `LOCATIONS` ≤20 entries; purpose: Steps 1–3 precondition.
- Question: SUMMARY of 238c's exports ledger (renderer flow outputs this packet must not
  disturb); scope: `docs/spec_packets/238c-support-renderer-flow-interfaces/packet.spec.md`;
  return: `SUMMARY` ≤200 words; purpose: interface check.
- Question: canonical `_extrude` flow-product shape; scope: per requirements snippet;
  return: `SNIPPETS` ≤30 lines; purpose: Step 6 fix shape (only if `MISSCALE_FIXED`).

## Data and Contract Notes

- IR/manifest contracts: none changed. `AnchoredEntity`, `OrderedEventCollection`,
  `CommittedLayerEvent`, `LayerCollectionIR` consumed as-is; no schema/version constant is
  bumped, so the version-locking constraint is dormant. If a conditional step must add a
  field (e.g. per-row provenance), that step owns the struct-literal blast radius (every
  literal site compiling against the struct) and the matching `docs/02_ir_schemas.md`
  section edit + grep in the same step.
- WIT boundary: untouched; no `.wit` edits, therefore no guest rebuild is forced by contract
  change — freshness gate still guards drift (E4/G-24: staleness presents as count
  divergence).
- Determinism/scheduler constraints: anchored interleaving order comes from the committed
  event stream (physical Z, then stable local id); parallel force must not reorder it
  (invariant 12, AC-3). Row synthesis sorts by `(z, local_id)` — the
  `sort_deterministically` keying — never by HashMap iteration.

## Locked Assumptions and Invariants

- On-tolerance same-z-support merging keeps today's exact behavior (AC-N1 pins it).
- Off-grid entities emit at their DECLARED planes; snapping is prohibited.
- No config keys, manifests, WIT, or IR shapes change in this packet.
- The `height_delta` emitter surface changes ONLY on the measured `MISSCALE_FIXED` verdict.
- Human gate requires the §9 enabled-feature references to exist under `tmp/`; they are
  never generated by this packet.

## Risks and Tradeoffs

- **Off-grid scheduling determinism** (highest): inserting intermediate rows between grid
  rows must be deterministic under rayon-forced parallelism; mitigated by deriving order
  exclusively from the committed event stream and pinning AC-3.
- **Invariant 15/16 interaction**: per-region plan attribution stays planner-owned; if row
  synthesis needed attribution data it doesn't have, that signals scope leakage — reject
  and revisit design rather than touching planner surfaces.
- **Emitter blast radius if fixed**: `height_delta` sits on the hot path of every move;
  a wrong correction mis-scales ALL extrusion, not just support. Mitigated by the
  assert-equal-within-1e-6 verdict test locking the chosen branch and by keeping the fix
  scoped to passes whose row provenance is anchored.
- **Empty-collection equivalence**: enabling the committed variant must not perturb
  support-free slices; covered by AC-N1/N3 plus existing suites (tree/traditional family,
  support-disabled) staying green.
- **Stale-guest masking** (T4): any count-divergence during evidence runs is attributed only
  after `cargo xtask build-guests --check` exits 0.

## Context Cost Estimate

- Aggregate: `M`
- Largest step: `M` (Step 4, production enablement + row synthesis)
- Highest-risk dispatch and required return format: the Step 5 measurement — `FACT` with
  three numbers + verdict string, else rejected and redispatched narrower.

## Open Questions

- [FWD] If the measured verdict is `CONSISTENT`, should the verdict test live permanently
  in `crates/slicer-gcode` as the regression lock (default assumption: yes, per AC-5)?
  Implementer-resolvable; does not block activation.
- [FWD] Exact placement of support-only rows relative to equal-Z model rows when a
  reference shows a different tie-break: implementer records the chosen deterministic
  tie-break (committed-stream order wins) and the matched-height delta at the human gate.
- [BLOCK] None at authoring time. The §9 reference dependency is a HUMAN-GATE blocker by
  design (gate cannot sign off without the files), not an activation blocker; 238c
  implementation is the dependency gate.
