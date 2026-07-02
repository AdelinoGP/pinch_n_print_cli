# Implementation Plan: 136_infill-parity-integration

## Execution Rules

- One atomic step at a time.
- Each step must map back to the packet's grouped task IDs.
- TDD first, then implementation, then the narrowest falsifying validation.
- Each step honors the context-discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. The fields below are not optional metadata — they are the budget contract for this step.

## Steps

### Step 1: Fixture decision + M3 fixture

- Task IDs:
  - `TASK-261`
- Objective: resolve the fixture `[FWD]` (loader-metadata FACT: can
  `cube_cilindrical_modifier.3mf`-style metadata carry a per-volume density delta?), then
  land the fixture (extend the existing 3MF or author `resources/cube_infill_modifier.3mf`;
  fallback: programmatic 3MF construction in-test) with base 0.15 / modifier 0.40.
- Precondition: packets 129–135 closed.
- Postcondition: fixture loads (a loader smoke assert passes: object + 1 modifier volume with
  the density delta).
- Files allowed to read: none directly beyond the fixture (loader facts delegated).
- Files allowed to edit (≤ 3):
  - `resources/cube_infill_modifier.3mf` (or the extended existing fixture)
  - a loader smoke test file (assert volume + delta parsed)
- Files explicitly out-of-bounds for this step: pipeline code.
- Expected sub-agent dispatches:
  - the loader-metadata FACT dispatch (design §Expected Sub-Agent Dispatches)
  - "Run the loader smoke test; FACT"
- Context cost: `S`
- Authoritative docs: `docs/specs/modifier-region-infill.md` §M3.
- OrcaSlicer refs: none.
- Verification:
  - loader smoke test — FACT
- Exit condition: fixture committed + parsing proven; `[FWD]` recorded as resolved.

### Step 2: E2e tests — modifier composition + wedge linkage + no-linker guard (RED→GREEN)

- Task IDs:
  - `TASK-261`
- Objective: author and green the four tests: `modifier_infill_two_densities` (AC-1),
  `modifier_infill_boundary_anchoring` (AC-2), `wedge_linked_infill_report` (AC-3),
  `no_linker_module_degraded_raw_output` (AC-N1). Failures here are 129–135 defects — triage
  per the scope fence (≤ 20-line deviations here; else packetize) before proceeding.
- Precondition: Step 1 exit condition.
- Postcondition: all four green; any deviation recorded in `packet.spec.md` §Deviations.
- Files allowed to read (with line-range hints when > 300 lines):
  - one neighboring e2e test (harness idiom); the new fixture path
- Files allowed to edit (≤ 3 per wave):
  - `crates/slicer-runtime/tests/e2e/modifier_infill_tdd.rs` (new) + harness mod line
  - `crates/slicer-runtime/tests/e2e/` wedge-report test file
  - `crates/slicer-runtime/tests/integration/no_linker_module_degraded_raw_output_tdd.rs`
    (new) + harness mod line
- Files explicitly out-of-bounds for this step: module/linker sources (fence).
- Expected sub-agent dispatches:
  - "Run `cargo test -p slicer-runtime --test e2e -- modifier_infill … | grep '^test
    result'`; FACT + counts; SNIPPETS ≤20 on failure"
  - "Run `cargo test -p slicer-runtime --test integration -- no_linker … `; FACT"
- Context cost: `M`
- Authoritative docs: `docs/specs/modifier-region-infill.md` §M3 assert list.
- OrcaSlicer refs: none.
- Verification:
  - AC-1, AC-2, AC-3, AC-N1 pipe commands — FACT each
- Exit condition: four tests green; visual report note staged for the closure log.

### Step 3: `infill_overlap` CLI binding

- Task IDs:
  - `TASK-261`
- Objective: bind `infill_overlap` through the CLI config path (pattern:
  `fill_holder_cli_binding_tdd.rs`); test that 0.30 reaches the linker and changes the
  overlap boundary vs the 0.45 default.
- Precondition: Step 2 exit condition.
- Postcondition: AC-4 green.
- Files allowed to read (with line-range hints when > 300 lines):
  - `crates/slicer-ir/tests/fill_holder_cli_binding_tdd.rs` (pattern)
  - the binding production site it points to (ranged)
