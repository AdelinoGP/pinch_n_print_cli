# Task Map: 186-custom-gcode-placeholder-engine

This packet groups a single task ID (`TASK-305`). The crosswalk is carried anyway because `TASK-305` had zero hits in `docs/07_implementation_status.md` at original authoring time — registering it there is packet work (AC-14, Step 6), and this table is the mapping that registration must reproduce. Its backlog authority is not `docs/07` but `docs/specs/deviation-backlog-remediation-plan.md` §Packet Queue, whose row for this packet directory was created by the "Queue amendment (2026-07-25b)" note that decomposed the original single `custom-G-code injection deviation` row into one row per packet directory because the plan rated it aggregate `L` and the Batch Protocol forbids shipping at `L`. **Reference the queue by packet-directory identity, never by row number or cell text** — the split already invalidated both once.

**Do not quote the original queue row's headline counts.** The amendment records them as measured-wrong: canonical `PrintConfigDef::init_fff_params` registers **16** custom-G-code injection points (13 `coString` + 3 `coStrings`), not 15, and the row's claim that the extrusion-role family appears in `docs/ORCA_CONFIG_REFERENCE.md` is false. Correcting the `custom-G-code injection deviation` row is this packet's Step 6 work, not a fact to inherit.

**Re-authoring note.** This packet was re-authored after its central policy — an unresolved `[key]` as a fatal slice error — was rejected by the repo owner and reverted in code. Every step row below describes the **delivered** warn-and-pass behaviour. The step count grew from seven to eight: a new **Step 4** carries the list-valued config fix (review finding F2) and the fixture-level e2e gate that the original packet lacked.

| docs/07 task ID | Packet step | Primary docs | Expected code surface | OrcaSlicer refs | Context cost | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| `TASK-305` | `Step 0` | none required | none (reconstructs `target/pkt-186-baseline-ref.txt` at coordinator closure) | none | `S` | Reconstructs the current `HEAD` baseline from the closure manifest; it does not assume a clean pre-edit worktree or trust a stale scratch ref. Every no-touch guard uses the coordinator-approved closure baseline. |
| `TASK-305` | `Step 1` | `docs/15_config_keys_reference.md` (ranged) | `modules/core-modules/machine-gcode-emit/tests/machine_gcode_emit_tdd.rs` | `GCode.cpp` — `GCode::update_placeholder_parser_with_variant_params` (**sibling checkout only**) | `S` | RED step, assertions only. Adds `try_run`, the non-ASCII test, the both-templates passthrough test, and the alias test; **keeps `unknown_placeholder_passes_through_verbatim` asserting passthrough.** Unlike the superseded plan, this step names no new module symbol, so the binary **compiles** — a compile error here is a defect, not a valid RED. |
| `TASK-305` | `Step 2` | `docs/02_ir_schemas.md` (delegated SUMMARY) | `modules/core-modules/machine-gcode-emit/src/lib.rs` | `GCode.cpp` — `GCode::placeholder_parser_process` / `GCode::check_placeholder_parser_failed` (**sibling checkout only**) | `M` | The engine: slice-based literal runs (kills the `bytes[i] as char` mojibake), `(String, Vec<String>)` return, `PLACEHOLDER_ALIASES`, and **one aggregated `slicer_sdk::host::log_warn`** over a `BTreeSet` union, after which emission proceeds and the call returns `Ok`. **No `ERR_UNRESOLVED_PLACEHOLDER`, no `ModuleError::fatal` on the placeholder path, no `sites_clause` helper** — all three were built by the superseded plan and deleted on reversal; AC-4 asserts their absence. Canonical is borrowed for the *aggregation*, not the throw. |
| `TASK-305` | `Step 3` | `docs/15_config_keys_reference.md` (marker boundaries only) | `modules/core-modules/machine-gcode-emit/machine-gcode-emit.toml`, `crates/slicer-runtime/tests/integration/machine_start_end_gcode_emission_tdd.rs` | `PrintConfig.cpp` — `PrintConfigDef::init_fff_params` (confirm `nozzle_diameter` is a real option) | `M` | Declares `nozzle_diameter` (no engine change needed — the existing `config.keys()` sweep does the work once the key is declared) and **creates `try_slice_with_raw`**, the fallible sibling of `slice_with_raw`. Packets 187 and 188 cite `try_slice_with_raw` as a forward dependency on this step, which is why it is retained even though no criterion here needs its fallibility. |
| `TASK-305` | `Step 4` | none additional | `modules/core-modules/machine-gcode-emit/src/lib.rs` (`format_placeholder_value`), `modules/core-modules/machine-gcode-emit/tests/machine_gcode_emit_tdd.rs`; **runs** `crates/slicer-runtime/tests/e2e/modifier_infill_tdd.rs` and `crates/slicer-runtime/tests/e2e/cube_painted_e2e_tdd.rs` unedited | `GCode.cpp` — `GCode::_do_export`'s `; first_layer_temperature = %d` preamble reads element 0 (**sibling checkout only**) | `M` | **New step.** `ConfigValue::List` resolves from its first element; an empty list yields `None` and the placeholder passes through. This is review finding **F2**: without it `[nozzle_diameter]` is inert for every real slice, because real 3MF supplies `['0.4']`, and Step 3's scalar-default AC-8 cannot see the difference. Second half is the **fixture-level e2e gate (AC-18)** — `cargo build -p pnp-cli` then both suites. The e2e suites and `resources/*.3mf` are **run, never edited**. |
| `TASK-305` | `Step 5` | `docs/15_config_keys_reference.md` | none (doc only) | `PrintConfig.cpp` — `PrintStatisticsConfigDef`, `OtherSlicingStatesConfigDef`, `DimensionsConfigDef`, `s_CustomGcodeSpecificPlaceholders` | `S` | Rewrites the macro contract to **warn-and-pass** — the literal phrase `passes through verbatim and is warned about`, and `fatal slice error` nowhere in the file (the superseded plan mandated the opposite sentence). **Write no numeral.** The domain rule (manifest keys plus the alias table) is the durable claim; a count is falsified by the module's own template keys resolving through the `config.keys()` sweep, and again by packet 187 adding three more keys. The escape-syntax paragraph inverts: none exists **and none is needed**. |
| `TASK-305` | `Step 6` | `docs/DEVIATION_LOG.md`, `docs/07_implementation_status.md` | none (doc only) | `PrintConfig.cpp` — `PrintConfigDef::init_fff_params`; `GCode.cpp` — `GCode::_do_export` (**sibling checkout only**) | `S` | Files the `DEV-100` residual row carrying all eight macros, canonical counterparts, domain asymmetry, warn-and-pass, and the record that the rejected fatal-on-unresolved policy was reverted; the deleted aggregate custom-G-code label is not recreated; and registers exactly one `TASK-305` row outside the generated block. The `slice-fatal` token is forbidden. `cargo xtask check-deviations` chains into `gen_config_docs::run`, so it may touch `docs/15_config_keys_reference.md`; that write is generator-owned, confined to the generated marker spans, and a no-op after Step 5. |
| `TASK-305` | `Step 7` | none additional | none (measurement only) | none | `S` | Closure gates: workspace check/clippy, `cargo xtask build-guests --check`, `cargo build -p pnp-cli`, and the full **21-AC matrix** (AC-1..AC-18, AC-N1..AC-N3) re-dispatched — **AC-18 last, and blocking regardless of every other result.** Status is implemented after the coordinator's closure ceremony. |

