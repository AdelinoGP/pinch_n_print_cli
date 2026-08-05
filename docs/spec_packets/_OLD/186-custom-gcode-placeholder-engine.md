---
status: implemented
packet: 186-custom-gcode-placeholder-engine
task_ids:
  - TASK-305
---

# 186-custom-gcode-placeholder-engine

## Goal

Make the `[key]` placeholder engine in
`modules/core-modules/machine-gcode-emit/src/lib.rs` correct and honest before
any new injection point is added:

1. Replace `substitute_placeholders`' `out.push(bytes[i] as char)` literal-text
   path with a char-boundary-correct `&str` slice copy, so non-ASCII template
   text stops arriving at the printer as mojibake.
2. Resolve `ConfigValue::List` from its **first element** in
   `format_placeholder_value`, because real 3MF input supplies per-extruder
   settings as vectors — `nozzle_diameter` arrives as `['0.4']`, never as a
   scalar — and without this the packet's own headline macro is inert for every
   real slice.
3. Declare `nozzle_diameter` in the module manifest so the last config-valued
   macro `docs/15_config_keys_reference.md` ever advertised actually resolves.
4. Adopt `first_layer_temperature` as a **placeholder alias** of the
   already-declared `nozzle_temperature_initial_layer`, as canonical does.
5. Add an **aggregated warning** for unresolved keys: one
   `slicer_sdk::host::log_warn` naming every unresolved key (sorted,
   deduplicated) and every injection point that contributed one.
6. Rewrite `docs/15_config_keys_reference.md` §"Machine start / end G-code" to
   state the resolvable-domain rule and the warn-and-pass policy, and file the
   residual deviation row.

**This packet does NOT change unknown-key passthrough, and that is a decision,
not an omission.** An unresolved `[key]` is emitted verbatim (brackets included)
and the slice proceeds. The reason is architectural: a module's `ConfigView` is
scoped to **its own manifest**, so *"unknown to `machine-gcode-emit`"* is not
*"unknown to the slicer"*. A template may legitimately name a key owned by a
module that is not loaded in this pipeline — or by no module at all, as in the
two 3MF fixtures above. Aborting the slice breaks composition, which is the
property PnP's module system exists to provide.

## Problem Statement

`custom-G-code injection deviation` has two halves. This packet closes the second, user-facing one and the
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

**The `custom-G-code injection deviation` row's own counts are wrong and must not be quoted.** Measured
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

## Architecture Constraints

<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and, if `STALE:` is reported, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.

- **`cargo build -p pnp-cli` is a precondition of every `--test e2e` run.** The e2e harness resolves the CLI through `slicer_test_support::pnp_cli_bin`, whose `staleness_reason` seam returns a diagnostic when the artifact is absent or older than `crates/*/src/**`, and `pnp_cli_bin` **panics** on it. There is deliberately **no release/debug fallback probe** (removed by packet 162), so the binary must exist in the caller's own profile directory. Measured during this re-authoring: the first e2e attempt failed on that guard, not on an assertion. The Cargo package is `pnp-cli`; the binary is `pnp_cli`.
- Config key strings must be snake_case in Rust and in the manifest. `nozzle_diameter` already satisfies this; do not introduce a kebab-case variant anywhere.
- `ConfigView::keys` (`crates/slicer-ir/src/slice_ir.rs`) returns **every key in its own `fields` map**, sorted — it enforces no manifest scoping itself. The scoping to manifest-declared keys happens when the host **constructs** the view, so "the placeholder domain is the module's manifest" is true of the live pipeline but is *not* a property of `keys()`. Stating it as a property of the function is wrong and misleads anyone reading the accessor. This is the single fact that makes the placeholder domain a manifest question rather than a code question: the `for key in config.keys()` sweep in `run_gcode_postprocess` is not dead code to delete, it is the mechanism by which AC-8 works once `nozzle_diameter` is declared. Keep it.
- **`ConfigValue` arrives from real input in shapes a synthetic test does not produce.** A `ConfigView` built by `slicer_sdk::test_prelude::config_with` holds exactly the pairs the test wrote, so a test can hand `nozzle_diameter` a `ConfigValue::Float`. A `ConfigView` built from a real 3MF holds `ConfigValue::List(['0.4'])`, because OrcaSlicer stores per-extruder settings as vectors. `format_placeholder_value` must therefore handle `List` (element 0, recursively) and must treat an **empty** `List` as "no value" rather than as the empty string. This is the shape-mismatch class that made `[nozzle_diameter]` inert for every real slice while its unit test stayed green (review finding F2).
- **The observable error chain is no longer load-bearing for this packet, and the assertions that depended on it are gone.** `run_gcode_postprocess` returns `Err` only on `GcodeOutputBuilder` push failures (the pre-existing numeric codes 1-11), never on an unresolved placeholder. The chain still exists — a guest `ModuleError` becomes `DispatchError.reason`, formatted in `crates/slicer-wasm-host/src/dispatch.rs` as `"module error (code={}, fatal={}): {}"`; `PostpassStageRunner::run_gcode_postprocess` wraps it as `slicer_ir::PostpassError::FatalModule { message }`, whose `Display` in `crates/slicer-ir/src/stage_io.rs` reads `"fatal postpass module failure in {stage_id} for {module_id}: {message}"`; `crates/slicer-runtime/src/postpass.rs` returns it unchanged and `crates/slicer-runtime/src/pipeline.rs` renders `"postpass failed: {e}"` — but **nothing in this packet may assert against it**, because there is no placeholder failure to observe. AC-N2 now asserts the opposite: `Ok`, G-code produced, bracketed text present in the start block.
- No geometry, no millimetre/internal-unit conversion, and no coordinate arithmetic occurs in this packet, so the `coord-system` constraint does not apply. `nozzle_diameter` is carried as a plain-mm scalar for text substitution only and must **not** be passed through `mm_to_units`.

