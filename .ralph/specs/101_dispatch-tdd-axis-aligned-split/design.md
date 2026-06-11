# Design: 101_dispatch-tdd-axis-aligned-split

## Controlling Code Paths

- Primary code path: `crates/slicer-runtime/tests/contract/dispatch_tdd.rs` (4,875 LOC) is the source; the eight new sibling files in the same directory are the destinations. `crates/slicer-runtime/tests/contract/main.rs` is the registration site — the file declares each axis-test file as `pub mod dispatch_<axis>_tdd;`. After the migration, the `pub mod dispatch_tdd;` declaration is removed and the source file is deleted.
- Neighboring tests or fixtures: `crates/slicer-runtime/tests/common/dispatch_fixture.rs` and `crates/slicer-runtime/tests/common/ir_builders.rs` (created by packet 100) are imported by every new file via `use crate::common::{dispatch_fixture::DispatchFixture, ir_builders};`. The existing `TestModuleBundle`, `run_layer_and_commit*`, `*_input` projectors in `tests/common/mod.rs` remain unchanged and may still be referenced by the migrated tests when convenient (the fixture exposes them).
- OrcaSlicer comparison surface: not applicable.

## Architecture Constraints

- The eight axis files MUST mirror the trait-and-Claim structure documented in the ADR-0007 amendment: per-runner protocol on one file, per-stage-output IR contracts on four files, per-Claim concerns (config, identity, pathopt, prepass harvest) on three files. The amendment explicitly enumerates this split.
- `dispatch_protocol_tdd.rs` MUST use `slicer_schema::export_for_stage_id` exclusively for export-name lookup. ADR-0006 forbids any hardcoded stage→export table in dispatcher tests; the migration must preserve this behavior and may not reintroduce a parallel table even as a "convenience" inline `const`.
- No file in the new set may instantiate a `WasmRuntimeDispatcher`, `Blackboard`, or `LayerArena` directly. All such state lives inside the `DispatchFixture` per the ADR-0007 amendment's locked invariant.
- The eight files share no scratch state with each other and may run in any order. `cargo test -p slicer-runtime --test contract` parallelises across them.
- The wasm-staleness snippet does NOT apply: this packet edits only files under `crates/slicer-runtime/tests/contract/` and `tests/contract/main.rs`. No path under `wit/`, `slicer-macros/`, `slicer-sdk/`, `slicer-ir/`, `slicer-schema/`, `modules/core-modules/`, or `slicer-runtime/test-guests/` source is touched. Pre-built test-guest `.wasm` artifacts are loaded but not modified.
- The coord-system snippet does NOT apply: every test's polygon and point coordinates are preserved bit-identically across migration. Where `ir_builders` constructs the default 1mm-square polygon it uses the same `Point2 { x: 10_000, y: 10_000 }` values the legacy `make_slice_ir` used.

## Code Change Surface

- Selected approach: per-axis migration in nine sequential steps. Step 1 creates eight empty skeletons + the `pub mod` lines in `main.rs`. Steps 2 through 9 each fully populate one of the eight files end-to-end and run its narrow per-axis test command before moving on. Step 10 deletes `dispatch_tdd.rs` and removes its `pub mod` declaration. This sequencing keeps each step at `S` or `M` context cost and guarantees `cargo check --workspace --all-targets` is green at every step boundary.
- Exact functions, traits, manifests, tests, or fixtures expected to change:
  - NEW: 8 files in `crates/slicer-runtime/tests/contract/`, each one starting with `use crate::common::{dispatch_fixture::DispatchFixture, ir_builders};` plus selected `use` items from `slicer_ir`, `slicer_runtime`, and `slicer_wasm_host` as needed. Per-axis test counts are approximate (subject to dispatcher-side `LOCATIONS` dispatch confirmation in Step 1): protocol ≈ 10, config ≈ 5, infill output ≈ 20, perimeter output ≈ 15, support output ≈ 5, pathopt ≈ 25, identity ≈ 15, prepass harvest ≈ 5.
  - EDIT: `crates/slicer-runtime/tests/contract/main.rs` — add 8 `pub mod` lines in Step 1; remove the `pub mod dispatch_tdd;` line in Step 10.
  - DELETE: `crates/slicer-runtime/tests/contract/dispatch_tdd.rs` — in Step 10, after every test has moved.
- Rejected alternatives:
  - **All-at-once migration in a single step** — would be `L` context cost and cannot honor the discipline rule. Per-step migration is the only viable form.
  - **Split by Runner (4 files: layer / prepass / finalization / postpass)** — was considered during grilling. The user chose split by Concern because output-cardinality tests for a given stage form a clearer Claim than a per-runner grouping. The ADR-0007 amendment locks the Concern split.
  - **Hybrid: keep `dispatch_tdd.rs` for protocol tests; split only the heavy IR contracts** — was considered during grilling. Rejected because `dispatch_tdd.rs` would still be ≈ 1,500 LOC; the worst part survives.
  - **Delete the `make_*` helpers in a follow-up packet after the split** — rejected because each helper's last caller goes away during the migration. Keeping them after deletion of `dispatch_tdd.rs` would leave dead code; the helpers go with the file.

