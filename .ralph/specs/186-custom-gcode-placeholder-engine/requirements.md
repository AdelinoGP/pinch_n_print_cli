# Requirements: 186-custom-gcode-placeholder-engine

## Packet Metadata

- Grouped task IDs: `TASK-305`
- Backlog source: `docs/specs/deviation-backlog-remediation-plan.md` the Packet Queue entry for `DEV-085`, tranche T3 (referenced by identity — row numbers rot), split 1 of 3; registered in `docs/07_implementation_status.md` by this packet
- Packet status: `implemented` after the warn-and-pass migration, ADR alignment, and full closure ceremony
- Aggregate context cost: `M`

## Problem Statement

`DEV-085` has two halves. This packet closes the second, user-facing one and the
engine defects underneath it; packets 187 and 188 close the injection-point half.

`substitute_placeholders` and its caller in
`modules/core-modules/machine-gcode-emit/src/lib.rs` carry four defects that must
be fixed before any new injection point multiplies them:

1. **Mojibake.** Its literal-text path was `out.push(bytes[i] as char)`, which
   reinterprets each UTF-8 byte as a Latin-1 scalar. Every other write in the
   function went through `push_str(std::str::from_utf8(…))`, so the corruption
   was confined to literal text outside brackets — a start-G-code comment
   containing an accented character or an emoji was silently garbled on its way
   to the printer.

2. **Unknown-placeholder passthrough was silent — not wrong, but undiagnosable.**
   The deciding branch is the `else` arm on the bracketed-key path. Passthrough
   *itself* is correct and stays: a module's `ConfigView` is scoped to its own
   manifest, so *"unknown to `machine-gcode-emit`"* is not *"unknown to the
   slicer"*. A template may legitimately name a key owned by a module that is not
   loaded in this pipeline, or by no module at all. **What was missing was any
   signal at all**, so a user whose start block reached the printer as literal
   `[foo]` had nothing to look at. The fix is one aggregated
   `slicer_sdk::host::log_warn` naming every unresolved key (sorted, deduplicated
   through a `BTreeSet`) and every injection point that contributed one. AC-5
   proves this behavior through `install_log_capture()` and
   `take_log_messages()`, including a duplicate key shared by start and end.

   **This item previously argued the opposite and that argument is retracted.**
   A prior revision of this packet cited canonical — `MyContext::legacy_variable_expansion`
   throws `"Variable does not exist"`, `GCode::placeholder_parser_process` records
   the failure, `GCode::check_placeholder_parser_failed` fails the export — and
   concluded that PnP should fail the slice too. That policy was implemented,
   passed all 18 of its acceptance criteria, and was then found by adversarial
   review to break three e2e tests across two real OrcaSlicer-authored 3MF
   fixtures. The repo owner rejected it outright: *"Unresolved keys cannot be a
   fatal slice error, as PnP is modular and MUST accept keys from modules that
   aren't loaded."* The reasoning error was importing a policy from a monolith
   with a single global config into a composable module system whose whole point
   is that no component sees the whole configuration. Canonical remains the
   citation for the *aggregation* shape (collect across all templates, act once);
   it is explicitly **not** the citation for the action taken.

3. **A documented macro set that did not exist.** `docs/15_config_keys_reference.md`
   advertised twelve `[key]` macros. Substitution resolves a placeholder **iff a
   config key of that exact name is declared**, and `ConfigView::keys`
   (`crates/slicer-ir/src/slice_ir.rs`) returns the view's own `fields` map
   sorted, with manifest scoping applied by the host at construction rather than
   by `keys()`, so in the live pipeline the visible set is exactly the manifest's
   — the generic `for key in config.keys()` sweep in `run_gcode_postprocess`
    contributed nothing beyond the keys `machine-gcode-emit.toml` declared.
   Ten of the twelve shipped as literal bracketed text. The doc was corrected on
   2026-07-17 to state the truth; the implementation gap is what this packet
   addresses.

4. **List-valued config keys were dropped on the floor.** `format_placeholder_value`'s
   catch-all arm discarded `ConfigValue::List`. Real 3MF input supplies
   per-extruder settings as vectors — `nozzle_diameter` reaches the module as
   `['0.4']`, never as a scalar — so the packet's own headline macro
   `[nozzle_diameter]` was **inert for every real slice** while the
   scalar-schema-default test stayed green. This was adversarial-review finding
   F2 and it is fixed here: a `List` resolves from its **first element**,
   recursively; an **empty** list yields no lookup entry at all, so the
   placeholder stays unresolved rather than collapsing to an empty string.

