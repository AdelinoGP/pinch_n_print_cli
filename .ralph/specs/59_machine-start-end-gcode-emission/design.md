# Design: 59_machine-start-end-gcode-emission

## Controlling Code Paths

- **Primary code path:**
  - **NEW core module** `modules/core-modules/machine-gcode-emit/` with three files:
    - `machine-gcode-emit.toml` — declares four `[config.schema.<key>]` blocks (verbatim shape below).
    - `Cargo.toml` — mirrors `modules/core-modules/part-cooling/Cargo.toml`.
    - `src/lib.rs` — implements `FinalizationModule` with a REAL `run_finalization` body that reads the four config keys from `&ConfigView`, runs a private `substitute_placeholders(template: &str, lookup: &HashMap<String, ConfigValue>) -> String` helper (≤ 60 LOC, scoped to this module file) on both templates, and calls `output.push_print_start_gcode(resolved_start)` + `output.push_print_end_gcode(resolved_end)`.
  - **WIT contract extension** (`wit/world-finalization.wit:62-104`, the `finalization-output-builder` resource): add two new methods — `push-print-start-gcode: func(text: string) -> result<_, string>;` and `push-print-end-gcode: func(text: string) -> result<_, string>;`. Additive; no existing method removed.
  - **SDK extension** (`crates/slicer-sdk/src/traits.rs`): the `FinalizationOutputBuilder` impl at `:717` gets two new methods `push_print_start_gcode(&mut self, text: String)` and `push_print_end_gcode(&mut self, text: String)`. Internal storage adds two `Option<String>` fields on the struct at `:704-:715`. The `FinalizationModule` trait at `:1196` is unchanged in signature.
  - **Host enum extension** (`crates/slicer-host/src/wit_host.rs:837`): `FinalizationBuilderPush` grows from 6 variants to 8 by adding `PrintStartGcode(String)` and `PrintEndGcode(String)`. The bindgen-generated `finalization-output-builder` resource bridge near `:4895-:5020` (where existing variants are pushed onto `data.pushes`) gets two new push sites mirroring the new WIT methods.
  - **Dispatch routing** (`crates/slicer-host/src/dispatch.rs`): `dispatch_finalization_call` at `:1079` returns the captured `Vec<wit_host::FinalizationBuilderPush>`. The apply-site loop at `:2892-:2978` (inside `FinalizationStageRunner`) gains two new match arms that DEPOSIT the resolved strings into NEW `Option<String>` fields on `GCodeIR` — `gcode_ir.print_start_gcode` and `gcode_ir.print_end_gcode`. The deposit happens via the existing pipeline path that produces the `GCodeIR` consumed by the host serializer.
  - **IR extension** (`crates/slicer-ir/src/slice_ir.rs:1781`): `GCodeIR` grows by two additive fields `pub print_start_gcode: Option<String>,` and `pub print_end_gcode: Option<String>,`. The existing `Default for GCodeIR` impl at `:1790` extends to initialize both to `None`. No existing field removed or changed.
  - **Host serializer** (`crates/slicer-host/src/gcode_emit.rs`): `DefaultGCodeSerializer::serialize_gcode()` body at `:1021` reads `gcode_ir.print_start_gcode` / `gcode_ir.print_end_gcode` and emits at the contractual byte positions:
    - Start string AFTER `serialize_header_block` (`:667`) + `serialize_width_comments` (`:712`) emission, BEFORE the M82/M83 preamble emission (M83 at `:1067`, M82 at `:1069`).
    - End string AFTER the last layer's commands, BEFORE the `ThumbnailAwareSerializer` wrapper (`:973`) appends THUMBNAIL/CONFIG_BLOCK.
    - Empty / whitespace-only ⇒ no bytes emitted.
  - **The serializer contains NO `substitute_placeholders` helper.** Substitution lives in the WASM guest module.
  - `crates/slicer-host/src/pipeline.rs:435-449` — the build site of `effective_config: HashMap<String, ConfigValue>`. **Not modified**, but cited so the lookup type and origin (which the guest module sees through `ConfigView`) are unambiguous.

- **Neighboring tests or fixtures:**
  - `crates/slicer-host/tests/gcode_header_thumbnail_config_blocks_tdd.rs` — the packet-55 fixture pattern: small STL fixture, slicer invocation, output scan against literal sentinels. The new test file mirrors this structure.
  - `crates/slicer-host/tests/gcode_emit_tdd.rs:1-120` — layer fixture pattern.
  - `crates/slicer-host/tests/postpass_gcode_emit_contract_tdd.rs:1-80` — slicer-cli invocation harness pattern.
  - The new test file exercises END-TO-END flow: `slicer-cli` invocation → `ResolvedConfig` → module reads keys → module substitutes → module pushes via new variants → dispatch routes to `GCodeIR` fields → serializer emits at correct positions.

- **OrcaSlicer comparison surface:**
  - `OrcaSlicerDocumented/src/libslic3r/PrintConfig.hpp` (`machine_start_gcode` / `machine_end_gcode` field declarations) — confirms key names.
  - `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` (stock defaults `add` + `set_default_value`) — defaults intentionally not borrowed.
  - `OrcaSlicerDocumented/src/libslic3r/GCode.cpp:3181` / `:3200` / `:3258` — start-block-then-preamble ordering (borrowed).
  - `OrcaSlicerDocumented/src/libslic3r/GCode.cpp:3544` — end-block-after-CONFIG ordering (intentionally not borrowed; explained under Risks and Tradeoffs).
  - `OrcaSlicerDocumented/src/libslic3r/PlaceholderParser.cpp` (`apply_config()` symbol-table contract) — borrowed.

