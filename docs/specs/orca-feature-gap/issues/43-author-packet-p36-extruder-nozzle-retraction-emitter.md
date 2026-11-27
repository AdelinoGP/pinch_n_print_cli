# 43 — Author packet P36 — Extruder / Nozzle / Retraction (1/2) — emitter

Type: task
Status: resolved
Assignee: wayfinder session (ses_f8b96f142ffeYCkZsSj9Tzo9J9) — claimed 2026-09-06, resolved 2026-09-06
Blocked by: 06, 101, 107
Map: ../map.md

## Question

Author the spec packet for **P36 — Extruder / Nozzle / Retraction (1/2) — emitter** — 10 keys, Tier B new logic, owner host emitter (crates/slicer-gcode). Key membership from [05-asset-packet-list.md](./05-asset-packet-list.md) (packet P36 — Extruder / Nozzle / Retraction (1/2) — emitter):

`deretraction_speed`, `long_retractions_when_cut`, `long_retractions_when_ec`, `retract_before_wipe`, `retract_length_toolchange`, `retract_lift_above`, `retract_lift_below`, `retract_lift_enforce`, `retract_restart_extra`, `retract_restart_extra_toolchange`

Authoring obligations:
- Use `/spec-packet-generator`; the authoring gate is `/spec-review <packet> --preflight` (must pass).
- Apply 02's parity-evidence standard — canonical function-read + invariant tests; `OrcaSlicerDocumented/` is readable, not runnable; unverifiable behaviour surfaces to the human first, never blocks.
- Packet number + status: derive from disk at authoring time per ticket 06's rule — ledger facts (next free number, `status: draft` vs `active`) are never frozen.
- Verify the owner's seam and the missing decision point per key (04) — re-derive from code. Work: new behaviour inside the existing owner.

Resolved when the packet is authored, preflighted, and its directory linked here.

## Answer

Packet `docs/spec_packets/276-retraction-toolchange-restart-lift-emitter/` authored (`draft`), preflight **PASS** after one BLOCKED round (6 findings, all fixed and re-verified).

**Sizing survived, membership did not.** All ten keys are zero-occurrence in `crates/`/`modules/`/`xtask/`, so Tier B holds — but the ten do not belong in one packet. Human-approved scope ruling (Q3–Q5): packet 276 carries **8 keys**; `retract_before_wipe` is **shed to P37** (ticket 44 — it partitions retraction around wipe moves this tree does not emit yet, so wiring it here would be declaration-only under Authoring rule 1); `long_retractions_when_ec` is **returned to the queue as unimplemented** (nullable per-filament bool with no geometric read site; the `_cut` placeholder wiring below names its landing pattern). Canonical spells six kept keys per-filament — declared **scalar-global** with DEV-171; the vector model stays with the ticket-125 ruling.

**Disposition of the eight** (every key drives a behaviour-changing decision point, rule 1): `retract_length_toolchange` → pre-`T<n>` synthesis length with `tool_config:` override (one intended default-output change: 2.0 → 10.0, pinned by its own test); `retract_restart_extra` / `_toolchange` → `Unretract.length` at the single construction site, variant selected by a toolchange-boundary set (preflight caught the two-site fiction); `deretraction_speed` → `Unretract.speed` as `mm/s * 60` with `0 = passthrough`; `retract_lift_above` / `_below` → ZHop execution gating on `layer_z` (`0` disables); `retract_lift_enforce` → layer-position gating with strict-parse rejection (AC-N1); `long_retractions_when_cut` → eighth `ResolvedConfig` field + `machine-gcode-emit.toml` declaration (ticket-42 shape) published via the existing placeholder seam with a key-specific `1`/`0` guarantee per ADR-0050 (the generic formatter renders word-form `Bool` — preflight caught the neighbouring-arm fiction). Recorded divergences: layer-position (not surface-role) enforce gating; firmware `G11` carries no speed so deretraction is G-code-mode-only.

**Incidental finding filed, not fixed:** the `TravelRetract` path may emit unconverted mm/s speeds (`F30` where synthesis emits `F2400`) — Step 1 confirms or refutes and files a follow-up without fixing in-packet.

Queue records updated herewith: 04 tier rows annotated, 05 P36 → 8 keys / P37 → 11 keys, ticket 44 adopted `retract_before_wipe`. No code change, no deviation rows beyond the planned DEV-171, no `ORCA_CONFIG_PADDING` edit.
