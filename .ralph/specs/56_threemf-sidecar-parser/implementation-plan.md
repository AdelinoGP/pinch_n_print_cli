# Implementation Plan: 56_threemf-sidecar-parser

## Execution Rules

- One atomic step at a time.
- Each step maps back to TASK-190.
- TDD first, then implementation, then the narrowest falsifying validation.
- Each step honors the context-discipline preamble. The fields below are not optional — they are the budget contract.
- Aggregate context cost is **M**. The implementer may run all steps from a single worker if budget allows, but is encouraged to dispatch heavier steps (Step 2, Step 6) to fresh workers.

## Steps

### Step 1: Sidecar parser TDD-RED + `PartSubtype` enum stub

- Task IDs:
  - `TASK-190`
- Objective: Introduce a host-local `PartSubtype` enum + carrier structs (`ObjectSidecarInfo`, `PartSidecarInfo`) and a stub `parse_3mf_sidecar` returning `HashMap::new()`. Author the failing TDD that exercises the parser's full API surface against a well-formed sidecar (from `resources/benchy_4color.3mf`), a malformed sidecar (in-test synthetic), a missing-sidecar archive, an unknown-subtype sidecar, an empty-`<part>`-list sidecar, and the Bambu object/part ID mapping convention.
- Precondition: Packet activated. Master branch clean.
- Postcondition: New test file compiles and fails on assertions naming the missing parser behavior. `model_loader.rs` carries the new types and stub but no functional change.
- Files allowed to read:
  - `crates/slicer-host/src/model_loader.rs` — lines 130-203, 285-360, 430-587, 599-650.
  - `crates/slicer-ir/src/slice_ir.rs` — lines 230-265 (informational only).
- Files allowed to edit (≤ 3):
  - `crates/slicer-host/tests/threemf_sidecar_classification_tdd.rs` — NEW.
  - `crates/slicer-host/src/model_loader.rs` — add types + stub.
- Files explicitly out-of-bounds: every other file in `crates/slicer-host/src/`, `crates/slicer-ir/`, `crates/slicer-sdk/`, `crates/slicer-macros/`, all `wit/**`, `crates/slicer-host/src/wit_host.rs`, `dispatch.rs`.
- Expected sub-agent dispatches:
  - Question: "Name the function(s) in `OrcaSlicerDocumented/src/libslic3r/Format/bbs_3mf.cpp` that parse `Metadata/model_settings.config` and the function(s) that branch on `<part subtype>`. Return LOCATIONS with one-line role each; ≤ 8 entries. No source pasted." → LOCATIONS.
  - Question: "Run `unzip -p resources/benchy_4color.3mf Metadata/model_settings.config | head -80`. Return the raw output verbatim; SNIPPETS, ≤ 30 lines." → SNIPPETS.
  - Question: "Run `cargo check -p slicer-host --tests` after Step 1's edits. Return FACT pass/fail and one assertion line on failure." → FACT.
