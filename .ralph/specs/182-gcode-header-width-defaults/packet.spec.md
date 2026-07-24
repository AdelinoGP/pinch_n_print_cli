---
status: draft
packet: 182-gcode-header-width-defaults
task_ids:
  - TASK-295
backlog_source: docs/07_implementation_status.md
context_cost_estimate: S
---

# Packet Contract: 182-gcode-header-width-defaults

## Goal

Correct the `DefaultGCodeSerializer` header-comment wall-width defaults from the non-governing `0.42`/`0.45` to the governing `0.4`/`0.4` fallback, delete the dangling `config_schema.rs` citation, and re-bless the one golden fixture that captured the old values — so the emitted `; outer_wall_line_width` / `; inner_wall_line_width` header lines report the value the pipeline actually falls back to (`resolve_line_width_mm` → `0.4`). Closes deviation D-165-GCODE-HEADER-WIDTH-DEFAULTS-LIE.

## Scope Boundaries

Touches only `outer_wall_line_width` and `inner_wall_line_width` (their constructor defaults and doc comments) in `crates/slicer-gcode/src/serialize.rs`, a new value-asserting test in the existing `golden_emit_tdd` binary, and the byte-identical golden `crates/slicer-runtime/tests/fixtures/golden/precision_legacy_20mmbox.gcode` which records those two lines. The sibling header fields (`sparse_infill_line_width`, `top_surface_line_width`, `support_line_width`) and any config-driven wiring are out of scope — this packet fixes a wrong hard-coded default, it does not introduce config plumbing.

## Prerequisites and Blockers

- Depends on: none.
- Unblocks: none (weak agreement: the T2 flow packets retype these same config keys to float-or-percent with an auto `0`→nozzle default and must preserve `0.4` as the resolved fallback so this header stays truthful).
- Activation blockers: none. Packet `140_lightning-module-rewrite` is currently `active`; this packet stays `draft` until that clears.

## Acceptance Criteria

Assertions below are **whole-line and per-key**. `; outer_wall_line_width = 0.4` is a strict prefix of `; outer_wall_line_width = 0.42`, so a `contains()` check on the positive form passes on the *unfixed* tree; and `sparse_infill_line_width = 0.45` / `top_surface_line_width = 0.42` remain in the same header, so an unqualified `!contains("= 0.45")` fails *after* the fix. Both traps are avoided by matching complete lines keyed to the two fields.

- **AC-1. Given** a `DefaultGCodeSerializer` built via `DefaultGCodeSerializer::default()` with no width overrides — matching the construction the existing `golden_emit_tdd.rs` tests already use, so Step 1 can reuse that setup — **when** a minimal plan is serialized, **then** the output contains the exact whole line `; outer_wall_line_width = 0.4` and the exact whole line `; inner_wall_line_width = 0.4`, and contains neither the line `; outer_wall_line_width = 0.42` nor the line `; inner_wall_line_width = 0.45`. | `bash -c 'cargo test -p slicer-gcode --test golden_emit_tdd -- header_reports_governing_wall_width_defaults 2>&1 | rg "^test result" | rg -v "0 passed" || echo "FAIL: 0 tests ran"'`
- **AC-2. Given** `crates/slicer-gcode/src/serialize.rs`, **when** grepped, **then** the `outer_wall_line_width`/`inner_wall_line_width` constructor defaults are `0.4`, no `0.42`/`0.45` literal remains on those two fields, and no `config_schema` citation remains anywhere in the file. | `bash -c 'rg -q "outer_wall_line_width: 0\.4," crates/slicer-gcode/src/serialize.rs && rg -q "inner_wall_line_width: 0\.4," crates/slicer-gcode/src/serialize.rs && ! rg -q "config_schema" crates/slicer-gcode/src/serialize.rs && echo PASS || echo FAIL'`
- **AC-3. Given** the byte-identity golden `crates/slicer-runtime/tests/fixtures/golden/precision_legacy_20mmbox.gcode`, **when** `legacy_zero_matches_golden` runs, **then** the golden's header carries the whole lines `; outer_wall_line_width = 0.4` and `; inner_wall_line_width = 0.4` and the test passes. | `bash -c 'rg -q "^; outer_wall_line_width = 0\.4$" crates/slicer-runtime/tests/fixtures/golden/precision_legacy_20mmbox.gcode && rg -q "^; inner_wall_line_width = 0\.4$" crates/slicer-runtime/tests/fixtures/golden/precision_legacy_20mmbox.gcode && cargo test -p slicer-runtime --test e2e -- legacy_zero_matches_golden 2>&1 | rg "^test result" | rg -v "0 passed" || echo "FAIL: golden not re-blessed or 0 tests ran"'`

## Verification

- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `bash -c 'cargo test -p slicer-runtime --test e2e -- legacy_zero_matches_golden 2>&1 | rg "^test result"'`

## Authoritative Docs

- `docs/15_config_keys_reference.md` — delegated grep only; verified to contain no `0.42`/`0.45` literal (it already documents both keys at default `0.4`), so no doc edit is required.
- `docs/DEVIATION_LOG.md` — the D-165 row only; flipped to `Closed` at the completion gate.

## Doc Impact Statement (Required)

- **`none`** — this changes a code default value in the G-code serializer and re-blesses one recorded golden; it touches no IR, WIT, scheduler, claim, manifest, host-service, or SDK contract. `docs/15_config_keys_reference.md` documents config-key schemas, not the serializer's internal header fallback, and contains no `0.42`/`0.45` literal (verified: `rg -q '0\.42|0\.45' docs/15_config_keys_reference.md` returns no match). The D-165 status flip in `docs/DEVIATION_LOG.md` plus the `cargo xtask check-deviations` regeneration of `docs/07_implementation_status.md` are handled at the completion gate as status bookkeeping, not contract-doc edits.

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- obey the shared absolute context bands: 120k reading budget with hand-off at 150k (standard); the extended band (240k reading / 300k hard stop) only via swarm's escalation protocol

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation (an extended-band run may carry a single L step only when `design.md` justifies why it cannot be split).
