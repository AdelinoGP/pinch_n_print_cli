# Implementation Plan: 139_lightning-layer-generator

## Execution Rules

- One atomic step at a time.
- Each step must map back to the packet's grouped task IDs.
- TDD first, then implementation, then the narrowest falsifying validation.
- Every field below is a context-budget contract and must be filled independently; never write "see Step 1".
- All `cargo check`, `cargo clippy`, and `cargo test` invocations must use `--all-targets`
  where applicable so the test, bench, and example targets compile.

## Steps

### Step 1: Overhang pass — `generate_initial_internal_overhangs` (RED→GREEN)

- Task IDs: `TASK-264`
- Objective: constants FACT first (dilation constant, per-layer move distance, density-
  coupled inputs from `FillLightning.cpp`); author the AC-1 two-layer synthetic test
  (RED); port the overhang pass into `generator.rs` (attribution header) to GREEN.
- Precondition: packet 138 closed (primitive APIs frozen).
- Postcondition: AC-1 green; `[FWD]` density-coupling recorded resolved.
- Files allowed to read: own lightning module + `FillLightning.cpp` (delegated SUMMARY).
- Files allowed to edit (at most 3):
  - `crates/slicer-core/src/algos/lightning/generator.rs` (new)
  - `crates/slicer-core/src/algos/lightning/mod.rs` (one `pub use` export line)
  - the lightning test home (decided in 138; add the generator test beside the
    primitive tests)
- Blast-radius discipline: none — both files are net-new; the 137 skeleton's
  `generate_lightning_trees` signature is unchanged at this step.
- Files explicitly out-of-bounds for this step: `OrcaSlicerDocumented/**` directly;
  `layer.rs` (Step 2).
- Expected sub-agent dispatches:
  - the constants FACT + the `Generator.cpp` overhang-pass section dispatch
  - "Run `cargo test -p slicer-core -- lightning_generator_overhangs …`; FACT"
- Context cost: `M`
- Authoritative docs: `docs/specs/lightning-infill-parity.md` §L3.
- OrcaSlicer refs: `OrcaSlicerDocumented/src/libslic3r/Fill/Lightning/Generator.cpp`
  (sectioned), `OrcaSlicerDocumented/src/libslic3r/Fill/FillLightning.cpp` (delegate).
- Verification:
  - AC-1 pipe command — FACT
- Exit condition: AC-1 green; `[FWD]` density-coupling recorded.

### Step 2: `Lightning::Layer` port — seeding, reconnect, convert

- Task IDs: `TASK-264`
- Objective: port `generateNewTrees`, `reconnectRoots`, `convertToLines` into `layer.rs`
  (attribution header), TDD'd on single-layer synthetics (seed inside overhang; roots
  reconnect to outline; conversion yields 2-point segments).
- Precondition: Step 1 exit condition.
- Postcondition: layer-level tests green.
- Files allowed to read: own lightning module.
- Files allowed to edit (at most 3):
  - `crates/slicer-core/src/algos/lightning/layer.rs` (new)
  - `crates/slicer-core/src/algos/lightning/mod.rs` (one `pub use` export line)
  - the lightning test home (add the layer tests)
- Blast-radius discipline: none — `layer.rs` is net-new; the `// 139 wiring point` is
  still in `mod.rs` until Step 4.
- Files explicitly out-of-bounds for this step: `OrcaSlicerDocumented/**` directly.
- Expected sub-agent dispatches:
  - the `Layer.cpp` section dispatches (≥ 4 sections)
  - "Run `cargo test -p slicer-core -- lightning_layer …`; FACT + counts; SNIPPETS
    ≤ 20 on failure"
- Context cost: `M`
- Authoritative docs: none new.
- OrcaSlicer refs: `OrcaSlicerDocumented/src/libslic3r/Fill/Lightning/Layer.cpp` —
  delegate, sectioned.
- Verification:
  - `cargo test -p slicer-core -- lightning 2>&1 | tee target/test-output.log | grep "^test result"` — FACT
- Exit condition: layer tests green. **Split tripwire:** if the port exceeds M
  mid-flight, STOP and split (convertToLines + producer wiring become the successor) —
  never rate L and continue.

### Step 3: `generate_trees` two-pass + continuity + determinism

- Task IDs: `TASK-264`
- Objective: port the two-pass `generate_trees` loop (top-down outlines pass, then
  top-down growth with `propagate_to_next_layer`); AC-2 continuity on the single-
  overhang prism; AC-4 determinism; AC-N1 no-overhang case.
