# Design: 167-config-block-viewer-keys

## Controlling Code Paths

- Primary code path: `ThumbnailAwareSerializer` (used by `run_pipeline_with_raw_config`) → `serialize_config_block(raw_config, filament_colour_csv)` (`crates/slicer-gcode/src/serialize.rs:283-382`): synthesizes `filament_diameter`/`filament_colour`/`extruder_colour` when absent (309-328), dumps raw_config keys sorted (330-364), then pads from `ORCA_CONFIG_PADDING` until `emitted.len() >= 96` (373-379). Dedup: `emit_config_kv` inserts into a `BTreeSet<String>` and skips already-emitted keys (386-395). Padding table: `ORCA_CONFIG_PADDING` (402-475, 72 entries).
- Neighboring tests/fixtures: `crates/slicer-runtime/tests/integration/gcode_header_thumbnail_config_blocks_tdd.rs` (765 lines) — has `region_between` helper, duplicate-key test (AC-9 around line 468), block-ordering test (around line 571); new tests append here.
- OrcaSlicer comparison: no port. Upstream behavior cited by function: `ConfigBase::load_from_gcode_file` (`Config.cpp`) rejects blocks under ~80 pairs; `GCodeProcessor::apply_config` (`GCodeProcessor.cpp`) consumes machine-limit and accel/jerk keys for time estimation; `s_IsBBLPrinter` defaults toward Bambu behavior when `printer_model` is absent on drag-in. Cite by file + function only; never line numbers.

## Architecture Constraints

- CONFIG_BLOCK is part of the normative G-code envelope contract (`docs/02_ir_schemas.md`, "G-code envelope blocks (Normative — packet 55)"): `CONFIG_BLOCK_*` stays the final semicolon-prefixed content; block ordering must not change.
- Golden-output hazard: `crates/slicer-runtime/tests/fixtures/golden/precision_legacy_20mmbox.gcode` contains a CONFIG_BLOCK; changing padding changes its bytes. The golden must be re-blessed in this packet with the diff reviewed (only CONFIG_BLOCK lines may differ — motion lines byte-identical). Any motion-line diff falsifies the packet.
- No guest-WASM impact: `crates/slicer-gcode` is not in CLAUDE.md's guest-input path list; no `cargo xtask build-guests --check` obligation beyond normal hygiene.

## Code Change Surface

- Selected approach: minimal surgical rework of the padding table plus one synthesis branch, keeping the emission machinery untouched.
- Exact changes in `crates/slicer-gcode/src/serialize.rs`:
  1. **Remove these 34 entries from `ORCA_CONFIG_PADDING`** (the complete speed/accel/jerk-valued class as grounded): `travel_speed`, `travel_speed_z`, `initial_layer_speed`, `initial_layer_infill_speed`, `sparse_infill_speed`, `internal_solid_infill_speed`, `top_surface_speed`, `gap_infill_speed`, `bridge_speed`, `small_perimeter_speed`, `overhang_1_4_speed`, `overhang_2_4_speed`, `overhang_3_4_speed`, `overhang_4_4_speed`, `default_acceleration`, `outer_wall_acceleration`, `inner_wall_acceleration`, `initial_layer_acceleration`, `top_surface_acceleration`, `bridge_acceleration`, `sparse_infill_acceleration`, `travel_acceleration`, `default_jerk`, `outer_wall_jerk`, `inner_wall_jerk`, `infill_jerk`, `top_surface_jerk`, `initial_layer_jerk`, `travel_jerk`, `slow_down_min_speed`, `ironing_speed`, `internal_bridge_speed`, `support_speed`, `support_interface_speed`.
  2. **Add ~45 neutral replacement entries** so the table alone (plus the 3 synthesized filament keys and `printer_model`) reaches ≥80 emitted lines with empty raw_config. Neutral = keys the viewer's `GCodeProcessor` does not feed into motion/time computation: pattern/enum/toggle/count/geometry-cosmetic keys (e.g. `wall_loops`, `top_shell_layers`, `infill_direction`, `wall_generator`, `ironing_pattern`, `support_type`, `support_style`, `interface_shells`, `seam_slope_type`, retraction toggles at benign values, etc.). Pick from `docs/ORCA_CONFIG_REFERENCE.md` upstream defaults via delegated lookup; every added value must equal the upstream default. Never add a key matching `machine_max_*`, `*speed*`, `*acceleration*`, `*jerk*` (AC-1 grep is the gate).
  3. **Synthesize `printer_model`**: in `serialize_config_block`, alongside the existing `filament_diameter` synthesis block (serialize.rs:309-312 pattern), add `if !raw_config.contains_key("printer_model") { emit_config_kv(&mut out, &mut emitted, "printer_model", "Generic PNP Printer"); }`.
  4. Update the padding-table doc comment (serialize.rs:397-401) and the padding-loop comment (366-372) to state the machine-limit/speed exclusion invariant and cite the fork contract subsection in `docs/02_ir_schemas.md`.
