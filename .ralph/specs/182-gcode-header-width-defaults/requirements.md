# Requirements: 182-gcode-header-width-defaults

## Packet Metadata

- Grouped task IDs: `TASK-295`
- Backlog source: `docs/07_implementation_status.md`
- Packet status: `draft`
- Aggregate context cost: `S`

## Problem Statement

Deviation **G-code header width-default gap** (Open, filed 2026-07-16 during the D-160 session). `DefaultGCodeSerializer` (`crates/slicer-gcode/src/serialize.rs`) hard-codes `outer_wall_line_width: 0.42` and `inner_wall_line_width: 0.45` in its `with_extrusion_mode` constructor, and `serialize_width_comments` prints those values verbatim into every G-code header as `; outer_wall_line_width = 0.42` / `; inner_wall_line_width = 0.45`.

Two things make this a lie rather than a stale constant:

1. **The cited authority does not exist.** The constructor comment reads "matches `config_schema.rs` registration"; `config_schema.rs` was deleted from the tree and registers nothing. The per-field doc comments repeat the same wrong values.
2. **The governing fallback is `0.4`, not `0.42`/`0.45`.** Two independent code paths establish it: `resolve_line_width_mm` (`crates/slicer-runtime/src/builtins/overhang_annotation_producer.rs`) falls back to `0.4`, and the `classic-perimeters` guest resolves `legacy_line_width` to `0.4` when config omits the keys, yielding outer = inner = `0.4`.

There is **no config-driven setter** for these two serializer fields — the only constructors (`new`, `with_extrusion_mode`, `with_flavor`, `with_filament_config`) never touch line widths — so the header emits the hard-coded values regardless of the slice's actual configuration. The header therefore misreports the width for every print. This is one coherent slice: a wrong constant plus its dangling citation, in one file.

## In Scope

