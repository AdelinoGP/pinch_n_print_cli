---
status: implemented
packet: config-scope-resolution_04_automatic-value-expansion
task_ids:
  - TASK-565
---

# config-scope-resolution_04_automatic-value-expansion

## Goal

Implement Phase B automatic-value expansion after each applicable scope resolution and before interning or `ConfigView` binding, using a registry-driven `ExpansionContext` for nozzle and tool bases, one width-family implementation, typed percent bases including the overhang-speed family, and the two config-only `-1 = auto` mirrors.

## Problem Statement

Automatic config values currently escape resolution in incompatible forms. `resolve_role_width` implements the width precedence and nozzle fallback, `resolve_support_line_width_mm` separately implements support width, percent-aware module readers choose bases at consumption, and `feedrate.rs::read_speed` drops percent-authored overhang speeds because it has no expansion base. The two config-only negative sentinels still mirror their top-side settings inside support guests. This duplicates policy and allows raw placeholders whose bases are fully config-known to reach a `ConfigView` even though all required Phase-B inputs are already known after scope merge. Canonical declares the overhang-speed percent family over `outer_wall_speed`; only numeric zero retains the existing role-context no-override behavior.

## Architecture Constraints

- Expansion is evaluation, not registry assembly: packet 1's registry remains declaration/provenance metadata, while this packet runs only after applicable scope values merge (ADR-0068 amendment).
- At this packet's current runtime seam, “applicable scope values” means each fully resolved map that exists before packet 5: global/default, per-object, per-paint-semantic, and per-tool. Packet 5 must reuse `expand_automatic_values` after its layer-range/modifier scope-stack merges; this packet does not pre-expand a delta and pretend it is equivalent to expanding a merged config.
- `slicer-config` continues to depend on `slicer-ir` only. It must not import runtime, scheduler, module-loader, WIT, geometry, or emitter types.
- `expand_automatic_values` is atomic. It derives replacements into scratch state, validates every recursive base and sentinel dependency, and mutates the supplied `ResolvedConfig` only after the whole expansion succeeds.
- Base resolution is deterministic and cycle-free because packet 1 validates the registry graph. For the selected tool, `ExpansionContext.tool_bases[tool_index][base_key]` wins when present; otherwise the already-merged config value wins; `nozzle_diameter` may fall back to `ExpansionContext.nozzle_diameter_mm`. Missing/non-positive required bases are errors, never fallback guesses.
- Only values with `RegistryEntry.base_key: Some(_)` receive generic percent expansion. Unitless percentages without a typed absolute base remain ratio values; this packet does not reinterpret every `percent` declaration.
- The four overhang-speed percent forms are Phase B: each registry entry has typed base `outer_wall_speed`, and expansion emits absolute mm/s before `FeedrateConfig::from_raw_config` or a module `ConfigView` reads it. Canonical `PrintConfigDef::init_fff_params` supplies this `ratio_over`; the active extrusion role is not the percent base.
- The only Phase-B zero-width rules are `line_width -> 1.125 * nozzle_diameter` and `support_line_width -> nozzle_diameter`. Zero role-specific width overrides remain a Phase-C dispatch signal to `resolve_role_width`, whose final fallback is the now-expanded `line_width`. Numeric overhang-speed zero remains the existing no-override value and is not converted to `outer_wall_speed` by generic percent expansion.
- The only Phase-B negative mirror rules are `support_interface_bottom_layers -> support_interface_top_layers` and `support_bottom_interface_spacing -> support_interface_spacing`. Packet 10 owns all context-dependent negative sentinels.
- No mm↔internal-unit boundary is introduced or changed: all expansion inputs/outputs are config-space millimetres or mm/s. The coordinate-system snippet is therefore intentionally absent.
- Compatibility checklist: no serialized IR field/type/variant changes; no WIT edit/package bump; no CLI JSON field/semantic change; no manifest field-vocabulary change; no schema/version constant bump. Existing manifest `type` and `base_key` values are declaration content, not a manifest schema-shape change.
<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and inspect its exit code: exit 0 means fresh, non-zero means stale (a distinct exit code signals `wasm-tools` is unavailable). Never use `rg -q 'STALE:'` — a `wasm-tools`-missing infrastructure error prints no `STALE:` and would read as fresh. If stale, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.

