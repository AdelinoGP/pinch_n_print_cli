# Task Map: 188-custom-gcode-conditional-points

This packet groups a single task ID (`TASK-307`). The crosswalk is carried anyway because `TASK-307` is **newly allocated** and has zero hits in `docs/07_implementation_status.md` at authoring time — registering it there is packet work (AC-18, Step 7), and this table is the mapping that registration must reproduce. Its backlog authority is not `docs/07` but `docs/specs/deviation-backlog-remediation-plan.md` §Packet Queue **row 8c** (`.ralph/specs/188-custom-gcode-conditional-points` · `DEV-085` (tool/role-scoped points + residuals) · tranche `T3` · **depends on #8b**), added by that plan's "Queue amendment (2026-07-25b)" note, which decomposed row 8 into 8a/8b/8c because the plan rated it aggregate `L`.

**This is the row that closes `DEV-085`.** The amendment also moved `file_start_gcode` 8b→8c **as a recorded residual, not an implementation**: `DefaultGCodeSerializer::serialize_gcode` (`crates/slicer-gcode/src/serialize.rs`) writes `serialize_header_block` before it iterates `gcode_ir.commands`, so nothing a `PostPass::GCodePostProcess` module emits can precede the header. `wrapping_detection_gcode`, `machine_pause_gcode`, `template_custom_gcode` and `printing_by_object_gcode` are recorded the same way.

**Dependency, not merely ordering.** Row 8c inherits 8b's dependency on 8a. This packet extends `INJECTION_POINTS` from five entries to eleven — the table packet 187 creates — and its Step 4 read list cites `try_slice_with_raw`, which packet **186** creates. Either absence breaks the 186 → 187 → 188 chain and the packet stops rather than reimplementing the missing half.

| docs/07 task ID | Packet step | Primary docs | Expected code surface | OrcaSlicer refs | Context cost | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| `TASK-307` | `Step 0` | none required | none (writes `target/pkt-188-baseline-ref.txt` only) | none | `S` | Baseline SHA for AC-12's no-touch guard over `crates/slicer-gcode/src/emit.rs`, `src/serialize.rs` and `tests/golden_emit_tdd.rs`. |
| `TASK-307` | `Step 1` | `docs/02_ir_schemas.md` (delegated SUMMARY) | `modules/core-modules/machine-gcode-emit/tests/machine_gcode_emit_tdd.rs` | `GCode.cpp` — `GCode::set_extruder`, `GCode::_extrude`; `PrintConfig.cpp` — `s_CustomGcodeSpecificPlaceholders` | `S` | RED: eight tests (AC-3, AC-4, AC-5, AC-6, AC-19, AC-20, AC-N1, AC-N2). AC-20 is the positive half of the placeholder-table drift — the table lists only `{filament_extruder_id}` for `filament_start_gcode`, but the runtime `DynamicConfig` also sets `layer_num` / `layer_z` / `max_layer_z`, so a literal transcription would fatally reject a template canonical resolves. |
| `TASK-307` | `Step 2` | `docs/02_ir_schemas.md` | `modules/core-modules/machine-gcode-emit/src/lib.rs` | `GCode.cpp` — `GCode::set_extruder`, `WipeTowerIntegration::append_tcr` | `M` | Toolchange sites: registry entries, the `GCodeCommand::ToolChange` walk, and the per-site variable sets. `ToolChange` is always re-emitted — bracketed, never dropped or reordered. |
| `TASK-307` | `Step 3` | `docs/02_ir_schemas.md` | `modules/core-modules/machine-gcode-emit/src/lib.rs` | `GCode.cpp` — `GCode::_extrude` | `M` | Role sites: three templates sharing one `;TYPE:` site, emitted in canonical order via `INJECTION_POINTS` **declaration-order precedence** (ADR-0050). PnP splices at canonical's `m_last_processor_extrusion_role` **tag** gate, not its `m_last_extrusion_role` **template** gate — a divergence that is now an AC-16 predicate token, not prose alone. |
| `TASK-307` | `Step 4` | `docs/03_wit_and_manifest.md` | `modules/core-modules/machine-gcode-emit/machine-gcode-emit.toml`, `crates/slicer-runtime/tests/integration/machine_start_end_gcode_emission_tdd.rs` | `PrintConfig.cpp` — `PrintConfigDef::init_fff_params` | `M` | Six new `[config.schema.*]` string blocks (fourteen keys total), the manifest `[module] description` and crate doc comment, and the single-material role e2e pin. |
| `TASK-307` | `Step 5` | none required | `crates/slicer-runtime/tests/executor/cube_4color_gcode_output_tdd.rs` | `GCode.cpp` — `GCode::set_extruder` | `M` | The four-tool end-to-end toolchange pin. **Slow** — a full 3MF slice per test; read `target/log-188-c4c.txt`, never re-run to "see more output". |
| `TASK-307` | `Step 6` | `docs/15_config_keys_reference.md` | none (doc only) | `PrintConfig.cpp` — `custom_gcode_specific_placeholders` | `S` | Extends the anchored injection-point section to eleven points. **Extend `<!-- anchor: custom-gcode-injection-points -->`; do not create a second section, remove the anchor, or duplicate it** — packet 187's AC-11 slices on it and must keep passing at this packet's closure. AC-13's own probe is whole-file scoped. |
| `TASK-307` | `Step 7` | `docs/DEVIATION_LOG.md`, `docs/07_implementation_status.md` | none (doc only) | `PrintConfig.cpp`, `GCode.cpp` — delegate; supply all three rows' evidence | `S` | **Three** residual rows, then `DEV-085` → `Closed` citing `TASK-305` / `TASK-306` / `TASK-307`, then `TASK-307` registered outside the generated block. `DEV-085` may only be closed after all three rows exist. |
| `TASK-307` | `Step 8` | none additional | none (measurement only) | none | `S` | Closure gates: `cargo check`/`clippy --workspace --all-targets`, `cargo xtask build-guests --check`, and all twenty-four numbered AC commands (AC-1..AC-21, AC-N1..AC-N3). |

Costs are copied from `implementation-plan.md` §Per-Step Budget Roll-Up. Aggregate `M` — **at the top of M**: four of nine steps are `M` and the packet spans three test binaries. If any step's read set grows beyond the ranges in `implementation-plan.md`, split Step 2 (toolchange) from Step 3 (role) into separate packets rather than escalating the band.

## Deviation crosswalk

| Deviation | Relationship | Where discharged |
| --- | --- | --- |
| `DEV-085` | **Closed by this packet**, citing all three task IDs. Closure is gated on all three residual rows existing first, so the log never carries a closed row whose remainder is untracked. | Step 7 — AC-17 |
| new `DEV-###` (re-derive) | Residual #1: the five canonical injection points PnP cannot reach — `file_start_gcode` (blocked by `serialize_header_block` ordering), `wrapping_detection_gcode`, `machine_pause_gcode`, `template_custom_gcode`, `printing_by_object_gcode` — each with its measured reason. | Step 7 — AC-15 |
| new `DEV-###` (re-derive) | Residual #2, variable-level: `coStrings` per-filament vectors declared as scalars; the unmodelled `change_filament_gcode` flush/travel group (a deliberately **scoped subset**, not canonical's whole set); `filament_start_gcode`'s four incompatible canonical scopes; the `erNone` → empty-string substitution; the `manual_filament_change` first-toolchange suppression; and the **tag-gate vs template-gate** divergence (predicate token `m_last_processor_extrusion_role`). | Step 7 — AC-16 |
| new `DEV-###` (re-derive) | Residual #3, **its own row so it can be closed piecemeal**: the `filament_extruder_id` two-id-space divergence. Canonical binds an **extruder** id at `filament_end_gcode` (`get_extruder_id(old_filament_id)`) and a raw **filament** id at `filament_start_gcode` (`new_filament_id`); PnP's `GCodeCommand::ToolChange { from, to }` carries tool indices only and cannot represent the split. Direction is ported faithfully, id space is not. Cites `docs/adr/0050-custom-gcode-architecture.md`. | Step 7 — AC-21 |

Every `DEV-###` above must have its number **re-derived at the moment the row is written** (`rg -o '^\| DEV-[0-9]{3}' docs/DEVIATION_LOG.md | sort -u | tail -1`, take the next). This packet files **three** rows, so re-derive before **each** one — filing the first changes the answer for the second — and sibling packets file rows concurrently.

## Task-ID allocation

`TASK-307` is allocated to this packet by `docs/specs/deviation-backlog-remediation-plan.md` (186→`TASK-305`, 187→`TASK-306`, 188→`TASK-307`, … 192→`TASK-311`), allocated centrally because per-author re-derivation is what made packet 181's first allocation clash with 178's `TASK-294`.

**Do not trust any "highest TASK id" figure written here or anywhere else — re-derive it.** It is a mutable ledger fact that changes while you work:

```bash
rg -o --no-filename 'TASK-[0-9]{3}' docs/07_implementation_status.md docs/specs/*.md .ralph/specs/*/*.md | sort -u | tail -1
```

Check **both** `docs/07_implementation_status.md` and `.ralph/specs/**`; a `docs/07`-only grep is the search that missed the 178/181 collision.

## ADR relationship

This packet **implements** decisions recorded in `docs/adr/0050-custom-gcode-architecture.md` (unknown-key fail-the-slice policy with no escape syntax; placeholder domain = one module's manifest keys plus the alias table; engine ownership private to `machine-gcode-emit`; `INJECTION_POINTS` as a private closed `const` with **declaration-order precedence**, which is what lets three role templates share one `;TYPE:` site in canonical order; and the `filament_extruder_id` id-space constraint as an **IR-level constraint binding future MMU / multi-extruder work**) and in `docs/adr/0051-gcode-marker-contract-ownership.md` (the host-published marker contract, of which the `;TYPE:` role marker is the same unversioned class as `;LAYER_CHANGE` / `;Z:` / `;HEIGHT:`). Both ADRs are authored by a separate workstream: **do not author or edit them from this packet.**

Two residuals deliberately get **no** ADR: the `erNone` → empty-string substitution and the `manual_filament_change` suppression are documentation residuals, fully served by their AC-16 clauses and predicate tokens.
