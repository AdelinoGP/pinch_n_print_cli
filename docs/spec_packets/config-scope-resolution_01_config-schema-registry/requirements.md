# Requirements: config-schema-registry

## Packet Metadata

- Grouped task IDs: `TASK-562`
- Backlog source: `docs/07_implementation_status.md`; the orchestrator owns the registered row and this packet does not edit it.
- Packet status: `implemented`
- Aggregate context cost: `M`
- Approved scope: `docs/specs/config-scope-resolution-plan.md` Packet Queue row 1 only.

## Problem Statement

Schema declarations currently live in four unjoined channels: `ResolvedConfig::host_config_keys()`, module-level `slicer_ir::feedrate::SPEED_KEYS`, the scheduler's runtime-key tuple, and parsed module manifests. The shared `ConfigFieldEntry`/`ConfigSchema` definitions and `AggregatedRegionSplitEntry` also sit above the IR layer, while the scheduler still carries a dead `validate` field and the registry has no typed selector, base-key, enum, deny-list, or provenance reconciliation. The row-1 fix is coherent because the registry cannot be assembled until its carriers and host channels have stable homes, and the three manifest conflicts must be repaired before strict assembly can load the real declarations. The host selector row must also carry its explicit denial policy before selector validation can be truthful.

This is a ground-up correction of the prior draft. The live TOML parse is 270 entries / 179 distinct keys / 54 multi-declared keys, including only `object_height:*` and `layer_height:*` as single-declarer wildcard keys. The `ConfigFieldEntry` literal census is 19 construction literals across 7 files, excluding the struct definition. These counts guide discovery; the census implementation must derive its expected set from the channels, never from a hand-maintained roster.

## In Scope

