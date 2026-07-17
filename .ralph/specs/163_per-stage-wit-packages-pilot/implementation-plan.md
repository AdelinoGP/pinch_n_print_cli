# Implementation Plan: 163_per-stage-wit-packages-pilot

## Execution Rules

- Work one atomic step at a time; map every step to grouped task IDs.
- Use TDD, then implementation, then the narrowest falsifying validation.
- Every field below is a context-budget contract and must be filled independently; never write "see Step 1".
- **The tree does not compile between Step 1 and Step 6.** Six layers of one contract move together. No step before Step 6 lists a `cargo check` exit; a red `cargo check` at Steps 1-5 is expected, not a defect, and must not be "fixed" by editing files outside the step's edit list.
- **Prerequisite:** `162_wit-lifecycle-export-removal` must be **implemented and merged** before Step 1. Verify first: `rg -q 'on_print_start|on_print_end' crates/slicer-sdk/src/traits.rs && echo 'STOP: 162 has not landed' || echo 'ok: post-162 baseline'`.

## Steps

### Step 1: Author the three per-stage WIT packages; delete the two tier worlds

- Task IDs: `TASK-146b`
- Objective: create `crates/slicer-schema/wit/deps/{postpass-gcode-postprocess,postpass-text-postprocess,finalization-layer-finalization}/<same>.wit`, each `package slicer:<dir>@1.0.0;` with an exported interface holding exactly one `run: func` and — for the two resource-bearing stages — an imported `<iface>-types` interface holding every `resource` and plain type moved verbatim from the tier world body. Delete `deps/world-postpass/` and `deps/world-finalization/`.
- Precondition: 162 is merged; `deps/world-postpass/world-postpass.wit` (51 lines) and `deps/world-finalization/world-finalization.wit` (118 lines) exist as read in `design.md` §"Read-Only Context".
- Postcondition: three package files exist matching `design.md` §"Selected approach" verbatim; zero `resource` declarations inside any exported interface; both tier world dirs are gone.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-schema/wit/deps/world-postpass/world-postpass.wit` - whole (51 lines)
  - `crates/slicer-schema/wit/deps/world-finalization/world-finalization.wit` - whole (118 lines)
  - `crates/slicer-schema/wit/deps/common.wit` - whole - the in-tree proof an interface cannot carry a version
- Files allowed to edit (at most 3):
  - `crates/slicer-schema/wit/deps/postpass-gcode-postprocess/postpass-gcode-postprocess.wit` (new)
  - `crates/slicer-schema/wit/deps/postpass-text-postprocess/postpass-text-postprocess.wit` (new)
  - `crates/slicer-schema/wit/deps/finalization-layer-finalization/finalization-layer-finalization.wit` (new)
  - (plus the deletion of the two `deps/world-{postpass,finalization}/` directories — a removal, not an edit)
- Files explicitly out of bounds:
  - `crates/slicer-schema/wit/deps/world-layer/`, `crates/slicer-schema/wit/deps/world-prepass/` - packet #3's contracts
  - `crates/slicer-schema/wit/deps/{types,config,ir-types,common}.wit`, `crates/slicer-schema/wit/root.wit` - shared deps, unversioned, unchanged
  - all `.rs` - later steps
- Expected sub-agent dispatches:
  - none — both source files are small enough to read directly.
- Context cost: `S`
- Authoritative docs:
  - `docs/adr/0045-per-stage-versioned-interfaces-over-monolithic-tier-worlds.md` §"Decision", §"Versions reset to 1.0.0", §"Verified empirically, not just read" - direct read
- OrcaSlicer refs:
  - none - this packet has no OrcaSlicer surface (see `design.md` §"Controlling Code Paths")
- Verification:
  - AC-1's command - FACT PASS/FAIL
- Exit condition: AC-1 prints `PASS`. Do **not** run `cargo check` — nothing else has moved yet and it will be red.

### Step 2: Extend `slicer_schema::STAGES` with the package columns and lookups

- Task IDs: `TASK-146b`
- Objective: add `wit_dir`, `wit_package`, `wit_interface`, `wit_world` to `StageSpec`; fill all 16 rows (`wit_dir` total; the other three empty for the 13 unmigrated stages); set the three pilot rows' `wit_export` to `"run"`; add `package_for_stage_id`, `interface_for_stage_id`, `wit_world_for_stage_id`, `wit_dir_for_stage_id`, `qualified_export_for_stage_id`, each a `STAGES.iter().find(...)` per ADR-0006. Extend `mod tests::stage_and_world_lookups_are_consistent` to assert `wit_dir` non-empty for all 16 rows and that a non-empty `wit_package` implies non-empty `wit_interface` + `wit_world`.
- Precondition: Step 1 complete; the three package/interface/world names exist on disk to copy literally.
- Postcondition: exactly one `pub const STAGES`; `WORLD_LAYER/PREPASS/FINALIZATION/POSTPASS`, `SUPPORTED_WIT_WORLDS`, `StageSpec.world_id`, `ExportKind`, `ExportBinding`, `SlicerModuleSchema` all byte-unchanged.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-schema/src/lib.rs` - whole (435 lines)
  - `crates/slicer-schema/tests/export_for_stage_id_tdd.rs` - whole (~30 lines)
