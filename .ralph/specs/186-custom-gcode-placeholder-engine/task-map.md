# Task Map: 186-custom-gcode-placeholder-engine

This packet groups a single task ID (`TASK-305`). The crosswalk is carried anyway because `TASK-305` is **newly allocated** and has zero hits in `docs/07_implementation_status.md` at authoring time — registering it there is packet work (AC-14, Step 5), and this table is the mapping that registration must reproduce. Its backlog authority is not `docs/07` but `docs/specs/deviation-backlog-remediation-plan.md` §Packet Queue **row 8a** (`.ralph/specs/186-custom-gcode-placeholder-engine` · `DEV-085` (engine half) · tranche `T3` · no dependency), added by that plan's "Queue amendment (2026-07-25b)" note, which decomposed row 8 into 8a/8b/8c because the plan rated it aggregate `L` and the Batch Protocol forbids shipping at `L`.

**Do not quote row 8's headline counts.** The amendment records them as measured-wrong: canonical `PrintConfigDef::init_fff_params` registers **16** custom-G-code injection points (13 `coString` + 3 `coStrings`), not 15, and the row's claim that the extrusion-role family appears in `docs/ORCA_CONFIG_REFERENCE.md` is false. Correcting the `DEV-085` row is this packet's Step 5 work, not a fact to inherit.

| docs/07 task ID | Packet step | Primary docs | Expected code surface | OrcaSlicer refs | Context cost | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| `TASK-305` | `Step 0` | none required | none (writes `target/pkt-186-baseline-ref.txt` only) | none | `S` | Records the packet's own baseline SHA. Every no-touch guard diffs against it; `HEAD` and `git merge-base HEAD master` were both measured and rejected. |
| `TASK-305` | `Step 1` | `docs/15_config_keys_reference.md` (ranged) | `modules/core-modules/machine-gcode-emit/tests/machine_gcode_emit_tdd.rs` | `PlaceholderParser.cpp` — `MyContext::legacy_variable_expansion` / `MyContext::throw_exception` | `S` | RED step. **A compile failure is a valid RED here**: the new tests name `ERR_UNRESOLVED_PLACEHOLDER`, which Step 2 introduces, so the binary need not link. The gate branches on whether a `^test result:` line exists at all. |
| `TASK-305` | `Step 2` | `docs/02_ir_schemas.md` (delegated SUMMARY) | `modules/core-modules/machine-gcode-emit/src/lib.rs` | `GCode.cpp` — `GCode::placeholder_parser_process` / `GCode::check_placeholder_parser_failed` | `M` | The engine: slice-based literal runs (kills the `bytes[i] as char` mojibake), `(String, Vec<String>)` return, `PLACEHOLDER_ALIASES`, and **`pub const ERR_UNRESOLVED_PLACEHOLDER: u32 = 20;`**. The `pub` is load-bearing — the module's `tests/` directory is a separate crate and AC-N1 names the constant symbolically; the crate exports only `pub struct MachineGcodeEmit` today. |
| `TASK-305` | `Step 3` | `docs/15_config_keys_reference.md` (marker boundaries only) | `modules/core-modules/machine-gcode-emit/machine-gcode-emit.toml`, `crates/slicer-runtime/tests/integration/machine_start_end_gcode_emission_tdd.rs` | `PrintConfig.cpp` — `PrintConfigDef::init_fff_params` (confirm `nozzle_diameter` is a real option) | `M` | Declares `nozzle_diameter` (no code change needed — the existing `config.keys()` sweep in `run_gcode_postprocess` does the work once the key is declared) and **creates `try_slice_with_raw`**, the fallible sibling of `slice_with_raw`. Packets 187 and 188 cite `try_slice_with_raw` as a forward dependency on this step. |
| `TASK-305` | `Step 4` | `docs/15_config_keys_reference.md` | none (doc only) | `PrintConfig.cpp` — `PrintStatisticsConfigDef`, `OtherSlicingStatesConfigDef`, `DimensionsConfigDef`, `custom_gcode_specific_placeholders` | `S` | Rewrites the macro contract. **Write no numeral.** The domain rule (manifest keys plus the alias table) is the durable claim; a count is falsified by the module's own template keys resolving through the `config.keys()` sweep, and again by packet 187 adding three more keys to the same manifest. |
| `TASK-305` | `Step 5` | `docs/DEVIATION_LOG.md`, `docs/07_implementation_status.md` | none (doc only) | `PrintConfig.cpp` — `PrintConfigDef::init_fff_params`; `GCode.cpp` — `GCode::_do_export` | `S` | Files the residual row, corrects two measured errors on `DEV-085` (leaving it `Open`), and registers `TASK-305`. `cargo xtask check-deviations` chains into `gen_config_docs::run`, so it may touch `docs/15_config_keys_reference.md` — that write is generator-owned, confined to the generated marker spans, and a no-op after Step 4. |
| `TASK-305` | `Step 6` | none additional | none (measurement only) | none | `S` | Closure gates: `cargo check`/`clippy --workspace --all-targets`, `cargo xtask build-guests --check`, and every pipe-suffixed AC re-dispatched. |

