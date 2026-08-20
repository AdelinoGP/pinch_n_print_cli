# Requirements: 230-output-based-guest-freshness

## Packet Metadata

- Grouped task IDs: `TASK-341`
- Backlog source: `docs/07_implementation_status.md`
- Packet status: `implemented`
- Aggregate context cost: `M`
- Approved plan: `docs/specs/guest-freshness-artifact-verification-plan.md` (queue row #2; locked decisions C1, C4, C5, C6, C9, C10, C11 as amended by Round 5 findings R5-2, R5-3, R5-4, R5-7)
- Depends on: `docs/spec_packets/229-wit-verify-declaration-model` — implemented (a FORWARD-DEP at authoring time, satisfied before Step 1).

## Problem Statement

`cargo xtask build-guests --check` answers "did any tracked input change?" instead of "does this artifact still agree with canonical?". `is_stale` unions a per-guest fingerprint with `compute_shared_freshness`, whose `shared_input_paths` charges every guest with all of `crates/slicer-schema/wit/**` plus the `slicer-{macros,sdk,ir,schema,core}` sources. One byte changed in any of them marks all 42 guests (21 core + 21 test) `STALE:`, `cargo xtask test` rebuilds all 42, and the first guest that fails to compile aborts the whole suite — so a WIT change made by one agent surfaces as another agent's tests breaking.

The semantic answer already exists but is unreachable from the gate: `verify_embedded_world` is called only from `build_one`, i.e. only *after* a build. All 42 artifacts are present on disk and `wasm-tools 1.250.0` resolves on `PATH`.

Four defects block simply calling the verifier from `check_command`, and this packet is the slice that fixes exactly those four:

1. **Stage resolution.** `build_one` resolves the WIT dir through `module_stage_wit_dir`, which needs a sibling module manifest — test guests have none, so their comparison runs against the ambiguity-stripped shared set. Resolution must come from the artifact. But resolution *solely* from the artifact makes the check self-referential (R5-4): a guest exporting the wrong stage declares its own stage and is judged against it, comparing equal. The manifest `[stage] id` therefore stays as an independent expectation for core guests.
2. **API shape.** `check_command` returns a bare `i32`, so `test_command` learns only "something is stale" and must rebuild everything.
3. **Reporting.** Freshness is asserted downstream by grepping for `STALE:`. A `wasm-tools`-missing failure prints no `STALE:` line and therefore reads as PASS (R5-3). The contract must be exit-code based, with a distinct code for infrastructure failure.
4. **Fingerprint lifecycle and content.** The sidecar is written at the end of `build_one_inner` — *before* `build_one`'s verification runs — so a guest that fails verification still leaves a fingerprint claiming freshness. Its content also omits the workspace-root `Cargo.toml` (which pins `wit-bindgen`, consumed as `wit-bindgen.workspace = true`), the guest's own `Cargo.lock`, and the rustc version, any of which can change the emitted bindings with byte-identical WIT (R5-2).

## In Scope

- **Artifact-based stage resolution** in `xtask/src/wit_verify.rs`: `resolve_stage_from_world` maps the single non-shared, non-`root:component` embedded package (version-stripped) to a `slicer_schema::STAGES` row and returns packet 229's `StageExpectation`. The row whose `wit_package` is the empty string (`PrePass::PaintSegmentation`, host-built-in) is excluded from the candidate set. Zero, multiple, or unresolvable → an error variant that maps to staleness.
- **Manifest cross-check (R5-4).** For core guests, `GuestSpec.stage_id` (from `parse_stage_id_from_module_manifest`) is retained as an independent expectation; the artifact's resolved `stage_id` must equal it, and a mismatch is staleness. For test guests (`stage_id == None`) artifact resolution is the sole resolver. `GuestSpec.stage_id` and `parse_stage_id_from_module_manifest` **survive**; `module_stage_wit_dir` retires, its callers resolving the WIT dir from `STAGES` instead.
- **`check_command` returns `CheckOutcome { stale: Vec<GuestSpec>, code: i32 }`**, and both call sites migrate: the `Some("--check")` arm in `xtask/src/main.rs` (currently `std::process::exit(build_guests::check_command(&ws))`, which needs `.code`) and `test_command` in `xtask/src/test.rs`.
- **`build_stale_command(ws_root, &[GuestSpec]) -> i32`** added; `test_command` rebuilds only the stale specs. `build_command` is unchanged and stays the entry point for dist and CI.
- **Exit-code reporting contract (R5-3).** `0` = all fresh; `1` = at least one stale; `EXIT_INFRA_ERROR` (a distinct non-zero code) = infrastructure failure. Exactly one `STALE: <crate_name>` line per stale guest, with the reason on a following line that never contains the substring `STALE:`. `wasm-tools` missing is an infrastructure error, never staleness, and `test_command` aborts on it **without** attempting a rebuild.
- **Per-guest staleness reasons.** `StaleReason` with variants for artifact-missing, fingerprint mismatch, embedded-world drift, unresolvable stage, stage mismatch and undecodable artifact; `stale_reason` becomes the primary predicate and `is_stale` is expressed in terms of it.
- **Fingerprint lifecycle.** Sidecar removed at build start and on every persistent failure (`EmbeddedWorldUndecodable`, `StaleEmbeddedWorld`); written only after the final verification in `build_one` succeeds — i.e. moved out of the tail of `build_one_inner`.
- **Fingerprint content and version.** Prefix `v2-`; content additionally covers the workspace-root `Cargo.toml`, the guest's own `Cargo.lock`, the `rustc -vV` output string, and the `wasm-tools --version` string (R5-2, C5).
- **Retire the residual fail-open (R5-7):** `build_one`'s `if canonical.is_empty() { return Ok(()) }` is deleted; an unusable canonical set is an infrastructure error on every path, `--check` included.
- **Record measured `--check` wall-clock** before and after the change, in this file under "Measured Freshness Timing".
- Add the `TASK-341` row to `docs/07_implementation_status.md` under "Workstream 5 — Governance and closure drift".

## Out of Scope

- The declaration model, the comparison engine, the canonical coverage audit and the `crates/slicer-macros/build.rs` watch list — packet 229. This packet consumes that API and must not re-open it.
- The per-guest dependency-closure walk; deleting `compute_shared_freshness` and `stage_wit_snapshot`; making pnp_cli freshness an unconditional `cargo build --bin pnp_cli`; reconciling `crates/pnp-cli-locator::staleness_reason` — packet 231. `compute_shared_freshness` therefore still exists and is still called after this packet, including from `ensure_pnp_cli_fresh_with` in `xtask/src/test.rs`.
- `xtask/src/dist.rs` — untouched (user-ruled); it keeps calling `build_command`.
- All doc, snippet, ADR and CI edits, including adding `cargo test -p xtask` to `.github/workflows/ci.yml` and rewriting the `wasm-staleness` snippet to the exit-code contract — packet 232. (Exception: a non-normative pointer note was added under `docs/03_wit_and_manifest.md`'s "Build & Freshness Contract (Normative)" heading post-review; see the Doc Impact Statement in `packet.spec.md`.)
- Editing any other packet directory, including the six whose ACs use the grep form of the freshness check (user-ruled 2026-08-19).

## Authoritative Docs

- `docs/03_wit_and_manifest.md` — section "Build & Freshness Contract (Normative)" and its staleness-guard table row; direct ranged read. Behaviour changes here; the prose is repaired by packet 232.
- `docs/05_module_sdk.md` — the "canonical pre-test gate" mention only; direct ranged read, for awareness of what packet 232 must later reconcile.
- `docs/07_implementation_status.md` — over 300 lines; delegate a `LOCATIONS` dispatch for the Workstream 5 heading and the current highest `TASK-###`.
- `docs/specs/guest-freshness-artifact-verification-plan.md` — "Locked decisions" C1/C4/C5/C6/C9/C10/C11 and the "Round 5" section. Round 5 amendments win over conflicting locked-decision text.
- `CLAUDE.md` — sections "Guest WASM Staleness (MUST follow)", "No Unverified Metrics", "Test Discipline"; direct read.

## Measured Freshness Timing

**Unmeasured at authoring time.** The approved plan's earlier figures were never measured on this machine and must not be quoted anywhere in this packet or its commit message. `CLAUDE.md` §"No Unverified Metrics" forbids restating them.

The implementer records, in this section, both figures and the exact command used, each tagged `measured <YYYY-MM-DD>`:

- **Before** (fingerprint-union --check, captured before any edit in this packet): not measured on this machine before edit; no figure quoted. Rationale: the pre-edit wall-clock was not captured on this host before the Step 5 rebuild, and the pre-change code path can no longer be timed without a worktree checkout; per AC-16 the non-capture is documented here rather than any figure being quoted — measured 2026-08-20.
- **After** (artifact-decoding --check, all 42 fresh, wasm-tools on PATH): `cargo xtask build-guests --check` — wall-clock 5.35s (PowerShell Measure-Command TotalSeconds 5.35, TotalMilliseconds 5349) — exit 0, 0 STALE lines, wasm-tools 1.250.0, cargo 1.96.0 — measured 2026-08-20

Both runs were taken with all guests fresh (Step 5 rebuilt all 42; `cargo xtask build-guests --check` returned exit 0 before measurement) so the figures compare the *check* path, not a rebuild. Exact command timed: `cargo xtask build-guests --check` (PowerShell: `Measure-Command { cargo xtask build-guests --check | Out-Default }`). AC-16 greps this section for the `measured <date>` tag.

## Acceptance Summary

Criteria live in `packet.spec.md`; referenced here by ID only.

- Positive: `AC-1` through `AC-18`.
  - `AC-1`/`AC-2`/`AC-3` are the resolution triple: successful resolution, the empty-`wit_package` exclusion, and the three unresolvable shapes.
  - `AC-4`/`AC-5`/`AC-15` are the R5-4 anti-self-reference group; none may be dropped or merged, because together they are the only proof that the check is not judging an artifact against its own claim.
  - `AC-6`/`AC-7`/`AC-8` pin the exit-code contract at `1` / `0` / `EXIT_INFRA_ERROR`, and `AC-6` additionally pins that the reason line never carries the `STALE:` marker.
  - `AC-11`/`AC-12` are the fingerprint lifecycle and content pair.
  - `AC-13` and `AC-18` pin the API migration: `AC-13` that both call sites moved off the bare `i32` (including that the pre-migration `let check_code` binding is gone), `AC-18` that `is_stale`'s third parameter is now `&CheckContext` and its body delegates to `stale_reason`.
  - `AC-16` is the measurement obligation; it is falsifiable by grep against this file's "Measured Freshness Timing" section, which must carry the after figure plus either a before figure or an explicit documented non-capture.
- Negative: `AC-N1` through `AC-N4` (undecodable artifact, unusable canonical set, never-built guest, failed stale rebuild).
- Cross-packet impact: packet 231 consumes `CheckOutcome`, `StaleReason`, `stale_reason`, `build_stale_command`, `FINGERPRINT_VERSION` and `EXIT_INFRA_ERROR`; packet 232 documents the exit-code contract this packet establishes. Any rename must be reported at closure so both dependents are reconciled before activation.

## Verification Commands

Authoritative full matrix; `packet.spec.md` lists only the 3 closure gates.

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo check --workspace --all-targets` | the migrated API compiles at both call sites and in test targets | FACT pass/fail |
| `cargo clippy --workspace --all-targets -- -D warnings` | required pre-commit gate | FACT pass/fail; SNIPPETS <=20 lines on failure |
| `cargo xtask check-literals` | required pre-commit gate (struct-literal churn; `GuestSpec` is a watched 7-field type) | FACT pass/fail |
| `mkdir -p target && cargo test -p xtask 2>&1 \| tee target/test-output.log \| rg '^test result:'` | the whole xtask suite: `wit_verify`, `build_guests` and `test` modules | FACT pass/fail with each `test result:` line |
| `mkdir -p target && cargo test -p xtask build_guests::tests::core_guest_artifact_stage_must_equal_manifest_stage_id -- --exact 2>&1 \| tee target/test-output.log \| rg '^test result: ok\. 1 passed'` | AC-4, the R5-4 anti-self-reference guard | FACT pass/fail |
| `mkdir -p target && cargo test -p xtask build_guests::tests::missing_wasm_tools_is_infrastructure_error_not_staleness -- --exact 2>&1 \| tee target/test-output.log \| rg '^test result: ok\. 1 passed'` | AC-8, the R5-3 false-PASS guard | FACT pass/fail |
| `mkdir -p target && cargo test -p xtask build_guests::tests::fingerprint_is_written_only_after_final_verification -- --exact 2>&1 \| tee target/test-output.log \| rg '^test result: ok\. 1 passed'` | AC-11, fingerprint lifecycle | FACT pass/fail |
| `cargo xtask build-guests --check; echo "exit=$?"` | end-to-end exit-code behaviour on the real tree | FACT: the `exit=` line plus the count of `STALE:` lines |
| `cargo xtask build-guests` | rebuild after any guest-input edit; confirms `--check` then returns `exit=0` | FACT pass/fail |
| `rg -q 'pub stage_id: Option<String>' xtask/src/build_guests.rs && rg -q 'parse_stage_id_from_module_manifest' xtask/src/build_guests.rs && echo PASS \|\| echo FAIL` | AC-15, R5-4 survival | FACT PASS/FAIL |
| see `packet.spec.md` `AC-13` (call-site migration) and `AC-18` (`is_stale` shape) | API migration proof; commands are authored there and not duplicated | FACT PASS/FAIL |
| `rg -q 'TASK-341 ' docs/07_implementation_status.md && echo PASS \|\| echo FAIL` | AC-17 doc impact | FACT PASS/FAIL |

`cargo test --workspace` is **not** in this matrix. Everything this packet changes lives in `xtask`, whose tests are inline `#[cfg(test)] mod tests` blocks; `cargo test -p xtask` compiles and runs all of them.

## Step Completion Expectations

- **`cargo xtask test` is the gated entry point.** After any step that changes freshness behaviour, run `cargo xtask build-guests --check` and, if `STALE:` is reported, rebuild before attributing any failure to code. This packet changes the gate itself, so a bug in the gate looks exactly like a stale guest — treat an unexpected `STALE:` report as a possible regression in this packet's own logic, and confirm by decoding the named artifact.
- The fingerprint prefix moves from `v1-` to `v2-`, so **every** guest reports stale exactly once after the change and requires one full `cargo xtask build-guests`. Budget that rebuild into the step that lands the prefix change; do not let it surface as a mysterious mass-stale report in a later step.
- `xtask/src/test.rs` already has an injectable-seam precedent (`ensure_pnp_cli_fresh` delegating to `ensure_pnp_cli_fresh_with(ws_root, run_rebuild)`). The guest-rebuild branch of `test_command` needs the same treatment so AC-9, AC-10 and AC-N4 can assert without spawning real builds.
- `GuestSpec` is a `pub` struct with 7 named fields under `crates/`-adjacent tooling; any new test constructing it must use a `..` rest or an `// exhaustive: <reason>` waiver per `docs/21_data_defaults_and_fixtures.md`, enforced by `cargo xtask check-literals`.
- The measurement in "Measured Freshness Timing" must be taken with all guests fresh, or the "after" figure measures a rebuild and is meaningless.

## Context Discipline Notes

- `xtask/src/build_guests.rs` and `xtask/src/test.rs` are both long — ranged reads only; never read either whole (re-derive their size with `wc -l` if you need to budget a read). Read `build_guests.rs` only around `GuestSpec`, `BuildError`, `build_one`, `build_one_inner`'s tail, the fingerprint functions and `check_command`; read `test.rs` only around `test_command`'s freshness block and the existing `mod tests`.
- Never read `.wasm` artifacts. Decoded output goes through `rg`/`head` and is returned as a bounded FACT.
- `docs/07_implementation_status.md` is far over 300 lines — always delegated, never opened directly.
- Do not re-read packet 229's `design.md` in full — read only its §Code Change Surface for the consumed signatures. If a consumed symbol's shape is uncertain, dispatch a `FACT` against the implemented `xtask/src/wit_verify.rs`, which by then is the ground truth.