- Files allowed to edit (at most 3):
  - `crates/slicer-schema/src/lib.rs`
  - `crates/slicer-schema/tests/export_for_stage_id_tdd.rs`
- Files explicitly out of bounds:
  - `crates/slicer-scheduler/src/manifest.rs`, `validate_wit_world` - packet #3
  - every consumer of `export_for_stage_id` outside `slicer-schema` - Steps 4-8 own their own fallout
- Expected sub-agent dispatches:
  - none.
- Context cost: `S`
- Authoritative docs:
  - `docs/adr/0006-export-for-stage-id-sole-lookup.md` - direct read; extend the table, never parallel it
- OrcaSlicer refs:
  - none.
- Verification:
  - `cargo test -p slicer-schema 2>&1 | rg '^test result'` - FACT pass/fail (unfiltered; prints `0 filtered out`)
  - AC-3's `STAGES`-table-count command - FACT PASS/FAIL
- Exit condition: AC-2 and AC-3 both print `PASS`. `cargo check --workspace` is still expected red.

### Step 3: Split the host `bindgen!` mods and repoint the `Host*` impls

- Task IDs: `TASK-146b`
- Objective: replace `pub mod postpass` and `pub mod finalization` (and their `pub use postpass::PostpassModule;` / `pub use finalization::FinalizationModule;`) with `pub mod postpass_gcode`, `pub mod postpass_text`, `pub mod finalization_layer`, each `bindgen!({ path: "../slicer-schema/wit", world: "slicer:<pkg>/<world>", imports: { default: trappable }, with: { …the same five dep alias keys… } })`, re-exporting `GcodePostprocessModule`, `TextPostprocessModule`, `LayerFinalizationModule`. Repoint `mod postpass_impls` (`use super::postpass as ppm;`) and `mod finalization_impls` at the new mods.
- Precondition: Steps 1-2 complete.
- Postcondition: exactly five `bindgen!` invocations in `host.rs`; the moved resources stay **unmapped** by `with:` (`postpass_impls` keeps `Resource<ppm::GcodeOutputBuilder>`); `PostpassGcodeOutputBuilderData`, `LayerCollectionViewData`, `FinalizationOutputBuilderData`, `WitEntityMutation`, `WitSortKey`, `FinalizationBuilderPush` unchanged.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-wasm-host/src/host.rs` - lines `306-330` (the `layer` mod and its `with:` block — the alias target), `574-626` (the two mods being replaced), `3797-3830` (`postpass_impls` head), `3277-3300` (`finalization_impls` head) only. **Never read whole (4100 lines).**
- Files allowed to edit (at most 3):
  - `crates/slicer-wasm-host/src/host.rs`
- Files explicitly out of bounds:
  - `crates/slicer-wasm-host/src/dispatch.rs` - Step 4
  - `crates/slicer-wasm-host/src/host.rs` lines `1964-3105` (layer role/region impls) - untouched by the split
- Expected sub-agent dispatches:
  - Question: within `crates/slicer-wasm-host/src/host.rs`, list every `file:line` referencing `host::postpass`, `host::finalization`, `super::postpass`, or `super::finalization`; scope: `crates/slicer-wasm-host/src/host.rs`; return: `LOCATIONS` (≤20 entries); purpose: find every repoint site without reading the file.
- Context cost: `M`
- Authoritative docs:
  - `docs/adr/0002-wit-marshalling-type-unification.md` - direct read; the `with:` remap contract
  - `docs/adr/0003-macro-per-world-wit-conversions.md` - direct read; why guests cannot share the way the host does
- OrcaSlicer refs:
  - none.
- Verification:
  - AC-4's command - FACT PASS/FAIL
- Exit condition: AC-4 prints `PASS`. `cargo check -p slicer-wasm-host` is still expected red (dispatch not yet moved).

### Step 4: Repoint the three dispatch runners and enrich the miss diagnostic

- Task IDs: `TASK-146b`
- Objective: retarget `retract_mode_to_postpass_wit` and `convert_gcode_command_to_postpass_wit` from `host::postpass::*` to `host::postpass_gcode::*`; in `dispatch_postpass_gcode_call`, `dispatch_postpass_text_call` and `dispatch_finalization_call`, swap `host::PostpassModule::instantiate` / `host::FinalizationModule::instantiate` for the stage world's `instantiate` and call through the generated interface accessor; extend each existing `DispatchPhase::TypedInstantiation` arm's `reason` with `slicer_schema::qualified_export_for_stage_id(stage_id)`.
- Precondition: Step 3 complete; `host::postpass_gcode` / `postpass_text` / `finalization_layer` exist.
- Postcondition: zero occurrences of `host::PostpassModule` / `host::FinalizationModule` in `dispatch.rs`; no new error type (`DispatchError` already carries `module_id`, `stage_id`, `export_name`, `phase`); the `reason` keeps wasmtime's own text (`no exported instance named …`) — **do not** decode the guest's actual exports or assert a "found @x.y.z" fragment; the engine supplies expected-only.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-wasm-host/src/dispatch.rs` - lines `95-180` (converters), `861-1000` (finalization runner head), `992-1170` (gcode runner), `1169-1300` (text runner) only. **Never read whole (2536 lines).**
- Files allowed to edit (at most 3):
  - `crates/slicer-wasm-host/src/dispatch.rs`