- Relocate `ConfigFieldEntry` and `ConfigSchema` to `slicer-ir::config_schema`, with `slicer-ir`'s exact flat `pub use config_schema::{ConfigFieldEntry, ConfigSchema};` line and scheduler transitional aliases. The scheduler crate's flat compatibility surface is pinned to the exact `pub use manifest::{ConfigFieldEntry, ConfigSchema, RegionSplitValueType};` and `pub use region_split::AggregatedRegionSplitEntry;` lines in `crates/slicer-scheduler/src/lib.rs`. `ConfigSchema` in the new API is always spelled `slicer_ir::config_schema::ConfigSchema`; the flat `slicer_ir::ConfigSchema` path remains an intentional compatibility alias.
- Relocate `AggregatedRegionSplitEntry` and `RegionSplitValueType` to `slicer-ir::slice_ir`, keep scheduler module and flat re-exports, switch the pure `slicer-core` consumer, and remove its normal `slicer-scheduler` dependency. Close ADR-0019 only after the dependency edge is gone.
- Replace the scheduler runtime tuple with the exact const-safe carrier `pub struct HostRuntimeKey { pub key: &'static str, pub field_type: &'static str, pub scope: &'static str, pub default: &'static str, pub meta: HostKeyMeta, pub selector: bool, pub denied_scopes: &'static [&'static str] }` and `pub const HOST_RUNTIME_KEYS: &[HostRuntimeKey]`. The rows are `use_relative_e_distances` (`bool`, `printer`, `true`, `HostKeyMeta::NONE`, `false`, `[]`), `thumbnail_path` (`string`, `printer`, empty, existing metadata, `false`, `[]`), and `wall_generator` (`string`, `print`, `classic`, `HostKeyMeta::NONE`, `true`, `["object", "layer_range", "modifier", "paint_semantic", "tool"]`). Those are the exact per-region denials for the whole-print selector; the plan's order is `global/print < object < layer_range < modifier < paint_semantic < tool`, so `global`/`print` remains allowed. Assembly converts the static default into `HostConfigKey.default: Option<String>` and copies the static denial slice into `RegistryEntry.denied_scopes`; `HostRuntimeKey` does not contain `HostConfigKey` because `HostConfigKey.default` is an owned `Option<String>` and is not const-friendly.
- Keep `slicer_ir::feedrate::SPEED_KEYS` at its module-level declaration `pub const SPEED_KEYS: &[(&str, fn(&mut FeedrateConfig) -> &mut f32)]` with 26 entries; no type-associated `SPEED_KEYS` API is introduced or named.
- Add `slicer-config` as a workspace member with only `slicer-ir` in normal dependencies and `toml = "0.8"` in dev dependencies. Its exact public contract is `ConfigSchemaRegistry`, `RegistryEntry`, `ModuleKeyMeta`, `ModuleDeclaration`, `HostChannels`, `AssemblyOutcome`, `RegistryWarning`, `assemble_registry`, and `RegistryLoadError` as pinned in `design.md`.
- Assemble the four channels plus `ModuleDeclaration` schemas under type agreement, enum-domain agreement, reported bounds intersection, host-then-alphabetical default selection, claim-exclusive divergence warnings, denied-scope union, advisory UI metadata precedence, module-default warnings for host-owned keys, selector metadata propagation plus structural validation of both host and represented-module eligibility policies, base-key existence/type/cycle validation, and sorted provenance. The `wall_generator` runtime selector is valid in packet 1 because its static denial slice explicitly denies `object`, `layer_range`, `modifier`, `paint_semantic`, and `tool`. Packet 7 (`TASK-568`) remains owner of the broad machine/emitter deny-list rollout for other host and module keys.
- Parse optional manifest `selector`, `base_key`, and `denied_scopes` fields in `read_config_schema`, with serde-compatible defaults. Denied-scope vocabulary is exactly `global`, `object`, `layer_range`, `modifier`, `paint_semantic`, and `tool`.
- Remove `ConfigFieldEntry.validate` and its JSON/doc-comment emission; bump `CONFIG_SCHEMA_WIRE_VERSION` from `"1.2.0"` to `"1.3.0"` under owner decision 1. Edit the three validate-bearing inline test assertions and the literal-pinned wire-version assertion in the same step; the runtime wiring test is confirm-only unless a literal is found.
- Repair the real manifest declarations: `bridge_line_width` in six manifests (including `wave-overhangs`) becomes `float_or_percent` with `base_key = "nozzle_diameter"`; `initial_layer_line_width` in exactly five manifests (`arachne-perimeters`, `classic-perimeters`, `gyroid-infill`, `lightning-infill`, `rectilinear-infill`) becomes `float_or_percent` with that base key; classic already has the type and gains only the base key; `wave-overhangs` does not declare this key; `traditional-support.support_style` becomes the seven-value enum while `tree-support-planner` remains the existing-domain control.
- Migrate only the percent-intolerant readers: host `layer_executor.rs` for `bridge_line_width`; guest `arachne-perimeters`, `gyroid-infill`, `lightning-infill`, and `rectilinear-infill` for both width keys; guest `wave-overhangs` for `bridge_line_width`. Keep classic's already-correct `get_abs_value` reads as a control. AC-8 proves each file/key pair with the exact key argument, a non-empty explicit base argument, and result consumption: the static gate rejects a discarded `get_abs_value("bridge_line_width", nozzle_diameter);` statement and `_ = get_abs_value("bridge_line_width", nozzle_diameter);` discard, accepts a real assignment, return, enclosing function/macro argument, or fallback/method chain, and tolerates receiver names and line breaks. Rebuild and freshness-check guest artifacts.
- Regenerate `docs/15_config_keys_reference.md`; update the `## Module Manifest Schema (TOML)` example, common per-field table, cross-validation sections, SDK diagnose checks, the bounded `### RegionMapping (Builtin) — \`aggregated_region_split\` Threading` section in `docs/04_host_scheduler.md`, scheduler config-schema wire documentation/serialization, and ADR-0019 `## Status`; remove `VALID_SEVERITIES` and its source comment only in the explicitly authorized documentation step. The host-scheduler edit replaces the stale sentence that leaves the `slicer-core` → `slicer-scheduler` edge deferred.

## Out of Scope

- Typed scoped ingestion, unknown-key warn/drop behavior, claim-selection migration beyond registry selector metadata, and every `ConfigView` or run-path consumer.
- Automatic-value expansion, `ExpansionContext`, scope-delta resolution, layer-range ingestion/geometry, modifier typing, `ModifierScope` removal, `ModifierVolume.applies_to`, and the `spiral_vase` naming gap.
- Broad authoring of real-key `denied_scopes` values or deleting `[config.overridable-per-region]` / `[config.overridable-per-layer]`; this packet carries the field, parser, union, unknown-value rejection, and the one required `wall_generator` host selector denial. Packet 7 (`TASK-568`) owns the remaining machine/emitter deny-list authoring and consumption.
- WIT files, IR schema-version constants, generated docs by hand, `Cargo.lock`, `target/`, other packet directories, or `docs/specs/config-scope-resolution-plan.md` / `docs/07_implementation_status.md` edits.
- New ADRs or any `crates/slicer-schema` edit other than `VALID_SEVERITIES` and its stale cross-validation comment in the permitted Step 6a.

