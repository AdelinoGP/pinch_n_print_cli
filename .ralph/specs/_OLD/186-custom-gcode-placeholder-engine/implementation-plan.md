# Implementation Plan: 186-custom-gcode-placeholder-engine

## Execution Rules

- Work one atomic step at a time; map every step to grouped task IDs.
- Use TDD, then implementation, then the narrowest falsifying validation.
- Every field below is a context-budget contract and must be filled independently; never write "see Step 1".
- **No step may make an unresolved `[key]` fail the slice.** The rejected
  fatal-on-unresolved policy was reverted after it broke three e2e tests on real
  3MF fixtures. `docs/adr/0050-custom-gcode-architecture.md` now records the
  aligned warn-and-pass decision; implement from that current ADR and this
  packet. See `design.md` §Decisions of Record.

## Steps

### Step 0: Reconstruct the current HEAD baseline at closure

- Task IDs: `TASK-305`
- Objective: at coordinator closure, reconstruct the baseline from the current
  `HEAD` and the closure manifest rather than trusting a stale scratch ref or
  claiming that the coordinated worktree was clean before editing. Every
  untouched-file guard must use that reconstructed closure baseline.
- Precondition: the coordinator has identified the closure `HEAD` and the
  packet's final touched-path manifest; the packet is awaiting final status.
- Postcondition: `target/pkt-186-baseline-ref.txt` is reconstructed at closure
  with the coordinator-approved baseline SHA, and the closure record names the
  paths intentionally changed by this packet. No source, ADR, sibling-packet,
  or deviation-log edit is authorized by this step.
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
   - `bash -c 'BASE=$(git rev-parse HEAD) && mkdir -p target && printf "%s\n" "$BASE" > target/pkt-186-baseline-ref.txt && rg -q "^[0-9a-f]{40}$" target/pkt-186-baseline-ref.txt && git diff --name-only "$BASE" -- .ralph/specs/186-custom-gcode-placeholder-engine | sort -u && echo PASS || echo "FAIL: closure baseline ref was not reconstructed"'` — run by the coordinator at closure, after the touched-path manifest is fixed.
- Exit condition: the closure record contains one approved SHA and the final
  touched-path manifest. Do not substitute a stale pre-coordination ref or
  `git merge-base HEAD master`; the latter includes unrelated branch work and
  the former cannot account for the ADR and sibling-packet migrations.

### Step 1: Red tests for the delivered engine behaviour

