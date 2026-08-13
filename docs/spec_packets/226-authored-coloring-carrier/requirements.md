# Requirements: 226-authored-coloring-carrier

## Packet Metadata

- Grouped task IDs: `TASK-337`
- Backlog source: `docs/07_implementation_status.md`
- Packet status: `draft`
- Aggregate context cost: `M`

## Problem Statement

A module has no way to control its own coloring: tool is host-resolved per region via `resolve_region_tool_index` (`crates/slicer-wasm-host/src/dispatch.rs:2094`), so the Dragon Curve module's "tool = f(tiling_index)" requirement (§2/§3 of the design spec) has no carrier. This packet adds the per-path `tool-index: option<u32>` field to `extrusion-path3d`, mirrors it onto `slicer_ir::ExtrusionPath3D`, and enforces it through a two-sided grant — the module must disclose `claim:authored-coloring` **and** its fill-role claim must be listed in the `fill_authored_coloring` config key — with the host silently stripping any ungranted or out-of-range `Some(tool)`. It also closes the "modules have no way to know how many tools exist" gap with a `tool-count` host service, and teaches the infill linker to treat per-path tool as a chaining-compatibility axis (ADR-0058 Consequences).

## In Scope

- Add `tool-index: option<u32>` to `record extrusion-path3d` in `crates/slicer-schema/wit/deps/types.wit` (ADR-0044: the spec's "bump slicer:types/geometry" means a doc-visible annotation in types.wit/docs only; no manifest `wit-world` change; no version-bump tax).
- Add `pub tool_index: Option<u32>` with `#[serde(default)]` to `slicer_ir::ExtrusionPath3D` (`crates/slicer-ir/src/slice_ir.rs:1941`), bump `CURRENT_LAYER_COLLECTION_IR_SCHEMA_VERSION` (live 1.2.0, `slice_ir.rs:315-319`) additively in the same step (packet-189 precedent), and update `docs/02_ir_schemas.md`.
- Map `tool_index` both directions: `convert_extrusion_path` (WIT→IR) and `ir_to_wit_extrusion_path` (IR→WIT) in `crates/slicer-wasm-host/src/marshal/leaf.rs` (fns at lines 223 and 423), plus the `slicer-macros` path converters `__slicer_path_ir_to_wit` / `__slicer_path_wit_to_ir` (lib.rs ~1290-1330) and `__slicer_wit_path_to_ir` / `__slicer_ir_path_to_wit` (~2599-2603, ~2739-2750).
- Add `tool-count: func() -> u32` to `slicer:common/host-services` in `crates/slicer-schema/wit/deps/common.wit`; implement it in `impl hs::Host for HostExecutionContext` (`crates/slicer-wasm-host/src/host.rs:2425`) with an authoritative host-side count source; add the SDK wrapper `slicer_sdk::host::tool_count()` (`crates/slicer-sdk/src/host.rs`) and the `sdk-host-services` inline WIT + `__sdk_host_services_import` block.
- Add the `fill_authored_coloring: Vec<String>` config key to `ResolvedConfig` (`crates/slicer-ir/src/resolved_config.rs` via `declare_resolved_config!`) with a net-new `extract_string_list` extractor (no `extract_string_list` exists today — grounded); per-region override flows through the existing `RegionMapIR` config-interner path, identical to `infill_density`.
- Implement the grant predicate (module holds a fill-role claim listed in `fill_authored_coloring` for the region **AND** module manifest discloses `claim:authored-coloring`) and wire it into the infill-output commit path so ungranted/out-of-range `Some(tool)` is stripped to the region-resolved tool (silent, not an error), while granted `Some(tool)` overrides `resolve_region_tool_index` including material-variant tools.
- Add tool equality to `paths_compatible` (`modules/core-modules/infill-linker/src/orchestrate.rs:377`) and `compatible_paths` (`modules/core-modules/infill-linker/src/connect.rs:493`), and make `chain_or_connect_infill` (`connect.rs:290`) split/refuse to chain across differing per-path tools (the same guard already applied at region level by `compatible_regions` / `majority_owner`).
- Update `docs/03_wit_and_manifest.md` (§host-api.wit function list + §Known claim IDs capability-claim row) and `docs/DEVIATION_LOG.md` (net-new `DEV-135` row, `| DEV-NNN |` convention).
- Register the new `slicer-wasm-host` contract test file in its aggregator (`crates/slicer-wasm-host/tests/contract/main.rs`) and add `cross_tool_paths_not_chained` to the existing `infill-linker` `connect_tdd` binary.

## Out of Scope

- The Dragon Curve module itself (`modules/community-modules/dragon-curve/`) — packet 227.
- The toolchain bump (wasmtime 47 / wit-bindgen 0.60) — packet 225 (sequenced first; this packet's guest rebuild must run on the post-bump toolchain).
- MoonBit/Go feasibility — packet 225.
- Support/finalization behavior change: those stages consume `extrusion-path3d` and set `tool_index = None`; behaviorally unchanged (ADR-0058).
- Adding WASI preview2 to the host, or any non-Rust guest support.
- A new ADR: ADR-0058 already governs this mechanism (Accepted). No ADR slot is allocated.

## Authoritative Docs

- `docs/specs/community-modules-dragon-curve-plan.md` — 102 lines; direct read (binding symbol contract).
- `docs/specs/community-modules-dragon-curve-infill.md` §2 and §3 — direct read; the mechanism's normative text.
- `docs/adr/0058-authored-coloring-per-path-tool-carrier.md` — 38 lines; direct read; this packet conforms to its Consequences.
- `docs/adr/0044-wit-world-version-is-not-an-identity-token.md` — direct read of §Decision/§Consequences; the no-version-tax rule.
- `docs/21_data_defaults_and_fixtures.md` — 116 lines; direct read; struct-literal discipline.
- `docs/DEVIATION_LOG.md` — delegated read of the last two `DEV-*` rows (never the whole 59+ line file); DEV-134 confirmed highest.

## Acceptance Summary

- Positive: `AC-1` through `AC-9`.
- Negative: `AC-N1` through `AC-N4`.
- Cross-packet impact: the `tool_index` field, `fill_authored_coloring` key, `tool-count` service, and the grant/strip semantics are the exact net-new symbols packet 227 consumes. Packet 227 must not activate until this packet is `implemented`; packet 225's toolchain bump must land first so this packet's `build-guests` rebuild is against the post-bump toolchain.

## Verification Commands

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `rg -n 'record extrusion-path3d \{' -A1 crates/slicer-schema/wit/deps/types.wit \| rg 'tool-index: option<u32>'` | WIT field landed | FACT pass/fail |
| `rg -n 'pub tool_index: Option<u32>' crates/slicer-ir/src/slice_ir.rs` | IR mirror landed | FACT pass/fail |
| `rg -n 'tool-count: func\(\) -> u32' crates/slicer-schema/wit/deps/common.wit` | host-service function landed | FACT pass/fail |
| `rg -n 'pub fn tool_count' crates/slicer-sdk/src/host.rs` | SDK wrapper landed | FACT pass/fail |
| `rg -n 'fill_authored_coloring' crates/slicer-ir/src/resolved_config.rs && rg -n 'pub fn extract_string_list' crates/slicer-ir/src/resolved_config.rs` | config key + extractor landed | FACT pass/fail |
| `python3 … (AC-6)` | schema bump > 1.2.0 | FACT pass/fail |
| `rg -n 'tool_index' crates/slicer-wasm-host/src/marshal/leaf.rs crates/slicer-macros/src/lib.rs` | both-direction mapping landed | FACT pass/fail |
| `rg -n 'tool_index' modules/core-modules/infill-linker/src/orchestrate.rs modules/core-modules/infill-linker/src/connect.rs` | linker guard landed | FACT pass/fail |
| `rg -n '^\| DEV-135 \|' docs/DEVIATION_LOG.md` | deviation row landed | FACT pass/fail |
| `cargo check --workspace --all-targets 2>&1 \| tail -30` | struct-literal blast radius closed | FACT exit 0 |
| `cargo clippy --workspace --all-targets -- -D warnings 2>&1 \| tail -30` | lint cleanliness | FACT exit 0 |
| `cargo xtask build-guests 2>&1 \| tail -20 && cargo xtask build-guests --check 2>&1 \| tail -20` | guest staleness closed | FACT exit 0 |
| `cargo test -p slicer-wasm-host --test contract authored_coloring_grant_and_strip_tdd 2>&1 \| tail -25` | grant/strip/out-of-range | FACT exit 0 |
| `cargo test -p infill-linker --test connect_tdd cross_tool_paths_not_chained 2>&1 \| tail -25` | linker cross-tool refuse | FACT exit 0 |
| `cargo test -p slicer-sdk --test layer_module_tdd 2>&1 \| tail -20` | SDK fixtures FRU-migrated | FACT exit 0 |
| `cargo test -p slicer-runtime --test contract wit_drift_detection_tdd 2>&1 \| tail -20` | WIT drift gate green | FACT exit 0 |

Commands must have small, parseable output suitable for delegation.

## Step Completion Expectations

- The WIT change (Step 1) and the IR mirror + schema bump (Step 2) must land before the converters (Step 3) and the grant/enforcement (Step 4), because the converters and enforcement compile against the new field.
- The schema-version bump and every hard-asserting test must land in the same step (Step 2) per the schema-version discipline; never defer test fallout to the acceptance ceremony.
- The struct-literal blast radius (Step 2/3) must be closed before `cargo check` is allowed as the falsifier; enumerate the sites first, then edit.
- The guest rebuild (`build-guests` + `--check`) is the Step 6 exit condition and must run on the post-packet-225 toolchain.

## Context Discipline Notes

- The struct-literal blast radius is large (grounded: 47 production `ExtrusionPath3D {` sites across 14 src files, 19 across 11 module src files, 121 across 62 crate test files, 33 across 18 module test files; plus 35 WIT `ExtrusionPath3d {` sites across 18 files). Dispatch a `LOCATIONS` worker per crate-family and edit mechanically; do not read every file in full.
- `docs/DEVIATION_LOG.md` is >59 long, truncated rows; delegate the "highest DEV-*" lookup, never read the whole file.
- `crates/slicer-wasm-host/src/host.rs` is >5,000 lines; bound reads to the `hs::Host` impl (~2425-2740) and `config_value_to_storage` (~95-148) only.
- `crates/slicer-macros/src/lib.rs` is ~3,000 lines; bound reads to the four path-converter ranges only.
- Heavy-dispatch return limits: every `cargo check`/`clippy`/`test` command is tail-filtered (≤30 lines).
