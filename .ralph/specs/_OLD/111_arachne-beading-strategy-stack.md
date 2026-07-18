---
status: implemented
packet: 111_arachne-beading-strategy-stack
task_ids:
  - T-210
  - T-211
  - T-212
  - T-213
  - T-214
  - T-215
  - T-215b
  - T-216
  - T-218
---

# 111_arachne-beading-strategy-stack

## Goal

Port the OrcaSlicer Arachne BeadingStrategy stack into `slicer_core::beading`: define the `BeadingStrategy` trait (T-210), port all 5 strategies — `Distributed` (Gaussian-weighted width distribution), `Redistribute` (preserve outer-wall width consistency), `Widening` (thin-feature single-wall regime), `OuterWallInset` (outer-wall toolpath offset decorator), `Limited` (max-bead-count cap with internal 0-width sentinel insertion) — implement the T-215b strip-pass that drops zero-width beads from output before `WallLoop` assembly (per D-9 in the roadmap — the decision is already made; this packet implements it and registers the rationale as `D-111-ARACHNE-SENTINEL-STRIP` in `docs/DEVIATION_LOG.md`), wire the `BeadingStrategyFactory` that composes the stack in the canonical order `Distributed → Redistribute → Widening → OuterWallInset → Limited`, and register all 11 Arachne `m_params.*` config keys in `docs/15_config_keys_reference.md` and the `arachne-perimeters.toml` manifest.

## Problem Statement

OrcaSlicer's Arachne wall generator selects per-segment bead widths through a stack of `BeadingStrategy` decorators. Each decorator transforms the `Beading` produced by the inner strategy: `Distributed` is the base (Gaussian-weighted thickness distribution), `Redistribute` preserves outer-wall width consistency, `Widening` handles features below `min_input_width` as single thin walls, `OuterWallInset` shifts the outer toolpath inward by a configured offset, and `Limited` caps total bead count and inserts zero-width sentinel beads at the cap boundary (an internal data invariant the propagation pass uses). Without all 5 strategies AND the canonical wrapping order from `BeadingStrategyFactory::create_strategy`, the centrality + bead-count passes in P112 (T-220..T-222) cannot produce wall widths that match OrcaSlicer.

The zero-width sentinels from `Limited` are an internal book-keeping mechanism: downstream centrality propagation reads them to keep bead-index alignment, but the wall-loop output should never carry zero-width entries. P96 originally surfaced this as D-9 (Arachne zero-width-sentinel handling) with two options: (a) coordinate with infill modules to recognize and skip sentinels, (b) strip sentinels before external output. D-9 closed via option (b) — T-215b implements the strip-pass at `LimitedBeadingStrategy::compute_and_strip`, and the deviation closure entry records the rationale.

T-218 registers the 11 `m_params.*` config keys both in `docs/15_config_keys_reference.md` (descriptions + defaults + units) and in `modules/core-modules/arachne-perimeters/arachne-perimeters.toml` (manifest schema blocks). **That manifest does NOT exist yet:** P108 (`implemented`) deleted the old fake `arachne-perimeters` module (its manifest + its 512-line iterative-inset impl), and P110/T-205 CREATES a fresh skeleton manifest — so AC-9 forward-deps on P110. P111 adds ONLY the 11 new schema blocks to the P110-created manifest; the implementer reads P110's skeleton manifest at activation and confirms no key collides before adding. The 11 keys' values are passed into `BeadingStrategyFactory::create_stack` at P112's T-230 wire-up time (P111 does not touch any `run_perimeters` code path).

This is a pure-data packet — no IR changes, no WIT changes, no host changes. Every test runs as a `slicer-core` unit test against recorded OrcaSlicer reference outputs.

## Architecture Constraints

<!-- snippet: coord-system -->
- **Coordinate system hazard.** All `Beading` widths and toolpath_locations are in slicer units (1 unit = 100 nm). OrcaSlicer config defaults are typically in real units (mm) or SCALED OrcaSlicer units (1 unit = 1 nm). The implementer MUST translate via `mm_to_units` or the explicit `/100` rule per `docs/08_coordinate_system.md`. Confirm each of the 11 config keys' translated default during the PrintConfig.cpp LOCATIONS dispatch (see OrcaSlicer Reference Obligations).

- **Object-safe trait.** `BeadingStrategy` MUST be object-safe so `Box<dyn BeadingStrategy>` works in the decorator chain. No generic methods; no `Self` returns; no associated types tied to `Self`. If reflection/downcast is needed (AC-8), add an explicit `fn type_label(&self) -> &'static str` trait method.
- **No floating-point HashMap keys.** Determinism is required: `Beading` outputs MUST be byte-identical for byte-identical inputs. No HashMap over `f64` keys; if keying is needed, use sorted `Vec<(f64, T)>` with stable ordering.
- **No panics outside debug-asserts.** Strategy `compute` returns a `Beading` — invariant violations (`toolpath_locations.len() != bead_widths.len()`) get a `debug_assert_eq!` in debug builds and silent acceptance in release (with the caller responsible for downstream validation). Documented in AC-N1.

