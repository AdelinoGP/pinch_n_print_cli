# Implementation Plan: 307-prime-tower-body-parity

## Execution Rules

- Work one atomic step at a time; map every step to grouped task IDs.
- Use TDD, then implementation, then the narrowest falsifying validation.
- Every field below is a context-budget contract and must be filled independently; never write "see Step 1".

## Steps

### Step 0: Reconcile FORWARD-DEPs (read-only discovery)

- Task IDs: `— (queue packet, task_ids: [])`
- Objective: prove the drafted helper names/shapes this packet consumes still match their producers' plans, or reconcile before any body code.
- Precondition: packet files authored; no tree edits yet.
- Postcondition: a reconciliation table (helper → producer → verified name/shape, or reconciled edits) recorded in the step's working notes; Steps 2–3 proceed against verified names only.
- Files allowed to read, with ranges when over 300 lines:
  - `docs/spec_packets/254a-prime-tower-geometry-keys/packet.spec.md` - AC-1–AC-6
  - `docs/spec_packets/254b-prime-tower-interface-and-ramming/packet.spec.md` - AC-1–AC-3
  - `docs/spec_packets/255-wipe-tower-geometry-keys/packet.spec.md` - AC-1 + ordering section
- Files allowed to edit (at most 3):
  - (none — read-only step)
- Files explicitly out of bounds:
  - `OrcaSlicerDocumented/...`, `target/`, all `crates/` + `modules/` sources
- Expected sub-agent dispatches:
  - Question: extract drafted helper identifiers (depth fn, pitch expr, brim fn, wall fns, schema-guard filename, `toml` dev-dep state); scope: `docs/spec_packets/254a* docs/spec_packets/254b* docs/spec_packets/255*`; return: `LOCATIONS`
- Context cost: `S`
- Authoritative docs:
  - `docs/04_host_scheduler.md` - delegated SUMMARY (finalization merge semantics)
- OrcaSlicer refs: (none this step)
- Verification:
  - Reconciliation table complete, every Step 2–3 consumed name traced to a producer AC — working-notes check, no cargo
- Exit condition: every consumed helper name resolves to a producer-planned symbol, or the producer spec is amended first (never worked around silently).

### Step 1: Manifest declarations + schema guard + doc regen

- Task IDs: `— (queue packet, task_ids: [])`
- Objective: declare the five tower tables + two emitter reader rows with AC-1 exact specs; extend (or, contingency, author) the schema guard; regen docs.
- Precondition: Step 0 names verified; 254a's landing state known (landed → extend its guard; not landed → author guard + add `toml = "0.8"` to wipe-tower dev-deps).
- Postcondition: manifests parse with the seven tables; guard fails on any drift (AC-1, AC-N3); `gen-config-docs --check` green (AC-13).
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/wipe-tower/wipe-tower.toml` - full (∼120 lines)
  - `modules/core-modules/machine-gcode-emit/machine-gcode-emit.toml` - `[config.schema]` ranges only
- Files allowed to edit (at most 3):
  - `modules/core-modules/wipe-tower/wipe-tower.toml`
  - `modules/core-modules/machine-gcode-emit/machine-gcode-emit.toml`
  - `modules/core-modules/wipe-tower/tests/wipe_tower_config_schema_tdd.rs`
- Files explicitly out of bounds:
  - `crates/slicer-gcode/src/serialize.rs`, all producer packet dirs, `OrcaSlicerDocumented/...`
- Blast-radius discipline (mandatory when adding a new struct field or schema constant):
  - No struct field or version constant is added this step (manifest tables only) — no blast radius.
- Expected sub-agent dispatches:
  - Question: does `wipe_tower_config_schema_tdd.rs` exist and does wipe-tower dev-deps carry `toml`?; scope: `modules/core-modules/wipe-tower/`; return: `FACT`
- Context cost: `S`
- Authoritative docs:
  - `docs/03_wit_and_manifest.md` - `[config.schema]` declaration shape ranges
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` - delegate; never load (five declaration facts, Step 0-verified)
- Verification:
  - `cargo test -p wipe-tower --test wipe_tower_config_schema_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"` - FACT pass/fail or bounded failure SNIPPETS
  - `cargo xtask gen-config-docs --check; echo "exit=$?"` - FACT pass/fail
