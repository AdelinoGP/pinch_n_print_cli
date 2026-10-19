# Implementation Plan: config-schema-registry

## Execution Rules

- Work the atomic steps in order; every step maps to `TASK-562` and stays within its own ≤3-file edit list.
- Write falsifying tests before the implementation they exercise, then run the narrowest command that can fail for the named defect.
- Re-derive ledger counts at the step that uses them; do not copy a count into a hand-maintained roster.
- Every cargo command in this plan is wrapped with `set -euo pipefail`, creates `target` before `tee`, and uses `--all-targets` where it invokes Cargo tests/checks/clippy. No step runs a workspace test pipe.
- AC-8's static reader proof must match each repaired key as a real `get_abs_value` method call with a non-empty explicit base argument and a proven result consumer. Its name-resolution-tolerant consumer check must reject a discarded `get_abs_value("bridge_line_width", nozzle_diameter);` statement and `_ = get_abs_value("bridge_line_width", nozzle_diameter);` while accepting a real assignment, `return`, enclosing function/macro argument, or fallback/method chain; receiver names and line breaks remain unconstrained.
- AC-9's generated-row proof must identify the first key cell and final owner cell with a regex, capture raw-pipe enum ranges, and reject malformed, duplicate, or extra-owner target rows without a fixed pipe-count parser.
- AC-10's retirement proof must scope the manifest example, common-field table, cross-validation sections, scheduler wire doc/serialization, source constant/comment, the docs/04 RegionMapping section, and ADR `## Status` independently; a global occurrence must not satisfy a section check.
- AC-2's relocation proof uses `rg -q -x -F` against the exact `pub use` source lines in both `slicer-ir/src/lib.rs` and `slicer-scheduler/src/lib.rs`; scheduler `src/lib.rs` is owned by Step 1b-ii.
- AC-3's source proof parses only the `HostRuntimeKey` body, compares the ordered `(name, normalized type)` list to exactly seven fields, and therefore rejects an extra field while allowing rustfmt whitespace and an optional trailing comma in `denied_scopes`. Its feedrate-channel check accepts module-level `SPEED_KEYS` through either a qualified `slicer_ir::feedrate::SPEED_KEYS` reference or a grouped/unqualified import, rejects associated `FeedrateConfig::SPEED_KEYS`, and retains the exact 26-entry/host-row contract test.
- The direct integration tests are `crates/slicer-config/tests/registry_assembly_tdd.rs` and `registry_census_tdd.rs`; each is its own Cargo test target and needs no `main.rs`/`mod` aggregator.

## Steps

### Step 1a: Move config-schema carriers and add the IR module

- Task IDs: `TASK-562`
- Objective: Move `ConfigFieldEntry` and `ConfigSchema` from scheduler manifest ownership to new `slicer-ir::config_schema` ownership; preserve the current fields temporarily, add serde-defaulted `selector`, `base_key`, and `denied_scopes`, and add the exact flat IR re-export line.
- Precondition: `crates/slicer-scheduler/src/manifest.rs` is the live definition home; `ConfigSchema.entries` is a public `BTreeMap`; the 19-literal census is recorded before editing.
- Postcondition: `crates/slicer-ir/src/config_schema.rs` defines both structs; `slicer-ir/src/lib.rs` declares the module and has exact `pub use config_schema::{ConfigFieldEntry, ConfigSchema};`; scheduler's manifest keeps a `pub use slicer_ir::config_schema::{ConfigFieldEntry, ConfigSchema};`; no duplicate scheduler definitions remain.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-scheduler/src/manifest.rs` — located definition, parser construction, and inline-literal ranges only.
  - `crates/slicer-ir/src/lib.rs` — module and flat-export blocks.
  - `docs/21_data_defaults_and_fixtures.md` — watched-literal rule.
- Files allowed to edit (at most 3):
  - `crates/slicer-ir/src/config_schema.rs` (new)
  - `crates/slicer-ir/src/lib.rs`
  - `crates/slicer-scheduler/src/manifest.rs`
- Files explicitly out of bounds:
  - `crates/slicer-ir/src/slice_ir.rs`, scheduler region-split files, all tests, all docs, `Cargo.toml`, and every other packet directory.
- Blast-radius discipline: preserve `ConfigSchema.entries`; list the 19 construction sites from the pre-step `LOCATIONS` dispatch and use FRU or an explicit waiver for test literals when new fields are introduced. Update all nine construction literals in this manifest file here; the remaining ten literals are owned by Step 1e. Do not remove `validate` yet; its removal is Step 2.
- Expected sub-agent dispatches:
  - Question: which 19 `ConfigFieldEntry` construction sites remain after the move and which name new fields? Scope: `crates/**/*.rs`. Return: `LOCATIONS` ≤20.
- Context cost: `M`
- Authoritative docs:
  - `docs/specs/config-scope-resolution-plan.md` — bounded Crate topology/shared-types section.
  - `docs/21_data_defaults_and_fixtures.md` — direct relevant section.
- OrcaSlicer refs: none.
- Verification:
  - `bash -lc 'set -euo pipefail; mkdir -p target; rg -q "^pub struct ConfigFieldEntry" crates/slicer-ir/src/config_schema.rs; rg -q "^pub struct ConfigSchema" crates/slicer-ir/src/config_schema.rs; rg -q "^pub use config_schema::\\{ConfigFieldEntry, ConfigSchema\\};$" crates/slicer-ir/src/lib.rs; rg -q "^pub use slicer_ir::config_schema::\\{ConfigFieldEntry, ConfigSchema\\};$" crates/slicer-scheduler/src/manifest.rs; cargo check --all-targets -p slicer-ir 2>&1 | tee target/test-output.log >/dev/null; rg -q "Finished" target/test-output.log'` — FACT pass/fail.
- Exit condition: stop if any scheduler definition or exact IR re-export is missing, if a literal cannot compile without an unapproved edit, or if `slicer-ir` gains a dependency.

### Step 1b: Move region-split carriers and retain scheduler aliases

