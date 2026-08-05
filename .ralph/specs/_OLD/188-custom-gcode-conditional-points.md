---
status: implemented
packet: 188-custom-gcode-conditional-points
task_ids:
  - TASK-307
---

# 188-custom-gcode-conditional-points

## Goal

Land every remaining OrcaSlicer custom-G-code injection point that PnP's pipeline can actually reach — the toolchange trio `filament_end_gcode` / `change_filament_gcode` / `filament_start_gcode`, spliced around each `GCodeCommand::ToolChange` in canonical `GCode::set_extruder` order, and the extrusion-role trio `change_extrusion_role_gcode` / `filament_change_extrusion_role_gcode` / `process_change_extrusion_role_gcode`, spliced immediately before each host-emitted `;TYPE:` marker — by extending packet 187's `InjectionSite` enum and `INJECTION_POINTS` table rather than forking them; and record the five points PnP cannot reach today as residual deviations with the measured evidence for each, rather than faking them.

## Problem Statement

Packets 186 and 187 close the placeholder-engine half of `custom-G-code injection deviation` and the layer-scoped injection points. Six canonical points remain that PnP's pipeline **can** reach, and five that it **cannot**. This packet lands the six and files the five as residuals with measured evidence, so `custom-G-code injection deviation` closes honestly rather than by declaring a feature complete that is not.

**Reachability was determined against this tree, point by point, before this packet was written.** The evidence is recorded here so a reviewer does not have to re-derive it:

| Canonical point | PnP site | Reachable? | Evidence |
| --- | --- | --- | --- |
| `filament_end_gcode` | before `GCodeCommand::ToolChange` | **yes** | `DefaultGCodeEmitter` pushes `GCodeCommand::ToolChange { after_entity_index, from, to }` at both intra-layer and layer-boundary tool transitions (`crates/slicer-gcode/src/emit.rs`), and `crates/slicer-runtime/tests/executor/cube_4color_gcode_output_tdd.rs::cube_4color_gcode_emits_all_four_tool_indices` proves all four tool indices reach the G-code. |
| `change_filament_gcode` | before `ToolChange`, after `filament_end_gcode` | **yes** | same |
| `filament_start_gcode` | after `ToolChange` | **yes** | same |
| `change_extrusion_role_gcode` | before each `Raw(";TYPE:<label>")` | **yes** | `DefaultGCodeEmitter` pushes a `Raw` carrying `orca_type_label(role)` whenever `role_equals(prev, role)` is false (`crates/slicer-gcode/src/emit.rs`). **This matches canonical's `;TYPE:`-tag gate, not canonical's role-template gate — the two are separate members.** Measured in `GCode::_extrude`: the templates run under `path.role() != m_last_extrusion_role`; the tag (`GCodeProcessor::ETags::Role`, reserved tag string `"TYPE:"`) runs under `path.role() != m_last_processor_extrusion_role`. They are assigned independently — the wipe-tower path sets `m_last_processor_extrusion_role = erWipeTower` without touching `m_last_extrusion_role` — so canonical can fire one without the other. PnP has no wipe-tower path, so they coincide here; the site is reachable, and the two-gate difference is recorded rather than modelled. |
| `filament_change_extrusion_role_gcode` | same site, second | **yes** | same |
| `process_change_extrusion_role_gcode` | same site, third | **yes** | same |
| `file_start_gcode` | above `; HEADER_BLOCK_START` | **no** | `DefaultGCodeSerializer::serialize_gcode` (`crates/slicer-gcode/src/serialize.rs`) writes `serialize_header_block` **before** it iterates `gcode_ir.commands`, so nothing a `PostPass::GCodePostProcess` module emits can precede the header. `ThumbnailAwareSerializer::serialize_gcode` is the nearest candidate for a future hook, but it is **not** a pre-header inserter today: it calls `self.inner.serialize_gcode(gcode_ir)?` and splices its block **after** the `"; HEADER_BLOCK_END
"` sentinel. It holds `raw_config` but no substitution engine, so it would have to be both extended to prepend and given an engine. |
| `wrapping_detection_gcode` | per-extruder loop in `GCode::process_layer` | **no** | gated on canonical `enable_wrapping_detection`, which has **zero** occurrences under `crates/` or `modules/`; the per-extruder emission loop has no PnP analogue. It is also absent from canonical's own `s_CustomGcodeSpecificPlaceholders`. |
| `machine_pause_gcode` | `ProcessLayer::emit_custom_gcode_per_print_z` | **no** | gated on `custom_gcode->type == CustomGCode::PausePrint`. `CustomGCode`, `PausePrint` and `custom_gcode_per_print_z` together have **zero** occurrences under `crates/` or `modules/` — PnP has no per-print-Z custom-G-code item model at all. |
| `template_custom_gcode` | same function | **no** | same mechanism, `CustomGCode::Template`; same zero-occurrence evidence. |
| `printing_by_object_gcode` | by-object loop in `GCode::_do_export` | **no** | requires by-object print sequence; no by-object path exists in the pipeline. **Do not describe the CONFIG_BLOCK value as hard-coded** — measured, `("print_sequence", "by layer")` is one entry in `const ORCA_CONFIG_PADDING` (`crates/slicer-gcode/src/serialize.rs`), emitted by a capped fallback loop after the `raw_config` passthrough, and `emit_config_kv` skips already-emitted keys, so a `raw_config`-supplied value would win. It is a default. The unreachability rests on the missing pipeline path. |

