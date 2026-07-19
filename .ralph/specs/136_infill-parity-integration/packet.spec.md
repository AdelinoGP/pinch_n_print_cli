---
status: draft
packet: 136_infill-parity-integration
task_ids:
  - TASK-261
backlog_source: docs/07_implementation_status.md
context_cost_estimate: M
---

# Packet Contract: 136_infill-parity-integration

## Goal

Close the main infill-parity roadmap: the M3 modifier-infill e2e fixture, `infill_overlap`
CLI exposure, restoration + single re-bless of every `carved: infill-parity D6` golden
(5 `cube_4color_*` files in `crates/slicer-runtime/tests/executor/`), the no-linker
degraded-output guard, and the workspace acceptance ceremony.

## Scope Boundaries

Integration and closure only — no algorithm changes. This packet proves packets 129–135
compose: modifier densities reach the modules, the linker links across the pipeline, output
is visually and geometrically sane, and the golden baseline is re-established in one
justified bless event. The lightning sub-roadmap (137–140) follows separately with its own
contained bless.

Note on dependencies (verified 2026-07-19):
- TASK-254 (clip_polylines), TASK-255 (WIT contract), TASK-256 (per-region config) — CLOSED.
  Their conditions are already realized in the tree.
- TASK-257 (packet 132, modifier split), TASK-258 (packet 133, infill-linker) — currently
  OPEN. Both block the linker-driven ACs in this packet. Step 0 is a hard pre-activation
  gate; if 132/133 are not closed at activation, the activation ceremony refuses to start
  and the implementer either waits or files a recorded deviation.

## Prerequisites and Blockers

- Depends on: `129` (TASK-254 ✓) `130` (TASK-255 ✓) `131` (TASK-256 ✓) `132` (TASK-257 ✗)
  `133` (TASK-258 ✗) `134` (TASK-259 ✗) `135` (TASK-260 ✗). The four currently-open
  dependencies are activation-gating, not just nominal.
- Unblocks: `137_lightning-prepass-contract` (roadmap continuation).
- Activation blockers: TASK-257 and TASK-258 must be closed before this packet activates
  (the linker is what makes the AC-2/AC-3/AC-N1 assertions meaningful). If they are
  still open, the packet refuses to activate and the precondition is recorded in the
  closure log.

## Acceptance Criteria

- **AC-1. Given** a cube fixture with a centered infill-modifier volume (base density 0.15,
  modifier 0.40, both roles on `rectilinear-infill`), **when** sliced end-to-end, **then**
  the g-code contains exactly one wall set per layer (zero wall loops at the modifier
  boundary) and the sparse infill exhibits two distinct line spacings whose ratio matches
  0.40/0.15 within 10%. The modifier-density delta flows through the loader's existing
  path: `ModifierVolume.config_delta.fields` (loader.rs:702-710) →
  `ConfigDelta` → per-region resolved config. The 131 carve-list protects the
  `wedge_per_region_config_delivery_byte_identical` (AC-N2 there, digest
  `8a3b645ee54fa5dbfa1232008db4820d2a364a30b4d196a504b424271308019f`) from breaking. |
  `cargo test -p slicer-runtime --test e2e -- modifier_infill_two_densities 2>&1 | tee target/test-output.log | grep "^test result"`
- **AC-2. Given** the AC-1 slice, **when** the committed post-`InfillPostProcess` `InfillIR`
  is inspected, **then** every sparse path lies inside its own sub-region's polygon, paths
  adjacent to the wall-less shared arc reach it within 0.5 × spacing (no unfilled ring), and
  both regions' paths are linked (every bucket's mean points-per-path > 2). No e2e test
  currently asserts on `points_per_path` (verified 2026-07-19 — `rg` shows no hits); the
  new test is genuinely new. The linker's `claim:infill-link` is NOT in
  `FILL_CLAIM_IDS` today (`validation.rs:11-15` lists only the four fill claims); packet
  133 must add it. | `cargo test -p slicer-runtime --test e2e -- modifier_infill_boundary_anchoring 2>&1 | tee target/test-output.log | grep "^test result"`
