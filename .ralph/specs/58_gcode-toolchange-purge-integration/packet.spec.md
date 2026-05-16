---
status: draft
packet: 58_gcode-toolchange-purge-integration
task_ids:
  - TASK-143
  - TASK-152b
  - TASK-120d2
backlog_source: docs/07_implementation_status.md
context_cost_estimate: M
copy_note: This packet closes an integration gap between five prior implemented packets (17, 19, 11, 15, 34). It does not supersede any of them. The `task_ids:` listed in this frontmatter were each previously closed by their owning packet (TASK-143 by 17, TASK-152b by 19, TASK-120d2 by 15); this packet reopens them at the integration layer and re-closes them via Step 6's `docs/07_implementation_status.md` update.
---

# Packet Contract: 58_gcode-toolchange-purge-integration

## Goal

Wire the existing `wipe-tower` module's per-layer output through the live G-code emission path so every `T<n>` tool-change token in the final `.gcode` is bracketed by a retract → travel → load/prime → wipe sequence, and so every layer that contains at least one tool change emits a `;TYPE:Wipe tower` (or `;TYPE:Prime tower`) block.

Today, `crates/slicer-host/src/gcode_emit.rs:1155-1156` writes a bare `T<n>` token with no surrounding moves, and the `wipe-tower` module's `PostPass::LayerFinalization` output never surfaces in the serialized G-code for multi-material fixtures. This packet closes the integration without introducing a new config flag — `wipe_tower_enabled` (from packet 17) remains the canonical opt-in.

## Scope Boundaries