- Files allowed to edit (≤ 3):
  - the binding production site
  - `crates/slicer-ir/tests/infill_overlap_cli_binding_tdd.rs` (new)
- Files explicitly out-of-bounds for this step: linker internals.
- Expected sub-agent dispatches:
  - "Run `cargo test -p slicer-ir -- infill_overlap_cli_binding …`; FACT"
- Context cost: `S`
- Authoritative docs: none new.
- OrcaSlicer refs: none.
- Verification:
  - AC-4 pipe command — FACT
- Exit condition: AC-4 green.

### Step 4: Golden restore + bless sweep

- Task IDs:
  - `TASK-261`
- Objective: for every carve-list entry: remove the `carved: infill-parity D6` marker, re-run,
  re-bless the expectation from two consecutive identical runs, record old→new + 1-line
  justification in the closure log. Geometry gates (AC-1/2/3) are green by precondition —
  the bless is justified by them.
- Precondition: Steps 2-3 exit conditions.
- Postcondition: AC-5 green (zero markers); restored suites green.
- Files allowed to read: the carve list; restored test failure output (delegated).
- Files allowed to edit: the carved test files (markers + expectations only; ≤ 3 per wave).
- Files explicitly out-of-bounds for this step: everything else.
- Expected sub-agent dispatches:
  - the per-entry bless dispatch (design §Expected Sub-Agent Dispatches) — one per fixture
  - "Run `rg -c 'carved: infill-parity D6' --glob '*.rs' | wc -l`; FACT (expect 0)"
- Context cost: `M`
- Authoritative docs: none new.
- OrcaSlicer refs: none.
- Verification:
  - AC-5 pipe command — FACT
- Exit condition: zero markers; restored suites green; justifications logged.

### Step 5: Acceptance ceremony + docs/07 closure sweep

- Task IDs:
  - `TASK-261`
- Objective: `cargo xtask build-guests --check` (must be clean), then dispatch
  `cargo xtask test --workspace --summary` (FACT verdict only); triage any failure per the
  fence; flip TASK-254…TASK-261 rows in docs/07 with closure notes.
- Precondition: Step 4 exit condition.
- Postcondition: ceremony PASS; docs/07 rows closed; packet ready for `status: implemented`.
- Files allowed to read: none directly (all delegated).
- Files allowed to edit (≤ 3):
  - `docs/07_implementation_status.md` (via dispatch)
- Files explicitly out-of-bounds for this step: everything else.
- Expected sub-agent dispatches:
  - "Run `cargo xtask build-guests --check`; FACT"
  - "Run `cargo xtask test --workspace --summary`; return verdict block ONLY"
  - "Update docs/07 TASK-254…261 rows; FACT + the grep `rg -q 'TASK-261.*[Cc]losed'
    docs/07_implementation_status.md`"
- Context cost: `S` (all delegated)
- Authoritative docs: `CLAUDE.md` §Test Discipline (ceremony contract).
- OrcaSlicer refs: none.
- Verification:
  - the three dispatches — FACT each
- Exit condition: ceremony PASS recorded; Doc Impact grep hits.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | S | fixture decision + data |
| Step 2 | M | four composition tests |
| Step 3 | S | CLI binding |
| Step 4 | M | restore + bless sweep |
| Step 5 | S | ceremony + docs/07 (delegated) |

## Packet Completion Gate

- All steps complete.
- Every step exit condition is met.
- Packet acceptance criteria green (each verification command dispatched and returned PASS).
- `docs/07_implementation_status.md` updated for TASK-254…TASK-261 (via worker dispatch —
  never edited by loading the full backlog into the implementer's context).
- Reopened or superseded packet status transitions reconciled (none expected).
- `packet.spec.md` ready to move to `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed acceptance criterion command from `packet.spec.md`.
- Confirm packet-level verification commands are green (including the workspace `--summary`
  verdict).
- Record any remaining packet-local risk explicitly before moving to `status: implemented`.
- Confirm the implementer's peak context usage stayed under 70%; if not, log it as a
  packet-authoring lesson for future spec-packet-generator runs.