## Authoritative Docs

- `docs/specs/config-scope-resolution-plan.md` — bounded authority reads of row 1 and the named Registry, Crate topology, Reconciliation, Selector, the normative scope order `global/print < object < layer_range < modifier < paint_semantic < tool`, Type-conflict, Owner-decision, and Cross-cutting sections.
- `docs/adr/0067-unified-config-schema-registry.md`, `docs/adr/0068-config-scope-is-a-wire-encoding.md`, and `docs/adr/0069-scope-eligibility-is-a-per-key-deny-list.md` — direct short reads for the per-region selector load error, declaration-only Phase A, absent-denial default, and deny-list union.
- `docs/adr/0019-aggregated-region-split-entry-cross-crate-dependency.md` — direct short read for the relocation and closure amendment.
- `docs/02_ir_schemas.md` — delegated summary of IR versioning/config-key sections; no IR version change.
- `docs/03_wit_and_manifest.md` — bounded sections around `## Module Manifest Schema (TOML)`, common per-field keys, the cross-field validation example, and `## Validation Expression Language`; those sections are edited in Step 6a.
- `docs/04_host_scheduler.md` — the bounded `### RegionMapping (Builtin) — \`aggregated_region_split\` Threading` section; Step 6b replaces its stale deferred `slicer-core` → `slicer-scheduler` dependency paragraph after the type relocation.
- `docs/05_module_sdk.md` — targeted `pnp_cli module diagnose` `Checks` section.
- `crates/slicer-scheduler/src/manifest.rs` — targeted config-schema wire documentation, parser, and JSON serialization fragments for the retired field.
- `docs/11_operational_governance_and_acceptance_gate.md` — compatibility and CLI wire policy.
- `docs/15_config_keys_reference.md` — generated markers and rows only; regeneration is the authority.
- `docs/21_data_defaults_and_fixtures.md` and `docs/22_test_quality.md` — literal and independent-oracle gates.
- `docs/07_implementation_status.md` — targeted TASK-562 lookup only, no full read and no edit.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` — canonical `add("bridge_line_width", coFloatOrPercent)`, `add("initial_layer_line_width", coFloatOrPercent)`, their `ratio_over = "nozzle_diameter"`, and `add("support_style", coEnum)` with the seven enum values; cite function/add-call names, never line numbers.

## Acceptance Summary

- Positive: `AC-1`–`AC-10` in `packet.spec.md` cover topology, relocations/aliases, exact host rows, independent four-channel census, all reconciliation rules, corrected type sets, retirement/wire bump, per-reader migration/freshness, generated docs, and doc/ADR/dependency closure.
- Negative: `AC-N1`–`AC-N5` cover type disagreement, enum disagreement, the represented-module selector misuse case, all three base-key failures, and unknown denied scopes. AC-3/AC-5 positively pin `wall_generator`'s host selector denials; packet 7 (`TASK-568`) owns the remaining broad machine/emitter denial rollout.
- Cross-packet impact: rows 2–7 consume `slicer_config::ConfigSchemaRegistry`, `RegistryEntry`, `ModuleDeclaration`, `HostChannels`, `AssemblyOutcome`, `RegistryWarning`, `assemble_registry`, and `RegistryLoadError`; later rows consume the relocated IR aliases. Names and shapes are frozen in `design.md`.

## Verification Commands

This is the full matrix; `packet.spec.md` contains the closure subset and every AC has its own command. Each shell command is non-interactive, begins with `set -euo pipefail`, creates `target` before any `tee`, and filters test output to a small FACT. No command pipes a workspace test run.

| Proof | Command | Return format |
| --- | --- | --- |
| Topology and dev parse | `bash -lc 'set -euo pipefail; mkdir -p target; test -f crates/slicer-config/src/lib.rs; python -c "import tomllib; m=tomllib.load(open(\"crates/slicer-config/Cargo.toml\",\"rb\")); assert set(m.get(\"dependencies\",{})) == {\"slicer-ir\"}; assert m.get(\"dev-dependencies\",{}).get(\"toml\") == \"0.8\""'` | FACT pass/fail |
| Relocation and exact re-exports | `bash -lc 'set -euo pipefail; mkdir -p target; rg -q -x -F "pub use config_schema::{ConfigFieldEntry, ConfigSchema};" crates/slicer-ir/src/lib.rs; rg -q -x -F "pub use slice_ir::{AggregatedRegionSplitEntry, RegionSplitValueType};" crates/slicer-ir/src/lib.rs; rg -q -x -F "pub use manifest::{ConfigFieldEntry, ConfigSchema, RegionSplitValueType};" crates/slicer-scheduler/src/lib.rs; rg -q -x -F "pub use region_split::AggregatedRegionSplitEntry;" crates/slicer-scheduler/src/lib.rs; rg -q -F "schema: slicer_ir::config_schema::ConfigSchema" crates/slicer-config/src/lib.rs'` | FACT pass/fail |
| Host carrier | AC-3 command | FACT pass/fail; bounded failure snippet |
| Census | AC-4 command | FACT pass/fail; bounded failure snippet |
| Assembly and rejection suite | AC-5 plus AC-N1–N5 commands | FACT pass/fail; at most 20 failure lines |
| Validate retirement | AC-7 command | FACT pass/fail |
| Reader and guest freshness | AC-8 command | FACT pass/fail; every file/key call has an explicit base and a statically proven consumer (discarded statements fail; assignments, returns, arguments, and fallback chains pass), plus the two freshness exit codes |
| Generated documentation | AC-9 command | FACT pass/fail; regex-parsed exact rows, owner sets, and raw-pipe enum domain |
| ADR/docs/dependency closure | AC-10 command | FACT pass/fail; section-scoped retirement checks and ADR `Status` extraction |
| Workspace compile/lint | `bash -lc 'set -euo pipefail; mkdir -p target; cargo check --workspace --all-targets 2>&1 | tee target/test-output.log >/dev/null; rg -q "Finished" target/test-output.log'` and `bash -lc 'set -euo pipefail; mkdir -p target; cargo clippy --workspace --all-targets -- -D warnings 2>&1 | tee target/test-output.log >/dev/null; rg -q "Finished" target/test-output.log'` | FACT pass/fail |
| Literal/test-quality gates | `bash -lc 'set -euo pipefail; mkdir -p target; cargo xtask check-literals 2>&1 | tee target/test-output.log >/dev/null; rg -q "check-literals: 0 violation" target/test-output.log'` and `bash -lc 'set -euo pipefail; mkdir -p target; cargo xtask check-test-quality --report crates/slicer-config/tests crates/slicer-scheduler/src/manifest.rs 2>&1 | tee target/test-output.log >/dev/null; rg -q "^check-test-quality: 0 finding\(s\) in 0 file\(s\) \[" target/test-output.log'` | FACT pass/fail; zero findings |

## Step Completion Expectations

- Relocations and transitional aliases land before the registry implementation; no consumer is allowed to import a scheduler-owned definition after its move.
- `ConfigFieldEntry`'s 19 construction sites across 7 files are re-derived before the `validate` removal; any test literal that is watched uses FRU or a reasoned waiver. The three validate-bearing assertions and literal-pinned version assertion are edited with the removal.
- The census expected set is derived from live channels and parsed TOML, with wildcard entries retained in the registry. A passing assembly test cannot excuse a failing census.
- Manifest repairs and reader migrations are treated as one compatibility change; guest artifacts are rebuilt and checked before any guest failure is interpreted. AC-8 is not satisfied by a key substring: it requires the actual name-resolution-tolerant `get_abs_value(key, base)` method-call shape for each required pair, a non-empty explicit base, and a consumer proof that rejects a standalone semicolon statement and `_ = get_abs_value("bridge_line_width", nozzle_diameter);` while accepting assignments, returns, arguments, and fallback/method chains.
- Generated documentation is changed only by `cargo xtask gen-config-docs`; no manual edit of generated blocks is permitted.

## Context Discipline Notes

- `resolved_config.rs`, `manifest.rs`, `slice_ir.rs`, and `docs/15_config_keys_reference.md` are read only through located ranges or delegated summaries; never load them wholesale.
- Never load `target/`, `Cargo.lock`, generated code, or `OrcaSlicerDocumented/` directly.
- The manifest census, reader census, generator output, and all cargo commands have bounded return formats; a worker returns evidence, not a full log.
