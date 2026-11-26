# Requirements: 283-printer-timing-emitter

## Packet Metadata

- Grouped task IDs: `TASK-000` (queue packet, no backlog slice — wayfinder map ticket 56)
- Backlog source: `docs/07_implementation_status.md`
- Packet status: `draft`
- Aggregate context cost: `M`

## Problem Statement

P49 (Printer / Machine / Timing, host emitter) is four Tier B keys with zero live occurrences in `crates/`, `modules/`, or `xtask/`. Canonical charges filament load/unload and tool-change seconds against every filament change (`GCodeProcessor::process_filament_change`, both overloads) and prices the print hour (`GCode::update_print_estimated_stats`); this port's estimator counts `ToolChange` commands without charging them a second, and its footer stats block prices nothing. The packet builds both decision points where the port already computes them — the estimator's `ToolChange` arm and `filament_stats_comment_block` — closing P49 without a new module, IR field, or WIT change.

## In Scope

- Declare `machine_load_filament_time`, `machine_unload_filament_time`, `machine_tool_change_time` (seconds) and `time_cost` (money/h) as scalar-global `ResolvedConfig` fields in the adjacent machine-key shape (`cli_opt @printer`, `Option<f32> = None`, `extract_float_or_first`; effective default `0.0` via `unwrap_or`), with `to_config_map` arms in the existing Option-field emission loop (unset → omitted → CONFIG_BLOCK byte-stable at defaults) and `docs/config/host-keys.toml` `[resolved_config]` mirror rows (`default = 0.0` + lock-test `unwrap_or(0.0)` arms).
- Charge every `ToolChange` command `load + unload + tool_change` seconds in `estimate_command_deltas` (plain-sum PnP simplification of the canonical conditional table — DEV-175(a)); the elapsed timeline shifts with the total so M73 progress follows.
- Emit a `; printer cost = {value:.2}` footer line from `filament_stats_comment_block` only when `time_cost > 0`, with `value = time_cost * total_time_s / 3600.0` (canonical formula verbatim; the label is PnP-invented — DEV-175(b)).
- Emitter-side validation: negative timing or cost values reject the slice with a stable error (ticket-113 class: canonical min-0 is a GUI hint, enforcement here is a deliberate divergence — DEV-175(c)).
- Regenerate `docs/15_config_keys_reference.md` via `cargo xtask gen-config-docs`; add DEV-175 row; no `ORCA_CONFIG_PADDING` edit (rule 2).
- One new auto-discovered test file `crates/slicer-gcode/tests/printer_timing_stats_tdd.rs` carrying schema + behaviour + negative pins (no aggregator edit — `slicer-gcode` tests are auto-discovered per `Cargo.toml`).

## Out of Scope

- Per-filament vectors for any kept key — canonical declares all four scalar `coFloat`, so there is nothing to vectorise; ticket 125 is not engaged.
- The canonical filament-cost total (`; filament cost` / `; total filament cost` footer lines, `filament_cost` coFloats) — `filament_cost` is Tier D deferred with no in-tree source; the packet prices the printer hour only (DEV-175(d)). No collision: no drafted packet owns the cost footer.
- `ToolOrdering::build_filament_group_context`'s load/unload reads — verified stats-parameters only (no grouping, geometry, or tool-order effect); not borrowed, not wired.
- The canonical first-change/same-extruder conditionals — no extruder/nozzle model exists in tree (ticket-39/136 territory); the plain sum covers every `ToolChange` uniformly (DEV-175(a)).
- Per-tool overrides beyond the automatic `tool_config:<idx>:` composition the `declare_resolved_config!` macro already threads (ticket-126 precedent: overlay arms are generated per field, no allowlist edit).
- `ORCA_CONFIG_PADDING` twins, module-manifest declarations, `FeedrateConfig`/`SPEED_KEYS` membership (these are estimate additions, not role base speeds), and any change to the M73 injection shape itself.

## Authoritative Docs

