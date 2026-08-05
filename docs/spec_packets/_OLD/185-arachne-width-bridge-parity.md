---
status: implemented
packet: 185-arachne-width-bridge-parity
task_ids:
  - TASK-303
  - TASK-212b
---

# 185-arachne-width-bridge-parity

## Goal

Bring role-aware flow/width resolution to OrcaSlicer parity by adding a shared context-aware resolver `resolve_role_width` in `crates/slicer-core/src/flow.rs`, consumed by all five width-consuming core modules, with percent/`FloatOrPercent` config transport through `ResolvedConfig.extensions` and the canonical key rename `first_layer_line_width` → `initial_layer_line_width`.

## Problem Statement

Width/flow resolution in PnP is not role-aware at OrcaSlicer parity. Five core modules (classic-perimeters, arachne-perimeters, rectilinear-infill, gyroid-infill, lightning-infill) each hand-resolve widths from a partial key set, with no shared first-layer/bridge context, no percent transport on the live path, and a non-canonical key name (`first_layer_line_width` where canonical spells `initial_layer_line_width`). Concretely: `parse_percent_default` in `crates/slicer-scheduler/src/manifest.rs` parses `ConfigValue::Percent`/`FloatOrPercent` and then discards the value at both `parse_config_field_entry` call sites (TASK-303, percent-transport gap residual ii), so no live slice can carry a percent-typed width; classic-perimeters still insets its final infill boundary by a raw `-inner_wall_line_width` instead of the canonical `process_classic` formula (classic final-infill-boundary gap residual); and `object_metadata_to_config_data` in `crates/slicer-model-io/src/loader.rs` drops part-level width keys required by TASK-212b before they can reach module config.

This packet supersedes packet 184's residuals (`184-classic-perimeter-flow-parity`, implemented). Packet 184 retyped classic's wall-width keys to `float_or_percent` and documented three forward residuals — the parser discard (TASK-303 / wall-width/percent-transport residual), the absent-key default divergence (`[FWD-1]`), and the final-infill-boundary offset — all absorbed here; 184's own files are not edited (the orchestrator flips its status). It is one coherent slice because the resolver, the transport that feeds it, and the key rename that names it are useless without each other: percent values need the extensions channel to reach modules, modules need the shared resolver to interpret first-layer/bridge/auto precedence uniformly, and the rename must land in the same parsing pass that introduces the alias.

## Architecture Constraints

- Canonical precedence order (locked): configured `bridge_line_width`; else positive `initial_layer_line_width` on the first layer; else the role width; a zero role width falls back to `line_width`, then auto — `0` is the auto sentinel (canonical `Flow.cpp::auto_extrusion_width`, `1.125 × nozzle_diameter`). Geometric widths/spacing only; flow-ratio controls are deferred to `DEV-102`.
- Config keys belong to modules: each module manifest declares the flow keys it consumes; there is NO central schema mechanism. All keys snake_case. Canonical key set: `line_width`, `initial_layer_line_width`, `sparse_infill_line_width`, `internal_solid_infill_line_width`, `top_surface_line_width`, `bridge_line_width`.
- Defaults move to auto (`0`), including global `line_width`; an explicit `0.4` stays explicit. The scheduler accepts the schema-aware legacy alias `first_layer_line_width` and REJECTS any profile specifying both alias and canonical key.
- `BottomSolidInfill` uses `internal_solid_infill_line_width` except first-layer/bridge overrides; no bottom-specific width key is introduced.
- Classic owns the canonical overlap keys in its manifest: `infill_wall_overlap` is percent default `15`, `top_bottom_infill_wall_overlap` is percent default `25`, and both use `inner_wall_line_width` as `ratio_over`; the module selects the absolute value through `ConfigView::get_abs_value`.
- `D-152-TOP-AREA-SOURCE` stays OPEN: do not design `upper_slices` IR/WIT access in this packet.
- ADR-0043 amendment is explicit: its `Decision` item 2 says Arachne wall-width keys are “plain mm floats, default 0.4, range [0.1, 2.0]”. This packet changes that contract for canonical parity, adds `Arachne wall-width ADR amendment` to `docs/DEVIATION_LOG.md`, and appends an amendment record to the ADR file.
- ADR-0014 amendment is explicit: its freshness list says “`slicer-core` and `slicer-helpers` are explicitly NOT tracked”. The current guest freshness contract tracks `slicer-core` as a universal guest dependency, so this packet adds `guest-freshness ADR amendment` to `docs/DEVIATION_LOG.md` and appends an amendment record to the ADR file.
<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and, if `STALE:` is reported, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.
<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.

