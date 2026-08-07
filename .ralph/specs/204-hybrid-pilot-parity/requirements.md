# Requirements: 204-hybrid-pilot-parity

## Packet Metadata

- Grouped task IDs: `ADR-0056`, `ADR-0057` (no `docs/07_implementation_status.md` TASK rows exist for the distribution/editions workstream — verified 2026-08-07 by the plan's §"Backlog anchoring [FWD]"; the plan's `[FWD]` to add a "Distribution & Editions" workstream is unresolved and this packet does not create one)
- Backlog source: `docs/specs/multi-edition-distribution-plan.md` (queue row 5)
- Packet status: `draft`
- Aggregate context cost: `M`

## Problem Statement

ADR-0056 decides that a core module crate becomes single-source dual-target and that a module ships integrated **only** behind a contract test running identical inputs through both dispatch paths. ADR-0057 seeds the Hybrid edition's integrated set with the same three modules and requires that set to be a **dist-configuration list, not a hardcoded constant**. The measured record behind that seed lives in **ADR-0056 §Context**, not ADR-0057: `classic-perimeters` measured at ≈90% of per-layer module CPU with roughly 39% of it polygon ops (attributed there to ADR-0055), `arachne-perimeters`' heavy half already running natively through the ADR-0033 host-service bridge, and `support-planner`'s prepass time ≈98% geometry (attributed there to ADR-0049). These are quoted figures from that ADR, not measurements taken in this packet — AC-8 requires the implementer to re-measure before finalizing the list.

Packets 201 and 202 build the machinery but deliberately land it inert: 201's registry defaults to empty and 202's `native_entries()` returns an empty `Vec` "until 204". Nothing is integrated, no parity gate exists, and packet 205's `cargo xtask dist` edition work has no config list to read. This packet is the single coherent slice that closes all three gaps at once, because the parity gate and the Hybrid-set finalization are the *same evidence*: a module cannot enter the config list until its parity test is green, and the profiling that finalizes the list runs against exactly the build the parity tests certify.

The pilot crates are already native rlibs — all three are workspace members with a default lib target, and `crates/slicer-runtime/Cargo.toml` already dev-depends on `classic-perimeters` and `arachne-perimeters` to drive them natively (see its `[dev-dependencies.classic-perimeters]` / `[dev-dependencies.arachne-perimeters]` comments). "Dual-target" is therefore not a crate-type change; it is a dependency-and-registration change plus the feature-unification reasoning `arachne-perimeters` needs for `host-algos`.

No prior packet is reopened or superseded.

## In Scope

