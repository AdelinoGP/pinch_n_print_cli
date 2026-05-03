## Packet Metadata

- Slug: `34_retraction-mode-firmware-vs-gcode`
- Status: `draft`
- Task IDs: `TASK-120d2` (extension — see Cross-Packet Impact), `TASK-135` (Benchy regression assertion correction).
- Supersedes: `21_benchy-acceptance-evidence` (it carried the malformed acceptance assertion as AC-3).
- Related (NOT superseded): `15_live-travel-retraction-policy` — its G-code-mode emission stands; this packet extends it with a firmware-mode branch and a config toggle.

## Problem Statement

The end-to-end test `benchy_gcode_contains_balanced_retract_and_unretract_pairs` (`crates/slicer-host/tests/benchy_end_to_end_tdd.rs:1208`) asserts that the live Benchy run produces `M207` retract commands and `M208` unretract commands in equal numbers. The assertion fails because the production emitter at `crates/slicer-host/src/gcode_emit.rs:410-426` writes inline E-axis moves (`G1 E-<len> F<speed>` for retract, `G1 E<len> F<speed>` for unretract) and never emits `M207` or `M208`.

Two compounded errors caused the failure:

1. **Conceptual error in the test (packet 21, AC-3).** `M207` and `M208` are Marlin/RepRap *firmware-retraction configuration* commands (set retract length, set unretract length). They are NOT the retract action. OrcaSlicer's firmware-retraction mode emits `G10` (retract) and `G11` (unretract); it leaves `M207`/`M208` configuration to the printer's start G-code. The test was authored as if a single command family (`M207`/`M208`) covered both meanings.

2. **Missing capability.** The slicer has only one retraction emission mode (G-code, inline E moves). OrcaSlicer ships a `use_firmware_retraction` toggle. We have no equivalent. Even after the test is reframed against `G1 E-` patterns, the slicer cannot satisfy a firmware-retraction expectation if a profile asks for it.

