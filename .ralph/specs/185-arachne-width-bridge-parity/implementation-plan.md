# Implementation Plan: 185-arachne-width-bridge-parity

## Execution Rules

- Work one atomic step at a time; map every step to grouped task IDs.
- Use TDD, then implementation, then the narrowest falsifying validation.
- Every field below is a context-budget contract and must be filled independently; never write "see Step 1".

## Steps

### Step 1: Retype the arachne wall-width keys to `float_or_percent` and resolve them canonically (D-164, arachne half)

- Task IDs: `TASK-304`
- Objective: make the canonical `coFloatOrPercent` shape of `outer_wall_line_width` / `inner_wall_line_width` declarable and validated on `arachne-perimeters`, and resolve an explicit `0` through canonical `Flow::auto_extrusion_width`'s `1.125 × nozzle_diameter` — without moving default-config geometry.
- Precondition: `[config.schema.outer_wall_line_width]` and `[config.schema.inner_wall_line_width]` are `type = "float"`, `default = 0.4`, `min = 0.1`, `max = 2.0`, no `unit`; both read sites in `arachne_params_from_config` use `config.get_float(key).unwrap_or(defaults.*)` and already pipe the result through `line_width_to_spacing` (D-162, landed).
- Postcondition: both manifest blocks are `type = "float_or_percent"`, `default = 0.4` (bare TOML number), `min = 0.0`, `max = 2.0`, `unit = "mm"`, with descriptions naming canonical `PrintConfig.cpp::PrintConfigDef::init_fff_params`'s `coFloatOrPercent` / `ratio_over = "nozzle_diameter"` / upstream default `0`; both read sites resolve via `config.get_abs_value(key, nozzle_diameter_mm)` with the three-arm match from `design.md` §Code Change Surface; the existing `line_width_to_spacing` conversion, its `ERR_NEGATIVE_SPACING` fatal mapping, and `preferred_bead_width_outer_raw`'s separate retention for the `precise_outer_wall` inset formula are all **untouched**; three new in-crate unit tests exist (AC-2, AC-3, AC-N1).
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/arachne-perimeters/src/lib.rs` — long; ranged: `fn arachne_params_from_config` through its `Ok(ArachneParams { … })`, and the trailing `#[cfg(test)] mod`
  - `modules/core-modules/arachne-perimeters/arachne-perimeters.toml` — long; ranged: `[config.schema.outer_wall_line_width]`, `[config.schema.inner_wall_line_width]`, and `[config.schema.overhang_reverse_threshold]` + `[config.schema.min_width_top_surface]` as the `float_or_percent` block-shape precedent
  - `crates/slicer-core/src/flow.rs` — `line_width_to_spacing` and `flow_to_width` only
  - `crates/slicer-ir/src/slice_ir.rs` — long; ranged: `ConfigView::get_abs_value` and `ConfigView::get_float` only (there is **no** `crates/slicer-ir/src/config_view.rs`; both methods live on the `impl ConfigView` block in `slice_ir.rs` — verified on disk. Resolve by symbol, never by guessed filename.)
- Files allowed to edit (at most 3):
  - `modules/core-modules/arachne-perimeters/arachne-perimeters.toml`
  - `modules/core-modules/arachne-perimeters/src/lib.rs`
- Files explicitly out of bounds:
  - `modules/core-modules/classic-perimeters/**` and `classic-perimeters.toml` — packet 184's surface
  - `crates/slicer-ir/src/resolved_config.rs` — read-only reference for the residual; never edit
  - `OrcaSlicerDocumented/**` — delegate
  - `docs/**` — Steps 5a/5b own all doc edits
- Blast-radius discipline: no struct field or schema version constant is added. **Manifest-default blast radius:** `crates/slicer-runtime/tests/integration/manifest_default_reconcile_tdd.rs`'s `ARACHNE_FALLBACKS` pins `("outer_wall_line_width", Float(0.4))` and `("inner_wall_line_width", Float(0.4))` with **set-equality in both directions**. Because this step keeps `default = 0.4` as a bare TOML number, `assert_exhaustive_reconcile`'s `Float` arm (`default.as_float().or_else(as_integer)`) still resolves and **no edit to that file is needed** — verified against the tree during authoring. Writing `default = "0.4"` instead would panic that arm with "`outer_wall_line_width` default is not numeric"; run the AC-7 command in this step to prove it did not.
- Expected sub-agent dispatches:
  - Question: does `Flow::auto_extrusion_width` return `1.125f * nozzle_diameter` for `frExternalPerimeter`/`frPerimeter`, and does `Flow::new_from_config_width` route `!percent && value <= 0` to it?; scope: `OrcaSlicerDocumented/src/libslic3r/Flow.cpp`; return: `SUMMARY` ≤120 words, ≤15 lines of C++
  - Question: where is `ConfigView::get_abs_value` defined and what are its exact match arms?; scope: `crates/slicer-ir/src/**`; return: `LOCATIONS` ≤5 entries plus the fn body
