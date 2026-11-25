---
status: implemented
packet: config-scope-resolution_01_config-schema-registry
task_ids:
  - TASK-562
---

# config-scope-resolution_01_config-schema-registry

## Goal

Relocate the shared schema and region-split carriers, create `slicer-config` over `slicer-ir`, and assemble a deterministic four-channel `ConfigSchemaRegistry` with structurally validated selector metadata, validated base-key metadata, reconciled declarations, repaired manifest types, and an independent census oracle.

## Problem Statement

Schema declarations currently live in four unjoined channels: `ResolvedConfig::host_config_keys()`, module-level `slicer_ir::feedrate::SPEED_KEYS`, the scheduler's runtime-key tuple, and parsed module manifests. The shared `ConfigFieldEntry`/`ConfigSchema` definitions and `AggregatedRegionSplitEntry` also sit above the IR layer, while the scheduler still carries a dead `validate` field and the registry has no typed selector, base-key, enum, deny-list, or provenance reconciliation. The row-1 fix is coherent because the registry cannot be assembled until its carriers and host channels have stable homes, and the three manifest conflicts must be repaired before strict assembly can load the real declarations. The host selector row must also carry its explicit denial policy before selector validation can be truthful.

This is a ground-up correction of the prior draft. The live TOML parse is 270 entries / 179 distinct keys / 54 multi-declared keys, including only `object_height:*` and `layer_height:*` as single-declarer wildcard keys. The `ConfigFieldEntry` literal census is 19 construction literals across 7 files, excluding the struct definition. These counts guide discovery; the census implementation must derive its expected set from the channels, never from a hand-maintained roster.

## Architecture Constraints

- **Layering:** `slicer-config` depends on `slicer-ir` only. `slicer-ir` gains no first-party dependency. `slicer-core` drops its `slicer-scheduler` dependency; scheduler continues to depend on IR/schema and preserves transitional aliases.
- **Config-schema aliases:** `ConfigSchema` is defined at `slicer_ir::config_schema::ConfigSchema`; `slicer-ir/src/lib.rs` must contain the exact flat lines `pub use config_schema::{ConfigFieldEntry, ConfigSchema};` and `pub use slice_ir::{AggregatedRegionSplitEntry, RegionSplitValueType};`. Scheduler's module aliases and flat exports preserve existing downstream paths, with `crates/slicer-scheduler/src/lib.rs` containing the exact transitional lines `pub use manifest::{ConfigFieldEntry, ConfigSchema, RegionSplitValueType};` and `pub use region_split::AggregatedRegionSplitEntry;`.
- **Const-safe host carrier:** `HostConfigKey.default` is `Option<String>`, so it is not const-friendly. The only accepted runtime carrier is:
  ```rust
  pub struct HostRuntimeKey {
      pub key: &'static str,
      pub field_type: &'static str,
      pub scope: &'static str,
      pub default: &'static str,
      pub meta: HostKeyMeta,
      pub selector: bool,
      pub denied_scopes: &'static [&'static str],
  }
  pub const HOST_RUNTIME_KEYS: &[HostRuntimeKey] = &[
    HostRuntimeKey { key: "use_relative_e_distances", field_type: "bool", scope: SCOPE_PRINTER, default: "true", meta: HostKeyMeta::NONE, selector: false, denied_scopes: &[] },
    HostRuntimeKey { key: "thumbnail_path", field_type: "string", scope: SCOPE_PRINTER, default: "", meta: HostKeyMeta { display: Some("Thumbnail path"), description: Some("File path the slicer writes its thumbnail plate into; empty disables thumbnails."), group: Some("Output"), ..HostKeyMeta::NONE }, selector: false, denied_scopes: &[] },
    HostRuntimeKey { key: "wall_generator", field_type: "string", scope: SCOPE_PRINT, default: "classic", meta: HostKeyMeta::NONE, selector: true, denied_scopes: &["object", "layer_range", "modifier", "paint_semantic", "tool"] },
  ];
  ```
   Host rows may be formatted across lines but may not change their fields or values. The empty rows use `denied_scopes: &[]`; the `wall_generator` row uses `denied_scopes: &["object", "layer_range", "modifier", "paint_semantic", "tool"]`, exactly the scopes below `global/print` in the plan's order `global/print < object < layer_range < modifier < paint_semantic < tool`. `HostChannels::from_live`/assembly converts `default` to `Some(String)` when forming a common `HostConfigKey` and copies the static denial slice into `RegistryEntry.denied_scopes`; the `HostRuntimeKey` itself never embeds `HostConfigKey`. `build_host_key_entries` continues emitting the existing host JSON fields and ignores `selector` and `denied_scopes`; no runtime-carrier wire behavior is claimed to change.
