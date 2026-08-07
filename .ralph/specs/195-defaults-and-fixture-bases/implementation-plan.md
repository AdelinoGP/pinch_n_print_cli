# Implementation Plan: 195-defaults-and-fixture-bases

## Execution Rules

- Work one atomic step at a time; map every step to grouped task IDs.
- Use TDD, then implementation, then the narrowest falsifying validation.
- Every field below is a context-budget contract and must be filled independently; never write "see Step 1".

## Steps

### Step 1: Audit — re-derive the class lists from the packet-194 gate

- Task IDs: `TASK-317`
- Objective: Confirm (or amend, per `design.md` §Open Questions) the classification: class (a) `SliceRunOptions`; class (b) `PrintEntity`, `WallLoop`, `OrderedEntityView`; class (c) `PipelineConfig`; dropped `Diagnostic` (×2), `DeferredRetract`, `DeferredTravelMove`.
- Precondition: packet 194 implemented; `cargo xtask check-literals --report` runs.
- Postcondition: a ≤ 15-line audit note (recorded in the step's completion output, not a new file) listing per-type violation counts and the confirmed classification; any newly admitted class-(a) type carries an explicit §3.6/§5 pass rationale; any type failing the criteria is listed as "sweep-time waiver" for the completion report.
- Files allowed to read, with ranges when over 300 lines:
  - `docs/specs/_OLD/default-builder-migration.md` - lines 176-198 (§3.6) and 308-330 (§5 intro) only
- Files allowed to edit (at most 3): none (read-only discovery step).
- Files explicitly out of bounds:
  - every test file named by the report (counts only; never open them)
- Blast-radius discipline: not applicable — no edits.
- Expected sub-agent dispatches:
  - Question: run `cargo xtask check-literals --report`; return the distinct watched type names in violations plus, for `SliceRunOptions`, `PrintEntity`, `WallLoop`, `OrderedEntityView`, `PipelineConfig`, `Diagnostic`, `DeferredRetract`, `DeferredTravelMove`, the violation count and ≤ 3 sample `file:line` entries each; scope: repo root; return: `LOCATIONS` ≤ 20 entries + FACT counts.
- Context cost: `S`
- Authoritative docs:
  - `docs/specs/struct-literal-churn-gate-plan.md` - locked decision 3 (class definitions)
- OrcaSlicer refs: none.
- Verification:
  - `cargo xtask check-literals --report >/dev/null 2>&1; echo "exit=$?"` - FACT (expect `exit=0`, proving the dependency is in place)
- Exit condition: classification unconfirmed, or the gate binary missing (dependency-order violation — stop and report), fails the step.

### Step 2: Class (a) — `impl Default for SliceRunOptions` (TDD)

- Task IDs: `TASK-317`
- Objective: Add the manual quiet-baseline `Default` impl and its unit test file.
- Precondition: Step 1 confirmed `SliceRunOptions` in class (a); no `impl Default for SliceRunOptions` exists (verified 2026-08-07).
- Postcondition: `slice_run_options_default_tdd.rs` asserts every AC-1 field value (mesh schema version pinned via `CURRENT_MESH_IR_SCHEMA_VERSION`, empty `objects`, all `None`s, all `false`s, empty collections) and contains one FRU usage `SliceRunOptions { profile: true, ..Default::default() }`; the impl's rustdoc states the `progress_events: false` divergence from the CLI default.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-runtime/src/run.rs` - the `SliceRunOptions` definition window only (symbol-anchored; file > 300 lines)
  - `crates/slicer-runtime/tests/unit/main.rs` - first 40 lines (mod-line style)
- Files allowed to edit (at most 3):
  - `crates/slicer-runtime/src/run.rs`
  - `crates/slicer-runtime/tests/unit/slice_run_options_default_tdd.rs` (new)
  - `crates/slicer-runtime/tests/unit/main.rs` (one `mod slice_run_options_default_tdd;` line)
- Files explicitly out of bounds:
  - `crates/pnp-cli/src/main.rs` (production site; unaffected — it sets every field explicitly), all other test files
- Blast-radius discipline: adding a `Default` impl changes no struct shape — zero literal blast radius; the workspace compile gate at close is the safety net. No `LOCATIONS` dispatch needed.
- Expected sub-agent dispatches: none.
- Context cost: `S`
- Authoritative docs:
  - `docs/specs/_OLD/default-builder-migration.md` - §5 intro only (Bucket-B manual-impl pattern)
- OrcaSlicer refs: none.
- Verification:
  - `mkdir -p target && cargo test -p slicer-runtime --test unit slice_run_options_default 2>&1 | tee target/test-output.log | grep -E '^test result'` - FACT pass/fail
- Exit condition: the test failing any field assertion, the FRU line not compiling, or the rustdoc omitting the CLI-default divergence fails the step.

### Step 3: Class (b) — fixture bases in `sdk::test_support` + guest rebuild (TDD)

- Task IDs: `TASK-317`
- Objective: Add `print_entity_base`, `wall_loop_base`, `ordered_entity_view_base` to `fixtures.rs` with the exact `design.md` field values, plus the gated test file and its manifest entry; rebuild guests.
- Precondition: Step 1 confirmed the class-(b) list; no fn named `*_base` exists in `fixtures.rs` (verified 2026-08-07).
- Postcondition: AC-2/3/4 assertions pass, including one FRU composition per base (`PrintEntity { topo_order: 7, ..print_entity_base(role) }` style); `cargo xtask build-guests --check` is clean after rebuild.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-sdk/src/test_support/fixtures.rs` - the existing `print_entity`..`seam_candidate` window (~lines 890-1000) and imports header only (file is 1136 lines)
  - `crates/slicer-ir/src/slice_ir.rs` - definition windows of `PrintEntity`, `WallLoop`, `LoopType`, `WallBoundaryType`, `WidthProfile`, `RegionKey` only
  - `crates/slicer-sdk/src/views.rs` - the `OrderedEntityView` definition window only
  - `crates/slicer-sdk/Cargo.toml` - the `[[test]]` block region
- Files allowed to edit (at most 3):
  - `crates/slicer-sdk/src/test_support/fixtures.rs`
  - `crates/slicer-sdk/tests/test_support_fixture_bases_tdd.rs` (new)
  - `crates/slicer-sdk/Cargo.toml` (one `[[test]]` entry with `required-features = ["test"]`)
- Files explicitly out of bounds:
  - `crates/slicer-sdk/tests/layer_module_tdd.rs` (its 3 `OrderedEntityView` literals are sweep-packet territory), `modules/core-modules/**`
- Blast-radius discipline: additive fns only — no struct/constant change, no literal blast radius.
- Expected sub-agent dispatches:
  - Question: run `cargo xtask build-guests --check`; if `STALE:`, run `cargo xtask build-guests` then re-run `--check`; scope: repo root; return: `FACT` clean/stale+rebuilt.
- Context cost: `M`
- Authoritative docs:
  - `docs/specs/struct-literal-churn-gate-plan.md` - locked decision 3(b)
  - `CLAUDE.md` §Guest WASM Staleness - the `--check` obligation
- OrcaSlicer refs: none.
- Verification:
  - `mkdir -p target && cargo test -p slicer-sdk --test test_support_fixture_bases_tdd 2>&1 | tee target/test-output.log | grep -E '^test result'` - FACT pass/fail
  - `cargo xtask build-guests --check; echo "exit=$?"` - FACT (expect `exit=0` after rebuild)
- Exit condition: any AC-2/3/4 assertion failing, the `[[test]]` gating missing (test silently compiles to zero tests — check the reported test count is ≥ 6), or a stale guest left behind fails the step.

### Step 4: Class (c) — per-crate `pipeline_config_base` helpers

- Task IDs: `TASK-317`
- Objective: Add the `slicer-runtime` `tests/common` helper (waivered single literal) with its smoke test, and the pnp-cli file-local twin.
- Precondition: Steps 1-2 done; `tests/common/mod.rs` opens with `#![allow(dead_code)]` (verified 2026-08-07); no fn named `pipeline_config_base` exists in the tree (verified 2026-08-07).
- Postcondition: `pipeline_config_base_smoke` passes (built from `ExecutionPlan::default()` + the file's Noop runners; asserts `cancel_flag` `None`, empty `wasm_handles`, empty `resolved_configs`); the pnp-cli twin compiles under `#[allow(dead_code)]` with a `// exhaustive:` waiver on its literal and a comment noting sweep packet 197 removes the allow.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-runtime/src/pipeline.rs` - `PipelineConfig`/`PipelineStageRunners` definition windows only
  - `crates/slicer-runtime/tests/integration/pipeline_tdd.rs` - the Noop-runner block (~lines 85-150) and one existing `PipelineConfig` literal only (file > 300 lines)
  - `crates/pnp-cli/tests/e2e_integration_tdd.rs` - imports block and one existing `PipelineConfig` literal only
- Files allowed to edit (at most 3):
  - `crates/slicer-runtime/tests/common/mod.rs`
  - `crates/slicer-runtime/tests/integration/pipeline_tdd.rs` (append the smoke test only; convert no existing literal)
  - `crates/pnp-cli/tests/e2e_integration_tdd.rs` (add the helper fn only; convert no existing literal)
- Files explicitly out of bounds:
  - every other `PipelineConfig` call site (`perimeter_harness.rs`, remaining integration/contract tests) — sweep territory
- Blast-radius discipline: additive fns only — no struct/constant change, no literal blast radius.
- Expected sub-agent dispatches: none.
- Context cost: `M`
- Authoritative docs:
  - `docs/specs/struct-literal-churn-gate-plan.md` - locked decision 3(c)
- OrcaSlicer refs: none.
- Verification:
  - `mkdir -p target && cargo test -p slicer-runtime --test integration pipeline_config_base_smoke 2>&1 | tee target/test-output.log | grep -E '^test result'` - FACT pass/fail
  - `rg -q 'fn pipeline_config_base' crates/pnp-cli/tests/e2e_integration_tdd.rs && rg -q '// exhaustive:' crates/pnp-cli/tests/e2e_integration_tdd.rs && cargo check -p pnp-cli --tests && echo OK` - FACT pass/fail
- Exit condition: the smoke test failing, either helper's literal lacking its waiver, or any pre-existing literal in the two test files having been converted (diff must show additions only in those regions) fails the step.

### Step 5: ADR-0054 / ADR-0004 addenda + locator header touch

- Task IDs: `TASK-317`
- Objective: Record the "single IR-fixture home" policy in both ADRs and align the `pnp-cli-locator` header rustdoc.
- Precondition: Steps 3-4 landed (the amendments describe shipped surface, not intent).
- Postcondition: ADR-0054 gains an amendment section (dated, packet-195-attributed) superseding its Decision item 3 "guest-side surface" wording: `slicer_sdk::test_support` is the single IR-fixture home for host- and guest-side tests, host crates consume it via a `slicer-sdk` dev-dep with `feature = "test"` (added by sweep packets), the locator/test_support disjointness stands, and `WallLoopBuilder` (`crates/slicer-runtime/tests/common/ir_builders.rs`) is a recorded consolidation target. ADR-0004 gains a matching short amendment (scope extension; guests still never enable `feature = "test"`). The locator's header rustdoc (its "lives guest-side in `slicer_sdk::test_support`" and rule-3 "guest-side test support" sentences) now names `slicer_sdk::test_support` the IR-fixture home for host and guest tests while keeping the locator binary-location-only; both amendments and the header contain the anchor phrases AC-7 greps.
- Files allowed to read, with ranges when over 300 lines:
  - `docs/adr/0054-host-side-test-support-crate.md` - full (186 lines)
  - `docs/adr/0004-test-support-lives-in-slicer-sdk.md` - full (72 lines)
  - `crates/pnp-cli-locator/src/lib.rs` - lines 1-30 (header rustdoc only)
- Files allowed to edit (at most 3):
  - `docs/adr/0054-host-side-test-support-crate.md`
  - `docs/adr/0004-test-support-lives-in-slicer-sdk.md`
  - `crates/pnp-cli-locator/src/lib.rs` (header comment only; no code)
- Files explicitly out of bounds:
  - `crates/pnp-cli-locator/src/lib.rs` below the header (the four functions are ADR-locked), `docs/21_data_defaults_and_fixtures.md` (packet 194's page)
- Blast-radius discipline: not applicable — docs and one doc-comment.
- Expected sub-agent dispatches: none.
- Context cost: `S`
- Authoritative docs:
  - `docs/specs/struct-literal-churn-gate-plan.md` - locked decision 3(b) (addendum obligation)
- OrcaSlicer refs: none.
- Verification:
  - `rg -q 'single IR-fixture home' docs/adr/0054-host-side-test-support-crate.md && rg -q 'single IR-fixture home' docs/adr/0004-test-support-lives-in-slicer-sdk.md && rg -q 'IR-fixture home' crates/pnp-cli-locator/src/lib.rs && echo OK` - FACT pass/fail
  - `cargo check -p pnp-cli-locator` - FACT pass/fail (doc-comment edit cannot break code)
- Exit condition: any AC-7 grep failing, an amendment contradicting the locator's std-only/dev-dep-only/four-function constraints, or a non-comment edit to `lib.rs` fails the step.

### Step 6: Close — gate sweep

- Task IDs: `TASK-317`
- Objective: Prove the packet's additions are gate-clean and regression-free.
- Precondition: Steps 1-5 complete.
- Postcondition: AC-8 (new test files violation-free under `check-literals`), AC-9 (`build-guests --check` clean), AC-N1/AC-N2 (no forbidden `Default`s), `cargo check --workspace --all-targets`, and `cargo clippy --workspace --all-targets -- -D warnings` all pass.
- Files allowed to read, with ranges when over 300 lines: none (dispatch-only step).
- Files allowed to edit (at most 3): none (fix-forward edits reopen the owning step).
- Files explicitly out of bounds: all — this step runs commands only.
- Blast-radius discipline: not applicable — no edits.
- Expected sub-agent dispatches:
  - Question: run the six commands in this step's Verification plus AC-N1/AC-N2 from `packet.spec.md`; scope: repo root; return: `FACT` — one PASS/FAIL line per command, ≤ 20 error lines on any failure.
- Context cost: `S`
- Authoritative docs:
  - `CLAUDE.md` §Test Discipline - `--all-targets` gate rule
- OrcaSlicer refs: none.
- Verification:
  - `cargo xtask check-literals crates/slicer-sdk/tests/test_support_fixture_bases_tdd.rs crates/slicer-runtime/tests/unit/slice_run_options_default_tdd.rs; echo "exit=$?"` - FACT (expect `exit=0`)
  - `cargo xtask build-guests --check; echo "exit=$?"` - FACT (expect `exit=0`)
  - `cargo check --workspace --all-targets` - FACT pass/fail (delegated)
  - `cargo clippy --workspace --all-targets -- -D warnings` - FACT pass/fail (delegated)
- Exit condition: any command failing fails the step; a `check-literals` violation inside this packet's new files means a base/helper was authored without its waiver or FRU and reopens the owning step.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | S | audit via dispatch; read-only |
| Step 2 | S | one impl + one test file + one mod line |
| Step 3 | M | three fns + gated test + manifest entry + guest rebuild |
| Step 4 | M | two helpers + smoke test across two crates |
| Step 5 | S | two ADR amendments + header comment |
| Step 6 | S | dispatch-only gate sweep |

Split before activation if aggregate cost exceeds M or any step is L.

## Packet Completion Gate

- All steps and exits complete.
- Every pipe-suffixed AC command returns PASS.
- Update `docs/07_implementation_status.md` through a worker dispatch (register TASK-317 per `task-map.md`), never a full backlog read.
- Reconcile reopened/superseded status transitions: none for this packet (`docs/specs/_OLD/default-builder-migration.md` is historical reference, not a packet to supersede).
- `packet.spec.md` is ready for `status: implemented`; the completion report exports the fixture/helper signatures and the Step-1 audit outcome for packets 196-198.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed AC and packet-level gate command.
- Record remaining packet-local risk (dead-code allow in pnp-cli until packet 197; `WallLoopBuilder` consolidation deferred to sweeps; the two `[FWD]`s in `design.md`).
- Confirm context stayed at or below 150k standard, or at/below 300k only with a logged swarm ESCALATION; otherwise record a packet-authoring lesson.

All `cargo check`, `cargo clippy`, and `cargo test` invocations in gate and verification commands must use `--all-targets` so the test, bench, and example targets compile (workspace-level gates; narrow `--test <file>` runs target their named binaries).
