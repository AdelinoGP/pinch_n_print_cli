# Design: 59_machine-start-end-gcode-emission

## Controlling Code Paths

- **Primary code path:**
  - **`GCodeCommand::ExtrusionMode { absolute: bool }` (NEW variant)** at `crates/slicer-ir/src/slice_ir.rs:1697`. Additive. The existing variants (`Move, Retract, Unretract, FanSpeed, Temperature, ToolChange, Comment, Raw`) are unchanged.
  - **Emitter promotion** at `crates/slicer-host/src/gcode_emit.rs:304` (`DefaultGCodeEmitter::emit_gcode`). The body, which currently builds `Vec<GCodeCommand>` from layer IRs, is updated to push `GCodeCommand::ExtrusionMode { absolute }` as the FIRST command. `absolute` is derived from the same flavor decision that today drives the M82-vs-M83 selection at the serializer (`:1154-1156`).
  - **Serializer rewrite** at `crates/slicer-host/src/gcode_emit.rs:1107` (`DefaultGCodeSerializer::serialize_gcode`):
    - Remove the hard-coded `M82\n` / `M83\n` writes at `:1154-1156`.
    - Add a `GCodeCommand::ExtrusionMode { absolute }` arm in the per-command renderer match (sibling to the `Temperature` arm at `:1280-1281`). Render `"M82\n"` when `absolute == true`, `"M83\n"` otherwise.
  - **NEW core module** `modules/core-modules/machine-gcode-emit/` with three files:
    - `machine-gcode-emit.toml` — declares `[stage] id = "PostPass::GCodePostProcess"` and four `[config.schema.<key>]` blocks. Other top-level keys (`[module]`, `[ir-access]`, `[claims]`, `[compatibility]`, `[hints]`) mirror `modules/core-modules/part-cooling/part-cooling.toml`'s shape.
    - `Cargo.toml` — mirrors `modules/core-modules/part-cooling/Cargo.toml`.
    - `src/lib.rs` — implements the SDK's GCodePostProcess trait (exact name confirmed in Step 4 via single dispatch; likely `GCodePostProcessModule`, mirroring `FinalizationModule`'s naming). Body reads four keys from `ConfigView`, runs a private `substitute_placeholders(template: &str, lookup: &HashMap<String, ConfigValue>) -> String` helper (≤ 60 LOC), and rebuilds the output command list as `[Raw(resolved_start), ...existing input commands..., Raw(resolved_end)]`. Empty/whitespace resolved templates SKIP the corresponding `Raw` push.
  - **Scheduler** at `crates/slicer-host/src/execution_plan.rs:38` and `crates/slicer-host/src/postpass.rs:215`. UNCHANGED. The new module slots into the existing `PostPass::GCodePostProcess` stage.
  - **WIT** at `wit/world-postpass.wit:26` (`run-gcode-postprocess` export) and `wit/deps/ir-types.wit:144` (`gcode-output-builder.push-raw`). UNCHANGED structurally. `wit/deps/ir-types.wit`'s `gcode-command` variant set MAY need a new `extrusion-mode(extrusion-mode-payload)` variant if `gcode-command` is mirrored across the WIT boundary; confirm via single dispatch in Step 3.

- **Neighboring tests or fixtures:**
  - `crates/slicer-host/tests/gcode_emit_tdd.rs` — packet-52/54 regression. CRITICAL: contains existing assertions about `M82`/`M83` presence in the output. After the promotion, these must continue to pass without modification.
  - `crates/slicer-host/tests/gcode_header_thumbnail_config_blocks_tdd.rs` — packet-55 regression; fixture pattern reused.
  - `crates/slicer-host/tests/postpass_gcode_emit_contract_tdd.rs` — postpass dispatch contract; slicer-cli invocation harness pattern.
  - The new test file `crates/slicer-host/tests/machine_start_end_gcode_emission_tdd.rs` exercises END-TO-END: `slicer-cli` → `ResolvedConfig` → emitter (now pushing `ExtrusionMode` head) → `GCodePostProcess` module (prepends `Raw(start)`, re-emits, appends `Raw(end)`) → serializer → file scan.

