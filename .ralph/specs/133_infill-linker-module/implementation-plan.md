# Implementation Plan: 133_infill-linker-module

## Execution Rules

- One atomic step at a time.
- Each step must map back to the packet's grouped task IDs.
- TDD first, then implementation, then the narrowest falsifying validation.
- Each step honors the context-discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. The fields below are not optional metadata — they are the budget contract for this step.

## Steps

### Step 1: Module scaffold + claim + pass-through behavior

- Task IDs:
  - `TASK-258`
- Objective: create the module (manifest with `claim:infill-link` + `infill_overlap` schema,
  workspace member, `#[slicer_module]` entry) whose initial `run_infill_postprocess` reads
  `prior-infill` and re-emits it UNCHANGED (full re-emit pass-through); land the claim-catalog
  doc row; scheduler dedup test (AC-N3); `manifest_ingestion` count 20 → 21; ironing
  pass-through test (AC-8) — which passes already in pass-through mode and stays the canary
  for every later step.
- Precondition: packets 129–132 closed; clean tree.
- Postcondition: module builds as a guest, loads, runs at the stage, output == input;
  AC-8, AC-9, AC-N3 green; guests fresh.
- Files allowed to read (with line-range hints when > 300 lines):
  - `modules/core-modules/top-surface-ironing/` — scaffold structure only
  - `crates/slicer-scheduler/src/validation.rs` — lines 1-110
- Files allowed to edit (≤ 3 per wave):
  - Wave A: `modules/core-modules/infill-linker/**` (new tree), root `Cargo.toml`
  - Wave B: `docs/03_wit_and_manifest.md` (claim row), scheduler dedup test file,
    `manifest_ingestion` count assert
- Files explicitly out-of-bounds for this step:
  - `OrcaSlicerDocumented/**`; other modules' src
- Expected sub-agent dispatches:
  - "rg how `claim:ironing` is registered beyond its manifest (any code list?); LOCATIONS" —
    resolves design `[FWD]` 2
  - "Run `cargo xtask build-guests --check` (expect the new guest listed); FACT; rebuild if
    STALE"
  - "Run `cargo test -p infill-linker -- ironing_passthrough_identical …`; FACT"
- Context cost: `M`
- Authoritative docs: ADR-0025/0026 (full), `docs/03` claim table section.
- OrcaSlicer refs: none this step.
- Verification:
  - AC-8, AC-9, AC-N3 pipe commands — FACT each
- Exit condition: pass-through module live; three ACs green.

### Step 2: `ExPolygonWithOffset` port + overlap-sign verification + re-clip + short filter

- Task IDs:
  - `TASK-258`
- Objective: FIRST delegate the FillRectilinear.cpp:388-490 read and record the verified
  overlap direction in a design-memo comment + the AC-3 test constant; then port
  `ExPolygonWithOffset` (`offset.rs`, attribution header), wire re-clip
  (`clip_polylines` against the overlap boundary) and `remove_short_polylines`
  (0.8 × spacing, FillGyroid.cpp:356-359) into the pass-through pipeline.
- Precondition: Step 1 exit condition.
- Postcondition: AC-2, AC-3, AC-4 green; AC-8 still green (ironing untouched by clip/filter).
- Files allowed to read (with line-range hints when > 300 lines):
  - `modules/core-modules/infill-linker/src/**` (own module)
- Files allowed to edit (≤ 3):
  - `modules/core-modules/infill-linker/src/offset.rs` (new) + `src/lib.rs` wiring
  - `modules/core-modules/infill-linker/tests/infill_linker_tdd.rs`
- Files explicitly out-of-bounds for this step:
  - `OrcaSlicerDocumented/**` directly (delegated only)
- Expected sub-agent dispatches:
  - the MANDATORY sign-verification dispatch from `design.md` §Expected Sub-Agent Dispatches
  - "SNIPPETS of FillGyroid.cpp:356-359 (threshold semantics); ≤10 lines"
  - "Run `cargo test -p infill-linker …`; FACT + counts"
