# Design: 205a-integrated-edition-coverage

## Controlling Code Paths

- Primary code path: `crates/slicer-integrated-modules/src/lib.rs`'s `integrated_registrations()` and `native_entries()` (created by 201/202, extended by 204 to three pilots). This packet adds sixteen `#[cfg(feature = "...")]` push blocks to each.
- The dual-dispatch seam formed by `WasmRuntimeDispatcher` (`crates/slicer-wasm-host/src/dispatch.rs`) driving two `CompiledModuleLive` values through `LayerStageRunner::run_stage` / `PrepassStageRunner::run_stage` / the finalization runner (`crates/slicer-wasm-host/src/traits.rs`), exactly as packet 204 demonstrated for its three pilots.
- The parity comparator `crates/slicer-runtime/tests/common/parity_invariants.rs` (`assert_parity_structural`, `assert_prepass_parity_structural`, `ParityTolerance`) — 204's shared, self-tested comparator, reused here.
- The coverage gate `verify_integrated_feature_coverage` and `pnp_cli_integrated_features` in `xtask/src/dist.rs` (packet 205) — the consumer that reports which modules remain uncovered.
- OrcaSlicer comparison: none. This packet compares PnP's native path against PnP's own wasm path; there is no canonical C++ equivalent of a dispatch-path split.

## Architecture Constraints