- **OrcaSlicer comparison surface:**
  - `OrcaSlicerDocumented/src/libslic3r/PrintConfig.hpp` — confirms `machine_start_gcode` / `machine_end_gcode` field names.
  - `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` — stock defaults; intentionally NOT borrowed.
  - `OrcaSlicerDocumented/src/libslic3r/GCode.cpp` (machine_start_gcode write, extrusion-mode preamble) — ordering `machine_start_gcode` THEN preamble (borrowed; produced naturally by prepending `Raw(start)` before the `ExtrusionMode` head command).
  - `OrcaSlicerDocumented/src/libslic3r/GCode.cpp` (machine_end_gcode write) — end-block-after-CONFIG ordering; intentionally NOT borrowed.
  - `OrcaSlicerDocumented/src/libslic3r/PlaceholderParser.cpp` (`apply_config()` only) — symbol-table contract; borrowed.

## Architecture Constraints

- All config keys use snake_case literals (per `CLAUDE.md` Config Key Naming Convention). The four new keys conform.
- Config keys consumed by the runtime are declared in a core-module `[config.schema.<key>]` block. Precedent: `modules/core-modules/part-cooling/part-cooling.toml` (int keys), `modules/core-modules/seam-placer/seam-placer.toml` (string keys).
- The substitution work runs INSIDE the new module's `run_gcode_postprocess` body (in the WASM guest). The serializer only renders commands. This matches the rule that modules do the work their names imply: a module called `machine-gcode-emit` emits machine gcode.
- The substituted start string appears BEFORE the `ExtrusionMode` command (which serializes as `M82` or `M83`). This matches OrcaSlicer ordering.
- The substituted end string appears AFTER the last command, BEFORE the `ThumbnailAwareSerializer` wrapper's `THUMBNAIL_BLOCK` + `CONFIG_BLOCK` append (which lives outside `GCodeIR.commands`).
- The four new config keys flow into `CONFIG_BLOCK` automatically via packet-55's `serialize_config_block()` (`crates/slicer-host/src/gcode_emit.rs:928`). No explicit CONFIG_BLOCK wiring is needed.
- The substitution helper is a private free function INSIDE the module's `src/lib.rs`, NOT a public API. If a future packet needs it elsewhere, that packet may promote it.
- The helper performs ONE pass. A substituted value containing `[other_key]` is NOT re-scanned. Avoids non-termination and matches least surprise.
- Empty / whitespace-only resolved template ⇒ the module SKIPS the corresponding `Raw` push. The resulting serialized output contains zero bytes for that block.
- Additive IR change: ONE new `GCodeCommand` variant. No existing variant removed or changed; no `GCodeIR` field added or removed.
- Zero change to `wit/world-finalization.wit`, `wit/world-postpass.wit`, `FinalizationBuilderPush`, `dispatch.rs`, or `GCodeIR`.

## TOML Manifest Shape (verbatim)

The new module's manifest mirrors `modules/core-modules/part-cooling/part-cooling.toml`'s shape. The `[stage]` block sets `id = "PostPass::GCodePostProcess"` (NOT `LayerFinalization`). The `wit-world` references whichever WIT world the existing `GCodePostProcess` modules use (confirm via Step 4 dispatch — most likely `slicer:world-postpass@1.0.0`).

```toml
[module]
id           = "com.core.machine-gcode-emit"
version      = "0.1.0"
display-name = "Machine G-code Emit"
description  = "Emits machine_start_gcode / machine_end_gcode by prepending and appending Raw commands to the GCodePostProcess stream after running [key] substitution against the effective ConfigView."
author       = "modular-slicer"
license      = "MIT"
wit-world    = "slicer:world-postpass@1.0.0"

[stage]
id = "PostPass::GCodePostProcess"

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

The `[module]` header keys (`id`, `version`, `display-name`, `description`, `author`, `license`, `wit-world`) must match `part-cooling.toml`'s key shape verbatim — only the values differ. Triple-quoted `"""..."""` preserves newlines exactly. The `wit-world` value MAY differ from `part-cooling.toml`'s `slicer:world-finalization@1.0.0` — verify the correct postpass world via a Step 4 dispatch before writing.

## src/lib.rs shape

The implementer copies `modules/core-modules/part-cooling/src/lib.rs` as the trait-wiring skeleton, then replaces the body. The exact trait name and signature for `GCodePostProcess` modules is identified in Step 4 by reading `crates/slicer-sdk/src/traits.rs` ranged. Approximate shape:

```rust
//! Core module: emits machine_start_gcode / machine_end_gcode by prepending and appending
//! Raw commands to the GCodePostProcess stream. Performs single-pass [key] substitution
//! against the effective ConfigView. Substitution lives in the WASM guest; the host
//! serializer just renders the command list.

#![warn(missing_docs)]
#![warn(unused_imports)]

use std::collections::HashMap;

