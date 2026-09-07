# 61 — Author packet P54 — Quality / Walls and surfaces (1/2) — emitter

Type: task
Status: resolved
Assignee: wayfinder session (ses_f823d1c69ffe1dLXH3amxZ1fck) — claimed 2026-09-07, resolved 2026-09-07
Blocked by: 06, 101, 107
Map: ../map.md

## Question

Author the spec packet for **P54 — Quality / Walls and surfaces (1/2) — emitter** — 9 keys, Tier B new logic, owner host emitter (crates/slicer-gcode). Key membership from [05-asset-packet-list.md](./05-asset-packet-list.md) (packet P54 — Quality / Walls and surfaces (1/2) — emitter):

`bottom_solid_infill_flow_ratio`, `first_layer_flow_ratio`, `gap_fill_flow_ratio`, `inner_wall_flow_ratio`, `internal_solid_infill_flow_ratio`, `is_infill_first`, `max_travel_detour_distance`, `outer_wall_flow_ratio`, `overhang_flow_ratio`

Authoring obligations:
- Use `/spec-packet-generator`; the authoring gate is `/spec-review <packet> --preflight` (must pass).
- Apply 02's parity-evidence standard — canonical function-read + invariant tests; `OrcaSlicerDocumented/` is readable, not runnable; unverifiable behaviour surfaces to the human first, never blocks.
- Packet number + status: derive from disk at authoring time per ticket 06's rule — ledger facts (next free number, `status: draft` vs `active`) are never frozen.
- Verify the owner's seam and the missing decision point per key (04) — re-derive from code. Work: new behaviour inside the existing owner.

Resolved when the packet is authored, preflighted, and its directory linked here.

## Answer

Packet authored: [`docs/spec_packets/287-walls-flow-ratios-emitter/`](../../../spec_packets/287-walls-flow-ratios-emitter/),
`status: draft`, `PREFLIGHT PASS` (S0–S8 clean: S0 five files non-empty; S1 no satisfied-dep claim — 285/286 described as draft with no edge; S2 `DEV-179` absent from the log, format-conformant, collision-free; S3 no schema versions; S4 no new ADRs; S5 all symbols resolve — `emit_gcode`, `ResolvedConfig` + `declare_resolved_config!`, `entity.path.role`, `point.flow_factor` + `point.overhang_quartile`, `global_layer_index` + `order_lock`, `assemble_ordered_entities_with_support_identities`, `to_config_map`, `ORCA_CONFIG_PADDING`, `[resolved_config]`; S6 all seven IR variants resolve, WIT untouched; S7 file-per-binary auto-discovered, no aggregator, Step 1 owns the new guard within its edit cap; S8 locked-path bypass conforms to ADR-0062/0063).

**Claim-time sizing: Tier B held, owner stands, membership re-sized 9→8 — seven ratios + adopted gate in, two keys returned, no code change.**
Eight keys zero-occurrence as behaviour (seven ratios nowhere under `crates/`/`modules/`/`xtask/`; `max_travel_detour_distance` only as an `ORCA_CONFIG_PADDING` twin — rule 2, not evidence); none in `ResolvedConfig`, `FeedrateConfig`, or `to_config_map`. Every canonical read site for the seven ratios + gate is emission-time `GCode::extrude_entity`, so `crates/slicer-gcode` stands (ticket-27 hazard checked — `machine-gcode-emit`'s sweep publishes placeholders, E scaling is the host emitter's job). Canonical declares all eight scalar, so no ticket-125 vector model.

**Authoring-time grounding findings (recorded in the packet):**
- Canonical defaults/bounds grounded against the map oracle: seven ratios coFloat `1.0` (`min 0`, `max 2`), `set_other_flow_ratios` coBool `false` (no bounds); bottom-solid unconditional, the other six gated on the gate; first-layer modifier on layer 0 excluding brim/skirt; detour key coFloatOrPercent `0` (zero disables, strict `>` fallback); `is_infill_first` coBool `false` with the first-layer-always-walls exception.
- `set_other_flow_ratios` adopted P55→P54 as a split-boundary adjustment (05 calls splits proposals; the gate arms this family's ratios — ticket-35 fold precedent); P55 sheds it 9→8 with a backward dep, no forward edge.
- `is_infill_first` returned: ordering lives in `assemble_ordered_entities_with_support_identities` (walls-then-infill today, no per-region switch) — emitter reordering would fight the scheduler merge; wrong seam, not declared.
- `max_travel_detour_distance` returned: no avoidance planner exists (path-optimization emits direct travel; emitter consumes precomputed travels) — a limit with no planner would be declaration-only (rule 1); missing feature named (avoid-crossing-perimeters planner).
- Overhang selection via point-level `overhang_quartile.is_some()` (DEV-179(b)) — the port has no `OverhangPerimeter` role; bounds enforcement is reject-the-slice per ticket-113 (DEV-179(a) — canonical never enforces).
- Twin shadowing: none of the eight has a padding twin — host-only omitted (ticket-42 precedent), table untouched, defaults identity (no CONFIG_BLOCK change).
- Packet number `286` → `287` derived from disk per ticket 06; DEV-179 first collision-free (LOG max 171, drafts 172–178).
- 04 tier rows annotated (seven in-packet, gate adopted, two returned); 05 P54 9→8 + P55 9→8 rows annotated (split line 8+8); no queue-count change. No new fog graduated, nothing ruled out of scope.

### Gates

- Not run at authoring time (packet authoring only — no code changed): the packet's own Verification list governs its swarm; `cargo xtask build-guests --check` was NOT run because no guest-affecting edit happened in this session (host-only prose + docs annotations, no IR/WIT/manifest-schema touch; ticket-60 precedent).