Of the ten undelivered macros, **two are recoverable and eight are not**.
`nozzle_diameter` is a real OrcaSlicer config key that PnP already carries
(declared by `classic-perimeters` and `arachne-perimeters`); it is declared here
so the macro resolves — and only actually *works* because of defect 4's fix.
`first_layer_temperature` is recoverable a different way: it is not a config
option at all but a **parser alias**, set unconditionally by
`GCode::update_placeholder_parser_with_variant_params` under the verbatim comment
`// first_layer_temperature is a legacy alias of nozzle_temperature_initial_layer`
— a key this module already declares — so PnP ports it as an alias entry rather
than a second config key. The other **eight** are not PnP config keys under any
name and have no canonical alias PnP can honour; they are recorded as a residual
deviation rather than invented.

**The `DEV-085` row's own counts are wrong and must not be quoted.** Measured
against canonical `PrintConfigDef::init_fff_params`: there are **16**
custom-G-code options — **13 `coString`** (`file_start_gcode`,
`machine_start_gcode`, `machine_end_gcode`, `before_layer_change_gcode`,
`layer_change_gcode`, `time_lapse_gcode`, `wrapping_detection_gcode`,
`change_filament_gcode`, `change_extrusion_role_gcode`,
`process_change_extrusion_role_gcode`, `printing_by_object_gcode`,
`machine_pause_gcode`, `template_custom_gcode`) plus **3 `coStrings`**
per-filament vectors (`filament_start_gcode`, `filament_end_gcode`,
`filament_change_extrusion_role_gcode`) — not 15, and not all scalar.
`tool_change_gcode` is not an option at all (it survives only as a legacy alias
rewritten to `change_filament_gcode` in `PrintConfigDef::handle_legacy`), and
`per_object_gcode` does not exist in OrcaSlicer. The row also claimed all the
unimplemented names appear as inventory rows in `docs/ORCA_CONFIG_REFERENCE.md`;
`filament_change_extrusion_role_gcode` and `process_change_extrusion_role_gcode`
have zero occurrences in that file. This packet corrects the row in place.

## The authoring defect this re-authoring exists to fix

**Not one of the superseded 18 acceptance criteria exercised a real-world
template.** Every criterion drove the module through `slicer_sdk::test_prelude::config_with`
pairs or a hand-built `HashMap<ConfigKey, ConfigValue>` raw config — i.e. through
a placeholder domain the criterion's own author had chosen. That is why a
user-visible, irreversible behaviour reversal could pass a full 18/18 ceremony
while breaking real slicing, and it is the same shape of blindness that made
`[nozzle_diameter]` inert (finding F2): a synthetic `Float` where real input
delivers a `List`.

Two structural remedies are required of this packet and are not optional
polish:

- **AC-18** slices two committed OrcaSlicer-authored 3MF fixtures end to end and
  asserts success. Measured in this checkout, both carry templates naming
  `[first_layer_bed_temperature]`, `[initial_tool]`, `[max_layer_z]` and
  `[layer_z]` — none a `machine-gcode-emit` manifest key or alias — alongside
  `[first_layer_temperature]` and `[nozzle_diameter]`.
- **AC-16 / AC-17** pin the `List` and empty-`List` shapes that real config
  delivery actually uses.

Any future packet touching this engine inherits the rule: **a criterion that only
ever sees author-chosen config is not evidence about user-visible behaviour.**

## In Scope

- Rewrite `substitute_placeholders`' literal-text path so it copies `&str` slices
  of the template rather than casting bytes to `char`. The scan may stay
  byte-oriented — `[`, `]` and `\n` are all ASCII and UTF-8 is self-synchronising,
  so a `[` byte is always at a char boundary — but every emitted run must be a
  slice, never a per-byte cast.
- Keep `substitute_placeholders`' `(String, Vec<String>)` return shape: the
  rendered text plus the sorted, deduplicated list of bracketed keys that had no
  entry in `lookup`. The rendered text retains the verbatim `[key]` for an
  unresolved key. The list exists so the caller can **warn** about them once —
  not so it can fail.
- Have `run_gcode_postprocess` union both templates' unresolved lists into a
  `BTreeSet<String>` and emit exactly **one** `slicer_sdk::host::log_warn` naming
  every unresolved key in sorted order and every injection point that contributed
  one, then **proceed with emission normally** and return `Ok`.
