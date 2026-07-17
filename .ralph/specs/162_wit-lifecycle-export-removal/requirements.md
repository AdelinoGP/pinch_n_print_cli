# Requirements: 162_wit-lifecycle-export-removal

## Packet Metadata

- Grouped task IDs: `TASK-146a`
- Backlog source: `docs/07_implementation_status.md:37-39` (the governing TASK-144/145/146 slice)
- Packet status: `draft`
- Aggregate context cost: `M`

## Problem Statement

`on-print-start` / `on-print-end` are padding squatting on a real concept's name. Grounded against the tree:

- `call_on_print_start` / `call_on_print_end` have **zero callers in the host** — the exports are declared, generated, shipped in all 20 guest `.wasm` artifacts, and never invoked.
- The macro's `on_print_end` glue is hardcoded `fn on_print_end() -> Result<(), ModuleError> { Ok(()) }` (`crates/slicer-macros/src/lib.rs:2771`) and never dispatches to the trait. Every module's `on_print_end` body is unreachable; only `arachne-perimeters` (`modules/core-modules/arachne-perimeters/src/lib.rs:419`) even has one.
- The macro's `on_print_start` glue does `Ok(_m) => Ok(())` (`:2766-2769`) — it constructs the module and discards it. There is no `OnceCell`/`OnceLock`/`static mut`/`thread_local` anywhere in the macro, and all 15 `run_*` arms reconstruct the module per call. `docs/05_module_sdk.md:184`'s "initialize expensive resources once per print" is therefore inverted: it runs once per **layer**, per **stage**.
- `crates/slicer-schema/src/lib.rs:230` `WORLD_LIFECYCLE_EXPORTS` claims all four worlds ship lifecycle exports. Only `world-layer.wit:20-21` declares them; prepass/postpass/finalization declare none. Its guard test `every_world_has_lifecycle_exports` (`:428`) reads that table and asserts against the *same* table — it passes vacuously. This is the identical pathology ADR-0044 documented for `wit_world_major_version_mismatch_rejects_future_major`.
- Consequently the macro's `lifecycle_shim_tokens` emits fake `#[export_name = "on-print-start"]` shims for worlds whose `.wit` declares no such export — and the macro's own comment above `skip_lifecycle_shims` already admits "the world declares none (postpass/prepass/finalization)".
- `docs/04_host_scheduler.md:1449` ("call on-print-start on all modules") describes a step the host has never performed.
- `docs/03_wit_and_manifest.md:559` labels the pair `// Lifecycle — optional`. **The component model has no optional exports** — wasmtime's generated `Indices::new` eagerly resolves every export at `instantiate`, which is ADR-0045's central premise and the entire reason the per-stage split exists. This listing is where the "optional export" fiction was first written down, in the initial commit, before a host existed to contradict it. Every downstream artifact in this packet is a faithful implementation of a sentence that was never true.

Folded in (same blast radius, same tests): `pnp_cli` is a separate package, so `cargo test -p slicer-runtime` never rebuilds it, and **three** independent copies of the lookup probe the filesystem for whatever artifact happens to be on disk — `slicer_cache.rs::pnp_cli_bin` (`:112-146`, prefers a stale `target/release` over a fresh `target/debug`), `benches/gate_evidence.rs:48-74` (same release-then-debug fallback at the `for profile in ["release","debug"]` loop), and `slicer-scheduler`'s `dag_cli_integration.rs::bin` (`:15-31`, debug-then-release). None checks mtime.

**This trap has now burned two consecutive sessions, and the second nearly committed a wrong golden.** The first was this ADR's own prior session, which recorded a false baseline. The second was the parallel `object_id` session: its first `BLESS_GOLDEN=1` run blessed `9dda3c89` — **neither the old id nor the correct one** — because the e2e test spawned a `pnp_cli` that `cargo test` never rebuilt, so it blessed the *old code's* output into a golden file. It was caught only because that session checked the uuid against its derivation instead of trusting a green test. The correct value, after `cargo build --bin pnp_cli`, is `da3bd96b` = `uuid5(NS, "20mmbox-LF.stl#0")`.

That is the justification for AC-8/AC-9/AC-N2, and it is not hypothetical: a silent stale spawn does not merely produce a red test that someone investigates — it produces a **green** test whose output is wrong, and a `--bless` flow will happily write that wrong output to disk as the new truth. This packet's own blast radius is measured by exactly these tests, so the gate must be trustworthy before the measurement is believed.

