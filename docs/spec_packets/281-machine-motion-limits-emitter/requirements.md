# Requirements: 281-machine-motion-limits-emitter

## Packet Metadata

- Grouped task IDs: `[]` (wayfinder queue packet; ledger owner is ticket 54)
- Backlog source: `docs/specs/orca-feature-gap/issues/54-author-packet-p47-printer-machine-motion-limits-emitter.md`
- Packet status: `draft`
- Aggregate context cost: `M`

## Problem Statement

P47 (Printer / Machine / Motion limits, Tier B, 9 family entries / 18 scalar keys) is the machine-limit half of the host emitter story. Packet 267 builds the envelope seam (`M203`/`M204 P/T`/`M205` over the ten already-declared `machine_max_speed_*` / `machine_max_jerk_*` / `machine_max_acceleration_extruding|travel` fields) and explicitly excludes the P47 fields; eight scalars have zero occurrences anywhere in the tree (per-axis accelerations, retracting acceleration, junction deviation, both minimum rates). Without this packet every one of those keys is declaration-nowhere: configuring them changes nothing. This packet makes each of them drive a behaviour-changing decision point — envelope lines for the six maximum keys, estimator clamps for the two minimum keys — or the packet fails its own Authoring-rule gates.

## In Scope

- Eight new `ResolvedConfig` scalar-global `Option<f32>` fields with `@printer` CLI binding and vector-tolerant first-wins ingest (`extract_float_or_first`, ticket-140 precedent): `machine_max_acceleration_x`, `machine_max_acceleration_y`, `machine_max_acceleration_z`, `machine_max_acceleration_e`, `machine_max_acceleration_retracting`, `machine_max_junction_deviation`, `machine_min_extruding_rate`, `machine_min_travel_rate`.
- `to_config_map` arms for the eight fields, matching the ten live siblings (present when `Some`, absent when `None`).
- Envelope extension in `crates/slicer-gcode` on 267's builder: `M201 X Y Z E` (all four accel fields required), `M204 R` component (retracting; legacy-Marlin travel fallback preserved), `M205 J` via the existing `GcodeFlavor::set_junction_deviation` helper (emitted only when JD > 0), canonical line order `M201 → M203 → M204 → M205/M566 → M205 J`, RRF ×60 factor on `M203`/`M566` only.
- `EstimatorLimits` extension with `min_extruding_rate` / `min_travel_rate` (defaults `0.0` = disabled) and segment-feedrate clamping for extruding vs travel classes.
- Bounds rows: minimum `0` for all eight fields (no maxima — canonical maxima are GUI hints per the ticket-113 ruling, including JD's `0.3`).
- Recorded divergence `DEV-173`: canonical declares all of these as `coFloats` stride-2 normal/stealth pairs selected by `silent_mode`; the port ingests the first entry as a scalar-global and models no stealth variant until ticket 117 lands.
- Generated config-reference regen covering the eight keys.

## Out of Scope

- `silent_mode` in any form (no field, no manifest row, no emitter read) — stays in the queue under ticket 117; pinned by AC-N2.
- The ten already-live fields (`machine_max_speed_x/y/z/e`, `machine_max_jerk_x/y/z/e`, `machine_max_acceleration_extruding`, `machine_max_acceleration_travel`) — owned by packet 267; this packet reads but does not redeclare them.
- Per-extruder vector ingestion — stays with ticket 125's per-tool model; scalar-global is the ruled interim (DEV-173).
- `ORCA_CONFIG_PADDING` edits of any kind (Authoring rule 2 — the table is never a deliverable).
- Estimator modelling of per-axis acceleration or junction deviation (canonical `GCodeWriter` runtime caps have no port estimator counterpart; envelope-only is the ruled scope).
- Input-shaping emission (`M593` family, P48 scope) even though it sits textually adjacent in `print_machine_envelope`.

## Authoritative Docs

- `docs/01_system_architecture.md` - delegated SUMMARY (host emitter owns emission-time decisions; modules own claim-held geometry).
- `docs/03_wit_and_manifest.md` - delegated SUMMARY (host-only keys need no manifest row; P18/P96-AC-8 precedent).
- `docs/specs/orca-feature-gap/map.md` - direct read of Notes (Authoring rules 1–6, packets-for-complex-only rule, ticket-140 vector-ingest fix, ticket-113 GUI-hint ruling).
- `docs/spec_packets/267-printer-machine-power-recovery-emitter/packet.spec.md` - direct read of Goal + Scope + AC-1..AC-4 (envelope seam this packet extends; P47 exclusion list).

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/GCode.cpp` — `GCode::print_machine_envelope` (canonical M201/M203/M204/M205 order, flavor gate, RRF ×60 factor, legacy-Marlin travel fallback)
- `OrcaSlicerDocumented/src/libslic3r/GCodeWriter.cpp` — `GCodeWriter::set_junction_deviation` (M205 J emission only when JD > 0) and the M204/M205 command forms
- `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` — canonical defaults and minima for the eight new keys (accel x/y 1000, z 500, e 5000; retracting 1500; JD 0.01 max 0.3; min rates 0) and the `printer_options_with_variant_2` stride-2 set

## Acceptance Summary

Reference, never copy, criteria from `packet.spec.md`.

- Positive: `AC-1` through `AC-8`; each of the eight new keys is exercised at a non-default value in at least one AC (rule 6b): accel x/y/z/e in AC-1, retracting in AC-2, JD in AC-3, min rates in AC-4, flavor/scale/default/field-surface in AC-5..AC-8.
- Negative: `AC-N1` (range rejection), `AC-N2` (`silent_mode` zero-occurrence).
- Cross-packet impact: envelope output with all eight fields `None` is byte-identical to packet 267's output (AC-7); 267's AC-3/AC-4 keep passing unchanged.

## Verification Commands

This is the authoritative full matrix; `packet.spec.md` lists only 2-3 gate commands.

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo test -p slicer-gcode --test gcode_emit_tdd machine_envelope 2>&1 \| tee target/test-output.log \| grep -E '^test result'` | Envelope lines incl. 267's (no regression) | FACT pass/fail; SNIPPETS <=20 lines on failure |
| `cargo test -p slicer-gcode --test estimator 2>&1 \| tee target/test-output.log \| grep -E '^test result'` | Estimator incl. min-rate clamp | FACT pass/fail |
| `cargo test -p slicer-ir --test resolved_config_defaults_tdd 2>&1 \| tee target/test-output.log \| grep -E '^test result'` | Field defaults + map surface | FACT pass/fail |
| `cargo test -p slicer-scheduler --test integration 2>&1 \| tee target/test-output.log \| grep -E '^test result'` | Bounds rejection arm | FACT pass/fail |
| `cargo check --workspace --all-targets` | Blast-radius compile proof | FACT pass/fail |
| `cargo clippy --workspace --all-targets -- -D warnings` | Lint gate | FACT pass/fail |
| `cargo xtask check-literals` | Struct-literal gate for new fields | FACT pass/fail |

## Step Completion Expectations

Step 1 (config surface) lands before Step 2 (envelope) — the envelope tests construct configs with the new fields. Step 2 lands before Step 4 (close-out regen) — the reference regen must see the final defaults. The 267-dependency check in Step 2 is a hard precondition: if 267's builder is absent, stop with `[BLOCK]`, do not reimplement the envelope here.

## Context Discipline Notes

Packet-specific hazards: `crates/slicer-ir/src/resolved_config.rs` is over 2000 lines — read only the `to_config_map` window (~lines 270–300) and the `cli_opt` window (~lines 2180–2210) plus the macro arm shape; delegate everything else. `GCode.cpp::print_machine_envelope` is never loaded directly — delegate. The `ResolvedConfig {` literal blast radius (20 files) is enumerated by a `LOCATIONS` dispatch inside Step 1, not by browsing.
