# Implementation Plan: 204-hybrid-pilot-parity

## Execution Rules

- Work one atomic step at a time; map every step to grouped task IDs.
- Use TDD, then implementation, then the narrowest falsifying validation.
- Every field below is a context-budget contract and must be filled independently; never write "see Step 1".

## Steps

### Step 1: Reconcile the 201/202 FORWARD-DEP surface against the implemented tree

- Task IDs: `ADR-0056`
- Objective: replace this packet's quoted FORWARD-DEP shapes with the shapes 201 and 202 actually shipped, before any code is written against them.
- Precondition: `docs/spec_packets/201-integrated-module-registry-tier5/packet.spec.md` and `docs/spec_packets/202-native-adapter-and-dispatch/packet.spec.md` both read `status: implemented`; `crates/slicer-integrated-modules/` exists.
- Postcondition: a written inventory of exact signatures for `IntegratedModuleRegistration` (field names/types), `integrated_registrations()`, `native_entries()`, `NativeStageEntry` variants, `__slicer_native_entry()`, `CompiledModuleLive::with_native_entry`, and the per-module feature-naming convention — plus the file path and shape of `native_dispatch_parity_seam_tdd.rs`.
- Files allowed to read, with ranges when over 300 lines:
  - `docs/spec_packets/201-integrated-module-registry-tier5/packet.spec.md` — whole file (75 lines)
  - `docs/spec_packets/202-native-adapter-and-dispatch/packet.spec.md` — whole file (75 lines)
  - `crates/slicer-integrated-modules/src/lib.rs` — whole file (expected small)
- Files allowed to edit (at most 3):
  - none (read-only discovery step)
- Files explicitly out of bounds:
  - `docs/spec_packets/**` — read-only; never modified
  - `crates/slicer-sdk/src/native.rs`, `crates/slicer-macros/**` — packet 202's surface
- Expected sub-agent dispatches:
  - Question: exact declarations of `IntegratedModuleRegistration`, `integrated_registrations`, `native_entries`, `NativeStageEntry`; scope: `crates/slicer-integrated-modules/src/`, `crates/slicer-sdk/src/native.rs`; return: `SNIPPETS` (≤3, ≤30 lines each)
  - Question: how does `native_dispatch_parity_seam_tdd.rs` build its `WasmRuntimeDispatcher` and its two `CompiledModuleLive` values?; scope: `crates/slicer-runtime/tests/contract/native_dispatch_parity_seam_tdd.rs`; return: `SNIPPETS` (1, ≤30 lines)
- Context cost: `S`
- Authoritative docs:
  - `docs/adr/0056-integrated-modules-native-dispatch.md` — Decision items 1–5, direct read (122 lines)
- OrcaSlicer refs:
  - none — this packet has no canonical C++ counterpart
- Verification:
  - `sh -c 'rg -q "^status: implemented" docs/spec_packets/201-integrated-module-registry-tier5/packet.spec.md && rg -q "^status: implemented" docs/spec_packets/202-native-adapter-and-dispatch/packet.spec.md && rg -q "fn native_entries" crates/slicer-integrated-modules/src/lib.rs && echo PASS'` — FACT pass/fail
- Exit condition: every FORWARD-DEP symbol named in `design.md` §Code Change Surface resolves in the tree with the assumed name and shape, or the divergence is written down and the affected step is re-scoped before proceeding. If either dependency is still `draft`, STOP — this packet is not activatable.

### Step 2: Register the three pilot modules behind per-module cargo features

