# 66 — Author packet P59 — Speed / Jerk (XY) — emitter

Type: task
Status: resolved
Assignee: wayfinder session (ses_20260908_P59) — claimed 2026-09-08, resolved 2026-09-08
Blocked by: 06, 101, 107
Map: ../map.md

## Question

Author the spec packet for **P59 — Speed / Jerk (XY) — emitter** — 8 keys, Tier B new logic, owner host emitter (crates/slicer-gcode). Key membership from [05-asset-packet-list.md](./05-asset-packet-list.md) (packet P59 — Speed / Jerk (XY) — emitter):

`default_jerk`, `default_junction_deviation`, `infill_jerk`, `initial_layer_jerk`, `inner_wall_jerk`, `outer_wall_jerk`, `top_surface_jerk`, `travel_jerk`

Authoring obligations:
- Use `/spec-packet-generator`; the authoring gate is `/spec-review <packet> --preflight` (must pass).
- Apply 02's parity-evidence standard — canonical function-read + invariant tests; `OrcaSlicerDocumented/` is readable, not runnable; unverifiable behaviour surfaces to the human first, never blocks.
- Packet number + status: derive from disk at authoring time per ticket 06's rule — ledger facts (next free number, `status: draft` vs `active`) are never frozen.
- Verify the owner's seam and the missing decision point per key (04) — re-derive from code. Work: new behaviour inside the existing owner.

Resolved when the packet is authored, preflighted, and its directory linked here.

## Answer

Packet authored: [`docs/spec_packets/292-speed-jerk-xy-emitter/`](../../../spec_packets/292-speed-jerk-xy-emitter/),
`status: draft`, `PREFLIGHT PASS` (S0 five files non-empty; S1 no satisfied-dep claim — FORWARD-DEP on draft 281 for stream position only, 289 position-adjacent with no shared helper and no ordering edge; S2 `DEV-184` absent from the log and all other packets, format-conformant, collision-free — LOG max DEV-171, drafts claim DEV-172–DEV-183; S3 no schema versions; S4 no new ADRs; S5 all symbols resolve — `DefaultGCodeEmitter::emit_gcode` + `with_flavor` + `with_resolved_config` + `resolve_feedrate` in `emit.rs`, `set_jerk_xy` + Marlin2-only `set_junction_deviation` in `flavor.rs` with `M566` confirmed absent, `declare_resolved_config!` + `extract_float_or_first` + `extract_u32_or_first` + hand-built `to_config_map` in `resolved_config.rs`, `FeedrateConfig`, all named `ExtrusionRole` variants, `ORCA_CONFIG_PADDING` zero twins, ticket-113 `ConfigBoundsIndex` seam, `gen-config-docs --check` gate, both fixture patterns, `run.rs` flavor wiring, `host-keys.toml` `[resolved_config]`; S6 all role spellings resolve, no WIT claim; S7 guard auto-discovered per-file binary, `--test` form matches the 289/291 precedent; S8 ADR-0052/ADR-0062 conformant — parallel jerk stage, no `resolve_feedrate`/factor/E touch, no reorder — plus AC-command and Doc-Impact green).

**Claim-time sizing: Tier B held, owner stands but the seam corrected, membership held 8-in — no code change.**
All eight keys are live in canonical (not dead, no alias) and zero-occurrence as behaviour here (verified by grep — no eight-key spelling anywhere under `crates/`/`modules/`/`xtask/` outside map prose; the emitter's `set_jerk_xy`/`set_junction_deviation` builders exist unwired and `emit_gcode` emits no jerk command today). Every canonical read site is emission-time (`GCode::_extrude` role chain, `GCode::travel_to` travel jerk, `GCode::process_layer` first-layer header + Marlin-only JD arm), so `crates/slicer-gcode` stands (ticket-27 hazard checked — `machine-gcode-emit`'s sweep publishes placeholders, per-move motion commands are the host emitter's job). Tier table owner text corrected from `estimator.rs` (time-math only) to the per-entity selection stage in `emit.rs`.
**Canonical scalarity NOT held** — all eight are `coFloats` per-nozzle nullable vectors read via `NOZZLE_CONFIG`, so unlike packets 290/291 there IS a ticket-125 vector arm and DEV-184(b) records the scalar-global simplification with first-wins ingest (ticket-140 precedent).

**Authoring-time grounding findings (recorded in the packet):**
- Canonical declarations grounded against the map oracle: `default_jerk` coFloats `0` min 0; `default_junction_deviation` coFloats `0` min 0 max 0.3 (Marlin-only alternate representation, never converted to/from jerk — independent, co-emitted on layer 0); five role jerks coFloats `9` min 0; `travel_jerk` coFloats `12` min 0 (only non-9 default; `ratio_over` base for the unqueued `initial_layer_travel_jerk` coFloatsOrPercents).
- Master gate: `default_jerk > 0` gates the whole stage, so defaults are byte-identical (AC-2 pins it — the inverse of packet 289's emitting default).
- Port shape: per-entity selection at the print/travel call sites over `entity.path.role` + `global_layer_index` (canonical precedence first-layer → outer → inner → top-surface → infill → default, `> 0` fallthrough; six fallthrough roles recorded in AC-3); flat `travel_jerk` on every layer (canonical's percent-over-base first-layer override and short-travel `outer_wall_jerk` override are named non-borrows — no `initial_layer_travel_jerk` key exists in scope); first-layer `M205 J` through the existing Marlin2-only arm (other flavors jerk-only, no invented fallback); per-stream print/travel/JD change-dedup (289 shape, not a shared helper); no `M566` on any path (no builder exists — machine-max jerks stay 281's envelope).
- JD is Marlin-only upstream but renders Marlin2-only here because that is the port's existing `set_junction_deviation` contract (DEV-184(c)); canonical's stateful first-layer restore becomes stateless per-entity re-selection; canonical's warn-and-cap validation becomes reject-the-slice (DEV-184(a), ticket-113 rule).
- Packet number `291` → `292` derived from disk per ticket 06; DEV-184 first collision-free (LOG max 171, drafts 172–183).
- 04 tier rows corrected (estimator.rs → per-entity selection stage in emit.rs, packet 292); 05 P59 section annotated (8-in at 292); no queue-count change. No new fog graduated, nothing ruled out of scope.

### Gates

- `cargo check -p slicer-gcode --all-targets` green at authoring time (no code changed): proves the cited baseline the packet grounds against. Full packet verification (guard + clippy + gen-config-docs) belongs to the packet's own Verification list at swarm time.
