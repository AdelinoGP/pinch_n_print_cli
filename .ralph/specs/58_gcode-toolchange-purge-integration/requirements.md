# Requirements: 58_gcode-toolchange-purge-integration

## Packet Metadata

- Grouped task IDs:
  - `TASK-143` — WipeTower live finalization (closed in packet 17; integration gap remains).
  - `TASK-152b` — Mixed-tool ordering / `push-tool-change` (closed in packet 19; emission gap remains).
  - `TASK-120d2` — Retract/unretract pair emission (closed in packet 15; toolchange-specific wrap missing).
- Backlog source: `docs/07_implementation_status.md`
- Packet status: `draft`
- Aggregate context cost: `M`

## Problem Statement

Three closed packets each completed their own slice cleanly, but the slices never met:

- Packet 17 (`17_wipe-tower-finalization-live-path`) ported the `wipe-tower` module onto the live `run_finalization()` path. The module exists and runs in `PostPass::LayerFinalization`. **Gap**: its emitted geometry does not surface as `;TYPE:Wipe tower` (or `Prime tower`) blocks in the final `.gcode` for multi-material fixtures.
- Packet 19 (`19_path-optimization-tool-order-and-cooling-policy`) added deterministic mixed-tool ordering and `push-tool-change` via the WIT layer-collection-builder. **Gap**: when `path-optimization-default` pushes a tool change, the downstream `GCodeEmitter` still writes a bare `T<n>` line at `crates/slicer-host/src/gcode_emit.rs:1155-1156` with no retract before and no prime/wipe after.
- Packet 11 (`11_orca-gcode-emission-contract`) defined the canonical `;TYPE:<RoleName>` role labeling. **Gap**: an `ExtrusionRole::WipeTower` (or equivalent) serialization branch is not exercised because the wipe-tower module does not tag its entities with that role.

User reproduction (Slicer A = OrcaSlicer reference, Slicer B = ModularSlicer):

- Slicer A emits one `;TYPE:Prime tower` block per layer of a 292-layer multi-color print.
- Slicer B emits zero `;TYPE:Prime tower` blocks for the same print.
- Slicer B transitions straight from one filament's extrusion to `T<n>` and then to an extruding `G1 ... E+` move on the print model, leaving the previous color smeared on the part.

No prior packet is being superseded — each completed its declared scope. This packet closes the **integration gap** between them, reusing the existing `wipe_tower_enabled` config envelope and shipping as a single-release bugfix with a `docs/DEVIATION_LOG.md` entry.

## In Scope

- Inject retract/prime/wipe `PrintEntity` rows into `LayerCollectionIR` around every `ToolChange` from the `wipe-tower` module.
- Emit `;TYPE:Wipe tower` markers from `GCodeSerializer` for those entities via a serialization branch on `ExtrusionRole::WipeTower`.
- Add `PostpassError::MissingToolchangePurge` (additive variant on `crates/slicer-host/src/postpass.rs:39-59`) when a `ToolChange` is not bracketed by purge entities under `wipe_tower_enabled=true`.
- One synthetic multi-material STL fixture at `crates/slicer-host/tests/fixtures/multi_color_cube.stl` plus a checked-in OrcaSlicer reference G-code at `crates/slicer-host/tests/fixtures/multi_color_cube.orca.gcode`.
- New integration test file `crates/slicer-host/tests/gcode_toolchange_wrapping.rs` (TDD-first).
- New unit tests inside `modules/core-modules/wipe-tower/src/lib.rs` (`#[cfg(test)] mod tests`) for role marker emission and geometry placement.
- A single `docs/DEVIATION_LOG.md` entry covering the integration completion across packets 17/19/11.

## Out of Scope

- Any new config key. Reuse `wipe_tower_enabled`, `wipe_tower_purge_volume`, `wipe_tower_x`, `wipe_tower_y`, `wipe_tower_width`, `line_width`.
- Ramming/cooling-tube dynamics from OrcaSlicer (parity-deferred).
- Tree/grid tower interior infill — keep the rectilinear pattern from packet 17.
- The 3-release N/N+1/N+2 rollout from `docs/11` — this is a bugfix.
- Changes to the WIT layer-collection-builder interface (entity insertion uses existing exports).
- Other `PostPass::LayerFinalization` modules (`skirt-brim`, `part-cooling`, `top-surface-ironing`).

## Authoritative Docs

- `docs/02_ir_schemas.md` — > 600 lines; **delegate** with question: "list every variant of `ExtrusionRole` and the exact field shape of `ToolChange`, `PrintEntity`, and `LayerCollectionIR.tool_changes`; FACT ≤ 5 lines for the enum + SNIPPETS ≤ 30 lines for the structs."
- `docs/03_wit_and_manifest.md` — > 300 lines; **delegate** for the wipe-tower manifest schema and `FinalizationOutputBuilder` WIT export list.
- `docs/04_host_scheduler.md` — direct read of the LayerFinalization → GCodeEmit transition section only.
- `docs/08_coordinate_system.md` — direct read (units math is required for tower geometry).
- `docs/09_progress_events.md` — direct read; confirm no existing event contract is being violated. No new event is required.
- `docs/11_operational_governance_and_acceptance_gate.md` — range-read §1 only (DEVIATION_LOG entry format).

## OrcaSlicer Reference Obligations

All delegated. Never load `OrcaSlicerDocumented/` into the implementer's own context.

