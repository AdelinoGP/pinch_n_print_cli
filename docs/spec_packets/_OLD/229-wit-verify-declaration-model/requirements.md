# Requirements: 229-wit-verify-declaration-model

## Packet Metadata

- Grouped task IDs: `TASK-340`
- Backlog source: `docs/07_implementation_status.md`
- Packet status: `draft`
- Aggregate context cost: `M`
- Approved plan: `docs/specs/guest-freshness-artifact-verification-plan.md` (queue row #1; locked decisions C3, C12, C13 as amended by Round 5 findings R5-1, R5-5, R5-7, R5-8, R5-11)

## Problem Statement

`xtask/src/wit_verify.rs` is the only mechanism that answers the *semantic* question "does this built guest's embedded WIT world still agree with canonical?". Packet 230 wants to promote it into the freshness gate itself, but its current form cannot bear that weight, for five independently verified reasons:

1. **It is a hand-rolled scanner.** `extract_type_blocks` keys declarations by bare name in a flat `BTreeMap<String, String>`, so a name declared in two packages collapses; `canonical_type_blocks` works around that by *deleting* every ambiguous name when the stage is unknown. `matching_brace` also carries a latent index bug (R5-11): `open` is a byte index into `stripped` while `matching_brace(&bytes, open)` indexes a `Vec<char>`.
2. **It models only four keywords.** `BRACED_KEYWORDS` covers `variant`, `enum`, `record`, `flags`. Type aliases (`type X = ...;`), resources, interface functions, resource methods and `use` declarations are invisible, so nominal type-identity drift and signature drift pass silently.
3. **It compares only names present in both sides.** `verify_embedded_world`'s loop skips any canonical declaration the artifact does not embed, so a *missing* export is indistinguishable from an unused import.
4. **It fails open on infrastructure problems** (R5-7): `canonical_type_blocks` swallows unreadable files inside `if let Ok(text)`, and an empty canonical set reads as "nothing to verify".
5. **Its canonical file list is wrong by omission.** It reads 4 flat files plus one stage file; the `#[slicer_module]` macro `include_str!`s **20** (`prepass-types.wit` and all 15 stage files included). Separately, `crates/slicer-macros/build.rs` emits 9 `cargo:rerun-if-changed` paths of which **4 do not exist** (`deps/world-{prepass,postpass,finalization,layer}/world-*.wit`), and it watches none of the 15 stage files nor `prepass-types.wit` — so 16 of the 20 embedded files trigger no macro rebuild.

`wit-parser = "0.247"` is already a direct dependency of `crates/slicer-runtime` and `crates/slicer-wasm-host` and already resolved in `Cargo.lock`; `crates/slicer-runtime/tests/contract/wit_single_source_tdd.rs` already resolves the canonical WIT dir with `wit_parser::Resolve`. Parsing both sides with it (user-ruled) removes the whole scanner class of defect in one slice, which is why this is one coherent packet rather than five fixes.

## In Scope

- Add `wit-parser = "0.247"` to `xtask/Cargo.toml` `[dependencies]`, version-matched to the existing declarations in `crates/slicer-runtime/Cargo.toml` and `crates/slicer-wasm-host/Cargo.toml`.
- Replace the hand-rolled scanner in `xtask/src/wit_verify.rs` with a `wit_parser`-based model built from **both** sides: the canonical `.wit` tree and the `wasm-tools component wit` decode of an artifact. `extract_type_blocks`, `matching_brace`, `strip_comments`, `normalize`, `BRACED_KEYWORDS`, `ambiguous_type_names` and `canonical_type_blocks` are deleted.
- Model declarations **package-qualified and interface-qualified**: braced types (`variant`/`enum`/`record`/`flags`), aliases (`type X = ...;`), resources (`resource X { ... }` and `resource X;`), interface members (free functions and resource methods), and `use` declarations. Both statement-form and braced-form package syntax parse into the same model.
- Comparison engine, per the amended C3:
  - **Resolved stage package, exported interface** — full equality in both directions (missing and extra declarations both drift).
  - **Resolved stage package, every other interface** — subset direction (embedded ⊆ canonical); whole-member omission is legal, differing bodies are drift. (R5-1: decoded artifacts prove local `*-types` interfaces are not pruned.)
  - **The 5 shared packages** (`slicer:types`, `slicer:config`, `slicer:ir-handles`, `slicer:common`, `slicer:prepass-types`) — subset direction; `use` lists compared as sets.
  - **Any other embedded package** — drift, fail closed. Allowed set is exactly `{root:component}` ∪ the 5 shared ∪ the resolved stage package.
  - **Export name** — compared exactly, version suffix included.
- Declaration **bodies are order-preserving** (R5-8): record field order and variant case order are ABI-relevant. Only interface-level statement ordering may be normalized (declarations keyed by name; `use` sets unordered).
- Fail-closed error model (R5-7): `VerifyError` gains `CanonicalEmpty`, `CanonicalUnreadable { path, reason }` and `Parse { artifact, reason }`. An empty or unreadable canonical set is an infrastructure error, never a pass.
- Canonical coverage audit: a test asserting the verifier's canonical file list is exactly the macro's `include_str!` set. The macro's set is derived by a **multiline-aware** parse of `crates/slicer-macros/src/lib.rs` (many `include_str!` calls wrap across lines), and `crates/slicer-schema/wit/root.wit` is excluded because the macro does not embed it.
- Fix `crates/slicer-macros/build.rs` so its `cargo:rerun-if-changed` list is exactly that same 20-file set: drop the 4 non-existent `deps/world-*/world-*.wit` paths and `root.wit`, add `deps/prepass-types.wit` and all 15 per-stage files.
- Migrate the one existing consumer, `build_one` in `xtask/src/build_guests.rs`, to the new API: add a `BuildError` variant for the new infrastructure-error class, and retype `BuildError::StaleEmbeddedWorld`'s `mismatches` field from `Vec<TypeMismatch>` to `Vec<Drift>` (its `Display` arm moves with it). `module_stage_wit_dir` remains the stage resolver for this packet (packet 230 replaces it).
- Real-artifact clean-gate tests on one prepass and one finalization core-module artifact, asserting rather than skipping whenever `wasm-tools` is on `PATH` and the artifact exists.
- Add the `TASK-340` row to `docs/07_implementation_status.md` under "Workstream 5 — Governance and closure drift".

## Out of Scope

- `check_command`, `test_command`, `build_stale_command`, exit-code contract, `STALE:` reporting — packet 230.
- The fingerprint model, its `v2-` version prefix, its lifecycle, and `build_one`'s residual `if canonical.is_empty() { return Ok(()) }` guard (which the new fail-closed loader makes unreachable, and which packet 230 deletes) — packet 230.
- Artifact-based stage resolution, the `PrePass::PaintSegmentation` empty-`wit_package` exclusion, and retirement of `module_stage_wit_dir` — packet 230.
- The per-guest dependency-closure fingerprint, deletion of `compute_shared_freshness` / `stage_wit_snapshot`, and `crates/pnp-cli-locator::staleness_reason` — packet 231.
- Doc/CI surface: `CLAUDE.md`, `docs/03_wit_and_manifest.md`, `docs/05_module_sdk.md`, the ADRs, the `wasm-staleness` snippet, and adding `cargo test -p xtask` to `.github/workflows/ci.yml` — packet 232.
- Editing any other packet directory, including the six whose ACs use the grep form of the freshness check (user-ruled 2026-08-19).
- Any change to the canonical WIT files themselves.

## Authoritative Docs

- `docs/03_wit_and_manifest.md` — section "Build & Freshness Contract (Normative)" and its staleness-guard table row; direct ranged read.
- `docs/07_implementation_status.md` — over 300 lines; delegate a `LOCATIONS` dispatch for the "Workstream 5 — Governance and closure drift" heading and the current highest `TASK-###`, then append one row.
- `docs/specs/guest-freshness-artifact-verification-plan.md` — the approved plan; read the "Locked decisions" and "Round 5" sections. Round 5 amendments win over conflicting locked-decision text.
- `CLAUDE.md` — sections "Guest WASM Staleness", "In-Tree Citation Style (MUST follow)", "Test Discipline"; direct read.

## Acceptance Summary

Criteria live in `packet.spec.md`; referenced here by ID only.

- Positive: `AC-1` through `AC-14`.
  - `AC-1`/`AC-2` are the canonical-coverage audit pair; both assert against the same derived 20-file set, so a drift in either the verifier or `build.rs` is caught independently.
  - `AC-3`/`AC-4` are the R5-8 order-sensitivity pair and must use real canonical declarations (`record region-key`, `variant extrusion-role`), not invented types.
  - `AC-6`/`AC-7`/`AC-8` are the three comparison directions and must not be collapsed into one test.
  - `AC-11` is the only AC that touches real artifacts; it is the guard against the whole model being self-consistent but wrong.
- Negative: `AC-N1` through `AC-N4` (empty canonical set, unreadable canonical file, unparseable embedded text, missing stage package).
- Cross-packet impact: packet 230 consumes `WorldModel`, `Drift`, `StageExpectation`, `canonical_world_model`, `embedded_world_model`, `compare_worlds` and `VerifyError`'s new variants. Any rename during implementation must be reported so packet 230's design surface is reconciled before it activates.

## Verification Commands

Authoritative full matrix; `packet.spec.md` lists only the 3 closure gates.

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo check --workspace --all-targets` | the new dependency and API compile everywhere, test targets included | FACT pass/fail |
| `cargo clippy --workspace --all-targets -- -D warnings` | required pre-commit gate | FACT pass/fail; SNIPPETS <=20 lines on failure |
| `cargo xtask check-literals` | required pre-commit gate (struct-literal churn) | FACT pass/fail |
| `mkdir -p target && cargo test -p xtask wit_verify 2>&1 \| tee target/test-output.log \| rg '^test result:'` | whole rebuilt verifier suite | FACT pass/fail with the `test result:` line |
| `mkdir -p target && cargo test -p xtask wit_verify::tests::canonical_file_list_equals_macro_include_str_set -- --exact 2>&1 \| tee target/test-output.log \| rg '^test result: ok\. 1 passed'` | AC-1 coverage audit | FACT pass/fail |
| `mkdir -p target && cargo test -p xtask wit_verify::tests::macros_build_rs_watches_the_macro_include_str_set -- --exact 2>&1 \| tee target/test-output.log \| rg '^test result: ok\. 1 passed'` | AC-2 build.rs correction | FACT pass/fail |
| `mkdir -p target && cargo test -p xtask wit_verify::tests::real_core_module_artifacts_verify_clean -- --exact 2>&1 \| tee target/test-output.log \| rg '^test result: ok\. 1 passed'` | AC-11 real-artifact clean gate | FACT pass/fail |
| `cargo xtask build-guests --check` | confirms the `crates/slicer-macros/build.rs` edit's guest-staleness consequence is handled, not discovered later | FACT: clean / list of `STALE:` names |
| see `packet.spec.md` `AC-13` | scanner removal, including `TypeMismatch`, `fn strip_comments` and `fn normalize`; the command is authored in the AC and not duplicated here | FACT PASS/FAIL |
| `rg -q 'TASK-340 ' docs/07_implementation_status.md && echo PASS \|\| echo FAIL` | AC-14 doc impact | FACT PASS/FAIL |

`cargo test --workspace` is **not** part of this packet's matrix. The rebuilt verifier lives entirely in `xtask`, whose tests are `#[cfg(test)] mod tests` blocks inside `xtask/src/*.rs`; `-p xtask` compiles and runs all of them.

## Step Completion Expectations

- Steps 2-5 all edit `xtask/src/wit_verify.rs`. The file must compile at the end of each step; a step that leaves the module non-compiling has not met its exit condition, because the next step's narrow test run would report a false state.
- The `crates/slicer-macros/build.rs` edit (Step 6) changes a guest-WASM input path. Every later step's verification must be preceded by `cargo xtask build-guests --check`, and a `STALE:` report rebuilt before any test failure is attributed to the code.
- `xtask` has no `[lib]` target and no `xtask/tests/` directory. All new tests are added to the existing `#[cfg(test)] mod tests` in `xtask/src/wit_verify.rs`; there is no aggregator file to register them in.
- The 20-file canonical list is derived at runtime by parsing `crates/slicer-macros/src/lib.rs`, never hardcoded in the verifier — a hardcoded copy would make AC-1 tautological.

## Context Discipline Notes

- `xtask/src/wit_verify.rs` and `xtask/src/build_guests.rs` are both long — re-derive their size with `wc -l` before budgeting a read. Read `wit_verify.rs` in full only once, at Step 1; read `build_guests.rs` **only** around `build_one` and `BuildError`, never whole-file.
- `crates/slicer-macros/src/lib.rs` is very large. Never read it — extract the `include_str!` set with a bounded dispatch or a scripted regex over the file, returning `FACT: <count> + sorted paths`.
- `crates/slicer-schema/wit/**` — do not read all 21 files. Read at most `deps/types.wit` and one stage file to author fixtures; the rest are consumed by the parser, not by the implementer.
- Decoded artifact output (`wasm-tools component wit <artifact>`) can run to hundreds of lines. Always pipe through `rg`/`head`; never paste a whole decode into context.