## Files in Scope (read + edit)

- `crates/slicer-runtime/tests/contract/main.rs` — role: register the eight new files; expected change: 8 `pub mod` lines added (Step 1), 1 line removed (Step 10).
- `crates/slicer-runtime/tests/contract/dispatch_tdd.rs` — role: source of every migrated test; expected change: every test body relocated to a new file, then the file deleted.
- `crates/slicer-runtime/tests/contract/dispatch_protocol_tdd.rs` — role: cross-runner protocol axis; expected change: NEW file with all protocol tests (≈ 10 tests).
- `crates/slicer-runtime/tests/contract/dispatch_config_tdd.rs` — role: config wiring axis; NEW (≈ 5 tests).
- `crates/slicer-runtime/tests/contract/dispatch_infill_output_tdd.rs` — role: Infill IR contract axis; NEW (≈ 20 tests).
- `crates/slicer-runtime/tests/contract/dispatch_perimeter_output_tdd.rs` — role: Perimeter IR contract axis; NEW (≈ 15 tests).
- `crates/slicer-runtime/tests/contract/dispatch_support_output_tdd.rs` — role: Support IR contract axis; NEW (≈ 5 tests).
- `crates/slicer-runtime/tests/contract/dispatch_pathopt_tdd.rs` — role: PathOptimization overrides axis; NEW (≈ 25 tests).
- `crates/slicer-runtime/tests/contract/dispatch_identity_tdd.rs` — role: region-identity preservation axis; NEW (≈ 15 tests).
- `crates/slicer-runtime/tests/contract/dispatch_prepass_harvest_tdd.rs` — role: Global-layer harvest axis; NEW (≈ 5 tests).

Ten files. Justified: the packet's nature is the split. Each per-axis step touches at most 2 files (the new axis file + `main.rs` during Step 1; the new axis file alone during Steps 2–9; `dispatch_tdd.rs` and `main.rs` during Step 10).

## Read-Only Context

- `crates/slicer-runtime/tests/contract/dispatch_tdd.rs` — 4,875 lines. **Never load in full.** Per-axis line ranges:
  - Protocol: ≈ lines 250–700 (the export-name + per-runner success / error / pool / `MissingComponent` cluster)
  - Config wiring: ≈ lines 700–1100 (the `ConfigView` cluster)
  - Output commitment subsections (the comments mark these as `Section H. ...`, `Section M. ...`, etc.; rely on a LOCATIONS dispatch to confirm exact ranges before each step)
- `crates/slicer-runtime/tests/contract/main.rs` — small (< 50 lines); read in full.
- `crates/slicer-runtime/tests/common/dispatch_fixture.rs` and `ir_builders.rs` — created by packet 100; the implementer reads each in full to know which builder methods exist before migrating a test.
- `docs/adr/0007-compiled-module-static-live-split.md` — read the amendment section (lines 113+).
- `docs/adr/0005-runner-traits-in-slicer-wasm-host.md` — read lines 42–77 only.
- `docs/adr/0006-export-for-stage-id-sole-lookup.md` — full ≈ 90 lines. Required reading before Step 2 (protocol axis).

## Out-of-Bounds Files

- `OrcaSlicerDocumented/**` — delegate any parity checks (none expected for this packet).
- `target/`, `Cargo.lock`, generated code — never load.
- Any vendored deps — never load.
- `crates/slicer-wasm-host/tests/common/**` — must not be edited (AC-N2 invariant).
- `crates/slicer-wasm-host/src/**` — read-only context if needed; delegate LOCATIONS or SNIPPETS for trait signatures.
- `crates/slicer-runtime/src/**` — read-only; delegate symbol lookups.
- Any test bucket outside `tests/contract/` — `tests/e2e/`, `tests/executor/`, `tests/unit/`, `tests/integration/` are out of bounds.

## Expected Sub-Agent Dispatches