- Task IDs: `ADR-0056`
- Objective: make `integrated_registrations()` and `native_entries()` return the three pilot entries when their features are enabled, and prove it (AC-1, AC-2, AC-6's static half).
- Precondition: Step 1's inventory is written; `cargo test -p slicer-integrated-modules` is green on the empty registry.
- Postcondition: with `--features classic-perimeters,arachne-perimeters,support-planner`, `integrated_registrations()` yields 3 registrations with ids `com.core.classic-perimeters` / `com.core.arachne-perimeters` / `com.core.support-planner` and origin labels `integrated://<dir>`, and `native_entries()` yields 3 pairs with families `Layer`, `Layer`, `Prepass`. With no features, both return what they returned before (empty).
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-integrated-modules/Cargo.toml` — whole file
  - `modules/core-modules/{classic-perimeters,arachne-perimeters,support-planner}/Cargo.toml` — whole files (each under 30 lines)
  - `modules/core-modules/{classic-perimeters,arachne-perimeters,support-planner}/<name>.toml` — `[module]` and `[stage]` sections only
- Files allowed to edit (at most 3):
  - `crates/slicer-integrated-modules/Cargo.toml`
  - `crates/slicer-integrated-modules/src/lib.rs`
  - `crates/slicer-integrated-modules/tests/` — only if 201 placed registry tests in a `tests/` dir rather than in-file; otherwise the AC-1/AC-2 tests go in `src/lib.rs`'s `#[cfg(test)] mod tests` and this slot is unused
- Files explicitly out of bounds:
  - `modules/core-modules/*/src/**` — never edited by this packet (packet 200 owns classic-perimeters' call sites)
  - `modules/core-modules/*/*.toml` — manifests are embedded verbatim, never edited
- Expected sub-agent dispatches:
  - Question: what are the `#[slicer_module]`-annotated type names and the SDK trait each implements in the three pilot crates?; scope: `modules/core-modules/{classic-perimeters,arachne-perimeters,support-planner}/src/lib.rs`; return: `LOCATIONS` (≤6 entries)
- Context cost: `S`
- Authoritative docs:
  - `docs/adr/0056-integrated-modules-native-dispatch.md` Decision items 1–2 — direct read
  - `docs/adr/0033-host-service-bridge-for-host-only-algorithms.md` — direct read (short); the `cfg`-split that makes the arachne native arm compile
- OrcaSlicer refs:
  - none
- Verification:
  - `cargo test -p slicer-integrated-modules --features classic-perimeters,arachne-perimeters,support-planner hybrid_pilot_registrations_are_exactly_three` — FACT pass/fail (AC-1)
  - `cargo test -p slicer-integrated-modules --features classic-perimeters,arachne-perimeters,support-planner hybrid_pilot_native_entry_families_match_stage_ids` — FACT pass/fail (AC-2)
  - `sh -c 'rg -q "arachne-perimeters" crates/slicer-integrated-modules/Cargo.toml && rg -U -q "\[features\][^\[]*arachne-perimeters" crates/slicer-integrated-modules/Cargo.toml && echo PASS'` — FACT PASS (AC-6 static half)
  - `cargo check -p slicer-integrated-modules` (no features) — FACT pass/fail; proves the default build is unchanged
- Exit condition: both feature-gated tests pass and a no-feature `cargo check -p slicer-integrated-modules` still compiles. If `arachne-perimeters` fails to link because `slicer_core::arachne::pipeline` is absent, `host-algos` did not unify — stop and diagnose `crates/slicer-sdk/Cargo.toml`'s `cfg(not(target_arch = "wasm32"))` block before proceeding.

### Step 3: Author the structural parity comparator and prove it is neither vacuous nor byte-exact

- Task IDs: `ADR-0056`, `ADR-0057`
- Objective: land `assert_parity_structural` / `assert_prepass_parity_structural` / `ParityTolerance` with self-tests that pass on ULP-scale drift and fail on dropped geometry — **for both families** (AC-N2, AC-N3 for the layer family; AC-N6 for the prepass family) — **before** any pilot module is compared. ADR-0042's D5 discriminator requirement applies to the prepass comparator exactly as it does to the layer one; shipping `assert_prepass_parity_structural` without its own dropped-geometry proof would leave AC-5 resting on an unverified instrument.
- Precondition: Step 2 green. No parity subject test exists yet.
- Postcondition: `crates/slicer-runtime/tests/common/parity_invariants.rs` implements exactly the invariant set enumerated in `requirements.md` §In Scope — closure within tolerance, loop count, loop nesting depth, bead-count sequence, **transitions-present**, no self-intersection, coverage ratio (computed symmetrically between the two paths), no bead wider than `max_bead_width_factor × optimal_width`, plus the two deliberate additions (per-loop point count and `ExtrusionRole` sequence) — together with a `coord_mm` tolerance. `transitions-present` is not optional: without it, a native path that flattened every bead transition still passes the bead-count-sequence check on uniform-width geometry. The prepass comparator implements the shape-specific analogue (`entries` count, full `(global_layer_index, object_id, region_id)` key set, per-entry `branch_segments` count, per-segment `points` count and `role`, per-point `(x, y, z, width)`). The self-test module proves both families in both directions.
- Files allowed to read, with ranges when over 300 lines:
  - `docs/adr/0042-arachne-parity-structural-invariants-over-fixtures.md` — §Decision only
  - `crates/slicer-runtime/tests/arachne_structural_invariants.rs` — lines `84-140` only (`symmetric_coverage_ratio`, `coverage_predicate`, `assert_capture_is_structural` as shape reference; re-locate by symbol name)
  - `crates/slicer-wasm-host/src/traits.rs` — the `LayerStageRunner` / `PrepassStageRunner` declarations only (they *return* the commit types; they do not define them)
  - `crates/slicer-ir/src/stage_io.rs` — the `LayerStageCommit` enum only (declared at line 545 at authoring; re-locate by symbol name)
  - `crates/slicer-core/src/stage_io.rs` — the `PrepassStageOutput` enum only (declared at line 26 at authoring; re-locate by symbol name)
  - `crates/slicer-ir/src/slice_ir.rs` (over 2000 lines) — **only** the `SupportPlanEntry`, `SupportPlanIR`, `ExtrusionPath3D`, and `Point3WithWidth` declarations; locate each by symbol name, never read the file whole
- Files allowed to edit (at most 3):
  - `crates/slicer-runtime/tests/common/parity_invariants.rs` (new)
  - `crates/slicer-runtime/tests/common/mod.rs` (one `pub mod` line)
  - `crates/slicer-runtime/tests/contract/parity_invariants_selftest_tdd.rs` (new)
- Files explicitly out of bounds:
  - `crates/slicer-runtime/tests/arachne_structural_invariants.rs` — read-only shape reference; its assertions are not modified
  - any `modules/core-modules/**`
- Expected sub-agent dispatches:
  - Question: what are the public items of `tests/common/perimeter_harness.rs` and `tests/common/support_wedge.rs`?; scope: `crates/slicer-runtime/tests/common/`; return: `LOCATIONS` (≤20 entries)
  - Question: what are the exact variants of `LayerStageCommit` and `PrepassStageOutput` (the payloads the comparator must walk)?; scope: `crates/slicer-ir/src/stage_io.rs` (`LayerStageCommit`, `LayerStageError`, `PrepassRunnerError`) and `crates/slicer-core/src/stage_io.rs` (`PrepassStageOutput`) — **not** `crates/slicer-wasm-host/src/`, which defines neither type and would return empty; return: `SNIPPETS` (≤2, ≤30 lines)
  - Question: exact field lists of `SupportPlanEntry`, `ExtrusionPath3D`, and `Point3WithWidth`?; scope: `crates/slicer-ir/src/slice_ir.rs`; return: `SNIPPETS` (≤3, ≤20 lines)
- Context cost: `M`
- Authoritative docs:
  - `docs/adr/0042-arachne-parity-structural-invariants-over-fixtures.md` — §Decision, direct read; delegate the rest
  - `docs/adr/0056-integrated-modules-native-dispatch.md` Decision item 4 — direct read
- OrcaSlicer refs:
  - none
- Verification:
  - `cargo test -p slicer-runtime --test contract -- parity_comparator_accepts_ulp_perturbation` — FACT pass/fail (AC-N2)
  - `cargo test -p slicer-runtime --test contract -- parity_comparator_rejects_dropped_loop` — FACT pass/fail (AC-N3)
  - `cargo test -p slicer-runtime --test contract -- parity_comparator_rejects_dropped_support_entry` — FACT pass/fail (AC-N6); covers all four prepass cases, including the shifted-`global_layer_index` case that proves the entry key is the full triple
  - `sh -c 'rg -q "mod parity_invariants_selftest_tdd" crates/slicer-runtime/tests/contract/main.rs && rg -q "pub mod parity_invariants" crates/slicer-runtime/tests/common/mod.rs && echo PASS'` — FACT PASS; guards the S7 silent-zero-tests failure
- Exit condition: the ULP-perturbation self-test passes; the dropped-loop self-test and all four prepass discriminator cases report `Err` from the comparator (i.e. the tests asserting `Err` pass); and both new files are registered in their aggregators. A comparator that accepts any of these cases is vacuous — stop and tighten it. In particular, if the shifted-`global_layer_index` case passes, the entry key is wrong.

### Step 4: Parity-gate `classic-perimeters`

- Task IDs: `ADR-0056`
- Objective: prove AC-3 — one `WasmRuntimeDispatcher`, two `CompiledModuleLive` values (native vs wasm), one identical `LayerStageInput`, structural agreement.
- Precondition: Step 3's comparator is green; `cargo xtask build-guests --check` reports clean.
- Postcondition: `integrated_parity_classic_perimeters_tdd.rs` exists, is registered, and passes; `classic-perimeters` is eligible for `hybrid.integrated_modules`.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-runtime/tests/contract/native_dispatch_parity_seam_tdd.rs` — whole file (packet 202's demonstration)
  - `crates/slicer-runtime/tests/common/parity_invariants.rs` — whole file (own output of Step 3)
  - `crates/slicer-wasm-host/src/binding.rs` — around `CompiledModuleLive` (25) and `LayerStageInput` (69) only
- Files allowed to edit (at most 3):
  - `crates/slicer-runtime/tests/contract/integrated_parity_classic_perimeters_tdd.rs` (new)
  - `crates/slicer-runtime/tests/contract/main.rs` (one `mod` line)
  - `crates/slicer-runtime/tests/common/parity_invariants.rs` (only if a genuinely missing invariant is discovered — tightening is allowed, loosening is not)
- Files explicitly out of bounds:
  - `modules/core-modules/classic-perimeters/src/lib.rs` — packet 200's surface
  - `crates/slicer-wasm-host/src/dispatch.rs` — packet 202's surface
- Expected sub-agent dispatches:
  - Question: does `cargo xtask build-guests --check` report clean?; scope: repo root; return: `FACT` clean / `STALE:` list
  - Question: which `perimeter_harness` fixture subject yields ≥2 loops and ≥1 nesting level with the classic wall generator?; scope: `crates/slicer-runtime/tests/common/perimeter_harness.rs` and `crates/slicer-runtime/tests/fixtures/`; return: `FACT` (≤5 lines)
- Context cost: `M`
- Authoritative docs:
  - `docs/adr/0056-integrated-modules-native-dispatch.md` Decision item 4 — direct read
- OrcaSlicer refs:
  - none
- Verification:
  - `cargo xtask build-guests --check` — FACT clean (rebuild without `--check` if `STALE:`)
  - `mkdir -p target && cargo test -p slicer-runtime --test contract -- integrated_parity_classic_perimeters 2>&1 | tee target/test-output.log && rg -q "^test result: ok" target/test-output.log` — FACT pass/fail (AC-3); on failure, `rg -C 5 'panicked at' target/test-output.log` for bounded SNIPPETS
- Exit condition: AC-3's command returns PASS with the freshness gate clean immediately before it. A failure is a real divergence to diagnose — never a reason to relax `ParityTolerance`.

### Step 5: Parity-gate `arachne-perimeters`

- Task IDs: `ADR-0056`
- Objective: prove AC-4 and AC-6's runtime half — the arachne native arm reaches `slicer_core::arachne::pipeline::run_arachne_pipeline` and agrees structurally with the wasm arm, including bead-count sequence and the `2.0 ×` optimal-width bound.
- Precondition: Step 4 green; `cargo xtask build-guests --check` clean.
- Postcondition: `integrated_parity_arachne_perimeters_tdd.rs` exists, is registered, and passes with a non-empty wall set on the native path.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-runtime/tests/contract/integrated_parity_classic_perimeters_tdd.rs` — whole file (Step 4's own output, the template)
  - `crates/slicer-sdk/src/host.rs` — around `pub fn generate_arachne_walls` only (the `cfg(not(target_arch = "wasm32"))` arm)
  - `crates/slicer-sdk/Cargo.toml` — whole file (30 lines)
- Files allowed to edit (at most 3):
  - `crates/slicer-runtime/tests/contract/integrated_parity_arachne_perimeters_tdd.rs` (new)
  - `crates/slicer-runtime/tests/contract/main.rs` (one `mod` line)
  - `docs/DEVIATION_LOG.md` (one row, **only** if the `[FWD]` tolerance question resolves to a widened coordinate tolerance; ID re-derived at write time)
- Files explicitly out of bounds:
  - `modules/core-modules/arachne-perimeters/src/lib.rs`
  - `crates/slicer-core/src/arachne/**` — canonical geometry; not touched by a parity packet
- Expected sub-agent dispatches:
  - Question: what is the highest `DEV-###` in `docs/DEVIATION_LOG.md` right now?; scope: `docs/DEVIATION_LOG.md`; return: `FACT` (1 line, from `rg -o '^\| DEV-[0-9]{3}' docs/DEVIATION_LOG.md | sort -u | tail -1`) — dispatched **only** if a row is needed
  - Question: does `cargo xtask build-guests --check` report clean?; scope: repo root; return: `FACT` clean / `STALE:` list
- Context cost: `M`
- Authoritative docs:
  - `docs/adr/0042-arachne-parity-structural-invariants-over-fixtures.md` §Decision — direct read; the D4 `2× optimal_width` invariant
  - `docs/adr/0033-host-service-bridge-for-host-only-algorithms.md` — direct read (short)
- OrcaSlicer refs:
  - none — the comparison is PnP-native vs PnP-wasm, not PnP vs canonical
- Verification:
  - `cargo xtask build-guests --check` — FACT clean
  - `mkdir -p target && cargo test -p slicer-runtime --test contract -- integrated_parity_arachne_perimeters 2>&1 | tee target/test-output.log && rg -q "^test result: ok" target/test-output.log` — FACT pass/fail (AC-4, AC-6 runtime half)
- Exit condition: AC-4 passes. If the default `1e-3` mm coordinate tolerance fails, the measured maximum per-point delta is recorded, the tolerance widened to the smallest value that passes, and a DEVIATION_LOG row filed with that measured number.
  - **Absolute ceiling: `coord_mm` may never exceed `1e-2` mm.** Beyond that the coordinate gate is inert at nozzle scale (a 0.4 mm nozzle emits 0.45 mm-wide beads; a 0.1 mm tolerance would accept a quarter-bead displacement), so a divergence needing more than `1e-2` mm is a real defect, not drift. If `1e-2` mm still fails: **drop `arachne-perimeters` from `hybrid.integrated_modules` in Step 8** and file the divergence, rather than widening further. The same ceiling binds Step 6.
  - Widening without a measured number, widening past the ceiling, or dropping an invariant all fail this step.

### Step 6: Parity-gate `support-planner`

- Task IDs: `ADR-0056`
- Objective: prove AC-5 — the `PrePass::SupportGeometry` prepass agrees structurally across dispatch paths on the `SupportPlanIR` carried by `PrepassStageOutput::SupportPlan`, keyed on the full `(global_layer_index, object_id, region_id)` triple and compared down through `branch_segments` → `points` → `(x, y, z, width)`.
- Precondition: Step 5 green; `cargo xtask build-guests --check` clean; AC-N6's four prepass discriminator cases already green from Step 3 — the comparator this step consumes must be proven non-vacuous before it certifies a module.
- Postcondition: `integrated_parity_support_planner_tdd.rs` exists, is registered, and passes.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-runtime/tests/common/support_wedge.rs` — whole file
  - `crates/slicer-wasm-host/src/binding.rs` — around `PrepassStageInput` (101) only
  - `crates/slicer-wasm-host/src/traits.rs` — the `PrepassStageRunner` declaration only
  - `crates/slicer-core/src/stage_io.rs` — the `PrepassStageOutput` enum only (the `SupportPlan(Arc<SupportPlanIR>)` variant is the one AC-5 matches)
  - `crates/slicer-ir/src/slice_ir.rs` (over 2000 lines) — **only** `SupportPlanIR`, `SupportPlanEntry`, `ExtrusionPath3D`, `Point3WithWidth`, located by symbol name
- Files allowed to edit (at most 3):
  - `crates/slicer-runtime/tests/contract/integrated_parity_support_planner_tdd.rs` (new)
  - `crates/slicer-runtime/tests/contract/main.rs` (one `mod` line)
  - `crates/slicer-runtime/tests/common/parity_invariants.rs` (only to add `assert_prepass_parity_structural` detail; tightening only)
- Files explicitly out of bounds:
  - `modules/core-modules/support-planner/src/lib.rs`
  - `crates/slicer-sdk/src/host_batch.rs` — packet 200's surface
- Expected sub-agent dispatches:
  - Question: what does `support_wedge.rs` expose and how does an existing prepass test harvest `SupportPlanIR` from a `PrepassStageOutput`?; scope: `crates/slicer-runtime/tests/common/support_wedge.rs`, `crates/slicer-runtime/tests/integration/support_invariants_wedge_tdd.rs`; return: `SNIPPETS` (≤2, ≤30 lines)
  - Question: does `cargo xtask build-guests --check` report clean?; scope: repo root; return: `FACT` clean / `STALE:` list
- Context cost: `M`
- Authoritative docs:
  - `docs/adr/0056-integrated-modules-native-dispatch.md` Decision items 4–5 — direct read
- OrcaSlicer refs:
  - none
- Verification:
  - `cargo xtask build-guests --check` — FACT clean
  - `mkdir -p target && cargo test -p slicer-runtime --test contract -- integrated_parity_support_planner 2>&1 | tee target/test-output.log && rg -q "^test result: ok" target/test-output.log` — FACT pass/fail (AC-5)
- Exit condition: AC-5 passes, and any tolerance change made here re-runs AC-N6 to confirm the four discriminator cases still report `Err`. Step 5's absolute `coord_mm` ceiling of `1e-2` mm binds here too. A module whose parity test cannot be made green without weakening an assertion or exceeding the ceiling is dropped from `hybrid.integrated_modules` in Step 8 and recorded as such — its feature and test still ship.

### Step 7: External-override negative gate and the ADR-0056 Decision item 5 single-threaded check

- Task IDs: `ADR-0056`
- Objective: prove AC-N4 (a disk-root `com.core.classic-perimeters` still forces WASM dispatch even with the integrated registration and native entry present) and AC-9 (no internal parallelism in the three pilot crates).
- Precondition: Steps 4–6 green.
- Postcondition: `hybrid_pilot_external_override_tdd.rs` exists, is registered in the `integration` aggregator, and passes; AC-9's static check returns PASS.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-wasm-host/src/execution_plan_live.rs` — around `LiveModuleBinding` (39) and `compile_module_component` (326) only
  - `crates/slicer-runtime/tests/integration/live_module_loading_tdd.rs` — whole file (the existing loader-test template)
  - `modules/core-modules/{classic-perimeters,arachne-perimeters,support-planner}/Cargo.toml` — whole files
- Files allowed to edit (at most 3):
  - `crates/slicer-runtime/tests/integration/hybrid_pilot_external_override_tdd.rs` (new)
  - `crates/slicer-runtime/tests/integration/main.rs` (one `mod` line)
- Files explicitly out of bounds:
  - `crates/slicer-wasm-host/src/execution_plan_live.rs` — packet 201/202's surface; read-only
  - `crates/slicer-scheduler/src/manifest.rs` — packet 201's surface; read-only
- Expected sub-agent dispatches:
  - Question: how does an existing integration test construct a temporary disk search root containing one module's `.wasm` + `.toml`?; scope: `crates/slicer-runtime/tests/integration/live_module_loading_tdd.rs`; return: `SNIPPETS` (1, ≤30 lines)
- Context cost: `S`
- Authoritative docs:
  - `docs/adr/0056-integrated-modules-native-dispatch.md` Decision item 2 and Decision item 5, plus its "an edition must never stage an external copy" consequence — direct read
- OrcaSlicer refs:
  - none
- Verification:
  - `cargo test -p slicer-runtime --test integration hybrid_pilot_external_override_forces_wasm` — FACT pass/fail (AC-N4)
  - `sh -c 'for m in classic-perimeters arachne-perimeters support-planner; do rg -q "^(rayon|\[dependencies\.rayon\]|\[target\..*dependencies\.rayon\])" modules/core-modules/$m/Cargo.toml && { echo "FAIL rayon dep: $m"; exit 1; }; rg -q "par_iter|par_bridge|par_chunks|rayon::" modules/core-modules/$m/src/ && { echo "FAIL rayon use: $m"; exit 1; }; done; echo PASS'` — FACT PASS (AC-9). **Use this exact form, identical to `packet.spec.md` AC-9** — a bare `^rayon` alternation misses the `[dependencies.rayon]` and `[target.'cfg(...)'.dependencies.rayon]` table forms, so a dependency added that way would slip the gate.
  - `sh -c 'rg -q "mod hybrid_pilot_external_override_tdd" crates/slicer-runtime/tests/integration/main.rs && echo PASS'` — FACT PASS; aggregator registration guard
- Exit condition: AC-N4 and AC-9 both PASS. A green AC-N4 that ran zero tests (unregistered `mod`) is a false pass — the registration guard command must PASS in the same step.

### Step 8: Create `dist/editions.toml`, its `xtask` reader, and the profiling evidence

- Task IDs: `ADR-0057`
- Objective: land the Hybrid dist-config list with a validating reader (AC-7, AC-N1) and finalize its membership from an ADR-0055-methodology profiling run (AC-8).
- Precondition: Steps 4–7 green; the set of modules with a green parity gate is known.
- Postcondition: `dist/editions.toml` exists with three editions and a `# evidence:` header block carrying **both** ADR-0055 signals per module — guest fuel (primary) and profiling-off wall-clock share (secondary) — plus the model, the exact command, and the run-to-run spread; `xtask::editions::load_editions` parses and validates it; both `xtask` unit tests pass.
- Files allowed to read, with ranges when over 300 lines:
  - `xtask/src/build_guests.rs` — `discover_guests` (108), `GuestSpec` fields (15–24), `GuestTree` only
  - `xtask/src/dist.rs` — its core-tree staging loop (lines `78-120` at authoring; re-locate by `discover_guests`) — purpose: confirm the `file_stem` naming this config must match. Read-only.
  - `xtask/src/main.rs` — the `mod` declaration block only
  - `docs/adr/0055-fuel-based-module-profiling.md` — whole file (127 lines)
- Files allowed to edit (at most 3):
  - `dist/editions.toml` (new)
  - `xtask/src/editions.rs` (new, including its `#[cfg(test)] mod tests`)
  - `xtask/src/main.rs` (one `mod editions;` line)
- Files explicitly out of bounds:
  - `xtask/src/dist.rs` — packet 205's surface; this packet creates the config, not its consumer
  - `xtask/Cargo.toml` — no new dependency is needed; `toml = "0.8"` is already declared. If a step believes it needs one, stop and re-check.
- Expected sub-agent dispatches:
  - Question: for a release `--profile` capture, what is each pilot module's **guest fuel** and its wall-clock share, and what is the run-to-run spread over 3 runs?; scope: the step's own `target/p204-*.jsonl` captures, `target/p204-summary.json`, and the three `target/p204-noprof-{1,2,3}.log` instrumented runs; return: `FACT` (≤8 lines). Note fuel is a *guest* counter: ADR-0055 states verbatim that "native code is not metered", so a natively-dispatched module reports zero and the fuel figure must come from the external/wasm build. (The ADR's "burns no fuel" clause is scoped to the profile *mark* host call, not host calls in general; generalizing it is an inference. The ADR predates integrated modules and does not discuss them.) ADR-0055 also records that wall-clock under `--profile` is inflated by mark host calls, so the wall-clock figure comes from the profiling-off `--instrument-stderr` runs above.
- Context cost: `M`
- Authoritative docs:
  - `docs/adr/0057-three-editions-and-integrated-tier.md` — direct read (55 lines); the edition table and the dist-config-list clause
  - `docs/adr/0055-fuel-based-module-profiling.md` — direct read (127 lines); fuel primary, wall-clock secondary, explicit run-to-run spread
- OrcaSlicer refs:
  - none
- Verification:
  - `cargo test -p xtask editions_config_declares_three_editions` — FACT pass/fail (AC-7)
  - `cargo test -p xtask editions_config_rejects_unknown_module_name` — FACT pass/fail (AC-N1)
  - `sh -c 'cargo run --bin pnp_cli --release -- slice --model resources/extruder_idler.obj --module-dir modules/core-modules --output target/p204.gcode --profile 2> target/p204-profile.jsonl && cargo run --bin pnp_cli --release -- profile --from target/p204-profile.jsonl --json > target/p204-summary.json && rg -q "com.core.classic-perimeters" target/p204-summary.json && echo PASS'` — FACT PASS; produces the AC-8 raw numbers
  - `sh -c 'for i in 1 2 3; do cargo run --bin pnp_cli --release -- slice --model resources/extruder_idler.obj --module-dir modules/core-modules --output target/p204-noprof.gcode --instrument-stderr 2> target/p204-noprof-$i.log || exit 1; done; rg -c . target/p204-noprof-1.log > /dev/null && echo PASS'` — FACT PASS **plus the three captured logs**. This is the profiling-off absolute-timing run that supplies AC-8's wall-clock figures and its run-to-run spread. `--instrument-stderr` (not `--profile`) is the correct instrument here: ADR-0055 records that wall-clock under `--profile` is inflated by mark host calls while "a plain instrumented run is unaffected". Redirecting stderr to a file rather than `/dev/null` is the point — a run whose output is discarded proves only that the binary exited 0 and yields no measurable number, which would leave this step's dispatch contract and exit condition unsatisfiable.
  - `sh -c 'rg -q "^# *evidence:" dist/editions.toml && rg -qi "fuel" dist/editions.toml && rg -qi "wall-clock" dist/editions.toml && rg -q "extruder_idler|benchy" dist/editions.toml && rg -q "run-to-run spread" dist/editions.toml && echo PASS'` — FACT PASS (AC-8)
- Exit condition: AC-7, AC-N1, and AC-8 all PASS; every name in `hybrid.integrated_modules` has a green parity test from Steps 4–6; and the evidence block carries a fuel figure **and** a wall-clock figure **and** a run-to-run spread for each of the three seeded modules. Every number was measured in this step — an unmeasured figure fails the step regardless of the grep (`CLAUDE.md` §"No Unverified Metrics"). An evidence block with wall-clock but no fuel records only ADR-0055's *secondary* signal and fails.

### Step 9: Doc impact, residual-divergence row, and closure gates

- Task IDs: `ADR-0056`, `ADR-0057`
- Objective: land the `docs/01_system_architecture.md` paragraph (AC-N5), file a DEVIATION_LOG row if and only if residual divergence was observed, and clear the workspace static gates.
- Precondition: Steps 1–8 green.
- Postcondition: AC-N5's greps pass; `cargo check`/`cargo clippy --workspace --all-targets` are clean; `cargo xtask build-guests --check` is clean.
- Files allowed to read, with ranges when over 300 lines:
  - `docs/01_system_architecture.md` — §"Producing the tier-4 layout: `cargo xtask dist`" only (heading at line 982 at authoring; re-locate by heading text)
  - `dist/editions.toml` — whole file (own output of Step 8)
- Files allowed to edit (at most 3):
  - `docs/01_system_architecture.md`
  - `docs/DEVIATION_LOG.md` (conditional — only if a Step 5/6 divergence was accepted; ID re-derived at write time, never pinned)
- Files explicitly out of bounds:
  - `docs/adr/**`, `CONTEXT.md`, `docs/07_implementation_status.md`, `docs/specs/multi-edition-distribution-plan.md` — this packet amends no ADR and adds no backlog row
  - `docs/spec_packets/194-*` … `docs/spec_packets/203-*`
- Expected sub-agent dispatches:
  - Question: what is the highest `DEV-###` in `docs/DEVIATION_LOG.md` right now, and what is the column order of the header row?; scope: `docs/DEVIATION_LOG.md`; return: `FACT` (≤3 lines) — dispatched **only** if a row is needed
  - Question: do `cargo check --workspace --all-targets` and `cargo clippy --workspace --all-targets -- -D warnings` pass?; scope: repo root; return: `FACT` pass/fail with ≤20 lines on failure
- Context cost: `S`
- Authoritative docs:
  - `docs/01_system_architecture.md` §"Producing the tier-4 layout: `cargo xtask dist`" — ranged read
- OrcaSlicer refs:
  - none
- Verification:
  - `sh -c 'rg -q "dist/editions.toml" docs/01_system_architecture.md && rg -q "integrated_modules" docs/01_system_architecture.md && echo PASS'` — FACT PASS (AC-N5)
  - `cargo check --workspace --all-targets` — FACT pass/fail
  - `cargo clippy --workspace --all-targets -- -D warnings` — FACT pass/fail
  - `cargo xtask build-guests --check` — FACT clean
- Exit condition: all three gate commands and AC-N5 PASS, and any DEVIATION_LOG row uses an ID re-derived in this step (not one carried from earlier in the session — a parallel packet may have claimed it).

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | S | Read-only FORWARD-DEP reconciliation; two 75-line spec files + one small lib |
| Step 2 | S | Three small manifests + two cfg-gated push blocks |
| Step 3 | M | Comparator design against ADR-0042 §Decision + two commit shapes |
| Step 4 | M | First parity subject; establishes the template Steps 5–6 copy |
| Step 5 | M | Arachne; may require a measured tolerance widening + DEV row |
| Step 6 | M | Prepass family; different input/output shapes than Steps 4–5 |
| Step 7 | S | One negative test + two static greps |
| Step 8 | M | Config + reader + a release profiling run |
| Step 9 | S | One doc paragraph + workspace static gates |

Split before activation if aggregate cost exceeds M or any step is L. No step is L: the four M steps each read one ADR section plus two bounded snippets, and Steps 5–6 reuse Step 4's authored template rather than re-deriving the seam.

## Packet Completion Gate

- All steps and exits complete.
- Every pipe-suffixed AC command returns PASS.
- `docs/07_implementation_status.md` is **not** updated by this packet: no TASK row exists for the distribution/editions workstream (`requirements.md` §Packet Metadata), and the plan's `[FWD]` proposal to create one is unresolved and explicitly forbids editing that file while the parallel 194–199 session is active. Record this as a deliberate no-op in the closure report rather than dispatching a worker.
- No status transition to reconcile: this packet reopens and supersedes nothing.
- `packet.spec.md` is ready for `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed AC and packet-level gate command: AC-1, AC-2, AC-3, AC-4, AC-5, AC-6, AC-7, AC-8, AC-9, AC-N1, AC-N2, AC-N3, AC-N4, AC-N5, AC-N6, plus `cargo check --workspace --all-targets`, `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo xtask build-guests --check`.
- `cargo test --workspace` is **not** required for closure and must not be run for this packet (`CLAUDE.md` §Test Discipline permits it only when a packet's gate requires it; this one does not).
- Record remaining packet-local risk: any module dropped from `hybrid.integrated_modules` by the AC-8 evidence or by hitting Step 5's `1e-2` mm tolerance ceiling, and any tolerance widened in Step 5 or Step 6 with its measured delta and DEV row.
- Confirm context stayed at or below 150k standard, or at/below 300k only with a logged swarm ESCALATION; otherwise record a packet-authoring lesson.

All `cargo check`, `cargo clippy`, and `cargo test` invocations in gate and verification commands must use `--all-targets` where the command form admits it (the `-p <crate> --test <bin>` forms name a single target by construction and do not).
