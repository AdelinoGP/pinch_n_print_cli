# 65 — Author packet P58 — Speed / Initial layer speed — emitter

Type: task
Status: resolved
Assignee: wayfinder session (ses_20260907_P58) — claimed 2026-09-07, resolved 2026-09-07
Blocked by: 06, 101, 107
Map: ../map.md

## Question

Author the spec packet for **P58 — Speed / Initial layer speed — emitter** — 1 keys, Tier B new logic, owner host emitter (crates/slicer-gcode). Key membership from [05-asset-packet-list.md](./05-asset-packet-list.md) (packet P58 — Speed / Initial layer speed — emitter):

`slow_down_layers`

Authoring obligations:
- Use `/spec-packet-generator`; the authoring gate is `/spec-review <packet> --preflight` (must pass).
- Apply 02's parity-evidence standard — canonical function-read + invariant tests; `OrcaSlicerDocumented/` is readable, not runnable; unverifiable behaviour surfaces to the human first, never blocks.
- Packet number + status: derive from disk at authoring time per ticket 06's rule — ledger facts (next free number, `status: draft` vs `active`) are never frozen.
- Verify the owner's seam and the missing decision point per key (04) — re-derive from code. Work: new behaviour inside the existing owner.

Resolved when the packet is authored, preflighted, and its directory linked here.

## Answer

Packet authored: [`docs/spec_packets/291-slow-down-layers-initial-layer-speed/`](../../../spec_packets/291-slow-down-layers-initial-layer-speed/),
`status: draft`, `PREFLIGHT PASS` (S0 five files non-empty; S1 no satisfied-dep claim — no dep on any draft packet, packet 289 is position-adjacent prose only with no shared helper and no ordering edge; S2 `DEV-183` absent from the log and all drafts, format-conformant, collision-free — LOG max DEV-171, drafts claim DEV-172–DEV-182; S3 no schema versions; S4 no new ADRs; S5 all symbols resolve — `extract_int_as_u32` + `declare_resolved_config!` in `resolved_config.rs`, `emit_gcode` + `resolve_feedrate` per-point `F` emission site + `layer.global_layer_index` loop binding + `Custom("Travel")` travel roles + `GCodeCommand::Retract` in `emit.rs` (no blend exists anywhere), `FeedrateConfig.initial_layer_speed` / `initial_layer_infill_speed` + `from_raw_config` in `feedrate.rs` with the `run.rs` wiring, `ExtrusionRole` variants incl. `BottomSolidInfill`/`Skirt`/`Brim` in `slice_ir.rs`, `ORCA_CONFIG_PADDING` + `serialize_config_block` + `resolved_config_to_map` in `serialize.rs` (zero twins), `gen-config-docs --check` gate spelling, test autodiscovery; S6 all role identifiers resolve, travel-role spelling exact; S7 file-per-binary auto-discovered, guard self-contained within the 3-edit cap; S8 ADR-0052-conformant — mm/s-value scaling after the factor fallback, no contract change — and ADR-0062-conformant — F-scaling is speed-side, direction preserved).

**Claim-time sizing: Tier B held, owner stands but the seam corrected, membership held 1-in — no code change.**
The key is live in canonical (not dead, no alias) and zero-occurrence as behaviour here (verified by grep — no `slow_down_layers` spelling anywhere under `crates/`/`modules/`/`xtask/` outside map prose; the emitter resolves every layer at the same role speed today). Every canonical read site is emission-time `GCode::_extrude`, so `crates/slicer-gcode` stands (ticket-27 hazard checked — `machine-gcode-emit`'s sweep publishes placeholders, per-move feedrate math is the host emitter's job). Tier table owner text corrected from `feedrate.rs` (the speed table only) to the per-entity blend arm in `emit.rs` over that table.
**Canonical scalarity IS held** — `coInt` scalar in `PrintConfig.cpp`, so unlike packets 276/277/279–289 there is no ticket-125 vector arm and DEV-183 carries no vector clause.

**Authoring-time grounding findings (recorded in the packet):**
- Canonical declaration grounded against the map oracle: `slow_down_layers` coInt `0` min `0`, scalar (`PrintConfig.cpp`).
- Master gate: `slow_down_layers > 1` gates the whole arm, so defaults (`0`) AND `1` are identity (byte-identical default stream, AC-2 pins both; the inverse of packet 289's emitting default).
- Port shape: per-entity blend at the per-point `F` emission site over the live `FeedrateConfig` table (`run.rs` wires it from the raw config — the packet adds no speed plumbing, only the count): `slow_down_blend_factor` helper (`None` at `<= 1` or past the ramp, else `layer / N`), `is_perimeter` first-layer-speed selection over the tree's `ExtrusionRole` (re-derived at implementation), never-slow guard, lerp-then-`* 60` through the existing clamp+round rendering; travels/retracts never enter.
- No `u32` runtime bound exists to build: post-extraction values admit no representable violation (contrast packet-282's `f32` speeds); AC-N1 pins the `TypeMismatch` contract on Float/String/Bool spellings, and the negative-`Int` wrap is recorded as shared pre-existing extractor context in DEV-183(a), not introduced here.
- Bottom/skirt/brim held flat as deliberate port divergences DEV-183(b): canonical's `erBottomSurface` flatness falls out of the never-slow guard (its role speed IS the infill base) while this port's PnP-only `bottom_surface_speed` needs the explicit skip; the port resolves both skirt and brim to `skirt_speed` with no `erBrim` arm at the borrowed site, so both hold flat. `#if 0` over-raft arm not borrowed; raft offset is a comment (no raft prefix layers emit) — DEV-183(c).
- Packet number `290` → `291` derived from disk per ticket 06; DEV-183 first collision-free (LOG max 171, drafts 172–182).
- 04 tier row corrected (feedrate.rs → per-entity blend in emit.rs, packet 291); 05 P58 section annotated (1-in at 291); no queue-count change. No new fog graduated, nothing ruled out of scope.

### Gates

- `cargo check -p slicer-gcode --all-targets` green at authoring time (no code changed): proves the cited baseline the packet grounds against. Full packet verification (guard + clippy + gen-config-docs) belongs to the packet's own Verification list at swarm time.
