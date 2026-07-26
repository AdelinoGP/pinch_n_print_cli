# Implementation Plan: 186-custom-gcode-placeholder-engine

## Execution Rules

- Work one atomic step at a time; map every step to grouped task IDs.
- Use TDD, then implementation, then the narrowest falsifying validation.
- Every field below is a context-budget contract and must be filled independently; never write "see Step 1".

## Steps

### Step 0: Record the packet baseline ref

- Task IDs: `TASK-305`
- Objective: capture the commit this packet starts from, so every "this packet must not modify X" guard has a ref that survives the packet's own commits.
- Precondition: the working tree is at the commit from which this packet's work begins.
- Postcondition: `target/pkt-186-baseline-ref.txt` exists and contains a single commit SHA.
- Files allowed to read, with ranges when over 300 lines:
  - none.
- Files allowed to edit (at most 3):
  - none under version control; this step writes only `target/pkt-186-baseline-ref.txt`.
- Files explicitly out of bounds:
  - every source and doc file.
- Expected sub-agent dispatches:
  - none.
- Context cost: `S`
- Authoritative docs:
  - none.
- OrcaSlicer refs:
  - none.
- Verification:
  - `bash -c 'git diff --quiet && git diff --cached --quiet || { echo "FAIL: working tree or index is dirty - commit or stash first, or the baseline bakes in edits this packet must be measured against"; exit 1; }; mkdir -p target && git rev-parse HEAD > target/pkt-186-baseline-ref.txt && rg -q "^[0-9a-f]{40}$" target/pkt-186-baseline-ref.txt && echo PASS || echo "FAIL: baseline ref not recorded"'`
- Exit condition: the file exists and holds one SHA. **Every no-touch guard in this packet diffs against it.** Do not substitute `HEAD` (empty after the packet commits, so a committed edit passes) or `git merge-base HEAD master` (measured: `crates/slicer-gcode/src/emit.rs` already differs from that merge-base because of pre-existing branch work, so the guard would fail a correct implementation).

### Step 1: Red tests for the new engine behaviour