## Architecture Constraints

- All config keys use snake_case literals (per `CLAUDE.md` Config Key Naming Convention). The four new keys conform.
- Config keys consumed by the runtime are declared in a core-module `[config.schema.<key>]` block (canonical schema-declaration site). Precedent: `modules/core-modules/part-cooling/part-cooling.toml`, `modules/core-modules/seam-placer/seam-placer.toml` (for the `string` type). This packet introduces `modules/core-modules/machine-gcode-emit/` for the four keys.
- The substitution work runs INSIDE the new module's `run_finalization` body (in the WASM guest). The serializer only places the resolved strings. This matches the architectural rule that modules do the work their names imply: a module called `machine-gcode-emit` emits machine gcode.
- The substituted start string appears BEFORE the M82/M83 preamble (packet-54 invariant). This matches OrcaSlicer ordering.
- The substituted end string appears BEFORE the THUMBNAIL_BLOCK + CONFIG_BLOCK footer emitted by `ThumbnailAwareSerializer`.
- The four new config keys flow into CONFIG_BLOCK automatically via packet-55's `serialize_config_block()` (at `crates/slicer-host/src/gcode_emit.rs:928`) because that helper iterates the effective `ConfigView`. No explicit CONFIG_BLOCK wiring is needed in this packet.
- The substitution helper is a private free function INSIDE the module's `src/lib.rs`, NOT a public API. If a future packet (arithmetic, conditionals) needs it elsewhere, that future packet may promote it; this packet does not.
- The helper performs ONE pass. A substituted value containing `[other_key]` is NOT re-scanned. This avoids non-termination and matches the principle of least surprise.
- Empty / whitespace-only resolved string ⇒ no bytes emitted. No header comment line, no blank line, no phantom sentinel. This is what makes the default `machine_end_gcode = "PRINT_END"` distinguishable from a user-set `machine_end_gcode = ""`.
- Additive WIT/SDK/IR change. No existing variant removed, no existing field changed. The original packet text declared "No new WIT contracts" — that constraint was authored against the pass-1 no-op design and is explicitly rescinded by this refinement.

## TOML Manifest Shape (verbatim)

The new module's manifest mirrors `modules/core-modules/part-cooling/part-cooling.toml`'s shape — the `[module]` header keys (`id`, `version`, `display-name`, `description`, `author`, `license`, `wit-world`) and the separate `[stage]` block are copied verbatim. The `[config.schema.<key>]` blocks declare the four keys exactly:

```toml
[module]
id           = "com.core.machine-gcode-emit"
version      = "0.1.0"
display-name = "Machine G-code Emit"
description  = "Emits machine_start_gcode / machine_end_gcode with minimal [key] substitution against the effective ConfigView. Reads four config keys and pushes the resolved start/end strings via the FinalizationOutputBuilder's print-boundary methods."
author       = "modular-slicer"
license      = "MIT"
wit-world    = "slicer:world-finalization@1.0.0"

[stage]
id = "PostPass::LayerFinalization"

[ir-access]
reads  = []
writes = []

[claims]
holds    = []
requires = []

[compatibility]
incompatible-with = []
requires          = []
min-host-version  = "0.1.0"
min-ir-schema     = "1.0.0"
max-ir-schema     = "2.0.0"

[config.schema]

[config.schema.machine_start_gcode]
type    = "string"
default = """M190 S[bed_temperature_initial_layer_single]
M109 S[nozzle_temperature_initial_layer]
PRINT_START EXTRUDER=[nozzle_temperature_initial_layer] BED=[bed_temperature_initial_layer_single]"""
display = "Machine Start G-code"
group   = "Machine G-code"

[config.schema.machine_end_gcode]
type    = "string"
default = "PRINT_END"
display = "Machine End G-code"
group   = "Machine G-code"

[config.schema.bed_temperature_initial_layer_single]
type    = "int"
default = 60
min     = 0
max     = 120
display = "Bed Temperature (Initial Layer)"
group   = "Machine G-code"

[config.schema.nozzle_temperature_initial_layer]
type    = "int"
default = 215
min     = 0
max     = 300
display = "Nozzle Temperature (Initial Layer)"
group   = "Machine G-code"

[config.overridable-per-region]
keys = []

[config.overridable-per-layer]
keys = []

[hints]
estimated-ms-per-layer = 0
layer-parallel-safe    = true
```

The `[module]` block's exact key names (`id`, `version`, `display-name`, `description`, `author`, `license`, `wit-world`) are NOT placeholders — they must match `part-cooling.toml`'s shape verbatim. Triple-quoted `"""..."""` is used for `machine_start_gcode`'s multi-line default so newlines are preserved exactly.

## src/lib.rs shape

