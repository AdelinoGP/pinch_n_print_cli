---
status: implemented
packet: 205b-native-transport-completion
task_ids:
  - ADR-0056
  - ADR-0057
backlog_source: docs/specs/multi-edition-distribution-plan.md (queue row 6 follow-on; the two modules 205a defers)
context_cost_estimate: M
---

# Packet Contract: 205b-native-transport-completion

## Goal

Complete the two native dispatch transports packet 202 left as fatal errors — `Layer::PathOptimization` output commit and postpass gcode-command application — then integrate the two modules that depend on them (`path-optimization-default`, `machine-gcode-emit`) behind per-module cargo features with dual-dispatch parity gates, so that `cargo xtask dist --edition integrated` finally builds (every core module integrated, nothing staged externally).

## Scope Boundaries

This packet extends `crates/slicer-wasm-host/src/marshal/native.rs` (the two missing commit arms), adds the two modules to `crates/slicer-integrated-modules/` (features, `integrated_registrations()`, `native_entries()`), adds two parity contract tests, and extends `crates/pnp-cli/Cargo.toml`'s passthrough features. It does **not** change dispatch routing, macro emission, the CLI surface, or `cargo xtask dist` (202, 203, 205 respectively), and it does **not** touch the geometry call sites inside `modules/core-modules/{path-optimization-default,machine-gcode-emit}/src/lib.rs`. It is the final packet of the plan's "205a+" follow-on: after it, `--edition integrated` builds and the multi-edition plan's Integrated-edition row is closed.

**User-approved scope expansion:** completing the two transports required a user-approved scope expansion to `crates/slicer-sdk/src/native.rs` (added `path_optimization` field on `NativeLayerResponse` + `NativePathOptimizationOutput`), `crates/slicer-macros/src/lib.rs` (the `run_path_optimization` native entry now populates the field), `crates/slicer-wasm-host/src/dispatch.rs` (the two native postpass callers now pass the gcode command accumulator), and `crates/slicer-runtime/Cargo.toml` (dev-deps + features for the two modules). These files were originally out of bounds and are now in scope by user approval.

## Prerequisites and Blockers

