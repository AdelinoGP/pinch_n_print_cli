# Implementation Plan: 229-wit-verify-declaration-model

## Execution Rules

- Work one atomic step at a time; map every step to grouped task IDs.
- Use TDD, then implementation, then the narrowest falsifying validation.
- Every field below is a context-budget contract and must be filled independently; never write "see Step 1".
- `xtask` tests are `#[cfg(test)] mod tests` blocks inside `xtask/src/*.rs`; every narrow verification is `cargo test -p xtask <path::to::test> -- --exact`, tee'd to `target/test-output.log` per `CLAUDE.md` §"Test output must always tee".

## Steps

### Step 1: Inventory the existing verifier and its single consumer

- Task IDs: `TASK-340`
- Objective: establish the exact current surface to be replaced and the exact call-site shape in `build_one`, so no later step discovers a consumer mid-rewrite.
- Precondition: working tree clean; `cargo check --workspace --all-targets` passes.
- Postcondition: a written inventory of every item exported by `xtask/src/wit_verify.rs`, every in-tree reference to each, and the literal text of `build_one`'s verification block.
- Files allowed to read, with ranges when over 300 lines:
  - `xtask/src/wit_verify.rs` — long; full read, once (this is the only step permitted to read it whole)
  - `xtask/src/build_guests.rs` — long; ranged reads only. Only the `GuestSpec` definition, the `BuildError` enum, and `build_one`
- Files allowed to edit (at most 3):
  - none (read-only discovery step)
- Files explicitly out of bounds:
  - `crates/slicer-macros/src/lib.rs`, every other packet directory, `xtask/src/test.rs`, `xtask/src/main.rs`, `xtask/src/dist.rs`
- Expected sub-agent dispatches:
  - Question: "Every reference to `wit_verify::` symbols anywhere in the workspace outside `xtask/src/wit_verify.rs` itself"; scope: `**/*.rs`; return: `LOCATIONS` (<=20 entries)
- Context cost: `S`
- Authoritative docs:
  - `docs/specs/guest-freshness-artifact-verification-plan.md` — sections "Locked decisions" C3/C12/C13 and "Round 5"; direct read
- Verification:
  - `rg -n 'wit_verify::' --glob '!xtask/src/wit_verify.rs' -g '*.rs' | wc -l` — FACT: the count of external references (expected: only `xtask/src/build_guests.rs`)
- Exit condition: the inventory names every symbol in the deletion list from `design.md` §Code Change Surface and confirms `build_one` is the only external consumer. If any other consumer exists, stop and report — the change surface in `design.md` is then wrong.

### Step 2: Add `wit-parser` and derive the canonical file list (TDD)

- Task IDs: `TASK-340`
- Objective: add the dependency and land `macro_embedded_wit_files`, with the coverage-audit test (AC-1) written first and failing.
- Precondition: Step 1 exit met.
- Postcondition: `macro_embedded_wit_files(ws_root)` returns exactly the 20 `.wit` paths the macro `include_str!`s, `root.wit` excluded, derived by a multiline-aware parse of `crates/slicer-macros/src/lib.rs`; AC-1's test passes.
- Files allowed to read, with ranges when over 300 lines:
  - `xtask/Cargo.toml` — full (short manifest)
  - `crates/slicer-runtime/Cargo.toml` and `crates/slicer-wasm-host/Cargo.toml` — only the `wit-parser` line, to version-match
- Files allowed to edit (at most 3):
  - `xtask/Cargo.toml`
  - `xtask/src/wit_verify.rs`
- Files explicitly out of bounds:
  - `Cargo.lock` (never edit by hand; `cargo` updates it), `crates/slicer-macros/src/lib.rs` (extract by dispatch, never read)
- Expected sub-agent dispatches:
  - Question: "List every distinct `.wit` path passed to `include_str!` in `crates/slicer-macros/src/lib.rs`, multiline-aware, sorted, deduped, with the count"; scope: `crates/slicer-macros/src/lib.rs`; return: `FACT` (count + paths)
  - Question: "What exact version string do `crates/slicer-runtime/Cargo.toml` and `crates/slicer-wasm-host/Cargo.toml` declare for `wit-parser`?"; scope: those two manifests; return: `FACT` (<=3 lines)
