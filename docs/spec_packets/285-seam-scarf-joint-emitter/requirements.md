# Requirements: 285-seam-scarf-joint-emitter

## Packet Metadata

- Grouped task IDs: `TASK-000` (queue packet; no `docs/07` slice — backlog source is the wayfinder map ticket below)
- Backlog source: `docs/specs/orca-feature-gap/issues/59-author-packet-p52-quality-seam-emitter.md`
- Packet status: `draft`
- Aggregate context cost: `M`

## Problem Statement

OrcaSlicer's scarf-joint seam path — overlapped sloped seam segments with flow/speed modulation, smoothness and overhang gating, loop start/end gap clipping, and role-based wipe-speed selection — has no counterpart in this port. The emitter (`DefaultGCodeEmitter::emit_gcode`) emits wall loops verbatim as placed upstream by the seam placer's `LayerModule::run_wall_postprocess` (`modules/core-modules/seam-placer/src/lib.rs`); there is no scarf stage, no wipe path (draft packet 277 builds it), no gap clipping, and no slope-conditional logic. All eight P52 keys are true zero-occurrence gaps (verified 2026-09-07: seven with no occurrence under `crates/`/`modules/`/`xtask/`; `seam_gap` only as a padding twin plus an unrelated test fn name). The tier table (04) tiers all eight B with owner `crates/slicer-gcode (GCode::extrude_loop clipping)`; claim-time re-derivation confirms both the tier (new logic in the existing owner — the stage does not exist) and the owner (every canonical read site is emission-time `GCode`/`Wipe` code; the ticket-27 hazard is checked — `machine-gcode-emit`'s generic sweep is the wrong seam and manifest rows there would be dead under `ConfigView::from_declared`, so declaration rides the host `ResolvedConfig` + `machine-gcode-emit.toml` schema surface per the ticket-42 P35 precedent, while the decision points live in `crates/slicer-gcode`).

## In Scope

- Eight scalar-global `ResolvedConfig` fields — seven deliberately OMITTED from `to_config_map` (host-only emission control: the emitter reads the typed fields directly, modules never see these keys; ticket-42 P35 precedent), `seam_gap` carried by a `to_config_map` arm so its live value shadows the padding twin (284's `resolution` precedent) — plus `docs/config/host-keys.toml` `[resolved_config]` rows + `machine-gcode-emit.toml` `[config.schema.*]` tables, at canonical defaults/bounds (table in `design.md`).
- The scarf/slope emission stage in `DefaultGCodeEmitter::emit_gcode`: `has_scarf_joint_seam` master gate; `seam_slope_conditional` smoothness (`scarf_angle_threshold`) + overhang (`scarf_overhang_threshold`) gates; scarf-path flow (`scarf_joint_flow_ratio`) and speed-cap (`scarf_joint_speed`) modulation; `seam_gap` loop clipping + slope termination.
- The `role_based_wipe_speed` selection arm on draft 277's wipe `Move` (FORWARD-DEP; activation-blocked until 277 lands).
- Bounds rejection for the five ranged keys (ticket-113 rule) and the `order_lock` bypass (ADR-0062/0063 conformance).
- Re-baselining of fixtures whose defaults move under live `seam_gap` clipping, with measured justification (never silent updates).
- One `docs/DEVIATION_LOG.md` row (DEV-177, four clauses).

## Out of Scope

- P53's slope-variant/wipe keys (`seam_slope_type`, `seam_slope_*`, `wipe_on_loops`, … — queued under ticket 60): they extend this stage when authored; this packet does not declare or consume them.
- The wipe emission site itself (draft packet 277): AC-6 is activation-blocked on it and adds no second wipe path.
- `seam_gap`'s Fill-side concentric-clip arm (canonical `Layer::make_fills` / `generate_sparse_infill_polylines_for_anchoring` → concentric filler): the port has no concentric filler, so that arm is recorded unimplemented (DEV-177(b)), not built.
- A GCodeProcessor/viewer pipeline for `has_scarf_joint_seam` recognition: the port has none; the flag's viewer role is recorded unimplemented (DEV-177(a)) while its enable-gate role is wired.
- The CONFIG_BLOCK reader contract (ticket 132): bool word-form and percent spellings ride 132; this packet emits canonical spellings without fixing the reader.
- Per-tool/per-extruder vectors for any key (ticket 125): all eight are canonical scalars; the scalar-global subset is parity, not a divergence.

## Authoritative Docs

- `docs/00_project_overview.md` - delegated SUMMARY (modular-pipeline constraint: scarf stays an emitter-internal path transform, not a module/claim)
- `docs/08_coordinate_system.md` - direct range read (gap/length mm↔unit math)
- `docs/ORCASLICER_ATTRIBUTION.md` - direct short read (only if an Orca test is ported; otherwise unused)

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/GCode.cpp` — `GCode::extrude_loop` scarf/slope gating, smoothness test, overhang test, and seam-gap clipping shape (borrow the gate order; port it emitter-side)
- `OrcaSlicerDocumented/src/libslic3r/GCode.cpp` — `GCode::_extrude` scarf flow-ratio multiplication and scarf-speed capping shape (borrow the factor/cap application point)
- `OrcaSlicerDocumented/src/libslic3r/GCode.cpp` — `Wipe::wipe` + `Wipe::calculateWipeRetractionLengths` role-speed-vs-configured selection (reconcile with draft 277's wipe `Move` before wiring AC-6)
- `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` — the eight keys' declared types/defaults/bounds (confirm, do not re-derive the packet's table without this read)

<!-- snippet: parity-evidence -->
## Parity Evidence Standard

Every key this packet implements carries evidence per the map's ticket 02 standard:

- **Canonical read + described behaviour.** For each key, cite the canonical consumer (file + function, never line numbers) and describe its behaviour in `requirements.md`. Reads of `OrcaSlicerDocumented/` are delegated per the orca-delegation snippet.
- **Invariants, not goldens.** Behaviour is pinned with invariant/property tests (counts preserved, mappings hold, emitted values equal expected). Golden G-code comparison is not part of the standard — the checkout is not built and cannot be run.
- **Ported Orca tests are acceptable evidence.** When `OrcaSlicerDocumented/tests/fff_print/` covers the behaviour, port its assertions into PnP's suite with the standard porting header (`docs/ORCASLICER_ATTRIBUTION.md`).
- **Plumbing keys** (a threshold feeding an existing decision point): the default resolves to the canonical value AND a test proves the value reaches the consumer. No behavioural test required.
- **Unverifiable behaviour:** surface the key and the reason to the human first; only with their sign-off file a `docs/DEVIATION_LOG.md` row (single source of truth, CI-checked by `cargo xtask check-deviations`) and proceed with documented scope. Never defer the key or block the packet on unverifiability alone, and never file a row without the human having been asked.

## Canonical Behaviour Per Key (delegated grounding, 2026-09-07, map oracle)

- `has_scarf_joint_seam` (coBool, default false): canonical reads it ONLY in `GCodeProcessor::apply_config` (both overloads) so layer detection recognises tagged scarf-joint output; `Print::apply` derives/writes it as configuration plumbing. There is NO `extrude_loop` read — the tier table's consumer citation is corrected here (04's citation-fix note anticipated this). Behaviour in canonical: marks a print as containing scarf seams (viewer recognition). **Port role (deliberate divergence, DEV-177(a)):** the port has no viewer pipeline and the generation-side slope selector lives with the P53 slope keys (ticket 60, unauthored), so this packet wires the flag as the scarf stage's master enable gate instead — default `false` keeps the stage inert (today's behaviour), `true` both enables scarf emission and marks the output as scarf-jointed. P53 reconciles the selector dimension when authored; this flag remains the master enable.
- `role_based_wipe_speed` (coBool, default true): canonical `Wipe::calculateWipeRetractionLengths` + `Wipe::wipe` select the current extrusion-role speed instead of the configured wipe speed when enabled. Behaviour: wipe feedrate source selection.
- `scarf_angle_threshold` (coInt, default 155, range 0–180°): canonical `GCode::extrude_loop` converts to radians and passes to the loop-smoothness test; conditional scarf stays enabled only on loops meeting the threshold. Behaviour: smoothness gate.
- `scarf_joint_flow_ratio` (coFloat, default 1, range 0–2): canonical `GCode::_extrude` multiplies volumetric flow on sloped/scarf paths. Behaviour: scarf-path flow scaling.
- `scarf_joint_speed` (coFloatOrPercent, default 100%, min 1): canonical `GCode::_extrude` caps external/internal scarf perimeter speed and the reference speed for sloped estimation; accepts mm/s or percent. Behaviour: scarf-path speed cap.
- `scarf_overhang_threshold` (coPercent, default 40%, min 0): canonical `GCode::extrude_loop` disables scarf when estimated unsupported overhang is not below the percentage of wall line width. Behaviour: overhang-support gate.
- `seam_gap` (coFloatOrPercent, default 10%, min 0): canonical `GCode::extrude_loop` loop clipping + scarf slope termination; `Layer::make_fills` + `generate_sparse_infill_polylines_for_anchoring` feed the resolved distance into concentric-fill loop clipping; `ExtrusionLoopSloped::ExtrusionLoopSloped` drops the terminal no-extrusion segment. Percent base is nozzle diameter (`get_abs_value(nozzle_diameter)`), not line width. Behaviour: start/end gap + slope termination (emitter arm wired here; concentric arm recorded unimplemented).
- `seam_slope_conditional` (coBool, default false): canonical `GCode::extrude_loop` gates the angle-smoothness test and the overhang-threshold test — when enabled, scarf is restricted to smooth, supported perimeters. Behaviour: conditional restriction switch.

## Acceptance Summary

Reference, never copy, criteria from `packet.spec.md`.

- Positive: `AC-1` through `AC-6`; no refinements beyond their Given/When/Then text. AC-6 is activation-blocked on draft packet 277 (FORWARD-DEP).
- Negative: `AC-N1` (bounds rejection, ticket-113 class) through `AC-N2` (`order_lock` bypass, ADR-0062/0063).
- Cross-packet impact: draft 277 gains one consumer arm (AC-6) with reconciled names — no edit to 277's files; ticket 60 (P53) extends this stage when authored — no edge added.

## Verification Commands

This is the authoritative full matrix; `packet.spec.md` lists only 2-3 gate commands.

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo test -p slicer-gcode --test seam_scarf_joint_emission_tdd 2>&1 \| tee target/test-output.log \| tail -5` | all eight ACs + both negative cases | FACT pass/fail; failing-test name + ≤20 lines on failure |
| `cargo test -p slicer-gcode --test gcode_emit_tdd 2>&1 \| tee target/test-output.log \| tail -3` | no regression in the existing emit suite (seam_gap default clip is the only intended delta) | FACT pass/fail |
| `cargo xtask gen-config-docs 2>&1 \| tail -3` | regenerate `docs/15` after the schema/host-keys edits (Step 1b) | FACT pass/fail |
| `cargo xtask gen-config-docs --check 2>&1 \| tail -3` | generated tables fresh at closure (Step 6) | FACT pass/fail |
| `cargo check --workspace --all-targets 2>&1 \| tail -3` | whole-tree type check incl. test targets | FACT pass/fail |
| `cargo clippy --workspace --all-targets -- -D warnings 2>&1 \| tail -3` | lint gate | FACT pass/fail |
| `cargo xtask check-literals 2>&1 \| tail -3` | struct-literal churn gate (ResolvedConfig fields added) | FACT pass/fail |

## Step Completion Expectations

Step order is load-bearing: declare (Steps 1a–1b) before behaviour (Steps 2–5); bounds rejection (AC-N1) lands with the Step 2 emitter gate (284's DEV-176(c) precedent — canonical bounds are GUI hints, so the port's stable-error validator is new emission logic, not declaration metadata); the `role_based_wipe_speed` field is declared in Step 1a but its arm (Step 5) stays behind the 277 FORWARD-DEP — if 277 has not landed when this packet implements, Step 5 is skipped at implementation and re-entered after 277, with AC-6 as its exit. Fixture re-baselining (Step 6) runs last and must show the seam_gap clip as the sole default-path delta with measured justification per fixture.

## Context Discipline Notes

Packet-specific hazards: `crates/slicer-gcode/src/emit.rs` is over 300 lines — implementer reads only the bounded sites in `design.md`; `OrcaSlicerDocumented/` is delegation-only (see orca-delegation snippet); the `declare_resolved_config!` macro expansion is large — extend by the macro's own field syntax, never by reading the full expansion.
