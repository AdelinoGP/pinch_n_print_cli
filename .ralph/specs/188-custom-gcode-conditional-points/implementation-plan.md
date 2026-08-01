# Implementation Plan: 188-custom-gcode-conditional-points

## Execution Rules

- Work one atomic step at a time; map every step to grouped task IDs.
- Use TDD, then implementation, then the narrowest falsifying validation.
- Every field below is a context-budget contract and must be filled independently; never write "see Step 1".
- **Precondition for the whole packet:** packets `186-custom-gcode-placeholder-engine` (`TASK-305`) and `187-custom-gcode-injection-registry` (`TASK-306`) are both `implemented`. If either is still `draft`, stop and report — do not reimplement their surfaces here.

## Steps

### Step 0: Record the packet baseline ref

- Task IDs: `TASK-307`
- Objective: capture the commit this packet starts from, so every "this packet must not modify X" guard has a ref that survives the packet's own commits.
- Precondition: the working tree is at the commit from which this packet's work begins.
- Postcondition: `target/pkt-188-baseline-ref.txt` exists and contains a single commit SHA.
- Files allowed to read, with ranges when over 300 lines:
  - none.
- Files allowed to edit (at most 3):
  - none under version control; this step writes only `target/pkt-188-baseline-ref.txt`.
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
  - `bash -c 'git diff --quiet && git diff --cached --quiet || { echo "FAIL: working tree or index is dirty - commit or stash first, or the baseline bakes in edits this packet must be measured against"; exit 1; }; mkdir -p target && git rev-parse HEAD > target/pkt-188-baseline-ref.txt && rg -q "^[0-9a-f]{40}$" target/pkt-188-baseline-ref.txt && echo PASS || echo "FAIL: baseline ref not recorded"'`
- Exit condition: the file exists and holds one SHA. **Every no-touch guard in this packet diffs against it.** Do not substitute `HEAD` (empty after the packet commits, so a committed edit passes) or `git merge-base HEAD master` (measured: `crates/slicer-gcode/src/emit.rs` already differs from that merge-base because of pre-existing branch work, so the guard would fail a correct implementation).

### Step 1: Red tests for the toolchange and role sites

