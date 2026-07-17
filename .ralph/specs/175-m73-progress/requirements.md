# Requirements: 175-m73-progress

## Packet Metadata

- Grouped task IDs: `TASK-279`
- Backlog source: `docs/07_implementation_status.md` (row minted at closure via `task-map.md`)
- Packet status: `draft`
- Aggregate context cost: `M`
- Plan source: `docs/specs/fork-gaps-wave2-plan.md` (Packet 175 — fork handoff item 15)

## Problem Statement

The fork's print hosts and firmware read `M73` remaining-time lines and the `; filament used` / `; estimated printing time` comment block to drive progress bars and material accounting; PNP emits neither (verified: zero `M73` occurrences under `crates/`). Packet 169 builds the trapezoidal estimator but explicitly excludes M73 and names this packet as its wave-2 unblock. This is one coherent slice: everything here is a pure consumer of 169's `PrintEstimate`, layered onto the already-emitted `GCodeIR` command stream.

## In Scope

- Extend 169's estimator with a per-command elapsed-time variant `estimate_print_with_elapsed` (cumulative seconds after each `GCodeIR.commands` index) without changing the physics model.
- New `crates/slicer-gcode/src/m73.rs`: `inject_m73` inserting `Raw` command pairs (`M73 P<pct> R<min>` immediately followed by `M73 Q<pct> S<min>`, identical values) at stream start (`P0 R<total_min>`), after each `;LAYER_CHANGE` `Raw` marker (deduplicated: skip when both pct and minute value are unchanged since the last emission), and at stream end (`P100 R0`); `pct = round(elapsed/total*100)`, `min = round(remaining_s/60)` (Orca `time_in_minutes` semantics).
- `filament_stats_comment_block` rendering `; filament used [mm]`, `; filament used [cm3]`, `; filament used [g]` (density-gated), and `; estimated printing time (normal mode) = <get_time_dhms-style>` from `PrintEstimate`; appended to the command stream end as `Raw` commands.
- New config key `disable_m73` (bool, default `false`, snake_case) in `ResolvedConfig`; `true` suppresses all M73 lines but never the comment block.
- Wiring at the existing estimator call site inside `DefaultGCodeEmitter::emit_gcode` (`crates/slicer-gcode/src/emit.rs:757-758`) so every emitter caller (`run_slice`, postpass, tests) gets the emission; `postpass.rs` (which only stashes the already-filled IR at :49-51) is not edited.
- Doc rows: `docs/15_config_keys_reference.md` + `docs/ORCA_CONFIG_REFERENCE.md` marker flip.

## Out of Scope

- Any change to the estimator's physics, `PrintEstimate` field semantics, or 169's `slice_stats` event (169 owns those).
- Per-G1-line M73 emission (Orca's `process_line_move` granularity) — layer-boundary + first/last only; recorded as a deliberate deviation in `design.md`.
- `; filament cost` line and any cost field (fork exclusion, mirrors 169).
- Stealth-mode having a *different* estimate (Orca runs a second time-machine; PNP emits the same estimate under both masks per the approved plan).
- G-code flavor conditionality of M73 (packet 171 owns flavor; M73 emitted unconditionally-by-flavor here).
- `; estimated first layer printing time` and BBL `; model printing time` variants.
- WIT, manifests, guest WASM.

## Authoritative Docs

- `docs/15_config_keys_reference.md` — delegated grep; gains `disable_m73`.
- `docs/ORCA_CONFIG_REFERENCE.md` — delegated grep for the `disable_m73` row.
- `.ralph/specs/169-time-estimator-slice-stats/` — predecessor packet (read-only); consume export names via SUMMARY dispatch, never edit.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/GCode/GCodeProcessor.cpp` — M73 masks in the `GCodeProcessor` constructor (`"M73 P%s R%s\n"` normal, `"M73 Q%s S%s\n"` stealth), `run_post_process`'s `format_line_M73_main` disable gate, first/last-line placeholder behavior (`M73 P0 R<total>` / `M73 P100 R0`) in `process_placeholders`, dedup-on-changed-value in `process_line_move`, and `get_time_dhms`-style time formatting for `; estimated printing time (normal mode) = ...`.
- `OrcaSlicerDocumented/src/libslic3r/GCode.cpp` — `update_print_stats_and_format_filament_stats` for the exact `; filament used [mm]/[cm3]/[g]` block shape (the `; filament cost` line is deliberately NOT borrowed — the fork excludes cost).
- `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` — `disable_m73` definition (coBool, default false, comAdvanced).

## Acceptance Summary

- Positive: `AC-1` through `AC-5`. Refinements: AC-1's dedup clause is the likeliest silent regression (an M73 pair at *every* boundary regardless of value change would still pass a naive grep); AC-3's `[g]` values must be `volume_cm3 × filament_density` — never derived from the serializer's header-only `filament_density_g_cm3` default `1.24` field.
- Negative: `AC-N1` (disable gate), `AC-N2` (density-absent omission).
- Cross-packet impact: consumes 169's exports by exact name (already in tree: `estimator.rs:168/:91/:24`, emit-site fill at `emit.rs:757-758`) — if 169's remaining closure work renames or moves them, this packet's design must be reconciled before activation. Packet 171 (flavor) later owns making M73 flavor-conditional; this packet keeps emission flavor-agnostic so 171 can wrap it.

## Verification Commands

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `mkdir -p target && cargo test -p slicer-gcode --test m73 2>&1 \| tee target/test-output.log \| grep -E "^test result\|FAILED"` | All unit ACs (1, 2, 3, N2) | FACT pass/fail; SNIPPETS ≤20 lines on failure |
| `mkdir -p target && cargo test -p pnp-cli --test m73_progress_tdd 2>&1 \| tee target/test-output.log \| grep -E "^test result\|FAILED"` | E2E ACs (4, N1) through `run_slice` | FACT pass/fail |
| `rg -q 'disable_m73' docs/15_config_keys_reference.md && ! rg -q '"disable_m73".*coBool.*❌' docs/ORCA_CONFIG_REFERENCE.md && echo PASS` | AC-5 doc rows (definition row only; category rows 1063/1750 carry no marker) | FACT PASS/absent |
| `cargo check --workspace --all-targets` | Gate | FACT pass/fail |
| `cargo clippy --workspace --all-targets -- -D warnings` | Gate | FACT pass/fail |

## Step Completion Expectations

Step order matters: the estimator elapsed-time extension (Step 1) must land before `inject_m73` (Step 2) because Step 2's tests build their expected pct/min values from Step 1's returned vector. The emit-site wiring step must not run its e2e test before `cargo xtask build-guests --check` confirms guests are fresh (the fixture slice dispatches core modules).

## Context Discipline Notes

- `crates/slicer-gcode/src/serialize.rs` is ~800 lines — read only the command-loop arms (~lines 560-750) and never the CONFIG_BLOCK padding tables (~lines 400-490).
- `crates/slicer-ir/src/resolved_config.rs` is ~1000 lines — read only the macro-invocation block around the existing `cli "support_enabled" ... => extract_bool;` line (~line 723) and the `filament_density` line (~line 792) as the two patterns to mirror.
- Never open `.ralph/specs/169-time-estimator-slice-stats/implementation-plan.md`; consume 169 via a single SUMMARY dispatch of its `design.md` export list.