- Task IDs: `TASK-562`
- Objective: Move `AggregatedRegionSplitEntry` and `RegionSplitValueType` into `slicer-ir::slice_ir`, make `RegionSplitDeclaration.value_type` consume the relocated enum, preserve scheduler module aliases, and add the exact IR and scheduler flat aliases only after the relocated symbols exist.
- Precondition: Step 1a's config aliases compile; `RegionSplitValueType` is still defined in `slicer-scheduler/src/manifest.rs` and `AggregatedRegionSplitEntry` in `region_split.rs`.
- Postcondition: IR owns both definitions; `manifest.rs` and `region_split.rs` use/re-export them; exact `pub use slice_ir::{AggregatedRegionSplitEntry, RegionSplitValueType};` exists in `slicer-ir/src/lib.rs`; and `crates/slicer-scheduler/src/lib.rs` contains the exact transitional flat lines `pub use manifest::{ConfigFieldEntry, ConfigSchema, RegionSplitValueType};` and `pub use region_split::AggregatedRegionSplitEntry;`; existing scheduler paths resolve.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-scheduler/src/manifest.rs` — region declaration/enum range.
  - `crates/slicer-scheduler/src/region_split.rs` — definition/import range.
  - `crates/slicer-ir/src/slice_ir.rs` — `ModuleId` and placement neighborhood.
  - `crates/slicer-ir/src/lib.rs` — flat-export block for the `1b-ii` alias.
  - `crates/slicer-scheduler/src/lib.rs` — existing flat-export block; remove the relocated names from its broad manifest export and add the exact transitional lines in `1b-ii`.
- Files allowed to edit (at most 3 per substep):
  - **1b-i:** `crates/slicer-ir/src/slice_ir.rs`, `crates/slicer-scheduler/src/manifest.rs`, `crates/slicer-scheduler/src/region_split.rs`.
  - **1b-ii:** `crates/slicer-ir/src/lib.rs`, `crates/slicer-scheduler/src/lib.rs`.
- Files explicitly out of bounds:
  - core/runtime sources/tests, Cargo manifests, docs, and guest files.
- Blast-radius discipline: keep `AggregatedRegionSplitEntry` exactly `{ priority: u32, value_type: RegionSplitValueType, declaring_modules: Vec<ModuleId> }`; no field change is authorized. `1b-i` relocates the definitions and scheduler module aliases; `1b-ii` adds only the exact IR flat re-export and exact scheduler flat compatibility lines after those definitions are available.
- Expected sub-agent dispatches:
  - Question: verify all existing region-split import paths after the move. Scope: `crates/**/*.rs`. Return: `LOCATIONS` ≤20.
- Context cost: `S`
- Authoritative docs:
  - ADR-0019 — direct Decision/Consequences ranges.
  - `docs/specs/config-scope-resolution-plan.md` — bounded topology section.
- OrcaSlicer refs: none.
- Verification:
  - **1b-i:** `bash -lc 'set -euo pipefail; mkdir -p target; rg -q "^pub enum RegionSplitValueType" crates/slicer-ir/src/slice_ir.rs; rg -q "^pub struct AggregatedRegionSplitEntry" crates/slicer-ir/src/slice_ir.rs; ! rg -q "^pub enum RegionSplitValueType" crates/slicer-scheduler/src/manifest.rs; ! rg -q "^pub struct AggregatedRegionSplitEntry" crates/slicer-scheduler/src/region_split.rs; rg -q -x -F "pub use slicer_ir::slice_ir::RegionSplitValueType;" crates/slicer-scheduler/src/manifest.rs; rg -q -x -F "pub use slicer_ir::slice_ir::AggregatedRegionSplitEntry;" crates/slicer-scheduler/src/region_split.rs; cargo check --all-targets -p slicer-scheduler 2>&1 | tee target/test-output.log >/dev/null; rg -q "Finished" target/test-output.log'` — FACT pass/fail.
  - **1b-ii:** `bash -lc 'set -euo pipefail; mkdir -p target; rg -q -x -F "pub use slice_ir::{AggregatedRegionSplitEntry, RegionSplitValueType};" crates/slicer-ir/src/lib.rs; rg -q -x -F "pub use manifest::{ConfigFieldEntry, ConfigSchema, RegionSplitValueType};" crates/slicer-scheduler/src/lib.rs; rg -q -x -F "pub use region_split::AggregatedRegionSplitEntry;" crates/slicer-scheduler/src/lib.rs; cargo check --all-targets -p slicer-scheduler 2>&1 | tee target/test-output.log >/dev/null; rg -q "Finished" target/test-output.log'` — FACT pass/fail.
- Exit condition: stop if any old definition remains, if `RegionSplitDeclaration.value_type` no longer resolves to the IR enum, if a scheduler alias disappears, or if the `1b-ii` exact flat re-export is missing.

### Step 1c: Relocate the const-safe host runtime channel

- Task IDs: `TASK-562`
- Objective: Move `DEFAULT_WALL_GENERATOR` and the three scheduler runtime rows into `slicer-ir` using the exact seven-field `HostRuntimeKey`, including const-safe `denied_scopes: &'static [&'static str]`; author `wall_generator`'s exact `object`, `layer_range`, `modifier`, `paint_semantic`, and `tool` denials; migrate `build_host_key_entries` to convert static rows into owned `HostConfigKey` values without changing JSON output; preserve the scheduler constant alias.
- Precondition: `HostConfigKey.default` is `Option<String>`; the live scheduler table is the five-tuple `(&str, &str, &str, &str, HostKeyMeta)`; the current default is `"classic"`.
- Postcondition: `resolved_config.rs` contains the exact public seven-field struct and `pub const HOST_RUNTIME_KEYS: &[HostRuntimeKey]`; rows have the exact values, selectors, and denial slices in `packet.spec.md`; `manifest.rs` has no runtime tuple and its loop converts `row.default` to `Some(String)` while preserving `row.denied_scopes` for registry assembly; `execution_plan.rs` re-exports the IR default; the host JSON shape is unchanged and selector/denials are not serialized.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-ir/src/resolved_config.rs` — `HostConfigKey`, `HostKeyMeta`, scope constants, and host declarations.
  - `crates/slicer-scheduler/src/manifest.rs` — runtime tuple and `build_host_key_entries` ranges.
  - `crates/slicer-scheduler/src/execution_plan.rs` — default constant and consumers.
- Files allowed to edit (at most 3):
  - `crates/slicer-ir/src/resolved_config.rs`
  - `crates/slicer-scheduler/src/manifest.rs`
  - `crates/slicer-scheduler/src/execution_plan.rs`
