# Design: 136_infill-parity-integration

## Controlling Code Paths

- Primary code path: none changed — this packet exercises the composed pipeline
  (loader modifier ingestion → 132 split → 131 per-region config → 134/135 raw emit →
  133 linking → gcode emit) through new e2e tests and fixture data.
- CLI binding path: `infill_overlap` follows the `fill_holder` binding pattern
  (`crates/slicer-ir/tests/fill_holder_cli_binding_tdd.rs` and its production counterpart in
  the config/CLI binding code — locate by patterning that test, not by browsing).
- Neighboring tests or fixtures: `crates/slicer-runtime/tests/{e2e,integration}/`;
  `resources/cube_cilindrical_modifier.3mf` (extension candidate) or a new authored fixture;
  the carved tests across the tree (restored here).
- OrcaSlicer comparison surface: none — the reference behavior is encoded in
  `docs/specs/modifier-region-infill.md` §Context.

## Architecture Constraints

<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and, if `STALE:` is reported, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.
- `cargo test --workspace` is permitted here ONLY through `cargo xtask test --workspace
  --summary` at the acceptance ceremony (CLAUDE.md §Test Discipline; the freshness gate must
  fire first).
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
  (new).
- `resources/cube_infill_modifier.3mf` (new) OR `resources/cube_cilindrical_modifier.3mf`
  metadata extension — Step 1 decision.
- `infill_overlap` CLI binding site (patterned from fill_holder binding) + its `slicer-ir`
  test.
- Carved test files — marker removal + expectation re-bless only.
- `docs/07_implementation_status.md` — closure sweep (via dispatch).

## Read-Only Context

- `.ralph/specs/131_per-region-config-delivery/carve-list.md` — the restoration worklist.
- `crates/slicer-ir/tests/fill_holder_cli_binding_tdd.rs` — binding pattern (one read).
- One neighboring e2e test — fixture/slice harness idiom (`slicer_cache` usage).

## Out-of-Bounds Files

- All module and linker sources — algorithms are closed; defects route per the scope fence.
- `OrcaSlicerDocumented/**` — never load.
- The HTML report body; full workspace test output — summary/artifact-level only.
- `target/`, `Cargo.lock` — never load.

## Expected Sub-Agent Dispatches

- "FACT ≤5 lines: does the 3MF loader's modifier-delta path already read a per-volume
  density setting from `cube_cilindrical_modifier.3mf`-style metadata (cite loader lines)?" —
  Step 1 fixture decision.
- "Run `cargo test -p slicer-runtime --test e2e -- modifier_infill … | grep '^test result'`;
  FACT + counts; SNIPPETS ≤20 on failure" — AC-1/AC-2 gates.
- "For each carve-list entry: run the restored test, return FACT old→new expectation + 1-line
  justification" — Step 4 bless sweep.
- "Run `cargo xtask test --workspace --summary`; return the verdict block ONLY (PASS/FAIL +
  per-binary result lines + failing names)" — acceptance ceremony.
- "Update docs/07 TASK-254…261 rows to closed with one-line closure notes; return FACT" —
  closure sweep.

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
  fallback bounds it.
- The bless sweep is judgment-heavy: the per-fixture justification requirement plus the
  geometry-first gate is the guard against rubber-stamping wrong output.
- The workspace ceremony may surface far-flung stale asserts (packet-126-style surprises);
  each is triaged — fixed here if mechanical, packetized if not — and recorded.

## Context Cost Estimate

- Aggregate: `M`
- Largest single step: `M` (bless sweep)
- Highest-risk dispatch: the workspace ceremony — MUST return the summary block only.

## Open Questions

- `[FWD]` Fixture: extend `cube_cilindrical_modifier.3mf` vs author
  `cube_infill_modifier.3mf` — decided by the Step-1 loader-metadata FACT.
- `[FWD]` Gyroid multi-role e2e inclusion — only if the fixture extension makes it ≤ S extra
  (spec M3 "if cheap" clause); otherwise recorded as a follow-up note in docs/07.
