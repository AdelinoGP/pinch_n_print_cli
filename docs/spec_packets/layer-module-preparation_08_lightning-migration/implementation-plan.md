# Implementation Plan: lightning-migration

## Execution Rules

Every step maps to `TASK-577`, uses an S/M context slice, and is numbered in **execution order**: the oracle exists before the first production edit; the new path is proven before anything old is removed; deletions are leaf-first so every step's own check compiles.

- **Planning numeric logic is frozen, with an enumerated permission list.** The relocated code may differ from the host copy only in file location, the plan-type import, `crate::` → `slicer_core::` on ungated helper paths, the error type's name/path, and the NET-NEW `planning_input` adapter (inputs only, no arithmetic). Any sixth difference, or any change to layer iteration order, domain/parameter derivation, grounding, clipping or the `sparse_fill_holder` branch, is a stop — never a retune of the baseline. AC-2/AC-3 prove the move; they do not justify a planning change.
- **Deletion gate.** Steps 10a–13c are forbidden until AC-2 and AC-3 pass.
- **Edit cap (restated to match the steps below).** Ordinary production-source steps edit at most **three** files. Two declared exception classes, each stated in place and never a shim: a single symbol whose removal is atomic across its constructors and readers owns its whole blast radius in one step (11c: six production files; 12b: four; 13b: four), and a test-retarget step may batch up to **six test files**, provided it states per file which contract survives (14a: six, 14b: five, 11d: three). A single integration-test binary that compiles as one unit cannot be half-green, so it is split into two named halves of seven files each with separate verification commands — 14c (six modules plus `main.rs`) and 14d (seven modules) — rather than left as one fourteen-file sweep.
- **Test literals** use FRU/shared fixture bases or a recorded exhaustive waiver; production literals stay exhaustive.
- **Gate sequence across the deletion phase.** Removing a public type or accessor necessarily breaks test targets before the test steps repair them. Steps 11a–13c therefore verify `cargo check --workspace` (library targets of every crate), each test step verifies its own binary, and **`cargo check --workspace --all-targets` is mandatory at Step 16**. A green mid-stream check is never evidence that test targets compile.
- **Execution-order amendment (recorded after Step 7b).** Step 8's consumer half runs **before** Step 7c, as **Step 8a**, and Step 8's remaining half (projection and the AC-N2/N3 retarget) is Step 8b. Reason: AC-2 recovers its population from emitted infill, so while the ordinary stage still reads the paint view the comparison measures the live host producer's output against its own recording and passes vacuously. Switching the consumer first is what makes AC-2's plan comparison mean anything; without it, a 7c "green" would be a false green of exactly the kind D1 warns about. Steps keep their numbers so existing references stay valid.
- **Log discipline.** Archive the previous logs, then successfully remove `target/test-output.log` before every xtask invocation under `set -euo pipefail`.
- **Guest rebuild** is required after any change to a guest fingerprint input (module sources/manifest, WIT, `slicer-core`), before re-running the affected test.

## Step 1 — Record the behavioural baseline

- Task IDs: TASK-577. Objective: capture the live host producer's output as a committed, deterministic oracle before anything moves.
- Precondition: the tree builds; the five Classic case inputs exist under `crates/slicer-wasm-host/tests/fixtures/lightning_gate/`.
- Postcondition: `crates/slicer-runtime/tests/fixtures/lightning_baseline/{baseline_classic.json, manifest.json}` committed: per case the full-identity plan entries with directed edge multisets and grounding/root witnesses, the per-caller segment multisets, and `sha256` of plan and inputs. Two recorder runs byte-identical. The recorder's hashing needs `sha2`, which `crates/slicer-runtime` does not have today (only `crates/slicer-wasm-host` does), so this step adds it as a dev-dependency of `slicer-runtime`.
- Allowed reads: `crates/slicer-runtime/tests/e2e/run_slice_api_tdd.rs::run_slice_against_wedge_returns_nonempty_gcode` and its `SliceRunOptions` construction; the manifest-staging helper in `crates/slicer-runtime/tests/e2e/preparation_runtime_lifecycle_tdd.rs`; `crates/slicer-runtime/src/builtins/lightning_tree_producer.rs` symbol window.
- Allowed edits: NET-NEW `crates/slicer-runtime/tests/e2e/lightning_baseline.rs`; `crates/slicer-runtime/tests/e2e/main.rs`; `crates/slicer-runtime/Cargo.toml` (**dev-dependencies only**: add `sha2 = "0.10"`, the version `crates/slicer-wasm-host/Cargo.toml` already uses); the recorder-generated `crates/slicer-runtime/tests/fixtures/lightning_baseline/` directory (committed verbatim).
- Out of bounds: production source, the `lightning_gate` fixtures, other packet directories.
- Dispatch: confirm the recorder drives the real configured prepass and reads the committed plan; return SUMMARY ≤ 200 words. Inspect the fixture through the decoder.
- Context: M. Authorities: docs04, docs21, docs22.
- Narrow verification: `set -euo pipefail; mkdir -p target; rm -f target/test-output.log; cargo xtask test --summary -p slicer-runtime --test e2e lightning_baseline::record_lightning_baseline -- --ignored --exact >target/lightning-migration-command.log 2>&1; rg -q '^test result: ok\. 1 passed; 0 failed;' target/test-output.log; rg '^test result:' target/test-output.log`
- Exit: recorder executes, both files written, a second run byte-identical. Falsifying exit: a case yields zero entries, a digest is absent, or two runs differ.

## Step 1a — Extend the baseline to exercise non-empty variant chains

- Task IDs: TASK-577. Objective: make the oracle exercise the one identity axis the five fixture cases leave empty.
- Precondition: Step 1 baseline exists and AC-1 passes.
- Postcondition: the recorder's case material paints one top-face triangle of `hole_void_classic`'s first region object with `PaintValue::ToolIndex(1)` under the `material` semantic; the real paint segmentation emits a painted sub-region; the committed baseline records plan entries and caller records carrying `[("material", ToolIndex(1))]` on layers 4–6; `derivation.synthetic_paint` records the derivation; the recorder's caller dedup keys on the **full** `(global_layer_index, object_id, region_id, ordered variant_chain)` identity, not layer/object/region alone (the painted sub-region shares the base object and region id, so the narrower key silently dropped it); two recorder runs are byte-identical; AC-1 passes. The harness authored in Step 2 mirrors the same derivation; the committed `inputs_sha256` is the cross-check that it does.
- Allowed reads: `crates/slicer-runtime/tests/e2e/lightning_baseline.rs`; the gate fixture's `a_source`; the paint-segmentation contract in `crates/slicer-core/src/algos/paint_segmentation/mod.rs` (bounded window only).
- Allowed edits: `crates/slicer-runtime/tests/e2e/lightning_baseline.rs`; the regenerated `crates/slicer-runtime/tests/fixtures/lightning_baseline/` directory.
- Out of bounds: production source, the gate fixture, the `canonical.json` oracle.
- Dispatch: confirm the painted entry exists, the caller record exists, and the two runs hash identically; return FACT.
- Context: S. Authorities: docs21, docs22, docs08.
- Narrow verification: the Step 1 recorder command twice, then the AC-1 command.
- Exit: the baseline carries at least one non-empty ordered chain in both the plan and caller populations, and the two runs are byte-identical. Falsifying exit: the painted entry has no caller record, or the chain is serialized but not compared.

## Step 2 — Register the acceptance harness (red until implemented)

- Task IDs: TASK-577. Objective: land the nine NET-NEW tests with real drivers and independent comparators.
- Precondition: Step 1 baseline and Step 1a painted coverage exist; AC-1 passes.
- Postcondition: `lightning_migration_tdd` registered; AC-1 green; the other eight compile and fail for their stated reason, recorded as RED evidence. The harness's `CaseGeometry` mirrors Step 1a's synthetic paint derivation, and its `inputs_digest` covers the ordered chain that derivation produces.
- Allowed reads: Step 1 helpers; `crates/slicer-runtime/tests/e2e/main.rs`; `crates/slicer-runtime/src/preparation_lifecycle.rs::SliceRuntimeExtensions` window.
- Allowed edits: NET-NEW `crates/slicer-runtime/tests/e2e/lightning_migration_tdd.rs`; `crates/slicer-runtime/tests/e2e/main.rs`. No dev-dependency edit belongs here: Step 1 already added `sha2` to `crates/slicer-runtime/Cargo.toml`, and the harness needs nothing else.
- Out of bounds: production code, module code, the baseline fixture.
- Dispatch: does each test instantiate the real module and compare to the baseline rather than to itself? Return SUMMARY ≤ 200 words.
- Context: M. Authorities: docs22, docs21.
- Narrow verification: the AC-1 command from `packet.spec.md`.
- Exit: AC-1 passes and the other eight execute. Falsifying exit: a test compiles to an empty body or compares adapters to each other.

## Step 3 — Kernel scaffold, plan types and errors

- Task IDs: TASK-577. Objective: create the kernel module inside the consuming artifact and give the plan types their private home.
- Precondition: none beyond the current tree.
- Postcondition: `src/kernel/{mod.rs, plan.rs, error.rs}` exist; `kernel/plan.rs` is the moved home of the two plan types as `KernelPlan`/`PlanEntry` — `pub(crate)`, no `serde`, no `schema_version` field, no schema-version constant (the empty value is `KernelPlan { entries: Vec::new() }`); `KernelError` preserves the current variants. The public copies in `crates/slicer-ir/src/slice_ir.rs` are **not** touched here: they stay until their last consumer (the host algorithm copy, Step 10c) is deleted, so both paths compile side by side during the dual-owner window.
- Allowed reads: `crates/slicer-ir/src/slice_ir.rs` plan-type definitions; `crates/slicer-core/src/algos/lightning/error.rs`; `modules/core-modules/lightning-infill/src/lib.rs` module root.
- Allowed edits: NET-NEW `modules/core-modules/lightning-infill/src/kernel/mod.rs`; NET-NEW `src/kernel/plan.rs`; NET-NEW `src/kernel/error.rs`.
- Out of bounds: the host algorithm (it runs until Step 10c), `slicer-ir`, the consumer path.
- Dispatch: confirm the plan types carry every field the baseline records and no version constant; return FACT.
- Context: S. Authorities: docs02, docs08.
- Narrow verification: `cargo check -p lightning-infill`.
- Exit: the module compiles with the new module declared but unused. Falsifying exit: a recorded field has no home in `PlanEntry`, or the public type was edited here.

## Step 4 — Move the planning algorithm bodies

- Task IDs: TASK-577. Objective: relocate the algorithm into the kernel with no change to planning decisions.
- Precondition: Step 3 plan/error types exist.
- Postcondition: `src/kernel/{tree_node,layer,distance_field}.rs` contain the moved bodies with kernel-relative paths. The host copies remain and still compile, so the Step 1 recorder keeps producing the same baseline.
- Allowed reads: the three host files; `crates/slicer-core/src/algos/lightning/mod.rs` module list.
- Allowed edits: NET-NEW `src/kernel/tree_node.rs`; NET-NEW `src/kernel/layer.rs`; NET-NEW `src/kernel/distance_field.rs`.
- Out of bounds: the host copies, the generator (Step 5).
- Dispatch: diff the moved bodies against the originals and confirm only `use`/type paths changed; return FACT with any non-path difference quoted.
- Context: M. Authorities: docs02.
- Narrow verification: `cargo check -p lightning-infill`.
- Exit: the kernel modules compile standalone. Falsifying exit: a moved body differs beyond path/type adaptation.

## Step 5 — Generator move and kernel entry

- Task IDs: TASK-577. Objective: complete the kernel's planning bodies behind the same entry-point shape the host producer calls.
- Precondition: Step 4 moved bodies exist.
- Postcondition: `src/kernel/generator.rs` holds `Generator::generate_trees`/`committed_segments_for_layer`; `src/kernel/mod.rs` exposes the relocated entry point `generate_lightning_trees(&[SliceIR], &ResolvedConfig) -> Result<Arc<KernelPlan>, KernelError>`, with `SliceIR`/`ResolvedConfig` still imported from `slicer-ir` and the plan types imported from `kernel::plan`. Every `crate::` helper path becomes `slicer_core::` (`polygon_ops::clip_polylines`, `flow::{resolve_role_width, RoleWidthContext}`, `geometry::closest_point_on_segment`, `difference`/`offset`/`OffsetJoinType`) — the module already depends on `slicer-core`. The `sparse_fill_holder` branch is retained verbatim; eligibility is decided by selection before `prepare_print` is reached (D2). Doc comments that named `LightningTreeIR`/`LightningTreeEntry` are rewritten to `KernelPlan`/`PlanEntry` — comment text only, because AC-5 greps this module's `src/` as a public-name leak check.
- Allowed reads: host `generator.rs` and `mod.rs`; `modules/core-modules/lightning-infill/Cargo.toml`.
- Allowed edits: NET-NEW `src/kernel/generator.rs`; `src/kernel/mod.rs`.
- Out of bounds: the host copies, the consumer, the manifest, `planning_input.rs` (Step 5a).
- Dispatch: diff the moved `mod.rs` against the host original and return every non-path, non-import difference quoted; there must be none. Return SUMMARY ≤ 200 words.
- Context: M. Authorities: docs02, docs05, docs08.
- Narrow verification: `cargo check -p lightning-infill`.
- Exit: the kernel compiles with the host signature preserved. Falsifying exit: a statement inside the moved computation differs beyond the enumerated deltas, or `sparse_fill_holder` was dropped.

