# Design: 133_infill-linker-module

## Controlling Code Paths

- Primary code path: new `modules/core-modules/infill-linker/src/lib.rs` (module entry:
  `run_infill_postprocess(prior_infill, regions, config, output)`) with submodules
  `offset.rs` (`ExPolygonWithOffset`), `graph.rs` (`BoundaryInfillGraph`), `connect.rs`
  (`connect_infill` + `chain_or_connect_infill`), `orchestrate.rs` (wall-sharing groups, the
  two branches, bucket assignment, full re-emit).
- Host-side touch points (read-only for this packet — all shipped by 130/131):
  `prior-infill` input, six view fields, per-region config accessor.
- Neighboring tests or fixtures: `modules/core-modules/infill-linker/tests/` (module suite);
  `crates/slicer-runtime/tests/executor/` (pipeline smoke); `crates/slicer-scheduler` tests
  (claim dedup); `manifest_ingestion_tdd` count 20 → 21.
- OrcaSlicer comparison surface: see `requirements.md` §OrcaSlicer Reference Obligations
  (delegate; never load).

## Architecture Constraints

<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and, if `STALE:` is reported, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.
<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.
- ADR-0026 is binding: NO linking algorithm may land in `slicer-core` — the module owns
  `connect_infill` and friends end-to-end. `slicer-core` is consumed only for
  `clip_polylines`, `offset`, and geometry primitives.
- ADR-0025 §Amendment is binding: connection across wall-backed boundaries is invalid; the
  two branches and the compatibility predicate are locked design, not implementation choices.
- Full re-emit contract (ADR-0028 §Amendment): the module's output IS the next `InfillIR`;
  forgetting a bucket loses it — the ironing pass-through test (AC-8) is the canary.
- OrcaSlicer attribution header (docs/ORCASLICER_ATTRIBUTION.md) on every file containing
  ported code (`offset.rs`, `graph.rs`, `connect.rs`).

## Code Change Surface

- Selected approach: port shape mirrors Orca's layering — `ExPolygonWithOffset` provides the
  two boundaries (outer = wall-inset; inner = overlap boundary, sign VERIFIED from
  FillRectilinear.cpp:388-490 in Step 2 before any offset code); `BoundaryInfillGraph`
  parametrizes the inner boundary by arc length; `connect_infill` greedily joins segment
  endpoints whose boundary-walk distance is under the Orca link threshold; wall-less shared
  arcs are marked non-insettable in branch (b) by building the boundary from the region
  polygon with the shared arcs substituted un-offset (the union in branch (a) removes them
  entirely — which is why (a) needs no arc special-casing).
- Exact changes: the new module tree + manifest + workspace member; claim-catalog doc row;
  `manifest_ingestion_tdd` count; pipeline smoke test; scheduler dedup test.
- Rejected alternatives: (a) algorithms in `slicer-core::infill_ops` — rejected by ADR-0026
  (do not re-suggest); (b) endpoint-hopping cross-region logic instead of union-then-link —
  rejected in the grilling (D5.3): the union makes cross-region connection fall out of the
  ordinary boundary walk; (c) skipping branch (b) until modifiers ship — rejected: 132 lands
  first and provides real fixtures; deferring would strand AC-7.

## Files in Scope (read + edit)

- `modules/core-modules/infill-linker/**` (new: `Cargo.toml`, `infill-linker.toml`,
  `src/lib.rs`, `src/{offset,graph,connect,orchestrate}.rs`, `tests/infill_linker_tdd.rs`,
  `wit-guest/` shim per module convention) — the packet's home.
- Root `Cargo.toml` — workspace member entry.
- `crates/slicer-runtime/tests/executor/infill_linker_pipeline_smoke_tdd.rs` (new) + harness
  mod line; `crates/slicer-runtime/tests/contract/manifest_ingestion*` count assert.
- `crates/slicer-scheduler/` dedup test (pattern the existing non-fill dedup test file).
- `docs/03_wit_and_manifest.md` (claim row), `docs/01_system_architecture.md` (inventory).

## Read-Only Context

- `modules/core-modules/top-surface-ironing/` — scaffold idiom (small module with a non-fill
  claim) — structure only.
- `crates/slicer-sdk/src/views.rs` — the postprocess view surface (130/131 accessors) —
  ranged.
- `crates/slicer-sdk/src/builders.rs` — `InfillOutputBuilder` push methods — lines 21-141.
- `crates/slicer-scheduler/src/validation.rs` — lines 1-110 (claim dedup mechanism) — to
  pattern AC-N3, not to change.

## Out-of-Bounds Files

