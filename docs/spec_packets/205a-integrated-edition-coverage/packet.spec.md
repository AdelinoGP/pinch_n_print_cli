---
status: draft
packet: 205a-integrated-edition-coverage
task_ids:
  - ADR-0056
  - ADR-0057
backlog_source: docs/specs/multi-edition-distribution-plan.md (queue row 6 follow-on; "A follow-on packet (205a+) must integrate the remaining core modules")
context_cost_estimate: L
---

# Packet Contract: 205a-integrated-edition-coverage

## Goal

Integrate the sixteen core modules whose native dispatch stages are already committed by packet 202's transport — `fuzzy-skin`, `gyroid-infill`, `infill-linker`, `layer-planner-default`, `lightning-infill`, `overhang-classifier-default`, `part-cooling`, `rectilinear-infill`, `seam-placer`, `seam-planner-default`, `skirt-brim`, `support-surface-ironing`, `top-surface-ironing`, `traditional-support`, `tree-support`, `wipe-tower` — into `crates/slicer-integrated-modules/` behind per-module cargo features, each gated by a dual-dispatch parity contract test, so that the Integrated edition's coverage gate (packet 205's `verify_integrated_feature_coverage`) reports only the two transport-blocked modules (`path-optimization-default`, `machine-gcode-emit`) as missing.

## Scope Boundaries

This packet wires sixteen already-native module crates into `crates/slicer-integrated-modules/` (features, `integrated_registrations()`, `native_entries()`), adds sixteen parity contract tests plus the shared invariant helpers they need, and extends `crates/pnp-cli/Cargo.toml`'s passthrough features for the sixteen. It does **not** change dispatch routing, macro emission, marshalling, the CLI surface, or `cargo xtask dist` (202, 203, 205 respectively), and it does **not** touch the geometry call sites inside any `modules/core-modules/<name>/src/lib.rs`. It does **not** integrate `path-optimization-default` or `machine-gcode-emit` — those two stages (`Layer::PathOptimization`, `PostPass::GCodePostProcess`) are not natively committable today (packet 202's transport returns a fatal error for them), and completing those transports is packet 205b's prerequisite. This packet is the bulk of the plan's "205a+" follow-on; it does **not** by itself make `cargo xtask dist --edition integrated` build (two modules remain uncovered), which is why 205b must follow.

## Prerequisites and Blockers

- Depends on: `201-integrated-module-registry-tier5`, `202-native-adapter-and-dispatch`, `204-hybrid-pilot-parity`, and `205-editions-xtask-dist-ci`, all `implemented`. The registry (`integrated_registrations()`, `native_entries()`), the native-entry seam (`__slicer_native_entry()`, `CompiledModuleLive.native_entry`), the parity comparator (`assert_parity_structural`, `assert_prepass_parity_structural`, `ParityTolerance` in `crates/slicer-runtime/tests/common/parity_invariants.rs`), the dist-config list (`dist/editions.toml`), and the coverage gate (`verify_integrated_feature_coverage`) all exist in the tree.
- **Native-transport scope (verified against the tree, 2026-08-11):** `crates/slicer-wasm-host/src/marshal/native.rs` commits `PrePass::LayerPlanning` (line 393), `PrePass::SeamPlanning` (line 433), `PrePass::SupportGeometry` (467), `PrePass::MeshAnalysis` (500), `Layer::Infill | Layer::InfillPostProcess` (806), `Layer::Support | Layer::SupportPostProcess` (824), `Layer::Perimeters | Layer::PerimetersPostProcess` (842), and finalization (`commit_native_finalization_response`, line 684). It returns a fatal `Err` for `PrePass::PaintSegmentation` (546) and `Layer::SlicePostProcess | Layer::PathOptimization` (862). The postpass gcode commit (line 572) returns `GCodeSuccess` only when the module emits no commands and errors otherwise (line 655). **Every one of the sixteen modules in this packet's scope maps to a committed stage; the two excluded modules map to the two failing transports.** This is the packet's central scoping fact and MUST be re-verified against the tree at implementation time — if a later packet completes a transport, the corresponding module moves into this packet's scope.
- Sequencing assumption: packet 205b (transport completion for `Layer::PathOptimization` and gcode-command application) implements after this packet. This packet's sixteen modules are disjoint from 205b's two.
- Unblocks: 205b (which makes `--edition integrated` build), and the eventual closure of the multi-edition plan.
- Activation blockers: 201, 202, 204, 205 must be `implemented` first. See `design.md` §Open Questions for `[FWD]` items.

## Acceptance Criteria

State ACs only here; `requirements.md` references their IDs.