Costs are copied from `implementation-plan.md` §Per-Step Budget Roll-Up. Aggregate `M`; no step is `L`, so no pre-activation split is required.

## Deviation crosswalk

| Deviation | Relationship | Where discharged |
| --- | --- | --- |
| retired `custom-G-code injection deviation` label | **Remains absent.** This packet owns the engine half; packets 187 and 188 carry the injection-point residuals. The surviving placeholder evidence is `DEV-100`. | Step 6 — the post-purge ledger probe |
| residual `DEV-###` (**re-derive**) | Residual of `custom-G-code injection deviation`'s user-facing half: the eight advertised macros with no PnP key to bind them to, emitted as **warned verbatim text**. Records their measured canonical counterparts; the domain asymmetry (PnP resolves against one module's manifest plus the alias table, canonical against a persistent parser carrying the whole print config plus ~119 `placeholder_parser().set(...)` globals); the failure-handling divergence (canonical marks-and-continues then throws once via `check_placeholder_parser_failed`, so canonical ultimately **fails** an export PnP **completes** with literal `[key]` text in it); and the fact that the fatal policy was built and **reverted** on the repo owner's modularity ruling, with the measured 3MF evidence. **Must NOT carry a `slice-fatal` token.** | Step 6 — AC-13 |

Every `DEV-###` above must have its number **re-derived at the moment the row is written** (`rg -o '^\| DEV-[0-9]{3}' docs/DEVIATION_LOG.md | sort -u | tail -1`, take the next). Sibling packets in this batch file rows concurrently; a number captured earlier in the session will collide. The residual row already exists on disk from the first implementation pass — **verify its ID rather than allocating a new one**, or the log gains a duplicate.

## Task-ID allocation

`TASK-305` is allocated to this packet by `docs/specs/deviation-backlog-remediation-plan.md` (186→`TASK-305`, 187→`TASK-306`, 188→`TASK-307`, … 192→`TASK-311`), which allocates the block centrally precisely because per-author re-derivation is what made packet 181's first allocation clash with 178's `TASK-294`.

**Do not trust any "highest TASK id" figure written here or anywhere else — re-derive it.** It is a mutable ledger fact that changes while you work:

```bash
rg -o --no-filename 'TASK-[0-9]{3}' docs/07_implementation_status.md docs/specs/*.md .ralph/specs/*/*.md | sort -u | tail -1
```

Check **both** `docs/07_implementation_status.md` and `.ralph/specs/**` — a `docs/07`-only grep is exactly the search that missed the 178/181 collision.

## ADR relationship

`docs/adr/0050-custom-gcode-architecture.md` is aligned with this packet's
implementation. Its warn-and-pass decision, manifest-scoped domain, private
engine ownership, and closed registry are the authority consumed by the steps
below.

- This packet records and verifies the ADR rewrite in its closure evidence; the
  ADR is now aligned and accepted for the warn-and-pass contract.
- The packet must preserve the ADR's warn-and-pass contract: unresolved text is
  emitted verbatim, the run returns `Ok`, and one warning aggregates sorted,
  deduplicated keys and contributing sites.
- The two ADR-0050 decisions directly used here are the **placeholder domain**
  (one module's manifest-declared keys plus the alias table) and **engine
  ownership** (private to `machine-gcode-emit`).

`docs/15_config_keys_reference.md` §"Machine start / end G-code" and the
residual `DEV-###` row remain the packet-level evidence for those aligned
decisions. Packet 187 and packet 188 inherit the same relationship; their
migrations must retain the five-key integration contract and
`nozzle_diameter` assertion before adding their own keys.

No `docs/15_config_keys_reference.md` heading anchor is introduced here — packet 187 adds `<!-- anchor: custom-gcode-injection-points -->` when it retitles the section. This packet's AC-11 locates the section by its current heading prefix (`## Machine start / end G-code`, which on disk carries a `(packet 59)` suffix), which is correct because 186 runs first and does not retitle.
