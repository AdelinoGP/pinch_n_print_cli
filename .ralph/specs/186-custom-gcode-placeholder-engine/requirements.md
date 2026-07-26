# Requirements: 186-custom-gcode-placeholder-engine

## Packet Metadata

- Grouped task IDs: `TASK-305`
- Backlog source: `docs/specs/deviation-backlog-remediation-plan.md` the Packet Queue entry for `DEV-085`, tranche T3 (referenced by identity — row numbers rot), split 1 of 3; registered in `docs/07_implementation_status.md` by this packet
- Packet status: `draft`
- Aggregate context cost: `M`

## Problem Statement

`DEV-085` has two halves. This packet closes the second, user-facing one and the engine defects underneath it; packets 187 and 188 close the injection-point half.

`substitute_placeholders` in `modules/core-modules/machine-gcode-emit/src/lib.rs` carries three defects that must be fixed before any new injection point multiplies them:

1. **Mojibake.** Its literal-text path is `out.push(bytes[i] as char)`, which reinterprets each UTF-8 byte as a Latin-1 scalar. Every other write in the function goes through `push_str(std::str::from_utf8(…))`, so the corruption is confined to literal text outside brackets — a start-G-code comment containing an accented character or an emoji is silently garbled on its way to the printer.
2. **Silent unknown-placeholder passthrough.** The deciding branch is the `else` arm commented "Unknown key: pass through verbatim (including brackets)". There is no warning path, and `unwrap_or("")` swallows UTF-8 errors. Canonical does the opposite: `MyContext::legacy_variable_expansion` (`PlaceholderParser.cpp`) throws `"Variable does not exist"` for an undefined `[key]`, `GCode::placeholder_parser_process` records the failure and injects an inline marker, and `GCode::check_placeholder_parser_failed` fails the export. Packet 59 deliberately chose passthrough for its two-key subset; at three keys and rising it is no longer a defensible default, because the observable consequence is bracketed literal text arriving at a printer.
3. **A documented macro set that does not exist.** `docs/15_config_keys_reference.md` advertised twelve `[key]` macros. Substitution resolves a placeholder **iff a config key of that exact name is declared**, and `ConfigView::keys` (`crates/slicer-ir/src/slice_ir.rs`) returns the view's own `fields` map sorted, with manifest scoping applied by the host at construction rather than by `keys()`, so in the live pipeline the visible set is exactly the manifest's — so the generic `for key in config.keys()` sweep in `run_gcode_postprocess` contributes nothing beyond the four keys `machine-gcode-emit.toml` declares. Ten of the twelve shipped as literal bracketed text. The doc was corrected on 2026-07-17 to state the truth; the implementation gap is what this packet addresses.

Of those ten, **two are recoverable and eight are not**. `nozzle_diameter` is a real OrcaSlicer config key that PnP already carries (declared by `classic-perimeters` and `arachne-perimeters`); it is declared here so the macro resolves. `first_layer_temperature` is recoverable a different way: it is not a config option at all but a **parser alias**, set unconditionally by `GCode::update_placeholder_parser_with_variant_params` under the verbatim comment `// first_layer_temperature is a legacy alias of nozzle_temperature_initial_layer` — a key this module already declares — so PnP ports it as an alias entry rather than a second config key. The other **eight** are not PnP config keys under any name and have no canonical alias PnP can honour; they are recorded as a residual deviation rather than invented.

**The `DEV-085` row's own counts are wrong and must not be quoted.** Measured against canonical `PrintConfigDef::init_fff_params`: there are **16** custom-G-code options — **13 `coString`** (`file_start_gcode`, `machine_start_gcode`, `machine_end_gcode`, `before_layer_change_gcode`, `layer_change_gcode`, `time_lapse_gcode`, `wrapping_detection_gcode`, `change_filament_gcode`, `change_extrusion_role_gcode`, `process_change_extrusion_role_gcode`, `printing_by_object_gcode`, `machine_pause_gcode`, `template_custom_gcode`) plus **3 `coStrings`** per-filament vectors (`filament_start_gcode`, `filament_end_gcode`, `filament_change_extrusion_role_gcode`) — not 15, and not all scalar. `tool_change_gcode` is not an option at all (it survives only as a legacy alias rewritten to `change_filament_gcode` in `PrintConfigDef::handle_legacy`), and `per_object_gcode` does not exist in OrcaSlicer. The row also claims all the unimplemented names appear as inventory rows in `docs/ORCA_CONFIG_REFERENCE.md`; `filament_change_extrusion_role_gcode` and `process_change_extrusion_role_gcode` have zero occurrences in that file. This packet corrects the row in place.