- `docs/00_project_overview.md` - delegated SUMMARY (emitter as the modular-pipeline emission seam)
- `docs/01_system_architecture.md` - delegated SUMMARY (claim system non-applicability: no algorithm selection here, rule 4 does not fire)
- `docs/specs/orca-feature-gap/issues/04-asset-tier-assignment.md` - direct read of P49 rows (tier B, owner `crates/slicer-gcode (estimator.rs)`)
- `docs/specs/orca-feature-gap/issues/02-parity-evidence-standard.md` - delegated SUMMARY (canonical function-read + invariant-test standard)

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` — `PrintConfigDef::init_fff_params`: the four declarations (all `coFloat`, default `0.0`)
- `OrcaSlicerDocumented/src/libslic3r/GCode/GCodeProcessor.cpp` — `GCodeProcessor::process_filament_change` (both overloads): conditional composition (initial = load only; same-extruder swap = unload + load; extruder switch = tool-change + conditional load/unload)
- `OrcaSlicerDocumented/src/libslic3r/GCode.cpp` — `GCode::update_print_estimated_stats`: `total_cost += config.time_cost.getFloat() * (normal_print_time/3600.0)` over the filament-cost total; footer labels `; filament cost` / `; total filament cost` (no printer-cost footer label exists)

<!-- snippet: parity-evidence -->
## Parity Evidence Standard

Every key this packet implements carries evidence per the map's ticket 02 standard:

- **Canonical read + described behaviour.** For each key, cite the canonical consumer (file + function, never line numbers) and describe its behaviour in `requirements.md`. Reads of `OrcaSlicerDocumented/` are delegated per the orca-delegation snippet.
- **Invariants, not goldens.** Behaviour is pinned with invariant/property tests (counts preserved, mappings hold, emitted values equal expected). Golden G-code comparison is not part of the standard — the checkout is not built and cannot be run.
- **Ported Orca tests are acceptable evidence.** When `OrcaSlicerDocumented/tests/fff_print/` covers the behaviour, port its assertions into PnP's suite with the standard porting header (`docs/ORCASLICER_ATTRIBUTION.md`).
- **Plumbing keys** (a threshold feeding an existing decision point): the default resolves to the canonical value AND a test proves the value reaches the consumer. No behavioural test required.
- **Unverifiable behaviour:** surface the key and the reason to the human first; only with their sign-off file a `docs/DEVIATION_LOG.md` row (single source of truth, CI-checked by `cargo xtask check-deviations`) and proceed with documented scope. Never defer the key or block the packet on unverifiability alone, and never file a row without the human having been asked.

## Acceptance Summary

Reference, never copy, criteria from `packet.spec.md`.

- Positive: `AC-1` through `AC-7`; AC-3 pins the plain-sum charge (`2 + 3 + 5 = 10.0 s`) at non-default values; AC-4 pins elapsed/M73 propagation; AC-5 pins the `time_cost * T / 3600` value; AC-2/AC-6 pin default-path identity (no added seconds, no cost line, CONFIG_BLOCK byte-stable via `None`-omission).
- Negative: `AC-N1` (negative timing rejected), `AC-N2` (negative cost rejected), `AC-N3` (no padding twin — honest absence).
- Cross-packet impact: none at defaults (all four unset → estimator adds `0.0`, footer gains no line, CONFIG_BLOCK gains no line); DEV-175 is behaviour-only, not a default mismatch, so the deviation gate shows no new default row.

## Verification Commands

This is the authoritative full matrix; `packet.spec.md` lists only 2-3 gate commands.

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo test -p slicer-gcode --test printer_timing_stats_tdd 2>&1 \| tail -5` | schema + behaviour + negatives in one auto-discovered binary | FACT pass/fail; SNIPPETS <=20 lines on failure |
| `cargo test -p slicer-gcode --test estimator 2>&1 \| tail -3` | no-regression on existing estimator pins | FACT pass/fail |
| `cargo test -p slicer-gcode --test m73 2>&1 \| tail -3` | no-regression on M73 + footer stats block | FACT pass/fail |
| `cargo test -p slicer-gcode --test gcode_emit_tdd 2>&1 \| tail -3` | no-regression on general emission | FACT pass/fail |
| `cargo check --workspace --all-targets 2>&1 \| tail -3` | struct-literal blast radius (new ResolvedConfig fields) | FACT pass/fail |
| `cargo clippy --workspace --all-targets -- -D warnings 2>&1 \| tail -3` | lint gate | FACT pass/fail |
| `cargo xtask check-literals 2>&1 \| tail -3` | struct-literal churn gate (test code FRU/`exhaustive` waiver) | FACT pass/fail |

Commands must have small, parseable output suitable for delegation.

## Step Completion Expectations

- Step order is declaration → behaviour (Steps 2–3: charge first, then footer + validation) → docs/deviation (Step 4); Steps 2–3 must not land before the schema guard proves the four keys resolve (otherwise the charge reads fallbacks).
- `host-keys.toml`, the lock-test arms, and `gen-config-docs` regen land in the same step as the DEV-175 row so the lock test and the deviation gate observe one coherent tree.
- No `run_slice` e2e driver is required: the estimator charge and the footer block are pure functions of (`GCodeIR`, limits, costs) pinnable at unit level (the `estimator` + `m73` test precedents construct IR streams directly).

## Context Discipline Notes

- Tempting large reads to skip: `crates/slicer-ir/src/resolved_config.rs` (macro invocation — read only the `cli_opt @printer` machine-key window plus the Option-field `to_config_map` loop); `crates/slicer-gcode/src/estimator.rs` (read only the `ToolChange` arm + `PrintEstimate` shape); `OrcaSlicerDocumented/` (delegate always).
- `target/*.gcode` echoes of these keys are generated output, not evidence of liveness — never cite them as reads.