- `crates/slicer-integrated-modules/Cargo.toml`: add three optional path dependencies on `modules/core-modules/{classic-perimeters,arachne-perimeters,support-planner}` and three `[features]` entries named exactly after those directories, per packet 201's per-module-feature convention.
- `crates/slicer-integrated-modules/src/lib.rs`: extend `integrated_registrations()` with one `IntegratedModuleRegistration` per enabled pilot feature (`manifest_toml` via `include_str!` of the module's own `<name>.toml`, `origin_label` = `integrated://<module-dir-name>`), and extend `native_entries()` with one `(ModuleId, NativeStageEntry)` per enabled pilot feature sourced from the module type's macro-emitted `__slicer_native_entry()`.
- `crates/slicer-integrated-modules/`: unit tests backing AC-1 and AC-2.
- New shared parity comparator `crates/slicer-runtime/tests/common/parity_invariants.rs`, registered in `crates/slicer-runtime/tests/common/mod.rs`, exposing `assert_parity_structural` (layer family, over `LayerStageCommit`) and `assert_prepass_parity_structural` (prepass family, over `PrepassStageOutput::SupportPlan`), plus a coordinate tolerance of `1e-3` mm. Byte-equality of floats is never asserted. The invariant set is:
  - **From ADR-0042 §Decision's list** (its list is explicitly non-exhaustive): closure within tolerance, loop count and nesting, bead-count sequence, **transitions-present** (a bead-count change between adjacent loop positions must appear on both paths or neither — omitting it would let a native path that flattened every transition still pass the bead-count-sequence check on uniform-width geometry, which is exactly `propagation_fills_gap_from_central_neighbor`'s failure mode), no self-intersection, coverage ratio, and no bead wider than `2.0 ×` `optimal_width`.
  - **Two deliberate wording departures from ADR-0042, neither a conflict.** (i) ADR-0042 §Decision says "coverage ratio *vs. a known-correct reference*"; this packet says bare "coverage ratio" because there is **no known-correct reference here** — it compares two dispatch paths of the same implementation, so the ratio is computed symmetrically between them (the shape `symmetric_coverage_ratio` in `crates/slicer-runtime/tests/arachne_structural_invariants.rs` already uses). (ii) ADR-0042 says "no bead wider than *~2×* `optimal_width`"; this packet pins an exact `2.0 ×`, which is **stricter** than an approximate bound, not looser. Recorded so a later reader does not mistake either for drift.
  - **Additions beyond ADR-0042's list** — per-loop point count and `ExtrusionRole` sequence. These are *tightening*, not conflict: ADR-0042 does not name them, but it also does not close its list, and both are unit-independent structural properties of the kind it mandates. They are cheap and they catch the dispatch-path divergence class this packet is actually gating (a marshalling bug that drops or re-roles a path), which ADR-0042's arachne-shaped list was not written for.
- The prepass comparator's invariant set is the shape-specific analogue: `entries` count, the full `(global_layer_index, object_id, region_id)` key set, per-entry `branch_segments` count, per-segment `points` count and `role`, and per-point `(x, y, z, width)` within tolerance.
- Three new contract tests under `crates/slicer-runtime/tests/contract/`, each registered as a `mod` in `crates/slicer-runtime/tests/contract/main.rs`: `integrated_parity_classic_perimeters_tdd.rs`, `integrated_parity_arachne_perimeters_tdd.rs`, `integrated_parity_support_planner_tdd.rs`. Each builds one `WasmRuntimeDispatcher` and two `CompiledModuleLive` values (native vs wasm) over an identical `*StageInput`, following the pattern packet 202 demonstrates in `native_dispatch_parity_seam_tdd.rs`.
- Comparator self-tests for **both** families — AC-N2/AC-N3 for the layer family and AC-N6 for the prepass family — proving the invariants are tolerance-based and non-vacuous, mirroring ADR-0042's D5 sanity-discriminator requirement. Both self-tests live in `crates/slicer-runtime/tests/contract/parity_invariants_selftest_tdd.rs`.
- One new integration test `crates/slicer-runtime/tests/integration/hybrid_pilot_external_override_tdd.rs` (registered in `crates/slicer-runtime/tests/integration/main.rs`) backing AC-N4.
- New committed dist-config list `dist/editions.toml` with `schema_version`, three `[edition.<name>]` tables, snake_case keys `integrate_all` (bool) and `integrated_modules` (list of module directory names), and a leading `# evidence:` comment block recording the ADR-0055 profiling that finalized the Hybrid set.
- New `xtask/src/editions.rs` with `EDITIONS_CONFIG_PATH`, `EditionSpec { integrate_all: bool, integrated_modules: Vec<String> }`, and `load_editions(ws_root) -> Result<BTreeMap<String, EditionSpec>, String>` which validates every listed name against `build_guests::discover_guests`' core-tree stems; declared as `mod editions;` in `xtask/src/main.rs`. Unit tests live in `#[cfg(test)] mod tests` inside `editions.rs` (xtask has no lib target, so an integration test could not reach it).
- ADR-0055-methodology profiling run producing the AC-8 evidence block.
- `docs/01_system_architecture.md` §"Producing the tier-4 layout: `cargo xtask dist`" paragraph on `dist/editions.toml`.
- A `docs/DEVIATION_LOG.md` row **only if** a parity test reveals residual native-vs-wasm divergence that the tolerance-based comparator accepts but that is worth recording. The implementer MUST re-derive the next free ID at write time with `rg -o '^\| DEV-[0-9]{3}' docs/DEVIATION_LOG.md | sort -u | tail -1` and take the successor. No ID is pinned by this packet — the highest live ID is a ledger fact that rots (`CLAUDE.md` §"Ledger Facts Must Be Re-derived").

