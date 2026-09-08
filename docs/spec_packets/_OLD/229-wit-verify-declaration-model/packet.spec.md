---
status: implemented
packet: 229-wit-verify-declaration-model
task_ids:
  - TASK-340
backlog_source: docs/07_implementation_status.md
context_cost_estimate: M
---

# Packet Contract: 229-wit-verify-declaration-model

## Goal

Rebuild `xtask/src/wit_verify.rs` on `wit_parser` so that both the canonical `.wit` tree and a decoded artifact world are parsed into one package-qualified declaration model, compared with stage-package full equality on the exported interface plus subset direction everywhere else, and fail closed on unexpected packages and on an empty or unreadable canonical set.

## Scope Boundaries

This packet owns the verifier's parsing, modelling and comparison engine, the canonical-coverage audit against the macro's `include_str!` set, and the `crates/slicer-macros/build.rs` `rerun-if-changed` correction. It migrates the single existing call site inside `build_one` (`xtask/src/build_guests.rs`) to the new API, adds one `BuildError` variant, and retypes `BuildError::StaleEmbeddedWorld`'s `mismatches` payload from `Vec<TypeMismatch>` to `Vec<Drift>`. It does **not** touch `check_command`, `test_command`, the fingerprint model, or stage resolution from artifacts — those are packet 230.

## Prerequisites and Blockers

- Depends on: none. `wit-parser = "0.247"` is already a direct dependency of `crates/slicer-runtime` and `crates/slicer-wasm-host` and is already resolved in `Cargo.lock`, so adding it to `xtask/Cargo.toml` introduces no new version into the graph.
- Unblocks: `docs/spec_packets/230-output-based-guest-freshness`. It consumes exactly these 16 items from `xtask/src/wit_verify.rs`, whose full signatures are specified in this packet's `design.md` §Code Change Surface: `WorldModel`, `PackageModel`, `InterfaceModel`, `StageExpectation`, `stage_expectation`, `Drift`, `DriftKind`, `SHARED_PACKAGES`, `ROOT_COMPONENT_PACKAGE`, `canonical_world_model`, `embedded_world_model`, `compare_worlds`, `verify_embedded_world`, `embedded_wit_text` (retained unchanged), and the `VerifyError` variants `Decode`, `Parse`, `CanonicalEmpty`, `CanonicalUnreadable`.
- Activation blockers: none.

## Acceptance Criteria