use slicer_ir::{ConfigValue, GCodeCommand};
use slicer_sdk::error::ModuleError;
use slicer_sdk::slicer_module;
use slicer_sdk::traits::{ConfigView, GCodeOutputBuilder, GCodePostProcessModule};
// (Exact trait + builder names confirmed in Step 4.)

/// Machine-gcode-emit GCodePostProcess module.
pub struct MachineGcodeEmit;

#[slicer_module]
impl GCodePostProcessModule for MachineGcodeEmit {
    fn on_print_start(_config: &ConfigView) -> Result<Self, ModuleError> {
        Ok(Self)
    }

    fn run_gcode_postprocess(
        &self,
        commands: &[GCodeCommand],
        output: &mut GCodeOutputBuilder,
        config: &ConfigView,
    ) -> Result<(), ModuleError> {
        let start_template = config.get_string("machine_start_gcode").unwrap_or_default();
        let end_template   = config.get_string("machine_end_gcode").unwrap_or_default();
        let bed_temp       = config.get_int("bed_temperature_initial_layer_single").unwrap_or(60);
        let nozzle_temp    = config.get_int("nozzle_temperature_initial_layer").unwrap_or(215);

        let mut lookup: HashMap<String, ConfigValue> = HashMap::new();
        lookup.insert("bed_temperature_initial_layer_single".into(), ConfigValue::Int(bed_temp));
        lookup.insert("nozzle_temperature_initial_layer".into(),     ConfigValue::Int(nozzle_temp));

        let resolved_start = substitute_placeholders(&start_template, &lookup);
        let resolved_end   = substitute_placeholders(&end_template,   &lookup);

        if !resolved_start.trim().is_empty() {
            output.push_raw(resolved_start);
        }
        // Re-emit every input command. The exact SDK method (push-each vs extend) is
        // confirmed in Step 4; whichever the SDK exposes is what we use.
        for cmd in commands {
            output.push_command(cmd.clone());
        }
        if !resolved_end.trim().is_empty() {
            output.push_raw(resolved_end);
        }

        Ok(())
    }
}

/// Single-pass `[snake_case_key]` substitution. Unknown keys pass through verbatim;
/// unclosed `[` is treated as literal. ≤ 60 LOC.
fn substitute_placeholders(template: &str, lookup: &HashMap<String, ConfigValue>) -> String {
    // Walk left-to-right; on `[`, scan to matching `]` on the same line; on no match,
    // emit `[` literally. On match, emit stringified ConfigValue. Single pass.
    // Real body fills in during Step 4.
    String::new()
}
```

The exact `ConfigView` API (`get_string` / `get_int` vs unified `get`) and the exact `GCodeOutputBuilder` method names (`push_raw`, `push_command`, possibly `extend_from_snapshot`) are whichever the SDK exposes today. The implementer mirrors `part-cooling`'s pattern verbatim and the WIT contract in `wit/deps/ir-types.wit:144`.

## Code Change Surface

- **Selected approach:** Promote `M82`/`M83` from the hard-coded serializer preamble to a new `GCodeCommand::ExtrusionMode` variant pushed by the emitter; then add a `PostPass::GCodePostProcess` module that prepends `Raw(machine_start_gcode)` and appends `Raw(machine_end_gcode)` to `GCodeIR.commands`, with substitution performed inside the WASM guest. Minimal blast radius: zero new WIT methods, zero new IR fields, zero new dispatch arms, zero byte-offset arithmetic in the serializer.

- **Exact functions, traits, manifests, tests, or fixtures expected to change:**
  1. `crates/slicer-ir/src/slice_ir.rs` — additive: 1 new variant on `GCodeCommand` (`ExtrusionMode { absolute: bool }`) (≈ 5 LOC including any derived-trait coverage).
  2. `crates/slicer-host/src/gcode_emit.rs` — two coupled edits:
     - In `DefaultGCodeEmitter::emit_gcode` (`:304`), push `ExtrusionMode { absolute }` as the head command (≈ 6 LOC).
     - In `DefaultGCodeSerializer::serialize_gcode` (`:1107`), remove the hard-coded `M82`/`M83` writes at `:1154-1156` (≈ -8 LOC) and add an `ExtrusionMode { absolute }` arm in the per-command renderer (≈ 6 LOC).
  3. `wit/deps/ir-types.wit` — additive: 1 new `extrusion-mode(extrusion-mode-payload)` variant in the `gcode-command` variant ONLY IF `gcode-command` is mirrored in WIT. If not, skip. (Confirmed via Step 3 dispatch.) (≈ 0-4 lines of WIT.)
  4. `modules/core-modules/machine-gcode-emit/machine-gcode-emit.toml` — NEW manifest (≤ 70 LOC).
  5. `modules/core-modules/machine-gcode-emit/Cargo.toml` — NEW Cargo manifest (≤ 25 LOC).
  6. `modules/core-modules/machine-gcode-emit/src/lib.rs` — NEW Rust source. `MachineGcodeEmit` struct + `#[slicer_module] impl GCodePostProcessModule` with a real body. Total ~120-150 LOC.
  7. `crates/slicer-host/tests/machine_start_end_gcode_emission_tdd.rs` — NEW test file. 10 positive + 3 negative = 13 tests; reuses the predecessor STL fixture and the same `slicer-cli` invocation harness. ≤ 400 LOC.
  8. `docs/07_implementation_status.md` — append three rows for TASK-193, TASK-193a, TASK-193b in the appropriate queued section. Edit via worker dispatch.

