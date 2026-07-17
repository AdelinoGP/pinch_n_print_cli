# Plan: Wave-2 PNP packets for the OrcaSlicer-frontend fork

## Context

Wave 1 (`docs/specs/fork-gaps-wave1-plan.md`) generated packets 166–170 (all currently `draft`; none swarmed). This session packets the **remaining handoff items** from `OrcaSlicerDocumented/.wayfinder/assets/handoff-pnp-gap-implementation.md`: 3 (raft), 4 (MM proof), 5 (flavor), 9 (per-object keys), 11 (cancel), 13 (support preview), 14 (thumbnails), 15 (M73). The fork ships no UI gates and no user-visible warnings, so every gap fails silently — all items must actually land.

Grilled decisions (2026-07-17) are recorded below; deliverable is **6 new spec packets (171–176)** authored via `/spec-packet-generator` Batch Protocol, each gated with `/spec-review <packet> --preflight`.

## Grounding facts (verified this session)

- `SupportPlanIR.raft_plan` does not exist in source; draft packet `124_support-plan-raft-plan-and-raftinfill-role` owns that seam and explicitly excludes the `raft-default` module.
- TASK-210/211/212 all open in `docs/07_implementation_status.md:137-139`; MM model is filament-index-based (wipe-tower keys off `ToolChange.to_tool`), matching the fork's 1-nozzle/N-filament population. No real-fixture T0/T1 E2E exists (synthetic only). User has manually verified painted-3MF → correct-color G-code in Orca's viewer.
- Object-metadata allowlist = `object_metadata_to_config_data` (`crates/slicer-model-io/src/loader.rs:730-771`), exactly 3 keys; sidecar parser (`sidecar.rs`) captures all keys verbatim, filtering happens in the loader.
- Emit is pure Marlin literals (M104/M109, M106, M82/M83, T\<n\>, G11); zero flavor abstraction; `gcode_flavor` appears only as cosmetic padding (`serialize.rs:403`). No bed-temp/accel/jerk emission exists.
- `crates/pnp-cli` has zero signal handling; natural cancel checkpoint = module-execution loop over `global_layers` in `slicer-runtime/src/run.rs`.
- Thumbnail block omits Orca's inner `; thumbnail begin WxH len` / `; thumbnail end` lines (`thumbnail.rs:41-43`); CLI is single `Option<PathBuf>` (`pnp-cli/src/main.rs:70-72`).
- No M73 anywhere; packet 169 (estimator, `draft`) explicitly excludes M73 and names it as the wave-2 unblock.

## Item 3 (raft) — NO new packet (user decision)

Item 3 = run existing draft packet **124** as-is. The raft-default synthesizer module packet (and the open carrier decision, synthetic-layers vs RaftRegionIR) is deferred to a future wave; 124's IR seam is the wave-2 deliverable. Consequence acknowledged: no user-visible raft until the synthesizer packet exists.

## Packet 171 — gcode-flavor-writer (item 5)

- **Full 5-flavor support** (marlin, marlin2, klipper, reprapfirmware, repetier) via a **port of Orca's `GCodeWriter` per-flavor logic** — including commands PNP doesn't emit yet (M204/accel family etc.) so future emit features are flavor-correct from day one.
- `GcodeFlavor` enum parsed from config (default marlin); dialect layer in `crates/slicer-gcode/src/serialize.rs`; CONFIG_BLOCK echoes the real flavor instead of the padded literal.
- OrcaSlicer attribution header required; cite canonical by file+function (`GCodeWriter.cpp::set_temperature` etc.), never line numbers.

## Packet 172 — mm-e2e-and-object-keys (items 4 + 9, mega-packet: TASK-210 + 211 + 212)

