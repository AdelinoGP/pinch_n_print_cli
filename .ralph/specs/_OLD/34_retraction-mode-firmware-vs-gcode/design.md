## Controlling Code Paths

The producer/consumer chain for retract/unretract spans three crates:

1. **Producer** — `modules/core-modules/path-optimization-default/src/lib.rs`:
   - `on_print_start` (around line 196): reads module config keys (`retract_length`, `retract_speed`, `travel_z_hop`) into the module struct. The new `retract_mode` read goes here.
   - `run_path_optimization` (call sites at lines 269-271 and 290-292): pushes `output.push_retract(self.retract_length, self.retract_speed)` and `output.push_unretract(self.retract_length, self.retract_speed)` between travel segments. Each call must carry the resolved `RetractMode`.

2. **Carrier** — `crates/slicer-ir/src/slice_ir.rs`:
   - `GCodeCommand` enum at line 1429: variants `Retract { length, speed }` and `Unretract { length, speed }` are extended to `Retract { length, speed, mode }` / `Unretract { length, speed, mode }`.
   - New `RetractMode` enum sibling: variants `Gcode`, `Firmware`. Derives `Copy`, `Clone`, `Debug`, `PartialEq`, `Eq`, `Hash`, plus the IR module's standard serde derives.
   - The SDK shim that wraps `push_retract` / `push_unretract` for guest modules must accept the new mode parameter.

3. **Consumer** — `crates/slicer-host/src/gcode_emit.rs`:
   - `DefaultGCodeEmitter::emit_gcode` (line 96): the per-command match at lines 410-426. The current arms write `G1 E-{} F{}` / `G1 E{} F{}` unconditionally; they become an inner branch on `mode`.

## Architecture Constraints

- **IR additivity (`docs/02_ir_schemas.md`).** Adding a field to an existing `GCodeCommand` variant is a breaking change for any matcher that destructures the variant exhaustively. The slice-IR is internal to this workspace, so the change is workspace-wide but not externally observable. Every match arm on `GCodeCommand::Retract` / `GCodeCommand::Unretract` in the workspace must be updated; sub-agent dispatch in Step 1 enumerates them before editing.
- **Manifest schema (`docs/03_wit_and_manifest.md`).** Enum config fields require `type = "enum"`, a `values` array, a `default` whose value is in `values`, and a `display` string. Validation runs at module-load time at the host boundary; an unknown value MUST be rejected with a diagnostic that names the field and the offending value.
- **Module SDK lifecycle (`docs/05_module_sdk.md`).** Config reads happen exactly once in `on_print_start`; the module stores the resolved `RetractMode` on its struct (e.g., `self.retract_mode: RetractMode`) and never re-reads per-layer or per-region.
- **Coordinate system (`docs/08_coordinate_system.md`).** `length` and `speed` are mm and mm/min respectively at the IR boundary; emitter uses `format_coord`. Mode is orthogonal to units; no unit work in this packet.
- **OrcaSlicer parity.** Firmware mode emits `G10` (retract) and `G11` (unretract). It does NOT emit `M207`/`M208`. This is a deliberate parity decision; do not "fix" the test by emitting `M207`/`M208`.

## Code Change Surface

Primary edits (≤ 3 files per implementation step; total across packet ≈ 5 files):

1. `crates/slicer-ir/src/slice_ir.rs` — add `RetractMode` enum; extend `GCodeCommand::Retract` and `Unretract` with `mode` field.
2. `modules/core-modules/path-optimization-default/path-optimization-default.toml` — add `[config.schema.retract_mode]` block.
3. `modules/core-modules/path-optimization-default/src/lib.rs` — read `retract_mode` in `on_print_start`; pass it into every `push_retract` / `push_unretract` call site.
4. `crates/slicer-host/src/gcode_emit.rs` — branch the two retract/unretract match arms on `mode`.
5. `crates/slicer-host/tests/benchy_end_to_end_tdd.rs` — reframe `benchy_gcode_contains_balanced_retract_and_unretract_pairs`; add `benchy_gcode_firmware_retraction_emits_balanced_g10_g11`.