- **Byte-equality is not the gate (ADR-0056 Decision item 4).** The comparator asserts structural invariants plus a `1e-3` mm coordinate tolerance. It must never compare `f32`/`f64` bit patterns, and it must never be relaxed to make a red parity test green — a red parity test removes its module from the integrated set (it is simply not added to `dist/editions.toml`'s membership), it does not relax the assertion.
- **Single-threaded module logic on both paths (ADR-0056 Decision item 5).** Enforced cheaply by AC-7's static negative check over the sixteen crates' `Cargo.toml` and `src/**`. Verified at authoring: none of the sixteen declares `rayon` (the only `rayon` hits under `modules/core-modules/` are transitive entries in `wit-guest/Cargo.lock` files, which AC-7's `src/`-and-`Cargo.toml`-scoped greps correctly ignore).
- **The native-transport scope is the packet's load-bearing fact.** Sixteen of the eighteen remaining modules map to stages the native marshal commits; two (`path-optimization-default` → `Layer::PathOptimization`, `machine-gcode-emit` → `PostPass::GCodePostProcess`) map to stages that return a fatal `Err`. This packet integrates only the sixteen. The two excluded modules are packet 205b's scope, which must first complete the two transports. **Do not attempt to integrate a transport-blocked module** — the parity test would fail on the native path's fatal error, and the correct response is to defer, not to weaken the gate.
- **Feature unification is load-bearing for native capability.** As in 204, a module's native arm may depend on `host-algos` arriving through `slicer-sdk`'s `cfg(not(target_arch = "wasm32"))` dependency. `crates/slicer-integrated-modules/` must not be built with `--no-default-features` in a way that severs `slicer-sdk`. Each newly-integrated module's native path must be exercised by its parity test (a compile-time success does not prove the feature reached it).
- **New stage families beyond 204's pilots.** 204 demonstrated parity for `Layer::Perimeters` (×2) and `PrePass::SupportGeometry`. This packet's sixteen modules span `Layer::Infill`, `Layer::InfillPostProcess`, `Layer::Support`, `Layer::SupportPostProcess`, `Layer::PerimetersPostProcess`, `PrePass::LayerPlanning`, `PrePass::SeamPlanning`, and `PostPass::LayerFinalization`. Each family needs an appropriate stage input fixture and a family-appropriate parity assertion. The layer-family comparator (`assert_parity_structural`) covers the `Layer::*` stages; the prepass comparator (`assert_prepass_parity_structural`) covers `PrePass::*`; finalization needs a new comparator or a structural assertion over the merged `LayerCollectionIR` (see §Open Questions). This is the packet's main design risk and the reason it is rated L.
- **Guest WASM staleness applies.** Sixteen `Cargo.toml` edits invalidate their wasm twins. After any edit, run `cargo xtask build-guests --check` and rebuild without `--check` if `STALE:` is reported before re-running a parity test.
- **Coordinate units:** 1 unit = 100 nm (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary.
- No public schema/version constant is bumped and no struct gains a field in a crate with existing struct-literal sites; the only new fields are entries in the two registry vectors, which are net-new with no pre-existing literal sites. Blast-radius discipline does not bind.

## Code Change Surface

- **Selected approach:** register-then-gate-then-verify, mechanically repeating 204's pattern per module. (1) Add the sixteen modules to the registry crate behind per-module cargo features. (2) Author one parity contract test per module reusing 204's seam and comparator, plus any new family comparators. (3) Extend `crates/pnp-cli/Cargo.toml`'s passthrough features. (4) Verify the coverage gate now reports only the two transport-blocked modules.
- **Exact functions, traits, manifests, tests, and files:**
  - `crates/slicer-integrated-modules/Cargo.toml` — sixteen `[dependencies]` entries marked `optional = true` with path `../../modules/core-modules/<name>`, plus `[features]` `<name> = ["dep:<name>"]` (feature names = module directory names, per 201's convention).
  - `crates/slicer-integrated-modules/src/lib.rs` — `integrated_registrations()` gains sixteen `#[cfg(feature = "...")]` push blocks building `IntegratedModuleRegistration { manifest_toml: include_str!("../../../modules/core-modules/<name>/<name>.toml"), origin_label: "integrated://<name>" }`; `native_entries()` gains sixteen `#[cfg(feature = "...")]` push blocks of `(ModuleId::from("com.core.<name>"), <Type>::__slicer_native_entry())`. `ModuleId` is a type alias (`pub type ModuleId = String;` in `crates/slicer-ir/src/slice_ir.rs`), so `ModuleId::from(&str)` is `String::from`. The `#[slicer_module]`-annotated type name and the SDK trait each module implements MUST be re-derived by dispatch at implementation time (each module's `src/lib.rs`), not assumed.
  - `crates/slicer-runtime/tests/common/parity_invariants.rs` — extend with any new family comparators (finalization; possibly a seam-plan comparator) plus their self-tests, following 204's `parity_invariants_selftest_tdd.rs` pattern.
  - `crates/slicer-runtime/tests/contract/` — sixteen new per-module parity contract test files (`integrated_parity_fuzzy_skin_tdd.rs`, `integrated_parity_gyroid_infill_tdd.rs`, `integrated_parity_infill_linker_tdd.rs`, `integrated_parity_layer_planner_tdd.rs`, `integrated_parity_lightning_infill_tdd.rs`, `integrated_parity_overhang_classifier_tdd.rs`, `integrated_parity_part_cooling_tdd.rs`, `integrated_parity_rectilinear_infill_tdd.rs`, `integrated_parity_seam_placer_tdd.rs`, `integrated_parity_seam_planner_tdd.rs`, `integrated_parity_skirt_brim_tdd.rs`, `integrated_parity_support_surface_ironing_tdd.rs`, `integrated_parity_top_surface_ironing_tdd.rs`, `integrated_parity_traditional_support_tdd.rs`, `integrated_parity_tree_support_tdd.rs`, `integrated_parity_wipe_tower_tdd.rs`), each mounting the appropriate comparator on a byte-identical stage input; plus `mod` lines in `crates/slicer-runtime/tests/contract/main.rs`.
  - `crates/slicer-runtime/tests/integration/full_coverage_external_override_tdd.rs` — AC-N2; plus a `mod` line in `crates/slicer-runtime/tests/integration/main.rs`.
  - `crates/pnp-cli/Cargo.toml` — sixteen `integrated-<name> = ["slicer-integrated-modules/<name>"]` passthrough features (packet-205 AC-7 form).
  - `docs/01_system_architecture.md`, `docs/specs/multi-edition-distribution-plan.md` — the doc edits in `packet.spec.md` §Doc Impact Statement.
- **Rejected alternatives and reasons:**
  - *Integrate all eighteen in one packet.* Rejected: `path-optimization-default` and `machine-gcode-emit` map to native transports that return a fatal error today. A parity test on either would fail on the native path's `Err`, and weakening the gate to admit them would ship an "Integrated" edition that cannot actually dispatch those modules natively. They are 205b's scope, behind transport completion.
  - *One combined parity test parameterized over all sixteen.* Rejected (same reason as 204): a single test that fails cannot tell you which module diverged, and each module's gate must be independently red or green.
  - *Skip parity gates for the "simple" modules.* Rejected: ADR-0056 Decision item 4 requires a parity gate per integrated module; a module without a gate is not certified and must not be integrated.
  - *Compare final G-code instead of stage commits.* Rejected (same reason as 204): G-code folds in seam placement, path optimization, and emission, so a divergence there cannot be attributed to the dispatch path under test.

## Files in Scope (read + edit)

This packet is large by necessity: sixteen modules × (registry feature + native entry + parity test) plus the passthrough features and doc edits. Each implementation-plan step stays at or under 3 edits; the packet is split into per-family steps so no single step is L.

- `crates/slicer-integrated-modules/Cargo.toml` — role: sixteen feature/dependency declarations.
- `crates/slicer-integrated-modules/src/lib.rs` — role: registration and native-entry tables.
- `crates/slicer-runtime/tests/common/parity_invariants.rs` — role: new family comparators + self-tests.
- `crates/slicer-runtime/tests/contract/` — role: sixteen per-module parity gate test files (`integrated_parity_fuzzy_skin_tdd.rs`, `integrated_parity_gyroid_infill_tdd.rs`, `integrated_parity_infill_linker_tdd.rs`, `integrated_parity_layer_planner_tdd.rs`, `integrated_parity_lightning_infill_tdd.rs`, `integrated_parity_overhang_classifier_tdd.rs`, `integrated_parity_part_cooling_tdd.rs`, `integrated_parity_rectilinear_infill_tdd.rs`, `integrated_parity_seam_placer_tdd.rs`, `integrated_parity_seam_planner_tdd.rs`, `integrated_parity_skirt_brim_tdd.rs`, `integrated_parity_support_surface_ironing_tdd.rs`, `integrated_parity_top_surface_ironing_tdd.rs`, `integrated_parity_traditional_support_tdd.rs`, `integrated_parity_tree_support_tdd.rs`, `integrated_parity_wipe_tower_tdd.rs`).
- `crates/slicer-runtime/tests/contract/main.rs` — role: aggregator.
- `crates/slicer-runtime/tests/integration/full_coverage_external_override_tdd.rs` — role: AC-N2.
- `crates/slicer-runtime/tests/integration/main.rs` — role: aggregator.
- `crates/pnp-cli/Cargo.toml` — role: passthrough features.
- `docs/01_system_architecture.md`, `docs/specs/multi-edition-distribution-plan.md` — role: doc impact.
- `docs/DEVIATION_LOG.md` — role: **conditional edit, only if a measured residual divergence forces a tolerance widening** per §Open Questions (the `1e-3` → next order of magnitude rule). Append one row carrying the measured maximum per-point delta and rationale; re-derive the highest DEV-### at write time.

## Read-Only Context

- `docs/spec_packets/204-hybrid-pilot-parity/packet.spec.md` and `design.md` — whole files; the pattern to replicate.
- `crates/slicer-wasm-host/src/marshal/native.rs` — the committed-vs-fatal stage dispatch (re-verify the sixteen stages are committed).
- `crates/slicer-wasm-host/src/traits.rs` — the `LayerStageRunner`, `PrepassStageRunner`, and finalization runner declarations only.
- `crates/slicer-wasm-host/src/binding.rs` — `CompiledModuleLive`, `LayerStageInput`, `PrepassStageInput` construction shape.
- `crates/slicer-runtime/tests/common/` — `perimeter_harness.rs`, `dispatch_fixture.rs`, `support_wedge.rs`, `parity_invariants.rs` public items.
- `crates/slicer-ir/src/stage_io.rs`, `crates/slicer-ir/src/slice_ir.rs`, `crates/slicer-core/src/stage_io.rs` — the commit/IR type shapes (NOT `crates/slicer-wasm-host/`, which defines neither commit type).
- `modules/core-modules/<name>/src/lib.rs` for each of the sixteen — the `#[slicer_module]` type name and SDK trait, by `rg` only.
- `dist/editions.toml`, `xtask/src/dist.rs` — read-only; the coverage gate's consumer shape.

## Out-of-Bounds Files

- `modules/core-modules/<name>/src/lib.rs` for all sixteen — **never edited**; symbol lookups only, by dispatch.
- `crates/slicer-macros/**`, `crates/slicer-sdk/src/native.rs`, `crates/slicer-wasm-host/src/{dispatch.rs,marshal/**}` — packet 202's surface; read-only.
- `crates/slicer-scheduler/src/manifest.rs`, `crates/slicer-wasm-host/src/execution_plan_live.rs` — packet 201/202's surface; read-only.
- `xtask/src/dist.rs`, `xtask/src/editions.rs`, `dist/editions.toml` — packet 205/204's surface; read-only (this packet does not change edition membership).
- `modules/core-modules/path-optimization-default/**`, `modules/core-modules/machine-gcode-emit/**` — packet 205b's surface; out of scope here.
- `docs/adr/**`, `CONTEXT.md`, `docs/07_implementation_status.md` — never modified by this packet.
- `OrcaSlicerDocumented/` — not applicable; never load.
- `target/`, `Cargo.lock`, `modules/core-modules/*/wit-guest/Cargo.lock`, generated code, vendored dependencies — never load.

## Expected Sub-Agent Dispatches

- Question: what is the `#[slicer_module]`-annotated type name and the SDK trait each of the sixteen modules implements? scope: `modules/core-modules/<name>/src/lib.rs`; return: `LOCATIONS` (≤32 entries); purpose: the registration/native-entry tables.
- Question: what are the exact commit/IR type shapes for the new stage families (finalization, seam-plan, layer-planning)? scope: `crates/slicer-ir/src/stage_io.rs`, `crates/slicer-ir/src/slice_ir.rs`, `crates/slicer-core/src/stage_io.rs`; return: `SNIPPETS` (≤4, ≤30 lines); purpose: the new family comparators.
- Question: how does `native_dispatch_parity_seam_tdd.rs` construct its two `CompiledModuleLive` values and its `WasmRuntimeDispatcher`? scope: `crates/slicer-runtime/tests/contract/native_dispatch_parity_seam_tdd.rs`; return: `SNIPPETS` (1, ≤30 lines); purpose: seam reuse.
- Question: what are the public items of `tests/common/perimeter_harness.rs`, `dispatch_fixture.rs`, `support_wedge.rs`, `parity_invariants.rs`? scope: `crates/slicer-runtime/tests/common/`; return: `LOCATIONS` (≤20); purpose: fixtures and comparators.
- Question: does `cargo xtask build-guests --check` report clean? scope: repo root; return: `FACT` clean / `STALE:` list; purpose: before every parity run.

## Data and Contract Notes

- **IR/manifest contracts:** unchanged. Every module manifest is embedded verbatim by `include_str!`; no `[module]`, `[stage]`, `[ir-access]`, `[claims]`, or `[compatibility]` key is edited. Module ids stay `com.core.<name>`; stage ids stay as declared. Config keys touched: none — this packet does not edit `dist/editions.toml`.
- **WIT boundary:** unchanged. No WIT file is edited, so `crates/slicer-schema/wit/**` staleness does not apply — but the sixteen `Cargo.toml` edits still invalidate those guests' fingerprints, which is why `build-guests --check` is a gate command.
- **Determinism/scheduler constraints:** integrated modules must produce identical scheduling behavior to their wasm twins — claims, DAG position, and IR access are read from the same manifest text, so the scheduler cannot observe the difference (ADR-0056 Decision item 1). DEV-093 run-to-run nondeterminism means a parity test must dispatch both paths within one process on one fixture and must not compare against any stored snapshot.

## Locked Assumptions and Invariants

- **Locked:** edition membership lives in `dist/editions.toml` and nowhere else; this packet does not change it. It only makes more modules registry-available.
- **Locked:** names in the registry are module **directory** names — simultaneously the `slicer-integrated-modules` cargo feature, the `modules/core-modules/<name>` directory, and the `<name>.wasm` / `<name>.toml` stem `xtask dist` stages.
- **Locked:** parity is certified by structural invariants plus tolerance, never by byte-equality and never by a stored snapshot (ADR-0042, ADR-0056 Decision item 4).
- **Locked:** integrated modules stay single-threaded internally (ADR-0056 Decision item 5).
- **Locked:** a module whose native transport returns a fatal error is NOT integrated; it is deferred to 205b. Weakening the gate to admit it is a defect.
- **Reversible:** every module is behind an off-by-default cargo feature. A build with no new feature enabled is byte-identical in behavior to the pre-packet tree.

## Risks and Tradeoffs

- **The new stage families (finalization, seam-plan, layer-planning) lack a demonstrated parity-test pattern.** 204 only demonstrated `Layer::Perimeters` and `PrePass::SupportGeometry`. The finalization comparator over merged `LayerCollectionIR` and the seam-plan comparator are new design work. Mitigated by authoring each new comparator with its own self-tests (the 204 `parity_invariants_selftest_tdd.rs` pattern) before any subject test, and by the `1e-2` mm absolute `coord_mm` ceiling (never widen past it; a divergence needing more is a defect, not drift).
- **A stale wasm twin silently invalidates every parity result.** Mitigated by the freshness gate immediately before each parity run.
- **A module's native arm may not actually reach its host-algos feature.** Mitigated by each parity test exercising the native path (a compile-time success does not prove the feature reached it), mirroring 204's AC-6.
- **The packet is large (sixteen modules).** Mitigated by splitting into per-family steps, each at or under 3 edits, and by the mechanical repetition of 204's pattern. The two transport-blocked modules are explicitly out of scope, keeping the packet to committable work.

## Context Cost Estimate

- Aggregate: `L` — this packet is the largest in the plan. It is split into per-family steps so no single step is L; the aggregate is L because sixteen modules × (registry + native entry + parity test) is inherently large. Per the swarm escalation protocol, this packet runs in the extended band or is split further at activation.

## Open Questions

- `[FWD]` What is the finalization parity assertion? `PostPass::LayerFinalization` merges into `LayerCollectionIR` via `commit_native_finalization_response`. The comparator should assert structural equality of the merged layer collections (per-layer path counts, roles, point counts, coordinate tolerance) between the two paths. Implementer-resolvable; does not change scope.
- `[FWD]` What is the seam-plan parity assertion? `PrePass::SeamPlanning` produces `SeamPlanIR`. The comparator should assert structural equality of the seam plan (per-loop chosen seam position, candidate scores within tolerance) between the two paths. Implementer-resolvable.
- `[FWD]` Which fixture drives each parity test? Reuse `tests/common/` fixtures where possible; for the new families, the smallest subject that exercises multiple loops and at least one transition. Implementer-resolvable.
- `[FWD]` Does any newly-integrated module need a widened coordinate tolerance because of a WIT round-trip? Resolve empirically per module: start at `ParityTolerance::default()` `1e-3` mm; if it fails, measure the actual maximum per-point delta, widen to the next order of magnitude that passes, and record the measured number plus rationale in a DEVIATION_LOG row. Never widen without a measured number, and never past `1e-2` mm.