## In Scope

- Rewrite `substitute_placeholders`' literal-text path so it copies `&str` slices of the template rather than casting bytes to `char`. The scan may stay byte-oriented — `[`, `]` and `\n` are all ASCII and UTF-8 is self-synchronising, so a `[` byte is always at a char boundary — but every emitted run must be a slice, never a per-byte cast.
- Change `substitute_placeholders`' return type so the caller learns which bracketed keys did not resolve, and have `run_gcode_postprocess` collect them across **all** templates it processes and return **one** `ModuleError::fatal(ERR_UNRESOLVED_PLACEHOLDER, …)` naming every unresolved key (sorted, deduplicated) and every offending injection point.
- Introduce the named constant **`pub const ERR_UNRESOLVED_PLACEHOLDER: u32`** for that code — `pub` because `modules/core-modules/machine-gcode-emit/tests/machine_gcode_emit_tdd.rs` is a separate crate linking the module as a library and must name the constant symbolically (AC-N1); the crate exports only `pub struct MachineGcodeEmit` today. Leave the file's other numeric `ModuleError::fatal` codes (1-11) alone.
- Preserve the unclosed-bracket rule (`[` with no `]` before end-of-line ⇒ literal remainder of the line, no failure) and the empty/whitespace-only-template rule (⇒ no emission, no failure).
- Add `[config.schema.nozzle_diameter]` to `modules/core-modules/machine-gcode-emit/machine-gcode-emit.toml`, mirroring `classic-perimeters`' block exactly (`type = "float"`, `default = 0.4`, `min = 0.1`, `max = 2.0`, `unit = "mm"`).
- Add `const PLACEHOLDER_ALIASES: &[(&str, &str)]` to `modules/core-modules/machine-gcode-emit/src/lib.rs` with the single entry `("first_layer_temperature", "nozzle_temperature_initial_layer")`, applied **after** the `config.keys()` sweep. It must **not** become a manifest key — the schema stays at five.
- Rewrite the two tests that pin the removed passthrough rule — `unknown_placeholder_passes_through_verbatim` in `modules/core-modules/machine-gcode-emit/tests/machine_gcode_emit_tdd.rs` and the same-named test in `crates/slicer-runtime/tests/integration/machine_start_end_gcode_emission_tdd.rs` — into their inverses, and add `try_slice_with_raw` to the latter so the fallible pipeline result is expressible.
- Add the new tests named by AC-1, AC-5, AC-8, AC-15, AC-N1 and AC-N2.
- Rewrite `docs/15_config_keys_reference.md` §"Machine start / end G-code" and regenerate the `module-config-keys` block; file one new residual `DEV-###` row; correct the two measured errors in the existing `DEV-085` row; register `TASK-305`.

## Out of Scope

- **Any new injection point.** `machine_start_gcode` and `machine_end_gcode` remain the only two sites. The registry that generalises them is packet 187 (`TASK-306`); the toolchange- and role-scoped points are packet 188 (`TASK-307`).
- **Computed placeholder variables** (`layer_num`, `layer_z`, `max_layer_z`, `total_layer_count`, `print_time_sec`, `num_extruders`, `print_bed_max`). These are not config keys and cannot come from `ConfigView`; they require the per-site context packet 187's registry introduces.
- **The eight unresolvable macros.** `[bed_temperature]`, `[filament_type]`, `[tool_count]`, `[layer_count]`, `[print_time_estimate_s]`, `[x_max]`, `[y_max]`, `[z_max]` are recorded as a residual deviation row with their measured canonical counterparts, not implemented and not faked. Inventing a PnP config key for any of them is explicitly forbidden. Note `[bed_temperature]` **does** exist canonically — as a placeholder variable declared in `TemperaturesConfigDef` and set in `GCode::_do_export` — but it is the steady-state bed temperature resolved through `get_bed_temp_key((BedType)curr_bed_type)` — a six-way dispatch over `supertack_plate_temp` / `cool_plate_temp` / `textured_cool_plate_temp` / `eng_plate_temp` / `hot_plate_temp` / `textured_plate_temp`, all `coInts`, of which `hot_plate_temp` is only the `btPEI` branch — and **not** an alias of the declared `bed_temperature_initial_layer_single`. PnP models no bed-type dispatch at all, so aliasing it would silently substitute the wrong number. `[first_layer_temperature]` is **not** in this list; it is adopted as an alias (see §In Scope).
- **A `{…}` expression syntax.** Canonical's `PlaceholderParser` supports arithmetic, conditionals and builtins in `{}`; PnP scans only `[…]` and leaves `{}` untouched. That remains a permanent, separately-tracked divergence and is not narrowed here.
- **An escape syntax for literal square brackets.** Canonical has none for the legacy `[key]` form either. A template that needs a literal `[foo]` where `foo` is not a declared key will now fail the slice; that consequence is documented in `docs/15` and in the residual row rather than worked around.
- **`docs/ORCA_CONFIG_REFERENCE.md`** — no edit (see `packet.spec.md` §Doc Impact Statement).
- **The host-side G-code serializer and emitter** (`crates/slicer-gcode/`). Nothing in this packet changes what the host emits.