- **Rejected alternatives:**
  - **A. Keep current packet-59 draft design (finalization-stage + `FinalizationBuilderPush` variants + `Option<String>` IR fields + dispatch arms + byte-offset placement in serializer).** Rejected: significantly more surface area for what amounts to "prepend/append text to the print stream". Adds two WIT methods, two enum variants, two IR fields, two dispatch arms, and ~25 LOC of byte-offset arithmetic in the serializer — all to express something the existing `GCodePostProcess` contract already supports natively.
  - **B. GCodePostProcess module WITHOUT promoting M82/M83.** Rejected: `Raw(machine_start_gcode)` at index 0 of `GCodeIR.commands` would appear AFTER the hard-coded `M82`/`M83` lines (which the serializer writes between `HEADER_BLOCK` and the command loop). Deviates from OrcaSlicer ordering. Acceptable functionally, but the user explicitly chose the promotion approach.
  - **C. `TextPostProcess` module injecting after serialization.** Rejected: positioning relative to `HEADER_BLOCK_END` / `CONFIG_BLOCK_START` would require string-search and splice, which is fragile and structurally a serializer concern. `GCodePostProcess` operates on typed commands and is the right layer.
  - **D. Move M82/M83 into the *commands list* but build them as `Raw("M82")` instead of a typed `ExtrusionMode` variant.** Rejected: less type-safe and inconsistent with `M104/M109` (`GCodeCommand::Temperature`) and other typed variants. A typed variant pays a small cost (one match arm in the renderer) for IR clarity, debug-print quality, and forward compatibility with any future tool that inspects `GCodeIR.commands` for extrusion-mode state.
  - **E. Full OrcaSlicer `PlaceholderParser` grammar.** Rejected on scope. Tracked as three follow-up packets (arithmetic, conditionals, builtins).

## Files in Scope (read + edit)

Primary edit targets:

- `crates/slicer-ir/src/slice_ir.rs` — additive: 1 new `GCodeCommand` variant (≈ 5 LOC).
- `crates/slicer-host/src/gcode_emit.rs` — emitter push + serializer arm; remove hard-coded M82/M83 writes (net ≈ +4 LOC).
- `wit/deps/ir-types.wit` — additive variant ONLY IF `gcode-command` is mirrored in WIT (≈ 0-4 lines).
- `modules/core-modules/machine-gcode-emit/machine-gcode-emit.toml` — NEW (≤ 70 LOC).
- `modules/core-modules/machine-gcode-emit/Cargo.toml` — NEW (≤ 25 LOC).
- `modules/core-modules/machine-gcode-emit/src/lib.rs` — NEW (~120-150 LOC including the `substitute_placeholders` helper).
- `crates/slicer-host/tests/machine_start_end_gcode_emission_tdd.rs` — NEW (≤ 400 LOC).

Secondary edit (worker dispatch only):

- `docs/07_implementation_status.md` — append three rows. NEVER loaded into the implementer's context; all reads/edits via worker dispatch.

## Read-Only Context

Files the implementer is allowed to read but not edit. Range-read when > 300 lines.

