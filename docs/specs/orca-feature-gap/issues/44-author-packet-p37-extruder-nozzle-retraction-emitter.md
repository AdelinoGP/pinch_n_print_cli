# 44 — Author packet P37 — Extruder / Nozzle / Retraction (2/2) — emitter

Type: task
Status: resolved
Assignee: wayfinder session (ses_f8b63e9acffe11ks1RdGwhLDzt) — claimed 2026-09-06, resolved 2026-09-07
Blocked by: 06, 101, 107
Map: ../map.md

## Question

Author the spec packet for **P37 — Extruder / Nozzle / Retraction (2/2) — emitter** — 11 keys, Tier B new logic, owner host emitter (crates/slicer-gcode). Key membership from [05-asset-packet-list.md](./05-asset-packet-list.md) (packet P37 — Extruder / Nozzle / Retraction (2/2) — emitter; `retract_before_wipe` adopted from P36 by ticket 43, since it partitions retraction around the wipe moves this packet's `wipe`/`wipe_distance` keys build):

`retract_before_wipe`, `retract_when_changing_layer`, `retraction_distances_when_cut`, `retraction_distances_when_ec`, `retraction_minimum_travel`, `travel_slope`, `use_firmware_retraction`, `wipe`, `wipe_distance`, `z_hop_types`, `z_offset`

Authoring obligations:
- Use `/spec-packet-generator`; the authoring gate is `/spec-review <packet> --preflight` (must pass).
- Apply 02's parity-evidence standard — canonical function-read + invariant tests; `OrcaSlicerDocumented/` is readable, not runnable; unverifiable behaviour surfaces to the human first, never blocks.
- Packet number + status: derive from disk at authoring time per ticket 06's rule — ledger facts (next free number, `status: draft` vs `active`) are never frozen.
- Verify the owner's seam and the missing decision point per key (04) — re-derive from code. Work: new behaviour inside the existing owner.

Resolved when the packet is authored, preflighted, and its directory linked here.

## Answer

Packet `docs/spec_packets/277-retraction-wipe-travel-firmware-emitter/` authored (`draft`), preflight **PASS** after two S7 rounds (new-guard-binary home, then the Step 2/3 split it forced — both fixed and re-verified).

**Sizing survived, membership held.** All eleven keys are zero-occurrence as configuration-driven behaviour in `crates/`/`modules/`/`xtask/` (`wipe` hits are overlay labels, `z_offset` is a serializer literal only), so Tier B holds — and unlike P36, no key leaves the packet: the wipe trio lands together as one new emission site (this tree emits no wipe move at all, so `retract_before_wipe` finally gets the moves ticket 43 was missing), and the `_cut`/`_ec` distances publish through the `machine-gcode-emit` placeholder seam (276's `_cut`-bool precedent, generalized to float spelling). Canonical spells nine of the eleven keys per-filament — declared **scalar-global** with DEV-172; the vector model stays with the ticket-125 ruling.

**Disposition of the eleven** (every key drives a behaviour-changing decision point, rule 1): `retraction_minimum_travel` → `needs_retraction`-style travel gate (one intended default-output change: short travels newly skip retracts); `retract_when_changing_layer` → layer-change retract gate; `wipe` + `wipe_distance` → new wipe `Move` emission; `retract_before_wipe` → percent split (pre = total × value/100); `use_firmware_retraction` → `RetractMode::Firmware` selection (`G10`/`G11` render already exists); `z_hop_types` + `travel_slope` → hop-style arms with `hop_height / tan` diagonal math (second intended default-output change: default lift is newly slope-shaped) and strict-parse rejection (AC-N1); `z_offset` → preamble + layer-change Z shift; `retraction_distances_when_cut`/`_ec` → eighth/ninth `ResolvedConfig` fields + `machine-gcode-emit.toml` declarations published via the existing placeholder seam (canonical's EC-null state not represented — unset means default). Recorded divergences: scalar-global (DEV-172), snake-case hop strings, slope-default lift shape. No code change.

**Preflight note for future authoring:** the S7 re-check caught the new-binary-home defect twice — first the guard binary with no step home, then (after the fix) the existing-binary guard case with no edit-list slot. Resolution: Step 2 split into fields (Step 2) + proof (Step 3); the shared `retraction_keys_schema_tdd` guard rides Step 6 with 276's bool case as queue-order merge churn.

Queue records updated herewith: 04 `_cut`/`_ec` distance rows narrowed to the placeholder seam, 05 P37 → packet 277. No queue-count change.
