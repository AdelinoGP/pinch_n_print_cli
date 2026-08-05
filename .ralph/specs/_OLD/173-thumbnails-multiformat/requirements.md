# Requirements: 173-thumbnails-multiformat

## Packet Metadata

- Grouped task IDs: `TASK-277` (new; minted at closure via `task-map.md` — not yet a row in `docs/07_implementation_status.md`)
- Backlog source: `docs/07_implementation_status.md` (wave-2 plan `docs/specs/fork-gaps-wave2-plan.md` §Packet 173, handoff item 14)
- Packet status: `draft`
- Aggregate context cost: `M`

## Problem Statement

The current `THUMBNAIL_BLOCK` (`crates/slicer-gcode/src/thumbnail.rs`, 59 lines) emits only outer sentinels plus bare 76-col `; <chunk>` base64 lines. No printer firmware or Orca-family parser can locate a thumbnail in it: they all key off the inner `; <tag> begin <W>x<H> <len>` / `; <tag> end` framing that canonical `export_thumbnails_to_file` (`Thumbnails.hpp`) writes at 78 columns. Additionally, PNP can only embed the one PNG handed to `--thumbnail` — there is no way to satisfy printers that require JPG (some Klipper screens), QOI (Prusa/Orca previews), BTT_TFT RGB565 hex, or QIDI ColPic. The user-decided contract (deviating from fork ticket 011): the fork renders **one** high-res PNG; PNP decodes it, rescales per `thumbnails` config entry, and encodes every requested format itself. The existing roundtrip test asserts the wrong (current) format and must be rewritten to parse the real one.

## In Scope

- Rewrite the wire format in `crates/slicer-gcode/src/thumbnail.rs`: per-entry `; <tag> begin <W>x<H> <encoded_len>` / `; <tag> end` framing inside the retained outer `; THUMBNAIL_BLOCK_START` / `; THUMBNAIL_BLOCK_END` sentinels; base64 wrapped at 78 columns (was 76).
- Format tags exactly per Orca `tag()` overrides: `thumbnail` (PNG), `thumbnail_JPG`, `thumbnail_QOI`, `thumbnail_BIQU` (BTT_TFT), `thumbnail_QIDI` (ColPic).
- `parse_thumbnails_key`: parse the Orca-format `thumbnails` config key (`"WxH/EXT,WxH/EXT"`, e.g. `"48x48/PNG,300x300/PNG"`; EXT case-insensitive among PNG/JPG/QOI/BTT_TFT/COLPIC) into `Vec<ThumbnailSpec>`; reject malformed entries with an error naming the entry.
- Decode + rescale + encode: decode the single source PNG, rescale per entry, and encode PNG/JPG/QOI (base64 block entries), ColPic (port of `ColPic_EncodeStr` — RGB565 palette/RLE, 512px cap aspect-preserved, `;gimage:` first chunk / `;simage:` subsequent), and BTT_TFT (RGB565 hex text, `;<WWWW><HHHH>` header, `\r\n` line endings). New image-processing dependency for slicer-gcode (choice + licensing recorded in `design.md`).
- Row order: source PNGs arriving from the fork are already top-down (PNG is the only Orca encoder that does not flip the GL buffer) — transcoders must NOT re-flip. Covered by AC-6.
- Default behaviour without a `thumbnails` key: one `thumbnail` (PNG) entry at source dimensions, source bytes passed through un-re-encoded.
- Wiring in `crates/slicer-runtime/src/pipeline.rs::run_postpass_with_thumbnail`: read the `thumbnails` raw-config key, render all entries, hand rendered entries to `ThumbnailAwareSerializer`. `thumbnails` stays visible in CONFIG_BLOCK (it is a real config key, unlike the stripped invocation-time `thumbnail_path`).
- Attribution headers (per `docs/ORCASLICER_ATTRIBUTION.md`) on the ported ColPic and BTT_TFT codec files; canonical citations by file + function only.
- Rewrite roundtrip/chunking assertions in `crates/slicer-runtime/tests/integration/gcode_header_thumbnail_config_blocks_tdd.rs` — the packet-55 tests `thumbnail_roundtrip_matches_input` (line 584) and `thumbnail_base64_chunking_orca_parity` — to parse the real inner-framed format (names kept; only assertions change).
- Fork-facing contract note (docs/02) + deviation row `D-173-THUMBNAIL-SINGLE-PNG` (DEVIATION_LOG) flagging the single-source-PNG deviation from fork ticket 011.

