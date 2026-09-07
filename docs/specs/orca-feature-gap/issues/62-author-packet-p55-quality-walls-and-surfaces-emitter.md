# 62 — Author packet P55 — Quality / Walls and surfaces (2/2) — emitter

Type: task
Status: resolved
Assignee: wayfinder session (ses_f822e1b4cffeLLYOsQiZ3nv63z) — claimed 2026-09-07, resolved 2026-09-07
Blocked by: 06, 101, 107
Map: ../map.md

## Question

Author the spec packet for **P55 — Quality / Walls and surfaces (2/2) — emitter** — 9 keys, Tier B new logic, owner host emitter (crates/slicer-gcode). Key membership from [05-asset-packet-list.md](./05-asset-packet-list.md) (packet P55 — Quality / Walls and surfaces (2/2) — emitter):

`print_flow_ratio`, `reduce_crossing_wall`, `set_other_flow_ratios`, `small_area_infill_flow_compensation`, `small_area_infill_flow_compensation_model`, `sparse_infill_flow_ratio`, `support_flow_ratio`, `support_interface_flow_ratio`, `top_solid_infill_flow_ratio`

Authoring obligations:
- Use `/spec-packet-generator`; the authoring gate is `/spec-review <packet> --preflight` (must pass).
- Apply 02's parity-evidence standard — canonical function-read + invariant tests; `OrcaSlicerDocumented/` is readable, not runnable; unverifiable behaviour surfaces to the human first, never blocks.
- Packet number + status: derive from disk at authoring time per ticket 06's rule — ledger facts (next free number, `status: draft` vs `active`) are never frozen.
- Verify the owner's seam and the missing decision point per key (04) — re-derive from code. Work: new behaviour inside the existing owner.

Resolved when the packet is authored, preflighted, and its directory linked here.

## Answer

Packet authored: [`docs/spec_packets/288-walls-flow-compensation-emitter/`](../../../spec_packets/288-walls-flow-compensation-emitter/),
`status: draft`, `PREFLIGHT PASS` (S0 five files non-empty; S1 no satisfied-dep claim — 287 described as draft FORWARD-DEP with no edge; S2 `DEV-180` absent from the log, format-conformant, collision-free; S3 no schema versions; S4 no new ADRs; S5 all symbols resolve — `emit_gcode`, `ResolvedConfig` + `declare_resolved_config!`, `entity.path.role`, per-point `distance` + `point.flow_factor`, `order_lock`, `ExtrusionRole` sparse/support/interface/top-solid/internal-solid/bottom-solid variants, `to_config_map` omission, `ORCA_CONFIG_PADDING`, `[resolved_config]`; S6 no WIT drift (role variants verified in `slice_ir.rs`); S7 file-per-binary auto-discovered, no aggregator, Step 1 owns the new guard within its edit cap; S8 locked-path bypass conforms to ADR-0062/0063).

**Claim-time sizing: Tier B held, owner stands, membership re-sized 9→7+1 with no code change.**
Seven keys zero-occurrence as behaviour (five ratios nowhere under `crates/`/`modules/`/`xtask/`; small-area pair nowhere; `reduce_crossing_wall` only as an `ORCA_CONFIG_PADDING` twin — rule 2, not evidence); none in `ResolvedConfig`, `FeedrateConfig`, or `to_config_map`. Every canonical read site for the seven is emission-time `GCode::extrude_entity` (role ratios) or the `SmallAreaInfillFlowCompensator` per-segment path, so `crates/slicer-gcode` stands (ticket-27 hazard checked — `machine-gcode-emit`'s sweep publishes placeholders, E scaling is the host emitter's job). Canonical declares all seven scalar, so no ticket-125 vector model.

**Authoring-time grounding findings (recorded in the packet):**
- Canonical defaults/bounds grounded against the map oracle: five ratios coFloat `1.0` (`min 0`, `max 2`) except `print_flow_ratio` floor `0.01`; `small_area_infill_flow_compensation` coBool `false`; model coStrings ten-pair default adopted verbatim; `reduce_crossing_wall` coBool `false` (`INITIAL_REDUCE_CROSSING_WALL`).
- Gate split: sparse/support/interface gated on `set_other_flow_ratios` (adopted P55→P54 by ticket 61 — this packet references draft 287's gate, never redeclares; activation sequences after 287); print/top unconditional (same unconditional class as 287's bottom-solid).
- `reduce_crossing_wall` returned: enable for `AvoidCrossingPerimeters::travel_to` + `init_layer` over a planner the port lacks (path-optimization emits direct travel; emitter consumes precomputed travels — ticket-61 detour precedent); wiring the bool alone would be declaration-only (rule 1); missing feature named (avoid-crossing-perimeters planner, shared with `max_travel_detour_distance`).
- Small-area pattern gate omitted as DEV-180(c): canonical `_needSAFC` requires a rectilinear-family pattern the emitter cannot see (patterns are holder-selected module identity, rule 4 holder-only); default holders are rectilinear-family so defaults stay faithful.
- Twin shadowing: none of the seven has a padding twin — host-only omitted (ticket-42 precedent), table untouched (`reduce_crossing_wall`'s twin stays with the returned key).
- Packet number `287` → `288` derived from disk per ticket 06; DEV-180 first collision-free (LOG max 171, drafts 172–179).
- 04 tier rows annotated (seven in-packet, one returned); 05 P54/P55 split line + P55 section annotated (8→7+1); no queue-count change. No new fog graduated, nothing ruled out of scope.

### Gates

- Not run at authoring time (packet authoring only — no code changed): the packet's own Verification list governs its swarm; `cargo xtask build-guests --check` was NOT run because no guest-affecting edit happened in this session (host-only prose + docs annotations, no IR/WIT/manifest-schema touch; ticket-60 precedent).