## Data and Contract Notes

- **IR/manifest contracts.** One new scalar key on one module's `[config.schema]`. No IR schema version changes, no new struct field, no `PROGRESS_EVENT_SCHEMA_VERSION`-class constant bump, so the blast-radius discipline for struct literals does not apply. The count-shaped assertion nearby is `module_manifest_registers_five_keys_with_expected_types_and_defaults` (`crates/slicer-runtime/tests/integration/machine_start_end_gcode_emission_tdd.rs`); it asserts all five `; key =` lines in the CONFIG_BLOCK, including `nozzle_diameter`, and therefore pins the exact delivered manifest surface. `gcode_header_thumbnail_config_blocks_tdd`'s CONFIG_BLOCK check is a **lower bound** ("at least 80 key-value lines"), so adding a key cannot break it. `crates/slicer-runtime/tests/integration/manifest_default_reconcile_tdd.rs` has fallback tables for `classic-perimeters` and `arachne-perimeters` only — `machine-gcode-emit` is not enrolled, so no reconcile row needs adding.
- **WIT boundary.** Unchanged. `run_gcode_postprocess` already returns `Result<(), ModuleError>` across the boundary; this packet removes a failure value rather than adding one, and adds a `log` host-services call that the SDK already wires. No `crates/slicer-schema/wit/**` edit, therefore no bindgen invalidation — but `modules/core-modules/machine-gcode-emit/src/**` and its `.toml` **are** guest-WASM inputs, so the freshness gate still applies.
- **Determinism/scheduler constraints.** The unresolved-key set must be a `BTreeSet` so the warning text is byte-identical across runs; a `HashSet` iteration order would make it non-deterministic. `machine-gcode-emit` is the only module registered at `PostPass::GCodePostProcess` (it is the sole manifest under `modules/core-modules/*/*.toml` naming that stage), so there is no ordering interaction with a sibling postpass module. Because emission now always proceeds, a warning has **no** effect on emitted bytes — the G-code is identical whether or not the warning fires, which is what makes AC-18's fixtures safe to use as a pass/fail gate.

## Risks and Tradeoffs

- **Bracketed literal text can still reach a printer.** Accepted — this is the
  residual of `custom-G-code injection deviation`'s user-facing half and is recorded in the residual
  `DEV-###` row. Mitigated only by the aggregated warning. The alternative
  (failing the slice) was tried, measured, and rejected: it breaks composition
  and it broke real fixtures.
- **A template canonical rejects now slices successfully in PnP.** The divergence
  runs the *opposite* way from what the superseded design claimed. Recorded in
  the residual row.
- **A warning is easy to miss.** A `log_warn` is weaker than an error by
  construction. If this proves insufficient in practice, the escalation path is a
  host/CLI-level strict mode with full-config visibility — **not** a module-level
  failure — and it needs an ADR decision first.
- **ADR-0050 and packet prose can drift.** The live ADR is aligned; the risk is
  a future change that edits one contract without the other. Mitigated by the
  ADR verification in the Doc Impact Statement, AC-4, and AC-18.
- **Synthetic-config blindness.** The whole reason this packet was re-authored.
  Mitigated by AC-16, AC-17 and AC-18; the rule to carry forward is that a
  criterion driven only by author-chosen `config_with` pairs is not evidence
  about user-visible behaviour.
- **Guest staleness / CLI staleness.** Every `--test integration` result is
  meaningless until `cargo xtask build-guests --check` is clean, and every
  `--test e2e` result is meaningless until `cargo build -p pnp-cli` has run.
  Called out in `requirements.md` §Step Completion Expectations and in every
  affected step's verification list.
