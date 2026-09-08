---
status: implemented
packet: 226-authored-coloring-carrier
task_ids:
  - TASK-337
---

# 226-authored-coloring-carrier

## Goal

Land the per-path `tool-index: option<u32>` WIT carrier on `extrusion-path3d`, the Rust mirror on `slicer_ir::ExtrusionPath3D`, the two-sided `claim:authored-coloring` grant gated by the `fill_authored_coloring` config key, the `tool-count` host service, the infill-linker tool-equality guard, and the DEV-135 deviation row — conforming to ADR-0058.

## Problem Statement

A module has no way to control its own coloring: tool is host-resolved per region via `resolve_region_tool_index` (`crates/slicer-wasm-host/src/dispatch.rs:2094`), so the Dragon Curve module's "tool = f(tiling_index)" requirement (§2/§3 of the design spec) has no carrier. This packet adds the per-path `tool-index: option<u32>` field to `extrusion-path3d`, mirrors it onto `slicer_ir::ExtrusionPath3D`, and enforces it through a two-sided grant — the module must disclose `claim:authored-coloring` **and** its fill-role claim must be listed in the `fill_authored_coloring` config key — with the host silently stripping any ungranted or out-of-range `Some(tool)`. It also closes the "modules have no way to know how many tools exist" gap with a `tool-count` host service, and teaches the infill linker to treat per-path tool as a chaining-compatibility axis (ADR-0058 Consequences).

## Architecture Constraints

- **ADR-0044 — no version tax.** `slicer:types/geometry` stays unversioned; the spec's "bump the package version" is satisfied by a doc-visible annotation in `types.wit`/`docs/03`, never a manifest `wit-world` change and never a world-version bump. Conforms to ADR-0044 §Decision/§Consequences.
- **ADR-0058 — conform, do not amend.** The carrier is a field on `extrusion-path3d` (survives `chain_or_connect_infill` cloning); support/finalization set `None` and are behaviorally unchanged; the linker's `paths_compatible` must add tool equality and split/refuse chains across differing per-path tools (the same guard already applied at region level); per-line tool changes raise tool-change count and wipe-tower purge volume (known cost, documented in DEV-135, not engineered around). No new ADR; no ADR amendment deviation.
- **Guest WASM staleness.** Changing `types.wit`/`common.wit` invalidates every guest's generated bindings. Mandatory closure: `cargo xtask build-guests --check` → `cargo xtask build-guests` (rebuild) → `cargo xtask build-guests --check` (green). This runs on the post-packet-225 toolchain.
- **Schema-version bump is computed, not hardcoded.** `CURRENT_LAYER_COLLECTION_IR_SCHEMA_VERSION` is live at `1.2.0` (`crates/slicer-ir/src/slice_ir.rs:315-319`); the bump lands in the same step as the field and its hard-asserting test fallout. The version target is derived from the live constant at activation (additive minor), never a future literal.
- **Out-of-range is silent strip/clamp, never a guest error.** Any emitted `tool >= tool_count` is stripped to the region-resolved tool (or clamped to it), matching the ungranted-strip posture.

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