This packet adds the missing toggle, defaults to G-code mode (preserving packet 15's shipped behavior bit-for-bit), reframes the existing assertion against the actual G-code-mode artifact format, and adds new tests that prove the firmware branch emits balanced `G10`/`G11`.

## In Scope

- Add `RetractMode` enum (`Gcode`, `Firmware`) to the IR at `crates/slicer-ir/src/slice_ir.rs`.
- Extend `GCodeCommand::Retract` and `GCodeCommand::Unretract` with a `mode: RetractMode` field at the same location.
- Add `[config.schema.retract_mode]` to `modules/core-modules/path-optimization-default/path-optimization-default.toml` (enum, values `gcode`/`firmware`, default `gcode`).
- Read the new config in `modules/core-modules/path-optimization-default/src/lib.rs` (`on_print_start`, around line 196) and propagate it into every `push_retract` / `push_unretract` call (around lines 269-292).
- Branch on `mode` in `DefaultGCodeEmitter` at `crates/slicer-host/src/gcode_emit.rs:410-426` to emit either `G1 E[-]<len> F<speed>` or `G10` / `G11`.
- Reframe `benchy_gcode_contains_balanced_retract_and_unretract_pairs` to assert balanced `G1 E-` / `G1 E<positive>` pairs, plus a negative clause that no `G10`/`G11` lines appear under default config.
- Add `benchy_gcode_firmware_retraction_emits_balanced_g10_g11` (E2E with config override) and `gcode_emit_dispatches_per_command_retract_mode` (unit, synthetic IR) and `retract_mode_propagates_into_ir_commands` (path-optimization-default unit) and `config_schema_rejects_unknown_retract_mode` (host config-validation unit).

## Out of Scope

- Wipe-on-retract (`retract_before_wipe`).
- Per-extruder or multi-tool retraction state machines.
- `retraction_minimum_travel`, `retract_restart_extra`, `deretraction_speed` differentiation (current emit uses one `speed` for both directions; that's packet 15's contract and remains).
- Emitting `M207` / `M208` *configuration* setters. OrcaSlicer does not emit them; we follow that convention. Users wanting firmware-retraction parameters set them in their printer's start G-code.
- Z-hop integration changes (already handled by packet 15 / path-optimization travel policy).
- Per-region or per-role retraction-mode mixing — `retract_mode` is a single print-level setting; the IR field is per-command only because that is the cheapest carrier across the producer/consumer boundary.

## Authoritative Docs

- `docs/02_ir_schemas.md` — IR enum/struct evolution rules; required for adding `RetractMode` and the `mode` field non-breakingly.
- `docs/03_wit_and_manifest.md` — Module Manifest Schema (TOML); §"Config Field Types Reference" for `enum` type semantics and required `values` array; §config-validation enforcement.
- `docs/05_module_sdk.md` — `on_print_start` config-read lifecycle and how `cfg.read_string("retract_mode")?` (or equivalent enum read) reaches the module.
- `docs/08_coordinate_system.md` — sanity check: retract `length` is mm at IR boundary; emitter formats with `format_coord` (already in place at `gcode_emit.rs:413, 422`); orthogonal to mode.

## OrcaSlicer Reference Obligations

- `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` — `use_firmware_retraction` config definition.
- `OrcaSlicerDocumented/src/libslic3r/GCodeWriter.cpp` — `_retract()` branch on firmware mode (G10/G11 vs inline E); `unretract()` branch likewise.
- `OrcaSlicerDocumented/src/libslic3r/Extruder.cpp` and `Extruder.hpp` — confirms M207/M208 are NOT emitted by OrcaSlicer; firmware retraction parameters are user-configured via printer start G-code. This packet honors that convention.

The implementer must not read OrcaSlicer source directly. Inspect via `OrcaSlicerDocumented/` only, and only via sub-agent dispatches that return one-line role descriptions or ≤ 30-line snippets.

## Acceptance Summary

### Positive Cases

- Default config (no `retract_mode` set) emits balanced `G1 E-` retracts and `G1 E<positive>` unretracts on the Benchy fixture; counts equal; > 0 each. (AC-1)
- Config override `retract_mode = "firmware"` emits balanced `G10` / `G11` on the same fixture; counts equal; > 0 each. (AC-2)
- Path-optimization-default propagates the configured mode into every `GCodeCommand::Retract` / `GCodeCommand::Unretract` it pushes. (AC-3)
- The manifest declares `retract_mode` as an enum with values `["gcode", "firmware"]`, default `"gcode"`. (AC-4)
- The emitter dispatches per command on `mode`, not on a global emitter flag. (AC-5)

### Negative Cases

- Default mode emits zero `G10` / `G11` lines (NC-1, embedded in AC-1).
- Firmware mode emits zero `G1 E-` retract-style lines (NC-2, embedded in AC-2).
- Config-validation rejects `retract_mode = "marlin"` (or any non-enum value) with a diagnostic that names the field and the offending value (NC-3 / AC-4).

### Measurable Outcomes

- One existing failing E2E test (`benchy_gcode_contains_balanced_retract_and_unretract_pairs`) becomes green after reframing.
- One new E2E test green (`benchy_gcode_firmware_retraction_emits_balanced_g10_g11`).
- One new emitter unit test green (`gcode_emit_dispatches_per_command_retract_mode`).
- One new module unit test green (`retract_mode_propagates_into_ir_commands`).
- One new host config-validation unit test green (`config_schema_rejects_unknown_retract_mode`).
- `cargo build --workspace`, `cargo test --workspace`, `cargo clippy --workspace -- -D warnings` all green.
- Zero net change to the produced G-code byte stream for any existing test that does not exercise `retract_mode` (verified by running the full Benchy E2E suite).

### Cross-Packet Impact

- **Packet 21 (`benchy-acceptance-evidence`, currently `draft`):** flipped to `status: superseded` because its AC-3 was a malformed assertion against `M207`/`M208`. Packet 34 absorbs the retract/unretract acceptance evidence and reframes the assertion. The flip is performed in step 7 of the implementation plan.
- **Packet 15 (`live-travel-retraction-policy`, `implemented`):** unchanged. Its G-code-mode emission remains the default behavior. Packet 34 extends it; it does not replace it. No status change.
- **Backlog (`docs/07_implementation_status.md`):** TASK-120d2 was marked `[x]` based on packet 15's emission work plus packet 21's (incorrect) acceptance evidence. After packet 34 lands, TASK-120d2 should remain `[x]` (the underlying capability is still emitted; the regression assertion is now correct). TASK-135 should remain `[ ]` until all four Benchy regression-assertion families (supports, top/bottom fills, seams, retract/unretract pairs) are green together. The retract/unretract family becomes the first to satisfy its share.

## Verification Commands

Per acceptance criterion (delegation-friendly; each emits a single PASS/FAIL with cargo's standard summary line):

- AC-1: `cargo test -p slicer-host --test benchy_end_to_end_tdd benchy_gcode_contains_balanced_retract_and_unretract_pairs -- --exact --nocapture`
- AC-2: `cargo test -p slicer-host --test benchy_end_to_end_tdd benchy_gcode_firmware_retraction_emits_balanced_g10_g11 -- --exact --nocapture`
- AC-3: `cargo test -p path-optimization-default retract_mode_propagates_into_ir_commands -- --exact --nocapture`
- AC-4: `cargo test -p slicer-host config_schema_rejects_unknown_retract_mode -- --exact --nocapture`
- AC-5: `cargo test -p slicer-host gcode_emit_dispatches_per_command_retract_mode -- --exact --nocapture`

Workspace-level (after step verifications pass):
- `cargo build --workspace`
- `cargo test --workspace`
- `cargo clippy --workspace -- -D warnings`
- `cargo test -p slicer-host --test benchy_end_to_end_tdd` (full Benchy suite — guards against silent regressions elsewhere in Benchy E2E while reframing one of its assertions)

## Step Completion Expectations

Each step in `implementation-plan.md` declares its own falsifying-check command. Steps must be completed in order: the IR change (Step 1) unblocks the module change (Step 2), which unblocks the emitter change (Step 3), which unblocks the test reframing (Step 4) and the new firmware test (Step 5). Final step is the workspace-level acceptance gate plus the predecessor-flip and packet closure.
