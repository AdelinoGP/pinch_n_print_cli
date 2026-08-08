# Requirements: 198-literal-sweep-sdk-modules

## Packet Metadata

- Grouped task IDs: `TASK-320`
- Backlog source: `docs/07_implementation_status.md` (new row added at completion; TASK-320 allocated by `docs/specs/struct-literal-churn-gate-plan.md` — re-derive the highest existing TASK id before writing the row)
- Packet status: `draft`
- Aggregate context cost: `M`

## Problem Statement

Queue row #5 of `docs/specs/struct-literal-churn-gate-plan.md`. The guest-side area: `slicer-sdk`'s own test files and the 21 core modules' native test dirs. Sizing estimates (measured 2026-08-07, re-derive from the Step-1 report): 8 sdk test/bench-scope files with `Point3WithWidth` literals, 2 with `LayerCollectionIR`, 1 with `PrintEntity`, 3 with `WallLoop`, 1 with `OrderedEntityView`; 26 module test files across 10 modules (`seam-placer` 7, `infill-linker` 4, `path-optimization-default` 3, `wipe-tower` 3, `fuzzy-skin` 2, `skirt-brim` 2, `support-planner` 2, `arachne-perimeters` 1, `overhang-classifier-default` 1, `part-cooling` 1). One coherent slice: both sides consume the same `slicer_sdk::test_support` fixture home (ADR-0004/0054 "single IR-fixture home"), and the sdk manifest gating decision affects both.

## In Scope