## Data and Contract Notes

- IR/manifest contracts: `ResolvedConfig`, `ConfigValue`, and `RegionMapIR` wire shapes are unchanged. Existing manifest fields `type` and `base_key` receive corrected values; the manifest schema vocabulary is unchanged.
- WIT boundary: no WIT edit. The behavioral invariant is that covered placeholders are gone before a `ConfigView` is constructed; typed instantiation shape is unchanged.
- Determinism/scheduler constraints: registry traversal plus object/tool/paint maps are ordered; expansion occurs after each applicable map's merge and before binding/interning, so equivalent expanded configs still deduplicate under `ResolvedConfig` bitwise equality/hash. Failure commits nothing and no partially expanded map reaches RegionMapping.
- Tool-base precedence: a matching selected-tool absolute base wins over the global merged base only for that tool's expansion. No selected tool means no tool-map lookup.
- Warnings: packet-1 registry warnings are preserved; expansion failures are fatal config-resolution errors with dependent/base names, not warnings followed by fallback.

## Locked Assumptions and Invariants

- `line_width = 0` resolves to `1.125 * nozzle_diameter`; `support_line_width = 0` resolves to exactly the nozzle diameter.
- Every percent-authored `overhang_1_4_speed` through `overhang_4_4_speed` resolves in Phase B against scope-resolved `outer_wall_speed`; numeric zero remains zero and is not moved to packet 10.
- Bridge override > first-layer override > role width > expanded base line width remains the role-width precedence.
- A bottom sentinel mirrors the corresponding top value only when the bottom value is exactly numeric `-1`; numeric zero is explicit.
- Generic percent expansion uses only `RegistryEntry.base_key`; no caller-selected ad hoc base remains for covered values.
- No Phase-B-covered placeholder reaches `bind_module_config_view`, `FeedrateConfig::from_raw_config`, `commit_region_mapping_builtin`, or any downstream `RegionMapIR::intern_config`; overhang-speed percentages are covered placeholders for this invariant.
- Both production entry points expand global/object values before plan binding and use the same expansion-aware configured prepass ordering. The prepass must not reconstruct raw tool/paint maps after the last expansion point.
- Packet 10 retains exclusive ownership of emitter volumetric auto and geometry-dependent sentinels.

## Risks and Tradeoffs

- `ResolvedConfig::to_config_map` is currently incomplete. This packet may use it only for the keys in the explicit Phase-B/base-key census and must fail on an unavailable base; packet 6 later replaces the projection registry-wide without changing the expansion API.
- The live host speed channel currently describes the four overhang rows as plain floats and `read_speed` drops percent variants. Retyping their `SPEED_META` wire declarations plus expanding before `FeedrateConfig::from_raw_config` is required; teaching `read_speed` to choose a base would recreate a second expansion owner.
- Direct guest tests bypass host expansion and previously relied on guest mirrors. Updating their fixtures to expanded values is required to avoid retaining a second production policy implementation.
- A `run.rs`-only implementation is a false fix: `execute_prepass_with_builtins_configured_instr_collecting` reconstructs tool and paint-semantic configs from `raw_config_source`, so covered scoped placeholders reappear immediately before RegionMapping unless `prepass.rs` expands those outputs.
- A non-positive nozzle previously yielded zero or a fallback. Fatal rejection is intentionally safer because successful zero-width geometry is invalid and NaN can poison interning.
- Guest mirror removal changes guest source and can expose stale artifacts; the mandatory freshness gate owns this risk.