- Context cost: `S`
- Authoritative docs:
  - `docs/specs/guest-freshness-artifact-verification-plan.md` — locked decision C12 and amended C13; direct read
- Verification:
  - `mkdir -p target && cargo test -p xtask wit_verify::tests::canonical_file_list_equals_macro_include_str_set -- --exact 2>&1 | tee target/test-output.log | rg '^test result: ok\. 1 passed'` — FACT pass/fail
  - `cargo check --workspace --all-targets` — FACT pass/fail
- Exit condition: the audit test asserts a set of 20 paths that it derived from `crates/slicer-macros/src/lib.rs` at runtime, not from a constant in `wit_verify.rs`; `crates/slicer-schema/wit/root.wit` is absent from that set.

### Step 3: Parse both sides into `WorldModel`

- Task IDs: `TASK-340`
- Objective: land `WorldModel` / `PackageModel` / `InterfaceModel`, `canonical_world_model`, `embedded_world_model`, and the fail-closed `VerifyError` variants; no comparison logic yet.
- Precondition: Step 2 exit met; `wit-parser` resolves.
- Postcondition: both sides parse into the same model shape; braced-form and statement-form packages key identically (AC-12); the three fail-closed error paths return `Err` (AC-N1, AC-N2, AC-N3).
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-runtime/tests/contract/wit_single_source_tdd.rs` — only the `wit_parser::Resolve` / `UnresolvedPackageGroup::parse` call sites
  - `crates/slicer-schema/wit/deps/types.wit` and `crates/slicer-schema/wit/deps/ir-types.wit` — full, for real declaration names (`extrusion-role` lives in the former; `region-key` and the `layer-idx` alias in the latter)
- Files allowed to edit (at most 3):
  - `xtask/src/wit_verify.rs`
- Files explicitly out of bounds:
  - `xtask/src/build_guests.rs` (migrated in Step 5), `crates/slicer-macros/build.rs` (Step 6)
- Expected sub-agent dispatches:
  - Question: "Minimal `wit_parser` 0.247 API to (a) resolve a directory of `.wit` files and (b) parse a single in-memory WIT string, and how to enumerate packages → interfaces → type/function items from the result"; scope: the `wit-parser` crate docs/source in the cargo registry; return: `SUMMARY` (<=200 words)
- Context cost: `M`
- Authoritative docs:
  - `docs/specs/guest-freshness-artifact-verification-plan.md` — amended C3 and finding R5-5; direct read
- Verification:
  - `mkdir -p target && cargo test -p xtask wit_verify::tests::braced_package_form_parses_like_statement_form -- --exact 2>&1 | tee target/test-output.log | rg '^test result: ok\. 1 passed'` — FACT pass/fail
  - `mkdir -p target && cargo test -p xtask wit_verify::tests::empty_canonical_set_is_an_error_not_a_pass -- --exact 2>&1 | tee target/test-output.log | rg '^test result: ok\. 1 passed'` — FACT pass/fail
  - `mkdir -p target && cargo test -p xtask wit_verify::tests::unreadable_canonical_file_is_an_error_not_a_pass -- --exact 2>&1 | tee target/test-output.log | rg '^test result: ok\. 1 passed'` — FACT pass/fail
  - `mkdir -p target && cargo test -p xtask wit_verify::tests::unparseable_embedded_text_is_an_error_not_a_pass -- --exact 2>&1 | tee target/test-output.log | rg '^test result: ok\. 1 passed'` — FACT pass/fail
- Exit condition: `canonical_world_model` on an empty directory returns `Err(VerifyError::CanonicalEmpty)` and on a missing listed file returns `Err(VerifyError::CanonicalUnreadable)`; neither returns an `Ok` empty model. `xtask/src/wit_verify.rs` still compiles alongside the not-yet-deleted scanner if the implementer stages the deletion.

### Step 4: Comparison engine and its declaration-level fixtures

- Task IDs: `TASK-340`
- Objective: land `StageExpectation`, `stage_expectation`, `Drift`/`DriftKind`, `SHARED_PACKAGES`, `compare_worlds` and the new `verify_embedded_world`, covering all three comparison directions and order sensitivity.
- Precondition: Step 3 exit met.
- Postcondition: AC-3 through AC-10 and AC-N4 pass.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-schema/src/lib.rs` — over 600 lines; only the `StageSpec` definition and the `wit_dir_for_stage_id` / `package_for_stage_id` / `interface_for_stage_id` / `qualified_export_for_stage_id` bodies
  - `crates/slicer-schema/wit/deps/finalization-layer-finalization/finalization-layer-finalization.wit` — full, for the non-exported `*-types` interface fixture