- Files explicitly out of bounds:
  - `crates/slicer-wasm-host/src/dispatch.rs` lines `246-860` (layer + prepass runners) - packet #3
  - `crates/slicer-macros/src/lib.rs` - Step 5
- Expected sub-agent dispatches:
  - none — the four ranges are known and bounded.
- Context cost: `M`
- Authoritative docs:
  - `docs/adr/0045-per-stage-versioned-interfaces-over-monolithic-tier-worlds.md` §"Decision", §"Verified empirically, not just read" - direct read; the no-probing + fatal-on-miss rule and the measured diagnostic wording
  - `docs/adr/0015-prepass-export-normalization.md` - delegated `SUMMARY` (≤200 words); the "do not swallow" rule
- OrcaSlicer refs:
  - none.
- Verification:
  - AC-5's `host::PostpassModule|host::FinalizationModule` count command - FACT PASS/FAIL
- Exit condition: the count command prints `PASS`. Tests are not runnable yet — the macro still emits tier-world glue.

### Step 5: Split the macro glue per stage and delete the padding arms

- Task IDs: `TASK-146b`
- Objective: rename `WorldGlueKind` → `StageGlueKind { PostpassGcode, PostpassText, Finalization, Prepass, Layer }` and `resolve_world_glue` → `resolve_stage_glue`, splitting its `PostPass::TextPostProcess | PostPass::GCodePostProcess` arm in two and **deleting** its `Some("PostpassModule") => …` / `Some("FinalizationModule") => …` trait fallbacks (a stageless impl has no package, so it gets no glue). Replace `build_postpass_world_glue(self_ty, detected_stage)` with `build_postpass_gcode_glue(self_ty)` + `build_postpass_text_glue(self_ty)`, each emitting one `impl exports::slicer::<pkg>::<iface>::Guest for __Slicer<Stage>Component` with a single `fn run` and **no benign-`Ok` sibling arm**; give `build_finalization_world_glue(self_ty)` the same interface-grouped shape. Point the three `include_str!`s at the new package files and pass the new world names to `emit_world_preamble`.
- Precondition: Steps 1-4 complete.
- Postcondition: zero occurrences of `build_postpass_world_glue`, `__SlicerPostpassComponent`, `resolve_world_glue`, `WorldGlueKind`; `emit_world_preamble`'s signature and its nested-package assembly are unchanged (a stage package file is a drop-in replacement for the world file as the single top-level statement-form `package …;` header).
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-macros/src/lib.rs` - lines `120-200` (schema/metadata assembly), `380-470` (`resolve_world_glue`, glue dispatch), `476-560` (`emit_world_preamble`), `615-640` + `840-895` (postpass glue head/tail), `891-915` (finalization glue head) only. **Never read whole (2919 lines).**
  - `crates/slicer-schema/wit/README.md` - whole (61 lines) - the nested-package assembly contract
- Files allowed to edit (at most 3):
  - `crates/slicer-macros/src/lib.rs`
- Files explicitly out of bounds:
  - `crates/slicer-macros/src/lib.rs` lines `1203-2860` (`build_prepass_world_glue`, `build_layer_world_glue`) - packet #3's surface; touch only the `StageGlueKind` variant names
  - `crates/slicer-macros/tests/**` - Step 8
- Expected sub-agent dispatches:
  - Question: within `crates/slicer-macros/src/lib.rs`, list every `file:line` referencing `WorldGlueKind`, `resolve_world_glue`, or `build_postpass_world_glue`; scope: `crates/slicer-macros/src/lib.rs`; return: `LOCATIONS` (≤20 entries); purpose: rename completeness without a full read.
- Context cost: `M`
- Authoritative docs:
  - `docs/adr/0045-per-stage-versioned-interfaces-over-monolithic-tier-worlds.md` §"Verified empirically, not just read" - direct read; the guest skeleton is `impl exports::<pkg>::<iface>::Guest`, **not** the flat `Guest` this file emits today
  - `docs/05_module_sdk.md` §"Module Entry Point (`#[slicer_module]`)" - ranged read only (>1100 lines)
- OrcaSlicer refs:
  - none.
- Verification:
  - AC-6's command - FACT PASS/FAIL
- Exit condition: AC-6 prints `PASS`.

### Step 6: Retarget the three hand-written test guests; make the workspace compile

- Task IDs: `TASK-146b`
- Objective: retarget `postpass-guest` (`generate!` `world:` → `slicer:postpass-gcode-postprocess/gcode-postprocess-module`; `impl Guest` → `impl exports::slicer::postpass_gcode_postprocess::gcode_postprocess::Guest`; `fn run_gcode_postprocess` → `fn run`; **delete `fn run_text_postprocess`**), `finalization-guest` and `finalization-mutation-roundtrip-guest` (→ `slicer:finalization-layer-finalization/layer-finalization-module`, `fn run_finalization` → `fn run`). Then rebuild guests and drive the workspace to a green type-check.
- Precondition: Steps 1-5 complete. Deleting `postpass-guest::run_text_postprocess` is verified safe: all five `postpass-guest` consumers (`postpass_gcode_boundary_tdd.rs:101`, `postpass_gcode_empty_list_tdd.rs:101`, `postpass_gcode_command_preservation_tdd.rs:105`, `dispatch_infill_output_tdd.rs:152`, `dispatch_protocol_tdd.rs:36`) exercise gcode only; all three text round-trips use `sdk-postpass-text-guest` (`macro_postpass_text_roundtrip_tdd.rs:122,154,189`).
- Postcondition: `cargo xtask build-guests` succeeds for all 32 guests; `cargo check --workspace --all-targets` is green. The macro-authored guests (`sdk-postpass-text-guest`, `sdk-finalization-guest`, and the five pilot core modules) need **no** source edit.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-wasm-host/test-guests/postpass-guest/src/lib.rs` - whole
  - `crates/slicer-wasm-host/test-guests/finalization-guest/src/lib.rs` - whole
  - `crates/slicer-wasm-host/test-guests/finalization-mutation-roundtrip-guest/src/lib.rs` - whole
- Files allowed to edit (at most 3):
  - `crates/slicer-wasm-host/test-guests/postpass-guest/src/lib.rs`
  - `crates/slicer-wasm-host/test-guests/finalization-guest/src/lib.rs`
  - `crates/slicer-wasm-host/test-guests/finalization-mutation-roundtrip-guest/src/lib.rs`
- Files explicitly out of bounds:
  - `crates/slicer-wasm-host/test-guests/target/`, any `.wasm` - never load
  - `crates/slicer-wasm-host/test-guests/{layer-infill-guest,prepass-guest,infill-postprocess-echo-guest,path-optimization-multi-read,sdk-*}` - unmigrated or macro-authored
- Expected sub-agent dispatches:
  - Question: full `cargo check --workspace --all-targets` result — pass, or the first 3 distinct error codes with `file:line`; scope: workspace; return: `FACT` pass/fail plus ≤20-line `SNIPPETS` on failure; purpose: this step's exit.
- Context cost: `M`
- Authoritative docs:
  - `docs/adr/0045-per-stage-versioned-interfaces-over-monolithic-tier-worlds.md` §"Verified empirically, not just read" - direct read; the guest skeleton and the unchanged two-step build
  - `CLAUDE.md` §"Guest WASM Staleness", §"WIT/Type Changes Checklist" - direct read
- OrcaSlicer refs:
  - none.
- Verification:
  - `cargo xtask build-guests` - FACT pass/fail
  - `cargo check --workspace --all-targets` - FACT pass/fail; ≤20-line SNIPPETS on failure
- Exit condition: `cargo xtask build-guests --check` reports 0 stale **and** `cargo check --workspace --all-targets` is green. **This is the first point at which any test result means anything.** If the failure is a resource-identity/linking error across the new `bindgen!` mods (`design.md` §"Risks"), stop and report — it invalidates packet #3's premise, not just this code.

### Step 7: Make `xtask` guest staleness per-stage

- Task IDs: `TASK-146b`
- Objective: add `slicer-schema` to `xtask/Cargo.toml`; add `GuestSpec.stage_id: Option<String>`, filled for `GuestTree::Core` by parsing `[stage] id` from `modules/core-modules/<dir>/<dir>.toml` and left `None` for `GuestTree::TestGuest`; restrict `compute_shared_mtime`'s `wit` walk to `wit/root.wit` + the flat `wit/deps/*.wit`; add `stage_wit_mtime(ws_root, stage_id) -> Option<SystemTime>` resolving `wit/deps/<slicer_schema::wit_dir_for_stage_id(id)>/` (union of all package dirs when the stage is `None` or unknown — conservative, never under-rebuilds); fold it into `is_stale`. Add `#[cfg(test)] mod tests` with `stage_wit_dir_is_charged_only_to_matching_guest` and `stage_wit_unknown_stage_is_conservative`.
- Precondition: Step 6 green. This step is why AC-N2 is provable: `compute_shared_mtime` currently maxes over `wit/**/*.wit` and applies it to every guest, so ADR-0045's "doesn't even rebuild" is false in-tree regardless of packaging.
- Postcondition: `cargo test -p xtask` green; the conservative default is never inverted to make an AC pass.
- Files allowed to read, with ranges when over 300 lines:
  - `xtask/src/build_guests.rs` - lines `88-180` (`discover_guests` core branch), `459-560` (`compute_shared_mtime`, `is_stale`, `check_command`) only
  - `xtask/Cargo.toml` - whole (9 lines)
  - `modules/core-modules/machine-gcode-emit/machine-gcode-emit.toml` - whole - the `[stage] id` shape to parse
- Files allowed to edit (at most 3):
  - `xtask/src/build_guests.rs`
  - `xtask/Cargo.toml`
- Files explicitly out of bounds:
  - `xtask/src/{test.rs,dist.rs,gen_config_docs.rs,check_deviations.rs,compact_specs.rs}` - unrelated
  - `crates/slicer-schema/src/lib.rs` - Step 2 already added `wit_dir_for_stage_id`; consume it, do not extend it
- Expected sub-agent dispatches:
  - none.
- Context cost: `M`
- Authoritative docs:
  - `docs/adr/0045-per-stage-versioned-interfaces-over-monolithic-tier-worlds.md` §"Verified empirically, not just read" - direct read; isolation is measured as `sha256` byte-identity, which is how AC-N2 measures it
  - `docs/adr/0014-xtask-guest-discovery-via-validated-filesystem-walk.md` - direct read; the discovery contract `GuestSpec.stage_id` extends
  - `CLAUDE.md` §"Guest WASM Staleness" - direct read
- OrcaSlicer refs:
  - none.
- Verification:
  - AC-9's command - FACT PASS/FAIL
  - AC-N2's command - FACT PASS/FAIL
- Exit condition: AC-9 and AC-N2 both print `PASS`. AC-N2 leaves guests stale — finish with `cargo xtask build-guests` so `--check` is clean.

### Step 8: Update the emitted-surface and WIT-conformance test suites; add the two new guards

- Task IDs: `TASK-146b`
- Objective: move `wit_single_source_tdd.rs` (`canonical_wit_resolves`' world list, `worlds_are_not_self_contained`' `world_dirs` array) and `wit_drift_detection_tdd.rs` (`macro_other_world_package_names_are_canonical`, `host_inline_wit_uses_canonical_world_package_names`, `canonical_world_files_exist_on_disk`) from four tier worlds to the five delivered worlds; add `every_stage_package_major_is_at_least_one` (AC-1b) to `wit_drift_detection_tdd.rs`; add `stage_miss_is_fatal_at_instantiation` (AC-N1) to `dispatch_protocol_tdd.rs`; update the `"run"` export name in `crates/slicer-macros/tests/{binding_surface_tdd.rs, all_worlds_glue_tdd.rs, slicer_module_tdd.rs}` and in the five pilot modules' `tests/slicer_module_binding_tdd.rs`.
- Precondition: Step 7 complete; guests fresh (`cargo xtask build-guests --check` reports 0 stale).
- Postcondition: neither WIT-conformance file mentions `world-postpass` or `world-finalization`; `stage_miss_is_fatal_at_instantiation` asserts the engine's own wording and **no** "found @x.y.z" fragment.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-runtime/tests/contract/wit_single_source_tdd.rs` - whole (~320 lines; read the four ranges `34-100`, `121-160`, `183-220`, `251-310` if over budget)
  - `crates/slicer-runtime/tests/contract/wit_drift_detection_tdd.rs` - ranges `36-150`, `230-280` only
  - `crates/slicer-runtime/tests/contract/dispatch_protocol_tdd.rs` - ranges `1-60`, `130-160` only