- `WipeTower2.cpp:1557-1640` — borrow Unload/Change/Load/Wipe sequence ordering.
- `WipeTower2.cpp:1603,1646` — note Orca's `;Wipe_Tower_Start` / `;Wipe_Tower_End` form; ModularSlicer must use `;TYPE:Wipe tower` per packet 11.
- `Print.cpp:3180-3268` — per-layer toolchange planning reference.
- `GCode.cpp:7624` — `set_extruder()` retract → filament_end → toolchange flow reference.
- `WipeTower2.cpp:2258-2270` — `flush_volumes_matrix` purge-volume table reference.
- **Deliberately not borrowed**: Orca's ramming velocity profile, cooling-tube load timing, `;Wipe_Tower_Start/End` marker style (rejected — packet 11 owns the marker contract).

## Acceptance Summary

- **Positive cases (seven)**:
  1. `ToolChange` bracketed by retract + travel + prime + wipe entities in IR (Rust test).
  2a. Retract (`E-`) precedes every `T<n>` within the 5 preceding lines on the produced `.gcode` (awk).
  2b. Positive-`E` `G1` follows every `T<n>` within the 10 following lines on the produced `.gcode` (awk).
  3. Per-`(from_tool, to_tool)` purge volume within `[0.80, 1.20]` × OrcaSlicer reference (Rust test).
  4. Exactly one `;TYPE:Wipe tower` marker per wipe-tower block, role reverts on next entity (Rust test).
  5. Marker count `≥ L` for `L` tool-change-containing layers (python).
  6. Tower polygon within stub `bed_polygon` and outside stub object footprints (Rust test; real host-service bed-bounds enforcement is deferred per the Step 6 `docs/DEVIATION_LOG.md` entry).

- **Negative cases (three)**:
  1. `ToolChange` without surrounding purge entities → `PostpassError::MissingToolchangePurge`.
  2. Bare `T<n>` followed by extruding `G1 ... E+` in the file → CLI scan exits non-zero.
  3. Multi-tool file with zero `;TYPE:Wipe tower`/`Prime tower` blocks → CLI scan exits non-zero.

- **Measurable outcomes**: zero `MissingToolchangePurge` errors on the fixture; ≥ 1 `;TYPE:Wipe tower` marker per tool-change layer; purge volume per `(from, to)` pair within `[0.80, 1.20]` × OrcaSlicer reference; no clippy warnings.

- **Cross-packet impact**: no packet supersession; unblocks any downstream multi-material correctness work.

## Verification Commands

- `cargo check --workspace` — fast type-check before any test run.
- `cargo clippy --workspace -- -D warnings` — required pre-close gate.
- `cargo test -p slicer-host --test gcode_toolchange_wrapping` — all wrapping tests.
- `cargo test -p wipe-tower` — all module tests.
- `./modules/core-modules/build-core-modules.sh` — WASM module rebuild.
- `cargo run --bin slicer-cli --release --slice --input crates/slicer-host/tests/fixtures/multi_color_cube.stl --output target/test-output/multi_color_cube.gcode` — end-to-end slice.
- AC and NC awk/python scripts from `packet.spec.md` against `target/test-output/multi_color_cube.gcode`.

All are delegation-friendly (exit code, FACT pass/fail return).

## Step Completion Expectations

See `implementation-plan.md` for per-step preconditions, postconditions, falsifying checks, files-allowed-to-read/edit, expected sub-agent dispatches, and per-step S/M cost.

## Context Discipline Notes

- **Large files to range-read or delegate**:
  - `docs/02_ir_schemas.md` (> 600 lines) — delegate via SUMMARY.
  - `crates/slicer-ir/src/slice_ir.rs` (~ 1500+ lines) — range-read only at lines `1435-1469` (`ToolChange` at 1435-1442, `TravelRetract` at 1455-1469), `1524-1543` (`LayerCollectionIR.tool_changes` at 1534), `740-760` (`ActiveRegion.tool_index` at 750), and `1233-1262` (`ExtrusionRole` enum). Do not open in full.
  - `crates/slicer-host/src/gcode_emit.rs` (~ 1200 lines) — range-read at `290-410` (toolchange emission entry) and `1140-1170` (the bare `T<n>` writeln). Plus the role-to-`;TYPE:` mapping function (location returned by Step 1 dispatch).
- **Out-of-bounds**: all of `OrcaSlicerDocumented/`, `target/`, `Cargo.lock`, vendored deps, and crates not on the change list (specifically: `slicer-helpers`, `slicer-cli`, all other `modules/core-modules/*` modules).
- **Temptation reads to skip**:
  - The full `crates/slicer-ir/src/slice_ir.rs`. Resist; range-read at the documented line ranges only.
  - Other core-modules' source. The only relevant module is `wipe-tower`.
  - OrcaSlicer source bodies. All Orca reads go through dispatch.
  - `docs/07_implementation_status.md` in full. Use a dispatch to locate just the three TASK-### line ranges for Step 6.
- **Sub-agent return-format hints for the heaviest dispatches**:
  - `ExtrusionRole` enum query: FACT, ≤ 5 lines, list variants only.
  - Wipe-tower module's existing output structure: SNIPPETS, ≤ 3 snippets of ≤ 30 lines each at the `run_finalization` entry.
  - OrcaSlicer parity queries: LOCATIONS + one-line role; no code body.
  - Multi-material fixture choice: FACT — "is there an existing multi-material STL under `crates/slicer-host/tests/fixtures/`? If yes, file name; if no, return 'absent'."
