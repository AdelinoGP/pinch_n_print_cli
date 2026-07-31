# Implementation Plan: 165_cli-binary-locator-extraction

## Execution Rules

- Work one atomic step at a time; map every step to grouped task IDs.
- Use TDD, then implementation, then the narrowest falsifying validation.
- Every field below is a context-budget contract and must be filled independently; never write "see Step 1".
- **Step 0 gate:** before any edit, dispatch the precondition check from `packet.spec.md` §Prerequisites. `BLOCKED` ⇒ stop; packet 162 has not landed and this packet must not run against the pre-162 tree.

## Steps

### Step 1: Author the ADR deciding the host-side test-support home

- Task IDs: `TASK-146d`
- Objective: write `docs/adr/<NNNN>-host-side-test-support-crate.md` at the next free number, recording the `slicer-test-support` decision and the five weighed alternatives per `design.md` §Code Change Surface.
- Precondition: Step 0 gate returned `READY`; the derived ADR number is fresh (derive it in this step, immediately before writing — `ls docs/adr | rg -o '^[0-9]{4}' | sort | tail -1`, +1; a parallel session may have consumed a number since any earlier derivation).
- Postcondition: the ADR file exists; AC-7's command prints `PASS`.
- Files allowed to read, with ranges when over 300 lines:
  - `docs/adr/0004-test-support-lives-in-slicer-sdk.md` (short; read whole)
  - `crates/pnp-cli/Cargo.toml` (short; read whole)
  - `.ralph/specs/162_wit-lifecycle-export-removal/design.md` - §"CLI freshness — three sites, fixed in place" and §"Open Questions" only
- Files allowed to edit (at most 3):
  - `docs/adr/<NNNN>-host-side-test-support-crate.md` (new)
- Files explicitly out of bounds:
  - all `crates/**`, `Cargo.toml` (root) — no code in this step
- Expected sub-agent dispatches:
  - Question: "`ls docs/adr | rg -o '^[0-9]{4}' | sort | tail -1` — report the value"; scope: `docs/adr/`; return: `FACT`
- Context cost: `S`
- Authoritative docs:
  - `docs/specs/adr-0045-per-stage-wit-packages-plan.md` - §"Grounding corrections" items 1/4/6 only (ranged)
- OrcaSlicer refs: none — no parity content.
- Verification:
  - AC-7 command from `packet.spec.md` - FACT PASS/FAIL
- Exit condition: AC-7 prints `PASS`; the ADR names all of: `slicer-test-support`, ADR-0004, packet 78, the `pnp-cli` `test-support`-feature alternative with the `report` feature-unification analysis, and the `xtask` bin-only constraint. Missing any ⇒ step not done.

### Step 2: Create `crates/slicer-test-support` and register it