- Precondition: Step 2 exit condition.
- Postcondition: AC-2, AC-4, AC-N1 green.
- Files allowed to read: own lightning module.
- Files allowed to edit (at most 3):
  - `crates/slicer-core/src/algos/lightning/generator.rs` (extend)
  - the lightning test home (add the two-pass / continuity / determinism tests)
- Blast-radius discipline: none — the 138 APIs are frozen; this step is in
  `generator.rs` only.
- Files explicitly out-of-bounds for this step: everything else.
- Expected sub-agent dispatches:
  - the `Generator.cpp` growth-pass section dispatch
  - "Run `cargo test -p slicer-core -- lightning_generator …`; FACT + counts"
- Context cost: `M`
- Authoritative docs: ADR-0029 (two-pass structure, delegate).
- OrcaSlicer refs: `OrcaSlicerDocumented/src/libslic3r/Fill/Lightning/Generator.cpp`
  growth-pass region — delegate.
- Verification:
  - AC-2, AC-4, AC-N1 pipe commands — FACT each
- Exit condition: generation semantics green.

### Step 4: Producer wiring + guards + gates

- Task IDs: `TASK-264`
- Objective: replace the 137 skeleton's `// 139 wiring point` body with the real
  driver — per object, construct the generator over the committed sparse outlines
  (inputs per the Step-1 FACT), store per-layer `convert_to_lines` output into
  `LightningTreeIR`; extend the 137 executor test with the commits-real-trees case
  (AC-3); run the wedge guard (AC-N2); workspace gates + guest freshness.
- Precondition: Step 3 exit condition.
- Postcondition: all packet ACs green.
- Files allowed to read, with ranges when over 300 lines:
  - the support-producer input-access LOCATIONS results (ranged)
- Files allowed to edit (at most 3):
  - `crates/slicer-core/src/algos/lightning/mod.rs` (replace skeleton body; delete
    `// 139 wiring point` comment)
  - `crates/slicer-runtime/tests/executor/lightning_prepass_tdd.rs` (extend with
    the commits-real-trees case)
  - (if the test home is the separate `algo_lightning_tdd.rs` file from 138, this step
    adds the AC-3 fixture inputs there; otherwise no third file edit)
- Blast-radius discipline:
  - **The `generate_lightning_trees` signature change in `mod.rs`** (now wires the
    real generator instead of returning empty IR) — verify the 137 builtin wrapper
    call site still compiles against the new signature (FACT before edit; adjust the
    wrapper call if the signature changes shape). If the wrapper needs to change,
    the wrapper edit is budgeted into this step (≤ 3-file edit cap).
  - Dispatch a `LOCATIONS` FACT for the `generate_lightning_trees` call sites
    (expected: 1 — the 137 builtin wrapper at
    `crates/slicer-runtime/src/builtins/lightning_tree_producer.rs`) before this step
    edits.
- Files explicitly out-of-bounds for this step: WIT/SDK/module files.
- Expected sub-agent dispatches:
  - "LOCATIONS ≤ 10: support-geometry producer whole-print input access"
  - "LOCATIONS ≤ 5: call sites of `generate_lightning_trees` (the 137 builtin wrapper)"
  - "Run `cargo test -p slicer-runtime --test executor -- lightning …`; FACT" — AC-3.
  - "Run `cargo test -p slicer-runtime --test e2e -- wedge …`; FACT" — AC-N2.
  - "Run `cargo clippy --workspace --all-targets -- -D warnings` + `cargo check
    --workspace --all-targets` + `cargo xtask build-guests --check`; FACT each"
- Context cost: `M`
- Authoritative docs: none new.
- OrcaSlicer refs: none.
- Verification:
  - AC-3, AC-N2 pipe commands + §Verification gates — FACT each
- Exit condition: all ACs green; gates green.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | M | overhang pass + constants |
| Step 2 | M | `Layer.cpp` port (448 lines; tripwire armed) |
| Step 3 | M | two-pass growth + determinism |
| Step 4 | M | wiring + guards + gates |

## Packet Completion Gate

- All steps and exits complete.
- Every pipe-suffixed AC command returns PASS.
- Update `docs/07_implementation_status.md` through a worker dispatch (TASK-264 flip),
  never a full backlog read.
- Reconcile reopened/superseded status transitions (138 API deviations, if any,
  recorded; 138 tests co-updated in the same step).
- `packet.spec.md` is ready for `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed AC and packet-level gate command.
- Confirm context stayed at or below 150k standard, or at/below 300k only with a logged
  swarm ESCALATION; otherwise record a packet-authoring lesson.
- Record any remaining packet-local risk explicitly before moving to `status: implemented`.