- Extend `format_placeholder_value` so `ConfigValue::List` resolves from its
  **first element** (recursively), and an **empty** list yields `None` so the key
  stays out of the lookup. `Percent` and `FloatOrPercent` stay unrendered: they
  are meaningless without the base they resolve against.
- Preserve the unclosed-bracket rule (`[` with no `]` before end-of-line ⇒
  literal remainder of the line, not even a candidate placeholder) and the
  empty/whitespace-only-template rule (⇒ no emission, no warning).
- Add `[config.schema.nozzle_diameter]` to
  `modules/core-modules/machine-gcode-emit/machine-gcode-emit.toml`, mirroring
  `classic-perimeters`' block exactly (`type = "float"`, `default = 0.4`,
  `min = 0.1`, `max = 2.0`, `unit = "mm"`).
- Keep `[config.schema]` exactly equal to the five-key set asserted by AC-7;
  `nozzle_diameter`'s fields and its end-to-end macro resolution are required,
  while `first_layer_temperature` remains an alias rather than a sixth key.
- Add `const PLACEHOLDER_ALIASES: &[(&str, &str)]` to
  `modules/core-modules/machine-gcode-emit/src/lib.rs` with the single entry
  `("first_layer_temperature", "nozzle_temperature_initial_layer")`, applied
  **after** the `config.keys()` sweep. It must **not** become a manifest key —
  the schema stays at five.
- Keep the module-level and pipeline-level passthrough tests
  (`unknown_placeholder_passes_through_verbatim` in both
  `modules/core-modules/machine-gcode-emit/tests/machine_gcode_emit_tdd.rs` and
  `crates/slicer-runtime/tests/integration/machine_start_end_gcode_emission_tdd.rs`)
  asserting passthrough, and keep `try_slice_with_raw(raw) -> Result<String, PipelineError>`
  in the latter beside `slice_with_raw` — packets 187 and 188 both depend on that
  helper existing.
- Add the tests named by AC-1, AC-5, AC-8, AC-15, AC-16 and AC-17, and the
  fixture-level e2e gate of AC-18.
- Rewrite `docs/15_config_keys_reference.md` §"Machine start / end G-code" to the
  warn-and-pass contract and the domain rule; regenerate the `module-config-keys`
  block; file one residual `DEV-###` row; correct the two measured errors in the
  existing `DEV-085` row; register `TASK-305`.

## Out of Scope

- **Changing unknown-key passthrough. FORBIDDEN.** An unresolved `[key]` is
  emitted verbatim and the slice proceeds. Not a fatal `ModuleError`, not an
  empty-string substitution, not a `strict_placeholders` opt-out, not a
  `Diagnostic` that a strict mode could promote. **The reason is modularity:** a
  module's `ConfigView` is scoped to its own manifest, so this module cannot
  distinguish "no such key anywhere" from "a key owned by a module that is not
  loaded in this pipeline". Failing on the second case breaks composition, which
  is the property PnP's module system exists to provide. This was measured, not
  argued: the fatal policy broke three e2e tests across
  `resources/cube_cilindrical_modifier.3mf` and `resources/cube_4color.3mf`,
  whose OrcaSlicer-authored templates reference `[first_layer_bed_temperature]`,
  `[initial_tool]` and `[max_layer_z]`. Reinstating any variant of it requires a
   repo-owner decision and a new packet, not this packet's implementation.
- **Any new injection point.** `machine_start_gcode` and `machine_end_gcode`
  remain the only two sites. The registry that generalises them is packet 187
  (`TASK-306`); the toolchange- and role-scoped points are packet 188
  (`TASK-307`).
- **Computed placeholder variables** (`layer_num`, `layer_z`, `max_layer_z`,
  `total_layer_count`, `print_time_sec`, `num_extruders`, `print_bed_max`). These
  are not config keys and cannot come from `ConfigView`; they require the
  per-site context packet 187's registry introduces. Note that the two 3MF
  fixtures of AC-18 already reference `[layer_z]` and `[max_layer_z]`, so 187 has
  live input to validate against from day one.