- Files allowed to edit (at most 3):
  - `xtask/src/wit_verify.rs`
- Files explicitly out of bounds:
  - the other 19 canonical `.wit` files, `crates/slicer-macros/src/lib.rs`, `xtask/src/build_guests.rs`
- Expected sub-agent dispatches:
  - Question: "Exact declaration text of `record region-key`, `variant extrusion-role`, the `layer-idx` alias, and `resource layer-collection-view`, with the owning canonical file"; scope: `crates/slicer-schema/wit/**`; return: `SNIPPETS` (<=3 snippets, <=30 lines each)
- Context cost: `M`
- Authoritative docs:
  - `docs/specs/guest-freshness-artifact-verification-plan.md` — amended C3 plus findings R5-1 and R5-8; direct read
- Verification:
  - `mkdir -p target && cargo test -p xtask wit_verify::tests::record_field_reorder_is_drift -- --exact 2>&1 | tee target/test-output.log | rg '^test result: ok\. 1 passed'` — FACT pass/fail
  - `mkdir -p target && cargo test -p xtask wit_verify::tests::variant_case_reorder_is_drift -- --exact 2>&1 | tee target/test-output.log | rg '^test result: ok\. 1 passed'` — FACT pass/fail
  - `mkdir -p target && cargo test -p xtask wit_verify::tests::non_exported_stage_interfaces_use_subset_direction -- --exact 2>&1 | tee target/test-output.log | rg '^test result: ok\. 1 passed'` — FACT pass/fail
  - `mkdir -p target && cargo test -p xtask wit_verify::tests::unexpected_package_is_drift -- --exact 2>&1 | tee target/test-output.log | rg '^test result: ok\. 1 passed'` — FACT pass/fail
- Exit condition: swapping two fields of `record region-key` and two cases of `variant extrusion-role` each produce a `DriftKind::DeclarationBody`; omitting a whole member from a non-exported stage interface produces none; a package outside the allowed set produces `DriftKind::UnexpectedPackage`.

### Step 5: Delete the scanner and migrate `build_one`

- Task IDs: `TASK-340`
- Objective: remove every hand-rolled parsing symbol and its scanner-only tests, repoint `build_one` at the new API with a new `BuildError` variant for the canonical-unusable class, and retype `BuildError::StaleEmbeddedWorld`'s `mismatches` field from `Vec<TypeMismatch>` to `Vec<Drift>` (mandatory: `TypeMismatch` is deleted in this step, and that variant is its only field-level consumer).
- Precondition: Step 4 exit met; all Step 2-4 tests green.
- Postcondition: AC-13 passes; `cargo check --workspace --all-targets` and `cargo clippy --workspace --all-targets -- -D warnings` pass.
- Files allowed to read, with ranges when over 300 lines:
  - `xtask/src/build_guests.rs` — only `BuildError` and `build_one`
- Files allowed to edit (at most 3):
  - `xtask/src/wit_verify.rs`
  - `xtask/src/build_guests.rs`
- Files explicitly out of bounds:
  - `xtask/src/test.rs`, `xtask/src/main.rs`, `xtask/src/dist.rs`; `check_command`, `is_stale`, `compute_shared_freshness`, `stage_wit_snapshot`, `fingerprint_*` and `build_one_inner` inside `build_guests.rs` must not be edited
- Blast-radius discipline: this step both adds a `BuildError` variant and changes an existing variant's payload type (`StaleEmbeddedWorld { mismatches }`). No struct-literal blast radius applies (no struct field, no schema constant), but every construction site and every `match` arm of `StaleEmbeddedWorld` must move in the same edit — including its `Display` arm. Confirm with `rg -n 'BuildError::|StaleEmbeddedWorld' xtask/src/` before editing and cover every site the grep returns.
- Expected sub-agent dispatches:
  - Question: "Every `match` arm, construction site and `Display` arm of `BuildError` — and of `BuildError::StaleEmbeddedWorld` specifically — in `xtask/src/`"; scope: `xtask/src/*.rs`; return: `LOCATIONS` (<=20 entries)
