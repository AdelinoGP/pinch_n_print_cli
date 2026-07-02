# Design: 137_lightning-prepass-contract

## Controlling Code Paths

- Primary code path: `crates/slicer-scheduler/src/execution_plan.rs` (`STAGE_ORDER`) →
  `crates/slicer-core/src/algos/lightning/mod.rs` (producer skeleton) → runtime builtin
  wrapper + blackboard commit (pattern: the support-geometry producer wiring) →
  `crates/slicer-ir/src/slice_ir.rs` (`LightningTreeIR`) →
  `crates/slicer-schema/wit/deps/ir-types.wit` (+ world) read-view →
  `crates/slicer-sdk/src/views.rs` accessor.
- Neighboring tests or fixtures: scheduler stage-order tests; `crates/slicer-runtime/tests/
  {executor,contract}` (new skip/commit + roundtrip tests; a small test guest or an extension
  of an existing layer echo guest); wit drift suite.
- OrcaSlicer comparison surface: none in this packet (contract only; ADR-0029 records the
  cross-layer facts).

## Architecture Constraints

<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and, if `STALE:` is reported, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.
- ADR-0029 is binding: host-side producer (not a WASM prepass module); skip-when-unused;
  compact per-layer 2-point segment storage.
- CLAUDE.md §WIT/Type Changes Checklist governs the read-view addition.
- The 136-blessed golden baseline must survive untouched (AC-N1 wedge byte-identity).

## Code Change Surface

- Selected approach: clone the `SupportPlanIR` seam shape — IR struct + version constant +
  blackboard slot/commit + host producer registered at the new stage + a read-view guests
  reach from `Layer::Infill` dispatch context. The skip predicate reuses the fill-holder
  resolution (`sparse_fill_holder == lightning-infill` for ANY region of the print) computed
  once at producer entry.
- Exact changes: 1 stage entry + stage-order test; `LightningTreeIR` + constant + slot;
  `algos/lightning/mod.rs` skeleton (`generate_lightning_trees(...) -> LightningTreeIR`
  returning empty trees for now, with the 139 wiring point marked); WIT view + SDK accessor +
  macros glue; drift-test rows; docs sections.
- Rejected alternatives: (a) WASM prepass module for generation — rejected by ADR-0029
  (recorded trade-off); (b) lazily generating trees at first `Layer::Infill` touch —
  rejected: hides a whole-print computation inside a per-layer dispatch, breaking
  layer-parallel-safety expectations; (c) storing full tree topology in the IR — rejected:
  per-layer 2-point segments are what the module needs (ADR-0029 memory note).

## Files in Scope (read + edit)

- `crates/slicer-scheduler/src/execution_plan.rs` — stage entry.
- `crates/slicer-ir/src/slice_ir.rs` — `LightningTreeIR` + constant.
- `crates/slicer-core/src/algos/lightning/mod.rs` (new) + the runtime builtin wrapper file
  (pattern the support-geometry wrapper's home).
- `crates/slicer-schema/wit/deps/ir-types.wit` (+ world file) — read-view.
- `crates/slicer-sdk/src/views.rs` + `crates/slicer-macros/src/lib.rs` — accessor plumbing.
- Tests: scheduler stage-order, `slicer-ir` IR shape, executor skip/commit, contract
  roundtrip, wit drift.
- Docs: `docs/02_ir_schemas.md`, `docs/03_wit_and_manifest.md`.

## Read-Only Context

- `crates/slicer-core/src/algos/support_geometry.rs` — lines 80-140 (producer pattern).
- `crates/slicer-ir/src/slice_ir.rs` — `SupportPlanIR` region (~1046) + version-constant
  block.
- How support modules read `SupportPlanIR` views — one LOCATIONS dispatch, then ranged reads
  of the named sites.

## Out-of-Bounds Files

- `OrcaSlicerDocumented/**` — not needed; never load.
- `modules/core-modules/lightning-infill/**` — packet 140's surface.
- `target/`, `Cargo.lock`, generated code — never load.

## Expected Sub-Agent Dispatches

- "How does a guest module read SupportPlanIR-derived views today (WIT resource + dispatch
  site)? LOCATIONS ≤10" — view-pattern anchor.
- "Run `cargo build --tests 2>&1 | tail -40`; FACT or LOCATIONS ≤30" — after WIT edit.
- "Run `cargo test -p slicer-runtime --test contract -- lightning_tree_view_roundtrip …`;
  FACT" — AC-4.
- "Run `cargo xtask build-guests --check`; FACT; rebuild if STALE".
- "Run `cargo test -p slicer-runtime --test e2e -- wedge …`; FACT" — AC-N1.

## Data and Contract Notes

- IR: new top-level IR (minor ecosystem addition; `max_ir_schema` implications checked
  against the packet-91 precedent — delegate a FACT if a global schema ceiling exists).
- WIT: additive view; world version bump on the exposing world(s); full guest rebuild.
- Determinism: the IR's per-layer segment ordering is producer-defined and must be stable
  (Vec order, no hash containers) — 139's determinism test builds on this.

## Locked Assumptions and Invariants

- Producer skipped (no commit) when no lightning holder — the zero-cost promise (AC-3).
- `LightningTreeIR` stores per-layer 2-point integer segments, not topology (ADR-0029).
- The view exposes exactly the dispatching (object, layer)'s segments — no whole-print guest
  visibility.
- Non-lightning output byte-identical (AC-N1).

## Risks and Tradeoffs

- The "which world exposes the view" bump ripples like 130/131's — smaller surface, same
  ceremony; front-loaded knowledge from those packets applies.
- An empty-trees producer is temporarily misleading (lightning configured → no trees → module
  still uses its stub until 140): acceptable and explicit — the stub path is untouched until
  140, so behavior is unchanged for lightning users during 137-139.

## Context Cost Estimate

- Aggregate: `M`
- Largest single step: `M` (WIT view + plumbing)
- Highest-risk dispatch: the view-pattern LOCATIONS — must stay ≤10 entries.

## Open Questions

- `[FWD]` Whether a global `max_ir_schema` constant must be bumped for a NEW IR type (packet
  91 precedent) — resolve by FACT dispatch at the IR step; record either way.