- **AC-1. Given** the canonical WIT tree at `crates/slicer-schema/wit/`, **when** `canonical_world_model` builds its canonical file list, **then** the list contains exactly the 20 `.wit` paths that `crates/slicer-macros/src/lib.rs` passes to `include_str!` (the 5 flat files `deps/types.wit`, `deps/config.wit`, `deps/ir-types.wit`, `deps/common.wit`, `deps/prepass-types.wit`, plus the 15 per-stage `deps/<dir>/<dir>.wit` files) and does **not** contain `crates/slicer-schema/wit/root.wit`. | `mkdir -p target && cargo test -p xtask wit_verify::tests::canonical_file_list_equals_macro_include_str_set -- --exact 2>&1 | tee target/test-output.log | rg '^test result: ok\. 1 passed'`
- **AC-2. Given** `crates/slicer-macros/build.rs`, **when** its `cargo:rerun-if-changed` list is parsed, **then** it names exactly the same 20 canonical files as AC-1, and contains none of `deps/world-prepass/world-prepass.wit`, `deps/world-postpass/world-postpass.wit`, `deps/world-finalization/world-finalization.wit`, `deps/world-layer/world-layer.wit`. | `mkdir -p target && cargo test -p xtask wit_verify::tests::macros_build_rs_watches_the_macro_include_str_set -- --exact 2>&1 | tee target/test-output.log | rg '^test result: ok\. 1 passed'`
- **AC-3. Given** a synthetic embedded world whose `record region-key` declares the same fields as canonical but in swapped order, **when** `compare_worlds` runs, **then** it returns at least one `Drift` of kind `DeclarationBody` naming `region-key` (record field order is ABI-relevant and must not be normalized away). | `mkdir -p target && cargo test -p xtask wit_verify::tests::record_field_reorder_is_drift -- --exact 2>&1 | tee target/test-output.log | rg '^test result: ok\. 1 passed'`
- **AC-4. Given** a synthetic embedded world whose `variant extrusion-role` declares the same cases as canonical but with two cases swapped, **when** `compare_worlds` runs, **then** it returns at least one `Drift` of kind `DeclarationBody` naming `extrusion-role`. | `mkdir -p target && cargo test -p xtask wit_verify::tests::variant_case_reorder_is_drift -- --exact 2>&1 | tee target/test-output.log | rg '^test result: ok\. 1 passed'`
- **AC-5. Given** embedded worlds that respectively change a `type`-alias right-hand side, change a `resource` method signature, and omit a `use` target that canonical declares for the exported stage interface, **when** `compare_worlds` runs on each, **then** the drifts reported are `DeclarationBody`, `DeclarationBody` and `MissingUse` — proving aliases, resources and `use` declarations are all in the model. | `mkdir -p target && cargo test -p xtask wit_verify::tests::aliases_resources_and_uses_are_modelled -- --exact 2>&1 | tee target/test-output.log | rg '^test result: ok\. 1 passed'`
- **AC-6. Given** the resolved stage package of an embedded world, **when** its **exported** interface is compared, **then** a declaration present in canonical but absent from the embedded exported interface is reported as `Drift` kind `MissingDeclaration`, and one present in the embedded exported interface but absent from canonical is reported as `ExtraDeclaration` (full equality, both directions). | `mkdir -p target && cargo test -p xtask wit_verify::tests::exported_stage_interface_is_full_equality_both_directions -- --exact 2>&1 | tee target/test-output.log | rg '^test result: ok\. 1 passed'`
- **AC-7. Given** a **non-exported** interface of the resolved stage package (e.g. `layer-finalization-types`), **when** the embedded world omits whole members that canonical declares, **then** no drift is reported; and when a member declared on both sides has a differing body, `Drift` kind `DeclarationBody` is reported (R5-1 subset direction — local `*-types` interfaces are present in decoded artifacts, not pruned). | `mkdir -p target && cargo test -p xtask wit_verify::tests::non_exported_stage_interfaces_use_subset_direction -- --exact 2>&1 | tee target/test-output.log | rg '^test result: ok\. 1 passed'`
- **AC-8. Given** the 5 shared packages `slicer:types`, `slicer:config`, `slicer:ir-handles`, `slicer:common`, `slicer:prepass-types`, **when** an embedded world declares only a subset of a shared interface's members (e.g. `config-view` carrying only `get` and `keys`), **then** no drift is reported; and `use` declarations for shared packages are compared as sets, so reordering two `use` statements yields no drift. | `mkdir -p target && cargo test -p xtask wit_verify::tests::shared_packages_use_subset_direction -- --exact 2>&1 | tee target/test-output.log | rg '^test result: ok\. 1 passed'`
- **AC-9. Given** an embedded world declaring a package outside the allowed set `{root:component, the 5 shared packages, the resolved stage package}`, **when** `compare_worlds` runs, **then** it returns `Drift` kind `UnexpectedPackage` naming that package (fail closed). | `mkdir -p target && cargo test -p xtask wit_verify::tests::unexpected_package_is_drift -- --exact 2>&1 | tee target/test-output.log | rg '^test result: ok\. 1 passed'`
- **AC-10. Given** a `StageExpectation` whose `qualified_export` comes from `slicer_schema::qualified_export_for_stage_id`, **when** the embedded world's export name differs only in its version suffix, **then** `Drift` kind `ExportName` is reported (export name compared exactly, version included). | `mkdir -p target && cargo test -p xtask wit_verify::tests::export_name_compared_exactly_including_version -- --exact 2>&1 | tee target/test-output.log | rg '^test result: ok\. 1 passed'`
- **AC-11. Given** the on-disk artifacts `modules/core-modules/wipe-tower/wipe-tower.wasm` (`PostPass::LayerFinalization` stage) and `modules/core-modules/layer-planner-default/layer-planner-default.wasm` (`PrePass::LayerPlanning` stage), **when** each is decoded and compared against its canonical stage model, **then** `compare_worlds` returns an empty drift list; the test asserts rather than returning early whenever `wasm-tools` resolves on `PATH` and the artifact exists, and only the genuinely-absent-tool case is skipped. | `mkdir -p target && cargo test -p xtask wit_verify::tests::real_core_module_artifacts_verify_clean -- --exact 2>&1 | tee target/test-output.log | rg '^test result: ok\. 1 passed'`
- **AC-12. Given** a WIT source using the braced package form `package slicer:types { interface ... }`, **when** it is parsed into a `WorldModel`, **then** its declarations key under package `slicer:types` identically to the statement form. No braced-form file exists under `crates/slicer-schema/wit/` today (all 21 use `package x:y;`), so this case is covered by a synthetic in-memory source and is defensive only — no tree fixture is invented. | `mkdir -p target && cargo test -p xtask wit_verify::tests::braced_package_form_parses_like_statement_form -- --exact 2>&1 | tee target/test-output.log | rg '^test result: ok\. 1 passed'`
- **AC-13. Given** the rebuilt verifier, **when** every retired hand-rolled scanner symbol is searched for, **then** none of `extract_type_blocks`, `matching_brace`, `BRACED_KEYWORDS`, `ambiguous_type_names`, `canonical_type_blocks`, `TypeMismatch`, `fn strip_comments` or `fn normalize` appears in `xtask/src/wit_verify.rs` or `xtask/src/build_guests.rs` — the list matches `design.md` §Code Change Surface's deletion list exactly, including `BuildError::StaleEmbeddedWorld`'s retyped payload no longer naming `TypeMismatch`. | `if rg -q 'extract_type_blocks|matching_brace|BRACED_KEYWORDS|ambiguous_type_names|canonical_type_blocks|TypeMismatch|fn strip_comments|fn normalize' xtask/src/wit_verify.rs xtask/src/build_guests.rs; then echo FAIL; else echo PASS; fi`
- **AC-14. Given** `docs/07_implementation_status.md`, **when** the packet closes, **then** a `TASK-340` row exists under "Workstream 5 — Governance and closure drift". | `rg -q 'TASK-340 ' docs/07_implementation_status.md && echo PASS || echo FAIL`

