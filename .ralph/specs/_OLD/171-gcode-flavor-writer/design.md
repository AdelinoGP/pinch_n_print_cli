# Design: 171-gcode-flavor-writer

## Controlling Code Paths

- Primary code path: `crates/slicer-gcode/src/serialize.rs` — `DefaultGCodeSerializer::serialize_gcode` (command match, lines 555-744), `serialize_config_block` (lines 283-382), `ORCA_CONFIG_PADDING` (line 402+); serializer construction in `crates/slicer-runtime/src/run.rs:619-637` (reads `config_source`, builds `DefaultGCodeSerializer::with_extrusion_mode(relative)`); secondary default construction at `crates/slicer-runtime/src/pipeline.rs:461`.
- Neighboring tests/fixtures: `crates/slicer-gcode/tests/golden_emit_tdd.rs`, `gcode_relative_extrusion_tdd.rs`, `gcode_toolchange_wrapping.rs`; `crates/slicer-runtime/tests/integration/gcode_header_thumbnail_config_blocks_tdd.rs` (CONFIG_BLOCK invariants; read-only here — packet 167 owns edits to it).
- OrcaSlicer comparison: see `requirements.md` §OrcaSlicer Reference Obligations; do not repeat delegation rules.

## Architecture Constraints

- Config key strings are snake_case: `gcode_flavor`, value strings `marlin|marlin2|klipper|reprapfirmware|repetier` (matching OrcaSlicer's config-enum spellings for the five supported variants).
- The dialect layer is a pure string-rendering layer over existing `GCodeCommand` variants — it must not change `GCodeIR`, WIT contracts, or any guest-visible schema. No guest WASM is rebuilt by this packet.
- Pure G-code text work: no geometry or mm/unit conversion is involved (coord-system snippet deliberately omitted).

## Code Change Surface

- Selected approach: a `GcodeFlavor` enum with inherent dialect methods (not a trait object) in a new `crates/slicer-gcode/src/flavor.rs`, ported from canonical `GCodeWriter.cpp` flavor branches; `DefaultGCodeSerializer` stores a `GcodeFlavor` and routes divergent match arms through it. This mirrors Orca's `FLAVOR_IS(...)` branching as Rust `match self` per method, keeping each command's cross-flavor table in one function (greppable against the canonical function of the same name).
- Exact functions/types:
  - `flavor.rs`: `pub enum GcodeFlavor { Marlin, Marlin2, Klipper, RepRapFirmware, Repetier }` (+`Default = Marlin`); `pub fn from_config_str(&str) -> GcodeFlavor` (warn+default on unknown); `pub fn config_str(&self) -> &'static str`; `pub fn set_temperature(&self, tool: u32, celsius: f32, wait: bool) -> String` (multi-line for RRF wait: `G10 P<t> S<c>\nM116\n`); `pub fn set_bed_temperature(&self, celsius: f32, wait: bool) -> String`; `pub fn set_acceleration(&self, mm_s2: u32) -> String`; `pub fn set_travel_acceleration(&self, mm_s2: u32) -> Option<String>` (None where unsupported); `pub fn supports_separate_travel_acceleration(&self) -> bool`; `pub fn set_jerk_xy(&self, jerk: f32) -> String`; `pub fn set_junction_deviation(&self, jd: f32) -> Option<String>` (Marlin2 only); `pub fn set_pressure_advance(&self, pa: f32) -> String`.
  - `serialize.rs`: add `flavor: GcodeFlavor` field + `pub fn with_flavor(self, flavor: GcodeFlavor) -> Self` builder on `DefaultGCodeSerializer` (default `Marlin` in `Default`/`new()`/`with_extrusion_mode()`); route the `GCodeCommand::Temperature` arm (lines 718-725) through `self.flavor.set_temperature(...)`; extend `serialize_config_block` signature with `flavor: GcodeFlavor` (or the pre-resolved string) and emit `gcode_flavor` as a real key before the padding loop when `raw_config` lacks it; delete `("gcode_flavor", "marlin")` from `ORCA_CONFIG_PADDING`; `ThumbnailAwareSerializer` (lines 480-552) gains/forwards the flavor to `serialize_config_block`.
  - `lib.rs`: `pub mod flavor;` + re-export `GcodeFlavor`.
  - `run.rs` (lines 619-637): parse `config_source.get("gcode_flavor")` (`ConfigValue::String`) via `GcodeFlavor::from_config_str`, default `Marlin` when absent; `.with_flavor(flavor)` on the serializer.
  - Tests: new `crates/slicer-gcode/tests/gcode_flavor_dialect_tdd.rs`; new `crates/slicer-runtime/tests/integration/gcode_flavor_config_block_tdd.rs` (+ one `mod` line in the integration bucket harness).
- Rejected alternatives:
  - Trait-per-flavor (`dyn GcodeDialect`): five small variants with mostly-shared behavior; a trait scatters each command's table across five impls and breaks the one-function-per-canonical-function grep property.
  - Dialecting inside `emit.rs` (IR construction): flavor is a serialization concern; keeping `GCodeIR` flavor-agnostic preserves IR determinism and existing IR-level tests.
  - Emitting the real flavor only via padding-order tricks: fragile against packet 167's padding rework; an explicit real-key emit is unambiguous.

## Files in Scope (read + edit)

- `crates/slicer-gcode/src/flavor.rs` (new) - role: dialect layer + attribution header; expected change: whole file (~250 lines incl. tests-adjacent docs).
- `crates/slicer-gcode/src/serialize.rs` - role: serializer routing + CONFIG_BLOCK echo; expected change: flavor field/builder, Temperature arm, `serialize_config_block` signature, padding entry removal.
- `crates/slicer-runtime/src/run.rs` - role: config threading; expected change: ~6 lines at the serializer construction site.
- Justified extras: `crates/slicer-gcode/src/lib.rs` (one `pub mod` + re-export), `crates/slicer-runtime/src/pipeline.rs:461` (default stays `Marlin`; touch only if the constructor signature forces it), the two new test files + integration harness `mod` line.

## Read-Only Context

- `crates/slicer-gcode/src/serialize.rs` - lines 264-410 and 480-744 only - purpose: config-block mechanics, padding table, command match arms.
- `crates/slicer-runtime/src/run.rs` - lines 600-660 only - purpose: serializer construction and `config_source` access pattern.
- `crates/slicer-runtime/tests/integration/gcode_header_thumbnail_config_blocks_tdd.rs` - purpose: existing CONFIG_BLOCK assertions that must keep passing; do not edit (packet 167 owns it).
- `docs/ORCASLICER_ATTRIBUTION.md` - whole (short) - purpose: header text.

## Out-of-Bounds Files

- `OrcaSlicerDocumented/...` - delegate; never load
- `target/`, `Cargo.lock`, generated code, vendored dependencies - never load
- `.ralph/specs/167-config-block-viewer-keys/**` - another packet's directory; consult via SUMMARY dispatch only
- `crates/slicer-gcode/src/emit.rs` - IR construction is unchanged; delegate symbol lookups if needed

## Expected Sub-Agent Dispatches

- Question: exact parameter spellings of `GCodeWriter.cpp::set_temperature`, `set_acceleration_internal`, `set_jerk_xy`, `set_pressure_advance`, `set_junction_deviation` for the five flavors; scope: `OrcaSlicerDocumented/src/libslic3r/GCodeWriter.cpp`; return: `SNIPPETS` (≤3 × 30 lines); purpose: Step 2 port fidelity.
- Question: which integration-bucket harness file lists `mod` entries for `crates/slicer-runtime/tests/integration/*`; scope: `crates/slicer-runtime/tests/`; return: `FACT`; purpose: Step 4 test registration.
- Question: does `cargo check --workspace --all-targets` pass; scope: workspace; return: `FACT` + ≤20 error lines; purpose: every step's gate.

## Data and Contract Notes

- IR/manifest contracts: `GCodeIR` and `GCodeCommand` are untouched; flavor is applied only at serialization.
- WIT boundary: none crossed; no guest rebuild required.
- Canonical divergence table (verified via delegated Orca survey this session; cite by file+function only):
  - `set_temperature`: RRF uses `G10` with `P<tool>` and appends `M116` on wait; Marlin/Marlin2/Klipper/Repetier use `M104`/`M109` with `T<tool> S<temp>` (`GCodeWriter.cpp::set_temperature`).
  - `set_acceleration_internal`: Repetier `M201 X.. Y..` (travel: `M202 X.. Y..`); Marlin2/RRF `M204 P..` (travel `M204 T..`); Klipper `SET_VELOCITY_LIMIT ACCEL=..`; legacy Marlin `M204 S..`.
  - `supports_separate_travel_acceleration`: true only for gcfRepetier, gcfMarlinFirmware, gcfRepRapFirmware.
  - `set_jerk_xy`: Klipper `SET_VELOCITY_LIMIT SQUARE_CORNER_VELOCITY=..`; Repetier `M207 X..`; others `M205 X.. Y..`.
  - `set_junction_deviation`: `M205 J..`, gcfMarlinFirmware only.
  - `set_pressure_advance`: Klipper `SET_PRESSURE_ADVANCE ADVANCE=..`; RRF `M572 D0 S..`; Repetier `M233 X.. Y..`; Marlin/Marlin2 `M900 K..`.
  - Fan `M106 S`, toolchange `T<n>`, `M82`/`M83`, firmware retract `G10`/`G11`, bed `M140`/`M190` are uniform across the five supported flavors (divergences exist only in flavors this packet excludes, e.g. MakerWare `M127`/`M126`, Machinekit `G22`/`G23`).
- Determinism: output remains a pure function of (IR, config); flavor comes from config only.

## Locked Assumptions and Invariants

- Default flavor is `Marlin` everywhere a serializer is constructed without config; all pre-existing emitted bytes are unchanged under the default (AC-6 falsifies this).
- Unknown `gcode_flavor` values never abort a slice — warn-and-default is locked behavior (AC-N1).
- The textual collision between firmware-retract `G10` and RRF's `G10 P.. S..` temperature command is accepted as canonical Orca behavior (RRF disambiguates by parameters); do not remap retract commands for RRF.

## Risks and Tradeoffs

- Merge risk with packet 167: both edit `ORCA_CONFIG_PADDING` and `serialize_config_block` call sites. Entries touched are disjoint (`gcode_flavor` here; speed/accel/jerk/`printer_model` there); the second packet to land rebases textually.
- RRF wait semantics: Orca appends `M116` (wait-for-all) rather than a blocking `G10 R`; the port copies Orca exactly — printer-side behavior differences are out of scope.
- `set_temperature` returning a multi-line string for RRF-wait must end each line with `\n` exactly once to keep golden-diff tooling stable; the dialect unit tests pin the exact strings.
- `serialize_config_block` signature change ripples into `ThumbnailAwareSerializer`; contained within `serialize.rs`.

## Context Cost Estimate

- Aggregate: `M`
- Largest step: `M` (Step 2, the dialect port)
- Highest-risk dispatch and required return format: Orca `GCodeWriter.cpp` parameter-spelling verification — `SNIPPETS` capped at 3×30 lines; reject anything larger.

## Open Questions

- [FWD] Should `from_config_str` also accept Orca's legacy alias spellings (e.g. `"reprap"` for RRF) if the fork frontend is found to emit them? Default answer: no — accept exactly the five strings; extend only with fork-side evidence.
- [FWD] `set_temperature` tool parameter when only one tool exists: Orca drops the `T` param for single-extruder configs (`GCodeWriter.cpp::set_temperature` member overload). PNP currently always emits `T<tool>`; keep PNP's existing behavior for Marlin-family (byte identity, AC-6) and mirror it as `P<tool>` for RRF. Implementer may revisit only without breaking AC-6.
