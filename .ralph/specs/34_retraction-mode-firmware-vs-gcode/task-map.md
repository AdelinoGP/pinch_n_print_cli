## Cross-Packet Mapping

| Step | Backlog Task IDs | Authoritative Doc | OrcaSlicer Reference | Predecessor Relationship |
|------|------------------|-------------------|----------------------|--------------------------|
| 1 — IR additivity | TASK-120d2 (extension) | `docs/02_ir_schemas.md` | none | Extends packet 15's `GCodeCommand::Retract` / `Unretract` non-destructively |
| 2 — Manifest enum + config read | TASK-120d2 (extension) | `docs/03_wit_and_manifest.md`, `docs/05_module_sdk.md` | `PrintConfig.cpp::use_firmware_retraction` | Extends packet 15's existing `[config.schema.retract_*]` block |
| 3 — Producer propagation | TASK-120d2 (extension) | `docs/02_ir_schemas.md` | none | Extends packet 15's `push_retract` / `push_unretract` call sites |
| 4 — Emitter dispatch | TASK-120d2 (extension) | `docs/02_ir_schemas.md` | `GCodeWriter.cpp::_retract` (firmware vs G-code branch) | Extends packet 15's `gcode_emit.rs` retract/unretract arms |
| 5 — Reframe failing E2E | TASK-120d2, TASK-135 (partial) | none | none | Absorbs packet 21's malformed AC-3 |
| 6 — Firmware-mode E2E | TASK-120d2 (new), TASK-135 (partial) | none | `GCodeWriter.cpp::_retract` (G10/G11 output parity) | New acceptance evidence |
| 7 — Flip packet 21 | TASK-120d2 (housekeeping) | none | none | Marks packet 21 `superseded`; no edits to packet 21 body |
| 8 — Packet completion gate | TASK-120d2, TASK-135 (partial) | `docs/11_operational_governance_and_acceptance_gate.md` | none | Workspace acceptance ceremony |

## Backlog Delta Summary

After packet 34 lands, `docs/07_implementation_status.md` should reflect the following:

- **TASK-120d2** — remains `[x]`. The underlying retract/unretract emission was already implemented in packet 15; packet 34 extends it with firmware-mode parity and re-aligns the regression assertion. The capability flag should not regress to `[ ]`.
- **TASK-135** — remains `[ ]` until all four Benchy regression-assertion families (supports, top/bottom fills, seams, retract/unretract pairs) are green together. Packet 34 closes only the retract/unretract family.
- **Packet 21 (`benchy-acceptance-evidence`)** — flipped to `superseded` in Step 7. The retract/unretract acceptance evidence it carried is now in packet 34's AC-1 + AC-2.
- **Packet 15 (`live-travel-retraction-policy`)** — unchanged; its G-code-mode emission is still the default behavior. No status flip.
- **No new TASK-### proposed.** Adding the `retract_mode` toggle is small enough to live under TASK-120d2's extension umbrella; backlog churn is not warranted.

## Predecessor Relationships at a Glance

```
                                         ┌─────────────────────────────────┐
                                         │ Packet 15 (implemented)         │
                                         │ live-travel-retraction-policy   │
                                         │ — emits G1 E- / G1 E+           │
                                         │ — STILL THE DEFAULT             │  ← extended, not replaced
                                         └────────────────┬────────────────┘
                                                          │
                                                          ▼
┌─────────────────────────────────┐        ┌─────────────────────────────────┐
│ Packet 21 (was draft)           │ ───►   │ Packet 34 (this packet, draft)  │
│ benchy-acceptance-evidence      │ super  │ retraction-mode-firmware-vs-gco │
│ — AC-3 asserted M207/M208 (bug) │ seded  │ — adds retract_mode toggle      │
│                                 │        │ — reframes AC-3 against G1 E-   │
│                                 │        │ — adds G10/G11 firmware test    │
└─────────────────────────────────┘        └─────────────────────────────────┘
```