- Context cost: `S`
- Authoritative docs:
  - `docs/15_config_keys_reference.md` — delegated SUMMARY of the `inner_wall_line_width` / `min_width_top_surface` prose paragraphs only, for the description wording precedent. No edit in this step.
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/Flow.cpp` — `Flow::new_from_config_width`, `Flow::auto_extrusion_width`; delegate, never load
  - `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` — `PrintConfigDef::init_fff_params`; delegate, never load
- Verification:
  - `bash -c 'python3 -c "import tomllib; d=tomllib.load(open(r\"modules/core-modules/arachne-perimeters/arachne-perimeters.toml\",\"rb\"))[\"config\"][\"schema\"]; bad=[k for k in (\"outer_wall_line_width\",\"inner_wall_line_width\") if not (d[k][\"type\"]==\"float_or_percent\" and isinstance(d[k][\"default\"],float) and abs(d[k][\"default\"]-0.4)<1e-9 and abs(d[k][\"min\"])<1e-9 and abs(d[k][\"max\"]-2.0)<1e-9 and d[k].get(\"unit\")==\"mm\")]; print(\"FAIL: \"+str(bad) if bad else \"PASS\")"'` — AC-1, AC-N3; FACT PASS/FAIL
  - `bash -c 'rg -q "get_abs_value\(\s*\"inner_wall_line_width\"" modules/core-modules/arachne-perimeters/src/lib.rs && rg -q "get_abs_value\(\s*\"outer_wall_line_width\"" modules/core-modules/arachne-perimeters/src/lib.rs && ! rg -q "get_float\(\"(inner|outer)_wall_line_width\"\)" modules/core-modules/arachne-perimeters/src/lib.rs && rg -q "line_width_to_spacing" modules/core-modules/arachne-perimeters/src/lib.rs && echo PASS || echo FAIL'` — AC-4; FACT PASS/FAIL
  - `bash -c 'cargo test -p arachne-perimeters --lib 2>&1 | rg "^test result:" | rg -q "^test result: ok\. [1-9]" && echo PASS || echo "FAIL: the arachne-perimeters lib suite did not run clean"'` — AC-2, AC-3, AC-N1; `--lib` selects one binary, so this takes the single-result-line guard form; FACT pass/fail, bounded failure SNIPPETS ≤20 lines
  - `bash -c 'cargo test -p slicer-runtime --test integration -- manifest_default_reconcile_tdd::arachne_manifest_defaults_are_the_code_fallbacks --exact 2>&1 | rg "^test result:" | rg -q "^test result: ok\. [1-9]" && echo PASS || echo "FAIL: the selected test did not run or did not pass"'` — AC-7 half; proves the numeric-default requirement held
  - `bash -c 'cargo test -p slicer-runtime --test arachne_parity 2>&1 | rg "^test result:" | rg -q "^test result: ok\. [1-9]" && echo PASS || echo "FAIL: arachne_parity did not run clean"'` — the manifest negative-substring guards (`detect_thin_wall`'s description must not contain `printconfig.cpp` case-insensitively; `[module].description` must not contain `walls not yet produced`; `[module].display-name` must not contain `skeleton`). The new width descriptions name `PrintConfig.cpp`, which is a **different key's** description and therefore safe — this command proves it.
  - `cargo xtask build-guests --check` — guest freshness after editing `modules/core-modules/arachne-perimeters/src/**`; FACT `clean` / `STALE:` list
- Exit condition: AC-1, AC-2, AC-3, AC-4 and AC-N3 pass; `arachne_manifest_defaults_are_the_code_fallbacks` and `arachne_parity` still pass; `build-guests --check` reports clean.

### Step 2a: Move `ArachneParams`' wall-simplification tolerance constants to canonical and correct the falsified provenance (D-168, code half)

**Step 2 is split into 2a and 2b so that each half honours the three-file edit cap.** The two halves together are what other artifacts call "Step 2"; they must run adjacent and in order, and neither is independently shippable (2a moves the constant, 2b brings the declaration surface and its guard test into lockstep with it). Downstream step numbering is unchanged by this split; Step 5 is separately split into 5a/5b for the same reason (see its header).

- Task IDs: `TASK-304`
- Objective: replace the 10×/5× tighter code fallbacks with canonical's `wall_maximum_resolution = 0.5 mm` / `wall_maximum_deviation = 0.025 mm` (squared: `0.25` / `0.000625`) in **both** copies of the `ArachneParams` default literal, and correct the three provenance comments that misattribute the values to `meshfix_*` constants. The manifest, the exhaustive reconcile table and the new fallback unit test are **Step 2b's**.
- Precondition: Step 1 is complete. (Step 1 and Step 2b both edit `modules/core-modules/arachne-perimeters/src/lib.rs`; running 1 → 2a → 2b adjacently avoids re-reading `arachne_params_from_config`. Step 2a itself touches neither the module nor its manifest.) `ArachneParams::default` sets `smallest_line_segment_squared: 0.0025` / `allowed_error_distance_squared: 0.000025` in `crates/slicer-core/src/arachne/pipeline.rs`, with a duplicate literal in `crates/slicer-sdk/src/host.rs`.
- Postcondition: both constants are `0.25` / `0.000625` in **both** `pipeline.rs` and `host.rs`; the two provenance comments no longer claim `meshfix_maximum_resolution = 0.05mm` / `meshfix_maximum_deviation = 0.005mm` and instead cite canonical `PrintConfigDef::init_fff_params` and `WallToolPaths::simplifyToolPaths`; the third comment on `maximum_extrusion_area_deviation` is corrected per `design.md` §Open Questions to record canonical's `scaled<coord_t>(2.)` (value unchanged at `0.005`). **The value divergence is resolved at authoring, not left open: canonical is ~400× looser, so a SECOND new `DEV-###` row is required in addition to the one the D-164 residuals get — §Doc Impact allocates one row for D-164 and one for this. `Step 5a` owns both** (it owns every `docs/DEVIATION_LOG.md` edit in the packet); do not file either row from this step, and do not carry an ID forward from here — each is re-derived at the moment of writing. Because `arachne_params_from_config`'s two `unwrap_or(defaults.…)` fallbacks read `ArachneParams::default()` rather than local literals (verified against the tree), this step alone is what makes AC-8's assertion true; 2b adds the test that states it.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-core/src/arachne/pipeline.rs` — long; ranged: the `impl Default for ArachneParams` body and the three tolerance field doc comments on `pub struct ArachneParams`. Do not read the pipeline body.
  - `crates/slicer-sdk/src/host.rs` — long; ranged: the body of `impl Default for ArachneParams` only. This is a **second `impl Default` on a mirror struct** of the same name, not a bare struct literal — `host.rs` declares its own `ArachneParams` and its own `Default` impl, which is why the two copies can drift and why AC-6 pins both
  - `crates/slicer-core/tests/arachne_simplify_distance_gates.rs` and `arachne_simplify_intersection_distance_gate_tdd.rs` — **read only, do not edit.** Verified during authoring: both pass the tolerances as literal arguments to `simplify_toolpaths` and never read `ArachneParams::default()`, so they are unaffected. Listed here so a reviewer does not read the omission as an oversight.
  - `crates/slicer-core/src/arachne/simplify.rs` — `ultra_short_threshold` only, to confirm it is an independent `ExtrusionLine.cpp` ~5 µm epsilon and **not** `allowed_error_distance_squared`. Do not edit.
- Files allowed to edit (at most 3):
  - `crates/slicer-core/src/arachne/pipeline.rs`
  - `crates/slicer-sdk/src/host.rs` (two-line default sync)
- Files explicitly out of bounds:
  - `crates/slicer-core/src/arachne/simplify.rs` — read-only
  - `crates/slicer-core/tests/arachne_simplify_*.rs` — read-only
  - `modules/core-modules/arachne-perimeters/**` and `crates/slicer-runtime/tests/integration/manifest_default_reconcile_tdd.rs` — Step 2b
  - `modules/core-modules/classic-perimeters/**`, `classic-perimeters.toml`, and `CLASSIC_FALLBACKS`
  - `crates/slicer-runtime/tests/arachne_structural_invariants.rs` — Step 3 evaluates it; **never weaken `COVERAGE_THRESHOLD`**
  - `docs/**` — Steps 5a/5b
- Blast-radius discipline: no struct field is added, but a **default constant** changes, so the blast radius is behavioural rather than compile-time. Dispatch a `LOCATIONS` worker for every `ArachneParams::default` site before editing and cite the result inline. Sampled during authoring (re-derive — this is a ledger fact): `crates/slicer-core/src/arachne/pipeline.rs`, `crates/slicer-sdk/src/host.rs`, ten `crates/slicer-core/tests/arachne_*.rs` binaries, `crates/slicer-runtime/tests/arachne_parity.rs`, `arachne_parity_round2.rs`, `executor/arachne_perimeters_simple_square.rs`, `integration/manifest_default_reconcile_tdd.rs`, plus `arachne-perimeters/src/lib.rs`. Only the two source copies are **edited** here; the declaration surface is 2b's and the behavioural fallout is Step 3's budget by design, because absorbing it here would make the step `L`.
- Expected sub-agent dispatches:
  - Question: `wall_maximum_resolution` / `wall_maximum_deviation` default+min+max in `PrintConfigDef::init_fff_params`, and the return values of `meshfix_maximum_resolution()`, `meshfix_maximum_deviation()`, `meshfix_maximum_extrusion_area_deviation()` in `Arachne/WallToolPaths.hpp`; scope: `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp`, `.../Arachne/WallToolPaths.hpp`; return: `FACT` (six values)
  - Question: does `WallToolPaths::simplifyToolPaths` square `params.wall_maximum_resolution` / `params.wall_maximum_deviation` (and not the `meshfix_*` pair)?; scope: `OrcaSlicerDocumented/src/libslic3r/Arachne/WallToolPaths.cpp`; return: `SUMMARY` ≤100 words, ≤10 lines of C++
- Context cost: `S`
- Authoritative docs:
  - `docs/DEVIATION_LOG.md` — the D-168 row only, delegated; needed verbatim so Step 5a can correct its provenance clause. No edit in this step.
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` — `PrintConfigDef::init_fff_params`; delegate, never load
  - `OrcaSlicerDocumented/src/libslic3r/Arachne/WallToolPaths.hpp` — `WallToolPathsParams`, `meshfix_maximum_resolution`, `meshfix_maximum_deviation`, `meshfix_maximum_extrusion_area_deviation`; delegate, never load
  - `OrcaSlicerDocumented/src/libslic3r/Arachne/WallToolPaths.cpp` — `simplifyToolPaths`; delegate, never load
- Verification:
  - `bash -c 'rg -q "smallest_line_segment_squared: 0\.25," crates/slicer-core/src/arachne/pipeline.rs && rg -q "allowed_error_distance_squared: 0\.000625," crates/slicer-core/src/arachne/pipeline.rs && ! rg -q "smallest_line_segment_squared: 0\.0025," crates/slicer-core/src/arachne/pipeline.rs && ! rg -q "allowed_error_distance_squared: 0\.000025," crates/slicer-core/src/arachne/pipeline.rs && ! rg -q "meshfix_maximum_resolution = 0\.05mm" crates/slicer-core/src/arachne/pipeline.rs && ! rg -q "meshfix_maximum_deviation = 0\.005mm" crates/slicer-core/src/arachne/pipeline.rs && echo PASS || echo FAIL'` — AC-5; FACT PASS/FAIL
  - `bash -c 'rg -q "smallest_line_segment_squared: 0\.25," crates/slicer-sdk/src/host.rs && rg -q "allowed_error_distance_squared: 0\.000625," crates/slicer-sdk/src/host.rs && ! rg -q "smallest_line_segment_squared: 0\.0025," crates/slicer-sdk/src/host.rs && echo PASS || echo FAIL'` — AC-6; FACT PASS/FAIL
  - `cargo check --workspace --all-targets` — FACT pass/fail
- Exit condition: AC-5 and AC-6 pass; `cargo check --workspace --all-targets` is clean. AC-7 and AC-8 are **not** yet asserted — 2b owns them — and behavioural fallout is explicitly Step 3's.

### Step 2b: Bring the manifest, the exhaustive reconcile table and the fallback unit test into lockstep (D-168, declaration half)

- Task IDs: `TASK-304`
- Objective: move the two manifest defaults to `0.5` / `0.025` with rewritten descriptions, repoint `ARACHNE_FALLBACKS`' two arachne rows at the same pair, and add the in-crate unit test that asserts the **fallback** path Step 2a just changed.
- Precondition: Step 2a is complete (AC-5 and AC-6 pass). The manifest declares `wall_maximum_resolution` default `0.05` and `wall_maximum_deviation` default `0.005`; `ARACHNE_FALLBACKS` pins `Float(0.05)` / `Float(0.005)` and therefore still *matches* the manifest, so `arachne_manifest_defaults_are_the_code_fallbacks` is green entering this step (verified against the tree) — it is the manifest **and** the table moving together that keeps it green on exit.
- Postcondition: the manifest defaults are `0.5` / `0.025` with rewritten descriptions (the current text asserting "The previous 0.5 here was a 10x lie against the code fallback" — ASCII `10x`, as it appears in the manifest — is now false and must go); `ARACHNE_FALLBACKS` reads `Float(0.5)` / `Float(0.025)`; a new in-crate unit test `wall_simplify_fallbacks_are_canonical_defaults` asserts the **defaults** (AC-8) — the pre-existing `wall_maximum_resolution_wired` supplies both keys explicitly and proves nothing about the fallbacks, verified against the tree.
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/arachne-perimeters/src/lib.rs` — ranged: the `wall_maximum_resolution` / `wall_maximum_deviation` read block inside `arachne_params_from_config`, and the trailing `#[cfg(test)] mod`
  - `modules/core-modules/arachne-perimeters/arachne-perimeters.toml` — ranged: `[config.schema.wall_maximum_resolution]` and `[config.schema.wall_maximum_deviation]` only
  - `crates/slicer-runtime/tests/integration/manifest_default_reconcile_tdd.rs` — `assert_exhaustive_reconcile` and `ARACHNE_FALLBACKS` only
- Files allowed to edit (at most 3):
  - `modules/core-modules/arachne-perimeters/arachne-perimeters.toml`
  - `crates/slicer-runtime/tests/integration/manifest_default_reconcile_tdd.rs` (**two arachne rows only** — `CLASSIC_FALLBACKS` is packet 184's)
  - `modules/core-modules/arachne-perimeters/src/lib.rs` (new unit test only)
- Files explicitly out of bounds:
  - `crates/slicer-core/src/arachne/pipeline.rs` and `crates/slicer-sdk/src/host.rs` — Step 2a; do not re-touch
  - `modules/core-modules/classic-perimeters/**`, `classic-perimeters.toml`, and `CLASSIC_FALLBACKS`
  - `crates/slicer-runtime/tests/arachne_structural_invariants.rs` — Step 3 evaluates it; **never weaken `COVERAGE_THRESHOLD`**
  - `docs/**` — Steps 5a/5b
- Blast-radius discipline: manifest schema defaults change, so `cargo xtask gen-config-docs --check` will go red until Step 5a. That is expected and is Step 5a's budget; do **not** regenerate docs here.
- Expected sub-agent dispatches: none beyond Step 2a's (its `FACT` six-value return is what both halves consume).
- Context cost: `S`
- Authoritative docs: none read in this step.
- OrcaSlicer refs: none beyond Step 2a's.
- Verification:
  - `bash -c 'cargo test -p slicer-runtime --test integration -- manifest_default_reconcile_tdd::arachne_manifest_defaults_are_the_code_fallbacks --exact 2>&1 | rg "^test result:" | rg -q "^test result: ok\. [1-9]" && rg -q "\(\"wall_maximum_resolution\", Float\(0\.5\)\)" crates/slicer-runtime/tests/integration/manifest_default_reconcile_tdd.rs && rg -q "\(\"wall_maximum_deviation\", Float\(0\.025\)\)" crates/slicer-runtime/tests/integration/manifest_default_reconcile_tdd.rs && echo PASS || echo "FAIL: reconcile did not run/pass, or ARACHNE_FALLBACKS still pins the old 0.05/0.005 rows"'` — AC-7; **this is the guard the manifest move would break if the table were left behind, and it lands in this step, not at the ceremony.** The two trailing greps are what make this criterion change-proving: `ARACHNE_FALLBACKS` currently pins `Float(0.05)` / `Float(0.005)`, which *matches* today's manifest, so the reconcile test alone is already green on the unfixed tree (verified) and would report PASS before any edit. Measured: the full command as written returns FAIL on the unfixed tree.
  - `bash -c 'cargo test -p arachne-perimeters --lib -- tests::wall_simplify_fallbacks_are_canonical_defaults --exact 2>&1 | rg "^test result:" | rg -q "^test result: ok\. [1-9]" && echo PASS || echo "FAIL: the selected test did not run or did not pass"'` — AC-8; FACT pass/fail
  - `bash -c 'cargo test -p arachne-perimeters --lib -- tests::wall_maximum_resolution_wired --exact 2>&1 | rg "^test result:" | rg -q "^test result: ok\. [1-9]" && echo PASS || echo "FAIL: the selected test did not run or did not pass"'` — the pre-existing config-supplied-path test must remain green (it asserts `0.5² = 0.25` and `0.025² = 0.000625` from explicitly supplied keys and is therefore already written for the canonical values)
  - `cargo check --workspace --all-targets` — FACT pass/fail
  - `cargo xtask build-guests --check` — guest freshness after editing `crates/slicer-sdk/**` (Step 2a) and `modules/core-modules/arachne-perimeters/src/**` (this step); FACT `clean` / `STALE:` list
- Exit condition: AC-7 and AC-8 pass; `wall_maximum_resolution_wired` still passes; `cargo check --workspace --all-targets` is clean; `build-guests --check` reports clean. Behavioural fallout is explicitly **not** yet asserted — Step 3 owns it.

### Step 3: Absorb and re-validate D-168's geometry blast radius

- Task IDs: `TASK-304`
- Objective: prove that raising the wall-simplification segment gate from `0.05 mm` to `0.5 mm` at default config leaves every `ArachneParams::default()`-driven suite green, and in particular that the classic-vs-arachne coverage ratio still clears `0.99`.
- Precondition: Steps 2a **and** 2b are complete and `cargo check --workspace --all-targets` is clean. `crates/slicer-runtime/tests/arachne_structural_invariants.rs` currently passes with `COVERAGE_THRESHOLD = 0.99`, slicing every fixture with both `WallGenerator::Classic` and `WallGenerator::Arachne`.
- Postcondition: every binary named in the verification list reports `0 failed`, or each failure is traced to a specific geometric consequence of the tolerance change and resolved by correcting the **implementation**, never by loosening an assertion. Any residual movement that cannot be resolved is recorded as a new deviation and the packet does **not** close.
- Files allowed to read, with ranges when over 300 lines:
  - `target/test-output.log` — the tee'd run output; **grep it, never re-run a test to see more output** (`CLAUDE.md` §"Test output must always tee")
  - `crates/slicer-runtime/tests/arachne_structural_invariants.rs` — `COVERAGE_THRESHOLD`, `symmetric_coverage_ratio`, `coverage_predicate`, `measure_subject` only
  - `crates/slicer-runtime/tests/integration/perimeter_parity.rs` — the structural assertions only (wall-loop counts, width vs `flow_to_width(line_width_to_spacing(...))`, bead-count cap, finiteness)
  - `crates/slicer-core/src/arachne/simplify.rs` — only if a failure needs localising
- Files allowed to edit (at most 3):
  - None by default. This is a read-only validation step. If a failure is traced to a genuine implementation defect, edit only the file the defect lives in and record it in the step's exit note. **`crates/slicer-runtime/tests/arachne_structural_invariants.rs` is not editable** — `COVERAGE_THRESHOLD` must not move (`CLAUDE.md` §Test Discipline: never weaken assertions to get a pass).
- Files explicitly out of bounds:
  - `crates/slicer-runtime/tests/arachne_structural_invariants.rs` (edit); `crates/slicer-runtime/tests/fixtures/**` (read or edit — there are no arachne goldens to re-bless; packet 177 deleted them all)
  - `modules/core-modules/classic-perimeters/**`
  - `docs/**` — Steps 5a/5b
- Blast-radius discipline: not applicable (no struct field or schema constant added). This step **is** the blast-radius absorption for Steps 2a–2b.
- Expected sub-agent dispatches:
  - Question: run `cargo test -p slicer-core --features host-algos`, tee to `target/test-output.log`, and report per-binary pass/fail plus failing test names only; scope: `crates/slicer-core/tests/**`; return: `FACT pass/fail` + failing names, **never the log body**
  - Question: run `cargo test -p slicer-runtime --test arachne_structural_invariants` and report the printed coverage ratios per subject; scope: that binary; return: `FACT` (subject → ratio, ≤20 lines)
- Context cost: `S`
- Authoritative docs:
  - `docs/17_agent_debugging.md` — delegated SUMMARY, only if a failure needs DAG/timing localisation
  - `docs/19_visual_debug.md` — delegated SUMMARY, only if a coverage-ratio drop needs visual localisation
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/Arachne/WallToolPaths.cpp` — `simplifyToolPaths`; delegate, never load. Consult only if a failure suggests PnP's simplification differs structurally from canonical rather than only in tolerance.
- Verification:
  - `bash -c 'cargo test -p slicer-core --features host-algos 2>&1 | tee target/test-output.log | rg "^test result:" > target/guard-ac9.txt; rg -q "[1-9][0-9]* failed|^test result: FAILED" target/guard-ac9.txt && echo "FAIL: see target/test-output.log" || (rg -q "^test result: ok\. [1-9]" target/guard-ac9.txt && cargo test -p slicer-core --features host-algos --test arachne_pipeline 2>&1 | rg -q "^test result: ok\. [1-9]" && echo PASS || echo "FAIL: zero tests ran, or the host-algos-gated arachne_pipeline binary ran zero tests — host-algos likely not enabled; see target/test-output.log")'` — AC-9; **byte-identical to the AC-9 row in `packet.spec.md` §Acceptance Criteria and to `packet.spec.md` §Verification — keep all three in lockstep.** This is one of only two multi-result-line commands in the packet, hence the guard-file form. **`--features host-algos` is mandatory**; without it every `arachne_*` binary is `#![cfg(feature = "host-algos")]`-compiled to a no-op. **The non-emptiness conjunct alone does not enforce that** — measured on this tree with the feature dropped, the run is only *partially* vacuous: the `arachne_*` binaries report `ok. 0 passed` while unrelated `slicer-core` binaries still pass (dozens on each side; re-derive rather than trusting a frozen count), so `rg -q "^test result: ok\. [1-9]"` still succeeds. The third conjunct is what enforces the feature: it pins `arachne_pipeline`, gated at file scope by `#![cfg(feature = "host-algos")]` and measured at `0 passed` without the feature and a non-zero pass count with it. Do **not** substitute an "any `0 passed` ⇒ FAIL" conjunct — the legitimate green run contains an `ok. 0 passed` doc-test line
  - `bash -c 'cargo test -p slicer-runtime --test arachne_structural_invariants 2>&1 | tee target/test-output.log | rg "^test result:" | rg -q "^test result: ok\. [1-9]" && echo PASS || echo "FAIL: arachne_structural_invariants did not run clean — see target/test-output.log"'` — AC-10; one binary, one summary line, so the single-result-line form applies. Long-running; budget accordingly
  - `bash -c 'cargo test -p slicer-runtime --test integration -- perimeter_parity 2>&1 | rg "^test result:" | rg -q "^test result: ok\. [1-9]" && echo PASS || echo "FAIL: the perimeter_parity filter selected nothing, or a fixture regressed"'` — the 11 `perimeter_parity` fixture dirs. **Name-filtered, so filtered-to-zero is a live failure mode**: a bare `rg -v "0 failed"` guard reports PASS when the filter matches nothing (measured with a deliberately misspelled filter), which would silently retire all 11 fixtures
  - `bash -c 'cargo test -p slicer-runtime --test executor -- arachne_ 2>&1 | rg "^test result:" | rg -q "^test result: ok\. [1-9]" && echo PASS || echo "FAIL: the arachne_ filter selected nothing, or an executor test regressed"'` — AC-14 plus `arachne_negative_spacing_fatal`. Name-filtered, so the same filtered-to-zero hazard applies
  - `bash -c 'cargo test -p slicer-runtime --test arachne_parity_round2 2>&1 | rg "^test result:" | rg -q "^test result: ok\. [1-9]" && echo PASS || echo "FAIL: arachne_parity_round2 did not run clean"'` — expected unaffected (it passes its own `1e-3, 1.0, f64::INFINITY` params); this command proves it
- Exit condition: all five commands report PASS, and the recorded coverage ratios are captured in the step note so packet 184 can compare against them when it moves the classic side.

### Step 4: Add the per-vertex thick-bridge width exemption to `build_walls` (D-163)

- Task IDs: `TASK-304`
- Objective: at bridge vertices of Outer/Inner walls, with `thick_bridges` enabled, substitute canonical's bridge-thread width `dmr = nozzle_diameter × √bridge_flow_ratio` instead of applying the spacing→width back-conversion — matching canonical `VariableWidth.cpp::thick_polyline_to_multi_path`'s `role == erOverhangPerimeter && flow.bridge()` exemption at PnP's per-vertex granularity.
- Precondition: in `build_walls`, `ring_pts_units` is constructed **after** the `for pt in &mut path.points` conversion loop, and the `feature_flags[i].is_bridge` marking block runs after both. `bridge_areas` (from `region.bridge_areas()`) is already in scope above the conversion loop. The conversion loop applies `flow_to_width(pt.width, layer_height_mm)` to every vertex unconditionally. `thick_bridges` defaults to `false` (manifest and `get_bool(...).unwrap_or(false)`), so this change is inert at default config.
- Postcondition: `ring_pts_units` is hoisted above the conversion loop (provably inert — it reads only `p.x`/`p.y`, which the loop never writes); a `bridge_vertex: Vec<bool>` is computed once from `point_in_any_polygon` gated on `!bridge_areas.is_empty() && matches!(loop_type, LoopType::Outer | LoopType::Inner)` and reused by both the conversion loop and the marking block; the conversion loop sets `pt.width = nozzle_diameter_mm * bridge_flow_ratio.sqrt()` when `thick_bridges && bridge_vertex[i]` and `flow_to_width(pt.width, layer_height_mm)` otherwise; `let widths: Vec<f32>` still snapshots **after** the loop; `build_wall_flags` still receives `Some(&ring_pts_units)`; the existing long attribution comment is updated to state that the seam is an analogy by formula (`unscale(w) + height × (1 - π/4)` is byte-identical in `thick_polyline_to_multi_path`, `thick_polyline_to_extrusion_paths_2` and Arachne's `extrusion_paths_append` path) rather than a call-graph match, and to record that the exemption is `thick_bridges`-gated because `flow.bridge()` is true only for a `Flow::bridging_flow`-produced flow.
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/arachne-perimeters/src/lib.rs` — long; ranged: `fn build_walls` from its signature through the `overhang_quartile` block, plus the `bridge_flow` / `thick_bridges` config reads above it
  - `crates/slicer-core/src/flow.rs` — `bridging_flow` and `flow_to_width` only
  - `crates/slicer-ir/src/slice_ir.rs` — `WallFeatureFlags` and `ExtrusionRole` only. `ExtrusionRole` is `#[non_exhaustive]` and is `Clone`, not `Copy` (because of `Custom(String)`); `role` is **moved** into `extrusion_line_to_extrusion_path3d(line, role)` but stays readable as `path.role`, so reading it inside `for pt in &mut path.points` needs a snapshot taken before the loop
  - `modules/core-modules/arachne-perimeters/tests/bridge_flow_factor_tdd.rs` — whole file (short); note its config keys are `bridge_flow` and `thick_bridges`
- Files allowed to edit (at most 3):
  - `modules/core-modules/arachne-perimeters/src/lib.rs`
  - `modules/core-modules/arachne-perimeters/tests/bridge_flow_factor_tdd.rs`
- Files explicitly out of bounds:
  - `crates/slicer-runtime/tests/arachne_parity.rs` — `arachne_parity_arachne_path_is_bridge_flag_set_per_vertex` must keep passing **unchanged**; it pins the per-vertex `is_bridge` contract this design depends on
  - `crates/slicer-runtime/tests/executor/arachne_perimeters_simple_square.rs` — must keep passing unchanged (AC-14); its fixture sets `bridge_areas: vec![]`, so the exemption cannot fire there
  - `crates/slicer-core/src/flow.rs` — `bridging_flow`'s signature and semantics are unchanged; it returns a **flow factor**, not a width
  - `modules/core-modules/arachne-perimeters/tests/` siblings (`arachne_parity_is_bridge_flag_tdd`, `arachne_parity_overhang_quartile_tdd`, `precise_outer_wall_tdd`, and the rest) — must keep passing unchanged
  - `docs/**` — Steps 5a/5b
- Blast-radius discipline: no struct field or schema constant is added. `WallFeatureFlags` gains no field; `Point3WithWidth` gains no field. The behavioural radius is bounded by `thick_bridges == true`, which no default-config fixture sets — verified: the manifest default is `false` and the code fallback is `get_bool("thick_bridges").unwrap_or(false)`.
- Expected sub-agent dispatches:
  - Question: confirm `LayerRegion::bridging_flow`'s `thick_bridge == false` branch does not produce a bridge-flagged `Flow`, and that `Flow::bridging_flow(dmr, nozzle)` yields `width() == dmr` and `mm3_per_mm() == π·dmr²/4`; scope: `OrcaSlicerDocumented/src/libslic3r/LayerRegion.cpp`, `.../Flow.hpp`, `.../Flow.cpp`; return: `SUMMARY` ≤200 words, ≤30 lines of C++
  - Question: list every reader of `WallLoop.feature_flags[i].is_bridge` and of `Point3WithWidth.width` downstream of `run_perimeters`, so a width-domain change at bridge vertices cannot surprise a consumer; scope: `crates/**`, `modules/**`; return: `LOCATIONS` ≤20 entries
- Context cost: `M`
- Authoritative docs:
  - `docs/08_coordinate_system.md` — delegated SUMMARY; confirms `dmr` is a plain-mm scalar and must not be scaled, while `ring_pts_units` must be
  - `docs/02_ir_schemas.md` — delegated SUMMARY of `WallLoop` / `Point3WithWidth` / `WallFeatureFlags` only, to confirm no schema field is implied
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/VariableWidth.cpp` — `thick_polyline_to_multi_path`; delegate, never load
  - `OrcaSlicerDocumented/src/libslic3r/Flow.hpp`, `.../Flow.cpp` — `Flow::bridging_flow`, `Flow::with_width`; delegate, never load
  - `OrcaSlicerDocumented/src/libslic3r/LayerRegion.cpp` — `LayerRegion::bridging_flow`; delegate, never load
- Verification:
  - `bash -c 'python3 -c "import sys; s=open(r\"modules/core-modules/arachne-perimeters/src/lib.rs\",encoding=\"utf-8\").read(); b=s.index(\"fn build_walls\"); seg=s[b:b+14000]; r=seg.index(\"ring_pts_units\"); c=seg.index(\"flow_to_width(\"); print(\"PASS\" if r<c and \"thick_bridges &&\" in seg[r:c+200] else \"FAIL: ring_pts_units not hoisted above flow_to_width, or the exemption is not gated by a thick_bridges && ... test between the hoist and the back-conversion\")"'` — AC-11; FACT PASS/FAIL
  - `bash -c 'cargo test -p arachne-perimeters --test bridge_flow_factor_tdd 2>&1 | rg "^test result:" | rg -q "^test result: ok\. [1-9]" && echo PASS || echo "FAIL: bridge_flow_factor_tdd did not run clean"'` — AC-12, AC-13, AC-N2. **The pre-existing `bridge_vertices_get_round_section_factor_when_thick_bridges_on` derives its expectation from `pt.width` itself and is therefore insensitive to this change — it cannot serve as the proof.** AC-12's new test must assert `pt.width == 0.4 × √0.7 ≈ 0.334664` within `1e-4` **and** that `width × 0.2 × flow_factor ≈ 0.0879646` within `1e-5`. **`bridge_vertices_get_bridge_flow_ratio_when_thin` must also be strengthened in this step (AC-13, F8).** As it stands on the tree it exists, is green, and asserts only `pt.flow_factor` — it never reads `pt.width`, so it cannot detect the exemption leaking into the non-thick path and cannot carry AC-13's "keeps the ordinary back-converted width" clause. Add to its `flag.is_bridge` branch: `pt.width` equals the ordinary `flow_to_width(spacing, layer_height_mm)` back-conversion — equivalently, equal within `1e-4` to the `pt.width` of the loop's **non-bridge** vertices, which take that conversion by construction — and is **not** `nozzle_diameter_mm * (0.7f32).sqrt()`. Extend the existing `found_bridge_vertex` anti-vacuity guard to cover the new width branch.
  - `bash -c 'cargo test -p slicer-runtime --test executor -- arachne_perimeters_simple_square 2>&1 | rg "^test result:" | rg -q "^test result: ok\. [1-9]" && echo PASS || echo "FAIL: the selected test did not run or did not pass"'` — AC-14; the D-160 Bug B guard (a leaked `0.357 mm` on a `0.4 mm` wall) must survive
  - `bash -c 'cargo test -p slicer-runtime --test arachne_parity 2>&1 | rg "^test result:" | rg -q "^test result: ok\. [1-9]" && echo PASS || echo "FAIL: arachne_parity did not run clean"'` — `arachne_parity_arachne_path_is_bridge_flag_set_per_vertex` unchanged
  - `bash -c 'cargo test -p arachne-perimeters --tests 2>&1 | rg "^test result:" > target/guard-s4-ap-tests.txt; rg -q "[1-9][0-9]* failed|^test result: FAILED" target/guard-s4-ap-tests.txt && echo "FAIL: a sibling TDD binary regressed" || (rg -q "^test result: ok\. [1-9]" target/guard-s4-ap-tests.txt && echo PASS || echo "FAIL: zero tests ran")'` — the eight sibling per-file TDD binaries in the same dir. `--tests` selects **many** binaries, so this is the packet's second multi-result-line command and takes the guard-file form: the first conjunct catches one red among greens (a single-line `rg -q` would let a green sibling mask it), the second catches an all-zero sweep
  - `cargo clippy --workspace --all-targets -- -D warnings` — FACT pass/fail
  - `cargo xtask build-guests --check` — guest freshness after editing `modules/core-modules/arachne-perimeters/src/**`; FACT `clean` / `STALE:` list
- Exit condition: AC-11, AC-12, AC-13, AC-14 and AC-N2 pass; every sibling test in `modules/core-modules/arachne-perimeters/tests/` and `arachne_parity` still passes; clippy is clean; `build-guests --check` reports clean.

### Step 5a: Regenerate config docs and close the deviation ledger (D-163 / D-164 / D-168 + two new rows)

**Step 5 is split into 5a and 5b so that each half honours the three-file edit cap** — the single step edited four files (`docs/15_*`, `docs/DEVIATION_LOG.md`, `docs/adr/0043-*`, `docs/07_*`), which the cap does not allow. The split is by **file ownership, not by topic**: 5a owns *every* edit to `docs/15_config_keys_reference.md` and `docs/DEVIATION_LOG.md` (two files); 5b owns `docs/adr/0043-*` and `docs/07_implementation_status.md` (two files). No file is edited by both halves. They must run adjacent and in order, and neither is independently shippable. **`D-185-ADR-0043-AMENDED` is therefore filed in 5a**, one step ahead of the ADR edit it records — that is deliberate and safe, because the row quotes the *contested* clause, which is on the tree at 5a time; 5b's exit condition re-verifies the row alongside the amended ADR text. There are no Steps 6+, so nothing downstream is renumbered; other artifacts that said "Step 5" now say "Step 5a" or "Step 5b" according to which file they name.

- Task IDs: `TASK-304`
- Objective: bring `docs/15_config_keys_reference.md` back in sync with the changed manifest schema, flip D-163 / D-164 / D-168 to `Closed` with D-168's provenance corrected, file **two** new `DEV-###` rows — one for the two D-164 residuals (the shared row this packet owns on behalf of packet 184 as well; see `packet.spec.md` §Doc Impact) and one for the `maximum_extrusion_area_deviation` value divergence Step 2a leaves unfixed — each ID re-derived independently at the moment of writing, and file `D-185-ADR-0043-AMENDED` for the ADR amendment 5b performs.
- Precondition: Steps 1–4 are complete and their ACs pass. `cargo xtask gen-config-docs --check` currently fails (the manifest changed under it). `docs/15_config_keys_reference.md` contains `wall_maximum_resolution` at `0.05` in exactly **two** places — once inside the `<!-- BEGIN GENERATED: module-config-keys -->` block and once in the hand-written "Six keys registered on `arachne-perimeters`" table (probed against the tree during authoring; re-derive the count before asserting on it). `docs/ORCA_CONFIG_REFERENCE.md` has `outer_wall_line_width` / `inner_wall_line_width` rows marked `In Codebase: ❌` and no rows at all for the two tolerance keys.
- Postcondition: `cargo xtask gen-config-docs --check` exits 0; both `wall_maximum_resolution` occurrences read `0.5` and both `wall_maximum_deviation` occurrences read `0.025`; the `inner_wall_line_width` / `outer_wall_line_width` prose paragraphs record the `float_or_percent` retype, the `0 → 1.125 × nozzle_diameter` auto rule and the ingestion residual; **the hand-written `wall_maximum_resolution` / `wall_maximum_deviation` prose paragraph — the one currently reading "PnP manifest defaults state the CODE fallbacks `0.05` mm / `0.005` mm per the reconcile guard — the code-vs-upstream default divergence is logged as `D-168-ARACHNE-SIMPLIFY-FALLBACKS-TIGHTER-THAN-CANONICAL`" — is rewritten to state that the manifest defaults now match canonical and that D-168 is closed, so the phrase `state the CODE fallbacks` no longer appears in the file** (this is prose, not a table row: the `| float |` row clauses cannot see it, which is why AC-15 carries a dedicated `! rg -q "state the CODE fallbacks"` clause). **That paragraph carries OrcaSlicer line pins (`PrintConfig.cpp:<range>` for the key registration and `WallToolPaths.cpp:<ranges>` for the wall-path replacement); the rewritten sentence must drop them and cite `PrintConfigDef::init_fff_params` and `WallToolPaths::simplifyToolPaths` by function name only** — `CLAUDE.md` §"OrcaSlicer Citation Style" requires dropping line numbers on any citation you touch, and this rewrite touches both. Leave the surrounding paragraphs' legacy pins alone; do not mass-rewrite. Continuing the postcondition: the three deviation rows read `Closed`; D-168's row additionally states that its original "transcribed the `meshfix_*` constants" cause was wrong (canonical `meshfix_maximum_resolution()` / `meshfix_maximum_deviation()` return `scaled<coord_t>(0.5)` / `scaled<coord_t>(0.025)`, identical to the wall defaults); D-163's row records the two accepted residuals (no per-vertex height analog for canonical's `height() == dmr`; no `bridge_width` key); the shared D-164 residual row exists — worded to cover the arachne **and** classic halves, since packet 184 cross-references it instead of filing a duplicate, and there is no new ADR for that divergence — covering the D-164 residuals — **worded about the parser** (`parse_percent_default`'s discarded return means no live-path code constructs a `Percent`/`FloatOrPercent`), explicitly **not** about `ResolvedConfig::to_config_map`, which merges `self.extensions` verbatim and already transports any variant; a `D-185-ADR-0043-AMENDED` row exists in `docs/DEVIATION_LOG.md` quoting the contested clause "plain mm floats, default 0.4, range [0.1, 2.0]" (convention: `D-161-ADR-0037-AMENDED`, `D-283-ADR-0046-AMENDED` — the two committed precedents; verify both are present before citing) and naming Step 5b as the step that performs the amendment; `docs/ORCA_CONFIG_REFERENCE.md` is untouched.
- Files allowed to read, with ranges when over 300 lines:
  - `docs/15_config_keys_reference.md` — >600 lines; ranged: the `BEGIN GENERATED: module-config-keys` / `END GENERATED` marker lines, the hand-written six-key arachne table, and the `inner_wall_line_width` / `min_width_top_surface` prose paragraphs. **Never hand-edit inside the markers.**
  - `docs/DEVIATION_LOG.md` — large; delegate. Fetch the D-163 / D-164 / D-168 rows and the highest `DEV-###` only
  - `.ralph/specs/185-arachne-width-bridge-parity/design.md` — §Code Change Surface, §Data and Contract Notes, and §ADR Alignment, for the exact wording of the provenance correction and of the ADR amendment
  - `docs/adr/0043-derive-arachne-bead-widths-from-wall-flows.md` — short; **read-only in this step** (the edit is 5b's). Read §Decision item 2 only, to quote the contested clause verbatim in the `D-185-ADR-0043-AMENDED` row.
- Files allowed to edit (doc-only step; two files, each a single-purpose edit):
  - `docs/15_config_keys_reference.md`
  - `docs/DEVIATION_LOG.md` — **this step owns every deviation-log edit in the packet**: the three closures, the new `DEV-###` residual row, and `D-185-ADR-0043-AMENDED`
- Files explicitly out of bounds:
  - `docs/adr/0043-derive-arachne-bead-widths-from-wall-flows.md` and `docs/07_implementation_status.md` — **Step 5b**
  - `docs/ORCA_CONFIG_REFERENCE.md` — deliberately untouched; see `packet.spec.md` §Doc Impact Statement for why the `❌` marks stay and why no tolerance-key rows are added
  - `docs/specs/deviation-backlog-remediation-plan.md` — the orchestrator maintains the Packet Queue; **do not edit**
  - Any ADR other than `0043` — none is contradicted (see `design.md` §ADR Alignment)
  - The interior of `docs/15_config_keys_reference.md`'s generated markers — regenerate, never hand-edit
  - All code and test files — Steps 1–4 own them
- Blast-radius discipline: not applicable (no struct field or schema constant added). The one ledger hazard is the new `DEV-###`: it **must** be derived inside this step, at the moment of writing.
- Expected sub-agent dispatches:
  - Question: what is the current highest `DEV-###` in `docs/DEVIATION_LOG.md`? Run `rg -o '^\| DEV-[0-9]{3}' docs/DEVIATION_LOG.md | sort -u | tail -1`; scope: `docs/DEVIATION_LOG.md`; return: `FACT` (one ID). **Take the next free number from this result.** Do not reuse any ID quoted in a spec artifact — packet 184 files a same-shaped row and whichever lands second must re-derive its own (`CLAUDE.md` §"Ledger Facts Must Be Re-derived")
  - Question: return the D-163 / D-164 / D-168 rows verbatim with their line numbers; scope: `docs/DEVIATION_LOG.md`; return: `SNIPPETS` (3 rows)
- Context cost: `S`
- Authoritative docs:
  - `docs/15_config_keys_reference.md` — ranged read as specified above; edited in this step
  - `docs/DEVIATION_LOG.md` — delegated row fetch; edited in this step
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/Arachne/WallToolPaths.hpp` — `meshfix_maximum_resolution`, `meshfix_maximum_deviation`; delegate, never load. Needed only to restate the provenance correction accurately in the D-168 row. **Cite by function name; never by line number.**
- Verification:
  - `bash -c 'cargo xtask gen-config-docs --check >/dev/null 2>&1 && echo PASS || echo "FAIL: doc 15 generated block is stale"'` — FACT PASS/FAIL
  - `bash -c 'python3 -c "import io; s=io.open(r\"docs/15_config_keys_reference.md\",encoding=\"utf-8\").read(); b=chr(96); p=chr(124); row=lambda v: b+\"wall_maximum_resolution\"+b+\" \"+p+\" float \"+p+\" \"+b+v+b; print(\"PASS\" if s.count(row(\"0.5\"))==2 and s.count(row(\"0.05\"))==0 and \"state the CODE fallbacks\" not in s else \"FAIL\")"'` — AC-15's content half; **byte-identical to AC-15's command in `packet.spec.md` minus its leading `cargo xtask gen-config-docs --check` conjunct, which is the separate FACT command above — keep them in lockstep.** It is written in `python3` rather than `rg` on purpose: the row being asserted contains both backticks and pipes, and an `rg "..." `-with-backticks form is **not executable** — bash performs command substitution on the backticks inside the double-quoted pattern. Re-measured by execution on this tree: the `rg` form errors with `<key>: command not found` (`wall_maximum_resolution: command not found`, then `0.5: command not found`), the failed substitutions collapse the pattern to a bare `\| float \|` literal that matches every float row in the file, and it therefore returns a **non-discriminating** result — measured **PASS** on the unfixed tree, where a working guard must return FAIL. It cannot discriminate in either direction, before or after the fix. Building the row from `chr(96)`/`chr(124)` avoids both hazards and survives markdown table-cell escaping. The `^2$` count covers the generated row **and** the hand-written row, so a partial update fails; the `! rg -q "state the CODE fallbacks"` clause covers the tolerance-key **prose paragraph**, which is invisible to both row-shaped clauses. Measured: that phrase is present on the tree today, so this command returns FAIL on the unfixed tree and only the paragraph rewrite clears it
  - `bash -c 'rg -q "float_or_percent" docs/15_config_keys_reference.md && rg -q "1\.125" docs/15_config_keys_reference.md && echo PASS || echo FAIL'` — the wall-width prose-paragraph doc impact
  - `bash -c 'python3 -c "import re; L=open(r\"docs/DEVIATION_LOG.md\",encoding=\"utf-8\").read().splitlines(); rows={i:l for i,l in enumerate(L) if re.match(r\"\\|\\s*D-16[348]-(ARACHNE|WALL)\",l)}; bad=[l.split(\"|\")[1].strip() for l in rows.values() if \"Closed\" not in l]; d168=[l for l in rows.values() if \"D-168\" in l]; prov=d168 and \"meshfix\" in d168[0] and \"0.5\" in d168[0]; print(\"FAIL: not closed \"+str(bad) if bad else (\"PASS\" if prov else \"FAIL: D-168 provenance correction missing\"))"'` — AC-16; FACT PASS/FAIL
  - `bash -c 'rg -q "parse_percent_default" docs/DEVIATION_LOG.md && rg -q "1\.125" docs/DEVIATION_LOG.md && echo PASS || echo FAIL'` — the new residual row exists and names both residuals
  - `bash -c 'rg -q "D-185-ADR-0043-AMENDED" docs/DEVIATION_LOG.md && echo PASS || echo FAIL'` — the ADR amendment's deviation row is filed here; **its ADR-text half is 5b's** (see 5b's verification for the full conjunction). Measured: FAIL on the unfixed tree
  - `bash -c 'git diff --name-only HEAD -- docs/ORCA_CONFIG_REFERENCE.md docs/specs/deviation-backlog-remediation-plan.md docs/adr/0043-derive-arachne-bead-widths-from-wall-flows.md docs/07_implementation_status.md | sort > target/guard-untouched-5a-after.txt; diff -q target/guard-untouched-5a-before.txt target/guard-untouched-5a-after.txt >/dev/null && echo PASS || echo "FAIL: unexpected edit to a file this step does not own"'` — the two deliberately-untouched docs, plus 5b's two files, which this step must not touch. Two things about this form, both load-bearing. **(a) The `HEAD` ref is mandatory**: a bare `git diff --name-only --` compares the worktree against the *index*, so a staged or committed edit yields a false PASS. **(b) It is a before/after comparison, not an emptiness test.** `docs/specs/deviation-backlog-remediation-plan.md` is orchestrator-owned and may already be dirty relative to `HEAD` when this packet activates — measured dirty on the tree at authoring time, which would make a plain `rg -q .` emptiness test report FAIL before the step does anything. Capture the baseline as the **first action of the step**, before any edit, with `bash -c 'git diff --name-only HEAD -- docs/ORCA_CONFIG_REFERENCE.md docs/specs/deviation-backlog-remediation-plan.md docs/adr/0043-derive-arachne-bead-widths-from-wall-flows.md docs/07_implementation_status.md | sort > target/guard-untouched-5a-before.txt'`. The `-5a-` / `-5b-` key suffixes keep the two steps' captures from overwriting each other. Do **not** substitute a frozen SHA for the baseline (`CLAUDE.md` §"Ledger Facts Must Be Re-derived")
- Exit condition: AC-15 and AC-16 pass, the new `DEV-###` row exists with a freshly derived ID, the `D-185-ADR-0043-AMENDED` row exists, the tolerance-key prose paragraph no longer contains `state the CODE fallbacks`, and `docs/ORCA_CONFIG_REFERENCE.md`, `docs/specs/deviation-backlog-remediation-plan.md` and 5b's two files are provably unmodified relative to `HEAD`.

### Step 5b: Amend ADR-0043 §Decision item 2 and register `TASK-304`

- Task IDs: `TASK-304`
- Objective: bring `docs/adr/0043-derive-arachne-bead-widths-from-wall-flows.md` §Decision item 2 into agreement with the AC-1 retype, and register `TASK-304` in `docs/07_implementation_status.md`.
- Precondition: Step 5a is complete; `D-185-ADR-0043-AMENDED` is already filed in `docs/DEVIATION_LOG.md` (5a owns that file). `docs/adr/0043-*.md` §Decision item 2 still reads "plain mm floats, default 0.4, range [0.1, 2.0], group Walls". `TASK-304` has zero hits in `docs/07_implementation_status.md` (a ledger fact — re-derive it, and the file's current maximum `TASK-###`, at the moment of writing).
- Postcondition: §Decision item 2 no longer reads "plain mm floats, default 0.4, range [0.1, 2.0]" and instead specifies `float_or_percent`, default `0.4`, range `[0.0, 2.0]` with `0` as canonical's auto sentinel, citing that ADR-0043's own §Consequences anticipated the change; `TASK-304` is registered in `docs/07_implementation_status.md` as a hand-added row **outside** the generated block, followed by `cargo xtask check-deviations` to regenerate the open-deviations block.
- Files allowed to read, with ranges when over 300 lines:
  - `docs/adr/0043-derive-arachne-bead-widths-from-wall-flows.md` — short; read §Decision item 2 and §Consequences in full. Item 2's type/range clause is what this packet contradicts; §Consequences is what authorizes the amendment.
  - `.ralph/specs/185-arachne-width-bridge-parity/design.md` — §ADR Alignment only, for the exact wording of the amendment
  - `docs/07_implementation_status.md` — **never read whole**; delegate the row insertion to a worker and have it return the current maximum `TASK-###` and the generated-block marker lines only
- Files allowed to edit (doc-only step; two files, each a single-purpose edit):
  - `docs/adr/0043-derive-arachne-bead-widths-from-wall-flows.md` — **§Decision item 2's type/range clause only.** Items 1 and 3 and §Consequences are not edited. This is the packet's only edit outside `docs/15_*`, `docs/DEVIATION_LOG.md` and `docs/07_*`.
  - `docs/07_implementation_status.md` — the hand-added `TASK-304` backlog row **outside** the generated block; then run `cargo xtask check-deviations` to regenerate the open-deviations block. Never hand-edit inside the generated markers, and never read the whole backlog — dispatch the row insertion to a worker.
- Files explicitly out of bounds:
  - `docs/15_config_keys_reference.md` and `docs/DEVIATION_LOG.md` — **Step 5a**; this step must not touch either
  - `docs/ORCA_CONFIG_REFERENCE.md`, `docs/specs/deviation-backlog-remediation-plan.md` — untouched, as in 5a
  - Any ADR other than `0043`
  - The generated block of `docs/07_implementation_status.md` — regenerate with `cargo xtask check-deviations`, never hand-edit
  - All code and test files — Steps 1–4 own them
- Blast-radius discipline: not applicable (doc-only, no schema or struct surface).
- Expected sub-agent dispatches:
  - Question: insert the `TASK-304` backlog row outside the generated block of `docs/07_implementation_status.md` and return the file's current maximum `TASK-###`; scope: `docs/07_implementation_status.md`; return: `FACT` (one ID). **Never read the whole backlog into the parent context.**
- Context cost: `S`
- Authoritative docs:
  - `docs/adr/0043-derive-arachne-bead-widths-from-wall-flows.md` — read in full (short); edited in this step
  - `docs/07_implementation_status.md` — delegated row insert; edited in this step
- OrcaSlicer refs: none — the amendment restates PnP's own schema decision, not canonical behaviour.
- Verification:
  - `bash -c 'rg -q "D-185-ADR-0043-AMENDED" docs/DEVIATION_LOG.md && rg -q "float_or_percent" docs/adr/0043-derive-arachne-bead-widths-from-wall-flows.md && ! rg -q "plain mm floats, default 0\.4, range \[0\.1, 2\.0\]" docs/adr/0043-derive-arachne-bead-widths-from-wall-flows.md && echo PASS || echo FAIL'` — the ADR-0043 amendment plus its 5a-filed deviation row, verified together. Measured: returns FAIL on the unfixed tree, and the negative clause matches the ADR's current single-line wrapping of "(plain mm floats, default 0.4, range [0.1, 2.0]," so it genuinely flips only when item 2 is rewritten
  - `bash -c 'rg -q "TASK-304" docs/07_implementation_status.md && echo PASS || echo FAIL'` — `TASK-304` registered. Measured: FAIL on the unfixed tree (zero hits today; the file's current maximum is `TASK-302` — a ledger fact, re-derive rather than trusting that number)
  - `bash -c 'git diff --name-only HEAD -- docs/ORCA_CONFIG_REFERENCE.md docs/specs/deviation-backlog-remediation-plan.md docs/15_config_keys_reference.md docs/DEVIATION_LOG.md | sort > target/guard-untouched-5b-after.txt; diff -q target/guard-untouched-5b-before.txt target/guard-untouched-5b-after.txt >/dev/null && echo PASS || echo "FAIL: unexpected edit to a file this step does not own"'` — the two deliberately-untouched docs plus 5a's two files, which this step must not touch. Capture the baseline as the **first action of the step** with the same command writing `target/guard-untouched-5b-before.txt`. Both the `HEAD` ref and the before/after shape are mandatory for the reasons given in 5a — and 5a's own edits to `docs/15_*` / `docs/DEVIATION_LOG.md` are exactly why an emptiness test cannot be used here
- Exit condition: all three commands report PASS; `docs/15_config_keys_reference.md` and `docs/DEVIATION_LOG.md` carry no edit from this step.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | S | Two manifest blocks + two read sites + three unit tests; ranged reads only |
| Step 2a | S | Two files (`pipeline.rs`, `host.rs`), each edit 1–4 lines; the cost is the delegated canonical confirmation and the provenance rewrite |
| Step 2b | S | Three files (manifest, `ARACHNE_FALLBACKS`, one new unit test), each edit 1–4 lines; consumes 2a's delegated `FACT` and dispatches nothing new |
| Step 3 | S | Read-only validation; the whole budget is dispatched runs returning FACT pass/fail |
| Step 4 | M | The only genuinely new logic in the packet; needs the `build_walls` body plus two delegated canonical confirmations |
| Step 5a | S | Two doc files (`docs/15_*`, `docs/DEVIATION_LOG.md`), ranged; one delegated `DEV-###` re-derivation |
| Step 5b | S | Two doc files (`docs/adr/0043-*`, `docs/07_*`), ranged; one delegated backlog-row insert |

Aggregate: `M`. No step is `L`. **Every step is within the three-file edit cap**: Steps 1, 2a, 4, 5a and 5b edit two files each; Step 2b edits three; Step 3 is read-only. Steps 2 and 5 were each split precisely to keep that true. Further splitting before activation is not required.

## Packet Completion Gate

- All steps and exits complete.
- Every pipe-suffixed AC command returns PASS (AC-1 through AC-16, AC-N1 through AC-N3).
- `cargo xtask build-guests --check` reports clean after the final code-editing step.
- `TASK-304` is registered in `docs/07_implementation_status.md` (Step 5b owns the edit; verify with `rg -q 'TASK-304' docs/07_implementation_status.md`). Any further backlog update goes through a worker dispatch, never a full backlog read.
- `docs/adr/0043-derive-arachne-bead-widths-from-wall-flows.md` §Decision item 2 is amended and `D-185-ADR-0043-AMENDED` is filed (the ADR edit in Step 5b, the row in Step 5a).
- Reconcile reopened/superseded status transitions: none — this packet reopens and supersedes nothing.
- Hand packet 184 the coverage ratios captured in Step 3 so it can compare when it moves the classic side of `arachne_structural_invariants.rs`.
- `packet.spec.md` is ready for `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed AC and packet-level gate command.
- Run the whole-suite closure gate through the enforced entry point — `cargo xtask test --summary --workspace` — never a bare `cargo test --workspace`, and dispatch it to a sub-agent returning `FACT pass/fail` only. This is the one broad run this packet is permitted, and it is permitted only here.
- Record remaining packet-local risk: the two D-164 residuals (absent-key `0.4` vs canonical auto, held on **fixture-stability** grounds — packet 182 is `draft` and has not landed, so it is not the load-bearing reason; and **no live-path producer of a `Percent`/`FloatOrPercent` exists**, because `parse_percent_default`'s return is discarded — *not* an inability of `to_config_map`, which merges `extensions` verbatim) and the two D-163 residuals (no per-vertex height analog for canonical's `height() == dmr`; no `bridge_width` key), all four carried in the deviation log rather than the code.
- Confirm context stayed at or below 150k standard, or at/below 300k only with a logged swarm ESCALATION; otherwise record a packet-authoring lesson.

All `cargo check`, `cargo clippy`, and `cargo test` invocations in gate and verification commands must use `--all-targets` so the test, bench, and example targets compile.
