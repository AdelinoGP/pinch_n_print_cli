---
status: implemented
packet: 186-custom-gcode-placeholder-engine
task_ids:
  - TASK-305
backlog_source: "docs/specs/deviation-backlog-remediation-plan.md - the Packet Queue entry for custom-G-code injection deviation, tranche T3; split 1 of 3 (engine). Referenced by identity: the queue has been renumbered once already and row numbers are ledger facts."
context_cost_estimate: M
---

# Packet Contract: 186-custom-gcode-placeholder-engine

## Why this packet was re-authored (read first)

A previous revision of this packet made an unresolved `[key]` a **fatal slice
error**. It was implemented, and **all 18 of its acceptance criteria passed**. An
adversarial review then found that three e2e tests across two real
OrcaSlicer-authored 3MF fixtures had broken, and the repo owner rejected the
policy outright: *"Unresolved keys cannot be a fatal slice error, as PnP is
modular and MUST accept keys from modules that aren't loaded."* The fatal path
was reverted in code before landing.

**The authoring defect that let it ship: not one acceptance criterion exercised a
real-world template.** Every criterion drove the module through synthetic
`config_with` pairs or a hand-built `HashMap<ConfigKey, ConfigValue>` raw config,
so every criterion saw a placeholder domain the author had chosen. A user-visible,
irreversible behaviour reversal therefore passed a full 18/18 ceremony while
breaking real slicing. Measured in this checkout, both fixtures carry
OrcaSlicer-authored templates naming `[first_layer_bed_temperature]`,
`[initial_tool]`, `[max_layer_z]` and `[layer_z]` — none of which is a
`machine-gcode-emit` manifest key — alongside `[first_layer_temperature]` and
`[nozzle_diameter]`, which this packet *does* make resolve:

- `resources/cube_cilindrical_modifier.3mf` — sliced by
  `crates/slicer-runtime/tests/e2e/modifier_infill_tdd.rs`
- `resources/cube_4color.3mf` — sliced by
  `crates/slicer-runtime/tests/e2e/cube_painted_e2e_tdd.rs`

**AC-18 exists to close that hole permanently** and is the criterion this whole
re-authoring turns on.

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

## Scope Boundaries

Touches the substitution engine and the config-value reads that feed it —
`substitute_placeholders`, `format_placeholder_value` and `run_gcode_postprocess`
in `modules/core-modules/machine-gcode-emit/src/lib.rs`, one new
`[config.schema.nozzle_diameter]` block in
`modules/core-modules/machine-gcode-emit/machine-gcode-emit.toml`, and the two
test files that pin placeholder behaviour.

No new injection point is added here: `machine_start_gcode` and
`machine_end_gcode` remain the only two sites, and the registry that generalises
them is packet 187.

**Explicitly out of scope: changing the unknown-key rule.** Neither a fatal
error, nor an empty-string substitution, nor a `strict_placeholders` opt-out.
Verbatim passthrough plus one aggregated warning is the shipped contract.

Of the ten macros the pre-correction `docs/15` list advertised but never
delivered, **two are recovered here and eight are not**. `[nozzle_diameter]` is
recovered as a real config key (and only *works* because of the list-valued fix —
see AC-16). `[first_layer_temperature]` is recovered as a **placeholder alias** of
the already-declared `nozzle_temperature_initial_layer` — not a new config key —
because canonical does exactly that: `GCode::update_placeholder_parser_with_variant_params`
sets it **unconditionally** under the verbatim comment
`// first_layer_temperature is a legacy alias of nozzle_temperature_initial_layer`
(see AC-15). The remaining **eight** (`[bed_temperature]`, `[filament_type]`,
`[tool_count]`, `[layer_count]`, `[print_time_estimate_s]`, `[x_max]`, `[y_max]`,
`[z_max]`) are **not** implemented — none is a PnP config key and none has a
canonical alias PnP can honour — and each is recorded in one residual deviation
row with its measured canonical counterpart.

## Prerequisites and Blockers

- Depends on: none.
- Unblocks: `187-custom-gcode-injection-registry` (TASK-306) and, transitively,
  `188-custom-gcode-conditional-points` (TASK-307). Both add injection points
  whose templates run through this engine, and both consume
  `try_slice_with_raw` (added by this packet to
  `crates/slicer-runtime/tests/integration/machine_start_end_gcode_emission_tdd.rs`).
  187's registry is what supplies the computed variables (`layer_num`, `layer_z`,
  `max_layer_z`) that several of the eight residual macros map onto — and that
  the two 3MF fixtures already reference — so the residual row filed here must
  not be closed by 187 without re-checking each entry.
- **ADR alignment:** `docs/adr/0050-custom-gcode-architecture.md` now records the
  shipped warn-and-pass policy, the manifest-scoped placeholder domain, and the
  module-private engine. This packet consumes those aligned decisions; it does
  not author or edit the ADR (see `design.md` §Decisions of Record), and there is
  no ADR blocker.

## Acceptance Criteria

**Mandatory conventions for every `cargo test` command below — all measured, not
stylistic:**

1. **`rg "^test result:"` — the colon is load-bearing.** Unanchored
   `^test result` also matches libtest's per-test NAME lines, so a green run
   whose test names begin with `result` trips a `0 failed`-style guard.
2. **Filter resolution differs between the test binaries used here, and every
   form below was measured on this tree.**
   `modules/core-modules/machine-gcode-emit/tests/machine_gcode_emit_tdd.rs` is
   its own Cargo integration binary with top-level `#[test]` fns, so a **bare**
   name plus `--exact` resolves (measured:
   `-- unknown_placeholder_passes_through_verbatim --exact` → `1 passed; 13 filtered out`).
   `crates/slicer-runtime/tests/integration/machine_start_end_gcode_emission_tdd.rs`
   is mounted as `mod machine_start_end_gcode_emission_tdd;` in
   `crates/slicer-runtime/tests/integration/main.rs`, and the e2e suites are
   mounted the same way in `crates/slicer-runtime/tests/e2e/main.rs`, so both
   need the **module-qualified** path. A bare name with `--exact` against the
   `integration` or `e2e` binary selects nothing forever.
3. **`cargo build -p pnp-cli` must precede every `--test e2e` run.** The e2e
   suites spawn the CLI through `slicer_test_support::pnp_cli_bin`, whose
   `staleness_reason` guard **panics** when the binary is absent or older than
   `crates/*/src/**`, and which has **no release/debug fallback probe** — it
   resolves the sibling of the caller's own profile dir. Measured during this
   re-authoring: the first e2e attempt failed on that guard, not on an assertion.
   The package name is `pnp-cli`; the binary is `pnp_cli`.

