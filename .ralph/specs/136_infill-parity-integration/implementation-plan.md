# Implementation Plan: 136_infill-parity-integration

## Execution Rules

- One atomic step at a time.
- Each step must map back to the packet's grouped task IDs.
- TDD first, then implementation, then the narrowest falsifying validation.
- Each step honors the context-discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. The fields below are not optional metadata — they are the budget contract for this step.

## Steps

### Step 0: Pre-activation dependency check

- Task IDs: none (verification only).
- Objective: confirm the four upstream tasks are closed before this packet activates.
  Five FACT dispatches, no code changes.
  - `rg -q 'TASK-254.*\[[xX]\]' docs/07_implementation_status.md` (clip_polylines — ✓ today)
  - `rg -q 'TASK-255.*\[[xX]\]' docs/07_implementation_status.md` (WIT contract — ✓ today)
  - `rg -q 'TASK-256.*\[[xX]\]' docs/07_implementation_status.md` (per-region config — ✓ today)
  - `rg -q 'TASK-257.*\[[xX]\]' docs/07_implementation_status.md` (modifier split — ✗ today)
  - `rg -q 'TASK-258.*\[[xX]\]' docs/07_implementation_status.md` (linker — ✗ today)
  - `rg -q 'TASK-259.*\[[xX]\]' docs/07_implementation_status.md` (rectilinear rewrite — ✗ today)
  - `rg -q 'TASK-260.*\[[xX]\]' docs/07_implementation_status.md` (gyroid rewrite — ✗ today)
- Precondition: clean tree.
- Postcondition: a one-line PASS/FAIL note recorded. FAIL refuses activation; the
  implementer either waits for upstream closure or files a recorded deviation.
- Files allowed to edit: none (read-only verification step).
- Context cost: `S`.
- Exit condition: PASS. If FAIL, the packet is parked.

### Step 1: Fixture decision + M3 fixture

- Task IDs:
  - `TASK-261`
- Objective: resolve the fixture `[FWD]` (FACT: does the loader's
  `ModifierVolume.config_delta.fields` path at `loader.rs:702-710` already read a
  per-volume density setting from the 3MF sidecar `Metadata/model_settings.config`?),
  then land the fixture (extend the existing `cube_cilindrical_modifier.3mf` sidecar,
  or author `resources/cube_infill_modifier.3mf` offline, or programmatic 3MF
  construction in-test) with base 0.15 / modifier 0.40.
- Precondition: Step 0 PASS.
- Postcondition: fixture loads (a loader smoke assert passes: object + 1 modifier volume with
  the density delta).
- Files allowed to read: none directly beyond the fixture (loader facts delegated).
- Files allowed to edit (≤ 3):
  - `resources/cube_cilindrical_modifier.3mf` sidecar `Metadata/model_settings.config`
    (preferred), OR `resources/cube_infill_modifier.3mf` (new), OR the new test file with
    programmatic 3MF
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
  `no_linker_module_degraded_raw_output` (AC-N1). No e2e test today asserts on
  `points_per_path` (verified 2026-07-19); these are genuinely new IR-level assertions.
  The no-linker guard uses the `collector.is_degraded()` precedent from
  `tests/e2e/scenario_traces_tdd.rs:336-365`. Failures here are 129–135 defects — triage
  per the scope fence (≤ 20-line deviations here; else packetize) before proceeding.
- Precondition: Step 1 exit condition.
- Postcondition: all four green; any deviation recorded in `packet.spec.md` §Deviations.
- Files allowed to read (with line-range hints when > 300 lines):
  - one neighboring e2e test (harness idiom); the new fixture path
  - `tests/e2e/scenario_traces_tdd.rs:336-365` for the degraded-state pattern
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
  `fill_holder_cli_binding_tdd.rs` at `crates/slicer-ir/tests/`, 3 tests, 66 lines;
  production site at `crates/slicer-ir/src/resolved_config.rs:99-112`); test that 0.30
  reaches the linker and changes the overlap boundary vs the 0.45 default.
- Precondition: Step 2 exit condition.
- Postcondition: AC-4 green.
- Files allowed to read (with line-range hints when > 300 lines):
  - `crates/slicer-ir/tests/fill_holder_cli_binding_tdd.rs` (pattern, 66 lines)
  - the binding production site at `crates/slicer-ir/src/resolved_config.rs:99-112`
- Files allowed to edit (≤ 3):
  - the binding production site (resolved_config.rs, around line 99-112)
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
- Objective: for every carve-list entry (~20 carved tests across 5 `cube_4color_*` files
  in `crates/slicer-runtime/tests/executor/`): remove the `carved: infill-parity D6`
  marker, re-run, re-bless the expectation from two consecutive identical runs, record
  old→new + 1-line justification in the closure log. Geometry gates (AC-1/2/3) are green
  by precondition — the bless is justified by them. The wedge-e2e
  `wedge_per_region_config_delivery_byte_identical` test (AC-N2 from packet 131, digest
  `8a3b645ee54fa5dbfa1232008db4820d2a364a30b4d196a504b424271308019f`) is the regression
  canary for byte-identical output on `regression_wedge.stl` (single-region, NOT
  carved).
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
  fence; flip TASK-257, TASK-258, TASK-259, TASK-260, TASK-261 rows in docs/07 with
  closure notes (TASK-254/255/256 are already closed and are NOT in this packet's
  scope).
- Precondition: Step 4 exit condition.
- Postcondition: ceremony PASS; docs/07 rows closed; packet ready for `status: implemented`.
- Files allowed to read: none directly (all delegated).
- Files allowed to edit (≤ 3):
  - `docs/07_implementation_status.md` (via dispatch)
- Files explicitly out-of-bounds for this step: everything else.
- Expected sub-agent dispatches:
  - "Run `cargo xtask build-guests --check`; FACT"
  - "Run `cargo xtask test --workspace --summary`; return verdict block ONLY"
  - "Update docs/07 TASK-257/258/259/260/261 rows; FACT + the grep `rg -q
    'TASK-261.*[Cc]losed' docs/07_implementation_status.md`"
- Context cost: `S` (all delegated)
- Authoritative docs: `CLAUDE.md` §Test Discipline (ceremony contract).
- OrcaSlicer refs: none.
- Verification:
  - the three dispatches — FACT each
- Exit condition: ceremony PASS recorded; Doc Impact grep hits.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 0 | S | pre-activation dependency check (4 upstream tasks) |
| Step 1 | S | fixture decision + data |
| Step 2 | M | four composition tests |
| Step 3 | S | CLI binding |
| Step 4 | M | restore + bless sweep (~20 fixtures) |
| Step 5 | S | ceremony + docs/07 (delegated) |

## Packet Completion Gate

- All steps complete.
- Every step exit condition is met.
- Packet acceptance criteria green (each verification command dispatched and returned PASS).
- `docs/07_implementation_status.md` updated for TASK-257, TASK-258, TASK-259, TASK-260,
  TASK-261 (via worker dispatch — never edited by loading the full backlog into the
  implementer's context).
- Reopened or superseded packet status transitions reconciled (none expected).
- `packet.spec.md` ready to move to `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed acceptance criterion command from `packet.spec.md`.
- Confirm packet-level verification commands are green (including the workspace `--summary`
  verdict).
- Record any remaining packet-local risk explicitly before moving to `status: implemented`.
- Confirm the implementer's peak context usage stayed under 70%; if not, log it as a
  packet-authoring lesson for future spec-packet-generator runs.
