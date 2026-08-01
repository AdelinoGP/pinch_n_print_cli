# Task Map: 187-custom-gcode-injection-registry

This packet groups a single task ID (`TASK-306`). The crosswalk is carried anyway because `TASK-306` is **newly allocated** and has zero hits in `docs/07_implementation_status.md` at authoring time — registering it there is packet work (AC-14, Step 5), and this table is the mapping that registration must reproduce. Its backlog authority is not `docs/07` but `docs/specs/deviation-backlog-remediation-plan.md` §Packet Queue **row 8b** (`.ralph/specs/187-custom-gcode-injection-registry` · `DEV-085` (layer-scoped points) · tranche `T3` · **depends on #8a**), added by that plan's "Queue amendment (2026-07-25b)" note, which decomposed row 8 into 8a/8b/8c because the plan rated it aggregate `L`.

**Dependency, not merely ordering.** Row 8b's `#8a` dependency is real code: this packet rewrites `run_gcode_postprocess` around the engine packet 186 ships, and `try_slice_with_raw` — cited in Step 3's read list — is **created by 186**, not present on today's tree. If it is absent, 186 has not landed and the packet stops. `time_lapse_gcode` was moved 8c→8b by the same amendment because it is layer-scoped and shares this packet's `;LAYER_CHANGE` walk.

| docs/07 task ID | Packet step | Primary docs | Expected code surface | OrcaSlicer refs | Context cost | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| `TASK-306` | `Step 0` | none required | `.ralph/specs/187-custom-gcode-injection-registry/baseline-ref.txt` (durable) + `target/pkt-187-baseline-ref.txt` (cache) | none | `S` | The SHA is recorded **twice**: `target/` is gitignored scratch destroyed by `cargo clean`, and AC-10's no-touch guard reads it. Recovery is a `cp` from the durable copy — **never** a re-run of this step, which would record a HEAD already containing the packet's edits and make the guard pass vacuously. |
| `TASK-306` | `Step 0a` | `docs/15_config_keys_reference.md` (ranged) | none (measurement only) | none | `S` | **Re-measures every AC baseline against the post-186 tree.** AC-11's `stale` clause targets the same `docs/15` blockquote packet 186 deletes, so it may already be green when this packet starts. Name any pre-satisfied clause rather than treating its eventual PASS as evidence. |
| `TASK-306` | `Step 1` | `docs/02_ir_schemas.md` (delegated SUMMARY) | `modules/core-modules/machine-gcode-emit/tests/machine_gcode_emit_tdd.rs` | `GCode.cpp` — `GCode::process_layer`; `PrintConfig.cpp` — `s_CustomGcodeSpecificPlaceholders` | `S` | RED: seven tests over a synthetic stream carrying the host's `;LAYER_CHANGE` / `;Z:` / `;HEIGHT:` triple. Copy the marker text from `crates/slicer-gcode/src/emit.rs` rather than retyping it. |
| `TASK-306` | `Step 2` | `docs/02_ir_schemas.md` | `modules/core-modules/machine-gcode-emit/src/lib.rs` | `GCode.cpp` — `GCode::process_layer`, `GCode::change_layer` | `M` | `InjectionSite` / `InjectionPoint` / `INJECTION_POINTS` (private, closed, declaration-ordered — ADR-0050), `LayerContext`, `ERR_MALFORMED_LAYER_MARKER`, and the single forward site walk. Start/end migrate onto the registry in the same atomic change. |
| `TASK-306` | `Step 3` | `docs/15_config_keys_reference.md` (marker boundaries only) | `modules/core-modules/machine-gcode-emit/machine-gcode-emit.toml`, `crates/slicer-runtime/tests/integration/machine_start_end_gcode_emission_tdd.rs` | `PrintConfig.cpp` — `PrintConfigDef::init_fff_params` | `M` | Three new `""`-default string keys plus the e2e layer-count pin. `crates/slicer-gcode/**` is read-only for the whole packet; AC-10 diffs it against Step 0's ref. |
| `TASK-306` | `Step 4` | `docs/15_config_keys_reference.md` | none (doc only) | `PrintConfig.cpp` — `custom_gcode_specific_placeholders` | `S` | Retitles the section to `## Custom G-code injection points` **and writes the anchor `<!-- anchor: custom-gcode-injection-points -->` below it** — without the anchor the retitle blanks AC-11's section slice. Restate the resolvable-placeholder set as a **rule**, never a count. |
| `TASK-306` | `Step 5` | `docs/DEVIATION_LOG.md`, `docs/07_implementation_status.md` | none (doc only) | `GCode.cpp` — `GCode::process_layer`, `GCode::generate_timelapse_gcode` | `S` | One residual row carrying all four accepted parity residuals; `DEV-085` cites `TASK-306` and stays `Open`; `TASK-306` registered outside the generated block. |
| `TASK-306` | `Step 6` | none additional | none (measurement only) | none | `S` | Closure gates. Step 0's ref must still exist; if `cargo clean` removed it, restore from the durable copy. |

Costs are copied from `implementation-plan.md` §Per-Step Budget Roll-Up. Aggregate `M`; no step is `L`, so no pre-activation split is required.

## Deviation crosswalk

| Deviation | Relationship | Where discharged |
| --- | --- | --- |
| `DEV-085` | **Stays `Open`.** This packet lands the layer-scoped half and cites `TASK-306` on the row; packet 188 closes it. | Step 5 — the `DEV-085` row probe |
| new `DEV-###` (re-derive) | Residual, four parts in one row: (a) canonical's `layer_change_gcode` resolves **no** `max_layer_z` at all — the `set_key_value` sits after the parse and writes into a local — where PnP supplies one; (a2) the *no*-divergence finding at the other two sites; (b) the unported BBL `generate_timelapse_gcode` path; (c) six unmodelled `layer_change_gcode` variables; (d) canonical's `GCode::change_layer` may interleave `update_progress` / a retract / `add_object_change_labels` between the templates, where PnP interleaves nothing. | Step 5 — AC-13 |

Every `DEV-###` above must have its number **re-derived at the moment the row is written** (`rg -o '^\| DEV-[0-9]{3}' docs/DEVIATION_LOG.md | sort -u | tail -1`, take the next). Sibling packets file rows concurrently; a number captured earlier in the session will collide.

## Task-ID allocation

`TASK-306` is allocated to this packet by `docs/specs/deviation-backlog-remediation-plan.md` (186→`TASK-305`, 187→`TASK-306`, 188→`TASK-307`, … 192→`TASK-311`), allocated centrally because per-author re-derivation is what made packet 181's first allocation clash with 178's `TASK-294`.

**Do not trust any "highest TASK id" figure written here or anywhere else — re-derive it.** It is a mutable ledger fact that changes while you work:

```bash
rg -o --no-filename 'TASK-[0-9]{3}' docs/07_implementation_status.md docs/specs/*.md .ralph/specs/*/*.md | sort -u | tail -1
```

Check **both** `docs/07_implementation_status.md` and `.ralph/specs/**`; a `docs/07`-only grep is the search that missed the 178/181 collision.

## ADR relationship

This packet **implements** the registry and ownership decisions recorded in `docs/adr/0050-custom-gcode-architecture.md` (placeholder domain = one module's manifest keys plus the alias table; engine ownership private to `machine-gcode-emit`; `INJECTION_POINTS` as a **private closed `const` with declaration-order precedence**) and in `docs/adr/0051-gcode-marker-contract-ownership.md` (the `;LAYER_CHANGE` / `;Z:` / `;HEIGHT:` marker contract — who owns the strings, who may consume them, and what a consumer owes when they change). Its unknown-key contract is warn-and-pass: unavailable per-site variables remain verbatim, the run returns `Ok`, and one warning names the config key and site. Both ADRs are authored by a separate workstream: **do not author or edit them from this packet.** `design.md` no longer carries these as packet-local "Locked Assumptions"; the `;Z:`-only lookahead and its BBL consequence are likewise recorded as a decision pointing at ADR-0051, not as an answered Open Question.

`ERR_MALFORMED_LAYER_MARKER` is this packet's warning diagnostic for ADR-0051: a marker-contract consumer carries it while reusing prior Z instead of aborting or mis-splicing silently. The second consumer, `crates/pnp-cli/src/visual_debug_gcode.rs`, is named in `design.md` §Risks.

## Activation blockers

`140_lightning-module-rewrite` is the only `status: active` packet — **verified with an anchored match**, `rg -l '^status: active' .ralph/specs/*/packet.spec.md`. (An unanchored search for the word "active" matches prose in several draft packets and produces false positives; packet 190's frontmatter reads `status: draft`.) This packet stays `draft` until 140 clears **and** until 186 is `implemented`.