- **The eight unresolvable macros.** `[bed_temperature]`, `[filament_type]`,
  `[tool_count]`, `[layer_count]`, `[print_time_estimate_s]`, `[x_max]`,
  `[y_max]`, `[z_max]` are recorded as a residual deviation row with their
  measured canonical counterparts, not implemented and not faked. Inventing a PnP
  config key for any of them is explicitly forbidden. Note `[bed_temperature]`
  **does** exist canonically — as a placeholder variable declared `coInts` in
  `TemperaturesConfigDef` and set in `GCode::_do_export` — but it is the
  steady-state bed temperature resolved through
  `get_bed_temp_key((BedType)curr_bed_type)`, a six-way dispatch over
  `supertack_plate_temp` / `cool_plate_temp` / `textured_cool_plate_temp` /
  `eng_plate_temp` / `hot_plate_temp` / `textured_plate_temp` — and **not** an
  alias of the declared `bed_temperature_initial_layer_single`. PnP models no
  bed-type dispatch at all, so aliasing it would silently substitute the wrong
  number for five of the six bed types. `[first_layer_temperature]` is **not** in
  this list; it is adopted as an alias (see §In Scope).
- **A `{…}` expression syntax.** Canonical's `PlaceholderParser` supports
  arithmetic, conditionals and builtins in `{}`; PnP scans only `[…]` and leaves
  `{}` untouched. That remains a permanent, separately-tracked divergence and is
  not narrowed here.
- **An escape syntax for literal square brackets.** None is needed under
  warn-and-pass: a bracketed word that is not a declared key or alias is left
  alone and passed through unchanged. (Under the reverted fatal policy this was a
  real gap; it is not one now.) Canonical has no escape for the legacy `[key]`
  form either.
- **`docs/adr/0050-custom-gcode-architecture.md`** — rewritten in place by this
  closure work to align its title and unknown-placeholder policy with
  warn-and-pass. See `design.md` §Decisions of Record.
- **`docs/ORCA_CONFIG_REFERENCE.md`** — no edit (see `packet.spec.md`
  §Doc Impact Statement).
- **The host-side G-code serializer and emitter** (`crates/slicer-gcode/`).
  Nothing in this packet changes what the host emits.

## ADR Relationship

- `docs/adr/0050-custom-gcode-architecture.md` is the current aligned authority:
  unresolved placeholders warn and pass, the domain is one module manifest plus
  aliases, and the engine remains module-private. This packet records and
  verifies the in-place rewrite and no longer treats the ADR as a blocker.
- `docs/adr/0055-fuel-based-module-profiling.md` is unrelated to this packet and
  remains untouched; its fuel/profiling contract must not be folded into the
  placeholder scope.
- Packets 187 and 188 inherit the same ADR-0050 relationship and migrate the
  five-key integration contract before adding their own manifest keys.

## Authoritative Docs

- `docs/15_config_keys_reference.md` — long; ranged reads only (§"Machine start / end G-code" and the `module-config-keys` marker boundaries). Delegate anything wider.
- `docs/DEVIATION_LOG.md` — long; delegate. Read only the `DEV-085` row and the residual row, and re-derive the highest `DEV-###`.
- `docs/07_implementation_status.md` — always delegate.
- `docs/adr/0050-custom-gcode-architecture.md` — read-only and aligned on the
  unknown-key policy; use its warn-and-pass decision as the authority.
- `docs/02_ir_schemas.md` — delegated SUMMARY only, for the `PostPass::GCodePostProcess` input surface.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file + function + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines. **Cite canonical by function name, never by line number.**

