# Task Map: 134_rectilinear-raw-emit

Single-task-ID packet (`TASK-259`); the map is retained because the preflight gate (S0)
requires all five contract files. Backlog row: `TASK-259` in `docs/07_implementation_status.md`
(line 225; currently `- [ ]` open).

| docs/07 task ID | Packet step | Primary docs | Expected code surface | OrcaSlicer refs | Context cost | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| `TASK-259` | `Step 0` | `docs/07_implementation_status.md` rows 224 (TASK-258), 225 (TASK-259), 223 (TASK-257) | none (read-only verification) | none | S | Pre-activation dependency check: WIT contract + per-region config + structural state. |
| `TASK-259` | `Step 1` | `docs/specs/infill-parity-rectilinear-gyroid-linker.md` §Phase 2 | `modules/core-modules/rectilinear-infill/tests/rectilinear_raw_emit_tdd.rs` (new) | none | M | RED: 7 new tests in a new file; stale-geometry inventory in test file header. |
| `TASK-259` | `Step 2` | spec §Phase 2 + `docs/08_coordinate_system.md` (rotation rounding ≤ 50 nm) | `modules/core-modules/rectilinear-infill/src/lib.rs` (rotation block, per-ExPolygon scan, `infill_direction` port) | FillRectilinear.cpp:842-1154 (delegate), FillBase.cpp:352-391 (delegate) | M | GREEN: AC-1/2/3/4/6/N1; structural deletion grep for `fill_expolygon_multi` + `collect_edges`. |
| `TASK-259` | `Step 3` | spec §Phase 2 | same `lib.rs` (solid spacing + `pattern_shift`); same test file (AC-5, AC-7) | FillBase.cpp:326-340, FillRectilinear.cpp:3023-3024 (delegate) | S | GREEN: AC-5, AC-7. Wave-core byte-identical. |
| `TASK-259` | `Step 4` | `docs/ORCASLICER_ATTRIBUTION.md` + `.ralph/specs/131_per-region-config-delivery/carve-list.md` | `modules/core-modules/rectilinear-infill/tests/rectilinear_infill_tdd.rs`, `rectilinear_infill_edge_cases_tdd.rs` (stale-test rewrites with bug-naming headers); carve-list (append-only) | none | M | Stale-test reconciliation + workspace gates; `cargo xtask build-guests --check`. |

Aggregate context cost: `M` (S + M + M + S + M). No step rated `L`.

## Pre-activation FORWARD-DEPs (consumer name/shape matches producer state)

- The four `perimeter-region-view` partition fields (`sparse_infill_area`,
  `top_solid_fill`, `bottom_solid_fill`, `bridge_areas`) are produced by packet 130
  (TASK-255, closed 2026-07-17) at `crates/slicer-sdk/src/views.rs:103-108` — the
  consumer's accessor pattern is verified at `rectilinear-infill/src/lib.rs:108-109, 120,
  139, 158, 178-179`. Names and shapes match.
- The per-region config accessor is produced by packet 131 (TASK-256, closed 2026-07-19) —
  the consumer reads through the SDK region accessor inside its region loop. Names and
  shapes match.
- The `clip_polylines` helper in `slicer-core::polygon_ops` is produced by packet 129
  (TASK-254, closed 2026-07-16). The module does NOT call it (raw emit only); the
  linker (packet 133, currently OPEN) is the consumer. Names and shapes match.
- The linker (packet 133, TASK-258, currently OPEN) is the consumer of this packet's
  raw 2-point output. Until 133 lands, the user-visible print is degraded per ADR-0025's
  degraded-not-failed trade-off. This is NOT a packet-blocker; the packet ships
  standalone.
