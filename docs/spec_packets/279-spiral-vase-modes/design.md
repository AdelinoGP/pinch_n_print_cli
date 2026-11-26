# Design: 279-spiral-vase-modes

## Controlling Code Paths

- Primary code path: `DefaultGCodeEmitter::emit_gcode` (`crates/slicer-gcode/src/emit.rs`) — the SpiralVase stage runs after layer lowering, before serialization, as a post-processing filter over generated layers (canonical `GCode::process_layers` precedent).
- Neighboring tests/fixtures: `crates/slicer-gcode/tests/` per-feature `_tdd.rs` files (new `spiral_vase_modes_tdd.rs`); `crates/slicer-scheduler/tests/integration/` (extend via new `spiral_vase_modes_tdd` cases); `modules/core-modules/machine-gcode-emit/tests/machine_gcode_emit_tdd.rs` (sibling pattern for new `spiral_timelapse_gate_tdd.rs`).
- OrcaSlicer comparison: see `requirements.md` §OrcaSlicer Reference Obligations; do not repeat delegation rules.

## Architecture Constraints

- The PnP way: SpiralVase is a mode branch inside the existing host emitter, not a new module. Per the map's trigger test, a module branching internally over a mode it implements itself stays as it is — no `claim:*` holder, no new module, no WIT change.
- No second mechanism: `spiral_mode` (Orca spelling) is adopted and the live `spiral_vase` dispatch becomes a fallback alias. The alias lives in exactly one place (the scheduler live-wiring read); manifests carry both rows only during transition.
<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and inspect its exit code: exit 0 means fresh, non-zero means stale (a distinct exit code signals `wasm-tools` is unavailable). Never use `rg -q 'STALE:'` — a `wasm-tools`-missing infrastructure error prints no `STALE:` and would read as fresh. If stale, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.
<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.
- Percent-key hazard (open ticket 128): `spiral_mode_max_xy_smoothing` is `FloatOrPercent` with canonical spelling `200%`. Manifest bounds check the raw number as percent while `ConfigView::get_abs_value` reads a bare float as fraction — so tests must spell the canonical percent string, and the packet must not claim bare-number parity beyond what the test proves.
- Bool spelling (map value-spelling note, tickets 112/132): canonical bools deserialize `1`/`0`; the packet specifies canonical spellings for its keys and changes nothing in `serialize.rs` (ticket 132 owns that fix).

## Code Change Surface

- Selected approach: (1) five `ResolvedConfig` fields + host-keys rows + perimeter-manifest `spiral_mode` alias rows; (2) scheduler live-wiring reads `spiral_mode` first, `spiral_vase` fallback, plus three fatal validations — the classic-forcing branch in `execution_plan.rs` needs no change because it already keys off the threaded boolean the live wiring computes; (3) new pure SpiralVase stage in `crates/slicer-gcode` hooked into `emit_gcode`; (4) `!spiral` clause in the machine-gcode-emit TimeLapse gate.
- Exact functions, traits, manifests, tests, and fixtures:
  - `crates/slicer-ir/src/resolved_config.rs` — five field declarations in `declare_resolved_config!` (`spiral_mode: bool = false`, `spiral_mode_smooth: bool = false`, `spiral_starting_flow_ratio: f32 = 0.0`, `spiral_finishing_flow_ratio: f32 = 0.0`, `spiral_mode_max_xy_smoothing` percent-kind `= 200%` form).
  - `crates/slicer-gcode/src/spiral_vase.rs` (new) — pure `apply_spiral_vase(layers, params)` stage: Z-ramp, XY smoothing under cap with extrusion rescale, flow transitions, tiny-move removal.
  - `crates/slicer-gcode/src/emit.rs` — `DefaultGCodeEmitter::emit_gcode` calls the stage when the mode is on.
  - `crates/slicer-scheduler/src/execution_plan.rs` — classic-forcing branch keyed off the already-threaded boolean (read-only; no change needed).
  - `crates/slicer-wasm-host/src/execution_plan_live.rs` — unified read (`spiral_mode`, fallback `spiral_vase`) + three fatal validations.
  - `modules/core-modules/machine-gcode-emit/src/lib.rs` — TimeLapse gate gains the `!spiral` clause (mode via module config declaration).
  - Perimeter manifests (`arachne-perimeters`, `classic-perimeters`) — add `spiral_mode` bool rows beside `spiral_vase`.
  - Tests: new `spiral_vase_modes_tdd` (slicer-gcode), new cases in `scheduler_integration` (`spiral_vase_modes_tdd`), new `spiral_timelapse_gate_tdd` (machine-gcode-emit), new `spiral_vase_modes_tdd` (runtime integration).
- Rejected alternatives and reasons:
  - New SpiralVase module with claim holders: rejected — the mode branches inside emission the emitter already owns; no cross-module algorithm selection exists here.
  - Keeping `spiral_vase` and `spiral_mode` as independent keys: rejected — same decision point; two keys would be a second mechanism.
  - Removing `spiral_vase` outright: rejected — live user-facing key; alias-then-retire is the rename-workstream shape.

