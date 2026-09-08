# 63 — Author packet P56 — Speed / Acceleration — emitter

Type: task
Status: resolved
Assignee: wayfinder session (ses_20260907_P56) — claimed 2026-09-07, resolved 2026-09-07
Blocked by: 06, 101, 107
Map: ../map.md

## Question

Author the spec packet for **P56 — Speed / Acceleration — emitter** — 11 keys, Tier B new logic, owner host emitter (crates/slicer-gcode). Key membership from [05-asset-packet-list.md](./05-asset-packet-list.md) (packet P56 — Speed / Acceleration — emitter):

`accel_to_decel_enable`, `accel_to_decel_factor`, `bridge_acceleration`, `default_acceleration`, `initial_layer_acceleration`, `inner_wall_acceleration`, `internal_solid_infill_acceleration`, `outer_wall_acceleration`, `sparse_infill_acceleration`, `top_surface_acceleration`, `travel_acceleration`

Authoring obligations:
- Use `/spec-packet-generator`; the authoring gate is `/spec-review <packet> --preflight` (must pass).
- Apply 02's parity-evidence standard — canonical function-read + invariant tests; `OrcaSlicerDocumented/` is readable, not runnable; unverifiable behaviour surfaces to the human first, never blocks.
- Packet number + status: derive from disk at authoring time per ticket 06's rule — ledger facts (next free number, `status: draft` vs `active`) are never frozen.
- Verify the owner's seam and the missing decision point per key (04) — re-derive from code. Work: new behaviour inside the existing owner.

Resolved when the packet is authored, preflighted, and its directory linked here.

## Answer

Packet authored: [`docs/spec_packets/289-speed-acceleration-emitter/`](../../../spec_packets/289-speed-acceleration-emitter/),
`status: draft`, `PREFLIGHT PASS` (S0 five files non-empty; S1 281 `status: draft` — described as draft FORWARD-DEP with no satisfied-dep claim; S2 `DEV-181` absent from the log, format-conformant, collision-free — LOG max DEV-171, drafts claim 172–180; S3 no schema versions; S4 no new ADRs; S5 all symbols resolve — `set_acceleration(u32)->String` / `set_travel_acceleration(u32)->Option<String>` / `supports_separate_travel_acceleration` in `flavor.rs`, `emit_gcode` + per-entity `entity.path.role` loop + `layer.global_layer_index` in `emit.rs` (no M204 emitted today), `declare_resolved_config!` + `extract_float_or_percent` + `ResolvedFloatOrPercent` + `to_config_map` in `resolved_config.rs`, `gcode_flavor`→`with_flavor` in `run.rs`; S6 all fourteen `ExtrusionRole` variants resolve, WIT untouched; S7 file-per-binary auto-discovered (15 files, no aggregator), guard self-contained with named re-baseline home `gcode_emit_tdd.rs` within the 3-edit cap; S8 no ADR-governed surface contradicted).

**Claim-time sizing: Tier B held, owner stands, membership held 11-in — no code change.**
All eleven keys live in canonical (none dead, no alias) and zero-occurrence as behaviour here (`set_acceleration`/`set_travel_acceleration` exist as unwired builders taking `u32`, never fed a config value; `emit_gcode` emits no accel command — verified by grep). Every canonical read site for the eleven is emission-time `GCode::_extrude` / `GCode::travel_to` / `GCodeWriter::set_acceleration_internal`, so `crates/slicer-gcode` stands (ticket-27 hazard checked — `machine-gcode-emit`'s sweep publishes placeholders, motion commands are the host emitter's job). Tier table owner text corrected from `estimator.rs` (time-math only) to the emission stage.

**Authoring-time grounding findings (recorded in the packet):**
- Canonical defaults/bounds grounded against the map oracle: `default_acceleration` 500 / `travel_acceleration` 10000 / `inner_wall_acceleration` 10000 / `initial_layer_acceleration` 300 / `outer_wall_acceleration` 500 / `top_surface_acceleration` 500 (all coFloat nullable, min 0); percent trio `bridge_acceleration` 50% over outer / `sparse_infill_acceleration` 100% over default / `internal_solid_infill_acceleration` 100% over default (coFloatsOrPercents, min 0); `accel_to_decel_enable` coBool `true`; `accel_to_decel_factor` coPercent 50 (min 1, max 100).
- Chain order: first-layer → bridge → sparse → internal-solid → outer → inner → top-surface → default, `> 0` fallthrough per arm, `default_acceleration > 0` master gate (whole stage inert at 0); travels default to `travel_acceleration` via the separate-travel flavor gate; Klipper suffix `ACCEL_TO_DECEL = accel * factor / 100` under the enable bool (Repetier ignores it; Bambu emits none — port has no Bambu flavor).
- Scalar-global is a recorded simplification DEV-181(b): canonical declares the nine accel keys per-nozzle nullable vectors (`NOZZLE_CONFIG`); the vector model stays with ticket 125, consistent with packets 276/277/279–288. Short-travel/first-layer-travel/wipe-tower/Calib-PA arms are `[FWD]`, not stubs.
- Defaults NOT identity: canonical `default_acceleration` is `500 > 0`, so the default stream newly emits `M204 P500` / `M204 T10000` (flavor-adjusted) — the intended default output change, pinned by AC-2 with measured re-baseline in Step 3.
- Stream position: 281's envelope opens the stream; this packet's per-path lines follow it (FORWARD-DEP on draft 281, never redeclared); shared code is reuse of the existing flavor arms, never a fork — P59 (jerk) reuses them later under canonical names.
- Twin shadowing: zero padding twins for all eleven (verified by grep; header bans speed/accel/jerk keys) — host-only omitted (ticket-42 precedent), table untouched.
- Packet number `288` → `289` derived from disk per ticket 06; DEV-181 first collision-free (LOG max 171, drafts 172–180).
- 04 tier rows corrected (estimator.rs → emission stage, packet 289); 05 P56 section annotated (11-in at 289); no queue-count change. No new fog graduated, nothing ruled out of scope.

### Gates

- Not run at authoring time (packet authoring only — no code changed): the packet's own Verification list governs its swarm; `cargo xtask build-guests --check` was NOT run because no guest-affecting edit happened in this session (host-only prose + docs annotations, no IR/WIT/manifest-schema touch; ticket-60 precedent).