Plus:
- A new unit test in `modules/core-modules/path-optimization-default/src/lib.rs` (or a sibling `tests/` module): `retract_mode_propagates_into_ir_commands`.
- A new emitter unit test in `crates/slicer-host/src/gcode_emit.rs` or a sibling tests file: `gcode_emit_dispatches_per_command_retract_mode`.
- A new host config-validation unit test: `config_schema_rejects_unknown_retract_mode`.

The SDK helper `output.push_retract(length, speed)` becomes `output.push_retract(length, speed, mode)` (or a sibling `push_retract_with_mode` that delegates). Either signature is acceptable; the chosen one must be consistent across producers. Discovery in Step 1 confirms how many call sites exist (the dispatch at lines 269-271 and 290-292 is the only known producer; if more surface, they are addressed in the same step).

## Selected Approach

**Per-command mode field on `GCodeCommand::Retract` / `Unretract`.** The mode is carried as a `RetractMode` enum field on each command. The producer (path-optimization-default) writes the resolved mode at push time; the consumer (DefaultGCodeEmitter) dispatches per command.

Rejected alternatives:

- **Global emitter mode.** Store `retract_mode` on `DefaultGCodeEmitter` and read it once. Rejected: the emitter does not currently consume module configs directly; threading a path-optimization-default config field through to a host emitter via a side channel breaks the IR-as-contract pattern in `docs/01_system_architecture.md`. Per-command field is cheaper and clearer.
- **Two distinct command variants (`FirmwareRetract` / `GcodeRetract`).** Rejected: doubles the variant count and requires every matcher to handle two cases instead of one with an inner branch. Adds no expressivity over the enum field.
- **String-typed mode.** Rejected: defeats type-checking; the manifest validator already enforces enum membership at config-load, but the IR carrier should be a Rust enum to make the emitter exhaustive.

## Read-only Context the Implementer Needs

- `crates/slicer-ir/src/slice_ir.rs` lines 1420-1450 — the existing `GCodeCommand` definition and any neighboring derive macros / serde attributes. (Total file is large; do NOT read in full — symbol-search for `GCodeCommand` and read ±40 lines.)
- `crates/slicer-host/src/gcode_emit.rs` lines 380-440 — the match arm region around the retract/unretract serialization. The whole file is large; do NOT read in full.
- `modules/core-modules/path-optimization-default/src/lib.rs` lines 180-310 — the `on_print_start` config reads (≈ 196-207) and the `run_path_optimization` body that contains the `push_retract` / `push_unretract` call sites (≈ 269-292).
- `modules/core-modules/path-optimization-default/path-optimization-default.toml` lines 30-60 — existing `[config.schema.*]` blocks for retraction-related fields, to match formatting.
- `crates/slicer-host/tests/benchy_end_to_end_tdd.rs` lines 1200-1260 — the failing test and adjacent helpers (`preview`, `gcode` collection). Read just this range.

## Out-of-bounds Files

- `OrcaSlicerDocumented/**` — only via sub-agent SUMMARY or LOCATIONS dispatches; never read directly.
- `OrcaSlicer/` source (if present elsewhere on disk) — never read.
- `target/`, `Cargo.lock`, generated WIT bindings, vendored dependencies — never load.
- Other packet directories under `.ralph/specs/` — read at most their `packet.spec.md` frontmatter via grep; do not pull in their bodies. The packet-21 status flip is a single-line edit and does not require reading packet 21's body.
- `docs/00_project_overview.md`, `docs/04_host_scheduler.md`, `docs/09_progress_events.md`, `docs/10_glossary_and_scenario_traces.md`, `docs/11_*`, `docs/12_*`, `docs/13_*`, `docs/14_*` — out of scope unless a sub-agent surfaces a specific section as needed; they are not on the critical path for this packet.

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

## Open Questions

None blocking. The user's clarifying answers (G10/G11 only for firmware mode, manifest-level config in path-optimization-default, supersede packet 21 only) closed every ambiguity surfaced during scoping. The packet may be activated as soon as the user confirms.