## Out of Scope

- `modules/core-modules/classic-perimeters/src/lib.rs`, `.../arachne-perimeters/src/lib.rs`, `.../support-planner/src/lib.rs` — **no edits at all**. Packet 200 owns the classic-perimeters geometry call sites; this packet's only pilot-crate edits are to `Cargo.toml` files, and only if a `[lib]` or feature entry proves necessary (see `design.md`).
- Dispatch routing, macro native-adapter emission, marshalling (`crates/slicer-macros/`, `crates/slicer-sdk/src/native.rs`, `crates/slicer-wasm-host/src/marshal/native.rs`, `crates/slicer-wasm-host/src/dispatch.rs`) — packet 202.
- Manifest ingestion, `ModuleProvenance`, tier-5 search ordering, the shadow diagnostic — packet 201.
- `--no-integrated-modules` and `pnp_cli module` provenance output — packet 203.
- `cargo xtask dist`'s edition dimension, the staging/disjointness enforcement, and CI edition artifacts — packet 205. This packet only creates and validates the config list 205 consumes; `xtask/src/dist.rs` is not edited.
- Integrating any of the other 18 core modules; the Integrated edition's full set is expressed as `integrate_all = true`, not enumerated here.
- Per-module internal parallelism (ADR-0056 Decision item 5 defers it), wasm-less builds (ADR-0056 Decision item 6), and platform build matrices (ADR-0057 phase 4).
- Re-recording or weakening any existing arachne or perimeter fixture. Canonical parity correctness outranks a green board (`CLAUDE.md` §Test Discipline).

## Authoritative Docs

- `docs/adr/0056-integrated-modules-native-dispatch.md` — 122 lines; direct read. Decision item 4 (parity gate, byte-equality explicitly not the gate) and Decision item 5 (single-threaded module logic) are the normative contract.
- `docs/adr/0057-three-editions-and-integrated-tier.md` — 55 lines; direct read. The edition table and the "dist-configuration list, not a hardcoded constant" clause.
- `docs/adr/0042-arachne-parity-structural-invariants-over-fixtures.md` — long; direct read of §Decision only, delegate anything else. Supplies the invariant class and the D5 non-vacuity discriminator.
- `docs/adr/0055-fuel-based-module-profiling.md` — 127 lines; direct read. Fuel primary, wall-clock secondary, profiling-off absolute timings, explicit run-to-run spread.
- `docs/adr/0033-host-service-bridge-for-host-only-algorithms.md` — short; direct read. The `cfg`-split wrapper layer that lets `arachne-perimeters` run natively.
- `docs/01_system_architecture.md` — large; ranged read of §"Producing the tier-4 layout: `cargo xtask dist`" only. Delegate the rest.
- `docs/08_coordinate_system.md` — delegate; consult only if a tolerance constant needs the mm↔unit factor.
- `.ralph/specs/201-integrated-module-registry-tier5/packet.spec.md`, `.ralph/specs/202-native-adapter-and-dispatch/packet.spec.md` — read-only, for FORWARD-DEP shapes. Never modify.

## Acceptance Summary

Reference, never copy, criteria from `packet.spec.md`.