- Task IDs: `TASK-146d`
- Objective: create the crate (`Cargo.toml` with zero `[dependencies]` + `[lints] workspace = true`; `src/lib.rs` with `workspace_root`, `newest_source_mtime`, `staleness_reason`, `pnp_cli_bin` moved from the post-162 `slicer_cache.rs`, rustdoc citing the Step-1 ADR and the `is_stale` mirror pin) and add the workspace member line to the root `Cargo.toml`.
- Precondition: Step 1 done (the rustdoc cites the ADR by its real number). The moved code is copied from the post-162 `slicer_cache.rs` locator block — behavior-identical, message strings unchanged.
- Postcondition: `cargo check -p slicer-test-support` passes; the three original copies still exist (deleted in Step 3) — AC-2 is expected to FAIL at this point, by design.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-runtime/tests/common/slicer_cache.rs` - locator block only (locate `pnp_cli_bin`, `staleness_reason`, `newest_source_mtime` by name)
  - `xtask/src/build_guests.rs` - `is_stale` fn only (locate by name)
  - `Cargo.toml` (root) - `[workspace] members` list only
- Files allowed to edit (at most 3):
  - `crates/slicer-test-support/Cargo.toml` (new)
  - `crates/slicer-test-support/src/lib.rs` (new)
  - `Cargo.toml` (root — one member line inside the `crates/*` block, before the first `modules/core-modules/` entry; the list is grouped, not alphabetical)
- Files explicitly out of bounds:
  - the three consumer sites (Step 3); `crates/pnp-cli/**`; `xtask/**` beyond the read-only `is_stale` lookup
- Expected sub-agent dispatches:
  - Question: "Run `cargo check -p slicer-test-support`; pass/fail + first 20 error lines on failure"; scope: workspace; return: `FACT` + SNIPPETS ≤20
- Context cost: `S`
- Authoritative docs:
  - `docs/adr/<NNNN>-host-side-test-support-crate.md` (Step 1 output) - whole
- OrcaSlicer refs: none — no parity content.
- Verification:
  - AC-1 command from `packet.spec.md` - FACT PASS/FAIL
- Exit condition: AC-1 prints `PASS`. A crate that compiles but exposes a fn-count other than exactly {1,1,1,1}, or carries any `[dependencies]` entry, fails the step.

### Step 3a: Point the two slicer-runtime sites at the crate

- Task IDs: `TASK-146d`
- Objective: add the `slicer-test-support` dev-dependency to `crates/slicer-runtime/Cargo.toml`; in `slicer_cache.rs` delete the moved fn bodies and add `#[allow(unused_imports)] pub use slicer_test_support::{pnp_cli_bin, staleness_reason};` (two symbols only — `newest_source_mtime` has zero consumers outside `slicer-test-support`, re-derive with `rg -n 'newest_source_mtime' crates/ --glob '!crates/slicer-test-support/**'`; the `#[allow]` is required because the module is `#[path]`-included as a private module and the otherwise-unused `staleness_reason` re-export fails `cargo clippy --workspace --all-targets -- -D warnings` in `arachne_wall_sequence_e2e_tdd`); in `gate_evidence.rs` delete the `pnp_cli_bin` mirror, import from the crate, and correct the module doc-comment whose self-containment justification is now void.
- Precondition: Step 2 done (the crate compiles and is a member).
- Postcondition: `cargo check -p slicer-runtime --all-targets` passes (tests + benches compile); no locator fn body remains in either file.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-runtime/tests/common/slicer_cache.rs` - locator block + module header only
  - `crates/slicer-runtime/benches/gate_evidence.rs` (short; read whole if ≤300 lines, else locator block + doc-comment)
  - `crates/slicer-runtime/Cargo.toml` - `[dev-dependencies]` section only
- Files allowed to edit (at most 3):
  - `crates/slicer-runtime/Cargo.toml`
  - `crates/slicer-runtime/tests/common/slicer_cache.rs`
  - `crates/slicer-runtime/benches/gate_evidence.rs`
- Files explicitly out of bounds:
  - every file under `crates/slicer-runtime/tests/e2e/**` and `tests/integration/**` — the re-export exists so the caller files need zero edits; touching one means the re-export is wrong. The four locator-copy sites in those trees are Step 3c's, not this step's.
  - `crates/slicer-scheduler/**` (Step 3b)
- Expected sub-agent dispatches:
  - Question: "Run `cargo check -p slicer-runtime --all-targets`; pass/fail + first 20 error lines"; scope: workspace; return: `FACT` + SNIPPETS ≤20
- Context cost: `S`
- Authoritative docs:
  - `.ralph/specs/162_wit-lifecycle-export-removal/design.md` - §"CLI freshness" only (the message/shape contract being preserved)
- OrcaSlicer refs: none — no parity content.
- Verification:
  - `cargo check -p slicer-runtime --all-targets` - FACT pass/fail
  - `cargo bench -p slicer-runtime --bench gate_evidence --no-run 2>&1 | tail -3` - FACT pass/fail (compile-only; never run the bench)
- Exit condition: both commands pass AND `rg -c 'fn (pnp_cli_bin|staleness_reason|newest_source_mtime)\(' crates/slicer-runtime/tests/common/slicer_cache.rs crates/slicer-runtime/benches/gate_evidence.rs` finds no match. A green check with a surviving local fn body means the site still shadows the crate — step not done. The grep is scoped to these two files, not all of `crates/slicer-runtime/`, because the premise correction found four further copies under `tests/e2e/**` and `tests/integration/**` that Step 3c owns; a crate-wide grep here would report failure for work this step does not do.

### Step 3b: Point the slicer-scheduler site at the crate

- Task IDs: `TASK-146d`
- Objective: add the `slicer-test-support` dev-dependency to `crates/slicer-scheduler/Cargo.toml`; in `dag_cli_integration.rs` delete `fn bin()` and route all its call sites through `slicer_test_support::pnp_cli_bin()`.
- Precondition: Step 2 done. (Independent of Step 3a; either order.)
- Postcondition: `cargo check -p slicer-scheduler --all-targets` passes; `fn bin(` gone from the file.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-scheduler/tests/integration/dag_cli_integration.rs` (long; ranged reads only — the header plus the `bin`/`workspace_root` block, and the `Command::new` call sites located by grep)
  - `crates/slicer-scheduler/Cargo.toml` - `[dev-dependencies]` section only
- Files allowed to edit (at most 3):
  - `crates/slicer-scheduler/Cargo.toml`
  - `crates/slicer-scheduler/tests/integration/dag_cli_integration.rs`
- Files explicitly out of bounds:
  - all other `crates/slicer-scheduler/tests/**`; `crates/slicer-scheduler/src/**`
- Expected sub-agent dispatches:
  - Question: "Run `cargo check -p slicer-scheduler --all-targets`; pass/fail + first 20 error lines"; scope: workspace; return: `FACT` + SNIPPETS ≤20
- Context cost: `S`
- Authoritative docs:
  - `.ralph/specs/162_wit-lifecycle-export-removal/design.md` - §"CLI freshness" only (the `cargo build -p pnp-cli` panic-message contract, which now must hold in the shared crate)
- OrcaSlicer refs: none — no parity content.
- Verification:
  - `cargo check -p slicer-scheduler --all-targets` - FACT pass/fail
  - AC-3 command from `packet.spec.md` - FACT PASS/FAIL, **expected FAIL here**: sites 4–7 are still unmigrated until Step 3c. Run it for the diagnostic (its `missing=`/`local=` lists should name only those four); do not gate the step on it.
- Exit condition: `cargo check -p slicer-scheduler --all-targets` passes and `rg -c '\bfn bin\(' crates/slicer-scheduler/tests/integration/dag_cli_integration.rs` finds no match. **AC-2 and AC-3 can *not* go green here** — the premise correction found four further locator copies (sites 4–7, see `design.md` §Code Change Surface), which are still present at this point. Both first go green at Step 3c. Expecting them green here would either stall the step or invite an implementer to widen Step 3b's file list, which is wrong.

### Step 3c: Migrate the four discovered locator copies

- Task IDs: `TASK-146d`
- Objective: at each of the four locator-copy sites found by the premise correction, delete the local `fn pnp_cli_bin` (and, with it, the inert `std::env::var("PROFILE")` branch it contains — Cargo sets `PROFILE` for build scripts, not test binaries, so the branch never selects `release`) and add `use slicer_test_support::pnp_cli_bin;`. A qualified `slicer_test_support::pnp_cli_bin()` call site is equally acceptable; AC-3 and AC-8 accept both name-resolution-equivalent forms. This closes the missing freshness gate at four sites packet 162 never covered.
- Precondition: Step 2 done (the crate exists, compiles, and is a workspace member) **and Step 3a done** — Step 3a adds `[dev-dependencies] slicer-test-support` to `crates/slicer-runtime/Cargo.toml`, which is the manifest all four of these files resolve through. These four need **no manifest edit of their own**: one `slicer-runtime` dev-dependency serves the entire `tests/` tree. Independent of Step 3b (either order).
- Postcondition: `cargo check -p slicer-runtime --all-targets` passes; none of the four defines `fn pnp_cli_bin(`; none mentions `PROFILE`; each names `slicer_test_support`. Exactly one `fn pnp_cli_bin(` definition remains workspace-wide (the shared crate's).
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-runtime/tests/integration/no_linker_module_degraded_raw_output_tdd.rs` - the `pnp_cli_bin` block + the `use` header only (locate by name)
  - `crates/slicer-runtime/tests/e2e/infill_overlap_changes_gcode_tdd.rs` - same
  - `crates/slicer-runtime/tests/e2e/modifier_infill_tdd.rs` - same
  - `crates/slicer-runtime/tests/e2e/wedge_linked_infill_report_tdd.rs` - same
- Files allowed to edit (four — exceeds the usual at-most-3 cap; justified):
  - `crates/slicer-runtime/tests/integration/no_linker_module_degraded_raw_output_tdd.rs`
  - `crates/slicer-runtime/tests/e2e/infill_overlap_changes_gcode_tdd.rs`
  - `crates/slicer-runtime/tests/e2e/modifier_infill_tdd.rs`
  - `crates/slicer-runtime/tests/e2e/wedge_linked_infill_report_tdd.rs`
  - Justification for the cap overrun: these are four instances of one edit — deleting an identical copied block and adding one `use` line. No file is edited for more than one reason, no file requires reading beyond its locator block, and splitting into two steps would leave AC-2 red at an intermediate boundary for no diagnostic benefit. The 3-file cap exists to bound reasoning surface, not file count; the reasoning surface here is a single edit shape.
- Files explicitly out of bounds:
  - every *other* file under `crates/slicer-runtime/tests/e2e/**` and `crates/slicer-runtime/tests/integration/**` — the `slicer_cache.rs` re-export exists so the other caller files need zero edits; touching one means the re-export is wrong. The caller population is single-digit, not the "~30" earlier revisions claimed (re-derive with `rg -l 'slicer_cache' crates/slicer-runtime/tests/`), and only `crates/slicer-runtime/tests/integration/pnp_cli_freshness_tdd.rs` consumes a re-exported *locator* symbol. The `pub use` is justified independently of the magnitude: it preserves packet 162's registered regression home (`pnp_cli_freshness_tdd.rs`) without relocating that test.
  - `crates/slicer-runtime/Cargo.toml` (Step 3a's; already carries the dev-dependency — re-editing it here signals a misdiagnosis)
  - `crates/slicer-runtime/tests/common/slicer_cache.rs`, `crates/slicer-runtime/benches/gate_evidence.rs` (Step 3a); `crates/slicer-scheduler/**` (Step 3b); `crates/slicer-test-support/**` (Step 2)
- Expected sub-agent dispatches:
  - Question: "Run `cargo check -p slicer-runtime --all-targets`; pass/fail + first 20 error lines on failure"; scope: workspace; return: `FACT` + SNIPPETS ≤20
  - Question: "Run the AC-2, AC-3, and AC-8 audit commands from `packet.spec.md`; return the three PASS/FAIL lines"; scope: repo files; return: `FACT` ≤3 lines
- Context cost: `S`
- Authoritative docs:
  - `.ralph/specs/165_cli-binary-locator-extraction/design.md` - §"Code Change Surface" → "Sites 4–7" and §"Risks and Tradeoffs" (the loudness-contract consequence) only
- OrcaSlicer refs: none — no parity content.
- Verification:
  - `cargo check -p slicer-runtime --all-targets` - FACT pass/fail
  - AC-8 command from `packet.spec.md` - FACT PASS/FAIL (the premise-correction gate; FAILs on the pre-migration tree by construction)
  - AC-2 command from `packet.spec.md` - FACT PASS/FAIL. **AC-2 first goes green here, not at Step 3b** — with four copies outstanding, Step 3b cannot satisfy it.
- Exit condition: `cargo check -p slicer-runtime --all-targets` passes AND AC-8 prints `PASS` AND AC-2 prints `PASS` AND AC-3 prints `PASS`. A green check with a surviving `PROFILE` reference in any of the four means the inert branch was left behind — step not done.

### Step 4: Full gates, baseline, and backlog row

- Task IDs: `TASK-146d`
- Objective: run every packet gate; add the TASK-146d row to `docs/07_implementation_status.md` by dispatch.
- Precondition: Steps 1–3c done (3c included — AC-2, AC-3, and AC-8 cannot be green without it).
- Postcondition: all AC commands PASS; backlog row present.
- Files allowed to read, with ranges when over 300 lines:
  - `target/test-output.log` - `^test result` lines and failure context only (never re-run to see more output)
- Files allowed to edit (at most 3):
  - none directly (`docs/07_implementation_status.md` via dispatch only)
- Files explicitly out of bounds:
  - `docs/07_implementation_status.md` (dispatch, never read); all source files (no code edits in this step — a failure returns to the owning step)
- Expected sub-agent dispatches:
  - Question: "Run `cargo check --workspace --all-targets` then `cargo clippy --workspace --all-targets -- -D warnings`; pass/fail each + first 20 error lines on failure"; scope: workspace; return: `FACT` + SNIPPETS ≤20
  - Question: "Run the AC-4, AC-5, `perimeter_parity`, and `legacy_zero_matches_golden` commands from `requirements.md` §Verification Commands (each already rg-filtered); return the four `test result:` lines"; scope: workspace; return: `FACT` ≤5 lines
  - Question: "Run the AC-1, AC-2, AC-3, AC-7, AC-8, AC-N1 audit commands; return the six PASS/FAIL lines"; scope: repo files; return: `FACT` ≤6 lines
  - Question: "Append the TASK-146d row to `docs/07_implementation_status.md` (TASK-119a/TASK-194a sub-letter convention); return the added line"; scope: `docs/07_implementation_status.md`; return: `FACT`
- Context cost: `S`
- Authoritative docs:
  - `CLAUDE.md` §"Test Discipline" - direct read (already loaded)
- OrcaSlicer refs: none — no parity content.
- Verification:
  - every pipe-suffixed AC command in `packet.spec.md` — AC-1 through **AC-8** plus AC-N1 - FACT PASS/FAIL each. AC-8 is the premise-correction gate over the four discovered sites; it is not optional and it is not a duplicate of AC-2 (AC-2 counts definitions workspace-wide, AC-8 additionally asserts the inert `PROFILE` branch is gone).
  - baseline: `perimeter_parity` reports `3 passed; 0 failed` and `legacy_zero_matches_golden` reports `1 passed; 0 failed` (both name-filtered; `0 passed` = FAIL)
- Exit condition: all ACs PASS, clippy clean, baseline green, `rg -q 'TASK-146d' docs/07_implementation_status.md` succeeds. Any red returns to the owning step; do not patch forward from here.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | S | ADR authoring; number derived in-step |
| Step 2 | S | code moved, not written |
| Step 3a | S | 3 files, mechanical |
| Step 3b | S | 2 files, mechanical |
| Step 3c | S | 4 files, one identical mechanical edit each; premise correction — no manifest edit |
| Step 4 | S | dispatch-only gates |

Aggregate `S`. No step approaches L.

## Packet Completion Gate

- All steps and exits complete.
- Every pipe-suffixed AC command returns PASS.
- Update `docs/07_implementation_status.md` through a worker dispatch, never a full backlog read.
- No reopened/superseded transitions: packet 162 stays `implemented`; its `[FWD]` on locator extraction is resolved by this packet (note it in the closure report; do not edit 162's files).
- `packet.spec.md` is ready for `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed AC and packet-level gate command.
- This packet's `packet.spec.md`/`requirements.md` do **not** require `cargo test --workspace`: the surface is dev-only plumbing; `--all-targets` check/clippy plus the targeted runs cover every consumer. Do not run it.
- Record remaining packet-local risk (expected: the scan-scope over-approximation noted in `design.md` §Risks).
- Confirm context stayed at or below 150k standard, or at/below 300k only with a logged swarm ESCALATION; otherwise record a packet-authoring lesson.

All `cargo check`, `cargo clippy`, and `cargo test` invocations in gate and verification commands must use `--all-targets` so the test, bench, and example targets compile.
