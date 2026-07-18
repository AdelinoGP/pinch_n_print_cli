---
status: implemented
packet: 149-pipeline-config-wiring
task_ids:
  - none
---

# 149-pipeline-config-wiring

## Goal

Close four pipeline-wide red tests in `crates/slicer-runtime/tests/arachne_parity.rs` (rewritten where applicable to drive behavior natively) by registering the missing OrcaSlicer config keys — `detect_overhang_wall`, `overhang_reverse`, `overhang_reverse_internal_only`, `extra_perimeters_on_overhangs` (re-publish), `min_width_top_surface`, `alternate_extra_wall`, `bridge_flow` (new), `thick_bridges` (new) — implementing D3 (`alternate_extra_wall` bumps `ArachneParams.max_bead_count` on odd layers, mirroring OrcaSlicer's `loop_number++ → inset_count → max_bead_count` beading-stack mechanism) and D4 (`bridging_flow(bridge_flow_ratio, thick_bridges)` returns the ratio for bridge vertices, mirroring OrcaSlicer's real formula NOT a fabricated 0.85 constant). D-104f (concentric infill Arachne wiring) is registered as an open deviation; the corresponding red test stays red. D-104g (per-vertex `flow_factor` divergence from OrcaSlicer's per-path `Flow` model) is added as an open deviation per the grilling decision.

## Problem Statement

The audit found four pipeline-wide config keys (`detect_overhang_wall`, `overhang_reverse`, `overhang_reverse_internal_only`, `min_width_top_surface`, `alternate_extra_wall`) that are absent from the PnP pipeline as a whole (none registered, none consumed), plus a missing `bridging_flow()` implementation that should reduce `flow_factor` for bridge segments (OrcaSlicer's `LayerRegion.cpp:135` computes a flow factor via `base_flow.with_flow_ratio(bridge_flow_ratio)` — the canonical formula is a ratio, not a constant 0.85; PnP always emits `flow_factor = 1.0`). The fifth pipeline-wide gap is concentric infill Arachne wiring (G23 / D-104f) — too large to fit in this packet, deferred to a follow-up workstream; the corresponding red test stays red as the explicit success criterion for closing D-104f. A sixth deviation (D-104g) is added to document the per-vertex `flow_factor` model vs OrcaSlicer's per-path `Flow` model divergence — the `bridge_flow` ratio is correctly modelable per-vertex, but PnP doesn't model Flow height/width/thread_diameter the way OrcaSlicer does (the `thick_bridges` branch in the helper is the realization site). Six deviation rows are added to `docs/DEVIATION_LOG.md` (D-104b, D-104c, D-104d, D-104e, D-104f, D-104g); four of them close when this packet lands (D-104b/c/d/e), one stays open-but-deferred (D-104f), and one stays open as a limited divergence (D-104g).

The classic path already has `extra_perimeters_on_overhangs` (T-077, P108) and reads `only_one_wall_top`. This packet's D1 + D2 sub-tasks re-publish those keys in the arachne manifest for discoverability, register the missing keys (`detect_overhang_wall`, `overhang_reverse`, `overhang_reverse_internal_only`, `min_width_top_surface`), and add the new keys (`alternate_extra_wall`, `bridge_flow`, `thick_bridges`). D3 and D4 add the alternating-layer `max_bead_count` bump (mirrors OrcaSlicer's `loop_number++` beading-stack mechanism) and the ratio-based bridge flow reduction.

## Architecture Constraints

<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (manifest, `slicer-core/src/flow.rs` if the helper is in scope, both perimeter modules' `src/lib.rs`), the implementer MUST run `cargo xtask build-guests --check` and, if `STALE:` is reported, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.

<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.

- The 11 new arachne manifest keys (8 audit keys + `spiral_vase` + `sparse_infill_density` for the D3 gate + `only_one_wall_top` for the AC-2 read) are all read via the existing `ConfigView::get_bool` / `get_float` pattern; no WIT boundary change.
- The D-104f (concentric infill Arachne) gap is explicitly OUT OF SCOPE for this packet. The `arachne_parity_pipeline_concentric_infill_uses_arachne` test stays red at packet close. The deviation row is registered as `Status: Open — deferred to follow-up workstream`.
- D1 (`detect_overhang_wall`, `overhang_reverse`, `overhang_reverse_internal_only`) and D2 (`min_width_top_surface`) are manifest-only registrations. The red tests assert key presence, not behavior. The behavior implementation (overhang reverse logic, min-width-top-surface threshold for `only_one_wall_top`) is deferred to a follow-up packet; this packet does NOT implement the behavior.
- D3 (`alternate_extra_wall`) and D4 (`bridging_flow`) ARE behavior changes — they require source code in the perimeter modules, not just manifest entries.

## Data and Contract Notes

- **IR or manifest contracts touched:**
  - `Point3WithWidth.flow_factor: f32` — already present, no shape change. The new behavior sets it to `bridge_flow_ratio` (config-driven, default 1.0) for bridge vertices when `thick_bridges=false`, and leaves 1.0 when `thick_bridges=true`.
  - `WallFeatureFlags.is_bridge: bool` — already set per-vertex (packet 148). The D4 logic reads it.
  - `arachne-perimeters.toml [config.schema]` — 11 new entries: 4 D1 keys (`detect_overhang_wall`, `overhang_reverse`, `overhang_reverse_internal_only`, `extra_perimeters_on_overhangs`), 1 D2 key (`min_width_top_surface`), 1 D3 key (`alternate_extra_wall`), 2 D4 keys (`bridge_flow`, `thick_bridges`), 2 D3-gate keys (`spiral_vase`, `sparse_infill_density`), 1 AC-2 key (`only_one_wall_top`). All bool/float; default values match `docs/ORCA_CONFIG_REFERENCE.md`.
  - `classic-perimeters.toml [config.schema]` — 7 new entries (4 missing keys + `alternate_extra_wall` + `bridge_flow` + `thick_bridges`; `extra_perimeters_on_overhangs` and `only_one_wall_top` are already there).
- **WIT boundary considerations:** none. The new config keys are read via the existing `ConfigView::get_bool`/`get_float` pattern. The `bridging_flow()` helper is host-side, not guest-side.
- **Determinism or scheduler constraints:** none beyond what packet 148 + classic already enforce. The bridge flow reduction is a pure function of config (`bridge_flow_ratio` or 1.0) per vertex, deterministic.

## Locked Assumptions and Invariants

- The 11 new arachne manifest keys (and 7 classic) MUST have **identical** defaults to the OrcaSlicer reference. The implementer should `diff` against `docs/ORCA_CONFIG_REFERENCE.md` before committing. `min_width_top_surface`'s default is verified via sub-agent dispatch BEFORE commit (the spec's guess of 1.2mm is unverified).
- The `bridging_flow()` helper MUST be defined in `slicer_core::flow` (not in `slicer-sdk`); both perimeter modules call it. Single source of truth.
- The bridge flow factor MUST be applied to BOTH the classic and arachne paths. Classic was not in the audit's red tests, but the parity is implied by the audit's framing ("the PnP pipeline as a whole").
- The D3 (`alternate_extra_wall`) mechanism MUST be a `max_bead_count` bump on odd layers, NOT a wall-count mutation. Gate: `layer_index % 2 == 1 && !spiral_vase && sparse_infill_density > 0`. Mirrors OrcaSlicer's `loop_number++` at `PerimeterGenerator.cpp:1227` (classic) and `:2133` (arachne).
- The D-104f row's `Status` field is `Open — deferred to follow-up workstream`; its `Target Close` field is `— (deferred; follow-up workstream TBD)` (no fabricated schedule). The corresponding red test `arachne_parity_pipeline_concentric_infill_uses_arachne` STAYS RED at packet close. This is the explicit success criterion for the deviation registration (NOT a defect).
- The D-104g row's `Status` field is `Open` (documents the per-vertex `flow_factor` vs OrcaSlicer's per-path `Flow` model divergence — the `thick_bridges` branch in the helper is the canonical realization site; the `bridge_flow` ratio itself is correctly modelable per-vertex, so this is a limited divergence, not a gap).
- The D-104b/c/d/e rows' `Status` fields flip to `Closed — 2026-07-09: packet 149` at packet close.
- None — change is reversible via existing config defaults (the 4 D1 keys default to `false`; `min_width_top_surface` defaults to the OrcaSlicer-resolved value; `alternate_extra_wall` defaults to `false`; `bridge_flow` defaults to `1.0`; `thick_bridges` defaults to `false`). The bridge flow factor is gated on `is_bridge` being set, which only happens when `region.bridge_areas()` is non-empty. No behavior locks introduced beyond the test suite.