**A note on this packet's own ACs, earned the hard way.** The first draft of this packet shipped `AC-N1` and `AC-N2` as bare `cargo test -p <crate> --test <bin> <name> | rg '^test result'`. Against a tree where the test does not yet exist, libtest filters to nothing, prints `ok. 0 passed; 0 failed; … 196 filtered out`, and exits **0** — so the acceptance criterion **passed by doing nothing**. That is precisely the defect this packet exists to delete: `every_world_has_lifecycle_exports` asserts a table against itself and passes vacuously; a `0 passed` AC asserts nothing against nothing and passes vacuously. Writing one *inside the packet that deletes the other* is not irony, it is the same reflex — a green check mistaken for evidence. Every `cargo test <name>` gate here therefore carries a `| rg -v '0 passed'` guard. **The next author will reach for the unguarded form; this paragraph exists to stop them.**

TASK-146 (`docs/07_implementation_status.md:39`) is **reopened as TASK-146a**: it closed by adding `validate_wit_world`, which ADR-0044 showed compares one hand-written string to another with no artifact to check against, and which ADR-0045 retires outright. Sub-lettering follows the existing `TASK-119a/b/c`, `TASK-120a-d`, `TASK-194a/b` convention. This packet lands **before** the per-stage split so the world-layer export surface shrinks 10 → 8 first, giving packets #2/#3 a smaller, honest surface to split.

No packet is superseded by this one.

## In Scope

