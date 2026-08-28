---
status: implemented
packet: 226-authored-coloring-carrier
task_ids:
  - TASK-337
backlog_source: docs/07_implementation_status.md
context_cost_estimate: M
---

# Packet Contract: 226-authored-coloring-carrier

## Goal

Land the per-path `tool-index: option<u32>` WIT carrier on `extrusion-path3d`, the Rust mirror on `slicer_ir::ExtrusionPath3D`, the two-sided `claim:authored-coloring` grant gated by the `fill_authored_coloring` config key, the `tool-count` host service, the infill-linker tool-equality guard, and the DEV-135 deviation row — conforming to ADR-0058.

## Scope Boundaries

This packet is the WIT-rippling core of the authored-coloring mechanism. It changes one WIT record, one IR struct, the host-services interface, the SDK host wrapper, the infill linker's compatibility predicates, and two docs. It authors the carrier and its enforcement, not the Dragon Curve module (packet 227) that consumes it, and not the toolchain bump (packet 225, sequenced first).

## Prerequisites and Blockers

- Depends on: `225-dragon-curve-feasibility-gate` — **FORWARD-DEP on draft** (225 is `draft`, not `implemented`; its toolchain bump and guest rebuild must land first so this packet's `build-guests` rebuild targets the post-bump toolchain rather than forcing a redundant full rebuild).
- Unblocks: `227-dragon-curve-community-module` (consumes `tool-index`, `fill_authored_coloring`, and `slicer_sdk::host::tool_count()`).
- Activation blockers: none beyond the forward-dep on 225's draft.

## Acceptance Criteria

- **AC-1. Given** `crates/slicer-schema/wit/deps/types.wit`, **when** this packet lands, **then** the `extrusion-path3d` record declares `tool-index: option<u32>` after `speed-factor`. | `rg -n 'record extrusion-path3d \{' -A1 crates/slicer-schema/wit/deps/types.wit | rg 'tool-index: option<u32>'`
- **AC-2. Given** `crates/slicer-ir/src/slice_ir.rs`, **when** this packet lands, **then** `pub struct ExtrusionPath3D` carries `pub tool_index: Option<u32>` with `#[serde(default)]`. | `rg -n 'pub tool_index: Option<u32>' crates/slicer-ir/src/slice_ir.rs && rg -n '#\[serde\(default\)\]' crates/slicer-ir/src/slice_ir.rs`
- **AC-3. Given** `crates/slicer-schema/wit/deps/common.wit`, **when** this packet lands, **then** `host-services` declares `tool-count: func() -> u32`. | `rg -n 'tool-count: func\(\) -> u32' crates/slicer-schema/wit/deps/common.wit`
- **AC-4. Given** `crates/slicer-sdk/src/host.rs`, **when** this packet lands, **then** `pub fn tool_count()` exists with a `#[cfg(target_arch = "wasm32")]` arm calling the WIT import. | `rg -n 'pub fn tool_count' crates/slicer-sdk/src/host.rs`
- **AC-5. Given** `crates/slicer-ir/src/resolved_config.rs`, **when** this packet lands, **then** `fill_authored_coloring: Vec<String>` is declared and a net-new `extract_string_list` extractor exists. | `rg -n 'fill_authored_coloring' crates/slicer-ir/src/resolved_config.rs && rg -n 'pub fn extract_string_list' crates/slicer-ir/src/resolved_config.rs`
- **AC-6. Given** `crates/slicer-ir/src/slice_ir.rs`, **when** this packet lands, **then** `CURRENT_LAYER_COLLECTION_IR_SCHEMA_VERSION` is strictly greater than 1.2.0 (bumped in the same step as the field addition; value computed at activation). | `python3 -c "import re; t=open('crates/slicer-ir/src/slice_ir.rs',encoding='utf-8').read(); m=re.search(r'CURRENT_LAYER_COLLECTION_IR_SCHEMA_VERSION: SemVer = SemVer \{\s*major: (\d+),\s*minor: (\d+),', t); assert m, 'const missing'; assert (int(m.group(1)),int(m.group(2))) > (1,2), (m.group(1),m.group(2)); print('PASS')"`
- **AC-7. Given** the marshal and macro converters, **when** this packet lands, **then** `tool_index` is mapped in both directions (`convert_extrusion_path`/`ir_to_wit_extrusion_path` in `slicer-wasm-host`, and the `slicer-macros` `__slicer_path_*` helpers). | `rg -n 'tool_index' crates/slicer-wasm-host/src/marshal/leaf.rs crates/slicer-macros/src/lib.rs`
- **AC-8. Given** the infill linker, **when** this packet lands, **then** `paths_compatible` (orchestrate.rs) and `compatible_paths` (connect.rs) both require tool equality. | `rg -n 'tool_index' modules/core-modules/infill-linker/src/orchestrate.rs modules/core-modules/infill-linker/src/connect.rs`
- **AC-9. Given** `docs/DEVIATION_LOG.md`, **when** this packet lands, **then** a `| DEV-135 |` row exists and is format-conformant (the log uses `| DEV-NNN |`). | `rg -n '^\| DEV-135 \|' docs/DEVIATION_LOG.md`

## Negative Test Cases

- **AC-N1. Given** an infill path with `tool_index: Some(t)` emitted by a module that is **not granted** authored-coloring for its region, **when** `convert_infill_output` runs, **then** the committed path's `tool_index` is `None` (silent strip, not an error). | `cargo test -p slicer-wasm-host --test contract authored_coloring_grant_and_strip_tdd 2>&1 | tail -25`
- **AC-N2. Given** an infill path with `tool_index: Some(t)` where `t >= tool_count`, **when** `convert_infill_output` runs, **then** the committed path's `tool_index` is stripped/clamped to the region tool. | `cargo test -p slicer-wasm-host --test contract authored_coloring_grant_and_strip_tdd 2>&1 | tail -25`
- **AC-N3. Given** two paths with differing `tool_index`, **when** the linker's `compatible_paths`/`chain_or_connect_infill` runs, **then** they are not chained (split/refuse across per-path tools). | `cargo test -p infill-linker --test connect_tdd cross_tool_paths_not_chained 2>&1 | tail -25`
- **AC-N4. Given** a module that discloses `claim:authored-coloring` but whose held fill-role claim is **not** listed in `fill_authored_coloring`, **when** the grant predicate runs, **then** the grant is `false`; and symmetrically for config-listed-but-not-disclosed. | `cargo test -p slicer-wasm-host --test contract authored_coloring_grant_and_strip_tdd 2>&1 | tail -25`

## Verification

- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo xtask build-guests && cargo xtask build-guests --check`

## Authoritative Docs

- `docs/specs/community-modules-dragon-curve-plan.md` — direct read (102 lines) — binding symbol contract.
- `docs/specs/community-modules-dragon-curve-infill.md` §2 and §3 — direct read — the mechanism requirements.
- `docs/adr/0058-authored-coloring-per-path-tool-carrier.md` — direct read (38 lines) — the Accepted ADR this packet conforms to.
- `docs/adr/0044-wit-world-version-is-not-an-identity-token.md` — direct read of §Decision/§Consequences — the version-annotation rule this packet obeys.
- `docs/21_data_defaults_and_fixtures.md` — direct read (116 lines) — the struct-literal discipline (ExtrusionPath3D is below the 5-field watchlist threshold).
- `docs/DEVIATION_LOG.md` — delegated read of the last two `DEV-*` rows only — DEV-134 confirmed highest; DEV-135 is net-new.

## Doc Impact Statement (Required)

- `docs/02_ir_schemas.md` — IR 7 `PerimeterIR` / IR 8 `InfillIR` / IR 10 `LayerCollectionIR` sections: document the new `ExtrusionPath3D.tool_index` field and the `CURRENT_LAYER_COLLECTION_IR_SCHEMA_VERSION` bump. `rg -q 'tool_index: Option<u32>' docs/02_ir_schemas.md`; `rg -q 'tool_index' docs/02_ir_schemas.md`
- `docs/03_wit_and_manifest.md` — §host-api.wit: add `tool-count` to the `host-services` function list; §Known claim IDs: add the `claim:authored-coloring` capability-claim row. `rg -q 'tool-count' docs/03_wit_and_manifest.md`; `rg -q 'claim:authored-coloring' docs/03_wit_and_manifest.md`

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- obey the shared absolute context bands: 120k reading budget with hand-off at 150k (standard); the extended band (240k reading / 300k hard stop) only via swarm's escalation protocol

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation (an extended-band run may carry a single L step only when `design.md` justifies why it cannot be split).