## Authoritative Docs

- `docs/15_config_keys_reference.md` — long; ranged reads only (§"Machine start / end G-code" and the `module-config-keys` marker boundaries). Delegate anything wider.
- `docs/DEVIATION_LOG.md` — long; delegate. Read only the `DEV-085` row and re-derive the highest `DEV-###`.
- `docs/07_implementation_status.md` — always delegate.
- `docs/02_ir_schemas.md` — delegated SUMMARY only, for the `PostPass::GCodePostProcess` input surface.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/PlaceholderParser.cpp` — `MyContext::legacy_variable_expansion` and `MyContext::throw_exception`, for the fact that canonical's `[key]` legacy bracket form **errors** on an undefined variable (`"Variable does not exist"`) rather than passing it through, and `MyContext::process_error_message` for the message shape. Borrowed as the justification for this packet's failure policy.
- `OrcaSlicerDocumented/src/libslic3r/GCode.cpp` — `GCode::placeholder_parser_process` and `GCode::check_placeholder_parser_failed`, for the deferral shape: each failure is recorded into `PlaceholderParserIntegration::failed_templates` and the throw happens once, later, over all of them. Borrowed for AC-5's collect-all-then-fail-once requirement.
- `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` — `PrintConfigDef::init_fff_params`, for the true registration count (13 `coString` + 3 `coStrings`) that corrects the `DEV-085` row. **Four canonical placeholder names this packet cites live in three OTHER structures, and attributing them to the custom-gcode table sends a worker to an empty result:** `total_layer_count` and `print_time_sec` are in `PrintStatisticsConfigDef` (coInt / coString), `num_extruders` is in `OtherSlicingStatesConfigDef` (coInt), and `print_bed_max` is in `DimensionsConfigDef` (coFloats). None of the four is in `s_CustomGcodeSpecificPlaceholders`, and none is in `CustomGcodeSpecificConfigDef` either. Only `max_layer_z` (and `layer_num`) genuinely live in both custom-gcode structures. Dispatch against the named struct, not the table, for the residual row maps the eight residual macros onto.
- `OrcaSlicerDocumented/src/libslic3r/GCode.cpp` — `GCode::update_placeholder_parser_with_variant_params`, for the **unconditional** `placeholder_parser().set("first_layer_temperature", …)` and its verbatim comment `// first_layer_temperature is a legacy alias of nozzle_temperature_initial_layer`. This is AC-15's authority. **An earlier draft of this packet claimed the name was set *only* on `GCode::set_extruder`'s `toolchange_temp_override` path; that is refuted.** Re-counted by execution: the name occurs **nine** times in `GCode.cpp` — two unconditional `placeholder_parser().set` calls (this one, using `remap_ints_by_filament(m_config.nozzle_temperature_initial_layer)`, plus one in `GCode::_do_export` using `new ConfigOptionInts(m_config.nozzle_temperature_initial_layer)`); two `toolchange_temp_override > 0`-gated `set_key_value`s in `GCode::set_extruder` (one on the `change_filament_gcode` config, one on the `filament_start_gcode` config); one `set_key_value` on a local `DynamicConfig` in **`WipeTowerIntegration::append_tcr`**, gated `full_config.enable_tower_interface_features && tcr.is_contact`; two `; first_layer_temperature = %d` CONFIG_BLOCK emissions in `GCode::_do_export`; and two source comments. **The `tcr.is_contact` set is in `WipeTowerIntegration::append_tcr`, not `get_path_of_change_filament`** — an earlier draft named the latter, which contains no such set, and a worker dispatched there returns empty. Note also that `append_tcr`'s is a `set_key_value` on a local config, not a `placeholder_parser().set`. **Deliberately NOT borrowed:** the surrounding toolchange emission, which is packet 188's subject.

## Acceptance Summary

Reference, never copy, criteria from `packet.spec.md`.