## Files in Scope (read + edit)

Primary surface is wider than three files because the ticket is cross-cutting (orchestration + emitter + guest gate); each slice below is small and ordered so later steps never reopen earlier ones.

- `crates/slicer-ir/src/resolved_config.rs` - role: host field declarations; expected change: five fields with canonical defaults.
- `crates/slicer-gcode/src/spiral_vase.rs` (new) + `crates/slicer-gcode/src/emit.rs` - role: SpiralVase stage + hook; expected change: pure stage function plus one call site.
- `crates/slicer-scheduler/src/execution_plan.rs` + `crates/slicer-wasm-host/src/execution_plan_live.rs` - role: unified dispatch + validation; expected change: alias read plus three fatal errors in live wiring (the `execution_plan.rs` forcing branch already keys off the threaded boolean).
- `modules/core-modules/machine-gcode-emit/src/lib.rs` - role: time-lapse gate; expected change: one `!spiral` clause plus config declaration.
- `docs/config/host-keys.toml`, perimeter manifests, `docs/15_config_keys_reference.md` (regenerated), `04`/`05` annotations - role: config surface and ledger; expected change: rows plus regeneration.

## Read-Only Context

- `crates/slicer-scheduler/src/execution_plan.rs` - lines around `dedup_same_claim_modules_with_wall_generator` and `SPIRAL_VASE_CONFIG_KEY` only - purpose: alias edit site.
- `crates/slicer-wasm-host/src/execution_plan_live.rs` - lines around the `spiral_vase` read only - purpose: unified-read edit site.
- `modules/core-modules/machine-gcode-emit/src/lib.rs` - lines around `run_gcode_postprocess` TimeLapse gate only - purpose: gate edit site.
- `crates/slicer-gcode/src/flavor.rs` - purpose: confirm no flavor interaction (SpiralVase is flavor-independent).

## Out-of-Bounds Files

- `OrcaSlicerDocumented/...` - delegate; never load
- `target/`, `Cargo.lock`, generated code, vendored dependencies - never load
- `crates/slicer-gcode/src/serialize.rs` - frozen by AC-N3; never touch
- Unrelated crates - delegate symbol lookups; do not browse

## Expected Sub-Agent Dispatches

- Question: enumerate `ResolvedConfig` struct-literal sites affected by adding five fields; scope: `crates/ modules/` Rust tests and non-tests; return: `LOCATIONS` (≤20 entries); purpose: implementation-plan Step 1 blast radius.
- Question: confirm `machine-gcode-emit` module test-binary naming and config-declaration pattern for a new bool; scope: `modules/core-modules/machine-gcode-emit/`; return: `FACT` (≤5 lines); purpose: Step 4 fixture shape.
- Question: summarize `PrintObject` spiral slicing reads beyond classic-forcing; scope: oracle only; return: `SUMMARY` (≤200 words); purpose: Open Questions FWD resolution.

## Data and Contract Notes

- IR/manifest contracts: host-resolved keys flow via `ResolvedConfig::to_config_map`; the guest TimeLapse gate reads its declared module key through `ConfigView` — so the machine-gcode-emit manifest must declare the spiral flag it gates on (undeclared keys are silently filtered).
- WIT boundary: none — no new WIT type, no guest/host signature change.
- Determinism/scheduler constraints: SpiralVase is a pure function of ordered layer input plus resolved params; Z-ramp uses cumulative extruding length so input order is the output order — no reordering, no nondeterminism.

## Locked Assumptions and Invariants

- `0` disables its own flow-ramp end (documented interpretation; canonical edge semantics unverified — recorded, not a gap).
- When both spellings are present, `spiral_mode` wins; the fallback never errors.
- SpiralVase requires relative extrusion; absolute extrusion fails validation before processing.
- XY smoothing never moves a point farther than the cap from its un-smoothed position.

## Risks and Tradeoffs

- Bounded canonical reads could not evidence slicing-side changes beyond classic-forcing and validation; if the delegated FWD read finds more, the packet grows a slicing step rather than declaring the keys done.
- `FloatOrPercent` coercion for the cap may need the ticket-128 ruling mid-implementation; the fallback is percent-string-only acceptance with a recorded divergence.
- The alias transition keeps two spellings visible in manifests; retiring `spiral_vase` is deferred rename-workstream scope, flagged in 04/05.

## Context Cost Estimate

- Aggregate: `M`
- Largest step: `M` (Step 3, emitter stage)
- Highest-risk dispatch and required return format: struct-literal blast-radius `LOCATIONS` (Step 1) — a missed site breaks compilation workspace-wide.

## Open Questions

- `[FWD]` Do canonical's `LayerRegion`/`PrintObject`/ironing spiral reads change slicing beyond forcing classic perimeters? Implementer resolves via one delegated oracle `SUMMARY` in Step 2; default is no slicing change with rationale recorded in the step. Not activation-blocking.
- None other. No `[BLOCK]`.