- **In scope**:
  - Make `modules/core-modules/wipe-tower/src/lib.rs` inject explicit retract/prime/wipe `PrintEntity` rows into each `LayerCollectionIR` bracketing every `ToolChange` entry, tagged with `ExtrusionRole::WipeTower`.
  - Make `crates/slicer-host/src/gcode_emit.rs` serialize `ExtrusionRole::WipeTower` as `;TYPE:Wipe tower` (matching packet 11's `;TYPE:<RoleName>` emission contract) and revert to the prior role on the next entity.
  - Add a defensive guard in `gcode_emit.rs` so a `ToolChange` not bracketed by a retract entity (negative E delta) and a wipe-tower-role entity is rejected as `PostpassError::MissingToolchangePurge { layer_index, tool_change_index }` when `wipe_tower_enabled=true`. The variant is added additively to `PostpassError` in `crates/slicer-host/src/postpass.rs:39-59`.
  - `ExtrusionRole::WipeTower` is already present at `crates/slicer-ir/src/slice_ir.rs:1233-1262` (confirmed during spec-review). Step 1 reverifies; no enum edit required.
  - Check in one synthetic multi-material STL at `crates/slicer-host/tests/fixtures/multi_color_cube.stl` and one OrcaSlicer reference G-code at `crates/slicer-host/tests/fixtures/multi_color_cube.orca.gcode` for parity comparison.
- **Out of scope**:
  - Any new config key. Reuse `wipe_tower_enabled`, `wipe_tower_purge_volume`, `wipe_tower_x`, `wipe_tower_y`, `wipe_tower_width`, `line_width` exactly as declared in `modules/core-modules/wipe-tower/wipe-tower.toml`.
  - Ramming and cooling-tube load-dynamics modeling from OrcaSlicer (deferred — borrow only the call ordering, not the velocity profile).
  - Tree/grid tower interior infill geometry beyond the existing rectilinear purge paths from packet 17.
  - The 3-release N/N+1/N+2 rollout from `docs/11`. This packet ships as a single-release bugfix completing prior packets' declared scope; a `docs/DEVIATION_LOG.md` entry records the integration completion.
  - Any change to the WIT layer-collection-builder interface — entity insertion uses existing exports.

## Prerequisites and Blockers

- **Depends on**: packets 17 (`17_wipe-tower-finalization-live-path`), 19 (`19_path-optimization-tool-order-and-cooling-policy`), 11 (`11_orca-gcode-emission-contract`), 15 (`15_live-travel-retraction-policy`), 34 (`34_retraction-mode-firmware-vs-gcode`) — all `implemented`.
- **Unblocks**: any downstream multi-material end-to-end correctness packet.
- **Activation blockers**: none. No other packet is currently `active`.

## Acceptance Criteria

- **Given** a `LayerCollectionIR` containing one `ToolChange { from_tool: 0, to_tool: 1, after_entity_index: K }` and `wipe_tower_enabled=true`, **when** `GCodeSerializer` serializes the layer, **then** the produced text contains, in order: at least one entity emitting negative `E` delta (retract), at least one `G1` travel move, the literal line `T1`, at least one entity emitting cumulative positive `E` delta ≥ `wipe_tower_purge_volume` mm (the prime+wipe), and the literal line `;TYPE:Wipe tower` appears before the first of these new entities; the next print-role extrusion appears only after the wipe block ends. | `cargo test -p slicer-host --test gcode_toolchange_wrapping toolchange_emits_retract_prime_wipe -- --nocapture`

- **AC2a — retract precedes `T<n>`**: **Given** a final `.gcode` produced for `crates/slicer-host/tests/fixtures/multi_color_cube.stl` with `wipe_tower_enabled=true`, **when** every line matching `^T[0-9]+` is examined, **then** at least one line containing `E-` (retract) appears in the 5 preceding lines. | `awk '/^T[0-9]/{ok=0; for(i=NR-5;i<NR;i++) if(i>0 && prev[i]~/E-/) ok=1; if(!ok){print "no retract before line "NR": "$0; bad=1}} {prev[NR]=$0} END{exit bad+0}' target/test-output/multi_color_cube.gcode`

- **AC2b — positive-`E` `G1` follows `T<n>` within 10 lines**: **Given** the same final `.gcode`, **when** every line matching `^T[0-9]+` is examined, **then** at least one `G1` move with a positive `E` token (and no `E-`) appears in the 10 following lines before the next print-model extrusion. | `awk '{lines[NR]=$0} END{bad=0; for(i=1;i<=NR;i++) if(lines[i]~/^T[0-9]/){ok=0; for(j=i+1;j<=i+10 && j<=NR;j++) if(lines[j]~/^G1.*E[0-9]/ && lines[j]!~/E-/){ok=1; break} if(!ok){print "no prime after line "i": "lines[i]; bad=1}} exit bad+0}' target/test-output/multi_color_cube.gcode`

- **Given** the multi-material fixture sliced into `target/test-output/multi_color_cube.gcode` and the checked-in OrcaSlicer reference at `crates/slicer-host/tests/fixtures/multi_color_cube.orca.gcode`, **when** per-toolchange purge extrusion volume (sum of positive `E` deltas between `T<n>` and the next non-wipe-tower entity that is not a `G1` travel) is computed for each matched `(from_tool, to_tool)` pair, **then** every matched pair's Slicer B volume is within `[0.80, 1.20]` × the Slicer A volume for the same pair. | `cargo test -p slicer-host --test gcode_toolchange_wrapping purge_volume_within_tolerance -- --nocapture`

- **Given** a `LayerCollectionIR` containing one `ToolChange` entry and the `wipe-tower` module is invoked with `wipe_tower_enabled=true`, **when** the resulting layer is serialized by `GCodeSerializer`, **then** the serialized text contains exactly one occurrence of the literal line `;TYPE:Wipe tower` preceding the first wipe-tower extrusion, and the next non-tower entity emits a non-`Wipe tower` `;TYPE:` line (one of `;TYPE:External perimeter`, `;TYPE:Internal perimeter`, `;TYPE:Internal solid infill`, `;TYPE:Sparse infill`, or the role carried by the resumed print entity). | `cargo test -p wipe-tower --lib emits_wipe_tower_role_marker -- --nocapture`

- **Given** the multi-material fixture with `L` layers containing at least one `T<n>` token, **when** the final `.gcode` is scanned, **then** the count of `;TYPE:Wipe tower` plus `;TYPE:Prime tower` lines is `≥ L`. | `python -c "import re,sys; lines=open('target/test-output/multi_color_cube.gcode').readlines(); tc=sum(1 for l in lines if re.match(r'T[0-9]+',l)); pt=sum(1 for l in lines if 'Wipe tower' in l or 'Prime tower' in l); print('OK' if pt>=tc else f'FAIL pt={pt} tc={tc}'); sys.exit(0 if pt>=tc else 1)"`

- **Given** `wipe_tower_enabled=true` and the wipe-tower module emits a tower polygon for the first layer of the multi-material fixture against module-internal stub `bed_polygon` and stub object footprints (real host-service bed-bounds access is deferred per the Step 6 `docs/DEVIATION_LOG.md` entry), **when** the tower polygon's vertices are checked against the stub `bed_polygon` and the stub object footprint union, **then** every vertex is inside the stub bed polygon AND the tower polygon does not intersect any stub object footprint polygon. | `cargo test -p wipe-tower --lib tower_geometry_within_bed_outside_objects -- --nocapture`

## Negative Test Cases

- **Given** a synthetic `LayerCollectionIR` containing one `ToolChange { from_tool: 0, to_tool: 1, after_entity_index: 0 }` with no preceding retract entity and no following wipe-tower entity, **when** `GCodeEmitter` is invoked with `wipe_tower_enabled=true`, **then** it returns `Err(PostpassError::MissingToolchangePurge { layer_index, tool_change_index })` rather than writing a bare `T1` line. | `cargo test -p slicer-host --test gcode_toolchange_wrapping bare_toolchange_rejected -- --nocapture`

- **Given** any `.gcode` line matching `^T[0-9]+` immediately followed (skipping comment lines and blank lines) by a `G1` move containing a positive `E` token (an extruding move with no prime preamble), **when** the validation script runs, **then** it exits non-zero printing the offending line numbers. | `awk '/^T[0-9]/{getline n; while(n~/^;/||n==""){getline n} if(n ~ /G1.*E[0-9]/ && n !~ /E-/){print "no prime: "$0" then "n; exit 1}}' target/test-output/multi_color_cube.gcode`

- **Given** a multi-tool `.gcode` (`grep -oE '^T[0-9]+' | sort -u | wc -l` > 1) with zero `;TYPE:Wipe tower` and zero `;TYPE:Prime tower` lines, **when** the validation script runs, **then** it exits non-zero. | `python -c "import sys,re; lines=open('target/test-output/multi_color_cube.gcode').readlines(); tools=set(re.match(r'(T[0-9]+)',l).group(1) for l in lines if re.match(r'T[0-9]+',l)); towers=sum(1 for l in lines if 'Wipe tower' in l or 'Prime tower' in l); sys.exit(1 if len(tools)>1 and towers==0 else 0)"`

## Verification

Supplemental packet-level commands (not per-criterion):

- `cargo check --workspace`
- `cargo clippy --workspace -- -D warnings`
- `cargo test -p slicer-host --test gcode_toolchange_wrapping`
- `cargo test -p wipe-tower`
- `./modules/core-modules/build-core-modules.sh`
- `cargo run --bin slicer-cli --release --slice --input crates/slicer-host/tests/fixtures/multi_color_cube.stl --output target/test-output/multi_color_cube.gcode`

(`cargo test --workspace` is NOT listed here. It is invoked exactly once at the acceptance ceremony in `implementation-plan.md`, per the project's Test Discipline rule.)

## Authoritative Docs

- `docs/02_ir_schemas.md` — > 600 lines; **delegate via SUMMARY**. Ask: "what variants does `ExtrusionRole` carry today, and what is the exact shape of `ToolChange` and `PrintEntity`?"
- `docs/03_wit_and_manifest.md` — **range-read** wipe-tower manifest schema and `FinalizationOutputBuilder` exports only.
- `docs/04_host_scheduler.md` — **direct read** of the LayerFinalization → GCodeEmit transition only.
- `docs/08_coordinate_system.md` — **direct read** (file is short; unit math is required for tower geometry).
- `docs/09_progress_events.md` — **direct read**; confirm no existing event contract is being violated (no new event needed for this packet).
- `docs/11_operational_governance_and_acceptance_gate.md` — **range-read §1** only (DEVIATION_LOG entry format and bugfix vs rollout criteria).

## OrcaSlicer Reference Obligations

All reads delegated; never load these into the implementer's context.

- `OrcaSlicerDocumented/src/libslic3r/GCode/WipeTower2.cpp:1557-1640` — toolchange Unload/Change/Load/Wipe call ordering. **Borrow ordering**; ramming detail is parity-deferred.
- `OrcaSlicerDocumented/src/libslic3r/GCode/WipeTower2.cpp:1603,1646` — Orca's `;Wipe_Tower_Start` / `;Wipe_Tower_End` marker pair. **Deliberately not borrowed**: ModularSlicer uses `;TYPE:Wipe tower` per packet 11's emission contract; introducing a parallel marker style fragments that contract.
- `OrcaSlicerDocumented/src/libslic3r/Print.cpp:3180-3268` — per-layer `plan_toolchange()` loop. Reference for understanding when a layer needs a tower visit.
- `OrcaSlicerDocumented/src/libslic3r/GCode.cpp:7624` — `GCode::set_extruder()` retract → filament_end → toolchange flow. Reference for wrap ordering.
- `OrcaSlicerDocumented/src/libslic3r/GCode/WipeTower2.cpp:2258-2270` — `flush_volumes_matrix` purge-volume table consumption. Reference for the ±20% parity AC (consumed values, not the matrix structure).

## Packet Files

- `requirements.md`
- `design.md`
- `implementation-plan.md`
- `task-map.md` (required — packet spans three task IDs and threads through five prior packets)

## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- stop reading at 60% context and hand off at 85%

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. No single step is L.
