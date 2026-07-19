# Design: 136_infill-parity-integration

## Controlling Code Paths

- Primary code path: none changed — this packet exercises the composed pipeline
  (loader modifier ingestion → 132 split → 131 per-region config → 134/135 raw emit →
  133 linking → gcode emit) through new e2e tests and fixture data.
- CLI binding path: `infill_overlap` follows the `fill_holder` binding pattern at
  `crates/slicer-ir/tests/fill_holder_cli_binding_tdd.rs` (3 tests, 66 lines; production
  site at `crates/slicer-ir/src/resolved_config.rs:99-112`).
- Neighboring tests or fixtures: `crates/slicer-runtime/tests/{e2e,integration}/`;
  `resources/cube_cilindrical_modifier.3mf` (30625 bytes, exists) or a new authored
  fixture; the carved tests across the 5 `cube_4color_*` files in
  `crates/slicer-runtime/tests/executor/`.
- OrcaSlicer comparison surface: none — the reference behavior is encoded in
  `docs/specs/modifier-region-infill.md` §Context.

## Architecture Constraints

<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and, if `STALE:` is reported, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.
- `cargo test --workspace` is permitted here ONLY through `cargo xtask test --workspace
  --summary` at the acceptance ceremony (CLAUDE.md §Test Discipline; the freshness gate must
  fire first).
- Pre-activation gate (verified 2026-07-19): TASK-257, TASK-258, TASK-259, TASK-260 must
  be closed before this packet activates. The linker is what makes AC-2/AC-3/AC-N1
  meaningful; without it, the e2e assertions either fail or are vacuous.
- Bless-after-geometry ordering (requirements §Step Completion Expectations) is a hard gate.
- If integration exposes a defect in 129–135 output: ≤ 20-line fixes land here as recorded
  deviations; anything larger spawns a follow-up packet (scope fence).

## Code Change Surface

- Selected approach: prove composition at three levels — IR-level (post-postprocess
  `InfillIR` asserts), gcode-level (wall-set count, spacing ratio), artifact-level (report
  exists + closure-log visual note) — then restore goldens in one sweep with per-fixture
  justification.
- Exact changes: fixture (extend or author — Step 1 `[FWD]`), 4 new e2e/integration tests,
  `infill_overlap` CLI binding + test, carve-marker removals + re-blessed expectations,
  docs/07 closure sweep.
- Rejected alternatives: (a) blessing per-packet (rejected in the grilling — D6 chose
  carve-once/bless-once); (b) skipping the no-linker guard as "obvious" — rejected: it pins
  ADR-0025's degraded-not-failed trade-off, the roadmap's most surprising property for a
  future maintainer; (c) programmatic 3MF construction in-test for the modifier fixture —
  viable fallback, but a committed fixture matches the repo's authored-fixture precedent and
  is reusable by later packets.

## Files in Scope (read + edit)

- `crates/slicer-runtime/tests/e2e/` — new `modifier_infill_*` + `wedge_linked_infill_report`
  tests (+ harness mod lines).
- `crates/slicer-runtime/tests/integration/no_linker_module_degraded_raw_output_tdd.rs`
  (new) — pattern: `tests/e2e/scenario_traces_tdd.rs:336-365`
  (`scenario_3_non_fatal_module_failure_marks_slice_degraded_not_aborted`).
- `resources/cube_cilindrical_modifier.3mf` sidecar extension (preferred, since the
  sidecar `Metadata/model_settings.config` is the existing channel), OR a new authored
  `resources/cube_infill_modifier.3mf`, OR programmatic 3MF construction in-test — Step 1
  decision.
- `infill_overlap` CLI binding site (patterned from `fill_holder` at
  `crates/slicer-ir/src/resolved_config.rs:99-112`) + its `slicer-ir` test.
- The 5 carved test files in `crates/slicer-runtime/tests/executor/cube_4color_*` (and
  `cube_4color_arachne.rs`) — marker removal + expectation re-bless only.
- `docs/07_implementation_status.md` — closure sweep for TASK-257/258/259/260/261 (via
  dispatch).

## Read-Only Context

- `.ralph/specs/131_per-region-config-delivery/carve-list.md` — the restoration worklist
  (93 lines, enumerates ~20 carved tests).
