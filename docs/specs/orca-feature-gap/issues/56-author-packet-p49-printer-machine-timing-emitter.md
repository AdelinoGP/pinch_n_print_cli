# 56 — Author packet P49 — Printer / Machine / Timing — emitter

Type: task
Status: resolved
Assignee: Adelino Penedo
Blocked by: 06, 101, 107
Map: ../map.md

## Question

Author the spec packet for **P49 — Printer / Machine / Timing — emitter** — 4 keys, Tier B new logic, owner host emitter (crates/slicer-gcode). Key membership from [05-asset-packet-list.md](./05-asset-packet-list.md) (packet P49 — Printer / Machine / Timing — emitter):

`machine_load_filament_time`, `machine_tool_change_time`, `machine_unload_filament_time`, `time_cost`

Authoring obligations:
- Use `/spec-packet-generator`; the authoring gate is `/spec-review <packet> --preflight` (must pass).
- Apply 02's parity-evidence standard — canonical function-read + invariant tests; `OrcaSlicerDocumented/` is readable, not runnable; unverifiable behaviour surfaces to the human first, never blocks.
- Packet number + status: derive from disk at authoring time per ticket 06's rule — ledger facts (next free number, `status: draft` vs `active`) are never frozen.
- Verify the owner's seam and the missing decision point per key (04) — re-derive from code. Work: new behaviour inside the existing owner.

Resolved when the packet is authored, preflighted, and its directory linked here.

## Answer

**Authored as packet 283** (`docs/spec_packets/283-printer-timing-emitter/`, `draft`), preflight **PASS** (S0–S8 + AC-command + Doc-Impact; one self-review fix before the gate: the behaviour step touched 4 files, split into Step 2 charge + Step 3 footer/validation; one re-verify retraction during the gate — three first-pass `N` greps were wrong-file operator error, all three symbols confirmed on re-run).

Claim-time re-derivation kept Tier B with all **4 keys in, none shed, no code change**. All four zero-occurrence in `crates/`, `modules/`, `xtask/`; owner `crates/slicer-gcode` stands (ticket-27 hazard checked: canonical reads are `GCodeProcessor` time estimation + `GCode` stats accumulation — emission/statistics-time, not placeholder publication, so `machine-gcode-emit` is the wrong seam). All four pass rule 3 (live in `libslic3r/`); all four canonical scalar `coFloat` default `0.0` (no ticket-125 vector model).

Canonical grounding (oracle `pinch_n_print_cli/OrcaSlicerDocumented`, delegated): `machine_load/unload_filament_time` via `GCodeProcessor::apply_config` → `get_filament_load/unload_time` → both `process_filament_change` overloads (conditional table: initial = load only, same-extruder swap = unload + load, switch = tool-change + conditional load/unload); `machine_tool_change_time` via `get_extruder_change_time` into the same overloads; `time_cost` via `GCode::update_print_estimated_stats` (`total_cost += time_cost * normal_print_time/3600`). `ToolOrdering::build_filament_group_context` reads load/unload into stats params only — not borrowed, no grouping effect, no fog.

Packet shape: 4 scalar-global `ResolvedConfig` fields in the adjacent machine-key shape (`cli_opt @printer`, `Option<f32> = None` — the 282 plain-`f32` shape rejected: always-emitted fields would add 4 CONFIG_BLOCK lines with no padding twin to shadow; `None`-omission keeps defaults byte-identical) + `to_config_map` loop extension + `host-keys.toml` mirror (`0.0` + lock-test `unwrap_or(0.0)` arms) + per-`ToolChange` plain-sum charge (DEV-175(a)) + gated `; printer cost` footer line at the verbatim formula (DEV-175(b)) + negative rejection (DEV-175(c)) + filament-cost total omitted, `filament_cost` Tier D (DEV-175(d)); one new auto-discovered test file. 04/05 rows confirmed unchanged. No new fog graduated.
