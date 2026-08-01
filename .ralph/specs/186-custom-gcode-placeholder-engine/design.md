# Design: 186-custom-gcode-placeholder-engine

## Controlling Code Paths

- Primary code path: `substitute_placeholders`, `format_placeholder_value` and `run_gcode_postprocess` in `modules/core-modules/machine-gcode-emit/src/lib.rs` (the `#[slicer_module] impl PostpassModule for MachineGcodeEmit` block). `run_gcode_postprocess` reads the two templates, builds a `HashMap<String, String>` lookup seeded with `bed_temperature_initial_layer_single` / `nozzle_temperature_initial_layer` and then swept over `config.keys()`, applies `PLACEHOLDER_ALIASES`, substitutes both templates, warns once about anything unresolved, and frames the re-emitted command stream with `push_raw`.
- Config surface: `modules/core-modules/machine-gcode-emit/machine-gcode-emit.toml` `[config.schema]` — five declared keys after this packet.
- Neighboring tests/fixtures: `modules/core-modules/machine-gcode-emit/tests/machine_gcode_emit_tdd.rs` (top-level `#[test]` fns, driven through `slicer_sdk::test_prelude::config_with`, which builds a `ConfigView` containing **exactly** the supplied pairs — so a unit test controls the placeholder domain precisely, which is both its power and, as §Why warn-and-pass records, the blind spot that let a rejected policy pass 18/18); `modules/core-modules/machine-gcode-emit/tests/slicer_module_binding_tdd.rs` (`binding_surface_matches_gcode_postprocess_stage`); `crates/slicer-runtime/tests/integration/machine_start_end_gcode_emission_tdd.rs` (real WASM component, real pipeline, via `slice_with_raw` / `try_slice_with_raw` → `run_pipeline_with_raw_config`); and — new to this packet's criterion set — `crates/slicer-runtime/tests/e2e/modifier_infill_tdd.rs` and `crates/slicer-runtime/tests/e2e/cube_painted_e2e_tdd.rs`, which slice real OrcaSlicer-authored 3MFs through `pnp_cli`.
- OrcaSlicer comparison: see `requirements.md` §OrcaSlicer Reference Obligations; do not repeat delegation rules. **Note the checkout hazard recorded there: this repo's `OrcaSlicerDocumented/` has no `GCode.cpp`.**

## Architecture Constraints

<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and, if `STALE:` is reported, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.

- **`cargo build -p pnp-cli` is a precondition of every `--test e2e` run.** The e2e harness resolves the CLI through `slicer_test_support::pnp_cli_bin`, whose `staleness_reason` seam returns a diagnostic when the artifact is absent or older than `crates/*/src/**`, and `pnp_cli_bin` **panics** on it. There is deliberately **no release/debug fallback probe** (removed by packet 162), so the binary must exist in the caller's own profile directory. Measured during this re-authoring: the first e2e attempt failed on that guard, not on an assertion. The Cargo package is `pnp-cli`; the binary is `pnp_cli`.
- Config key strings must be snake_case in Rust and in the manifest. `nozzle_diameter` already satisfies this; do not introduce a kebab-case variant anywhere.
- `ConfigView::keys` (`crates/slicer-ir/src/slice_ir.rs`) returns **every key in its own `fields` map**, sorted — it enforces no manifest scoping itself. The scoping to manifest-declared keys happens when the host **constructs** the view, so "the placeholder domain is the module's manifest" is true of the live pipeline but is *not* a property of `keys()`. Stating it as a property of the function is wrong and misleads anyone reading the accessor. This is the single fact that makes the placeholder domain a manifest question rather than a code question: the `for key in config.keys()` sweep in `run_gcode_postprocess` is not dead code to delete, it is the mechanism by which AC-8 works once `nozzle_diameter` is declared. Keep it.
- **`ConfigValue` arrives from real input in shapes a synthetic test does not produce.** A `ConfigView` built by `slicer_sdk::test_prelude::config_with` holds exactly the pairs the test wrote, so a test can hand `nozzle_diameter` a `ConfigValue::Float`. A `ConfigView` built from a real 3MF holds `ConfigValue::List(['0.4'])`, because OrcaSlicer stores per-extruder settings as vectors. `format_placeholder_value` must therefore handle `List` (element 0, recursively) and must treat an **empty** `List` as "no value" rather than as the empty string. This is the shape-mismatch class that made `[nozzle_diameter]` inert for every real slice while its unit test stayed green (review finding F2).
- **The observable error chain is no longer load-bearing for this packet, and the assertions that depended on it are gone.** `run_gcode_postprocess` returns `Err` only on `GcodeOutputBuilder` push failures (the pre-existing numeric codes 1-11), never on an unresolved placeholder. The chain still exists — a guest `ModuleError` becomes `DispatchError.reason`, formatted in `crates/slicer-wasm-host/src/dispatch.rs` as `"module error (code={}, fatal={}): {}"`; `PostpassStageRunner::run_gcode_postprocess` wraps it as `slicer_ir::PostpassError::FatalModule { message }`, whose `Display` in `crates/slicer-ir/src/stage_io.rs` reads `"fatal postpass module failure in {stage_id} for {module_id}: {message}"`; `crates/slicer-runtime/src/postpass.rs` returns it unchanged and `crates/slicer-runtime/src/pipeline.rs` renders `"postpass failed: {e}"` — but **nothing in this packet may assert against it**, because there is no placeholder failure to observe. AC-N2 now asserts the opposite: `Ok`, G-code produced, bracketed text present in the start block.
- No geometry, no millimetre/internal-unit conversion, and no coordinate arithmetic occurs in this packet, so the `coord-system` constraint does not apply. `nozzle_diameter` is carried as a plain-mm scalar for text substitution only and must **not** be passed through `mm_to_units`.