- Files explicitly out of bounds:
  - feedrate, JSON consumers, runtime lib, docs/config host-keys, and all guest/core files.
- Blast-radius discipline: do not add fields to `HostConfigKey` or `HostKeyMeta`; the runtime carrier is a new static struct. Conversion is at channel assembly/host-entry construction, not a const literal of `HostConfigKey`. Only `wall_generator` receives authored denials in this step; packet 7 (`TASK-568`) owns the broad machine/emitter deny-list rollout.
- Expected sub-agent dispatches:
  - Question: run the exact host-row contract check and verify no scheduler runtime tuple remains. Scope: the three listed files. Return: `FACT` ≤5 lines.
- Context cost: `M`
- Authoritative docs:
  - `docs/specs/config-scope-resolution-plan.md` — Registry/channel section.
  - `docs/adr/0067-unified-config-schema-registry.md` — all-host-channel amendment.
- OrcaSlicer refs: none.
- Verification:
  - `bash -lc 'set -euo pipefail; mkdir -p target; rg -q "pub struct HostRuntimeKey" crates/slicer-ir/src/resolved_config.rs; rg -q "pub const HOST_RUNTIME_KEYS: &\\[HostRuntimeKey\\]" crates/slicer-ir/src/resolved_config.rs; ! rg -q -F "const HOST_RUNTIME_KEYS: &[(&str, &str, &str, &str, HostKeyMeta)]" crates/slicer-scheduler/src/manifest.rs; rg -q "Some\\(\\(row|rk\\)\\.default.*to_string" crates/slicer-scheduler/src/manifest.rs; rg -q "pub use slicer_ir::resolved_config::DEFAULT_WALL_GENERATOR" crates/slicer-scheduler/src/execution_plan.rs; cargo check --all-targets -p slicer-scheduler 2>&1 | tee target/test-output.log >/dev/null; rg -q "Finished" target/test-output.log'` — FACT pass/fail.
- Exit condition: stop if the carrier embeds `HostConfigKey`, any row selector/default/scope/denial differs, the scheduler serializes selector or denials, or the host JSON regression is not explained by a deliberate wire edit (none is authorized here).

### Step 1d: Remove the core edge and create the crate shell

- Task IDs: `TASK-562`
- Objective: make `slicer-core` consume the relocated IR region-split type, remove its normal scheduler dependency, register `slicer-config`, and create its minimal crate shell with only `slicer-ir` normal dependency and `toml = "0.8"` dev dependency.
- Precondition: Steps 1a–1c preserve scheduler aliases; the core dependency and production import still point at scheduler.
- Postcondition: `cargo tree -p slicer-core --edges normal` does not contain `slicer-scheduler`; root membership and crate manifest are valid; `slicer-config/src/lib.rs` is a compiling shell that uses the module-path `slicer_ir::config_schema::ConfigSchema` in its eventual `ModuleDeclaration` declaration; no registry stub is claimed complete here.
- Files allowed to read, with ranges when over 300 lines:
  - `Cargo.toml` — workspace members.
  - `crates/slicer-core/Cargo.toml` — dependency block.
  - `crates/slicer-core/src/algos/region_mapping.rs` — import block.
  - `crates/slicer-scheduler/src/lib.rs` — transitional flat exports.
- Files allowed to edit (at most 3) in substeps:
  - **1d-i:** `crates/slicer-core/Cargo.toml`, `crates/slicer-core/src/algos/region_mapping.rs`, `crates/slicer-scheduler/src/lib.rs`.
  - **1d-ii:** `Cargo.toml`, `crates/slicer-config/Cargo.toml` (new), `crates/slicer-config/src/lib.rs` (new).
- Files explicitly out of bounds:
  - all core tests, runtime code, registry tests, scheduler parser, docs, and lockfiles.
- Blast-radius discipline: retain scheduler re-export paths for existing tests; only the production core import is switched in this step. The crate shell includes no fake `assemble_registry` body.
- Expected sub-agent dispatches:
  - Question: run `cargo tree -p slicer-core --edges normal` and identify any remaining scheduler path. Scope: Cargo graph. Return: `FACT` ≤5 lines.
- Context cost: `M`
- Authoritative docs:
  - ADR-0019 — direct closure contract.
  - `docs/specs/config-scope-resolution-plan.md` — crate topology.
- OrcaSlicer refs: none.
- Verification:
  - `bash -lc 'set -euo pipefail; mkdir -p target; cargo tree -p slicer-core --edges normal 2>&1 | tee target/test-output.log >/dev/null; ! rg -q "slicer-scheduler" target/test-output.log; python -c "import tomllib; m=tomllib.load(open(\"crates/slicer-config/Cargo.toml\",\"rb\")); assert set(m.get(\"dependencies\",{})) == {\"slicer-ir\"}; assert m.get(\"dev-dependencies\",{}).get(\"toml\") == \"0.8\""; cargo check --all-targets -p slicer-core -p slicer-config 2>&1 | tee target/test-output.log >/dev/null; rg -q "Finished" target/test-output.log'` — FACT pass/fail.
- Exit condition: stop if the edge remains, the new crate has another normal first-party dependency, or any compatibility alias is removed.

### Step 1e: Complete `ConfigFieldEntry` literal fallout