- Positive: `AC-1` through `AC-9`.
  - `AC-3`/`AC-4`/`AC-5` are the ADR-0056 Decision item 4 parity gate, one per pilot module; each must construct its two `CompiledModuleLive` values from the **same** fixture bytes so the only variable is the dispatch path.
  - `AC-5` keys on the full `(global_layer_index, object_id, region_id)` triple. `global_layer_index` is not optional: `SupportPlanIR.entries`' doc comment states multiple entries may share `(layer, object)`, so a two-field key collapses layers and a dropped layer would pass silently.
  - `AC-8` requires **both** ADR-0055 signals — guest fuel (primary) and profiling-off wall-clock (secondary) — plus an explicit run-to-run spread. The spread requirement is stricter than ADR-0055 and is retained deliberately (`CLAUDE.md` §"No Unverified Metrics"). An unmeasured or unlabelled figure fails the AC even if the grep passes.
  - `AC-9` is the ADR-0056 Decision item 5 invariant expressed as a cheap static negative check; it is a floor, not a proof of single-threadedness.
- Negative: `AC-N1` (unknown module name rejected by the config reader), `AC-N2` (layer comparator accepts ULP-scale drift), `AC-N3` (layer comparator rejects dropped loops and point-count changes), `AC-N4` (external override still wins), `AC-N6` (prepass comparator rejects a dropped entry, a shifted `global_layer_index`, a dropped `branch_segment`, and a dropped point), `AC-N5` (doc greps).
- Cross-packet impact: `dist/editions.toml` + `xtask::editions::load_editions` are packet 205's sole input for the edition dimension. `integrated_registrations()` and `native_entries()` stop returning empty, which activates packet 201's tier-5 path and packet 202's native branch for the first time in a shipped build — every existing test that loads modules from `modules/core-modules/` now has a tier-5 duplicate available, so packet 201's first-root-wins dedup and shadow diagnostic get their first real exercise here (AC-N4).

## Verification Commands

