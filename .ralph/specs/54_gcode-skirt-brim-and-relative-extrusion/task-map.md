# Task Map: 54_gcode-skirt-brim-and-relative-extrusion

Bridges this packet's steps back to `docs/07_implementation_status.md`, to DEV-009 in `docs/DEVIATION_LOG.md`, and to TASK-142 (predecessor for Track A).

This packet introduces two new task IDs across two independent tracks:

- **TASK-142a** (new — Track A) — "Live SkirtBrim emit gap follow-up: diagnose & fix why TASK-142's port produces zero `;TYPE:Skirt|;TYPE:Brim` blocks". Predecessor: TASK-142 (Closed 2026-04-25; NOT reopened).
- **TASK-155** (new — Track B) — "Relative-extrusion mode toggle (M82/M83) via `use_relative_e_distances`".

| docs/07 task ID | Packet step | Primary docs | Expected code surface | OrcaSlicer refs | Context cost | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| `TASK-142a` | Step 1 — Track A diagnosis | none direct | none (pure-dispatch) | `OrcaSlicerDocumented/src/libslic3r/Brim.cpp` (SUMMARY), `OrcaSlicerDocumented/src/libslic3r/Print.cpp` (SUMMARY) | S | Returns SUMMARY ≤ 100 words with ONE cause + ONE fix, OR `ESCALATE` to hand off to packet 54a. |
| `TASK-142a` | Step 2A — Track A fix + tests | `docs/05_module_sdk.md` (Finalization Stage, ≤ 40 lines) | ONE of {`modules/core-modules/skirt-brim/src/lib.rs`, `crates/slicer-host/src/dispatch.rs:2840-:2900`, `crates/slicer-host/src/config_schema.rs`, `modules/core-modules/skirt-brim/skirt-brim.toml`} + `crates/slicer-host/tests/gcode_skirt_brim_emission_tdd.rs` (new) | Brim/Print SUMMARY from Step 1 | M | Skipped if Step 1 returned ESCALATE. All Track A ACs + negative test green. |
| `TASK-155` | Step 2B — Track B schema + first test | none direct | `crates/slicer-host/src/config_schema.rs`, `crates/slicer-host/tests/gcode_relative_extrusion_tdd.rs` (new) | none | S | `config_schema_registers_bool_default_true` green; rest red. |
| `TASK-155` | Step 3B — Serializer branch | `docs/02_ir_schemas.md` (SUMMARY) | `crates/slicer-host/src/gcode_emit.rs` (`:200-:480` range), `crates/slicer-host/tests/gcode_relative_extrusion_tdd.rs` | `OrcaSlicerDocumented/src/libslic3r/GCodeWriter.cpp` (FACT ≤ 8 lines on M82/M83) | M | 5 of 6 ACs + 4 of 4 negative tests green; pipeline-threading test still red. |
| `TASK-155` | Step 4B — Pipeline threading | none | `crates/slicer-host/src/pipeline.rs:200-:280` | none | S | All Track B tests green. |
| `TASK-142a`, `TASK-155` | Step 5 — Docs hygiene | `docs/07_implementation_status.md`, `docs/DEVIATION_LOG.md`, `docs/14_deviation_audit_history.md` | docs only | none | S | TASK-142a + TASK-155 rows; DEV-009 progress entries for both subsets. |

Aggregate context cost: M (no row is L).

## Why this packet is sufficient evidence

For **TASK-142a** (Track A):
- Step 1 produces a diagnosis SUMMARY recorded in `design.md` — that is the audit trail for the predecessor packet's silent gap.
- Step 2A's 4 ACs + 1 negative case prove: skirt block present when enabled; absent when disabled; loop count honored; brim block present when `brim_width > 0`; negative test fails when configuration demands a skirt but none is emitted.
- TASK-142 is NOT reopened; TASK-142a is a follow-up row that cites TASK-142 as predecessor.

For **TASK-155** (Track B):
- 6 ACs + 4 negative cases prove: default is relative (M83); explicit `Bool(false)` is absolute (M82) with byte-identical E text vs IR; relative-mode E values are deltas; X/Y/Z/F identical across modes; delta-sum identity across each `G92 E0` block; config registration is correct typed/defaulted; rejection for non-bool config; rejection for M82 in relative; rejection for M83/deltas in absolute; rejection for monotonic E run in relative mode.
- IR contract preserved (no IR changes).

## Relationship to DEV-009, TASK-142, and the prior packets

- DEV-009 has three remediation subsets being closed by packets 52 (feedrate), 53 (cooling), and 54 (this — skirt-brim + relative-E).
- TASK-142 (closed 2026-04-25) ported live SkirtBrim geometry. The emission gap discovered in this packet's Step 1 is a follow-up, not a defect of TASK-142's stated scope. TASK-142a closes the follow-up; TASK-142's closure record stands.
- If Step 1 returns ESCALATE, Track A is hand-off to a NEW packet 54a; this packet's slug + task_ids are reduced to Track B only. The hand-off is visible in the implementer's Step 1 report.

## Cross-Packet Mutation Note

Per the spec-packet-generator skill's Cross-Packet Mutation Rule:

- This packet does NOT modify any file inside `.ralph/specs/16_skirt-brim-finalization-live-path/` (the closed predecessor packet for TASK-142). That packet's directory remains read-only.
- This packet does NOT modify `.ralph/specs/19_path-optimization-tool-order-and-cooling-policy/` (the packet that introduced TASK-152c). The cooling supersession is owned by packet 53, not by packet 54.