- Task IDs: `TASK-562`
- Objective: update the ten non-manifest `ConfigFieldEntry` construction literals after the relocated struct gains `selector`, `base_key`, and `denied_scopes`, using FRU or a narrowly justified test waiver without changing test meaning.
- Precondition: Step 1a's relocated struct and the nine manifest literals compile; the pre-step census identifies exactly four literals in scheduler `execution_plan.rs`, four in the three runtime test files, and two in scheduler test files.
- Postcondition: all 19 construction literals across 7 files compile against the new field shape; production literals are explicit, test literals use FRU or an `// exhaustive: <reason>` waiver, and no literal is left for compiler discovery in Step 2.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-scheduler/src/execution_plan.rs` — four located literals.
  - `crates/slicer-runtime/tests/contract/{config_view_binding_tdd.rs,raft_bounds_tdd.rs}` — three located literals.
  - `crates/slicer-runtime/tests/integration/region_mapping_tdd.rs` — one located literal.
  - `crates/slicer-scheduler/tests/{integration/config_resolution_tdd.rs,unit/execution_plan_tdd.rs}` — two located literals.
  - `docs/21_data_defaults_and_fixtures.md` — FRU/waiver rule only.
- Files allowed to edit (at most 3 per substep):
  - **1e-i:** `crates/slicer-scheduler/src/execution_plan.rs`, `crates/slicer-runtime/tests/contract/config_view_binding_tdd.rs`, `crates/slicer-runtime/tests/contract/raft_bounds_tdd.rs`.
  - **1e-ii:** `crates/slicer-runtime/tests/integration/region_mapping_tdd.rs`, `crates/slicer-scheduler/tests/integration/config_resolution_tdd.rs`, `crates/slicer-scheduler/tests/unit/execution_plan_tdd.rs`.
- Files explicitly out of bounds:
  - all other source/tests, the struct definition, parser semantics, `validate` retirement, manifests, docs, generated output, and every other packet directory.
- Blast-radius discipline: do not change assertions or fixture values; add only the new-field FRU/waiver handling required by the watched-literal rule. Reconfirm the manifest's nine literals from Step 1a rather than duplicating them here.
- Expected sub-agent dispatches:
  - Question: verify the ten named non-manifest literal sites use FRU or a justified waiver and report any omitted site. Scope: the six files listed above. Return: `LOCATIONS` ≤20.
- Context cost: `S`
- Authoritative docs:
  - `docs/21_data_defaults_and_fixtures.md` — watched-literal rule.
- OrcaSlicer refs: none.
- Verification:
  - `bash -lc 'set -euo pipefail; mkdir -p target; cargo check --all-targets -p slicer-scheduler -p slicer-runtime 2>&1 | tee target/test-output.log >/dev/null; rg -q "Finished" target/test-output.log'` — FACT pass/fail.
- Exit condition: stop if any of the ten sites is missing, a production literal becomes non-explicit, a test waiver lacks a reason, or compilation still discovers an unlisted watched literal.

### Step 2: Retire `validate` and bump the CLI wire version

- Task IDs: `TASK-562`
- Objective: remove `ConfigFieldEntry.validate`, its parser and JSON/wire-comment emission, update the three validate-bearing inline assertions, and change `CONFIG_SCHEMA_WIRE_VERSION` from `"1.2.0"` to `"1.3.0"` with the literal-pinned version assertion in the same edit.
- Precondition: the final definition lives in `slicer-ir/src/config_schema.rs`; Steps 1a and 1e have completed the 19-literal fallout across 7 files; the live wire constant is `"1.2.0"`.
- Postcondition: no `validate` field/parser/JSON key/wire example remains in the config-schema path; all 19 literals compile under the FRU/waiver rule; manifest wire tests assert `"1.3.0"`; runtime wiring remains constant-relative.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-ir/src/config_schema.rs` — final field definition.
  - `crates/slicer-scheduler/src/manifest.rs` — parser, JSON emitter, wire doc, and inline tests at located ranges.
  - `crates/slicer-runtime/tests/integration/runtime_wiring_tdd.rs` — config-schema assertions only.
- Files allowed to edit (at most 3):
  - `crates/slicer-ir/src/config_schema.rs`
  - `crates/slicer-scheduler/src/manifest.rs`
  - `crates/slicer-runtime/tests/integration/runtime_wiring_tdd.rs` only if a literal version pin is discovered.
- Files explicitly out of bounds:
  - docs (Step 6), `VALID_SEVERITIES` (Step 6a), pnp-cli version literals, module-version fixtures, `Cargo.lock`, and all other tests.
- Blast-radius discipline: the 19 construction literals are the known set: scheduler manifest (9 including 2 production, handled in Step 1a), scheduler execution plan (4), runtime contract `config_view_binding_tdd` (2), runtime contract `raft_bounds_tdd` (1), runtime integration `region_mapping_tdd` (1), scheduler integration `config_resolution_tdd` (1), and scheduler unit `execution_plan_tdd` (1), with the ten non-manifest sites handled in Step 1e. Only the three manifest inline sites naming `validate` require semantic edits; all other test literals use FRU/waivers. The runtime wiring test is confirm-only unless a literal version pin is found.
- Expected sub-agent dispatches:
  - Question: run AC-7's exact retirement command and the runtime wire test. Scope: those targets. Return: `FACT` pass/fail plus ≤20 failure lines.
- Context cost: `S`
- Authoritative docs:
  - `docs/specs/config-scope-resolution-plan.md` — Owner decision 1.
  - `docs/11_operational_governance_and_acceptance_gate.md` — CLI wire policy and exception.
- OrcaSlicer refs: none.
- Verification:
  - `bash -lc 'set -euo pipefail; mkdir -p target; ! rg -q "^\\s*pub validate:" crates/slicer-ir/src/config_schema.rs; ! rg -q "get_string_opt\\(table, \"validate\"\\)|\"validate\":" crates/slicer-scheduler/src/manifest.rs; rg -q "CONFIG_SCHEMA_WIRE_VERSION: &str = \"1\\.3\\.0\"" crates/slicer-scheduler/src/manifest.rs; cargo test --all-targets -p slicer-scheduler --lib config_schema -- --nocapture 2>&1 | tee target/test-output.log >/dev/null; rg -q "test result: ok" target/test-output.log; cargo test -p slicer-runtime --test integration runtime_wiring -- --nocapture 2>&1 | tee target/test-output.log >/dev/null; rg -q "test result: ok" target/test-output.log'` — FACT pass/fail.
- Exit condition: stop if any literal-bearing file fails to compile, if a stale `validate` JSON/document example remains, or if the version bump is not accompanied by its inline assertion update.

### Step 3a: Implement the registry contract and reconciliation tests

