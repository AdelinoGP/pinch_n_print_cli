# Implementation Plan: 187-custom-gcode-injection-registry

## Execution Rules

- Work one atomic step at a time; map every step to grouped task IDs.
- Use TDD, then implementation, then the narrowest falsifying validation.
- Every field below is a context-budget contract and must be filled independently; never write "see Step 1".
- **Precondition for the whole packet:** packet `186-custom-gcode-placeholder-engine` (`TASK-305`) is `implemented`. If it is still `draft`, stop and report — do not reimplement its engine here.

### Step 0: Record the packet baseline ref

- Task IDs: `TASK-306`
- Objective: capture the commit this packet starts from, so every "this packet must not modify X" guard has a ref that survives the packet's own commits.
- Precondition: the working tree is at the commit from which this packet's work begins.
- Postcondition: **two** copies of the same SHA exist — the durable one at `.ralph/specs/187-custom-gcode-injection-registry/baseline-ref.txt` (version-controlled) and the working one at `target/pkt-187-baseline-ref.txt` (scratch cache, which is what every embedded guard command reads).
- **Why two, and the recovery rule.** `target/` is gitignored scratch and is destroyed by `cargo clean`, which this packet's own guest-WASM rebuild instructions make a realistic mid-packet event. With only the `target/` copy, a `cargo clean` mid-packet leaves AC-10 (and the `docs/ORCA_CONFIG_REFERENCE.md` no-touch guard, which uses the same ref) hard-failing with `FAIL: baseline ref … missing` and **no way to recover**: re-running Step 0 at that point records the *current* HEAD, which already contains this packet's edits, so the guard would then pass vacuously — the exact false-PASS the ref exists to prevent. **Recovery rule, mandatory: never re-run Step 0 to recreate a lost ref.** Restore the cache from the durable copy instead: `bash -c 'mkdir -p target && cp .ralph/specs/187-custom-gcode-injection-registry/baseline-ref.txt target/pkt-187-baseline-ref.txt && echo RESTORED'`. If the durable copy is *also* gone, stop and report — do not synthesise a ref.
- Files allowed to read, with ranges when over 300 lines:
  - none.
- Files allowed to edit (at most 3):
  - `.ralph/specs/187-custom-gcode-injection-registry/baseline-ref.txt` (new; version-controlled, so it survives `cargo clean`)
  - `target/pkt-187-baseline-ref.txt` (not under version control)
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
  - `bash -c 'git diff --quiet && git diff --cached --quiet || { echo "FAIL: working tree or index is dirty - commit or stash first, or the baseline bakes in edits this packet must be measured against"; exit 1; }; mkdir -p target && git rev-parse HEAD > .ralph/specs/187-custom-gcode-injection-registry/baseline-ref.txt && cp .ralph/specs/187-custom-gcode-injection-registry/baseline-ref.txt target/pkt-187-baseline-ref.txt && rg -q "^[0-9a-f]{40}$" target/pkt-187-baseline-ref.txt && echo PASS || echo "FAIL: baseline ref not recorded"'`
- Exit condition: both files exist and hold the same single SHA. **Every no-touch guard in this packet diffs against the `target/` copy.** Do not substitute `HEAD` (empty after the packet commits, so a committed edit passes) or `git merge-base HEAD master` (measured: `crates/slicer-gcode/src/emit.rs` already differs from that merge-base because of pre-existing branch work, so the guard would fail a correct implementation). If a guard later reports the ref missing, apply the recovery rule above — do **not** re-run this step.

### Step 0a: Re-measure the AC baselines against the post-186 tree

