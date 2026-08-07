---
status: draft
packet: 198-literal-sweep-sdk-modules
task_ids:
  - TASK-320
backlog_source: docs/07_implementation_status.md
context_cost_estimate: M
---

# Packet Contract: 198-literal-sweep-sdk-modules

## Goal

Convert every `cargo xtask check-literals` violation in `slicer-sdk` and `modules/core-modules/*/tests` test code to FRU-over-a-base (sdk fixtures / `Default`) or a reasoned waiver — gating the sdk's own not-yet-gated fixture-consuming test files via `[[test]] required-features = ["test"]` entries — so `cargo xtask check-literals crates/slicer-sdk modules/core-modules` exits 0 with every suite green, test counts unchanged, and guests rebuilt fresh after the sdk manifest edit.

## Scope Boundaries

Construction-syntax-only edits to `crates/slicer-sdk/tests/**`, any reported test-scope literals in `crates/slicer-sdk/src/test_support/**`, and `modules/core-modules/*/tests/**`, plus `[[test]]` gating entries in `crates/slicer-sdk/Cargo.toml`. All 21 module manifests already carry the sdk dev-dep with `features = ["test"]` (verified 2026-08-07), so no module `Cargo.toml` changes are expected; the sdk manifest edit is a guest-WASM input, so this packet carries the freshness gate. No assertion changes, no `Default` additions, no enforcement wiring (packet 199).

## Prerequisites and Blockers

- Depends on: packet 194 (`cargo xtask check-literals`), packet 195 (`slicer_sdk::test_support::fixtures::{print_entity_base, wall_loop_base, ordered_entity_view_base}`). Both must be `implemented` before activation.
- Unblocks: packet 199.
- Activation blockers: packets 194 and 195 not yet `implemented`.

## Acceptance Criteria

State ACs only here; `requirements.md` references their IDs.