- Task IDs: `TASK-305`
- Objective: pin the intended module-level behaviour before touching the engine. Add the `try_run` helper (surfaces the module's `Result` instead of unwrapping it, and returns the partially-filled builder alongside), add `non_ascii_template_text_survives_substitution`, `every_unresolved_placeholder_passes_through_verbatim` and `first_layer_temperature_alias_resolves_to_nozzle_temperature_initial_layer`, and **keep `unknown_placeholder_passes_through_verbatim` asserting passthrough** while re-expressing it through `try_run` so the `Ok` return is asserted explicitly rather than implied by the absence of a panic.
- **Deliberate contrast with the superseded plan:** the previous Step 1 *inverted* `unknown_placeholder_passes_through_verbatim` into `unknown_placeholder_is_a_fatal_module_error` and expected the binary to fail to compile because the new tests named a not-yet-defined `ERR_UNRESOLVED_PLACEHOLDER`. None of that applies. This step names no new module symbol, so the binary **compiles** at the end of it and is red only on assertions.
- Precondition: `cargo test -p machine-gcode-emit --test machine_gcode_emit_tdd` is green (re-derive the test count rather than quoting one).
- Postcondition: the file contains `try_run`, `non_ascii_template_text_survives_substitution`, `every_unresolved_placeholder_passes_through_verbatim`, `first_layer_temperature_alias_resolves_to_nozzle_temperature_initial_layer`, and `unknown_placeholder_passes_through_verbatim` still asserting verbatim passthrough. **No `fn unknown_placeholder_is_a_fatal_module_error` anywhere.** The binary is RED on exactly the new tests (mojibake and the missing alias table).
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/machine-gcode-emit/tests/machine_gcode_emit_tdd.rs` — short; readable whole. The `run` and `raw_texts` helpers are what the new tests reuse. (Do not carry a line count forward — line counts are ledger facts that rot.)
  - `modules/core-modules/machine-gcode-emit/src/lib.rs` — short; readable whole. Read `substitute_placeholders` and `run_gcode_postprocess` only, to name the behaviour being asserted.
- Files allowed to edit (at most 3):
  - `modules/core-modules/machine-gcode-emit/tests/machine_gcode_emit_tdd.rs`
- Files explicitly out of bounds:
  - `modules/core-modules/machine-gcode-emit/src/lib.rs` (Step 2 owns it)
  - `crates/slicer-runtime/**` (Step 3 owns the pipeline half)
  - `docs/adr/0050-custom-gcode-architecture.md` — read-only and aligned; use it
    as the current warn-and-pass authority
  - `OrcaSlicerDocumented/**`, `target/**`
- Expected sub-agent dispatches:
  - Question: in `GCode::update_placeholder_parser_with_variant_params`, is `first_layer_temperature` set unconditionally, and what is the verbatim comment above it? Scope: `src/libslic3r/GCode.cpp` in the **sibling checkout** `F:\slicerProject\pinch_n_print_cli_2\OrcaSlicerDocumented` — this repo's `OrcaSlicerDocumented/` has **no `GCode.cpp`**; return: `FACT` ≤ 5 lines
- Context cost: `S`
- Authoritative docs:
  - `docs/15_config_keys_reference.md` §"Machine start / end G-code" — ranged read only; establishes the contract being pinned.
- OrcaSlicer refs:
  - `src/libslic3r/GCode.cpp` (sibling checkout) — `GCode::update_placeholder_parser_with_variant_params`. Delegate; never load.
- Verification:
  - `bash -c 'mkdir -p target; cargo test -p machine-gcode-emit --test machine_gcode_emit_tdd >target/log-186-mge-red.txt 2>&1; if rg -q "^test result:" target/log-186-mge-red.txt; then rg -q "FAILED" target/log-186-mge-red.txt && echo "RED (assertion failures — expected)" || echo "NOT RED — the binary compiled and every test passed; the new tests are not discriminating"; else rg -q "^error(\[|:)" target/log-186-mge-red.txt && echo "RED (compile failure — investigate: this step introduces no new module symbol, so a compile error is NOT expected here)" || echo "NOT RED — no test-result line and no compiler error; the run did not happen"; fi'` — read `target/log-186-mge-red.txt` rather than re-running.
  - `bash -c '! rg -q "fn unknown_placeholder_is_a_fatal_module_error" modules/core-modules/machine-gcode-emit/tests/machine_gcode_emit_tdd.rs && rg -q "fn unknown_placeholder_passes_through_verbatim" modules/core-modules/machine-gcode-emit/tests/machine_gcode_emit_tdd.rs && echo PASS || echo "FAIL: the reverted fatal test is present, or the passthrough test is missing"'`
- Exit condition: the three new tests exist, `try_run` exists, the passthrough test still asserts passthrough, and the binary compiles and fails on exactly the new tests. A **compile** failure here is a defect, not a valid RED (unlike the superseded plan): this step names no symbol the module does not already export.

### Step 2: Fix the engine — char-boundary-correct literals, alias table, collect-and-warn

- Task IDs: `TASK-305`
- Objective: change `substitute_placeholders` to `(&str, &HashMap<String, String>) -> (String, Vec<String>)` with slice-based literal copying and no `as char`; add `const PLACEHOLDER_ALIASES: &[(&str, &str)]` with its single `first_layer_temperature` → `nozzle_temperature_initial_layer` entry (applied **after** the `config.keys()` sweep, never as a manifest key); and make `run_gcode_postprocess` union the two templates' unresolved keys into a `BTreeSet<String>`, emit exactly **one** `slicer_sdk::host::log_warn` naming every key and every contributing injection point, then **proceed to emission and return `Ok`**.
- **Explicitly NOT in this step, and not in any step:** `ERR_UNRESOLVED_PLACEHOLDER`, a `ModuleError::fatal` call on the placeholder path, a `sites_clause` message helper, or any early return before `push_raw`. All three were built by the superseded plan and deleted on reversal; AC-4 asserts their absence.
- Precondition: Step 1's tests exist, compile, and are red on their own assertions.
- Postcondition: `cargo test -p machine-gcode-emit --test machine_gcode_emit_tdd` is fully green; `rg "bytes\[i\] as char"` finds nothing in the file; `slicer_sdk::host::log_warn` is called exactly once in the file; `ERR_UNRESOLVED_PLACEHOLDER` appears nowhere.
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/machine-gcode-emit/src/lib.rs` — short; readable whole.
  - `modules/core-modules/machine-gcode-emit/tests/machine_gcode_emit_tdd.rs` — the Step 1 tests only.
  - `crates/slicer-sdk/src/host.rs` — **grep only** for `pub fn log_warn`; do not browse.
- Files allowed to edit (at most 3):
  - `modules/core-modules/machine-gcode-emit/src/lib.rs`
- Files explicitly out of bounds:
  - `crates/slicer-ir/**`, `crates/slicer-sdk/**`, `crates/slicer-schema/**` — touching any of these invalidates every guest's bindgen and is not required here.
  - `crates/slicer-gcode/**`, `crates/slicer-wasm-host/**`
  - `docs/adr/0050-custom-gcode-architecture.md` — read-only and aligned; do not
    edit it from this packet
  - `OrcaSlicerDocumented/**`, `target/**`
- Blast-radius discipline: this step adds no struct field and bumps no schema/version constant, so the struct-literal sweep does not apply. The only cross-file compile fallout is `substitute_placeholders`' signature, and the function is private to `modules/core-modules/machine-gcode-emit/src/lib.rs` with a single call-site pair inside `run_gcode_postprocess` — the tests reach it only through the `PostpassModule` trait.
- Expected sub-agent dispatches:
  - Question: how does `GCode::placeholder_parser_process` record a template failure, which function rethrows it, and does the export continue in between? Scope: `src/libslic3r/GCode.cpp` in the **sibling checkout** (this repo has no `GCode.cpp`); return: `FACT` ≤ 5 lines
  - Question: does `cargo xtask build-guests --check` report `STALE:` after this edit? Scope: cargo run; return: `FACT` clean/stale
- Context cost: `M`
- Authoritative docs:
  - `docs/02_ir_schemas.md` — delegated SUMMARY only, to confirm `PostPass::GCodePostProcess` receives commands + `ConfigView` and no `PrintMetadata`.
- OrcaSlicer refs:
  - `src/libslic3r/GCode.cpp` (sibling checkout) — `GCode::placeholder_parser_process`, `GCode::check_placeholder_parser_failed`. Delegate; never load. Borrowed for the **aggregation**, not the throw.
- Verification:
  - `cargo xtask build-guests --check` — FACT clean; rebuild without `--check` if `STALE:` before believing any later test result.
  - `bash -c 'mkdir -p target && cargo test -p machine-gcode-emit --test machine_gcode_emit_tdd 2>&1 | tee target/log-186-mge.txt | rg "^test result:" | rg -q "^test result: ok\. [1-9]" && echo PASS || echo "FAIL: see target/log-186-mge.txt"'`
  - `bash -c '! rg -q "bytes\[i\] as char" modules/core-modules/machine-gcode-emit/src/lib.rs && rg -q "push_str" modules/core-modules/machine-gcode-emit/src/lib.rs && echo PASS || echo "FAIL: the bytes[i] as char cast is still present, or the literal path no longer uses push_str"'` — **verbatim copy of `packet.spec.md` AC-2; if either changes, change both.**
  - `bash -c 'rg -q "slicer_sdk::host::log_warn" modules/core-modules/machine-gcode-emit/src/lib.rs && ! rg -q "ERR_UNRESOLVED_PLACEHOLDER" modules/core-modules/machine-gcode-emit/src/lib.rs && echo PASS || echo "FAIL: the aggregated warn path is absent, or the reverted ERR_UNRESOLVED_PLACEHOLDER constant is back"'` — **verbatim copy of `packet.spec.md` AC-4; if either changes, change both.**
- Exit condition: AC-1, AC-2, AC-3, AC-4, AC-5, AC-6, AC-9, AC-15, AC-N1 and AC-N3 all print PASS.

### Step 3: Declare `nozzle_diameter` and add the pipeline-level pins

- Task IDs: `TASK-305`
- Objective: add `[config.schema.nozzle_diameter]` to the manifest; add `try_slice_with_raw(raw) -> Result<String, PipelineError>` beside `slice_with_raw` and re-express `slice_with_raw` as `try_slice_with_raw(raw).expect("pipeline must succeed")`; add `nozzle_diameter_macro_resolves_end_to_end`; and **keep `unknown_placeholder_passes_through_verbatim` in that file asserting that the bracketed text reaches the emitted start block**, routed through `try_slice_with_raw` so a pipeline failure reports the error text instead of panicking on a bare `expect`.
- `try_slice_with_raw` is retained even though no criterion in this packet needs its fallibility: **packets 187 and 188 both cite it as a forward dependency on this step.**
- Precondition: Step 2 is green and `cargo xtask build-guests --check` is clean.
- Postcondition: `[config.schema]` has exactly five keys; the `machine_start_end_gcode_emission_tdd` module is fully green; `fn unknown_placeholder_is_a_fatal_slice_error` is absent from `crates/slicer-runtime/tests/integration/machine_start_end_gcode_emission_tdd.rs`.
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/machine-gcode-emit/machine-gcode-emit.toml` — whole file (short).
  - `modules/core-modules/classic-perimeters/classic-perimeters.toml` — the `[config.schema.nozzle_diameter]` block only.
  - `crates/slicer-runtime/tests/integration/machine_start_end_gcode_emission_tdd.rs` — **long; ranged reads only.** Read `slice_with_raw` / `slice_default` (to add `try_slice_with_raw` beside them), the `count_occurrences` helper, and the negative tests near the end. Do not load the whole file.
- Files allowed to edit (at most 3):
  - `modules/core-modules/machine-gcode-emit/machine-gcode-emit.toml`
  - `crates/slicer-runtime/tests/integration/machine_start_end_gcode_emission_tdd.rs`
- Files explicitly out of bounds:
  - `modules/core-modules/machine-gcode-emit/src/lib.rs` (Step 2 owns it for the engine; Step 4 owns the `format_placeholder_value` fix)
  - `crates/slicer-runtime/src/**`, `crates/slicer-gcode/**`
  - `crates/slicer-runtime/tests/e2e/**` — Step 4 runs those suites; nothing may edit them
  - `docs/**` (Steps 5 and 6 own the docs)
  - `OrcaSlicerDocumented/**`, `target/**`
- Blast-radius discipline: adding a manifest key changes what `slice_with_raw` seeds into `binding_source` / `pipeline_source` — it iterates `machine_binding.module.config_schema().entries` generically, routing `float` defaults into **both** sources, so `nozzle_diameter = 0.4` reaches both the module `ConfigView` and the CONFIG_BLOCK with no harness edit. The count-shaped neighbour is `module_manifest_registers_five_keys_with_expected_types_and_defaults`; it asserts the exact five-key CONFIG_BLOCK surface, including `nozzle_diameter`, alongside `new_keys_appear_in_config_block`. `gcode_header_thumbnail_config_blocks_tdd`'s "at least 80 key-value lines" **lower bound** is also unaffected. None needs editing.
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
   - `bash -c 'python3 -c "import tomllib; d=tomllib.load(open(r\"modules/core-modules/machine-gcode-emit/machine-gcode-emit.toml\",\"rb\"))[\"config\"][\"schema\"]; expected={\"machine_start_gcode\",\"machine_end_gcode\",\"bed_temperature_initial_layer_single\",\"nozzle_temperature_initial_layer\",\"nozzle_diameter\"}; k=\"nozzle_diameter\"; ok=set(d)==expected and d[k][\"type\"]==\"float\" and isinstance(d[k][\"default\"],float) and abs(d[k][\"default\"]-0.4)<1e-9 and abs(d[k][\"min\"]-0.1)<1e-9 and abs(d[k][\"max\"]-2.0)<1e-9 and d[k].get(\"unit\")==\"mm\"; print(\"PASS\" if ok else \"FAIL: schema key set or nozzle_diameter fields are wrong (found \"+str(sorted(d))+\")\")"'` — **verbatim copy of `packet.spec.md` AC-7; if either changes, change both.**
  - `bash -c 'mkdir -p target && cargo test -p slicer-runtime --test integration -- machine_start_end_gcode_emission_tdd:: 2>&1 | tee target/log-186-msege.txt | rg "^test result:" | rg -q "^test result: ok\. [1-9]" && echo PASS || echo "FAIL: see target/log-186-msege.txt"'`
  - `bash -c '! rg -q "fn unknown_placeholder_is_a_fatal_slice_error" crates/slicer-runtime/tests/integration/machine_start_end_gcode_emission_tdd.rs && rg -q "fn try_slice_with_raw" crates/slicer-runtime/tests/integration/machine_start_end_gcode_emission_tdd.rs && echo PASS || echo "FAIL: the reverted fatal e2e test is present, or try_slice_with_raw is missing (187/188 depend on it)"'`
- Exit condition: AC-7, AC-8, AC-10 and AC-N2 all print PASS.

### Step 4: Resolve list-valued config keys, and prove it on real 3MF fixtures

- Task IDs: `TASK-305`
- Objective: two things that belong together because the first is only observable through the second.
  1. Extend `format_placeholder_value` in `modules/core-modules/machine-gcode-emit/src/lib.rs` so `ConfigValue::List` resolves from its **first element**, recursively, and an **empty** `List` yields `None` (so the key stays out of the lookup and its placeholder passes through verbatim rather than collapsing to an empty string). Add the two module tests `list_valued_config_key_resolves_from_first_element` (AC-16) and `empty_list_config_key_passes_through_verbatim` (AC-17).
  2. Run the **fixture-level e2e gate** (AC-18): build `pnp-cli`, then slice two committed OrcaSlicer-authored 3MFs end to end and assert both suites are green.
- **Why this is its own step, and why the two halves are inseparable.** The list fix is adversarial-review finding **F2**: without it `[nozzle_diameter]` — this packet's headline recovered macro — is **inert for every real slice**, because real 3MF input supplies per-extruder settings as vectors (`['0.4']`), never as scalars. Step 3's AC-8 exercises the *schema default*, a `Float`, and stayed green throughout. Only a real fixture distinguishes them. The same blindness, one level up, is what let the reverted fatal policy pass 18/18 while breaking three e2e tests: **not one criterion in the superseded set ever sliced a real fixture.** This step exists so that can never be true again.
- Precondition: Step 3 landed; the manifest declares `nozzle_diameter`; `cargo xtask build-guests --check` is clean.
- Postcondition: `format_placeholder_value` handles `List`; both new module tests pass; `cargo build -p pnp-cli` succeeds; `modifier_infill_tdd::` and `cube_painted_e2e_tdd::` are both green under `--test e2e`.
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/machine-gcode-emit/src/lib.rs` — short; readable whole. Read `format_placeholder_value` and the `config.keys()` sweep in `run_gcode_postprocess`.
  - `modules/core-modules/machine-gcode-emit/tests/machine_gcode_emit_tdd.rs` — the `try_run` and `raw_texts` helpers only.
  - `crates/slicer-runtime/tests/e2e/modifier_infill_tdd.rs`, `crates/slicer-runtime/tests/e2e/cube_painted_e2e_tdd.rs` — **fixture-path helpers only** (`cube_cilindrical_modifier_3mf`, `cube_4color_3mf`), to confirm which archive each slices. Do not browse; do not edit.
- Files allowed to edit (at most 3):
  - `modules/core-modules/machine-gcode-emit/src/lib.rs`
  - `modules/core-modules/machine-gcode-emit/tests/machine_gcode_emit_tdd.rs`
- Files explicitly out of bounds:
  - `crates/slicer-runtime/tests/e2e/**` — **run, never edit.** Adapting an e2e suite to accommodate this packet would recreate exactly the blindness this step removes.
  - `resources/*.3mf` — the fixtures are the evidence; never regenerate or re-export them to make a test pass.
  - `crates/slicer-runtime/src/**`, `crates/slicer-gcode/**`
  - `docs/**`, `docs/adr/**`
  - `OrcaSlicerDocumented/**`, `target/**`
- Blast-radius discipline: `format_placeholder_value` is a private free function in one file with a single call site (the `config.keys()` sweep). Widening its accepted `ConfigValue` set can only add lookup entries, never remove them, so it cannot un-resolve a key that resolved before. The one behaviour it must **not** acquire is rendering an empty `List` as `""` — AC-17 is the guard.
- Expected sub-agent dispatches:
  - Question: in `GCode::_do_export`'s `; first_layer_temperature = %d` preamble, does canonical read element 0 of the per-extruder `nozzle_temperature_initial_layer` vector? Scope: `src/libslic3r/GCode.cpp` in the **sibling checkout** `F:\slicerProject\pinch_n_print_cli_2\OrcaSlicerDocumented` (this repo has no `GCode.cpp`); return: `FACT` ≤ 3 lines
  - Question: after `cargo build -p pnp-cli`, are `modifier_infill_tdd::` and `cube_painted_e2e_tdd::` both green under `cargo test -p slicer-runtime --test e2e`? Scope: cargo run; return: `FACT` pass/fail plus the two `^test result:` lines
- Context cost: `M`
- Authoritative docs:
  - none additional.
- OrcaSlicer refs:
  - `src/libslic3r/GCode.cpp` (sibling checkout) — `GCode::_do_export`. Delegate; never load.
- Verification:
  - `cargo xtask build-guests --check` — FACT clean; rebuild if `STALE:`. **Mandatory: this step edits `modules/core-modules/machine-gcode-emit/src/**`, and the e2e suites instantiate the real component.**
  - `bash -c 'cargo test -p machine-gcode-emit --test machine_gcode_emit_tdd -- list_valued_config_key_resolves_from_first_element --exact 2>&1 | rg "^test result:" | rg -q "^test result: ok\. [1-9]" && echo PASS || echo "FAIL: list_valued_config_key_resolves_from_first_element did not run or did not pass"'` — **verbatim copy of `packet.spec.md` AC-16.**
  - `bash -c 'cargo test -p machine-gcode-emit --test machine_gcode_emit_tdd -- empty_list_config_key_passes_through_verbatim --exact 2>&1 | rg "^test result:" | rg -q "^test result: ok\. [1-9]" && echo PASS || echo "FAIL: empty_list_config_key_passes_through_verbatim did not run or did not pass"'` — **verbatim copy of `packet.spec.md` AC-17.**
  - `bash -c 'mkdir -p target && cargo build -p pnp-cli >/dev/null 2>&1 || { echo "FAIL: cargo build -p pnp-cli failed; the e2e harness cannot run"; exit 1; }; cargo test -p slicer-runtime --test e2e -- modifier_infill_tdd:: 2>&1 | tee target/log-186-e2e-modifier.txt | rg "^test result:" | rg -q "^test result: ok\. [1-9]" || { echo "FAIL: modifier_infill_tdd (cube_cilindrical_modifier.3mf) is not green — see target/log-186-e2e-modifier.txt"; exit 1; }; cargo test -p slicer-runtime --test e2e -- cube_painted_e2e_tdd:: 2>&1 | tee target/log-186-e2e-painted.txt | rg "^test result:" | rg -q "^test result: ok\. [1-9]" || { echo "FAIL: cube_painted_e2e_tdd (cube_4color.3mf) is not green — see target/log-186-e2e-painted.txt"; exit 1; }; echo PASS'` — **verbatim copy of `packet.spec.md` AC-18; if either changes, change both.** The `cargo build -p pnp-cli` is inside the command deliberately: `slicer_test_support::pnp_cli_bin`'s staleness guard **panics** on an absent or stale binary and has no cross-profile fallback, and that panic reads as an e2e regression when it is not one.
- Exit condition: AC-16, AC-17 and AC-18 all print PASS, and AC-9 (whole module binary) is still green after the two added tests.

### Step 5: Rewrite the `docs/15` macro contract and regenerate the config-keys block

- Task IDs: `TASK-305`
- Objective: rewrite §"Machine start / end G-code" to state (a) the **domain rule** — a `[key]` resolves iff it is declared in this module's own manifest or is a legacy name in the placeholder-alias table — rather than a count; (b) the macros a user is expected to reach for (`[bed_temperature_initial_layer_single]`, `[nozzle_temperature_initial_layer]`, `[nozzle_diameter]`, and the alias `[first_layer_temperature]`); and (c) the **warn-and-pass policy** in the literal words `passes through verbatim and is warned about`, with the modularity reason and the consequence that bracketed text can reach the printer. Then regenerate the `module-config-keys` block so `nozzle_diameter` appears against `machine-gcode-emit`.
- **The superseded plan mandated the opposite sentence** (`unresolved placeholder is a fatal slice error`). That string must appear nowhere in the file. Also note the escape-syntax paragraph inverts: under warn-and-pass there is no escape syntax **and none is needed**, because a bracketed non-key is simply left alone.
- **Write no numeral.** `run_gcode_postprocess`'s `config.keys()` sweep also resolves the manifest's own `machine_start_gcode` / `machine_end_gcode` string keys, so any count is wrong on this packet's own implementation; and packet 187 adds three more keys to the same manifest and rewrites this same section. Note the template-keys-resolve-inside-their-own-templates wrinkle as a residual of the domain rule; do not special-case it in code.
- Precondition: Steps 3 and 4 landed, so the manifest is final and `gen-config-docs` will produce a stable table.
- Postcondition: `cargo xtask gen-config-docs --check` exits 0; the four macro names appear in backticked-bracket form within the section; the section carries the literal phrase `passes through verbatim and is warned about`; `fatal slice error` appears nowhere in the file; and the section claims no total number of resolvable macros.
- Files allowed to read, with ranges when over 300 lines:
  - `docs/15_config_keys_reference.md` — **long; ranged reads only.** §"Machine start / end G-code" (its heading on disk is `## Machine start / end G-code (packet 59)`) and the `<!-- BEGIN GENERATED: module-config-keys (cargo xtask gen-config-docs) -->` / `<!-- END GENERATED: module-config-keys -->` marker lines.
- Files allowed to edit (at most 3):
  - `docs/15_config_keys_reference.md`
- Files explicitly out of bounds:
  - Anything between the `module-config-keys` markers — regenerate with `cargo xtask gen-config-docs`, never hand-edit.
  - `docs/adr/0050-custom-gcode-architecture.md` — rewrite in place as the
    current authority; verify the warn-and-pass clauses and D-ADR row.
  - `docs/ORCA_CONFIG_REFERENCE.md` — deliberately untouched.
  - `docs/DEVIATION_LOG.md`, `docs/07_implementation_status.md` (Step 6 owns them)
  - `modules/**`, `crates/**`
- Expected sub-agent dispatches:
  - Question: does `cargo xtask gen-config-docs --check` exit 0 after regeneration, and does the generated table pair `nozzle_diameter` with `machine-gcode-emit`? Scope: cargo run + `docs/15_config_keys_reference.md`; return: `FACT` pass/fail
- Context cost: `S`
- Authoritative docs:
  - `docs/15_config_keys_reference.md` — the section being rewritten; ranged read.
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` — delegate. **Dispatch against the right structure:** `total_layer_count` and `print_time_sec` are in `PrintStatisticsConfigDef`, `num_extruders` in `OtherSlicingStatesConfigDef`, `print_bed_max` in `DimensionsConfigDef`. Only `max_layer_z` and `layer_num` are in `s_CustomGcodeSpecificPlaceholders` / `CustomGcodeSpecificConfigDef`. A dispatch that asks the custom-gcode table for the first four returns empty.
- Verification:
  - `bash -c 'python3 -c "import io; s=io.open(r\"docs/15_config_keys_reference.md\",encoding=\"utf-8\").read(); b=chr(96); i=s.find(\"## Machine start / end G-code\"); sec = s[i:] if i>=0 else \"\"; j=sec.find(chr(10)+\"## \",1); sec = sec[:j] if j>0 else sec; want=(\"bed_temperature_initial_layer_single\",\"nozzle_temperature_initial_layer\",\"nozzle_diameter\",\"first_layer_temperature\"); miss=[k for k in want if (b+chr(91)+k+chr(93)+b) not in sec]; pol=\"passes through verbatim and is warned about\" in sec; nofatal=\"fatal slice error\" not in s; ok = i>=0 and not miss and pol and nofatal; print(\"PASS\" if ok else \"FAIL: section=\"+str(i>=0)+\", missing macros=\"+str(miss)+\", warn-policy-sentence=\"+str(pol)+\", fatal-sentence-absent=\"+str(nofatal))"'` — **verbatim copy of `packet.spec.md` AC-11; if either changes, change both.** An earlier draft left a superseded probe here, so a worker gating on this step used a weaker check than the AC it claims to satisfy.
  - `bash -c 'cargo xtask gen-config-docs --check >/dev/null 2>&1 && echo PASS || echo "FAIL: gen-config-docs --check is red"'`
- Exit condition: AC-11 and AC-12 both print PASS.

### Step 6: File the residual deviation, reconcile the retired aggregate label, register `TASK-305`

- Task IDs: `TASK-305`
- Objective: maintain the surviving `DEV-100` row enumerating the **eight** unresolvable macros and their canonical counterparts, the domain asymmetry, and the warn-and-pass policy; record the closed `D-<n>-ADR-0050-AMENDED` row; preserve the deleted aggregate custom-G-code label's absence; hand-add the `TASK-305` backlog row outside the generated block and regenerate that block.
- **The `slice-fatal` token the superseded plan required is now forbidden** — the AC-13 probe fails a row that carries it. The eight macros are warned verbatim text, not rejected input.
- Precondition: Steps 1-5 complete, so the residual set is final and the doc claims it references are true.
- Postcondition: the surviving `DEV-100` row exists and names `print_time_estimate_s`, `total_layer_count`, `print_bed_max`, `num_extruders`, `max_layer_z`, `log_warn`, `reverted` and `check_placeholder_parser_failed`; the deleted aggregate row remains absent; `TASK-305` resolves in `docs/07_implementation_status.md` with warn-and-pass text.
- Files allowed to read, with ranges when over 300 lines:
  - `docs/DEVIATION_LOG.md` — **long; delegate or range-read.** The surviving `DEV-100` placeholder row and current custom-G-code residual rows, plus a re-derivation of the highest `DEV-###`.
  - `docs/07_implementation_status.md` — **always delegate.** Needed: the last three `TASK-3xx` row formats and the generated-block marker positions.
- Files allowed to edit (at most 3):
  - `docs/DEVIATION_LOG.md`
  - `docs/07_implementation_status.md`
- Files explicitly out of bounds:
  - The `<!-- BEGIN GENERATED: open-deviations (cargo xtask check-deviations) -->` … `<!-- END GENERATED: open-deviations -->` span of `docs/07_implementation_status.md` — regenerate with `cargo xtask check-deviations`, never hand-edit.
  - `docs/adr/0050-custom-gcode-architecture.md` — aligned external authority;
    do not edit it from this packet. The relationship is recorded in `design.md`
    §Decisions of Record.
  - `docs/15_config_keys_reference.md` — **hand-edits are out of bounds (Step 5 owns the prose); a generator-owned write is not.** `cargo xtask check-deviations` is mandated by this step, and — verified in `xtask/src/main.rs`'s `check-deviations` arm — when it exits 0 it chains into `gen_config_docs::run(&ws, check_only)`, the same code path as `cargo xtask gen-config-docs`, which writes `docs/15_config_keys_reference.md`. That is **not** a conflict with the out-of-bounds rule: the write is confined to the generated marker spans (`module-config-keys`, `host-speeds`, `orca-deviations`), and `gen_config_docs::run` short-circuits with `doc 15 generated sections already current` and performs **no write at all** when the spliced result equals the file on disk. Because Step 5 already ran `gen-config-docs` after the manifest edit, and the `orca-deviations` block is rendered from module manifests plus `orca_defaults` (**not** from `docs/DEVIATION_LOG.md`), the new `DEV-###` row this step files cannot make doc 15 stale, so this step's run is expected to be a no-op. If it does write, that is a *correct* generator write and the step must not revert it; it is only a defect if the diff falls outside the marker spans.
  - `modules/**`, `crates/**`
- Expected sub-agent dispatches:
  - Question: what is the highest `DEV-###` in `docs/DEVIATION_LOG.md` right now? Scope: `rg -o '^\| DEV-[0-9]{3}' docs/DEVIATION_LOG.md | sort -u | tail -1`; return: `FACT` one line. **Re-derive at the moment of writing — parallel packets file rows concurrently and a number captured earlier in the session will collide.**
  - Question: what row format do the three most recent `TASK-3xx` entries in `docs/07_implementation_status.md` use, and is `TASK-305` present? Scope: `docs/07_implementation_status.md`; return: `FACT` ≤ 5 lines
- Context cost: `S`
- Authoritative docs:
  - `docs/DEVIATION_LOG.md` — delegated; row format and next free ID.
  - `docs/07_implementation_status.md` — delegated; row format.
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` — `PrintConfigDef::init_fff_params` for the corrected 13 `coString` + 3 `coStrings` count; and for the residual row's canonical counterparts, `PrintStatisticsConfigDef` (`total_layer_count`, `print_time_sec`), `OtherSlicingStatesConfigDef` (`num_extruders`), `DimensionsConfigDef` (`print_bed_max`) — **not** `s_CustomGcodeSpecificPlaceholders`, which contains none of them. Delegate; never load.
  - `src/libslic3r/GCode.cpp` (**sibling checkout only**) — `GCode::_do_export`, to confirm that `print_time_sec` is set **twice** (once on a local `DynamicConfig` for `file_start_gcode`, once on the global `placeholder_parser()` before `machine_start_gcode`) and is therefore **not** `file_start_gcode`-only; and `GCode::placeholder_parser_process` / `GCode::check_placeholder_parser_failed` for the divergence clause. Delegate.
- Verification:
   - `bash -c 'python3 -c "import io; b=chr(96); p=chr(124); L=io.open(r\"docs/DEVIATION_LOG.md\",encoding=\"utf-8\").read().splitlines(); macros=[b+chr(91)+k+chr(93)+b for k in (\"bed_temperature\",\"filament_type\",\"tool_count\",\"layer_count\",\"print_time_estimate_s\",\"x_max\",\"y_max\",\"z_max\")]; terms=macros+[\"TemperaturesConfigDef\",\"get_bed_temp_key\",\"PrintConfigDef\",\"num_extruders\",\"OtherSlicingStatesConfigDef\",\"total_layer_count\",\"PrintStatisticsConfigDef\",\"print_time_sec\",\"print_bed_max\",\"DimensionsConfigDef\",\"printable_height\",\"max_layer_z\",\"log_warn\",\"reverted\",\"check_placeholder_parser_failed\"]; rows=[l for l in L if l.startswith(p+chr(32)+\"DEV-\")]; residual=[l for l in rows if all(t in l for t in macros)]; retired=any(l.startswith(p+\" custom-G-code injection deviation \") for l in L); ok=len(residual)==1 and residual[0].startswith(p+chr(32)+\"DEV-100\") and all(t in residual[0] for t in terms) and \"slice-fatal\" not in residual[0] and not retired; print(\"PASS\" if ok else \"FAIL: DEV-100 evidence or deleted-row absence is wrong\")"'` — **verbatim copy of `packet.spec.md` AC-13; if either changes, change both.**
   - `bash -c '! rg -q "^\| custom-G-code injection deviation " docs/DEVIATION_LOG.md && rg -q "^\| DEV-100 " docs/DEVIATION_LOG.md && rg -q "^\| DEV-103 " docs/DEVIATION_LOG.md && rg -q "^\| DEV-104 " docs/DEVIATION_LOG.md && rg -q "^\| DEV-105 " docs/DEVIATION_LOG.md && echo PASS || echo FAIL'` — the post-purge replacement for the deleted aggregate-row probe; it does not assume a unique number for packet 187's residual row.
   - `bash -c 'python3 -c "import io; L=io.open(r\"docs/07_implementation_status.md\",encoding=\"utf-8\").read().splitlines(); B=[i for i,l in enumerate(L) if l.startswith(\"<!-- BEGIN GENERATED: open-deviations\")]; E=[i for i,l in enumerate(L) if l.startswith(\"<!-- END GENERATED: open-deviations\")]; H=[(i,l) for i,l in enumerate(L) if l.startswith(\"- [\") and \"TASK-305\" in l]; ok=len(B)==1 and len(E)==1 and len(H)==1 and not (B[0]<H[0][0]<E[0]) and all(t in H[0][1] for t in (\"Delivered\",\"warn-and-pass\",\"reverted before landing\")); print(\"PASS\" if ok else \"FAIL: expected exactly one delivered TASK-305 row outside generated markers\")"'` — **verbatim copy of `packet.spec.md` AC-14.**
 - Exit condition: AC-13 and AC-14 print PASS, and the post-purge custom-G-code ledger probe passes.

### Step 7: Closure gates

- Task IDs: `TASK-305`
- Objective: run every workspace, guest, CLI, generated-doc, untouched-file,
  and acceptance gate, then re-dispatch the full **21-AC matrix**: AC-1 through
  AC-18 and AC-N1 through AC-N3. AC-18 remains the final behavioral fixture
  gate, but it is not a substitute for the other twenty commands.
- Precondition: Steps 0-6 complete.
- Postcondition: all gates are green and every command in the full 21-AC matrix
  prints PASS. The packet front matter is changed to `status: implemented` at
  final closure.
- Files allowed to read, with ranges when over 300 lines:
  - `target/log-*.txt` — the per-criterion capture files named by each command in `packet.spec.md`; **grep only** (`^test result:`, `FAILED`, `panicked at`), never read whole. Each command writes its own path so two criteria running concurrently cannot clobber each other's evidence; do not collapse them back onto one shared `target/test-output.log`.
- Files allowed to edit (at most 3):
  - none (fix-forward edits belong to the step that owns the file)
- Files explicitly out of bounds:
  - every source and doc file; this step only measures.
- Expected sub-agent dispatches:
  - Question: do `cargo check --workspace --all-targets` and `cargo clippy --workspace --all-targets -- -D warnings` both exit 0? Scope: cargo run; return: `FACT` pass/fail plus ≤ 20 lines of the first error on failure
  - Question: do `cargo xtask build-guests --check` and `cargo build -p pnp-cli` both succeed with no `STALE:`? Scope: cargo run; return: `FACT` clean/stale
- Context cost: `S`
- Authoritative docs:
  - none additional.
- OrcaSlicer refs:
  - none.
- Verification:
   - `cargo check --workspace --all-targets`
   - `cargo clippy --workspace --all-targets -- -D warnings`
   - `cargo xtask build-guests --check`
   - `cargo build -p pnp-cli`
   - The complete AC-1..AC-18 and AC-N1..AC-N3 command matrix, including all
     behavioral, generated-doc, deviation-row, task-row, and untouched-file
     probes; run AC-18 **last** as the real-3MF gate.
- Exit condition: both workspace gates exit 0, `build-guests --check` reports
  no `STALE:`, `cargo build -p pnp-cli` succeeds, every one of the **twenty-one**
  AC commands prints PASS, and the packet front matter is `implemented`.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 0 | S | Reconstructs the current `HEAD` baseline and touched-path manifest at coordinator closure; no repo edit. |
| Step 1 | S | One short test file; one delegated canonical FACT. Compiles at the end — a compile error here is a defect, not a valid RED. |
| Step 2 | M | The engine change plus one delegated canonical FACT plus the guest-freshness gate. |
| Step 3 | M | Ranged reads of a long pipeline test file plus a manifest edit and a guest rebuild. |
| Step 4 | M | The `format_placeholder_value` list fix, two module tests, and the two real-3MF e2e suites (build `pnp-cli` first). |
| Step 5 | S | One ranged doc section plus a generator run. |
| Step 6 | S | Two delegated doc reads; no code. |
| Step 7 | S | Full 21-AC matrix plus all workspace, freshness, CLI, generated-doc, and untouched-file gates; status changes to implemented at closure. |

Split before activation if aggregate cost exceeds M or any step is L.

## Packet Completion Gate

- All steps and exits complete.
- Every pipe-suffixed AC command returns PASS — **including AC-18**, which is the gate whose absence let a rejected policy pass a full acceptance ceremony.
- `cargo xtask build-guests --check` reports no `STALE:` as the last action before closure.
- `cargo build -p pnp-cli` has run before the final AC-18 dispatch.
- Update `docs/07_implementation_status.md` through a worker dispatch, never a full backlog read.
- The deleted aggregate custom-G-code row stays absent — packets 187 and 188 carry the injection-point residuals. Do not recreate it.
- **`docs/adr/0050-custom-gcode-architecture.md` is rewritten in place by this
  closure work.** The packet consumes its warn-and-pass decision and records
  the closed `D-<n>-ADR-0050-AMENDED` row.
- `packet.spec.md` is `status: implemented` after the complete acceptance
  ceremony.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed AC and packet-level gate command.
- **Run AC-18 last and treat a failure there as blocking regardless of every other result.** The superseded ceremony was 18/18 green against three broken e2e tests; the ordering is deliberate.
- Record remaining packet-local risk: bracketed literal text can still reach a
  printer, mitigated only by one aggregated warning. Confirm the residual
  `DEV-###` row states that accepted risk and the reverted policy plainly; ADR-
  0050 is aligned and is not a blocker.
- Confirm context stayed at or below 150k standard, or at/below 300k only with a logged swarm ESCALATION; otherwise record a packet-authoring lesson.
- **Record the authoring lesson from the first attempt explicitly:** a criterion driven only by author-chosen `config_with` pairs or a hand-built raw config is not evidence about user-visible behaviour. Any packet touching a user-facing contract needs at least one criterion that slices a real committed fixture end to end.

All `cargo check`, `cargo clippy`, and `cargo test` invocations in gate and verification commands must use `--all-targets` where the command accepts it, so the test, bench, and example targets compile.