- Context cost: `S`
- Authoritative docs:
  - `CLAUDE.md` — section "In-Tree Citation Style (MUST follow)"; direct read
- Verification:
  - `if rg -q 'extract_type_blocks|matching_brace|BRACED_KEYWORDS|ambiguous_type_names|canonical_type_blocks|TypeMismatch|fn strip_comments|fn normalize' xtask/src/wit_verify.rs xtask/src/build_guests.rs; then echo FAIL; else echo PASS; fi` — FACT PASS/FAIL
  - `cargo clippy --workspace --all-targets -- -D warnings` — FACT pass/fail
- Exit condition: every symbol on the deletion list — including `TypeMismatch`, `strip_comments` and `normalize` — is absent from both files; `BuildError::StaleEmbeddedWorld` carries `Vec<Drift>` and its `Display` arm compiles; `build_one` retains its rebuild-once-then-fail structure; and `module_stage_wit_dir` plus `core_modules_resolve_their_stage_wit_dir` still exist untouched (packet 230 owns their retirement).

### Step 6: Correct `crates/slicer-macros/build.rs`

- Task IDs: `TASK-340`
- Objective: make the macro's `rerun-if-changed` list exactly the 20-file `include_str!` set, and pin it with a test.
- Precondition: Step 5 exit met.
- Postcondition: AC-2 passes; the 4 non-existent `deps/world-*/world-*.wit` paths are gone; `deps/prepass-types.wit` and all 15 per-stage files are watched.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-macros/build.rs` — full (short build script)
- Files allowed to edit (at most 3):
  - `crates/slicer-macros/build.rs`
  - `xtask/src/wit_verify.rs` (the audit test only)
- Files explicitly out of bounds:
  - `crates/slicer-macros/src/lib.rs`, `crates/slicer-macros/Cargo.toml`
- Expected sub-agent dispatches:
  - Question: "After the `crates/slicer-macros/build.rs` edit, does `cargo xtask build-guests --check` report any `STALE:` lines? Return the count and the first 5 names"; scope: workspace; return: `FACT` (<=6 lines)
- Context cost: `S`
- Authoritative docs:
  - `CLAUDE.md` — section "Guest WASM Staleness (MUST follow)"; direct read
- Verification:
  - `mkdir -p target && cargo test -p xtask wit_verify::tests::macros_build_rs_watches_the_macro_include_str_set -- --exact 2>&1 | tee target/test-output.log | rg '^test result: ok\. 1 passed'` — FACT pass/fail
  - `cargo xtask build-guests --check` — FACT: clean, or the list of `STALE:` names to rebuild
- Exit condition: the audit test compares `build.rs`'s parsed watch list against the same derived 20-file set as AC-1 and passes; `cargo xtask build-guests --check` has been run and any `STALE:` report rebuilt with `cargo xtask build-guests` before the next step.

### Step 7: Real-artifact clean gate

- Task IDs: `TASK-340`
- Objective: prove the model against real built artifacts, and make the real-artifact tests assert rather than skip.
- Precondition: Step 6 exit met and guests rebuilt if `--check` reported `STALE:`.
- Postcondition: AC-11 passes; the test skips only when `wasm-tools` genuinely does not resolve on `PATH`, and asserts in every other case.
- Files allowed to read, with ranges when over 300 lines:
  - `xtask/src/wit_verify.rs` — only its `#[cfg(test)] mod tests`
- Files allowed to edit (at most 3):
  - `xtask/src/wit_verify.rs`
- Files explicitly out of bounds:
  - all `.wasm` artifacts (inputs to `wasm-tools`, never read into context)
- Expected sub-agent dispatches:
  - Question: "Decode `modules/core-modules/wipe-tower/wipe-tower.wasm` and `modules/core-modules/layer-planner-default/layer-planner-default.wasm` with `wasm-tools component wit`; return only the `package` declaration lines and the export line from each"; scope: those two artifacts; return: `FACT` (<=10 lines)
- Context cost: `S`
- Authoritative docs:
  - `docs/specs/guest-freshness-artifact-verification-plan.md` — finding R5-10 (why skipping is vacuous); direct read
