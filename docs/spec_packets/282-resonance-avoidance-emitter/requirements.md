# Requirements: 282-resonance-avoidance-emitter

## Packet Metadata

- Grouped task IDs: `TASK-000` (queue packet, no backlog slice — wayfinder map ticket 55)
- Backlog source: `docs/07_implementation_status.md`
- Packet status: `draft`
- Aggregate context cost: `M`

## Problem Statement

P48 (Printer / Machine / Resonance, host emitter) is three Tier B keys with zero live occurrences in `crates/`, `modules/`, or `xtask/` — only generated `target/*.gcode` CONFIG_BLOCK echoes of values the tree never reads. Canonical adjusts external-perimeter extrusion speeds when avoidance is enabled (`GCode::_extrude`); this port emits every OuterWall move at its role speed untouched. The packet builds that decision point in the host emitter, where per-move feedrates are already selected, closing P48 without a new module, IR field, or WIT change.

## In Scope

- Declare `resonance_avoidance` (bool, canonical default `false`), `min_resonance_avoidance_speed` (float, `70.0`), `max_resonance_avoidance_speed` (float, `120.0`) as scalar-global `ResolvedConfig` fields with `host-keys.toml` mirror rows (packet-267 precedent: authoritative default lives in the Rust consumer, mirrored for docs + lock test).
- Build the OuterWall-only feedrate adjustment in `DefaultGCodeEmitter::resolve_feedrate`, applied to the factored speed after ADR-0052 clamping: below max and below midpoint `(min+max)/2` → `min(speed, min)`; below max and at/above midpoint → `max`; above max → unchanged (loop-disable); avoidance off or non-OuterWall → unchanged.
- Emitter-side validation: negative min/max rejected; min above max rejected; both with stable errors and unit pins (ticket-113 class: canonical min-0 is a GUI hint, enforcement here is a deliberate divergence — DEV-174(a)).
- Regenerate `docs/15_config_keys_reference.md` via `cargo xtask gen-config-docs`; add DEV-174 row; no `ORCA_CONFIG_PADDING` edit (rule 2).
- One new auto-discovered test file `crates/slicer-gcode/tests/resonance_avoidance_emission_tdd.rs` carrying schema + behaviour + negative pins (no aggregator edit — `slicer-gcode` tests are auto-discovered per `Cargo.toml`).

## Out of Scope

- Per-filament vectors for any kept key — canonical declares all three scalar, so there is nothing to vectorise; ticket 125 is not engaged.
- `M593` / input-shaping emission — canonical's P48 reads adjust feedrates only; 281's requirements names the M593 family "P48 scope" textually, but no M593 read site belongs to these keys. Not built, not declared.
- The in-block volumetric re-cap (`filament_max_volumetric_speed / _mm3_per_mm`) — Tier D per-filament data with no emitter-path source; omitted and recorded in DEV-174(b).
- `ref_speed` vs `speed` as distinct inputs — canonical disables on the pre-cap reference then adjusts the post-cap speed; with the cap omitted the two coincide, so the port uses the single factored speed for both (DEV-174(c)).
- Per-tool overrides beyond the automatic `tool_config:<idx>:` composition the `declare_resolved_config!` macro already threads (ticket-126 precedent: overlay arms are generated per field, no allowlist edit).
- `ORCA_CONFIG_PADDING` twins, module-manifest declarations, `FeedrateConfig`/`SPEED_KEYS` membership (these are clamps on role speeds, not base speeds), and any change to 267/281's envelope builders.

## Authoritative Docs

- `docs/00_project_overview.md` - delegated SUMMARY (emitter as the modular-pipeline emission seam)
- `docs/01_system_architecture.md` - delegated SUMMARY (claim system non-applicability: no algorithm selection here, rule 4 does not fire)
- `docs/specs/orca-feature-gap/issues/04-asset-tier-assignment.md` - direct read of P48 rows (tier B, owner stands)
- `docs/specs/orca-feature-gap/issues/02-parity-evidence-standard.md` - delegated SUMMARY (canonical function-read + invariant-test standard)

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` — `PrintConfigDef::init_fff_params`: the three declarations (types, defaults, GUI-only min-0)
- `OrcaSlicerDocumented/src/libslic3r/GCode.cpp` — `GCode::_extrude`: the resonance block quoted verbatim in `design.md` (enable gate, max-disable, half-range adjustment, per-loop reset)

## Acceptance Summary

Reference, never copy, criteria from `packet.spec.md`.

- Positive: `AC-1` through `AC-7`; AC-3/AC-4 pin the two adjustment halves at midpoint `95.0` for defaults (80→70, 100→120); AC-5 pins the above-max disable; AC-6 pins the OuterWall-only gate; AC-7 pins docs + lock-test coherence.
- Negative: `AC-N1` (negative rejected), `AC-N2` (inverted range rejected), `AC-N3` (no padding twin — honest absence).
- Cross-packet impact: none on 267/281 output at defaults (avoidance defaults off → identity, AC-2); DEV-174 is behaviour-only, not a default mismatch, so the deviation gate shows no new default row.

## Verification Commands

This is the authoritative full matrix; `packet.spec.md` lists only 2-3 gate commands.

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo test -p slicer-gcode --test resonance_avoidance_emission_tdd 2>&1 \| tail -5` | schema + behaviour + negatives in one auto-discovered binary | FACT pass/fail; SNIPPETS <=20 lines on failure |
| `cargo test -p slicer-gcode --test gcode_feedrate_emission_tdd 2>&1 \| tail -3` | no-regression on existing feedrate emission | FACT pass/fail |
| `cargo test -p slicer-gcode --test gcode_emit_tdd 2>&1 \| tail -3` | no-regression on general emission | FACT pass/fail |
| `cargo check --workspace --all-targets 2>&1 \| tail -3` | struct-literal blast radius (new ResolvedConfig fields) | FACT pass/fail |
| `cargo clippy --workspace --all-targets -- -D warnings 2>&1 \| tail -3` | lint gate | FACT pass/fail |
| `cargo xtask check-literals 2>&1 \| tail -3` | struct-literal churn gate (test code FRU/`exhaustive` waiver) | FACT pass/fail |

Commands must have small, parseable output suitable for delegation.

## Step Completion Expectations

- Step order is declaration → behaviour → docs/deviation; the behaviour step must not land before the schema guard proves the three keys resolve (otherwise the adjustment reads fallbacks).
- `host-keys.toml` and `gen-config-docs` regen land in the same step as the DEV-174 row so the lock test and the deviation gate observe one coherent tree.
- No `run_slice` e2e driver is required: `resolve_feedrate` is a pure emitter function pinnable at unit level (the `gcode_feedrate_emission_tdd` precedent constructs the emitter directly).

## Context Discipline Notes

- Tempting large reads to skip: `crates/slicer-ir/src/resolved_config.rs` (1821-line macro invocation — read only the field-syntax window plus the macro's arm-generation note); `crates/slicer-gcode/src/emit.rs` (read only `resolve_feedrate` + call site); `OrcaSlicerDocumented/` (delegate always).
- `target/*.gcode` echoes of these keys are generated output, not evidence of liveness — never cite them as reads.
