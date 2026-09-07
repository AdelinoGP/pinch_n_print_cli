# 59 — Author packet P52 — Quality / Seam (1/2) — emitter

Type: task
Status: resolved
Assignee: wayfinder session (ses_f826d3608ffeWqVDEkHWZRQVxU) — claimed 2026-09-07, resolved 2026-09-07
Blocked by: 06, 101, 107
Map: ../map.md

## Question

Author the spec packet for **P52 — Quality / Seam (1/2) — emitter** — 8 keys, Tier B new logic, owner host emitter (crates/slicer-gcode). Key membership from [05-asset-packet-list.md](./05-asset-packet-list.md) (packet P52 — Quality / Seam (1/2) — emitter):

`has_scarf_joint_seam`, `role_based_wipe_speed`, `scarf_angle_threshold`, `scarf_joint_flow_ratio`, `scarf_joint_speed`, `scarf_overhang_threshold`, `seam_gap`, `seam_slope_conditional`

Authoring obligations:
- Use `/spec-packet-generator`; the authoring gate is `/spec-review <packet> --preflight` (must pass).
- Apply 02's parity-evidence standard — canonical function-read + invariant tests; `OrcaSlicerDocumented/` is readable, not runnable; unverifiable behaviour surfaces to the human first, never blocks.
- Packet number + status: derive from disk at authoring time per ticket 06's rule — ledger facts (next free number, `status: draft` vs `active`) are never frozen.
- Verify the owner's seam and the missing decision point per key (04) — re-derive from code. Work: new behaviour inside the existing owner.

Resolved when the packet is authored, preflighted, and its directory linked here.

## Answer

Packet authored: [`docs/spec_packets/285-seam-scarf-joint-emitter/`](../../../spec_packets/285-seam-scarf-joint-emitter/),
`status: draft`, `PREFLIGHT PASS` (S0–S8 clean after three gate rounds: round 1 caught a fictional `SeamPlacer::run` — real symbol is the `LayerModule::run_wall_postprocess` trait-method impl — plus Step-1 edit-cap, log-tee, overhang-bound, `none`-Doc-Impact, `to_config_map`, and viewer-role findings; round 2 caught the host-only-omission vs padding-shadow impossibility and the scheduler-unreachable bounds exit; round 3 clean).

**Claim-time sizing: Tier B held, owner stands, membership held — all 8 keys in, none shed, none returned, no code change.**
All eight are zero-occurrence as behaviour (seven nowhere under `crates/`/`modules/`/`xtask/`; `seam_gap` only as the `("seam_gap", "10%")` padding twin plus an unrelated wave-test fn name); every canonical read site is emission-time `GCode`/`Wipe` code, so `crates/slicer-gcode` stands (ticket-27 hazard checked — `machine-gcode-emit`'s sweep is the wrong seam). Canonical declares all eight scalar, so no ticket-125 vector model.

**Authoring-time grounding findings (recorded in the packet):**
- Canonical defaults/bounds grounded against the map oracle: `has_scarf_joint_seam` coBool `false`, `role_based_wipe_speed` coBool `true`, `seam_slope_conditional` coBool `false`, `scarf_angle_threshold` coInt `155` [0,180], `scarf_joint_flow_ratio` coFloat `1` [0,2], `scarf_joint_speed` coFloatOrPercent `100%` min 1, `scarf_overhang_threshold` coPercent `40%` min 0, `seam_gap` coFloatOrPercent `10%` min 0 (percent base = nozzle diameter, not line width).
- `has_scarf_joint_seam` is viewer-only in canonical (`GCodeProcessor::apply_config`; 04's citation-fix anticipated this) — the packet wires it as the port-side stage enable gate as deliberate divergence DEV-177(a) (no viewer pipeline here; generation selection lives with P53's slope keys, ticket 60).
- `seam_gap` is the only key with a padding twin: seven keys host-only omitted from `to_config_map` (ticket-42 precedent), `seam_gap` carried by an arm in the same file so the live value shadows the twin (284 precedent).
- `role_based_wipe_speed` rides draft 277's wipe `Move` as a reconciled FORWARD-DEP (activation-blocked; never a second wipe path); the Fill-side concentric arm of `seam_gap` is recorded unimplemented DEV-177(b) (no concentric filler); bounds reject at the emitter gate, ticket-113 class, DEV-177(c); canonical value spellings ride ticket 132 (out of scope, not a divergence row).
- One intended default output change: live `seam_gap` clipping (loops shorten, count unchanged); scarf inert at defaults; no default change from the wipe arm (no wipe moves until 277 lands).
- Packet number `284` → `285` derived from disk per ticket 06; DEV-177 first collision-free (LOG max 171, drafts 172–176).
- 04/05 rows unchanged (membership held, no queue-count change).

### Gates

- Not run at authoring time (packet authoring only — no code changed): the packet's own Verification list governs its swarm; `cargo xtask build-guests --check` was NOT run because no guest-affecting edit happened in this session (host-only prose + no IR/WIT/manifest-schema touch; ticket-58 precedent).