## Step 5a — Module-side planning-input adapter

- Task IDs: TASK-577. Objective: build the planner's inputs from the typed preparation records, additively and without touching the computation.
- Precondition: Step 5 kernel entry exists.
- Postcondition: `src/planning_input.rs` builds `Vec<SliceIR>` from the `PreparationRegion`s by grouping them on `global_layer_index` — one layer per distinct index, each `SliceIR { global_layer_index, z, ..SliceIR::default() }` — and one `SlicedRegion` per region in that group, built from `SlicedRegion::default()` with `object_id`, `region_id`, `polygons` copied verbatim (`region.polygons.clone()`) and `variant_chain: region.variant_chain.iter().map(|(k, v)| (k.clone(), slicer_ir::PaintValue::from(v))).collect()`, converting `PreparationVariantChain = Vec<(String, PreparationPaintValue)>` element-wise through `slicer_ir::PaintValue`'s `impl From<&PreparationPaintValue>`. The planner reads only `global_layer_index`, `regions` and, per region, `object_id`, `region_id`, `variant_chain`, `polygons`, so every other `SlicedRegion` field keeps its default and `z` is carried unread. The config is `ResolvedConfig { sparse_fill_holder: "lightning-infill".into(), line_width, infill_density, layer_height, infill_angle, ..Default::default() }`, i.e. exactly the five fields the planner reads, sourced from `ConfigView`/`effective_layer_height`. `line_width == 0.0` is the host's **automatic sentinel**, not a width: `ExpansionResolver::expand_key`'s `is_plain_zero` branch (`crates/slicer-config/src/lib.rs`) expands it to `1.125 × nozzle_diameter` before the config is committed, so the kernel must never see `0.0`; a committed value that is still `0.0` means the sentinel survived expansion, and the step stops for an input-contract fix rather than forwarding a zero width. The five scalars must agree across every `PreparationRegionConfig` read and `layer_height` must agree with `PreparationRegion.effective_layer_height`, else preparation fails with `ModuleError` code `57412`, `config disagrees across committed regions: {key}`. `lightning-infill.toml` declares `[config.schema.layer_height]` and `[config.schema.infill_angle]` so `ConfigView` can surface them.
- Allowed reads: `crates/slicer-sdk/src/preparation_input.rs::{PreparationRegion, PreparationRegionConfig, PreparationVariantChain, PreparationPaintValue}` and the `impl From<&PreparationPaintValue> for slicer_ir::PaintValue` beside them; `crates/slicer-ir/src/slice_ir.rs::{SliceIR, SlicedRegion, ConfigView}` — `SliceIR`'s public fields + `impl Default`, `SlicedRegion`'s `derive(Default)`, and `ConfigView::{get_float, require_float}`; `crates/slicer-ir/src/resolved_config.rs` `declare_resolved_config!` field list and its `impl Default`; `crates/slicer-config/src/lib.rs`'s `ExpansionResolver::expand_key` `line_width`/`is_plain_zero` branch.
- Allowed edits: NET-NEW `modules/core-modules/lightning-infill/src/planning_input.rs`; `modules/core-modules/lightning-infill/lightning-infill.toml`; `modules/core-modules/lightning-infill/src/lib.rs` (`mod` declaration only).
- Out of bounds: `src/kernel/**` (Step 5 is closed), the consumer path, the `[preparation]` section (Step 7b).
- Dispatch: list every value the adapter supplies and the typed read it comes from; any value with no typed source is a stop, never a default. Return SUMMARY ≤ 200 words.
- Context: M. Authorities: docs02, docs05, docs08.
- Narrow verification: `cargo check -p lightning-infill`.
- Exit: the adapter compiles and every planner input has a named typed source. Falsifying exit: a scalar is defaulted because its source was not found, or a disagreement is silently resolved by picking one region's value.
- **Correction (Step 7b, extended in Step 8a).** The `layer_height` cross-check against `PreparationRegion.effective_layer_height` was unconditional, but derived regions cannot supply that field: paint segmentation and region split construct `SlicedRegion`s from `SlicedRegion::default()`, so it holds the `f32` default `0.0`. Measured on the painted `hole_void_classic` case through the real preparation path: `57412` with `layer 0 object object-a-region-0 region 0 chain []: config 0.2 region 0`. The check now applies only when the region **carries** a height (`> 0.0`; no layer can be 0 mm, so a non-positive value means "not supplied"), and the planner still multiplies `config.layer_height`. Six regression tests in `planning_input.rs` cover it and the neighbouring failures. **Step 8a then exposed a second, independent assumption:** the adapter required the committed region population and the committed region-config population to be key-identical in *both* directions. They are not. Measured in the same case: layer 0's geometry carries chain `[]` while its region-map key carries `[("material", ToolIndex(1))]`, so a config can name a key with no geometry on that layer, and a region can lack its own config. Since the planner reads one print-wide config, the cross-checks now apply only when a config does name a region, and the surviving hard requirement is "regions exist but no configs at all" (`57412`). The `57412` scalar-agreement contract is unchanged.

## Step 6 — Move the algorithm tests

- Task IDs: TASK-577. Objective: keep the algorithm's unit coverage with its new owner.
- Precondition: Step 5 kernel entry exists (Step 5a is not a test dependency).
- Postcondition: `modules/core-modules/lightning-infill/src/kernel/tests.rs` (a `#[cfg(test)] mod tests;` inside `kernel/mod.rs`) carries the moved assertions adapted to the relocated entry point. **Unit test, not an integration test**: the kernel's plan types and submodules are `pub(crate)`, so `tests/kernel_tdd.rs` could only compile if the private plan types were made public — exactly what D3 forbids. Two assertions are retargeted rather than dropped: the two-run determinism check compares `KernelPlan::entries` structurally instead of `serde_json` bytes (the private plan has no `serde`), and the `schema_version` assertion is retired with the public IR — the private type has no version field, which the compiler enforces. `crates/slicer-core/tests/algo_lightning_tdd.rs` is deleted; its `[[test]]` block in `crates/slicer-core/Cargo.toml` is gone. The gating that kept that test out of a bare `-p slicer-core` run is the `required-features = ["host-algos"]` entry in that `[[test]]` block — the file carries no `#![cfg(feature = …)]` header — so the `[[test]]` entry is the edit, and deleting the file without deleting the entry leaves a Cargo target pointing at nothing.
- Allowed reads: `crates/slicer-core/tests/algo_lightning_tdd.rs`; the kernel entry; `crates/slicer-core/Cargo.toml` `[[test]]` block.
- Allowed edits: NET-NEW `modules/core-modules/lightning-infill/src/kernel/tests.rs`; `modules/core-modules/lightning-infill/src/kernel/mod.rs` (`#[cfg(test)] mod tests;` only); `crates/slicer-core/tests/algo_lightning_tdd.rs` (delete); `crates/slicer-core/Cargo.toml`.
- Out of bounds: the host algorithm sources, the runtime.
- Dispatch: confirm no assertion is dropped except the two named retargets; return moved-vs-original assertion counts.
- Context: S. Authorities: docs22.
- **Gate-scope note**: the moved file lives under `modules/core-modules/*/src`, which the current `check-literals` and `check-test-quality` collectors do not scan — they cover `crates/*/src` in cfg-test mode but only `modules/*/tests`. This is a tooling-scope gap, not a waiver: the assertion population is the previously scanned integration test's, the two retargets are named above, and this step runs `cargo test -p lightning-infill --lib kernel::tests`. Extending the collectors to `modules/*/src` is a separate tooling packet; until then, do not claim the file was gate-scanned.
- Narrow verification: `set -euo pipefail; mkdir -p target; rm -f target/test-output.log; cargo test -p lightning-infill --lib kernel::tests 2>&1 | tee target/test-output.log >/dev/null; rg -q '^test result: ok\.' target/test-output.log; rg '^test result:' target/test-output.log`
- Exit: the moved tests pass under the kernel owner with the original assertion count minus the retired schema-version assertion. Falsifying exit: an assertion count drops without a named retarget, a test becomes `#[ignore]`d, or the plan types are made `pub` to satisfy an integration test.

## Step 7a — Module-private piece codec

- Task IDs: TASK-577. Objective: implement D3's encoder/decoder and error contract, module-local.
- Precondition: Step 3 plan types exist; D3 is authoritative.
- Postcondition: `private_plan` encodes one piece per layer and decodes it with bounds checks; round-trip preserves full identity; malformed → `57410`, unsupported version → `57411`; the inherited `57401` missing-piece path is untouched; no host dependency, no float on the wire.
- Allowed reads: D3 in `design.md`; `src/kernel/plan.rs`; `crates/slicer-sdk/src/preparation.rs::PreparedPiece` window.
- Allowed edits: NET-NEW `modules/core-modules/lightning-infill/src/private_plan.rs`; `modules/core-modules/lightning-infill/src/lib.rs` (`mod` declaration only); `modules/core-modules/lightning-infill/tests/lightning_infill_tdd.rs`.
- Out of bounds: generic SDK/schema/WIT, the kernel internals, the projection.
- Dispatch: bounded-read decode preserves identity and rejects each malformed case with the exact code; return SUMMARY ≤ 200 words.
- Context: M. Authorities: docs08 units, docs21, docs22.
- **Test placement** (decided at implementation; D3 unchanged): the round-trip and malformed/version assertions are a `#[cfg(test)] mod tests` inside `src/private_plan.rs`, not additions to `tests/lightning_infill_tdd.rs`. The round-trip subject is `KernelPlan`, whose fields are `pub(crate)`; an integration test could reach the codec only by making the codec — and therefore the piece format — public, which is the host-visible boundary D3 forbids ("opaque to the host"). The integration-binary allowance above is therefore unused by this step, and the codec's proof is the `--lib` run below. The wire layout is pinned by a hand-written byte literal, not by re-running the encoder.
- **Gate-scope note**: identical to Step 6's. `modules/core-modules/*/src` is outside the current `check-literals` and `check-test-quality` collectors, so this test population is not gate-scanned; do not claim it was. Extending the collectors to `modules/*/src` remains a separate tooling packet.
- Narrow verification: `set -euo pipefail; mkdir -p target; rm -f target/test-output.log; cargo test -p lightning-infill --lib private_plan 2>&1 | tee target/test-output.log >/dev/null; rg -q '^test result: ok\. 12 passed; 0 failed;' target/test-output.log; rm -f target/test-output.log; cargo test -p lightning-infill --test lightning_infill_tdd 2>&1 | tee target/test-output.log >/dev/null; rg -q '^test result: ok\.' target/test-output.log; rg '^test result:' target/test-output.log`
- Exit: round-trip and both error codes assert with literal expected bodies. Falsifying exit: a truncated buffer decodes, or a version mismatch returns `57410`.

## Step 7b — Implement preparation and declare the capability

