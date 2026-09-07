# 50 — Author packet P43 — Multimaterial / Prime tower — emitter

Type: task
Status: resolved
Assignee: wayfinder session (ses_f87dc4c86ffeu5OPQmBe4z8dHq) — claimed 2026-09-06, resolved 2026-09-06
Blocked by: 06, 101, 107
Map: ../map.md

## Question

Author the spec packet for **P43 — Multimaterial / Prime tower — emitter** — 2 keys, Tier B new logic, owner host emitter (crates/slicer-gcode). Key membership from [05-asset-packet-list.md](./05-asset-packet-list.md) (packet P43 — Multimaterial / Prime tower — emitter):

`manual_filament_change`, `single_extruder_multi_material_priming`

Authoring obligations:
- Use `/spec-packet-generator`; the authoring gate is `/spec-review <packet> --preflight` (must pass).
- Apply 02's parity-evidence standard — canonical function-read + invariant tests; `OrcaSlicerDocumented/` is readable, not runnable; unverifiable behaviour surfaces to the human first, never blocks.
- Packet number + status: derive from disk at authoring time per ticket 06's rule — ledger facts (next free number, `status: draft` vs `active`) are never frozen.
- Verify the owner's seam and the missing decision point per key (04) — re-derive from code. Work: new behaviour inside the existing owner.

Resolved when the packet is authored, preflighted, and its directory linked here.

## Answer

**Split at claim time under the "Packets are for complex implementation
only" rule: `manual_filament_change` implemented directly (no packet);
`single_extruder_multi_material_priming` returned to the queue as
unimplemented** (the ticket-40/42 shape).

### Canonical grounding (oracle `D:\slicerProject\pinch_n_print_cli\OrcaSlicerDocumented` — re-derive at point of use)

Both keys pass Authoring rule 3 (live in `libslic3r/`, stay in scope):

- `manual_filament_change` (`coBool`, default `false`): three behaviour
sites — `GCodeWriter::toolchange_prefix` (emits the `; MANUAL_TOOL_CHANGE
T<n>` tag line instead of the bare `T<n>` / BBL `M1020`), the `GCode.cpp`
toolchange path (skips `change_filament_gcode` when
`m_toolchange_count == 1`), and a `GCodeProcessor` comment-analysis arm
(post-hoc analysis, no emission — unwired, noted not deviated).
- `single_extruder_multi_material_priming` (`coBool`, default `false`):
four sites, all inside `WipeTowerType::Type2` priming-tower flows
(initial-extruder selection skipping the priming towers, the
`has_single_extruder_multi_material_priming` placeholder, the
`set_extruder` skip, `m_wipe_tower->prime()` emission).

### `manual_filament_change` — live

Both emission seams already existed; the tier table's `crates/slicer-gcode`
owner holds for the tag line, and the skip lands module-side (the
ticket-27 split-owner shape — one key, two seams, both small):

- `ResolvedConfig.manual_filament_change` (`cli` bool, default false) +
`to_config_map` `Bool` arm (the module reads it through its
manifest-declared schema) + `PartialEq`/`Hash` arms
(`crates/slicer-ir/src/resolved_config.rs`).
- `DefaultGCodeSerializer.manual_filament_change` flag (default false) +
`with_manual_filament_change` builder (the `with_support_line_width`
precedent) + `ToolChange` arm emitting `; MANUAL_TOOL_CHANGE T<n>`
verbatim (`crates/slicer-gcode/src/serialize.rs`); wired in `run_slice`
(`crates/slicer-runtime/src/run.rs`). Default path byte-identical.
- `[config.schema.manual_filament_change]` bool default false
(`machine-gcode-emit.toml`); `run_gcode_postprocess` skips the
`FilamentChange` injection point at 1-based `toolchange_count == 1` only
— `FilamentEnd`/`FilamentStart` still inject
(`modules/core-modules/machine-gcode-emit/src/lib.rs`).
- 6 new tests: resolved default + map carriage + explicit-true round-trip
(`resolved_config_defaults_tdd.rs`); tag-line emission + bare-`T1` absence
(`gcode_emit_tdd.rs`); first-only skip + `ToolChange` re-emission +
`FilamentEnd` unaffected (`machine_gcode_emit_tdd.rs`); manifest guard
(`manual_filament_change_config_schema_tdd.rs`).
- CONFIG_BLOCK gains exactly one line at defaults
(`; manual_filament_change = false`, the tree-wide word-form-bool
spelling whose `true` read-back fix rides ticket 132 — no spot fix).
No deviation rows.

### `single_extruder_multi_material_priming` — returned to the queue

Its entire subject (Type2 priming towers, `WipeTowerIntegration::prime`)
does not exist in this tree — the tower is purge-scanlines-only (ticket
29 census) and `single_extruder_multi_material` itself is an absent P02
key. Wiring the bool alone would be declaration-only (rule 1). It
sequences after [122](122-author-packet-prime-tower-body-parity.md)
(prime tower body parity), the same seat `wipe_tower_filament` holds
(ticket 29). No key declared, no packet number taken for it.

### Records

- `04-asset-tier-assignment.md`: `manual_filament_change` row annotated
live (ticket 50); `single_extruder_multi_material_priming` row annotated
returned-to-queue, sequenced after ticket 122.
- `05-asset-packet-list.md`: P43 now covers 1 key
(`manual_filament_change`); priming key returned to the queue.
- Gates: `cargo check`, `cargo clippy --workspace --all-targets` clean,
`cargo xtask check-literals` clean; `slicer-ir` / `slicer-gcode` /
`machine-gcode-emit` suites green; guests rebuilt (`slicer-ir` sits in
every guest's dependency closure); runtime `contract` 296/296 green
post-rebuild; runtime `integration` 345/345 green (one infra-only
stale-`pnp_cli`-binary failure, fixed by rebuilding the binary, re-ran
green); e2e 139/144 — the 5 failures are a **pre-existing HEAD
regression from ticket 42's commit**, not this ticket: real Orca fixtures
carry `pressure_advance` / `enable_pressure_advance` as per-filament
vectors which ticket 42's committed strict scalar extractors reject at
config resolution (proven via `git show HEAD` + fixture bytes, no
stash cycle needed; my paths are default-inert and resolution fires
before emission). Filed as
[140](140-e2e-pressure-advance-vector-ingest-regression.md), blocked on
125 (ticket 42's own answer already rides vector ingest there).