- **AC-1. Given** the swept tree, **when** the area gate runs in enforce mode, **then** it exits 0 across `crates/slicer-sdk` and `modules/core-modules` (tests, benches, `#[cfg(test)]` mods in src; module manifests' watchlist contribution is nil — the watchlist derives only from `crates/*/src`). | `cargo xtask check-literals crates/slicer-sdk modules/core-modules; test $? -eq 0 && echo PASS`
- **AC-2. Given** the pre-sweep baseline `target/sweep-198-slicer-sdk-baseline.txt` (captured in Step 1 with `--features test` — bare `-p slicer-sdk` silently skips the 17+ `required-features = ["test"]` targets, same hazard class as slicer-core's `host-algos`), **when** the sdk suite re-runs post-sweep with `--features test`, **then** every `test result` line reports `0 failed` and the sorted, time-stripped summary multiset is byte-identical to the baseline. | `mkdir -p target && cargo test -p slicer-sdk --features test 2>&1 | tee target/test-output.log >/dev/null; grep -E '^test result' target/test-output.log | sed 's/; finished in .*//' | sort > target/sweep-198-slicer-sdk-post.txt; test "$(grep -vc ' 0 failed' target/sweep-198-slicer-sdk-post.txt)" -eq 0 && diff target/sweep-198-slicer-sdk-baseline.txt target/sweep-198-slicer-sdk-post.txt && echo PASS`
- **AC-3. Given** the module list `target/sweep-198-modules.txt` (one crate name per line, derived in Step 1 from the report's `modules/core-modules/<name>/tests/` paths), **when** each listed module's suite re-runs post-sweep, **then** every `test result` line reports `0 failed` and the aggregate summary multiset matches the Step-1 baseline `target/sweep-198-modules-baseline.txt`. | `mkdir -p target && rm -f target/sweep-198-modules-post-raw.txt; while read -r m; do cargo test -p "$m" 2>&1 | tee -a target/sweep-198-modules-post-raw.txt >/dev/null; done < target/sweep-198-modules.txt; grep -E '^test result' target/sweep-198-modules-post-raw.txt | sed 's/; finished in .*//' | sort > target/sweep-198-modules-post.txt; test "$(grep -vc ' 0 failed' target/sweep-198-modules-post.txt)" -eq 0 && diff target/sweep-198-modules-baseline.txt target/sweep-198-modules-post.txt && echo PASS`
- **AC-4. Given** the sdk gating rule (every `crates/slicer-sdk/tests/*.rs` file that references `test_support` must be a `[[test]]` target with `required-features = ["test"]` — otherwise a bare `cargo test -p slicer-sdk` fails to compile it), **when** each sdk test file is checked against `crates/slicer-sdk/Cargo.toml`, **then** no fixture-consuming file is missing its gating entry (grounded candidates measured 2026-08-07, re-derive: `layer_module_tdd.rs`, `finalization_builder_tdd.rs`). | `ok=1; for f in crates/slicer-sdk/tests/*.rs; do rg -q 'test_support' "$f" || continue; n=$(basename "${f%.rs}"); rg -A2 "name = \"$n\"" crates/slicer-sdk/Cargo.toml | rg -q 'required-features' || { echo "MISSING: $n"; ok=0; }; done; test $ok -eq 1 && echo PASS`
- **AC-5. Given** that `crates/slicer-sdk/Cargo.toml` is a guest-WASM input (`shared_input_paths` in `xtask/src/build_guests.rs` collects shared-crate `Cargo.toml` files), **when** the freshness gate runs at close (after rebuilding without `--check` — `STALE:` is EXPECTED right after the manifest edit), **then** it reports clean. | `cargo xtask build-guests --check; test $? -eq 0 && echo PASS`
- **AC-6. Given** a converted module test site, **when** module test trees are grepped post-sweep, **then** at least one module test calls a packet-195 fixture (any name-resolution-equivalent form of `print_entity_base`, `wall_loop_base`, or `ordered_entity_view_base`: fully-qualified `slicer_sdk::test_support::fixtures::<f>`, imported `fixtures::<f>`, or bare `<f>(`). | `rg -q 'print_entity_base|wall_loop_base|ordered_entity_view_base' modules/core-modules/*/tests && echo PASS`
- **AC-7. Given** bare-run integrity after gating, **when** `cargo test -p slicer-sdk` runs WITHOUT `--features test`, **then** it still compiles and passes (newly gated files are skipped, not broken; nothing ungated references `test_support`). | `mkdir -p target && cargo test -p slicer-sdk 2>&1 | tee target/test-output.log >/dev/null; test "$(grep -E '^test result' target/test-output.log | grep -vc ' 0 failed')" -eq 0 && echo PASS`

## Negative Test Cases

- **AC-N1. Given** the packet-195 locks, **when** the tree is grepped post-sweep, **then** none of `PrintEntity`, `WallLoop`, `Diagnostic`, `DeferredRetract`, `DeferredTravelMove` gained a `Default` impl, and `OrderedEntityView` gained none either (it is class (b): fixture, not `Default`). | `! rg -q 'impl Default for PrintEntity|impl Default for WallLoop|impl Default for Diagnostic|impl Default for DeferredRetract|impl Default for DeferredTravelMove|impl Default for OrderedEntityView' crates && echo PASS`
- **AC-N2. Given** the frozen waiver format, **when** the area is grepped, **then** no waiver comment has an empty reason. | `rg -n '// exhaustive:[[:space:]]*$' crates/slicer-sdk modules/core-modules; test $? -eq 1 && echo PASS`
- **AC-N3. Given** the pre-sweep counts in `target/sweep-198-assert-baseline.txt` and `target/sweep-198-testattr-baseline.txt` (captured in Step 1 via the identical pipelines; scope excludes `wit-guest` shims and built `.wasm` artifacts by construction — the count runs over `crates/slicer-sdk` and `modules/core-modules/*/tests` only), **when** re-counted post-sweep, **then** both counts are unchanged. | `a=$(rg -o 'assert(_eq|_ne)?!' crates/slicer-sdk modules/core-modules/*/tests | wc -l); t=$(rg -o '#\[test\]' crates/slicer-sdk modules/core-modules/*/tests | wc -l); test "$a" = "$(cat target/sweep-198-assert-baseline.txt)" && test "$t" = "$(cat target/sweep-198-testattr-baseline.txt)" && echo PASS`
- **AC-N4. Given** that no module manifest change is expected (all 21 already dev-dep the sdk with `features = ["test"]`, verified 2026-08-07), **when** the diff is inspected at close, **then** no `modules/core-modules/*/Cargo.toml` changed; if implementation finds one that genuinely must change, this AC is renegotiated as a deviation BEFORE the edit, and the freshness gate (AC-5) already covers the fallout. | `git status --porcelain modules/core-modules/*/Cargo.toml | wc -l | grep -qx '0' && echo PASS`

## Verification

- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo xtask check-literals crates/slicer-sdk modules/core-modules; test $? -eq 0 && echo PASS`

## Authoritative Docs

- `docs/specs/struct-literal-churn-gate-plan.md` - short; direct read; locked decisions 1-3 govern.
- `docs/21_data_defaults_and_fixtures.md` - authored by packet 194; direct read at implementation time.
- `docs/adr/0004-test-support-lives-in-slicer-sdk.md` + `docs/adr/0054-host-side-test-support-crate.md` - "single IR-fixture home" amendments (packet 195); guests never enable `feature = "test"`.
- `CLAUDE.md` §Guest WASM Staleness - the sdk-manifest trigger and rebuild protocol.

## Doc Impact Statement (Required)

- **`none`** - construction-syntax-only refactor of test code plus sdk `[[test]]` gating entries; no IR, WIT, scheduler, claim, manifest-schema, host-service, or SDK API contract changes (the `test` feature's public surface is unchanged; only which test binaries require it).

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- obey the shared absolute context bands: 120k reading budget with hand-off at 150k (standard); the extended band (240k reading / 300k hard stop) only via swarm's escalation protocol

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation (an extended-band run may carry a single L step only when `design.md` justifies why it cannot be split).