- Task IDs: `TASK-307`
- Objective: add the eight module-level tests named by AC-3, AC-4, AC-5, AC-6, AC-19, AC-20, AC-N1 and AC-N2, each driving `run_gcode_postprocess` over a synthetic stream containing `GCodeCommand::ToolChange` commands and `Raw(";TYPE:<label>")` markers alongside packet 187's layer triples.
- Precondition: `cargo test -p machine-gcode-emit --test machine_gcode_emit_tdd` is green on the post-187 tree.
- Postcondition: the eight tests exist and the binary is red on exactly those eight.
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/machine-gcode-emit/tests/machine_gcode_emit_tdd.rs` — whole file; the `run` / `raw_texts` helpers and packet 187's layer-marker fixtures.
  - `modules/core-modules/machine-gcode-emit/src/lib.rs` — whole file; `InjectionSite`, `INJECTION_POINTS` and the error constants the tests assert on.
  - `crates/slicer-gcode/src/emit.rs` — **long; ranged read only** — the `ToolChange` push sites and the `role_changed` block, to copy the exact command and marker shapes into the fixtures.
- Files allowed to edit (at most 3):
  - `modules/core-modules/machine-gcode-emit/tests/machine_gcode_emit_tdd.rs`
- Files explicitly out of bounds:
  - `modules/core-modules/machine-gcode-emit/src/lib.rs` and its `.toml` (Steps 2-4 own them)
  - `crates/slicer-gcode/**` — read-only for the whole packet
  - `crates/slicer-runtime/**` (Steps 4 and 5 own the e2e halves)
  - `OrcaSlicerDocumented/**`, `target/**`
- Expected sub-agent dispatches:
  - Question: what exact variable set does `s_CustomGcodeSpecificPlaceholders` list for each of the six new options? Scope: `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp`; return: `FACT` ≤ 8 lines, one per option, names only
  - Question: in `GCode::set_extruder`, what is the exact emission order of `filament_end_gcode`, `change_filament_gcode`, `m_writer.toolchange` and `filament_start_gcode`? Scope: `OrcaSlicerDocumented/src/libslic3r/GCode.cpp`; return: `SUMMARY` ≤ 150 words as an ordered list
- Context cost: `S`
- Authoritative docs:
  - `docs/02_ir_schemas.md` — delegated SUMMARY only, for `GCodeCommand::ToolChange`'s fields.
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/GCode.cpp`, `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` — delegate; never load.
- Verification:
  - `bash -c 'cargo test -p machine-gcode-emit --test machine_gcode_emit_tdd 2>&1 | tee target/log-188-mge-red.txt | rg "^test result:" | rg -q "FAILED" && echo "RED (expected)" || echo "NOT RED — the new tests are not discriminating"'` — the only step whose success condition is a **red** binary; read `target/log-188-mge-red.txt` rather than re-running.
- Exit condition: the eight named tests exist, and the binary's failures are exactly those eight.

### Step 2: Toolchange sites — registry entries, walk, and per-site variables

- Task IDs: `TASK-307`
- Objective: add the `FilamentEnd` / `FilamentChange` / `FilamentStart` `InjectionSite` variants and their three `INJECTION_POINTS` entries; extend packet 187's single forward walk to splice `filament_end_gcode` and `change_filament_gcode` before each `GCodeCommand::ToolChange` and `filament_start_gcode` after it; bind `previous_extruder`, `next_extruder`, `toolchange_count` and `filament_extruder_id` per canonical's per-option sets.
- Precondition: Step 1's eight tests exist and are red for their own reasons.
- Postcondition: AC-3, AC-4, AC-20 and AC-N2 pass; the `ToolChange` command itself is still re-emitted unchanged.
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/machine-gcode-emit/src/lib.rs` — whole file.
  - `modules/core-modules/machine-gcode-emit/tests/machine_gcode_emit_tdd.rs` — the eight tests from Step 1 (AC-3, AC-4, AC-5, AC-6, AC-19, AC-20, AC-N1, AC-N2).
  - `crates/slicer-gcode/src/emit.rs` — **long; ranged read only** — the `ToolChange` push sites.
- Files allowed to edit (at most 3):
  - `modules/core-modules/machine-gcode-emit/src/lib.rs`
- Files explicitly out of bounds:
  - `crates/slicer-ir/**`, `crates/slicer-sdk/**`, `crates/slicer-schema/**`, `crates/slicer-macros/**`
  - `crates/slicer-gcode/**` — read-only; AC-12 fails if `emit.rs`, `serialize.rs` or `golden_emit_tdd.rs` is modified
  - `crates/slicer-runtime/**`, `docs/**`
  - `OrcaSlicerDocumented/**`, `target/**`
- Blast-radius discipline: no struct field is added to a shared type and no schema/version constant is bumped, so the struct-literal sweep does not apply. The new `InjectionSite` variants make every `match` on that enum in the file non-exhaustive — enumerate those match sites before editing (they are all inside `run_gcode_postprocess` and its helpers in this one file) and handle each explicitly rather than adding a `_ =>` arm, which would silently swallow a future site.
- Expected sub-agent dispatches:
  - Question: in `GCode::set_extruder`, is `filament_extruder_id` bound to the old or the new tool for `filament_end_gcode` and for `filament_start_gcode`? Scope: `OrcaSlicerDocumented/src/libslic3r/GCode.cpp`; return: `FACT` ≤ 3 lines
  - Question: does `cargo xtask build-guests --check` report `STALE:` after this edit? Scope: cargo run; return: `FACT` clean/stale
- Context cost: `M`
- Authoritative docs:
  - `docs/02_ir_schemas.md` — delegated SUMMARY only.
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/GCode.cpp` — `GCode::set_extruder`; delegate.
- Verification:
  - `cargo xtask build-guests --check` — FACT clean; rebuild without `--check` if `STALE:`.
  - `bash -c 'cargo test -p machine-gcode-emit --test machine_gcode_emit_tdd -- toolchange_trio_brackets_the_tool_select_in_canonical_order --exact 2>&1 | rg "^test result:" | rg -q "^test result: ok\. [1-9]" && echo PASS || echo "FAIL: toolchange_trio_brackets_the_tool_select_in_canonical_order did not run or did not pass"'` — **verbatim copy of `packet.spec.md` AC-3; if either changes, change both.**
  - `bash -c 'cargo test -p machine-gcode-emit --test machine_gcode_emit_tdd -- toolchange_variables_carry_from_to_and_running_count --exact 2>&1 | rg "^test result:" | rg -q "^test result: ok\. [1-9]" && echo PASS || echo "FAIL: toolchange_variables_carry_from_to_and_running_count did not run or did not pass"'` — **verbatim copy of `packet.spec.md` AC-4; if either changes, change both.**
  - `bash -c 'cargo test -p machine-gcode-emit --test machine_gcode_emit_tdd -- layer_variables_resolve_at_filament_start_gcode --exact 2>&1 | rg "^test result:" | rg -q "^test result: ok\. [1-9]" && echo PASS || echo "FAIL: layer_variables_resolve_at_filament_start_gcode did not run or did not pass"'` — **verbatim copy of `packet.spec.md` AC-20; if either changes, change both.** An earlier draft named AC-20 in this step's Exit condition while giving it no command anywhere in the plan; the whole-binary run in Step 3 cannot substitute, because a binary that is simply **missing** one of the eight tests still prints `test result: ok. …`. AC-20 is the positive half of the `s_CustomGcodeSpecificPlaceholders` third drift and is one of the two highest-risk parity claims in this packet.
  - `bash -c 'cargo test -p machine-gcode-emit --test machine_gcode_emit_tdd -- next_extruder_in_filament_start_gcode_passes_through_verbatim --exact 2>&1 | rg "^test result:" | rg -q "^test result: ok\. [1-9]" && echo PASS || echo "FAIL: next_extruder_in_filament_start_gcode_passes_through_verbatim did not run or did not pass"'` — **verbatim copy of `packet.spec.md` AC-N2; if either changes, change both.**
- Exit condition: AC-3, AC-4, AC-20 and AC-N2 print PASS.

### Step 3: Extrusion-role sites — registry entries, walk, and per-site variables

- Task IDs: `TASK-307`
- Objective: add the `ExtrusionRoleChange` `InjectionSite` variant and the three role `INJECTION_POINTS` entries in canonical order; splice all three before each `Raw` whose text begins `;TYPE:`; bind `extrusion_role` and `last_extrusion_role`; deliberately **omit** `max_layer_z` from the role sites' variable set; and give the role sites their **own** `layer_num` of **N+1** (canonical `GCode::_extrude` uses `m_layer_index + 1`) rather than reusing 187's `LayerContext` value, which is N at every other site.
- Precondition: Step 2 is green.
- Postcondition: AC-5, AC-6, AC-19 and AC-N1 pass; the whole `machine_gcode_emit_tdd` binary is green.
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/machine-gcode-emit/src/lib.rs` — whole file.
  - `modules/core-modules/machine-gcode-emit/tests/machine_gcode_emit_tdd.rs` — the eight tests from Step 1 (AC-3, AC-4, AC-5, AC-6, AC-19, AC-20, AC-N1, AC-N2).
  - `crates/slicer-gcode/src/emit.rs` — **long; ranged read only** — the `role_changed` block and `orca_type_label`, to confirm the marker text and that `role_equals` governs when a marker appears.
- Files allowed to edit (at most 3):
  - `modules/core-modules/machine-gcode-emit/src/lib.rs`
- Files explicitly out of bounds:
  - `crates/slicer-ir/**`, `crates/slicer-sdk/**`, `crates/slicer-schema/**`, `crates/slicer-macros/**`
  - `crates/slicer-gcode/**` — read-only
  - `crates/slicer-runtime/**`, `docs/**`
  - `OrcaSlicerDocumented/**`, `target/**`
- Blast-radius discipline: same as Step 2 — one new `InjectionSite` variant, so re-check the `match` sites in this file and handle it explicitly rather than with a catch-all arm.
- Expected sub-agent dispatches:
  - Question: in `GCode::_extrude`, are the three role templates emitted before or after the `;_EXTRUSION_ROLE:` / `;TYPE:` markers, and in what order? Scope: `OrcaSlicerDocumented/src/libslic3r/GCode.cpp`; return: `FACT` ≤ 4 lines
  - Question: does `cargo xtask build-guests --check` report `STALE:` after this edit? Scope: cargo run; return: `FACT` clean/stale
- Context cost: `M`
- Authoritative docs:
  - `docs/02_ir_schemas.md` — delegated SUMMARY only.
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/GCode.cpp` — `GCode::_extrude`; delegate.
- Verification:
  - `cargo xtask build-guests --check` — FACT clean; rebuild if `STALE:`.
  - `bash -c 'cargo test -p machine-gcode-emit --test machine_gcode_emit_tdd -- role_trio_precedes_the_type_marker_with_current_and_last_role --exact 2>&1 | rg "^test result:" | rg -q "^test result: ok\. [1-9]" && echo PASS || echo "FAIL: role_trio_precedes_the_type_marker_with_current_and_last_role did not run or did not pass"'` — **verbatim copy of `packet.spec.md` AC-5; if either changes, change both.**
  - `bash -c 'cargo test -p machine-gcode-emit --test machine_gcode_emit_tdd -- unset_toolchange_and_role_points_emit_nothing --exact 2>&1 | rg "^test result:" | rg -q "^test result: ok\. [1-9]" && echo PASS || echo "FAIL: unset_toolchange_and_role_points_emit_nothing did not run or did not pass"'` — **verbatim copy of `packet.spec.md` AC-6; if either changes, change both.**
  - `bash -c 'cargo test -p machine-gcode-emit --test machine_gcode_emit_tdd -- role_sites_carry_layer_num_plus_one_while_toolchange_sites_do_not --exact 2>&1 | rg "^test result:" | rg -q "^test result: ok\. [1-9]" && echo PASS || echo "FAIL: role_sites_carry_layer_num_plus_one_while_toolchange_sites_do_not did not run or did not pass"'` — **verbatim copy of `packet.spec.md` AC-19; if either changes, change both.** An earlier draft named AC-19 in this step's Exit condition while giving it no command anywhere in the plan; the whole-binary run below cannot substitute, because a binary that is simply **missing** this test still prints `test result: ok. …`. AC-19 pins the `layer_num` = N+1 asymmetry at the role sites, the value-union shortcut this packet exists to prevent.
  - `bash -c 'cargo test -p machine-gcode-emit --test machine_gcode_emit_tdd -- max_layer_z_in_role_gcode_passes_through_verbatim --exact 2>&1 | rg "^test result:" | rg -q "^test result: ok\. [1-9]" && echo PASS || echo "FAIL: max_layer_z_in_role_gcode_passes_through_verbatim did not run or did not pass"'` — **verbatim copy of `packet.spec.md` AC-N1; if either changes, change both.**
  - `bash -c 'cargo test -p machine-gcode-emit --test machine_gcode_emit_tdd 2>&1 | tee target/log-188-mge.txt | rg "^test result:" | rg -q "^test result: ok\. [1-9]" && echo PASS || echo "FAIL: machine_gcode_emit_tdd is not fully green — see target/log-188-mge.txt"'` — **verbatim copy of `packet.spec.md` AC-9; if either changes, change both.**
  - `bash -c 'python3 -c "import io; s=io.open(r\"modules/core-modules/machine-gcode-emit/src/lib.rs\",encoding=\"utf-8\").read(); hits=[i for i in range(len(s)) if s.startswith(\"INJECTION_POINTS\",i)]; anc=[i for i in hits if \"=&[\" in s[i:i+160].replace(chr(32),\"\").replace(chr(10),\"\").replace(chr(9),\"\")]; tbl=s[anc[0]:] if anc else \"\"; ev=\"enum InjectionSite\" in s; keys=(\"machine_start_gcode\",\"before_layer_change_gcode\",\"time_lapse_gcode\",\"layer_change_gcode\",\"machine_end_gcode\",\"filament_end_gcode\",\"change_filament_gcode\",\"filament_start_gcode\",\"change_extrusion_role_gcode\",\"filament_change_extrusion_role_gcode\",\"process_change_extrusion_role_gcode\"); miss=[k for k in keys if (chr(34)+k+chr(34)) not in tbl]; print(\"PASS\" if anc and ev and not miss else \"FAIL: table literal found=\"+str(bool(anc))+\", enum InjectionSite found=\"+str(ev)+\", missing INJECTION_POINTS entries \"+str(miss))"'` — **verbatim copy of `packet.spec.md` AC-1; if either changes, change both.** An earlier draft left a windowed `s[b:b+4000]` variant here that false-FAILs a correct implementation once Step 4's doc-comment rewrite mentions `INJECTION_POINTS`; see AC-1's own prose for the measurement.
- Exit condition: AC-1, AC-5, AC-6, AC-9, AC-19 and AC-N1 print PASS.

### Step 4: Declare the six keys and add the single-material role e2e pin

- Task IDs: `TASK-307`
- Objective: add the six `[config.schema.*]` string blocks, update the manifest `[module] description` and the module's crate-level doc comment, and add `role_change_gcode_precedes_every_type_marker` to the `integration` bucket.
- Precondition: Step 3 is green and `cargo xtask build-guests --check` is clean.
- Postcondition: `[config.schema]` has fourteen keys; the whole `machine_start_end_gcode_emission_tdd` module is green; AC-N3 still passes (the five unreachable names remain undeclared).
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/machine-gcode-emit/machine-gcode-emit.toml` — whole file.
  - `crates/slicer-runtime/tests/integration/machine_start_end_gcode_emission_tdd.rs` — **long; ranged reads only.** `slice_with_raw` / `try_slice_with_raw` and `count_occurrences`. Do not load the whole file. **`try_slice_with_raw` is a FORWARD-DEP on packet 186, not an existing symbol** — 186 adds it beside `slice_with_raw` in this file (its `design.md` §Code Change Surface and its `implementation-plan.md` Step 3 both specify the addition). If it is absent when this step runs, 186 has not landed and this packet's precondition chain (186 → 187 → 188) is broken — **stop and report; do not add it here.**
- Files allowed to edit (at most 3):
  - `modules/core-modules/machine-gcode-emit/machine-gcode-emit.toml`
  - `crates/slicer-runtime/tests/integration/machine_start_end_gcode_emission_tdd.rs`
  - `modules/core-modules/machine-gcode-emit/src/lib.rs` (doc comment only — no logic change in this step)
- Files explicitly out of bounds:
  - `crates/slicer-gcode/**`, `crates/slicer-runtime/src/**`
  - `crates/slicer-runtime/tests/executor/**` (Step 5 owns it)
  - `docs/**` (Steps 6 and 7 own them)
  - `OrcaSlicerDocumented/**`, `target/**`
- Blast-radius discipline: six new **string** keys change what `slice_with_raw` seeds — it iterates `machine_binding.module.config_schema().entries` generically and routes string defaults to `binding_source` as the real value and to `pipeline_source` as an empty sentinel, so `default = ""` keys need no harness edit and yield `; <key> = ` CONFIG_BLOCK lines. Confirm before assuming: the count-shaped neighbours are `module_manifest_registers_five_keys_with_expected_types_and_defaults` and `new_keys_appear_in_config_block` (both assert **presence**, not a total) and `gcode_header_thumbnail_config_blocks_tdd`'s "at least 80 key-value lines" **lower bound**. If any turns out to assert a total, it belongs in this step's edit list.
- Expected sub-agent dispatches:
  - Question: does `cargo xtask build-guests --check` report `STALE:` after the manifest edit? Scope: cargo run; return: `FACT` clean/stale
  - Question: does any test under `crates/slicer-runtime/tests/` assert an exact total number of CONFIG_BLOCK key lines or an exact `machine-gcode-emit` schema length? Scope: `crates/slicer-runtime/tests/**`; return: `LOCATIONS` ≤ 20
- Context cost: `M`
- Authoritative docs:
  - `docs/15_config_keys_reference.md` — the generated `module-config-keys` marker boundaries only; ranged read.
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` — `PrintConfigDef::init_fff_params`; delegate; confirms the six options' types and upstream defaults.
- Verification:
  - `cargo xtask build-guests --check` — FACT clean; rebuild if `STALE:`.
  - `bash -c 'python3 -c "import tomllib; d=tomllib.load(open(r\"modules/core-modules/machine-gcode-emit/machine-gcode-emit.toml\",\"rb\"))[\"config\"][\"schema\"]; want=(\"filament_end_gcode\",\"change_filament_gcode\",\"filament_start_gcode\",\"change_extrusion_role_gcode\",\"filament_change_extrusion_role_gcode\",\"process_change_extrusion_role_gcode\"); bad=[k for k in want if not (k in d and d[k][\"type\"]==\"string\" and d[k].get(\"default\")==\"\" and d[k].get(\"group\")==\"Machine G-code\")]; print(\"PASS\" if not bad and len(d)==14 else \"FAIL: \"+str(bad)+\" ; schema keys=\"+str(sorted(d)))"'` — **verbatim copy of `packet.spec.md` AC-2; if either changes, change both.**
  - `bash -c 'cargo test -p slicer-runtime --test integration -- machine_start_end_gcode_emission_tdd::role_change_gcode_precedes_every_type_marker --exact 2>&1 | rg "^test result:" | rg -q "^test result: ok\. [1-9]" && echo PASS || echo "FAIL: role_change_gcode_precedes_every_type_marker did not run or did not pass"'` — **verbatim copy of `packet.spec.md` AC-7; if either changes, change both.** The module-scoped run below cannot substitute: a bucket missing this test still prints `test result: ok. …`.
  - `bash -c 'cargo test -p slicer-runtime --test integration -- machine_start_end_gcode_emission_tdd:: 2>&1 | tee target/log-188-msege.txt | rg "^test result:" | rg -q "^test result: ok\. [1-9]" && echo PASS || echo "FAIL: the machine_start_end_gcode_emission_tdd module is not fully green — see target/log-188-msege.txt"'` — **verbatim copy of `packet.spec.md` AC-11; if either changes, change both.**
  - `bash -c 'python3 -c "import io,tomllib; s=io.open(r\"modules/core-modules/machine-gcode-emit/src/lib.rs\",encoding=\"utf-8\").read(); d=tomllib.load(open(r\"modules/core-modules/machine-gcode-emit/machine-gcode-emit.toml\",\"rb\"))[\"config\"][\"schema\"]; bad=[k for k in (\"file_start_gcode\",\"wrapping_detection_gcode\",\"machine_pause_gcode\",\"template_custom_gcode\",\"printing_by_object_gcode\") if k in d or (chr(34)+k+chr(34)) in s]; print(\"PASS\" if not bad else \"FAIL: unreachable point(s) \"+str(bad)+\" were declared or registered — they must stay unimplemented and recorded as residuals\")"'` — **verbatim copy of `packet.spec.md` AC-N3; if either changes, change both.**
- Exit condition: AC-2, AC-7, AC-11 and AC-N3 print PASS.

### Step 5: Four-tool end-to-end toolchange pin

- Task IDs: `TASK-307`
- Objective: add a `config_overrides`-accepting sibling of `slice_fixture_file` to `crates/slicer-runtime/tests/executor/cube_4color_gcode_output_tdd.rs` and the `change_filament_gcode_precedes_every_tool_select` test that drives `resources/cube_4color.3mf` through it.
- Precondition: Step 4 is green and `cargo xtask build-guests --check` is clean.
- Postcondition: AC-8 and AC-10 pass; the existing `slice_fixture_file` still passes an empty `config_overrides` map and its callers are unchanged.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-runtime/tests/executor/cube_4color_gcode_output_tdd.rs` — **long; ranged reads only.** `cube_4color_path`, `slice_fixture_file`, and one nearby test (`cube_4color_gcode_emits_all_four_tool_indices`) for the assertion idiom. Do not load the whole file.
- Files allowed to edit (at most 3):
  - `crates/slicer-runtime/tests/executor/cube_4color_gcode_output_tdd.rs`
- Files explicitly out of bounds:
  - `crates/slicer-runtime/tests/executor/main.rs` — the module is already mounted; no new mount is needed.
  - `crates/slicer-gcode/**`, `crates/slicer-runtime/src/**`
  - `modules/**`, `docs/**`
  - `OrcaSlicerDocumented/**`, `target/**`
- Blast-radius discipline: adding a harness sibling must not change `slice_fixture_file`'s signature — every existing caller in the file compiles against it today. Extract the shared body into a private helper that both call, or add the sibling as a thin wrapper; do not add a parameter to the existing function.
- Expected sub-agent dispatches:
  - Question: which field of `SliceRunOptions` carries per-key config overrides, and what is its exact type? Scope: `crates/slicer-runtime/src/**`; return: `FACT` ≤ 3 lines
- Context cost: `M`
- Authoritative docs:
  - none additional.
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/GCode.cpp` — `GCode::set_extruder`; delegate; the ordering AC-8 asserts.
- Verification:
  - `bash -c 'cargo test -p slicer-runtime --test executor -- cube_4color_gcode_output_tdd::change_filament_gcode_precedes_every_tool_select --exact 2>&1 | rg "^test result:" | rg -q "^test result: ok\. [1-9]" && echo PASS || echo "FAIL: change_filament_gcode_precedes_every_tool_select did not run or did not pass"'` — **verbatim copy of `packet.spec.md` AC-8; if either changes, change both.**
  - `bash -c 'cargo test -p slicer-runtime --test executor -- cube_4color_gcode_output_tdd:: 2>&1 | tee target/log-188-c4c.txt | rg "^test result:" | rg -q "^test result: ok\. [1-9]" && echo PASS || echo "FAIL: cube_4color_gcode_output_tdd is not fully green — see target/log-188-c4c.txt"'` — **verbatim copy of `packet.spec.md` AC-10; if either changes, change both.** Long-running; read the log rather than re-running.
  - `bash -c 'BASE=$(cat target/pkt-188-baseline-ref.txt 2>/dev/null); { [ -n "$BASE" ] && git rev-parse --verify -q "$BASE^{commit}" >/dev/null; } || { echo "FAIL: baseline ref target/pkt-188-baseline-ref.txt missing, empty, or not a valid commit - run Step 0 before this guard"; exit 1; }; git diff --name-only "$BASE" -- crates/slicer-gcode/src/emit.rs crates/slicer-gcode/src/serialize.rs crates/slicer-gcode/tests/golden_emit_tdd.rs  | rg -q . && echo "FAIL: this packet must not modify the host emitter, serializer, or golden" || echo PASS'` — **verbatim copy of `packet.spec.md` AC-12; if either changes, change both.**
- Exit condition: AC-8, AC-10 and AC-12 print PASS.

### Step 6: Rewrite the `docs/15` injection-point section and regenerate the config-keys block

- Task IDs: `TASK-307`
- Objective: extend the injection-point section to eleven points, document the toolchange order relative to the `T<n>` select and the role order relative to the `;TYPE:` marker, document the new per-site macros, add the explicit "not implemented, and why" list for the five unreachable points, and regenerate the `module-config-keys` block.
- Precondition: Step 5 is green, so the manifest and behaviour are final.
- Postcondition: `cargo xtask gen-config-docs --check` exits 0; AC-13 and AC-14 pass.
- Files allowed to read, with ranges when over 300 lines:
  - `docs/15_config_keys_reference.md` — **long; ranged reads only.** The injection-point section packet 187 rewrote, and the `<!-- BEGIN GENERATED: module-config-keys (cargo xtask gen-config-docs) -->` / `<!-- END GENERATED: module-config-keys -->` marker lines.
- Files allowed to edit (at most 3):
  - `docs/15_config_keys_reference.md`
- Files explicitly out of bounds:
  - Anything between the `module-config-keys` markers — regenerate, never hand-edit.
  - `docs/ORCA_CONFIG_REFERENCE.md` — deliberately untouched.
  - `docs/DEVIATION_LOG.md`, `docs/07_implementation_status.md` (Step 7 owns them)
  - `modules/**`, `crates/**`
- Expected sub-agent dispatches:
  - Question: does `cargo xtask gen-config-docs --check` exit 0 after regeneration, and does the generated table pair each of the six new keys with `machine-gcode-emit`? Scope: cargo run + `docs/15_config_keys_reference.md`; return: `FACT` pass/fail
- Context cost: `S`
- Authoritative docs:
  - `docs/15_config_keys_reference.md` — the section being extended; ranged read.
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` — `custom_gcode_specific_placeholders`; delegate; supplies the per-site variable sets quoted in the prose.
- Verification:
  - `bash -c 'python3 -c "import io; s=io.open(r\"docs/15_config_keys_reference.md\",encoding=\"utf-8\").read(); b=chr(96); pts=(\"filament_end_gcode\",\"change_filament_gcode\",\"filament_start_gcode\",\"change_extrusion_role_gcode\",\"filament_change_extrusion_role_gcode\",\"process_change_extrusion_role_gcode\"); miss=[k for k in pts if k not in s]; mac=all((b+\"[\"+v+\"]\"+b) in s for v in (\"previous_extruder\",\"next_extruder\",\"toolchange_count\",\"extrusion_role\",\"last_extrusion_role\")); unreach=all(k in s for k in (\"file_start_gcode\",\"wrapping_detection_gcode\",\"machine_pause_gcode\",\"template_custom_gcode\",\"printing_by_object_gcode\")); print(\"PASS\" if not miss and mac and unreach else \"FAIL: missing points \"+str(miss)+\", macros documented=\"+str(mac)+\", unreachable list present=\"+str(unreach))"'` — **verbatim copy of `packet.spec.md` AC-13; if either changes, change both.**
  - `bash -c 'cargo xtask gen-config-docs --check >/dev/null 2>&1 || { echo "FAIL: cargo xtask gen-config-docs --check did not exit 0 - it is red, or this command was not run from the repo root"; exit 1; }; python3 -c "import io; s=io.open(r\"docs/15_config_keys_reference.md\",encoding=\"utf-8\").read(); b=chr(96); p=chr(124); rows=[ln for ln in s.splitlines() if ln.startswith(p)]; miss=[k for k in (\"filament_end_gcode\",\"change_filament_gcode\",\"filament_start_gcode\",\"change_extrusion_role_gcode\",\"filament_change_extrusion_role_gcode\",\"process_change_extrusion_role_gcode\") if not any((b+k+b) in ln and (b+\"machine-gcode-emit\"+b) in ln for ln in rows)]; print(\"PASS\" if not miss else \"FAIL: no generated row pairs \"+str(miss)+\" with machine-gcode-emit\")"'` — **verbatim copy of `packet.spec.md` AC-14; if either changes, change both.** An earlier draft kept only the `gen-config-docs --check` half here and dropped the row-pairing clause, while this step's Exit condition still claimed AC-14. Measured on today's tree: the half-form printed **PASS** and full AC-14 printed `FAIL: no generated row pairs [all six] with machine-gcode-emit` — the half the packet itself labels a green do-not-regress guard was gating a step that claimed the discriminating criterion.
- Exit condition: AC-13 and AC-14 print PASS.

### Step 7: File the three residual rows, close `DEV-085`, register `TASK-307`

- Task IDs: `TASK-307`
- Objective: file one `DEV-###` row for the five unreachable points with their measured evidence (AC-15); a second for the `coStrings` per-filament gap, the unmodelled `change_filament_gcode` flush/travel variable group, the `erNone` substitution, the `manual_filament_change` suppression and the **tag-gate vs template-gate** divergence (AC-16); a **third**, standalone, for the `filament_extruder_id` two-id-space divergence, citing `docs/adr/0050-custom-gcode-architecture.md` (AC-21); then flip `DEV-085` to `Closed` citing all three task IDs, and hand-add the `TASK-307` backlog row outside the generated block.
- **Three rows, not two, and the split is deliberate.** AC-16 previously bundled six unrelated residuals behind one predicate demanding eight co-occurring tokens, so no part of it could be closed without the whole. The `filament_extruder_id` clause is the one that does not belong there at all: it is not a documentation residual but an IR-level constraint on `GCodeCommand::ToolChange` that binds future MMU / multi-extruder work, and a token inside another row's predicate is not a decision of record. It now has its own row and its own ADR citation. The tag-gate/template-gate divergence moves the other way — it was stated in three packet files and carried by **no** row and **no** token, so it gains the AC-16 predicate token `m_last_processor_extrusion_role`. The `erNone` substitution needs only its existing AC-16 clause and token; it gets no ADR.
- Precondition: Steps 1-6 complete. **`DEV-085` may only be closed after all three residual rows exist**, so the log never contains a closed row whose remainder is untracked.
- Postcondition: all three residual rows exist and carry three distinct `DEV-###` ids; `DEV-085` reads `Closed` and cites `TASK-305` / `TASK-306` / `TASK-307`; `TASK-307` resolves in `docs/07_implementation_status.md`.
- Files allowed to read, with ranges when over 300 lines:
  - `docs/DEVIATION_LOG.md` — **long; delegate or range-read.** The `DEV-085` row, the two most recent rows for format, and a re-derivation of the highest `DEV-###`.
  - `docs/07_implementation_status.md` — **always delegate.**
- Files allowed to edit (at most 3):
  - `docs/DEVIATION_LOG.md`
  - `docs/07_implementation_status.md`
- Files explicitly out of bounds:
  - The `<!-- BEGIN GENERATED: open-deviations (cargo xtask check-deviations) -->` … `<!-- END GENERATED: open-deviations -->` span of `docs/07_implementation_status.md` — regenerate with `cargo xtask check-deviations`, never hand-edit.
  - `docs/15_config_keys_reference.md` (Step 6 owns it)
  - `modules/**`, `crates/**`
- Expected sub-agent dispatches:
  - Question: what is the highest `DEV-###` in `docs/DEVIATION_LOG.md` right now? Scope: `rg -o '^\| DEV-[0-9]{3}' docs/DEVIATION_LOG.md | sort -u | tail -1`; return: `FACT` one line. **Dispatch this three times — once before each new row** — because filing the first row changes the answer for the second, and because parallel packets file rows concurrently.
  - Question: which of the six new options are `coStrings` rather than `coString`, and what is the full `change_filament_gcode` placeholder variable list? Scope: `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp`; return: `FACT` ≤ 8 lines, names only
- Context cost: `S`
- Authoritative docs:
  - `docs/DEVIATION_LOG.md`, `docs/07_implementation_status.md` — delegated; row formats and next free IDs.
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp`, `OrcaSlicerDocumented/src/libslic3r/GCode.cpp` — delegate; supply both residual rows' evidence.
- Verification:
  - `bash -c 'python3 -c "import io; p=chr(124); L=io.open(r\"docs/DEVIATION_LOG.md\",encoding=\"utf-8\").read().splitlines(); need=(\"file_start_gcode\",\"wrapping_detection_gcode\",\"machine_pause_gcode\",\"template_custom_gcode\",\"printing_by_object_gcode\",\"ThumbnailAwareSerializer\",\"emit_custom_gcode_per_print_z\",\"enable_wrapping_detection\",\"ORCA_CONFIG_PADDING\"); rows=[l for l in L if l.startswith(p+chr(32)+\"DEV-\") and not l.startswith(p+chr(32)+\"DEV-085\")]; hit=[l for l in rows if all(t in l for t in need)]; best=max(rows,key=lambda l:sum(t in l for t in need),default=\"\"); print(\"PASS\" if hit else \"FAIL: no single new DEV row carries all tokens; best row misses \"+str([t for t in need if t not in best]))"'` — **verbatim copy of `packet.spec.md` AC-15; if either changes, change both.** An earlier draft left the superseded round-1 probe here, so a worker gating on this step used a weaker check than the AC it claims to satisfy.
  - `bash -c 'python3 -c "import io; p=chr(124); L=io.open(r\"docs/DEVIATION_LOG.md\",encoding=\"utf-8\").read().splitlines(); need=(\"coStrings\",\"get_at\",\"flush_length_1\",\"x_after_toolchange\",\"erNone\",\"append_tcr\",\"m_last_processor_extrusion_role\",\"manual_filament_change\"); rows=[l for l in L if l.startswith(p+chr(32)+\"DEV-\") and not l.startswith(p+chr(32)+\"DEV-085\")]; hit=[l for l in rows if all(t in l for t in need)]; best=max(rows,key=lambda l:sum(t in l for t in need),default=\"\"); print(\"PASS\" if hit else \"FAIL: no single new DEV row carries all tokens; best row misses \"+str([t for t in need if t not in best]))"'` — **verbatim copy of `packet.spec.md` AC-16; if either changes, change both.** An earlier draft left the superseded round-1 probe here, so a worker gating on this step used a weaker check than the AC it claims to satisfy.
  - `bash -c 'python3 -c "import io; p=chr(124); L=io.open(r\"docs/DEVIATION_LOG.md\",encoding=\"utf-8\").read().splitlines(); need=(\"filament_extruder_id\",\"get_extruder_id\",\"new_filament_id\",\"ToolChange\",\"ADR-0050\"); rows=[l for l in L if l.startswith(p+chr(32)+\"DEV-\") and not l.startswith(p+chr(32)+\"DEV-085\")]; hit=[l for l in rows if all(t in l for t in need)]; best=max(rows,key=lambda l:sum(t in l for t in need),default=\"\"); print(\"PASS\" if hit else \"FAIL: no single new DEV row carries all tokens; best row misses \"+str([t for t in need if t not in best]))"'` — **verbatim copy of `packet.spec.md` AC-21; if either changes, change both.** This row must be a **different** `DEV-###` from AC-16's; if one row satisfies both predicates the split has not happened.
  - `bash -c 'python3 -c "import io; p=chr(124); L=io.open(r\"docs/DEVIATION_LOG.md\",encoding=\"utf-8\").read().splitlines(); r=[l for l in L if l.startswith(p+chr(32)+\"DEV-085\")]; row=(r[0] if r else \"\").replace(chr(96),\"\"); need=[t for t in (\"Closed\",\"TASK-305\",\"TASK-306\",\"TASK-307\",\"11 of 16\",\"13 coString\",\"3 coStrings\") if t not in row]; print(\"PASS\" if r and not need else (\"FAIL: no DEV-085 row\" if not r else \"FAIL: DEV-085 row missing \"+str(need)))"'` — **verbatim copy of `packet.spec.md` AC-17; if either changes, change both.** An earlier draft left the superseded round-1 probe here, which asserted only `Closed` plus the three task IDs and dropped the backtick-stripping normalisation and the `11 of 16` / `13 coString` / `3 coStrings` clauses AC-17's own prose mandates. Measured against a synthetic row `| DEV-085 | … | Closed by TASK-305 / TASK-306 / TASK-307. | Closed |`: the superseded probe printed **PASS**, AC-17 printed `FAIL: DEV-085 row missing ['11 of 16', '13 coString', '3 coStrings']` — the step gate accepted exactly what the AC it names rejects.
  - `bash -c 'python3 -c "import io; L=io.open(r\"docs/07_implementation_status.md\",encoding=\"utf-8\").read().splitlines(); B=[i for i,l in enumerate(L) if l.startswith(\"<!-- BEGIN GENERATED: open-deviations\")]; E=[i for i,l in enumerate(L) if l.startswith(\"<!-- END GENERATED: open-deviations\")]; H=[i for i,l in enumerate(L) if \"TASK-307\" in l]; print(\"FAIL: open-deviations markers not found\" if not (B and E) else (\"FAIL: TASK-307 not registered anywhere\" if not H else (\"FAIL: TASK-307 appears only INSIDE the generated block\" if all(B[0]<i<E[0] for i in H) else \"PASS\")))"'`
- Exit condition: AC-15, AC-16, AC-17, AC-18 and AC-21 print PASS.

### Step 8: Closure gates

- Task IDs: `TASK-307`
- Objective: run the workspace check/clippy gates with `--all-targets` and re-dispatch every pipe-suffixed AC command.
- Precondition: Steps 1-7 complete.
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
- Exit condition: both gates exit 0, `build-guests --check` reports no `STALE:`, and all twenty-four numbered AC commands (AC-1..AC-21, AC-N1..AC-N3) print PASS. Note Step 0's baseline ref must still exist at `target/pkt-188-baseline-ref.txt`; AC-12's no-touch guard reads it.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 0 | S | Records `target/pkt-188-baseline-ref.txt`; no repo edit. |
| Step 1 | S | One short test file; two delegated canonical dispatches. |
| Step 2 | M | Toolchange walk + four-variable per-site set + guest-freshness gate. |
| Step 3 | M | Role walk + per-site set (deliberately without `max_layer_z`) + guest rebuild. |
| Step 4 | M | Manifest edit, ranged reads of a long e2e file, guest rebuild. |
| Step 5 | M | Ranged reads of the long four-colour file plus a slow suite run. |
| Step 6 | S | One ranged doc section plus a generator run. |
| Step 7 | S | Delegated doc reads plus one canonical FACT; no code. |
| Step 8 | S | Measurement only. |

Aggregate is `M`, **at the top of M** — of nine steps, four are M (Steps 2, 3, 4, 5) and five are S (Steps 0, 1, 6, 7, 8), and the packet spans three test binaries. `design.md` §Context Cost Estimate states the same figures; keep them in sync, because the split rule keys on them. If any step's read set grows beyond the ranges listed above, split Step 2 (toolchange) from Step 3 (role) into two packets rather than escalating the context band.

## Packet Completion Gate

- All steps and exits complete.
- Every pipe-suffixed AC command returns PASS.
- `cargo xtask build-guests --check` reports no `STALE:` as the last action before closure.
- `crates/slicer-gcode/src/emit.rs`, `crates/slicer-gcode/src/serialize.rs` and `crates/slicer-gcode/tests/golden_emit_tdd.rs` are unmodified **against `target/pkt-188-baseline-ref.txt`**, the ref Step 0 records — not against `HEAD`, which is empty once this packet's work is committed and would let a committed edit through.
- Neither the manifest nor `INJECTION_POINTS` mentions any of the five unreachable points (AC-N3).
- Update `docs/07_implementation_status.md` through a worker dispatch, never a full backlog read.
- `DEV-085` is `Closed` and both residual rows exist.
- `packet.spec.md` is ready for `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed AC and packet-level gate command.
- Record remaining packet-local risk: the `;TYPE:` role site has no runtime tripwire (unlike packet 187's layer sites); confirm AC-7's one-`; PNP_ROLE`-per-`;TYPE:` count equality is green and that the limitation is stated in the module's doc comment.
- Confirm context stayed at or below 150k standard, or at/below 300k only with a logged swarm ESCALATION; otherwise record a packet-authoring lesson.

All `cargo check`, `cargo clippy`, and `cargo test` invocations in gate and verification commands must use `--all-targets` where the command accepts it, so the test, bench, and example targets compile.
