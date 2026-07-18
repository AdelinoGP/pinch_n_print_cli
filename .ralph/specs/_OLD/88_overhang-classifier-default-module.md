---
status: implemented
packet: 88
task_ids: [TASK-238]
---

# 88_overhang-classifier-default-module

## Goal

Ship `modules/core-modules/overhang-classifier-default/` — a guest WASM module implementing the existing `FinalizationModule` trait — that **owns the complete overhang-classification logic** (the 319-LOC `classify_layers` algorithm relocated from `slicer-core/src/algos/overhang_classifier.rs` plus the `LinesDistancer2D` primitive it consumes and any other helpers it pulls from `slicer-core`'s internals) and emits per-wall-entity `modify-entity(entity_id, set-speed-factor(factor))` mutations through the `finalization-output-builder` already defined in `world-finalization@1.0.0`; delete `slicer-gcode`'s direct `classify_layers` call site, delete `slicer-runtime/src/lib.rs:192`'s P84-era `pub use slicer_core::algos::overhang_classifier::classify_layers;` re-export, delete `crates/slicer-core/src/algos/overhang_classifier.rs` and the P84 golden test at `crates/slicer-core/tests/algo_overhang_classifier_tdd.rs` (or migrate the golden into the guest's tests); the guest is self-contained — no `slicer-core` dep — preventing the `host-algos` feature gate from contaminating the guest dep tree. Default `pnp_cli slice --module-dir modules/core-modules` invocations preserve current behavior modulo a possible LSB-precision shift in feedrate decimals per AC-7; users who curate a custom module dir without this module get NO overhang annotation (the explicit Q6-resolution from the deepening-plan grilling).

## Problem Statement

After P84 / P86 / P87, the overhang-classification kernel lives in `slicer-core`, `FeedrateConfig` lives in `slicer-ir`, and `slicer-gcode`'s `emit_gcode` body calls `slicer_core::classify_layers` directly — a host-only path. This works, but it bakes overhang-feedrate selection into the host serializer, leaving zero swap-point for users who want different overhang behavior (different angle thresholds, different quartile counts, non-linear speed mappings, per-feature curves, etc.). The deepening-plan grilling's Q3 resolved that overhang classification is a real swap point worth modularising — but Q3's exploration also found the existing `world-finalization::run-finalization` export already provides everything needed: `list<layer-collection-view>` input + `modify-entity(entity_id, set-speed-factor)` output. No new stage, no new WIT export, no contract churn for the 20 existing guest modules.

P88 ships `modules/core-modules/overhang-classifier-default/` — a `FinalizationModule`-implementing WASM module that uses the same `#[slicer_module]` macro and SDK patterns as the existing 20 core-modules. It reads the four overhang-speed config fields from `config-view`, calls `slicer_core::classify_layers`, and emits `set-speed-factor` mutations for each wall entity in a non-Q4 (overhang) quartile. `slicer-gcode`'s direct `classify_layers` call is deleted; the emit path already consumes `set-speed-factor` annotations (the existing `finalization-default` module exercises that path for non-overhang reasons).

Q6 resolution preserved: ship the module; no host fallback. The module ships under `modules/core-modules/`, so the standard `--module-dir modules/core-modules` invocation loads it and behavior matches the pre-batch baseline. Users who curate a different module dir without `overhang-classifier-default` get NO overhang annotation — they opted out by curating.

This is the final packet in the deepening batch; the workspace test gate at close is the batch's final acceptance ceremony.

## Architecture Constraints

- ADR-0001 / 0002 / 0003 (preserved); ADR-0005 / 0006 (P83 — runner traits + export_for_stage_id); ADR-0008 (P85 — CompiledModule Static/Live split) preserved. ADR-0004 (Test support in slicer-sdk, P77) unrelated to this packet's surface.
- ADR-0008 (drafted at P88 close) — overhang annotation as a `FinalizationModule`; no new stage; module-absent ⇒ no annotation; guest owns the complete algorithm (no `slicer-core` dep) to prevent the `host-algos` feature gate from contaminating the guest dep tree.
- No WIT file is edited. AC-N3 verifies via `git diff`.
- The new module's manifest claims the same world (`slicer:world-finalization@1.0.0`) as `finalization-default`. Both run in the same stage; ordering is per the DAG's claim-based topology (no two modules claim the same role, so they run sequentially in declared order).

<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and, if `STALE:` is reported, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by this edit.

<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.

(`slicer_core::classify_layers` operates in integer units throughout — the module passes layer geometry to the kernel unchanged.)

## Data and Contract Notes

- `factor` value range: typically 0.0 < factor ≤ 1.0 (overhang speeds are slower than base). The WIT type is `f32`; the SDK should accept any positive finite value.
- `base_speed_for_role` lookup: the module reads `feedrate_config` for the base speed of the entity's role. For wall-family roles, the base speed is typically `outer_wall_speed` or `inner_wall_speed`; the existing `crates/slicer-runtime/src/gcode_emit.rs::resolve_feedrate` (pre-P86; now in `slicer-gcode`) has the role→base-speed mapping table — copy the relevant subset into the module.
- `factor = overhang_speed / base_speed`: f32 division. AC-7 documents the LSB-precision shift risk.
- Zero-overhang-speed short-circuit: if all four `overhang_*_4_speed` are 0.0, the module returns early without emitting any mutations. Preserves the pre-P84 AC-2 byte-identical baseline for printers that don't configure overhang speeds.

## Locked Assumptions and Invariants

- ADR-0008 (drafted at close): overhang is a FinalizationModule; no new stage; no host fallback.
- No WIT contract change (AC-N3).
- 8 builtins in `runtime_builtins()` (unchanged).
- Default `pnp_cli slice --module-dir modules/core-modules` invocations include the new module; behavior matches the documented batch baseline (byte-identical or AC-7-documented LSB shift).
- Custom invocations without the module produce slice output without overhang annotation; AC-6 documents this as the explicit user opt-out.

## Risks and Tradeoffs

- **Risk: AC-7 SHA divergence beyond LSB precision.** Mitigation: dispatch #9's diff verdict catches any non-F-word divergence; bisect the module logic (most likely culprits: wrong quartile classification, wrong base-speed lookup, or `set-speed-factor` not being multiplicatively applied as expected by `slicer-gcode`'s emit path).
- **Risk: workspace test gate flakes** (large suite). Mitigation: dispatch #10 captures duration and count; re-dispatch specific failing tests individually.
- **Risk: the existing `finalization-default` module claims a role that conflicts with `overhang-classifier-default`'s claims.** Mitigation: the new module's claims should be a unique set (e.g., `overhang-speed-factor`); confirm via the DAG validation that no two modules claim the same role.
- **Tradeoff: f32 LSB precision in `factor = overhang_speed / base_speed`.** Acceptable; AC-7 explicitly documents the trade-off. Real-world printers tolerate f32 feedrate decimal noise.
