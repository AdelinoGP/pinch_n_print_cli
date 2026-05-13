# Implementation Plan: 55_gcode-header-thumbnail-config-blocks

## Execution Rules

- One atomic step at a time.
- Each step must map back to `TASK-156` (envelope) or `TASK-157` (thumbnail) — both new in this packet (insert rows in `docs/07_implementation_status.md` via Step 7's dispatch, not by hand-editing the backlog from inside the implementer).
- TDD first: Step 1 writes the failing tests; subsequent steps make them pass one cluster at a time.
- Each step honors the shared context-discipline preamble. The fields below are the budget contract.
- Steps stay inside the packet boundary; no edits in any other `.ralph/specs/*/` directory.

## Steps

### Step 1: Author the failing TDD test file

- Task IDs:
  - `TASK-156`
  - `TASK-157`
- Objective: create `crates/slicer-host/tests/gcode_header_thumbnail_config_blocks_tdd.rs` with one test per AC in `packet.spec.md` plus the five negative-case tests. Test bodies invoke the existing slicing entrypoint with a fixed small fixture (Benchy or a smaller test mesh already used by `orca_comment_contract_tdd`) and assert against the resulting G-code text. All tests must FAIL because no envelope emission exists yet.
- Precondition: `cargo check --workspace` is green at branch HEAD.
- Postcondition: every test in the new file compiles and fails with the expected "sentinel not found" / "field missing" / "thumbnail file not read" message.
- Files allowed to read:
  - `crates/slicer-host/tests/orca_comment_contract_tdd.rs` — pattern to mirror for fixture setup.
  - `crates/slicer-host/src/gcode_emit.rs` lines `:374-:490` — to know exact output shape today.
  - `crates/slicer-host/src/pipeline.rs` lines `:217-:265` — `run_pipeline_with_raw_config` signature.
- Files allowed to edit (≤ 3):
  - `crates/slicer-host/tests/gcode_header_thumbnail_config_blocks_tdd.rs` (new).
  - `crates/slicer-host/tests/fixtures/test_thumb.png` (new, ~1 KB).
  - `crates/slicer-host/tests/fixtures/not_a_png.bin` (new, ≤ 64 bytes).
- Files explicitly out-of-bounds for this step:
  - `crates/slicer-host/src/gcode_emit.rs` (editing) — Step 3+ only.
  - `crates/slicer-host/src/config_schema.rs` — Step 2.
  - `crates/slicer-host/src/cli.rs`, `main.rs` — Step 5.
- Expected sub-agent dispatches:
  - "Show the fixture setup pattern in `orca_comment_contract_tdd.rs`; return SNIPPETS ≤ 30 lines of the setup helper only" — scope: that file; return: SNIPPETS.
- Context cost: `S`.
- Authoritative docs:
  - `docs/02_ir_schemas.md` — `PrintMetadata`, `LayerCollectionIR` only; range-read.
- OrcaSlicer refs:
  - None at this step; ACs encode the expected literals.
- Verification:
  - `cargo test -p slicer-host --test gcode_header_thumbnail_config_blocks_tdd` — dispatch as FACT; expected result: every test fails (the negative confirmation we want before any implementation).
- Exit condition: every AC test exists, compiles, and fails for the expected reason.

### Step 2: Register the four new config keys

- Task IDs:
  - `TASK-156`
- Objective: add `filament_diameter` (f32, default `1.75`), `filament_density` (f32, default `1.24`), `max_z_height` (f32, default `256.0`), and `thumbnail_path` (String, default `""`) to `crates/slicer-host/src/config_schema.rs`. Also register any width keys listed in AC4 that are not already registered (`outer_wall_line_width`, `inner_wall_line_width`, `sparse_infill_line_width`, `top_surface_line_width`, `support_line_width`) with OrcaSlicer-parity defaults for a 0.4 mm nozzle.
- Precondition: Step 1 complete.
- Postcondition: `query_config_schema()` (`:164`) returns entries for all four new keys plus any width keys that were missing.
- Files allowed to read:
  - `crates/slicer-host/src/config_schema.rs` — full file (~170 lines).
- Files allowed to edit (≤ 3):
  - `crates/slicer-host/src/config_schema.rs`.
- Files explicitly out-of-bounds for this step:
  - `crates/slicer-host/src/gcode_emit.rs` (no consumption yet).
- Expected sub-agent dispatches:
  - "List every key currently registered in `crates/slicer-host/src/config_schema.rs`; return LOCATIONS ≤ 40 entries with type + default" — purpose: discover which AC4 width keys are already registered vs need adding.
- Context cost: `S`.
- Authoritative docs:
  - `docs/03_wit_and_manifest.md` — config-schema-validation section only.
- OrcaSlicer refs:
  - None; defaults are well-known PLA/0.4mm-nozzle values.
- Verification:
  - `cargo check --workspace` — FACT pass/fail.
  - `cargo test -p slicer-host --test gcode_header_thumbnail_config_blocks_tdd -- header_four_required_fields --nocapture` — still fails (no emission yet), but the test should now find the keys in `ConfigView` rather than panic on lookup.
- Exit condition: `cargo check` green; the four new keys appear in the LOCATIONS dispatch result.

### Step 3: Emit HEADER_BLOCK

- Task IDs:
  - `TASK-156`
- Objective: add `serialize_header_block(metadata: &PrintMetadata, cfg: &ConfigView, max_z_mm: f32) -> String` to `crates/slicer-host/src/gcode_emit.rs`. Wire it at the top of `DefaultGCodeSerializer::serialize_gcode()` so its output is the first thing in the file (before any `;TYPE:` or motion command, BUT after any pre-existing slicer-version header line if one exists today). Emit, in OrcaSlicer order: `; HEADER_BLOCK_START` line, then `; total layer number: <metadata.layer_count>`, `; filament_diameter: <cfg["filament_diameter"]>`, `; filament_density: <cfg["filament_density"]>`, `; max_z_height: <max_z_mm>`, `; filament: <comma-sep used tool indices>`, then `; HEADER_BLOCK_END`. Compute `max_z_mm` as `layer_irs.last().z` converted to mm (or the registered `max_z_height` default — pick the larger; FACT-3 confirms OrcaSlicer's choice). Compute used-tools list from `metadata.filament_used_mm` non-zero entries in ascending index order.
- Precondition: Steps 1 + 2 complete.
- Postcondition: ACs `header_four_required_fields`, `header_layer_count_matches_sliced`, `header_max_z_matches_top_layer`, `header_filament_order_matches_used`, and `rejects_layer_count_drift` PASS.
- Files allowed to read:
  - `crates/slicer-host/src/gcode_emit.rs` lines `:370-:490`.
  - `crates/slicer-ir/src/slice_ir.rs` lines `:1634-:1660` (`PrintMetadata`), `:1524-:1540` (`LayerCollectionIR.z`).
- Files allowed to edit (≤ 3):
  - `crates/slicer-host/src/gcode_emit.rs`.
- Files explicitly out-of-bounds for this step:
  - `crates/slicer-host/src/cli.rs`, `main.rs` — Step 5.
  - OrcaSlicer source — delegated only.
- Expected sub-agent dispatches:
  - "Read `OrcaSlicerDocumented/src/libslic3r/GCode.cpp` lines `2640-2710`; return FACT ≤ 12 lines: exact sentinel literals, line prefix, `total layer number` value formatting" — purpose: ground sentinel + prefix.
  - "Read `OrcaSlicerDocumented/src/libslic3r/GCode.cpp` lines `2670-2705`; return FACT ≤ 10 lines: spelling and value format of filament_diameter / filament_density / max_z_height / filament" — purpose: ground field formatting.
- Context cost: `S`.
- Authoritative docs:
  - `docs/08_coordinate_system.md` — unit-conversion paragraph for 100 nm → mm.
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/GCode.cpp` lines `2640-2710` (delegate FACT).
- Verification:
  - `cargo test -p slicer-host --test gcode_header_thumbnail_config_blocks_tdd -- header_four_required_fields header_layer_count_matches_sliced header_max_z_matches_top_layer header_filament_order_matches_used rejects_layer_count_drift --nocapture` — FACT pass/fail; SNIPPETS on first failure.
- Exit condition: all five named tests pass; previously-failing tests for sentinel presence (`sentinels_present_no_thumbnail`) move from "HEADER_BLOCK_START not found" to "CONFIG_BLOCK_START not found" or similar (progress, not regression).

### Step 4: Emit extrusion-width comments

- Task IDs:
  - `TASK-156`
- Objective: add `serialize_width_comments(cfg: &ConfigView) -> String` and append its output immediately after `HEADER_BLOCK_END`, before any THUMBNAIL or motion. Emit `; outer_wall_line_width = <value>`, `; inner_wall_line_width = <value>`, `; sparse_infill_line_width = <value>`, `; top_surface_line_width = <value>`, `; support_line_width = <value>`. Read each value from `cfg` (registered or user-passed). If FACT-3 returns a different OrcaSlicer canonical list, amend this list to match (the AC is grounded in "OrcaSlicer parity"; the implementer adjusts AC4 in the test file if FACT-3 contradicts).
- Precondition: Step 3 complete.
- Postcondition: AC `width_comments_emitted` PASSES.
- Files allowed to read:
  - `crates/slicer-host/src/gcode_emit.rs` lines `:370-:490`.
- Files allowed to edit (≤ 3):
  - `crates/slicer-host/src/gcode_emit.rs`.
  - `crates/slicer-host/tests/gcode_header_thumbnail_config_blocks_tdd.rs` — only if FACT-3 amends the canonical width-key list.
- Files explicitly out-of-bounds for this step:
  - `crates/slicer-host/src/config_schema.rs` (registration done in Step 2).
- Expected sub-agent dispatches:
  - "Read `OrcaSlicerDocumented/src/libslic3r/GCode.cpp` lines `2750-2765`; return FACT ≤ 8 lines: width-comment format and the canonical list of width keys" — purpose: confirm or amend AC4.
- Context cost: `S`.
- Authoritative docs: none additional beyond Step 3.
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/GCode.cpp` lines `2750-2765` (delegate FACT).
- Verification:
  - `cargo test -p slicer-host --test gcode_header_thumbnail_config_blocks_tdd -- width_comments_emitted --nocapture` — FACT pass/fail.
- Exit condition: `width_comments_emitted` passes.

### Step 5: Add `--thumbnail` CLI flag and emit THUMBNAIL_BLOCK

- Task IDs:
  - `TASK-157`
- Objective: thread an external PNG through to the serializer and emit `THUMBNAIL_BLOCK_START..THUMBNAIL_BLOCK_END` containing Base64 of the file. Three sub-edits:
  1. `cli.rs`: add `thumbnail: Option<PathBuf>` to the slice subcommand's args.
  2. `main.rs`: when the flag is present, canonicalize the path, fail-fast non-zero with `thumbnail_path: file not found` if it does not exist, read the bytes, fail-fast with `thumbnail_path: invalid PNG magic` if the first 8 bytes are not `89 50 4E 47 0D 0A 1A 0A`, then insert `("thumbnail_path", ConfigValue::String(canonical_path))` into `config_source`. Pass the validated PNG bytes through to the serializer's call site (smallest-change route: add an `Option<Vec<u8>>` field to whatever struct/argument tuple flows into `serialize_gcode()` today, OR thread a side-channel parameter — the implementer picks the smaller diff after reading `:374-:490`).
  3. `gcode_emit.rs`: add `serialize_thumbnail_block(png_bytes: &[u8]) -> String`. Emit `; THUMBNAIL_BLOCK_START`, optionally a `; thumbnail begin WxH <byte_count>` line if FACT-4 confirms OrcaSlicer emits it (parse W/H from the PNG IHDR — 8 bytes at offset 16, big-endian u32 each), then the Base64 payload wrapped per OrcaSlicer's column width (≤ 76 chars per line) each prefixed `; `, then `; THUMBNAIL_BLOCK_END`. Wire after `serialize_width_comments` and before the first motion command. Skip the entire block when `cfg["thumbnail_path"]` is empty.
- Precondition: Step 4 complete.
- Postcondition: ACs `sentinels_present_with_thumbnail`, `sentinels_present_no_thumbnail`, `thumbnail_roundtrip_matches_input`, `thumbnail_base64_chunking_orca_parity`, `rejects_missing_thumbnail_file`, `rejects_non_png_thumbnail` PASS.
- Files allowed to read:
  - `crates/slicer-host/src/cli.rs` (full, short).
  - `crates/slicer-host/src/main.rs` lines around `:260-:300` (call site).
  - `crates/slicer-host/src/gcode_emit.rs` lines `:370-:490`.
- Files allowed to edit (≤ 3):
  - `crates/slicer-host/src/cli.rs`.
  - `crates/slicer-host/src/main.rs`.
  - `crates/slicer-host/src/gcode_emit.rs`.
- Files explicitly out-of-bounds for this step:
  - `crates/slicer-host/src/config_schema.rs` (registration done in Step 2).
  - Any third-party Base64 crate's source — use whatever Base64 crate is already in the workspace, or a 30-line hand-rolled encoder; do NOT add a new dependency without justification.
- Expected sub-agent dispatches:
  - "Read `OrcaSlicerDocumented/src/libslic3r/GCode/Thumbnails.hpp` lines `100-135`; return FACT ≤ 12 lines: THUMBNAIL_BLOCK_START/END literals, line prefix, Base64 column width (76 or 78), and whether a `; thumbnail begin WxH bytes` metadata line is emitted" — purpose: ground wire format.
  - "Does the workspace already depend on a Base64 crate? Return FACT: yes/no + crate name + version; scope `Cargo.toml`, `crates/*/Cargo.toml`" — purpose: avoid spurious dependency addition.
- Context cost: `M`.
- Authoritative docs:
  - `docs/03_wit_and_manifest.md` — config-schema section (to confirm `ConfigValue::String` propagation).
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/GCode/Thumbnails.hpp` lines `100-135` (delegate FACT).
- Verification:
  - `cargo test -p slicer-host --test gcode_header_thumbnail_config_blocks_tdd -- sentinels_present_with_thumbnail sentinels_present_no_thumbnail thumbnail_roundtrip_matches_input thumbnail_base64_chunking_orca_parity rejects_missing_thumbnail_file rejects_non_png_thumbnail --nocapture` — FACT pass/fail.
- Exit condition: all six named tests pass.

### Step 6: Emit CONFIG_BLOCK

- Task IDs:
  - `TASK-156`
- Objective: add `serialize_config_block(cfg: &ConfigView) -> String` and append at the END of the serialized output (after the final motion line). Iterate `cfg` in deterministic, lexically sorted key order. For each key emit `; <key> = <value>` where `<value>` formats as: integers `{}`, floats `{:.4}` with trailing zeros stripped, bools `true`/`false` (lowercase), strings verbatim. Bracket with `; CONFIG_BLOCK_START` and `; CONFIG_BLOCK_END`. Emit the sentinel pair even when `cfg` is empty (covers `empty_config_view_still_emits_sentinels`).
- Precondition: Step 5 complete.
- Postcondition: ACs `config_block_includes_user_passed`, `config_block_covers_effective_config`, `block_ordering_header_before_body_config_after`, `empty_config_view_still_emits_sentinels`, and `rejects_missing_sentinel_block` PASS.
- Files allowed to read:
  - `crates/slicer-host/src/gcode_emit.rs` lines `:370-:490`.
  - `crates/slicer-ir/src/slice_ir.rs` lines `:467-:520` (`ConfigView`).
- Files allowed to edit (≤ 3):
  - `crates/slicer-host/src/gcode_emit.rs`.
- Files explicitly out-of-bounds for this step:
  - OrcaSlicer source — delegated only.
- Expected sub-agent dispatches:
  - "Read `OrcaSlicerDocumented/src/libslic3r/GCode.cpp` lines `5590-5620`; return FACT ≤ 8 lines: iteration order (sorted or insertion), separator (` = ` vs `=`), and any keys/comments skipped" — purpose: ground iteration order + separator.
- Context cost: `S`.
- Authoritative docs:
  - `docs/02_ir_schemas.md` — `ConfigView` section only.
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/GCode.cpp` lines `5590-5620` (delegate FACT).
- Verification:
  - `cargo test -p slicer-host --test gcode_header_thumbnail_config_blocks_tdd -- config_block_includes_user_passed config_block_covers_effective_config block_ordering_header_before_body_config_after empty_config_view_still_emits_sentinels rejects_missing_sentinel_block --nocapture` — FACT pass/fail.
- Exit condition: all five named tests pass.

### Step 7: Regression + lint + backlog update

- Task IDs:
  - `TASK-156`
  - `TASK-157`
- Objective: confirm no regression in existing G-code envelope behavior; pass workspace check and clippy; update `docs/07_implementation_status.md` to add `TASK-156` and `TASK-157` rows.
- Precondition: Steps 1-6 complete.
- Postcondition: full packet test file green; `orca_comment_contract_tdd` green; `cargo check --workspace` green; `cargo clippy --workspace -- -D warnings` green; `docs/07` updated.
- Files allowed to read:
  - None directly; pure-dispatch step (the implementer adjudicates returned FACTs).
- Files allowed to edit (≤ 3):
  - `docs/07_implementation_status.md` — via worker dispatch only; do not load this file into the implementer's context.
- Files explicitly out-of-bounds for this step:
  - Everything else — this step is regression and bookkeeping only.
- Expected sub-agent dispatches:
  - "Run `cargo test -p slicer-host --test gcode_header_thumbnail_config_blocks_tdd`; return FACT pass/fail; SNIPPETS ≤ 20 lines on first failure" — primary.
  - "Run `cargo test -p slicer-host --test orca_comment_contract_tdd`; return FACT pass/fail" — regression.
  - "Run `cargo check --workspace`; return FACT pass/fail; SNIPPETS ≤ 10 lines on failure" — type-check gate.
  - "Run `cargo clippy --workspace -- -D warnings`; return FACT pass/fail; SNIPPETS ≤ 10 lines on failure" — lint gate.
  - "Insert two rows in `docs/07_implementation_status.md`: `TASK-156` (emit HEADER_BLOCK, extrusion-width comments, CONFIG_BLOCK in final G-code; packet 55) status `[~]`, and `TASK-157` (--thumbnail CLI flag + THUMBNAIL_BLOCK emission; packet 55) status `[~]`. Append at the end of the in-progress section. Return FACT: row line numbers" — backlog update.
- Context cost: `S`.
- Authoritative docs: none.
- OrcaSlicer refs: none.
- Verification:
  - All four dispatches return PASS; backlog update returns FACT with row line numbers.
- Exit condition: every dispatch above returns PASS; the packet acceptance ceremony in the next section can begin.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | S | New test file + 2 fixtures; mirrors existing TDD pattern. |
| Step 2 | S | Pure config_schema.rs registration; no semantic change yet. |
| Step 3 | S | One helper + serializer head-insertion; 2 OrcaSlicer FACTs. |
| Step 4 | S | One helper + serializer head-insertion; 1 OrcaSlicer FACT. |
| Step 5 | M | Three-file change (cli.rs + main.rs + gcode_emit.rs) plus PNG validation + Base64; 1 OrcaSlicer FACT + 1 dependency FACT. |
| Step 6 | S | One helper + serializer tail-insertion; 1 OrcaSlicer FACT. |
| Step 7 | S | Pure-dispatch regression + backlog row insertion. |

Aggregate: `M`. No step is `L`. If Step 5 grows beyond `M` during implementation (e.g., the chosen Base64-passing route forces a `run_pipeline_with_raw_config` signature change cascading into many test callers), STOP, hand off, and split into a follow-up packet 55b for the `--thumbnail` flag — do not absorb the cascade.

## Packet Completion Gate

- All seven steps complete; every step exit condition met.
- All `packet.spec.md` acceptance criteria green: each pipe-suffixed command dispatched and returned PASS.
- `cargo test -p slicer-host --test orca_comment_contract_tdd` green (regression).
- `cargo check --workspace` green.
- `cargo clippy --workspace -- -D warnings` green.
- `docs/07_implementation_status.md` updated with `TASK-156` and `TASK-157` rows via worker dispatch.
- No predecessor packet status transitions required (no supersession).
- `packet.spec.md` ready to move to `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed acceptance criterion command from `packet.spec.md` (positive + negative). Each returns FACT pass.
- Re-run packet-level verification commands listed in `packet.spec.md` Verification section.
- Confirm implementer's peak context usage stayed under 70%; if not, log it (this packet was generated with a documented context budget — overshoot is a packet-authoring lesson, not an implementation flaw).
- Record any packet-local risk that materialized (e.g., OrcaSlicer FACT-3 returning a different canonical width-key list than AC4's enumerated five — log as a parity-correction note).
- Only then update `packet.spec.md` to `status: implemented`.
