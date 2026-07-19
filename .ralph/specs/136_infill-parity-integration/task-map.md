# Task Map: 136_infill-parity-integration

Single-task-ID packet (`TASK-261`); the map is retained because the preflight gate (S0)
requires all five contract files. Backlog row: `TASK-261` in `docs/07_implementation_status.md`
(line 227; currently `- [ ]` open).

| docs/07 task ID | Packet step | Primary docs | Expected code surface | OrcaSlicer refs | Context cost | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| `TASK-261` | `Step 0` | `docs/07_implementation_status.md` rows 223-226 (TASK-257/258/259/260 must be `[x]`) | none (read-only verification) | none | S | Hard pre-activation gate. Refuse activation if any of TASK-257/258/259/260 are still `- [ ]`. |
| `TASK-261` | `Step 1` | `docs/specs/modifier-region-infill.md` §Phase M3, `docs/specs/infill-parity-rectilinear-gyroid-linker.md` §Phase 5 | `resources/cube_cilindrical_modifier.3mf` sidecar extension (preferred), OR `resources/cube_infill_modifier.3mf` (new), OR programmatic 3MF in-test; loader smoke test | none | S | Fixture decision: extend sidecar `Metadata/model_settings.config` is the cheapest path; existing 3MF is 30625 bytes. |
| `TASK-261` | `Step 2` | spec §Phase 5, `docs/specs/modifier-region-infill.md` §M3 assert list, `tests/e2e/scenario_traces_tdd.rs:336-365` (degraded-state pattern) | `crates/slicer-runtime/tests/e2e/modifier_infill_tdd.rs` (new), `crates/slicer-runtime/tests/e2e/wedge_linked_infill_report_tdd.rs` (new), `crates/slicer-runtime/tests/integration/no_linker_module_degraded_raw_output_tdd.rs` (new) | none | M | RED→GREEN: AC-1, AC-2, AC-3, AC-N1. |
| `TASK-261` | `Step 3` | `crates/slicer-ir/tests/fill_holder_cli_binding_tdd.rs` (pattern) + `crates/slicer-ir/src/resolved_config.rs:99-112` (production site) | `crates/slicer-ir/src/resolved_config.rs` (1 key: `infill_overlap`); `crates/slicer-ir/tests/infill_overlap_cli_binding_tdd.rs` (new) | none | S | CLI binding test (3 precedent tests, 66 lines). |
| `TASK-261` | `Step 4` | `.ralph/specs/131_per-region-config-delivery/carve-list.md` | 5 carved test files in `crates/slicer-runtime/tests/executor/cube_4color_*` (and `cube_4color_arachne.rs`) — marker removal + re-bless | none | M | Golden restore: ~20 carved tests; wedge digest canary `8a3b645ee54fa5dbfa1232008db4820d2a364a30b4d196a504b424271308019f` (131's AC-N2). |
| `TASK-261` | `Step 5` | `CLAUDE.md` §Test Discipline (ceremony contract) | `docs/07_implementation_status.md` (TASK-257/258/259/260/261 rows flipped; 254/255/256 already closed) | none | S | Acceptance ceremony + closure sweep. All delegated. |

Aggregate context cost: `M` (S + S + M + S + M + S). No step rated `L`.

## Pre-activation FORWARD-DEPs (consumer name/shape matches producer state)

- The 5 carved files in `crates/slicer-runtime/tests/executor/cube_4color_*` (and
  `cube_4color_arachne.rs`) are produced by packet 131's `carve-list.md` (TASK-256
  closed 2026-07-19). Names and shapes match. The 131 carve-list is the worklist.
- The `wedge_per_region_config_delivery_byte_identical` test (131's AC-N2, digest
  `8a3b645ee54fa5dbfa1232008db4820d2a364a30b4d196a504b424271308019f`) is the regression
  canary for byte-identical output on `regression_wedge.stl` (single-region, NOT
  carved). Names and shapes match.
- The `apply_cli_key` mechanism in `resolved_config.rs:99-112` is the production site
  for `fill_holder` bindings (TASK-256). Packet 136 adds `infill_overlap` alongside
  via the same mechanism. Names and shapes match.
- The `is_degraded()` mechanism on the slice event collector is the precedent for
  AC-N1 (no-linker degraded guard). Names and shapes match.
- Packet 133 (TASK-258, currently OPEN) must add `claim:infill-link` to the manifest
  catalog + scheduler. The packet text assumes this; without it, the linker's
  first-winner dedup fails and the AC-2/AC-3 assertions are vacuous. Blocked at Step 0
  if 133 is not closed.
- Packet 132 (TASK-257, currently OPEN) must provide the modifier-volume geometric
  region split that AC-1's two-density assertion depends on. Blocked at Step 0 if 132
  is not closed.
- Packet 134/135 (TASK-259/260, currently OPEN) provide the raw emit algorithms.
  Blocked at Step 0 if either is not closed.