1. **WIT.** Delete `export on-print-start` and `export on-print-end` from `crates/slicer-schema/wit/deps/world-layer/world-layer.wit:20-21`. No other `.wit` declares them.
2. **SDK traits.** In `crates/slicer-sdk/src/traits.rs`: rename `on_print_start(config) -> Result<Self>` to `from_config` in all four traits — `LayerModule`, `PrepassModule`, `PostpassModule`, `FinalizationModule`; delete each trait's defaulted `on_print_end`; rewrite the four trait-level doc comments that describe the fictional lifecycle (including `LayerModule`'s two bullets quoting the deleted WIT exports verbatim).
3. **Macro** (`crates/slicer-macros/src/lib.rs`) — cited by construct, not coordinate (see §Context Discipline Notes): remove the `WORLD_LIFECYCLE` import and the `let lifecycle_exports: &[&str] = WORLD_LIFECYCLE.iter()…` lookup, so `wit_exports` collapses to the detected stage export (0 or 1 entries); remove the `lifecycle_count` binding, the `lifecycle_binding_tokens` closure (**through its terminating `});`**), and its `#( #lifecycle_binding_tokens ),*` interpolation in the `exports:` array; remove the `__SLICER_LIFECYCLE_EXPORT_COUNT` const; remove the `lifecycle_shim_tokens` statement and the `skip_lifecycle_shims`/`active_lifecycle_shims` bindings plus their `#( #active_lifecycle_shims )*` interpolation; remove `fn on_print_start`/`fn on_print_end` from `impl Guest for __SlicerLayerComponent` (the only glue block with them); rename the 15 `run_*` module-construction sites from `::on_print_start(&ir_config)` to `::from_config(&ir_config)` (**16 occurrences exist; the 16th lives inside the deleted glue block — see AC-5**).
4. **Macro tests.** `crates/slicer-macros/tests/binding_surface_tdd.rs`, `all_worlds_glue_tdd.rs`, `slicer_module_tdd.rs`, `smoke.rs` — update assertions to the lifecycle-free surface; `design.md` names each affected test by function name. The stageless-impl fixture `LayerLifecycleOnly` now reports an **empty** export list.
5. **Schema** (`crates/slicer-schema/src/lib.rs`): delete the `WORLD_LIFECYCLE_EXPORTS` const, the `lifecycle_exports_for_world` fn, and the test `every_world_has_lifecycle_exports`; delete the `ExportKind::Lifecycle` variant; rewrite the `SlicerModuleSchema.exports` doc whose "lifecycle order then stage" ordering contract collapses; fix the orphaned `SUPPORTED_WIT_WORLDS` doc comment ("Mirrors the world column of [`WORLD_LIFECYCLE_EXPORTS`]" — a rustdoc link that dangles once the table dies, which is this packet's one real `-D warnings` forcing function).
6. **`crates/pnp-cli/src/module_new.rs`**: drop the `lifecycle_exports_for_world` import and its use in `generate_manifest` so `expected_exports` is the single stage export (and drop the manifest comment's `# (lifecycle exports + the stage-specific export…)` line); replace the scaffold template's `fn on_print_start` with `from_config`; delete the generated `on_print_start_succeeds()` test and the packet-local test `lib_rs_has_on_print_start_lifecycle`.
7. **Mechanical sweep.** `on_print_start` → `from_config` and `on_print_end` deleted across: all 20 `modules/core-modules/*/src/lib.rs` and their `tests/`; the 9 `crates/slicer-wasm-host/test-guests/*/src/lib.rs` that define it; the 4 `crates/slicer-sdk/tests/*_module_tdd.rs`; and the ~30 `crates/slicer-runtime/tests/**` files that construct modules in-process. 535 occurrences across 110 files (`rg -l 'on_print_start|on_print_end' --type rust`, excluding `target/`). Delete `arachne-perimeters`' `on_print_end` body (`src/lib.rs:419` — the only one in the tree).
8. **CLI freshness — the gated entry point.** `xtask/src/test.rs` Step 1 (`:129-137`, where `build_guests::check_command` runs) additionally checks and, on staleness, rebuilds `pnp_cli`.
9. **CLI freshness — all three spawn sites.** Each asserts freshness itself and panics loudly, so plain `cargo test -p slicer-runtime` / `cargo bench` — the narrow invocations `CLAUDE.md` recommends, and the ones that burned two sessions — fail rather than spawning a stale binary. The release-preferring fallback is removed from every copy. Freshness logic mirrors `xtask/src/build_guests.rs::is_stale` and `compute_shared_mtime`.
   - `crates/slicer-runtime/tests/common/slicer_cache.rs::pnp_cli_bin` (`:112-146`).
   - `crates/slicer-runtime/benches/gate_evidence.rs::pnp_cli_bin` (`:48-74`) — the sole producer of DEV-026's `~438ms` 50-layer governance evidence.
   - `crates/slicer-scheduler/tests/integration/dag_cli_integration.rs::bin` (`:15-31`) — also correct its panic message: `"Run `cargo build --workspace` first"` builds the binary once and does nothing to keep it fresh afterwards, which is precisely how the trap survives. It must name `cargo build -p pnp-cli` and state the staleness cause.
   - **The triplication is deliberate and stays.** The shared-helper extraction is explicitly deferred to its own packet — see Out of Scope.
10. **Regression test.** New `crates/slicer-runtime/tests/integration/pnp_cli_freshness_tdd.rs` (mounted in `tests/integration/main.rs`) proving a stale/absent binary produces a loud failure.
11. **Guard test.** New `no_lifecycle_exports_anywhere` in `crates/slicer-runtime/tests/contract/wit_drift_detection_tdd.rs`.
12. **Docs.** `docs/03_wit_and_manifest.md:559-561` (the `// Lifecycle — optional` comment and the two exports it labels); `docs/04_host_scheduler.md:1449`; `docs/05_module_sdk.md:168`, `:176`, `:183-193`, `:298`, `:375`, `:728-733`, `:848`, `:950`, `:964`, `:1147`; `docs/07_implementation_status.md`.

## Out of Scope

- **Per-stage WIT packages, `bindgen!` fan-out, dispatch-by-stage, fatal-on-miss load.** Packets #2 (`163_per-stage-wit-packages-pilot`) and #3 (`164_per-stage-wit-packages-bulk`).
- **`SUPPORTED_WIT_WORLDS`, `wit-world` manifest key, `validate_wit_world`.** Retire in packet #3. This packet only repairs `SUPPORTED_WIT_WORLDS`' orphaned doc comment.
- **Restructuring `ExportBinding` / `SlicerModuleSchema.exports` into package+interface form.** Packet #2 does this; see `design.md` §"Rejected alternatives" for why `ExportKind` is not collapsed here.
- **`docs/03_wit_and_manifest.md`'s WIT listing as a whole** — its restructure to per-stage packages is packet #3's. This packet deletes only `:559-561`, the three lines its own WIT edit falsifies. The two edits are disjoint: #3 rewrites the listing's shape; this deletes a lifecycle stanza that will not exist in any shape.
- **`CONTEXT.md`** ("Module tier", "Stage contract"). Packet #3, per the plan's §"Status since approval".
- **Extracting the three duplicated `pnp_cli` lookups into one shared helper.** Deferred to its own packet by explicit decision. It needs an ADR, not a refactor: the options are a dev-dep cycle onto a `pnp-cli` lib target, or a new host-side test-support crate — and neither is covered by existing precedent. ADR-0004 governs only *guest-side* test support in `slicer-sdk`, and the `slicer-test` crate that might have hosted this was deleted by packet 78. This packet therefore fixes all three sites **in place, identically**; see `design.md` for why a reviewer must not DRY them.
- **Re-adding a real per-print lifecycle.** Deleting the hook forecloses a layer module holding cheap private state across layers. It cannot do so today (rebuilt per call), so nothing that currently works is lost; re-adding it later needs a new contract, not this one.
- **Re-fixing `object_id` / `crates/slicer-model-io/src/loader.rs::path_object_id`.** The parallel session **landed at `ff21378e`**: `object_id` is now `basename + index` (`uuid5(NS, "20mmbox-LF.stl#0")` = `da3bd96b`), and the previously-red set is **green from HEAD**. This packet inherits that as a committed passing baseline it must not regress; it does not touch that code. See `design.md` §"Risks and Tradeoffs".

## Authoritative Docs

- `docs/specs/adr-0045-per-stage-wit-packages-plan.md` (long; ranged reads only) - direct read of §"The lifecycle finding", §"Grounding corrections", §"Packet Queue".
- `docs/adr/0045-per-stage-versioned-interfaces-over-monolithic-tier-worlds.md` (accepted) - ranged read of §"The finding"; the split itself is out of scope.
- `docs/adr/0044-wit-world-version-is-not-an-identity-token.md` (accepted) - delegate a SUMMARY; supplies the vacuous-guard-test pathology.
- `docs/05_module_sdk.md` (>1100 lines) - **delegate or read ranges only**: `:165-200`, `:290-300`, `:370-380`, `:726-736`, `:840-970`, `:1140-1150`.
- `docs/04_host_scheduler.md` (>1400 lines) - **ranged read only**: `:1444-1455`.
- `docs/07_implementation_status.md` (>300 lines) - **always delegate**; `:37-39` is the governing slice.
- `CLAUDE.md` §"Guest WASM Staleness", §"Test Discipline", §"WIT/Type Changes Checklist" - direct read.

## Acceptance Summary

Criteria are stated in `packet.spec.md`; referenced here by ID.

- Positive: `AC-1` (WIT: 8 exports, 0 lifecycle) through `AC-10` (docs de-fictionalized, incl. `docs/03:559-561`), plus `AC-8b`. `AC-6` is the only AC that requires rebuilt guest artifacts and is the empirical proof the world-layer surface shrank 10 → 8. `AC-4`/`AC-5` are the macro's metadata and glue surfaces respectively — both must be asserted; the macro uses `WORLD_LIFECYCLE` only for metadata (`:148`) and never for the `impl Guest` glue (which is hardcoded), so a green `AC-4` does not imply a green `AC-5`. `AC-8` covers all three spawn sites; `AC-8b` exists separately because the freshness test's own `mod` registration is the difference between a real test and a 0-tests-ran false pass.
- Negative: `AC-N1` (no `on-print-*` / `on_print_*` survives in canonical WIT or any workspace `.rs`, enforced by a walking guard test rather than a self-referential table), `AC-N2` (a stale or absent `pnp_cli` yields a loud `Some(reason)` / panic, and a fresh one yields `None`), `AC-N3` (the green parity set stays green — the behavior-neutrality proof).
- Cross-packet impact: packet #2 consumes an 8-export `world-layer`, a `SlicerModuleSchema.exports` of length ≤ 1 carrying only `ExportKind::Stage`, and `from_config` as the sole SDK constructor. Packet #2 then restructures `ExportBinding` into package+interface form and may collapse or replace `ExportKind` at that point.

## Verification Commands

This is the authoritative full matrix; `packet.spec.md` lists only 3 gate commands.

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo xtask build-guests --check` | MUST be clean before believing any guest/component/host-integration failure | FACT: clean or `STALE:` list |
| `cargo xtask build-guests` | WIT + macro + sdk + schema each invalidate every guest; rebuild all 20 + test guests | FACT pass/fail; tail 20 lines on failure |
| `cargo check --workspace --all-targets` | The rename touches 110 files; the type system is the sweep's completeness proof | FACT pass/fail; SNIPPETS ≤20 lines on failure |
| `cargo clippy --workspace --all-targets -- -D warnings` | Required before commit; catches any newly-unused import/variant | FACT pass/fail; SNIPPETS ≤20 lines |
| `cargo test -p slicer-schema 2>&1 \| rg '^test result'` | AC-3: table + vacuous guard gone, remaining schema tests green | FACT pass/fail |
| `cargo test -p slicer-macros --test binding_surface_tdd 2>&1 \| rg '^test result'` | AC-4: metadata surface is stage-only | FACT pass/fail |
| `cargo test -p slicer-macros --test all_worlds_glue_tdd 2>&1 \| rg '^test result'` | AC-5 support: glue no longer names lifecycle arms | FACT pass/fail |
| `cargo test -p slicer-sdk 2>&1 \| rg '^test result'` | Trait rename compiles and the four SDK trait test files pass | FACT pass/fail |
| `cargo test -p pnp-cli --lib module_new 2>&1 \| rg '^test result'` | AC-7: scaffold and its tests | FACT pass/fail |
| `cargo test -p slicer-runtime --test contract no_lifecycle_exports_anywhere 2>&1 \| rg '^test result'` | AC-N1: the walking guard | FACT pass/fail |
| `cargo test -p slicer-runtime --test contract wit_drift_detection_tdd 2>&1 \| rg '^test result'` | The whole drift-guard file still passes after the WIT edit | FACT pass/fail |
| `cargo test -p slicer-runtime --test integration pnp_cli_freshness 2>&1 \| rg '^test result'` | AC-8b/AC-N2: the freshness gate fires. **A `0 passed` result is a FAIL, not a pass** — it means the `mod` registration is missing | FACT pass/fail + the count |
| `cargo test -p slicer-scheduler --test integration dag_cli 2>&1 \| rg '^test result'` | Step 10b: spawn site 3 still works after its freshness assert | FACT pass/fail |
| `cargo test -p slicer-runtime --test integration -- perimeter_parity 2>&1 \| rg '^test result'` | AC-N3: must stay `12 passed; 0 failed` — green-before/green-after | FACT pass/fail |
| `cargo test -p slicer-runtime --test e2e -- legacy_zero_matches_golden 2>&1 \| rg '^test result'` | AC-N3: must stay `1 passed; 0 failed`; this is the test the stale-binary trap corrupted | FACT pass/fail |
| `for w in modules/core-modules/*/*.wasm; do wasm-tools component wit "$w"; done \| grep -c 'on-print'` | AC-6: decoded guest exports carry no lifecycle. Expect `0` | FACT: the integer |
| `cargo xtask test --summary --workspace` | **Packet close only.** Dispatch to a sub-agent with a `FACT pass/fail` return; never absorb the output | FACT pass/fail + failing-test names |

**Mandatory `0 passed` guard on every name-filtered row above.** `cargo test … <name>` and `cargo test … -- <name>` filter; when the filter matches nothing — test absent, renamed, or unregistered in its bucket aggregator — libtest prints `ok. 0 passed; … N filtered out` and exits **0**, so the gate passes by doing nothing. Every name-filtered command in `packet.spec.md` therefore ends `| rg -v '0 passed' || echo 'FAIL: 0 tests ran'`, and the rows above are shown in their bare form only for readability — **run them in the guarded form given in the ACs.** Unfiltered runs (`cargo test -p slicer-schema`, `--test binding_surface_tdd`) need no guard: they run a whole crate or binary and cannot silently match nothing. **A reviewer must treat any unguarded name-filtered gate as unproven.** See §Problem Statement — this packet shipped that exact defect in draft 1.

## Step Completion Expectations

- **The rename is one compile unit.** Steps 3-7 individually leave the workspace non-compiling by design; their exits are grep-based counts, not `cargo check`. The first `cargo check --workspace --all-targets` gate is Step 8. Do not "fix" an intermediate step by re-adding `on_print_start`.
- **Guest rebuild ordering.** `cargo xtask build-guests` (Step 9) must run *after* the WIT, macro, SDK, schema, and module-source edits have all landed; every one of those paths invalidates every guest. `AC-6` is meaningless before it.
- **`cargo xtask build-guests --check` must be clean before any guest/component/host-integration/module-dispatch failure is attributed to this packet's code**, to "flaky tests", or to "unrelated infrastructure".
- **Known-GREEN baseline, committed (status changed — do not use stale guidance).** The `object_id` fix **landed at `ff21378e`**, so the parity set is green **from HEAD**, not from one session's uncommitted working tree: `cargo test -p slicer-runtime --test integration -- perimeter_parity` → `12 passed; 0 failed; 11 ignored`, and `cargo test -p slicer-runtime --test e2e -- legacy_zero_matches_golden` → `1 passed; 0 failed`, both re-verified on a clean tree at `b7f17f75` (0 stale guests). Earlier drafts of this packet (and the plan's §"Out of scope" item 3) described these as known-red requiring red-before/red-after; **that is obsolete.** They must be **green before and green after**. A regression in any of them is **caused by this packet** and is a gate failure — not an inherited fault, not flakiness. Behavior-neutrality is now proven by staying green, which is a strictly stronger signal than matching a red set.
- **The set is 8 tests, not 7.** `deliberate_broken_fixture_file_is_detected` was masked in the old red set: `compare_perimeter_ir` stops at the first mismatch, and `object_id` mismatched first, so the fixture-detection test never reached its own assertion. It is now unmasked and green — meaning the parity harness's own negative control works again. Treat a failure there as a signal the harness stopped detecting corruption, not as a fixture problem.

## Citation Policy

**Governing rule: `CLAUDE.md` §"In-Tree Citation Style (MUST follow)"** — cite in-tree code by symbol name; a line number is a navigation hint, never the identifier. That section is authoritative for `.ralph/specs/**`, ADRs, `docs/`, and code comments. This section does not restate or fork it; it records the packet-local evidence CLAUDE.md cites, because an implementer working this packet will be tempted by exactly these coordinates.

An independent review of this packet's first draft resolved every *symbol* it named cleanly, but **11+ of ~34 line ranges did not** — clustering at range ends and in files that were dirty in the working tree when they were read. Three were actively dangerous:

- `lifecycle_binding_tokens` was pinned as `:159-170`; the closure's terminating `});` is on the **next** line. Deleting the literal range leaves a dangling `});` and a **non-compiling file**.
- `gate_evidence.rs::pnp_cli_bin` was pinned `:44-60`, which truncates mid-body and **excludes the very release-fallback loop the same bullet ordered deleted**.
- `typed_schema_kinds_distinguish_lifecycle_from_stage` was pinned `:545-570` in a **560-line file** — 10 lines past EOF.

These pins rotted **within a single session** — they were captured from files that were dirty in the working tree at read time (the `object_id` fix, since committed as `ff21378e`, was uncommitted then). Coordinates taken from a dirty buffer are pinned to one worktree and are unverifiable for anyone else; symbol names are not.

Packet-local application of the CLAUDE.md rule:
- Name the **fn / const / test / binding / closure**, and describe it precisely enough to `rg`. Quote a distinctive line rather than citing coordinates.
- A line number may accompany a name as a navigation hint **only** where it aids a large file, and only if re-verified against disk at authoring time. **A pin that is off by one is worse than no pin** — the three findings above are the proof, and an off-by-one pin is indistinguishable from a correct one until it silently produces broken code.
- **Never delete by coordinate range.** Delete by construct, following the syntax to its true terminator.
- This applies to `design.md`'s change surface above, which is deliberately written as constructs — implement from the names, not from any number.

## Context Discipline Notes

- `docs/05_module_sdk.md` (>1100 lines) and `docs/04_host_scheduler.md` (>1400 lines) must never be read in full — the listed ranges are sufficient and were verified. The line numbers cited for `docs/05` in earlier plan drafts (`:53`, `:61-62`, `:238-243`) are **wrong**; those windows are §"Guest Build Invariants" and §`run_infill_postprocess`. Use the ranges in this file.
- `crates/slicer-macros/src/lib.rs` is >2800 lines. Never read it whole; every edit site is enumerated in `design.md` with a line number. Open ±40-line windows.
- The Step 7 mechanical sweep spans 110 files / 535 occurrences. Do **not** read them; drive it with a scripted rename and let `cargo check --workspace --all-targets` prove completeness. Any sub-agent asked about the sweep returns `FACT` (a count), never file contents.
- Never load `target/`, `Cargo.lock`, or any `.wasm`. Guest export surfaces are inspected only via `wasm-tools component wit <path> | grep`, whose output is ≤ 25 lines.
- `cargo xtask test --summary --workspace` (packet close) must be dispatched to a sub-agent returning `FACT pass/fail`.