- `crates/slicer-host/src/gcode_emit.rs:300-:340` — `DefaultGCodeEmitter::emit_gcode` entry; purpose: identify the push site for the head `ExtrusionMode` command.
- `crates/slicer-host/src/gcode_emit.rs:670-:740` — `serialize_header_block` + `serialize_width_comments`; purpose: confirm header structure unchanged.
- `crates/slicer-host/src/gcode_emit.rs:1100-:1170` — `serialize_gcode` body + the soon-to-be-removed M82/M83 writes at `:1154-1156`; purpose: identify removal site and confirm surrounding logic.
- `crates/slicer-host/src/gcode_emit.rs:1270-:1300` — existing per-command renderer arms (`Temperature` at `:1280-1281`); purpose: model the new `ExtrusionMode` arm on the existing variants.
- `crates/slicer-host/src/postpass.rs:140-:280` — `GCodeEmitter` / `GCodeSerializer` traits and the `execute_postpass_with_instrumentation` body where `run_gcode_postprocess` is dispatched at `:215`.
- `crates/slicer-host/src/execution_plan.rs:38-:80` — confirm `PostPass::GCodePostProcess` is in the canonical stage-ID list.
- `crates/slicer-ir/src/slice_ir.rs:1697-:1770` — `GCodeCommand` enum; purpose: site of the additive variant.
- `crates/slicer-ir/src/slice_ir.rs:1779-:1799` — `GCodeIR` struct (UNCHANGED in this packet; reference only).
- `crates/slicer-ir/src/slice_ir.rs:550-:580` — `ConfigValue` enum at `:557`; purpose: stringification rules for substitution.
- `crates/slicer-sdk/src/traits.rs` — ranged dispatch to find the GCodePostProcess module trait name and signature; purpose: model `MachineGcodeEmit`'s impl.
- `wit/world-postpass.wit` — full read OK (≤ 50 lines); purpose: `run-gcode-postprocess` export signature.
- `wit/deps/ir-types.wit` — full read OK (≤ 200 lines); purpose: `gcode-output-builder` resource methods + `gcode-command` variant set.
- `modules/core-modules/part-cooling/part-cooling.toml` (full — ≤ 100 lines) — `[module]` / `[stage]` / `[config.schema.<key>]` shape; note: this module uses `LayerFinalization`, NOT `GCodePostProcess`; the new module switches stage but reuses the file shape.
- `modules/core-modules/part-cooling/Cargo.toml` (full — ≤ 25 lines).
- `modules/core-modules/part-cooling/src/lib.rs` (full — 150 LOC) — `#[slicer_module]` precedent; trait differs but wiring shape is the same.
- `modules/core-modules/seam-placer/seam-placer.toml` (full — ≤ 50 lines) — string-type `[config.schema.<key>]` precedent.
- `crates/slicer-host/tests/gcode_emit_tdd.rs:1-120` — layer fixture + M82/M83 capture pattern (CRITICAL: confirms regression target).
- `crates/slicer-host/tests/gcode_header_thumbnail_config_blocks_tdd.rs` (full) — TDD scaffolding pattern.
- `crates/slicer-host/tests/postpass_gcode_emit_contract_tdd.rs:1-80` — slicer-cli invocation harness pattern.

## Out-of-Bounds Files

Files the implementer must NOT load directly. Delegate any fact-checks against this list.

- `OrcaSlicerDocumented/**` — delegate ALL parity checks. NEVER read `OrcaSlicerDocumented/src/libslic3r/PlaceholderParser.cpp` (~2400 lines of Boost.Spirit grammar).
- `target/`, `Cargo.lock`, generated WASM under `modules/core-modules/*/dist/` — never load.
- Vendored deps — never load.
- `docs/07_implementation_status.md` — > 500 lines; worker dispatch only.
- `docs/02_ir_schemas.md` — > 300 lines; range-read only at the ranges listed above.
- `crates/slicer-host/src/gcode_emit.rs` — > 1100 lines; range-read at the four ranges listed above.
- `crates/slicer-host/src/dispatch.rs` — > 3000 lines; **NOT TOUCHED in this packet** — no new match arms needed. If a sub-agent reads it for any reason, range-read only.
- `crates/slicer-host/src/wit_host.rs` — multi-thousand lines; **NOT TOUCHED in this packet** — no new `FinalizationBuilderPush` variants. If read at all, range-read only.
- `crates/slicer-ir/src/slice_ir.rs` — > 1600 lines; range-read at the listed ranges.
- `crates/slicer-sdk/src/traits.rs` — > 1200 lines; range-read only the GCodePostProcess trait section.
- All other core modules (`modules/core-modules/{arrange,monotonic-fill,…}/src/**`) beyond the enumerated reads — delegate any pattern checks.
- All other crates (`slicer-cli`, `slicer-helpers`, etc.) beyond the change surface — delegate.
- All other packets (`.ralph/specs/01*` through `58_*`) other than packet 55 — delegate.

## Expected Sub-Agent Dispatches

