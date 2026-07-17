# Implementation Plan: 173-thumbnails-multiformat

## Execution Rules

- Work one atomic step at a time; map every step to grouped task IDs.
- Use TDD, then implementation, then the narrowest falsifying validation.
- Every field below is a context-budget contract and must be filled independently; never write "see Step 1".

## Steps

### Step 1: ThumbnailSpec model + `parse_thumbnails_key` (TDD)

- Task IDs: `TASK-277`
- Objective: add `ThumbnailFormat` (with `tag()`), `ThumbnailSpec`, `ThumbnailError`, and `parse_thumbnails_key` to `crates/slicer-gcode/src/thumbnail.rs`; new standalone test file `crates/slicer-gcode/tests/thumbnail_formats_tdd.rs` with parser tests (valid multi-entry, case-insensitive EXT, `parse_thumbnails_key_rejects_*` for `"48x/PNG"`, `"48x48"`, `"48x48/BMP"` asserting the offending entry appears in the error string).
- Precondition: tree builds clean; `thumbnail.rs` still has the packet-55 shape (`serialize_thumbnail_block(&[u8])`).
- Postcondition: parser + model compile and pass; `serialize_thumbnail_block` untouched.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-gcode/src/thumbnail.rs` - full (59 lines)
  - `crates/slicer-gcode/src/lib.rs` - full (short)
- Files allowed to edit (at most 3):
  - `crates/slicer-gcode/src/thumbnail.rs`
  - `crates/slicer-gcode/tests/thumbnail_formats_tdd.rs` (new)
  - `crates/slicer-gcode/src/lib.rs`
- Files explicitly out of bounds:
  - `crates/slicer-runtime/**`, `crates/pnp-cli/**`, `OrcaSlicerDocumented/**`
- Expected sub-agent dispatches:
  - Question: does `make_and_check_thumbnail_list` accept `JPEG` as alias and how are whitespace/empty entries handled?; scope: `OrcaSlicerDocumented/src/libslic3r/GCode/Thumbnails.cpp`; return: `FACT`
- Context cost: `S`
- Authoritative docs:
  - `docs/specs/fork-gaps-wave2-plan.md` §Packet 173 - direct read
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/GCode/Thumbnails.cpp` (`make_and_check_thumbnail_list`) - delegate; never load
- Verification:
  - `mkdir -p target && cargo test -p slicer-gcode --test thumbnail_formats_tdd parse 2>&1 | tee target/test-output.log | grep -E "^test result"` - FACT pass/fail
- Exit condition: parser tests green; a malformed entry error names the entry verbatim (falsified if the error is generic).

### Step 2: Wire-format rewrite — `RenderedThumbnail` + inner framing at 78 cols

- Task IDs: `TASK-277`
- Objective: add `RenderedThumbnail`/`ThumbnailBody`; re-shape `serialize_thumbnail_block(entries: &[RenderedThumbnail]) -> String` to emit outer sentinels + per-entry `; <tag> begin <W>x<H> <len>` / 78-col `; `-prefixed base64 / `; <tag> end` (Base64 body) or verbatim splice (Raw body); update `ThumbnailAwareSerializer` (`serialize.rs:490-530`) to carry `Option<Vec<RenderedThumbnail>>`; unit tests in `thumbnail_formats_tdd.rs` for framing (header line contents, 78-col wrap, multi-entry ordering).
- Precondition: Step 1 merged; `run_pipeline` callers still compile only after `pipeline.rs` is adapted — this step also makes the minimal `pipeline.rs` call-site fix (single PNG entry at source dimensions, passthrough bytes) so the tree stays green.
- Postcondition: tree compiles; block output is the new format; integration tests asserting the old format are RED (expected until Step 5).
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-gcode/src/serialize.rs` - lines `480-535`
  - `crates/slicer-runtime/src/pipeline.rs` - lines `412-480`
- Files allowed to edit (at most 3):
  - `crates/slicer-gcode/src/thumbnail.rs`
  - `crates/slicer-gcode/src/serialize.rs`
  - `crates/slicer-runtime/src/pipeline.rs`
- Files explicitly out of bounds:
  - `crates/pnp-cli/**`, `OrcaSlicerDocumented/**`, integration test file (Step 5 owns it)
- Expected sub-agent dispatches:
  - Question: exact begin/end line text and `max_row_length` in `export_thumbnails_to_file`; scope: `OrcaSlicerDocumented/src/libslic3r/GCode/Thumbnails.hpp`; return: `SNIPPETS`
- Context cost: `M`
- Authoritative docs:
  - `docs/02_ir_schemas.md` - delegated LOCATIONS for the THUMBNAIL_BLOCK section (anchor only; edit happens in Step 6)
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/GCode/Thumbnails.hpp` (`export_thumbnails_to_file`, `tag()` overrides) - delegate; never load
- Verification:
  - `mkdir -p target && cargo test -p slicer-gcode --test thumbnail_formats_tdd framing 2>&1 | tee target/test-output.log | grep -E "^test result"` - FACT pass/fail
  - `cargo check --workspace --all-targets` - FACT pass/fail
- Exit condition: framing unit tests green and workspace type-checks; falsified if any base64 line exceeds 78 payload chars or the begin line lacks `<W>x<H> <len>`.

### Step 3: `image` dependency + renderer + ColPic port (TDD)

- Task IDs: `TASK-277`
- Objective: add `image = { version = "0.25", default-features = false, features = ["png", "jpeg", "qoi"] }` to `crates/slicer-gcode/Cargo.toml`; implement `render_thumbnail_entries` (decode once, resize `CatmullRom`, encode PNG/JPG/QOI to base64 via existing `base64_encode`, PNG-at-source-size passthrough) in `thumbnail.rs`; create `crates/slicer-gcode/src/thumbnail_colpic.rs` with attribution header porting canonical `ColPic_EncodeStr`/`ColPicEncode`/`Byte8bitEncode`/`ADList0` plus the 512px aspect-preserved cap of `compress_thumbnail_colpic` and `;gimage:`/`;simage:` framing. Tests: `colpic_*` (magic prefixes, 512 cap, tiny-image RLE stability), `no_reflip` (top-down gradient preserved through resize+PNG re-encode), JPG SOI / QOI `qoif` magic checks.
- Precondition: Step 2 merged.
- Postcondition: all five formats renderable; `mod thumbnail_colpic;` registered in `lib.rs` (counts within the 3-edit cap via the Cargo.toml/lib.rs pairing below — `thumbnail.rs` edit, `thumbnail_colpic.rs` new file, plus Cargo.toml; `lib.rs` mod line is part of the new-file creation commit and is the single allowed overflow, split into Step 3b if the worker's tooling enforces the cap strictly).
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-gcode/Cargo.toml` - full
  - `crates/slicer-gcode/src/thumbnail.rs` - full post-Step-2
- Files allowed to edit (at most 3):
  - `crates/slicer-gcode/src/thumbnail.rs`
  - `crates/slicer-gcode/src/thumbnail_colpic.rs` (new; includes the `lib.rs`/module registration as its creation side-edit)
  - `crates/slicer-gcode/Cargo.toml`
- Files explicitly out of bounds:
  - `crates/slicer-runtime/**`, `crates/pnp-cli/**`, `OrcaSlicerDocumented/**`
- Expected sub-agent dispatches:
  - Question: verbatim palette/RLE algorithm of `ColPic_EncodeStr`, `ColPicEncode`, `Byte8bitEncode`, `ADList0` incl. chunk framing; scope: `OrcaSlicerDocumented/src/libslic3r/GCode/Thumbnails.cpp`; return: `SNIPPETS` + `SUMMARY`
- Context cost: `M`
- Authoritative docs:
  - `docs/ORCASLICER_ATTRIBUTION.md` - direct read (header text)
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/GCode/Thumbnails.cpp` (`ColPic_EncodeStr`, `compress_thumbnail_colpic`) - delegate; never load
- Verification:
  - `mkdir -p target && cargo test -p slicer-gcode --test thumbnail_formats_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"` - FACT pass/fail
- Exit condition: `colpic`, `no_reflip`, JPG/QOI magic tests green; falsified if a >512px source yields a ColPic dimension >512 or the gradient test detects row inversion.

### Step 4: BTT_TFT port (TDD)

- Task IDs: `TASK-277`
- Objective: create `crates/slicer-gcode/src/thumbnail_btt.rs` (attribution header) porting canonical `compress_thumbnail_btt_tft`/`get_hex`/`rjust`: `;<WWWW><HHHH>` 4-hex-digit header, per-row RGB565 hex (`((r>>3)<<11)|((g>>2)<<5)|(b>>3)`), `\r\n` endings; wire `ThumbnailFormat::BttTft` in `render_thumbnail_entries`; `btt_tft_*` tests with a 2x2 known-colour image asserting exact hex output and `\r\n`.
- Precondition: Step 3 merged.
- Postcondition: all five formats produce output through `render_thumbnail_entries`.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-gcode/src/thumbnail.rs` - full post-Step-3
- Files allowed to edit (at most 3):
  - `crates/slicer-gcode/src/thumbnail_btt.rs` (new; includes module registration as its creation side-edit)
  - `crates/slicer-gcode/src/thumbnail.rs`
  - `crates/slicer-gcode/tests/thumbnail_formats_tdd.rs`
- Files explicitly out of bounds:
  - `crates/slicer-runtime/**`, `OrcaSlicerDocumented/**`
- Expected sub-agent dispatches:
  - Question: exact `compress_thumbnail_btt_tft` line format (`get_hex` nibble order, `rjust` pad width, where `\r\n` falls, header composition); scope: `OrcaSlicerDocumented/src/libslic3r/GCode/Thumbnails.cpp`; return: `SNIPPETS`
- Context cost: `S`
- Authoritative docs:
  - `docs/ORCASLICER_ATTRIBUTION.md` - direct read (header text)
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/GCode/Thumbnails.cpp` (`compress_thumbnail_btt_tft`) - delegate; never load
- Verification:
  - `mkdir -p target && cargo test -p slicer-gcode --test thumbnail_formats_tdd btt_tft 2>&1 | tee target/test-output.log | grep -E "^test result"` - FACT pass/fail
- Exit condition: `btt_tft` tests green with byte-exact expected hex; falsified if endings are `\n` or the header is not 8 hex digits.

### Step 5: Pipeline `thumbnails`-key wiring + integration-test rewrite

- Task IDs: `TASK-277`
- Objective: in `run_postpass_with_thumbnail` (`pipeline.rs:419-480`), read the `thumbnails` raw-config string when `thumbnail_bytes` is `Some`, call `parse_thumbnails_key` + `render_thumbnail_entries`, map `ThumbnailError` → `PostpassError::GCodeSerialization` (message includes the offending entry), keep `thumbnails` in CONFIG_BLOCK. Rewrite `thumbnail_roundtrip_matches_input` (line 584) and `thumbnail_base64_chunking_orca_parity` in `gcode_header_thumbnail_config_blocks_tdd.rs` — names kept, assertions rewritten — to parse the inner framing (strip `; <tag> begin/end` lines, 78-col assertion); add `thumbnail_multi_entry_resized_png` (48x48 + 300x300, IHDR bytes 16-23 dimension check), `thumbnail_jpg_qoi_entries`, and a no-source-with-key case reusing `sentinels_present_no_thumbnail` semantics.
- Precondition: Steps 1-4 merged; integration tests currently RED on the old-format assertions.
- Postcondition: full integration file green.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-runtime/src/pipeline.rs` - lines `412-480`
  - `crates/slicer-runtime/tests/integration/gcode_header_thumbnail_config_blocks_tdd.rs` - lines `140-260`, `580-650`
- Files allowed to edit (at most 3):
  - `crates/slicer-runtime/src/pipeline.rs`
  - `crates/slicer-runtime/tests/integration/gcode_header_thumbnail_config_blocks_tdd.rs`
- Files explicitly out of bounds:
  - `crates/pnp-cli/**`, `tests/integration/main.rs` (file already registered at line 18 — no aggregator edit needed), `OrcaSlicerDocumented/**`
- Expected sub-agent dispatches: none
- Context cost: `M`
- Authoritative docs:
  - `docs/02_ir_schemas.md` - delegated LOCATIONS only
- OrcaSlicer refs: none (format already locked by Steps 2-4)
- Verification:
  - `mkdir -p target && cargo test -p slicer-runtime --test integration gcode_header_thumbnail_config_blocks 2>&1 | tee target/test-output.log | grep -E "^test result"` - FACT pass/fail
- Exit condition: all integration tests green including the two rewritten and three new; falsified if the multi-entry test passes without dimension-checking decoded IHDR bytes.

### Step 6: Docs + deviation row

- Task IDs: `TASK-277`
- Objective: edit the THUMBNAIL_BLOCK section of `docs/02_ir_schemas.md` (new wire format: five tags, inner begin/end framing, 78-col wrap, outer sentinels retained; fork-facing single-source-PNG contract note naming fork ticket 011); append `D-173-THUMBNAIL-SINGLE-PNG` to `docs/DEVIATION_LOG.md` in the log's live row format.
- Precondition: Steps 1-5 merged.
- Postcondition: AC-7 greps pass.
- Files allowed to read, with ranges when over 300 lines:
  - `docs/02_ir_schemas.md` - only the section located by dispatch
  - `docs/DEVIATION_LOG.md` - last ~20 rows only (format sample)
- Files allowed to edit (at most 3):
  - `docs/02_ir_schemas.md`
  - `docs/DEVIATION_LOG.md`
- Files explicitly out of bounds:
  - all source crates; `OrcaSlicerDocumented/**`
- Expected sub-agent dispatches:
  - Question: locate the THUMBNAIL_BLOCK/CONFIG_BLOCK serialization section heading; scope: `docs/02_ir_schemas.md`; return: `LOCATIONS`
- Context cost: `S`
- Authoritative docs:
  - `docs/02_ir_schemas.md` - ranged (located section)
- OrcaSlicer refs: none
- Verification:
  - `rg -q 'thumbnail begin' docs/02_ir_schemas.md && rg -q 'D-173-THUMBNAIL-SINGLE-PNG' docs/DEVIATION_LOG.md && echo PASS` - FACT PASS/absent
- Exit condition: both greps return PASS; falsified if the docs note omits the fork-ticket-011 deviation statement.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | S | parser + model |
| Step 2 | M | wire format + serializer/pipeline shim |
| Step 3 | M | image dep + renderer + ColPic port |
| Step 4 | S | BTT_TFT port |
| Step 5 | M | pipeline wiring + integration rewrite |
| Step 6 | S | docs + deviation |

## Packet Completion Gate

- All steps and exits complete.
- Every pipe-suffixed AC command returns PASS.
- Update `docs/07_implementation_status.md` through a worker dispatch, never a full backlog read: add the `TASK-277` row (thumbnail wire format + multi-format generation, packet 173) and tick it.
- Reconcile reopened/superseded status transitions: none (no packet superseded).
- `packet.spec.md` is ready for `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed AC and packet-level gate command.
- Record remaining packet-local risk.
- Confirm context stayed at or below 150k standard, or at/below 300k only with a logged swarm ESCALATION; otherwise record a packet-authoring lesson.

All `cargo check`, `cargo clippy`, and `cargo test` invocations in gate and verification commands must use `--all-targets` so the test, bench, and example targets compile.