- **Four channels:** assembly reads `ResolvedConfig::host_config_keys()`, module-level `slicer_ir::feedrate::SPEED_KEYS` plus `SPEED_META`, `slicer_ir::resolved_config::HOST_RUNTIME_KEYS`, and `ModuleDeclaration` manifest schemas. It does not expand automatic values.
- **Reconciliation:** type and enum domains are fatal disagreements; numeric bounds intersect and produce a warning naming all declarers; defaults choose a host declaration before the alphabetically-first module; claim-exclusive divergent defaults/bounds warn without changing deterministic selection; `denied_scopes` is a union; UI metadata is advisory (host then alphabetical); every entry retains sorted provenance.
- **Validation boundary:** packet 1 carries and reconciles selector flags, and structurally rejects any host or module declaration marked `selector` when its represented `denied_scopes` policy leaves one of the per-region scopes `object`, `layer_range`, `modifier`, `paint_semantic`, or `tool` statable. An absent/empty denial means every scope is statable under ADR-0069. The `wall_generator` host row explicitly denies all five narrower scopes and therefore passes; `global/print` remains allowed. Base keys must exist, be percent-compatible, and be acyclic; denied scopes are limited to `global`, `object`, `layer_range`, `modifier`, `paint_semantic`, and `tool`. Packet 7 (`TASK-568`) owns the later broad machine/emitter deny-list rollout, not this required selector policy.
- **Wire/version lock:** `CONFIG_SCHEMA_WIRE_VERSION` moves exactly from `"1.2.0"` to `"1.3.0"` only for the `validate` removal under owner decision 1. The version assertion and all validate-bearing literal/assertion fallout land in that step.
- **Schema field shape:** final `ConfigFieldEntry` is the current field set without `validate`, plus `selector: bool`, `base_key: Option<String>`, and `denied_scopes: Vec<String>` with serde defaults. `ConfigSchema` remains `{ pub entries: BTreeMap<String, ConfigFieldEntry> }`.
- **Guest freshness:**
<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and inspect its exit code: exit 0 means fresh, non-zero means stale (a distinct exit code signals `wasm-tools` is unavailable). Never use `rg -q 'STALE:'` — a `wasm-tools`-missing infrastructure error prints no `STALE:` and would read as fresh. If stale, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.
- AC-8's static reader check is syntax-tolerant about receiver names, whitespace, and line breaks, but not about semantics: for every required file/key pair it requires a `get_abs_value` method call whose first argument is the exact repaired key, whose second argument is a non-empty explicit base expression, and whose returned option is consumed. Its result-consumer proof rejects a standalone `get_abs_value("bridge_line_width", nozzle_diameter);` statement, rejects `_ = get_abs_value("bridge_line_width", nozzle_diameter);` discard, and accepts a real assignment, `return`, enclosing function/macro argument, or fallback/method chain.
- **No geometry/coordinate work:** the coordinate-system snippet is not applicable.

## Data and Contract Notes

- `ConfigFieldEntry` retains owned module-parsed strings and `ConfigSchema.entries`; new optional manifest fields default to `false`/`None`/empty so existing TOML remains loadable. `validate` is absent only after the retirement step.
- Host metadata remains `HostKeyMeta` with static fields; module metadata is `ModuleKeyMeta` with owned `String`s. No lifetime extension or module-string coercion into `HostKeyMeta` is allowed.
   - `HostRuntimeKey`'s `default` is converted to `HostConfigKey.default = Some(row.default.to_owned())` at channel assembly, while `row.denied_scopes` is copied into the registry entry. The current scheduler host JSON continues to use key/type/default/scope/metadata and does not expose selector or denials. `wall_generator` is the one host selector whose complete five-scope denial policy is authored and validated in packet 1; packet 7 owns the broad machine/emitter rollout.
- `slicer_ir::feedrate::SPEED_KEYS` is iterated together with `SPEED_META`; its function pointers are used to derive defaults from `FeedrateConfig::default()` without inventing a second table.
- Registry ordering is deterministic: BTreeMap keys, alphabetical module tie-break, sorted warning declarers/provenance, and no filesystem-order dependence in expected output.
- The census uses `toml::from_str` over the same manifest path/name rule used by `xtask` and `read_config_schema`; wildcard entries remain keys in the registry.

## Locked Assumptions and Invariants

   - The exact seven-field `HostRuntimeKey` shape and three rows are fixed; no `config: HostConfigKey` field is permitted, and `wall_generator`'s denial slice is exactly `object`, `layer_range`, `modifier`, `paint_semantic`, `tool`.
- `ConfigSchema`'s new API field uses `slicer_ir::config_schema::ConfigSchema`; flat aliases are compatibility surfaces, not the definition home.
- `slicer-config` has only `slicer-ir` as a normal first-party dependency; `toml = "0.8"` is dev-only.
- `SPEED_KEYS` remains module-level, 26 entries, and is referenced through `slicer_ir::feedrate::SPEED_KEYS`.
- `initial_layer_line_width` has exactly five declarers and each receives `base_key = "nozzle_diameter"`; `wave-overhangs` has no such declaration. `bridge_line_width` has six declarers, including wave.
   - `RegistryEntry` preserves enum values, denied-scope union, UI carriers, selector/base key, and sorted provenance; no warning is silently discarded. Selector eligibility is validated from the represented denial policy for both host and module rows; an absent/empty policy remains invalid for a selector.
- `CONFIG_SCHEMA_WIRE_VERSION` is exactly `"1.3.0"` after `validate` removal; no IR/WIT version changes.
- Generated docs are regenerated, not hand-edited; guest artifacts are fresh before closure.

## Risks and Tradeoffs

- Moving a public struct affects 19 literals across 7 files; the plan owns the census and FRU/waiver fallout rather than relying on a late compiler discovery.
- The scheduler manifest is large and combines parser, JSON, host rows, and inline tests; every read is ranged and each semantic edit has a separate step exit.
- Strict type/enum validation can expose an unlisted real declaration; the implementation must stop and re-census rather than widen the packet.
- Guest source changes require an artifact rebuild and an exit-code check; an infrastructure exit is not evidence of freshness.
- The host carrier relocation intentionally leaves selector out of the existing host JSON, so registry tests—not wire snapshots—prove selector propagation.