- Task IDs: `TASK-562`
- Objective: implement the exact `slicer-config` API and `assemble_registry`, then author behavior-first tests for four-channel reconciliation, warnings, host and module selector metadata propagation, structural selector validation, `wall_generator`'s valid five-scope denial policy, base-key validation, enum values, denied scopes, UI metadata, and provenance.
- Precondition: steps 1–2 expose all carriers, `HostRuntimeKey` rows, and final `ConfigFieldEntry` fields; `slicer-config` has `toml = "0.8"` dev dependency.
- Postcondition: `ConfigSchemaRegistry`, `RegistryEntry`, `ModuleKeyMeta`, `ModuleDeclaration`, `HostChannels`, `AssemblyOutcome`, `RegistryWarning`, `assemble_registry`, and `RegistryLoadError` have the exact shapes in `design.md`; synthetic tests independently assert host runtime selector propagation and its retained five-scope denial policy, reject selector rows with any statable narrower scope, and reject every in-scope AC-N path.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-ir/src/{config_schema.rs,resolved_config.rs,feedrate.rs}` — public field/carrier ranges.
  - `crates/slicer-schema/src/lib.rs` — `VALID_CONFIG_TYPES` vocabulary.
  - ADR-0067/0068/0069 — reconciliation and Phase A constraints.
  - `docs/22_test_quality.md` and `docs/21_data_defaults_and_fixtures.md` — test/literal rules.
- Files allowed to edit (at most 3):
  - `crates/slicer-config/src/lib.rs`
  - `crates/slicer-config/tests/registry_assembly_tdd.rs` (new direct target)
  - `crates/slicer-config/tests/registry_census_tdd.rs` (new direct target; census body begins here)
- Files explicitly out of bounds:
  - scheduler parser (Step 3b), manifests/readers/docs, source declarations, and all existing test aggregators.
- Blast-radius discipline: every new test literal of a watched type uses FRU or an inline `// exhaustive: <reason>` waiver; synthetic expectations do not copy production assembly output. The `SelectorStatablePerRegion` case is exercised with a module declaration whose `denied_scopes` representation leaves at least one of `object`, `layer_range`, `modifier`, `paint_semantic`, or `tool` statable (an empty/default list means all scopes under ADR-0069). The `wall_generator` fixture asserts the exact five denials and a successful selector validation. Packet 7 (`TASK-568`) owns only the later broad machine/emitter deny-list rollout.
- Expected sub-agent dispatches:
  - Question: run both new test targets and report discovery plus result lines. Scope: `crates/slicer-config/tests/**`. Return: `FACT` pass/fail + ≤20 failure lines.
- Context cost: `M`
- Authoritative docs:
  - ADR-0067, ADR-0068, ADR-0069 — direct relevant clauses.
  - `docs/22_test_quality.md` §2.4/§4 — independent census rule.
- OrcaSlicer refs: none; canonical parity is Step 5a's delegated read.
- Verification:
  - `bash -lc 'set -euo pipefail; mkdir -p target; cargo test --all-targets -p slicer-config --test registry_assembly_tdd -- --list 2>&1 | tee target/test-output.log >/dev/null; rg -q ": test$" target/test-output.log; cargo test --all-targets -p slicer-config --test registry_assembly_tdd -- --nocapture 2>&1 | tee target/test-output.log >/dev/null; rg -q "test result: ok" target/test-output.log'` — FACT pass/fail.
- Exit condition: stop if any public symbol differs from `design.md`, if a rejection test can pass without driving its named error, or if the census test contains a hand-listed production-key roster.

### Step 3b: Extend manifest parsing for registry fields

- Task IDs: `TASK-562`
- Objective: parse optional `selector`, `base_key`, and `denied_scopes` from each full `[config.schema.<key>]` table into the relocated `ConfigFieldEntry`, preserving shorthand/default behavior and rejecting malformed types through the existing `LoadError` path.
- Precondition: Step 3a's final field shape compiles and `read_config_schema` still parses only the pre-existing fields.
- Postcondition: real module manifests populate the three fields with serde-compatible defaults; the parser does not reintroduce `validate`; registry assembly can consume `ConfigSchema` from a loaded manifest without a scheduler-owned type.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-scheduler/src/manifest.rs` — `read_config_schema` and `parse_config_field_entry` ranges.
  - `crates/slicer-ir/src/config_schema.rs` — final fields.
  - `docs/03_wit_and_manifest.md` — field vocabulary only.
- Files allowed to edit (at most 3):
  - `crates/slicer-scheduler/src/manifest.rs`
- Files explicitly out of bounds:
  - all docs, test files, manifests, registry implementation, and generated output.
- Blast-radius discipline: use `..Default::default()` or explicit production fields as appropriate; no test struct-literal sweep is deferred from Step 2.
- Expected sub-agent dispatches:
  - Question: feed a synthetic parsed TOML table through the parser and report the three resulting fields. Scope: scheduler manifest tests/target. Return: `FACT` ≤5 lines.
- Context cost: `S`
- Authoritative docs:
  - `docs/specs/config-scope-resolution-plan.md` — selector/base-key field decision.
  - `docs/03_wit_and_manifest.md` — bounded common-field section.
- OrcaSlicer refs: none.
- Verification:
  - `bash -lc 'set -euo pipefail; mkdir -p target; rg -q "selector" crates/slicer-scheduler/src/manifest.rs; rg -q "base_key" crates/slicer-scheduler/src/manifest.rs; rg -q "denied_scopes" crates/slicer-scheduler/src/manifest.rs; ! rg -q "get_string_opt\\(table, \"validate\"\\)" crates/slicer-scheduler/src/manifest.rs; cargo test --all-targets -p slicer-scheduler --lib config_schema -- --nocapture 2>&1 | tee target/test-output.log >/dev/null; rg -q "test result: ok" target/test-output.log'` — FACT pass/fail.
- Exit condition: stop if any optional field is required for old manifests, is parsed under a wrong snake_case key, or leaks `validate` back into the type/wire path.

### Step 4: Run the channel-derived census against real manifests

- Task IDs: `TASK-562`
- Objective: finish `registry_census_tdd.rs` as an independent oracle deriving all expected keys from all four channels and real TOML tables, retaining wildcard keys and checking exact host-row selectors.
- Precondition: Steps 3a–3b assemble parsed `ConfigSchema` values and expose `HostChannels::from_live`; the 270/179/54 ledger has been independently re-derived.
- Postcondition: census tests fail if a declaration is omitted or an extra registry key appears; they do not carry a hand-maintained key roster; real parse diagnostics reproduce 270 entries, 179 distinct, 54 multi-declared, and the two wildcard keys.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-config/src/lib.rs` — public assembly API.
  - `crates/slicer-ir/src/{resolved_config.rs,feedrate.rs}` — channel constructors.
  - `crates/slicer-scheduler/src/manifest.rs` — parse semantics.
  - `xtask/src/gen_config_docs.rs` — same TOML path/name and table parse.
  - all 24 `<name>/<name>.toml` files by parser, not full text reads.