**Checkout hazard, measured: this repo's `OrcaSlicerDocumented/src/libslic3r/` has NO `GCode.cpp`** — only `GCodeReader`, `GCodeSender` and `GCodeWriter`. Every `GCode.cpp` fact below was verified against a full sibling checkout at `F:\slicerProject\pinch_n_print_cli_2\OrcaSlicerDocumented`. A dispatch that asks the in-repo mirror for `GCode::placeholder_parser_process` returns empty; an agent reading that emptiness as "the function does not exist" concludes the opposite of the truth.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/PlaceholderParser.cpp` — `MyContext::legacy_variable_expansion` and `MyContext::throw_exception`, for the fact that canonical's `[key]` legacy bracket form **errors** on an undefined variable (`"Variable does not exist"`). Borrowed **only as the contrast** this packet documents as a deliberate divergence — no longer as justification for a failure policy.
- `src/libslic3r/GCode.cpp` (**sibling checkout only**) — `GCode::placeholder_parser_process` and `GCode::check_placeholder_parser_failed`, for the deferral shape: each failure is recorded into `PlaceholderParserIntegration::failed_templates` (dedupe by template name comes from the insert-if-absent guard in `placeholder_parser_process`, not from the checker) and the throw happens once, later, over all of them. Borrowed for the collect-all-then-**warn**-once aggregation, which is the half PnP adopts.
- `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` — `PrintConfigDef::init_fff_params`, for the true registration count (13 `coString` + 3 `coStrings`) that corrects the `DEV-085` row. **Four canonical placeholder names this packet cites live in three OTHER structures, and attributing them to the custom-gcode table sends a worker to an empty result:** `total_layer_count` and `print_time_sec` are in `PrintStatisticsConfigDef` (coInt / coString), `num_extruders` is in `OtherSlicingStatesConfigDef` (coInt), and `print_bed_max` is in `DimensionsConfigDef` (coFloats). None of the four is in `s_CustomGcodeSpecificPlaceholders`, and none is in `CustomGcodeSpecificConfigDef` either. Only `max_layer_z` (and `layer_num`) genuinely live in both custom-gcode structures.
- `src/libslic3r/GCode.cpp` (**sibling checkout only**) — `GCode::update_placeholder_parser_with_variant_params`, for the **unconditional** `placeholder_parser().set("first_layer_temperature", …)` and its verbatim comment `// first_layer_temperature is a legacy alias of nozzle_temperature_initial_layer`. This is AC-15's authority. **An earlier draft claimed the name was set *only* on `GCode::set_extruder`'s `toolchange_temp_override` path; that is refuted** — the name occurs nine times in that file, and the `tcr.is_contact`-gated `set_key_value` is in `WipeTowerIntegration::append_tcr`, not `get_path_of_change_filament`.
- `src/libslic3r/GCode.cpp` (**sibling checkout only**) — `GCode::_do_export`, for `nozzle_temperature_initial_layer.get_at(0)` in the `; first_layer_temperature = %d` preamble: canonical also reads **element 0** of a per-extruder vector where a placeholder needs one value. This is AC-16's canonical anchor.

## Acceptance Summary

Reference, never copy, criteria from `packet.spec.md`.

- Positive: `AC-1` through `AC-18`.
  - Change-proving: `AC-1`, `AC-2`, `AC-4`, `AC-5`, `AC-7`, `AC-8`, `AC-11`,
    `AC-12` (row clause), `AC-13`, `AC-14`, `AC-15`, `AC-16`, `AC-17`.
  - **New in this re-authoring:** `AC-16` (list-valued config resolves from
    element 0), `AC-17` (an empty list does not substitute an empty string),
    `AC-18` (two real OrcaSlicer-authored 3MF fixtures slice **successfully**,
    end to end).
  - Explicit do-not-regress guards: `AC-3`, `AC-6`, `AC-9`, `AC-10`, and
    `AC-12`'s `gen-config-docs --check` half.
- Negative: `AC-N1` (module-level: an unknown key passes through verbatim and the
  call returns `Ok`), `AC-N2` (the same through the real WASM component and the
  full pipeline), `AC-N3` (behavioral do-not-regress: an empty or whitespace-only
  template emits nothing and native log capture contains zero warnings).
  - `AC-N1` and `AC-N2` are the **exact inverses** of the criteria they replace.
    Both additionally assert that the reverted fatal tests
    (`unknown_placeholder_is_a_fatal_module_error`,
    `unknown_placeholder_is_a_fatal_slice_error`) are **absent**, so the policy
    cannot be reinstated alongside the passthrough tests.
- **`AC-18` is a packet-level gate, not merely a criterion.** Its absence is what
  let a rejected policy pass a full acceptance ceremony.
- Cross-packet impact: `try_slice_with_raw` and the `(String, Vec<String>)` shape
  of `substitute_placeholders` are consumed by packets 187 and 188, which route
  additional templates through the same collector. **Packet 187 must extend, not
  bypass, the collect-all-then-warn-once rule, and must not reintroduce a failure
  path.** 187 should also add its own fixture-level e2e criterion in AC-18's
  shape.

## Verification Commands

