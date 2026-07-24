# Implementation Plan: 182-gcode-header-width-defaults

## Execution Rules

- Work one atomic step at a time; map every step to grouped task IDs.
- Use TDD, then implementation, then the narrowest falsifying validation.
- Every field below is a context-budget contract and must be filled independently; never write "see Step 1".
- **Whole-line assertion rule:** `; outer_wall_line_width = 0.4` is a strict prefix of `; outer_wall_line_width = 0.42`, so any `contains()` check on the positive form passes on the unfixed tree. Assert complete lines (`output.lines().any(|l| l.trim() == "; outer_wall_line_width = 0.4")`). Likewise, negative assertions must be keyed to the field — `sparse_infill_line_width = 0.45` and `top_surface_line_width = 0.42` legitimately remain in the header.
- **Test-filter rule:** `--test e2e` is an aggregated mod-list binary, so libtest names are module-qualified; use substring filters, never `--exact` on a bare fn name. `--test golden_emit_tdd` is a standalone binary where a bare filter is unambiguous.
- **RED vs GREEN guard rule.** The `rg "^test result" | rg -v "0 passed"` guard is correct only for **GREEN** gates. It is wrong for a **RED** step: a genuine failure prints `test result: FAILED. 0 passed; 1 failed; …`, which that guard filters out, producing the identical `FAIL: 0 tests ran` message as a filter that matched nothing — so it cannot distinguish "failed for the right reason" from "matched no tests". For RED steps, print the result line unfiltered and require it to contain `1 failed`.

## Steps

### Step 1: Add the failing whole-line assertion for the emitted header widths