- **AC-1. Given** `slicer-integrated-modules` built with the full integrated feature set enabled (the crate's `default` feature set is empty, so `--features` must explicitly name the three 204 pilots **and** this packet's sixteen), **when** `integrated_registrations()` is called, **then** the returned registrations equal the union of the pilot set and this packet's sixteen-module set — the test derives the expected set (and therefore the count) from the registered set rather than asserting a literal count — and each of the sixteen embedded `manifest_toml` values carries `module.id` `com.core.<name>` with `origin_label` `integrated://<name>`. | `cargo test -p slicer-integrated-modules --features arachne-perimeters,classic-perimeters,fuzzy-skin,gyroid-infill,infill-linker,layer-planner-default,lightning-infill,overhang-classifier-default,part-cooling,rectilinear-infill,seam-placer,seam-planner-default,skirt-brim,support-planner,support-surface-ironing,top-surface-ironing,traditional-support,tree-support,wipe-tower full_coverage_registrations_match_registered_set`
- **AC-2. Given** the same full feature set (pilots + sixteen), **when** `native_entries()` is called, **then** the returned `(ModuleId, NativeStageEntry)` pairs cover exactly the union of the pilot set and this packet's sixteen (the expected set is derived from the registered set, not a literal count) and each of the sixteen new modules' `NativeStageEntry` family matches its declared `[stage] id`: `Layer::Infill`/`Layer::InfillPostProcess`/`Layer::Support`/`Layer::SupportPostProcess`/`Layer::PerimetersPostProcess` → `Layer(..)`, `PrePass::LayerPlanning`/`PrePass::SeamPlanning` → `Prepass(..)`, `PostPass::LayerFinalization` → `Finalization(..)`. | `cargo test -p slicer-integrated-modules --features arachne-perimeters,classic-perimeters,fuzzy-skin,gyroid-infill,infill-linker,layer-planner-default,lightning-infill,overhang-classifier-default,part-cooling,rectilinear-infill,seam-placer,seam-planner-default,skirt-brim,support-planner,support-surface-ironing,top-surface-ironing,traditional-support,tree-support,wipe-tower full_coverage_native_entry_families_match_stage_ids`
- **AC-3. Given** one `WasmRuntimeDispatcher` and two `CompiledModuleLive` values (native vs wasm) for each of the sixteen modules on a byte-identical stage input appropriate to its family, **when** the corresponding `*StageRunner::run_stage` runs on both, **then** both return `Ok(..)` and the family-appropriate parity comparator passes (structural invariants plus `1e-3` mm tolerance; float byte-equality is not asserted). One parity contract test per module, each independently red or green. | `sh -c 'for t in integrated_parity_fuzzy_skin integrated_parity_gyroid_infill integrated_parity_infill_linker integrated_parity_layer_planner integrated_parity_lightning_infill integrated_parity_overhang_classifier integrated_parity_part_cooling integrated_parity_rectilinear_infill integrated_parity_seam_placer integrated_parity_seam_planner integrated_parity_skirt_brim integrated_parity_support_surface_ironing integrated_parity_top_surface_ironing integrated_parity_traditional_support integrated_parity_tree_support integrated_parity_wipe_tower; do cargo test -p slicer-runtime --test contract -- $t >/dev/null 2>&1 || { echo "FAIL: $t"; exit 1; }; done; echo PASS'`
- **AC-4. Given** the committed dist-config list at `dist/editions.toml`, **when** `xtask`'s `load_editions` reads it, **then** `hybrid.integrated_modules` is unchanged (still the three pilots) and `integrated.integrate_all` is still `true` — this packet does **not** change edition membership; it only makes more modules registry-available. | `cargo test -p xtask editions_config_declares_three_editions`
- **AC-5. Given** the sixteen modules integrated, **when** `cargo xtask dist --edition integrated --plan` runs, **then** it still exits `1` (packet 205's coverage gate) but its error names `path-optimization-default` and `machine-gcode-emit` as the **only** missing `integrated-<name>` features, and none of the sixteen modules integrated by this packet is reported missing — the "exactly these two remain" property is derived from the registered/core-module set (every registered core module minus the integrated set must be exactly the two transport-blocked modules). The command below enforces this by asserting the two are named, the sixteen are NOT named, **and the derived missing-feature set equals the expected two** (a set comparison, so any additional uncovered registered module — or any missing one of the sixteen — fails the check). | `sh -c 'out=$(cargo xtask dist --edition integrated --plan 2>&1); rc=$?; [ "$rc" = "1" ] || { echo "FAIL rc=$rc"; exit 1; }; printf "%s" "$out" | rg -q "path-optimization-default" || { echo "FAIL: path-optimization-default not named"; exit 1; }; printf "%s" "$out" | rg -q "machine-gcode-emit" || { echo "FAIL: machine-gcode-emit not named"; exit 1; }; for m in fuzzy-skin gyroid-infill infill-linker layer-planner-default lightning-infill overhang-classifier-default part-cooling rectilinear-infill seam-placer seam-planner-default skirt-brim support-surface-ironing top-surface-ironing traditional-support tree-support wipe-tower; do printf "%s" "$out" | rg -q "integrated-$m" && { echo "FAIL: $m still reported missing"; exit 1; }; done; missing=$(printf "%s" "$out" | rg -o "integrated-[a-z0-9-]+" | sort -u); expected=$(printf "integrated-machine-gcode-emit\nintegrated-path-optimization-default\n" | sort); [ "$missing" = "$expected" ] || { echo "FAIL: missing set mismatch; got: $missing"; exit 1; }; echo PASS'`
- **AC-6. Given** every module name in the sixteen-module set, **when** `crates/pnp-cli/Cargo.toml` is inspected, **then** each has a passthrough cargo feature whose **body names** `slicer-integrated-modules/<name>` (the packet-205 AC-7 form). | `sh -c 'for m in fuzzy-skin gyroid-infill infill-linker layer-planner-default lightning-infill overhang-classifier-default part-cooling rectilinear-infill seam-placer seam-planner-default skirt-brim support-surface-ironing top-surface-ironing traditional-support tree-support wipe-tower; do rg -q "^integrated-$m *= *\[[^]]*\"slicer-integrated-modules/$m\"" crates/pnp-cli/Cargo.toml || { echo "FAIL: integrated-$m absent or does not delegate"; exit 1; }; done; echo PASS'`
- **AC-7. Given** ADR-0056 Decision item 5 (integrated modules stay single-threaded internally), **when** the sixteen module crates are inspected, **then** none of their `Cargo.toml` declares a `rayon` dependency and none of their `src/**` uses `par_iter`, `par_bridge`, `par_chunks`, or `rayon::`. | `sh -c 'for m in fuzzy-skin gyroid-infill infill-linker layer-planner-default lightning-infill overhang-classifier-default part-cooling rectilinear-infill seam-placer seam-planner-default skirt-brim support-surface-ironing top-surface-ironing traditional-support tree-support wipe-tower; do rg -q "^(rayon|\[dependencies\.rayon\]|\[target\..*dependencies\.rayon\])" modules/core-modules/$m/Cargo.toml && { echo "FAIL rayon dep: $m"; exit 1; }; rg -q "par_iter|par_bridge|par_chunks|rayon::" modules/core-modules/$m/src/ && { echo "FAIL rayon use: $m"; exit 1; }; done; echo PASS'`

## Negative Test Cases

- **AC-N1. Given** the parity comparator for a layer-family module, **when** it is handed two commits where the second is missing one closed loop (the ADR-0042 D5 geometry-dropping failure class), **then** it returns `Err` naming the differing invariant — proving the new per-module gates are not vacuous. | `cargo test -p slicer-runtime --test contract -- parity_comparator_rejects_dropped_loop`
- **AC-N2. Given** an external `com.core.<name>` on a disk search root plus the integrated registration and native-entry table for a newly-integrated module, **when** `load_live_modules_for_plan_with_integrated` builds bindings, **then** that module's `LiveModuleBinding` has `native_entry: None` and `wasm_component: Some(..)` — the integration never bypasses a user's external override. | `cargo test -p slicer-runtime --test integration full_coverage_external_override_forces_wasm`
- **AC-N3. Given** the coverage gate from packet 205, **when** it is handed an integrated set containing a module with no `integrated-<name>` feature, **then** it returns `Err` naming that module — the gate still fires for any module this packet does not cover. | `cargo test -p xtask dist_registry_coverage_rejects_missing_pnp_cli_feature`

## Verification

- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo xtask build-guests --check` (sixteen `Cargo.toml` edits invalidate their wasm twins; parity tests need **both** artifacts fresh — must report clean after rebuild)

## Authoritative Docs

- `docs/adr/0056-integrated-modules-native-dispatch.md` — short; direct read; Decision items 4–5 are this packet's contract.
- `docs/adr/0057-three-editions-and-integrated-tier.md` — short; direct read; the Integrated-edition "every core module" clause.
- `docs/adr/0042-arachne-parity-structural-invariants-over-fixtures.md` — long; direct read of §Decision only; the invariant class list.
- `docs/spec_packets/204-hybrid-pilot-parity/packet.spec.md` — whole file; the registration/native-entry/parity pattern this packet replicates.
- `docs/spec_packets/204-hybrid-pilot-parity/design.md` — whole file; the integration pattern and the 202-gap list (re-verify against the tree, as this packet did).
- `crates/slicer-wasm-host/src/marshal/native.rs` — the committed-vs-fatal stage dispatch (re-verify the sixteen stages are committed).
- `CONTEXT.md` — terms **Integrated module**, **External module**; delegate a `FACT` lookup rather than reading the file.

## Doc Impact Statement (Required)

Specific same-packet doc edits:

- `docs/01_system_architecture.md` §"Producing the tier-4 layout: `cargo xtask dist`": update the edition-selection paragraph to note that the Integrated edition's coverage is now limited to the two transport-blocked modules (`path-optimization-default`, `machine-gcode-emit`) pending packet 205b. — `rg -q 'path-optimization-default' docs/01_system_architecture.md`
- `docs/specs/multi-edition-distribution-plan.md` §"Also unscheduled": update the "follow-on packet (205a+)" note to record that 205a integrates sixteen modules and 205b completes the two remaining transports. — `rg -q '205a' docs/specs/multi-edition-distribution-plan.md`

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- obey the shared absolute context bands: 120k reading budget with hand-off at 150k (standard); the extended band (240k reading / 300k hard stop) only via swarm's escalation protocol

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation (an extended-band run may carry a single L step only when `design.md` justifies why it cannot be split).