Costs are copied from `implementation-plan.md` §Per-Step Budget Roll-Up. Aggregate `M`; no step is `L`, so no pre-activation split is required.

## Deviation crosswalk

| Deviation | Relationship | Where discharged |
| --- | --- | --- |
| `DEV-085` | **Stays `Open`.** This packet owns only the engine half; packets 187 and 188 carry the injection-point half, and 188 closes the row. Two measured errors in the row's text are corrected here (13 `coString` + 3 `coStrings`; the `filament_change_extrusion_role_gcode` claim). | Step 5 — the `DEV-085` correction probe |
| new `DEV-###` (re-derive) | Residual: the placeholder-domain asymmetry. PnP's domain is one module's manifest plus the alias table; canonical layers a local `DynamicConfig` override over a persistent parser carrying the full print config and 119 explicit `placeholder_parser().set(...)` globals. A template canonical accepts can now fail a PnP slice — visibly, by design. Also records the deliberate ordering departure (canonical emits a `!!!!! Failed to process…` marker and continues; PnP fails before emitting anything). | Step 5 — AC-13 |

Every `DEV-###` above must have its number **re-derived at the moment the row is written** (`rg -o '^\| DEV-[0-9]{3}' docs/DEVIATION_LOG.md | sort -u | tail -1`, take the next). Sibling packets in this batch file rows concurrently; a number captured earlier in the session will collide.

## Task-ID allocation

`TASK-305` is allocated to this packet by `docs/specs/deviation-backlog-remediation-plan.md` (186→`TASK-305`, 187→`TASK-306`, 188→`TASK-307`, … 192→`TASK-311`), which allocates the block centrally precisely because per-author re-derivation is what made packet 181's first allocation clash with 178's `TASK-294`.

**Do not trust any "highest TASK id" figure written here or anywhere else — re-derive it.** It is a mutable ledger fact that changes while you work:

```bash
rg -o --no-filename 'TASK-[0-9]{3}' docs/07_implementation_status.md docs/specs/*.md .ralph/specs/*/*.md | sort -u | tail -1
```

Check **both** `docs/07_implementation_status.md` and `.ralph/specs/**` — a `docs/07`-only grep is exactly the search that missed the 178/181 collision.

## ADR relationship

This packet **implements** decisions recorded in `docs/adr/0050-custom-gcode-architecture.md` — the unknown-key fail-the-slice policy with no escape syntax, the placeholder domain as one module's manifest keys plus the alias table, and engine ownership staying private to `machine-gcode-emit`. It does **not** author or amend that ADR (a separate workstream owns it), and `design.md` no longer carries those decisions as packet-local "Locked Assumptions". Reversing any of them requires amending ADR-0050, not editing this packet.

No `docs/15_config_keys_reference.md` heading anchor is introduced here — packet 187 adds `<!-- anchor: custom-gcode-injection-points -->` when it retitles the section. This packet's AC-11 still locates the section by its current heading prefix, which is correct because 186 runs first and does not retitle.
