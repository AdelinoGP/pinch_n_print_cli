# Implementation Plan: 267-support-ironing-gate

## Execution Rules

- Work one atomic step at a time; map every step to [22 - Author packet P15 - Support / Support ironing - support-surface-ironing](../specs/orca-feature-gap/issues/22-author-packet-p15-support-support-ironing-support-surface-ironing.md); this queue packet has `task_ids: []`.
- Use TDD, then implementation, then the narrowest falsifying validation.
- Every field below is a context-budget contract and must be filled independently; never write "see Step 1".

## Steps

### Step 1: Declare the canonical gate key and its schema guard

- Task IDs: `[]` (wayfinder ticket 22)
- Objective: replace the manifest's `[config.schema.ironing_enabled]` table with `[config.schema.support_ironing]` (`type = "bool"`, `default = false`, `display = "Ironing Support Interface"`, `group = "Support"`), and add a TOML-direct-parse guard that pins both the new table and the removal of the old one.
- Precondition: `support-surface-ironing.toml` carries the existing five-key schema, including `ironing_enabled`; no packet directory or test guard elsewhere claims `support_ironing`.
- Postcondition: the manifest has exactly one gate table, named `support_ironing`, with canonical type and default; the four sibling tables are unchanged field for field; the guard binary is auto-discovered and passes.
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/support-surface-ironing/support-surface-ironing.toml` - full manifest; it is short.
  - `modules/core-modules/support-surface-ironing/Cargo.toml` - package and dev-dependency sections.
  - `modules/core-modules/part-cooling/Cargo.toml` - TOML dev-dependency precedent only.
- Files allowed to edit (at most 3):
  - `modules/core-modules/support-surface-ironing/support-surface-ironing.toml`
  - `modules/core-modules/support-surface-ironing/Cargo.toml`
  - `modules/core-modules/support-surface-ironing/tests/support_ironing_config_schema_tdd.rs`
- Files explicitly out of bounds:
  - `modules/core-modules/top-surface-ironing/**` - packet 266's surface.
  - `crates/slicer-gcode/src/serialize.rs` - `ORCA_CONFIG_PADDING`; map rule 2.
  - `docs/15_config_keys_reference.md` - generated in Step 4.
  - `OrcaSlicerDocumented/**` - delegate; never load.
- Blast-radius discipline: none; manifest data plus a new test target add no Rust struct field and no schema constant.
- Expected sub-agent dispatches:
  - Question: is `toml` absent from this module's dev-dependencies, and does it use test auto-discovery with no explicit `[[test]]` entries?; scope: the module `Cargo.toml` and `tests/`; return: `FACT`.
- Context cost: `S`
- Authoritative docs:
  - `docs/03_wit_and_manifest.md` - delegated SUMMARY of the bool schema form.
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` - the `support_ironing` `coBool` default; delegated, and already captured in `requirements.md`. Re-dispatch only if the default is disputed.
- Verification:
  - `cargo test -p support-surface-ironing --test support_ironing_config_schema_tdd 2>&1 | tee target/test-output.log | grep -E '^test result'` - FACT pass/fail.
- Exit condition: the guard passes, asserts the `support_ironing` table and the absence of an `ironing_enabled` table, and a targeted TOML search of this manifest returns no `ironing_enabled`. The module's own suites are expected to be red at this point — Step 2 closes them.

### Step 2: Move the module gate onto the canonical key, with invariants

- Task IDs: `[]` (wayfinder ticket 22)
- Objective: change `SupportSurfaceIroning::from_config`'s single gate read to `config.get("support_ironing")` (field and getter renamed with it, absent still meaning `false`), migrate the module's two test binaries, and add the AC-2 off/absent arms plus the AC-N1 legacy regression.
- Precondition: Step 1's manifest and guard pass; the module's other four keys and the scan-line generator are untouched.
- Postcondition: `support_ironing = true` emits at least one `ExtrusionRole::Ironing` path on a square-region fixture; `false` and absent emit none; a config carrying only legacy `ironing_enabled = true` emits none; the scan-line parity suite still passes unchanged in substance.
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/support-surface-ironing/src/lib.rs` - `from_config`, the struct fields and getters, and `run_support_postprocess`'s early return only.
  - `modules/core-modules/support-surface-ironing/tests/ironing_tdd.rs` - `config_with` / `enabled_config` helpers and the existing assertions.
  - `modules/core-modules/support-surface-ironing/tests/ironing_scanline_parity_tdd.rs` - its config fixture only.
- Files allowed to edit (at most 3):
  - `modules/core-modules/support-surface-ironing/src/lib.rs`
  - `modules/core-modules/support-surface-ironing/tests/ironing_tdd.rs`
  - `modules/core-modules/support-surface-ironing/tests/ironing_scanline_parity_tdd.rs`
- Files explicitly out of bounds:
  - `crates/slicer-ir/src/**`, `crates/slicer-sdk/src/**`, `crates/slicer-schema/wit/**` - no boundary change; use the existing `ConfigView` accessor.
  - `modules/core-modules/top-surface-ironing/**` - packet 266's surface.
  - `crates/slicer-runtime/**` - Step 3.
  - `OrcaSlicerDocumented/**` - delegate; never load.
- Blast-radius discipline: the renamed field and getter are module-private plus one `pub fn`; no public struct gains a field, so no struct-literal waiver is needed. Run `cargo xtask check-literals` if any test constructs a watched type.
- Expected sub-agent dispatches:
  - Question: quote the exact gate read and struct initialisation in `from_config`, and list every `ironing_enabled` occurrence in the module's two test binaries; scope: that module's `src/` and `tests/`; return: `LOCATIONS`.
  - Question: confirm canonical gates support ironing on `support_params.ironing` in `generate_support_toolpaths` and that the default is off; scope: the delegated Orca files in `requirements.md`; return: `SNIPPETS` (1, <=20 lines).
- Context cost: `S`
- Authoritative docs:
  - `docs/03_wit_and_manifest.md` - delegated summary of per-module `ConfigView` filtering and undeclared-key dropping.
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/Support/SupportCommon.cpp` - `generate_support_toolpaths`, delegated only.
  - `OrcaSlicerDocumented/src/libslic3r/Support/SupportParameters.hpp` - the `ironing` assignment, delegated only.
- Verification:
  - `cargo test -p support-surface-ironing 2>&1 | tee target/test-output.log | grep -E '^test result'` - FACT pass/fail across all three binaries.
- Exit condition: all module binaries pass with named `support_ironing` on/off/absent tests and the legacy-key regression; `rg -n 'ironing_enabled' modules/core-modules/support-surface-ironing` returns nothing; the production `run_support_postprocess` path (not only a helper's own test) is exercised by at least one of the passing tests.

### Step 3: Migrate the support-owned integrated-parity contract test

- Task IDs: `[]` (wayfinder ticket 22)
- Objective: switch the support integrated-parity contract test's config map to `support_ironing = true`, proving the key reaches the module through the real host path and that the native and wasm legs agree.
- Precondition: Steps 1-2 pass; the guest artifact for this module has been rebuilt after the manifest/source edits.
- Postcondition: `integrated_parity_support_surface_ironing` passes with the canonical key and still asserts ironing paths on both legs; no top-owned fixture is touched.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-runtime/tests/contract/integrated_parity_support_surface_ironing_tdd.rs` - the config-map construction and the parity spec only.
- Files allowed to edit (at most 3):
  - `crates/slicer-runtime/tests/contract/integrated_parity_support_surface_ironing_tdd.rs`
- Files explicitly out of bounds:
  - `crates/slicer-runtime/tests/contract/integrated_parity_top_surface_ironing_tdd.rs`, `crates/slicer-runtime/tests/executor/cube_4color_ironing_per_painted_top_color_tdd.rs`, `crates/slicer-runtime/tests/e2e/slicing_promotion_e2e_dispatch_regression_tdd.rs`, `resources/test_config/benchy_combined_feature_evidence.json` - all top-owned; packet 266 migrates them.
  - `crates/slicer-integrated-modules/src/lib.rs` - manifest is embedded by path; no edit needed.
  - `OrcaSlicerDocumented/**` - delegate; never load.
- Blast-radius discipline: a config-map key string change only; no struct literal is added.
- Expected sub-agent dispatches:
  - Question: quote the config-map construction and confirm the module id and stage used by this contract test; scope: that file; return: `SNIPPETS` (1, <=20 lines).
  - Question: run the guest freshness check and report its exit code; scope: `cargo xtask build-guests --check`; return: `FACT` exit code.
- Context cost: `S`
- Authoritative docs:
  - `CLAUDE.md` section "Guest WASM Staleness" - freshness is an exit code, never a `STALE:` grep.
- OrcaSlicer refs:
  - None. This step is host-path plumbing with no canonical counterpart to read.
- Verification:
  - `cargo test -p slicer-runtime --test contract integrated_parity_support_surface_ironing 2>&1 | tee target/test-output.log | grep -E '^test result'` - FACT pass/fail.
- Exit condition: the contract test passes on the canonical key, `cargo xtask build-guests --check` exits 0, and `rg -n 'ironing_enabled' crates/slicer-runtime/tests/contract/integrated_parity_support_surface_ironing_tdd.rs` returns nothing.

### Step 4: Regenerate the config reference and close the gates

- Task IDs: `[]` (wayfinder ticket 22)
- Objective: regenerate `docs/15_config_keys_reference.md`, confirm the support-owned `support_ironing` row replaced the `ironing_enabled` row with no new deviation row, and run the workspace gates.
- Precondition: Steps 1-3 pass and all manifest/source edits are final; guests are fresh.
- Postcondition: the generated reference carries `support_ironing` under `support-surface-ironing` and no support-owned `ironing_enabled` row; `gen-config-docs --check` is clean; the deviation block's row count is re-measured and unchanged; `check-literals`, `check`, and `clippy` are green.
- Files allowed to read, with ranges when over 300 lines:
  - `docs/15_config_keys_reference.md` - targeted `rg` only; it is generated and long.
- Files allowed to edit (at most 3):
  - `docs/15_config_keys_reference.md` - **only** as regenerated output of `cargo xtask gen-config-docs`; never hand-edited.
- Files explicitly out of bounds:
  - `docs/ORCA_CONFIG_REFERENCE.md` - hand-maintained upstream snapshot.
  - `xtask/src/gen_config_docs.rs` - the generator itself is not this packet's business.
  - `OrcaSlicerDocumented/**` - delegate; never load.
- Blast-radius discipline: generated documentation only.
- Expected sub-agent dispatches:
  - Question: after regeneration, does the module-key table carry `support_ironing` under `support-surface-ironing` and no support-owned `ironing_enabled` row, and did the deviation block's row count change?; scope: `docs/15_config_keys_reference.md`; return: `FACT` plus the measured row count.
- Context cost: `S`
- Authoritative docs:
  - `docs/15_config_keys_reference.md` - generated output; targeted verification only.
- OrcaSlicer refs:
  - None; the canonical default was captured in `requirements.md` and is compared by the generator against the snapshot's `Default` column.
- Verification:
  - `cargo xtask gen-config-docs && cargo xtask gen-config-docs --check` - FACT exit codes.
  - `cargo xtask check-literals` - FACT exit code.
  - `cargo check --workspace --all-targets` and `cargo clippy --workspace --all-targets -- -D warnings` - FACT pass/fail.
- Exit condition: all four verification commands are green, the AC-4 row assertions hold against the regenerated file, and the deviation row count is reported as a measured number rather than quoted from this packet.

## Closure Ceremony

- Narrow verification: the four steps' commands above, all green.
- Broad verification: `cargo xtask test --summary --workspace` — required only at packet close, per `CLAUDE.md` Test Discipline, and dispatched to a sub-agent with a `FACT pass/fail` return. Do not absorb the full output.
- Ledger updates at closure (re-derive each at the moment of writing, never from this file): the `support_ironing_pattern` row in `docs/specs/orca-feature-gap/issues/04-asset-tier-assignment.md` and the P15 entry in `05-asset-packet-list.md` gain the returned-to-queue note; the map's queue-coverage count drops by one key; `DIV-267-A` graduates to the map's "Not yet specified".
