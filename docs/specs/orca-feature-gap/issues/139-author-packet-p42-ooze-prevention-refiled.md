# 139 — Author packet P42 (re-filed) — Multimaterial / Ooze prevention keys

Type: task
Status: open
Assignee: —
Blocked by: 125
Map: ../map.md

## Question

Close **P42 — Multimaterial / Ooze prevention — emitter** — 4 keys, Tier B —
once [ticket 125](125-rule-per-tool-config-model.md) (itself gated on
[ticket 126](126-overlay-resolved-field-narrowing.md)) has ruled on the
per-tool config model: `ooze_prevention`, `standby_temperature_delta`,
`preheat_time`, `preheat_steps`.

Re-filed by
[ticket 49](49-author-packet-p42-multimaterial-ooze-prevention-emitter.md):
a packet today would be 100% declaration-only (Authoring rule 1), so no packet
number was taken and no key was declared. Key membership is unchanged from
[05-asset-packet-list.md](./05-asset-packet-list.md) P42; the authoring ticket
for these keys is this one (139), not 49 — the 28→119 / 39→136 pattern.

### Canonical grounding (ticket 49's measurement, oracle `D:\slicerProject\pinch_n_print_cli\OrcaSlicerDocumented` — re-derive the path at point of use)

All four keys are **live in canonical** (rule 3 keeps them in scope):

- `ooze_prevention` (`coBool`, default `false`): `init_ooze_prevention`
(`GCode.cpp`) enables iff set and **not** `single_extruder_multi_material`
(a P02 key absent from this tree); `OozePrevention::pre_toolchange`
parks the outgoing extruder at standby (delta branch below, else the
per-filament `idle_temperature` absolute), `post_toolchange` restores the
filament temp. `GCodeProcessor` additionally gates its preheat backtrace
on this key (see below).
- `standby_temperature_delta` (`coInt`, default `-5`, `[−max_temp,
max_temp]`): the delta applied by both ooze arms when the per-filament
`idle_temperature` entry is `0`. `idle_temperature` is Tier D deferred.
- `preheat_time` (`coFloat`, default `30.0`, `[0, 120]`) + `preheat_steps`
(`coInt`, default `1`, `[1, 10]`, dev-mode, clamped to `>= 1`): the
`GCodeProcessor` backtrace window and step count. The consumer is a
post-process pass (usage-block builder + advance-`M104` injector), gated
on `ooze_prevention && preheat_time > 0 && (is_XL_printer ||
(!single_extruder_multi_material && filament_count > 1))`.

### Tree state (ticket 49, re-derive at claim time)

- Zero occurrences of all four keys as behaviour under `crates/` /
`modules/` / `xtask/` (the sole hit is the `("ooze_prevention", "0")`
`ORCA_CONFIG_PADDING` row — rule 2, not evidence). No `ResolvedConfig`
field, no manifest row, no read site.
- The host emitter (`crates/slicer-gcode/src/emit.rs`) never emits
`Temperature` commands — only modules do (`machine-gcode-emit` start
temps) — and holds no base nozzle temperature: its only temp field is
ticket 47's unrelated `filament_flush_temp`. Canonical's standby target
derives from per-filament `nozzle_temperature` /
`nozzle_temperature_initial_layer` vectors (Tier D); the port's only
nozzle temp (`nozzle_temperature_initial_layer`) is module-manifest-side
and unreadable from the host without a seam violation.
- No `GCodeProcessor` equivalent: no usage-block builder, no backtrace
injector, no XL-printer concept, no multi-filament count source
(`filament_diameter` vectors are Tier D). `single_extruder_multi_material`
is absent (P02), so the init gate's second term is unwirable.

### Authoring obligations (same as any queue ticket)

- Use `/spec-packet-generator`; the gate is `/spec-review <packet> --preflight`.
- Apply 02's parity-evidence standard; re-derive packet number from disk per
ticket 06's rule; re-derive the owner seam per the ticket-27 lesson
(`crates/slicer-gcode` stands as tiered — emission-time — until disproven).
- Re-size under the "Packets are for complex implementation only" rule: if the
temp source exists by then and the ooze pair is declare-and-wire, implement
directly in the session instead of authoring (the ticket-42 precedent splits
the pairs if they size differently).

### Open design questions (for claim time, not now)

- **Base-temperature source for the standby arms.** Scalar host field, Tier D
vector read, or something the port already holds — decided against 125's
model, not before it. Reading the module-owned
`nozzle_temperature_initial_layer` from the host is a seam violation, not
an option.
- **`idle_temperature` interplay.** Tier D absent means the delta branch is
always taken; whether the scalar port records that as a divergence (the
DEV-169/170/171 precedent) or wires a fallback is a claim-time ruling.
- **Preheat injector scope.** Full usage-block builder vs a minimal
advance-`M104` insertion at the existing toolchange seam; the XL-gating has
no port counterpart (vendor-specific — 03-class question, do not pre-decide).
- **`single_extruder_multi_material` in the init gate.** Absent (P02); with
the default `false` the gate reduces to the bool alone — record whether that
reduction is a divergence or inert-by-default.

Resolved when all four keys are either live behind behaviour-changing decision
points with tests asserting the change at non-default values, or consciously
ruled out of scope per rule 3 with the queue count shrunk accordingly.

## Answer