- Files allowed to edit (at most 3):
  - `crates/slicer-config/tests/registry_census_tdd.rs`
- Files explicitly out of bounds:
  - any source implementation, generated docs, `Cargo.lock`, target artifacts, and other tests.
- Blast-radius discipline: counts are diagnostics derived at runtime; expected membership is a set union built from channels, not an array of key strings.
- Expected sub-agent dispatches:
  - Question: run the census with `--list` and `--nocapture`, confirming nonzero discovery and all three union tests. Scope: one test binary. Return: `FACT`.
- Context cost: `M`
- Authoritative docs:
  - `docs/22_test_quality.md` §2.4/§4.
  - `docs/specs/config-scope-resolution-plan.md` — census requirement.
- OrcaSlicer refs: none.
- Verification:
  - AC-4's complete command, including independent TOML count assertions, returns pass.
- Exit condition: stop if the test can remain green after deleting a channel declaration, if a wildcard is filtered from the registry, or if any expected set is hand-maintained.

### Step 5a: Repair manifest type declarations

- Task IDs: `TASK-562`
- Objective: apply the exact `bridge_line_width`, `initial_layer_line_width`, and `support_style` declarations to the real manifests, with no wave initial-layer declaration.
- Precondition: Step 4 census passes and the delegated canonical parity read confirms the three repair directions.
- Postcondition: six bridge owners have `float_or_percent` plus `base_key`; five initial owners have `float_or_percent` plus `base_key`; classic gains only the missing base key for the initial key and already has the type; traditional support has the exact enum values; tree-support-planner is unchanged.
- Files allowed to read, with ranges when over 300 lines:
  - the seven named module TOMLs — only the three schema tables.
  - `docs/specs/config-scope-resolution-plan.md` — Type conflicts range.
  - delegated Orca path — worker only.
- Files allowed to edit (at most 3):
  - **5a-i:** `modules/core-modules/arachne-perimeters/arachne-perimeters.toml`, `classic-perimeters/classic-perimeters.toml`, `gyroid-infill/gyroid-infill.toml`.
  - **5a-ii:** `modules/core-modules/lightning-infill/lightning-infill.toml`, `rectilinear-infill/rectilinear-infill.toml`, `wave-overhangs/wave-overhangs.toml`.
  - **5a-iii:** `modules/core-modules/traditional-support/traditional-support.toml`.
- Files explicitly out of bounds:
  - `tree-support-planner.toml`, all other manifests, all source readers, docs, and generated output.
- Blast-radius discipline: do not alter defaults/ranges/UI metadata; the only classic initial-layer edit is `base_key = "nozzle_diameter"`.
- Expected sub-agent dispatches:
  - Question: parse the seven target schema tables and report exact owner/type/base/value tuples. Scope: those TOMLs. Return: `FACT` ≤5 lines.
  - Question: inspect the canonical three `PrintConfig.cpp` add calls. Scope: one Orca file. Return: `LOCATIONS` ≤20.
- Context cost: `S`
- Authoritative docs:
  - `docs/specs/config-scope-resolution-plan.md` — Type conflicts.
  - ADR-0067 — strict type/enum agreement.
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` — delegated canonical width/enum declarations.
- Verification:
  - `bash -lc 'set -euo pipefail; mkdir -p target; python -c "import pathlib,tomllib; root=pathlib.Path(\"modules/core-modules\"); initial={\"arachne-perimeters\",\"classic-perimeters\",\"gyroid-infill\",\"lightning-infill\",\"rectilinear-infill\"}; bridge=initial|{\"wave-overhangs\"}; get=lambda n: tomllib.loads((root/n/(n+\".toml\")).read_text(encoding=\"utf-8\"))[\"config\"][\"schema\"]; assert all(get(n)[\"bridge_line_width\"][\"type\"]==\"float_or_percent\" and get(n)[\"bridge_line_width\"][\"base_key\"]==\"nozzle_diameter\" for n in bridge); assert all(get(n)[\"initial_layer_line_width\"][\"type\"]==\"float_or_percent\" and get(n)[\"initial_layer_line_width\"][\"base_key\"]==\"nozzle_diameter\" for n in initial); assert \"initial_layer_line_width\" not in get(\"wave-overhangs\")"'` — FACT pass/fail.
- Exit condition: stop if a module outside the exact set is edited, if wave gains an initial-layer declaration, or if any required base key/value/domain is absent.

### Step 5b: Migrate the percent-intolerant readers

- Task IDs: `TASK-562`
- Objective: replace scalar/float-only reads with `get_abs_value` semantics for exactly the required key/file pairs, leaving the classic control untouched but verified. The static gate checks each key literal as the first call argument, requires a non-empty explicit base as the second argument, and proves result consumption without assuming a receiver variable name: discarded semicolon statements and `_ = get_abs_value("bridge_line_width", nozzle_diameter);` fail, while assignments, returns, enclosing function/macro arguments, and fallback/method chains pass.
- Precondition: Step 5a manifests parse as compatible; reader census identifies no additional percent-intolerant production site.
- Postcondition: host bridge read, four guest dual-key reads, and wave bridge read each resolve percent and absolute inputs through the proper base; classic remains dual-key `get_abs_value`; every required key/file pair is independently proven by the method-call/base/result-consumer gate, including its discarded-call negative probe; guest artifacts are rebuilt/fresh.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-runtime/src/layer_executor.rs` — bridge closure.
  - five guest `src/lib.rs` files — named width reads only.
  - classic guest `src/lib.rs` — control reads only.
  - `crates/slicer-ir/src/slice_ir.rs` and paint-segmentation reference — `get_abs_value` semantics.
- Files allowed to edit (at most 3):
  - **5b-i:** `crates/slicer-runtime/src/layer_executor.rs`, `modules/core-modules/arachne-perimeters/src/lib.rs`, `modules/core-modules/gyroid-infill/src/lib.rs`.
  - **5b-ii:** `modules/core-modules/lightning-infill/src/lib.rs`, `modules/core-modules/rectilinear-infill/src/lib.rs`, `modules/core-modules/wave-overhangs/src/lib.rs`.
