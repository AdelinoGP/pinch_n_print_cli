---
status: draft
packet: 197-literal-sweep-host-runtime
task_ids:
  - TASK-319
backlog_source: docs/07_implementation_status.md
context_cost_estimate: M
---

# Packet Contract: 197-literal-sweep-host-runtime

## Goal

Convert every `cargo xtask check-literals` violation in `slicer-runtime`, `slicer-scheduler`, `slicer-wasm-host`, and `pnp-cli` test code to FRU-over-a-base — including the 14 `PipelineConfig` sites in `crates/slicer-runtime/tests/integration/pipeline_tdd.rs` routed through `common::pipeline_config_base` and the `pnp-cli` e2e sites routed through their packet-195 file-local twin — so `cargo xtask check-literals crates/slicer-runtime crates/slicer-scheduler crates/slicer-wasm-host crates/pnp-cli` exits 0 with every suite green and test counts unchanged.

## Scope Boundaries

Construction-syntax-only edits to test code (integration tests, benches, `#[cfg(test)]` mods in src) of the four host crates, plus one `[dev-dependencies]` addition to `crates/pnp-cli/Cargo.toml` and removal of the `#[allow(dead_code)]` on the pnp-cli `pipeline_config_base` twin. No assertion, fixture-semantic, or production-code change; no `Default` impl added anywhere; `crates/slicer-wasm-host/test-guests/**` untouched (rule-exempt WIT adapter shims); enforcement wiring is packet 199; other areas are packets 196/198.

## Prerequisites and Blockers

- Depends on: packet 194 (`cargo xtask check-literals`), packet 195 (`SliceRunOptions` `Default`, `slicer_sdk::test_support::fixtures`, `common::pipeline_config_base` in `crates/slicer-runtime/tests/common/mod.rs`, the pnp-cli file-local twin). Both must be `implemented` before activation.
- Unblocks: packet 199.
- Activation blockers: packets 194 and 195 not yet `implemented`.

## Acceptance Criteria

State ACs only here; `requirements.md` references their IDs.

- **AC-1. Given** the swept tree, **when** the area gate runs in enforce mode, **then** it exits 0 across the four crate roots (tests, benches, `#[cfg(test)]` mods in src). | `cargo xtask check-literals crates/slicer-runtime crates/slicer-scheduler crates/slicer-wasm-host crates/pnp-cli; test $? -eq 0 && echo PASS`
- **AC-2. Given** the pre-sweep baseline `target/sweep-197-slicer-runtime-baseline.txt` (captured in Step 1 via the identical pipeline), **when** the `slicer-runtime` suite re-runs post-sweep, **then** every `test result` line reports `0 failed` and the sorted, time-stripped summary multiset is byte-identical to the baseline. | `mkdir -p target && cargo test -p slicer-runtime 2>&1 | tee target/test-output.log >/dev/null; grep -E '^test result' target/test-output.log | sed 's/; finished in .*//' | sort > target/sweep-197-slicer-runtime-post.txt; test "$(grep -vc ' 0 failed' target/sweep-197-slicer-runtime-post.txt)" -eq 0 && diff target/sweep-197-slicer-runtime-baseline.txt target/sweep-197-slicer-runtime-post.txt && echo PASS`
- **AC-3. Given** the pre-sweep baselines for the other three crates, **when** their suites re-run post-sweep, **then** each is green with an unchanged summary multiset (same pipeline per crate). | `for c in slicer-scheduler slicer-wasm-host pnp-cli; do mkdir -p target && cargo test -p $c 2>&1 | tee target/test-output.log >/dev/null; grep -E '^test result' target/test-output.log | sed 's/; finished in .*//' | sort > target/sweep-197-$c-post.txt; test "$(grep -vc ' 0 failed' target/sweep-197-$c-post.txt)" -eq 0 && diff target/sweep-197-$c-baseline.txt target/sweep-197-$c-post.txt || { echo FAIL:$c; exit 1; }; done; echo PASS`
- **AC-4. Given** `crates/slicer-runtime/tests/integration/pipeline_tdd.rs` (14 exhaustive `PipelineConfig` literals measured 2026-08-07; re-derive in Step 1), **when** grepped post-sweep, **then** it calls `pipeline_config_base` (any name-resolution-equivalent form: `common::pipeline_config_base(...)`, an imported `pipeline_config_base(...)`, or FRU `..common::pipeline_config_base(...)`) and contains no waiver comment (nothing there needs one). | `rg -q 'pipeline_config_base\(' crates/slicer-runtime/tests/integration/pipeline_tdd.rs && ! rg -q '// exhaustive:' crates/slicer-runtime/tests/integration/pipeline_tdd.rs && echo PASS`
- **AC-5. Given** the packet-195 file-local twin `fn pipeline_config_base` in `crates/pnp-cli/tests/e2e_integration_tdd.rs` (landed with `#[allow(dead_code)]`; NOTE the file also carries an unrelated pre-existing `#[allow(dead_code)]` on `fn make_global_layer` which must survive untouched — the check is anchored to the attribute-fn adjacency so it cannot trip on it), **when** grepped post-sweep, **then** no `dead_code` allow sits directly on the twin and the twin is called at least once (any form: `pipeline_config_base(...)` call or `..pipeline_config_base(...)` FRU base). | `! rg -U -q 'allow\(dead_code\)\]\s*(pub\s+)?fn pipeline_config_base' crates/pnp-cli/tests/e2e_integration_tdd.rs && test "$(rg -c 'pipeline_config_base' crates/pnp-cli/tests/e2e_integration_tdd.rs)" -ge 2 && echo PASS`
- **AC-6. Given** `crates/pnp-cli/Cargo.toml`, **when** grepped, **then** `[dev-dependencies]` contains `slicer-sdk` with `features = ["test"]`, and at least one `pnp-cli` test file calls `print_entity_base` (any name-resolution-equivalent form). | `rg -q 'slicer.sdk.*features\s*=\s*\[\s*"test"' crates/pnp-cli/Cargo.toml && rg -q 'print_entity_base' crates/pnp-cli/tests && echo PASS`
- **AC-7. Given** `SliceRunOptions` construction sites in `slicer-runtime` tests (packet 195 gave it `Default`), **when** grepped post-sweep, **then** no exhaustive `SliceRunOptions` literal remains: every `SliceRunOptions {` literal in `crates/slicer-runtime/tests` and `crates/pnp-cli/tests` contains a `..` rest (verified by the area gate in AC-1) and at least one converted site uses `..Default::default()` or `..SliceRunOptions::default()`. | `rg -q 'SliceRunOptions \{' crates/slicer-runtime/tests && rg -q '\.\.(SliceRunOptions::)?[Dd]efault' crates/slicer-runtime/tests && echo PASS`

