# Task Map — Packet 75

Four backlog tasks, one per phase, each an independent deepening with no hard cross-dependency. They share a
packet because they touch mostly disjoint files and ship behind one acceptance ceremony; sequencing is
risk-optimized (most contained first, biggest last).

| Phase | Task | Concern | Primary files | Authoritative doc |
|-------|------|---------|---------------|-------------------|
| 1 | TASK-216 | PrePass stage runner (bracket unification) | `prepass.rs` | `docs/04_host_scheduler.md`, `docs/01_system_architecture.md` |
| 2 | TASK-217 | Pure IR harvest extraction + region-id dedup | `dispatch.rs`, `wit_host.rs` | `docs/02_ir_schemas.md` |
| 3 | TASK-218 | WIT marshalling `with:` type unification | `wit_host.rs` | `docs/03_wit_and_manifest.md` |
| 4 | TASK-219 | Model intake assembly seam + z-extent dedup | `model_loader.rs`, `helpers_cmd.rs`, `CONTEXT.md` | `docs/02_ir_schemas.md`, `docs/08_coordinate_system.md` |

## Ordering rationale

Risk-optimized **2 → 4 → 1 → 3** in original-report option numbers = Phase **1 → 2 → 3 → 4** here: the contained,
no-ABI changes first (prepass runner, harvest), the largest (WIT unification) and the cross-file model intake
last. Phases 2 and 3 both edit `wit_host.rs` in non-overlapping regions — Phase 2 only changes a function's
visibility and deletes a copy in `dispatch.rs`; Phase 3 deletes converters elsewhere in `wit_host.rs`.

## ADR lineage

- **ADR-0001** (Phase 1) and **ADR-0002** (Phase 3) are new; `docs/adr/` is created by this packet (first ADRs in
  the repo). No prior packet is superseded.

## Deferrals (future packets)

- All-prepass-ordering declarative graph (from Phase 1).
- Layer-world-only region-view accessors / builder `push_*` repetition (from Phase 3).
- Per-world extrusion-role / extrusion-path / retract-mode converters (`finalization_role_*`,
  `finalization_path_*`, `convert_postpass_*`) — smaller dedup with asymmetric layer coverage (from Phase 3; see ADR-0002).
- 3MF XML-parser decomposition — extracting `decode_paint_hex_strokes` from the parse loop (from Phase 4).