```rust
//! Core module: emits machine_start_gcode / machine_end_gcode with minimal [key] substitution
//! against the effective ConfigView. The module reads four declared config keys, runs a private
//! single-pass substitution helper on both templates, and pushes the resolved start/end strings
//! through FinalizationOutputBuilder::push_print_start_gcode / push_print_end_gcode. The host
//! serializer (gcode_emit.rs) places those resolved strings at the contractual byte offsets;
//! it does no substitution itself.

#![warn(missing_docs)]
#![warn(unused_imports)]

use std::collections::HashMap;

use slicer_ir::{ConfigValue, ConfigView};
use slicer_sdk::error::ModuleError;
use slicer_sdk::slicer_module;
use slicer_sdk::traits::{FinalizationModule, FinalizationOutputBuilder, LayerCollectionView};

/// Machine-gcode-emit finalization module.
pub struct MachineGcodeEmit;

#[slicer_module]
impl FinalizationModule for MachineGcodeEmit {
    fn on_print_start(_config: &ConfigView) -> Result<Self, ModuleError> {
        Ok(Self)
    }

    fn run_finalization(
        &self,
        _layers: &[LayerCollectionView],
        output: &mut FinalizationOutputBuilder,
        config: &ConfigView,
    ) -> Result<(), ModuleError> {
        // 1. Read the four declared keys from ConfigView.
        let start_template = config.get_string("machine_start_gcode").unwrap_or_default();
        let end_template   = config.get_string("machine_end_gcode").unwrap_or_default();
        let bed_temp       = config.get_int("bed_temperature_initial_layer_single").unwrap_or(60);
        let nozzle_temp    = config.get_int("nozzle_temperature_initial_layer").unwrap_or(215);

        // 2. Build a lookup HashMap<String, ConfigValue> for substitution.
        let mut lookup: HashMap<String, ConfigValue> = HashMap::new();
        lookup.insert("bed_temperature_initial_layer_single".into(), ConfigValue::Int(bed_temp));
        lookup.insert("nozzle_temperature_initial_layer".into(),     ConfigValue::Int(nozzle_temp));
        // Other keys may be added by future packets.

        // 3. Single-pass substitution on both templates.
        let resolved_start = substitute_placeholders(&start_template, &lookup);
        let resolved_end   = substitute_placeholders(&end_template,   &lookup);

        // 4. Push the resolved strings via the new print-boundary methods.
        output.push_print_start_gcode(resolved_start);
        output.push_print_end_gcode(resolved_end);

        Ok(())
    }
}

/// Single-pass `[snake_case_key]` substitution. Unknown keys pass through verbatim;
/// unclosed `[` is treated as literal text. ≤ 60 LOC.
fn substitute_placeholders(template: &str, lookup: &HashMap<String, ConfigValue>) -> String {
    // Implementation sketch — full body fills in:
    //   - Walk `template` left-to-right.
    //   - On `[`, scan to matching `]` on the same line; if no match, treat `[` as literal.
    //   - If the bracketed identifier is in `lookup`, append the stringified ConfigValue.
    //   - Otherwise, append the original `[key]` substring verbatim.
    //   - Single pass — do not re-scan substituted values.
    String::new() // placeholder; real body is implemented during Step 4
}
```

The exact `ConfigView` API surface (`get_string` / `get_int` / a unified `get`) is whichever the SDK provides today; the implementer mirrors `part-cooling`'s pattern verbatim and treats absent keys as defaults that match the manifest declaration.

## Code Change Surface

- **Selected approach:** New core module performs real substitution work (matching its name) inside `run_finalization`; the resolved strings are shipped to the serializer through two new `FinalizationBuilderPush` variants routed by the dispatch layer into two new additive `GCodeIR` fields. The serializer places the strings at the contractual byte offsets. Minimal blast radius for the architectural correctness it gains (modules do the work their names imply).

- **Exact functions, traits, manifests, tests, or fixtures expected to change:**
  1. `modules/core-modules/machine-gcode-emit/machine-gcode-emit.toml` — NEW manifest (≤ 70 LOC) declaring four `[config.schema.<key>]` blocks per the TOML shape above.
  2. `modules/core-modules/machine-gcode-emit/Cargo.toml` — NEW Cargo manifest (≤ 25 LOC) mirroring `modules/core-modules/part-cooling/Cargo.toml`.
  3. `modules/core-modules/machine-gcode-emit/src/lib.rs` — NEW Rust source. `MachineGcodeEmit` struct + `#[slicer_module] impl FinalizationModule` with a REAL `run_finalization` body that reads the four keys, runs `substitute_placeholders` (≤ 60 LOC, private to the module file), and calls the two new builder methods. Total file size ~120-150 LOC.
  4. `wit/world-finalization.wit` — additive: 2 new methods on `finalization-output-builder` (~2 lines of WIT).
  5. `crates/slicer-sdk/src/traits.rs` — additive: 2 new methods on `FinalizationOutputBuilder` impl + 2 `Option<String>` fields on the struct (≈ 15 LOC total).
  6. `crates/slicer-host/src/wit_host.rs` — additive: 2 new variants on `FinalizationBuilderPush` enum + 2 new push sites in the bindgen-generated resource bridge near `:4895-:5020` (≈ 25-30 LOC).
  7. `crates/slicer-host/src/dispatch.rs` — additive: 2 new match arms inside the apply-site loop at `:2892-:2978`. The arms deposit the resolved strings into the new `GCodeIR` fields (≈ 15-20 LOC).
  8. `crates/slicer-ir/src/slice_ir.rs` — additive: 2 `Option<String>` fields on `GCodeIR` at `:1781` + `Default` impl update at `:1790` (≈ 6 LOC).
  9. `crates/slicer-host/src/gcode_emit.rs` — read `gcode_ir.print_start_gcode` and `gcode_ir.print_end_gcode` inside `DefaultGCodeSerializer::serialize_gcode()` body at `:1021` and insert at the contractual byte positions. No `substitute_placeholders` helper added (it lives in the module). ≈ 20-25 LOC total addition.
  10. `crates/slicer-host/tests/machine_start_end_gcode_emission_tdd.rs` — NEW test file. 9 positive + 3 negative tests = 12 total; reuses the predecessor STL fixture and the same `slicer-cli` invocation harness used by the packet-55 tests. ≤ 400 LOC. Exercises end-to-end (`slicer-cli` → `ResolvedConfig` → module → dispatch → `GCodeIR` → serializer → file scan).
  11. `docs/07_implementation_status.md` — append three rows for TASK-193, TASK-193a, TASK-193b in the appropriate queued/in-progress section. Edit via worker dispatch.

- **Rejected alternatives:**
  - **A. No-op `FinalizationModule` (schema-only stub).** Rejected on architectural-honesty grounds: a module named `machine-gcode-emit` that doesn't emit machine gcode is misnamed and breaks the pattern of every other module doing the work it claims to do. Was the pass-1 refinement's design; reverted in pass 2 after user pushback.
  - **B. Host-only substitution + injection.** Rejected: leaves no module owner for the four config keys, contradicting the architectural rule that config keys live in module manifests.
  - **C. Module emits via existing layer-scoped `LayerAnnotation::Raw` against layer 0 / last layer.** Rejected: layer 0's annotations land inside the layer's commands, not before HEADER_BLOCK. The byte-offset contract (start block BEFORE M82/M83 preamble) cannot be satisfied. New `PrintStartGcode` / `PrintEndGcode` variants are required to express the correct emission boundary.
  - **D. `TextPostProcess` injects after serialization.** Rejected: positioning relative to HEADER_BLOCK_END / first G1 / CONFIG_BLOCK_START is structurally a serializer concern.
  - **E. Full OrcaSlicer `PlaceholderParser` grammar.** Rejected on scope. Tracked as three follow-up packets (arithmetic, conditionals, builtins).
- **Routing choice for the new `FinalizationBuilderPush` variants:** the apply-site loop in `dispatch.rs` writes the resolved strings into NEW `Option<String>` fields on `GCodeIR` (option 4a in the refinement notes), NOT into ad-hoc serializer constructor arguments (option 4b). Justification: explicit IR fields are easier to test (the `GCodeIR` struct is observable from postpass tests), easier to trace in a debugger (a typed field rather than a map merge), and naturally serializable via the existing `Serialize, Deserialize` derives on `GCodeIR`. Option 4b (passing the strings through pipeline.rs into the serializer constructor) would require touching `crates/slicer-host/src/pipeline.rs:435-449` which is otherwise unmodified by this packet, and would couple the serializer signature to print-boundary concerns instead of exposing them through the IR contract.

## Files in Scope (read + edit)

Primary edit targets:

- `wit/world-finalization.wit` — additive: 2 new methods on `finalization-output-builder` (~2 lines).
- `crates/slicer-sdk/src/traits.rs` — additive: 2 new `FinalizationOutputBuilder` methods + 2 struct fields (≈ 15 LOC).
- `crates/slicer-host/src/wit_host.rs` — additive: 2 new variants on `FinalizationBuilderPush` + 2 push sites in resource bridge (≈ 25-30 LOC).
- `crates/slicer-host/src/dispatch.rs` — additive: 2 new match arms in apply-site loop (≈ 15-20 LOC).
- `crates/slicer-ir/src/slice_ir.rs` — additive: 2 fields on `GCodeIR` + `Default` extension (≈ 6 LOC).
- `crates/slicer-host/src/gcode_emit.rs` — read GCodeIR fields + insert at byte positions (≈ 20-25 LOC). NO `substitute_placeholders` helper here.
- `crates/slicer-host/tests/machine_start_end_gcode_emission_tdd.rs` — NEW (≤ 400 LOC).
- `modules/core-modules/machine-gcode-emit/machine-gcode-emit.toml` — NEW (≤ 70 LOC).
- `modules/core-modules/machine-gcode-emit/Cargo.toml` — NEW (≤ 25 LOC).
- `modules/core-modules/machine-gcode-emit/src/lib.rs` — NEW (~120-150 LOC including the `substitute_placeholders` helper).

Secondary edit (worker dispatch only):

- `docs/07_implementation_status.md` — append three rows (TASK-193, TASK-193a, TASK-193b). NEVER loaded into the implementer's context; all reads/edits via worker dispatch.

## Read-Only Context

Files the implementer is allowed to read but not edit. Range-read when > 300 lines.

- `crates/slicer-host/src/gcode_emit.rs:667-:740` — HEADER + width comments; purpose: confirm byte boundary AFTER which the start block is inserted.
- `crates/slicer-host/src/gcode_emit.rs:928-:1020` — CONFIG_BLOCK at `:928` + `ThumbnailAwareSerializer` at `:973`; purpose: confirm the end block is emitted by the INNER serializer (before the wrapper's THUMBNAIL/CONFIG_BLOCK append).
- `crates/slicer-host/src/gcode_emit.rs:1021-:1166` — `DefaultGCodeSerializer::serialize_gcode()` body + preamble emission (M83 at `:1067`, M82 at `:1069`); purpose: identify both insertion points by inspection.
- `crates/slicer-host/src/dispatch.rs:1070-:1100` — `dispatch_finalization_call` entry at `:1079`; purpose: function signature.
- `crates/slicer-host/src/dispatch.rs:2885-:2980` — apply-site loop at `:2892-:2978`; purpose: where the new match arms slot in.
- `crates/slicer-host/src/wit_host.rs:830-:895` — `FinalizationBuilderPush` enum at `:837`; purpose: existing 6 variants, where to insert 2 new ones.
- `crates/slicer-host/src/wit_host.rs:4895-:5020` — existing bindgen-generated bridge push sites; purpose: pattern to mirror for the 2 new methods.
- `crates/slicer-host/src/pipeline.rs:435-449` — `effective_config: HashMap<String, ConfigValue>` build site; purpose: confirm the lookup map's type and origin (consumed indirectly via `ConfigView` from the guest).
- `crates/slicer-ir/src/slice_ir.rs:550-:580` — `ConfigValue` enum at `:557` (`Bool`, `Int`, `Float`, `String`, `List`); purpose: stringification rules.
- `crates/slicer-ir/src/slice_ir.rs:1779-:1799` — `GCodeIR` struct at `:1781` + `Default` impl at `:1790`; purpose: site of the additive field extension.
- `crates/slicer-sdk/src/traits.rs:700-:730` — `FinalizationOutputBuilder` struct at `:704` + impl start at `:717`; purpose: where to add the 2 new methods.
- `crates/slicer-sdk/src/traits.rs:1196-:1230` — `FinalizationModule` trait at `:1196`; purpose: confirm signature unchanged.
- `wit/world-finalization.wit` — full read OK (file is short, ≤ 130 lines); purpose: see resource `:62-104` and add 2 methods inside.
- `modules/core-modules/part-cooling/part-cooling.toml` (full — ≤ 100 lines) — purpose: `[module]` / `[stage]` / `[config.schema.<key>]` shape for `int` keys.
- `modules/core-modules/part-cooling/Cargo.toml` (full — ≤ 25 lines) — purpose: Cargo manifest template.
- `modules/core-modules/part-cooling/src/lib.rs` (full — 150 LOC) — purpose: the `FinalizationModule` impl precedent (the implementer copies the trait wiring and replaces the body with the substitution + push calls).
- `modules/core-modules/seam-placer/seam-placer.toml` (full — ≤ 50 lines) — purpose: `string`-type `[config.schema.<key>]` precedent (`seam_mode`).
- `crates/slicer-host/tests/gcode_header_thumbnail_config_blocks_tdd.rs` (full file) — purpose: TDD scaffolding pattern (slicer invocation, output scan, sentinel substring assertions).
- `crates/slicer-host/tests/gcode_emit_tdd.rs:1-120` — purpose: layer fixture + M104/M109 capture pattern.
- `crates/slicer-host/tests/postpass_gcode_emit_contract_tdd.rs:1-80` — purpose: slicer-cli invocation harness pattern.
- `.ralph/specs/55_gcode-header-thumbnail-config-blocks/packet.spec.md` (already loaded in generation context) — purpose: row-formatting precedent for docs/07 edit dispatch.

## Out-of-Bounds Files

Files the implementer must NOT load directly. The implementer should delegate any fact-checks against this list.

- `OrcaSlicerDocumented/**` — delegate ALL parity checks; the FACT dispatches enumerated in `packet.spec.md` are the only OrcaSlicer evidence this packet needs. In particular, NEVER read `OrcaSlicerDocumented/src/libslic3r/PlaceholderParser.cpp` (~2400 lines of Boost.Spirit grammar).
- `target/`, `Cargo.lock`, generated WASM under `modules/core-modules/*/dist/` — never load.
- Vendored deps — never load.
- `docs/07_implementation_status.md` — > 500 lines; never load full; worker dispatch only.
- `docs/02_ir_schemas.md` — > 300 lines; range-read only at the ranges listed above.
- `crates/slicer-host/src/gcode_emit.rs` — > 1100 lines; range-read at the three ranges listed above. Read-only outside the touched insertion points; the file is touched only for the read-from-GCodeIR + insertion-point additions in Step 5.
- `crates/slicer-host/src/dispatch.rs` — > 3000 lines; range-read at the two ranges listed above.
- `crates/slicer-host/src/wit_host.rs` — multi-thousand lines; range-read at the two ranges listed above. **Touched only by Step 3 to add 2 enum variants + 2 bridge push sites** — not "out of bounds" overall, but read-restricted.
- `crates/slicer-ir/src/slice_ir.rs` — > 1600 lines; range-read at the two ranges listed above.
- `crates/slicer-sdk/src/traits.rs` — > 1200 lines; range-read at the two ranges listed above.
- All other core modules (`modules/core-modules/{seam-placer,arrange,monotonic-fill,…}/src/**`) beyond what is enumerated in Read-Only Context — delegate any pattern checks.
- All other crates (`slicer-cli`, `slicer-helpers`, etc.) beyond the change surface — delegate trait/impl lookups; do not browse.
- All other packets (`.ralph/specs/01*` through `58_*`) other than packet 55 — delegate any pattern checks.

## Expected Sub-Agent Dispatches

Implementers should plan for at least the following dispatches.

- **Step 1 dispatches:**
  - "In `docs/07_implementation_status.md`, find the line range where queued / in-progress G-code-output TASK entries live. Return LOCATIONS, ≤ 5 entries, each with the adjacent row's verbatim text." — purpose: find insertion point.
  - "Append three rows to `docs/07_implementation_status.md` after `<insertion-point-line>`: TASK-193, TASK-193a, TASK-193b. Return FACT: bytes appended." — purpose: edit.
  - "`grep -n 'TASK-193' docs/07_implementation_status.md` — return FACT (hits = 3 expected)." — purpose: verification.
- **Step 2 dispatches:**
  - "In `crates/slicer-host/tests/`, find the small STL fixture path used by `gcode_header_thumbnail_config_blocks_tdd.rs` and `gcode_emit_tdd.rs`. Return FACT: exact `concat!(env!('CARGO_MANIFEST_DIR'), '/../../resources/<filename>.stl')`." — purpose: reuse fixture.
- **Step 3 dispatches:**
  - "Run `grep -rn 'FinalizationBuilderPush' crates/ test-guests/ modules/` and report all sites that match a guest- or host-side reference to the enum. Return FACT (≤ 20 lines)." — purpose: WIT/Type Changes Checklist (identify every site that references the enum to mirror the addition).
  - "Run `cargo build --tests` after the WIT/SDK/dispatch/IR additions. Return FACT pass/fail; SNIPPETS (≤ 30 lines) on fail." — purpose: validate Step 3.
- **Step 4 dispatches:**
  - "After writing `modules/core-modules/machine-gcode-emit/{machine-gcode-emit.toml, Cargo.toml, src/lib.rs}`, run `./modules/core-modules/build-core-modules.sh` then `./modules/core-modules/build-core-modules.sh --check`. Return FACT: '--check returned clean' or SNIPPETS (≤ 20 lines) of the STALE list." — purpose: guest-wasm freshness.
  - "Run `./test-guests/build-test-guests.sh --check` (the WIT change in Step 3 invalidates test-guest bindgen). Return FACT clean/stale." — purpose: test-guest freshness.
  - "Locate the host manifest-discovery API for module `[config.schema.<key>]` lookup (likely `crates/slicer-host/src/manifest.rs` near the `ConfigFieldEntry` parse site at `:827-:828`). Return FACT: API name + file:line, ≤ 6 lines. If no test-friendly API exists, return FACT: 'no direct API; AC uses CONFIG_BLOCK fallback'." — purpose: AC implementation choice.
  - "Run `cargo test -p slicer-host --test machine_start_end_gcode_emission_tdd -- module_manifest_registers_four_keys_with_expected_types_and_defaults --nocapture`. Return FACT (pass/fail); SNIPPETS (≤ 20 lines) on fail." — purpose: validate Step 4.
  - "Run `cargo test -p slicer-host --test machine_start_end_gcode_emission_tdd -- new_keys_appear_in_config_block --nocapture`. Return FACT (pass/fail)." — purpose: validate CONFIG_BLOCK propagation.
- **Step 5 dispatches:**
  - "From `crates/slicer-host/src/gcode_emit.rs`, return SNIPPETS (≤ 30 lines) of the exact insertion site after `serialize_header_block` + `serialize_width_comments` calls within `DefaultGCodeSerializer::serialize_gcode()` body (line ~`:1021`-onwards, up to but not crossing the M83 emission at `:1067`). Cite file:line." — purpose: locate start-block insertion.
  - "From `crates/slicer-host/src/gcode_emit.rs`, return SNIPPETS (≤ 30 lines) of the inner serializer's final accumulation point before the function returns (end-block insertion site). Cite file:line." — purpose: locate end-block insertion.
  - "Run `cargo test -p slicer-host --test machine_start_end_gcode_emission_tdd`. Return FACT pass/fail; SNIPPETS (≤ 20 lines) on first-failing-assertion." — purpose: AC turn-green.
  - "Run `cargo test -p slicer-host --test gcode_header_thumbnail_config_blocks_tdd`. Return FACT pass/fail." — purpose: packet-55 regression.
  - "Run `cargo test -p slicer-host --test gcode_emit_tdd`. Return FACT pass/fail." — purpose: packet-52/54 regression.
- **Step 6 dispatches:**
  - "Run `cargo test -p slicer-host --test postpass_gcode_emit_contract_tdd`. Return FACT." — purpose: postpass regression.
  - "Run `./modules/core-modules/build-core-modules.sh --check`. Return FACT clean/stale." — purpose: guest-wasm freshness re-verification.
  - "Run `./test-guests/build-test-guests.sh --check`. Return FACT clean/stale." — purpose: test-guest freshness re-verification.
  - "Run `cargo check --workspace`. Return FACT pass/fail; SNIPPETS (≤ 30 lines) of first error on fail." — purpose: workspace gate.
  - "Run `cargo clippy --workspace -- -D warnings`. Return FACT pass/fail; SNIPPETS (≤ 30 lines) of first warning on fail." — purpose: lint gate.
- **Step 7 dispatches:**
  - Re-dispatch every pipe-suffixed AC command from `packet.spec.md` (12 total).
  - "Run `cargo test --workspace` once at closure ceremony — return FACT pass/fail; on fail SNIPPETS (≤ 40 lines) of the first failing test name + assertion. NEVER return the full test output." — purpose: final closure gate per CLAUDE.md test discipline.
  - "Update `docs/07_implementation_status.md` rows for TASK-193 / TASK-193a / TASK-193b to status `[x]`; return FACT: rows updated." — purpose: backlog close.

## Data and Contract Notes

- **IR contract touched:** additive only. `GCodeIR` (`crates/slicer-ir/src/slice_ir.rs:1781`) grows by two `Option<String>` fields (`print_start_gcode`, `print_end_gcode`), both defaulting to `None`. No existing field changed; no variant removed; the `schema_version`, `commands`, and `metadata` fields and the `CURRENT_GCODE_IR_SCHEMA_VERSION` constant are untouched. If `docs/02_ir_schemas.md` documents `GCodeIR`'s field set, that doc gets a single-line append; the change does not require a `schema_version` bump because no consumer needs the new fields to interpret the existing ones.
- **WIT boundary considerations:** `wit/world-finalization.wit:62-104` (the `finalization-output-builder` resource) gets two new methods. Additive — no method removed or changed. The new module declares `wit-world = "slicer:world-finalization@1.0.0"` (same as part-cooling). The CLAUDE.md WIT/Type Changes Checklist applies — search all `wit_host.rs`, `dispatch.rs`, and `wit_guest` modules for the affected type, verify type identity matches across component boundaries, run `cargo build --tests`, and update both inline WIT and external package references consistently.
- **SDK contract touched:** additive. `FinalizationOutputBuilder` impl gets two new methods (`push_print_start_gcode`, `push_print_end_gcode`) and the struct gets two `Option<String>` fields. No existing method or field changed. `FinalizationModule` trait signature is unchanged.
- **Dispatch contract touched:** additive. `FinalizationBuilderPush` (`wit_host.rs:837`) grows from 6 to 8 variants. The apply-site loop in `dispatch.rs` (`:2892-:2978`) gets two new match arms. Old guests that don't emit the new variants continue to behave exactly as before.
- **Determinism or scheduler constraints:** The substituted block is deterministic given the same effective `ConfigView`. The module's `substitute_placeholders()` helper iterates the template left-to-right; HashMap key lookup is value-equality and does not introduce ordering nondeterminism. Empty / whitespace-only ⇒ zero bytes.
- **CONFIG_BLOCK propagation:** Packet 55's `serialize_config_block()` (at `:928`) iterates the effective `ConfigView` and emits `; <key> = <value>` per key. The four new keys flow through automatically once declared in the manifest and resolved into `effective_config`. AC `new_keys_appear_in_config_block` verifies this propagation; no additional CONFIG_BLOCK wiring code is added.
- **Multi-line `String` value in CONFIG_BLOCK:** The default `machine_start_gcode` contains `\n` characters. Packet 55's CONFIG_BLOCK formatter handles multi-line values per its existing convention (single comment line with `\n` literalized, or per-continuation `; `). Whichever it picked applies. AC `new_keys_appear_in_config_block` asserts equality after un-escaping; a Step 5 SNIPPETS dispatch confirms the exact wire format.

## Locked Assumptions and Invariants

- The implementer must preserve the byte-offset ordering established by packets 54 / 55:
  - `HEADER_BLOCK_START` < width comments < (NEW: resolved start string) < M82/M83 preamble < first `;LAYER_CHANGE` < first `G1` extrusion move.
  - last `G1` extrusion move < (NEW: resolved end string) < `THUMBNAIL_BLOCK_START` (if present) < `CONFIG_BLOCK_START` < `CONFIG_BLOCK_END` < EOF.
- The module's `substitute_placeholders()` is single-pass — substituted values are not re-scanned.
- The module's `substitute_placeholders()` does not panic, does not infinite-loop, and does not allocate unboundedly. For an N-byte template with K placeholders, runtime is O(N + K · avg-key-length) and allocation is one output `String` of ≤ N + K · max-value-length bytes.
- Empty/whitespace-only resolved string ⇒ emit zero bytes (no header comment, no blank line). A user explicitly setting `machine_end_gcode = ""` must produce a file structurally identical (modulo CONFIG_BLOCK's listing of the empty value) to a file produced without the end block.
- `FinalizationBuilderPush` adds exactly two new variants (`PrintStartGcode(String)`, `PrintEndGcode(String)`). No existing variant changed.
- `GCodeIR` adds exactly two new fields (`print_start_gcode: Option<String>`, `print_end_gcode: Option<String>`). Both default to `None`.
- `FinalizationOutputBuilder` adds exactly two new push methods. Internal storage uses two `Option<String>` fields (single-value, not Vec — there is exactly one print-start and one print-end per slicing run).
- The four new config keys' defaults are EXACTLY as specified in `packet.spec.md` Goal and in the TOML Manifest Shape section above.
- The host serializer (`gcode_emit.rs`) contains NO substitution logic. All substitution happens inside the WASM guest module.

## Risks and Tradeoffs

- **Risk: WIT contract addition.** Two new methods on `finalization-output-builder` (~2 lines of WIT) + two new variants on `FinalizationBuilderPush` (~8 LOC SDK + matching dispatch) + ~25-30 LOC dispatch routing + two `Option<String>` fields on `GCodeIR`. CLAUDE.md's WIT/Type Changes Checklist applies. The addition is additive (no existing variant removed or changed), so older guests continue to build; only the new module exercises the new variants. The original packet declared "No new WIT contracts" — that constraint was authored against a different architecture and is rescinded by this refinement, explicitly noted here.
- **Risk: cross-component diagnostics for unknown placeholders.** With substitution moved into the WASM guest, host-side `log::warn!` capture for unknown placeholders is not trivially available. Pass-2 refinement DROPS the WARN-capture requirement from the negative AC (renamed `unknown_placeholder_passes_through_verbatim`); the verbatim-presence assertion still proves passthrough. Cross-component diagnostic forwarding is tracked as a separate future packet.
- **Risk: end-block position differs from OrcaSlicer.** OrcaSlicer emits `machine_end_gcode` AFTER its CONFIG_BLOCK (`OrcaSlicerDocumented/src/libslic3r/GCode.cpp:3544`). We emit it BEFORE THUMBNAIL/CONFIG_BLOCK because our CONFIG_BLOCK is structurally a footer wrapper. **Mitigation:** comments after the last printable command are ignored by all firmwares. Documented as an intentional deviation in `requirements.md`.
- **Risk: range enforcement is declarative-only (see Out-of-Scope).** The manifest declares `min = 0, max = 120` (bed) and `min = 0, max = 300` (nozzle) but `ResolvedConfig::apply_cli_key` (`crates/slicer-ir/src/resolved_config.rs:194`) does not consult these. A user passing `nozzle_temperature_initial_layer = 999` will get `M109 S999` in the output. Tracked as a separate future TASK-### packet.
- **Risk: multi-line `machine_start_gcode` value in CONFIG_BLOCK may be unparseable by some downstream tools.** Whichever convention packet 55 picked applies. **Mitigation:** Step 5 dispatch confirms the exact wire format; AC `new_keys_appear_in_config_block` validates round-trip via un-escaping.
- **Risk: guest-wasm staleness after adding the new module AND after the WIT change.** Per CLAUDE.md Guest WASM Staleness, `cargo build` does NOT rebuild `modules/core-modules/*/target/wasm32-*/release/*.wasm` and the WIT change invalidates every guest's bindgen output. **Mitigation:** Step 4 mandates running `./modules/core-modules/build-core-modules.sh` once after the new module is created, then `--check` to confirm clean. Step 6 re-runs both `--check` commands.
- **Risk: future "arithmetic" packet breaks one-pass invariant.** Adding `[bed*2]` evaluation means substituted output could contain identifiers that look like further placeholders. **Mitigation:** scope boundary is clear; future packet decides whether to re-scan or to extend the helper into a tokenizer.
- **Tradeoff: minimal substitution vs full OrcaSlicer parity.** Accepted at scope-approval gate. Users who need conditionals can either wait for the follow-up packet or write the literal expansion themselves.
- **Tradeoff: option 4a (IR fields) vs option 4b (constructor plumbing) for routing the new variants.** Chose option 4a (explicit `Option<String>` fields on `GCodeIR`) because IR fields are easier to test and trace than implicit map merges; option 4b would couple the serializer constructor to print-boundary concerns.

## Context Cost Estimate

- Aggregate (sum across all steps): **`M`**.
- Largest single step: Step 3 (WIT/SDK/dispatch/IR extension touching 5 files in additive form) or Step 4 (create the module with real `run_finalization`) — both `M`.
- Highest-risk dispatch: the `cargo test -p slicer-host --test machine_start_end_gcode_emission_tdd` failure return. **Required return format:** FACT pass / SNIPPETS ≤ 20 lines (test name + assertion + minimal context). NEVER return the full test runner output.

## Open Questions

All open questions resolved at the scope-approval gate. **No remaining blockers for activation.** The non-blocking confirmations below are tactical and resolve during Step 4 / Step 5 dispatches:

- **Non-blocking confirmation (Step 4):** Whether the host's manifest-discovery API exposes a test-friendly lookup of `[config.schema.<key>]` blocks. If yes, AC `module_manifest_registers_four_keys_with_expected_types_and_defaults` asserts directly on the loaded manifest. If no, the AC falls back to asserting the four `; <key> = <default>` lines in the CONFIG_BLOCK of `out.gcode`.
- **Non-blocking confirmation (Step 4):** Whether the SDK's `ConfigView` exposes `get_string` / `get_int` separately or via a unified `get` returning `ConfigValue`. The module's `src/lib.rs` mirrors `part-cooling`'s pattern verbatim — whichever shape that uses is what this module uses too.
- **Non-blocking confirmation (Step 5):** Whether the inner `DefaultGCodeSerializer::serialize_gcode()` end-point is unambiguously identifiable, or whether the implementation needs a small refactor to expose a clean injection point. If a refactor is needed, surface it as a packet-local risk and split into a follow-up packet rather than expanding scope.

None of the non-blocking confirmations changes scope, interface, or verification strategy — they are tactical implementation choices. They do NOT block activation.