- `crates/slicer-ir/tests/fill_holder_cli_binding_tdd.rs` — binding pattern (66 lines,
  3 tests).
- `crates/slicer-model-io/src/loader.rs:702-710` — `ModifierVolume.config_delta` flow
  (the path the fixture must travel to reach per-region config).
- `crates/slicer-scheduler/src/validation.rs:11-15` — `FILL_CLAIM_IDS` (the four fill
  claims; `claim:infill-link` is NOT here today — packet 133 will add it).
- One neighboring e2e test (`wedge_mvp_gcode_has_extrusion_moves` or
  `wedge_default_emits_sparse_infill_marker`) — fixture/slice harness idiom.
- `tests/e2e/scenario_traces_tdd.rs:336-365` — the degraded-state pattern for AC-N1.

## Out-of-Bounds Files

- All module and linker sources — algorithms are closed; defects route per the scope fence.
- `OrcaSlicerDocumented/**` — never load.
- The HTML report body; full workspace test output — summary/artifact-level only.
- `target/`, `Cargo.lock` — never load.

## Expected Sub-Agent Dispatches

- "FACT ≤8 lines: does the 3MF loader's `ModifierVolume.config_delta` path already read a
  per-volume density setting from the `cube_cilindrical_modifier.3mf` sidecar metadata?
  Cite `loader.rs:702-710` and any test that exercises per-volume density" — Step 1 fixture
  decision.
- "Run `cargo test -p slicer-runtime --test e2e -- modifier_infill … | grep '^test result'`;
  FACT + counts; SNIPPETS ≤20 on failure" — AC-1/AC-2 gates.
- "For each carve-list entry: run the restored test, return FACT old→new expectation + 1-line
  justification" — Step 4 bless sweep.
- "Run `cargo xtask test --workspace --summary`; return the verdict block ONLY (PASS/FAIL +
  per-binary result lines + failing names)" — acceptance ceremony.
- "Update docs/07 TASK-257/258/259/260/261 rows to closed with one-line closure notes;
  return FACT" — closure sweep.

## Data and Contract Notes

- IR/WIT/manifest contracts: none changed. The CLI binding adds a key routing, not a new
  config semantic (the key + default live in the linker manifest since 133).
- Determinism: re-blessed SHAs must come from two consecutive identical runs (bless twice,
  compare) to avoid pinning a flaky value.

## Locked Assumptions and Invariants

- Bless order: geometry ACs before SHAs (D6 gate).
- Degraded-not-failed without the linker (AC-N1) — the pipeline must never hard-require the
  linker's presence.
- The modifier fixture's expected behavior is exactly ADR-0030's: one wall set, split fill,
  anchored shared arc — deviations mean 132/133 bugs, not fixture adjustments.
- No algorithm edits beyond the ≤ 20-line deviation fence.

## Risks and Tradeoffs

- Fixture authorship is the schedule risk (offline authoring loop); the programmatic-3MF
  fallback bounds it. The sidecar extension (extending
  `cube_cilindrical_modifier.3mf`'s `Metadata/model_settings.config`) is the cheapest
  option and exercises the existing per-volume-density loader path.
- The bless sweep is judgment-heavy: the per-fixture justification requirement plus the
  geometry-first gate is the guard against rubber-stamping wrong output.
- The workspace ceremony may surface far-flung stale asserts (packet-126-style surprises);
  each is triaged — fixed here if mechanical, packetized if not — and recorded.
- TASK-257 and TASK-258 are still open (verified 2026-07-19); if they remain open at the
  time of activation, the activation ceremony refuses to start and the precondition is
  recorded in the closure log. The packet text does NOT offer a "ship without linker"
  variant — that's the design's whole point.

## Context Cost Estimate

- Aggregate: `M`
- Largest single step: `M` (bless sweep)
- Highest-risk dispatch: the workspace ceremony — MUST return the summary block only.

## Open Questions

- `[FWD]` Fixture: extend `cube_cilindrical_modifier.3mf` vs author
  `cube_infill_modifier.3mf` — decided by the Step-1 loader-metadata FACT.
- `[FWD]` Gyroid multi-role e2e inclusion — only if the fixture extension makes it ≤ S extra
  (spec M3 "if cheap" clause); otherwise recorded as a follow-up note in docs/07.