- Change the `outer_wall_line_width` default in `DefaultGCodeSerializer::with_extrusion_mode` from `0.42` to `0.4`.
- Change the `inner_wall_line_width` default in `DefaultGCodeSerializer::with_extrusion_mode` from `0.45` to `0.4`.
- Correct the two per-field doc comments on `outer_wall_line_width` / `inner_wall_line_width` that state the old `0.42` / `0.45` parity defaults.
- Delete the dangling `config_schema.rs` citation from the constructor comment and replace it with the real governing authority (`resolve_line_width_mm`'s `0.4` fallback).
- Add a **whole-line** value-asserting test to `crates/slicer-gcode/tests/golden_emit_tdd.rs` proving the emitted header reports `0.4` for both keys. Whole-line, because `= 0.4` is a strict prefix of `= 0.42` and a `contains()` check would pass on the unfixed tree.
- Re-bless `crates/slicer-runtime/tests/fixtures/golden/precision_legacy_20mmbox.gcode`, which records `; outer_wall_line_width = 0.42` and `; inner_wall_line_width = 0.45` and is asserted **byte-for-byte** by `legacy_zero_matches_golden` (`crates/slicer-runtime/tests/e2e/slicing_precision_integration_tdd.rs`). Changing the literals turns that test RED, so the re-bless belongs to this packet.
- Flip the `G-code header width-default gap` row in `docs/DEVIATION_LOG.md` to `Closed` at the completion gate, then regenerate the open-deviations block in `docs/07_implementation_status.md` with `cargo xtask check-deviations` — that block is machine-generated and must not be hand-edited.

## Out of Scope

- The sibling header fields `sparse_infill_line_width` (`0.45`), `top_surface_line_width` (`0.42`), and `support_line_width` (`0.35`). D-165 names only the two wall-width keys; correcting the others requires separately establishing each one's governing fallback and is not chartered here.
- **Knowingly surviving: the three sibling per-field doc comments keep the disproved parity attribution.** Step 2 deletes the *shared* constructor comment ("OrcaSlicer 0.4 mm nozzle parity defaults (matches config_schema.rs registration)") and rewrites the two wall-width field doc comments. The three sibling field doc comments on `sparse_infill_line_width`, `top_surface_line_width`, and `support_line_width` (`crates/slicer-gcode/src/serialize.rs`) each carry the identical "(OrcaSlicer 0.4 mm nozzle parity default: N)" attribution, and `wall-width config-type gap` disproves that framing for the wall keys specifically — not for these three, whose governing fallback this packet has **not** established. They therefore survive deliberately, not by oversight. Do not delete or rewrite them here: correcting an attribution without proving the replacement is how D-165 was created. Whichever packet charters those three fields inherits the cleanup.
- Introducing config-driven wiring so the header reports the *actual* resolved widths rather than a default. That is a larger change (a new setter plus a runtime call-site change in `crates/slicer-runtime/src/run.rs` and `pipeline.rs`); this packet only makes the hard-coded default truthful.
- Any change to `resolve_line_width_mm` or the `classic-perimeters` / `arachne-perimeters` width resolution — those are the *authority* here, not the subject. The T2 flow packets own them.
- Re-recording any perimeter or arachne fixture, and any golden other than `precision_legacy_20mmbox.gcode`. This packet changes no geometry — but note that "no geometry change" does **not** imply "no fixture fallout": the one golden in scope breaks precisely because it records the emitted *header text*. Do not generalize this exclusion into skipping the recorded-output blast radius.

## Authoritative Docs

- `docs/15_config_keys_reference.md` — delegated grep only; every `outer_wall_line_width` / `inner_wall_line_width` row already documents default `0.4`, so no doc edit is required. The evidence must be **field-scoped**: an unqualified `0.42|0.45` grep matches an unrelated `infill_overlap | float | 0.45` row and is not admissible. See `packet.spec.md` §Doc Impact for the scoped command.
- `docs/DEVIATION_LOG.md` — the D-165 row only (ranged read; the file is large). Status flip at the completion gate.

## Acceptance Summary

Reference, never copy, criteria from `packet.spec.md`.

- Positive: `AC-1` (emitted header reports `0.4` on whole lines for both wall-width keys and no longer reports the `0.42`/`0.45` forms of those two keys), `AC-2` (source defaults are `0.4` and the `config_schema` citation is gone), `AC-3` (the byte-identity golden `precision_legacy_20mmbox.gcode` is re-blessed and `legacy_zero_matches_golden` passes).
- Negative: none. This is a constant correction with no validator, scheduler-rule, contract-boundary, or error-path surface; there is no rejection behavior to assert.
- Cross-packet impact: the T2 packet **`184-classic-perimeter-flow-parity`** retypes these same config keys to float-or-percent with an auto (`0`→nozzle) default. Its `[FWD-1]` decision (`.ralph/specs/184-classic-perimeter-flow-parity/design.md` §Open Questions) deliberately preserves `0.4` as the absent-key resolved fallback, which is what keeps this header truthful; it does not change this packet's assertions. **The two packets do collide on `crates/slicer-runtime/tests/fixtures/golden/precision_legacy_20mmbox.gcode`** — see `packet.spec.md` §Prerequisites for the whichever-lands-second rule.

## Verification Commands

| Command | Purpose | Return format hint |
| --- | --- | --- |
**Copy commands from `packet.spec.md`, not from this table** — markdown escaping of `|` here is not valid shell.

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo test -p slicer-gcode --test golden_emit_tdd -- header_reports_governing_wall_width_defaults` guarded by `rg "^test result"` + `rg -v "0 passed"` | AC-1: emitted header reports `0.4` on whole lines for both keys | FACT pass/fail; SNIPPETS <=20 lines on failure |
| `rg -q "outer_wall_line_width: 0\.4," …` + `rg -q "inner_wall_line_width: 0\.4," …` + `! rg -q "config_schema" …` on `crates/slicer-gcode/src/serialize.rs` | AC-2: source defaults corrected, dangling citation removed | FACT PASS/FAIL |
| `rg -q "^; outer_wall_line_width = 0\.4$"` on the golden + `cargo test -p slicer-runtime --test e2e -- legacy_zero_matches_golden` with the `0 passed` guard | AC-3: byte-identity golden re-blessed and green | FACT pass/fail |
| `git diff crates/slicer-runtime/tests/fixtures/golden/precision_legacy_20mmbox.gcode \| rg "^[-+][^-+]"` | Re-bless touched **the two wall-width header lines and nothing else** — every changed line must be an `; outer_wall_line_width` or `; inner_wall_line_width` comment. Do **not** pin this at 2 insertions / 2 deletions: packet 184 re-blesses the same golden for wall-coordinate drift and, if it lands first, the numstat baseline moves. See `packet.spec.md` §Prerequisites | FACT: the changed-line listing |
| `cargo test -p slicer-gcode --test golden_emit_tdd` | No regression in the existing sentinel test | FACT pass/fail |
| `cargo check --workspace --all-targets` | Compilation gate | FACT pass/fail |
| `cargo clippy --workspace --all-targets -- -D warnings` | Lint gate | FACT pass/fail |

## Step Completion Expectations

The existing test `serialize_gcode_emits_documented_sentinels` asserts only the *presence* of `; outer_wall_line_width = ` (no value), so it stays green across this change and must not be weakened to compensate. The new test is additive.

Two assertion traps must be avoided, both stemming from the header's own text:
1. **Prefix trap.** `; outer_wall_line_width = 0.4` is a strict prefix of `; outer_wall_line_width = 0.42`, so a positive `contains()` assertion passes on the *unfixed* tree. Assert whole lines.
2. **Sibling collision.** `sparse_infill_line_width = 0.45` and `top_surface_line_width = 0.42` legitimately remain in the header, so an unqualified `!contains("= 0.45")` fails *after* the fix. Key every negative assertion to its field name.

Step 2 must not be treated as complete on its own: it has no struct-literal fallout, but it does have **recorded-output** fallout in the golden fixture, which Step 3 owns.

## Context Discipline Notes

`crates/slicer-gcode/src/serialize.rs` is long — never read it in full. Three ranged reads cover everything; locate each by symbol rather than trusting the line pin: the struct declaration plus `with_extrusion_mode` constructor (~lines 66-116), `serialize_width_comments` (~lines 274-301), and its call site (~lines 687-695). `docs/DEVIATION_LOG.md` is large; read only the D-165 row. The golden `precision_legacy_20mmbox.gcode` is a recorded G-code file — locate its two wall-width lines by `rg`, never load it wholesale, and verify the re-bless by `git diff --numstat` rather than by reading the file.