## Risks and Tradeoffs

- **Risk:** the bridge flow factor test fixture (AC-4) needs `region.bridge_areas()` to be non-empty. The audit's test reads the `arachne-perimeters` source via `include_str!`; it does not actually exercise a bridge-area fixture. The implementer must add a unit test that does (`arachne-perimeters/tests/bridge_flow_factor_tdd.rs`). **Mitigation:** the unit test scaffold is straightforward — use the existing `SliceRegionViewBuilder` test-support to set `bridge_areas` to a small polygon, run the perimeter module, and assert the wall loop's `flow_factor`.
- **Risk:** the D3 (alternating_extra_wall) implementation requires a wall count parameter. The classic and arachne paths have different wall-count sources (classic: explicit `wall_count` config; arachne: derived from the beading-strategy stack). The implementer must be careful to apply the alternating logic to the FINAL wall count, not the base config. **Mitigation:** for arachne, the wall count is determined by `run_arachne_pipeline`'s output (one wall per `inset_idx`); the alternating logic adds a +1 on odd layers, which means the `inset_idx` range grows by 1 on odd layers. The unit test verifies the resulting wall count.
- **Risk:** the 6 new deviation log rows change the table's row count. The `cargo xtask check-deviations` tool (per `docs/14_deviation_audit_history.md` "non-authoritative views") regenerates `docs/07_implementation_status.md`'s Open Deviation Map. **Mitigation:** the implementer runs `cargo xtask check-deviations` after the deviation log edit; the regenerated file is committed in the same packet.
- **Risk:** the new manifest keys may collide with packet 148's keys (`precise_outer_wall`, `seam_candidate_angle_threshold_deg`). **Mitigation:** the names are distinct (`detect_overhang_wall` vs `precise_outer_wall`; `seam_candidate_angle_threshold_deg` is unique). The implementer should grep for each new key against the existing manifest to confirm no collision.