- Convert exhaustive watched-type struct literals to FRU (`..Default::default()` or `..<fixture>()`) in:
  - `crates/slicer-sdk/tests/**` (27 files today; 17 already `[[test]]`-gated on `required-features = ["test"]`, the rest auto-discovered — measured 2026-08-07, re-derive);
  - `crates/slicer-sdk/src/test_support/**` where the Step-1 report flags test-scope literals (fixture bases themselves keep exhaustive literals WITH waivers — they are deliberate propagation checkpoints, the packet-195 precedent);
  - `modules/core-modules/*/tests/**` (all 21 modules' native test dirs; the 10 modules above are the measured candidates).
- Conversion rule (locked decision 2; `docs/21_data_defaults_and_fixtures.md` normative): meaningful non-default fields stay spelled, base-equal fields are OMITTED, `..Default::default()` / `..<fixture>()` appended; never spell-all-plus-FRU (`clippy::needless_update`); never change what a test asserts.
- Fixture routing (packet-195 exports, consumed not re-decided): `PrintEntity` → `print_entity_base(role)`; `WallLoop` → `wall_loop_base(loop_type, boundary_type)`; `OrderedEntityView` → `ordered_entity_view_base(role)`; `Point3WithWidth`/`GlobalLayer`/`LayerCollectionIR` → plain FRU over their verified `Default`s.
- Sdk manifest gating: for each `crates/slicer-sdk/tests/*.rs` newly referencing `test_support`, add a `[[test]] name = "<file>" required-features = ["test"]` entry to `crates/slicer-sdk/Cargo.toml` (grounded candidates: `layer_module_tdd.rs` — constructs `OrderedEntityView`/`WallLoop`/`Point3WithWidth`; `finalization_builder_tdd.rs` — constructs `PrintEntity`; `finalization_module_tdd.rs` needs only `LayerCollectionIR` FRU, so it likely stays ungated; re-derive all three from the report).
- All sdk suite invocations in this packet use `--features test` so baseline and post runs compile the identical target set (gating moves files between bare-run visibility classes; `--features test` sees both classes both times).
- Module tests consume fixtures through their existing dev-dep `slicer-sdk = { path = ..., features = ["test"] }` (verified present in all 21 module manifests 2026-08-07; zero `required-features` entries exist or are needed there — the dev-dep enables the feature for all test targets).
- Guest freshness: the sdk `Cargo.toml` edit trips `shared_input_paths` (`xtask/src/build_guests.rs`); after the manifest edit, run `cargo xtask build-guests` (rebuild), then `--check` must be clean at close.
- Baseline capture before any edit (sdk suite summary with `--features test`, per-module aggregate summary, assert/`#[test]` counts, module list) into `target/sweep-198-*`; post-sweep invariance proof.

## Out of Scope

- Any assertion, expected-value, or fixture-semantic change; fixture base VALUES are packet-195 contract (e.g. `print_entity_base` returns `entity_id 0`, 1-point path) — never adjusted here.
- Adding `Default` to `PrintEntity`, `WallLoop`, `Diagnostic` (both crates), `DeferredRetract`, `DeferredTravelMove`, `OrderedEntityView` (packet-195 locks; AC-N1).
- Editing `crates/slicer-sdk/src/**` outside `test_support/**` (the sdk's production API and prelude are untouched; `src/test_support/fixtures.rs` signatures are packet-195 contract — waiver comments inside are the only permitted edit there).
- Enabling `feature = "test"` for any guest/wasm build: guests never enable it (ADR-0004 amendment); module [dependencies] (as opposed to [dev-dependencies]) stay clean of it.
- `modules/core-modules/*/src/**`, `modules/core-modules/*/wit-guest/**`, `modules/core-modules/*/Cargo.toml` (no change expected; AC-N4 guards manifests).
- Sweeps of other areas (packets 196/197) and residue crates `slicer-helpers`/`slicer-model-io`/`slicer-macros` (packet 199 absorbs).
- Enforcement wiring (packet 199); the checker itself (`xtask/src/check_literals.rs`, packet 194 — defects are deviations).

## Authoritative Docs

- `docs/specs/struct-literal-churn-gate-plan.md` - 89 lines; direct read.
- `docs/21_data_defaults_and_fixtures.md` - authored by packet 194; direct read at implementation time; delegate SUMMARY if over 300 lines.
- `docs/adr/0004-test-support-lives-in-slicer-sdk.md` - 72 lines + packet-195 amendment; direct read (guests never enable `test`).
- `docs/adr/0054-host-side-test-support-crate.md` - direct read of the packet-195 amendment section only.
- `CLAUDE.md` §Guest WASM Staleness, §Test Discipline - named sections only.

## Acceptance Summary

Reference, never copy, criteria from `packet.spec.md`.

- Positive: `AC-1` through `AC-7`.
- Negative: `AC-N1` through `AC-N4`.
- Cross-packet impact: AC-1's clean area is the final third of packet 199's enforcement precondition; AC-4's gating pattern extends the sdk's existing 17-entry convention (a post-198 bare `cargo test -p slicer-sdk` skips more binaries — same documented hazard class, no new doc needed since CLAUDE.md's feature-gating section already teaches the reconciliation rule); waiver inventory feeds 199's audit.

## Verification Commands

This is the authoritative full matrix; `packet.spec.md` lists only 2-3 gate commands.

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo xtask check-literals --report crates/slicer-sdk modules/core-modules \| tee target/sweep-198-report.txt \| tail -1` | Step-1 enumeration (report mode exits 0) | FACT: summary line |
| `cargo xtask check-literals crates/slicer-sdk; test $? -eq 0 && echo PASS` | per-area green after the sdk step | FACT PASS/FAIL |
| `cargo xtask check-literals modules/core-modules; test $? -eq 0 && echo PASS` | per-area green after the module steps | FACT PASS/FAIL |
| AC-1 through AC-7, AC-N1 through AC-N4 pipe-suffixed commands (see `packet.spec.md`) | packet acceptance | FACT PASS/FAIL each; on suite FAIL, `grep -E 'FAILED\|panicked at' -C 5 target/test-output.log` SNIPPETS ≤20 lines |
| `cargo test -p slicer-sdk --features test --test layer_module_tdd 2>&1 \| tee target/test-output.log \| grep -E '^test result'` | narrow mid-sweep check after gating that file | FACT: summary line |
| `cargo xtask build-guests` then `cargo xtask build-guests --check; test $? -eq 0 && echo PASS` | rebuild after the sdk manifest edit; clean gate at close | FACT PASS/FAIL |
| `cargo check --workspace --all-targets` | compile gate | FACT pass/fail |
| `cargo clippy --workspace --all-targets -- -D warnings` | lint gate; catches `needless_update` misconversions | FACT pass/fail |

All `cargo test` invocations tee to `target/test-output.log` (append with `tee -a` inside the module loop); inspect the log, never re-run for output.

## Step Completion Expectations

- Step 1 writes the shared scratch state: `target/sweep-198-report.txt`, `target/sweep-198-slicer-sdk-baseline.txt`, `target/sweep-198-modules.txt` (module crate names, one per line, derived from report paths `modules/core-modules/<name>/tests/...`), `target/sweep-198-modules-baseline.txt` (aggregate sorted multiset over exactly the modules in the list, in list order), `target/sweep-198-assert-baseline.txt`, `target/sweep-198-testattr-baseline.txt`. Implementation-time scratch, never committed, never quoted into the packet as frozen counts.
- The sdk baseline MUST be captured with `--features test` (AC-2's stated reason); the module baseline loop MUST iterate `target/sweep-198-modules.txt` in file order so baseline and post multisets aggregate identically.
- A pre-existing red baseline test halts the packet (blocker), never swept over.
- The sdk manifest edit precedes the guest rebuild; the rebuild precedes the close gate (AC-5). Module test runs do not depend on the rebuild (they are native), so step order between module sweeps and the rebuild is free.

## Context Discipline Notes

- `target/sweep-198-report.txt`: grep per module (`grep '^modules/core-modules/seam-placer/'`), never read whole.
- `crates/slicer-sdk/Cargo.toml` gating section is long (17+ `[[test]]` entries): append new entries; read only the tail.
- Do not open packets 194/195's `design.md`/`implementation-plan.md`; their exports are restated here and in `design.md`.
- The guest rebuild output is long; dispatch it and consume a FACT (clean / STALE list length only).