- Exit condition: AC-1 + AC-N3 + AC-13 green; no other test touched.

### Step 2: Depth planning + idle entries + no-sparse gate

- Task IDs: `— (queue packet, task_ids: [])`
- Objective: build `plan_tower_depths` (per-layer purge depth → backward max-propagation → tower max → idle-entry set) with the `wipe_tower_no_sparse_layers` gate; TDD in the new body binary.
- Precondition: Step 1 green; 254a depth-model names verified (FORWARD-DEP).
- Postcondition: AC-3 + AC-4 green; planner is a pure fn of (layers, config) with no I/O.
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/wipe-tower/src/lib.rs` - lines `345-410` (`purge_depth_for`, `max_purge_depth`, `generate_purge_paths` signature)
  - `modules/core-modules/wipe-tower/src/lib.rs` - lines `660-730` (`run_finalization` head + skip guard)
- Files allowed to edit (at most 3):
  - `modules/core-modules/wipe-tower/src/lib.rs`
  - `modules/core-modules/wipe-tower/tests/wipe_tower_body_tdd.rs`
- Files explicitly out of bounds:
  - `modules/core-modules/machine-gcode-emit/**`, producer packet dirs, `OrcaSlicerDocumented/...`
- Expected sub-agent dispatches:
  - Question: canonical propagation band + max formula arms; scope: `OrcaSlicerDocumented/src/libslic3r/GCode/WipeTower2.cpp`; return: `LOCATIONS`
- Context cost: `M`
- Authoritative docs:
  - `docs/08_coordinate_system.md` - delegated SUMMARY (depth/width mm↔unit boundaries)
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/GCode/WipeTower2.cpp` - delegate; never load (`plan_tower` backward pass)
- Verification:
  - `cargo test -p wipe-tower --test wipe_tower_body_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"` - FACT pass/fail or bounded failure SNIPPETS
- Exit condition: AC-3 + AC-4 green; AC-2 still green (planner inert when disabled).

### Step 3: finish_layer assembly (perimeter → infill → wall → brim)

- Task IDs: `— (queue packet, task_ids: [])`
- Objective: build `emit_finish_layer` in canonical order, calling 254a's brim builder (first layer only) and 255's wall helper, with bridging spacing + first-layer-solid infill; body via anchor-free push.
- Precondition: Step 2 green; 254a brim + 255 wall names verified (FORWARD-DEP).
- Postcondition: AC-5 + AC-6 + AC-7 green; purge anchors untouched.
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/wipe-tower/src/lib.rs` - lines `409-560` (`generate_purge_paths` body idiom: travel/scan-line/prime entity shapes)
  - `modules/core-modules/wipe-tower/tests/finalization_live_tdd.rs` - lines `1-60` (push/insert coexistence idiom)
- Files allowed to edit (at most 3):
  - `modules/core-modules/wipe-tower/src/lib.rs`
  - `modules/core-modules/wipe-tower/tests/wipe_tower_body_tdd.rs`
- Files explicitly out of bounds:
  - `modules/core-modules/machine-gcode-emit/**`, `crates/slicer-gcode/src/emit.rs`, producer packet dirs, `OrcaSlicerDocumented/...`
- Expected sub-agent dispatches:
  - Question: `push_entity_with_priority` vs `insert_entity_at` merge channels/priority semantics; scope: `crates/slicer-sdk/src/traits.rs`; return: `FACT`
  - Question: canonical `finish_layer` arm order + solid rule; scope: `OrcaSlicerDocumented/src/libslic3r/GCode/WipeTower2.cpp`; return: `LOCATIONS`
- Context cost: `M`
- Authoritative docs:
  - `docs/08_coordinate_system.md` - delegated SUMMARY (loop offsets, spacing formula units)
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/GCode/WipeTower2.cpp` - delegate; never load (`finish_layer`)
- Verification:
  - `cargo test -p wipe-tower --test wipe_tower_body_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"` - FACT pass/fail or bounded failure SNIPPETS
  - `cargo test -p wipe-tower --test wipe_tower_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"` - FACT pass/fail (purge coexistence)
- Exit condition: AC-5 + AC-6 + AC-7 green; AC-2 still green.

### Step 4: Forced-filament tool selection + range fatal

- Task IDs: `— (queue packet, task_ids: [])`
- Objective: wire `wipe_tower_filament` (boundary tool identity + masking-semantics short-circuit) with fatal out-of-range validation.
- Precondition: Step 3 green (body entities exist to stamp).
- Postcondition: AC-8 + AC-N2 green; auto(0) path byte-identical to Step 3 output.
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/wipe-tower/src/lib.rs` - lines `730-775` (`run_finalization` tail: tool stamping site)
- Files allowed to edit (at most 3):
  - `modules/core-modules/wipe-tower/src/lib.rs`
  - `modules/core-modules/wipe-tower/tests/wipe_tower_body_tdd.rs`
- Files explicitly out of bounds:
  - `modules/core-modules/machine-gcode-emit/**`, `crates/slicer-gcode/src/emit.rs`, `OrcaSlicerDocumented/...`
- Expected sub-agent dispatches:
  - Question: canonical forced-branch + masking + validate arms; scope: `OrcaSlicerDocumented/src/libslic3r/GCode/ToolOrdering.cpp` + `Print.cpp`; return: `LOCATIONS`
- Context cost: `S`
- Authoritative docs:
  - `docs/03_wit_and_manifest.md` - module-error shape ranges (fatal naming the key)
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/GCode/ToolOrdering.cpp` - delegate; never load (`insert_wipe_tower_extruder`)
- Verification:
  - `cargo test -p wipe-tower --test wipe_tower_body_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"` - FACT pass/fail or bounded failure SNIPPETS
- Exit condition: AC-8 + AC-N2 green; AC-2 still green.

### Step 5: Smooth mode (forced tower + equalise + wall-only) and suppression condition

- Task IDs: `— (queue packet, task_ids: [])`
- Objective: smooth planning in wipe-tower (force tower, every-layer entries, floor + equalise, outer-wall-only) AND the suppression conjunction in machine-gcode-emit; both readers gain the canonical `"0"`/`"1"` ingest arms (AC-14 tower half, AC-10 emitter half); the emitter test arm lands in Step 8 with the other cross-crate arms.
- Precondition: Steps 3–4 green.
- Postcondition: AC-9 + AC-14 + AC-N4 green; suppression condition in place with the pre-existing emitter suite still green; AC-10 pends its Step 8 arm.
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/machine-gcode-emit/src/lib.rs` - lines `200-310` (`run_gcode_postprocess` + DEV-168 gate)
- Files allowed to edit (at most 3):
  - `modules/core-modules/wipe-tower/src/lib.rs`
  - `modules/core-modules/machine-gcode-emit/src/lib.rs`
  - `modules/core-modules/wipe-tower/tests/wipe_tower_body_tdd.rs`
- Files explicitly out of bounds:
  - Producer packet dirs, `crates/slicer-gcode/src/serialize.rs`, `OrcaSlicerDocumented/...`
- Expected sub-agent dispatches:
  - Question: canonical smooth arms (force-tower, every-layer partitions, floor/equalise, wall-only, suppression clause); scope: `OrcaSlicerDocumented/src/libslic3r/Print.cpp` + `GCode.cpp` + `GCode/WipeTower.cpp`; return: `LOCATIONS`
- Context cost: `M`
- Authoritative docs:
  - `docs/00_project_overview.md` - delegated SUMMARY (why suppression lives in the emitting module, not the host)
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/GCode.cpp` - delegate; never load (`process_layer` suppression)
  - `OrcaSlicerDocumented/src/libslic3r/Print.cpp` - delegate; never load (`has_wipe_tower`, `enable_timelapse_print`)
- Verification:
  - `cargo test -p wipe-tower --test wipe_tower_body_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"` - FACT pass/fail or bounded failure SNIPPETS
  - `cargo test -p machine-gcode-emit --test machine_gcode_emit_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"` - FACT pass/fail (pre-existing suite still green under the new conjunction)
- Exit condition: AC-9 + AC-14 + AC-N4 green; suppression condition in place; AC-10 pends Step 8.

### Step 6: skip_points gap-wall gate

- Task IDs: `— (queue packet, task_ids: [])`
- Objective: wire `prime_tower_skip_points` as the tower-side wall gap corridor (open vs closed), default-true pinned.
- Precondition: Step 3 green (wall emission exists to gate).
- Postcondition: AC-11 green; emitter side explicitly untouched (DEV-203).
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/wipe-tower/src/lib.rs` - wall-emission range only (locate via Step 3 working notes)
- Files allowed to edit (at most 3):
  - `modules/core-modules/wipe-tower/src/lib.rs`
  - `modules/core-modules/wipe-tower/tests/wipe_tower_body_tdd.rs`
- Files explicitly out of bounds:
  - `crates/slicer-gcode/src/emit.rs` (travel-avoid routing is DEV-203, not this step), producer packet dirs, `OrcaSlicerDocumented/...`
- Expected sub-agent dispatches:
  - Question: canonical `use_gap_wall` gate shape; scope: `OrcaSlicerDocumented/src/libslic3r/GCode/WipeTower2.cpp`; return: `LOCATIONS`
- Context cost: `S`
- Authoritative docs:
  - `docs/08_coordinate_system.md` - delegated SUMMARY (gap corridor geometry units)
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/GCode/WipeTower2.cpp` - delegate; never load (`use_gap_wall`)
- Verification:
  - `cargo test -p wipe-tower --test wipe_tower_body_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"` - FACT pass/fail or bounded failure SNIPPETS
- Exit condition: AC-11 green; AC-2 still green.

### Step 7: Bed-bounds at planned max + coexistence + identity re-verify

- Task IDs: `— (queue packet, task_ids: [])`
- Objective: move the bed check to the planned max depth (post-plan, same fatal shape); prove purge/interface/body coexistence and full default identity.
- Precondition: Steps 2–6 green.
- Postcondition: bed overrun at planned depth rejects; all pre-existing wipe-tower binaries green unmodified.
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/wipe-tower/tests/bed_bounds_tdd.rs` - full (∼60 lines)
- Files allowed to edit (at most 3):
  - `modules/core-modules/wipe-tower/src/lib.rs`
  - `modules/core-modules/wipe-tower/tests/bed_bounds_tdd.rs`
  - `modules/core-modules/wipe-tower/tests/wipe_tower_tdd.rs`
- Files explicitly out of bounds:
  - Producer packet dirs, `modules/core-modules/machine-gcode-emit/**`, `OrcaSlicerDocumented/...`
- Expected sub-agent dispatches: none
- Context cost: `S`
- Authoritative docs: (none new)
- OrcaSlicer refs: (none this step)
- Verification:
  - `cargo test -p wipe-tower --test bed_bounds_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"` - FACT pass/fail or bounded failure SNIPPETS
  - `cargo test -p wipe-tower --test wipe_tower_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"` - FACT pass/fail
  - `cargo test -p wipe-tower --test finalization_live_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"` - FACT pass/fail (untouched coexistence witness)
- Exit condition: bed-check arms + AC-2 green with zero modifications to `finalization_live_tdd.rs`.

### Step 8: Cross-crate test arms (bounds + binding + suppression)

- Task IDs: `— (queue packet, task_ids: [])`
- Objective: land the three arms that live outside the tower module — scheduler bounds (AC-12), runtime binding-hiding (AC-N1), emitter suppression behaviour (AC-10) — each in its pre-existing, already-registered file.
- Precondition: Steps 1–7 green (all behaviour complete; this step only pins it from neighbouring binaries).
- Postcondition: AC-10 + AC-12 + AC-N1 green; no production file touched.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-scheduler/tests/integration/config_bounds_enforcement_tdd.rs` - wipe-tower arm section only
  - `crates/slicer-runtime/tests/contract/config_view_binding_tdd.rs` - hiding-arm section only
  - `modules/core-modules/machine-gcode-emit/tests/machine_gcode_emit_tdd.rs` - time-lapse gate section only (locate via the `printer_structure` gate comment)
- Files allowed to edit (at most 3):
  - `crates/slicer-scheduler/tests/integration/config_bounds_enforcement_tdd.rs`
  - `crates/slicer-runtime/tests/contract/config_view_binding_tdd.rs`
  - `modules/core-modules/machine-gcode-emit/tests/machine_gcode_emit_tdd.rs`
- Files explicitly out of bounds:
  - All production sources, producer packet dirs, `crates/slicer-gcode/src/serialize.rs`, `OrcaSlicerDocumented/...`
- Expected sub-agent dispatches: none (all shapes verified in prior steps)
- Context cost: `S`
- Authoritative docs: (none new)
- OrcaSlicer refs: (none this step)
- Verification:
  - `cargo test -p slicer-scheduler --test integration config_bounds_enforcement_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"` - FACT pass/fail or bounded failure SNIPPETS
  - `cargo test -p slicer-runtime --test contract config_view_binding_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"` - FACT pass/fail or bounded failure SNIPPETS
  - `cargo test -p machine-gcode-emit --test machine_gcode_emit_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"` - FACT pass/fail or bounded failure SNIPPETS
- Exit condition: AC-10 + AC-12 + AC-N1 green; zero production edits in this step.

### Step 9: DEV rows + doc regen + closure gates

- Task IDs: `— (queue packet, task_ids: [])`
- Objective: file the three deviation rows, regen docs, and run the whole-tree gates.
- Precondition: Steps 1–8 green.
- Postcondition: AC-13 green; DEV-201/202/203 in the log; check/clippy/guests green.
- Files allowed to read, with ranges when over 300 lines: (none — row format mirrored from the adjacent DEV-200 row)
- Files allowed to edit (at most 3):
  - `docs/DEVIATION_LOG.md`
- Files explicitly out of bounds:
  - All production and test sources, producer packet dirs, `crates/slicer-gcode/src/serialize.rs`, `OrcaSlicerDocumented/...`
- Expected sub-agent dispatches: none (all shapes verified in prior steps)
- Context cost: `S`
- Authoritative docs:
  - `docs/DEVIATION_LOG.md` - row format (mirror the DEV-200 row shape)
- OrcaSlicer refs: (none this step)
- Verification:
  - `cargo xtask gen-config-docs --check; echo "exit=$?"` - FACT pass/fail
  - `cargo check --workspace --all-targets; echo "exit=$?"` - FACT pass/fail
  - `cargo clippy --workspace --all-targets -- -D warnings; echo "exit=$?"` - FACT pass/fail
  - `cargo xtask build-guests --check; echo "exit=$?"` - FACT pass/fail (exit 3 is infra, not clean)
- Exit condition: AC-13 green, all gates exit 0, DEV-201/202/203 greppable in the log.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 0 | S | Read-only reconciliation; no cargo |
| Step 1 | S | Manifests + guard + docs regen |
| Step 2 | M | Planning kernel + idle set |
| Step 3 | M | finish_layer assembly (largest code) |
| Step 4 | S | Tool selection + fatal |
| Step 5 | M | Smooth pair across two modules (test arms split to respect edit cap) |
| Step 6 | S | Gap-wall gate |
| Step 7 | S | Bed-bounds + coexistence |
| Step 8 | S | Three cross-crate arms, no production edits |
| Step 9 | S | DEV rows + doc regen + gates |

Split before activation if aggregate cost exceeds M or any step is L.

## Packet Completion Gate

- All steps and exits complete.
- Every pipe-suffixed AC command returns PASS.
- Update `docs/07_implementation_status.md` through a worker dispatch, never a full backlog read.
- Reconcile reopened/superseded status transitions.
- `packet.spec.md` is ready for `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed AC and packet-level gate command.
- Record remaining packet-local risk.
- Confirm context stayed at or below 150k standard, or at/below 300k only with a logged swarm ESCALATION; otherwise record a packet-authoring lesson.

All `cargo check`, `cargo clippy`, and `cargo test` invocations in gate and verification commands must use `--all-targets` so the test, bench, and example targets compile.