- Files allowed to edit (at most 3, per sub-batch — run this step as three sequential sub-batches: (a) the two WIT-conformance files, (b) `dispatch_protocol_tdd.rs`, (c) the macro + module binding tests):
  - `crates/slicer-runtime/tests/contract/wit_single_source_tdd.rs`, `crates/slicer-runtime/tests/contract/wit_drift_detection_tdd.rs`
  - `crates/slicer-runtime/tests/contract/dispatch_protocol_tdd.rs`
  - `crates/slicer-macros/tests/{binding_surface_tdd.rs,all_worlds_glue_tdd.rs,slicer_module_tdd.rs}` and `modules/core-modules/{machine-gcode-emit,skirt-brim,part-cooling,wipe-tower,overhang-classifier-default}/tests/slicer_module_binding_tdd.rs`
- Files explicitly out of bounds:
  - the 15 non-pilot `modules/core-modules/*/tests/slicer_module_binding_tdd.rs` - unmigrated stages, assertions unchanged
  - `crates/slicer-runtime/tests/integration/**`, `crates/slicer-runtime/tests/e2e/**` - AC-N3 requires these unchanged
- Expected sub-agent dispatches:
  - Question: does `crates/slicer-runtime/tests/contract/main.rs` already declare `mod dispatch_protocol_tdd;` and `mod wit_drift_detection_tdd;`, and does `crates/slicer-macros/tests/all_worlds_glue_tdd.rs` assert any literal `run-gcode-postprocess` / `run-finalization` / `run-text-postprocess` string?; scope: `crates/slicer-runtime/tests/contract/main.rs`, `crates/slicer-macros/tests/*.rs`; return: `FACT` (≤5 lines); purpose: an unregistered test file reports `0 passed` and exits 0 — a false pass.
