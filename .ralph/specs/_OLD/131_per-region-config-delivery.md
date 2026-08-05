---
status: implemented
packet: 131_per-region-config-delivery
task_ids: [TASK-256]
---

# 131_per-region-config-delivery

## Goal

Deliver per-region config to guest modules: replace the first-match global `ConfigView` derivation with `RegionKey`-matched resolution from `RegionMapIR`'s interned pool, expose it through a config accessor on the region views (additive WIT bump), and open the roadmap's golden carve window with a baseline survey.

## Problem Statement

Dispatch built ONE global `ConfigView` from the FIRST `RegionKey` matching the layer index (`crates/slicer-wasm-host/src/dispatch.rs` first-match derivation) — a latent wrong-config bug for painted multi-region layers and a blocker for modifier sub-regions (packet 132) and per-region infill spacing (packets 133-135). The first-match bug fix is the backlog row's substance; multi-region layers may legitimately change output (they currently read an arbitrary region's config), so the golden carve window opens here.

## Architecture Constraints

<!-- snippet: wasm-staleness -->
- WIT change (additive): both `slice-region-view` and `perimeter-region-view` gain `config: func() -> config-view`, reusing the existing `slicer:config/config-types.config-view` resource via a new `use` in the `ir-handles` interface (locked shape — no six per-key getters). Guest rebuild ceremony applies.
- Config DELIVERY only: no config values/defaults change, no geometry split (packet 132), no infill algorithm change.
- FORWARD-DEP on packet 130 (adjacent WIT churn): this packet's world-version bump computed as +0.1 from whatever 130 lands as.

## Data and Contract Notes

- Dispatch derivation replaced by full `RegionKey` match (object + region + variant chain); the looser substring `global_layer_index == layer` also matches two unrelated sites (`push_perimeter_regions`'s seam-plan lookup, a separate `held_claims_map` resolution) — out of scope; AC-2 greps the exact `.find(...)` expression.
- Module consumption: per-region values (e.g. `infill_density`, `line_width`, speed keys) readable inside the module's per-region loop via the region-view config accessor, retaining the module-level `ConfigView`.
- Behavior guards: single-region layers read exactly the config they read before (AC-N1); `regression_wedge.stl` default-config g-code SHA-256 byte-identical (AC-N2, digest `8a3b645e…` — later re-blessed by packet 136).
- Golden carve: `.ralph/specs/131_per-region-config-delivery/carve-list.md` with one `### <test path>` heading + `Reason:` + `Baseline:` per carved test (heading count == baseline count).
- Multi-region golden survey + carve window per roadmap D6; packet 136 restores.

## Locked Assumptions and Invariants

- The config accessor returns a per-region resolved config; AC-1 proves two densities (0.15 / 0.40) on one layer read distinctly.
- 03_wit_and_manifest.md / 05_module_sdk.md Doc Impact: region-view config accessor + per-region config usage example.

## Risks and Tradeoffs

- Painted multi-region fixtures may legitimately change output — that is the carve window's purpose, surveyed in the carve-list with per-test baselines.
- The first-match fix is the latent arbitrary-config bug fix for painted multi-region layers (ADR-0030 Decision point 3).

## Implementation Deviations (recorded at close)

None. The carve-list survives as the packet's record of the survey.