- TASK-210: route `support_filament`/`support_interface_filament` through G-code emit so supports select their extruder.
- TASK-211: real-fixture T0/T1 G-code E2E (user's painted Orca 3MF fixtures exist and were manually verified — codify them).
- TASK-212 + item 9: **extend the existing hand-written match** in `object_metadata_to_config_data` (no data-driven table) with the Orca per-object/per-volume keys the fork writes; unknown keys logged rather than silently dropped where cheap.

## Packet 173 — thumbnails-multiformat (item 14, one packet, all formats)

- Fix wire format: inner `; thumbnail begin <WxH> <len>` / `; thumbnail end` lines (tag per format: `thumbnail`/`thumbnail_JPG`/`thumbnail_QOI`/`thumbnail_BIQU`/`thumbnail_QIDI`), 78-col wrap, outer sentinels retained; fix the roundtrip test to parse the real format.
- **Contract deviation from fork ticket 011 (user-decided, must be flagged to the fork):** CLI keeps a **single** `--thumbnail <png>`; requested sizes/formats come from the `thumbnails` config key (`"WxH/EXT,..."`) via raw_config/3MF. **PNP decodes + rescales** the source PNG per entry (new image dependency, e.g. `image` crate) and encodes all five formats: PNG/JPG/QOI base64 + ColPic (`ColPic_EncodeStr` port, 512px cap) + BTT_TFT (RGB565 hex, `\r\n`). Attribution headers for the `Thumbnails.cpp` ports. Row order: incoming PNGs are top-down; transcoders must not re-flip.
- Packet must include updating the fork-facing contract doc note (the fork now renders ONE high-res PNG, not one per size).

## Packet 174 — graceful-cancel (item 11)

- Handle CTRL_BREAK_EVENT/SIGINT (`ctrlc` crate) **and** stdin EOF as cancel; AtomicBool checked at the module-execution/per-layer checkpoint in `run.rs`; emit a `cancelled` JSONL progress event, remove partial output, exit with a distinct code. Windows-first.

## Packet 175 — m73-progress (item 15) — depends_on 169

- Full set: `M73 P<pct> R<min>` **and** stealth `M73 Q<pct> S<min>` (same estimate for both), at layer boundaries + start/end; `; filament used [g]/[mm]/[cm3]` + `; estimated printing time` comment blocks; honor `disable_m73` when present. Strictly downstream of packet 169's estimator — do not activate before 169 is implemented.

## Packet 176 — support-preview-verb (item 13, full implementation)

- New `pnp_cli support-preview --input <3mf> --output <path>` verb: runs the pipeline through the support stage only (no G-code emit), writes per-layer support polygons (ExPolygons + z) as JSON/JSONL. Fork renders the overlay itself. Reuses existing prepass/partial-pipeline machinery. Output schema is a fork-facing contract — document it.

## Packet Queue (for /spec-packet-generator Batch Protocol)

| # | packet slug | goal (one sentence) | task ids | depends on | status | packet dir |
|---|-------------|---------------------|----------|------------|--------|------------|
| 1 | 171-gcode-flavor-writer | Port Orca GCodeWriter per-flavor logic (5 flavors) into slicer-gcode with a GcodeFlavor enum honored from config and echoed in CONFIG_BLOCK. | TASK-276 (new) | - | generated | `.ralph/specs/171-gcode-flavor-writer/` |
| 2 | 172-mm-e2e-and-object-keys | Close TASK-210/211/212 + item 9: support-filament routing, real-fixture T0/T1 E2E, extended Orca per-object key allowlist. | TASK-210, TASK-211, TASK-212 (item 9 folded into TASK-212) | - | generated | `.ralph/specs/172-mm-e2e-and-object-keys/` |
| 3 | 173-thumbnails-multiformat | Orca-parseable thumbnail wire format + config-driven multi-entry generation (PNG/JPG/QOI/ColPic/BTT) from one CLI PNG via PNP-side resize (fork-contract deviation flagged). | TASK-277 (new) | - | generated | `.ralph/specs/173-thumbnails-multiformat/` |
| 4 | 174-graceful-cancel | CTRL_BREAK/SIGINT + stdin-EOF graceful cancel with cancelled JSONL event, partial-output cleanup, distinct exit code. | TASK-278 (new) | - | generated | `.ralph/specs/174-graceful-cancel/` |
| 5 | 175-m73-progress | Emit M73 P/R + Q/S progress and filament-used/estimated-time comment blocks off packet 169's estimator; honor disable_m73. | TASK-279 (new) | 169 (wave 1, draft — hard prerequisite) | generated | `.ralph/specs/175-m73-progress/` |
| 6 | 176-support-preview-verb | New pnp_cli support-preview verb running the pipeline through the support stage and emitting per-layer support polygons as JSON (fork-facing contract). | TASK-280 (new) | - | generated | `.ralph/specs/176-support-preview-verb/` |

## Domain model / glossary (CONTEXT.md, at packet acceptance — not now)

**G-code Flavor / Dialect**, **Thumbnail Entry** (size+format request from `thumbnails` key; single-source-PNG resize contract), **Graceful Cancel contract** (signal/stdin-EOF → `cancelled` event → distinct exit code), **Support Preview verb** (partial-pipeline geometry-out). ADR candidates: the thumbnail single-PNG-resize contract (reverses a fork-ticket-011 decision — surprising without context) and the support-preview output schema.

## Verification

- Per packet: narrow tests per Test Discipline, tee to `target/test-output.log`; `cargo clippy --workspace --all-targets -- -D warnings`; `cargo xtask build-guests --check` after any WIT/module-adjacent edit (172 touches loader only — likely exempt; 171/173/175 are host-side slicer-gcode; 176 touches runtime entry points).
- E2E: 171 — slice benchy per flavor, assert dialected commands + CONFIG_BLOCK flavor; 172 — painted fixture slices to T0/T1 with correct support filament; 173 — G-code parseable by Orca's `; thumbnail begin` reader shape, all configured entries present at correct dimensions; 174 — kill via stdin-close mid-slice, assert `cancelled` event + no partial file; 175 — monotonic P ascending, R descending to 0; 176 — verb emits valid JSON polygons for a supported fixture without writing G-code.
- Packet-close: `cargo xtask test --workspace` at each acceptance ceremony via sub-agent, FACT pass/fail.

## Next step after approval

Run `/spec-packet-generator` in Batch Protocol mode over this plan to author packets 171–176, then `/spec-review --preflight` each. Also append/refresh this plan as `docs/specs/fork-gaps-wave2-plan.md` (team-visible, per shared-memory rule).