This is the authoritative full matrix; `packet.spec.md` lists only 2–3 gate commands.

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo check --workspace --all-targets` | Whole tree, incl. test/bench targets, compiles | FACT pass/fail |
| `cargo clippy --workspace --all-targets -- -D warnings` | Lint gate | FACT pass/fail; SNIPPETS ≤20 lines on failure |
| `cargo xtask build-guests --check` | Both artifacts of every pilot module are fresh; parity tests are meaningless against a stale wasm twin | FACT clean / `STALE:` list |
| `cargo test -p slicer-integrated-modules --features classic-perimeters,arachne-perimeters,support-planner hybrid_pilot_registrations_are_exactly_three` | AC-1 | FACT pass/fail |
| `cargo test -p slicer-integrated-modules --features classic-perimeters,arachne-perimeters,support-planner hybrid_pilot_native_entry_families_match_stage_ids` | AC-2 | FACT pass/fail |
| `mkdir -p target && cargo test -p slicer-runtime --test contract -- integrated_parity_classic_perimeters 2>&1 \| tee target/test-output.log && rg -q "^test result: ok" target/test-output.log` | AC-3 | FACT pass/fail; grep the log for failures |
| `mkdir -p target && cargo test -p slicer-runtime --test contract -- integrated_parity_arachne_perimeters 2>&1 \| tee target/test-output.log && rg -q "^test result: ok" target/test-output.log` | AC-4 | FACT pass/fail |
| `mkdir -p target && cargo test -p slicer-runtime --test contract -- integrated_parity_support_planner 2>&1 \| tee target/test-output.log && rg -q "^test result: ok" target/test-output.log` | AC-5 | FACT pass/fail |
| `cargo test -p slicer-runtime --test contract -- parity_comparator_accepts_ulp_perturbation` | AC-N2 | FACT pass/fail |
| `cargo test -p slicer-runtime --test contract -- parity_comparator_rejects_dropped_loop` | AC-N3 | FACT pass/fail |
| `cargo test -p slicer-runtime --test contract -- parity_comparator_rejects_dropped_support_entry` | AC-N6 | FACT pass/fail |
| `cargo test -p slicer-runtime --test integration hybrid_pilot_external_override_forces_wasm` | AC-N4 | FACT pass/fail |
| `cargo test -p xtask editions_config_declares_three_editions` | AC-7 | FACT pass/fail |
| `cargo test -p xtask editions_config_rejects_unknown_module_name` | AC-N1 | FACT pass/fail |
| `sh -c 'rg -q "^# *evidence:" dist/editions.toml && rg -qi "fuel" dist/editions.toml && rg -qi "wall-clock" dist/editions.toml && rg -q "extruder_idler\|benchy" dist/editions.toml && rg -q "run-to-run spread" dist/editions.toml && echo PASS'` | AC-8 | FACT PASS or silence |
| `sh -c 'for m in classic-perimeters arachne-perimeters support-planner; do rg -q "^(rayon\|\[dependencies\.rayon\]\|\[target\..*dependencies\.rayon\])" modules/core-modules/$m/Cargo.toml && exit 1; rg -q "par_iter\|par_bridge\|par_chunks\|rayon::" modules/core-modules/$m/src/ && exit 1; done; echo PASS'` | AC-9 | FACT PASS or silence |
| `sh -c 'rg -q "dist/editions.toml" docs/01_system_architecture.md && rg -q "integrated_modules" docs/01_system_architecture.md && echo PASS'` | AC-N5 | FACT PASS or silence |

`cargo test --workspace` is **not** part of this matrix. Packet closure uses the targeted commands above plus the two workspace static gates.

## Step Completion Expectations

- **Ordering lock.** The comparator (`parity_invariants.rs`) lands and passes its own non-vacuity self-tests for **both** families — AC-N2/AC-N3 for the layer comparator and AC-N6 for the prepass comparator — **before** any of the three parity tests is written. The prepass half is not optional: AC-5 rests on `assert_prepass_parity_structural`, so shipping it unproven would leave the support-planner gate certified by an unverified instrument. A comparator authored alongside its first subject will be shaped to whatever that subject produced — which is exactly the self-captured-baseline failure ADR-0042 rejects.
- **One module at a time.** Each pilot module is registered, parity-tested, and only then added to `dist/editions.toml`'s `hybrid.integrated_modules`. A module whose parity test is red is removed from the list and gets a `design.md` `[FWD]` entry — it is never added with a weakened assertion.
- **Freshness precedes every parity run.** `cargo xtask build-guests --check` must be clean immediately before any parity-test execution, because the pilot-crate `Cargo.toml` edits invalidate their wasm twins and a stale twin makes a parity test compare the native path against yesterday's guest.
- **Shared scratch:** `target/test-output.log` is overwritten by every teed run. Capture findings before launching the next run.
- Profiling (AC-8) runs last, on the final registered set, in `--release`.

## Context Discipline Notes

- `modules/core-modules/classic-perimeters/src/lib.rs` (1466 lines), `arachne-perimeters/src/lib.rs` (1205 lines), and `support-planner/src/lib.rs` (2058 lines) are **out of bounds for editing and should not be read in full**. The only facts needed from them — the `#[slicer_module]`-annotated type names and their SDK traits — must come from a `LOCATIONS` dispatch.
- `docs/adr/0042-...md` is long. Read §Decision directly; delegate anything else.
- `docs/DEVIATION_LOG.md` is large. Never read it; grep it (`rg -o '^\| DEV-[0-9]{3}'`).
- `crates/slicer-runtime/tests/common/perimeter_harness.rs` and `crates/slicer-runtime/tests/common/dispatch_fixture.rs` are existing helpers; request their public API by `LOCATIONS` (`rg -n '^pub (fn|struct|enum)'`) rather than reading them.
- Packet 202's `native_dispatch_parity_seam_tdd.rs` will not exist until 202 is implemented. Its shape must be read from `.ralph/specs/202-native-adapter-and-dispatch/` at implementation time, not assumed.
