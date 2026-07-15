# Implementation Plan: 161-visual-debug-agent-verification

## Execution Rules

- Work one atomic step at a time; map every step to `TASK-271`.
- Use contract tests first, then guidance, then deterministic and overhead verification.
- Every field below is an independent context-budget contract; do not infer missing packet exports.

## Steps

### Step 1: Confirm forward seams and reusable fixtures

- Task IDs: `TASK-271`
- Objective: inventory packet-157 lifecycle, packet-159 typed capture, and packet-160 final-renderer exports plus deterministic fixtures without implementing missing surfaces.
- Precondition: packets 159 and 160 exist as generated/draft prerequisites.
- Postcondition: `[FWD-157-1]`, `[FWD-159-1]`, and `[FWD-160-1]` have exact symbols or named activation blockers.
- Files allowed to read, with ranges when over 300 lines:
  - `.ralph/specs/159-visual-debug-intermediate-renderer/**` - contract artifacts only.
  - `.ralph/specs/160-visual-debug-gcode-renderer/**` - contract artifacts only.
  - Packet-157 implementation/test seam - exact locations returned by dispatch.
- Files allowed to edit (at most 3):
  - None; read-only discovery.
- Files explicitly out of bounds:
  - Renderer/parser/capture implementation, CLI contract, WIT/IR/schema, WASM, coordinate helpers, Orca sources, and ordinary slice production code.
- Expected sub-agent dispatches:
  - Question: identify the three exact published integration/test seams; scope: named packet artifacts and bounded implementation locations; return: `LOCATIONS` at most 20 entries.
- Context cost: `S`
- Authoritative docs:
  - `docs/specs/visual-pipeline-debug.md` - lines 112-163, 180-221.
  - `docs/19_visual_debug.md` - lines 18-50.
  - `docs/17_agent_debugging.md` - lines 7-19.
  - `docs/adr/0038-visual-debug-skill-pairs-with-debug-pipeline.md` - complete decision.
- Verification:
  - Bounded seam inventory - `LOCATIONS` or explicit `[BLOCK]`.
- Exit condition: exact seams are recorded, or activation remains blocked with no guessed API.

### Step 2: Add agent skill and guide examples

- Task IDs: `TASK-271`
- Objective: document independent visual-debug source selection, request examples, manifest-first inspection, warnings, scale/cost guidance, failure behavior, and debug-pipeline cross-links.
- Precondition: Step 1 confirms the command examples are packet-157 compatible; unresolved renderer exports do not block prose-only guidance.
- Postcondition: skill and examples satisfy AC-1, AC-2, and AC-N1 without claiming renderer ownership, Orca parity, WASM behavior, or coordinate changes.
- Files allowed to read, with ranges when over 300 lines:
  - `docs/19_visual_debug.md` - lines 9-58.
  - `docs/17_agent_debugging.md` - lines 7-19, 21-55, 103-132.
  - `docs/adr/0038-visual-debug-skill-pairs-with-debug-pipeline.md` - complete.
- Files allowed to edit (at most 3):
  - `.claude/skills/visual-debug/SKILL.md`
  - `.claude/skills/visual-debug/examples/model-backed.md`
  - `.claude/skills/visual-debug/examples/standalone-gcode.md`
- Files explicitly out of bounds:
  - All renderer/parser/capture/CLI implementation, `docs/19_visual_debug.md`, `docs/17_agent_debugging.md`, WIT/IR/schema, WASM, coordinate helpers, Orca sources, and ordinary slice paths.
- Expected sub-agent dispatches:
  - Question: validate the two examples against the documented request and evidence boundary; scope: the two new example files and listed docs; return: `FACT` in 5 lines or fewer.
- Context cost: `S`
- Authoritative docs:
  - `docs/specs/visual-pipeline-debug.md` - lines 20-35, 61-110, 215-221.
  - `docs/19_visual_debug.md` - complete.
  - `docs/17_agent_debugging.md` - lines 7-19, 21-55, 103-132.
- Verification:
  - `python3 -c "from pathlib import Path; ..."` - FACT pass/fail for commands, manifest-first guidance, and cross-links.
- Exit condition: both source-mode examples and the negative routing guidance are present and exact.

### Step 3: Add contract and deterministic evidence tests

- Task IDs: `TASK-271`
- Objective: pin all documented intermediate tap fields in the runtime seam, pin the packet-160 final-renderer manifest at its owning `pnp-cli` seam, then compare complete model and standalone-G-code bundles byte-for-byte.
- Precondition: Step 1's forward seams are confirmed; otherwise record a named failing `[FWD]` test and do not substitute implementation.
- Postcondition: AC-3, AC-4, AC-5, and AC-N2 have focused tests covering exact fields, both source modes, deterministic ordering/bytes, and invalid-request failure.
- Files allowed to read, with ranges when over 300 lines:
  - Exact packet-159/160/157 seam files returned by Step 1.
  - Existing visual-debug test fixtures in the two named test directories only.
  - `docs/specs/visual-pipeline-debug.md` - lines 180-213.
