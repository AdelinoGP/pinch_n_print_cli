# 60 — Author packet P53 — Quality / Seam (2/2) — emitter

Type: task
Status: resolved
Assignee: wayfinder session (ses_f82446674ffecZDtzZYDbECkwD) — claimed 2026-09-07, resolved 2026-09-07
Blocked by: 06, 101, 107
Map: ../map.md

## Question

Author the spec packet for **P53 — Quality / Seam (2/2) — emitter** — 8 keys, Tier B new logic, owner host emitter (crates/slicer-gcode). Key membership from [05-asset-packet-list.md](./05-asset-packet-list.md) (packet P53 — Quality / Seam (2/2) — emitter):

`seam_slope_entire_loop`, `seam_slope_inner_walls`, `seam_slope_min_length`, `seam_slope_start_height`, `seam_slope_steps`, `seam_slope_type`, `wipe_before_external_loop`, `wipe_on_loops`

Authoring obligations:
- Use `/spec-packet-generator`; the authoring gate is `/spec-review <packet> --preflight` (must pass).
- Apply 02's parity-evidence standard — canonical function-read + invariant tests; `OrcaSlicerDocumented/` is readable, not runnable; unverifiable behaviour surfaces to the human first, never blocks.
- Packet number + status: derive from disk at authoring time per ticket 06's rule — ledger facts (next free number, `status: draft` vs `active`) are never frozen.
- Verify the owner's seam and the missing decision point per key (04) — re-derive from code. Work: new behaviour inside the existing owner.

Resolved when the packet is authored, preflighted, and its directory linked here.

## Answer

Packet authored: [`docs/spec_packets/286-seam-slope-wipe-emitter/`](../../../spec_packets/286-seam-slope-wipe-emitter/),
`status: draft`, `PREFLIGHT PASS` (S0–S8 clean first round: S0 five files incl. `task-map.md`; S1 both prereq packets confirmed `draft` with the activation-block on 285 and the 277 no-dep as written; S2 `DEV-178` absent from the log — the one `docs/` hit re-verified as this packet's own files; S5 all eight symbols resolved in-tree; S7 `slicer-gcode` tests auto-discovered, no registration; S8 slope/wipe bypass conforms to ADR-0062/0063).

**Claim-time sizing: Tier B held, owner stands, membership held — all 8 keys in, none shed, none returned, no code change.**
Six keys zero-occurrence under `crates/`/`modules/`/`xtask/`; `seam_slope_type` + `wipe_on_loops` occur only as `ORCA_CONFIG_PADDING` twins (rule 2: not evidence). Every canonical read site is emission-time `GCode::extrude_loop`, so `crates/slicer-gcode` stands (ticket-27 hazard checked — `machine-gcode-emit`'s sweep is the wrong seam, 285 precedent). Canonical declares all eight scalar, so no ticket-125 vector model.

**Authoring-time grounding findings (recorded in the packet):**
- Canonical defaults/bounds grounded against the map oracle: `seam_slope_type` coEnum `none` (`none`/`external`/`all`, no bounds), `seam_slope_start_height` coFloatOrPercent `0` min 0, `seam_slope_entire_loop` coBool `false`, `seam_slope_min_length` coFloat `20.0` min 0, `seam_slope_steps` coInt `10` min 1, `seam_slope_inner_walls` / `wipe_before_external_loop` / `wipe_on_loops` coBool `false` (no bounds).
- Slope generalises 285's scarf stage **additively** (one gate chain, one ramp emitter — a parallel stage would double-clip); implementation sequences after 285 lands (activation-blocked, no authoring dep).
- Loop wipe is its **own emission site** at loop end (pre-leave / pre-external moves), not a policy gate on draft 277's retract `Move` — different trigger; only the coincidence precedence (at most one wipe) is shared, with no dependency edge.
- Twin shadowing without table edits: `seam_slope_type` + `wipe_on_loops` ride `to_config_map` arms (284 precedent), the other six host-only omitted (ticket-42 precedent); one intended default CONFIG_BLOCK value change (live `wipe_on_loops = false` shadows the stale `"1"` twin, count unchanged).
- `Layer::is_perimeter_compatible` grouping not ported unless the implementer's delegated read proves the loop-type gate insufficient; `start_height` percent-base ambiguity carried as `[FWD]` with an absolute-mm fallback divergence — neither blocks.
- Packet number `285` → `286` derived from disk per ticket 06; DEV-178 first collision-free (LOG max 171, drafts 172–177).
- 04/05 rows unchanged (membership held, no queue-count change). No new fog graduated, nothing ruled out of scope.

### Gates

- Not run at authoring time (packet authoring only — no code changed): the packet's own Verification list governs its swarm; `cargo xtask build-guests --check` was NOT run because no guest-affecting edit happened in this session (host-only prose + no IR/WIT/manifest-schema touch; ticket-59 precedent).