- Context cost: `M`
- Authoritative docs:
  - `docs/adr/0045-per-stage-versioned-interfaces-over-monolithic-tier-worlds.md` §"Verified empirically, not just read" - direct read; the exact diagnostic string and the `major >= 1` requirement
  - `CLAUDE.md` §"Test Discipline" - direct read
- OrcaSlicer refs:
  - none.
- Verification:
  - AC-11's command - FACT PASS/FAIL
  - AC-1b's command - FACT PASS/FAIL
  - AC-N1's command - FACT PASS/FAIL
  - AC-7's command - FACT PASS/FAIL
- Exit condition: AC-1b, AC-7, AC-11 and AC-N1 all print `PASS`.

### Step 9: Prove resource identity survived, and the behavior is neutral

- Task IDs: `TASK-146b`
- Objective: run the packet's two load-bearing proofs — AC-12 (real typed instantiation + real host↔guest resource round-trip through each pilot stage, the falsifier for `with:`-mapped resource identity across 4→5 `bindgen!` calls) and AC-8 (rebuilt artifacts decode to versioned interface exports; `arachne-perimeters` decodes none) — then AC-N3 (behavior neutrality) and AC-10 (#3's surface untouched). Fix only what these falsify.
- Precondition: Step 8 complete; `cargo xtask build-guests --check` reports 0 stale.
- Postcondition: AC-8, AC-10, AC-12 and AC-N3 green. `perimeter_parity` still `12 passed; 0 failed; 11 ignored` and `legacy_zero_matches_golden` still `1 passed; 0 failed` against the committed `ff21378e` baseline.
- Files allowed to read, with ranges when over 300 lines:
  - none by default — this is a verification step. Open a failing test's assertion (±40 lines) only when a command fails.
