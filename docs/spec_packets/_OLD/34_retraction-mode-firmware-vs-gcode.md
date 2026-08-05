---
status: implemented
packet: retraction-mode-firmware-vs-gcode
task_ids:
  - TASK-120d2
  - TASK-135
supersedes:
  - 21_benchy-acceptance-evidence
---

# 34_retraction-mode-firmware-vs-gcode

## Goal

Add an OrcaSlicer-style retraction-mode toggle (`retract_mode`: `gcode` | `firmware`, default `gcode`) so the slicer can either emit travel retracts as inline E-axis moves (`G1 E-<len> F<speed>`) or as firmware retract/unretract opcodes (`G10` / `G11`). Reframe the currently failing E2E acceptance assertion to validate the default G-code path correctly, and add new tests that prove the firmware path emits `G10` / `G11` when the toggle is flipped. Match OrcaSlicer parity: M207/M208 are firmware *configuration* setters and are NOT emitted by this slicer; the firmware mode produces exactly `G10` (retract) and `G11` (unretract).

## Problem Statement

The end-to-end test `benchy_gcode_contains_balanced_retract_and_unretract_pairs` (`crates/slicer-host/tests/benchy_end_to_end_tdd.rs:1208`) asserts that the live Benchy run produces `M207` retract commands and `M208` unretract commands in equal numbers. The assertion fails because the production emitter at `crates/slicer-host/src/gcode_emit.rs:410-426` writes inline E-axis moves (`G1 E-<len> F<speed>` for retract, `G1 E<len> F<speed>` for unretract) and never emits `M207` or `M208`.

Two compounded errors caused the failure:

1. **Conceptual error in the test (packet 21, AC-3).** `M207` and `M208` are Marlin/RepRap *firmware-retraction configuration* commands (set retract length, set unretract length). They are NOT the retract action. OrcaSlicer's firmware-retraction mode emits `G10` (retract) and `G11` (unretract); it leaves `M207`/`M208` configuration to the printer's start G-code. The test was authored as if a single command family (`M207`/`M208`) covered both meanings.

2. **Missing capability.** The slicer has only one retraction emission mode (G-code, inline E moves). OrcaSlicer ships a `use_firmware_retraction` toggle. We have no equivalent. Even after the test is reframed against `G1 E-` patterns, the slicer cannot satisfy a firmware-retraction expectation if a profile asks for it.

This packet adds the missing toggle, defaults to G-code mode (preserving packet 15's shipped behavior bit-for-bit), reframes the existing assertion against the actual G-code-mode artifact format, and adds new tests that prove the firmware branch emits balanced `G10`/`G11`.

## Architecture Constraints

- **IR additivity (`docs/02_ir_schemas.md`).** Adding a field to an existing `GCodeCommand` variant is a breaking change for any matcher that destructures the variant exhaustively. The slice-IR is internal to this workspace, so the change is workspace-wide but not externally observable. Every match arm on `GCodeCommand::Retract` / `GCodeCommand::Unretract` in the workspace must be updated; sub-agent dispatch in Step 1 enumerates them before editing.
- **Manifest schema (`docs/03_wit_and_manifest.md`).** Enum config fields require `type = "enum"`, a `values` array, a `default` whose value is in `values`, and a `display` string. Validation runs at module-load time at the host boundary; an unknown value MUST be rejected with a diagnostic that names the field and the offending value.
- **Module SDK lifecycle (`docs/05_module_sdk.md`).** Config reads happen exactly once in `on_print_start`; the module stores the resolved `RetractMode` on its struct (e.g., `self.retract_mode: RetractMode`) and never re-reads per-layer or per-region.
- **Coordinate system (`docs/08_coordinate_system.md`).** `length` and `speed` are mm and mm/min respectively at the IR boundary; emitter uses `format_coord`. Mode is orthogonal to units; no unit work in this packet.
- **OrcaSlicer parity.** Firmware mode emits `G10` (retract) and `G11` (unretract). It does NOT emit `M207`/`M208`. This is a deliberate parity decision; do not "fix" the test by emitting `M207`/`M208`.

## Data and Contract Notes

- `RetractMode` lives in `slice_ir.rs` next to `GCodeCommand`. It is `pub` and re-exported through `slice_ir`'s public surface so producer and consumer crates resolve the same type.
- `GCodeCommand::Retract { length: f64, speed: f64, mode: RetractMode }` and `GCodeCommand::Unretract { length: f64, speed: f64, mode: RetractMode }`. Field order is `length, speed, mode` so existing positional pattern matches that include `..` continue to compile after the field is added; explicit destructures must be updated.
- The default for `retract_mode` is `"gcode"` (matches packet 15's existing emission); when the module config does not set the key, the read returns `RetractMode::Gcode`.
- `format_coord` continues to format both length and speed; firmware mode does NOT use those numbers (G10/G11 are parameterless), so the resolved length/speed are still carried in the IR for diagnostics and for parity with G-code mode but are not serialized in firmware mode.
- The emitter's firmware-mode lines are exactly `G10\n` and `G11\n` — no trailing spaces, no comments, no parameters. This matches OrcaSlicer's behavior and keeps assertions tight.

## Locked Assumptions and Invariants

- Packet 15's G-code-mode behavior is preserved bit-for-bit when `retract_mode = "gcode"` (the default). Any change to the formatted output of `G1 E-{length} F{speed}` is a regression and out of scope.
- `path-optimization-default` is the only producer of `GCodeCommand::Retract` / `Unretract` in the workspace today. If Step 1's discovery surfaces a second producer, it is added to the file list of the relevant step rather than ignored.
- Retract balance (count equality) is invariant across modes: the producer pushes pairs; the emitter does not drop or insert retracts.
- M207/M208 are intentionally NOT emitted in either mode. Users wanting M207/M208 configure them in their printer's start G-code (OrcaSlicer parity).
- `retract_mode` is a print-level setting. Even though the field is per-command in the IR, every command in a single print run carries the same value.

## Risks and Tradeoffs

- **Risk:** exhaustive match arms on `GCodeCommand::Retract` / `Unretract` elsewhere in the workspace fail to compile after the field is added. **Mitigation:** Step 1 begins with a sub-agent grep that returns LOCATIONS of every match arm; the same step updates them all.
- **Risk:** the new SDK shim signature (`push_retract(length, speed, mode)`) breaks any existing producer outside `path-optimization-default`. **Mitigation:** the same Step 1 grep enumerates `push_retract` / `push_unretract` call sites; if any exist outside `path-optimization-default`, they receive `RetractMode::Gcode` to preserve the default.
- **Risk:** Reframing the failing E2E test to `G1 E-` patterns could be over- or under-permissive (e.g., catching a `G1 E-0.0001` purge as a retract). **Mitigation:** match the exact emit format `G1 E-{nonzero} F{positive}` with a regex that requires the negative sign and non-zero magnitude, plus a separate check that retract/unretract counts are non-zero AND equal. The emit code at `gcode_emit.rs:410-426` is the only producer of these strings, so format drift is detectable by the emitter unit test (AC-5).
- **Tradeoff:** carrying `mode` per command costs one byte per retract command in the IR; negligible at workspace scale.
- **Tradeoff:** firmware-mode emission ignores the carried `length` and `speed` (G10/G11 are parameterless). Some users may expect these values to influence firmware behavior; this matches OrcaSlicer and is documented in the manifest's `display` string.