## Data and Contract Notes

- IR/manifest contracts: the rename is a public-struct field rename on `ResolvedConfig` (serde-derived). `to_config_map`, `PartialEq`, `Hash`, and region mapping must use the canonical field; the literal legacy key is retained only in scheduler alias handling and its negative test. Per `TASK-302`'s finding, `ResolvedConfig`'s serde shape is constructed in-process and marshalled over WIT, never persisted — the rename is not a wire break and requires no `CURRENT_*_SCHEMA_VERSION` bump. The CLI/config-map key changes (`first_layer_line_width` → `initial_layer_line_width`); the legacy alias keeps old profiles loading, and both-keys profiles are hard errors.
- WIT boundary: unchanged. Percent values ride the existing `float-or-percent` WIT record shape that `ConfigValue::FloatOrPercent` already mirrors (`slice_ir.rs:706-714`) and the extensions map; no `.wit` edit, so guest bindgen is unaffected by the transport change (guest staleness still applies to `slicer-ir`/`slicer-core`/module source edits per the staleness bullet).
- Determinism/scheduler constraints: `resolve_role_width` is a pure function of (role, first-layer flag, bridge flag, resolved config values); no scheduling-order change. `extensions` is a `BTreeMap`, so percent-carrying iteration stays deterministic. Both-keys rejection is a validation error, not a warning, and fires at config resolution before any module dispatch.

## Locked Assumptions and Invariants

- Resolver math is in mm at the module boundary, matching `flow.rs`'s existing convention; all geometry handed to `polygon_ops`/offsetting converts via `mm_to_units()` (1 unit = 100 nm).
- All config keys snake_case; canonical key set fixed as listed in Architecture Constraints.
- Precedence chain and the `0` = auto sentinel (`1.125 × nozzle_diameter`, canonical `Flow.cpp::auto_extrusion_width`) are locked per Architecture Constraints.
- `BottomSolidInfill` role maps to `internal_solid_infill_line_width`; no bottom-specific key.
- Parameterized precedence-matrix tests land and pass BEFORE any golden re-bless; self-captured baselines that drift in the canonical-correct direction are re-blessed, never the reverse (per CLAUDE.md Test Discipline).
- `D-152-TOP-AREA-SOURCE` untouched; `crates/slicer-core/src/top_surface_split.rs::split_top_surfaces` untouched. TASK-212b is not considered complete until the loader allowlist regression passes.
- `Arachne wall-width ADR amendment` quotes the contested ADR clause and records why canonical `float_or_percent`/auto-`0` behavior is intentional; no silent ADR drift is permitted.
- `guest-freshness ADR amendment` quotes the stale `slicer-core` freshness exclusion and records why the current universal-guest dependency check is required; no stale-guest deflection is permitted.

## Risks and Tradeoffs

- The field rename's blast radius may exceed the surveyed sites — mitigated by the mandatory `LOCATIONS` dispatch plus `cargo check --workspace --all-targets` in Step 2's exit.
- Defaults moving to auto (`0`) is a user-visible output change on every profile that relied on implicit `0.4`; accepted per brief decision 4, absorbed by the matrix-tests-then-re-bless order.
- Classic one-wall-top and the flow-spacing integration gap infill-boundary formula will drift classic goldens; re-bless only after matrix tests pin the intended semantics.
- Percent retention grows `ResolvedConfig.extensions` contents on percent-typed manifests; harmless in-process (TASK-302) but asserted by a round-trip test to prevent silent re-coercion to `Float`.
- Editing `slicer-ir` + `slicer-core` + five module sources stales all guest WASMs; every module step gates on `cargo xtask build-guests --check`.
