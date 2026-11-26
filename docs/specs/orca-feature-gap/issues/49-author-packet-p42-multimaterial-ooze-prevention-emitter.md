# 49 — Author packet P42 — Multimaterial / Ooze prevention — emitter

Type: task
Status: resolved
Assignee: wayfinder session (ses_f87dc4c86ffeu5OPQmBe4z8dHq) — claimed 2026-09-06, resolved 2026-09-06
Blocked by: 06, 101, 107
Map: ../map.md

## Question

Author the spec packet for **P42 — Multimaterial / Ooze prevention — emitter** — 4 keys, Tier B new logic, owner host emitter (crates/slicer-gcode). Key membership from [05-asset-packet-list.md](./05-asset-packet-list.md) (packet P42 — Multimaterial / Ooze prevention — emitter):

`ooze_prevention`, `preheat_steps`, `preheat_time`, `standby_temperature_delta`

Authoring obligations:
- Use `/spec-packet-generator`; the authoring gate is `/spec-review <packet> --preflight` (must pass).
- Apply 02's parity-evidence standard — canonical function-read + invariant tests; `OrcaSlicerDocumented/` is readable, not runnable; unverifiable behaviour surfaces to the human first, never blocks.
- Packet number + status: derive from disk at authoring time per ticket 06's rule — ledger facts (next free number, `status: draft` vs `active`) are never frozen.
- Verify the owner's seam and the missing decision point per key (04) — re-derive from code. Work: new behaviour inside the existing owner.

Resolved when the packet is authored, preflighted, and its directory linked here.

## Answer

**Re-sized at claim time: not authorable now, all four keys re-filed, no
packet, no code change** (the ticket-28/39/41/45 shape).

Claim-time grounding from the tree, not the tier table (map Notes,
"Packets are for complex implementation only"):

- **Zero occurrences as behaviour.** None of the four keys appears under
`crates/` / `modules/` / `xtask/` except the `("ooze_prevention", "0")`
`ORCA_CONFIG_PADDING` row (`crates/slicer-gcode/src/serialize.rs`) —
which Authoring rule 2 rejects as evidence. No `ResolvedConfig` field,
no manifest row, no read site for any of them.
- **All four pass Authoring rule 3 (live in canonical, stay in scope).**
`ooze_prevention` (`coBool`, default `false`): `init_ooze_prevention`
(`GCode.cpp` — enabled iff set and **not**
`single_extruder_multi_material`, itself a P02 key absent from this
tree) plus the `GCodeProcessor` backtrace-enabled gate; the emission
sites are `OozePrevention::pre_toolchange` (parks the outgoing extruder
at standby) / `post_toolchange` (restores it). `standby_temperature_delta`
(`coInt`, default `-5`): the delta applied by both arms when the
per-filament `idle_temperature` entry is `0` (that key is Tier D
deferred — see below). `preheat_time` (`coFloat`, default `30.0`,
`[0, 120]`) + `preheat_steps` (`coInt`, default `1`, `[1, 10]`,
dev-mode; clamped to `>= 1`): the `GCodeProcessor` backtrace window and
step count — a post-process pass that inserts advance `M104` preheats
ahead of toolchanges, itself gated on `ooze_prevention && preheat_time >
0 && (XL-printer || (multi-filament && !SEMM))`.
- **Neither pair is wireable today, for different missing seams:**
  1. *Ooze pair.* Canonical computes the standby target from per-filament
  `nozzle_temperature` / `nozzle_temperature_initial_layer` vectors
  (Tier D deferred). This tree's host side holds exactly one temperature
  field (`filament_flush_temp` — ticket 47's unrelated placeholder key);
  every real nozzle temp lives module-side
  (`nozzle_temperature_initial_layer` on `machine-gcode-emit`), which the
  host emitter cannot read without a seam violation, and the host emitter
  (`crates/slicer-gcode/src/emit.rs`) never emits `Temperature` commands
  at all — only modules do. There is no base temperature to apply the
  delta to.
  2. *Preheat pair.* This tree has no `GCodeProcessor` equivalent — no
  usage-block builder, no backtrace injector, no XL-printer concept, no
  `filament_count > 1` source (`filament_diameter` vectors are Tier D).
- Declaring any of the four now would be 100% declaration-only
(Authoring rule 1). Owner stands as tiered (`crates/slicer-gcode`,
emission-time — the canonical reads are `GCode.cpp` / `GCodeProcessor`,
so no ticket-27 correction); re-derive at claim time.

**Re-filed as
[139](139-author-packet-p42-ooze-prevention-refiled.md), blocked on 125**
(itself gated on 126) — the 28→119 / 39→136 pattern. One ticket for all
four keys: the ooze pair needs the per-filament temp vectors 125 rules
on, and the preheat injector is designed in the same sitting once the
temp source exists. No packet number taken, no key declared, no code
change, no deviation row. Docs-only change: no build, clippy, or test
gates affected.
