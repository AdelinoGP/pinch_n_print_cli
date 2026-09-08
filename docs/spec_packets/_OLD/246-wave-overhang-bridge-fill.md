---
status: implemented
packet: 246-wave-overhang-bridge-fill
task_ids:
  - TASK-356
---

# 246-wave-overhang-bridge-fill

## Goal

Ship the `com.core.wave-overhangs` bridge-fill module — a PnP bridge-fill adaptation of the
canonical `WaveOverhangs.cpp` generator that emits order-locked, anchor-first `BridgeInfill` waves
over external bridge areas, excludes internal bridges (unlocked rectilinear fallback), and falls back
to conventional rectilinear scanlines when waves cannot be generated.

## Problem Statement

PnP has no bridge-fill pattern that bonds its first fronts to solid ground. Canonical
`WaveOverhangs.cpp` (OrcaSlicer fork `dennisklappe/OrcaSlicer-WaveOverhangs`) extrudes an anchor band
into supported material beside the overhang so wave fronts bond to solid ground, then meanders
wave-shaped fronts across the overhang. This packet ports that generator as a PnP bridge-fill module
over the existing `bridge_areas` sites, with two deliberate adaptations: (1) it is a bridge-fill
adaptation, not a faithful port of the canonical stage/site semantics — the generator runs over
`bridge_areas` rather than replacing selected overhang perimeters; (2) internal bridges are excluded
and get unlocked rectilinear fallback. The waves are order-locked (packet 244) so the linker,
optimizer, and emitter (packet 245) preserve their anchor-first print order.

## Architecture Constraints

- The generator is an own copy inside the module (ADR-0026 single-caller rule): no extraction to
  `slicer-core`, no sharing with `rectilinear-infill`. The fallback rectilinear scanlines are an
  owned copy inside the wave module, not a call into `rectilinear-infill`.
- Holder selection forces waves: being configured as `bridge_fill_holder` is the enable (equivalent
  to canonical `use_instead_of_bridges = true` for external bridge sites). No master enable bool.
- The `internal-bridge-areas` accessor is a WIT/SDK view addition over the already-existing
  `SlicedRegion.internal_bridge_areas` field; no IR schema version bump (the `slicer:ir-handles`
  package is unversioned per ADR-0044).
<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and inspect its exit code: exit 0 means fresh, non-zero means stale (a distinct exit code signals `wasm-tools` is unavailable). Never use `rg -q 'STALE:'` — a `wasm-tools`-missing infrastructure error prints no `STALE:` and would read as fresh. If stale, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.
<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.

## Data and Contract Notes

- IR/manifest contracts: `holds = ["claim:bridge-fill"]` only; `[ir-access] reads = ["SliceIR"] writes
  = ["InfillIR"]`. No new claim id; no IR schema bump.
- WIT boundary: `slice-region-view` gains `internal-bridge-areas` (additive, unversioned package).
- Determinism/scheduler constraints: smart traversal must be deterministic (double-run identical);
  native and wasm dispatch must agree.

## Locked Assumptions and Invariants

- Waves are order-locked with one tag per connected wave domain; the linker/optimizer/emitter (packet
  245) preserve them verbatim and carve untagged fill around their swept footprint.
- Internal-qualified polygons never receive waves; they get unlocked rectilinear fallback and the host
  `InternalBridgeInfill` constructor still emits them.
- `speed_factor` outside `[0.05, 5.0]` is a fatal rejection, never a silent clamp.

## Risks and Tradeoffs

- The generator is a substantial port; the own-copy rule (ADR-0026) means no shared geometry helper,
  so the port must be self-contained and tested at the module boundary.
- The end-to-end AC depends on the visual-debug typed-capture exposing `order_lock`; if it does not,
  the AC driver must use a `run_pipeline`-based capturing runner instead (see `[FWD]` below).