- **Step 1 dispatches:**
  - "In `docs/07_implementation_status.md`, find the line range where queued / in-progress G-code-output TASK entries live (proximity to TASK-184 / TASK-185 / TASK-191 / TASK-192a). Return LOCATIONS, ≤ 5 entries, each with the adjacent row's verbatim text." — insertion point.
  - "Append three rows to `docs/07_implementation_status.md` immediately after `<insertion-point>`: TASK-193, TASK-193a, TASK-193b. Match adjacent row format. Return FACT: bytes appended + line numbers." — edit.
  - "`grep -n 'TASK-193' docs/07_implementation_status.md` — return FACT (hits = 3 expected)." — verification.
- **Step 2 dispatches:**
  - "In `crates/slicer-host/tests/`, find the small STL fixture path used by `gcode_emit_tdd.rs` and `gcode_header_thumbnail_config_blocks_tdd.rs`. Return FACT: exact `concat!(env!('CARGO_MANIFEST_DIR'), '/../../resources/<filename>.stl')`." — reuse fixture.
- **Step 3 dispatches:**
  - "In `wit/deps/ir-types.wit`, does the WIT `gcode-command` variant set mirror the Rust `GCodeCommand` enum? Return FACT: yes/no + the line containing the WIT variant declaration if yes." — decide whether WIT needs editing.
  - "Run `cargo build --tests` after adding `GCodeCommand::ExtrusionMode` + emitter push + serializer arm. Return FACT pass/fail; SNIPPETS (≤ 30 lines) on fail." — validate compilation.
  - "Run `cargo test -p slicer-host --test gcode_emit_tdd`. Return FACT pass/fail; SNIPPETS (≤ 20 lines) on fail." — regression after promotion.
  - "Run `cargo test -p slicer-host --test machine_start_end_gcode_emission_tdd -- extrusion_mode_still_emitted_after_promotion --nocapture`. Return FACT pass/fail." — promotion sentry.
- **Step 4 dispatches:**
  - "In `crates/slicer-sdk/src/traits.rs`, find the GCodePostProcess module trait (likely named `GCodePostProcessModule` or similar). Return FACT: trait name + full signature + file:line." — model the impl.
  - "After writing `modules/core-modules/machine-gcode-emit/{machine-gcode-emit.toml, Cargo.toml, src/lib.rs}`, run `./modules/core-modules/build-core-modules.sh` then `--check`. Return FACT clean/stale." — guest WASM build + freshness.
  - "Run `./test-guests/build-test-guests.sh --check`. Return FACT clean/stale." — test-guest freshness.
  - "Locate the host manifest-discovery API for `[config.schema.<key>]` lookup (likely `crates/slicer-host/src/manifest.rs`). Return FACT: API name + file:line, ≤ 6 lines. If absent, return 'no direct API; use CONFIG_BLOCK fallback'." — AC implementation choice.
  - "Run `cargo test -p slicer-host --test machine_start_end_gcode_emission_tdd`. Return FACT pass/fail; SNIPPETS (≤ 20 lines) on fail." — full AC sweep.
- **Step 5 dispatches:**
  - "Run `cargo test -p slicer-host --test postpass_gcode_emit_contract_tdd`. Return FACT pass/fail." — postpass regression.
  - "Run `cargo test -p slicer-host --test gcode_header_thumbnail_config_blocks_tdd`. Return FACT pass/fail." — packet-55 regression.
  - "Run `./modules/core-modules/build-core-modules.sh --check`. Return FACT clean/stale." — guest-wasm re-verification.
  - "Run `./test-guests/build-test-guests.sh --check`. Return FACT clean/stale." — test-guest re-verification.
  - "Run `cargo check --workspace`. Return FACT pass/fail; SNIPPETS (≤ 30 lines) of first error on fail." — workspace gate.
  - "Run `cargo clippy --workspace -- -D warnings`. Return FACT pass/fail; SNIPPETS (≤ 30 lines) of first warning on fail." — lint gate.
- **Step 6 dispatches:**
  - Re-dispatch every pipe-suffixed AC command from `packet.spec.md` (13 total: 10 positive + 3 negative).
  - "Run `cargo test --workspace` once. Return FACT pass/fail; on fail SNIPPETS (≤ 40 lines) of first failing test name + assertion. NEVER return the full output (>1000 tests)." — closure ceremony.
  - "Update `docs/07_implementation_status.md` rows for TASK-193 / TASK-193a / TASK-193b to status `[x]`; return FACT: rows updated." — backlog close.

## Data and Contract Notes