- Depends on: 201, 202, 204, and 205 (all `implemented`; the registry, native-entry seam, parity comparator, dist-config list, and coverage gate all exist) **and** `205a-integrated-edition-coverage` (currently `draft`; it must be `implemented` before this packet activates — its sixteen modules are the prerequisite coverage this packet's two complete).
- **The two transports to complete (verified against the tree, 2026-08-11):**
  - `Layer::PathOptimization` — `crates/slicer-wasm-host/src/marshal/native.rs` line 862 returns `Err("native path does not yet support stage {stage_export} output commit")` for `Layer::SlicePostProcess | Layer::PathOptimization`. The path-optimization stage's output shape (`LayerStageCommit::PathOptimization(PathOptimizationCommit)`, the `PathOptimizationCommit` payload in `crates/slicer-ir/src/stage_io.rs`) must be committed through the native transport, mirroring how the infill/perimeter/support commits work.
  - Postpass gcode — `commit_native_postpass_response` (line 572) returns `GCodeSuccess` only when the module emits no commands and errors otherwise (line 655: "native gcode postpass emitted commands, but the native commit transport cannot apply them"). The gcode-command application path must be completed so a gcode-emitting module's commands are applied to the output.
- Sequencing assumption: 205a implements before this packet. 205a integrates the sixteen committable modules; this packet's two modules are disjoint from 205a's sixteen.
- Unblocks: the closure of the multi-edition plan's Integrated-edition row; `cargo xtask dist --edition integrated` builds.
- Activation blockers: 205a must be `implemented` first. See `design.md` §Open Questions for `[FWD]` items.

## Acceptance Criteria

State ACs only here; `requirements.md` references their IDs.

- **AC-1. Given** `slicer-integrated-modules` built with the full integrated feature set enabled (the crate's `default` feature set is empty, so `--features` must explicitly name the three 204 pilots, the sixteen 205a modules, **and** this packet's two), **when** `integrated_registrations()` is called, **then** the returned registrations equal the union of the pilot set, the 205a set, and this packet's two-module set — the test derives the expected set (and therefore the count) from the registered set rather than asserting a literal count — and the two new embedded `manifest_toml` values carry `module.id` values `com.core.path-optimization-default` and `com.core.machine-gcode-emit`, with `origin_label` values `integrated://path-optimization-default` and `integrated://machine-gcode-emit`. | `cargo test -p slicer-integrated-modules --features arachne-perimeters,classic-perimeters,fuzzy-skin,gyroid-infill,infill-linker,layer-planner-default,lightning-infill,machine-gcode-emit,overhang-classifier-default,part-cooling,path-optimization-default,rectilinear-infill,seam-placer,seam-planner-default,skirt-brim,support-planner,support-surface-ironing,top-surface-ironing,traditional-support,tree-support,wipe-tower full_coverage_registrations_match_registered_set`
- **AC-2. Given** the same full feature set, **when** `native_entries()` is called, **then** the returned `(ModuleId, NativeStageEntry)` pairs cover exactly the union of the pilot set, the 205a set, and this packet's two (the expected set is derived from the registered set, not a literal count) and the two new modules' `NativeStageEntry` families match their declared `[stage] id`: `path-optimization-default` (`Layer::PathOptimization`) → `Layer(..)`, `machine-gcode-emit` (`PostPass::GCodePostProcess`) → `Postpass(..)` — the concrete `NativeStageEntry` variants, per `crates/slicer-sdk/src/native.rs`'s four-variant enum (`Layer`, `Prepass`, `Postpass`, `Finalization`). | `cargo test -p slicer-integrated-modules --features arachne-perimeters,classic-perimeters,fuzzy-skin,gyroid-infill,infill-linker,layer-planner-default,lightning-infill,machine-gcode-emit,overhang-classifier-default,part-cooling,path-optimization-default,rectilinear-infill,seam-placer,seam-planner-default,skirt-brim,support-planner,support-surface-ironing,top-surface-ironing,traditional-support,tree-support,wipe-tower full_coverage_native_entry_families_match_stage_ids`
- **AC-3. Given** one `WasmRuntimeDispatcher` and two `CompiledModuleLive` values (native vs wasm) for `com.core.path-optimization-default` on a byte-identical `LayerStageInput`, **when** `LayerStageRunner::run_stage` runs on both, **then** both return `Ok(Some(LayerStageCommit))` and the path-optimization parity comparator passes (structural invariants plus `1e-3` mm tolerance) — proving the `Layer::PathOptimization` native commit is complete and correct. | `mkdir -p target && cargo test -p slicer-runtime --test contract -- integrated_parity_path_optimization 2>&1 | tee target/test-output.log && rg -q "^test result: ok" target/test-output.log`
- **AC-4. Given** one `WasmRuntimeDispatcher` and two `CompiledModuleLive` values (native vs wasm) for `com.core.machine-gcode-emit` on a byte-identical postpass input, **when** the postpass runner runs on both, **then** both return `Ok(PostpassOutput)` and the emitted gcode command sequences agree structurally (same command count, same command kinds in order, same parameters within tolerance) — proving the gcode-command application path is complete. | `mkdir -p target && cargo test -p slicer-runtime --test contract -- integrated_parity_machine_gcode_emit 2>&1 | tee target/test-output.log && rg -q "^test result: ok" target/test-output.log`
- **AC-5. Given** the committed dist-config list at `dist/editions.toml`, **when** `xtask`'s `load_editions` reads it, **then** `integrated.integrate_all` is still `true` and `hybrid.integrated_modules` is unchanged — this packet does not change edition membership. | `cargo test -p xtask editions_config_declares_three_editions`
- **AC-6. Given** all core modules integrated, **when** `cargo xtask dist --edition integrated --plan` runs, **then** it exits `0`, prints the `integrated` edition header, and its `integrated` lines cover **every registered core module stem** — the expected stem set is derived from `discover_guests` over `modules/core-modules/` (the same source the coverage gate uses), not a literal list — and its `external` lines are empty. The Integrated edition now builds. | `sh -c 'cargo test -p xtask dist_plan_integrated_stages_nothing_externally >/dev/null || { echo "FAIL: discover_guests-derived coverage test"; exit 1; }; out=$(cargo xtask dist --edition integrated --plan 2>&1); rc=$?; [ "$rc" = "0" ] || { echo "FAIL rc=$rc"; exit 1; }; printf "%s" "$out" | rg -q "^edition\tintegrated" || { echo "FAIL: not integrated edition"; exit 1; }; printf "%s" "$out" | rg -q "^external\t" && { echo "FAIL: integrated edition stages externally"; exit 1; }; echo PASS'`
- **AC-7. Given** every module name in the two-module set, **when** `crates/pnp-cli/Cargo.toml` is inspected, **then** each has a passthrough cargo feature whose **body names** `slicer-integrated-modules/<name>`. | `sh -c 'for m in path-optimization-default machine-gcode-emit; do rg -q "^integrated-$m *= *\[[^]]*\"slicer-integrated-modules/$m\"" crates/pnp-cli/Cargo.toml || { echo "FAIL: integrated-$m absent or does not delegate"; exit 1; }; done; echo PASS'`
- **AC-8. Given** ADR-0056 Decision item 5, **when** the two module crates are inspected, **then** neither declares a `rayon` dependency and neither uses `par_iter`, `par_bridge`, `par_chunks`, or `rayon::`. | `sh -c 'for m in path-optimization-default machine-gcode-emit; do rg -q "^(rayon|\[dependencies\.rayon\]|\[target\..*dependencies\.rayon\])" modules/core-modules/$m/Cargo.toml && { echo "FAIL rayon dep: $m"; exit 1; }; rg -q "par_iter|par_bridge|par_chunks|rayon::" modules/core-modules/$m/src/ && { echo "FAIL rayon use: $m"; exit 1; }; done; echo PASS'`

## Negative Test Cases

- **AC-N1. Given** the path-optimization parity comparator, **when** it is handed two commits where the second is missing one path (the ADR-0042 D5 geometry-dropping failure class), **then** it returns `Err` naming the differing invariant — proving the new gate is not vacuous. | `cargo test -p slicer-runtime --test contract -- parity_comparator_rejects_dropped_path`
- **AC-N2. Given** an external `com.core.path-optimization-default` on a disk search root plus the integrated registration and native-entry table, **when** `load_live_modules_for_plan_with_integrated` builds bindings, **then** that module's `LiveModuleBinding` has `native_entry: None` and `wasm_component: Some(..)`. | `cargo test -p slicer-runtime --test integration full_coverage_external_override_forces_wasm`
- **AC-N3. Given** the coverage gate from packet 205, **when** it is handed an integrated set containing a module with no `integrated-<name>` feature, **then** it returns `Err` naming that module — the gate still fires for any module not covered. | `cargo test -p xtask dist_registry_coverage_rejects_missing_pnp_cli_feature`

## Verification

- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo xtask build-guests --check` (two `Cargo.toml` edits invalidate their wasm twins; parity tests need **both** artifacts fresh)

## Authoritative Docs

- `docs/adr/0056-integrated-modules-native-dispatch.md` — short; direct read; Decision items 4-5.
- `docs/adr/0057-three-editions-and-integrated-tier.md` — short; direct read; the Integrated-edition "every core module" clause.
- `docs/adr/0042-arachne-parity-structural-invariants-over-fixtures.md` — long; direct read of §Decision only.
- `docs/spec_packets/205a-integrated-edition-coverage/packet.spec.md` and `design.md` — whole files; the pattern this packet completes.
- `crates/slicer-wasm-host/src/marshal/native.rs` — the two commit arms to complete.
- `CONTEXT.md` — terms **Integrated module**, **External module**; delegate a `FACT` lookup.

## Doc Impact Statement (Required)

Specific same-packet doc edits:

- `docs/01_system_architecture.md` §"Producing the tier-4 layout: `cargo xtask dist`": update the edition-selection paragraph to record that the Integrated edition now builds (every registered core module integrated, nothing staged externally). — `rg -q 'integrated' docs/01_system_architecture.md`
- `docs/specs/multi-edition-distribution-plan.md` §"Also unscheduled": update the "follow-on packet (205a+)" note to record that 205b completes the two remaining transports and the Integrated edition now builds, closing the plan's Integrated-edition row. — `rg -q '205b' docs/specs/multi-edition-distribution-plan.md`

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- obey the shared absolute context bands: 120k reading budget with hand-off at 150k (standard); the extended band (240k reading / 300k hard stop) only via swarm's escalation protocol

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation (an extended-band run may carry a single L step only when `design.md` justifies why it cannot be split).