- Context cost: `M`
- Authoritative docs: `docs/08_coordinate_system.md` (delegate; ÷100 on all constants).
- OrcaSlicer refs: FillRectilinear.cpp:388-490, FillGyroid.cpp:356-359 — delegate; never
  load.
- Verification:
  - AC-2, AC-3, AC-4 pipe commands — FACT each
- Exit condition: offset structure ported with verified sign recorded; clip + filter live.

### Step 3: `BoundaryInfillGraph` port

- Task IDs:
  - `TASK-258`
- Objective: port the arc-length boundary parametrization (FillBase.cpp:1432-1544) into
  `graph.rs` (attribution header): boundary point projection, arc positions, walk-distance
  queries; unit tests on a square + square-with-hole boundary.
- Precondition: Step 2 exit condition.
- Postcondition: graph unit tests green (projection, arc distance, wrap-around walk).
- Files allowed to read: own module only.
- Files allowed to edit (≤ 3):
  - `modules/core-modules/infill-linker/src/graph.rs` (new)
  - `modules/core-modules/infill-linker/tests/infill_linker_tdd.rs`
- Files explicitly out-of-bounds for this step: `OrcaSlicerDocumented/**` directly.
- Expected sub-agent dispatches:
  - "SUMMARY then per-section SNIPPETS (≤30 lines) of FillBase.cpp:1432-1544" — port driver
  - "Run `cargo test -p infill-linker -- graph …`; FACT"
- Context cost: `M`
- Authoritative docs: none new.
- OrcaSlicer refs: FillBase.cpp:1432-1544 — delegate.
- Verification:
  - `cargo test -p infill-linker 2>&1 | tee target/test-output.log | grep "^test result"` — FACT
- Exit condition: graph primitives tested green.

### Step 4: `connect_infill` port (core)

- Task IDs:
  - `TASK-258`
- Objective: port the greedy endpoint connection via boundary walks
  (FillBase.cpp:1580-1818, minus the graph section already ported): candidate pairing, walk
  cost vs link threshold (constants ÷ 100), splice into polylines; AC-1 goes green here.
- Precondition: Step 3 exit condition.
- Postcondition: AC-1 green (raw segments → linked polylines on a square); AC-5 green
  (role/speed preserved); determinism test green (two runs identical output).
- Files allowed to read: own module only.
- Files allowed to edit (≤ 3):
  - `modules/core-modules/infill-linker/src/connect.rs` (new)
  - `modules/core-modules/infill-linker/src/lib.rs` (wiring)
  - `modules/core-modules/infill-linker/tests/infill_linker_tdd.rs`
- Files explicitly out-of-bounds for this step: `OrcaSlicerDocumented/**` directly.
- Expected sub-agent dispatches:
  - the sectioned connect_infill SUMMARY/SNIPPETS series (design §Expected Sub-Agent
    Dispatches) — one section at a time
  - "Run `cargo test -p infill-linker …`; FACT + counts; SNIPPETS ≤20 on failure"
- Context cost: `M`
- Authoritative docs: none new.
- OrcaSlicer refs: FillBase.cpp:1580-1818 — delegate, sectioned.
- Verification:
  - AC-1, AC-5 pipe commands — FACT each
- Exit condition: core connection green + deterministic. If this step's port exceeds M
  mid-flight, STOP and split the packet (graph+connect already landed stay; chain +
  orchestration become a successor packet) — do not rate it L and continue.

### Step 5: `chain_or_connect_infill` + wall-sharing-group orchestration

- Task IDs:
  - `TASK-258`
- Objective: port the nearest-neighbor ordering wrapper (FillBase.cpp:1820-2246) into
  `connect.rs`; implement `orchestrate.rs`: group regions by `wall_source_region_id`,
  apply the compatibility predicate, branch (a) union-then-link with majority-length bucket
  assignment, branch (b) per-region linking with un-offset wall-less shared arcs; per-role
  spacing via the 131 accessor.
