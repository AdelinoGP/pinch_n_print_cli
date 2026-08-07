# Implementation Plan: 200-batched-host-bridge-wasm-arms

## Execution Rules

- Work one atomic step at a time; every step maps to `DEV-094`.
- Use TDD, then implementation, then the narrowest falsifying validation.
- Every field below is a context-budget contract and must be filled independently; never write "see Step 1".
- Every `cargo test` invocation tees to `target/test-output.log` and results are read from the log (CLAUDE.md §Test output). Every step that edits WIT, `slicer-sdk`, or `modules/core-modules` ends with `cargo xtask build-guests --check` clean before its verification tests run.

## Steps

### Step 1: Baseline evidence capture (before any code change)

- Task IDs: `DEV-094`
- Objective: capture the pre-change fuel and wall-clock baseline the Step 6 comparison and Step 7 amendment consume.
- Precondition: clean working tree at the packet's start commit; `cargo xtask build-guests --check` reports clean (if STALE, rebuild first — a stale baseline poisons the whole A/B).
- Postcondition: `target/p200-profile-before.jsonl` exists; the step's completion note quotes (a) per-(module, scope) `self_fuel`/`total_fuel` rows for `com.core.classic-perimeters` and `com.core.support-planner` from the summary, (b) three profiling-off wall-clock timings + median, (c) the run-to-run spread that Step 6's decision rule uses.
- Files allowed to read, with ranges when over 300 lines:
  - none directly — both runs are dispatched; the controller reads only returned FACTs.
- Files allowed to edit (at most 3): none (evidence artifacts under `target/` only).
- Files explicitly out of bounds: everything; this step edits nothing.
- Expected sub-agent dispatches:
  - Question: run `cargo run --bin pnp_cli --release -- slice --model resources/extruder_idler.obj --module-dir modules/core-modules --output target/p200.gcode --profile 2> target/p200-profile-before.jsonl` then `cargo run --bin pnp_cli --release -- profile --from target/p200-profile-before.jsonl --json > target/p200-summary-before.json`; return the two hot modules' scope rows; scope: workspace root; return: `FACT` (≤5 lines)
  - Question: time 3 profiling-off runs of the same slice command (no `--profile`); scope: workspace root; return: `FACT` (3 timings + median)
- Context cost: `S`
- Authoritative docs:
  - `docs/adr/0055-fuel-based-module-profiling.md` — direct (methodology: fuel primary, profiling-off wall-clock for absolute timings)
- OrcaSlicer refs: none.
- Verification:
  - `sh -c 'test -s target/p200-profile-before.jsonl && rg -q "com.core.classic-perimeters" target/p200-summary-before.json && rg -q "com.core.support-planner" target/p200-summary-before.json && echo PASS'` — FACT PASS/fail. If `com.core.support-planner` is absent, apply design.md [FWD-1] (switch model, re-capture) before proceeding.
- Exit condition: PASS above AND the completion note contains the quoted fuel rows and wall-clock numbers (the `target/` artifacts are disposable; the note is the durable record). Falsified if any number is missing or the summary lacks either module.

### Step 2: Arc-tolerance contract — WIT + host layer

