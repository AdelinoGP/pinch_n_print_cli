# Implementation Plan: 139_lightning-layer-generator

## Execution Rules

- One atomic step at a time.
- Each step must map back to the packet's grouped task IDs.
- TDD first, then implementation, then the narrowest falsifying validation.
- Each step honors the context-discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. The fields below are not optional metadata — they are the budget contract for this step.

## Steps

### Step 1: Overhang pass — `generate_initial_internal_overhangs` (RED→GREEN)

- Task IDs:
  - `TASK-264`
- Objective: constants FACT first (dilation, move distance, density-coupled inputs); author
  the AC-1 two-layer synthetic test (RED); port the overhang pass into `generator.rs`
  (attribution header) to GREEN.
- Precondition: packet 138 closed.
- Postcondition: AC-1 green; `[FWD]` density-coupling recorded resolved.
- Files allowed to read: own lightning module.
- Files allowed to edit (≤ 3):
  - `crates/slicer-core/src/algos/lightning/generator.rs` (new)
  - `crates/slicer-core/src/algos/lightning/mod.rs` (export)
  - the lightning test file
- Files explicitly out-of-bounds for this step: `OrcaSlicerDocumented/**` directly;
  `layer.rs` (Step 2).
- Expected sub-agent dispatches:
  - the constants FACT + the Generator.cpp overhang-pass section dispatch
  - "Run `cargo test -p slicer-core -- lightning_generator_overhangs …`; FACT"
- Context cost: `M`
- Authoritative docs: `docs/specs/lightning-infill-parity.md` §L3.
- OrcaSlicer refs: Generator.cpp (sectioned), FillLightning.cpp:145 — delegate.
- Verification:
  - AC-1 pipe command — FACT
- Exit condition: AC-1 green.

### Step 2: `Lightning::Layer` port — seeding, reconnect, convert

- Task IDs:
  - `TASK-264`
- Objective: port `generateNewTrees`, `reconnectRoots`, `convertToLines` into `layer.rs`
  (attribution header), TDD'd on single-layer synthetics (seed inside overhang; roots
  reconnect to outline; conversion yields 2-point segments).
- Precondition: Step 1 exit condition.
- Postcondition: layer-level tests green.
- Files allowed to read: own lightning module.
- Files allowed to edit (≤ 3):
  - `crates/slicer-core/src/algos/lightning/layer.rs` (new)
  - `crates/slicer-core/src/algos/lightning/mod.rs` (export)
  - the lightning test file
- Files explicitly out-of-bounds for this step: `OrcaSlicerDocumented/**` directly.
- Expected sub-agent dispatches:
  - the Layer.cpp section dispatches (≥4 sections)
  - "Run `cargo test -p slicer-core -- lightning_layer …`; FACT + counts; SNIPPETS ≤20 on
    failure"
- Context cost: `M`
- Authoritative docs: none new.
- OrcaSlicer refs: Layer.cpp — delegate, sectioned.
- Verification:
  - `cargo test -p slicer-core -- lightning 2>&1 | tee target/test-output.log | grep "^test result"` — FACT
- Exit condition: layer tests green. Split tripwire: if the port exceeds M, stop and split
  (convertToLines + producer wiring become the successor) — never rate L and continue.

### Step 3: `generate_trees` two-pass + continuity + determinism

- Task IDs:
  - `TASK-264`
- Objective: port the two-pass `generateTrees` loop (top-down outlines pass, then top-down
  growth with `propagate_to_next_layer`); AC-2 continuity on the single-overhang prism;
  AC-4 determinism; AC-N1 no-overhang case.
- Precondition: Step 2 exit condition.
- Postcondition: AC-2, AC-4, AC-N1 green.
- Files allowed to read: own lightning module.
- Files allowed to edit (≤ 3):
  - `crates/slicer-core/src/algos/lightning/generator.rs`
  - the lightning test file
- Files explicitly out-of-bounds for this step: everything else.
- Expected sub-agent dispatches:
  - the Generator.cpp growth-pass section dispatch
  - "Run `cargo test -p slicer-core -- lightning_generator …`; FACT + counts"
- Context cost: `M`
- Authoritative docs: ADR-0029 (two-pass structure, delegate).
- OrcaSlicer refs: Generator.cpp:342 region — delegate.
- Verification:
  - AC-2, AC-4, AC-N1 pipe commands — FACT each
- Exit condition: generation semantics green.

### Step 4: Producer wiring + guards + gates

- Task IDs:
  - `TASK-264`
- Objective: replace the 137 skeleton body: per object, construct the generator over the
  committed sparse outlines (inputs per the Step-1 FACT), store per-layer
  `convert_to_lines` output into `LightningTreeIR`; AC-3 (producer commits == generator
  output); AC-N2 wedge byte-identity; workspace gates + guest freshness.
- Precondition: Step 3 exit condition.
- Postcondition: all packet ACs green.
- Files allowed to read (with line-range hints when > 300 lines):
  - the support-producer input-access LOCATIONS results (ranged)
- Files allowed to edit (≤ 3):
  - `crates/slicer-core/src/algos/lightning/mod.rs`
  - `crates/slicer-runtime/tests/executor/lightning_prepass_tdd.rs` (extend)
- Files explicitly out-of-bounds for this step: WIT/SDK/module files.
- Expected sub-agent dispatches:
  - "LOCATIONS ≤10: support-geometry producer whole-print input access"
  - "Run `cargo test -p slicer-runtime --test executor -- lightning …`; FACT"
  - "Run `cargo test -p slicer-runtime --test e2e -- wedge …`; FACT"
  - "Run `cargo clippy --workspace --all-targets -- -D warnings` + `cargo check --workspace
    --all-targets` + `cargo xtask build-guests --check`; FACT each"
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
| Step 2 | M | Layer port (tripwire armed) |
| Step 3 | M | two-pass growth + determinism |
| Step 4 | M | wiring + guards + gates |

## Packet Completion Gate

- All steps complete.
- Every step exit condition is met.
- Packet acceptance criteria green (each verification command dispatched and returned PASS).
- `docs/07_implementation_status.md` updated for TASK-264 (via worker dispatch — never edited
  by loading the full backlog into the implementer's context).
- Reopened or superseded packet status transitions reconciled (138 API deviations, if any,
  recorded).
- `packet.spec.md` ready to move to `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed acceptance criterion command from `packet.spec.md`.
- Confirm packet-level verification commands are green.
- Record any remaining packet-local risk explicitly before moving to `status: implemented`.
- Confirm the implementer's peak context usage stayed under 70%; if not, log it as a
  packet-authoring lesson for future spec-packet-generator runs.