- Task IDs: `TASK-295`
- Objective: Author `header_reports_governing_wall_width_defaults` in the existing `golden_emit_tdd` binary, asserting the emitted header contains the exact whole lines `; outer_wall_line_width = 0.4` and `; inner_wall_line_width = 0.4`, and contains neither the whole line `; outer_wall_line_width = 0.42` nor `; inner_wall_line_width = 0.45`. Must FAIL on the current tree (RED).
- Precondition: `crates/slicer-gcode/src/serialize.rs` still assigns `0.42`/`0.45` in `DefaultGCodeSerializer::with_extrusion_mode`.
- Postcondition: the new test exists and fails, reporting the actual emitted `0.42`/`0.45`; the pre-existing `serialize_gcode_emits_documented_sentinels` still passes untouched.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-gcode/tests/golden_emit_tdd.rs` — full (148 lines); reuse its existing serializer construction and minimal-plan setup.
  - `crates/slicer-gcode/src/serialize.rs` — lines `274-301` only (`serialize_width_comments`, for the exact emitted string format).
- Files allowed to edit (at most 3):
  - `crates/slicer-gcode/tests/golden_emit_tdd.rs`
- Files explicitly out of bounds:
  - `crates/slicer-gcode/src/serialize.rs` (no production edit in this step — the test must go red against unmodified source)
  - `crates/slicer-runtime/**`, `modules/core-modules/**`, `OrcaSlicerDocumented/**`
- Blast-radius discipline: not applicable — this step adds no struct field and bumps no schema/version constant.
- Expected sub-agent dispatches:
  - Question: run the new test and report the `test result` line plus the observed emitted values; scope: `cargo test -p slicer-gcode --test golden_emit_tdd -- header_reports_governing_wall_width_defaults`; return: `FACT` (<=5 lines)
- Context cost: `S`
- Authoritative docs:
  - None required for this step.
- OrcaSlicer refs:
  - None — this packet ports no canonical behavior.
- Verification:
  - `bash -c 'cargo test -p slicer-gcode --test golden_emit_tdd -- header_reports_governing_wall_width_defaults --nocapture 2>&1 | rg "^test result|panicked at|assertion"'` — FACT: the `test result:` line must contain **`1 failed`** (a RED for the right reason). A line reading `0 passed; 0 failed` means the filter matched nothing and is itself a failure of this step. Do **not** apply the `rg -v "0 passed"` guard here — see the RED vs GREEN guard rule above.
  - `bash -c 'cargo test -p slicer-gcode --test golden_emit_tdd -- serialize_gcode_emits_documented_sentinels 2>&1 | rg "^test result"'` — FACT: must still pass.
- Exit condition: the result line shows `1 failed`, the panic message quotes the observed `0.42`/`0.45`, and the test asserts whole lines rather than substrings.

### Step 2: Correct the defaults and the three misleading comments

- Task IDs: `TASK-295`
- Objective: In `DefaultGCodeSerializer::with_extrusion_mode`, change `outer_wall_line_width: 0.42` → `0.4` and `inner_wall_line_width: 0.45` → `0.4`; replace the constructor comment citing the deleted `config_schema.rs`; update the two per-field doc comments. Attribute `0.4` to `resolve_line_width_mm`, **not** to "OrcaSlicer parity" — upstream registers both keys as `coFloatOrPercent` default `0` (see open deviation `D-164`), so re-using the parity framing would plant a fresh false citation. Turns Step 1's test GREEN.
- Precondition: Step 1's test exists and is RED.
- Postcondition: both literals are `0.4`; no `config_schema` string remains in `serialize.rs`; Step 1's test passes.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-gcode/src/serialize.rs` (long; never read in full) — the `DefaultGCodeSerializer` struct declaration + `with_extrusion_mode` only (~lines `66-116`; locate by symbol, the pin is a hint).
  - `crates/slicer-runtime/src/builtins/overhang_annotation_producer.rs` — `resolve_line_width_mm` only, via the dispatch below; do not open directly.
- Files allowed to edit (at most 3):
  - `crates/slicer-gcode/src/serialize.rs`
- Files explicitly out of bounds:
  - `crates/slicer-gcode/tests/golden_emit_tdd.rs` (frozen after Step 1 — do not weaken the assertion to obtain a pass)
  - `crates/slicer-runtime/src/run.rs`, `crates/slicer-runtime/src/pipeline.rs`
  - The sibling fields `sparse_infill_line_width`, `top_surface_line_width`, `support_line_width` — leave their values unchanged
- Blast-radius discipline: no struct field is added and no constant is bumped, and `DefaultGCodeSerializer` has no struct-literal sites outside its constructors. **But the emitted header is captured byte-for-byte in a golden fixture** — that fallout is real and is owned by Step 3, which must not be skipped on the grounds that there are no struct literals.
- Expected sub-agent dispatches:
  - Question: what is `resolve_line_width_mm`'s exact fallback expression and doc-comment wording?; scope: `crates/slicer-runtime/src/builtins/overhang_annotation_producer.rs`; return: `SNIPPETS` (<=15 lines)
- Context cost: `S`
- Authoritative docs:
  - None required for this step.
- OrcaSlicer refs:
  - None — this packet ports no canonical behavior.
- Verification:
  - `bash -c 'cargo test -p slicer-gcode --test golden_emit_tdd -- header_reports_governing_wall_width_defaults 2>&1 | rg "^test result" | rg -v "0 passed" || echo "FAIL: 0 tests ran"'` — FACT pass/fail (AC-1).
  - `bash -c 'rg -q "outer_wall_line_width: 0\.4," crates/slicer-gcode/src/serialize.rs && rg -q "inner_wall_line_width: 0\.4," crates/slicer-gcode/src/serialize.rs && ! rg -q "config_schema" crates/slicer-gcode/src/serialize.rs && echo PASS || echo FAIL'` — FACT PASS/FAIL (AC-2).
  - `bash -c 'cargo test -p slicer-gcode --test golden_emit_tdd 2>&1 | rg "^test result"'` — FACT pass/fail for the whole binary.
- Exit condition: AC-1 and AC-2 both PASS, the `golden_emit_tdd` binary is green, and no comment attributes `0.4` to OrcaSlicer parity.

### Step 3: Re-bless the byte-identity golden

- Task IDs: `TASK-295`
- Objective: Re-bless `crates/slicer-runtime/tests/fixtures/golden/precision_legacy_20mmbox.gcode`, which records `; outer_wall_line_width = 0.42` and `; inner_wall_line_width = 0.45` and is asserted byte-for-byte by `legacy_zero_matches_golden`. Confirm by diff that **only** those two lines changed.
- Precondition: Step 2 complete; `legacy_zero_matches_golden` is now RED. **`cargo build --workspace` has been run first.** This test does not exercise the library in-process — it shells out to a `pnp_cli` executable located on disk (`legacy_zero_matches_golden` → `run_pnp_cli_uncached` → `pnp_cli_bin()`), and `crates/slicer-runtime` has no dependency on the CLI crate, so `cargo test -p slicer-runtime --test e2e` will **not** rebuild `pnp_cli` after Step 2 edited `serialize.rs`. Re-blessing against a stale binary would re-record `0.42`/`0.45` and produce a green test that still encodes the bug. (`cargo check --workspace --all-targets` does not help — it builds no executable.)
- Postcondition: the golden's two wall-width lines read `0.4`, the test is green, and the diff shows no other change.
- Bless command (run after `cargo build --workspace`): `BLESS_GOLDEN=1 cargo test -p slicer-runtime --test e2e -- legacy_zero_matches_golden --nocapture`
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-runtime/tests/e2e/slicing_precision_integration_tdd.rs` — the `legacy_zero_matches_golden` body only, to learn the comparison/bless mechanism. **The `BLESS_GOLDEN=1 … --test slicing_precision_integration_tdd` hint inside that file is stale and names a non-existent cargo target — the real binary is `--test e2e`.**
- Files allowed to edit (at most 3):
  - `crates/slicer-runtime/tests/fixtures/golden/precision_legacy_20mmbox.gcode`
- Files explicitly out of bounds:
  - Every other fixture under `crates/slicer-runtime/tests/fixtures/golden/` — do not bulk re-bless.
  - `crates/slicer-runtime/tests/e2e/slicing_precision_integration_tdd.rs` — the test asserts byte identity; re-bless the data, never relax the assertion.
  - `crates/slicer-gcode/**` (frozen after Step 2).
- Blast-radius discipline: this step IS the recorded-output blast radius Step 2 defers to it. Before editing, confirm via the dispatch below that exactly one golden records these lines.
- Expected sub-agent dispatches:
  - Question: which files under `crates/slicer-runtime/tests/fixtures/golden/` contain a `; outer_wall_line_width =` or `; inner_wall_line_width =` line, and with what value?; scope: `crates/slicer-runtime/tests/fixtures/golden/**`; return: `LOCATIONS` (<=20 entries)
  - Question: does `cargo build --workspace` succeed, and does `cargo xtask build-guests --check` report clean?; scope: repo root; return: `FACT` (<=5 lines); purpose: rule out a stale `pnp_cli` **and** a stale guest before re-blessing or attributing any failure. This test drives the real core-module guests via `pnp_cli --module-dir`, so guest staleness can fail it for reasons unrelated to this change.
- Context cost: `S`
- Authoritative docs:
  - None required for this step.
- OrcaSlicer refs:
  - None — this packet ports no canonical behavior.
- Verification:
  - `bash -c 'rg -q "^; outer_wall_line_width = 0\.4$" crates/slicer-runtime/tests/fixtures/golden/precision_legacy_20mmbox.gcode && rg -q "^; inner_wall_line_width = 0\.4$" crates/slicer-runtime/tests/fixtures/golden/precision_legacy_20mmbox.gcode && cargo test -p slicer-runtime --test e2e -- legacy_zero_matches_golden 2>&1 | rg "^test result" | rg -v "0 passed" || echo "FAIL: golden not re-blessed or 0 tests ran"'` — FACT pass/fail (AC-3).
  - `bash -c 'git diff --numstat crates/slicer-runtime/tests/fixtures/golden/precision_legacy_20mmbox.gcode'` — FACT: must show exactly 2 insertions and 2 deletions.
- Exit condition: AC-3 passes and the golden's diff is exactly the two wall-width lines.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | S | One test file (148 lines) + one 28-line ranged read. |
| Step 2 | S | Two literals and three comments inside one ~50-line range; one bounded dispatch. |
| Step 3 | S | One fixture re-bless, diff-verified to two lines. |

Split before activation if aggregate cost exceeds M or any step is L.

## Packet Completion Gate

- All steps and exits complete.
- Every pipe-suffixed AC command returns PASS with a non-zero test count.
- `cargo check --workspace --all-targets` and `cargo clippy --workspace --all-targets -- -D warnings` are clean.
- `cargo build --workspace` succeeds and `cargo xtask build-guests --check` reports clean — both are preconditions for trusting AC-3, which runs the real `pnp_cli` against the real core-module guests.
- Flip the `D-165-GCODE-HEADER-WIDTH-DEFAULTS-LIE` row in `docs/DEVIATION_LOG.md` to `Closed — <date> (packet 182): header defaults corrected to the governing 0.4/0.4 fallback; dangling config_schema.rs citation removed; precision_legacy_20mmbox golden re-blessed`, via a worker dispatch editing only that row.
- **Then run `cargo xtask check-deviations`** to regenerate the open-deviations block in `docs/07_implementation_status.md`. That block sits inside `<!-- BEGIN GENERATED: open-deviations -->` and must **not** be hand-edited — the file's own note says CI fails if it drifts from the log.
- Separately, hand-add the `TASK-295` backlog row to `docs/07_implementation_status.md` (the task rows sit well outside the generated block, near the other `TASK-###` entries). `TASK-295` does not exist there yet — the highest present is `TASK-294`. This is the one legitimate hand-edit to that file.
- No reopened/superseded packet transitions apply to this packet.
- `packet.spec.md` is ready for `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed AC and packet-level gate command.
- Record remaining packet-local risk: the header still reports a compile-time default rather than the slice's resolved widths (recorded in `requirements.md` §Out of Scope, not closed by this packet).
- Confirm context stayed at or below 150k standard, or at/below 300k only with a logged swarm ESCALATION; otherwise record a packet-authoring lesson.

The workspace-wide `cargo check` and `cargo clippy` gate commands must use `--all-targets` so the test, bench, and example targets compile. This does **not** apply to the narrow `cargo test -p <crate> --test <binary>` verification commands above — `--all-targets` is not a valid combination with `--test`, and the narrow form is deliberate per `CLAUDE.md` §Test Discipline.