## Negative Test Cases

- **AC-N1. Given** the packet-195 locks, **when** the tree is grepped post-sweep, **then** none of `PrintEntity`, `WallLoop`, `Diagnostic`, `DeferredRetract`, `DeferredTravelMove` gained a `Default` impl. | `! rg -q 'impl Default for PrintEntity|impl Default for WallLoop|impl Default for Diagnostic|impl Default for DeferredRetract|impl Default for DeferredTravelMove' crates && echo PASS`
- **AC-N2. Given** the frozen waiver format, **when** the four crates are grepped, **then** no waiver comment has an empty reason. | `rg -n '// exhaustive:[[:space:]]*$' crates/slicer-runtime crates/slicer-scheduler crates/slicer-wasm-host crates/pnp-cli; test $? -eq 1 && echo PASS`
- **AC-N3. Given** the pre-sweep counts in `target/sweep-197-assert-baseline.txt` and `target/sweep-197-testattr-baseline.txt` (captured in Step 1 via the identical pipelines; the count scope excludes `crates/slicer-wasm-host/test-guests`), **when** re-counted post-sweep, **then** both counts are unchanged. | `a=$(rg -o 'assert(_eq|_ne)?!' crates/slicer-runtime crates/slicer-scheduler crates/slicer-wasm-host crates/pnp-cli -g '!**/test-guests/**' | wc -l); t=$(rg -o '#\[test\]' crates/slicer-runtime crates/slicer-scheduler crates/slicer-wasm-host crates/pnp-cli -g '!**/test-guests/**' | wc -l); test "$a" = "$(cat target/sweep-197-assert-baseline.txt)" && test "$t" = "$(cat target/sweep-197-testattr-baseline.txt)" && echo PASS`
- **AC-N4. Given** the rule exemption for WIT adapter shims, **when** the diff is inspected at close, **then** `crates/slicer-wasm-host/test-guests/**` is untouched. | `git status --porcelain crates/slicer-wasm-host/test-guests | wc -l | grep -qx '0' && echo PASS`

## Verification

- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo xtask check-literals crates/slicer-runtime crates/slicer-scheduler crates/slicer-wasm-host crates/pnp-cli; test $? -eq 0 && echo PASS`

## Authoritative Docs

- `docs/specs/struct-literal-churn-gate-plan.md` - short; direct read; locked decisions 1-3 govern.
- `docs/21_data_defaults_and_fixtures.md` - authored by packet 194; direct read at implementation time.
- `CLAUDE.md` §Test Discipline (tee rule; no `cargo test --workspace`), §Guest WASM Staleness (why this packet's surface does NOT trip the gate — see `design.md`).

## Doc Impact Statement (Required)

- **`none`** - construction-syntax-only refactor of test code plus one dev-dependency line; no IR, WIT, scheduler, claim, manifest, host-service, or SDK contract changes.

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- obey the shared absolute context bands: 120k reading budget with hand-off at 150k (standard); the extended band (240k reading / 300k hard stop) only via swarm's escalation protocol

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation (an extended-band run may carry a single L step only when `design.md` justifies why it cannot be split).
