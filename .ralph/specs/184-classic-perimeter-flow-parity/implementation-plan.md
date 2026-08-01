# Implementation Plan: 184-classic-perimeter-flow-parity

## Execution Rules

- Work one atomic step at a time; map every step to grouped task IDs.
- Use TDD, then implementation, then the narrowest falsifying validation.
- Every field below is a context-budget contract and must be filled independently; never write "see Step 1".
- **Step order is fixed and is a data dependency.** D-164 (Steps 1-2) → D-105 (Steps 3-5) → D-152 (Steps 6-7) → docs and closure (Step 8). D-164 defines the resolved wall widths that D-105's `line_width_to_spacing` consumes; running D-105 first re-blesses the golden twice against meaningless intermediate numbers.
- **The GREEN ok-guard.** Every GREEN gate in this plan ends in `| rg "^test result" | rg -q "^test result: ok\. [1-9]" || echo "FAIL: …"` — the single positive-match form defined once in `packet.spec.md` §Acceptance Criteria. The older two-stage `rg "^test result" | rg -v "0 passed"` form is **banned throughout this packet**: `test result: FAILED. 1 passed; 1 failed;` contains no literal `0 passed`, so `rg -v` exits `0`, the `|| echo FAIL` never fires, and a partially red suite reports success. The `[1-9]` clause keeps the original "did any test actually run" protection by also rejecting `ok. 0 passed; 0 failed; N filtered out`. Verified against real `cargo test` output in all three states. Valid only for a single-`--test`-binary run — with two binaries, `rg -q` would match the passing one and mask the failing one.
- **RED vs GREEN guard rule.** Neither guard form is correct for a **RED** step. A genuine failure prints `test result: FAILED. 0 passed; 1 failed; …`; the ok-guard rejects it (correctly, but uselessly — a RED step *wants* that), and cannot distinguish "failed for the right reason" from "matched no tests". For RED steps, print the result line unfiltered and require it to contain a non-zero `failed` count **and** a panic message naming the intended assertion.
- **Derived-expectation rule.** Every spacing expectation in a test must be **computed by calling `slicer_core::flow::line_width_to_spacing`**, never transcribed as a decimal. `crates/slicer-runtime/tests/integration/perimeter_parity.rs` already does `use slicer_core::flow::{flow_to_width, line_width_to_spacing};` inside the `integration` binary, so the import is proven available there. Transcribing decimals is the exact failure mode this packet repairs.
- **Manifest assertion rule.** Use the windowed form `rg -U '\[config\.schema\.<key>\][^\[]*<assertion>'`. The `[^\[]*` cannot cross into the next section because `[` terminates it — so any `description` string this packet adds must contain **no `[` character**. Always quote the closing `"` in a type assertion: an unquoted `"float` would match `"float_or_percent"`.
- **Test-filter rule.** `--test integration` and `--test e2e` are aggregated mod-list binaries, so libtest names are module-qualified; use substring filters, never `--exact` on a bare fn name. `--test arachne_parity` and `--test arachne_structural_invariants` are standalone binaries. **Qualify every filter with its module path (`<module>::<fn>`) whenever a bare name would also select a sibling test** — a bare module name is a prefix of every test in that file, so it selects all of them (verified in the `integration` binary: `precise_outer_wall_tdd` selects the three tests AC-6 names; `precise_outer_wall_tdd::precise_mode_off_standard_spacing` selects one). AC-5 (`outer_inner_width_and_spacing_tdd::outer_inner_width_and_spacing`) and AC-N1 (`outer_inner_width_and_spacing_tdd::negative_spacing_config_is_a_fatal_module_error`) are qualified for exactly this reason: Step 3 puts both tests in the same file, and an unqualified AC-5 filter would swallow AC-N1's test.
- **Guest freshness rule.** Every step that edits `modules/core-modules/classic-perimeters/src/**` or `classic-perimeters.toml` must end with `cargo xtask build-guests --check`; if `STALE:` is reported, rebuild without `--check` before running or interpreting any test.
- All `cargo check` / `cargo clippy` **gate** invocations use `--all-targets`. The narrow `cargo test -p <crate> --test <binary>` verification commands do not — `--all-targets` is not valid with `--test`, and the narrow form is deliberate per `CLAUDE.md` §Test Discipline.

## Steps

### Step 1: RED — assert canonical wall-width resolution (D-164)

