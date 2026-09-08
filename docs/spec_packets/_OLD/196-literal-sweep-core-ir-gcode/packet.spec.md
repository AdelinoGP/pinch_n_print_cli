---
status: implemented
packet: 196-literal-sweep-core-ir-gcode
task_ids:
  - TASK-318
backlog_source: docs/07_implementation_status.md
context_cost_estimate: M
---

# Packet Contract: 196-literal-sweep-core-ir-gcode

## Goal

Convert every `cargo xtask check-literals` violation in `slicer-ir`, `slicer-core`, and `slicer-gcode` test code to FRU-over-a-base (or a reasoned `// exhaustive:` waiver where exhaustiveness is the test's intent), so the area gate `cargo xtask check-literals crates/slicer-ir crates/slicer-core crates/slicer-gcode` exits 0 with every suite green and test counts unchanged.

## Scope Boundaries

Construction-syntax-only edits to test code (integration tests, benches, `#[cfg(test)]` mods in src) of the three crates, plus one `[dev-dependencies]` addition to `crates/slicer-gcode/Cargo.toml`. No assertion, fixture-semantic, or production-code change; no `Default` impl added anywhere; no enforcement wiring (packet 199); other crates' sweeps belong to packets 197/198.

## Prerequisites and Blockers

- Depends on: packet 194 (`cargo xtask check-literals` CLI, waiver format, `docs/21_data_defaults_and_fixtures.md`), packet 195 (`slicer_sdk::test_support::fixtures::print_entity_base`, ADR "single IR-fixture home" addendum). Both must be `implemented` before this packet activates — every AC below invokes the 194 tool or a 195 fixture.
- Unblocks: packet 199 (gate flip needs all three areas green).

## Acceptance Criteria

State ACs only here; `requirements.md` references their IDs.

- **AC-1. Given** the swept tree, **when** the area gate runs in enforce mode, **then** it exits 0 (zero violations across `crates/slicer-ir`, `crates/slicer-core`, `crates/slicer-gcode`, covering `tests/`, `benches/`, and `#[cfg(test)]` mods in src). | `cargo xtask check-literals crates/slicer-ir crates/slicer-core crates/slicer-gcode; test $? -eq 0 && echo PASS`
- **AC-2. Given** the pre-sweep baseline file `target/sweep-196-slicer-ir-baseline.txt` (captured in Step 1 via the identical pipeline), **when** the `slicer-ir` suite re-runs post-sweep, **then** every `test result` line reports `0 failed` and the sorted, time-stripped summary multiset is byte-identical to the baseline. | `mkdir -p target && cargo test -p slicer-ir 2>&1 | tee target/test-output.log >/dev/null; grep -E '^test result: ' target/test-output.log | sed 's/; finished in .*//' | sort > target/sweep-196-slicer-ir-post.txt; test "$(grep -vc ' 0 failed' target/sweep-196-slicer-ir-post.txt)" -eq 0 && diff target/sweep-196-slicer-ir-baseline.txt target/sweep-196-slicer-ir-post.txt && echo PASS`
- **AC-3. Given** the pre-sweep baseline file `target/sweep-196-slicer-core-baseline.txt`, **when** the `slicer-core` suite re-runs post-sweep **with `--features host-algos`** (bare `-p slicer-core` silently compiles the 11 `required-features = ["host-algos"]` targets and the `#![cfg(feature = "host-algos")]` arachne files to zero tests — CLAUDE.md §Feature-gated test files; a 12th target gated on `required-features = ["host-algos", "voronoi-panic-regression"]` stays unbuilt under `--features host-algos` alone, identically in baseline and post runs), **then** every `test result` line reports `0 failed` and the summary multiset matches the baseline. | `mkdir -p target && cargo test -p slicer-core --features host-algos --no-fail-fast 2>&1 | tee target/test-output.log >/dev/null; grep -E '^test result: ' target/test-output.log | sed 's/; finished in .*//' | sort > target/sweep-196-slicer-core-post.txt; test "$(grep -vc ' 0 failed' target/sweep-196-slicer-core-post.txt)" -eq 0 && diff target/sweep-196-slicer-core-baseline.txt target/sweep-196-slicer-core-post.txt && echo PASS`
- **AC-4. Given** the pre-sweep baseline file `target/sweep-196-slicer-gcode-baseline.txt`, **when** the `slicer-gcode` suite re-runs post-sweep, **then** every `test result` line reports `0 failed` and the summary multiset matches the baseline. | `mkdir -p target && cargo test -p slicer-gcode 2>&1 | tee target/test-output.log >/dev/null; grep -E '^test result: ' target/test-output.log | sed 's/; finished in .*//' | sort > target/sweep-196-slicer-gcode-post.txt; test "$(grep -vc ' 0 failed' target/sweep-196-slicer-gcode-post.txt)" -eq 0 && diff target/sweep-196-slicer-gcode-baseline.txt target/sweep-196-slicer-gcode-post.txt && echo PASS`
- **AC-5. Given** `crates/slicer-gcode/Cargo.toml`, **when** grepped, **then** `[dev-dependencies]` contains `slicer-sdk` with `features = ["test"]`, and at least one `slicer-gcode` test file calls the packet-195 fixture `print_entity_base` (any name-resolution-equivalent form: fully-qualified `slicer_sdk::test_support::fixtures::print_entity_base`, a `use`-imported `fixtures::print_entity_base`, or bare `print_entity_base(`). | `rg -q 'slicer.sdk.*features\s*=\s*\[\s*"test"' crates/slicer-gcode/Cargo.toml && rg -q 'print_entity_base' crates/slicer-gcode/tests && echo PASS`
- **AC-6. Given** that this sweep may edit `#[cfg(test)]` mods inside `crates/slicer-ir/src/slice_ir.rs` and `crates/slicer-core/src/**` (both guest-WASM input paths per CLAUDE.md §Guest WASM Staleness), **when** the freshness gate runs at close (after rebuilding without `--check` if `STALE:` was reported), **then** it reports clean. | `cargo xtask build-guests --check; test $? -eq 0 && echo PASS`

## Negative Test Cases

- **AC-N1. Given** the packet-195 lock that unsafe-default IR types must not gain `Default`, **when** the tree is grepped post-sweep, **then** `PrintEntity` and `WallLoop` still have no `Default` impl and `PrintEntity`'s "intentionally has no `Default` derive" doc comment survives (the sweep converted call sites; it did not "fix" types). | `! rg -q 'impl Default for PrintEntity|impl Default for WallLoop' crates && rg -q 'intentionally has no .Default. derive' crates/slicer-ir/src/slice_ir.rs && echo PASS`
- **AC-N2. Given** the frozen waiver format (`// exhaustive: <reason>`, reason mandatory), **when** the three crates are grepped, **then** no waiver comment has an empty reason. | `rg -n '// exhaustive:[[:space:]]*$' crates/slicer-ir crates/slicer-core crates/slicer-gcode; test $? -eq 1 && echo PASS`
- **AC-N3. Given** the pre-sweep counts in `target/sweep-196-assert-baseline.txt` and `target/sweep-196-testattr-baseline.txt` (captured in Step 1 via the identical pipelines), **when** `assert!`/`assert_eq!`/`assert_ne!` occurrences and `#[test]` attributes are re-counted across the three crate roots post-sweep, **then** both counts are unchanged (conversion is construction-syntax-only; no assertion added, removed, or weakened). | `a=$(rg -o 'assert(_eq|_ne)?!' crates/slicer-ir crates/slicer-core crates/slicer-gcode | wc -l); t=$(rg -o '#\[test\]' crates/slicer-ir crates/slicer-core crates/slicer-gcode | wc -l); test "$a" = "$(cat target/sweep-196-assert-baseline.txt)" && test "$t" = "$(cat target/sweep-196-testattr-baseline.txt)" && echo PASS`

## Verification

- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo xtask check-literals crates/slicer-ir crates/slicer-core crates/slicer-gcode; test $? -eq 0 && echo PASS`

## Authoritative Docs

- `docs/specs/struct-literal-churn-gate-plan.md` - short; direct read; locked decisions 1-3 govern the conversion rule and waiver policy.
- `docs/21_data_defaults_and_fixtures.md` - authored by packet 194; direct read at implementation time (conversion rule, waiver format, `clippy::needless_update` guidance).
- `CLAUDE.md` §Test Discipline, §Guest WASM Staleness - direct read of named sections only (host-algos invocation form; guest-input path list).

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