- Context cost: `M`
- Authoritative docs:
  - `docs/02_ir_schemas.md` — lines 192-211. Informational only.
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/Format/bbs_3mf.cpp` — function names only (LOCATIONS dispatch).
- Verification:
  - `cargo test -p slicer-host --test threemf_sidecar_classification_tdd parses_benchy_4color_sidecar -- --exact --nocapture` — expected RED (assertion failure on `parts.is_empty()` because stub returns empty map).
  - `cargo test -p slicer-host --test threemf_sidecar_classification_tdd missing_sidecar_is_silent_default -- --exact --nocapture` — expected GREEN even pre-impl (stub returns empty map).
- Exit condition: TDD-RED assertion messages name the missing parser behavior; the missing-sidecar test passes.

### Step 2: Implement `parse_3mf_sidecar` to make Step 1 tests GREEN

- Task IDs:
  - `TASK-190`
- Objective: Implement the parser using the existing `quick_xml::Reader` pattern. Handle: well-formed → populated map; missing → empty map silent; malformed → empty map + warning; unknown subtype → `NormalPart` + warning; empty-`<part>` list → empty `parts` map without warning.
- Precondition: Step 1 RED.
- Postcondition: All `threemf_sidecar_classification_tdd.rs` tests authored at Step 1 are GREEN (except `load_3mf_invokes_sidecar_parser_before_archive_drop`, which is GREEN only after Step 3).
- Files allowed to read:
  - `crates/slicer-host/src/model_loader.rs` — lines 130-203, 555-587, 599-650.
- Files allowed to edit (≤ 3):
  - `crates/slicer-host/src/model_loader.rs` — sidecar parser implementation. If the file exceeds 800 lines after the addition, extract the parser body to `model_loader_sidecar.rs`.
  - `crates/slicer-host/src/model_loader_sidecar.rs` — NEW if needed.
- Files explicitly out-of-bounds: all WIT, SDK, IR, macros files; all non-`model_loader*` host source files.
- Expected sub-agent dispatches:
  - Question: "Return the log target string used in the 5 most recent `log::warn!` / `log::trace!` calls in `crates/slicer-host/src/model_loader.rs`. SNIPPETS, ≤ 5 lines." → SNIPPETS. Use the returned target string in `parse_3mf_sidecar`'s warnings.
  - Question: "Return the existing `quick_xml::Reader` parse-loop pattern used in `crates/slicer-host/src/model_loader.rs::parse_3mf_model_xml`. SNIPPETS, ≤ 30 lines." → SNIPPETS.
  - Question: "Run `cargo test -p slicer-host --test threemf_sidecar_classification_tdd`. Return FACT pass-count vs total." → FACT.
- Context cost: `M`
- Authoritative docs:
  - `docs/02_ir_schemas.md` line 5 — versioning rule (informational; no IR change in this packet).
- OrcaSlicer refs:
  - Already named at Step 1; no fresh dispatch.
- Verification:
  - `cargo test -p slicer-host --test threemf_sidecar_classification_tdd parses_benchy_4color_sidecar -- --exact --nocapture` → GREEN.
  - `cargo test -p slicer-host --test threemf_sidecar_classification_tdd malformed_sidecar_falls_back_to_normal_part -- --exact --nocapture` → GREEN.
  - `cargo test -p slicer-host --test threemf_sidecar_classification_tdd unknown_subtype_downgrades_to_normal_part -- --exact --nocapture` → GREEN.
  - `cargo test -p slicer-host --test threemf_sidecar_classification_tdd object_and_part_id_mapping_matches_bambu_convention -- --exact --nocapture` → GREEN.
  - `cargo test -p slicer-host --test threemf_sidecar_classification_tdd empty_object_in_sidecar_returns_empty_parts -- --exact --nocapture` → GREEN.
  - `cargo clippy -p slicer-host --tests -- -D warnings` → GREEN.
- Exit condition: Five of six parser tests GREEN; `load_3mf_invokes_sidecar_parser_before_archive_drop` still RED (Step 3 fixes it); clippy clean.

### Step 3: Plumb `parse_3mf_sidecar` into `load_3mf` and widen `resolve_object` signature

- Task IDs:
  - `TASK-190`
- Objective: Call `parse_3mf_sidecar(&mut zip)` in `load_3mf` between `parse_3mf_model_xml(...)` and the `ZipArchive` drop. Thread the resulting map into `parse_3mf_model_xml` (signature widen) and through to `resolve_object` (signature widen). `resolve_object`'s body is unchanged; the new parameter is named `_sidecar` to silence the `unused_variables` lint.
- Precondition: Step 2 GREEN (parser unit suite passes).
- Postcondition: `load_3mf_invokes_sidecar_parser_before_archive_drop` GREEN.
- Files allowed to read:
  - `crates/slicer-host/src/model_loader.rs` — lines 430-650.
- Files allowed to edit (≤ 3):
  - `crates/slicer-host/src/model_loader.rs` — `load_3mf` plumbing + `parse_3mf_model_xml` and `resolve_object` signature widen.
  - `crates/slicer-host/tests/threemf_sidecar_classification_tdd.rs` — add/extend the `load_3mf_invokes_sidecar_parser_before_archive_drop` test if not already present from Step 1.
- Files explicitly out-of-bounds: WIT, SDK, IR, macros, other host source files.
- Expected sub-agent dispatches:
  - Question: "Enumerate every call site of `resolve_object` in the workspace. Return LOCATIONS with file:line; ≤ 10 entries." → LOCATIONS. Expected count: 1 (inside `parse_3mf_model_xml`).
  - Question: "Run `cargo test -p slicer-host --test threemf_sidecar_classification_tdd load_3mf_invokes_sidecar_parser_before_archive_drop -- --exact --nocapture`. Return FACT pass/fail." → FACT.
- Context cost: `M`
- Authoritative docs: none new.
- OrcaSlicer refs: none new.
- Verification:
  - `cargo test -p slicer-host --test threemf_sidecar_classification_tdd` — all six tests GREEN.
  - `cargo check --workspace` — GREEN.
- Exit condition: All six parser tests GREEN; workspace compiles.

### Step 4: Regression sweep (no-sidecar + transform + gcode)

- Task IDs:
  - `TASK-190`
- Objective: Confirm the producer-only plumbing produces byte-identical slice output for fixtures with and without sidecars. The parser map is threaded but unused; behavior must be unchanged.
- Precondition: Step 3 GREEN.
- Postcondition: All regression suites GREEN.
- Files allowed to read: none.
- Files allowed to edit (≤ 3): none.
- Files explicitly out-of-bounds: all source.
- Expected sub-agent dispatches:
  - Question: "Run `cargo test -p slicer-host --test threemf_transform_tdd && cargo test -p slicer-host --test gcode_emit_tdd && cargo test -p slicer-host --test benchy_painted_e2e_tdd && cargo test -p slicer-host --test benchy_painted_overrides_e2e_tdd`. Return FACT pass/fail per file with totals." → FACT.
- Context cost: `S`
- Authoritative docs: none.
- OrcaSlicer refs: none.
- Verification: All four FACTs GREEN.
- Exit condition: No regressions. If any suite fails, return to Step 3 to debug the plumbing; do not advance to Step 5.

### Step 5: Clippy + check sweep

- Task IDs:
  - `TASK-190`
- Objective: Confirm lint + build cleanliness.
- Precondition: Step 4 GREEN.
- Postcondition: `cargo clippy --workspace -- -D warnings` GREEN; `cargo check --workspace` GREEN.
- Files allowed to read: none.
- Files allowed to edit (≤ 3): any source file the lint pass demands (sticking to files-in-scope from earlier steps; almost certainly only `model_loader.rs`).
- Files explicitly out-of-bounds: macros, WIT, SDK, IR.
- Expected sub-agent dispatches:
  - Question: "Run `cargo clippy --workspace -- -D warnings`. FACT pass/fail with first warning if fail." → FACT.
  - Question: "Run `cargo check --workspace`. FACT pass/fail with first error if fail." → FACT.
- Context cost: `S`
- Authoritative docs: none.
- OrcaSlicer refs: none.
- Verification: Both FACTs GREEN.
- Exit condition: Clean workspace.

### Step 6: Doc + deviation registration

- Task IDs:
  - `TASK-190`
- Objective: Append `[x] TASK-190` row to `docs/07_implementation_status.md` naming this packet. Register DEV-047 and DEV-049 as `Closed — Packet 56, 2026-MM-DD` in `docs/DEVIATION_LOG.md`. Add chronology entries in `docs/14_deviation_audit_history.md`.
- Precondition: Step 5 GREEN.
- Postcondition: Docs reflect packet outcome.
- Files allowed to read:
  - `docs/07_implementation_status.md` — delegate FACT for the "highest existing TASK row line" only; do not load the full file.
  - `docs/DEVIATION_LOG.md` — delegate FACT for the "highest existing DEV-### slot" only.
  - `docs/14_deviation_audit_history.md` — delegate FACT for the append position only.
- Files allowed to edit (≤ 3 per dispatch; 3 files spread across two dispatches):
  - `docs/07_implementation_status.md`
  - `docs/DEVIATION_LOG.md`
  - `docs/14_deviation_audit_history.md`
- Files explicitly out-of-bounds: all source.
- Expected sub-agent dispatches:
  - Question: "Append a `[x] TASK-190` row to `docs/07_implementation_status.md` immediately after the highest existing TASK row, naming packet `56_threemf-sidecar-parser`. Return the resulting line verbatim. SNIPPETS, ≤ 3 lines." → SNIPPETS.
  - Question: "Confirm next free DEV-### slot in `docs/DEVIATION_LOG.md` is 047, and 049 is also free (048 is reserved for Packet 56b). Return FACT yes/no + the highest existing DEV-###." → FACT.
- Context cost: `S`
- Authoritative docs:
  - `docs/14_deviation_audit_history.md` — chronology pattern (delegate read of an existing entry as SNIPPETS).
- OrcaSlicer refs: none.
- Verification:
  - `rg -q '\[x\] TASK-190.*56_threemf-sidecar-parser' docs/07_implementation_status.md` → exit 0.
  - `rg -c '^\| DEV-04[79].*Closed.*Packet 56[^b]' docs/DEVIATION_LOG.md` → 2.
  - `! rg -q '^\| DEV-048.*Closed.*Packet 56[^b]' docs/DEVIATION_LOG.md` → exit 0 (DEV-048 must NOT be closed by Packet 56).
- Exit condition: All `rg` checks return the expected exit codes.

### Step 7: Packet acceptance ceremony

- Task IDs:
  - `TASK-190`
- Objective: Dispatch every pipe-suffixed acceptance criterion command from `packet.spec.md` to a worker and record FACT pass/fail. If any criterion fails, return to the relevant step; do not flip status.
- Precondition: Steps 1-6 GREEN.
- Postcondition: All AC commands GREEN; `packet.spec.md` ready to flip to `status: implemented`.
- Files allowed to read: `packet.spec.md` (this packet).
- Files allowed to edit (≤ 3): `packet.spec.md` (status flip on success only).
- Files explicitly out-of-bounds: every source file.
- Expected sub-agent dispatches:
  - One dispatch per AC command, each returning FACT pass/fail.
- Context cost: `S`
- Authoritative docs: this packet's `packet.spec.md`.
- OrcaSlicer refs: none.
- Verification:
  - All AC commands return PASS.
- Exit condition: Status flippable to `implemented`.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
|---|---|---|
| Step 1 | M | New test file + parser stub + Bambu sidecar SNIPPETS. |
| Step 2 | M | Sidecar parser implementation; reuses `quick_xml` pattern. |
| Step 3 | M | `load_3mf` plumbing + signature widen of `parse_3mf_model_xml` and `resolve_object`. |
| Step 4 | S | Regression sweep dispatch. |
| Step 5 | S | Clippy + check sweep dispatch. |
| Step 6 | S | Doc + deviation registration; three small file edits. |
| Step 7 | S | Acceptance ceremony dispatches. |

Aggregate: **M** (3 M + 4 S).

## Packet Completion Gate

- All 7 steps complete.
- Every step exit condition met.
- Packet acceptance criteria GREEN (each verification command dispatched and returned PASS).
- `docs/07_implementation_status.md` updated for TASK-190 via worker dispatch.
- DEV-047 and DEV-049 registered in `docs/DEVIATION_LOG.md` and chronology in `docs/14_deviation_audit_history.md`.
- DEV-048 reserved for Packet 56b (NOT closed in this packet).
- `packet.spec.md` ready to move to `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed acceptance criterion command from `packet.spec.md` (Step 7).
- Confirm packet-level verification commands are GREEN (Steps 4, 5, 7).
- Confirm the implementer's peak context usage stayed under 70%; if not, log it as a packet-authoring lesson.
- This packet does NOT run `cargo test --workspace` at closure. The parser is producer-only and threaded but unused; the targeted regression suites in Step 4 cover the full behavioral surface. If a future packet (56b, 56c) requires a workspace-wide check, that check belongs to that packet's ceremony.