- Files explicitly out of bounds:
  - classic guest source, paint-segmentation, core flow/IR fields, all other modules, manifests, docs, and WIT.
- Blast-radius discipline: retain absolute-value behavior and existing defaults; do not widen `ResolvedConfig` scalar fields.
- Expected sub-agent dispatches:
  - Question: classify each width read and confirm the required key/file map. Scope: the seven host/guest files. Return: `LOCATIONS` ≤20.
  - Question: run `cargo xtask build-guests` then `cargo xtask build-guests --check`. Scope: exact commands. Return: `FACT` with both exit codes.
- Context cost: `M`
- Authoritative docs:
  - `docs/specs/config-scope-resolution-plan.md` — Type conflicts and Phase A/B boundary.
  - `docs/03_wit_and_manifest.md` — `float_or_percent` read semantics.
- OrcaSlicer refs: none beyond Step 5a.
- Verification:
  - `bash -lc 'set -euo pipefail; mkdir -p target; python -c "from pathlib import Path; import re; expected={\"crates/slicer-runtime/src/layer_executor.rs\":[\"bridge_line_width\"],\"modules/core-modules/arachne-perimeters/src/lib.rs\":[\"bridge_line_width\",\"initial_layer_line_width\"],\"modules/core-modules/gyroid-infill/src/lib.rs\":[\"bridge_line_width\",\"initial_layer_line_width\"],\"modules/core-modules/lightning-infill/src/lib.rs\":[\"bridge_line_width\",\"initial_layer_line_width\"],\"modules/core-modules/rectilinear-infill/src/lib.rs\":[\"bridge_line_width\",\"initial_layer_line_width\"],\"modules/core-modules/wave-overhangs/src/lib.rs\":[\"bridge_line_width\"],\"modules/core-modules/classic-perimeters/src/lib.rs\":[\"bridge_line_width\",\"initial_layer_line_width\"]}; texts={p:Path(p).read_text(encoding=\"utf-8\") for p in expected}; call=re.compile(r\"(?s)(?P<receiver>[A-Za-z_][A-Za-z0-9_]*(?:\s*(?:\.|::)\s*[A-Za-z_][A-Za-z0-9_]*)*)\s*\.\s*get_abs_value\s*\(\s*\"+chr(34)+r\"(?P<key>bridge_line_width|initial_layer_line_width)\"+chr(34)+r\"\s*,\s*(?P<base>(?:[^()]|\([^()]*\))+?)\s*\)\"); assignment=re.compile(r\"(?:\blet\s+(?:mut\s+)?(?!_)[A-Za-z_][A-Za-z0-9_]*(?:\s*:\s*[^=\n]+)?|(?!_)[A-Za-z_][A-Za-z0-9_]*(?:\s*\.\s*[A-Za-z_][A-Za-z0-9_]*)*)\s*(?<![=!<>])=(?!=)\s*(?:[^;\n]*?)$\",re.S); argument=re.compile(r\"(?:[A-Za-z_][A-Za-z0-9_]*(?:\s*(?:::|\.)\s*[A-Za-z_][A-Za-z0-9_]*)*!?)\s*\([^;{}]*$\",re.S); boundary=lambda t,i:max(t.rfind(\";\",0,i),t.rfind(\"{\",0,i),t.rfind(\"}\",0,i))+1; consumed=lambda m,t: bool(re.search(r\"\breturn\b\",t[boundary(t,m.start()):m.start()])) or bool(assignment.search(t[boundary(t,m.start()):m.start()])) or bool(re.match(r\"\s*(?:\?|\.|[+\-*/%])\",t[m.end():])) or (bool(re.match(r\"\s*[,)]\",t[m.end():])) and bool(argument.search(t[boundary(t,m.start()):m.start()]))); usable=lambda b: bool(b.strip()) and b.strip() not in {chr(34)+chr(34),chr(39)+chr(39)}; probe=\"cfg.get_abs_value(\"+chr(34)+\"bridge_line_width\"+chr(34)+\", nozzle);\"; assert not consumed(next(call.finditer(probe)),probe); samples=(\"let width = cfg.get_abs_value(\"+chr(34)+\"bridge_line_width\"+chr(34)+\", nozzle);\",\"return cfg.get_abs_value(\"+chr(34)+\"bridge_line_width\"+chr(34)+\", nozzle);\",\"consume(cfg.get_abs_value(\"+chr(34)+\"bridge_line_width\"+chr(34)+\", nozzle));\"); assert all(consumed(next(call.finditer(s)),s) for s in samples); missing=[f\"{p}:{k}\" for p,ks in expected.items() for k in ks if not any(usable(m.group(\"base\")) and consumed(m,texts[p]) for m in call.finditer(texts[p]) if m.group(\"key\")==k)]; assert not missing, f\"missing exact get_abs_value(key, base) reads with a consumed result: {missing}\""; cargo xtask build-guests 2>&1 | tee target/test-output.log >/dev/null; cargo xtask build-guests --check 2>&1 | tee target/test-output.log >/dev/null' ` — FACT pass/fail.
- Exit condition: stop if any required key pair is not independently checked, if a call lacks its explicit base or result consumer, if the static negative probe would accept `get_abs_value("bridge_line_width", nozzle_diameter);` or `_ = get_abs_value("bridge_line_width", nozzle_diameter);`, if a percent literal can still be dropped, if an extra site appears, or if freshness returns nonzero (rebuild before diagnosis).

### Step 6: Complete docs, `VALID_SEVERITIES` retirement, generated output, and ADR closure

