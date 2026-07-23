# Task Map: 135_gyroid-raw-emit

Single-task-ID packet (`TASK-260`); the map is retained because the preflight gate (S0)
requires all five contract files. Backlog row: `TASK-260` in `docs/07_implementation_status.md`
(line 226; currently `- [ ]` open). DEV-082 row in `docs/DEVIATION_LOG.md` is the
recorded divergence this packet realizes (Open since 2026-07-03).

| docs/07 task ID | Packet step | Primary docs | Expected code surface | OrcaSlicer refs | Context cost | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| `TASK-260` | `Step 0` | `docs/07_implementation_status.md` rows 224 (TASK-258), 225 (TASK-259), 226 (TASK-260), 223 (TASK-257) | none (read-only verification) | none | S | Pre-activation dependency check: 4 functions to delete (lib.rs:551, 570, 585, 611), 1 claim in manifest, 1 expand-factor at lib.rs:259. |
| `TASK-260` | `Step 1` | `docs/adr/0027-gyroid-multi-role-fill-holder.md` + `docs/DEVIATION_LOG.md` DEV-082 | `modules/core-modules/gyroid-infill/gyroid-infill.toml` (3 claims), `modules/core-modules/gyroid-infill/tests/gyroid_infill_tdd.rs` (6 new tests RED — 5 AC tests + 1 regression helper `adjacent_layers_have_phase_coherent_bbox`) | none | M | RED: 6 new tests; rotation-block-affected test inventory in test file header. |
| `TASK-260` | `Step 2` | spec §Phase 3 + `docs/08_coordinate_system.md` (mm-domain for `gyroid_f`) | `modules/core-modules/gyroid-infill/src/lib.rs` (rotation block at lib.rs:344, expand at lib.rs:259, 4 deletions lib.rs:551/570/585/611, `align_to_grid` helper) | FillGyroid.cpp:300-376 (delegate), FillGyroid.cpp:322, FillGyroid.cpp:326 (delegate) | M | GREEN: AC-1..AC-4, AC-6, AC-N1. Wave-core byte-identical at lib.rs:394, 430, 491. |
| `TASK-260` | `Step 3` | `.ralph/specs/131_per-region-config-delivery/carve-list.md` | `crates/slicer-runtime/tests/executor/cube_4color_*` (carve-list append, likely empty for this packet) | none | S | Gates: `cargo xtask build-guests --check`, `cargo check --workspace --all-targets`, `cargo clippy`. |

Aggregate context cost: `M` (S + M + M + S). No step rated `L`.

## Pre-activation FORWARD-DEPs (consumer name/shape matches producer state)

- The four `perimeter-region-view` partition fields are produced by packet 130 (TASK-255
  closed 2026-07-17) at `crates/slicer-sdk/src/views.rs:103-108`. This packet reads
  them through the same accessor pattern used by packet 134. Names and shapes match.
- The per-region config accessor is produced by packet 131 (TASK-256 closed 2026-07-19).
  The module reads through the SDK region accessor inside its region loop. Names and
  shapes match.
- The `clip_polylines` helper in `slicer-core::polygon_ops` is produced by packet 129
  (TASK-254 closed 2026-07-16). The module does NOT call it; the linker (packet 133,
  TASK-258, currently OPEN) is the consumer. Names and shapes match.
- The four fill claims `claim:(sparse|top|bottom|bridge)-fill` exist in
  `docs/03_wit_and_manifest.md` and `crates/slicer-scheduler/src/validation.rs:11-15`
  (`FILL_CLAIM_IDS`). Packet 135 adds three to gyroid's `claims.holds`; no scheduler
  change. Names and shapes match.
- The linker (packet 133, TASK-258, currently OPEN) is the consumer of this packet's
  raw wave output. Until 133 lands, the user-visible print is degraded per ADR-0025's
  degraded-not-failed trade-off. This is NOT a packet-blocker.