## Data & Contract Notes

- **`Beading`**: `{ total_thickness: f64, bead_widths: Vec<f64>, toolpath_locations: Vec<f64>, left_over: f64 }`. Invariant: `bead_widths.len() == toolpath_locations.len()` (debug-asserted); `total_thickness == bead_widths.iter().sum::<f64>() + left_over` within 0.0001 mm; `bead_widths` ordered from outermost to innermost (matches OrcaSlicer ordering).
- **`BeadingFactoryParams`**: bundles the 11 `m_params.*` values + the `outer_wall_offset` toggle. Default trait impl reads from manifest defaults; runtime values come from `ConfigView` in P112 wire-up.
- **Trait surface (final)**:
  ```rust
  pub trait BeadingStrategy: Send + Sync {
      fn compute(&self, thickness: f64, bead_count: usize) -> Beading;
      fn optimal_bead_count(&self, thickness: f64) -> usize;
      fn get_transition_thickness(&self, lower_bead_count: usize) -> f64;
      fn optimal_thickness(&self, bead_count: usize) -> f64;
      fn type_label(&self) -> &'static str;  // for AC-8 composition verification
  }
  ```
- **No serde on Beading**: Beading is a runtime value, not an IR type. JSON goldens use a test-helper `BeadingForTest` newtype with serde, converting back to `Beading` at read time.

## Locked Assumptions and Invariants

- `beading/` placed in `slicer-core` per docs/13 §Out of Scope (Tier-2 pipeline data structures belong in slicer-core). Part of roadmap-wide correction `D-ROADMAP-CRATE-PLACEMENT`.
- OrcaSlicer wraps decorators in the order `Limited(OuterWallInset(Widening(Redistribute(Distributed))))` where `Limited` is the outermost — the call site sees a `Limited` and dispatch flows inward. This packet matches.
- `WideningBeadingStrategy` checks input thickness against `min_input_width` BEFORE delegating. Below threshold: produces single thin bead and returns early (does NOT call inner). At/above threshold: delegates to inner unmodified.
- `LimitedBeadingStrategy` caps `optimal_bead_count` at `max_bead_count`. Sentinel insertion happens in `compute` when delegated `bead_count` exceeds the cap. The actual mechanics: sentinels are zero-width entries at the cap boundary; their `toolpath_locations` are placeholder values that downstream centrality propagation reads but doesn't follow.
- `OuterWallInsetBeadingStrategy` modifies ONLY `toolpath_locations[0]` and `toolpath_locations[bead_widths.len() - 1]` by `±outer_wall_offset`; widths and inner locations untouched. The decorator is a no-op when `outer_wall_offset == 0`.
- Strip-pass strips zero-width entries in pairs: `(widths[i], locations[i])` removed together; length invariant preserved post-strip.

## Risks and Tradeoffs

- **Float comparison fragility.** The 10-thickness `Distributed` golden uses 0.0001 mm (1 unit) tolerance. If boostvoronoi's primitives or the Gaussian decay constant differ by even 1 ULP between platforms, tests may flake on x86 vs aarch64. Mitigation: assert `(a - b).abs() < 1e-4` (slicer units) rather than `assert_eq!` on raw f64.
- **Trait object overhead.** Box<dyn> dispatch adds vtable cost per call. Acceptable for now; M2 perf budget is not in scope for this packet.
- **OrcaSlicer SUMMARY drift.** A SUMMARY ≤ 150 words may omit edge cases (e.g., what `LimitedBeadingStrategy` does when `bead_count == 0`). Mitigation: the goldens are the source of truth; if a strategy can't make the golden green, re-dispatch a tighter SUMMARY for that edge case.
- **`D-111-ARACHNE-SENTINEL-STRIP` rationale.** The strip-pass is a deliberate divergence from OrcaSlicer (which carries zero-width sentinels into the toolpath output and relies on infill modules to skip them). The `DEVIATION_LOG.md` entry MUST explain WHY this codebase strips instead — namely, the WIT-boundary `WallLoop` type's invariant that `bead_widths.iter().all(|&w| w > 0.0)` (avoiding a contract change at the boundary). D-9 in the roadmap already records the decision; `D-111-ARACHNE-SENTINEL-STRIP` records the implementation log entry using the log's `D-<pkt>-<SLUG>` convention.