## Out of Scope

- Any CLI change: `--thumbnail` remains a single `Option<PathBuf>` (`crates/pnp-cli/src/main.rs:70-72`). No `--thumbnail-size`/multi-flag surface.
- The fork's side of the contract (rendering the high-res PNG, populating the `thumbnails` key in 3MF/raw config).
- Printer-profile-derived default `thumbnails` values; `thumbnails_format` key handling.
- Streaming/size-limit policing of G-code output; WIT/IR/module/guest-WASM changes.
- Non-PNG `--thumbnail` sources (PNG magic validation in `pipeline.rs` is unchanged).

## Authoritative Docs

- `docs/02_ir_schemas.md` — large; delegated bounded lookup of the CONFIG_BLOCK / THUMBNAIL_BLOCK section only.
- `docs/ORCASLICER_ATTRIBUTION.md` — short; direct read for the porting header.
- `docs/specs/fork-gaps-wave2-plan.md` §Packet 173 — direct read; user-decided contract source.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/GCode/Thumbnails.cpp` — `ColPic_EncodeStr` (plus internal `ColPicEncode`, `Byte8bitEncode`, `ADList0`), `compress_thumbnail_btt_tft` (with `get_hex`, `rjust`), `compress_thumbnail_colpic` (512px cap, aspect preserved), the `compress_thumbnail` dispatch switch, and `make_and_check_thumbnail_list` (the `"WxH/EXT,..."` parser).
- `OrcaSlicerDocumented/src/libslic3r/GCode/Thumbnails.hpp` — `export_thumbnails_to_file` (the `; <tag> begin WxH len` / `; <tag> end` writer, `max_row_length = 78`) and the per-format `tag()` overrides (`thumbnail`, `thumbnail_JPG`, `thumbnail_QOI`, `thumbnail_BIQU`, `thumbnail_QIDI`).

## Acceptance Summary

- Positive: `AC-1` through `AC-7`. Refinement absent from the AC text: AC-2's decoded-PNG dimension check reads IHDR bytes 16-23 (big-endian width/height) — no full image decode needed in the test.
- Negative: `AC-N1` (malformed `thumbnails` key rejected, entry named), `AC-N2` (no source PNG → no block, even with a `thumbnails` key).
- Cross-packet impact: none structural. Packet 171 (gcode-flavor-writer, draft) also touches `slicer-gcode/src/serialize.rs`; overlap is textual only (independent functions) — coordinate merge order at swarm time.

## Verification Commands

This is the authoritative full matrix; `packet.spec.md` lists only the gate commands.

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `mkdir -p target && cargo test -p slicer-gcode --test thumbnail_formats_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"` | Codec + parser unit contract (AC-4/5/6/N1) | FACT pass/fail; SNIPPETS ≤20 lines on failure |
| `mkdir -p target && cargo test -p slicer-runtime --test integration gcode_header_thumbnail_config_blocks 2>&1 | tee target/test-output.log | grep -E "^test result"` | Wire format + multi-entry pipeline integration (AC-1/2/3/N2) | FACT pass/fail; SNIPPETS ≤20 lines on failure |
| `cargo check --workspace --all-targets` | Whole-tree type gate incl. tests | FACT pass/fail |
| `cargo clippy --workspace --all-targets -- -D warnings` | Lint gate (required before commit) | FACT pass/fail |
| `rg -q 'thumbnail begin' docs/02_ir_schemas.md && rg -q 'D-173-THUMBNAIL-SINGLE-PNG' docs/DEVIATION_LOG.md && echo PASS` | Doc-impact greps (AC-7) | FACT PASS/absent |

## Step Completion Expectations

The wire-format rewrite (Step 2) intentionally breaks the two existing integration assertions rewritten in Step 5; between those steps `cargo test -p slicer-runtime --test integration gcode_header_thumbnail_config_blocks` is expected RED and must not be "fixed" by weakening the new format. All other cross-step state is per-step.

## Context Discipline Notes

- `crates/slicer-runtime/tests/integration/gcode_header_thumbnail_config_blocks_tdd.rs` is 848 lines — read only the helper block (~lines 140-210) and the roundtrip/chunking tests (~lines 580-650); never the full file.
- `crates/slicer-runtime/src/pipeline.rs` — read only `run_postpass_with_thumbnail` (~lines 412-480).
- All `OrcaSlicerDocumented/` reads delegated (see obligations above).