- **IR contract touched:** additive only. `GCodeCommand` gains one new variant `ExtrusionMode { absolute: bool }`. All existing variants and `GCodeIR` fields are unchanged. `docs/02_ir_schemas.md` gets a single-line append for the new variant if it documents the enum. No `schema_version` bump is required — old consumers that match all existing variants will see a new variant they don't recognize ONLY if they read an IR produced by this version of the slicer; in practice the IR is produced and consumed by the same version, so no cross-version compatibility concern arises.
- **WIT boundary considerations:** `wit/world-postpass.wit` and `wit/world-finalization.wit` are unchanged. `wit/deps/ir-types.wit`'s `gcode-command` variant set MAY need a new `extrusion-mode` variant if it mirrors the Rust enum; Step 3 confirms via dispatch. If touched, the CLAUDE.md WIT/Type Changes Checklist applies. The new module declares `wit-world = "slicer:world-postpass@1.0.0"` (or whichever string the existing GCodePostProcess host uses).
- **SDK contract touched:** none. The new module consumes the existing GCodePostProcess trait surface unchanged. No SDK methods added.
- **Dispatch contract touched:** none. The existing `runner.run_gcode_postprocess(...)` loop at `crates/slicer-host/src/postpass.rs:215` already executes against `&mut gcode_ir`; the new module slots in as one more iteration of that loop.
- **Determinism or scheduler constraints:** The substituted block is deterministic given the same effective `ConfigView`. The module's `substitute_placeholders()` helper iterates the template left-to-right; HashMap key lookup is value-equality and does not introduce ordering nondeterminism. Empty / whitespace-only ⇒ zero bytes.
- **CONFIG_BLOCK propagation:** Packet 55's `serialize_config_block()` at `:928` iterates the effective `ConfigView` and emits `; <key> = <value>` per key. The four new keys flow through automatically once declared in the manifest and resolved into `effective_config`. AC `new_keys_appear_in_config_block` verifies this.
- **Multi-line `String` value in CONFIG_BLOCK:** The default `machine_start_gcode` contains `\n` characters. Packet 55's CONFIG_BLOCK formatter handles multi-line values per its existing convention. Whichever it picked applies. AC `new_keys_appear_in_config_block` asserts equality after un-escaping; a Step 4 SNIPPETS dispatch confirms the exact wire format.

## Locked Assumptions and Invariants

- The implementer MUST preserve byte-level ordering established by packets 54 / 55:
  - `; HEADER_BLOCK_START` < width comments < (NEW: resolved start string) < `M82` or `M83` line < first `;LAYER_CHANGE` < first `G1` extrusion move.
  - last `G1` extrusion move < (NEW: resolved end string) < `; THUMBNAIL_BLOCK_START` (if present) < `; CONFIG_BLOCK_START` < `; CONFIG_BLOCK_END` < EOF.
- After the promotion, exactly one `M82` or `M83` line appears in default output, in the same byte position relative to surrounding markers as before — only its origin shifts from a serializer hard-coded write to a typed command pushed by the emitter.
- The module's `substitute_placeholders()` is single-pass. Substituted values are not re-scanned.
- `substitute_placeholders()` does not panic, does not infinite-loop, and does not allocate unboundedly. For an N-byte template with K placeholders, runtime is O(N + K · avg-key-length).
- Empty/whitespace-only resolved string ⇒ the module SKIPS the corresponding `Raw` push. Output contains zero bytes for that block.
- `GCodeCommand` grows by exactly one variant (`ExtrusionMode { absolute: bool }`). All other variants unchanged.
- `GCodeIR` struct is UNCHANGED.
- `FinalizationBuilderPush` is UNCHANGED.
- `dispatch.rs` is UNCHANGED.
- `wit/world-finalization.wit` and `wit/world-postpass.wit` are UNCHANGED structurally.
- The host serializer contains NO substitution logic. All substitution happens inside the WASM guest.
- The four new config keys' defaults are EXACTLY as specified in `packet.spec.md` Goal.

## Risks and Tradeoffs