## Code Change Surface

- **Selected approach.** `substitute_placeholders` keeps its byte-oriented scan and changes shape twice.
  - *Literal runs.* Track the start index of the current literal run and, on reaching a `[` or the end of input, emit `out.push_str(&template[run_start..i])`. Because `[` is ASCII and UTF-8 is self-synchronising, `i` is always at a char boundary when a `[` is found, so slicing is safe and the `bytes[i] as char` cast disappears. The former `std::str::from_utf8(...).unwrap_or("")` calls on the key and passthrough paths become direct `&template[..]` slices for the same reason; `unwrap_or("")` silently discarding a UTF-8 error is a defect of the same family.
  - *Unresolved keys.* Return `(String, Vec<String>)` — the rendered text and the sorted, deduplicated list of bracketed keys that had no entry in `lookup`. **The rendered text keeps the verbatim `[key]`, brackets included, and that text is what gets emitted.** The list exists so the caller can warn once. An unclosed `[` never enters the list at all: the remainder of that line is literal text and is not a placeholder.
- **Caller.** `run_gcode_postprocess` substitutes both templates, unions the two unresolved lists into a `BTreeSet<String>` (sorted, deduplicated), and — if it is non-empty — emits exactly **one** `slicer_sdk::host::log_warn` naming every unresolved key in `[key]` form and every injection point that contributed one (`machine_start_gcode`, `machine_end_gcode`), then **continues to Step 4's `push_raw` and returns `Ok`**. The `BTreeSet` is not stylistic: a `HashSet` iteration order would make the warning text non-deterministic across runs.
- **`format_placeholder_value`.** A free function rendering one `ConfigValue` as the text a `[key]` substitutes to, or `None` when it has no single-value rendering. `String` / `Int` / `Float` / `Bool` render directly. **`List` resolves from its first element, recursively** — canonical does the same element-0 read where a placeholder needs one value (`nozzle_temperature_initial_layer.get_at(0)` in `GCode::_do_export`'s `; first_layer_temperature = %d` preamble). An **empty** `List` returns `None`, so the key stays out of the lookup and its placeholder stays unresolved (emitted verbatim and warned about) rather than collapsing to an empty string. `Percent` and `FloatOrPercent` stay unrendered: they are meaningless without the base they resolve against.
- **Placeholder aliases.** A `const PLACEHOLDER_ALIASES: &[(&str, &str)]` table with the single entry `("first_layer_temperature", "nozzle_temperature_initial_layer")`. Applied **after** the `config.keys()` sweep so a real config key of the same name would win if one ever appeared. This is a port, not a convenience: `GCode::update_placeholder_parser_with_variant_params` sets the name unconditionally under the comment `// first_layer_temperature is a legacy alias of nozzle_temperature_initial_layer`. It must not become a second manifest key — canonical models it as a parser alias, and two config keys for one value can disagree.

- **Why warn-and-pass rather than fatal.**

  *(This section replaces "Why fatal rather than warn-and-pass", which argued the
  opposite. The reversal is recorded here rather than deleted, because the
  reasoning error is reusable and the ADR that encodes the old conclusion is
  still on disk.)*

  **The architectural reason.** A module's `ConfigView` is scoped to **its own
  manifest**. `machine-gcode-emit` therefore cannot distinguish two situations
  that look identical from inside the guest: (a) `[foo]` names nothing anywhere,
  and (b) `[foo]` names a key owned by a module that is not loaded in this
  pipeline, or by a host/profile surface this stage never sees. Failing case (b)
  makes a template's validity depend on which modules happen to be wired into the
  current DAG — which is precisely the coupling PnP's module system exists to
  prevent. In the repo owner's words: *"PnP is modular and MUST accept keys from
  modules that aren't loaded."* Passthrough is the only rule that composes.

  **The measured evidence.** The fatal policy was implemented and **all 18 of its
  acceptance criteria passed**. Adversarial review then found three broken e2e
  tests across two committed, real, OrcaSlicer-authored 3MF fixtures.
  Re-measured during this re-authoring by reading the templates out of the
  archives: `resources/cube_cilindrical_modifier.3mf` and
  `resources/cube_4color.3mf` both carry custom-G-code templates naming
  `[first_layer_bed_temperature]`, `[initial_tool]`, `[max_layer_z]` and
  `[layer_z]` — none of which is a `machine-gcode-emit` manifest key or an alias
  — as well as `[first_layer_temperature]` and `[nozzle_diameter]`, which this
  packet does make resolve. Under the fatal policy each of the first four aborted
  the slice.

  **The reasoning error, stated so it is not repeated.** The old rationale
  imported canonical's policy without checking that canonical's *premise* holds
  in PnP. Canonical's `PlaceholderParser` resolves against a persistent parser
  carrying the **entire** print config plus ~119 explicit
  `placeholder_parser().set(...)` global assignments, with a per-call local
  `DynamicConfig` layered on top as an *override*
  (`ppi.parser.process(templ, current_filament_id, config_override, &ppi.output_config, &ppi.context)`).
  In that world "not found" really does mean "not a thing", so erroring is sound.
  PnP's domain is one module's manifest — five keys plus one alias after this
  packet. The same rule applied to a two-order-of-magnitude smaller domain turns
  "unknown" from a user error into a routine, correct condition.

  **What is still borrowed from canonical.** The *aggregation*, not the action:
  `GCode::placeholder_parser_process` records each failure into
  `PlaceholderParserIntegration::failed_templates` (deduped by template name via
  its insert-if-absent guard) and defers to
  `GCode::check_placeholder_parser_failed` rather than acting on the first one, so
  a user with three bad macros learns about all three in one pass. PnP keeps that
  shape and substitutes a single `log_warn` for the throw. `slicer_sdk::host::log_warn`
  is a real host-services call: on `wasm32` it forwards through the
  `slicer:common/host-services#log` WIT import; on native targets `log` in
  `crates/slicer-sdk/src/host.rs` writes to stderr, or into the per-thread
  `LOG_CAPTURE` sink when `host::test_support::install_log_capture` has been
  called.

  **The residual harm is stated, not hidden.** Bracketed literal text can still
  reach a printer — now accompanied by a warning naming every offending key and
  site. That is the harm `DEV-085` originally named, and it is not fully closed
  here; the residual `DEV-###` row records it as the accepted cost of
  composability, together with the divergence that canonical ultimately *fails*
  an export PnP completes.

- **Exact functions, manifests, tests, and fixtures.**
  - `substitute_placeholders` — signature `(&str, &HashMap<String, String>) -> (String, Vec<String>)`; literal-run slicing; no `as char`; unresolved keys retained verbatim in the rendered text.
  - `format_placeholder_value` — `(&ConfigValue) -> Option<String>`; `List` → first element, recursively; empty `List` → `None`.
  - `run_gcode_postprocess` — union the unresolved lists into a `BTreeSet`, emit one `log_warn` naming keys and sites, then emit normally and return `Ok`.
  - **No error constant.** `ERR_UNRESOLVED_PLACEHOLDER`, its `ModuleError::fatal` call site, and the `sites_clause` test helper that formatted its message are **deleted**, and must stay deleted (AC-4 asserts absence). Verified during re-authoring: zero occurrences anywhere under `modules/` or `crates/`. The file's other numeric `ModuleError::fatal` codes (1-11, all `push_*` failures) are untouched.
  - `machine-gcode-emit.toml` — one new `[config.schema.nozzle_diameter]` block copied field-for-field from `modules/core-modules/classic-perimeters/classic-perimeters.toml`'s block (`type = "float"`, `default = 0.4`, `min = 0.1`, `max = 2.0`, `unit = "mm"`), with a `display` and a `group = "Machine G-code"` of its own.
  - `machine_gcode_emit_tdd.rs` — keep `unknown_placeholder_passes_through_verbatim` asserting passthrough; add `non_ascii_template_text_survives_substitution`, `every_unresolved_placeholder_passes_through_verbatim`, `first_layer_temperature_alias_resolves_to_nozzle_temperature_initial_layer`, `list_valued_config_key_resolves_from_first_element` (AC-16), `empty_list_config_key_passes_through_verbatim` (AC-17), and the `try_run` helper that surfaces the module's `Result` instead of unwrapping it. **All are required**: the §Context Discipline Note makes this section the authoritative files-in-scope and change list, so a test omitted here is a test the implementer never writes.
  - `machine_start_end_gcode_emission_tdd.rs` — add `try_slice_with_raw(raw) -> Result<String, PipelineError>` and re-express `slice_with_raw` as `try_slice_with_raw(raw).expect("pipeline must succeed")`; keep `unknown_placeholder_passes_through_verbatim` asserting the bracketed text reaches the emitted start block; add `nozzle_diameter_macro_resolves_end_to_end`. `try_slice_with_raw` is retained even though no criterion in this packet now needs its fallibility — the passthrough test uses it so a pipeline failure surfaces the error text instead of a bare `expect` panic, and **packets 187 and 188 both depend on the helper existing**.
  - `crates/slicer-runtime/tests/e2e/modifier_infill_tdd.rs` and `crates/slicer-runtime/tests/e2e/cube_painted_e2e_tdd.rs` — **no edit; used as vehicles.** AC-18 runs them unchanged as the real-3MF gate.
- **Rejected alternatives.**
  - *Fail the slice on an unresolved key* (`ModuleError::fatal(ERR_UNRESOLVED_PLACEHOLDER, …)`, collect-all-then-fail-once, fail-before-emit): **rejected — this was the chosen path in the superseded revision and it was implemented, then reverted.** Rejected by the repo owner as architecturally wrong for a modular slicer, and refuted by measurement: it broke three e2e tests across two real OrcaSlicer-authored 3MF fixtures. Not to be reinstated by a packet; see §Decisions of Record.
  - *Substitute unknown keys with the empty string*: **rejected, and still rejected** — turns `M104 S[first_layer_temperature]` into `M104 S`, which is a worse printer command than the bracketed form and is silent. AC-17 pins the empty-`List` boundary of exactly this failure mode.
  - *Silent passthrough with no diagnostic at all* (the pre-packet state): rejected — the user has nothing to look at when a machine rejects a start block. The aggregated `log_warn` is the minimum honest signal.
  - *Warn and pass through, with one aggregated `slicer_sdk::host::log_warn` naming every unresolved key and injection point*: **chosen.** Composes under module loading, keeps the diagnostic, and keeps the canonical aggregation shape.
  - *Inline `!!!!! Failed to process the custom G-code template` marker in the emitted G-code*: rejected — canonical emits the marker **and** fails, so borrowing the marker alone would ship a comment to the printer with no accompanying signal a user is likely to read. The `log_warn` is the PnP-shaped equivalent and does not alter the emitted text.
  - *A `strict_placeholders` opt-out config key that restores the fatal behaviour*: rejected — it makes the composition-breaking path reachable again, and canonical has no such toggle. If a strict mode is ever wanted it belongs at the host/CLI level with full-config visibility, not inside one module's `ConfigView`, and it requires a separate architecture decision.
  - *Declaring `[bed_temperature]`, `[layer_count]`, `[x_max]` … as manifest keys so they resolve*: rejected — none is an OrcaSlicer config key (`layer_count`, `x_max`, `y_max`, `z_max`, `tool_count`, `print_time_estimate_s` are computed print values whose canonical placeholder names are `total_layer_count`, `print_bed_max`, `max_layer_z`, `num_extruders`, `print_time_sec`; `bed_temperature` is a bed-type-dispatched placeholder variable, not an option under that name; `filament_type` is a canonical option with zero occurrences anywhere in this workspace). Declaring them would be inventing config keys, which `requirements.md` §Out of Scope forbids. **`[first_layer_temperature]` is deliberately not in that list**: it is adopted, but as a `PLACEHOLDER_ALIASES` entry rather than a manifest key. Adding it as a sixth `[config.schema]` key remains rejected — two config keys for one value can be set to different numbers.

## Files in Scope (read + edit)

Two primary source files plus the two test files that pin placeholder behaviour.
The test files are not optional extras: they and the engine must move together,
so they belong to the same step (see `implementation-plan.md` Step 2).

- `modules/core-modules/machine-gcode-emit/src/lib.rs` — role: the engine; expected change: `substitute_placeholders`' return type and literal path, `format_placeholder_value`'s `List` arm, `PLACEHOLDER_ALIASES`, and the collect-and-**warn** block in `run_gcode_postprocess`.
- `modules/core-modules/machine-gcode-emit/machine-gcode-emit.toml` — role: the placeholder domain; expected change: one new `[config.schema.nozzle_diameter]` block.
- `modules/core-modules/machine-gcode-emit/tests/machine_gcode_emit_tdd.rs` — role: module-level behaviour pins; expected change: `try_run` helper plus five added tests; the passthrough test keeps asserting passthrough.
- `crates/slicer-runtime/tests/integration/machine_start_end_gcode_emission_tdd.rs` — role: pipeline-level pins through the real WASM component; expected change: `try_slice_with_raw` added, one test added.
- `docs/15_config_keys_reference.md` — role: the user-facing macro contract; expected change: §"Machine start / end G-code" rewritten to the domain rule and the warn-and-pass policy, generated block regenerated.
- `docs/DEVIATION_LOG.md` — role: parity ledger; expected change: one new residual row, two corrections to the `DEV-085` row.
- `docs/07_implementation_status.md` — role: backlog; expected change: one `TASK-305` row outside the generated markers.

## Read-Only Context

- `crates/slicer-runtime/tests/e2e/modifier_infill_tdd.rs`, `crates/slicer-runtime/tests/e2e/cube_painted_e2e_tdd.rs` — **run, do not edit.** AC-18's vehicles. Read only their fixture-path helpers (`cube_cilindrical_modifier_3mf`, `cube_4color_3mf`) if you need to confirm which archive each slices.
- `modules/core-modules/classic-perimeters/classic-perimeters.toml` — the `[config.schema.nozzle_diameter]` block only — purpose: the exact field set to mirror.
- `crates/slicer-ir/src/slice_ir.rs` — do **not** open; the one needed fact (`ConfigView::keys` returns the view's own `fields` map sorted, with manifest scoping applied by the host at construction rather than by `keys()`) is stated in §Architecture Constraints.
- `crates/slicer-sdk/src/host.rs` — grep only, for `log_warn` and `log`'s native-vs-`wasm32` split, if a step needs to confirm where a warning goes. Do not browse.
- `crates/slicer-test-support/src/lib.rs` — grep only, for `pnp_cli_bin` / `staleness_reason`, if an e2e run fails with a `pnp_cli is stale` panic. Do not browse.
- `crates/slicer-wasm-host/src/dispatch.rs`, `crates/slicer-ir/src/stage_io.rs` — **no longer needed by any criterion.** The error-chain literals they carry were load-bearing only for the reverted fatal AC-N2. Do not open them for this packet.
- `crates/slicer-gcode/src/emit.rs` — do **not** open in this packet; nothing here changes host emission. Packet 187 needs it.

## Out-of-Bounds Files

- `OrcaSlicerDocumented/...` — delegate; never load. (And remember: no `GCode.cpp` in this checkout.)
- `target/`, `Cargo.lock`, generated code, vendored dependencies — never load.
- `docs/adr/0050-custom-gcode-architecture.md` — aligned authority rewritten in
  place by this closure work; verify its warn-and-pass policy and references.
  See §Decisions of Record.
- `crates/slicer-gcode/**` — no change; the serializer and emitter are untouched by this packet.
- `crates/slicer-runtime/tests/e2e/**` — run, never edit. Changing an e2e suite to accommodate this packet would recreate exactly the blindness AC-18 exists to remove.
- `.ralph/specs/187-*`, `.ralph/specs/188-*` — sibling contract packets whose
  inherited unknown-placeholder policy is updated in this closure work; do not
  edit their production source.
- The `<!-- BEGIN GENERATED: open-deviations (cargo xtask check-deviations) -->` … `<!-- END GENERATED: open-deviations -->` block of `docs/07_implementation_status.md` and the `<!-- BEGIN GENERATED: module-config-keys (cargo xtask gen-config-docs) -->` … `<!-- END GENERATED: module-config-keys -->` block of `docs/15_config_keys_reference.md` — regenerate via `cargo xtask check-deviations` / `cargo xtask gen-config-docs`; never hand-edit inside the markers.
- Unrelated crates — delegate symbol lookups; do not browse.

## Expected Sub-Agent Dispatches

- Question: does canonical's `[key]` legacy bracket form error on an undefined variable, and what is the exact message text? Scope: `OrcaSlicerDocumented/src/libslic3r/PlaceholderParser.cpp`, functions `MyContext::legacy_variable_expansion` / `MyContext::throw_exception`; return: `FACT`; purpose: the divergence sentence in the residual `DEV-###` row — **not** a justification for a failure policy.
- Question: how does `GCode::placeholder_parser_process` record a failure, where is it rethrown, and does the export continue in between? Scope: `src/libslic3r/GCode.cpp` in the **sibling checkout** `F:\slicerProject\pinch_n_print_cli_2\OrcaSlicerDocumented` (this repo's mirror has no `GCode.cpp`); return: `FACT`; purpose: the collect-all-then-warn-once shape and the residual row's divergence clause.
- Question: in `GCode::_do_export`'s `; first_layer_temperature = %d` preamble, does canonical read element 0 of the per-extruder vector? Scope: same sibling checkout; return: `FACT` ≤ 3 lines; purpose: AC-16's canonical anchor.
- Question: what is the current highest `DEV-###` in `docs/DEVIATION_LOG.md`, and does the `DEV-085` row still say "2 of OrcaSlicer's 15"? Scope: `docs/DEVIATION_LOG.md`; return: `FACT` (two lines); purpose: Step 5's row edits. **Re-derive at the moment of writing.**
- Question: is `TASK-305` present in `docs/07_implementation_status.md`, and what is the exact row format used by the last three `TASK-3xx` entries? Scope: `docs/07_implementation_status.md`; return: `FACT` (≤ 5 lines); purpose: Step 5's registration.
- Question: after the manifest edit, does `cargo xtask gen-config-docs --check` exit 0 and does the generated table pair `nozzle_diameter` with `machine-gcode-emit`? Scope: cargo run; return: `FACT` pass/fail; purpose: AC-12.
- Question: after `cargo build -p pnp-cli`, are `modifier_infill_tdd::` and `cube_painted_e2e_tdd::` both green under `--test e2e`? Scope: cargo run; return: `FACT` pass/fail plus the two `^test result:` lines; purpose: AC-18.

## Data and Contract Notes

- **IR/manifest contracts.** One new scalar key on one module's `[config.schema]`. No IR schema version changes, no new struct field, no `PROGRESS_EVENT_SCHEMA_VERSION`-class constant bump, so the blast-radius discipline for struct literals does not apply. The count-shaped assertion nearby is `module_manifest_registers_five_keys_with_expected_types_and_defaults` (`crates/slicer-runtime/tests/integration/machine_start_end_gcode_emission_tdd.rs`); it asserts all five `; key =` lines in the CONFIG_BLOCK, including `nozzle_diameter`, and therefore pins the exact delivered manifest surface. `gcode_header_thumbnail_config_blocks_tdd`'s CONFIG_BLOCK check is a **lower bound** ("at least 80 key-value lines"), so adding a key cannot break it. `crates/slicer-runtime/tests/integration/manifest_default_reconcile_tdd.rs` has fallback tables for `classic-perimeters` and `arachne-perimeters` only — `machine-gcode-emit` is not enrolled, so no reconcile row needs adding.
- **WIT boundary.** Unchanged. `run_gcode_postprocess` already returns `Result<(), ModuleError>` across the boundary; this packet removes a failure value rather than adding one, and adds a `log` host-services call that the SDK already wires. No `crates/slicer-schema/wit/**` edit, therefore no bindgen invalidation — but `modules/core-modules/machine-gcode-emit/src/**` and its `.toml` **are** guest-WASM inputs, so the freshness gate still applies.
- **Determinism/scheduler constraints.** The unresolved-key set must be a `BTreeSet` so the warning text is byte-identical across runs; a `HashSet` iteration order would make it non-deterministic. `machine-gcode-emit` is the only module registered at `PostPass::GCodePostProcess` (it is the sole manifest under `modules/core-modules/*/*.toml` naming that stage), so there is no ordering interaction with a sibling postpass module. Because emission now always proceeds, a warning has **no** effect on emitted bytes — the G-code is identical whether or not the warning fires, which is what makes AC-18's fixtures safe to use as a pass/fail gate.

## Decisions of Record (ADR-0050) and Packet-Local Invariants

`docs/adr/0050-custom-gcode-architecture.md` is the accepted, aligned decision
record for this packet. It records warn-and-pass for unresolved placeholders,
the manifest-scoped placeholder domain, private engine ownership, and the
closed injection registry. This packet consumes those decisions and records the
in-place ADR rewrite in its closure evidence.

The ADR relationship is explicit:

- **Warn-and-pass.** An unresolved `[key]` remains verbatim, the run returns
  `Ok`, and one `slicer_sdk::host::log_warn` aggregates sorted, deduplicated keys
  and contributing sites. The rejected fatal-on-unresolved attempt is historical
  evidence recorded in the residual row, not an implementation target.
- **Placeholder domain.** The domain is exactly one module's manifest-declared
  key set plus `PLACEHOLDER_ALIASES`; nothing reads outside `ConfigView` at this
  stage. The asymmetry against canonical's persistent parser is recorded in the
  residual `DEV-###` row.
- **Engine ownership.** `substitute_placeholders`, `format_placeholder_value`
  and `PLACEHOLDER_ALIASES` stay private to `machine-gcode-emit`; the engine is
  not promoted to `slicer-sdk`, `slicer-ir` or the host.

The aligned ADR removes the former blocker. The coordinator must verify the ADR
relationship and the closed `D-<n>-ADR-0050-AMENDED` row during closure.

Packet-local invariants (this packet's to keep):

- **Locked: unresolved `[key]` ⇒ verbatim passthrough + one aggregated warning +
  `Ok`.** No packet in this batch may change it without a repo-owner decision.
- **Locked: `format_placeholder_value`'s empty-`List` ⇒ `None`.** An empty list
  must never render as an empty string (AC-17).
- Not locked: `substitute_placeholders`' `(String, Vec<String>)` return shape may
  become a small struct in packet 187 when per-site context arrives; that is an
  anticipated, in-family change. The `Vec<String>` must stay sorted and
  deduplicated whatever the container.
- Not locked: the exact wording of the warning message. AC-5 asserts it
  behaviorally through native log capture (one warning, sorted/deduplicated
  keys, and each contributing site once), not through a prose snapshot.

## Risks and Tradeoffs

- **Bracketed literal text can still reach a printer.** Accepted — this is the
  residual of `DEV-085`'s user-facing half and is recorded in the residual
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

## Context Cost Estimate

- Aggregate: `M`
- Largest step: `M` (Step 2 — engine change plus the module test file)
- Highest-risk dispatch and required return format: the `docs/DEVIATION_LOG.md`
  read (long file; both the `DEV-085` and residual rows are single very long
  lines). Must return `FACT` of at most five lines: the highest `DEV-###`, and
  whether the two quoted error strings are still present. Never request either
  row verbatim.

## Open Questions

- `[FWD]` **Preserve behavioral warning capture in packet 187.** The native
  `LOG_CAPTURE` seam now proves the one-warning contract here. Packet 187 must
  extend the same collector for added injection sites, retaining sorted,
  deduplicated keys and one site occurrence per contributing point.
- `[FWD]` **`[layer_z]` and `[max_layer_z]` are already in live fixture input.**
  Both 3MF fixtures of AC-18 reference them, and both are computed per-site
  variables that packet 187's registry is meant to supply. 187 should treat those
  fixtures as its acceptance input rather than inventing synthetic templates.
- `[FWD]` The integration test symbol is
  `module_manifest_registers_five_keys_with_expected_types_and_defaults`; its
  assertions include `nozzle_diameter`. Sibling packets must preserve the
  five-key contract when they migrate the integration fixture.
- `[FWD]` `modules/core-modules/machine-gcode-emit/tests/machine_gcode_emit_tdd.rs`'s
  module doc comment must describe passthrough, not failure. It is prose, so no
  AC can see it.
