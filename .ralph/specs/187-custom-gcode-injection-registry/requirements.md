# Requirements: 187-custom-gcode-injection-registry

## Packet Metadata

- Grouped task IDs: `TASK-306`
- Backlog source: `docs/specs/deviation-backlog-remediation-plan.md` the Packet Queue entry for `DEV-085`, tranche T3 (referenced by identity — row numbers rot), split 2 of 3; registered in `docs/07_implementation_status.md` by this packet
- Packet status: `draft`
- Aggregate context cost: `M`

## Problem Statement

`machine-gcode-emit` has no notion of an injection point. `run_gcode_postprocess` reads two config keys with two hand-written `match config.get("machine_…_gcode")` arms, substitutes them, and frames the re-emitted stream. Adding a third point by copy-paste is exactly the shape `DEV-085` warns about, because OrcaSlicer already demonstrates where it leads: canonical has no abstraction here either, and the same `if (!empty) { DynamicConfig; set_key_value; placeholder_parser_process }` block is hand-inlined 20+ times across `GCode::_do_export`, `GCode::process_layer`, `GCode::set_extruder` and `GCode::_extrude`. Its one registry, `s_CustomGcodeSpecificPlaceholders` (`PrintConfig.cpp`), is validation-only and has already drifted. (**Precision, because the existing `DEV-085` row gets this wrong and the error was inherited verbatim into an earlier draft of this packet:** the table itself compiles **unconditionally** — only its *consumers* are `#if ORCA_CHECK_GCODE_PLACEHOLDERS`-gated, and that macro is defined only under `#if !defined(NDEBUG)`. "Validation-only" is accurate; "the table is gated" is not.) The drift: it keys timelapse under the parser name `timelapse_gcode`, which does **not** match the config key `time_lapse_gcode`, and it omits `wrapping_detection_gcode` entirely even though `GCode::process_layer` emits that option with five placeholder variables.

**So a real injection-point registry is an improvement over canonical, not merely parity, and this packet must be read that way.** There is no canonical structure to mirror; what is mirrored is the *behaviour* — the emission order at a layer boundary, and the per-site variable sets that `s_CustomGcodeSpecificPlaceholders` records, drift included, corrected where measured.

The three layer-scoped points are reachable today. `DefaultGCodeEmitter` (`crates/slicer-gcode/src/emit.rs`) pushes three consecutive `GCodeCommand::Raw` markers at every emitted layer — `;LAYER_CHANGE`, `;Z:<z>`, `;HEIGHT:<h>` — before the layer's first command, and `machine-gcode-emit` already receives and re-emits `Raw` commands verbatim. It is the sole module registered at `PostPass::GCodePostProcess` (verified: no other manifest under `modules/core-modules/*/*.toml` names that stage), so nothing else can perturb the stream between the emitter and the splice.

## In Scope

- Introduce `InjectionPoint` (config key + site), `InjectionSite` (a `PrintStart` / `BeforeLayerChange` / `TimeLapse` / `LayerChange` / `PrintEnd` enum), and `const INJECTION_POINTS: &[InjectionPoint]` in `modules/core-modules/machine-gcode-emit/src/lib.rs`, with five entries.
- Re-express `machine_start_gcode` and `machine_end_gcode` through the registry so the table is load-bearing rather than decorative, **without moving either block** — the start block must still precede the M73 pair and `ExtrusionMode`, and the end block must still follow the last `G1` and precede the CONFIG_BLOCK.
- Walk `commands` once to locate every layer boundary (a `Raw` whose text is exactly `;LAYER_CHANGE`), splice the resolved `before_layer_change_gcode`, `time_lapse_gcode` and `layer_change_gcode` immediately after that boundary's `;HEIGHT:` marker, in that order.
- Supply a per-site variable set on top of the manifest config keys: `layer_num` (1-based), `layer_z` and `max_layer_z` at `BeforeLayerChange`, `TimeLapse`, `LayerChange` and `PrintEnd`; **none** at `PrintStart`.
- Substitute `layer_z` and `max_layer_z` as the **verbatim text** that followed the `;Z:` marker, never as a re-formatted parse of it.
- Add `ERR_MALFORMED_LAYER_MARKER` and fail when a `;LAYER_CHANGE` marker is not followed within two commands by a `;Z:` marker.
- Declare `before_layer_change_gcode`, `layer_change_gcode` and `time_lapse_gcode` in `modules/core-modules/machine-gcode-emit/machine-gcode-emit.toml`, each `type = "string"`, `default = ""`, `group = "Machine G-code"`.
- Add the tests named by AC-3 through AC-7 and AC-N1 through AC-N3.
- Rewrite `docs/15_config_keys_reference.md` §"Machine start / end G-code" for the registry; regenerate the `module-config-keys` block; update the `DEV-085` row; file one new residual `DEV-###` row; register `TASK-306`.

## Out of Scope

