# 58 — Author packet P51 — Quality / Precision — emitter

Type: task
Status: resolved
Assignee: wayfinder session (ses_f85245964ffegenz4pCORVdVCA) — claimed 2026-09-07, resolved 2026-09-07
Blocked by: 06, 101, 107
Map: ../map.md

## Question

Author the spec packet for **P51 — Quality / Precision — emitter** — 2 keys, Tier B new logic, owner host emitter (crates/slicer-gcode). Key membership from [05-asset-packet-list.md](./05-asset-packet-list.md) (packet P51 — Quality / Precision — emitter):

`enable_arc_fitting`, `resolution` (the latter re-adjudicated from the rename pool in ticket 105: canonical `resolution` is a generation-time global simplify — `PerimeterGenerator.cpp` `ex.simplify_p`, `Brim.cpp`, `Fill.cpp`, `Layer.cpp`, `PrintObjectSlice.cpp`, `Print.cpp`, `TreeSupport` — plus emit-side arc density in `GCodeWriter.cpp`; the host's emit-time per-role `gcode_resolution` is a different decision point that stays. Packet grounding decides where the generation-time decision lands; check the `ORCA_CONFIG_PADDING` `("resolution", "0.012")` entry in `crates/slicer-gcode/src/serialize.rs` against canonical default 0.01 while in there)

Authoring obligations:
- Use `/spec-packet-generator`; the authoring gate is `/spec-review <packet> --preflight` (must pass).
- Apply 02's parity-evidence standard — canonical function-read + invariant tests; `OrcaSlicerDocumented/` is readable, not runnable; unverifiable behaviour surfaces to the human first, never blocks.
- Packet number + status: derive from disk at authoring time per ticket 06's rule — ledger facts (next free number, `status: draft` vs `active`) are never frozen.
- Verify the owner's seam and the missing decision point per key (04) — re-derive from code. Work: new behaviour inside the existing owner.

Resolved when the packet is authored, preflighted, and its directory linked here.

## Answer

Packet authored: [`docs/spec_packets/284-quality-precision-emitter/`](../../../spec_packets/284-quality-precision-emitter/),
`status: draft`, `PREFLIGHT PASS` (S0–S8 clean; AC-runnable + Doc-Impact clean; one self-retraction during the gate — S2 initially misread DEV-176's presence in the packet's own five files as a collision, re-check proves it appears only there, absent from `docs/DEVIATION_LOG.md` and every other packet).

**Claim-time sizing: Tier B held, membership held — both keys in, none shed, none returned, no code change.**
Both keys are zero-occurrence as behaviour (`enable_arc_fitting` undeclared per `classify_declared_key`; bare `resolution` only in `ORCA_CONFIG_PADDING` as `("resolution", "0.012")`); owner `crates/slicer-gcode` stands (ticket-27 hazard checked — `machine-gcode-emit`'s generic sweep is the wrong seam and manifest rows would be dead under `ConfigView::from_declared`, ticket-34 shape).

**Authoring-time grounding findings (recorded in the packet):**
- Canonical defaults grounded against the map oracle (`D:\slicerProject\pinch_n_print_cli\OrcaSlicerDocumented`, verified to exist; `Orca(pnp_gui)` confirmed unusable): `enable_arc_fitting` `coBool` default `0` (false), `resolution` `coFloat` default `0.01` min `0` no max — the padding `0.012` is stale by `0.002`.
- `resolution` lands **once at emission** as the effective-tolerance selection (arc off → `max(per_role, resolution)`, arc on → `min(per_role, 0.2 * resolution)` per canonical `PerimeterGenerator`), not as per-module plumbing — the PnP better seam under Authoring rule 4 (DEV-176(a)); small globals floor at per-role tolerances.
- Arc fitting is **emitter-side** coalescing into `Raw` `G2`/`G3` (XY-plane, same-Z, extrusion-only, travel/lock-excluded, E-conserving). Serializer-side fitting rejected — `Move` carries no `order_lock`, so it cannot honour ADR-0063; a new `GCodeCommand::Arc` variant rejected as IR blast radius for two keys.
- `enable_arc_fitting` is **host-only omitted** from `to_config_map` (P35 `enable_pressure_advance` / P18 `disable_m73` precedent); the canonical `0`/`1` spelling debt rides ticket 132, not a spot-fix (DEV-176(d)). `resolution` is emitted and shadows the stale padding row at runtime via `emit_config_kv` dedup — one intended default value change (`0.012` → `0.01`), line count unchanged; the padding table itself is untouched (rule 2, AC-N3).
- Negative `resolution` rejects via the emitter's stable error (ticket-113 class: canonical min is a GUI hint — DEV-176(c)). Both keys scalar in canonical, so no ticket-125 vector model.
- Packet number derived from disk per ticket 06 (`283` → `284`); DEV-176 is first collision-free (LOG max 171, drafts propose 172–175 — re-derive `max(DEV-*)` before writing the row).

### Gates

- Not run at authoring time (packet authoring only — no code changed): the packet's own Verification list governs its swarm; `cargo xtask build-guests --check` was NOT run because no guest-affecting edit happened in this session (host-only prose + no IR/WIT/manifest-schema touch; ticket-08 precedent).