- Files allowed to edit (at most 3):
   - `crates/slicer-runtime/tests/visual_debug_agent_contract_tdd.rs`
   - `crates/pnp-cli/tests/visual_debug_gcode_renderer_tdd.rs`
   - `crates/pnp-cli/tests/visual_debug_agent_determinism_tdd.rs`
  - Existing test fixture helper only if the confirmed seam requires a minimal constructor.
- Files explicitly out of bounds:
  - Any renderer/parser/capture/CLI production file, WIT/IR/schema, modules, WASM, coordinate code, Orca sources, and ordinary slice production path.
- Expected sub-agent dispatches:
  - Question: identify the smallest real fixtures and exact field accessors for every documented tap and both source modes; scope: confirmed seams and existing tests; return: `SNIPPETS` at most 3 snippets, 30 lines each.
- Context cost: `M`
- Authoritative docs:
  - `docs/specs/visual-pipeline-debug.md` - lines 112-141 and 180-213.
  - `docs/11_operational_governance_and_acceptance_gate.md` - lines 86-117 and 167-179.
  - `docs/01_system_architecture.md` - lines 621-665.
- Verification:
   - `cargo test -p slicer-runtime --all-targets --test visual_debug_agent_contract_tdd -- intermediate_tap_manifest_contracts --exact 2>&1 | tee target/test-output.log` - FACT from the log.
   - `cargo test -p pnp-cli --all-targets --test visual_debug_gcode_renderer_tdd -- final_gcode_manifest_contracts --exact 2>&1 | tee target/test-output.log` - FACT from the log.
   - `cargo test -p pnp-cli --all-targets --test visual_debug_agent_determinism_tdd -- visual_debug_bundles_are_byte_deterministic --exact 2>&1 | tee target/test-output.log` - FACT from the log.
   - Exit condition: tests assert exact fields, complete metadata, byte identity, ordering, and invalid-request failure rather than merely PNG existence.

### Step 4: Prove ordinary-slice opt-out and run closure gates

- Task IDs: `TASK-271`
- Objective: prove no visual-debug work occurs in ordinary slicing and run focused quality gates.
- Precondition: Steps 2 and 3 pass, or any unresolved draft seam is recorded as a named blocker.
- Postcondition: AC-6 passes; focused contract/determinism/overhead tests, all-target check, and clippy provide bounded evidence.
- Files allowed to read, with ranges when over 300 lines:
  - `target/test-output.log` - summary or bounded failure ranges only.
  - Changed packet-local skill/examples/tests only for diagnostics.
- Files allowed to edit (at most 3):
  - `crates/slicer-runtime/tests/visual_debug_agent_overhead_tdd.rs`
  - Packet-local skill/example/test files only for a packet-local failure.
- Files explicitly out of bounds:
  - Ordinary slice production code, renderers/parsers/capture, WIT/IR/schema, modules, WASM, coordinate code, Orca sources, lockfiles, generated output, and unrelated packets.
- Expected sub-agent dispatches:
   - Question: run the four focused tests, all-target check, and clippy; scope: repository commands only; return: `FACT` in 5 lines or fewer.
- Context cost: `S`
- Authoritative docs:
  - `docs/specs/visual-pipeline-debug.md` - lines 41-59.
  - `docs/11_operational_governance_and_acceptance_gate.md` - lines 86-117.
  - `docs/07_implementation_status.md` - line 243 only.
- Verification:
  - `cargo test -p slicer-runtime --all-targets --test visual_debug_agent_overhead_tdd 2>&1 | tee target/test-output.log` - FACT from the log.
  - `cargo check --workspace --all-targets` - FACT pass/fail.
  - `cargo clippy --workspace --all-targets -- -D warnings` - FACT pass/fail.
- Exit condition: ordinary slice has no visual-debug signal/artifact and all packet-local gates pass with no known unintended side effect.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | S | Forward-contract inventory only. |
| Step 2 | S | Skill and two guide examples. |
| Step 3 | M | Cross-crate contract and byte-determinism tests. |
| Step 4 | S | Overhead proof and bounded quality gates. |

Split before activation if aggregate cost exceeds M or any step is L.

## Packet Completion Gate

- All steps and exits complete.
- Every pipe-suffixed AC command returns PASS.
- Forward contracts are resolved or the packet remains draft with explicit blockers.
- Update `docs/07_implementation_status.md` through a worker dispatch, never a full backlog read.
- `packet.spec.md` is ready for `status: implemented` only after the independent reviewer clears the draft.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed AC and packet-level gate command.
- Record remaining packet-local risk and all resolved `[FWD]` contracts.
- Confirm context stayed at or below 150k standard, or at/below 300k only with a logged swarm ESCALATION; otherwise record a packet-authoring lesson.

All `cargo check`, `cargo clippy`, and `cargo test` invocations in gate and verification commands must use `--all-targets` so test, bench, and example targets compile.