- Files allowed to edit (at most 3):
  - only files already listed in Steps 1-8, and only to fix a falsified assertion.
- Files explicitly out of bounds:
  - `crates/slicer-runtime/tests/integration/perimeter_parity*.rs`, `crates/slicer-runtime/tests/e2e/**` and every golden fixture - an AC-N3 regression is **caused by this packet**; it is a gate failure, not a golden to update
  - any `.wasm`, `target/`, `crates/slicer-wasm-host/test-guests/target/` - inspect artifacts only via `wasm-tools component wit … | grep …`
- Expected sub-agent dispatches:
  - Question: AC-12's command — PASS/FAIL, plus the failing test name and assertion only on failure; scope: workspace; return: `FACT` pass/fail plus ≤20-line `SNIPPETS`; purpose: this step's exit.
  - Question: AC-N3's command — PASS/FAIL plus the `test result` lines; scope: workspace; return: `FACT` (≤5 lines); purpose: behavior neutrality.
- Context cost: `M`
- Authoritative docs:
  - `docs/adr/0002-wit-marshalling-type-unification.md` - direct read; what identity is supposed to hold
  - `CLAUDE.md` §"Test Discipline" - direct read; never re-run to see more output, read `target/test-output.log`
- OrcaSlicer refs:
  - none.