- Task IDs: `TASK-305`
- Objective: add the three new module-level tests and invert the existing passthrough test, so the intended behaviour is pinned before the engine changes. `modules/core-modules/machine-gcode-emit/tests/machine_gcode_emit_tdd.rs` must be red for exactly the packet's reasons at the end of this step.
- Precondition: `cargo test -p machine-gcode-emit --test machine_gcode_emit_tdd` is green today (measured; re-derive the test count rather than quoting it).
- Postcondition: the file contains `non_ascii_template_text_survives_substitution`, `every_unresolved_placeholder_is_reported_in_one_error`, `first_layer_temperature_alias_resolves_to_nozzle_temperature_initial_layer`, `unknown_placeholder_is_a_fatal_module_error`, and **no** `fn unknown_placeholder_passes_through_verbatim`. The binary is RED — either failing to compile (expected, if the tests name `ERR_UNRESOLVED_PLACEHOLDER` before Step 2 defines it) or compiling and failing on exactly those four tests.
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/machine-gcode-emit/tests/machine_gcode_emit_tdd.rs` — short; readable whole. The `run` and `raw_texts` helpers are what the new tests reuse. (Do not carry a line count forward — an earlier draft called this file "under 300 lines" when it was longer; line counts are ledger facts that rot.)
  - `modules/core-modules/machine-gcode-emit/src/lib.rs` — short; readable whole. Read `substitute_placeholders` and `run_gcode_postprocess` only, to name the constant and the message shape being asserted.
- Files allowed to edit (at most 3):
  - `modules/core-modules/machine-gcode-emit/tests/machine_gcode_emit_tdd.rs`
- Files explicitly out of bounds:
  - `modules/core-modules/machine-gcode-emit/src/lib.rs` (Step 2 owns it)
  - `crates/slicer-runtime/**` (Step 3 owns the e2e half)
  - `OrcaSlicerDocumented/**`, `target/**`
- Expected sub-agent dispatches:
  - Question: does canonical's `[key]` legacy form throw on an undefined variable, and what is the message text? Scope: `OrcaSlicerDocumented/src/libslic3r/PlaceholderParser.cpp` (`MyContext::legacy_variable_expansion`, `MyContext::throw_exception`); return: `FACT` ≤ 5 lines
- Context cost: `S`
- Authoritative docs:
  - `docs/15_config_keys_reference.md` §"Machine start / end G-code" — ranged read only; establishes the behaviour being reversed.
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/PlaceholderParser.cpp` — delegate; never load.
- Verification:
  - `bash -c 'mkdir -p target; cargo test -p machine-gcode-emit --test machine_gcode_emit_tdd >target/log-186-mge-red.txt 2>&1; if rg -q "^test result:" target/log-186-mge-red.txt; then rg -q "FAILED" target/log-186-mge-red.txt && echo "RED (assertion failures — expected)" || echo "NOT RED — the binary compiled and every test passed; the new tests are not discriminating"; else rg -q "^error(\[|:)" target/log-186-mge-red.txt && echo "RED (compile failure — expected: the new tests name pub const ERR_UNRESOLVED_PLACEHOLDER, which Step 2 introduces)" || echo "NOT RED — no test-result line and no compiler error; the run did not happen"; fi'` — this step is the only one whose success condition is a **red** binary; read `target/log-186-mge-red.txt` rather than re-running.
  - **A compile failure IS a valid RED here, and the gate must say so.** The earlier form piped through `rg "^test result:"` before looking for `FAILED`. Step 1's tests deliberately name `ERR_UNRESOLVED_PLACEHOLDER`, a constant Step 2 introduces, so the test binary **does not compile** at the end of Step 1: cargo emits `error[E0425]`-class diagnostics, prints **no** `^test result:` line at all, the `rg` chain finds nothing, and the old command printed `NOT RED — the new tests are not discriminating` — i.e. it reported the *opposite* of the truth in the single most likely outcome of a correctly-executed step. The replacement branches on whether the binary compiled: `^test result:` present ⇒ require `FAILED`; absent ⇒ require a compiler error. Only "compiled, ran, everything green" and "nothing ran at all" are NOT RED.
  - `bash -c '! rg -q "fn unknown_placeholder_passes_through_verbatim" modules/core-modules/machine-gcode-emit/tests/machine_gcode_emit_tdd.rs && echo PASS || echo "FAIL: old passthrough test still present"'`
- Exit condition: the four named tests exist, the old passthrough test does not, and **either** the binary fails to compile solely because the new tests name `ERR_UNRESOLVED_PLACEHOLDER` (Step 2 introduces it) **or** the binary compiles and its failures are exactly the new tests. Any *other* failing test, or any compiler error not traceable to the not-yet-introduced constant, falsifies the step and must be diagnosed before Step 2.

### Step 2: Fix the engine — char-boundary-correct literals, collect-and-fail on unresolved keys

- Task IDs: `TASK-305`
- Objective: change `substitute_placeholders` to `(&str, &HashMap<String, String>) -> (String, Vec<String>)` with slice-based literal copying and no `as char`; introduce **`pub const ERR_UNRESOLVED_PLACEHOLDER: u32 = 20;`** (`pub` is required — the module's `tests/` directory is a separate crate and AC-N1 names the constant symbolically; the crate exports only `pub struct MachineGcodeEmit` today) and `const PLACEHOLDER_ALIASES: &[(&str, &str)]` with its single `first_layer_temperature` -> `nozzle_temperature_initial_layer` entry (applied **after** the `config.keys()` sweep, never as a manifest key); and make `run_gcode_postprocess` union the two templates' unresolved keys into a `BTreeSet` and fail once, before any `push_raw`.
- Precondition: Step 1's four tests exist and are red for their own reasons.
- Postcondition: `cargo test -p machine-gcode-emit --test machine_gcode_emit_tdd` is fully green; `rg "bytes\[i\] as char"` finds nothing in the file; `ERR_UNRESOLVED_PLACEHOLDER` is the code passed to `ModuleError::fatal` on the new path.
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/machine-gcode-emit/src/lib.rs` — short; readable whole.
  - `modules/core-modules/machine-gcode-emit/tests/machine_gcode_emit_tdd.rs` — the four tests from Step 1 only.
- Files allowed to edit (at most 3):
  - `modules/core-modules/machine-gcode-emit/src/lib.rs`
- Files explicitly out of bounds:
  - `crates/slicer-ir/**`, `crates/slicer-sdk/**`, `crates/slicer-schema/**` — touching any of these invalidates every guest's bindgen and is not required here.
  - `crates/slicer-gcode/**`, `crates/slicer-wasm-host/**`
  - `OrcaSlicerDocumented/**`, `target/**`
- Blast-radius discipline: this step adds no struct field and bumps no schema/version constant, so the struct-literal sweep does not apply. The only cross-file compile fallout is `substitute_placeholders`' signature, and the function is private to `modules/core-modules/machine-gcode-emit/src/lib.rs` with a single call site pair inside `run_gcode_postprocess` — confirmed by the fact that the tests reach it only through the `PostpassModule` trait.
- Expected sub-agent dispatches:
  - Question: how does `GCode::placeholder_parser_process` record a template failure, and which function rethrows it? Scope: `OrcaSlicerDocumented/src/libslic3r/GCode.cpp`; return: `FACT` ≤ 5 lines
  - Question: does `cargo xtask build-guests --check` report `STALE:` after this edit? Scope: cargo run; return: `FACT` clean/stale
- Context cost: `M`
- Authoritative docs:
  - `docs/02_ir_schemas.md` — delegated SUMMARY only, to confirm `PostPass::GCodePostProcess` receives commands + `ConfigView` and no `PrintMetadata`.
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/GCode.cpp` — delegate; never load.
- Verification:
  - `cargo xtask build-guests --check` — FACT clean; rebuild without `--check` if `STALE:` before believing any later test result.
  - `bash -c 'cargo test -p machine-gcode-emit --test machine_gcode_emit_tdd 2>&1 | tee target/log-186-mge.txt | rg "^test result:" | rg -q "^test result: ok\. [1-9]" && echo PASS || echo "FAIL: see target/log-186-mge.txt"'`
  - `bash -c '! rg -q "bytes\[i\] as char" modules/core-modules/machine-gcode-emit/src/lib.rs && rg -q "push_str" modules/core-modules/machine-gcode-emit/src/lib.rs && echo PASS || echo "FAIL: the bytes[i] as char cast is still present, or the literal path no longer uses push_str"'`
  - `bash -c 'rg -q "pub const ERR_UNRESOLVED_PLACEHOLDER" modules/core-modules/machine-gcode-emit/src/lib.rs && rg -q "ModuleError::fatal\(\s*ERR_UNRESOLVED_PLACEHOLDER" modules/core-modules/machine-gcode-emit/src/lib.rs && echo PASS || echo "FAIL: ERR_UNRESOLVED_PLACEHOLDER is absent or not pub, or it is not the code passed to ModuleError::fatal"'` — **verbatim copy of `packet.spec.md` AC-4; if either changes, change both.**
- Exit condition: AC-1, AC-2, AC-3, AC-4, AC-5, AC-6, AC-9, AC-15, AC-N1 and AC-N3 all print PASS.

### Step 3: Declare `nozzle_diameter` and reverse the end-to-end passthrough test

- Task IDs: `TASK-305`
- Objective: add `[config.schema.nozzle_diameter]` to the manifest, add `try_slice_with_raw` to the e2e test file, invert its `unknown_placeholder_passes_through_verbatim`, and add `nozzle_diameter_macro_resolves_end_to_end`.
- Precondition: Step 2 is green and `cargo xtask build-guests --check` is clean.
- Postcondition: `[config.schema]` has exactly five keys; the e2e module is fully green; `fn unknown_placeholder_passes_through_verbatim` is absent from `crates/slicer-runtime/tests/integration/machine_start_end_gcode_emission_tdd.rs`.
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/machine-gcode-emit/machine-gcode-emit.toml` — whole file (short).
  - `modules/core-modules/classic-perimeters/classic-perimeters.toml` — the `[config.schema.nozzle_diameter]` block only.
  - `crates/slicer-runtime/tests/integration/machine_start_end_gcode_emission_tdd.rs` — **long; ranged reads only.** Read `slice_with_raw` and `slice_default` (to add `try_slice_with_raw` beside them), the `count_occurrences` helper, and the two negative tests near the end. Do not load the whole file.
- Files allowed to edit (at most 3):
  - `modules/core-modules/machine-gcode-emit/machine-gcode-emit.toml`
  - `crates/slicer-runtime/tests/integration/machine_start_end_gcode_emission_tdd.rs`
- Files explicitly out of bounds:
  - `modules/core-modules/machine-gcode-emit/src/lib.rs` (Step 2 owns it; no code change is needed for AC-8 — the existing `config.keys()` sweep does the work once the key is declared)
  - `crates/slicer-runtime/src/**`, `crates/slicer-gcode/**`
  - `docs/**` (Step 4 and Step 5 own the docs)
  - `OrcaSlicerDocumented/**`, `target/**`
- Blast-radius discipline: adding a manifest key changes what `slice_with_raw` seeds into `binding_source` / `pipeline_source` — it iterates `machine_binding.module.config_schema().entries` generically, routing `float` defaults into **both** sources, so `nozzle_diameter = 0.4` reaches both the module `ConfigView` and the CONFIG_BLOCK with no harness edit. Confirm before assuming: the count-shaped neighbours are `module_manifest_registers_four_keys_with_expected_types_and_defaults` and `new_keys_appear_in_config_block` (both assert **presence**, not a total) and `gcode_header_thumbnail_config_blocks_tdd`'s "at least 80 key-value lines" **lower bound**. None needs editing; if any turns out to assert a total, it belongs in this step's edit list.
- Expected sub-agent dispatches:
  - Question: after the manifest edit, does `cargo xtask build-guests --check` report `STALE:` for `machine-gcode-emit`? Scope: cargo run; return: `FACT` clean/stale
  - Question: does any test under `crates/slicer-runtime/tests/` assert an exact total number of CONFIG_BLOCK key lines or an exact `machine-gcode-emit` schema length? Scope: `crates/slicer-runtime/tests/**`; return: `LOCATIONS` ≤ 20
- Context cost: `M`
- Authoritative docs:
  - `docs/15_config_keys_reference.md` — the generated `module-config-keys` block boundaries only; ranged read.
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` — `PrintConfigDef::init_fff_params`, delegate; only to confirm `nozzle_diameter` is a real canonical option.
- Verification:
  - `cargo xtask build-guests --check` — FACT clean; rebuild if `STALE:`.
  - `bash -c 'python3 -c "import tomllib; d=tomllib.load(open(r\"modules/core-modules/machine-gcode-emit/machine-gcode-emit.toml\",\"rb\"))[\"config\"][\"schema\"]; k=\"nozzle_diameter\"; ok = k in d and d[k][\"type\"]==\"float\" and abs(d[k][\"default\"]-0.4)<1e-9 and abs(d[k][\"min\"]-0.1)<1e-9 and abs(d[k][\"max\"]-2.0)<1e-9 and d[k].get(\"unit\")==\"mm\" and len(d)==5; print(\"PASS\" if ok else \"FAIL: \"+str(sorted(d)))"'`
  - `bash -c 'cargo test -p slicer-runtime --test integration -- machine_start_end_gcode_emission_tdd:: 2>&1 | tee target/log-186-msege.txt | rg "^test result:" | rg -q "^test result: ok\. [1-9]" && echo PASS || echo "FAIL: see target/log-186-msege.txt"'`
- Exit condition: AC-7, AC-8, AC-10 and AC-N2 all print PASS.

### Step 4: Rewrite the `docs/15` macro contract and regenerate the config-keys block

- Task IDs: `TASK-305`
- Objective: replace §"Machine start / end G-code"'s macro list and passthrough blockquote with the truth — name the four macros a user is expected to reach for (the two temperature keys, `nozzle_diameter`, and the `first_layer_temperature` alias), state the **domain rule** recorded in `docs/adr/0050-custom-gcode-architecture.md` (the placeholder domain is exactly this module's manifest-declared key set plus the alias table) instead of a count, and state the policy in the literal words `unresolved placeholder is a fatal slice error` that AC-11 probes for — then regenerate the `module-config-keys` block so `nozzle_diameter` appears against `machine-gcode-emit`. **Write no numeral.** An earlier draft mandated the sentence `four macros resolve`; that is falsified by the implementation this packet ships (`run_gcode_postprocess`'s `for key in config.keys()` sweep also resolves the manifest's own `machine_start_gcode` / `machine_end_gcode` string keys — six, not four) and it is stale one packet later (187 adds three more keys to the same manifest and rewrites this same section). Note the template-keys-resolve-as-placeholders wrinkle as a residual of the domain rule, which ADR-0050 records; do not special-case it in code.
- Precondition: Step 3 landed, so the manifest is final and `gen-config-docs` will produce a stable table.
- Postcondition: `cargo xtask gen-config-docs --check` exits 0; the phrase "Unknown placeholders pass through verbatim" is gone; the four macro names appear in backticks (the two temperature keys, `nozzle_diameter`, and the `first_layer_temperature` alias); the section contains the literal policy sentence `unresolved placeholder is a fatal slice error` **within the section**, not merely somewhere in the file; and the section claims **no** total number of resolvable macros.
- Files allowed to read, with ranges when over 300 lines:
  - `docs/15_config_keys_reference.md` — **long; ranged reads only.** §"Machine start / end G-code" and the `<!-- BEGIN GENERATED: module-config-keys (cargo xtask gen-config-docs) -->` / `<!-- END GENERATED: module-config-keys -->` marker lines.
- Files allowed to edit (at most 3):
  - `docs/15_config_keys_reference.md`
- Files explicitly out of bounds:
  - Anything between the `module-config-keys` markers — regenerate with `cargo xtask gen-config-docs`, never hand-edit.
  - `docs/ORCA_CONFIG_REFERENCE.md` — deliberately untouched.
  - `docs/DEVIATION_LOG.md`, `docs/07_implementation_status.md` (Step 5 owns them)
  - `modules/**`, `crates/**`
- Expected sub-agent dispatches:
  - Question: does `cargo xtask gen-config-docs --check` exit 0 after regeneration, and does the generated table pair `nozzle_diameter` with `machine-gcode-emit`? Scope: cargo run + `docs/15_config_keys_reference.md`; return: `FACT` pass/fail
- Context cost: `S`
- Authoritative docs:
  - `docs/15_config_keys_reference.md` — the section being rewritten; ranged read.
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` — delegate. **Dispatch against the right structure:** `total_layer_count` and `print_time_sec` are in `PrintStatisticsConfigDef`, `num_extruders` in `OtherSlicingStatesConfigDef`, `print_bed_max` in `DimensionsConfigDef`. Only `max_layer_z` and `layer_num` are in `custom_gcode_specific_placeholders` / `CustomGcodeSpecificConfigDef`. A dispatch that asks the custom-gcode table for the first four returns empty.
- Verification:
  - `bash -c 'python3 -c "import io; s=io.open(r\"docs/15_config_keys_reference.md\",encoding=\"utf-8\").read(); b=chr(96); i=s.find(\"## Machine start / end G-code\"); sec = s[i:] if i>=0 else \"\"; j=sec.find(chr(10)+\"## \",1); sec = sec[:j] if j>0 else sec; want=(\"bed_temperature_initial_layer_single\",\"nozzle_temperature_initial_layer\",\"nozzle_diameter\",\"first_layer_temperature\"); miss=[k for k in want if (b+chr(91)+k+chr(93)+b) not in sec]; pol=\"unresolved placeholder is a fatal slice error\" in sec; ok = i>=0 and (\"Unknown placeholders pass through verbatim\" not in s) and not miss and pol; print(\"PASS\" if ok else \"FAIL: section=\"+str(i>=0)+\", missing macros=\"+str(miss)+\", fatal-policy-sentence=\"+str(pol))"'` — **verbatim copy of `packet.spec.md` AC-11; if either changes, change both.** An earlier draft left the superseded round-1 probe here, so a worker gating on this step used a weaker check than the AC it claims to satisfy.
  - `bash -c 'cargo xtask gen-config-docs --check >/dev/null 2>&1 && echo PASS || echo "FAIL: gen-config-docs --check is red"'`
- Exit condition: AC-11 and AC-12 both print PASS.

### Step 5: File the residual deviation, correct `DEV-085`, register `TASK-305`

- Task IDs: `TASK-305`
- Objective: add one new `DEV-###` row enumerating the **eight** unresolvable macros and their canonical counterparts; correct the two measured errors in the existing `DEV-085` row while leaving it `Open`; hand-add the `TASK-305` backlog row outside the generated block and regenerate that block.
- Precondition: Steps 1-4 complete, so the residual set is final and the doc claims it references are true.
- Postcondition: the residual row exists and names `print_time_estimate_s`, `total_layer_count`, `print_bed_max`, `num_extruders` and `max_layer_z`; the `DEV-085` row states 13 `coString` + 3 `coStrings`; `TASK-305` resolves in `docs/07_implementation_status.md`.
- Files allowed to read, with ranges when over 300 lines:
  - `docs/DEVIATION_LOG.md` — **long; delegate or range-read.** The `DEV-085` row, the two most recent rows for format, and a re-derivation of the highest `DEV-###`.
  - `docs/07_implementation_status.md` — **always delegate.** Needed: the last three `TASK-3xx` row formats and the generated-block marker positions.
- Files allowed to edit (at most 3):
  - `docs/DEVIATION_LOG.md`
  - `docs/07_implementation_status.md`
- Files explicitly out of bounds:
  - The `<!-- BEGIN GENERATED: open-deviations (cargo xtask check-deviations) -->` … `<!-- END GENERATED: open-deviations -->` span of `docs/07_implementation_status.md` — regenerate with `cargo xtask check-deviations`, never hand-edit.
  - `docs/15_config_keys_reference.md` — **hand-edits are out of bounds (Step 4 owns the prose); a generator-owned write is not.** `cargo xtask check-deviations` is mandated by this step, and — verified in `xtask/src/main.rs`'s `check-deviations` arm — when it exits 0 it chains into `gen_config_docs::run(&ws, check_only)`, which is the same code path as `cargo xtask gen-config-docs` and therefore writes `docs/15_config_keys_reference.md`. That is **not** a conflict with the out-of-bounds rule: the write is confined to the three generated marker spans (`module-config-keys`, `host-speeds`, `orca-deviations`), and `gen_config_docs::run` short-circuits with `doc 15 generated sections already current` and performs **no write at all** when the spliced result equals the file on disk. Because Step 4 already ran `gen-config-docs` after the manifest edit, and the `orca-deviations` block is rendered from module manifests plus `orca_defaults` (**not** from `docs/DEVIATION_LOG.md` — verified in `gen_config_docs::run`'s `steps` array), the new `DEV-###` row this step files cannot make doc 15 stale, so this step's run is expected to be a no-op. If it does write, that is a *correct* generator write and the step must not revert it; it is only a defect if the diff falls outside the marker spans.
  - `modules/**`, `crates/**`
- Expected sub-agent dispatches:
  - Question: what is the highest `DEV-###` in `docs/DEVIATION_LOG.md` right now? Scope: `rg -o '^\| DEV-[0-9]{3}' docs/DEVIATION_LOG.md | sort -u | tail -1`; return: `FACT` one line. **Re-derive at the moment of writing — parallel packets file rows concurrently and a number captured earlier in the session will collide.**
  - Question: what row format do the three most recent `TASK-3xx` entries in `docs/07_implementation_status.md` use, and is `TASK-305` present? Scope: `docs/07_implementation_status.md`; return: `FACT` ≤ 5 lines
- Context cost: `S`
- Authoritative docs:
  - `docs/DEVIATION_LOG.md` — delegated; row format and next free ID.
  - `docs/07_implementation_status.md` — delegated; row format.
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` — `PrintConfigDef::init_fff_params` for the corrected 13 `coString` + 3 `coStrings` count; and for the residual row's canonical counterparts, `PrintStatisticsConfigDef` (`total_layer_count`, `print_time_sec`), `OtherSlicingStatesConfigDef` (`num_extruders`), `DimensionsConfigDef` (`print_bed_max`) — **not** `custom_gcode_specific_placeholders`, which contains none of them. Delegate; never load.
  - `OrcaSlicerDocumented/src/libslic3r/GCode.cpp` — `GCode::_do_export`, to confirm that `print_time_sec` is set **twice** (once on a local `DynamicConfig` for `file_start_gcode`, once on the global `placeholder_parser()` before `machine_start_gcode`) and is therefore **not** `file_start_gcode`-only. Delegate.
- Verification:
  - `bash -c 'python3 -c "import io; b=chr(96); p=chr(124); L=io.open(r\"docs/DEVIATION_LOG.md\",encoding=\"utf-8\").read().splitlines(); macros=[b+chr(91)+k+chr(93)+b for k in (\"bed_temperature\",\"filament_type\",\"tool_count\",\"layer_count\",\"print_time_estimate_s\",\"x_max\",\"y_max\",\"z_max\")]; canon=[\"total_layer_count\",\"num_extruders\",\"print_bed_max\",\"print_time_sec\",\"PrintStatisticsConfigDef\",\"OtherSlicingStatesConfigDef\",\"DimensionsConfigDef\"]; need=macros+canon+[\"slice-fatal\",\"placeholder_parser\",\"check_placeholder_parser_failed\"]; rows=[l for l in L if l.startswith(p+chr(32)+\"DEV-\") and not l.startswith(p+chr(32)+\"DEV-085\")]; hit=[l for l in rows if all(t in l for t in need)]; best=max(rows,key=lambda l:sum(t in l for t in need),default=\"\"); print(\"PASS\" if hit else \"FAIL: no single new DEV row carries all tokens; best row misses \"+str([t for t in need if t not in best]))"'` — **verbatim copy of `packet.spec.md` AC-13; if either changes, change both.** An earlier draft left the superseded round-1 probe here, so a worker gating on this step used a weaker check than the AC it claims to satisfy.
  - `bash -c 'python3 -c "import io; p=chr(124); b=chr(96); L=io.open(r\"docs/DEVIATION_LOG.md\",encoding=\"utf-8\").read().splitlines(); r=[l for l in L if l.startswith(p+chr(32)+\"DEV-085\")]; row=(r[0] if r else \"\").replace(b,\"\"); miss=[t for t in (\"13 coString\",\"3 coStrings\",\"filament_change_extrusion_role_gcode\") if t not in row]; print(\"PASS\" if r and not miss else (\"FAIL: no DEV-085 row\" if not r else \"FAIL: DEV-085 row still missing \"+str(miss)))"'` — **verbatim copy of `packet.spec.md` §Doc Impact's `DEV-085` correction probe; if either changes, change both.**
  - `bash -c 'python3 -c "import io; L=io.open(r\"docs/07_implementation_status.md\",encoding=\"utf-8\").read().splitlines(); B=[i for i,l in enumerate(L) if l.startswith(\"<!-- BEGIN GENERATED: open-deviations\")]; E=[i for i,l in enumerate(L) if l.startswith(\"<!-- END GENERATED: open-deviations\")]; H=[i for i,l in enumerate(L) if \"TASK-305\" in l]; print(\"FAIL: open-deviations markers not found\" if not (B and E) else (\"FAIL: TASK-305 not registered anywhere\" if not H else (\"FAIL: TASK-305 appears only INSIDE the generated block\" if all(B[0]<i<E[0] for i in H) else \"PASS\")))"'`
- Exit condition: AC-13 and AC-14 print PASS, and the `DEV-085` correction grep passes.

### Step 6: Closure gates

- Task IDs: `TASK-305`
- Objective: run the workspace check/clippy gates with `--all-targets` and re-dispatch every pipe-suffixed AC command.
- Precondition: Steps 1-5 complete.
- Postcondition: both gates green; every AC command prints PASS.
- Files allowed to read, with ranges when over 300 lines:
  - `target/log-*.txt` — the per-criterion capture files named by each command in `packet.spec.md`; **grep only** (`^test result:`, `FAILED`, `panicked at`), never read whole. Each command writes its own path so two criteria running concurrently cannot clobber each other's evidence; do not collapse them back onto one shared `target/test-output.log`.
- Files allowed to edit (at most 3):
  - none (fix-forward edits belong to the step that owns the file)
- Files explicitly out of bounds:
  - every source and doc file; this step only measures.
- Expected sub-agent dispatches:
  - Question: do `cargo check --workspace --all-targets` and `cargo clippy --workspace --all-targets -- -D warnings` both exit 0? Scope: cargo run; return: `FACT` pass/fail plus ≤ 20 lines of the first error on failure
- Context cost: `S`
- Authoritative docs:
  - none additional.
- OrcaSlicer refs:
  - none.
- Verification:
  - `cargo check --workspace --all-targets`
  - `cargo clippy --workspace --all-targets -- -D warnings`
  - `cargo xtask build-guests --check`
- Exit condition: both gates exit 0, `build-guests --check` reports no `STALE:`, and all eighteen numbered AC commands (AC-1..AC-15, AC-N1..AC-N3) print PASS.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 0 | S | Records `target/pkt-186-baseline-ref.txt`; no repo edit. |
| Step 1 | S | One short test file; one delegated canonical FACT. |
| Step 2 | M | The engine change plus one delegated canonical FACT plus the guest-freshness gate. |
| Step 3 | M | Ranged reads of a long e2e test file plus a manifest edit and a guest rebuild. |
| Step 4 | S | One ranged doc section plus a generator run. |
| Step 5 | S | Two delegated doc reads; no code. |
| Step 6 | S | Measurement only. |

Split before activation if aggregate cost exceeds M or any step is L.

## Packet Completion Gate

- All steps and exits complete.
- Every pipe-suffixed AC command returns PASS.
- `cargo xtask build-guests --check` reports no `STALE:` as the last action before closure.
- Update `docs/07_implementation_status.md` through a worker dispatch, never a full backlog read.
- `DEV-085` stays `Open` — packets 187 and 188 carry the injection-point half. Do not flip it.
- `packet.spec.md` is ready for `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed AC and packet-level gate command.
- Record remaining packet-local risk: the fatal-on-unknown policy is a user-visible reversal with no opt-out; confirm the residual `DEV-###` row states it plainly.
- Confirm context stayed at or below 150k standard, or at/below 300k only with a logged swarm ESCALATION; otherwise record a packet-authoring lesson.

All `cargo check`, `cargo clippy`, and `cargo test` invocations in gate and verification commands must use `--all-targets` where the command accepts it, so the test, bench, and example targets compile.