- Task IDs: `TASK-303`
- Objective: Add `crates/slicer-runtime/tests/integration/classic_wall_width_resolution_tdd.rs` with two tests driving `ClassicPerimeters::run_perimeters` directly: `zero_width_resolves_to_canonical_auto_extrusion_width` (config `nozzle_diameter = 0.6`, `outer_wall_line_width = 0.0`, `wall_count = 3`, 10 mm square → every `perimeter_index == 0` vertex width is `1.125 * 0.6 = 0.675` within `0.005`) and `absent_width_keys_still_resolve_to_legacy_default` (no `outer_wall_line_width`, `inner_wall_line_width`, or `line_width` key → outer and inner vertex widths both `0.4` within `0.005`). Register the file in `crates/slicer-runtime/tests/integration/main.rs`. The first test must FAIL on the current tree; the second must already PASS (it is a lock against regression, not a new behaviour).
- Precondition: `run_perimeters` still resolves widths through `match _config.get("outer_wall_line_width") { Some(ConfigValue::Float(w)) => *w as f32, _ => legacy_line_width }`, so `0.0` yields a literal `0.0` wall width, not `0.675`.
- Postcondition: `zero_width_resolves_to_canonical_auto_extrusion_width` fails with an observed width of `0.0` (or a vanished loop); `absent_width_keys_still_resolve_to_legacy_default` passes; the rest of the `integration` binary is unaffected.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-runtime/tests/integration/outer_inner_width_and_spacing_tdd.rs` — short; read in full. Reuse its `make_region` / `ConfigViewBuilder` / `find_max_x` setup wholesale.
  - `crates/slicer-runtime/tests/integration/main.rs` — the `mod` list only, to place the new registration alphabetically.
  - `modules/core-modules/classic-perimeters/src/lib.rs` — long; **one window only**: the R2 config-read block at the top of `run_perimeters`, from the `legacy_line_width` match through the `layer_height` read.
- Files allowed to edit (at most 3):
  - `crates/slicer-runtime/tests/integration/classic_wall_width_resolution_tdd.rs` (new)
  - `crates/slicer-runtime/tests/integration/main.rs`
- Files explicitly out of bounds:
  - `modules/core-modules/classic-perimeters/**` — no production edit in this step; the test must go red against unmodified source.
  - `crates/slicer-runtime/tests/integration/outer_inner_width_and_spacing_tdd.rs` — Step 3's surface.
  - `OrcaSlicerDocumented/**` — delegate only.
- Blast-radius discipline: not applicable — this step adds no struct field and bumps no schema/version constant. It adds one `mod` line, whose only fallout is the `integration` binary's own compilation.
- Expected sub-agent dispatches:
  - Question: confirm `Flow::new_from_config_width`'s auto branch condition and `Flow::auto_extrusion_width`'s return for `frExternalPerimeter`/`frPerimeter`; scope: `OrcaSlicerDocumented/src/libslic3r/Flow.cpp`; return: `SUMMARY` (≤ 150 words)
- Context cost: `S`
- Authoritative docs:
  - None required for this step.
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/Flow.cpp` — `Flow::auto_extrusion_width`; delegate, never load.
- Verification:
  - `bash -c 'cargo test -p slicer-runtime --test integration -- classic_wall_width_resolution_tdd::zero_width_resolves_to_canonical_auto_extrusion_width --nocapture 2>&1 | rg "^test result|panicked at|assertion"'` — FACT: the `test result:` line must show a non-zero `failed` count (RED for the right reason), and the panic must quote the observed width. Do **not** apply the GREEN ok-guard here — this step is RED by design.
  - `bash -c 'cargo test -p slicer-runtime --test integration -- classic_wall_width_resolution_tdd::absent_width_keys_still_resolve_to_legacy_default 2>&1 | rg "^test result" | rg -q "^test result: ok\. [1-9]" || echo "FAIL: 0 tests ran or test failed"'` — FACT pass/fail; must already be GREEN.
- Exit condition: the auto-sentinel test is RED quoting the observed `0.0`, the absent-key test is GREEN, and both are reachable by substring filter in the `integration` binary.

### Step 2: GREEN — retype the wall-width keys and resolve them via `get_abs_value` (D-164)

- Task IDs: `TASK-303`
- Objective: (a) In `classic-perimeters.toml`, set both `[config.schema.outer_wall_line_width]` and `[config.schema.inner_wall_line_width]` to `type = "float_or_percent"`, `min = 0.0`, `max = 2.0`, `unit = "mm"`, `default = 0.4` (value unchanged), each with a `description` naming canonical `PrintConfig.cpp`'s `coFloatOrPercent` declaration with `ratio_over = "nozzle_diameter"` and upstream default `0`. **The description must contain no `[` character.** (b) In `run_perimeters`, move the `nozzle_diameter` read **above** the two width reads and change its fallback from `inner_wall_line_width` to `legacy_line_width`, breaking the read cycle. (c) Resolve each width as `_config.get_abs_value(key, nozzle_diameter as f64)` with three arms: `Some(v) if v > 0.0 => v as f32`; `Some(_) => 1.125 * nozzle_diameter` (canonical `Flow.cpp::auto_extrusion_width`); `None => legacy_line_width`. (d) Update the `nozzle_diameter` explanatory comment in `crates/slicer-runtime/tests/integration/manifest_default_reconcile_tdd.rs`'s `CLASSIC_FALLBACKS` — its `Float(0.4)` **value is unchanged** because `legacy_line_width` still bottoms out at `0.4`, but the comment's "falls back to `inner_wall_line_width`" chain is now wrong.
- Precondition: Step 1 complete; `zero_width_resolves_to_canonical_auto_extrusion_width` is RED. The two wall-width rows in `CLASSIC_FALLBACKS` are `Float(0.4)` and must **stay** `Float(0.4)` — `assert_exhaustive_reconcile` parses the raw TOML `default`, and a bare TOML `0.4` remains a float regardless of the `type` string.
- Postcondition: both Step 1 tests pass; the manifest declares both keys as `float_or_percent` with `min = 0.0`; `manifest_default_reconcile_tdd` is green; every currently-green classic test remains green.
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/classic-perimeters/src/lib.rs` — long; **one window**: the R2 config-read block at the top of `run_perimeters` (the `legacy_line_width` match through the `layer_height` read). Locate by symbol; treat any line pin as a hint.
  - `modules/core-modules/classic-perimeters/classic-perimeters.toml` — the two wall-width `[config.schema]` blocks and the `nozzle_diameter` block.
  - `modules/core-modules/arachne-perimeters/arachne-perimeters.toml` — the `overhang_reverse_threshold` and `min_width_top_surface` blocks only, as `float_or_percent` precedent.
  - `crates/slicer-runtime/tests/integration/manifest_default_reconcile_tdd.rs` — the `CLASSIC_FALLBACKS` table and the `assert_exhaustive_reconcile` value-equality arm.
- Files allowed to edit (at most 3):
  - `modules/core-modules/classic-perimeters/classic-perimeters.toml`
  - `modules/core-modules/classic-perimeters/src/lib.rs`
  - `crates/slicer-runtime/tests/integration/manifest_default_reconcile_tdd.rs`
- Files explicitly out of bounds:
  - `crates/slicer-runtime/tests/integration/classic_wall_width_resolution_tdd.rs` — frozen after Step 1; do not weaken it to obtain a pass.
  - `crates/slicer-ir/src/resolved_config.rs` — the ingestion half is out of scope and would stale all 34 guest WASMs.
  - `modules/core-modules/arachne-perimeters/**` — read-only precedent.
  - `crates/slicer-gcode/**` — packet 182's surface.
- Blast-radius discipline: no struct field, no schema/version constant. The measured fallout of a manifest **default or key** change is `crates/slicer-runtime/tests/integration/manifest_default_reconcile_tdd.rs`'s exhaustive set-equality plus per-key value equality — which is why that file is in this step's edit list rather than deferred to a trailing `cargo check`. Holding `default = 0.4` is what keeps the two wall-width rows unchanged; only the `nozzle_diameter` comment moves.
- Expected sub-agent dispatches:
  - Question: confirm `PrintConfigDef::init_fff_params`' exact declaration of `outer_wall_line_width` / `inner_wall_line_width` (`coFloatOrPercent`, `ratio_over`, `min`, default); scope: `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp`; return: `SUMMARY` (≤ 150 words)
  - Question: does `parse_config_field_entry` accept a bare TOML float `0.4` as the `default` of a `type = "float_or_percent"` field, and does `check_scalar` compare `min`/`max` against the unresolved magnitude?; scope: `crates/slicer-scheduler/src/manifest.rs`, `crates/slicer-scheduler/src/config_resolution.rs`; return: `FACT` (≤ 8 lines)
- Context cost: `S`
- Authoritative docs:
  - `docs/15_config_keys_reference.md` — delegated `rg` on the two key names only; the doc edit itself is Step 8.
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp`, `OrcaSlicerDocumented/src/libslic3r/Flow.cpp` — delegate; never load.
- Verification:
  - `bash -c 'M=modules/core-modules/classic-perimeters/classic-perimeters.toml; ok=1; for k in outer_wall_line_width inner_wall_line_width; do rg -U -q "\[config\.schema\.$k\][^\[]*type\s*=\s*\"float_or_percent\"" $M || ok=0; rg -U -q "\[config\.schema\.$k\][^\[]*min\s*=\s*0\.0" $M || ok=0; rg -U -q "\[config\.schema\.$k\][^\[]*default\s*=\s*0\.4" $M || ok=0; done; [ $ok = 1 ] && echo PASS || echo FAIL'` — FACT PASS/FAIL (AC-1).
  - `bash -c 'cargo test -p slicer-runtime --test integration -- classic_wall_width_resolution_tdd 2>&1 | rg "^test result" | rg -q "^test result: ok\. [1-9]" || echo "FAIL: 0 tests ran or a test failed"'` — FACT pass/fail (AC-2, AC-3, AC-N2).
  - `bash -c 'cargo test -p slicer-runtime --test integration -- manifest_default_reconcile_tdd 2>&1 | rg "^test result" | rg -q "^test result: ok\. [1-9]" || echo "FAIL: reconcile red"'` — FACT pass/fail.
  - `bash -c 'cargo xtask build-guests --check 2>&1 | rg -c "STALE:" || echo "0 stale"'` — FACT: rebuild without `--check` if non-zero.
- Exit condition: AC-1, AC-2, AC-3, AC-N2 all PASS; `manifest_default_reconcile_tdd` green; guests fresh.

### Step 3: RED — derive the spacing expectations from `line_width_to_spacing` and assert the fatal error path (D-105)

- Task IDs: `TASK-303`
- Objective: Rewrite `crates/slicer-runtime/tests/integration/outer_inner_width_and_spacing_tdd.rs`'s five spacing expectations so they are **computed** from `slicer_core::flow::line_width_to_spacing(width, 0.2).unwrap()` instead of transcribed decimals — `ext_perimeter_spacing2 = 0.5 * (line_width_to_spacing(outer_w, 0.2)? + line_width_to_spacing(inner_w, 0.2)?)` for the outer→first-inner gap and the first-inner X, and `line_width_to_spacing(inner_w, 0.2)?` for the first→second gap and the second-inner X. **`expected_outer_right = half_side - outer_w / 2.0` (4.75) is unchanged** — canonical's first-loop inset is `ext_perimeter_width / 2`, which PnP already matches. Also add `negative_spacing_config_is_a_fatal_module_error`: config `inner_wall_line_width = 0.4`, `layer_height = 2.0` (so `spacing = 0.4 - 2.0 * (1 - PI/4) < 0`), asserting `run_perimeters` returns `Err` whose `code` is `1` — not `Ok`, not a panic. Keep the module doc comment's stated values in sync with the derivation. Must FAIL on the current tree.
- Precondition: Step 2 complete and green. `emit_walls` still computes `ext_perimeter_spacing2` as `(outer_wall_line_width + inner_wall_line_width) / 2.0` and the `i >= 2` inset as `-inner_wall_line_width`, and there is no fallible path at all, so `negative_spacing_config_is_a_fatal_module_error` currently gets `Ok`.
- Postcondition: `outer_inner_width_and_spacing` fails on the first-inner/second-inner X and both gaps (the outer X assertion still passes), and `negative_spacing_config_is_a_fatal_module_error` fails because `run_perimeters` returned `Ok`.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-runtime/tests/integration/outer_inner_width_and_spacing_tdd.rs` — short; read in full.
  - `crates/slicer-runtime/tests/integration/perimeter_parity.rs` — the `use slicer_core::flow::{flow_to_width, line_width_to_spacing};` line and its `flow_to_width(line_width_to_spacing(...))` usage, to confirm the import is already available inside the `integration` binary.
  - `crates/slicer-core/src/flow.rs` — `line_width_to_spacing`, `flow_to_width`, `NegativeSpacingError` only.
  - `crates/slicer-runtime/tests/integration/precise_outer_wall_tdd.rs` — **read only**, to confirm it asserts nothing beyond wall widths and the *outer* wall's X, and therefore does not move.
- Files allowed to edit (at most 3):
  - `crates/slicer-runtime/tests/integration/outer_inner_width_and_spacing_tdd.rs`
- Files explicitly out of bounds:
  - `modules/core-modules/classic-perimeters/**` — no production edit in this step.
  - `crates/slicer-runtime/tests/integration/precise_outer_wall_tdd.rs` — must stay byte-identical; it is AC-6's do-not-change criterion.
  - `crates/slicer-core/src/flow.rs` — consumed, never modified.
- Blast-radius discipline: not applicable — no struct field, no schema/version constant. The transcribed-constant blast radius of this packet is exactly the five expectations in this file plus the reconcile table row (Steps 2 and 7); both are enumerated rather than discovered.
- Expected sub-agent dispatches:
  - Question: confirm `PerimeterGenerator::process_classic`'s `ext_perimeter_spacing2` precise/non-precise branches and its `distance = (i == 1) ? ext_perimeter_spacing2 : perimeter_spacing` inset selection; scope: `OrcaSlicerDocumented/src/libslic3r/PerimeterGenerator.cpp`; return: `SUMMARY` (≤ 200 words)
- Context cost: `S`
- Authoritative docs:
  - None required for this step.
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/PerimeterGenerator.cpp` — `process_classic`; delegate, never load.
- Verification:
  - `bash -c 'cargo test -p slicer-runtime --test integration -- outer_inner_width_and_spacing_tdd::outer_inner_width_and_spacing --nocapture 2>&1 | rg "^test result|panicked at|Gap outer|Gap first|inner wall right edge"'` — FACT: non-zero `failed` count; the panic must name a gap or inner-wall-X assertion, **not** the outer-wall-X assertion. Do not apply the AC-form ok-guard (`rg -q "^test result: ok\. [1-9]"`) here — this step is RED by design and the guard would correctly, but uselessly, print `FAIL`.
  - `bash -c 'cargo test -p slicer-runtime --test integration -- outer_inner_width_and_spacing_tdd::negative_spacing_config_is_a_fatal_module_error --nocapture 2>&1 | rg "^test result|panicked at"'` — FACT: non-zero `failed` count.
  - `bash -c 'T=crates/slicer-runtime/tests/integration/outer_inner_width_and_spacing_tdd.rs; ! rg -q "0\.45" $T && ! rg -q -F "(outer_w + inner_w) / 2.0" $T && echo PASS || echo "FAIL: width-average 0.45 comment or (outer_w + inner_w) / 2.0 assertion survived"'` — FACT PASS/FAIL: **both** halves of the old width-average must go. The literal `0.45` today lives only in the module doc comment; the assertions themselves already spell the width-average algebraically as `(outer_w + inner_w) / 2.0` (three occurrences). Checking only `0.45` would let a comment-only edit satisfy this probe.
- Exit condition: both tests RED for the stated reasons, every expectation in the file derived by a `line_width_to_spacing` call, and `crates/slicer-runtime/tests/integration/precise_outer_wall_tdd.rs` untouched.

### Step 4: GREEN — wire `line_width_to_spacing` into `emit_walls` (D-105)

- Task IDs: `TASK-303`
- Objective: In `ClassicPerimeters::emit_walls`, add `const ERR_NEGATIVE_SPACING: u32 = 1;` (module-local, same value and shape as `arachne-perimeters`') and `use slicer_core::flow::line_width_to_spacing;`, then hoist above the `for i in 0..wall_count` loop: `ext_perimeter_spacing = line_width_to_spacing(outer_wall_line_width, layer_height).map_err(|e| ModuleError::fatal(ERR_NEGATIVE_SPACING, e.to_string()))?` and the same for `perimeter_spacing` from `inner_wall_line_width`; `ext_perimeter_spacing2 = if precise_outer_wall { 0.5 * (outer_wall_line_width + inner_wall_line_width) } else { 0.5 * (ext_perimeter_spacing + perimeter_spacing) }`. Inset deltas become: `i == 0` → `-ext_perimeter_spacing2` when `precise_outer_wall`, else `-(outer_wall_line_width / 2.0)` (unchanged — canonical `ext_perimeter_width / 2`); `i == 1` → `-ext_perimeter_spacing2`; `i >= 2` → `-perimeter_spacing`. In the gap-fill block, `min_gap_fill_width = 0.2 * outer_wall_line_width.min(inner_wall_line_width) * (1.0 - 0.4_f32)` (canonical `0.2 * std::min(perimeter_width, ext_perimeter_width) * (1 - INSET_OVERLAP_TOLERANCE)`; **corrected 2026-07-31** — an earlier revision of this step used two tenths as the tolerance factor here, which was wrong. `INSET_OVERLAP_TOLERANCE` is declared exactly once, as `static constexpr double INSET_OVERLAP_TOLERANCE = 0.4;` in OrcaSlicer's `libslic3r/libslic3r.h`, verified twice by independent dispatch. The leading `0.2` is a separate canonical coefficient and is unchanged) and `max_gap_fill_width = 2.0 * perimeter_spacing` using the hoisted true spacing, replacing the local width-average `perimeter_spacing` binding. **Do not change `emit_walls`' signature** — `layer_height`, `precise_outer_wall`, and both widths are already parameters, and all five call sites in `run_perimeters` must remain untouched. Update the OrcaSlicer citations on the touched comments to function names only, dropping their legacy line pins.
- Precondition: Step 3 complete; `outer_inner_width_and_spacing` and `negative_spacing_config_is_a_fatal_module_error` are both RED.
- Postcondition: both Step 3 tests pass; `precise_outer_wall_tdd` still passes unmodified; `emit_walls` still has exactly 26 parameters and exactly five call sites.
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/classic-perimeters/src/lib.rs` — long; **two windows only**: (a) `emit_walls`' parameter list through the `for i in 0..wall_count` inset-delta chain; (b) the `if emit_inner && !gaps.is_empty() && medial_axis_enabled` gap-fill block containing `min_gap_fill_width` / `perimeter_spacing` / `max_gap_fill_width`. Locate both by symbol.
  - `modules/core-modules/arachne-perimeters/src/lib.rs` — the `ERR_NEGATIVE_SPACING` declaration and one `.map_err(|e| ModuleError::fatal(ERR_NEGATIVE_SPACING, e.to_string()))?` site, as the shape to mirror. **Read-only.**
  - `crates/slicer-core/src/flow.rs` — `line_width_to_spacing` and `NegativeSpacingError` only.
- Files allowed to edit (at most 3):
  - `modules/core-modules/classic-perimeters/src/lib.rs`
- Files explicitly out of bounds:
  - `crates/slicer-runtime/tests/integration/outer_inner_width_and_spacing_tdd.rs` — frozen after Step 3; do not weaken assertions or widen the `0.005` tolerance to obtain a pass.
  - `crates/slicer-runtime/tests/integration/precise_outer_wall_tdd.rs`, `crates/slicer-runtime/tests/integration/gap_fill_emission_tdd.rs` — must stay green unmodified.
  - `modules/core-modules/arachne-perimeters/**` — read-only shape reference.
  - `crates/slicer-core/src/flow.rs` — consumed, never modified.
  - `crates/slicer-runtime/tests/fixtures/golden/precision_legacy_20mmbox.gcode` — Step 5's surface; do not re-bless here.
- Blast-radius discipline: no struct field, no schema/version constant, **and no `emit_walls` signature change** — so there is no call-site fallout. The real fallout is **recorded output**: the byte-identity golden, owned by Step 5. Do not treat this step as complete on the grounds that nothing else failed to compile.
- Expected sub-agent dispatches:
  - Question: confirm `PerimeterGenerator::process_classic`'s gap-fill bounds — `min = 0.2 * std::min(perimeter_width, ext_perimeter_width) * (1 - INSET_OVERLAP_TOLERANCE)` and `max = 2. * perimeter_spacing` — and the value of `INSET_OVERLAP_TOLERANCE`; scope: `OrcaSlicerDocumented/src/libslic3r/PerimeterGenerator.cpp`; return: `SUMMARY` (≤ 150 words)
  - Question: report `arachne-perimeters`' `ERR_NEGATIVE_SPACING` declaration and one `.map_err` site verbatim; scope: `modules/core-modules/arachne-perimeters/src/lib.rs`; return: `SNIPPETS` (≤ 10 lines)
- Context cost: `M` — four interlocking canonical formulas plus the no-signature-change constraint; the largest step in the packet.
- Authoritative docs:
  - `docs/08_coordinate_system.md` — delegated `SUMMARY` only if a mm↔unit question arises. All arithmetic here is mm-domain.
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/PerimeterGenerator.cpp` — `process_classic`; delegate, never load.
- Verification:
  - `bash -c 'T=crates/slicer-runtime/tests/integration/outer_inner_width_and_spacing_tdd.rs; ! rg -q "0\.45" $T && ! rg -q -F "(outer_w + inner_w) / 2.0" $T && rg -q "line_width_to_spacing" $T && cargo test -p slicer-runtime --test integration -- outer_inner_width_and_spacing_tdd::outer_inner_width_and_spacing 2>&1 | rg "^test result" | rg -q "^test result: ok\. [1-9]" || echo "FAIL: width-average 0.45 comment or (outer_w + inner_w) / 2.0 assertion survived, spacing not derived via line_width_to_spacing, 0 tests ran, or test failed"'` — FACT pass/fail (AC-5). **All three static probes are load-bearing and must not be dropped:** the cargo run alone passes on the pre-Step-3 tree (measured `test result: ok. 1 passed; 0 failed; 246 filtered out`), because the old test asserts the old width-average numbers; and the `(outer_w + inner_w) / 2.0` clause is what stops a comment-only edit from satisfying the `0.45` clause. Step 3's rewrite is what makes this command discriminating.
  - `bash -c 'cargo test -p slicer-runtime --test integration -- outer_inner_width_and_spacing_tdd::negative_spacing_config_is_a_fatal_module_error 2>&1 | rg "^test result" | rg -q "^test result: ok\. [1-9]" || echo "FAIL: 0 tests ran or error path missing"'` — FACT pass/fail (AC-N1).
  - `bash -c 'F=modules/core-modules/classic-perimeters/src/lib.rs; A=$(rg -n "fn emit_walls" $F | head -1 | cut -d: -f1); B=$(rg -n "fn emit_nonplanar_shells" $F | head -1 | cut -d: -f1); { [ -n "$A" ] && [ -n "$B" ] && [ "$A" -lt "$B" ]; } || { echo "FAIL: emit_walls / emit_nonplanar_shells markers missing or out of order"; exit 0; }; W=$(sed -n "${A},$((B-1))p" $F); { rg -q "slicer_core::flow::line_width_to_spacing" $F || rg -q "flow::line_width_to_spacing" $F || rg -q "line_width_to_spacing" $F; } && rg -q "ERR_NEGATIVE_SPACING" $F && printf "%s\n" "$W" | rg -q -- "-perimeter_spacing" && ! printf "%s\n" "$W" | rg -q -- "-inner_wall_line_width$" && echo PASS || echo "FAIL: line_width_to_spacing/ERR_NEGATIVE_SPACING missing, or the emit_walls inset chain still uses -inner_wall_line_width instead of -perimeter_spacing"'` — FACT PASS/FAIL (AC-4). **The `emit_walls` window scoping must not be simplified back to a whole-file `! rg -q -- "-inner_wall_line_width$"`.** That form is unsatisfiable: `emit_nonplanar_shells` carries a second end-of-line occurrence which `[FWD-3]` and §Out of Scope declare untouched, so a whole-file negation could only pass by editing out-of-scope code.
  - `bash -c 'cargo test -p slicer-runtime --test integration -- precise_outer_wall_tdd 2>&1 | rg "^test result" | rg -q "^test result: ok\. [1-9]" || echo "FAIL: precise-mode regression"'` — FACT pass/fail (AC-6).
  - `bash -c 'cargo test -p slicer-runtime --test integration -- gap_fill_emission_tdd 2>&1 | rg "^test result" | rg -q "^test result: ok\. [1-9]" || echo "FAIL: gap-fill regression"'` — FACT pass/fail.
  - `bash -c 'cargo xtask build-guests --check 2>&1 | rg -c "STALE:" || echo "0 stale"'` — FACT: rebuild without `--check` if non-zero.
- Exit condition: AC-4, AC-5, AC-6, AC-N1 all PASS; `gap_fill_emission_tdd` green; guests fresh; `emit_walls` signature and all five call sites unchanged.

### Step 5: Re-bless the byte-identity golden and re-verify the two-sided invariants (D-105 fallout)

- Task IDs: `TASK-303`
- Objective: Re-bless `crates/slicer-runtime/tests/fixtures/golden/precision_legacy_20mmbox.gcode`, which is produced by **classic** (`LEGACY_PRECISION_JSON` carries no `wall_generator` key; `slicer_scheduler::execution_plan::DEFAULT_WALL_GENERATOR` is `"classic"`; the golden records `; wall_generator = Classic`) and which Step 4's inset change moves. Then re-verify the two tests that straddle both wall generators: `arachne_structural_invariants` (classic sits on **both** sides of its `min(a,c)/max(a,c) >= 0.99` ratio) and `perimeter_parity` (whose classic-driving test is `annulus_true_hole_produces_inner_perimeters`). Inspect the golden diff and confirm it shows coordinate drift on wall moves only — no changed layer count, no changed `;TYPE:` sequence, no changed header keys.
- Precondition: Step 4 complete and green. **`cargo build --workspace` has been run** — `legacy_zero_matches_golden` shells out to a `pnp_cli` executable located on disk, and `crates/slicer-runtime` has no dependency on the CLI crate, so `cargo test` alone will not rebuild it; re-blessing against a stale binary silently re-records the *pre-fix* geometry and yields a green test that still encodes the bug. `cargo check` is insufficient — it produces no executable. **`cargo xtask build-guests --check` must also be clean**, since this test drives the real core-module guests via `pnp_cli --module-dir`.
- Postcondition: `legacy_zero_matches_golden` green; `arachne_structural_invariants` green with `COVERAGE_THRESHOLD` still `0.99`; `perimeter_parity` green; the golden still records `; wall_generator = Classic`.
- Bless command (run only after `cargo build --workspace` and a clean `build-guests --check`): `BLESS_GOLDEN=1 cargo test -p slicer-runtime --test e2e -- legacy_zero_matches_golden --nocapture`
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-runtime/tests/e2e/slicing_precision_integration_tdd.rs` — the `legacy_zero_matches_golden` body only. **Its in-file `BLESS_GOLDEN=1 … --test slicing_precision_integration_tdd` hint is stale and names a cargo target that does not exist; the real binary is `--test e2e`. Do not follow it verbatim.**
  - `crates/slicer-runtime/tests/arachne_structural_invariants.rs` — the `COVERAGE_THRESHOLD` declaration and the ratio assertion only.
  - `crates/slicer-runtime/tests/fixtures/golden/precision_legacy_20mmbox.gcode` — **never load; thousands of lines.** Locate individual lines by `rg`; verify the change by `git diff --numstat` and `git diff --stat`.
- Files allowed to edit (at most 3):
  - `crates/slicer-runtime/tests/fixtures/golden/precision_legacy_20mmbox.gcode`
- Files explicitly out of bounds:
  - Every other fixture under `crates/slicer-runtime/tests/fixtures/golden/` — do not bulk re-bless.
  - `crates/slicer-runtime/tests/e2e/slicing_precision_integration_tdd.rs` — the test asserts byte identity; re-bless the **data**, never relax the assertion.
  - `crates/slicer-runtime/tests/arachne_structural_invariants.rs` — **never lower `COVERAGE_THRESHOLD`.** If the ratio breaks, the spacing change is wrong, not the threshold.
  - `crates/slicer-runtime/tests/fixtures/perimeter_parity/**` — the arachne T2 sibling packet owns these configs.
  - `modules/core-modules/**` — frozen after Step 4.
- Blast-radius discipline: this step **is** the recorded-output blast radius Step 4 defers to it. Before editing, confirm via the dispatch below that exactly one golden is classic-produced.
- **Cross-packet collision on this exact file.** Draft packet `182-gcode-header-width-defaults` Step 3 re-blesses the *same* `precision_legacy_20mmbox.gcode`, for a different reason (its two `; …_line_width` header comment lines, `0.42`/`0.45` → `0.4`/`0.4`). Both packets are `draft`; neither may assume it lands first. **Whichever lands second re-blesses and re-verifies rather than assuming** — the same rule this packet already applies to `arachne_structural_invariants`' coverage ratio. Practical consequence for this step's diff inspection below: if packet 182 landed first, the two header lines will already read `0.4` and must **not** be treated as fallout of the spacing change; if this packet lands first, packet 182's `2 insertions / 2 deletions` numstat expectation is measured against the post-Step-5 baseline, not against `master`. Before re-blessing, run `git log --oneline -1 -- crates/slicer-gcode/src/serialize.rs` and check whether `outer_wall_line_width: 0.42` still stands in that file; record which order actually happened in the step's completion note. The arachne T2 sibling packet does **not** touch this golden — `legacy_zero_matches_golden` runs Classic (`LEGACY_PRECISION_JSON` carries no `wall_generator` key, `slicer_scheduler::execution_plan::DEFAULT_WALL_GENERATOR` is `"classic"`, golden records `; wall_generator = Classic`).
- Expected sub-agent dispatches:
  - Question: which files under `crates/slicer-runtime/tests/fixtures/golden/` contain the line `; wall_generator = Classic`?; scope: `crates/slicer-runtime/tests/fixtures/golden/**`; return: `LOCATIONS` (≤ 20 entries)
  - Question: does `cargo build --workspace` succeed and does `cargo xtask build-guests --check` report clean?; scope: repo root; return: `FACT` (≤ 5 lines)
- Context cost: `S`
- Authoritative docs:
  - None required for this step.
- OrcaSlicer refs:
  - None — this step records output, it ports nothing.
- Verification:
  - `bash -c 'ls target/debug/pnp_cli target/debug/pnp_cli.exe target/release/pnp_cli target/release/pnp_cli.exe 2>/dev/null | rg -q . || { echo "FAIL (precondition): pnp_cli binary absent under target/{debug,release} - run cargo build --bin pnp_cli first; AC-10 cannot discriminate without it"; exit 0; }; rg -q "line_width_to_spacing" modules/core-modules/classic-perimeters/src/lib.rs && rg -q "^; wall_generator = Classic$" crates/slicer-runtime/tests/fixtures/golden/precision_legacy_20mmbox.gcode && cargo test -p slicer-runtime --test e2e -- legacy_zero_matches_golden 2>&1 | rg "^test result" | rg -q "^test result: ok\. [1-9]" || echo "FAIL: classic still computes the width-average wall gap (line_width_to_spacing unwired), or the golden was not re-blessed, or its wall_generator line changed, or 0 tests ran"'` — FACT pass/fail (AC-10). **The precondition and the `line_width_to_spacing` discriminator are load-bearing and must not be dropped.** A golden test passes by construction on the tree that produced it, so with `pnp_cli` built the bare cargo form returns `ok. 1 passed` even on a completely untouched tree; and with `pnp_cli` *absent* it returns `FAILED` for a purely environmental reason (`pnp_cli_bin` in `crates/slicer-runtime/tests/common/slicer_cache.rs` panics with `pnp_cli binary not found`), which would masquerade as a real red. The precondition's distinct message keeps the two apart.
  - `bash -c 'rg -q "COVERAGE_THRESHOLD: f64 = 0\.99" crates/slicer-runtime/tests/arachne_structural_invariants.rs && cargo test -p slicer-runtime --test arachne_structural_invariants 2>&1 | rg "^test result" | rg -q "^test result: ok\. [1-9]" || echo "FAIL: threshold weakened or invariants red"'` — FACT pass/fail (AC-11).
  - `bash -c 'cargo test -p slicer-runtime --test integration -- perimeter_parity 2>&1 | rg "^test result" | rg -q "^test result: ok\. [1-9]" || echo "FAIL: parity fixtures red"'` — FACT pass/fail.
  - `bash -c 'git diff --stat crates/slicer-runtime/tests/fixtures/golden/precision_legacy_20mmbox.gcode; git diff crates/slicer-runtime/tests/fixtures/golden/precision_legacy_20mmbox.gcode | rg "^[-+];" | head -40'` — FACT: the changed-comment listing must be empty or confined to width metadata; **any changed `;LAYER_CHANGE` or `;TYPE:` line means the change is structural, not a spacing shift, and must be diagnosed before proceeding.**
- Exit condition: AC-10 and AC-11 PASS, `perimeter_parity` green, and the golden diff contains no structural comment change.

### Step 6: RED — assert the `min_width_top_surface` gate on the `only_one_wall_top` collapse (D-152)

- Task IDs: `TASK-303`
- Objective: Add `crates/slicer-runtime/tests/integration/classic_min_width_top_surface_tdd.rs` with a test that builds a region whose `top_solid_fill` contains at least one sub-area narrower than a configured `min_width_top_surface` and one wider, sets `only_one_wall_top = true` with `top_shell_index = Some(n)` for `n > 0` and `wall_count = 3`, runs `ClassicPerimeters::run_perimeters`, and asserts that the narrow sub-area's walls keep the full `layer_wall_count` (3) rather than collapsing to 1, while the wide sub-area collapses to 1. Register the file in `crates/slicer-runtime/tests/integration/main.rs`. Must FAIL on the current tree.
- Precondition: Steps 1-5 complete and green. `run_perimeters` still contains `let _ = min_width_top_surface;` and neither `only_one_wall_top` site consults the value, so **every** top sub-area collapses to one wall.
- Postcondition: the new test fails, reporting that the narrow sub-area collapsed to 1 wall; the rest of the `integration` binary is unaffected.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-runtime/tests/integration/outer_inner_width_and_spacing_tdd.rs` — full; reuse its region/config builder setup.
  - `crates/slicer-sdk/src/views.rs` — `SliceRegionView::top_shell_index` and `top_solid_fill` signatures only, via dispatch if the file is large.
  - `modules/core-modules/classic-perimeters/src/lib.rs` — long; **one window**: the `else if only_one_wall_top && matches!(top_shell, Some(n) if n > 0)` branch with its `split_top_surfaces` call and two `emit_walls` calls.
  - `modules/core-modules/arachne-perimeters/src/lib.rs` — `emit_only_one_wall_top_second_pass`'s gate sequence, as the behaviour to mirror. **Read-only.**
- Files allowed to edit (at most 3):
  - `crates/slicer-runtime/tests/integration/classic_min_width_top_surface_tdd.rs` (new)
  - `crates/slicer-runtime/tests/integration/main.rs`
- Files explicitly out of bounds:
  - `modules/core-modules/**` — no production edit in this step.
  - `crates/slicer-runtime/tests/arachne_parity.rs` — asserts presence of the `"min_width_top_surface"` and `"only_one_wall_top"` literals in classic's `modules/core-modules/classic-perimeters/src/lib.rs`; read if needed, never edit.
- Blast-radius discipline: not applicable — no struct field, no schema/version constant. One `mod` line, whose only fallout is the `integration` binary's compilation.
- Expected sub-agent dispatches:
  - Question: confirm `PerimeterGenerator::split_top_surfaces` uses `min_width_top_surface` as an erosion threshold (`offset`/`diff_ex` on the upper-slice series, floored at `ext_perimeter_spacing/2 + 10`, resolved with `get_abs_value` against the perimeter width) and **not** as a per-loop width comparison; scope: `OrcaSlicerDocumented/src/libslic3r/PerimeterGenerator.cpp`; return: `SUMMARY` (≤ 150 words)
  - Question: report `ArachnePerimeters::emit_only_one_wall_top_second_pass`'s `min_width_top_surface` resolution, `retain` predicate, and `offset2_ex` arguments verbatim, plus the free fn `ex_polygon_min_width_mm`; scope: `modules/core-modules/arachne-perimeters/src/lib.rs`; return: `SNIPPETS` (≤ 25 lines)
- Context cost: `S`
- Authoritative docs:
  - `docs/15_config_keys_reference.md` — delegated `rg` on `min_width_top_surface` only, to read the recorded classic-vs-arachne divergence.
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/PerimeterGenerator.cpp` — `split_top_surfaces`; delegate, never load.
- Verification:
  - `bash -c 'cargo test -p slicer-runtime --test integration -- classic_min_width_top_surface_tdd --nocapture 2>&1 | rg "^test result|panicked at|assertion"'` — FACT: non-zero `failed` count, with the panic naming the narrow sub-area's observed wall count. Do not apply the AC-form ok-guard (`rg -q "^test result: ok\. [1-9]"`) here — this step is RED by design and the guard would correctly, but uselessly, print `FAIL`.
- Exit condition: the test is RED for the stated reason and is reachable by substring filter in the `integration` binary.

### Step 7: GREEN — wire `min_width_top_surface` and converge the manifest with arachne (D-152)

- Task IDs: `TASK-303`
- Objective: (a) Replace the read-`debug_assert!`-discard block in `run_perimeters` with `let min_width_top = _config.get_abs_value("min_width_top_surface", perimeter_width_mm).unwrap_or(0.0);`, where `perimeter_width_mm` is the resolved inner wall width (canonical's `ratio_over = "inner_wall_line_width"`). **Delete the `let _ = min_width_top_surface;` line but keep both the `"only_one_wall_top"` and `"min_width_top_surface"` string literals present** — `crates/slicer-runtime/tests/arachne_parity.rs` `include_str!`s this file and requires both. (b) At the `only_one_wall_top` split site, apply the arachne-shaped gate to the top portion when `min_width_top > 0.0`: `retain` sub-areas whose minimum bounding-box extent is `>= min_width_top`, then `offset2_ex(&top_area, -min_width_top, min_width_top + 0.85 * perimeter_width_mm, OffsetJoinType::Miter, 3.0)` with a fallback to the unexpanded set when the result is empty. Add a free fn mirroring arachne's `ex_polygon_min_width_mm` (min of bbox width and height via `units_to_mm`). Dropped sub-areas fall through to the full-`layer_wall_count` path. **Do not change `split_top_surfaces`' signature** — classic reuses that free fn for a second, non-top purpose with `region.overhang_areas()` as the mask. `offset2_ex`, `OffsetJoinType`, and `split_top_surfaces` are already imported; no new import is needed for the gate. (c) In `classic-perimeters.toml`, retype `[config.schema.min_width_top_surface]` to `type = "float_or_percent"`, `default = "0.0"`, `unit = "%"`, `min = 0.0`, matching arachne verbatim, and rewrite its `description` (which today claims no `coFloatOrPercent` precedent exists — it does now) with **no `[` character**. (d) Change `crates/slicer-runtime/tests/integration/manifest_default_reconcile_tdd.rs`'s `CLASSIC_FALLBACKS` row for `min_width_top_surface` from `Float(1.2)` to `Str("0.0")`. (e) **Drop the OrcaSlicer line pins from every comment this step rewrites**, exactly as Step 4 requires. The `min_width_top_surface` read-`debug_assert!`-discard block that (a) replaces carries two legacy pins — `PerimeterGenerator.cpp:2160-2245` and `PrintConfig.cpp:1491-1511` — and its "deferred, see D-104d-MIN-WIDTH-TOP-SURFACE-NONE" text becomes false the moment the gate is wired. Rewrite it to cite `PerimeterGenerator.cpp::split_top_surfaces` and `PrintConfigDef::init_fff_params` by **function name only**, per CLAUDE.md §OrcaSlicer Citation Style ("Drop the line numbers on any citation you touch"). Do not mass-rewrite untouched citations elsewhere in the file.
- Precondition: Step 6 complete; `classic_min_width_top_surface_tdd` is RED. The classic `CLASSIC_FALLBACKS` row is `("min_width_top_surface", Float(1.2))` and the arachne row is already `("min_width_top_surface", Str("0.0"))`.
- Postcondition: Step 6's test passes; `manifest_default_reconcile_tdd` green with the row updated; `arachne_parity` green with both literals still present; the gate is **off at the `"0.0"` default**, so no default-config geometry moves and the golden does not move a second time.
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/classic-perimeters/src/lib.rs` — long; **two windows**: the `min_width_top_surface` read-`debug_assert!`-discard block, and the `else if only_one_wall_top && matches!(top_shell, Some(n) if n > 0)` branch with its `split_top_surfaces` and two `emit_walls` calls.
  - `modules/core-modules/arachne-perimeters/src/lib.rs` — `emit_only_one_wall_top_second_pass` and `ex_polygon_min_width_mm`. **Read-only.**
  - `modules/core-modules/arachne-perimeters/arachne-perimeters.toml` — the `[config.schema.min_width_top_surface]` block, to copy type/default/unit exactly.
  - `crates/slicer-core/src/top_surface_split.rs` — `split_top_surfaces` and `TopSurfaceSplit` signatures only, to confirm no threshold parameter exists and none is being added.
- Files allowed to edit (at most 3):
  - `modules/core-modules/classic-perimeters/src/lib.rs`
  - `modules/core-modules/classic-perimeters/classic-perimeters.toml`
  - `crates/slicer-runtime/tests/integration/manifest_default_reconcile_tdd.rs`
- Files explicitly out of bounds:
  - `crates/slicer-runtime/tests/integration/classic_min_width_top_surface_tdd.rs` — frozen after Step 6.
  - `crates/slicer-core/src/top_surface_split.rs` — signature unchanged; the gate goes at the call site.
  - `modules/core-modules/arachne-perimeters/**` — read-only shape reference.
  - `crates/slicer-runtime/tests/arachne_parity.rs` — must stay green unmodified; it enforces literal presence, so satisfy it by keeping the identifiers, never by editing it.
  - `crates/slicer-runtime/tests/fixtures/golden/precision_legacy_20mmbox.gcode` — the `"0.0"` default keeps the gate off, so this must **not** move again. If it does, the gate is firing at the default and the implementation is wrong.
- Blast-radius discipline: no struct field, no schema/version constant. The manifest **default** change to `min_width_top_surface` has a measured fallout of exactly one row in `crates/slicer-runtime/tests/integration/manifest_default_reconcile_tdd.rs`'s `CLASSIC_FALLBACKS` (`Float(1.2)` → `Str("0.0")`), which is in this step's edit list. `assert_exhaustive_reconcile` enforces set-equality in both directions plus per-key value equality, so a missed row fails loudly and immediately.
- Expected sub-agent dispatches:
  - Question: report `ArachnePerimeters::emit_only_one_wall_top_second_pass`'s gate sequence and `ex_polygon_min_width_mm` verbatim; scope: `modules/core-modules/arachne-perimeters/src/lib.rs`; return: `SNIPPETS` (≤ 25 lines)
  - Question: confirm `split_top_surfaces`' exact signature, its `TopSurfaceSplit` fields, and every call site of it inside `classic-perimeters`; scope: `crates/slicer-core/src/top_surface_split.rs`, `modules/core-modules/classic-perimeters/src/lib.rs`; return: `LOCATIONS` (≤ 10 entries)
- Context cost: `S`
- Authoritative docs:
  - `docs/15_config_keys_reference.md` — delegated `rg` on `min_width_top_surface` only; the doc edit itself is Step 8.
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/PerimeterGenerator.cpp` — `split_top_surfaces`; delegate, never load.
- Verification:
  - `bash -c 'cargo test -p slicer-runtime --test integration -- classic_min_width_top_surface_tdd 2>&1 | rg "^test result" | rg -q "^test result: ok\. [1-9]" || echo "FAIL: 0 tests ran or gate not wired"'` — FACT pass/fail (AC-9).
  - `bash -c 'F=modules/core-modules/classic-perimeters/src/lib.rs; rg -q "get_abs_value\(\"min_width_top_surface\"" $F && ! rg -q "let _ = min_width_top_surface;" $F && rg -q "only_one_wall_top" $F && rg -q "min_width_top_surface" $F && echo PASS || echo FAIL'` — FACT PASS/FAIL (AC-7).
  - `bash -c 'M=modules/core-modules/classic-perimeters/classic-perimeters.toml; rg -U -q "\[config\.schema\.min_width_top_surface\][^\[]*type\s*=\s*\"float_or_percent\"" $M && rg -U -q "\[config\.schema\.min_width_top_surface\][^\[]*default\s*=\s*\"0\.0\"" $M && cargo test -p slicer-runtime --test integration -- manifest_default_reconcile_tdd 2>&1 | rg "^test result" | rg -q "^test result: ok\. [1-9]" || echo "FAIL: manifest not retyped or reconcile red"'` — FACT pass/fail (AC-8).
  - `bash -c 'cargo test -p slicer-runtime --test arachne_parity 2>&1 | rg "^test result" | rg -q "^test result: ok\. [1-9]" || echo "FAIL: parity-literal regression"'` — FACT pass/fail.
  - `bash -c 'git diff --numstat crates/slicer-runtime/tests/fixtures/golden/precision_legacy_20mmbox.gcode | rg -q "." && echo "FAIL: golden moved again — gate is firing at the 0.0 default" || echo PASS'` — FACT PASS/FAIL, run **after** committing or stashing Step 5's re-bless so this compares against the post-Step-5 baseline.
  - `bash -c 'cargo xtask build-guests --check 2>&1 | rg -c "STALE:" || echo "0 stale"'` — FACT: rebuild without `--check` if non-zero.
- Exit condition: AC-7, AC-8, AC-9 PASS; `arachne_parity` green; the golden has not moved since Step 5; guests fresh.

### Step 8: Docs, deviation rows, and the residual-gap filing

- Task IDs: `TASK-303`
- Objective: (a) Run `cargo xtask gen-config-docs` to regenerate `docs/15_config_keys_reference.md`'s `module-config-keys` block for the three retyped keys — **never hand-edit inside a `BEGIN GENERATED` marker.** (b) Hand-correct the narrative *outside* every marker: the per-key `min_width_top_surface` section must stop asserting that `classic-perimeters` registers the key as a fixed mm float and must record that both perimeter modules now agree on `float_or_percent` / `"0.0"`; the wall-width narrative must record the `float_or_percent` retype, the `0` → `1.125 × nozzle_diameter` auto resolution, and the ingestion residual. (c) Flip the classic halves of `D-105-FLOW-NOT-WIRED` and `D-152-CLASSIC-MIN-WIDTH-TOP-SURFACE-REMAINDER` to `Closed` in `docs/DEVIATION_LOG.md`, editing only those rows. **`D-164-WALL-WIDTH-KEYS-NOT-FLOAT-OR-PERCENT` is a PARTIAL closure — its Status cell must begin `Closed (partial)` and name the surviving residual**, because `[FWD-1]` deliberately keeps `default = 0.4` and an absent-key fallback of `0.4` rather than adopting canonical's auto, so the row's central claim (canonical registers both keys as `coFloatOrPercent` default `0` = auto) is still true after this packet. Word it: what closed (retype to `float_or_percent`, `min` lowered to `0.0`, the `0` → `1.125 × nozzle_diameter` auto sentinel honoured) and what did not (PnP's `0.4` default and absent-key fallback), cross-referencing `[FWD-1]` and the shared residual row that names `parse_percent_default`. A bare `Closed` here would misreport the ledger. (d) **Cross-reference the shared residual row — do NOT file a second one.** The two residuals are (i) PnP resolves an *absent* wall-width key to `0.4` mm while canonical resolves it to auto `1.125 × nozzle_diameter`, and (ii) no live slice can carry a `Percent`/`FloatOrPercent` end-to-end because **no live-path producer of one exists** (the barrier is the config **parser**: `parse_percent_default`, `crates/slicer-scheduler/src/manifest.rs`, is the only non-test origin of either variant and both of its `parse_config_field_entry` call sites discard its return value — do **not** blame `ResolvedConfig::to_config_map`, whose `extensions` pass-through is a transparent channel that already carries any variant; see `design.md` §Architecture Constraints). **Both residuals are ONE decision taken identically by this packet and by packet `185-arachne-width-bridge-parity`, and by maintainer ruling they get exactly ONE shared row — 185's.** 185 is the amender of record for ADR-0043 (it files `D-185-ADR-0043-AMENDED`) and lands first by queue order, and its Doc Impact already instructs it to word the row to cover **both** the arachne and the classic halves. This packet therefore **greps for that row and cross-references it**, adding the classic-side evidence to it if the wording does not already cover classic. Two packets independently deriving "the next free `DEV-###`" for the same finding is exactly how a duplicated row lands. Locate it with `rg -n 'parse_percent_default' docs/DEVIATION_LOG.md`. **If — and only if — that row does not exist when this step runs** (i.e. 185 has not landed), file it here instead, worded for both modules, and derive the ID at the moment of writing with `rg -o '^\| DEV-[0-9]{3}' docs/DEVIATION_LOG.md | sort -u | tail -1`, taking the next free number — never an ID quoted anywhere in this packet. Record in the step's completion note which branch was taken. **Do not edit packet 185's files under either branch.** There is **no new ADR** for this divergence: it stays carried by the open `D-164` row plus that single shared residual row. (e) Hand-add the `TASK-303` backlog row to `docs/07_implementation_status.md` (task rows sit well outside the `<!-- BEGIN GENERATED: open-deviations -->` block), then run `cargo xtask check-deviations` to regenerate that block.
- Precondition: Steps 1-7 complete; all code and test ACs PASS.
- Postcondition: `cargo xtask gen-config-docs --check` and `cargo xtask check-deviations --check` both exit `0`; `D-105` and `D-152` read `Closed` and `D-164` reads `Closed (partial)` with its residual named; **exactly one** shared residual row naming `parse_percent_default` exists (filed by 185, or by this packet only if 185 has not landed); `TASK-303` is in `docs/07_implementation_status.md`.
- Files allowed to read, with ranges when over 300 lines:
  - `docs/DEVIATION_LOG.md` — very large; **only** the three rows, located by `rg` on the deviation ID. Never load the file; never cite it by line number.
  - `docs/15_config_keys_reference.md` — large; only the `min_width_top_surface` and wall-width narrative sections plus the `BEGIN/END GENERATED` marker lines, located by `rg`.
  - `docs/07_implementation_status.md` — large; only the `TASK-###` neighbourhood, located by `rg`.
- Files allowed to edit (at most 3):
  - `docs/DEVIATION_LOG.md`
  - `docs/15_config_keys_reference.md` (hand edits **outside** the generated markers; the generated blocks are rewritten by `cargo xtask gen-config-docs`, not by hand)
  - `docs/07_implementation_status.md` (the `TASK-303` row by hand; the open-deviations block by `cargo xtask check-deviations`)
- Files explicitly out of bounds:
  - `docs/specs/deviation-backlog-remediation-plan.md` — the batch orchestrator owns the Packet Queue.
  - `docs/14_deviation_audit_history.md` — a generated, non-authoritative view.
  - All code and test files — frozen after Step 7.
  - Any region of `docs/15_config_keys_reference.md` or `docs/07_implementation_status.md` inside a `BEGIN GENERATED` marker.
- Blast-radius discipline: not applicable — no struct field, no schema/version constant. The doc blast radius is enumerated above and enforced by the two `--check` gates.
- Expected sub-agent dispatches:
  - Question: report the current `Status` cell verbatim for the three deviation IDs, and the highest `DEV-###` id currently present; scope: `docs/DEVIATION_LOG.md`; return: `FACT` (≤ 10 lines). **Re-derive the highest id at this moment; treat any id quoted in this packet as stale.**
  - Question: which `docs/15_config_keys_reference.md` lines mentioning `min_width_top_surface` or the two wall-width keys fall **outside** every `BEGIN/END GENERATED` marker pair?; scope: `docs/15_config_keys_reference.md`; return: `LOCATIONS` (≤ 20 entries)
- Context cost: `S`
- Authoritative docs:
  - `docs/15_config_keys_reference.md` — this step's subject; ranged reads only.
  - `docs/DEVIATION_LOG.md` — this step's subject; row-scoped reads only.
- OrcaSlicer refs:
  - None — this step edits documentation.
- Verification:
  - `bash -c 'cargo xtask gen-config-docs --check >/dev/null 2>&1 && ! rg -q "classic-perimeters. still registers this as a fixed mm float" docs/15_config_keys_reference.md && echo PASS || echo FAIL'` — FACT PASS/FAIL (AC-12).
  - `bash -c 'rg -q "1\.125" docs/15_config_keys_reference.md && echo PASS || echo FAIL'` — FACT PASS/FAIL: the auto-resolution constant is documented.
  - `bash -c 'C=$(rg -c "parse_percent_default" docs/DEVIATION_LOG.md 2>/dev/null || echo 0); [ "$C" = "1" ] && echo "PASS (exactly one shared residual row)" || echo "FAIL: expected exactly 1 DEVIATION_LOG row naming parse_percent_default, found $C — 0 means neither 184 nor 185 filed it; >1 means 184 duplicated 185 row"'` — FACT PASS/FAIL: **exactly one** residual row exists, naming the actual blocking symbol (the parser, not `to_config_map`). The count is the point: `>1` is the duplicate-filing failure this step exists to prevent.
  - `bash -c 'rg -q "1\.125" docs/DEVIATION_LOG.md && echo PASS || echo "FAIL: the shared residual row does not record the canonical auto constant"'` — FACT PASS/FAIL: the row covers residual (i) as well as (ii).
  - `bash -c 'rg -q "TASK-303" docs/07_implementation_status.md && cargo xtask check-deviations --check >/dev/null 2>&1 && echo PASS || echo FAIL'` — FACT PASS/FAIL.
- Exit condition: AC-12 PASS, both xtask `--check` gates exit `0`, `D-105`/`D-152` read `Closed` and `D-164` reads `Closed (partial)` naming its surviving residual, exactly one shared residual row naming `parse_percent_default` exists (cross-referenced, not duplicated), the branch taken is recorded in the completion note, and `TASK-303` is present.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | S | One new test file + one `mod` line; one ranged read of the R2 block. |
| Step 2 | S | Two manifest blocks, one config-read block, one comment; two bounded dispatches. |
| Step 3 | S | One short test file rewritten to derived expectations + one negative test. |
| Step 4 | M | Four canonical formulas inside `emit_walls`, two symbol-located windows, no signature change across five call sites. |
| Step 5 | S | One golden re-bless, diff-inspected; two two-sided invariants re-verified. |
| Step 6 | S | One new test file + one `mod` line; two ranged reads. |
| Step 7 | S | One call-site gate mirroring a known-good arachne shape; one manifest block; one table row. |
| Step 8 | S | Two xtask regenerations plus bounded hand edits in three docs. |

Aggregate: `M`. Split before activation if aggregate cost exceeds M or any step is L.

## Packet Completion Gate

- All steps and exits complete.
- Every pipe-suffixed AC command returns PASS with a non-zero test count.
- `cargo check --workspace --all-targets` and `cargo clippy --workspace --all-targets -- -D warnings` are clean.
- `cargo build --workspace` succeeds and `cargo xtask build-guests --check` reports clean — both are preconditions for trusting AC-10 and AC-11, which run the real `pnp_cli` against the real core-module guests.
- `cargo xtask gen-config-docs --check` and `cargo xtask check-deviations --check` both exit `0`.
- `D-105` and `D-152` read `Closed — <date> (packet 184)` with a one-line statement of what closed them. `D-164` reads `Closed (partial) — <date> (packet 184)` and names the surviving residual (`[FWD-1]`: PnP keeps `default = 0.4` and an absent-key fallback of `0.4`, so canonical's `coFloatOrPercent` default-`0` auto registration is still a live divergence). **Exactly one** shared residual row naming `parse_percent_default` exists across packets 184 and 185 — cross-referenced by this packet, filed by it only if 185 has not landed, and never duplicated. **No new ADR is authored for the wall-width divergence**; it is carried by the open/partial `D-164` row plus that single shared residual row.
- `docs/07_implementation_status.md` carries the `TASK-303` row (hand-added, outside the generated block) and a freshly regenerated open-deviations block.
- **Completion note — one authorized out-of-bounds test edit (2026-07-31).** `crates/slicer-runtime/tests/integration/gap_fill_emission_tdd.rs` is on `design.md`'s out-of-bounds list ("must stay green unmodified"), and it **was modified**, with maintainer authorization. Correcting the gap-fill `min` bound to canonical `INSET_OVERLAP_TOLERANCE = 0.4` exposed a **pre-existing defect in the test**, not a production regression: `gap_fill_emitted_for_narrow_gap` asserted that *every individual segment* of the medial axis is at least `0.5` mm, whereas the production filter in `ClassicPerimeters`, the test's own AC-4 contract, and the test's own doc comment are all about the **total polyline length**. The `0.5` literal carried no OrcaSlicer citation, and canonical guarantees nothing about per-segment length. The assertion was changed to a total-polyline-length assertion; the **threshold is unchanged at `0.5` mm** and **no other assertion was touched**. This is a correction to match the contract the test always claimed, not a weakening — recorded here and in `design.md` §Out-of-Bounds Files so it cannot be mistaken for one.
- **Completion note — the inset overlap tolerance is `0.4`, not two tenths.** This packet's `requirements.md`, `design.md`, and this plan originally all instructed the smaller, wrong literal. That was wrong; the constant is declared exactly once, as `static constexpr double INSET_OVERLAP_TOLERANCE = 0.4;` in OrcaSlicer's `libslic3r/libslic3r.h` (verified twice by independent dispatch). The implementation shipped canonical `0.4` and all three packet docs have been corrected. Root cause: a pre-existing false comment in `modules/core-modules/classic-perimeters/src/lib.rs` that asserted the wrong value and claimed no matching const existed upstream; that comment is now corrected in code.
- No reopened/superseded packet transitions apply.
- Re-run AC-11 (`arachne_structural_invariants`) **after** the arachne T2 sibling packet lands, or record that it has not yet landed. That test is two-sided and both packets move it.
- `packet.spec.md` is ready for `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed AC and packet-level gate command.
- Record remaining packet-local risk: (1) the absent-key wall-width default remains PnP's `0.4` rather than canonical's auto `1.125 × nozzle_diameter` (`[FWD-1]`, filed as a residual deviation); (2) percent-valued config still cannot round-trip end-to-end because config ingestion has no producer of a `Percent`/`FloatOrPercent` — the parser validates and discards it (filed in the same residual row; `ResolvedConfig::to_config_map`'s `extensions` pass-through is *not* the barrier); (3) canonical's `ext_perimeter_spacing / 2` floor on `min_width_top_surface` is deliberately not ported (`[FWD-2]`); (4) `emit_nonplanar_shells` still consumes raw widths rather than spacings (`[FWD-3]`).
- Confirm context stayed at or below 150k standard, or at/below 300k only with a logged swarm ESCALATION; otherwise record a packet-authoring lesson.

All `cargo check`, `cargo clippy`, and `cargo test` invocations in **gate** commands use `--all-targets` so the test, bench, and example targets compile. This does **not** apply to the narrow `cargo test -p <crate> --test <binary>` verification commands above — `--all-targets` is not a valid combination with `--test`, and the narrow form is deliberate per `CLAUDE.md` §Test Discipline.