- **Any host crate change.** `crates/slicer-gcode/src/emit.rs` and `crates/slicer-gcode/tests/golden_emit_tdd.rs` must be byte-identical at closure (AC-10 guards this). The marker triple is consumed, not created.
- **Toolchange- and extrusion-role-scoped points.** `change_filament_gcode`, `filament_start_gcode`, `filament_end_gcode` and the three-way `change_extrusion_role_gcode` / `filament_change_extrusion_role_gcode` / `process_change_extrusion_role_gcode` family are packet 188 (`TASK-307`), together with the `filament_extruder_id`, `next_extruder`, `previous_extruder`, `extrusion_role` and `last_extrusion_role` variables they need.
- **`file_start_gcode`.** Canonical emits it at the very top of the file, **before** `; HEADER_BLOCK_START`. In PnP the header is written by `DefaultGCodeSerializer::serialize_gcode` (`crates/slicer-gcode/src/serialize.rs`) *before* it iterates `gcode_ir.commands`, so nothing a `PostPass::GCodePostProcess` module emits can precede it; the nearest candidate for a future hook, `ThumbnailAwareSerializer::serialize_gcode`, is **not** a pre-header inserter today — measured, it calls `self.inner.serialize_gcode(gcode_ir)?` and then splices its block **after** the `"; HEADER_BLOCK_END
"` sentinel, so it is a *post*-header inserter that would have to be both extended to prepend and given a substitution engine. `file_start_gcode` is therefore unreachable from this stage and is recorded, with that evidence, as a residual by packet 188 rather than faked into a post-header position here.
- **The BBL timelapse path.** `GCode::generate_timelapse_gcode`, its `M624`/`M625` object labels and its eight extra variables are not ported; PnP implements canonical's non-BBL inline `time_lapse_gcode` emission only. Recorded in this packet's new residual row.
- **Canonical's dead-write `max_layer_z` at `layer_change_gcode`.** Re-measured, and sharper than an earlier draft claimed: in `GCode::process_layer` each template gets its **own block-scoped** `DynamicConfig`, and the `layer_change_gcode` block never sets `max_layer_z` before its parse — the `set_key_value("max_layer_z", …)` sits after the parse and writes into a local destroyed at the closing brace. No base or global layer carries the key (`max_layer_z` is declared only in `CustomGcodeSpecificConfigDef`, and there is no `placeholder_parser().set` for it). So canonical resolves **no `max_layer_z` at all** at that site, not a one-layer-late value. PnP supplies the running maximum inclusive of the current layer. **At `before_layer_change_gcode` and `time_lapse_gcode` there is no divergence at all** — canonical sets the key before both parses, and `m_max_layer_z = std::max(m_max_layer_z, m_last_layer_z)` runs before the `before_layer_change_gcode` block, so PnP's inclusive running maximum is exact parity there. Only the `layer_change_gcode` site diverges, and it is recorded in the new residual row rather than reproduced.
- **A `{…}` expression syntax**, an escape for literal square brackets, and any relaxation of packet 186's fatal-on-unresolved-placeholder rule.
- **`docs/ORCA_CONFIG_REFERENCE.md`** — no edit (see `packet.spec.md` §Doc Impact Statement).

## Authoritative Docs