- Task IDs: `TASK-562`
- Objective: update the intended manifest example, common-field, cross-validation, SDK-diagnose, host-scheduler RegionMapping threading, scheduler-wire, generated-doc, source-constant/comment, and ADR-0019 status sections; replace the stale docs/04 sentence that leaves the `slicer-core` → `slicer-scheduler` edge deferred; remove the orphaned `VALID_SEVERITIES` code/comment in its permitted step, regenerate docs/15, and amend ADR-0019 status to Closed.
- Precondition: Steps 1–5 pass; repaired manifests and readers are compiled; generated-doc source `render_table` has been read and no manual generated-block edit is planned.
- Postcondition: AC-9/AC-10 pass; the `## Module Manifest Schema (TOML)` example has no retired validate assignment; docs/03 common-field section contains exactly the three new keys and no validate row; both cross-validation sections and their examples are gone; the SDK diagnose line, scheduler wire doc/serialization/parser, `VALID_SEVERITIES` constant, and source comment are gone; the bounded docs/04 `RegionMapping (Builtin) — aggregated_region_split Threading` section contains the exact replacement sentence `**Cross-crate dependency:** \`AggregatedRegionSplitEntry\` is owned by \`slicer-ir::slice_ir\`; \`slicer-core\` imports it from \`slicer-ir\` and no longer has a normal \`slicer-scheduler\` dependency.`, with the stale deferred paragraph absent; generated rows match actual formatter output; ADR-0019's scoped `## Status` body is Closed; no scheduler edge remains.
- Files allowed to read, with ranges when over 300 lines:
  - `docs/03_wit_and_manifest.md` — `## Module Manifest Schema (TOML)`, common per-field table, cross-field validation example, and `## Validation Expression Language` ranges.
  - `docs/04_host_scheduler.md` — the `### RegionMapping (Builtin) — \`aggregated_region_split\` Threading` section through its next `###` heading only.
  - `docs/05_module_sdk.md` — `pnp_cli module diagnose` `Checks` range.
  - `crates/slicer-scheduler/src/manifest.rs` — config-schema wire doc, parser, and JSON serialization ranges.
  - `crates/slicer-ir/src/config_schema.rs` — retired field check.
  - `crates/slicer-schema/src/lib.rs` — `VALID_SEVERITIES` constant and source comment range only.
  - ADR-0019 — `## Status` range; future-reviewer rationale is retained unless the status closure requires its stale wording to be amended.
  - `xtask/src/gen_config_docs.rs` — `render_table` range.
- Files allowed to edit (at most 3):
  - **6a:** `docs/03_wit_and_manifest.md`, `docs/05_module_sdk.md`, `crates/slicer-schema/src/lib.rs` (`VALID_SEVERITIES` only).
  - **6b:** `docs/04_host_scheduler.md` (the bounded RegionMapping threading section only), `docs/adr/0019-aggregated-region-split-entry-cross-crate-dependency.md`.
  - `docs/15_config_keys_reference.md` is modified only by the `cargo xtask gen-config-docs` tool run, never by hand.
- Files explicitly out of bounds:
  - `docs/02_ir_schemas.md`, plan/backlog files, other ADRs, generator source, WIT, all unrelated code, and all generated files except tool output.
- Blast-radius discipline: the `VALID_SEVERITIES` removal is an authorized code edit in 6a, not an out-of-bounds docs-only convenience; 6b may replace only the stale cross-crate paragraph inside the named docs/04 section and the ADR status/closure note; run compile/lint after the code edit.
- Expected sub-agent dispatches:
  - Question: run `cargo xtask gen-config-docs` and `--check`, extract the bridge/initial/support rows, and report exit codes. Scope: exact commands plus docs/15 rows. Return: `FACT`.
  - Question: inspect only the IR-versioning section needed to ensure docs edits introduce no IR-version claim. Scope: docs/02 range. Return: `SUMMARY` ≤200 words.
- Context cost: `S`
- Authoritative docs:
  - docs/03, docs/04, docs/05, ADR-0019 — edited surfaces.
  - docs/11 — version/compatibility policy.
- OrcaSlicer refs: none.
- Verification:
  - AC-9's generation/row command (the first-key/final-owner regex with raw-pipe range capture and exact owner/type/default/range rows) and AC-10's section-scoped doc/ADR/dependency command both pass; AC-10 extracts docs/04's named section up to the next `###` heading, rejects the old deferred core→scheduler paragraph, and requires the relocated IR ownership plus removed normal edge wording; do not substitute a fixed pipe-count parser or a global validate grep.
  - `bash -lc 'set -euo pipefail; mkdir -p target; cargo check --all-targets --workspace 2>&1 | tee target/test-output.log >/dev/null; rg -q "Finished" target/test-output.log'` — FACT pass/fail.
- Exit condition: stop if generated output was hand-edited, either docs/03 or docs/04 section-scoped proof would pass only because of a global occurrence, the docs/04 stale deferred paragraph remains, `VALID_SEVERITIES` removal has another consumer, or ADR-0019 changes beyond its status/closure note.

## Per-Step Budget Roll-Up

| Step | Context cost | Notes |
| --- | --- | --- |
| 1a | M | Config carrier move plus literal census. |
| 1b | S | Region-split definitions and aliases. |
| 1c | M | Const-safe host carrier and scheduler consumer. |
| 1d | M | Core edge plus crate shell, split into two ≤3-file substeps. |
| 1e | S | Remaining ten `ConfigFieldEntry` literal sites, split into two ≤3-file substeps. |
| 2 | S | Validate retirement and version assertion fallout. |
| 3a | M | Registry API and behavior-first tests. |
| 3b | S | Manifest parser fields. |
| 4 | M | Independent four-channel census and real TOML parse. |
| 5a | S | Seven manifest declarations split into three edit groups. |
| 5b | M | Six migrations plus guest freshness. |
| 6 | S | Docs (including the bounded docs/04 RegionMapping threading section), permitted code retirement, generated output, ADR closure. |

Aggregate remains `M`; no step is `L`. Split any substep immediately if an unlisted file becomes necessary.

## Packet Completion Gate

- All steps and falsifying exits pass; every pipe-suffixed AC command returns pass.
- `cargo check --workspace --all-targets`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo xtask check-literals`, and the touched-scope `cargo xtask check-test-quality --report` pass.
- `cargo xtask build-guests --check` exits 0 and `cargo xtask gen-config-docs --check` exits 0.
- Later implementation updates the TASK-562 backlog row only through the authorized worker process; this packet never edits `docs/07_implementation_status.md`.
- The final registry API and exact host carrier match `design.md`; no later queue row is marked complete by this packet.

## Acceptance Ceremony

- Re-dispatch every AC command and packet gate as bounded FACT checks.
- Record final measured channel-union/registry counts and warning counts as implementation evidence, not as a hand-maintained census oracle.
- Confirm guest freshness by exit code, generated docs by `--check`, and context use within the standard budget; if not, record the escalation rather than weakening a gate.
