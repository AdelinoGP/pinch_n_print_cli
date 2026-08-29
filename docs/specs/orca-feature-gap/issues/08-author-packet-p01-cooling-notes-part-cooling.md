# 8 — Author packet P01 — Cooling / Notes — part-cooling

Type: task
Status: resolved
Assignee: wayfinder session (ses_fb5115d01ffeou5t0uGszua4t8)
Blocked by: 99 (resolved)
Map: ../map.md

## Question

Author the spec packet for **P01 — Cooling / Notes — part-cooling** — 19 keys (17 Tier A plumbing + 2 Tier B logic), owner part-cooling. Key membership from [05-asset-packet-list.md](./05-asset-packet-list.md) (packet P01 — Cooling / Notes — part-cooling), amended by ticket 99:

`activate_air_filtration`, `activate_chamber_temp_control`, `additional_cooling_fan_speed`, `auxiliary_fan`, `complete_print_exhaust_fan_speed`, `dont_slow_down_outer_wall`, `during_print_exhaust_fan_speed`, `fan_cooling_layer_time`, `fan_kickstart`, `fan_max_speed`, `fan_min_speed`, `fan_speedup_overhangs`, `fan_speedup_time`, `full_fan_speed_layer`, `internal_bridge_fan_speed`, `ironing_fan_speed`, `overhang_fan_threshold`, `reduce_fan_stop_start_freq`, `support_material_interface_fan_speed`

The `fan_max_speed`/`fan_min_speed` keys are the 99 reclassification: the
rename workstream (ticket 99) exposed Orca's percent (0–100) scale vs Pinch's
raw (0–255) scale, and `fan_min_speed` is declared but never read. Packet work
is the scale conversion (Tier B logic) plus wiring `fan_min_speed` to its
consumer alongside `reduce_fan_stop_start_freq`; parity evidence per 02
(canonical: `CoolingBuffer.cpp` fan-speed handling).

Authoring obligations:
- Use `/spec-packet-generator`; the authoring gate is `/spec-review <packet> --preflight` (must pass).
- Apply 02's parity-evidence standard — canonical function-read + invariant tests; `OrcaSlicerDocumented/` is readable, not runnable; unverifiable behaviour surfaces to the human first, never blocks.
- Packet number + status: derive from disk at authoring time per ticket 06's rule — ledger facts (next free number, `status: draft` vs `active`) are never frozen.
- Verify each key's decision point exists (04's mechanical proxy, refined at authoring time) — re-derive from code, don't trust the tier table. Work: declare in the owner's manifest + wire.

Resolved when the packet is authored, preflighted, and its directory linked here.

## Answer

Packet **`docs/spec_packets/253-part-cooling-fan-scale-and-cooling-keys/`** authored
(packet number derived from disk per ticket 06; `status: draft`), `/spec-review
--preflight` verdict: **PREFLIGHT PASS** (0 blockers, 0 high).

Files: `packet.spec.md`, `requirements.md`, `design.md`, `implementation-plan.md`, `task-map.md`.

### What the packet does

- Percent-normalizes `fan_max_speed`/`fan_min_speed` to Orca's 0–100 (defaults
  100/20), converting to S-values only at emission via canonical
  `GCodeWriter::set_fan`'s `floor(255.5 × p / 100)` — defaults stay byte-identical
  (S255/S51), so existing emission fixtures remain honest.
- Ports `CoolingBuffer.cpp::apply_layer_cooldown`'s `change_extruder_set_fan`
  branch chain (interpolation, ramp, stop/start suppression) with a per-layer
  time model from IR geometry, wires `fan_min_speed` (declared-but-never-read
  today) as the stop/start idle base, adds role-fan keys with canonical
  precedence and `-1` fallbacks, the `overhang_fan_threshold` enum (canonical
  default `"95%"` — the repo snapshot's stated 50% default is contradicted by a
  fresh canonical read and the packet records that discrepancy), and
  kickstart/speedup re-timing.
- Lands the 4 header/footer keys (air filtration, chamber temp, exhaust ×2) via
  co-declaration into `machine-gcode-emit` (verified existing cross-module
  co-declaration pattern) making them substitute-able placeholders in custom
  G-code templates; `auxiliary_fan`/`additional_cooling_fan_speed` drive a
  per-layer `M106 P2` channel.

### Authoring-time grounding findings (recorded in the packet, amending the tier table's assumptions)

- `dont_slow_down_outer_wall` (Tier A per 04-asset) gates a layer-slowdown
  decision point that **does not exist in this tree** — the packet declares +
  emits it and records the missing consumer as a known gap rather than
  pretending consumption. Building the slowdown stage is future work.
- `auxiliary_fan`'s canonical role is machine-capability switch for the P2
  channel (which the packet implements) rather than template-only; the
  emission-surface set is 4 template keys + `dont_slow_down_outer_wall`, not 5.
- Only 2 of the 19 keys exist in code today (as inert config-block padding
  defaults); the other 17 are genuinely new declarations.

### Canonical-path note for the map (stale ledger fact)

Ticket 02's "in-tree `OrcaSlicerDocumented/`" assumption is stale for this
clone: the checkout is the **sibling** `F:\slicerProject\pinch_n_print_cli\OrcaSlicerDocumented`
(both `pinch_n_print_cli` and `pinch_n_print_cli_2` carry one; this `_3` clone
does not). Ticket 02's fallback — "checkout assumed available on this machine" —
is what sub-agent dispatches actually used. The packet's orca-delegation
sections pin the sibling path explicitly.

### Gates

- Not run at authoring time (packet authoring only — no code changed): the
  packet's own Verification list governs its swarm; `cargo xtask
  build-guests --check` was NOT run because no guest-affecting edit happened in
  this session (manifests untouched; packet prose only).

## Answer