- Precondition: Step 4 exit condition.
- Postcondition: AC-6, AC-7, AC-N1, AC-N2 green; AC-8 still green.
- Files allowed to read: own module; `crates/slicer-sdk/src/views.rs` (accessor surface,
  ranged).
- Files allowed to edit (≤ 3):
  - `modules/core-modules/infill-linker/src/orchestrate.rs` (new) + `src/connect.rs`
  - `modules/core-modules/infill-linker/tests/infill_linker_tdd.rs`
- Files explicitly out-of-bounds for this step: host dispatch code.
- Expected sub-agent dispatches:
  - "SUMMARY + SNIPPETS of FillBase.cpp:1820-2246"
  - "Run `cargo test -p infill-linker …`; FACT + counts"
- Context cost: `M`
- Authoritative docs: ADR-0025 §Amendment (the branches, verbatim source of truth).
- OrcaSlicer refs: FillBase.cpp:1820-2246 — delegate.
- Verification:
  - AC-6, AC-7, AC-N1, AC-N2 pipe commands — FACT each
- Exit condition: both branches + predicate green.

### Step 6: Pipeline smoke + Doc Impact + gates

- Task IDs:
  - `TASK-258`
- Objective: add the executor pipeline smoke test (AC-10); land the
  `docs/01_system_architecture.md` inventory/pipeline mention; run the packet gates; append
  any newly-affected golden tests to the 131 carve list (recorded deviation).
- Precondition: Step 5 exit condition.
- Postcondition: AC-10 green; Doc Impact greps hit; gates green; carve-list delta recorded
  (possibly empty).
- Files allowed to read (with line-range hints when > 300 lines):
  - one neighboring executor test (idiom); `docs/01_system_architecture.md` rg-located
    section
- Files allowed to edit (≤ 3):
  - `crates/slicer-runtime/tests/executor/infill_linker_pipeline_smoke_tdd.rs` (new) +
    harness mod line
  - `docs/01_system_architecture.md`
  - `.ralph/specs/131_per-region-config-delivery/carve-list.md` (append-only, if needed)
- Files explicitly out-of-bounds for this step: everything else.
- Expected sub-agent dispatches:
  - "Run `cargo test -p slicer-runtime --test executor -- infill_linker_pipeline_smoke …`;
    FACT"
  - "Run the affected-suite sweep (executor + e2e non-carved subset); FACT; list any newly
    red golden tests" — carve-delta discovery
  - "Run `cargo clippy -p infill-linker --all-targets -- -D warnings` + workspace check;
    FACT"
- Context cost: `M`
- Authoritative docs: `docs/01_system_architecture.md` (target).
- OrcaSlicer refs: none.
- Verification:
  - AC-10 + Doc Impact greps + §Verification gates — FACT each
- Exit condition: all packet ACs green; carve delta recorded.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | M | scaffold + claim + pass-through |
| Step 2 | M | offset port + sign verification + clip/filter |
| Step 3 | M | boundary graph port |
| Step 4 | M | connect_infill core (split-packet tripwire armed) |
| Step 5 | M | chain + orchestration branches |
| Step 6 | M | smoke + docs + gates + carve delta |

Aggregate M is justified by genuine packet complexity (the roadmap's core algorithm packet:
5 ported structures, 2 linking branches, 13 ACs); every heavy read is delegated and each step
has its own test seam. The Step-4 tripwire is the anti-L guard.

## Packet Completion Gate

- All steps complete.
- Every step exit condition is met.
- Packet acceptance criteria green (each verification command dispatched and returned PASS).
- `docs/07_implementation_status.md` updated for TASK-258 (via worker dispatch — never edited
  by loading the full backlog into the implementer's context).
- Reopened or superseded packet status transitions reconciled (none expected).
- `packet.spec.md` ready to move to `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed acceptance criterion command from `packet.spec.md`.
- Confirm packet-level verification commands are green.
- Record any remaining packet-local risk explicitly before moving to `status: implemented`.
- Confirm the implementer's peak context usage stayed under 70%; if not, log it as a
  packet-authoring lesson for future spec-packet-generator runs.
