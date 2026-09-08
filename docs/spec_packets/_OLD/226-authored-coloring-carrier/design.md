# Design: 226-authored-coloring-carrier

## Controlling Code Paths

- Primary code path: WIT `slicer:types/geometry.extrusion-path3d` → `convert_extrusion_path`/`ir_to_wit_extrusion_path` (`crates/slicer-wasm-host/src/marshal/leaf.rs:223,423`) → `InfillIR` regions → `assemble_ordered_entities` (`crates/slicer-runtime/src/layer_executor.rs:1616`, the per-path/entity tool-stamping site) → the infill linker (`modules/core-modules/infill-linker/src/{orchestrate.rs,connect.rs}`).
- Neighboring tests/fixtures: `crates/slicer-wasm-host/tests/contract/infill_holder_resolution_painted_region_tdd.rs` (the proven contract-test harness for per-region infill dispatch, with `run_infill_stage`), `modules/core-modules/infill-linker/tests/connect_tdd.rs` (existing `path()`/`raw_paths()` fixtures), and the SDK `test_support` fixtures that construct `ExtrusionPath3D` literally.
- OrcaSlicer comparison: n/a — this work has NO OrcaSlicer parity consultation (the mechanism has no OrcaSlicer precedent, per ADR-0058 and the deviation row).

## Architecture Constraints

- **ADR-0044 — no version tax.** `slicer:types/geometry` stays unversioned; the spec's "bump the package version" is satisfied by a doc-visible annotation in `types.wit`/`docs/03`, never a manifest `wit-world` change and never a world-version bump. Conforms to ADR-0044 §Decision/§Consequences.
- **ADR-0058 — conform, do not amend.** The carrier is a field on `extrusion-path3d` (survives `chain_or_connect_infill` cloning); support/finalization set `None` and are behaviorally unchanged; the linker's `paths_compatible` must add tool equality and split/refuse chains across differing per-path tools (the same guard already applied at region level); per-line tool changes raise tool-change count and wipe-tower purge volume (known cost, documented in DEV-135, not engineered around). No new ADR; no ADR amendment deviation.
- **Guest WASM staleness.** Changing `types.wit`/`common.wit` invalidates every guest's generated bindings. Mandatory closure: `cargo xtask build-guests --check` → `cargo xtask build-guests` (rebuild) → `cargo xtask build-guests --check` (green). This runs on the post-packet-225 toolchain.
- **Schema-version bump is computed, not hardcoded.** `CURRENT_LAYER_COLLECTION_IR_SCHEMA_VERSION` is live at `1.2.0` (`crates/slicer-ir/src/slice_ir.rs:315-319`); the bump lands in the same step as the field and its hard-asserting test fallout. The version target is derived from the live constant at activation (additive minor), never a future literal.
- **Out-of-range is silent strip/clamp, never a guest error.** Any emitted `tool >= tool_count` is stripped to the region-resolved tool (or clamped to it), matching the ungranted-strip posture.

## Code Change Surface