## Negative Test Cases

- **AC-N1. Given** a canonical WIT root directory containing no readable `.wit` file, **when** `canonical_world_model` is called on it, **then** it returns `Err(VerifyError::CanonicalEmpty)` — never an empty-but-`Ok` model. This retires the old `canonical_type_blocks` behaviour of swallowing read failures inside `if let Ok(text)`. | `mkdir -p target && cargo test -p xtask wit_verify::tests::empty_canonical_set_is_an_error_not_a_pass -- --exact 2>&1 | tee target/test-output.log | rg '^test result: ok\. 1 passed'`
- **AC-N2. Given** a canonical file list in which one required file is absent from disk, **when** `canonical_world_model` is called, **then** it returns `Err(VerifyError::CanonicalUnreadable { path, reason })` whose `path` names the missing file. | `mkdir -p target && cargo test -p xtask wit_verify::tests::unreadable_canonical_file_is_an_error_not_a_pass -- --exact 2>&1 | tee target/test-output.log | rg '^test result: ok\. 1 passed'`
- **AC-N3. Given** decoded artifact text that `wit_parser` cannot resolve, **when** `embedded_world_model` is called, **then** it returns `Err(VerifyError::Parse { artifact, reason })`; the comparison is never reported clean on a parse failure. | `mkdir -p target && cargo test -p xtask wit_verify::tests::unparseable_embedded_text_is_an_error_not_a_pass -- --exact 2>&1 | tee target/test-output.log | rg '^test result: ok\. 1 passed'`
- **AC-N4. Given** an embedded world whose resolved stage package is absent entirely (only shared packages present), **when** `compare_worlds` runs with a `StageExpectation`, **then** it returns `Drift` kind `MissingStagePackage` naming the expected package — an artifact that exports nothing for its stage must not compare clean. | `mkdir -p target && cargo test -p xtask wit_verify::tests::missing_stage_package_is_drift -- --exact 2>&1 | tee target/test-output.log | rg '^test result: ok\. 1 passed'`

## Verification

- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `mkdir -p target && cargo test -p xtask wit_verify 2>&1 | tee target/test-output.log | rg '^test result:'`

## Authoritative Docs

- `docs/03_wit_and_manifest.md` — direct ranged read of section "Build & Freshness Contract (Normative)" only; establishes the freshness contract this verifier sits beside. The doc text itself is rewritten by packet 232, not here.
- `docs/07_implementation_status.md` — over 300 lines; delegate. Only the "Workstream 5 — Governance and closure drift" section is edited, to add the `TASK-340` row.
- `CLAUDE.md` — direct read of sections "Guest WASM Staleness", "In-Tree Citation Style (MUST follow)", "Test Discipline".

## Doc Impact Statement (Required)

- `docs/07_implementation_status.md` section "Workstream 5 — Governance and closure drift" — add the `TASK-340` row: `rg -q 'TASK-340 ' docs/07_implementation_status.md`

No IR, WIT, scheduler, claim, manifest, host-service or SDK contract changes: this packet changes only how canonical WIT is read and compared, never the WIT itself.

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- obey the shared absolute context bands: 120k reading budget with hand-off at 150k (standard); the extended band (240k reading / 300k hard stop) only via swarm's escalation protocol

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation (an extended-band run may carry a single L step only when `design.md` justifies why it cannot be split).