- **Risk: M82/M83 promotion breaks `gcode_emit_tdd.rs` or other suites.** The packet-54 regression suite asserts presence/position of `M82`/`M83`. After the promotion, the line appears in the same byte position by construction (the emitter pushes `ExtrusionMode` at index 0; the serializer renders it as the first per-command output, right after the header/width comments). **Mitigation:** Step 3 includes `cargo test -p slicer-host --test gcode_emit_tdd` as the immediate falsifying check. If the suite breaks, investigate the exact byte offset assertion and adjust either the test (preferred — assertion should compare position relative to markers, not absolute byte offsets) or surface as a packet-local risk.
- **Risk: cross-component diagnostics for unknown placeholders.** With substitution in the WASM guest, host-side `log::warn!` capture is not trivially available. The negative AC `unknown_placeholder_passes_through_verbatim` asserts verbatim passthrough only. Cross-component diagnostic forwarding is tracked as a separate future packet.
- **Risk: end-block position differs from OrcaSlicer.** OrcaSlicer emits `machine_end_gcode` AFTER its CONFIG_BLOCK. We emit it BEFORE `CONFIG_BLOCK` because our CONFIG_BLOCK is structurally a metadata footer that the printer ignores. **Mitigation:** Documented as an intentional deviation here and in `requirements.md`; the printer-visible ordering is correct (machine_end_gcode is the last printable command).
- **Risk: range enforcement is declarative-only.** The manifest declares `min = 0, max = 120` (bed) and `min = 0, max = 300` (nozzle) but `ResolvedConfig::apply_cli_key` does not consult these. A user passing `nozzle_temperature_initial_layer = 999` gets `M109 S999`. Tracked as a separate future packet.
- **Risk: multi-line `machine_start_gcode` value in CONFIG_BLOCK may be unparseable by some downstream tools.** Whichever convention packet 55 picked applies. **Mitigation:** Step 4 dispatch confirms wire format; AC `new_keys_appear_in_config_block` validates round-trip via un-escaping.
- **Risk: guest-wasm staleness after the new module AND after the `slicer-ir` variant change.** Per CLAUDE.md Guest WASM Staleness, `slicer-ir` is a universal guest dep — adding a variant invalidates every guest's bindgen output. **Mitigation:** Steps 4 and 5 both dispatch `--check`.
- **Risk: future "arithmetic" packet breaks one-pass invariant.** Adding `[bed*2]` evaluation means substituted output could contain identifiers that look like further placeholders. **Mitigation:** scope boundary is clear; future packet decides whether to re-scan or to extend the helper into a tokenizer.
- **Risk: `Raw` command in the middle of the stream may surprise other GCodePostProcess modules that match `GCodeCommand` exhaustively.** **Mitigation:** `Raw` is an existing variant; any module that matches `GCodeCommand` already handles `Raw`. The new module's only contribution is two more `Raw` instances at the boundaries.
- **Tradeoff: minimal substitution vs full OrcaSlicer parity.** Accepted at scope-approval gate. Users who need conditionals can wait for the follow-up packet or expand the literal value.
- **Tradeoff: typed `ExtrusionMode` variant vs `Raw("M82")` shortcut.** Chose typed variant for IR clarity, debug-print quality, and parity with how M104/M109 are modeled (`Temperature`, not `Raw`).

## Context Cost Estimate

- Aggregate (sum across all steps): **`M`**.
- Largest single step: Step 3 (`GCodeCommand` variant + emitter push + serializer arm; touches 2 source files + possibly 1 WIT file; runs regression suite) — `M`.
- Highest-risk dispatch: the `cargo test -p slicer-host --test gcode_emit_tdd` regression check in Step 3. **Required return format:** FACT pass / SNIPPETS ≤ 20 lines on fail. NEVER return the full test runner output.

## Open Questions

All open questions resolved at the scope-approval gate. **No remaining blockers for activation.** The non-blocking confirmations below resolve during Steps 3/4 dispatches:

- **Non-blocking confirmation (Step 3):** Whether `wit/deps/ir-types.wit` mirrors `GCodeCommand` and therefore needs an `extrusion-mode` variant addition. If yes, the WIT edit lands in this step. If no, the IR change is purely Rust-internal.
- **Non-blocking confirmation (Step 4):** The exact SDK trait name for GCodePostProcess modules (likely `GCodePostProcessModule`) and its method signatures. Identified via a single ranged read of `crates/slicer-sdk/src/traits.rs`.
- **Non-blocking confirmation (Step 4):** Whether the SDK exposes a `extend_from_snapshot(commands: &[GCodeCommand])` helper on the output builder, or whether the module must `push_command(cmd.clone())` per item. Either pattern is acceptable; the implementer uses whichever exists.
- **Non-blocking confirmation (Step 4):** Whether the host's manifest-discovery API exposes a test-friendly lookup of `[config.schema.<key>]` blocks. If yes, AC `module_manifest_registers_four_keys_with_expected_types_and_defaults` asserts directly on the manifest. If no, the AC falls back to CONFIG_BLOCK substring presence.

None of these change scope, interface, or verification strategy.
