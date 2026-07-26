# Task Map: 182-gcode-header-width-defaults

| docs/07 task ID | Packet step | Primary docs | Expected code surface | OrcaSlicer refs | Context cost | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| `TASK-295` | `Step 1` | none required | `crates/slicer-gcode/tests/golden_emit_tdd.rs` | none | `S` | RED whole-line test proves the emitted header currently reports `0.42`/`0.45`. Whole-line because `= 0.4` is a prefix of `= 0.42`. |
| `TASK-295` | `Step 2` | none required | `crates/slicer-gcode/src/serialize.rs` (`DefaultGCodeSerializer::with_extrusion_mode`, two field doc comments) | none | `S` | Corrects both literals to the governing `0.4` and deletes the dangling `config_schema.rs` citation — the two halves of D-165. Attributes `0.4` to `resolve_line_width_mm`, not to OrcaSlicer parity. |
| `TASK-295` | `Step 3` | none required | `crates/slicer-runtime/tests/fixtures/golden/precision_legacy_20mmbox.gcode` | none | `S` | Re-blesses the byte-identity golden that recorded the old header values; `legacy_zero_matches_golden` (`--test e2e`) is RED until this lands. |

Backlog anchor: deviation `D-165-GCODE-HEADER-WIDTH-DEFAULTS-LIE` in `docs/DEVIATION_LOG.md` (also listed in the generated open-deviations block of `docs/07_implementation_status.md`, which is regenerated via `cargo xtask check-deviations` — never hand-edited).

**Task-ID allocation note.** `TASK-295` was free against both `docs/07_implementation_status.md` and `.ralph/specs/**` **at authoring time only**; sibling packets 181 and 183 take `TASK-297` and `TASK-296`. **Never quote a "highest id" figure — it is mutable shared state and rots while you work.** Re-derive from BOTH sources immediately before allocating: `rg -o 'TASK-[0-9]{3}' docs/07_implementation_status.md | sort -u | tail -5` and `rg -o 'TASK-[0-9]{3}' .ralph/specs/ --no-filename | sort -u | tail -5`. Packet 181's first allocation collided precisely because only `docs/07` was checked. If `TASK-295` is taken by implementation time, take the next free id and update every reference in this packet's five files.

No OrcaSlicer parity refs: the corrected value is PnP's own governing internal fallback (`resolve_line_width_mm`, `crates/slicer-runtime/src/builtins/overhang_annotation_producer.rs`), not a ported canonical constant.