- Task IDs: `TASK-306`
- Objective: re-run every change-proving AC command **before** writing a line of this packet, and record which clauses are actually red now that packet 186 has landed. The AC baselines quoted throughout `packet.spec.md` were measured on the **pre-186** tree.
- **Why this step exists — a measured cross-packet interaction, not a formality.** AC-11 must be measured against the post-186 documentation rather than assuming its baseline. The injection-point section must state the current warn-and-pass contract: an unavailable per-site variable remains verbatim, the run returns `Ok`, and exactly one warning names the config key and site. If 186 has already removed or rewritten the old placeholder caveat, that clause is already green when this packet starts; the remaining injection-point and layer-variable clauses still discriminate, but the state must be *known*, not assumed.
- Precondition: packet 186 is `implemented` and its commits are in the tree; Step 0's baseline ref is recorded.
- Postcondition: a written record (in the packet's swarm log, not a repo file) of the PASS/FAIL state of each change-proving AC command, and an explicit note of any clause that is already green.
- Files allowed to read, with ranges when over 300 lines:
  - `docs/15_config_keys_reference.md` — **ranged read only**: the injection-point section, to see what 186 actually left behind.
- Files allowed to edit (at most 3):
  - none. This step only measures.
- Files explicitly out of bounds:
  - every source and doc file.
- Expected sub-agent dispatches:
  - Question: after 186, does `docs/15_config_keys_reference.md` still contain the string `The wider OrcaSlicer placeholder set` or a contradictory aborting placeholder-policy claim, and does it still contain a numeric claim about how many macros resolve? Scope: `docs/15_config_keys_reference.md`; return: `FACT` ≤ 3 lines
- Context cost: `S`
- Authoritative docs:
  - none.
- OrcaSlicer refs:
  - none.
- Verification:
  - Re-dispatch every pipe-suffixed command in `packet.spec.md` and record the result. Any command that prints **PASS** before this packet has written a line is a **non-discriminating AC**: say so explicitly in the step's report rather than treating the eventual PASS as evidence.
  - `bash -c 'python3 -c "import io; s=io.open(r"docs/15_config_keys_reference.md",encoding="utf-8").read(); print("stale caveat or contradictory aborting policy present (AC-11 clause still red): "+str("The wider OrcaSlicer placeholder set" in s or "abort" in s and "placeholder" in s))"'`
- Exit condition: the post-186 baseline is recorded, and every AC that turns out to be pre-satisfied is named. Do not amend the AC text to hide a pre-satisfied clause; AC-11 stays as written because its other clauses still discriminate.

### Step 1: Red tests for the registry and the layer-scoped points

- Task IDs: `TASK-306`
- Objective: add the seven module-level tests named by AC-3, AC-4, AC-5, AC-6, AC-N1, AC-N2 and AC-N3, each driving `run_gcode_postprocess` through the existing `run` / `raw_texts` helpers over a synthetic stream that contains the host's `;LAYER_CHANGE` / `;Z:` / `;HEIGHT:` marker triple.
- Precondition: `cargo test -p machine-gcode-emit --test machine_gcode_emit_tdd` is green on the post-186 tree.
- Postcondition: the seven tests exist and the binary is red on exactly those seven.
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/machine-gcode-emit/tests/machine_gcode_emit_tdd.rs` — whole file (short); the `run` and `raw_texts` helpers.
  - `modules/core-modules/machine-gcode-emit/src/lib.rs` — whole file (short); to name the constants the tests assert on.
  - `crates/slicer-gcode/src/emit.rs` — **long; ranged read only** — the layer-boundary block that pushes the three `Raw` markers, to copy the exact marker text into the fixtures.
- Files allowed to edit (at most 3):
  - `modules/core-modules/machine-gcode-emit/tests/machine_gcode_emit_tdd.rs`
- Files explicitly out of bounds:
  - `modules/core-modules/machine-gcode-emit/src/lib.rs` and its `.toml` (Steps 2 and 3 own them)
  - `crates/slicer-gcode/**` — read-only for the whole packet
  - `crates/slicer-runtime/**` (Step 3 owns the e2e half)
  - `OrcaSlicerDocumented/**`, `target/**`
- Expected sub-agent dispatches:
  - Question: in `GCode::process_layer`, what is the precise ordered list of items appended between the start of a layer and the first extrusion? Scope: `OrcaSlicerDocumented/src/libslic3r/GCode.cpp`; return: `SUMMARY` ≤ 200 words as an ordered list of named items, no source
  - Question: what extra placeholder variables does `s_CustomGcodeSpecificPlaceholders` list for `machine_start_gcode`, `before_layer_change_gcode`, `layer_change_gcode`, `timelapse_gcode` and `machine_end_gcode`? Scope: `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp`; return: `FACT` ≤ 6 lines
- Context cost: `S`
- Authoritative docs:
  - `docs/02_ir_schemas.md` — delegated SUMMARY only, for `GCodeCommand::Raw` and the postpass input surface.
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/GCode.cpp`, `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` — delegate; never load.
- Verification:
  - `bash -c 'cargo test -p machine-gcode-emit --test machine_gcode_emit_tdd 2>&1 | tee target/log-187-mge-red.txt | rg "^test result:" | rg -q "FAILED" && echo "RED (expected)" || echo "NOT RED — the new tests are not discriminating"'` — the only step whose success condition is a **red** binary; read `target/log-187-mge-red.txt` rather than re-running.
- Exit condition: the seven named tests exist, and the binary's failures are exactly those seven (any other failing test falsifies the step and must be diagnosed before Step 2).

### Step 2: Build the registry, migrate start/end onto it, and splice the layer-scoped points

- Task IDs: `TASK-306`
- Objective: add `InjectionSite`, `InjectionPoint`, `INJECTION_POINTS` (five entries), `LayerContext` and the `ERR_MALFORMED_LAYER_MARKER` warning identifier; rewrite `run_gcode_postprocess` to resolve every template through the table against a **per-site** lookup; preserve unavailable placeholders verbatim with one key/site warning and an `Ok` result; splice `before_layer_change_gcode`, `time_lapse_gcode` and `layer_change_gcode` immediately after each layer's `;HEIGHT:` marker, in that order.
- Precondition: Step 1's seven tests exist and are red for their own reasons.
- Postcondition: `cargo test -p machine-gcode-emit --test machine_gcode_emit_tdd` is fully green; `INJECTION_POINTS` drives every template read; the start and end blocks have not moved.
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/machine-gcode-emit/src/lib.rs` — whole file (short).
  - `modules/core-modules/machine-gcode-emit/tests/machine_gcode_emit_tdd.rs` — the seven tests from Step 1.
  - `crates/slicer-gcode/src/emit.rs` — **long; ranged read only** — the layer-boundary block and the comment above `let mut commands = vec![GCodeCommand::ExtrusionMode …]` explaining why `machine-gcode-emit` rebuilds rather than splices, which is why the start block keeps its position.
- Files allowed to edit (at most 3):
  - `modules/core-modules/machine-gcode-emit/src/lib.rs`
- Files explicitly out of bounds:
  - `crates/slicer-ir/**`, `crates/slicer-sdk/**`, `crates/slicer-schema/**`, `crates/slicer-macros/**` — editing any of these invalidates every guest's bindgen and is not required.
  - `crates/slicer-gcode/**` — read-only for the whole packet; AC-10 fails if `emit.rs` or `golden_emit_tdd.rs` is modified.
  - `crates/slicer-wasm-host/**`, `crates/slicer-runtime/src/**`
  - `docs/**` (Steps 4 and 5 own them)
  - `OrcaSlicerDocumented/**`, `target/**`
- Blast-radius discipline: this step adds no struct field to a shared type and bumps no schema/version constant, so the struct-literal sweep does not apply. The new types are private to `modules/core-modules/machine-gcode-emit/src/lib.rs`; the only cross-file surface is the `PostpassModule` trait impl, whose signature is unchanged. **The one non-obvious fallout is behavioural, not structural:** migrating `machine_start_gcode` / `machine_end_gcode` onto the table must not change where their blocks land, and the tests that would catch a move are split across **two crates** — `start_block_position_before_extrusion_mode_and_first_g1` and `end_block_position_after_last_g1_before_config_block` in `crates/slicer-runtime/tests/integration/machine_start_end_gcode_emission_tdd.rs`, but `machine_start_gcode_precedes_m73_and_extrusion_mode` in `modules/core-modules/machine-gcode-emit/tests/machine_gcode_emit_tdd.rs`. An earlier draft placed all three in the runtime crate; `cargo test -p slicer-runtime --test integration -- --list` does not contain the third. The first two are covered by AC-9 and run in Step 3; the third is covered by AC-8 and runs in **this** step's own binary, so a start-block move is caught here rather than one step later.
- Expected sub-agent dispatches:
  - Question: does `GCode::change_layer` emit a Z move outside spiral-vase mode? Scope: `OrcaSlicerDocumented/src/libslic3r/GCode.cpp`; return: `FACT` ≤ 3 lines
  - Question: does `cargo xtask build-guests --check` report `STALE:` after this edit? Scope: cargo run; return: `FACT` clean/stale
- Context cost: `M`
- Authoritative docs:
  - `docs/02_ir_schemas.md` — delegated SUMMARY only.
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/GCode.cpp` — `GCode::process_layer`, `GCode::change_layer`; delegate.
- Verification:
  - `cargo xtask build-guests --check` — FACT clean; rebuild without `--check` if `STALE:` before believing any later test result.
  - `bash -c 'cargo test -p machine-gcode-emit --test machine_gcode_emit_tdd 2>&1 | tee target/log-187-mge.txt | rg "^test result:" | rg -q "^test result: ok\. [1-9]" && echo PASS || echo "FAIL: see target/log-187-mge.txt"'`
  - `bash -c 'python3 -c "import io; s=io.open(r\"modules/core-modules/machine-gcode-emit/src/lib.rs\",encoding=\"utf-8\").read(); ok = \"INJECTION_POINTS\" in s and \"enum InjectionSite\" in s; b=s.find(\"INJECTION_POINTS\"); tbl=s[b:b+2000] if b>=0 else \"\"; keys=(\"machine_start_gcode\",\"before_layer_change_gcode\",\"time_lapse_gcode\",\"layer_change_gcode\",\"machine_end_gcode\"); miss=[k for k in keys if (chr(34)+k+chr(34)) not in tbl]; print(\"PASS\" if ok and not miss else \"FAIL: \"+str(miss))"'`
  - `bash -c 'rg -q "ERR_MALFORMED_LAYER_MARKER" modules/core-modules/machine-gcode-emit/src/lib.rs && echo PASS || echo "FAIL: ERR_MALFORMED_LAYER_MARKER diagnostic identifier does not exist"'`
- Exit condition: AC-1, AC-3, AC-4, AC-5, AC-6, AC-8, AC-N1, AC-N2 and AC-N3 all print PASS.

### Step 3: Declare the three keys and add the end-to-end layer-count pin

- Task IDs: `TASK-306`
- Objective: add the three `[config.schema.*]` string blocks and the `layer_change_gcode_fires_once_per_emitted_layer` e2e test, then prove the start/end migration did not move either block.
- Precondition: Step 2 is green and `cargo xtask build-guests --check` is clean.
- Postcondition: `[config.schema]` has eight keys; the whole `machine_start_end_gcode_emission_tdd` module is green.
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/machine-gcode-emit/machine-gcode-emit.toml` — whole file (short).
  - `crates/slicer-runtime/tests/integration/machine_start_end_gcode_emission_tdd.rs` — **long; ranged reads only.** Read `slice_with_raw` / `try_slice_with_raw`, `count_occurrences`, and the two block-position tests. Do not load the whole file. **`try_slice_with_raw` is a FORWARD-DEP on packet 186, not an existing symbol** — 186 adds it beside `slice_with_raw` in this file (its `design.md` §Code Change Surface and its `implementation-plan.md` Step 3 both specify the addition, re-expressing `slice_with_raw` as `try_slice_with_raw(raw).expect("pipeline must succeed")`). If it is absent when this step runs, 186 has not landed and this packet's stated precondition is violated — **stop and report; do not add it here.**
- Files allowed to edit (at most 3):
  - `modules/core-modules/machine-gcode-emit/machine-gcode-emit.toml`
  - `crates/slicer-runtime/tests/integration/machine_start_end_gcode_emission_tdd.rs`
- Files explicitly out of bounds:
  - `modules/core-modules/machine-gcode-emit/src/lib.rs` (Step 2 owns it)
  - `crates/slicer-gcode/**`, `crates/slicer-runtime/src/**`
  - `docs/**` (Steps 4 and 5 own them)
  - `OrcaSlicerDocumented/**`, `target/**`
- Blast-radius discipline: three new **string** keys change what `slice_with_raw` seeds — it iterates `machine_binding.module.config_schema().entries` generically and routes string defaults to `binding_source` as the real value and to `pipeline_source` as an empty sentinel, so a `default = ""` key needs no harness edit and yields a `; <key> = ` CONFIG_BLOCK line. Confirm before assuming: the count-shaped neighbours are `module_manifest_registers_five_keys_with_expected_types_and_defaults` and `new_keys_appear_in_config_block` (both assert **presence**, not a total) and `gcode_header_thumbnail_config_blocks_tdd`'s "at least 80 key-value lines" **lower bound**. If any turns out to assert a total, it belongs in this step's edit list.
- Expected sub-agent dispatches:
  - Question: does `cargo xtask build-guests --check` report `STALE:` after the manifest edit? Scope: cargo run; return: `FACT` clean/stale
  - Question: does any test under `crates/slicer-runtime/tests/` assert an exact total number of CONFIG_BLOCK key lines or an exact `machine-gcode-emit` schema length? Scope: `crates/slicer-runtime/tests/**`; return: `LOCATIONS` ≤ 20
- Context cost: `M`
- Authoritative docs:
  - `docs/15_config_keys_reference.md` — the generated `module-config-keys` marker boundaries only; ranged read.
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` — `PrintConfigDef::init_fff_params`; delegate; confirms the three options are `coString` and their upstream defaults.
- Verification:
  - `cargo xtask build-guests --check` — FACT clean; rebuild if `STALE:`.
  - `bash -c 'python3 -c "import tomllib; d=tomllib.load(open(r\"modules/core-modules/machine-gcode-emit/machine-gcode-emit.toml\",\"rb\"))[\"config\"][\"schema\"]; want=(\"before_layer_change_gcode\",\"layer_change_gcode\",\"time_lapse_gcode\"); bad=[k for k in want if not (k in d and d[k][\"type\"]==\"string\" and d[k].get(\"default\")==\"\" and d[k].get(\"group\")==\"Machine G-code\")]; print(\"PASS\" if not bad and len(d)==8 else \"FAIL: \"+str(bad)+\" ; keys=\"+str(sorted(d)))"'`
  - `bash -c 'cargo test -p slicer-runtime --test integration -- machine_start_end_gcode_emission_tdd:: 2>&1 | tee target/log-187-msege.txt | rg "^test result:" | rg -q "^test result: ok\. [1-9]" && echo PASS || echo "FAIL: see target/log-187-msege.txt"'`
  - `bash -c 'cargo test -p slicer-gcode --test golden_emit_tdd 2>&1 | rg "^test result:" | rg -q "^test result: ok\. [1-9]" && echo PASS || echo "FAIL: the host marker golden regressed"'`
  - `bash -c 'BASE=$(cat target/pkt-187-baseline-ref.txt 2>/dev/null); { [ -n "$BASE" ] && git rev-parse --verify -q "$BASE^{commit}" >/dev/null; } || { echo "FAIL: baseline ref target/pkt-187-baseline-ref.txt missing, empty, or not a valid commit - run Step 0 before this guard"; exit 1; }; git diff --name-only "$BASE" -- crates/slicer-gcode/src/emit.rs crates/slicer-gcode/tests/golden_emit_tdd.rs  | rg -q . && echo "FAIL: emitter or golden modified" || echo PASS'`
- Exit condition: AC-2, AC-7, AC-9 and AC-10 all print PASS.

### Step 4: Rewrite the `docs/15` injection-point section and regenerate the config-keys block

- Task IDs: `TASK-306`
- Objective: retitle §"Machine start / end G-code (packet 59)" to `## Custom G-code injection points`, write the literal anchor `<!-- anchor: custom-gcode-injection-points -->` on the line immediately below the new heading, and extend the section into a registry-shaped one that names all five registered points, states the canonical layer-boundary order, and documents that `[layer_num]` / `[layer_z]` / `[max_layer_z]` resolve only at the four layer-aware sites; then regenerate the `module-config-keys` block.
- **The anchor is what makes the retitle and AC-11 compatible.** AC-11 slices the section from the anchor, not from the heading text; without the anchor the retitle blanks AC-11's section slice and fails every one of its content clauses. Place the anchor outside every `<!-- BEGIN GENERATED … -->` span.
- **State the placeholder rule, not a numeric count.** Packet 186 rewrote this same section and this packet adds three more manifest keys to the same module, so any inherited numeral would be brittle. Write the **rule** instead — the placeholder domain is this module's manifest-declared key set plus the alias table, per `docs/adr/0050-custom-gcode-architecture.md`; an unavailable per-site variable remains verbatim, the run returns `Ok`, and exactly one warning names the config key and site — then enumerate the injection points and the per-site variables, which are the facts AC-11 actually probes. This is a **cross-packet coordination point with 186 Step 4**: if the section arrives with a count or contradictory policy sentence, replace it here rather than adjusting the count.
- Precondition: Step 3 landed, so the manifest is final.
- Postcondition: `cargo xtask gen-config-docs --check` exits 0; the anchor `<!-- anchor: custom-gcode-injection-points -->` exists exactly once and outside every generated span; the three new key names and the three layer macros appear inside the anchored section; the section states **no** numeric total of resolvable placeholders; and it states the warn-and-pass behavior for unavailable per-site variables.
- Files allowed to read, with ranges when over 300 lines:
  - `docs/15_config_keys_reference.md` — **long; ranged reads only.** §"Machine start / end G-code" and the `<!-- BEGIN GENERATED: module-config-keys (cargo xtask gen-config-docs) -->` / `<!-- END GENERATED: module-config-keys -->` marker lines.
- Files allowed to edit (at most 3):
  - `docs/15_config_keys_reference.md`
- Files explicitly out of bounds:
  - Anything between the `module-config-keys` markers — regenerate, never hand-edit.
  - `docs/ORCA_CONFIG_REFERENCE.md` — deliberately untouched.
  - `docs/DEVIATION_LOG.md`, `docs/07_implementation_status.md` (Step 5 owns them)
  - `modules/**`, `crates/**`
- Expected sub-agent dispatches:
  - Question: does `cargo xtask gen-config-docs --check` exit 0 after regeneration, and does the generated table pair each of the three new keys with `machine-gcode-emit`? Scope: cargo run + `docs/15_config_keys_reference.md`; return: `FACT` pass/fail
- Context cost: `S`
- Authoritative docs:
  - `docs/15_config_keys_reference.md` — the section being rewritten; ranged read.
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` — `custom_gcode_specific_placeholders`; delegate; supplies the per-site variable sets quoted in the rewritten prose, including the note that the table itself keys timelapse as `timelapse_gcode` while the option is `time_lapse_gcode`.
- Verification:
  - `bash -c 'python3 -c "import io; s=io.open(r\"docs/15_config_keys_reference.md\",encoding=\"utf-8\").read(); b=chr(96); a=\"<!-- anchor: custom-gcode-injection-points -->\"; i=s.find(a); sec=s[i:] if i>=0 else \"\"; j=sec.find(chr(10)+\"## \",1); sec=sec[:j] if j>0 else sec; pts=[k for k in (\"before_layer_change_gcode\",\"time_lapse_gcode\",\"layer_change_gcode\") if k not in sec]; mac=[v for v in (\"layer_num\",\"layer_z\",\"max_layer_z\") if (b+chr(91)+v+chr(93)+b) not in sec]; stale=\"The wider OrcaSlicer placeholder set\" in s; policy=\"remains verbatim\" in sec and \"returns `Ok`\" in sec and \"one warning\" in sec; ok=i>=0 and not pts and not mac and not stale and policy; print(\"PASS\" if ok else \"FAIL: anchor=\"+str(i>=0)+\", missing points=\"+str(pts)+\", missing macros=\"+str(mac)+\", stale=\"+str(stale)+\", policy=\"+str(policy))"'` — **the AC-11 section probe; keep it in sync with `packet.spec.md`.** It checks the stable anchor, the registered point names, the layer variables, the absence of the stale caveat, and the warn-and-pass wording.
  - `bash -c 'python3 -c "import io; s=io.open(r\"docs/15_config_keys_reference.md\",encoding=\"utf-8\").read(); a=\"<!-- anchor: custom-gcode-injection-points -->\"; n=s.count(a); print(\"PASS\" if n==1 else \"FAIL: anchor occurs \"+str(n)+\" times; expected exactly 1\")"'` — the anchor is now load-bearing for AC-11 here and for packet 188's doc AC; a duplicate or a missing one silently changes which text both probes read.
  - `bash -c 'cargo xtask gen-config-docs --check >/dev/null 2>&1 && python3 -c "import io; s=io.open(r\"docs/15_config_keys_reference.md\",encoding=\"utf-8\").read(); b=chr(96); p=chr(124); rows=[ln for ln in s.splitlines() if ln.startswith(p)]; miss=[k for k in (\"before_layer_change_gcode\",\"layer_change_gcode\",\"time_lapse_gcode\") if not any((b+k+b) in ln and (b+\"machine-gcode-emit\"+b) in ln for ln in rows)]; print(\"PASS\" if not miss else \"FAIL: no generated row pairs \"+str(miss)+\" with machine-gcode-emit\")" || echo "FAIL: gen-config-docs --check is red"'` — **verbatim copy of `packet.spec.md` AC-12; if either changes, change both.** The bare `cargo xtask gen-config-docs --check` that stood here is green today, so it could not fail; AC-12 keeps that guard and adds the generated-row clauses that actually discriminate.
- Exit condition: AC-11 and AC-12 both print PASS.

### Step 5: Update `DEV-085`, file the residual row, register `TASK-306`

- Task IDs: `TASK-306`
- Objective: record the three newly-implemented points on the `DEV-085` row (citing `TASK-306`, keeping it `Open`); file one new `DEV-###` row carrying **all four** accepted parity residuals — AC-13 items (a)+(a2) the `max_layer_z` dead-write and the *no*-divergence finding at the other two sites, (b) the unported BBL timelapse path, (c) the six unmodelled `layer_change_gcode` variables, (d) the `change_layer` interleaving difference; hand-add the `TASK-306` backlog row outside the generated block and regenerate that block.
- Precondition: Steps 1-4 complete.
- Postcondition: the `DEV-085` row cites `TASK-306` and is still `Open`; the residual row names `is_BBL_Printer`, `generate_timelapse_gcode`, `timelapse_inline_photo` and `max_layer_z`; `TASK-306` resolves in `docs/07_implementation_status.md`.
- Files allowed to read, with ranges when over 300 lines:
  - `docs/DEVIATION_LOG.md` — **long; delegate or range-read.** The `DEV-085` row, the two most recent rows for format, and a re-derivation of the highest `DEV-###`.
  - `docs/07_implementation_status.md` — **always delegate.**
- Files allowed to edit (at most 3):
  - `docs/DEVIATION_LOG.md`
  - `docs/07_implementation_status.md`
- Files explicitly out of bounds:
  - The `<!-- BEGIN GENERATED: open-deviations (cargo xtask check-deviations) -->` … `<!-- END GENERATED: open-deviations -->` span of `docs/07_implementation_status.md` — regenerate with `cargo xtask check-deviations`, never hand-edit.
  - `docs/15_config_keys_reference.md` (Step 4 owns it)
  - `modules/**`, `crates/**`
- Expected sub-agent dispatches:
  - Question: what is the highest `DEV-###` in `docs/DEVIATION_LOG.md` right now? Scope: `rg -o '^\| DEV-[0-9]{3}' docs/DEVIATION_LOG.md | sort -u | tail -1`; return: `FACT` one line. **Re-derive at the moment of writing — parallel packets file rows concurrently.**
  - Question: which variables does `GCode::generate_timelapse_gcode` set that the inline non-BBL path does not? Scope: `OrcaSlicerDocumented/src/libslic3r/GCode.cpp`; return: `FACT` ≤ 5 lines
- Context cost: `S`
- Authoritative docs:
  - `docs/DEVIATION_LOG.md`, `docs/07_implementation_status.md` — delegated; row formats and next free ID.
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/GCode.cpp` — `GCode::process_layer`, `GCode::generate_timelapse_gcode`; delegate; supply the four residuals' evidence (AC-13 items a/a2, b, c and d).
- Verification:
  - `bash -c 'python3 -c "import io; L=io.open(r\"docs/DEVIATION_LOG.md\",encoding=\"utf-8\").read().splitlines(); r=[l for l in L if l.startswith(chr(124)+\" DEV-085\")]; print(\"PASS\" if r and \"TASK-306\" in r[0] and \"Open\" in r[0] else (\"FAIL: no DEV-085 row\" if not r else \"FAIL: DEV-085 row does not cite TASK-306, or is no longer Open\"))"'` — **verbatim copy of `packet.spec.md` §Doc Impact's `DEV-085` row probe; if either changes, change both.**
  - `bash -c 'python3 -c "import io; p=chr(124); L=io.open(r\"docs/DEVIATION_LOG.md\",encoding=\"utf-8\").read().splitlines(); need=(\"max_layer_z\",\"layer_change_gcode\",\"dead-write\",\"m_max_layer_z\",\"exact parity\",\"generate_timelapse_gcode\",\"timelapse_inline_photo\",\"is_BBL_Printer\",\"curr_accumulated_mass\",\"add_object_change_labels\"); rows=[l for l in L if l.startswith(p+chr(32)+\"DEV-\") and not l.startswith(p+chr(32)+\"DEV-085\")]; hit=[l for l in rows if all(t in l for t in need)]; best=max(rows,key=lambda l:sum(t in l for t in need),default=\"\"); print(\"PASS\" if hit else \"FAIL: no single new DEV row carries all tokens; best row misses \"+str([t for t in need if t not in best]))"'` — **verbatim copy of `packet.spec.md` AC-13; if either changes, change both.** An earlier draft left the superseded round-1 probe here, so a worker gating on this step used a weaker check than the AC it claims to satisfy.
  - `bash -c 'python3 -c "import io; L=io.open(r\"docs/07_implementation_status.md\",encoding=\"utf-8\").read().splitlines(); B=[i for i,l in enumerate(L) if l.startswith(\"<!-- BEGIN GENERATED: open-deviations\")]; E=[i for i,l in enumerate(L) if l.startswith(\"<!-- END GENERATED: open-deviations\")]; H=[i for i,l in enumerate(L) if \"TASK-306\" in l]; print(\"FAIL: open-deviations markers not found\" if not (B and E) else (\"FAIL: TASK-306 not registered anywhere\" if not H else (\"FAIL: TASK-306 appears only INSIDE the generated block\" if all(B[0]<i<E[0] for i in H) else \"PASS\")))"'`
- Exit condition: AC-13, AC-14 and the `DEV-085` row probe all print PASS.

### Step 6: Closure gates

- Task IDs: `TASK-306`
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
- Exit condition: both gates exit 0, `build-guests --check` reports no `STALE:`, and all seventeen numbered AC commands (AC-1..AC-14, AC-N1..AC-N3) print PASS. Note Step 0's baseline ref must still exist at `target/pkt-187-baseline-ref.txt`; AC-10's no-touch guard reads it. If `cargo clean` removed it at any point, restore it from `.ralph/specs/187-custom-gcode-injection-registry/baseline-ref.txt` per Step 0's recovery rule — **never** by re-running Step 0, which would record a HEAD that already contains this packet's edits and make the guard pass vacuously.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 0 | S | Records the baseline SHA twice: durably at `.ralph/specs/187-custom-gcode-injection-registry/baseline-ref.txt`, cached at `target/pkt-187-baseline-ref.txt`. |
| Step 0a | S | Measurement only: re-runs the AC baselines against the post-186 tree and names any pre-satisfied clause. |
| Step 1 | S | One short test file; two delegated canonical dispatches. |
| Step 2 | M | Registry + site walk + per-site lookup + start/end migration, plus the guest-freshness gate. |
| Step 3 | M | Manifest edit, ranged reads of a long e2e test file, guest rebuild, emitter no-touch proof. |
| Step 4 | S | One ranged doc section plus a generator run. |
| Step 5 | S | Two delegated doc reads plus one canonical FACT; no code. |
| Step 6 | S | Measurement only. |

Split before activation if aggregate cost exceeds M or any step is L.

## Packet Completion Gate

- All steps and exits complete.
- Every pipe-suffixed AC command returns PASS.
- `cargo xtask build-guests --check` reports no `STALE:` as the last action before closure.
- `crates/slicer-gcode/src/emit.rs` and `crates/slicer-gcode/tests/golden_emit_tdd.rs` are unmodified **against `target/pkt-187-baseline-ref.txt`**, the ref Step 0 records — not against `HEAD`, which is empty once this packet's work is committed and would let a committed edit through.
- Update `docs/07_implementation_status.md` through a worker dispatch, never a full backlog read.
- `DEV-085` stays `Open` — packet 188 carries the toolchange-, role- and unreachable-site remainder. Do not flip it.
- `packet.spec.md` is ready for `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed AC and packet-level gate command.
- Record remaining packet-local risk: the module now depends on host-emitted comment text (`;LAYER_CHANGE` / `;Z:` / `;HEIGHT:`); confirm AC-N2 exercises the `ERR_MALFORMED_LAYER_MARKER` warning and prior-Z recovery, and that the dependency is stated in the module's doc comment.
- Confirm context stayed at or below 150k standard, or at/below 300k only with a logged swarm ESCALATION; otherwise record a packet-authoring lesson.

All `cargo check`, `cargo clippy`, and `cargo test` invocations in gate and verification commands must use `--all-targets` where the command accepts it, so the test, bench, and example targets compile.