Every `cargo test` command in this packet selects exactly **one** test binary
(`--test machine_gcode_emit_tdd`, `--test integration`, or `--test e2e`) and
therefore emits exactly one `^test result:` line, so all of them take the
**single-result-line** guard form:
`… 2>&1 | rg "^test result:" | rg -q "^test result: ok\. [1-9]" && echo PASS || echo "FAIL: …"`.
The `[1-9]` rejects a filtered-to-zero run as well as a partial failure.
`rg -v "0 passed"` and `rg -v "0 failed"` are **banned**: the first reports
success on `test result: FAILED. 3 passed; 4 failed;`, the second reports success
on a run that selected zero tests.

Doc probes are written in `python3` rather than `rg` wherever the pattern
contains a backtick or a `|`: inside `bash -c '…'` a backtick in a double-quoted
`rg` pattern is command-substituted, which collapses the pattern into a
non-discriminating literal. Every command below was executed against the live
tree during this re-authoring and the recorded baseline is stated in its own
text.

- **AC-1. Given** a `machine_start_gcode` template carrying non-ASCII literal text around a declared placeholder — `"; café ☕ M140 S[bed_temperature_initial_layer_single]"` with `bed_temperature_initial_layer_single = 60` — **when** `run_gcode_postprocess` runs, **then** the emitted `GCodeCommand::Raw` text is byte-identical to `"; café ☕ M140 S60"`. On the unfixed tree the literal path is `out.push(bytes[i] as char)`, which reinterprets each UTF-8 continuation byte as a Latin-1 `char`, so `café` (`c a f 0xC3 0xA9`) emerges as `cafÃ©` and the assertion fails. **This is the one engine fix that was never in dispute** and it survives the policy reversal unchanged. | `bash -c 'cargo test -p machine-gcode-emit --test machine_gcode_emit_tdd -- non_ascii_template_text_survives_substitution --exact 2>&1 | rg "^test result:" | rg -q "^test result: ok\. [1-9]" && echo PASS || echo "FAIL: non_ascii_template_text_survives_substitution did not run or did not pass"'`
- **AC-2. Given** `modules/core-modules/machine-gcode-emit/src/lib.rs`, **when** grepped, **then** no `bytes[i] as char` byte-to-`char` cast remains anywhere in the file, and the literal-run copy goes through `push_str` on a `&str` slice of the template. **This is a structural smoke check, not the proof.** Its `push_str` clause already passed on the unfixed tree, so AC-2 hinges entirely on the absence of the literal `bytes[i] as char` — which a rename of the loop index would satisfy with the cast intact. **AC-1 is the behavioural proof**; AC-2 exists only to stop the specific known-bad construct from reappearing. | `bash -c '! rg -q "bytes\[i\] as char" modules/core-modules/machine-gcode-emit/src/lib.rs && rg -q "push_str" modules/core-modules/machine-gcode-emit/src/lib.rs && echo PASS || echo "FAIL: mojibake byte-cast still present, or the literal path no longer uses push_str"'`
- **AC-3. Given** a template referencing a placeholder that **is** a declared config key, **when** `run_gcode_postprocess` runs, **then** it is still substituted (`M140 S[bed_temperature_initial_layer_single]` with `bed_temperature_initial_layer_single = 60` yields `M140 S60`) and the call returns `Ok`. `known_placeholder_is_substituted` was green before this packet — this is a **do-not-regress guard**, not a change-proving AC. | `bash -c 'cargo test -p machine-gcode-emit --test machine_gcode_emit_tdd -- known_placeholder_is_substituted --exact 2>&1 | rg "^test result:" | rg -q "^test result: ok\. [1-9]" && echo PASS || echo "FAIL: known_placeholder_is_substituted did not run or did not pass"'`
- **AC-4. Given** `modules/core-modules/machine-gcode-emit/src/lib.rs`, **when** grepped, **then** the unresolved-key path goes through **`slicer_sdk::host::log_warn`** and the identifier **`ERR_UNRESOLVED_PLACEHOLDER` does not appear anywhere in the file**. This AC is the **inverse of the criterion it replaces**, which required `pub const ERR_UNRESOLVED_PLACEHOLDER` to exist and to be passed to `ModuleError::fatal`. That constant, its `ModuleError::fatal` call site, and the `sites_clause` test helper that formatted its message were all deleted when the policy was reverted; asserting their absence is what stops the rejected fatal path being reinstated. Measured during re-authoring: this command prints `PASS` on the current tree, and `ERR_UNRESOLVED_PLACEHOLDER` / `sites_clause` have **zero** occurrences anywhere under `modules/` or `crates/`. | `bash -c 'rg -q "slicer_sdk::host::log_warn" modules/core-modules/machine-gcode-emit/src/lib.rs && ! rg -q "ERR_UNRESOLVED_PLACEHOLDER" modules/core-modules/machine-gcode-emit/src/lib.rs && echo PASS || echo "FAIL: the aggregated warn path is absent, or the reverted ERR_UNRESOLVED_PLACEHOLDER constant is back"'`
- **AC-5. Given** a `machine_start_gcode` carrying `[zzz_two]` then `[zzz_one]` and a `machine_end_gcode` carrying `[zzz_three]` and the duplicate `[zzz_one]`, **when** `run_gcode_postprocess` runs with native `install_log_capture()` installed, **then** it returns `Ok`, emits both templates verbatim, and `take_log_messages()` captures exactly one `Warn` record. The behavioral assertion requires `[zzz_one]` to occur exactly once in the warning despite appearing at both sites, `[zzz_one]`, `[zzz_three]`, `[zzz_two]` to occur once each in sorted order, and `machine_start_gcode` and `machine_end_gcode` to occur once each. This proves collect-all-then-warn-once behavior, cross-site deduplication, deterministic `BTreeSet` ordering, and both-site attribution; it is not a structural source check. | `bash -c 'mkdir -p target && cargo test -p machine-gcode-emit --test machine_gcode_emit_tdd -- every_unresolved_placeholder_passes_through_verbatim --exact 2>&1 | tee target/log-186-ac5.txt && rg -q "^test result:" target/log-186-ac5.txt && rg -q "^test result: ok\. [1-9]" target/log-186-ac5.txt && python3 -c "import io,sys; s=io.open(r\"modules/core-modules/machine-gcode-emit/tests/machine_gcode_emit_tdd.rs\",encoding=\"utf-8\").read(); need=(\"install_log_capture()\",\"take_log_messages()\",\"warnings.len()\",\"occurrences(\"+chr(34)+\"[zzz_one]\"+chr(34)+\")\",\"machine_start_gcode\",\"machine_end_gcode\"); missing=[x for x in need if x not in s]; print(\"PASS\" if not missing else \"FAIL: behavioral AC-5 assertions missing \"+str(missing)); sys.exit(0 if not missing else 1)" && echo PASS || echo "FAIL: AC-5 did not run green before its behavioral assertion"'`
- **AC-6. Given** a `machine_start_gcode` of `"hello [world"` (an opening bracket with no `]` before end-of-line), **when** `run_gcode_postprocess` runs, **then** the remainder of the line is still treated as literal text, the call returns `Ok`, and the text is emitted verbatim — an unclosed bracket is **not** a placeholder at all and must not even enter the unresolved set. `unclosed_bracket_is_literal` was green before this packet — **do-not-regress guard**. | `bash -c 'cargo test -p machine-gcode-emit --test machine_gcode_emit_tdd -- unclosed_bracket_is_literal --exact 2>&1 | rg "^test result:" | rg -q "^test result: ok\. [1-9]" && echo PASS || echo "FAIL: unclosed_bracket_is_literal did not run or did not pass"'`
- **AC-7. Given** `modules/core-modules/machine-gcode-emit/machine-gcode-emit.toml`, **when** parsed, **then** `[config.schema]` has exactly the five-key set `{machine_start_gcode, machine_end_gcode, bed_temperature_initial_layer_single, nozzle_temperature_initial_layer, nozzle_diameter}` — exact set equality, not merely a length check. `nozzle_diameter` must have `type = "float"`, bare-number `default = 0.4`, `min = 0.1`, `max = 2.0`, and `unit = "mm"`, mirroring `classic-perimeters`; the AC-15 alias must not become a sixth manifest key. | `bash -c 'python3 -c "import tomllib; d=tomllib.load(open(r\"modules/core-modules/machine-gcode-emit/machine-gcode-emit.toml\",\"rb\"))[\"config\"][\"schema\"]; expected={\"machine_start_gcode\",\"machine_end_gcode\",\"bed_temperature_initial_layer_single\",\"nozzle_temperature_initial_layer\",\"nozzle_diameter\"}; k=\"nozzle_diameter\"; ok=set(d)==expected and d[k][\"type\"]==\"float\" and isinstance(d[k][\"default\"],float) and abs(d[k][\"default\"]-0.4)<1e-9 and abs(d[k][\"min\"]-0.1)<1e-9 and abs(d[k][\"max\"]-2.0)<1e-9 and d[k].get(\"unit\")==\"mm\"; print(\"PASS\" if ok else \"FAIL: schema key set or nozzle_diameter fields are wrong (found \"+str(sorted(d))+\")\")"'`
- **AC-8. Given** a full slice through `run_pipeline_with_raw_config` with `machine_start_gcode = "; nozzle is [nozzle_diameter]"`, **when** the pipeline runs, **then** it succeeds and the emitted start block contains the literal `; nozzle is 0.4` — proving the new manifest declaration actually reaches the module's `ConfigView`. `ConfigView::keys` returns its own `fields` map sorted (manifest scoping is applied by the host at construction, not by `keys()`), which is why the pre-existing `for key in config.keys()` sweep in `run_gcode_postprocess` contributed nothing before the key was declared. **AC-8 is necessary but not sufficient** — it exercises the *schema default*, a `Float`; AC-16 covers the `List` shape real 3MF input actually delivers. | `bash -c 'cargo test -p slicer-runtime --test integration -- machine_start_end_gcode_emission_tdd::nozzle_diameter_macro_resolves_end_to_end --exact 2>&1 | rg "^test result:" | rg -q "^test result: ok\. [1-9]" && echo PASS || echo "FAIL: nozzle_diameter_macro_resolves_end_to_end did not run or did not pass"'`
- **AC-9. Given** the whole `machine_gcode_emit_tdd` binary, **when** it runs, **then** every test in it passes. Measured during re-authoring: `test result: ok. 14 passed; 0 failed`. **Re-derive the count at closure rather than quoting `14`** — a test count is a ledger fact. | `bash -c 'mkdir -p target && cargo test -p machine-gcode-emit --test machine_gcode_emit_tdd 2>&1 | tee target/log-186-mge.txt | rg "^test result:" | rg -q "^test result: ok\. [1-9]" && echo PASS || echo "FAIL: machine_gcode_emit_tdd is not fully green — see target/log-186-mge.txt"'`
- **AC-10. Given** the whole `machine_start_end_gcode_emission_tdd` module of the `integration` binary, **when** it runs, **then** every test in it passes. Measured during re-authoring: `test result: ok. 14 passed; 0 failed; … 240 filtered out`. This is the end-to-end guard for start/end block placement (`start_block_position_before_extrusion_mode_and_first_g1`, `end_block_position_after_last_g1_before_config_block`, `extrusion_mode_still_emitted_after_promotion`, `module_manifest_registers_five_keys_with_expected_types_and_defaults`, `new_keys_appear_in_config_block`, …), including the asserted `nozzle_diameter` CONFIG_BLOCK line, which the manifest declaration must not disturb. The name-prefix filter carries no `--exact`, so it substring-matches the module path and selects the whole module; it still emits exactly one `^test result:` line because it selects one binary. | `bash -c 'mkdir -p target && cargo test -p slicer-runtime --test integration -- machine_start_end_gcode_emission_tdd:: 2>&1 | tee target/log-186-msege.txt | rg "^test result:" | rg -q "^test result: ok\. [1-9]" && echo PASS || echo "FAIL: the machine_start_end_gcode_emission_tdd module is not fully green — see target/log-186-msege.txt"'`
- **AC-11. Given** `docs/15_config_keys_reference.md` §"Machine start / end G-code", **when** read, **then** it states **warn-and-pass**, not the fatal policy. Concretely: the section is locatable by its heading prefix (the heading on disk is `## Machine start / end G-code (packet 59)`); it names the macros a user is expected to reach for — `` `[bed_temperature_initial_layer_single]` ``, `` `[nozzle_temperature_initial_layer]` ``, `` `[nozzle_diameter]` `` and the alias `` `[first_layer_temperature]` `` — in backticked-bracket form; it carries the literal words `passes through verbatim and is warned about`; and the string `fatal slice error` appears **nowhere in the whole file**. The superseded criterion demanded the exact opposite (`unresolved placeholder is a fatal slice error`); the planner has already rewritten the doc, and this command was executed against what is now on disk and prints `PASS`. **No numeral is asserted, and the doc must not write one.** `run_gcode_postprocess`'s `config.keys()` sweep also resolves the manifest's own `machine_start_gcode` / `machine_end_gcode` string keys, so any count is wrong on this packet's own implementation; and packet 187 adds three more keys to the same manifest and rewrites this same section, so any numeral is stale one packet later. The durable statement is the *rule* — the placeholder domain is exactly this module's manifest-declared key set plus the alias table — which the section now states. A set-equality test over every bracketed name in the section is also rejected: the section legitimately names unresolvable macros (`[bed_temperature]`, `[layer_count]`, …) in its "these do not resolve" prose. Backticked names are built from `chr(96)` / `chr(91)` / `chr(93)` because a backtick inside a double-quoted `rg` pattern inside `bash -c '…'` is command-substituted and yields a non-discriminating probe. | `bash -c 'python3 -c "import io; s=io.open(r\"docs/15_config_keys_reference.md\",encoding=\"utf-8\").read(); b=chr(96); i=s.find(\"## Machine start / end G-code\"); sec = s[i:] if i>=0 else \"\"; j=sec.find(chr(10)+\"## \",1); sec = sec[:j] if j>0 else sec; want=(\"bed_temperature_initial_layer_single\",\"nozzle_temperature_initial_layer\",\"nozzle_diameter\",\"first_layer_temperature\"); miss=[k for k in want if (b+chr(91)+k+chr(93)+b) not in sec]; pol=\"passes through verbatim and is warned about\" in sec; nofatal=\"fatal slice error\" not in s; ok = i>=0 and not miss and pol and nofatal; print(\"PASS\" if ok else \"FAIL: section=\"+str(i>=0)+\", missing macros=\"+str(miss)+\", warn-policy-sentence=\"+str(pol)+\", fatal-sentence-absent=\"+str(nofatal))"'`
- **AC-12. Given** the manifest change, **when** `cargo xtask gen-config-docs` has been re-run, **then** `cargo xtask gen-config-docs --check` exits 0 and the generated `module-config-keys` table in `docs/15_config_keys_reference.md` carries a `nozzle_diameter` row attributed to `machine-gcode-emit`. The `--check` half alone is a do-not-regress guard; the row clause is what discriminates. Measured during re-authoring: a generated table row pairs `nozzle_diameter` (float, default `0.4`, range `[0.1, 2.0]`) with `machine-gcode-emit`, alongside the pre-existing `arachne-perimeters` and `classic-perimeters` rows for the same key. The row is assembled from `chr(96)` / `chr(124)` for the same substitution reason as AC-11. | `bash -c 'cargo xtask gen-config-docs --check >/dev/null 2>&1 && python3 -c "import io; s=io.open(r\"docs/15_config_keys_reference.md\",encoding=\"utf-8\").read(); b=chr(96); p=chr(124); print(\"PASS\" if any((b+\"nozzle_diameter\"+b) in ln and (b+\"machine-gcode-emit\"+b) in ln for ln in s.splitlines() if ln.startswith(p)) else \"FAIL: no generated table row pairs nozzle_diameter with machine-gcode-emit\")" || echo "FAIL: gen-config-docs --check is red"'`
- **AC-13. Given** the post-purge `docs/DEVIATION_LOG.md`, **when** the packet closes, **then** exactly one surviving `DEV-100` table row records all eight unresolved macros with their canonical counterpart terms and defining structures: `[bed_temperature]` / `TemperaturesConfigDef` / `get_bed_temp_key`, `[filament_type]` / `PrintConfigDef`, `[tool_count]` / `num_extruders` / `OtherSlicingStatesConfigDef`, `[layer_count]` / `total_layer_count` / `PrintStatisticsConfigDef`, `[print_time_estimate_s]` / `print_time_sec`, `[x_max]` and `[y_max]` / `print_bed_max` / `DimensionsConfigDef`, and `[z_max]` / `printable_height` / `max_layer_z`. The same single row must state the manifest-domain asymmetry, warn-and-pass, `log_warn`, `reverted`, and canonical `check_placeholder_parser_failed` divergence, with no `slice-fatal` token; the deleted aggregate custom-G-code row must remain absent. The row ID is re-derived at closure; it is not hard-coded. | `bash -c 'python3 -c "import io; b=chr(96); p=chr(124); L=io.open(r\"docs/DEVIATION_LOG.md\",encoding=\"utf-8\").read().splitlines(); macros=[b+chr(91)+k+chr(93)+b for k in (\"bed_temperature\",\"filament_type\",\"tool_count\",\"layer_count\",\"print_time_estimate_s\",\"x_max\",\"y_max\",\"z_max\")]; terms=macros+[\"TemperaturesConfigDef\",\"get_bed_temp_key\",\"PrintConfigDef\",\"num_extruders\",\"OtherSlicingStatesConfigDef\",\"total_layer_count\",\"PrintStatisticsConfigDef\",\"print_time_sec\",\"print_bed_max\",\"DimensionsConfigDef\",\"printable_height\",\"max_layer_z\",\"log_warn\",\"reverted\",\"check_placeholder_parser_failed\"]; rows=[l for l in L if l.startswith(p+chr(32)+\"DEV-\")]; residual=[l for l in rows if all(t in l for t in macros)]; retired=any(l.startswith(p+\" custom-G-code injection deviation \") for l in L); ok=len(residual)==1 and residual[0].startswith(p+chr(32)+\"DEV-100\") and all(t in residual[0] for t in terms) and \"slice-fatal\" not in residual[0] and not retired; print(\"PASS\" if ok else \"FAIL: DEV-100 evidence or deleted-row absence is wrong\")"'`
- **AC-14. Given** `docs/07_implementation_status.md`, **when** the packet closes, **then** exactly one `TASK-305` backlog row exists **outside** the `<!-- BEGIN GENERATED: open-deviations (cargo xtask check-deviations) -->` / `<!-- END GENERATED: open-deviations -->` markers, and that row describes the **delivered** warn-and-pass behavior plus the fact that the rejected fatal-on-unresolved attempt was **reverted before landing** after breaking real OrcaSlicer-authored 3MF e2e fixtures. The generated block remains generator-owned and is never hand-edited. | `bash -c 'python3 -c "import io; L=io.open(r\"docs/07_implementation_status.md\",encoding=\"utf-8\").read().splitlines(); B=[i for i,l in enumerate(L) if l.startswith(\"<!-- BEGIN GENERATED: open-deviations\")]; E=[i for i,l in enumerate(L) if l.startswith(\"<!-- END GENERATED: open-deviations\")]; H=[(i,l) for i,l in enumerate(L) if l.startswith(\"- [\") and \"TASK-305\" in l]; ok=len(B)==1 and len(E)==1 and len(H)==1 and not (B[0]<H[0][0]<E[0]) and all(t in H[0][1] for t in (\"Delivered\",\"warn-and-pass\",\"reverted before landing\")); print(\"PASS\" if ok else \"FAIL: expected exactly one delivered TASK-305 row outside generated markers\")"'`
- **AC-15. Given** a `machine_start_gcode` of `"M109 S[first_layer_temperature]"` and a declared `nozzle_temperature_initial_layer = 215` — with **no** config key named `first_layer_temperature` anywhere — **when** `run_gcode_postprocess` runs, **then** it returns `Ok` and emits `M109 S215`, resolved through a `const PLACEHOLDER_ALIASES: &[(&str, &str)]` table whose single entry maps `"first_layer_temperature"` to `"nozzle_temperature_initial_layer"`. **This is a canonical port, not a convenience:** `GCode::update_placeholder_parser_with_variant_params` sets the name **unconditionally** under the verbatim comment `// first_layer_temperature is a legacy alias of nozzle_temperature_initial_layer`. It must **not** be implemented as a second manifest key — AC-7 still requires exactly five — because canonical models it as a parser alias and two config keys for one value could disagree. Aliases resolve **after** the config sweep, so a real key of the same name would win if one ever appeared. **Both 3MF fixtures in AC-18 reference `[first_layer_temperature]`,** so AC-15 and AC-18 overlap by design: AC-15 pins the mechanism, AC-18 proves it on real input. | `bash -c 'cargo test -p machine-gcode-emit --test machine_gcode_emit_tdd -- first_layer_temperature_alias_resolves_to_nozzle_temperature_initial_layer --exact 2>&1 | rg "^test result:" | rg -q "^test result: ok\. [1-9]" && python3 -c "import io,sys; s=io.open(r\"modules/core-modules/machine-gcode-emit/src/lib.rs\",encoding=\"utf-8\").read(); q=chr(34); i=s.find(\"PLACEHOLDER_ALIASES\"); tbl=s[i:i+600] if i>=0 else \"\"; sys.exit(0 if i>=0 and (q+\"first_layer_temperature\"+q) in tbl and (q+\"nozzle_temperature_initial_layer\"+q) in tbl else 1)" && echo PASS || echo "FAIL: the alias test did not run/pass, or PLACEHOLDER_ALIASES does not map first_layer_temperature to nozzle_temperature_initial_layer"'`
- **AC-16 (new). Given** a `machine_start_gcode` of `"; nozzle is [nozzle_diameter]"` and `nozzle_diameter` supplied as **`ConfigValue::List([Float(0.4)])`** rather than a scalar, **when** `run_gcode_postprocess` runs, **then** it returns `Ok` and emits `; nozzle is 0.4` — `format_placeholder_value` resolves a `List` **from its first element**, recursively. **This criterion exists because of adversarial review finding F2, and it is the reason AC-8 alone was not enough.** Real 3MF input supplies per-extruder settings as vectors: `nozzle_diameter` reaches the module as `['0.4']`, never as a scalar, so the sweep's original catch-all arm dropped it and the packet's headline macro was **inert for every real slice** while the scalar-default AC-8 stayed green. Canonical does the same element-0 read where a placeholder needs a single value (`nozzle_temperature_initial_layer.get_at(0)` in `GCode::_do_export`'s `; first_layer_temperature = %d` preamble). This is the same class of gap as AC-18's: a criterion that only ever saw author-chosen input. | `bash -c 'cargo test -p machine-gcode-emit --test machine_gcode_emit_tdd -- list_valued_config_key_resolves_from_first_element --exact 2>&1 | rg "^test result:" | rg -q "^test result: ok\. [1-9]" && echo PASS || echo "FAIL: list_valued_config_key_resolves_from_first_element did not run or did not pass"'`
- **AC-17 (new). Given** a `machine_start_gcode` of `"M104 S[nozzle_temperature]"` and `nozzle_temperature` supplied as an **empty** `ConfigValue::List([])`, **when** `run_gcode_postprocess` runs, **then** it returns `Ok`, the key stays **out** of the lookup, `M104 S[nozzle_temperature]` is emitted verbatim, and **no** `M104 S` is emitted. An empty list must not collapse to an empty string: `M104 S` is a worse printer command than the bracketed form and it is silent, which is exactly the "substitute unknown keys with the empty string" alternative `design.md` §Rejected alternatives rules out. AC-17 is the boundary condition of AC-16 — without it, `items.first()` could be "fixed" with an `unwrap_or_default()` and no criterion would notice. | `bash -c 'cargo test -p machine-gcode-emit --test machine_gcode_emit_tdd -- empty_list_config_key_passes_through_verbatim --exact 2>&1 | rg "^test result:" | rg -q "^test result: ok\. [1-9]" && echo PASS || echo "FAIL: empty_list_config_key_passes_through_verbatim did not run or did not pass"'`
- **AC-18 (new, anti-regression — the criterion whose absence let the fatal policy ship). Given** the two committed OrcaSlicer-authored 3MF fixtures whose embedded print profiles carry real custom-G-code templates, **when** each is sliced end to end through `pnp_cli`, **then** the slice **SUCCEEDS** and its suite is fully green. Measured in this checkout, both fixtures' templates reference `[first_layer_bed_temperature]`, `[initial_tool]`, `[max_layer_z]` and `[layer_z]` — **none of which is a `machine-gcode-emit` manifest key or an alias** — as well as `[first_layer_temperature]` and `[nozzle_diameter]`, which AC-15 and AC-16 make resolve. Under the reverted fatal policy every one of the unresolvable four aborted the slice. **No criterion in the superseded 18 ever sliced a real fixture**; all of them drove the module through synthetic `config_with` pairs or a hand-built raw config, which is precisely why an 18/18 green ceremony coexisted with three broken e2e tests. Vehicles: `crates/slicer-runtime/tests/e2e/modifier_infill_tdd.rs` (`resources/cube_cilindrical_modifier.3mf`) and `crates/slicer-runtime/tests/e2e/cube_painted_e2e_tdd.rs` (`resources/cube_4color.3mf`) — 2 tests each, both green at re-authoring. **`cargo build -p pnp-cli` is part of the command, not a precondition to remember:** the e2e harness resolves the CLI through `slicer_test_support::pnp_cli_bin`, whose `staleness_reason` guard panics on an absent or stale binary and has no cross-profile fallback. Measured: the first e2e attempt during re-authoring failed on that guard, not on an assertion. | `bash -c 'mkdir -p target && cargo build -p pnp-cli >/dev/null 2>&1 || { echo "FAIL: cargo build -p pnp-cli failed; the e2e harness cannot run"; exit 1; }; cargo test -p slicer-runtime --test e2e -- modifier_infill_tdd:: 2>&1 | tee target/log-186-e2e-modifier.txt | rg "^test result:" | rg -q "^test result: ok\. [1-9]" || { echo "FAIL: modifier_infill_tdd (cube_cilindrical_modifier.3mf) is not green — see target/log-186-e2e-modifier.txt"; exit 1; }; cargo test -p slicer-runtime --test e2e -- cube_painted_e2e_tdd:: 2>&1 | tee target/log-186-e2e-painted.txt | rg "^test result:" | rg -q "^test result: ok\. [1-9]" || { echo "FAIL: cube_painted_e2e_tdd (cube_4color.3mf) is not green — see target/log-186-e2e-painted.txt"; exit 1; }; echo PASS'`

## Negative Test Cases

- **AC-N1. Given** a `machine_start_gcode` of `"X[unknown_key]Y"` where `unknown_key` is **not** a declared config key and **not** an alias, **when** `run_gcode_postprocess` runs, **then** it returns **`Ok`** and a `GCodeCommand::Raw` carrying the literal text `X[unknown_key]Y` **is** emitted, brackets included. **This criterion is the exact inverse of the one it replaces**, which required `Err(ModuleError)` with `code == ERR_UNRESOLVED_PLACEHOLDER` and asserted that no such `Raw` was emitted. Passthrough is the asserted behaviour at module level because a module's `ConfigView` is scoped to its own manifest: a key `machine-gcode-emit` cannot resolve may be owned by a module that is not loaded, so the module has no standing to fail the slice over it. The guard clause additionally asserts that the reverted `unknown_placeholder_is_a_fatal_module_error` test is **absent**, so the inversion cannot be reinstated alongside the passthrough test and leave the file self-contradicting. | `bash -c 'cargo test -p machine-gcode-emit --test machine_gcode_emit_tdd -- unknown_placeholder_passes_through_verbatim --exact 2>&1 | rg "^test result:" | rg -q "^test result: ok\. [1-9]" && ! rg -q "fn unknown_placeholder_is_a_fatal_module_error" modules/core-modules/machine-gcode-emit/tests/machine_gcode_emit_tdd.rs && echo PASS || echo "FAIL: the module-level passthrough test did not run/pass, or the reverted fatal test is back in machine_gcode_emit_tdd.rs"'`
- **AC-N2. Given** a full slice through `run_pipeline_with_raw_config` with `machine_start_gcode = "TEMP [no_such_key] DONE"`, **when** the pipeline runs, **then** it returns **`Ok`**, G-code **is** produced, and the emitted start block — the span between `; HEADER_BLOCK_END` and the `M82`/`M83` extrusion-mode line — contains `TEMP [no_such_key] DONE` verbatim. This is AC-N1's pipeline-level half, through the real `machine-gcode-emit.wasm` component, and it is likewise the **exact inverse** of the criterion it replaces (which asserted `Err`, the `postpass failed` / `module error (code=` chain, and that no G-code was produced). That whole error-chain apparatus is now irrelevant to this packet: there is no failure to observe. **`try_slice_with_raw(raw) -> Result<String, PipelineError>` is retained** in `crates/slicer-runtime/tests/integration/machine_start_end_gcode_emission_tdd.rs` even though this criterion no longer needs its fallibility — the test uses it so a pipeline failure produces the error text instead of a bare `expect` panic, and packets 187 and 188 both depend on the helper existing. `slice_with_raw` stays expressed as `try_slice_with_raw(raw).expect("pipeline must succeed")`. The guard clause asserts the reverted `unknown_placeholder_is_a_fatal_slice_error` test is absent. | `bash -c 'cargo test -p slicer-runtime --test integration -- machine_start_end_gcode_emission_tdd::unknown_placeholder_passes_through_verbatim --exact 2>&1 | rg "^test result:" | rg -q "^test result: ok\. [1-9]" && ! rg -q "fn unknown_placeholder_is_a_fatal_slice_error" crates/slicer-runtime/tests/integration/machine_start_end_gcode_emission_tdd.rs && echo PASS || echo "FAIL: the pipeline-level passthrough test did not run/pass, or the reverted fatal e2e test is back"'`
- **AC-N3. Given** an **empty** or whitespace-only `machine_start_gcode`, **when** `run_gcode_postprocess` runs, **then** it returns `Ok`, emits no `Raw` wrapper, and behavioral log capture via `install_log_capture()` / `take_log_messages()` contains **zero warning records**. Both `empty_templates_emit_no_raw_wrappers` and `whitespace_only_template_is_skipped` must assert `logs.is_empty()` after draining the capture sink; a structural scan for unresolved keys is not sufficient. This matches canonical, where every injection site is guarded before parsing. | `bash -c 'mkdir -p target && cargo test -p machine-gcode-emit --test machine_gcode_emit_tdd -- empty_templates_emit_no_raw_wrappers --exact 2>&1 | tee target/log-186-acn3-empty.txt && rg -q "^test result: ok\. [1-9]" target/log-186-acn3-empty.txt && cargo test -p machine-gcode-emit --test machine_gcode_emit_tdd -- whitespace_only_template_is_skipped --exact 2>&1 | tee target/log-186-acn3-whitespace.txt && rg -q "^test result: ok\. [1-9]" target/log-186-acn3-whitespace.txt && python3 -c "import io,sys; s=io.open(r\"modules/core-modules/machine-gcode-emit/tests/machine_gcode_emit_tdd.rs\",encoding=\"utf-8\").read(); ok=s.count(\"install_log_capture()\")>=2 and s.count(\"take_log_messages()\")>=2 and s.count(\"logs.is_empty()\")>=2; print(\"PASS\" if ok else \"FAIL: AC-N3 lacks behavioral zero-warning assertions\"); sys.exit(0 if ok else 1)" && echo PASS || echo "FAIL: an AC-N3 test did not run green or lacks a behavioral zero-warning assertion"'`

## Verification

- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo xtask build-guests --check`
- The AC-10 command (whole `machine_start_end_gcode_emission_tdd` module).
- **The AC-18 command (both real-3MF e2e suites).** This is a packet-level gate,
  not merely a criterion: it is the check whose absence let a rejected policy
  pass a full acceptance ceremony.

## Authoritative Docs

- `docs/15_config_keys_reference.md` — long; **ranged reads only**. Read §"Machine start / end G-code" and the `<!-- BEGIN GENERATED: module-config-keys (cargo xtask gen-config-docs) -->` / `<!-- END GENERATED: module-config-keys -->` marker boundaries; delegate anything wider.
- `docs/DEVIATION_LOG.md` — long; the surviving `DEV-100` placeholder row and current custom-G-code residual rows only, plus a re-derivation of the highest `DEV-###`. Delegate; never load whole. Rows are single very long lines.
- `docs/07_implementation_status.md` — **always delegate** (skill Context Contract). Needed only for the `TASK-305` row outside the generated block.
- `docs/adr/0050-custom-gcode-architecture.md` — rewritten in place by this
  closure work and now the current warn-and-pass authority for the placeholder
  domain and engine ownership. Verify its title, Decision 1, consequences, and
  alternatives together; see `design.md` §Decisions of Record.
- `docs/02_ir_schemas.md` — delegated SUMMARY only, to confirm that `PostPass::GCodePostProcess` receives `GCodeIR.commands` and a `ConfigView` and nothing else (no `PrintMetadata` access), which is the fact that makes `[layer_count]` unreachable from this stage.

## Doc Impact Statement (Required)

This packet changes a module manifest schema (one new key) and rewrites a
user-facing behaviour contract, so `none` is not available. Every item below
describes what is **on disk now** — the planner has already applied the reversal
to `docs/**`, and each entry was re-verified during this re-authoring.

- `docs/15_config_keys_reference.md`, generated block `module-config-keys` — regenerate with `cargo xtask gen-config-docs`; **never hand-edit inside the markers**. CI runs `gen-config-docs --check`. On disk: the `nozzle_diameter` / `machine-gcode-emit` row is present. Verification: the AC-12 command.
- `docs/15_config_keys_reference.md`, §"Machine start / end G-code" prose (outside the generated markers) — states the **domain rule** (a `[key]` resolves iff it is declared in this module's manifest or is a legacy name in the alias table) rather than a count, names the macros a user reaches for, and carries the warn-and-pass blockquote **"An unresolved placeholder passes through verbatim and is warned about — it is not a slice error"** with the modularity reason. It also records the residual wrinkle that the template keys are themselves manifest `string` keys and so resolve inside their own templates, and that there is no escape syntax for a literal `[foo]` **and none is needed**, since a bracketed non-key is simply left alone. **No numeral appears, and none may be added.** Verification: the AC-11 command.
- `docs/DEVIATION_LOG.md`, the residual row — exactly one residual row with a
  re-derived `DEV-###` ID (**never quote the current number**). It enumerates the
  eight unresolvable macros with their canonical counterparts, records the domain
  asymmetry, records that the rejected fatal-on-unresolved policy was **reverted
  before landing** with the repo owner's modularity ruling and measured 3MF
  evidence, and records the failure-handling divergence (canonical
  marks-and-continues then throws once; PnP warns once and completes).
  Verification: the AC-13 command.
- `docs/DEVIATION_LOG.md`, the retired aggregate custom-G-code label — no historical aggregate row is restored. The surviving placeholder evidence is `DEV-100`; packet 187/188 residuals are carried by the surviving packet residual rows, including `DEV-103` through `DEV-105`. Verification: `! rg -q '^\| custom-G-code injection deviation ' docs/DEVIATION_LOG.md && rg -q '^\| DEV-100 ' docs/DEVIATION_LOG.md && rg -q '^\| DEV-103 ' docs/DEVIATION_LOG.md && rg -q '^\| DEV-104 ' docs/DEVIATION_LOG.md && rg -q '^\| DEV-105 ' docs/DEVIATION_LOG.md && echo PASS`
- `docs/07_implementation_status.md` — exactly one `TASK-305` row is registered
  **outside** the `<!-- BEGIN GENERATED: open-deviations (cargo xtask
  check-deviations) -->` markers, and its text describes delivered warn-and-pass
  plus the reverted fatal-on-unresolved attempt. Verification: the AC-14
  command, followed by `cargo xtask check-deviations` to confirm the generated
  block remains generator-owned.
- `docs/adr/0050-custom-gcode-architecture.md` — rewritten in place by this
  packet and verified as the warn-and-pass authority. Verify that its title,
  Decision 1, consequences, and alternatives agree with one aggregated warning
  and no fatal unknown-placeholder policy. Verification:
  `bash -c 'python3 -c "import io; s=io.open(r\"docs/adr/0050-custom-gcode-architecture.md\",encoding=\"utf-8\").read(); need=(\"warn-and-pass\",\"manifest-declared\",\"slicer_sdk::host::log_warn\",\"recoverable diagnostics\"); print(\"PASS\" if all(x in s for x in need) and \"fails the slice\" not in s else \"FAIL: ADR-0050 is not aligned\")"'`.
- `docs/specs/deviation-backlog-remediation-plan.md`, the **Packet Queue entry for `custom-G-code injection deviation` (tranche T3)** — **referenced by identity, never quoted, and never counted; the batch orchestrator owns this file and this packet performs no edit here.** An earlier draft pasted the then-current row's cell text verbatim and asserted a frozen `rg -c` hit-count; the orchestrator has since split row 8 into `8a`/`8b`/`8c`, one per packet directory, so the quoted text, the count, and a `startswith("| 8 |")` probe all became permanently false. The probe below re-derives instead: it scans every Packet Queue row naming a `.ralph/specs/` directory and asserts each of the three packet directories appears in some row, with no dependence on row numbering, row count, or one-row-versus-three layout. Measured during re-authoring: prints **PASS**, so it is a **do-not-regress guard**. Verification: `bash -c 'python3 -c "import io; p=chr(124); b=chr(96); L=io.open(r\"docs/specs/deviation-backlog-remediation-plan.md\",encoding=\"utf-8\").read().splitlines(); rows=[l for l in L if l.startswith(p) and (b+\".ralph/specs/\") in l]; dirs=(\"186-custom-gcode-placeholder-engine\",\"187-custom-gcode-injection-registry\",\"188-custom-gcode-conditional-points\"); miss=[d for d in dirs if not any(d in r for r in rows)]; print(\"PASS\" if rows and not miss else (\"FAIL: no Packet Queue row names any .ralph/specs directory\" if not rows else \"FAIL: no queue row names \"+str(miss)))"'`
- `docs/ORCA_CONFIG_REFERENCE.md` — **no edit; deliberately out of scope.** Its
  custom-G-code inventory remains unchanged. Untouched-file verification at
  closure: `bash -c 'BASE=$(<target/pkt-186-baseline-ref.txt); test -n "$BASE" && git rev-parse --verify -q "$BASE^{commit}" >/dev/null && if git diff --name-only "$BASE" -- docs/ORCA_CONFIG_REFERENCE.md | rg -q .; then echo "FAIL: unexpected edit"; exit 1; else echo PASS; fi'`.
- `docs/adr/0055-fuel-based-module-profiling.md` — renumbered from the
  duplicate 0050 slot, with every fuel-specific reference updated to ADR-0055;
  custom-G-code references remain ADR-0050. Verification:
  `bash -c 'rg -q "^# ADR-0055 — Fuel-based module profiling$" docs/adr/0055-fuel-based-module-profiling.md && ! rg -q "0050-fuel-based-module-profiling|ADR-0050" crates docs/09_progress_events.md docs/17_agent_debugging.md && echo PASS || echo "FAIL: fuel ADR reference sweep is incomplete"'`.
- `.ralph/specs/187-custom-gcode-injection-registry/` and
  `.ralph/specs/188-custom-gcode-conditional-points/` — all five contract files
  now inherit warn-and-pass for unknown per-site variables and the revised
  marker policy. Verification:
  `bash -c 'rg -q "warn-and-pass" .ralph/specs/187-custom-gcode-injection-registry .ralph/specs/188-custom-gcode-conditional-points && ! rg -q "ERR_UNRESOLVED_PLACEHOLDER|unknown.*fatal" .ralph/specs/187-custom-gcode-injection-registry .ralph/specs/188-custom-gcode-conditional-points && echo PASS || echo "FAIL: sibling packet policy migration is incomplete"'`.
- `.ralph/specs/187-custom-gcode-injection-registry/**` and
  `.ralph/specs/188-custom-gcode-conditional-points/**` — **read-only sibling
  packet migrations.** They inherit ADR-0050's aligned warn-and-pass rule and
  the five-key integration contract before adding their own keys; this packet
  must not edit them. Verification: `bash -c 'BASE=$(<target/pkt-186-baseline-ref.txt); test -n "$BASE" && git rev-parse --verify -q "$BASE^{commit}" >/dev/null && ! git diff --name-only "$BASE" -- .ralph/specs/187-custom-gcode-injection-registry .ralph/specs/188-custom-gcode-conditional-points | rg -q . && rg -q "try_slice_with_raw|warn-and-pass|ADR-0050" .ralph/specs/187-custom-gcode-injection-registry/*.md .ralph/specs/188-custom-gcode-conditional-points/*.md && echo PASS || echo "FAIL: sibling packet migration guard failed"'`.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file + function + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines. **Cite canonical by function name, never by line number.**

**Checkout hazard, measured during this re-authoring: this repo's `OrcaSlicerDocumented/src/libslic3r/` has NO `GCode.cpp`.** It carries `GCodeReader`, `GCodeSender` and `GCodeWriter` only. Every `GCode.cpp` fact this packet relies on was verified against a full sibling checkout at `F:\slicerProject\pinch_n_print_cli_2\OrcaSlicerDocumented`, where the file is present. A dispatch that asks the in-repo mirror for `GCode::placeholder_parser_process` returns empty, and an agent that reads that emptiness as "the function does not exist" will conclude the opposite of the truth. State the checkout in any dispatch that needs `GCode.cpp`.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/PlaceholderParser.cpp` — `MyContext::legacy_variable_expansion` and `MyContext::throw_exception`, for the fact that canonical's `[key]` legacy bracket form **errors** on an undefined variable (`"Variable does not exist"`) rather than passing it through, and `MyContext::process_error_message` for the message shape. Borrowed **only as the contrast** this packet documents as a deliberate divergence — no longer as justification for a failure policy.
- `src/libslic3r/GCode.cpp` (**sibling checkout only**) — `GCode::placeholder_parser_process` and `GCode::check_placeholder_parser_failed`, for the deferral shape: each failure is recorded into `PlaceholderParserIntegration::failed_templates` (dedupe by template name is performed by the insert-if-absent guard in `placeholder_parser_process`, not by the checker) and the throw happens once, later, over all of them. Borrowed for AC-5's collect-all-then-**warn**-once aggregation, which is the half PnP adopts.
- `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` — `PrintConfigDef::init_fff_params`, for the true registration count (13 `coString` + 3 `coStrings`) that corrects the `custom-G-code injection deviation` row. **Four canonical placeholder names this packet cites live in three OTHER structures, and attributing them to the custom-gcode table sends a worker to an empty result:** `total_layer_count` and `print_time_sec` are in `PrintStatisticsConfigDef` (coInt / coString), `num_extruders` is in `OtherSlicingStatesConfigDef` (coInt), and `print_bed_max` is in `DimensionsConfigDef` (coFloats). None of the four is in `s_CustomGcodeSpecificPlaceholders`, and none is in `CustomGcodeSpecificConfigDef` either. Only `max_layer_z` (and `layer_num`) genuinely live in both custom-gcode structures. Dispatch against the named struct, not the table.
- `src/libslic3r/GCode.cpp` (**sibling checkout only**) — `GCode::update_placeholder_parser_with_variant_params`, for the **unconditional** `placeholder_parser().set("first_layer_temperature", …)` and its verbatim comment `// first_layer_temperature is a legacy alias of nozzle_temperature_initial_layer`. This is AC-15's authority. **An earlier draft claimed the name was set *only* on `GCode::set_extruder`'s `toolchange_temp_override` path; that is refuted.** Re-counted by execution: the name occurs **nine** times in `GCode.cpp` — two unconditional `placeholder_parser().set` calls (this one, using `remap_ints_by_filament(m_config.nozzle_temperature_initial_layer)`, plus one in `GCode::_do_export` using `new ConfigOptionInts(m_config.nozzle_temperature_initial_layer)`); two `toolchange_temp_override > 0`-gated `set_key_value`s in `GCode::set_extruder`; one `set_key_value` on a local `DynamicConfig` in **`WipeTowerIntegration::append_tcr`**, gated `full_config.enable_tower_interface_features && tcr.is_contact` (**not** in `get_path_of_change_filament`, which an earlier draft named and which contains no such set); two `; first_layer_temperature = %d` CONFIG_BLOCK emissions in `GCode::_do_export`; and two source comments. **Deliberately NOT borrowed:** the surrounding toolchange emission, which is packet 188's subject.
- `src/libslic3r/GCode.cpp` (**sibling checkout only**) — `GCode::_do_export`, for `nozzle_temperature_initial_layer.get_at(0)` in the `; first_layer_temperature = %d` preamble: canonical also reads **element 0** of a per-extruder vector where a placeholder needs one value. This is AC-16's canonical anchor.

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- obey the shared absolute context bands: 120k reading budget with hand-off at 150k (standard); the extended band (240k reading / 300k hard stop) only via swarm's escalation protocol

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation (an extended-band run may carry a single L step only when `design.md` justifies why it cannot be split).