- Verification:
  - AC-12's command - FACT PASS/FAIL
  - AC-8's command - FACT PASS/FAIL
  - AC-10's command - FACT PASS/FAIL
  - AC-N3's command - FACT PASS/FAIL
- Exit condition: AC-8, AC-10, AC-12 and AC-N3 all print `PASS`. If AC-12 cannot be made green, **stop and report the finding** rather than widening scope — it falsifies packet #3's premise.

### Step 10: Docs, deviation row, and backlog

- Task IDs: `TASK-146b`
- Objective: apply every edit in `packet.spec.md` §"Doc Impact Statement" — `crates/slicer-schema/wit/README.md` (layout rows; the stale "World packages carry `@1.0.0`"; the fictional host path `crates/slicer-runtime/src/wit_host.rs`, which does not exist — the file is `crates/slicer-wasm-host/src/host.rs`), `docs/03_wit_and_manifest.md` (the two tier-world listings), `docs/DEVIATION_LOG.md` (**exactly one new row**: `DEV-086` for the accepted two-mechanism intermediate, owner `164_per-stage-wit-packages-bulk`/TASK-146c — match the existing column format rather than inventing one), `docs/07_implementation_status.md` (TASK-146b). **Do not file a row for the `postpass_gcode_boundary_tdd.rs` defect** — it is already committed as `DEV-087` (`57ceae39`); reference it, do not duplicate it.
- Precondition: Step 9 green — document what shipped, not what was planned.
- Postcondition: every verification grep in §"Doc Impact Statement" passes.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-schema/wit/README.md` - whole (61 lines)
  - `docs/03_wit_and_manifest.md` - only the ranges the dispatch below returns (long; ranged reads only)
  - `docs/DEVIATION_LOG.md` - the last 40 lines only, for the row format and the next free DEV id. **Re-confirm against the log at implementation time; do not trust this packet's number.** `DEV-085` and `DEV-087` both landed *while this packet was being authored*, and an earlier draft duplicated a committed row because it trusted a stale "086 is free" reading. Run `rg -o '^\| DEV-[0-9]{3}' docs/DEVIATION_LOG.md | sort -u | tail -3` first; if `DEV-086` is taken, take the next free id and update the AC's grep to match.
- Files allowed to edit (at most 3, per sub-batch — run as two sub-batches: (a) `wit/README.md` + `docs/03`, (b) `DEVIATION_LOG.md` + `docs/07`):
  - `crates/slicer-schema/wit/README.md`, `docs/03_wit_and_manifest.md`
  - `docs/DEVIATION_LOG.md`, `docs/07_implementation_status.md`
- Files explicitly out of bounds:
  - `CONTEXT.md` - its "Module tier" / "Stage contract" entries describe the end state and are packet #3's, per `docs/specs/adr-0045-per-stage-wit-packages-plan.md` §"Status since approval"
  - `docs/04_host_scheduler.md`, `docs/05_module_sdk.md` - packet 162's surface
  - `docs/03_wit_and_manifest.md`'s `world-layer` / `world-prepass` listings - packet #3
- Expected sub-agent dispatches:
  - Question: which `docs/03_wit_and_manifest.md` line ranges hold the `world-postpass.wit` and `world-finalization.wit` listings, and do any other sections mention those two package names?; scope: `docs/03_wit_and_manifest.md`; return: `LOCATIONS` (≤20 entries); purpose: avoid loading a long doc.
  - Question: ADR-0044's argument for why `wit-world` / `SUPPORTED_WIT_WORLDS` enforce nothing; scope: `docs/adr/0044-wit-world-version-is-not-an-identity-token.md`; return: `SUMMARY` (≤200 words, no code); purpose: the DEVIATION_LOG row's rationale.
  - Question: record TASK-146b in `docs/07_implementation_status.md`; scope: `docs/07_implementation_status.md:37-39`; return: `FACT` (≤5 lines); purpose: never read the full backlog.
- Context cost: `S`
- Authoritative docs:
  - `docs/specs/adr-0045-per-stage-wit-packages-plan.md` §"Packet Queue", §"Status since approval" - ranged read only (>380 lines)
- OrcaSlicer refs:
  - none.
- Verification:
  - each §"Doc Impact Statement" grep - FACT pass/fail
- Exit condition: all five doc greps pass.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | S | Two small WIT files in, three out. No Rust. |
| Step 2 | S | One 435-line file read whole; mechanical column addition. |
| Step 3 | M | `host.rs` is 4100 lines — ranged reads only; 4 ranges + one LOCATIONS dispatch. |
| Step 4 | M | `dispatch.rs` is 2536 lines — 4 bounded ranges, no dispatch. |
| Step 5 | M | `lib.rs` is 2919 lines — 5 bounded ranges + one LOCATIONS dispatch. Largest step. |
| Step 6 | M | Three small guests; the workspace type-check is delegated. First green gate. |
| Step 7 | M | Two bounded ranges in `build_guests.rs`; unblocks AC-N2. |
| Step 8 | M | Split into three sub-batches to honor the 3-edit rule. |
| Step 9 | M | Verification only; delegated. No reads unless something fails. |
| Step 10 | S | Docs; two sub-batches; three dispatches keep the long-doc and backlog reads out of context. |

Aggregate: `M`. No step is L. Steps 8 and 10 are explicitly sub-batched so no single batch exceeds 3 edits.

## Packet Completion Gate

- All steps and exits complete.
- Every pipe-suffixed AC command returns PASS: AC-1, AC-1b, AC-2 … AC-12, AC-N1, AC-N2, AC-N3.
- `cargo xtask build-guests --check` reports 0 stale (AC-N2 deliberately dirties guests — rebuild after it).
- `cargo check --workspace --all-targets` and `cargo clippy --workspace --all-targets -- -D warnings` both green.
- Update `docs/07_implementation_status.md` through a worker dispatch, never a full backlog read.
- No reopened/superseded transitions to reconcile — 162 is a prerequisite, not a predecessor.
- Commit the packet directory together with any update to `docs/specs/adr-0045-per-stage-wit-packages-plan.md`'s Packet Queue row.
- `packet.spec.md` is ready for `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed AC and packet-level gate command.
- Dispatch `cargo xtask test --summary --workspace` to a sub-agent with a `FACT pass/fail` return plus failing-test names only (`CLAUDE.md` §"Test Discipline"). Never absorb the full output. Run it only after every narrower command above has passed.
- Record remaining packet-local risk: the four `[FWD]`s in `design.md` §"Open Questions", all owned by packet #3.
- Confirm context stayed at or below 150k standard, or at/below 300k only with a logged swarm ESCALATION; otherwise record a packet-authoring lesson.

All `cargo check`, `cargo clippy`, and `cargo test` invocations in gate and verification commands must use `--all-targets` so the test, bench, and example targets compile.