- Test changes in `crates/slicer-runtime/tests/integration/gcode_header_thumbnail_config_blocks_tdd.rs`: three new tests — `config_block_meets_orca_minimum_key_gate` (count `; key = value` lines ≥80 via `region_between`), `config_block_synthesizes_non_bbl_printer_model` (exactly one `; printer_model = Generic PNP Printer`; no `Bambu` substring in the block's printer_model line), `config_block_fork_keys_never_shadowed` (pass `machine_max_acceleration_extruding=20000` and `printer_model=MyFork Printer` in raw_config; assert each appears exactly once with the supplied value).
- Doc change: new subsection "CONFIG_BLOCK viewer-key contract" appended inside "G-code envelope blocks" in `docs/02_ir_schemas.md`, listing fork-required keys (`printer_model`, `filament_density`, `filament_cost`, `printable_area`, `nozzle_diameter`, `machine_max_*` family), stating that PNP padding never emits speed/accel/jerk/machine-limit values, and that PNP synthesizes `printer_model = Generic PNP Printer` only when absent.
- Rejected alternatives:
  - Dropping padding entirely and requiring the fork to supply 80+ keys: breaks every non-fork `pnp_cli` user's viewer preview; the gate would fail on plain CLI slices.
  - Emitting PNP's own resolved speeds/accels as "real" values: PNP's config keys are not 1:1 with Orca's viewer keys and would still be wrong for the fork's printer; the contract (fork supplies real values) is the correct boundary.

## Files in Scope (read + edit)

- `crates/slicer-gcode/src/serialize.rs` — role: owns `ORCA_CONFIG_PADDING` and `serialize_config_block`; expected change: table rework + printer_model synthesis + comment updates.
- `crates/slicer-runtime/tests/integration/gcode_header_thumbnail_config_blocks_tdd.rs` — role: CONFIG_BLOCK integration coverage; expected change: three appended tests.
- `docs/02_ir_schemas.md` — role: normative envelope contract; expected change: one appended subsection.
- (`crates/slicer-runtime/tests/fixtures/golden/precision_legacy_20mmbox.gcode` — conditional 4th: re-bless if its golden test compares CONFIG_BLOCK bytes; verify via the golden test run first.)

## Read-Only Context

- `crates/slicer-gcode/src/serialize.rs` (807 lines) — lines 200-480 only.
- `crates/slicer-runtime/tests/integration/gcode_header_thumbnail_config_blocks_tdd.rs` (765 lines) — lines 1-120 (harness/helpers) and 420-500 (AC-8/AC-9 patterns) only.
- `docs/02_ir_schemas.md` (1811 lines) — lines 1660-1720 only.

## Out-of-Bounds Files

- `docs/ORCA_CONFIG_REFERENCE.md` (2404 lines) — delegate LOCATIONS/FACT lookups for replacement-key defaults; never load.
- `OrcaSlicerDocumented/` — delegate; never load.
- `crates/slicer-runtime/src/**` (pipeline wiring is untouched); `.claude/worktrees/**`; `target/`, `Cargo.lock`, generated code, vendored dependencies — never load.

## Expected Sub-Agent Dispatches

- Question: "For candidate neutral padding keys <list>, what are OrcaSlicer's upstream defaults per docs/ORCA_CONFIG_REFERENCE.md, and are any consumed by GCodeProcessor for motion/time computation?"; scope: `docs/ORCA_CONFIG_REFERENCE.md`; return: `FACT` per key (≤5 lines each batch); purpose: Step 2 replacement-key selection.
- Question: "Which golden/e2e tests assert CONFIG_BLOCK bytes or line counts?"; scope: `crates/slicer-runtime/tests`, `crates/slicer-gcode/tests`; return: `LOCATIONS` ≤20; purpose: Step 4 re-bless inventory.
- All `cargo` invocations dispatched with `FACT pass/fail` returns.

## Data and Contract Notes

- IR/manifest contracts: none. CONFIG_BLOCK is a wire-format (G-code text) contract documented in `docs/02_ir_schemas.md`.
- WIT boundary: none.
- Determinism: key emission stays sorted/deterministic (`BTreeSet` + sorted raw keys + fixed table order). The `emitted.len() >= 96` stop condition is retained unchanged.

## Locked Assumptions and Invariants

- `emit_config_kv`'s insert-or-skip dedup is the single shadowing guard; the printer_model synthesis must run through it (and, like the filament synthesis, is additionally guarded by `raw_config.contains_key`).
- Grounded 2026-07-17: `ORCA_CONFIG_PADDING` has 72 entries at serialize.rs:402-475; the padding loop gate is `emitted.len() >= 96` at serialize.rs:374; `printer_model` occurs nowhere in `crates/slicer-gcode`. Re-verify before editing if the file has moved.
- The wave-1 plan's claim that padding emits `machine_max_*` keys was falsified; the packet's contract is strengthened to "never emit them" rather than "remove them".

## Risks and Tradeoffs

- Risk: a "neutral" replacement key is actually consumed by the viewer's processor. Mitigation: every candidate is checked against `docs/ORCA_CONFIG_REFERENCE.md` via dispatch and excluded if speed/accel/jerk/machine-limit-typed; AC-1's grep enforces the name classes mechanically.
- Risk: golden `.gcode` fixtures churn. Mitigation: Step 4 inventories and re-blesses with a motion-lines-identical check.
- Tradeoff: `Generic PNP Printer` appears in the viewer's config panel for non-fork CLI users; accepted as strictly better than an absent key triggering Bambu-mode heuristics.

## Context Cost Estimate

- Aggregate: `S`
- Largest step: `S` (Step 2, padding rework)
- Highest-risk dispatch and required return format: replacement-key default lookup over `docs/ORCA_CONFIG_REFERENCE.md`; batched `FACT` returns, ≤5 lines per key batch.

## Open Questions

- [FWD] Exact choice of the ~45 neutral replacement keys is implementer-resolved via the ORCA_CONFIG_REFERENCE dispatch; the AC-1 name-class grep and AC-2 count test are the binding constraints, not a fixed key list.
- [FWD] If `precision_legacy_20mmbox.gcode`'s golden comparison excludes the CONFIG_BLOCK region, no re-bless is needed — Step 4's inventory decides.