- Selected approach: a single WIT record field + its IR mirror + a two-sided grant enforced at the marshal boundary, with the linker gaining tool as a chaining axis. No side-list, no separate entities.
- Exact functions, traits, manifests, tests, and fixtures:
  - `crates/slicer-schema/wit/deps/types.wit:12` — `record extrusion-path3d { points, role, speed-factor, tool-index: option<u32> }` (doc comment: `None` = host decides).
  - `crates/slicer-ir/src/slice_ir.rs:1941` — `ExtrusionPath3D { points, role, speed_factor, #[serde(default)] pub tool_index: Option<u32> }`.
  - `crates/slicer-ir/src/slice_ir.rs:315` — `CURRENT_LAYER_COLLECTION_IR_SCHEMA_VERSION` bumped additively (compute at activation; current 1.2.0).
  - `crates/slicer-wasm-host/src/marshal/leaf.rs` — `ir_to_wit_extrusion_path` maps `tool_index: path.tool_index`; `convert_extrusion_path` maps `tool_index: path.tool_index` (Option round-trips without validation here; the grant/out-of-range enforcement lives at the commit boundary, not in this pure converter).
  - `crates/slicer-macros/src/lib.rs` — the four generated path converters (`__slicer_path_ir_to_wit`, `__slicer_path_wit_to_ir`, `__slicer_wit_path_to_ir`, `__slicer_ir_path_to_wit`) map the field both directions; the two IR→WIT sites emit `tool_index: p.tool_index`, the two WIT→IR sites emit `tool_index: p.tool_index`.
  - `crates/slicer-schema/wit/deps/common.wit` — `tool-count: func() -> u32` on `slicer:common/host-services` (no new record).
  - `crates/slicer-sdk/src/host.rs` — `pub fn tool_count() -> u32` with `#[cfg(target_arch = "wasm32")]` arm calling `__sdk_host_services_import::slicer::common::host_services::tool_count()`; the `sdk-host-services` inline WIT block gains the `tool-count` function line.
  - `crates/slicer-wasm-host/src/host.rs` — `impl hs::Host for HostExecutionContext::tool_count` returns the authoritative host-side count. The authoritative source is `ResolvedConfig.filament_density.len()` (the per-tool list, one entry per filament/tool), passed into `HostExecutionContext` as a new field populated by the dispatcher from the same per-region `ResolvedConfig` it already resolves for held-claims; `tool_count()` returns `max(1, len)`. (Grounded: `filament_density` is the repo's only per-tool count carrier — `ResolvedConfig::filament_density_for(tool_index)` reads `.get(tool_index as usize)`, and `slicer-gcode` prices each tool via `.get(tool)`; no `tool_count`/`extruder_count` symbol exists anywhere. See Open Questions for the [FWD] note if the reviewer deems this ambiguous — the WIT function shape is fixed regardless.)
  - `crates/slicer-ir/src/resolved_config.rs` — `extract_string_list` (net-new, mirrors `extract_float_list` shape: `List` of `String` accepted, scalar coerced to one-element list) and `cli "fill_authored_coloring" fill_authored_coloring: Vec<String> = Vec::new() => extract_string_list;`.
  - `crates/slicer-wasm-host/src/dispatch.rs` — extend `resolve_held_claims`'s per-region config resolution and `convert_infill_output`'s call path so the commit boundary knows, per `(object_id, region_id)`: (a) the module's held fill-role claims (already computed at `dispatch.rs:2492-2535`), (b) the module's disclosed `claim:authored-coloring` (from `module.claims`), (c) the region's `fill_authored_coloring` list (from the same `ResolvedConfig`). The grant predicate is `(held_fill_claims ∩ fill_authored_coloring ≠ ∅) && claims.contains("claim:authored-coloring")`. `convert_infill_output` gains an optional grant+tool-count context and, per path, either honors `Some(t)` (granted and `t < tool_count`) or strips/clamps to the region tool.
  - `crates/slicer-runtime/src/layer_executor.rs:1616` — `assemble_ordered_entities` already stamps `PrintEntity.tool_index` per path; the infill arm (`infill_push`, ~1857-1882) must prefer `path.tool_index` (validated at the marshal boundary) over the existing `variant_tool.or(spatial_tool).unwrap_or(DEFAULT_TOOL)` chain, mirroring the wall-loop precedence that already lets paint-derived tools win over the region default. The marshal boundary is the enforcement point; this site only consumes the already-normalized value.
  - `modules/core-modules/infill-linker/src/orchestrate.rs:377` — `paths_compatible` adds `first.tool_index == second.tool_index`.
  - `modules/core-modules/infill-linker/src/connect.rs:493` — `compatible_paths` adds `first.tool_index == second.tool_index`; `chain_or_connect_infill` (290) splits/refuses across differing per-path tools by never chaining paths that fail the predicate (the existing nearest-neighbor walk already refuses when `compatible_paths` is false).
  - `docs/DEVIATION_LOG.md` — net-new `DEV-135` row.
- Rejected alternatives and reasons:
  - **Per-path tool side-list on the infill-output-builder.** Rejected (ADR-0058): the linker clones and re-emits paths inside `chain_or_connect_infill`; a parallel list cannot survive that, so the field is the only carrier that does.
  - **Emit separate entities.** Rejected: no existing per-entity tool seam for infill output and it does not survive linking any better (ADR-0058).
  - **Config-side tool-count only (no host service).** Rejected: the design spec §4 mandates a tool-count query the module can call; a config key would leak a host fact into guest config resolution and cannot adapt to a per-call resolved config. The WIT function shape is fixed by the plan file.
  - **New ADR.** Rejected: ADR-0058 already governs this mechanism (Accepted); the deviation is recorded via DEV-135 only.

## Files in Scope (read + edit)

- `crates/slicer-schema/wit/deps/types.wit` — role: WIT carrier; expected change: one record field + doc comment.
- `crates/slicer-schema/wit/deps/common.wit` — role: host-service function; expected change: one `tool-count` line.
- `crates/slicer-ir/src/slice_ir.rs` — role: IR mirror + schema bump; expected change: one field, one const bump, `ExtrusionPath3D` doc comment.
- `crates/slicer-ir/src/resolved_config.rs` — role: config key + extractor; expected change: `extract_string_list` + one `cli` line.
- `crates/slicer-wasm-host/src/marshal/leaf.rs` — role: WIT↔IR mapping; expected change: two converter fields.
- `crates/slicer-wasm-host/src/host.rs` — role: `tool_count` host impl + new context field; expected change: one trait method + one struct field + `config_value_to_storage` unchanged (list handling already exists).
- `crates/slicer-wasm-host/src/dispatch.rs` — role: grant predicate + commit-boundary enforcement; expected change: resolve the region `fill_authored_coloring` + disclosed claim, thread a grant/tool-count context into `convert_infill_output`, strip/clamp at commit.
- `crates/slicer-runtime/src/layer_executor.rs` — role: entity tool-stamping precedence; expected change: infill arm prefers validated `path.tool_index`.
- `crates/slicer-sdk/src/host.rs` — role: SDK wrapper; expected change: `tool_count()` + inline WIT function line.
- `crates/slicer-macros/src/lib.rs` — role: generated path converters; expected change: four converter fields.
- `modules/core-modules/infill-linker/src/orchestrate.rs` — role: linker guard; expected change: `paths_compatible` tool equality.
- `modules/core-modules/infill-linker/src/connect.rs` — role: linker guard; expected change: `compatible_paths` tool equality.
- `docs/02_ir_schemas.md` — role: IR doc; expected change: `tool_index` field + schema bump note.
- `docs/03_wit_and_manifest.md` — role: WIT/claim doc; expected change: `tool-count` + `claim:authored-coloring`.
- `docs/DEVIATION_LOG.md` — role: deviation record; expected change: `DEV-135` row.
- All struct-literal sites (production src + test files + SDK fixtures) — role: blast-radius closure; expected change: `tool_index: None` in production literals, `..Default::default()` / fixture base in test literals (or explicit field per docs/21).

Justification for >3 files: this is the packet's stated WIT-rippling core. The WIT record + its IR mirror + the converters + the enforcement + the linker guard are inseparable; each file is a single mechanical change, and the blast radius is enumerated up front rather than discovered by the compiler.

## Read-Only Context

- `crates/slicer-ir/src/slice_ir.rs` — lines 185-327 (schema constants), 1744-1972 (Point3WithWidth + ExtrusionPath3D) — purpose: verify exact field list and const value.
- `crates/slicer-ir/src/resolved_config.rs` — lines 455-521 (`extract_float_list` shape) and 809-990 (`declare_resolved_config!` + fill-holder block) — purpose: mirror the list extractor and add the key in the right block.
- `crates/slicer-wasm-host/src/dispatch.rs` — lines 2085-2203 (`resolve_region_tool_index` + its call site) and 2483-2535 (held-claims resolution) — purpose: ground the grant inputs and the region-tool stamping.
- `crates/slicer-wasm-host/src/marshal/leaf.rs` — lines 219-243 and 418-443 — purpose: the two converters.
- `crates/slicer-wasm-host/src/host.rs` — lines 95-148 (`config_value_to_storage`), 2425-2740 (`hs::Host` impl), 1134-1340 (`HostExecutionContext` fields) — purpose: tool-count impl + new field.
- `crates/slicer-runtime/src/layer_executor.rs` — lines 1616-1943 (`assemble_ordered_entities`, wall + infill arms) — purpose: the entity-stamping site.
- `crates/slicer-macros/src/lib.rs` — lines 1285-1330 and 2590-2605 and 2735-2751 — purpose: the four path converters.
- `modules/core-modules/infill-linker/src/orchestrate.rs` — lines 358-395 (`compatible_regions`, `paths_compatible`) — purpose: the predicate body.
- `modules/core-modules/infill-linker/src/connect.rs` — lines 290-349 and 488-517 — purpose: `chain_or_connect_infill`, `compatible_paths`, `endpoint_widths_compatible`.

## Out-of-Bounds Files

- `OrcaSlicerDocumented/...` — delegate; never load (no parity obligation).
- `target/`, `Cargo.lock`, generated code (`**/wit-guest/**` is regenerated, never hand-edited), vendored dependencies.
- `docs/DEVIATION_LOG.md` full body — delegate the highest-`DEV-*` lookup; never read the whole file.
- `docs/specs/community-modules-dragon-curve-infill.md` §1/§4/§5/§6/§7 — not edited here (packet 225 owns §6, packet 227 owns §1/§4/§5, packet 228 owns §7).

## Expected Sub-Agent Dispatches

- Question: enumerate every `ExtrusionPath3D {` (Rust IR) and `ExtrusionPath3d {` (WIT) struct-literal site with file:line, split by (production src / test / fixture / test-guest); scope: `crates/ modules/`; return: `LOCATIONS`; purpose: Step 2/3 blast-radius closure.
- Question: confirm `docs/DEVIATION_LOG.md`'s highest `DEV-NNN` (expected 134) and sample two recent `DEV-*` row formats; scope: `docs/DEVIATION_LOG.md`; return: `FACT` + `SNIPPETS`; purpose: Step 7 deviation row.
- Question: confirm `resolve_region_tool_index`'s exact signature/body and every `resolve_region_tool_index` call site; scope: `crates/slicer-wasm-host/src/dispatch.rs`; return: `LOCATIONS`; purpose: Step 4 enforcement wiring.
- Question: locate every `CURRENT_LAYER_COLLECTION_IR_SCHEMA_VERSION` hard-asserting test; scope: `crates/ modules/`; return: `LOCATIONS`; purpose: Step 2 schema-bump fallout.

## Data and Contract Notes

- IR/manifest contracts: `ExtrusionPath3D.tool_index: Option<u32>` is additive with `#[serde(default)]`, so old fixtures parse and `None` round-trips; the `CURRENT_LAYER_COLLECTION_IR_SCHEMA_VERSION` bump is additive-minor (packet-189 precedent). No manifest field changes.
- WIT boundary: `tool-index: option<u32>` on `extrusion-path3d`; `tool-count: func() -> u32` on `slicer:common/host-services`. Both are additive; the WIT package stays unversioned per ADR-0044.
- Determinism/scheduler constraints: grant is a pure function of (module, region, setting, tool_count); stripping is silent and deterministic; no scheduler-order dependence.

## Locked Assumptions and Invariants

- `tool-count` returns `max(1, filament_density.len())`; the authoritative count source is `ResolvedConfig.filament_density` (the repo's only per-tool count carrier). If review rejects this source, the function shape stays fixed and the source becomes a [FWD] (see Open Questions) — the carrier/enforcement do not change.
- Granted `Some(tool)` overrides the region-resolved tool (including material-variant); ungranted or out-of-range `Some(tool)` strips/clamps to the region tool silently.
- `slicer:types/geometry` is not version-bumped; ADR-0044's no-version-tax rule is absolute.
- ADR-0058 is conformed to, never amended; no new ADR.

## Risks and Tradeoffs

- The struct-literal blast radius is large (200+ literal sites). Mitigation: enumerated by dispatch up front, edited mechanically, and `ExtrusionPath3D` stays below the docs/21 watchlist threshold (only 4 fields after this change, so test FRU is not mandatory — but the packet still prefers FRU in shared fixtures and uses explicit `tool_index: None` elsewhere to keep the diff mechanical and clippy-clean).
- Choosing `filament_density.len()` as the count source is a judgment call; the alternative sources (`tool_configs` keys, gcode `filament_per_tool` max) are emit-time only and not reachable from the host-services boundary. Recorded as [FWD] if the reviewer disagrees.
- The marshal-boundary enforcement adds a grant context parameter to `convert_infill_output`; both call sites (`dispatch.rs` deconstruct and `native.rs` commit) must thread it. The native path is behaviorally unchanged (native modules do not emit `Some(tool)` today) but must still compile.

## Context Cost Estimate

- Aggregate: `M`
- Largest step: `M` (Step 4, grant predicate + commit-boundary enforcement — bounded by the already-read dispatch ranges)
- Highest-risk dispatch and required return format: the struct-literal `LOCATIONS` dispatch (Step 2/3) — return `LOCATIONS` with file:line and a per-family count; a SUMMARY would fail to bound the mechanical edit.

## Open Questions

- [FWD] Confirm the authoritative host-side tool-count source is `ResolvedConfig.filament_density.len()` (with `max(1, …)`), versus any per-machine extruder list the reviewer can name. The WIT function shape (`tool-count: func() -> u32`) and the SDK wrapper are fixed regardless; only the implementation's source expression is open.
- [FWD] Whether `chain_or_connect_infill`'s existing nearest-neighbor walk already refuses on `compatible_paths == false` or needs an explicit guard added at the chain step. The predicate change (tool equality) is mandatory either way; the exact line of the refusal is resolved at activation.

Neither is an activation blocker; both are implementer-resolvable with the already-delegated reads.