- Task IDs: `DEV-094`
- Objective: add `arc-tolerance-mm: f32` to the singular `offset-polygons` func and the `offset-request` record in canonical WIT, and pass it through both host impls into `slicer_core::polygon_ops::offset`.
- Precondition: Step 1 exit met.
- Postcondition: AC-1 grep passes; `cargo check -p slicer-wasm-host --all-targets` compiles; host contract suite green. (The tree's wasm32-only inline copies are intentionally not yet updated — they compile only under the guest build, which is not run in this step.)
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-schema/wit/deps/common.wit` — whole file (162 lines)
  - `crates/slicer-wasm-host/src/host.rs` — the `impl hs::Host for HostExecutionContext` block only (≈ lines 2425–2735)
  - `crates/slicer-wasm-host/tests/contract/host_services_tdd.rs` — whole file (test fallout check; `ir_offset_polygons` keeps its own shape)
- Files allowed to edit (at most 3):
  - `crates/slicer-schema/wit/deps/common.wit`
  - `crates/slicer-wasm-host/src/host.rs` (only `fn offset_polygons` and `fn offset_polygons_batch`)
  - `crates/slicer-wasm-host/tests/contract/host_services_tdd.rs` (only if the regenerated trait signature breaks a direct call — expected fallout ≤2 lines)
- Files explicitly out of bounds: `crates/slicer-sdk/**` (Step 3's surface), `modules/**`, all docs.
- Blast-radius discipline (mandatory — record field addition): the `offset-request` WIT record gains a field, so every constructor of the host-side bindgen `hs::OffsetRequest` and the SDK-side records must be found before editing. LOCATIONS result (grounded 2026-08-07 by the authoring session): host-side, no test constructs `hs::OffsetRequest` (`batch_call_audit_tdd.rs` constructs only `RaycastRequest`/`SurfaceNormalRequest`, which are untouched); SDK/module-side literals are `crates/slicer-sdk/src/host_batch.rs` (doc example ≈107, tests ≈418 and ≈441, wasm32 marshal ≈151) and `modules/core-modules/support-planner/src/lib.rs` (`batch_offset` closure ≈300) — those are Steps 3–4 fallout, listed here so no step "discovers" them via cargo check.
- Expected sub-agent dispatches:
  - Question: does `cargo check -p slicer-wasm-host --all-targets` pass?; scope: workspace; return: `FACT pass/fail` + first error if fail
- Context cost: `S`
- Authoritative docs:
  - `docs/adr/0033-host-service-bridge-for-host-only-algorithms.md` — direct (layer 1/2 shape)
- OrcaSlicer refs: none.
- Verification:
  - `sh -c 'rg -U -q "offset-polygons:[^;]*arc-tolerance-mm: f32" crates/slicer-schema/wit/deps/common.wit && rg -U -q "record offset-request \{[^}]*arc-tolerance-mm: f32" crates/slicer-schema/wit/deps/common.wit && echo PASS'` — FACT PASS/fail (AC-1)
  - `mkdir -p target && cargo test -p slicer-wasm-host --test contract host_services 2>&1 | tee target/test-output.log && rg -q "^test result: ok" target/test-output.log` — FACT pass/fail
- Exit condition: both PASS. Falsified if the host impls still pass a literal `0.0` into `slicer_core::polygon_ops::offset` (grep `offset\(&ir_polys, r?\.?delta_mm` context must show the request/param tolerance, not `0.0`).

### Step 3: Arc-tolerance contract — SDK layer + fallout, and the seven wasm32 bridge arms

- Task IDs: `DEV-094`
- Objective: (a) `slicer_sdk::host::offset_polygons` gains `arc_tolerance_mm: f32` (native arm passes it to `slicer_core::polygon_ops::offset`); (b) `host_batch::OffsetRequest` gains `arc_tolerance_mm: f32` and both arms plus the inline `mod wit` copy carry it; (c) each of the seven singular wrappers gains its `#[cfg(target_arch = "wasm32")]` arm marshalling through one new shared inline import-only world (design.md §Code Change Surface move 2). Native arms byte-for-byte unchanged.
- Precondition: Step 2 exit met (inline worlds must copy the final wire shape).
- Postcondition: AC-2 grep passes; all SDK native suites green with updated call arity; `cargo xtask build-guests` rebuilds all guests and `--check` reports clean.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-sdk/src/host.rs` — wrapper bodies and mini-world templates only (≈ 120–500 and 850–899)
  - `crates/slicer-sdk/src/host_batch.rs` — whole file (469 lines; marshal-helper template)
  - `crates/slicer-schema/wit/deps/common.wit` — whole file (copy source for the inline world)
  - `crates/slicer-sdk/tests/host_wrappers_tdd.rs`, `crates/slicer-sdk/tests/smoke.rs` — whole files (arity fallout)
- Files allowed to edit (at most 3):
  - `crates/slicer-sdk/src/host.rs` (and/or a new sibling `crates/slicer-sdk/src/host_wit.rs` + its `mod` line in `crates/slicer-sdk/src/lib.rs` — counted together as the wrapper-layer edit)
  - `crates/slicer-sdk/src/host_batch.rs`
  - `crates/slicer-sdk/tests/host_wrappers_tdd.rs` + `crates/slicer-sdk/tests/smoke.rs` (arity-only fallout, counted together as the test-fallout edit)
- Files explicitly out of bounds: `modules/**` (Step 4/5 surface), `crates/slicer-wasm-host/**`, docs.
- Expected sub-agent dispatches:
  - Question: does `cargo xtask build-guests` complete and `--check` report clean?; scope: workspace; return: `FACT` (clean | STALE list | first build error)
  - Question: `cargo test -p slicer-sdk --features test` verdict; scope: workspace; return: `FACT pass/fail` + ≤20-line SNIPPETS from `target/test-output.log` on failure
- Context cost: `M`
- Authoritative docs:
  - `docs/adr/0033-host-service-bridge-for-host-only-algorithms.md` — direct (layer 3: cfg-split wrapper)
  - `docs/adr/0049-batched-host-services-over-threaded-guests.md` — §Decision only ("The SDK's batch wrappers must bridge to the host on wasm32"; singular forms stay)
- OrcaSlicer refs: none.
- Verification:
  - `sh -c 'for f in raycast-z-down surface-normal-at object-bounds clip-polygons "offset-polygons:" simplify-polygon now-us; do rg -q -- "$f" crates/slicer-sdk/src/host.rs || rg -q -- "$f" crates/slicer-sdk/src/host_wit.rs || { echo "MISSING: $f"; exit 1; }; done; echo PASS'` — FACT PASS/fail (AC-2)
  - `mkdir -p target && cargo test -p slicer-sdk --features test 2>&1 | tee target/test-output.log; rg -q "^test result: ok" target/test-output.log && ! rg -q "test result: FAILED" target/test-output.log && echo PASS || echo FAIL` — FACT PASS/FAIL (compile failure yields no ok-summary line → FAIL)
  - `cargo xtask build-guests --check` — FACT clean
- Exit condition: all three PASS/clean. Falsified if any native arm's body changed (git diff of the native blocks must be additive-cfg only, apart from `offset_polygons`'s new parameter pass-through) or if `host_batch.rs`'s inline `offset-request` lacks `arc-tolerance-mm`.

### Step 4: End-to-end bridge proof — test guest + integration tests (AC-3, AC-4, AC-N1)

- Task IDs: `DEV-094`
- Objective: prove the arms cross the real boundary: new `sdk-host-bridge-guest` calls `raycast_z_down`, `object_bounds`, `now_us`, `offset_polygons`, `clip_polygons`, `simplify_polygon`, `surface_normal_at` inside a prepass stage and encodes observable results in its diagnostic messages; new integration tests assert the values (positive) and the loud unknown-object failure (negative, config key `bridge_probe_object`).
- Precondition: Step 3 exit met; guests fresh.
- Postcondition: `host_bridge_roundtrip` and `host_bridge_unknown_object_errs` tests pass under the real WASM dispatch path.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-runtime/tests/integration/prepass_diagnostic_roundtrip_tdd.rs` — whole file (harness to mirror: `cube_mesh` builds object id `"cube"` with a top plate; `blackboard_with_layer_plan`)
  - `crates/slicer-wasm-host/test-guests/sdk-support-diagnostic-guest/` — both files (guest scaffolding template incl. `[workspace]` sentinel)
  - `xtask/src/build_guests.rs` — `discover_guests` only (≈ lines 108–290)
- Files allowed to edit (at most 3):
  - `crates/slicer-wasm-host/test-guests/sdk-host-bridge-guest/` (new: `Cargo.toml` + `src/lib.rs`, counted as one new-guest edit)
  - `crates/slicer-runtime/tests/integration/host_bridge_roundtrip_tdd.rs` (new)
  - `crates/slicer-runtime/tests/integration/main.rs` (one `mod host_bridge_roundtrip_tdd;` line — S7 registration; without it the binary silently compiles zero of these tests and reports ok)
- Files explicitly out of bounds: `crates/slicer-sdk/**` (frozen this step), `modules/core-modules/**`, docs.
- Expected sub-agent dispatches:
  - Question: rebuild guests and run the two new tests; scope: `cargo xtask build-guests && mkdir -p target && cargo test -p slicer-runtime --test integration -- host_bridge 2>&1 | tee target/test-output.log`; return: `FACT pass/fail` + ≤20-line SNIPPETS on failure
- Context cost: `M`
- Authoritative docs:
  - `docs/05_module_sdk.md` — §Host Service Wrappers snippet only (call shapes)
- OrcaSlicer refs: none.
- Verification:
  - `mkdir -p target && cargo test -p slicer-runtime --test integration -- host_bridge 2>&1 | tee target/test-output.log && rg -q "^test result: ok" target/test-output.log && rg -q "host_bridge_roundtrip_tdd::" target/test-output.log` — FACT pass/fail (AC-3, AC-4, AC-N1; assertions per the AC text: raycast hit 10.0 ±1e-4, offset width 12.0 mm ±0.05, `now_us` monotonic pair, unknown-object dispatch error; the name-grep guards against the zero-tests-filtered false green)
  - `rg -q "mod host_bridge_roundtrip_tdd;" crates/slicer-runtime/tests/integration/main.rs` — FACT (S7 wiring)
- Exit condition: both PASS with a nonzero test count in the log (a `0 passed; 0 failed` line falsifies the registration). TDD note: authoring the integration test first against the Step-2 tree must fail (raycast reports `None`, `now_us` traps) — capture that red run in the note before Step 3's arms make it green, if steps are executed by one worker in sequence; otherwise assert red by temporarily targeting a stale guest build only if cheap, else skip (the pre-fix impossibility is documented in DEV-094 itself).

### Step 5: classic-perimeters call-site migration (AC-5, AC-6)

- Task IDs: `DEV-094`
- Objective: move the 8 `offset` sites onto `slicer_sdk::host::offset_polygons(…, self.perimeter_arc_tolerance)` and the 4 `difference_ex` sites onto `slicer_sdk::host::clip_polygons(…, ClipOperation::Difference)`; trim the `use slicer_core::polygon_ops` list to `{offset2_ex, opening_ex, remove_small_and_small_holes, OffsetJoinType (aliased for the retained calls)}`.
- Precondition: Step 4 exit met (bridge proven before the hot consumer rides it — ADR-0049 one-module-at-a-time).
- Postcondition: AC-5 greps pass; guests rebuilt + `--check` clean; perimeter parity fixtures green with zero re-records.
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/classic-perimeters/src/lib.rs` — windows only: 25–45 (imports), 475–505, 676–795, 960–990, 1040–1065, 1140–1155, 1200–1215 (the twelve call sites; line anchors verified 2026-08-07 — re-locate by symbol if drifted: `run_perimeters` inset loop, gap collection, thin-wall `opening_ex` block, gap-fill filter block, infill inset, fallback inset loop)
  - `crates/slicer-sdk/src/host.rs` — `offset_polygons`/`clip_polygons` signatures only
- Files allowed to edit (at most 3):
  - `modules/core-modules/classic-perimeters/src/lib.rs`
- Files explicitly out of bounds: `slicer-core` (`polygon_ops` and `top_surface_split` are untouchable — the through-slicer-core path stays in-guest by decision), `crates/slicer-sdk/**`, fixtures under `crates/slicer-runtime/tests/**` (zero re-record is the invariant).
- Expected sub-agent dispatches:
  - Question: rebuild guests, then run the parity suites; scope: `cargo xtask build-guests && mkdir -p target && cargo test -p slicer-runtime --test integration -- perimeter_parity gap_fill_emission classic 2>&1 | tee target/test-output.log`; return: `FACT pass/fail` + failing-fixture names on failure
- Context cost: `M`
- Authoritative docs:
  - `docs/adr/0049-batched-host-services-over-threaded-guests.md` — §Consequences (adoption discipline, marshalling caveat)
- OrcaSlicer refs: none.
- Verification:
  - AC-5 compound command from `packet.spec.md` — FACT PASS/fail
  - `mkdir -p target && cargo test -p slicer-runtime --test integration -- perimeter_parity gap_fill_emission 2>&1 | tee target/test-output.log && rg -q "^test result: ok" target/test-output.log` — FACT pass/fail (AC-6)
  - `cargo xtask build-guests --check` — FACT clean
- Exit condition: all PASS/clean AND `git status --porcelain` shows no fixture-file changes. Falsified by any fixture re-record or any surviving `polygon_ops::offset(`/`difference_ex(` call. Per Test Discipline: a red fixture means the migration is wrong (an argument changed) — fix the call, never the fixture.

### Step 6: Post-migration evidence capture + comparison

- Task IDs: `DEV-094`
- Objective: repeat Step 1's capture on the migrated tree; compute the per-scope guest-fuel delta for `com.core.classic-perimeters` (and the support-planner rows), the profiling-off wall-clock delta, and apply the design.md §Risks decision rule.
- Precondition: Step 5 exit met; same machine, release profile, same model as Step 1.
- Postcondition: completion note holds the before/after table (fuel rows, wall-clock medians, spread) and the decision-rule outcome (keep migration | revert call sites); AC-7 command passes.
- Files allowed to read: none directly (dispatched runs only).
- Files allowed to edit (at most 3): none — unless the decision rule fires, in which case `modules/core-modules/classic-perimeters/src/lib.rs` (revert of Step 5) is the single permitted edit, followed by guest rebuild and a Step 5 verification re-run.
- Files explicitly out of bounds: everything else.
- Expected sub-agent dispatches:
  - Question: AC-7 command chain (slice `--profile`, `profile --from … --json`, greps) + return both modules' scope rows; scope: workspace root; return: `FACT` (≤5 lines)
  - Question: time 3 profiling-off release slices, return timings + median; scope: workspace root; return: `FACT`
- Context cost: `S`
- Authoritative docs:
  - `docs/adr/0055-fuel-based-module-profiling.md` — direct (fuel answers "did this get cheaper", wall-clock answers "by how many milliseconds"; wall-clock under `--profile` is indicative only)
- OrcaSlicer refs: none.
- Verification:
  - AC-7 command from `packet.spec.md` — FACT PASS/fail
- Exit condition: AC-7 PASS and the note contains the full comparison table with the spread statement. Falsified if any "after" number is quoted without its "before" counterpart, or if a wall-clock claim comes from a `--profile` run.

### Step 7: Docs — DEV-094 closure, ADR-0055 amendment, docs/05 caveat (AC-8, AC-9) + packet gates

- Task IDs: `DEV-094`
- Objective: record the evidence and retire the stale prose: DEV-094 Status → `Closed — <date>: …` naming the seven bridged wrappers, the classic-perimeters migration (or its evidence-driven revert), the support-planner verification, and the DEV-093 caveat; append the ADR-0055 `## Amendment` with the Step 1/6 table answering the in-guest-vs-host-native question; rewrite the docs/05 §Host Service Wrappers geometry comment (drop "the host bridge is not wired", state "bridged to the host" on wasm32, and fix the `host::offset_polygons` example to the new 4-arg arity).
- Precondition: Step 6 exit met.
- Postcondition: AC-8 and AC-9 greps pass; all packet gates green.
- Files allowed to read, with ranges when over 300 lines:
  - `docs/DEVIATION_LOG.md` — the `^\| DEV-094` line only (delegate the read; single row)
  - `docs/adr/0055-fuel-based-module-profiling.md` — whole file (127 lines)
  - `docs/05_module_sdk.md` — §Host Service Wrappers window only (locate by `rg -n 'Host Service Wrappers' docs/05_module_sdk.md`)
- Files allowed to edit (at most 3):
  - `docs/DEVIATION_LOG.md` (DEV-094 row only)
  - `docs/adr/0055-fuel-based-module-profiling.md` (append-only amendment)
  - `docs/05_module_sdk.md` (§Host Service Wrappers only)
- Files explicitly out of bounds: every other doc, every other deviation row, `docs/adr/0049-*` (its amendment is self-dated; do not touch), `CONTEXT.md`, `docs/07_implementation_status.md`.
- Expected sub-agent dispatches:
  - Question: run the three packet gates (`cargo check --workspace --all-targets`; `cargo clippy --workspace --all-targets -- -D warnings`; `cargo xtask build-guests --check`); scope: workspace; return: `FACT pass/fail` each
- Context cost: `S`
- Authoritative docs:
  - `docs/DEVIATION_LOG.md` header block — direct (closure convention: Status cell must begin with `Closed`)
- OrcaSlicer refs: none.
- Verification:
  - `sh -c 'rg -q "^\| DEV-094 \|.*\| Closed" docs/DEVIATION_LOG.md && rg -q "^## Amendment" docs/adr/0055-fuel-based-module-profiling.md && rg -q "in-guest" docs/adr/0055-fuel-based-module-profiling.md && echo PASS'` — FACT PASS/fail (AC-8)
  - `sh -c '! rg -q "the host bridge is not wired" docs/05_module_sdk.md && rg -q "bridged to the host" docs/05_module_sdk.md && echo PASS'` — FACT PASS/fail (AC-9)
  - `cargo check --workspace --all-targets` and `cargo clippy --workspace --all-targets -- -D warnings` — FACT pass/fail
- Exit condition: all PASS. Falsified if the amendment quotes any unmeasured number, omits the spread statement, or the DEV-094 row is rewritten beyond its own cells.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | S | dispatched runs only |
| Step 2 | S | WIT + 2 host fns |
| Step 3 | M | SDK arms + record fallout + guest rebuild |
| Step 4 | M | new guest + integration tests |
| Step 5 | M | 12 call sites, windowed reads |
| Step 6 | S | dispatched runs + decision rule |
| Step 7 | S | 3 doc edits + gates |

Aggregate M. No step is L.

## Packet Completion Gate

- All steps and exits complete; Step 6's decision-rule outcome recorded either way.
- Every pipe-suffixed AC command (AC-1 … AC-9, AC-N1) returns PASS.
- `docs/07_implementation_status.md` is NOT updated by this packet (no TASK rows exist for the program; the plan's Backlog anchoring [FWD] defers the workstream rows, and the file is frozen while the parallel 194–199 session is active). The queue row in `docs/specs/multi-edition-distribution-plan.md` is updated by the orchestrator, not this packet.
- No reopened/superseded packet transitions (nothing absorbed; the support-planner correction is a plan-assumption note in `requirements.md`, not a packet supersession).
- `packet.spec.md` ready for `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed AC and packet-level gate command; read verdicts from `target/test-output.log`, never from re-runs.
- Record remaining packet-local risk (expected residuals: `offset2_ex`/`opening_ex`/`split_top_surfaces` still in-guest with their measured share; DEV-093 caveat on whole-slice fuel totals; marshalling ledger if the decision rule fired).
- Confirm context stayed at or below 150k standard, or at/below 300k only with a logged swarm ESCALATION; otherwise record a packet-authoring lesson.

All `cargo check`, `cargo clippy`, and `cargo test` invocations in gate and verification commands must use `--all-targets` where applicable so test, bench, and example targets compile (`cargo test --test <bin>` selects its own target explicitly).