- `docs/15_config_keys_reference.md` — long; ranged reads only (§"Machine start / end G-code" and the `module-config-keys` marker boundaries).
- `docs/02_ir_schemas.md` — delegated SUMMARY only, for `GCodeIR.commands` and the `PostPass::GCodePostProcess` input surface.
- `docs/DEVIATION_LOG.md` — long; delegate. The `DEV-085` row only, plus a re-derivation of the highest `DEV-###`.
- `docs/07_implementation_status.md` — always delegate.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/GCode.cpp` — `GCode::process_layer`, for the exact ordered sequence at a layer boundary: the `;LAYER_CHANGE` / Z-height / `;HEIGHT:` reserved tags, then `before_layer_change_gcode`, then `GCode::change_layer`, then the non-BBL `time_lapse_gcode`, then `layer_change_gcode`.
- `OrcaSlicerDocumented/src/libslic3r/GCode.cpp` — `GCode::change_layer`, for what it emits **outside** spiral-vase mode: `m_writer.update_progress` (M73, when `m_layer_count > 0`), a `retract(...)` block under `retract_when_changing_layer && will_move_z(z)` with a forced `SpiralLift` under `zhtAuto`, and `m_writer.add_object_change_labels` **unconditionally**. Only `travel_to_z` is spiral-vase-only. **Borrowed as a refutation:** it establishes that canonical's three layer templates are *not* guaranteed consecutive, so PnP's adjacency must be justified on PnP's own structure (no counterpart command exists at that point in the stream) and the interleaving difference recorded as a residual. Do not restate the earlier claim that `change_layer` emits nothing.
- `OrcaSlicerDocumented/src/libslic3r/GCode.cpp` — `GCode::generate_timelapse_gcode`, for the BBL path this packet deliberately does **not** port, and its eight extra variables.
- `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` — `custom_gcode_specific_placeholders` / `s_CustomGcodeSpecificPlaceholders`, for the per-site variable sets. **Note the table's own drift and do not treat it as complete:** it keys timelapse under the parser name `timelapse_gcode`, not the config key `time_lapse_gcode`, and it omits `wrapping_detection_gcode`.
- `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` — `PrintConfigDef::init_fff_params`, for the three new options' types and defaults (all `coString`).

## Acceptance Summary

Reference, never copy, criteria from `packet.spec.md`.

- Positive: `AC-1` through `AC-14`. Change-proving: `AC-1`, `AC-2`, `AC-3`, `AC-4`, `AC-5`, `AC-6`, `AC-7`, `AC-11`, `AC-12` (row clauses), `AC-13`, `AC-14`. Explicit do-not-regress guards: `AC-8`, `AC-9`, `AC-10`, and `AC-12`'s `gen-config-docs --check` half.
- Negative: `AC-N1` (a layer macro at `PrintStart` is fatal — the criterion that proves the lookup is per-site), `AC-N2` (a `;LAYER_CHANGE` with no `;Z:` within two commands is fatal under a distinct code), `AC-N3` (packet 186's unresolved-placeholder rule extends to the new sites).
- Cross-packet impact: `InjectionPoint`, `InjectionSite` and `INJECTION_POINTS` are the surface packet 188 extends with toolchange and extrusion-role variants; the per-site variable lookup is the mechanism 188 uses to add `filament_extruder_id`, `extrusion_role` and `last_extrusion_role`. 188 must extend the enum and table, never fork them.

## Verification Commands

This is the authoritative full matrix; `packet.spec.md` lists only the four gate commands.

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo check --workspace --all-targets` | test/bench targets still compile | FACT pass/fail |
| `cargo clippy --workspace --all-targets -- -D warnings` | closure gate | FACT pass/fail |
| `cargo xtask build-guests --check` | mandatory after editing `modules/core-modules/machine-gcode-emit/src/**` and its `.toml`; rebuild without `--check` if `STALE:` | FACT clean/stale list |
| `bash -c 'cargo test -p machine-gcode-emit --test machine_gcode_emit_tdd 2>&1 \| tee target/log-187-mge.txt \| rg "^test result:" \| rg -q "^test result: ok\. [1-9]" && echo PASS \|\| echo "FAIL: see target/log-187-mge.txt"'` | whole module-test binary green (AC-8) | FACT PASS/FAIL; SNIPPETS ≤20 lines on failure |
| `bash -c 'cargo test -p slicer-runtime --test integration -- machine_start_end_gcode_emission_tdd:: 2>&1 \| tee target/log-187-msege.txt \| rg "^test result:" \| rg -q "^test result: ok\. [1-9]" && echo PASS \|\| echo "FAIL: see target/log-187-msege.txt"'` | whole e2e module green (AC-9) | FACT PASS/FAIL |
| `bash -c 'cargo test -p slicer-gcode --test golden_emit_tdd 2>&1 \| rg "^test result:" \| rg -q "^test result: ok\. [1-9]" && echo PASS \|\| echo "FAIL: golden_emit_tdd did not run or did not pass"'` | the host marker-triple golden still passes untouched | FACT PASS/FAIL |
| Each individual `--exact` command in `packet.spec.md` AC-3..AC-7 and AC-N1..AC-N3 | per-criterion proof | FACT PASS/FAIL |
| Each `python3` / `rg` / `git diff` probe in `packet.spec.md` AC-1, AC-2, AC-10..AC-14 and §Doc Impact | static, doc and no-touch proof | FACT PASS/FAIL |
| `cargo xtask gen-config-docs --check` | generated block in sync after the manifest change | FACT exit code |

## Step Completion Expectations

- The registry refactor and the re-expression of `machine_start_gcode` / `machine_end_gcode` through it must land in the **same** step. A half-migrated `run_gcode_postprocess` — table present, start/end still hand-read — satisfies AC-1's grep while leaving the table decorative, which is the exact failure AC-1's third clause exists to catch.
- `cargo xtask build-guests --check` must be run (and a rebuild performed if it reports `STALE:`) **before** any `--test integration` result is attributed to this packet. That binary instantiates the real `machine-gcode-emit.wasm`.
- `cargo xtask gen-config-docs` must run **after** the manifest edit and **before** the AC-11/AC-12 probes.
- The residual `DEV-###` ID is a ledger fact: re-derive it at the moment of writing. `TASK-306` is already allocated to this packet and must not be re-derived.
- Packet 186 must be `implemented` before this packet's Step 1. If 186 is still `draft`, stop — do not re-implement its engine here.

## Context Discipline Notes

- `crates/slicer-runtime/tests/integration/machine_start_end_gcode_emission_tdd.rs` is long. Read only `slice_with_raw` / `try_slice_with_raw`, `count_occurrences`, and the two block-position tests; do not load the whole file.
- `crates/slicer-gcode/src/emit.rs` is long and is **read-only** here. Read only the layer-boundary block that pushes the three `Raw` markers; that is the entire fact this packet needs from it.
- `docs/15_config_keys_reference.md` and `docs/DEVIATION_LOG.md` are both long and must be range-read or delegated.
- Do not read `OrcaSlicerDocumented/` directly; the five facts this packet borrows are enumerated above and each is delegable in one dispatch.