- "In `crates/slicer-runtime/tests/contract/dispatch_tdd.rs`, return `LOCATIONS` (≤ 30 entries: `line: test_fn_name`) of every test that belongs to axis `<axis-name>`. Use the section-header comments (`// ── X. <topic> ──`) and the test bodies' first three lines to classify; do NOT return test bodies." — one dispatch per axis migration step.
- "Return `SNIPPETS` of test `<test_fn_name>` from `dispatch_tdd.rs:<line_start>-<line_end>`; one test per dispatch; never multiple bodies in one return." — per-test during migration when the dispatcher needs to confirm assertion content.
- "Run `cargo check --workspace --all-targets`; return `FACT pass/fail`." — after every step.
- "Run `cargo test -p slicer-runtime --test contract dispatch_<axis>_tdd::`; return `FACT pass/fail`; on fail return `SNIPPETS` ≤ 20 lines around the first failing assertion." — per-axis migration step verification.
- "Run `cargo test -p slicer-runtime --test contract 2>&1 | tee target/test-output.log`, return `FACT pass/fail` and the line matching `^test result: ok\.`." — Step 10 final gate.
- "Run `! grep -rE 'make_compiled_module|make_slice_ir|make_perimeter_ir|make_wall_loop|make_loaded_module|make_object' crates/slicer-runtime/tests/contract/dispatch_*_tdd.rs`; return `FACT pass/fail`. Pass = no matches." — Step 10 AC-4 verification.
- "Run `grep -cE '^\\s*#\\[ignore\\]' crates/slicer-runtime/tests/contract/dispatch_tdd.rs` BEFORE Step 1; record the number. Then after Step 9 run `grep -cE '^\\s*#\\[ignore\\]' crates/slicer-runtime/tests/contract/dispatch_*_tdd.rs` (excluding the original) and assert equality." — AC-N1.
- "Run `git status --porcelain crates/slicer-wasm-host/tests/common/`; return `FACT pass/fail`. Pass = empty output." — AC-N2 at the end.

## Data and Contract Notes

- IR or manifest contracts touched: none. Migrated tests instantiate the same IR shapes via `ir_builders` instead of `make_*`; the IR struct fields, the WIT boundary, and the runner trait inputs/outputs are unchanged.
- WIT boundary considerations: none. `dispatch_protocol_tdd.rs` uses `slicer_schema::export_for_stage_id` (ADR-0006); no dispatcher-side parallel table is introduced.
- Determinism or scheduler constraints: every test that today asserts a deterministic count or ordering continues to do so. The default `ir_builders` synthetic IDs (`obj-0`, `obj-1`, …) match the legacy `make_*` IDs exactly, so identity-sensitive tests in `dispatch_identity_tdd.rs` see bit-identical inputs.

## Locked Assumptions and Invariants

- `crates/slicer-wasm-host/tests/common/` is byte-identical at packet end vs packet start (AC-N2).
- Every migrated test preserves its observable contract: same `assert*!` content, same input values, same `#[ignore]` markers (AC-N1).
- The eight new files use `DispatchFixture` and `ir_builders` exclusively for module / arena / Blackboard / IR setup; no legacy `make_*` call survives in the new files (AC-4).
- `dispatch_tdd.rs` is deleted only after every test it once held has moved; the deletion step is last and is preceded by a successful run of `cargo test -p slicer-runtime --test contract` to confirm zero coverage loss.
- Per the ADR-0007 amendment: per-runner methods only; two distinct constructors per IR type; no generic `run::<R>` method; no type-state fixture parameter. The amendment's locked conventions govern every migrated test.

## Risks and Tradeoffs

- The packet touches 10 files, which is far above the "≤ 3 primary files" template target. Justification: a file split is irreducibly multi-file work. Per-step file count stays at 1 or 2.
- Per-axis test classification (which test belongs to which axis) is the highest-risk decision in the packet. A test misclassified into the wrong file produces an immediate AC-3 / per-axis test failure (the test runs against a fixture it doesn't fit and panics), so the cost of misclassification is bounded but visible. Step 1's LOCATIONS dispatches per axis serve as a sanity check before any migration begins.
- The `make_*` helpers and `dispatch_tdd.rs` are deleted together in Step 10. If a not-yet-migrated test still references a helper at that point, Step 10's `cargo check --workspace --all-targets` fails immediately. This is the intended forcing function — the implementer cannot delete the file until the migration is complete.
- The architecture exploration estimated ≈ 99 tests across the eight axes; the actual count is confirmed at Step 1 via a LOCATIONS dispatch enumerating every `#[test]` and `#[ignore]` line in `dispatch_tdd.rs`. The estimate is allowed to drift modestly (±10 tests) without changing the packet plan; a larger drift would warrant a packet revision.

## Context Cost Estimate

- Aggregate (sum across all steps): `M` (6 S-steps + 4 M-steps; the M steps cluster around the biggest axes — infill output, perimeter output, pathopt, identity).
- Largest single step: `M` (Step 7 — PathOpt overrides; ≈ 25 tests across tool changes / z-hops / retracts / unretracts / deferred travel / comments / raw).
- Highest-risk dispatch: the per-axis `LOCATIONS` dispatch at the start of each migration step. If the dispatcher returns full test bodies (`SNIPPETS`) instead of `LOCATIONS` (line + name), per-step budget can blow on a single dispatch. Required return format: `LOCATIONS` with at most one line of context per match; re-dispatch with tighter format if the first return exceeds 30 entries or includes test bodies.

## Open Questions

`None.`