- Task IDs: TASK-577. Objective: plan from typed committed inputs and publish pieces.
- Precondition: kernel entry exists (Step 5) and its input adapter exists (Step 5a); Step 7a codec exists; baseline available.
- Postcondition: `impl LayerPreparation for LightningInfill` reads regions/configs/targets, builds the planner inputs through `planning_input` (Step 5a), calls the kernel entry, encodes one piece per layer through `private_plan`; manifest declares `[preparation]`; the module compiles into the guest; the **WASM** arm reaches a `Ready` owner with `lightning-layer-*` pieces. The temporary `#[allow(dead_code)]` bridges on `mod kernel`/`mod planning_input` in `src/lib.rs` are removed here: this step is their real consumer. AC-2's native leg is Step 7c, which needs the generated pair this step creates.
- Allowed reads: `crates/slicer-sdk/src/preparation.rs` (`PrintPreparationInput`, `PreparedPlanOutput`); `crates/slicer-wasm-host/src/preparation_input.rs::validate_reads`; an existing preparation-capable test guest manifest.
- Allowed edits: NET-NEW `modules/core-modules/lightning-infill/src/preparation.rs`; `modules/core-modules/lightning-infill/lightning-infill.toml`; `modules/core-modules/lightning-infill/src/lib.rs` (`mod` declarations only).
- Out of bounds: the consumer path (Step 8), the kernel internals, the native-pair export (Step 7c).
- Dispatch: confirm each declared `input_reads` matches the accessor it feeds; on failure return the `PreparationInputError::AccessDenied` (ungranted read) or `MissingRequired { read }` (declared `*_optional` capability with no committed product) name.
- Context: M. Authorities: ADR-0074, ADR-0066 as amended, docs04/05, docs08.
- Narrow verification: `cargo check -p lightning-infill`, then `cargo xtask build-guests --check` (exit 0 required; exit 1 → rebuild → recheck). The AC-2 command is Step 7c's gate: it cannot pass while the harness still lacks the generated native pair, and this step does not pretend otherwise.
- Exit: the module compiles into the guest and the manifest declares the capability. Falsifying exit: the module compiles into no guest, a declared read is unbacked, or the `[preparation]` section is absent from the built artifact's manifest.
- **Implemented (Step 7b), with three corrections to the text above.**
  - **The adapter had to be fixed first.** `planning_input::build_planner_inputs` cross-checked `PreparationRegion.effective_layer_height` against the committed `layer_height` unconditionally, but paint segmentation and region split build their regions from `SlicedRegion::default()`, so that field is the `f32` default `0.0` there. Measured through the real preparation path, `hole_void_classic` failed with `57412` and the message `layer 0 object object-a-region-0 region 0 chain []: config 0.2 region 0`. The check now fires only for a **carried** height (non-positive means "not supplied"); the planner still reads `config.layer_height` in every case. Five regression tests in `planning_input.rs` pin that rule plus the neighbouring failures (a carried-height disagreement, a per-layer disagreement, a cross-region scalar disagreement, a surviving `0.0` line width). This corrects Step 5a's NET-NEW adapter, not planning logic; it is the one edit outside this step's allowed list, and Step 5a is marked corrected in the ledger.
  - **`input_reads` is `["layer_plan", "slices", "region_map"]`, not D2's original list.** Whole-print preparation needs the committed schedule's own layer indices to publish one piece per layer, including layers with no regions, and only `layer_plan` supplies them. `selected_targets` is not declared because nothing in the module reads it — selection decides participation in `PrintPreparationScope::capture_committed_selection`, and declaring an unconsumed read would fail this step's own dispatch rule. D2 is corrected to match.
  - **Two `#[allow(dead_code)]` scopes survive, and neither hides this step's consumer.** `kernel` keeps the host copy's `pub` surface verbatim — two written-only `Generator` fields, `Generator::{get_trees_for_layer, take_committed_segments}`, `DistanceField::unsupported_count`, `Layer::new`, `KernelError::Unimplemented`. Those were `pub` items of a public crate in `crates/slicer-core/src/algos/lightning/` and so never dead; inside this private module the same items are unreachable, and deleting them would be a change to moved code the freeze rule forbids. `private_plan` keeps its decoder alive only through its own tests until the ordinary stage reads pieces in Step 8. Both are recorded as residue, not presented as clean.
  - **Evidence.** The three temporary bridges are gone: preparation is the real consumer of the kernel, the adapter and the codec's encoder, and the WASM arm reaches a `Ready` owner with `lightning-layer-*` pieces on all five baseline cases (measured piece bodies range from the 10-byte header on tree-less layers to 107 251 bytes on `anchor_bridge_classic` layer 6). AC-2 still fails, and only at its Step-2 `B1` blocker — Step 7c's edit. The plan comparison that runs today reads the emitted infill through the still-live host producer, so the module's *piece content* is not yet compared; that becomes real when Step 8 switches the consumer, and this step does not claim it.

## Step 8a — Switch the ordinary consumer to private pieces (executed before 7c)