**Restructure note, stated plainly.** The plan's decomposition put `file_start_gcode` in packet 187 and `time_lapse_gcode` in this packet. Both moved: `time_lapse_gcode` is layer-scoped and shares 187's marker walk and `{layer_num, layer_z, max_layer_z}` variable set exactly, so it belongs there; `file_start_gcode` is not reachable from this stage at all, so it is a residual here rather than an implementation anywhere in the trilogy. The resulting split is layer-scoped (187) versus toolchange- and role-scoped (188), which is the seam the code actually has.

## Architecture Constraints

<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and, if `STALE:` is reported, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.

- Config key strings must be snake_case in Rust and in the manifest. All six new keys already are.
- `ConfigView::keys` (`crates/slicer-ir/src/slice_ir.rs`) returns its own `fields` map sorted, with manifest scoping applied by the host at construction rather than by `keys()` itself. This is why AC-N3 works: an undeclared `file_start_gcode` is simply invisible to the module, so leaving it out of the manifest is a sufficient and observable way to "not implement" it.
- A guest `ModuleError` is mapped by `crates/slicer-wasm-host/src/dispatch.rs` to `slicer_ir::PostpassError::FatalModule` with the message `"module error (code={}, fatal={}): {}"`, returned unchanged by `crates/slicer-runtime/src/postpass.rs` and rendered as `"postpass failed: {e}"` by `crates/slicer-runtime/src/pipeline.rs`.
- No geometry and no millimetre/internal-unit conversion occurs here. `layer_z` / `max_layer_z` remain packet 187's carried **text**, and the new variables are integers or role labels, so the `coord-system` constraint does not apply and `mm_to_units` must not appear in this change surface.
- Determinism: the walk stays a single forward pass; per-site state is `(toolchange_count, current_tool, last_role_label)` plus 187's layer state. No map iteration may enter the emitted text.

## Data and Contract Notes

- **IR/manifest contracts.** Six new scalar string keys on one module's `[config.schema]`, taking it from eight to fourteen. No IR schema version change, no new struct field on a shared type, no public version constant bump — the struct-literal blast-radius discipline does not apply. `crates/slicer-runtime/tests/integration/manifest_default_reconcile_tdd.rs` enrols `classic-perimeters` and `arachne-perimeters` only. `gcode_header_thumbnail_config_blocks_tdd`'s CONFIG_BLOCK assertion is a lower bound ("at least 80 key-value lines"). The `integration` harness `slice_with_raw` seeds manifest defaults generically from `module.config_schema().entries`, routing string defaults to `binding_source` as the real value and to `pipeline_source` as an empty sentinel, so six `default = ""` keys need no harness edit. The `executor` harness goes through `run_slice` / `SliceRunOptions` and picks the schema up the same way.
- **The host↔module marker contracts.** `GCodeCommand::ToolChange` is a typed IR variant, so the toolchange site is structurally safe. The `;TYPE:` role marker is **text**, like packet 187's layer markers, and carries the same fragility: a future emitter change to `orca_type_label`'s prefix would silently stop the role sites from firing. Unlike the layer markers there is no malformed-state to detect — an absent `;TYPE:` marker is indistinguishable from a print with one role — so no `ERR_*` guard is possible here. State that limitation in the module's doc comment and rely on AC-7's end-to-end count equality (one `; PNP_ROLE` per `;TYPE:`) as the regression detector.
- **WIT boundary.** Unchanged. No `crates/slicer-schema/wit/**` edit and therefore no bindgen invalidation — but `modules/core-modules/machine-gcode-emit/src/**` and its `.toml` **are** guest-WASM inputs, so the freshness gate still applies.
- **Determinism/scheduler constraints.** `machine-gcode-emit` is the sole module registered at `PostPass::GCodePostProcess`, so there is no sibling-ordering interaction. All new state is scalar and advances monotonically through the single forward pass.

## Risks and Tradeoffs

- **The role site depends on host-emitted comment text with no runtime tripwire.** Weaker than packet 187's layer sites, which can detect a malformed triple. Mitigated by AC-7's end-to-end count equality and by AC-12's guard that the emitter is untouched.
- **`filament_start_gcode` / `filament_end_gcode` / `filament_change_extrusion_role_gcode` are scalar where canonical is per-filament.** A user with different purge routines per material cannot express them. Recorded as a residual rather than half-modelled.
- **`change_filament_gcode`'s flush/travel variable group is absent**, so an imported OrcaSlicer toolchange template will pass through the slice with those placeholders verbatim and a warning under packet 186's rule. That is the intended visible outcome, and it is exactly the asymmetry packet 186's own residual row already describes; this packet's row makes the toolchange instance of it concrete.
- **AC-10 is slow.** The four-colour bucket runs a full 3MF slice per test. Budget for it and do not re-run it to "see more output" — read the capture file named in AC-10's own command (`target/log-188-c4c.txt`). Every `cargo test` command in this packet writes a **distinct** capture path for exactly this reason: a shared `target/test-output.log` is clobbered within seconds by any concurrent run, which silently turns "read the log" into reading someone else's evidence.
- **Guest staleness.** Every `--test integration` and `--test executor` result is meaningless until `cargo xtask build-guests --check` is clean.