This is the authoritative full matrix; `packet.spec.md` §Verification lists only
the gate commands.

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo check --workspace --all-targets` | test/bench targets still compile | FACT pass/fail |
| `cargo clippy --workspace --all-targets -- -D warnings` | closure gate | FACT pass/fail |
| `cargo xtask build-guests --check` | mandatory after editing `modules/core-modules/machine-gcode-emit/src/**` and its `.toml`; rebuild without `--check` if `STALE:` | FACT clean/stale list |
| `cargo build -p pnp-cli` | **mandatory before any `--test e2e` run** — `slicer_test_support::pnp_cli_bin`'s `staleness_reason` guard panics on an absent or stale binary and has no cross-profile fallback | FACT pass/fail |
| `bash -c 'cargo test -p machine-gcode-emit --test machine_gcode_emit_tdd 2>&1 \| tee target/log-186-mge.txt \| rg "^test result:" \| rg -q "^test result: ok\. [1-9]" && echo PASS \|\| echo "FAIL: see target/log-186-mge.txt"'` | whole module-test binary green (AC-9) | FACT PASS/FAIL; SNIPPETS ≤20 lines on failure |
| `bash -c 'cargo test -p slicer-runtime --test integration -- machine_start_end_gcode_emission_tdd:: 2>&1 \| tee target/log-186-msege.txt \| rg "^test result:" \| rg -q "^test result: ok\. [1-9]" && echo PASS \|\| echo "FAIL: see target/log-186-msege.txt"'` | whole pipeline-level module green (AC-10) | FACT PASS/FAIL |
| The **AC-18** command in `packet.spec.md` (build `pnp-cli`, then `--test e2e -- modifier_infill_tdd::` and `-- cube_painted_e2e_tdd::`) | **real-3MF anti-regression gate** | FACT PASS/FAIL |
| Each individual `--exact` command in `packet.spec.md` AC-1, AC-3, AC-5, AC-6, AC-8, AC-15, AC-16, AC-17, AC-N1, AC-N2, AC-N3 | per-criterion proof | FACT PASS/FAIL |
| Each `python3` / `rg` probe in `packet.spec.md` AC-2, AC-4, AC-7, AC-11, AC-12, AC-13, AC-14 and §Doc Impact | static and doc proof | FACT PASS/FAIL |
| `cargo xtask gen-config-docs --check` | generated `module-config-keys` block in sync after the manifest change | FACT exit code |

## Step Completion Expectations

- The `substitute_placeholders` signature change and both test-file updates must
  land in the **same** step. Changing the return type without updating the tests
  leaves two binaries red for a reason the packet intends, which is
  indistinguishable from a real regression at the next backpressure gate.
- **`cargo xtask build-guests --check` must be run (and a rebuild performed if it
  reports `STALE:`) before any `--test integration` or `--test e2e` command is
  attributed to this packet's changes.** Both binaries instantiate the real
  `machine-gcode-emit.wasm` component; a stale guest fails typed instantiation
  and looks like a code bug.
- **`cargo build -p pnp-cli` must precede every `--test e2e` command.** Measured
  during re-authoring: the first e2e attempt failed on
  `slicer_test_support`'s `pnp_cli` staleness guard, not on an assertion. That
  failure mode reads as an e2e regression and is not one.
- `cargo xtask gen-config-docs` must run **after** the manifest edit and
  **before** the AC-11/AC-12 doc probes, otherwise the generated block and the
  hand-written prose disagree and `--check` is red.
- The residual `DEV-###` ID is a ledger fact. Re-derive it at the moment of
  writing; do not carry a number forward from planning notes or from this packet.
- **No step may reintroduce a failure path for an unresolved placeholder.** Every
  step must preserve the aligned warn-and-pass decision in
  `docs/adr/0050-custom-gcode-architecture.md` §1.

## Context Discipline Notes

- `crates/slicer-runtime/tests/integration/machine_start_end_gcode_emission_tdd.rs`
  is long. Read only `slice_with_raw` / `try_slice_with_raw` and the two
  placeholder tests; do not load the whole file.
- `docs/15_config_keys_reference.md` and `docs/DEVIATION_LOG.md` are both long
  and must be range-read or delegated. The residual and `DEV-085` rows are each a
  single very long line.
- Never open `crates/slicer-ir/src/slice_ir.rs` in full to confirm
  `ConfigView::keys`; its behaviour ("returns the view's own `fields` map,
  sorted; manifest scoping is applied by the host at construction") is stated
  here and is the only fact needed.
- Do not read `OrcaSlicerDocumented/` directly under any circumstance; the facts
  this packet borrows are enumerated above and each is delegable in one dispatch.
  Remember the missing-`GCode.cpp` hazard when writing the dispatch.