- Positive: `AC-1` through `AC-15`. Change-proving: `AC-1`, `AC-2`, `AC-4`, `AC-5`, `AC-7`, `AC-8`, `AC-11`, `AC-12` (row clause), `AC-13`, `AC-14`, `AC-15`. Explicit do-not-regress guards: `AC-3`, `AC-6`, `AC-9`, `AC-10`, and `AC-12`'s `gen-config-docs --check` half.
- Negative: `AC-N1` (module-level fatal on an unknown key, plus proof the old passthrough test is gone), `AC-N2` (the same end to end through `run_pipeline_with_raw_config`), `AC-N3` (do-not-regress: an empty template is not a failed template).
- Cross-packet impact: `ERR_UNRESOLVED_PLACEHOLDER` and the `(String, Vec<String>)` shape of `substitute_placeholders` are consumed by packets 187 and 188, which route additional templates through the same collector. Packet 187's registry must extend, not bypass, the collect-all-then-fail-once rule.

## Verification Commands

This is the authoritative full matrix; `packet.spec.md` lists only the three gate commands.

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo check --workspace --all-targets` | test/bench targets still compile after the `substitute_placeholders` signature change | FACT pass/fail |
| `cargo clippy --workspace --all-targets -- -D warnings` | closure gate | FACT pass/fail |
| `cargo xtask build-guests --check` | mandatory after editing `modules/core-modules/machine-gcode-emit/src/**` and its `.toml`; rebuild without `--check` if `STALE:` | FACT clean/stale list |
| `bash -c 'cargo test -p machine-gcode-emit --test machine_gcode_emit_tdd 2>&1 \| tee target/log-186-mge.txt \| rg "^test result:" \| rg -q "^test result: ok\. [1-9]" && echo PASS \|\| echo "FAIL: see target/log-186-mge.txt"'` | whole module-test binary green (AC-9) | FACT PASS/FAIL; SNIPPETS ≤20 lines on failure |
| `bash -c 'cargo test -p slicer-runtime --test integration -- machine_start_end_gcode_emission_tdd:: 2>&1 \| tee target/log-186-msege.txt \| rg "^test result:" \| rg -q "^test result: ok\. [1-9]" && echo PASS \|\| echo "FAIL: see target/log-186-msege.txt"'` | whole e2e module green (AC-10) | FACT PASS/FAIL |
| Each individual `--exact` command in `packet.spec.md` AC-1, AC-3, AC-5, AC-6, AC-8, AC-15, AC-N1, AC-N2, AC-N3 | per-criterion proof | FACT PASS/FAIL |
| Each `python3` / `rg` probe in `packet.spec.md` AC-2, AC-4, AC-7, AC-11, AC-12, AC-13, AC-14 and §Doc Impact | static and doc proof | FACT PASS/FAIL |
| `cargo xtask gen-config-docs --check` | generated `module-config-keys` block is in sync after the manifest change | FACT exit code |

## Step Completion Expectations

- The `substitute_placeholders` signature change and both test-file rewrites must land in the **same** step. Changing the return type without rewriting `unknown_placeholder_passes_through_verbatim` leaves `machine_gcode_emit_tdd` red for a reason the packet intends, which is indistinguishable from a real regression at the next backpressure gate.
- `cargo xtask build-guests --check` must be run (and a rebuild performed if it reports `STALE:`) **before** any `--test integration` command is attributed to this packet's changes. The `integration` binary instantiates the real `machine-gcode-emit.wasm` component; a stale guest fails typed instantiation and looks like a code bug.
- `cargo xtask gen-config-docs` must run **after** the manifest edit and **before** the AC-11/AC-12 doc probes, otherwise the generated block and the hand-written prose disagree and `--check` is red.
- The residual `DEV-###` ID is a ledger fact. Re-derive it at the moment of writing; do not carry a number forward from planning notes.

## Context Discipline Notes

- `crates/slicer-runtime/tests/integration/machine_start_end_gcode_emission_tdd.rs` is long. Read only `slice_with_raw` (to add `try_slice_with_raw` beside it) and the two tests being rewritten; do not load the whole file.
- `docs/15_config_keys_reference.md` and `docs/DEVIATION_LOG.md` are both long and must be range-read or delegated.
- Never open `crates/slicer-ir/src/slice_ir.rs` in full to confirm `ConfigView::keys`; its behaviour ("returns all manifest-declared keys, sorted") is stated here and is the only fact needed.
- Do not read `OrcaSlicerDocumented/` directly under any circumstance; the four facts this packet borrows are enumerated above and each is delegable in one dispatch.