- **AC-3. Given** `resources/regression_wedge.stl` (4084 bytes, exists) sliced with `--report`,
  **when** the run completes, **then** the HTML report exists and the committed `InfillIR`
  contains linked sparse polylines (mean points-per-path > 2 — raw 2-point output would
  fail); the visual confirmation of linked paths (no disjoint-segment travel storms) is
  recorded in the closure log. Existing `wedge_default_emits_sparse_infill_marker` and
  `wedge_default_emits_bridge_infill_marker` tests in `slice_end_to_end_tdd.rs` cover
  gcode-level marker presence; the new test specifically asserts IR-level linkage. |
  `cargo test -p slicer-runtime --test e2e -- wedge_linked_infill_report 2>&1 | tee target/test-output.log | grep "^test result"`
- **AC-4. Given** the CLI, **when** `infill_overlap` is set via the existing config-binding
  mechanism (pattern: `crates/slicer-ir/tests/fill_holder_cli_binding_tdd.rs` —
  `apply_cli_key` on the resolved config; 3 precedent tests in 66 lines), **then** the
  linker receives the value (0.30 produces a measurably different overlap boundary than
  the 0.45 default). The `fill_holder` binding in `resolved_config.rs:99-112` is the
  model; `infill_overlap` adds a single `f32` key alongside. | `cargo test -p slicer-ir -- infill_overlap_cli_binding 2>&1 | tee target/test-output.log | grep "^test result"`
- **AC-5. Given** the carve list from packet 131 (`.ralph/specs/131_per-region-config-delivery/carve-list.md`,
  enumerates ~20 carved multi-region tests across 5 `cube_4color_*` files in
  `crates/slicer-runtime/tests/executor/`), **when** this packet closes, **then** zero
  `carved: infill-parity D6` markers remain in the test tree, and every restored test
  carries its re-blessed expectation with a closure-log justification per fixture. The
  today-count is 5 carved files (one shared `#[ignore]` marker pattern across the
  ~20 tests); the AC target is 0. | `rg -c 'carved: infill-parity D6' --glob '*.rs' | wc -l | grep -q '^0$' && echo RESTORED`

## Negative Test Cases

- **AC-N1. Given** a module set WITHOUT `infill-linker` (module-dir excluding it; the
  `claim:infill-link` is therefore absent), **when** a slice runs, **then** it completes
  without error and the committed `InfillIR` is raw disjoint output (sparse mean
  points-per-path ≤ 2 for rectilinear regions) — degraded, not failed (ADR-0025 trade-off
  pin). The existing `scenario_3_non_fatal_module_failure_marks_slice_degraded_not_aborted`
  test in `tests/e2e/scenario_traces_tdd.rs:336-365` is the precedent for the
  `collector.is_degraded()` mechanism; this new test specifically asserts the
  no-linker case. | `cargo test -p slicer-runtime --test integration -- no_linker_module_degraded_raw_output 2>&1 | tee target/test-output.log | grep "^test result"`

## Verification

- `cargo xtask test --workspace --summary` (packet-close acceptance ceremony — dispatch to a
  sub-agent; FACT verdict + failing-test names only)
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo xtask build-guests --check`
- The four e2e/integration AC commands (AC-1, AC-2, AC-3, AC-N1)
- The `fill_holder_cli_binding` precedent test still green (no regression in the binding
  pattern)

## Authoritative Docs

- `docs/specs/infill-parity-rectilinear-gyroid-linker.md` §Phase 5 — closure contract.
- `docs/specs/modifier-region-infill.md` §Phase M3 — the fixture's assert list.
- `docs/16_slicer_report.md` — report format (delegate; only if the report assert needs it).

## Doc Impact Statement (Required)

- `docs/07_implementation_status.md` — TASK-257, TASK-258, TASK-259, TASK-260, TASK-261
  rows flipped to closed with closure notes. TASK-254/255/256 are already closed
  (verified 2026-07-19) and are not in this packet's scope. Confirmation grep:
  `rg -q 'TASK-261.*[Cc]losed' docs/07_implementation_status.md`

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- stop reading at 60% context and hand off at 85%

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation.