- Task IDs: TASK-577. Objective: make the ordinary stage read its own layer's piece, so the acceptance comparison measures the module's plan instead of the still-live host producer's output.
- Precondition: Step 7b publishes pieces; the WASM arm reaches a `Ready` owner on all five baseline cases.
- Postcondition: `run_infill` acquires `PreparedPlanView::current()`, requires `lightning-layer-{layer_index}`, decodes that piece through `private_plan`, and emits the same per-region segment multiset with the same role, width and speed factor as before, matched on the full `(object_id, region_id, ordered variant_chain)` identity. The paint view is no longer read for segments; there is no fallback to it. `modules/core-modules/lightning-infill/tests/lightning_infill_tdd.rs` is retargeted in the same edit, because the ordinary stage now requires a plan scope: the file installs encoded pieces through `with_prepared_plan_view`, and its encoder is a **second, independent** implementation of the D3 layout (the module's encoder is `pub(crate)`, so this is forced, and two implementations agreeing is the point).
- Why before 7c: AC-2's population is recovered from emitted infill (`population_from_infill` in `crates/slicer-runtime/tests/e2e/lightning_migration_tdd.rs`). Until this step, that infill comes from the host prepass producer, so the comparison would pass no matter what the module planned.
- Allowed reads: `crates/slicer-sdk/src/preparation.rs` (`PreparedPlanView`, `with_prepared_plan_view`, `PreparedPlanRead`, `PreparedPieceRead`), `src/private_plan.rs`, the current `run_infill` window.
- Allowed edits: `modules/core-modules/lightning-infill/src/lib.rs`; `modules/core-modules/lightning-infill/tests/lightning_infill_tdd.rs`. **Two files.**
- Out of bounds: the projection and harness retarget (8b), the diagnostic key (7d), the host producer (10a/10b), the kernel.
- Dispatch: confirm the emitted segment multiset is unchanged for a chain-less case and that no paint-view segment read remains; return FACT.
- Context: M. Authorities: AC-3 wording, D2, D3, docs22.
- Narrow verification: `cargo test -p lightning-infill --lib`, then `cargo test -p lightning-infill`, then the AC-3 command from `packet.spec.md`.
- Exit: the module suite is green through prepared pieces and AC-3 passes; AC-2 remains blocked at `B1` until 7c. Falsifying exit: a paint-view fallback remains, or the test pieces are produced by the module's own encoder rather than by the independent one.
- **Implemented (Step 8a), with the edit list corrected from two files to four.** The production edit is `run_infill` reading its own piece (`PreparedPlanView::current` → `require` → `private_plan` decode) plus the module's integration test, whose encoder is a second implementation of the D3 layout. Three further edits were forced by what the now-substantive comparison exposed, and each is recorded rather than folded into the production change:
  - `planning_input.rs`: the committed region and region-config populations are not key-identical (measured: a painted config key on a layer whose geometry is chain-less). Recorded as Step 5a's extended correction.
  - The harness's caller lookup matched `(layer, object_id, region_id)` only. The painted case has two callers sharing those three fields, so the lookup compared the painted region against its chain-less sibling — the identity collapse Step 1a extended the baseline to catch. It now matches the full identity including the ordered chain.
  - AC-3 compared the whole recorded caller population against the emitted one. The recorder enumerates every committed slice region (including zero-segment ones), while an emitted region exists only if the stage saw it, so the two populations legitimately differ on empty callers. The comparison now takes the **nonempty** subset on both sides; a caller that should emit nothing but emits something still appears as an unexpected entry, and one that should emit but does not still appears as missing.
  - **Measured result:** AC-3 `1 passed` in 697 s on all five cases, including both painted cases. The plan comparison it runs is now the module's own.

## Step 7c — Export the native prepared pair and take AC-2 green

- Task IDs: TASK-577. Objective: let AC-2's native leg run the module's real paired artifact instead of a surrogate.
- Precondition: Step 7b attached `#[slicer_module(preparation)]`, so `__slicer_prepared_native_entry()` and `__SLICER_PREPARATION_SCHEMA` exist; AC-2's WASM leg has reached a `Ready` owner; **Step 8a has switched the ordinary consumer**, so the harness's infill-derived population now measures the module's plan on both arms instead of the live host producer's.
- Postcondition: `slicer_integrated_modules::prepared_native_entries()` returns `(ModuleId, NativePreparedStageEntry, PreparationExportSchema)` for every enabled integrated module that declares preparation; today that is `com.core.lightning-infill` only. It is `#[cfg(not(target_arch = "wasm32"))]`, gated on the `lightning-infill` feature, and returns an empty vector when the feature is off. The harness's native arm passes `prepared_native_entries()` into `SliceRuntimeExtensions::new` (with integrated modules enabled) and compares the resulting native plan to the recorded baseline; the WASM arm remains the staged artifact with integrated modules disabled. Both are compared to the baseline, never to each other. `preparation_native::NativePreparationArtifact::with_native_plan` already installs the owner-bound plan scope for the paired ordinary entry, which is what lets the consumer read pieces natively.
- Allowed reads: `crates/slicer-integrated-modules/src/lib.rs` (`native_entries()` and the `integrated_registry!` macro window); the harness's `extensions`/`case_options` helpers.
- Allowed edits: `crates/slicer-integrated-modules/src/lib.rs`; `crates/slicer-runtime/tests/e2e/lightning_migration_tdd.rs` (the native-arm construction and the retirement of blocker `B1`).
- Out of bounds: production runtime selection (`run.rs` already accepts the pair through `SliceRuntimeExtensions::prepared_native`), the module's preparation code, the baseline.
- Dispatch: confirm the returned pair's `stage` is the lightning module's real ordinary entry and that its `prepare` is the generated `LayerPreparation::prepare_print` trampoline; return FACT.
- Context: S. Authorities: docs05, docs22, ADR-0074.
- Narrow verification: `cargo check -p slicer-integrated-modules`; then the AC-2 command from `packet.spec.md`, preceded by `cargo xtask build-guests --check` (exit 0 required; exit 1 → rebuild → recheck).
- Exit: AC-2 green with nonempty populations on both adapters, and the painted `hole_void_classic` entries present on both. Falsifying exit: the native arm is built from a hand-constructed entry rather than `prepared_native_entries()`, or the WASM arm is skipped.
- **Implemented (Step 7c).** `slicer_integrated_modules::prepared_native_entries()` returns the module's generated pair, `#[cfg(not(target_arch = "wasm32"))]` and gated on the `lightning-infill` feature (empty otherwise); `slicer-integrated-modules` gained a `slicer-schema` dependency for the schema type's name. The harness's native arm runs with integrated modules enabled and the lightning guest removed from the search roots — integrated registrations are the loader's last tier and lose to any on-disk manifest with the same id, so leaving the directory in would silently run the WASM artifact. Both arms are compared through one shared `assert_arm_matches_baseline`, and the `B1` blocker is deleted. A crate-level test asserts the accessor returns exactly the lightning row, a `NativeStageEntry::Layer`, the real `infill-module` ordinary world, and an id present in `integrated_inventory()`.
- **Measured result:** AC-2 `1 passed` in 645 s — both arms reproduce the recorded baseline for all five cases, with piece-derived populations on each. `cargo test -p slicer-integrated-modules --features lightning-infill` green; workspace `--all-targets` check and clippy `-D warnings` green; guests fresh.
- **Sequencing hazard (resolved).** As first authored, AC-2's population came from emitted infill, which the still-live host producer fed, so a 7c green would have compared that producer to its own recording. The execution-order amendment moved Step 8a (the consumer switch) ahead of this step; both arms now compare the module's own plan.

## Step 7d — Module-owned diagnostic publication mode

- Task IDs: TASK-577. Objective: make the module's own `57401`/`57410`/`57411`/`57412` failure paths reachable through the deployed artifact, without a host-side fake.
- Precondition: Steps 7a/7b applied (codec and preparation exist); D6 is authoritative.
- Postcondition: `lightning-infill.toml` declares the advanced, default-`none` enum `prepared_plan_diagnostic` with values `none`, `missing_piece`, `malformed_piece`, `unsupported_version`, `empty_plan`; `src/preparation.rs` reads it once from the first committed `PreparationRegionConfig.config`, excludes it from the five-scalar agreement check, and applies it only after planning: `missing_piece` omits the topmost planned layer's piece, `malformed_piece` stages a body the decoder rejects with `57410`, `unsupported_version` stages a well-formed header with an unsupported format version (`57411`), and `empty_plan` encodes an empty plan per layer and publishes an empty scene. The `none` path is byte-identical to Step 7b's output; the key never reaches the kernel.
- Allowed reads: D6; `src/preparation.rs`; `src/private_plan.rs`; `lightning-infill.toml`; the `ConfigView` enum accessor.
- Allowed edits: `modules/core-modules/lightning-infill/src/preparation.rs`; `modules/core-modules/lightning-infill/lightning-infill.toml`; `modules/core-modules/lightning-infill/src/lib.rs` (module declarations only, if needed).
- Out of bounds: the kernel, the codec's error mapping (Step 7a), the harness (Step 8 retargets it), any generic SDK/schema/WIT surface.
- Dispatch: list every code path the key can reach and confirm the `none` path cannot; return SUMMARY ≤ 200 words.
- Context: M. Authorities: D6, docs21, docs22.
- Narrow verification: `cargo check -p lightning-infill`; then `cargo test -p lightning-infill --test lightning_infill_tdd` (the codec's round-trip and error tests must stay green with the new key present at its default).
- Exit: the key is declared, inert at `none`, and each non-default value produces its named code through the real publication path. Falsifying exit: the key changes a planning scalar, changes the `none` output, or `empty_plan` is not `Ready`/no-work.
- **Implemented (Step 7d).** `lightning-infill.toml` declares the advanced, default-`none` enum with the five values; `src/preparation.rs` reads it once from the first committed region config and applies it after planning; NET-NEW `src/projection.rs` owns the declared view/scene constants and the explicit empty scene (`classes` declared, `primitives: []`); `src/lib.rs` declares the module. The `none` path still publishes the same pieces and, since the view was not requested, allocates and publishes no scene. Four decisions this step had to settle, recorded rather than left implicit:
  - **`missing_piece` omits the topmost layer that carries plan entries, not the topmost scheduled layer.** The ordinary consumer is gated by `prepared_call_has_available_work` (`crates/slicer-runtime/src/layer_executor.rs`), and a schedule top that is supported and tree-less still has a piece but may never be a caller. The topmost *entry-bearing* layer is guaranteed to be consumed (its region produced plan segments). Measured on a five-layer `10/10/10/12/12` fixture: entries exist on layer 3 only, and the mode omits layer 3 rather than schedule top 4.
  - **An absent key is fatal, not silently `none`.** A committed region config is built from the manifest's declared keys, so an absent `prepared_plan_diagnostic` means the view was never registry-seeded; `require_string` fails preparation with this module's generic code and names the key. Unknown spellings fail the same way.
  - **The malformed and unsupported-version bodies come from the real encoder.** `malformed_piece` encodes the normal layer piece and truncates one byte (every strict prefix is `57410`); `unsupported_version` encodes the empty plan and patches the D3 version field (`bytes 4..6`) to `2`. No magic/version literal is duplicated in `preparation.rs`, and the module tests decode both bodies through the private decoder.
  - **The empty scene is published only when `planning_geometry` was requested.** `PreparedPlanOutput::projection()` → `requested_views()` is checked first, so capture-off runs derive and allocate nothing.
  - Evidence: `cargo test -p lightning-infill` → 55 lib (10 in `preparation::tests`), 10 integration, 1 binding; `cargo clippy -p lightning-infill --all-targets -- -D warnings` exit 0; guest rebuilt (`cargo xtask build-guests --check` exit 0); `cargo xtask gen-config-docs` added the generated `prepared_plan_diagnostic` row to `docs/15_config_keys_reference.md`.


## Step 8b — Typed projection and the acceptance-harness retarget

- Task IDs: TASK-577. Objective: publish `planning_geometry` from the same in-memory plan and retire the harness blockers the production change replaces.
- Precondition: Step 8a applied (the consumer already reads pieces); AC-2 green (7c); the diagnostic mode exists (7d).
- Postcondition: `projection.rs` builds the `lightning-plan` scene from the same in-memory plan; AC-4 green. The acceptance harness is retargeted in the same step: blocker `B1` is retired by Step 7c's native pair, `B3` by Step 7d's diagnostic publication mode, and `B5` by the `empty_plan` scene plus the public `PreparedOwnerStore`/`freeze_native` foreign/disposed control. AC-N2's four codes become four distinct real-adapter failures, AC-N3's `Ready`/no-work and `AccessDenied` legs become real, and no `panic!("BLOCKED …")` remains in the harness. The diagnostic assertions carry the D6 test-quality waiver naming the module-owned decoder/publication surface. The consumer half of the former Step 8 is Step 8a and is already done.
- Allowed reads: `crates/slicer-sdk/src/preparation_visual.rs`; `crates/slicer-runtime/src/preparation_visual_capture.rs::{PreparationViewRequest, PreparationVisualCapture}` and `crates/slicer-runtime/src/run.rs::run_slice_preparation_visual_capture`; current `run_infill` window; `crates/slicer-wasm-host/src/preparation_store.rs` public authority surface.
- Allowed edits: `modules/core-modules/lightning-infill/src/lib.rs`; NET-NEW `modules/core-modules/lightning-infill/src/projection.rs`; `crates/slicer-runtime/tests/e2e/lightning_migration_tdd.rs`. **Three files**, within the ordinary cap: the harness edit is the retarget of the blockers this step's production change retires, not a second implementation.
- Out of bounds: generic projection schema, CLI renderer, runtime capture structs, the accessor retirement, the diagnostic key's definition (Step 7d).
- Dispatch: does the scene derive from the same in-memory plan, and does capture-off preserve semantic output? For each retired blocker, name the real surface that replaces it; return FACT.
- Context: M. Authorities: row 06 contract, docs19, docs08, docs22, D6.
- Narrow verification: the AC-4, AC-N2 and AC-N3 commands from `packet.spec.md` (AC-3 belongs to Step 8a).
- Exit: AC-4 and both negative criteria green; `retained_bytes == 0` after disposal; empty scene succeeds, missing scene names its row-06 error; the four failure codes stay distinct; a disposed owner's piece read is `AccessDenied`. Falsifying exit: capture-off output identical but the scene built by a second planning pass, a failure leg is asserted against a mock rather than the real store/decoder, or a `BLOCKED` panic remains.
- **Implemented (Step 8b).** `projection.rs` derives the scene from the same in-memory plan and the committed planning domains; `preparation.rs` passes both plus the `empty_plan` flag; `lib.rs` already declared the module in 7d. The authored edit list omitted `preparation.rs`; without it the projection would never be invoked. Three production files (`lib.rs` declaration already present, `projection.rs`, `preparation.rs`) plus the harness retarget fit the ordinary cap, and this correction is the only change to the step's file list.
  - **Scene derivation.** `planning_domain` is every committed region's `ExPolygon`, `filled: true`, holes preserved. `tree_branch` is one open two-point polyline per committed edge. The committed plan carries clipped directed edges and no parent/child topology (the same limitation the recorded "grounding/root witnesses" document), so `tree_root` is the root side of each directed run — a vertex with positive directed incidence imbalance (`out - in > 0`, clipped-run starts included) — and `grounding` is that root's closest point on its planning domain, the wall attachment canonical `attach` uses for a seeded tree. The module cannot name the canonical `GroundingLocation` kind from the plan, and `projection.rs` says so rather than inventing one. Primitive ids are identity-derived (`domain:{layer}:{object}:{region}:{chain}:p{n}`, `branch:{identity}:e{n}`, `root|ground:{identity}:n{n}`), never array positions.
  - **The harness's four legs.** AC-4 adds the explicit empty scene (`empty_plan` → `Ready`, four classes, `primitives: []`, no sparse paths on the consumed layers) and retires `B5`. AC-N2 drives `57401`/`57410`/`57411` through real `Layer::Infill` consumers and keeps the real per-object `57412`, asserting each code is distinct and never `host:lightning_tree`; `B3` is retired. AC-N3 adds the `empty_plan` `Ready`/no-work leg and the foreign/disposed `AccessDenied` leg through `PreparedOwnerStore::begin/put/finish/open` with the module's real `freeze_native` pair; `B5` is retired. No `BLOCKED` panic remains.
  - **AC-N1 control corrections.** The reversed-edge control is rejected by the witness stage before the edge multiset (a reversal flips the directed imbalance), so the assertion now accepts the witness rejection while still requiring the case and key. The swapped-identity control was vacuous: it swapped `region_id` between the first two entries of one layer, and both are region `0` on the recorded case, so nothing changed. It now swaps the whole keys of two same-layer entries with **different** edge sets, which is the "same key set, wrong edge partition" shape the criterion names.
  - **AC-N3 trigger correction (measured, not a production change).** The authored trigger — `sparse_fill_holder = "rectilinear-infill"` — is not a no-target scenario. Selection targets are per committed region and claim-agnostic (`build_layer` pushes one per matching slice region; the holder only changes `held_claims`), while `prepared_call_has_available_work` gates the *consumer*, not selection. Measured: the holder override still produced `Preparation` `Start`/`Ready` and `Consumption` `Start` records with piece bytes. A loaded module with any committed region therefore always has a target: an empty mesh is rejected by the layer planner (`no objects provided for layer planning`), and an open surface still slices to per-region targets with empty polygons. The leg now (a) asserts the criterion's `selection.invoke && !selection.targets.is_empty()` directly on `build_layer_selection_index` with a committed schedule and zero slices, and (b) runs a real print with the artifact excluded from the selected roots and asserts zero lifecycle records, real G-code from the remaining modules, and no fabricated `Ready`. The predicate cannot be made false for a loaded module with committed regions today; making it so would be a production selection change outside this packet's scope.
  - **Measured results.** AC-1 `1 passed` 2.73 s; AC-2 `1 passed` 490.38 s; AC-3 `1 passed` 601.81 s; AC-4 `1 passed` 100.35 s; AC-N1 `1 passed` 16.84 s; AC-N2 `1 passed` 64.18 s; AC-N3 `1 passed` 73.60 s. AC-5 and AC-N4 remain red by design until the retirement steps (10a–14) delete the symbols they witness.
  - **Gates.** `cargo check --workspace --all-targets` exit 0; `cargo clippy --workspace --all-targets -- -D warnings` exit 0; `cargo xtask build-guests --check` exit 0; `cargo xtask check-literals` 0 violations; `cargo xtask check-test-quality --report` 0 findings; `cargo xtask check-deviations --check` OK (73 open).


## Step 9 — Retarget module-local tests

- Task IDs: TASK-577. Objective: finish the module's own tests on real prepared pieces without shrinking their population.
- Precondition: Step 8a retargeted `lightning_infill_tdd`; Step 8b green.
- Postcondition: `slicer_module_binding_tdd` asserts the paired export without a tree fixture; `lightning_infill_tdd` keeps the prepared-piece coverage Step 8a gave it, with no success-only replacement.
- Allowed reads: both test files; Step 7/8 helpers.
- Allowed edits: `modules/core-modules/lightning-infill/tests/lightning_infill_tdd.rs`; `modules/core-modules/lightning-infill/tests/slicer_module_binding_tdd.rs`.
- Out of bounds: production module code, other crates' tests.
- Dispatch: for each changed test, name the surviving contract and its asserted population; return SUMMARY ≤ 200 words.
- Context: S. Authorities: docs22, docs21.
- Narrow verification: `set -euo pipefail; mkdir -p target; rm -f target/test-output.log; cargo test -p lightning-infill 2>&1 | tee target/test-output.log >/dev/null; rg -q '^test result: ok\.' target/test-output.log; rg '^test result:' target/test-output.log`
- Exit: all module tests pass with no success-only replacement. Falsifying exit: an assertion count drops without a stated contract change.
- **Implemented (Step 9).** `lightning_infill_tdd` keeps the eight prepared-piece tests Step 8a gave it (plus the two unchanged config tests). `slicer_module_binding_tdd` gains `paired_preparation_export_matches_manifest`, which pins the generated `__SLICER_PREPARATION_SCHEMA` against `slicer_schema::PREPARATION_QUALIFIED_EXPORT` and the literal `slicer:layer-preparation/prepare@1.0.0#prepare-print`, asserts the ordinary world is `infill-module`, and asserts `__slicer_prepared_native_entry().stage` is the module's ordinary `NativeStageEntry::Layer` family. No plan/tree fixture is constructed, and no existing assertion was removed. Evidence: `cargo test -p lightning-infill` → 55 lib, 10 integration, 2 binding, 0 failed.


## Step 10a — Remove the prepass wiring

- Task IDs: TASK-577. Objective: unhook the stage from the configured prepass while its producer still exists. This is the leaf end of the chain: nothing calls into the prepass block.
- Precondition: **AC-2 and AC-3 green.**
- Postcondition: no `run_builtin_stage` block for lightning, no `PrepassExecutionError::LightningTree`, no `"PrePass::LightningTreeGen" => &[…]` arm in `required_slots`, and no `BlackboardPrepassSlot::LightningTreeIR => blackboard.lightning_tree_ir().is_some()` arm in `ensure_stage_prerequisites` — that last arm is the final reader of the getter Step 12a deletes. The executor test and its registration are retired; the producer module still compiles, now unreferenced.
- Allowed reads: `prepass.rs` window; `crates/slicer-runtime/tests/executor/lightning_prepass_tdd.rs`; `tests/executor/main.rs`.
- Allowed edits: `crates/slicer-runtime/src/prepass.rs`; `crates/slicer-runtime/tests/executor/lightning_prepass_tdd.rs`; `crates/slicer-runtime/tests/executor/main.rs`.
- Out of bounds: the producer file (Step 10b), blackboard, scheduler.
- Dispatch: confirm no other configured-prepass path references the stage; return FACT.
- Context: S. Authorities: docs04, docs22.
- Narrow verification: `cargo check -p slicer-runtime --tests` (every runtime test target compiles after the executor test is retired).
- Exit: check passes; the producer is dead code pending deletion. Falsifying exit: the stage is still scheduled anywhere.
- **Implemented (Step 10a).** The `run_builtin_stage` block, `PrepassExecutionError::LightningTree` with its `Display` arm, the `required_slots` arm and the `ensure_stage_prerequisites` arm are gone; `lightning_prepass_tdd` is deleted with its `mod` line. `BlackboardPrepassSlot::LightningTreeIR` outlives the producer until Step 12b, so the exhaustive match keeps one arm `=> false` (nothing schedules the stage, so it is unreachable) rather than reading the getter Step 12a deletes; Step 12b removes the arm with the variant. Measured: `cargo check -p slicer-runtime --tests` exit 0.

## Step 10b — Delete the host producer

- Task IDs: TASK-577. Objective: remove the now-unreferenced producer artifact and its exports.
- Precondition: Step 10a applied.
- Postcondition: `lightning_tree_producer.rs` deleted; `builtins/mod.rs` no longer exports it; the `crates/slicer-runtime/src/lib.rs` re-export and its "not registered" commentary removed.
- Allowed reads: the three files' symbol windows.
- Allowed edits: `crates/slicer-runtime/src/builtins/lightning_tree_producer.rs` (delete); `crates/slicer-runtime/src/builtins/mod.rs`; `crates/slicer-runtime/src/lib.rs`.
- Out of bounds: the host algorithm copy (Step 10c), `slicer-ir`.
- Dispatch: confirm the closure for `commit_lightning_tree_ir_builtin` is empty; return LOCATIONS ≤ 10.
- Context: S. Authorities: ADR-0074, docs01/04.
- Narrow verification: `cargo check --workspace --all-targets`.
- Exit: no producer symbol remains. Falsifying exit: a deprecated alias appears instead of deletion.
- **Implemented (Step 10b), with its narrow verification corrected.** The producer file, its `builtins/mod.rs` export, its `lib.rs` re-export and the "not registered" commentary are deleted. The authored command `cargo check --workspace --all-targets` cannot pass at this point: `crates/slicer-runtime/tests/executor/source_identity_repair_tdd.rs` calls the deleted `commit_lightning_tree_ir_builtin`, and Step 14b owns that file. That is the step's own gate-sequence rule — `--all-targets` is mandatory at Step 16, and the mid-stream steps verify libraries only — so this step verifies `cargo check --workspace`. Measured: libraries exit 0; the only new `--all-targets` failure is that one target, `E0425`, error count 1.

## Step 10c — Delete the host copy of the algorithm

- Task IDs: TASK-577. Objective: make the module the only owner of lightning planning.
- Precondition: Step 10b applied (no host caller remains); Steps 3–6 delivered the kernel.
- Postcondition: `crates/slicer-core/src/algos/lightning/` is deleted; `algos/mod.rs` no longer declares it; `host-algos` keeps only its rayon/voronoi algorithms.
- Allowed reads: `crates/slicer-core/src/algos/mod.rs`; `rg` closure for `algos::lightning`.
- Allowed edits: the directory delete; `crates/slicer-core/src/algos/mod.rs`.
- Out of bounds: the module kernel, `slicer-sdk`.
- Dispatch: confirm the closure returns no remaining caller; return LOCATIONS ≤ 10.
- Context: S. Authorities: docs01.
- Narrow verification: `cargo check --workspace --all-targets`.
- Exit: the workspace compiles with no host lightning algorithm. Falsifying exit: a `pub use` shim re-exports the deleted path.
- **Implemented (Step 10c).** `crates/slicer-core/src/algos/lightning/` (six files) and its `algos/mod.rs` declaration are deleted; `host-algos` keeps its rayon/voronoi algorithms and is otherwise untouched. Measured: `cargo check --workspace` exit 0, the same single pre-existing `--all-targets` failure as Step 10b (no new one), and no `algos::lightning` hit anywhere in `crates/slicer-core/src`.

## Step 11a — Remove the SDK accessor

- Task IDs: TASK-577. Objective: delete the guest-facing query method whose only production caller was switched in Step 8.
- Precondition: Step 8 applied; the module no longer calls it.
- Postcondition: `PaintRegionLayerView::lightning_tree_segments_for` is gone; `with_lightning_tree_ir`/the `lightning_tree_ir` field remain for now because `marshal/native.rs` still uses them (Step 11c removes the field, both builders and both `marshal/native.rs` uses together).
- Allowed reads: `crates/slicer-sdk/src/traits.rs` accessor window; `rg` closure for `lightning_tree_segments_for`.
- Allowed edits: `crates/slicer-sdk/src/traits.rs`.
- Out of bounds: the field and its builders (atomic with Step 11b).
- Dispatch: confirm zero remaining callers before editing; return FACT.
- Context: S. Authorities: docs05.
- Narrow verification: `cargo check --workspace`.
- Exit: check passes with the accessor deleted. Falsifying exit: an empty-vector forwarding stub is left behind.
- **Implemented (Step 11a).** `lightning_tree_segments_for` is deleted from `PaintRegionLayerView`; `with_lightning_tree_ir`, the field and the `lightning_tree_ir()` getter survive until Step 11c's atomic removal. No production caller remained — the module stopped reading the accessor in Step 8a. Measured: `cargo check --workspace` exit 0.

## Step 11b — Retire the macro glue, watch entry and linker key

- Task IDs: TASK-577. Objective: remove generated-code references before the SDK field and the contract package disappear.
- Precondition: Step 11a applied.
- Postcondition: the macro no longer emits the lightning-taking form of `__slicer_adapt_paint_layer` (`crates/slicer-macros/src/lib.rs::build_layer_infill_glue`, the only caller of `PaintRegionLayerView::with_lightning_tree_ir` outside `marshal/native.rs`) nor `__slicer_lightning_tree_from_view`; the generated `run` signature no longer takes `lightning_queries: LightningQueryView` and its `layer_per_stage_aliases` list no longer imports that resource, so removing the WIT import in Step 13a leaves no dangling parameter; `LIGHTNING_QUERIES_WIT` and its `nest_dep` branch are gone; `build.rs` no longer watches the package; `preparation_bindings.rs` has no linker key for it.
- Allowed reads: the three files' windows; `rg` closure for the glue symbols.
- Allowed edits: `crates/slicer-macros/src/lib.rs`; `crates/slicer-macros/build.rs`; `crates/slicer-wasm-host/src/preparation_bindings.rs`.
- Out of bounds: the WIT package file (Step 13a), `slicer-sdk`.
- Dispatch: confirm no generated path still references the removed symbols; return LOCATIONS ≤ 20.
- Context: S. Authorities: docs03, docs05.
- Narrow verification: `cargo check --workspace`.
- Exit: check passes with no glue emitter and no `with_lightning_tree_ir` caller outside `marshal/native.rs`. Falsifying exit: the glue is kept behind a `cfg` to keep a binding alive.
- **Implemented (Step 11b).** The lightning-taking `__slicer_adapt_paint_layer` form, `__slicer_lightning_tree_from_view`, the generated `run`'s `lightning_queries` parameter with its alias-list import, `LIGHTNING_QUERIES_WIT` and its `nest_dep` branch, the `build.rs` watch entry and the `preparation_bindings.rs` linker key are deleted; the ordinary paint-layer adaptation is unchanged. Measured: `cargo check --workspace` exit 0. Three `slicer:lightning-queries` strings remained in `crates/slicer-wasm-host/src/host.rs` (a doc comment, the runtime instance key and a world comment) — Step 11c's `LightningQueryViewData` removal, not residue of this step. Guests reported stale (exit 1, 48 guests), exactly as this step predicts: the macro is a guest build input and the WIT package is not deleted until Step 13a, so no rebuild is attempted here.

## Step 11c — Remove the stage-input field and its whole blast radius

- Task IDs: TASK-577. Objective: remove `LayerStageInput::lightning_tree_ir`, the host query data it fed, the SDK field, and every constructor/reader, in one transition.
- Precondition: Step 11b applied (the macro no longer calls it); Steps 10a–10c removed the producer side.
- Postcondition: the field, its two `layer_executor.rs` constructors, its two `marshal/native.rs` uses, the `dispatch.rs` parameters/builders including the legacy test helper, the `host.rs` `LightningQueryViewData`/handler, and the `PaintRegionLayerView::with_lightning_tree_ir`/field pair are all gone. No empty-list handler survives.
- Allowed reads: the six files' windows; `rg` closure for `lightning_tree_ir` and `with_lightning_tree_ir`.
- Allowed edits: `crates/slicer-wasm-host/src/binding.rs`; `crates/slicer-runtime/src/layer_executor.rs`; `crates/slicer-wasm-host/src/marshal/native.rs`; `crates/slicer-wasm-host/src/dispatch.rs`; `crates/slicer-wasm-host/src/host.rs`; `crates/slicer-sdk/src/traits.rs`. **Six-file step by design** (the largest of the packet's three declared atomic blast radii): the field cannot be removed without every initializer and reader, and the packet forbids a transitional shim. Per file, what survives: `binding.rs` keeps the stage binding and WIT-keyed entry points minus the tree field; `layer_executor.rs` keeps stage construction minus the tree input at both constructors; `marshal/native.rs` keeps native marshalling minus the native attachment; `dispatch.rs` keeps per-region dispatch minus the query parameter (and retires the legacy test helper with it); `host.rs` keeps the host services minus the `LightningQueryViewData`/handler pair; `traits.rs` keeps `PaintRegionLayerView` and its region accessors minus the retired accessor and field. Test-literal sites are Steps 14a–14f.
- Out of bounds: the WIT files (Step 13a), `slicer-ir`, test sources (Steps 14a–14f).
- Dispatch: enumerate every site before editing and confirm each is in the edit list; any site outside it stops the step.
- Context: M. Authorities: docs03, docs04, docs05.
- Narrow verification: `cargo check --workspace`.
- Exit: check passes with no field, no handler and no shim. Falsifying exit: a `None`-returning accessor persists.
- **Implemented (Step 11c), with a compile-forced ordering correction to Step 13a.** All six files are edited as declared: the `binding.rs` field, both `layer_executor.rs` constructions, both `marshal/native.rs` uses, the `dispatch.rs` parameter/builders and legacy helper, the `host.rs` `LightningQueryViewData`/handler/bindgen key, and the `traits.rs` field/accessor/builder pair are gone. Measured: `cargo check -p slicer-sdk` exit 0; `crates/slicer-wasm-host` failed with five WIT-boundary errors only — `E0277` at `add_to_linker` (`Host` unsatisfied for the deleted package), `E0061` at `call_run` (8 arguments supplied, 7 declared) and two shifted-argument `E0308`. That is not residue of this step: removing the handler while the world still declares the import cannot type-check. Step 13a's canonical-WIT deletion therefore ran next and `cargo check --workspace` returned 0 (see Step 13a). No shim was added; a marked temporary probe that restored the boundary was compiled once to prove no non-WIT error remained, then reverted with no deliverable change.

## Step 11d — Retarget the test guest and its two contract tests

- Task IDs: TASK-577. Objective: keep the export/import linkage proof without the retired query.
- Precondition: Step 11c applied.
- Postcondition: `crates/slicer-wasm-host/test-guests/layer-infill-guest/src/lib.rs` imports no `LightningQueryView`; `wit_boundary_tdd` and `preparation_pilot_host` assert the prepared path or drop only accessor-specific assertions.
- Allowed reads: the three files.
- Allowed edits: the test guest `src/lib.rs`; `crates/slicer-wasm-host/tests/contract/wit_boundary_tdd.rs`; `crates/slicer-wasm-host/tests/contract/preparation_pilot_host.rs`.
- Out of bounds: production wasm-host code, the real `lightning-infill` module.
- Dispatch: confirm each surviving test names the contract it protects; return SUMMARY ≤ 200 words.
- Context: S. Authorities: docs03, docs22.
- Narrow verification: `cargo check --workspace` (the wasm-host contract binary stays red until Step 14d, by design).
- Exit: the guest imports no deleted package and the workspace libraries compile. Falsifying exit: the guest still imports a deleted package while the check passes for another reason.
- **Implemented (Step 11d).** The test guest imports no `LightningQueryView`, its export takes no query parameter, and its lightning witness strings are gone; `wit_boundary_tdd`'s six `.push_lightning_query_view(...)` calls and its `LightningQueryViewData` import are removed; `preparation_pilot_host`'s linker key, its `add_to_linker` entry and its push call are gone. Every other assertion survives — the export/import linkage proof now runs through the prepared query surface. Measured: no `lightning` hit in the three files; the contract binary stays red until Step 14d, as this step predicts.

## Step 12a — Retire the Blackboard storage and the plan IR

- Task IDs: TASK-577. Objective: remove the public plan IR and its runtime storage, now that nothing reads them.
- Precondition: Steps 11a–11d applied, **and Steps 10b/10c applied** — `crates/slicer-core/src/algos/lightning/mod.rs` returns `Arc<LightningTreeIR>` and is the last consumer of the public types. Deleting them while it exists is compile-fatal, so this step follows the host copy rather than preceding it.
- Postcondition: `blackboard.rs` has no lightning field/commit/getter/byte estimate; `slice_ir.rs` has no `LightningTreeIR`/`LightningTreeEntry`/version const; `slicer-ir` re-exports gone. `stage_io.rs` still carries the slot/error names, which hold no type reference and are removed in Step 12b.
- Allowed reads: the three files' windows plus an `rg` closure for the four symbols.
- Allowed edits: `crates/slicer-runtime/src/blackboard.rs`; `crates/slicer-ir/src/lib.rs`; `crates/slicer-ir/src/slice_ir.rs`.
- Out of bounds: `stage_io.rs` (Step 12b), the module kernel.
- Dispatch: re-derive the remaining-reference closure before editing; return LOCATIONS ≤ 20.
- Context: M. Authorities: docs02, docs21.
- Narrow verification: `cargo check --workspace`.
- Exit: check passes. Falsifying exit: a `#[deprecated]` struct or compat alias survives.
- **Implemented (Step 12a).** The blackboard field, its initialization, its byte accounting, `commit_lightning_tree_ir`, the getter and `estimated_lightning_tree_ir_bytes` are gone, as are `LightningTreeIR`/`LightningTreeEntry`/`CURRENT_LIGHTNING_TREE_IR_SCHEMA_VERSION`, their re-exports and the schema-version test module that only asserted the deleted constant. Measured: `cargo check --workspace` exit 0; `cargo test -p slicer-ir --lib --no-run` exit 0; `cargo test -p slicer-runtime --lib --no-run` exit 0.

## Step 12b — Retire the stage identity from slots and the scheduler

- Task IDs: TASK-577. Objective: remove the stage name everywhere it is enumerated, in one transition.
- Precondition: Step 12a applied.
- Postcondition: `stage_io.rs` has no `BlackboardPrepassSlot::LightningTreeIR`, display arm or `BlackboardError::LightningTreeGeneration`; `STAGE_ORDER` has no `PrePass::LightningTreeGen`; both scheduler tests that assert its membership (`stage_order_tdd`, `stage_list_consistency_tdd`'s `HOST_ONLY_STAGES`) are updated in the same step.
- Allowed reads: the four files' windows.
- Allowed edits: `crates/slicer-ir/src/stage_io.rs`; `crates/slicer-scheduler/src/execution_plan.rs`; `crates/slicer-scheduler/tests/stage_order_tdd.rs`; `crates/slicer-scheduler/tests/contract/stage_list_consistency_tdd.rs`. **Four-file step by design**: the roster test asserts the stage's presence, so it cannot be deferred.
- Out of bounds: blackboard, wasm-host.
- Dispatch: confirm no other stage list derives the entry; return LOCATIONS ≤ 10.
- Context: M. Authorities: docs04.
- Narrow verification: `set -euo pipefail; mkdir -p target; rm -f target/test-output.log; cargo test -p slicer-scheduler 2>&1 | tee target/test-output.log >/dev/null; rg -q '^test result: ok\.' target/test-output.log; rg '^test result:' target/test-output.log`
- Exit: scheduler tests pass and the stage is absent from product code. Falsifying exit: a hand-maintained roster still lists it.
- **Implemented (Step 12b), with a compile-forced fifth file.** `BlackboardPrepassSlot::LightningTreeIR` with its `Display` arm, and `BlackboardError::LightningTreeGeneration` with its arm, are deleted; `STAGE_ORDER` loses the entry and both scheduler rosters are updated. **Fifth file:** `crates/slicer-runtime/src/prepass.rs` still carried the `=> false` arm Step 10a left for exhaustiveness, and deleting the variant makes that arm an unknown-variant error, so the arm is removed in the same transition — no wildcard arm was added. Measured: `cargo check --workspace` exit 0; `cargo test -p slicer-scheduler` green across 10 binaries; `cargo test -p slicer-runtime --lib --no-run` and `cargo test -p slicer-ir --lib --no-run` exit 0; guests rebuilt (48) with `--check` exit 0; `check-literals` 0 violations and `check-test-quality --report` no findings in the touched files. **Operational note:** the scheduler's `dag_cli_integration` guard rejects a `target/debug/pnp_cli.exe` older than `crates/*/src/**`, so a narrow `-p slicer-scheduler` run after source edits needs `cargo build -p pnp-cli` first — the first run failed 10 guard checks, not code checks.

## Step 13a — Delete the WIT package and its imports

- Task IDs: TASK-577. Objective: remove the contract surface the accessor lived in.
- Precondition: Steps 11b–11d applied (no `with:` key, no handler, no guest import).
- Postcondition: `crates/slicer-schema/wit/deps/lightning-queries/` is gone; `layer-infill.wit` and the dragon-curve mirror no longer import it; the world resolves.
- Allowed reads: both `layer-infill.wit` files; the package file; `crates/slicer-schema/wit/deps/layer-preparation/layer-preparation.wit` (its `prepared-infill-module` includes `layer-infill`).
- Allowed edits: delete `crates/slicer-schema/wit/deps/lightning-queries/lightning-queries.wit`; `crates/slicer-schema/wit/deps/layer-infill/layer-infill.wit`; `modules/community-modules/dragon-curve/wit/layer-infill.wit`.
- Out of bounds: `xtask` expectations (Step 13c), the module's manifest.
- Dispatch: run `cargo xtask build-guests --check`; exit 0 fresh, 1 stale → rebuild → recheck, 3 infrastructure (never clean). Return the exit code.
- Context: S. Authorities: docs03, AGENTS.md guest freshness.
- Narrow verification: `cargo xtask build-guests --check`, then `cargo check --workspace` (the wasm-host contract binary is knowingly red until Step 14d).
- Exit: the world compiles and guests are fresh. Falsifying exit: a hollow interface or package is retained to keep a binding alive.
- **Implemented (Step 13a), executed one position early.** The package directory, the `use`/`run`-parameter/world-`import` trio in `crates/slicer-schema/wit/deps/layer-infill/layer-infill.wit`, and the same trio in the dragon-curve mirror are deleted. **Ordering correction:** this step ran immediately after Step 11c, because 11c's exit condition was unreachable while the world still declared the package (measured `E0277`/`E0061`). The compile-forced deletion order `design.md` already states for the plan IR applies equally to the WIT world. Measured: `cargo check --workspace` exit 0; `cargo check -p slicer-macros` exit 0 (proves Step 11b removed the `include_str!` rather than leaving it dangling); `build-guests --check` exit 1 → rebuild → exit 0, 49 artifacts; `cargo check -p slicer-wasm-host --test contract` red in exactly the 13 modules Steps 14c/14d own, with zero WIT or bindgen errors.

## Step 13b — Retire the example module's vendored copy and regenerate its bindings

- Task IDs: TASK-577. Objective: keep the committed dragon-curve example consistent.
- Precondition: Step 13a applied.
- Postcondition: the vendored package copy is gone; `Makefile` no longer lists it as wit-bindgen input; the glue templates no longer reference it; generated `interface/` output is regenerated by the module's documented `make bindings`, never hand-edited.
- Allowed reads: `modules/community-modules/dragon-curve/{Makefile, src/glue/main.mbt.in, src/glue/moon.pkg.json.in}`.
- Allowed edits: delete the vendored `wit/deps/lightning-queries/`; `Makefile`; `src/glue/main.mbt.in`; `src/glue/moon.pkg.json.in`. **Four-file step by design**: the generator reads all four as one input set.
- Out of bounds: the historical probe under `docs/feasibility-probes/`; generated output as a hand edit.
- Dispatch: confirm the regenerated tree contains no `lightning-queries` reference.
- Context: S. Authorities: docs03, docs14.
- Narrow verification: `rg lightning-queries modules/community-modules/dragon-curve` must return **no match**, and `cargo check --workspace` passes.
- Exit: no reference remains in the example. Falsifying exit: generated files are hand-edited instead of regenerated.
- **Implemented (Step 13b), with one recorded extension.** The vendored package directory, the `Makefile` wit-bindgen input, the `main.mbt.in` parameter/binding and its `.drop()` uses, and the `moon.pkg.json.in` import entry are deleted; `gen/`, `interface/` and `world/` were regenerated by `make bindings`, not hand-edited, and generated output is byte-identical to the edited templates. **Extension:** the committed `dragon-curve.wasm` still embedded the retired package (measured: 10 occurrences), so it was rebuilt with the module's own `make component` — it is a tracked artifact and this step's objective is a consistent example, not a consistent source tree. Measured: `rg lightning-queries modules/community-modules/dragon-curve` (including `--no-ignore`) exit 1 before the wasm rebuild and after it; the wasm shrank from 52 802 to 51 942 bytes and is still a valid component (`wasm-tools component wit` exit 0); `cargo xtask build-guests --check` exit 0. **Tooling note:** GNU make on this Windows host runs recipes through `cmd.exe`, where the Makefile's `#` lines are not comments, so `make bindings` exits 2 after wit-bindgen itself succeeds; the documented command works unchanged as `make bindings SHELL="C:/Program Files/Git/bin/bash.exe"`. The Makefile was not edited to work around a host-shell issue.

## Step 13c — Retire the verifier expectations that hardcode the package

- Task IDs: TASK-577. Objective: make the WIT verifier and its snapshot tests agree with the delivered tree.
- Precondition: Steps 13a/13b applied.
- Postcondition: `SHARED_PACKAGES` (a `[&str; 7]` literal) no longer lists the package and its length constant matches; `preparation_pilot_expected_wit_set`, the include-str assertion, the watch assertion and its difference array no longer name the package path; the two tests that read the live package off disk through `canonical_package_decl` (`stage_resolves_from_embedded_package_name`, `specialist_imports_do_not_ambiguate_stage_resolution`) and the `SHARED_PACKAGES` membership assertion are retargeted to the surviving set; the dragon-curve snapshot pair and the runtime drift test no longer assert the method. **Out of scope in that file:** the synthetic WIT text inside `specialist_query_packages_use_subset_direction_in_compare` is a self-contained verifier fixture, not the live package, and there is no per-method "verified inventory" in `xtask/src/wit_verify.rs` to retire — only the package entry and the live-file reads.
- Allowed reads: `xtask/src/wit_verify.rs` windows around each listed expectation; the two test files.
- Allowed edits: `xtask/src/wit_verify.rs`; `crates/slicer-schema/tests/dragon_curve_wit_snapshot_tdd.rs`; `crates/slicer-runtime/tests/contract/wit_drift_detection_tdd.rs`.
- Out of bounds: the real WIT files (already done), the synthetic verifier fixtures, other crates.
- Dispatch: list every site that names the live package and confirm each is retargeted or removed with its contract retired; return LOCATIONS ≤ 20.
- Context: M. Authorities: docs03, docs22.
- Narrow verification: `cargo test -p xtask` with tee, then `rg '^test result:'`; plus `cargo test -p slicer-schema` for the snapshot pair.
- Exit: verifier tests pass against the real tree. Falsifying exit: an assertion is deleted while the invariant it protected still exists.
- **Implemented (Step 13c), with the synthetic-fixture note corrected.** `SHARED_PACKAGES` loses the entry and its `[&str; 7]` length; the expected-set helper, the include-str set, the watch set and the difference array lose the path; the two live-file readers and the membership assertion are retargeted to the surviving set; the dragon-curve snapshot and the runtime drift test no longer assert the retired method. **Correction:** the step's claim that the synthetic WIT text inside `specialist_query_packages_use_subset_direction_in_compare` is self-contained and out of scope is false — that fixture feeds `compare_worlds`' `allowed` set, so the deleted package surfaced as `UnexpectedPackage` and the extra-method control would never have reached interface comparison. The fixture's specialist package is retargeted to the surviving `slicer:support-queries`; the omission and extra-method controls keep their exact claims and `compare_worlds`' classification was not weakened. Measured: `cargo test -p xtask` 186 passed / 0 failed; `cargo test -p slicer-schema` green across four binaries; `cargo check --workspace` exit 0; `check-literals` 0 violations; no `lightning-queries` hit remains in the four scanned paths.

## Step 14a — Retarget the runtime contract binary

- Task IDs: TASK-577. Objective: repair one test **binary** at a time. `cargo test --test contract` compiles every module in that binary, so a binary is the smallest unit that can go green.
- Precondition: Steps 12–13 applied.
- Postcondition: the runtime contract binary compiles and passes: round-trip tests retired as scaffolding, `integrated_parity_lightning_infill_tdd` asserting against the prepared path, `wit_drift_detection_tdd` no longer asserting the removed method, `tests/contract/main.rs` listing exactly the files on disk, and `tests/common/mod.rs` free of the tree literal its `mod common` declaration pulls in.
- Allowed reads: the six files below.
- Allowed edits: `crates/slicer-runtime/tests/contract/integrated_parity_lightning_infill_tdd.rs`; `crates/slicer-runtime/tests/contract/lightning_tree_per_region_roundtrip_tdd.rs`; `crates/slicer-runtime/tests/contract/lightning_tree_view_roundtrip_tdd.rs`; `crates/slicer-runtime/tests/contract/wit_drift_detection_tdd.rs`; `crates/slicer-runtime/tests/contract/main.rs`; `crates/slicer-runtime/tests/common/mod.rs`. **Six-file step by design**: the binary compiles as a unit, and `main.rs` pulls `common` in with it, so deferring any of these leaves the step's own command red. Per file, the contract that survives: `integrated_parity_lightning_infill_tdd` keeps the native/WASM structural-parity assertion population (its subject survives, retargeted); `lightning_tree_per_region_roundtrip_tdd` and `lightning_tree_view_roundtrip_tdd` are retired as scaffolding whose only subject was the retired transport; `wit_drift_detection_tdd` keeps drift detection minus the removed method; `main.rs` keeps exactly the on-disk module list; `tests/common/mod.rs` keeps the shared helper minus the tree literal its `mod common` declaration pulls in.
- Out of bounds: production runtime, the other test binaries.
- Dispatch: confirm no parity assertion is weakened by the move and no identity check degrades to a count; return SUMMARY ≤ 200 words.
- Context: M. Authorities: docs22, docs02, ADR-0073.
- Narrow verification: `set -euo pipefail; mkdir -p target; rm -f target/test-output.log; cargo test -p slicer-runtime --test contract 2>&1 | tee target/test-output.log >/dev/null; rg -q '^test result: ok\.' target/test-output.log; rg '^test result:' target/test-output.log`
- Exit: the binary passes with a nonzero count. Falsifying exit: a parity test injects a pre-committed piece payload, or a test survives only as `#[ignore]`.
- **Implemented (Step 14a).** `integrated_parity_lightning_infill_tdd` drives the real prepared module path and keeps its native/WASM structural-parity population; `lightning_tree_per_region_roundtrip_tdd` and `lightning_tree_view_roundtrip_tdd` are deleted as scaffolding with their `mod` lines (their subject was the retired transport); `tests/common/mod.rs` loses the tree literal; `main.rs` matches disk. Measured: `cargo test -p slicer-runtime --test contract` → `308 passed; 0 failed`, exit 0.

## Step 14b — Retarget the runtime executor and integration binaries

- Task IDs: TASK-577. Objective: repair the other two runtime test binaries and the shared helper they use.
- Precondition: Step 14a applied.
- Postcondition: `--test executor` and `--test integration` compile and pass: `lightning_pipeline_linked_tdd` retargeted to the prepared path (its chaining property survives — see the implemented note), `source_identity_repair_tdd` asserting identity through the prepared path, `infill_partitioned_input_tdd` constructing no tree, and both `main.rs` lists matching disk.
- Allowed reads: the five files below.
- Allowed edits: `crates/slicer-runtime/tests/executor/lightning_pipeline_linked_tdd.rs`; `crates/slicer-runtime/tests/executor/source_identity_repair_tdd.rs`; `crates/slicer-runtime/tests/executor/main.rs`; `crates/slicer-runtime/tests/integration/infill_partitioned_input_tdd.rs`; `crates/slicer-runtime/tests/integration/main.rs`. **Five-file step by design**, same reason as 14a.
- Out of bounds: production code, wasm-host tests.
- Dispatch: confirm source-identity assertions survive as identity, not length, checks; return SUMMARY ≤ 200 words.
- Context: M. Authorities: docs22, docs02/04.
- Narrow verification: `set -euo pipefail; mkdir -p target; rm -f target/test-output.log; cargo test -p slicer-runtime --test executor 2>&1 | tee target/test-output.log >/dev/null; rg -q '^test result: ok\.' target/test-output.log; cargo test -p slicer-runtime --test integration 2>&1 | tee -a target/test-output.log >/dev/null; rg -q '^test result: ok\.' target/test-output.log; rg '^test result:' target/test-output.log`
- Exit: both binaries pass with nonzero counts. Falsifying exit: a source-identity assertion degrades to a length check.
- **Implemented (Step 14b), with two recorded corrections.**
  - **The five files are handled** (`lightning_pipeline_linked_tdd` restored from HEAD and retargeted — see Correction 1b, `source_identity_repair_tdd` retargeted to the prepared path, both `main.rs` lists match disk, `infill_partitioned_input_tdd` constructs no tree), and all four `source_identity_repair_tdd` tests pass.
  - **Correction 1 — the `ac9` fixture, and the linker defect it exposes.** The retarget stages the planner's `SliceIR` from `region.polygons()`, the committed sliced cross-section. Provenance is verified: `SlicedRegion.polygons` is set once at `PrePass::Slice` (`crates/slicer-core/src/algos/prepass_slice.rs`, `execute_prepass_slice_single_layer_impl`) and `sync_perimeter_infill_areas_into_slice` neither clears nor filters it, so a layer whose four fill partitions are all empty still hands the planner a non-empty outline. With that faithful input, `ac9_all_four_polygons_empty_yields_zero_paths_no_panic` is **red**: `[lightning-infill] expected 0 paths when all four polygons empty; got 48`. Cause: `infill-linker`'s `RoleBoundaries::for_role` falls back to the union outline whenever all four partitions are empty (`is_partitioned()` false), so the lightning paths are never clipped away. The defect is **pre-existing and reachable in the pre-migration pipeline** (the host producer planned from the same cross-section); this test passed at HEAD only because it called the module directly and never the linker. It is recorded as a follow-up, not fixed here — the principled fix is an explicit "partitions declared" signal carried through `PerimeterRegionView`, the WIT resource `ir-types.wit`'s `perimeter-region-view`, the host `PerimeterRegionData`, `marshal/native.rs`, `dispatch.rs` and the macro glue. An intermediate attempt to make the test green by staging one region per non-empty partition was **reverted**: it pre-clipped in the kernel, removed the deliberate overshoot that `ac7`/`ac7b`/`ac8` use to prove linker clipping, and silenced a live defect. The assertion text is unchanged. Measured final state: `infill_partitioned_input_tdd` 6 passed / 1 failed (ac9 only); `ac7`/`ac7b`/`ac8` green with the overshoot restored.
  - **Correction 1b — the chaining test is retargeted, not retired.** `lightning_pipeline_linked_tdd` was deleted as "scaffolding", but its asserted property survives: raw two-point lightning segments must be chained by `infill-linker` into longer paths (`mean_points > 2.0` over the committed sparse population), all `SparseInfill`. It is restored from HEAD, retargeted to the prepared path (`prepare_lightning` plus `with_native_plan`), re-registered in `executor/main.rs`, and green (`1 passed`). Its `StageApplyContext` literal carries the documented `// exhaustive:` waiver, and `check-literals` reports 0 violations.
  - **Correction 2 — the executor binary has 19 pre-existing failures.** `cargo test -p slicer-runtime --test executor` reports 217 passed / 19 failed, all in `cube_4color_paint_tdd` (12), `cube_fuzzy_painted_tdd` (5) and `paint_channel_consumer_paths_tdd` (2). Every one asserts that `SlicedRegion.variant_chain` is non-empty after `execute_paint_segmentation`, while the tests hand-build a chain-less `RegionMapIR` and the segmentation kernel retains paint layers only for axes the map declares (`paint_segmentation/mod.rs`'s `declared_by_object` gate, introduced by packet 05). Measured: those three test files and the paint kernel are byte-identical to HEAD (`git diff --stat HEAD` empty) and reference no deleted lightning symbol. Step 14b's own retargets are green except `ac9` (Correction 1); the binary's remaining redness is recorded, not masked, and is not attributable to this packet.

## Step 14c — Retarget the wasm-host contract binary, half 1: the retired query call sites

- Task IDs: TASK-577. Objective: repair the wasm-host contract modules whose subject is the retired `lightning-tree-segments` accessor/dispatch surface, plus the binary's registration, in the half where the legacy identifier actually appears.
- Precondition: Steps 11c/11d applied.
- Postcondition: `authored_coloring_grant_and_strip_tdd`, `infill_holder_resolution_painted_region_tdd`, `lightning_dispatch_per_region_keying_tdd`, `lightning_infill_guest_calls_lightning_tree_segments_tdd`, `region_eligibility_tdd` and `shared_selection_dispatch_tdd` assert through the prepared path and construct no tree literal; `main.rs` lists exactly the files on disk. Per file, the contract that survives: authored coloring keeps its grant/strip coverage; holder resolution keeps SparseInfill holder selection; dispatch keying keeps per-region key identity; the guest-calls test keeps the guest→host export/import linkage proof through the prepared query; region eligibility keeps the selection predicate (`invoke && !targets.is_empty()`); shared selection keeps override precedence. The binary itself cannot compile until 14d repairs the other seven modules, so this step's command proves the half structurally plus the still-green production library, and 14d owns the binary-green gate.
- Allowed reads: the seven files below.
- Allowed edits: `crates/slicer-wasm-host/tests/contract/{authored_coloring_grant_and_strip_tdd, infill_holder_resolution_painted_region_tdd, lightning_dispatch_per_region_keying_tdd, lightning_infill_guest_calls_lightning_tree_segments_tdd, region_eligibility_tdd, shared_selection_dispatch_tdd}.rs` and `crates/slicer-wasm-host/tests/contract/main.rs`. **Seven files — six test modules plus the binary's registry.** The binary compiles as one unit, so this half is defined by the retired identifier's call sites, not by a file count; the fourteen-file sweep is exactly what the 14c/14d split avoids.
- Out of bounds: production wasm-host code, the other seven contract modules (Step 14d), the standalone `raft_plan_read_accessor_tdd` target (Step 14e).
- Dispatch: for each module, name the retained contract and asserted population; return SUMMARY ≤ 200 words.
- Context: M. Authorities: docs22, docs03, ADR-0073.
- Narrow verification: `set -euo pipefail; files=(crates/slicer-wasm-host/tests/contract/authored_coloring_grant_and_strip_tdd.rs crates/slicer-wasm-host/tests/contract/infill_holder_resolution_painted_region_tdd.rs crates/slicer-wasm-host/tests/contract/lightning_dispatch_per_region_keying_tdd.rs crates/slicer-wasm-host/tests/contract/lightning_infill_guest_calls_lightning_tree_segments_tdd.rs crates/slicer-wasm-host/tests/contract/region_eligibility_tdd.rs crates/slicer-wasm-host/tests/contract/shared_selection_dispatch_tdd.rs crates/slicer-wasm-host/tests/contract/main.rs); cargo check -p slicer-wasm-host --lib; for f in "${files[@]}"; do test -f "$f"; done; test -z "$(rg -l -e 'LightningTreeIR' -e 'LightningTreeEntry' -e 'lightning_tree_segments_for' -e 'with_lightning_tree_ir' -e 'lightning_tree_ir' -e 'LightningQueryViewData' -e 'lightning-tree-segments' -e 'lightning-queries' "${files[@]}" || true)"`
- Exit: the production library compiles and none of the half's seven files names a retired symbol. Falsifying exit: a module is emptied while its contract still exists elsewhere, or a file is missing from the declared list (the `test -f` loop fails closed).
- **Implemented (Step 14c), with a fixture follow-up.** The six modules lose every deleted field/tree construction; `lightning_dispatch_per_region_keying_tdd` asserts per-region **key identity** through the surviving surface; the guest-calls module keeps its export/import linkage proof through the prepared query; `main.rs` matches disk. Structural gate passed (seven files exist, no retired symbol, production lib exit 0). The first whole-binary run then reported **189 passed / 5 failed**, all five in this half and all `MissingModelConfig`/`None` fixture failures caused by setup the tree literal had incidentally supplied; the follow-up restored the `RegionMapIR` config authority and sparse-fill holder, and corrected the merged-role lookup to source-space `raw_ids[0]`. Measured after the follow-up: `cargo test -p slicer-wasm-host --test contract` → `194 passed; 0 failed`.

## Step 14d — Retarget the wasm-host contract binary, half 2: identity, parity and view-seam modules

- Task IDs: TASK-577. Objective: repair the remaining seven modules of the same binary and take it green as a unit.
- Precondition: Step 14c applied.
- Postcondition: `infill_postprocess_view_identity_tdd`, `sdk_view_transport_parity_tdd`, `source_identity_repair_tdd`, `support_identity_layer_dispatch_tdd`, `variant_identity_accessors_tdd`, `variant_identity_delivery_tdd` and `view_seam_identity_tdd` keep their identity/parity/view-seam populations through the prepared path, and no module constructs a tree literal. Per file, the contract that survives: postprocess-view identity keeps the view-seam identity map; SDK transport parity keeps native↔WASM parity; `source_identity_repair_tdd` keeps full-identity repair; support dispatch keeps support identity; the two variant-identity modules keep ordered `variant_chain` identity and delivery; view-seam keeps seam identity. With both halves applied the binary is green here.
- Allowed reads: the seven files below.
- Allowed edits: `crates/slicer-wasm-host/tests/contract/{infill_postprocess_view_identity_tdd, sdk_view_transport_parity_tdd, source_identity_repair_tdd, support_identity_layer_dispatch_tdd, variant_identity_accessors_tdd, variant_identity_delivery_tdd, view_seam_identity_tdd}.rs`. **Seven-file half** — the second half of the indivisible binary 14c/14d split, with its own named file list and its own command.
- Out of bounds: production wasm-host code, the 14c half, the standalone raft target (Step 14e).
- Dispatch: for each module, name the retained contract and asserted population; return SUMMARY ≤ 300 words.
- Context: M. Authorities: docs22, docs03, ADR-0073.
- Narrow verification: `set -euo pipefail; mkdir -p target; rm -f target/test-output.log; cargo test -p slicer-wasm-host --test contract 2>&1 | tee target/test-output.log >/dev/null; rg -q '^test result: ok\.' target/test-output.log; rg '^test result:' target/test-output.log`
- Exit: the binary passes with all thirteen retargeted modules and `main.rs`. Falsifying exit: a module is emptied while its contract still exists elsewhere, or an identity assertion degrades to a count.
- **Implemented (Step 14d).** The seven modules lose the deleted literals and imports; `source_identity_repair_tdd` retires its tree fixture with its subject and keeps full-identity repair; `view_seam_identity_tdd` drops the accessor comparison that has no surviving subject. Measured as part of the whole-binary run above: all seven pass, and the binary is green at 194 passed / 0 failed once the Step 14c fixture follow-up lands.

## Step 14e — Retarget the standalone raft accessor target

- Task IDs: TASK-577. Objective: repair the one auto-discovered wasm-host test target that is not part of `--test contract`.
- Precondition: Step 14d applied.
- Postcondition: `crates/slicer-wasm-host/tests/raft_plan_read_accessor_tdd.rs` constructs no tree input and keeps its accessor assertions against the surviving surface.
- Allowed reads: that file.
- Allowed edits: `crates/slicer-wasm-host/tests/raft_plan_read_accessor_tdd.rs`.
- Out of bounds: production code, the contract binary.
- Dispatch: name the contract the file protects after the change; return FACT.
- Context: S. Authorities: docs22.
- Narrow verification: `set -euo pipefail; mkdir -p target; rm -f target/test-output.log; cargo test -p slicer-wasm-host --test raft_plan_read_accessor_tdd 2>&1 | tee target/test-output.log >/dev/null; rg -q '^test result: ok\.' target/test-output.log; rg '^test result:' target/test-output.log`
- Exit: the target passes. Falsifying exit: the file is deleted while its accessor contract still exists.
- **Implemented (Step 14e).** Both retired literals removed; the accessor assertions are kept. Measured: `cargo test -p slicer-wasm-host --test raft_plan_read_accessor_tdd` → `3 passed; 0 failed`.

## Step 14f — Prove the test tree is clean

- Task IDs: TASK-577. Objective: close the test phase with a whole-tree check rather than a per-file claim.
- Precondition: Steps 14a–14e applied.
- Postcondition: `cargo check --workspace --all-targets` passes and the `rg` closure for the four legacy symbols over `crates/**/tests/**` and `modules/**/tests/**` returns zero hits **outside the migration harness**, which names them by design: `lightning_migration_tdd.rs` and `lightning_baseline.rs` must be excluded, along with the committed baseline fixture and the row-07 `lightning_gate` input material.
- Allowed reads: the closure command; nothing else.
- Allowed edits: **none** except the recorder retirement below. Any other remaining hit is a defect in Steps 14a–14e, returned to the owning step.
- **Correction (measured at Step 11b).** Retire the recorder here: delete `crates/slicer-runtime/tests/e2e/lightning_baseline.rs` and its `mod` line in `crates/slicer-runtime/tests/e2e/main.rs`. The recorder drove the **live host producer** to write the committed oracle, and Step 10b deleted that producer, so the file cannot reach `--all-targets` green (measured: `E0599` at its `with_lightning_tree_ir` call site). Retargeting it to the module path would make the oracle compare the module to itself — the vacuity D1 forbids. The committed fixture and its `derivation` block remain the oracle, and AC-1 never calls this module (it has no `pub` item, so no sibling test can reach it; its body validates the committed bytes, and its own header states the producer is not re-run). Re-recording is a property of the pre-migration tree, not a capability this packet claims to keep. The AC-5 and 14f globs that name the deleted file become harmless dead globs.
- Out of bounds: every source file.
- Dispatch: return the closure result and the check exit as FACT.
- Context: S. Authorities: docs22, docs21.
- Narrow verification: `set -euo pipefail; cargo check --workspace --all-targets; test -z "$(rg -l -e 'LightningTreeIR' -e 'LightningTreeEntry' -e 'PrePass::LightningTreeGen' -e 'lightning_tree_segments_for' -e 'lightning_tree_ir' -e 'with_lightning_tree_ir' -e 'LightningQueryViewData' -e 'lightning-queries' --glob 'crates/**/tests/**' --glob 'modules/**/tests/**' --glob '!crates/slicer-runtime/tests/e2e/lightning_migration_tdd.rs' --glob '!crates/slicer-runtime/tests/e2e/lightning_baseline.rs' --glob '!crates/slicer-runtime/tests/fixtures/lightning_baseline/**' --glob '!crates/slicer-wasm-host/tests/fixtures/lightning_gate/**' || true)"`
- Exit: both green. Falsifying exit: an `#[ignore]`d test keeps a reference compiling, or a witness file is excluded for any reason other than being the absence harness itself.
- **Implemented (Step 14f).** The recorder is deleted with its registration (see the correction above; it was an untracked new file, so `git status` shows no deletion entry). Measured: `cargo check --workspace --all-targets` exit 0; the closure `rg` over `crates/**/tests/**` and `modules/**/tests/**` returns no hit outside the migration harness, the committed fixtures and the row-07 gate material. `check-test-quality --report` returns 0 findings and scans 0 files for these paths — the known collector-scope gap recorded at Steps 6/7a, not a clean bill of health.
- **Recorded limitation (review, 2026-10-22).** Because the recorder was retired together with the producer it drove, its two-run byte-identity and its pre-deletion AC-2/AC-3 passes are **not independently re-verifiable from this tree**. AC-1 is an artifact validator over the committed bytes, not a producer re-run, and a future audit needs the pre-migration tree (`348366ed`) plus a re-authored recorder. This packet's equivalence evidence is therefore the committed fixture and its digests, not a reproducible capture procedure; re-deriving the capture is future work, not a capability this packet ships. The AC-2 input digest carries a second, narrower limit recorded with this one: it hashes the override map the run is handed, so a defect inside the adapter's resolution of those already-supplied scalars would not move it — the module publishes no resolved-config view. Named here rather than left implicit so no later reader mistakes AC-1's green for a determinism proof.

## Step 15a — Replace live legacy guidance (architecture, IR, WIT)

- Task IDs: TASK-577. Objective: make the docs describe the migrated contract.
- Precondition: Steps 10–14 applied and green.
- Postcondition: `docs/01_system_architecture.md` carries `### Lightning private preparation migration`; `docs/02_ir_schemas.md` carries `### Lightning legacy IR retirement`; `docs/03_wit_and_manifest.md` carries `### Lightning accessor retirement`, including the note that `docs/feasibility-probes/` is a stale historical mirror outside the live contract. Each heading exists exactly once and its section contains the sentence `The legacy path is retired.` verbatim — describing the retirement is the point, so the section legitimately names the retired symbols; AC-5's absence greps cover code and WIT only.
- Allowed reads: the current lightning sections of those docs.
- Allowed edits: those three files.
- Out of bounds: historical packet prose, ADR records (amended during authoring), `docs/feasibility-probes/**`.
- Dispatch: confirm each heading exists exactly once and its section states the legacy path is retired; return FACT.
- Context: S. Authorities: docs00 map, ADR-0074, ADR-0066/0029 as amended.
- Narrow verification: the documentation-placement bullet marked "Step 15a" in `packet.spec.md` `## Verification`.
- Exit: the grep passes. Falsifying exit: the heading is added while the old text asserting the legacy path stays.
- **Implemented (Step 15a).** `docs/01`, `02` and `03` carry their replacement headings exactly once, each section states `The legacy path is retired.` verbatim, and the legacy IR/WIT guidance is replaced rather than left standing beside the new text. `docs/03` also records that `docs/feasibility-probes/` is a stale historical mirror outside the live contract. Measured: the exact Step 15a placement loop exits 0; `rg -c '^### Lightning'` returns 1 for each of the three files.

## Step 15b — Replace live legacy guidance (scheduler, SDK, visual debug)

- Task IDs: TASK-577. Objective: same for the remaining three docs.
- Precondition: Step 15a applied.
- Postcondition: `docs/04_host_scheduler.md` carries `### Lightning selection-driven preparation`; `docs/05_module_sdk.md` carries `### Lightning prepared module consumption`, including the generated `prepared_native_entries()` export and the advanced, default-`none` `prepared_plan_diagnostic` key with its explicit statement that the key never reaches the kernel; `docs/19_visual_debug.md` carries `### Lightning private preparation migration` describing the `planning_geometry` view and `lightning-plan` scene. As in 15a, each heading exists exactly once and its section contains `The legacy path is retired.` verbatim.
- Allowed reads: the current lightning sections of those docs.
- Allowed edits: those three files.
- Out of bounds: as Step 15a.
- Dispatch: same placement check; return FACT.
- Context: S. Authorities: docs04/05/19.
- Narrow verification: the documentation-placement bullet marked "Step 15b" in `packet.spec.md` `## Verification`.
- Exit: greps pass. Falsifying exit: the visual-debug section documents a renderer that does not exist.
- **Implemented (Step 15b).** `docs/04`, `05` and `19` carry their replacement headings exactly once with the retirement sentence verbatim. `docs/05` names `prepared_native_entries()` and the advanced, default-`none` `prepared_plan_diagnostic` key with its "never reaches the kernel" statement; `docs/19` documents the `planning_geometry` view and the `lightning-plan` scene, including the declared empty scene, and describes no renderer that does not exist. Measured: the exact Step 15b placement loop exits 0; `rg -c '^### Lightning'` returns 1 for each of the three files.

## Step 16 — Closure and ledger reconciliation

- Task IDs: TASK-577. Objective: verify the whole packet and record the re-scope truthfully.
- Precondition: every prior step green; guests fresh.
- Postcondition: nine AC commands plus the aggregate `9 passed` pass; `cargo check --workspace --all-targets` and `clippy --all-targets` clean (the packet-scope re-run of the check Step 14f first ran for the test tree); `check-literals` and `check-test-quality --report` findings resolved or waived with a named surface; the source plan's **existing** `**Row 08 re-scope (2026-10-20).**` note is confirmed accurate and re-derived (not re-added), the blocking paragraph that follows it agrees with it, and the `docs/07` `TASK-577` line states the equivalence premise and cites ADR-0074 and its two deviation rows. The `docs/07` `TASK-300` line's reference to the moved lightning test file is retargeted to `modules/core-modules/lightning-infill/src/kernel/tests.rs` — a path correction only; its open/blocked status and its finding are unchanged. The `docs/07` open-deviations block is **generated**, so it is refreshed with `cargo xtask check-deviations` (the owning command) and never hand-edited; only the `TASK-577` and `TASK-300` lines are authored in place. Every ledger value is re-derived at edit time.
- Allowed reads: `packet.spec.md` verification section; the two ledger windows.
- Allowed edits: `docs/specs/layer-module-preparation-plan.md`; `docs/07_implementation_status.md`; `packet.spec.md` (status only, on explicit finalization approval).
- Out of bounds: other packets, the ADR-0074 text and the ADR amendments (recorded during authoring), gate outcomes for rows 07/09.
- Dispatch: run each AC command through an execution worker; return FACT pass/fail with bounded failure text.
- Context: M. Authorities: `packet.spec.md`, ADR-0074, docs21/22.
- Narrow verification: the `## Verification` list in `packet.spec.md`.
- Exit: everything green and both ledger lines state the equivalence premise without claiming parity. Falsifying exit: a ledger line implies Gate A/B now pass, the row-08 note is duplicated instead of confirmed, or the packet claims canonical parity.
- **Implemented (Step 16).** Nine migration ACs green (AC-1–AC-5, AC-N1–AC-N4), aggregate `9 passed; 0 failed`; workspace `check --all-targets` and `clippy --all-targets -- -D warnings` exit 0, guests fresh (`build-guests --check` exit 0), `check-literals` 0 violations, `check-test-quality --report` 0 findings (the noted module test paths remain outside its scan scope), and `check-deviations --check` OK. Ledgers state recorded behavioural equivalence, not canonical parity; Gate A/B outcomes and packet `status: draft` are unchanged pending explicit finalization approval. Recorded `ac9_all_four_polygons_empty_yields_zero_paths_no_panic` red (expected 0 paths, got 48) exposed the pre-existing empty-partition union fallback in `RoleBoundaries::{is_partitioned, for_role}` (`modules/core-modules/infill-linker/src/orchestrate.rs`); the final state of that module is 6 passed / 1 failed, and no production fallback fix is claimed. `git diff` confirms `Cargo.lock` changes only add Step 1's `sha2` dev-dependency and Step 7c's `slicer-schema` dependency to their package dependency lists: mechanical Cargo resolution, never a hand-edited lockfile step.

## Budget and Completion

| Steps | Context | Scope |
| --- | --- | --- |
| 1, 1a, 2 | M | Baseline, painted-chain coverage, harness |
| 3, 4, 5 | S/M | Kernel scaffold, algorithm move, generator + entry point |
| 5a | M | Module-side planning-input adapter (typed reads → planner inputs) |
| 6, 7a | S/M | Kernel tests, piece codec |
| 7b, 7c, 7d | M | Preparation, native-pair export, diagnostic publication mode |
| 8, 9 | M | Consumers/projection, module tests |
| 10a–10c | S | Prepass wiring, producer, host algorithm copy |
| 11a–11d | M | SDK accessor, stage-input field blast radius, macros, test guest |
| 12a, 12b | M | Plan IR, stage identity |
| 13a–13c | S/M | WIT package, example module, verifier expectations |
| 14a–14f | M | Test fallout |
| 15a, 15b | S | Docs |
| 16 | M | Closure + ledger |

No step is rated L. The edit cap is stated honestly rather than uniformly: ordinary production-source steps edit at most three files; a single symbol's atomic removal owns its declared blast radius (11c: six production files; 12b and 13b: four each); a test-retarget step batches at most six test files and names per file the contract that survives (14a: six, 14b: five, 11d: three); and the wasm-host `--test contract` binary, which compiles as one unit, is split into two named seven-file halves (14c: six modules plus `main.rs`; 14d: seven modules) with separate verification commands instead of one fourteen-file step. Each case says so in place; a transitional shim is never the alternative. The step count follows the retirement's real breadth — a public IR, a scheduler stage, a WIT package with hardcoded verifier expectations, and roughly thirty test files — plus the three mechanisms Step 2 exposed: 1a's painted oracle coverage, 7c's native-pair export, and 7d's module-owned diagnostic publication mode. Step 8's third file is the harness retarget of the blockers those mechanisms retire, so it stays within the ordinary three-file cap rather than becoming a second test-retarget batch. Packet completion requires every AC, the negative controls, guest freshness and an independent closure review.