- `OrcaSlicerDocumented/**` — delegate section-by-section; never load.
- `modules/core-modules/{rectilinear,gyroid,lightning}-infill/**` — their output shapes are
  facts (2-point segments / clipped waves / 2-point branches), not reading material; delegate
  a FACT if a shape question arises.
- `crates/slicer-wasm-host/src/dispatch.rs` — shipped by 130/131; not this packet's surface.
- `target/`, `Cargo.lock`, generated code — never load.

## Expected Sub-Agent Dispatches

- "SUMMARY + SNIPPETS (≤30 lines) of `ExPolygonWithOffset` construction
  (FillRectilinear.cpp:388-490): what do aoffset1/aoffset2 mean, WHICH DIRECTION does the
  overlap offset go (expand vs inset), and how are contour/hole flags encoded?" — Step 2
  sign verification (MANDATORY before coding the offset).
- "SUMMARY of `connect_infill` (FillBase.cpp:1580-1818) in 4 sections: boundary
  parametrization, candidate pairing, walk cost, splice; then SNIPPETS per section on
  demand" — Step 3/4 port driver.
- "SUMMARY + SNIPPETS of `chain_or_connect_infill` (FillBase.cpp:1820-2246)" — Step 5.
- "Run `cargo test -p infill-linker 2>&1 | tee target/test-output.log | grep '^test
  result'`; FACT + counts; SNIPPETS ≤20 on failure" — every GREEN gate.
- "Run `cargo xtask build-guests --check`; FACT; rebuild if STALE" — after each guest edit
  wave (the new module must join the guest build set — verify it appears in the build list).

## Data and Contract Notes

- IR contracts: consumes `prior-infill` (130) read-only; emits via `InfillOutputBuilder`
  (`begin_region` per bucket — packet-127 origin propagation applies to the linker too).
- WIT boundary: none changed here; the module is a consumer.
- Determinism: connection order must be deterministic — sort candidate endpoints by
  (arc-position, segment index), never iterate a HashMap; two identical runs emit identical
  `InfillIR`.
- Spacing derivation: per (region, role) — `line_width / infill_density` for sparse,
  `line_width`-based for solid roles, read through the 131 accessor; the linker never guesses
  from path widths EXCEPT in the cross-region compatibility predicate (endpoint widths),
  which is deliberately path-observable (D5.2).

## Locked Assumptions and Invariants

- Linking is per (region, role) except inside wall-sharing groups; branches (a)/(b) and the
  predicate are locked (ADR-0025 §Amendment) — do not re-open cross-wall connection.
- Overlap is the linker's concern alone (ADR-0025 Future-Reviewer note): modules emit over
  the un-offset wall-inset polygon; the host partition stays overlap-free.
- Full re-emit: every input bucket appears in the output (transformed or passed through).
- `claim:infill-link` is non-fill: no `FILL_CLAIM_IDS` entry, no `ResolvedConfig` field (D4).
- Already-linked multi-point input (today's gyroid waves, lightning's 2-point trees) is
  handled by the same path: re-clip + chain. No module-identity detection exists or may be
  attempted (paths carry no module id — ADR-0025 Amendment point 2).

## Risks and Tradeoffs

- **The overlap sign.** If the Step-2 verification contradicts the spec's original
  "wall-inset minus overlap" phrasing (Orca EXPANDS fill toward the perimeters), the ported
  semantics win and the spec sentence is corrected as part of this packet — a recorded
  deviation, not a silent fix. AC-2/AC-3 are written sign-agnostic (containment against the
  ported boundary + hand-computed square case).
- **connect_infill port size** (~700 lines C++): mitigated by the graph/connect/chain step
  split, each with its own test seam. If Step 4 balloons, split the packet rather than rate a
  step L.
- **Output churn**: linking today's stub output changes gcode — expected; belongs to the 131
  carve window (any newly-affected test is appended to the carve list as a recorded
  deviation).
- **Branch (b) boundary construction** (un-offset shared arcs) is the subtlest geometry in
  the packet; AC-7's reach tolerance is the falsifier.

## Context Cost Estimate

- Aggregate: `M`
- Largest single step: `M` (Step 4, connect_infill core)
- Highest-risk dispatch: the connect_infill section SUMMARY/SNIPPETS series — must stay
  sectioned; a whole-function dump would blow the budget.

## Open Questions

- `[FWD]` Orca link-length threshold constant(s) used by `connect_infill` (e.g. max walk
  distance as a multiple of spacing) — extract during the Step-3/4 delegated reads; divide
  by 100.
- `[FWD]` Whether the scheduler's non-fill dedup needs a claim-id registration anywhere
  beyond the manifest + docs (rg `claim:ironing` handling at Step 1; if a code list exists,
  add `claim:infill-link` there and record it).