- Verification:
  - `mkdir -p target && cargo test -p xtask wit_verify::tests::real_core_module_artifacts_verify_clean -- --exact 2>&1 | tee target/test-output.log | rg '^test result: ok\. 1 passed'` — FACT pass/fail
  - `mkdir -p target && cargo test -p xtask wit_verify 2>&1 | tee target/test-output.log | rg '^test result:'` — FACT pass/fail for the whole verifier suite
- Exit condition: both real artifacts compare with an empty drift list, and the test contains no `return` that fires when the artifact exists and `wasm-tools` resolves. If a real artifact reports drift, that is a genuine finding: rebuild the guest and re-run; never weaken the comparison to make it pass.

### Step 8: Backlog row and closure

- Task IDs: `TASK-340`
- Objective: register `TASK-340` in the backlog and run the closure gates.
- Precondition: Steps 1-7 exits met.
- Postcondition: AC-14 passes; all three `packet.spec.md` gate commands pass.
- Files allowed to read, with ranges when over 300 lines:
  - `docs/07_implementation_status.md` — over 300 lines; delegate. Read only the returned line range around "Workstream 5 — Governance and closure drift"
- Files allowed to edit (at most 3):
  - `docs/07_implementation_status.md`
  - `docs/spec_packets/229-wit-verify-declaration-model/packet.spec.md` (status transition only)
- Files explicitly out of bounds:
  - every other packet directory under `docs/spec_packets/**`
- Expected sub-agent dispatches:
  - Question: "Line number of the `### Workstream 5 — Governance and closure drift` heading and the current highest `TASK-###` in `docs/07_implementation_status.md`"; scope: that file; return: `FACT` (<=3 lines)
- Context cost: `S`
- Authoritative docs:
  - `CLAUDE.md` — section "Ledger Facts Must Be Re-derived, Not Quoted"; direct read. Re-derive the highest `TASK-###` at write time and renumber on collision.
- Verification:
  - `rg -q 'TASK-340 ' docs/07_implementation_status.md && echo PASS || echo FAIL` — FACT PASS/FAIL
  - `cargo check --workspace --all-targets` — FACT pass/fail
  - `cargo clippy --workspace --all-targets -- -D warnings` — FACT pass/fail
  - `cargo xtask check-literals` — FACT pass/fail
- Exit condition: the `TASK-340` row exists under Workstream 5, every pipe-suffixed AC command in `packet.spec.md` returns PASS, and the packet is ready for `status: implemented`.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | S | Read-only inventory; one full read of a long file (re-derive size with `wc -l` before budgeting) |
| Step 2 | S | Dependency line + derived file list; macro lib.rs by dispatch only |
| Step 3 | M | New `wit_parser` API surface on both sides |
| Step 4 | M | Three comparison directions plus order-sensitivity fixtures |
| Step 5 | S | Deletions plus one call-site migration |
| Step 6 | S | short `build.rs` plus one audit test |
| Step 7 | S | Two artifact decodes, returned as bounded FACTs |
| Step 8 | S | One backlog row plus closure gates |

Aggregate: `M`. No step is L; no split required before activation.

## Packet Completion Gate

- All steps and exits complete.
- Every pipe-suffixed AC command in `packet.spec.md` returns PASS.
- `cargo xtask build-guests --check` reports clean (after the `crates/slicer-macros/build.rs` change forced a rebuild).
- Update `docs/07_implementation_status.md` through a worker dispatch, never a full backlog read.
- Report any rename of a net-new symbol relative to `design.md` §Code Change Surface, because packet 230's design consumes those names verbatim.
- No reopened/superseded packet transitions apply.
- `packet.spec.md` is ready for `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed AC and the three packet-level gate commands.
- Record remaining packet-local risk, specifically: any real artifact that reported drift and had to be rebuilt.
- Confirm context stayed at or below 150k standard, or at/below 300k only with a logged swarm ESCALATION; otherwise record a packet-authoring lesson.

All `cargo check`, `cargo clippy`, and `cargo test` invocations in gate and verification commands must use `--all-targets` where the command supports it (`cargo test -p xtask <name> -- --exact` targets the crate's inline test module and is exempt, since `xtask` has no separate test targets).
